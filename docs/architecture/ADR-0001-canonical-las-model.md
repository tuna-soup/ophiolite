# ADR-0001: Canonical LAS Model

## Status

Accepted

## Decision

`lithos` exposes a domain-first LAS model rather than a storage-first API.

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

## Why

- LAS semantics are the product surface, not the storage format
- the same public API should work whether data came from raw LAS or an optimized package
- storage/runtime details need to remain replaceable

## Consequences

- package readers and raw LAS readers must resolve to the same canonical concepts
- storage-specific types remain internal or secondary
- future schema changes should preserve domain semantics first
