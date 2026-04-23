# Trace-Local Execution Service And Partitioned Batch Benchmarking

## Purpose

This note records the execution-planning, job-service, trace-local partitioning, and shared batch-throughput work that was added to `ophiolite`, plus the first actual-data benchmark results after those changes.

It is intended to answer five questions:

- what changed in the runtime, shared execution layer, desktop app, and headless tooling
- how those changes shift Ophiolite toward a real SDK/API execution model rather than app-local orchestration
- whether partition-aware trace-local execution improves wall-clock time on a real seismic cube
- whether the shared execution service improves multi-job throughput in practice
- whether the new scheduler modes behave sensibly on actual data
- which optimization experiments are worth benchmarking before deeper implementation
- what remains as the next optimization frontier

## Superseding Note

The adaptive partition-sizing numbers in this note now have two evidence levels:

- earlier exploratory reruns on an uncontrolled workstation
- a later authoritative rerun with fixed benchmark-worker count

For any decision about defaults, use the authoritative fixed-worker rerun recorded later in this document. The earlier adaptive numbers are still useful as exploratory context, but they are not the decision-grade evidence.

## What Changed

The implementation landed in several layers rather than one big refactor.

### 1. Shared execution planning model in the runtime

The runtime now has an explicit execution model:

- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic-runtime/src/planner.rs`

That work introduced:

- `OperatorExecutionTraits`
- `ExecutionPlan`
- `ExecutionStage`
- partition and cache metadata
- planning modes for preview, foreground materialization, and background batch execution

This is the architectural base described in [ADR-0031](../../docs/architecture/ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md).

### 2. Shared bounded execution service

The new shared orchestration boundary lives in:

- `crates/ophiolite-seismic-execution/src/lib.rs`

That layer now owns:

- job registration and state
- batch registration and bounded concurrency
- plan-aware job status
- batch polling and cancellation semantics

The important change is structural: `traceboost-demo` is no longer the right place to own seismic job orchestration logic.

That layer now also owns first-pass scheduler policy resolution for batch work. Instead of only accepting a raw `max_active_jobs` number, the service can now resolve a bounded policy decision with:

- `execution_mode = auto | conservative | throughput | custom`
- `scheduler_reason`
- requested versus effective job concurrency
- worker-budget and machine-cap clamping
- plan-derived worst-stage memory and partition hints

### 3. TraceBoost desktop now consumes the shared execution model

The desktop bridge and app moved from app-local worker spawning toward the shared planner/service model:

- `apps/traceboost-demo/src-tauri/src/lib.rs`
- `apps/traceboost-demo/src-tauri/src/processing.rs`
- `apps/traceboost-demo/src/lib/processing-model.svelte.ts`
- `apps/traceboost-demo/src/lib/components/PipelineOperatorEditor.svelte`
- `apps/traceboost-demo/src/lib/components/NeighborhoodOperatorEditor.svelte`

The user-visible additions are:

- multi-cube batch submission
- selectable batch scheduler mode (`auto`, `conservative`, `throughput`)
- auto batch concurrency when the caller does not pin `max_active_jobs`
- plan summaries on jobs
- actual execution summaries on jobs
- requested versus effective batch-concurrency reporting
- resolved execution mode and scheduler reason on active batches
- visible partition counts, active-partition peaks, and stage-level execution metadata

### 4. Trace-local whole-volume execution is now partition-aware

The key runtime change is in:

- `crates/ophiolite-seismic-runtime/src/compute.rs`

`MaterializeOptions` now carries `partition_target_bytes`, and the TBVOL trace-local materialization path can execute tile groups in parallel instead of one purely serial whole-volume tile loop.

That change also emits actual partition progress so downstream callers can report:

- total partitions
- completed partitions
- peak active partitions
- retry count

### 5. Headless benchmark commands for actual-data runs

To benchmark the real partition-aware path without UI noise, `traceboost-app` now exposes:

- `traceboost-app benchmark-trace-local-processing`
- `traceboost-app benchmark-trace-local-batch-processing`
- `traceboost-app benchmark-post-stack-neighborhood-preview`
- `traceboost-app benchmark-post-stack-neighborhood-processing`

Implemented in:

- `traceboost/app/traceboost-app/src/lib.rs`
- `traceboost/app/traceboost-app/src/main.rs`

These commands:

- run named trace-local benchmark scenarios
- run neighborhood preview and full-processing baseline scenarios
- support serial, partitioned, and shared-service batch variants
- record actual runtime partition metrics
- record scheduler-policy fields for batch runs
- record plan classification summaries for both trace-local and neighborhood cases
- emit machine-readable JSON
- can discard generated output stores after measuring them

That last point matters. Benchmarking the older `run-processing` compatibility path would have been misleading because it did not yet exercise the new partition-aware execution option or the shared execution-service batch gate.

## Benchmark Dataset And Method

### Source data

The benchmark used the local F3 SEG-Y:

- `C:\Users\crooijmanss\Downloads\archive\f3_dataset.sgy`

It was imported once into a temporary TBVOL store:

- `C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol`

Import result:

- shape: `651 x 951 x 462`
- chunk shape: `82 x 56 x 462`
- total tiles: `136`
- sample interval: `4.0 ms`

### Benchmark scenarios

Two trace-local scenarios were measured:

1. `agc`
   - `trace_rms_normalize`
   - `agc_rms(window_ms = 250.0)`
2. `analytic`
   - `trace_rms_normalize`
   - `envelope`
   - `instantaneous_phase`
   - `instantaneous_frequency`
   - `sweetness`

### Single-job execution variants

Each scenario was run with:

- serial execution
- partitioned with `64 MiB` target partitions
- partitioned with `256 MiB` target partitions

Each variant was repeated twice, and the benchmark discarded produced output stores after measuring them.

### Batch-throughput variants

The same two scenarios were also measured through the shared execution service as four-job batches against the imported F3 TBVOL.

That is not the final "many different cubes" benchmark shape, but it is still useful because it exercises:

- shared batch registration
- bounded `max_active_jobs`
- queue wait
- actual job makespan under contention
- scheduler decision reporting
- planner and runtime integration through the real service path

Each batch run used:

- `job_count = 4`
- `partition_target_mib = 64`
- `max_active_jobs = 1, 2, 4`
- `repeat_count = 2`

### Scheduler-mode variants

The same four-job batch shape was also run through the new plan-aware scheduler modes:

- `execution_mode = auto`
- `execution_mode = conservative`
- `execution_mode = throughput`
- `repeat_count = 2`

These runs are important because they exercise the actual shared-service policy surface instead of only manual concurrency overrides.

### Commands

Import:

```powershell
.\target\debug\traceboost-app.exe import-dataset `
  C:\Users\crooijmanss\Downloads\archive\f3_dataset.sgy `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --inline-byte 189 `
  --crossline-byte 193 `
  --overwrite-existing
