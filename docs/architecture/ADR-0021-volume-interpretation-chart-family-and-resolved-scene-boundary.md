# ADR-0021: Volume-Interpretation Chart Family and Resolved Scene Boundary

## Status

Accepted

## Decision

`Ophiolite Charts` will add a new `volume-interpretation` chart family for 3D seismic interpretation workspaces.

The durable boundary is:

- `ophiolite` owns canonical seismic volumes, horizons, well trajectories, markers, and interpretation workflows
- backend/app layers resolve those canonical assets into an explicit 3D interpretation scene DTO
- `Ophiolite Charts` owns chart-native scene payloads, controller state, runtime styling, interactions, rendering, and demo surfaces
- dense numeric payloads stay out of the chart manifest and should move through packed/binary payload paths as the production transport matures
- orthogonal slice planes are the precision interaction surface
- optional volume rendering is context rather than the primary authored-editing surface

The first implementation slice is browser-native and chart-scoped:

- one active seismic volume
- inline/xline/sample slice planes
- structured horizon surfaces
- well trajectories and markers
- crop-box display
- explicit tool modes and emitted interaction signals
- isolated demo coverage

## Why

This matches the direction already established in the chart workspace:

- strict chart families over generic scene abstractions
- backend-owned semantic resolution
- frontend-native chart payloads and render kernels
- WebGL2-first heavy rendering paths with HTML/Canvas/Svelte overlay surfaces

The current repo already follows this shape for:

- section display DTOs and chart-native seismic payloads
- project-aware well-on-section overlay DTOs
- survey-map resolved sources
- rock-physics point-cloud crossplots

If a 3D interpretation view bypassed those boundaries, it would collapse:

- canonical asset ownership
- app/workflow transport concerns
- chart-native runtime styling
- renderer-only scene concerns

into one model.

## Exact Boundary

### 1. Canonical source family

The upstream family is a resolved 3D interpretation scene, not raw seismic or well assets.

Recommended semantic source identity:

- `ophiolite-volume-interpretation-source`

That source may be resolved from:

- one active seismic volume
- selected horizon assets or derived interpretation layers
- resolved well trajectory geometry
- marker/pick/annotation assets
- active interpretation-workflow context

### 2. Chart-facing source contract

The chart-facing manifest is explicit and domain-shaped rather than a generic scene graph.

Recommended first-pass shape:

```ts
type ResolvedVolumeInterpretationSourceDto = {
  schema_version: number;
  id: string;
  name: string;
  sample_domain: "time" | "depth";
  scene_bounds: Box3Dto;
  volumes: VolumeSceneItemDto[];
  slice_planes: SlicePlaneSceneItemDto[];
  horizon_surfaces: HorizonSurfaceSceneItemDto[];
  well_trajectories: WellTrajectorySceneItemDto[];
  markers: SceneMarkerDto[];
  annotations: SceneAnnotationDto[];
  interpretation_context: InterpretationContextDto | null;
  capabilities: VolumeInterpretationCapabilitiesDto;
};
```

Rules:

- the manifest owns ids, semantic roles, scene membership, defaults, and capabilities
- dense geometry/sample payloads should be referenced, not inlined, once production transport arrives
- runtime styling remains chart-native and may override display defaults without changing canonical asset types

### 3. Chart-native payload

`charts/` adapts the resolved source into a chart-native payload focused on:

- scene bounds
- active view state
- slice-plane runtime state
- horizon/well/marker runtime styles
- interaction targets
- selection/probe state
- interpretation request surfaces

This payload is not the canonical Ophiolite DTO.

### 4. Renderer boundary

The renderer kernel is family-specific:

- `volume-interpretation`

Rules:

- renderer adapters may use specialized 3D libraries internally
- renderer internals do not become the public chart model
- controller code owns tool semantics, state transitions, and emitted events
- renderers own low-level drawing, projected hit primitives, and pick results

### 5. Interaction boundary

The 3D chart exposes explicit tool modes rather than inferring intent from cursor context alone.

Initial tool set:

- `pointer`
- `orbit`
- `pan`
- `slice-drag`
- `crop`
- `select`
- `interpret-seed`

Initial public actions:

- `fitToData`
- `resetView`
- `centerSelection`

Initial semantic event surfaces:

- probe change
- selection change
- slice-plane change
- view-state change
- interaction-state change
- interaction event
- interpretation request

### 6. Interpretation workflow ownership

Interpretation workflows remain backend-owned.

Frontend responsibilities:

- author seeds/constraints
- emit interpretation requests
- visualize job state and returned patches
- update chart-native runtime state

Backend responsibilities:

- autotrack/interpretation math
- provenance
- async job lifecycle
- horizon/marker patch generation
- geometry transforms and resolved display-space coordinates

## Phase 1 Scope

Phase 1 proves the family shape rather than the full production transport:

- one active volume
- orthogonal slices only
- no oblique slicing
- crop-box display
- structured horizons
- line/tube well styles
- marker display
- isolated demo
- stub interpretation request/update loop

Phase 1 does not require:

- streaming/bricking
- full production packed binary transport
- mandatory volume ray casting
- authored mesh editing on arbitrary 3D surfaces
- well-log drape rendering

## Transport Guidance

As this family moves into app/backend integration:

- keep the scene manifest explicit and semantic
- deliver dense payloads through packed/binary payload paths
- prefer patch updates for local horizon refresh over full-scene replacement

Recommended patch families:

- `horizon-surface-patch`
- `horizon-contour-refresh`
- `marker-state-patch`
- `job-state-patch`

## Consequences

- the chart workspace gains a clean landing zone for 3D interpretation workflows
- the repo keeps the same canonical/app/chart separation already used elsewhere
- the first implementation can be an isolated demo without prematurely committing to a final production transport
- future renderer upgrades, including optional richer volume rendering, stay behind the family kernel boundary rather than leaking through the chart model

## Non-Goals

This ADR does not yet define:

- the final upstream Rust/TypeScript DTO names
- the final packed binary payload format for volume and surface transport
- the exact production renderer-library choice for every scene primitive
- streaming/bricking contracts
- authored oblique slicing
- multi-volume compositing

Those belong to the next implementation slices after the family scaffold is stable.
