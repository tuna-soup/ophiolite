use std::fs;
use std::path::{Path, PathBuf};

use ndarray::{Array2, Array3};
use ophiolite_seismic::{
    AxisSummaryF32, AxisSummaryI32, CoordinateReferenceBinding, CoordinateReferenceDescriptor,
    CoordinateReferenceSource, DatasetId, GeometryDescriptor, GeometryProvenanceSummary,
    GeometrySummary, ProcessingLineageSummary, SectionAxis, SectionColorMap, SectionCoordinate,
    SectionDisplayDefaults, SectionMetadata, SectionPolarity, SectionRenderMode, SectionUnits,
    SectionView, SurveySpatialDescriptor, VolumeDescriptor,
};

use crate::error::SeismicStoreError;
use crate::metadata::DatasetKind;
use crate::storage::section_assembler;
use crate::storage::tbvol::{TbvolManifest, TbvolReader, TbvolWriter, load_tbvol_manifest};
use crate::storage::volume_store::{
    VolumeStoreWriter, read_dense_occupancy, read_dense_volume, write_dense_volume,
};

const GEOMETRY_COMPARE_FAMILY: &str = "seismic-grid:v1";
const GEOMETRY_FINGERPRINT_VERSION: &str = "geom:v1";

#[derive(Debug, Clone)]
pub struct StoreHandle {
    pub root: PathBuf,
    pub manifest: TbvolManifest,
}

#[derive(Debug, Clone)]
pub struct SectionPlane {
    pub axis: SectionAxis,
    pub coordinate_index: usize,
    pub coordinate_value: f64,
    pub traces: usize,
    pub samples: usize,
    pub horizontal_axis: Vec<f64>,
    pub sample_axis_ms: Vec<f32>,
    pub amplitudes: Vec<f32>,
    pub occupancy: Option<Vec<u8>>,
}

impl StoreHandle {
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    pub fn dataset_id(&self) -> DatasetId {
        DatasetId(dataset_id_string(&self.root))
    }

    pub fn volume_descriptor(&self) -> VolumeDescriptor {
        VolumeDescriptor {
            id: self.dataset_id(),
            store_id: self.manifest.volume.store_id.clone(),
            label: dataset_label(&self.root),
            shape: self.manifest.volume.shape,
            chunk_shape: self.manifest.tile_shape,
            sample_interval_ms: self.manifest.volume.source.sample_interval_us as f32 / 1000.0,
            sample_data_fidelity: self.manifest.volume.source.sample_data_fidelity.clone(),
            geometry: self.geometry_descriptor(),
            coordinate_reference_binding: self.manifest.volume.coordinate_reference_binding.clone(),
            spatial: self.manifest.volume.spatial.clone(),
            processing_lineage_summary: processing_lineage_summary(
                self.manifest.volume.processing_lineage.as_ref(),
            ),
        }
    }

    pub fn section_view(
        &self,
        axis: SectionAxis,
        index: usize,
    ) -> Result<SectionView, SeismicStoreError> {
        let plane = self.read_section_plane(axis, index)?;
        Ok(self.section_view_from_plane(&plane))
    }

    pub fn read_section_plane(
        &self,
        axis: SectionAxis,
        index: usize,
    ) -> Result<SectionPlane, SeismicStoreError> {
        let reader = TbvolReader::open(&self.root)?;
        section_assembler::read_section_plane(&reader, axis, index)
    }

    pub fn section_view_from_plane(&self, plane: &SectionPlane) -> SectionView {
        SectionView {
            dataset_id: self.dataset_id(),
            axis: plane.axis,
            coordinate: SectionCoordinate {
                index: plane.coordinate_index,
                value: plane.coordinate_value,
            },
            traces: plane.traces,
            samples: plane.samples,
            horizontal_axis_f64le: f64_vec_to_le_bytes(&plane.horizontal_axis),
            inline_axis_f64le: None,
            xline_axis_f64le: None,
            sample_axis_f32le: f32_vec_to_le_bytes(&plane.sample_axis_ms),
            amplitudes_f32le: f32_vec_to_le_bytes(&plane.amplitudes),
            units: Some(SectionUnits {
                horizontal: Some(match plane.axis {
                    SectionAxis::Inline => "xline".to_string(),
                    SectionAxis::Xline => "inline".to_string(),
                }),
                sample: Some("ms".to_string()),
                amplitude: Some("amplitude".to_string()),
            }),
            metadata: Some(SectionMetadata {
                store_id: Some(self.manifest.volume.store_id.clone()),
                derived_from: self
                    .manifest
                    .volume
                    .processing_lineage
                    .as_ref()
                    .map(|lineage| lineage.parent_store.to_string_lossy().into_owned()),
                notes: vec![format!("kind:{:?}", self.manifest.volume.kind)],
            }),
            display_defaults: Some(SectionDisplayDefaults {
                gain: 1.0,
                clip_min: None,
                clip_max: None,
                render_mode: SectionRenderMode::Heatmap,
                colormap: SectionColorMap::Grayscale,
                polarity: SectionPolarity::Normal,
            }),
        }
    }

