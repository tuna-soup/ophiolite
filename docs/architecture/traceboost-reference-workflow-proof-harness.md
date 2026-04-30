# TraceBoost Reference Workflow Proof Harness

## Purpose

TraceBoost should act as the first-party proof harness for Ophiolite seismic
workflows.

It should prove that Ophiolite and Ophiolite Charts can take real or curated
subsurface datasets through preflight, import, inspection, processing, charting,
and output with reproducible evidence.

This document is app-level architecture. Canonical processing identity, runtime
events, artifact identity, and lineage remain Ophiolite-owned.

## Ownership

TraceBoost owns:

- workflow recipes
- workflow run orchestration
- app-local desktop/session behavior
- report rendering
- workflow assertions
- curated proof presets

Ophiolite owns:

- canonical contracts
- import/preflight evidence that should be reusable outside TraceBoost
- processing identity and runtime events
- artifact lineage and compatibility
- platform CLI/Python control surfaces

Ophiolite Charts owns:

- reusable chart-family rendering and interactions
- chart model/adapters
- visual regression and chart-performance evidence

## Recipe Model

Recipes should be typed JSON or TOML documents. They should not be shell scripts
or ad hoc command strings.

Minimal shape:

```json
{
  "schema_version": "1",
  "recipe_id": "f3_segy_section_processing_smoke",
  "name": "F3 SEG-Y section processing smoke",
  "dataset_inputs": [
    {
      "id": "survey",
      "source": "file:///path/to/source.sgy",
      "adapter_hint": "segy"
    }
  ],
  "steps": [
    {
      "step_id": "preflight_survey",
      "kind": "preflight",
      "input": "survey",
      "request": {}
    },
    {
      "step_id": "import_survey",
      "kind": "import",
      "depends_on": ["preflight_survey"],
      "request_ref": "preflight_survey.import_plan"
    },
    {
      "step_id": "preview_agc",
      "kind": "processing_preview",
      "depends_on": ["import_survey"],
      "request": {
        "pipeline": "trace_local_agc_preview"
      }
    }
  ],
  "expected_outputs": [
    {
      "step_id": "preview_agc",
      "assertions": ["status_ok", "non_empty_section"]
    }
  ]
}
```

Production recipes should use existing request payloads where possible:

- dataset preflight/import requests
- `SegyImportPlan` or successor adapter-detail plans
- `ProcessingPipelineSpec`
- processing preview/run/batch requests
- export requests
- assertion payloads

## Report Model

The canonical workflow report is JSON. Other formats are renderers.

Required report fields:

- `run_id`
- `recipe_id`
- `recipe_digest`
- `started_at`
- `completed_at`
- `status`
- app version and runtime/contract versions
- environment summary
- source fingerprints
- per-step request digest
- per-step response digest
- per-step timestamps and status
- runtime job ids
- Ophiolite inspectable plans
- Ophiolite runtime events
- output artifacts and digests
- lineage/package compatibility results
- assertion results
- benchmark/timing summaries where relevant

The report should link:

```text
recipe step
  -> request payload
  -> runtime job id
  -> inspectable plan
  -> runtime events
  -> output artifact
  -> lineage/compatibility result
```

## Renderer Model

Report rendering should be separated from report collection.

Initial renderers:

- JSON: canonical source of truth
- Markdown: readable run summary
- Mermaid: recipe DAG and stage graph
- HTML: later, if useful for public examples

Rendered reports must not become the canonical evidence store.

## CLI Boundary

TraceBoost should expose workflow commands through the app workflow layer, not
the platform CLI:

```text
traceboost workflow validate <recipe>
traceboost workflow run <recipe> --report <path>
traceboost workflow render-report <report> --format json|md|mermaid|html
```

The commands should call shared app orchestration, the same way browser dev
endpoints and Tauri commands should remain thin control surfaces.

## Proposed File Targets

Rust app workflow layer:

- `traceboost/app/traceboost-app/src/workflow_recipe.rs`
- `traceboost/app/traceboost-app/src/workflow_report.rs`
- `traceboost/app/traceboost-app/src/workflow_runner.rs`

Desktop persistence:

- `apps/traceboost-demo/src-tauri/src/workflow_recipes.rs`

Docs and examples:

- `examples/golden_paths/seismic_processing/`
- future `examples/golden_paths/traceboost_workflows/`

## First Workflows To Support

### First Golden Workflow

The first proof workflow targets a technical buyer or developer-evaluator.

It should use an F3-style SEG-Y or existing small post-stack fixture and prove:

```text
preflight
  -> import
  -> open
  -> section view
  -> AGC RMS preview
  -> materialized output
  -> JSON report
  -> Markdown report
  -> chart screenshot
  -> timing notes
```

The claim level is engineering correctness only:

- reproducible workflow execution
- structured source and geometry evidence
- source fingerprint capture
- CRS unresolved or source-native warning where applicable
- storage estimate capture
- runtime/report linkage
- chartable before/after evidence

It should not claim interpretation correctness or geoscientific optimality.

Use `amplitude_scalar` only as a trivial baseline. Use AGC RMS as the first
operator with a real before/after story.

### Later Workflows

After the first post-stack proof loop works, expand in this order:

1. SEG-Y coordinate inspection and survey map resolution.
2. Runtime store open, processing preview, materialized output.
3. Public-data manifest, import subset, generated report.
4. Poseidon MDIO ROI import/preflight.
5. Prestack gather load, analysis run, chart output.

## Anti-Patterns

Avoid:

- storing TraceBoost proof recipes in Ophiolite platform contracts
- treating shell command strings as recipes
- duplicating Ophiolite processing identity in app DTOs
- recomputing artifact identity in TraceBoost reports
- treating Markdown/HTML reports as source of truth
- letting desktop command names become platform API names
