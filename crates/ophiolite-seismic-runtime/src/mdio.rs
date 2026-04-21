use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ophiolite_seismic::{
    CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
    ProjectedPoint2, ProjectedPolygon2, ProjectedVector2, SampleDataConversionKind,
    SampleDataFidelity, SampleValuePreservation, SurveyGridTransform, SurveySpatialAvailability,
    SurveySpatialDescriptor, TimeDepthDomain,
};
use serde_json::{Map, Value};
use zarrs::array::{Array, DataType};
use zarrs::array_subset::ArraySubset;
use zarrs::filesystem::FilesystemStore;
use zarrs::storage::{ReadableWritableListableStorage, ReadableWritableListableStorageTraits};

use crate::error::SeismicStoreError;
use crate::metadata::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes, VolumeMetadata,
    generate_store_id,
};
use crate::storage::tbvol::{
    TbvolWriter, recommended_default_tbvol_tile_target_mib, recommended_tbvol_tile_shape,
};
use crate::storage::tile_geometry::{TileCoord, TileGeometry};
use crate::storage::volume_store::{
    OccupancyTile, TileBuffer, VolumeStoreReader, VolumeStoreWriter,
};
use crate::store::{StoreHandle, open_store};

const SEISMIC_ARRAY_PATH: &str = "/seismic";
const TRACE_MASK_ARRAY_PATH: &str = "/trace_mask";
const CDP_X_ARRAY_PATH: &str = "/cdp-x";
const CDP_Y_ARRAY_PATH: &str = "/cdp-y";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeSubset {
    pub inline_start: usize,
    pub inline_count: usize,
    pub xline_start: usize,
    pub xline_count: usize,
    pub sample_start: usize,
    pub sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdioTbvolStorageEstimate {
    pub shape: [usize; 3],
    pub tile_shape: [usize; 3],
    pub tile_count: usize,
    pub has_occupancy: bool,
    pub amplitude_bytes: u64,
    pub occupancy_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolvedVolumeSubset {
    inline_start: usize,
    inline_end: usize,
    xline_start: usize,
    xline_end: usize,
    sample_start: usize,
    sample_end: usize,
}

impl ResolvedVolumeSubset {
    fn shape(&self) -> [usize; 3] {
        [
            self.inline_end - self.inline_start,
            self.xline_end - self.xline_start,
            self.sample_end - self.sample_start,
        ]
    }
}

struct MdioStoreDescriptor {
    subset: ResolvedVolumeSubset,
    volume: VolumeMetadata,
    seismic: Array<dyn ReadableWritableListableStorageTraits>,
    trace_mask: Option<Array<dyn ReadableWritableListableStorageTraits>>,
}

pub fn looks_like_mdio_path(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    if matches!(
        path.extension().and_then(|value| value.to_str()),
        Some(ext) if ext.eq_ignore_ascii_case("mdio")
    ) {
        return true;
    }
    path.is_dir() && path.join(".zgroup").exists() && path.join("seismic").join(".zarray").exists()
}

pub fn ingest_mdio_store(
    input_root: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    chunk_shape: [usize; 3],
    subset: Option<VolumeSubset>,
) -> Result<StoreHandle, SeismicStoreError> {
    let descriptor = describe_mdio_store(input_root, subset)?;
    let shape = descriptor.volume.shape;
    let mut tile_shape = resolve_tbvol_tile_shape(chunk_shape, shape);
    tile_shape[2] = shape[2];
    let writer = TbvolWriter::create(
        &store_root,
        descriptor.volume.clone(),
        tile_shape,
        descriptor.trace_mask.is_some(),
    )?;
    let reader = MdioVolumeReader {
        descriptor,
        geometry: writer.tile_geometry().clone(),
    };

    for tile in reader.tile_geometry().iter_tiles() {
        let amplitudes = reader.read_tile(tile)?;
        writer.write_tile(tile, amplitudes.as_slice())?;
        if let Some(occupancy) = reader.read_tile_occupancy(tile)? {
            writer.write_tile_occupancy(tile, occupancy.as_slice())?;
        }
    }

    writer.finalize()?;
    open_store(store_root)
}

pub fn estimate_mdio_tbvol_storage(
    input_root: impl AsRef<Path>,
    chunk_shape: [usize; 3],
    subset: Option<VolumeSubset>,
) -> Result<MdioTbvolStorageEstimate, SeismicStoreError> {
    let (subset, has_occupancy) = describe_mdio_layout(input_root, subset)?;
    let shape = subset.shape();
    let mut tile_shape = resolve_tbvol_tile_shape(chunk_shape, shape);
    tile_shape[2] = shape[2];
    let geometry = TileGeometry::new(shape, tile_shape);
    let tile_count = geometry.tile_count();
    let amplitude_bytes = tile_count as u64 * geometry.amplitude_tile_bytes();
    let occupancy_bytes = if has_occupancy {
        tile_count as u64 * geometry.occupancy_tile_bytes()
    } else {
        0
    };

    Ok(MdioTbvolStorageEstimate {
        shape,
        tile_shape,
        tile_count,
        has_occupancy,
        amplitude_bytes,
        occupancy_bytes,
        total_bytes: amplitude_bytes.saturating_add(occupancy_bytes),
    })
}

fn describe_mdio_layout(
    input_root: impl AsRef<Path>,
    subset: Option<VolumeSubset>,
) -> Result<(ResolvedVolumeSubset, bool), SeismicStoreError> {
    let root = input_root.as_ref().to_path_buf();
    let seismic = open_array_at_path(&root, SEISMIC_ARRAY_PATH)?;
    let full_shape = array_shape3(&seismic, "seismic")?;
    let subset = resolve_volume_subset(subset, full_shape)?;
    let has_occupancy = try_open_array_at_path(&root, TRACE_MASK_ARRAY_PATH)?.is_some();
    Ok((subset, has_occupancy))
}

fn describe_mdio_store(
    input_root: impl AsRef<Path>,
    subset: Option<VolumeSubset>,
) -> Result<MdioStoreDescriptor, SeismicStoreError> {
    let root = input_root.as_ref().to_path_buf();
    let seismic = open_array_at_path(&root, SEISMIC_ARRAY_PATH)?;
    let seismic_dimensions = read_array_dimensions(root.join("seismic").join(".zattrs"))
        .or_else(|_| array_dimension_names(&seismic, "seismic"))?;
    let inline_axis_name = seismic_dimensions[0].clone();
    let xline_axis_name = seismic_dimensions[1].clone();
    let sample_axis_name = seismic_dimensions[2].clone();
    let full_shape = array_shape3(&seismic, "seismic")?;
    let subset = resolve_volume_subset(subset, full_shape)?;

    let inline_values = read_axis_values_f64(&open_array_at_path(
        &root,
        &format!("/{}", inline_axis_name),
    )?)?[subset.inline_start..subset.inline_end]
        .to_vec();
    let xline_values = read_axis_values_f64(&open_array_at_path(
        &root,
        &format!("/{}", xline_axis_name),
    )?)?[subset.xline_start..subset.xline_end]
        .to_vec();
    let sample_axis = read_axis_values_f32(&open_array_at_path(
        &root,
        &format!("/{}", sample_axis_name),
    )?)?[subset.sample_start..subset.sample_end]
        .to_vec();

    let sample_axis_attributes =
        try_read_json_object(root.join(&sample_axis_name).join(".zattrs"))?.unwrap_or_default();
    let sample_axis_unit = axis_unit(&sample_axis_attributes, &sample_axis_name)
        .unwrap_or_else(|| default_sample_axis_unit(&sample_axis_name));
    let sample_axis_domain = infer_sample_axis_domain(&sample_axis_name, &sample_axis_unit);
    let trace_mask = try_open_array_at_path(&root, TRACE_MASK_ARRAY_PATH)?;
    let spatial = derive_mdio_spatial_descriptor(&root, &subset)?;
    let coordinate_reference_binding = spatial
        .as_ref()
        .and_then(|descriptor| descriptor.coordinate_reference.clone())
        .map(|detected| CoordinateReferenceBinding {
            detected: Some(detected.clone()),
            effective: Some(detected),
            source: CoordinateReferenceSource::ImportManifest,
            notes: Vec::new(),
        });
    let root_attributes = try_read_json_object(root.join(".zattrs"))?.unwrap_or_default();
    let created_on = root_attributes
        .get("createdOn")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let source = build_mdio_source_identity(
        &root,
        subset.shape(),
        &sample_axis,
        &sample_axis_unit,
        created_on,
    )?;

    Ok(MdioStoreDescriptor {
        subset,
        volume: VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source,
            shape: subset.shape(),
            axes: VolumeAxes::with_vertical_axis(
                inline_values,
                xline_values,
                sample_axis_domain,
                sample_axis_unit,
                sample_axis,
            ),
            segy_export: None,
            coordinate_reference_binding,
            spatial,
            created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
            processing_lineage: None,
        },
        seismic,
        trace_mask,
    })
}

struct MdioVolumeReader {
    descriptor: MdioStoreDescriptor,
    geometry: TileGeometry,
}

impl VolumeStoreReader for MdioVolumeReader {
    fn volume(&self) -> &VolumeMetadata {
        &self.descriptor.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn read_tile<'a>(&'a self, tile: TileCoord) -> Result<TileBuffer<'a>, SeismicStoreError> {
        let origin = self.geometry.tile_origin(tile);
        let effective = self.geometry.effective_tile_shape(tile);
        let subset = ArraySubset::new_with_ranges(&[
            (self.descriptor.subset.inline_start + origin[0]) as u64
                ..(self.descriptor.subset.inline_start + origin[0] + effective[0]) as u64,
            (self.descriptor.subset.xline_start + origin[1]) as u64
                ..(self.descriptor.subset.xline_start + origin[1] + effective[1]) as u64,
            self.descriptor.subset.sample_start as u64..self.descriptor.subset.sample_end as u64,
        ]);
        let raw = retrieve_f32_subset(&self.descriptor.seismic, &subset)?;
        Ok(TileBuffer::Owned(pad_amplitude_tile(
            self.geometry.tile_shape(),
            effective,
            &raw,
        )))
    }

    fn read_tile_occupancy<'a>(
        &'a self,
        tile: TileCoord,
    ) -> Result<Option<OccupancyTile<'a>>, SeismicStoreError> {
        let Some(trace_mask) = &self.descriptor.trace_mask else {
            return Ok(None);
        };
        let origin = self.geometry.tile_origin(tile);
        let effective = self.geometry.effective_tile_shape(tile);
        let subset = ArraySubset::new_with_ranges(&[
            (self.descriptor.subset.inline_start + origin[0]) as u64
                ..(self.descriptor.subset.inline_start + origin[0] + effective[0]) as u64,
            (self.descriptor.subset.xline_start + origin[1]) as u64
                ..(self.descriptor.subset.xline_start + origin[1] + effective[1]) as u64,
        ]);
        let raw = retrieve_mask_subset(trace_mask, &subset)?;
        Ok(Some(OccupancyTile::Owned(pad_occupancy_tile(
            self.geometry.tile_shape(),
            effective,
            &raw,
        ))))
    }
}

