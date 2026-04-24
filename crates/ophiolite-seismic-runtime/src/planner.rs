use std::collections::BTreeSet;
use std::fmt::Write;
use std::mem::size_of;

use uuid::Uuid;

use ophiolite_seismic::contracts::{
    OperatorSetIdentity, PipelineArtifactIdentity, PipelineSemanticIdentity,
    PlannerProfileIdentity, ReuseArtifactKind, ReuseBoundaryKind, ReuseMissReason,
    ReuseRequirement, ReuseResolution, SourceSemanticIdentity,
    current_reuse_identity_schema_version,
};
use ophiolite_seismic::{
    GatherProcessingPipeline, PostStackNeighborhoodProcessingPipeline, ProcessingArtifactRole,
    ProcessingPipelineFamily, ProcessingPipelineSpec, SeismicLayout, SubvolumeProcessingPipeline,
    TraceLocalProcessingOperation, TraceLocalProcessingPipeline,
};

use crate::ProcessingCacheFingerprint;
use crate::execution::{
    ArtifactBoundaryReason, ArtifactDerivation, ArtifactDescriptor, ArtifactKey,
    ArtifactLifetimeClass, ArtifactLiveSet, ArtifactLiveSetEntry, CacheMode, ChunkGridSpec,
    ChunkPlanningMode, ChunkShapePolicy, Chunkability, CostEstimate, CpuCostClass, DecisionFactor,
    ExecutionArtifactRole, ExecutionExclusiveScope, ExecutionMemoryBudget,
    ExecutionPipelineSegment, ExecutionPlan, ExecutionPlanSummary, ExecutionPriorityClass,
    ExecutionProgressGranularity, ExecutionQueueClass, ExecutionRetryGranularity,
    ExecutionRuntimeEnvironment, ExecutionSourceDescriptor, ExecutionSpillabilityClass,
    ExecutionStage, ExecutionStageKind, GeometryFingerprints, HaloSpec, IoCostClass, LogicalDomain,
    LoweredStageSchedulerPolicy, MaterializationClass, MemoryCostClass, OperatorExecutionTraits,
    ParallelEfficiencyClass, PartitionFamily, PartitionOrdering, PartitionSpec, PipelineDescriptor,
    PlanDecision, PlanDecisionKind, PlanDecisionSubjectKind, PlannerDiagnostics,
    PlannerPassSnapshot, PlanningMode, PlanningPassId, PreferredPartitioning, ProgressUnits,
    RetryPolicy, ReuseDecision, ReuseDecisionEvidence, ReuseDecisionOutcome, SampleHaloRequirement,
    SchedulerHints, SectionDomain, StageExecutionClassification, StageMemoryProfile,
    StagePlanningDecision, StageResourceEnvelope, StageResourceOwnership,
    TraceLocalChunkPlanRecommendation, ValidationReport, VolumeDomain, execution_stage_label,
    operator_execution_traits_for_pipeline_spec,
};
use crate::identity::{
    CanonicalIdentityStatus, canonical_artifact_identity, combine_canonical_identity_status,
    operator_set_identity_for_pipeline, pipeline_external_identity_status,
    pipeline_semantic_identity, planner_profile_identity_for_pipeline,
    source_artifact_identity_from_source_identity, source_identity_digest,
    source_semantic_identity_or_degraded,
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

#[derive(Debug, Clone, PartialEq)]
struct PlannerPassContext<'a> {
    request: &'a PlanProcessingRequest,
    source_artifact_id: String,
    source_identity: Option<SourceSemanticIdentity>,
    canonical_identity_status: CanonicalIdentityStatus,
    pipeline_descriptor: Option<PipelineDescriptor>,
    pipeline_identity: Option<PipelineSemanticIdentity>,
    operator_set_identity: Option<OperatorSetIdentity>,
    planner_profile_identity: Option<PlannerProfileIdentity>,
    operator_traits: Vec<OperatorExecutionTraits>,
    validation: Option<ValidationReport>,
    normalized_pipeline: Option<NormalizedPipeline>,
    semantic_segments: Vec<SemanticPipelineSegment>,
    partition_outlook: Vec<SemanticSegmentPartitionOutlook>,
    stages: Vec<ExecutionStage>,
    artifacts: Vec<ArtifactDescriptor>,
    expected_partition_count: Option<usize>,
    plan_summary: Option<ExecutionPlanSummary>,
    runtime_environment: Option<ExecutionRuntimeEnvironment>,
    planner_pass_snapshots: Vec<PlannerPassSnapshot>,
    plan_decisions: Vec<PlanDecision>,
}

