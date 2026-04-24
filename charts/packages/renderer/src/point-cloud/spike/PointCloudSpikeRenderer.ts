import {
  buildCartesianTicks,
  cloneCartesianAxisOverrides,
  formatCartesianCanvasFont,
  formatCartesianTick,
  fitRockPhysicsViewport,
  getRockPhysicsCrossplotPlotRect,
  resolveCartesianPresentationProfile,
  resolveCartesianStageLayout,
  resolveCartesianAxisTitle,
  resolveCartesianTickCount,
  valueToRockPhysicsScreenX,
  valueToRockPhysicsScreenY
} from "@ophiolite/charts-core";
import type {
  CartesianAxisOverrides,
  InteractionState,
  RockPhysicsAxisRange,
  RockPhysicsPointSymbol,
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotProbe,
  RockPhysicsCrossplotViewport
} from "@ophiolite/charts-data-models";
import type {
  RockPhysicsCrossplotRenderFrame,
  RockPhysicsCrossplotRendererAdapter
} from "../adapter";
import { createRendererTelemetryEvent, type RendererTelemetryListener } from "../../telemetry";

interface PaletteSwatch {
  label: string;
  color: string;
  symbol?: RockPhysicsPointSymbol;
}

const BACKGROUND_COLOR = "#06141c";
const AXIS_COLOR = "#a9c1cf";
const GRID_COLOR = "rgba(169, 193, 207, 0.12)";
const TITLE_COLOR = "#eef7fb";
const SUBTITLE_COLOR = "#8eb0bf";
const PROBE_COLOR = "rgba(255, 244, 169, 0.92)";
const PROBE_FILL = "rgba(255, 244, 169, 0.18)";
const ROCK_PHYSICS_PRESENTATION = resolveCartesianPresentationProfile("rockPhysics");
const TICK_FONT = formatCartesianCanvasFont(ROCK_PHYSICS_PRESENTATION.typography.tick);
const AXIS_LABEL_FONT = formatCartesianCanvasFont(ROCK_PHYSICS_PRESENTATION.typography.axisLabel);
const TITLE_FONT = formatCartesianCanvasFont(ROCK_PHYSICS_PRESENTATION.typography.title);
const SUBTITLE_FONT = formatCartesianCanvasFont(ROCK_PHYSICS_PRESENTATION.typography.subtitle);
const ENABLE_WEBGL_POINT_CLOUD = false;

const EMPTY_INTERACTIONS: InteractionState = {
  capabilities: {
    primaryModes: ["cursor", "panZoom"],
    modifiers: ["crosshair"]
  },
  primaryMode: "cursor",
  modifiers: [],
  focused: false,
  hoverTarget: null,
  session: null
};

interface NormalizedRenderState {
  model: RockPhysicsCrossplotModel | null;
  viewport: RockPhysicsCrossplotViewport | null;
  probe: RockPhysicsCrossplotProbe | null;
  axisOverrides: CartesianAxisOverrides;
  interactions: InteractionState;
}

export class PointCloudSpikeRenderer implements RockPhysicsCrossplotRendererAdapter {
  private host: HTMLDivElement | null = null;
  private glCanvas: HTMLCanvasElement | null = null;
  private pointContext: CanvasRenderingContext2D | null = null;
  private overlayCanvas: HTMLCanvasElement | null = null;
  private overlayContext: CanvasRenderingContext2D | null = null;
  private gl: WebGL2RenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private positionBuffer: WebGLBuffer | null = null;
  private colorBuffer: WebGLBuffer | null = null;
  private symbolBuffer: WebGLBuffer | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private currentState: NormalizedRenderState = {
    model: null,
    viewport: null,
    probe: null,
    axisOverrides: {},
    interactions: EMPTY_INTERACTIONS
  };
  private uploadedModel: RockPhysicsCrossplotModel | null = null;
  private currentColors: Uint8Array | null = null;
  private currentSymbols: Float32Array | null = null;
  private telemetryListener: RendererTelemetryListener | null = null;

  setTelemetryListener(listener: RendererTelemetryListener | null): void {
    this.telemetryListener = listener;
  }

  mount(container: HTMLElement): void {
    this.host = document.createElement("div");
    this.host.style.position = "relative";
    this.host.style.width = "100%";
    this.host.style.height = "100%";
    this.host.style.background = BACKGROUND_COLOR;

    this.glCanvas = document.createElement("canvas");
    this.glCanvas.style.position = "absolute";
    this.glCanvas.style.inset = "0";
    this.glCanvas.style.width = "100%";
    this.glCanvas.style.height = "100%";

    this.overlayCanvas = document.createElement("canvas");
    this.overlayCanvas.style.position = "absolute";
    this.overlayCanvas.style.inset = "0";
    this.overlayCanvas.style.width = "100%";
    this.overlayCanvas.style.height = "100%";
    this.overlayCanvas.style.pointerEvents = "none";

    this.host.append(this.glCanvas, this.overlayCanvas);
    container.replaceChildren(this.host);

    if (ENABLE_WEBGL_POINT_CLOUD) {
      try {
        this.gl = this.glCanvas.getContext("webgl2", {
          antialias: true,
          premultipliedAlpha: false
        });
      } catch (error) {
        console.warn("RockPhysicsCrossplot: WebGL2 context creation failed, falling back to 2D canvas.", error);
        this.emitTelemetry({
          kind: "fallback-used",
          phase: "probe",
          backend: "canvas-2d",
          previousBackend: "webgl",
          recoverable: true,
          message: "Rock physics renderer fell back to canvas after WebGL2 context creation failed.",
          detail: error instanceof Error ? error.message : String(error)
        });
        this.gl = null;
      }
    }
    this.overlayContext = this.overlayCanvas.getContext("2d");
    if (!this.overlayContext) {
      this.emitTelemetry({
        kind: "mount-failed",
        phase: "mount",
        backend: this.gl ? "webgl" : "canvas-2d",
        recoverable: false,
        message: "Rock physics renderer could not acquire an overlay 2D canvas context."
      });
      throw new Error("PointCloudSpikeRenderer could not acquire an overlay 2D canvas context.");
    }
    if (this.gl) {
      try {
        this.program = createProgram(this.gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);
        this.positionBuffer = createBuffer(this.gl);
        this.colorBuffer = createBuffer(this.gl);
        this.symbolBuffer = createBuffer(this.gl);
        this.emitTelemetry({
          kind: "backend-selected",
          phase: "mount",
          backend: "webgl",
          recoverable: true,
          message: "Rock physics renderer selected the WebGL backend."
        });
      } catch (error) {
        console.warn("RockPhysicsCrossplot: WebGL program initialization failed, falling back to 2D canvas.", error);
        this.emitTelemetry({
          kind: "fallback-used",
          phase: "mount",
          backend: "canvas-2d",
          previousBackend: "webgl",
          recoverable: true,
          message: "Rock physics renderer fell back to canvas after WebGL program initialization failed.",
          detail: error instanceof Error ? error.message : String(error)
        });
        this.program = null;
        this.positionBuffer = null;
        this.colorBuffer = null;
        this.symbolBuffer = null;
        this.gl = null;
      }
    }
    if (!this.gl) {
      this.pointContext = this.glCanvas.getContext("2d");
      if (!this.pointContext) {
        this.emitTelemetry({
          kind: "mount-failed",
          phase: "mount",
          backend: "canvas-2d",
          recoverable: false,
          message: "Rock physics renderer could not acquire a 2D canvas context."
        });
        throw new Error("PointCloudSpikeRenderer could not acquire a 2D canvas context.");
      }
      this.emitTelemetry({
        kind: "backend-selected",
        phase: "mount",
        backend: "canvas-2d",
        recoverable: true,
        message: "Rock physics renderer selected the canvas-2d backend."
      });
    }

    this.resizeObserver = new ResizeObserver(() => {
      this.resize();
      this.draw();
    });
    this.resizeObserver.observe(container);
    this.resize();
  }

