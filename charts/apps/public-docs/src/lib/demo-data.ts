import type {
  RockPhysicsCrossplotData,
  SeismicGatherData,
  SeismicSectionData,
  SurveyMapData,
  WellCorrelationPanelData
} from "@ophiolite/charts";

export const section: SeismicSectionData = {
  axis: "inline",
  coordinate: {
    index: 111,
    value: 111
  },
  horizontalAxis: Float64Array.from([875, 876, 877, 878, 879, 880]),
  sampleAxis: Float32Array.from([0, 4, 8, 12, 16, 20]),
  amplitudes: Float32Array.from([
    0.18, -0.05, 0.34, 0.26, -0.16, 0.09,
    0.1, 0.02, -0.14, 0.42, 0.16, -0.22,
    -0.22, 0.18, 0.12, -0.08, 0.28, 0.05,
    0.03, 0.32, 0.2, -0.14, 0.08, -0.18,
    -0.15, 0.04, 0.24, 0.31, -0.1, 0.14,
    0.06, -0.11, 0.19, 0.27, 0.12, -0.06
  ]),
  dimensions: {
    traces: 6,
    samples: 6
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

export const gather: SeismicGatherData = {
  label: "Gather 042",
  gatherAxisKind: "offset",
  sampleDomain: "time",
  horizontalAxis: Float64Array.from([250, 500, 750, 1000, 1250, 1500]),
  sampleAxis: Float32Array.from([0, 4, 8, 12, 16, 20]),
  amplitudes: Float32Array.from([
    0.08, 0.16, 0.24, 0.18, 0.1, 0.02,
    -0.04, 0.12, 0.3, 0.44, 0.36, 0.18,
    -0.14, 0.05, 0.22, 0.38, 0.28, 0.12,
    -0.18, -0.04, 0.16, 0.31, 0.22, 0.08,
    -0.22, -0.08, 0.1, 0.24, 0.2, 0.06,
    -0.16, -0.02, 0.08, 0.17, 0.12, 0.02
  ]),
  dimensions: {
    traces: 6,
    samples: 6
  },
  units: {
    horizontal: "m",
    sample: "ms",
    amplitude: "arb"
  },
  displayDefaults: {
    gain: 1.4
  }
};

export const surveyMap: SurveyMapData = {
  name: "North Survey",
  xLabel: "Easting",
  yLabel: "Northing",
  coordinateUnit: "m",
  background: "#f4f2ee",
  areas: [
    {
      name: "North Survey",
      points: [
        { x: 120, y: 160 },
        { x: 2060, y: 180 },
        { x: 2120, y: 1540 },
        { x: 180, y: 1620 }
      ],
      stroke: "rgba(39, 79, 68, 0.9)",
      fill: "rgba(39, 79, 68, 0.08)"
    }
  ],
  wells: [
    {
      name: "Well A",
      position: { x: 420, y: 480 },
      trajectory: [
        { x: 420, y: 480 },
        { x: 520, y: 620 },
        { x: 610, y: 760 }
      ],
      color: "#0e7490"
    },
    {
      name: "Well B",
      position: { x: 1240, y: 760 },
      trajectory: [
        { x: 1240, y: 760 },
        { x: 1320, y: 860 },
        { x: 1400, y: 980 }
      ],
      color: "#9a3412"
    },
    {
      name: "Well C",
      position: { x: 1680, y: 1180 },
      trajectory: [
        { x: 1680, y: 1180 },
        { x: 1750, y: 1280 },
        { x: 1820, y: 1390 }
      ],
      color: "#4d7c0f"
    }
  ]
};

export const rockPhysics: RockPhysicsCrossplotData = {
  templateId: "vp-vs-vs-ai",
  title: "Vp/Vs vs AI",
  subtitle: "Small public model example",
  groups: [
    { name: "Well A", color: "#0f766e" },
    { name: "Well B", color: "#b45309", symbol: "diamond" }
  ],
  points: [
    { x: 5850, y: 1.62, group: "Well A", depthM: 2410 },
    { x: 6120, y: 1.68, group: "Well A", depthM: 2422 },
    { x: 6490, y: 1.74, group: "Well A", depthM: 2436 },
    { x: 6820, y: 1.81, group: "Well B", depthM: 2448 },
    { x: 7180, y: 1.89, group: "Well B", depthM: 2462 },
    { x: 7560, y: 1.96, group: "Well B", depthM: 2474 }
  ]
};

export const wellPanel: WellCorrelationPanelData = {
  name: "Well Correlation",
  depthDomain: {
    start: 1500,
    end: 1620,
    unit: "m",
    label: "MD"
  },
  background: "#faf7f2",
  wells: [
    {
      name: "Well A",
      depthDatum: "md",
      curves: [
        {
          name: "GR",
          color: "#1f2937",
          values: Float32Array.from([72, 86, 102, 118, 94, 88, 76]),
          depths: Float32Array.from([1500, 1520, 1540, 1560, 1580, 1600, 1620]),
          unit: "API",
          axis: {
            min: 0,
            max: 180,
            label: "GR",
            unit: "API"
          }
        }
      ],
      tops: [
        {
          name: "Reservoir Top",
          depth: 1540,
          color: "#b45309",
          source: "picked"
        }
      ]
    },
    {
      name: "Well B",
      depthDatum: "md",
      curves: [
        {
          name: "GR",
          color: "#334155",
          values: Float32Array.from([64, 78, 92, 108, 116, 104, 90]),
          depths: Float32Array.from([1500, 1520, 1540, 1560, 1580, 1600, 1620]),
          unit: "API",
          axis: {
            min: 0,
            max: 180,
            label: "GR",
            unit: "API"
          }
        }
      ],
      tops: [
        {
          name: "Reservoir Top",
          depth: 1546,
          color: "#b45309",
          source: "picked"
        }
      ]
    }
  ]
};