```

AGC benchmark:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario agc `
  --partition-target-mib 64,256 `
  --include-serial `
  --repeat-count 2
```

Analytic benchmark:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario analytic `
  --partition-target-mib 64,256 `
  --include-serial `
  --repeat-count 2
```

AGC batch benchmark:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-batch-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario agc `
  --job-count 4 `
  --max-active-jobs 1,2,4 `
  --partition-target-mib 64 `
  --repeat-count 2
```

Analytic batch benchmark:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-batch-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario analytic `
  --job-count 4 `
  --max-active-jobs 1,2,4 `
  --partition-target-mib 64 `
  --repeat-count 2
```

AGC batch benchmark with scheduler mode:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-batch-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario agc `
  --job-count 4 `
  --execution-mode auto `
  --partition-target-mib 64 `
  --repeat-count 2
```

Analytic batch benchmark with scheduler mode:

```powershell
.\target\debug\traceboost-app.exe benchmark-trace-local-batch-processing `
  C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol `
  --scenario analytic `
  --job-count 4 `
  --execution-mode conservative `
  --partition-target-mib 64 `
  --repeat-count 2
```

Raw results are stored in:

- [2026-04-22-f3-trace-local-agc-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-agc-benchmark.json)
- [2026-04-22-f3-trace-local-analytic-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-analytic-benchmark.json)
- [2026-04-22-f3-trace-local-batch-agc-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-agc-benchmark.json)
- [2026-04-22-f3-trace-local-batch-analytic-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-analytic-benchmark.json)
- [2026-04-22-f3-trace-local-batch-agc-auto-mode-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-agc-auto-mode-benchmark.json)
- [2026-04-22-f3-trace-local-batch-agc-conservative-mode-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-agc-conservative-mode-benchmark.json)
- [2026-04-22-f3-trace-local-batch-agc-throughput-mode-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-agc-throughput-mode-benchmark.json)
- [2026-04-22-f3-trace-local-batch-analytic-auto-mode-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-analytic-auto-mode-benchmark.json)
- [2026-04-22-f3-trace-local-batch-analytic-conservative-mode-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-analytic-conservative-mode-benchmark.json)
- [2026-04-22-f3-trace-local-batch-analytic-throughput-mode-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-analytic-throughput-mode-benchmark.json)
- [2026-04-22-f3-trace-local-agc-64mib-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-agc-64mib-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-agc-adaptive-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-agc-adaptive-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-analytic-64mib-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-analytic-64mib-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-analytic-adaptive-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-analytic-adaptive-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-batch-agc-auto-64mib-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-agc-auto-64mib-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-batch-agc-auto-adaptive-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-agc-auto-adaptive-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-batch-analytic-auto-64mib-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-analytic-auto-64mib-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-batch-analytic-auto-adaptive-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-batch-analytic-auto-adaptive-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-agc-classification-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-agc-classification-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-trace-local-analytic-classification-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-trace-local-analytic-classification-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-preview-similarity-small-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-small-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-preview-similarity-small-prefix-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-small-prefix-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-preview-similarity-medium-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-medium-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-preview-similarity-large-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-large-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-processing-similarity-small-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-small-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-processing-similarity-small-prefix-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-small-prefix-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-processing-similarity-medium-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-medium-authoritative-workers8-benchmark.json)
- [2026-04-22-f3-post-stack-neighborhood-processing-similarity-large-authoritative-workers8-benchmark.json](../benchmarking/results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-large-authoritative-workers8-benchmark.json)