fn build_mdio_source_identity(
    root: &Path,
    shape: [usize; 3],
    sample_axis: &[f32],
    sample_axis_unit: &str,
    created_on: &str,
) -> Result<SourceIdentity, SeismicStoreError> {
    let sample_interval_us = if sample_axis_unit == "ms" && sample_axis.len() >= 2 {
        ((sample_axis[1] - sample_axis[0]) * 1000.0)
            .round()
            .clamp(0.0, u16::MAX as f32) as u16
    } else {
        0
    };
    Ok(SourceIdentity {
        source_path: root.to_path_buf(),
        file_size: directory_size_bytes(root)?,
        trace_count: (shape[0] * shape[1]) as u64,
        samples_per_trace: shape[2],
        sample_interval_us,
        sample_format_code: 5,
        sample_data_fidelity: mdio_sample_data_fidelity(created_on),
        endianness: "little".to_string(),
        revision_raw: 0,
        fixed_length_trace_flag_raw: 1,
        extended_textual_headers: 0,
        geometry: GeometryProvenance {
            inline_field: HeaderFieldSpec {
                name: "mdio:inline".to_string(),
                start_byte: 0,
                value_type: "axis_coordinate".to_string(),
            },
            crossline_field: HeaderFieldSpec {
                name: "mdio:crossline".to_string(),
                start_byte: 0,
                value_type: "axis_coordinate".to_string(),
            },
            third_axis_field: None,
        },
        regularization: None,
    })
}

