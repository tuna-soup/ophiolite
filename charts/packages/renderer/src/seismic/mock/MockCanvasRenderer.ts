import type {
  CursorProbe,
  DisplayTransform,
  Horizon,
  RenderFrame,
  SectionHorizonLineStyle,
  SectionHorizonOverlay,
  SectionWellOverlay,
  SectionScalarOverlay,
  SectionScalarOverlayColorMap,
  SectionPayload
} from "@ophiolite/charts-data-models";
import type { RendererAdapter } from "../adapter";
import {
  buildOverlaySpatialIndex,
  createBaseRenderState,
  createOverlayRenderState,
  diffRenderStates,
  prepareHeatmapData,
  prepareWiggleData,
  type BaseRenderState,
  type OverlayRenderState,
  type PreparedHeatmapData,
  type PreparedWiggleData
} from "./renderModel";
import {
  canUseOffscreenWorkerRenderer,
  cloneOverlayForWorker,
  cloneSectionForWorker,
  type WorkerBaseStatePayload,
  type WorkerOutgoingMessage
} from "./workerProtocol";
import { getPlotRect, sampleIndexToScreenY, sampleValueToScreenY, traceIndexToScreenX } from "./sectionTransforms";
import { mapCoordinateToPlotX, type PlotRect } from "./wiggleGeometry";

interface GLResources {
  heatmapProgram: WebGLProgram;
  geometryProgram: WebGLProgram;
  heatmapQuadBuffer: WebGLBuffer;
  wiggleLineBuffer: WebGLBuffer;
  wiggleFillBuffer: WebGLBuffer;
  amplitudeTexture: WebGLTexture;
  secondaryAmplitudeTexture: WebGLTexture;
  overlayTexture: WebGLTexture;
  lutTexture: WebGLTexture;
  lineVertexCount: number;
  fillVertexCount: number;
}

interface PlotPalette {
  shellBackground: string;
  plotBackground: string;
  traceColor: string;
  fillColor: string;
  guideColor: string;
  axisStroke: string;
  axisLabel: string;
  metaText: string;
}

const scalarOverlayImageCache = new WeakMap<
  SectionScalarOverlay,
  {
    key: string;
    canvas: HTMLCanvasElement;
  }
>();

export class MockCanvasRenderer implements RendererAdapter {
  private host: HTMLDivElement | null = null;
  private baseCanvas: HTMLCanvasElement | null = null;
  private overlayCanvas: HTMLCanvasElement | null = null;
  private baseContext2d: CanvasRenderingContext2D | null = null;
  private overlayContext: CanvasRenderingContext2D | null = null;
  private gl: WebGL2RenderingContext | null = null;
  private resources: GLResources | null = null;
  private lastBaseState: BaseRenderState | null = null;
  private lastOverlayState: OverlayRenderState | null = null;
  private preparedHeatmap: PreparedHeatmapData | null = null;
  private preparedWiggles: PreparedWiggleData | null = null;
  private lastUploadedSection: SectionPayload | null = null;
  private lastUploadedSecondarySection: SectionPayload | null = null;
  private lastUploadedOverlay: RenderFrame["state"]["overlay"] = null;
  private worker: Worker | null = null;
  private workerMode = false;
  private workerReady = false;
  private workerInitTimeout: number | null = null;

  mount(container: HTMLElement): void {
    this.host = document.createElement("div");
    this.host.style.position = "relative";
    this.host.style.width = "100%";
    this.host.style.height = "100%";

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

    this.host.append(this.baseCanvas, this.overlayCanvas);
    container.replaceChildren(this.host);
    this.overlayContext = this.overlayCanvas.getContext("2d");

    if (canUseOffscreenWorkerRenderer(this.baseCanvas)) {
      this.startWorkerRenderer();
    } else {
      this.initLocalBaseRenderer();
    }
  }

  render(frame: RenderFrame): void {
    if (!this.baseCanvas || !this.overlayCanvas || !this.overlayContext || !this.host) {
      return;
    }

    const { width, height } = this.ensureCanvasSize();
    const plotRect = getPlotRect(width, height);
    const nextBaseState = createBaseRenderState(frame, plotRect, width, height);
    const nextOverlayState = createOverlayRenderState(frame, plotRect, width, height);
    if (
      nextBaseState.comparisonMode === "split" &&
      nextBaseState.displayTransform.renderMode === "heatmap" &&
      nextBaseState.secondarySection &&
      this.workerMode
    ) {
      this.fallbackToLocalRenderer();
    }
    const invalidation = diffRenderStates(this.lastBaseState, nextBaseState, this.lastOverlayState, nextOverlayState);

    if (invalidation.baseChanged) {
      if (this.workerMode && this.worker) {
        this.renderBaseWorker(nextBaseState, invalidation);
      } else if (this.gl && this.resources) {
        this.renderBaseWebGl(nextBaseState, invalidation);
      } else if (this.baseContext2d) {
        this.renderBaseCanvas(nextBaseState);
      }
    }

    if (invalidation.overlayNeedsDraw || invalidation.baseChanged) {
      this.renderOverlay(nextOverlayState);
    }

    this.lastBaseState = nextBaseState;
    this.lastOverlayState = nextOverlayState;
  }

  dispose(): void {
    if (this.worker) {
      this.worker.postMessage({ type: "dispose" });
      this.worker.terminate();
      this.worker = null;
    }
    if (this.workerInitTimeout !== null) {
      window.clearTimeout(this.workerInitTimeout);
      this.workerInitTimeout = null;
    }

    if (this.gl && this.resources) {
      const { gl, resources } = this;
      gl.deleteProgram(resources.heatmapProgram);
      gl.deleteProgram(resources.geometryProgram);
      gl.deleteBuffer(resources.heatmapQuadBuffer);
      gl.deleteBuffer(resources.wiggleLineBuffer);
      gl.deleteBuffer(resources.wiggleFillBuffer);
      gl.deleteTexture(resources.amplitudeTexture);
      gl.deleteTexture(resources.secondaryAmplitudeTexture);
      gl.deleteTexture(resources.overlayTexture);
      gl.deleteTexture(resources.lutTexture);
    }

    this.host?.remove();
    this.host = null;
    this.baseCanvas = null;
    this.overlayCanvas = null;
    this.baseContext2d = null;
    this.overlayContext = null;
    this.gl = null;
    this.resources = null;
    this.lastBaseState = null;
    this.lastOverlayState = null;
    this.preparedHeatmap = null;
    this.preparedWiggles = null;
    this.lastUploadedSection = null;
    this.lastUploadedSecondarySection = null;
    this.lastUploadedOverlay = null;
    this.workerMode = false;
    this.workerReady = false;
  }

  private ensureCanvasSize(): { width: number; height: number } {
    const width = Math.max(1, Math.round(this.host?.clientWidth || 1));
    const height = Math.max(1, Math.round(this.host?.clientHeight || 1));

    for (const canvas of [this.baseCanvas, this.overlayCanvas]) {
      if (canvas && (canvas.width !== width || canvas.height !== height)) {
        canvas.width = width;
        canvas.height = height;
      }
    }

    return { width, height };
  }

  private startWorkerRenderer(): void {
    if (!this.baseCanvas) {
      return;
    }

    try {
      this.setBaseRendererKind("worker-pending");
      const { width, height } = this.ensureCanvasSize();
      const plotRect = getPlotRect(width, height);
      const offscreen = this.baseCanvas.transferControlToOffscreen();
      this.worker = new Worker(new URL("./baseRenderWorker.ts", import.meta.url), {
        type: "module"
      });
      this.worker.onmessage = (event: MessageEvent<WorkerOutgoingMessage>) => this.handleWorkerMessage(event.data);
      this.worker.onerror = (event) => {
        console.error(event);
        this.fallbackToLocalRenderer();
      };
      this.worker.postMessage(
        {
          type: "init",
          canvas: offscreen,
          state: this.createWorkerState(width, height, plotRect)
        },
        [offscreen]
      );
      this.workerMode = true;
      this.workerInitTimeout = window.setTimeout(() => {
        if (!this.workerReady) {
          this.fallbackToLocalRenderer();
        }
      }, 750);
    } catch (error) {
      console.error(error);
      this.fallbackToLocalRenderer();
    }
  }

