# ADR-0034: Canonical Processing Identity, Debug, and Compatibility Surface

## Status

Accepted

## Context

`ophiolite` already had the main architectural pieces for shared seismic processing:

- family-specific processing contracts in `ophiolite-seismic`
- explicit planner and runtime layers in `ophiolite-seismic-runtime`
- bounded local job orchestration in `ophiolite-seismic-execution`
- a thin-client migration direction for TraceBoost and other consumers

The remaining integration risk was no longer "missing architecture". It was divergence across surfaces that should have described the same derived artifact and the same execution reality:

- cache keys versus realized artifact identity
- plan-time reuse assumptions versus realized runtime reuse
- planner outputs versus persisted lineage
- store/prestack/package metadata versus app-facing debug views
- handwritten app compatibility logic versus generated shared contracts

Without one canonical model for semantic/version envelopes, artifact identity, debug records, and compatibility policy, the system would look shared while still behaving as several loosely aligned implementations.

## Decision

`ophiolite` will treat canonical processing identity and debugability as shared architecture, not app-local tooling.

The durable rules are:

- shared processing contracts own explicit semantic/version envelopes for:
  - pipeline semantics
  - operator-set identity
  - planner-profile identity
  - source identity
  - inspectable-plan schema
  - execution-plan schema
  - lineage schema
- canonical processing identity is path-independent and derived from normalized seed structs rather than ad hoc path strings or handwritten manifest hashing
- the planner computes canonical artifact identity up front, and runtime/store/prestack/package paths persist that planner-computed identity unchanged rather than recomputing competing identities later
- persisted lineage, cache registration, package manifests, and app-facing inspectable/debug views must all describe the same artifact model:
  - artifact key
  - logical domain
  - chunk grid
  - geometry fingerprints
  - materialization class
  - reuse semantics
  - runtime/store-writer semantic envelope
- realized reuse is allowed to diverge from plan-time reuse assumptions, but that divergence must be represented explicitly through structured runtime/debug fields rather than hidden shortcuts
- the canonical debug contract lives in shared generated contracts and includes:
  - typed plan decisions
  - typed reuse decisions
  - runtime stage snapshots
  - runtime events/timelines
  - section-assembly debug records
  - structured runtime policy divergence
- product apps such as TraceBoost consume the canonical generated model directly and should not invent a second canonical processing/debug contract
- package open and cache reuse are strict compatibility operations:
  - package blobs are digest-verified
  - packaged config and packaged lineage must agree
  - cache validation is canonical and family-aware
  - noncanonical lineage may remain readable for inspection/migration, but is rejected for silent canonical reuse

## Why

This decision closes the last major gap between "shared runtime" and "shared meaning".

The wrong shape would be:

- planner identity in one place
- store lineage in another
- cache validation in a third
- app debug interpretation in a fourth

That leads to brittle reuse, package drift, and explanations that cannot actually be trusted.

The right shape is:

- one canonical semantic envelope
- one canonical artifact identity path
- one stable inspectable/debug contract
- one explicit compatibility policy

This gives `ophiolite` a defensible answer to:

- why this output exists
- why this reuse happened or did not happen
- why a package was accepted or rejected
- whether runtime behavior diverged from planner intent

## Consequences

### Accepted consequences

- canonical identity derivation becomes shared runtime infrastructure rather than a convenience helper
- runtime, store, prestack, cache, and package code must all treat planner-produced artifact identity as authoritative
- debug surfaces become more structured and more durable, even if that increases contract volume
- generated TS/schema artifacts become part of the canonical processing surface rather than a thin compatibility afterthought
- compatibility policy becomes explicit and fail-closed for canonical reuse/package acceptance

### Explicit non-goals

- no promise that every legacy cache artifact can be reused canonically forever
- no support for silent downgrade from canonical lineage to partially readable legacy lineage
- no second app-owned processing debug model beside the shared contract model
- no path-dependent identity shortcuts for convenience

## Implementation Shape

The intended shape is:

```text
pipeline + operator-set + planner-profile semantics
  -> planner-owned canonical artifact identity
  -> runtime/store/prestack/package persisted lineage
  -> cache/package compatibility validation
  -> inspectable plan/debug contract
  -> generated TS contracts
  -> thin clients
```

The key boundaries are:

- `ophiolite-seismic`
  - canonical shared contracts
  - semantic/version envelopes
  - inspectable plan/debug model
- `ophiolite-seismic-runtime`
  - canonical identity derivation
  - planner-produced artifact identity
  - persisted lineage/package/store metadata
  - package compatibility enforcement
- `ophiolite-seismic-execution`
  - realized runtime snapshots, runtime events, divergence reporting
- TraceBoost and other clients
  - render canonical generated contracts
  - do not reconstruct canonical processing/debug meaning locally

## Success Criteria

This decision is working when:

- one derived artifact receives the same canonical identity across planner, runtime, store, prestack, package, cache, and debug views
- plan-time reuse keys and realized artifact identity are both visible and do not silently drift
- package open and cache reuse fail closed on canonical incompatibility
- debug UI can explain plan, runtime, reuse, lineage, and section assembly from shared contracts without app-local semantic reconstruction
- contract/schema generation checks catch drift before product code depends on stale generated shapes

## Follow-on Documents

- `processing-lineage-cache-compatibility-policy.md`
- `processing-canonical-integration-plan-2026-04.md`
- `ADR-0015-authored-models-compiled-runtime-assets-and-display-dtos.md`
- `ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md`
- `ADR-0032-processing-authority-and-thin-client-migration.md`
