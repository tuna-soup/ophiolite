use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ndarray::{Array2, Array3};
use ophiolite_seismic_io::{
    ChunkReadConfig, GeometryClassification, GeometryOptions, GeometryReport, HeaderField,
    HeaderMapping, ValidationMode, inspect_file, open,
};

use crate::TileCoord;
use crate::error::SeismicStoreError;
use crate::metadata::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, RegularizationProvenance, SourceIdentity,
    VolumeAxes, VolumeMetadata, generate_store_id, segy_sample_data_fidelity,
};
use crate::openvds::{ingest_openvds_store, looks_like_openvds_path};
use crate::prestack_store::{PrestackStoreHandle, TbgathManifest, create_tbgath_store};
use crate::segy_export::attach_store_segy_export;
use crate::storage::tbvol::{
    TbvolWriter, recommended_default_tbvol_tile_target_mib, recommended_tbvol_tile_shape,
};
use crate::storage::volume_store::{
    VolumeStoreReader, VolumeStoreWriter, read_dense_occupancy, read_dense_volume,
    write_dense_volume,
};
use crate::storage::zarr::ZarrVolumeStoreReader;
use crate::store::{StoreHandle, open_store};
use ophiolite_seismic::{
    CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
    GatherAxisKind, ProjectedPoint2, ProjectedPolygon2, ProjectedVector2, SeismicLayout,
    SeismicStackingState, SurveyGridTransform, SurveySpatialAvailability, SurveySpatialDescriptor,
};

#[derive(Debug, Clone)]
pub struct SourceVolume {
    pub source: SourceIdentity,
    pub axes: VolumeAxes,
    pub coordinate_reference_binding: Option<CoordinateReferenceBinding>,
    pub spatial: Option<SurveySpatialDescriptor>,
    pub data: Array3<f32>,
    pub occupancy: Option<Array2<u8>>,
}

#[derive(Debug, Clone)]
pub struct SeisGeometryOptions {
    pub header_mapping: HeaderMapping,
    pub third_axis_field: Option<HeaderField>,
}

impl Default for SeisGeometryOptions {
    fn default() -> Self {
        Self {
            header_mapping: HeaderMapping::default(),
            third_axis_field: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SparseSurveyPolicy {
    Reject,
    RegularizeToDense { fill_value: f32 },
}

impl Default for SparseSurveyPolicy {
    fn default() -> Self {
        Self::RegularizeToDense { fill_value: 0.0 }
    }
}

#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub chunk_shape: [usize; 3],
    pub validation_mode: ValidationMode,
    pub geometry: SeisGeometryOptions,
    pub sparse_survey_policy: SparseSurveyPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeImportFormat {
    Segy,
    ZarrStore,
    OpenVdsStore,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            chunk_shape: [0, 0, 0],
            validation_mode: ValidationMode::Strict,
            geometry: SeisGeometryOptions::default(),
            sparse_survey_policy: SparseSurveyPolicy::default(),
        }
    }
}

pub fn ingest_segy(
    segy_path: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    let segy_path = segy_path.as_ref();
    let summary = inspect_file(segy_path)?;
    let reader = open(
        segy_path,
        ophiolite_seismic_io::ReaderOptions {
            validation_mode: options.validation_mode,
            header_mapping: options.geometry.header_mapping.clone(),
            ..ophiolite_seismic_io::ReaderOptions::default()
        },
    )?;
    let geometry_report = reader.analyze_geometry(GeometryOptions {
        third_axis_field: options.geometry.third_axis_field,
        ..GeometryOptions::default()
    })?;

    if geometry_report.stacking_state == SeismicStackingState::PreStack {
        return Err(SeismicStoreError::Message(format!(
            "prestack ingest requires ingest_prestack_offset_segy; generic ingest_segy remains post-stack-only, resolved layout {:?}",
            geometry_report.layout
        )));
    }

    if geometry_report.classification == GeometryClassification::RegularSparse
        && matches!(
            options.sparse_survey_policy,
            SparseSurveyPolicy::RegularizeToDense { .. }
        )
    {
        return ingest_sparse_regular_poststack_to_tbvol(
            segy_path,
            store_root,
            &summary,
            &reader,
            &geometry_report,
            &options,
        );
    }

    let volume = load_source_volume_with_options(segy_path, &options)?;
    let shape = [
        volume.data.shape()[0],
        volume.data.shape()[1],
        volume.data.shape()[2],
    ];
    let volume_metadata = VolumeMetadata {
        kind: DatasetKind::Source,
        store_id: generate_store_id(),
        source: volume.source.clone(),
        shape,
        axes: volume.axes.clone(),
        segy_export: None,
        coordinate_reference_binding: volume.coordinate_reference_binding.clone(),
        spatial: volume.spatial.clone(),
        created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
        processing_lineage: None,
    };
    let tile_shape = resolve_chunk_shape(options.chunk_shape, shape);
    let writer = TbvolWriter::create(
        &store_root,
        volume_metadata,
        tile_shape,
        volume.occupancy.is_some(),
    )?;
    write_dense_volume(&writer, &volume.data, volume.occupancy.as_ref())?;
    writer.finalize()?;
    attach_store_segy_export(
        store_root.as_ref(),
        segy_path,
        &summary,
        &reader,
        &geometry_report,
        volume.occupancy.is_some(),
    )?;
    open_store(store_root)
}

pub fn ingest_volume(
    input_path: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    let input_path = normalize_volume_import_path(input_path);
    match detect_volume_import_format(&input_path)? {
        VolumeImportFormat::Segy => ingest_segy(&input_path, store_root, options),
        VolumeImportFormat::ZarrStore => ingest_zarr_store(&input_path, store_root, options),
        VolumeImportFormat::OpenVdsStore => ingest_openvds_store(&input_path, store_root, options),
    }
}

pub fn detect_volume_import_format(
    input_path: impl AsRef<Path>,
) -> Result<VolumeImportFormat, SeismicStoreError> {
    let input_path = normalize_volume_import_path(input_path);
    let extension = input_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("sgy") | Some("segy") => return Ok(VolumeImportFormat::Segy),
        Some("zarr") => return Ok(VolumeImportFormat::ZarrStore),
        Some("vds") => return Ok(VolumeImportFormat::OpenVdsStore),
        _ => {}
    }

