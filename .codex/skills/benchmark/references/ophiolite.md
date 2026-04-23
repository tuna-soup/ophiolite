# Ophiolite Benchmarking Notes

Use this reference when the benchmark work is in the `ophiolite` workspace or on the `traceboost` benchmark surfaces.

## Required Launcher On Windows

When a benchmark is actually being launched from the shared local Windows workstation, prefer:

- `.\scripts\run-benchmark-gated.ps1 -Mode development ...`
- `.\scripts\run-benchmark-gated.ps1 -Mode authoritative ...`

instead of invoking heavy benchmark commands directly.

Why:

- it snapshots CPU, RAM, and disk state before launch
- it serializes heavy local benchmark families through one shared heavy lane
- it sets controlled worker env vars for the child process
- it rewrites leading Windows `cargo ...` commands through `scripts/windows-msvc-cargo.cmd`
- it waits for a quieter machine state before `authoritative` runs

Use `-BenchmarkCommandLine '...'` with a single explicit command line.

Example:

```powershell
.\scripts\run-benchmark-gated.ps1 `
  -Mode authoritative `
  -BenchmarkCommandLine '.\target\debug\traceboost-app.exe benchmark-trace-local-processing C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol --scenario agc --partition-target-mib 64 --include-serial --repeat-count 3'
```

For Windows Criterion or bench/test compile surfaces:

```powershell
.\scripts\run-benchmark-gated.ps1 `
  -Mode development `
  -BenchmarkCommandLine 'cargo bench -p ophiolite-seismic-runtime --bench post_stack_neighborhood_kernels'
```

The launcher will rewrite the leading `cargo` invocation through
`.\scripts\windows-msvc-cargo.cmd` so the MSVC and SQLite setup is consistent.

## Remote Benchmark Flow

When the user wants a cleaner host than the shared workstation, prefer the
manual GitHub Actions workflow:

- `.github/workflows/benchmark.yml`

on a self-hosted Windows runner labeled:

- `self-hosted`
- `windows`
- `benchmark`

This workflow:

- validates the benchmark runner environment
- runs `verify-windows-benchmark-env.ps1`
- optionally runs `windows-msvc-cargo.cmd check -p ophiolite-seismic-runtime --benches --tests`
- runs `scripts/run-benchmark-gated.ps1`
- uploads benchmark logs and result artifacts

Treat that as the default remote authoritative path for this repo rather than
using generic hosted CI runners for publishable numbers.

## Benchmark Families In This Repo

Treat these benchmark families differently. Do not mix their evidence levels.

### 1. `traceboost-app` headless benchmark commands

These are the preferred benchmark surfaces for actual-data trace-local execution and shared batch execution.

Primary commands:

- `traceboost-app benchmark-trace-local-processing`
- `traceboost-app benchmark-trace-local-batch-processing`

Why these matter:

- They exercise the real runtime and planner/service path rather than a synthetic inner loop.
- They emit machine-readable JSON.
- They can discard generated output stores after measuring them.
- They expose partition and scheduler-policy fields that matter for runtime decisions.

Use these commands when the question is about:

- trace-local whole-volume processing performance
- partition target sizing
- shared execution service behavior
- batch concurrency and scheduler mode behavior
- end-to-end runtime decisions that affect the product path

Prefer these commands over older compatibility paths when the decision is about the current runtime architecture.

### 2. Criterion benches in Rust crates

Examples:

- `crates/ophiolite-seismic-runtime/benches/compute_storage.rs`
- `traceboost/io/benches/ingest.rs`

These are useful for:

- microbenchmarks
- storage and kernel comparisons on controlled synthetic fixtures
- local optimization work inside a crate

Do not use Criterion numbers alone to support claims about full application behavior when the real path includes planner overhead, store layout, orchestration, or app-specific command surfaces.

### 3. Ignored test-style harnesses in `apps/traceboost-demo/src-tauri`

Examples:

- `preview_session_bench.rs`
- `processing_cache_bench.rs`

