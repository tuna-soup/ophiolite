# Seismic Section: Production

Use the same public model, but add controlled viewport and callback handling.

```ts
import {
  SeismicSectionChart,
  type SeismicSectionData,
  type SeismicSectionProbeChangePayload,
  type SeismicSectionViewportChangePayload
} from "@ophiolite/charts";
import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";

const section: SeismicSectionData = adaptOphioliteSectionViewToSeismicSectionData(sectionView);

function handleViewportChange(event: SeismicSectionViewportChangePayload) {
  console.log(event.viewport);
}

function handleProbeChange(event: SeismicSectionProbeChangePayload) {
  console.log(event.probe);
}
```

Production guidance:

- keep DTO adaptation outside the component props
- use typed arrays for trace/sample payloads
- treat viewport/probe payloads as the stable public callback shape
