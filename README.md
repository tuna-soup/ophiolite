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
- explicit editable package session model: `PackageSession`
- Tauri/backend adapter surface: `PackageBackend`
- Tauri-ready shared backend state wrapper: `PackageBackendState`
- app-boundary command service: `PackageCommandService`
- internal Tauri capability harness: `apps/lithos-harness`
- typed canonical metadata view: `CanonicalMetadata`, `VersionInfo`, `WellInfo`, `IndexInfo`, `CurveInfo`
- explicit grouped package metadata schema: `package`, `document`, `storage`, `raw`, and `diagnostics`
- in-memory app/query layer: `CurveTable`
- DTO/query layer for package-backed applications
- optimized package format: `metadata.json + curves.parquet`
- CLI for import and inspection
- local example corpus and parity tests against `lasio` non-v3 behavior

Arrow/Parquet currently exist at the storage boundary and now also back backend-session window reads internally. The runtime API remains domain-first rather than Arrow-first.

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
- versioned DTO/query contract for frontend-safe access
- package session dirty-state, identity, and optimistic save-conflict detection
- session summaries and session-backed DTOs report the current bound package root
- Tauri-oriented backend session/query adapter
- separate command-boundary transport service with structured command errors
- structured diagnostic issues for package, edit, and save validation flows
- metadata-only package opens without loading sample data
- backend session open avoids eager sample materialization for metadata, catalog, and window read paths
- metadata-only lazy package edits and save/save-as flows
- first curve edits materialize directly from lazy backend session state rather than reopening through the eager SDK path
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

- package identity and metadata schema version
- document summary, provenance, and encoding
- canonical metadata and storage-facing column descriptors
- raw preserved LAS sections
- diagnostics and import issues

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
Direct `open_package(...)` and public `PackageSession` access remain eager/materialized in the current phase. The new lazy path is currently backend-session-only.

## Package Sessions and DTOs

Package-backed editing is modeled explicitly through `PackageSession`.

The current backend contract distinguishes:

- read-only flows: package summary, metadata views, curve catalog, and windowed curve reads
- editable flows: metadata edits, curve edits, dirty-state inspection, save, and save-as
- app-boundary command groups:
  - inspect commands do not require a session
  - session commands require or produce a valid `SessionId`
  - edit/persist commands operate on an existing session

`PackageBackend` provides the current Tauri-oriented backend adapter on top of shared package sessions.
`PackageBackendState` wraps it in shared mutable state used by the internal Tauri harness and suitable for further Tauri command registration.
`PackageCommandService` is the separate app-boundary transport layer that maps command requests into structured success/error envelopes.

`PackageSession` owns:

- package identity
- session identity
- the current in-memory `LasFile` snapshot
- dirty-state
- the current revision token used for optimistic save conflict checks

Current session semantics:

- editable session open reuses one shared backend session per package path by default
- edits are applied to the in-memory session snapshot
- edit requests are atomic at the request level
- `save` writes the current snapshot back to the original package if the revision still matches
- `save_as` writes the current snapshot to a new package path and updates the current session baseline
- session-backed DTOs carry the current bound package root so clients can observe rebinding after `save_as`
- successful save clears dirty-state
- sessions remain alive until explicitly closed in the current desktop MVP
- metadata-only package opens do not require loading `curves.parquet`
- backend session open validates package metadata and parquet footer without eagerly decoding all samples
- backend-session lazy loading is intentionally scoped: session open avoids full sample decode, read-only session queries decode only requested columns and row windows, metadata-only edits and metadata-only save/save-as remain lazy, and curve/sample edits trigger full materialization
- session metadata, session summaries, and curve catalogs are served from cached package metadata
- window queries use projected parquet reads and row selection as internal implementation details rather than forcing full frontend materialization
- clean `save` on an unchanged lazy session is a no-op success path that preserves lazy state
- metadata-only dirty lazy sessions can rewrite `metadata.json` and save/save-as without materializing sample data
- the first accepted curve/sample edit and any later save/save-as path that needs the canonical snapshot materializes the eager in-memory `PackageSession`
- first curve/sample materialization is constructed directly from the current lazy session metadata and cached parquet descriptors rather than reopening the package through the eager SDK path
- revision tokens are for persistence conflict detection against the currently bound package baseline/root, not collaborative synchronization

Session invariants:

