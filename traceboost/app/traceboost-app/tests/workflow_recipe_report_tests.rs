use serde_json::json;
use traceboost_app::workflow_recipe::{
    WORKFLOW_RECIPE_SCHEMA_VERSION, WorkflowAssertion, WorkflowDatasetInput,
    WorkflowExpectedOutput, WorkflowRecipe, WorkflowRecipeStep, WorkflowRecipeStepKind,
};
use traceboost_app::workflow_report::{
    WORKFLOW_REPORT_SCHEMA_VERSION, WorkflowEnvironmentSummary, WorkflowRunReport,
    WorkflowRunStatus, WorkflowStepRunRecord, WorkflowTimingSummary, WorkflowVersionSummary,
};

fn valid_recipe() -> WorkflowRecipe {
    WorkflowRecipe {
        schema_version: WORKFLOW_RECIPE_SCHEMA_VERSION,
        recipe_id: "recipe-smoke".to_string(),
        name: "Recipe Smoke".to_string(),
        description: Some("domain-neutral workflow smoke recipe".to_string()),
        dataset_inputs: vec![WorkflowDatasetInput {
            dataset_id: "source-a".to_string(),
            name: Some("Source A".to_string()),
            source: Some("file:///tmp/source-a".to_string()),
            metadata: json!({"kind": "fixture"}),
        }],
        steps: vec![
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
                step_id: "import".to_string(),
                name: "Import".to_string(),
                step_kind: WorkflowRecipeStepKind::Import,
                operation: "import_source".to_string(),
                depends_on: vec!["preflight".to_string()],
                dataset_refs: vec!["source-a".to_string()],
                request: json!({"policy": "strict"}),
                expected_outputs: vec![WorkflowExpectedOutput {
                    output_id: "imported-a".to_string(),
                    output_kind: Some("dataset".to_string()),
                    spec: json!({"dataset_id": "source-a"}),
                }],
                assertions: vec![WorkflowAssertion {
                    assertion_id: "imported".to_string(),
                    description: None,
                    assertion_kind: "exists".to_string(),
                    spec: json!({"output_id": "imported-a"}),
                }],
            },
        ],
        expected_outputs: vec![WorkflowExpectedOutput {
            output_id: "final-a".to_string(),
            output_kind: Some("dataset".to_string()),
            spec: json!({"from_step": "import"}),
        }],
        assertions: vec![WorkflowAssertion {
            assertion_id: "final-exists".to_string(),
            description: Some("final artifact exists".to_string()),
            assertion_kind: "exists".to_string(),
            spec: json!({"output_id": "final-a"}),
        }],
    }
}

fn valid_report() -> WorkflowRunReport {
    WorkflowRunReport {
        schema_version: WORKFLOW_REPORT_SCHEMA_VERSION,
        run_id: "run-001".to_string(),
        recipe_id: "recipe-smoke".to_string(),
        recipe_digest: "blake3:abc123".to_string(),
        status: WorkflowRunStatus::Succeeded,
        started_at: "2026-04-30T10:00:00Z".to_string(),
        finished_at: Some("2026-04-30T10:00:01Z".to_string()),
        versions: WorkflowVersionSummary {
            traceboost_app: "0.1.0".to_string(),
            ophiolite: None,
            components: Vec::new(),
        },
        environment: WorkflowEnvironmentSummary {
            os: Some("test-os".to_string()),
            arch: Some("test-arch".to_string()),
            host: None,
            variables: Vec::new(),
        },
        source_fingerprints: Vec::new(),
        steps: vec![
            WorkflowStepRunRecord {
                step_id: "preflight".to_string(),
                status: WorkflowRunStatus::Succeeded,
                depends_on: Vec::new(),
                started_at: "2026-04-30T10:00:00Z".to_string(),
                finished_at: Some("2026-04-30T10:00:00Z".to_string()),
                request_digest: "blake3:req1".to_string(),
                response_digest: Some("blake3:res1".to_string()),
                warnings: Vec::new(),
                blockers: Vec::new(),
                runtime_evidence: vec![json!({"log": "preflight ok"})],
                artifacts: Vec::new(),
                timings: WorkflowTimingSummary {
                    elapsed_ms: Some(10.0),
                    stages: Vec::new(),
                },
            },
            WorkflowStepRunRecord {
                step_id: "import".to_string(),
                status: WorkflowRunStatus::Succeeded,
                depends_on: vec!["preflight".to_string()],
                started_at: "2026-04-30T10:00:00Z".to_string(),
                finished_at: Some("2026-04-30T10:00:01Z".to_string()),
                request_digest: "blake3:req2".to_string(),
                response_digest: Some("blake3:res2".to_string()),
                warnings: Vec::new(),
                blockers: Vec::new(),
                runtime_evidence: Vec::new(),
                artifacts: Vec::new(),
                timings: WorkflowTimingSummary {
                    elapsed_ms: Some(20.0),
                    stages: Vec::new(),
                },
            },
        ],
        warnings: Vec::new(),
        blockers: Vec::new(),
        runtime_evidence: vec![json!({"runner": "integration-test"})],
        artifacts: Vec::new(),
        assertions: Vec::new(),
        timings: WorkflowTimingSummary {
            elapsed_ms: Some(30.0),
            stages: Vec::new(),
        },
    }
}

