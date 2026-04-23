# Prestack Gather: Simple

```ts
import { SeismicGatherChart, type SeismicGatherData } from "@ophiolite/charts";

const gather: SeismicGatherData = {
  label: "Gather 042",
  gatherAxisKind: "offset",
  sampleDomain: "time",
  horizontalAxis: Float64Array.from([250, 500, 750, 1000]),
  sampleAxis: Float32Array.from([0, 4, 8, 12]),
  amplitudes: Float32Array.from([
    0.08, 0.16, 0.24, 0.18,
    -0.04, 0.12, 0.3, 0.44,
    -0.14, 0.05, 0.22, 0.38,
    -0.18, -0.04, 0.16, 0.31
  ]),
  dimensions: { traces: 4, samples: 4 }
};
```

Use this path when the gather is already decoded into offsets, samples, and amplitudes.
