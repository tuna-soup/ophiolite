# Architecture Overview

This folder captures the durable architectural decisions for `ophiolite`.

The goal is that someone new to the codebase can read this directory and understand:

- what the system does today
- which design choices are intentional
- which documents describe current behavior versus target-state behavior
- where Arrow/Parquet fit in the roadmap
- how the project/catalog layer fits above single-asset packages
- how LAS/log data relates to the broader multi-asset well-domain model

## System Today

```text
source artifacts
  -> LAS / CSV importers
  -> canonical log + typed asset models
  -> single-asset packages
  -> OphioliteProject catalog
  -> typed compute / derived assets
  -> DTO/query/edit layers
```

Current properties:

- the repo now uses a staged workspace split with `ophiolite-core`, `ophiolite-parser`, `ophiolite-table`, `ophiolite-package`, `ophiolite-seismic`, and `ophiolite-cli`
- the root `ophiolite` crate is a compatibility facade over those workspace crates
- `ophiolite-seismic`, `ophiolite-seismic-io`, and `ophiolite-seismic-runtime` now provide the shared seismic core for canonical descriptors, SEG-Y IO, and runtime/store execution
- `LasFile` is the canonical public domain object for the log/LAS slice
- `PackageSession` is the backend-owned editable package session model
- `OphioliteProject` is the local-first multi-well project/catalog root
- project-managed typed asset families now include log, trajectory, tops, pressure observations, and drilling observations
- `ophiolite-compute` now provides a typed compute/UDF registry across log and structured asset families
- `PackageBackend` and `PackageBackendState` are the current Tauri-ready backend adapter layer
- `PackageCommandService` is the app-boundary transport layer above the shared backend state
- `CurveTable` is the app-facing in-memory table abstraction
- DTOs are the intended frontend/backend transfer boundary
- seismic processing now also has canonical semantic/version envelopes, canonical artifact identity, canonical lineage/package compatibility checks, and a shared inspectable runtime/debug contract
- package storage uses `metadata.json + curves.parquet` for log assets and `metadata.json + data.parquet + asset_manifest.json` for typed non-log assets
- packages remain single-asset storage units; the project catalog organizes them into well-domain relationships
- `metadata.json` now groups package identity, document metadata, storage descriptors, raw preserved sections, and diagnostics explicitly
- Arrow/Parquet are internal storage/package details, and now also power backend-session lazy window reads and depth-range query execution
- non-v3 `lasio` read/model parity is the main compatibility baseline
- seismic CRS identity, native/effective CRS binding, and display-CRS resolution are defined in `ADR-0014-seismic-crs-native-effective-display-boundary.md`
- canonical wellbore geometry, resolved trajectory, well time/depth authored models, and section well overlays are defined in `ADR-0016-canonical-wellbore-geometry-and-resolved-trajectory-boundary.md`, `ADR-0017-well-time-depth-source-assets-authored-models-and-compiled-runtime-output.md`, and `ADR-0018-project-aware-well-on-section-overlay-dtos-and-backend-projection-rules.md`
- chart-kernel direction, rock-physics crossplot DTO guidance, the first point-cloud spike, and the AVO chart-family analysis boundary are defined in `ADR-0019-chart-kernels-rock-physics-crossplot-and-point-cloud-spike.md` and `ADR-0020-avo-chart-family-analysis-contracts-and-initial-rendering-plan.md`
- the domain-first Python SDK surface and advanced namespace rule are defined in `ADR-0028-domain-first-python-sdk-surface-and-advanced-namespaces.md`
- the unified TraceBoost import manager, app-local backend provider registry, and normalized import lifecycle are defined in `ADR-0029-unified-import-manager-and-provider-registry.md`
- the unified operator catalog and seismic first-class operator discovery boundary are defined in `ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md`
- the shared seismic execution planner and bounded local job-service direction are defined in `ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md`
- the processing authority, thin-client migration, and contract-distribution consolidation direction are defined in `ADR-0032-processing-authority-and-thin-client-migration.md`
- the public SDK core versus adapter/application boundary is defined in `ADR-0033-public-sdk-core-and-adapter-boundaries.md`
- canonical processing identity, debug, and compatibility semantics are defined in `ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`
- the machine-readable workspace boundary manifest and shared capability-registry direction are defined in `ADR-0035-boundary-manifest-and-capability-registry.md`
- the `tbvolc` v1 padded-payload storage contract is defined in `ADR-0036-tbvolc-v1-padded-payload-contract.md`
- the current-to-target authority breakdown for processing concerns is tracked in `processing-authority-matrix.md`
- the active implementation plan for canonical processing integration hardening is tracked in `processing-canonical-integration-plan-2026-04.md`
- the current public-core candidate, blocked, and internal package split is tracked in `public-sdk-package-matrix.md`
- the current public-core versioning and support expectations are tracked in `public-sdk-support-policy.md`
- the explicit readable/reusable/rewritable/reject policy for processing lineage and caches is tracked in `processing-lineage-cache-compatibility-policy.md`
- the narrow public-core facade crate is `crates/ophiolite-sdk`, while the root `ophiolite` crate remains an internal compatibility facade
- the shared capability vocabulary crate is `crates/ophiolite-capabilities`, and the initial boundary metadata source of truth now lives under `workspace.metadata.ophiolite.boundaries` in the root `Cargo.toml`
- TraceBoost desktop remains an app-local command boundary over shared/runtime behavior rather than a public platform control surface
- compatibility packages and re-export paths remain valid, but they are explicitly not equivalent to canonical platform ownership
- the TraceBoost desktop command table is now described by `apps/traceboost-demo/desktop-command-boundary.json` and validated by `scripts/validate-traceboost-command-boundary.mjs`