  private initLocalBaseRenderer(): void {
    if (!this.baseCanvas) {
      return;
    }

    this.gl = this.baseCanvas.getContext("webgl2", {
      antialias: true,
      alpha: true,
      premultipliedAlpha: false
    });
    if (this.gl) {
      this.resources = createGlResources(this.gl);
      this.baseContext2d = null;
      this.setBaseRendererKind("local-webgl");
      return;
    }

    this.baseContext2d = this.baseCanvas.getContext("2d");
    this.setBaseRendererKind("local-canvas");
  }

  private renderBaseWorker(baseState: BaseRenderState, invalidation: ReturnType<typeof diffRenderStates>): void {
    if (!this.worker) {
      return;
    }

    if (invalidation.sizeChanged) {
      this.worker.postMessage({
        type: "resize",
        state: this.createWorkerState(baseState.width, baseState.height, baseState.plotRect)
      });
    }

    if (invalidation.dataChanged && baseState.section) {
      const sectionClone = cloneSectionForWorker(baseState.section);
      this.worker.postMessage(
        {
          type: "setSection",
          section: sectionClone.payload
        },
        sectionClone.transfer
      );
    }

    if ((invalidation.dataChanged || invalidation.overlayChanged) && baseState.section) {
      const overlayClone = cloneOverlayForWorker(baseState.overlay);
      this.worker.postMessage(
        {
          type: "setOverlay",
          overlay: overlayClone.payload
        },
        overlayClone.transfer
      );
    }

    if ((invalidation.dataChanged || invalidation.viewportChanged) && baseState.viewport) {
      this.worker.postMessage({
        type: "setViewport",
        viewport: baseState.viewport
      });
    }

    if (invalidation.dataChanged || invalidation.styleChanged) {
      this.worker.postMessage({
        type: "setDisplayTransform",
        displayTransform: baseState.displayTransform
      });
    }
  }

  private handleWorkerMessage(message: WorkerOutgoingMessage): void {
    switch (message.type) {
      case "ready":
        this.workerReady = message.webgl2;
        if (this.workerInitTimeout !== null) {
          window.clearTimeout(this.workerInitTimeout);
          this.workerInitTimeout = null;
        }
        if (!message.webgl2) {
          this.fallbackToLocalRenderer();
        } else {
          this.setBaseRendererKind("worker-webgl");
        }
        break;
      case "error":
        console.error(message.message);
        this.fallbackToLocalRenderer();
        break;
      case "frameRendered":
        break;
    }
  }

  private fallbackToLocalRenderer(): void {
    if (!this.host || !this.overlayCanvas) {
      return;
    }

    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    if (this.workerInitTimeout !== null) {
      window.clearTimeout(this.workerInitTimeout);
      this.workerInitTimeout = null;
    }

    this.workerMode = false;
    this.workerReady = false;
    this.gl = null;
    this.resources = null;

    const replacement = document.createElement("canvas");
    replacement.style.position = "absolute";
    replacement.style.inset = "0";
    replacement.style.width = "100%";
    replacement.style.height = "100%";
    this.host.insertBefore(replacement, this.overlayCanvas);
    this.baseCanvas?.remove();
    this.baseCanvas = replacement;
    this.ensureCanvasSize();
    this.initLocalBaseRenderer();

    if (this.lastBaseState) {
      if (this.gl && this.resources) {
        this.renderBaseWebGl(this.lastBaseState, {
          dataChanged: true,
          viewportChanged: true,
          styleChanged: true,
          selectionChanged: true,
          overlayChanged: true,
          sizeChanged: true,
          baseChanged: true,
          overlayNeedsDraw: true
        });
      } else if (this.baseContext2d) {
        this.renderBaseCanvas(this.lastBaseState);
      }
    }
  }

  private createWorkerState(width: number, height: number, plotRect: PlotRect): WorkerBaseStatePayload {
    return {
      width,
      height,
      plotRect
    };
  }

  private setBaseRendererKind(kind: string): void {
    this.host?.setAttribute("data-base-renderer", kind);
  }

  private renderBaseWebGl(baseState: BaseRenderState, invalidation: ReturnType<typeof diffRenderStates>): void {
    if (!this.gl || !this.resources || !this.baseCanvas) {
      return;
    }

    const { gl, resources } = this;
    gl.viewport(0, 0, baseState.width, baseState.height);

    if (!baseState.section || !baseState.viewport) {
      clearGl(gl, [0.016, 0.075, 0.114, 1]);
      return;
    }

    if (invalidation.dataChanged || this.lastUploadedSection !== baseState.section) {
      uploadAmplitudeTexture(gl, resources.amplitudeTexture, baseState.section);
      this.lastUploadedSection = baseState.section;
    }

    if (baseState.secondarySection && this.lastUploadedSecondarySection !== baseState.secondarySection) {
      uploadAmplitudeTexture(gl, resources.secondaryAmplitudeTexture, baseState.secondarySection);
      this.lastUploadedSecondarySection = baseState.secondarySection;
    } else if (!baseState.secondarySection) {
      this.lastUploadedSecondarySection = null;
    }

    if ((invalidation.dataChanged || invalidation.overlayChanged) && baseState.overlay) {
      uploadOverlayTexture(gl, resources.overlayTexture, baseState.overlay);
      this.lastUploadedOverlay = baseState.overlay;
    } else if (!baseState.overlay) {
      this.lastUploadedOverlay = null;
    }

    if (invalidation.styleChanged || invalidation.dataChanged || invalidation.viewportChanged || invalidation.overlayChanged) {
      if (baseState.displayTransform.renderMode === "heatmap") {
        this.preparedHeatmap = prepareHeatmapData(
          baseState.section,
          baseState.viewport,
          baseState.displayTransform,
          baseState.overlay,
          baseState.comparisonMode === "split" ? baseState.secondarySection : null
        );
        uploadLutTexture(gl, resources.lutTexture, this.preparedHeatmap.lut);
      } else {
        this.preparedWiggles = prepareWiggleData(
          baseState.section,
          baseState.viewport,
          baseState.displayTransform,
          baseState.plotRect,
          baseState.width,
          baseState.height
        );
        gl.bindBuffer(gl.ARRAY_BUFFER, resources.wiggleLineBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, this.preparedWiggles.lineVertices, gl.DYNAMIC_DRAW);
        gl.bindBuffer(gl.ARRAY_BUFFER, resources.wiggleFillBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, this.preparedWiggles.fillVertices, gl.DYNAMIC_DRAW);
        resources.lineVertexCount = this.preparedWiggles.lineVertices.length / 2;
        resources.fillVertexCount = this.preparedWiggles.fillVertices.length / 2;
      }
    }

    if (invalidation.sizeChanged || invalidation.viewportChanged || invalidation.styleChanged) {
      gl.bindBuffer(gl.ARRAY_BUFFER, resources.heatmapQuadBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, buildPlotQuadVertices(baseState.plotRect, baseState.width, baseState.height), gl.DYNAMIC_DRAW);
    }

    const palette = paletteForMode(baseState.displayTransform.renderMode);
    clearGl(gl, hexToRgbaArray(palette.shellBackground));
    clearPlotGl(gl, baseState.plotRect, baseState.height, hexToRgbaArray(palette.plotBackground));

    if (baseState.displayTransform.renderMode === "heatmap" && this.preparedHeatmap) {
      const splitHeatmapEnabled =
        baseState.comparisonMode === "split" && Boolean(baseState.secondarySection);
      if (splitHeatmapEnabled && baseState.secondarySection) {
        const splitX = Math.round(baseState.plotRect.x + baseState.plotRect.width * baseState.splitPosition);
        drawHeatmapGl(gl, resources, baseState, this.preparedHeatmap, resources.amplitudeTexture, {
          x: baseState.plotRect.x,
          width: Math.max(0, splitX - baseState.plotRect.x)
        });
        drawHeatmapGl(
          gl,
          resources,
          { ...baseState, section: baseState.secondarySection, overlay: null },
          this.preparedHeatmap,
          resources.secondaryAmplitudeTexture,
          {
            x: splitX,
            width: Math.max(0, baseState.plotRect.x + baseState.plotRect.width - splitX)
          }
        );
      } else {
        drawHeatmapGl(gl, resources, baseState, this.preparedHeatmap, resources.amplitudeTexture);
      }
      return;
    }

    if (baseState.displayTransform.renderMode === "wiggle" && this.preparedWiggles) {
      drawWigglesGl(gl, resources, palette);
    }
  }

