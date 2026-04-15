import type {
  OphioliteResolvedDrillingObservationRow,
  OphioliteResolvedLogCurve,
  OphioliteResolvedPressureObservationRow,
  OphioliteResolvedSeismicSectionAsset,
  OphioliteResolvedSeismicTraceSetAsset,
  OphioliteResolvedTopRow,
  OphioliteResolvedWellPanelColumn,
  OphioliteResolvedWellPanelSource,
  OphioliteWellPanelLayout
} from "./ophiolite-well-panel-adapter";
import { adaptOphioliteWellPanelToChart } from "./ophiolite-well-panel-adapter";
import type { SectionPayload } from "./seismic";
import type { WellPanelModel } from "./well-panel";
import type { TrackAxis } from "./well-correlation";

const PANEL_DEPTH_START = 1580;
const PANEL_DEPTH_END = 2360;

export function createMockWellPanel(): WellPanelModel {
  return adaptOphioliteWellPanelToChart(
    createMockOphioliteWellPanelSource(),
    createMockWellPanelLayout()
  );
}

export function createMockOphioliteWellPanelSource(): OphioliteResolvedWellPanelSource {
  return {
    id: "mock-well-panel",
    name: "Stybarrow Layered Well Panel",
    schema_version: 1,
    depth_domain: {
      start: PANEL_DEPTH_START,
      end: PANEL_DEPTH_END,
      unit: "m",
      label: "Correlation Depth"
    },
    background: "#faf7f2",
    wells: [
      createWellColumn("stybarrow-2", "Stybarrow-2", -20, "heatmap"),
      createWellColumn("stybarrow-1", "Stybarrow-1", 0, "wiggle"),
      createWellColumn("stybarrow-3", "Stybarrow-3", 26, "heatmap")
    ]
  };
}

export function createMockWellPanelLayout(): OphioliteWellPanelLayout {
  return {
    wells: [
      createWellLayout("stybarrow-2", "heatmap"),
      createWellLayout("stybarrow-1", "wiggle"),
      createWellLayout("stybarrow-3", "heatmap")
    ]
  };
}

function createWellColumn(
  id: string,
  name: string,
  offset: number,
  sectionRenderMode: "heatmap" | "wiggle"
): OphioliteResolvedWellPanelColumn {
  const sampleCount = 900;
  const nativeDepths = Float32Array.from({ length: sampleCount }, (_, index) => 1550 + index * 0.95);
  const panelDepthMapping = Array.from({ length: sampleCount }, (_, index) => ({
    native_depth: nativeDepths[index]!,
    panel_depth: nativeDepths[index]! + offset + Math.sin(index / 170) * 6
  }));

  const gammaRay = buildLogCurve(id, "gr", "GR", "GammaRay", "API", nativeDepths, (depth) =>
    95 + Math.sin(depth / 18) * 28 + Math.sin(depth / 7.5) * 6 + offset * 0.15
  );
  const density = buildLogCurve(id, "rhob", "RHOB", "BulkDensity", "g/cc", nativeDepths, (depth) =>
    2.35 + Math.cos(depth / 23) * 0.18 + Math.sin(depth / 51) * 0.08
  );

  const neutron = buildLogCurve(id, "nphi", "NPHI", "NeutronPorosity", "%", nativeDepths, (depth) =>
    14 + Math.sin(depth / 25) * 18 + Math.cos(depth / 39) * 9 - offset * 0.12
  );

  const vshale = buildLogCurve(id, "vsh", "VSH", "VShale", "%", nativeDepths, (depth) =>
    clamp(35 + Math.sin(depth / 21) * 25 + Math.cos(depth / 44) * 10, 6, 86)
  );

  return {
    well_id: id,
    wellbore_id: id,
    name,
    native_depth_datum: "measured_depth",
    panel_depth_mapping: panelDepthMapping,
    header_note: "MD",
    logs: [gammaRay, density, neutron, vshale],
    trajectories: [],
    top_sets: [
      {
        asset_id: `${id}-tops`,
        logical_asset_id: `${id}-tops-logical`,
        asset_name: `${name} tops`,
        rows: buildTopRows(offset)
      }
    ],
    pressure_axis: axis(0, 100, "Observations", "%"),
    pressure_observations: [
      {
        asset_id: `${id}-pressure`,
        logical_asset_id: `${id}-pressure-logical`,
        asset_name: "Pressure Observations",
        rows: buildPressureRows(nativeDepths, 68, 118, (depth) =>
          clamp(54 + Math.sin(depth / 32) * 22 + offset * 0.18, 8, 96)
        )
      }
    ],
    drilling_axis: axis(0, 100, "Observations", "%"),
    drilling_observations: [
      {
        asset_id: `${id}-drilling`,
        logical_asset_id: `${id}-drilling-logical`,
        asset_name: "Drilling Observations",
        rows: buildDrillingRows(nativeDepths, 42, 145, (depth) =>
          clamp(40 + Math.cos(depth / 17) * 28 - offset * 0.15, 5, 92)
        )
      }
    ],
    seismic_trace_sets: [createTraceData(id, panelDepthMapping, offset)],
    seismic_sections: [createSectionData(id, panelDepthMapping, offset, sectionRenderMode)]
  };
}

