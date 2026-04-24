# Phase 1 Benchmark Gap Map (2026-04)

Date: 2026-04-24

Related notes:

- [docs/research/phase-1-improvement-program-2026-04.md](/Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md)
- [docs/research/external-repo-study-prompts-2026-04.md](/Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md)
- [articles/benchmarking/INTERACTIVE_SECTION_BROWSING_HARNESS_PLAN.md](/Users/sc/dev/ophiolite/articles/benchmarking/INTERACTIVE_SECTION_BROWSING_HARNESS_PLAN.md)
- [articles/benchmarking/PREVIEW_INCREMENTAL_EXECUTION_BENCHMARK_PLAN.md](/Users/sc/dev/ophiolite/articles/benchmarking/PREVIEW_INCREMENTAL_EXECUTION_BENCHMARK_PLAN.md)
- [articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md](/Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md)

## Purpose

This note turns Wave 0 from a general instruction into a bounded benchmark map for Phase 1.

It does three things:

- ties the current harnesses to Waves 1-4
- names the minimum missing benchmark targets that should exist before kernel or codec work starts
- fixes the validation bar for spectral, prestack, and future `tbvolc` work without proposing a new benchmark framework

This note is intentionally repo-specific. It does not reopen the earlier decisions around chunking, prefix caches, or the existing evidence surfaces.

## Existing Harness Inventory

The repo already has enough benchmark surface area to avoid inventing anything new. The main issue is uneven coverage.

| Surface | Current role | Best evidence level | Best Phase 1 fit | Current blind spot |
| --- | --- | --- | --- | --- |
| `traceboost-app benchmark-trace-local-processing` and `benchmark-trace-local-batch-processing` | Actual-data trace-local materialization, partition sizing, batch scheduler behavior | authoritative | Wave 2, Wave 3 | no prestack cases, no `tbvolc` source path |
| `crates/ophiolite-seismic-runtime/src/bin/section_tile_bench.rs` | Full-section vs tiled/decimated section reads, payload size, LOD effects | development to authoritative depending on host | Wave 1 | no compressed-store path, no app-layer viewport checks |
| `crates/ophiolite-seismic-runtime/src/bin/preview_incremental_bench.rs` | Same-section rerun vs unchanged-prefix reuse on section preview | development | Wave 1, Wave 3 | only trace-local preview; no prestack or compressed-input coverage |
| `crates/ophiolite-seismic-runtime/benches/compute_storage.rs` | Synthetic microbench for trace-local preview/materialize operators | component | Wave 3 | spectral matrix is incomplete |
| `crates/ophiolite-seismic-runtime/src/bin/compute_storage_bench.rs` | Storage/layout and trace-local preview/apply comparisons across store types | component | Wave 1, Wave 4 | no `tbvolc` candidate, no prestack targets |
| `crates/ophiolite-seismic-runtime/benches/post_stack_neighborhood_kernels.rs` | Criterion coverage for dip, similarity, and local stats preview kernels | component | Wave 3 | no prestack gather/semblance coverage |
| `apps/traceboost-demo/src-tauri/src/preview_session_bench.rs` | Large-F3 exploratory desktop preview session timing for trace-local and dip preview | exploratory | Wave 1, Wave 2 | ignored-test harness, not a controlled benchmark surface |
| `apps/traceboost-demo/src-tauri/src/processing_cache_bench.rs` | Exploratory cache/cold rerun comparisons for trace-local processing | exploratory | Wave 1, Wave 2 | not decision-grade for current Phase 1 defaults |

## Wave Map

### Wave 1: Layout And Catalog Foundations

Relevant existing harnesses:

- `section_tile_bench`
- `preview_incremental_bench`
- `compute_storage_bench` tile sweep and preview/apply cases
- `preview_session_bench` as exploratory context only

What is already covered well enough:

- full-section versus focused-tile section access
- payload shrinkage from LOD and narrowed viewport requests
- same-section preview rerun versus prefix reuse
- `tbvol` tile-shape sensitivity on trace-local preview/apply paths

Minimum missing target before Wave 1 code changes:

