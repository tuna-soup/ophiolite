use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use ophiolite_seismic::{
    GatherProcessingOperation, PostStackNeighborhoodProcessingOperation,
    ProcessingLayoutCompatibility, ProcessingPipelineFamily, ProcessingPipelineSpec, SeismicLayout,
    SubvolumeCropOperation, TraceLocalProcessingOperation,
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
    pub memory_cost_class: MemoryCostClass,
    pub cpu_cost_class: CpuCostClass,
    pub io_cost_class: IoCostClass,
    pub parallel_efficiency_class: ParallelEfficiencyClass,
}

impl OperatorExecutionTraits {
    pub fn from_trace_local_operation(operation: &TraceLocalProcessingOperation) -> Self {
        match operation {
            TraceLocalProcessingOperation::AmplitudeScalar { .. } => Self {
                operator_id: operation.operator_id().to_string(),
                scope: ExecutionOperatorScope::TraceLocal,
                layout_support: operation.compatibility(),
                preferred_partitioning: PreferredPartitioning::TileGroup,
                sample_halo: SampleHaloRequirement::None,
                spatial_dependency: ExecutionSpatialDependency::SingleTrace,
                halo_inline: 0,
                halo_xline: 0,
                requires_full_volume: false,
                checkpoint_safe: true,
                deterministic: true,
                preview_prefix_reuse_safe: true,
                memory_cost_class: MemoryCostClass::Low,
                cpu_cost_class: CpuCostClass::Low,
                io_cost_class: IoCostClass::Low,
                parallel_efficiency_class: ParallelEfficiencyClass::High,
            },
            TraceLocalProcessingOperation::TraceRmsNormalize
            | TraceLocalProcessingOperation::PhaseRotation { .. } => Self {
                operator_id: operation.operator_id().to_string(),
                scope: ExecutionOperatorScope::TraceLocal,
                layout_support: operation.compatibility(),
                preferred_partitioning: PreferredPartitioning::TileGroup,
                sample_halo: SampleHaloRequirement::WholeTrace,
                spatial_dependency: ExecutionSpatialDependency::SingleTrace,
                halo_inline: 0,
                halo_xline: 0,
                requires_full_volume: false,
                checkpoint_safe: true,
                deterministic: true,
                preview_prefix_reuse_safe: true,
                memory_cost_class: MemoryCostClass::Medium,
                cpu_cost_class: CpuCostClass::Medium,
                io_cost_class: IoCostClass::Low,
                parallel_efficiency_class: ParallelEfficiencyClass::High,
            },
            TraceLocalProcessingOperation::AgcRms { window_ms } => Self {
                operator_id: operation.operator_id().to_string(),
                scope: ExecutionOperatorScope::TraceLocal,
                layout_support: operation.compatibility(),
                preferred_partitioning: PreferredPartitioning::TileGroup,
                sample_halo: SampleHaloRequirement::BoundedWindowMs {
                    window_ms_hint: *window_ms,
                },
                spatial_dependency: ExecutionSpatialDependency::SingleTrace,
                halo_inline: 0,
                halo_xline: 0,
                requires_full_volume: false,
                checkpoint_safe: true,
                deterministic: true,
                preview_prefix_reuse_safe: true,
                memory_cost_class: MemoryCostClass::Medium,
                cpu_cost_class: CpuCostClass::Medium,
                io_cost_class: IoCostClass::Low,
                parallel_efficiency_class: ParallelEfficiencyClass::High,
            },
            TraceLocalProcessingOperation::Envelope
            | TraceLocalProcessingOperation::InstantaneousPhase
            | TraceLocalProcessingOperation::InstantaneousFrequency
            | TraceLocalProcessingOperation::Sweetness
            | TraceLocalProcessingOperation::LowpassFilter { .. }
            | TraceLocalProcessingOperation::HighpassFilter { .. }
            | TraceLocalProcessingOperation::BandpassFilter { .. } => Self {
                operator_id: operation.operator_id().to_string(),
                scope: ExecutionOperatorScope::TraceLocal,
                layout_support: operation.compatibility(),
                preferred_partitioning: PreferredPartitioning::TileGroup,
                sample_halo: SampleHaloRequirement::WholeTrace,
                spatial_dependency: ExecutionSpatialDependency::SingleTrace,
                halo_inline: 0,
                halo_xline: 0,
                requires_full_volume: false,
                checkpoint_safe: true,
                deterministic: true,
                preview_prefix_reuse_safe: true,
                memory_cost_class: MemoryCostClass::Medium,
                cpu_cost_class: CpuCostClass::High,
                io_cost_class: IoCostClass::Low,
                parallel_efficiency_class: ParallelEfficiencyClass::Medium,
            },
            TraceLocalProcessingOperation::VolumeArithmetic { .. } => Self {
                operator_id: operation.operator_id().to_string(),
                scope: ExecutionOperatorScope::TraceLocal,
                layout_support: operation.compatibility(),
                preferred_partitioning: PreferredPartitioning::TileGroup,
                sample_halo: SampleHaloRequirement::None,
                spatial_dependency: ExecutionSpatialDependency::ExternalVolumePointwise,
                halo_inline: 0,
                halo_xline: 0,
                requires_full_volume: false,
                checkpoint_safe: true,
                deterministic: true,
                preview_prefix_reuse_safe: true,
                memory_cost_class: MemoryCostClass::Medium,
                cpu_cost_class: CpuCostClass::Medium,
                io_cost_class: IoCostClass::Medium,
                parallel_efficiency_class: ParallelEfficiencyClass::High,
            },
        }
    }