function createWellLayout(
  id: string,
  sectionRenderMode: "heatmap" | "wiggle"
): OphioliteWellPanelLayout["wells"][number] {
  return {
    wellId: id,
    tracks: [
      {
        kind: "reference",
        id: `${id}-reference`,
        title: "Depth",
        width: 82,
        layers: [
          {
            kind: "top-overlay",
            id: `${id}-reference-tops`,
            dataId: `${id}-tops`,
            style: topOverlayStyle()
          }
        ]
      },
      {
        kind: "scalar",
        id: `${id}-gamma`,
        title: "GR",
        width: 110,
        xAxis: axis(0, 180, "GR", "API"),
        layers: [
          {
            kind: "curve",
            id: `${id}-gamma-curve`,
            dataId: `${id}-gr`,
            style: {
              color: "#202020",
              lineWidth: 1.1,
              fill: {
                mode: "baseline",
                color: "rgba(81, 214, 216, 0.72)",
                baseline: 0,
                fillWhen: "rightOf",
                gradientStops: [
                  { offset: 0, color: "rgba(47, 101, 211, 0.9)" },
                  { offset: 0.22, color: "rgba(82, 211, 228, 0.82)" },
                  { offset: 0.45, color: "rgba(63, 212, 130, 0.78)" },
                  { offset: 0.72, color: "rgba(248, 218, 83, 0.76)" },
                  { offset: 1, color: "rgba(233, 108, 92, 0.74)" }
                ]
              }
            }
          },
          {
            kind: "top-overlay",
            id: `${id}-gamma-tops`,
            dataId: `${id}-tops`,
            style: topOverlayStyle()
          }
        ]
      },
      {
        kind: "scalar",
        id: `${id}-density-neutron`,
        title: "RHOB / NPHI",
        width: 132,
        xAxis: axis(1.9, 2.8, "RHOB / NPHI", "g/cc"),
        layers: [
          {
            kind: "curve",
            id: `${id}-density-curve`,
            dataId: `${id}-rhob`,
            style: {
              color: "#111111",
              lineWidth: 1.2,
              fill: {
                mode: "between-curves",
                color: "rgba(246, 151, 123, 0.78)",
                targetCurveId: `${id}-nphi`,
                fillWhen: "rightOf"
              }
            }
          },
          {
            kind: "curve",
            id: `${id}-neutron-curve`,
            dataId: `${id}-nphi`,
            style: {
              color: "#be7a2a",
              lineWidth: 1.1
            }
          },
          {
            kind: "top-overlay",
            id: `${id}-density-tops`,
            dataId: `${id}-tops`,
            style: topOverlayStyle("#9a4848")
          }
        ]
      },
      {
        kind: "scalar",
        id: `${id}-observations`,
        title: "Observations",
        width: 118,
        xAxis: axis(0, 100, "Observations", "%"),
        layers: [
          {
            kind: "curve",
            id: `${id}-vsh-curve`,
            dataId: `${id}-vsh`,
            style: {
              color: "#4d5968",
              lineWidth: 1.1,
              fill: {
                mode: "baseline",
                color: "rgba(142, 150, 163, 0.28)",
                baseline: 0,
                fillWhen: "rightOf"
              }
            }
          },
          {
            kind: "point-observation",
            id: `${id}-pressure-points`,
            dataId: `${id}-pressure`,
            style: {
              shape: "diamond",
              size: 7,
              fillColor: "#0c7e65",
              strokeColor: "#063e34",
              strokeWidth: 1
            }
          },
          {
            kind: "point-observation",
            id: `${id}-drilling-points`,
            dataId: `${id}-drilling`,
            style: {
              shape: "square",
              size: 7,
              fillColor: "#bf4e24",
              strokeColor: "#5b250f",
              strokeWidth: 1
            }
          },
          {
            kind: "top-overlay",
            id: `${id}-observations-tops`,
            dataId: `${id}-tops`,
            style: topOverlayStyle("#7b3a3a")
          }
        ]
      },
      {
        kind: "seismic-trace",
        id: `${id}-trace`,
        title: "Trace Tie",
        width: 96,
        layers: [
          {
            kind: "seismic-trace",
            id: `${id}-trace-layer`,
            dataId: `${id}-trace-data`,
            normalization: "shared-domain",
            styleByTraceId: {
              [`${id}-stack`]: {
                positiveFill: "#111111",
                negativeFill: "#ffffff",
                lineColor: "#111111",
                lineWidth: 1
              },
              [`${id}-synthetic`]: {
                positiveFill: "#e84545",
                negativeFill: "#ffffff",
                lineColor: "#8d1717",
                lineWidth: 1
              }
            }
          },
          {
            kind: "top-overlay",
            id: `${id}-trace-tops`,
            dataId: `${id}-tops`,
            style: topOverlayStyle("#7c2d2d")
          }
        ]
      },
      {
        kind: "seismic-section",
        id: `${id}-section`,
        title: "Inline 3269",
        width: 238,
        layers: [
          {
            kind: "seismic-section",
            id: `${id}-section-layer`,
            dataId: `${id}-section-data`,
            style: {
              transform: {
                gain: 1.15,
                renderMode: sectionRenderMode,
                colormap: sectionRenderMode === "wiggle" ? "grayscale" : "red-white-blue",
                polarity: "normal"
              }
            }
          },
          {
            kind: "top-overlay",
            id: `${id}-section-tops`,
            dataId: `${id}-tops`,
            style: topOverlayStyle("#7c2d2d")
          }
        ]
      }
    ]
  };
}

