# Survey Map: Production

```ts
import { SurveyMapChart, type SurveyMapData } from "@ophiolite/charts";
import { adaptOphioliteSurveyMapToChart } from "@ophiolite/charts/adapters/ophiolite";

const map: SurveyMapData = adaptOphioliteSurveyMapToChart(resolvedSurveyMapSource);
```

Production guidance:

- keep Ophiolite source adaptation outside the component boundary
- include scalar grids, trajectories, and controlled viewport callbacks when needed
- preserve clear `surface` and `trajectory` naming instead of DTO field names
