use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::workflow_recipe::{WorkflowRecipe, WorkflowRecipeValidationError};
use crate::workflow_report::{
    WORKFLOW_REPORT_SCHEMA_VERSION, WorkflowEnvironmentSummary, WorkflowEnvironmentVariable,
    WorkflowRunNote, WorkflowRunReport, WorkflowRunStatus, WorkflowStepRunRecord,
    WorkflowTimingSummary, WorkflowVersionSummary,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunWorkflowOptions {
    pub run_id: String,
    pub started_at: String,
    pub app_version: String,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub host: Option<String>,
    pub environment_variables: Vec<WorkflowEnvironmentVariable>,
}

#[derive(Debug)]
pub enum WorkflowRunnerError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidRecipe(Vec<WorkflowRecipeValidationError>),
    DependencyCycle(Vec<String>),
}

impl std::fmt::Display for WorkflowRunnerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "failed to read workflow recipe: {error}"),
            Self::Json(error) => write!(formatter, "failed to parse workflow recipe JSON: {error}"),
            Self::InvalidRecipe(errors) => {
                write!(formatter, "workflow recipe validation failed")?;
                for error in errors {
                    write!(formatter, "; {}", error.message)?;
                }
                Ok(())
            }
            Self::DependencyCycle(step_ids) => {
                write!(
                    formatter,
                    "workflow recipe contains a dependency cycle involving: {}",
                    step_ids.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for WorkflowRunnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::InvalidRecipe(_) | Self::DependencyCycle(_) => None,
        }
    }
}

impl From<std::io::Error> for WorkflowRunnerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for WorkflowRunnerError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn load_workflow_recipe_from_json_text(
    json_text: &str,
) -> Result<WorkflowRecipe, WorkflowRunnerError> {
    Ok(serde_json::from_str(json_text)?)
}

pub fn load_workflow_recipe_from_json_path(
    path: impl AsRef<Path>,
) -> Result<WorkflowRecipe, WorkflowRunnerError> {
    let json_text = fs::read_to_string(path)?;
    load_workflow_recipe_from_json_text(&json_text)
}

pub fn validate_workflow_recipe(recipe: &WorkflowRecipe) -> Result<(), WorkflowRunnerError> {
    recipe
        .validate()
        .map_err(WorkflowRunnerError::InvalidRecipe)
}

pub fn workflow_recipe_digest(recipe: &WorkflowRecipe) -> Result<String, WorkflowRunnerError> {
    stable_json_digest(recipe)
}

pub fn run_workflow_recipe(
    recipe: &WorkflowRecipe,
    options: RunWorkflowOptions,
) -> Result<WorkflowRunReport, WorkflowRunnerError> {
    validate_workflow_recipe(recipe)?;

    let ordered_step_indexes = dependency_order(recipe)?;
    let mut step_records = Vec::with_capacity(ordered_step_indexes.len());
    let mut report_warnings = Vec::new();
    let mut report_blockers = Vec::new();

    for step_index in ordered_step_indexes {
        let step = &recipe.steps[step_index];
        let warnings = notes_from_request_array(&step.request, "warnings");
        let blockers = notes_from_request_array(&step.request, "blockers");

        report_warnings.extend(warnings.clone());
        report_blockers.extend(blockers.clone());

        step_records.push(WorkflowStepRunRecord {
            step_id: step.step_id.clone(),
            status: WorkflowRunStatus::Succeeded,
            depends_on: step.depends_on.clone(),
            started_at: options.started_at.clone(),
            finished_at: None,
            request_digest: stable_value_digest(&step.request)?,
            response_digest: Some(stable_value_digest(&json!({
                "status": "succeeded",
                "step_id": &step.step_id,
            }))?),
            warnings,
            blockers,
            runtime_evidence: vec![json!({
                "operation": &step.operation,
                "step_kind": &step.step_kind,
            })],
            artifacts: Vec::new(),
            timings: zero_timing(),
        });
    }

    Ok(WorkflowRunReport {
        schema_version: WORKFLOW_REPORT_SCHEMA_VERSION,
        run_id: options.run_id,
        recipe_id: recipe.recipe_id.clone(),
        recipe_digest: workflow_recipe_digest(recipe)?,
        status: WorkflowRunStatus::Succeeded,
        started_at: options.started_at,
        finished_at: None,
        versions: WorkflowVersionSummary {
            traceboost_app: options.app_version,
            ophiolite: None,
            components: Vec::new(),
        },
        environment: WorkflowEnvironmentSummary {
            os: options.os,
            arch: options.arch,
            host: options.host,
            variables: options.environment_variables,
        },
        source_fingerprints: Vec::new(),
        steps: step_records,
        warnings: report_warnings,
        blockers: report_blockers,
        runtime_evidence: Vec::new(),
        artifacts: Vec::new(),
        assertions: Vec::new(),
        timings: zero_timing(),
    })
}