  private renderBaseCanvas(baseState: BaseRenderState): void {
    if (!this.baseContext2d) {
      return;
    }

    if (!baseState.section || !baseState.viewport) {
      drawEmptyState(this.baseContext2d, baseState.width, baseState.height);
      return;
    }

    drawBaseLayer2d(
      this.baseContext2d,
      baseState.width,
      baseState.height,
      baseState.section,
      baseState.secondarySection,
      baseState.viewport,
      baseState.displayTransform,
      baseState.overlay,
      baseState.comparisonMode,
      baseState.splitPosition,
      baseState.plotRect,
      paletteForMode(baseState.displayTransform.renderMode)
    );
  }

  private renderOverlay(overlayState: OverlayRenderState): void {
    if (!this.overlayContext) {
      return;
    }

    const ctx = this.overlayContext;
    if (this.host) {
      this.host.style.cursor = cursorForInteractionState(overlayState.interactions);
    }
    ctx.clearRect(0, 0, overlayState.width, overlayState.height);

    if (!overlayState.section || !overlayState.viewport) {
      drawEmptyState(ctx, overlayState.width, overlayState.height);
      return;
    }

    const palette = paletteForMode(overlayState.displayTransform.renderMode);
    for (const overlay of overlayState.sectionScalarOverlays) {
      drawSectionScalarOverlay(
        ctx,
        overlayState.viewport,
        overlayState.plotRect,
        overlay,
        overlayState.comparisonMode,
        overlayState.splitPosition,
        Boolean(overlayState.secondarySection)
      );
    }

    if (overlayState.displayTransform.renderMode === "wiggle") {
      drawHorizontalGuides(
        ctx,
        overlayState.viewport.sampleStart,
        overlayState.viewport.sampleEnd,
        overlayState.plotRect,
        palette.guideColor
      );
    }

    drawAxes(
      ctx,
      overlayState.section,
      overlayState.viewport.traceStart,
      overlayState.viewport.traceEnd,
      overlayState.viewport.sampleStart,
      overlayState.viewport.sampleEnd,
      overlayState.plotRect,
      overlayState.displayTransform.renderMode,
      palette
    );

    for (const overlay of overlayState.sectionHorizonOverlays) {
      drawSectionHorizonOverlay(
        ctx,
        overlayState.section,
        overlayState.viewport,
        overlayState.displayTransform.renderMode,
        overlayState.plotRect,
        overlay
      );
    }

    for (const overlay of overlayState.sectionWellOverlays) {
      drawSectionWellOverlay(
        ctx,
        overlayState.section,
        overlayState.viewport,
        overlayState.displayTransform.renderMode,
        overlayState.plotRect,
        overlay
      );
    }

    const spatialIndex = buildOverlaySpatialIndex(
      overlayState.section,
      overlayState.viewport,
      overlayState.displayTransform.renderMode,
      overlayState.plotRect,
      overlayState.horizons
    );

    for (const horizon of overlayState.horizons) {
      const activeAnchorIds = new Set(
        spatialIndex.points.filter((point) => point.horizonId === horizon.id).map((point) => point.anchorId)
      );
      drawHorizon(
        ctx,
        overlayState.section,
        overlayState.viewport,
        overlayState.displayTransform.renderMode,
        overlayState.plotRect,
        horizon,
        horizon.id === overlayState.activeHorizonId,
        activeAnchorIds
      );
    }

    if (overlayState.probe && overlayState.interactions.modifiers.includes("crosshair")) {
      drawProbe(ctx, overlayState.plotRect, overlayState.probe);
    }

    if (overlayState.interactions.session?.kind === "zoomRect") {
      drawZoomRectOverlay(ctx, overlayState.plotRect, overlayState.interactions.session);
    }
  }
}

function createGlResources(gl: WebGL2RenderingContext): GLResources {
  const heatmapProgram = createProgram(gl, HEATMAP_VERTEX_SHADER, HEATMAP_FRAGMENT_SHADER);
  const geometryProgram = createProgram(gl, GEOMETRY_VERTEX_SHADER, GEOMETRY_FRAGMENT_SHADER);
  const heatmapQuadBuffer = createBuffer(gl);
  const wiggleLineBuffer = createBuffer(gl);
  const wiggleFillBuffer = createBuffer(gl);
  const amplitudeTexture = createTexture(gl);
  const secondaryAmplitudeTexture = createTexture(gl);
  const overlayTexture = createTexture(gl);
  const lutTexture = createTexture(gl);

  return {
    heatmapProgram,
    geometryProgram,
    heatmapQuadBuffer,
    wiggleLineBuffer,
    wiggleFillBuffer,
    amplitudeTexture,
    secondaryAmplitudeTexture,
    overlayTexture,
    lutTexture,
    lineVertexCount: 0,
    fillVertexCount: 0
  };
}

function drawHeatmapGl(
  gl: WebGL2RenderingContext,
  resources: GLResources,
  baseState: BaseRenderState,
  prepared: PreparedHeatmapData,
  amplitudeTexture: WebGLTexture,
  scissorOverride?: { x: number; width: number }
): void {
  if (!baseState.viewport || !baseState.section) {
    return;
  }

  const plotRect = baseState.plotRect;
  const scissorX = scissorOverride?.x ?? plotRect.x;
  const scissorWidth = scissorOverride?.width ?? plotRect.width;
  if (scissorWidth <= 0) {
    return;
  }
  gl.enable(gl.SCISSOR_TEST);
  gl.scissor(scissorX, baseState.height - plotRect.y - plotRect.height, scissorWidth, plotRect.height);

  gl.useProgram(resources.heatmapProgram);
  gl.bindBuffer(gl.ARRAY_BUFFER, resources.heatmapQuadBuffer);

  const positionLocation = gl.getAttribLocation(resources.heatmapProgram, "aPosition");
  const uvLocation = gl.getAttribLocation(resources.heatmapProgram, "aUv");
  gl.enableVertexAttribArray(positionLocation);
  gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 16, 0);
  gl.enableVertexAttribArray(uvLocation);
  gl.vertexAttribPointer(uvLocation, 2, gl.FLOAT, false, 16, 8);

  bindTexture(gl, amplitudeTexture, 0);
  bindTexture(gl, resources.lutTexture, 1);
  bindTexture(gl, resources.overlayTexture, 2);

  gl.uniform1i(gl.getUniformLocation(resources.heatmapProgram, "uAmplitude"), 0);
  gl.uniform1i(gl.getUniformLocation(resources.heatmapProgram, "uLut"), 1);
  gl.uniform1i(gl.getUniformLocation(resources.heatmapProgram, "uOverlay"), 2);
  gl.uniform2f(
    gl.getUniformLocation(resources.heatmapProgram, "uSectionSize"),
    baseState.section.dimensions.samples,
    baseState.section.dimensions.traces
  );
  gl.uniform4f(
    gl.getUniformLocation(resources.heatmapProgram, "uViewport"),
    baseState.viewport.traceStart,
    baseState.viewport.traceEnd,
    baseState.viewport.sampleStart,
    baseState.viewport.sampleEnd
  );
  gl.uniform1f(gl.getUniformLocation(resources.heatmapProgram, "uGain"), baseState.displayTransform.gain);
  gl.uniform1f(gl.getUniformLocation(resources.heatmapProgram, "uClipMin"), prepared.clipMin);
  gl.uniform1f(gl.getUniformLocation(resources.heatmapProgram, "uClipMax"), prepared.clipMax);
  gl.uniform1f(gl.getUniformLocation(resources.heatmapProgram, "uSymmetricExtent"), prepared.symmetricExtent);
  gl.uniform1f(
    gl.getUniformLocation(resources.heatmapProgram, "uUseDiverging"),
    baseState.displayTransform.colormap === "red-white-blue" ? 1 : 0
  );
  gl.uniform1f(
    gl.getUniformLocation(resources.heatmapProgram, "uPolaritySign"),
    baseState.displayTransform.polarity === "reversed" ? -1 : 1
  );
  gl.uniform1f(gl.getUniformLocation(resources.heatmapProgram, "uOverlayEnabled"), prepared.overlayEnabled ? 1 : 0);
  gl.uniform1f(gl.getUniformLocation(resources.heatmapProgram, "uOverlayOpacity"), prepared.overlayOpacity);

  gl.drawArrays(gl.TRIANGLES, 0, 6);
  gl.disable(gl.SCISSOR_TEST);
}