  render(input: RockPhysicsCrossplotRenderFrame | RockPhysicsCrossplotModel): void {
    try {
      const state = normalizeRenderState(input);
      this.currentState = state;
      this.uploadModelIfNeeded(state.model);
      this.draw();
    } catch (error) {
      this.emitTelemetry({
        kind: "frame-failed",
        phase: "render",
        backend: this.gl ? "webgl" : "canvas-2d",
        recoverable: !this.gl,
        message: error instanceof Error ? error.message : String(error),
        detail: "Rock physics renderer failed while drawing a frame."
      });
      throw error;
    }
  }

  dispose(): void {
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;

    if (this.gl && this.program) {
      this.gl.deleteProgram(this.program);
    }
    if (this.gl && this.positionBuffer) {
      this.gl.deleteBuffer(this.positionBuffer);
    }
    if (this.gl && this.colorBuffer) {
      this.gl.deleteBuffer(this.colorBuffer);
    }
    if (this.gl && this.symbolBuffer) {
      this.gl.deleteBuffer(this.symbolBuffer);
    }

    this.host?.remove();
    this.host = null;
    this.glCanvas = null;
    this.pointContext = null;
    this.overlayCanvas = null;
    this.overlayContext = null;
    this.gl = null;
    this.program = null;
    this.positionBuffer = null;
    this.colorBuffer = null;
    this.symbolBuffer = null;
    this.currentColors = null;
    this.currentSymbols = null;
    this.uploadedModel = null;
    this.currentState = {
      model: null,
      viewport: null,
      probe: null,
      axisOverrides: {},
      interactions: EMPTY_INTERACTIONS
    };
  }