## Results

### AGC scenario

| Variant | Avg wall time | Partitions | Peak active partitions | Speedup vs serial |
| --- | ---: | ---: | ---: | ---: |
| Serial | 3988 ms | 1 | 1 | baseline |
| Partitioned 256 MiB | 3538 ms | 5 | 5 | 1.13x |
| Partitioned 64 MiB | 3169 ms | 20 | 20 | 1.26x |

Observations:

- `64 MiB` was the best AGC configuration on this machine.
- The best run reduced wall-clock time by about `20.5%` versus the serial baseline.
- The runtime exposed enough partitions at `64 MiB` to keep all available workers busy.

### Analytic scenario

| Variant | Avg wall time | Partitions | Peak active partitions | Speedup vs serial |
| --- | ---: | ---: | ---: | ---: |
| Serial | 23678 ms | 1 | 1 | baseline |
| Partitioned 256 MiB | 22571 ms | 5 | 5 | 1.05x |
| Partitioned 64 MiB | 22229 ms | 20 | 20 | 1.07x |

Observations:

- The heavier analytic stack still improved, but more modestly.
- The best average improvement was about `6.1%`.
- That suggests this operator mix is less bottlenecked by tile-loop seriality than the AGC case, or is hitting other costs such as trace-local kernel work and write bandwidth more heavily.

### Batch throughput through the shared execution service

#### AGC batch scenario

| `max_active_jobs` | Avg batch makespan | Avg queue wait | Avg per-job elapsed | Makespan speedup vs `1` |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 14647 ms | 5598 ms | 3494 ms | baseline |
| 2 | 11978 ms | 3178 ms | 5407 ms | 1.22x |
| 4 | 9939 ms | 0.06 ms | 9052 ms | 1.47x |

Observations:

- `max_active_jobs = 4` delivered the best pure throughput for AGC on this machine.
- `max_active_jobs = 2` was the more balanced operating point if foreground responsiveness still matters, because it cut queue wait without inflating per-job elapsed time as aggressively as `4`.
- This is exactly why the scheduler needs separate notions of throughput and latency: the best batch makespan and the best single-job experience are not the same setting.

#### Analytic batch scenario

| `max_active_jobs` | Avg batch makespan | Avg queue wait | Avg per-job elapsed | Makespan speedup vs `1` |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 86297 ms | 32373 ms | 21391 ms | baseline |
| 2 | 83434 ms | 20856 ms | 37465 ms | 1.03x |
| 4 | 80983 ms | 0.06 ms | 71693 ms | 1.07x |

Observations:

- The analytic stack preferred `max_active_jobs = 4` for throughput, but only modestly.
- `max_active_jobs = 2` improved queue wait relative to serial batch execution, but did not improve throughput enough to call it the obvious default for this workload.
- The heavier operator mix amplifies the policy problem: the same scheduler setting that is reasonable for AGC is much less attractive once the per-job pipeline is compute-heavier.

### Scheduler-mode results

#### AGC scheduler modes

| Mode | Effective jobs | Scheduler reason | Avg batch makespan | Avg queue wait | Avg per-job elapsed |
| --- | ---: | --- | ---: | ---: | ---: |
| `auto` | 3 | `auto_medium_cost_batch` | 12354 ms | 2038 ms | 7158 ms |
| `conservative` | 2 | `conservative_mode` | 16235 ms | 4298 ms | 7449 ms |
| `throughput` | 4 | `throughput_mode` | 16168 ms | 0.08 ms | 15527 ms |