## Layered Architecture

```text
                Applications
      (Tauri UI, CLI tools, pipelines)

                  Ophiolite SDK API
    (OphioliteProject, PackageSession, LasFile, DTOs)

            Canonical Domain + Asset Model
     (wells, wellbores, logs, trajectory, tops,
       pressure, drilling, provenance, diagnostics)

           Runtime / Query / Editing Layer
      (CurveTable, typed reads, package sessions)

             Storage / Interchange Layer
     LAS | CSV | single-asset package conventions
```

This separation is intentional: the SDK owns well-domain semantics, DTOs own transfer shapes, and storage formats remain replaceable implementation details.

The current stack split is also intentional:

- `ophiolite` owns canonical contracts, runtime semantics, public control surfaces, and shared capability vocabulary
- Ophiolite Charts owns reusable embedder-facing chart behavior
- TraceBoost owns first-party workflow composition, desktop shell transport, session UX, and app-local capability activation

That means the TraceBoost desktop/Tauri command boundary is an internal application seam. It may carry canonical contracts, but its command names and transport details are not part of the public Ophiolite API promise.

## Asset And Compute Taxonomy

As the seismic and subsurface scope expands, not every new capability should be forced into one "processing" bucket.

The working taxonomy is:

- `Source Assets`
  - imported or referenced external data such as seismic volumes, horizons, sparse velocity functions, wells, logs, and velocity cubes
- `Authored Models`
  - user-authored or workflow-authored earth models built from multiple inputs, such as layered velocity models or future horizon-guided property models
- `Compiled Runtime Assets`
  - runtime-ready outputs derived from authored models, such as `SurveyTimeDepthTransform3D` or future survey property fields
- `Analysis APIs`
  - compute and estimation flows that inspect or estimate from data without becoming the canonical authored model itself
- `Display DTOs`
  - app/chart-facing views such as resolved sections, overlays, and map DTOs

This distinction is intentional:

- source assets preserve provenance and import semantics
- authored models capture user intent and editable modeling structure
- compiled runtime assets optimize query/display/materialization
- analysis APIs remain separate from durable authored models
- display DTOs stay transport- and rendering-oriented rather than becoming the canonical model

### Classification Rule

When adding a new capability, classify it before designing contracts:

- if it maps traces to traces on unchanged geometry, it belongs in a processing operator family
- if it estimates or inspects without becoming the source of truth, it belongs in an analysis API family
- if it authors or refines an earth model from multiple inputs, it belongs in an authored-model family
- if it compiles an authored model into a fast runtime field/transform/grid, it belongs in a compiled runtime asset family
- if it only exists to drive app/chart rendering, it belongs in a display DTO family