function drawWigglesGl(gl: WebGL2RenderingContext, resources: GLResources, palette: PlotPalette): void {
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.useProgram(resources.geometryProgram);

  const positionLocation = gl.getAttribLocation(resources.geometryProgram, "aPosition");
  const colorLocation = gl.getUniformLocation(resources.geometryProgram, "uColor");

  if (resources.fillVertexCount > 0) {
    gl.bindBuffer(gl.ARRAY_BUFFER, resources.wiggleFillBuffer);
    gl.enableVertexAttribArray(positionLocation);
    gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 8, 0);
    gl.uniform4fv(colorLocation, hexToRgbaArray(palette.fillColor));
    gl.drawArrays(gl.TRIANGLES, 0, resources.fillVertexCount);
  }

  if (resources.lineVertexCount > 0) {
    gl.bindBuffer(gl.ARRAY_BUFFER, resources.wiggleLineBuffer);
    gl.enableVertexAttribArray(positionLocation);
    gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 8, 0);
    gl.uniform4fv(colorLocation, hexToRgbaArray(palette.traceColor));
    gl.drawArrays(gl.LINES, 0, resources.lineVertexCount);
  }

  gl.disable(gl.BLEND);
}

function drawEmptyState(ctx: CanvasRenderingContext2D, width: number, height: number): void {
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = "#f2f6f8";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#435c6b";
  ctx.font = "18px sans-serif";
  ctx.fillText("No section loaded", 32, 48);
}

function drawBaseLayer2d(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  section: SectionPayload,
  secondarySection: SectionPayload | null,
  viewport: RenderFrame["state"]["viewport"] & object,
  displayTransform: DisplayTransform,
  overlay: RenderFrame["state"]["overlay"],
  comparisonMode: RenderFrame["state"]["comparisonMode"],
  splitPosition: number,
  plotRect: PlotRect,
  palette: PlotPalette
): void {
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = palette.shellBackground;
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = palette.plotBackground;
  ctx.fillRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);

  const splitHeatmapEnabled =
    comparisonMode === "split" && displayTransform.renderMode === "heatmap" && Boolean(secondarySection);

  if (displayTransform.renderMode === "wiggle") {
    drawHorizontalGuides(ctx, viewport.sampleStart, viewport.sampleEnd, plotRect, palette.guideColor);
  }

  if (displayTransform.renderMode === "wiggle") {
    drawWiggles2d(
      ctx,
      section,
      viewport.traceStart,
      viewport.traceEnd,
      viewport.sampleStart,
      viewport.sampleEnd,
      displayTransform,
      plotRect,
      palette.traceColor,
      width,
      height
    );
  } else if (splitHeatmapEnabled && secondarySection) {
    drawSplitHeatmap2d(
      ctx,
      section,
      secondarySection,
      viewport.traceStart,
      viewport.traceEnd,
      viewport.sampleStart,
      viewport.sampleEnd,
      displayTransform,
      plotRect,
      splitPosition
    );
  } else {
    drawHeatmap2d(
      ctx,
      section,
      viewport.traceStart,
      viewport.traceEnd,
      viewport.sampleStart,
      viewport.sampleEnd,
      displayTransform,
      plotRect
    );
  }

  if (overlay && displayTransform.renderMode === "heatmap" && !splitHeatmapEnabled) {
    drawOccupancyOverlay(
      ctx,
      overlay.values,
      overlay.height,
      overlay.opacity ?? 0.35,
      viewport.traceStart,
      viewport.traceEnd,
      viewport.sampleStart,
      viewport.sampleEnd,
      plotRect
    );
  }
}

function drawSplitHeatmap2d(
  ctx: CanvasRenderingContext2D,
  primarySection: SectionPayload,
  secondarySection: SectionPayload,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  displayTransform: DisplayTransform,
  plotRect: PlotRect,
  splitPosition: number
): void {
  const { clipMin, clipMax } = prepareHeatmapData(
    primarySection,
    { traceStart, traceEnd, sampleStart, sampleEnd },
    displayTransform,
    null,
    secondarySection
  );
  const splitX = plotRect.x + plotRect.width * splitPosition;

  ctx.save();
  ctx.beginPath();
  ctx.rect(plotRect.x, plotRect.y, Math.max(0, splitX - plotRect.x), plotRect.height);
  ctx.clip();
  drawHeatmap2d(
    ctx,
    primarySection,
    traceStart,
    traceEnd,
    sampleStart,
    sampleEnd,
    displayTransform,
    plotRect,
    clipMin,
    clipMax
  );
  ctx.restore();

  ctx.save();
  ctx.beginPath();
  ctx.rect(splitX, plotRect.y, Math.max(0, plotRect.x + plotRect.width - splitX), plotRect.height);
  ctx.clip();
  drawHeatmap2d(
    ctx,
    secondarySection,
    traceStart,
    traceEnd,
    sampleStart,
    sampleEnd,
    displayTransform,
    plotRect,
    clipMin,
    clipMax
  );
  ctx.restore();
}

function drawHeatmap2d(
  ctx: CanvasRenderingContext2D,
  section: SectionPayload,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  displayTransform: DisplayTransform,
  plotRect: PlotRect,
  forcedClipMin?: number,
  forcedClipMax?: number
): void {
  const traceCount = traceEnd - traceStart;
  const sampleCount = sampleEnd - sampleStart;
  const image = ctx.createImageData(traceCount, sampleCount);
  const amplitudes = section.amplitudes;
  const samples = section.dimensions.samples;

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const value = amplitudes[trace * samples + sample] * displayTransform.gain;
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
  }

  const clipMin = forcedClipMin ?? displayTransform.clipMin ?? min;
  const clipMax = forcedClipMax ?? displayTransform.clipMax ?? max;
  const denominator = Math.max(1e-6, clipMax - clipMin);
  const symmetricExtent = Math.max(Math.abs(clipMin), Math.abs(clipMax), 1e-6);

  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const source = amplitudes[trace * samples + sample] * displayTransform.gain;
      const normalized =
        displayTransform.colormap === "red-white-blue"
          ? Math.max(0, Math.min(1, (source / symmetricExtent + 1) / 2))
          : Math.max(0, Math.min(1, (source - clipMin) / denominator));
      const mapped = displayTransform.polarity === "reversed" ? 1 - normalized : normalized;
      const [red, green, blue] = colorFromMap(displayTransform.colormap, mapped);
      const imageIndex = ((sample - sampleStart) * traceCount + (trace - traceStart)) * 4;
      image.data[imageIndex] = red;
      image.data[imageIndex + 1] = green;
      image.data[imageIndex + 2] = blue;
      image.data[imageIndex + 3] = 255;
    }
  }

  const offscreen = document.createElement("canvas");
  offscreen.width = traceCount;
  offscreen.height = sampleCount;
  const offscreenContext = offscreen.getContext("2d");
  if (!offscreenContext) {
    return;
  }
  offscreenContext.putImageData(image, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(offscreen, plotRect.x, plotRect.y, plotRect.width, plotRect.height);
}