fn mdio_sample_data_fidelity(created_on: &str) -> SampleDataFidelity {
    SampleDataFidelity {
        source_sample_type: "mdio_float32".to_string(),
        working_sample_type: "f32".to_string(),
        conversion: SampleDataConversionKind::Identity,
        preservation: SampleValuePreservation::Exact,
        notes: vec![format!(
            "MDIO float32 samples were copied into the working store without numeric narrowing (source createdOn={created_on})."
        )],
    }
}

fn derive_mdio_spatial_descriptor(
    root: &Path,
    subset: &ResolvedVolumeSubset,
) -> Result<Option<SurveySpatialDescriptor>, SeismicStoreError> {
    let Some(cdp_x) = try_open_array_at_path(root, CDP_X_ARRAY_PATH)? else {
        return Ok(None);
    };
    let Some(cdp_y) = try_open_array_at_path(root, CDP_Y_ARRAY_PATH)? else {
        return Ok(None);
    };
    let coordinate_attributes =
        try_read_json_object(root.join("cdp-x").join(".zattrs"))?.unwrap_or_default();
    let unit = axis_unit(&coordinate_attributes, "length");
    let mut notes = Vec::new();
    let mut observations = Vec::new();
    for local_inline in sample_positions(subset.shape()[0]) {
        for local_xline in sample_positions(subset.shape()[1]) {
            let x = retrieve_scalar_f64(
                &cdp_x,
                subset.inline_start + local_inline,
                subset.xline_start + local_xline,
            )?;
            let y = retrieve_scalar_f64(
                &cdp_y,
                subset.inline_start + local_inline,
                subset.xline_start + local_xline,
            )?;
            if x.is_finite() && y.is_finite() {
                observations.push((local_inline as f64, local_xline as f64, x, y));
            }
        }
    }

    if observations.len() < 3 {
        notes.push(String::from(
            "not enough finite MDIO coordinate samples were available to derive a survey map transform",
        ));
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: None,
                name: None,
                geodetic_datum: None,
                unit,
            }),
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes,
        }));
    }

    let Some((origin, inline_basis, xline_basis)) = fit_grid_transform(&observations) else {
        notes.push(String::from(
            "MDIO coordinate samples do not support a stable affine grid transform",
        ));
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: None,
                name: None,
                geodetic_datum: None,
                unit,
            }),
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes,
        }));
    };

    notes.push(String::from(
        "survey map geometry was derived from MDIO cdp-x/cdp-y coordinate arrays",
    ));
    notes.push(String::from(
        "coordinate reference system identity remains unresolved because MDIO ingest does not yet capture a canonical CRS identifier",
    ));

    let inline_extent = subset.shape()[0].saturating_sub(1) as f64;
    let xline_extent = subset.shape()[1].saturating_sub(1) as f64;
    let corner_00 = origin.clone();
    let corner_10 = affine_grid_point(&origin, &inline_basis, &xline_basis, inline_extent, 0.0);
    let corner_11 = affine_grid_point(
        &origin,
        &inline_basis,
        &xline_basis,
        inline_extent,
        xline_extent,
    );
    let corner_01 = affine_grid_point(&origin, &inline_basis, &xline_basis, 0.0, xline_extent);

    Ok(Some(SurveySpatialDescriptor {
        coordinate_reference: Some(CoordinateReferenceDescriptor {
            id: None,
            name: None,
            geodetic_datum: None,
            unit,
        }),
        grid_transform: Some(SurveyGridTransform {
            origin: origin.clone(),
            inline_basis,
            xline_basis,
        }),
        footprint: Some(ProjectedPolygon2 {
            exterior: vec![
                corner_00.clone(),
                corner_10,
                corner_11,
                corner_01,
                corner_00,
            ],
        }),
        availability: SurveySpatialAvailability::Partial,
        notes,
    }))
}

