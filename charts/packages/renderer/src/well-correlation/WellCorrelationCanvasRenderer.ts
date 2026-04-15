import {
  buildDepthBinnedCurveLod,
  layoutWellCorrelationPanel,
  mapNativeDepthToPanelDepth,
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
import type { WellCorrelationRenderFrame, WellCorrelationRendererAdapter } from "./adapter";

const SCROLLBAR_WIDTH = 14;
const SCROLLBAR_GUTTER = 18;
const TRACK_ROW_HEIGHT = 20;

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export class WellCorrelationCanvasRenderer implements WellCorrelationRendererAdapter {
  private host: HTMLDivElement | null = null;
  private scrollHost: HTMLDivElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private context: CanvasRenderingContext2D | null = null;
  private scrollbarTrack: HTMLDivElement | null = null;
  private scrollbarThumb: HTMLDivElement | null = null;
  private lastFrame: WellCorrelationRenderFrame | null = null;
  private dragOffsetY = 0;
  private draggingScrollbar = false;

  mount(container: HTMLElement): void {
    this.host = document.createElement("div");
    this.host.style.position = "relative";
    this.host.style.width = "100%";
    this.host.style.height = "100%";
    this.host.style.overflow = "hidden";

    this.scrollHost = document.createElement("div");
    this.scrollHost.style.position = "absolute";
    this.scrollHost.style.inset = "0 18px 0 0";
    this.scrollHost.style.overflowX = "auto";
    this.scrollHost.style.overflowY = "hidden";
    this.scrollHost.style.scrollbarWidth = "thin";

    this.canvas = document.createElement("canvas");
    this.canvas.style.display = "block";
    this.canvas.style.height = "100%";
    this.scrollHost.append(this.canvas);

    this.scrollbarTrack = document.createElement("div");
    this.scrollbarTrack.style.position = "absolute";
    this.scrollbarTrack.style.right = "2px";
    this.scrollbarTrack.style.width = `${SCROLLBAR_WIDTH}px`;
    this.scrollbarTrack.style.borderRadius = "999px";
    this.scrollbarTrack.style.background = "rgba(122, 122, 122, 0.22)";
    this.scrollbarTrack.style.cursor = "pointer";

    this.scrollbarThumb = document.createElement("div");
    this.scrollbarThumb.style.position = "absolute";
    this.scrollbarThumb.style.left = "1px";
    this.scrollbarThumb.style.width = `${SCROLLBAR_WIDTH - 2}px`;
    this.scrollbarThumb.style.borderRadius = "999px";
    this.scrollbarThumb.style.background = "rgba(37, 96, 187, 0.78)";
    this.scrollbarThumb.style.cursor = "grab";
    this.scrollbarTrack.append(this.scrollbarThumb);

    this.host.append(this.scrollHost, this.scrollbarTrack);
    container.replaceChildren(this.host);
    this.context = this.canvas.getContext("2d");

    this.scrollbarTrack.addEventListener("pointerdown", (event) => {
      if (event.target === this.scrollbarThumb) {
        return;
      }
      this.jumpScrollbar(event.clientY);
    });
    this.scrollbarThumb.addEventListener("pointerdown", (event) => {
      this.draggingScrollbar = true;
      this.dragOffsetY = event.offsetY;
      this.scrollbarThumb?.setPointerCapture(event.pointerId);
      this.scrollbarThumb!.style.cursor = "grabbing";
      event.preventDefault();
    });
    this.scrollbarThumb.addEventListener("pointermove", (event) => {
      if (!this.draggingScrollbar) {
        return;
      }
      this.dragScrollbar(event.clientY);
      event.preventDefault();
    });
    this.scrollbarThumb.addEventListener("pointerup", (event) => {
      this.draggingScrollbar = false;
      this.scrollbarThumb?.releasePointerCapture(event.pointerId);
      if (this.scrollbarThumb) {
        this.scrollbarThumb.style.cursor = "grab";
      }
    });
  }

  render(frame: WellCorrelationRenderFrame): void {
    if (!this.host || !this.scrollHost || !this.canvas || !this.context) {
      return;
    }
    this.lastFrame = frame;
    const viewportWidth = Math.max(1, Math.round(this.host.clientWidth || 1));
    const viewportHeight = Math.max(1, Math.round(this.host.clientHeight || 1));
    const { panel, viewport, probe, correlationLines, previewCorrelationLines, interactions } = frame.state;
    if (!panel || !viewport) {
      this.context.clearRect(0, 0, this.canvas.width, this.canvas.height);
      return;
    }

    const layout = layoutWellCorrelationPanel(panel, viewportWidth, viewportHeight);
    if (this.canvas.width !== layout.contentWidth || this.canvas.height !== viewportHeight) {
      this.canvas.width = layout.contentWidth;
      this.canvas.height = viewportHeight;
      this.canvas.style.width = `${layout.contentWidth}px`;
    }
    this.scrollHost.style.width = `${layout.viewportWidth}px`;
    if (this.scrollbarTrack) {
      this.scrollbarTrack.style.top = `${layout.scrollbarRect.y}px`;
      this.scrollbarTrack.style.height = `${layout.scrollbarRect.height}px`;
    }

    const context = this.context;
    context.clearRect(0, 0, layout.contentWidth, viewportHeight);
    context.fillStyle = panel.background ?? "#f8f5ef";
    context.fillRect(0, 0, layout.contentWidth, viewportHeight);
    this.host.style.cursor = cursorForInteractionState(interactions);
    drawDepthGrid(context, layout.plotRect, viewport);

    for (const column of layout.columns) {
      const well = panel.wells.find((candidate) => candidate.id === column.wellId);
      if (!well) {
        continue;
      }
      drawWellHeader(context, column.headerRect, well.name);
      for (const trackFrame of column.trackFrames) {
        const track = well.tracks.find((candidate) => candidate.id === trackFrame.trackId);
        if (!track) {
          continue;
        }
        drawTrackHeader(context, trackFrame.headerRect, track, well.nativeDepthDatum);
        drawTrackBodyFrame(context, trackFrame.bodyRect);
        drawTrack(context, track, well.panelDepthMapping, viewport, trackFrame.bodyRect);
      }
    }

    drawCorrelationLines(context, layout, previewCorrelationLines ?? correlationLines, viewport);
    if (probe && interactions.modifiers.includes("crosshair")) {
      drawProbeGuides(context, layout.plotRect, probe.screenX, probe.screenY);
    }
    if (interactions.session?.kind === "lasso") {
      drawLassoOverlay(context, interactions.session.points);
    }
    updateScrollbarThumb(this.scrollbarThumb, panel.depthDomain.start, panel.depthDomain.end, viewport, layout.scrollbarRect);
  }

  dispose(): void {
    this.host?.remove();
    this.host = null;
    this.scrollHost = null;
    this.canvas = null;
    this.context = null;
    this.scrollbarTrack = null;
    this.scrollbarThumb = null;
    this.lastFrame = null;
  }

  private jumpScrollbar(clientY: number): void {
    if (!this.scrollbarTrack || !this.scrollbarThumb) {
      return;
    }
    const trackRect = this.scrollbarTrack.getBoundingClientRect();
    const thumbRect = this.scrollbarThumb.getBoundingClientRect();
    this.requestViewportFromThumb(clientY - trackRect.top - thumbRect.height / 2, trackRect.height, thumbRect.height);
  }

  private dragScrollbar(clientY: number): void {
    if (!this.scrollbarTrack || !this.scrollbarThumb) {
      return;
    }
    const trackRect = this.scrollbarTrack.getBoundingClientRect();
    const thumbRect = this.scrollbarThumb.getBoundingClientRect();
    this.requestViewportFromThumb(clientY - trackRect.top - this.dragOffsetY, trackRect.height, thumbRect.height);
  }

  private requestViewportFromThumb(rawTop: number, trackHeight: number, thumbHeight: number): void {
    if (!this.host || !this.lastFrame?.state.panel || !this.lastFrame.state.viewport) {
      return;
    }
    const { panel, viewport } = this.lastFrame.state;
    const fullStart = panel.depthDomain.start;
    const fullEnd = panel.depthDomain.end;
    const fullSpan = fullEnd - fullStart;
    const viewportSpan = viewport.depthEnd - viewport.depthStart;
    if (fullSpan <= viewportSpan) {
      return;
    }
    const available = Math.max(1, trackHeight - thumbHeight);
    const top = clamp(rawTop, 0, available);
    const ratio = top / available;
    const depthStart = fullStart + ratio * (fullSpan - viewportSpan);
    this.host.dispatchEvent(new CustomEvent("ophiolite-charts:correlation-viewport-request", {
      bubbles: true,
      detail: { depthStart, depthEnd: depthStart + viewportSpan }
    }));
  }
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
  headerRowsForTrack(track, nativeDepthDatum).forEach((row, index) => {
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
    context.fillText(formatAxisValue(row.axis.min), rect.x + 4, y + 17);
    context.textAlign = "right";
    context.fillText(formatAxisValue(row.axis.max), rect.x + rect.width - 4, y + 17);
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
  rect: Rect
): void {
  if (track.kind === "reference") {
    drawReferenceTrack(context, track, rect, viewport);
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

function drawReferenceTrack(context: CanvasRenderingContext2D, track: NormalizedReferenceTrack, rect: Rect, viewport: WellCorrelationViewport): void {
  const step = chooseDepthStep(viewport.depthEnd - viewport.depthStart);
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

function drawCorrelationLines(context: CanvasRenderingContext2D, layout: ReturnType<typeof layoutWellCorrelationPanel>, lines: WellCorrelationRenderFrame["state"]["correlationLines"], viewport: WellCorrelationViewport): void {
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
  const majorStep = chooseDepthStep(viewport.depthEnd - viewport.depthStart);
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

function updateScrollbarThumb(thumb: HTMLDivElement | null, fullStart: number, fullEnd: number, viewport: WellCorrelationViewport, scrollbarRect: Rect): void {
  if (!thumb) {
    return;
  }
  const fullSpan = Math.max(1e-6, fullEnd - fullStart);
  const viewportSpan = viewport.depthEnd - viewport.depthStart;
  const thumbHeight = Math.max(28, scrollbarRect.height * (viewportSpan / fullSpan));
  const available = Math.max(1, scrollbarRect.height - thumbHeight);
  const offset = ((viewport.depthStart - fullStart) / Math.max(1e-6, fullSpan - viewportSpan)) * available;
  thumb.style.top = `${clamp(offset, 0, available)}px`;
  thumb.style.height = `${thumbHeight}px`;
}

function headerRowsForTrack(track: NormalizedTrack, nativeDepthDatum: string): Array<{ label: string; color: string; axis?: { min: number; max: number } }> {
  if (track.kind === "reference") {
    return track.topOverlays.length > 0 ? [{ label: track.topOverlays[0]!.name, color: track.topOverlays[0]!.style.color }] : [{ label: nativeDepthDatum.toUpperCase(), color: "#444444" }];
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
    return track.layers.filter((layer): layer is NormalizedSeismicTraceLayer => layer.kind === "seismic-trace").flatMap((layer) => layer.traces.map((trace) => ({ label: trace.name, color: trace.style.positiveFill })));
  }
  return track.layers.filter((layer): layer is NormalizedSeismicSectionLayer => layer.kind === "seismic-section").map((layer) => ({ label: `${layer.name} (${layer.style.renderMode})`, color: layer.style.renderMode === "wiggle" ? "#9c2d2d" : "#4b4b4b" }));
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

function chooseDepthStep(span: number): number {
  if (span <= 100) return 10;
  if (span <= 250) return 25;
  if (span <= 500) return 50;
  return 100;
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

function formatAxisValue(value: number): string {
  if (Math.abs(value) >= 100) return value.toFixed(0);
  if (Math.abs(value) >= 10) return value.toFixed(1);
  return value.toFixed(2);
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
