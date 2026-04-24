use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use ophiolite_seismic::contracts::{
    InspectableArtifactDerivation, InspectableArtifactKey, InspectableArtifactLifetimeClass,
    InspectableBoundaryReason, InspectableCacheMode, InspectableChunkGridSpec,
    InspectableChunkShapePolicy, InspectableCostClass, InspectableCostEstimate,
    InspectableDecisionFactor, InspectableExclusiveScope, InspectableExecutionArtifactRole,
    InspectableExecutionPipelineSegment, InspectableExecutionPlan, InspectableExecutionPlanSummary,
    InspectableExecutionPriorityClass, InspectableExecutionQueueClass, InspectableExecutionStage,
    InspectableExecutionStageKind, InspectableGeometryFingerprints, InspectableHaloSpec,
    InspectableLogicalDomain, InspectableMaterializationClass, InspectableParallelEfficiencyClass,
    InspectablePartitionFamily, InspectablePartitionOrdering, InspectablePartitionPlan,
    InspectablePlanDecision, InspectablePlanDecisionKind, InspectablePlanDecisionSubjectKind,
    InspectablePlanSource, InspectablePlannedArtifact, InspectablePlannerDiagnostics,
    InspectablePlannerPassId, InspectablePlannerPassSnapshot, InspectablePlanningMode,
    InspectableProcessingPlan, InspectableProgressGranularity, InspectableProgressUnits,
    InspectableRetryGranularity, InspectableRetryPolicy, InspectableReuseClass,
    InspectableReuseDecision, InspectableReuseDecisionEvidence, InspectableReuseDecisionOutcome,
    InspectableSchedulerHints, InspectableSemanticPlan, InspectableSemanticRootNode,
    InspectableSpillabilityClass, InspectableStageClassification, InspectableStageMemoryProfile,
    InspectableStagePlanningDecision, InspectableStageResourceEnvelope,
    InspectableTraceLocalSegment, InspectableTraceLocalSemanticPlan, InspectableValidationReport,
    ProcessingJobQueueClass, ProcessingJobRuntimeSnapshot, ProcessingJobRuntimeState,
    ProcessingJobWaitReason, ProcessingRuntimeEvent, ProcessingRuntimeEventDetails,
    ProcessingRuntimeEventKind, ProcessingRuntimePolicyDivergence,
    ProcessingRuntimePolicyDivergenceField, ProcessingRuntimeState, ProcessingStageRuntimeSnapshot,
};
use ophiolite_seismic::{
    ProcessingBatchItemRequest, ProcessingBatchItemStatus, ProcessingBatchProgress,
    ProcessingBatchState, ProcessingBatchStatus, ProcessingExecutionMode, ProcessingJobArtifact,
    ProcessingJobExecutionSummary, ProcessingJobPlanSummary, ProcessingJobProgress,
    ProcessingJobStageClassificationSummary, ProcessingJobState, ProcessingJobStatus,
    ProcessingPipelineFamily, ProcessingPipelineSpec, ProcessingSchedulerReason,
    TraceLocalProcessingPipeline,
};
use ophiolite_seismic_runtime::{
    ExecutionExclusiveScope, ExecutionPlan, ExecutionPriorityClass, ExecutionQueueClass,
    ExecutionStageKind, MemoryCostClass, operator_execution_traits_for_pipeline_spec,
};

pub struct ProcessingJobRecord {
    status: Mutex<ProcessingJobStatus>,
    plan: Option<ExecutionPlan>,
    cancel_requested: AtomicBool,
    runtime_state: Mutex<ProcessingJobRuntimeState>,
    runtime_events: Mutex<Vec<ProcessingRuntimeEvent>>,
    next_runtime_seq: AtomicU64,
}

pub struct BatchExecutionGate {
    max_active_jobs: usize,
    active_jobs: Mutex<usize>,
}

impl BatchExecutionGate {
    fn new(max_active_jobs: usize) -> Arc<Self> {
        Arc::new(Self {
            max_active_jobs: max_active_jobs.max(1),
            active_jobs: Mutex::new(0),
        })
    }

    fn try_acquire(self: &Arc<Self>) -> Option<BatchExecutionPermit> {
        let mut active_jobs = self
            .active_jobs
            .lock()
            .expect("batch execution gate mutex poisoned");
        if *active_jobs < self.max_active_jobs {
            *active_jobs += 1;
            Some(BatchExecutionPermit {
                gate: Arc::clone(self),
            })
        } else {
            None
        }
    }
}

struct BatchExecutionPermit {
    gate: Arc<BatchExecutionGate>,
}

impl Drop for BatchExecutionPermit {
    fn drop(&mut self) {
        let mut active_jobs = self
            .gate
            .active_jobs
            .lock()
            .expect("batch execution gate mutex poisoned");
        if *active_jobs > 0 {
            *active_jobs -= 1;
        }
    }
}

impl ProcessingJobRecord {
    pub fn new(status: ProcessingJobStatus, plan: Option<ExecutionPlan>) -> Self {
        let runtime_state = initial_runtime_state(&status);
        Self {
            status: Mutex::new(status),
            plan,
            cancel_requested: AtomicBool::new(false),
            runtime_state: Mutex::new(runtime_state),
            runtime_events: Mutex::new(Vec::new()),
            next_runtime_seq: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> ProcessingJobStatus {
        self.status
            .lock()
            .expect("processing job status mutex poisoned")
            .clone()
    }

    pub fn plan(&self) -> Option<&ExecutionPlan> {
        self.plan.as_ref()
    }

    pub fn debug_plan(&self) -> Option<InspectableProcessingPlan> {
        self.status
            .lock()
            .expect("processing job status mutex poisoned")
            .inspectable_plan
            .clone()
    }

    pub fn runtime_state(&self) -> ProcessingJobRuntimeState {
        self.runtime_state
            .lock()
            .expect("processing runtime state mutex poisoned")
            .clone()
    }

    pub fn runtime_events_after(&self, after_seq: Option<u64>) -> Vec<ProcessingRuntimeEvent> {
        let after_seq = after_seq.unwrap_or(0);
        self.runtime_events
            .lock()
            .expect("processing runtime events mutex poisoned")
            .iter()
            .filter(|event| event.seq > after_seq)
            .cloned()
            .collect()
    }

    pub fn mark_running(&self, current_stage_label: Option<String>) -> ProcessingJobStatus {
        let updated = self.update(|status| {
            status.state = ProcessingJobState::Running;
            status.current_stage_label = current_stage_label.clone();
            status.updated_at_unix_s = unix_timestamp_s();
        });
        self.with_runtime_state(|runtime| {
            runtime.state = ProcessingRuntimeState::Running;
            if let Some(snapshot) = runtime.snapshot.as_mut() {
                snapshot.wait_reason = ProcessingJobWaitReason::Running;
            }
        });
        self.push_runtime_event(
            None,
            current_stage_label,
            ProcessingRuntimeEventKind::JobStarted,
            Some(ProcessingRuntimeState::Running),
            ProcessingRuntimeEventDetails::None,
        );
        updated
    }

    pub fn mark_progress(
        &self,
        completed: usize,
        total: usize,
        current_stage_label: Option<&str>,
    ) -> ProcessingJobStatus {
        let stage_id = self.stage_id_for_label(current_stage_label);
        let updated = self.update(|status| {
            status.progress = ProcessingJobProgress { completed, total };
            status.current_stage_label = current_stage_label.map(str::to_string);
            status.updated_at_unix_s = unix_timestamp_s();
        });
        if let (Some(stage_id), Some(stage_label)) = (stage_id, current_stage_label) {
            self.update_stage_snapshot(
                &stage_id,
                stage_label,
                ProcessingRuntimeState::Running,
                None,
                None,
                true,
                false,
                0,
                None,
                None,
                Some(completed),
                Some(total),
                None,
            );
            self.push_runtime_event(
                Some(stage_id),
                Some(stage_label.to_string()),
                ProcessingRuntimeEventKind::StageProgress,
                Some(ProcessingRuntimeState::Running),
                ProcessingRuntimeEventDetails::Progress {
                    completed,
                    total,
                    retry_count: None,
                },
            );
        }
        updated
    }

    pub fn push_artifact(&self, artifact: ProcessingJobArtifact) -> ProcessingJobStatus {
        let current_stage = self.snapshot().current_stage_label;
        let stage_id = self.stage_id_for_label(current_stage.as_deref());
        let updated = self.update(|status| {
            status.artifacts.push(artifact.clone());
            status.updated_at_unix_s = unix_timestamp_s();
        });
        self.push_runtime_event(
            stage_id,
            current_stage,
            ProcessingRuntimeEventKind::ArtifactEmitted,
            None,
            ProcessingRuntimeEventDetails::ArtifactEmitted {
                artifact_id: format!(
                    "{}:{}",
                    match artifact.kind {
                        ophiolite_seismic::ProcessingJobArtifactKind::Checkpoint => "checkpoint",
                        ophiolite_seismic::ProcessingJobArtifactKind::FinalOutput => "final_output",
                    },
                    artifact.step_index
                ),
                artifact_label: artifact.label.clone(),
                artifact_kind: artifact.kind,
                artifact_store_path: artifact.store_path.clone(),
            },
        );
        updated
    }

    pub fn set_execution_summary(
        &self,
        execution_summary: ProcessingJobExecutionSummary,
    ) -> ProcessingJobStatus {
        self.update(|status| {
            status.execution_summary = Some(execution_summary.clone());
            status.updated_at_unix_s = unix_timestamp_s();
        })
    }

    pub fn set_runtime_snapshot(
        &self,
        runtime_snapshot: Option<ProcessingJobRuntimeSnapshot>,
    ) -> ProcessingJobStatus {
        let updated = self.update(|status| {
            status.runtime_snapshot = runtime_snapshot.clone();
            status.updated_at_unix_s = unix_timestamp_s();
        });
        self.with_runtime_state(|runtime| {
            runtime.snapshot = runtime_snapshot.clone();
            if let Some(runtime_snapshot) = runtime_snapshot.as_ref() {
                runtime.state = runtime_state_from_snapshot(runtime_snapshot);
            }
        });
        updated
    }

    pub fn mark_completed(&self, output_store_path: String) -> ProcessingJobStatus {
        let updated = self.update(|status| {
            status.state = ProcessingJobState::Completed;
            status.output_store_path = Some(output_store_path.clone());
            status.current_stage_label = None;
            if let Some(summary) = status.execution_summary.as_mut() {
                summary.active_partitions = 0;
            }
            status.runtime_snapshot = None;
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = None;
        });
        self.with_runtime_state(|runtime| {
            runtime.state = ProcessingRuntimeState::Completed;
            for snapshot in &mut runtime.stage_snapshots {
                if !matches!(
                    snapshot.state,
                    ProcessingRuntimeState::Completed
                        | ProcessingRuntimeState::Failed
                        | ProcessingRuntimeState::Cancelled
                ) {
                    snapshot.state = ProcessingRuntimeState::Completed;
                    snapshot.updated_at_unix_s = unix_timestamp_s();
                }
            }
        });
        self.push_runtime_event(
            None,
            None,
            ProcessingRuntimeEventKind::StageCompleted,
            Some(ProcessingRuntimeState::Completed),
            ProcessingRuntimeEventDetails::None,
        );
        updated
    }

    pub fn mark_failed(&self, message: String) -> ProcessingJobStatus {
        let current_stage = self.snapshot().current_stage_label;
        let stage_id = self.stage_id_for_label(current_stage.as_deref());
        let updated = self.update(|status| {
            status.state = ProcessingJobState::Failed;
            status.current_stage_label = None;
            if let Some(summary) = status.execution_summary.as_mut() {
                summary.active_partitions = 0;
            }
            status.runtime_snapshot = None;
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = Some(message.clone());
        });
        self.with_runtime_state(|runtime| {
            runtime.state = ProcessingRuntimeState::Failed;
        });
        self.push_runtime_event(
            stage_id,
            current_stage,
            ProcessingRuntimeEventKind::StageFailed,
            Some(ProcessingRuntimeState::Failed),
            ProcessingRuntimeEventDetails::None,
        );
        updated
    }

    pub fn mark_cancelled(&self) -> ProcessingJobStatus {
        let updated = self.update(|status| {
            status.state = ProcessingJobState::Cancelled;
            status.current_stage_label = None;
            if let Some(summary) = status.execution_summary.as_mut() {
                summary.active_partitions = 0;
            }
            status.runtime_snapshot = None;
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = None;
        });
        self.with_runtime_state(|runtime| {
            runtime.state = ProcessingRuntimeState::Cancelled;
            for snapshot in &mut runtime.stage_snapshots {
                if !matches!(
                    snapshot.state,
                    ProcessingRuntimeState::Completed
                        | ProcessingRuntimeState::Failed
                        | ProcessingRuntimeState::Cancelled
                ) {
                    snapshot.state = ProcessingRuntimeState::Cancelled;
                    snapshot.updated_at_unix_s = unix_timestamp_s();
                }
            }
        });
        self.push_runtime_event(
            None,
            None,
            ProcessingRuntimeEventKind::JobCancelled,
            Some(ProcessingRuntimeState::Cancelled),
            ProcessingRuntimeEventDetails::None,
        );
        updated
    }

    pub fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::Relaxed);
        self.push_runtime_event(
            None,
            None,
            ProcessingRuntimeEventKind::JobCancelRequested,
            Some(ProcessingRuntimeState::Cancelled),
            ProcessingRuntimeEventDetails::None,
        );
    }

    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested.load(Ordering::Relaxed)
    }

    fn update<F>(&self, mut update: F) -> ProcessingJobStatus
    where
        F: FnMut(&mut ProcessingJobStatus),
    {
        let mut status = self
            .status
            .lock()
            .expect("processing job status mutex poisoned");
        update(&mut status);
        status.clone()
    }

    fn with_runtime_state<F>(&self, mut update: F)
    where
        F: FnMut(&mut ProcessingJobRuntimeState),
    {
        let mut runtime = self
            .runtime_state
            .lock()
            .expect("processing runtime state mutex poisoned");
        update(&mut runtime);
        runtime.latest_event_seq = self.latest_runtime_seq();
    }

    fn latest_runtime_seq(&self) -> Option<u64> {
        let current = self.next_runtime_seq.load(Ordering::Relaxed);
        (current > 0).then_some(current)
    }

    fn push_runtime_event(
        &self,
        stage_id: Option<String>,
        stage_label: Option<String>,
        event_kind: ProcessingRuntimeEventKind,
        state: Option<ProcessingRuntimeState>,
        details: ProcessingRuntimeEventDetails,
    ) -> u64 {
        let seq = self.next_runtime_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let job_id = self.snapshot().job_id;
        let event = ProcessingRuntimeEvent {
            seq,
            job_id,
            stage_id,
            stage_label,
            event_kind,
            state,
            timestamp_unix_s: unix_timestamp_s(),
            details,
        };
        self.runtime_events
            .lock()
            .expect("processing runtime events mutex poisoned")
            .push(event);
        self.with_runtime_state(|runtime| {
            runtime.latest_event_seq = Some(seq);
        });
        seq
    }

    fn stage_id_for_label(&self, stage_label: Option<&str>) -> Option<String> {
        let runtime = self
            .runtime_state
            .lock()
            .expect("processing runtime state mutex poisoned");
        stage_label.and_then(|label| {
            runtime
                .stage_snapshots
                .iter()
                .find(|snapshot| snapshot.stage_label == label)
                .map(|snapshot| snapshot.stage_id.clone())
        })
    }

    fn update_stage_snapshot(
        &self,
        stage_id: &str,
        stage_label: &str,
        state: ProcessingRuntimeState,
        wait_reason: Option<ProcessingJobWaitReason>,
        queue_class: Option<ProcessingJobQueueClass>,
        admitted: bool,
        exclusive_scope_active: bool,
        reserved_memory_bytes: u64,
        effective_max_active_partitions: Option<usize>,
        started_at_unix_s: Option<u64>,
        completed_partitions: Option<usize>,
        total_partitions: Option<usize>,
        policy_divergences: Option<Vec<ProcessingRuntimePolicyDivergence>>,
    ) {
        let policy_divergences = policy_divergences.clone();
        self.with_runtime_state(|runtime| {
            if let Some(snapshot) = runtime
                .stage_snapshots
                .iter_mut()
                .find(|snapshot| snapshot.stage_id == stage_id)
            {
                snapshot.stage_label = stage_label.to_string();
                snapshot.state = state;
                snapshot.wait_reason = wait_reason;
                snapshot.queue_class = queue_class;
                snapshot.admitted = admitted;
                snapshot.exclusive_scope_active = exclusive_scope_active;
                snapshot.reserved_memory_bytes = reserved_memory_bytes;
                snapshot.effective_max_active_partitions = effective_max_active_partitions;
                if let Some(started_at_unix_s) = started_at_unix_s {
                    snapshot.started_at_unix_s = Some(started_at_unix_s);
                }
                snapshot.updated_at_unix_s = unix_timestamp_s();
                if let Some(completed_partitions) = completed_partitions {
                    snapshot.completed_partitions = Some(completed_partitions);
                }
                if let Some(total_partitions) = total_partitions {
                    snapshot.total_partitions = Some(total_partitions);
                }
                if let Some(policy_divergences) = policy_divergences.as_ref() {
                    snapshot.policy_divergences = policy_divergences.clone();
                }
            }
        });
    }

    pub fn set_scheduler_runtime(
        &self,
        queue_class: ProcessingJobQueueClass,
        wait_reason: ProcessingJobWaitReason,
        reserved_memory_bytes: u64,
        memory_budget_bytes: u64,
        effective_max_active_partitions: usize,
        admitted: bool,
        exclusive_scope_active: bool,
    ) {
        let snapshot = ProcessingJobRuntimeSnapshot {
            queue_class,
            wait_reason,
            reserved_memory_bytes,
            memory_budget_bytes,
            effective_max_active_partitions,
            admitted,
            exclusive_scope_active,
            policy_divergences: Vec::new(),
        };
        let mut runtime_snapshot = snapshot.clone();
        let stage_index = {
            let runtime = self
                .runtime_state
                .lock()
                .expect("processing runtime state mutex poisoned");
            runtime.stage_snapshots.iter().position(|snapshot| {
                !matches!(
                    snapshot.state,
                    ProcessingRuntimeState::Completed
                        | ProcessingRuntimeState::Failed
                        | ProcessingRuntimeState::Cancelled
                )
            })
        };
        if let Some(stage_index) = stage_index {
            let (stage_id, stage_label, policy_divergences) = {
                let runtime = self
                    .runtime_state
                    .lock()
                    .expect("processing runtime state mutex poisoned");
                let snapshot = &runtime.stage_snapshots[stage_index];
                (
                    snapshot.stage_id.clone(),
                    snapshot.stage_label.clone(),
                    self.runtime_policy_divergences_for_stage(
                        &snapshot.stage_id,
                        queue_class,
                        reserved_memory_bytes,
                        effective_max_active_partitions,
                        exclusive_scope_active,
                    ),
                )
            };
            runtime_snapshot.policy_divergences = policy_divergences.clone();
            let _ = self.set_runtime_snapshot(Some(runtime_snapshot.clone()));
            let stage_state = match wait_reason {
                ProcessingJobWaitReason::Queued => ProcessingRuntimeState::Queued,
                ProcessingJobWaitReason::AwaitingWorker => ProcessingRuntimeState::Waiting,
                ProcessingJobWaitReason::AwaitingMemory
                | ProcessingJobWaitReason::AwaitingBatchGate
                | ProcessingJobWaitReason::AwaitingExclusiveScope => {
                    ProcessingRuntimeState::Blocked
                }
                ProcessingJobWaitReason::Running => {
                    if admitted {
                        ProcessingRuntimeState::Admitted
                    } else {
                        ProcessingRuntimeState::Running
                    }
                }
            };
            self.update_stage_snapshot(
                &stage_id,
                &stage_label,
                stage_state,
                Some(wait_reason),
                Some(queue_class),
                admitted,
                exclusive_scope_active,
                reserved_memory_bytes,
                Some(effective_max_active_partitions),
                None,
                None,
                None,
                Some(policy_divergences),
            );
            self.push_runtime_event(
                Some(stage_id),
                Some(stage_label),
                match stage_state {
                    ProcessingRuntimeState::Queued => ProcessingRuntimeEventKind::StageQueued,
                    ProcessingRuntimeState::Waiting => ProcessingRuntimeEventKind::StageWaiting,
                    ProcessingRuntimeState::Admitted => ProcessingRuntimeEventKind::StageAdmitted,
                    ProcessingRuntimeState::Blocked => ProcessingRuntimeEventKind::StageBlocked,
                    ProcessingRuntimeState::Running => ProcessingRuntimeEventKind::StageRunning,
                    ProcessingRuntimeState::Completed => ProcessingRuntimeEventKind::StageCompleted,
                    ProcessingRuntimeState::Failed => ProcessingRuntimeEventKind::StageFailed,
                    ProcessingRuntimeState::Cancelled => ProcessingRuntimeEventKind::JobCancelled,
                },
                Some(stage_state),
                ProcessingRuntimeEventDetails::QueueState {
                    queue_class: Some(queue_class),
                    wait_reason: Some(wait_reason),
                    reserved_memory_bytes,
                    admitted,
                    exclusive_scope_active,
                    effective_max_active_partitions: Some(effective_max_active_partitions),
                },
            );
        } else {
            let _ = self.set_runtime_snapshot(Some(runtime_snapshot));
        }
    }

    pub fn set_stage_running(&self, stage_label: &str) {
        if let Some(stage_id) = self.stage_id_for_label(Some(stage_label)) {
            let planned_policy = self.plan.as_ref().and_then(|plan| {
                plan.stages
                    .iter()
                    .find(|stage| stage.stage_id == stage_id)
                    .map(|stage| stage.lowered_scheduler_policy.clone())
            });
            let started_at = unix_timestamp_s();
            self.update_stage_snapshot(
                &stage_id,
                stage_label,
                ProcessingRuntimeState::Running,
                Some(ProcessingJobWaitReason::Running),
                planned_policy
                    .as_ref()
                    .map(|policy| processing_job_queue_class(policy.queue_class)),
                true,
                planned_policy.as_ref().is_some_and(|policy| {
                    matches!(policy.exclusive_scope, ExecutionExclusiveScope::FullVolume)
                }),
                planned_policy
                    .as_ref()
                    .map(|policy| policy.reservation_bytes)
                    .unwrap_or(0),
                planned_policy
                    .as_ref()
                    .map(|policy| policy.effective_max_active_partitions),
                Some(started_at),
                None,
                None,
                None,
            );
            self.push_runtime_event(
                Some(stage_id),
                Some(stage_label.to_string()),
                ProcessingRuntimeEventKind::StageRunning,
                Some(ProcessingRuntimeState::Running),
                ProcessingRuntimeEventDetails::QueueState {
                    queue_class: planned_policy
                        .as_ref()
                        .map(|policy| processing_job_queue_class(policy.queue_class)),
                    wait_reason: Some(ProcessingJobWaitReason::Running),
                    reserved_memory_bytes: planned_policy
                        .as_ref()
                        .map(|policy| policy.reservation_bytes)
                        .unwrap_or(0),
                    admitted: true,
                    exclusive_scope_active: planned_policy.as_ref().is_some_and(|policy| {
                        matches!(policy.exclusive_scope, ExecutionExclusiveScope::FullVolume)
                    }),
                    effective_max_active_partitions: planned_policy
                        .as_ref()
                        .map(|policy| policy.effective_max_active_partitions),
                },
            );
        }
    }

    pub fn set_stage_completed(&self, stage_label: &str) {
        if let Some(stage_id) = self.stage_id_for_label(Some(stage_label)) {
            self.update_stage_snapshot(
                &stage_id,
                stage_label,
                ProcessingRuntimeState::Completed,
                None,
                None,
                false,
                false,
                0,
                None,
                None,
                None,
                None,
                None,
            );
            self.push_runtime_event(
                Some(stage_id),
                Some(stage_label.to_string()),
                ProcessingRuntimeEventKind::StageCompleted,
                Some(ProcessingRuntimeState::Completed),
                ProcessingRuntimeEventDetails::None,
            );
        }
    }
}

