import type {
  CurveSeries,
  CurveTrack,
  FilledCurveTrack,
  TrackAxis,
  WellCorrelationPanelModel
} from "@ophiolite/charts-data-models";
import type { WellPanelModel } from "@ophiolite/charts-data-models";
import type {
  WellCorrelationPanelData,
  WellCorrelationPanelSimpleCurve,
  WellCorrelationPanelSimpleData,
  WellCorrelationPanelSimpleWell
} from "./types";

const wellCorrelationCache = new WeakMap<object, WellCorrelationPanelModel>();
const DEFAULT_REFERENCE_TRACK_WIDTH = 72;
const DEFAULT_CURVE_TRACK_WIDTH = 104;
const DEFAULT_TOPS_TRACK_WIDTH = 72;
const DEFAULT_CURVE_COLORS = ["#1f2937", "#0f766e", "#b45309", "#1d4ed8", "#7c3aed"];

export function adaptWellCorrelationPanelInputToModel(
  input: WellCorrelationPanelData | null
): WellCorrelationPanelModel | WellPanelModel | null {
  if (!input) {
    return null;
  }
  if (isAdvancedWellCorrelationPanel(input)) {
    return input;
  }

  const cached = wellCorrelationCache.get(input);
  if (cached) {
    return cached;
  }

  const normalizedWells = input.wells.map((well, wellIndex) => adaptWell(well, wellIndex));
  const depthDomain = input.depthDomain ?? deriveDepthDomain(normalizedWells);
  const normalized: WellCorrelationPanelModel = {
    id: input.id ?? slugify(input.name, "well-correlation-panel"),
    name: input.name,
    depthDomain,
    wells: normalizedWells,
    background: input.background
  };

  wellCorrelationCache.set(input, normalized);
  return normalized;
}

function isAdvancedWellCorrelationPanel(
  input: WellCorrelationPanelData
): input is WellCorrelationPanelModel | WellPanelModel {
  const firstWell = input.wells[0];
  return Boolean(firstWell && ("tracks" in firstWell || "data" in firstWell));
}

function adaptWell(
  well: WellCorrelationPanelSimpleWell,
  wellIndex: number
): WellCorrelationPanelModel["wells"][number] {
  const mapping = resolvePanelDepthMapping(well);
  const tracks: WellCorrelationPanelModel["wells"][number]["tracks"] = [
    {
      kind: "reference",
      id: `${well.id ?? slugify(well.name, `well-${wellIndex + 1}`)}:reference`,
      title: "Depth",
      width: DEFAULT_REFERENCE_TRACK_WIDTH
    }
  ];

  (well.curves ?? []).forEach((curve, curveIndex) => {
    tracks.push(adaptCurveTrack(well, curve, curveIndex));
  });

  if ((well.tops?.length ?? 0) > 0) {
    tracks.push({
      kind: "tops",
      id: `${well.id ?? slugify(well.name, `well-${wellIndex + 1}`)}:tops`,
      title: "Tops",
      width: DEFAULT_TOPS_TRACK_WIDTH
    });
  }

  return {
    id: well.id ?? slugify(well.name, `well-${wellIndex + 1}`),
    name: well.name,
    nativeDepthDatum: well.depthDatum ?? "md",
    panelDepthMapping: mapping,
    tracks,
    tops: (well.tops ?? []).map((top, index) => ({
      id: top.id ?? `${well.id ?? slugify(well.name, `well-${wellIndex + 1}`)}:top:${index + 1}`,
      name: top.name,
      nativeDepth: top.depth,
      color: top.color ?? "#b45309",
      source: top.source ?? "picked"
    })),
    headerNote: well.headerNote
  };
}