Observations:

- `auto` was clearly the best AGC mode on this machine.
- `throughput` removed queueing almost entirely, but that gain was overwhelmed by much slower per-job execution.
- `conservative` was safer than `throughput`, but slower than `auto` both on makespan and per-job latency.

#### Analytic scheduler modes

| Mode | Effective jobs | Scheduler reason | Avg batch makespan | Avg queue wait | Avg per-job elapsed |
| --- | ---: | --- | ---: | ---: | ---: |
| `auto` | 3 | `auto_medium_cost_batch` | 89073 ms | 12431 ms | 58574 ms |
| `conservative` | 2 | `conservative_mode` | 88354 ms | 22855 ms | 38942 ms |
| `throughput` | 4 | `throughput_mode` | 93171 ms | 0.08 ms | 83161 ms |

Observations:

- The heavier analytic stack preferred `conservative` over both `auto` and `throughput`.
- `throughput` was the worst mode here. It removed queue wait, but made each job so much slower that total makespan got worse too.
- `auto` was close to the best result, but still chose a slightly too-aggressive concurrency for this workload.

## Authoritative Adaptive Partition-Sizing Rerun

### Conditions

This rerun used the repo-preferred headless benchmark family:

- `traceboost-app benchmark-trace-local-processing`
- `traceboost-app benchmark-trace-local-batch-processing`

The controlled conditions were:

- benchmark mode: `authoritative`
- store: `C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol`
- code revision: `099b011d6792feb456077a351bd4c875949e7135`
- fixed worker count: `OPHIOLITE_BENCHMARK_WORKERS=8`
- batch mode: `execution_mode=auto`, `job_count=4`
- repeat count: `3`
- runs serialized, one heavy workload at a time
- process priority: `High`
- workstation state during sampling: interactive, but lightly loaded enough for controlled reruns
  - sampled total CPU load after the run series was about `7-16%`
  - processor queue length stayed at `0`
  - available memory was about `13.5 GiB`

These conditions supersede the earlier adaptive reruns that did not actually pin the worker count.

### Chosen Adaptive Targets

Under the fixed 8-worker configuration, the adaptive recommender chose:

- AGC: about `72.9 MiB`, targeting `16` partitions
- Analytic: about `137.7 MiB`, targeting `8` partitions

That is the first meaningful sign that the adaptive heuristic is doing what it was designed to do: it keeps lighter trace-local work more finely partitioned, and heavier trace-local work coarser.

### Single-Job Comparison

| Scenario | Fixed target | Adaptive target | Fixed avg wall time | Adaptive avg wall time | Delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| AGC | `64 MiB` | `72.9 MiB` | `6161 ms` | `6207 ms` | `+0.74%` |
| Analytic | `64 MiB` | `137.7 MiB` | `55313 ms` | `55312 ms` | `-0.00%` |

Interpretation:

- For single-job trace-local execution, adaptive sizing was effectively neutral on this machine.
- AGC became slightly slower, but the difference was under `1%`.
- Analytic was effectively identical within the observed spread.

### Batch Comparison With `execution_mode=auto`

| Scenario | Fixed target | Adaptive target | Fixed avg batch makespan | Adaptive avg batch makespan | Delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| AGC batch | `64 MiB` | `72.9 MiB` | `21196 ms` | `20615 ms` | `-2.74%` |
| Analytic batch | `64 MiB` | `137.7 MiB` | `218029 ms` | `217205 ms` | `-0.38%` |

Interpretation:

- Adaptive sizing slightly improved batch makespan for both measured scenarios.
- The AGC batch gain was the only one large enough to care about without more reruns, at about `2.7%`.
- The analytic batch gain was very small and should be treated as neutral until confirmed on a cleaner or more repeated run.

### What This Proves

The authoritative rerun changes the earlier conclusion.

- Adaptive partition sizing is not a strong standalone win on the current trace-local runtime path.
- It does not materially improve single-job latency on F3.
- It may provide a small batch-throughput improvement, especially for lighter pipelines, but the effect size is modest.
- The more important value right now is architectural: the benchmark harness can now pin worker count and test adaptive partition policy under controlled conditions.

So the correct next step is not "make adaptive partition sizing the default." The correct next step is to keep it available as an experiment, and move to the other OpenDTect-inspired benchmark ideas that are more likely to shift the curve:

- staged read/decode plus compute
- halo-expanded neighborhood reads
- resume-after-interruption
- stronger plan-level workload classification

## Authoritative Batch-Aware Adaptive Follow-Up

The earlier authoritative adaptive rerun above was still using a per-job adaptive target that was compiled as if each batch job owned the full machine memory budget.

That was good enough to validate the benchmark surface, but it was still too coarse for `execution_mode=auto` batch decisions. After those reruns, the planner and benchmark path were tightened in three places:

- the trace-local chunk planner now penalizes under-partitioned `throughput` candidates instead of rewarding a few oversized chunks
- the adaptive recommender can now compile against an explicit concurrent-job count
- the TraceBoost batch benchmark path now resolves the batch policy first, then derives the adaptive partition target from the effective concurrent batch-job count rather than the whole-machine budget

This is still not a full executor-memory refactor. It is a targeted change in the planning layer and benchmark surface, but it is the first benchmark-driven step that makes adaptive partitioning genuinely batch-aware.

### Conditions

The follow-up rerun used:

- benchmark mode: `authoritative`
- repo launcher: `scripts/run-benchmark-gated.ps1`
- store: `C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol`
- fixed worker count: `OPHIOLITE_BENCHMARK_WORKERS=8`
- batch mode: `execution_mode=auto`, `job_count=4`
- repeat count: `2`
- binary: local rebuilt `target\debug\traceboost-app.exe`
- runs serialized through the heavy lane, one heavy workload at a time
- clean rerun conditions for the final analytic adaptive confirmation:
  - no contending `traceboost-app`
  - sampled CPU load about `14-17%`
  - available memory about `13.6 GiB`

One earlier analytic adaptive rerun on this branch was rejected as evidence because it launched while another heavy `traceboost-app` workload was still active. The clean rerun below supersedes that contaminated attempt.

### Chosen Adaptive Targets

Under the batch-aware adaptive logic, both measured batch scenarios converged on a much smaller target than the earlier batch-adaptive experiment:

- AGC batch: about `81 MiB`, targeting `15` partitions, capped at `8` active partitions
- Analytic batch: about `81 MiB`, targeting `15` partitions, capped at `8` active partitions

That is the behavior the earlier batch numbers were missing. Once the planner accounts for concurrent jobs, it no longer picks a per-job chunk size that is appropriate only for an isolated single job.

### Batch Comparison With `execution_mode=auto`

| Scenario | Fixed target | Adaptive target | Fixed avg batch makespan | Adaptive avg batch makespan | Delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| AGC batch | `64 MiB` | `81 MiB` | `19420 ms` | `19121 ms` | `-1.54%` |
| Analytic batch | `64 MiB` | `81 MiB` | `212653 ms` | `206360 ms` | `-2.96%` |

Interpretation:

- The batch-aware adaptive change improves both authoritative batch comparisons.
- The AGC improvement is modest, but now clearly on the right side of neutral.
- The analytic improvement is large enough to matter, and the rerun conditions were clean enough to treat it as decision-grade for this workstation.
- The earlier conclusion that adaptive should stay "experiment only" for batch defaults no longer holds in the same way once the adaptive target is compiled against concurrent batch jobs.

### What This Changes

The benchmark-backed recommendation is now more specific:

- single-job adaptive sizing is still best described as "reasonable, not clearly better than a tuned fixed target"
- batch-aware adaptive sizing is now a viable default candidate for trace-local `execution_mode=auto`
- the richer memory/chunk model is already paying for itself at the planning boundary, even though the runtime executor still consumes a compatibility `partition_target_bytes` plus `max_active_partitions` rather than a fully generic chunk plan

That means the next step should not be another round of hand-tuning fixed `partition_target_mib` values. The next step should be to keep moving the execution model toward explicit budgeted chunk planning inside the shared runtime and shared execution service.

## Authoritative Stage-Classification Validation And Neighborhood Baselines

### Conditions

This follow-on benchmark pass answered two separate questions:

- do AGC-like and analytic trace-local plans now classify differently enough to justify richer planner hints
- what does the current neighborhood baseline look like before we change any runtime defaults there

The controlled conditions were:

- benchmark mode: `authoritative`
- repo launcher: `scripts/run-benchmark-gated.ps1`
- store: `C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol`
- code revision: `77bcc7d82234521c34058a30109fdce52732b0f9`
- fixed worker count: `OPHIOLITE_BENCHMARK_WORKERS=8`
- repeat count:
  - trace-local classification validation: `3`
  - neighborhood preview: `3`
  - neighborhood processing:
    - `small` / `small + prefix` / `medium`: `3`
    - `large`: rerun at `1` after the original `repeat_count=3` attempt exceeded three hours
