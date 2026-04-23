# Seismic Execution Service Implementation Sketch

This document turns `ADR-0031` into a concrete implementation sketch for the current repo.

It is intentionally local-first and milestone-oriented.

## Scope

The immediate user-facing goal is:

- run one pipeline across many seismic cubes with less waiting and better control

The immediate architecture goal is:

- move seismic execution planning and job orchestration into shared Ophiolite boundaries instead of keeping them app-local

This sketch does not yet propose:

- distributed execution
- GPU execution
- generic non-seismic orchestration across every operator family in the repo

## Current Gaps

Today the repo already has meaningful runtime efficiency work:

- trace-local kernels use shared Rayon parallelism
- preview paths reuse prefix work
- TraceBoost persists reusable processing artifacts and lineage

The main gaps are higher in the stack:

- whole-volume materialization is still too serial at the job level
- scheduling is too app-local
- batch-across-cubes is not a first-class SDK/API capability
- cache reuse and checkpoint reuse are not yet governed by one shared planner/service boundary
- Python automation is still too synchronous for serious batch throughput

## Proposed Repo Shape

Recommended boundary split:

- `crates/ophiolite-seismic-runtime`
  - operator execution traits
  - planner
  - stage and partition execution helpers
  - store-aware partition readers/writers
- `crates/ophiolite-seismic-execution`
  - new crate
  - job service
  - batch service
  - bounded scheduler
  - cache registry wiring
  - metrics emission
- `apps/traceboost-demo/src-tauri`
  - thin client over the shared service
- `python/src/ophiolite_sdk`
  - thin client over the same job semantics

The design intent is:

- runtime crate owns execution meaning
- execution crate owns orchestration policy
- apps own UX only

## Core Runtime Types

Recommended first-pass types:

```rust
pub struct OperatorExecutionTraits {
    pub layout_support: LayoutSupport,
    pub preferred_partitioning: PreferredPartitioning,
    pub halo_inline: usize,
    pub halo_xline: usize,
    pub halo_samples: usize,
    pub requires_full_volume: bool,
    pub checkpoint_safe: bool,
    pub deterministic: bool,
    pub preview_prefix_reuse_safe: bool,
    pub memory_cost_class: MemoryCostClass,
}

pub struct ExecutionPlan {
    pub plan_id: String,
    pub planning_mode: PlanningMode,
    pub source: SourceDescriptor,
    pub pipeline: PipelineDescriptor,
    pub stages: Vec<ExecutionStage>,
    pub artifacts: Vec<ArtifactDescriptor>,
    pub scheduler_hints: SchedulerHints,
    pub validation: ValidationReport,
}

pub struct ExecutionStage {
    pub stage_id: String,
    pub stage_kind: StageKind,
    pub input_artifact_ids: Vec<String>,
    pub output_artifact_id: String,
    pub pipeline_segment: Option<PipelineSegment>,
    pub partition_spec: PartitionSpec,
    pub halo_spec: HaloSpec,
    pub chunk_shape_policy: ChunkShapePolicy,
    pub cache_policy: CachePolicy,
    pub retry_policy: RetryPolicy,
    pub progress_units: ProgressUnits,
    pub estimated_cost: CostEstimate,
    pub estimated_peak_memory: ByteSize,
}
```

Recommended planning properties:

- one immutable plan per submitted job
- preview and materialization share one schema with different planning modes
- plans are serializable for diagnostics and future remote execution
- partition enumeration stays mostly lazy so large cubes do not explode plan size

## Execution Service Surface

Recommended shared API:

```rust
pub trait ExecutionService {
    fn submit_job(&self, req: SubmitJobRequest) -> SubmitJobResponse;
    fn submit_batch(&self, req: SubmitBatchRequest) -> SubmitBatchResponse;
    fn get_job(&self, job_id: &str) -> JobStatus;
    fn get_batch(&self, batch_id: &str) -> BatchStatus;
    fn cancel_job(&self, job_id: &str) -> CancelResult;
    fn cancel_batch(&self, batch_id: &str) -> CancelResult;
}
```

Recommended batch options:

- `priority`
- `max_active_jobs`
- `overwrite`
- `cache_mode`
- `fail_policy`

Recommended runtime behavior:

- one process-wide bounded scheduler
- priority-aware queue
- preview lane protected from background batch saturation
- deterministic outputs regardless of partition order
- per-job and per-batch progress
- batch default of continue-on-error with per-item status

## Partitioning Direction

Recommended v1 intra-job partition shape for post-stack trace-local materialization:

- `tile_group`

Why:

- the runtime is already tile-native
- it is the lowest-risk migration from the current serial tile loop
- it aligns with future chunk-aware reads and writes

Recommended partition contract requirements:

- read/compute/write ownership per partition
- explicit halo metadata even when zero
- independent partition retries
- cancellation at partition boundaries at minimum
- global budgets across jobs and partitions

Neighborhood and gather-heavy paths should follow later, once the partition contract is stable.

## Cache and Artifact Policy

Recommended cache model in the first phases:

- cache exact final artifacts
- cache reusable hidden checkpoints
- do not cache arbitrary micro-partition fragments

Recommended cache behavior:

- automatic admission with policy
- size-aware eviction
- cache validation against source fingerprint, pipeline hash, runtime version, and store format version
- invalidate corrupt entries and recompute from source

The first cache model should stay artifact-oriented because it matches the current repo better and is easier to validate.

## Milestones

### Milestone 1

Planner plus shared local job service plus batch API.

Deliver:

- operator execution traits
- serializable plans
- bounded global queue
- `submit_job` and `submit_batch`
- poll and cancel semantics
- TraceBoost migration onto shared job semantics
- Python compatibility layer over the same job model

Do not deliver:

- remote execution
- halo-aware partition scheduling
- shard-level cache

### Milestone 2

Partitioned trace-local materialization.

Deliver:

- `tile_group` partition execution
- per-partition retries
- scheduler-visible partition progress
- improved cancellation latency
- bounded concurrency across jobs and partitions

### Milestone 3

Shared cache hardening and chunk-shape policy.

Deliver:

- exact-output reuse through shared service
- prefix-checkpoint reuse through shared service
- size-aware eviction
- derived-output chunking policy

### Milestone 4

Halo-aware planning for neighborhood and subvolume-style operators.

Deliver:

- lateral/sample halo metadata enforcement
- planner validation for cross-partition-safe execution
- neighborhood-aware stage planning

Distributed execution should be revisited only after Milestones 1 through 4 are stable and benchmarked.

## First PR Sequence

Recommended first PR sequence:

1. add `OperatorExecutionTraits` and related enums in `ophiolite-seismic-runtime`
2. add `ExecutionPlan`, `ExecutionStage`, and planning-mode types
3. add a trace-local post-stack planner that still emits serial execution stages
4. add a new `ophiolite-seismic-execution` crate with job records, batch records, queueing, and metrics hooks
5. add CLI or service commands for submit, poll, and cancel
6. migrate `traceboost-demo` to the shared execution service
7. migrate Python onto the same job semantics
8. add partitioned trace-local execution
9. move cache decisions into planner and service ownership
10. add chunk-shape policy and later halo-aware planning

The first PR should stay small. It should stabilize the shared abstraction before changing execution behavior.

## Metrics and Benchmarks

Parallelization without measurement is likely to become theater.

Required metric classes:

- planner metrics
  - plan build time
  - cache decision counts
  - estimated versus actual partition counts
- scheduler metrics
  - queue wait
  - active jobs
  - active partitions
  - cancellation latency
- execution metrics
  - per-stage duration
  - per-partition duration
  - retry counts
  - bytes read and written
  - peak memory
- cache metrics
  - exact hit rate
  - prefix hit rate
  - reused bytes
  - eviction counts
- UX metrics
  - preview time to first result
  - full-job completion time
  - batch makespan
  - completed-with-errors rate

Required benchmark classes:

- single interactive preview
- single full materialization
- multi-cube batch throughput

Those benchmarks should exist for at least:

- trace-local post-stack paths
- neighborhood-style paths when they land
- gather processing after the post-stack path is stable

## External Reference Points

Useful external patterns for this work:

- OpenDTect for batch/distributed job framing and memory-aware chunking
- DataFusion for plan-oriented execution
- Ballista for local-to-distributed evolution concepts
- Polars for lazy execution and streaming mindset
- Tokio guidance for bounded blocking work
- Moka for size-aware cache policy patterns

Those references should inform the shape of the implementation without being copied mechanically into Ophiolite.