  private uploadModelIfNeeded(model: RockPhysicsCrossplotModel | null): void {
    if (!model) {
      this.currentColors = null;
      this.currentSymbols = null;
      this.uploadedModel = model;
      return;
    }
    if (this.uploadedModel === model) {
      return;
    }

    this.currentColors = buildPointColors(model);
    this.currentSymbols = buildPointSymbols(model);

    const interleaved = new Float32Array(model.pointCount * 2);
    for (let index = 0; index < model.pointCount; index += 1) {
      interleaved[index * 2] = model.columns.x[index] ?? 0;
      interleaved[index * 2 + 1] = model.columns.y[index] ?? 0;
    }

    if (this.gl && this.positionBuffer && this.colorBuffer && this.symbolBuffer) {
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.positionBuffer);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, interleaved, this.gl.STATIC_DRAW);

      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.colorBuffer);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, this.currentColors, this.gl.STATIC_DRAW);

      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.symbolBuffer);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, this.currentSymbols, this.gl.STATIC_DRAW);
    }

    this.uploadedModel = model;
  }

  private resize(): void {
    const width = Math.max(1, Math.round(this.host?.clientWidth ?? 1));
    const height = Math.max(1, Math.round(this.host?.clientHeight ?? 1));
    const dpr = Math.max(1, window.devicePixelRatio || 1);
    const actualWidth = Math.round(width * dpr);
    const actualHeight = Math.round(height * dpr);

    for (const canvas of [this.glCanvas, this.overlayCanvas]) {
      if (!canvas) {
        continue;
      }
      if (canvas.width !== actualWidth || canvas.height !== actualHeight) {
        canvas.width = actualWidth;
        canvas.height = actualHeight;
      }
    }
    if (this.gl) {
      this.gl.viewport(0, 0, actualWidth, actualHeight);
    }
    if (this.pointContext) {
      this.pointContext.setTransform(1, 0, 0, 1, 0, 0);
      this.pointContext.scale(dpr, dpr);
    }
    if (this.overlayContext) {
      this.overlayContext.setTransform(1, 0, 0, 1, 0, 0);
      this.overlayContext.scale(dpr, dpr);
    }
  }

  private draw(): void {
    if (
      !this.host ||
      !this.glCanvas ||
      !this.overlayCanvas ||
      !this.overlayContext ||
      (!this.gl && !this.pointContext)
    ) {
      return;
    }

    const { model, probe, interactions } = this.currentState;
    const width = Math.max(1, this.host.clientWidth);
    const height = Math.max(1, this.host.clientHeight);
    const plotRect = getRockPhysicsCrossplotPlotRect(width, height);
    const dpr = Math.max(1, window.devicePixelRatio || 1);

    if (this.gl) {
      this.gl.clearColor(6 / 255, 20 / 255, 28 / 255, 1);
      this.gl.clear(this.gl.COLOR_BUFFER_BIT);
    }
    if (this.pointContext) {
      this.pointContext.clearRect(0, 0, width, height);
    }
    this.overlayContext.clearRect(0, 0, width, height);

    if (!model) {
      return;
    }

    const viewport = resolveViewport(model, this.currentState.viewport);

    if (this.gl && this.program && this.positionBuffer && this.colorBuffer && this.symbolBuffer) {
      try {
        this.gl.enable(this.gl.BLEND);
        this.gl.blendFunc(this.gl.SRC_ALPHA, this.gl.ONE_MINUS_SRC_ALPHA);
        this.gl.useProgram(this.program);

        const positionLocation = this.gl.getAttribLocation(this.program, "aPosition");
        const colorLocation = this.gl.getAttribLocation(this.program, "aColor");
        const symbolLocation = this.gl.getAttribLocation(this.program, "aSymbol");

        this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.positionBuffer);
        this.gl.enableVertexAttribArray(positionLocation);
        this.gl.vertexAttribPointer(positionLocation, 2, this.gl.FLOAT, false, 0, 0);

        this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.colorBuffer);
        this.gl.enableVertexAttribArray(colorLocation);
        this.gl.vertexAttribPointer(colorLocation, 4, this.gl.UNSIGNED_BYTE, true, 0, 0);

        this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.symbolBuffer);
        this.gl.enableVertexAttribArray(symbolLocation);
        this.gl.vertexAttribPointer(symbolLocation, 1, this.gl.FLOAT, false, 0, 0);

        this.gl.uniform2f(
          this.getRequiredUniformLocation("uCanvasSizePx"),
          this.glCanvas.width,
          this.glCanvas.height
        );
        this.gl.uniform4f(
          this.getRequiredUniformLocation("uPlotRectPx"),
          plotRect.x * dpr,
          plotRect.y * dpr,
          plotRect.width * dpr,
          plotRect.height * dpr
        );
        this.gl.uniform2f(this.getRequiredUniformLocation("uDomainX"), viewport.xMin, viewport.xMax);
        this.gl.uniform2f(this.getRequiredUniformLocation("uDomainY"), viewport.yMin, viewport.yMax);
        this.gl.uniform1f(this.getRequiredUniformLocation("uPointSizePx"), 5 * dpr);
        this.gl.drawArrays(this.gl.POINTS, 0, model.pointCount);
        const glError = this.gl.getError();
        if (glError !== this.gl.NO_ERROR || this.gl.isContextLost()) {
          throw new Error(`WebGL draw failed with code ${glError}.`);
        }
      } catch (error) {
        console.warn("RockPhysicsCrossplot: WebGL draw failed, falling back to 2D canvas.", error);
        this.emitTelemetry({
          kind: "fallback-used",
          phase: "render",
          backend: "canvas-2d",
          previousBackend: "webgl",
          recoverable: true,
          message: "Rock physics renderer fell back to canvas after a WebGL draw failure.",
          detail: error instanceof Error ? error.message : String(error)
        });
        this.gl = null;
        this.program = null;
        this.positionBuffer = null;
        this.colorBuffer = null;
        this.symbolBuffer = null;
        this.pointContext ??= this.glCanvas.getContext("2d");
        if (this.pointContext) {
          this.pointContext.setTransform(1, 0, 0, 1, 0, 0);
          this.pointContext.scale(dpr, dpr);
          this.pointContext.clearRect(0, 0, width, height);
          drawPointsCanvas(this.pointContext, model, viewport, plotRect, this.currentColors, this.currentSymbols);
        } else {
          this.emitTelemetry({
            kind: "mount-failed",
            phase: "render",
            backend: "canvas-2d",
            recoverable: false,
            message: "Rock physics renderer could not recover to a 2D canvas after WebGL draw failure."
          });
          throw new Error("PointCloudSpikeRenderer could not recover to a 2D canvas context.");
        }
      }
    } else if (this.pointContext) {
      drawPointsCanvas(this.pointContext, model, viewport, plotRect, this.currentColors, this.currentSymbols);
    }

    drawOverlay(this.overlayContext, {
      model,
      viewport,
      probe,
      axisOverrides: this.currentState.axisOverrides,
      interactions,
      width,
      height,
      plotRect
    });
  }

  private getRequiredUniformLocation(name: string): WebGLUniformLocation {
    if (!this.gl || !this.program) {
      throw new Error("Renderer is not initialized.");
    }
    const location = this.gl.getUniformLocation(this.program, name);
    if (!location) {
      throw new Error(`Uniform ${name} was not found.`);
    }
    return location;
  }

  private emitTelemetry(event: Parameters<typeof createRendererTelemetryEvent>[0]): void {
    this.telemetryListener?.(createRendererTelemetryEvent(event));
  }
}

function normalizeRenderState(
  input: RockPhysicsCrossplotRenderFrame | RockPhysicsCrossplotModel
): NormalizedRenderState {
  if ("state" in input) {
    return {
      model: input.state.model,
      viewport: input.state.viewport,
      probe: input.state.probe,
      axisOverrides: cloneCartesianAxisOverrides(input.state.axisOverrides),
      interactions: input.state.interactions
    };
  }

  return {
    model: input,
    viewport: fitRockPhysicsViewport(input),
    probe: null,
    axisOverrides: {},
    interactions: EMPTY_INTERACTIONS
  };
}

function resolveViewport(
  model: RockPhysicsCrossplotModel,
  viewport: RockPhysicsCrossplotViewport | null
): RockPhysicsCrossplotViewport {
  return (
    viewport ??
    fitRockPhysicsViewport(model) ?? {
      xMin: model.xAxis.range.min,
      xMax: model.xAxis.range.max,
      yMin: model.yAxis.range.min,
      yMax: model.yAxis.range.max
    }
  );
}

