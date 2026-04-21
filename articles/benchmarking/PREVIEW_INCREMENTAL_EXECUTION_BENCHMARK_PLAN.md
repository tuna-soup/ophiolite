# Preview Incremental Execution Benchmark Plan

## Purpose

This note defines the next research phase after the processing-cache experiment.

The goal is not to add more production cache machinery yet. The goal is to determine whether TraceBoost can materially improve repeated preview interaction when a user tweaks later operators in a trace-local pipeline.

The key question is:

> can repeated preview updates on the same displayed section become near-instant by reusing unchanged work at section or block granularity?

## Why this is the next step

The earlier processing-cache benchmark established two important facts:

- exact full-pipeline reruns are worth caching
- automatic hidden whole-volume prefix checkpoints are not worth keeping in the current implementation

That result strongly suggests that the cache granularity was wrong for late-edit reuse. Whole-volume intermediate stores are too expensive to write and read back relative to the saved compute.

Preview interaction is a better next target because:

- it touches only a requested section rather than the whole volume
- users are sensitive to interactive latency there
- unchanged-prefix reuse can be evaluated without committing to large persistent hidden artifacts

## Current preview architecture

The current trace-local preview path is already simple and localized.

### Frontend/Tauri entry

- [lib.rs](C:/Users/crooijmanss/dev/TraceBoost/app/traceboost-frontend/src-tauri/src/lib.rs#L1187) calls `preview_processing`

### App layer

- [lib.rs](C:/Users/crooijmanss/dev/TraceBoost/app/traceboost-app/src/lib.rs#L154) opens the store and delegates to `preview_processing_section_view`

### Runtime section preview

- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L484) validates the pipeline and calls `preview_section_view`
- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L470) opens the `tbvol`, reads the requested section plane, and prepares any secondary readers
- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L673) reads the section plane and applies the pipeline trace-by-trace in memory

### Section fetch

- [section_assembler.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/section_assembler.rs#L9) builds a section by iterating all tiles intersecting that section and copying the relevant traces into a dense section plane

### Trace-local execution

- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L785) applies each operator per trace
- spectral operators reuse per-trace workspace, but the whole unchanged prefix is still rerun for every preview request

This matters because it gives a clean baseline:

- read section
- apply full pipeline
- return section view

That is the path the benchmark should instrument and compare against.

## Research from other ecosystems

The consistent pattern across modern open-source build/data pipeline systems is:

- reuse exact prior outputs when all semantic inputs match
- keep cache identity content-addressed
- choose a reuse unit small enough that reuse is cheaper than recompute

Examples:

- Bazel and DVC cache exact action/stage outputs by content identity
- Nextflow and Snakemake resume cached process outputs by hashed inputs/params/environment
- Prefect caches task results by cache keys derived from inputs and task definition
- Pachyderm gets incremental wins by splitting work into smaller data units called datums
- Dask, Zarr, and TensorStore focus on chunked arrays, chunk alignment, cache size limits, and avoiding pathological chunk shapes

The main lesson for TraceBoost is:

> if unchanged-prefix reuse is going to work, the reusable unit likely needs to be smaller than a whole intermediate volume

## Working hypothesis

The next research hypothesis is:

> preview interaction can likely be improved by reusing unchanged work at section or block granularity for locality-bounded operators, or by fusing recomputation more intelligently, without persisting whole intermediate volumes

This leads to three candidate strategies to benchmark.

## Candidate strategies

### Strategy A: Baseline

Current behavior:

- read requested section from source store
- run the entire trace-local pipeline on that section
- return the section

This remains the correctness baseline and the control.

### Strategy B: Ephemeral unchanged-prefix section/block reuse

Reuse a previously computed section or section-block result for the unchanged prefix in the current interactive session, then rerun only the modified suffix.

Important constraints:

- in-memory or short-lived only
- no persistent hidden whole-volume artifacts
- limited to operators whose dependency footprint is declared and bounded

### Strategy C: Fused selective recompute

Do not cache intermediate section results at all. Instead:

- keep execution scoped to the requested section or section-block
- rerun the full logical pipeline on that smaller working set
- rely on the fact that a section is much cheaper than a volume

