---
name: benchmark
description: Plan, review, and run trustworthy performance benchmarks. Use when Codex needs to design benchmark methodology, assess whether results are valid, choose what can run in parallel, reduce measurement noise, compare implementations, investigate regressions, adapt benchmark launch policy to the current machine, or document benchmark evidence for engineering decisions.
---

# Benchmark

## Overview

Use this skill to keep benchmark work decision-grade rather than anecdotal. Separate exploratory timing from authoritative measurement, control local contention, and reject results gathered under noisy or changing conditions.

When the benchmark task is inside the `ophiolite` or `traceboost` workspace, read [`references/ophiolite.md`](references/ophiolite.md) for repo-specific benchmark surfaces, dataset conventions, and interpretation rules.

When local benchmark execution is needed inside the `ophiolite` workspace on Windows, use the repo launcher `scripts/run-benchmark-gated.ps1` instead of launching heavy benchmark commands directly. Let the launcher inspect the machine, serialize heavy workloads, pin worker counts, rewrite leading Windows `cargo ...` commands through `scripts/windows-msvc-cargo.cmd`, and block authoritative runs until the host is quiet enough.

When the user wants benchmark execution on a cleaner host instead of the shared workstation, prefer the manual GitHub Actions workflow `.github/workflows/benchmark.yml` on a dedicated self-hosted Windows benchmark runner. Treat that path as the repo-preferred remote benchmark flow.

## Workflow

1. Classify the benchmark goal before running anything.
2. In `ophiolite` on Windows, prefer `scripts/run-benchmark-gated.ps1 -PrintPlan ...` before the real run so the launch policy is explicit.
3. If cleaner remote execution is needed, prefer the manual benchmark workflow on the self-hosted Windows benchmark runner instead of improvising a fresh CI path.
4. Inspect the current environment when local access is available.
5. Decide what must be isolated and what can be approximate.
6. Standardize the environment, workload, and thread count.
7. Run enough repetitions to observe variance.
8. Report both numbers and measurement conditions.

## Classify The Goal

Put every benchmark request into one of these modes and say which mode you are using.

- `smoke`: Confirm the code path works and is not catastrophically slow. Parallel execution is acceptable. Numbers are not publishable.
- `development`: Compare coarse options during implementation. Moderate noise is acceptable, but keep setup stable enough to avoid misleading direction changes.
- `authoritative`: Produce numbers that drive defaults, regression calls, or published claims. Run in controlled conditions and do not mix with unrelated heavy work.

If the user does not specify, infer the strictest plausible mode from the decision being made. Default to `authoritative` for regression validation, published results, sizing decisions, or concurrency, tile, and partition policy choices.

## Inspect And Adapt To The Current Environment

When local inspection is possible, inspect the machine before proposing or launching benchmark work. Do not assume a generic workstation.

Gather at least:

- physical and logical core count
- current total CPU load
- free physical memory
- free disk space on the volume that holds the dataset, temp outputs, and benchmark results
- currently running heavy processes that can contend for CPU, memory, or disk

State the discovered values and say how they change the benchmark plan. Prefer environment-derived launch policy over generic defaults.

If local inspection is not possible, say that the recommendation is a generic fallback.

When local inspection is possible in `ophiolite`, do not stop at prose advice if a benchmark is actually being launched. Use the repo launcher so the environment check is part of execution rather than a separate manual step.

When the run is remote through GitHub Actions, report that the launcher still governs the benchmark process on the runner, but host cleanliness now comes from the dedicated runner rather than the interactive workstation.

## Budget CPU Explicitly

Do not let multiple heavy jobs each auto-detect and consume the full machine.

Use these rules:

- For interactive work, reserve enough CPU for the terminal, editor, browser, and OS. Prefer a capped benchmark thread count rather than full-machine auto-threading.
- For authoritative runs, prefer one heavy benchmark job on a quiet machine. If other heavy jobs are still active, either stop them or reduce benchmark workers and downgrade confidence.
- If more than one heavy benchmark must run at once, divide the CPU budget explicitly across jobs and mark the results as exploratory unless the shared-host scenario is itself the subject of the benchmark.
- Keep thread-count policy consistent across compared runs. Do not compare one run with all cores against another with a smaller cap unless thread scaling is the thing being measured.

When recommending a cap, reason from the current machine:

- start from available logical processors
- subtract budget for active non-benchmark heavy processes
- reserve headroom for the OS and interactive tools when the machine is in active use
- apply the same cap to all comparable runs

If the benchmark framework supports explicit thread controls such as `RAYON_NUM_THREADS`, benchmark-worker counts, or process affinity, prefer setting them deliberately rather than leaving all runs on auto.

In `ophiolite`, prefer the repo-standard worker env set by the launcher. On the current shared Windows workstation, treat `OPHIOLITE_BENCHMARK_WORKERS=8` as the default controlled setting for `development` and `authoritative` runs unless the benchmark question is explicitly about thread scaling.

If a Windows benchmark command begins with `cargo`, prefer letting the launcher rewrite it through `scripts/windows-msvc-cargo.cmd` so the MSVC, SQLite, and PATH hygiene rules apply automatically.

## Budget Memory And Disk Explicitly

Check resource headroom before increasing concurrency.

Memory rules:

- If free RAM is already low or memory compression and paging are active, do not add more parallel benchmark jobs.
- If the benchmark materializes large output stores or holds multiple datasets in memory, reduce job concurrency before the machine starts paging.
- If memory headroom differs substantially between compared runs, note that the comparison is contaminated.

