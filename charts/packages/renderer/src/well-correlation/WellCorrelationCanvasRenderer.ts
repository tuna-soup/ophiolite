import {
  buildDepthBinnedCurveLod,
  buildWellCorrelationLayoutCache,
  buildWellCorrelationHeaderRows,
  chooseWellCorrelationDepthStep,
  type CorrelationPanelLayout,
  formatWellCorrelationAxisValue,
  mapNativeDepthToPanelDepth,
  trackHitsForWell,
  type WellCorrelationLayoutCache,
  type NormalizedCurveLayer,
  type NormalizedPointLayer,
  type NormalizedReferenceTrack,
  type NormalizedScalarTrack,
  type NormalizedSeismicSectionLayer,
  type NormalizedSeismicSectionTrack,
  type NormalizedSeismicTraceLayer,
  type NormalizedSeismicTraceTrack,
  type NormalizedTopOverlayLayer,
  type NormalizedTrack
} from "@ophiolite/charts-core";
import type { InteractionState, WellCorrelationViewport } from "@ophiolite/charts-data-models";
import {
  applyCanvasSurfaceTransform,
  createRasterSurfaceMetrics,
  resizeCanvasBackingStore
} from "../internal/rasterSurface";
import type { WellCorrelationRenderFrame, WellCorrelationRendererAdapter } from "./adapter";
import { createRendererTelemetryEvent, type RendererTelemetryListener } from "../telemetry";
import {
  createBaseRenderState,
  createOverlayRenderState,
  diffRenderStates,
  type BaseRenderState,
  type OverlayRenderState
} from "./renderModel";

const SCROLLBAR_GUTTER = 18;
const TRACK_ROW_HEIGHT = 20;

