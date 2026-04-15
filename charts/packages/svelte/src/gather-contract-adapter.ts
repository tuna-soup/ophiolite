import type {
  GatherAxisKind,
  GatherInteractionChanged,
  GatherProbeChanged,
  GatherSampleDomain,
  GatherViewport,
  GatherViewportChanged
} from "@ophiolite/contracts";
import type { CursorProbe, GatherPayload, SectionViewport as InternalViewport } from "@ophiolite/charts-data-models";
import type { SeismicChartDisplayTransform, SeismicChartPrimaryMode, GatherViewLike } from "./types";
import {
  decodeFloat32,
  decodeFloat64,
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

  const payload: GatherPayload = {
    label: contract.label,
    gatherAxisKind: fromContractGatherAxisKind(contract.gather_axis_kind),
    sampleDomain: fromContractGatherSampleDomain(contract.sample_domain),
    horizontalAxis: decodeFloat64(contract.horizontal_axis_f64le),
    sampleAxis: decodeFloat32(contract.sample_axis_f32le),
    amplitudes: decodeFloat32(contract.amplitudes_f32le),
    dimensions: {
      traces: contract.traces,
      samples: contract.samples
    },
    units: contract.units
      ? {
          horizontal: contract.units.horizontal ?? undefined,
          sample: contract.units.sample ?? undefined,
          amplitude: contract.units.amplitude ?? undefined
        }
      : undefined,
    metadata: contract.metadata
      ? {
          storeId: contract.metadata.store_id ?? undefined,
          derivedFrom: contract.metadata.derived_from ?? undefined,
          notes: contract.metadata.notes
        }
      : undefined,
    displayDefaults: contract.display_defaults
      ? {
          gain: contract.display_defaults.gain,
          clipMin: contract.display_defaults.clip_min ?? undefined,
          clipMax: contract.display_defaults.clip_max ?? undefined,
          renderMode: fromContractRenderMode(contract.display_defaults.render_mode),
          colormap: fromContractColorMap(contract.display_defaults.colormap),
          polarity: fromContractPolarity(contract.display_defaults.polarity)
        }
      : undefined
  };

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

function fromContractGatherAxisKind(value: GatherAxisKind): GatherPayload["gatherAxisKind"] {
  switch (value) {
    case "offset":
      return "offset";
    case "angle":
      return "angle";
    case "azimuth":
      return "azimuth";
    case "shot":
      return "shot";
    case "receiver":
      return "receiver";
    case "cmp":
      return "cmp";
    case "trace_ordinal":
      return "trace-ordinal";
    default:
      return "unknown";
  }
}

function fromContractGatherSampleDomain(value: GatherSampleDomain): GatherPayload["sampleDomain"] {
  return value === "depth" ? "depth" : "time";
}