function drawOverlay(
  context: CanvasRenderingContext2D,
  options: {
    model: RockPhysicsCrossplotModel;
    viewport: RockPhysicsCrossplotViewport;
    probe: RockPhysicsCrossplotProbe | null;
    axisOverrides: CartesianAxisOverrides;
    interactions: InteractionState;
    width: number;
    height: number;
    plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>;
  }
): void {
  const { model, viewport, probe, axisOverrides, interactions, width, height } = options;
  const layout = resolveCartesianStageLayout(width, height, ROCK_PHYSICS_PRESENTATION);
  drawGrid(context, layout, viewport, axisOverrides, model);
  drawTemplateOverlays(context, layout.plotRect, model, viewport);
  drawAxes(context, layout, model, axisOverrides);
  drawTitle(context, model, layout);
  drawLegend(context, model, layout.plotRect, width);
  if (probe && interactions.modifiers.includes("crosshair")) {
    drawProbeCursor(context, layout.plotRect, viewport, probe, model);
  }
  if (interactions.session?.kind === "zoomRect") {
    drawZoomRectOverlay(context, layout.plotRect, interactions.session);
  }
}

function drawGrid(
  context: CanvasRenderingContext2D,
  layout: ReturnType<typeof resolveCartesianStageLayout>,
  viewport: RockPhysicsCrossplotViewport,
  axisOverrides: CartesianAxisOverrides,
  model: RockPhysicsCrossplotModel
): void {
  const { plotRect } = layout;
  context.save();
  context.strokeStyle = GRID_COLOR;
  context.lineWidth = 1;

  for (let step = 0; step <= 5; step += 1) {
    const ratio = step / 5;
    const x = plotRect.x + ratio * plotRect.width;
    const y = plotRect.y + ratio * plotRect.height;

    context.beginPath();
    context.moveTo(x, plotRect.y);
    context.lineTo(x, plotRect.y + plotRect.height);
    context.stroke();

    context.beginPath();
    context.moveTo(plotRect.x, y);
    context.lineTo(plotRect.x + plotRect.width, y);
    context.stroke();
  }

  context.restore();

  const xTicks = buildCartesianTicks(viewport.xMin, viewport.xMax, resolveCartesianTickCount(axisOverrides.x));
  const yTicks = buildCartesianTicks(viewport.yMin, viewport.yMax, resolveCartesianTickCount(axisOverrides.y));

  context.fillStyle = SUBTITLE_COLOR;
  context.font = TICK_FONT;
  context.textAlign = "center";
  context.textBaseline = "top";
  for (const tick of xTicks) {
    const x = valueToRockPhysicsScreenX(tick, viewport, plotRect);
    context.fillText(formatCartesianTick(tick, axisOverrides.x?.tickFormat), x, layout.xTickY);
  }

  context.save();
  context.textAlign = "right";
  context.textBaseline = "middle";
  for (const tick of yTicks) {
    const y = valueToRockPhysicsScreenY(tick, viewport, plotRect, model.yAxis.direction);
    context.fillText(formatCartesianTick(tick, axisOverrides.y?.tickFormat), layout.yTickX, y);
  }
  context.restore();
}

function drawTemplateOverlays(
  context: CanvasRenderingContext2D,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  model: RockPhysicsCrossplotModel,
  viewport: RockPhysicsCrossplotViewport
): void {
  const overlays = model.templateOverlays ?? templateOverlaysFromLines(model.templateLines);
  if (!overlays?.length) {
    return;
  }

  context.save();
  context.font = "12px Segoe UI";
  context.textBaseline = "bottom";
  context.shadowColor = "rgba(0, 0, 0, 0.35)";
  context.shadowBlur = 8;

  for (const overlay of overlays) {
    if (overlay.kind !== "polygon") {
      continue;
    }
    context.save();
    context.fillStyle = overlay.fillColor;
    context.strokeStyle = overlay.strokeColor ?? overlay.fillColor;
    context.lineWidth = 1.2;
    context.beginPath();
    overlay.points.forEach((point, index) => {
      const x = valueToRockPhysicsScreenX(point.x, viewport, plotRect);
      const y = valueToRockPhysicsScreenY(point.y, viewport, plotRect, model.yAxis.direction);
      if (index === 0) {
        context.moveTo(x, y);
      } else {
        context.lineTo(x, y);
      }
    });
    context.closePath();
    context.fill();
    context.stroke();
    if (overlay.label) {
      const position = overlay.labelPosition ?? centroid(overlay.points);
      const x = valueToRockPhysicsScreenX(position.x, viewport, plotRect);
      const y = valueToRockPhysicsScreenY(position.y, viewport, plotRect, model.yAxis.direction);
      context.fillStyle = TITLE_COLOR;
      context.fillText(overlay.label, x, y);
    }
    context.restore();
  }

  for (const overlay of overlays) {
    if (overlay.kind !== "axis") {
      continue;
    }
    drawAxisOverlay(context, plotRect, viewport, overlay, model.yAxis.direction);
  }

  for (const overlay of overlays) {
    if (overlay.kind !== "polyline") {
      continue;
    }
    context.save();
    context.strokeStyle = overlay.color;
    context.fillStyle = overlay.color;
    context.lineWidth = overlay.width ?? 1.5;
    context.setLineDash(overlay.dashed ? [7, 5] : []);
    context.beginPath();
    overlay.points.forEach((point, index) => {
      const x = valueToRockPhysicsScreenX(point.x, viewport, plotRect);
      const y = valueToRockPhysicsScreenY(point.y, viewport, plotRect, model.yAxis.direction);
      if (index === 0) {
        context.moveTo(x, y);
      } else {
        context.lineTo(x, y);
      }
    });
    context.stroke();

    const anchor = overlay.points[overlay.points.length - 1];
    if (anchor && overlay.label) {
      const x = valueToRockPhysicsScreenX(anchor.x, viewport, plotRect) - 4;
      const y = valueToRockPhysicsScreenY(anchor.y, viewport, plotRect, model.yAxis.direction) - 4;
      context.fillText(overlay.label, x, y);
    }
    context.restore();
  }

  for (const overlay of overlays) {
    if (overlay.kind !== "text") {
      continue;
    }
    const x = valueToRockPhysicsScreenX(overlay.x, viewport, plotRect);
    const y = valueToRockPhysicsScreenY(overlay.y, viewport, plotRect, model.yAxis.direction);
    context.save();
    context.translate(x, y);
    context.rotate(((overlay.rotationDeg ?? 0) * Math.PI) / 180);
    context.fillStyle = overlay.color;
    context.textAlign = overlay.align ?? "left";
    context.textBaseline = overlay.baseline ?? "middle";
    context.fillText(overlay.text, 0, 0);
    context.restore();
  }

  context.restore();
}