These are local harnesses wrapped as ignored tests. Treat them as development or exploratory tools by default, not publishable product benchmarks.

Reasons:

- they depend on local benchmark stores outside normal test fixtures
- they are often launched with `cargo test ... -- --ignored --nocapture`
- they are sensitive to workstation contention and local disk state

Use them to investigate preview-session behavior or processing-cache behavior, but qualify the results accordingly.

## Standard Datasets And Paths

Known dataset conventions in this repo:

- Imported F3 TBVOL for trace-local and batch benchmark work:
  - `C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol`
- Source F3 SEG-Y used in the benchmark writeup:
  - `C:\Users\crooijmanss\Downloads\archive\f3_dataset.sgy`
- Large smoke-store candidates used by ignored test harnesses:
  - `%TEMP%\f3_dataset-smoke.tbvol`
  - `H:\traceboost-bench\f3_dataset-smoke.tbvol`
- Small repo fixture for processing-cache local work:
  - `test-data/f3.tbvol`

Do not compare numbers from different stores or fixture scales as if they were the same benchmark class.

In particular:

- `f3_dataset_regularized.tbvol` and `f3_dataset-smoke.tbvol` are not interchangeable evidence surfaces.
- `test-data/f3.tbvol` is useful for development but is not equivalent to the larger imported F3 benchmark store.

## Repo-Specific Rules

### Rule 1: Use `traceboost-app` commands for authoritative trace-local evidence

If the benchmark question is about partition-aware materialization, batch throughput, or scheduler mode behavior, prefer:

- `.\target\debug\traceboost-app.exe benchmark-trace-local-processing ...`
- `.\target\debug\traceboost-app.exe benchmark-trace-local-batch-processing ...`

Do not substitute a Criterion bench or an ignored test harness for these questions.

### Rule 2: Keep scenario, store, and concurrency knobs fixed when comparing runtime strategies

For `traceboost-app` benchmark comparisons, hold constant:

- input store path
- `--scenario`
- build profile and binary revision
- whether the run is serial or batch
- repeat count
- machine load

Change only one family of knobs at a time:

- `--partition-target-mib`
- `--max-active-jobs`
- `--execution-mode`
- adaptive versus fixed partitioning

Do not compare across multiple changed dimensions and then guess which change mattered.

### Rule 3: Do not run local F3-heavy benchmark families at the same time

Avoid overlapping any of these on the same workstation:

- `traceboost-app benchmark-trace-local-processing`
- `traceboost-app benchmark-trace-local-batch-processing`
- `benchmark_desktop_preview_session_*`
- `benchmark_processing_cache_*`

They compete for the same local CPU and memory bandwidth, and often for the same disk-backed store families. That contaminates both wall-clock numbers and scheduler behavior.

### Rule 4: Treat ignored test harnesses as exploratory unless promoted into a controlled benchmark plan

For:

- `cargo test --manifest-path apps/traceboost-demo/src-tauri/Cargo.toml benchmark_desktop_preview_session_* --lib -- --ignored --nocapture`
- `cargo test --manifest-path apps/traceboost-demo/src-tauri/Cargo.toml benchmark_processing_cache_* --lib -- --ignored --nocapture`

assume `development` evidence level unless the user explicitly builds a controlled rerun plan and documents the local dataset, machine state, and repeat policy.

### Rule 5: Treat Criterion as component evidence, not full-path evidence

For:

- `cargo bench -p ophiolite-seismic-runtime`
- `cargo bench -p traceboost-io`
- similar `criterion` surfaces in crate benches

use the numbers to guide implementation inside the component, but do not present them as proof of end-user performance without a matching application-path benchmark.

### Rule 6: Convert machine capacity into an explicit launch plan

When local inspection is possible, inspect the workstation and turn the result into a launch recommendation for this repo.

Always check:

- logical processor count
- current CPU saturation and other heavy local processes
- free RAM
- free space on the benchmark-output volume

Then translate that into:

- how many benchmark jobs may run concurrently
- what per-job thread cap to use
- whether the run is safe to execute interactively or should wait for a quieter machine

