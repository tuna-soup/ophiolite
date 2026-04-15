# ophiolite

Status: Early development. The public product boundary is now clear even where implementation is still catching up.

`ophiolite` is the platform repo for the Ophiolite stack. It owns canonical subsurface contracts, local-first runtime primitives, typed asset semantics, and the embeddable `Ophiolite Charts` SDK used by applications.

It began from a strong LAS/log foundation, but it is no longer a LAS-focused project. Today it also includes a local-first multi-asset project/catalog layer, shared seismic runtime paths, chart SDK packages, and frontend-safe DTO boundaries used by applications and embedders.

The active platform layout is:

- `crates/` for Rust core/runtime crates
- `contracts/` for shared schemas and generated TypeScript contracts
- `charts/` for `Ophiolite Charts`
- `python/` for the thin `ophiolite-automation` wrapper over platform CLI operations
- `apps/ophiolite-docs` for the public docs site at `https://ophiolite.dev`

The intended product story is:

- `Ophiolite` is the platform
- `Ophiolite Charts` is the embeddable charts SDK within the platform
- applications can build workflow products on top of the platform

Today, Ophiolite can:

- parse and model raw LAS log data
- import typed non-log wellbore datasets from structured files
- persist optimized local asset packages
- organize multiple linked assets under one `OphioliteProject`
- discover and run typed compute/UDF functions against eligible log and structured wellbore data
- expose app-facing query and editing surfaces for desktop workflows
- resolve focused frontend DTO/query payloads such as well-panel, survey-map, section, and gather views without exposing raw catalog internals
- generate TypeScript contracts for frontend-safe DTOs under `contracts/ts/ophiolite-contracts`
- run seismic processing pipelines through the shared seismic runtime, including section preview, derived-volume materialization, and persisted processing lineage
- classify seismic datasets with explicit stacking/layout metadata so post-stack and future prestack/gather flows can share one canonical foundation without forcing a product layer into one runtime shape
- ingest and persist phase-one prestack offset surveys through a dedicated gather-native `tbgath` runtime path, with gather preview and gather-processing materialization separated from the post-stack `tbvol` section path
- expose phase-one prestack analysis as separate request/response APIs, including offset-gather velocity scan / semblance evaluation and optional first-pass velocity autopick that do not masquerade as materializing operators
- store canonical seismic trace-data assets in the project/catalog layer while keeping the current volume-oriented runtime paths available as compatibility surfaces

At the product boundary, Ophiolite owns canonical subsurface meaning and runtime primitives rather than commercial workflow orchestration or chart rendering.

Today the shared seismic operator family is intentionally narrow:

- current live trace-local operators are `amplitude_scalar`, `trace_rms_normalize`, `agc_rms`, `phase_rotation`, `lowpass_filter`, `highpass_filter`, `bandpass_filter`, and same-geometry `volume_arithmetic`
- geometry-changing post-stack derivation is a separate live family, currently terminal `subvolume crop`, rather than something forced into the trace-local executor
- section/gather-matrix operators such as `f-k` filtering are a separate future scope, not something to force into the trace-local executor
- inverse-wavelet operators such as deconvolution are also a separate future scope with different assumptions and validation needs

The project is designed primarily for Rust desktop applications such as Tauri backends and local data tooling, while remaining interoperable with common data ecosystems.

## Why Ophiolite Exists

Ophiolite started from a real gap in the LAS ecosystem, but the underlying need is broader than LAS alone.

Real subsurface applications need a coherent way to work with:

- well and wellbore identities
- log curves
- trajectories
- tops
- pressure observations
- drilling observations
- seismic volumes, sections, and trace sets
- survey maps, time-depth transforms, and related derived/display workflows
- related provenance, diagnostics, and package/query workflows

Ophiolite therefore aims to provide:

- canonical subsurface contracts and DTO meaning
- a robust LAS parser and canonical log-domain model where LAS is the source artifact
- typed non-log asset families for other wellbore datasets
- a shared seismic core for canonical seismic descriptors, app-boundary section/gather/trace models, SEG-Y IO, and runtime/store execution
- an additive seismic trace-data descriptor layer that distinguishes stacking state, organization, layout, and gather-axis semantics before product/runtime layers decide what they support
- separate runtime storage/query paths for post-stack volumes and prestack gathers so future apps do not need to force both through one fake common shape
- canonical map, CRS, well-overlay, and time-depth boundaries that products and embedders can share
- an app-friendly runtime/query abstraction
- optimized local single-asset package formats
- a local-first project/catalog layer for assembling multiple linked assets coherently
- a typed compute layer for derived assets and domain-aware transforms across logs, trajectory, tops, pressure, and drilling
- a Rust-native core suitable for desktop subsurface applications

The design philosophy is domain-first: APIs should reflect subsurface concepts and workflows rather than raw storage formats.

## Quick Examples

Log/LAS access:

```rust
use ophiolite::read_path;

fn main() -> Result<(), ophiolite::LasError> {
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

Multi-asset project access:

```rust
use ophiolite::OphioliteProject;

fn main() -> Result<(), ophiolite::LasError> {
    let project = OphioliteProject::open("examples/my-study")?;
    let wells = project.list_wells()?;

    println!("Wells: {}", wells.len());
    Ok(())
}
```

## Project Architecture

Ophiolite separates ingest, canonical subsurface meaning, runtime/query access, and storage formats into distinct layers.

```text
                 Applications
       (product shells, internal harnesses, CLI tools)

                  Ophiolite SDK API
   (OphioliteProject, PackageSession, LasFile, DTOs)

             Canonical Domain + Contract Model
   (wells, wellbores, logs, trajectory, tops, pressure,
     drilling, seismic, map, time-depth, provenance)

          Runtime / Query / Editing Layer
   (CurveTable, typed reads, runtime stores, sessions)

            Storage / Interchange Layer
    LAS | SEG-Y | CSV | package formats | runtime stores
```

This layered architecture allows Ophiolite to evolve storage and runtime implementations without breaking the domain API.

## Current State

The current implementation still has its deepest maturity in the LAS/log slice, but the core is no longer only log-centric. It now combines:

- a strong LAS/log import and package/edit path
- a local-first multi-well project/catalog layer
- typed non-log asset packages with read/query and bounded in-family edit APIs
- a typed compute layer for derived assets, with the richest surface currently in logs and family-specific transforms for structured assets
- canonical contract families for sections, gathers, survey maps, wells, and related overlays
- shared seismic runtime/store paths and app-boundary DTO projection
- an internal desktop app that exercises those assets together

Core components:

- workspace crates: `ophiolite-core`, `ophiolite-parser`, `ophiolite-table`, `ophiolite-package`, `ophiolite-project`, `ophiolite-ingest`, `ophiolite-compute`, `ophiolite-seismic`, `ophiolite-seismic-io`, `ophiolite-seismic-runtime`, `ophiolite-cli`
- root compatibility crate: `ophiolite`
- thin Python automation package: `python/ophiolite_automation`
- canonical domain object: `LasFile`
- explicit editable package session model: `PackageSession`
- local multi-well project/catalog root: `OphioliteProject`
- typed multi-well asset families: log, trajectory, tops, pressure observations, drilling observations, and seismic trace data
- shared seismic descriptors, SEG-Y IO, runtime/store backends, and contract DTO families for volumes, sections, gathers, maps, and related views
- Tauri/backend adapter surface: `PackageBackend`
- Tauri-ready shared backend state wrapper: `PackageBackendState`
- app-boundary command service: `PackageCommandService`
- internal Tauri capability harness: `apps/ophiolite-harness`
- the internal Tauri capability harness now prefers depth-range curve queries for the workspace curve inspector and falls back to row windows when no valid depth range is available
- typed canonical metadata view: `CanonicalMetadata`, `VersionInfo`, `WellInfo`, `IndexInfo`, `CurveInfo`
- explicit grouped package metadata schema: `package`, `document`, `storage`, `raw`, and `diagnostics`
- in-memory app/query layer: `CurveTable`
- DTO/query layer for package-backed applications and embedders
- optimized package format: `metadata.json + curves.parquet`
- SQLite-backed project catalog for well, wellbore, collection, and asset discovery
- Parquet-backed project-managed asset packages for non-log structured wellbore data
- typed compute registry with semantic eligibility and derived sibling assets across supported families
- CLI for developer and validation workflows around the core
- local example corpus and parity tests against `lasio` non-v3 behavior

Arrow/Parquet currently exist at the storage boundary and now also back backend-session window reads internally. The runtime API remains domain-first rather than Arrow-first.

## Current Architecture

```text
source artifacts
  -> import and normalization
  -> canonical subsurface contracts + typed asset models
  -> package, catalog, and runtime stores
  -> type-safe compute / derived assets
  -> app/query/edit/display workflows