- Add one named baseline target set that uses the existing runners on the same F3-family `tbvol` store:
  - `section_tile_bench` for inline and xline `full_section`, `overview_fit`, and `focus_tile_lod_{0,1}`
  - `preview_incremental_bench` for inline and xline `late_scalar_edit`, `late_filter_edit`, and `late_agc_edit`
  - `compute_storage_bench sweep-tbvol` on the same store when tile-geometry or section-assembly changes are part of the patch

Why this is the minimum:

- Wave 1 does not need a new harness
- Wave 1 does need one explicit baseline bundle so later layout or buffer work is compared against the same store, axis set, and payload-sensitive cases

### Wave 2: Runtime Orchestration And Stage Evidence

Relevant existing harnesses:

- `traceboost-app benchmark-trace-local-processing`
- `traceboost-app benchmark-trace-local-batch-processing`
- `preview_session_bench` as exploratory desktop-path context

What is already covered well enough:

- partition-aware trace-local materialization on actual F3 data
- batch makespan and queueing behavior
- scheduler-mode comparisons for `auto`, `conservative`, and `throughput`

Minimum missing target before Wave 2 code changes:

- None at the framework level.
- The required work is to keep using the current authoritative targets as the runtime baseline:
  - `--scenario agc`
  - `--scenario analytic`
  - single-job serial versus partitioned
  - four-job batch runs with fixed partition target and scheduler mode

Why no new target is required:

- Wave 2 is changing runtime ownership and stage evidence, not numeric behavior
- the existing headless `traceboost-app` surfaces already measure the runtime decisions that matter

### Wave 3: Numeric Kernel Improvements

Relevant existing harnesses:

- `compute_storage.rs`
- `compute_storage_bench.rs`
- `post_stack_neighborhood_kernels.rs`
- `traceboost-app benchmark-trace-local-processing`
- `preview_incremental_bench`

What is already covered well enough:

- trace-local AGC and analytic whole-volume scenarios on actual data
- post-stack neighborhood preview kernels for dip, similarity, and local stats
- microbench coverage for `amplitude_scalar`, `trace_rms_normalize`, `phase_rotation`, `bandpass_filter`, and `bandpass_plus_phase_rotation`

Minimum missing targets before Wave 3 code changes:

1. Extend the existing trace-local spectral microbench matrix in `compute_storage.rs` and `compute_storage_bench.rs` to cover the operators that are implemented in `compute.rs` but not benchmarked directly today:
   - `lowpass_filter`
   - `highpass_filter`
   - `envelope`
   - `instantaneous_phase`
   - `instantaneous_frequency`
   - `sweetness`

2. Add one explicit spectral-stack target to the existing trace-local surfaces:
   - a fixed analytic stack made of `trace_rms_normalize -> envelope -> instantaneous_phase -> instantaneous_frequency -> sweetness`
   - measured both as a component path and as an actual-data trace-local path

3. Add the first prestack benchmark surface instead of changing prestack code blindly.
   - Minimum target set:
     - `semblance_panel_constant_velocity`
     - `velocity_autopick`
     - `gather_nmo_correction`
     - `gather_stretch_mute`
     - `gather_offset_mute`
   - These should live on existing crate benchmark surfaces, not on a new framework.

Why these are the real gaps:

- spectral correctness is already tested, but spectral performance evidence is incomplete
- prestack code has validation tests, but no benchmark harness at all
- Phase 1 kernel work should not start with only AGC and post-stack dip numbers

### Wave 4: Compression And Codec Architecture

Relevant existing harnesses:

- `compute_storage_bench.rs`
- `section_tile_bench.rs`
- `tbvolc_transcode.rs`

What is already covered well enough:

- `tbvol` versus Zarr layout comparisons
- section and trace-local preview/apply timing on uncompressed store paths
- offline `tbvol <-> tbvolc` transcode entry points exist

Minimum missing targets before Wave 4 code changes:

1. Add `tbvolc` as a benchmarked storage candidate on the existing compute/storage bench path.
   - Minimum metrics:
     - inline section read latency
     - xline section read latency
     - trace-local preview latency
     - trace-local apply/materialize latency
     - input-store bytes and file counts versus `tbvol`

2. Add one transcode benchmark target around the existing `tbvolc_transcode` path.
   - Minimum metrics:
     - `tbvol -> tbvolc` elapsed time
     - `tbvolc -> tbvol` elapsed time
     - archive size fraction versus source `tbvol`