    if input_path.is_dir()
        && input_path
            .join(crate::metadata::StoreManifest::FILE_NAME)
            .exists()
    {
        return Ok(VolumeImportFormat::ZarrStore);
    }

    if looks_like_openvds_path(&input_path) {
        return Ok(VolumeImportFormat::OpenVdsStore);
    }

    Err(SeismicStoreError::Message(format!(
        "unsupported volume import format: {}",
        input_path.display()
    )))
}

pub fn normalize_volume_import_path(input_path: impl AsRef<Path>) -> PathBuf {
    let input_path = input_path.as_ref();
    let file_name = input_path.file_name().and_then(|value| value.to_str());
    match file_name.map(|value| value.to_ascii_lowercase()) {
        Some(name) if name == crate::metadata::StoreManifest::FILE_NAME || name == "zarr.json" => {
            input_path.parent().unwrap_or(input_path).to_path_buf()
        }
        _ => input_path.to_path_buf(),
    }
}

pub fn ingest_zarr_store(
    input_root: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    let input_root = normalize_volume_import_path(input_root);
    let reader = ZarrVolumeStoreReader::open(&input_root)?;
    let data = read_dense_volume(&reader)?;
    let occupancy = read_dense_occupancy(&reader)?;
    let shape = [data.shape()[0], data.shape()[1], data.shape()[2]];
    let mut volume_metadata = reader.volume().clone();
    volume_metadata.store_id = generate_store_id();
    volume_metadata.shape = shape;
    volume_metadata.created_by = "ophiolite-seismic-runtime-0.1.0".to_string();
    let tile_shape = resolve_chunk_shape(options.chunk_shape, shape);
    let writer = TbvolWriter::create(
        &store_root,
        volume_metadata,
        tile_shape,
        occupancy.is_some(),
    )?;
    write_dense_volume(&writer, &data, occupancy.as_ref())?;
    writer.finalize()?;
    open_store(store_root)
}

pub fn ingest_prestack_offset_segy(
    segy_path: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<PrestackStoreHandle, SeismicStoreError> {
    let segy_path = segy_path.as_ref();
    let summary = inspect_file(segy_path)?;
    let reader = open(
        segy_path,
        ophiolite_seismic_io::ReaderOptions {
            validation_mode: options.validation_mode,
            header_mapping: options.geometry.header_mapping.clone(),
            ..ophiolite_seismic_io::ReaderOptions::default()
        },
    )?;
    let geometry_report = reader.analyze_geometry(GeometryOptions {
        third_axis_field: options.geometry.third_axis_field,
        ..GeometryOptions::default()
    })?;
    let spatial = derive_survey_spatial_descriptor(&reader, &geometry_report)?;
    let coordinate_reference_binding = coordinate_reference_binding_from_spatial(spatial.as_ref());

    validate_prestack_offset_geometry(&geometry_report, &options)?;
    let cube = reader.assemble_cube()?;
    let shape = [cube.ilines.len(), cube.xlines.len(), cube.samples_per_trace];
    let source = build_source_identity(
        segy_path,
        &summary,
        cube.samples_per_trace,
        cube.sample_interval_us,
        &geometry_report,
        None,
    );
    let axes = VolumeAxes {
        ilines: cube.ilines.iter().map(|value| *value as f64).collect(),
        xlines: cube.xlines.iter().map(|value| *value as f64).collect(),
        sample_axis_ms: cube.sample_axis_ms.clone(),
    };
    let manifest = TbgathManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source,
            shape,
            axes,
            segy_export: None,
            coordinate_reference_binding,
            spatial,
            created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
            processing_lineage: None,
        },
        SeismicLayout::PreStack3DOffset,
        GatherAxisKind::Offset,
        cube.offsets.into_iter().map(|value| value as f64).collect(),
    );
    create_tbgath_store(store_root, manifest, &cube.data)
}