fn initial_runtime_state(status: &ProcessingJobStatus) -> ProcessingJobRuntimeState {
    let stage_snapshots = status
        .inspectable_plan
        .as_ref()
        .map(|plan| {
            plan.execution_plan
                .stages
                .iter()
                .map(|stage| ProcessingStageRuntimeSnapshot {
                    stage_id: stage.stage_id.clone(),
                    stage_label: stage.stage_label.clone(),
                    state: ProcessingRuntimeState::Queued,
                    wait_reason: Some(ProcessingJobWaitReason::Queued),
                    queue_class: Some(match stage.resource_envelope.preferred_queue_class {
                        InspectableExecutionQueueClass::Control => ProcessingJobQueueClass::Control,
                        InspectableExecutionQueueClass::InteractivePartition => {
                            ProcessingJobQueueClass::InteractivePartition
                        }
                        InspectableExecutionQueueClass::ForegroundPartition => {
                            ProcessingJobQueueClass::ForegroundPartition
                        }
                        InspectableExecutionQueueClass::BackgroundPartition => {
                            ProcessingJobQueueClass::BackgroundPartition
                        }
                        InspectableExecutionQueueClass::ExclusiveFullVolume => {
                            ProcessingJobQueueClass::ExclusiveFullVolume
                        }
                    }),
                    admitted: false,
                    exclusive_scope_active: false,
                    reserved_memory_bytes: 0,
                    effective_max_active_partitions: None,
                    attempt: 1,
                    started_at_unix_s: None,
                    updated_at_unix_s: status.created_at_unix_s,
                    completed_partitions: None,
                    total_partitions: stage
                        .expected_partition_count
                        .or(stage.resource_envelope.target_partition_count),
                    policy_divergences: Vec::new(),
                })
                .collect()
        })
        .unwrap_or_default();
    ProcessingJobRuntimeState {
        job_id: status.job_id.clone(),
        state: match status.state {
            ProcessingJobState::Queued => ProcessingRuntimeState::Queued,
            ProcessingJobState::Running => ProcessingRuntimeState::Running,
            ProcessingJobState::Completed => ProcessingRuntimeState::Completed,
            ProcessingJobState::Failed => ProcessingRuntimeState::Failed,
            ProcessingJobState::Cancelled => ProcessingRuntimeState::Cancelled,
        },
        snapshot: status.runtime_snapshot.clone(),
        stage_snapshots,
        latest_event_seq: None,
    }
}

fn runtime_state_from_snapshot(snapshot: &ProcessingJobRuntimeSnapshot) -> ProcessingRuntimeState {
    match snapshot.wait_reason {
        ProcessingJobWaitReason::Queued => ProcessingRuntimeState::Queued,
        ProcessingJobWaitReason::AwaitingWorker => ProcessingRuntimeState::Waiting,
        ProcessingJobWaitReason::AwaitingMemory
        | ProcessingJobWaitReason::AwaitingBatchGate
        | ProcessingJobWaitReason::AwaitingExclusiveScope => ProcessingRuntimeState::Blocked,
        ProcessingJobWaitReason::Running => {
            if snapshot.admitted {
                ProcessingRuntimeState::Admitted
            } else {
                ProcessingRuntimeState::Running
            }
        }
    }
}

impl ProcessingJobRecord {
    fn runtime_policy_divergences_for_stage(
        &self,
        stage_id: &str,
        actual_queue_class: ProcessingJobQueueClass,
        actual_reserved_memory_bytes: u64,
        actual_effective_max_active_partitions: usize,
        actual_exclusive_scope_active: bool,
    ) -> Vec<ProcessingRuntimePolicyDivergence> {
        let Some(plan) = self.plan.as_ref() else {
            return Vec::new();
        };
        let Some(stage) = plan.stages.iter().find(|stage| stage.stage_id == stage_id) else {
            return Vec::new();
        };

        let planned_queue_class =
            processing_job_queue_class(stage.lowered_scheduler_policy.queue_class);
        let planned_exclusive_scope_active = matches!(
            stage.lowered_scheduler_policy.exclusive_scope,
            ExecutionExclusiveScope::FullVolume
        );
        let planned_effective_max_active_partitions = stage
            .lowered_scheduler_policy
            .effective_max_active_partitions;
        let planned_reserved_memory_bytes = stage.lowered_scheduler_policy.reservation_bytes;

        let mut divergences = Vec::new();
        if planned_queue_class != actual_queue_class {
            divergences.push(ProcessingRuntimePolicyDivergence {
                field: ProcessingRuntimePolicyDivergenceField::QueueClass,
                planned_value: format!("{planned_queue_class:?}").to_ascii_lowercase(),
                actual_value: format!("{actual_queue_class:?}").to_ascii_lowercase(),
            });
        }
        if planned_exclusive_scope_active != actual_exclusive_scope_active {
            divergences.push(ProcessingRuntimePolicyDivergence {
                field: ProcessingRuntimePolicyDivergenceField::ExclusiveScope,
                planned_value: planned_exclusive_scope_active.to_string(),
                actual_value: actual_exclusive_scope_active.to_string(),
            });
        }
        if planned_reserved_memory_bytes != actual_reserved_memory_bytes {
            divergences.push(ProcessingRuntimePolicyDivergence {
                field: ProcessingRuntimePolicyDivergenceField::ReservedMemoryBytes,
                planned_value: planned_reserved_memory_bytes.to_string(),
                actual_value: actual_reserved_memory_bytes.to_string(),
            });
        }
        if planned_effective_max_active_partitions != actual_effective_max_active_partitions {
            divergences.push(ProcessingRuntimePolicyDivergence {
                field: ProcessingRuntimePolicyDivergenceField::EffectiveMaxActivePartitions,
                planned_value: planned_effective_max_active_partitions.to_string(),
                actual_value: actual_effective_max_active_partitions.to_string(),
            });
        }
        divergences
    }
}

#[derive(Debug, Clone)]
struct BatchTrackedItem {
    store_path: String,
    output_store_path: Option<String>,
    job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchExecutionPolicyDecision {
    pub requested_max_active_jobs: Option<usize>,
    pub effective_max_active_jobs: usize,
    pub execution_mode: ProcessingExecutionMode,
    pub scheduler_reason: ProcessingSchedulerReason,
    pub worker_budget: usize,
    pub global_cap: usize,
    pub max_memory_cost_class: MemoryCostClass,
    pub max_estimated_peak_memory_bytes: u64,
    pub max_expected_partition_count: Option<usize>,
}

struct ProcessingBatchRecord {
    batch_id: String,
    pipeline: ProcessingPipelineSpec,
    items: Vec<BatchTrackedItem>,
    requested_max_active_jobs: Option<usize>,
    effective_max_active_jobs: usize,
    execution_mode: ProcessingExecutionMode,
    scheduler_reason: ProcessingSchedulerReason,
    created_at_unix_s: u64,
}

impl ProcessingBatchRecord {
    fn snapshot_with_jobs(
        &self,
        jobs: &HashMap<String, Arc<ProcessingJobRecord>>,
    ) -> ProcessingBatchStatus {
        let mut item_statuses = Vec::with_capacity(self.items.len());
        let mut completed_jobs = 0usize;
        let mut any_running = false;
        let mut any_queued = false;
        let mut any_failed = false;
        let mut all_cancelled = !self.items.is_empty();
        let mut latest_updated = self.created_at_unix_s;

        for item in &self.items {
            let snapshot = jobs.get(&item.job_id).map(|record| record.snapshot());
            let (state, error_message, updated_at_unix_s) = match snapshot {
                Some(status) => {
                    latest_updated = latest_updated.max(status.updated_at_unix_s);
                    match status.state {
                        ProcessingJobState::Completed
                        | ProcessingJobState::Failed
                        | ProcessingJobState::Cancelled => {
                            completed_jobs += 1;
                        }
                        ProcessingJobState::Running => any_running = true,
                        ProcessingJobState::Queued => any_queued = true,
                    }
                    if matches!(status.state, ProcessingJobState::Failed) {
                        any_failed = true;
                    }
                    if !matches!(status.state, ProcessingJobState::Cancelled) {
                        all_cancelled = false;
                    }
                    (status.state, status.error_message, status.updated_at_unix_s)
                }
                None => {
                    any_failed = true;
                    all_cancelled = false;
                    (
                        ProcessingJobState::Failed,
                        Some("job record missing".to_string()),
                        self.created_at_unix_s,
                    )
                }
            };

            latest_updated = latest_updated.max(updated_at_unix_s);
            item_statuses.push(ProcessingBatchItemStatus {
                store_path: item.store_path.clone(),
                output_store_path: item.output_store_path.clone(),
                job_id: item.job_id.clone(),
                state,
                error_message,
            });
        }

        let state = if self.items.is_empty() {
            ProcessingBatchState::Completed
        } else if all_cancelled {
            ProcessingBatchState::Cancelled
        } else if completed_jobs == self.items.len() {
            if any_failed {
                ProcessingBatchState::CompletedWithErrors
            } else {
                ProcessingBatchState::Completed
            }
        } else if any_running {
            ProcessingBatchState::Running
        } else if any_queued {
            ProcessingBatchState::Queued
        } else {
            ProcessingBatchState::Running
        };

        ProcessingBatchStatus {
            batch_id: self.batch_id.clone(),
            state,
            progress: ProcessingBatchProgress {
                completed_jobs,
                total_jobs: self.items.len(),
            },
            pipeline: self.pipeline.clone(),
            items: item_statuses,
            requested_max_active_jobs: self.requested_max_active_jobs,
            effective_max_active_jobs: self.effective_max_active_jobs,
            execution_mode: self.execution_mode,
            scheduler_reason: self.scheduler_reason,
            created_at_unix_s: self.created_at_unix_s,
            updated_at_unix_s: latest_updated,
        }
    }
}

type ScheduledJobFn = Box<dyn FnOnce(Arc<ProcessingJobRecord>) + Send + 'static>;

struct ScheduledTask {
    job_id: String,
    record: Arc<ProcessingJobRecord>,
    queue_class: ExecutionQueueClass,
    reserved_memory_bytes: u64,
    memory_budget_bytes: u64,
    effective_max_active_partitions: usize,
    exclusive_scope: ExecutionExclusiveScope,
    batch_gate: Option<Arc<BatchExecutionGate>>,
    task: Option<ScheduledJobFn>,
}

impl ScheduledTask {
    fn new<F>(
        job_id: String,
        record: Arc<ProcessingJobRecord>,
        queue_class: ExecutionQueueClass,
        reserved_memory_bytes: u64,
        memory_budget_bytes: u64,
        effective_max_active_partitions: usize,
        exclusive_scope: ExecutionExclusiveScope,
        batch_gate: Option<Arc<BatchExecutionGate>>,
        task: F,
    ) -> Self
    where
        F: FnOnce(Arc<ProcessingJobRecord>) + Send + 'static,
    {
        Self {
            job_id,
            record,
            queue_class,
            reserved_memory_bytes,
            memory_budget_bytes,
            effective_max_active_partitions,
            exclusive_scope,
            batch_gate,
            task: Some(Box::new(task)),
        }
    }

