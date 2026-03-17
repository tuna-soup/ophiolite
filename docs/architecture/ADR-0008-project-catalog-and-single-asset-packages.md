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

The initial multi-well implementation is intentionally read-mostly and currently makes log assets the first project-managed family. The near-term typed asset family direction is:

- log assets
- trajectory assets
- tops assets
- pressure observation assets
- drilling observation assets

## Why

- the existing package/session model is already strong and should not be discarded for multi-well support
- multi-well applications need discovery, binding, and cross-asset organization that do not belong inside one package
- SQLite is a pragmatic local-first fit for identities and relationships
- the architecture needs an explicit grouping layer between wellbore and individual asset instances
- logical asset identity must be separated from storage/package identity to support reimport, supersession, and versioned deliveries cleanly

## Consequences

- `LithosProject` becomes the multi-well entry point without turning `.laspkg` into a multi-well container
- project-managed assets use a shared `AssetManifest` contract
- asset collections group related or versioned assets under one wellbore
- the catalog is for discovery and relationships; the package is the authoritative storage unit for the asset itself
- the first multi-well slice focuses on project creation, import, binding, search, and read/query rather than rich edit sessions for every asset family
- non-log asset ingestion and richer reconciliation workflows remain follow-on work
