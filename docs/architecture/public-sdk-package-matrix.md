# Public SDK Package Matrix

This document records which workspace packages are intended public-core candidates, which are blocked, and which are explicitly adapter/internal.

It exists to keep the publication story honest.

## Status Legend

- `Public-core candidate`: intended part of the future publishable SDK surface
- `Blocked`: directionally part of the public-core story, but currently coupled to internal/application concerns
- `Internal`: explicitly not part of the public SDK surface

## Matrix

| Package | Status | Reason | Notes |
| --- | --- | --- | --- |
| `ophiolite-sdk` | Public-core candidate | narrow public facade over publishable core layers | intended top-level SDK entry point |
| `ophiolite-operators` | Public-core candidate | shared operator vocabulary and metadata types | intended durable operator-definition surface |
| `ophiolite-seismic` | Public-core candidate | canonical seismic contracts and shared domain types | transitive support crate for runtime/contracts |
| `ophiolite-seismic-runtime` | Public-core candidate | planner/runtime semantics | core execution meaning belongs here |
| `ophiolite-seismic-execution` | Public-core candidate | shared job/batch orchestration | thin execution-service layer |
| `seis-contracts-core` | Public-core candidate | shared transport/domain contract package | no current dependency on app adapters |
| `seis-contracts-views` | Public-core candidate | shared display/view contract package | publishable as part of shared contract surface |
| `seis-runtime` | Public-core candidate | stable consumer-facing runtime bridge over seismic runtime/contracts | suitable public runtime-facing layer if kept app-neutral |
| `seis-contracts-operations` | Public-core candidate | shared operations contract package | now decoupled from `ophiolite-project` |
| `seis-contracts-interop` | Internal | TraceBoost compatibility rename shim over `seis-contracts-operations` | keep outside `ophiolite-sdk` and public-core promises |
| root `ophiolite` crate | Internal | broad compatibility facade over mixed workspace concerns | useful internally, wrong public promise |
| `ophiolite-project` | Internal | project/catalog integration layer with broad local-app concerns | not current public SDK boundary |
| `ophiolite-cli` | Internal | app/tooling surface | not a library SDK package |
| `traceboost-app` | Internal | TraceBoost application adapter | product-specific |
| `traceboost-desktop` | Internal | Tauri desktop shell | adapter only |
| `contracts-export` | Internal | repo tooling | not SDK |
| `traceboost-contracts-export` | Internal | repo tooling / compatibility | not SDK |

## Immediate Boundary Rules

1. Public-core candidates may depend on other public-core candidates and small supporting domain crates.
2. Public-core candidates must not depend on Tauri, app paths, workspace persistence, or desktop-specific filesystem policy.
3. Public-core candidates must not depend on product-specific adapters such as `traceboost-app` or `traceboost-desktop`.
4. Authoring semantics remain app-local until a second real consumer exists.

## Current Extraction Rule

`apps/traceboost-demo/src-tauri/src/processing_authoring.rs` is intentionally adapter-local.

It should only move into a shared crate when:

- a second real consumer appears
- the extracted surface no longer depends on app paths, workspace persistence, or Tauri

## Related Policy

Versioning and support expectations for these package classes are defined in `public-sdk-support-policy.md`.