pub fn load_source_volume(
    segy_path: impl AsRef<Path>,
    validation_mode: ValidationMode,
) -> Result<SourceVolume, SeismicStoreError> {
    load_source_volume_with_options(
        segy_path,
        &IngestOptions {
            validation_mode,
            ..IngestOptions::default()
        },
    )
}

pub fn load_source_volume_with_options(
    segy_path: impl AsRef<Path>,
    options: &IngestOptions,
) -> Result<SourceVolume, SeismicStoreError> {
    let segy_path = segy_path.as_ref();
    let summary = inspect_file(segy_path)?;
    let reader = open(
        segy_path,
        ophiolite_seismic_io::ReaderOptions {
            validation_mode: options.validation_mode,
            header_mapping: options.geometry.header_mapping.clone(),
            ..ophiolite_seismic_io::ReaderOptions::default()
        },
    )?;
    let geometry_report = reader.analyze_geometry(GeometryOptions {
        third_axis_field: options.geometry.third_axis_field,
        ..GeometryOptions::default()
    })?;
    let spatial = derive_survey_spatial_descriptor(&reader, &geometry_report)?;
    let coordinate_reference_binding = coordinate_reference_binding_from_spatial(spatial.as_ref());

    match geometry_report.classification {
        GeometryClassification::RegularDense => {
            let cube = reader.assemble_cube()?;
            if cube.offsets.len() != 1 {
                return Err(SeismicStoreError::UnsupportedOffsetCount {
                    offset_count: cube.offsets.len(),
                });
            }

            let shape = [cube.ilines.len(), cube.xlines.len(), cube.samples_per_trace];
            let data = Array3::from_shape_vec(shape, cube.data)?;
            Ok(SourceVolume {
                source: build_source_identity(
                    segy_path,
                    &summary,
                    cube.samples_per_trace,
                    cube.sample_interval_us,
                    &geometry_report,
                    None,
                ),
                axes: VolumeAxes {
                    ilines: cube.ilines.into_iter().map(|value| value as f64).collect(),
                    xlines: cube.xlines.into_iter().map(|value| value as f64).collect(),
                    sample_axis_ms: cube.sample_axis_ms,
                },
                coordinate_reference_binding,
                spatial,
                data,
                occupancy: None,
            })
        }
        GeometryClassification::RegularSparse => regularize_sparse_regular_poststack(
            segy_path,
            &summary,
            &reader,
            &geometry_report,
            options,
        ),
        _ => Err(unsupported_geometry_error(&geometry_report)),
    }
}

fn regularize_sparse_regular_poststack(
    segy_path: &Path,
    summary: &ophiolite_seismic_io::FileSummary,
    reader: &ophiolite_seismic_io::SegyReader,
    geometry_report: &GeometryReport,
    options: &IngestOptions,
) -> Result<SourceVolume, SeismicStoreError> {
    let SparseSurveyPolicy::RegularizeToDense { fill_value } = options.sparse_survey_policy else {
        return Err(unsupported_geometry_error(geometry_report));
    };

    if options.geometry.third_axis_field.is_some() || !geometry_report.third_axis_values.is_empty()
    {
        return Err(SeismicStoreError::UnsupportedRegularizationTarget);
    }

    let headers = reader.load_trace_headers(
        &[
            geometry_report.inline_field,
            geometry_report.crossline_field,
        ],
        ophiolite_seismic_io::TraceSelection::All,
    )?;
    let ilines = headers
        .column(geometry_report.inline_field)
        .expect("geometry analysis validated inline field");
    let xlines = headers
        .column(geometry_report.crossline_field)
        .expect("geometry analysis validated crossline field");
    let traces = reader.read_all_traces(ChunkReadConfig::default())?;
    let samples_per_trace = traces.samples_per_trace;

    let shape = [
        geometry_report.inline_values.len(),
        geometry_report.crossline_values.len(),
        samples_per_trace,
    ];
    let mut data = vec![fill_value; shape[0] * shape[1] * shape[2]];
    let mut occupancy = Array2::<u8>::zeros((shape[0], shape[1]));
    let inline_lookup = index_lookup(&geometry_report.inline_values);
    let xline_lookup = index_lookup(&geometry_report.crossline_values);

    for trace_index in 0..headers.rows() {
        let inline_index = inline_lookup[&ilines[trace_index]];
        let xline_index = xline_lookup[&xlines[trace_index]];
        let dst_start = (inline_index * shape[1] + xline_index) * shape[2];
        let dst_end = dst_start + shape[2];
        data[dst_start..dst_end].copy_from_slice(traces.trace(trace_index));
        occupancy[[inline_index, xline_index]] = 1;
    }

    let spatial = derive_survey_spatial_descriptor(reader, geometry_report)?;
    let coordinate_reference_binding = coordinate_reference_binding_from_spatial(spatial.as_ref());

    Ok(SourceVolume {
        source: build_source_identity(
            segy_path,
            summary,
            samples_per_trace,
            reader.resolved_sample_interval_us(),
            geometry_report,
            Some(RegularizationProvenance {
                source_classification: geometry_classification_label(
                    geometry_report.classification,
                )
                .to_string(),
                fill_value,
                observed_trace_count: geometry_report.observed_trace_count,
                expected_trace_count: geometry_report.expected_trace_count,
                missing_bin_count: geometry_report.missing_bin_count,
            }),
        ),
        axes: VolumeAxes {
            ilines: geometry_report
                .inline_values
                .iter()
                .map(|value| *value as f64)
                .collect(),
            xlines: geometry_report
                .crossline_values
                .iter()
                .map(|value| *value as f64)
                .collect(),
            sample_axis_ms: reader.sample_axis_ms(),
        },
        coordinate_reference_binding,
        spatial,
        data: Array3::from_shape_vec(shape, data)?,
        occupancy: Some(occupancy),
    })
}