    fn geometry_descriptor(&self) -> GeometryDescriptor {
        GeometryDescriptor {
            compare_family: GEOMETRY_COMPARE_FAMILY.to_string(),
            fingerprint: geometry_fingerprint(&self.manifest),
            summary: GeometrySummary {
                inline_axis: summarize_i32_axis(&self.manifest.volume.axes.ilines),
                xline_axis: summarize_i32_axis(&self.manifest.volume.axes.xlines),
                sample_axis: summarize_f32_axis(&self.manifest.volume.axes.sample_axis_ms),
                layout: None,
                gather_axis_kind: None,
                gather_axis: None,
                provenance: geometry_provenance_summary(&self.manifest),
            },
        }
    }
}

fn processing_lineage_summary(
    lineage: Option<&crate::metadata::ProcessingLineage>,
) -> Option<ProcessingLineageSummary> {
    let lineage = lineage?;
    let (pipeline_name, pipeline_revision) = match &lineage.pipeline {
        ophiolite_seismic::ProcessingPipelineSpec::TraceLocal { pipeline } => {
            (pipeline.name.clone(), pipeline.revision)
        }
        ophiolite_seismic::ProcessingPipelineSpec::Subvolume { pipeline } => {
            (pipeline.name.clone(), pipeline.revision)
        }
        ophiolite_seismic::ProcessingPipelineSpec::Gather { pipeline } => {
            (pipeline.name.clone(), pipeline.revision)
        }
    };
    Some(ProcessingLineageSummary {
        parent_store_path: lineage.parent_store.to_string_lossy().into_owned(),
        parent_store_id: lineage.parent_store_id.clone(),
        artifact_role: lineage.artifact_role,
        pipeline_family: lineage.pipeline.family(),
        pipeline_name: pipeline_name.filter(|value| !value.trim().is_empty()),
        pipeline_revision,
    })
}

pub fn create_tbvol_store(
    root: impl AsRef<Path>,
    manifest: TbvolManifest,
    data: &Array3<f32>,
    occupancy: Option<&Array2<u8>>,
) -> Result<StoreHandle, SeismicStoreError> {
    let writer = TbvolWriter::create(
        root.as_ref(),
        manifest.volume.clone(),
        manifest.tile_shape,
        manifest.has_occupancy,
    )?;
    write_dense_volume(&writer, data, occupancy)?;
    writer.finalize()?;
    open_store(root)
}

pub fn open_store(root: impl AsRef<Path>) -> Result<StoreHandle, SeismicStoreError> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = root.join("manifest.json");
    if !manifest_path.exists() {
        return Err(SeismicStoreError::MissingManifest(manifest_path));
    }
    let manifest = load_tbvol_manifest(&manifest_path).map_err(|error| match error {
        SeismicStoreError::Message(message) => SeismicStoreError::Message(format!(
            "failed to parse tbvol manifest at {}: {message}",
            manifest_path.display()
        )),
        other => other,
    })?;
    Ok(StoreHandle { root, manifest })
}

pub fn describe_store(root: impl AsRef<Path>) -> Result<VolumeDescriptor, SeismicStoreError> {
    Ok(open_store(root)?.volume_descriptor())
}

pub fn set_store_native_coordinate_reference(
    root: impl AsRef<Path>,
    coordinate_reference_id: Option<&str>,
    coordinate_reference_name: Option<&str>,
) -> Result<VolumeDescriptor, SeismicStoreError> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = root.join("manifest.json");
    let mut manifest = serde_json::from_slice::<TbvolManifest>(&fs::read(&manifest_path)?)?;
    manifest.volume.coordinate_reference_binding = apply_native_coordinate_reference_override(
        manifest.volume.coordinate_reference_binding.take(),
        manifest.volume.spatial.as_mut(),
        coordinate_reference_id,
        coordinate_reference_name,
    );
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(StoreHandle { root, manifest }.volume_descriptor())
}

pub fn section_view(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
) -> Result<SectionView, SeismicStoreError> {
    open_store(root)?.section_view(axis, index)
}

