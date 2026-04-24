use std::fs;
use std::path::{Path, PathBuf};

use ophiolite_seismic_execution::{
    BatchExecutionGate, BatchExecutionPolicyDecision, ProcessingExecutionService,
    ProcessingJobRecord,
};
use seis_runtime::{
    ExecutionPlan, ExecutionPriorityClass, FrequencyPhaseMode, FrequencyWindowShape,
    NeighborhoodDipOutput, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, PostStackNeighborhoodWindow,
    ProcessingBatchItemRequest, ProcessingBatchStatus, ProcessingExecutionMode,
    ProcessingJobArtifact, ProcessingJobRuntimeState, ProcessingJobStatus, ProcessingPipelineSpec,
    ProcessingPreset, ProcessingRuntimeEvent, TraceLocalProcessingOperation,
    TraceLocalProcessingPipeline, TraceLocalProcessingStep,
};
use serde::{Deserialize, Serialize};

pub struct ProcessingState {
    pipeline_presets_dir: PathBuf,
    execution: ProcessingExecutionService,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LegacyTraceLocalProcessingPreset {
    preset_id: String,
    pipeline: TraceLocalProcessingPipeline,
    created_at_unix_s: u64,
    updated_at_unix_s: u64,
}

const BUILTIN_PRESET_BOOTSTRAP_MARKER_FILENAME: &str = ".builtin-presets-seeded-v1";

impl ProcessingState {
    pub fn initialize(pipeline_presets_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(pipeline_presets_dir).map_err(|error| error.to_string())?;
        let max_active_jobs = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(4)
            .saturating_sub(1)
            .max(1);
        Ok(Self {
            pipeline_presets_dir: pipeline_presets_dir.to_path_buf(),
            execution: ProcessingExecutionService::new(max_active_jobs),
        })
    }

    pub fn enqueue_job<F>(
        &self,
        input_store_path: String,
        output_store_path: Option<String>,
        pipeline: ProcessingPipelineSpec,
        plan: Option<ExecutionPlan>,
        priority: ExecutionPriorityClass,
        batch_gate: Option<std::sync::Arc<BatchExecutionGate>>,
        task: F,
    ) -> ProcessingJobStatus
    where
        F: FnOnce(std::sync::Arc<ProcessingJobRecord>) + Send + 'static,
    {
        self.execution.enqueue_job(
            input_store_path,
            output_store_path,
            pipeline,
            plan,
            priority,
            batch_gate,
            task,
        )
    }

    pub fn enqueue_completed_job(
        &self,
        input_store_path: String,
        output_store_path: String,
        pipeline: ProcessingPipelineSpec,
        plan: Option<ExecutionPlan>,
        artifacts: Vec<ProcessingJobArtifact>,
    ) -> ProcessingJobStatus {
        self.execution.enqueue_completed_job(
            input_store_path,
            output_store_path,
            pipeline,
            plan,
            artifacts,
        )
    }

    pub fn job_status(&self, job_id: &str) -> Result<ProcessingJobStatus, String> {
        self.execution.job_status(job_id)
    }

    pub fn job_debug_plan(
        &self,
        job_id: &str,
    ) -> Result<Option<seis_runtime::InspectableProcessingPlan>, String> {
        self.execution.job_debug_plan(job_id)
    }

    pub fn job_runtime_state(&self, job_id: &str) -> Result<ProcessingJobRuntimeState, String> {
        self.execution.job_runtime_state(job_id)
    }

    pub fn job_runtime_events(
        &self,
        job_id: &str,
        after_seq: Option<u64>,
    ) -> Result<Vec<ProcessingRuntimeEvent>, String> {
        self.execution.job_runtime_events(job_id, after_seq)
    }

    pub fn cancel_job(&self, job_id: &str) -> Result<ProcessingJobStatus, String> {
        self.execution.cancel_job(job_id)
    }

    pub fn register_batch(
        &self,
        items: Vec<ProcessingBatchItemRequest>,
        job_ids: Vec<String>,
        pipeline: ProcessingPipelineSpec,
        policy: &BatchExecutionPolicyDecision,
    ) -> Result<ProcessingBatchStatus, String> {
        self.execution
            .register_batch(pipeline, items, job_ids, policy)
    }

