# Rock Physics Crossplot: Simple

```ts
import { RockPhysicsCrossplotChart, type RockPhysicsCrossplotData } from "@ophiolite/charts";

const model: RockPhysicsCrossplotData = {
  templateId: "vp-vs-vs-ai",
  title: "Vp/Vs vs AI",
  groups: [
    { name: "Well A", color: "#0f766e" },
    { name: "Well B", color: "#b45309", symbol: "diamond" }
  ],
  points: [
    { x: 5850, y: 1.62, group: "Well A", depthM: 2410 },
    { x: 6120, y: 1.68, group: "Well A", depthM: 2422 },
    { x: 7180, y: 1.89, group: "Well B", depthM: 2462 },
    { x: 7560, y: 1.96, group: "Well B", depthM: 2474 }
  ]
};
```

This keeps the public model template-scoped and domain-specific without becoming a generic scatter config object.
