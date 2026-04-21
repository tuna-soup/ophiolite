import type {
  ColorMapName,
  DisplayTransform,
  Horizon,
  OverlayPayload,
  RenderFrame,
  SectionHorizonOverlay,
  SectionWellOverlay,
  SectionScalarOverlay,
  SectionPayload,
  SectionViewport
} from "@ophiolite/charts-data-models";
import {
  sectionAmplitudeAt,
  sectionHorizontalCoordinateAt
} from "@ophiolite/charts-data-models";
import { sampleIndexToScreenY, traceIndexToScreenX } from "./sectionTransforms";
import { buildWigglePanelGeometry, mapCoordinateToPlotX, type PlotRect } from "./wiggleGeometry";

export interface RenderInvalidation {
  dataChanged: boolean;
  viewportChanged: boolean;
  styleChanged: boolean;
  selectionChanged: boolean;
  overlayChanged: boolean;
  sizeChanged: boolean;
  baseChanged: boolean;
  overlayNeedsDraw: boolean;
}

export interface BaseRenderState {
  section: SectionPayload | null;
  secondarySection: SectionPayload | null;
  viewport: SectionViewport | null;
  displayTransform: DisplayTransform;
  overlay: OverlayPayload | null;
  comparisonMode: RenderFrame["state"]["comparisonMode"];
  splitPosition: number;
  plotRect: PlotRect;
  width: number;
  height: number;
  pixelRatio: number;
}

export interface OverlayRenderState {
  section: SectionPayload | null;
  secondarySection: SectionPayload | null;
  viewport: SectionViewport | null;
  displayTransform: DisplayTransform;
  comparisonMode: RenderFrame["state"]["comparisonMode"];
  splitPosition: number;
  plotRect: PlotRect;
  width: number;
  height: number;
  pixelRatio: number;
  probe: RenderFrame["state"]["probe"];
  interactions: RenderFrame["state"]["interactions"];
  sectionScalarOverlays: SectionScalarOverlay[];
  sectionHorizonOverlays: SectionHorizonOverlay[];
  sectionWellOverlays: SectionWellOverlay[];
  horizons: Horizon[];
  activeHorizonId: string | null;
}

export interface PreparedHeatmapData {
  clipMin: number;
  clipMax: number;
  symmetricExtent: number;
  lut: Uint8Array;
  overlayEnabled: boolean;
  overlayOpacity: number;
}

export interface PreparedWiggleData {
  lineVertices: Float32Array;
  fillVertices: Float32Array;
  traceStride: number;
}

export interface PreparedWiggleInstances {
  traceIndices: Float32Array;
  baselineClipX: Float32Array;
  amplitudeScaleClip: Float32Array;
  sampleStart: number;
  sampleCount: number;
  traceStride: number;
}

export interface OverlaySpatialPoint {
  horizonId: string;
  anchorId: string;
  x: number;
  y: number;
}

export interface OverlaySpatialIndex {
  points: OverlaySpatialPoint[];
}

export function createBaseRenderState(
  frame: RenderFrame,
  plotRect: PlotRect,
  width: number,
  height: number,
  pixelRatio: number
): BaseRenderState {
  return {
    section: frame.state.section,
    secondarySection: frame.state.secondarySection,
    viewport: frame.state.viewport,
    displayTransform: frame.state.displayTransform,
    overlay: frame.state.overlay,
    comparisonMode: frame.state.comparisonMode,
    splitPosition: frame.state.splitPosition,
    plotRect,
    width,
    height,
    pixelRatio
  };
}

export function createOverlayRenderState(
  frame: RenderFrame,
  plotRect: PlotRect,
  width: number,
  height: number,
  pixelRatio: number
): OverlayRenderState {
  return {
    section: frame.state.section,
    secondarySection: frame.state.secondarySection,
    viewport: frame.state.viewport,
    displayTransform: frame.state.displayTransform,
    comparisonMode: frame.state.comparisonMode,
    splitPosition: frame.state.splitPosition,
    plotRect,
    width,
    height,
    pixelRatio,
    probe: frame.state.probe,
    interactions: frame.state.interactions,
    sectionScalarOverlays: frame.state.sectionScalarOverlays,
    sectionHorizonOverlays: frame.state.sectionHorizonOverlays,
    sectionWellOverlays: frame.state.sectionWellOverlays,
    horizons: frame.state.horizons,
    activeHorizonId: frame.state.activeHorizonId
  };
}