function drawAxisOverlay(
  context: CanvasRenderingContext2D,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  viewport: RockPhysicsCrossplotViewport,
  overlay: Extract<NonNullable<RockPhysicsCrossplotModel["templateOverlays"]>[number], { kind: "axis" }>,
  yDirection: NonNullable<RockPhysicsCrossplotModel["yAxis"]["direction"]> = "normal"
): void {
  const screenPoints = overlay.points.map((point) => ({
    x: valueToScreenXUnclamped(point.x, viewport, plotRect),
    y: valueToScreenYUnclamped(point.y, viewport, plotRect, yDirection)
  }));
  if (screenPoints.length < 2) {
    return;
  }

  const path = buildMeasuredPath(screenPoints);
  if (path.totalLength <= 0) {
    return;
  }

  const tickLengthPx = overlay.tickLengthPx ?? 8;
  const tickLabelOffsetPx = overlay.tickLabelOffsetPx ?? 5;
  const labelOffsetPx = overlay.labelOffsetPx ?? tickLengthPx + tickLabelOffsetPx + 10;

  context.save();
  clipToRect(context, plotRect);

  context.strokeStyle = overlay.color;
  context.lineWidth = overlay.width ?? 1.5;
  context.beginPath();
  screenPoints.forEach((point, index) => {
    if (index === 0) {
      context.moveTo(point.x, point.y);
    } else {
      context.lineTo(point.x, point.y);
    }
  });
  context.stroke();

  context.fillStyle = overlay.color;
  context.strokeStyle = overlay.color;
  context.font = TICK_FONT;
  context.lineWidth = 1;

  for (const tick of overlay.ticks) {
    const sample = sampleMeasuredPath(path, ratioForDomainValue(tick.value, overlay.domain));
    if (!sample) {
      continue;
    }
    const normal = resolveAxisNormal(sample.tangent, overlay.side ?? "left");
    const tickLengthPx = tick.lengthPx ?? overlay.tickLengthPx ?? 8;
    const tickEnd = {
      x: sample.point.x + normal.x * tickLengthPx,
      y: sample.point.y + normal.y * tickLengthPx
    };

    context.beginPath();
    context.moveTo(sample.point.x, sample.point.y);
    context.lineTo(tickEnd.x, tickEnd.y);
    context.stroke();

    const labelPoint = {
      x: sample.point.x + normal.x * (tickLengthPx + tickLabelOffsetPx),
      y: sample.point.y + normal.y * (tickLengthPx + tickLabelOffsetPx)
    };
    applyAxisTextAlignment(context, normal);
    context.fillText(tick.label ?? formatAxisTick(tick.value), labelPoint.x, labelPoint.y);
  }

  const labelValue = overlay.labelValue ?? (overlay.domain.min + overlay.domain.max) / 2;
  const labelSample = sampleMeasuredPath(path, ratioForDomainValue(labelValue, overlay.domain));
  if (labelSample) {
    const normal = resolveAxisNormal(labelSample.tangent, overlay.side ?? "left");
    const labelPoint = {
      x: labelSample.point.x + normal.x * labelOffsetPx,
      y: labelSample.point.y + normal.y * labelOffsetPx
    };
    let angle = Math.atan2(labelSample.tangent.y, labelSample.tangent.x);
    if (Math.abs(angle) > Math.PI / 2) {
      angle += Math.PI;
    }
    context.save();
    context.translate(labelPoint.x, labelPoint.y);
    context.rotate(angle);
    context.fillStyle = overlay.color;
    context.font = AXIS_LABEL_FONT;
    context.textAlign = "center";
    context.textBaseline = "middle";
    context.fillText(overlay.label, 0, 0);
    context.restore();
  }

  context.restore();
}

function drawAxes(
  context: CanvasRenderingContext2D,
  layout: ReturnType<typeof resolveCartesianStageLayout>,
  model: RockPhysicsCrossplotModel,
  axisOverrides: CartesianAxisOverrides
): void {
  const { plotRect } = layout;
  context.save();
  context.strokeStyle = AXIS_COLOR;
  context.lineWidth = 1.2;

  context.strokeStyle = "rgba(142, 176, 191, 0.35)";
  context.strokeRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);
  context.strokeStyle = AXIS_COLOR;

  context.beginPath();
  context.moveTo(plotRect.x, plotRect.y);
  context.lineTo(plotRect.x, plotRect.y + plotRect.height);
  context.lineTo(plotRect.x + plotRect.width, plotRect.y + plotRect.height);
  context.stroke();

  context.fillStyle = AXIS_COLOR;
  context.font = AXIS_LABEL_FONT;
  context.textAlign = "center";
  context.textBaseline = "alphabetic";
  context.fillText(
    resolveCartesianAxisTitle("X Axis", model.xAxis.label, model.xAxis.unit, axisOverrides.x),
    plotRect.x + plotRect.width / 2,
    layout.xAxisLabelY
  );

  context.save();
  context.translate(layout.yAxisLabelX, plotRect.y + plotRect.height / 2);
  context.rotate(-Math.PI / 2);
  context.fillText(
    resolveCartesianAxisTitle("Y Axis", model.yAxis.label, model.yAxis.unit, axisOverrides.y),
    0,
    0
  );
  context.restore();
  context.restore();
}

function drawTitle(
  context: CanvasRenderingContext2D,
  model: RockPhysicsCrossplotModel,
  layout: ReturnType<typeof resolveCartesianStageLayout>
): void {
  context.fillStyle = TITLE_COLOR;
  context.font = TITLE_FONT;
  context.textAlign = "left";
  context.textBaseline = "top";
  context.fillText(model.title, layout.title.x, layout.title.y);

  context.fillStyle = SUBTITLE_COLOR;
  context.font = SUBTITLE_FONT;
  context.fillText(
    `${model.subtitle ?? model.name} • ${model.pointCount.toLocaleString()} samples`,
    layout.subtitle.x,
    layout.subtitle.y
  );

  context.textAlign = "right";
  context.fillText(model.templateId, layout.plotRect.x + layout.plotRect.width, layout.title.y);
}