Phase 1 for this modeling taxonomy stays in the current crates:

- contracts and canonical model types live in `ophiolite-seismic`
- build/runtime implementations live in `ophiolite-seismic-runtime`
- app-boundary DTOs continue to flow through existing Ophiolite and product-facing APIs

That keeps the current repo structure stable while leaving room for a later `ophiolite-modeling` split if the authored-model/build surface becomes broad enough to justify it.

## Current Focus vs Later Ecosystem

The current repo already implements the local-first core:

- source import
- canonical log and typed wellbore asset models
- initial shared seismic descriptors and boundary DTOs
- single-asset packages
- `OphioliteProject`
- typed compute and derived sibling assets, with the deepest eligibility/binding logic currently in logs
- desktop app validation through the internal harness

Later ecosystem layers remain intentionally separate from that core:

- sync / distribution
- broader deployment or enterprise packaging concerns

Sync/distribution and broader deployment remain roadmap items rather than current architecture. See:

- `../../ophiolite_roadmap.md`
- `ADR-0009-future-ecosystem-boundaries.md`
- `ADR-0013-shared-subsurface-core-and-seismic-expansion.md`
- `ADR-0015-authored-models-compiled-runtime-assets-and-display-dtos.md`

## Multi-Well Layer

`ophiolite` now has an explicit layer above single-asset packages:

```text
OphioliteProject
  -> catalog.sqlite
  -> wells
  -> wellbores
  -> asset collections
  -> typed asset packages
```

Current properties of this layer:

- `OphioliteProject` is the user-facing/root concept
- SQLite stores discovery, identities, relationships, package paths, and lightweight status
- single-asset packages remain the authoritative storage unit for asset-local metadata and bulk data
- log assets are the first project-managed asset family
- trajectory, tops, pressure observations, and drilling observations now exist as typed project-managed asset families
- every project-managed asset can carry both:
  - logical identity
  - storage/package identity
- asset collections exist between wellbore and asset instances so versions or alternate deliveries do not collapse into a flat pile

Current multi-well status:

- project creation/open exists now
- LAS import into project-managed log packages exists now
- well/wellbore/collection/asset registry exists now
- CSV ingest for trajectory, tops, pressure observations, and drilling observations exists now
- typed read/query helpers for those non-log families exist now
- cross-asset depth-range discovery across one wellbore exists now
- typed compute discovery/execution exists now for log, trajectory, tops, pressure, and drilling assets and persists derived sibling assets with execution manifests
- synthetic project-fixture generation now exists for testing and app validation; it generates raw LAS/CSV sources and imports them into one coherent `OphioliteProject`
- trajectory, tops, pressure observations, and drilling observations now also support bounded project-scoped edit sessions with explicit save and in-place overwrite of the active asset package

## Package Session Contract

Package-backed editing and inspection now use an explicit backend session model.

Current session properties:

- a package can be opened through metadata-only read paths or through an editable `PackageSession`
- editable session open reuses one shared backend session per package path by default
- `PackageSession` owns package identity, session identity, current in-memory `LasFile` state, dirty-state, and the current head revision token
- backend session open validates package metadata and parquet footer without eagerly decoding all sample rows
- backend-session lazy loading is intentionally scoped: session open avoids full sample decode, read-only session queries decode only requested columns and row windows, metadata-only edits and metadata-only save/save-as remain lazy, and curve/sample edits trigger full materialization
- backend session queries now support both row-window and depth-range access; depth-range requests resolve against the monotonic numeric index curve and then reuse the projected parquet window path
- for regular-step depth logs, lazy backend sessions can derive the row window directly from package metadata and only fall back to reading the full index column when needed
- session summary, metadata, and curve catalog reads are served from cached package metadata while the session remains clean
- backend window reads use projected parquet scans and row selections as internal implementation details instead of preloading a full sample table
- clean `save` on an unchanged lazy session is a no-op success path that preserves lazy state
- metadata-only dirty lazy sessions can rewrite `metadata.json` and save/save-as without materializing sample data
- the first accepted curve/sample edit and any later save/save-as path that needs the canonical snapshot materializes a real eager `PackageSession`
- first curve/sample materialization is built directly from the current lazy session metadata and cached parquet descriptors rather than reopening through the eager SDK package path
- edits are applied to the session snapshot in memory
- `save` persists the current session snapshot back to the same package using overwrite-oriented save semantics
- `save` also creates a new immutable local package revision in a hidden `.ophiolite/revisions/` store
- the hidden revision store is canonical; the visible package root is a materialized current-head view
- `save_as` persists the current session snapshot to a new package root and updates the session baseline
- session summaries and session-context DTOs expose the currently bound package root
- sessions remain alive until explicitly closed in the current desktop MVP
- metadata-only opens do not require loading sample data
- windowed reads are part of the frontend contract and avoid forcing full frontend materialization
- rejected edit requests must not partially mutate session state
- save/save-as verifies enough to confirm the written package is readable and internally coherent before treating the write as successful
- package revision records carry parent linkage, blob refs, typed machine diffs, and readable change summaries for metadata, curve, and row/value changes

