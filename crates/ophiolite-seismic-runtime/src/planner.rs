use std::collections::BTreeSet;
use std::mem::size_of;

use uuid::Uuid;

use ophiolite_seismic::{
    ProcessingPipelineFamily, ProcessingPipelineSpec, SeismicLayout, TraceLocalProcessingOperation,
    TraceLocalProcessingPipeline,
};

use crate::execution::{
    ArtifactDescriptor, CacheMode, ChunkPlanningMode, ChunkShapePolicy, Chunkability, CostEstimate,
    CpuCostClass, ExecutionArtifactRole, ExecutionMemoryBudget, ExecutionPipelineSegment,
    ExecutionPlan, ExecutionPlanSummary, ExecutionPriorityClass, ExecutionSourceDescriptor,
    ExecutionStage, ExecutionStageKind, HaloSpec, IoCostClass, MemoryCostClass,
    OperatorExecutionTraits, ParallelEfficiencyClass, PartitionFamily, PartitionOrdering,
    PartitionSpec, PipelineDescriptor, PlanningMode, PreferredPartitioning, ProgressUnits,
    RetryPolicy, SampleHaloRequirement, SchedulerHints, StageExecutionClassification,
    StageMemoryProfile, TraceLocalChunkPlanRecommendation, ValidationReport,
    operator_execution_traits_for_pipeline_spec,
};
use crate::trace_local_chunk_planning::{
    compile_trace_local_chunk_plan, recommendation_from_chunk_plan,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PlanProcessingRequest {
    pub store_path: String,
    pub layout: SeismicLayout,
    pub source_shape: Option<[usize; 3]>,
    pub source_chunk_shape: Option<[usize; 3]>,
    pub pipeline: ProcessingPipelineSpec,
    pub output_store_path: Option<String>,
    pub planning_mode: PlanningMode,
    pub max_active_partitions: Option<usize>,
}

pub fn build_execution_plan(request: &PlanProcessingRequest) -> Result<ExecutionPlan, String> {
    let pipeline_descriptor = PipelineDescriptor::from_pipeline_spec(&request.pipeline);
    let operator_traits = operator_execution_traits_for_pipeline_spec(&request.pipeline);
    let validation = validation_report_for_layout(request.layout, &operator_traits);
    if !validation.plan_valid {
        return Err(validation.blockers.join("; "));
    }

    let source_artifact_id = "source".to_string();
    let mut artifacts = vec![ArtifactDescriptor {
        artifact_id: source_artifact_id.clone(),
        role: ExecutionArtifactRole::Input,
        store_path: Some(request.store_path.clone()),
        cache_key: None,
    }];

    let (stages, mut derived_artifacts) = match &request.pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => build_trace_local_stages(
            pipeline,
            &source_artifact_id,
            request.output_store_path.as_deref(),
            &operator_traits,
            request.source_shape,
            request.source_chunk_shape,
        ),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
            let label = pipeline
                .name
                .clone()
                .unwrap_or_else(|| "post_stack_neighborhood".to_string());
            build_single_stage_plan(
                &pipeline_descriptor,
                &source_artifact_id,
                request.output_store_path.as_deref(),
                &operator_traits,
                label,
                request.source_shape,
                request.source_chunk_shape,
            )
        }
        ProcessingPipelineSpec::Subvolume { pipeline } => {
            let label = pipeline
                .name
                .clone()
                .unwrap_or_else(|| "subvolume".to_string());
            build_single_stage_plan(
                &pipeline_descriptor,
                &source_artifact_id,
                request.output_store_path.as_deref(),
                &operator_traits,
                label,
                request.source_shape,
                request.source_chunk_shape,
            )
        }
        ProcessingPipelineSpec::Gather { pipeline } => {
            let label = pipeline
                .name
                .clone()
                .unwrap_or_else(|| "gather".to_string());
            build_single_stage_plan(
                &pipeline_descriptor,
                &source_artifact_id,
                request.output_store_path.as_deref(),
                &operator_traits,
                label,
                request.source_shape,
                request.source_chunk_shape,
            )
        }
    };
    artifacts.append(&mut derived_artifacts);
    let expected_partition_count = expected_partition_count_for_stages(&stages);

    Ok(ExecutionPlan {
        plan_id: format!("plan-{}", Uuid::new_v4()),
        planning_mode: request.planning_mode,
        source: ExecutionSourceDescriptor {
            store_path: request.store_path.clone(),
            layout: request.layout,
            shape: request.source_shape,
            chunk_shape: request.source_chunk_shape,
        },
        pipeline: pipeline_descriptor,
        plan_summary: execution_plan_summary_for_stages(&stages),
        stages,
        artifacts,
        scheduler_hints: SchedulerHints {
            priority_class: priority_for_mode(request.planning_mode),
            max_active_partitions: request.max_active_partitions,
            expected_partition_count,
        },
        validation,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptivePartitionTargetRecommendation {
    pub chunk_plan: TraceLocalChunkPlanRecommendation,
    pub bytes_per_tile: u64,
    pub total_tiles: usize,
    pub preferred_partition_count: usize,
    pub available_memory_bytes: Option<u64>,
    pub reserved_memory_bytes: u64,
    pub usable_memory_bytes: Option<u64>,
}

impl AdaptivePartitionTargetRecommendation {
    pub fn trace_local_chunk_plan(&self) -> TraceLocalChunkPlanRecommendation {
        self.chunk_plan.clone()
    }

    pub fn target_bytes(&self) -> u64 {
        self.chunk_plan.compatibility_target_bytes
    }

    pub fn recommended_partition_count(&self) -> usize {
        self.chunk_plan.partition_count
    }

    pub fn recommended_max_active_partitions(&self) -> usize {
        self.chunk_plan.max_active_partitions
    }

    pub fn tiles_per_partition(&self) -> usize {
        self.chunk_plan.tiles_per_partition
    }

    pub fn resident_partition_bytes(&self) -> u64 {
        self.chunk_plan.resident_partition_bytes
    }

    pub fn global_worker_workspace_bytes(&self) -> u64 {
        self.chunk_plan.global_worker_workspace_bytes
    }

    pub fn estimated_peak_bytes(&self) -> u64 {
        self.chunk_plan.estimated_peak_bytes
    }
}

pub fn recommend_adaptive_partition_target(
    plan: &ExecutionPlan,
    source_shape: [usize; 3],
    source_chunk_shape: [usize; 3],
    worker_count: usize,
    available_memory_bytes: Option<u64>,
) -> AdaptivePartitionTargetRecommendation {
    recommend_adaptive_partition_target_for_job_concurrency(
        plan,
        source_shape,
        source_chunk_shape,
        worker_count,
        available_memory_bytes,
        1,
    )
}

pub fn recommend_adaptive_partition_target_for_job_concurrency(
    plan: &ExecutionPlan,
    source_shape: [usize; 3],
    source_chunk_shape: [usize; 3],
    worker_count: usize,
    available_memory_bytes: Option<u64>,
    concurrent_job_count: usize,
) -> AdaptivePartitionTargetRecommendation {
    trace_local_chunk_plan_recommendation(
        plan,
        source_shape,
        source_chunk_shape,
        worker_count,
        available_memory_bytes,
        concurrent_job_count,
    )
    .unwrap_or_else(|| {
        legacy_adaptive_partition_target_recommendation(
            plan,
            source_shape,
            source_chunk_shape,
            worker_count,
            available_memory_bytes,
            concurrent_job_count,
        )
    })
}

pub fn recommend_trace_local_chunk_plan_for_execution(
    plan: &ExecutionPlan,
    worker_count: usize,
    available_memory_bytes: Option<u64>,
    concurrent_job_count: usize,
) -> Option<AdaptivePartitionTargetRecommendation> {
    trace_local_chunk_plan_recommendation(
        plan,
        plan.source.shape?,
        plan.source.chunk_shape?,
        worker_count,
        available_memory_bytes,
        concurrent_job_count,
    )
}

fn trace_local_chunk_plan_recommendation(
    plan: &ExecutionPlan,
    source_shape: [usize; 3],
    source_chunk_shape: [usize; 3],
    worker_count: usize,
    available_memory_bytes: Option<u64>,
    concurrent_job_count: usize,
) -> Option<AdaptivePartitionTargetRecommendation> {
    let chunk_inline = source_chunk_shape[0].max(1).min(source_shape[0].max(1));
    let chunk_xline = source_chunk_shape[1].max(1).min(source_shape[1].max(1));
    let chunk_samples = source_chunk_shape[2].max(1).min(source_shape[2].max(1));
    let total_tiles =
        source_shape[0].div_ceil(chunk_inline) * source_shape[1].div_ceil(chunk_xline);
    let bytes_per_tile = (chunk_inline as u64 * chunk_xline as u64 * chunk_samples as u64 * 4)
        .saturating_add(chunk_inline as u64 * chunk_xline as u64)
        .max(1);
    let stage_memory_profiles = plan
        .stages
        .iter()
        .filter_map(|stage| stage.stage_memory_profile.as_ref().cloned())
        .collect::<Vec<_>>();
    if stage_memory_profiles.is_empty() {
        return None;
    }

    let worker_count = worker_count.max(1);
    let concurrent_job_count = concurrent_job_count.max(1);
    let desired_active_partitions = total_tiles.max(1).min(worker_count);
    let preferred_partition_waves = match chunk_planning_mode_for_planning_mode(plan.planning_mode)
    {
        ChunkPlanningMode::Conservative => 1usize,
        ChunkPlanningMode::Auto => 2usize,
        ChunkPlanningMode::Throughput => 1usize,
    };
    let heuristic_preferred_partition_count = total_tiles.max(1).min(
        desired_active_partitions
            .saturating_mul(preferred_partition_waves)
            .max(1),
    );
    let base_preferred_partition_count = plan
        .plan_summary
        .max_expected_partition_count
        .unwrap_or(heuristic_preferred_partition_count)
        .max(1);
    let preferred_partition_count =
        if matches!(plan.planning_mode, PlanningMode::BackgroundBatch) {
            base_preferred_partition_count.saturating_mul(concurrent_job_count)
        } else {
            base_preferred_partition_count
        }
        .min(total_tiles.max(1))
        .max(1);
    let effective_available_memory_bytes =
        available_memory_bytes.map(|available| available / concurrent_job_count as u64);
    let reserved_memory_bytes =
        reserved_memory_bytes_for_plan(plan, effective_available_memory_bytes);
    let usable_memory_bytes = effective_available_memory_bytes
        .map(|available| available.saturating_sub(reserved_memory_bytes));
    let fallback_usable_bytes = 512_u64
        .saturating_mul(1024 * 1024)
        .saturating_mul(worker_count as u64);
    let budget = ExecutionMemoryBudget {
        usable_bytes: usable_memory_bytes
            .unwrap_or(fallback_usable_bytes)
            .max(bytes_per_tile),
        reserve_bytes: reserved_memory_bytes,
        worker_count,
    };
    let compiled = compile_trace_local_chunk_plan(
        total_tiles,
        PartitionOrdering::StorageOrder,
        &stage_memory_profiles,
        &budget,
        chunk_planning_mode_for_planning_mode(plan.planning_mode),
        Some(preferred_partition_count),
    )?;
    let recommendation = recommendation_from_chunk_plan(&compiled);

    Some(AdaptivePartitionTargetRecommendation {
        chunk_plan: recommendation,
        bytes_per_tile,
        total_tiles,
        preferred_partition_count,
        available_memory_bytes: effective_available_memory_bytes,
        reserved_memory_bytes,
        usable_memory_bytes,
    })
}

fn legacy_adaptive_partition_target_recommendation(
    plan: &ExecutionPlan,
    source_shape: [usize; 3],
    source_chunk_shape: [usize; 3],
    worker_count: usize,
    available_memory_bytes: Option<u64>,
    concurrent_job_count: usize,
) -> AdaptivePartitionTargetRecommendation {
    let chunk_inline = source_chunk_shape[0].max(1).min(source_shape[0].max(1));
    let chunk_xline = source_chunk_shape[1].max(1).min(source_shape[1].max(1));
    let chunk_samples = source_chunk_shape[2].max(1).min(source_shape[2].max(1));
    let total_tiles =
        source_shape[0].div_ceil(chunk_inline) * source_shape[1].div_ceil(chunk_xline);
    let bytes_per_tile = (chunk_inline as u64 * chunk_xline as u64 * chunk_samples as u64 * 4)
        .saturating_add(chunk_inline as u64 * chunk_xline as u64)
        .max(1);
    let worker_count = worker_count.max(1);
    let concurrent_job_count = concurrent_job_count.max(1);
    let desired_active_partitions = total_tiles.max(1).min(worker_count);
    let preferred_partition_waves = if plan.plan_summary.max_relative_cpu_cost >= 4.0 {
        1usize
    } else {
        2usize
    };
    let base_preferred_partition_count = total_tiles.max(1).min(
        desired_active_partitions
            .saturating_mul(preferred_partition_waves)
            .max(1),
    );
    let preferred_partition_count =
        if matches!(plan.planning_mode, PlanningMode::BackgroundBatch) {
            base_preferred_partition_count.saturating_mul(concurrent_job_count)
        } else {
            base_preferred_partition_count
        }
        .min(total_tiles.max(1))
        .max(1);
    let target_by_parallelism = bytes_per_tile.saturating_mul(
        total_tiles
            .div_ceil(preferred_partition_count.max(1))
            .max(1) as u64,
    );
    let effective_available_memory_bytes =
        available_memory_bytes.map(|available| available / concurrent_job_count as u64);
    let reserved_memory_bytes =
        reserved_memory_bytes_for_plan(plan, effective_available_memory_bytes);
    let usable_memory_bytes = effective_available_memory_bytes
        .map(|available| available.saturating_sub(reserved_memory_bytes));
    let max_target_by_memory = usable_memory_bytes
        .map(|usable| {
            let per_active_partition_budget = usable / desired_active_partitions.max(1) as u64;
            per_active_partition_budget
                .saturating_sub(plan.plan_summary.max_estimated_peak_memory_bytes)
                .max(bytes_per_tile)
        })
        .unwrap_or(512 * 1024 * 1024);
    let target_bytes = target_by_parallelism
        .min(max_target_by_memory.max(bytes_per_tile))
        .max(bytes_per_tile);
    let tiles_per_partition = (target_bytes / bytes_per_tile).max(1) as usize;
    let recommended_partition_count = total_tiles.div_ceil(tiles_per_partition.max(1)).max(1);

    AdaptivePartitionTargetRecommendation {
        chunk_plan: TraceLocalChunkPlanRecommendation {
            max_active_partitions: desired_active_partitions.max(1),
            tiles_per_partition,
            partition_count: recommended_partition_count,
            compatibility_target_bytes: target_bytes,
            resident_partition_bytes: bytes_per_tile.saturating_mul(tiles_per_partition as u64),
            global_worker_workspace_bytes: 0,
            estimated_peak_bytes: reserved_memory_bytes
                .saturating_add(bytes_per_tile.saturating_mul(tiles_per_partition as u64)),
        },
        bytes_per_tile,
        total_tiles,
        preferred_partition_count,
        available_memory_bytes: effective_available_memory_bytes,
        reserved_memory_bytes,
        usable_memory_bytes,
    }
}

fn build_trace_local_stages(
    pipeline: &TraceLocalProcessingPipeline,
    source_artifact_id: &str,
    output_store_path: Option<&str>,
    operator_traits: &[OperatorExecutionTraits],
    source_shape: Option<[usize; 3]>,
    source_chunk_shape: Option<[usize; 3]>,
) -> (Vec<ExecutionStage>, Vec<ArtifactDescriptor>) {
    let mut stages = Vec::new();
    let mut artifacts = Vec::new();
    let mut current_input_artifact_id = source_artifact_id.to_string();
    let mut segment_start = 0usize;

    for (index, step) in pipeline.steps.iter().enumerate() {
        let is_final_step = index + 1 == pipeline.steps.len();
        if !step.checkpoint && !is_final_step {
            continue;
        }

        let stage_index = stages.len() + 1;
        let is_final_stage = is_final_step;
        let output_artifact_id = if is_final_stage {
            "final-output".to_string()
        } else {
            format!("checkpoint-{stage_index:02}")
        };
        let role = if is_final_stage {
            ExecutionArtifactRole::FinalOutput
        } else {
            ExecutionArtifactRole::Checkpoint
        };
        let stage_traits = &operator_traits[segment_start..=index];
        let stage_memory_profile = stage_memory_profile_for_trace_local_steps(
            &pipeline.steps[segment_start..=index],
            stage_traits,
            source_chunk_shape,
        );
        let cache_mode = CacheMode::PreferReuse;
        let stage_kind = if is_final_stage {
            ExecutionStageKind::FinalizeOutput
        } else {
            ExecutionStageKind::Checkpoint
        };
        let store_path = if is_final_stage {
            output_store_path.map(str::to_string)
        } else {
            None
        };

        artifacts.push(ArtifactDescriptor {
            artifact_id: output_artifact_id.clone(),
            role,
            store_path,
            cache_key: None,
        });

        let partition_spec = partition_spec_for_traits(stage_traits);
        let classification = stage_execution_classification_for_traits(stage_traits);
        stages.push(ExecutionStage {
            stage_id: format!("stage-{stage_index:02}"),
            stage_kind,
            input_artifact_ids: vec![current_input_artifact_id.clone()],
            output_artifact_id: output_artifact_id.clone(),
            pipeline_segment: Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::TraceLocal,
                start_step_index: segment_start,
                end_step_index: index,
            }),
            expected_partition_count: estimate_partition_count(
                &partition_spec,
                source_shape,
                source_chunk_shape,
            ),
            partition_spec,
            halo_spec: halo_spec_for_traits(stage_traits),
            chunk_shape_policy: ChunkShapePolicy::InheritSource,
            cache_mode,
            retry_policy: RetryPolicy { max_attempts: 1 },
            progress_units: ProgressUnits { total: 1 },
            classification: classification.clone(),
            memory_cost_class: classification.max_memory_cost_class,
            estimated_cost: cost_estimate_for_traits(stage_traits),
            stage_memory_profile,
        });

        current_input_artifact_id = output_artifact_id;
        segment_start = index + 1;
    }

    (stages, artifacts)
}

