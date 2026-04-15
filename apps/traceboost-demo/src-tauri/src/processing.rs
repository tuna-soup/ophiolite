use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use seis_runtime::{
    ProcessingJobArtifact, ProcessingJobProgress, ProcessingJobState, ProcessingJobStatus,
    ProcessingPipelineSpec, TraceLocalProcessingPreset,
};

pub(crate) struct ProcessingJobRecord {
    status: Mutex<ProcessingJobStatus>,
    cancel_requested: AtomicBool,
}

impl ProcessingJobRecord {
    pub(crate) fn new(status: ProcessingJobStatus) -> Self {
        Self {
            status: Mutex::new(status),
            cancel_requested: AtomicBool::new(false),
        }
    }

    pub(crate) fn snapshot(&self) -> ProcessingJobStatus {
        self.status
            .lock()
            .expect("processing job status mutex poisoned")
            .clone()
    }

    pub(crate) fn mark_running(&self, current_stage_label: Option<String>) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Running;
            status.current_stage_label = current_stage_label.clone();
            status.updated_at_unix_s = unix_timestamp_s();
        })
    }

    pub(crate) fn mark_progress(
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

    pub(crate) fn push_artifact(&self, artifact: ProcessingJobArtifact) -> ProcessingJobStatus {
        self.update(|status| {
            status.artifacts.push(artifact.clone());
            status.updated_at_unix_s = unix_timestamp_s();
        })
    }

    pub(crate) fn mark_completed(&self, output_store_path: String) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Completed;
            status.output_store_path = Some(output_store_path.clone());
            status.current_stage_label = None;
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = None;
        })
    }

    pub(crate) fn mark_failed(&self, message: String) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Failed;
            status.current_stage_label = None;
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = Some(message.clone());
        })
    }

    pub(crate) fn mark_cancelled(&self) -> ProcessingJobStatus {
        self.update(|status| {
            status.state = ProcessingJobState::Cancelled;
            status.current_stage_label = None;
            status.updated_at_unix_s = unix_timestamp_s();
            status.error_message = None;
        })
    }

    pub(crate) fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::Relaxed);
    }

    pub(crate) fn cancel_requested(&self) -> bool {
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

pub struct ProcessingState {
    pipeline_presets_dir: PathBuf,
    job_counter: AtomicU64,
    jobs: Mutex<HashMap<String, Arc<ProcessingJobRecord>>>,
}

impl ProcessingState {
    pub fn initialize(pipeline_presets_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(pipeline_presets_dir).map_err(|error| error.to_string())?;
        Ok(Self {
            pipeline_presets_dir: pipeline_presets_dir.to_path_buf(),
            job_counter: AtomicU64::new(0),
            jobs: Mutex::new(HashMap::new()),
        })
    }

    pub fn enqueue_job(
        &self,
        input_store_path: String,
        output_store_path: Option<String>,
        pipeline: ProcessingPipelineSpec,
    ) -> ProcessingJobStatus {
        let created_at_unix_s = unix_timestamp_s();
        let job_number = self.job_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let job_id = format!("processing-{created_at_unix_s}-{job_number:04}");
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
            created_at_unix_s,
            updated_at_unix_s: created_at_unix_s,
            error_message: None,
        };
        let record = Arc::new(ProcessingJobRecord::new(status.clone()));
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
        artifacts: Vec<ProcessingJobArtifact>,
    ) -> ProcessingJobStatus {
        let created_at_unix_s = unix_timestamp_s();
        let job_number = self.job_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let job_id = format!("processing-{created_at_unix_s}-{job_number:04}");
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
            created_at_unix_s,
            updated_at_unix_s: created_at_unix_s,
            error_message: None,
        };
        let record = Arc::new(ProcessingJobRecord::new(status.clone()));
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

    pub fn list_presets(&self) -> Result<Vec<TraceLocalProcessingPreset>, String> {
        let mut presets = Vec::new();
        for entry in fs::read_dir(&self.pipeline_presets_dir).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let preset = serde_json::from_slice::<TraceLocalProcessingPreset>(
                &fs::read(&path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            presets.push(preset);
        }
        presets.sort_by(|left, right| left.preset_id.cmp(&right.preset_id));
        Ok(presets)
    }

    pub fn save_preset(
        &self,
        preset: TraceLocalProcessingPreset,
    ) -> Result<TraceLocalProcessingPreset, String> {
        if preset.preset_id.trim().is_empty() {
            return Err("Processing preset id must not be empty".to_string());
        }
        let now = unix_timestamp_s();
        let preset = TraceLocalProcessingPreset {
            created_at_unix_s: if preset.created_at_unix_s == 0 {
                now
            } else {
                preset.created_at_unix_s
            },
            updated_at_unix_s: now,
            ..preset
        };
        let path = self.preset_path(&preset.preset_id);
        let json = serde_json::to_vec_pretty(&preset).map_err(|error| error.to_string())?;
        fs::write(path, json).map_err(|error| error.to_string())?;
        Ok(preset)
    }

    pub fn delete_preset(&self, preset_id: &str) -> Result<bool, String> {
        let path = self.preset_path(preset_id);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path).map_err(|error| error.to_string())?;
        Ok(true)
    }

    fn preset_path(&self, preset_id: &str) -> PathBuf {
        self.pipeline_presets_dir.join(format!("{preset_id}.json"))
    }
}

pub fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) use ProcessingJobRecord as JobRecord;
