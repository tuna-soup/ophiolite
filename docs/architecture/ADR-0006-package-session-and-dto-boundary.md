# ADR-0006: Package Session and DTO Boundary

## Status

Accepted

## Decision

`lithos` uses an explicit backend-owned package session model for package-backed inspection and editing.

This ADR is intentionally scoped to package-backed asset editing, which remains the deepest model for log assets. The broader `LithosProject` catalog and typed non-log asset families sit above this layer; those structured families now use separate project-scoped typed edit sessions rather than `PackageSession`.

The current session type is `PackageSession`.

By default, editable package open reuses one shared backend session per package path.

Multiple frontend windows talk to the same in-memory session state in this phase. The backend is the source of truth and no per-window copy exists by default.

`PackageSession` currently owns:

- package identity
- session identity
- the current in-memory `LasFile` snapshot
- dirty-state
- the current revision token used as a package snapshot/version fingerprint

Conceptually, it should continue to be treated as:

- identity
- document snapshot
- persistence/session state

so the implementation can be split further later without changing the contract.

The package contract is split conceptually into:

- read DTOs:
  - package summary
  - metadata view
  - curve catalog
  - curve window query result
  - depth-range curve window query result
  - session summary
- edit DTOs:
  - metadata edit requests
  - curve edit requests
  - dirty-state
  - validation reports
  - save/save-as results

At the app boundary, commands follow a single envelope pattern:

- `CommandResponse<T> = Ok(T) | Err(CommandErrorDto)`

Inspect commands do not require a session. Session commands require or produce a valid `SessionId`. Edit/persist commands operate on an existing session.

DTOs are transport shapes only. They do not replace the canonical domain model. `LasFile` remains the authoritative in-memory LAS representation inside the backend.

The DTO contract is versioned with a lightweight `dto_contract_version` field.
Session-backed metadata, catalog, and window reads should carry explicit session context rather than forcing desktop clients to reconstruct it from separate calls.
Validation reports should expose structured diagnostic issues with stable codes, severity, human-readable messages, and optional target context.

The current Tauri-ready adapter layer is `PackageBackend`, with `PackageBackendState` as the shared-state wrapper and `PackageCommandService` as the thin, transport-focused app-boundary service above it.
Session-backed DTOs should expose the currently bound package root so clients can observe rebinding after `save_as`.
In the current phase, backend session read paths may stay lazy internally, while the public eager `PackageSession` and direct `open_package(...)` APIs remain unchanged.
That lazy scope is intentionally narrow: session open avoids full sample decode, read-only session queries decode only requested columns and row windows, metadata-only edits and metadata-only save/save-as remain lazy, and curve/sample edits trigger full materialization.
Depth-range reads are first-class alongside row-window reads. They resolve against the monotonic numeric index curve and then reuse the same projected parquet window machinery internally.
For regular-step depth logs, the backend may derive row bounds directly from package metadata before falling back to reading the full index column.
When first curve/sample materialization is required, it should be built directly from the current lazy session metadata and cached parquet descriptors rather than reopening the package through the eager SDK path.

DTO evolution should remain additive where possible. Formal compatibility guarantees can harden later once the Tauri contract stops moving quickly.
Public command error kinds should remain small and caller-actionable rather than implementation-shaped.

## Why

- package-backed desktop workflows need a clear owner for pending edits and save semantics
- session identity and revision tracking are required once multiple queries and edits can occur against the same open package
- separating read DTOs from edit DTOs keeps the frontend/backend contract easier to reason about
- last-save-wins persistence better matches current local-first desktop workflows than merge/conflict-centric behavior
- keeping DTOs distinct from the domain model preserves the domain-first architecture

## Consequences

- package editing behavior should be described in terms of `PackageSession`, not ad hoc package helpers
- documentation must distinguish metadata-only/read-only flows from editable session flows
- dirty-state and revision fingerprint handling are now part of the supported backend contract
- metadata-only opens are an explicit architectural behavior, not an accidental implementation detail
- windowed reads are part of the frontend contract even though full lazy sample-table loading is still an evolving internal concern
- backend session reads may reuse Arrow/Parquet projection and row-selection internally without changing the public runtime abstraction
- `curves.parquet` should be written with a depth-query-oriented Parquet profile: column projection-friendly storage, page statistics, offset index, bounded row-group/data-page row counts, and sort metadata on the index column when the index is monotonic
- parquet row selection remains an internal implementation detail; public query semantics do not change
- Tauri command handlers, including the internal harness, should be built on top of this session model rather than reaching directly into storage internals
- the app-boundary command layer should preserve structured backend errors rather than collapsing them into ad hoc strings
- edit requests must be atomic at the request level; rejected edits must not partially mutate session state
- save/save-as failure behavior should preserve session usability and identity rather than partially mutating session state

## Session Lifecycle

For the current desktop MVP:

- sessions are backend-owned
- sessions remain alive until explicitly closed
- there is no automatic session expiry yet
- shared sessions may outlive a window

This keeps lifecycle rules simple while the Tauri contract is still being defined.

Session invariants for the current model:

- same package path returns the same shared session while it remains open
- close invalidates the current `SessionId`
- reopen after close returns a new `SessionId`
- lazy and materialized backend-session states preserve the same session identity and bound package root semantics
- `save` preserves session identity and bound package root on success
- `save_as` preserves session identity and rebinds the currently bound package root on success
- clean `save` on an unchanged lazy session is a no-op success path that preserves lazy state
- metadata-only dirty lazy sessions may rewrite `metadata.json` and save/save-as without materializing sample data
- once a backend session materializes, it does not transition back to lazy in the current phase
- failed `save` and `save_as` leave the session open with unchanged identity, dirty-state, bound root, and in-memory document snapshot
- failed materialization leaves the session open with unchanged identity, dirty-state, bound root, and no partial mutation applied
- materialization preserves all accepted lazy metadata edits already present in the session and must not reconstruct from stale on-disk metadata

`save_as` should be understood as: the user remains in the same editing session, but that session is now editing the newly written package.

Backend-session parquet metadata caches are session-local in the current phase. They are reused across repeated reads within one open session and dropped when that session is closed.

## Validation Boundaries

`lithos` now recognizes three separate validation concerns:

1. package validity
2. edit validity
3. save validity

These concerns should remain distinct in result shapes and error reporting.
In particular, save/save-as validation failures should not be reported as generic edit validation.
Post-write validation should remain bounded to confirming that the written package is readable and internally coherent.
Validation DTOs should be structured enough that desktop clients can render diagnostics without string matching.

## Deferred Work

This ADR does not imply:

- raw LAS write-back support
- full undo/redo support
- a final stable Tauri command surface
- collaborative or multi-user editing semantics
- duplicate or forked live-session semantics
- a hard `tauri` dependency in this repo yet

Revision tokens are informational snapshot/version fingerprints, not merge or distributed-synchronization coordination tokens.

Those remain future layers on top of the current session contract.

## Success Criteria

This lifecycle milestone is considered complete when session lifecycle behavior, save/save-as rebinding semantics, and structured failure cases are encoded in tests and reflected consistently across the backend, command-service, and durable docs.
