# Lithos Roadmap: From LAS SDK to Local-First Subsurface Platform

## Overview

This roadmap captures the next practical steps for evolving Lithos from a strong LAS and log-asset SDK into a local-first foundation for real subsurface applications.

The key principle is:

> Validate the architecture through real application workflows, not by expanding abstractions in isolation.

Lithos is no longer at the stage of asking whether a multi-well architecture should exist. The repo already has the first concrete slice of that architecture. The roadmap now needs to focus on validating, hardening, and extending it.

---

## Current State

Lithos already provides:

- LAS ingestion and parsing
- canonical LAS domain model (`LasFile`)
- optimized single-asset packages (`metadata.json + curves.parquet`)
- editable package sessions and save/save-as semantics
- depth-range and row-window query paths for log curves
- backend + command service + Tauri harness
- local-first `LithosProject` with SQLite catalog
- project-managed asset packages for:
  - logs
  - trajectory
  - tops
  - pressure observations
  - drilling observations
- logical asset identity plus per-package storage identity
- cross-asset depth-range discovery for one wellbore

This means Lithos already has:

- a strong log engine
- a first multi-well/project slice
- a usable internal desktop validation surface

What it does not yet have is a fully validated, end-to-end multi-asset application workflow or a hardened import-governance layer.

---

## Current Gaps

The highest-value missing pieces are now:

- project-centric app workflows across multiple asset families
- stronger import reconciliation and asset binding rules
- asset lifecycle and review flows for ambiguous imports
- richer cross-asset query and visualization use cases
- broader ingest coverage after the project model is proven
- mature non-log editing workflows

The next steps should reflect that ordering.

---

# Roadmap

---

## Near Term 1 - Validate the Multi-Asset Application Workflow

### Goal
Use the Tauri app as the first real consumer of the project/catalog architecture.

### Deliverables

#### 1. Make the app project-centric

- create/open `LithosProject`
- browse wells and wellbores
- browse asset collections and assets by type/status
- open project-managed assets from the app rather than treating packages as the only primary entry point

#### 2. Add first real cross-asset workflows

- inspect one wellbore across logs + tops + trajectory
- inspect pressure observations alongside wellbore context
- show which assets cover a selected depth interval
- use the depth-range query path as the default log-viewer interaction model

#### 3. Add asset-aware viewers in the harness

- log track / curve viewer
- tops list / interval view
- trajectory table/profile view
- pressure observation view
- drilling observation view

#### 4. Validate the current SDK ergonomics through the app

- confirm naming and DTO/query shape are usable from a real frontend
- identify missing summary APIs, filters, and cross-asset joins
- refine the app boundary where usage exposes real friction

### Outcome

- Lithos proves it can support a real multi-asset desktop workflow
- the next API changes are driven by application use, not speculation

---

## Near Term 2 - Harden Import Reconciliation and Asset Governance

### Goal
Prevent the project/catalog layer from becoming inconsistent as more assets are imported.

### Deliverables

#### 1. Add explicit import resolution and binding rules

- how a source maps to `Well`
- how a source maps to `Wellbore`
- which identifiers are trusted first
- how duplicate detection works
- how unmatched or ambiguous imports are staged

#### 2. Add asset lifecycle states to the catalog

- `Imported`
- `Validated`
- `Bound`
- `NeedsReview`
- `Rejected`
- `Superseded`

#### 3. Add review-oriented project APIs

- unresolved asset listing
- duplicate candidates
- asset supersession history
- rebind/review actions for ambiguous imports

#### 4. Keep the authority split explicit

- SQLite remains the discovery/relationship layer
- asset packages remain the authoritative storage unit for asset-local data and manifests

### Outcome

- imports become governable rather than ad hoc
- projects stay clean as multiple wells and repeated deliveries are introduced

---

## Near Term 3 - Expand Cross-Asset Query and Read Use Cases

### Goal
Make Lithos useful for actual subsurface workflows, not just asset storage.

### Deliverables

#### 1. Add higher-level project queries

- list assets by well / wellbore / kind / status
- query all assets covering a requested depth range
- find wells with specific asset combinations
- locate observations near a log or trajectory interval

#### 2. Tighten shared semantic reference types

- `DepthReference`
- `VerticalDatum`
- `CoordinateReference`
- `UnitSystem`
- `WellIdentifierSet`

#### 3. Add cross-asset consistency checks

- datum mismatches
- unit mismatches
- coverage inconsistencies
- identifier conflicts

### Outcome

- the project model becomes useful as an analysis/query substrate
- cross-asset workflows become safer and more predictable

---

## Medium Term - Broaden Ingest Once the Project Model is Proven

### Goal
Expand source coverage only after the project/catalog and reconciliation model is validated.

### Deliverables

#### 1. Deeper LAS 3 extraction

- trajectory/inclinometry sections
- tops/marker sections
- test/pressure-like sections
- drilling-related structured sections

#### 2. Stronger structured import adapters

- CSV and Parquet import hardening for existing non-log asset families
- clearer import contracts for schema mapping, units, and references

#### 3. Later source support

- DLIS/LIS and other richer wireline sources

### Outcome

- Lithos expands source coverage on top of a stable well-domain foundation
- ingest breadth no longer outruns the project model

---

## Longer Term - Selective Editing Expansion

### Goal
Extend editing carefully beyond logs once read/query and reconciliation workflows are stable.

### Deliverables

- tops editing
- trajectory editing
- pressure observation correction/annotation flows
- drilling observation correction/annotation flows

Keep this intentionally later than the read/query and import-governance work. Log/package editing remains the mature editing path until these models are validated independently.

### Outcome

- Lithos moves from read-mostly non-log support into selective, domain-appropriate editing

---

## Not the Next Step

Do not prioritize these before the roadmap above:

- redesigning single-asset packages into multi-well containers
- cloud/object-store/Iceberg/Postgres architecture
- collaborative editing or sync
- mirroring OSDU schemas directly in public Rust types
- broad storage redesign without a concrete app-driven need

OSDU remains a useful reference for domain boundaries and discoverability patterns, but not a schema source of truth for Lithos.

---

## Validation Milestones

### Milestone A

> A project can be opened in the Tauri app, browsed by well/wellbore, and used to inspect log + tops + trajectory together for one wellbore.

### Milestone B

> Ambiguous or duplicate imports can be surfaced, reviewed, and bound without polluting the catalog.

### Milestone C

> Cross-asset queries over depth ranges and asset coverage become a normal application workflow.

### Milestone D

> Broader source support can be added without destabilizing the project/catalog model.

---

## Final Summary

### Already Done

- canonical LAS/log foundation
- package/session/edit model
- depth-range optimized log querying
- Tauri app shell
- multi-well project/catalog foundation
- typed multi-asset package families

### Next

- validate the architecture through the Tauri app
- harden import reconciliation and lifecycle governance
- deepen cross-asset query/use-case support

### After That

- broaden ingest support
- expand editing carefully beyond logs

---

## Closing Thought

You are no longer primarily building:

> a better LAS package SDK

You are now building:

> a local-first subsurface application foundation

The fastest way to validate that claim is:

> make the multi-asset application workflow first-class and let it drive the next round of SDK hardening