This may beat caching if storage or copy overhead dominates.

## Operator metadata required before any serious optimization

The runtime needs explicit operator dependency metadata.

Each trace-local operator should eventually declare:

- determinism
- dependency scope
- sample radius
- inline radius
- crossline radius
- whether output layout equals input layout
- whether the operator is eligible for unchanged-prefix reuse

Initial taxonomy:

- `trace_local`
- `sample_window`
- `xy_window`
- `global`

The first prototype should only include:

- `trace_local`
- bounded `sample_window`

`xy_window` and `global` operators should be benchmarked later.

## Benchmark goals

The benchmark should answer these questions:

1. How much of preview time is section fetch versus operator execution?
2. Is repeated late-edit interaction on the same section already fast enough?
3. Does ephemeral unchanged-prefix reuse beat the baseline materially?
4. Does fused selective recompute beat ephemeral reuse?
5. Are current `tbvol` tile shapes acceptable for preview access patterns?

## Benchmark dataset

Use the real larger imported F3 `tbvol`:

- `f3_dataset-smoke.tbvol`
- shape: `651 x 951 x 462`
- tile shape: `41 x 56 x 462`

This is currently the most representative local benchmark dataset available for this work.

## Benchmark scenarios

### Sections

Use one fixed inline and one fixed crossline section.

Recommended starting points:

- inline near the middle of the survey
- crossline near the middle of the survey

This avoids boundary artifacts dominating the first benchmark pass.

### Pipelines

Keep the first matrix small:

- amplitude scalar
- phase rotation
- one filter
- AGC

Representative pipelines:

- 4-step pipeline with only trace-local operators
- 4-step pipeline with one bounded-window operator near the end
- 6-step pipeline with a modified late-stage parameter

### Edit patterns

Simulate repeated user interaction by:

- changing the final amplitude scalar
- changing phase rotation
- changing filter parameters
- changing AGC window
- toggling between two nearby presets

### Warm/cold states

Measure:

- cold run
- repeated run on same section
- repeated run after a late-stage parameter change

## Metrics to capture

Every benchmark case should record:

- total preview latency
- section fetch latency
- pipeline execution latency
- result shaping/render-prep latency
- peak or approximate working memory if practical

For reuse strategies, also record:

- cache lookup overhead
- hit/miss status
- reused prefix length
- estimated bytes reused

## Correctness rules

Every candidate optimization must compare against the baseline result.

For each case:

- compute the baseline preview output
- compute the optimized preview output
- assert numerical equivalence within an agreed tolerance

Performance numbers without correctness checks are not actionable.

## Acceptance bar

This should be stricter than the whole-volume cache experiment.

Recommended acceptance criteria:

- repeated tweaks on the same section show an obvious latency win
- the optimized path is clearly faster than the baseline on the large F3 dataset
- complexity is proportional to the gain

Suggested initial bar:

- at least `30%` latency reduction on repeated late-edit preview updates for eligible operators
- no major regression on cold preview runs

If the candidate path does not clear that bar, it should not be productized.

## Proposed implementation order

### Phase 1: Benchmark and instrumentation only

Add a runtime benchmark harness that:

- loads one or two fixed sections
- runs representative pipelines
- records per-phase timings
- emits markdown-table output

Recommended home:

- `ophiolite-seismic-runtime`

This keeps the benchmark close to the real section-execution primitives.

Current implementation:

- [preview_incremental_bench.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/bin/preview_incremental_bench.rs)

Example command:

```powershell
cargo run -p ophiolite-seismic-runtime --bin preview_incremental_bench --release -- --store "H:\traceboost-bench\f3_dataset-smoke.tbvol" --iterations 3 --format text
```

### Phase 2: Operator metadata

Add operator dependency metadata without changing product behavior.

This should be enough to classify:

- `trace_local`
- bounded `sample_window`
- unsupported for prototype

Current implementation status:

- Trace-local operator dependency metadata now distinguishes:
  - `pointwise`
  - `bounded_window`
  - `whole_trace`
- It also records spatial dependency and whether same-section ephemeral prefix reuse is safe.

### Phase 3: Prototype Strategy B

