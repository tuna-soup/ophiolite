use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const WORKFLOW_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowReportValidationError {
    pub message: String,
}

impl WorkflowReportValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRunReport {
    pub schema_version: u32,
    pub run_id: String,
    pub recipe_id: String,
    pub recipe_digest: String,
    pub status: WorkflowRunStatus,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub versions: WorkflowVersionSummary,
    pub environment: WorkflowEnvironmentSummary,
    #[serde(default)]
    pub source_fingerprints: Vec<WorkflowSourceFingerprint>,
    #[serde(default)]
    pub steps: Vec<WorkflowStepRunRecord>,
    #[serde(default)]
    pub warnings: Vec<WorkflowRunNote>,
    #[serde(default)]
    pub blockers: Vec<WorkflowRunNote>,
    #[serde(default)]
    pub runtime_evidence: Vec<Value>,
    #[serde(default)]
    pub artifacts: Vec<WorkflowArtifactRecord>,
    #[serde(default)]
    pub assertions: Vec<WorkflowAssertionRecord>,
    pub timings: WorkflowTimingSummary,
}

impl WorkflowRunReport {
    pub fn validate(&self) -> Result<(), Vec<WorkflowReportValidationError>> {
        let mut errors = Vec::new();

        if self.schema_version != WORKFLOW_REPORT_SCHEMA_VERSION {
            errors.push(WorkflowReportValidationError::new(format!(
                "invalid schema_version {}; expected {}",
                self.schema_version, WORKFLOW_REPORT_SCHEMA_VERSION
            )));
        }
        if self.run_id.trim().is_empty() {
            errors.push(WorkflowReportValidationError::new(
                "run_id must not be empty",
            ));
        }
        if self.recipe_id.trim().is_empty() {
            errors.push(WorkflowReportValidationError::new(
                "recipe_id must not be empty",
            ));
        }
        if self.recipe_digest.trim().is_empty() {
            errors.push(WorkflowReportValidationError::new(
                "recipe_digest must not be empty",
            ));
        }
        if self.started_at.trim().is_empty() {
            errors.push(WorkflowReportValidationError::new(
                "started_at must not be empty",
            ));
        }

        let mut step_ids = HashSet::new();
        for step in &self.steps {
            if step.step_id.trim().is_empty() {
                errors.push(WorkflowReportValidationError::new(
                    "step_id must not be empty",
                ));
            } else if !step_ids.insert(step.step_id.as_str()) {
                errors.push(WorkflowReportValidationError::new(format!(
                    "duplicate step id '{}'",
                    step.step_id
                )));
            }
        }

        for step in &self.steps {
            for dependency in &step.depends_on {
                if dependency.trim().is_empty() {
                    errors.push(WorkflowReportValidationError::new(format!(
                        "step '{}' has an empty dependency id",
                        step.step_id
                    )));
                } else if !step_ids.contains(dependency.as_str()) {
                    errors.push(WorkflowReportValidationError::new(format!(
                        "step '{}' depends on missing step '{}'",
                        step.step_id, dependency
                    )));
                }
            }
        }

        for source in &self.source_fingerprints {
            if source.source_id.trim().is_empty() {
                errors.push(WorkflowReportValidationError::new(
                    "source_id must not be empty",
                ));
            }
        }
        validate_artifacts("report", &self.artifacts, &mut errors);
        validate_assertions("report", &self.assertions, &mut errors);
        for step in &self.steps {
            validate_artifacts(
                &format!("step '{}'", step.step_id),
                &step.artifacts,
                &mut errors,
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn validate_artifacts(
    owner: &str,
    artifacts: &[WorkflowArtifactRecord],
    errors: &mut Vec<WorkflowReportValidationError>,
) {
    for artifact in artifacts {
        if artifact.artifact_id.trim().is_empty() {
            errors.push(WorkflowReportValidationError::new(format!(
                "{owner} artifact_id must not be empty"
            )));
        }
    }
}

fn validate_assertions(
    owner: &str,
    assertions: &[WorkflowAssertionRecord],
    errors: &mut Vec<WorkflowReportValidationError>,
) {
    for assertion in assertions {
        if assertion.assertion_id.trim().is_empty() {
            errors.push(WorkflowReportValidationError::new(format!(
                "{owner} assertion_id must not be empty"
            )));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowVersionSummary {
    pub traceboost_app: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ophiolite: Option<String>,
    #[serde(default)]
    pub components: Vec<WorkflowComponentVersion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowComponentVersion {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEnvironmentSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default)]
    pub variables: Vec<WorkflowEnvironmentVariable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEnvironmentVariable {
    pub name: String,
    pub value_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowSourceFingerprint {
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    pub digest: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowStepRunRecord {
    pub step_id: String,
    pub status: WorkflowRunStatus,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub request_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_digest: Option<String>,
    #[serde(default)]
    pub warnings: Vec<WorkflowRunNote>,
    #[serde(default)]
    pub blockers: Vec<WorkflowRunNote>,
    #[serde(default)]
    pub runtime_evidence: Vec<Value>,
    #[serde(default)]
    pub artifacts: Vec<WorkflowArtifactRecord>,
    pub timings: WorkflowTimingSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRunNote {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowArtifactRecord {
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowAssertionRecord {
    pub assertion_id: String,
    pub status: WorkflowRunStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default)]
    pub evidence: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTimingSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<f64>,
    #[serde(default)]
    pub stages: Vec<WorkflowTimingRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTimingRecord {
    pub name: String,
    pub elapsed_ms: f64,
}
