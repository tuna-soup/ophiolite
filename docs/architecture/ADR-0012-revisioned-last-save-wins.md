# ADR-0012: Revisioned Overwrite-Oriented Saves

## Status

Accepted

## Decision

Ophiolite keeps a simple **overwrite-oriented** local edit model, but every successful save now creates a new **immutable local revision**.

This applies to:

- package-backed log edits through `PackageSession`
- structured asset edit sessions
- imported assets
- compute-derived assets

Historical saved states live in hidden local revision stores, and those hidden stores are canonical.
The visible package or asset path remains a materialized current-head view:

- package roots use `.ophiolite/revisions/`
- `OphioliteProject` uses a hidden project-local asset revision store

Each revision record captures:

- revision id
- parent revision id when present
- blob refs for saved metadata and Parquet payloads
- a typed machine diff
- a readable change summary
- within-asset revision lineage only

Derivation provenance for compute or other derived assets remains separate from same-asset revision lineage.

## Why

- Parquet is treated as immutable saved payload data rather than something Ophiolite patches in place
- users want simple overwrite-oriented workflows rather than merge/conflict-heavy desktop behavior
- future sync still needs lineage and diff information
- revision history and diff inspection should not require changing the normal save UX
- canonical revision-first storage keeps future sync and rollback options open without forcing extra UI complexity now

## Consequences

- saves still feel like overwrite saves in the UI
- every successful save writes a canonical new saved snapshot and advances the current head
- package and asset history are now inspectable without introducing Git-like branching semantics
- typed machine diffs plus readable change summaries become the bridge to future sync, audit, and comparison workflows
- historical revisions are stored locally even though the visible asset/package path remains stable
- one save of one asset creates one asset revision; multi-asset save grouping is deferred
- revision retention, garbage collection, and compaction are deferred design concerns, not part of the first implementation

## Non-Goals

This ADR does not introduce:

- collaborative live editing
- merge or CRDT semantics
- partial in-place Parquet mutation
- Iceberg/Delta/Hudi as the local package format
- cell-level Parquet patching
- byte-level file diffing

Those remain possible future server/distribution concerns rather than the local editing model.