fn build_single_stage_plan(
    pipeline_descriptor: &PipelineDescriptor,
    source_artifact_id: &str,
    output_store_path: Option<&str>,
    operator_traits: &[OperatorExecutionTraits],
    artifact_label: String,
    source_shape: Option<[usize; 3]>,
    source_chunk_shape: Option<[usize; 3]>,
) -> (Vec<ExecutionStage>, Vec<ArtifactDescriptor>) {
    let output_artifact_id = "final-output".to_string();
    let partition_spec = partition_spec_for_traits(operator_traits);
    let classification = stage_execution_classification_for_traits(operator_traits);
    let stage = ExecutionStage {
        stage_id: "stage-01".to_string(),
        stage_kind: ExecutionStageKind::FinalizeOutput,
        input_artifact_ids: vec![source_artifact_id.to_string()],
        output_artifact_id: output_artifact_id.clone(),
        pipeline_segment: Some(ExecutionPipelineSegment {
            family: pipeline_descriptor.family,
            start_step_index: 0,
            end_step_index: operator_traits.len().saturating_sub(1),
        }),
        expected_partition_count: estimate_partition_count(
            &partition_spec,
            source_shape,
            source_chunk_shape,
        ),
        partition_spec,
        halo_spec: halo_spec_for_traits(operator_traits),
        chunk_shape_policy: ChunkShapePolicy::InheritSource,
        cache_mode: CacheMode::PreferReuse,
        retry_policy: RetryPolicy { max_attempts: 1 },
        progress_units: ProgressUnits { total: 1 },
        classification: classification.clone(),
        memory_cost_class: classification.max_memory_cost_class,
        estimated_cost: cost_estimate_for_traits(operator_traits),
        stage_memory_profile: None,
    };
    let artifact = ArtifactDescriptor {
        artifact_id: output_artifact_id,
        role: ExecutionArtifactRole::FinalOutput,
        store_path: output_store_path
            .map(str::to_string)
            .or(Some(artifact_label)),
        cache_key: None,
    };
    (vec![stage], vec![artifact])
}

