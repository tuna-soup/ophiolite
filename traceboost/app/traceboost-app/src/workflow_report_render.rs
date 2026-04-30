use std::collections::HashMap;

use crate::workflow_report::{
    WorkflowArtifactRecord, WorkflowAssertionRecord, WorkflowRunNote, WorkflowRunReport,
    WorkflowRunStatus, WorkflowTimingSummary,
};

pub fn render_workflow_report_markdown(report: &WorkflowRunReport) -> String {
    let mut markdown = String::new();

    push_line(&mut markdown, &format!("# Workflow Run {}", report.run_id));
    push_line(&mut markdown, "");
    push_line(&mut markdown, &format!("- Run id: `{}`", report.run_id));
    push_line(
        &mut markdown,
        &format!("- Recipe id: `{}`", report.recipe_id),
    );
    push_line(
        &mut markdown,
        &format!("- Status: `{}`", status_label(&report.status)),
    );
    push_line(
        &mut markdown,
        &format!("- Started: `{}`", report.started_at),
    );
    push_line(
        &mut markdown,
        &format!(
            "- Finished: `{}`",
            optional_value(report.finished_at.as_deref())
        ),
    );

    push_notes_section(&mut markdown, "Warnings", &report.warnings);
    push_notes_section(&mut markdown, "Blockers", &report.blockers);

    push_line(&mut markdown, "");
    push_line(&mut markdown, "## Steps");
    push_line(&mut markdown, "");
    if report.steps.is_empty() {
        push_line(&mut markdown, "_No steps recorded._");
    } else {
        push_line(
            &mut markdown,
            "| Step | Status | Depends on | Started | Finished | Request | Response | Elapsed ms |",
        );
        push_line(
            &mut markdown,
            "| --- | --- | --- | --- | --- | --- | --- | ---: |",
        );
        for step in &report.steps {
            push_line(
                &mut markdown,
                &format!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} |",
                    table_cell(&step.step_id),
                    table_cell(status_label(&step.status)),
                    table_cell(&joined_or_dash(&step.depends_on)),
                    table_cell(&step.started_at),
                    table_cell(optional_value(step.finished_at.as_deref())),
                    table_cell(&step.request_digest),
                    table_cell(optional_value(step.response_digest.as_deref())),
                    table_cell(&elapsed_or_dash(&step.timings)),
                ),
            );
        }
    }

    push_artifacts_section(&mut markdown, "Artifacts", &report.artifacts);
    for step in &report.steps {
        if !step.warnings.is_empty() {
            push_notes_section(
                &mut markdown,
                &format!("Step `{}` Warnings", step.step_id),
                &step.warnings,
            );
        }
        if !step.blockers.is_empty() {
            push_notes_section(
                &mut markdown,
                &format!("Step `{}` Blockers", step.step_id),
                &step.blockers,
            );
        }
        if !step.artifacts.is_empty() {
            push_artifacts_section(
                &mut markdown,
                &format!("Step `{}` Artifacts", step.step_id),
                &step.artifacts,
            );
        }
    }

    push_assertions_section(&mut markdown, "Assertions", &report.assertions);
    push_timing_section(&mut markdown, "Timings", &report.timings);
    for step in &report.steps {
        if has_timings(&step.timings) {
            push_timing_section(
                &mut markdown,
                &format!("Step `{}` Timings", step.step_id),
                &step.timings,
            );
        }
    }

    markdown
}

pub fn render_workflow_report_mermaid(report: &WorkflowRunReport) -> String {
    let mut mermaid = String::new();
    push_line(&mut mermaid, "flowchart TD");

    if report.steps.is_empty() {
        push_line(&mut mermaid, "    empty[\"no steps\"]");
        return mermaid;
    }

    let node_ids: HashMap<&str, String> = report
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.step_id.as_str(), format!("step_{index}")))
        .collect();

    for step in &report.steps {
        let node_id = node_ids
            .get(step.step_id.as_str())
            .expect("step node id should exist");
        push_line(
            &mut mermaid,
            &format!(
                "    {node_id}[\"{}\"]",
                mermaid_label(&format!(
                    "{}\\n{}",
                    step.step_id,
                    status_label(&step.status)
                ))
            ),
        );
    }

    for step in &report.steps {
        let target_id = node_ids
            .get(step.step_id.as_str())
            .expect("step node id should exist");
        for dependency in &step.depends_on {
            if let Some(source_id) = node_ids.get(dependency.as_str()) {
                push_line(&mut mermaid, &format!("    {source_id} --> {target_id}"));
            }
        }
    }

    mermaid
}