```

Current workspace wiring:

```text
root compatibility crate: ophiolite
  -> ophiolite-core
  -> ophiolite-parser
  -> ophiolite-table
  -> ophiolite-package
  -> ophiolite-project
  -> ophiolite-ingest
  -> ophiolite-compute
  -> ophiolite-seismic
  -> ophiolite-seismic-io
  -> ophiolite-seismic-runtime
  -> ophiolite-cli
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
- package session dirty-state, identity, and overwrite-oriented persistence
- session summaries and session-backed DTOs report the current bound package root
- Tauri-oriented backend session/query adapter
- separate command-boundary transport service with structured command errors
- structured diagnostic issues for package, edit, and save validation flows
- metadata-only package opens without loading sample data
- backend session open avoids eager sample materialization for metadata, catalog, and window read paths
- session/query APIs now support both row-window reads and depth-range reads for projected curve access
- metadata-only lazy package edits and save/save-as flows
- first curve edits materialize directly from lazy backend session state rather than reopening through the eager SDK path
- compute/UDF discovery is type-safe against semantic curve classifications rather than loose mnemonic matches
- compute runs target typed assets, create derived sibling assets in the same family, and persist execution provenance on the derived asset manifest
- trajectory, tops, pressure, and drilling assets now support project-scoped typed edit sessions with row add/update/delete and explicit save
- package write/read round-trip
- mixed numeric/text curve column support

## Asset Packages

Ophiolite persists asset data into optimized local single-asset packages.

Example layout:

```text
well_123.laspkg/
  metadata.json
  curves.parquet
```

For log assets, `metadata.json` contains:

- package identity and metadata schema version
- document summary, provenance, and encoding
- canonical metadata and storage-facing column descriptors
- raw preserved LAS sections
- diagnostics and import issues

`curves.parquet` stores the sampled curve matrix. Today it preserves the imported curve mnemonics, including the index curve name.
It is now written with an explicit depth-query-oriented profile:

- `SNAPPY` compression
- page statistics and offset index enabled
- bounded row-group and data-page row counts
- sorting metadata on the monotonic numeric index column when available

Illustrative shape:

```text
DEPT      DT      RHOB    NPHI
1670.000  123.45  2550.0  0.45
1669.875  123.45  2550.0  0.45
1669.750  123.45  2550.0  0.45
```

This keeps metadata and sampled data cleanly separated while remaining easy to inspect from other tools.

For non-log wellbore assets, Ophiolite uses the same general pattern but with a typed `asset_manifest.json`, `metadata.json`, and `data.parquet` inside a project-managed asset package.

## OphioliteProject

Ophiolite now also has a local-first project/catalog layer for multi-well organization.

Illustrative shape:

```text
my-study/
  ophiolite-project.json
  catalog.sqlite
  assets/
    logs/
      asset_123.laspkg/
        metadata.json
        curves.parquet
        asset_manifest.json
```

The current rule is:

- the catalog is for discovery and relationships
- the package is the authoritative storage unit for the asset itself

`OphioliteProject` currently provides:

- project creation/open
- well and wellbore discovery
- asset-collection grouping
- log-asset import from LAS into project-managed packages
- structured non-log asset import from CSV into project-managed packages for:
  - trajectory
  - tops
  - pressure observations
  - drilling observations
- stable logical asset identity plus per-import storage identity
- typed read/query APIs for those non-log asset families
- cross-asset depth-range discovery for one wellbore
- project-facing summary APIs for project, well, wellbore, collection, and asset overviews
- focused frontend DTO/query resolution for well-panel-oriented non-seismic well data
- project-scoped structured edit sessions for trajectory, tops, pressure observations, and drilling observations
- synthetic multi-asset project fixture generation for testing and manual inspection

