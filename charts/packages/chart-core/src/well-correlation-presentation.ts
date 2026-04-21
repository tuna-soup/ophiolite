import type {
  NormalizedSeismicSectionLayer,
  NormalizedSeismicTraceLayer,
  NormalizedTrack
} from "./well-panel-normalize";

export interface WellCorrelationHeaderRow {
  label: string;
  color: string;
  axis?: {
    min: number;
    max: number;
  };
}

export function buildWellCorrelationHeaderRows(
  track: NormalizedTrack,
  nativeDepthDatum: string
): WellCorrelationHeaderRow[] {
  if (track.kind === "reference") {
    return track.topOverlays.length > 0
      ? [{ label: track.topOverlays[0]!.name, color: track.topOverlays[0]!.style.color }]
      : [{ label: nativeDepthDatum.toUpperCase(), color: "#444444" }];
  }
  if (track.kind === "scalar") {
    return track.layers.filter((layer) => layer.kind !== "top-overlay").map((layer) => {
      if (layer.kind === "curve") {
        const axis = layer.series.axis ?? track.xAxis;
        return { label: layer.name, color: layer.series.color, axis: { min: axis.min, max: axis.max } };
      }
      if (layer.kind === "point-observation") {
        return { label: layer.name, color: layer.style.fillColor, axis: { min: layer.axis.min, max: layer.axis.max } };
      }
      return { label: layer.name, color: "#555555", axis: { min: track.xAxis.min, max: track.xAxis.max } };
    });
  }
  if (track.kind === "seismic-trace") {
    return track.layers
      .filter((layer): layer is NormalizedSeismicTraceLayer => layer.kind === "seismic-trace")
      .flatMap((layer) => layer.traces.map((trace) => ({ label: trace.name, color: trace.style.positiveFill })));
  }
  return track.layers
    .filter((layer): layer is NormalizedSeismicSectionLayer => layer.kind === "seismic-section")
    .map((layer) => ({
      label: `${layer.name} (${layer.style.renderMode})`,
      color: layer.style.renderMode === "wiggle" ? "#9c2d2d" : "#4b4b4b"
    }));
}

export function chooseWellCorrelationDepthStep(span: number): number {
  if (span <= 100) return 10;
  if (span <= 250) return 25;
  if (span <= 500) return 50;
  return 100;
}

export function formatWellCorrelationAxisValue(value: number): string {
  if (Math.abs(value) >= 100) return value.toFixed(0);
  if (Math.abs(value) >= 10) return value.toFixed(1);
  return value.toFixed(2);
}