fn ingest_sparse_regular_poststack_to_tbvol(
    segy_path: &Path,
    store_root: impl AsRef<Path>,
    summary: &ophiolite_seismic_io::FileSummary,
    reader: &ophiolite_seismic_io::SegyReader,
    geometry_report: &GeometryReport,
    options: &IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    let SparseSurveyPolicy::RegularizeToDense { fill_value } = options.sparse_survey_policy else {
        return Err(unsupported_geometry_error(geometry_report));
    };

    if options.geometry.third_axis_field.is_some() || !geometry_report.third_axis_values.is_empty()
    {
        return Err(SeismicStoreError::UnsupportedRegularizationTarget);
    }

    let shape = [
        geometry_report.inline_values.len(),
        geometry_report.crossline_values.len(),
        summary.samples_per_trace as usize,
    ];
    let source = build_source_identity(
        segy_path,
        summary,
        shape[2],
        reader.resolved_sample_interval_us(),
        geometry_report,
        Some(RegularizationProvenance {
            source_classification: geometry_classification_label(geometry_report.classification)
                .to_string(),
            fill_value,
            observed_trace_count: geometry_report.observed_trace_count,
            expected_trace_count: geometry_report.expected_trace_count,
            missing_bin_count: geometry_report.missing_bin_count,
        }),
    );
    let axes = VolumeAxes {
        ilines: geometry_report
            .inline_values
            .iter()
            .map(|value| *value as f64)
            .collect(),
        xlines: geometry_report
            .crossline_values
            .iter()
            .map(|value| *value as f64)
            .collect(),
        sample_axis_ms: reader.sample_axis_ms(),
    };
    let spatial = derive_survey_spatial_descriptor(reader, geometry_report)?;
    let volume_metadata = VolumeMetadata {
        kind: DatasetKind::Source,
        store_id: generate_store_id(),
        source,
        shape,
        axes,
        segy_export: None,
        coordinate_reference_binding: coordinate_reference_binding_from_spatial(spatial.as_ref()),
        spatial,
        created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
        processing_lineage: None,
    };
    let tile_shape = resolve_chunk_shape(options.chunk_shape, shape);
    let writer = TbvolWriter::create(&store_root, volume_metadata, tile_shape, true)?;
    let geometry = writer.tile_geometry().clone();
    let tile_shape = geometry.tile_shape();

    let headers = reader.load_trace_headers(
        &[
            geometry_report.inline_field,
            geometry_report.crossline_field,
        ],
        ophiolite_seismic_io::TraceSelection::All,
    )?;
    let ilines = headers
        .column(geometry_report.inline_field)
        .expect("geometry analysis validated inline field");
    let xlines = headers
        .column(geometry_report.crossline_field)
        .expect("geometry analysis validated crossline field");
    let inline_lookup = index_lookup(&geometry_report.inline_values);
    let xline_lookup = index_lookup(&geometry_report.crossline_values);

    let mut amplitude_map = writer.map_amplitude_mut()?;
    let amplitudes = amplitude_map_as_f32_slice(&mut amplitude_map)?;
    if fill_value != 0.0 {
        amplitudes.fill(fill_value);
    }
    let mut occupancy_map = writer.map_occupancy_mut()?;
    if let Some(mask) = occupancy_map.as_mut() {
        mask.fill(0);
    }

    let chunk = ChunkReadConfig::default();
    let mut scratch = vec![0.0_f32; chunk.traces_per_chunk * shape[2]];
    reader
        .process_trace_chunks_into(chunk, &mut scratch, |trace_chunk| {
            for local_trace in 0..trace_chunk.trace_count {
                let trace_index = trace_chunk.start_trace as usize + local_trace;
                let inline_index = inline_lookup[&ilines[trace_index]];
                let xline_index = xline_lookup[&xlines[trace_index]];
                let tile = TileCoord {
                    tile_i: inline_index / tile_shape[0],
                    tile_x: xline_index / tile_shape[1],
                };
                let local_i = inline_index % tile_shape[0];
                let local_x = xline_index % tile_shape[1];
                let tile_trace_index = (local_i * tile_shape[1]) + local_x;
                let dst_trace_start = (geometry.amplitude_offset(tile) as usize
                    / std::mem::size_of::<f32>())
                    + tile_trace_index * tile_shape[2];
                let src_trace_start = local_trace * trace_chunk.samples_per_trace;
                let src_trace_end = src_trace_start + trace_chunk.samples_per_trace;
                amplitudes[dst_trace_start..dst_trace_start + trace_chunk.samples_per_trace]
                    .copy_from_slice(&trace_chunk.data[src_trace_start..src_trace_end]);

                if let Some(mask) = occupancy_map.as_mut() {
                    let occupancy_index =
                        geometry.occupancy_offset(tile) as usize + tile_trace_index;
                    mask[occupancy_index] = 1;
                }
            }
            Ok::<(), SeismicStoreError>(())
        })
        .map_err(|error| match error {
            ophiolite_seismic_io::ChunkProcessingError::Read(error) => {
                SeismicStoreError::SeisIoRead(error)
            }
            ophiolite_seismic_io::ChunkProcessingError::Sink(error) => error,
        })?;

    amplitude_map.flush()?;
    drop(amplitude_map);
    if let Some(mask) = occupancy_map.as_mut() {
        mask.flush()?;
    }
    drop(occupancy_map);
    writer.finalize()?;
    attach_store_segy_export(
        store_root.as_ref(),
        segy_path,
        summary,
        reader,
        geometry_report,
        true,
    )?;
    open_store(store_root)
}

