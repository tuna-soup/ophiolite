# Interactive Section Browsing Harness Plan

## Purpose

This note defines the next practical step for benchmarking `SeismicSection` interaction in TraceBoost and Ophiolite Charts.

The goal is not to compare against old builds retroactively. The goal is to create a repeatable framework so future optimization work can answer:

- did the new change improve the target interaction?
- did it accidentally regress a nearby interaction?
- are we still preserving the expected user behavior, especially viewport stability?

## What "harness" means here

For this stack, a harness is:

1. a fixed set of canonical user interaction scenarios
2. a repeatable way to run them
3. a structured artifact format for the results
4. optional pass/fail thresholds once the measurements are stable

This is not just "one benchmark command."

It is a performance contract for the interactions that matter to interpretation flow.

## Recommended harness layers

The harness should be split into three layers because they serve different purposes.

## 1. Runtime layer

This is already partly in place.

Use:

- `crates/ophiolite-seismic-runtime/src/bin/section_tile_bench.rs`
- `crates/ophiolite-seismic-runtime/src/bin/preview_incremental_bench.rs`
- existing compute/storage benches

What it measures well:

- full section vs tiled section cost
- LOD effects
- runtime payload size
- storage/tile geometry effects
- preview reuse behavior inside the runtime

What it does **not** measure:

- Tauri IPC
- frontend decode
- chart update cost
- viewport preservation bugs
- desktop orchestration behavior

This layer is safe for ordinary CI.

## 2. Desktop app interaction layer

This is the missing piece.

This layer should exercise the actual TraceBoost desktop app path:

- open dataset
- load line
- zoom or pan to a fixed viewport
- step to neighboring lines
- switch axis
- verify viewport remains stable
- record section-tile diagnostics

What it measures well:

- real user-facing section browsing latency
- payload size after viewport narrowing
- cache and prefetch behavior
- frontend/backend split in the real app
- viewport reset regressions

This layer is the most valuable one for the commercial chart and app story, because it measures the product path the user actually experiences.

## 3. Diagnostics summarization layer

This layer turns session logs into a stable benchmark artifact.

It should:

- parse the desktop session log
- summarize the relevant `section_tile` events
- emit text and JSON
- optionally enforce thresholds

This layer is cheap and CI-friendly once a session log exists.

## Canonical scenarios

Start with a small fixed scenario set.

## Scenario A: Cold full-extent section open

Purpose:

- capture first open cost
- capture full-section payload size
- establish the cold baseline

Actions:

1. Open a known `tbvol`
2. Load one middle inline
3. Leave viewport at full extents

Metrics:

- active viewport fetch `elapsedMs`
- backend tile load `duration_ms`
- `payloadBytes`
- prefetch timings

## Scenario B: Zoomed inline browse

Purpose:

- measure the main happy path we just optimized

Actions:

1. Open middle inline
2. Zoom to a fixed sub-viewport
3. Step `+1`
4. Step `+1`
5. Step `-1`

Metrics:

- active viewport fetch `elapsedMs`
- backend tile load `duration_ms`
- `payloadBytes`
- cache hit count
- prefetch request count
- fallback count

## Scenario C: Axis switch with viewport preservation

Purpose:

- catch the exact regression we just fixed

Actions:

1. Zoom to a fixed inline viewport
2. Switch to xline
3. Step once

Metrics:

- viewport remains bounded and does not reset to full extents
- no unexpected forced fit-to-data behavior
- tile request ranges remain local where geometry allows

## Scenario D: Xline browse

Purpose:

- ensure the optimization is not inline-only

Actions:

1. Open middle xline
2. Zoom to fixed sub-viewport
3. Step neighboring xlines

Metrics:

- same as Scenario B

## Scenario E: Intentional tile disablement

Purpose:

- confirm expected fallback modes are explicit rather than silent regressions

Actions:

Run a fixed case with one of:

- depth mode
- velocity overlay
- split compare

Metrics:

- tile diagnostics status changes away from active
- tile fetch counters do not grow unexpectedly
- behavior remains correct

## Recommended artifacts

Each scenario run should produce:

1. runtime benchmark JSON where applicable
2. desktop session log
3. summarized JSON report
4. concise markdown or console summary