3. Add one section-access comparison that uses the existing section harness semantics on `tbvol` and `tbvolc` with the same shape and tile geometry.

Why these are the real gaps:

- there is no benchmark evidence yet for compressed-input read cost
- there is no benchmark evidence yet for whether `tbvolc` helps or hurts the section and preview paths that matter to Phase 1
- Wave 4 should not begin with only storage-ratio anecdotes

## Minimum Benchmark Targets To Add

This is the bounded Batch 0A target list.

### Add before Wave 3 starts

- Spectral operator microbench targets for `lowpass_filter`, `highpass_filter`, `envelope`, `instantaneous_phase`, `instantaneous_frequency`, and `sweetness`
- One fixed analytic-stack target on both component and actual-data trace-local paths
- First prestack benchmark target set for semblance, autopick, NMO, stretch mute, and offset mute

### Add before Wave 4 starts

- `tbvolc` storage candidate in the existing compute/storage bench matrix
- `tbvol <-> tbvolc` transcode timing target
- `tbvol` versus `tbvolc` section-access comparison on matching store metadata

### No new target required before code starts

- Wave 2 runtime orchestration beyond rerunning the current actual-data runtime targets

## Validation Bar

Performance numbers are not enough for Phase 1. The repo already contains the right correctness ideas in tests; the gap is that they were not yet written down as the benchmark acceptance bar.

### Spectral Work

Every spectral optimization should keep both property checks and benchmark-result comparisons.

Required validation bar:

- `phase_rotation`:
  - 180-degree inversion remains within `1.0e-5` absolute error
  - 90-degree sine-to-cosine rotation stays within `3.0e-3` max error
- `envelope`:
  - mean absolute error against analytic magnitude stays below `2.0e-3`
- `instantaneous_phase`:
  - reconstructing the original sinusoid from envelope plus phase stays below `2.5e-3` mean absolute error
- `instantaneous_frequency`:
  - periodic sinusoid tracking stays below `0.2` mean absolute error in Hz
- `sweetness`:
  - mean absolute error against the stabilized analytic expectation stays below `2.5e-3`
- `lowpass`, `highpass`, and `bandpass`:
  - preserved band keeps at least `70%` of original amplitude
  - rejected band is reduced below `25%` of original amplitude

Benchmark-specific rule:

- every new spectral benchmark case must compute the current baseline output and reject the optimization if the benchmarked output violates the corresponding rule above

### Prestack Work

Prestack work should not be accepted on timing wins alone.

Required validation bar:

- semblance synthetic hyperbola:
  - the peak remains at the true trial velocity
- velocity autopick:
  - picked velocities stay within one velocity step of the synthetic truth
  - picked times stay on the requested sample grid
- NMO correction:
  - flattened-event peak remains within one sample of the zero-offset target on every trace
- stretch mute:
  - early far-offset muted samples remain zeroed where the current tests expect zero
- offset mute:
  - muted trace mask remains exact for traces outside the requested offset window

Benchmark-specific rule:

- every prestack benchmark case must carry a numerical regression check on the same synthetic gather or semblance panel fixture used by the timing run

### Future `tbvolc` Work

`tbvolc` is currently lossless. Its validation bar should be exact until that assumption changes.

Required validation bar:

- `tbvol -> tbvolc -> tbvol` roundtrip preserves amplitudes bit-for-bit
- occupancy bytes remain exact
- `store_id`, `shape`, `tile_shape`, axes, source identity, and occupancy flags remain compatible
- `describe_tbvol_archive_sibling(...)` should report no compatibility warnings for the benchmark pair
- section reads from `tbvolc` match `tbvol` exactly on the same tile coordinates
- trace-local preview or apply outputs sourced from `tbvolc` match the `tbvol` baseline exactly

Benchmark-specific rule:

- no Wave 4 benchmark result should be used to justify codec or reader changes unless the matching exactness check passes first

## Doc-Only Patch Plan

This Batch 0A patch should stay small.

1. Add this research note.
2. Use it as the Wave 0 reference for later benchmark or kernel batches.
3. Do not add result JSON, a new framework, or runtime/storage code as part of this batch.