fn dependency_order(recipe: &WorkflowRecipe) -> Result<Vec<usize>, WorkflowRunnerError> {
    let step_index_by_id: HashMap<&str, usize> = recipe
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.step_id.as_str(), index))
        .collect();
    let mut pending_dependencies: BTreeMap<&str, HashSet<&str>> = recipe
        .steps
        .iter()
        .map(|step| {
            (
                step.step_id.as_str(),
                step.depends_on
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>(),
            )
        })
        .collect();
    let mut ready: Vec<&str> = recipe
        .steps
        .iter()
        .filter(|step| step.depends_on.is_empty())
        .map(|step| step.step_id.as_str())
        .collect();
    let mut ordered = Vec::with_capacity(recipe.steps.len());

    while let Some(step_id) = ready.pop() {
        if !pending_dependencies.contains_key(step_id) {
            continue;
        }
        pending_dependencies.remove(step_id);
        ordered.push(step_index_by_id[step_id]);

        for (candidate_step_id, dependencies) in &mut pending_dependencies {
            dependencies.remove(step_id);
            if dependencies.is_empty() {
                ready.push(candidate_step_id);
            }
        }
        ready.sort_by(|left, right| right.cmp(left));
        ready.dedup();
    }

    if !pending_dependencies.is_empty() {
        return Err(WorkflowRunnerError::DependencyCycle(
            pending_dependencies
                .keys()
                .map(|step_id| (*step_id).to_string())
                .collect(),
        ));
    }

    Ok(ordered)
}

fn notes_from_request_array(request: &Value, field_name: &str) -> Vec<WorkflowRunNote> {
    request
        .get(field_name)
        .and_then(Value::as_array)
        .map(|notes| notes.iter().map(note_from_value).collect())
        .unwrap_or_default()
}

fn note_from_value(value: &Value) -> WorkflowRunNote {
    if let Some(message) = value.as_str() {
        return WorkflowRunNote {
            message: message.to_string(),
            code: None,
            detail: None,
        };
    }

    if let Some(note) = value.as_object() {
        return WorkflowRunNote {
            message: note
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("workflow runner note")
                .to_string(),
            code: note
                .get("code")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            detail: note.get("detail").cloned(),
        };
    }

    WorkflowRunNote {
        message: value.to_string(),
        code: None,
        detail: Some(value.clone()),
    }
}

fn zero_timing() -> WorkflowTimingSummary {
    WorkflowTimingSummary {
        elapsed_ms: Some(0.0),
        stages: Vec::new(),
    }
}

fn stable_json_digest<T>(value: &T) -> Result<String, WorkflowRunnerError>
where
    T: serde::Serialize,
{
    let value = serde_json::to_value(value)?;
    stable_value_digest(&value)
}

fn stable_value_digest(value: &Value) -> Result<String, WorkflowRunnerError> {
    let canonical_value = canonicalize_json_value(value);
    let bytes = serde_json::to_vec_pretty(&canonical_value)?;
    Ok(format!("blake3:{}", blake3::hash(&bytes).to_hex()))
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json_value).collect()),
        Value::Object(object) => {
            let sorted_object = object
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize_json_value(value)))
                .collect::<BTreeMap<String, Value>>();

            Value::Object(sorted_object.into_iter().collect::<Map<String, Value>>())
        }
        _ => value.clone(),
    }
}