function createTraceData(
  id: string,
  panelDepthMapping: Array<{ native_depth: number; panel_depth: number }>,
  offset: number
): OphioliteResolvedSeismicTraceSetAsset {
  const sampleCount = 248;
  const nativeDepths = Float32Array.from({ length: sampleCount }, (_, index) => PANEL_DEPTH_START + 18 + index * 2.85);
  const panelDepths = Float32Array.from(nativeDepths, (depth) => mapDepthThroughMapping(panelDepthMapping, depth));
  const stack = new Float32Array(sampleCount);
  const synthetic = new Float32Array(sampleCount);

  for (let index = 0; index < sampleCount; index += 1) {
    const depth = nativeDepths[index]!;
    const strat = Math.sin(depth / 14.5 + offset * 0.015) * 0.72;
    const tuning = Math.sin(depth / 6.8) * 0.18;
    const event = Math.exp(-((depth - (1890 + offset * 0.6)) ** 2) / 1400) * 1.15;
    stack[index] = strat + tuning + event;
    synthetic[index] = Math.sin(depth / 15.2 + 0.45) * 0.66 + Math.sin(depth / 7.6) * 0.16 + event * 0.93;
  }

  return {
    id: `${id}-trace-data`,
    name: "Zero Offset / Synthetic",
    nativeDepths,
    panelDepths,
    amplitudeUnit: "arb",
    traces: [
      {
        id: `${id}-stack`,
        name: "Stack",
        amplitudes: stack
      },
      {
        id: `${id}-synthetic`,
        name: "Synthetic",
        amplitudes: synthetic
      }
    ]
  };
}

function createSectionData(
  id: string,
  panelDepthMapping: Array<{ native_depth: number; panel_depth: number }>,
  offset: number,
  sectionRenderMode: "heatmap" | "wiggle"
): OphioliteResolvedSeismicSectionAsset {
  const traces = 42;
  const samples = 248;
  const sampleAxis = Float32Array.from({ length: samples }, (_, index) => PANEL_DEPTH_START + 18 + index * 2.85);
  const panelDepths = Float32Array.from(sampleAxis, (depth) => mapDepthThroughMapping(panelDepthMapping, depth));
  const amplitudes = new Float32Array(traces * samples);

  for (let trace = 0; trace < traces; trace += 1) {
    for (let sample = 0; sample < samples; sample += 1) {
      const depth = sampleAxis[sample]!;
      const index = trace * samples + sample;
      const layered = Math.sin(sample / 6.8 + trace / 4.6 + offset * 0.025) * 0.68;
      const dip = Math.sin(sample / 17.5 - trace / 6.5) * 0.34;
      const channel = Math.exp(-((trace - 20) ** 2 + (sample - 122) ** 2) / 210) * 1.1;
      const baseReflector = Math.exp(-((sample - 198) ** 2) / 260) * (trace > 12 ? 0.82 : 0.35);
      amplitudes[index] = layered + dip + channel + baseReflector + Math.sin(depth / 43) * 0.06;
    }
  }

  const section: SectionPayload = {
    axis: "inline",
    coordinate: { index: 3269, value: 3269 },
    horizontalAxis: Float64Array.from({ length: traces }, (_, index) => index - Math.floor(traces / 2)),
    sampleAxis,
    amplitudes,
    dimensions: { traces, samples },
    units: {
      horizontal: "trace",
      sample: "m",
      amplitude: "arb"
    },
    metadata: {
      notes: ["Synthetic well-panel inline section aligned to the well depth domain."]
    },
    displayDefaults: {
      gain: 1.15,
      renderMode: sectionRenderMode,
      colormap: sectionRenderMode === "wiggle" ? "grayscale" : "red-white-blue",
      polarity: "normal"
    }
  };

  return {
    id: `${id}-section-data`,
    name: "Inline 3269",
    section,
    panelDepths,
    nativeDepths: sampleAxis,
    wellTraceIndex: Math.floor(traces / 2)
  };
}

