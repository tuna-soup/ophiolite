import type {
  CurveTrack,
  CurveSeries,
  FilledCurveTrack,
  LithologyTrack,
  MultiCurveTrack,
  ReferenceTrack,
  TopsTrack,
  WellColumn,
  WellCorrelationPanelModel,
  WellTop
} from "./well-correlation";

export function createMockCorrelationPanel(): WellCorrelationPanelModel {
  const wells = [
    createWell("stybarrow-2", "Stybarrow-2", -20),
    createWell("stybarrow-1", "Stybarrow-1", 0),
    createWell("stybarrow-3", "Stybarrow-3", 26)
  ];

  return {
    id: "mock-correlation",
    name: "Stybarrow Correlation Panel",
    depthDomain: {
      start: 1580,
      end: 2360,
      unit: "m",
      label: "Correlation Depth"
    },
    wells,
    background: "#faf7f2"
  };
}

function createWell(id: string, name: string, offset: number): WellColumn {
  const sampleCount = 900;
  const nativeDepths = Float32Array.from({ length: sampleCount }, (_, index) => 1550 + index * 0.95);
  const grValues = new Float32Array(sampleCount);
  const densityValues = new Float32Array(sampleCount);
  const neutronValues = new Float32Array(sampleCount);
  const sonicValues = new Float32Array(sampleCount);
  const resistivityValues = new Float32Array(sampleCount);
  const vshaleValues = new Float32Array(sampleCount);
  const vsandValues = new Float32Array(sampleCount);
  const vcarbonateValues = new Float32Array(sampleCount);

  for (let index = 0; index < sampleCount; index += 1) {
    const depth = nativeDepths[index]!;
    grValues[index] = 95 + Math.sin(depth / 18) * 28 + Math.sin(depth / 7.5) * 6 + offset * 0.15;
    densityValues[index] = 2.35 + Math.cos(depth / 23) * 0.18 + Math.sin(depth / 51) * 0.08;
    neutronValues[index] = 14 + Math.sin(depth / 25) * 18 + Math.cos(depth / 39) * 9 - offset * 0.12;
    sonicValues[index] = 105 + Math.sin(depth / 31) * 18 + Math.cos(depth / 11) * 7 - offset * 0.1;
    resistivityValues[index] =
      10 ** (0.35 + Math.sin(depth / 37) * 0.55 + Math.cos(depth / 16) * 0.24 + offset * 0.0025);
    const shale = clamp(35 + Math.sin(depth / 21) * 25 + Math.cos(depth / 44) * 10, 6, 86);
    const sand = clamp(40 + Math.cos(depth / 29) * 22 - Math.sin(depth / 13) * 7, 4, 82);
    const carbonate = Math.max(4, 100 - shale - sand);
    const total = shale + sand + carbonate;
    vshaleValues[index] = (shale / total) * 100;
    vsandValues[index] = (sand / total) * 100;
    vcarbonateValues[index] = (carbonate / total) * 100;
  }

  const referenceTrack: ReferenceTrack = {
    kind: "reference",
    id: `${id}-reference`,
    title: "Depth",
    width: 82
  };
  const gammaTrack: FilledCurveTrack = {
    kind: "filled-curve",
    id: `${id}-gr`,
    title: "GR",
    width: 110,
    xAxis: {
      min: 0,
      max: 180,
      label: "GR",
      unit: "API",
      tickCount: 4
    },
    series: [
      series(`${id}-gr-series`, "GR", "#202020", grValues, nativeDepths, 1, {
        min: 0,
        max: 180,
        label: "GR",
        unit: "API",
        tickCount: 4
      })
    ],
    fill: {
      color: "rgba(81, 214, 216, 0.72)",
      baseline: 0,
      direction: "right",
      gradientStops: [
        { offset: 0, color: "rgba(47, 101, 211, 0.9)" },
        { offset: 0.22, color: "rgba(82, 211, 228, 0.82)" },
        { offset: 0.45, color: "rgba(63, 212, 130, 0.78)" },
        { offset: 0.72, color: "rgba(248, 218, 83, 0.76)" },
        { offset: 1, color: "rgba(233, 108, 92, 0.74)" }
      ]
    }
  };
  const densityTrack: MultiCurveTrack = {
    kind: "multi-curve",
    id: `${id}-density-neutron`,
    title: "RHOB / NEU",
    width: 132,
    xAxis: {
      min: 1.9,
      max: 2.8,
      label: "RHOB / NEU",
      unit: "g/cc",
      tickCount: 4
    },
    series: [
      series(`${id}-density`, "RHOB", "#111111", densityValues, nativeDepths, 1.2, {
        min: 1.9,
        max: 2.8,
        label: "RHOB",
        unit: "g/cc",
        tickCount: 4
      }),
      series(
        `${id}-neutron`,
        "NEU Porosity",
        "#be7a2a",
        neutronValues,
        nativeDepths,
        1.1,
        {
          min: 45,
          max: -15,
          label: "NEU Porosity",
          unit: "%",
          tickCount: 4
        }
      )
    ],
    crossoverFill: {
      leftSeriesId: `${id}-density`,
      rightSeriesId: `${id}-neutron`,
      color: "rgba(246, 151, 123, 0.78)",
      fillWhen: "rightOf"
    }
  };
  const resistivityTrack: CurveTrack = {
    kind: "curve",
    id: `${id}-resistivity`,
    title: "Resistivity",
    width: 92,
    xAxis: {
      min: 0.2,
      max: 200,
      label: "RT",
      unit: "ohm.m",
      tickCount: 4,
      scale: "log"
    },
    series: [
      series(`${id}-rt`, "RT", "#2f8e68", resistivityValues, nativeDepths, 1.1, {
        min: 0.2,
        max: 200,
        label: "RT",
        unit: "ohm.m",
        tickCount: 4,
        scale: "log"
      })
    ]
  };
  const sonicTrack: CurveTrack = {
    kind: "curve",
    id: `${id}-sonic`,
    title: "SONIC",
    width: 88,
    xAxis: {
      min: 70,
      max: 150,
      label: "SONIC",
      unit: "us/ft",
      tickCount: 4
    },
    series: [
      series(`${id}-sonic-raw`, "SONIC", "#384b72", sonicValues, nativeDepths, 1.1, {
        min: 70,
        max: 150,
        label: "SONIC",
        unit: "us/ft",
        tickCount: 4
      }, {
        color: "rgba(234, 191, 115, 0.20)",
        baseline: 70,
        direction: "right",
        gradientStops: [
          { offset: 0, color: "rgba(249, 220, 154, 0.15)" },
          { offset: 1, color: "rgba(220, 157, 92, 0.33)" }
        ]
      })
    ]
  };
  const lithologyTrack: LithologyTrack = {
    kind: "lithology",
    id: `${id}-lithology`,
    title: "Lithology",
    width: 118,
    xAxis: {
      min: 0,
      max: 100,
      label: "Lithology",
      unit: "%",
      tickCount: 5
    },
    nativeDepths,
    components: [
      { id: `${id}-vsh`, name: "VShale", color: "#8c8c8c", values: vshaleValues },
      { id: `${id}-vsand`, name: "VSand", color: "#f2dd3d", values: vsandValues },
      { id: `${id}-vcarb`, name: "VCarbonate", color: "#2456c6", values: vcarbonateValues }
    ]
  };
  const topsTrack: TopsTrack = {
    kind: "tops",
    id: `${id}-tops`,
    title: "Tops",
    width: 70
  };

  return {
    id,
    name,
    nativeDepthDatum: "md",
    panelDepthMapping: Array.from({ length: sampleCount }, (_, index) => ({
      nativeDepth: nativeDepths[index]!,
      panelDepth: nativeDepths[index]! + offset + Math.sin(index / 170) * 6
    })),
    tracks: [referenceTrack, gammaTrack, densityTrack, resistivityTrack, sonicTrack, lithologyTrack, topsTrack],
    tops: buildTops(offset),
    headerNote: `MD`
  };
}


function buildTops(offset: number): WellTop[] {
  const topTemplates = [
    ["Whale", 1672, "#b64f4f"],
    ["Lower Barrow", 1752, "#c56156"],
    ["Pyrenees", 1812, "#c97d71"],
    ["Lower Muderong", 1916, "#c45a5a"],
    ["Macedon", 2104, "#9c3e3e"],
    ["Base Macedon", 2246, "#7c2d2d"]
  ] as const;

  return topTemplates.map(([name, nativeDepth, color]) => ({
    id: `${name.toLowerCase().replace(/\s+/g, "-")}-${offset}`,
    name,
    nativeDepth: nativeDepth + offset * 0.45,
    color,
    source: "imported"
  }));
}

function series(
  id: string,
  name: string,
  color: string,
  values: Float32Array,
  nativeDepths: Float32Array,
  lineWidth: number = 1,
  axis?: CurveSeries["axis"],
  fill?: CurveSeries["fill"]
): CurveSeries {
  return {
    id,
    name,
    color,
    values,
    nativeDepths,
    lineWidth,
    axis,
    fill
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