pub fn read_section_plane(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
) -> Result<SectionPlane, SeismicStoreError> {
    open_store(root)?.read_section_plane(axis, index)
}

pub fn load_array(handle: &StoreHandle) -> Result<Array3<f32>, SeismicStoreError> {
    let reader = TbvolReader::open(&handle.root)?;
    read_dense_volume(&reader)
}

pub fn load_occupancy(handle: &StoreHandle) -> Result<Option<Array2<u8>>, SeismicStoreError> {
    let reader = TbvolReader::open(&handle.root)?;
    read_dense_occupancy(&reader)
}

pub(crate) fn apply_native_coordinate_reference_override(
    binding: Option<CoordinateReferenceBinding>,
    spatial: Option<&mut SurveySpatialDescriptor>,
    coordinate_reference_id: Option<&str>,
    coordinate_reference_name: Option<&str>,
) -> Option<CoordinateReferenceBinding> {
    let coordinate_reference_id = coordinate_reference_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let coordinate_reference_name = coordinate_reference_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    let mut binding = binding.unwrap_or(CoordinateReferenceBinding {
        detected: None,
        effective: None,
        source: CoordinateReferenceSource::Unknown,
        notes: Vec::new(),
    });
    let template = binding
        .effective
        .clone()
        .or_else(|| binding.detected.clone())
        .or_else(|| {
            spatial
                .as_ref()
                .and_then(|item| item.coordinate_reference.clone())
        });

    if let Some(coordinate_reference_id) = coordinate_reference_id {
        let mut effective = template.unwrap_or(CoordinateReferenceDescriptor {
            id: None,
            name: None,
            geodetic_datum: None,
            unit: None,
        });
        effective.id = Some(coordinate_reference_id);
        if coordinate_reference_name.is_some() {
            effective.name = coordinate_reference_name;
        }
        binding.effective = Some(effective.clone());
        binding.source = CoordinateReferenceSource::UserOverride;
        binding
            .notes
            .retain(|note| note != "effective native coordinate reference overridden by user");
        binding.notes.push(String::from(
            "effective native coordinate reference overridden by user",
        ));
        if let Some(spatial) = spatial {
            spatial.coordinate_reference = Some(effective);
        }
        return Some(binding);
    }

    if let Some(detected) = binding.detected.clone() {
        binding.effective = Some(detected.clone());
        if let Some(spatial) = spatial {
            spatial.coordinate_reference = Some(detected);
        }
        binding
            .notes
            .retain(|note| note != "effective native coordinate reference overridden by user");
        return Some(binding);
    }

    if let Some(spatial) = spatial {
        spatial.coordinate_reference = None;
    }
    None
}

fn dataset_leaf_name(root: &Path) -> String {
    let raw = root.to_string_lossy();
    raw.rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| raw.into_owned())
}

fn dataset_id_string(root: &Path) -> String {
    dataset_leaf_name(root)
}

fn dataset_label(root: &Path) -> String {
    Path::new(&dataset_leaf_name(root))
        .file_stem()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| dataset_id_string(root))
}

fn summarize_i32_axis(values: &[f64]) -> AxisSummaryI32 {
    let first = values.first().copied().unwrap_or_default().round() as i32;
    let last = values.last().copied().unwrap_or_default().round() as i32;
    let step = regular_i32_step(values);
    AxisSummaryI32 {
        count: values.len(),
        first,
        last,
        step,
        regular: step.is_some(),
    }
}

fn summarize_f32_axis(values: &[f32]) -> AxisSummaryF32 {
    let first = values.first().copied().unwrap_or_default();
    let last = values.last().copied().unwrap_or_default();
    let step = regular_f32_step(values);
    AxisSummaryF32 {
        count: values.len(),
        first,
        last,
        step,
        regular: step.is_some(),
        units: Some("ms".to_string()),
    }
}

fn regular_i32_step(values: &[f64]) -> Option<i32> {
    if values.len() < 2 {
        return None;
    }

    let expected = (values[1] - values[0]).round() as i32;
    let regular = values
        .windows(2)
        .all(|window| ((window[1] - window[0]).round() as i32) == expected);

    regular.then_some(expected)
}

fn regular_f32_step(values: &[f32]) -> Option<f32> {
    if values.len() < 2 {
        return None;
    }

    let expected = values[1] - values[0];
    let regular = values
        .windows(2)
        .all(|window| (window[1] - window[0] - expected).abs() <= f32::EPSILON * 16.0);

    regular.then_some(expected)
}

