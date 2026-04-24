use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use ophiolite_seismic::contracts::{
    OperatorSetIdentity, PipelineSemanticIdentity, PlannerProfileIdentity, SourceArtifactIdentity,
    SourceSemanticIdentity, StoreFormatIdentity, current_reuse_identity_schema_version,
    default_processing_lineage_schema_version,
};
use ophiolite_seismic::{
    GatherProcessingOperation, GatherProcessingPipeline, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, ProcessingArtifactRole, ProcessingLayoutCompatibility,
    ProcessingOperatorDependencyProfile, ProcessingPipelineFamily, ProcessingPipelineSpec,
    ProcessingPlannerHints, SeismicLayout, SubvolumeCropOperation, SubvolumeProcessingPipeline,
    TraceLocalProcessingOperation, TraceLocalProcessingPipeline, gather_operator_planner_hints,
    post_stack_neighborhood_operator_planner_hints, subvolume_operator_planner_hints,
    trace_local_operator_planner_hints,
};

use crate::ProcessingCacheFingerprint;
use crate::execution::{
    ArtifactBoundaryReason, ArtifactKey, ChunkGridSpec, GeometryFingerprints, LogicalDomain,
    MaterializationClass, VolumeDomain,
};
use crate::metadata::ProcessingLineage;
use crate::prestack_store::open_prestack_store;
use crate::storage::tbvol::inspect_tbvol_manifest;
use crate::storage::tbvolc::inspect_tbvolc_manifest;
use crate::storage::volume_store::PostStackStoreEnvelope;