function drawWiggles2d(
  ctx: CanvasRenderingContext2D,
  section: SectionPayload,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  displayTransform: DisplayTransform,
  plotRect: PlotRect,
  traceColor: string,
  canvasWidth: number,
  canvasHeight: number
): void {
  const prepared = prepareWiggleData(
    section,
    {
      traceStart,
      traceEnd,
      sampleStart,
      sampleEnd
    },
    displayTransform,
    plotRect,
    canvasWidth,
    canvasHeight
  );

  ctx.save();
  ctx.strokeStyle = traceColor;
  ctx.lineWidth = 1;
  ctx.fillStyle = traceColor;

  for (let index = 0; index < prepared.fillVertices.length; index += 6) {
    const ax = clipToCanvasX(prepared.fillVertices[index], canvasWidth);
    const ay = clipToCanvasY(prepared.fillVertices[index + 1], canvasHeight);
    const bx = clipToCanvasX(prepared.fillVertices[index + 2], canvasWidth);
    const by = clipToCanvasY(prepared.fillVertices[index + 3], canvasHeight);
    const cx = clipToCanvasX(prepared.fillVertices[index + 4], canvasWidth);
    const cy = clipToCanvasY(prepared.fillVertices[index + 5], canvasHeight);
    ctx.beginPath();
    ctx.moveTo(ax, ay);
    ctx.lineTo(bx, by);
    ctx.lineTo(cx, cy);
    ctx.closePath();
    ctx.fill();
  }

  for (let index = 0; index < prepared.lineVertices.length; index += 4) {
    ctx.beginPath();
    ctx.moveTo(
      clipToCanvasX(prepared.lineVertices[index], canvasWidth),
      clipToCanvasY(prepared.lineVertices[index + 1], canvasHeight)
    );
    ctx.lineTo(
      clipToCanvasX(prepared.lineVertices[index + 2], canvasWidth),
      clipToCanvasY(prepared.lineVertices[index + 3], canvasHeight)
    );
    ctx.stroke();
  }

  ctx.restore();
}

function drawOccupancyOverlay(
  ctx: CanvasRenderingContext2D,
  values: Uint8Array,
  height: number,
  opacity: number,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  plotRect: PlotRect
): void {
  ctx.save();
  ctx.globalAlpha = opacity;
  ctx.fillStyle = "#f97316";
  const visibleTraces = Math.max(1, traceEnd - traceStart);
  const visibleSamples = Math.max(1, sampleEnd - sampleStart);

  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      if (values[trace * height + sample] === 0) {
        continue;
      }
      const x = plotRect.x + ((trace - traceStart) / visibleTraces) * plotRect.width;
      const y = plotRect.y + ((sample - sampleStart) / visibleSamples) * plotRect.height;
      ctx.fillRect(
        x,
        y,
        Math.max(1, plotRect.width / visibleTraces),
        Math.max(1, plotRect.height / visibleSamples)
      );
    }
  }

  ctx.restore();
}

function drawSectionScalarOverlay(
  ctx: CanvasRenderingContext2D,
  viewport: RenderFrame["state"]["viewport"] & object,
  plotRect: PlotRect,
  overlay: SectionScalarOverlay,
  comparisonMode: RenderFrame["state"]["comparisonMode"],
  splitPosition: number,
  hasSecondarySection: boolean
): void {
  if (overlay.opacity !== undefined && overlay.opacity <= 0) {
    return;
  }

  const raster = getScalarOverlayRaster(
    overlay,
    viewport.traceStart,
    viewport.traceEnd,
    viewport.sampleStart,
    viewport.sampleEnd
  );
  if (!raster) {
    return;
  }

  ctx.save();
  ctx.beginPath();
  if (comparisonMode === "split" && hasSecondarySection) {
    const splitX = plotRect.x + plotRect.width * splitPosition;
    ctx.rect(plotRect.x, plotRect.y, Math.max(0, splitX - plotRect.x), plotRect.height);
  } else {
    ctx.rect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);
  }
  ctx.clip();
  ctx.globalAlpha = clamp01(overlay.opacity ?? 0.55);
  ctx.imageSmoothingEnabled = true;
  ctx.drawImage(raster, plotRect.x, plotRect.y, plotRect.width, plotRect.height);
  ctx.restore();
}

function getScalarOverlayRaster(
  overlay: SectionScalarOverlay,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number
): HTMLCanvasElement | null {
  const traceCount = Math.max(1, traceEnd - traceStart);
  const sampleCount = Math.max(1, sampleEnd - sampleStart);
  const colorMap = overlay.colorMap ?? "turbo";
  const rangeKey = overlay.valueRange ? `${overlay.valueRange.min}:${overlay.valueRange.max}` : "auto";
  const noDataKey = overlay.noDataValue ?? "nan";
  const cacheKey = `${traceStart}:${traceEnd}:${sampleStart}:${sampleEnd}:${colorMap}:${rangeKey}:${noDataKey}`;
  const cached = scalarOverlayImageCache.get(overlay);
  if (cached?.key === cacheKey) {
    return cached.canvas;
  }

  const canvas = document.createElement("canvas");
  canvas.width = traceCount;
  canvas.height = sampleCount;
  const context = canvas.getContext("2d");
  if (!context) {
    return null;
  }

  const image = context.createImageData(traceCount, sampleCount);
  const { min, max } = resolveScalarOverlayRange(overlay, traceStart, traceEnd, sampleStart, sampleEnd);
  const denominator = Math.max(1e-6, max - min);

  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const value = overlay.values[trace * overlay.height + sample];
      if (!isScalarOverlayValueVisible(value, overlay.noDataValue)) {
        continue;
      }
      const normalized = clamp01((value - min) / denominator);
      const [red, green, blue] = scalarColorFromMap(colorMap, normalized);
      const imageIndex = ((sample - sampleStart) * traceCount + (trace - traceStart)) * 4;
      image.data[imageIndex] = red;
      image.data[imageIndex + 1] = green;
      image.data[imageIndex + 2] = blue;
      image.data[imageIndex + 3] = 255;
    }
  }

  context.putImageData(image, 0, 0);
  scalarOverlayImageCache.set(overlay, {
    key: cacheKey,
    canvas
  });
  return canvas;
}

function resolveScalarOverlayRange(
  overlay: SectionScalarOverlay,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number
): { min: number; max: number } {
  if (overlay.valueRange) {
    return overlay.valueRange;
  }

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;

  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const value = overlay.values[trace * overlay.height + sample];
      if (!isScalarOverlayValueVisible(value, overlay.noDataValue)) {
        continue;
      }
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
  }

  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return { min: 0, max: 1 };
  }

  if (Math.abs(max - min) < 1e-6) {
    return { min, max: min + 1 };
  }

  return { min, max };
}

function isScalarOverlayValueVisible(value: number, noDataValue?: number): boolean {
  if (!Number.isFinite(value)) {
    return false;
  }
  if (noDataValue === undefined) {
    return true;
  }
  return Math.abs(value - noDataValue) > 1e-6;
}

