import type {
  RockPhysicsCategoricalSemantic,
  RockPhysicsCurveSemantic,
  RockPhysicsTemplateId,
  RockPhysicsTemplateLine,
  RockPhysicsTemplateOverlay
} from "./rock-physics-crossplot";

export interface RockPhysicsTemplateSpec {
  templateId: RockPhysicsTemplateId;
  title: string;
  xSemantics: readonly RockPhysicsCurveSemantic[];
  ySemantics: readonly RockPhysicsCurveSemantic[];
  xLabel: string;
  yLabel: string;
  xUnit?: string;
  yUnit?: string;
  allowedContinuousColorSemantics: readonly RockPhysicsCurveSemantic[];
  allowedCategoricalColorSemantics: readonly RockPhysicsCategoricalSemantic[];
  templateLines?: readonly RockPhysicsTemplateLine[];
  templateOverlays?: readonly RockPhysicsTemplateOverlay[];
  recommendedColorSemantic: RockPhysicsCurveSemantic | RockPhysicsCategoricalSemantic;
}

export const ROCK_PHYSICS_TEMPLATE_SPECS: Record<RockPhysicsTemplateId, RockPhysicsTemplateSpec> = {
  "vp-vs-vs-ai": {
    templateId: "vp-vs-vs-ai",
    title: "Vp/Vs vs AI",
    xSemantics: ["acoustic-impedance"],
    ySemantics: ["vp-vs-ratio"],
    xLabel: "AI",
    yLabel: "Vp/Vs",
    xUnit: "(m/s)*(g/cc)",
    yUnit: "ratio",
    allowedContinuousColorSemantics: [
      "water-saturation",
      "v-shale",
      "bulk-density",
      "neutron-porosity",
      "poissons-ratio",
      "gamma-ray"
    ],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    templateLines: [
      {
        id: "wet-sand-trend",
        label: "Wet Sand",
        color: "#ff7b72",
        points: [
          { x: 5_950, y: 1.95 },
          { x: 7_350, y: 1.78 },
          { x: 9_300, y: 1.58 }
        ]
      },
      {
        id: "gas-sand-trend",
        label: "Gas Sand",
        color: "#ffd166",
        points: [
          { x: 6_100, y: 2.12 },
          { x: 7_800, y: 1.92 },
          { x: 9_650, y: 1.74 }
        ]
      }
    ],
    recommendedColorSemantic: "water-saturation"
  },
  "ai-vs-si": {
    templateId: "ai-vs-si",
    title: "AI vs SI",
    xSemantics: ["acoustic-impedance"],
    ySemantics: ["shear-impedance"],
    xLabel: "AI",
    yLabel: "SI",
    xUnit: "(m/s)*(g/cc)",
    yUnit: "(m/s)*(g/cc)",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "bulk-density", "gamma-ray", "neutron-porosity"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    templateLines: [
      {
        id: "stiff-sand-trend",
        label: "Stiff Sand",
        color: "#6ee7b7",
        points: [
          { x: 5_800, y: 2_650 },
          { x: 7_500, y: 3_850 },
          { x: 9_000, y: 4_950 }
        ]
      },
      {
        id: "shale-trend",
        label: "Shale",
        color: "#f59e0b",
        points: [
          { x: 5_400, y: 2_050 },
          { x: 7_000, y: 3_050 },
          { x: 8_500, y: 3_850 }
        ]
      }
    ],
    recommendedColorSemantic: "facies"
  },
  "vp-vs-vs": {
    templateId: "vp-vs-vs",
    title: "Vp vs Vs",
    xSemantics: ["s-velocity"],
    ySemantics: ["p-velocity"],
    xLabel: "Vs",
    yLabel: "Vp",
    xUnit: "m/s",
    yUnit: "m/s",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "gamma-ray", "bulk-density", "neutron-porosity"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    templateLines: [
      {
        id: "brine-line",
        label: "Brine Trend",
        color: "#60a5fa",
        points: [
          { x: 1_450, y: 2_700 },
          { x: 2_150, y: 3_700 },
          { x: 2_950, y: 4_950 }
        ]
      },
      {
        id: "gas-line",
        label: "Gas Trend",
        color: "#f87171",
        points: [
          { x: 1_150, y: 2_250 },
          { x: 1_950, y: 3_150 },
          { x: 2_650, y: 4_100 }
        ]
      }
    ],
    recommendedColorSemantic: "v-shale"
  },
  "porosity-vs-vp": {
    templateId: "porosity-vs-vp",
    title: "Porosity vs Vp",
    xSemantics: ["neutron-porosity", "effective-porosity"],
    ySemantics: ["p-velocity"],
    xLabel: "Porosity",
    yLabel: "Vp",
    xUnit: "%",
    yUnit: "m/s",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "gamma-ray"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    templateOverlays: [
      {
        kind: "polygon",
        id: "compaction-corridor",
        label: "Compaction Corridor",
        fillColor: "rgba(49, 208, 170, 0.10)",
        strokeColor: "#31d0aa",
        points: [
          { x: 6, y: 5_800 },
          { x: 18, y: 4_700 },
          { x: 30, y: 3_450 },
          { x: 38, y: 2_200 },
          { x: 28, y: 3_450 },
          { x: 14, y: 4_550 }
        ],
        labelPosition: { x: 24, y: 3_850 }
      },
      {
        kind: "polyline",
        id: "voigt-bound",
        label: "Voigt Avg.",
        color: "#f59e0b",
        dashed: true,
        points: [
          { x: 0, y: 6_000 },
          { x: 18, y: 5_760 },
          { x: 42, y: 5_250 },
          { x: 70, y: 4_100 },
          { x: 100, y: 1_600 }
        ]
      },
      {
        kind: "polyline",
        id: "reuss-bound",
        label: "Reuss Avg.",
        color: "#60a5fa",
        dashed: true,
        points: [
          { x: 0, y: 5_800 },
          { x: 6, y: 3_700 },
          { x: 18, y: 2_350 },
          { x: 38, y: 1_820 },
          { x: 100, y: 1_520 }
        ]
      },
      {
        kind: "text",
        id: "critical-porosity",
        text: "Critical Porosity",
        color: "#f8fafc",
        x: 39,
        y: 2_050
      }
    ],
    recommendedColorSemantic: "facies"
  },
  "lambda-rho-vs-mu-rho": {
    templateId: "lambda-rho-vs-mu-rho",
    title: "Lambda-Rho vs Mu-Rho",
    xSemantics: ["lambda-rho"],
    ySemantics: ["mu-rho"],
    xLabel: "Lambda-Rho",
    yLabel: "Mu-Rho",
    xUnit: "GPa",
    yUnit: "GPa",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "gamma-ray", "bulk-density"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    templateLines: [
      {
        id: "fluid-trend",
        label: "Fluid Trend",
        color: "#f43f5e",
        points: [
          { x: 5, y: 78 },
          { x: 28, y: 72 },
          { x: 55, y: 65 }
        ]
      },
      {
        id: "rigidity-trend",
        label: "Rigidity Trend",
        color: "#22d3ee",
        points: [
          { x: 20, y: 38 },
          { x: 28, y: 62 },
          { x: 32, y: 108 }
        ]
      }
    ],
    recommendedColorSemantic: "well"
  },
  "neutron-porosity-vs-bulk-density": {
    templateId: "neutron-porosity-vs-bulk-density",
    title: "Neutron Porosity vs Bulk Density",
    xSemantics: ["neutron-porosity"],
    ySemantics: ["bulk-density"],
    xLabel: "Neutron Porosity",
    yLabel: "Bulk Density",
    xUnit: "%",
    yUnit: "g/cc",
    allowedContinuousColorSemantics: ["gamma-ray", "water-saturation", "v-shale"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    templateOverlays: [
      {
        kind: "polyline",
        id: "sandstone-matrix",
        label: "Sandstone",
        color: "#d97706",
        points: [
          { x: -2, y: 2.66 },
          { x: 6, y: 2.48 },
          { x: 16, y: 2.34 },
          { x: 28, y: 2.18 }
        ]
      },
      {
        kind: "polyline",
        id: "limestone-matrix",
        label: "Limestone",
        color: "#60a5fa",
        points: [
          { x: 0, y: 2.72 },
          { x: 10, y: 2.55 },
          { x: 22, y: 2.35 },
          { x: 34, y: 2.15 }
        ]
      },
      {
        kind: "polyline",
        id: "dolomite-matrix",
        label: "Dolomite",
        color: "#c084fc",
        points: [
          { x: 4, y: 2.84 },
          { x: 14, y: 2.62 },
          { x: 28, y: 2.38 },
          { x: 42, y: 2.12 }
        ]
      },
      {
        kind: "polygon",
        id: "shale-window",
        label: "Shale Window",
        fillColor: "rgba(250, 204, 21, 0.10)",
        strokeColor: "#facc15",
        points: [
          { x: 26, y: 2.44 },
          { x: 34, y: 2.56 },
          { x: 46, y: 2.48 },
          { x: 48, y: 2.16 },
          { x: 32, y: 2.10 }
        ],
        labelPosition: { x: 39, y: 2.28 }
      },
      {
        kind: "text",
        id: "gas-correction",
        text: "Approximate Gas Correction",
        color: "#e2e8f0",
        x: 9,
        y: 2.18,
        rotationDeg: -58
      }
    ],
    recommendedColorSemantic: "gamma-ray"
  },
  "phi-vs-ai": {
    templateId: "phi-vs-ai",
    title: "Phi vs AI",
    xSemantics: ["acoustic-impedance"],
    ySemantics: ["neutron-porosity", "effective-porosity"],
    xLabel: "AI",
    yLabel: "Phi",
    xUnit: "(m/s)*(g/cc)",
    yUnit: "%",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "bulk-density", "gamma-ray"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    recommendedColorSemantic: "water-saturation"
  },
  "pr-vs-ai": {
    templateId: "pr-vs-ai",
    title: "PR vs AI",
    xSemantics: ["acoustic-impedance"],
    ySemantics: ["poissons-ratio"],
    xLabel: "AI",
    yLabel: "PR",
    xUnit: "(m/s)*(g/cc)",
    yUnit: "ratio",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "bulk-density", "neutron-porosity"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    recommendedColorSemantic: "water-saturation"
  },
  "vp-vs-density": {
    templateId: "vp-vs-density",
    title: "Vp vs Density",
    xSemantics: ["bulk-density"],
    ySemantics: ["p-velocity"],
    xLabel: "Density",
    yLabel: "Vp",
    xUnit: "g/cc",
    yUnit: "m/s",
    allowedContinuousColorSemantics: ["water-saturation", "v-shale", "neutron-porosity", "gamma-ray"],
    allowedCategoricalColorSemantics: ["well", "wellbore", "facies"],
    recommendedColorSemantic: "water-saturation"
  }
};

export const STANDARD_ROCK_PHYSICS_TEMPLATE_IDS = [
  "vp-vs-vs-ai",
  "ai-vs-si",
  "vp-vs-vs",
  "porosity-vs-vp",
  "lambda-rho-vs-mu-rho",
  "neutron-porosity-vs-bulk-density"
] as const satisfies readonly RockPhysicsTemplateId[];

export function getRockPhysicsTemplateSpec(templateId: RockPhysicsTemplateId): RockPhysicsTemplateSpec {
  return ROCK_PHYSICS_TEMPLATE_SPECS[templateId];
}