function drawLegend(
  context: CanvasRenderingContext2D,
  model: RockPhysicsCrossplotModel,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  width: number
): void {
  const legendX = Math.min(width - 184, plotRect.x + plotRect.width - 164);
  const legendY = plotRect.y + 14;

  context.save();
  context.fillStyle = "rgba(8, 20, 28, 0.84)";
  context.fillRect(
    legendX,
    legendY,
    164,
    model.colorBinding.kind === "categorical" ? 28 + model.colorBinding.categories.length * 18 : 88
  );

  context.fillStyle = TITLE_COLOR;
  context.font = "600 12px Segoe UI";
  context.textAlign = "left";
  context.textBaseline = "top";
  context.fillText(model.colorBinding.label, legendX + 10, legendY + 8);

  if (model.colorBinding.kind === "categorical") {
    const swatches: PaletteSwatch[] = model.colorBinding.categories.map((category) => ({
      label: category.label,
      color: category.color,
      symbol: category.symbol
    }));
    drawSwatches(context, swatches, legendX + 10, legendY + 28);
  } else {
    const gradient = context.createLinearGradient(legendX + 10, 0, legendX + 130, 0);
    const { palette, range } = model.colorBinding;
    palette.forEach((color, index) => {
      gradient.addColorStop(index / Math.max(1, palette.length - 1), color);
    });
    context.fillStyle = gradient;
    context.fillRect(legendX + 10, legendY + 34, 120, 12);
    context.fillStyle = SUBTITLE_COLOR;
    context.font = "11px Segoe UI";
    context.fillText(range.min.toFixed(2), legendX + 10, legendY + 54);
    context.textAlign = "right";
    context.fillText(range.max.toFixed(2), legendX + 130, legendY + 54);
  }

  context.restore();
}

function drawSwatches(context: CanvasRenderingContext2D, swatches: PaletteSwatch[], x: number, y: number): void {
  context.font = "11px Segoe UI";
  context.textAlign = "left";
  context.textBaseline = "middle";
  swatches.forEach((swatch, index) => {
    const top = y + index * 18;
    context.fillStyle = swatch.color;
    drawSymbol(context, swatch.symbol ?? "circle", x + 5, top + 6, 4, swatch.color);
    context.fillStyle = SUBTITLE_COLOR;
    context.fillText(swatch.label, x + 16, top + 6);
  });
}

function drawProbeCursor(
  context: CanvasRenderingContext2D,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  viewport: RockPhysicsCrossplotViewport,
  probe: RockPhysicsCrossplotProbe,
  model: RockPhysicsCrossplotModel
): void {
  const x = valueToRockPhysicsScreenX(probe.xValue, viewport, plotRect);
  const y = valueToRockPhysicsScreenY(probe.yValue, viewport, plotRect, model.yAxis.direction);

  context.save();
  context.strokeStyle = PROBE_COLOR;
  context.fillStyle = PROBE_FILL;
  context.lineWidth = 1;

  context.beginPath();
  context.moveTo(x, plotRect.y);
  context.lineTo(x, plotRect.y + plotRect.height);
  context.stroke();

  context.beginPath();
  context.moveTo(plotRect.x, y);
  context.lineTo(plotRect.x + plotRect.width, y);
  context.stroke();

  context.beginPath();
  context.arc(x, y, 7, 0, Math.PI * 2);
  context.fill();
  context.stroke();
  context.restore();
}

function drawPointsCanvas(
  context: CanvasRenderingContext2D,
  model: RockPhysicsCrossplotModel,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  colors: Uint8Array | null,
  symbols: Float32Array | null
): void {
  context.save();
  clipToRect(context, plotRect);
  for (let index = 0; index < model.pointCount; index += 1) {
    const xValue = model.columns.x[index];
    const yValue = model.columns.y[index];
    if (!Number.isFinite(xValue) || !Number.isFinite(yValue)) {
      continue;
    }

    const x = valueToRockPhysicsScreenX(xValue, viewport, plotRect);
    const y = valueToRockPhysicsScreenY(yValue, viewport, plotRect, model.yAxis.direction);
    const rgba = colorAt(colors, index);
    const color = `rgba(${rgba[0]}, ${rgba[1]}, ${rgba[2]}, ${rgba[3] / 255})`;
    drawSymbol(context, indexToSymbol(symbols?.[index] ?? 0), x, y, 2.5, color);
  }
  context.restore();
}

function buildPointColors(model: RockPhysicsCrossplotModel): Uint8Array {
  const colors = new Uint8Array(model.pointCount * 4);

  if (model.colorBinding.kind === "categorical" && model.columns.colorCategoryIds) {
    const palette = new Map(
      model.colorBinding.categories.map((category) => [category.id, parseHexColor(category.color)])
    );
    for (let index = 0; index < model.pointCount; index += 1) {
      const rgba = palette.get(model.columns.colorCategoryIds[index] ?? 0) ?? [255, 255, 255, 220];
      writeColor(colors, index, rgba);
    }
    return colors;
  }

  const min = model.colorBinding.kind === "continuous" ? model.colorBinding.range.min : 0;
  const max = model.colorBinding.kind === "continuous" ? model.colorBinding.range.max : 1;
  const palette: Array<[number, number, number, number]> =
    model.colorBinding.kind === "continuous"
      ? model.colorBinding.palette.map((color) => parseHexColor(color))
      : [[255, 255, 255, 220]];
  const scalars = model.columns.colorScalars;

  for (let index = 0; index < model.pointCount; index += 1) {
    const value = scalars?.[index] ?? 0;
    const rgba = interpolatePalette(palette, min === max ? 0 : (value - min) / (max - min));
    writeColor(colors, index, rgba);
  }

  return colors;
}

function buildPointSymbols(model: RockPhysicsCrossplotModel): Float32Array {
  const symbols = new Float32Array(model.pointCount);
  if (model.colorBinding.kind !== "categorical") {
    return symbols;
  }

  const sourceCategoryIds = model.columns.symbolCategoryIds ?? model.columns.colorCategoryIds;
  if (!sourceCategoryIds) {
    return symbols;
  }

  const symbolByCategoryId = new Map(
    model.colorBinding.categories.map((category) => [category.id, symbolToIndex(category.symbol ?? "circle")])
  );
  for (let index = 0; index < model.pointCount; index += 1) {
    symbols[index] = symbolByCategoryId.get(sourceCategoryIds[index] ?? 0) ?? 0;
  }
  return symbols;
}

