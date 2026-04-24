# External Repo Study Prompts For Ophiolite

Date: 2026-04-24

Related rollout document:

- [docs/research/phase-1-improvement-program-2026-04.md](/Users/sc/dev/ophiolite/docs/research/phase-1-improvement-program-2026-04.md)

## Clone Status

Repos available locally in `/Users/sc/dev`:

- `/Users/sc/dev/rayon`
- `/Users/sc/dev/RustFFT`
- `/Users/sc/dev/realfft`
- `/Users/sc/dev/faer-rs`
- `/Users/sc/dev/arrow-rs`
- `/Users/sc/dev/datafusion`
- `/Users/sc/dev/zarrs`
- `/Users/sc/dev/oneTBB`
- `/Users/sc/dev/TileDB`
- `/Users/sc/dev/c-blosc2`
- `/Users/sc/dev/OpendTect`
- `/Users/sc/dev/madagascar-src`
- `/Users/sc/dev/fftw3`

Notes:

- `arrow-rs` and `OpendTect` were already present locally and were reused.
- `fftw3` is useful as a kernel-design reference, but the FFTW repo itself is generator-oriented; the project README says most users should prefer official release tarballs over the raw git checkout.

## Upstream URLs

- Rayon: https://github.com/rayon-rs/rayon
- RustFFT: https://github.com/ejmahler/RustFFT
- RealFFT: https://github.com/HEnquist/realfft
- faer: https://github.com/sarah-quinones/faer-rs
- Arrow Rust: https://github.com/apache/arrow-rs
- DataFusion: https://github.com/apache/datafusion
- zarrs: https://github.com/zarrs/zarrs
- oneTBB: https://github.com/uxlfoundation/oneTBB
- TileDB: https://github.com/TileDB-Inc/TileDB
- C-Blosc2: https://github.com/Blosc/c-blosc2
- OpendTect: https://github.com/OpendTect/OpendTect
- Madagascar: https://github.com/ahay/src
- FFTW: https://github.com/FFTW/fftw3
- FFTW project site: https://fftw.org/

## Ophiolite Baseline

Relevant local Ophiolite entry points:

- `crates/ophiolite-seismic/src/contracts/processing.rs`
- `crates/ophiolite-seismic/src/contracts/operator_catalog.rs`
- `crates/ophiolite-seismic-runtime/src/compute.rs`
- `crates/ophiolite-seismic-runtime/src/gather_processing.rs`
- `crates/ophiolite-seismic-runtime/src/post_stack_neighborhood.rs`
- `crates/ophiolite-seismic-runtime/src/prestack_analysis.rs`
- `crates/ophiolite-seismic-runtime/src/planner.rs`
- `crates/ophiolite-seismic-runtime/src/processing_runtime.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs`
- `crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs`
- `crates/ophiolite-seismic-runtime/src/storage/zarr.rs`

Current Ophiolite strengths that should be preserved:

- Typed seismic operator contracts and explicit operator families.
- Planner hints per operator family and adaptive chunk planning.
- Checkpoint/reuse/lineage model for materialized outputs.
- Tile-based storage geometry and preview prefix cache.
- Real seismic kernels already implemented in Rust, not just wrappers.
- Existing benchmark harnesses in `crates/ophiolite-seismic-runtime/benches` and `articles/benchmarking`.

## Existing Local Prior Art And Overlap Boundaries

Agents using the prompts below should treat these documents as already-captured work, not as new territory to rediscover:

- `articles/performance/PROCESSING_CACHE_ARCHITECTURE_AND_BENCHMARKING.md`
- `articles/performance/SECTION_TILING_AND_INTERACTIVE_SECTION_BROWSING_OPTIMIZATIONS.md`
- `articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md`
- `articles/performance/POST_STACK_DIP_PREVIEW_BENCHMARKING_AND_LAG_SEARCH_OPTIMIZATION.md`
- `articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md`
- `articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md`
- `articles/storage/TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md`
- `articles/benchmarking/INTERACTIVE_SECTION_BROWSING_HARNESS_PLAN.md`
- `articles/benchmarking/PREVIEW_INCREMENTAL_EXECUTION_BENCHMARK_PLAN.md`
- `docs/architecture/ADR-0010-typed-compute-and-derived-assets.md`
- `docs/architecture/ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md`
- `docs/architecture/ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md`
- `docs/architecture/ADR-0032-processing-authority-and-thin-client-migration.md`
- `docs/architecture/ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`
- `docs/architecture/processing-canonical-integration-plan-2026-04.md`
- `docs/architecture/processing-lineage-cache-compatibility-policy.md`
- `docs/architecture/seismic-execution-service-implementation-sketch.md`

Use these overlap rules:

- Do not restate already-made decisions unless the external repos provide a concrete reason to revisit them.
- Do not propose "more chunking work" or "more prefix cache work" as generic next steps if the existing notes already evaluated those paths.
- Prefer identifying gaps, contradictions, or missing abstractions around the current design.
- When a prior note already measured something, build on it by naming what remains unmeasured rather than redefining the benchmark from scratch.
- If a previous document already contains the likely answer, say so and move on to adjacent unresolved questions.

## Topic 1: Parallel Scheduling And Execution Planning

### Relevant repos

- `/Users/sc/dev/rayon`
- `/Users/sc/dev/oneTBB`
- `/Users/sc/dev/datafusion`
- `/Users/sc/dev/TileDB`

### Why this topic matters

Ophiolite already parallelizes trace-local work and already computes chunk plans, but the next improvement frontier is not "add threads." It is making execution policy, resource policy, and task decomposition more explicit and more inspectable.

### Current Ophiolite state

- Trace-local execution uses a Rayon pool and `par_chunks_mut` in `crates/ophiolite-seismic-runtime/src/compute.rs`.
- Adaptive partition targeting and chunk-plan recommendation live in `crates/ophiolite-seismic-runtime/src/planner.rs`.
- Checkpoint stage construction and reuse logic live in `crates/ophiolite-seismic-runtime/src/processing_runtime.rs`.
- Planner hints are explicit in `crates/ophiolite-seismic/src/contracts/operator_catalog.rs`.

### What the external repos do better

- `rayon` gives a mature work-stealing runtime with clearer scheduler internals and separation between user-facing APIs and core runtime.
- `oneTBB` is more explicit about arenas, task dispatch, worker management, and logical parallelism rather than "threads as the model."
- `datafusion` has a first-class execution-plan abstraction plus runtime resource objects for memory, disk, cache, and object stores.
- `TileDB` exposes planning and execution metadata as first-class concepts instead of hiding all resource choices inside kernels.

### Comparison against Ophiolite

Ophiolite is already ahead on domain-aware planning because it knows seismic operator semantics, checkpoint safety, and chunkability. It is behind on generic runtime architecture: it does not yet have as strong a separation between plan node, runtime environment, scheduler policy, and storage policy as DataFusion or TileDB. It also lacks the kind of pluggable threading/backend story that appears in oneTBB and other HPC systems.

### Prompt

