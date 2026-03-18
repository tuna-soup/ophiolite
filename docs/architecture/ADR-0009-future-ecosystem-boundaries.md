# ADR-0009: Future Ecosystem Boundaries

## Status

Accepted

## Decision

`lithos` keeps the current monorepo and local-first platform core as the present implementation center, while treating desktop interaction, compute, and sync/distribution as separate ecosystem concerns.

The current implementation center is:

- source import
- canonical log and typed wellbore asset models
- single-asset package conventions
- `LithosProject`
- package sessions for the mature log-editing path
- app-facing DTO/query boundaries

Future ecosystem layers are recognized, but not treated as part of the current core:

- desktop app/system of interaction
- compute/UDF layer/system of computation
- sync/distribution layer/system of distribution

## Why

- the repo already contains a strong local-first core and should keep hardening that first
- the app, compute, and sync concerns have different lifecycles and should not distort the core data model prematurely
- recognizing these future layers now helps keep boundaries clean without forcing an early repo split
- the roadmap needs one durable place to say that these layers are future direction rather than current architecture

## Consequences

- the monorepo remains the practical implementation home for now
- roadmap language should distinguish current core from later ecosystem expansion
- root-level speculative architecture notes are unnecessary once their remaining useful content is captured in the roadmap and ADRs
- future compute or sync work should build on the current core rather than being mixed into package/catalog logic prematurely
