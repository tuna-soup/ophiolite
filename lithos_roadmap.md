# Lithos Roadmap

## Overview

Lithos is no longer just a LAS/log SDK. The current repo already contains:

- a strong LAS/log import and package/edit foundation
- a local-first `LithosProject` catalog
- typed non-log asset families for trajectory, tops, pressure observations, and drilling observations
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
- last-save-wins package/session persistence

### Multi-Asset Project Layer

- `LithosProject`
- well, wellbore, asset collection, and asset identities
- logical vs storage asset identity
- project catalog in SQLite
- project-managed single-asset packages
- typed asset families:
  - log
  - trajectory
  - tops
  - pressure observations
  - drilling observations
- typed read/query APIs for those families
- cross-asset depth-range discovery for one wellbore
- current/superseded asset history for simple replace workflows

### Validation Surface

- internal project-first Tauri harness
- synthetic multi-asset project fixture generation
- parity and package/editing regression coverage

## Next Platform-Core Milestones

These are the highest-value missing pieces for turning the current foundation into a durable application platform.

### 1. Project-Facing Platform API

Stabilize a cleaner project-facing summary/service layer above raw catalog internals:

- well summaries
- wellbore summaries
- asset collection summaries
- asset summaries by type and status
- cross-asset coverage queries

The goal is to give apps a durable platform surface instead of making them bind directly to lower-level catalog details.

### 2. Monorepo Platform Skeleton

Make the platform shape explicit in the workspace by extracting:

- `crates/lithos-project`
  - catalog
  - manifests
  - typed asset queries
  - synthetic project fixtures
- `crates/lithos-ingest`
  - import orchestration
  - source import adapters
  - future ingest adapters

Keep `lithos_las` as the compatibility facade over the workspace crates.

### 3. Simpler Import and Versioning Workflows

Keep the operational model simple and local-first:

- last save wins for package/session persistence
- latest import becomes the current asset in a collection by default
- previous asset versions remain traceable as superseded history
- provenance stays available without forcing review-heavy workflows

The goal is to make Lithos predictable for everyday subsurface work rather than Git-like.

### 4. Cross-Asset App Workflows

Use the harness as the validation target for:

- better project browsing over wells, wellbores, collections, and assets
- current vs superseded asset visibility
- richer viewers for logs + trajectory + tops together

## Application-Validation Milestones

Once the platform-core skeleton is in place, validate it through real workflows:

- open a project and browse multiple wells / wellbores
- inspect logs, tops, trajectory, pressure, and drilling together
- reimport and confirm last-save-wins/supersession behavior
- exercise synthetic project fixtures as a default demo/test path

The rule is:

> validate the platform through real application workflows as early as possible.

## Later Ecosystem Direction

These are intentionally later than the current platform-core work.

### Compute Layer

Add a separate computation layer for:

- pure functions / UDFs
- derived asset generation
- stateless transformations over assets

This should sit above Lithos Core rather than inside the catalog/package layer.

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

Lithos is now best described as:

> a local-first subsurface well-data platform foundation with a particularly strong LAS/log engine

The next step is not to rebuild the foundation. It is to make the current foundation app-facing, structurally clear in the monorepo, and easy to build on.
