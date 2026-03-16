# lithos

Status: Early development. The canonical LAS model and optimized package schema are still evolving.

`lithos` is a Rust-first LAS SDK for desktop and local-tooling workflows. It reads raw LAS files into a canonical domain model, exposes an application-facing table abstraction for curve data, and can persist an optimized local package split into `metadata.json` and `curves.parquet`.

The project is designed primarily for Rust desktop applications such as Tauri backends and local data tooling, while remaining interoperable with common data ecosystems.

## Why Lithos Exists

The LAS ecosystem currently has:

- parsers such as Python `lasio`
- proprietary vendor implementations
- limited modern developer tooling

What it does not have widely is:

- a Rust-native LAS SDK
- a canonical domain model for LAS
- a clean separation between LAS semantics and storage formats
- an optimized packaging format suitable for local analytics and ML workflows

Lithos aims to fill that gap with:

- a robust LAS parser
- a canonical LAS domain model
- an app-friendly runtime table abstraction
- an optimized local package format
- a Rust-native SDK suitable for desktop applications

The design philosophy is domain-first, meaning the API reflects LAS concepts rather than storage formats.

## Quick Example

```rust
use lithos_las::read_path;

fn main() -> Result<(), lithos_las::LasError> {
    let las = read_path("examples/sample.las", &Default::default())?;

    println!("Well name: {:?}", las.well_info().well);
    println!("Curves: {:?}", las.curve_names());

    let dt = las.curve("DT")?;
    println!("DT samples: {}", dt.len());

    Ok(())
}
```

This example demonstrates:

- opening a LAS file
- inspecting metadata
- accessing curve data

The caller does not need to know whether the data originated from a LAS file or an optimized package.

## Project Architecture

Lithos separates LAS semantics, runtime access, and storage formats into distinct layers.

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

This layered architecture allows Lithos to evolve storage and runtime implementations without breaking the domain API.

## Current State

The current implementation focuses on LAS read/model parity for non-v3 LAS files and a staged runtime/package architecture.

Core components:

- workspace crates: `lithos-core`, `lithos-parser`, `lithos-table`, `lithos-package`, `lithos-cli`
- root compatibility crate: `lithos_las`
- canonical domain object: `LasFile`
- typed canonical metadata view: `CanonicalMetadata`, `VersionInfo`, `WellInfo`, `IndexInfo`, `CurveInfo`
- in-memory app/query layer: `CurveTable`
- DTO/query layer for package-backed applications
- optimized package format: `metadata.json + curves.parquet`
- CLI for import and inspection
- local example corpus and parity tests against `lasio` non-v3 behavior

Arrow/Parquet currently exist at the storage boundary. The runtime API remains domain-first rather than Arrow-first.

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

Current workspace wiring:

```text
root compatibility crate: lithos_las
  -> lithos-core
  -> lithos-parser
  -> lithos-table
  -> lithos-package
  -> lithos-cli
```

Key behaviors implemented:

- LAS 1.2, 2.0, and 2.1 read support for the tested non-v3 corpus
- wrapped and unwrapped parsing
- null-policy handling and encoding support
- mnemonic normalization and duplicate suffixing
- structured section and header access
- typed canonical metadata derivation and explicit package metadata schema versioning
- package-backed edit/save and save-as flows
- curve window DTOs for frontend-safe access
- package write/read round-trip
- mixed numeric/text curve column support

## Package Format

Lithos can persist LAS data into an optimized local package.

Example layout:

```text
well_123.laspkg/
  metadata.json
  curves.parquet
```

`metadata.json` contains:

- version, well, parameter, and other LAS metadata
- canonical metadata and curve descriptors
- provenance and import information
- diagnostics and package/schema version metadata

`curves.parquet` stores the sampled curve matrix. Today it preserves the imported curve mnemonics, including the index curve name.

Illustrative shape:

```text
DEPT      DT      RHOB    NPHI
1670.000  123.45  2550.0  0.45
1669.875  123.45  2550.0  0.45
1669.750  123.45  2550.0  0.45
```

This keeps metadata and sampled data cleanly separated while remaining easy to inspect from other tools.

## CurveTable

`CurveTable` is the application-facing abstraction for sampled curve data.

Capabilities include:

- column access by mnemonic
- row slicing
- table descriptors for storage kinds
- package window/query support through DTOs

Internally this abstraction may evolve toward a more Arrow-backed runtime, but the public API remains storage-agnostic.