pub const CURRENT_RUNTIME_SEMANTICS_VERSION: &str = "ophiolite-runtime-semantics:v2";
pub const CURRENT_STORE_WRITER_SEMANTICS_VERSION: &str = "ophiolite-store-writer-semantics:v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalIdentityStatus {
    Canonical,
    NormalizedLegacyReadable,
    LegacyReadableNoCanonicalReuse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSourceSemanticIdentity {
    pub identity: SourceSemanticIdentity,
    pub status: CanonicalIdentityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalArtifactIdentity {
    pub artifact_key: ArtifactKey,
    pub logical_domain: LogicalDomain,
    pub chunk_grid_spec: ChunkGridSpec,
    pub geometry_fingerprints: GeometryFingerprints,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLineageValidation {
    pub artifact_key: ArtifactKey,
    pub logical_domain: LogicalDomain,
    pub chunk_grid_spec: ChunkGridSpec,
    pub geometry_fingerprints: GeometryFingerprints,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineContentSeed {
    schema_version: u32,
    family: ProcessingPipelineFamily,
    pipeline_schema_version: u32,
    revision: u32,
    semantics: Value,
}

#[derive(Debug, Clone, Serialize)]
struct OperatorSemanticSeed {
    operator_id: String,
    compatibility: ProcessingLayoutCompatibility,
    dependency_profile: ProcessingOperatorDependencyProfile,
}

#[derive(Debug, Clone, Serialize)]
struct OperatorSetSeed {
    schema_version: u32,
    family: ProcessingPipelineFamily,
    operators: Vec<OperatorSemanticSeed>,
}

#[derive(Debug, Clone, Serialize)]
struct StructuralPlannerSeed {
    operator_id: String,
    preferred_partitioning: String,
    requires_full_volume: bool,
    checkpoint_safe: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PlannerProfileSeed {
    schema_version: u32,
    family: ProcessingPipelineFamily,
    structural_hints: Vec<StructuralPlannerSeed>,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalSurveyGeometrySeed<'a> {
    source_identity_digest: &'a str,
    output_layout: SeismicLayout,
    output_shape: [usize; 3],
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalStorageGridSeed<'a> {
    source_identity_digest: &'a str,
    output_layout: SeismicLayout,
    output_shape: [usize; 3],
    output_chunk_shape: [usize; 3],
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalArtifactLineageFingerprintSeed<'a> {
    source_identity_digest: &'a str,
    pipeline_family: ProcessingPipelineFamily,
    pipeline_schema_version: u32,
    pipeline_revision: u32,
    pipeline_content_digest: &'a str,
    operator_set_version: &'a str,
    planner_profile_version: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalArtifactLineageSeed<'a> {
    source_identity_digest: &'a str,
    pipeline_family: ProcessingPipelineFamily,
    pipeline_schema_version: u32,
    pipeline_revision: u32,
    pipeline_content_digest: &'a str,
    operator_set_version: &'a str,
    planner_profile_version: &'a str,
    artifact_role: ProcessingArtifactRole,
    boundary_reason: ArtifactBoundaryReason,
}

pub fn fingerprint_json<T: Serialize>(value: &T) -> Result<String, String> {
    ProcessingCacheFingerprint::fingerprint_json(value)
}

pub fn pipeline_semantic_identity(
    pipeline: &ProcessingPipelineSpec,
) -> Result<PipelineSemanticIdentity, String> {
    let (family, pipeline_schema_version, revision, semantics) = match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => (
            ProcessingPipelineFamily::TraceLocal,
            pipeline.schema_version,
            pipeline.revision,
            normalize_trace_local_pipeline(pipeline)?,
        ),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => (
            ProcessingPipelineFamily::PostStackNeighborhood,
            pipeline.schema_version,
            pipeline.revision,
            normalize_post_stack_pipeline(pipeline)?,
        ),
        ProcessingPipelineSpec::Subvolume { pipeline } => (
            ProcessingPipelineFamily::Subvolume,
            pipeline.schema_version,
            pipeline.revision,
            normalize_subvolume_pipeline(pipeline)?,
        ),
        ProcessingPipelineSpec::Gather { pipeline } => (
            ProcessingPipelineFamily::Gather,
            pipeline.schema_version,
            pipeline.revision,
            normalize_gather_pipeline(pipeline)?,
        ),
    };
    let content_digest = fingerprint_json(&PipelineContentSeed {
        schema_version: 1,
        family,
        pipeline_schema_version,
        revision,
        semantics,
    })?;
    Ok(PipelineSemanticIdentity {
        schema_version: 1,
        family,
        pipeline_schema_version,
        revision,
        content_digest,
    })
}

pub fn operator_set_identity_for_pipeline(
    pipeline: &ProcessingPipelineSpec,
) -> Result<OperatorSetIdentity, String> {
    let family = pipeline.family();
    let operators = operator_semantic_seeds(pipeline);
    let version = fingerprint_json(&OperatorSetSeed {
        schema_version: 1,
        family,
        operators: operators.clone(),
    })?;
    Ok(OperatorSetIdentity {
        schema_version: 1,
        family,
        version: version.clone(),
        effective_operator_digest: version,
    })
}

pub fn planner_profile_identity_for_pipeline(
    pipeline: &ProcessingPipelineSpec,
) -> Result<PlannerProfileIdentity, String> {
    let family = pipeline.family();
    let structural_hints = planner_structural_seeds(pipeline);
    let version = fingerprint_json(&PlannerProfileSeed {
        schema_version: 1,
        family,
        structural_hints: structural_hints.clone(),
    })?;
    Ok(PlannerProfileIdentity {
        schema_version: 1,
        family,
        version: version.clone(),
        effective_structural_digest: version,
    })
}

pub fn source_semantic_identity_from_store_path(
    store_path: &str,
    layout: SeismicLayout,
) -> Result<SourceSemanticIdentity, String> {
    Ok(load_source_semantic_identity_from_store_path(store_path, layout)?.identity)
}

pub fn source_semantic_identity_with_status_from_store_path(
    store_path: &str,
    layout: SeismicLayout,
) -> Result<LoadedSourceSemanticIdentity, String> {
    load_source_semantic_identity_from_store_path(store_path, layout)
}

fn load_source_semantic_identity_from_store_path(
    store_path: &str,
    layout: SeismicLayout,
) -> Result<LoadedSourceSemanticIdentity, String> {
    let extension = Path::new(store_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(layout, SeismicLayout::PreStack3DOffset) || extension == "tbgath" {
        let handle = open_prestack_store(store_path).map_err(|error| error.to_string())?;
        return Ok(LoadedSourceSemanticIdentity {
            identity: SourceSemanticIdentity {
                schema_version: 1,
                store_id: handle.manifest.volume.store_id.clone(),
                store_format: StoreFormatIdentity {
                    schema_version: 1,
                    store_kind: handle.manifest.format.clone(),
                    store_format_version: format!(
                        "{}@{}",
                        handle.manifest.format, handle.manifest.version
                    ),
                },
                layout: handle.manifest.layout,
                shape: Some(handle.manifest.volume.shape),
                chunk_shape: None,
                sample_type: Some(handle.manifest.sample_type.clone()),
                endianness: Some(handle.manifest.endianness.clone()),
                parent_artifact_key: handle
                    .manifest
                    .volume
                    .processing_lineage
                    .as_ref()
                    .and_then(|lineage| lineage.artifact_key.as_ref())
                    .map(|artifact_key| artifact_key.cache_key.clone()),
            },
            status: CanonicalIdentityStatus::Canonical,
        });
    }

    if extension == "tbvolc" {
        let (manifest, normalized_legacy) =
            inspect_tbvolc_manifest(&Path::new(store_path).join("manifest.json"))
                .map_err(|error| error.to_string())?;
        return Ok(LoadedSourceSemanticIdentity {
            identity: source_semantic_identity_from_poststack_store_envelope(
                manifest.envelope(),
                layout,
            ),
            status: if normalized_legacy {
                CanonicalIdentityStatus::NormalizedLegacyReadable
            } else {
                CanonicalIdentityStatus::Canonical
            },
        });
    }

    let (manifest, normalized_legacy) =
        inspect_tbvol_manifest(&Path::new(store_path).join("manifest.json"))
            .map_err(|error| error.to_string())?;
    Ok(LoadedSourceSemanticIdentity {
        identity: source_semantic_identity_from_poststack_store_envelope(
            manifest.envelope(),
            layout,
        ),
        status: if normalized_legacy {
            CanonicalIdentityStatus::NormalizedLegacyReadable
        } else {
            CanonicalIdentityStatus::Canonical
        },
    })
}

fn source_semantic_identity_from_poststack_store_envelope(
    envelope: PostStackStoreEnvelope,
    layout: SeismicLayout,
) -> SourceSemanticIdentity {
    let parent_artifact_key = envelope.parent_artifact_key();
    SourceSemanticIdentity {
        schema_version: 1,
        store_id: envelope.volume.store_id.clone(),
        store_format: StoreFormatIdentity {
            schema_version: 1,
            store_kind: envelope.format.clone(),
            store_format_version: envelope.store_format_version(),
        },
        layout,
        shape: Some(envelope.volume.shape),
        chunk_shape: Some(envelope.tile_shape),
        sample_type: Some(envelope.sample_type),
        endianness: Some(envelope.endianness),
        parent_artifact_key,
    }
}

pub fn source_semantic_identity_or_degraded(
    store_path: &str,
    layout: SeismicLayout,
    shape: Option<[usize; 3]>,
    chunk_shape: Option<[usize; 3]>,
) -> LoadedSourceSemanticIdentity {
    source_semantic_identity_with_status_from_store_path(store_path, layout).unwrap_or_else(|_| {
        LoadedSourceSemanticIdentity {
            identity: synthetic_source_semantic_identity(store_path, layout, shape, chunk_shape),
            status: CanonicalIdentityStatus::LegacyReadableNoCanonicalReuse,
        }
    })
}

#[allow(dead_code)]
pub fn source_semantic_identity_or_synthetic(
    store_path: &str,
    layout: SeismicLayout,
    shape: Option<[usize; 3]>,
    chunk_shape: Option<[usize; 3]>,
) -> SourceSemanticIdentity {
    source_semantic_identity_or_degraded(store_path, layout, shape, chunk_shape).identity
}

fn synthetic_source_semantic_identity(
    store_path: &str,
    layout: SeismicLayout,
    shape: Option<[usize; 3]>,
    chunk_shape: Option<[usize; 3]>,
) -> SourceSemanticIdentity {
    let store_kind = Path::new(store_path)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "synthetic".to_string());
    let store_id = fingerprint_json(&(&store_kind, layout, shape, chunk_shape))
        .map(|digest| format!("legacy-unresolved:{digest}"))
        .unwrap_or_else(|_| "legacy-unresolved".to_string());
    SourceSemanticIdentity {
        schema_version: 1,
        store_id,
        store_format: StoreFormatIdentity {
            schema_version: 1,
            store_kind: store_kind.clone(),
            store_format_version: format!("{store_kind}@synthetic"),
        },
        layout,
        shape,
        chunk_shape,
        sample_type: None,
        endianness: None,
        parent_artifact_key: None,
    }
}

pub fn source_artifact_identity_from_source_identity(
    identity: &SourceSemanticIdentity,
) -> SourceArtifactIdentity {
    SourceArtifactIdentity {
        schema_version: current_reuse_identity_schema_version(),
        store_path: None,
        store_id: Some(identity.store_id.clone()),
        store_kind: Some(identity.store_format.store_kind.clone()),
        store_format_version: Some(identity.store_format.store_format_version.clone()),
        layout: identity.layout,
        shape: identity.shape,
        chunk_shape: identity.chunk_shape,
        sample_type: identity.sample_type.clone(),
        endianness: identity.endianness.clone(),
        parent_artifact_key: identity.parent_artifact_key.clone(),
    }
}

pub fn source_identity_digest(identity: &SourceSemanticIdentity) -> Result<String, String> {
    fingerprint_json(identity)
}

pub fn canonical_identity_status_supports_canonical_reuse(status: CanonicalIdentityStatus) -> bool {
    matches!(status, CanonicalIdentityStatus::Canonical)
}

pub fn combine_canonical_identity_status(
    left: CanonicalIdentityStatus,
    right: CanonicalIdentityStatus,
) -> CanonicalIdentityStatus {
    use CanonicalIdentityStatus::{
        Canonical, LegacyReadableNoCanonicalReuse, NormalizedLegacyReadable,
    };

    match (left, right) {
        (LegacyReadableNoCanonicalReuse, _) | (_, LegacyReadableNoCanonicalReuse) => {
            LegacyReadableNoCanonicalReuse
        }
        (NormalizedLegacyReadable, _) | (_, NormalizedLegacyReadable) => NormalizedLegacyReadable,
        _ => Canonical,
    }
}

pub fn pipeline_external_identity_status(
    pipeline: &ProcessingPipelineSpec,
) -> CanonicalIdentityStatus {
    let mut status = CanonicalIdentityStatus::Canonical;
    for secondary_store_path in referenced_secondary_store_paths(pipeline) {
        let secondary_status = source_semantic_identity_or_degraded(
            secondary_store_path,
            SeismicLayout::PostStack3D,
            None,
            None,
        )
        .status;
        status = combine_canonical_identity_status(status, secondary_status);
    }
    status
}

pub fn canonical_artifact_identity(
    source_identity: &SourceSemanticIdentity,
    source_status: CanonicalIdentityStatus,
    pipeline_identity: &PipelineSemanticIdentity,
    operator_set_identity: &OperatorSetIdentity,
    planner_profile_identity: &PlannerProfileIdentity,
    output_layout: SeismicLayout,
    output_shape: [usize; 3],
    output_chunk_shape: [usize; 3],
    artifact_role: ProcessingArtifactRole,
    boundary_reason: ArtifactBoundaryReason,
    materialization_class: MaterializationClass,
    logical_domain: LogicalDomain,
) -> Result<Option<CanonicalArtifactIdentity>, String> {
    if !canonical_identity_status_supports_canonical_reuse(source_status) {
        return Ok(None);
    }

    let source_identity_digest = source_identity_digest(source_identity)?;
    let chunk_grid_spec = ChunkGridSpec::Regular {
        origin: [0, 0, 0],
        chunk_shape: output_chunk_shape,
    };
    let geometry_fingerprints = canonical_geometry_fingerprints(
        &source_identity_digest,
        output_layout,
        output_shape,
        output_chunk_shape,
        pipeline_identity,
        operator_set_identity,
        planner_profile_identity,
    )?;
    let lineage_digest = fingerprint_json(&CanonicalArtifactLineageSeed {
        source_identity_digest: &source_identity_digest,
        pipeline_family: pipeline_identity.family,
        pipeline_schema_version: pipeline_identity.pipeline_schema_version,
        pipeline_revision: pipeline_identity.revision,
        pipeline_content_digest: &pipeline_identity.content_digest,
        operator_set_version: &operator_set_identity.version,
        planner_profile_version: &planner_profile_identity.version,
        artifact_role,
        boundary_reason,
    })?;
    let cache_key = fingerprint_json(&(
        &lineage_digest,
        &geometry_fingerprints,
        &logical_domain,
        &chunk_grid_spec,
        materialization_class,
    ))?;
    Ok(Some(CanonicalArtifactIdentity {
        artifact_key: ArtifactKey {
            lineage_digest,
            geometry_fingerprints: geometry_fingerprints.clone(),
            logical_domain: logical_domain.clone(),
            chunk_grid_spec: chunk_grid_spec.clone(),
            materialization_class,
            cache_key,
        },
        logical_domain,
        chunk_grid_spec,
        geometry_fingerprints,
    }))
}

pub fn canonical_processing_lineage_validation(
    lineage: &ProcessingLineage,
    output_layout: SeismicLayout,
    output_shape: [usize; 3],
    output_chunk_shape: [usize; 3],
    expected_artifact_role: Option<ProcessingArtifactRole>,
) -> Result<CanonicalLineageValidation, String> {
    if lineage.schema_version < default_processing_lineage_schema_version() {
        return Err("processing lineage schema version is not canonical".to_string());
    }
    if lineage.runtime_semantics_version != CURRENT_RUNTIME_SEMANTICS_VERSION
        || lineage.store_writer_semantics_version != CURRENT_STORE_WRITER_SEMANTICS_VERSION
    {
        return Err(
            "processing lineage semantics version does not match current canonical runtime"
                .to_string(),
        );
    }

    let pipeline_identity = lineage
        .pipeline_identity
        .as_ref()
        .ok_or_else(|| "processing lineage requires pipeline identity".to_string())?;
    let operator_set_identity = lineage
        .operator_set_identity
        .as_ref()
        .ok_or_else(|| "processing lineage requires operator-set identity".to_string())?;
    let planner_profile_identity = lineage
        .planner_profile_identity
        .as_ref()
        .ok_or_else(|| "processing lineage requires planner-profile identity".to_string())?;
    let source_identity = lineage
        .source_identity
        .as_ref()
        .ok_or_else(|| "processing lineage requires source identity".to_string())?;
    let boundary_reason = lineage
        .boundary_reason
        .ok_or_else(|| "processing lineage requires boundary reason".to_string())?;
    let artifact_key = lineage
        .artifact_key
        .as_ref()
        .ok_or_else(|| "processing lineage requires canonical artifact key".to_string())?;
    let logical_domain = lineage
        .logical_domain
        .as_ref()
        .ok_or_else(|| "processing lineage requires logical domain".to_string())?;
    let chunk_grid_spec = lineage
        .chunk_grid_spec
        .as_ref()
        .ok_or_else(|| "processing lineage requires chunk grid specification".to_string())?;
    let geometry_fingerprints = lineage
        .geometry_fingerprints
        .as_ref()
        .ok_or_else(|| "processing lineage requires geometry fingerprints".to_string())?;
    if let Some(expected_artifact_role) = expected_artifact_role {
        if lineage.artifact_role != expected_artifact_role {
            return Err("processing lineage artifact role mismatch".to_string());
        }
    }

    let expected_materialization_class = match lineage.artifact_role {
        ProcessingArtifactRole::Checkpoint => MaterializationClass::Checkpoint,
        ProcessingArtifactRole::FinalOutput => MaterializationClass::PublishedOutput,
    };
    let expected_logical_domain = LogicalDomain::Volume {
        volume: VolumeDomain {
            shape: output_shape,
        },
    };
    if logical_domain != &expected_logical_domain {
        return Err(
            "processing lineage logical domain is not canonical volume identity".to_string(),
        );
    }

    let expected = canonical_artifact_identity(
        source_identity,
        combine_canonical_identity_status(
            CanonicalIdentityStatus::Canonical,
            pipeline_external_identity_status(&lineage.pipeline),
        ),
        pipeline_identity,
        operator_set_identity,
        planner_profile_identity,
        output_layout,
        output_shape,
        output_chunk_shape,
        lineage.artifact_role,
        boundary_reason,
        expected_materialization_class,
        expected_logical_domain,
    )?
    .ok_or_else(|| "processing lineage source identity is not canonical".to_string())?;

    if chunk_grid_spec != &expected.chunk_grid_spec
        || geometry_fingerprints != &expected.geometry_fingerprints
        || logical_domain != &expected.logical_domain
    {
        return Err(
            "processing lineage canonical components are internally inconsistent".to_string(),
        );
    }
    if artifact_key != &expected.artifact_key {
        return Err(
            "processing lineage artifact key does not match canonical derivation".to_string(),
        );
    }

    Ok(CanonicalLineageValidation {
        artifact_key: expected.artifact_key,
        logical_domain: expected.logical_domain,
        chunk_grid_spec: expected.chunk_grid_spec,
        geometry_fingerprints: expected.geometry_fingerprints,
    })
}

pub fn pipeline_identity_status(lineage: &ProcessingLineage) -> CanonicalIdentityStatus {
    let has_full_semantics = lineage.pipeline_identity.is_some()
        && lineage.operator_set_identity.is_some()
        && lineage.planner_profile_identity.is_some()
        && lineage.source_identity.is_some()
        && lineage.boundary_reason.is_some()
        && lineage.logical_domain.is_some()
        && lineage.chunk_grid_spec.is_some()
        && lineage.geometry_fingerprints.is_some()
        && !lineage.runtime_semantics_version.trim().is_empty()
        && !lineage.store_writer_semantics_version.trim().is_empty();
    if has_full_semantics {
        CanonicalIdentityStatus::Canonical
    } else if lineage.artifact_key.is_some() {
        CanonicalIdentityStatus::NormalizedLegacyReadable
    } else {
        CanonicalIdentityStatus::LegacyReadableNoCanonicalReuse
    }
}

fn normalize_trace_local_pipeline(
    pipeline: &TraceLocalProcessingPipeline,
) -> Result<Value, String> {
    let steps = pipeline
        .steps
        .iter()
        .map(|step| {
            Ok(json!({
                "checkpoint": step.checkpoint,
                "operation": normalize_trace_local_operation(&step.operation)?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({ "steps": steps }))
}

fn normalize_post_stack_pipeline(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> Result<Value, String> {
    let operations = pipeline
        .operations
        .iter()
        .map(normalize_post_stack_operation)
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "trace_local_pipeline": match pipeline.trace_local_pipeline.as_ref() {
            Some(prefix) => Some(normalize_trace_local_pipeline(prefix)?),
            None => None,
        },
        "operations": operations,
    }))
}

fn normalize_subvolume_pipeline(pipeline: &SubvolumeProcessingPipeline) -> Result<Value, String> {
    Ok(json!({
        "trace_local_pipeline": match pipeline.trace_local_pipeline.as_ref() {
            Some(prefix) => Some(normalize_trace_local_pipeline(prefix)?),
            None => None,
        },
        "crop_operation": json!({
            "operator_id": pipeline.crop.operator_id(),
            "operation": serde_json::to_value(&pipeline.crop).map_err(|error| error.to_string())?,
        }),
    }))
}

fn normalize_gather_pipeline(pipeline: &GatherProcessingPipeline) -> Result<Value, String> {
    let operations = pipeline
        .operations
        .iter()
        .map(normalize_gather_operation)
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "trace_local_pipeline": match pipeline.trace_local_pipeline.as_ref() {
            Some(prefix) => Some(normalize_trace_local_pipeline(prefix)?),
            None => None,
        },
        "operations": operations,
    }))
}

fn normalize_trace_local_operation(
    operation: &TraceLocalProcessingOperation,
) -> Result<Value, String> {
    let value = match operation {
        TraceLocalProcessingOperation::VolumeArithmetic {
            operator,
            secondary_store_path,
        } => {
            let secondary_source = (!secondary_store_path.trim().is_empty()).then(|| {
                source_semantic_identity_or_degraded(
                    secondary_store_path,
                    SeismicLayout::PostStack3D,
                    None,
                    None,
                )
            });
            let secondary_source_identity_digest = match secondary_source.as_ref() {
                Some(loaded)
                    if canonical_identity_status_supports_canonical_reuse(loaded.status) =>
                {
                    Some(source_identity_digest(&loaded.identity)?)
                }
                _ => None,
            };
            json!({
                "operator_id": operation.operator_id(),
                "operation": {
                    "volume_arithmetic": {
                        "operator": operator,
                        "secondary_source_identity_digest": secondary_source_identity_digest,
                        "secondary_source_identity_status": secondary_source
                            .as_ref()
                            .map(|loaded| canonical_identity_status_label(loaded.status)),
                    }
                }
            })
        }
        _ => json!({
            "operator_id": operation.operator_id(),
            "operation": serde_json::to_value(operation).map_err(|error| error.to_string())?,
        }),
    };
    Ok(value)
}

fn canonical_geometry_fingerprints(
    source_identity_digest: &str,
    output_layout: SeismicLayout,
    output_shape: [usize; 3],
    output_chunk_shape: [usize; 3],
    pipeline_identity: &PipelineSemanticIdentity,
    operator_set_identity: &OperatorSetIdentity,
    planner_profile_identity: &PlannerProfileIdentity,
) -> Result<GeometryFingerprints, String> {
    Ok(GeometryFingerprints {
        survey_geometry_fingerprint: fingerprint_json(&CanonicalSurveyGeometrySeed {
            source_identity_digest,
            output_layout,
            output_shape,
        })?,
        storage_grid_fingerprint: fingerprint_json(&CanonicalStorageGridSeed {
            source_identity_digest,
            output_layout,
            output_shape,
            output_chunk_shape,
        })?,
        section_projection_fingerprint: fingerprint_json(&(output_layout, output_shape))?,
        artifact_lineage_fingerprint: fingerprint_json(&CanonicalArtifactLineageFingerprintSeed {
            source_identity_digest,
            pipeline_family: pipeline_identity.family,
            pipeline_schema_version: pipeline_identity.pipeline_schema_version,
            pipeline_revision: pipeline_identity.revision,
            pipeline_content_digest: &pipeline_identity.content_digest,
            operator_set_version: &operator_set_identity.version,
            planner_profile_version: &planner_profile_identity.version,
        })?,
    })
}

fn canonical_identity_status_label(status: CanonicalIdentityStatus) -> &'static str {
    match status {
        CanonicalIdentityStatus::Canonical => "canonical",
        CanonicalIdentityStatus::NormalizedLegacyReadable => "normalized_legacy_readable",
        CanonicalIdentityStatus::LegacyReadableNoCanonicalReuse => {
            "legacy_readable_no_canonical_reuse"
        }
    }
}

fn referenced_secondary_store_paths(pipeline: &ProcessingPipelineSpec) -> Vec<&str> {
    let mut paths = Vec::new();
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => {
            collect_trace_local_secondary_store_paths(pipeline, &mut paths);
        }
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
            if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
                collect_trace_local_secondary_store_paths(trace_local_pipeline, &mut paths);
            }
        }
        ProcessingPipelineSpec::Subvolume { pipeline } => {
            if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
                collect_trace_local_secondary_store_paths(trace_local_pipeline, &mut paths);
            }
        }
        ProcessingPipelineSpec::Gather { pipeline } => {
            if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
                collect_trace_local_secondary_store_paths(trace_local_pipeline, &mut paths);
            }
        }
    }
    paths
}