fn validation_report_for_layout(
    layout: SeismicLayout,
    operator_traits: &[OperatorExecutionTraits],
) -> ValidationReport {
    let blockers: Vec<String> = operator_traits
        .iter()
        .filter(|traits| !traits.layout_support.supports_layout(layout))
        .map(|traits| {
            format!(
                "operator '{}' does not support layout {:?}",
                traits.operator_id, layout
            )
        })
        .collect();
    ValidationReport {
        plan_valid: blockers.is_empty(),
        warnings: Vec::new(),
        blockers,
    }
}

fn priority_for_mode(mode: PlanningMode) -> ExecutionPriorityClass {
    match mode {
        PlanningMode::InteractivePreview => ExecutionPriorityClass::InteractivePreview,
        PlanningMode::ForegroundMaterialize => ExecutionPriorityClass::ForegroundMaterialize,
        PlanningMode::BackgroundBatch => ExecutionPriorityClass::BackgroundBatch,
    }
}

fn partition_spec_for_traits(operator_traits: &[OperatorExecutionTraits]) -> PartitionSpec {
    let preferred = operator_traits
        .iter()
        .find(|traits| {
            matches!(
                traits.preferred_partitioning,
                PreferredPartitioning::FullVolume
            )
        })
        .map(|traits| traits.preferred_partitioning)
        .or_else(|| {
            operator_traits
                .first()
                .map(|traits| traits.preferred_partitioning)
        })
        .unwrap_or(PreferredPartitioning::TileGroup);

    let family = match preferred {
        PreferredPartitioning::TileGroup => PartitionFamily::TileGroup,
        PreferredPartitioning::Section => PartitionFamily::Section,
        PreferredPartitioning::GatherGroup => PartitionFamily::GatherGroup,
        PreferredPartitioning::FullVolume => PartitionFamily::FullVolume,
    };

    PartitionSpec {
        family,
        target_bytes: match family {
            PartitionFamily::FullVolume => None,
            PartitionFamily::TileGroup => Some(256 * 1024 * 1024),
            PartitionFamily::Section => Some(128 * 1024 * 1024),
            PartitionFamily::GatherGroup => Some(128 * 1024 * 1024),
        },
        target_partition_count: None,
        ordering: PartitionOrdering::StorageOrder,
        requires_barrier: false,
    }
}

