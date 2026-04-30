# TraceBoost post-stack AGC golden workflow placeholder

This placeholder defines the first TraceBoost golden workflow for a technical buyer or developer-evaluator. It is an engineering correctness workflow, not an interpretation workflow.

The intended dataset is either an F3-style SEG-Y post-stack line/cube or an existing small post-stack fixture configured locally. All paths in `recipe.json` are placeholders and must be replaced with local fixture paths before execution.

## Workflow

1. Preflight the configured post-stack SEG-Y fixture.
2. Import the fixture into a local TraceBoost workspace.
3. Open the imported survey/session.
4. Render a section view for visual and runtime sanity checks.
5. Preview AGC RMS as the primary operator.
6. Materialize the AGC RMS output.
7. Emit a workflow report with request, artifact, and assertion evidence.

## Operator intent

`agc_rms` is the primary operator for this workflow. The recipe checks that the preview and materialized output preserve expected engineering contracts such as non-empty dimensions, finite samples, stable trace count, and reportable artifacts.

`amplitude_scalar` is only a baseline comparison candidate for future workflows or manual local checks. It is intentionally not part of this first recipe.

## Local fixture configuration

Replace these placeholders in `recipe.json` before running:

- `${LOCAL_FIXTURE_ROOT}/post_stack/f3_style_small.sgy`
- `${LOCAL_WORKSPACE_ROOT}/traceboost-golden/post_stack_agc`
- `${LOCAL_OUTPUT_ROOT}/traceboost-golden/post_stack_agc/agc_rms.sgy`
- `${LOCAL_OUTPUT_ROOT}/traceboost-golden/post_stack_agc/workflow_report.json`

Do not use this recipe to claim geologic or interpretation correctness. Passing evidence should only support that the workflow can ingest a configured post-stack fixture, preview AGC RMS, materialize output, and report engineering evidence.
