import { normalizeWellPanelModel } from "@ophiolite/charts-core";
import type { WellCorrelationPanelModel, WellPanelModel } from "@ophiolite/charts-data-models";

interface WellCorrelationStageMetrics {
  width: number;
  height: number;
  plotTop: number;
  plotRight: number;
  plotBottom: number;
  plotLeft: number;
}

const PANEL_PADDING = 12;
const LEFT_LABEL_GUTTER = 96;
const RIGHT_LABEL_GUTTER = 116;
const WELL_HEADER_HEIGHT = 24;
const HEADER_GAP = 4;
const COLUMN_GAP = 26;
const TRACK_TITLE_HEIGHT = 4;
const TRACK_ROW_HEIGHT = 20;
const SCROLLBAR_GUTTER = 18;
const MIN_STAGE_WIDTH = 960;
const MAX_STAGE_WIDTH = 1400;
const DEFAULT_STAGE_HEIGHT = 560;

export function resolveWellCorrelationStageMetrics(
  panel: WellCorrelationPanelModel | WellPanelModel | null,
  scale: number | null | undefined
): WellCorrelationStageMetrics {
  const normalizedPanel = normalizeWellPanelModel(panel);
  const trackHeaderHeight = normalizedPanel ? computeTrackHeaderHeight(normalizedPanel) : TRACK_TITLE_HEIGHT + TRACK_ROW_HEIGHT;
  const intrinsicWidth = normalizedPanel
    ? PANEL_PADDING * 2 +
      LEFT_LABEL_GUTTER +
      RIGHT_LABEL_GUTTER +
      normalizedPanel.wells.reduce(
        (sum, well) => sum + well.tracks.reduce((trackSum, track) => trackSum + track.width, 0),
        0
      ) +
      COLUMN_GAP * Math.max(0, normalizedPanel.wells.length - 1) +
      SCROLLBAR_GUTTER
    : MIN_STAGE_WIDTH;

  const safeScale = Number.isFinite(scale) && (scale ?? 0) > 0 ? (scale as number) : 1;

  return {
    width: Math.max(1, Math.round(clamp(intrinsicWidth, MIN_STAGE_WIDTH, MAX_STAGE_WIDTH) * safeScale)),
    height: Math.max(1, Math.round(DEFAULT_STAGE_HEIGHT * safeScale)),
    plotTop: PANEL_PADDING + WELL_HEADER_HEIGHT + trackHeaderHeight + HEADER_GAP,
    plotRight: PANEL_PADDING + RIGHT_LABEL_GUTTER + SCROLLBAR_GUTTER,
    plotBottom: PANEL_PADDING,
    plotLeft: PANEL_PADDING + LEFT_LABEL_GUTTER
  };
}

function computeTrackHeaderHeight(panel: NonNullable<ReturnType<typeof normalizeWellPanelModel>>): number {
  const maxRows = Math.max(
    1,
    ...panel.wells.flatMap((well) => well.tracks.map((track) => trackRowCount(track)))
  );
  return TRACK_TITLE_HEIGHT + maxRows * TRACK_ROW_HEIGHT;
}

function trackRowCount(track: NonNullable<ReturnType<typeof normalizeWellPanelModel>>["wells"][number]["tracks"][number]): number {
  if (track.kind === "scalar") {
    return Math.max(1, track.layers.filter((layer) => layer.kind !== "top-overlay").length);
  }
  if (track.kind === "seismic-trace") {
    return Math.max(
      1,
      ...track.layers
        .filter((layer): layer is Extract<typeof layer, { kind: "seismic-trace" }> => layer.kind === "seismic-trace")
        .map((layer) => layer.traces.length)
    );
  }
  if (track.kind === "seismic-section") {
    return 1;
  }
  return Math.max(0, track.topOverlays.length > 0 ? 1 : 0);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