interface WellCorrelationCanvasRendererOptions {
  axisChrome?: "canvas" | "none";
}

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export class WellCorrelationCanvasRenderer implements WellCorrelationRendererAdapter {
  private container: HTMLElement | null = null;
  private host: HTMLDivElement | null = null;
  private baseCanvas: HTMLCanvasElement | null = null;
  private overlayCanvas: HTMLCanvasElement | null = null;
  private baseContext: CanvasRenderingContext2D | null = null;
  private overlayContext: CanvasRenderingContext2D | null = null;
  private lastFrame: WellCorrelationRenderFrame | null = null;
  private lastBaseState: BaseRenderState | null = null;
  private lastOverlayState: OverlayRenderState | null = null;
  private lastLayoutCache: WellCorrelationLayoutCache | null = null;
  private readonly axisChrome: "canvas" | "none";
  private telemetryListener: RendererTelemetryListener | null = null;

  constructor(options: WellCorrelationCanvasRendererOptions = {}) {
    this.axisChrome = options.axisChrome ?? "canvas";
  }

  setTelemetryListener(listener: RendererTelemetryListener | null): void {
    this.telemetryListener = listener;
  }

  mount(container: HTMLElement): void {
    this.container = container;
    this.host = document.createElement("div");
    this.host.style.position = "relative";
    this.host.style.width = "100%";
    this.host.style.height = "100%";
    this.host.style.overflow = "hidden";

    this.baseCanvas = document.createElement("canvas");
    this.baseCanvas.style.position = "absolute";
    this.baseCanvas.style.inset = "0";
    this.baseCanvas.style.width = "100%";
    this.baseCanvas.style.height = "100%";

    this.overlayCanvas = document.createElement("canvas");
    this.overlayCanvas.style.position = "absolute";
    this.overlayCanvas.style.inset = "0";
    this.overlayCanvas.style.width = "100%";
    this.overlayCanvas.style.height = "100%";
    this.overlayCanvas.style.pointerEvents = "none";

    this.host.append(this.baseCanvas, this.overlayCanvas);
    container.replaceChildren(this.host);
    this.baseContext = this.baseCanvas.getContext("2d");
    this.overlayContext = this.overlayCanvas.getContext("2d");
    if (!this.baseContext || !this.overlayContext) {
      this.emitTelemetry({
        kind: "mount-failed",
        phase: "mount",
        backend: "canvas-2d",
        recoverable: false,
        message: "Well correlation renderer could not acquire required 2D canvas contexts."
      });
      throw new Error("WellCorrelationCanvasRenderer could not acquire required 2D canvas contexts.");
    }
    this.emitTelemetry({
      kind: "backend-selected",
      phase: "mount",
      backend: "canvas-2d",
      recoverable: true,
      message: "Well correlation renderer selected the canvas-2d backend."
    });
  }

  render(frame: WellCorrelationRenderFrame): void {
    const renderStart = typeof performance !== "undefined" ? performance.now() : Date.now();
    try {
      if (
        !this.host ||
        !this.baseCanvas ||
        !this.overlayCanvas ||
        !this.baseContext ||
        !this.overlayContext
      ) {
        return;
      }
      this.lastFrame = frame;
      const viewportWidth = Math.max(1, Math.round(resolveViewportWidth(this.host, this.container) || this.host.clientWidth || 1));
      const viewportHeight = Math.max(1, Math.round(this.host.clientHeight || 1));
      const baseState = createBaseRenderState(
        frame,
        viewportWidth,
        viewportHeight,
        window.devicePixelRatio || 1,
        0,
        viewportWidth
      );
      const overlayState = createOverlayRenderState(
        frame,
        viewportWidth,
        viewportHeight,
        window.devicePixelRatio || 1,
        0,
        viewportWidth
      );
      const invalidation = diffRenderStates(this.lastBaseState, baseState, this.lastOverlayState, overlayState);
      const { panel, viewport } = frame.state;
      if (!panel || !viewport) {
        clearCanvas(this.baseContext, this.baseCanvas);
        clearCanvas(this.overlayContext, this.overlayCanvas);
        this.lastBaseState = baseState;
        this.lastOverlayState = overlayState;
        this.lastLayoutCache = null;
        this.host.style.width = "100%";
        return;
      }

      if (invalidation.baseChanged || !this.lastLayoutCache) {
        this.lastLayoutCache = buildWellCorrelationLayoutCache(panel, viewportWidth, viewportHeight);
      }
      const layoutCache = this.lastLayoutCache;
      const layout = layoutCache.layout;
      const visibleRect = visibleContentRect(layout.contentWidth, viewportHeight);
      const surface = createRasterSurfaceMetrics(layout.contentWidth, viewportHeight);
      resizeCanvasBackingStore(this.baseCanvas, surface);
      resizeCanvasBackingStore(this.overlayCanvas, surface);
      this.host.style.width = `${layout.contentWidth}px`;
      this.baseCanvas.style.width = `${layout.contentWidth}px`;
      this.overlayCanvas.style.width = `${layout.contentWidth}px`;

      if (invalidation.baseChanged) {
        const context = this.baseContext;
        applyCanvasSurfaceTransform(context, surface);
        context.clearRect(visibleRect.x, visibleRect.y, visibleRect.width, visibleRect.height);
        context.fillStyle = panel.background ?? "#f8f5ef";
        context.fillRect(visibleRect.x, visibleRect.y, visibleRect.width, visibleRect.height);
        withHorizontalClip(context, visibleRect, () => {
          drawDepthGrid(context, layout.plotRect, viewport);
        });

        for (const column of layout.columns) {
          if (!rectIntersectsHorizontally(column.bodyRect, visibleRect)) {
            continue;
          }
          const wellHits = trackHitsForWell(layoutCache, column.wellId);
          if (wellHits.length === 0) {
            continue;
          }
          const well = wellHits[0]!.well;
          if (this.axisChrome === "canvas") {
            drawWellHeader(context, column.headerRect, well.name);
          }
          for (const hit of wellHits) {
            if (!rectIntersectsHorizontally(hit.trackFrame.bodyRect, visibleRect)) {
              continue;
            }
            if (this.axisChrome === "canvas") {
              drawTrackHeader(context, hit.trackFrame.headerRect, hit.track, well.nativeDepthDatum);
            }
            drawTrackBodyFrame(context, hit.trackFrame.bodyRect);
            drawTrack(context, hit.track, well.panelDepthMapping, viewport, hit.trackFrame.bodyRect, this.axisChrome);
          }
        }
      }

      if (invalidation.overlayNeedsDraw) {
        const context = this.overlayContext;
        applyCanvasSurfaceTransform(context, surface);
        context.clearRect(visibleRect.x, visibleRect.y, visibleRect.width, visibleRect.height);
        withHorizontalClip(context, visibleRect, () => {
          drawCorrelationLines(context, layout, overlayState.previewCorrelationLines ?? overlayState.correlationLines, viewport);
          if (overlayState.probe && overlayState.interactions.modifiers.includes("crosshair")) {
            drawProbeGuides(context, layout.plotRect, overlayState.probe.screenX, overlayState.probe.screenY);
          }
          if (overlayState.interactions.session?.kind === "lasso") {
            drawLassoOverlay(context, overlayState.interactions.session.points);
          }
        });
      }

      this.host.style.cursor = cursorForInteractionState(overlayState.interactions);
      this.lastBaseState = baseState;
      this.lastOverlayState = overlayState;
      this.host.dispatchEvent(new CustomEvent("ophiolite-charts:correlation-render-debug", {
        bubbles: true,
        detail: {
          renderMs: (typeof performance !== "undefined" ? performance.now() : Date.now()) - renderStart,
          baseChanged: invalidation.baseChanged,
          overlayDraw: invalidation.overlayNeedsDraw,
          contentWidth: layout.contentWidth,
          viewportWidth: layout.viewportWidth
        }
      }));
    } catch (error) {
      this.emitTelemetry({
        kind: "frame-failed",
        phase: "render",
        backend: "canvas-2d",
        recoverable: true,
        message: error instanceof Error ? error.message : String(error),
        detail: "Well correlation renderer failed while drawing a frame."
      });
      throw error;
    }
  }

  dispose(): void {
    this.host?.remove();
    this.container = null;
    this.host = null;
    this.baseCanvas = null;
    this.overlayCanvas = null;
    this.baseContext = null;
    this.overlayContext = null;
    this.lastFrame = null;
    this.lastBaseState = null;
    this.lastOverlayState = null;
    this.lastLayoutCache = null;
  }

  private emitTelemetry(event: Parameters<typeof createRendererTelemetryEvent>[0]): void {
    this.telemetryListener?.(createRendererTelemetryEvent(event));
  }
}

