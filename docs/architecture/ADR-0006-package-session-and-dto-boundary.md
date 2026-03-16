# ADR-0006: Package Session and DTO Boundary

## Status

Accepted

## Decision

`lithos` uses an explicit backend-owned package session model for package-backed inspection and editing.

The current session type is `PackageSession`.

By default, editable package open reuses one shared backend session per package path.

Multiple frontend windows talk to the same in-memory session state in this phase. The backend is the source of truth and no per-window copy exists by default.

`PackageSession` currently owns:

- package identity
- session identity
- the current in-memory `LasFile` snapshot
- dirty-state
- the current revision token used for optimistic save conflict checks

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
  - session summary
- edit DTOs:
  - metadata edit requests
  - curve edit requests
  - dirty-state
  - validation reports
  - save/save-as results
  - save conflict results

DTOs are transport shapes only. They do not replace the canonical domain model. `LasFile` remains the authoritative in-memory LAS representation inside the backend.

The DTO contract is versioned with a lightweight `dto_contract_version` field.

The current Tauri-ready adapter layer is `PackageBackend`, with `PackageBackendState` as the shared-state wrapper a future Tauri app can mount behind thin command handlers.

DTO evolution should remain additive where possible. Formal compatibility guarantees can harden later once the Tauri contract stops moving quickly.

## Why

- package-backed desktop workflows need a clear owner for pending edits and save semantics
- session identity and revision tracking are required once multiple queries and edits can occur against the same open package
- separating read DTOs from edit DTOs keeps the frontend/backend contract easier to reason about
- optimistic save conflict handling is safer than silent last-writer-wins behavior
- keeping DTOs distinct from the domain model preserves the domain-first architecture

## Consequences

- package editing behavior should be described in terms of `PackageSession`, not ad hoc package helpers
- documentation must distinguish metadata-only/read-only flows from editable session flows
- dirty-state and revision handling are now part of the supported backend contract
- metadata-only opens are an explicit architectural behavior, not an accidental implementation detail
- windowed reads are part of the frontend contract even though full lazy sample-table loading is still an evolving internal concern
- future Tauri command handlers should be built on top of this session model rather than reaching directly into storage internals
- edit requests must be atomic at the request level; rejected edits must not partially mutate session state

## Session Lifecycle

For the current desktop MVP:

- sessions are backend-owned
- sessions remain alive until explicitly closed
- there is no automatic session expiry yet
- shared sessions may outlive a window

This keeps lifecycle rules simple while the Tauri contract is still being defined.

## Validation Boundaries

`lithos` now recognizes three separate validation concerns:

1. package validity
2. edit validity
3. save validity or save conflict

These concerns should remain distinct in result shapes and error reporting.

## Deferred Work

This ADR does not imply:

- raw LAS write-back support
- full undo/redo support
- a final stable Tauri command surface
- collaborative or multi-user editing semantics

Revision tokens are used for persistence conflict checks, not live distributed synchronization.

Those remain future layers on top of the current session contract.
