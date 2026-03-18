# ADR-0008: Project Catalog and Single-Asset Packages

## Status

Accepted

## Decision

`lithos` keeps single-asset packages as the authoritative storage unit and adds a local-first `LithosProject` layer above them for multi-well organization.

The project layer owns:

- well and wellbore identities
- asset collections and typed asset registry
- package locations
- lightweight status and searchable summaries
- relationship and discovery queries

The package layer remains authoritative for:

- asset-local metadata
- provenance tied to packaged content
- import/package diagnostics
- bulk data descriptors
- Parquet-backed or file-backed payloads

The initial multi-well implementation is intentionally read-mostly. It currently includes:

- log assets
- trajectory assets
- tops assets
- pressure observation assets
- drilling observation assets

Current ingest/query status:

- LAS import feeds project-managed log assets
- CSV import feeds the first non-log asset families
- typed read/query APIs exist for trajectory, tops, pressure observations, and drilling observations
- logs-first typed compute now produces derived sibling log assets through the same project/catalog and single-asset package model
- project-facing summary APIs exist for project, well, wellbore, collection, and asset overviews
- synthetic multi-asset project fixtures are generated from raw LAS/CSV source files and then imported through those same project APIs
- rich edit sessions are still primarily a log/package capability, but trajectory/tops/pressure/drilling now also support bounded in-family project-scoped edit sessions

## Why

- the existing package/session model is already strong and should not be discarded for multi-well support
- multi-well applications need discovery, binding, and cross-asset organization that do not belong inside one package
- SQLite is a pragmatic local-first fit for identities and relationships
- the architecture needs an explicit grouping layer between wellbore and individual asset instances
- logical asset identity must be separated from storage/package identity to support reimport, supersession, and versioned deliveries cleanly

## Consequences

- `LithosProject` becomes the multi-well entry point without turning `.laspkg` into a multi-well container
- the monorepo now carries an explicit `lithos-project` crate for this layer and a separate `lithos-ingest` crate boundary for import-oriented orchestration
- project-managed assets use a shared `AssetManifest` contract
- asset collections group related or versioned assets under one wellbore
- compute outputs fit the same pattern by persisting as derived sibling assets rather than mutating the source asset in place
- manual saves and imports now also produce immutable local asset revisions in a hidden project-local revision store while keeping the visible asset package path stable as the current head
- the catalog is for discovery and relationships; the package is the authoritative storage unit for the asset itself
- the first multi-well slice still focuses primarily on project creation, import, binding, search, and read/query, but it now includes bounded in-family edit sessions for the first structured asset families
- synthetic fixtures should validate the real import path and the project/catalog linkage rather than writing package internals directly
- richer reconciliation workflows, broader structured editing UX, and broader source support remain follow-on work
