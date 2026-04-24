use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ophiolite_seismic::SeismicLayout;
use ophiolite_seismic::contracts::{
    OperatorSetIdentity, PipelineSemanticIdentity, PlannerProfileIdentity, SourceSemanticIdentity,
};
use serde::{Deserialize, Serialize};

use crate::execution::{ArtifactKey, ChunkGridSpec, GeometryFingerprints, LogicalDomain};
use crate::identity::canonical_processing_lineage_validation;
use crate::metadata::ProcessingLineage;

pub const PROCESSING_OUTPUT_PACKAGE_SCHEMA_VERSION: u32 = 2;
pub const PROCESSING_OUTPUT_PACKAGE_CONFIG_SCHEMA_VERSION: u32 = 2;
const PACKAGE_MANIFEST_FILE: &str = "processing-package.manifest.json";
const PACKAGE_CONFIG_FILE: &str = "processing-package.config.json";
const PACKAGED_STORE_DIR: &str = "store";
const STORE_MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOutputPackageBlobRef {
    pub media_type: String,
    pub path: String,
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOutputPackageManifest {
    pub schema_version: u32,
    pub package_kind: String,
    pub config_blob: ProcessingOutputPackageBlobRef,
    pub store_manifest_blob: ProcessingOutputPackageBlobRef,
    pub store_root: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub store_blobs: Vec<ProcessingOutputPackageBlobRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOutputPackageConfig {
    pub schema_version: u32,
    pub store_kind: String,
    pub store_format_version: String,
    pub processing_lineage_schema_version: u32,
    pub runtime_semantics_version: String,
    pub store_writer_semantics_version: String,
    pub output_artifact_key: ArtifactKey,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_artifact_keys: Vec<ArtifactKey>,
    pub pipeline_identity: PipelineSemanticIdentity,
    pub operator_set_identity: OperatorSetIdentity,
    pub planner_profile_identity: PlannerProfileIdentity,
    pub source_identity: SourceSemanticIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_domain: Option<LogicalDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_grid_spec: Option<ChunkGridSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_fingerprints: Option<GeometryFingerprints>,
    pub store_manifest_path: String,
}

#[derive(Debug, Clone)]
pub struct ProcessingOutputPackage {
    pub manifest: ProcessingOutputPackageManifest,
    pub config: ProcessingOutputPackageConfig,
}

pub fn package_processing_output(
    store_path: impl AsRef<Path>,
    package_root: impl AsRef<Path>,
) -> Result<ProcessingOutputPackage, String> {
    let store_path = store_path.as_ref();
    let package_root = package_root.as_ref();
    let store_manifest_path = store_path.join(STORE_MANIFEST_FILE);
    let store_manifest_bytes = fs::read(&store_manifest_path).map_err(|error| error.to_string())?;
    let descriptor = package_descriptor_from_manifest(&store_manifest_bytes)?;

    if package_root.exists() {
        fs::remove_dir_all(package_root).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(package_root).map_err(|error| error.to_string())?;
    let packaged_store_root = package_root.join(PACKAGED_STORE_DIR);
    let store_blobs = copy_directory(store_path, &packaged_store_root, PACKAGED_STORE_DIR)?;

    let config = ProcessingOutputPackageConfig {
        schema_version: PROCESSING_OUTPUT_PACKAGE_CONFIG_SCHEMA_VERSION,
        store_kind: descriptor.store_kind.clone(),
        store_format_version: descriptor.store_format_version.clone(),
        processing_lineage_schema_version: descriptor.lineage.schema_version,
        runtime_semantics_version: descriptor.lineage.runtime_semantics_version.clone(),
        store_writer_semantics_version: descriptor.lineage.store_writer_semantics_version.clone(),
        output_artifact_key: descriptor.lineage.artifact_key.clone().ok_or_else(|| {
            "processing output package requires canonical artifact key".to_string()
        })?,
        input_artifact_keys: descriptor.lineage.input_artifact_keys.clone(),
        pipeline_identity: descriptor
            .lineage
            .pipeline_identity
            .clone()
            .ok_or_else(|| "processing output package requires pipeline identity".to_string())?,
        operator_set_identity: descriptor
            .lineage
            .operator_set_identity
            .clone()
            .ok_or_else(|| {
                "processing output package requires operator-set identity".to_string()
            })?,
        planner_profile_identity: descriptor
            .lineage
            .planner_profile_identity
            .clone()
            .ok_or_else(|| {
                "processing output package requires planner-profile identity".to_string()
            })?,
        source_identity: descriptor
            .lineage
            .source_identity
            .clone()
            .ok_or_else(|| "processing output package requires source identity".to_string())?,
        logical_domain: descriptor.lineage.logical_domain.clone(),
        chunk_grid_spec: descriptor.lineage.chunk_grid_spec.clone(),
        geometry_fingerprints: descriptor.lineage.geometry_fingerprints.clone(),
        store_manifest_path: format!("{PACKAGED_STORE_DIR}/{STORE_MANIFEST_FILE}"),
    };
    let config_bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
    let config_path = package_root.join(PACKAGE_CONFIG_FILE);
    fs::write(&config_path, &config_bytes).map_err(|error| error.to_string())?;

    let manifest = ProcessingOutputPackageManifest {
        schema_version: PROCESSING_OUTPUT_PACKAGE_SCHEMA_VERSION,
        package_kind: "ophiolite_processing_output".to_string(),
        config_blob: blob_ref(
            "application/vnd.ophiolite.processing-output.config+json",
            PACKAGE_CONFIG_FILE,
            &config_bytes,
        ),
        store_manifest_blob: blob_ref(
            &format!(
                "application/vnd.ophiolite.processing-store-manifest.{}+json",
                descriptor.store_kind
            ),
            &format!("{PACKAGED_STORE_DIR}/{STORE_MANIFEST_FILE}"),
            &store_manifest_bytes,
        ),
        store_root: PACKAGED_STORE_DIR.to_string(),
        store_blobs,
        annotations: BTreeMap::from([
            (
                "artifactKey".to_string(),
                config.output_artifact_key.cache_key.clone(),
            ),
            (
                "pipelineContentDigest".to_string(),
                config.pipeline_identity.content_digest.clone(),
            ),
            (
                "operatorSetVersion".to_string(),
                config.operator_set_identity.version.clone(),
            ),
            (
                "plannerProfileVersion".to_string(),
                config.planner_profile_identity.version.clone(),
            ),
            (
                "runtimeSemanticsVersion".to_string(),
                config.runtime_semantics_version.clone(),
            ),
            (
                "storeWriterSemanticsVersion".to_string(),
                config.store_writer_semantics_version.clone(),
            ),
        ]),
    };
    fs::write(
        package_root.join(PACKAGE_MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    Ok(ProcessingOutputPackage { manifest, config })
}

pub fn open_processing_output_package(
    package_root: impl AsRef<Path>,
) -> Result<ProcessingOutputPackage, String> {
    let package_root = package_root.as_ref();
    let manifest_bytes =
        fs::read(package_root.join(PACKAGE_MANIFEST_FILE)).map_err(|error| error.to_string())?;
    let manifest: ProcessingOutputPackageManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    let config_bytes =
        fs::read(package_root.join(PACKAGE_CONFIG_FILE)).map_err(|error| error.to_string())?;
    let config: ProcessingOutputPackageConfig =
        serde_json::from_slice(&config_bytes).map_err(|error| error.to_string())?;
    validate_blob_ref(
        package_root,
        PACKAGE_CONFIG_FILE,
        &manifest.config_blob,
        Some(&config_bytes),
    )?;
    let packaged_store_manifest_path = manifest
        .store_root
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let expected_store_manifest_path =
        format!("{packaged_store_manifest_path}/{STORE_MANIFEST_FILE}");
    let store_manifest_bytes = fs::read(package_root.join(&expected_store_manifest_path))
        .map_err(|error| error.to_string())?;
    validate_blob_ref(
        package_root,
        &expected_store_manifest_path,
        &manifest.store_manifest_blob,
        Some(&store_manifest_bytes),
    )?;
    for blob in &manifest.store_blobs {
        validate_blob_ref(package_root, &blob.path, blob, None)?;
    }
    let descriptor = package_descriptor_from_manifest(&store_manifest_bytes)?;
    ensure_package_annotations(&manifest, &config)?;
    ensure_package_config_matches_descriptor(&config, &descriptor, &expected_store_manifest_path)?;
    Ok(ProcessingOutputPackage { manifest, config })
}

#[derive(Debug, Clone)]
struct PackagedStoreDescriptor {
    store_kind: String,
    store_format_version: String,
    layout: SeismicLayout,
    shape: [usize; 3],
    chunk_shape: [usize; 3],
    lineage: ProcessingLineage,
}

fn package_descriptor_from_manifest(bytes: &[u8]) -> Result<PackagedStoreDescriptor, String> {
    let manifest =
        serde_json::from_slice::<serde_json::Value>(bytes).map_err(|error| error.to_string())?;
    let store_kind = manifest
        .get("format")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "processing output package requires store format".to_string())?
        .to_string();
    let version = manifest
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "processing output package requires store format version".to_string())?;
    let lineage_value = manifest
        .get("volume")
        .and_then(|value| value.get("processing_lineage"))
        .cloned()
        .ok_or_else(|| "processing output package requires processing lineage".to_string())?;
    let lineage: ProcessingLineage =
        serde_json::from_value(lineage_value).map_err(|error| error.to_string())?;
    let layout = packaged_store_layout(&manifest, &lineage)?;
    let shape = packaged_store_shape(&manifest)?;
    let chunk_shape = packaged_store_chunk_shape(&manifest, &store_kind)?;
    ensure_canonical_lineage(&lineage, layout, shape, chunk_shape)?;
    Ok(PackagedStoreDescriptor {
        store_format_version: format!("{store_kind}@{version}"),
        store_kind,
        layout,
        shape,
        chunk_shape,
        lineage,
    })
}

fn ensure_canonical_lineage(
    lineage: &ProcessingLineage,
    layout: SeismicLayout,
    shape: [usize; 3],
    chunk_shape: [usize; 3],
) -> Result<(), String> {
    canonical_processing_lineage_validation(lineage, layout, shape, chunk_shape, None).map(|_| ())
}

fn ensure_package_annotations(
    manifest: &ProcessingOutputPackageManifest,
    config: &ProcessingOutputPackageConfig,
) -> Result<(), String> {
    for (key, expected) in [
        ("artifactKey", config.output_artifact_key.cache_key.as_str()),
        (
            "pipelineContentDigest",
            config.pipeline_identity.content_digest.as_str(),
        ),
        (
            "operatorSetVersion",
            config.operator_set_identity.version.as_str(),
        ),
        (
            "plannerProfileVersion",
            config.planner_profile_identity.version.as_str(),
        ),
        (
            "runtimeSemanticsVersion",
            config.runtime_semantics_version.as_str(),
        ),
        (
            "storeWriterSemanticsVersion",
            config.store_writer_semantics_version.as_str(),
        ),
    ] {
        let actual = manifest
            .annotations
            .get(key)
            .ok_or_else(|| format!("processing output package is missing annotation `{key}`"))?;
        if actual != expected {
            return Err(format!(
                "processing output package annotation `{key}` does not match packaged config"
            ));
        }
    }
    Ok(())
}

fn ensure_package_config_matches_descriptor(
    config: &ProcessingOutputPackageConfig,
    descriptor: &PackagedStoreDescriptor,
    expected_store_manifest_path: &str,
) -> Result<(), String> {
    if config.store_kind != descriptor.store_kind {
        return Err(
            "processing output package store kind does not match packaged store".to_string(),
        );
    }
    if config.store_format_version != descriptor.store_format_version {
        return Err(
            "processing output package store format version does not match packaged store"
                .to_string(),
        );
    }
    if config.processing_lineage_schema_version != descriptor.lineage.schema_version {
        return Err(
            "processing output package lineage schema version does not match packaged store"
                .to_string(),
        );
    }
    if config.runtime_semantics_version != descriptor.lineage.runtime_semantics_version {
        return Err(
            "processing output package runtime semantics version does not match packaged store"
                .to_string(),
        );
    }
    if config.store_writer_semantics_version != descriptor.lineage.store_writer_semantics_version {
        return Err(
            "processing output package store writer semantics version does not match packaged store"
                .to_string(),
        );
    }
    if config.store_manifest_path.replace('\\', "/") != expected_store_manifest_path {
        return Err("processing output package store manifest path is inconsistent".to_string());
    }

    let lineage = &descriptor.lineage;
    if config.output_artifact_key
        != *lineage.artifact_key.as_ref().ok_or_else(|| {
            "processing output package requires canonical artifact key".to_string()
        })?
    {
        return Err(
            "processing output package artifact key does not match packaged lineage".to_string(),
        );
    }
    if config.input_artifact_keys != lineage.input_artifact_keys {
        return Err(
            "processing output package input artifact keys do not match packaged lineage"
                .to_string(),
        );
    }
    if config.pipeline_identity
        != *lineage
            .pipeline_identity
            .as_ref()
            .ok_or_else(|| "processing output package requires pipeline identity".to_string())?
    {
        return Err(
            "processing output package pipeline identity does not match packaged lineage"
                .to_string(),
        );
    }
    if config.operator_set_identity
        != *lineage
            .operator_set_identity
            .as_ref()
            .ok_or_else(|| "processing output package requires operator-set identity".to_string())?
    {
        return Err(
            "processing output package operator-set identity does not match packaged lineage"
                .to_string(),
        );
    }
    if config.planner_profile_identity
        != *lineage.planner_profile_identity.as_ref().ok_or_else(|| {
            "processing output package requires planner-profile identity".to_string()
        })?
    {
        return Err(
            "processing output package planner-profile identity does not match packaged lineage"
                .to_string(),
        );
    }
    if config.source_identity
        != *lineage
            .source_identity
            .as_ref()
            .ok_or_else(|| "processing output package requires source identity".to_string())?
    {
        return Err(
            "processing output package source identity does not match packaged lineage".to_string(),
        );
    }
    if config.logical_domain != lineage.logical_domain {
        return Err(
            "processing output package logical domain does not match packaged lineage".to_string(),
        );
    }
    if config.chunk_grid_spec != lineage.chunk_grid_spec {
        return Err(
            "processing output package chunk grid does not match packaged lineage".to_string(),
        );
    }
    if config.geometry_fingerprints != lineage.geometry_fingerprints {
        return Err(
            "processing output package geometry fingerprints do not match packaged lineage"
                .to_string(),
        );
    }

    ensure_canonical_lineage(
        &descriptor.lineage,
        descriptor.layout,
        descriptor.shape,
        descriptor.chunk_shape,
    )?;
    Ok(())
}

fn blob_ref(media_type: &str, path: &str, bytes: &[u8]) -> ProcessingOutputPackageBlobRef {
    ProcessingOutputPackageBlobRef {
        media_type: media_type.to_string(),
        path: path.replace('\\', "/"),
        digest: blake3::hash(bytes).to_hex().to_string(),
        size_bytes: bytes.len() as u64,
    }
}

fn copy_directory(
    source_root: &Path,
    target_root: &Path,
    target_root_label: &str,
) -> Result<Vec<ProcessingOutputPackageBlobRef>, String> {
    fs::create_dir_all(target_root).map_err(|error| error.to_string())?;
    let mut blobs = Vec::new();
    let mut entries = fs::read_dir(source_root)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_string());
    for entry in entries {
        let source_path = entry.path();
        let target_path = target_root.join(entry.file_name());
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            let child_label = format!(
                "{}/{}",
                target_root_label.replace('\\', "/"),
                entry.file_name().to_string_lossy()
            );
            blobs.extend(copy_directory(&source_path, &target_path, &child_label)?);
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(&source_path, &target_path).map_err(|error| error.to_string())?;
            let bytes = fs::read(&target_path).map_err(|error| error.to_string())?;
            blobs.push(blob_ref(
                "application/octet-stream",
                &format!(
                    "{}/{}",
                    target_root_label.replace('\\', "/"),
                    entry.file_name().to_string_lossy()
                ),
                &bytes,
            ));
        }
    }
    blobs.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(blobs)
}

fn validate_blob_ref(
    package_root: &Path,
    expected_path: &str,
    blob: &ProcessingOutputPackageBlobRef,
    expected_bytes: Option<&[u8]>,
) -> Result<(), String> {
    let normalized_expected_path = expected_path.replace('\\', "/");
    if blob.path.replace('\\', "/") != normalized_expected_path {
        return Err(format!(
            "processing output package blob path mismatch for `{normalized_expected_path}`"
        ));
    }
    let bytes = match expected_bytes {
        Some(bytes) => bytes.to_vec(),
        None => fs::read(package_root.join(&normalized_expected_path))
            .map_err(|error| error.to_string())?,
    };
    if blob.size_bytes != bytes.len() as u64 {
        return Err(format!(
            "processing output package blob size mismatch for `{normalized_expected_path}`"
        ));
    }
    let digest = blake3::hash(&bytes).to_hex().to_string();
    if blob.digest != digest {
        return Err(format!(
            "processing output package blob digest mismatch for `{normalized_expected_path}`"
        ));
    }
    Ok(())
}

fn packaged_store_layout(
    manifest: &serde_json::Value,
    lineage: &ProcessingLineage,
) -> Result<SeismicLayout, String> {
    manifest
        .get("layout")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| error.to_string())?
        .or_else(|| {
            lineage
                .source_identity
                .as_ref()
                .map(|identity| identity.layout)
        })
        .ok_or_else(|| "processing output package requires store layout".to_string())
}

