# ADR-0009: Future Ecosystem Boundaries

## Status

Accepted

## Decision

`lithos` keeps the current monorepo and local-first platform core as the present implementation center, while treating desktop interaction, compute, and sync/distribution as separate ecosystem concerns with distinct boundaries.

The current implementation center is:

- source import
- canonical log and typed wellbore asset models
- single-asset package conventions
- `LithosProject`
- package sessions for the mature log-editing path
- app-facing DTO/query boundaries
- the logs-first typed compute layer

Future ecosystem layers are recognized, but not all are part of the present core to the same degree:

- desktop app/system of interaction
- sync/distribution layer/system of distribution

The compute/UDF layer now exists in the monorepo and current architecture, but it remains intentionally separated from package/catalog storage concerns and should continue to evolve as a distinct layer above the core data model.

## Why

- the repo already contains a strong local-first core and should keep hardening that first
- the app, compute, and sync concerns have different lifecycles and should not distort the core data model prematurely
- future sync/distribution work should start from simple replication and export/import workflows rather than conflict-resolution-heavy collaboration models
- recognizing these future layers now helps keep boundaries clean without forcing an early repo split
- the roadmap needs one durable place to distinguish current implementation from later ecosystem expansion

## Consequences

- the monorepo remains the practical implementation home for now
- roadmap language should distinguish current core from later ecosystem expansion
- root-level speculative architecture notes are unnecessary once their remaining useful content is captured in the roadmap and ADRs
- compute and future sync work should build on the current core rather than being mixed into package/catalog logic prematurely