fn geometry_provenance_summary(manifest: &TbvolManifest) -> GeometryProvenanceSummary {
    if manifest.volume.source.regularization.is_some() {
        GeometryProvenanceSummary::Regularized
    } else {
        match &manifest.volume.kind {
            DatasetKind::Source => GeometryProvenanceSummary::Source,
            DatasetKind::Derived => GeometryProvenanceSummary::Derived,
        }
    }
}

fn geometry_fingerprint(manifest: &TbvolManifest) -> String {
    let mut hash = fnv1a64_begin();
    hash = fnv1a64_update(hash, b"inline");
    hash = fnv1a64_update_f64_slice(hash, &manifest.volume.axes.ilines);
    hash = fnv1a64_update(hash, b"xline");
    hash = fnv1a64_update_f64_slice(hash, &manifest.volume.axes.xlines);
    hash = fnv1a64_update(hash, b"sample");
    hash = fnv1a64_update_f32_slice(hash, &manifest.volume.axes.sample_axis_ms);
    format!("{GEOMETRY_FINGERPRINT_VERSION}:{hash:016x}")
}

fn fnv1a64_begin() -> u64 {
    0xcbf29ce484222325
}

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn fnv1a64_update_u64(hash: u64, value: u64) -> u64 {
    fnv1a64_update(hash, &value.to_le_bytes())
}

fn fnv1a64_update_f64_slice(mut hash: u64, values: &[f64]) -> u64 {
    hash = fnv1a64_update_u64(hash, values.len() as u64);
    for value in values {
        hash = fnv1a64_update(hash, &value.to_le_bytes());
    }
    hash
}

fn fnv1a64_update_f32_slice(mut hash: u64, values: &[f32]) -> u64 {
    hash = fnv1a64_update_u64(hash, values.len() as u64);
    for value in values {
        hash = fnv1a64_update(hash, &value.to_le_bytes());
    }
    hash
}

