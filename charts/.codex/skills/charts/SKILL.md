---
name: charts
description: Use when working on Ophiolite Charts wrappers, shared presentation tokens, interaction plumbing, stage sizing, overlay anchors, or embedder-facing chart APIs. Covers the layered architecture between data models, chart-core, controllers/renderers, Svelte wrappers, and consuming apps, with emphasis on centralized styling, high-DPI rendering, and wrapper-owned chrome.
---

# Charts

Build `Ophiolite Charts` as an embedder-facing SDK. The playground, benchmarks, and product apps are consumers, not the API boundary.

## Architecture

- Keep concerns in the highest reusable layer that fits:
  - app contracts and transport
  - `packages/data-models`: payloads, registry, shared interaction and axis types
  - `packages/chart-core`: reusable presentation, geometry, layout, and shared chart logic
  - `packages/domain-geoscience`: controller state and semantic interaction handling
  - `packages/renderer`: raster and GPU drawing kernels
  - `packages/svelte`: embedder-facing wrappers, DOM/SVG overlays, and app callbacks
- Do not solve a family-wide problem in one wrapper or renderer file if `chart-core` or `data-models` can own it.

## Styling And Presentation

- Shared presentation belongs in `packages/chart-core`, not in wrappers or renderers.
- Traditional 2D cartesian charts should start from:
  - [packages/chart-core/src/cartesian-presentation.ts](../../../packages/chart-core/src/cartesian-presentation.ts)
  - [packages/chart-core/src/cartesian-axis.ts](../../../packages/chart-core/src/cartesian-axis.ts)
  - [packages/chart-core/src/probe-panel-presentation.ts](../../../packages/chart-core/src/probe-panel-presentation.ts)
- Seismic section and gather presentation is centralized in:
  - [packages/chart-core/src/seismic-presentation.ts](../../../packages/chart-core/src/seismic-presentation.ts)
  - [packages/chart-core/src/seismic-axis.ts](../../../packages/chart-core/src/seismic-axis.ts)
- Rock-physics and AVO charts should extend existing cartesian profiles instead of introducing local spacing and typography constants.
- Probe/readout box styling should stay centralized. Reuse the probe-panel presentation system before inventing chart-local boxes.

## High-DPI Raster Standard

- Raster chart surfaces should be device-pixel-ratio aware.
- Reuse [packages/renderer/src/internal/rasterSurface.ts](../../../packages/renderer/src/internal/rasterSurface.ts) for:
  - clamped DPR resolution
  - backing-store sizing
  - logical-to-raster transform setup
  - plot-rect scaling for WebGL/worker paths
- Current standard:
  - use CSS-space coordinates for controller and wrapper layout
  - scale the backing store internally
  - clamp DPR to `2` unless there is a strong reason to change it
- When a chart mixes local canvas, local WebGL, and worker WebGL, keep all paths on the same raster-surface helper so they do not drift.

## Renderer Vs Wrapper Chrome

- The renderer should own dense data drawing and interaction affordances that must stay aligned with the sampled data:
  - heatmaps, wiggles, overlays baked into the plot
  - crosshair lines
  - zoom-rect previews
  - horizon and well geometry
- The wrapper should own sparse chrome and text that benefits from crisp DOM/SVG rendering:
  - titles
  - axis labels and tick labels
  - probe/readout panels
  - split handles
  - scrollbars
  - anchor-mounted UI
- For seismic section and gather, the current standard is wrapper-owned SVG axis/title chrome via [packages/svelte/src/SeismicAxisOverlay.svelte](../../../packages/svelte/src/SeismicAxisOverlay.svelte), backed by shared logic in `chart-core`.
- For well correlation / well panel, the current standard is wrapper-owned scroll and SVG chrome via:
  - [packages/chart-core/src/well-correlation-chrome.ts](../../../packages/chart-core/src/well-correlation-chrome.ts)
  - [packages/svelte/src/WellCorrelationAxisOverlay.svelte](../../../packages/svelte/src/WellCorrelationAxisOverlay.svelte)
  - [packages/svelte/src/WellCorrelationPanelChart.svelte](../../../packages/svelte/src/WellCorrelationPanelChart.svelte)
- Dense well-panel data stays in the renderer. Depth ticks, track headers, probe panels, and scrollbars belong in the wrapper unless a non-wrapper fallback is explicitly required.
- If a renderer still needs an internal fallback for non-wrapper consumers, keep it internal and optional. Example: `MockCanvasRenderer` supports internal `axisChrome` control so wrappers can disable renderer-drawn axis chrome without changing the public chart API.

## Interaction Standard

- Shared interaction vocabulary lives in [packages/data-models/src/interactions.ts](../../../packages/data-models/src/interactions.ts).
- Chart-level tool availability lives in [packages/data-models/src/chart-registry.ts](../../../packages/data-models/src/chart-registry.ts).
- Wrapper-facing capabilities in [packages/svelte/src/types.ts](../../../packages/svelte/src/types.ts) should derive from the registry, not be invented ad hoc.
- Controller-backed charts should use [packages/chart-core/src/interaction-manager.ts](../../../packages/chart-core/src/interaction-manager.ts) or the equivalent controller pattern already established in the family.

### Standard 2D Behavior

- Default tool vocabulary for standard 2D families: `pointer`, `crosshair`, `pan`
- `crosshair` is a modifier/toggle, not its own zoom mode
- `Shift + left drag` starts rectangular zoom
- Plain drag pans only when the active tool is `pan`
- Right-click inside the plot zooms out around the cursor
- Right-click on an axis band should trigger typed axis-context hooks, not a chart-owned dialog

## Layout And Overlays