fn halo_spec_for_traits(operator_traits: &[OperatorExecutionTraits]) -> HaloSpec {
    let inline_radius = operator_traits
        .iter()
        .map(|traits| traits.halo_inline)
        .max()
        .unwrap_or(0);
    let xline_radius = operator_traits
        .iter()
        .map(|traits| traits.halo_xline)
        .max()
        .unwrap_or(0);
    HaloSpec {
        inline_radius,
        xline_radius,
    }
}

fn cost_estimate_for_traits(operator_traits: &[OperatorExecutionTraits]) -> CostEstimate {
    let relative_cpu_cost = stage_execution_classification_for_traits(operator_traits)
        .combined_cpu_weight
        .max(1.0);
    let estimated_peak_memory_bytes = operator_traits
        .iter()
        .map(
            |traits| match (traits.memory_cost_class, traits.sample_halo) {
                (MemoryCostClass::Low, SampleHaloRequirement::None) => 64 * 1024 * 1024,
                (MemoryCostClass::Low, _) => 96 * 1024 * 1024,
                (MemoryCostClass::Medium, SampleHaloRequirement::None) => 128 * 1024 * 1024,
                (MemoryCostClass::Medium, _) => 192 * 1024 * 1024,
                (MemoryCostClass::High, SampleHaloRequirement::None) => 256 * 1024 * 1024,
                (MemoryCostClass::High, _) => 384 * 1024 * 1024,
            },
        )
        .max()
        .unwrap_or(64 * 1024 * 1024);
    CostEstimate {
        relative_cpu_cost,
        estimated_peak_memory_bytes,
    }
}