    fn update_wait_reason(
        &self,
        wait_reason: ProcessingJobWaitReason,
        admitted: bool,
        exclusive_scope_active: bool,
    ) {
        self.record.set_scheduler_runtime(
            processing_job_queue_class(self.queue_class),
            wait_reason,
            if admitted {
                self.reserved_memory_bytes
            } else {
                0
            },
            self.memory_budget_bytes,
            self.effective_max_active_partitions,
            admitted,
            exclusive_scope_active,
        );
    }
}

struct AdmittedTask {
    job_id: String,
    record: Arc<ProcessingJobRecord>,
    reserved_memory_bytes: u64,
    exclusive_scope: ExecutionExclusiveScope,
    _batch_permit: Option<BatchExecutionPermit>,
    task: Option<ScheduledJobFn>,
}

impl AdmittedTask {
    fn run(mut self) {
        if self.record.cancel_requested()
            || matches!(self.record.snapshot().state, ProcessingJobState::Cancelled)
        {
            let _ = self.record.mark_cancelled();
            return;
        }
        let _ = self.record.mark_running(None);
        if let Some(task) = self.task.take() {
            task(Arc::clone(&self.record));
        }
    }
}

#[derive(Default)]
struct SchedulerQueues {
    control: VecDeque<ScheduledTask>,
    interactive_partition: VecDeque<ScheduledTask>,
    foreground_partition: VecDeque<ScheduledTask>,
    background_partition: VecDeque<ScheduledTask>,
    exclusive_full_volume: VecDeque<ScheduledTask>,
    ready: VecDeque<AdmittedTask>,
    active_workers: usize,
    reserved_memory_bytes: u64,
    exclusive_scope_active: bool,
    shutdown: bool,
}

impl SchedulerQueues {
    fn push_pending(&mut self, task: ScheduledTask) {
        match task.queue_class {
            ExecutionQueueClass::Control => self.control.push_back(task),
            ExecutionQueueClass::InteractivePartition => self.interactive_partition.push_back(task),
            ExecutionQueueClass::ForegroundPartition => self.foreground_partition.push_back(task),
            ExecutionQueueClass::BackgroundPartition => self.background_partition.push_back(task),
            ExecutionQueueClass::ExclusiveFullVolume => self.exclusive_full_volume.push_back(task),
        }
    }

    fn queue_mut(&mut self, queue_class: ExecutionQueueClass) -> &mut VecDeque<ScheduledTask> {
        match queue_class {
            ExecutionQueueClass::Control => &mut self.control,
            ExecutionQueueClass::InteractivePartition => &mut self.interactive_partition,
            ExecutionQueueClass::ForegroundPartition => &mut self.foreground_partition,
            ExecutionQueueClass::BackgroundPartition => &mut self.background_partition,
            ExecutionQueueClass::ExclusiveFullVolume => &mut self.exclusive_full_volume,
        }
    }
}

struct SharedScheduler {
    queues: Mutex<SchedulerQueues>,
    cv: Condvar,
    worker_capacity: usize,
}

pub struct ExecutionScheduler {
    shared: Arc<SharedScheduler>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl ExecutionScheduler {
    pub fn new(max_active_jobs: usize) -> Self {
        let worker_count = max_active_jobs.max(1);
        let shared = Arc::new(SharedScheduler {
            queues: Mutex::new(SchedulerQueues::default()),
            cv: Condvar::new(),
            worker_capacity: worker_count,
        });
        let mut workers = Vec::with_capacity(worker_count);
        for worker_index in 0..worker_count {
            let worker_shared = Arc::clone(&shared);
            workers.push(
                thread::Builder::new()
                    .name(format!("ophiolite-seismic-execution-{worker_index:02}"))
                    .spawn(move || worker_loop(worker_shared))
                    .expect("failed to spawn execution worker"),
            );
        }
        Self { shared, workers }
    }

    pub fn submit<F>(
        &self,
        job_id: String,
        record: Arc<ProcessingJobRecord>,
        queue_class: ExecutionQueueClass,
        reserved_memory_bytes: u64,
        memory_budget_bytes: u64,
        effective_max_active_partitions: usize,
        exclusive_scope: ExecutionExclusiveScope,
        batch_gate: Option<Arc<BatchExecutionGate>>,
        task: F,
    ) where
        F: FnOnce(Arc<ProcessingJobRecord>) + Send + 'static,
    {
        let task = ScheduledTask::new(
            job_id,
            record,
            queue_class,
            reserved_memory_bytes,
            memory_budget_bytes,
            effective_max_active_partitions,
            exclusive_scope,
            batch_gate,
            task,
        );
        let mut queues = self
            .shared
            .queues
            .lock()
            .expect("execution scheduler mutex poisoned");
        task.update_wait_reason(
            ProcessingJobWaitReason::Queued,
            false,
            queues.exclusive_scope_active,
        );
        queues.push_pending(task);
        promote_pending(&mut queues, self.shared.worker_capacity);
        self.shared.cv.notify_all();
    }

    pub fn cancel_pending(&self, job_id: &str) -> bool {
        let mut queues = self
            .shared
            .queues
            .lock()
            .expect("execution scheduler mutex poisoned");
        let removed_pending = remove_pending_by_job_id(&mut queues, job_id);
        let removed_ready = remove_ready_by_job_id(&mut queues, job_id);
        if removed_pending || removed_ready {
            promote_pending(&mut queues, self.shared.worker_capacity);
            self.shared.cv.notify_all();
        }
        removed_pending || removed_ready
    }
}

impl Drop for ExecutionScheduler {
    fn drop(&mut self) {
        {
            let mut queues = self
                .shared
                .queues
                .lock()
                .expect("execution scheduler mutex poisoned");
            queues.shutdown = true;
        }
        self.shared.cv.notify_all();
        while let Some(worker) = self.workers.pop() {
            let _ = worker.join();
        }
    }
}

fn worker_loop(shared: Arc<SharedScheduler>) {
    loop {
        let task = {
            let mut queues = shared
                .queues
                .lock()
                .expect("execution scheduler mutex poisoned");
            loop {
                if let Some(task) = queues.ready.pop_front() {
                    break task;
                }
                if queues.shutdown {
                    return;
                }
                queues = shared
                    .cv
                    .wait(queues)
                    .expect("execution scheduler mutex poisoned");
            }
        };
        let reserved_memory_bytes = task.reserved_memory_bytes;
        let exclusive_scope = task.exclusive_scope;
        task.run();
        let mut queues = shared
            .queues
            .lock()
            .expect("execution scheduler mutex poisoned");
        queues.active_workers = queues.active_workers.saturating_sub(1);
        queues.reserved_memory_bytes = queues
            .reserved_memory_bytes
            .saturating_sub(reserved_memory_bytes);
        if matches!(exclusive_scope, ExecutionExclusiveScope::FullVolume) {
            queues.exclusive_scope_active = false;
        }
        promote_pending(&mut queues, shared.worker_capacity);
        shared.cv.notify_all();
    }
}

fn promote_pending(queues: &mut SchedulerQueues, worker_capacity: usize) {
    loop {
        if queues.active_workers >= worker_capacity {
            break;
        }

        let mut promoted = false;
        for queue_class in [
            ExecutionQueueClass::Control,
            ExecutionQueueClass::InteractivePartition,
            ExecutionQueueClass::ForegroundPartition,
            ExecutionQueueClass::BackgroundPartition,
            ExecutionQueueClass::ExclusiveFullVolume,
        ] {
            let Some(task) = queues.queue_mut(queue_class).pop_front() else {
                continue;
            };

            if task.record.cancel_requested()
                || matches!(task.record.snapshot().state, ProcessingJobState::Cancelled)
            {
                let _ = task.record.mark_cancelled();
                promoted = true;
                break;
            }

            if let Some(wait_reason) = admission_wait_reason(queues, &task, worker_capacity) {
                task.update_wait_reason(wait_reason, false, queues.exclusive_scope_active);
                queues.queue_mut(queue_class).push_front(task);
                continue;
            }

            let batch_permit = match task
                .batch_gate
                .as_ref()
                .and_then(BatchExecutionGate::try_acquire)
            {
                Some(permit) => Some(permit),
                None if task.batch_gate.is_some() => {
                    task.update_wait_reason(
                        ProcessingJobWaitReason::AwaitingBatchGate,
                        false,
                        queues.exclusive_scope_active,
                    );
                    queues.queue_mut(queue_class).push_front(task);
                    continue;
                }
                None => None,
            };

            queues.active_workers += 1;
            queues.reserved_memory_bytes = queues
                .reserved_memory_bytes
                .saturating_add(task.reserved_memory_bytes);
            if matches!(task.exclusive_scope, ExecutionExclusiveScope::FullVolume) {
                queues.exclusive_scope_active = true;
            }
            task.update_wait_reason(
                ProcessingJobWaitReason::Running,
                true,
                queues.exclusive_scope_active,
            );
            queues.ready.push_back(AdmittedTask {
                job_id: task.job_id,
                record: task.record,
                reserved_memory_bytes: task.reserved_memory_bytes,
                exclusive_scope: task.exclusive_scope,
                _batch_permit: batch_permit,
                task: task.task,
            });
            promoted = true;
            break;
        }

        if !promoted {
            break;
        }
    }
}

fn admission_wait_reason(
    queues: &SchedulerQueues,
    task: &ScheduledTask,
    worker_capacity: usize,
) -> Option<ProcessingJobWaitReason> {
    if queues.active_workers >= worker_capacity {
        return Some(ProcessingJobWaitReason::AwaitingWorker);
    }
    if queues.exclusive_scope_active
        && !matches!(task.exclusive_scope, ExecutionExclusiveScope::FullVolume)
    {
        return Some(ProcessingJobWaitReason::AwaitingExclusiveScope);
    }
    if matches!(task.exclusive_scope, ExecutionExclusiveScope::FullVolume)
        && (queues.active_workers > 0 || queues.reserved_memory_bytes > 0)
    {
        return Some(ProcessingJobWaitReason::AwaitingExclusiveScope);
    }
    let fits_memory_budget = queues
        .reserved_memory_bytes
        .saturating_add(task.reserved_memory_bytes)
        <= task.memory_budget_bytes;
    if !fits_memory_budget && !(queues.active_workers == 0 && queues.reserved_memory_bytes == 0) {
        return Some(ProcessingJobWaitReason::AwaitingMemory);
    }
    None
}

fn remove_pending_by_job_id(queues: &mut SchedulerQueues, job_id: &str) -> bool {
    let mut removed = false;
    for queue_class in [
        ExecutionQueueClass::Control,
        ExecutionQueueClass::InteractivePartition,
        ExecutionQueueClass::ForegroundPartition,
        ExecutionQueueClass::BackgroundPartition,
        ExecutionQueueClass::ExclusiveFullVolume,
    ] {
        let queue = queues.queue_mut(queue_class);
        let initial_len = queue.len();
        queue.retain(|task| task.job_id != job_id);
        removed |= queue.len() != initial_len;
    }
    removed
}

fn remove_ready_by_job_id(queues: &mut SchedulerQueues, job_id: &str) -> bool {
    let mut removed = false;
    let mut remaining = VecDeque::with_capacity(queues.ready.len());
    while let Some(task) = queues.ready.pop_front() {
        if task.job_id == job_id {
            queues.active_workers = queues.active_workers.saturating_sub(1);
            queues.reserved_memory_bytes = queues
                .reserved_memory_bytes
                .saturating_sub(task.reserved_memory_bytes);
            if matches!(task.exclusive_scope, ExecutionExclusiveScope::FullVolume) {
                queues.exclusive_scope_active = false;
            }
            drop(task);
            removed = true;
        } else {
            remaining.push_back(task);
        }
    }
    queues.ready = remaining;
    removed
}

fn processing_job_queue_class(queue_class: ExecutionQueueClass) -> ProcessingJobQueueClass {
    match queue_class {
        ExecutionQueueClass::Control => ProcessingJobQueueClass::Control,
        ExecutionQueueClass::InteractivePartition => ProcessingJobQueueClass::InteractivePartition,
        ExecutionQueueClass::ForegroundPartition => ProcessingJobQueueClass::ForegroundPartition,
        ExecutionQueueClass::BackgroundPartition => ProcessingJobQueueClass::BackgroundPartition,
        ExecutionQueueClass::ExclusiveFullVolume => ProcessingJobQueueClass::ExclusiveFullVolume,
    }
}

fn queue_class_for_job(
    priority: ExecutionPriorityClass,
    plan: Option<&ExecutionPlan>,
) -> ExecutionQueueClass {
    if let Some(plan) = plan {
        return plan.runtime_environment.queue_class;
    }
    match priority {
        ExecutionPriorityClass::InteractivePreview => ExecutionQueueClass::InteractivePartition,
        ExecutionPriorityClass::ForegroundMaterialize => ExecutionQueueClass::ForegroundPartition,
        ExecutionPriorityClass::BackgroundBatch => ExecutionQueueClass::BackgroundPartition,
    }
}

fn effective_max_active_partitions_for_job(plan: Option<&ExecutionPlan>) -> usize {
    plan.and_then(|plan| {
        plan.stages
            .first()
            .map(|stage| {
                stage
                    .lowered_scheduler_policy
                    .effective_max_active_partitions
            })
            .or(Some(plan.runtime_environment.worker_budget))
    })
    .unwrap_or(1)
}

fn exclusive_scope_for_job(plan: Option<&ExecutionPlan>) -> ExecutionExclusiveScope {
    plan.map(|plan| plan.runtime_environment.exclusive_scope)
        .unwrap_or(ExecutionExclusiveScope::None)
}

fn reserved_memory_bytes_for_job(
    plan: Option<&ExecutionPlan>,
    pipeline: &ProcessingPipelineSpec,
) -> u64 {
    plan.map(|plan| {
        let plan_summary_bytes = plan.plan_summary.max_estimated_peak_memory_bytes;
        let stage_policy_bytes = plan
            .stages
            .iter()
            .map(|stage| stage.lowered_scheduler_policy.reservation_bytes)
            .max()
            .unwrap_or(0);
        plan.runtime_environment
            .memory_budget
            .reserve_bytes
            .max(plan_summary_bytes)
            .max(stage_policy_bytes)
    })
    .filter(|bytes| *bytes > 0)
    .unwrap_or_else(|| default_reserved_memory_bytes(pipeline))
}

fn default_reserved_memory_bytes(pipeline: &ProcessingPipelineSpec) -> u64 {
    let max_memory_cost_class = operator_execution_traits_for_pipeline_spec(pipeline)
        .into_iter()
        .map(|traits| traits.memory_cost_class)
        .max_by_key(|cost| memory_cost_class_rank(*cost))
        .unwrap_or(MemoryCostClass::Medium);
    match max_memory_cost_class {
        MemoryCostClass::Low => 128 * 1024 * 1024,
        MemoryCostClass::Medium => 512 * 1024 * 1024,
        MemoryCostClass::High => 1024 * 1024 * 1024,
    }
}

fn memory_cost_class_rank(cost: MemoryCostClass) -> usize {
    match cost {
        MemoryCostClass::Low => 0,
        MemoryCostClass::Medium => 1,
        MemoryCostClass::High => 2,
    }
}

pub struct ProcessingExecutionService {
    scheduler: ExecutionScheduler,
    scheduler_worker_count: usize,
    global_batch_cap: usize,
    memory_budget_bytes: u64,
    job_counter: AtomicU64,
    batch_counter: AtomicU64,
    jobs: Mutex<HashMap<String, Arc<ProcessingJobRecord>>>,
    batches: Mutex<HashMap<String, Arc<ProcessingBatchRecord>>>,
}

impl ProcessingExecutionService {
    pub fn new(max_active_jobs: usize) -> Self {
        let scheduler_worker_count = max_active_jobs.max(1);
        let global_batch_cap = global_batch_cap_from_env().unwrap_or(scheduler_worker_count);
        let memory_budget_bytes = processing_memory_budget_from_env()
            .unwrap_or(default_processing_memory_budget(scheduler_worker_count));
        Self {
            scheduler: ExecutionScheduler::new(scheduler_worker_count),
            scheduler_worker_count,
            global_batch_cap: global_batch_cap.max(1),
            memory_budget_bytes,
            job_counter: AtomicU64::new(0),
            batch_counter: AtomicU64::new(0),
            jobs: Mutex::new(HashMap::new()),
            batches: Mutex::new(HashMap::new()),
        }
    }