```text
Study Ophiolite's execution planning and scheduling model and compare it against Rayon, oneTBB, DataFusion, and TileDB.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- Rayon: /Users/sc/dev/rayon
- oneTBB: /Users/sc/dev/oneTBB
- DataFusion: /Users/sc/dev/datafusion
- TileDB: /Users/sc/dev/TileDB

Start with these Ophiolite files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/planner.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_runtime.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic/src/contracts/operator_catalog.rs

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md
- /Users/sc/dev/ophiolite/docs/architecture/ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md
- /Users/sc/dev/ophiolite/docs/architecture/seismic-execution-service-implementation-sketch.md

Start with these external files:
- /Users/sc/dev/rayon/rayon-core/src/registry.rs
- /Users/sc/dev/oneTBB/src/tbb/task_dispatcher.cpp
- /Users/sc/dev/oneTBB/src/tbb/arena.h
- /Users/sc/dev/datafusion/datafusion/physical-plan/src/execution_plan.rs
- /Users/sc/dev/datafusion/datafusion/execution/src/runtime_env.rs
- /Users/sc/dev/TileDB/tiledb/sm/query_plan/query_plan.h

Goal:
Find architecture improvements Ophiolite can make around execution plans, runtime resource ownership, scheduling policy, and inspectability without losing its seismic-specific planner semantics.

Overlap guardrails:
- Assume chunk planning and bounded local job service work already exist.
- Do not spend effort re-proposing generic partitioned batch execution.
- Focus on what the current planner/runtime split still lacks relative to the external repos.

Deliver:
1. A concise comparison of the execution model in each repo.
2. A ranked list of 5-8 concrete changes Ophiolite should consider.
3. For each proposed change, name the exact Ophiolite files that would need to change.
4. Separate low-risk incremental improvements from larger architectural refactors.
5. Call out anything Ophiolite already does better and should keep.

Do not give generic concurrency advice. Ground every recommendation in specific source files and explain why it fits seismic workloads rather than analytics or generic task graphs.
```

## Topic 2: Spectral Kernels, Numeric Methods, And SIMD Strategy

### Relevant repos

- `/Users/sc/dev/RustFFT`
- `/Users/sc/dev/realfft`
- `/Users/sc/dev/fftw3`
- `/Users/sc/dev/faer-rs`
- `/Users/sc/dev/madagascar-src`

### Why this topic matters

Ophiolite already has meaningful seismic math, but its kernels are still mostly expressed as clean Rust loops plus library calls. The biggest likely gains here are kernel selection, batched execution, SIMD specialization, and benchmark-guided choice of algorithm shape.

### Current Ophiolite state

- Trace-local spectral operators are dispatched in `crates/ophiolite-seismic-runtime/src/compute.rs`.
- Gather-native NMO/stretch-mute/offset-mute kernels live in `crates/ophiolite-seismic-runtime/src/gather_processing.rs`.
- Velocity scan and semblance logic live in `crates/ophiolite-seismic-runtime/src/prestack_analysis.rs`.

### What the external repos do better

- `RustFFT` has planner-driven algorithm selection and CPU-feature-aware SIMD paths.
- `realfft` is disciplined about real-signal layouts and scratch/buffer management.
- `fftw3` is still the reference for planning, generated kernels, and runtime self-optimization.
- `faer` is strong on explicit SIMD context and microkernel-style numeric structure.
- `Madagascar` shows decades of geophysical operator packaging and includes filters, Hilbert, FFT, conjugate gradient, GMRES, and dot-product test tooling.

### Comparison against Ophiolite

Ophiolite is already better integrated with its seismic pipeline than these libraries, but it is weaker on CPU-specific specialization and on evidence that a chosen kernel form is the best one. The main risk is not Rust. The main risk is stopping at "correct and parallel" instead of moving to "correct, parallel, batched, and hardware-aware."

### Prompt