- The wrapper owns intrinsic stage size. Do not stretch the rendered chart surface to fill arbitrary parent space.
- If the host is smaller, allow overflow and scrollbars. If larger, anchor top-left.
- For horizontally wide families such as well correlation, the wrapper should own the scroll viewport and content lane, while the renderer draws against the full intrinsic content width.
- Keep stage sizing centralized:
  - seismic: [packages/svelte/src/seismic-stage.ts](../../../packages/svelte/src/seismic-stage.ts)
  - AVO: [packages/svelte/src/avo-stage.ts](../../../packages/svelte/src/avo-stage.ts)
- Preserve the named anchor model from [packages/svelte/src/types.ts](../../../packages/svelte/src/types.ts): `stageTopLeft`, `plotTopCenter`, `plotTopRight`, `plotBottomRight`, `plotBottomLeft`
- App overlays should attach through anchors rather than arbitrary absolute positioning.

## Controller And Wrapper Lifecycle

- Prefer a single controller owner inside the wrapper.
- When a controller render emits synchronous state updates, avoid attachment-local `$effect` loops that call controller mutators and immediately read the resulting Svelte state in the same reactive stack.
- The safe default for controller-backed wrappers is:
  - create the controller once
  - pre-sync required model state before the first mount render when needed
  - mount once from `onMount` or an equally stable lifecycle boundary
  - use a top-level wrapper sync path for later prop changes
- For controlled props such as `interactions`, do not bounce wrapper callbacks straight back into the same prop unless the value actually changed. No-op callback echo can create unnecessary update depth and render churn.

## Axis Editing Boundary

- Axis editing UI is app-owned.
- Charts should expose typed hooks and axis override plumbing, not ship product dialogs.
- Reuse:
  - [packages/data-models/src/cartesian-axis.ts](../../../packages/data-models/src/cartesian-axis.ts)
  - [packages/chart-core/src/cartesian-axis.ts](../../../packages/chart-core/src/cartesian-axis.ts)
  - wrapper props and callbacks in [packages/svelte/src/types.ts](../../../packages/svelte/src/types.ts)

## Workflow

1. Inspect the nearest existing chart in the same family.
2. Decide whether the concern belongs in `data-models`, `chart-core`, controller/domain, renderer, or wrapper.
3. If the rule is reusable, add it to shared presentation/geometry helpers first.
4. If the problem is sparse text/chrome on a raster chart, default to wrapper-owned DOM/SVG, not more canvas text.
5. If the problem is raster sharpness, fix the backing-store path before redesigning the chart.
6. If a wrapper is lagging or hitting Svelte update-depth errors, audit controller mount/sync order and callback echo before adding more memoization.
7. Keep internal renderer toggles internal unless there is a real embedder need.

## Validation

- Run `npx @sveltejs/mcp svelte-autofixer` on every touched `.svelte` file.
- Prefer package-scoped checks for the affected surfaces:
  - `bun --filter @ophiolite/charts-core typecheck`
  - `bun --filter @ophiolite/charts-renderer typecheck`
  - `bun --filter @ophiolite/charts typecheck`
  - `bun --filter @ophiolite/charts-playground typecheck`
  - `bun --filter @ophiolite/charts-playground build`
- Run `bun run typecheck` only when repo-wide contract generation and unrelated packages are in a healthy state. If it fails because of unrelated repo issues, say so clearly and report the targeted checks that passed.
- For visual changes, validate both a narrow host and a wide host so stage sizing, anchors, overlays, and scroll behavior remain stable.
- For well-panel changes, explicitly check:
  - first paint after mount
  - horizontal overflow and scroll
  - SVG chrome presence
  - clear/reload flows
  - absence of `effect_update_depth_exceeded`

## Pointers

- Shared chart registry: [packages/data-models/src/chart-registry.ts](../../../packages/data-models/src/chart-registry.ts)
- Shared interactions: [packages/data-models/src/interactions.ts](../../../packages/data-models/src/interactions.ts)
- Shared cartesian presentation: [packages/chart-core/src/cartesian-presentation.ts](../../../packages/chart-core/src/cartesian-presentation.ts)
- Shared seismic presentation: [packages/chart-core/src/seismic-presentation.ts](../../../packages/chart-core/src/seismic-presentation.ts)
- Shared seismic axis logic: [packages/chart-core/src/seismic-axis.ts](../../../packages/chart-core/src/seismic-axis.ts)
- Shared well-correlation chrome model: [packages/chart-core/src/well-correlation-chrome.ts](../../../packages/chart-core/src/well-correlation-chrome.ts)
- Shared probe panel presentation: [packages/chart-core/src/probe-panel-presentation.ts](../../../packages/chart-core/src/probe-panel-presentation.ts)
- Shared raster surface helper: [packages/renderer/src/internal/rasterSurface.ts](../../../packages/renderer/src/internal/rasterSurface.ts)
- Representative wrappers:
  - [packages/svelte/src/SeismicSectionChart.svelte](../../../packages/svelte/src/SeismicSectionChart.svelte)
  - [packages/svelte/src/SeismicGatherChart.svelte](../../../packages/svelte/src/SeismicGatherChart.svelte)
  - [packages/svelte/src/SeismicAxisOverlay.svelte](../../../packages/svelte/src/SeismicAxisOverlay.svelte)
  - [packages/svelte/src/WellCorrelationPanelChart.svelte](../../../packages/svelte/src/WellCorrelationPanelChart.svelte)
  - [packages/svelte/src/WellCorrelationAxisOverlay.svelte](../../../packages/svelte/src/WellCorrelationAxisOverlay.svelte)
  - [packages/svelte/src/RockPhysicsCrossplotChart.svelte](../../../packages/svelte/src/RockPhysicsCrossplotChart.svelte)
  - [packages/svelte/src/AvoResponseChart.svelte](../../../packages/svelte/src/AvoResponseChart.svelte)