#[derive(Debug, Clone, PartialEq)]
enum NormalizedPipeline {
    TraceLocal {
        pipeline: TraceLocalProcessingPipeline,
    },
    PostStackNeighborhood {
        pipeline: PostStackNeighborhoodProcessingPipeline,
    },
    Subvolume {
        pipeline: SubvolumeProcessingPipeline,
    },
    Gather {
        pipeline: GatherProcessingPipeline,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticPipelineSegment {
    family: ProcessingPipelineFamily,
    start_step_index: usize,
    end_step_index: usize,
    role: &'static str,
    boundary_reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticSegmentPartitionOutlook {
    segment: SemanticPipelineSegment,
    partition_family: PartitionFamily,
    target_bytes: Option<u64>,
    expected_partition_count: Option<usize>,
}

impl<'a> PlannerPassContext<'a> {
    fn new(request: &'a PlanProcessingRequest) -> Self {
        Self {
            request,
            source_artifact_id: "source".to_string(),
            source_identity: None,
            canonical_identity_status: CanonicalIdentityStatus::Canonical,
            pipeline_descriptor: None,
            pipeline_identity: None,
            operator_set_identity: None,
            planner_profile_identity: None,
            operator_traits: Vec::new(),
            validation: None,
            normalized_pipeline: None,
            semantic_segments: Vec::new(),
            partition_outlook: Vec::new(),
            stages: Vec::new(),
            artifacts: Vec::new(),
            expected_partition_count: None,
            plan_summary: None,
            runtime_environment: None,
            planner_pass_snapshots: Vec::new(),
            plan_decisions: Vec::new(),
        }
    }

    fn push_decision(&mut self, decision: PlanDecision) -> String {
        let decision_id = decision.decision_id.clone();
        self.plan_decisions.push(decision);
        decision_id
    }

    fn record_snapshot(
        &mut self,
        pass_id: PlanningPassId,
        pass_name: &str,
        snapshot_text: String,
        decision_ids: Vec<String>,
    ) {
        self.planner_pass_snapshots.push(PlannerPassSnapshot {
            pass_id,
            pass_name: pass_name.to_string(),
            snapshot_text: Some(snapshot_text),
            decision_ids,
        });
    }
}

pub fn build_execution_plan(request: &PlanProcessingRequest) -> Result<ExecutionPlan, String> {
    let mut context = PlannerPassContext::new(request);
    validate_authored_pipeline_pass(&mut context)?;
    normalize_pipeline_pass(&mut context);
    derive_semantic_segments_pass(&mut context);
    derive_execution_hints_pass(&mut context);
    plan_partitions_pass(&mut context);
    plan_artifacts_and_reuse_pass(&mut context);
    assemble_execution_plan_pass(context)
}

fn validate_authored_pipeline_pass(context: &mut PlannerPassContext<'_>) -> Result<(), String> {
    let pipeline_identity = pipeline_semantic_identity(&context.request.pipeline)?;
    let operator_set_identity = operator_set_identity_for_pipeline(&context.request.pipeline)?;
    let planner_profile_identity =
        planner_profile_identity_for_pipeline(&context.request.pipeline)?;
    let loaded_source_identity = planner_source_identity(context.request);
    let source_identity = loaded_source_identity.identity;
    let canonical_identity_status = combine_canonical_identity_status(
        loaded_source_identity.status,
        pipeline_external_identity_status(&context.request.pipeline),
    );
    let mut pipeline_descriptor = PipelineDescriptor::from_pipeline_spec(&context.request.pipeline);
    pipeline_descriptor.content_digest = pipeline_identity.content_digest.clone();
    pipeline_descriptor.operator_set_version = operator_set_identity.version.clone();
    pipeline_descriptor.planner_profile_version = planner_profile_identity.version.clone();
    let operator_traits = operator_execution_traits_for_pipeline_spec(&context.request.pipeline);
    let validation = validation_report_for_layout(context.request.layout, &operator_traits);
    let mut snapshot = String::new();
    let _ = write!(
        snapshot,
        "family={} operators={} valid={} warnings={} blockers={} pipeline_digest={}",
        pipeline_family_name(pipeline_descriptor.family),
        operator_traits.len(),
        validation.plan_valid,
        validation.warnings.len(),
        validation.blockers.len(),
        pipeline_identity.content_digest
    );
    if !validation.blockers.is_empty() {
        let _ = write!(
            snapshot,
            " blocker_text={}",
            validation.blockers.join(" | ")
        );
    }
    context.source_identity = Some(source_identity);
    context.canonical_identity_status = canonical_identity_status;
    context.pipeline_descriptor = Some(pipeline_descriptor);
    context.pipeline_identity = Some(pipeline_identity);
    context.operator_set_identity = Some(operator_set_identity);
    context.planner_profile_identity = Some(planner_profile_identity);
    context.operator_traits = operator_traits;
    context.validation = Some(validation.clone());
    let decision_id = context.push_decision(PlanDecision {
        decision_id: "pass-validate-authored-pipeline".to_string(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "validate_authored_pipeline".to_string(),
        decision_kind: PlanDecisionKind::Lowering,
        reason_code: if validation.plan_valid {
            "validation_ok".to_string()
        } else {
            "validation_failed".to_string()
        },
        human_summary: snapshot.clone(),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    context.record_snapshot(
        PlanningPassId::ValidateAuthoredPipeline,
        "validate_authored_pipeline",
        snapshot,
        vec![decision_id],
    );
    if !validation.plan_valid {
        return Err(validation.blockers.join("; "));
    }
    Ok(())
}

fn planner_source_identity(
    request: &PlanProcessingRequest,
) -> crate::identity::LoadedSourceSemanticIdentity {
    source_semantic_identity_or_degraded(
        &request.store_path,
        request.layout,
        request.source_shape,
        request.source_chunk_shape,
    )
}

fn normalize_pipeline_pass(context: &mut PlannerPassContext<'_>) {
    let (normalized_pipeline, snapshot) = match &context.request.pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => (
            NormalizedPipeline::TraceLocal {
                pipeline: pipeline.clone(),
            },
            format!(
                "family=trace_local steps={} checkpoints={}",
                pipeline.steps.len(),
                pipeline.checkpoint_indexes().len()
            ),
        ),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => (
            NormalizedPipeline::PostStackNeighborhood {
                pipeline: pipeline.clone(),
            },
            format!(
                "family=post_stack_neighborhood trace_local_prefix_steps={} family_operations={}",
                pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(|pipeline| pipeline.steps.len())
                    .unwrap_or(0),
                pipeline.operations.len()
            ),
        ),
        ProcessingPipelineSpec::Subvolume { pipeline } => (
            NormalizedPipeline::Subvolume {
                pipeline: pipeline.clone(),
            },
            format!(
                "family=subvolume trace_local_prefix_steps={} crop_operator=subvolume_crop",
                pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(|pipeline| pipeline.steps.len())
                    .unwrap_or(0)
            ),
        ),
        ProcessingPipelineSpec::Gather { pipeline } => (
            NormalizedPipeline::Gather {
                pipeline: pipeline.clone(),
            },
            format!(
                "family=gather trace_local_prefix_steps={} family_operations={}",
                pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(|pipeline| pipeline.steps.len())
                    .unwrap_or(0),
                pipeline.operations.len()
            ),
        ),
    };
    context.normalized_pipeline = Some(normalized_pipeline);
    let decision_id = context.push_decision(PlanDecision {
        decision_id: "pass-normalize-pipeline".to_string(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "normalize_pipeline".to_string(),
        decision_kind: PlanDecisionKind::Lowering,
        reason_code: "normalized_pipeline".to_string(),
        human_summary: snapshot.clone(),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    context.record_snapshot(
        PlanningPassId::NormalizePipeline,
        "normalize_pipeline",
        snapshot,
        vec![decision_id],
    );
}

fn derive_semantic_segments_pass(context: &mut PlannerPassContext<'_>) {
    let segments = semantic_segments_for_pipeline(
        context
            .normalized_pipeline
            .as_ref()
            .expect("normalize pass should run first"),
    );
    let mut snapshot = String::new();
    let _ = writeln!(snapshot, "segments={}", segments.len());
    for segment in &segments {
        let _ = writeln!(
            snapshot,
            "{} {}..{} role={} boundary={}",
            pipeline_family_name(segment.family),
            segment.start_step_index,
            segment.end_step_index,
            segment.role,
            segment.boundary_reason
        );
    }
    context.semantic_segments = segments;
    let snapshot = snapshot.trim_end().to_string();
    let decision_id = context.push_decision(PlanDecision {
        decision_id: "pass-derive-semantic-segments".to_string(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "derive_semantic_segments".to_string(),
        decision_kind: PlanDecisionKind::Lowering,
        reason_code: "derived_semantic_segments".to_string(),
        human_summary: snapshot.clone(),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    context.record_snapshot(
        PlanningPassId::DeriveSemanticSegments,
        "derive_semantic_segments",
        snapshot,
        vec![decision_id],
    );
}

fn derive_execution_hints_pass(context: &mut PlannerPassContext<'_>) {
    let classification = stage_execution_classification_for_traits(&context.operator_traits);
    let partition_family = partition_spec_for_traits(&context.operator_traits).family;
    let snapshot = format!(
        "partition_family={} max_memory={} max_cpu={} max_io={} min_parallel={} uses_external_inputs={} requires_full_volume={}",
        partition_family_name(partition_family),
        memory_cost_class_name(classification.max_memory_cost_class),
        cpu_cost_class_name(classification.max_cpu_cost_class),
        io_cost_class_name(classification.max_io_cost_class),
        parallel_efficiency_name(classification.min_parallel_efficiency_class),
        classification.uses_external_inputs,
        classification.requires_full_volume
    );
    let decision_id = context.push_decision(PlanDecision {
        decision_id: "pass-derive-execution-hints".to_string(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "derive_execution_hints".to_string(),
        decision_kind: PlanDecisionKind::Scheduling,
        reason_code: "derived_execution_hints".to_string(),
        human_summary: snapshot.clone(),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    context.record_snapshot(
        PlanningPassId::DeriveExecutionHints,
        "derive_execution_hints",
        snapshot,
        vec![decision_id],
    );
}

fn plan_partitions_pass(context: &mut PlannerPassContext<'_>) {
    let outlook = semantic_segment_partition_outlook(
        &context.semantic_segments,
        &context.operator_traits,
        context.request.source_shape,
        context.request.source_chunk_shape,
    );
    let mut snapshot = String::new();
    let _ = writeln!(snapshot, "segments={}", outlook.len());
    for item in &outlook {
        let _ = writeln!(
            snapshot,
            "{} {}..{} partition={} target_bytes={} expected_partitions={}",
            pipeline_family_name(item.segment.family),
            item.segment.start_step_index,
            item.segment.end_step_index,
            partition_family_name(item.partition_family),
            item.target_bytes
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            item.expected_partition_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    context.partition_outlook = outlook;
    let snapshot = snapshot.trim_end().to_string();
    let decision_id = context.push_decision(PlanDecision {
        decision_id: "pass-plan-partitions".to_string(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "plan_partitions".to_string(),
        decision_kind: PlanDecisionKind::Scheduling,
        reason_code: "planned_partitions".to_string(),
        human_summary: snapshot.clone(),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    context.record_snapshot(
        PlanningPassId::PlanPartitions,
        "plan_partitions",
        snapshot,
        vec![decision_id],
    );
}

fn plan_artifacts_and_reuse_pass(context: &mut PlannerPassContext<'_>) {
    let source_identity = context
        .source_identity
        .as_ref()
        .expect("validate pass should populate source identity");
    let source_geometry = geometry_fingerprints_for_request(context.request, source_identity);
    let source_domain = source_logical_domain(context.request);
    let source_grid = chunk_grid_spec_for_request(context.request);
    let source_key = if matches!(
        context.canonical_identity_status,
        CanonicalIdentityStatus::Canonical
    ) {
        Some(artifact_key_from_parts(
            source_lineage_digest(source_identity),
            source_geometry.clone(),
            source_domain.clone(),
            source_grid.clone(),
            MaterializationClass::ReusedArtifact,
        ))
    } else {
        None
    };
    context.artifacts = vec![ArtifactDescriptor {
        artifact_id: context.source_artifact_id.clone(),
        role: ExecutionArtifactRole::Input,
        store_path: Some(context.request.store_path.clone()),
        cache_key: source_key
            .as_ref()
            .map(|artifact_key| artifact_key.cache_key.clone()),
        artifact_key: source_key,
        logical_domain: Some(source_domain),
        chunk_grid_spec: Some(source_grid),
        geometry_fingerprints: Some(source_geometry),
        materialization_class: Some(MaterializationClass::ReusedArtifact),
        boundary_reason: Some(ArtifactBoundaryReason::SourceInput),
        lifetime_class: Some(ArtifactLifetimeClass::Source),
        reuse_requirement: None,
        reuse_resolution: None,
        reuse_decision_id: None,
        artifact_derivation_decision_id: None,
    }];

    let pipeline_descriptor = context
        .pipeline_descriptor
        .as_ref()
        .expect("validate pass should populate pipeline descriptor");
    let (mut stages, mut derived_artifacts) = match &context.request.pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => build_trace_local_stages(
            pipeline,
            &context.source_artifact_id,
            context.request.output_store_path.as_deref(),
            &context.operator_traits,
            context.request.source_shape,
            context.request.source_chunk_shape,
            context.request.planning_mode,
        ),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => build_prefixed_family_plan(
            pipeline_descriptor,
            pipeline.trace_local_pipeline.as_ref(),
            &context.source_artifact_id,
            context.request.output_store_path.as_deref(),
            &context.operator_traits,
            pipeline
                .name
                .clone()
                .unwrap_or_else(|| "post_stack_neighborhood".to_string()),
            context.request.source_shape,
            context.request.source_chunk_shape,
            context.request.planning_mode,
        ),
        ProcessingPipelineSpec::Subvolume { pipeline } => build_subvolume_plan(
            pipeline_descriptor,
            pipeline,
            &context.source_artifact_id,
            context.request.output_store_path.as_deref(),
            &context.operator_traits,
            pipeline
                .name
                .clone()
                .unwrap_or_else(|| "subvolume".to_string()),
            context.request.source_shape,
            context.request.source_chunk_shape,
            context.request.planning_mode,
        ),
        ProcessingPipelineSpec::Gather { pipeline } => build_prefixed_family_plan(
            pipeline_descriptor,
            pipeline.trace_local_pipeline.as_ref(),
            &context.source_artifact_id,
            context.request.output_store_path.as_deref(),
            &context.operator_traits,
            pipeline
                .name
                .clone()
                .unwrap_or_else(|| "gather".to_string()),
            context.request.source_shape,
            context.request.source_chunk_shape,
            context.request.planning_mode,
        ),
    };
    populate_reuse_candidates(
        context.request,
        source_identity,
        context.canonical_identity_status,
        &mut stages,
        &mut derived_artifacts,
        &mut context.plan_decisions,
    );
    annotate_materialization_and_identity(
        context.request,
        source_identity,
        context.canonical_identity_status,
        pipeline_descriptor,
        &context.operator_traits,
        &mut stages,
        &mut derived_artifacts,
        &mut context.plan_decisions,
    );
    annotate_live_sets(&mut stages, &derived_artifacts);
    context.expected_partition_count = expected_partition_count_for_stages(&stages);
    context.plan_summary = Some(execution_plan_summary_for_stages(&stages));
    context.runtime_environment = Some(runtime_environment_for_stages(
        context.request,
        &stages,
        context
            .plan_summary
            .as_ref()
            .expect("plan summary should exist before runtime lowering"),
    ));
    annotate_stage_lowering(
        context.request,
        context
            .runtime_environment
            .as_ref()
            .expect("runtime environment should exist before stage lowering"),
        &mut stages,
        &mut context.plan_decisions,
    );
    context.artifacts.append(&mut derived_artifacts);
    context.stages.append(&mut stages);
    let reuse_candidate_count = context
        .artifacts
        .iter()
        .filter(|artifact| artifact.reuse_requirement.is_some())
        .count();
    let snapshot = format!(
        "stages={} artifacts={} reuse_candidates={} expected_partitions={}",
        context.stages.len(),
        context.artifacts.len(),
        reuse_candidate_count,
        context
            .expected_partition_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    let decision_id = context.push_decision(PlanDecision {
        decision_id: "pass-plan-artifacts-and-reuse".to_string(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "plan_artifacts_and_reuse".to_string(),
        decision_kind: PlanDecisionKind::Reuse,
        reason_code: "planned_artifacts_and_reuse".to_string(),
        human_summary: snapshot.clone(),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    context.record_snapshot(
        PlanningPassId::PlanArtifactsAndReuse,
        "plan_artifacts_and_reuse",
        snapshot,
        vec![decision_id],
    );
}

fn populate_reuse_candidates(
    request: &PlanProcessingRequest,
    source_identity: &SourceSemanticIdentity,
    canonical_identity_status: CanonicalIdentityStatus,
    stages: &mut [ExecutionStage],
    artifacts: &mut [ArtifactDescriptor],
    plan_decisions: &mut Vec<PlanDecision>,
) {
    if !matches!(
        canonical_identity_status,
        CanonicalIdentityStatus::Canonical
    ) {
        return;
    }
    let pipeline_descriptor = PipelineDescriptor::from_pipeline_spec(&request.pipeline);
    for artifact in artifacts.iter_mut() {
        let Some(stage) = stages
            .iter_mut()
            .find(|stage| stage.output_artifact_id == artifact.artifact_id)
        else {
            continue;
        };
        let Some(requirement) = build_reuse_requirement(
            request,
            source_identity,
            &pipeline_descriptor,
            artifact.role,
            stage,
        ) else {
            continue;
        };
        let resolution = planning_time_reuse_resolution(&requirement);
        artifact.cache_key = Some(requirement.reuse_key.clone());
        artifact.reuse_requirement = Some(requirement.clone());
        artifact.reuse_resolution = Some(resolution.clone());
        let reuse_decision_id = format!("reuse-{}", artifact.artifact_id);
        let outcome = if resolution.reused {
            ReuseDecisionOutcome::Reused
        } else if matches!(
            resolution.miss_reason,
            Some(ReuseMissReason::UnresolvedAtPlanningTime)
        ) {
            ReuseDecisionOutcome::Unresolved
        } else {
            ReuseDecisionOutcome::Miss
        };
        plan_decisions.push(PlanDecision {
            decision_id: reuse_decision_id.clone(),
            subject_kind: PlanDecisionSubjectKind::Artifact,
            subject_id: artifact.artifact_id.clone(),
            decision_kind: PlanDecisionKind::Reuse,
            reason_code: format!(
                "{:?}",
                resolution
                    .miss_reason
                    .unwrap_or(ReuseMissReason::NoReusableArtifactResolved)
            )
            .to_ascii_lowercase(),
            human_summary: format!(
                "reuse {} for {:?} {:?}",
                if resolution.reused {
                    "selected"
                } else {
                    "miss"
                },
                requirement.artifact_kind,
                requirement.boundary_kind
            ),
            stage_planning: None,
            reuse_decision: Some(ReuseDecision {
                stage_id: Some(stage.stage_id.clone()),
                artifact_id: artifact.artifact_id.clone(),
                cache_mode: stage.cache_mode,
                artifact_kind: requirement.artifact_kind,
                boundary_kind: requirement.boundary_kind,
                candidate_count: usize::from(resolution.reused),
                selected_candidate_reuse_key: if resolution.reused {
                    Some(requirement.reuse_key.clone())
                } else {
                    None
                },
                selected_candidate_artifact_key: None,
                selected_candidate_store_path: resolution.artifact_store_path.clone(),
                outcome,
                miss_reason: resolution.miss_reason,
                evidence: vec![ReuseDecisionEvidence {
                    label: if resolution.reused {
                        "planning lookup resolved reusable artifact".to_string()
                    } else {
                        "planning lookup kept runtime or fresh-compute fallback".to_string()
                    },
                    matched: resolution.reused,
                    artifact_key: None,
                    artifact_store_path: resolution.artifact_store_path.clone(),
                    miss_reason: resolution.miss_reason,
                }],
            }),
            artifact_derivation: None,
        });
        stage.reuse_requirement = Some(requirement);
        stage.reuse_resolution = Some(resolution);
        stage.reuse_decision_id = Some(reuse_decision_id.clone());
        artifact.reuse_decision_id = Some(reuse_decision_id);
    }
}

fn build_reuse_requirement(
    request: &PlanProcessingRequest,
    source_identity: &SourceSemanticIdentity,
    _pipeline_descriptor: &PipelineDescriptor,
    artifact_role: ExecutionArtifactRole,
    stage: &ExecutionStage,
) -> Option<ReuseRequirement> {
    let Some(stage_segment) = stage.pipeline_segment.as_ref() else {
        return None;
    };
    let stage_pipeline = reuse_identity_pipeline_spec_for_stage(request, stage_segment.family);
    let pipeline_identity = pipeline_semantic_identity(&stage_pipeline).ok()?;
    let operator_set_identity = operator_set_identity_for_pipeline(&stage_pipeline).ok()?;
    let planner_profile_identity = planner_profile_identity_for_pipeline(&stage_pipeline).ok()?;
    let stage_operator_ids = operator_execution_traits_for_pipeline_spec(&stage_pipeline)
        .into_iter()
        .map(|traits| traits.operator_id)
        .collect::<Vec<_>>();
    let is_trace_local_prefix_boundary = is_trace_local_prefix_boundary(request, stage_segment);
    let (artifact_kind, boundary_kind, end_step_index) = match artifact_role {
        ExecutionArtifactRole::FinalOutput => (
            ReuseArtifactKind::ExactVisibleFinal,
            ReuseBoundaryKind::ExactOutput,
            stage_operator_ids.len().saturating_sub(1),
        ),
        ExecutionArtifactRole::Checkpoint => (
            ReuseArtifactKind::VisibleCheckpoint,
            if is_trace_local_prefix_boundary {
                ReuseBoundaryKind::TraceLocalPrefix
            } else {
                ReuseBoundaryKind::AuthoredCheckpoint
            },
            stage_segment.end_step_index,
        ),
        ExecutionArtifactRole::Input | ExecutionArtifactRole::CachedReuse => return None,
    };
    let operator_ids = if stage_operator_ids.is_empty() {
        Vec::new()
    } else {
        let bounded_end: usize = end_step_index.min(stage_operator_ids.len() - 1);
        stage_operator_ids[..=bounded_end].to_vec()
    };
    let artifact = PipelineArtifactIdentity {
        schema_version: current_reuse_identity_schema_version(),
        pipeline_family: pipeline_identity.family,
        pipeline_schema_version: pipeline_identity.pipeline_schema_version,
        pipeline_revision: pipeline_identity.revision,
        pipeline_content_digest: pipeline_identity.content_digest.clone(),
        operator_set_version: operator_set_identity.version.clone(),
        effective_operator_digest: operator_set_identity.effective_operator_digest.clone(),
        planner_profile_version: planner_profile_identity.version.clone(),
        effective_structural_digest: planner_profile_identity.effective_structural_digest.clone(),
        artifact_kind,
        boundary_kind,
        start_step_index: 0,
        end_step_index,
        operator_ids,
    };
    let source = source_artifact_identity_from_source_identity(source_identity);
    let mut requirement = ReuseRequirement {
        reuse_key: String::new(),
        artifact_kind,
        boundary_kind,
        source,
        artifact,
    };
    requirement.reuse_key = reuse_requirement_key(&requirement);
    Some(requirement)
}

fn planning_time_reuse_resolution(requirement: &ReuseRequirement) -> ReuseResolution {
    ReuseResolution {
        reuse_key: requirement.reuse_key.clone(),
        artifact_kind: requirement.artifact_kind,
        boundary_kind: requirement.boundary_kind,
        reused: false,
        miss_reason: Some(ReuseMissReason::UnresolvedAtPlanningTime),
        artifact_store_path: None,
    }
}

fn reuse_requirement_key(requirement: &ReuseRequirement) -> String {
    let payload =
        serde_json::to_vec(requirement).expect("reuse requirement should serialize to JSON");
    blake3::hash(&payload).to_hex().to_string()
}

fn reuse_identity_pipeline_spec_for_stage(
    request: &PlanProcessingRequest,
    stage_family: ProcessingPipelineFamily,
) -> ProcessingPipelineSpec {
    if matches!(stage_family, ProcessingPipelineFamily::TraceLocal) {
        match &request.pipeline {
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
                if let Some(prefix) = pipeline.trace_local_pipeline.as_ref() {
                    return ProcessingPipelineSpec::TraceLocal {
                        pipeline: prefix.clone(),
                    };
                }
            }
            ProcessingPipelineSpec::Subvolume { pipeline } => {
                if let Some(prefix) = pipeline.trace_local_pipeline.as_ref() {
                    return ProcessingPipelineSpec::TraceLocal {
                        pipeline: prefix.clone(),
                    };
                }
            }
            ProcessingPipelineSpec::Gather { pipeline } => {
                if let Some(prefix) = pipeline.trace_local_pipeline.as_ref() {
                    return ProcessingPipelineSpec::TraceLocal {
                        pipeline: prefix.clone(),
                    };
                }
            }
            ProcessingPipelineSpec::TraceLocal { .. } => {}
        }
    }
    request.pipeline.clone()
}

fn canonical_identity_pipeline_spec_for_stage(
    request: &PlanProcessingRequest,
    stage: &ExecutionStage,
) -> ProcessingPipelineSpec {
    let Some(segment) = stage.pipeline_segment.as_ref() else {
        return request.pipeline.clone();
    };
    if matches!(stage.stage_kind, ExecutionStageKind::Checkpoint)
        && matches!(segment.family, ProcessingPipelineFamily::TraceLocal)
    {
        return trace_local_prefix_pipeline_spec(request, segment.end_step_index);
    }
    reuse_identity_pipeline_spec_for_stage(request, segment.family)
}

fn trace_local_prefix_pipeline_spec(
    request: &PlanProcessingRequest,
    end_step_index: usize,
) -> ProcessingPipelineSpec {
    let pipeline = match &request.pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => Some(pipeline),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
            pipeline.trace_local_pipeline.as_ref()
        }
        ProcessingPipelineSpec::Subvolume { pipeline } => pipeline.trace_local_pipeline.as_ref(),
        ProcessingPipelineSpec::Gather { pipeline } => pipeline.trace_local_pipeline.as_ref(),
    };
    match pipeline {
        Some(pipeline) => ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                steps: pipeline.steps
                    [..=end_step_index.min(pipeline.steps.len().saturating_sub(1))]
                    .to_vec(),
                ..pipeline.clone()
            },
        },
        None => request.pipeline.clone(),
    }
}

fn is_trace_local_prefix_boundary(
    request: &PlanProcessingRequest,
    stage_segment: &ExecutionPipelineSegment,
) -> bool {
    if !matches!(stage_segment.family, ProcessingPipelineFamily::TraceLocal) {
        return false;
    }
    trace_local_prefix_end_step_index(&request.pipeline)
        .map(|end_step_index| end_step_index == stage_segment.end_step_index)
        .unwrap_or(false)
}

fn trace_local_prefix_end_step_index(pipeline: &ProcessingPipelineSpec) -> Option<usize> {
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { .. } => None,
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => pipeline
            .trace_local_pipeline
            .as_ref()
            .and_then(|prefix| prefix.operation_count().checked_sub(1)),
        ProcessingPipelineSpec::Subvolume { pipeline } => pipeline
            .trace_local_pipeline
            .as_ref()
            .and_then(|prefix| prefix.operation_count().checked_sub(1)),
        ProcessingPipelineSpec::Gather { pipeline } => pipeline
            .trace_local_pipeline
            .as_ref()
            .and_then(|prefix| prefix.operation_count().checked_sub(1)),
    }
}

fn annotate_materialization_and_identity(
    request: &PlanProcessingRequest,
    source_identity: &SourceSemanticIdentity,
    canonical_identity_status: CanonicalIdentityStatus,
    pipeline_descriptor: &PipelineDescriptor,
    operator_traits: &[OperatorExecutionTraits],
    stages: &mut [ExecutionStage],
    artifacts: &mut [ArtifactDescriptor],
    plan_decisions: &mut Vec<PlanDecision>,
) {
    for stage in stages.iter_mut() {
        let boundary_reason = boundary_reason_for_stage(stage, request, pipeline_descriptor.family);
        let materialization_class = materialization_class_for_stage(stage);
        let logical_domain = logical_domain_for_stage(stage, request);
        let artifact_key = canonical_artifact_key_for_stage(
            request,
            source_identity,
            canonical_identity_status,
            pipeline_descriptor,
            stage,
            boundary_reason,
            materialization_class,
        );
        stage.boundary_reason = Some(boundary_reason);
        stage.materialization_class = Some(materialization_class);
        stage.reuse_class = stage
            .pipeline_segment
            .as_ref()
            .and_then(|segment| operator_traits.get(segment.end_step_index))
            .map(|traits| traits.reuse_class);
        stage.output_artifact_key = artifact_key
            .as_ref()
            .map(|identity| identity.artifact_key.clone());
        if let Some(artifact) = artifacts
            .iter_mut()
            .find(|artifact| artifact.artifact_id == stage.output_artifact_id)
        {
            let derivation_decision_id = format!("artifact-derive-{}", artifact.artifact_id);
            artifact.cache_key = artifact_key
                .as_ref()
                .map(|identity| identity.artifact_key.cache_key.clone());
            artifact.artifact_key = artifact_key
                .as_ref()
                .map(|identity| identity.artifact_key.clone());
            artifact.logical_domain = artifact_key
                .as_ref()
                .map(|identity| identity.logical_domain.clone())
                .or_else(|| Some(logical_domain.clone()));
            artifact.chunk_grid_spec = artifact_key
                .as_ref()
                .map(|identity| identity.chunk_grid_spec.clone())
                .or_else(|| Some(chunk_grid_spec_for_request(request)));
            artifact.geometry_fingerprints = artifact_key
                .as_ref()
                .map(|identity| identity.geometry_fingerprints.clone())
                .or_else(|| Some(geometry_fingerprints_for_request(request, source_identity)));
            artifact.materialization_class = Some(materialization_class);
            artifact.boundary_reason = Some(boundary_reason);
            artifact.lifetime_class = Some(lifetime_class_for_role(artifact.role));
            artifact.artifact_derivation_decision_id = Some(derivation_decision_id.clone());
            plan_decisions.push(PlanDecision {
                decision_id: derivation_decision_id,
                subject_kind: PlanDecisionSubjectKind::Artifact,
                subject_id: artifact.artifact_id.clone(),
                decision_kind: PlanDecisionKind::ArtifactDerivation,
                reason_code: format!("{boundary_reason:?}").to_ascii_lowercase(),
                human_summary: format!(
                    "artifact {} derives {:?} output from {} inputs{}",
                    artifact.artifact_id,
                    materialization_class,
                    stage.input_artifact_ids.len(),
                    if artifact.artifact_key.is_none() {
                        " (canonical artifact identity disabled)"
                    } else {
                        ""
                    }
                ),
                stage_planning: None,
                reuse_decision: None,
                artifact_derivation: artifact.artifact_key.clone().map(|artifact_key| {
                    ArtifactDerivation {
                        artifact_id: artifact.artifact_id.clone(),
                        artifact_key,
                        input_artifact_ids: stage.input_artifact_ids.clone(),
                        logical_domain: artifact
                            .logical_domain
                            .clone()
                            .expect("logical domain should exist before derivation decision"),
                        chunk_grid_spec: artifact
                            .chunk_grid_spec
                            .clone()
                            .expect("chunk grid should exist before derivation decision"),
                        geometry_fingerprints: artifact
                            .geometry_fingerprints
                            .clone()
                            .expect("geometry should exist before derivation decision"),
                        materialization_class,
                        boundary_reason,
                    }
                }),
            });
        }
    }
}

fn annotate_live_sets(stages: &mut [ExecutionStage], artifacts: &[ArtifactDescriptor]) {
    let mut published_artifacts = Vec::<String>::new();
    for stage in stages.iter_mut() {
        let mut resident_artifacts = Vec::new();
        for artifact_id in stage
            .input_artifact_ids
            .iter()
            .cloned()
            .chain(std::iter::once(stage.output_artifact_id.clone()))
        {
            let Some(artifact) = artifacts
                .iter()
                .find(|candidate| candidate.artifact_id == artifact_id)
            else {
                continue;
            };
            let estimated_resident_bytes = stage
                .stage_memory_profile
                .as_ref()
                .map(estimated_live_set_bytes_for_profile)
                .unwrap_or(0);
            resident_artifacts.push(ArtifactLiveSetEntry {
                artifact_id: artifact.artifact_id.clone(),
                artifact_key: artifact.artifact_key.clone(),
                estimated_resident_bytes,
            });
        }
        for artifact_id in &published_artifacts {
            if resident_artifacts
                .iter()
                .any(|artifact| &artifact.artifact_id == artifact_id)
            {
                continue;
            }
            resident_artifacts.push(ArtifactLiveSetEntry {
                artifact_id: artifact_id.clone(),
                artifact_key: artifacts
                    .iter()
                    .find(|artifact| &artifact.artifact_id == artifact_id)
                    .and_then(|artifact| artifact.artifact_key.clone()),
                estimated_resident_bytes: 0,
            });
        }
        let estimated_resident_bytes = resident_artifacts
            .iter()
            .map(|artifact| artifact.estimated_resident_bytes)
            .sum();
        stage.live_set = Some(ArtifactLiveSet {
            resident_artifacts,
            estimated_resident_bytes,
        });
        published_artifacts.push(stage.output_artifact_id.clone());
    }
}

fn source_logical_domain(request: &PlanProcessingRequest) -> LogicalDomain {
    LogicalDomain::Volume {
        volume: VolumeDomain {
            shape: request.source_shape.unwrap_or([0, 0, 0]),
        },
    }
}

fn logical_domain_for_stage(
    stage: &ExecutionStage,
    request: &PlanProcessingRequest,
) -> LogicalDomain {
    if matches!(
        stage.stage_kind,
        ExecutionStageKind::Checkpoint
            | ExecutionStageKind::FinalizeOutput
            | ExecutionStageKind::ReuseArtifact
    ) {
        return LogicalDomain::Volume {
            volume: VolumeDomain {
                shape: request.source_shape.unwrap_or([0, 0, 0]),
            },
        };
    }
    match stage.partition_spec.family {
        PartitionFamily::Section => LogicalDomain::Section {
            section: SectionDomain {
                axis: ophiolite_seismic::SectionAxis::Inline,
                section_index: 0,
            },
        },
        _ => LogicalDomain::Volume {
            volume: VolumeDomain {
                shape: request.source_shape.unwrap_or([0, 0, 0]),
            },
        },
    }
}

fn chunk_grid_spec_for_request(request: &PlanProcessingRequest) -> ChunkGridSpec {
    ChunkGridSpec::Regular {
        origin: [0, 0, 0],
        chunk_shape: request.source_chunk_shape.unwrap_or([0, 0, 0]),
    }
}

fn geometry_fingerprints_for_request(
    request: &PlanProcessingRequest,
    source_identity: &SourceSemanticIdentity,
) -> GeometryFingerprints {
    let source_digest =
        source_identity_digest(source_identity).expect("source identity should serialize");
    let survey_geometry_fingerprint = ProcessingCacheFingerprint::fingerprint_json(&(
        &source_digest,
        request.layout,
        request.source_shape.unwrap_or([0, 0, 0]),
    ))
    .expect("planner geometry seed should serialize");
    let storage_grid_fingerprint = ProcessingCacheFingerprint::fingerprint_json(&(
        &source_digest,
        request.layout,
        request.source_shape.unwrap_or([0, 0, 0]),
        request.source_chunk_shape.unwrap_or([0, 0, 0]),
    ))
    .expect("planner storage geometry seed should serialize");
    let section_projection_fingerprint = ProcessingCacheFingerprint::fingerprint_json(&(
        request.layout,
        request.source_shape,
        request.source_shape.unwrap_or([0, 0, 0]),
    ))
    .expect("planner section projection seed should serialize");
    let artifact_lineage_fingerprint = ProcessingCacheFingerprint::fingerprint_json(&source_digest)
        .expect("planner lineage geometry seed should serialize");
    GeometryFingerprints {
        survey_geometry_fingerprint,
        storage_grid_fingerprint,
        section_projection_fingerprint,
        artifact_lineage_fingerprint,
    }
}

fn canonical_artifact_key_for_stage(
    request: &PlanProcessingRequest,
    source_identity: &SourceSemanticIdentity,
    canonical_identity_status: CanonicalIdentityStatus,
    _pipeline_descriptor: &PipelineDescriptor,
    stage: &ExecutionStage,
    boundary_reason: ArtifactBoundaryReason,
    materialization_class: MaterializationClass,
) -> Option<crate::identity::CanonicalArtifactIdentity> {
    let pipeline = canonical_identity_pipeline_spec_for_stage(request, stage);
    let pipeline_identity = pipeline_semantic_identity(&pipeline).ok()?;
    let operator_set_identity = operator_set_identity_for_pipeline(&pipeline).ok()?;
    let planner_profile_identity = planner_profile_identity_for_pipeline(&pipeline).ok()?;
    let artifact_role = match stage.stage_kind {
        ExecutionStageKind::Checkpoint => ProcessingArtifactRole::Checkpoint,
        ExecutionStageKind::FinalizeOutput => ProcessingArtifactRole::FinalOutput,
        ExecutionStageKind::ReuseArtifact | ExecutionStageKind::Compute => return None,
    };
    canonical_artifact_identity(
        source_identity,
        canonical_identity_status,
        &pipeline_identity,
        &operator_set_identity,
        &planner_profile_identity,
        request.layout,
        request.source_shape.unwrap_or([0, 0, 0]),
        request.source_chunk_shape.unwrap_or([0, 0, 0]),
        artifact_role,
        boundary_reason,
        materialization_class,
        logical_domain_for_stage(stage, request),
    )
    .ok()
    .flatten()
}

fn source_lineage_digest(source_identity: &SourceSemanticIdentity) -> String {
    source_identity_digest(source_identity).expect("source lineage should serialize")
}

fn artifact_key_from_parts(
    lineage_digest: String,
    geometry_fingerprints: GeometryFingerprints,
    logical_domain: LogicalDomain,
    chunk_grid_spec: ChunkGridSpec,
    materialization_class: MaterializationClass,
) -> ArtifactKey {
    let cache_key = ProcessingCacheFingerprint::fingerprint_json(&(
        &lineage_digest,
        &geometry_fingerprints,
        &logical_domain,
        &chunk_grid_spec,
        materialization_class,
    ))
    .expect("artifact key should serialize");
    ArtifactKey {
        lineage_digest,
        geometry_fingerprints,
        logical_domain,
        chunk_grid_spec,
        materialization_class,
        cache_key,
    }
}

fn boundary_reason_for_stage(
    stage: &ExecutionStage,
    request: &PlanProcessingRequest,
    family: ProcessingPipelineFamily,
) -> ArtifactBoundaryReason {
    match stage.stage_kind {
        ExecutionStageKind::Checkpoint => {
            if stage
                .pipeline_segment
                .as_ref()
                .is_some_and(|segment| is_trace_local_prefix_boundary(request, segment))
            {
                ArtifactBoundaryReason::TraceLocalPrefix
            } else {
                ArtifactBoundaryReason::AuthoredCheckpoint
            }
        }
        ExecutionStageKind::FinalizeOutput => match family {
            ProcessingPipelineFamily::Subvolume => ArtifactBoundaryReason::GeometryDomainChange,
            ProcessingPipelineFamily::TraceLocal
            | ProcessingPipelineFamily::PostStackNeighborhood
            | ProcessingPipelineFamily::Gather => ArtifactBoundaryReason::FinalOutput,
        },
        ExecutionStageKind::ReuseArtifact => ArtifactBoundaryReason::ExternalInputFanIn,
        ExecutionStageKind::Compute => {
            if stage.classification.requires_full_volume {
                ArtifactBoundaryReason::FullVolumeBarrier
            } else if stage.classification.uses_external_inputs {
                ArtifactBoundaryReason::ExternalInputFanIn
            } else {
                ArtifactBoundaryReason::FamilyOperationBlock
            }
        }
    }
}

fn materialization_class_for_stage(stage: &ExecutionStage) -> MaterializationClass {
    match stage.stage_kind {
        ExecutionStageKind::Checkpoint => MaterializationClass::Checkpoint,
        ExecutionStageKind::FinalizeOutput => MaterializationClass::PublishedOutput,
        ExecutionStageKind::ReuseArtifact => MaterializationClass::ReusedArtifact,
        ExecutionStageKind::Compute => match stage.partition_spec.family {
            PartitionFamily::Section => MaterializationClass::EphemeralWindow,
            _ => MaterializationClass::EphemeralPartition,
        },
    }
}

fn lifetime_class_for_role(role: ExecutionArtifactRole) -> ArtifactLifetimeClass {
    match role {
        ExecutionArtifactRole::Input => ArtifactLifetimeClass::Source,
        ExecutionArtifactRole::Checkpoint => ArtifactLifetimeClass::Checkpoint,
        ExecutionArtifactRole::FinalOutput => ArtifactLifetimeClass::Published,
        ExecutionArtifactRole::CachedReuse => ArtifactLifetimeClass::CachedReuse,
    }
}

fn estimated_live_set_bytes_for_profile(profile: &StageMemoryProfile) -> u64 {
    profile
        .primary_tile_bytes
        .saturating_add(
            profile
                .secondary_tile_bytes_per_input
                .saturating_mul(profile.secondary_input_count as u64),
        )
        .saturating_add(profile.output_tile_bytes)
        .saturating_add(profile.shared_stage_bytes)
}

fn assemble_execution_plan_pass(context: PlannerPassContext<'_>) -> Result<ExecutionPlan, String> {
    let plan_summary = context
        .plan_summary
        .clone()
        .expect("artifact planning should populate plan summary");
    let pipeline = context
        .pipeline_descriptor
        .clone()
        .expect("validate pass should populate pipeline descriptor");
    let source_identity = context
        .source_identity
        .clone()
        .expect("validate pass should populate source identity");
    let pipeline_identity = context
        .pipeline_identity
        .clone()
        .expect("validate pass should populate pipeline identity");
    let operator_set_identity = context
        .operator_set_identity
        .clone()
        .expect("validate pass should populate operator set identity");
    let planner_profile_identity = context
        .planner_profile_identity
        .clone()
        .expect("validate pass should populate planner profile identity");
    let validation = context
        .validation
        .clone()
        .expect("validate pass should populate validation report");
    let runtime_environment = context
        .runtime_environment
        .clone()
        .expect("plan artifacts pass should populate runtime environment");
    let mut planner_pass_snapshots = context.planner_pass_snapshots;
    let mut plan_decisions = context.plan_decisions;
    let assemble_decision_id = "pass-assemble-execution-plan".to_string();
    plan_decisions.push(PlanDecision {
        decision_id: assemble_decision_id.clone(),
        subject_kind: PlanDecisionSubjectKind::PlannerPass,
        subject_id: "assemble_execution_plan".to_string(),
        decision_kind: PlanDecisionKind::Lowering,
        reason_code: "assembled_execution_plan".to_string(),
        human_summary: format!(
            "compute_stages={} max_peak_memory_bytes={} combined_cpu_weight={} combined_io_weight={}",
            plan_summary.compute_stage_count,
            plan_summary.max_estimated_peak_memory_bytes,
            plan_summary.combined_cpu_weight,
            plan_summary.combined_io_weight
        ),
        stage_planning: None,
        reuse_decision: None,
        artifact_derivation: None,
    });
    planner_pass_snapshots.push(PlannerPassSnapshot {
        pass_id: PlanningPassId::AssembleExecutionPlan,
        pass_name: "assemble_execution_plan".to_string(),
        snapshot_text: Some(format!(
            "compute_stages={} max_peak_memory_bytes={} combined_cpu_weight={} combined_io_weight={}",
            plan_summary.compute_stage_count,
            plan_summary.max_estimated_peak_memory_bytes,
            plan_summary.combined_cpu_weight,
            plan_summary.combined_io_weight
        )),
        decision_ids: vec![assemble_decision_id],
    });

    Ok(ExecutionPlan {
        schema_version: 1,
        plan_id: format!("plan-{}", Uuid::new_v4()),
        planning_mode: context.request.planning_mode,
        source: ExecutionSourceDescriptor {
            store_path: context.request.store_path.clone(),
            layout: context.request.layout,
            shape: context.request.source_shape,
            chunk_shape: context.request.source_chunk_shape,
        },
        source_identity,
        pipeline,
        pipeline_identity,
        operator_set_identity,
        planner_profile_identity,
        runtime_environment,
        stages: context.stages,
        plan_summary,
        artifacts: context.artifacts,
        scheduler_hints: SchedulerHints {
            priority_class: priority_for_mode(context.request.planning_mode),
            max_active_partitions: context.request.max_active_partitions,
            expected_partition_count: context.expected_partition_count,
        },
        validation,
        planner_diagnostics: PlannerDiagnostics {
            pass_snapshots: planner_pass_snapshots,
        },
        plan_decisions,
    })
}

fn semantic_segments_for_pipeline(pipeline: &NormalizedPipeline) -> Vec<SemanticPipelineSegment> {
    match pipeline {
        NormalizedPipeline::TraceLocal { pipeline } => trace_local_semantic_segments(pipeline),
        NormalizedPipeline::PostStackNeighborhood { pipeline } => {
            let mut segments = prefixed_family_segments(
                pipeline.trace_local_pipeline.as_ref(),
                pipeline.operations.len(),
                ProcessingPipelineFamily::PostStackNeighborhood,
                "family_operations",
                "family_operation_block",
            );
            if segments.is_empty() {
                segments.push(SemanticPipelineSegment {
                    family: ProcessingPipelineFamily::PostStackNeighborhood,
                    start_step_index: 0,
                    end_step_index: 0,
                    role: "family_operations",
                    boundary_reason: "family_operation_block",
                });
            }
            segments
        }
        NormalizedPipeline::Subvolume { pipeline } => prefixed_family_segments(
            pipeline.trace_local_pipeline.as_ref(),
            1,
            ProcessingPipelineFamily::Subvolume,
            "crop",
            "geometry_boundary",
        ),
        NormalizedPipeline::Gather { pipeline } => prefixed_family_segments(
            pipeline.trace_local_pipeline.as_ref(),
            pipeline.operations.len(),
            ProcessingPipelineFamily::Gather,
            "family_operations",
            "family_operation_block",
        ),
    }
}

fn trace_local_semantic_segments(
    pipeline: &TraceLocalProcessingPipeline,
) -> Vec<SemanticPipelineSegment> {
    let mut segments = Vec::new();
    let mut segment_start = 0usize;
    for (index, step) in pipeline.steps.iter().enumerate() {
        let is_final_step = index + 1 == pipeline.steps.len();
        if !step.checkpoint && !is_final_step {
            continue;
        }
        segments.push(SemanticPipelineSegment {
            family: ProcessingPipelineFamily::TraceLocal,
            start_step_index: segment_start,
            end_step_index: index,
            role: "trace_local_segment",
            boundary_reason: if is_final_step {
                "final_output"
            } else {
                "authored_checkpoint"
            },
        });
        segment_start = index + 1;
    }
    segments
}

fn prefixed_family_segments(
    trace_local_pipeline: Option<&TraceLocalProcessingPipeline>,
    family_operation_count: usize,
    family: ProcessingPipelineFamily,
    family_role: &'static str,
    family_boundary_reason: &'static str,
) -> Vec<SemanticPipelineSegment> {
    let mut segments = Vec::new();
    let prefix_len = trace_local_pipeline
        .map(|pipeline| pipeline.operation_count())
        .unwrap_or(0);
    if let Some(prefix) = trace_local_pipeline {
        segments.extend(
            trace_local_semantic_segments(prefix)
                .into_iter()
                .map(|segment| SemanticPipelineSegment {
                    boundary_reason: "trace_local_prefix",
                    ..segment
                }),
        );
    }
    if family_operation_count > 0 {
        segments.push(SemanticPipelineSegment {
            family,
            start_step_index: prefix_len,
            end_step_index: prefix_len + family_operation_count - 1,
            role: family_role,
            boundary_reason: family_boundary_reason,
        });
    }
    segments
}

fn semantic_segment_partition_outlook(
    segments: &[SemanticPipelineSegment],
    operator_traits: &[OperatorExecutionTraits],
    source_shape: Option<[usize; 3]>,
    source_chunk_shape: Option<[usize; 3]>,
) -> Vec<SemanticSegmentPartitionOutlook> {
    segments
        .iter()
        .filter_map(|segment| {
            let end_index = segment
                .end_step_index
                .min(operator_traits.len().saturating_sub(1));
            if segment.start_step_index > end_index || operator_traits.is_empty() {
                return None;
            }
            let segment_traits = &operator_traits[segment.start_step_index..=end_index];
            let partition_spec = partition_spec_for_traits(segment_traits);
            Some(SemanticSegmentPartitionOutlook {
                segment: segment.clone(),
                partition_family: partition_spec.family,
                target_bytes: partition_spec.target_bytes,
                expected_partition_count: estimate_partition_count(
                    &partition_spec,
                    source_shape,
                    source_chunk_shape,
                ),
            })
        })
        .collect()
}

fn pipeline_family_name(family: ProcessingPipelineFamily) -> &'static str {
    match family {
        ProcessingPipelineFamily::TraceLocal => "trace_local",
        ProcessingPipelineFamily::PostStackNeighborhood => "post_stack_neighborhood",
        ProcessingPipelineFamily::Subvolume => "subvolume",
        ProcessingPipelineFamily::Gather => "gather",
    }
}

fn partition_family_name(family: PartitionFamily) -> &'static str {
    match family {
        PartitionFamily::TileGroup => "tile_group",
        PartitionFamily::Section => "section",
        PartitionFamily::GatherGroup => "gather_group",
        PartitionFamily::FullVolume => "full_volume",
    }
}

fn execution_queue_class_name(queue_class: ExecutionQueueClass) -> &'static str {
    match queue_class {
        ExecutionQueueClass::Control => "control",
        ExecutionQueueClass::InteractivePartition => "interactive_partition",
        ExecutionQueueClass::ForegroundPartition => "foreground_partition",
        ExecutionQueueClass::BackgroundPartition => "background_partition",
        ExecutionQueueClass::ExclusiveFullVolume => "exclusive_full_volume",
    }
}

fn memory_cost_class_name(cost: MemoryCostClass) -> &'static str {
    match cost {
        MemoryCostClass::Low => "low",
        MemoryCostClass::Medium => "medium",
        MemoryCostClass::High => "high",
    }
}

fn cpu_cost_class_name(cost: CpuCostClass) -> &'static str {
    match cost {
        CpuCostClass::Low => "low",
        CpuCostClass::Medium => "medium",
        CpuCostClass::High => "high",
    }
}

fn io_cost_class_name(cost: IoCostClass) -> &'static str {
    match cost {
        IoCostClass::Low => "low",
        IoCostClass::Medium => "medium",
        IoCostClass::High => "high",
    }
}

fn parallel_efficiency_name(efficiency: ParallelEfficiencyClass) -> &'static str {
    match efficiency {
        ParallelEfficiencyClass::High => "high",
        ParallelEfficiencyClass::Medium => "medium",
        ParallelEfficiencyClass::Low => "low",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceLocalChunkPlanResolution {
    pub chunk_plan: TraceLocalChunkPlanRecommendation,
    pub bytes_per_tile: u64,
    pub total_tiles: usize,
    pub preferred_partition_count: usize,
    pub available_memory_bytes: Option<u64>,
    pub reserved_memory_bytes: u64,
    pub usable_memory_bytes: Option<u64>,
}

impl TraceLocalChunkPlanResolution {
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
) -> TraceLocalChunkPlanResolution {
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
) -> TraceLocalChunkPlanResolution {
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
) -> Option<TraceLocalChunkPlanResolution> {
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
) -> Option<TraceLocalChunkPlanResolution> {
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

    Some(TraceLocalChunkPlanResolution {
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
) -> TraceLocalChunkPlanResolution {
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

    TraceLocalChunkPlanResolution {
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
    planning_mode: PlanningMode,
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
            artifact_key: None,
            logical_domain: None,
            chunk_grid_spec: None,
            geometry_fingerprints: None,
            materialization_class: None,
            boundary_reason: None,
            lifetime_class: None,
            reuse_requirement: None,
            reuse_resolution: None,
            reuse_decision_id: None,
            artifact_derivation_decision_id: None,
        });

        let mut partition_spec = partition_spec_for_traits(stage_traits);
        let classification = stage_execution_classification_for_traits(stage_traits);
        let expected_partition_count =
            estimate_partition_count(&partition_spec, source_shape, source_chunk_shape);
        partition_spec.requires_barrier = matches!(
            stage_kind,
            ExecutionStageKind::Checkpoint | ExecutionStageKind::FinalizeOutput
        );
        let progress_total = expected_partition_count.unwrap_or(1) as u64;
        let estimated_cost = cost_estimate_for_traits(stage_traits);
        let resource_envelope = stage_resource_envelope_for_stage(
            planning_mode,
            stage_kind,
            &classification,
            stage_memory_profile.as_ref(),
            expected_partition_count,
            &estimated_cost,
        );
        stages.push(ExecutionStage {
            stage_id: format!("stage-{stage_index:02}"),
            stage_label: String::new(),
            stage_kind,
            input_artifact_ids: vec![current_input_artifact_id.clone()],
            output_artifact_id: output_artifact_id.clone(),
            pipeline_segment: Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::TraceLocal,
                start_step_index: segment_start,
                end_step_index: index,
            }),
            expected_partition_count,
            partition_spec,
            halo_spec: halo_spec_for_traits(stage_traits),
            chunk_shape_policy: ChunkShapePolicy::InheritSource,
            cache_mode,
            retry_policy: RetryPolicy { max_attempts: 1 },
            progress_units: ProgressUnits {
                total: progress_total,
            },
            classification: classification.clone(),
            memory_cost_class: classification.max_memory_cost_class,
            estimated_cost,
            stage_memory_profile,
            resource_envelope,
            lowered_scheduler_policy: unlowered_stage_scheduler_policy(
                &[current_input_artifact_id.clone()],
                &output_artifact_id,
            ),
            boundary_reason: None,
            materialization_class: None,
            reuse_class: None,
            output_artifact_key: None,
            live_set: None,
            reuse_requirement: None,
            reuse_resolution: None,
            planning_decision_id: None,
            reuse_decision_id: None,
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
    planning_mode: PlanningMode,
) -> (Vec<ExecutionStage>, Vec<ArtifactDescriptor>) {
    let output_artifact_id = "final-output".to_string();
    let mut partition_spec = partition_spec_for_traits(operator_traits);
    let classification = stage_execution_classification_for_traits(operator_traits);
    let expected_partition_count =
        estimate_partition_count(&partition_spec, source_shape, source_chunk_shape);
    partition_spec.requires_barrier = matches!(
        pipeline_descriptor.family,
        ProcessingPipelineFamily::Subvolume
    );
    let estimated_cost = cost_estimate_for_traits(operator_traits);
    let resource_envelope = stage_resource_envelope_for_stage(
        planning_mode,
        ExecutionStageKind::FinalizeOutput,
        &classification,
        None,
        expected_partition_count,
        &estimated_cost,
    );
    let stage = ExecutionStage {
        stage_id: "stage-01".to_string(),
        stage_label: String::new(),
        stage_kind: ExecutionStageKind::FinalizeOutput,
        input_artifact_ids: vec![source_artifact_id.to_string()],
        output_artifact_id: output_artifact_id.clone(),
        pipeline_segment: Some(ExecutionPipelineSegment {
            family: pipeline_descriptor.family,
            start_step_index: 0,
            end_step_index: operator_traits.len().saturating_sub(1),
        }),
        expected_partition_count,
        partition_spec,
        halo_spec: halo_spec_for_traits(operator_traits),
        chunk_shape_policy: ChunkShapePolicy::InheritSource,
        cache_mode: CacheMode::PreferReuse,
        retry_policy: RetryPolicy { max_attempts: 1 },
        progress_units: ProgressUnits {
            total: expected_partition_count.unwrap_or(1) as u64,
        },
        classification: classification.clone(),
        memory_cost_class: classification.max_memory_cost_class,
        estimated_cost,
        stage_memory_profile: None,
        resource_envelope,
        lowered_scheduler_policy: unlowered_stage_scheduler_policy(
            &[source_artifact_id.to_string()],
            &output_artifact_id,
        ),
        boundary_reason: None,
        materialization_class: None,
        reuse_class: None,
        output_artifact_key: None,
        live_set: None,
        reuse_requirement: None,
        reuse_resolution: None,
        planning_decision_id: None,
        reuse_decision_id: None,
    };
    let artifact = ArtifactDescriptor {
        artifact_id: output_artifact_id,
        role: ExecutionArtifactRole::FinalOutput,
        store_path: output_store_path
            .map(str::to_string)
            .or(Some(artifact_label)),
        cache_key: None,
        artifact_key: None,
        logical_domain: None,
        chunk_grid_spec: None,
        geometry_fingerprints: None,
        materialization_class: None,
        boundary_reason: None,
        lifetime_class: None,
        reuse_requirement: None,
        reuse_resolution: None,
        reuse_decision_id: None,
        artifact_derivation_decision_id: None,
    };
    (vec![stage], vec![artifact])
}

fn build_subvolume_plan(
    pipeline_descriptor: &PipelineDescriptor,
    pipeline: &SubvolumeProcessingPipeline,
    source_artifact_id: &str,
    output_store_path: Option<&str>,
    operator_traits: &[OperatorExecutionTraits],
    artifact_label: String,
    source_shape: Option<[usize; 3]>,
    source_chunk_shape: Option<[usize; 3]>,
    planning_mode: PlanningMode,
) -> (Vec<ExecutionStage>, Vec<ArtifactDescriptor>) {
    build_prefixed_family_plan(
        pipeline_descriptor,
        pipeline.trace_local_pipeline.as_ref(),
        source_artifact_id,
        output_store_path,
        operator_traits,
        artifact_label,
        source_shape,
        source_chunk_shape,
        planning_mode,
    )
}

fn build_prefixed_family_plan(
    pipeline_descriptor: &PipelineDescriptor,
    trace_local_pipeline: Option<&TraceLocalProcessingPipeline>,
    source_artifact_id: &str,
    output_store_path: Option<&str>,
    operator_traits: &[OperatorExecutionTraits],
    artifact_label: String,
    source_shape: Option<[usize; 3]>,
    source_chunk_shape: Option<[usize; 3]>,
    planning_mode: PlanningMode,
) -> (Vec<ExecutionStage>, Vec<ArtifactDescriptor>) {
    let Some(trace_local_pipeline) = trace_local_pipeline else {
        return build_single_stage_plan(
            pipeline_descriptor,
            source_artifact_id,
            output_store_path,
            operator_traits,
            artifact_label,
            source_shape,
            source_chunk_shape,
            planning_mode,
        );
    };
    let prefix_len = trace_local_pipeline.operation_count();
    if prefix_len == 0 || prefix_len >= operator_traits.len() {
        return build_single_stage_plan(
            pipeline_descriptor,
            source_artifact_id,
            output_store_path,
            operator_traits,
            artifact_label,
            source_shape,
            source_chunk_shape,
            planning_mode,
        );
    }

    let (mut stages, mut artifacts) = build_trace_local_stages(
        trace_local_pipeline,
        source_artifact_id,
        None,
        &operator_traits[..prefix_len],
        source_shape,
        source_chunk_shape,
        planning_mode,
    );
    let prefix_checkpoint_id = format!("checkpoint-{:02}", stages.len());
    let old_prefix_output_artifact_id = stages
        .last()
        .map(|stage| stage.output_artifact_id.clone())
        .unwrap_or_else(|| "final-output".to_string());
    if let Some(prefix_stage) = stages.last_mut() {
        prefix_stage.stage_kind = ExecutionStageKind::Checkpoint;
        prefix_stage.output_artifact_id = prefix_checkpoint_id.clone();
    }
    if let Some(prefix_artifact) = artifacts
        .iter_mut()
        .find(|artifact| artifact.artifact_id == old_prefix_output_artifact_id)
    {
        prefix_artifact.artifact_id = prefix_checkpoint_id.clone();
        prefix_artifact.role = ExecutionArtifactRole::Checkpoint;
        prefix_artifact.store_path = None;
    }

    let family_traits = &operator_traits[prefix_len..];
    let output_artifact_id = "final-output".to_string();
    let mut partition_spec = partition_spec_for_traits(family_traits);
    let classification = stage_execution_classification_for_traits(family_traits);
    let expected_partition_count =
        estimate_partition_count(&partition_spec, source_shape, source_chunk_shape);
    partition_spec.requires_barrier = true;
    let estimated_cost = cost_estimate_for_traits(family_traits);
    let resource_envelope = stage_resource_envelope_for_stage(
        planning_mode,
        ExecutionStageKind::FinalizeOutput,
        &classification,
        None,
        expected_partition_count,
        &estimated_cost,
    );
    stages.push(ExecutionStage {
        stage_id: format!("stage-{:02}", stages.len() + 1),
        stage_label: String::new(),
        stage_kind: ExecutionStageKind::FinalizeOutput,
        input_artifact_ids: vec![prefix_checkpoint_id.clone()],
        output_artifact_id: output_artifact_id.clone(),
        pipeline_segment: Some(ExecutionPipelineSegment {
            family: pipeline_descriptor.family,
            start_step_index: prefix_len,
            end_step_index: operator_traits.len() - 1,
        }),
        expected_partition_count,
        partition_spec,
        halo_spec: halo_spec_for_traits(family_traits),
        chunk_shape_policy: ChunkShapePolicy::InheritSource,
        cache_mode: CacheMode::PreferReuse,
        retry_policy: RetryPolicy { max_attempts: 1 },
        progress_units: ProgressUnits {
            total: expected_partition_count.unwrap_or(1) as u64,
        },
        classification: classification.clone(),
        memory_cost_class: classification.max_memory_cost_class,
        estimated_cost,
        stage_memory_profile: None,
        resource_envelope,
        lowered_scheduler_policy: unlowered_stage_scheduler_policy(
            &[prefix_checkpoint_id.clone()],
            &output_artifact_id,
        ),
        boundary_reason: None,
        materialization_class: None,
        reuse_class: None,
        output_artifact_key: None,
        live_set: None,
        reuse_requirement: None,
        reuse_resolution: None,
        planning_decision_id: None,
        reuse_decision_id: None,
    });
    artifacts.push(ArtifactDescriptor {
        artifact_id: output_artifact_id,
        role: ExecutionArtifactRole::FinalOutput,
        store_path: output_store_path
            .map(str::to_string)
            .or(Some(artifact_label)),
        cache_key: None,
        artifact_key: None,
        logical_domain: None,
        chunk_grid_spec: None,
        geometry_fingerprints: None,
        materialization_class: None,
        boundary_reason: None,
        lifetime_class: None,
        reuse_requirement: None,
        reuse_resolution: None,
        reuse_decision_id: None,
        artifact_derivation_decision_id: None,
    });
    (stages, artifacts)
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

fn stage_resource_envelope_for_stage(
    planning_mode: PlanningMode,
    stage_kind: ExecutionStageKind,
    classification: &StageExecutionClassification,
    stage_memory_profile: Option<&StageMemoryProfile>,
    expected_partition_count: Option<usize>,
    estimated_cost: &CostEstimate,
) -> StageResourceEnvelope {
    let preferred_queue_class = if classification.requires_full_volume {
        ExecutionQueueClass::ExclusiveFullVolume
    } else {
        match planning_mode {
            PlanningMode::InteractivePreview => ExecutionQueueClass::InteractivePartition,
            PlanningMode::ForegroundMaterialize => ExecutionQueueClass::ForegroundPartition,
            PlanningMode::BackgroundBatch => ExecutionQueueClass::BackgroundPartition,
        }
    };
    let resident_bytes_per_partition = stage_memory_profile
        .map(|profile| {
            profile
                .reserve_hint_bytes
                .saturating_add(profile.shared_stage_bytes)
                .saturating_add(profile.output_tile_bytes)
                .max(estimated_cost.estimated_peak_memory_bytes)
        })
        .unwrap_or(estimated_cost.estimated_peak_memory_bytes);
    let workspace_bytes_per_worker = stage_memory_profile
        .map(|profile| profile.per_worker_workspace_bytes)
        .unwrap_or_else(|| resident_bytes_per_partition / 4);
    let partition_count = expected_partition_count.unwrap_or(1).max(1);
    let progress_granularity = if partition_count > 1 {
        ExecutionProgressGranularity::Partition
    } else {
        ExecutionProgressGranularity::Stage
    };
    let retry_granularity = if partition_count > 1 {
        ExecutionRetryGranularity::Partition
    } else {
        ExecutionRetryGranularity::Stage
    };

    StageResourceEnvelope {
        preferred_queue_class,
        spillability: if classification.requires_full_volume {
            ExecutionSpillabilityClass::Exclusive
        } else if stage_memory_profile.is_some() {
            ExecutionSpillabilityClass::Spillable
        } else {
            ExecutionSpillabilityClass::Unspillable
        },
        exclusive_scope: if classification.requires_full_volume {
            ExecutionExclusiveScope::FullVolume
        } else {
            ExecutionExclusiveScope::None
        },
        retry_granularity,
        progress_granularity,
        min_partition_count: Some(1),
        target_partition_count: expected_partition_count,
        max_partition_count: expected_partition_count,
        preferred_partition_waves: match (planning_mode, stage_kind) {
            (PlanningMode::InteractivePreview, _) => 1,
            (_, ExecutionStageKind::Checkpoint | ExecutionStageKind::FinalizeOutput) => 1,
            (PlanningMode::ForegroundMaterialize, _) => 2,
            (PlanningMode::BackgroundBatch, _) => 1,
        },
        resident_bytes_per_partition,
        workspace_bytes_per_worker,
    }
}

fn runtime_environment_for_stages(
    request: &PlanProcessingRequest,
    stages: &[ExecutionStage],
    plan_summary: &ExecutionPlanSummary,
) -> ExecutionRuntimeEnvironment {
    let worker_budget = request
        .max_active_partitions
        .or_else(|| expected_partition_count_for_stages(stages))
        .unwrap_or(1)
        .max(1);
    let reserve_bytes = stages
        .iter()
        .map(stage_reservation_bytes)
        .max()
        .unwrap_or(plan_summary.max_estimated_peak_memory_bytes)
        .max(plan_summary.max_estimated_peak_memory_bytes);
    let usable_bytes = plan_summary
        .max_live_set_bytes
        .unwrap_or(0)
        .max(reserve_bytes.saturating_mul(worker_budget as u64))
        .max(reserve_bytes);
    let queue_class = stages
        .first()
        .map(|stage| stage.resource_envelope.preferred_queue_class)
        .unwrap_or_else(|| match request.planning_mode {
            PlanningMode::InteractivePreview => ExecutionQueueClass::InteractivePartition,
            PlanningMode::ForegroundMaterialize => ExecutionQueueClass::ForegroundPartition,
            PlanningMode::BackgroundBatch => ExecutionQueueClass::BackgroundPartition,
        });
    let exclusive_scope = stages
        .first()
        .map(|stage| stage.resource_envelope.exclusive_scope)
        .unwrap_or(ExecutionExclusiveScope::None);

    ExecutionRuntimeEnvironment {
        worker_budget,
        memory_budget: ExecutionMemoryBudget {
            usable_bytes,
            reserve_bytes,
            worker_count: worker_budget,
        },
        queue_class,
        exclusive_scope,
        requested_max_active_partitions: request.max_active_partitions,
    }
}

fn annotate_stage_lowering(
    request: &PlanProcessingRequest,
    runtime_environment: &ExecutionRuntimeEnvironment,
    stages: &mut [ExecutionStage],
    plan_decisions: &mut Vec<PlanDecision>,
) {
    for stage in stages.iter_mut() {
        stage.stage_label = execution_stage_label(stage, &request.pipeline);
        stage.lowered_scheduler_policy = LoweredStageSchedulerPolicy {
            queue_class: stage.resource_envelope.preferred_queue_class,
            spillability: stage.resource_envelope.spillability,
            exclusive_scope: stage.resource_envelope.exclusive_scope,
            retry_granularity: stage.resource_envelope.retry_granularity,
            progress_granularity: stage.resource_envelope.progress_granularity,
            min_partition_count: stage.resource_envelope.min_partition_count,
            target_partition_count: stage.resource_envelope.target_partition_count,
            max_partition_count: stage.resource_envelope.max_partition_count,
            effective_max_active_partitions: effective_max_active_partitions_for_stage(
                stage,
                runtime_environment.worker_budget,
            ),
            preferred_partition_waves: stage.resource_envelope.preferred_partition_waves,
            reservation_bytes: stage_reservation_bytes(stage),
            resident_bytes_per_partition: stage.resource_envelope.resident_bytes_per_partition,
            workspace_bytes_per_worker: stage.resource_envelope.workspace_bytes_per_worker,
            ownership: StageResourceOwnership {
                input_artifact_ids: stage.input_artifact_ids.clone(),
                output_artifact_id: stage.output_artifact_id.clone(),
                live_artifact_ids: stage
                    .live_set
                    .as_ref()
                    .map(|live_set| {
                        live_set
                            .resident_artifacts
                            .iter()
                            .map(|entry| entry.artifact_id.clone())
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        };

        let planning_decision_id = format!("stage-plan-{}", stage.stage_id);
        plan_decisions.push(PlanDecision {
            decision_id: planning_decision_id.clone(),
            subject_kind: PlanDecisionSubjectKind::Stage,
            subject_id: stage.stage_id.clone(),
            decision_kind: PlanDecisionKind::Scheduling,
            reason_code: partition_family_name(stage.partition_spec.family).to_string(),
            human_summary: format!(
                "stage {} lowers to {} queue reserve={} active_partitions={} ownership={}=>{}",
                stage.stage_id,
                execution_queue_class_name(stage.lowered_scheduler_policy.queue_class),
                stage.lowered_scheduler_policy.reservation_bytes,
                stage
                    .lowered_scheduler_policy
                    .effective_max_active_partitions,
                stage.input_artifact_ids.join(","),
                stage.output_artifact_id
            ),
            stage_planning: Some(StagePlanningDecision {
                selected_partition_family: stage.partition_spec.family,
                selected_ordering: stage.partition_spec.ordering,
                selected_target_bytes: stage.partition_spec.target_bytes,
                selected_expected_partition_count: stage.expected_partition_count,
                selected_queue_class: stage.lowered_scheduler_policy.queue_class,
                selected_spillability: stage.lowered_scheduler_policy.spillability,
                selected_exclusive_scope: stage.lowered_scheduler_policy.exclusive_scope,
                selected_preferred_partition_waves: stage
                    .lowered_scheduler_policy
                    .preferred_partition_waves,
                selected_reservation_bytes: stage.lowered_scheduler_policy.reservation_bytes,
                factors: vec![
                    DecisionFactor {
                        code: "planning_mode".to_string(),
                        summary: "planner mode".to_string(),
                        value: Some(format!("{:?}", request.planning_mode).to_ascii_lowercase()),
                    },
                    DecisionFactor {
                        code: "stage_label".to_string(),
                        summary: "lowered execution label".to_string(),
                        value: Some(stage.stage_label.clone()),
                    },
                    DecisionFactor {
                        code: "worker_budget".to_string(),
                        summary: "planner-visible worker budget".to_string(),
                        value: Some(runtime_environment.worker_budget.to_string()),
                    },
                    DecisionFactor {
                        code: "memory_budget_bytes".to_string(),
                        summary: "planner-visible memory budget".to_string(),
                        value: Some(runtime_environment.memory_budget.usable_bytes.to_string()),
                    },
                    DecisionFactor {
                        code: "effective_max_active_partitions".to_string(),
                        summary: "admitted partition concurrency".to_string(),
                        value: Some(
                            stage
                                .lowered_scheduler_policy
                                .effective_max_active_partitions
                                .to_string(),
                        ),
                    },
                    DecisionFactor {
                        code: "live_artifact_count".to_string(),
                        summary: "planner-visible live artifacts".to_string(),
                        value: Some(
                            stage
                                .lowered_scheduler_policy
                                .ownership
                                .live_artifact_ids
                                .len()
                                .to_string(),
                        ),
                    },
                ],
            }),
            reuse_decision: None,
            artifact_derivation: None,
        });
        stage.planning_decision_id = Some(planning_decision_id);
    }
}

fn stage_reservation_bytes(stage: &ExecutionStage) -> u64 {
    stage
        .stage_memory_profile
        .as_ref()
        .map(|profile| profile.reserve_hint_bytes)
        .unwrap_or_else(|| {
            stage.estimated_cost.estimated_peak_memory_bytes.max(
                stage
                    .resource_envelope
                    .resident_bytes_per_partition
                    .saturating_add(stage.resource_envelope.workspace_bytes_per_worker),
            )
        })
}

fn effective_max_active_partitions_for_stage(
    stage: &ExecutionStage,
    worker_budget: usize,
) -> usize {
    if matches!(
        stage.resource_envelope.exclusive_scope,
        ExecutionExclusiveScope::FullVolume
    ) {
        return 1;
    }
    let partition_cap = stage
        .resource_envelope
        .max_partition_count
        .or(stage.resource_envelope.target_partition_count)
        .or(stage.expected_partition_count)
        .unwrap_or(1)
        .max(1);
    partition_cap.min(worker_budget.max(1)).max(1)
}

fn unlowered_stage_scheduler_policy(
    input_artifact_ids: &[String],
    output_artifact_id: &str,
) -> LoweredStageSchedulerPolicy {
    LoweredStageSchedulerPolicy {
        queue_class: ExecutionQueueClass::BackgroundPartition,
        spillability: ExecutionSpillabilityClass::Unspillable,
        exclusive_scope: ExecutionExclusiveScope::None,
        retry_granularity: ExecutionRetryGranularity::Stage,
        progress_granularity: ExecutionProgressGranularity::Stage,
        min_partition_count: None,
        target_partition_count: None,
        max_partition_count: None,
        effective_max_active_partitions: 1,
        preferred_partition_waves: 1,
        reservation_bytes: 0,
        resident_bytes_per_partition: 0,
        workspace_bytes_per_worker: 0,
        ownership: StageResourceOwnership {
            input_artifact_ids: input_artifact_ids.to_vec(),
            output_artifact_id: output_artifact_id.to_string(),
            live_artifact_ids: Vec::new(),
        },
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
        max_live_set_bytes: compute_stages
            .iter()
            .filter_map(|stage| {
                stage
                    .live_set
                    .as_ref()
                    .map(|live_set| live_set.estimated_resident_bytes)
            })
            .max(),
        max_live_artifact_count: compute_stages
            .iter()
            .filter_map(|stage| {
                stage
                    .live_set
                    .as_ref()
                    .map(|live_set| live_set.resident_artifacts.len())
            })
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
        GatherInterpolationMode, GatherProcessingOperation, GatherProcessingPipeline,
        PostStackNeighborhoodProcessingOperation, PostStackNeighborhoodProcessingPipeline,
        PostStackNeighborhoodWindow, SubvolumeCropOperation, SubvolumeProcessingPipeline,
        TraceLocalProcessingOperation, TraceLocalProcessingPipeline, TraceLocalProcessingStep,
        TraceLocalVolumeArithmeticOperator, VelocityFunctionSource,
    };
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        VolumeMetadata, generate_store_id, segy_sample_data_fidelity,
    };
    use crate::storage::tbvol::TbvolManifest;

    fn canonical_test_store_path(
        label: &str,
        shape: [usize; 3],
        chunk_shape: [usize; 3],
    ) -> String {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ophiolite-planner-{label}-{unique}.tbvol"));
        fs::create_dir_all(&path).expect("create test store directory");
        let manifest = TbvolManifest::new(
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
                    sample_data_fidelity: segy_sample_data_fidelity(5),
                    endianness: "little".to_string(),
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
                axes: VolumeAxes::from_time_axis(
                    (0..shape[0]).map(|value| value as f64).collect(),
                    (0..shape[1]).map(|value| value as f64).collect(),
                    (0..shape[2]).map(|value| value as f32 * 2.0).collect(),
                ),
                segy_export: None,
                coordinate_reference_binding: None,
                spatial: None,
                created_by: "test".to_string(),
                processing_lineage: None,
            },
            chunk_shape,
            false,
        );
        fs::write(
            path.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
        path.to_string_lossy().into_owned()
    }

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
        assert_eq!(plan.plan_summary.max_io_cost_class, IoCostClass::Medium);
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
    fn planner_lowers_runtime_environment_and_stage_policy() {
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
                    name: Some("policy-demo".to_string()),
                    description: None,
                    steps: vec![
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::AmplitudeScalar {
                                factor: 2.0,
                            },
                            checkpoint: true,
                        },
                        TraceLocalProcessingStep {
                            operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        };

        let plan = build_execution_plan(&request).expect("plan should build");
        let checkpoint_stage = &plan.stages[0];

        assert_eq!(plan.runtime_environment.worker_budget, 2);
        assert_eq!(
            plan.runtime_environment.queue_class,
            ExecutionQueueClass::ForegroundPartition
        );
        assert_eq!(
            plan.runtime_environment.exclusive_scope,
            ExecutionExclusiveScope::None
        );
        assert!(plan.runtime_environment.memory_budget.usable_bytes > 0);
        assert!(plan.runtime_environment.memory_budget.reserve_bytes > 0);

        assert_eq!(checkpoint_stage.stage_label, "Step 1: Amplitude Scale");
        assert_eq!(
            checkpoint_stage.lowered_scheduler_policy.queue_class,
            ExecutionQueueClass::ForegroundPartition
        );
        assert_eq!(
            checkpoint_stage
                .lowered_scheduler_policy
                .effective_max_active_partitions,
            1
        );
        assert_eq!(
            checkpoint_stage
                .lowered_scheduler_policy
                .ownership
                .input_artifact_ids,
            vec!["source".to_string()]
        );
        assert_eq!(
            checkpoint_stage
                .lowered_scheduler_policy
                .ownership
                .output_artifact_id,
            checkpoint_stage.output_artifact_id
        );
        assert!(
            checkpoint_stage
                .lowered_scheduler_policy
                .ownership
                .live_artifact_ids
                .contains(&checkpoint_stage.output_artifact_id)
        );
        let decision = plan
            .plan_decisions
            .iter()
            .find(|decision| decision.subject_id == checkpoint_stage.stage_id)
            .and_then(|decision| decision.stage_planning.as_ref())
            .expect("stage planning decision");
        assert_eq!(
            decision.selected_queue_class,
            checkpoint_stage.lowered_scheduler_policy.queue_class
        );
        assert_eq!(
            decision.selected_reservation_bytes,
            checkpoint_stage.lowered_scheduler_policy.reservation_bytes
        );
        assert!(
            decision.factors.iter().any(
                |factor| factor.code == "worker_budget" && factor.value.as_deref() == Some("2")
            )
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

    #[test]
    fn planner_populates_artifact_identity_and_live_set_metadata() {
        let input_store_path =
            canonical_test_store_path("identity-input", [8, 8, 128], [4, 4, 128]);
        let secondary_store_path =
            canonical_test_store_path("identity-secondary", [8, 8, 128], [4, 4, 128]);
        let request = PlanProcessingRequest {
            store_path: input_store_path,
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([8, 8, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: TraceLocalProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: Some("identity-demo".to_string()),
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
                                secondary_store_path,
                            },
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        };

        let plan = build_execution_plan(&request).expect("plan should build");
        let source = plan
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "source")
            .expect("source artifact");
        assert_eq!(
            source.materialization_class,
            Some(MaterializationClass::ReusedArtifact)
        );
        assert_eq!(
            source.boundary_reason,
            Some(ArtifactBoundaryReason::SourceInput)
        );
        assert_eq!(source.lifetime_class, Some(ArtifactLifetimeClass::Source));
        assert!(source.cache_key.is_some());
        assert!(source.artifact_key.is_some());
        assert!(matches!(
            source.logical_domain,
            Some(LogicalDomain::Volume { .. })
        ));

        let checkpoint_stage = plan
            .stages
            .iter()
            .find(|stage| stage.stage_kind == ExecutionStageKind::Checkpoint)
            .expect("checkpoint stage");
        assert_eq!(
            checkpoint_stage.materialization_class,
            Some(MaterializationClass::Checkpoint)
        );
        assert_eq!(
            checkpoint_stage.boundary_reason,
            Some(ArtifactBoundaryReason::AuthoredCheckpoint)
        );
        assert_eq!(
            checkpoint_stage.reuse_class,
            Some(crate::ReuseClass::InPlaceSameWindow)
        );
        assert!(checkpoint_stage.output_artifact_key.is_some());
        assert!(
            checkpoint_stage
                .live_set
                .as_ref()
                .is_some_and(|live_set| live_set.estimated_resident_bytes > 0)
        );

        let final_stage = plan
            .stages
            .iter()
            .find(|stage| stage.stage_kind == ExecutionStageKind::FinalizeOutput)
            .expect("final stage");
        assert_eq!(
            final_stage.materialization_class,
            Some(MaterializationClass::PublishedOutput)
        );
        assert_eq!(
            final_stage.boundary_reason,
            Some(ArtifactBoundaryReason::FinalOutput)
        );
        assert_eq!(
            final_stage.reuse_class,
            Some(crate::ReuseClass::RequiresExternalInputs)
        );
        assert!(plan.plan_summary.max_live_set_bytes.is_some());
        assert!(plan.plan_summary.max_live_artifact_count.is_some());
    }

    #[test]
    fn subvolume_prefix_is_planned_as_native_trace_local_checkpoint_boundary() {
        let input_store_path =
            canonical_test_store_path("subvolume-prefix", [8, 8, 128], [4, 4, 128]);
        let request = PlanProcessingRequest {
            store_path: input_store_path,
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([8, 8, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: ProcessingPipelineSpec::Subvolume {
                pipeline: SubvolumeProcessingPipeline {
                    schema_version: 1,
                    revision: 11,
                    preset_id: None,
                    name: Some("subvolume-with-prefix".to_string()),
                    description: None,
                    trace_local_pipeline: Some(TraceLocalProcessingPipeline {
                        schema_version: 1,
                        revision: 7,
                        preset_id: None,
                        name: Some("prefix".to_string()),
                        description: None,
                        steps: vec![
                            TraceLocalProcessingStep {
                                operation: TraceLocalProcessingOperation::Envelope,
                                checkpoint: false,
                            },
                            TraceLocalProcessingStep {
                                operation: TraceLocalProcessingOperation::InstantaneousPhase,
                                checkpoint: false,
                            },
                        ],
                    }),
                    crop: SubvolumeCropOperation {
                        inline_min: 1,
                        inline_max: 4,
                        xline_min: 2,
                        xline_max: 5,
                        z_min_ms: 0.0,
                        z_max_ms: 127.0,
                    },
                },
            },
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        };

        let plan = build_execution_plan(&request).expect("plan should build");
        assert_eq!(plan.stages.len(), 2);

        let prefix_stage = &plan.stages[0];
        assert_eq!(prefix_stage.stage_kind, ExecutionStageKind::Checkpoint);
        assert_eq!(
            prefix_stage.pipeline_segment,
            Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::TraceLocal,
                start_step_index: 0,
                end_step_index: 1,
            })
        );
        assert_eq!(
            prefix_stage.boundary_reason,
            Some(ArtifactBoundaryReason::TraceLocalPrefix)
        );
        assert_eq!(
            prefix_stage
                .reuse_requirement
                .as_ref()
                .map(|requirement| requirement.boundary_kind),
            Some(ReuseBoundaryKind::TraceLocalPrefix)
        );
        assert_eq!(
            prefix_stage
                .reuse_requirement
                .as_ref()
                .map(|requirement| requirement.artifact.pipeline_family),
            Some(ProcessingPipelineFamily::TraceLocal)
        );
        assert_eq!(
            prefix_stage
                .reuse_requirement
                .as_ref()
                .map(|requirement| requirement.artifact.pipeline_revision),
            Some(7)
        );

        let final_stage = &plan.stages[1];
        assert_eq!(final_stage.stage_kind, ExecutionStageKind::FinalizeOutput);
        assert_eq!(
            final_stage.pipeline_segment,
            Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::Subvolume,
                start_step_index: 2,
                end_step_index: 2,
            })
        );

        let prefix_artifact = plan
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == prefix_stage.output_artifact_id)
            .expect("prefix artifact");
        assert_eq!(prefix_artifact.role, ExecutionArtifactRole::Checkpoint);
        assert_eq!(
            prefix_artifact.boundary_reason,
            Some(ArtifactBoundaryReason::TraceLocalPrefix)
        );
    }

    #[test]
    fn post_stack_prefix_is_planned_as_checkpoint_plus_family_stage() {
        let request = PlanProcessingRequest {
            store_path: "input.tbvol".to_string(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([8, 8, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: PostStackNeighborhoodProcessingPipeline {
                    schema_version: 1,
                    revision: 5,
                    preset_id: None,
                    name: Some("post-stack-with-prefix".to_string()),
                    description: None,
                    trace_local_pipeline: Some(TraceLocalProcessingPipeline {
                        schema_version: 1,
                        revision: 3,
                        preset_id: None,
                        name: Some("prefix".to_string()),
                        description: None,
                        steps: vec![
                            TraceLocalProcessingStep {
                                operation: TraceLocalProcessingOperation::Envelope,
                                checkpoint: false,
                            },
                            TraceLocalProcessingStep {
                                operation: TraceLocalProcessingOperation::InstantaneousPhase,
                                checkpoint: false,
                            },
                        ],
                    }),
                    operations: vec![PostStackNeighborhoodProcessingOperation::Similarity {
                        window: PostStackNeighborhoodWindow {
                            gate_ms: 24.0,
                            inline_stepout: 1,
                            xline_stepout: 1,
                        },
                    }],
                },
            },
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        };

        let plan = build_execution_plan(&request).expect("plan should build");
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.stages[0].stage_kind, ExecutionStageKind::Checkpoint);
        assert_eq!(
            plan.stages[0].pipeline_segment,
            Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::TraceLocal,
                start_step_index: 0,
                end_step_index: 1,
            })
        );
        assert_eq!(
            plan.stages[0].boundary_reason,
            Some(ArtifactBoundaryReason::TraceLocalPrefix)
        );
        assert_eq!(
            plan.stages[1].stage_kind,
            ExecutionStageKind::FinalizeOutput
        );
        assert_eq!(
            plan.stages[1].pipeline_segment,
            Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::PostStackNeighborhood,
                start_step_index: 2,
                end_step_index: 2,
            })
        );
        assert_eq!(
            plan.stages[1].boundary_reason,
            Some(ArtifactBoundaryReason::FinalOutput)
        );
    }

    #[test]
    fn gather_prefix_is_planned_as_checkpoint_plus_family_stage() {
        let request = PlanProcessingRequest {
            store_path: "input.tbgath".to_string(),
            layout: SeismicLayout::PreStack3DOffset,
            source_shape: Some([8, 8, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: ProcessingPipelineSpec::Gather {
                pipeline: GatherProcessingPipeline {
                    schema_version: 1,
                    revision: 8,
                    preset_id: None,
                    name: Some("gather-with-prefix".to_string()),
                    description: None,
                    trace_local_pipeline: Some(TraceLocalProcessingPipeline {
                        schema_version: 1,
                        revision: 4,
                        preset_id: None,
                        name: Some("prefix".to_string()),
                        description: None,
                        steps: vec![
                            TraceLocalProcessingStep {
                                operation: TraceLocalProcessingOperation::AmplitudeScalar {
                                    factor: 2.0,
                                },
                                checkpoint: false,
                            },
                            TraceLocalProcessingStep {
                                operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                                checkpoint: false,
                            },
                        ],
                    }),
                    operations: vec![GatherProcessingOperation::NmoCorrection {
                        velocity_model: VelocityFunctionSource::ConstantVelocity {
                            velocity_m_per_s: 2000.0,
                        },
                        interpolation: GatherInterpolationMode::Linear,
                    }],
                },
            },
            output_store_path: Some("output.tbgath".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        };

        let plan = build_execution_plan(&request).expect("plan should build");
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.stages[0].stage_kind, ExecutionStageKind::Checkpoint);
        assert_eq!(
            plan.stages[0].pipeline_segment,
            Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::TraceLocal,
                start_step_index: 0,
                end_step_index: 1,
            })
        );
        assert_eq!(
            plan.stages[0].boundary_reason,
            Some(ArtifactBoundaryReason::TraceLocalPrefix)
        );
        assert_eq!(
            plan.stages[1].stage_kind,
            ExecutionStageKind::FinalizeOutput
        );
        assert_eq!(
            plan.stages[1].pipeline_segment,
            Some(ExecutionPipelineSegment {
                family: ProcessingPipelineFamily::Gather,
                start_step_index: 2,
                end_step_index: 2,
            })
        );
        assert_eq!(
            plan.stages[1].boundary_reason,
            Some(ArtifactBoundaryReason::FinalOutput)
        );
    }
}
