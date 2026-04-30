# Public seismic fixture manifests

This directory contains manifest scaffolds for public or curated seismic fixtures.
The manifests describe candidate fixture sources and small regions of interest
without storing bulk seismic data in the repository.

These files are intentionally domain-light. They record where a fixture could be
fetched from, how a test harness should treat fetches, which adapter family is
expected to read it, and what lightweight preview shape a future checked-in
snapshot may expose. They do not declare scientifically authoritative processing
warnings, quality flags, or interpretation outcomes.

## No bulk data policy

Do not commit SEG-Y, MDIO, ZGY, VDS, velocity volumes, gathers, or other large
seismic payloads here. The repository should only contain manifests, small
metadata snapshots, checksums, and human-readable notes that are suitable for
code review.

Downloaded fixture payloads should live outside the repository or under a local
ignored cache chosen by the fetch harness. Manifests may name cache keys or
relative snapshot file names, but they should not require checked-in bulk data.

## Fetch-gated behavior

Public data portals can be large, rate limited, license gated, login gated, or
temporarily unavailable. Tests that use these manifests should default to
offline behavior and only fetch data when an explicit environment flag or
command-line option enables network access.

Recommended policy values:

- `fetch_policy.default`: `offline`
- `fetch_policy.requires_opt_in`: `true`
- `fetch_policy.env_var`: the environment variable that permits network fetches
- `fetch_policy.cache_key`: a stable local cache key for fetched payloads

Offline tests may validate manifest structure and pre-recorded metadata
snapshots. Network-enabled tests may fetch source data, refresh snapshots, or
compute optional checksums.

## Manifest fields

- `schema_version`: Manifest schema version string.
- `id`: Stable fixture identifier. Use lowercase words separated by hyphens.
- `status`: Current manifest status, such as `example-placeholder`,
  `candidate`, or `curated`.
- `source`: Public source descriptor with `kind`, `uri`, and optional notes.
- `license_note`: Human-readable license or access note. This is not legal
  advice and should point readers to the upstream terms.
- `adapter_hint`: Suggested reader or adapter family, such as `mdio` or `segy`.
- `fetch_policy`: Offline-first network access policy.
- `subset`: Small region, line, trace, sample, or byte-range description that
  keeps tests bounded.
- `expected`: Placeholder expectations for canonical preview shape, warnings,
  and blockers. Example manifests must mark warnings and blockers as
  non-authoritative unless they are backed by a reviewed fixture contract.
- `checksums`: Optional hashes for fetched payloads or small metadata snapshots.
- `snapshots`: Optional paths to small metadata previews committed separately.

See `schema.example.json` for a representative manifest shape.
