use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const WORKFLOW_RECIPE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRecipeValidationError {
    pub message: String,
}

impl WorkflowRecipeValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRecipe {
    pub schema_version: u32,
    pub recipe_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub dataset_inputs: Vec<WorkflowDatasetInput>,
    #[serde(default)]
    pub steps: Vec<WorkflowRecipeStep>,
    #[serde(default)]
    pub expected_outputs: Vec<WorkflowExpectedOutput>,
    #[serde(default)]
    pub assertions: Vec<WorkflowAssertion>,
}

impl WorkflowRecipe {
    pub fn validate(&self) -> Result<(), Vec<WorkflowRecipeValidationError>> {
        let mut errors = Vec::new();

        if self.schema_version != WORKFLOW_RECIPE_SCHEMA_VERSION {
            errors.push(WorkflowRecipeValidationError::new(format!(
                "invalid schema_version {}; expected {}",
                self.schema_version, WORKFLOW_RECIPE_SCHEMA_VERSION
            )));
        }
        if self.recipe_id.trim().is_empty() {
            errors.push(WorkflowRecipeValidationError::new(
                "recipe_id must not be empty",
            ));
        }
        if self.name.trim().is_empty() {
            errors.push(WorkflowRecipeValidationError::new("name must not be empty"));
        }

        let mut dataset_ids = HashSet::new();
        for input in &self.dataset_inputs {
            if input.dataset_id.trim().is_empty() {
                errors.push(WorkflowRecipeValidationError::new(
                    "dataset input id must not be empty",
                ));
            } else if !dataset_ids.insert(input.dataset_id.as_str()) {
                errors.push(WorkflowRecipeValidationError::new(format!(
                    "duplicate dataset input id '{}'",
                    input.dataset_id
                )));
            }
        }

        let mut step_ids = HashSet::new();
        for step in &self.steps {
            if step.step_id.trim().is_empty() {
                errors.push(WorkflowRecipeValidationError::new(
                    "step_id must not be empty",
                ));
            } else if !step_ids.insert(step.step_id.as_str()) {
                errors.push(WorkflowRecipeValidationError::new(format!(
                    "duplicate step id '{}'",
                    step.step_id
                )));
            }
        }

        for step in &self.steps {
            for dependency in &step.depends_on {
                if dependency.trim().is_empty() {
                    errors.push(WorkflowRecipeValidationError::new(format!(
                        "step '{}' has an empty dependency id",
                        step.step_id
                    )));
                } else if !step_ids.contains(dependency.as_str()) {
                    errors.push(WorkflowRecipeValidationError::new(format!(
                        "step '{}' depends on missing step '{}'",
                        step.step_id, dependency
                    )));
                }
            }

            if matches!(
                step.step_kind,
                WorkflowRecipeStepKind::Preflight | WorkflowRecipeStepKind::Import
            ) {
                for dataset_ref in &step.dataset_refs {
                    if dataset_ref.trim().is_empty() {
                        errors.push(WorkflowRecipeValidationError::new(format!(
                            "step '{}' has an empty dataset ref",
                            step.step_id
                        )));
                    } else if !dataset_ids.contains(dataset_ref.as_str()) {
                        errors.push(WorkflowRecipeValidationError::new(format!(
                            "step '{}' references unknown dataset '{}'",
                            step.step_id, dataset_ref
                        )));
                    }
                }
            }
        }

        validate_expected_outputs("recipe", &self.expected_outputs, &mut errors);
        validate_assertions("recipe", &self.assertions, &mut errors);
        for step in &self.steps {
            validate_expected_outputs(
                &format!("step '{}'", step.step_id),
                &step.expected_outputs,
                &mut errors,
            );
            validate_assertions(
                &format!("step '{}'", step.step_id),
                &step.assertions,
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

fn validate_expected_outputs(
    owner: &str,
    expected_outputs: &[WorkflowExpectedOutput],
    errors: &mut Vec<WorkflowRecipeValidationError>,
) {
    for expected_output in expected_outputs {
        if expected_output.output_id.trim().is_empty() {
            errors.push(WorkflowRecipeValidationError::new(format!(
                "{owner} expected output id must not be empty"
            )));
        }
    }
}

fn validate_assertions(
    owner: &str,
    assertions: &[WorkflowAssertion],
    errors: &mut Vec<WorkflowRecipeValidationError>,
) {
    for assertion in assertions {
        if assertion.assertion_id.trim().is_empty() {
            errors.push(WorkflowRecipeValidationError::new(format!(
                "{owner} assertion_id must not be empty"
            )));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDatasetInput {
    pub dataset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRecipeStepKind {
    Preflight,
    Import,
    Processing,
    Export,
    Analysis,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRecipeStep {
    pub step_id: String,
    pub name: String,
    pub step_kind: WorkflowRecipeStepKind,
    pub operation: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub dataset_refs: Vec<String>,
    #[serde(default)]
    pub request: Value,
    #[serde(default)]
    pub expected_outputs: Vec<WorkflowExpectedOutput>,
    #[serde(default)]
    pub assertions: Vec<WorkflowAssertion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExpectedOutput {
    pub output_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_kind: Option<String>,
    #[serde(default)]
    pub spec: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowAssertion {
    pub assertion_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub assertion_kind: String,
    #[serde(default)]
    pub spec: Value,
}