function clearCanvas(context: CanvasRenderingContext2D, canvas: HTMLCanvasElement): void {
  context.setTransform(1, 0, 0, 1, 0, 0);
  context.clearRect(0, 0, canvas.width, canvas.height);
}

function resolveViewportWidth(host: HTMLDivElement, container: HTMLElement | null): number {
  const scrollViewport = host.closest<HTMLElement>(".ophiolite-charts-correlation-scroll-viewport");
  if (scrollViewport) {
    return Math.max(1, scrollViewport.clientWidth + SCROLLBAR_GUTTER);
  }
  return Math.max(1, container?.clientWidth ?? host.clientWidth);
}

function visibleContentRect(contentWidth: number, viewportHeight: number): Rect {
  return {
    x: 0,
    y: 0,
    width: Math.max(1, contentWidth),
    height: viewportHeight
  };
}

function rectIntersectsHorizontally(rect: Rect, visibleRect: Rect): boolean {
  return rect.x + rect.width >= visibleRect.x && rect.x <= visibleRect.x + visibleRect.width;
}

function withHorizontalClip(
  context: CanvasRenderingContext2D,
  visibleRect: Rect,
  draw: () => void
): void {
  context.save();
  context.beginPath();
  context.rect(visibleRect.x, visibleRect.y, visibleRect.width, visibleRect.height);
  context.clip();
  draw();
  context.restore();
}

function drawWellHeader(context: CanvasRenderingContext2D, rect: Rect, title: string): void {
  context.fillStyle = "#ffffff";
  context.fillRect(rect.x, rect.y, rect.width, rect.height);
  context.strokeStyle = "#b8b8b8";
  context.strokeRect(rect.x, rect.y, rect.width, rect.height);
  context.fillStyle = "#1a1a1a";
  context.font = "600 12px Segoe UI";
  context.textAlign = "center";
  context.fillText(title, rect.x + rect.width / 2, rect.y + 16);
}

function drawTrackHeader(context: CanvasRenderingContext2D, rect: Rect, track: NormalizedTrack, nativeDepthDatum: string): void {
  context.fillStyle = "#f3f1ed";
  context.fillRect(rect.x, rect.y, rect.width, rect.height);
  context.strokeStyle = "#c8c1b8";
  context.strokeRect(rect.x, rect.y, rect.width, rect.height);
  buildWellCorrelationHeaderRows(track, nativeDepthDatum).forEach((row, index) => {
    const y = rect.y + 4 + index * TRACK_ROW_HEIGHT;
    context.fillStyle = row.color;
    context.font = "600 10px Segoe UI";
    context.textAlign = "center";
    context.fillText(row.label, rect.x + rect.width / 2, y + 8);
    if (!row.axis) {
      return;
    }
    context.fillStyle = "#6b6b6b";
    context.font = "10px Segoe UI";
    context.textAlign = "left";
    context.fillText(formatWellCorrelationAxisValue(row.axis.min), rect.x + 4, y + 17);
    context.textAlign = "right";
    context.fillText(formatWellCorrelationAxisValue(row.axis.max), rect.x + rect.width - 4, y + 17);
  });
}

