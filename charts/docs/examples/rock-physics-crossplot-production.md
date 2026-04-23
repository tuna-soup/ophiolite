# Rock Physics Crossplot: Production

```ts
import { RockPhysicsCrossplotChart, type RockPhysicsCrossplotData } from "@ophiolite/charts";
import { adaptOphioliteRockPhysicsCrossplotToChart } from "@ophiolite/charts/adapters/ophiolite";

const model: RockPhysicsCrossplotData = adaptOphioliteRockPhysicsCrossplotToChart(resolvedCrossplotSource);
```

Production guidance:

- use typed columns for large point sets
- keep template semantics explicit through `templateId`, `xAxis.semantic`, and `yAxis.semantic`
- drive probe and viewport state from the host app rather than chart-internal workflow state
