# Phase 1 Improvement Program (2026-04)

Date: 2026-04-24

## Purpose

This document turns the external-repo study outcomes into a concrete implementation program for Ophiolite.

It is intentionally conflict-aware:

- it assumes the prior-art notes in `articles/` and `docs/architecture/` are already part of the baseline
- it avoids reopening chunking/tiling/cache decisions that were already captured
- it breaks work into lanes that can be delegated without colliding in the same files

## Scope Boundary

Phase 1 should improve:

- runtime inspectability and execution structure
- buffer/layout discipline in hot seismic paths
- operator catalog richness and internal registration shape
- numeric kernel quality where the evidence is already strong
- benchmark coverage needed to support those changes

Phase 1 should not try to do all of this at once:

- generic new chunking work
- another prefix-cache redesign
- a broad package-session/backend refactor across the top-level `src/storage.rs` stack
- a public plugin ABI
- a generic `run_operator(...)` surface

## Existing Prior Art To Respect

Treat these as already-decided unless new evidence directly contradicts them:

- [articles/README.md](/Users/sc/dev/ophiolite/articles/README.md)
- [articles/performance/PROCESSING_CACHE_ARCHITECTURE_AND_BENCHMARKING.md](/Users/sc/dev/ophiolite/articles/performance/PROCESSING_CACHE_ARCHITECTURE_AND_BENCHMARKING.md)
- [articles/performance/SECTION_TILING_AND_INTERACTIVE_SECTION_BROWSING_OPTIMIZATIONS.md](/Users/sc/dev/ophiolite/articles/performance/SECTION_TILING_AND_INTERACTIVE_SECTION_BROWSING_OPTIMIZATIONS.md)
- [articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md](/Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md)
- [articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md](/Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md)
- [articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md](/Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md)
- [articles/storage/TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md](/Users/sc/dev/ophiolite/articles/storage/TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md)
- [docs/architecture/ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md](/Users/sc/dev/ophiolite/docs/architecture/ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md)
- [docs/architecture/ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md](/Users/sc/dev/ophiolite/docs/architecture/ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md)
- [docs/architecture/seismic-execution-service-implementation-sketch.md](/Users/sc/dev/ophiolite/docs/architecture/seismic-execution-service-implementation-sketch.md)
- [docs/research/external-repo-study-prompts-2026-04.md](/Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md)

## Repo Touch Map

These are the main live overlap zones in the repo today.

### Lane A: Layout And Buffer Discipline

Primary files:

- `crates/ophiolite-seismic-runtime/src/store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/section_assembler.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs`
- `crates/ophiolite-seismic-runtime/src/storage/volume_store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tbvol.rs`
- `crates/ophiolite-seismic-runtime/src/ingest.rs`

Why this lane exists:

- the hot seismic paths still pass around raw payload vectors and ad hoc section/tile shapes
- this is the cleanest place to establish stable runtime-local buffer vocabulary before deeper kernel work

### Lane B: Runtime Orchestration And Stage Evidence

Primary files:

- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic-runtime/src/planner.rs`
- `crates/ophiolite-seismic-runtime/src/processing_runtime.rs`
- `crates/ophiolite-seismic-execution/src/lib.rs`

Why this lane exists:

- the repo already has planning and execution semantics, but stage policy and executed truth are not yet as explicit or inspectable as they should be

### Lane C: Catalog Metadata And Internal Registration

Primary files:

- `crates/ophiolite-seismic/src/contracts/operator_catalog.rs`
- `crates/ophiolite-seismic/src/contracts/processing.rs`
- `crates/ophiolite-operators/src/lib.rs`
- `src/project.rs`
- `apps/traceboost-demo/src/lib/processing-model.svelte.ts`
- `apps/traceboost-demo/src/lib/bridge.ts`
- `apps/traceboost-demo/src-tauri/src/lib.rs`

Why this lane exists:

- the typed family model is good, but metadata richness and registration consistency still lag the rest of the architecture

### Lane D: Numeric Kernel Quality

Primary files:

- `crates/ophiolite-seismic-runtime/src/compute.rs`
- `crates/ophiolite-seismic-runtime/src/gather_processing.rs`
- `crates/ophiolite-seismic-runtime/src/prestack_analysis.rs`
- `crates/ophiolite-seismic-runtime/benches/compute_storage.rs`
- `crates/ophiolite-seismic-runtime/src/bin/compute_storage_bench.rs`

Why this lane exists:

- the next gains are algorithm shape, scratch reuse, plan caching, and selective SIMD, not more generic parallelism

### Lane E: Compression And Codec Architecture

Primary files:

- `crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs`
- `crates/ophiolite-seismic-runtime/src/storage/volume_store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tbvol.rs`
- `crates/ophiolite-seismic-runtime/src/storage/mod.rs`
- `crates/ophiolite-seismic-runtime/src/identity.rs`
- `crates/ophiolite-seismic-runtime/src/bin/tbvolc_transcode.rs`

Why this lane exists:

- `tbvol` and `tbvolc` need clearer shared encoding/contracts before compressed-input execution work expands

### Lane F: Benchmark And Evidence

Primary files:

- `crates/ophiolite-seismic-runtime/benches/compute_storage.rs`
- `crates/ophiolite-seismic-runtime/benches/post_stack_neighborhood_kernels.rs`
- `crates/ophiolite-seismic-runtime/src/bin/compute_storage_bench.rs`
- `crates/ophiolite-seismic-runtime/src/bin/preview_incremental_bench.rs`
- `crates/ophiolite-seismic-runtime/src/bin/section_tile_bench.rs`
- `apps/traceboost-demo/src-tauri/src/processing_cache_bench.rs`
- `apps/traceboost-demo/src-tauri/src/preview_session_bench.rs`

Why this lane exists:

- Phase 1 should extend the existing evidence system, not invent a new one

## Concurrency Rules

Safe or mostly safe in parallel:

- Lane A with Lane C
- Lane B with Lane C
- Lane D with Lane C if Lane D avoids planner-hint/catalog changes
- Lane E with Lane C

Not safe to run as simultaneous code-change lanes:

- Lane A with Lane D
  - both will want to reshape `compute.rs` and related storage/runtime views
- Lane B with Lane A
  - both touch the planning/runtime boundary and store-driven execution assumptions
- Lane B with Lane E
  - both can pull on runtime/store ownership and identity/planning seams
- Lane D with Lane F
  - benchmark work should usually follow or bracket the kernel change it measures
- Lane E with Lane F
  - same reason, especially if compressed-input benches are added

Special scope warning:

- do not mix top-level package-session/backend refactors in `src/storage.rs` and `src/backend.rs` into the seismic runtime waves unless there is a strong reason
- that is a separate lane and should not be bundled into Phase 1 by default

## Recommended Execution Order

### Wave 0: Evidence Prep

Goal:

- tighten the benchmark matrix around the areas most likely to change next

Work:

- identify missing kernel-level benchmarks for spectral operators and prestack kernels
- identify missing compressed-input benchmark coverage
- define exact validation checks needed for kernel rewrites

Preferred scope:

- benchmark gaps and benchmark plan updates first
- avoid touching hot runtime code yet

### Wave 1: Layout And Catalog Foundations

Run these in parallel:

- Lane A: layout/buffer foundation
- Lane C: catalog metadata and internal registration cleanup

Why:

- they touch mostly separate files
- they both reduce future churn before runtime and kernel work

Expected outcomes:

- canonical runtime-local buffer vocabulary for trace/tile/occupancy handling
- richer catalog detail and less duplicated family registration plumbing

### Wave 2: Runtime Orchestration

Run after Wave 1 stabilizes:

- Lane B

Why:

- runtime/stage evidence should be built against a clearer layout/runtime baseline
- this lane overlaps too much with layout and execution assumptions to do concurrently

Expected outcomes:

- explicit runtime environment ownership
- clearer lowering from plan to executed stage configuration
- actual per-stage/per-partition evidence

### Wave 3: Numeric Kernel Improvements

Run after Wave 1 and after the relevant Wave 0 benchmarks exist:

- Lane D

Why:

- kernel work benefits from stable buffer/layout shapes
- the measurement harness should already exist before swapping algorithm forms

Expected outcomes:

- scratch reuse and FFT plan reuse
- cached filter responses
- compiled interpolation/velocity maps where justified
- selective SIMD only on clean loops

### Wave 4: Compression And Codec Architecture

Run after Wave 2 unless narrowed to metadata-only work:

- Lane E

Why:

- compressed-input execution and scheduling policy are easier to reason about once runtime ownership is cleaner

Expected outcomes:

- shared encoding metadata contract
- narrow codec-pipeline abstraction
- better `tbvol`/`tbvolc` reader and transcode boundaries

## Agent Task Batches

These prompts are written to avoid overlapping write sets.

### Batch 0A: Benchmark Gap Map

Mode:

- explorer or worker

Write scope:

- docs only in `docs/research/` or `articles/benchmarking/`

Do not touch:

- `crates/ophiolite-seismic-runtime/src/compute.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs`
- `crates/ophiolite-seismic-runtime/src/execution.rs`

Prompt:

```text
Work in /Users/sc/dev/ophiolite.