fn stage_execution_classification_for_traits(
    operator_traits: &[OperatorExecutionTraits],
) -> StageExecutionClassification {
    let max_memory_cost_class = operator_traits
        .iter()
        .map(|traits| traits.memory_cost_class)
        .max_by_key(|cost| memory_cost_class_rank(*cost))
        .unwrap_or(MemoryCostClass::Low);
    let max_cpu_cost_class = operator_traits
        .iter()
        .map(|traits| traits.cpu_cost_class)
        .max_by_key(|cost| cpu_cost_class_rank(*cost))
        .unwrap_or(CpuCostClass::Low);
    let max_io_cost_class = operator_traits
        .iter()
        .map(|traits| traits.io_cost_class)
        .max_by_key(|cost| io_cost_class_rank(*cost))
        .unwrap_or(IoCostClass::Low);
    let min_parallel_efficiency_class = operator_traits
        .iter()
        .map(|traits| traits.parallel_efficiency_class)
        .min_by_key(|efficiency| parallel_efficiency_rank(*efficiency))
        .unwrap_or(ParallelEfficiencyClass::High);

    StageExecutionClassification {
        max_memory_cost_class,
        max_cpu_cost_class,
        max_io_cost_class,
        min_parallel_efficiency_class,
        combined_cpu_weight: operator_traits
            .iter()
            .map(|traits| cpu_cost_weight(traits.cpu_cost_class) * dependency_multiplier(traits))
            .sum(),
        combined_io_weight: operator_traits
            .iter()
            .map(|traits| {
                io_cost_weight(traits.io_cost_class)
                    * external_input_io_multiplier(traits)
                    * halo_io_multiplier(traits)
            })
            .sum(),
        uses_external_inputs: operator_traits.iter().any(|traits| {
            matches!(
                traits.spatial_dependency,
                crate::execution::ExecutionSpatialDependency::ExternalVolumePointwise
            )
        }),
        requires_full_volume: operator_traits
            .iter()
            .any(|traits| traits.requires_full_volume),
        has_sample_halo: operator_traits
            .iter()
            .any(|traits| !matches!(traits.sample_halo, SampleHaloRequirement::None)),
        has_spatial_halo: operator_traits
            .iter()
            .any(|traits| traits.halo_inline > 0 || traits.halo_xline > 0),
    }
}

fn execution_plan_summary_for_stages(stages: &[ExecutionStage]) -> ExecutionPlanSummary {
    let compute_stages: Vec<&ExecutionStage> = stages
        .iter()
        .filter(|stage| {
            stage.pipeline_segment.is_some()
                && !matches!(stage.stage_kind, ExecutionStageKind::ReuseArtifact)
        })
        .collect();
    ExecutionPlanSummary {
        compute_stage_count: compute_stages.len(),
        max_memory_cost_class: compute_stages
            .iter()
            .map(|stage| stage.classification.max_memory_cost_class)
            .max_by_key(|cost| memory_cost_class_rank(*cost))
            .unwrap_or(MemoryCostClass::Low),
        max_cpu_cost_class: compute_stages
            .iter()
            .map(|stage| stage.classification.max_cpu_cost_class)
            .max_by_key(|cost| cpu_cost_class_rank(*cost))
            .unwrap_or(CpuCostClass::Low),
        max_io_cost_class: compute_stages
            .iter()
            .map(|stage| stage.classification.max_io_cost_class)
            .max_by_key(|cost| io_cost_class_rank(*cost))
            .unwrap_or(IoCostClass::Low),
        min_parallel_efficiency_class: compute_stages
            .iter()
            .map(|stage| stage.classification.min_parallel_efficiency_class)
            .min_by_key(|efficiency| parallel_efficiency_rank(*efficiency))
            .unwrap_or(ParallelEfficiencyClass::High),
        max_relative_cpu_cost: compute_stages
            .iter()
            .map(|stage| stage.estimated_cost.relative_cpu_cost)
            .fold(0.0_f32, f32::max),
        max_estimated_peak_memory_bytes: compute_stages
            .iter()
            .map(|stage| stage.estimated_cost.estimated_peak_memory_bytes)
            .max()
            .unwrap_or(0),
        combined_cpu_weight: compute_stages
            .iter()
            .map(|stage| stage.classification.combined_cpu_weight)
            .sum(),
        combined_io_weight: compute_stages
            .iter()
            .map(|stage| stage.classification.combined_io_weight)
            .sum(),
        max_expected_partition_count: compute_stages
            .iter()
            .filter_map(|stage| stage.expected_partition_count)
            .max(),
    }
}

