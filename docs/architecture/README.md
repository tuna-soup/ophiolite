# Architecture Overview

This folder captures the durable architectural decisions for `lithos`.

The goal is that someone new to the codebase can read this directory and understand:

- what the system does today
- which design choices are intentional
- which documents describe current behavior versus target-state behavior
- where Arrow/Parquet fit in the roadmap

## System Today

```text
LAS file
  -> parser/importer
  -> LasFile
  -> CurveTable
  -> optional package:
       metadata.json
       curves.parquet
```

Current properties:

- `LasFile` is the canonical public domain object
- `CurveTable` is the app-facing in-memory table abstraction
- package storage uses `metadata.json + curves.parquet`
- Arrow/Parquet are internal storage/package details, not the dominant public API
- non-v3 `lasio` read/model parity is the main compatibility baseline

## Current vs Target

| Area | Current implementation | Target direction |
| --- | --- | --- |
| Domain model | `LasFile` with section-oriented metadata | tighter typed canonical metadata model |
| In-memory samples | `CurveTable` backed by current in-memory values | potentially more formal Arrow-backed runtime contract later |
| Package format | `metadata.json + curves.parquet` with mixed-column preservation | stricter canonical schema and package guarantees |
| Canonical schema | partially aligned | `las_canonical_schema.md` is the target-state reference |
| Frontend/backend boundary | CLI and Rust API | explicit Tauri DTO/query contract |

## Roadmap Placement of Arrow/Parquet

Arrow/Parquet is already in use for package persistence, but it is not yet the full canonical runtime model.

Before deepening Arrow/Parquet integration, `lithos` should first stabilize:

1. canonical metadata shapes
2. package schema/version guarantees
3. Tauri/backend DTOs and query semantics
4. nullability/index/curve descriptor rules

Only after those are stable should the project tighten runtime/package behavior toward the full canonical schema target.

## Decision Records

- `ADR-0001-canonical-las-model.md`
- `ADR-0002-staged-arrow-parquet-adoption.md`
- `ADR-0003-package-format-metadata-json-plus-curves-parquet.md`
- `ADR-0004-lasio-parity-and-scope.md`

## Related Docs

- `../lasio_non_v3_parity.md`
- `../../las_canonical_schema.md`
- `../../lasio-basic-example.md`
