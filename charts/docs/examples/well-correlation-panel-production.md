# Well Correlation Panel: Production

```ts
import { WellCorrelationPanelChart, type WellCorrelationPanelData } from "@ophiolite/charts";
import { adaptOphioliteWellPanelToChart } from "@ophiolite/charts/adapters/ophiolite";

const panel: WellCorrelationPanelData = adaptOphioliteWellPanelToChart(
  resolvedWellPanelSource,
  resolvedWellPanelLayout
);
```

Production guidance:

- keep panel layout adaptation explicit at the adapter boundary
- use typed arrays for dense curve and seismic-track data
- manage viewport, probe, and chart handles from the embedding app