    pub fn enqueue_job<F>(
        &self,
        input_store_path: String,
        output_store_path: Option<String>,
        pipeline: ProcessingPipelineSpec,
        plan: Option<ExecutionPlan>,
        priority: ExecutionPriorityClass,
        batch_gate: Option<Arc<BatchExecutionGate>>,
        task: F,
    ) -> ProcessingJobStatus
    where
        F: FnOnce(Arc<ProcessingJobRecord>) + Send + 'static,
    {
        let status = self.register_job(input_store_path, output_store_path, pipeline, plan);
        let record = self
            .jobs
            .lock()
            .expect("processing jobs mutex poisoned")
            .get(&status.job_id)
            .cloned()
            .expect("processing job record should exist immediately after registration");
        let queue_class = queue_class_for_job(priority, record.plan());
        let reserved_memory_bytes = reserved_memory_bytes_for_job(record.plan(), &status.pipeline);
        let effective_max_active_partitions =
            effective_max_active_partitions_for_job(record.plan()).max(1);
        let exclusive_scope = exclusive_scope_for_job(record.plan());
        self.scheduler.submit(
            status.job_id.clone(),
            record,
            queue_class,
            reserved_memory_bytes,
            self.memory_budget_bytes,
            effective_max_active_partitions,
            exclusive_scope,
            batch_gate,
            task,
        );
        status
    }

    pub fn resolve_batch_execution_policy(
        &self,
        requested_max_active_jobs: Option<usize>,
        requested_execution_mode: Option<ProcessingExecutionMode>,
        pipeline: &ProcessingPipelineSpec,
        plan: Option<&ExecutionPlan>,
        priority: ExecutionPriorityClass,
    ) -> BatchExecutionPolicyDecision {
        resolve_batch_execution_policy(
            self.scheduler_worker_count,
            self.global_batch_cap,
            requested_max_active_jobs,
            requested_execution_mode,
            pipeline,
            plan,
            priority,
        )
    }

    pub fn resolve_batch_max_active_jobs(
        &self,
        requested_max_active_jobs: Option<usize>,
        pipeline: &ProcessingPipelineSpec,
        priority: ExecutionPriorityClass,
    ) -> usize {
        self.resolve_batch_execution_policy(
            requested_max_active_jobs,
            None,
            pipeline,
            None,
            priority,
        )
        .effective_max_active_jobs
    }

    pub fn create_batch_gate(&self, max_active_jobs: usize) -> Arc<BatchExecutionGate> {
        BatchExecutionGate::new(max_active_jobs.max(1))
    }

    pub fn register_job(
        &self,
        input_store_path: String,
        output_store_path: Option<String>,
        pipeline: ProcessingPipelineSpec,
        plan: Option<ExecutionPlan>,
    ) -> ProcessingJobStatus {
        let created_at_unix_s = unix_timestamp_s();
        let job_number = self.job_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let job_id = format!("processing-{created_at_unix_s}-{job_number:04}");
        let inspectable_plan = plan
            .as_ref()
            .map(|plan| inspectable_processing_plan(&pipeline, plan));
        let plan_summary = inspectable_plan
            .as_ref()
            .map(processing_job_plan_summary_from_inspectable_plan);
        let status = ProcessingJobStatus {
            job_id: job_id.clone(),
            state: ProcessingJobState::Queued,
            progress: ProcessingJobProgress {
                completed: 0,
                total: 0,
            },
            input_store_path,
            output_store_path,
            pipeline,
            current_stage_label: None,
            artifacts: Vec::new(),
            inspectable_plan,
            plan_summary,
            execution_summary: None,
            runtime_snapshot: None,
            created_at_unix_s,
            updated_at_unix_s: created_at_unix_s,
            error_message: None,
        };
        let record = Arc::new(ProcessingJobRecord::new(status.clone(), plan));
        self.jobs
            .lock()
            .expect("processing jobs mutex poisoned")
            .insert(job_id, record);
        status
    }

    pub fn enqueue_completed_job(
        &self,
        input_store_path: String,
        output_store_path: String,
        pipeline: ProcessingPipelineSpec,
        plan: Option<ExecutionPlan>,
        artifacts: Vec<ProcessingJobArtifact>,
    ) -> ProcessingJobStatus {
        let created_at_unix_s = unix_timestamp_s();
        let job_number = self.job_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let job_id = format!("processing-{created_at_unix_s}-{job_number:04}");
        let inspectable_plan = plan
            .as_ref()
            .map(|plan| inspectable_processing_plan(&pipeline, plan));
        let plan_summary = inspectable_plan
            .as_ref()
            .map(processing_job_plan_summary_from_inspectable_plan);
        let status = ProcessingJobStatus {
            job_id: job_id.clone(),
            state: ProcessingJobState::Completed,
            progress: ProcessingJobProgress {
                completed: 1,
                total: 1,
            },
            input_store_path,
            output_store_path: Some(output_store_path),
            pipeline,
            current_stage_label: None,
            artifacts,
            inspectable_plan,
            plan_summary,
            execution_summary: None,
            runtime_snapshot: None,
            created_at_unix_s,
            updated_at_unix_s: created_at_unix_s,
            error_message: None,
        };
        let record = Arc::new(ProcessingJobRecord::new(status.clone(), plan));
        self.jobs
            .lock()
            .expect("processing jobs mutex poisoned")
            .insert(job_id, record);
        status
    }

    pub fn job_record(&self, job_id: &str) -> Result<Arc<ProcessingJobRecord>, String> {
        self.jobs
            .lock()
            .expect("processing jobs mutex poisoned")
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("Unknown processing job: {job_id}"))
    }

    pub fn job_status(&self, job_id: &str) -> Result<ProcessingJobStatus, String> {
        Ok(self.job_record(job_id)?.snapshot())
    }

    pub fn job_debug_plan(
        &self,
        job_id: &str,
    ) -> Result<Option<InspectableProcessingPlan>, String> {
        Ok(self.job_record(job_id)?.debug_plan())
    }

    pub fn job_runtime_state(&self, job_id: &str) -> Result<ProcessingJobRuntimeState, String> {
        Ok(self.job_record(job_id)?.runtime_state())
    }

    pub fn job_runtime_events(
        &self,
        job_id: &str,
        after_seq: Option<u64>,
    ) -> Result<Vec<ProcessingRuntimeEvent>, String> {
        Ok(self.job_record(job_id)?.runtime_events_after(after_seq))
    }

    pub fn cancel_job(&self, job_id: &str) -> Result<ProcessingJobStatus, String> {
        let record = self.job_record(job_id)?;
        record.request_cancel();
        if self.scheduler.cancel_pending(job_id) {
            return Ok(record.mark_cancelled());
        }
        let current = record.snapshot();
        Ok(match current.state {
            ProcessingJobState::Queued => record.mark_cancelled(),
            _ => current,
        })
    }

    pub fn register_batch(
        &self,
        pipeline: ProcessingPipelineSpec,
        items: Vec<ProcessingBatchItemRequest>,
        job_ids: Vec<String>,
        policy: &BatchExecutionPolicyDecision,
    ) -> Result<ProcessingBatchStatus, String> {
        if items.len() != job_ids.len() {
            return Err("batch items and job ids must have the same length".to_string());
        }

        let created_at_unix_s = unix_timestamp_s();
        let batch_number = self.batch_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let batch_id = format!("processing-batch-{created_at_unix_s}-{batch_number:04}");
        let tracked_items = items
            .into_iter()
            .zip(job_ids)
            .map(|(item, job_id)| BatchTrackedItem {
                store_path: item.store_path,
                output_store_path: item.output_store_path,
                job_id,
            })
            .collect::<Vec<_>>();
        let record = Arc::new(ProcessingBatchRecord {
            batch_id: batch_id.clone(),
            pipeline,
            items: tracked_items,
            requested_max_active_jobs: policy.requested_max_active_jobs,
            effective_max_active_jobs: policy.effective_max_active_jobs.max(1),
            execution_mode: policy.execution_mode,
            scheduler_reason: policy.scheduler_reason,
            created_at_unix_s,
        });
        let snapshot = {
            let jobs = self.jobs.lock().expect("processing jobs mutex poisoned");
            record.snapshot_with_jobs(&jobs)
        };
        self.batches
            .lock()
            .expect("processing batches mutex poisoned")
            .insert(batch_id, record);
        Ok(snapshot)
    }

    pub fn batch_status(&self, batch_id: &str) -> Result<ProcessingBatchStatus, String> {
        let record = self
            .batches
            .lock()
            .expect("processing batches mutex poisoned")
            .get(batch_id)
            .cloned()
            .ok_or_else(|| format!("Unknown processing batch: {batch_id}"))?;
        let jobs = self.jobs.lock().expect("processing jobs mutex poisoned");
        Ok(record.snapshot_with_jobs(&jobs))
    }

    pub fn cancel_batch(&self, batch_id: &str) -> Result<ProcessingBatchStatus, String> {
        let record = self
            .batches
            .lock()
            .expect("processing batches mutex poisoned")
            .get(batch_id)
            .cloned()
            .ok_or_else(|| format!("Unknown processing batch: {batch_id}"))?;
        for item in &record.items {
            let _ = self.cancel_job(&item.job_id);
        }
        let jobs = self.jobs.lock().expect("processing jobs mutex poisoned");
        Ok(record.snapshot_with_jobs(&jobs))
    }
}

fn inspectable_processing_plan(
    pipeline: &ProcessingPipelineSpec,
    plan: &ExecutionPlan,
) -> InspectableProcessingPlan {
    InspectableProcessingPlan {
        schema_version: 1,
        plan_id: plan.plan_id.clone(),
        planning_mode: inspectable_planning_mode(plan.planning_mode),
        source: InspectablePlanSource {
            store_path: plan.source.store_path.clone(),
            layout: plan.source.layout,
            shape: plan.source.shape,
            chunk_shape: plan.source.chunk_shape,
        },
        source_identity: plan.source_identity.clone(),
        pipeline_identity: plan.pipeline_identity.clone(),
        operator_set_identity: plan.operator_set_identity.clone(),
        planner_profile_identity: plan.planner_profile_identity.clone(),
        semantic_plan: inspectable_semantic_plan(pipeline),
        execution_plan: inspectable_execution_plan(pipeline, plan),
        artifacts: inspectable_artifacts(plan),
        planner_diagnostics: InspectablePlannerDiagnostics {
            validation: InspectableValidationReport {
                plan_valid: plan.validation.plan_valid,
                warnings: plan.validation.warnings.clone(),
                blockers: plan.validation.blockers.clone(),
            },
            pass_snapshots: plan
                .planner_diagnostics
                .pass_snapshots
                .iter()
                .map(|snapshot| InspectablePlannerPassSnapshot {
                    pass_id: inspectable_planner_pass_id(snapshot.pass_id),
                    pass_name: snapshot.pass_name.clone(),
                    snapshot_text: snapshot.snapshot_text.clone(),
                    decision_ids: snapshot.decision_ids.clone(),
                })
                .collect(),
        },
        decisions: plan
            .plan_decisions
            .iter()
            .map(inspectable_plan_decision)
            .collect(),
    }
}

fn inspectable_semantic_plan(pipeline: &ProcessingPipelineSpec) -> InspectableSemanticPlan {
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => InspectableSemanticPlan {
            pipeline_family: ProcessingPipelineFamily::TraceLocal,
            pipeline_name: pipeline.name.clone(),
            pipeline_revision: pipeline.revision,
            authored_pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: pipeline.clone(),
            },
            root: InspectableSemanticRootNode::TraceLocal {
                trace_local: inspectable_trace_local_semantic_plan(pipeline),
            },
        },
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => InspectableSemanticPlan {
            pipeline_family: ProcessingPipelineFamily::PostStackNeighborhood,
            pipeline_name: pipeline.name.clone(),
            pipeline_revision: pipeline.revision,
            authored_pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: pipeline.clone(),
            },
            root: InspectableSemanticRootNode::PostStackNeighborhood {
                pipeline: pipeline.clone(),
                trace_local_prefix: pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(inspectable_trace_local_semantic_plan),
                operation_ids: pipeline
                    .operations
                    .iter()
                    .map(|operation| operation.operator_id().to_string())
                    .collect(),
                operation_count: pipeline.operations.len(),
            },
        },
        ProcessingPipelineSpec::Subvolume { pipeline } => InspectableSemanticPlan {
            pipeline_family: ProcessingPipelineFamily::Subvolume,
            pipeline_name: pipeline.name.clone(),
            pipeline_revision: pipeline.revision,
            authored_pipeline: ProcessingPipelineSpec::Subvolume {
                pipeline: pipeline.clone(),
            },
            root: InspectableSemanticRootNode::Subvolume {
                pipeline: pipeline.clone(),
                trace_local_prefix: pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(inspectable_trace_local_semantic_plan),
                crop_operator_id: "subvolume_crop".to_string(),
            },
        },
        ProcessingPipelineSpec::Gather { pipeline } => InspectableSemanticPlan {
            pipeline_family: ProcessingPipelineFamily::Gather,
            pipeline_name: pipeline.name.clone(),
            pipeline_revision: pipeline.revision,
            authored_pipeline: ProcessingPipelineSpec::Gather {
                pipeline: pipeline.clone(),
            },
            root: InspectableSemanticRootNode::Gather {
                pipeline: pipeline.clone(),
                trace_local_prefix: pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(inspectable_trace_local_semantic_plan),
                operation_ids: pipeline
                    .operations
                    .iter()
                    .map(|operation| operation.operator_id().to_string())
                    .collect(),
                operation_count: pipeline.operations.len(),
            },
        },
    }
}

fn inspectable_trace_local_semantic_plan(
    pipeline: &TraceLocalProcessingPipeline,
) -> InspectableTraceLocalSemanticPlan {
    let mut segments = Vec::new();
    let mut segment_start = 0usize;

    for (index, step) in pipeline.steps.iter().enumerate() {
        let is_final_step = index + 1 == pipeline.steps.len();
        if !step.checkpoint && !is_final_step {
            continue;
        }

        segments.push(InspectableTraceLocalSegment {
            start_step_index: segment_start,
            end_step_index: index,
            step_count: index + 1 - segment_start,
            boundary_reason: if is_final_step {
                InspectableBoundaryReason::FinalOutput
            } else {
                InspectableBoundaryReason::AuthoredCheckpoint
            },
            checkpoint: step.checkpoint,
            operator_ids: pipeline.steps[segment_start..=index]
                .iter()
                .map(|step| step.operation.operator_id().to_string())
                .collect(),
        });
        segment_start = index + 1;
    }

    InspectableTraceLocalSemanticPlan {
        pipeline: pipeline.clone(),
        segments,
    }
}