function writeColor(target: Uint8Array, index: number, rgba: [number, number, number, number]): void {
  const offset = index * 4;
  target[offset] = rgba[0];
  target[offset + 1] = rgba[1];
  target[offset + 2] = rgba[2];
  target[offset + 3] = rgba[3];
}

function interpolatePalette(
  palette: Array<[number, number, number, number]>,
  ratio: number
): [number, number, number, number] {
  const clamped = Math.min(Math.max(ratio, 0), 1);
  if (palette.length === 1) {
    return palette[0]!;
  }
  const scaled = clamped * (palette.length - 1);
  const leftIndex = Math.floor(scaled);
  const rightIndex = Math.min(palette.length - 1, leftIndex + 1);
  const blend = scaled - leftIndex;
  const left = palette[leftIndex]!;
  const right = palette[rightIndex]!;
  return [
    Math.round(left[0] + (right[0] - left[0]) * blend),
    Math.round(left[1] + (right[1] - left[1]) * blend),
    Math.round(left[2] + (right[2] - left[2]) * blend),
    Math.round(left[3] + (right[3] - left[3]) * blend)
  ];
}

function parseHexColor(color: string): [number, number, number, number] {
  const normalized = color.replace("#", "");
  const hex =
    normalized.length === 3
      ? normalized
          .split("")
          .map((character) => `${character}${character}`)
          .join("")
      : normalized;
  const red = Number.parseInt(hex.slice(0, 2), 16);
  const green = Number.parseInt(hex.slice(2, 4), 16);
  const blue = Number.parseInt(hex.slice(4, 6), 16);
  return [red, green, blue, 220];
}

function drawSymbol(
  context: CanvasRenderingContext2D,
  symbol: RockPhysicsPointSymbol,
  x: number,
  y: number,
  radius: number,
  color: string
): void {
  context.save();
  context.fillStyle = color;
  context.strokeStyle = color;
  context.lineWidth = 1;
  context.beginPath();
  switch (symbol) {
    case "square":
      context.rect(x - radius, y - radius, radius * 2, radius * 2);
      break;
    case "diamond":
      context.moveTo(x, y - radius);
      context.lineTo(x + radius, y);
      context.lineTo(x, y + radius);
      context.lineTo(x - radius, y);
      context.closePath();
      break;
    case "triangle":
      context.moveTo(x, y - radius);
      context.lineTo(x + radius, y + radius);
      context.lineTo(x - radius, y + radius);
      context.closePath();
      break;
    case "circle":
    default:
      context.arc(x, y, radius, 0, Math.PI * 2);
      break;
  }
  context.fill();
  context.restore();
}

function colorAt(colors: Uint8Array | null, index: number): [number, number, number, number] {
  if (!colors) {
    return [255, 255, 255, 220];
  }
  const offset = index * 4;
  return [
    colors[offset] ?? 255,
    colors[offset + 1] ?? 255,
    colors[offset + 2] ?? 255,
    colors[offset + 3] ?? 220
  ];
}

function symbolToIndex(symbol: RockPhysicsPointSymbol): number {
  switch (symbol) {
    case "square":
      return 1;
    case "diamond":
      return 2;
    case "triangle":
      return 3;
    case "circle":
    default:
      return 0;
  }
}

function indexToSymbol(value: number): RockPhysicsPointSymbol {
  switch (Math.round(value)) {
    case 1:
      return "square";
    case 2:
      return "diamond";
    case 3:
      return "triangle";
    default:
      return "circle";
  }
}

function templateOverlaysFromLines(
  lines: readonly NonNullable<RockPhysicsCrossplotModel["templateLines"]>[number][] | undefined
): RockPhysicsCrossplotModel["templateOverlays"] {
  return lines?.map((line) => ({
    kind: "polyline",
    id: line.id,
    label: line.label,
    color: line.color,
    points: line.points.map((point) => ({ ...point }))
  }));
}

function centroid(points: Array<{ x: number; y: number }>): { x: number; y: number } {
  const sum = points.reduce(
    (accumulator, point) => ({
      x: accumulator.x + point.x,
      y: accumulator.y + point.y
    }),
    { x: 0, y: 0 }
  );
  return {
    x: sum.x / Math.max(1, points.length),
    y: sum.y / Math.max(1, points.length)
  };
}

function drawZoomRectOverlay(
  context: CanvasRenderingContext2D,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  session: Extract<InteractionState["session"], { kind: "zoomRect" }>
): void {
  const left = Math.max(plotRect.x, Math.min(session.origin.x, session.current.x));
  const top = Math.max(plotRect.y, Math.min(session.origin.y, session.current.y));
  const right = Math.min(plotRect.x + plotRect.width, Math.max(session.origin.x, session.current.x));
  const bottom = Math.min(plotRect.y + plotRect.height, Math.max(session.origin.y, session.current.y));
  const width = right - left;
  const height = bottom - top;
  if (width < 2 || height < 2) {
    return;
  }

  context.save();
  context.fillStyle = "rgba(180, 214, 232, 0.12)";
  context.strokeStyle = "rgba(223, 232, 238, 0.88)";
  context.lineWidth = 1;
  context.setLineDash([5, 4]);
  context.fillRect(left, top, width, height);
  context.strokeRect(left + 0.5, top + 0.5, Math.max(0, width - 1), Math.max(0, height - 1));
  context.restore();
}

function clipToRect(
  context: CanvasRenderingContext2D,
  rect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>
): void {
  context.beginPath();
  context.rect(rect.x, rect.y, rect.width, rect.height);
  context.clip();
}

function valueToScreenXUnclamped(
  value: number,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>
): number {
  const ratio = (value - viewport.xMin) / Math.max(1e-6, viewport.xMax - viewport.xMin);
  return plotRect.x + ratio * plotRect.width;
}