```text
Study Ophiolite's numeric kernels and compare them against RustFFT, RealFFT, FFTW, faer, and Madagascar to identify improvements in algorithm selection, SIMD use, batching, and numerical validation.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- RustFFT: /Users/sc/dev/RustFFT
- RealFFT: /Users/sc/dev/realfft
- FFTW: /Users/sc/dev/fftw3
- faer: /Users/sc/dev/faer-rs
- Madagascar: /Users/sc/dev/madagascar-src

Start with these Ophiolite files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/gather_processing.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/prestack_analysis.rs

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/articles/performance/POST_STACK_DIP_PREVIEW_BENCHMARKING_AND_LAG_SEARCH_OPTIMIZATION.md
- /Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md

Start with these external files:
- /Users/sc/dev/RustFFT/src/plan.rs
- /Users/sc/dev/realfft/src/lib.rs
- /Users/sc/dev/fftw3/README
- /Users/sc/dev/faer-rs/faer/src/linalg/matmul/mod.rs
- /Users/sc/dev/faer-rs/faer/src/utils/simd.rs
- /Users/sc/dev/madagascar-src/api/c/freqfilt.c
- /Users/sc/dev/madagascar-src/api/c/hilbert.c
- /Users/sc/dev/madagascar-src/api/c/conjgrad.c
- /Users/sc/dev/madagascar-src/system/main/dottest.c

Goal:
Produce a realistic plan for improving Ophiolite's hot numeric paths, especially spectral attributes and prestack kernels, without turning the codebase into a generic HPC library.

Overlap guardrails:
- Do not spend effort on more prefix-cache tuning for dip-like operators unless the external repos reveal a concrete missed mechanism.
- Treat existing trace-local batch scheduling work as prior art; focus on kernel quality, algorithm choice, SIMD, and validation.

Deliver:
1. A map of Ophiolite kernels that are most likely CPU-bound.
2. Concrete opportunities for SIMD specialization, batched transforms, scratch reuse, and planner-level kernel selection.
3. A shortlist of benchmarks Ophiolite should add or tighten before changing kernels.
4. Specific advice on whether to stay on current library usage or introduce deeper optional specializations.
5. Clear separation between ideas that improve trace-local spectral operators and ideas that improve gather/semblance-style kernels.

Ground everything in source files. Avoid vague statements like "use SIMD more." Explain exactly where it would fit and what measurement would justify it.
```

## Topic 3: Memory Layout, Buffer Discipline, And Vectorized Processing

### Relevant repos

- `/Users/sc/dev/faer-rs`
- `/Users/sc/dev/arrow-rs`
- `/Users/sc/dev/datafusion`

### Why this topic matters

Many performance ceilings are really layout ceilings. Ophiolite already uses flat buffers and tile geometry, which is good. The question is whether more explicit buffer/view types, alignment rules, and vector-friendly layouts would make kernels simpler and faster.

### Current Ophiolite state

- `TileGeometry` in `crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs` defines tile byte sizes and offsets.
- Kernels often operate over `Vec<f32>` plus optional occupancy masks.
- Section assembly and trace-local execution are buffer-oriented, but the buffer contracts are still fairly ad hoc compared with Arrow or faer.

### What the external repos do better

- `faer` is strong on explicit contiguous views, SIMD contexts, and microkernel-friendly layout.
- `arrow-rs` is strong on buffer alignment, fixed-width vs variable-width buffer contracts, and low-level data ownership discipline.
- `datafusion` benefits from Arrow's columnar layout and expresses vectorized execution expectations more explicitly.

### Comparison against Ophiolite

Ophiolite is simpler and easier to follow than those systems, which is a real advantage. The gap is not that Ophiolite uses `Vec<f32>`. The gap is that the repo does not yet have a small set of canonical, performance-aware buffer/view abstractions that make alignment, mutability, occupancy representation, and batch shape explicit.

### Prompt

```text
Study Ophiolite's memory layout and buffer abstractions and compare them against faer, Arrow Rust, and DataFusion. Focus on opportunities to make hot-path layout rules more explicit without overcomplicating the domain code.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- faer: /Users/sc/dev/faer-rs
- arrow-rs: /Users/sc/dev/arrow-rs
- DataFusion: /Users/sc/dev/datafusion

Start with these Ophiolite files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/store.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/volume_store.rs

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md
- /Users/sc/dev/ophiolite/articles/performance/SECTION_TILING_AND_INTERACTIVE_SECTION_BROWSING_OPTIMIZATIONS.md

Start with these external files:
- /Users/sc/dev/faer-rs/faer/src/linalg/matmul/mod.rs
- /Users/sc/dev/faer-rs/faer/src/utils/simd.rs
- /Users/sc/dev/arrow-rs/arrow-buffer/src/buffer/ops.rs
- /Users/sc/dev/arrow-rs/arrow-data/src/data.rs
- /Users/sc/dev/datafusion/datafusion/physical-plan/src/execution_plan.rs

Goal:
Recommend a small, coherent buffer/layout strategy for Ophiolite that improves cache behavior and future SIMD/vectorization work.

Overlap guardrails:
- Do not re-argue for tile-based storage at a high level; assume that decision stands.
- Focus on internal buffer/view discipline and vectorization readiness beyond the current tile design.

Deliver:
1. A diagnosis of current Ophiolite buffer/layout patterns.
2. A proposal for 3-6 canonical buffer/view abstractions Ophiolite should standardize on.
3. A judgment on whether occupancy masks should remain byte-oriented or move to a denser representation in some paths.
4. An assessment of where multi-trace vectorization is plausible and where trace-local scalar loops are still the right call.
5. A migration plan that does not require rewriting the whole runtime at once.

Keep the recommendations specific to seismic trace/tile/gather workloads. Do not propose importing Arrow-style architecture wholesale unless you can justify it.
```