function drawHorizon(
  ctx: CanvasRenderingContext2D,
  section: SectionPayload,
  viewport: RenderFrame["state"]["viewport"] & object,
  renderMode: DisplayTransform["renderMode"],
  plotRect: PlotRect,
  horizon: Horizon,
  isActive: boolean,
  activeAnchorIds: Set<string>
): void {
  if (horizon.picks.length === 0) {
    return;
  }

  ctx.save();
  ctx.strokeStyle = horizon.color;
  ctx.lineWidth = isActive ? 3 : 2;
  const path = new Path2D();
  let first = true;
  for (const pick of horizon.picks) {
    const x = traceIndexToScreenX(section, viewport, renderMode, plotRect, pick.traceIndex);
    const y = sampleIndexToScreenY(viewport, plotRect, pick.sampleIndex);
    if (first) {
      path.moveTo(x, y);
      first = false;
    } else {
      path.lineTo(x, y);
    }
  }
  ctx.stroke(path);

  if (isActive) {
    ctx.fillStyle = "#ffffff";
    ctx.strokeStyle = horizon.color;
    for (const anchor of horizon.anchors) {
      const x = traceIndexToScreenX(section, viewport, renderMode, plotRect, anchor.traceIndex);
      const y = sampleIndexToScreenY(viewport, plotRect, anchor.sampleIndex);
      ctx.beginPath();
      ctx.arc(x, y, activeAnchorIds.has(anchor.id) ? 4.5 : 4, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
  }
  ctx.restore();
}

function drawSectionHorizonOverlay(
  ctx: CanvasRenderingContext2D,
  section: SectionPayload,
  viewport: RenderFrame["state"]["viewport"] & object,
  renderMode: DisplayTransform["renderMode"],
  plotRect: PlotRect,
  overlay: SectionHorizonOverlay
): void {
  if (overlay.samples.length === 0) {
    return;
  }

  ctx.save();
  ctx.beginPath();
  ctx.rect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);
  ctx.clip();
  ctx.strokeStyle = overlay.color;
  ctx.globalAlpha = clamp01(overlay.opacity ?? 1);
  const lineWidth = Math.max(1, overlay.lineWidth ?? 2);
  ctx.lineWidth = lineWidth;
  ctx.setLineDash(lineDashForStyle(overlay.lineStyle ?? "solid", lineWidth));

  const samples = [...overlay.samples].sort((left, right) => left.traceIndex - right.traceIndex);
  let pathStarted = false;
  let previousTraceIndex: number | null = null;

  for (const sample of samples) {
    const x = traceIndexToScreenX(section, viewport, renderMode, plotRect, sample.traceIndex);
    const y = sampleIndexToScreenY(viewport, plotRect, sample.sampleIndex);
    const discontinuity =
      previousTraceIndex === null ||
      sample.traceIndex <= previousTraceIndex ||
      sample.traceIndex - previousTraceIndex > 1;

    if (discontinuity) {
      if (pathStarted) {
        ctx.stroke();
      }
      ctx.beginPath();
      ctx.moveTo(x, y);
      pathStarted = true;
    } else {
      ctx.lineTo(x, y);
    }

    previousTraceIndex = sample.traceIndex;
  }

  if (pathStarted) {
    ctx.stroke();
  }

  ctx.restore();
}

function drawSectionWellOverlay(
  ctx: CanvasRenderingContext2D,
  section: SectionPayload,
  viewport: RenderFrame["state"]["viewport"] & object,
  renderMode: DisplayTransform["renderMode"],
  plotRect: PlotRect,
  overlay: SectionWellOverlay
): void {
  if (overlay.segments.length === 0) {
    return;
  }

  ctx.save();
  ctx.beginPath();
  ctx.rect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);
  ctx.clip();
  ctx.strokeStyle = overlay.color;
  ctx.fillStyle = overlay.color;
  ctx.globalAlpha = clamp01(overlay.opacity ?? 0.95);
  const lineWidth = Math.max(1, overlay.lineWidth ?? 2.5);
  ctx.lineWidth = lineWidth;
  ctx.setLineDash(lineDashForStyle(overlay.lineStyle ?? "solid", lineWidth));

  let labelPoint: { x: number; y: number } | null = null;

  for (const segment of overlay.segments) {
    const samples = segment.samples
      .map((sample) => {
        const y =
          sample.sampleIndex !== undefined
            ? sampleIndexToScreenY(viewport, plotRect, sample.sampleIndex)
            : sample.sampleValue !== undefined
              ? sampleValueToScreenY(section, viewport, plotRect, sample.sampleValue)
              : null;
        if (y === null) {
          return null;
        }
        return {
          x: traceIndexToScreenX(section, viewport, renderMode, plotRect, sample.traceIndex),
          y
        };
      })
      .filter((sample): sample is { x: number; y: number } => sample !== null);

    if (samples.length < 2) {
      continue;
    }

    ctx.beginPath();
    ctx.moveTo(samples[0]!.x, samples[0]!.y);
    for (let index = 1; index < samples.length; index += 1) {
      ctx.lineTo(samples[index]!.x, samples[index]!.y);
    }
    ctx.stroke();
    labelPoint = samples[samples.length - 1]!;
  }

  if (overlay.name && labelPoint) {
    ctx.save();
    ctx.font = "600 12px sans-serif";
    ctx.textBaseline = "middle";
    ctx.lineWidth = 3;
    ctx.strokeStyle = "rgba(4, 19, 29, 0.82)";
    ctx.strokeText(overlay.name, labelPoint.x + 6, labelPoint.y);
    ctx.fillText(overlay.name, labelPoint.x + 6, labelPoint.y);
    ctx.restore();
  }

  ctx.restore();
}