function drawTrackBodyFrame(context: CanvasRenderingContext2D, rect: Rect): void {
  context.fillStyle = "#fbfbfb";
  context.fillRect(rect.x, rect.y, rect.width, rect.height);
  context.strokeStyle = "#a9a9a9";
  context.strokeRect(rect.x, rect.y, rect.width, rect.height);
}

function drawTrack(
  context: CanvasRenderingContext2D,
  track: NormalizedTrack,
  mapping: Array<{ nativeDepth: number; panelDepth: number }>,
  viewport: WellCorrelationViewport,
  rect: Rect,
  axisChrome: "canvas" | "none"
): void {
  if (track.kind === "reference") {
    drawReferenceTrack(context, track, rect, viewport, axisChrome);
    return;
  }
  if (track.kind === "scalar") {
    drawScalarTrack(context, track, mapping, viewport, rect);
    return;
  }
  if (track.kind === "seismic-trace") {
    drawSeismicTraceTrack(context, track, viewport, rect);
    return;
  }
  drawSeismicSectionTrack(context, track, viewport, rect);
}

function drawReferenceTrack(
  context: CanvasRenderingContext2D,
  track: NormalizedReferenceTrack,
  rect: Rect,
  viewport: WellCorrelationViewport,
  axisChrome: "canvas" | "none"
): void {
  if (axisChrome === "canvas") {
    const step = chooseWellCorrelationDepthStep(viewport.depthEnd - viewport.depthStart);
    const firstTick = Math.ceil(viewport.depthStart / step) * step;
    context.strokeStyle = "#767676";
    context.fillStyle = "#404040";
    context.font = "11px Segoe UI";
    context.textAlign = "right";
    for (let depth = firstTick; depth <= viewport.depthEnd; depth += step) {
      const y = depthToScreenY(rect, viewport, depth);
      context.beginPath();
      context.moveTo(rect.x + rect.width - 12, y);
      context.lineTo(rect.x + rect.width, y);
      context.stroke();
      context.fillText(depth.toFixed(0), rect.x + rect.width - 16, y + 3);
    }
  }
  drawTopOverlays(context, track.topOverlays, rect, viewport);
}

function drawScalarTrack(
  context: CanvasRenderingContext2D,
  track: NormalizedScalarTrack,
  mapping: Array<{ nativeDepth: number; panelDepth: number }>,
  viewport: WellCorrelationViewport,
  rect: Rect
): void {
  drawTrackGrid(context, rect, track.xAxis);
  drawBetweenCurveFills(context, track, mapping, viewport, rect);
  for (const layer of track.layers) {
    if (layer.kind === "curve") {
      drawCurveLayer(context, layer, mapping, viewport, rect, track.xAxis);
    } else if (layer.kind === "point-observation") {
      drawPointLayer(context, layer, mapping, viewport, rect);
    } else if (layer.kind === "composition") {
      drawCompositionLayer(context, layer, mapping, viewport, rect);
    }
  }
  drawTopOverlays(context, track.layers.filter((layer): layer is NormalizedTopOverlayLayer => layer.kind === "top-overlay"), rect, viewport);
}

function drawSeismicTraceTrack(context: CanvasRenderingContext2D, track: NormalizedSeismicTraceTrack, viewport: WellCorrelationViewport, rect: Rect): void {
  for (const layer of track.layers) {
    if (layer.kind === "seismic-trace") {
      drawTraceLayer(context, layer, viewport, rect);
    }
  }
  drawTopOverlays(context, track.layers.filter((layer): layer is NormalizedTopOverlayLayer => layer.kind === "top-overlay"), rect, viewport);
}

function drawSeismicSectionTrack(context: CanvasRenderingContext2D, track: NormalizedSeismicSectionTrack, viewport: WellCorrelationViewport, rect: Rect): void {
  for (const layer of track.layers) {
    if (layer.kind !== "seismic-section") {
      continue;
    }
    if (layer.style.renderMode === "wiggle") {
      drawSectionWiggles(context, layer, viewport, rect);
    } else {
      drawSectionHeatmap(context, layer, viewport, rect);
    }
  }
  drawTopOverlays(context, track.layers.filter((layer): layer is NormalizedTopOverlayLayer => layer.kind === "top-overlay"), rect, viewport);
}

function drawTrackGrid(context: CanvasRenderingContext2D, rect: Rect, axis: { min: number; max: number; scale?: "linear" | "log"; tickCount?: number }): void {
  context.strokeStyle = "rgba(95, 95, 95, 0.3)";
  const ticks = axis.scale === "log" ? buildLogTicks(axis.min, axis.max) : buildLinearTicks(axis.min, axis.max, axis.tickCount ?? 4);
  for (const tick of ticks) {
    const x = valueToTrackX(tick, axis, rect);
    context.beginPath();
    context.moveTo(x, rect.y);
    context.lineTo(x, rect.y + rect.height);
    context.stroke();
  }
}