## Topic 4: Chunked IO, Storage Planning, And Cache Hierarchy

### Relevant repos

- `/Users/sc/dev/TileDB`
- `/Users/sc/dev/zarrs`
- `/Users/sc/dev/datafusion`

### Why this topic matters

Ophiolite already has tile geometry and adaptive chunk planning. The next architectural gains are likely in making IO policy, cache policy, and storage backends more explicit and more composable.

### Current Ophiolite state

- Tile geometry and byte accounting live in `crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs`.
- Trace-local materialization options and partitioning logic live in `crates/ophiolite-seismic-runtime/src/compute.rs` and `crates/ophiolite-seismic-runtime/src/planner.rs`.
- Processing lineage and chunk-grid metadata live in `crates/ophiolite-seismic-runtime/src/processing_runtime.rs`.
- Zarr import/export and codec mapping already exist in `crates/ophiolite-seismic-runtime/src/storage/zarr.rs` and `crates/ophiolite-seismic-runtime/src/zarr_export.rs`.

### What the external repos do better

- `TileDB` makes query plans, storage-manager concerns, filter pipelines, read-ahead, and cache behavior much more explicit.
- `zarrs` has a clearer codec ecosystem and concurrency-aware chunk pipeline surface.
- `datafusion` has a better factored runtime story for object stores, cache managers, and memory/disk management.

### Comparison against Ophiolite

Ophiolite is already more domain-specific than those repos, and its checkpoint/prefix semantics are strong. It is weaker on general storage runtime factoring. Today, storage planning, IO scheduling, and cache policy are present, but not as cleanly isolated into named subsystems with stable contracts.

### Prompt

```text
Study Ophiolite's chunked storage and cache architecture and compare it against TileDB, zarrs, and DataFusion. Focus on how to make IO policy, cache policy, and chunk planning cleaner and more extensible.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- TileDB: /Users/sc/dev/TileDB
- zarrs: /Users/sc/dev/zarrs
- DataFusion: /Users/sc/dev/datafusion

Start with these Ophiolite files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tile_geometry.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/planner.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_runtime.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/zarr.rs

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md
- /Users/sc/dev/ophiolite/articles/performance/SECTION_TILING_AND_INTERACTIVE_SECTION_BROWSING_OPTIMIZATIONS.md
- /Users/sc/dev/ophiolite/docs/architecture/processing-lineage-cache-compatibility-policy.md

Start with these external files:
- /Users/sc/dev/TileDB/tiledb/sm/query_plan/query_plan.h
- /Users/sc/dev/zarrs/zarrs_codec/src/recommended_concurrency.rs
- /Users/sc/dev/datafusion/datafusion/execution/src/runtime_env.rs
- /Users/sc/dev/datafusion/datafusion/datasource/src/file_stream/scan_state.rs

Goal:
Propose a cleaner storage/runtime architecture for Ophiolite that keeps the seismic semantics but better separates chunk planning, cache management, IO scheduling, and backend concerns.

Overlap guardrails:
- Assume the repo already has tile geometry, adaptive chunk planning, preview prefix caching, and Zarr integration.
- Do not propose generic "more chunking" work that duplicates the existing section-tiling or storage-benchmark notes.
- Focus on subsystem boundaries, cache hierarchy, runtime resource ownership, and backend factoring.

Deliver:
1. A comparison of Ophiolite's current storage/runtime factoring versus the external repos.
2. A recommended target architecture with named subsystems.
3. A list of concrete cache layers Ophiolite should or should not add.
4. A view on whether preview prefix caching should remain special-case logic or become part of a general cache/reuse framework.
5. A phased implementation plan with exact file targets.

Do not recommend replacing tbvol with TileDB or Zarr wholesale. Work from the assumption that Ophiolite keeps domain-owned storage semantics and wants to improve architecture around them.
```

