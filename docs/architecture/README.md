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
- `PackageSession` is the backend-owned editable package session model
- `PackageBackend` and `PackageBackendState` are the current Tauri-ready backend adapter layer
- `PackageCommandService` is the app-boundary transport layer above the shared backend state
- `CurveTable` is the app-facing in-memory table abstraction
- DTOs are the intended frontend/backend transfer boundary
- package storage uses `metadata.json + curves.parquet`
- `metadata.json` now groups package identity, document metadata, storage descriptors, raw preserved sections, and diagnostics explicitly
- Arrow/Parquet are internal storage/package details, and now also power backend-session lazy window reads
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

## Package Session Contract

Package-backed editing and inspection now use an explicit backend session model.

Current session properties:

- a package can be opened through metadata-only read paths or through an editable `PackageSession`
- editable session open reuses one shared backend session per package path by default
- `PackageSession` owns package identity, session identity, current in-memory `LasFile` state, dirty-state, and a revision token
- backend session open validates package metadata and parquet footer without eagerly decoding all sample rows
- backend-session lazy loading is intentionally scoped: session open avoids full sample decode, read-only session queries decode only requested columns and row windows, metadata-only edits and metadata-only save/save-as remain lazy, and curve/sample edits trigger full materialization
- session summary, metadata, and curve catalog reads are served from cached package metadata while the session remains clean
- backend window reads use projected parquet scans and row selections as internal implementation details instead of preloading a full sample table
- clean `save` on an unchanged lazy session is a no-op success path that preserves lazy state
- metadata-only dirty lazy sessions can rewrite `metadata.json` and save/save-as without materializing sample data
- the first accepted curve/sample edit and any later save/save-as path that needs the canonical snapshot materializes a real eager `PackageSession`
- first curve/sample materialization is built directly from the current lazy session metadata and cached parquet descriptors rather than reopening through the eager SDK package path
- edits are applied to the session snapshot in memory
- `save` persists the current session snapshot back to the same package using optimistic revision checks
- `save_as` persists the current session snapshot to a new package root and updates the session baseline
- session summaries and session-context DTOs expose the currently bound package root
- sessions remain alive until explicitly closed in the current desktop MVP
- metadata-only opens do not require loading sample data
- windowed reads are part of the frontend contract and avoid forcing full frontend materialization
- rejected edit requests must not partially mutate session state
- save/save-as verifies enough to confirm the written package is readable and internally coherent before treating the write as successful

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
- edit DTOs: metadata edits, curve edits, dirty-state, validation reports, save results, save conflicts
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
- save validity/conflict: can the current snapshot be persisted safely now
- save conflict detection is against the currently bound package baseline/root and its revision fingerprint
- validation reports are structured for app consumers rather than only exposing raw message lists

At the command boundary, save and save-as validation failures are reported as save-scoped validation rather than generic edit validation.

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
- direct SDK package opens remain eager; only backend session reads are lazy in this phase

## Current vs Target

| Area | Current implementation | Target direction |
| --- | --- | --- |
| Domain model | `LasFile` plus typed canonical metadata and explicit index/curve descriptors | further canonical tightening around index/null semantics |
| In-memory samples | `CurveTable` backed by current in-memory values | potentially more formal Arrow-backed runtime contract later |
| Package format | grouped `metadata.json + curves.parquet` with mixed-column preservation and legacy metadata read-compat | stricter canonical schema and package guarantees |
| Canonical schema | partially aligned | `ADR-0007-canonical-schema-target.md` is the target-state reference |
| Frontend/backend boundary | CLI, Rust API, shared backend state, structured command wrapper, and an internal Tauri capability harness | broader desktop-app integration later |

## Roadmap Placement of Arrow/Parquet

Arrow/Parquet is already in use for package persistence, but it is not yet the full canonical runtime model.

Before deepening Arrow/Parquet integration, `lithos` should first stabilize:

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

## Related Docs

- `../lasio_non_v3_parity.md`
- `../../lasio-basic-example.md`
