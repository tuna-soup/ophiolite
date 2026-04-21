# Contract Architecture And Migration Plan

## Audience and intent

This note is for engineers working across `Ophiolite`, `TraceBoost`, and `Ophiolite Charts` who need a contract system that can grow beyond seismic-only section workflows.

It documents:

- what is good about the current contract organization
- where it is still biased toward the original seismic-first product shape
- a target taxonomy for long-term growth
- how to migrate there without a destabilizing rewrite

This is not a proposal to replace the current contracts immediately. It is a proposal to stop reinforcing the current accidental taxonomy as the permanent one.

## Current state

Today the TraceBoost contract layer is split into three crates:

- `seis-contracts-core`
- `seis-contracts-views`
- `seis-contracts-interop`

That split is directionally correct because it distinguishes shared types, display-facing types, and app/workflow request-response payloads.

But the current shape still reflects how the product started:

- the root naming is explicitly seismic-owned
- most shared contract ownership still lives in `ophiolite_seismic`
- view contracts are dominated by section and gather payloads
- interop mixes app-shell state, seismic operations, and project/map DTOs
- transport-specialized payloads such as packed section responses live in the frontend bridge layer rather than a first-class transport boundary

That means the contract system is not yet wrong, but it is also not yet the taxonomy we want to scale with.

## Architectural problem

TraceBoost is already drifting beyond a pure seismic-section application:

- imported horizons
- section well overlays
- survey maps
- project trajectories
- time-depth assets

The long-term product direction is broader still:

- additional non-seismic data families
- new computation families that are not section-centric
- new Ophiolite Charts charts that are not "seismic section with overlays"

If the contract system remains organized around the original seismic entrypoint, teams will keep doing the easy thing:

- put non-seismic concepts into seismic buckets
- put chart-specific payloads into generic "views"
- put wire-format choices into UI bridge code

That is the drift to stop.

## Design goals

The contract system should make four distinctions explicit.

### 1. Domain vs operation

These are different.

- Domain contracts describe stable concepts: dataset identity, horizons, trajectories, well markers, survey geometry, velocity models.
- Operation contracts describe workflows: import, preflight, resolve, process, export, preview.

### 2. Display view vs canonical domain shape

These are different.

- A horizon asset is not the same thing as a section-horizon overlay.
- A seismic volume is not the same thing as a section display payload.
- A trajectory asset is not the same thing as the points needed by one chart.

### 3. Transport vs semantic contract

These are different.

- `SectionView` describes the meaning of a resolved section payload.
- `TransportSectionView` describes how that payload is carried efficiently over a specific boundary.

Transport specialization should not be mistaken for domain modeling.

### 4. Chart-facing payload vs app API

These are different.

- A chart payload is a rendering-oriented resolved view.
- An app API payload is a request-response contract between subsystems.

The system should not force every chart payload to masquerade as an API primitive or vice versa.

## Target taxonomy

The recommended long-term shape is:

```text
contracts/
  domain/
    seismic/
    horizons/
    wells/
    trajectories/
    survey-map/
    shared/
  operations/
    datasets/
    import/
    processing/
    preview/
    resolve/
    export/
    workspace/
  views/
    section/
    gather/
    map/
    well-panel/
    shared/
  transport/
    packed-binary/
    tauri-ipc/
    http/
```

This is a taxonomy by responsibility, not by current crate count.

The key point is the ownership model:

- `domain` owns stable cross-system concepts
- `operations` owns request-response workflows
- `views` owns chart/display-oriented resolved payloads
- `transport` owns wire-shape adaptations and packing decisions

## What belongs where

### Domain

Examples that belong in domain contracts:

- `DatasetId`
- `VolumeDescriptor`
- `GeometryDescriptor`
- `ImportedHorizonDescriptor`
- `LayeredVelocityModel`
- well/trajectory/map identity and geometry descriptors

These are not UI-only and not operation-specific.

### Operations

Examples that belong in operation contracts:

- `ImportDatasetRequest` / `ImportDatasetResponse`
- `SurveyPreflightRequest` / `SurveyPreflightResponse`
- `OpenDatasetRequest` / `OpenDatasetResponse`
- processing run/cancel/get-status requests
- workspace save/load/update requests

These describe actions and their results.

### Views

Examples that belong in view contracts:

- `SectionView`
- `ResolvedSectionDisplayView`
- `GatherView`
- `PreviewView`
- `GatherPreviewView`
- `ResolvedSurveyMapSourceDto`
- well panel resolved display payloads

These are not canonical assets. They are resolved shapes prepared for display or interaction.

### Transport

Examples that belong in transport contracts:

- packed section-response headers
- packed resolved-section-display headers
- `TransportSectionView`
- `TransportResolvedSectionDisplayView`

These exist because the wire path has different needs:

- compact payloads
- binary packing
- Tauri or HTTP compatibility
- minimal JSON overhead

## Current-to-target mapping

The migration should start by treating the current modules as inputs to a better taxonomy.

### Current `seis-contracts-core`

Keep temporarily, but gradually split conceptually into:

- `domain/seismic`
- `domain/shared`
- `operations/processing` for processing-specific request/response types that do not belong in shared domain

Likely moves:

- keep in domain:
  - `DatasetId`
  - `VolumeDescriptor`
  - `GeometryDescriptor`
  - `ImportedHorizonDescriptor`
  - `SampleDataFidelity`
