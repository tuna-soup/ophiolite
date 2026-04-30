use serde_json::json;
use traceboost_app::workflow_report::{
    WORKFLOW_REPORT_SCHEMA_VERSION, WorkflowArtifactRecord, WorkflowAssertionRecord,
    WorkflowEnvironmentSummary, WorkflowRunNote, WorkflowRunReport, WorkflowRunStatus,
    WorkflowStepRunRecord, WorkflowTimingRecord, WorkflowTimingSummary, WorkflowVersionSummary,
};

mod workflow_report {
    pub use traceboost_app::workflow_report::*;
}

#[path = "../src/workflow_report_render.rs"]
mod workflow_report_render;

use workflow_report_render::{render_workflow_report_markdown, render_workflow_report_mermaid};

fn synthetic_report() -> WorkflowRunReport {
    WorkflowRunReport {
        schema_version: WORKFLOW_REPORT_SCHEMA_VERSION,
        run_id: "run-render-001".to_string(),
        recipe_id: "recipe-render".to_string(),
        recipe_digest: "blake3:recipe".to_string(),
        status: WorkflowRunStatus::Succeeded,
        started_at: "2026-04-30T12:00:00Z".to_string(),
        finished_at: Some("2026-04-30T12:00:03Z".to_string()),
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
                started_at: "2026-04-30T12:00:00Z".to_string(),
                finished_at: Some("2026-04-30T12:00:01Z".to_string()),
                request_digest: "blake3:req-preflight".to_string(),
                response_digest: Some("blake3:res-preflight".to_string()),
                warnings: vec![WorkflowRunNote {
                    message: "used fallback header".to_string(),
                    code: Some("STEP-WARN".to_string()),
                    detail: None,
                }],
                blockers: Vec::new(),
                runtime_evidence: Vec::new(),
                artifacts: vec![WorkflowArtifactRecord {
                    artifact_id: "preflight-log".to_string(),
                    artifact_kind: Some("log".to_string()),
                    uri: Some("file:///tmp/preflight.log".to_string()),
                    digest: Some("blake3:preflight-log".to_string()),
                    metadata: json!({}),
                }],
                timings: WorkflowTimingSummary {
                    elapsed_ms: Some(10.0),
                    stages: Vec::new(),
                },
            },
            WorkflowStepRunRecord {
                step_id: "import".to_string(),
                status: WorkflowRunStatus::Succeeded,
                depends_on: vec!["preflight".to_string()],
                started_at: "2026-04-30T12:00:01Z".to_string(),
                finished_at: Some("2026-04-30T12:00:03Z".to_string()),
                request_digest: "blake3:req-import".to_string(),
                response_digest: Some("blake3:res-import".to_string()),
                warnings: Vec::new(),
                blockers: vec![WorkflowRunNote {
                    message: "resolved by retry".to_string(),
                    code: Some("STEP-BLOCK".to_string()),
                    detail: None,
                }],
                runtime_evidence: Vec::new(),
                artifacts: Vec::new(),
                timings: WorkflowTimingSummary {
                    elapsed_ms: Some(20.5),
                    stages: vec![WorkflowTimingRecord {
                        name: "materialize".to_string(),
                        elapsed_ms: 18.25,
                    }],
                },
            },
        ],
        warnings: vec![WorkflowRunNote {
            message: "non-fatal issue".to_string(),
            code: Some("W001".to_string()),
            detail: None,
        }],
        blockers: Vec::new(),
        runtime_evidence: Vec::new(),
        artifacts: vec![WorkflowArtifactRecord {
            artifact_id: "summary".to_string(),
            artifact_kind: Some("json".to_string()),
            uri: Some("file:///tmp/summary.json".to_string()),
            digest: Some("blake3:summary".to_string()),
            metadata: json!({}),
        }],
        assertions: vec![WorkflowAssertionRecord {
            assertion_id: "final-exists".to_string(),
            status: WorkflowRunStatus::Succeeded,
            message: Some("final artifact exists".to_string()),
            evidence: json!({"artifact_id": "summary"}),
        }],
        timings: WorkflowTimingSummary {
            elapsed_ms: Some(30.5),
            stages: vec![
                WorkflowTimingRecord {
                    name: "setup".to_string(),
                    elapsed_ms: 2.0,
                },
                WorkflowTimingRecord {
                    name: "execute".to_string(),
                    elapsed_ms: 28.5,
                },
            ],
        },
    }
}

#[test]
fn markdown_render_includes_report_sections() {
    let markdown = render_workflow_report_markdown(&synthetic_report());

    assert!(markdown.contains("# Workflow Run run-render-001"));
    assert!(markdown.contains("- Recipe id: `recipe-render`"));
    assert!(markdown.contains("- Status: `succeeded`"));
    assert!(markdown.contains("- `W001`: non-fatal issue"));
    assert!(markdown.contains("| preflight | succeeded | - |"));
    assert!(markdown.contains("| import | succeeded | preflight |"));
    assert!(markdown.contains("## Step `preflight` Warnings"));
    assert!(markdown.contains("- `STEP-WARN`: used fallback header"));
    assert!(markdown.contains("## Step `import` Blockers"));
    assert!(markdown.contains("- `STEP-BLOCK`: resolved by retry"));
    assert!(markdown.contains("| summary | json | file:///tmp/summary.json | blake3:summary |"));
    assert!(markdown.contains("## Step `preflight` Artifacts"));
    assert!(markdown.contains("| final-exists | succeeded | final artifact exists |"));
    assert!(markdown.contains("- Elapsed ms: `30.5`"));
    assert!(markdown.contains("| execute | 28.5 |"));
    assert!(markdown.contains("## Step `import` Timings"));
    assert!(markdown.contains("| materialize | 18.25 |"));
}

#[test]
fn mermaid_render_includes_nodes_and_dependency_arrows() {
    let mermaid = render_workflow_report_mermaid(&synthetic_report());

    assert_eq!(
        mermaid,
        "flowchart TD\n    step_0[\"preflight\\nsucceeded\"]\n    step_1[\"import\\nsucceeded\"]\n    step_0 --> step_1\n"
    );
}

#[test]
fn mermaid_render_handles_steps_without_dependencies() {
    let mut report = synthetic_report();
    report.steps[1].depends_on.clear();

    let mermaid = render_workflow_report_mermaid(&report);

    assert_eq!(
        mermaid,
        "flowchart TD\n    step_0[\"preflight\\nsucceeded\"]\n    step_1[\"import\\nsucceeded\"]\n"
    );
}
