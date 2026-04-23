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
import {
  resolveLogicalSectionDimensions,
  type CursorProbe,
  type SectionPayload,
  type SectionViewport as InternalViewport
} from "@ophiolite/charts-data-models";
import {
  adaptSeismicSectionInputToPayload
} from "./seismic-public-model";
import type {
  OphioliteSectionView,
  SeismicChartDisplayTransform,
  SeismicChartPrimaryMode,
  SeismicSectionData
} from "./types";
import {
  fromContractColorMap,
  fromContractPolarity,
  fromContractRenderMode,
  toContractPrimaryMode
} from "./seismic-contract-adapter-shared";

const sectionPayloadCache = new WeakMap<object, SectionPayload>();

export function decodeSectionView(section: SeismicSectionData | OphioliteSectionView): SectionPayload {
  const cached = sectionPayloadCache.get(section);
  if (cached) {
    return cached;
  }

  const payload = adaptSeismicSectionInputToPayload(section);
  sectionPayloadCache.set(section, payload);
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

export function canReuseSectionViewport(
  previous: SeismicSectionData | OphioliteSectionView | null,
  next: SeismicSectionData | OphioliteSectionView | null
): boolean {
  if (!previous || !next) {
    return false;
  }
  const previousLogical = resolveLogicalSectionDimensions(decodeSectionView(previous));
  const nextLogical = resolveLogicalSectionDimensions(decodeSectionView(next));
  return previousLogical.traces === nextLogical.traces && previousLogical.samples === nextLogical.samples;
}

export function shouldIgnoreExternalSectionViewport(
  previous: SeismicSectionData | OphioliteSectionView | null,
  next: SeismicSectionData | OphioliteSectionView | null,
  viewportKey: string | null,
  ignoredViewportKey: string | null
): boolean {
  if (!viewportKey) {
    return false;
  }
  if (ignoredViewportKey === viewportKey) {
    return true;
  }
  return !!previous && previous !== next && !canReuseSectionViewport(previous, next);
}

export function mergeDisplayTransform(
  section: SeismicSectionData | OphioliteSectionView | null,
  override: Partial<SeismicChartDisplayTransform> | undefined
): SeismicChartDisplayTransform {
  const defaults = section ? decodeSectionView(section).displayDefaults : undefined;
  return {
    gain: override?.gain ?? defaults?.gain ?? 1,
    clipMin: override?.clipMin ?? defaults?.clipMin ?? undefined,
    clipMax: override?.clipMax ?? defaults?.clipMax ?? undefined,
    renderMode: override?.renderMode ?? fromContractRenderMode((defaults?.renderMode ?? "heatmap") as SectionRenderMode),
    colormap:
      override?.colormap ??
      (defaults?.colormap === "red-white-blue"
        ? "red-white-blue"
        : fromContractColorMap((defaults?.colormap ?? "grayscale") as SectionColorMap)),
    polarity: override?.polarity ?? fromContractPolarity((defaults?.polarity ?? "normal") as SectionPolarity)
  };
}
