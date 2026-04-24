use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::identity::{
    operator_set_identity_for_pipeline, pipeline_semantic_identity,
    planner_profile_identity_for_pipeline,
};
use ophiolite_seismic::contracts::{
    OperatorSetIdentity, PipelineSemanticIdentity, PlannerProfileIdentity, ReuseArtifactKind,
    ReuseBoundaryKind, ReuseMissReason, ReuseRequirement, ReuseResolution, SourceSemanticIdentity,
    default_execution_plan_schema_version,
};
use ophiolite_seismic::{
    GatherProcessingOperation, GatherProcessingPipeline, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, ProcessingLayoutCompatibility,
    ProcessingOperatorDependencyProfile, ProcessingPipelineFamily, ProcessingPipelineSpec,
    ProcessingPlannerCostClass, ProcessingPlannerHints, ProcessingPlannerParallelEfficiencyClass,
    ProcessingPlannerPartitioningHint, ProcessingSampleDependency, ProcessingSpatialDependency,
    SectionAxis, SeismicLayout, SubvolumeCropOperation, TraceLocalProcessingOperation,
    gather_operator_planner_hints, post_stack_neighborhood_operator_planner_hints,
    subvolume_operator_planner_hints, trace_local_operator_planner_hints,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionOperatorScope {
    TraceLocal,
    PostStackNeighborhood,
    Subvolume,
    GatherMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PreferredPartitioning {
    TileGroup,
    Section,
    GatherGroup,
    FullVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SampleHaloRequirement {
    None,
    WholeTrace,
    BoundedWindowMs { window_ms_hint: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionSpatialDependency {
    SingleTrace,
    SectionNeighborhood,
    GatherNeighborhood,
    ExternalVolumePointwise,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum MemoryCostClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CpuCostClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum IoCostClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ParallelEfficiencyClass {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct OperatorExecutionTraits {
    pub operator_id: String,
    pub scope: ExecutionOperatorScope,
    pub layout_support: ProcessingLayoutCompatibility,
    pub preferred_partitioning: PreferredPartitioning,
    pub sample_halo: SampleHaloRequirement,
    pub spatial_dependency: ExecutionSpatialDependency,
    pub halo_inline: usize,
    pub halo_xline: usize,
    pub requires_full_volume: bool,
    pub checkpoint_safe: bool,
    pub deterministic: bool,
    pub preview_prefix_reuse_safe: bool,
    pub reuse_class: ReuseClass,
    pub memory_cost_class: MemoryCostClass,
    pub cpu_cost_class: CpuCostClass,
    pub io_cost_class: IoCostClass,
    pub parallel_efficiency_class: ParallelEfficiencyClass,
}

impl OperatorExecutionTraits {
    fn from_contract_semantics(
        operator_id: &str,
        scope: ExecutionOperatorScope,
        layout_support: ProcessingLayoutCompatibility,
        dependency_profile: ProcessingOperatorDependencyProfile,
        hints: ProcessingPlannerHints,
    ) -> Self {
        let reuse_class = reuse_class_from_contract_semantics(&dependency_profile, &hints);
        let preview_prefix_reuse_safe = reuse_class.allows_preview_prefix_reuse();
        let ProcessingPlannerHints {
            preferred_partitioning,
            requires_full_volume,
            checkpoint_safe,
            memory_cost_class,
            cpu_cost_class,
            io_cost_class,
            parallel_efficiency_class,
        } = hints;

        Self {
            operator_id: operator_id.to_string(),
            scope,
            layout_support,
            preferred_partitioning: preferred_partitioning_from_hint(preferred_partitioning),
            sample_halo: match dependency_profile.sample_dependency {
                ProcessingSampleDependency::Pointwise => SampleHaloRequirement::None,
                ProcessingSampleDependency::BoundedWindow { window_ms_hint } => {
                    SampleHaloRequirement::BoundedWindowMs { window_ms_hint }
                }
                ProcessingSampleDependency::WholeTrace => SampleHaloRequirement::WholeTrace,
            },
            spatial_dependency: match dependency_profile.spatial_dependency {
                ProcessingSpatialDependency::SingleTrace => ExecutionSpatialDependency::SingleTrace,
                ProcessingSpatialDependency::SectionNeighborhood => {
                    ExecutionSpatialDependency::SectionNeighborhood
                }
                ProcessingSpatialDependency::GatherNeighborhood => {
                    ExecutionSpatialDependency::GatherNeighborhood
                }
                ProcessingSpatialDependency::ExternalVolumePointwise => {
                    ExecutionSpatialDependency::ExternalVolumePointwise
                }
                ProcessingSpatialDependency::Global => ExecutionSpatialDependency::Global,
            },
            halo_inline: dependency_profile.inline_radius,
            halo_xline: dependency_profile.crossline_radius,
            requires_full_volume,
            checkpoint_safe,
            deterministic: dependency_profile.deterministic,
            preview_prefix_reuse_safe,
            reuse_class,
            memory_cost_class: memory_cost_class_from_hint(memory_cost_class),
            cpu_cost_class: cpu_cost_class_from_hint(cpu_cost_class),
            io_cost_class: io_cost_class_from_hint(io_cost_class),
            parallel_efficiency_class: parallel_efficiency_from_hint(parallel_efficiency_class),
        }
    }

    pub fn from_trace_local_operation(operation: &TraceLocalProcessingOperation) -> Self {
        Self::from_contract_semantics(
            operation.operator_id(),
            ExecutionOperatorScope::TraceLocal,
            operation.compatibility(),
            operation.dependency_profile(),
            trace_local_operator_planner_hints(operation),
        )
    }

    pub fn from_post_stack_neighborhood_operation(
        operation: &PostStackNeighborhoodProcessingOperation,
    ) -> Self {
        Self::from_contract_semantics(
            operation.operator_id(),
            ExecutionOperatorScope::PostStackNeighborhood,
            operation.compatibility(),
            operation.dependency_profile(),
            post_stack_neighborhood_operator_planner_hints(operation),
        )
    }

    pub fn from_gather_operation(operation: &GatherProcessingOperation) -> Self {
        Self::from_contract_semantics(
            operation.operator_id(),
            ExecutionOperatorScope::GatherMatrix,
            operation.compatibility(),
            operation.dependency_profile(),
            gather_operator_planner_hints(operation),
        )
    }

    pub fn from_subvolume_crop(crop: &SubvolumeCropOperation) -> Self {
        Self::from_contract_semantics(
            crop.operator_id(),
            ExecutionOperatorScope::Subvolume,
            crop.compatibility(),
            crop.dependency_profile(),
            subvolume_operator_planner_hints(crop),
        )
    }
}

fn reuse_class_from_contract_semantics(
    dependency_profile: &ProcessingOperatorDependencyProfile,
    hints: &ProcessingPlannerHints,
) -> ReuseClass {
    if hints.requires_full_volume {
        return ReuseClass::FullVolumeBarrier;
    }
    match dependency_profile.spatial_dependency {
        ProcessingSpatialDependency::ExternalVolumePointwise => {
            return ReuseClass::RequiresExternalInputs;
        }
        ProcessingSpatialDependency::Global => return ReuseClass::GeometryBarrier,
        ProcessingSpatialDependency::SingleTrace
        | ProcessingSpatialDependency::SectionNeighborhood
        | ProcessingSpatialDependency::GatherNeighborhood => {}
    }
    if !dependency_profile.same_section_ephemeral_reuse_safe {
        return ReuseClass::GeometryBarrier;
    }
    match (
        dependency_profile.sample_dependency,
        dependency_profile.spatial_dependency,
    ) {
        (ProcessingSampleDependency::Pointwise, ProcessingSpatialDependency::SingleTrace) => {
            ReuseClass::InPlaceSameWindow
        }
        (_, ProcessingSpatialDependency::SingleTrace) => ReuseClass::ReusableSameSection,
        (_, ProcessingSpatialDependency::SectionNeighborhood)
        | (_, ProcessingSpatialDependency::GatherNeighborhood) => ReuseClass::ReusableSameGeometry,
        _ => ReuseClass::GeometryBarrier,
    }
}

fn preferred_partitioning_from_hint(
    hint: ProcessingPlannerPartitioningHint,
) -> PreferredPartitioning {
    match hint {
        ProcessingPlannerPartitioningHint::TileGroup => PreferredPartitioning::TileGroup,
        ProcessingPlannerPartitioningHint::Section => PreferredPartitioning::Section,
        ProcessingPlannerPartitioningHint::GatherGroup => PreferredPartitioning::GatherGroup,
        ProcessingPlannerPartitioningHint::FullVolume => PreferredPartitioning::FullVolume,
    }
}

fn memory_cost_class_from_hint(hint: ProcessingPlannerCostClass) -> MemoryCostClass {
    match hint {
        ProcessingPlannerCostClass::Low => MemoryCostClass::Low,
        ProcessingPlannerCostClass::Medium => MemoryCostClass::Medium,
        ProcessingPlannerCostClass::High => MemoryCostClass::High,
    }
}

fn cpu_cost_class_from_hint(hint: ProcessingPlannerCostClass) -> CpuCostClass {
    match hint {
        ProcessingPlannerCostClass::Low => CpuCostClass::Low,
        ProcessingPlannerCostClass::Medium => CpuCostClass::Medium,
        ProcessingPlannerCostClass::High => CpuCostClass::High,
    }
}

fn io_cost_class_from_hint(hint: ProcessingPlannerCostClass) -> IoCostClass {
    match hint {
        ProcessingPlannerCostClass::Low => IoCostClass::Low,
        ProcessingPlannerCostClass::Medium => IoCostClass::Medium,
        ProcessingPlannerCostClass::High => IoCostClass::High,
    }
}

fn parallel_efficiency_from_hint(
    hint: ProcessingPlannerParallelEfficiencyClass,
) -> ParallelEfficiencyClass {
    match hint {
        ProcessingPlannerParallelEfficiencyClass::High => ParallelEfficiencyClass::High,
        ProcessingPlannerParallelEfficiencyClass::Medium => ParallelEfficiencyClass::Medium,
        ProcessingPlannerParallelEfficiencyClass::Low => ParallelEfficiencyClass::Low,
    }
}

pub fn operator_execution_traits_for_pipeline_spec(
    pipeline: &ProcessingPipelineSpec,
) -> Vec<OperatorExecutionTraits> {
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => pipeline
            .steps
            .iter()
            .map(|step| OperatorExecutionTraits::from_trace_local_operation(&step.operation))
            .collect(),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|trace_local| {
                trace_local.steps.iter().map(|step| {
                    OperatorExecutionTraits::from_trace_local_operation(&step.operation)
                })
            })
            .chain(
                pipeline
                    .operations
                    .iter()
                    .map(OperatorExecutionTraits::from_post_stack_neighborhood_operation),
            )
            .collect(),
        ProcessingPipelineSpec::Subvolume { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|trace_local| {
                trace_local.steps.iter().map(|step| {
                    OperatorExecutionTraits::from_trace_local_operation(&step.operation)
                })
            })
            .chain(std::iter::once(
                OperatorExecutionTraits::from_subvolume_crop(&pipeline.crop),
            ))
            .collect(),
        ProcessingPipelineSpec::Gather { pipeline } => pipeline
            .trace_local_pipeline
            .iter()
            .flat_map(|trace_local| {
                trace_local.steps.iter().map(|step| {
                    OperatorExecutionTraits::from_trace_local_operation(&step.operation)
                })
            })
            .chain(
                pipeline
                    .operations
                    .iter()
                    .map(OperatorExecutionTraits::from_gather_operation),
            )
            .collect(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PlanningMode {
    InteractivePreview,
    ForegroundMaterialize,
    BackgroundBatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionStageKind {
    Compute,
    Checkpoint,
    ReuseArtifact,
    FinalizeOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionArtifactRole {
    Input,
    Checkpoint,
    FinalOutput,
    CachedReuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PartitionFamily {
    TileGroup,
    Section,
    GatherGroup,
    FullVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PartitionOrdering {
    StorageOrder,
    InlineMajor,
    CrosslineMajor,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum Chunkability {
    TileSpan,
    FullVolumeOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DomainSpace {
    Survey,
    SectionAxis,
    StorageGrid,
    ExecutionPartition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VolumeDomain {
    pub shape: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionDomain {
    pub axis: SectionAxis,
    pub section_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionWindowDomain {
    pub axis: SectionAxis,
    pub section_index: usize,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub lod: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TileDomain {
    pub tile_index: [usize; 2],
    pub tile_origin: [usize; 3],
    pub tile_shape: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PartitionDomain {
    pub partition_index: usize,
    pub partition_count: usize,
    pub tile_range: [usize; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(rename_all = "snake_case")]
pub enum LogicalDomain {
    Volume { volume: VolumeDomain },
    Section { section: SectionDomain },
    SectionWindow { section_window: SectionWindowDomain },
    Tile { tile: TileDomain },
    Partition { partition: PartitionDomain },
}

impl LogicalDomain {
    pub fn domain_space(&self) -> DomainSpace {
        match self {
            Self::Volume { .. } => DomainSpace::Survey,
            Self::Section { .. } | Self::SectionWindow { .. } => DomainSpace::SectionAxis,
            Self::Tile { .. } => DomainSpace::StorageGrid,
            Self::Partition { .. } => DomainSpace::ExecutionPartition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(rename_all = "snake_case")]
pub enum ChunkGridSpec {
    Regular {
        origin: [usize; 3],
        chunk_shape: [usize; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GeometryFingerprints {
    pub survey_geometry_fingerprint: String,
    pub storage_grid_fingerprint: String,
    pub section_projection_fingerprint: String,
    pub artifact_lineage_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ReuseClass {
    InPlaceSameWindow,
    ReusableSameSection,
    ReusableSameGeometry,
    RequiresExternalInputs,
    GeometryBarrier,
    FullVolumeBarrier,
}

impl ReuseClass {
    pub fn allows_preview_prefix_reuse(self) -> bool {
        matches!(
            self,
            Self::InPlaceSameWindow | Self::ReusableSameSection | Self::ReusableSameGeometry
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum MaterializationClass {
    EphemeralWindow,
    EphemeralPartition,
    Checkpoint,
    PublishedOutput,
    ReusedArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ArtifactBoundaryReason {
    SourceInput,
    AuthoredCheckpoint,
    FinalOutput,
    GeometryDomainChange,
    ExternalInputFanIn,
    FullVolumeBarrier,
    TraceLocalPrefix,
    FamilyOperationBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ArtifactLifetimeClass {
    Source,
    Ephemeral,
    Checkpoint,
    Published,
    CachedReuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PlanningPassId {
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
pub enum PlanDecisionSubjectKind {
    PlannerPass,
    Stage,
    Artifact,
    Scheduler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PlanDecisionKind {
    Lowering,
    Scheduling,
    Reuse,
    ArtifactDerivation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DecisionFactor {
    pub code: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct StagePlanningDecision {
    pub selected_partition_family: PartitionFamily,
    pub selected_ordering: PartitionOrdering,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_target_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_expected_partition_count: Option<usize>,
    pub selected_queue_class: ExecutionQueueClass,
    pub selected_spillability: ExecutionSpillabilityClass,
    pub selected_exclusive_scope: ExecutionExclusiveScope,
    pub selected_preferred_partition_waves: usize,
    pub selected_reservation_bytes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factors: Vec<DecisionFactor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ReuseDecisionOutcome {
    Reused,
    Miss,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ReuseDecisionEvidence {
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
pub struct ReuseDecision {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_id: Option<String>,
    pub artifact_id: String,
    pub cache_mode: CacheMode,
    pub artifact_kind: ReuseArtifactKind,
    pub boundary_kind: ReuseBoundaryKind,
    pub candidate_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_reuse_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_artifact_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_store_path: Option<String>,
    pub outcome: ReuseDecisionOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub miss_reason: Option<ReuseMissReason>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<ReuseDecisionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ArtifactDerivation {
    pub artifact_id: String,
    pub artifact_key: ArtifactKey,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_artifact_ids: Vec<String>,
    pub logical_domain: LogicalDomain,
    pub chunk_grid_spec: ChunkGridSpec,
    pub geometry_fingerprints: GeometryFingerprints,
    pub materialization_class: MaterializationClass,
    pub boundary_reason: ArtifactBoundaryReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PlanDecision {
    pub decision_id: String,
    pub subject_kind: PlanDecisionSubjectKind,
    pub subject_id: String,
    pub decision_kind: PlanDecisionKind,
    pub reason_code: String,
    pub human_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_planning: Option<StagePlanningDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_decision: Option<ReuseDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_derivation: Option<ArtifactDerivation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ArtifactKey {
    pub lineage_digest: String,
    pub geometry_fingerprints: GeometryFingerprints,
    pub logical_domain: LogicalDomain,
    pub chunk_grid_spec: ChunkGridSpec,
    pub materialization_class: MaterializationClass,
    pub cache_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PartitionKey {
    pub artifact_key: ArtifactKey,
    pub partition_domain: PartitionDomain,
    pub generation: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ArtifactLiveSetEntry {
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_key: Option<ArtifactKey>,
    #[ts(type = "number")]
    pub estimated_resident_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ArtifactLiveSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resident_artifacts: Vec<ArtifactLiveSetEntry>,
    #[ts(type = "number")]
    pub estimated_resident_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CacheMode {
    PreferReuse,
    RequireReuse,
    FreshCompute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionPriorityClass {
    InteractivePreview,
    ForegroundMaterialize,
    BackgroundBatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionQueueClass {
    Control,
    InteractivePartition,
    ForegroundPartition,
    BackgroundPartition,
    ExclusiveFullVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionSpillabilityClass {
    Unspillable,
    Spillable,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionRetryGranularity {
    Job,
    Stage,
    Partition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionProgressGranularity {
    Stage,
    Partition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExecutionExclusiveScope {
    None,
    FullVolume,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionSourceDescriptor {
    pub store_path: String,
    pub layout: SeismicLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<[usize; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_shape: Option<[usize; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PipelineDescriptor {
    pub family: ProcessingPipelineFamily,
    pub name: Option<String>,
    pub schema_version: u32,
    pub revision: u32,
    pub content_digest: String,
    pub operator_set_version: String,
    pub planner_profile_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ArtifactDescriptor {
    pub artifact_id: String,
    pub role: ExecutionArtifactRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_key: Option<ArtifactKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_domain: Option<LogicalDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_grid_spec: Option<ChunkGridSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_fingerprints: Option<GeometryFingerprints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialization_class: Option<MaterializationClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_reason: Option<ArtifactBoundaryReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime_class: Option<ArtifactLifetimeClass>,
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
pub struct ExecutionPipelineSegment {
    pub family: ProcessingPipelineFamily,
    pub start_step_index: usize,
    pub end_step_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PartitionSpec {
    pub family: PartitionFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_partition_count: Option<usize>,
    pub ordering: PartitionOrdering,
    pub requires_barrier: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct HaloSpec {
    pub inline_radius: usize,
    pub xline_radius: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ChunkShapePolicy {
    InheritSource,
    PlannerSelected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RetryPolicy {
    pub max_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProgressUnits {
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CostEstimate {
    pub relative_cpu_cost: f32,
    pub estimated_peak_memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct StageExecutionClassification {
    pub max_memory_cost_class: MemoryCostClass,
    pub max_cpu_cost_class: CpuCostClass,
    pub max_io_cost_class: IoCostClass,
    pub min_parallel_efficiency_class: ParallelEfficiencyClass,
    pub combined_cpu_weight: f32,
    pub combined_io_weight: f32,
    pub uses_external_inputs: bool,
    pub requires_full_volume: bool,
    pub has_sample_halo: bool,
    pub has_spatial_halo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct StageMemoryProfile {
    pub chunkability: Chunkability,
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
pub struct StageResourceEnvelope {
    pub preferred_queue_class: ExecutionQueueClass,
    pub spillability: ExecutionSpillabilityClass,
    pub exclusive_scope: ExecutionExclusiveScope,
    pub retry_granularity: ExecutionRetryGranularity,
    pub progress_granularity: ExecutionProgressGranularity,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct StageResourceOwnership {
    pub input_artifact_ids: Vec<String>,
    pub output_artifact_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub live_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct LoweredStageSchedulerPolicy {
    pub queue_class: ExecutionQueueClass,
    pub spillability: ExecutionSpillabilityClass,
    pub exclusive_scope: ExecutionExclusiveScope,
    pub retry_granularity: ExecutionRetryGranularity,
    pub progress_granularity: ExecutionProgressGranularity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_partition_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_partition_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_partition_count: Option<usize>,
    pub effective_max_active_partitions: usize,
    pub preferred_partition_waves: usize,
    #[ts(type = "number")]
    pub reservation_bytes: u64,
    #[ts(type = "number")]
    pub resident_bytes_per_partition: u64,
    #[ts(type = "number")]
    pub workspace_bytes_per_worker: u64,
    pub ownership: StageResourceOwnership,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionStage {
    pub stage_id: String,
    pub stage_label: String,
    pub stage_kind: ExecutionStageKind,
    pub input_artifact_ids: Vec<String>,
    pub output_artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_segment: Option<ExecutionPipelineSegment>,
    pub partition_spec: PartitionSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_partition_count: Option<usize>,
    pub halo_spec: HaloSpec,
    pub chunk_shape_policy: ChunkShapePolicy,
    pub cache_mode: CacheMode,
    pub retry_policy: RetryPolicy,
    pub progress_units: ProgressUnits,
    pub classification: StageExecutionClassification,
    pub memory_cost_class: MemoryCostClass,
    pub estimated_cost: CostEstimate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_memory_profile: Option<StageMemoryProfile>,
    pub resource_envelope: StageResourceEnvelope,
    pub lowered_scheduler_policy: LoweredStageSchedulerPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_reason: Option<ArtifactBoundaryReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialization_class: Option<MaterializationClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_class: Option<ReuseClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_artifact_key: Option<ArtifactKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_set: Option<ArtifactLiveSet>,
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
pub struct ExecutionPlanSummary {
    pub compute_stage_count: usize,
    pub max_memory_cost_class: MemoryCostClass,
    pub max_cpu_cost_class: CpuCostClass,
    pub max_io_cost_class: IoCostClass,
    pub min_parallel_efficiency_class: ParallelEfficiencyClass,
    pub max_relative_cpu_cost: f32,
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
pub struct SchedulerHints {
    pub priority_class: ExecutionPriorityClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_active_partitions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_partition_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ValidationReport {
    pub plan_valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionRuntimeEnvironment {
    pub worker_budget: usize,
    pub memory_budget: ExecutionMemoryBudget,
    pub queue_class: ExecutionQueueClass,
    pub exclusive_scope: ExecutionExclusiveScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_max_active_partitions: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PlannerPassSnapshot {
    pub pass_id: PlanningPassId,
    pub pass_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PlannerDiagnostics {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pass_snapshots: Vec<PlannerPassSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionPlan {
    #[serde(default = "default_execution_plan_schema_version")]
    pub schema_version: u32,
    pub plan_id: String,
    pub planning_mode: PlanningMode,
    pub source: ExecutionSourceDescriptor,
    pub source_identity: SourceSemanticIdentity,
    pub pipeline: PipelineDescriptor,
    pub pipeline_identity: PipelineSemanticIdentity,
    pub operator_set_identity: OperatorSetIdentity,
    pub planner_profile_identity: PlannerProfileIdentity,
    pub runtime_environment: ExecutionRuntimeEnvironment,
    pub stages: Vec<ExecutionStage>,
    pub plan_summary: ExecutionPlanSummary,
    pub artifacts: Vec<ArtifactDescriptor>,
    pub scheduler_hints: SchedulerHints,
    pub validation: ValidationReport,
    pub planner_diagnostics: PlannerDiagnostics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_decisions: Vec<PlanDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionMemoryBudget {
    #[ts(type = "number")]
    pub usable_bytes: u64,
    #[ts(type = "number")]
    pub reserve_bytes: u64,
    pub worker_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ChunkPlanningMode {
    Conservative,
    Auto,
    Throughput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TraceLocalChunkPlanRecommendation {
    pub max_active_partitions: usize,
    pub tiles_per_partition: usize,
    pub partition_count: usize,
    #[ts(type = "number")]
    pub compatibility_target_bytes: u64,
    #[ts(type = "number")]
    pub resident_partition_bytes: u64,
    #[ts(type = "number")]
    pub global_worker_workspace_bytes: u64,
    #[ts(type = "number")]
    pub estimated_peak_bytes: u64,
}

impl PipelineDescriptor {
    pub fn from_pipeline_spec(pipeline: &ProcessingPipelineSpec) -> Self {
        let pipeline_identity = pipeline_semantic_identity(pipeline).ok();
        let operator_set_identity = operator_set_identity_for_pipeline(pipeline).ok();
        let planner_profile_identity = planner_profile_identity_for_pipeline(pipeline).ok();
        match pipeline {
            ProcessingPipelineSpec::TraceLocal { pipeline } => Self {
                family: ProcessingPipelineFamily::TraceLocal,
                name: pipeline.name.clone(),
                schema_version: pipeline.schema_version,
                revision: pipeline.revision,
                content_digest: pipeline_identity
                    .as_ref()
                    .map(|identity| identity.content_digest.clone())
                    .unwrap_or_default(),
                operator_set_version: operator_set_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
                planner_profile_version: planner_profile_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
            },
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => Self {
                family: ProcessingPipelineFamily::PostStackNeighborhood,
                name: pipeline.name.clone(),
                schema_version: pipeline.schema_version,
                revision: pipeline.revision,
                content_digest: pipeline_identity
                    .as_ref()
                    .map(|identity| identity.content_digest.clone())
                    .unwrap_or_default(),
                operator_set_version: operator_set_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
                planner_profile_version: planner_profile_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
            },
            ProcessingPipelineSpec::Subvolume { pipeline } => Self {
                family: ProcessingPipelineFamily::Subvolume,
                name: pipeline.name.clone(),
                schema_version: pipeline.schema_version,
                revision: pipeline.revision,
                content_digest: pipeline_identity
                    .as_ref()
                    .map(|identity| identity.content_digest.clone())
                    .unwrap_or_default(),
                operator_set_version: operator_set_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
                planner_profile_version: planner_profile_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
            },
            ProcessingPipelineSpec::Gather { pipeline } => Self {
                family: ProcessingPipelineFamily::Gather,
                name: pipeline.name.clone(),
                schema_version: pipeline.schema_version,
                revision: pipeline.revision,
                content_digest: pipeline_identity
                    .as_ref()
                    .map(|identity| identity.content_digest.clone())
                    .unwrap_or_default(),
                operator_set_version: operator_set_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
                planner_profile_version: planner_profile_identity
                    .as_ref()
                    .map(|identity| identity.version.clone())
                    .unwrap_or_default(),
            },
        }
    }
}

pub fn execution_stage_label(stage: &ExecutionStage, pipeline: &ProcessingPipelineSpec) -> String {
    if let Some(segment) = stage.pipeline_segment.as_ref() {
        match segment.family {
            ProcessingPipelineFamily::TraceLocal => {
                let trace_local_pipeline = match pipeline {
                    ProcessingPipelineSpec::TraceLocal { pipeline } => Some(pipeline),
                    ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
                        pipeline.trace_local_pipeline.as_ref()
                    }
                    ProcessingPipelineSpec::Subvolume { pipeline } => {
                        pipeline.trace_local_pipeline.as_ref()
                    }
                    ProcessingPipelineSpec::Gather { pipeline } => {
                        pipeline.trace_local_pipeline.as_ref()
                    }
                };
                if let Some(trace_local_pipeline) = trace_local_pipeline {
                    if let Some(operation) = trace_local_pipeline
                        .steps
                        .get(segment.end_step_index)
                        .map(|step| &step.operation)
                    {
                        return format!(
                            "Step {}: {}",
                            segment.end_step_index + 1,
                            execution_trace_local_operation_label(operation)
                        );
                    }
                }
            }
            ProcessingPipelineFamily::PostStackNeighborhood => {
                if let ProcessingPipelineSpec::PostStackNeighborhood { pipeline } = pipeline {
                    return execution_post_stack_progress_label(pipeline).to_string();
                }
            }
            ProcessingPipelineFamily::Subvolume => {
                if matches!(stage.stage_kind, ExecutionStageKind::FinalizeOutput) {
                    return "Crop Subvolume".to_string();
                }
            }
            ProcessingPipelineFamily::Gather => {
                if let ProcessingPipelineSpec::Gather { pipeline } = pipeline {
                    return execution_gather_progress_label(pipeline);
                }
            }
        }
    }

    match stage.stage_kind {
        ExecutionStageKind::FinalizeOutput => match pipeline {
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
                execution_post_stack_progress_label(pipeline).to_string()
            }
            ProcessingPipelineSpec::Subvolume { .. } => "Crop Subvolume".to_string(),
            ProcessingPipelineSpec::Gather { pipeline } => {
                execution_gather_progress_label(pipeline)
            }
            ProcessingPipelineSpec::TraceLocal { pipeline } => pipeline
                .steps
                .last()
                .map(|step| {
                    format!(
                        "Step {}: {}",
                        pipeline.steps.len(),
                        execution_trace_local_operation_label(&step.operation)
                    )
                })
                .unwrap_or_else(|| stage.output_artifact_id.clone()),
        },
        _ => stage.output_artifact_id.clone(),
    }
}

pub fn execution_trace_local_operation_label(
    operation: &TraceLocalProcessingOperation,
) -> &'static str {
    match operation {
        TraceLocalProcessingOperation::AmplitudeScalar { .. } => "Amplitude Scale",
        TraceLocalProcessingOperation::TraceRmsNormalize => "Trace RMS Normalize",
        TraceLocalProcessingOperation::AgcRms { .. } => "AGC RMS",
        TraceLocalProcessingOperation::PhaseRotation { .. } => "Phase Rotation",
        TraceLocalProcessingOperation::Envelope => "Envelope",
        TraceLocalProcessingOperation::InstantaneousPhase => "Instantaneous Phase",
        TraceLocalProcessingOperation::InstantaneousFrequency => "Instantaneous Frequency",
        TraceLocalProcessingOperation::Sweetness => "Sweetness",
        TraceLocalProcessingOperation::LowpassFilter { .. } => "Lowpass Filter",
        TraceLocalProcessingOperation::HighpassFilter { .. } => "Highpass Filter",
        TraceLocalProcessingOperation::BandpassFilter { .. } => "Bandpass Filter",
        TraceLocalProcessingOperation::VolumeArithmetic { .. } => "Volume Arithmetic",
    }
}

pub fn execution_post_stack_progress_label(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> &'static str {
    match pipeline.operations.first() {
        Some(PostStackNeighborhoodProcessingOperation::Similarity { .. }) => "Similarity",
        Some(PostStackNeighborhoodProcessingOperation::LocalVolumeStats { .. }) => {
            "Local Volume Stats"
        }
        Some(PostStackNeighborhoodProcessingOperation::Dip { .. }) => "Dip",
        None => "Neighborhood",
    }
}

pub fn execution_gather_progress_label(pipeline: &GatherProcessingPipeline) -> String {
    let operations = pipeline
        .operations
        .iter()
        .map(|operation| match operation {
            GatherProcessingOperation::NmoCorrection { .. } => "NMO Correction",
            GatherProcessingOperation::StretchMute { .. } => "Stretch Mute",
            GatherProcessingOperation::OffsetMute { .. } => "Offset Mute",
        })
        .collect::<Vec<_>>();
    if operations.is_empty() {
        "Gather".to_string()
    } else {
        operations.join(" -> ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{
        GatherInterpolationMode, GatherProcessingPipeline, PostStackNeighborhoodProcessingPipeline,
        PostStackNeighborhoodWindow, TraceLocalProcessingPipeline, TraceLocalProcessingStep,
        TraceLocalVolumeArithmeticOperator, VelocityFunctionSource,
    };

    #[test]
    fn trace_local_traits_capture_sample_dependency() {
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 7,
                preset_id: None,
                name: Some("test".to_string()),
                description: None,
                steps: vec![
                    TraceLocalProcessingStep {
                        operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 2.0 },
                        checkpoint: false,
                    },
                    TraceLocalProcessingStep {
                        operation: TraceLocalProcessingOperation::AgcRms { window_ms: 24.0 },
                        checkpoint: true,
                    },
                    TraceLocalProcessingStep {
                        operation: TraceLocalProcessingOperation::VolumeArithmetic {
                            operator: TraceLocalVolumeArithmeticOperator::Add,
                            secondary_store_path: "secondary.tbvol".to_string(),
                        },
                        checkpoint: false,
                    },
                ],
            },
        };

        let traits = operator_execution_traits_for_pipeline_spec(&pipeline);
        assert_eq!(traits.len(), 3);
        assert_eq!(traits[0].memory_cost_class, MemoryCostClass::Low);
        assert_eq!(traits[0].cpu_cost_class, CpuCostClass::Low);
        assert_eq!(traits[0].io_cost_class, IoCostClass::Low);
        assert_eq!(
            traits[1].sample_halo,
            SampleHaloRequirement::BoundedWindowMs {
                window_ms_hint: 24.0,
            }
        );
        assert_eq!(traits[1].cpu_cost_class, CpuCostClass::Medium);
        assert_eq!(
            traits[1].parallel_efficiency_class,
            ParallelEfficiencyClass::High
        );
        assert_eq!(
            traits[2].spatial_dependency,
            ExecutionSpatialDependency::ExternalVolumePointwise
        );
        assert_eq!(traits[2].io_cost_class, IoCostClass::Medium);
    }

    #[test]
    fn mixed_pipeline_traits_preserve_stage_order() {
        let pipeline = ProcessingPipelineSpec::PostStackNeighborhood {
            pipeline: PostStackNeighborhoodProcessingPipeline {
                schema_version: 1,
                revision: 3,
                preset_id: None,
                name: Some("similarity".to_string()),
                description: None,
                trace_local_pipeline: Some(TraceLocalProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: None,
                    description: None,
                    steps: vec![TraceLocalProcessingStep {
                        operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                        checkpoint: false,
                    }],
                }),
                operations: vec![PostStackNeighborhoodProcessingOperation::Similarity {
                    window: PostStackNeighborhoodWindow {
                        gate_ms: 16.0,
                        inline_stepout: 2,
                        xline_stepout: 3,
                    },
                }],
            },
        };

        let traits = operator_execution_traits_for_pipeline_spec(&pipeline);
        assert_eq!(traits.len(), 2);
        assert_eq!(traits[0].scope, ExecutionOperatorScope::TraceLocal);
        assert_eq!(
            traits[1].scope,
            ExecutionOperatorScope::PostStackNeighborhood
        );
        assert_eq!(traits[1].halo_inline, 2);
        assert_eq!(traits[1].halo_xline, 3);
        assert_eq!(traits[1].cpu_cost_class, CpuCostClass::High);
        assert_eq!(traits[1].io_cost_class, IoCostClass::High);
        assert_eq!(
            traits[1].parallel_efficiency_class,
            ParallelEfficiencyClass::Low
        );
    }

    #[test]
    fn pipeline_descriptor_uses_pipeline_metadata() {
        let pipeline = ProcessingPipelineSpec::Gather {
            pipeline: GatherProcessingPipeline {
                schema_version: 1,
                revision: 9,
                preset_id: None,
                name: Some("nmo".to_string()),
                description: None,
                trace_local_pipeline: None,
                operations: vec![GatherProcessingOperation::NmoCorrection {
                    velocity_model: VelocityFunctionSource::ConstantVelocity {
                        velocity_m_per_s: 2500.0,
                    },
                    interpolation: GatherInterpolationMode::Linear,
                }],
            },
        };

        let descriptor = PipelineDescriptor::from_pipeline_spec(&pipeline);
        assert_eq!(descriptor.family, ProcessingPipelineFamily::Gather);
        assert_eq!(descriptor.name.as_deref(), Some("nmo"));
        assert_eq!(descriptor.revision, 9);
    }
}
