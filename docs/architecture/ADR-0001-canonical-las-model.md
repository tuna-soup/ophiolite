# ADR-0001: Canonical LAS Model

## Status

Accepted

## Decision

`ophiolite` exposes a domain-first LAS model rather than a storage-first API.

This ADR governs the log/LAS slice of the system. In the current architecture, that slice sits inside a broader well-domain SDK that also includes project/catalog organization and typed non-log asset families.

The public surface centers on:

- `LasFile`
- section/header access
- curve access/query helpers
- `CurveTable`

It does not center on:

- raw LAS text
- Arrow types
- Parquet types
- package file internals

The architecture is intentionally separated into:

1. canonical domain model
2. runtime data-access layer
3. storage and package layer

These layers must not collapse into a single public representation.

## Why

- LAS semantics are the product surface, not the storage format
- the same public API should work whether data came from raw LAS or an optimized package
- storage/runtime details need to remain replaceable
- Arrow/Parquet and package layout should be able to evolve without changing the meaning of the domain model

## Consequences

- package readers and raw LAS readers must resolve to the same canonical concepts
- storage-specific types remain internal or secondary
- future schema changes should preserve domain semantics first
- DTOs and package schemas must not replace the domain model as the source of truth