- heavy workloads were run one at a time
- the launcher enforced quiet-machine waiting for the trace-local validation runs, and the neighborhood runs were executed manually one at a time on the same quiet host before the neighborhood heavy-pattern fix landed

One caveat matters here. The gated runner initially did not classify the new `benchmark-post-stack-neighborhood-*` commands as heavy benchmarks, so the heavy-lane mutex did not apply to those commands until that pattern was patched later in `scripts/run-benchmark-gated.ps1`. The actual runs recorded below were still executed one at a time on a quiet host, but the tooling gap was real and is now fixed in-repo.

For the runtime-only `traceboost-app.exe` commands, I used the gate with `-SkipEnvVerification`. The benchmark binary was already built and the local verifier was failing on missing SQLite build-environment variables (`OPHIOLITE_SQLITE_INCLUDE` / `OPHIOLITE_SQLITE_LIB_DIR`), which are relevant to MSVC build hygiene but not to executing an already-built headless benchmark binary.

### Trace-Local Classification Validation

The richer planner traits now separate the two measured trace-local scenarios cleanly:

| Scenario | Max memory | Max CPU | Max IO | Min parallel efficiency | Combined CPU weight | Combined IO weight |
| --- | --- | --- | --- | --- | ---: | ---: |
| AGC | `medium` | `medium` | `low` | `high` | `5.5` | `2.5` |
| Analytic | `medium` | `high` | `low` | `medium` | `27.0` | `6.25` |

That is the key planner result from this slice. Before this work, AGC-like and analytic-like trace-local pipelines collapsed too easily into the same execution summary. They no longer do.

The runtime numbers stayed directionally consistent with the earlier fixed-`64 MiB` evidence:

| Scenario | Serial avg wall time | Partitioned `64 MiB` avg wall time | Delta |
| --- | ---: | ---: | ---: |
| AGC | `6372 ms` | `5741 ms` | `-9.9%` |
| Analytic | `52551 ms` | `51806 ms` | `-1.4%` |

Interpretation:

- The new classification model is explaining a real runtime difference, not inventing one.
- AGC remains materially more parallel-friendly than the heavier analytic stack on this F3 workload.
- The richer summary fields are now useful benchmark and diagnostics inputs even before the scheduler consumes them directly.

### Neighborhood Preview Baseline

The first authoritative neighborhood preview matrix used `similarity` on one inline section (`section_index=325`) with a small window ladder:

| Case | Avg elapsed | Delta vs small |
| --- | ---: | ---: |
| Small (`24 ms`, `1x1`) | `730 ms` | baseline |
| Small + trace-local prefix | `749 ms` | `+2.6%` |
| Medium (`48 ms`, `2x2`) | `2625 ms` | `3.6x` |
| Large (`96 ms`, `4x4`) | `10868 ms` | `14.9x` |

All of those preview plans classified as:

- `max_memory_cost_class = high`
- `max_cpu_cost_class = high`
- `max_io_cost_class = high`
- `min_parallel_efficiency_class = low`

Interpretation:

- The neighborhood classification is intentionally conservative right now, and the timings justify that conservatism.
- Prefixing the small preview with a simple trace-local normalization step did not help in this current implementation; it was slightly slower.
- Preview cost grows quickly with window size and stepout, which is exactly why neighborhood needs its own benchmark surface before we tune shared defaults around it.

### Neighborhood Full-Processing Baseline

The current full-volume neighborhood materialization path is much more expensive than trace-local whole-volume processing on the same cube:

| Case | Avg wall time | Approx. wall time |
| --- | ---: | ---: |
| Small (`24 ms`, `1x1`) | `377772 ms` | `6.30 min` |
| Small + trace-local prefix | `377443 ms` | `6.29 min` |
| Medium (`48 ms`, `2x2`) | `1402953 ms` | `23.38 min` |
| Large (`96 ms`, `4x4`) | `5920527 ms` | `98.68 min` |

All of these plans also classified as `high cpu / high io / low efficiency`.

Interpretation:

- The neighborhood baseline is now explicit enough that we do not need to guess whether this family is “probably heavier.” It is.
- Adding a simple trace-local prefix to the small case was effectively neutral for full processing, just as it was not meaningfully helpful for preview.
- The large full-processing case is so expensive on the current path that the original three-repeat attempt was impractical on the workstation. A single authoritative rerun was still worth keeping because it establishes the scale of the current baseline.
- This is the clearest evidence in the repo right now that neighborhood execution maturity still lags trace-local maturity substantially.