fn push_notes_section(markdown: &mut String, heading: &str, notes: &[WorkflowRunNote]) {
    push_line(markdown, "");
    push_line(markdown, &format!("## {heading}"));
    push_line(markdown, "");
    if notes.is_empty() {
        push_line(markdown, "_None._");
        return;
    }

    for note in notes {
        match note.code.as_deref() {
            Some(code) => push_line(markdown, &format!("- `{code}`: {}", note.message)),
            None => push_line(markdown, &format!("- {}", note.message)),
        }
    }
}

fn push_artifacts_section(
    markdown: &mut String,
    heading: &str,
    artifacts: &[WorkflowArtifactRecord],
) {
    push_line(markdown, "");
    push_line(markdown, &format!("## {heading}"));
    push_line(markdown, "");
    if artifacts.is_empty() {
        push_line(markdown, "_None._");
        return;
    }

    push_line(markdown, "| Artifact | Kind | URI | Digest |");
    push_line(markdown, "| --- | --- | --- | --- |");
    for artifact in artifacts {
        push_line(
            markdown,
            &format!(
                "| {} | {} | {} | {} |",
                table_cell(&artifact.artifact_id),
                table_cell(optional_value(artifact.artifact_kind.as_deref())),
                table_cell(optional_value(artifact.uri.as_deref())),
                table_cell(optional_value(artifact.digest.as_deref())),
            ),
        );
    }
}

fn push_assertions_section(
    markdown: &mut String,
    heading: &str,
    assertions: &[WorkflowAssertionRecord],
) {
    push_line(markdown, "");
    push_line(markdown, &format!("## {heading}"));
    push_line(markdown, "");
    if assertions.is_empty() {
        push_line(markdown, "_None._");
        return;
    }

    push_line(markdown, "| Assertion | Status | Message |");
    push_line(markdown, "| --- | --- | --- |");
    for assertion in assertions {
        push_line(
            markdown,
            &format!(
                "| {} | {} | {} |",
                table_cell(&assertion.assertion_id),
                table_cell(status_label(&assertion.status)),
                table_cell(optional_value(assertion.message.as_deref())),
            ),
        );
    }
}

fn push_timing_section(markdown: &mut String, heading: &str, timings: &WorkflowTimingSummary) {
    push_line(markdown, "");
    push_line(markdown, &format!("## {heading}"));
    push_line(markdown, "");
    if !has_timings(timings) {
        push_line(markdown, "_None._");
        return;
    }

    if let Some(elapsed_ms) = timings.elapsed_ms {
        push_line(
            markdown,
            &format!("- Elapsed ms: `{}`", format_ms(elapsed_ms)),
        );
    }
    if !timings.stages.is_empty() {
        push_line(markdown, "");
        push_line(markdown, "| Stage | Elapsed ms |");
        push_line(markdown, "| --- | ---: |");
        for stage in &timings.stages {
            push_line(
                markdown,
                &format!(
                    "| {} | {} |",
                    table_cell(&stage.name),
                    table_cell(&format_ms(stage.elapsed_ms)),
                ),
            );
        }
    }
}

fn has_timings(timings: &WorkflowTimingSummary) -> bool {
    timings.elapsed_ms.is_some() || !timings.stages.is_empty()
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn status_label(status: &WorkflowRunStatus) -> &'static str {
    match status {
        WorkflowRunStatus::Pending => "pending",
        WorkflowRunStatus::Running => "running",
        WorkflowRunStatus::Succeeded => "succeeded",
        WorkflowRunStatus::Failed => "failed",
        WorkflowRunStatus::Blocked => "blocked",
        WorkflowRunStatus::Cancelled => "cancelled",
    }
}

fn optional_value(value: Option<&str>) -> &str {
    value.filter(|value| !value.is_empty()).unwrap_or("-")
}

fn joined_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn elapsed_or_dash(timings: &WorkflowTimingSummary) -> String {
    timings
        .elapsed_ms
        .map(format_ms)
        .unwrap_or_else(|| "-".to_string())
}

fn format_ms(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn table_cell(value: &str) -> String {
    value.replace('\n', " ").replace('|', "\\|")
}

fn mermaid_label(value: &str) -> String {
    value.replace('"', "'")
}
