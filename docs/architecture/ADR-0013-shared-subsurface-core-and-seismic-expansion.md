# ADR-0013: Shared Subsurface Core and Seismic Expansion

## Status

Accepted

## Decision

`ophiolite` becomes the long-term shared subsurface SDK and platform-core repository for both well and seismic domain families.

This means:

- `ophiolite` owns canonical domain models, import/IO layers, package/storage conventions, project/catalog identity, compute, and generated app-boundary contracts
- product repositories such as `TraceBoost` consume that core rather than owning duplicate canonical seismic types over time
- visualization repositories such as `Ophiolite Charts` stay outside the core and use adapter/view models over canonical asset/query results

The first implementation step is a shared seismic core inside `ophiolite` that owns canonical seismic descriptors, app-boundary section/trace DTOs, SEG-Y IO, and runtime/store execution.

## Why

- the current repo split has started to duplicate concerns:
  - `ophiolite` is becoming the domain-first subsurface SDK
  - `TraceBoost` already owns seismic IO, runtime contracts, and product workflows
- long-term expansion needs one place for canonical subsurface types, not separate well and seismic cores
- desktop apps, compute layers, and chart adapters should share one semantic backbone while still keeping UI/view models separate from domain models
- the existing `ophiolite` compatibility facade and workspace split already make it the better foundation for a shared core than a product-first repo

## OSDU Position

OSDU remains a conceptual alignment target, not the literal internal schema.

Use OSDU for:

- entity and identity thinking
- interoperability vocabulary
- future mapping/import-export opportunities

Do not use OSDU as:

- the direct public SDK surface
- the local-first package/query/edit model
- the primary desktop app contract shape

`ophiolite` should remain domain-first and app-optimized even when it later adds explicit OSDU mapping layers.

## Consequences

- seismic becomes a first-class family in the shared core rather than a product-only concern
- `TraceBoost` can evolve toward a thinner product/app composition layer over time
- generated cross-language contracts should eventually come from core-owned Rust boundary types rather than product-owned schemas
- runtime stores and section/tile views are treated as derived representations over canonical seismic assets, not as the only conceptual source of truth
- migration should be phased so product velocity is not blocked by a one-shot repo reset