## Topic 5: Compression And Codec Pipeline Design

### Relevant repos

- `/Users/sc/dev/c-blosc2`
- `/Users/sc/dev/zarrs`
- `/Users/sc/dev/TileDB`

### Why this topic matters

Compression is not just a storage detail. It interacts with tile size, preview latency, partial reads, CPU budget, cache behavior, and export interoperability.

### Current Ophiolite state

- Ophiolite already exposes storage compression kinds and Zarr codec mapping in `crates/ophiolite-seismic-runtime/src/storage/zarr.rs`.
- `tbvolc` exists as a compressed/archive sibling format in `crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs`.
- Runtime exports currently surface `tbvol`, `tbvolc`, and Zarr pathways from `crates/ophiolite-seismic-runtime/src/lib.rs`.

### What the external repos do better

- `c-blosc2` treats filter pipelines, block structure, threading, and chunk format as first-class design objects.
- `zarrs` makes codec composition a formal part of array storage.
- `TileDB` has a more formal filter-pipeline story attached to its storage engine.

### Comparison against Ophiolite

Ophiolite already has a practical compression story, but it is less explicit as architecture. The likely next step is not "add more codecs." It is making codec/filter decisions visible in a stable contract and clarifying how chunk shape, codec choice, preview path, and archive path interact.

### Prompt

```text
Study Ophiolite's compression and codec architecture and compare it against c-blosc2, zarrs, and TileDB. Focus on how codec/filter design should influence Ophiolite's storage architecture and preview/materialization behavior.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- c-blosc2: /Users/sc/dev/c-blosc2
- zarrs: /Users/sc/dev/zarrs
- TileDB: /Users/sc/dev/TileDB

Start with these Ophiolite files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/storage/zarr.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/metadata.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/lib.rs

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/articles/storage/TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md
- /Users/sc/dev/ophiolite/articles/storage/SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md

Start with these external files:
- /Users/sc/dev/c-blosc2/README.rst
- /Users/sc/dev/c-blosc2/README_THREADED.rst
- /Users/sc/dev/c-blosc2/README_CHUNK_FORMAT.rst
- /Users/sc/dev/zarrs/zarrs_codec/src/lib.rs
- /Users/sc/dev/zarrs/zarrs_codec/src/recommended_concurrency.rs
- /Users/sc/dev/TileDB/tiledb/sm/serialization/array_schema.cc

Goal:
Recommend how Ophiolite should evolve its codec/filter model so compression choices become better aligned with storage format design, partial reads, and compute behavior.

Overlap guardrails:
- Treat existing `tbvolc` and current Zarr codec mapping as prior art.
- Focus on codec/filter architecture and measurement gaps, not on redoing the basic "should we compress seismic storage" debate.

Deliver:
1. A clear assessment of Ophiolite's current codec architecture.
2. Specific recommendations for what should be first-class metadata versus implementation detail.
3. Advice on chunk-size and codec co-design for preview, batch processing, and archival use cases.
4. A proposal for whether Ophiolite needs an explicit internal codec pipeline abstraction.
5. A benchmark plan for evaluating codec choices on realistic seismic workloads.

Keep the answer grounded in Ophiolite's current code rather than generic compression theory.
```

## Topic 6: Operator Architecture, Extensibility, And Pluginability

### Relevant repos

- `/Users/sc/dev/OpendTect`
- `/Users/sc/dev/madagascar-src`

