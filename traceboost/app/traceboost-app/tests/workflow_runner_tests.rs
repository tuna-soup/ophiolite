use serde_json::json;

mod workflow_recipe {
    pub use traceboost_app::workflow_recipe::*;
}

mod workflow_report {
    pub use traceboost_app::workflow_report::*;
}

#[path = "../src/workflow_runner.rs"]
mod workflow_runner;

use traceboost_app::workflow_recipe::{
    WORKFLOW_RECIPE_SCHEMA_VERSION, WorkflowDatasetInput, WorkflowRecipe, WorkflowRecipeStep,
    WorkflowRecipeStepKind,
};
use workflow_report::WorkflowRunStatus;
use workflow_runner::{
    RunWorkflowOptions, WorkflowRunnerError, load_workflow_recipe_from_json_path,
    load_workflow_recipe_from_json_text, run_workflow_recipe, workflow_recipe_digest,
};

fn valid_recipe() -> WorkflowRecipe {
    WorkflowRecipe {
        schema_version: WORKFLOW_RECIPE_SCHEMA_VERSION,
        recipe_id: "recipe-runner".to_string(),
        name: "Runner Smoke".to_string(),
        description: None,
        dataset_inputs: vec![WorkflowDatasetInput {
            dataset_id: "source-a".to_string(),
            name: Some("Source A".to_string()),
            source: Some("file:///tmp/source-a.sgy".to_string()),
            metadata: json!({"fixture": true}),
        }],
        steps: vec![
            WorkflowRecipeStep {
                step_id: "import".to_string(),
                name: "Import".to_string(),
                step_kind: WorkflowRecipeStepKind::Import,
                operation: "import_source".to_string(),
                depends_on: vec!["preflight".to_string()],
                dataset_refs: vec!["source-a".to_string()],
                request: json!({
                    "policy": "strict",
                    "warnings": ["stub warning"],
                    "blockers": [{"message": "stub blocker", "code": "stub"}]
                }),
                expected_outputs: Vec::new(),
                assertions: Vec::new(),
            },
            WorkflowRecipeStep {
                step_id: "preflight".to_string(),
                name: "Preflight".to_string(),
                step_kind: WorkflowRecipeStepKind::Preflight,
                operation: "preflight_source".to_string(),
                depends_on: Vec::new(),
                dataset_refs: vec!["source-a".to_string()],
                request: json!({"dataset_id": "source-a"}),
                expected_outputs: Vec::new(),
                assertions: Vec::new(),
            },
            WorkflowRecipeStep {
                step_id: "analyze".to_string(),
                name: "Analyze".to_string(),
                step_kind: WorkflowRecipeStepKind::Analysis,
                operation: "analyze_import".to_string(),
                depends_on: vec!["import".to_string()],
                dataset_refs: Vec::new(),
                request: json!({"mode": "summary"}),
                expected_outputs: Vec::new(),
                assertions: Vec::new(),
            },
        ],
        expected_outputs: Vec::new(),
        assertions: Vec::new(),
    }
}

fn options() -> RunWorkflowOptions {
    RunWorkflowOptions {
        run_id: "run-001".to_string(),
        started_at: "2026-04-30T10:00:00Z".to_string(),
        app_version: "0.1.0-test".to_string(),
        os: Some("test-os".to_string()),
        arch: Some("test-arch".to_string()),
        host: None,
        environment_variables: Vec::new(),
    }
}

#[test]
fn loads_recipe_from_json_text_and_path() {
    let recipe = valid_recipe();
    let json_text = serde_json::to_string_pretty(&recipe).expect("serialize recipe");
    let from_text = load_workflow_recipe_from_json_text(&json_text).expect("load recipe text");

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let recipe_path = temp_dir.path().join("recipe.json");
    std::fs::write(&recipe_path, json_text).expect("write recipe");
    let from_path = load_workflow_recipe_from_json_path(&recipe_path).expect("load recipe path");

    assert_eq!(from_text, recipe);
    assert_eq!(from_path, recipe);
}

#[test]
fn run_workflow_stub_executes_steps_in_dependency_order() {
    let recipe = valid_recipe();
    let expected_digest = workflow_recipe_digest(&recipe).expect("recipe digest");

    let report = run_workflow_recipe(&recipe, options()).expect("run workflow");

    assert_eq!(report.run_id, "run-001");
    assert_eq!(report.recipe_id, "recipe-runner");
    assert_eq!(report.recipe_digest, expected_digest);
    assert_eq!(report.status, WorkflowRunStatus::Succeeded);
    assert_eq!(report.steps.len(), 3);
    assert_eq!(report.steps[0].step_id, "preflight");
    assert_eq!(report.steps[1].step_id, "import");
    assert_eq!(report.steps[2].step_id, "analyze");
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(report.steps[1].warnings[0].message, "stub warning");
    assert_eq!(report.steps[1].blockers[0].code.as_deref(), Some("stub"));
    assert!(
        report
            .steps
            .iter()
            .all(|step| !step.request_digest.is_empty())
    );
    assert!(
        report
            .steps
            .iter()
            .all(|step| step.response_digest.is_some())
    );
    assert_eq!(
        report.steps[0].runtime_evidence[0],
        json!({"operation": "preflight_source", "step_kind": "preflight"})
    );
    report.validate().expect("valid report");
}

#[test]
fn recipe_digest_is_stable_for_json_object_key_order() {
    let first = json!({"b": 2, "a": {"z": 1, "m": 2}});
    let second = json!({"a": {"m": 2, "z": 1}, "b": 2});

    let mut recipe_a = valid_recipe();
    recipe_a.steps[2].request = first;
    let mut recipe_b = valid_recipe();
    recipe_b.steps[2].request = second;

    assert_eq!(
        workflow_recipe_digest(&recipe_a).expect("digest a"),
        workflow_recipe_digest(&recipe_b).expect("digest b")
    );
}

#[test]
fn run_workflow_propagates_validation_failures() {
    let mut recipe = valid_recipe();
    recipe.recipe_id.clear();
    recipe.steps[0].depends_on = vec!["missing-step".to_string()];

    let error = run_workflow_recipe(&recipe, options()).expect_err("invalid recipe should fail");

    match error {
        WorkflowRunnerError::InvalidRecipe(errors) => {
            let messages: Vec<String> = errors.into_iter().map(|error| error.message).collect();
            assert!(
                messages
                    .iter()
                    .any(|message| message.contains("recipe_id must not be empty"))
            );
            assert!(
                messages
                    .iter()
                    .any(|message| message.contains("depends on missing step 'missing-step'"))
            );
        }
        other => panic!("expected InvalidRecipe, got {other:?}"),
    }
}