The first multi-well slice is still log-first in editing maturity, but it is no longer read-only outside logs. Structured assets now support bounded in-family row/field editing with explicit save and overwrite-oriented save semantics for the active asset package.

Those structured saves are revisioned too:

- every successful structured save creates a new immutable asset revision
- the active asset package path remains the stable current head
- historical structured revisions are stored in a hidden project-local revision store
- the hidden revision store is canonical; the visible asset/package path is a materialized current-head view
- each revision records a typed machine diff plus a readable change summary such as row adds/removes/updates and extent changes

For test and app-validation workflows, Ophiolite can also generate a coherent synthetic project fixture:

```text
synthetic_well_project/
  ophiolite-project.json
  catalog.sqlite
  sources/
    logs/synthetic_well.las
    trajectory/synthetic_trajectory.csv
    tops/synthetic_tops.csv
    pressure/synthetic_pressure.csv
    drilling/synthetic_drilling.csv
  assets/
    ...
```

The raw files are generated first and then imported through the normal `OphioliteProject` APIs, so the fixture validates the real import pipeline rather than bypassing it.

## Typed Compute

Ophiolite now has a typed compute layer in `ophiolite-compute`.

Current compute properties:

- functions are exposed through a typed registry rather than ad hoc curve scripts
- eligibility is driven by semantic curve types such as `GammaRay`, `BulkDensity`, `Sonic`, and `PVelocity`
- functions only appear as available when the selected log asset actually contains compatible inputs
- curve semantics are persisted on log asset manifests, and manual overrides can be stored when classification is uncertain
- compute runs create derived sibling assets under the same `OphioliteProject`
- derived assets record both `derived_from` lineage and a `compute_manifest` describing the execution
- log assets support semantic curve binding plus editable parameter/mnemonic controls
- trajectory, tops, pressure, and drilling assets support family-specific same-shape transforms that persist as derived sibling assets

Current built-in function families:

- generic numeric log transforms:
  - moving average
  - z-score normalization
  - min-max scaling
  - gap flags
- domain-specific petrophysics / rock-physics:
  - `VShale (Linear|Clavier|Steiber)` on gamma ray
  - `Sonic to Vp`
  - `Shear Sonic to Vs`
  - `Acoustic Impedance`
  - `Poisson's Ratio`

This keeps compute attached to typed wellbore-linked assets rather than treating curves as anonymous arrays.

## Runtime Query Surface

`CurveTable` is the application-facing abstraction for sampled curve data.

Capabilities include:

- column access by mnemonic
- row slicing
- table descriptors for storage kinds
- package window/query support through DTOs

Internally this abstraction may evolve toward a more Arrow-backed runtime, but the public API remains storage-agnostic.
Direct `open_package(...)` and public `PackageSession` access remain eager/materialized in the current phase. The new lazy path is currently backend-session-only.

For non-log assets, Ophiolite currently exposes typed project-level read/query APIs rather than one generic table abstraction.

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
- the current head revision token for the saved package snapshot

Current session semantics:

- editable session open reuses one shared backend session per package path by default
- edits are applied to the in-memory session snapshot
- edit requests are atomic at the request level
- `save` writes the current snapshot back to the original package using overwrite-oriented save semantics and advances the package head to a new immutable local revision
- `save_as` writes the current snapshot to a new package path and updates the current session baseline
- session-backed DTOs carry the current bound package root so clients can observe rebinding after `save_as`
- successful save clears dirty-state
- sessions remain alive until explicitly closed in the current desktop MVP
- metadata-only package opens do not require loading `curves.parquet`
- backend session open validates package metadata and parquet footer without eagerly decoding all samples
- backend-session lazy loading is intentionally scoped: session open avoids full sample decode, read-only session queries decode only requested columns and row windows, metadata-only edits and metadata-only save/save-as remain lazy, and curve/sample edits trigger full materialization
- read-only backend session queries now include a first-class depth-range path in addition to row-window reads; depth-range requests are resolved against the monotonic numeric index curve and then executed through the same projected parquet window machinery
- for regular-step depth logs, lazy backend sessions can resolve depth ranges directly from package metadata before falling back to reading the full index column
- session metadata, session summaries, and curve catalogs are served from cached package metadata
- window queries use projected parquet reads and row selection as internal implementation details rather than forcing full frontend materialization
- clean `save` on an unchanged lazy session is a no-op success path that preserves lazy state
- metadata-only dirty lazy sessions can rewrite `metadata.json` and save/save-as without materializing sample data
- the first accepted curve/sample edit and any later save/save-as path that needs the canonical snapshot materializes the eager in-memory `PackageSession`
- first curve/sample materialization is constructed directly from the current lazy session metadata and cached parquet descriptors rather than reopening the package through the eager SDK path
- package saves do not patch Parquet in place; they rewrite the affected payload and record a new immutable local revision snapshot
- local package revision history lives in a hidden `.ophiolite/revisions/` store under the package root while the visible package path remains the stable current head
- package revision records include parent linkage, blob refs, a typed machine diff, and a readable change summary for metadata changes, curve additions/removals, and curve value edits
- revision tokens identify the current saved head; they still do not block saves or act as merge/conflict tokens

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

