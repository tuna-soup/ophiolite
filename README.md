# lithos

`lithos` is a Rust-first LAS SDK for desktop and local-tooling workflows. It reads raw LAS files into a canonical domain model, exposes an app-facing in-memory table abstraction, and can persist an optimized package split into `metadata.json` and `curves.parquet`.

## Current State

The current implementation is centered on non-v3 LAS read/model parity and a staged package/runtime architecture:

- Canonical domain object: `LasFile`
- In-memory app/query layer: `CurveTable`
- Optimized package format: `metadata.json + curves.parquet`
- CLI for import and inspection
- Local example corpus and parity tests against `lasio` non-v3 behaviors

This means Arrow/Parquet are already present at the package/storage boundary, but the runtime model is still a staged approximation of the longer-term canonical schema. The public API remains domain-first rather than Arrow-first.

## Current Architecture

```text
LAS file
  -> parser/importer
  -> LasFile (canonical domain model)
  -> CurveTable (app-facing in-memory table)
  -> optional package:
       metadata.json
       curves.parquet
```

Key behaviors currently implemented:

- LAS 1.2, 2.0, and 2.1 read support for the tested non-v3 corpus
- wrapped/unwrapped parsing
- null-policy handling and encoding support
- mnemonic normalization/case handling and duplicate suffixing
- structured section/header access
- stacked curve access and curve mutation helpers
- package write/read round-trip, including mixed numeric/text curve columns

## CLI

```bash
cargo run -- import <input.las> <package_dir>
cargo run -- inspect-file <input.las>
cargo run -- summary <package_dir>
cargo run -- list-curves <package_dir>
```

## Design Docs

- `docs/architecture/README.md`: architecture overview and current-vs-target summary
- `docs/architecture/ADR-0001-canonical-las-model.md`
- `docs/architecture/ADR-0002-staged-arrow-parquet-adoption.md`
- `docs/architecture/ADR-0003-package-format-metadata-json-plus-curves-parquet.md`
- `docs/architecture/ADR-0004-lasio-parity-and-scope.md`
- `docs/lasio_non_v3_parity.md`
- `las_canonical_schema.md`: target-state canonical schema for the later Arrow/Parquet phase
- `lasio-basic-example.md`: current Rust usage examples

## Roadmap

### Done

- Canonical `LasFile` model and tolerant LAS parser
- `CurveTable` as the app-facing in-memory table abstraction
- `metadata.json + curves.parquet` package support
- non-v3 `lasio` parity coverage for read/model behavior
- package round-trip tests including mixed-type columns

### Next

- Stabilize the canonical metadata model:
  - promote the current section-centric structures toward a minimal typed canonical model
  - define explicit package schema/version guarantees
  - tighten curve/index descriptors and nullability metadata
- Shape the backend/query API for Tauri:
  - metadata DTOs
  - curve window/query DTOs
  - metadata-only open paths
- Improve validation and diagnostics:
  - more explicit repair decisions
  - more structured issue reporting

### After That

- Move toward fuller canonical-schema alignment:
  - canonical `index` naming rules
  - stricter package schema
  - clearer separation between domain metadata and sample-table representation
- Expand Arrow/Parquet from package plumbing into a more formal runtime contract only after the canonical metadata model and Tauri query shapes are stable
- Add optional computation adapters if needed, such as `ndarray`, without making them core SDK dependencies

### Later

- LAS 3 support
- larger local-library/indexing workflows
- controlled export/round-trip support
- broader subsurface abstractions beyond LAS

## Verification

```bash
cargo fmt --check
cargo test
```
