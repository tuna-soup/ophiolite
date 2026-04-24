# Processing Lineage And Cache Compatibility Policy

## Purpose

This note makes the current canonical processing compatibility policy explicit.

It applies to:

- persisted `ProcessingLineage`
- cached runtime artifacts
- reuse validation
- processing output packages

## Compatibility States

- `Canonical`
  - readable
  - reusable
  - packageable
  - eligible for exact canonical cache validation

- `NormalizedLegacyReadable`
  - readable
  - not silently reusable
  - may be rewritten into canonical form by an explicit migration or rewrite step
  - may be inspected and surfaced in debug/UI as legacy-readable

- `LegacyReadableNoCanonicalReuse`
  - readable only
  - not reusable
  - not treated as canonical package input
  - kept for inspection, provenance, and operator continuity only

## Validation Outcomes

- readable and reusable
  - canonical semantic envelope is present
  - runtime/store-writer semantics versions match current canonical versions
  - `artifact_key` matches canonical derivation
  - `logical_domain`, `chunk_grid_spec`, and `geometry_fingerprints` match validated mirrors

- readable but not reusable
  - enough metadata exists to inspect lineage and show debug state
  - canonical identity is incomplete, downgraded, or version-incompatible
  - runtime must not silently resolve reuse from this artifact

- readable but should be rewritten
  - legacy fields can be normalized into the canonical envelope
  - rewrite is explicit and produces a fresh canonical artifact/cache/package identity

- reject
  - manifest/package/cache payload is structurally invalid
  - family/schema/semantics mismatch cannot be normalized safely
  - canonical mirrors disagree internally
  - packaged config and packaged store lineage disagree

## Operational Rules

- canonical identity changes invalidate by default through explicit semantics/version bumps
- cache lookup is family-aware and requires pipeline digest, artifact role, store format, and canonical lineage validation
- old artifacts may remain readable after an upgrade without remaining reusable
- package open is strict:
  - blob digests must match
  - config must match packaged lineage
  - packaged lineage must pass canonical validation

## Current Migration Rule

The repo currently prefers:

1. read legacy payloads when possible
2. classify them explicitly
3. refuse silent canonical reuse when identity is downgraded
4. rewrite explicitly when a canonical artifact is needed
