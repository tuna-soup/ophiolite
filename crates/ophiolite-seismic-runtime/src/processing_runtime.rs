use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::time::Instant;

use ophiolite_seismic::{
    PostStackNeighborhoodProcessingPipeline, ProcessingArtifactRole, ProcessingJobArtifact,
    ProcessingJobArtifactKind, ProcessingJobChunkPlanSummary, ProcessingJobExecutionSummary,
    ProcessingJobStageExecutionSummary, ProcessingPipelineSpec, RunGatherProcessingRequest,
    RunPostStackNeighborhoodProcessingRequest, RunSubvolumeProcessingRequest,
    RunTraceLocalProcessingRequest, SubvolumeProcessingPipeline, TraceLocalProcessingOperation,
    TraceLocalProcessingPipeline, TraceLocalProcessingStep,
};

use crate::execution::{
    ArtifactBoundaryReason, ArtifactKey, ChunkGridSpec, ExecutionPlan, ExecutionStageKind,
    GeometryFingerprints, LogicalDomain, MaterializationClass,
};
use crate::identity::{
    CURRENT_RUNTIME_SEMANTICS_VERSION, CURRENT_STORE_WRITER_SEMANTICS_VERSION,
    CanonicalIdentityStatus, canonical_artifact_identity, combine_canonical_identity_status,
    operator_set_identity_for_pipeline, pipeline_external_identity_status,
    pipeline_semantic_identity, planner_profile_identity_for_pipeline, source_identity_digest,
    source_semantic_identity_from_store_path, source_semantic_identity_with_status_from_store_path,
};
use crate::post_stack_neighborhood::materialize_post_stack_neighborhood_without_prefix_with_progress;
use crate::prestack_store::{
    TbgathManifest, materialize_gather_processing_store_without_prefix_with_progress,
    materialize_trace_local_gather_processing_store_with_progress,
};
use crate::storage::tbvol::TbvolManifest;
use crate::store::open_store;
use crate::{
    MaterializeOptions, PartitionExecutionProgress,
    materialize_processing_volume_with_partition_progress,
    materialize_subvolume_processing_volume_with_progress, resolve_trace_local_materialize_options,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedArtifactLineageSeed {
    pub artifact_key: Option<ArtifactKey>,
    pub input_artifact_keys: Vec<ArtifactKey>,
    pub produced_by_stage_id: Option<String>,
    pub boundary_reason: Option<ArtifactBoundaryReason>,
    pub logical_domain: Option<LogicalDomain>,
    pub chunk_grid_spec: Option<ChunkGridSpec>,
    pub geometry_fingerprints: Option<GeometryFingerprints>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceLocalProcessingStagePlan {
    pub segment_pipeline: TraceLocalProcessingPipeline,
    pub lineage_pipeline: TraceLocalProcessingPipeline,
    pub stage_label: String,
    pub artifact: ProcessingJobArtifact,
    pub partition_target_bytes: Option<u64>,
    pub planned_lineage: Option<PlannedArtifactLineageSeed>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceLocalCheckpointLookupKey {
    pub source_fingerprint: String,
    pub prefix_hash: String,
    pub prefix_step_count: usize,
    pub checkpoint_index: usize,
    pub artifact_key: ArtifactKey,
    pub logical_domain: LogicalDomain,
    pub chunk_grid_spec: ChunkGridSpec,
    pub geometry_fingerprints: GeometryFingerprints,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusedTraceLocalCheckpoint {
    pub after_operation_index: usize,
    pub path: String,
    pub artifact: ProcessingJobArtifact,
}

#[derive(Debug, Clone)]
struct StageExecutionSummaryState {
    stage_label: String,
    completed_partitions: usize,
    total_partitions: Option<usize>,
    retry_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessingExecutionSummaryState {
    completed_partitions: usize,
    total_partitions: Option<usize>,
    active_partitions: usize,
    peak_active_partitions: usize,
    retry_count: usize,
    resolved_chunk_plan: Option<ProcessingJobChunkPlanSummary>,
    stages: Vec<StageExecutionSummaryState>,
}

impl ProcessingExecutionSummaryState {
    fn ensure_stage(&mut self, stage_label: &str) -> usize {
        if let Some(index) = self
            .stages
            .iter()
            .position(|stage| stage.stage_label == stage_label)
        {
            return index;
        }
        self.stages.push(StageExecutionSummaryState {
            stage_label: stage_label.to_string(),
            completed_partitions: 0,
            total_partitions: None,
            retry_count: 0,
        });
        self.stages.len() - 1
    }

    pub fn apply_partition_progress(
        &mut self,
        stage_label: &str,
        progress: PartitionExecutionProgress,
    ) {
        let stage_index = self.ensure_stage(stage_label);
        let previous_stage_completed = self.stages[stage_index].completed_partitions;
        self.stages[stage_index].completed_partitions = progress.completed_partitions;
        self.stages[stage_index].total_partitions = Some(progress.total_partitions);
        self.stages[stage_index].retry_count = progress.retry_count;
        self.completed_partitions = self.completed_partitions.saturating_add(
            progress
                .completed_partitions
                .saturating_sub(previous_stage_completed),
        );
        self.total_partitions = Some(
            self.stages
                .iter()
                .filter_map(|stage| stage.total_partitions)
                .sum::<usize>(),
        );
        self.active_partitions = progress.active_partitions;
        self.peak_active_partitions = self
            .peak_active_partitions
            .max(progress.peak_active_partitions);
        self.retry_count = self.stages.iter().map(|stage| stage.retry_count).sum();
    }

    pub fn set_resolved_chunk_plan(
        &mut self,
        resolved_chunk_plan: Option<ProcessingJobChunkPlanSummary>,
    ) {
        if let Some(resolved_chunk_plan) = resolved_chunk_plan {
            self.resolved_chunk_plan = Some(resolved_chunk_plan);
        }
    }

    pub fn into_contract(self) -> ProcessingJobExecutionSummary {
        ProcessingJobExecutionSummary {
            completed_partitions: self.completed_partitions,
            total_partitions: self.total_partitions,
            active_partitions: self.active_partitions,
            peak_active_partitions: self.peak_active_partitions,
            retry_count: self.retry_count,
            resolved_chunk_plan: self.resolved_chunk_plan,
            stages: self
                .stages
                .into_iter()
                .map(|stage| ProcessingJobStageExecutionSummary {
                    stage_label: stage.stage_label,
                    completed_partitions: stage.completed_partitions,
                    total_partitions: stage.total_partitions,
                    retry_count: stage.retry_count,
                })
                .collect(),
        }
    }
}

fn planned_lineage_seed_for_stage(
    plan: &ExecutionPlan,
    stage_id: &str,
) -> Option<PlannedArtifactLineageSeed> {
    let stage = plan
        .stages
        .iter()
        .find(|stage| stage.stage_id == stage_id)?;
    let artifact = plan
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == stage.output_artifact_id)?;
    let input_artifact_keys = stage
        .input_artifact_ids
        .iter()
        .filter_map(|artifact_id| {
            plan.artifacts
                .iter()
                .find(|artifact| artifact.artifact_id == *artifact_id)
                .and_then(|artifact| artifact.artifact_key.clone())
        })
        .collect();
    Some(PlannedArtifactLineageSeed {
        artifact_key: artifact.artifact_key.clone(),
        input_artifact_keys,
        produced_by_stage_id: Some(stage.stage_id.clone()),
        boundary_reason: artifact.boundary_reason.or(stage.boundary_reason),
        logical_domain: artifact.logical_domain.clone(),
        chunk_grid_spec: artifact.chunk_grid_spec.clone(),
        geometry_fingerprints: artifact.geometry_fingerprints.clone(),
    })
}

fn planned_lineage_seed_for_runtime_stage_label(
    plan: &ExecutionPlan,
    pipeline: &ProcessingPipelineSpec,
    stage_label: &str,
) -> Option<PlannedArtifactLineageSeed> {
    plan.stages
        .iter()
        .find(|stage| runtime_stage_label_for_execution_stage(stage, pipeline) == stage_label)
        .and_then(|stage| planned_lineage_seed_for_stage(plan, &stage.stage_id))
}

#[derive(Debug, Clone)]
pub struct TraceLocalJobStartedEvent {
    pub output_store_path: String,
    pub initial_stage_label: Option<String>,
    pub stage_count: usize,
    pub operator_count: usize,
    pub reused_checkpoint: bool,
}

#[derive(Debug, Clone)]
pub struct TraceLocalStageStartedEvent {
    pub stage: TraceLocalProcessingStagePlan,
    pub input_store_path: String,
}

#[derive(Debug, Clone)]
pub struct TraceLocalStageCompletedEvent {
    pub stage: TraceLocalProcessingStagePlan,
    pub input_store_path: String,
    pub stage_duration_ms: u64,
    pub materialize_duration_ms: u64,
    pub lineage_rewrite_duration_ms: u64,
}

pub trait TraceLocalExecutionObserver {
    fn is_cancelled(&self) -> bool;

    fn on_job_started(&mut self, _event: &TraceLocalJobStartedEvent) {}

    fn on_reused_checkpoint(&mut self, _checkpoint: &ReusedTraceLocalCheckpoint) {}

    fn on_stage_started(&mut self, _event: &TraceLocalStageStartedEvent) {}

    fn on_stage_progress(&mut self, _stage_label: &str, _completed: usize, _total: usize) {}

    fn on_execution_summary(&mut self, _summary: &ProcessingJobExecutionSummary) {}

    fn on_artifact_emitted(
        &mut self,
        _artifact: &ProcessingJobArtifact,
        _lineage_pipeline: &TraceLocalProcessingPipeline,
    ) {
    }

    fn on_stage_completed(&mut self, _event: &TraceLocalStageCompletedEvent) {}
}

#[derive(Debug, Clone)]
pub struct PostStackNeighborhoodJobStartedEvent {
    pub output_store_path: String,
    pub initial_stage_label: String,
    pub stage_count: usize,
    pub operator_count: usize,
}

pub trait PostStackNeighborhoodExecutionObserver {
    fn is_cancelled(&self) -> bool;

    fn on_job_started(&mut self, _event: &PostStackNeighborhoodJobStartedEvent) {}

    fn on_stage_started(&mut self, _stage_label: &str) {}

    fn on_progress(&mut self, _progress_label: &str, _completed: usize, _total: usize) {}
}

#[derive(Debug, Clone)]
pub struct GatherJobStartedEvent {
    pub output_store_path: String,
    pub initial_stage_label: String,
    pub stage_count: usize,
    pub operator_count: usize,
}

pub trait GatherExecutionObserver {
    fn is_cancelled(&self) -> bool;

    fn on_job_started(&mut self, _event: &GatherJobStartedEvent) {}

    fn on_stage_started(&mut self, _stage_label: &str) {}

    fn on_progress(&mut self, _stage_label: &str, _completed: usize, _total: usize) {}
}

#[derive(Debug, Clone)]
pub struct SubvolumeJobStartedEvent {
    pub output_store_path: String,
    pub initial_stage_label: String,
    pub trace_local_operator_count: usize,
    pub checkpoint_count: usize,
    pub reused_checkpoint: bool,
}

#[derive(Debug, Clone)]
pub struct SubvolumeCheckpointStageStartedEvent {
    pub stage: TraceLocalProcessingStagePlan,
    pub input_store_path: String,
}

#[derive(Debug, Clone)]
pub struct SubvolumeCheckpointStageCompletedEvent {
    pub stage: TraceLocalProcessingStagePlan,
    pub stage_duration_ms: u64,
    pub materialize_duration_ms: u64,
    pub lineage_rewrite_duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SubvolumeFinalStageStartedEvent {
    pub input_store_path: String,
    pub output_store_path: String,
}

pub trait SubvolumeExecutionObserver {
    fn is_cancelled(&self) -> bool;

    fn on_job_started(&mut self, _event: &SubvolumeJobStartedEvent) {}

    fn on_reused_checkpoint(&mut self, _checkpoint: &ReusedTraceLocalCheckpoint) {}

    fn on_checkpoint_stage_started(&mut self, _event: &SubvolumeCheckpointStageStartedEvent) {}

    fn on_checkpoint_progress(&mut self, _stage_label: &str, _completed: usize, _total: usize) {}

    fn on_execution_summary(&mut self, _summary: &ProcessingJobExecutionSummary) {}

    fn on_checkpoint_artifact_emitted(
        &mut self,
        _artifact: &ProcessingJobArtifact,
        _lineage_pipeline: &TraceLocalProcessingPipeline,
    ) {
    }

    fn on_checkpoint_stage_completed(&mut self, _event: &SubvolumeCheckpointStageCompletedEvent) {}

    fn on_final_stage_started(&mut self, _event: &SubvolumeFinalStageStartedEvent) {}

    fn on_final_progress(&mut self, _completed: usize, _total: usize) {}
}

pub fn trace_local_pipeline_prefix(
    pipeline: &TraceLocalProcessingPipeline,
    end_operation_index: usize,
) -> TraceLocalProcessingPipeline {
    clone_pipeline_with_steps(pipeline, pipeline.steps[..=end_operation_index].to_vec())
}

pub fn trace_local_pipeline_segment(
    pipeline: &TraceLocalProcessingPipeline,
    start_operation_index: usize,
    end_operation_index: usize,
) -> TraceLocalProcessingPipeline {
    clone_pipeline_with_steps(
        pipeline,
        pipeline.steps[start_operation_index..=end_operation_index].to_vec(),
    )
}

pub fn resolve_trace_local_checkpoint_indexes(
    pipeline: &TraceLocalProcessingPipeline,
    allow_final_checkpoint: bool,
) -> Result<Vec<usize>, String> {
    if pipeline.steps.is_empty() {
        return Ok(Vec::new());
    }

    let last_index = pipeline.steps.len() - 1;
    let indexes = pipeline.checkpoint_indexes();

    for index in &indexes {
        if *index >= pipeline.steps.len() {
            return Err(format!(
                "Checkpoint index {index} is out of range for a pipeline with {} steps.",
                pipeline.steps.len()
            ));
        }
        if *index == last_index && !allow_final_checkpoint {
            return Err(
                "Checkpoint markers cannot target the final step because the final output is emitted automatically."
                    .to_string(),
            );
        }
    }

    Ok(indexes)
}

pub fn checkpoint_output_store_path(
    final_output_store_path: &str,
    job_id: &str,
    step_index: usize,
    operation: &TraceLocalProcessingOperation,
) -> String {
    let output_path = Path::new(final_output_store_path);
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("processed");
    let job_stem = sanitized_stem(job_id, "job");
    let operation_stem = sanitized_stem(operation.operator_id(), "step");
    let extension = output_path
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("tbvol");
    parent
        .join(format!(
            "{stem}.{job_stem}.step-{:02}-{operation_stem}.{extension}",
            step_index + 1
        ))
        .display()
        .to_string()
}

pub fn build_trace_local_processing_stages_from_plan(
    request: &RunTraceLocalProcessingRequest,
    plan: &ExecutionPlan,
    final_output_store_path: &str,
    job_id: &str,
    start_operation_index: usize,
) -> Result<Vec<TraceLocalProcessingStagePlan>, String> {
    if !matches!(
        plan.pipeline.family,
        crate::ProcessingPipelineFamily::TraceLocal
    ) {
        return Err("execution plan family does not match trace-local processing".to_string());
    }

    let mut stages = Vec::new();
    for stage in &plan.stages {
        if matches!(stage.stage_kind, ExecutionStageKind::ReuseArtifact)
            || stage
                .reuse_resolution
                .as_ref()
                .map(|resolution| resolution.reused)
                .unwrap_or(false)
        {
            continue;
        }
        let Some(segment) = stage.pipeline_segment.as_ref() else {
            continue;
        };
        if !matches!(segment.family, crate::ProcessingPipelineFamily::TraceLocal) {
            continue;
        }
        if segment.end_step_index < start_operation_index {
            continue;
        }

        let segment_start = segment.start_step_index.max(start_operation_index);
        let end_index = segment.end_step_index;
        let operation = request
            .pipeline
            .steps
            .get(end_index)
            .map(|step| &step.operation)
            .ok_or_else(|| format!("Missing operation at stage end index {end_index}"))?;
        let stage_label = format!(
            "Step {}: {}",
            end_index + 1,
            processing_operation_display_label(operation)
        );
        let artifact = ProcessingJobArtifact {
            kind: if matches!(stage.stage_kind, ExecutionStageKind::FinalizeOutput) {
                ProcessingJobArtifactKind::FinalOutput
            } else {
                ProcessingJobArtifactKind::Checkpoint
            },
            step_index: end_index,
            label: stage_label.clone(),
            store_path: if matches!(stage.stage_kind, ExecutionStageKind::FinalizeOutput) {
                final_output_store_path.to_string()
            } else {
                checkpoint_output_store_path(final_output_store_path, job_id, end_index, operation)
            },
        };
        stages.push(TraceLocalProcessingStagePlan {
            segment_pipeline: trace_local_pipeline_segment(
                &request.pipeline,
                segment_start,
                end_index,
            ),
            lineage_pipeline: trace_local_pipeline_prefix(&request.pipeline, end_index),
            stage_label,
            artifact,
            partition_target_bytes: stage.partition_spec.target_bytes,
            planned_lineage: planned_lineage_seed_for_stage(plan, &stage.stage_id),
        });
    }

    Ok(stages)
}

pub fn build_trace_local_checkpoint_stages_from_pipeline(
    pipeline: &TraceLocalProcessingPipeline,
    final_output_store_path: &str,
    job_id: &str,
    start_operation_index: usize,
    allow_final_checkpoint: bool,
) -> Result<Vec<TraceLocalProcessingStagePlan>, String> {
    let stage_end_indexes =
        resolve_trace_local_checkpoint_indexes(pipeline, allow_final_checkpoint)?
            .into_iter()
            .filter(|index| *index >= start_operation_index)
            .collect::<Vec<_>>();

    let mut stages = Vec::with_capacity(stage_end_indexes.len());
    let mut segment_start = start_operation_index;
    for end_index in stage_end_indexes {
        let operation = pipeline
            .steps
            .get(end_index)
            .map(|step| &step.operation)
            .ok_or_else(|| format!("Missing operation at checkpoint index {end_index}"))?;
        let stage_label = format!(
            "Step {}: {}",
            end_index + 1,
            processing_operation_display_label(operation)
        );
        let artifact = ProcessingJobArtifact {
            kind: ProcessingJobArtifactKind::Checkpoint,
            step_index: end_index,
            label: stage_label.clone(),
            store_path: checkpoint_output_store_path(
                final_output_store_path,
                job_id,
                end_index,
                operation,
            ),
        };
        stages.push(TraceLocalProcessingStagePlan {
            segment_pipeline: trace_local_pipeline_segment(pipeline, segment_start, end_index),
            lineage_pipeline: trace_local_pipeline_prefix(pipeline, end_index),
            stage_label,
            artifact,
            partition_target_bytes: None,
            planned_lineage: None,
        });
        segment_start = end_index + 1;
    }

    Ok(stages)
}

fn attach_planned_lineage_to_trace_local_stages(
    stages: &mut [TraceLocalProcessingStagePlan],
    plan: Option<&ExecutionPlan>,
    pipeline: &ProcessingPipelineSpec,
) {
    let Some(plan) = plan else {
        return;
    };
    for stage in stages {
        if stage.planned_lineage.is_none() {
            stage.planned_lineage =
                planned_lineage_seed_for_runtime_stage_label(plan, pipeline, &stage.stage_label);
        }
    }
}

pub fn resolve_reused_trace_local_checkpoint<F>(
    request: &RunTraceLocalProcessingRequest,
    allow_final_checkpoint: bool,
    mut lookup: F,
) -> Result<Option<ReusedTraceLocalCheckpoint>, String>
where
    F: FnMut(&TraceLocalCheckpointLookupKey) -> Result<Option<String>, String>,
{
    let checkpoint_indexes =
        resolve_trace_local_checkpoint_indexes(&request.pipeline, allow_final_checkpoint)?;
    if checkpoint_indexes.is_empty() {
        return Ok(None);
    }

    let loaded_source_identity = source_semantic_identity_with_status_from_store_path(
        &request.store_path,
        crate::SeismicLayout::PostStack3D,
    )?;
    let source_status = combine_canonical_identity_status(
        loaded_source_identity.status,
        pipeline_external_identity_status(&ProcessingPipelineSpec::TraceLocal {
            pipeline: request.pipeline.clone(),
        }),
    );
    if !matches!(source_status, CanonicalIdentityStatus::Canonical) {
        return Ok(None);
    }

    let source_fingerprint = source_identity_digest(&loaded_source_identity.identity)?;
    let source_handle = open_store(&request.store_path).map_err(|error| error.to_string())?;
    for checkpoint_index in checkpoint_indexes.into_iter().rev() {
        let lineage_pipeline = trace_local_pipeline_prefix(&request.pipeline, checkpoint_index);
        let prefix_hash = trace_local_pipeline_hash(&lineage_pipeline)?;
        let pipeline_spec = ProcessingPipelineSpec::TraceLocal {
            pipeline: lineage_pipeline.clone(),
        };
        let pipeline_identity = pipeline_semantic_identity(&pipeline_spec)?;
        let operator_set_identity = operator_set_identity_for_pipeline(&pipeline_spec)?;
        let planner_profile_identity = planner_profile_identity_for_pipeline(&pipeline_spec)?;
        let artifact_identity = canonical_artifact_identity(
            &loaded_source_identity.identity,
            source_status,
            &pipeline_identity,
            &operator_set_identity,
            &planner_profile_identity,
            crate::SeismicLayout::PostStack3D,
            source_handle.manifest.volume.shape,
            source_handle.manifest.tile_shape,
            ProcessingArtifactRole::Checkpoint,
            ArtifactBoundaryReason::AuthoredCheckpoint,
            MaterializationClass::Checkpoint,
            LogicalDomain::Volume {
                volume: crate::VolumeDomain {
                    shape: source_handle.manifest.volume.shape,
                },
            },
        )?
        .ok_or_else(|| "trace-local checkpoint identity is not canonical".to_string())?;
        let key = TraceLocalCheckpointLookupKey {
            source_fingerprint: source_fingerprint.clone(),
            prefix_hash,
            prefix_step_count: checkpoint_index + 1,
            checkpoint_index,
            artifact_key: artifact_identity.artifact_key.clone(),
            logical_domain: artifact_identity.logical_domain.clone(),
            chunk_grid_spec: artifact_identity.chunk_grid_spec.clone(),
            geometry_fingerprints: artifact_identity.geometry_fingerprints.clone(),
        };
        if let Some(path) = lookup(&key)? {
            let operation = request
                .pipeline
                .steps
                .get(checkpoint_index)
                .map(|step| &step.operation)
                .ok_or_else(|| {
                    format!("Missing operation at checkpoint index {checkpoint_index}")
                })?;
            let artifact = ProcessingJobArtifact {
                kind: ProcessingJobArtifactKind::Checkpoint,
                step_index: checkpoint_index,
                label: format!(
                    "Reused checkpoint after step {}: {}",
                    checkpoint_index + 1,
                    processing_operation_display_label(operation)
                ),
                store_path: path.clone(),
            };
            return Ok(Some(ReusedTraceLocalCheckpoint {
                after_operation_index: checkpoint_index,
                path,
                artifact,
            }));
        }
    }

    Ok(None)
}

pub fn trace_local_source_fingerprint(store_path: &str) -> Result<String, String> {
    let source_identity =
        source_semantic_identity_from_store_path(store_path, crate::SeismicLayout::PostStack3D)?;
    source_identity_digest(&source_identity)
}

pub fn trace_local_pipeline_hash(
    pipeline: &TraceLocalProcessingPipeline,
) -> Result<String, String> {
    pipeline_semantic_identity(&ProcessingPipelineSpec::TraceLocal {
        pipeline: pipeline.clone(),
    })
    .map(|identity| identity.content_digest)
}

#[allow(dead_code)]
fn seed_tbvol_processing_lineage(
    store_path: &str,
    seed: &PlannedArtifactLineageSeed,
) -> Result<(), String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let mut manifest: TbvolManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let lineage = manifest
        .volume
        .processing_lineage
        .as_mut()
        .ok_or_else(|| format!("Derived store is missing processing lineage: {store_path}"))?;
    if let Some(artifact_key) = seed.artifact_key.clone() {
        lineage.artifact_key = Some(artifact_key);
    }
    if !seed.input_artifact_keys.is_empty() {
        lineage.input_artifact_keys = seed.input_artifact_keys.clone();
    }
    if let Some(produced_by_stage_id) = seed.produced_by_stage_id.clone() {
        lineage.produced_by_stage_id = Some(produced_by_stage_id);
    }
    if let Some(boundary_reason) = seed.boundary_reason {
        lineage.boundary_reason = Some(boundary_reason);
    }
    if let Some(logical_domain) = seed.logical_domain.clone() {
        lineage.logical_domain = Some(logical_domain);
    }
    if let Some(chunk_grid_spec) = seed.chunk_grid_spec.clone() {
        lineage.chunk_grid_spec = Some(chunk_grid_spec);
    }
    if let Some(geometry_fingerprints) = seed.geometry_fingerprints.clone() {
        lineage.geometry_fingerprints = Some(geometry_fingerprints);
    }
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[allow(dead_code)]
fn seed_tbgath_processing_lineage(
    store_path: &str,
    seed: &PlannedArtifactLineageSeed,
) -> Result<(), String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let mut manifest: TbgathManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let lineage = manifest
        .volume
        .processing_lineage
        .as_mut()
        .ok_or_else(|| format!("Derived store is missing processing lineage: {store_path}"))?;
    if let Some(artifact_key) = seed.artifact_key.clone() {
        lineage.artifact_key = Some(artifact_key);
    }
    if !seed.input_artifact_keys.is_empty() {
        lineage.input_artifact_keys = seed.input_artifact_keys.clone();
    }
    if let Some(produced_by_stage_id) = seed.produced_by_stage_id.clone() {
        lineage.produced_by_stage_id = Some(produced_by_stage_id);
    }
    if let Some(boundary_reason) = seed.boundary_reason {
        lineage.boundary_reason = Some(boundary_reason);
    }
    if let Some(logical_domain) = seed.logical_domain.clone() {
        lineage.logical_domain = Some(logical_domain);
    }
    if let Some(chunk_grid_spec) = seed.chunk_grid_spec.clone() {
        lineage.chunk_grid_spec = Some(chunk_grid_spec);
    }
    if let Some(geometry_fingerprints) = seed.geometry_fingerprints.clone() {
        lineage.geometry_fingerprints = Some(geometry_fingerprints);
    }
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub fn rewrite_tbvol_processing_lineage(
    store_path: &str,
    pipeline: ProcessingPipelineSpec,
    artifact_role: ProcessingArtifactRole,
) -> Result<(), String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let mut manifest: TbvolManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let lineage = manifest
        .volume
        .processing_lineage
        .as_mut()
        .ok_or_else(|| format!("Derived store is missing processing lineage: {store_path}"))?;
    let pipeline_identity = pipeline_semantic_identity(&pipeline)?;
    let operator_set_identity = operator_set_identity_for_pipeline(&pipeline)?;
    let planner_profile_identity = planner_profile_identity_for_pipeline(&pipeline)?;
    let parent_store_path = lineage.parent_store.to_string_lossy().into_owned();
    let loaded_source_identity = source_semantic_identity_with_status_from_store_path(
        &parent_store_path,
        crate::SeismicLayout::PostStack3D,
    )?;
    let source_status = combine_canonical_identity_status(
        loaded_source_identity.status,
        pipeline_external_identity_status(&pipeline),
    );
    let source_identity = loaded_source_identity.identity;
    let logical_domain = LogicalDomain::Volume {
        volume: crate::VolumeDomain {
            shape: manifest.volume.shape,
        },
    };
    let boundary_reason = lineage.boundary_reason.unwrap_or(match artifact_role {
        ProcessingArtifactRole::Checkpoint => ArtifactBoundaryReason::AuthoredCheckpoint,
        ProcessingArtifactRole::FinalOutput => ArtifactBoundaryReason::FinalOutput,
    });
    let canonical_artifact = canonical_artifact_identity(
        &source_identity,
        source_status,
        &pipeline_identity,
        &operator_set_identity,
        &planner_profile_identity,
        source_identity.layout,
        manifest.volume.shape,
        manifest.tile_shape,
        artifact_role,
        boundary_reason,
        match artifact_role {
            ProcessingArtifactRole::Checkpoint => MaterializationClass::Checkpoint,
            ProcessingArtifactRole::FinalOutput => MaterializationClass::PublishedOutput,
        },
        logical_domain.clone(),
    )?;
    lineage.schema_version = 2;
    lineage.pipeline = pipeline;
    lineage.pipeline_identity = Some(pipeline_identity);
    lineage.operator_set_identity = Some(operator_set_identity);
    lineage.planner_profile_identity = Some(planner_profile_identity);
    lineage.source_identity = Some(source_identity);
    lineage.runtime_semantics_version = CURRENT_RUNTIME_SEMANTICS_VERSION.to_string();
    lineage.store_writer_semantics_version = CURRENT_STORE_WRITER_SEMANTICS_VERSION.to_string();
    lineage.artifact_role = artifact_role;
    lineage.boundary_reason = Some(boundary_reason);
    lineage.artifact_key = canonical_artifact
        .as_ref()
        .map(|identity| identity.artifact_key.clone());
    lineage.logical_domain = canonical_artifact
        .as_ref()
        .map(|identity| identity.logical_domain.clone());
    lineage.chunk_grid_spec = canonical_artifact
        .as_ref()
        .map(|identity| identity.chunk_grid_spec.clone());
    lineage.geometry_fingerprints = canonical_artifact
        .as_ref()
        .map(|identity| identity.geometry_fingerprints.clone());
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub fn rewrite_tbgath_processing_lineage(
    store_path: &str,
    pipeline: ProcessingPipelineSpec,
    artifact_role: ProcessingArtifactRole,
) -> Result<(), String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let mut manifest: TbgathManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let lineage = manifest
        .volume
        .processing_lineage
        .as_mut()
        .ok_or_else(|| format!("Derived store is missing processing lineage: {store_path}"))?;
    let pipeline_identity = pipeline_semantic_identity(&pipeline)?;
    let operator_set_identity = operator_set_identity_for_pipeline(&pipeline)?;
    let planner_profile_identity = planner_profile_identity_for_pipeline(&pipeline)?;
    let parent_store_path = lineage.parent_store.to_string_lossy().into_owned();
    let loaded_source_identity =
        source_semantic_identity_with_status_from_store_path(&parent_store_path, manifest.layout)?;
    let source_status = combine_canonical_identity_status(
        loaded_source_identity.status,
        pipeline_external_identity_status(&pipeline),
    );
    let source_identity = loaded_source_identity.identity;
    let logical_domain = LogicalDomain::Volume {
        volume: crate::VolumeDomain {
            shape: manifest.volume.shape,
        },
    };
    let boundary_reason = lineage.boundary_reason.unwrap_or(match artifact_role {
        ProcessingArtifactRole::Checkpoint => ArtifactBoundaryReason::AuthoredCheckpoint,
        ProcessingArtifactRole::FinalOutput => ArtifactBoundaryReason::FinalOutput,
    });
    let canonical_artifact = canonical_artifact_identity(
        &source_identity,
        source_status,
        &pipeline_identity,
        &operator_set_identity,
        &planner_profile_identity,
        manifest.layout,
        manifest.volume.shape,
        manifest.volume.shape,
        artifact_role,
        boundary_reason,
        match artifact_role {
            ProcessingArtifactRole::Checkpoint => MaterializationClass::Checkpoint,
            ProcessingArtifactRole::FinalOutput => MaterializationClass::PublishedOutput,
        },
        logical_domain.clone(),
    )?;
    lineage.schema_version = 2;
    lineage.pipeline_identity = Some(pipeline_identity);
    lineage.operator_set_identity = Some(operator_set_identity);
    lineage.planner_profile_identity = Some(planner_profile_identity);
    lineage.source_identity = Some(source_identity);
    lineage.runtime_semantics_version = CURRENT_RUNTIME_SEMANTICS_VERSION.to_string();
    lineage.store_writer_semantics_version = CURRENT_STORE_WRITER_SEMANTICS_VERSION.to_string();
    lineage.artifact_role = artifact_role;
    lineage.pipeline = pipeline;
    lineage.boundary_reason = Some(boundary_reason);
    lineage.artifact_key = canonical_artifact
        .as_ref()
        .map(|identity| identity.artifact_key.clone());
    lineage.logical_domain = canonical_artifact
        .as_ref()
        .map(|identity| identity.logical_domain.clone());
    lineage.chunk_grid_spec = canonical_artifact
        .as_ref()
        .map(|identity| identity.chunk_grid_spec.clone());
    lineage.geometry_fingerprints = canonical_artifact
        .as_ref()
        .map(|identity| identity.geometry_fingerprints.clone());
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn cleanup_temporary_processing_stores(paths: &[String]) {
    for path in paths {
        let _ = fs::remove_dir_all(path);
        let _ = fs::remove_file(path);
    }
}

pub fn execute_trace_local_processing_job(
    request: &RunTraceLocalProcessingRequest,
    execution_plan: &ExecutionPlan,
    output_store_path: &str,
    job_id: &str,
    overwrite_existing: bool,
    reused_checkpoint: Option<ReusedTraceLocalCheckpoint>,
    observer: &mut dyn TraceLocalExecutionObserver,
) -> Result<(), String> {
    let stages = build_trace_local_processing_stages_from_plan(
        request,
        execution_plan,
        output_store_path,
        job_id,
        reused_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.after_operation_index + 1)
            .unwrap_or(0),
    )?;
    observer.on_job_started(&TraceLocalJobStartedEvent {
        output_store_path: output_store_path.to_string(),
        initial_stage_label: stages.first().map(|stage| stage.stage_label.clone()),
        stage_count: stages.len(),
        operator_count: request.pipeline.operation_count(),
        reused_checkpoint: reused_checkpoint.is_some(),
    });

    prepare_processing_output_store(&request.store_path, output_store_path, overwrite_existing)?;

    if let Some(reused_checkpoint) = reused_checkpoint.as_ref() {
        observer.on_reused_checkpoint(reused_checkpoint);
    }

    let mut current_input_store_path = reused_checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.path.clone())
        .unwrap_or_else(|| request.store_path.clone());
    let observer = RefCell::new(observer);
    let execution_summary = RefCell::new(ProcessingExecutionSummaryState::default());

    for stage in &stages {
        if observer.borrow().is_cancelled() {
            return Err("processing cancelled".to_string());
        }
        let stage_started_at = Instant::now();
        observer
            .borrow_mut()
            .on_stage_started(&TraceLocalStageStartedEvent {
                stage: stage.clone(),
                input_store_path: current_input_store_path.clone(),
            });

        if !matches!(stage.artifact.kind, ProcessingJobArtifactKind::FinalOutput) {
            prepare_processing_output_store(
                &current_input_store_path,
                &stage.artifact.store_path,
                false,
            )?;
        }

        let chunk_shape = open_store(&current_input_store_path)
            .map_err(|error| error.to_string())?
            .manifest
            .tile_shape;
        let worker_count = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(4)
            .saturating_sub(1)
            .max(1);
        let materialize_resolution = resolve_trace_local_materialize_options(
            Some(execution_plan),
            chunk_shape,
            false,
            stage.partition_target_bytes,
            worker_count,
            None,
            1,
        );
        let resolved_chunk_plan = materialize_resolution.resolved_chunk_plan.clone();
        let materialize_options = materialize_resolution.options;
        let materialize_started_at = Instant::now();
        materialize_processing_volume_with_partition_progress(
            &current_input_store_path,
            &stage.artifact.store_path,
            &stage.segment_pipeline,
            materialize_options,
            |completed, total| {
                if observer.borrow().is_cancelled() {
                    return Err(crate::SeismicStoreError::Message(
                        "processing cancelled".to_string(),
                    ));
                }
                observer
                    .borrow_mut()
                    .on_stage_progress(&stage.stage_label, completed, total);
                Ok(())
            },
            |progress| {
                let mut summary = execution_summary.borrow_mut();
                summary.set_resolved_chunk_plan(resolved_chunk_plan.clone());
                summary.apply_partition_progress(&stage.stage_label, progress);
                let contract = summary.clone().into_contract();
                drop(summary);
                observer.borrow_mut().on_execution_summary(&contract);
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
        let materialize_duration_ms = materialize_started_at.elapsed().as_millis() as u64;
        let lineage_rewrite_started_at = Instant::now();
        if let Some(seed) = stage.planned_lineage.as_ref() {
            seed_tbvol_processing_lineage(&stage.artifact.store_path, seed)?;
        }
        rewrite_tbvol_processing_lineage(
            &stage.artifact.store_path,
            ProcessingPipelineSpec::TraceLocal {
                pipeline: stage.lineage_pipeline.clone(),
            },
            match stage.artifact.kind {
                ProcessingJobArtifactKind::Checkpoint => ProcessingArtifactRole::Checkpoint,
                ProcessingJobArtifactKind::FinalOutput => ProcessingArtifactRole::FinalOutput,
            },
        )?;
        let lineage_rewrite_duration_ms = lineage_rewrite_started_at.elapsed().as_millis() as u64;
        observer
            .borrow_mut()
            .on_artifact_emitted(&stage.artifact, &stage.lineage_pipeline);
        observer
            .borrow_mut()
            .on_stage_completed(&TraceLocalStageCompletedEvent {
                stage: stage.clone(),
                input_store_path: current_input_store_path.clone(),
                stage_duration_ms: stage_started_at.elapsed().as_millis() as u64,
                materialize_duration_ms,
                lineage_rewrite_duration_ms,
            });
        current_input_store_path = stage.artifact.store_path.clone();
    }

    Ok(())
}

pub fn execute_post_stack_neighborhood_processing_job(
    request: &RunPostStackNeighborhoodProcessingRequest,
    execution_plan: Option<&ExecutionPlan>,
    output_store_path: &str,
    job_id: &str,
    overwrite_existing: bool,
    observer: &mut dyn PostStackNeighborhoodExecutionObserver,
) -> Result<(), String> {
    let final_stage_label = post_stack_neighborhood_progress_label(&request.pipeline).to_string();
    let mut prefix_stages = request
        .pipeline
        .trace_local_pipeline
        .as_ref()
        .map(|pipeline| {
            build_trace_local_checkpoint_stages_from_pipeline(
                pipeline,
                output_store_path,
                job_id,
                0,
                true,
            )
        })
        .transpose()?
        .unwrap_or_default();
    attach_planned_lineage_to_trace_local_stages(
        &mut prefix_stages,
        execution_plan,
        &ProcessingPipelineSpec::PostStackNeighborhood {
            pipeline: request.pipeline.clone(),
        },
    );
    let initial_stage_label = prefix_stages
        .first()
        .map(|stage| stage.stage_label.clone())
        .unwrap_or_else(|| final_stage_label.clone());
    observer.on_job_started(&PostStackNeighborhoodJobStartedEvent {
        output_store_path: output_store_path.to_string(),
        initial_stage_label,
        stage_count: prefix_stages.len() + 1,
        operator_count: request.pipeline.operations.len(),
    });

    let mut cleanup_paths = Vec::new();
    let result = (|| -> Result<(), String> {
        let mut current_input_store_path = request.store_path.clone();
        for stage in &prefix_stages {
            if observer.is_cancelled() {
                return Err("processing cancelled".to_string());
            }
            observer.on_stage_started(&stage.stage_label);
            prepare_processing_output_store(
                &current_input_store_path,
                &stage.artifact.store_path,
                false,
            )?;
            let materialize_options = materialize_options_for_store(
                &current_input_store_path,
                stage.partition_target_bytes,
            )?;
            materialize_processing_volume_with_partition_progress(
                &current_input_store_path,
                &stage.artifact.store_path,
                &stage.segment_pipeline,
                materialize_options,
                |completed, total| {
                    if observer.is_cancelled() {
                        return Err(crate::SeismicStoreError::Message(
                            "processing cancelled".to_string(),
                        ));
                    }
                    observer.on_progress(&stage.stage_label, completed, total);
                    Ok(())
                },
                |_| Ok(()),
            )
            .map_err(|error| error.to_string())?;
            if let Some(seed) = stage.planned_lineage.as_ref() {
                seed_tbvol_processing_lineage(&stage.artifact.store_path, seed)?;
            }
            rewrite_tbvol_processing_lineage(
                &stage.artifact.store_path,
                ProcessingPipelineSpec::TraceLocal {
                    pipeline: stage.lineage_pipeline.clone(),
                },
                ProcessingArtifactRole::Checkpoint,
            )?;
            cleanup_paths.push(stage.artifact.store_path.clone());
            current_input_store_path = stage.artifact.store_path.clone();
        }

        if observer.is_cancelled() {
            return Err("processing cancelled".to_string());
        }
        observer.on_stage_started(&final_stage_label);
        prepare_processing_output_store(
            &current_input_store_path,
            output_store_path,
            overwrite_existing,
        )?;
        let materialize_options = materialize_options_for_store(&current_input_store_path, None)?;
        let execution_pipeline = PostStackNeighborhoodProcessingPipeline {
            trace_local_pipeline: None,
            ..request.pipeline.clone()
        };
        let mut on_progress = |completed, total| {
            if observer.is_cancelled() {
                return Err(crate::SeismicStoreError::Message(
                    "processing cancelled".to_string(),
                ));
            }
            observer.on_progress(&final_stage_label, completed, total);
            Ok(())
        };
        materialize_post_stack_neighborhood_without_prefix_with_progress(
            &current_input_store_path,
            output_store_path,
            &execution_pipeline,
            materialize_options,
            &mut on_progress,
        )
        .map_err(|error| error.to_string())?;
        if let Some(seed) = execution_plan.and_then(|plan| {
            planned_lineage_seed_for_runtime_stage_label(
                plan,
                &ProcessingPipelineSpec::PostStackNeighborhood {
                    pipeline: request.pipeline.clone(),
                },
                &final_stage_label,
            )
        }) {
            seed_tbvol_processing_lineage(output_store_path, &seed)?;
        }
        rewrite_tbvol_processing_lineage(
            output_store_path,
            ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: request.pipeline.clone(),
            },
            ProcessingArtifactRole::FinalOutput,
        )?;
        Ok(())
    })();

    cleanup_temporary_processing_stores(&cleanup_paths);
    result
}

pub fn execute_gather_processing_job(
    request: &RunGatherProcessingRequest,
    execution_plan: Option<&ExecutionPlan>,
    output_store_path: &str,
    job_id: &str,
    overwrite_existing: bool,
    observer: &mut dyn GatherExecutionObserver,
) -> Result<(), String> {
    let final_stage_label = gather_processing_progress_label(&request.pipeline);
    let mut prefix_stages = request
        .pipeline
        .trace_local_pipeline
        .as_ref()
        .map(|pipeline| {
            build_trace_local_checkpoint_stages_from_pipeline(
                pipeline,
                output_store_path,
                job_id,
                0,
                true,
            )
        })
        .transpose()?
        .unwrap_or_default();
    attach_planned_lineage_to_trace_local_stages(
        &mut prefix_stages,
        execution_plan,
        &ProcessingPipelineSpec::Gather {
            pipeline: request.pipeline.clone(),
        },
    );
    observer.on_job_started(&GatherJobStartedEvent {
        output_store_path: output_store_path.to_string(),
        initial_stage_label: prefix_stages
            .first()
            .map(|stage| stage.stage_label.clone())
            .unwrap_or_else(|| final_stage_label.clone()),
        stage_count: prefix_stages.len() + 1,
        operator_count: request.pipeline.operations.len(),
    });

    let mut cleanup_paths = Vec::new();
    let result = (|| -> Result<(), String> {
        let mut current_input_store_path = request.store_path.clone();
        for stage in &prefix_stages {
            if observer.is_cancelled() {
                return Err("processing cancelled".to_string());
            }
            observer.on_stage_started(&stage.stage_label);
            prepare_processing_output_store(
                &current_input_store_path,
                &stage.artifact.store_path,
                false,
            )?;
            materialize_trace_local_gather_processing_store_with_progress(
                &current_input_store_path,
                &stage.artifact.store_path,
                &stage.segment_pipeline,
                |completed, total| {
                    if observer.is_cancelled() {
                        return Err(crate::SeismicStoreError::Message(
                            "processing cancelled".to_string(),
                        ));
                    }
                    observer.on_progress(&stage.stage_label, completed, total);
                    Ok(())
                },
            )
            .map_err(|error| error.to_string())?;
            if let Some(seed) = stage.planned_lineage.as_ref() {
                seed_tbgath_processing_lineage(&stage.artifact.store_path, seed)?;
            }
            rewrite_tbgath_processing_lineage(
                &stage.artifact.store_path,
                ProcessingPipelineSpec::TraceLocal {
                    pipeline: stage.lineage_pipeline.clone(),
                },
                ProcessingArtifactRole::Checkpoint,
            )?;
            cleanup_paths.push(stage.artifact.store_path.clone());
            current_input_store_path = stage.artifact.store_path.clone();
        }

        if observer.is_cancelled() {
            return Err("processing cancelled".to_string());
        }
        observer.on_stage_started(&final_stage_label);
        prepare_processing_output_store(
            &current_input_store_path,
            output_store_path,
            overwrite_existing,
        )?;
        let execution_pipeline = ophiolite_seismic::GatherProcessingPipeline {
            trace_local_pipeline: None,
            ..request.pipeline.clone()
        };
        materialize_gather_processing_store_without_prefix_with_progress(
            &current_input_store_path,
            output_store_path,
            &execution_pipeline,
            |completed, total| {
                if observer.is_cancelled() {
                    return Err(crate::SeismicStoreError::Message(
                        "processing cancelled".to_string(),
                    ));
                }
                observer.on_progress(&final_stage_label, completed, total);
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
        if let Some(seed) = execution_plan.and_then(|plan| {
            planned_lineage_seed_for_runtime_stage_label(
                plan,
                &ProcessingPipelineSpec::Gather {
                    pipeline: request.pipeline.clone(),
                },
                &final_stage_label,
            )
        }) {
            seed_tbgath_processing_lineage(output_store_path, &seed)?;
        }
        rewrite_tbgath_processing_lineage(
            output_store_path,
            ProcessingPipelineSpec::Gather {
                pipeline: request.pipeline.clone(),
            },
            ProcessingArtifactRole::FinalOutput,
        )?;
        Ok(())
    })();

    cleanup_temporary_processing_stores(&cleanup_paths);
    result
}

pub fn execute_subvolume_processing_job(
    request: &RunSubvolumeProcessingRequest,
    execution_plan: Option<&ExecutionPlan>,
    output_store_path: &str,
    job_id: &str,
    overwrite_existing: bool,
    reused_checkpoint: Option<ReusedTraceLocalCheckpoint>,
    observer: &mut dyn SubvolumeExecutionObserver,
) -> Result<(), String> {
    let observer = RefCell::new(observer);
    let mut checkpoint_stages = match request.pipeline.trace_local_pipeline.as_ref() {
        Some(pipeline) => build_trace_local_checkpoint_stages_from_pipeline(
            pipeline,
            output_store_path,
            job_id,
            reused_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.after_operation_index + 1)
                .unwrap_or(0),
            true,
        )?,
        None => Vec::new(),
    };
    attach_planned_lineage_to_trace_local_stages(
        &mut checkpoint_stages,
        execution_plan,
        &ProcessingPipelineSpec::Subvolume {
            pipeline: request.pipeline.clone(),
        },
    );
    let initial_stage_label = checkpoint_stages
        .first()
        .map(|stage| stage.stage_label.clone())
        .unwrap_or_else(|| "Crop Subvolume".to_string());
    observer
        .borrow_mut()
        .on_job_started(&SubvolumeJobStartedEvent {
            output_store_path: output_store_path.to_string(),
            initial_stage_label,
            trace_local_operator_count: request
                .pipeline
                .trace_local_pipeline
                .as_ref()
                .map(|pipeline| pipeline.operation_count())
                .unwrap_or(0),
            checkpoint_count: checkpoint_stages.len(),
            reused_checkpoint: reused_checkpoint.is_some(),
        });
    prepare_processing_output_store(&request.store_path, output_store_path, overwrite_existing)?;
    let final_materialize_options = materialize_options_for_store(&request.store_path, None)?;

    if let Some(reused_checkpoint) = reused_checkpoint.as_ref() {
        observer
            .borrow_mut()
            .on_reused_checkpoint(reused_checkpoint);
    }

    let mut current_input_store_path = reused_checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.path.clone())
        .unwrap_or_else(|| request.store_path.clone());
    let execution_summary = RefCell::new(ProcessingExecutionSummaryState::default());

    for stage in &checkpoint_stages {
        if observer.borrow().is_cancelled() {
            return Err("processing cancelled".to_string());
        }
        let stage_started_at = Instant::now();
        observer
            .borrow_mut()
            .on_checkpoint_stage_started(&SubvolumeCheckpointStageStartedEvent {
                stage: stage.clone(),
                input_store_path: current_input_store_path.clone(),
            });
        prepare_processing_output_store(
            &current_input_store_path,
            &stage.artifact.store_path,
            false,
        )?;
        let stage_materialize_options =
            materialize_options_for_store(&current_input_store_path, None)?;
        let materialize_started_at = Instant::now();
        materialize_processing_volume_with_partition_progress(
            &current_input_store_path,
            &stage.artifact.store_path,
            &stage.segment_pipeline,
            stage_materialize_options,
            |completed, total| {
                if observer.borrow().is_cancelled() {
                    return Err(crate::SeismicStoreError::Message(
                        "processing cancelled".to_string(),
                    ));
                }
                observer
                    .borrow_mut()
                    .on_checkpoint_progress(&stage.stage_label, completed, total);
                Ok(())
            },
            |progress| {
                let mut summary = execution_summary.borrow_mut();
                summary.apply_partition_progress(&stage.stage_label, progress);
                let contract = summary.clone().into_contract();
                drop(summary);
                observer.borrow_mut().on_execution_summary(&contract);
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
        let materialize_duration_ms = materialize_started_at.elapsed().as_millis() as u64;
        let lineage_rewrite_started_at = Instant::now();
        if let Some(seed) = stage.planned_lineage.as_ref() {
            seed_tbvol_processing_lineage(&stage.artifact.store_path, seed)?;
        }
        rewrite_tbvol_processing_lineage(
            &stage.artifact.store_path,
            ProcessingPipelineSpec::TraceLocal {
                pipeline: stage.lineage_pipeline.clone(),
            },
            ProcessingArtifactRole::Checkpoint,
        )?;
        let lineage_rewrite_duration_ms = lineage_rewrite_started_at.elapsed().as_millis() as u64;
        observer
            .borrow_mut()
            .on_checkpoint_artifact_emitted(&stage.artifact, &stage.lineage_pipeline);
        observer.borrow_mut().on_checkpoint_stage_completed(
            &SubvolumeCheckpointStageCompletedEvent {
                stage: stage.clone(),
                stage_duration_ms: stage_started_at.elapsed().as_millis() as u64,
                materialize_duration_ms,
                lineage_rewrite_duration_ms,
            },
        );
        current_input_store_path = stage.artifact.store_path.clone();
    }

    let remaining_trace_local_pipeline =
        request
            .pipeline
            .trace_local_pipeline
            .as_ref()
            .and_then(|pipeline| {
                let start_index = checkpoint_stages
                    .last()
                    .map(|stage| stage.artifact.step_index + 1)
                    .or_else(|| {
                        reused_checkpoint
                            .as_ref()
                            .map(|checkpoint| checkpoint.after_operation_index + 1)
                    })
                    .unwrap_or(0);
                (start_index < pipeline.operation_count()).then(|| {
                    trace_local_pipeline_segment(
                        pipeline,
                        start_index,
                        pipeline.operation_count() - 1,
                    )
                })
            });
    let execution_pipeline = SubvolumeProcessingPipeline {
        schema_version: request.pipeline.schema_version,
        revision: request.pipeline.revision,
        preset_id: request.pipeline.preset_id.clone(),
        name: request.pipeline.name.clone(),
        description: request.pipeline.description.clone(),
        trace_local_pipeline: remaining_trace_local_pipeline,
        crop: request.pipeline.crop.clone(),
    };
    observer
        .borrow_mut()
        .on_final_stage_started(&SubvolumeFinalStageStartedEvent {
            input_store_path: current_input_store_path.clone(),
            output_store_path: output_store_path.to_string(),
        });
    materialize_subvolume_processing_volume_with_progress(
        &current_input_store_path,
        output_store_path,
        &execution_pipeline,
        final_materialize_options,
        |completed, total| {
            if observer.borrow().is_cancelled() {
                return Err(crate::SeismicStoreError::Message(
                    "processing cancelled".to_string(),
                ));
            }
            observer.borrow_mut().on_final_progress(completed, total);
            Ok(())
        },
    )
    .and_then(|_| {
        if let Some(seed) = execution_plan.and_then(|plan| {
            planned_lineage_seed_for_runtime_stage_label(
                plan,
                &ProcessingPipelineSpec::Subvolume {
                    pipeline: request.pipeline.clone(),
                },
                "Crop Subvolume",
            )
        }) {
            seed_tbvol_processing_lineage(output_store_path, &seed)
                .map_err(crate::SeismicStoreError::Message)?;
        }
        Ok(())
    })
    .map_err(|error| error.to_string())
}

fn clone_pipeline_with_steps(
    pipeline: &TraceLocalProcessingPipeline,
    steps: Vec<TraceLocalProcessingStep>,
) -> TraceLocalProcessingPipeline {
    TraceLocalProcessingPipeline {
        schema_version: pipeline.schema_version,
        revision: pipeline.revision,
        preset_id: pipeline.preset_id.clone(),
        name: pipeline.name.clone(),
        description: pipeline.description.clone(),
        steps,
    }
}

fn prepare_processing_output_store(
    input_store_path: &str,
    output_store_path: &str,
    overwrite_existing: bool,
) -> Result<(), String> {
    let input_path = Path::new(input_store_path);
    let output_path = Path::new(output_store_path);
    let input_canonical = input_path
        .canonicalize()
        .unwrap_or_else(|_| input_path.to_path_buf());
    let output_canonical = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.to_path_buf());
    if input_canonical == output_canonical {
        return Err("Output store path cannot overwrite the input store.".to_string());
    }
    if !output_path.exists() {
        return Ok(());
    }
    if !overwrite_existing {
        return Err(format!(
            "Output processing store already exists: {}",
            output_path.display()
        ));
    }
    let metadata = fs::symlink_metadata(output_path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(output_path).map_err(|error| error.to_string())?;
    } else {
        fs::remove_file(output_path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn runtime_stage_label_for_execution_stage(
    stage: &crate::execution::ExecutionStage,
    pipeline: &ProcessingPipelineSpec,
) -> String {
    if let Some(segment) = stage.pipeline_segment.as_ref() {
        match segment.family {
            crate::ProcessingPipelineFamily::TraceLocal => {
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
                            processing_operation_display_label(operation)
                        );
                    }
                }
            }
            crate::ProcessingPipelineFamily::PostStackNeighborhood => {
                if let ProcessingPipelineSpec::PostStackNeighborhood { pipeline } = pipeline {
                    return post_stack_neighborhood_progress_label(pipeline).to_string();
                }
            }
            crate::ProcessingPipelineFamily::Subvolume => {
                if matches!(stage.stage_kind, ExecutionStageKind::FinalizeOutput) {
                    return "Crop Subvolume".to_string();
                }
            }
            crate::ProcessingPipelineFamily::Gather => {
                if let ProcessingPipelineSpec::Gather { pipeline } = pipeline {
                    return gather_processing_progress_label(pipeline);
                }
            }
        }
    }

    match stage.stage_kind {
        ExecutionStageKind::FinalizeOutput => match pipeline {
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
                post_stack_neighborhood_progress_label(pipeline).to_string()
            }
            ProcessingPipelineSpec::Subvolume { .. } => "Crop Subvolume".to_string(),
            ProcessingPipelineSpec::Gather { pipeline } => {
                gather_processing_progress_label(pipeline)
            }
            ProcessingPipelineSpec::TraceLocal { pipeline } => pipeline
                .steps
                .last()
                .map(|step| {
                    format!(
                        "Step {}: {}",
                        pipeline.steps.len(),
                        processing_operation_display_label(&step.operation)
                    )
                })
                .unwrap_or_else(|| stage.output_artifact_id.clone()),
        },
        _ => stage.output_artifact_id.clone(),
    }
}

fn processing_operation_display_label(operation: &TraceLocalProcessingOperation) -> &'static str {
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

fn post_stack_neighborhood_progress_label(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> &'static str {
    match pipeline.operations.first() {
        Some(ophiolite_seismic::PostStackNeighborhoodProcessingOperation::Similarity {
            ..
        }) => "Similarity",
        Some(ophiolite_seismic::PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
            ..
        }) => "Local Volume Stats",
        Some(ophiolite_seismic::PostStackNeighborhoodProcessingOperation::Dip { .. }) => "Dip",
        None => "Neighborhood",
    }
}

fn gather_processing_progress_label(
    pipeline: &ophiolite_seismic::GatherProcessingPipeline,
) -> String {
    let operations = pipeline
        .operations
        .iter()
        .map(|operation| match operation {
            ophiolite_seismic::GatherProcessingOperation::NmoCorrection { .. } => "NMO Correction",
            ophiolite_seismic::GatherProcessingOperation::StretchMute { .. } => "Stretch Mute",
            ophiolite_seismic::GatherProcessingOperation::OffsetMute { .. } => "Offset Mute",
        })
        .collect::<Vec<_>>();
    if operations.is_empty() {
        "Gather Processing".to_string()
    } else {
        format!("Gather: {}", operations.join(" + "))
    }
}

fn materialize_options_for_store(
    input_store_path: &str,
    partition_target_bytes: Option<u64>,
) -> Result<MaterializeOptions, String> {
    let chunk_shape = open_store(input_store_path)
        .map_err(|error| error.to_string())?
        .manifest
        .tile_shape;
    Ok(MaterializeOptions {
        chunk_shape,
        partition_target_bytes,
        ..MaterializeOptions::default()
    })
}

fn sanitized_stem(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

pub struct ProcessingCacheFingerprint;

impl ProcessingCacheFingerprint {
    pub fn fingerprint_bytes(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    pub fn fingerprint_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
        let payload = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        Ok(Self::fingerprint_bytes(&payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_output_store_path_preserves_output_extension() {
        let gather_checkpoint = checkpoint_output_store_path(
            "C:\\temp\\derived-output.tbgath",
            "job-1",
            0,
            &TraceLocalProcessingOperation::Envelope,
        );
        let volume_checkpoint = checkpoint_output_store_path(
            "C:\\temp\\derived-output.tbvol",
            "job-1",
            1,
            &TraceLocalProcessingOperation::InstantaneousPhase,
        );

        assert!(gather_checkpoint.ends_with(".tbgath"));
        assert!(volume_checkpoint.ends_with(".tbvol"));
    }
}
