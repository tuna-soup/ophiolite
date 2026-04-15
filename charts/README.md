# Ophiolite Charts

`Ophiolite Charts` is the embeddable chart SDK inside the Ophiolite platform.

It is designed for desktop-class interpretation workflows while remaining frontend-native TypeScript that can power demos, applications, and future browser deployments.

## Scope

Current chart families:

- seismic sections
- prestack gathers
- survey maps
- well correlation panels
- rock physics crossplots

## Package Layout

### Packages

- `@ophiolite/charts-data-models`
- `@ophiolite/charts-core`
- `@ophiolite/charts-renderer`
- `@ophiolite/charts-domain`
- `@ophiolite/charts`
- `@ophiolite/charts-toolbar`

### Apps

- `@ophiolite/charts-demo`
- `@ophiolite/charts-playground`
- `@ophiolite/charts-benchmark`

### First-Party Consumer

The flagship first-party workflow demo lives outside the chart workspace at `../apps/traceboost-demo`.

That app must consume the public chart packages the same way an external customer would.

## Boundary Rule

Ophiolite Charts owns chart-native rendering, viewport behavior, anchors, and embedder-facing wrapper APIs.

It does not own canonical subsurface meaning, backend transport details, or product workflow state.

Canonical contract DTOs should be adapted into chart payloads at the wrapper boundary rather than leaking app-specific transport concerns into renderer or controller internals.

## Development

From `charts/`:

```bash
bun install
bun run dev
```

Useful commands:

```bash
bun run dev:svelte
bun run typecheck
bun run build
bun run dev:benchmark
```

## Embedding

`@ophiolite/charts` exports the main Svelte wrapper surface, including `SeismicSectionChart`, `SeismicGatherChart`, `SurveyMapChart`, `WellCorrelationPanelChart`, and `RockPhysicsCrossplotChart`.

Use `@ophiolite/charts/contracts` when you want the wrapper-layer adapters that translate canonical Ophiolite DTOs into chart payloads without pulling app-specific transport code into the SDK.

The public surface also re-exports `CHART_FAMILIES`, `CHART_DEFINITIONS`, and lookup helpers from `@ophiolite/charts-data-models`. That registry records the canonical source boundary, renderer kernel, allowed asset families, and validation entry points for each chart family so embedders can keep the same constraints the SDK enforces internally.