fn f64_vec_to_le_bytes(values: &[f64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f64>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn f32_vec_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        VolumeMetadata, generate_store_id,
    };
    use ndarray::Array3;
    use serde_json::json;
    use tempfile::tempdir;

    fn fixture_manifest(shape: [usize; 3]) -> TbvolManifest {
        TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: generate_store_id(),
                source: SourceIdentity {
                    source_path: PathBuf::from("input.sgy"),
                    file_size: 1,
                    trace_count: 1,
                    samples_per_trace: shape[2],
                    sample_interval_us: 2000,
                    sample_format_code: 5,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(5),
                    endianness: "big".to_string(),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
                        inline_field: HeaderFieldSpec {
                            name: "INLINE_3D".to_string(),
                            start_byte: 189,
                            value_type: "I32".to_string(),
                        },
                        crossline_field: HeaderFieldSpec {
                            name: "CROSSLINE_3D".to_string(),
                            start_byte: 193,
                            value_type: "I32".to_string(),
                        },
                        third_axis_field: None,
                    },
                    regularization: None,
                },
                shape,
                axes: VolumeAxes {
                    ilines: vec![100.0, 101.0, 102.0],
                    xlines: vec![200.0, 201.0, 202.0, 203.0],
                    sample_axis_ms: vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0],
                },
                segy_export: None,
                coordinate_reference_binding: None,
                spatial: None,
                created_by: "test".to_string(),
                processing_lineage: None,
            },
            [2, 2, 6],
            false,
        )
    }

    #[test]
    fn volume_descriptor_uses_shared_domain_type() {
        let handle = StoreHandle {
            root: PathBuf::from("C:\\data\\survey.tbvol"),
            manifest: fixture_manifest([3, 4, 6]),
        };
        let descriptor = handle.volume_descriptor();

        assert_eq!(descriptor.id.0, "survey.tbvol");
        assert_eq!(descriptor.label, "survey");
        assert_eq!(descriptor.shape, [3, 4, 6]);
        assert_eq!(descriptor.chunk_shape, [2, 2, 6]);
        assert_eq!(descriptor.sample_interval_ms, 2.0);
    }

    #[test]
    fn section_view_uses_shared_view_type() {
        let temp_dir = tempdir().expect("temp dir");
        let root = temp_dir.path().join("survey.tbvol");
        let manifest = fixture_manifest([3, 4, 6]);
        let data = Array3::from_shape_fn((3, 4, 6), |(iline, xline, sample)| {
            iline as f32 * 100.0 + xline as f32 * 10.0 + sample as f32
        });
        create_tbvol_store(&root, manifest, &data, None).expect("store should be created");
        let handle = open_store(&root).expect("store should open");
        let view = handle
            .section_view(SectionAxis::Inline, 1)
            .expect("inline section should be valid");

        assert_eq!(view.dataset_id.0, "survey.tbvol");
        assert_eq!(view.axis, SectionAxis::Inline);
        assert_eq!(view.coordinate.index, 1);
        assert_eq!(view.coordinate.value, 101.0);
        assert_eq!(view.traces, 4);
        assert_eq!(view.samples, 6);
        assert_eq!(view.horizontal_axis_f64le.len(), 4 * 8);
        assert_eq!(view.sample_axis_f32le.len(), 6 * 4);
        assert_eq!(view.amplitudes_f32le.len(), 4 * 6 * 4);

        fs::remove_dir_all(&root).expect("temp store should be removable");
    }

    #[test]
    fn read_section_plane_reads_subset_without_loading_full_volume() {
        let temp_dir = tempdir().expect("temp dir");
        let root = temp_dir.path().join("survey.tbvol");
        let manifest = fixture_manifest([3, 4, 6]);
        let data = Array3::from_shape_fn((3, 4, 6), |(iline, xline, sample)| {
            iline as f32 * 100.0 + xline as f32 * 10.0 + sample as f32
        });
        create_tbvol_store(&root, manifest, &data, None).expect("store should be created");
        let handle = open_store(&root).expect("store should open");

        let inline = handle
            .read_section_plane(SectionAxis::Inline, 1)
            .expect("inline section plane should be valid");
        assert_eq!(inline.traces, 4);
        assert_eq!(inline.samples, 6);
        assert_eq!(inline.coordinate_index, 1);
        assert_eq!(inline.coordinate_value, 101.0);
        assert_eq!(inline.amplitudes.len(), 4 * 6);
        assert_eq!(inline.amplitudes[0], 100.0);
        assert_eq!(inline.amplitudes[1], 101.0);
        assert_eq!(inline.amplitudes[6], 110.0);

        let xline = handle
            .read_section_plane(SectionAxis::Xline, 2)
            .expect("xline section plane should be valid");
        assert_eq!(xline.traces, 3);
        assert_eq!(xline.samples, 6);
        assert_eq!(xline.coordinate_index, 2);
        assert_eq!(xline.coordinate_value, 202.0);
        assert_eq!(xline.amplitudes.len(), 3 * 6);
        assert_eq!(xline.amplitudes[0], 20.0);
        assert_eq!(xline.amplitudes[6], 120.0);
        assert_eq!(xline.amplitudes[12], 220.0);

        fs::remove_dir_all(&root).expect("temp store should be removable");
    }

    #[test]
    fn open_store_accepts_legacy_manifest_without_expanded_segy_fields() {
        let temp_dir = tempdir().expect("temp dir");
        let root = temp_dir.path().join("legacy.tbvol");
        fs::create_dir_all(&root).expect("store root");
        let legacy_manifest = json!({
            "format": "tbvol",
            "version": 1,
            "volume": {
                "kind": "Source",
                "source": {
                    "source_path": "C:\\legacy\\survey.sgy",
                    "file_size": 165060,
                    "trace_count": 414,
                    "samples_per_trace": 75,
                    "sample_interval_us": 4000,
                    "sample_format_code": 3,
                    "geometry": {
                        "inline_field": {
                            "name": "INLINE_3D",
                            "start_byte": 189,
                            "value_type": "I32"
                        },
                        "crossline_field": {
                            "name": "CROSSLINE_3D",
                            "start_byte": 193,
                            "value_type": "I32"
                        },
                        "third_axis_field": null
                    },
                    "regularization": null
                },
                "shape": [23, 18, 75],
                "axes": {
                    "ilines": [111.0, 112.0],
                    "xlines": [875.0, 876.0],
                    "sample_axis_ms": [4.0, 8.0]
                },
                "created_by": "legacy-runtime"
            },
            "tile_shape": [23, 18, 75],
            "tile_grid_shape": [1, 1],
            "sample_type": "f32",
            "endianness": "little",
            "has_occupancy": false,
            "amplitude_tile_bytes": 124200,
            "occupancy_tile_bytes": null
        });
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec_pretty(&legacy_manifest).expect("manifest json"),
        )
        .expect("manifest write");

        let handle = open_store(&root).expect("legacy manifest should open");
        assert!(!handle.manifest.volume.store_id.trim().is_empty());
        assert_eq!(handle.manifest.volume.source.endianness, "big");
        assert_eq!(handle.manifest.volume.source.revision_raw, 0);
        assert_eq!(handle.manifest.volume.source.fixed_length_trace_flag_raw, 1);
        assert_eq!(handle.manifest.volume.source.extended_textual_headers, 0);
    }
}