fn build_source_identity(
    segy_path: &Path,
    summary: &ophiolite_seismic_io::FileSummary,
    samples_per_trace: usize,
    sample_interval_us: u16,
    geometry_report: &GeometryReport,
    regularization: Option<RegularizationProvenance>,
) -> SourceIdentity {
    SourceIdentity {
        source_path: segy_path.to_path_buf(),
        file_size: summary.file_size,
        trace_count: summary.trace_count,
        samples_per_trace,
        sample_interval_us,
        sample_format_code: summary.sample_format_code,
        sample_data_fidelity: segy_sample_data_fidelity(summary.sample_format_code),
        endianness: match summary.endianness {
            ophiolite_seismic_io::Endianness::Big => "big".to_string(),
            ophiolite_seismic_io::Endianness::Little => "little".to_string(),
        },
        revision_raw: summary.revision_raw,
        fixed_length_trace_flag_raw: summary.fixed_length_trace_flag_raw,
        extended_textual_headers: summary.extended_textual_headers,
        geometry: GeometryProvenance {
            inline_field: header_field_spec(geometry_report.inline_field),
            crossline_field: header_field_spec(geometry_report.crossline_field),
            third_axis_field: geometry_report.third_axis_field.map(header_field_spec),
        },
        regularization,
    }
}