Disk rules:

- Verify that the dataset, temp output path, and result path live on a volume with enough free space for the full run.
- Prefer the fastest local storage available for authoritative runs, and keep the storage class constant across compared runs.
- Clean stale temporary benchmark outputs when they consume meaningful space or cause extra scanning overhead.
- If antivirus, indexing, or background scans are touching the benchmark paths, call that out as a source of noise.

Do not treat extra free disk space by itself as a performance improvement. Capacity helps avoid failure and fragmentation pressure; storage speed and background activity usually matter more.

## Control The Environment

For `authoritative` runs, require these controls unless the user explicitly accepts weaker evidence:

- Run one heavy benchmark workload at a time.
- Avoid concurrent IDE indexing, builds, dev servers, browser load, local AI agents, or unrelated tests.
- Fix the thread count. Do not compare one run at auto-threading and another at a capped setting.
- Keep dataset, input parameters, build profile, and binary revision constant.
- Prefer a warm machine state that is consistent across runs. If cold-cache behavior matters, measure it deliberately as a separate case.
- Record CPU limit, affinity, priority class, and whether the machine was interactive or otherwise busy.

Treat parallel benchmark jobs as contamination unless the purpose is explicitly to test shared-host contention.

In `ophiolite`, if the run is F3-heavy, `traceboost-app`-based, or Criterion-based, default to one heavy benchmark lane at a time. Do not open a second heavy local benchmark process just because spare logical processors exist.

If an environment-aware optimization is requested, recommend concrete launch policy, not only principles. That usually means naming:

- how many heavy benchmark jobs may run at once
- what thread cap each job should use
- whether the run should be delayed until other local work stops
- whether memory or disk headroom is sufficient for the proposed run

## Decide What May Run In Parallel

Use this rule set:

- Allow parallel runs only for `smoke` work or broad exploratory screening.
- Do not run multiple CPU-heavy benchmarks in parallel if you care about latency, throughput, scaling curves, or small regressions.
- Do not compare implementation A from an idle run against implementation B from a contended run.
- Do not treat restricted-CPU parallel runs as authoritative unless the benchmark question is specifically about that restricted shared-host scenario.

If the user asks whether to run everything in parallel, recommend a selective subset for interactive work and a serialized controlled subset for final evidence.

When executing rather than only advising, encode that choice into the launch command. In `ophiolite`, that usually means:

- `smoke`: direct launch or launcher in `smoke` mode
- `development`: launcher in `development` mode
- `authoritative`: launcher in `authoritative` mode

If the execution host should be remote, encode the same mode choice into the manual GitHub workflow dispatch inputs rather than bypassing the launcher logic.

## Design The Measurement

When helping write or review a benchmark:

- Measure the user-visible behavior that matters, not only a convenient micro-operation.
- Separate setup cost, steady-state cost, and teardown cost when they answer different questions.
- Choose representative datasets and input shapes; avoid only best-case synthetic data unless the benchmark is explicitly synthetic.
- Keep output validation in place if removing it would change the workload meaningfully.
- Include warmup when JITs, caches, or lazy initialization affect early runs.
- Repeat enough times to inspect spread, not just the best number.

For benchmarks that compare concurrency strategies, storage layouts, tile sizes, or partition targets, keep every other variable fixed.

## Interpret Results Conservatively

Do not over-claim from noisy numbers.

- If variance is large, say so and avoid strong conclusions.
- If the machine was saturated or contended, downgrade confidence.
- If a result changes only by a few percent on a noisy workstation, do not call it a regression without cleaner reruns.
- If one run includes extra local load, treat the comparison as invalid.
- Report median or mean with context, and include min and max or another spread indicator when possible.

When numbers look surprising, check:

- CPU saturation
- background disk or network activity
- memory pressure or paging
- accidental debug vs release mismatch
- thread-count mismatch
- dataset mismatch
- cache-state mismatch

## Report Results With Conditions

Every serious benchmark summary should state:

- exact command or benchmark entrypoint
- code revision or branch context
- dataset or fixture used
- machine constraints relevant to performance
- thread count and concurrency model
- whether runs were isolated or contended
- repeat count and summary statistic
- major caveats that limit interpretation

Do not present a single bare timing number with no provenance.

## Common Anti-Patterns

Reject or warn on these patterns:

- benchmarking while several other heavy benchmarks are running
- using interactive workstation numbers as publishable evidence without qualification
- changing workload shape and implementation at the same time
- comparing runs from different binaries, profiles, or datasets
- reporting only the fastest run
- hiding large variance
- concluding too much from microbenchmarks when the product path is dominated by I/O, orchestration, or UI work

## Default Recommendations

Use these defaults unless the benchmark question requires something else:

- Prefer serial execution for authoritative runs.
- Prefer fixed thread counts over auto-scaling when comparing alternatives.
- Keep the machine as idle as practical.
- Separate exploratory runs from final evidence.
- Be selective: benchmark the decisions you actually need to make, not every permutation.

For `ophiolite` on the shared Windows workstation:

- use `scripts/run-benchmark-gated.ps1`
- keep heavy local benchmark families serialized
- default to `8` workers for controlled heavy runs

For `ophiolite` remote benchmark execution:

- use `.github/workflows/benchmark.yml`
- run it on a self-hosted Windows runner labeled `benchmark`
- keep the launcher in the execution path
