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
  -> DTO/query layer
  -> optional package:
       metadata.json
       curves.parquet
```

Current properties:

- the repo now uses a staged workspace split with `lithos-core`, `lithos-parser`, `lithos-table`, `lithos-package`, and `lithos-cli`
- the root `lithos_las` crate is a compatibility facade over those workspace crates
- `LasFile` is the canonical public domain object
- `CurveTable` is the app-facing in-memory table abstraction
- DTOs are the intended frontend/backend transfer boundary
- package storage uses `metadata.json + curves.parquet`
- Arrow/Parquet are internal storage/package details, not the dominant public API
- non-v3 `lasio` read/model parity is the main compatibility baseline

## Project Architecture

```text
                Applications
      (Tauri UI, CLI tools, pipelines)

                 Lithos SDK API
         (LasFile, DTOs, package access)

              Canonical Domain Model
                 (LAS semantics)

        Runtime Data Representation Layer
         (CurveTable and windowed access)

              Storage / Interchange
      LAS files | metadata.json + curves.parquet
```

This separation is intentional: the SDK owns LAS semantics, DTOs own transfer shapes, and storage formats remain replaceable implementation details.

## Workspace Layout

```text
root compatibility crate: lithos_las
  -> lithos-core
  -> lithos-parser
  -> lithos-table
  -> lithos-package
  -> lithos-cli
```

Current staged compromise:

- parser, package, and CLI are split into their own crates
- the runtime table boundary has its own crate, but `CurveTable` still originates from the core layer in this phase to preserve the current `LasFile::data()` API
- Arrow/Parquet conversion now lives in the package crate rather than the runtime table type

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
- `ADR-0005-staged-workspace-split-and-table-boundary.md`

## Related Docs

- `../lasio_non_v3_parity.md`
- `../../las_canonical_schema.md`
- `../../lasio-basic-example.md`

`../../las_canonical_schema.md` remains a target-state schema note. It should not be deleted until its remaining unique content is either implemented or moved into durable ADRs.
