# traceboost-app

`traceboost-app` is the Rust workflow backend for the `traceboost-demo` consumer app inside the Ophiolite repo.

It is demo support code, not part of the public `Ophiolite Charts` product surface. This crate turns shared runtime and contract capabilities into TraceBoost demo workflows and feeds both the desktop shell and the browser/Tauri development surfaces.

## Stack And Role

- Rust 2024 binary + library crate
- `clap` for the developer-facing CLI
- `serde` / `serde_json` for app-facing payloads
- depends on:
  - `seis-contracts-operations`
  - `seis-runtime`
  - `seis-io`

This crate is where demo workflow commands should live. It should orchestrate the runtime; it should not absorb raw SEG-Y parsing, chart rendering, or runtime-store internals.

The intended shape is one Rust workflow layer with multiple control surfaces on top of it:

- browser dev endpoints
- desktop/Tauri commands
- CLI commands

Those surfaces should reuse shared workflow orchestration here rather than reimplementing the same demo flows independently.

## Implemented

- reusable library helpers for:
  - survey preflight
  - dataset import
  - dataset open/summary
  - trace-local processing preview
  - trace-local processing materialization
  - survey-map resolution
  - native coordinate-reference assignment
  - survey time-depth demo/model workflows
  - a shared `TraceBoostWorkflowService` for app-facing orchestration
- CLI commands for:
  - backend info
  - inspect/analyze
  - ingest/validate
  - preflight-import
  - import-dataset
  - open-dataset
  - set-native-coordinate-reference
  - resolve-survey-map
  - view-section
  - preview-processing
  - run-processing
  - load-velocity-models
  - ensure-demo-survey-time-depth-transform
  - prepare-survey-demo
  - import-velocity-functions-model

## Roadmap

1. Keep the current import/open/view workflow stable for both CLI and Tauri consumers.
2. Add app-facing error and progress surfaces suitable for desktop UX.
3. Add lightweight session/recent-dataset support here or in a closely related app crate.
4. Keep lower-level processing logic in `seis-runtime`, not here.