fn inspectable_execution_plan(
    pipeline: &ProcessingPipelineSpec,
    plan: &ExecutionPlan,
) -> InspectableExecutionPlan {
    InspectableExecutionPlan {
        stages: plan
            .stages
            .iter()
            .map(|stage| inspectable_execution_stage(pipeline, stage))
            .collect(),
        summary: InspectableExecutionPlanSummary {
            compute_stage_count: plan.plan_summary.compute_stage_count,
            max_memory_cost_class: inspectable_cost_class(plan.plan_summary.max_memory_cost_class),
            max_cpu_cost_class: inspectable_cpu_cost_class(plan.plan_summary.max_cpu_cost_class),
            max_io_cost_class: inspectable_io_cost_class(plan.plan_summary.max_io_cost_class),
            min_parallel_efficiency_class: inspectable_parallel_efficiency_class(
                plan.plan_summary.min_parallel_efficiency_class,
            ),
            max_relative_cpu_cost: plan.plan_summary.max_relative_cpu_cost,
            max_estimated_peak_memory_bytes: plan.plan_summary.max_estimated_peak_memory_bytes,
            combined_cpu_weight: plan.plan_summary.combined_cpu_weight,
            combined_io_weight: plan.plan_summary.combined_io_weight,
            max_expected_partition_count: plan.plan_summary.max_expected_partition_count,
            max_live_set_bytes: plan.plan_summary.max_live_set_bytes,
            max_live_artifact_count: plan.plan_summary.max_live_artifact_count,
        },
        scheduler_hints: InspectableSchedulerHints {
            priority_class: inspectable_priority_class(plan.scheduler_hints.priority_class),
            max_active_partitions: plan.scheduler_hints.max_active_partitions,
            expected_partition_count: plan.scheduler_hints.expected_partition_count,
        },
    }
}

fn inspectable_execution_stage(
    pipeline: &ProcessingPipelineSpec,
    stage: &ophiolite_seismic_runtime::ExecutionStage,
) -> InspectableExecutionStage {
    InspectableExecutionStage {
        stage_id: stage.stage_id.clone(),
        stage_kind: inspectable_stage_kind(stage.stage_kind),
        stage_label: if stage.stage_label.is_empty() {
            summarize_stage_label(pipeline, stage)
        } else {
            stage.stage_label.clone()
        },
        boundary_reason: boundary_reason_for_execution_stage(stage),
        input_artifact_ids: stage.input_artifact_ids.clone(),
        output_artifact_id: stage.output_artifact_id.clone(),
        pipeline_segment: stage.pipeline_segment.as_ref().map(|segment| {
            InspectableExecutionPipelineSegment {
                family: segment.family,
                start_step_index: segment.start_step_index,
                end_step_index: segment.end_step_index,
            }
        }),
        partition: InspectablePartitionPlan {
            family: inspectable_partition_family(stage.partition_spec.family),
            target_bytes: stage.partition_spec.target_bytes,
            target_partition_count: stage.partition_spec.target_partition_count,
            ordering: inspectable_partition_ordering(stage.partition_spec.ordering),
            requires_barrier: stage.partition_spec.requires_barrier,
        },
        expected_partition_count: stage.expected_partition_count,
        halo: InspectableHaloSpec {
            inline_radius: stage.halo_spec.inline_radius,
            xline_radius: stage.halo_spec.xline_radius,
        },
        chunk_shape_policy: inspectable_chunk_shape_policy(stage.chunk_shape_policy),
        cache_mode: inspectable_cache_mode(stage.cache_mode),
        retry_policy: InspectableRetryPolicy {
            max_attempts: stage.retry_policy.max_attempts,
        },
        progress_units: InspectableProgressUnits {
            total: stage.progress_units.total,
        },
        classification: InspectableStageClassification {
            max_memory_cost_class: inspectable_cost_class(
                stage.classification.max_memory_cost_class,
            ),
            max_cpu_cost_class: inspectable_cpu_cost_class(stage.classification.max_cpu_cost_class),
            max_io_cost_class: inspectable_io_cost_class(stage.classification.max_io_cost_class),
            min_parallel_efficiency_class: inspectable_parallel_efficiency_class(
                stage.classification.min_parallel_efficiency_class,
            ),
            combined_cpu_weight: stage.classification.combined_cpu_weight,
            combined_io_weight: stage.classification.combined_io_weight,
            uses_external_inputs: stage.classification.uses_external_inputs,
            requires_full_volume: stage.classification.requires_full_volume,
            has_sample_halo: stage.classification.has_sample_halo,
            has_spatial_halo: stage.classification.has_spatial_halo,
        },
        memory_cost_class: inspectable_cost_class(stage.memory_cost_class),
        estimated_cost: InspectableCostEstimate {
            relative_cpu_cost: stage.estimated_cost.relative_cpu_cost,
            estimated_peak_memory_bytes: stage.estimated_cost.estimated_peak_memory_bytes,
        },
        memory_profile: stage.stage_memory_profile.as_ref().map(|profile| {
            InspectableStageMemoryProfile {
                chunkability: match profile.chunkability {
                    ophiolite_seismic_runtime::Chunkability::TileSpan => "tile_span".to_string(),
                    ophiolite_seismic_runtime::Chunkability::FullVolumeOnly => {
                        "full_volume_only".to_string()
                    }
                },
                primary_tile_bytes: profile.primary_tile_bytes,
                secondary_input_count: profile.secondary_input_count,
                secondary_tile_bytes_per_input: profile.secondary_tile_bytes_per_input,
                output_tile_bytes: profile.output_tile_bytes,
                per_worker_workspace_bytes: profile.per_worker_workspace_bytes,
                shared_stage_bytes: profile.shared_stage_bytes,
                reserve_hint_bytes: profile.reserve_hint_bytes,
            }
        }),
        resource_envelope: InspectableStageResourceEnvelope {
            preferred_queue_class: inspectable_execution_queue_class(
                stage.resource_envelope.preferred_queue_class,
            ),
            spillability: inspectable_spillability_class(stage.resource_envelope.spillability),
            exclusive_scope: inspectable_exclusive_scope(stage.resource_envelope.exclusive_scope),
            retry_granularity: inspectable_retry_granularity(
                stage.resource_envelope.retry_granularity,
            ),
            progress_granularity: inspectable_progress_granularity(
                stage.resource_envelope.progress_granularity,
            ),
            min_partition_count: stage.resource_envelope.min_partition_count,
            target_partition_count: stage.resource_envelope.target_partition_count,
            max_partition_count: stage.resource_envelope.max_partition_count,
            preferred_partition_waves: stage.resource_envelope.preferred_partition_waves,
            resident_bytes_per_partition: stage.resource_envelope.resident_bytes_per_partition,
            workspace_bytes_per_worker: stage.resource_envelope.workspace_bytes_per_worker,
        },
        materialization_class: stage
            .materialization_class
            .map(inspectable_materialization_class),
        reuse_class: stage.reuse_class.map(inspectable_reuse_class),
        output_artifact_key: stage
            .output_artifact_key
            .as_ref()
            .map(inspectable_artifact_key),
        output_artifact_cache_key: stage
            .output_artifact_key
            .as_ref()
            .map(|artifact_key| artifact_key.cache_key.clone()),
        estimated_live_set_bytes: stage
            .live_set
            .as_ref()
            .map(|live_set| live_set.estimated_resident_bytes),
        reuse_requirement: stage.reuse_requirement.clone(),
        reuse_resolution: stage.reuse_resolution.clone(),
        planning_decision_id: stage.planning_decision_id.clone(),
        reuse_decision_id: stage.reuse_decision_id.clone(),
    }
}

fn inspectable_artifacts(plan: &ExecutionPlan) -> Vec<InspectablePlannedArtifact> {
    plan.artifacts
        .iter()
        .map(|artifact| InspectablePlannedArtifact {
            artifact_id: artifact.artifact_id.clone(),
            role: inspectable_artifact_role(artifact.role),
            store_path: artifact.store_path.clone(),
            cache_key: artifact.cache_key.clone(),
            materialization_class: artifact
                .materialization_class
                .map(inspectable_materialization_class),
            artifact_key: artifact.artifact_key.as_ref().map(inspectable_artifact_key),
            logical_domain: artifact
                .logical_domain
                .as_ref()
                .map(inspectable_logical_domain),
            chunk_grid_spec: artifact
                .chunk_grid_spec
                .as_ref()
                .map(inspectable_chunk_grid_spec),
            geometry_fingerprints: artifact
                .geometry_fingerprints
                .as_ref()
                .map(inspectable_geometry_fingerprints),
            boundary_reason: artifact.boundary_reason.map(inspectable_boundary_reason),
            lifetime_class: artifact
                .lifetime_class
                .map(inspectable_artifact_lifetime_class),
            produced_by_stage_id: plan
                .stages
                .iter()
                .find(|stage| stage.output_artifact_id == artifact.artifact_id)
                .map(|stage| stage.stage_id.clone()),
            consumed_by_stage_ids: plan
                .stages
                .iter()
                .filter(|stage| {
                    stage
                        .input_artifact_ids
                        .iter()
                        .any(|artifact_id| artifact_id == &artifact.artifact_id)
                })
                .map(|stage| stage.stage_id.clone())
                .collect(),
            reuse_requirement: artifact.reuse_requirement.clone(),
            reuse_resolution: artifact.reuse_resolution.clone(),
            reuse_decision_id: artifact.reuse_decision_id.clone(),
            artifact_derivation_decision_id: artifact.artifact_derivation_decision_id.clone(),
        })
        .collect()
}

fn processing_job_plan_summary_from_inspectable_plan(
    plan: &InspectableProcessingPlan,
) -> ProcessingJobPlanSummary {
    ProcessingJobPlanSummary {
        plan_id: plan.plan_id.clone(),
        planning_mode: format_planning_mode(plan.planning_mode),
        stage_count: plan.execution_plan.stages.len(),
        stage_labels: plan
            .execution_plan
            .stages
            .iter()
            .map(|stage| stage.stage_label.clone())
            .collect(),
        expected_partition_count: plan.execution_plan.scheduler_hints.expected_partition_count,
        max_active_partitions: plan.execution_plan.scheduler_hints.max_active_partitions,
        stage_partition_summaries: plan
            .execution_plan
            .stages
            .iter()
            .map(summarize_stage_partition)
            .collect(),
        max_memory_cost_class: format_cost_class(plan.execution_plan.summary.max_memory_cost_class),
        max_cpu_cost_class: format_cost_class(plan.execution_plan.summary.max_cpu_cost_class),
        max_io_cost_class: format_cost_class(plan.execution_plan.summary.max_io_cost_class),
        min_parallel_efficiency_class: format_parallel_efficiency_class(
            plan.execution_plan.summary.min_parallel_efficiency_class,
        ),
        combined_cpu_weight: plan.execution_plan.summary.combined_cpu_weight,
        combined_io_weight: plan.execution_plan.summary.combined_io_weight,
        stage_classification_summaries: plan
            .execution_plan
            .stages
            .iter()
            .map(processing_job_stage_classification_summary)
            .collect(),
    }
}

fn processing_job_stage_classification_summary(
    stage: &InspectableExecutionStage,
) -> ProcessingJobStageClassificationSummary {
    ProcessingJobStageClassificationSummary {
        stage_label: stage.stage_label.clone(),
        max_memory_cost_class: format_cost_class(stage.classification.max_memory_cost_class),
        max_cpu_cost_class: format_cost_class(stage.classification.max_cpu_cost_class),
        max_io_cost_class: format_cost_class(stage.classification.max_io_cost_class),
        min_parallel_efficiency_class: format_parallel_efficiency_class(
            stage.classification.min_parallel_efficiency_class,
        ),
        combined_cpu_weight: stage.classification.combined_cpu_weight,
        combined_io_weight: stage.classification.combined_io_weight,
        uses_external_inputs: stage.classification.uses_external_inputs,
        requires_full_volume: stage.classification.requires_full_volume,
        has_sample_halo: stage.classification.has_sample_halo,
        has_spatial_halo: stage.classification.has_spatial_halo,
    }
}

fn summarize_stage_label(
    _pipeline: &ProcessingPipelineSpec,
    stage: &ophiolite_seismic_runtime::ExecutionStage,
) -> String {
    if stage.stage_label.is_empty() {
        stage.output_artifact_id.clone()
    } else {
        stage.stage_label.clone()
    }
}

fn summarize_stage_partition(stage: &InspectableExecutionStage) -> String {
    let family = match stage.partition.family {
        InspectablePartitionFamily::TileGroup => "tile_group",
        InspectablePartitionFamily::Section => "section",
        InspectablePartitionFamily::GatherGroup => "gather_group",
        InspectablePartitionFamily::FullVolume => "full_volume",
    };
    let count = stage
        .expected_partition_count
        .map(|value| format!(" x{value}"))
        .unwrap_or_default();
    let target = stage
        .partition
        .target_bytes
        .map(|bytes| format!(" (~{} MiB target)", bytes / (1024 * 1024)))
        .unwrap_or_default();
    format!("{family}{count}{target}")
}

fn boundary_reason_for_execution_stage(
    stage: &ophiolite_seismic_runtime::ExecutionStage,
) -> Option<InspectableBoundaryReason> {
    if let Some(boundary_reason) = stage.boundary_reason {
        return Some(match boundary_reason {
            ophiolite_seismic_runtime::ArtifactBoundaryReason::SourceInput => {
                InspectableBoundaryReason::FamilyRoot
            }
            ophiolite_seismic_runtime::ArtifactBoundaryReason::AuthoredCheckpoint => {
                InspectableBoundaryReason::AuthoredCheckpoint
            }
            ophiolite_seismic_runtime::ArtifactBoundaryReason::FinalOutput => {
                InspectableBoundaryReason::FinalOutput
            }
            ophiolite_seismic_runtime::ArtifactBoundaryReason::GeometryDomainChange => {
                InspectableBoundaryReason::GeometryBoundary
            }
            ophiolite_seismic_runtime::ArtifactBoundaryReason::ExternalInputFanIn => {
                InspectableBoundaryReason::ExternalInputFanIn
            }
            ophiolite_seismic_runtime::ArtifactBoundaryReason::FullVolumeBarrier
            | ophiolite_seismic_runtime::ArtifactBoundaryReason::FamilyOperationBlock => {
                InspectableBoundaryReason::FamilyOperationBlock
            }
            ophiolite_seismic_runtime::ArtifactBoundaryReason::TraceLocalPrefix => {
                InspectableBoundaryReason::TraceLocalPrefix
            }
        });
    }

    Some(match stage.stage_kind {
        ExecutionStageKind::Checkpoint => InspectableBoundaryReason::AuthoredCheckpoint,
        ExecutionStageKind::FinalizeOutput => InspectableBoundaryReason::FinalOutput,
        ExecutionStageKind::ReuseArtifact => InspectableBoundaryReason::ExternalInputFanIn,
        ExecutionStageKind::Compute => match stage
            .pipeline_segment
            .as_ref()
            .map(|segment| segment.family)
        {
            Some(ProcessingPipelineFamily::Subvolume) => {
                InspectableBoundaryReason::GeometryBoundary
            }
            Some(_) => InspectableBoundaryReason::FamilyOperationBlock,
            None => return None,
        },
    })
}

fn inspectable_planning_mode(
    mode: ophiolite_seismic_runtime::PlanningMode,
) -> InspectablePlanningMode {
    match mode {
        ophiolite_seismic_runtime::PlanningMode::InteractivePreview => {
            InspectablePlanningMode::InteractivePreview
        }
        ophiolite_seismic_runtime::PlanningMode::ForegroundMaterialize => {
            InspectablePlanningMode::ForegroundMaterialize
        }
        ophiolite_seismic_runtime::PlanningMode::BackgroundBatch => {
            InspectablePlanningMode::BackgroundBatch
        }
    }
}

fn inspectable_priority_class(
    priority: ExecutionPriorityClass,
) -> InspectableExecutionPriorityClass {
    match priority {
        ExecutionPriorityClass::InteractivePreview => {
            InspectableExecutionPriorityClass::InteractivePreview
        }
        ExecutionPriorityClass::ForegroundMaterialize => {
            InspectableExecutionPriorityClass::ForegroundMaterialize
        }
        ExecutionPriorityClass::BackgroundBatch => {
            InspectableExecutionPriorityClass::BackgroundBatch
        }
    }
}

