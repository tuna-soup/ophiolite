# ADR-0005: Staged Workspace Split and Table Boundary

## Status

Accepted

## Decision

`ophiolite` now uses a staged Cargo workspace split:

- `ophiolite-core`
- `ophiolite-parser`
- `ophiolite-table`
- `ophiolite-package`
- `ophiolite-cli`
- root compatibility facade: `ophiolite`

This split is intentionally incomplete in one specific area:

- `ophiolite-table` exists as its own crate boundary now
- `CurveTable` still originates from the core layer in this phase
- `LasFile::data()` continues to work because the runtime table type remains colocated with the `LasFile` owner

Arrow/Parquet conversion has already been moved out of `CurveTable` and into the package crate.

## Why

- the workspace split reduces coupling immediately for parser, package, and CLI concerns
- preserving the current `LasFile::data()` API avoids a breaking redesign during the same migration
- Rust ownership rules make it awkward to move `CurveTable` fully behind a separate crate while keeping the same inherent-method API

## Consequences

- the current architecture should be described as a staged workspace, not a fully completed crate separation
- documentation must call out that the table boundary exists, but the runtime table type still originates in core for now
- future work must revisit the `LasFile` to `CurveTable` boundary explicitly rather than letting the staging compromise become permanent

## Planned Resolution

This compromise should be revisited after:

1. DTO/query contracts stabilize
2. package edit/save semantics stabilize
3. package schema/version guarantees stabilize

At that point, `ophiolite` should choose one of:

- a redesigned accessor boundary for runtime table access
- a trait-based/runtime-view approach
- a fuller move of sample-table ownership out of core