The same principle applies to `OphioliteProject`: the project/catalog is the canonical container of wells, wellbores, and asset families, while frontend-oriented DTOs such as a resolved well-panel source are query results shaped for one application workflow.

For TypeScript consumers, those DTOs are now exported as a generated package:

- `contracts/ts/ophiolite-contracts`

Regenerate them from the repo root with:

```powershell
.\scripts\generate-ts-contracts.ps1
```

The DTO contract is versioned with a lightweight `dto_contract_version` field. Session-backed metadata, curve-catalog, and curve-window reads now carry explicit session context so desktop clients do not need to infer package/session/revision state from unrelated calls.

## Internal Tauri Harness

`apps/ophiolite-harness` is now a first-party internal Tauri + React desktop shell over the current SDK contract.

It is now project-first rather than package-first:

- `Home`
  - create `OphioliteProject`
  - open existing `OphioliteProject`
  - recent projects
- `Workspace`
  - browse wells
  - browse wellbores, asset collections, and assets
  - inspect selected log, tops, trajectory, pressure, and drilling assets
  - import LAS and structured CSV assets into the selected project
  - run depth-range coverage queries across one wellbore

The harness keeps the SDK concepts explicit:

- project = the multi-well root with `catalog.sqlite` and `assets/`
- asset package = one authoritative storage unit for one asset
- session = live editable SDK state for a selected log package
- workspace = app shell around one open project

Current harness behavior:

- creating or opening a project lands in a project browser rather than a single-package editor
- wells, wellbores, collections, and assets are browsed directly from `OphioliteProject`
- selecting a log asset opens the existing package/session-backed log inspection path
- selecting a non-log asset loads typed trajectory, tops, pressure, or drilling rows through the project query APIs
- selected non-log assets can also be opened into structured edit sessions for typed row/field changes and explicit save
- LAS and structured CSV asset imports happen from the project workspace
- save/save-as remain available for selected log sessions from both the visible toolbar and the native File menu

This means Ophiolite now has a real multi-asset desktop validation surface in-repo. The next gaps are mostly workflow hardening, richer cross-asset viewers, stronger import/reconciliation governance, and broader acceptance coverage.

Harness verification commands:

```powershell
cd apps/ophiolite-harness
bun install
bun run test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
bun tauri dev
```
The command service is intentionally thin and transport-focused. It should not become a second place where domain or save semantics live.
At the app boundary, commands use `CommandResponse<T> = Ok(T) | Err(CommandErrorDto)`.
The public command error kinds are intentionally small and caller-actionable: `OpenFailed`, `ValidationFailed`, `SessionNotFound`, and `Internal`.
Validation reports now carry structured diagnostic issues with code, severity, message, and optional target context.
Save and save-as validation failures report as save-scoped validation rather than generic edit failures.
Post-write validation is bounded: save/save-as verifies enough to confirm the written package is readable and internally coherent, rather than promising an arbitrary full roundtrip guarantee.

## Interoperability

Because Ophiolite stores bulk asset payloads in Parquet-backed packages, project-managed assets can interoperate with common data tools.

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

This lets Ophiolite asset packages fit naturally into analytics pipelines and ML workflows while keeping well-domain semantics intact in the SDK layer.

## CLI

