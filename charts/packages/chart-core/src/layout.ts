import type { NormalizedTrack, NormalizedWellPanelModel } from "./well-panel-normalize";

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface TrackFrame {
  wellId: string;
  trackId: string;
  title: string;
  kind: NormalizedTrack["kind"];
  headerRect: Rect;
  bodyRect: Rect;
  rowCount: number;
}

export interface WellColumnFrame {
  wellId: string;
  headerRect: Rect;
  bodyRect: Rect;
  trackFrames: TrackFrame[];
  leftEdgeX: number;
  rightEdgeX: number;
}

export interface CorrelationPanelLayout {
  plotRect: Rect;
  columns: WellColumnFrame[];
  contentWidth: number;
  viewportWidth: number;
  viewportHeight: number;
  scrollbarRect: Rect;
  trackHeaderHeight: number;
  leftLabelGutterWidth: number;
  rightLabelGutterWidth: number;
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

export function layoutWellCorrelationPanel(
  panel: NormalizedWellPanelModel,
  width: number,
  height: number
): CorrelationPanelLayout {
  const trackHeaderHeight = computeTrackHeaderHeight(panel);
  const intrinsicWidth =
    PANEL_PADDING * 2 +
    LEFT_LABEL_GUTTER +
    RIGHT_LABEL_GUTTER +
    panel.wells.reduce((sum, well) => sum + well.tracks.reduce((trackSum, track) => trackSum + track.width, 0), 0) +
    COLUMN_GAP * Math.max(0, panel.wells.length - 1);
  const viewportWidth = Math.max(1, width - SCROLLBAR_GUTTER);
  const contentWidth = Math.max(viewportWidth, intrinsicWidth);
  const columnAreaWidth =
    panel.wells.reduce((sum, well) => sum + well.tracks.reduce((trackSum, track) => trackSum + track.width, 0), 0) +
    COLUMN_GAP * Math.max(0, panel.wells.length - 1);
  const plotRect = {
    x: PANEL_PADDING + LEFT_LABEL_GUTTER,
    y: PANEL_PADDING + WELL_HEADER_HEIGHT + trackHeaderHeight + HEADER_GAP,
    width: Math.max(1, columnAreaWidth),
    height: Math.max(1, height - (PANEL_PADDING * 2 + WELL_HEADER_HEIGHT + trackHeaderHeight + HEADER_GAP))
  };

  const columns: WellColumnFrame[] = [];
  let currentX = plotRect.x;
  for (const well of panel.wells) {
    const trackFrames: TrackFrame[] = [];
    const columnWidth = well.tracks.reduce((sum, track) => sum + track.width, 0);
    let trackX = currentX;
    for (const track of well.tracks) {
      const rowCount = trackRowCount(track);
      trackFrames.push({
        wellId: well.id,
        trackId: track.id,
        title: track.title,
        kind: track.kind,
        headerRect: {
          x: trackX,
          y: PANEL_PADDING + WELL_HEADER_HEIGHT,
          width: track.width,
          height: trackHeaderHeight
        },
        bodyRect: {
          x: trackX,
          y: plotRect.y,
          width: track.width,
          height: plotRect.height
        },
        rowCount
      });
      trackX += track.width;
    }

    columns.push({
      wellId: well.id,
      headerRect: {
        x: currentX,
        y: PANEL_PADDING,
        width: columnWidth,
        height: WELL_HEADER_HEIGHT
      },
      bodyRect: {
        x: currentX,
        y: plotRect.y,
        width: columnWidth,
        height: plotRect.height
      },
      trackFrames,
      leftEdgeX: currentX,
      rightEdgeX: currentX + columnWidth
    });
    currentX += columnWidth + COLUMN_GAP;
  }

  return {
    plotRect,
    columns,
    contentWidth,
    viewportWidth,
    viewportHeight: height,
    scrollbarRect: {
      x: width - SCROLLBAR_GUTTER,
      y: plotRect.y,
      width: SCROLLBAR_GUTTER,
      height: plotRect.height
    },
    trackHeaderHeight,
    leftLabelGutterWidth: LEFT_LABEL_GUTTER,
    rightLabelGutterWidth: RIGHT_LABEL_GUTTER
  };
}

function computeTrackHeaderHeight(panel: NormalizedWellPanelModel): number {
  const maxRows = Math.max(
    1,
    ...panel.wells.flatMap((well) => well.tracks.map((track) => trackRowCount(track)))
  );
  return TRACK_TITLE_HEIGHT + maxRows * TRACK_ROW_HEIGHT;
}

function trackRowCount(track: NormalizedTrack): number {
  if (track.kind === "scalar") {
    return Math.max(
      1,
      track.layers.filter((layer) => layer.kind !== "top-overlay").length
    );
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
