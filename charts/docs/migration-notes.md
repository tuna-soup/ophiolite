# Migration Notes

These notes track the current public-SDK cleanup.

## Root Export Narrowing

`@ophiolite/charts` now focuses on the launch chart families:

- `SeismicSectionChart`
- `SeismicGatherChart`
- `SurveyMapChart`
- `RockPhysicsCrossplotChart`
- `WellCorrelationPanelChart`

Preview and adjacent surfaces moved to explicit subpaths:

- `@ophiolite/charts/preview`
- `@ophiolite/charts/extras`

## Ophiolite Integration

If your app previously relied on Ophiolite contract-shaped payloads directly in the wrapper props, prefer the explicit adapter path:

```ts
import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";
```

The adapter output is intended to match the neutral public model that a non-Ophiolite consumer could author directly.

## Seismic Callback Payloads

Seismic viewport and probe callbacks now use neutral public payloads rather than contract-shaped DTO callback types.

Use:

- `SeismicSectionViewportChangePayload`
- `SeismicSectionProbeChangePayload`
- `SeismicGatherViewportChangePayload`
- `SeismicGatherProbeChangePayload`

## Remaining Work

The current migration is still early-access quality:

- lower-level packages are not part of the default public promise
- preview families are intentionally segregated
- non-seismic launch families still need continued public-model cleanup over time