fn fit_grid_transform(
    observations: &[(f64, f64, f64, f64)],
) -> Option<(ProjectedPoint2, ProjectedVector2, ProjectedVector2)> {
    let mut n = 0.0;
    let mut sum_i = 0.0;
    let mut sum_x = 0.0;
    let mut sum_ii = 0.0;
    let mut sum_ix = 0.0;
    let mut sum_xx = 0.0;
    let mut sum_px = 0.0;
    let mut sum_i_px = 0.0;
    let mut sum_x_px = 0.0;
    let mut sum_py = 0.0;
    let mut sum_i_py = 0.0;
    let mut sum_x_py = 0.0;

    for (inline_index, xline_index, px, py) in observations {
        n += 1.0;
        sum_i += *inline_index;
        sum_x += *xline_index;
        sum_ii += inline_index * inline_index;
        sum_ix += inline_index * xline_index;
        sum_xx += xline_index * xline_index;
        sum_px += *px;
        sum_i_px += inline_index * px;
        sum_x_px += xline_index * px;
        sum_py += *py;
        sum_i_py += inline_index * py;
        sum_x_py += xline_index * py;
    }

    let matrix = [
        [n, sum_i, sum_x],
        [sum_i, sum_ii, sum_ix],
        [sum_x, sum_ix, sum_xx],
    ];
    let coeff_x = solve_3x3(matrix, [sum_px, sum_i_px, sum_x_px])?;
    let coeff_y = solve_3x3(matrix, [sum_py, sum_i_py, sum_x_py])?;

    Some((
        ProjectedPoint2 {
            x: coeff_x[0],
            y: coeff_y[0],
        },
        ProjectedVector2 {
            x: coeff_x[1],
            y: coeff_y[1],
        },
        ProjectedVector2 {
            x: coeff_x[2],
            y: coeff_y[2],
        },
    ))
}

fn solve_3x3(matrix: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let det = determinant_3x3(matrix);
    if det.abs() < 1e-9 {
        return None;
    }

    let det_0 = determinant_3x3([
        [rhs[0], matrix[0][1], matrix[0][2]],
        [rhs[1], matrix[1][1], matrix[1][2]],
        [rhs[2], matrix[2][1], matrix[2][2]],
    ]);
    let det_1 = determinant_3x3([
        [matrix[0][0], rhs[0], matrix[0][2]],
        [matrix[1][0], rhs[1], matrix[1][2]],
        [matrix[2][0], rhs[2], matrix[2][2]],
    ]);
    let det_2 = determinant_3x3([
        [matrix[0][0], matrix[0][1], rhs[0]],
        [matrix[1][0], matrix[1][1], rhs[1]],
        [matrix[2][0], matrix[2][1], rhs[2]],
    ]);

    Some([det_0 / det, det_1 / det, det_2 / det])
}

fn determinant_3x3(matrix: [[f64; 3]; 3]) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn affine_grid_point(
    origin: &ProjectedPoint2,
    inline_basis: &ProjectedVector2,
    xline_basis: &ProjectedVector2,
    inline_index: f64,
    xline_index: f64,
) -> ProjectedPoint2 {
    ProjectedPoint2 {
        x: origin.x + inline_basis.x * inline_index + xline_basis.x * xline_index,
        y: origin.y + inline_basis.y * inline_index + xline_basis.y * xline_index,
    }
}

fn sample_positions(len: usize) -> Vec<usize> {
    if len <= 1 {
        return vec![0];
    }
    let mut values = vec![0, len / 2, len - 1];
    values.sort_unstable();
    values.dedup();
    values
}