fn collect_trace_local_secondary_store_paths<'a>(
    pipeline: &'a TraceLocalProcessingPipeline,
    paths: &mut Vec<&'a str>,
) {
    for step in &pipeline.steps {
        if let TraceLocalProcessingOperation::VolumeArithmetic {
            secondary_store_path,
            ..
        } = &step.operation
        {
            if !secondary_store_path.trim().is_empty() {
                paths.push(secondary_store_path.as_str());
            }
        }
    }
}

fn normalize_post_stack_operation(
    operation: &PostStackNeighborhoodProcessingOperation,
) -> Result<Value, String> {
    Ok(json!({
        "operator_id": operation.operator_id(),
        "operation": serde_json::to_value(operation).map_err(|error| error.to_string())?,
    }))
}

fn normalize_gather_operation(operation: &GatherProcessingOperation) -> Result<Value, String> {
    Ok(json!({
        "operator_id": operation.operator_id(),
        "operation": serde_json::to_value(operation).map_err(|error| error.to_string())?,
    }))
}

fn operator_semantic_seeds(pipeline: &ProcessingPipelineSpec) -> Vec<OperatorSemanticSeed> {
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => pipeline
            .steps
            .iter()
            .map(|step| operator_seed_for_trace_local(&step.operation))
            .collect(),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|prefix| {
                prefix
                    .steps
                    .iter()
                    .map(|step| operator_seed_for_trace_local(&step.operation))
            })
            .chain(
                pipeline
                    .operations
                    .iter()
                    .map(operator_seed_for_post_stack_operation),
            )
            .collect(),
        ProcessingPipelineSpec::Subvolume { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|prefix| {
                prefix
                    .steps
                    .iter()
                    .map(|step| operator_seed_for_trace_local(&step.operation))
            })
            .chain(std::iter::once(operator_seed_for_subvolume_operation(
                &pipeline.crop,
            )))
            .collect(),
        ProcessingPipelineSpec::Gather { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|prefix| {
                prefix
                    .steps
                    .iter()
                    .map(|step| operator_seed_for_trace_local(&step.operation))
            })
            .chain(
                pipeline
                    .operations
                    .iter()
                    .map(operator_seed_for_gather_operation),
            )
            .collect(),
    }
}