fn memory_cost_class_rank(cost: MemoryCostClass) -> usize {
    match cost {
        MemoryCostClass::Low => 0,
        MemoryCostClass::Medium => 1,
        MemoryCostClass::High => 2,
    }
}

fn cpu_cost_class_rank(cost: CpuCostClass) -> usize {
    match cost {
        CpuCostClass::Low => 0,
        CpuCostClass::Medium => 1,
        CpuCostClass::High => 2,
    }
}

fn io_cost_class_rank(cost: IoCostClass) -> usize {
    match cost {
        IoCostClass::Low => 0,
        IoCostClass::Medium => 1,
        IoCostClass::High => 2,
    }
}

fn parallel_efficiency_rank(efficiency: ParallelEfficiencyClass) -> usize {
    match efficiency {
        ParallelEfficiencyClass::Low => 0,
        ParallelEfficiencyClass::Medium => 1,
        ParallelEfficiencyClass::High => 2,
    }
}

fn cpu_cost_weight(cost: CpuCostClass) -> f32 {
    match cost {
        CpuCostClass::Low => 1.0,
        CpuCostClass::Medium => 2.0,
        CpuCostClass::High => 4.0,
    }
}

fn io_cost_weight(cost: IoCostClass) -> f32 {
    match cost {
        IoCostClass::Low => 1.0,
        IoCostClass::Medium => 2.0,
        IoCostClass::High => 4.0,
    }
}

fn dependency_multiplier(traits: &OperatorExecutionTraits) -> f32 {
    match traits.sample_halo {
        SampleHaloRequirement::None => 1.0,
        SampleHaloRequirement::BoundedWindowMs { .. } => 1.25,
        SampleHaloRequirement::WholeTrace => 1.5,
    }
}

fn external_input_io_multiplier(traits: &OperatorExecutionTraits) -> f32 {
    match traits.spatial_dependency {
        crate::execution::ExecutionSpatialDependency::ExternalVolumePointwise => 1.5,
        _ => 1.0,
    }
}

fn halo_io_multiplier(traits: &OperatorExecutionTraits) -> f32 {
    let has_sample_halo = !matches!(traits.sample_halo, SampleHaloRequirement::None);
    let has_spatial_halo = traits.halo_inline > 0 || traits.halo_xline > 0;
    match (has_sample_halo, has_spatial_halo) {
        (false, false) => 1.0,
        (true, true) => 1.5,
        (true, false) | (false, true) => 1.25,
    }
}

fn expected_partition_count_for_stages(stages: &[ExecutionStage]) -> Option<usize> {
    stages
        .iter()
        .map(|stage| stage.expected_partition_count)
        .try_fold(0usize, |total, stage_count| {
            stage_count.map(|count| total + count)
        })
}

fn estimate_partition_count(
    partition_spec: &PartitionSpec,
    source_shape: Option<[usize; 3]>,
    source_chunk_shape: Option<[usize; 3]>,
) -> Option<usize> {
    if let Some(target_partition_count) = partition_spec.target_partition_count {
        return Some(target_partition_count.max(1));
    }

    match partition_spec.family {
        PartitionFamily::FullVolume => Some(1),
        PartitionFamily::TileGroup => estimate_tile_group_partition_count(
            source_shape?,
            source_chunk_shape?,
            partition_spec.target_bytes,
        ),
        PartitionFamily::Section | PartitionFamily::GatherGroup => None,
    }
}

fn estimate_tile_group_partition_count(
    source_shape: [usize; 3],
    source_chunk_shape: [usize; 3],
    target_bytes: Option<u64>,
) -> Option<usize> {
    let chunk_inline = source_chunk_shape[0].max(1).min(source_shape[0].max(1));
    let chunk_xline = source_chunk_shape[1].max(1).min(source_shape[1].max(1));
    let chunk_samples = source_chunk_shape[2].max(1).min(source_shape[2].max(1));
    let tile_count = source_shape[0].div_ceil(chunk_inline) * source_shape[1].div_ceil(chunk_xline);
    let target_bytes = target_bytes.unwrap_or(
        (chunk_inline as u64 * chunk_xline as u64 * chunk_samples as u64 * 4)
            .saturating_add(chunk_inline as u64 * chunk_xline as u64),
    );
    let bytes_per_tile = (chunk_inline as u64 * chunk_xline as u64 * chunk_samples as u64 * 4)
        .saturating_add(chunk_inline as u64 * chunk_xline as u64)
        .max(1);
    let tiles_per_partition = (target_bytes / bytes_per_tile).max(1) as usize;
    Some(tile_count.div_ceil(tiles_per_partition.max(1)))
}

fn reserved_memory_bytes_for_plan(
    plan: &ExecutionPlan,
    available_memory_bytes: Option<u64>,
) -> u64 {
    available_memory_bytes
        .map(|available| {
            let quarter = available / 4;
            let peak_reserve = plan
                .plan_summary
                .max_estimated_peak_memory_bytes
                .saturating_mul(2);
            quarter.max(peak_reserve).max(1024 * 1024 * 1024)
        })
        .unwrap_or(0)
}

