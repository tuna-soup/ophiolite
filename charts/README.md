# Ophiolite Charts

`Ophiolite Charts` is the embeddable chart SDK inside the Ophiolite platform.

It is designed for desktop-class interpretation workflows while remaining frontend-native TypeScript that can power demos, applications, and future browser deployments.

## Public Story

Teach the SDK in this order:

1. `@ophiolite/charts`
2. `@ophiolite/charts/adapters/ophiolite` when the input begins as Ophiolite DTOs
3. `@ophiolite/charts/preview` and `@ophiolite/charts/extras` only by explicit opt-in
4. `traceboost-demo` as a reference consumer of those public package boundaries

The public docs app is `apps/public-docs`. The Svelte playground remains an internal lab and integration surface rather than the default public learning path.

## Docs Map

- [docs/start-here.md](./docs/start-here.md)
- [docs/support-tiers.md](./docs/support-tiers.md)
- [docs/recipes/README.md](./docs/recipes/README.md)
- [docs/examples/seismic-section-simple.md](./docs/examples/seismic-section-simple.md)
- [docs/examples/seismic-section-production.md](./docs/examples/seismic-section-production.md)
- [docs/public-sdk-roadmap.md](./docs/public-sdk-roadmap.md)
- [docs/public-sdk-audit.md](./docs/public-sdk-audit.md)

## Support Tiers

The package-manifest vocabulary is intentionally small:

- `public-launch`
  Default documented surface for embedders.
- `public-adapter`
  Explicit Ophiolite integration helpers.
- `preview`
  Opt-in surfaces with narrower guarantees.
- `internal`
  Implementation layers behind the public SDK boundary.

Package manifests live in `packages/*/ophiolite.module.json`.

Validate them with:

```bash
bun run validate:manifests
```

That command also emits a normalized catalog at `manifests/generated/module-catalog.json` and a docs-facing mirror at `apps/public-docs/src/lib/generated/manifest-catalog.ts`.

See [manifests/README.md](./manifests/README.md) and [docs/support-tiers.md](./docs/support-tiers.md).

## Scope

Current launch chart families:

- seismic sections
- prestack gathers
- survey maps
- well correlation panels
- rock physics crossplots

Current preview and adjacent surfaces:

- volume interpretation
- AVO response, intercept-gradient, and chi-projection charts
- adjacent inspectors and analysis widgets

## Package Layout

### Public and companion surfaces

- `@ophiolite/charts`
- `@ophiolite/charts-toolbar`

### Internal implementation packages

- `@ophiolite/charts-data-models`
- `@ophiolite/charts-core`
- `@ophiolite/charts-renderer`
- `@ophiolite/charts-domain`

### Apps

- `@ophiolite/charts-docs`
- internal playground and benchmark apps

### First-party consumer

The flagship first-party workflow demo lives outside the chart workspace at `../apps/traceboost-demo`.

That app must consume the public chart packages the same way an external customer would.

## Boundary Rule

Ophiolite Charts owns chart-native rendering, viewport behavior, anchors, and embedder-facing wrapper APIs.

It does not own canonical subsurface meaning, backend transport details, or product workflow state.

Canonical contract DTOs should be adapted into chart payloads at the wrapper boundary rather than leaking app-specific transport concerns into renderer or controller internals.

## Embedding

`@ophiolite/charts` exports the launch chart surface:

- `SeismicSectionChart`
- `SeismicGatherChart`
- `SurveyMapChart`
- `WellCorrelationPanelChart`
- `RockPhysicsCrossplotChart`

The public direction is:

- use neutral chart-family data models from `@ophiolite/charts`
- use `@ophiolite/charts/adapters/ophiolite` only when adapting Ophiolite DTOs into those neutral models

Preview-only families stay off the root path:

- `@ophiolite/charts/preview`
- `@ophiolite/charts/extras`

For the seismic launch surfaces, the public wrapper props accept neutral public models such as `SeismicSectionData` and `SeismicGatherData`, while `@ophiolite/charts/adapters/ophiolite` exposes adapter functions such as:

- `adaptOphioliteSectionViewToSeismicSectionData`
- `adaptOphioliteGatherViewToSeismicGatherData`
- `adaptOphioliteSurveyMapToChart`
- `adaptOphioliteRockPhysicsCrossplotToChart`
- `adaptOphioliteWellPanelToChart`

See [docs/seismic-section-simple-example.md](./docs/seismic-section-simple-example.md) for a minimal public-style seismic section example.

The public surface also re-exports `CHART_FAMILIES`, `CHART_DEFINITIONS`, and lookup helpers from `@ophiolite/charts-data-models`. That registry records the canonical source boundary, renderer kernel, allowed asset families, and validation entry points for each chart family so embedders can keep the same constraints the SDK enforces internally.

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