Session invariants:

- same package path returns the same shared session while it remains open
- close invalidates the current `SessionId`
- reopen after close creates a new `SessionId`
- `Lazy` and materialized backend-session states preserve the same session identity and bound package root semantics
- `save` preserves session identity and bound package root on success
- `save_as` preserves session identity and rebinds the currently bound package root on success
- once a backend session materializes, it does not transition back to lazy in the current phase
- failed `save` and `save_as` leave the session open with unchanged identity, dirty-state, bound root, and in-memory document snapshot
- failed materialization leaves the session open with unchanged identity, dirty-state, bound root, and no partial mutation applied
- materialization preserves all accepted lazy metadata edits already present in the session and must not reconstruct from stale on-disk metadata

Backend-session parquet metadata caches are session-local in the current phase. They are reused across repeated reads within one open session and dropped when that session is closed.

DTOs are transport shapes for this contract. They do not replace the canonical domain model.

Current DTO families:

- read DTOs: package summary, metadata, curve catalog, curve windows, session summary
- edit DTOs: metadata edits, curve edits, dirty-state, validation reports, and save results
- `PackageBackendState` is the shared-state wrapper used by the internal Tauri capability harness and intended for further Tauri command registration
- `PackageCommandService` is the thin, transport-focused service that converts command calls into structured transport responses
- session-backed metadata, catalog, and window reads now carry explicit session context and DTO contract versions
- validation reports now carry structured diagnostic issues with code, severity, message, and optional target context
- app-boundary command rules:
  - inspect commands do not require a session
  - session commands require or produce a valid `SessionId`
  - edit/persist commands operate on an existing session
- transport envelope rule:
  - `CommandResponse<T> = Ok(T) | Err(CommandErrorDto)`
  - public error kinds stay small and caller-actionable

Current validation layers:

- package validity: is the package structurally readable and coherent
- edit validity: is the requested mutation allowed against the current in-memory snapshot
- save validity: can the current snapshot be persisted and reopened coherently now
- validation reports are structured for app consumers rather than only exposing raw message lists

At the command boundary, save and save-as validation failures are reported as save-scoped validation rather than generic edit validation.

## Workspace Layout

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
  -> ophiolite-cli
