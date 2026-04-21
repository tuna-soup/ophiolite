# ADR-0028: Domain-First Python SDK Surface and Advanced Namespaces

## Status

Accepted

## Decision

The primary Python builder surface for `ophiolite` stays intentionally small and domain-first.

The top-level package should center on stable subsurface nouns and a few durable workflow entry points:

- `Project`
- `Well`
- `Wellbore`
- `Survey`
- high-signal analysis helpers such as `avo_reflectivity(...)`

Lower-level transport, admin, and extension shapes move into explicit advanced namespaces instead of competing with the main builder story:

- `ophiolite_sdk.analysis`
- `ophiolite_sdk.operators`
- `ophiolite_sdk.platform`
- `ophiolite_sdk.interop`

Domain objects may expose convenience methods that delegate to Rust-owned platform operations as long as they preserve canonical meaning and do not introduce a second implementation in Python.

Examples:

- `Well.wellbores()`
- `Well.surveys()`
- `Well.panel(...)`
- `Wellbore.trajectory()`
- `Wellbore.panel(...)`
- `Survey.map_view(...)`
- `Survey.section_well_overlays(...)`

Compatibility re-exports may remain at the top level for one preview cycle with deprecation warnings.

## Why

Local inspection of `QuantLib` shows a stable public vocabulary built from domain nouns plus a small number of durable technical abstractions.

Representative examples in `ql/` are:

- domain families such as `instruments`, `indexes`, `cashflows`, and `termstructures`
- stable technical abstractions such as `Instrument`, `PricingEngine`, `Handle`, `Quote`, and `Schedule`

The local `QuantLib` tree does not present generic application nouns such as `Project`, `Asset`, `Contract`, `Request`, or `Response` as the primary organizing language for end users.

That pattern is useful for `ophiolite`:

- end users think in `Project`, `Well`, `Wellbore`, and `Survey`
- workflow builders need object navigation and reusable operators
- advanced users still need access to raw contracts, admin surfaces, and extension points
- exposing every transport or integration noun at the top level makes the public API feel less canonical and less teachable

`Project` remains a valid top-level noun for `ophiolite` even though it has no direct `QuantLib` analogue, because the local-first project root is part of the canonical workflow model in this stack.

## Consequences

- docs and examples should teach object-first workflows before raw request/response payloads
- Rust stays the owner of canonical behavior; Python remains a thin typed control surface
- new top-level Python exports should be added only when they are durable domain concepts or very high-signal workflow helpers
- transport-shaped DTOs, operation catalogs, operator authoring helpers, and compatibility models should default to advanced namespaces
- future API reviews should reject generic application nouns at the main package root unless they are clearly canonical for subsurface workflows

## Success Criteria

This decision is working when:

- new users can learn the Python SDK through `Project -> Well -> Wellbore -> Survey`
- advanced users can still reach admin, interop, and extension surfaces without ambiguity
- CLI, Rust, and Python continue to expose the same platform meanings without copying core logic into Python