```bash
cargo run -- import <input.las> <package_dir>
cargo run -- inspect-file <input.las>
cargo run -- summary <package_dir>
cargo run -- list-curves <package_dir>
cargo run -- generate-fixture-packages test_data/logs test_data/logs/packages
cargo run -- generate-synthetic-project test_data/projects/synthetic_well_project
```

The CLI currently provides import, inspection, package introspection, and synthetic project-fixture generation functionality.

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
- `docs/architecture/ADR-0008-project-catalog-and-single-asset-packages.md`
- `docs/architecture/ADR-0009-future-ecosystem-boundaries.md`
- `docs/architecture/ADR-0010-typed-compute-and-derived-assets.md`
- `docs/architecture/ADR-0011-structured-asset-edit-sessions.md`
- `docs/architecture/ADR-0012-revisioned-last-save-wins.md`
- `docs/architecture/ADR-0013-shared-subsurface-core-and-seismic-expansion.md`
- `ophiolite_roadmap.md`
- `docs/lasio_non_v3_parity.md`
- `lasio-basic-example.md`

## Design Philosophy

Ophiolite follows several core principles:

- domain-first APIs rather than storage-format APIs
- storage formats are implementation details
- simple, inspectable artifacts rather than opaque binaries
- strong Rust ergonomics and safety
- clear separation between importers, runtime models, DTOs, packaging, and project/catalog concerns

## Comparison to Other Tools

| Tool | Language | Scope |
| --- | --- | --- |
| `lasio` | Python | LAS parser and utilities |
| `ophiolite` | Rust | Local-first subsurface core with canonical contracts, typed assets, runtime primitives, packaging, and project/catalog services |
| Vendor software | Various | Integrated interpretation platforms |

Ophiolite focuses on developer-facing infrastructure rather than end-user interpretation tools.

## Non-Goals

Ophiolite currently does not aim to be:

- a full geoscience interpretation platform
- a GUI visualization system
- a cloud data platform
- a replacement for Python LAS analytics libraries
- a collaborative or multi-user editing system
- duplicate or forked live-session semantics
- a crate with a hard `tauri` dependency at this stage

Instead, Ophiolite focuses on:

- robust source-file import
- canonical and typed well-domain modeling
- application-friendly runtime/query APIs
- efficient local asset packaging
- local-first multi-well project organization

## Roadmap Snapshot

Implemented foundations include:

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
- `OphioliteProject` catalog and typed single-asset packages for logs, trajectory, tops, pressure observations, and drilling observations
- `ophiolite-compute` typed function registry and derived sibling assets
- synthetic multi-asset project-fixture generation for testing and app validation
- non-v3 `lasio` parity coverage
- project-scoped structured asset edit sessions for trajectory, tops, pressure, and drilling data
- package round-trip tests including mixed-type columns

Current next priorities are:

- deepen semantic classification and override workflows for compute eligibility
- expand built-in compute/UDF coverage and derived-asset workflows
- harden richer cross-asset project workflows in the harness
- improve structured asset editing UX and comparison/overlay workflows in the harness
- deepen LAS 3 and broader structured ingest for non-log assets
- keep the command and app boundary thin while the project-first workflow settles
- expand the new seismic core from shared descriptors and boundary DTOs into project-managed seismic asset families without collapsing product workflow logic into the core too early

Later directions include:

- optional sync/distribution layers
- broader ingest and asset-family expansion
- broader subsurface asset families and richer cross-asset application workflows

For the full status-based roadmap, see [ophiolite_roadmap.md](/C:/Users/crooijmanss/dev/ophiolite/ophiolite_roadmap.md).

## Contributing

Ophiolite is in early development and contributions are welcome.

Areas likely to benefit from contributions:

- LAS corpus testing
- parser robustness improvements
- metadata validation rules
- project/catalog and multi-asset workflow hardening
- CLI tooling
- documentation improvements
- future LAS 3 support

Before contributing large changes, open an issue first to discuss direction. Ophiolite uses architecture decision records to document major design decisions.

## Repository Layout

```text
src/                    root compatibility crate and thin CLI entrypoint
crates/                 workspace crates for core, parser, table, package, seismic, and CLI
apps/ophiolite-harness/    internal Tauri + React capability harness
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