### Why this topic matters

This is where Ophiolite can get sharper about what kind of platform it wants to be. Today it has a strong typed internal operator model. The next question is how much of that should become a broader extension system, catalog discipline, or external authoring surface.

### Current Ophiolite state

- Operator definitions and dependency profiles live in `crates/ophiolite-seismic/src/contracts/processing.rs`.
- Operator catalog entries and planner hints live in `crates/ophiolite-seismic/src/contracts/operator_catalog.rs`.
- Runtime families are split across trace-local, gather, post-stack-neighborhood, and analysis modules.

### What the external repos do better

- `OpendTect` has a mature provider/plugin model for attributes and a long-lived operator ecosystem.
- `Madagascar` is stronger as a research environment and as a composition layer for many small operator programs and reproducible flows.

### Comparison against Ophiolite

Ophiolite is stronger on typed contracts and SDK coherence than Madagascar, and cleaner than OpendTect in several places. It is weaker on external extensibility and on the ecosystem conventions around operator registration, third-party authoring, and research-style composition. The opportunity is to add extensibility without giving up type safety or collapsing into a loose plugin sprawl.

### Prompt

```text
Study Ophiolite's operator architecture and compare it against OpendTect's provider/plugin model and Madagascar's operator/program composition model. The goal is to recommend how Ophiolite should evolve its operator system without losing typed contracts and platform coherence.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- OpendTect: /Users/sc/dev/OpendTect
- Madagascar: /Users/sc/dev/madagascar-src

Start with these Ophiolite files:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic/src/contracts/processing.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic/src/contracts/operator_catalog.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/compute.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/gather_processing.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/post_stack_neighborhood.rs
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/src/prestack_analysis.rs

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/docs/architecture/ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md
- /Users/sc/dev/ophiolite/docs/architecture/ADR-0010-typed-compute-and-derived-assets.md

Start with these external files:
- /Users/sc/dev/OpendTect/plugins/ExpAttribs/similaritybyaw.h
- /Users/sc/dev/OpendTect/plugins/ExpAttribs/similaritybyaw.cc
- /Users/sc/dev/OpendTect/plugins/ExpAttribs/integratedtrace.h
- /Users/sc/dev/madagascar-src/README.md
- /Users/sc/dev/madagascar-src/api/c/SConstruct
- /Users/sc/dev/madagascar-src/framework/rsf/flow.py

Goal:
Recommend an extensibility direction for Ophiolite's operator system that balances typed contracts, runtime safety, ecosystem growth, and domain clarity.

Overlap guardrails:
- Assume the existing typed catalog/family model is intentional and should not be replaced casually.
- Focus on unresolved extension seams and future-proofing, not on rebuilding the current catalog from scratch.

Deliver:
1. A comparison between Ophiolite families, OpendTect providers, and Madagascar program/flow composition.
2. A judgment on whether Ophiolite should add a richer plugin model, a richer catalog model, or both.
3. A proposal for what extension seams should remain internal versus public.
4. A list of concrete contract or catalog changes that would make the operator system more future-proof.
5. Explicit warnings about design moves that would dilute Ophiolite's current strengths.

Avoid generic plugin-system advice. Make the answer specific to seismic operators and to Ophiolite's current contract/runtime split.
```

## Topic 7: Benchmarking, Validation, And Evidence Discipline

### Relevant repos

- `/Users/sc/dev/RustFFT`
- `/Users/sc/dev/datafusion`
- `/Users/sc/dev/c-blosc2`
- `/Users/sc/dev/madagascar-src`

### Why this topic matters

Ophiolite already has benchmark infrastructure. The opportunity is to make architecture and kernel changes more tightly evidence-backed and easier to compare over time.

### Current Ophiolite state

- Runtime benches exist in `crates/ophiolite-seismic-runtime/benches`.
- Benchmark result artifacts already exist in `articles/benchmarking/results`.
- There is real momentum here; this topic is about tightening discipline, not inventing it from scratch.

### What the external repos do better