- same package path returns the same shared session while it remains open
- close invalidates the current `SessionId`
- reopen after close returns a new `SessionId`
- `Lazy` and materialized backend-session states preserve the same session identity and bound package root semantics
- `save` preserves session identity and package root on success
- `save_as` keeps the same session identity, but that session is now editing the newly written package
- once a backend session materializes, it does not transition back to lazy in the current phase
- failed `save` and `save_as` leave the session open with the same session id, dirty-state, package root, and in-memory document snapshot
- failed materialization leaves the session open with the same session id, dirty-state, package root, and no partial mutation applied
- materialization preserves all accepted lazy metadata edits already present in the session and must not reconstruct from stale on-disk metadata

Backend-session parquet metadata caches are session-local in the current phase. They are reused across repeated reads within one open session and dropped when that session is closed.

DTOs are boundary and transport shapes. They are not the canonical domain model. `LasFile` remains the authoritative in-memory LAS representation inside the backend.

The DTO contract is versioned with a lightweight `dto_contract_version` field. Session-backed metadata, curve-catalog, and curve-window reads now carry explicit session context so desktop clients do not need to infer package/session/revision state from unrelated calls.

## Internal Tauri Harness

`apps/lithos-harness` is now a first-party internal Tauri + React capability harness over the current SDK contract. It mounts thin Tauri handlers over `PackageCommandService` and is intended to exercise:

- package inspection and validation
- session lifecycle
- curve catalog and windowed reads
- metadata edits and curve edits
- save/save-as flows
- structured validation and conflict rendering

This means Lithos is no longer far from a usable test desktop app. The backend contract and a thin desktop shell already exist in-repo. The main remaining gap is not SDK wiring; it is frontend install/build automation and then iterating on UI polish, acceptance coverage, and product-specific workflow design.

Harness verification commands:

```powershell
cd apps/lithos-harness
bun install
bun run test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
bun tauri dev
```
The command service is intentionally thin and transport-focused. It should not become a second place where domain or save semantics live.
At the app boundary, commands use `CommandResponse<T> = Ok(T) | Err(CommandErrorDto)`.
The public command error kinds are intentionally small and caller-actionable: `OpenFailed`, `ValidationFailed`, `SaveConflict`, `SessionNotFound`, and `Internal`.
Validation reports now carry structured diagnostic issues with code, severity, message, and optional target context.
Save and save-as validation failures report as save-scoped validation rather than generic edit failures.
Post-write validation is bounded: save/save-as verifies enough to confirm the written package is readable and internally coherent, rather than promising an arbitrary full roundtrip guarantee.

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
- `docs/architecture/ADR-0006-package-session-and-dto-boundary.md`
- `docs/architecture/ADR-0007-canonical-schema-target.md`
- `docs/lasio_non_v3_parity.md`
- `lasio-basic-example.md`

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
- a collaborative or multi-user editing system
- duplicate or forked live-session semantics
- a crate with a hard `tauri` dependency at this stage

Instead, Lithos focuses on:

- robust LAS parsing
- canonical LAS domain modeling
- application-friendly runtime APIs
- efficient local data packaging

## Roadmap

### Done

- canonical `LasFile` model and tolerant LAS parser
- typed canonical metadata layer and explicit package metadata contract
- grouped package metadata schema with compatibility reads for the legacy flat shape
- `CurveTable` runtime table abstraction
- package-backed edit/save primitives
- explicit package session model with dirty-state and revision tracking
- `PackageBackend` adapter for Tauri-style inspection and edit flows
- `PackageBackendState` wrapper for command-style shared backend state
- `PackageCommandService` app-boundary transport service with structured command errors
- DTO layer for summaries, metadata, curve catalog, windowed reads, and edit flows
- explicit session-context DTOs for session metadata, curve catalogs, and curve-window queries
- structured diagnostic DTOs for package, edit, and save validation
- backend-only lazy session reads on top of Arrow/Parquet projection and row selection
- lazy metadata-only backend edits and metadata-only save/save-as paths
- direct first curve-edit materialization from lazy backend session state
- internal first-party Tauri capability harness for exercising SDK flows end to end
- `metadata.json + curves.parquet` package format
- non-v3 `lasio` parity coverage
- package round-trip tests including mixed-type columns

### Next

- deepen validation coverage and diagnostic rules now that structured reports exist
- keep the command service thin and transport-focused while the app boundary settles
- extend lazy backend-session reads beyond metadata-only flows only where they do not complicate sample-edit semantics or stale-session correctness
- keep consolidating architecture guidance under `docs/architecture/` rather than root-level notes

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
apps/lithos-harness/    internal Tauri + React capability harness
docs/                   architecture notes and ADRs
examples/               LAS example corpus
tests/                  parity and package/editing integration tests
lasio-basic-example.md  usage examples
```

## Verification

```bash
cargo fmt --check
cargo test
```
