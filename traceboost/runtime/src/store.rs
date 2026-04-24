use std::fs;
use std::path::Path;

use ophiolite_seismic::{ImportedHorizonDescriptor, SectionHorizonOverlayView, VolumeDescriptor};
use ophiolite_seismic_runtime::{self as runtime, SectionAxis, SeismicStoreError};
use serde_json::{Value, json};

pub use ophiolite_seismic_runtime::{
    SectionPlane, SectionTileView, StoreHandle, create_tbvol_store, load_array, load_occupancy,
};

fn maybe_normalize_legacy_processing_manifest(
    root: impl AsRef<Path>,
) -> Result<(), SeismicStoreError> {
    let manifest_path = root.as_ref().join("manifest.json");
    if !manifest_path.exists() {
        return Ok(());
    }

    let bytes = fs::read(&manifest_path)?;
    let mut manifest: Value = serde_json::from_slice(&bytes)?;
    let Some(pipeline_value) = manifest.pointer_mut("/volume/processing_lineage/pipeline") else {
        return Ok(());
    };

    let Some(object) = pipeline_value.as_object() else {
        return Ok(());
    };

    if object.contains_key("trace_local") || object.contains_key("gather") {
        return Ok(());
    }

    let mut legacy_pipeline = pipeline_value.clone();
    if let Some(pipeline_object) = legacy_pipeline.as_object_mut() {
        pipeline_object.insert("schema_version".to_string(), json!(2));
    }
    let normalized = if object.contains_key("trace_local_pipeline") {
        json!({
            "gather": {
                "pipeline": legacy_pipeline,
            }
        })
    } else if object.contains_key("schema_version") && object.contains_key("operations") {
        json!({
            "trace_local": {
                "pipeline": legacy_pipeline,
            }
        })
    } else {
        return Ok(());
    };

    *pipeline_value = normalized;
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

pub fn open_store(root: impl AsRef<Path>) -> Result<StoreHandle, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::open_store(root)
}

pub fn describe_store(root: impl AsRef<Path>) -> Result<VolumeDescriptor, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::describe_store(root)
}

pub fn section_view(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
) -> Result<ophiolite_seismic::SectionView, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::section_view(root, axis, index)
}

pub fn read_section_plane(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
) -> Result<SectionPlane, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::read_section_plane(root, axis, index)
}

pub fn section_tile_view(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
) -> Result<SectionTileView, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::section_tile_view(root, axis, index, trace_range, sample_range, lod)
}

pub fn import_horizon_xyzs<P: AsRef<Path>>(
    root: impl AsRef<Path>,
    input_paths: &[P],
    source_coordinate_reference_id: Option<&str>,
    source_coordinate_reference_name: Option<&str>,
    assume_same_as_survey: bool,
) -> Result<Vec<ImportedHorizonDescriptor>, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::import_horizon_xyzs(
        root,
        input_paths,
        source_coordinate_reference_id,
        source_coordinate_reference_name,
        assume_same_as_survey,
    )
}

pub fn section_horizon_overlays(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
) -> Result<Vec<SectionHorizonOverlayView>, SeismicStoreError> {
    maybe_normalize_legacy_processing_manifest(root.as_ref())?;
    runtime::section_horizon_overlays(root, axis, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn open_store_normalizes_legacy_trace_local_lineage_manifest() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("legacy.tbvol");
        fs::create_dir_all(&root).expect("create root");
        let manifest = json!({
            "format": "tbvol",
            "version": 1,
            "volume": {
                "kind": "Derived",
                "source": {
                    "source_path": "C:\\data\\legacy.sgy",
                    "file_size": 1024,
                    "trace_count": 25,
                    "samples_per_trace": 50,
                    "sample_interval_us": 4000,
                    "sample_format_code": 1,
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
                "shape": [5, 5, 50],
                "axes": {
                    "ilines": [1.0, 2.0, 3.0, 4.0, 5.0],
                    "xlines": [20.0, 21.0, 22.0, 23.0, 24.0],
                    "sample_axis_ms": [0.0, 4.0, 8.0]
                },
                "created_by": "ophiolite-seismic-runtime-0.1.0",
                "processing_lineage": {
                    "parent_store": "C:\\data\\source.tbvol",
                    "pipeline": {
                        "schema_version": 1,
                        "revision": 9,
                        "preset_id": "legacy",
                        "name": "combo",
                        "operations": [
                            {
                                "amplitude_scalar": {
                                    "factor": 5.0
                                }
                            },
                            "trace_rms_normalize"
                        ]
                    },
                    "runtime_version": "ophiolite-seismic-runtime-0.1.0",
                    "created_at_unix_s": 1775152392
                }
            },
            "tile_shape": [5, 5, 50],
            "tile_grid_shape": [1, 1],
            "sample_type": "f32",
            "endianness": "little",
            "has_occupancy": false,
            "amplitude_tile_bytes": 5000,
            "occupancy_tile_bytes": null
        });
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");

        let handle = open_store(&root).expect("open migrated store");
        assert_eq!(handle.manifest.volume.shape, [5, 5, 50]);

        let rewritten: ophiolite_seismic_runtime::TbvolManifest = serde_json::from_slice(
            &fs::read(root.join("manifest.json")).expect("read rewritten manifest"),
        )
        .expect("parse rewritten manifest");
        let lineage = rewritten
            .volume
            .processing_lineage
            .expect("processing lineage");
        match lineage.pipeline {
            ophiolite_seismic::ProcessingPipelineSpec::TraceLocal { pipeline } => {
                assert_eq!(pipeline.schema_version, 2);
                assert_eq!(pipeline.revision, 9);
                assert_eq!(pipeline.operation_count(), 2);
            }
            other => panic!("expected trace-local pipeline, got {other:?}"),
        }
    }
}