function drawCurveLayer(
  context: CanvasRenderingContext2D,
  layer: NormalizedCurveLayer,
  mapping: Array<{ nativeDepth: number; panelDepth: number }>,
  viewport: WellCorrelationViewport,
  rect: Rect,
  defaultAxis: { min: number; max: number; scale?: "linear" | "log"; tickCount?: number }
): void {
  const axis = layer.series.axis ?? defaultAxis;
  const lod = buildDepthBinnedCurveLod(layer.series, mapping, viewport.depthStart, viewport.depthEnd, Math.round(rect.height));
  if (layer.series.fill) {
    const baselineX = valueToTrackX(layer.series.fill.baseline, axis, rect);
    context.beginPath();
    let started = false;
    for (const point of lod) {
      const x = valueToTrackX(point.maxValue, axis, rect);
      const y = depthToScreenY(rect, viewport, point.depth);
      if (!started) {
        context.moveTo(baselineX, y);
        context.lineTo(x, y);
        started = true;
      } else {
        context.lineTo(x, y);
      }
    }
    for (let index = lod.length - 1; index >= 0; index -= 1) {
      context.lineTo(baselineX, depthToScreenY(rect, viewport, lod[index]!.depth));
    }
    context.closePath();
    context.fillStyle = layer.series.fill.color;
    context.fill();
  }
  context.strokeStyle = layer.series.color;
  context.lineWidth = layer.series.lineWidth ?? 1.2;
  context.beginPath();
  let started = false;
  for (const point of lod) {
    const x = valueToTrackX(point.maxValue, axis, rect);
    const y = depthToScreenY(rect, viewport, point.depth);
    if (!started) {
      context.moveTo(x, y);
      started = true;
    } else {
      context.lineTo(x, y);
    }
  }
  context.stroke();
}

function drawBetweenCurveFills(
  context: CanvasRenderingContext2D,
  track: NormalizedScalarTrack,
  mapping: Array<{ nativeDepth: number; panelDepth: number }>,
  viewport: WellCorrelationViewport,
  rect: Rect
): void {
  for (const fill of track.betweenCurveFills ?? []) {
    const left = track.layers.find((layer): layer is NormalizedCurveLayer => layer.kind === "curve" && layer.series.id === fill.leftSeriesId);
    const right = track.layers.find((layer): layer is NormalizedCurveLayer => layer.kind === "curve" && layer.series.id === fill.rightSeriesId);
    if (!left || !right) {
      continue;
    }
    const count = Math.min(left.series.values.length, right.series.values.length);
    context.fillStyle = fill.color;
    for (let index = 1; index < count; index += 1) {
      const depth = mapNativeDepthToPanelDepth(mapping, left.series.nativeDepths[index]!);
      const previousDepth = mapNativeDepthToPanelDepth(mapping, left.series.nativeDepths[index - 1]!);
      if (depth < viewport.depthStart || previousDepth > viewport.depthEnd) {
        continue;
      }
      const y1 = depthToScreenY(rect, viewport, previousDepth);
      const y2 = depthToScreenY(rect, viewport, depth);
      const lx1 = valueToTrackX(left.series.values[index - 1]!, left.series.axis ?? track.xAxis, rect);
      const lx2 = valueToTrackX(left.series.values[index]!, left.series.axis ?? track.xAxis, rect);
      const rx1 = valueToTrackX(right.series.values[index - 1]!, right.series.axis ?? track.xAxis, rect);
      const rx2 = valueToTrackX(right.series.values[index]!, right.series.axis ?? track.xAxis, rect);
      context.beginPath();
      context.moveTo(lx1, y1);
      context.lineTo(lx2, y2);
      context.lineTo(rx2, y2);
      context.lineTo(rx1, y1);
      context.closePath();
      context.fill();
    }
  }
}

function drawCompositionLayer(
  context: CanvasRenderingContext2D,
  layer: Extract<NormalizedScalarTrack["layers"][number], { kind: "composition" }>,
  mapping: Array<{ nativeDepth: number; panelDepth: number }>,
  viewport: WellCorrelationViewport,
  rect: Rect
): void {
  for (let index = 1; index < layer.nativeDepths.length; index += 1) {
    const depth = mapNativeDepthToPanelDepth(mapping, layer.nativeDepths[index]!);
    const previousDepth = mapNativeDepthToPanelDepth(mapping, layer.nativeDepths[index - 1]!);
    if (depth < viewport.depthStart || previousDepth > viewport.depthEnd) {
      continue;
    }
    const y1 = depthToScreenY(rect, viewport, previousDepth);
    const y2 = depthToScreenY(rect, viewport, depth);
    let cumulative = rect.x;
    for (const component of layer.components) {
      const width = rect.width * (component.values[index]! / 100);
      context.fillStyle = component.color;
      context.fillRect(cumulative, y1, width, Math.max(1, y2 - y1));
      cumulative += width;
    }
  }
}