fn chunk_planning_mode_for_planning_mode(mode: PlanningMode) -> ChunkPlanningMode {
    match mode {
        PlanningMode::InteractivePreview => ChunkPlanningMode::Conservative,
        PlanningMode::ForegroundMaterialize => ChunkPlanningMode::Auto,
        PlanningMode::BackgroundBatch => ChunkPlanningMode::Throughput,
    }
}

fn stage_memory_profile_for_trace_local_steps(
    steps: &[ophiolite_seismic::TraceLocalProcessingStep],
    operator_traits: &[OperatorExecutionTraits],
    source_chunk_shape: Option<[usize; 3]>,
) -> Option<StageMemoryProfile> {
    let source_chunk_shape = source_chunk_shape?;
    let traces = source_chunk_shape[0]
        .max(1)
        .saturating_mul(source_chunk_shape[1].max(1));
    let samples = source_chunk_shape[2].max(1);
    let primary_tile_bytes = (traces as u64)
        .saturating_mul(samples as u64)
        .saturating_mul(size_of::<f32>() as u64)
        .saturating_add(traces as u64);
    let secondary_input_count = secondary_input_count_for_steps(steps);
    let per_worker_workspace_bytes = trace_local_per_worker_workspace_bytes(steps, samples);
    let transient_bytes = primary_tile_bytes.saturating_mul(secondary_input_count as u64 + 1);

    Some(StageMemoryProfile {
        chunkability: if operator_traits
            .iter()
            .any(|traits| traits.requires_full_volume)
        {
            Chunkability::FullVolumeOnly
        } else {
            Chunkability::TileSpan
        },
        primary_tile_bytes,
        secondary_input_count,
        secondary_tile_bytes_per_input: if secondary_input_count > 0 {
            primary_tile_bytes
        } else {
            0
        },
        output_tile_bytes: primary_tile_bytes,
        per_worker_workspace_bytes,
        shared_stage_bytes: 0,
        reserve_hint_bytes: transient_bytes
            .saturating_add(per_worker_workspace_bytes)
            .max(primary_tile_bytes),
    })
}

fn secondary_input_count_for_steps(steps: &[ophiolite_seismic::TraceLocalProcessingStep]) -> usize {
    let mut unique_paths = BTreeSet::new();
    for step in steps {
        if let TraceLocalProcessingOperation::VolumeArithmetic {
            secondary_store_path,
            ..
        } = &step.operation
        {
            let trimmed = secondary_store_path.trim();
            if !trimmed.is_empty() {
                unique_paths.insert(trimmed.to_string());
            }
        }
    }
    unique_paths.len()
}

fn trace_local_per_worker_workspace_bytes(
    steps: &[ophiolite_seismic::TraceLocalProcessingStep],
    samples: usize,
) -> u64 {
    trace_local_agc_workspace_bytes(samples).saturating_add(
        if trace_local_requires_spectral_workspace(steps) {
            trace_local_spectral_workspace_bytes(samples)
        } else {
            0
        },
    )
}

fn trace_local_agc_workspace_bytes(samples: usize) -> u64 {
    (samples as u64)
        .saturating_mul(size_of::<f32>() as u64)
        .saturating_add((samples as u64 + 1).saturating_mul(size_of::<f64>() as u64))
}

fn trace_local_requires_spectral_workspace(
    steps: &[ophiolite_seismic::TraceLocalProcessingStep],
) -> bool {
    steps.iter().any(|step| {
        matches!(
            step.operation,
            TraceLocalProcessingOperation::PhaseRotation { .. }
                | TraceLocalProcessingOperation::Envelope
                | TraceLocalProcessingOperation::InstantaneousPhase
                | TraceLocalProcessingOperation::InstantaneousFrequency
                | TraceLocalProcessingOperation::Sweetness
                | TraceLocalProcessingOperation::LowpassFilter { .. }
                | TraceLocalProcessingOperation::HighpassFilter { .. }
                | TraceLocalProcessingOperation::BandpassFilter { .. }
        )
    })
}

fn trace_local_spectral_workspace_bytes(samples: usize) -> u64 {
    let real_buffer_bytes = (samples as u64).saturating_mul(size_of::<f32>() as u64);
    let fft_bins = samples / 2 + 1;
    let spectrum_bytes = (fft_bins as u64).saturating_mul(size_of::<f32>() as u64 * 2);
    real_buffer_bytes
        .saturating_mul(5)
        .saturating_add(spectrum_bytes)
}

#[cfg(test)]
mod tests {
    use ophiolite_seismic::{
        TraceLocalProcessingOperation, TraceLocalProcessingPipeline, TraceLocalProcessingStep,
        TraceLocalVolumeArithmeticOperator,
    };

    use super::*;