    pub fn resolve_batch_execution_policy(
        &self,
        requested_max_active_jobs: Option<usize>,
        requested_execution_mode: Option<ProcessingExecutionMode>,
        pipeline: &ProcessingPipelineSpec,
        plan: Option<&ExecutionPlan>,
        priority: ExecutionPriorityClass,
    ) -> BatchExecutionPolicyDecision {
        self.execution.resolve_batch_execution_policy(
            requested_max_active_jobs,
            requested_execution_mode,
            pipeline,
            plan,
            priority,
        )
    }

    pub fn create_batch_gate(&self, max_active_jobs: usize) -> std::sync::Arc<BatchExecutionGate> {
        self.execution.create_batch_gate(max_active_jobs)
    }

    pub fn batch_status(&self, batch_id: &str) -> Result<ProcessingBatchStatus, String> {
        self.execution.batch_status(batch_id)
    }

    pub fn cancel_batch(&self, batch_id: &str) -> Result<ProcessingBatchStatus, String> {
        self.execution.cancel_batch(batch_id)
    }

    pub fn list_presets(&self) -> Result<Vec<ProcessingPreset>, String> {
        let mut presets = Vec::new();
        for entry in fs::read_dir(&self.pipeline_presets_dir).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let preset = self.read_preset_file(&path)?;
            presets.push(preset);
        }
        presets.sort_by(|left, right| left.preset_id.cmp(&right.preset_id));
        Ok(presets)
    }

    pub fn save_preset(&self, preset: ProcessingPreset) -> Result<ProcessingPreset, String> {
        if preset.preset_id.trim().is_empty() {
            return Err("Processing preset id must not be empty".to_string());
        }
        let now = unix_timestamp_s();
        let mut preset = ProcessingPreset {
            created_at_unix_s: if preset.created_at_unix_s == 0 {
                now
            } else {
                preset.created_at_unix_s
            },
            updated_at_unix_s: now,
            ..preset
        };
        preset
            .pipeline
            .set_preset_id(Some(preset.preset_id.clone()));
        let path = self.preset_path(&preset.preset_id);
        let json = serde_json::to_vec_pretty(&preset).map_err(|error| error.to_string())?;
        fs::write(path, json).map_err(|error| error.to_string())?;
        Ok(preset)
    }

    pub fn seed_builtin_presets(&self) -> Result<(), String> {
        let marker_path = self.builtin_preset_bootstrap_marker_path();
        if marker_path.exists() {
            return Ok(());
        }

        for preset in builtin_processing_presets() {
            let path = self.preset_path(&preset.preset_id);
            if !path.exists() {
                self.save_preset(preset)?;
            }
        }

        fs::write(marker_path, b"builtin-presets-seeded-v1\n").map_err(|error| error.to_string())
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

    fn builtin_preset_bootstrap_marker_path(&self) -> PathBuf {
        self.pipeline_presets_dir
            .join(BUILTIN_PRESET_BOOTSTRAP_MARKER_FILENAME)
    }

    fn read_preset_file(&self, path: &Path) -> Result<ProcessingPreset, String> {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        if let Ok(preset) = serde_json::from_slice::<ProcessingPreset>(&bytes) {
            return Ok(preset);
        }
        let legacy_preset = serde_json::from_slice::<LegacyTraceLocalProcessingPreset>(&bytes)
            .map_err(|error| error.to_string())?;
        Ok(ProcessingPreset {
            preset_id: legacy_preset.preset_id.clone(),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: legacy_preset.pipeline,
            },
            created_at_unix_s: legacy_preset.created_at_unix_s,
            updated_at_unix_s: legacy_preset.updated_at_unix_s,
        })
    }
}

pub fn unix_timestamp_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) type JobRecord = ProcessingJobRecord;

fn trace_local_step(operation: TraceLocalProcessingOperation) -> TraceLocalProcessingStep {
    TraceLocalProcessingStep {
        operation,
        checkpoint: false,
    }
}