function drawProbe(ctx: CanvasRenderingContext2D, plotRect: PlotRect, probe: CursorProbe): void {
  ctx.save();
  ctx.strokeStyle = "rgba(255,255,255,0.55)";
  ctx.setLineDash([4, 4]);
  ctx.beginPath();
  ctx.moveTo(plotRect.x, probe.screenY);
  ctx.lineTo(plotRect.x + plotRect.width, probe.screenY);
  ctx.moveTo(probe.screenX, plotRect.y);
  ctx.lineTo(probe.screenX, plotRect.y + plotRect.height);
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.fillStyle = "#ffffff";
  ctx.beginPath();
  ctx.arc(probe.screenX, probe.screenY, 3, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

function lineDashForStyle(style: SectionHorizonLineStyle, lineWidth: number): number[] {
  const width = Math.max(1, lineWidth);
  switch (style) {
    case "dashed":
      return [width * 4, width * 2];
    case "dotted":
      return [width, width * 2];
    default:
      return [];
  }
}

function clamp01(value: number): number {
  return Math.min(Math.max(value, 0), 1);
}

function drawZoomRectOverlay(
  ctx: CanvasRenderingContext2D,
  plotRect: PlotRect,
  session: Extract<RenderFrame["state"]["interactions"]["session"], { kind: "zoomRect" }>
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

  ctx.save();
  ctx.fillStyle = "rgba(180, 214, 232, 0.12)";
  ctx.strokeStyle = "rgba(223, 232, 238, 0.88)";
  ctx.lineWidth = 1;
  ctx.setLineDash([5, 4]);
  ctx.fillRect(left, top, width, height);
  ctx.strokeRect(left + 0.5, top + 0.5, Math.max(0, width - 1), Math.max(0, height - 1));
  ctx.restore();
}

function drawAxes(
  ctx: CanvasRenderingContext2D,
  section: SectionPayload,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  plotRect: PlotRect,
  renderMode: DisplayTransform["renderMode"],
  palette: PlotPalette
): void {
  ctx.save();
  ctx.strokeStyle = palette.axisStroke;
  ctx.lineWidth = 1;
  ctx.strokeRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);

  ctx.fillStyle = palette.axisLabel;
  ctx.font = "12px sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "bottom";

  const coordMin = Math.min(...Array.from(section.horizontalAxis.slice(traceStart, traceEnd)));
  const coordMax = Math.max(...Array.from(section.horizontalAxis.slice(traceStart, traceEnd)));
  const topTicks = buildTickIndices(traceStart, traceEnd, 12);
  const topAxisRows = buildTopAxisRows(section);
  for (const traceIndex of topTicks) {
    const x =
      renderMode === "wiggle"
        ? mapCoordinateToPlotX(section.horizontalAxis[traceIndex], coordMin, coordMax, plotRect)
        : plotRect.x + ((traceIndex - traceStart) / Math.max(1, traceEnd - traceStart - 1)) * plotRect.width;
    ctx.beginPath();
    ctx.moveTo(x, plotRect.y);
    ctx.lineTo(x, plotRect.y - 7);
    ctx.stroke();
    for (const [rowIndex, row] of topAxisRows.entries()) {
      ctx.fillText(formatAxisValue(row.values[traceIndex]!), x, plotRect.y - 10 - rowIndex * 16);
    }
  }

  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  const leftTicks = buildTickIndices(sampleStart, sampleEnd, 14);
  for (const sampleIndex of leftTicks) {
    const ratio = (sampleIndex - sampleStart) / Math.max(1, sampleEnd - sampleStart - 1);
    const y = plotRect.y + ratio * plotRect.height;
    ctx.beginPath();
    ctx.moveTo(plotRect.x, y);
    ctx.lineTo(plotRect.x - 7, y);
    ctx.stroke();
    ctx.fillText(formatAxisValue(section.sampleAxis[sampleIndex]), plotRect.x - 10, y);
  }

  ctx.fillStyle = palette.metaText;
  ctx.font = "13px sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const sectionLabel =
    section.presentation?.title ??
    (isArbitrarySection(section) ? "Arbitrary Section" : `${capitalize(section.axis)}: ${formatAxisValue(section.coordinate.value)}`);
  ctx.fillText(sectionLabel, plotRect.x + plotRect.width / 2, 18);

  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  for (const [rowIndex, row] of topAxisRows.entries()) {
    ctx.fillText(row.label, 8, plotRect.y - 16 - rowIndex * 16);
  }
  ctx.save();
  ctx.translate(18, plotRect.y + plotRect.height / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.textAlign = "center";
  const sampleAxisLabel = section.presentation?.sampleAxisLabel ?? "Sample";
  ctx.fillText(section.units?.sample ? `${sampleAxisLabel} (${section.units.sample})` : sampleAxisLabel, 0, 0);
  ctx.restore();
  ctx.restore();
}

function drawHorizontalGuides(
  ctx: CanvasRenderingContext2D,
  sampleStart: number,
  sampleEnd: number,
  plotRect: PlotRect,
  guideColor: string
): void {
  ctx.save();
  ctx.strokeStyle = guideColor;
  ctx.lineWidth = 1;
  const ticks = buildTickIndices(sampleStart, sampleEnd, 14);
  for (const sampleIndex of ticks) {
    const ratio = (sampleIndex - sampleStart) / Math.max(1, sampleEnd - sampleStart - 1);
    const y = plotRect.y + ratio * plotRect.height;
    ctx.beginPath();
    ctx.moveTo(plotRect.x, y);
    ctx.lineTo(plotRect.x + plotRect.width, y);
    ctx.stroke();
  }
  ctx.restore();
}

function buildTickIndices(start: number, end: number, maxTicks: number): number[] {
  const count = end - start;
  if (count <= 0) {
    return [];
  }

  const tickCount = Math.min(maxTicks, count);
  const ticks = new Set<number>();
  for (let index = 0; index < tickCount; index += 1) {
    const ratio = tickCount === 1 ? 0 : index / (tickCount - 1);
    ticks.add(start + Math.round(ratio * (count - 1)));
  }
  return [...ticks].sort((a, b) => a - b);
}

function buildTopAxisRows(section: SectionPayload): Array<{ label: string; values: Float64Array }> {
  if (section.presentation?.topAxisRows?.length) {
    return section.presentation.topAxisRows;
  }

  if (isArbitrarySection(section) && section.inlineAxis && section.xlineAxis) {
    return [
      { label: "Trace", values: section.horizontalAxis },
      { label: "IL", values: section.inlineAxis },
      { label: "XL", values: section.xlineAxis }
    ];
  }

  return [
    {
      label: section.axis === "inline" ? "Xline" : "Inline",
      values: section.horizontalAxis
    }
  ];
}

function isArbitrarySection(section: SectionPayload): boolean {
  return hasAxisVariation(section.inlineAxis) && hasAxisVariation(section.xlineAxis);
}

function hasAxisVariation(axis: Float64Array | undefined): boolean {
  if (!axis || axis.length < 2) {
    return false;
  }

  const first = axis[0]!;
  for (let index = 1; index < axis.length; index += 1) {
    if (Math.abs(axis[index]! - first) > 1e-6) {
      return true;
    }
  }
  return false;
}

function cursorForInteractionState(interactions: RenderFrame["state"]["interactions"]): string {
  if (interactions.session?.kind === "zoomRect") {
    return "crosshair";
  }

  switch (interactions.primaryMode) {
    case "panZoom":
      return "grab";
    case "zoomRect":
      return "crosshair";
    default:
      return interactions.modifiers.includes("crosshair") ? "crosshair" : "default";
  }
}

function buildPlotQuadVertices(plotRect: PlotRect, width: number, height: number): Float32Array {
  const left = (plotRect.x / width) * 2 - 1;
  const right = ((plotRect.x + plotRect.width) / width) * 2 - 1;
  const top = 1 - (plotRect.y / height) * 2;
  const bottom = 1 - ((plotRect.y + plotRect.height) / height) * 2;

  return new Float32Array([
    left, bottom, 0, 1,
    right, bottom, 1, 1,
    right, top, 1, 0,
    left, bottom, 0, 1,
    right, top, 1, 0,
    left, top, 0, 0
  ]);
}

function clearGl(gl: WebGL2RenderingContext, rgba: [number, number, number, number]): void {
  gl.disable(gl.SCISSOR_TEST);
  gl.clearColor(rgba[0], rgba[1], rgba[2], rgba[3]);
  gl.clear(gl.COLOR_BUFFER_BIT);
}

function clearPlotGl(
  gl: WebGL2RenderingContext,
  plotRect: PlotRect,
  canvasHeight: number,
  rgba: [number, number, number, number]
): void {
  gl.enable(gl.SCISSOR_TEST);
  gl.scissor(plotRect.x, canvasHeight - plotRect.y - plotRect.height, plotRect.width, plotRect.height);
  gl.clearColor(rgba[0], rgba[1], rgba[2], rgba[3]);
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.disable(gl.SCISSOR_TEST);
}

function uploadAmplitudeTexture(gl: WebGL2RenderingContext, texture: WebGLTexture, section: SectionPayload): void {
  bindTexture(gl, texture, 0);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(
    gl.TEXTURE_2D,
    0,
    gl.R32F,
    section.dimensions.samples,
    section.dimensions.traces,
    0,
    gl.RED,
    gl.FLOAT,
    section.amplitudes
  );
}

function uploadOverlayTexture(gl: WebGL2RenderingContext, texture: WebGLTexture, overlay: NonNullable<RenderFrame["state"]["overlay"]>): void {
  bindTexture(gl, texture, 0);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, overlay.height, overlay.width, 0, gl.RED, gl.UNSIGNED_BYTE, overlay.values);
}

function uploadLutTexture(gl: WebGL2RenderingContext, texture: WebGLTexture, lut: Uint8Array): void {
  bindTexture(gl, texture, 0);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 256, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, lut);
}

