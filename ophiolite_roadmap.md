# Ophiolite Roadmap

## Overview

Ophiolite is no longer just a LAS/log SDK. The current repo already contains:

- a strong LAS/log import and package/edit foundation
- a local-first `OphioliteProject` catalog
- typed non-log asset families for trajectory, tops, pressure observations, and drilling observations
- a typed compute layer with derived sibling assets across log and structured families
- a project-first Tauri harness
- synthetic multi-asset project fixtures for testing and app validation

So the roadmap should start from that truth.

The current goal is:

> Turn the existing multi-asset foundation into a stable platform layer for building subsurface applications.

## Implemented Foundations

### Log and Package Engine

- tolerant LAS ingestion and parsing
- canonical `LasFile` model
- optimized log package format: `metadata.json + curves.parquet`
- `PackageSession` editing and save/save-as model
- lazy backend reads and depth-range log queries
- structured command and DTO boundary for desktop app use
- overwrite-oriented package/session persistence
- immutable local package revisions with hidden canonical revision stores, typed machine diffs, and readable change summaries

### Multi-Asset Project Layer

- `OphioliteProject`
- well, wellbore, asset collection, and asset identities
- logical vs storage asset identity
- project catalog in SQLite
- project-managed single-asset packages
- project-facing summary APIs for project, well, wellbore, collection, and asset overviews
- typed asset families:
  - log
  - trajectory
  - tops
  - pressure observations
  - drilling observations
- typed read/query APIs for those families
- cross-asset depth-range discovery for one wellbore
- current/superseded asset history for simple replace workflows
- project-scoped structured edit sessions for trajectory, tops, pressure observations, and drilling observations
- immutable local asset revisions for structured edits, imports, compute outputs, and synced log package heads

### Monorepo Platform Skeleton

- `ophiolite-project` workspace crate for project/catalog, manifests, typed queries, and synthetic fixtures
- `ophiolite-ingest` workspace crate for import-oriented orchestration boundaries
- `ophiolite-compute` workspace crate for semantic compute eligibility, function registry, and derived-asset execution
- `ophiolite` preserved as the compatibility facade over the workspace crates

### Validation Surface

- internal project-first Tauri harness
- synthetic multi-asset project fixture generation
- parity and package/editing regression coverage

## Next Platform-Core Milestones

These are the highest-value missing pieces for turning the current foundation into a durable application platform.

### 1. Simpler Import and Versioning Workflows

Keep the operational model simple and local-first:

- overwrite-oriented saves for package/session persistence
- latest import becomes the current asset in a collection by default
- previous asset versions remain traceable as superseded history
- provenance stays available without forcing review-heavy workflows
- hidden revision stores stay append-only for now; retention/GC remain later infrastructure work

The goal is to make Ophiolite predictable for everyday subsurface work rather than Git-like.

### 2. Broader Compute Surface

The first compute slice is now in place for log and structured wellbore assets. The next compute-specific steps are:

- deeper semantic classification and override workflows
- more petrophysics / rock-physics functions
- deepen non-log asset-family compute only where it clearly fits the typed model
- richer app workflows for selecting bindings, editing parameters, and inspecting derived assets

### 3. Cross-Asset App Workflows

Use the harness as the validation target for:

- better project browsing over wells, wellbores, collections, and assets
- current vs superseded asset visibility
- richer viewers for logs + trajectory + tops together
- stronger structured editing UX on top of the new typed edit-session layer

### 4. Broader Ingest Adapters

Deepen the explicit ingest layer after the crate extraction:

- stronger CSV adapter coverage
- richer LAS 3 section extraction into non-log asset families
- cleaner import entry points for apps and automation

## Application-Validation Milestones

Once the platform-core skeleton is in place, validate it through real workflows:

- open a project and browse multiple wells / wellbores
- inspect logs, tops, trajectory, pressure, and drilling together
- reimport and confirm overwrite-oriented save/supersession behavior
- exercise synthetic project fixtures as a default demo/test path

The rule is:

> validate the platform through real application workflows as early as possible.

## Later Ecosystem Direction

These are intentionally later than the current platform-core work.

### Sync / Distribution Layer

Add an optional sync layer for:

- push / pull of project state or asset packages
- version exchange
- simple replication, export/import, or distribution workflows

This remains outside the current local-first core.

### Broader Ingest and Asset Expansion

After platform-core and app validation:

- deeper LAS 3 extraction
- broader structured import adapters
- later DLIS/LIS support
- additional asset families such as completion, well plan, or well activity

## Principles

- optimize for enabling applications, not only for storage elegance
- keep packages single-asset and local-first
- keep OSDU alignment conceptual, not schema-literal
- validate architecture through real apps, not just backend abstractions
- keep core, app, compute, and sync concerns separate even if they remain in one monorepo for now
- prefer simple overwrite/supersede workflows over conflict-resolution-heavy collaboration models

## Current Checkpoint

Ophiolite is now best described as:

> a local-first subsurface well-data platform foundation with a particularly strong LAS/log and log-compute engine

The next step is not to rebuild the foundation. It is to make the current foundation app-facing, structurally clear in the monorepo, and easy to build on.