## Interoperability

Because curve samples are stored in Parquet, Lithos packages can interoperate with common data tools.

Example workflows:

Python / Pandas:

```python
import pandas as pd

df = pd.read_parquet("curves.parquet")
```

DuckDB:

```sql
SELECT DT, RHOB FROM 'curves.parquet'
```

Polars:

```python
import polars as pl

df = pl.read_parquet("curves.parquet")
```

This lets Lithos packages fit naturally into analytics pipelines and ML workflows while keeping LAS semantics intact in the SDK layer.

## CLI

```bash
cargo run -- import <input.las> <package_dir>
cargo run -- inspect-file <input.las>
cargo run -- summary <package_dir>
cargo run -- list-curves <package_dir>
```

The CLI currently provides basic import, inspection, and package introspection functionality.

## Design Docs

Architecture and design decisions are documented in:

- `docs/architecture/README.md`
- `docs/architecture/ADR-0001-canonical-las-model.md`
- `docs/architecture/ADR-0002-staged-arrow-parquet-adoption.md`
- `docs/architecture/ADR-0003-package-format-metadata-json-plus-curves-parquet.md`
- `docs/architecture/ADR-0004-lasio-parity-and-scope.md`
- `docs/architecture/ADR-0005-staged-workspace-split-and-table-boundary.md`
- `docs/lasio_non_v3_parity.md`
- `las_canonical_schema.md`
- `lasio-basic-example.md`

`las_canonical_schema.md` remains the target-state canonical schema note for the later tighter Arrow/Parquet phase. It is not a claim of full current conformance.

## Design Philosophy

Lithos follows several core principles:

- domain-first APIs rather than storage-format APIs
- storage formats are implementation details
- simple, inspectable artifacts rather than opaque binaries
- strong Rust ergonomics and safety
- clear separation between parsing, runtime models, DTOs, and packaging

## Comparison to Other Tools

| Tool | Language | Scope |
| --- | --- | --- |
| `lasio` | Python | LAS parser and utilities |
| `lithos` | Rust | LAS SDK with canonical model, DTO boundary, and packaging |
| Vendor software | Various | Integrated interpretation platforms |

Lithos focuses on developer-facing infrastructure rather than end-user interpretation tools.

## Non-Goals

Lithos currently does not aim to be:

- a full geoscience interpretation platform
- a GUI visualization system
- a cloud data platform
- a replacement for Python LAS analytics libraries

Instead, Lithos focuses on:

- robust LAS parsing
- canonical LAS domain modeling
- application-friendly runtime APIs
- efficient local data packaging

## Roadmap

### Done

- canonical `LasFile` model and tolerant LAS parser
- typed canonical metadata layer and explicit package metadata contract
- `CurveTable` runtime table abstraction
- package-backed edit/save primitives
- DTO layer for summaries, metadata, curve catalog, and windowed reads
- `metadata.json + curves.parquet` package format
- non-v3 `lasio` parity coverage
- package round-trip tests including mixed-type columns

### Next

- tighten the canonical metadata model
- stabilize DTO/query contracts for Tauri and other desktop frontends
- improve validation and diagnostics around edits and repair decisions
- make package overwrite semantics and validation rules more explicit

### After That

- move toward fuller canonical-schema alignment
- introduce canonical index naming rules and stricter package guarantees
- deepen Arrow/Parquet only after canonical metadata and query contracts stabilize
- add optional computation adapters such as `ndarray` without making them core dependencies

### Later

- LAS 3 support
- larger local-library and indexing workflows
- controlled export and round-trip support
- broader subsurface abstractions beyond LAS

## Contributing

Lithos is in early development and contributions are welcome.

Areas likely to benefit from contributions:

- LAS corpus testing
- parser robustness improvements
- metadata validation rules
- CLI tooling
- documentation improvements
- future LAS 3 support

Before contributing large changes, open an issue first to discuss direction. Lithos uses architecture decision records to document major design decisions.

## Repository Layout

```text
src/                    root compatibility crate and thin CLI entrypoint
crates/                 workspace crates for core, parser, table, package, and CLI
docs/                   architecture notes and ADRs
examples/               LAS example corpus
tests/                  parity and package/editing integration tests
las_canonical_schema.md target-state canonical schema note
lasio-basic-example.md  usage examples
```

## Verification

```bash
cargo fmt --check
cargo test
```