## What The Results Mean

The benchmark confirms four important things.

### 1. The partition-aware path is real, not just planned

This is no longer just planner metadata. The runtime is actually:

- splitting work into tile groups
- executing those partitions in parallel
- exposing actual partition counts and active-worker peaks

### 2. Smaller partitions are better on this F3 store

For both measured single-job scenarios, `64 MiB` partitions beat `256 MiB` partitions.

That is a strong signal that output chunking and partition policy should remain planner-controlled rather than inherited blindly from input storage defaults.

### 3. Speedup is workload-dependent

The AGC pipeline improved materially. The analytic pipeline improved, but less dramatically.

That is exactly why the benchmark harness matters. It prevents "parallelization theater" by making the real operator mix visible in measurements.

### 4. Scheduler policy is a product decision, not just an implementation detail

The batch results show a real tradeoff:

- higher `max_active_jobs` reduces queue wait and can improve total makespan
- the same setting can dramatically increase per-job elapsed time

That means the shared execution service should not stop at "bounded concurrency exists." It eventually needs policy that can distinguish:

- interactive or foreground materialization
- background batch throughput
- heavier pipelines whose kernels already saturate CPU and memory differently

That initial step is now in the repo: batch callers can express intent through `execution_mode`, the shared service resolves an effective concurrency from plan-derived workload hints, and status/benchmark results carry both the resolved mode and the scheduler reason instead of hiding that decision.

### 5. The first plan-aware policy is useful, but not yet discriminative enough

The new policy surface is working, but the planner currently summarized both the AGC and analytic batches as the same kind of work:

- `max_memory_cost_class = medium`
- `max_estimated_peak_memory_bytes = 201326592`
- `max_expected_partition_count = 5`

That means `auto` resolved both scenarios using the same `auto_medium_cost_batch` rule even though the runtime results were meaningfully different.

That is a useful failure mode because it points to the next concrete improvement:

- keep the shared scheduler surface
- keep the existing benchmark harness
- improve the planner and execution traits so heavier trace-local compute is distinguished from lighter trace-local compute before changing policy presets again

### 6. Benchmark outputs are now reproducible at the policy layer

The batch benchmark JSON now records not only elapsed time and queue wait, but also the policy inputs and outputs that shaped the run:

- requested concurrency
- effective concurrency
- execution mode
- scheduler reason
- worker budget
- machine-level cap
- worst-stage memory class
- worst-stage estimated peak memory

That makes later scheduler tuning materially safer because a change in performance can now be compared against the exact resolved policy, rather than guessing which concurrency rule happened to be active.

### 7. The richer stage-classification model is justified

The new stage summaries did the job they needed to do:

- AGC and analytic no longer collapse to the same trace-local execution class
- neighborhood plans now advertise themselves as high-CPU, high-IO, low-efficiency work before we touch scheduler defaults
- benchmark JSON now carries enough execution-shape metadata to explain why two pipelines behave differently

That means the next scheduler-policy step can be benchmark-led instead of heuristic-only.

### 8. Neighborhood is the next major runtime gap

The new baseline makes the priority visible.

- Neighborhood preview ranges from sub-second to roughly `11 s` across the first window ladder.
- Neighborhood full-volume processing ranges from about `6.3 min` to `98.7 min`.
- The current prefix/no-prefix experiments do not show a compelling shortcut on the current path.

So the next optimization work outside the parallel chunk-planning branch should stay focused on neighborhood execution and benchmarking, not on more trace-local partition-size tuning.

## OpenDTect-Inspired Experiments To Benchmark Before Implementing

Looking at the local OpenDTect codebase suggests several optimization strategies that are worth benchmarking in Ophiolite before building them deeply into the runtime.

### 1. Adaptive memory-aware partition sizing

OpenDTect does not rely on one static chunk size. In [`volprocchainexec.cc`](../../../OpendTect/src/VolumeProcessing/volprocchainexec.cc), `ChainExecutor::checkAndSplit(...)` and `nrChunks(...)` estimate required memory, compare that with free system memory, and keep splitting until execution fits both total and contiguous-memory constraints.

Ophiolite benchmark to add:

- compare fixed `partition_target_mib` against an adaptive target derived from plan-estimated peak memory and available system memory
- measure whether adaptive sizing beats static `64 MiB` on AGC, analytic, and future neighborhood workloads

### 2. Separate read/decode parallelism from compute parallelism

In [`seisparallelreader.cc`](../../../OpendTect/src/Seis/seisparallelreader.cc), `ParallelReader::doPrepare(...)` pre-splits ranges and fills a destination pack in parallel instead of treating all work as one read-compute-write blob.