function drawPointLayer(context: CanvasRenderingContext2D, layer: NormalizedPointLayer, mapping: Array<{ nativeDepth: number; panelDepth: number }>, viewport: WellCorrelationViewport, rect: Rect): void {
  context.fillStyle = layer.style.fillColor;
  context.strokeStyle = layer.style.strokeColor ?? layer.style.fillColor;
  context.lineWidth = layer.style.strokeWidth ?? 1;
  for (const point of layer.points) {
    const panelDepth = mapNativeDepthToPanelDepth(mapping, point.nativeDepth);
    if (panelDepth < viewport.depthStart || panelDepth > viewport.depthEnd) {
      continue;
    }
    drawSymbol(context, valueToTrackX(point.value, layer.axis, rect), depthToScreenY(rect, viewport, panelDepth), layer.style.shape, layer.style.size);
  }
}

function drawTraceLayer(context: CanvasRenderingContext2D, layer: NormalizedSeismicTraceLayer, viewport: WellCorrelationViewport, rect: Rect): void {
  const depths = layer.panelDepths ?? layer.nativeDepths;
  const spacing = rect.width / Math.max(1, layer.traces.length);
  const sharedExtent = layer.normalization === "shared-domain" ? maxAmplitude(layer.traces.flatMap((trace) => Array.from(trace.amplitudes))) : 1;
  layer.traces.forEach((trace, traceIndex) => {
    const centerX = rect.x + spacing * (traceIndex + 0.5);
    const extent = layer.normalization === "per-trace" ? maxAmplitude(Array.from(trace.amplitudes)) : sharedExtent;
    drawSingleWiggle(context, trace.amplitudes, depths, viewport, rect, centerX, spacing * 0.42, extent, trace.style);
  });
}

function drawSectionHeatmap(context: CanvasRenderingContext2D, layer: NormalizedSeismicSectionLayer, viewport: WellCorrelationViewport, rect: Rect): void {
  const { traces, samples } = layer.section.dimensions;
  const traceWidth = rect.width / Math.max(1, traces);
  for (let sampleIndex = 1; sampleIndex < samples; sampleIndex += 1) {
    const panelDepth = layer.panelDepths[sampleIndex]!;
    const previousDepth = layer.panelDepths[sampleIndex - 1]!;
    if (panelDepth < viewport.depthStart || previousDepth > viewport.depthEnd) {
      continue;
    }
    const y1 = depthToScreenY(rect, viewport, previousDepth);
    const y2 = depthToScreenY(rect, viewport, panelDepth);
    for (let traceIndex = 0; traceIndex < traces; traceIndex += 1) {
      context.fillStyle = colorForAmplitude(layer.section.amplitudes[traceIndex * samples + sampleIndex] ?? 0, layer.style.colormap ?? "grayscale");
      context.fillRect(rect.x + traceIndex * traceWidth, y1, traceWidth + 1, Math.max(1, y2 - y1));
    }
  }
}

function drawSectionWiggles(context: CanvasRenderingContext2D, layer: NormalizedSeismicSectionLayer, viewport: WellCorrelationViewport, rect: Rect): void {
  const { traces, samples } = layer.section.dimensions;
  const spacing = rect.width / Math.max(1, traces);
  const extent = maxAmplitude(Array.from(layer.section.amplitudes));
  for (let traceIndex = 0; traceIndex < traces; traceIndex += 1) {
    const amplitudes = new Float32Array(samples);
    for (let sampleIndex = 0; sampleIndex < samples; sampleIndex += 1) {
      amplitudes[sampleIndex] = layer.section.amplitudes[traceIndex * samples + sampleIndex] ?? 0;
    }
    drawSingleWiggle(context, amplitudes, layer.panelDepths, viewport, rect, rect.x + spacing * (traceIndex + 0.5), spacing * 0.42, extent, {
      positiveFill: "#d32323",
      negativeFill: "#111111",
      lineColor: "#111111",
      lineWidth: 0.8,
      fillOpacity: 0.55
    });
  }
}