export function diffRenderStates(
  previousBase: BaseRenderState | null,
  nextBase: BaseRenderState,
  previousOverlay: OverlayRenderState | null,
  nextOverlay: OverlayRenderState
): RenderInvalidation {
  const dataChanged = previousBase?.section !== nextBase.section;
  const secondaryDataChanged = previousBase?.secondarySection !== nextBase.secondarySection;
  const viewportChanged =
    !previousBase?.viewport ||
    !nextBase.viewport ||
    previousBase.viewport.traceStart !== nextBase.viewport.traceStart ||
    previousBase.viewport.traceEnd !== nextBase.viewport.traceEnd ||
    previousBase.viewport.sampleStart !== nextBase.viewport.sampleStart ||
    previousBase.viewport.sampleEnd !== nextBase.viewport.sampleEnd;
  const styleChanged =
    !previousBase ||
    previousBase.displayTransform.gain !== nextBase.displayTransform.gain ||
    previousBase.displayTransform.clipMin !== nextBase.displayTransform.clipMin ||
    previousBase.displayTransform.clipMax !== nextBase.displayTransform.clipMax ||
    previousBase.displayTransform.renderMode !== nextBase.displayTransform.renderMode ||
    previousBase.displayTransform.colormap !== nextBase.displayTransform.colormap ||
    previousBase.displayTransform.polarity !== nextBase.displayTransform.polarity ||
    previousBase.comparisonMode !== nextBase.comparisonMode ||
    previousBase.splitPosition !== nextBase.splitPosition;
  const overlayChanged =
    previousBase?.overlay !== nextBase.overlay ||
    previousBase?.overlay?.opacity !== nextBase.overlay?.opacity;
  const sizeChanged =
    !previousBase ||
    previousBase.width !== nextBase.width ||
    previousBase.height !== nextBase.height ||
    previousBase.pixelRatio !== nextBase.pixelRatio ||
    previousBase.plotRect.x !== nextBase.plotRect.x ||
    previousBase.plotRect.y !== nextBase.plotRect.y ||
    previousBase.plotRect.width !== nextBase.plotRect.width ||
    previousBase.plotRect.height !== nextBase.plotRect.height;
  const selectionChanged =
    !previousOverlay ||
    previousOverlay.probe !== nextOverlay.probe ||
    previousOverlay.interactions !== nextOverlay.interactions ||
    previousOverlay.sectionScalarOverlays !== nextOverlay.sectionScalarOverlays ||
    previousOverlay.sectionHorizonOverlays !== nextOverlay.sectionHorizonOverlays ||
    previousOverlay.sectionWellOverlays !== nextOverlay.sectionWellOverlays ||
    previousOverlay.activeHorizonId !== nextOverlay.activeHorizonId ||
    previousOverlay.horizons !== nextOverlay.horizons;

  return {
    dataChanged,
    viewportChanged,
    styleChanged,
    selectionChanged,
    overlayChanged,
    sizeChanged,
    baseChanged: dataChanged || secondaryDataChanged || viewportChanged || styleChanged || overlayChanged || sizeChanged,
    overlayNeedsDraw: dataChanged || viewportChanged || styleChanged || selectionChanged || sizeChanged
  };
}

export function prepareHeatmapData(
  section: SectionPayload,
  viewport: SectionViewport,
  displayTransform: DisplayTransform,
  overlay: OverlayPayload | null,
  secondarySection?: SectionPayload | null
): PreparedHeatmapData {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;

  for (const source of [section, secondarySection].filter((candidate): candidate is SectionPayload => Boolean(candidate))) {
    for (let trace = viewport.traceStart; trace < viewport.traceEnd; trace += 1) {
      for (let sample = viewport.sampleStart; sample < viewport.sampleEnd; sample += 1) {
        const amplitude = sectionAmplitudeAt(source, trace, sample);
        if (amplitude === null) {
          continue;
        }
        const value = amplitude * displayTransform.gain;
        min = Math.min(min, value);
        max = Math.max(max, value);
      }
    }
  }

  const clipMin = Number.isFinite(displayTransform.clipMin ?? min) ? (displayTransform.clipMin ?? min) : -1;
  const clipMax = Number.isFinite(displayTransform.clipMax ?? max) ? (displayTransform.clipMax ?? max) : 1;
  return {
    clipMin,
    clipMax,
    symmetricExtent: Math.max(Math.abs(clipMin), Math.abs(clipMax), 1e-6),
    lut: buildColorLut(displayTransform.colormap),
    overlayEnabled: Boolean(overlay) && displayTransform.renderMode === "heatmap",
    overlayOpacity: overlay?.opacity ?? 0.35
  };
}