For this repo, do not allow several F3-heavy jobs to each use all available cores. If two jobs run together, both must be capped explicitly and the result should usually be treated as exploratory.

On the current shared Windows workstation, the default controlled heavy-run policy is:

- one heavy benchmark job at a time
- `OPHIOLITE_BENCHMARK_WORKERS=8`
- `RAYON_NUM_THREADS=8`
- `authoritative` runs wait for a quiet machine rather than launching immediately into contention

On the dedicated self-hosted Windows benchmark runner, keep the same worker and
serialization policy unless the benchmark is explicitly about thread scaling or
shared-host contention.

### Rule 7: Prefer repo-benchmark isolation over maximizing raw machine usage

For `traceboost-app` and the ignored F3 harnesses, a quieter machine is usually more valuable than slightly higher raw concurrency.

Preferred order:

1. stop unrelated heavy local work
2. run one relevant benchmark family at a time
3. fix a thread cap and repeat count
4. only then consider higher concurrency inside that benchmark family

Do not try to "use all 24 logical processors" just because they exist if that makes the workstation interactive, saturates memory, or pollutes the comparison.

## Suggested Modes For Common Repo Questions

- "Did the new partition target help end-to-end trace-local processing on actual F3 data?"
  - Use `authoritative`
  - Prefer `traceboost-app benchmark-trace-local-processing`
- "Which batch scheduler mode should be the default?"
  - Use `authoritative`
  - Prefer `traceboost-app benchmark-trace-local-batch-processing`
- "Did this cache change improve the preview-session path?"
  - Use `development` first
  - Use `benchmark_desktop_preview_session_*`
  - Promote only after controlled reruns
- "Is this operator kernel faster in isolation?"
  - Use `development` or `authoritative` depending on the claim
  - Prefer Criterion
- "Is this storage ingest path faster inside the crate?"
  - Prefer Criterion or crate-local bench

## Environment-Aware Repo Advice

Use these translation rules when giving concrete launch advice:

- `traceboost-app benchmark-trace-local-processing`
  - Prefer one job at a time.
  - Cap threads deliberately if the machine is interactive.
  - Keep `--scenario`, store path, and partition settings fixed while tuning concurrency.
- `traceboost-app benchmark-trace-local-batch-processing`
  - Distinguish between internal batch concurrency and external workstation contention.
  - Do not run several separate batch benchmark commands at once.
  - If measuring scheduler mode or `--max-active-jobs`, keep the rest of the workstation quiet so the command's own concurrency is the variable under test.
- `benchmark_desktop_preview_session_*` and `benchmark_processing_cache_*`
  - Treat these as especially sensitive to workstation contention.
  - Avoid running them while `traceboost-app` F3 commands or other F3-heavy jobs are active.
- Criterion benches
  - They may use their own iteration model and should usually be run alone when you care about the numbers.
  - Do not run Criterion benches in parallel with `traceboost-app` F3 benchmarks if you want trustworthy results from either.

## Current Command Shapes Worth Reusing

Representative commands from the repo's benchmark writeup:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario agc `
  --partition-target-mib 64,256 `
  --include-serial `
  --repeat-count 2
```

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario analytic `
  --partition-target-mib 64,256 `
  --include-serial `
  --repeat-count 2
```

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-batch-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario analytic `
  --job-count 4 `
  --max-active-jobs 1,2,4 `
  --partition-target-mib 64 `
  --repeat-count 2
```

When reusing these, keep the command shape stable and explain any deviation.

When running them locally on Windows, prefer wrapping them in the gated launcher rather than invoking them bare.

## Reporting Requirements For This Repo

For serious benchmark summaries in `ophiolite`, report:

- benchmark family used
- exact command
- store path
- scenario
- partition target or job concurrency knobs
- repeat count
- whether the workstation was otherwise busy
- whether the benchmark ran alone or alongside other heavy jobs
- whether the result is exploratory, development, or authoritative

If the result came from an ignored test harness or a contended workstation, say that plainly.
