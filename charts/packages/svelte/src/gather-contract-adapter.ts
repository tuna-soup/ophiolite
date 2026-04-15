import type { GatherInteractionChanged, GatherProbeChanged, GatherViewport, GatherViewportChanged } from "@ophiolite/contracts";
import {
  adaptOphioliteGatherViewToPayload,
  type CursorProbe,
  type GatherPayload,
  type SectionViewport as InternalViewport
} from "@ophiolite/charts-data-models";
import type { SeismicChartDisplayTransform, SeismicChartPrimaryMode, GatherViewLike } from "./types";
import {
  fromContractColorMap,
  fromContractPolarity,
  fromContractRenderMode,
  toContractPrimaryMode
} from "./seismic-contract-adapter-shared";

const gatherPayloadCache = new WeakMap<GatherViewLike, GatherPayload>();

export function decodeGatherView(contract: GatherViewLike): GatherPayload {
  const cached = gatherPayloadCache.get(contract);
  if (cached) {
    return cached;
  }

  const payload: GatherPayload = adaptOphioliteGatherViewToPayload(contract);
  gatherPayloadCache.set(contract, payload);
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

export function isCompatibleGatherIdentity(previous: GatherViewLike | null, next: GatherViewLike | null): boolean {
  if (!previous || !next) {
    return false;
  }
  return (
    previous.gather_axis_kind === next.gather_axis_kind &&
    previous.traces === next.traces &&
    previous.samples === next.samples
  );
}

export function mergeGatherDisplayTransform(
  contract: GatherViewLike | null,
  override: Partial<SeismicChartDisplayTransform> | undefined
): SeismicChartDisplayTransform {
  const defaults = contract?.display_defaults;
  return {
    gain: override?.gain ?? defaults?.gain ?? 1,
    clipMin: override?.clipMin ?? defaults?.clip_min ?? undefined,
    clipMax: override?.clipMax ?? defaults?.clip_max ?? undefined,
    renderMode: override?.renderMode ?? fromContractRenderMode(defaults?.render_mode ?? "heatmap"),
    colormap: override?.colormap ?? fromContractColorMap(defaults?.colormap ?? "grayscale"),
    polarity: override?.polarity ?? fromContractPolarity(defaults?.polarity ?? "normal")
  };
}
