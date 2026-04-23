# Seismic Section Simple Example

This example shows the target public embedding style for the seismic section wrapper.

It uses:

- `SeismicSectionChart` from `@ophiolite/charts`
- a neutral `SeismicSectionData` object
- no `@ophiolite/contracts`
- no transport-shaped fields such as `horizontal_axis_f64le` or `amplitudes_f32le`

```svelte
<script lang="ts">
  import { SeismicSectionChart, type SeismicSectionData } from "@ophiolite/charts";

  const section: SeismicSectionData = {
    axis: "inline",
    coordinate: {
      index: 111,
      value: 111
    },
    horizontalAxis: Float64Array.from([875, 876, 877, 878]),
    sampleAxis: Float32Array.from([0, 4, 8, 12]),
    amplitudes: Float32Array.from([
      0.2, -0.1, 0.4, 0.3,
      0.1, 0.0, -0.2, 0.5,
      -0.3, 0.2, 0.1, -0.1,
      0.0, 0.4, 0.2, -0.2
    ]),
    dimensions: {
      traces: 4,
      samples: 4
    },
    units: {
      horizontal: "xline",
      sample: "ms",
      amplitude: "arb"
    },
    presentation: {
      title: "Inline 111",
      sampleAxisLabel: "Time"
    }
  };
</script>

<SeismicSectionChart
  chartId="example-section"
  viewId="inline:111"
  {section}
/>
```

## Ophiolite Integration

If the source data starts as an Ophiolite `SectionView`, keep the transport mapping explicit:

```ts
import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";

const section = adaptOphioliteSectionViewToSeismicSectionData(sectionView);
```

That adapter should produce the same neutral public model that a non-Ophiolite consumer could construct directly.