fn inspectable_stage_kind(kind: ExecutionStageKind) -> InspectableExecutionStageKind {
    match kind {
        ExecutionStageKind::Compute => InspectableExecutionStageKind::Compute,
        ExecutionStageKind::Checkpoint => InspectableExecutionStageKind::Checkpoint,
        ExecutionStageKind::ReuseArtifact => InspectableExecutionStageKind::ReuseArtifact,
        ExecutionStageKind::FinalizeOutput => InspectableExecutionStageKind::FinalizeOutput,
    }
}

fn inspectable_execution_queue_class(
    queue_class: ExecutionQueueClass,
) -> InspectableExecutionQueueClass {
    match queue_class {
        ExecutionQueueClass::Control => InspectableExecutionQueueClass::Control,
        ExecutionQueueClass::InteractivePartition => {
            InspectableExecutionQueueClass::InteractivePartition
        }
        ExecutionQueueClass::ForegroundPartition => {
            InspectableExecutionQueueClass::ForegroundPartition
        }
        ExecutionQueueClass::BackgroundPartition => {
            InspectableExecutionQueueClass::BackgroundPartition
        }
        ExecutionQueueClass::ExclusiveFullVolume => {
            InspectableExecutionQueueClass::ExclusiveFullVolume
        }
    }
}

fn inspectable_spillability_class(
    spillability: ophiolite_seismic_runtime::ExecutionSpillabilityClass,
) -> InspectableSpillabilityClass {
    match spillability {
        ophiolite_seismic_runtime::ExecutionSpillabilityClass::Unspillable => {
            InspectableSpillabilityClass::Unspillable
        }
        ophiolite_seismic_runtime::ExecutionSpillabilityClass::Spillable => {
            InspectableSpillabilityClass::Spillable
        }
        ophiolite_seismic_runtime::ExecutionSpillabilityClass::Exclusive => {
            InspectableSpillabilityClass::Exclusive
        }
    }
}

fn inspectable_retry_granularity(
    granularity: ophiolite_seismic_runtime::ExecutionRetryGranularity,
) -> InspectableRetryGranularity {
    match granularity {
        ophiolite_seismic_runtime::ExecutionRetryGranularity::Job => {
            InspectableRetryGranularity::Job
        }
        ophiolite_seismic_runtime::ExecutionRetryGranularity::Stage => {
            InspectableRetryGranularity::Stage
        }
        ophiolite_seismic_runtime::ExecutionRetryGranularity::Partition => {
            InspectableRetryGranularity::Partition
        }
    }
}

fn inspectable_progress_granularity(
    granularity: ophiolite_seismic_runtime::ExecutionProgressGranularity,
) -> InspectableProgressGranularity {
    match granularity {
        ophiolite_seismic_runtime::ExecutionProgressGranularity::Stage => {
            InspectableProgressGranularity::Stage
        }
        ophiolite_seismic_runtime::ExecutionProgressGranularity::Partition => {
            InspectableProgressGranularity::Partition
        }
    }
}

fn inspectable_exclusive_scope(scope: ExecutionExclusiveScope) -> InspectableExclusiveScope {
    match scope {
        ExecutionExclusiveScope::None => InspectableExclusiveScope::None,
        ExecutionExclusiveScope::FullVolume => InspectableExclusiveScope::FullVolume,
    }
}

fn inspectable_artifact_role(
    role: ophiolite_seismic_runtime::ExecutionArtifactRole,
) -> InspectableExecutionArtifactRole {
    match role {
        ophiolite_seismic_runtime::ExecutionArtifactRole::Input => {
            InspectableExecutionArtifactRole::Input
        }
        ophiolite_seismic_runtime::ExecutionArtifactRole::Checkpoint => {
            InspectableExecutionArtifactRole::Checkpoint
        }
        ophiolite_seismic_runtime::ExecutionArtifactRole::FinalOutput => {
            InspectableExecutionArtifactRole::FinalOutput
        }
        ophiolite_seismic_runtime::ExecutionArtifactRole::CachedReuse => {
            InspectableExecutionArtifactRole::CachedReuse
        }
    }
}

fn inspectable_partition_family(
    family: ophiolite_seismic_runtime::PartitionFamily,
) -> InspectablePartitionFamily {
    match family {
        ophiolite_seismic_runtime::PartitionFamily::TileGroup => {
            InspectablePartitionFamily::TileGroup
        }
        ophiolite_seismic_runtime::PartitionFamily::Section => InspectablePartitionFamily::Section,
        ophiolite_seismic_runtime::PartitionFamily::GatherGroup => {
            InspectablePartitionFamily::GatherGroup
        }
        ophiolite_seismic_runtime::PartitionFamily::FullVolume => {
            InspectablePartitionFamily::FullVolume
        }
    }
}

fn inspectable_partition_ordering(
    ordering: ophiolite_seismic_runtime::PartitionOrdering,
) -> InspectablePartitionOrdering {
    match ordering {
        ophiolite_seismic_runtime::PartitionOrdering::StorageOrder => {
            InspectablePartitionOrdering::StorageOrder
        }
        ophiolite_seismic_runtime::PartitionOrdering::InlineMajor => {
            InspectablePartitionOrdering::InlineMajor
        }
        ophiolite_seismic_runtime::PartitionOrdering::CrosslineMajor => {
            InspectablePartitionOrdering::CrosslineMajor
        }
        ophiolite_seismic_runtime::PartitionOrdering::Unspecified => {
            InspectablePartitionOrdering::Unspecified
        }
    }
}

fn inspectable_chunk_shape_policy(
    policy: ophiolite_seismic_runtime::ChunkShapePolicy,
) -> InspectableChunkShapePolicy {
    match policy {
        ophiolite_seismic_runtime::ChunkShapePolicy::InheritSource => {
            InspectableChunkShapePolicy::InheritSource
        }
        ophiolite_seismic_runtime::ChunkShapePolicy::PlannerSelected => {
            InspectableChunkShapePolicy::PlannerSelected
        }
    }
}

fn inspectable_cache_mode(mode: ophiolite_seismic_runtime::CacheMode) -> InspectableCacheMode {
    match mode {
        ophiolite_seismic_runtime::CacheMode::PreferReuse => InspectableCacheMode::PreferReuse,
        ophiolite_seismic_runtime::CacheMode::RequireReuse => InspectableCacheMode::RequireReuse,
        ophiolite_seismic_runtime::CacheMode::FreshCompute => InspectableCacheMode::FreshCompute,
    }
}

fn inspectable_cost_class(cost: MemoryCostClass) -> InspectableCostClass {
    match cost {
        MemoryCostClass::Low => InspectableCostClass::Low,
        MemoryCostClass::Medium => InspectableCostClass::Medium,
        MemoryCostClass::High => InspectableCostClass::High,
    }
}

fn inspectable_cpu_cost_class(
    cost: ophiolite_seismic_runtime::CpuCostClass,
) -> InspectableCostClass {
    match cost {
        ophiolite_seismic_runtime::CpuCostClass::Low => InspectableCostClass::Low,
        ophiolite_seismic_runtime::CpuCostClass::Medium => InspectableCostClass::Medium,
        ophiolite_seismic_runtime::CpuCostClass::High => InspectableCostClass::High,
    }
}

fn inspectable_io_cost_class(cost: ophiolite_seismic_runtime::IoCostClass) -> InspectableCostClass {
    match cost {
        ophiolite_seismic_runtime::IoCostClass::Low => InspectableCostClass::Low,
        ophiolite_seismic_runtime::IoCostClass::Medium => InspectableCostClass::Medium,
        ophiolite_seismic_runtime::IoCostClass::High => InspectableCostClass::High,
    }
}

fn inspectable_parallel_efficiency_class(
    efficiency: ophiolite_seismic_runtime::ParallelEfficiencyClass,
) -> InspectableParallelEfficiencyClass {
    match efficiency {
        ophiolite_seismic_runtime::ParallelEfficiencyClass::High => {
            InspectableParallelEfficiencyClass::High
        }
        ophiolite_seismic_runtime::ParallelEfficiencyClass::Medium => {
            InspectableParallelEfficiencyClass::Medium
        }
        ophiolite_seismic_runtime::ParallelEfficiencyClass::Low => {
            InspectableParallelEfficiencyClass::Low
        }
    }
}

fn inspectable_planner_pass_id(
    pass_id: ophiolite_seismic_runtime::PlanningPassId,
) -> InspectablePlannerPassId {
    match pass_id {
        ophiolite_seismic_runtime::PlanningPassId::ValidateAuthoredPipeline => {
            InspectablePlannerPassId::ValidateAuthoredPipeline
        }
        ophiolite_seismic_runtime::PlanningPassId::NormalizePipeline => {
            InspectablePlannerPassId::NormalizePipeline
        }
        ophiolite_seismic_runtime::PlanningPassId::DeriveSemanticSegments => {
            InspectablePlannerPassId::DeriveSemanticSegments
        }
        ophiolite_seismic_runtime::PlanningPassId::DeriveExecutionHints => {
            InspectablePlannerPassId::DeriveExecutionHints
        }
        ophiolite_seismic_runtime::PlanningPassId::PlanPartitions => {
            InspectablePlannerPassId::PlanPartitions
        }
        ophiolite_seismic_runtime::PlanningPassId::PlanArtifactsAndReuse => {
            InspectablePlannerPassId::PlanArtifactsAndReuse
        }
        ophiolite_seismic_runtime::PlanningPassId::AssembleExecutionPlan => {
            InspectablePlannerPassId::AssembleExecutionPlan
        }
    }
}

fn inspectable_materialization_class(
    class: ophiolite_seismic_runtime::MaterializationClass,
) -> InspectableMaterializationClass {
    match class {
        ophiolite_seismic_runtime::MaterializationClass::EphemeralWindow => {
            InspectableMaterializationClass::EphemeralWindow
        }
        ophiolite_seismic_runtime::MaterializationClass::EphemeralPartition => {
            InspectableMaterializationClass::EphemeralPartition
        }
        ophiolite_seismic_runtime::MaterializationClass::Checkpoint => {
            InspectableMaterializationClass::Checkpoint
        }
        ophiolite_seismic_runtime::MaterializationClass::PublishedOutput => {
            InspectableMaterializationClass::PublishedOutput
        }
        ophiolite_seismic_runtime::MaterializationClass::ReusedArtifact => {
            InspectableMaterializationClass::ReusedArtifact
        }
    }
}

fn inspectable_reuse_class(class: ophiolite_seismic_runtime::ReuseClass) -> InspectableReuseClass {
    match class {
        ophiolite_seismic_runtime::ReuseClass::InPlaceSameWindow => {
            InspectableReuseClass::InPlaceSameWindow
        }
        ophiolite_seismic_runtime::ReuseClass::ReusableSameSection => {
            InspectableReuseClass::ReusableSameSection
        }
        ophiolite_seismic_runtime::ReuseClass::ReusableSameGeometry => {
            InspectableReuseClass::ReusableSameGeometry
        }
        ophiolite_seismic_runtime::ReuseClass::RequiresExternalInputs => {
            InspectableReuseClass::RequiresExternalInputs
        }
        ophiolite_seismic_runtime::ReuseClass::GeometryBarrier => {
            InspectableReuseClass::GeometryBarrier
        }
        ophiolite_seismic_runtime::ReuseClass::FullVolumeBarrier => {
            InspectableReuseClass::FullVolumeBarrier
        }
    }
}

fn inspectable_artifact_lifetime_class(
    class: ophiolite_seismic_runtime::ArtifactLifetimeClass,
) -> InspectableArtifactLifetimeClass {
    match class {
        ophiolite_seismic_runtime::ArtifactLifetimeClass::Source => {
            InspectableArtifactLifetimeClass::Source
        }
        ophiolite_seismic_runtime::ArtifactLifetimeClass::Ephemeral => {
            InspectableArtifactLifetimeClass::Ephemeral
        }
        ophiolite_seismic_runtime::ArtifactLifetimeClass::Checkpoint => {
            InspectableArtifactLifetimeClass::Checkpoint
        }
        ophiolite_seismic_runtime::ArtifactLifetimeClass::Published => {
            InspectableArtifactLifetimeClass::Published
        }
        ophiolite_seismic_runtime::ArtifactLifetimeClass::CachedReuse => {
            InspectableArtifactLifetimeClass::CachedReuse
        }
    }
}

fn inspectable_boundary_reason(
    reason: ophiolite_seismic_runtime::ArtifactBoundaryReason,
) -> InspectableBoundaryReason {
    match reason {
        ophiolite_seismic_runtime::ArtifactBoundaryReason::SourceInput => {
            InspectableBoundaryReason::FamilyRoot
        }
        ophiolite_seismic_runtime::ArtifactBoundaryReason::AuthoredCheckpoint => {
            InspectableBoundaryReason::AuthoredCheckpoint
        }
        ophiolite_seismic_runtime::ArtifactBoundaryReason::FinalOutput => {
            InspectableBoundaryReason::FinalOutput
        }
        ophiolite_seismic_runtime::ArtifactBoundaryReason::GeometryDomainChange => {
            InspectableBoundaryReason::GeometryBoundary
        }
        ophiolite_seismic_runtime::ArtifactBoundaryReason::ExternalInputFanIn => {
            InspectableBoundaryReason::ExternalInputFanIn
        }
        ophiolite_seismic_runtime::ArtifactBoundaryReason::FullVolumeBarrier
        | ophiolite_seismic_runtime::ArtifactBoundaryReason::FamilyOperationBlock => {
            InspectableBoundaryReason::FamilyOperationBlock
        }
        ophiolite_seismic_runtime::ArtifactBoundaryReason::TraceLocalPrefix => {
            InspectableBoundaryReason::TraceLocalPrefix
        }
    }
}

fn inspectable_logical_domain(
    domain: &ophiolite_seismic_runtime::LogicalDomain,
) -> InspectableLogicalDomain {
    match domain {
        ophiolite_seismic_runtime::LogicalDomain::Volume { volume } => {
            InspectableLogicalDomain::Volume {
                volume: ophiolite_seismic::contracts::InspectableVolumeDomain {
                    shape: volume.shape,
                },
            }
        }
        ophiolite_seismic_runtime::LogicalDomain::Section { section } => {
            InspectableLogicalDomain::Section {
                section: ophiolite_seismic::contracts::InspectableSectionDomain {
                    axis: section.axis,
                    section_index: section.section_index,
                },
            }
        }
        ophiolite_seismic_runtime::LogicalDomain::SectionWindow { section_window } => {
            InspectableLogicalDomain::SectionWindow {
                section_window: ophiolite_seismic::contracts::InspectableSectionWindowDomain {
                    axis: section_window.axis,
                    section_index: section_window.section_index,
                    trace_range: section_window.trace_range,
                    sample_range: section_window.sample_range,
                    lod: section_window.lod,
                },
            }
        }
        ophiolite_seismic_runtime::LogicalDomain::Tile { tile } => InspectableLogicalDomain::Tile {
            tile: ophiolite_seismic::contracts::InspectableTileDomain {
                tile_index: tile.tile_index,
                tile_origin: tile.tile_origin,
                tile_shape: tile.tile_shape,
            },
        },
        ophiolite_seismic_runtime::LogicalDomain::Partition { partition } => {
            InspectableLogicalDomain::Partition {
                partition: ophiolite_seismic::contracts::InspectablePartitionDomain {
                    partition_index: partition.partition_index,
                    partition_count: partition.partition_count,
                    tile_range: partition.tile_range,
                },
            }
        }
    }
}

fn inspectable_chunk_grid_spec(
    spec: &ophiolite_seismic_runtime::ChunkGridSpec,
) -> InspectableChunkGridSpec {
    match spec {
        ophiolite_seismic_runtime::ChunkGridSpec::Regular {
            origin,
            chunk_shape,
        } => InspectableChunkGridSpec::Regular {
            origin: *origin,
            chunk_shape: *chunk_shape,
        },
    }
}

fn inspectable_geometry_fingerprints(
    geometry: &ophiolite_seismic_runtime::GeometryFingerprints,
) -> InspectableGeometryFingerprints {
    InspectableGeometryFingerprints {
        survey_geometry_fingerprint: geometry.survey_geometry_fingerprint.clone(),
        storage_grid_fingerprint: geometry.storage_grid_fingerprint.clone(),
        section_projection_fingerprint: geometry.section_projection_fingerprint.clone(),
        artifact_lineage_fingerprint: geometry.artifact_lineage_fingerprint.clone(),
    }
}

