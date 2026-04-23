# Well Correlation Panel: Simple

```ts
import { WellCorrelationPanelChart, type WellCorrelationPanelData } from "@ophiolite/charts";

const panel: WellCorrelationPanelData = {
  name: "Well Correlation",
  depthDomain: {
    start: 1500,
    end: 1620,
    unit: "m",
    label: "MD"
  },
  wells: [
    {
      name: "Well A",
      depthDatum: "md",
      curves: [
        {
          name: "GR",
          values: Float32Array.from([72, 86]),
          depths: Float32Array.from([1500, 1520]),
          unit: "API"
        }
      ],
      tops: [
        { name: "Reservoir Top", depth: 1540 }
      ],
    }
  ]
};
```

Start with a small number of wells and tracks. Add richer track families later.
