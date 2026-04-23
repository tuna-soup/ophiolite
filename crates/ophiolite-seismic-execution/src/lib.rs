use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ophiolite_seismic::{
    ProcessingBatchItemRequest, ProcessingBatchItemStatus, ProcessingBatchProgress,
    ProcessingBatchState, ProcessingBatchStatus, ProcessingExecutionMode, ProcessingJobArtifact,
    ProcessingJobExecutionSummary, ProcessingJobPlanSummary, ProcessingJobProgress,
    ProcessingJobStageClassificationSummary, ProcessingJobState, ProcessingJobStatus,
    ProcessingPipelineSpec, ProcessingSchedulerReason,
};
use ophiolite_seismic_runtime::{
    ExecutionPlan, ExecutionPriorityClass, ExecutionStageKind, MemoryCostClass,
    operator_execution_traits_for_pipeline_spec,
};

pub struct ProcessingJobRecord {
    status: Mutex<ProcessingJobStatus>,
    plan: Option<ExecutionPlan>,
    cancel_requested: AtomicBool,
}

pub struct BatchExecutionGate {
    max_active_jobs: usize,
    active_jobs: Mutex<usize>,
    cv: Condvar,
}

impl BatchExecutionGate {
    fn new(max_active_jobs: usize) -> Arc<Self> {
        Arc::new(Self {
            max_active_jobs: max_active_jobs.max(1),
            active_jobs: Mutex::new(0),
            cv: Condvar::new(),
        })
    }

    fn acquire(self: &Arc<Self>, record: &ProcessingJobRecord) -> Option<BatchExecutionPermit> {
        let mut active_jobs = self
            .active_jobs
            .lock()
            .expect("batch execution gate mutex poisoned");
        loop {
            if record.cancel_requested()
                || matches!(record.snapshot().state, ProcessingJobState::Cancelled)
            {
                return None;
            }
            if *active_jobs < self.max_active_jobs {
                *active_jobs += 1;
                return Some(BatchExecutionPermit {
                    gate: Arc::clone(self),
                });
            }
            let (next_active_jobs, _) = self
                .cv
                .wait_timeout(active_jobs, Duration::from_millis(100))
                .expect("batch execution gate mutex poisoned");
            active_jobs = next_active_jobs;
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
        self.gate.cv.notify_one();
    }
}

impl ProcessingJobRecord {
    pub fn new(status: ProcessingJobStatus, plan: Option<ExecutionPlan>) -> Self {
        Self {
            status: Mutex::new(status),
            plan,
            cancel_requested: AtomicBool::new(false),
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

    pub fn mark_running(&self, current_stage_label: Option<String>) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Running;
            status.current_stage_label = current_stage_label.clone();
            status.updated_at_unix_s = unix_timestamp_s();
        })
    }

    pub fn mark_progress(
        &self,
        completed: usize,
        total: usize,
        current_stage_label: Option<&str>,
    ) -> ProcessingJobStatus {
        self.update(|status| {
            status.progress = ProcessingJobProgress { completed, total };
            status.current_stage_label = current_stage_label.map(str::to_string);
            status.updated_at_unix_s = unix_timestamp_s();
        })
    }

    pub fn push_artifact(&self, artifact: ProcessingJobArtifact) -> ProcessingJobStatus {
        self.update(|status| {
            status.artifacts.push(artifact.clone());
            status.updated_at_unix_s = unix_timestamp_s();
        })
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

    pub fn mark_completed(&self, output_store_path: String) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Completed;
            status.output_store_path = Some(output_store_path.clone());
            status.current_stage_label = None;
            if let Some(summary) = status.execution_summary.as_mut() {
                summary.active_partitions = 0;
            }
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = None;
        })
    }

    pub fn mark_failed(&self, message: String) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Failed;
            status.current_stage_label = None;
            if let Some(summary) = status.execution_summary.as_mut() {
                summary.active_partitions = 0;
            }
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = Some(message.clone());
        })
    }

    pub fn mark_cancelled(&self) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Cancelled;
            status.current_stage_label = None;
            if let Some(summary) = status.execution_summary.as_mut() {
                summary.active_partitions = 0;
            }
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = None;
        })
    }

    pub fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::Relaxed);
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

