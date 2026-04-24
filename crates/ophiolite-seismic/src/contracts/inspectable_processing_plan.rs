use std::fmt::Write;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{SectionAxis, SeismicLayout};

use super::{
    GatherProcessingPipeline, OperatorSetIdentity, PipelineSemanticIdentity,
    PlannerProfileIdentity, PostStackNeighborhoodProcessingPipeline, ProcessingPipelineFamily,
    ProcessingPipelineSpec, ReuseArtifactKind, ReuseBoundaryKind, ReuseMissReason,
    ReuseRequirement, ReuseResolution, SourceSemanticIdentity, SubvolumeProcessingPipeline,
    TraceLocalProcessingPipeline, default_inspectable_plan_schema_version,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectablePlanningMode {
    InteractivePreview,
    ForegroundMaterialize,
    BackgroundBatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableExecutionPriorityClass {
    InteractivePreview,
    ForegroundMaterialize,
    BackgroundBatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableExecutionQueueClass {
    Control,
    InteractivePartition,
    ForegroundPartition,
    BackgroundPartition,
    ExclusiveFullVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableSpillabilityClass {
    Unspillable,
    Spillable,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableRetryGranularity {
    Job,
    Stage,
    Partition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableProgressGranularity {
    Stage,
    Partition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableExclusiveScope {
    None,
    FullVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableExecutionStageKind {
    Compute,
    Checkpoint,
    ReuseArtifact,
    FinalizeOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableExecutionArtifactRole {
    Input,
    Checkpoint,
    FinalOutput,
    CachedReuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectablePartitionFamily {
    TileGroup,
    Section,
    GatherGroup,
    FullVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectablePartitionOrdering {
    StorageOrder,
    InlineMajor,
    CrosslineMajor,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableChunkShapePolicy {
    InheritSource,
    PlannerSelected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableCacheMode {
    PreferReuse,
    RequireReuse,
    FreshCompute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableCostClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableParallelEfficiencyClass {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableBoundaryReason {
    AuthoredCheckpoint,
    FinalOutput,
    FamilyRoot,
    TraceLocalPrefix,
    FamilyOperationBlock,
    GeometryBoundary,
    ExternalInputFanIn,
    ReuseDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableMaterializationClass {
    EphemeralWindow,
    EphemeralPartition,
    Checkpoint,
    PublishedOutput,
    ReusedArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableReuseClass {
    InPlaceSameWindow,
    ReusableSameSection,
    ReusableSameGeometry,
    RequiresExternalInputs,
    GeometryBarrier,
    FullVolumeBarrier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableArtifactLifetimeClass {
    Source,
    Ephemeral,
    Checkpoint,
    Published,
    CachedReuse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableVolumeDomain {
    pub shape: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableSectionDomain {
    pub axis: SectionAxis,
    pub section_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableSectionWindowDomain {
    pub axis: SectionAxis,
    pub section_index: usize,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub lod: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableTileDomain {
    pub tile_index: [usize; 2],
    pub tile_origin: [usize; 3],
    pub tile_shape: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePartitionDomain {
    pub partition_index: usize,
    pub partition_count: usize,
    pub tile_range: [usize; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(rename_all = "snake_case", tag = "kind")]
pub enum InspectableLogicalDomain {
    Volume {
        volume: InspectableVolumeDomain,
    },
    Section {
        section: InspectableSectionDomain,
    },
    SectionWindow {
        section_window: InspectableSectionWindowDomain,
    },
    Tile {
        tile: InspectableTileDomain,
    },
    Partition {
        partition: InspectablePartitionDomain,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(rename_all = "snake_case", tag = "kind")]
pub enum InspectableChunkGridSpec {
    Regular {
        origin: [usize; 3],
        chunk_shape: [usize; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableGeometryFingerprints {
    pub survey_geometry_fingerprint: String,
    pub storage_grid_fingerprint: String,
    pub section_projection_fingerprint: String,
    pub artifact_lineage_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableArtifactKey {
    pub lineage_digest: String,
    pub geometry_fingerprints: InspectableGeometryFingerprints,
    pub logical_domain: InspectableLogicalDomain,
    pub chunk_grid_spec: InspectableChunkGridSpec,
    pub materialization_class: InspectableMaterializationClass,
    pub cache_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectablePlannerPassId {
    ValidateAuthoredPipeline,
    NormalizePipeline,
    DeriveSemanticSegments,
    DeriveExecutionHints,
    PlanPartitions,
    PlanArtifactsAndReuse,
    AssembleExecutionPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectablePlanDecisionSubjectKind {
    PlannerPass,
    Stage,
    Artifact,
    Scheduler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectablePlanDecisionKind {
    Lowering,
    Scheduling,
    Reuse,
    ArtifactDerivation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableDecisionFactor {
    pub code: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableStagePlanningDecision {
    pub selected_partition_family: InspectablePartitionFamily,
    pub selected_ordering: InspectablePartitionOrdering,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number | null")]
    pub selected_target_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_expected_partition_count: Option<usize>,
    pub selected_queue_class: InspectableExecutionQueueClass,
    pub selected_spillability: InspectableSpillabilityClass,
    pub selected_exclusive_scope: InspectableExclusiveScope,
    pub selected_preferred_partition_waves: usize,
    #[ts(type = "number")]
    pub selected_reservation_bytes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factors: Vec<InspectableDecisionFactor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum InspectableReuseDecisionOutcome {
    Reused,
    Miss,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableReuseDecisionEvidence {
    pub label: String,
    pub matched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_store_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub miss_reason: Option<ReuseMissReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableReuseDecision {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_id: Option<String>,
    pub artifact_id: String,
    pub cache_mode: InspectableCacheMode,
    pub artifact_kind: ReuseArtifactKind,
    pub boundary_kind: ReuseBoundaryKind,
    pub candidate_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_reuse_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_artifact_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_store_path: Option<String>,
    pub outcome: InspectableReuseDecisionOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub miss_reason: Option<ReuseMissReason>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<InspectableReuseDecisionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableArtifactDerivation {
    pub artifact_id: String,
    pub artifact_key: InspectableArtifactKey,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_artifact_ids: Vec<String>,
    pub logical_domain: InspectableLogicalDomain,
    pub chunk_grid_spec: InspectableChunkGridSpec,
    pub geometry_fingerprints: InspectableGeometryFingerprints,
    pub materialization_class: InspectableMaterializationClass,
    pub boundary_reason: InspectableBoundaryReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePlanDecision {
    pub decision_id: String,
    pub subject_kind: InspectablePlanDecisionSubjectKind,
    pub subject_id: String,
    pub decision_kind: InspectablePlanDecisionKind,
    pub reason_code: String,
    pub human_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_planning: Option<InspectableStagePlanningDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_decision: Option<InspectableReuseDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_derivation: Option<InspectableArtifactDerivation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePlanSource {
    pub store_path: String,
    pub layout: SeismicLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<[usize; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_shape: Option<[usize; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableTraceLocalSegment {
    pub start_step_index: usize,
    pub end_step_index: usize,
    pub step_count: usize,
    pub boundary_reason: InspectableBoundaryReason,
    #[serde(default)]
    pub checkpoint: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operator_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableTraceLocalSemanticPlan {
    pub pipeline: TraceLocalProcessingPipeline,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<InspectableTraceLocalSegment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(rename_all = "snake_case", tag = "kind")]
pub enum InspectableSemanticRootNode {
    TraceLocal {
        trace_local: InspectableTraceLocalSemanticPlan,
    },
    PostStackNeighborhood {
        pipeline: PostStackNeighborhoodProcessingPipeline,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trace_local_prefix: Option<InspectableTraceLocalSemanticPlan>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        operation_ids: Vec<String>,
        operation_count: usize,
    },
    Subvolume {
        pipeline: SubvolumeProcessingPipeline,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trace_local_prefix: Option<InspectableTraceLocalSemanticPlan>,
        crop_operator_id: String,
    },
    Gather {
        pipeline: GatherProcessingPipeline,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trace_local_prefix: Option<InspectableTraceLocalSemanticPlan>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        operation_ids: Vec<String>,
        operation_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableSemanticPlan {
    pub pipeline_family: ProcessingPipelineFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_name: Option<String>,
    pub pipeline_revision: u32,
    pub authored_pipeline: ProcessingPipelineSpec,
    pub root: InspectableSemanticRootNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableExecutionPipelineSegment {
    pub family: ProcessingPipelineFamily,
    pub start_step_index: usize,
    pub end_step_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePartitionPlan {
    pub family: InspectablePartitionFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number | null")]
    pub target_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_partition_count: Option<usize>,
    pub ordering: InspectablePartitionOrdering,
    pub requires_barrier: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableHaloSpec {
    pub inline_radius: usize,
    pub xline_radius: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableRetryPolicy {
    pub max_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableProgressUnits {
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableCostEstimate {
    pub relative_cpu_cost: f32,
    #[ts(type = "number")]
    pub estimated_peak_memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableStageClassification {
    pub max_memory_cost_class: InspectableCostClass,
    pub max_cpu_cost_class: InspectableCostClass,
    pub max_io_cost_class: InspectableCostClass,
    pub min_parallel_efficiency_class: InspectableParallelEfficiencyClass,
    pub combined_cpu_weight: f32,
    pub combined_io_weight: f32,
    pub uses_external_inputs: bool,
    pub requires_full_volume: bool,
    pub has_sample_halo: bool,
    pub has_spatial_halo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableStageMemoryProfile {
    pub chunkability: String,
    #[ts(type = "number")]
    pub primary_tile_bytes: u64,
    pub secondary_input_count: usize,
    #[ts(type = "number")]
    pub secondary_tile_bytes_per_input: u64,
    #[ts(type = "number")]
    pub output_tile_bytes: u64,
    #[ts(type = "number")]
    pub per_worker_workspace_bytes: u64,
    #[ts(type = "number")]
    pub shared_stage_bytes: u64,
    #[ts(type = "number")]
    pub reserve_hint_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableStageResourceEnvelope {
    pub preferred_queue_class: InspectableExecutionQueueClass,
    pub spillability: InspectableSpillabilityClass,
    pub exclusive_scope: InspectableExclusiveScope,
    pub retry_granularity: InspectableRetryGranularity,
    pub progress_granularity: InspectableProgressGranularity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_partition_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_partition_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_partition_count: Option<usize>,
    pub preferred_partition_waves: usize,
    #[ts(type = "number")]
    pub resident_bytes_per_partition: u64,
    #[ts(type = "number")]
    pub workspace_bytes_per_worker: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableExecutionStage {
    pub stage_id: String,
    pub stage_kind: InspectableExecutionStageKind,
    pub stage_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_reason: Option<InspectableBoundaryReason>,
    pub input_artifact_ids: Vec<String>,
    pub output_artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_segment: Option<InspectableExecutionPipelineSegment>,
    pub partition: InspectablePartitionPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_partition_count: Option<usize>,
    pub halo: InspectableHaloSpec,
    pub chunk_shape_policy: InspectableChunkShapePolicy,
    pub cache_mode: InspectableCacheMode,
    pub retry_policy: InspectableRetryPolicy,
    pub progress_units: InspectableProgressUnits,
    pub classification: InspectableStageClassification,
    pub memory_cost_class: InspectableCostClass,
    pub estimated_cost: InspectableCostEstimate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_profile: Option<InspectableStageMemoryProfile>,
    pub resource_envelope: InspectableStageResourceEnvelope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialization_class: Option<InspectableMaterializationClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_class: Option<InspectableReuseClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_artifact_key: Option<InspectableArtifactKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_artifact_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number")]
    pub estimated_live_set_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_requirement: Option<ReuseRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_resolution: Option<ReuseResolution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planning_decision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_decision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableExecutionPlanSummary {
    pub compute_stage_count: usize,
    pub max_memory_cost_class: InspectableCostClass,
    pub max_cpu_cost_class: InspectableCostClass,
    pub max_io_cost_class: InspectableCostClass,
    pub min_parallel_efficiency_class: InspectableParallelEfficiencyClass,
    pub max_relative_cpu_cost: f32,
    #[ts(type = "number")]
    pub max_estimated_peak_memory_bytes: u64,
    pub combined_cpu_weight: f32,
    pub combined_io_weight: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_expected_partition_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number")]
    pub max_live_set_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_live_artifact_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableSchedulerHints {
    pub priority_class: InspectableExecutionPriorityClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_active_partitions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_partition_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableExecutionPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<InspectableExecutionStage>,
    pub summary: InspectableExecutionPlanSummary,
    pub scheduler_hints: InspectableSchedulerHints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePlannedArtifact {
    pub artifact_id: String,
    pub role: InspectableExecutionArtifactRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialization_class: Option<InspectableMaterializationClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_key: Option<InspectableArtifactKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_domain: Option<InspectableLogicalDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_grid_spec: Option<InspectableChunkGridSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_fingerprints: Option<InspectableGeometryFingerprints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_reason: Option<InspectableBoundaryReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime_class: Option<InspectableArtifactLifetimeClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by_stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumed_by_stage_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_requirement: Option<ReuseRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_resolution: Option<ReuseResolution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_decision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_derivation_decision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableValidationReport {
    pub plan_valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePlannerPassSnapshot {
    pub pass_id: InspectablePlannerPassId,
    pub pass_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectablePlannerDiagnostics {
    pub validation: InspectableValidationReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pass_snapshots: Vec<InspectablePlannerPassSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InspectableProcessingPlan {
    #[serde(default = "default_inspectable_plan_schema_version")]
    pub schema_version: u32,
    pub plan_id: String,
    pub planning_mode: InspectablePlanningMode,
    pub source: InspectablePlanSource,
    pub source_identity: SourceSemanticIdentity,
    pub pipeline_identity: PipelineSemanticIdentity,
    pub operator_set_identity: OperatorSetIdentity,
    pub planner_profile_identity: PlannerProfileIdentity,
    pub semantic_plan: InspectableSemanticPlan,
    pub execution_plan: InspectableExecutionPlan,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<InspectablePlannedArtifact>,
    pub planner_diagnostics: InspectablePlannerDiagnostics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<InspectablePlanDecision>,
}

impl InspectableProcessingPlan {
    pub fn render_text_tree(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "plan {} ({})",
            self.plan_id,
            planning_mode_label(self.planning_mode)
        );
        let _ = writeln!(
            out,
            "source {} layout={}",
            self.source.store_path,
            layout_label(self.source.layout)
        );
        render_semantic_plan(&mut out, &self.semantic_plan);
        render_execution_plan(&mut out, &self.execution_plan);
        render_artifacts(&mut out, &self.artifacts);
        render_planner_diagnostics(&mut out, &self.planner_diagnostics);
        out.trim_end().to_string()
    }
}

fn render_semantic_plan(out: &mut String, plan: &InspectableSemanticPlan) {
    let _ = writeln!(out, "semantic_plan");
    let _ = writeln!(
        out,
        "  family={} name={} revision={}",
        pipeline_family_label(plan.pipeline_family),
        plan.pipeline_name.as_deref().unwrap_or("-"),
        plan.pipeline_revision
    );

    match &plan.root {
        InspectableSemanticRootNode::TraceLocal { trace_local } => {
            render_trace_local_plan(
                out,
                "  ",
                trace_local,
                InspectableBoundaryReason::FamilyRoot,
            );
        }
        InspectableSemanticRootNode::PostStackNeighborhood {
            pipeline,
            trace_local_prefix,
            operation_ids,
            operation_count,
        } => {
            if let Some(prefix) = trace_local_prefix {
                render_trace_local_plan(
                    out,
                    "  prefix ",
                    prefix,
                    InspectableBoundaryReason::TraceLocalPrefix,
                );
            }
            let _ = writeln!(
                out,
                "  neighborhood_ops count={} ids={}",
                operation_count,
                operator_list(operation_ids)
            );
            let _ = writeln!(
                out,
                "  pipeline_name={}",
                pipeline.name.as_deref().unwrap_or("-")
            );
        }
        InspectableSemanticRootNode::Subvolume {
            pipeline,
            trace_local_prefix,
            crop_operator_id,
        } => {
            if let Some(prefix) = trace_local_prefix {
                render_trace_local_plan(
                    out,
                    "  prefix ",
                    prefix,
                    InspectableBoundaryReason::TraceLocalPrefix,
                );
            }
            let _ = writeln!(
                out,
                "  crop operator_id={} pipeline_name={}",
                crop_operator_id,
                pipeline.name.as_deref().unwrap_or("-")
            );
        }
        InspectableSemanticRootNode::Gather {
            pipeline,
            trace_local_prefix,
            operation_ids,
            operation_count,
        } => {
            if let Some(prefix) = trace_local_prefix {
                render_trace_local_plan(
                    out,
                    "  prefix ",
                    prefix,
                    InspectableBoundaryReason::TraceLocalPrefix,
                );
            }
            let _ = writeln!(
                out,
                "  gather_ops count={} ids={}",
                operation_count,
                operator_list(operation_ids)
            );
            let _ = writeln!(
                out,
                "  pipeline_name={}",
                pipeline.name.as_deref().unwrap_or("-")
            );
        }
    }
}

fn render_trace_local_plan(
    out: &mut String,
    indent_prefix: &str,
    trace_local: &InspectableTraceLocalSemanticPlan,
    plan_reason: InspectableBoundaryReason,
) {
    let _ = writeln!(
        out,
        "{}trace_local name={} revision={} reason={}",
        indent_prefix,
        trace_local.pipeline.name.as_deref().unwrap_or("-"),
        trace_local.pipeline.revision,
        boundary_reason_label(plan_reason)
    );
    for segment in &trace_local.segments {
        let _ = writeln!(
            out,
            "{}  segment {}..{} steps={} boundary={} checkpoint={} ids={}",
            indent_prefix,
            segment.start_step_index,
            segment.end_step_index,
            segment.step_count,
            boundary_reason_label(segment.boundary_reason),
            segment.checkpoint,
            operator_list(&segment.operator_ids)
        );
    }
}

fn render_execution_plan(out: &mut String, plan: &InspectableExecutionPlan) {
    let _ = writeln!(out, "execution_plan");
    let _ = writeln!(
        out,
        "  summary compute_stages={} max_memory={} max_cpu={} max_io={} min_parallel={} expected_partitions={}",
        plan.summary.compute_stage_count,
        cost_class_label(plan.summary.max_memory_cost_class),
        cost_class_label(plan.summary.max_cpu_cost_class),
        cost_class_label(plan.summary.max_io_cost_class),
        parallel_efficiency_label(plan.summary.min_parallel_efficiency_class),
        option_usize_label(plan.summary.max_expected_partition_count)
    );
    let _ = writeln!(
        out,
        "  scheduler priority={} max_active_partitions={} expected_partition_count={}",
        priority_class_label(plan.scheduler_hints.priority_class),
        option_usize_label(plan.scheduler_hints.max_active_partitions),
        option_usize_label(plan.scheduler_hints.expected_partition_count)
    );
    for stage in &plan.stages {
        let _ = writeln!(
            out,
            "  stage {} kind={} label={} boundary={} partition={} expected_partitions={}",
            stage.stage_id,
            stage_kind_label(stage.stage_kind),
            stage.stage_label,
            stage
                .boundary_reason
                .map(boundary_reason_label)
                .unwrap_or("-"),
            partition_summary(&stage.partition),
            option_usize_label(stage.expected_partition_count)
        );
        if let Some(segment) = &stage.pipeline_segment {
            let _ = writeln!(
                out,
                "    segment family={} {}..{}",
                pipeline_family_label(segment.family),
                segment.start_step_index,
                segment.end_step_index
            );
        }
        if let Some(requirement) = &stage.reuse_requirement {
            let _ = writeln!(
                out,
                "    reuse required kind={} boundary={}",
                reuse_artifact_kind_label(requirement.artifact_kind),
                reuse_boundary_kind_label(requirement.boundary_kind)
            );
        }
        if let Some(resolution) = &stage.reuse_resolution {
            let _ = writeln!(
                out,
                "    reuse resolution reused={} miss_reason={}",
                resolution.reused,
                resolution
                    .miss_reason
                    .map(reuse_miss_reason_label)
                    .unwrap_or("-")
            );
        }
    }
}

fn render_artifacts(out: &mut String, artifacts: &[InspectablePlannedArtifact]) {
    let _ = writeln!(out, "artifacts");
    for artifact in artifacts {
        let _ = writeln!(
            out,
            "  artifact {} role={} produced_by={} consumed_by={}",
            artifact.artifact_id,
            artifact_role_label(artifact.role),
            artifact.produced_by_stage_id.as_deref().unwrap_or("-"),
            if artifact.consumed_by_stage_ids.is_empty() {
                "-".to_string()
            } else {
                artifact.consumed_by_stage_ids.join(",")
            }
        );
        if let Some(requirement) = &artifact.reuse_requirement {
            let _ = writeln!(
                out,
                "    reuse required kind={} boundary={}",
                reuse_artifact_kind_label(requirement.artifact_kind),
                reuse_boundary_kind_label(requirement.boundary_kind)
            );
        }
        if let Some(resolution) = &artifact.reuse_resolution {
            let _ = writeln!(
                out,
                "    reuse resolution reused={} miss_reason={}",
                resolution.reused,
                resolution
                    .miss_reason
                    .map(reuse_miss_reason_label)
                    .unwrap_or("-")
            );
        }
    }
}

fn render_planner_diagnostics(out: &mut String, diagnostics: &InspectablePlannerDiagnostics) {
    let _ = writeln!(
        out,
        "planner_diagnostics valid={} warnings={} blockers={} snapshots={}",
        diagnostics.validation.plan_valid,
        diagnostics.validation.warnings.len(),
        diagnostics.validation.blockers.len(),
        diagnostics.pass_snapshots.len()
    );
}

fn operator_list(ids: &[String]) -> String {
    if ids.is_empty() {
        "-".to_string()
    } else {
        ids.join(",")
    }
}

fn option_usize_label(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn layout_label(layout: SeismicLayout) -> &'static str {
    match layout {
        SeismicLayout::PostStack3D => "post_stack_3d",
        SeismicLayout::PostStack2D => "post_stack_2d",
        SeismicLayout::PreStack3DOffset => "pre_stack_3d_offset",
        SeismicLayout::PreStack3DAngle => "pre_stack_3d_angle",
        SeismicLayout::PreStack3DAzimuth => "pre_stack_3d_azimuth",
        SeismicLayout::PreStack3DUnknownAxis => "pre_stack_3d_unknown_axis",
        SeismicLayout::PreStack2DOffset => "pre_stack_2d_offset",
        SeismicLayout::ShotGatherSet => "shot_gather_set",
        SeismicLayout::ReceiverGatherSet => "receiver_gather_set",
        SeismicLayout::CmpGatherSet => "cmp_gather_set",
        SeismicLayout::UnstructuredTraceCollection => "unstructured_trace_collection",
    }
}

fn planning_mode_label(mode: InspectablePlanningMode) -> &'static str {
    match mode {
        InspectablePlanningMode::InteractivePreview => "interactive_preview",
        InspectablePlanningMode::ForegroundMaterialize => "foreground_materialize",
        InspectablePlanningMode::BackgroundBatch => "background_batch",
    }
}

fn priority_class_label(priority: InspectableExecutionPriorityClass) -> &'static str {
    match priority {
        InspectableExecutionPriorityClass::InteractivePreview => "interactive_preview",
        InspectableExecutionPriorityClass::ForegroundMaterialize => "foreground_materialize",
        InspectableExecutionPriorityClass::BackgroundBatch => "background_batch",
    }
}

fn pipeline_family_label(family: ProcessingPipelineFamily) -> &'static str {
    match family {
        ProcessingPipelineFamily::TraceLocal => "trace_local",
        ProcessingPipelineFamily::PostStackNeighborhood => "post_stack_neighborhood",
        ProcessingPipelineFamily::Subvolume => "subvolume",
        ProcessingPipelineFamily::Gather => "gather",
    }
}

fn boundary_reason_label(reason: InspectableBoundaryReason) -> &'static str {
    match reason {
        InspectableBoundaryReason::AuthoredCheckpoint => "authored_checkpoint",
        InspectableBoundaryReason::FinalOutput => "final_output",
        InspectableBoundaryReason::FamilyRoot => "family_root",
        InspectableBoundaryReason::TraceLocalPrefix => "trace_local_prefix",
        InspectableBoundaryReason::FamilyOperationBlock => "family_operation_block",
        InspectableBoundaryReason::GeometryBoundary => "geometry_boundary",
        InspectableBoundaryReason::ReuseDecision => "reuse_decision",
        InspectableBoundaryReason::ExternalInputFanIn => "external_input_fan_in",
    }
}

fn stage_kind_label(kind: InspectableExecutionStageKind) -> &'static str {
    match kind {
        InspectableExecutionStageKind::Compute => "compute",
        InspectableExecutionStageKind::Checkpoint => "checkpoint",
        InspectableExecutionStageKind::ReuseArtifact => "reuse_artifact",
        InspectableExecutionStageKind::FinalizeOutput => "finalize_output",
    }
}

fn artifact_role_label(role: InspectableExecutionArtifactRole) -> &'static str {
    match role {
        InspectableExecutionArtifactRole::Input => "input",
        InspectableExecutionArtifactRole::Checkpoint => "checkpoint",
        InspectableExecutionArtifactRole::FinalOutput => "final_output",
        InspectableExecutionArtifactRole::CachedReuse => "cached_reuse",
    }
}

fn reuse_artifact_kind_label(kind: ReuseArtifactKind) -> &'static str {
    match kind {
        ReuseArtifactKind::ExactVisibleFinal => "exact_visible_final",
        ReuseArtifactKind::VisibleCheckpoint => "visible_checkpoint",
        ReuseArtifactKind::PreviewPrefix => "preview_prefix",
    }
}

fn reuse_boundary_kind_label(kind: ReuseBoundaryKind) -> &'static str {
    match kind {
        ReuseBoundaryKind::ExactOutput => "exact_output",
        ReuseBoundaryKind::AuthoredCheckpoint => "authored_checkpoint",
        ReuseBoundaryKind::TraceLocalPrefix => "trace_local_prefix",
    }
}

fn reuse_miss_reason_label(reason: ReuseMissReason) -> &'static str {
    match reason {
        ReuseMissReason::UnresolvedAtPlanningTime => "unresolved_at_planning_time",
        ReuseMissReason::FreshComputeRequired => "fresh_compute_required",
        ReuseMissReason::NoReusableArtifactResolved => "no_reusable_artifact_resolved",
        ReuseMissReason::UnsupportedBoundary => "unsupported_boundary",
    }
}

fn cost_class_label(class: InspectableCostClass) -> &'static str {
    match class {
        InspectableCostClass::Low => "low",
        InspectableCostClass::Medium => "medium",
        InspectableCostClass::High => "high",
    }
}

fn parallel_efficiency_label(class: InspectableParallelEfficiencyClass) -> &'static str {
    match class {
        InspectableParallelEfficiencyClass::High => "high",
        InspectableParallelEfficiencyClass::Medium => "medium",
        InspectableParallelEfficiencyClass::Low => "low",
    }
}

fn partition_summary(partition: &InspectablePartitionPlan) -> String {
    let family = match partition.family {
        InspectablePartitionFamily::TileGroup => "tile_group",
        InspectablePartitionFamily::Section => "section",
        InspectablePartitionFamily::GatherGroup => "gather_group",
        InspectablePartitionFamily::FullVolume => "full_volume",
    };
    match partition.target_bytes {
        Some(target_bytes) => format!("{family} (~{} MiB target)", target_bytes / (1024 * 1024)),
        None => family.to_string(),
    }
}