Goal:
Define the minimum benchmark and validation additions needed before Phase 1 code changes begin.

Read first:
- /Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md
- /Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md
- /Users/sc/dev/ophiolite/articles/benchmarking/INTERACTIVE_SECTION_BROWSING_HARNESS_PLAN.md
- /Users/sc/dev/ophiolite/articles/benchmarking/PREVIEW_INCREMENTAL_EXECUTION_BENCHMARK_PLAN.md
- /Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md

Inspect these files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/benches/compute_storage.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/benches/post_stack_neighborhood_kernels.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/bin/compute_storage_bench.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/bin/preview_incremental_bench.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/bin/section_tile_bench.rs
- /Users/sc/dev/ophiolite/apps/traceboost-demo/src-tauri/src/processing_cache_bench.rs
- /Users/sc/dev/ophiolite/apps/traceboost-demo/src-tauri/src/preview_session_bench.rs

Deliver:
1. A concise benchmark gap map for Phase 1 waves.
2. Exact new or updated benchmark targets to add before kernel/runtime/codec changes.
3. Numerical validation checks that should accompany spectral and prestack kernel work.
4. A short proposed patch plan that changes docs only.

Constraints:
- Do not propose a new benchmark framework.
- Do not edit runtime or storage code.
- Keep changes limited to docs/research or articles/benchmarking.
```

### Batch 1A: Layout And Buffer Foundation

Mode:

- worker

Write scope:

- `crates/ophiolite-seismic-runtime/src/store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/section_assembler.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs`
- `crates/ophiolite-seismic-runtime/src/storage/volume_store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tbvol.rs`
- `crates/ophiolite-seismic-runtime/src/storage/mod.rs`
- `crates/ophiolite-seismic-runtime/src/ingest.rs`

Do not touch:

- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic-runtime/src/planner.rs`
- `crates/ophiolite-seismic-runtime/src/processing_runtime.rs`
- `crates/ophiolite-seismic-runtime/src/compute.rs`

Prompt:

```text
Work in /Users/sc/dev/ophiolite.

You own the layout/buffer foundation lane. You are not alone in the repo. Do not revert others' changes. Stay inside your write scope and adjust to existing code instead of broadening the task.

Goal:
Introduce cleaner runtime-local layout/buffer vocabulary for seismic storage and section assembly without changing planner/execution behavior yet.

Read first:
- /Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md
- /Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md

Study these external repos for implementation style only:
- /Users/sc/dev/arrow-rs
- /Users/sc/dev/datafusion
- /Users/sc/dev/faer-rs

Primary target files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/store.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/section_assembler.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/volume_store.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tbvol.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/mod.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/ingest.rs

Goal detail:
- add a small set of owned/borrowed runtime-local buffer abstractions for trace-major sample data and occupancy
- reduce ad hoc shape handling in section/tile code
- keep on-disk layout decisions unchanged
- keep planner/runtime orchestration unchanged

Deliver:
1. A bounded implementation in the write scope.
2. Tests for the new buffer/layout abstractions.
3. A short note describing what follow-on kernel/runtime work this unblocks.

Constraints:
- Do not change execution/planner contracts.
- Do not redesign chunking.
- Do not edit compute kernels in compute.rs yet.
```