fn inspectable_artifact_key(
    artifact_key: &ophiolite_seismic_runtime::ArtifactKey,
) -> InspectableArtifactKey {
    InspectableArtifactKey {
        lineage_digest: artifact_key.lineage_digest.clone(),
        geometry_fingerprints: inspectable_geometry_fingerprints(
            &artifact_key.geometry_fingerprints,
        ),
        logical_domain: inspectable_logical_domain(&artifact_key.logical_domain),
        chunk_grid_spec: inspectable_chunk_grid_spec(&artifact_key.chunk_grid_spec),
        materialization_class: inspectable_materialization_class(
            artifact_key.materialization_class,
        ),
        cache_key: artifact_key.cache_key.clone(),
    }
}

fn inspectable_plan_decision(
    decision: &ophiolite_seismic_runtime::PlanDecision,
) -> InspectablePlanDecision {
    InspectablePlanDecision {
        decision_id: decision.decision_id.clone(),
        subject_kind: match decision.subject_kind {
            ophiolite_seismic_runtime::PlanDecisionSubjectKind::PlannerPass => {
                InspectablePlanDecisionSubjectKind::PlannerPass
            }
            ophiolite_seismic_runtime::PlanDecisionSubjectKind::Stage => {
                InspectablePlanDecisionSubjectKind::Stage
            }
            ophiolite_seismic_runtime::PlanDecisionSubjectKind::Artifact => {
                InspectablePlanDecisionSubjectKind::Artifact
            }
            ophiolite_seismic_runtime::PlanDecisionSubjectKind::Scheduler => {
                InspectablePlanDecisionSubjectKind::Scheduler
            }
        },
        subject_id: decision.subject_id.clone(),
        decision_kind: match decision.decision_kind {
            ophiolite_seismic_runtime::PlanDecisionKind::Lowering => {
                InspectablePlanDecisionKind::Lowering
            }
            ophiolite_seismic_runtime::PlanDecisionKind::Scheduling => {
                InspectablePlanDecisionKind::Scheduling
            }
            ophiolite_seismic_runtime::PlanDecisionKind::Reuse => {
                InspectablePlanDecisionKind::Reuse
            }
            ophiolite_seismic_runtime::PlanDecisionKind::ArtifactDerivation => {
                InspectablePlanDecisionKind::ArtifactDerivation
            }
        },
        reason_code: decision.reason_code.clone(),
        human_summary: decision.human_summary.clone(),
        stage_planning: decision.stage_planning.as_ref().map(|stage_planning| {
            InspectableStagePlanningDecision {
                selected_partition_family: inspectable_partition_family(
                    stage_planning.selected_partition_family,
                ),
                selected_ordering: inspectable_partition_ordering(stage_planning.selected_ordering),
                selected_target_bytes: stage_planning.selected_target_bytes,
                selected_expected_partition_count: stage_planning.selected_expected_partition_count,
                selected_queue_class: inspectable_execution_queue_class(
                    stage_planning.selected_queue_class,
                ),
                selected_spillability: inspectable_spillability_class(
                    stage_planning.selected_spillability,
                ),
                selected_exclusive_scope: inspectable_exclusive_scope(
                    stage_planning.selected_exclusive_scope,
                ),
                selected_preferred_partition_waves: stage_planning
                    .selected_preferred_partition_waves,
                selected_reservation_bytes: stage_planning.selected_reservation_bytes,
                factors: stage_planning
                    .factors
                    .iter()
                    .map(|factor| InspectableDecisionFactor {
                        code: factor.code.clone(),
                        summary: factor.summary.clone(),
                        value: factor.value.clone(),
                    })
                    .collect(),
            }
        }),
        reuse_decision: decision.reuse_decision.as_ref().map(|reuse_decision| {
            InspectableReuseDecision {
                stage_id: reuse_decision.stage_id.clone(),
                artifact_id: reuse_decision.artifact_id.clone(),
                cache_mode: inspectable_cache_mode(reuse_decision.cache_mode),
                artifact_kind: reuse_decision.artifact_kind,
                boundary_kind: reuse_decision.boundary_kind,
                candidate_count: reuse_decision.candidate_count,
                selected_candidate_reuse_key: reuse_decision.selected_candidate_reuse_key.clone(),
                selected_candidate_artifact_key: reuse_decision
                    .selected_candidate_artifact_key
                    .clone(),
                selected_candidate_store_path: reuse_decision.selected_candidate_store_path.clone(),
                outcome: match reuse_decision.outcome {
                    ophiolite_seismic_runtime::ReuseDecisionOutcome::Reused => {
                        InspectableReuseDecisionOutcome::Reused
                    }
                    ophiolite_seismic_runtime::ReuseDecisionOutcome::Miss => {
                        InspectableReuseDecisionOutcome::Miss
                    }
                    ophiolite_seismic_runtime::ReuseDecisionOutcome::Unresolved => {
                        InspectableReuseDecisionOutcome::Unresolved
                    }
                },
                miss_reason: reuse_decision.miss_reason,
                evidence: reuse_decision
                    .evidence
                    .iter()
                    .map(|evidence| InspectableReuseDecisionEvidence {
                        label: evidence.label.clone(),
                        matched: evidence.matched,
                        artifact_key: evidence.artifact_key.clone(),
                        artifact_store_path: evidence.artifact_store_path.clone(),
                        miss_reason: evidence.miss_reason,
                    })
                    .collect(),
            }
        }),
        artifact_derivation: decision
            .artifact_derivation
            .as_ref()
            .map(|artifact_derivation| InspectableArtifactDerivation {
                artifact_id: artifact_derivation.artifact_id.clone(),
                artifact_key: inspectable_artifact_key(&artifact_derivation.artifact_key),
                input_artifact_ids: artifact_derivation.input_artifact_ids.clone(),
                logical_domain: inspectable_logical_domain(&artifact_derivation.logical_domain),
                chunk_grid_spec: inspectable_chunk_grid_spec(&artifact_derivation.chunk_grid_spec),
                geometry_fingerprints: inspectable_geometry_fingerprints(
                    &artifact_derivation.geometry_fingerprints,
                ),
                materialization_class: inspectable_materialization_class(
                    artifact_derivation.materialization_class,
                ),
                boundary_reason: inspectable_boundary_reason(artifact_derivation.boundary_reason),
            }),
    }
}

fn format_planning_mode(mode: InspectablePlanningMode) -> String {
    match mode {
        InspectablePlanningMode::InteractivePreview => "interactive_preview".to_string(),
        InspectablePlanningMode::ForegroundMaterialize => "foreground_materialize".to_string(),
        InspectablePlanningMode::BackgroundBatch => "background_batch".to_string(),
    }
}

fn format_cost_class(cost: InspectableCostClass) -> String {
    match cost {
        InspectableCostClass::Low => "low".to_string(),
        InspectableCostClass::Medium => "medium".to_string(),
        InspectableCostClass::High => "high".to_string(),
    }
}

fn format_parallel_efficiency_class(efficiency: InspectableParallelEfficiencyClass) -> String {
    match efficiency {
        InspectableParallelEfficiencyClass::High => "high".to_string(),
        InspectableParallelEfficiencyClass::Medium => "medium".to_string(),
        InspectableParallelEfficiencyClass::Low => "low".to_string(),
    }
}

fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn global_batch_cap_from_env() -> Option<usize> {
    std::env::var("OPHIOLITE_PROCESSING_GLOBAL_CAP")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .map(|value| value.max(1))
}

fn processing_memory_budget_from_env() -> Option<u64> {
    std::env::var("OPHIOLITE_PROCESSING_MEMORY_LIMIT_BYTES")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|value| value.max(1))
}

fn default_processing_memory_budget(worker_count: usize) -> u64 {
    let workers = u64::try_from(worker_count.max(1)).unwrap_or(1);
    workers.saturating_mul(1024 * 1024 * 1024)
}