export function prepareWiggleData(
  section: SectionPayload,
  viewport: SectionViewport,
  displayTransform: DisplayTransform,
  plotRect: PlotRect,
  canvasWidth: number,
  canvasHeight: number
): PreparedWiggleData {
  const panel = buildWigglePanelGeometry({
    section,
    traceStart: viewport.traceStart,
    traceEnd: viewport.traceEnd,
    sampleStart: viewport.sampleStart,
    sampleEnd: viewport.sampleEnd,
    gain: displayTransform.gain,
    polarity: displayTransform.polarity,
    plotRect,
    minTraceSpacingPx: 6,
    amplitudeRatio: 0.38
  });

  const lineVertices: number[] = [];
  const fillVertices: number[] = [];

  for (const trace of panel.traces) {
    for (let index = 1; index < trace.stroke.length; index += 1) {
      const previous = trace.stroke[index - 1];
      const current = trace.stroke[index];
      pushClipVertex(lineVertices, previous.x, previous.y, canvasWidth, canvasHeight);
      pushClipVertex(lineVertices, current.x, current.y, canvasWidth, canvasHeight);
    }

    for (const segment of trace.positiveFillSegments) {
      for (let index = 1; index < segment.length; index += 1) {
        const previous = segment[index - 1];
        const current = segment[index];
        pushClipVertex(fillVertices, trace.baselineX, previous.y, canvasWidth, canvasHeight);
        pushClipVertex(fillVertices, previous.x, previous.y, canvasWidth, canvasHeight);
        pushClipVertex(fillVertices, trace.baselineX, current.y, canvasWidth, canvasHeight);
        pushClipVertex(fillVertices, previous.x, previous.y, canvasWidth, canvasHeight);
        pushClipVertex(fillVertices, current.x, current.y, canvasWidth, canvasHeight);
        pushClipVertex(fillVertices, trace.baselineX, current.y, canvasWidth, canvasHeight);
      }
    }
  }

  return {
    lineVertices: Float32Array.from(lineVertices),
    fillVertices: Float32Array.from(fillVertices),
    traceStride: panel.traceStride
  };
}

export function prepareWiggleInstances(
  section: SectionPayload,
  viewport: SectionViewport,
  displayTransform: DisplayTransform,
  plotRect: PlotRect,
  canvasWidth: number
): PreparedWiggleInstances {
  const visibleTraceCount = Math.max(1, viewport.traceEnd - viewport.traceStart);
  const maxReadableTraces = Math.max(1, Math.floor(plotRect.width / 6));
  const traceStride = Math.max(1, Math.ceil(visibleTraceCount / maxReadableTraces));
  const traceIndices = buildTraceIndices(viewport.traceStart, viewport.traceEnd, traceStride);
  const visibleCoords = [];
  for (let trace = viewport.traceStart; trace < viewport.traceEnd; trace += 1) {
    const coordinate = sectionHorizontalCoordinateAt(section, trace);
    if (coordinate !== null) {
      visibleCoords.push(coordinate);
    }
  }
  if (visibleCoords.length === 0) {
    visibleCoords.push(viewport.traceStart, Math.max(viewport.traceStart + 1, viewport.traceEnd - 1));
  }
  const coordMin = Math.min(...visibleCoords);
  const coordMax = Math.max(...visibleCoords);
  const globalScale = visibleAmplitudeScale(
    section,
    viewport.traceStart,
    viewport.traceEnd,
    viewport.sampleStart,
    viewport.sampleEnd,
    displayTransform.gain
  );

  const baselineClipX: number[] = [];
  const amplitudeScaleClip: number[] = [];

  for (const traceIndex of traceIndices) {
    const baselineX = mapCoordinateToPlotX(
      sectionHorizontalCoordinateAt(section, traceIndex) ?? traceIndex,
      coordMin,
      coordMax,
      plotRect
    );
    const previous = traceIndex - traceStride >= viewport.traceStart ? traceIndex - traceStride : null;
    const next = traceIndex + traceStride < viewport.traceEnd ? traceIndex + traceStride : null;
    const previousX =
      previous === null
        ? Number.POSITIVE_INFINITY
        : mapCoordinateToPlotX(
            sectionHorizontalCoordinateAt(section, previous) ?? previous,
            coordMin,
            coordMax,
            plotRect
          );
    const nextX =
      next === null
        ? Number.POSITIVE_INFINITY
        : mapCoordinateToPlotX(
            sectionHorizontalCoordinateAt(section, next) ?? next,
            coordMin,
            coordMax,
            plotRect
          );
    const spacing = Math.min(
      Number.isFinite(previousX) ? Math.abs(baselineX - previousX) : Number.POSITIVE_INFINITY,
      Number.isFinite(nextX) ? Math.abs(nextX - baselineX) : Number.POSITIVE_INFINITY
    );
    const localSpacing = !Number.isFinite(spacing) || spacing === 0 ? plotRect.width : spacing;
    const amplitudeScalePx = Math.max(localSpacing * 0.38, 1) / Math.max(globalScale, 1e-6);
    baselineClipX.push((baselineX / canvasWidth) * 2 - 1);
    amplitudeScaleClip.push((amplitudeScalePx / canvasWidth) * 2);
  }

  return {
    traceIndices: Float32Array.from(traceIndices),
    baselineClipX: Float32Array.from(baselineClipX),
    amplitudeScaleClip: Float32Array.from(amplitudeScaleClip),
    sampleStart: viewport.sampleStart,
    sampleCount: Math.max(1, viewport.sampleEnd - viewport.sampleStart),
    traceStride
  };
}