function drawSingleWiggle(
  context: CanvasRenderingContext2D,
  amplitudes: Float32Array,
  depths: Float32Array,
  viewport: WellCorrelationViewport,
  rect: Rect,
  centerX: number,
  halfWidth: number,
  extent: number,
  style: { positiveFill: string; negativeFill?: string; lineColor?: string; lineWidth?: number; fillOpacity?: number }
): void {
  if (extent <= 0 || amplitudes.length === 0 || depths.length === 0) {
    return;
  }
  const points: Array<{ x: number; y: number }> = [];
  for (let index = 0; index < amplitudes.length; index += 1) {
    const depth = depths[index]!;
    if (depth < viewport.depthStart || depth > viewport.depthEnd) {
      continue;
    }
    points.push({
      x: centerX + (amplitudes[index]! / extent) * halfWidth,
      y: depthToScreenY(rect, viewport, depth)
    });
  }
  if (points.length < 2) {
    return;
  }
  context.globalAlpha = style.fillOpacity ?? 0.7;
  context.fillStyle = style.positiveFill;
  context.beginPath();
  context.moveTo(centerX, points[0]!.y);
  for (const point of points) {
    context.lineTo(point.x, point.y);
  }
  context.lineTo(centerX, points[points.length - 1]!.y);
  context.closePath();
  context.fill();
  context.globalAlpha = 1;
  context.strokeStyle = style.lineColor ?? "#111111";
  context.lineWidth = style.lineWidth ?? 1;
  context.beginPath();
  context.moveTo(points[0]!.x, points[0]!.y);
  for (const point of points.slice(1)) {
    context.lineTo(point.x, point.y);
  }
  context.stroke();
}

function drawTopOverlays(context: CanvasRenderingContext2D, overlays: NormalizedTopOverlayLayer[], rect: Rect, viewport: WellCorrelationViewport): void {
  for (const overlay of overlays) {
    for (const top of overlay.tops) {
      const y = depthToScreenY(rect, viewport, top.nativeDepth);
      if (y < rect.y || y > rect.y + rect.height) {
        continue;
      }
      context.strokeStyle = overlay.style.color || top.color;
      context.lineWidth = overlay.style.lineWidth ?? 1;
      context.beginPath();
      context.moveTo(rect.x, y);
      context.lineTo(rect.x + rect.width, y);
      context.stroke();
      if (overlay.style.showLabels) {
        context.fillStyle = overlay.style.labelColor ?? overlay.style.color ?? top.color;
        context.font = "10px Segoe UI";
        context.textAlign = "left";
        context.fillText(top.name, rect.x + 4, y - 3);
      }
    }
  }
}

function drawCorrelationLines(
  context: CanvasRenderingContext2D,
  layout: CorrelationPanelLayout,
  lines: WellCorrelationRenderFrame["state"]["correlationLines"],
  viewport: WellCorrelationViewport
): void {
  for (const line of lines) {
    const points = line.points.map((point) => {
      const column = layout.columns.find((candidate) => candidate.wellId === point.wellId);
      if (!column) {
        return null;
      }
      return { x: column.bodyRect.x + column.bodyRect.width / 2, y: depthToScreenY(column.bodyRect, viewport, point.panelDepth) };
    }).filter((point): point is { x: number; y: number } => Boolean(point));
    if (points.length < 2) {
      continue;
    }
    context.strokeStyle = line.color;
    context.lineWidth = 1.2;
    context.beginPath();
    context.moveTo(points[0]!.x, points[0]!.y);
    for (const point of points.slice(1)) {
      context.lineTo(point.x, point.y);
    }
    context.stroke();
  }
}

function drawProbeGuides(context: CanvasRenderingContext2D, rect: Rect, x: number, y: number): void {
  context.strokeStyle = "rgba(40, 40, 40, 0.38)";
  context.setLineDash([4, 4]);
  context.beginPath();
  context.moveTo(rect.x, y);
  context.lineTo(rect.x + rect.width, y);
  context.stroke();
  context.beginPath();
  context.moveTo(x, rect.y);
  context.lineTo(x, rect.y + rect.height);
  context.stroke();
  context.setLineDash([]);
}

function drawLassoOverlay(context: CanvasRenderingContext2D, points: Array<{ x: number; y: number }>): void {
  if (points.length < 2) {
    return;
  }
  context.strokeStyle = "rgba(37, 96, 187, 0.82)";
  context.fillStyle = "rgba(37, 96, 187, 0.14)";
  context.beginPath();
  context.moveTo(points[0]!.x, points[0]!.y);
  for (const point of points.slice(1)) {
    context.lineTo(point.x, point.y);
  }
  context.closePath();
  context.fill();
  context.stroke();
}

