# ADR-0031: Shared Seismic Execution Planner and Bounded Local Job Service

## Status

Accepted

## Context

`ophiolite` already has useful runtime efficiency work for seismic processing:

- trace-local kernels use shared CPU parallelism in `ophiolite-seismic-runtime`
- section preview reuses prefix work and cacheable artifacts
- `traceboost-demo` persists processing lineage and checkpoint-like reusable outputs

The current orchestration boundary is still too app-local and too coarse for the next user need:

- users want to run one pipeline across many seismic cubes
- `traceboost-demo` currently spawns per-request worker threads rather than using one bounded shared scheduler
- whole-volume materialization is still largely driven as a serial tile loop
- Python automation remains mostly synchronous subprocess orchestration rather than a first-class job model

OpenDTect is useful as a reference point here: it distinguishes multi-threading from distributed batch execution and exposes a real batch/distribution-oriented job model. `ophiolite` should learn from that direction without jumping straight to remote/distributed execution before the local execution model is ready.

Local inspection of Rust data-processing systems such as DataFusion, Ballista, Polars, Tokio guidance around blocking work, and cache libraries such as Moka also points to the same shape:

- explicit plans
- bounded execution
- reusable artifacts
- scheduler-visible partitions
- metrics before deeper optimization

## Decision

`ophiolite` will introduce a shared local seismic execution architecture built around:

- explicit operator execution traits in `ophiolite-seismic-runtime`
- an immutable, serializable `ExecutionPlan`
- a bounded process-wide job service for seismic execution
- first-class `submit_job` and `submit_batch` semantics for SDK/API clients
- hidden reusable checkpoints and artifact-level cache reuse by default
- future-ready artifact and partition abstractions that can later support remote executors

Phase 1 is intentionally local-first, not distributed-first.

The public user-facing unit of work is:

- one `PipelineRun` over one dataset
- one `PipelineBatchRun` over many datasets using the same pipeline in v1

The internal execution unit is:

- scheduler-visible partitions for eligible stages

Preview and materialization continue to be distinct planning modes, but they use one shared planner model.

## Why

This decision is intended to solve the immediate product/API gap without hardening the wrong abstraction.

The wrong first move would be to keep adding more loop-level parallelism inside app-local orchestration. That could improve isolated numbers while making the future SDK/API boundary worse.

The right first move is:

- move orchestration into a shared boundary
- make execution plans explicit
- bound concurrency across jobs
- then parallelize intra-job execution against that shared scheduler

This gives users a credible answer to "run the same pipeline over many cubes" while preserving a migration path toward:

- partition-aware retries
- deterministic cache reuse
- halo-aware planning for neighborhood operators
- eventual remote/distributed executors

## Consequences

### Accepted consequences

- `traceboost-demo` becomes a client of shared execution services rather than the owner of seismic job orchestration
- Python automation moves toward submit/poll/cancel job semantics, with blocking helpers layered above them
- execution planning becomes a real shared concern in `ophiolite-seismic-runtime`
- job scheduling becomes a real shared concern above the runtime kernels
- artifact reuse decisions move toward planner/service ownership rather than app-local ad hoc decisions
- derived outputs may choose chunking/layout policies better suited to downstream use instead of always inheriting source tiling

### Explicit non-goals for this phase

- no remote or multi-machine executor in the first implementation
- no generic cross-family `run_operator(...)` API
- no fine-grained partition-fragment cache as the first cache model
- no scheduler work that degrades preview responsiveness in favor of background throughput
- no requirement that every operator family become partitionable immediately

## Implementation Order

The intended migration order is:

1. add operator execution traits in `ophiolite-seismic-runtime`
2. add serializable execution-plan types and a planner
3. add a new shared local execution/job-service boundary
4. move `traceboost-demo` and Python compatibility layers onto submit/poll/cancel semantics
5. add bounded batch execution across many datasets
6. add partitioned trace-local whole-volume execution
7. add cache hardening and chunk-shape policy
8. add halo-aware planning for neighborhood-style operators
9. only then evaluate remote/distributed executors

## Initial Shape

The recommended initial package split is:

- `ophiolite-seismic-runtime`
  - kernels
  - store IO
  - execution traits
  - planner
  - partition execution
- a new shared execution crate
  - job service
  - batch orchestration
  - scheduler
  - metrics
  - artifact/cache policy wiring

The initial public execution surface should support:

- `submit_job`
- `submit_batch`
- `get_job`
- `get_batch`
- `cancel_job`
- `cancel_batch`

## Success Criteria

This decision is working when:

- users can submit one seismic pipeline across many cubes through shared SDK/API semantics
- queueing and bounded concurrency are visible and measurable
- `traceboost-demo` no longer depends on unmanaged per-request execution threads for this workflow
- Python callers no longer need to build their own outer job loop to manage many-cube runs
- planner output is inspectable enough to explain cache reuse, checkpoint insertion, and execution shape
- later partitioned execution work can land without redesigning the public orchestration boundary

## Follow-on Documents

- `seismic-execution-service-implementation-sketch.md`

