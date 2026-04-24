# Start Here

This is the default entry point for the public `Ophiolite Charts` story.

## The intended path

1. Start with `@ophiolite/charts`.
2. Use neutral chart-family models such as `SeismicSectionData`, `SeismicGatherData`, `SurveyMapData`, `RockPhysicsCrossplotData`, and `WellCorrelationPanelData`.
3. Add `@ophiolite/charts/adapters/ophiolite` only if your source data begins as Ophiolite contract DTOs.
4. Treat `traceboost-demo` as a reference consumer, not as the API boundary.

## Support tiers in one pass

- `public-launch`
  The default documented path for embedders.
- `public-adapter`
  Explicit Ophiolite integration helpers.
- `preview`
  Opt-in surfaces that stay outside the default product promise.
- `internal`
  Lower-level packages that back the SDK but are not the first public story.

Read more in [support-tiers.md](./support-tiers.md).

The manifest-backed package catalog lives at [../manifests/generated/module-catalog.json](../manifests/generated/module-catalog.json).

## Default package

Use `@ophiolite/charts` first.

That package owns the launch wrapper story:

- `SeismicSectionChart`
- `SeismicGatherChart`
- `SurveyMapChart`
- `RockPhysicsCrossplotChart`
- `WellCorrelationPanelChart`

It also re-exports the public model types that match those chart families.

## Adapter boundary

If your source data is already in Ophiolite contract DTO form, use the explicit adapter path:

```ts
import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";
```

The goal is to keep Ophiolite-specific decoding visible at the boundary instead of leaking transport-shaped DTO concerns into the default wrapper API.

See [recipes/ophiolite-adapters.md](./recipes/ophiolite-adapters.md).

## Preview boundary

Preview families and adjacent widgets stay off the root path:

- `@ophiolite/charts/preview`
- `@ophiolite/charts/extras`

That is intentional. The default launch story should remain focused and easier to consume.

## TraceBoost as reference consumer

`traceboost-demo` should consume the chart SDK through the same public package boundaries that an external customer would use.

That means:

- charts owns rendering, viewport behavior, and wrapper APIs
- TraceBoost owns app workflow, transport, session state, and product-specific orchestration

See [recipes/traceboost-reference-consumer.md](./recipes/traceboost-reference-consumer.md).

## What not to lead with

Do not start public examples with:

- `@ophiolite/charts-data-models`
- `@ophiolite/charts-core`
- `@ophiolite/charts-renderer`
- `@ophiolite/charts-domain`

Those packages matter, but they are implementation layers, not the first embedder story.

## Next reads

- [support-tiers.md](./support-tiers.md)
- [../manifests/generated/module-catalog.json](../manifests/generated/module-catalog.json)
- [recipes/README.md](./recipes/README.md)
- [examples/seismic-section-simple.md](./examples/seismic-section-simple.md)
- [examples/seismic-section-production.md](./examples/seismic-section-production.md)