fn derive_survey_spatial_descriptor(
    reader: &ophiolite_seismic_io::SegyReader,
    geometry_report: &GeometryReport,
) -> Result<Option<SurveySpatialDescriptor>, SeismicStoreError> {
    if matches!(
        geometry_report.classification,
        GeometryClassification::NonCartesian
    ) {
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: None,
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes: vec![String::from(
                "survey geometry is non-cartesian under the resolved inline/xline mapping",
            )],
        }));
    }

    let mapping = reader.header_mapping();
    let cdp_x_field = mapping.cdp_x();
    let cdp_y_field = mapping.cdp_y();
    let headers = reader.load_trace_headers(
        &[
            geometry_report.inline_field,
            geometry_report.crossline_field,
            cdp_x_field,
            cdp_y_field,
            HeaderField::SOURCE_GROUP_SCALAR,
            HeaderField::COORDINATE_UNITS,
        ],
        ophiolite_seismic_io::TraceSelection::All,
    )?;

    let ilines = headers
        .column(geometry_report.inline_field)
        .expect("geometry analysis validated inline field");
    let xlines = headers
        .column(geometry_report.crossline_field)
        .expect("geometry analysis validated crossline field");
    let cdp_x = headers
        .column(cdp_x_field)
        .expect("trace header extraction requested CDP_X");
    let cdp_y = headers
        .column(cdp_y_field)
        .expect("trace header extraction requested CDP_Y");
    let scalars = headers
        .column(HeaderField::SOURCE_GROUP_SCALAR)
        .expect("trace header extraction requested SOURCE_GROUP_SCALAR");
    let coordinate_units = headers
        .column(HeaderField::COORDINATE_UNITS)
        .expect("trace header extraction requested COORDINATE_UNITS");

    if cdp_x.iter().all(|value| *value == 0) && cdp_y.iter().all(|value| *value == 0) {
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: None,
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes: vec![String::from(
                "CDP_X and CDP_Y trace headers are zero for the resolved survey",
            )],
        }));
    }

    let coordinate_units_code = dominant_i16_value(coordinate_units);
    let mut notes = Vec::new();
    let unit = coordinate_unit_label(coordinate_units_code);
    let planar_coordinates = coordinate_units_code == Some(1);
    if coordinate_units_code.is_none() || coordinate_units_code == Some(0) {
        notes.push(String::from(
            "coordinate units are not declared in SEG-Y trace headers",
        ));
    } else if !planar_coordinates {
        notes.push(format!(
            "coordinate units code {:?} is not a planar projected coordinate system",
            coordinate_units_code
        ));
    }

    let inline_lookup = index_lookup(&geometry_report.inline_values);
    let xline_lookup = index_lookup(&geometry_report.crossline_values);
    let mut observations = Vec::with_capacity(headers.rows());

    for trace_index in 0..headers.rows() {
        let scalar = segy_coordinate_scalar(scalars[trace_index]);
        let x = cdp_x[trace_index] as f64 * scalar;
        let y = cdp_y[trace_index] as f64 * scalar;
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        if x == 0.0 && y == 0.0 {
            continue;
        }
        let inline_index = inline_lookup[&ilines[trace_index]] as f64;
        let xline_index = xline_lookup[&xlines[trace_index]] as f64;
        observations.push((inline_index, xline_index, x, y));
    }

    if observations.len() < 3 {
        notes.push(String::from(
            "not enough non-zero trace-coordinate samples were available to derive a survey map transform",
        ));
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: None,
                name: None,
                geodetic_datum: None,
                unit: unit.map(str::to_owned),
            }),
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes,
        }));
    }

    if !planar_coordinates {
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: None,
                name: None,
                geodetic_datum: None,
                unit: unit.map(str::to_owned),
            }),
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes,
        }));
    }

    let Some((origin, inline_basis, xline_basis)) = fit_grid_transform(&observations) else {
        notes.push(String::from(
            "trace-coordinate samples do not support a stable affine grid transform",
        ));
        return Ok(Some(SurveySpatialDescriptor {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: None,
                name: None,
                geodetic_datum: None,
                unit: unit.map(str::to_owned),
            }),
            grid_transform: None,
            footprint: None,
            availability: SurveySpatialAvailability::Unavailable,
            notes,
        }));
    };

    notes.push(String::from(
        "survey map geometry was derived from CDP_X/CDP_Y trace headers with SCALCO applied",
    ));
    notes.push(String::from(
        "coordinate reference system identity remains unresolved because SEG-Y ingest does not yet capture a canonical CRS identifier",
    ));

    let inline_extent = geometry_report.inline_values.len().saturating_sub(1) as f64;
    let xline_extent = geometry_report.crossline_values.len().saturating_sub(1) as f64;
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
            unit: unit.map(str::to_owned),
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

fn coordinate_reference_binding_from_spatial(
    spatial: Option<&SurveySpatialDescriptor>,
) -> Option<CoordinateReferenceBinding> {
    let detected = spatial?.coordinate_reference.clone()?;
    Some(CoordinateReferenceBinding {
        detected: Some(detected.clone()),
        effective: Some(detected),
        source: CoordinateReferenceSource::Header,
        notes: Vec::new(),
    })
}

fn dominant_i16_value(values: &[i64]) -> Option<i16> {
    let mut counts = HashMap::<i16, usize>::new();
    for value in values {
        let entry = counts.entry(*value as i16).or_default();
        *entry += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(value, _)| value)
}

fn coordinate_unit_label(code: Option<i16>) -> Option<&'static str> {
    match code {
        Some(1) => Some("length"),
        Some(2) => Some("seconds_of_arc"),
        _ => None,
    }
}