function valueToScreenYUnclamped(
  value: number,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  direction: NonNullable<RockPhysicsCrossplotModel["yAxis"]["direction"]> = "normal"
): number {
  const ratio = (value - viewport.yMin) / Math.max(1e-6, viewport.yMax - viewport.yMin);
  return direction === "reversed"
    ? plotRect.y + ratio * plotRect.height
    : plotRect.y + plotRect.height - ratio * plotRect.height;
}

function ratioForDomainValue(value: number, domain: RockPhysicsAxisRange): number {
  const span = Math.max(1e-6, domain.max - domain.min);
  return clamp01((value - domain.min) / span);
}

function formatAxisTick(value: number): string {
  return Number.isInteger(value) ? `${value}` : formatCartesianTick(value, "auto");
}

function applyAxisTextAlignment(
  context: CanvasRenderingContext2D,
  normal: { x: number; y: number }
): void {
  context.textAlign =
    Math.abs(normal.x) > 0.4 ? (normal.x >= 0 ? "left" : "right") : "center";
  context.textBaseline =
    Math.abs(normal.y) > 0.4 ? (normal.y >= 0 ? "top" : "bottom") : "middle";
}

function resolveAxisNormal(
  tangent: { x: number; y: number },
  side: "left" | "right"
): { x: number; y: number } {
  const normal =
    side === "left"
      ? { x: -tangent.y, y: tangent.x }
      : { x: tangent.y, y: -tangent.x };
  const length = Math.hypot(normal.x, normal.y) || 1;
  return {
    x: normal.x / length,
    y: normal.y / length
  };
}

function buildMeasuredPath(points: Array<{ x: number; y: number }>): {
  segments: Array<{
    start: { x: number; y: number };
    end: { x: number; y: number };
    startLength: number;
    length: number;
  }>;
  totalLength: number;
} {
  const segments: Array<{
    start: { x: number; y: number };
    end: { x: number; y: number };
    startLength: number;
    length: number;
  }> = [];
  let totalLength = 0;
  for (let index = 1; index < points.length; index += 1) {
    const start = points[index - 1]!;
    const end = points[index]!;
    const length = Math.hypot(end.x - start.x, end.y - start.y);
    if (length <= 0) {
      continue;
    }
    segments.push({
      start,
      end,
      startLength: totalLength,
      length
    });
    totalLength += length;
  }
  return { segments, totalLength };
}

function sampleMeasuredPath(
  path: ReturnType<typeof buildMeasuredPath>,
  ratio: number
): { point: { x: number; y: number }; tangent: { x: number; y: number } } | null {
  if (!path.segments.length || path.totalLength <= 0) {
    return null;
  }
  const targetLength = clamp01(ratio) * path.totalLength;
  const segment =
    path.segments.find((candidate) => candidate.startLength + candidate.length >= targetLength) ??
    path.segments[path.segments.length - 1]!;
  const segmentRatio = (targetLength - segment.startLength) / segment.length;
  const point = {
    x: segment.start.x + (segment.end.x - segment.start.x) * segmentRatio,
    y: segment.start.y + (segment.end.y - segment.start.y) * segmentRatio
  };
  const tangentLength = Math.hypot(segment.end.x - segment.start.x, segment.end.y - segment.start.y) || 1;
  return {
    point,
    tangent: {
      x: (segment.end.x - segment.start.x) / tangentLength,
      y: (segment.end.y - segment.start.y) / tangentLength
    }
  };
}

function clamp01(value: number): number {
  return Math.min(Math.max(value, 0), 1);
}

function createProgram(gl: WebGL2RenderingContext, vertexSource: string, fragmentSource: string): WebGLProgram {
  const program = gl.createProgram();
  if (!program) {
    throw new Error("Failed to create point-cloud WebGL program.");
  }
  const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);

  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);

  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) ?? "Failed to link point-cloud WebGL program.");
  }
  return program;
}

function compileShader(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) {
    throw new Error("Failed to create point-cloud shader.");
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader) ?? "Failed to compile point-cloud shader.");
  }
  return shader;
}

function createBuffer(gl: WebGL2RenderingContext): WebGLBuffer {
  const buffer = gl.createBuffer();
  if (!buffer) {
    throw new Error("Failed to create point-cloud buffer.");
  }
  return buffer;
}

const VERTEX_SHADER_SOURCE = `#version 300 es
precision highp float;

in vec2 aPosition;
in vec4 aColor;
in float aSymbol;

uniform vec2 uCanvasSizePx;
uniform vec4 uPlotRectPx;
uniform vec2 uDomainX;
uniform vec2 uDomainY;
uniform float uPointSizePx;

out vec4 vColor;
flat out int vSymbol;

void main() {
  float xRatio = clamp((aPosition.x - uDomainX.x) / max(0.000001, uDomainX.y - uDomainX.x), 0.0, 1.0);
  float yRatio = clamp((aPosition.y - uDomainY.x) / max(0.000001, uDomainY.y - uDomainY.x), 0.0, 1.0);

  float xPx = uPlotRectPx.x + xRatio * uPlotRectPx.z;
  float yPx = uPlotRectPx.y + (1.0 - yRatio) * uPlotRectPx.w;

  float xClip = (xPx / uCanvasSizePx.x) * 2.0 - 1.0;
  float yClip = 1.0 - (yPx / uCanvasSizePx.y) * 2.0;

  gl_Position = vec4(xClip, yClip, 0.0, 1.0);
  gl_PointSize = uPointSizePx;
  vColor = aColor;
  vSymbol = int(aSymbol + 0.5);
}
`;

const FRAGMENT_SHADER_SOURCE = `#version 300 es
precision mediump float;

in vec4 vColor;
flat in int vSymbol;
out vec4 outColor;

void main() {
  vec2 centered = gl_PointCoord * 2.0 - 1.0;
  if (vSymbol == 0) {
    if (dot(centered, centered) > 1.0) {
      discard;
    }
  } else if (vSymbol == 2) {
    if (abs(centered.x) + abs(centered.y) > 1.0) {
      discard;
    }
  } else if (vSymbol == 3) {
    if (centered.y < -1.0 || abs(centered.x) > (1.0 - centered.y) * 0.5) {
      discard;
    }
  }
  outColor = vColor;
}
`;
