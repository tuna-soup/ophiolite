---
name: geoviz
description: Use when working on geoviz charts, Svelte wrappers, stage sizing, overlay anchors, or embedder-facing APIs. Covers the SDK boundary between geoviz and apps like TraceBoost, intrinsic chart sizing, top-left stage anchoring, and validation expectations.
---

# Geoviz

Build `geoviz` as an embedder-friendly chart SDK. `TraceBoost` is the reference app, not the product boundary.

## Data Boundary

- Keep four layers distinct:
  - Ophiolite semantic contracts and canonical asset meaning
  - TraceBoost app and transport boundaries
  - geoviz chart payloads in `packages/data-models`
  - renderer/controller internals
- For seismic charts, `packages/svelte/src/contracts.ts` is the adapter boundary from contract-shaped payloads into `geoviz` models.
- Do not push `tbvol`, request-response DTOs, or packed transport headers directly into renderer/controller types.
- If an embedder needs binary-packing efficiency, keep that concern in the app transport layer. `geoviz` should consume normalized chart payloads or semantic view contracts, not wire-format details.

## Core Contract

- Keep chart-native features in `geoviz`: axes, probe/cursor, in-chart annotations, legends, scrollbars, split divider, optional built-in interaction toolbar.
- Keep product or workflow controls out of `geoviz`: axis selectors, index inputs, compare-survey cycling, app settings dialogs, app-specific status chips.
- If an app needs UI near the chart, expose chart-relative anchors or geometry from `geoviz`; do not hard-code that app UI into the SDK.

## Layout Rules

- Never stretch the rendered chart surface to fill available parent space. Fonts, traces, and interaction affordances should stay at fixed intrinsic size.
- The chart wrapper owns an intrinsic stage size. If the parent is smaller, overflow with scrollbars.
- If the parent is larger, anchor the stage to the top left. Extra space belongs on the right and bottom, not evenly around the chart.
- Keep sizing logic centralized. For seismic charts, prefer extending [packages/svelte/src/seismic-stage.ts](../../../packages/svelte/src/seismic-stage.ts) instead of scattering width or height heuristics.
- App-controlled enlargement is acceptable through explicit scaling such as `stageScale`; implicit stretch-to-fit is not.

## Overlay Rules

- Overlay content must be chart-relative, not parent-relative.
- For seismic Svelte wrappers, preserve the named anchor model in [packages/svelte/src/types.ts](../../../packages/svelte/src/types.ts):
  - `stageTopLeft`
  - `plotTopCenter`
  - `plotTopRight`
  - `plotBottomRight`
  - `plotBottomLeft`
- Use `stageTopLeft` for controls that belong in the title band or outer stage area.
- Use `plot*` anchors for items intentionally tied to the plot rectangle.
- Built-in chart readouts such as probe or amplitude hover values remain chart-owned and should not be displaced by app overlays.
- Do not position embedder UI with absolute offsets against the outer parent when an anchor API can express it.

## Wrapper Design

- Add SDK-facing behavior in the wrapper layer before pushing it deep into the renderer, unless the renderer truly owns the concern.
- Keep overlay and sizing APIs consistent across chart families when the concept applies. If you add a new chart type, match the existing anchor vocabulary unless there is a strong reason not to.
- Reuse existing public types instead of adding one-off app-specific props.

## Workflow

1. Inspect the existing wrapper and stage helper before adding new props or layout behavior.
2. Decide whether the feature is chart-native, adapter-boundary work, or embedder-specific. Put it on the correct side of the boundary.
3. If contract inputs changed, update the adapter in `packages/svelte/src/contracts.ts` before changing controller or renderer types.
4. If the feature affects layout, verify intrinsic sizing, scroll behavior, and top-left anchoring in both narrow and wide hosts.
5. If the feature affects overlays, verify anchors remain stable as the parent grows or shrinks.
6. Update the reference embedder only after the `geoviz` API is clear.

## Validation

- Run `npx @sveltejs/mcp svelte-autofixer` on every touched `.svelte` file.
- Run `bun run typecheck`.
- Build the Svelte playground with `bun --filter @geoviz/svelte-playground build`.
- If the change affects an embedder, validate that app too, but distinguish `geoviz` regressions from unrelated app breakage.
- When reporting results, call out whether failures are in `geoviz` itself or in external consumers.

## Pointers

- Svelte seismic wrapper contract: [packages/svelte/src/types.ts](../../../packages/svelte/src/types.ts)
- Seismic stage sizing helper: [packages/svelte/src/seismic-stage.ts](../../../packages/svelte/src/seismic-stage.ts)
- Current wrapper implementations:
  - [packages/svelte/src/SeismicSectionChart.svelte](../../../packages/svelte/src/SeismicSectionChart.svelte)
  - [packages/svelte/src/SeismicGatherChart.svelte](../../../packages/svelte/src/SeismicGatherChart.svelte)