fn bandpass_10_20_60_80() -> TraceLocalProcessingOperation {
    TraceLocalProcessingOperation::BandpassFilter {
        f1_hz: 10.0,
        f2_hz: 20.0,
        f3_hz: 60.0,
        f4_hz: 80.0,
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn trace_local_pipeline_named(
    name: &str,
    steps: Vec<TraceLocalProcessingStep>,
) -> TraceLocalProcessingPipeline {
    TraceLocalProcessingPipeline {
        schema_version: 2,
        revision: 1,
        preset_id: None,
        name: Some(name.to_string()),
        description: None,
        steps,
    }
}

fn neighborhood_window(
    gate_ms: f32,
    inline_stepout: usize,
    xline_stepout: usize,
) -> PostStackNeighborhoodWindow {
    PostStackNeighborhoodWindow {
        gate_ms,
        inline_stepout,
        xline_stepout,
    }
}

fn neighborhood_pipeline_named(
    name: &str,
    trace_local_pipeline: Option<TraceLocalProcessingPipeline>,
    operation: PostStackNeighborhoodProcessingOperation,
) -> PostStackNeighborhoodProcessingPipeline {
    PostStackNeighborhoodProcessingPipeline {
        schema_version: 1,
        revision: 1,
        preset_id: None,
        name: Some(name.to_string()),
        description: None,
        trace_local_pipeline,
        operations: vec![operation],
    }
}

fn builtin_processing_presets() -> Vec<ProcessingPreset> {
    vec![
        ProcessingPreset {
            preset_id: "builtin-trace-agc-rms-250ms".to_string(),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: trace_local_pipeline_named(
                    "AGC RMS 250 ms",
                    vec![trace_local_step(TraceLocalProcessingOperation::AgcRms {
                        window_ms: 250.0,
                    })],
                ),
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        },
        ProcessingPreset {
            preset_id: "builtin-trace-bandpass-10-20-60-80".to_string(),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: trace_local_pipeline_named(
                    "Bandpass 10-20-60-80 Hz",
                    vec![trace_local_step(bandpass_10_20_60_80())],
                ),
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        },
        ProcessingPreset {
            preset_id: "builtin-trace-envelope".to_string(),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: trace_local_pipeline_named(
                    "Envelope",
                    vec![trace_local_step(TraceLocalProcessingOperation::Envelope)],
                ),
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        },
        ProcessingPreset {
            preset_id: "builtin-neighborhood-similarity-tight".to_string(),
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: neighborhood_pipeline_named(
                    "Similarity Tight",
                    None,
                    PostStackNeighborhoodProcessingOperation::Similarity {
                        window: neighborhood_window(24.0, 1, 1),
                    },
                ),
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        },
        ProcessingPreset {
            preset_id: "builtin-neighborhood-similarity-balanced".to_string(),
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: neighborhood_pipeline_named(
                    "Similarity Balanced",
                    Some(trace_local_pipeline_named(
                        "Bandpass 10-20-60-80 Hz",
                        vec![trace_local_step(bandpass_10_20_60_80())],
                    )),
                    PostStackNeighborhoodProcessingOperation::Similarity {
                        window: neighborhood_window(32.0, 2, 2),
                    },
                ),
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        },
        ProcessingPreset {
            preset_id: "builtin-neighborhood-dip-balanced".to_string(),
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: neighborhood_pipeline_named(
                    "Dip Balanced",
                    Some(trace_local_pipeline_named(
                        "Bandpass 10-20-60-80 Hz",
                        vec![trace_local_step(bandpass_10_20_60_80())],
                    )),
                    PostStackNeighborhoodProcessingOperation::Dip {
                        window: neighborhood_window(32.0, 2, 2),
                        output: NeighborhoodDipOutput::Inline,
                    },
                ),
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("ophiolite-processing-preset-tests-{suffix}"))
    }

    fn empty_trace_local_pipeline(preset_id: Option<&str>) -> TraceLocalProcessingPipeline {
        TraceLocalProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: preset_id.map(str::to_string),
            name: Some("Example".to_string()),
            description: None,
            steps: Vec::new(),
        }
    }

    #[test]
    fn list_presets_reads_legacy_trace_local_preset_files() {
        let dir = unique_temp_dir();
        let state = ProcessingState::initialize(&dir).expect("state");
        let legacy = LegacyTraceLocalProcessingPreset {
            preset_id: "legacy-trace-local".to_string(),
            pipeline: empty_trace_local_pipeline(Some("legacy-trace-local")),
            created_at_unix_s: 11,
            updated_at_unix_s: 22,
        };
        let path = state.preset_path(&legacy.preset_id);
        fs::write(
            &path,
            serde_json::to_vec_pretty(&legacy).expect("legacy preset json"),
        )
        .expect("write legacy preset");

        let presets = state.list_presets().expect("list presets");

        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].preset_id, legacy.preset_id);
        assert_eq!(presets[0].created_at_unix_s, 11);
        assert_eq!(presets[0].updated_at_unix_s, 22);
        assert!(matches!(
            &presets[0].pipeline,
            ProcessingPipelineSpec::TraceLocal { pipeline }
                if pipeline.preset_id.as_deref() == Some("legacy-trace-local")
        ));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_preset_normalizes_nested_pipeline_preset_ids() {
        let dir = unique_temp_dir();
        let state = ProcessingState::initialize(&dir).expect("state");
        let preset = ProcessingPreset {
            preset_id: "neighborhood-template".to_string(),
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: seis_runtime::PostStackNeighborhoodProcessingPipeline {
                    schema_version: 1,
                    revision: 3,
                    preset_id: None,
                    name: Some("Neighborhood Template".to_string()),
                    description: None,
                    trace_local_pipeline: Some(empty_trace_local_pipeline(None)),
                    operations: Vec::new(),
                },
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        };

        let saved = state.save_preset(preset).expect("save preset");

        match &saved.pipeline {
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
                assert_eq!(pipeline.preset_id.as_deref(), Some("neighborhood-template"));
                assert_eq!(
                    pipeline
                        .trace_local_pipeline
                        .as_ref()
                        .and_then(|trace_local| trace_local.preset_id.as_deref()),
                    Some("neighborhood-template")
                );
            }
            other => panic!("expected post-stack neighborhood preset, got {other:?}"),
        }

        let persisted = state.list_presets().expect("reload presets");
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].preset_id, "neighborhood-template");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn seed_builtin_presets_bootstraps_expected_templates_once() {
        let dir = unique_temp_dir();
        let state = ProcessingState::initialize(&dir).expect("state");

        state.seed_builtin_presets().expect("seed built-in presets");
        let presets = state.list_presets().expect("list presets");

        assert_eq!(presets.len(), 6);
        assert!(state.builtin_preset_bootstrap_marker_path().exists());

        let balanced_similarity = presets
            .iter()
            .find(|preset| preset.preset_id == "builtin-neighborhood-similarity-balanced")
            .expect("balanced similarity preset");
        match &balanced_similarity.pipeline {
            ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
                assert_eq!(pipeline.name.as_deref(), Some("Similarity Balanced"));
                assert_eq!(
                    pipeline.preset_id.as_deref(),
                    Some("builtin-neighborhood-similarity-balanced")
                );
                assert_eq!(
                    pipeline
                        .trace_local_pipeline
                        .as_ref()
                        .map(|trace_local| trace_local.steps.len()),
                    Some(1)
                );
                assert!(matches!(
                    pipeline.operations.first(),
                    Some(PostStackNeighborhoodProcessingOperation::Similarity { window })
                        if window.gate_ms == 32.0 && window.inline_stepout == 2 && window.xline_stepout == 2
                ));
            }
            other => panic!("expected neighborhood preset, got {other:?}"),
        }

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn seed_builtin_presets_does_not_recreate_deleted_templates_after_marker() {
        let dir = unique_temp_dir();
        let state = ProcessingState::initialize(&dir).expect("state");

        state.seed_builtin_presets().expect("seed built-in presets");
        assert!(
            state
                .delete_preset("builtin-neighborhood-similarity-tight")
                .expect("delete preset"),
            "expected built-in preset to exist before delete"
        );

        state
            .seed_builtin_presets()
            .expect("reseed built-in presets");
        let preset_ids = state
            .list_presets()
            .expect("list presets")
            .into_iter()
            .map(|preset| preset.preset_id)
            .collect::<Vec<_>>();

        assert_eq!(preset_ids.len(), 5);
        assert!(
            !preset_ids
                .iter()
                .any(|preset_id| preset_id == "builtin-neighborhood-similarity-tight")
        );

        let _ = fs::remove_dir_all(dir);
    }
}