fn segy_coordinate_scalar(raw_value: i64) -> f64 {
    match raw_value as i16 {
        0 => 1.0,
        value if value < 0 => 1.0 / f64::from(-value),
        value => f64::from(value),
    }
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

fn validate_prestack_offset_geometry(
    geometry_report: &GeometryReport,
    options: &IngestOptions,
) -> Result<(), SeismicStoreError> {
    if geometry_report.stacking_state != SeismicStackingState::PreStack {
        return Err(SeismicStoreError::Message(format!(
            "prestack offset ingest expected a prestack survey, found {:?}",
            geometry_report.stacking_state
        )));
    }
    if geometry_report.layout != SeismicLayout::PreStack3DOffset {
        return Err(SeismicStoreError::Message(format!(
            "phase-one prestack ingest only supports {:?}, found {:?}",
            SeismicLayout::PreStack3DOffset,
            geometry_report.layout
        )));
    }
    if geometry_report.classification != GeometryClassification::RegularDense {
        return Err(SeismicStoreError::Message(format!(
            "phase-one prestack ingest requires regular dense geometry, found {:?}",
            geometry_report.classification
        )));
    }
    if !matches!(options.sparse_survey_policy, SparseSurveyPolicy::Reject)
        && geometry_report.classification == GeometryClassification::RegularSparse
    {
        return Err(SeismicStoreError::Message(
            "prestack sparse regularization is not implemented in phase one".to_string(),
        ));
    }
    if options.geometry.third_axis_field.is_none() {
        return Err(SeismicStoreError::Message(
            "prestack offset ingest requires resolving the third-axis header field explicitly"
                .to_string(),
        ));
    }
    Ok(())
}

fn unsupported_geometry_error(report: &GeometryReport) -> SeismicStoreError {
    SeismicStoreError::UnsupportedSurveyGeometry {
        classification: report.classification,
        observed_trace_count: report.observed_trace_count,
        expected_trace_count: report.expected_trace_count,
        missing_bin_count: report.missing_bin_count,
        duplicate_coordinate_count: report.duplicate_coordinate_count,
    }
}

pub(crate) fn geometry_classification_label(
    classification: GeometryClassification,
) -> &'static str {
    match classification {
        GeometryClassification::RegularDense => "regular_dense",
        GeometryClassification::RegularSparse => "regular_sparse",
        GeometryClassification::DuplicateCoordinates => "duplicate_coordinates",
        GeometryClassification::NonCartesian => "non_cartesian",
        GeometryClassification::AmbiguousMapping => "ambiguous_mapping",
    }
}

pub fn recommended_chunk_shape(shape: [usize; 3], chunk_target_mib: u16) -> [usize; 3] {
    let [ilines, xlines, samples] = shape;
    let bytes_per_trace = (samples.max(1) * std::mem::size_of::<f32>()) as u64;
    let target_bytes = chunk_target_mib as u64 * 1024 * 1024;
    let trace_budget = (target_bytes / bytes_per_trace).max(1) as usize;
    let max_traces = ilines.max(1) * xlines.max(1);
    let trace_budget = trace_budget.min(max_traces);

    if trace_budget >= max_traces {
        return [ilines.max(1), xlines.max(1), samples.max(1)];
    }

    let ilines_f = ilines.max(1) as f64;
    let xlines_f = xlines.max(1) as f64;
    let ratio = (ilines_f / xlines_f).sqrt();
    let mut ci = ((trace_budget as f64).sqrt() * ratio).floor() as usize;
    ci = ci.clamp(1, ilines.max(1));
    let mut cx = (trace_budget / ci).max(1);
    cx = cx.clamp(1, xlines.max(1));

    while ci < ilines && ci * cx < trace_budget {
        ci += 1;
        if ci * cx > trace_budget {
            ci -= 1;
            break;
        }
    }
    while cx < xlines && ci * cx < trace_budget {
        cx += 1;
        if ci * cx > trace_budget {
            cx -= 1;
            break;
        }
    }

    [ci.max(1), cx.max(1), samples.max(1)]
}

fn resolve_chunk_shape(chunk_shape: [usize; 3], shape: [usize; 3]) -> [usize; 3] {
    if chunk_shape.iter().all(|value| *value == 0) {
        return recommended_tbvol_tile_shape(
            shape,
            recommended_default_tbvol_tile_target_mib(shape),
        );
    }

    [
        chunk_shape[0].max(1).min(shape[0].max(1)),
        chunk_shape[1].max(1).min(shape[1].max(1)),
        chunk_shape[2].max(1).min(shape[2].max(1)),
    ]
}

pub(crate) fn header_field_spec(field: HeaderField) -> HeaderFieldSpec {
    HeaderFieldSpec {
        name: field.name.to_string(),
        start_byte: field.start_byte,
        value_type: format!("{:?}", field.value_type),
    }
}

fn index_lookup(values: &[i64]) -> HashMap<i64, usize> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| (*value, index))
        .collect()
}

