# ADR-0011: Structured Asset Edit Sessions

## Status

Accepted

## Decision

`lithos` adds project-scoped typed edit sessions for the first structured wellbore asset families:

- trajectory
- tops
- pressure observations
- drilling observations

These edit sessions are intentionally lighter than `PackageSession`.

They are:

- opened against one active structured asset package at a time
- typed by asset family
- limited to in-family row add/update/delete and field patch operations
- explicit-save workflows
- overwrite-oriented save semantics on persistence

They do not:

- convert one asset family into another
- expose generic arbitrary-column mutation
- introduce `save_as` in the first phase
- create derived sibling assets for manual edits by default

Persistence rule:

- successful save overwrites the current active structured asset package in place, writes a new immutable local asset revision into the hidden revision store, and rematerializes the visible asset root from that head
- failed save leaves the edit session open and dirty
- compute-derived structured assets remain separate sibling assets and can also be edited as normal structured assets later

## Why

- trajectory, tops, pressure, and drilling rows need editable application workflows, not just import/read/compute paths
- these families have clear typed schemas and family-specific validation rules
- they do not need the full `PackageSession` architecture that logs use
- a lighter project-scoped edit model keeps the implementation safe and comprehensible while matching the current local-first overwrite workflow

## Consequences

- `PackageSession` remains the log/package editing model
- structured families use a separate typed edit-session store
- row/field validation becomes family-specific:
  - trajectory save requires monotonic measured depth
  - tops require name and top depth, with base depth not above top depth
  - pressure requires finite pressure values
  - drilling requires event kind
- Tauri/app integrations can expose edit mode for structured assets without pretending all asset families are generic tables
- manual structured edits overwrite the active asset package in place, create revision history in the hidden project-local revision store, and leave compute-derived sibling assets as a separate workflow

## Scope Boundaries

This ADR does not imply:

- spreadsheet-style generic editing for every asset
- collaborative editing
- merge/conflict resolution
- cross-family conversion
- non-log `save_as` in the first phase

Those remain future workflow decisions.
