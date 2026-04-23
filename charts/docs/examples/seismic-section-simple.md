# Seismic Section: Simple

Use the neutral public model from `@ophiolite/charts`.

```ts
import { SeismicSectionChart, type SeismicSectionData } from "@ophiolite/charts";

const section: SeismicSectionData = {
  axis: "inline",
  coordinate: { index: 111, value: 111 },
  horizontalAxis: Float64Array.from([875, 876, 877, 878]),
  sampleAxis: Float32Array.from([0, 4, 8, 12]),
  amplitudes: Float32Array.from([
    0.2, -0.1, 0.4, 0.3,
    0.1, 0.0, -0.2, 0.5,
    -0.3, 0.2, 0.1, -0.1,
    0.0, 0.4, 0.2, -0.2
  ]),
  dimensions: { traces: 4, samples: 4 }
};
```

Use this for the first embed when the data is already materialized in memory.