    pub fn from_post_stack_neighborhood_operation(
        operation: &PostStackNeighborhoodProcessingOperation,
    ) -> Self {
        match operation {
            PostStackNeighborhoodProcessingOperation::Similarity { window }
            | PostStackNeighborhoodProcessingOperation::LocalVolumeStats { window, .. }
            | PostStackNeighborhoodProcessingOperation::Dip { window, .. } => Self {
                operator_id: operation.operator_id().to_string(),
                scope: ExecutionOperatorScope::PostStackNeighborhood,
                layout_support: operation.compatibility(),
                preferred_partitioning: PreferredPartitioning::TileGroup,
                sample_halo: SampleHaloRequirement::BoundedWindowMs {
                    window_ms_hint: window.gate_ms,
                },
                spatial_dependency: ExecutionSpatialDependency::SectionNeighborhood,
                halo_inline: window.inline_stepout,
                halo_xline: window.xline_stepout,
                requires_full_volume: false,
                checkpoint_safe: true,
                deterministic: true,
                preview_prefix_reuse_safe: true,
                memory_cost_class: MemoryCostClass::High,
                cpu_cost_class: CpuCostClass::High,
                io_cost_class: IoCostClass::High,
                parallel_efficiency_class: ParallelEfficiencyClass::Low,
            },
        }
    }

    pub fn from_gather_operation(operation: &GatherProcessingOperation) -> Self {
        let (memory_cost_class, cpu_cost_class, io_cost_class, parallel_efficiency_class) =
            match operation {
                GatherProcessingOperation::OffsetMute { .. } => (
                    MemoryCostClass::Low,
                    CpuCostClass::Low,
                    IoCostClass::Low,
                    ParallelEfficiencyClass::Medium,
                ),
                GatherProcessingOperation::NmoCorrection { .. }
                | GatherProcessingOperation::StretchMute { .. } => (
                    MemoryCostClass::High,
                    CpuCostClass::High,
                    IoCostClass::Medium,
                    ParallelEfficiencyClass::Low,
                ),
            };

        Self {
            operator_id: operation.operator_id().to_string(),
            scope: ExecutionOperatorScope::GatherMatrix,
            layout_support: operation.compatibility(),
            preferred_partitioning: PreferredPartitioning::GatherGroup,
            sample_halo: SampleHaloRequirement::WholeTrace,
            spatial_dependency: ExecutionSpatialDependency::GatherNeighborhood,
            halo_inline: 0,
            halo_xline: 0,
            requires_full_volume: false,
            checkpoint_safe: true,
            deterministic: true,
            preview_prefix_reuse_safe: false,
            memory_cost_class,
            cpu_cost_class,
            io_cost_class,
            parallel_efficiency_class,
        }
    }

    pub fn from_subvolume_crop(_crop: &SubvolumeCropOperation) -> Self {
        Self {
            operator_id: "subvolume_crop".to_string(),
            scope: ExecutionOperatorScope::Subvolume,
            layout_support: ProcessingLayoutCompatibility::PostStackOnly,
            preferred_partitioning: PreferredPartitioning::TileGroup,
            sample_halo: SampleHaloRequirement::None,
            spatial_dependency: ExecutionSpatialDependency::SingleTrace,
            halo_inline: 0,
            halo_xline: 0,
            requires_full_volume: false,
            checkpoint_safe: true,
            deterministic: true,
            preview_prefix_reuse_safe: false,
            memory_cost_class: MemoryCostClass::Low,
            cpu_cost_class: CpuCostClass::Low,
            io_cost_class: IoCostClass::Medium,
            parallel_efficiency_class: ParallelEfficiencyClass::High,
        }
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
    pub revision: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ArtifactDescriptor {
    pub artifact_id: String,
    pub role: ExecutionArtifactRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionStage {
    pub stage_id: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub planning_mode: PlanningMode,
    pub source: ExecutionSourceDescriptor,
    pub pipeline: PipelineDescriptor,
    pub stages: Vec<ExecutionStage>,
    pub plan_summary: ExecutionPlanSummary,
    pub artifacts: Vec<ArtifactDescriptor>,
    pub scheduler_hints: SchedulerHints,
    pub validation: ValidationReport,
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
        match pipeline {
            ProcessingPipelineSpec::TraceLocal { pipeline } => Self {
                family: ProcessingPipelineFamily::TraceLocal,
                name: pipeline.name.clone(),
                revision: pipeline.revision,
            },
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => Self {
                family: ProcessingPipelineFamily::PostStackNeighborhood,
                name: pipeline.name.clone(),
                revision: pipeline.revision,
            },
            ProcessingPipelineSpec::Subvolume { pipeline } => Self {
                family: ProcessingPipelineFamily::Subvolume,
                name: pipeline.name.clone(),
                revision: pipeline.revision,
            },
            ProcessingPipelineSpec::Gather { pipeline } => Self {
                family: ProcessingPipelineFamily::Gather,
                name: pipeline.name.clone(),
                revision: pipeline.revision,
            },
        }
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
