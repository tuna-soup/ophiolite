# Prestack Gather: Production

```ts
import {
  SeismicGatherChart,
  type SeismicGatherData,
  type SeismicGatherProbeChangePayload,
  type SeismicGatherViewportChangePayload
} from "@ophiolite/charts";
import { adaptOphioliteGatherViewToSeismicGatherData } from "@ophiolite/charts/adapters/ophiolite";

const gather: SeismicGatherData = adaptOphioliteGatherViewToSeismicGatherData(gatherView);

function handleViewportChange(event: SeismicGatherViewportChangePayload) {
  console.log(event.viewport);
}

function handleProbeChange(event: SeismicGatherProbeChangePayload) {
  console.log(event.probe);
}
```

Production guidance:

- keep gather-axis semantics explicit with `gatherAxisKind`
- use typed arrays for the dense payload
- wire viewport/probe callbacks through your host app state