function adaptCurveTrack(
  well: WellCorrelationPanelSimpleWell,
  curve: WellCorrelationPanelSimpleCurve,
  curveIndex: number
): CurveTrack | FilledCurveTrack {
  const nativeDepths = Float32Array.from(toNumberArray(curve.depths));
  const values = Float32Array.from(toNumberArray(curve.values));
  const axis = resolveCurveAxis(curve, values);
  const series: [CurveSeries] = [
    {
      id: curve.id ?? `${well.id ?? slugify(well.name, "well")}:curve:${curveIndex + 1}`,
      name: curve.name,
      color: curve.color ?? DEFAULT_CURVE_COLORS[curveIndex % DEFAULT_CURVE_COLORS.length]!,
      values,
      nativeDepths,
      lineWidth: curve.lineWidth ?? 1.1,
      axis
    }
  ];

  if (curve.fill) {
    return {
      kind: "filled-curve",
      id: `${series[0].id}:track`,
      title: curve.name,
      width: curve.width ?? DEFAULT_CURVE_TRACK_WIDTH,
      xAxis: axis,
      series,
      fill: {
        direction: curve.fill.direction ?? "right",
        baseline: curve.fill.baseline ?? axis.min,
        color: curve.fill.color,
        gradientStops: curve.fill.gradientStops?.map((stop) => ({ ...stop }))
      }
    };
  }

  return {
    kind: "curve",
    id: `${series[0].id}:track`,
    title: curve.name,
    width: curve.width ?? DEFAULT_CURVE_TRACK_WIDTH,
    xAxis: axis,
    series
  };
}

function resolveCurveAxis(curve: WellCorrelationPanelSimpleCurve, values: Float32Array): TrackAxis {
  let min = curve.axis?.min;
  let max = curve.axis?.max;

  if (min === undefined || max === undefined) {
    const range = deriveSeriesRange(values);
    min ??= range.min;
    max ??= range.max;
  }

  return {
    min,
    max,
    label: curve.axis?.label ?? curve.name,
    unit: curve.axis?.unit ?? curve.unit,
    tickCount: curve.axis?.tickCount ?? 4,
    scale: curve.axis?.scale ?? "linear"
  };
}

function resolvePanelDepthMapping(well: WellCorrelationPanelSimpleWell): Array<{ nativeDepth: number; panelDepth: number }> {
  if (well.panelDepthMapping?.length) {
    return well.panelDepthMapping.map((sample) => ({
      nativeDepth: sample.nativeDepth,
      panelDepth: sample.panelDepth
    }));
  }

  const nativeDepths = new Set<number>();
  (well.curves ?? []).forEach((curve) => {
    for (const depth of toNumberArray(curve.depths)) {
      if (Number.isFinite(depth)) {
        nativeDepths.add(depth);
      }
    }
  });
  (well.tops ?? []).forEach((top) => {
    if (Number.isFinite(top.depth)) {
      nativeDepths.add(top.depth);
    }
  });

  const sorted = [...nativeDepths].sort((left, right) => left - right);
  return sorted.map((depth) => ({
    nativeDepth: depth,
    panelDepth: depth
  }));
}

function deriveDepthDomain(
  wells: WellCorrelationPanelModel["wells"]
): WellCorrelationPanelModel["depthDomain"] {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;

  wells.forEach((well) => {
    well.panelDepthMapping.forEach((sample) => {
      min = Math.min(min, sample.panelDepth);
      max = Math.max(max, sample.panelDepth);
    });
    well.tops.forEach((top) => {
      min = Math.min(min, top.nativeDepth);
      max = Math.max(max, top.nativeDepth);
    });
  });

  if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) {
    min = 0;
    max = 1000;
  }

  return {
    start: min,
    end: max,
    unit: "m",
    label: "Depth"
  };
}

function deriveSeriesRange(values: Float32Array): { min: number; max: number } {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;

  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (!Number.isFinite(value)) {
      continue;
    }
    min = Math.min(min, value);
    max = Math.max(max, value);
  }

  if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) {
    return {
      min: Number.isFinite(min) ? min - 1 : 0,
      max: Number.isFinite(max) ? max + 1 : 1
    };
  }

  const span = max - min;
  return {
    min: min - span * 0.06,
    max: max + span * 0.06
  };
}

function toNumberArray(values: ArrayLike<number>): number[] {
  return Array.from({ length: values.length }, (_, index) => values[index] ?? 0);
}

function slugify(value: string, fallback: string): string {
  const normalized = value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || fallback;
}
