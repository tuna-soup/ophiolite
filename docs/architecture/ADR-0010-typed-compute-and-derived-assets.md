# ADR-0010: Typed Compute and Derived Assets

## Status

Accepted

## Context

Lithos already has:

- single-asset log packages
- `LithosProject` for well/wellbore/asset organization
- typed non-log asset families
- a Tauri harness that can browse and inspect those assets

The missing capability was a type-safe way to run computation against the data without falling back to loose mnemonic matching or app-specific scripting. In particular, functions such as VShale should be available for gamma ray curves, but not for unrelated curve types.

## Decision

Lithos adds a dedicated `lithos-compute` workspace crate and a logs-first typed compute layer.

The compute layer uses:

- semantic curve typing
- a typed function registry
- explicit input specs and parameter definitions
- project-aware execution that persists derived sibling assets

Current rules:

- compute eligibility is driven by semantic curve types such as `GammaRay`, `BulkDensity`, `Sonic`, `PVelocity`, and `SVelocity`
- log asset manifests persist curve semantics and may store explicit overrides
- compute runs execute against one selected log asset at a time in the current phase
- outputs are persisted as derived sibling log assets in `LithosProject`
- derived asset manifests record:
  - `derived_from`
  - `compute_manifest`
  - output curve semantics
- latest derived output in a derived collection supersedes the previous current output using the existing last-save-wins/supersede model

## Consequences

Positive:

- app surfaces can list only meaningful functions for the selected data
- compute stays attached to typed wellbore-linked assets rather than raw file blobs
- derived results are traceable and queryable like any other project asset
- the initial implementation stays compatible with the existing single-asset package model

Tradeoffs:

- the first implementation is logs-first; non-log compute functions are deferred
- semantic classification still relies on current aliases, mnemonic heuristics, and explicit overrides where needed
- compute currently materializes derived results as sibling assets rather than mutating the source package/session in place
