# ADR-0030: Unified Operator Catalog and Seismic First-Class Registry

## Status

Accepted

## Decision

`ophiolite` will own one additive operator-catalog vocabulary for discovery across structured compute families and seismic families.

Phase 1 introduces:

- a shared `ophiolite-operators` crate for cross-family catalog types
- seismic-owned built-in operator registration in `ophiolite-seismic`
- project-owned catalog assembly through `OphioliteProject::list_operator_catalog(...)`
- compatibility preservation for `list_compute_catalog(...)`

The public execution model does not change in this slice:

- existing structured compute families remain immediate execution
- seismic materializing families remain canonically job-based
- sync convenience wrappers continue to live above the core boundary

## Why

The current operator surface is split across:

- typed structured compute catalogs in core/project code
- seismic contracts and runtime metadata in `ophiolite-seismic`
- TraceBoost command catalogs and compatibility wrappers

That split makes seismic discovery a special case and weakens the SDK surface.

Local inspection of `QGIS` is useful for the internal model:

- one registry
- stable ids
- provider/family metadata
- grouping and discoverability
- compatibility-aware filtering

Local inspection of `QuantLib` is useful for the public API shape:

- domain-first nouns
- stable abstractions
- no generic transport-shaped top-level API

The result is a hybrid target:

- QGIS-like internal registry/catalog metadata
- QuantLib-like family-specific public SDK surface

## Consequences

- `operator` becomes the umbrella discovery concept
- `compute` remains a compatibility and legacy sublanguage for existing structured asset workflows
- seismic operator discovery now belongs in core/project code instead of TraceBoost catalogs
- family-owned metadata stays with the owning crates:
  - `ophiolite-compute` for structured compute
  - `ophiolite-seismic` for trace-local, subvolume, gather, and analysis families

Phase 1 intentionally does not attempt:

- a generic `run_operator(...)` API
- frontend operator catalog UX
- third-party seismic runtime plugins
- a forced rename or removal of legacy compute APIs

## Migration Boundary

Phase 1 is additive:

1. add shared operator catalog types
2. add seismic built-in catalog definitions
3. adapt project-level discovery onto the new catalog
4. keep `list_compute_catalog(...)` as the compatibility path for one migration window

Execution entry points remain family-specific.

## Success Criteria

This decision is working when:

- `OphioliteProject` can list both structured compute and seismic operators through one catalog API
- seismic operator discovery no longer depends on TraceBoost command catalogs
- structured compute callers continue to work unchanged during the migration window
- operator entries carry stable ids, family metadata, contract refs, and availability information
