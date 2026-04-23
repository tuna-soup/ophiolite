# Survey Map: Simple

```ts
import { SurveyMapChart, type SurveyMapData } from "@ophiolite/charts";

const map: SurveyMapData = {
  name: "North Survey",
  xLabel: "Easting",
  yLabel: "Northing",
  coordinateUnit: "m",
  areas: [
    {
      name: "North Survey",
      points: [
        { x: 120, y: 160 },
        { x: 2060, y: 180 },
        { x: 2120, y: 1540 },
        { x: 180, y: 1620 }
      ]
    }
  ],
  wells: [
    {
      name: "Well A",
      position: { x: 420, y: 480 },
      color: "#0e7490"
    }
  ]
};
```

This is the public path for survey outlines and simple well positions.