    #[test]
    fn trace_local_planner_splits_checkpoint_stages() {
        let request = PlanProcessingRequest {
            store_path: "input.tbvol".to_string(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([8, 8, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: TraceLocalProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: Some("demo".to_string()),
                    description: None,
                    steps: vec![
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::AmplitudeScalar {
                                factor: 2.0,
                            },
                            checkpoint: true,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::VolumeArithmetic {
                                operator: TraceLocalVolumeArithmeticOperator::Add,
                                secondary_store_path: "other.tbvol".to_string(),
                            },
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(4),
        };

        let plan = build_execution_plan(&request).expect("plan should build");
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.stages[0].stage_kind, ExecutionStageKind::Checkpoint);
        assert_eq!(
            plan.stages[1].stage_kind,
            ExecutionStageKind::FinalizeOutput
        );
        assert_eq!(
            plan.stages[0]
                .pipeline_segment
                .as_ref()
                .expect("segment")
                .start_step_index,
            0
        );
        assert_eq!(
            plan.stages[1]
                .pipeline_segment
                .as_ref()
                .expect("segment")
                .start_step_index,
            1
        );
        assert_eq!(plan.stages[0].expected_partition_count, Some(1));
        assert_eq!(plan.stages[1].expected_partition_count, Some(1));
        assert_eq!(plan.scheduler_hints.expected_partition_count, Some(2));
        assert_eq!(plan.plan_summary.compute_stage_count, 2);
        assert_eq!(
            plan.plan_summary.max_memory_cost_class,
            MemoryCostClass::Medium
        );
        assert_eq!(plan.plan_summary.max_cpu_cost_class, CpuCostClass::Medium);
        assert_eq!(plan.plan_summary.max_io_cost_class, IoCostClass::Low);
        assert_eq!(
            plan.plan_summary.min_parallel_efficiency_class,
            ParallelEfficiencyClass::High
        );
        assert!(plan.plan_summary.combined_cpu_weight >= 3.0);
        assert!(plan.plan_summary.combined_io_weight >= 2.0);
        assert_eq!(plan.plan_summary.max_expected_partition_count, Some(1));
        assert_eq!(
            plan.stages[0]
                .stage_memory_profile
                .as_ref()
                .expect("trace-local stage should expose a memory profile")
                .secondary_input_count,
            0
        );
        assert_eq!(
            plan.stages[1]
                .stage_memory_profile
                .as_ref()
                .expect("trace-local stage should expose a memory profile")
                .secondary_input_count,
            1
        );
    }

    #[test]
    fn planner_rejects_incompatible_layouts() {
        let request = PlanProcessingRequest {
            store_path: "input.tbgath".to_string(),
            layout: SeismicLayout::PreStack3DOffset,
            source_shape: None,
            source_chunk_shape: None,
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: ophiolite_seismic::PostStackNeighborhoodProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: None,
                    description: None,
                    trace_local_pipeline: None,
                    operations: vec![
                        ophiolite_seismic::PostStackNeighborhoodProcessingOperation::Similarity {
                            window: ophiolite_seismic::PostStackNeighborhoodWindow {
                                gate_ms: 16.0,
                                inline_stepout: 1,
                                xline_stepout: 1,
                            },
                        },
                    ],
                },
            },
            output_store_path: None,
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: None,
        };

        let error = build_execution_plan(&request).expect_err("plan should reject layout");
        assert!(error.contains("does not support layout"));
    }

    #[test]
    fn adaptive_partition_target_reduces_tile_span_for_heavier_workloads() {
        let shape = [651, 951, 462];
        let chunk_shape = [82, 56, 462];
        let agc_plan = build_execution_plan(&PlanProcessingRequest {
            store_path: "input.tbvol".to_string(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some(shape),
            source_chunk_shape: Some(chunk_shape),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: TraceLocalProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: None,
                    description: None,
                    steps: vec![
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                            checkpoint: false,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::AgcRms { window_ms: 250.0 },
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: None,
            planning_mode: PlanningMode::BackgroundBatch,
            max_active_partitions: None,
        })
        .expect("agc plan should build");
        let analytic_plan = build_execution_plan(&PlanProcessingRequest {
            store_path: "input.tbvol".to_string(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some(shape),
            source_chunk_shape: Some(chunk_shape),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: TraceLocalProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: None,
                    description: None,
                    steps: vec![
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                            checkpoint: false,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::Envelope,
                            checkpoint: false,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::InstantaneousPhase,
                            checkpoint: false,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::InstantaneousFrequency,
                            checkpoint: false,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::Sweetness,
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: None,
            planning_mode: PlanningMode::BackgroundBatch,
            max_active_partitions: None,
        })
        .expect("analytic plan should build");

        let agc = recommend_adaptive_partition_target(
            &agc_plan,
            shape,
            chunk_shape,
            16,
            Some(16 * 1024 * 1024 * 1024),
        );
        let analytic = recommend_adaptive_partition_target(
            &analytic_plan,
            shape,
            chunk_shape,
            16,
            Some(16 * 1024 * 1024 * 1024),
        );

        assert!(analytic.target_bytes() <= agc.target_bytes());
        assert!(analytic.tiles_per_partition() <= agc.tiles_per_partition());
        assert!(analytic.estimated_peak_bytes() >= agc.estimated_peak_bytes());
    }

    #[test]
    fn adaptive_partition_target_scales_for_batch_job_concurrency() {
        let shape = [651, 951, 462];
        let chunk_shape = [82, 56, 462];
        let plan = build_execution_plan(&PlanProcessingRequest {
            store_path: "input.tbvol".to_string(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some(shape),
            source_chunk_shape: Some(chunk_shape),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: TraceLocalProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: Some("batch-agc".to_string()),
                    description: None,
                    steps: vec![
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                            checkpoint: false,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::AgcRms { window_ms: 250.0 },
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: None,
            planning_mode: PlanningMode::BackgroundBatch,
            max_active_partitions: None,
        })
        .expect("batch plan should build");

        let single_job = recommend_adaptive_partition_target_for_job_concurrency(
            &plan,
            shape,
            chunk_shape,
            8,
            Some(16 * 1024 * 1024 * 1024),
            1,
        );
        let three_jobs = recommend_adaptive_partition_target_for_job_concurrency(
            &plan,
            shape,
            chunk_shape,
            8,
            Some(16 * 1024 * 1024 * 1024),
            3,
        );

        assert!(three_jobs.target_bytes() <= single_job.target_bytes());
        assert!(
            three_jobs.recommended_partition_count() >= single_job.recommended_partition_count()
        );
        assert_eq!(three_jobs.preferred_partition_count, 15);
    }
}