fn resolve_batch_execution_policy(
    scheduler_worker_count: usize,
    global_batch_cap: usize,
    requested_max_active_jobs: Option<usize>,
    requested_execution_mode: Option<ProcessingExecutionMode>,
    pipeline: &ProcessingPipelineSpec,
    plan: Option<&ExecutionPlan>,
    priority: ExecutionPriorityClass,
) -> BatchExecutionPolicyDecision {
    let worker_budget = scheduler_worker_count.max(1);
    let global_cap = global_batch_cap.max(1);
    let effective_cap = worker_budget.min(global_cap).max(1);
    let normalized_mode =
        normalize_execution_mode(requested_max_active_jobs, requested_execution_mode);
    let profile = plan
        .map(plan_batch_profile)
        .unwrap_or_else(|| pipeline_batch_profile(pipeline));
    match priority {
        ExecutionPriorityClass::InteractivePreview => BatchExecutionPolicyDecision {
            requested_max_active_jobs,
            effective_max_active_jobs: 1,
            execution_mode: normalized_mode,
            scheduler_reason: ProcessingSchedulerReason::InteractivePreviewPolicy,
            worker_budget,
            global_cap,
            max_memory_cost_class: profile.max_memory_cost_class,
            max_estimated_peak_memory_bytes: profile.max_estimated_peak_memory_bytes,
            max_expected_partition_count: profile.max_expected_partition_count,
        },
        ExecutionPriorityClass::ForegroundMaterialize => BatchExecutionPolicyDecision {
            requested_max_active_jobs,
            effective_max_active_jobs: effective_cap.min(2).max(1),
            execution_mode: normalized_mode,
            scheduler_reason: ProcessingSchedulerReason::ForegroundMaterializePolicy,
            worker_budget,
            global_cap,
            max_memory_cost_class: profile.max_memory_cost_class,
            max_estimated_peak_memory_bytes: profile.max_estimated_peak_memory_bytes,
            max_expected_partition_count: profile.max_expected_partition_count,
        },
        ExecutionPriorityClass::BackgroundBatch => {
            if let Some(requested) = requested_max_active_jobs {
                return BatchExecutionPolicyDecision {
                    requested_max_active_jobs: Some(requested),
                    effective_max_active_jobs: requested.max(1).min(effective_cap),
                    execution_mode: ProcessingExecutionMode::Custom,
                    scheduler_reason: ProcessingSchedulerReason::UserRequested,
                    worker_budget,
                    global_cap,
                    max_memory_cost_class: profile.max_memory_cost_class,
                    max_estimated_peak_memory_bytes: profile.max_estimated_peak_memory_bytes,
                    max_expected_partition_count: profile.max_expected_partition_count,
                };
            }

            let (recommended, scheduler_reason): (usize, ProcessingSchedulerReason) =
                if profile.requires_full_volume {
                    (1, ProcessingSchedulerReason::AutoFullVolumeBatch)
                } else {
                    match normalized_mode {
                        ProcessingExecutionMode::Auto | ProcessingExecutionMode::Custom => {
                            match profile.max_memory_cost_class {
                                MemoryCostClass::High => {
                                    (2, ProcessingSchedulerReason::AutoHighCostBatch)
                                }
                                MemoryCostClass::Medium => {
                                    (3, ProcessingSchedulerReason::AutoMediumCostBatch)
                                }
                                MemoryCostClass::Low => {
                                    (4, ProcessingSchedulerReason::AutoLowCostBatch)
                                }
                            }
                        }
                        ProcessingExecutionMode::Conservative => (
                            match profile.max_memory_cost_class {
                                MemoryCostClass::High => 1,
                                MemoryCostClass::Medium | MemoryCostClass::Low => 2,
                            },
                            ProcessingSchedulerReason::ConservativeMode,
                        ),
                        ProcessingExecutionMode::Throughput => (
                            match profile.max_memory_cost_class {
                                MemoryCostClass::High => 3,
                                MemoryCostClass::Medium => 4,
                                MemoryCostClass::Low => effective_cap,
                            },
                            ProcessingSchedulerReason::ThroughputMode,
                        ),
                    }
                };

            BatchExecutionPolicyDecision {
                requested_max_active_jobs: None,
                effective_max_active_jobs: recommended.min(effective_cap).max(1),
                execution_mode: normalized_mode,
                scheduler_reason,
                worker_budget,
                global_cap,
                max_memory_cost_class: profile.max_memory_cost_class,
                max_estimated_peak_memory_bytes: profile.max_estimated_peak_memory_bytes,
                max_expected_partition_count: profile.max_expected_partition_count,
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BatchExecutionProfile {
    max_memory_cost_class: MemoryCostClass,
    max_estimated_peak_memory_bytes: u64,
    max_expected_partition_count: Option<usize>,
    requires_full_volume: bool,
}

fn normalize_execution_mode(
    requested_max_active_jobs: Option<usize>,
    requested_execution_mode: Option<ProcessingExecutionMode>,
) -> ProcessingExecutionMode {
    if requested_max_active_jobs.is_some() {
        ProcessingExecutionMode::Custom
    } else {
        match requested_execution_mode.unwrap_or(ProcessingExecutionMode::Auto) {
            ProcessingExecutionMode::Custom => ProcessingExecutionMode::Auto,
            mode => mode,
        }
    }
}

fn plan_batch_profile(plan: &ExecutionPlan) -> BatchExecutionProfile {
    let requires_full_volume = plan.stages.iter().any(|stage| {
        stage.pipeline_segment.is_some()
            && matches!(
                stage.partition_spec.family,
                ophiolite_seismic_runtime::PartitionFamily::FullVolume
            )
    });
    BatchExecutionProfile {
        max_memory_cost_class: plan.plan_summary.max_memory_cost_class,
        max_estimated_peak_memory_bytes: plan.plan_summary.max_estimated_peak_memory_bytes,
        max_expected_partition_count: plan.plan_summary.max_expected_partition_count,
        requires_full_volume,
    }
}

fn pipeline_batch_profile(pipeline: &ProcessingPipelineSpec) -> BatchExecutionProfile {
    let traits = operator_execution_traits_for_pipeline_spec(pipeline);
    BatchExecutionProfile {
        max_memory_cost_class: traits
            .iter()
            .map(|traits| traits.memory_cost_class)
            .max_by_key(|cost| memory_cost_class_rank(*cost))
            .unwrap_or(MemoryCostClass::Low),
        max_estimated_peak_memory_bytes: traits
            .iter()
            .map(|traits| match traits.memory_cost_class {
                MemoryCostClass::Low => 64 * 1024 * 1024,
                MemoryCostClass::Medium => 192 * 1024 * 1024,
                MemoryCostClass::High => 384 * 1024 * 1024,
            })
            .max()
            .unwrap_or(64 * 1024 * 1024),
        max_expected_partition_count: None,
        requires_full_volume: traits.iter().any(|traits| traits.requires_full_volume),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{
        PostStackNeighborhoodProcessingOperation, PostStackNeighborhoodProcessingPipeline,
        PostStackNeighborhoodWindow, ProcessingJobQueueClass, SeismicLayout,
        SubvolumeCropOperation, SubvolumeProcessingPipeline, TraceLocalProcessingOperation,
        TraceLocalProcessingPipeline, TraceLocalProcessingStep,
        contracts::{
            ProcessingRuntimePolicyDivergenceField, ReuseArtifactKind, ReuseBoundaryKind,
            ReuseResolution,
        },
    };
    use ophiolite_seismic_runtime::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, PlanProcessingRequest, PlanningMode,
        SourceIdentity, TbvolManifest, VolumeAxes, VolumeMetadata, build_execution_plan,
        generate_store_id, segy_sample_data_fidelity,
    };
    use std::fs;
    use std::path::PathBuf;

    fn canonical_test_store_path(
        label: &str,
        shape: [usize; 3],
        chunk_shape: [usize; 3],
    ) -> String {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ophiolite-execution-{label}-{unique}.tbvol"));
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
    fn batch_status_reflects_child_jobs() {
        let service = ProcessingExecutionService::new(1);
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: None,
                description: None,
                steps: Vec::new(),
            },
        };
        let first = service.register_job(
            "a.tbvol".to_string(),
            Some("a.out.tbvol".to_string()),
            pipeline.clone(),
            None,
        );
        let second = service.register_job(
            "b.tbvol".to_string(),
            Some("b.out.tbvol".to_string()),
            pipeline.clone(),
            None,
        );
        let first_record = service.job_record(&first.job_id).expect("job should exist");
        let second_record = service
            .job_record(&second.job_id)
            .expect("job should exist");
        let _ = first_record.mark_completed("a.out.tbvol".to_string());
        let _ = second_record.mark_failed("boom".to_string());
        let policy = service.resolve_batch_execution_policy(
            Some(2),
            None,
            &pipeline,
            None,
            ExecutionPriorityClass::BackgroundBatch,
        );

        let batch = service
            .register_batch(
                pipeline,
                vec![
                    ProcessingBatchItemRequest {
                        store_path: "a.tbvol".to_string(),
                        output_store_path: Some("a.out.tbvol".to_string()),
                    },
                    ProcessingBatchItemRequest {
                        store_path: "b.tbvol".to_string(),
                        output_store_path: Some("b.out.tbvol".to_string()),
                    },
                ],
                vec![first.job_id, second.job_id],
                &policy,
            )
            .expect("batch should register");

        assert_eq!(batch.state, ProcessingBatchState::CompletedWithErrors);
        assert_eq!(batch.progress.completed_jobs, 2);
        assert_eq!(batch.progress.total_jobs, 2);
        assert_eq!(batch.requested_max_active_jobs, Some(2));
        assert_eq!(batch.effective_max_active_jobs, 1);
        assert_eq!(batch.execution_mode, ProcessingExecutionMode::Custom);
        assert_eq!(
            batch.scheduler_reason,
            ProcessingSchedulerReason::UserRequested
        );
    }

    #[test]
    fn planned_jobs_capture_plan_summary_on_status() {
        let service = ProcessingExecutionService::new(1);
        let input_store_path =
            canonical_test_store_path("planned-jobs-summary", [16, 16, 128], [4, 4, 128]);
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: Some("demo".to_string()),
                description: None,
                steps: vec![
                    TraceLocalProcessingStep {
                        operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 2.0 },
                        checkpoint: true,
                    },
                    TraceLocalProcessingStep {
                        operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                        checkpoint: false,
                    },
                ],
            },
        };
        let plan = build_execution_plan(&PlanProcessingRequest {
            store_path: input_store_path.clone(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([16, 16, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: pipeline.clone(),
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        })
        .expect("plan should build");

        let status = service.register_job(
            input_store_path.clone(),
            Some("output.tbvol".to_string()),
            pipeline,
            Some(plan),
        );

        let summary = status
            .plan_summary
            .expect("planned jobs should expose summary");
        let inspectable_plan = status
            .inspectable_plan
            .as_ref()
            .expect("planned jobs should expose inspectable plan");
        assert_eq!(summary.planning_mode, "foreground_materialize");
        assert_eq!(summary.stage_count, 2);
        assert_eq!(summary.stage_labels.len(), 2);
        assert_eq!(
            summary.stage_labels,
            inspectable_plan
                .execution_plan
                .stages
                .iter()
                .map(|stage| stage.stage_label.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(summary.expected_partition_count, Some(2));
        assert_eq!(summary.max_active_partitions, Some(2));
        assert_eq!(
            summary.stage_partition_summaries,
            vec![
                "tile_group x1 (~256 MiB target)".to_string(),
                "tile_group x1 (~256 MiB target)".to_string(),
            ]
        );
        assert_eq!(summary.max_memory_cost_class, "medium");
        assert_eq!(summary.max_cpu_cost_class, "medium");
        assert_eq!(summary.max_io_cost_class, "low");
        assert_eq!(summary.min_parallel_efficiency_class, "high");
        assert!(summary.combined_cpu_weight >= 3.0);
        assert!(summary.combined_io_weight >= 2.0);
        assert_eq!(summary.stage_classification_summaries.len(), 2);
        let rendered_plan = inspectable_plan
            .render_text_tree()
            .replace(&inspectable_plan.plan_id, "plan-id");
        assert_eq!(
            rendered_plan,
            format!(
                concat!(
                    "plan plan-id (foreground_materialize)\n",
                    "source {} layout=post_stack_3d\n",
                    "semantic_plan\n",
                    "  family=trace_local name=demo revision=1\n",
                    "  trace_local name=demo revision=1 reason=family_root\n",
                    "    segment 0..0 steps=1 boundary=authored_checkpoint checkpoint=true ids=amplitude_scalar\n",
                    "    segment 1..1 steps=1 boundary=final_output checkpoint=false ids=trace_rms_normalize\n",
                    "execution_plan\n",
                    "  summary compute_stages=2 max_memory=medium max_cpu=medium max_io=low min_parallel=high expected_partitions=1\n",
                    "  scheduler priority=foreground_materialize max_active_partitions=2 expected_partition_count=2\n",
                    "  stage stage-01 kind=checkpoint label=Step 1: Amplitude Scale boundary=authored_checkpoint partition=tile_group (~256 MiB target) expected_partitions=1\n",
                    "    segment family=trace_local 0..0\n",
                    "    reuse required kind=visible_checkpoint boundary=authored_checkpoint\n",
                    "    reuse resolution reused=false miss_reason=unresolved_at_planning_time\n",
                    "  stage stage-02 kind=finalize_output label=Step 2: Trace RMS Normalize boundary=final_output partition=tile_group (~256 MiB target) expected_partitions=1\n",
                    "    segment family=trace_local 1..1\n",
                    "    reuse required kind=exact_visible_final boundary=exact_output\n",
                    "    reuse resolution reused=false miss_reason=unresolved_at_planning_time\n",
                    "artifacts\n",
                    "  artifact source role=input produced_by=- consumed_by=stage-01\n",
                    "  artifact checkpoint-01 role=checkpoint produced_by=stage-01 consumed_by=stage-02\n",
                    "    reuse required kind=visible_checkpoint boundary=authored_checkpoint\n",
                    "    reuse resolution reused=false miss_reason=unresolved_at_planning_time\n",
                    "  artifact final-output role=final_output produced_by=stage-02 consumed_by=-\n",
                    "    reuse required kind=exact_visible_final boundary=exact_output\n",
                    "    reuse resolution reused=false miss_reason=unresolved_at_planning_time\n",
                    "planner_diagnostics valid=true warnings=0 blockers=0 snapshots=7"
                ),
                input_store_path
            )
        );
    }

    #[test]
    fn runtime_snapshot_reports_policy_divergences_against_planned_stage_policy() {
        let service = ProcessingExecutionService::new(1);
        let input_store_path =
            canonical_test_store_path("runtime-policy-divergence", [16, 16, 128], [4, 4, 128]);
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: Some("divergence-demo".to_string()),
                description: None,
                steps: vec![TraceLocalProcessingStep {
                    operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 2.0 },
                    checkpoint: false,
                }],
            },
        };
        let plan = build_execution_plan(&PlanProcessingRequest {
            store_path: input_store_path.clone(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([16, 16, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: pipeline.clone(),
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        })
        .expect("plan should build");

        let status = service.register_job(
            input_store_path,
            Some("output.tbvol".to_string()),
            pipeline,
            Some(plan),
        );
        let record = service
            .job_record(&status.job_id)
            .expect("registered job should exist");

        record.set_scheduler_runtime(
            ProcessingJobQueueClass::BackgroundPartition,
            ProcessingJobWaitReason::AwaitingWorker,
            123,
            1024 * 1024,
            5,
            false,
            true,
        );

        let runtime = record.runtime_state();
        let job_divergence_fields = runtime
            .snapshot
            .expect("runtime snapshot")
            .policy_divergences
            .into_iter()
            .map(|divergence| divergence.field)
            .collect::<Vec<_>>();
        assert!(
            job_divergence_fields.contains(&ProcessingRuntimePolicyDivergenceField::QueueClass)
        );
        assert!(
            job_divergence_fields.contains(&ProcessingRuntimePolicyDivergenceField::ExclusiveScope)
        );
        assert!(
            job_divergence_fields
                .contains(&ProcessingRuntimePolicyDivergenceField::ReservedMemoryBytes)
        );
        assert!(
            job_divergence_fields
                .contains(&ProcessingRuntimePolicyDivergenceField::EffectiveMaxActivePartitions)
        );

        let stage_divergence_fields = runtime
            .stage_snapshots
            .first()
            .expect("stage snapshot")
            .policy_divergences
            .iter()
            .map(|divergence| divergence.field)
            .collect::<Vec<_>>();
        assert!(
            stage_divergence_fields.contains(&ProcessingRuntimePolicyDivergenceField::QueueClass)
        );
        assert!(
            stage_divergence_fields
                .contains(&ProcessingRuntimePolicyDivergenceField::ExclusiveScope)
        );
    }

    #[test]
    fn stage_running_snapshot_uses_lowered_stage_policy_context() {
        let service = ProcessingExecutionService::new(1);
        let input_store_path =
            canonical_test_store_path("runtime-stage-policy", [16, 16, 128], [4, 4, 128]);
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: Some("stage-policy-demo".to_string()),
                description: None,
                steps: vec![TraceLocalProcessingStep {
                    operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 2.0 },
                    checkpoint: false,
                }],
            },
        };
        let mut plan = build_execution_plan(&PlanProcessingRequest {
            store_path: input_store_path.clone(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([16, 16, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: pipeline.clone(),
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        })
        .expect("plan should build");
        plan.stages[0].lowered_scheduler_policy.reservation_bytes = 777;
        plan.stages[0]
            .lowered_scheduler_policy
            .effective_max_active_partitions = 2;
        let stage_label = plan.stages[0].stage_label.clone();

        let status = service.register_job(
            input_store_path,
            Some("output.tbvol".to_string()),
            pipeline,
            Some(plan),
        );
        let record = service
            .job_record(&status.job_id)
            .expect("registered job should exist");

        record.set_stage_running(&stage_label);

        let runtime = record.runtime_state();
        let stage_snapshot = runtime
            .stage_snapshots
            .first()
            .expect("stage snapshot should exist");
        assert_eq!(stage_snapshot.state, ProcessingRuntimeState::Running);
        assert_eq!(
            stage_snapshot.queue_class,
            Some(ProcessingJobQueueClass::ForegroundPartition)
        );
        assert_eq!(stage_snapshot.reserved_memory_bytes, 777);
        assert_eq!(stage_snapshot.effective_max_active_partitions, Some(2));

        let running_event = record
            .runtime_events_after(None)
            .into_iter()
            .find(|event| event.event_kind == ProcessingRuntimeEventKind::StageRunning)
            .expect("stage running event");
        match running_event.details {
            ProcessingRuntimeEventDetails::QueueState {
                reserved_memory_bytes,
                effective_max_active_partitions,
                ..
            } => {
                assert_eq!(reserved_memory_bytes, 777);
                assert_eq!(effective_max_active_partitions, Some(2));
            }
            details => panic!("unexpected event details: {details:?}"),
        }
    }

    #[test]
    fn background_batch_policy_prefers_higher_concurrency_for_trace_local_pipelines() {
        let service = ProcessingExecutionService::new(8);
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: Some("agc".to_string()),
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
        };

        let policy = service.resolve_batch_execution_policy(
            None,
            Some(ProcessingExecutionMode::Auto),
            &pipeline,
            None,
            ExecutionPriorityClass::BackgroundBatch,
        );
        assert_eq!(policy.effective_max_active_jobs, 3);
        assert_eq!(policy.execution_mode, ProcessingExecutionMode::Auto);
        assert_eq!(
            policy.scheduler_reason,
            ProcessingSchedulerReason::AutoMediumCostBatch
        );
    }

    #[test]
    fn inspectable_plan_renders_exact_final_reuse_resolution() {
        let service = ProcessingExecutionService::new(1);
        let input_store_path =
            canonical_test_store_path("exact-final-reuse", [16, 16, 128], [4, 4, 128]);
        let pipeline = ProcessingPipelineSpec::TraceLocal {
            pipeline: TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 3,
                preset_id: None,
                name: Some("final-reuse".to_string()),
                description: None,
                steps: vec![TraceLocalProcessingStep {
                    operation: TraceLocalProcessingOperation::Envelope,
                    checkpoint: false,
                }],
            },
        };
        let mut plan = build_execution_plan(&PlanProcessingRequest {
            store_path: input_store_path.clone(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([16, 16, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: pipeline.clone(),
            output_store_path: Some("reused-output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        })
        .expect("plan should build");

        let final_resolution = ReuseResolution {
            reuse_key: plan.stages[0]
                .reuse_requirement
                .as_ref()
                .expect("final stage reuse requirement")
                .reuse_key
                .clone(),
            artifact_kind: ReuseArtifactKind::ExactVisibleFinal,
            boundary_kind: ReuseBoundaryKind::ExactOutput,
            reused: true,
            miss_reason: None,
            artifact_store_path: Some("reused-output.tbvol".to_string()),
        };
        plan.stages[0].reuse_resolution = Some(final_resolution.clone());
        plan.artifacts
            .iter_mut()
            .find(|artifact| artifact.artifact_id == "final-output")
            .expect("final artifact")
            .reuse_resolution = Some(final_resolution);

        let status = service.register_job(
            input_store_path,
            Some("reused-output.tbvol".to_string()),
            pipeline,
            Some(plan),
        );
        let rendered_plan = status
            .inspectable_plan
            .as_ref()
            .expect("planned jobs should expose inspectable plan")
            .render_text_tree()
            .replace(
                &status
                    .inspectable_plan
                    .as_ref()
                    .expect("inspectable plan")
                    .plan_id,
                "plan-id",
            );

        assert!(rendered_plan.contains(
            "stage stage-01 kind=finalize_output label=Step 1: Envelope boundary=final_output"
        ));
        assert!(
            rendered_plan.contains("reuse required kind=exact_visible_final boundary=exact_output")
        );
        assert!(rendered_plan.contains("reuse resolution reused=true miss_reason=-"));
        assert!(rendered_plan.contains(
            "artifact final-output role=final_output produced_by=stage-01 consumed_by=-"
        ));
    }

    #[test]
    fn inspectable_plan_renders_trace_local_prefix_checkpoint_miss() {
        let service = ProcessingExecutionService::new(1);
        let input_store_path =
            canonical_test_store_path("prefix-checkpoint-miss", [16, 16, 128], [4, 4, 128]);
        let pipeline = ProcessingPipelineSpec::Subvolume {
            pipeline: SubvolumeProcessingPipeline {
                schema_version: 1,
                revision: 11,
                preset_id: None,
                name: Some("subvolume-prefix".to_string()),
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
        };
        let plan = build_execution_plan(&PlanProcessingRequest {
            store_path: input_store_path.clone(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some([16, 16, 128]),
            source_chunk_shape: Some([4, 4, 128]),
            pipeline: pipeline.clone(),
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode: PlanningMode::ForegroundMaterialize,
            max_active_partitions: Some(2),
        })
        .expect("plan should build");

        let status = service.register_job(
            input_store_path,
            Some("output.tbvol".to_string()),
            pipeline,
            Some(plan),
        );
        let inspectable_plan = status
            .inspectable_plan
            .as_ref()
            .expect("planned jobs should expose inspectable plan");
        let rendered_plan = inspectable_plan
            .render_text_tree()
            .replace(&inspectable_plan.plan_id, "plan-id");

        assert!(
            rendered_plan
                .contains("prefix trace_local name=prefix revision=7 reason=trace_local_prefix")
        );
        assert!(rendered_plan.contains(
            "stage stage-01 kind=checkpoint label=Step 2: Instantaneous Phase boundary=trace_local_prefix"
        ));
        assert!(
            rendered_plan
                .contains("reuse required kind=visible_checkpoint boundary=trace_local_prefix")
        );
        assert!(
            rendered_plan
                .contains("reuse resolution reused=false miss_reason=unresolved_at_planning_time")
        );
        assert!(rendered_plan.contains(
            "artifact checkpoint-01 role=checkpoint produced_by=stage-01 consumed_by=stage-02"
        ));
    }

    #[test]
    fn background_batch_policy_limits_high_memory_pipelines() {
        let service = ProcessingExecutionService::new(8);
        let pipeline = ProcessingPipelineSpec::PostStackNeighborhood {
            pipeline: PostStackNeighborhoodProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: Some("similarity".to_string()),
                description: None,
                trace_local_pipeline: None,
                operations: vec![PostStackNeighborhoodProcessingOperation::Similarity {
                    window: PostStackNeighborhoodWindow {
                        inline_stepout: 2,
                        xline_stepout: 2,
                        gate_ms: 24.0,
                    },
                }],
            },
        };

        let policy = service.resolve_batch_execution_policy(
            None,
            Some(ProcessingExecutionMode::Throughput),
            &pipeline,
            None,
            ExecutionPriorityClass::BackgroundBatch,
        );
        assert_eq!(policy.effective_max_active_jobs, 3);
        assert_eq!(policy.execution_mode, ProcessingExecutionMode::Throughput);
        assert_eq!(
            policy.scheduler_reason,
            ProcessingSchedulerReason::ThroughputMode
        );
        assert_eq!(policy.max_memory_cost_class, MemoryCostClass::High);
    }
}