fn packaged_store_shape(manifest: &serde_json::Value) -> Result<[usize; 3], String> {
    serde_json::from_value(
        manifest
            .get("volume")
            .and_then(|value| value.get("shape"))
            .cloned()
            .ok_or_else(|| "processing output package requires store shape".to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn packaged_store_chunk_shape(
    manifest: &serde_json::Value,
    store_kind: &str,
) -> Result<[usize; 3], String> {
    if let Some(chunk_shape) = manifest.get("tile_shape").cloned() {
        return serde_json::from_value(chunk_shape).map_err(|error| error.to_string());
    }
    if store_kind == "tbgath" {
        return packaged_store_shape(manifest);
    }
    Err("processing output package requires store chunk shape".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::contracts::{SourceSemanticIdentity, StoreFormatIdentity};
    use ophiolite_seismic::{
        ProcessingArtifactRole, ProcessingPipelineSpec, SeismicLayout, TraceLocalProcessingPipeline,
    };
    use std::path::PathBuf;

    use crate::CanonicalIdentityStatus;
    use crate::execution::{ArtifactBoundaryReason, MaterializationClass, VolumeDomain};
    use crate::identity::{
        CURRENT_RUNTIME_SEMANTICS_VERSION, CURRENT_STORE_WRITER_SEMANTICS_VERSION,
        canonical_artifact_identity, operator_set_identity_for_pipeline,
        pipeline_semantic_identity, planner_profile_identity_for_pipeline,
    };

    fn temp_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ophiolite-package-{label}-{unique}"));
        fs::create_dir_all(&root).expect("create temp package dir");
        root
    }

    fn canonical_manifest_json() -> serde_json::Value {
        let pipeline = TraceLocalProcessingPipeline {
            schema_version: 2,
            revision: 1,
            preset_id: None,
            name: Some("Example".to_string()),
            description: None,
            steps: Vec::new(),
        };
        let pipeline_spec = ProcessingPipelineSpec::TraceLocal {
            pipeline: pipeline.clone(),
        };
        let source_identity = SourceSemanticIdentity {
            schema_version: 1,
            store_id: "source-store".to_string(),
            store_format: StoreFormatIdentity {
                schema_version: 1,
                store_kind: "tbvol".to_string(),
                store_format_version: "tbvol@2".to_string(),
            },
            layout: SeismicLayout::PostStack3D,
            shape: Some([4, 4, 8]),
            chunk_shape: Some([4, 4, 8]),
            sample_type: Some("f32".to_string()),
            endianness: Some("little".to_string()),
            parent_artifact_key: None,
        };
        let pipeline_identity =
            pipeline_semantic_identity(&pipeline_spec).expect("pipeline identity");
        let operator_set_identity =
            operator_set_identity_for_pipeline(&pipeline_spec).expect("operator set identity");
        let planner_profile_identity = planner_profile_identity_for_pipeline(&pipeline_spec)
            .expect("planner profile identity");
        let canonical_artifact = canonical_artifact_identity(
            &source_identity,
            CanonicalIdentityStatus::Canonical,
            &pipeline_identity,
            &operator_set_identity,
            &planner_profile_identity,
            SeismicLayout::PostStack3D,
            [4, 4, 8],
            [4, 4, 8],
            ProcessingArtifactRole::FinalOutput,
            ArtifactBoundaryReason::FinalOutput,
            MaterializationClass::PublishedOutput,
            LogicalDomain::Volume {
                volume: VolumeDomain { shape: [4, 4, 8] },
            },
        )
        .expect("canonical artifact identity")
        .expect("canonical artifact");
        let lineage = ProcessingLineage {
            schema_version: ophiolite_seismic::contracts::default_processing_lineage_schema_version(
            ),
            parent_store: PathBuf::from(r"C:\derived\source.tbvol"),
            parent_store_id: "source-store".to_string(),
            artifact_role: ProcessingArtifactRole::FinalOutput,
            pipeline: pipeline_spec,
            pipeline_identity: Some(pipeline_identity),
            operator_set_identity: Some(operator_set_identity),
            planner_profile_identity: Some(planner_profile_identity),
            source_identity: Some(source_identity),
            runtime_semantics_version: CURRENT_RUNTIME_SEMANTICS_VERSION.to_string(),
            store_writer_semantics_version: CURRENT_STORE_WRITER_SEMANTICS_VERSION.to_string(),
            runtime_version: "test-runtime".to_string(),
            created_at_unix_s: 1,
            artifact_key: Some(canonical_artifact.artifact_key.clone()),
            input_artifact_keys: Vec::new(),
            produced_by_stage_id: None,
            boundary_reason: Some(ArtifactBoundaryReason::FinalOutput),
            logical_domain: Some(canonical_artifact.logical_domain),
            chunk_grid_spec: Some(canonical_artifact.chunk_grid_spec),
            geometry_fingerprints: Some(canonical_artifact.geometry_fingerprints),
        };
        let mut lineage_json = serde_json::to_value(&lineage).expect("serialize lineage");
        if let Some(source_identity) = lineage_json
            .get_mut("source_identity")
            .and_then(serde_json::Value::as_object_mut)
        {
            source_identity.insert(
                "layout".to_string(),
                serde_json::Value::String("post_stack3_d".to_string()),
            );
        }
        serde_json::json!({
            "format": "tbvol",
            "version": 2,
            "layout": "post_stack3_d",
            "tile_shape": [4, 4, 8],
            "volume": {
                "shape": [4, 4, 8],
                "processing_lineage": lineage_json
            }
        })
    }

    #[test]
    fn package_processing_output_writes_manifest_and_config() {
        let store_root = temp_dir("store");
        let package_root = temp_dir("package");
        fs::write(
            store_root.join(STORE_MANIFEST_FILE),
            serde_json::to_vec_pretty(&canonical_manifest_json()).expect("serialize manifest"),
        )
        .expect("write store manifest");
        fs::write(store_root.join("amplitude.bin"), [0_u8, 1, 2, 3]).expect("write payload");

        let packaged =
            package_processing_output(&store_root, &package_root).expect("package output");

        assert_eq!(packaged.config.store_format_version, "tbvol@2");
        assert_eq!(
            packaged.config.output_artifact_key.cache_key,
            packaged
                .manifest
                .annotations
                .get("artifactKey")
                .expect("artifact key annotation")
                .as_str()
        );
        assert_eq!(
            packaged
                .manifest
                .annotations
                .get("pipelineContentDigest")
                .map(String::as_str),
            Some(packaged.config.pipeline_identity.content_digest.as_str())
        );
        assert!(package_root.join(PACKAGE_MANIFEST_FILE).exists());
        assert!(package_root.join(PACKAGE_CONFIG_FILE).exists());
        assert!(
            package_root
                .join(PACKAGED_STORE_DIR)
                .join(STORE_MANIFEST_FILE)
                .exists()
        );
    }

    #[test]
    fn package_processing_output_rejects_noncanonical_lineage() {
        let store_root = temp_dir("store-invalid");
        let package_root = temp_dir("package-invalid");
        let mut manifest = canonical_manifest_json();
        manifest["volume"]["processing_lineage"]["artifact_key"] = serde_json::Value::Null;
        fs::write(
            store_root.join(STORE_MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write store manifest");

        let error = package_processing_output(&store_root, &package_root)
            .expect_err("noncanonical lineage should be rejected");
        assert!(error.contains("artifact key") || error.contains("canonical"));
    }
}