```

Current staged compromise:

- `ophiolite-project` now owns the multi-asset catalog, asset manifests, typed project queries, summary APIs, and synthetic project fixtures
- `ophiolite-ingest` now provides the first explicit ingest-oriented crate boundary over project import flows
- `ophiolite-compute` now owns the typed compute registry, semantic curve eligibility, and execution-manifest model for derived assets
- `ophiolite-seismic` now owns the first shared seismic descriptor and section/trace contract layer intended to replace duplicated product-only seismic model ownership over time
- `ophiolite-seismic-io` now owns shared SEG-Y inspection and ingest-oriented IO
- `ophiolite-seismic-runtime` now owns the shared runtime/store backend, including the canonical `tbvol` path
- the root `ophiolite` crate remains the compatibility facade that re-exports the workspace surface
- the runtime table boundary has its own crate, but `CurveTable` still originates from the core layer in this phase to preserve the current `LasFile::data()` API
- Arrow/Parquet conversion now lives in the package crate rather than the runtime table type
- direct SDK package opens remain eager; only backend session reads are lazy in this phase

## Current vs Target

| Area | Current implementation | Target direction |
| --- | --- | --- |
| Domain model | `LasFile` plus typed canonical metadata and explicit index/curve descriptors | further canonical tightening around index/null semantics |
| In-memory samples | `CurveTable` backed by current in-memory values | potentially more formal Arrow-backed runtime contract later |
| Package format | grouped `metadata.json + curves.parquet` with mixed-column preservation, tuned Parquet writer properties for depth-track workloads, and legacy metadata read-compat | stricter canonical schema and package guarantees |
| Canonical schema | partially aligned | `ADR-0007-canonical-schema-target.md` is the target-state reference |
| Frontend/backend boundary | CLI, Rust API, shared backend state, structured command wrapper, and an internal Tauri capability harness | broader desktop-app integration later |

## Roadmap Placement of Arrow/Parquet

Arrow/Parquet is already in use for package persistence, but it is not yet the full canonical runtime model.

Before deepening Arrow/Parquet integration, `ophiolite` should first stabilize:

1. Tauri/backend DTOs and query semantics
2. package-session lifecycle and save semantics
3. nullability/index/curve descriptor rules
4. editable-session loading behavior where it materially helps the desktop workflow

Only after those are stable should the project tighten runtime/package behavior toward the full canonical schema target.

## Decision Records

- `ADR-0001-canonical-las-model.md`
- `ADR-0002-staged-arrow-parquet-adoption.md`
- `ADR-0003-package-format-metadata-json-plus-curves-parquet.md`
- `ADR-0004-lasio-parity-and-scope.md`
- `ADR-0005-staged-workspace-split-and-table-boundary.md`
- `ADR-0006-package-session-and-dto-boundary.md`
- `ADR-0007-canonical-schema-target.md`
- `ADR-0008-project-catalog-and-single-asset-packages.md`
- `ADR-0009-future-ecosystem-boundaries.md`
- `ADR-0010-typed-compute-and-derived-assets.md`
- `ADR-0011-structured-asset-edit-sessions.md`
- `ADR-0012-revisioned-last-save-wins.md`
- `ADR-0013-shared-subsurface-core-and-seismic-expansion.md`
- `ADR-0014-seismic-crs-native-effective-display-boundary.md`
- `ADR-0015-authored-models-compiled-runtime-assets-and-display-dtos.md`
- `ADR-0016-canonical-wellbore-geometry-and-resolved-trajectory-boundary.md`
- `ADR-0017-well-time-depth-source-assets-authored-models-and-compiled-runtime-output.md`
- `ADR-0018-project-aware-well-on-section-overlay-dtos-and-backend-projection-rules.md`
- `ADR-0019-chart-kernels-rock-physics-crossplot-and-point-cloud-spike.md`
- `ADR-0020-avo-chart-family-analysis-contracts-and-initial-rendering-plan.md`
- `ADR-0021-volume-interpretation-chart-family-and-resolved-scene-boundary.md`
- `ADR-0022-canonical-well-and-wellbore-metadata-phase-one.md`
- `ADR-0023-canonical-well-markers-phase-one.md`
- `ADR-0024-authored-marker-set-assets-and-canonical-marker-precedence.md`
- `ADR-0025-well-marker-depth-horizon-residuals.md`
- `ADR-0026-vendor-project-import-adapters.md`
- `ADR-0027-asset-owner-scopes-for-vendor-and-survey-assets.md`
- `ADR-0028-domain-first-python-sdk-surface-and-advanced-namespaces.md`
- `ADR-0029-unified-import-manager-and-provider-registry.md`
- `ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md`
- `ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md`
- `ADR-0032-processing-authority-and-thin-client-migration.md`
- `ADR-0033-public-sdk-core-and-adapter-boundaries.md`
- `ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`
- `processing-canonical-integration-plan-2026-04.md`

## Related Docs

- `../lasio_non_v3_parity.md`
- `../../lasio-basic-example.md`
- `seismic-execution-service-implementation-sketch.md`