struct ScheduledTask(Option<Box<dyn FnOnce() + Send + 'static>>);

impl ScheduledTask {
    fn new<F>(task: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self(Some(Box::new(task)))
    }

    fn run(mut self) {
        if let Some(task) = self.0.take() {
            task();
        }
    }
}

#[derive(Default)]
struct SchedulerQueues {
    interactive_preview: VecDeque<ScheduledTask>,
    foreground_materialize: VecDeque<ScheduledTask>,
    background_batch: VecDeque<ScheduledTask>,
    shutdown: bool,
}

impl SchedulerQueues {
    fn push(&mut self, priority: ExecutionPriorityClass, task: ScheduledTask) {
        match priority {
            ExecutionPriorityClass::InteractivePreview => self.interactive_preview.push_back(task),
            ExecutionPriorityClass::ForegroundMaterialize => {
                self.foreground_materialize.push_back(task)
            }
            ExecutionPriorityClass::BackgroundBatch => self.background_batch.push_back(task),
        }
    }

    fn pop_next(&mut self) -> Option<ScheduledTask> {
        self.interactive_preview
            .pop_front()
            .or_else(|| self.foreground_materialize.pop_front())
            .or_else(|| self.background_batch.pop_front())
    }
}

struct SharedScheduler {
    queues: Mutex<SchedulerQueues>,
    cv: Condvar,
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

    pub fn submit<F>(&self, priority: ExecutionPriorityClass, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut queues = self
            .shared
            .queues
            .lock()
            .expect("execution scheduler mutex poisoned");
        queues.push(priority, ScheduledTask::new(task));
        self.shared.cv.notify_one();
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
                if let Some(task) = queues.pop_next() {
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
        task.run();
    }
}

pub struct ProcessingExecutionService {
    scheduler: ExecutionScheduler,
    scheduler_worker_count: usize,
    global_batch_cap: usize,
    job_counter: AtomicU64,
    batch_counter: AtomicU64,
    jobs: Mutex<HashMap<String, Arc<ProcessingJobRecord>>>,
    batches: Mutex<HashMap<String, Arc<ProcessingBatchRecord>>>,
}

impl ProcessingExecutionService {
    pub fn new(max_active_jobs: usize) -> Self {
        let scheduler_worker_count = max_active_jobs.max(1);
        let global_batch_cap = global_batch_cap_from_env().unwrap_or(scheduler_worker_count);
        Self {
            scheduler: ExecutionScheduler::new(scheduler_worker_count),
            scheduler_worker_count,
            global_batch_cap: global_batch_cap.max(1),
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
        self.scheduler.submit(priority, move || {
            let _permit = match batch_gate {
                Some(gate) => gate.acquire(&record),
                None => None,
            };
            if record.cancel_requested()
                || matches!(record.snapshot().state, ProcessingJobState::Cancelled)
            {
                return;
            }
            task(record);
        });
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
        let plan_summary = plan.as_ref().map(processing_job_plan_summary);
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
            plan_summary,
            execution_summary: None,
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
        let plan_summary = plan.as_ref().map(processing_job_plan_summary);
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
            plan_summary,
            execution_summary: None,
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

    pub fn cancel_job(&self, job_id: &str) -> Result<ProcessingJobStatus, String> {
        let record = self.job_record(job_id)?;
        record.request_cancel();
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

fn processing_job_plan_summary(plan: &ExecutionPlan) -> ProcessingJobPlanSummary {
    ProcessingJobPlanSummary {
        plan_id: plan.plan_id.clone(),
        planning_mode: match plan.planning_mode {
            ophiolite_seismic_runtime::PlanningMode::InteractivePreview => {
                "interactive_preview".to_string()
            }
            ophiolite_seismic_runtime::PlanningMode::ForegroundMaterialize => {
                "foreground_materialize".to_string()
            }
            ophiolite_seismic_runtime::PlanningMode::BackgroundBatch => {
                "background_batch".to_string()
            }
        },
        stage_count: plan.stages.len(),
        stage_labels: plan.stages.iter().map(summarize_stage_label).collect(),
        expected_partition_count: plan.scheduler_hints.expected_partition_count,
        max_active_partitions: plan.scheduler_hints.max_active_partitions,
        stage_partition_summaries: plan.stages.iter().map(summarize_stage_partition).collect(),
        max_memory_cost_class: format_cost_class(plan.plan_summary.max_memory_cost_class),
        max_cpu_cost_class: format_cpu_cost_class(plan.plan_summary.max_cpu_cost_class),
        max_io_cost_class: format_io_cost_class(plan.plan_summary.max_io_cost_class),
        min_parallel_efficiency_class: format_parallel_efficiency_class(
            plan.plan_summary.min_parallel_efficiency_class,
        ),
        combined_cpu_weight: plan.plan_summary.combined_cpu_weight,
        combined_io_weight: plan.plan_summary.combined_io_weight,
        stage_classification_summaries: plan
            .stages
            .iter()
            .map(processing_job_stage_classification_summary)
            .collect(),
    }
}

fn processing_job_stage_classification_summary(
    stage: &ophiolite_seismic_runtime::ExecutionStage,
) -> ProcessingJobStageClassificationSummary {
    ProcessingJobStageClassificationSummary {
        stage_label: summarize_stage_label(stage),
        max_memory_cost_class: format_cost_class(stage.classification.max_memory_cost_class),
        max_cpu_cost_class: format_cpu_cost_class(stage.classification.max_cpu_cost_class),
        max_io_cost_class: format_io_cost_class(stage.classification.max_io_cost_class),
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

fn summarize_stage_label(stage: &ophiolite_seismic_runtime::ExecutionStage) -> String {
    let action = match stage.stage_kind {
        ExecutionStageKind::Compute => "Compute",
        ExecutionStageKind::Checkpoint => "Checkpoint",
        ExecutionStageKind::ReuseArtifact => "Reuse Artifact",
        ExecutionStageKind::FinalizeOutput => "Finalize Output",
    };
    match stage.pipeline_segment.as_ref() {
        Some(segment) => {
            let family = match segment.family {
                ophiolite_seismic::ProcessingPipelineFamily::TraceLocal => "trace-local",
                ophiolite_seismic::ProcessingPipelineFamily::PostStackNeighborhood => {
                    "post-stack neighborhood"
                }
                ophiolite_seismic::ProcessingPipelineFamily::Subvolume => "subvolume",
                ophiolite_seismic::ProcessingPipelineFamily::Gather => "gather",
            };
            let steps = if segment.start_step_index == segment.end_step_index {
                format!("step {}", segment.end_step_index + 1)
            } else {
                format!(
                    "steps {}-{}",
                    segment.start_step_index + 1,
                    segment.end_step_index + 1
                )
            };
            format!("{action}: {family} {steps}")
        }
        None => format!("{action}: {}", stage.output_artifact_id),
    }
}

fn summarize_stage_partition(stage: &ophiolite_seismic_runtime::ExecutionStage) -> String {
    let family = match stage.partition_spec.family {
        ophiolite_seismic_runtime::PartitionFamily::TileGroup => "tile_group",
        ophiolite_seismic_runtime::PartitionFamily::Section => "section",
        ophiolite_seismic_runtime::PartitionFamily::GatherGroup => "gather_group",
        ophiolite_seismic_runtime::PartitionFamily::FullVolume => "full_volume",
    };
    let count = stage
        .expected_partition_count
        .map(|value| format!(" x{value}"))
        .unwrap_or_default();
    let target = stage
        .partition_spec
        .target_bytes
        .map(|bytes| format!(" (~{} MiB target)", bytes / (1024 * 1024)))
        .unwrap_or_default();
    format!("{family}{count}{target}")
}

fn format_cost_class(cost: MemoryCostClass) -> String {
    match cost {
        MemoryCostClass::Low => "low".to_string(),
        MemoryCostClass::Medium => "medium".to_string(),
        MemoryCostClass::High => "high".to_string(),
    }
}

fn format_cpu_cost_class(cost: ophiolite_seismic_runtime::CpuCostClass) -> String {
    match cost {
        ophiolite_seismic_runtime::CpuCostClass::Low => "low".to_string(),
        ophiolite_seismic_runtime::CpuCostClass::Medium => "medium".to_string(),
        ophiolite_seismic_runtime::CpuCostClass::High => "high".to_string(),
    }
}

fn format_io_cost_class(cost: ophiolite_seismic_runtime::IoCostClass) -> String {
    match cost {
        ophiolite_seismic_runtime::IoCostClass::Low => "low".to_string(),
        ophiolite_seismic_runtime::IoCostClass::Medium => "medium".to_string(),
        ophiolite_seismic_runtime::IoCostClass::High => "high".to_string(),
    }
}

fn format_parallel_efficiency_class(
    efficiency: ophiolite_seismic_runtime::ParallelEfficiencyClass,
) -> String {
    match efficiency {
        ophiolite_seismic_runtime::ParallelEfficiencyClass::High => "high".to_string(),
        ophiolite_seismic_runtime::ParallelEfficiencyClass::Medium => "medium".to_string(),
        ophiolite_seismic_runtime::ParallelEfficiencyClass::Low => "low".to_string(),
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

fn memory_cost_class_rank(cost: MemoryCostClass) -> usize {
    match cost {
        MemoryCostClass::Low => 0,
        MemoryCostClass::Medium => 1,
        MemoryCostClass::High => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{
        PostStackNeighborhoodProcessingOperation, PostStackNeighborhoodProcessingPipeline,
        PostStackNeighborhoodWindow, SeismicLayout, TraceLocalProcessingOperation,
        TraceLocalProcessingPipeline, TraceLocalProcessingStep,
    };
    use ophiolite_seismic_runtime::{PlanProcessingRequest, PlanningMode, build_execution_plan};

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
        assert_eq!(batch.effective_max_active_jobs, 2);
        assert_eq!(batch.execution_mode, ProcessingExecutionMode::Custom);
        assert_eq!(
            batch.scheduler_reason,
            ProcessingSchedulerReason::UserRequested
        );
    }

    #[test]
    fn planned_jobs_capture_plan_summary_on_status() {
        let service = ProcessingExecutionService::new(1);
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
            store_path: "input.tbvol".to_string(),
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
            "input.tbvol".to_string(),
            Some("output.tbvol".to_string()),
            pipeline,
            Some(plan),
        );

        let summary = status
            .plan_summary
            .expect("planned jobs should expose summary");
        assert_eq!(summary.planning_mode, "foreground_materialize");
        assert_eq!(summary.stage_count, 2);
        assert_eq!(summary.stage_labels.len(), 2);
        assert!(summary.stage_labels[0].contains("Checkpoint"));
        assert!(summary.stage_labels[1].contains("Finalize Output"));
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
