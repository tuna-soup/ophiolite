import type {
  SectionColorMap,
  SectionInteractionChanged,
  SectionPolarity,
  SectionPrimaryMode,
  SectionProbeChanged,
  SectionRenderMode,
  SectionViewport,
  SectionViewportChanged
} from "@ophiolite/contracts";
import type { CursorProbe, SectionPayload, SectionViewport as InternalViewport } from "@ophiolite/charts-data-models";
import type { SeismicChartDisplayTransform, SeismicChartPrimaryMode, SectionViewLike } from "./types";
import {
  decodeFloat32,
  decodeFloat64,
  decodeOptionalFloat64,
  fromContractColorMap,
  fromContractPolarity,
  fromContractRenderMode,
  toContractPrimaryMode
} from "./seismic-contract-adapter-shared";

const sectionPayloadCache = new WeakMap<SectionViewLike, SectionPayload>();

export function decodeSectionView(contract: SectionViewLike): SectionPayload {
  const cached = sectionPayloadCache.get(contract);
  if (cached) {
    return cached;
  }

  const payload: SectionPayload = {
    axis: contract.axis,
    coordinate: {
      index: contract.coordinate.index,
      value: contract.coordinate.value
    },
    horizontalAxis: decodeFloat64(contract.horizontal_axis_f64le),
    inlineAxis: decodeOptionalFloat64(contract.inline_axis_f64le),
    xlineAxis: decodeOptionalFloat64(contract.xline_axis_f64le),
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
    presentation: decodeSectionPresentation(contract),
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

  sectionPayloadCache.set(contract, payload);
  return payload;
}

export function viewportFromContract(viewport: SectionViewport): InternalViewport {
  return {
    traceStart: viewport.trace_start,
    traceEnd: viewport.trace_end,
    sampleStart: viewport.sample_start,
    sampleEnd: viewport.sample_end
  };
}

export function viewportToContract(
  chartId: string,
  viewId: string,
  viewport: InternalViewport
): SectionViewportChanged {
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

export function probeToContract(
  chartId: string,
  viewId: string,
  probe: CursorProbe | null
): SectionProbeChanged {
  return {
    chart_id: chartId,
    view_id: viewId,
    probe: probe
      ? {
          trace_index: probe.traceIndex,
          trace_coordinate: probe.traceCoordinate,
          inline_coordinate: probe.inlineCoordinate ?? null,
          xline_coordinate: probe.xlineCoordinate ?? null,
          sample_index: probe.sampleIndex,
          sample_value: probe.sampleValue,
          amplitude: probe.amplitude
        }
      : null
  };
}

export function interactionToContract(
  chartId: string,
  viewId: string,
  primaryMode: SeismicChartPrimaryMode,
  crosshairEnabled: boolean
): SectionInteractionChanged {
  return {
    chart_id: chartId,
    view_id: viewId,
    primary_mode: toContractPrimaryMode(primaryMode),
    crosshair_enabled: crosshairEnabled
  };
}

export function isCompatibleSectionIdentity(previous: SectionViewLike | null, next: SectionViewLike | null): boolean {
  if (!previous || !next) {
    return false;
  }
  return (
    previous.axis === next.axis &&
    previous.coordinate.index === next.coordinate.index &&
    Math.abs(previous.coordinate.value - next.coordinate.value) <= 1e-6 &&
    previous.traces === next.traces &&
    previous.samples === next.samples &&
    previous.horizontal_axis_f64le.length === next.horizontal_axis_f64le.length &&
    (previous.inline_axis_f64le?.length ?? 0) === (next.inline_axis_f64le?.length ?? 0) &&
    (previous.xline_axis_f64le?.length ?? 0) === (next.xline_axis_f64le?.length ?? 0) &&
    previous.sample_axis_f32le.length === next.sample_axis_f32le.length
  );
}

export function mergeDisplayTransform(
  contract: SectionViewLike | null,
  override: Partial<SeismicChartDisplayTransform> | undefined
): SeismicChartDisplayTransform {
  const defaults = contract?.display_defaults;
  return {
    gain: override?.gain ?? defaults?.gain ?? 1,
    clipMin: override?.clipMin ?? defaults?.clip_min ?? undefined,
    clipMax: override?.clipMax ?? defaults?.clip_max ?? undefined,
    renderMode: override?.renderMode ?? fromContractRenderMode((defaults?.render_mode ?? "heatmap") as SectionRenderMode),
    colormap: override?.colormap ?? fromContractColorMap((defaults?.colormap ?? "grayscale") as SectionColorMap),
    polarity: override?.polarity ?? fromContractPolarity((defaults?.polarity ?? "normal") as SectionPolarity)
  };
}

function decodeSectionPresentation(contract: SectionViewLike): SectionPayload["presentation"] | undefined {
  const sampleAxisLabel = sectionSampleAxisLabel(contract);
  if (!sampleAxisLabel) {
    return undefined;
  }
  return { sampleAxisLabel };
}

function sectionSampleAxisLabel(contract: SectionViewLike): string | undefined {
  const notes = contract.metadata?.notes ?? [];
  for (const note of notes) {
    if (note === "sample_domain:depth") {
      return "Depth";
    }
    if (note === "sample_domain:time") {
      return "Time";
    }
  }
  const unit = contract.units?.sample?.toLowerCase();
  if (!unit) {
    return undefined;
  }
  if (unit === "ms" || unit === "s") {
    return "Time";
  }
  if (unit === "m" || unit === "ft") {
    return "Depth";
  }
  return undefined;
}
