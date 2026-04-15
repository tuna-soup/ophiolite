import {
  fitRockPhysicsViewport,
  getRockPhysicsCrossplotPlotRect,
  valueToRockPhysicsScreenX,
  valueToRockPhysicsScreenY
} from "@ophiolite/charts-core";
import type {
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
  interactions: InteractionState;
}

export class PointCloudSpikeRenderer implements RockPhysicsCrossplotRendererAdapter {
  private host: HTMLDivElement | null = null;
  private glCanvas: HTMLCanvasElement | null = null;
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
    interactions: EMPTY_INTERACTIONS
  };
  private uploadedModel: RockPhysicsCrossplotModel | null = null;
  private currentColors: Uint8Array | null = null;
  private currentSymbols: Float32Array | null = null;

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

    this.gl = this.glCanvas.getContext("webgl2", {
      antialias: true,
      premultipliedAlpha: false
    });
    this.overlayContext = this.overlayCanvas.getContext("2d");

    if (!this.gl) {
      throw new Error("WebGL2 is required for the point-cloud spike renderer.");
    }

    this.program = createProgram(this.gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);
    this.positionBuffer = createBuffer(this.gl);
    this.colorBuffer = createBuffer(this.gl);
    this.symbolBuffer = createBuffer(this.gl);

    this.resizeObserver = new ResizeObserver(() => {
      this.resize();
      this.draw();
    });
    this.resizeObserver.observe(container);
    this.resize();
  }

  render(input: RockPhysicsCrossplotRenderFrame | RockPhysicsCrossplotModel): void {
    const state = normalizeRenderState(input);
    this.currentState = state;
    this.uploadModelIfNeeded(state.model);
    this.draw();
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
      interactions: EMPTY_INTERACTIONS
    };
  }

  private uploadModelIfNeeded(model: RockPhysicsCrossplotModel | null): void {
    if (!this.gl || !this.positionBuffer || !this.colorBuffer || !this.symbolBuffer || !model) {
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

    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.positionBuffer);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, interleaved, this.gl.STATIC_DRAW);

    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.colorBuffer);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, this.currentColors, this.gl.STATIC_DRAW);

    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.symbolBuffer);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, this.currentSymbols, this.gl.STATIC_DRAW);

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
    if (this.overlayContext) {
      this.overlayContext.setTransform(1, 0, 0, 1, 0, 0);
      this.overlayContext.scale(dpr, dpr);
    }
  }

  private draw(): void {
    if (
      !this.gl ||
      !this.program ||
      !this.positionBuffer ||
      !this.colorBuffer ||
      !this.symbolBuffer ||
      !this.host ||
      !this.glCanvas ||
      !this.overlayCanvas ||
      !this.overlayContext
    ) {
      return;
    }

    const { model, probe, interactions } = this.currentState;
    const width = Math.max(1, this.host.clientWidth);
    const height = Math.max(1, this.host.clientHeight);
    const plotRect = getRockPhysicsCrossplotPlotRect(width, height);
    const dpr = Math.max(1, window.devicePixelRatio || 1);

    this.gl.clearColor(6 / 255, 20 / 255, 28 / 255, 1);
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);
    this.overlayContext.clearRect(0, 0, width, height);

    if (!model) {
      return;
    }

    const viewport = resolveViewport(model, this.currentState.viewport);

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

    drawOverlay(this.overlayContext, {
      model,
      viewport,
      probe,
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
}

function normalizeRenderState(
  input: RockPhysicsCrossplotRenderFrame | RockPhysicsCrossplotModel
): NormalizedRenderState {
  if ("state" in input) {
    return {
      model: input.state.model,
      viewport: input.state.viewport,
      probe: input.state.probe,
      interactions: input.state.interactions
    };
  }

  return {
    model: input,
    viewport: fitRockPhysicsViewport(input),
    probe: null,
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
    interactions: InteractionState;
    width: number;
    height: number;
    plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>;
  }
): void {
  const { model, viewport, probe, interactions, width, plotRect } = options;
  drawGrid(context, plotRect, viewport);
  drawTemplateOverlays(context, plotRect, model, viewport);
  drawAxes(context, plotRect, model);
  drawTitle(context, model, width);
  drawLegend(context, model, plotRect, width);
  if (probe && interactions.modifiers.includes("crosshair")) {
    drawProbeCursor(context, plotRect, viewport, probe);
  }
}

function drawGrid(
  context: CanvasRenderingContext2D,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  viewport: RockPhysicsCrossplotViewport
): void {
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

  const xTicks = buildTicks(viewport.xMin, viewport.xMax, 6);
  const yTicks = buildTicks(viewport.yMin, viewport.yMax, 6);

  context.fillStyle = SUBTITLE_COLOR;
  context.font = "12px Segoe UI";
  context.textAlign = "center";
  context.textBaseline = "top";
  for (const tick of xTicks) {
    const x = valueToRockPhysicsScreenX(tick, viewport, plotRect);
    context.fillText(formatTick(tick), x, plotRect.y + plotRect.height + 10);
  }

  context.save();
  context.textAlign = "right";
  context.textBaseline = "middle";
  for (const tick of yTicks) {
    const y = valueToRockPhysicsScreenY(tick, viewport, plotRect);
    context.fillText(formatTick(tick), plotRect.x - 10, y);
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
      const y = valueToRockPhysicsScreenY(point.y, viewport, plotRect);
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
      const y = valueToRockPhysicsScreenY(position.y, viewport, plotRect);
      context.fillStyle = TITLE_COLOR;
      context.fillText(overlay.label, x, y);
    }
    context.restore();
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
      const y = valueToRockPhysicsScreenY(point.y, viewport, plotRect);
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
      const y = valueToRockPhysicsScreenY(anchor.y, viewport, plotRect) - 4;
      context.fillText(overlay.label, x, y);
    }
    context.restore();
  }

  for (const overlay of overlays) {
    if (overlay.kind !== "text") {
      continue;
    }
    const x = valueToRockPhysicsScreenX(overlay.x, viewport, plotRect);
    const y = valueToRockPhysicsScreenY(overlay.y, viewport, plotRect);
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

function drawAxes(
  context: CanvasRenderingContext2D,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  model: RockPhysicsCrossplotModel
): void {
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
  context.font = "600 13px Segoe UI";
  context.textAlign = "center";
  context.textBaseline = "alphabetic";
  context.fillText(
    `${model.xAxis.label}${model.xAxis.unit ? ` (${model.xAxis.unit})` : ""}`,
    plotRect.x + plotRect.width / 2,
    plotRect.y + plotRect.height + 42
  );

  context.save();
  context.translate(20, plotRect.y + plotRect.height / 2);
  context.rotate(-Math.PI / 2);
  context.fillText(
    `${model.yAxis.label}${model.yAxis.unit ? ` (${model.yAxis.unit})` : ""}`,
    0,
    0
  );
  context.restore();
  context.restore();
}

function drawTitle(context: CanvasRenderingContext2D, model: RockPhysicsCrossplotModel, width: number): void {
  context.fillStyle = TITLE_COLOR;
  context.font = "600 16px Segoe UI";
  context.textAlign = "left";
  context.textBaseline = "top";
  context.fillText(model.title, 20, 12);

  context.fillStyle = SUBTITLE_COLOR;
  context.font = "12px Segoe UI";
  context.fillText(
    `${model.subtitle ?? model.name} • ${model.pointCount.toLocaleString()} samples`,
    20,
    34
  );

  context.textAlign = "right";
  context.fillText(model.templateId, width - 20, 12);
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
  probe: RockPhysicsCrossplotProbe
): void {
  const x = valueToRockPhysicsScreenX(probe.xValue, viewport, plotRect);
  const y = valueToRockPhysicsScreenY(probe.yValue, viewport, plotRect);

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

function buildTicks(min: number, max: number, count: number): number[] {
  if (count <= 1 || max === min) {
    return [min];
  }
  return Array.from({ length: count }, (_, index) => min + (index / (count - 1)) * (max - min));
}

function formatTick(value: number): string {
  if (Math.abs(value) >= 1_000) {
    return Math.round(value).toString();
  }
  return value.toFixed(2).replace(/\.00$/, "");
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
