# ADR-0033: Public SDK Core And Adapter Boundaries

## Status

Accepted

## Context

The repo now has a clearer processing authority split, and the packaging
boundary is now encoded more explicitly than it was when this ADR was first
written.

Before the recent hardening pass:

- core crates, app adapters, export scripts, and compatibility facades all sit in one workspace
- the root `ophiolite` crate is still a broad compatibility facade
- desktop- and TraceBoost-specific code can look publishable simply because Cargo defaults to `publish = true`
- the new processing authoring seam is app-local by design, but that intent was not encoded explicitly

That is workable for internal development, but it is the wrong default if `ophiolite` is expected to grow into a public SDK with a narrower and more durable surface.

## Decision

The public SDK direction is intentionally narrower than the full workspace.

This ADR adopts the four-point scope explicitly:

1. keep `ophiolite-operators`, shared contracts, runtime, and execution as publishable core
2. extract authoring semantics into a shared crate only when a second real consumer exists
3. leave desktop/Tauri concerns as adapters
4. keep filesystem policy, app paths, and workspace persistence out of the public SDK surface

The dedicated public-core facade crate is:

- `crates/ophiolite-sdk`

The same boundary rule now also applies to the embeddable chart SDK surface:

- public chart consumption happens through `@ophiolite/charts` and approved public subpaths
- first-party consumers such as `traceboost-demo` must use those public package entrypoints rather than chart internals
- chart wrappers may expose embedder-facing view seams such as viewport data sources, renderer status, and renderer telemetry
- renderer/controller internals remain internal implementation detail unless they are promoted deliberately into the public wrapper API

### Publishable core direction

The crates that define the intended public core direction are:

- `ophiolite-sdk`
- `ophiolite-operators`
- publishable shared contracts:
  - `seis-contracts-core`
  - `seis-contracts-views`
  - `seis-contracts-operations`
- `ophiolite-seismic-runtime`
- `ophiolite-seismic-execution`
- `seis-runtime`

Supporting crates that remain transitively required by those layers may also need to be published later, but they are not treated as the initial stable SDK promise by this ADR.

The contract layer is now expected to stay project-independent. Project-aware request DTOs may remain elsewhere, but shared operations/view/core contract packages should not depend on `ophiolite-project`.
Compatibility rename shims such as `seis-contracts-interop` remain adapter-local and must not be re-exported from `ophiolite-sdk`.

### Internal or adapter direction

The following are explicitly not part of the public SDK surface in the current phase:

- the root `ophiolite` compatibility facade
- `ophiolite-project`
- `ophiolite-cli`
- `traceboost-app`
- `traceboost-desktop`
- contract export binaries and similar repo tooling

These packages are internal integration, compatibility, or application layers and should not be treated as public API commitments.

### Authoring-boundary rule

Processing authoring remains app-local until a second real consumer exists.

That means:

- `apps/traceboost-demo/src-tauri/src/processing_authoring.rs` remains adapter-local
- it must not become an accidental public SDK surface
- extraction into a shared crate is a follow-on step only when a second consumer justifies the extra abstraction

### Filesystem and workspace rule

Filesystem policy, app paths, and workspace persistence must remain outside the public SDK contract.

They may be used by adapters and applications, but they are not part of the durable publishable core promise.

## Implementation

The repo now encodes this boundary in four concrete ways:

1. internal and adapter packages are marked `publish = false`
2. `workspace.metadata.ophiolite.boundaries` classifies package ownership and allowed dependency directions
3. `scripts/ophiolite-boundary-check` enforces those class rules against the live Cargo workspace
4. `crates/ophiolite-sdk` no longer re-exports adapter-local compatibility shims such as `seis-contracts-interop`

This ADR does not force early extraction of authoring semantics.

The app-local desktop shell remains separate by policy and tooling:

- TraceBoost desktop commands are declared in `apps/traceboost-demo/desktop-command-boundary.json`
- generated frontend bridge stubs consume that manifest/backend command table
- neither the command names nor the app-local transport glue are treated as public SDK surface

For the chart SDK, the same public-versus-internal split is now explicit in the wrapper layer:

- `@ophiolite/charts` owns the public launch-wrapper contract
- embedder-facing chart seams such as `dataSource`, `onDataSourceStateChange`, `onRendererStatusChange`, and `onRendererTelemetry` are allowed public wrapper APIs
- controller event channels and renderer callback wiring remain internal to `charts/` and are not part of the public platform/core SDK promise

## Consequences

### Positive

- accidental publication of app/adaptor crates becomes less likely
- the intended public surface is narrower and more defensible
- TraceBoost desktop code can evolve without pretending to be general SDK API
- a later public packaging effort can start from the already-identified core rather than from the whole workspace

### Tradeoff

- the current root `ophiolite` crate remains useful internally but is no longer the implied publication target
- some transitive dependencies of the future public core still need later review before real publication

## Non-goals

- no immediate crate split for processing authoring
- no immediate publication process
- no promise that every currently publishable crate is already stable enough for external consumers
- no attempt to turn TraceBoost desktop commands into public SDK API

## Follow-on

If and only if a second real consumer appears for processing authoring semantics, the next step is:

- extract authoring logic into a shared crate with no Tauri, workspace-state, or filesystem-policy dependency

Until then, the adapter-local authoring seam is the correct shape.