- `RustFFT` and `c-blosc2` are highly benchmark-driven around kernel behavior.
- `DataFusion` uses benchmarks around physical-plan behaviors and spill/vectorization paths.
- `Madagascar` has a strong reproducibility and test-driven culture around numerical experiments.

### Comparison against Ophiolite

Ophiolite already takes benchmarking more seriously than many domain repos. The gap is that some architecture decisions still look repo-local rather than being tied to stable benchmark matrices that cover kernel choice, chunk policy, codec choice, and realistic datasets together.

### Prompt

```text
Study Ophiolite's benchmark and validation discipline and compare it against RustFFT, DataFusion, c-blosc2, and Madagascar. The goal is to improve how Ophiolite justifies architecture and performance changes.

Workspace roots:
- Ophiolite: /Users/sc/dev/ophiolite
- RustFFT: /Users/sc/dev/RustFFT
- DataFusion: /Users/sc/dev/datafusion
- c-blosc2: /Users/sc/dev/c-blosc2
- Madagascar: /Users/sc/dev/madagascar-src

Start with these Ophiolite paths:
- /Users/sc/dev/ophiolite/crates/ophiolite-seismic-runtime/benches
- /Users/sc/dev/ophiolite/articles/benchmarking
- /Users/sc/dev/ophiolite/traceboost/io/benches

Read these prior-art docs before proposing changes:
- /Users/sc/dev/ophiolite/articles/benchmarking/INTERACTIVE_SECTION_BROWSING_HARNESS_PLAN.md
- /Users/sc/dev/ophiolite/articles/benchmarking/PREVIEW_INCREMENTAL_EXECUTION_BENCHMARK_PLAN.md
- /Users/sc/dev/ophiolite/articles/performance/TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md
- /Users/sc/dev/ophiolite/articles/performance/PROCESSING_CACHE_ARCHITECTURE_AND_BENCHMARKING.md

Start with these external paths:
- /Users/sc/dev/RustFFT/benches
- /Users/sc/dev/datafusion/datafusion/physical-plan/benches
- /Users/sc/dev/c-blosc2/bench
- /Users/sc/dev/madagascar-src/README.md
- /Users/sc/dev/madagascar-src/system/main/dottest.c

Goal:
Recommend a benchmark and validation strategy that better connects kernel changes, storage changes, and architectural changes to trustworthy evidence.

Overlap guardrails:
- Treat the existing benchmark notes and result artifacts as the baseline.
- Do not propose a new benchmark system from scratch unless a concrete deficiency requires it.
- Focus on coverage gaps, evidence quality, and sequencing of benchmark work relative to likely refactors.

Deliver:
1. A critique of Ophiolite's current benchmark coverage.
2. A proposed benchmark matrix covering kernels, storage layouts, chunk plans, codec choices, and dataset scales.
3. Guidance on what should be microbenchmarks versus end-to-end authoritative runs.
4. Advice on how to encode reproducibility and numerical validation checks alongside performance runs.
5. A prioritized list of benchmark gaps that should be filled first.

Assume Ophiolite wants credible engineering evidence, not marketing benchmarks.
```

## Recommended Delegation Order

If only a few agents are available, run these topics in this order:

1. Parallel Scheduling And Execution Planning
2. Chunked IO, Storage Planning, And Cache Hierarchy
3. Spectral Kernels, Numeric Methods, And SIMD Strategy
4. Compression And Codec Pipeline Design
5. Memory Layout, Buffer Discipline, And Vectorized Processing
6. Operator Architecture, Extensibility, And Pluginability
7. Benchmarking, Validation, And Evidence Discipline

Reasoning:

- Topics 1 and 4 shape the runtime architecture and will constrain many later changes.
- Topic 2 identifies the hottest compute wins.
- Topic 5 depends partly on storage design choices.
- Topic 3 is important but should be informed by conclusions from execution and kernel work.
- Topic 6 is strategic and should preserve whatever architecture direction emerges.
- Topic 7 should run continuously, but it is most useful once likely changes are known.