That gives you:

- raw evidence
- a stable machine-readable format
- something easy to read in PRs or nightly reports

## Manual vs CI/CD

## What should be manual now

The actual desktop GUI interaction run should start as a manual or operator-triggered workflow.

Reason:

- it depends on a real desktop app
- GUI automation for Tauri desktop is more brittle than CLI/runtime benchmarking
- local datasets are large and not ideal for generic hosted CI
- the interaction script is not yet formalized as an internal app command surface

That manual workflow is still useful because the output becomes structured rather than anecdotal.

## What can already be automated in CI

These parts can run in CI today:

- `section_tile_bench`
- `preview_incremental_bench`
- diagnostics-log summarization on a captured log artifact
- threshold checks on existing summary JSON

So the harness is **partly automatable now**.

## What can be automated later

There are two realistic future automation paths.

### Path A: Self-hosted macOS runner

Run the real desktop interaction harness on:

- a self-hosted macOS machine
- scheduled nightly
- or manually dispatched workflow

This is the most realistic full-product automation path.

### Path B: Internal app scenario runner

Add a test-only or benchmark-only command surface inside TraceBoost so a scenario can be driven programmatically without desktop pointer automation.

That would let the harness:

- open a dataset
- set viewport directly
- step section index directly
- export diagnostics bundle directly

This is the cleanest long-term architecture if we want stable automated product-path benchmarking without flaky desktop UI scripting.

## Implemented command surface

That benchmark-only command surface now exists in the desktop app backend.

Current shape:

- Tauri command: `run_section_browsing_benchmark_command`
- frontend bridge helper: `runSectionBrowsingBenchmark(...)`
- temporary desktop hook: `window.traceboostBenchmarks.runSectionBrowsingBenchmark(...)`
- ownership: `apps/traceboost-demo/src-tauri` and `apps/traceboost-demo/src/lib/bridge.ts`

What it does:

- opens a target `tbvol`
- measures an optional full-section baseline
- measures the active tiled viewport case
- measures neighboring line steps using fixed offsets
- optionally measures an axis-switch case
- writes structured diagnostics into the normal desktop session log
- returns a structured JSON response with medians, means, payload bytes, and dataset metadata

What it does not do yet:

- drive pointer gestures through the visible GUI
- capture frontend frame timing directly
- launch itself from a packaged app menu or CLI argument

That is intentional. The current command gives us a stable product-path backend scenario runner without turning the commercial demo UI into a benchmark console.

## Recommendation

Use a staged rollout.

## Stage 1: now

Implement:

- runtime benchmark commands
- desktop diagnostics summarizer
- documented scenario list

Run:

- runtime benches in CI
- desktop scenarios manually when making performance changes

## Stage 2: next

Add:

- a caller for the existing benchmark-only scenario runner surface in TraceBoost desktop
- stable persisted JSON output for scenario results

Run:

- manual on demand
- nightly on macOS if the dataset is available

## Stage 3: later

Add:

- thresholds for regressions
- trend tracking across runs
- optional self-hosted macOS scheduled automation

## Why this split is the right one

The important distinction is:

- runtime benchmarks are easy to automate
- desktop interaction benchmarks are higher value but harder to automate robustly

So the right move is not to wait for perfect full CI. The right move is:

- automate the parts that are already stable
- structure the desktop evidence
- add deeper automation only when the command surface is ready

## Concrete repo placement

Recommended ownership:

- runtime benches remain under `crates/ophiolite-seismic-runtime/src/bin`
- desktop diagnostics summarizer under `scripts/validation`
- benchmark notes under `articles/benchmarking`
- future benchmark-only desktop scenario runner in `apps/traceboost-demo/src-tauri`

## Immediate next implementation

The immediate next useful implementation is:

1. keep using `section_tile_bench` for runtime-level measurements
2. summarize desktop session logs with a stable script
3. invoke the desktop benchmark runner manually from an internal caller while we stabilize the scenario set
4. add persisted JSON artifacts and optional scheduled macOS automation once repeated desktop benchmarking becomes frequent

That gets the team a real harness now, without pretending the hardest automation piece is already solved.