fn planner_structural_seeds(pipeline: &ProcessingPipelineSpec) -> Vec<StructuralPlannerSeed> {
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => pipeline
            .steps
            .iter()
            .map(|step| {
                structural_seed(
                    step.operation.operator_id(),
                    trace_local_operator_planner_hints(&step.operation),
                )
            })
            .collect(),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|prefix| {
                prefix.steps.iter().map(|step| {
                    structural_seed(
                        step.operation.operator_id(),
                        trace_local_operator_planner_hints(&step.operation),
                    )
                })
            })
            .chain(pipeline.operations.iter().map(|operation| {
                structural_seed(
                    operation.operator_id(),
                    post_stack_neighborhood_operator_planner_hints(operation),
                )
            }))
            .collect(),
        ProcessingPipelineSpec::Subvolume { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|prefix| {
                prefix.steps.iter().map(|step| {
                    structural_seed(
                        step.operation.operator_id(),
                        trace_local_operator_planner_hints(&step.operation),
                    )
                })
            })
            .chain(std::iter::once(structural_seed(
                pipeline.crop.operator_id(),
                subvolume_operator_planner_hints(&pipeline.crop),
            )))
            .collect(),
        ProcessingPipelineSpec::Gather { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|prefix| {
                prefix.steps.iter().map(|step| {
                    structural_seed(
                        step.operation.operator_id(),
                        trace_local_operator_planner_hints(&step.operation),
                    )
                })
            })
            .chain(pipeline.operations.iter().map(|operation| {
                structural_seed(
                    operation.operator_id(),
                    gather_operator_planner_hints(operation),
                )
            }))
            .collect(),
    }
}