Ophiolite benchmark to add:

- current end-to-end partition task versus a staged path with dedicated parallel read/decode followed by compute
- compare throughput and per-job latency on large trace-local pipelines and secondary-volume pipelines

### 3. Explicit preload for repeated access patterns

OpenDTect has a distinct preload path in [`seispreload.cc`](../../../OpendTect/src/Seis/seispreload.cc) through `PreLoader::load(...)`.

Ophiolite benchmark to add:

- repeated preview or batch runs with and without an explicit preload phase
- secondary-volume arithmetic and repeated-batch workloads where the same source tiles are touched many times

### 4. Halo-expanded reads at planning time

In [`seisselection.cc`](../../../OpendTect/src/Seis/seisselection.cc), `Seis::SelData::extendH(...)` expands the requested selection by stepout before execution.

Ophiolite benchmark to add:

- neighborhood execution using explicit halo-expanded partition reads versus the current temporary-prefix-materialization approach
- compare total bytes read, temp bytes written, and wall time

### 5. Resume-from-missing work instead of restart-from-zero

OpenDTect's [`seisjobexecprov.cc`](../../../OpendTect/src/Seis/seisjobexecprov.cc) tracks `"Nr of Inlines per Job"` and can recover missing lines with `getMissingLines(...)`.

Ophiolite benchmark to add:

- interrupt a multi-partition job mid-run
- resume using finalized checkpoints only
- compare resumed completion time against full recompute

### 6. Distributed execution should stay downstream of chunkability

OpenDTect's distributed layer in [`mmbatchjobdispatch.cc`](../../../OpendTect/src/MMProc/mmbatchjobdispatch.cc) and [`uiseismmproc.cc`](../../../OpendTect/src/uiSeis/uiseismmproc.cc) is mostly a dispatch shell around already chunkable jobs.

Ophiolite implication:

- benchmark local chunkability, restartability, and planner-quality first
- do not jump to remote executors until those local benchmarks are strong

## Current Gaps

This work is meaningful, but it does not complete the execution roadmap.

Still missing or intentionally deferred:

- distributed execution across machines
- halo-aware neighborhood partition scheduling
- partition-level retries beyond the current trace-local happy path
- stronger chunk-shape policy for derived outputs
- deeper adaptive scheduler policy beyond the current first-pass plan-aware default
- batch benchmarks that use multiple distinct cubes rather than repeating one imported cube

The current benchmark harness now measures both the partition-aware single-job path and the shared-service batch path, but it is still trace-local only and does not yet benchmark neighborhood-style operators or mixed-dataset workloads.

## Recommended Next Steps

Based on the implementation and the F3 results, the next steps should be:

1. Improve plan-level workload classification so `auto` can distinguish lighter AGC-style trace-local work from heavier analytic-style trace-local work.
2. Add adaptive partition-sizing benchmarks inspired by OpenDTect's memory-aware chunk splitting before hard-coding more partition policy.
3. Add benchmark coverage for staged read/decode plus compute, not only the current end-to-end partition task shape.
4. Add planner-driven chunk-shape policy instead of always inheriting the source chunking fallback.
5. Add halo-aware planning and benchmark coverage for post-stack neighborhood operators, including halo-expanded reads.
6. Add resume-after-interruption benchmarks using finalized checkpoints.
7. Only then revisit remote or distributed executors.

## Bottom Line

The repository now has the beginnings of a credible execution architecture for Ophiolite SDK/API use:

- explicit plans
- bounded local orchestration
- first-class batch semantics
- first-pass plan-aware batch scheduler modes and reasons
- partition-aware trace-local runtime execution
- actual job execution telemetry
- headless actual-data benchmarking

On the F3 cube, that translated into real gains:

- about `1.26x` for the AGC benchmark at `64 MiB`
- about `1.07x` for the analytic benchmark at `64 MiB`
- about `1.47x` AGC batch-throughput improvement at `max_active_jobs = 4`
- about `1.07x` analytic batch-throughput improvement at `max_active_jobs = 4`
- `auto` beating both `conservative` and `throughput` for AGC batch mode
- `conservative` slightly beating `auto` and clearly beating `throughput` for the heavier analytic batch mode

That is enough evidence to keep pushing this architecture forward. It is also enough evidence to stop treating scheduler policy as "set a number and hope." The next work should stay benchmark-led, especially where OpenDTect already suggests promising strategies such as adaptive chunking, staged parallel reads, and restartable chunk execution.