export function buildOverlaySpatialIndex(
  section: SectionPayload,
  viewport: SectionViewport,
  renderMode: DisplayTransform["renderMode"],
  plotRect: PlotRect,
  horizons: Horizon[]
): OverlaySpatialIndex {
  const points: OverlaySpatialPoint[] = [];

  for (const horizon of horizons) {
    for (const anchor of horizon.anchors) {
      points.push({
        horizonId: horizon.id,
        anchorId: anchor.id,
        x: traceIndexToScreenX(section, viewport, renderMode, plotRect, anchor.traceIndex),
        y: sampleIndexToScreenY(viewport, plotRect, anchor.sampleIndex)
      });
    }
  }

  return { points };
}

function visibleAmplitudeScale(
  section: SectionPayload,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  gain: number
): number {
  let maxAbs = 0;
  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const amplitude = sectionAmplitudeAt(section, trace, sample);
      if (amplitude === null) {
        continue;
      }
      maxAbs = Math.max(maxAbs, Math.abs(amplitude * gain));
    }
  }
  return Math.max(maxAbs, 1e-6);
}

function buildTraceIndices(traceStart: number, traceEnd: number, traceStride: number): number[] {
  const indices: number[] = [];
  for (let trace = traceStart; trace < traceEnd; trace += traceStride) {
    indices.push(trace);
  }
  return indices;
}

function pushClipVertex(target: number[], x: number, y: number, width: number, height: number): void {
  target.push((x / width) * 2 - 1, 1 - (y / height) * 2);
}

function buildColorLut(colormap: ColorMapName): Uint8Array {
  const bytes = new Uint8Array(256 * 4);

  for (let index = 0; index < 256; index += 1) {
    const normalized = index / 255;
    const [red, green, blue] = colorFromMap(colormap, normalized);
    const offset = index * 4;
    bytes[offset] = red;
    bytes[offset + 1] = green;
    bytes[offset + 2] = blue;
    bytes[offset + 3] = 255;
  }

  return bytes;
}

function colorFromMap(colormap: ColorMapName, normalized: number): [number, number, number] {
  if (colormap === "red-white-blue") {
    return redWhiteBlueColor(normalized);
  }

  const pixel = Math.round(normalized * 255);
  return [pixel, pixel, pixel];
}

function redWhiteBlueColor(normalized: number): [number, number, number] {
  if (normalized <= 0.5) {
    const ratio = normalized / 0.5;
    return [
      Math.round(60 + ratio * 195),
      Math.round(90 + ratio * 165),
      255
    ];
  }

  const ratio = (normalized - 0.5) / 0.5;
  return [
    255,
    Math.round(255 - ratio * 185),
    Math.round(255 - ratio * 185)
  ];
}
