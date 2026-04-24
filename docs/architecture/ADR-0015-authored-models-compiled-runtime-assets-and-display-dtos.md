# ADR-0015: Authored Models, Compiled Runtime Assets, Analysis APIs, and Display DTOs

## Status

Accepted

## Decision

`ophiolite` will distinguish five different families when expanding beyond basic ingest and trace-local processing:

- `Source Assets`
- `Authored Models`
- `Compiled Runtime Assets`
- `Analysis APIs`
- `Display DTOs`

This classification applies across seismic, wells, horizons, velocity modeling, future property modeling, and later earth-modeling workflows.

Phase 1 stays in the current crate split:

- canonical/source/authored-model contracts continue to live in `ophiolite-seismic`
- runtime builders, compilers, and fast query paths continue to live in `ophiolite-seismic-runtime`
- application-facing DTO/query surfaces continue to be exposed through existing Ophiolite and product-facing boundaries

For authored processing pipelines specifically, inspectability is now an explicit boundary concern:

- `ophiolite-seismic` owns the stable inspectable plan contract (`InspectableProcessingPlan`)
- `ophiolite-seismic-runtime` may continue to own executor-facing internal plan structs
- app-facing compatibility summaries such as `ProcessingJobPlanSummary` are derived projections of the stable inspectable contract, not the primary source of truth
- planner-only execution hints belong to a backend-owned registry next to operator metadata rather than app code or runtime-local ad hoc matches
- runtime planning should be structured as named passes with optional per-pass snapshots rather than one opaque planner sweep
- mixed-family pipelines that carry a trace-local prefix should plan and execute as explicit prefix checkpoint stages plus a final family stage, instead of collapsing back into a single opaque materialization step
- the stable shared processing/debug contract also carries semantic/version envelopes, typed plan and reuse decisions, runtime-stage snapshots, runtime events, and section-debug records rather than leaving those as app-local debug inventions
- compiled processing outputs must carry canonical artifact identity, logical-domain, chunk-grid, geometry-fingerprint, and lineage semantics so runtime assets, cache reuse, packages, and debug inspection all refer to the same derived artifact model

We are not introducing a separate `ophiolite-modeling` crate yet, but new contracts should be designed so that a future split remains clean.

## Why

As the platform expands, there is a recurring temptation to collapse several distinct concerns into one generic "processing" or "operator" bucket. That does not hold up once the system needs to support:

- imported source data with provenance
- user-authored layered or horizon-guided earth models
- compiled runtime fields and transforms optimized for section/map queries
- analysis requests that estimate or inspect without becoming source-of-truth assets
- frontend/rendering DTOs that should not become canonical storage models

Velocity modeling is the clearest immediate example:

- sparse control profiles, velocity cubes, and well-local time-depth inputs are source assets
- layered velocity models and future horizon-guided property models are authored models
- `SurveyTimeDepthTransform3D` is a compiled runtime asset
- velocity scan and similar diagnostics are analysis APIs
- resolved section overlays and map previews are display DTOs

Without this taxonomy, contracts and runtimes drift toward a single overburdened model that is hard to validate, hard to evolve, and easy to misuse in frontend code.

## Consequences

- not every future computation is modeled as an operator
- authored earth models remain distinct from processing pipelines
- compiled runtime assets remain distinct from the editable/authored model they came from
- chart-facing DTOs remain transport/rendering shapes rather than canonical domain truth
- provenance and import semantics stay attached to source assets instead of being erased by later build steps
- inspectable processing/debug DTOs remain canonical shared contracts; product apps should render them rather than reconstructing execution meaning locally
- compiled processing artifacts now have stable identity and compatibility semantics that survive across runtime, cache, package, and app boundaries

## Classification Rule

When adding a new feature, classify it before defining contracts:

- if it maps traces to traces on unchanged geometry, it belongs in a processing operator family
- if it estimates or inspects without becoming the source of truth, it belongs in an analysis API family
- if it authors or refines an earth model from multiple inputs, it belongs in an authored-model family
- if it compiles an authored model into a runtime-ready field/transform/grid, it belongs in a compiled runtime asset family
- if it only exists to drive app/chart rendering, it belongs in a display DTO family

## Examples

### Source Assets

- seismic amplitude volume
- imported horizon XYZ
- sparse velocity control profiles
- well checkshots / VSP / sonic / Vp logs
- imported 3D velocity cube

### Authored Models

- `LayeredVelocityModel`
- future horizon-guided property model
- future facies/property interpolation model

### Compiled Runtime Assets

- `SurveyTimeDepthTransform3D`
- future survey property field/grid
- future depth-converted derived seismic store

### Analysis APIs

- velocity scan / semblance
- transform coverage diagnostics
- property-model QC summaries

### Display DTOs

- resolved section display bundles
- section scalar overlays
- survey map previews

## Implementation Guidance

- frontend and chart packages must not own authored-model math
- application repos may own workflow, activation, and diagnostics for authored models and compiled outputs
- backend/runtime layers own CRS, geometry compatibility, coverage checks, and compilation/build rules
- backend/shared contract layers own inspectable authored-model-to-runtime lowering surfaces; apps should consume them rather than reconstruct planner semantics locally
- TraceBoost and other apps should treat checkpoint reuse, lineage rewriting, and stage reconstruction as backend/runtime concerns and render the returned contracts rather than rebuilding them
- backend/runtime layers also own canonical processing identity derivation, reuse validation, package compatibility checks, and runtime/debug event production
- product apps should treat realized reuse, runtime policy divergence, and section-assembly debugging as returned backend-owned semantics rather than deriving them from app-local heuristics
- source assets, authored models, compiled outputs, and display DTOs must not be collapsed into one contract type

See also:

- `ADR-0031-shared-seismic-execution-planner-and-bounded-local-job-service.md`
- `ADR-0032-processing-authority-and-thin-client-migration.md`
- `ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`

## Follow-On Work

The first authored-model family in this taxonomy is velocity/property modeling:

- `VelocityControlProfileSet`
- `LayeredVelocityModel`
- `VelocityIntervalTrend`
- `BuildSurveyTimeDepthTransformRequest`
- `BuildSurveyPropertyFieldRequest`

Later expansions such as broader property modeling, horizon-guided interval trends, and other earth-modeling computations should follow the same taxonomy rather than creating ad hoc compute families.