function buildLogCurve(
  wellId: string,
  suffix: string,
  name: string,
  semanticType: string,
  unit: string,
  nativeDepths: Float32Array,
  valueAtDepth: (depth: number) => number
): OphioliteResolvedLogCurve {
  return {
    asset_id: `${wellId}-${suffix}`,
    logical_asset_id: `${wellId}-${suffix}-logical`,
    asset_name: `${wellId} ${name}`,
    curve_name: name,
    original_mnemonic: name,
    unit,
    semantic_type: semanticType,
    depths: Array.from(nativeDepths),
    values: Array.from(nativeDepths, (depth) => valueAtDepth(depth))
  };
}

function buildPressureRows(
  nativeDepths: Float32Array,
  startIndex: number,
  stride: number,
  valueAtDepth: (depth: number) => number
): OphioliteResolvedPressureObservationRow[] {
  const rows: OphioliteResolvedPressureObservationRow[] = [];
  for (let index = startIndex; index < nativeDepths.length; index += stride) {
    const measuredDepth = nativeDepths[index]!;
    rows.push({
      measured_depth: measuredDepth,
      pressure: valueAtDepth(measuredDepth),
      phase: index % 2 === 0 ? "oil" : "water",
      test_kind: index % 3 === 0 ? "RFT" : "MDT",
      timestamp: `2025-01-${String((index % 27) + 1).padStart(2, "0")}`
    });
  }
  return rows;
}

function buildDrillingRows(
  nativeDepths: Float32Array,
  startIndex: number,
  stride: number,
  valueAtDepth: (depth: number) => number
): OphioliteResolvedDrillingObservationRow[] {
  const rows: OphioliteResolvedDrillingObservationRow[] = [];
  for (let index = startIndex; index < nativeDepths.length; index += stride) {
    const measuredDepth = nativeDepths[index]!;
    rows.push({
      measured_depth: measuredDepth,
      event_kind: index % 2 === 0 ? "gas_show" : "mud_loss",
      value: valueAtDepth(measuredDepth),
      unit: "%",
      timestamp: `2025-02-${String((index % 27) + 1).padStart(2, "0")}`,
      comment: measuredDepth > 2000 ? "Reservoir entry" : "Background event"
    });
  }
  return rows;
}

function buildTopRows(offset: number): OphioliteResolvedTopRow[] {
  const topTemplates = [
    ["Whale", 1672],
    ["Lower Barrow", 1752],
    ["Pyrenees", 1812],
    ["Lower Muderong", 1916],
    ["Macedon", 2104],
    ["Base Macedon", 2246]
  ] as const;

  return topTemplates.map(([name, topDepth], index, rows) => ({
    name,
    top_depth: topDepth + offset * 0.45,
    base_depth: rows[index + 1]?.[1] ? rows[index + 1]![1] + offset * 0.45 : null,
    source: "interpretation",
    depth_reference: "measured_depth"
  }));
}

function topOverlayStyle(color: string = "#b44d4d") {
  return {
    color,
    lineWidth: 1,
    labelColor: "#5b2b2b",
    showLabels: true,
    editable: true
  };
}

function axis(min: number, max: number, label: string, unit: string): TrackAxis {
  return {
    min,
    max,
    label,
    unit,
    tickCount: 4
  };
}

function mapDepthThroughMapping(
  mapping: Array<{ native_depth: number; panel_depth: number }>,
  nativeDepth: number
): number {
  if (mapping.length === 0) {
    return nativeDepth;
  }
  if (nativeDepth <= mapping[0]!.native_depth) {
    return mapping[0]!.panel_depth;
  }
  for (let index = 1; index < mapping.length; index += 1) {
    const previous = mapping[index - 1]!;
    const current = mapping[index]!;
    if (nativeDepth <= current.native_depth) {
      const span = Math.max(1e-6, current.native_depth - previous.native_depth);
      const ratio = (nativeDepth - previous.native_depth) / span;
      return previous.panel_depth + (current.panel_depth - previous.panel_depth) * ratio;
    }
  }
  return mapping[mapping.length - 1]!.panel_depth;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
