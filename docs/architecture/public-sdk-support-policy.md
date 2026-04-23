# Public SDK Support Policy

This document defines the support and versioning posture for the current public-core direction.

It exists so `ophiolite-sdk` is not just a crate name. It is the start of an explicit external-consumer promise.

## Scope

The current public-core scope is:

- `ophiolite-sdk`
- `ophiolite-operators`
- shared contracts that remain app-neutral
- shared runtime and execution crates that remain app-neutral

The current scope explicitly excludes:

- desktop/Tauri commands
- workspace persistence
- filesystem path policy
- app-local authoring seams
- product-specific adapters and repo tooling

## Stability Bands

### Stable-by-direction

These surfaces are the ones the repo should actively shape toward public consumption:

- operator catalog vocabulary and metadata
- shared contract DTOs that are not project- or app-specific
- runtime/planner semantics for shared seismic execution
- job/batch execution service semantics
- the `ophiolite-sdk` facade and the modules it re-export publicly

Stable-by-direction does not mean frozen forever today. It means changes to these surfaces should be treated as SDK changes, not casual internal refactors.

### Incubating

These surfaces may still move materially while the architecture settles:

- newly introduced shared authoring crates, if they appear later
- advanced planning/execution diagnostics that are not yet consumed by more than one client
- newly added contract families before they have completed one migration window

Incubating surfaces should be documented as such before they are exposed through `ophiolite-sdk`.

### Internal

These surfaces are not part of the SDK promise:

- root `ophiolite` compatibility facade
- `ophiolite-project`
- CLI, desktop, and app crates
- export scripts
- adapter-local persistence and filesystem helpers

## Versioning Rules

1. `ophiolite-sdk` is the primary public entry point.
2. Crates re-exported by `ophiolite-sdk` must not take breaking public-shape changes casually.
3. Breaking changes to `ophiolite-sdk` or its re-exported public modules require:
   - an ADR or equivalent architecture note
   - migration notes
   - one compatibility window when practical
4. Purely internal refactors in adapter crates do not require SDK migration notes unless they leak into public-core crates.

## Change Classification

Treat these as SDK-affecting changes:

- removing or renaming public re-exports from `ophiolite-sdk`
- changing shared contract field names or serialized enum representations
- changing operator catalog identifiers or family identifiers
- changing execution/job DTO semantics in a way that affects callers

Treat these as internal changes:

- moving Tauri commands
- changing app-local persistence code
- changing filesystem layout for adapter-owned stores
- changing frontend-only view-model helpers

## Extraction Rule

Authoring semantics stay app-local until a second real consumer exists.

When that happens, extraction into a shared crate is allowed only if the extracted surface has no dependency on:

- Tauri
- app paths
- workspace persistence
- product-specific filesystem policy

## Publishing Rule

No crate should be published just because Cargo allows it.

Before publication, a public-core candidate should have:

- explicit scope documentation
- a clear owner
- migration expectations
- at least one focused compile/test job in CI

## Near-Term Operating Mode

For now, the repo should behave as if `ophiolite-sdk` is the future external entry point, even if publication is not immediate.

That means new public-facing shared capabilities should prefer one of these homes:

- `ophiolite-operators`
- shared contract crates
- shared runtime/execution crates
- `ophiolite-sdk` as the narrow facade

New desktop/app concerns should not be added to that facade.