### Batch 1B: Catalog Metadata And Internal Registration

Mode:

- worker

Write scope:

- `crates/ophiolite-seismic/src/contracts/operator_catalog.rs`
- `crates/ophiolite-seismic/src/contracts/processing.rs`
- `crates/ophiolite-operators/src/lib.rs`
- `src/project.rs`
- `apps/traceboost-demo/src/lib/bridge.ts`
- `apps/traceboost-demo/src/lib/processing-model.svelte.ts`
- `apps/traceboost-demo/src-tauri/src/lib.rs`

Do not touch:

- `crates/ophiolite-seismic-runtime/src/compute.rs`
- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic-runtime/src/storage/*`

Prompt:

```text
Work in /Users/sc/dev/ophiolite.

You own the catalog metadata and internal registration lane. You are not alone in the repo. Do not revert others' changes. Stay inside your write scope.

Goal:
Strengthen operator catalog richness and reduce duplicated family registration/metadata plumbing without introducing a generic operator ABI.

Read first:
- /Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md
- /Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md
- /Users/sc/dev/ophiolite/docs/architecture/ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md

Study these external repos for comparison only:
- /Users/sc/dev/OpendTect
- /Users/sc/dev/madagascar-src

Primary target files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic/src/contracts/operator_catalog.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic/src/contracts/processing.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-operators/src/lib.rs
- /Users/sc/dev/ophiolite/src/project.rs
- /Users/sc/dev/ophiolite/apps/traceboost-demo/src/lib/bridge.ts
- /Users/sc/dev/ophiolite/apps/traceboost-demo/src/lib/processing-model.svelte.ts
- /Users/sc/dev/ophiolite/apps/traceboost-demo/src-tauri/src/lib.rs

Deliver:
1. A bounded implementation that adds useful metadata and reduces duplication.
2. Contract and consumer updates needed to keep Rust/app surfaces aligned.
3. Tests covering catalog shape and availability behavior.

Constraints:
- Do not add a generic run_operator surface.
- Do not expose runtime/storage internals as a plugin ABI.
- Keep family typing explicit.
```

### Batch 2A: Runtime Orchestration And Stage Evidence

Mode:

- worker

Write scope:

- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic-runtime/src/planner.rs`
- `crates/ophiolite-seismic-runtime/src/processing_runtime.rs`
- `crates/ophiolite-seismic-execution/src/lib.rs`
- related tests in the same crates

Do not touch:

- `crates/ophiolite-seismic-runtime/src/store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/*`
- `crates/ophiolite-seismic-runtime/src/compute.rs`
- `src/storage.rs`
- `src/backend.rs`

Prompt:

```text
Work in /Users/sc/dev/ophiolite.

You own the runtime orchestration lane. You are not alone in the repo. Do not revert others' changes. Stay inside your write scope.

Goal:
Make runtime resource ownership, stage lowering, and executed-truth evidence more explicit without broadening into package-session/backend refactors.

Read first:
- /Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md
- /Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md
- /Users/sc/dev/ophiolite/docs/architecture/ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md
- /Users/sc/dev/ophiolite/docs/architecture/seismic-execution-service-implementation-sketch.md

Study these external repos for comparison only:
- /Users/sc/dev/rayon
- /Users/sc/dev/oneTBB
- /Users/sc/dev/datafusion
- /Users/sc/dev/TileDB

Primary target files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/execution.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/planner.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_runtime.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-execution/src/lib.rs

Deliver:
1. A bounded implementation for a clearer runtime environment / stage evidence surface.
2. Tests proving the new execution evidence and lowering behavior.
3. A short note on what remains deferred for later waves.

Constraints:
- Do not reopen generic chunking work.
- Do not refactor top-level package storage/session code in src/storage.rs or src/backend.rs.
- Do not simultaneously redesign compute kernels.
```

### Batch 3A: Numeric Kernel Improvements

Mode:

- worker

Write scope:

- `crates/ophiolite-seismic-runtime/src/compute.rs`
- `crates/ophiolite-seismic-runtime/src/gather_processing.rs`
- `crates/ophiolite-seismic-runtime/src/prestack_analysis.rs`
- `crates/ophiolite-seismic-runtime/benches/compute_storage.rs`
- `crates/ophiolite-seismic-runtime/src/bin/compute_storage_bench.rs`
- related tests in the same crate

Do not touch:

- `crates/ophiolite-seismic-runtime/src/store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/*`
- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic/src/contracts/operator_catalog.rs`

Prompt:

```text
Work in /Users/sc/dev/ophiolite.

You own the numeric kernel lane. You are not alone in the repo. Do not revert others' changes. Stay inside your write scope.

Goal:
Improve hot numeric kernels in a bounded way using stronger scratch reuse, plan caching, precomputed response data, and selective SIMD only where the code shape supports it.

Read first:
- /Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md
- /Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md
- /Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md
- /Users/sc/dev/ophiolite/articles/performance/POST_STACK_DIP_PREVIEW_BENCHMARKING_AND_LAG_SEARCH_OPTIMIZATION.md

Study these external repos for implementation style only:
- /Users/sc/dev/RustFFT
- /Users/sc/dev/realfft
- /Users/sc/dev/fftw3
- /Users/sc/dev/faer-rs
- /Users/sc/dev/madagascar-src

Primary target files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/gather_processing.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/prestack_analysis.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/benches/compute_storage.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/bin/compute_storage_bench.rs

Deliver:
1. A bounded kernel improvement patch.
2. Updated benchmarks or validation where needed to justify the change.
3. A short note separating spectral improvements from gather/prestack improvements.

Constraints:
- Do not reopen prefix-cache work unless there is a specific missed mechanism.
- Do not redesign planner or storage abstractions.
- Prefer staying on current library choices unless the measurement strongly justifies more specialization.
```

### Batch 4A: Compression And Codec Architecture

Mode:

- worker

Write scope:

- `crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs`
- `crates/ophiolite-seismic-runtime/src/storage/volume_store.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tbvol.rs`
- `crates/ophiolite-seismic-runtime/src/storage/mod.rs`
- `crates/ophiolite-seismic-runtime/src/identity.rs`
- `crates/ophiolite-seismic-runtime/src/bin/tbvolc_transcode.rs`
- related tests in the same crate

Do not touch:

- `crates/ophiolite-seismic-runtime/src/execution.rs`
- `crates/ophiolite-seismic-runtime/src/planner.rs`
- `src/storage.rs`
- `src/backend.rs`

Prompt:

```text
Work in /Users/sc/dev/ophiolite.

You own the compression/codec lane. You are not alone in the repo. Do not revert others' changes. Stay inside your write scope.

Goal:
Clarify and strengthen the tbvol/tbvolc architecture without broadening into a cross-cutting runtime or package-session refactor.

Read first:
- /Users/sc/dev/ophiolite/docs/research/external-repo-study-prompts-2026-04.md
- /Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md
- /Users/sc/dev/ophiolite/articles/storage/TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md

Study these external repos for comparison only:
- /Users/sc/dev/TileDB
- /Users/sc/dev/c-blosc2
- /Users/sc/dev/zarrs

Primary target files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/volume_store.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tbvol.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/mod.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/identity.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/bin/tbvolc_transcode.rs

Deliver:
1. A bounded implementation for clearer encoding metadata and codec-pipeline structure.
2. Tests covering metadata compatibility and tbvol/tbvolc behavior.
3. A short note on what compressed-input execution work is still deferred.

Constraints:
- Keep tbvol as the hot compute substrate.
- Do not drag src/storage.rs or src/backend.rs into this wave.
- Do not redesign generic runtime scheduling here.
```

## Suggested Initial Run

If the goal is to start now with the lowest collision risk, use this order:

1. Batch 0A
2. Batch 1A and Batch 1B in parallel
3. Batch 2A
4. Batch 3A
5. Batch 4A

If you want one more conservative variant:

1. Batch 0A
2. Batch 1A
3. Batch 1B
4. Batch 2A
5. Batch 3A
6. Batch 4A

The first variant is faster. The second variant reduces merge friction further.