fn structural_seed(operator_id: &str, hints: ProcessingPlannerHints) -> StructuralPlannerSeed {
    StructuralPlannerSeed {
        operator_id: operator_id.to_string(),
        preferred_partitioning: format!("{:?}", hints.preferred_partitioning),
        requires_full_volume: hints.requires_full_volume,
        checkpoint_safe: hints.checkpoint_safe,
    }
}

fn operator_seed_for_trace_local(
    operation: &TraceLocalProcessingOperation,
) -> OperatorSemanticSeed {
    OperatorSemanticSeed {
        operator_id: operation.operator_id().to_string(),
        compatibility: operation.compatibility(),
        dependency_profile: operation.dependency_profile(),
    }
}

fn operator_seed_for_post_stack_operation(
    operation: &PostStackNeighborhoodProcessingOperation,
) -> OperatorSemanticSeed {
    OperatorSemanticSeed {
        operator_id: operation.operator_id().to_string(),
        compatibility: operation.compatibility(),
        dependency_profile: operation.dependency_profile(),
    }
}

fn operator_seed_for_subvolume_operation(
    operation: &SubvolumeCropOperation,
) -> OperatorSemanticSeed {
    OperatorSemanticSeed {
        operator_id: operation.operator_id().to_string(),
        compatibility: operation.compatibility(),
        dependency_profile: operation.dependency_profile(),
    }
}