function drawDepthGrid(context: CanvasRenderingContext2D, plotRect: Rect, viewport: WellCorrelationViewport): void {
  const majorStep = chooseWellCorrelationDepthStep(viewport.depthEnd - viewport.depthStart);
  const firstTick = Math.ceil(viewport.depthStart / majorStep) * majorStep;
  context.strokeStyle = "rgba(130, 130, 130, 0.35)";
  for (let depth = firstTick; depth <= viewport.depthEnd; depth += majorStep) {
    const y = depthToScreenY(plotRect, viewport, depth);
    context.beginPath();
    context.moveTo(plotRect.x, y);
    context.lineTo(plotRect.x + plotRect.width, y);
    context.stroke();
  }
}

function drawSymbol(context: CanvasRenderingContext2D, x: number, y: number, shape: "circle" | "square" | "diamond" | "triangle" | "cross" | "x", size: number): void {
  const half = size / 2;
  context.beginPath();
  switch (shape) {
    case "square": context.rect(x - half, y - half, size, size); break;
    case "diamond": context.moveTo(x, y - half); context.lineTo(x + half, y); context.lineTo(x, y + half); context.lineTo(x - half, y); context.closePath(); break;
    case "triangle": context.moveTo(x, y - half); context.lineTo(x + half, y + half); context.lineTo(x - half, y + half); context.closePath(); break;
    case "cross": context.moveTo(x - half, y); context.lineTo(x + half, y); context.moveTo(x, y - half); context.lineTo(x, y + half); context.stroke(); return;
    case "x": context.moveTo(x - half, y - half); context.lineTo(x + half, y + half); context.moveTo(x + half, y - half); context.lineTo(x - half, y + half); context.stroke(); return;
    default: context.arc(x, y, half, 0, Math.PI * 2);
  }
  context.fill();
  context.stroke();
}

function valueToTrackX(value: number, axis: { min: number; max: number; scale?: "linear" | "log" }, rect: Rect): number {
  const ratio = axis.scale === "log"
    ? (Math.log10(Math.max(value, 1e-6)) - Math.log10(Math.max(axis.min, 1e-6))) /
      (Math.log10(Math.max(axis.max, axis.min * 1.0001)) - Math.log10(Math.max(axis.min, 1e-6)))
    : axis.max === axis.min ? 0.5 : (value - axis.min) / (axis.max - axis.min);
  return rect.x + clamp(ratio, 0, 1) * rect.width;
}

function depthToScreenY(rect: Rect, viewport: WellCorrelationViewport, depth: number): number {
  return rect.y + ((depth - viewport.depthStart) / Math.max(1e-6, viewport.depthEnd - viewport.depthStart)) * rect.height;
}

function buildLinearTicks(min: number, max: number, tickCount: number): number[] {
  const count = Math.max(2, tickCount);
  return Array.from({ length: count }, (_, index) => min + ((max - min) * index) / (count - 1));
}

function buildLogTicks(min: number, max: number): number[] {
  const safeMin = Math.max(min, 1e-6);
  const safeMax = Math.max(max, safeMin * 1.01);
  const ticks: number[] = [];
  for (let decade = Math.floor(Math.log10(safeMin)); decade <= Math.ceil(Math.log10(safeMax)); decade += 1) {
    for (const mantissa of [1, 2, 5]) {
      const value = mantissa * 10 ** decade;
      if (value >= safeMin && value <= safeMax) {
        ticks.push(value);
      }
    }
  }
  return ticks;
}

function maxAmplitude(values: number[]): number {
  let max = 0;
  for (const value of values) {
    max = Math.max(max, Math.abs(value));
  }
  return max || 1;
}

function colorForAmplitude(amplitude: number, colormap: string): string {
  const scaled = clamp(amplitude / 2, -1, 1);
  if (colormap === "red-white-blue") {
    if (scaled >= 0) {
      const channel = Math.round(255 * (1 - scaled));
      return `rgb(255,${channel},${channel})`;
    }
    const channel = Math.round(255 * (1 + scaled));
    return `rgb(${channel},${channel},255)`;
  }
  const gray = Math.round(((scaled + 1) / 2) * 255);
  return `rgb(${gray},${gray},${gray})`;
}

function cursorForInteractionState(interactions: InteractionState): string {
  if (interactions.primaryMode === "panZoom") return "grab";
  if (interactions.primaryMode === "topEdit") return "ns-resize";
  if (interactions.primaryMode === "lassoSelect") return "crosshair";
  return "default";
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