fn amplitude_map_as_f32_slice(map: &mut memmap2::MmapMut) -> Result<&mut [f32], SeismicStoreError> {
    if map.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "tbvol amplitude byte length is not f32 aligned: {}",
            map.len()
        )));
    }
    let (prefix, aligned, suffix) = unsafe { map.align_to_mut::<f32>() };
    if !prefix.is_empty() || !suffix.is_empty() {
        return Err(SeismicStoreError::Message(
            "tbvol amplitude mapping is not aligned to f32".to_string(),
        ));
    }
    Ok(aligned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{DatasetId, GatherRequest, GatherSelector};
    use tempfile::tempdir;

    fn prestack_fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../TraceBoost/test-data/small-ps.sgy")
    }

    fn zarr_fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../TraceBoost/test-data/survey.zarr")
    }

    #[test]
    fn ingest_prestack_offset_segy_builds_offset_gather_store_when_fixture_is_available() {
        let fixture = prestack_fixture_path();
        if !fixture.exists() {
            return;
        }

        let temp_dir = tempdir().expect("temp dir");
        let output_root = temp_dir.path().join("small-ps.tbgath");
        let handle = ingest_prestack_offset_segy(
            &fixture,
            &output_root,
            IngestOptions {
                geometry: SeisGeometryOptions {
                    third_axis_field: Some(HeaderField::OFFSET),
                    ..SeisGeometryOptions::default()
                },
                sparse_survey_policy: SparseSurveyPolicy::Reject,
                ..IngestOptions::default()
            },
        )
        .expect("fixture should ingest as prestack offset store");

        assert_eq!(handle.manifest.layout, SeismicLayout::PreStack3DOffset);
        assert_eq!(handle.manifest.gather_axis_kind, GatherAxisKind::Offset);
        assert!(!handle.manifest.gather_axis_values.is_empty());

        let gather = handle
            .read_gather_plane(&GatherRequest {
                dataset_id: DatasetId("small-ps.tbgath".to_string()),
                selector: GatherSelector::Ordinal { index: 0 },
            })
            .expect("first prestack gather should be readable");
        assert_eq!(gather.traces, handle.manifest.gather_axis_values.len());
        assert_eq!(gather.samples, handle.manifest.volume.shape[2]);
    }

    #[test]
    fn detect_volume_import_format_handles_zarr_fixture_and_manifest_file() {
        let fixture = zarr_fixture_path();
        if !fixture.exists() {
            return;
        }

        assert_eq!(
            detect_volume_import_format(&fixture).expect("zarr store should be detected"),
            VolumeImportFormat::ZarrStore
        );
        assert_eq!(
            detect_volume_import_format(fixture.join("seisrefine.manifest.json"))
                .expect("zarr manifest should resolve to store root"),
            VolumeImportFormat::ZarrStore
        );
    }

    #[test]
    fn detect_volume_import_format_handles_openvds_extension() {
        assert_eq!(
            detect_volume_import_format("synthetic.vds").expect("vds should be detected"),
            VolumeImportFormat::OpenVdsStore
        );
    }

    #[test]
    fn ingest_volume_reports_explicit_openvds_boundary_error() {
        let temp_dir = tempdir().expect("temp dir");
        let input = temp_dir.path().join("synthetic.vds");
        std::fs::write(&input, b"placeholder").expect("placeholder vds file");
        let output_root = temp_dir.path().join("synthetic.tbvol");

        let error = ingest_volume(&input, &output_root, IngestOptions::default())
            .expect_err("openvds scaffold should fail fast");
        let message = error.to_string();
        assert!(message.contains("OpenVDS import is not wired"));
        assert!(message.contains("synthetic.vds"));
    }

    #[test]
    fn ingest_volume_imports_zarr_store_fixture_to_tbvol_when_available() {
        let fixture = zarr_fixture_path();
        if !fixture.exists() {
            return;
        }

        let temp_dir = tempdir().expect("temp dir");
        let output_root = temp_dir.path().join("survey.tbvol");
        let handle =
            ingest_volume(&fixture, &output_root, IngestOptions::default()).expect("zarr ingest");

        assert_eq!(handle.manifest.volume.shape, [23, 18, 75]);
        assert_eq!(handle.manifest.tile_shape[2], 75);
        assert_eq!(handle.manifest.volume.axes.ilines.len(), 23);
        assert_eq!(handle.manifest.volume.axes.xlines.len(), 18);
    }

    #[test]
    fn segy_coordinate_scalar_applies_segy_convention() {
        assert_eq!(segy_coordinate_scalar(0), 1.0);
        assert_eq!(segy_coordinate_scalar(10), 10.0);
        assert!((segy_coordinate_scalar(-100) - 0.01).abs() < 1e-12);
    }

    #[test]
    fn fit_grid_transform_recovers_affine_survey_geometry() {
        let observations = vec![
            (0.0, 0.0, 1000.0, 2000.0),
            (1.0, 0.0, 1010.0, 2005.0),
            (0.0, 1.0, 998.0, 2020.0),
            (1.0, 1.0, 1008.0, 2025.0),
        ];

        let (origin, inline_basis, xline_basis) =
            fit_grid_transform(&observations).expect("transform should fit");

        assert!((origin.x - 1000.0).abs() < 1e-6);
        assert!((origin.y - 2000.0).abs() < 1e-6);
        assert!((inline_basis.x - 10.0).abs() < 1e-6);
        assert!((inline_basis.y - 5.0).abs() < 1e-6);
        assert!((xline_basis.x + 2.0).abs() < 1e-6);
        assert!((xline_basis.y - 20.0).abs() < 1e-6);
    }

    #[test]
    fn affine_grid_point_builds_expected_corner() {
        let point = affine_grid_point(
            &ProjectedPoint2 {
                x: 1000.0,
                y: 2000.0,
            },
            &ProjectedVector2 { x: 10.0, y: 5.0 },
            &ProjectedVector2 { x: -2.0, y: 20.0 },
            1.0,
            1.0,
        );

        assert!((point.x - 1008.0).abs() < 1e-6);
        assert!((point.y - 2025.0).abs() < 1e-6);
    }
}