#[test]
fn workflow_recipe_roundtrips_through_json() {
    let recipe = valid_recipe();

    let encoded = serde_json::to_string_pretty(&recipe).expect("serialize recipe");
    let decoded: WorkflowRecipe = serde_json::from_str(&encoded).expect("deserialize recipe");

    assert_eq!(decoded, recipe);
    decoded.validate().expect("valid recipe");
}

#[test]
fn workflow_report_roundtrips_through_json() {
    let report = valid_report();

    let encoded = serde_json::to_string_pretty(&report).expect("serialize report");
    let decoded: WorkflowRunReport = serde_json::from_str(&encoded).expect("deserialize report");

    assert_eq!(decoded, report);
    decoded.validate().expect("valid report");
}

#[test]
fn workflow_recipe_validation_catches_schema_ids_dependencies_and_dataset_refs() {
    let mut recipe = valid_recipe();
    recipe.schema_version = WORKFLOW_RECIPE_SCHEMA_VERSION + 1;
    recipe.recipe_id.clear();
    recipe.dataset_inputs.push(WorkflowDatasetInput {
        dataset_id: "source-a".to_string(),
        name: None,
        source: None,
        metadata: json!({}),
    });
    recipe.steps[1].step_id = "preflight".to_string();
    recipe.steps[1].depends_on = vec!["missing-step".to_string()];
    recipe.steps[1].dataset_refs = vec!["missing-dataset".to_string()];

    let messages: Vec<String> = recipe
        .validate()
        .expect_err("invalid recipe should fail")
        .into_iter()
        .map(|error| error.message)
        .collect();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("invalid schema_version"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("recipe_id must not be empty"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("duplicate dataset input id 'source-a'"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("duplicate step id 'preflight'"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("depends on missing step 'missing-step'"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("references unknown dataset 'missing-dataset'"))
    );
}

#[test]
fn workflow_report_validation_catches_schema_ids_and_dependencies() {
    let mut report = valid_report();
    report.schema_version = WORKFLOW_REPORT_SCHEMA_VERSION + 1;
    report.run_id.clear();
    report.recipe_id.clear();
    report.recipe_digest.clear();
    report.steps[1].step_id = "preflight".to_string();
    report.steps[1].depends_on = vec!["missing-step".to_string()];

    let messages: Vec<String> = report
        .validate()
        .expect_err("invalid report should fail")
        .into_iter()
        .map(|error| error.message)
        .collect();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("invalid schema_version"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("run_id must not be empty"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("recipe_id must not be empty"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("recipe_digest must not be empty"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("duplicate step id 'preflight'"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("depends on missing step 'missing-step'"))
    );
}
