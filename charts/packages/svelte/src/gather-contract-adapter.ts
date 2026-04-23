import type { GatherInteractionChanged, GatherProbeChanged, GatherViewport, GatherViewportChanged } from "@ophiolite/contracts";
import {
  type CursorProbe,
  type GatherPayload,
  type SectionViewport as InternalViewport
} from "@ophiolite/charts-data-models";
import {
  adaptSeismicGatherInputToPayload
} from "./seismic-public-model";
import type {
  OphioliteGatherView,
  SeismicChartDisplayTransform,
  SeismicChartPrimaryMode,
  SeismicGatherData
} from "./types";
import {
  fromContractColorMap,
  fromContractPolarity,
  fromContractRenderMode,
  toContractPrimaryMode
} from "./seismic-contract-adapter-shared";

const gatherPayloadCache = new WeakMap<object, GatherPayload>();

export function decodeGatherView(gather: SeismicGatherData | OphioliteGatherView): GatherPayload {
  const cached = gatherPayloadCache.get(gather);
  if (cached) {
    return cached;
  }

  const payload = adaptSeismicGatherInputToPayload(gather);
  gatherPayloadCache.set(gather, payload);
  return payload;
}

export function gatherViewportFromContract(viewport: GatherViewport): InternalViewport {
  return {
    traceStart: viewport.trace_start,
    traceEnd: viewport.trace_end,
    sampleStart: viewport.sample_start,
    sampleEnd: viewport.sample_end
  };
}

export function gatherViewportToContract(
  chartId: string,
  viewId: string,
  viewport: InternalViewport
): GatherViewportChanged {
  return {
    chart_id: chartId,
    view_id: viewId,
    viewport: {
      trace_start: viewport.traceStart,
      trace_end: viewport.traceEnd,
      sample_start: viewport.sampleStart,
      sample_end: viewport.sampleEnd
    }
  };
}

export function gatherProbeToContract(
  chartId: string,
  viewId: string,
  probe: CursorProbe | null
): GatherProbeChanged {
  return {
    chart_id: chartId,
    view_id: viewId,
    probe: probe
      ? {
          trace_index: probe.traceIndex,
          trace_coordinate: probe.traceCoordinate,
          sample_index: probe.sampleIndex,
          sample_value: probe.sampleValue,
          amplitude: probe.amplitude
        }
      : null
  };
}

export function gatherInteractionToContract(
  chartId: string,
  viewId: string,
  primaryMode: SeismicChartPrimaryMode,
  crosshairEnabled: boolean
): GatherInteractionChanged {
  return {
    chart_id: chartId,
    view_id: viewId,
    primary_mode: toContractPrimaryMode(primaryMode),
    crosshair_enabled: crosshairEnabled
  };
}

export function isCompatibleGatherIdentity(
  previous: SeismicGatherData | OphioliteGatherView | null,
  next: SeismicGatherData | OphioliteGatherView | null
): boolean {
  if (!previous || !next) {
    return false;
  }
  const previousPayload = decodeGatherView(previous);
  const nextPayload = decodeGatherView(next);
  return (
    previousPayload.gatherAxisKind === nextPayload.gatherAxisKind &&
    previousPayload.dimensions.traces === nextPayload.dimensions.traces &&
    previousPayload.dimensions.samples === nextPayload.dimensions.samples
  );
}

export function mergeGatherDisplayTransform(
  gather: SeismicGatherData | OphioliteGatherView | null,
  override: Partial<SeismicChartDisplayTransform> | undefined
): SeismicChartDisplayTransform {
  const defaults = gather ? decodeGatherView(gather).displayDefaults : undefined;
  return {
    gain: override?.gain ?? defaults?.gain ?? 1,
    clipMin: override?.clipMin ?? defaults?.clipMin ?? undefined,
    clipMax: override?.clipMax ?? defaults?.clipMax ?? undefined,
    renderMode: override?.renderMode ?? fromContractRenderMode(defaults?.renderMode ?? "heatmap"),
    colormap:
      override?.colormap ??
      (defaults?.colormap === "red-white-blue"
        ? "red-white-blue"
        : fromContractColorMap(defaults?.colormap ?? "grayscale")),
    polarity: override?.polarity ?? fromContractPolarity(defaults?.polarity ?? "normal")
  };
}
