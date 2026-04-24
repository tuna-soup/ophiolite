# ADR-0032: Processing Authority and Thin-Client Migration

## Status

Accepted

## Context

`ophiolite` already has most of the target building blocks for seismic processing:

- cross-family operator catalog vocabulary in `crates/ophiolite-operators`
- seismic built-in operator catalog assembly in `crates/ophiolite-seismic`
- project-level operator discovery in `OphioliteProject::list_operator_catalog(...)`
- explicit planning in `ophiolite-seismic-runtime`
- shared local job orchestration in `ophiolite-seismic-execution`

That means the main remaining gap is no longer "missing architecture". The main remaining gap is split authority.

Today processing semantics are still spread across several layers:

- shared contracts own canonical pipeline and job shapes
- runtime and execution crates own plan and scheduler behavior
- `traceboost-demo` still owns a large amount of canonical authoring behavior
- frontend bridge code still mirrors some canonical catalog DTOs locally
- TypeScript contract distribution still has both root and TraceBoost-oriented paths

This is directionally workable, but it hardens the wrong ownership model:

- operator discovery becomes partly backend-owned and partly frontend-owned
- pipeline authoring rules become partly contract-owned and partly UI-owned
- workspace session persistence accepts processing pipeline blobs without one canonical authoring boundary
- external SDK surfaces risk inheriting frontend-shaped semantics instead of shared product semantics

`ADR-0030` and `ADR-0031` already established the two core directions:

- one unified operator catalog vocabulary
- one shared planning and job-service boundary

This ADR turns those accepted directions into one migration program for processing authoring, contract authority, and client thinning.

## Decision

`ophiolite` will treat the processing stack as six explicit authority layers:

1. operator definition
2. pipeline contract
3. authoring/workspace semantics
4. execution plan
5. job/batch orchestration
6. client/UI state

The canonical owner for each layer is:

- operator definition:
  - `crates/ophiolite-compute`
  - `crates/ophiolite-seismic`
  - shared catalog vocabulary in `crates/ophiolite-operators`
- pipeline contract:
  - shared Rust contracts and generated TS contracts
- authoring/workspace semantics:
  - a backend-owned processing authoring boundary
  - phase one may be app-local in `apps/traceboost-demo/src-tauri`
- execution plan:
  - `crates/ophiolite-seismic-runtime`
- job/batch orchestration:
  - `crates/ophiolite-seismic-execution`
- client/UI state:
  - frontend view models and components

The migration will preserve the current public modeling constraints:

- processing remains family-specific
- execution entry points remain family-specific
- the public processing model remains linear and checkpoint-segmented
- no generic `run_operator(...)` API is introduced in this migration
- no DAG-style public authoring model is introduced in this migration

The TypeScript contract strategy is:

- `contracts/ts/ophiolite-contracts` becomes the canonical TS contract distribution target
- TraceBoost-oriented TS contract packages remain as compatibility re-export or compatibility distribution paths for a bounded migration window
- generated schema/version constants should be consumed by product code rather than duplicated by app-local request builders

The processing authoring strategy is:

- canonical processing authoring rules move out of `traceboost-demo` frontend state modules
- phase one introduces an app-local Rust backend authoring boundary rather than a new shared crate
- extraction into a shared crate happens only if a second real consumer appears
- canonical processing debug, lineage, reuse, and runtime-state semantics also move behind shared/backend-owned contracts rather than product-local debug panels inventing their own meaning

The compatibility strategy is:

- additive first
- switch consumers second
- delete compatibility last

The planned compatibility window is two release trains:

1. introduce the new authority boundary and switch primary consumers
2. remove compatibility paths after successful adoption

Legacy preset and workspace-processing shapes will migrate eagerly on load and then be rewritten in canonical form.

## Why

This decision is intended to solve three specific problems without overfitting to products such as MERLIC.

### 1. Ophiolite does not need a richer public graph model yet

The current product shape is well served by family-specific linear pipelines and planner-owned stage segmentation.

The wrong move would be to copy a more elaborate tool graph just because other products need one.

The right move is to make current authority boundaries coherent first.

### 2. Shared runtime and execution layers already exist

The repo has already paid for:

- explicit operator execution traits
- explicit execution plans
- bounded local job orchestration

If canonical authoring logic remains in the frontend, those shared layers cannot become the real SDK/product boundary.

### 3. The current duplication creates long-term API risk

If frontend fallback catalogs, cloned DTOs, and app-local authoring rules remain in place, future SDK and Python surfaces will copy app-specific behavior instead of using backend-owned semantics.

That would make the architecture look shared while still behaving app-local.

## Consequences

### Accepted consequences

- `traceboost-demo` becomes a thinner client over shared catalog, authoring, planner, and execution boundaries
- a backend authoring module becomes the canonical owner of processing workspace semantics
- generic workspace persistence remains the storage sink, but stops being the semantic owner of processing authoring rules
- TS contract distribution gets one canonical root target
- the migration is phased and compatibility-heavy rather than a one-shot rewrite
- TraceBoost debug UX becomes a renderer over canonical generated plan/runtime/lineage contracts rather than an app-owned semantic layer

### Explicit non-goals

- no distributed executor
- no public DAG authoring model
- no generic cross-family execution API
- no immediate crate renaming campaign for all contract packages
- no immediate extraction of an app-local processing authoring module into a shared crate before a second consumer exists

## Authority Model

The intended authority model is:

```text
family-owned operator definitions
  -> unified operator catalog
  -> backend-owned authoring/workspace semantics
  -> planner-owned execution plan
  -> execution-service-owned job/batch orchestration
  -> thin clients
```

The companion matrix is:

- `processing-authority-matrix.md`

## Implementation Order

The intended migration order is:

1. write and adopt a processing authority matrix
2. finish the operator catalog as the only canonical operator-discovery source
3. remove frontend-local semantic ownership of operator metadata
4. introduce a backend-owned processing authoring boundary
5. route processing session-pipeline persistence through that authoring boundary
6. thin `traceboost-demo` processing state down to UI/view concerns
7. make `contracts/ts/ophiolite-contracts` the canonical TS distribution target
8. migrate CLI/Python and other consumers onto the same authority model
9. remove compatibility paths

Additional hardening that now belongs to this migration includes:

10. move processing debug fetch/state/event APIs onto canonical generated contracts
11. replace app-local schema/version literals with generated contract versions
12. remove duplicate subvolume/debug/reuse interpretation paths once canonical consumers are fully switched

## Success Criteria

This decision is working when:

- the frontend no longer owns canonical operator metadata for processing authoring
- the frontend no longer owns canonical processing workspace rules such as checkpoint legality, preset normalization, or output-signature derivation
- the planner and execution service remain the only owners of execution meaning
- root TS contract distribution is canonical and product packages no longer act as competing sources of truth
- SDK, CLI, Python, and desktop consume the same authority layers
- product debug UIs consume shared inspectable/runtime contracts directly and do not reinterpret runtime/cache/package meaning locally
- deletion of compatibility paths becomes a routine cleanup step instead of a redesign

See also:

- `ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`

## Follow-on Documents

- `processing-authority-matrix.md`
- `ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md`
- `ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md`