Prototype ephemeral unchanged-prefix reuse for a narrow set of operators.

Scope:

- same section only
- same session only
- in-memory only
- no persistent disk cache

### Phase 4: Prototype Strategy C

Prototype fused selective recompute on the same benchmark harness.

Compare it directly against Strategy B and the baseline.

### Phase 5: Decision

Possible outcomes:

- `A` ephemeral reuse wins clearly
- `B` fused recompute wins clearly
- `C` neither wins enough, so keep the current preview path

## What should not happen yet

Do not do these before the benchmark proves the case:

- add more persistent cache product surface
- add user-facing preview cache settings
- add whole-volume hidden checkpointing back
- generalize to gathers
- add new storage formats just for the cache

## Concrete code touchpoints for Phase 1

Primary runtime targets:

- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L484)
- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L578)
- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L626)
- [compute.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs#L785)
- [section_assembler.rs](C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/section_assembler.rs#L9)

Thin app/frontend wrappers if needed:

- [lib.rs](C:/Users/crooijmanss/dev/TraceBoost/app/traceboost-app/src/lib.rs#L154)
- [lib.rs](C:/Users/crooijmanss/dev/TraceBoost/app/traceboost-frontend/src-tauri/src/lib.rs#L1187)

## Final recommendation

The next deliverable should be:

- a preview benchmark harness in runtime
- operator dependency metadata for a narrow initial operator set
- a measured decision between ephemeral reuse, fused recompute, or no change

It should not be another speculative production cache feature.

## Initial benchmark signal

The first runtime benchmark run on April 8, 2026 used:

- dataset: `f3_dataset-smoke.tbvol`
- shape: `651 x 951 x 462`
- tile shape: `41 x 56 x 462`
- sections: middle inline and middle crossline
- iterations: `3`
- strategies:
  - baseline full preview rerun
  - ephemeral in-memory prefix reuse on the same section

Selected medians:

| Axis | Scenario | Baseline ms | Reuse ms | Improvement |
| --- | --- | ---: | ---: | ---: |
| inline | `late_scalar_edit` | `5.267` | `2.221` | `57.8%` |
| inline | `late_filter_edit` | `4.261` | `3.108` | `27.1%` |
| inline | `late_agc_edit` | `2.661` | `1.723` | `35.2%` |
| xline | `late_scalar_edit` | `4.244` | `2.352` | `44.6%` |
| xline | `late_filter_edit` | `3.054` | `2.721` | `10.9%` |
| xline | `late_agc_edit` | `2.229` | `1.386` | `37.8%` |

Interpretation:

- preview interaction is a much better target than whole-volume hidden checkpointing
- simple in-memory unchanged-prefix reuse on the same section already clears the provisional `30%` bar for several late-edit cases
- the gains are not uniform, which means operator class still matters

The richer dependency summaries from the current runtime benchmark make that last point more concrete:

- suffixes dominated by `pointwise` or `bounded_window` work benefit more
- suffixes dominated by `whole_trace` work benefit less, because the changed operator itself still costs a meaningful amount
- current trace-local operators are all still `single_trace` spatially, which is favorable for same-section reuse

## Runtime cache prototype result

The next prototype step added a real in-memory session-scoped prefix cache in runtime:

- longest-prefix lookup by `(store, axis, section index, pipeline prefix)`
- bounded in-memory cache
- same-section only
- same-session only
- no persistent disk state

The benchmark now compares:

- low-level `fused_selective_recompute`
- low-level `ideal_ephemeral_prefix_reuse`
- actual `runtime_full_preview_api`
- actual `runtime_session_prefix_cache`
- `pinned_session_full_preview`
- `pinned_session_prefix_cache`

The two pinned-session rows are now the most important apples-to-apples comparison because they remove store-open noise and isolate the runtime helper itself.

The first pinned-session result was disappointing: the cache helper was often slower than just rerunning preview inside the same already-open session. Profiling by benchmark structure pointed to two self-inflicted costs:

- cache key construction was paying for path canonicalization and JSON serialization on every lookup
- cache hits replayed the suffix one operator at a time, turning one trace pass into multiple passes over the same section

The helper was then optimized to:

- use cheap in-process hashed identities instead of canonicalized path plus JSON prefix strings
- apply the remaining suffix in one pipeline pass after a cache hit

Selected medians after that optimization:

| Axis | Scenario | Pinned Baseline ms | Pinned Cache ms | Improvement |
| --- | --- | ---: | ---: | ---: |
| inline | `late_scalar_edit` | `4.488` | `1.447` | `67.8%` |
| inline | `late_filter_edit` | `3.872` | `2.665` | `31.2%` |
| inline | `late_agc_edit` | `3.628` | `1.730` | `52.3%` |
| xline | `late_scalar_edit` | `2.170` | `1.128` | `48.0%` |
| xline | `late_filter_edit` | `2.447` | `1.772` | `27.6%` |
| xline | `late_agc_edit` | `1.868` | `1.141` | `38.9%` |

Interpretation:

- the earlier regression was mostly helper overhead, not a fundamental problem with same-section prefix reuse
- after removing that overhead, the pinned-session cache path now clears the provisional `30%` bar in most cases and comes close in the remaining one
- filter edits still help less than scalar or AGC edits because the changed suffix is itself a relatively expensive whole-trace operator
- preview prefix reuse now looks like a credible runtime optimization path, unlike the earlier whole-volume hidden-checkpoint design

So the current conclusion is:

- same-session preview prefix reuse is technically viable
- the relevant comparison is the pinned-session path, not the idealized low-level upper bound and not the store-open-dominated API path alone
- the next step should be product integration only after a clean API shape and cold-path regression check, not more speculative cache architecture

## Desktop integration check

The next step after the runtime prototype was a narrow desktop-only integration:

- keep the cache in memory only
- keep it process-local
- key it by active preview dataset/store session
- use it only for trace-local section preview in the Tauri backend

That path was benchmarked directly against the existing stateless `traceboost_app::preview_processing` flow on the same large F3 `tbvol`.

Selected medians from the integrated desktop benchmark:

| Axis | Scenario | Desktop Stateless ms | Desktop Session Cache ms | Improvement |
| --- | --- | ---: | ---: | ---: |
| inline | `late_scalar_edit` | `5.760` | `1.622` | `71.8%` |
| inline | `late_filter_edit` | `6.082` | `2.486` | `59.1%` |
| inline | `late_agc_edit` | `6.221` | `2.039` | `67.2%` |
| xline | `late_scalar_edit` | `7.224` | `1.331` | `81.6%` |
| xline | `late_filter_edit` | `6.630` | `2.163` | `67.4%` |
| xline | `late_agc_edit` | `6.103` | `1.602` | `73.8%` |

Interpretation:

- the runtime win survives real desktop integration
- the previous fear that app-layer/Tauri overhead would erase the benefit did not materialize in this benchmark
- the integrated same-session preview path is now materially better than the old stateless desktop preview path for all tested late-edit cases

So the current design recommendation is stronger than before:

- keep exact full-result reuse for full processing reruns
- do not revive automatic hidden whole-volume prefix caching
- do keep the new same-session in-memory preview prefix reuse path, because it is now justified by both runtime-level and desktop-integrated measurements

## External references

- Bazel caching: https://bazel.build/remote/caching
- DVC run cache: https://doc.dvc.org/user-guide/pipelines/run-cache
- Nextflow cache and resume: https://www.nextflow.io/docs/stable/cache-and-resume.html
- Snakemake between-workflow caching: https://snakemake.readthedocs.io/en/v8.20.4/executing/caching.html
- Prefect caching: https://docs.prefect.io/v3/concepts/caching
- Dask array best practices: https://docs.dask.org/en/stable/array-best-practices.html
- Dask chunk sizing: https://blog.dask.org/2021/11/02/choosing-dask-chunk-sizes
- Zarr performance guidance: https://zarr.readthedocs.io/en/stable/user-guide/performance/
- TensorStore docs: https://google.github.io/tensorstore/
- Pachyderm input/data partitioning: https://docs.pachyderm.com/products/mldm/latest/build-dags/pipeline-spec/input-pfs/
- lakeFS branching model: https://docs.lakefs.io/