function bindTexture(gl: WebGL2RenderingContext, texture: WebGLTexture, unit: number): void {
  gl.activeTexture(gl.TEXTURE0 + unit);
  gl.bindTexture(gl.TEXTURE_2D, texture);
}

function createProgram(gl: WebGL2RenderingContext, vertexSource: string, fragmentSource: string): WebGLProgram {
  const program = gl.createProgram();
  const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);

  if (!program || !vertexShader || !fragmentShader) {
    throw new Error("Failed to create WebGL program.");
  }

  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) ?? "Failed to link WebGL program.");
  }

  return program;
}

function compileShader(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) {
    throw new Error("Failed to create shader.");
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const message = gl.getShaderInfoLog(shader) ?? "Failed to compile shader.";
    gl.deleteShader(shader);
    throw new Error(message);
  }

  return shader;
}

function createBuffer(gl: WebGL2RenderingContext): WebGLBuffer {
  const buffer = gl.createBuffer();
  if (!buffer) {
    throw new Error("Failed to create WebGL buffer.");
  }
  return buffer;
}

function createTexture(gl: WebGL2RenderingContext): WebGLTexture {
  const texture = gl.createTexture();
  if (!texture) {
    throw new Error("Failed to create WebGL texture.");
  }
  return texture;
}

function clipToCanvasX(value: number, width: number): number {
  return ((value + 1) * 0.5) * width;
}

function clipToCanvasY(value: number, height: number): number {
  return ((1 - value) * 0.5) * height;
}

function formatAxisValue(value: number): string {
  if (Math.abs(value) >= 100) {
    return Math.round(value).toString();
  }
  return value.toFixed(1);
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function colorFromMap(colormap: DisplayTransform["colormap"], normalized: number): [number, number, number] {
  if (colormap === "red-white-blue") {
    if (normalized <= 0.5) {
      const ratio = normalized / 0.5;
      return [Math.round(60 + ratio * 195), Math.round(90 + ratio * 165), 255];
    }

    const ratio = (normalized - 0.5) / 0.5;
    return [255, Math.round(255 - ratio * 185), Math.round(255 - ratio * 185)];
  }

  const pixel = Math.round(normalized * 255);
  return [pixel, pixel, pixel];
}

function scalarColorFromMap(colormap: SectionScalarOverlayColorMap, normalized: number): [number, number, number] {
  switch (colormap) {
    case "viridis":
      return colorFromStops(normalized, [
        [0, [68, 1, 84]],
        [0.25, [59, 82, 139]],
        [0.5, [33, 145, 140]],
        [0.75, [94, 201, 98]],
        [1, [253, 231, 37]]
      ]);
    case "turbo":
      return colorFromStops(normalized, [
        [0, [48, 18, 59]],
        [0.2, [40, 102, 203]],
        [0.4, [33, 185, 127]],
        [0.6, [251, 216, 36]],
        [0.8, [245, 115, 29]],
        [1, [122, 4, 3]]
      ]);
    default:
      return colorFromMap("grayscale", normalized);
  }
}

function colorFromStops(
  normalized: number,
  stops: ReadonlyArray<readonly [number, readonly [number, number, number]]>
): [number, number, number] {
  if (normalized <= stops[0]![0]) {
    const [red, green, blue] = stops[0]![1];
    return [red, green, blue];
  }

  for (let index = 1; index < stops.length; index += 1) {
    const previous = stops[index - 1]!;
    const current = stops[index]!;
    if (normalized <= current[0]) {
      const ratio = (normalized - previous[0]) / Math.max(1e-6, current[0] - previous[0]);
      return [
        Math.round(previous[1][0] + (current[1][0] - previous[1][0]) * ratio),
        Math.round(previous[1][1] + (current[1][1] - previous[1][1]) * ratio),
        Math.round(previous[1][2] + (current[1][2] - previous[1][2]) * ratio)
      ];
    }
  }

  const last = stops[stops.length - 1]!;
  return [last[1][0], last[1][1], last[1][2]];
}

function paletteForMode(renderMode: DisplayTransform["renderMode"]): PlotPalette {
  if (renderMode === "wiggle") {
    return {
      shellBackground: "#f2f6f8",
      plotBackground: "#ffffff",
      traceColor: "#101418",
      fillColor: "#101418aa",
      guideColor: "rgba(120, 128, 138, 0.24)",
      axisStroke: "#89a0af",
      axisLabel: "#35505f",
      metaText: "#4a6576"
    };
  }

  return {
    shellBackground: "#f2f6f8",
    plotBackground: "#f7fafc",
    traceColor: "#0b0f12",
    fillColor: "#0b0f1299",
    guideColor: "rgba(120, 140, 155, 0.22)",
    axisStroke: "#a4bac8",
    axisLabel: "#35505f",
    metaText: "#4a6576"
  };
}

function hexToRgbaArray(value: string): [number, number, number, number] {
  if (value.startsWith("rgba")) {
    const [red, green, blue, alpha] = value
      .replace("rgba(", "")
      .replace(")", "")
      .split(",")
      .map((part) => Number.parseFloat(part.trim()));
    return [red / 255, green / 255, blue / 255, alpha];
  }

  const hex = value.replace("#", "");
  const normalized = hex.length === 8 ? hex : `${hex}ff`;
  return [
    Number.parseInt(normalized.slice(0, 2), 16) / 255,
    Number.parseInt(normalized.slice(2, 4), 16) / 255,
    Number.parseInt(normalized.slice(4, 6), 16) / 255,
    Number.parseInt(normalized.slice(6, 8), 16) / 255
  ];
}

const HEATMAP_VERTEX_SHADER = `#version 300 es
in vec2 aPosition;
in vec2 aUv;
out vec2 vUv;

void main() {
  vUv = aUv;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`;

const HEATMAP_FRAGMENT_SHADER = `#version 300 es
precision highp float;

in vec2 vUv;
uniform sampler2D uAmplitude;
uniform sampler2D uLut;
uniform sampler2D uOverlay;
uniform vec2 uSectionSize;
uniform vec4 uViewport;
uniform float uGain;
uniform float uClipMin;
uniform float uClipMax;
uniform float uSymmetricExtent;
uniform float uUseDiverging;
uniform float uPolaritySign;
uniform float uOverlayEnabled;
uniform float uOverlayOpacity;
out vec4 outColor;

void main() {
  float traceIndex = mix(uViewport.x, uViewport.y - 1.0, vUv.x);
  float sampleIndex = mix(uViewport.z, uViewport.w - 1.0, vUv.y);
  vec2 sampleUv = vec2((sampleIndex + 0.5) / uSectionSize.x, (traceIndex + 0.5) / uSectionSize.y);
  float amplitude = texture(uAmplitude, sampleUv).r * uGain * uPolaritySign;

  float normalized = uUseDiverging > 0.5
    ? (amplitude / max(uSymmetricExtent, 0.000001) + 1.0) * 0.5
    : (amplitude - uClipMin) / max(uClipMax - uClipMin, 0.000001);
  normalized = clamp(normalized, 0.0, 1.0);

  vec4 color = texture(uLut, vec2(normalized, 0.5));

  if (uOverlayEnabled > 0.5) {
    float mask = texture(uOverlay, sampleUv).r;
    color.rgb = mix(color.rgb, vec3(0.976, 0.451, 0.086), mask * uOverlayOpacity);
  }

  outColor = color;
}
`;

const GEOMETRY_VERTEX_SHADER = `#version 300 es
in vec2 aPosition;
void main() {
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`;

const GEOMETRY_FRAGMENT_SHADER = `#version 300 es
precision mediump float;
uniform vec4 uColor;
out vec4 outColor;

void main() {
  outColor = uColor;
}
`;
