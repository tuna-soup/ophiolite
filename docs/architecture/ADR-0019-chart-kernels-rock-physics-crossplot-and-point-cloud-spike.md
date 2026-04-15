# ADR-0019: Chart Kernels, Rock-Physics Crossplot DTOs, and the First Point-Cloud Spike

## Status

Accepted

## Decision

`Ophiolite Charts` will expand through strict chart families that consume validated upstream display DTOs rather than raw project assets.

The architectural shape is:

- `ophiolite` owns canonical subsurface semantics, semantic eligibility, and binding resolution
- chart-facing rock-physics views are display DTOs rather than raw log/package payloads
- `Ophiolite Charts` owns chart-native adapted payloads, render kernels, modifiers, and wrappers
- render kernels are family-specific rather than one universal scene abstraction
- heavy geometry paths are WebGL2-first, with Canvas/HTML overlays for labels, legends, and readouts

The first new family in this structure is `RockPhysicsCrossplot`.

The first concrete template/preset is `VpVs_vs_AI`.

## Why

The current chart SDK already has a sound boundary rule:

- chart-native rendering belongs in `charts/`
- canonical subsurface meaning belongs upstream
- wrappers should adapt canonical DTOs into chart payloads rather than leak product/runtime details into renderers

That boundary is already visible in the current README and package split, but the internal architecture is still uneven:

- seismic already has a worker/WebGL2-oriented render path
- survey map and well-correlation still rely on separate Canvas-heavy renderers
- shared interactions exist, but shared family kernels do not yet

As chart scope expands into rock physics, generic line charts, and future point-cloud workflows, the SDK needs a stronger middle layer than "one renderer per chart".

## Decision Details

### 1. Strict chart families over raw generic charts

The SDK will expose strict chart families first:

- `SeismicSection`
- `SeismicGather`
- `SurveyMap`
- `WellPanel`
- `RockPhysicsCrossplot`

Generic chart families may exist later, but they do not replace strict family contracts.

### 2. Validated upstream DTOs

`RockPhysicsCrossplot` must consume a validated upstream DTO resolved from canonical Ophiolite data.

That DTO owns:

- template identity
- semantic eligibility
- curve/depth alignment
- derived channel resolution
- provenance bindings
- categorical/scalar color-policy decisions

`charts/` may perform lightweight invariant checks, but it is not the primary semantic resolver.

### 3. Composition over inheritance

The SDK will reuse behavior through composition:

- layers
- modifiers/plugins
- controllers
- chart-native view models

It will not rely on deep inheritance trees like "BaseChart -> XYChart -> RockPhysicsChart -> VpVsAiChart".

### 4. Family kernels

The rendering middle layer becomes explicit:

- `RasterTraceKernel`
- `WellPanelKernel`
- `PointCloudKernel`

Those kernels share lower-level primitives for:

- GPU buffer/texture upload
- picking/highlighting
- axes and plot geometry helpers
- crosshair/probe/lasso modifiers
- legends and annotation scaffolding
- worker/offscreen scheduling patterns where needed

### 5. Rock-physics template policy

`RockPhysicsCrossplot` remains strict even inside the broader rock-physics family.

Templates define exact axis-role and color-role policies.

Initial templates:

- `VpVs_vs_AI`
- `Phi_vs_AI`
- `PR_vs_AI`
- optional `Vp_vs_Density`

For example, `VpVs_vs_AI` means:

- X axis: `AcousticImpedance`
- Y axis: `VpVsRatio`
- color: constrained categorical/scalar roles only

### 6. Canonical semantics

Rock-physics charts depend on canonical semantic types rather than loose strings.

`Vp/Vs` is treated as a first-class canonical semantic, alongside types such as:

- `PVelocity`
- `SVelocity`
- `AcousticImpedance`
- `BulkDensity`
- `PoissonsRatio`
- `NeutronPorosity`
- `WaterSaturation`
- `VShale`

### 7. Point-cloud payloads

The chart-native adapted point-cloud payload is columnar.

It should use typed arrays for heavy per-point state:

- `x`
- `y`
- continuous color values and/or categorical ids
- symbol category ids where enabled
- well indices
- sample/depth references

Object-per-point payloads are not the primary runtime format.

### 8. Performance policy

Point-cloud charts must support explicit interaction/render policy:

- `interactionQuality: "auto" | "exact" | "progressive"`
- `renderQuality: "auto" | "quality" | "performance"`

Large datasets may use progressive interaction modes, but the mode must be explicit rather than hidden.

## Consequences

- `charts/` keeps ownership of chart-native APIs and rendering behavior
- canonical semantic drift stays upstream rather than leaking into chart wrappers
- new chart families have a cleaner landing zone than adding another one-off renderer stack
- the first implementation can prove the kernel concept with a narrow spike before broader migration
- future generic point-cloud/XY charts can reuse `PointCloudKernel` without weakening `RockPhysicsCrossplot`

## Immediate Implementation Plan

### 1. Document and scaffold now

Add:

- this ADR
- a chart-facing rock-physics crossplot model in `packages/data-models`
- a mock/synthetic model generator for spike/demo use
- a narrow point-cloud spike renderer for demos and benchmarks

### 2. Ship a first spike

The first spike is intentionally narrower than the final product:

- template: `VpVs_vs_AI`
- point geometry: fixed-size point cloud
- color modes: constrained categorical/continuous options
- no authored editing
- no production picking contract yet

### 3. Add demo and benchmark surfaces

The first spike must be visible in the repo:

- add a new Vp/Vs vs AI rock-physics demo alongside the existing chart demos
- extend the benchmark app with a point-cloud setup/render path

The demo is a required part of the rollout, not optional follow-on work.

### 4. Preserve the migration order

After the point-cloud spike is stable:

1. migrate/expand `SurveyMap` toward shared point/line primitives
2. migrate `WellPanel` surfaces where shared primitives pay off
3. migrate selected `Seismic` pieces only where the new kernel layer is genuinely helpful

## Non-Goals

This ADR does not yet define:

- the exact upstream Ophiolite contract type name for the resolved rock-physics DTO
- the full production `PointCloudKernel` API
- authored interpretation editing for crossplots
- a final public Svelte wrapper for `RockPhysicsCrossplot`
- full benchmark automation/gating infrastructure

Those belong to the next implementation slices after the first spike and DTO stabilization.

## Follow-On Work

The next implementation phases should add:

- generated contract types for resolved rock-physics chart DTOs
- public `RockPhysicsCrossplotChart` wrappers
- point-cloud picking and selection contracts
- exact/progressive interaction switching in the runtime kernel
- stronger benchmark scenarios for recolor, filter, and selection latency
