# runtime

`runtime/` contains `seis-runtime`, the demo support runtime layer for TraceBoost workflows inside the Ophiolite repo.

## Stack And Formats

- Rust 2024 library crate: `seis-runtime`
- `ophiolite-seismic-runtime` as the shared seismic runtime core
- `ndarray` and `rayon` for CPU-first data handling and processing
- `serde` / JSON at the boundary to app-facing contracts

This layer wraps the shared Ophiolite seismic runtime and exposes the workflow-facing helpers used by `traceboost-demo`.

## Implemented

- SEG-Y preflight helpers on top of `seis-io`
- ingest into the canonical `tbvol` runtime store
- reopen/describe existing stores
- `VolumeDescriptor` and `DatasetSummary`-ready metadata for app surfaces
- section-view generation for inline/xline browsing
- processing entry points for:
  - versioned operator pipelines
  - whole-section preview
  - derived-volume materialization
  - lineage persisted into derived `tbvol` manifests

Shared fixtures live in `test-data/`.

## Roadmap

1. Keep the first app path stable:
   preflight -> ingest -> open -> section view.
2. Add small app-facing helpers around recent stores, error mapping, and session-friendly dataset summaries.
3. Keep TraceBoost-specific orchestration thin while the shared runtime evolves in Ophiolite.
4. Defer deeper cache/residency and backend GPU work until the first desktop workflow is proven and profiled.

## Constraints

- This layer owns the runtime-store contract, not the frontend.
- It is CPU-first today by design.
- The canonical runtime store is `tbvol`.