- move out of core into operation or view areas:
  - preview payloads
  - processing request/response contracts
  - chart-display-specific payloads

### Current `seis-contracts-views`

This should become a real display-view surface rather than a seismic chart bucket.

Split conceptually into:

- `views/section`
- `views/gather`
- `views/map`
- `views/well-panel`
- `views/shared`

Likely moves:

- `SectionView`, `ResolvedSectionDisplayView` -> `views/section`
- `GatherView`, `GatherPreviewView` -> `views/gather`
- map and panel DTOs should live beside other view contracts rather than being stranded inside unrelated modules

### Current `seis-contracts-interop`

This is the noisiest bucket today.

It currently mixes:

- app-shell/workspace payloads
- import/open/preflight requests
- map and project DTOs
- processing operations

Split conceptually into:

- `operations/datasets`
- `operations/import`
- `operations/resolve`
- `operations/processing`
- `operations/workspace`

That makes ownership much easier to reason about.

### Current frontend bridge transport types

These should stop being treated as ad hoc frontend helpers.

Move conceptually into:

- `transport/packed-binary`

That keeps the distinction clean:

- `SectionView` is the semantic contract
- `TransportSectionView` is the packed transport adaptation

## Boundary rules

To avoid future drift, use these rules.

### Rule 1: domain contracts must not be chart-specific

If a type only exists because one chart needs it, it is not domain.

### Rule 2: view contracts must not be mistaken for canonical assets

If a payload has display defaults, rasterized overlays, viewport-derived subsets, or packed section bytes, it is a view.

### Rule 3: transport contracts must not become the semantic API

Packed binary headers and `Uint8Array`-friendly payloads are wire shapes, not business shapes.

### Rule 4: operations should be named by user/system action

Examples:

- `ImportHorizonXyzRequest`
- `ResolveSurveyMapRequest`
- `RunTraceLocalProcessingRequest`

That keeps the action boundary explicit.

### Rule 5: new data families should get their own domain ownership early

Do not wait until the fifth well-marker or trajectory type before creating a domain boundary for them.

## How this applies to Ophiolite Charts

`Ophiolite Charts` should consume resolved view payloads or chart-facing data models, not storage models and not operation requests.

The intended pipeline is:

```text
canonical asset / runtime state
  ->
domain contracts
  ->
resolve / load / preview operation
  ->
view contract
  ->
transport adaptation if needed
  ->
frontend adapter
  ->
Ophiolite Charts chart model
```

That means:

- `tbvol` is not a chart contract
- `SectionView` is a chart-facing resolved view contract
- `TransportSectionView` is a wire-format variant
- Ophiolite Charts' internal chart model still remains separate from both

This separation is especially important once new chart types arrive that do not look like section/gather charts.

## Migration plan

The migration should be incremental.

### Phase 0: document the taxonomy

Do this now.

- keep current crates stable
- stop treating current names as the final architecture
- make the target ownership model explicit

### Phase 1: split Ophiolite contract ownership by concern

Before renaming crates, break up the large contract surfaces inside Ophiolite.

Start with:

- seismic domain descriptors
- seismic view payloads
- import/open/preflight operations
- processing operations

This reduces future rename pain.

Implemented state:

- `ophiolite_seismic::contracts` now lives as a directory module split into:
  - `domain.rs`
  - `processing.rs`
  - `models.rs`
  - `views.rs`
  - `operations.rs`
- the crate root and `contracts::mod.rs` still re-export the flat surface so existing consumers keep compiling while the taxonomy becomes explicit

### Phase 2: stop mixing semantic and transport contracts

Promote packed payload definitions out of frontend bridge implementation details and into an explicit transport layer.

This can still be generated or hand-maintained, but it should be named and owned as transport.

Implemented state:

- packed section and preview transport shapes now live in `TraceBoost/app/traceboost-frontend/src/lib/transport/packed-sections.ts`
- `bridge.ts` re-exports those transport helpers for compatibility, but parsing logic no longer sits inline with unrelated bridge responsibilities

### Phase 3: introduce non-seismic domain groupings as first-class citizens

As horizons, wells, trajectories, markers, and map assets grow, give them dedicated domain modules instead of attaching them to seismic sections by default.

### Phase 4: rename package surfaces when the new ownership is real

Do not start with a cosmetic rename.

Only rename the contract package boundaries once the internal ownership split exists.

Possible eventual shapes include:

- `tb-contracts-domain`
- `tb-contracts-operations`
- `tb-contracts-views`
- `tb-contracts-transport`

Or a similar monorepo-local naming scheme.

The exact names matter less than the ownership model.

## Recommended immediate changes

These are the next practical steps.

1. Treat the current `seis-contracts-*` split as transitional, not final.
2. Break `ophiolite_seismic::contracts` into smaller modules by concern before adding more unrelated types.
3. Add an explicit transport-contract area for packed binary section/gather payloads.
4. When adding new data families, require a decision first:
   - domain
   - operation
   - view
   - transport
5. Avoid using `SectionView` or other section-shaped contracts as templates for unrelated chart types.

## Decision summary

The current contract organization is serviceable for the present product, but it should not be treated as the long-term architecture.

The main strategic change is not "more contracts." It is better ownership:

- stable concepts in domain
- workflows in operations
- display payloads in views
- wire shapes in transport

That is the separation the platform will need once TraceBoost becomes broader than seismic-section workflows.