fn resolve_volume_subset(
    subset: Option<VolumeSubset>,
    full_shape: [usize; 3],
) -> Result<ResolvedVolumeSubset, SeismicStoreError> {
    let Some(subset) = subset else {
        return Ok(ResolvedVolumeSubset {
            inline_start: 0,
            inline_end: full_shape[0],
            xline_start: 0,
            xline_end: full_shape[1],
            sample_start: 0,
            sample_end: full_shape[2],
        });
    };

    if subset.inline_count == 0 || subset.xline_count == 0 || subset.sample_count == 0 {
        return Err(SeismicStoreError::Message(
            "MDIO subset counts must all be positive".to_string(),
        ));
    }
    let inline_end = subset.inline_start.saturating_add(subset.inline_count);
    let xline_end = subset.xline_start.saturating_add(subset.xline_count);
    let sample_end = subset.sample_start.saturating_add(subset.sample_count);
    if inline_end > full_shape[0] || xline_end > full_shape[1] || sample_end > full_shape[2] {
        return Err(SeismicStoreError::Message(format!(
            "MDIO subset {:?} exceeds source shape {:?}",
            subset, full_shape
        )));
    }
    Ok(ResolvedVolumeSubset {
        inline_start: subset.inline_start,
        inline_end,
        xline_start: subset.xline_start,
        xline_end,
        sample_start: subset.sample_start,
        sample_end,
    })
}

fn resolve_tbvol_tile_shape(chunk_shape: [usize; 3], shape: [usize; 3]) -> [usize; 3] {
    if chunk_shape.iter().all(|value| *value == 0) {
        return recommended_tbvol_tile_shape(
            shape,
            recommended_default_tbvol_tile_target_mib(shape),
        );
    }

    [
        chunk_shape[0].max(1).min(shape[0].max(1)),
        chunk_shape[1].max(1).min(shape[1].max(1)),
        shape[2].max(1),
    ]
}

fn array_shape3(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
    label: &str,
) -> Result<[usize; 3], SeismicStoreError> {
    let shape = array.shape();
    if shape.len() != 3 {
        return Err(SeismicStoreError::Message(format!(
            "{label} array must be 3D, found shape {:?}",
            shape
        )));
    }
    Ok([shape[0] as usize, shape[1] as usize, shape[2] as usize])
}

fn array_dimension_names(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
    label: &str,
) -> Result<Vec<String>, SeismicStoreError> {
    let Some(names) = array.dimension_names() else {
        return Err(SeismicStoreError::Message(format!(
            "{label} array metadata is missing dimension names"
        )));
    };
    if names.len() != 3 {
        return Err(SeismicStoreError::Message(format!(
            "{label} array must expose three dimension names, found {:?}",
            names
        )));
    }
    names
        .iter()
        .map(|value| {
            value.clone().ok_or_else(|| {
                SeismicStoreError::Message(format!(
                    "{label} array dimension names must not contain null entries"
                ))
            })
        })
        .collect()
}

fn read_axis_values_f64(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
) -> Result<Vec<f64>, SeismicStoreError> {
    let subset = array.subset_all();
    match array.data_type() {
        DataType::Float64 => Ok(array.retrieve_array_subset_elements::<f64>(&subset)?),
        DataType::Float32 => Ok(array
            .retrieve_array_subset_elements::<f32>(&subset)?
            .into_iter()
            .map(f64::from)
            .collect()),
        DataType::Int32 => Ok(array
            .retrieve_array_subset_elements::<i32>(&subset)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),
        DataType::UInt32 => Ok(array
            .retrieve_array_subset_elements::<u32>(&subset)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),
        DataType::Int16 => Ok(array
            .retrieve_array_subset_elements::<i16>(&subset)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),
        DataType::UInt16 => Ok(array
            .retrieve_array_subset_elements::<u16>(&subset)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),
        DataType::Int64 => Ok(array
            .retrieve_array_subset_elements::<i64>(&subset)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),
        DataType::UInt64 => Ok(array
            .retrieve_array_subset_elements::<u64>(&subset)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),
        other => Err(SeismicStoreError::Message(format!(
            "unsupported MDIO axis dtype {:?}",
            other
        ))),
    }
}

