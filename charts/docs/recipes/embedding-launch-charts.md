# Embedding Launch Charts

This recipe is the default public embedding path.

## Use the root package first

Start with:

```ts
import {
  SeismicSectionChart,
  SeismicGatherChart,
  SurveyMapChart,
  RockPhysicsCrossplotChart,
  WellCorrelationPanelChart
} from "@ophiolite/charts";
```

That keeps the first learning path aligned with the public SDK boundary.

## Prefer neutral chart-family models

The launch wrappers are intended to receive chart-native public models, not Ophiolite transport DTOs.

Examples:

- `SeismicSectionData`
- `SeismicGatherData`
- `SurveyMapData`
- `RockPhysicsCrossplotData`
- `WellCorrelationPanelData`

That is deliberate. The public API should read like a subsurface chart SDK rather than a generic chart config DSL or a contract decoder.

## Keep the host responsible for app workflow

The embedding host should own:

- which dataset is active
- workflow-level state and selection
- transport and backend calls
- app-specific overlays and diagnostics wording

The chart SDK should own:

- chart rendering
- viewport behavior
- chart-native callbacks and handles
- wrapper-level chart chrome

## Use examples in pairs

Each launch family should be learned through two docs:

- `simple`
- `production`

Current example files live in `docs/examples/`.

## When to leave the default path

Only leave the default path when you need:

- Ophiolite DTO adaptation
- preview families
- companion toolbar surfaces

Those concerns are real, but they should not define the default first-run story.
