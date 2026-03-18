# ADR-0012: Revisioned Last-Save-Wins

## Status

Accepted

## Decision

Lithos keeps **last-save-wins** as the user-facing local edit model, but every successful save now creates a new **immutable local revision**.

This applies to:

- package-backed log edits through `PackageSession`
- structured asset edit sessions
- imported assets
- compute-derived assets

The visible package or asset path remains the stable current head.
Historical saved states live in hidden local revision stores:

- package roots use `.lithos/revisions/`
- `LithosProject` uses a hidden project-local asset revision store

Each revision record captures:

- revision id
- parent revision id when present
- blob refs for saved metadata and Parquet payloads
- a domain-level diff summary

## Why

- Parquet is treated as immutable saved payload data rather than something Lithos patches in place
- users want simple overwrite-oriented workflows rather than merge/conflict-heavy desktop behavior
- future sync still needs lineage and diff information
- revision history and diff inspection should not require changing the normal save UX

## Consequences

- saves still feel like overwrite saves in the UI
- every successful save writes a new saved snapshot and advances the current head
- package and asset history are now inspectable without introducing Git-like branching semantics
- domain-level diffs become the bridge to future sync, audit, and comparison workflows
- historical revisions are stored locally even though the visible asset/package path remains stable

## Non-Goals

This ADR does not introduce:

- collaborative live editing
- merge or CRDT semantics
- partial in-place Parquet mutation
- Iceberg/Delta/Hudi as the local package format

Those remain possible future server/distribution concerns rather than the local editing model.