fn read_axis_values_f32(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
) -> Result<Vec<f32>, SeismicStoreError> {
    let subset = array.subset_all();
    match array.data_type() {
        DataType::Float32 => Ok(array.retrieve_array_subset_elements::<f32>(&subset)?),
        DataType::Float64 => Ok(array
            .retrieve_array_subset_elements::<f64>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::Int32 => Ok(array
            .retrieve_array_subset_elements::<i32>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::UInt32 => Ok(array
            .retrieve_array_subset_elements::<u32>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::Int16 => Ok(array
            .retrieve_array_subset_elements::<i16>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::UInt16 => Ok(array
            .retrieve_array_subset_elements::<u16>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::Int64 => Ok(array
            .retrieve_array_subset_elements::<i64>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::UInt64 => Ok(array
            .retrieve_array_subset_elements::<u64>(&subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        other => Err(SeismicStoreError::Message(format!(
            "unsupported MDIO axis dtype {:?}",
            other
        ))),
    }
}

fn retrieve_f32_subset(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
    subset: &ArraySubset,
) -> Result<Vec<f32>, SeismicStoreError> {
    match array.data_type() {
        DataType::Float32 => Ok(array.retrieve_array_subset_elements::<f32>(subset)?),
        DataType::Float64 => Ok(array
            .retrieve_array_subset_elements::<f64>(subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::Int16 => Ok(array
            .retrieve_array_subset_elements::<i16>(subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::UInt16 => Ok(array
            .retrieve_array_subset_elements::<u16>(subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::Int32 => Ok(array
            .retrieve_array_subset_elements::<i32>(subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        DataType::UInt32 => Ok(array
            .retrieve_array_subset_elements::<u32>(subset)?
            .into_iter()
            .map(|value| value as f32)
            .collect()),
        other => Err(SeismicStoreError::Message(format!(
            "unsupported MDIO seismic dtype {:?}",
            other
        ))),
    }
}

fn retrieve_mask_subset(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
    subset: &ArraySubset,
) -> Result<Vec<u8>, SeismicStoreError> {
    match array.data_type() {
        DataType::Bool => Ok(array
            .retrieve_array_subset_elements::<bool>(subset)?
            .into_iter()
            .map(u8::from)
            .collect()),
        DataType::UInt8 => Ok(array.retrieve_array_subset_elements::<u8>(subset)?),
        other => Err(SeismicStoreError::Message(format!(
            "unsupported MDIO trace_mask dtype {:?}",
            other
        ))),
    }
}

fn retrieve_scalar_f64(
    array: &Array<dyn ReadableWritableListableStorageTraits>,
    inline_index: usize,
    xline_index: usize,
) -> Result<f64, SeismicStoreError> {
    let subset = ArraySubset::new_with_ranges(&[
        inline_index as u64..inline_index as u64 + 1,
        xline_index as u64..xline_index as u64 + 1,
    ]);
    let values: Vec<f64> = match array.data_type() {
        DataType::Float64 => array
            .retrieve_array_subset_elements::<f64>(&subset)?
            .into_iter()
            .collect(),
        DataType::Float32 => array
            .retrieve_array_subset_elements::<f32>(&subset)?
            .into_iter()
            .map(f64::from)
            .collect(),
        other => {
            return Err(SeismicStoreError::Message(format!(
                "unsupported MDIO coordinate dtype {:?}",
                other
            )));
        }
    };
    values
        .into_iter()
        .next()
        .ok_or_else(|| SeismicStoreError::Message("expected one coordinate sample".to_string()))
}

fn pad_amplitude_tile(tile_shape: [usize; 3], effective: [usize; 3], raw: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; tile_shape[0] * tile_shape[1] * tile_shape[2]];
    for local_i in 0..effective[0] {
        for local_x in 0..effective[1] {
            let src = ((local_i * effective[1]) + local_x) * effective[2];
            let dst = ((local_i * tile_shape[1]) + local_x) * tile_shape[2];
            out[dst..dst + effective[2]].copy_from_slice(&raw[src..src + effective[2]]);
        }
    }
    out
}

fn pad_occupancy_tile(tile_shape: [usize; 3], effective: [usize; 3], raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0_u8; tile_shape[0] * tile_shape[1]];
    for local_i in 0..effective[0] {
        let src = local_i * effective[1];
        let dst = local_i * tile_shape[1];
        out[dst..dst + effective[1]].copy_from_slice(&raw[src..src + effective[1]]);
    }
    out
}

fn infer_sample_axis_domain(axis_name: &str, unit: &str) -> TimeDepthDomain {
    if unit.eq_ignore_ascii_case("ms")
        || unit.eq_ignore_ascii_case("s")
        || axis_name.eq_ignore_ascii_case("time")
    {
        TimeDepthDomain::Time
    } else {
        TimeDepthDomain::Depth
    }
}

fn default_sample_axis_unit(axis_name: &str) -> String {
    if axis_name.eq_ignore_ascii_case("time") {
        "ms".to_string()
    } else {
        "m".to_string()
    }
}

fn axis_unit(attributes: &Map<String, Value>, key: &str) -> Option<String> {
    let units = attributes.get("unitsV1")?.as_object()?;
    units.get(key).and_then(Value::as_str).map(str::to_string)
}

fn read_array_dimensions(path: PathBuf) -> Result<Vec<String>, SeismicStoreError> {
    let attributes = read_json_object(path)?;
    let Some(dimensions) = attributes
        .get("_ARRAY_DIMENSIONS")
        .and_then(Value::as_array)
    else {
        return Err(SeismicStoreError::Message(
            "MDIO array metadata is missing _ARRAY_DIMENSIONS".to_string(),
        ));
    };
    dimensions
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                SeismicStoreError::Message(
                    "MDIO _ARRAY_DIMENSIONS must contain only strings".to_string(),
                )
            })
        })
        .collect()
}

fn read_json_object(path: PathBuf) -> Result<Map<String, Value>, SeismicStoreError> {
    let bytes = fs::read(&path).map_err(|error| {
        SeismicStoreError::Message(format!(
            "failed to read JSON object at {}: {error}",
            path.display()
        ))
    })?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        SeismicStoreError::Message(format!(
            "failed to parse JSON object at {}: {error}",
            path.display()
        ))
    })?;
    value.as_object().cloned().ok_or_else(|| {
        SeismicStoreError::Message(format!("expected JSON object at {}", path.display()))
    })
}

fn try_read_json_object(path: PathBuf) -> Result<Option<Map<String, Value>>, SeismicStoreError> {
    match fs::read(&path) {
        Ok(bytes) => {
            let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
                SeismicStoreError::Message(format!(
                    "failed to parse JSON object at {}: {error}",
                    path.display()
                ))
            })?;
            let object = value.as_object().cloned().ok_or_else(|| {
                SeismicStoreError::Message(format!("expected JSON object at {}", path.display()))
            })?;
            Ok(Some(object))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(SeismicStoreError::Message(format!(
            "failed to read JSON object at {}: {error}",
            path.display()
        ))),
    }
}

fn directory_size_bytes(root: &Path) -> Result<u64, SeismicStoreError> {
    let mut total = 0_u64;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total += directory_size_bytes(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }
    Ok(total)
}

fn open_array_at_path(
    root: &Path,
    path: &str,
) -> Result<Array<dyn ReadableWritableListableStorageTraits>, SeismicStoreError> {
    let store: ReadableWritableListableStorage = Arc::new(
        FilesystemStore::new(root)
            .map_err(|error| SeismicStoreError::Message(error.to_string()))?,
    );
    Array::open(store, path).map_err(|error| SeismicStoreError::Message(error.to_string()))
}

fn try_open_array_at_path(
    root: &Path,
    path: &str,
) -> Result<Option<Array<dyn ReadableWritableListableStorageTraits>>, SeismicStoreError> {
    match open_array_at_path(root, path) {
        Ok(array) => Ok(Some(array)),
        Err(SeismicStoreError::Message(message)) => {
            let lower_message = message.to_ascii_lowercase();
            if lower_message.contains("missing metadata")
                || lower_message.contains("metadata is missing")
                || lower_message.contains("no such file")
            {
                Ok(None)
            } else {
                Err(SeismicStoreError::Message(message))
            }
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use zarrs::array::{ArrayBuilder, DataType};
    use zarrs::group::GroupBuilder;

    use super::*;
    use crate::store::load_array;

    #[test]
    fn looks_like_mdio_path_detects_directory_fixture() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("synthetic.mdio");
        create_test_mdio_fixture(&root).expect("fixture");
        assert!(looks_like_mdio_path(&root));
    }

    #[test]
    fn ingest_mdio_store_imports_subset_and_preserves_axes() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("synthetic.mdio");
        create_test_mdio_fixture(&root).expect("fixture");
        let output = temp.path().join("synthetic.tbvol");

        let handle = ingest_mdio_store(
            &root,
            &output,
            [0, 0, 0],
            Some(VolumeSubset {
                inline_start: 0,
                inline_count: 2,
                xline_start: 1,
                xline_count: 2,
                sample_start: 1,
                sample_count: 3,
            }),
        )
        .expect("ingest mdio subset");

        assert_eq!(handle.manifest.volume.shape, [2, 2, 3]);
        assert_eq!(handle.manifest.volume.axes.ilines, vec![10.0, 11.0]);
        assert_eq!(handle.manifest.volume.axes.xlines, vec![21.0, 22.0]);
        assert_eq!(
            handle.manifest.volume.axes.sample_axis_ms,
            vec![1004.0, 1008.0, 1012.0]
        );
        assert_eq!(
            handle.manifest.volume.axes.sample_axis_domain,
            TimeDepthDomain::Time
        );
        assert_eq!(handle.manifest.volume.axes.sample_axis_unit, "ms");
        assert!(handle.manifest.volume.spatial.is_some());
        assert!(handle.manifest.has_occupancy);

        let amplitudes = load_array(&handle).expect("load amplitudes");
        assert_eq!(amplitudes.shape(), &[2, 2, 3]);
        assert_eq!(amplitudes[[0, 0, 0]], 5.0);
        assert_eq!(amplitudes[[1, 1, 2]], 23.0);
    }

    #[test]
    fn estimate_mdio_tbvol_storage_reports_exact_runtime_footprint() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("synthetic.mdio");
        create_test_mdio_fixture(&root).expect("fixture");

        let estimate =
            estimate_mdio_tbvol_storage(&root, [0, 0, 0], None).expect("estimate mdio storage");

        assert_eq!(estimate.shape, [2, 3, 4]);
        assert_eq!(estimate.tile_shape, [2, 3, 4]);
        assert_eq!(estimate.tile_count, 1);
        assert!(estimate.has_occupancy);
        assert_eq!(
            estimate.amplitude_bytes,
            24 * std::mem::size_of::<f32>() as u64
        );
        assert_eq!(estimate.occupancy_bytes, 6);
        assert_eq!(
            estimate.total_bytes,
            estimate.amplitude_bytes + estimate.occupancy_bytes
        );
    }

    fn create_test_mdio_fixture(root: &Path) -> Result<(), SeismicStoreError> {
        fs::create_dir_all(root)?;
        let store: ReadableWritableListableStorage = Arc::new(
            FilesystemStore::new(root)
                .map_err(|error| SeismicStoreError::Message(error.to_string()))?,
        );
        GroupBuilder::new()
            .attributes(
                serde_json::json!({
                    "apiVersion": "1.0.0a1",
                    "name": "Synthetic",
                    "createdOn": "2026-04-17T00:00:00Z",
                    "attributes": {
                        "processingStage": "post-stack",
                        "surveyDimensionality": "3D"
                    }
                })
                .as_object()
                .expect("object")
                .clone(),
            )
            .build(store.clone(), "/")
            .map_err(|error| SeismicStoreError::Message(error.to_string()))?
            .store_metadata()?;

        create_array_u16(store.clone(), "/inline", "inline", &[10, 11], None)?;
        create_array_u16(
            store.clone(),
            "/crossline",
            "crossline",
            &[20, 21, 22],
            None,
        )?;
        create_array_u16(
            store.clone(),
            "/time",
            "time",
            &[1000, 1004, 1008, 1012],
            Some(serde_json::json!({"unitsV1":{"time":"ms"}})),
        )?;

        let mut seismic =
            ArrayBuilder::new(vec![2, 3, 4], vec![2, 2, 4], DataType::Float32, 0.0_f32)
                .dimension_names(Some(["inline", "crossline", "time"]))
                .build(store.clone(), SEISMIC_ARRAY_PATH)
                .map_err(|error| SeismicStoreError::Message(error.to_string()))?;
        seismic.attributes_mut().insert(
            "coordinates".to_string(),
            Value::String("trace_mask cdp-x cdp-y".to_string()),
        );
        seismic.store_metadata()?;
        seismic.store_array_subset_elements(
            &ArraySubset::new_with_ranges(&[0..2, 0..3, 0..4]),
            &(0..24).map(|value| value as f32).collect::<Vec<_>>(),
        )?;

        let mask = ArrayBuilder::new(vec![2, 3], vec![2, 2], DataType::UInt8, 0_u8)
            .dimension_names(Some(["inline", "crossline"]))
            .build(store.clone(), TRACE_MASK_ARRAY_PATH)
            .map_err(|error| SeismicStoreError::Message(error.to_string()))?;
        mask.store_metadata()?;
        mask.store_array_subset_elements(
            &ArraySubset::new_with_ranges(&[0..2, 0..3]),
            &[1_u8, 1, 0, 1, 1, 1],
        )?;

        create_array_f64(
            store.clone(),
            CDP_X_ARRAY_PATH,
            ["inline", "crossline"],
            &[1000.0, 1002.0, 1004.0, 1010.0, 1012.0, 1014.0],
            Some(serde_json::json!({"unitsV1":{"length":"m"}})),
            [2, 3],
        )?;
        create_array_f64(
            store,
            CDP_Y_ARRAY_PATH,
            ["inline", "crossline"],
            &[2000.0, 2020.0, 2040.0, 2005.0, 2025.0, 2045.0],
            Some(serde_json::json!({"unitsV1":{"length":"m"}})),
            [2, 3],
        )?;

        Ok(())
    }

    fn create_array_u16(
        store: ReadableWritableListableStorage,
        path: &str,
        dimension_name: &str,
        values: &[u16],
        extra_attributes: Option<Value>,
    ) -> Result<(), SeismicStoreError> {
        let mut array = ArrayBuilder::new(
            vec![values.len() as u64],
            vec![values.len().max(1) as u64],
            DataType::UInt16,
            0_u16,
        )
        .dimension_names([dimension_name].into())
        .build(store, path)
        .map_err(|error| SeismicStoreError::Message(error.to_string()))?;
        if let Some(extra) = extra_attributes.and_then(|value| value.as_object().cloned()) {
            array.attributes_mut().extend(extra);
        }
        array.store_metadata()?;
        array.store_array_subset_elements(
            &ArraySubset::new_with_ranges(&[0..values.len() as u64]),
            values,
        )?;
        Ok(())
    }

    fn create_array_f64(
        store: ReadableWritableListableStorage,
        path: &str,
        dimensions: [&str; 2],
        values: &[f64],
        extra_attributes: Option<Value>,
        shape: [u64; 2],
    ) -> Result<(), SeismicStoreError> {
        let mut array = ArrayBuilder::new(
            vec![shape[0], shape[1]],
            vec![shape[0], shape[1]],
            DataType::Float64,
            0.0_f64,
        )
        .dimension_names(Some(dimensions))
        .build(store, path)
        .map_err(|error| SeismicStoreError::Message(error.to_string()))?;
        if let Some(extra) = extra_attributes.and_then(|value| value.as_object().cloned()) {
            array.attributes_mut().extend(extra);
        }
        array.store_metadata()?;
        array.store_array_subset_elements(
            &ArraySubset::new_with_ranges(&[0..shape[0], 0..shape[1]]),
            values,
        )?;
        Ok(())
    }
}