fn operator_seed_for_gather_operation(
    operation: &GatherProcessingOperation,
) -> OperatorSemanticSeed {
    OperatorSemanticSeed {
        operator_id: operation.operator_id().to_string(),
        compatibility: operation.compatibility(),
        dependency_profile: operation.dependency_profile(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        VolumeMetadata,
    };
    use crate::storage::tbvol::TbvolWriter;
    use crate::storage::tbvolc::transcode_tbvol_to_tbvolc;
    use crate::storage::tile_geometry::TileCoord;
    use crate::storage::volume_store::VolumeStoreWriter;
    use crate::{SeismicLayout, generate_store_id};
    use ophiolite_seismic::TimeDepthDomain;

    use super::{
        CanonicalIdentityStatus, source_semantic_identity_from_store_path,
        source_semantic_identity_with_status_from_store_path,
    };

    #[test]
    fn source_identity_uses_poststack_store_envelope_for_tbvol_and_tbvolc() {
        let temp = TempDir::new().expect("tempdir");
        let tbvol_root = temp.path().join("synthetic.tbvol");
        let tbvolc_root = temp.path().join("synthetic.tbvolc");
        let mut volume = test_volume_metadata([2, 2, 4]);
        volume.store_id = "identity-store-id".to_string();

        let writer =
            TbvolWriter::create(&tbvol_root, volume.clone(), [2, 2, 4], false).expect("writer");
        writer
            .write_tile(
                TileCoord {
                    tile_i: 0,
                    tile_x: 0,
                },
                &[0.0_f32; 16],
            )
            .expect("write tile");
        writer.finalize().expect("finalize writer");

        transcode_tbvol_to_tbvolc(&tbvol_root, &tbvolc_root).expect("transcode");

        let tbvol_identity = source_semantic_identity_from_store_path(
            &tbvol_root.to_string_lossy(),
            SeismicLayout::PostStack3D,
        )
        .expect("tbvol identity");
        assert_eq!(tbvol_identity.store_id, "identity-store-id");
        assert_eq!(tbvol_identity.store_format.store_kind, "tbvol");
        assert_eq!(tbvol_identity.store_format.store_format_version, "tbvol@2");
        assert_eq!(tbvol_identity.chunk_shape, Some([2, 2, 4]));

        let tbvolc_loaded = source_semantic_identity_with_status_from_store_path(
            &tbvolc_root.to_string_lossy(),
            SeismicLayout::PostStack3D,
        )
        .expect("tbvolc identity");
        assert_eq!(tbvolc_loaded.status, CanonicalIdentityStatus::Canonical);
        assert_eq!(tbvolc_loaded.identity.store_id, "identity-store-id");
        assert_eq!(tbvolc_loaded.identity.store_format.store_kind, "tbvolc");
        assert_eq!(
            tbvolc_loaded.identity.store_format.store_format_version,
            "tbvolc@1"
        );
        assert_eq!(tbvolc_loaded.identity.chunk_shape, Some([2, 2, 4]));
    }

    #[test]
    fn legacy_tbvol_identity_is_normalized_legacy_without_rewriting_manifest() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join("legacy.tbvol");
        std::fs::create_dir_all(&root).expect("root dir");
        let manifest_path = root.join("manifest.json");
        let manifest = serde_json::json!({
            "format": "tbvol",
            "version": 1,
            "volume": {
                "kind": "Source",
                "source": {
                    "source_path": "synthetic://legacy-tbvol",
                    "file_size": 0,
                    "trace_count": 4,
                    "samples_per_trace": 2,
                    "sample_interval_us": 2000,
                    "sample_format_code": 5,
                    "geometry": {
                        "inline_field": {
                            "name": "INLINE",
                            "start_byte": 189,
                            "value_type": "I32"
                        },
                        "crossline_field": {
                            "name": "XLINE",
                            "start_byte": 193,
                            "value_type": "I32"
                        },
                        "third_axis_field": null
                    },
                    "regularization": null
                },
                "shape": [2, 2, 2],
                "axes": {
                    "ilines": [0.0, 1.0],
                    "xlines": [0.0, 1.0],
                    "sample_axis_ms": [0.0, 2.0]
                },
                "created_by": "legacy-runtime"
            },
            "tile_shape": [2, 2, 2],
            "tile_grid_shape": [1, 1],
            "sample_type": "f32",
            "endianness": "little",
            "has_occupancy": false,
            "amplitude_tile_bytes": 32,
            "occupancy_tile_bytes": null
        });
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");

        let loaded = source_semantic_identity_with_status_from_store_path(
            &root.to_string_lossy(),
            SeismicLayout::PostStack3D,
        )
        .expect("load identity");

        assert_eq!(
            loaded.status,
            CanonicalIdentityStatus::NormalizedLegacyReadable
        );
        assert!(!loaded.identity.store_id.trim().is_empty());

        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).expect("read manifest"))
                .expect("parse manifest");
        assert!(persisted["volume"].get("store_id").is_none());
        assert!(
            persisted["volume"]["axes"]
                .get("sample_axis_domain")
                .is_none()
        );
        assert!(
            persisted["volume"]["axes"]
                .get("sample_axis_unit")
                .is_none()
        );
    }

    #[test]
    fn legacy_tbvolc_identity_is_normalized_legacy_without_rewriting_manifest() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join("legacy.tbvolc");
        std::fs::create_dir_all(&root).expect("root dir");
        let manifest_path = root.join("manifest.json");
        let manifest = serde_json::json!({
            "format": "tbvolc",
            "version": 0,
            "volume": {
                "kind": "Source",
                "source": {
                    "source_path": "synthetic://legacy-tbvolc",
                    "file_size": 0,
                    "trace_count": 4,
                    "samples_per_trace": 2,
                    "sample_interval_us": 2000,
                    "sample_format_code": 5,
                    "geometry": {
                        "inline_field": {
                            "name": "INLINE",
                            "start_byte": 189,
                            "value_type": "I32"
                        },
                        "crossline_field": {
                            "name": "XLINE",
                            "start_byte": 193,
                            "value_type": "I32"
                        },
                        "third_axis_field": null
                    },
                    "regularization": null
                },
                "shape": [2, 2, 2],
                "axes": {
                    "ilines": [0.0, 1.0],
                    "xlines": [0.0, 1.0],
                    "sample_axis_ms": [0.0, 2.0]
                },
                "created_by": "legacy-runtime"
            },
            "tile_shape": [2, 2, 2],
            "tile_grid_shape": [1, 1],
            "sample_type": "f32",
            "endianness": "little",
            "has_occupancy": false,
            "amplitude_encoding": {
                "codec": "native",
                "compressor": "lz4",
                "filters": ["bitshuffle_g8"],
                "compression_level": null,
                "lossless": true
            },
            "amplitude_tile_sample_count": 8,
            "tile_count": 1
        });
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
        std::fs::write(root.join("amplitude.index.bin"), vec![0_u8; 20]).expect("write index");
        std::fs::write(root.join("amplitude.bin"), vec![0_u8; 1]).expect("write amplitude");

        let loaded = source_semantic_identity_with_status_from_store_path(
            &root.to_string_lossy(),
            SeismicLayout::PostStack3D,
        )
        .expect("load identity");

        assert_eq!(
            loaded.status,
            CanonicalIdentityStatus::NormalizedLegacyReadable
        );
        assert!(!loaded.identity.store_id.trim().is_empty());

        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).expect("read manifest"))
                .expect("parse manifest");
        assert_eq!(persisted["version"], 0);
        assert!(persisted["volume"].get("store_id").is_none());
        assert!(
            persisted["volume"]["axes"]
                .get("sample_axis_domain")
                .is_none()
        );
        assert!(
            persisted["volume"]["axes"]
                .get("sample_axis_unit")
                .is_none()
        );
    }

    fn test_volume_metadata(shape: [usize; 3]) -> VolumeMetadata {
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://identity-test"),
                file_size: 0,
                trace_count: (shape[0] * shape[1]) as u64,
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
                        name: "INLINE".to_string(),
                        start_byte: 189,
                        value_type: "I32".to_string(),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: "XLINE".to_string(),
                        start_byte: 193,
                        value_type: "I32".to_string(),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape,
            axes: VolumeAxes::with_vertical_axis(
                (0..shape[0]).map(|value| value as f64).collect(),
                (0..shape[1]).map(|value| value as f64).collect(),
                TimeDepthDomain::Time,
                "ms".to_string(),
                (0..shape[2]).map(|value| value as f32 * 2.0).collect(),
            ),
            segy_export: None,
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "identity-test".to_string(),
            processing_lineage: None,
        }
    }
}
