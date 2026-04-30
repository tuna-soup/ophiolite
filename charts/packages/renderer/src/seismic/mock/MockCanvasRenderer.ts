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
import {
  globalTraceToLocalIndex,
  resolveLoadedSectionWindow,
  sectionAmplitudeAt,
  sectionHorizontalCoordinateAt,
  sectionSampleValueAt
} from "@ophiolite/charts-data-models";
import {
  buildSeismicTickIndices,
  buildSeismicTopAxisRows,
  formatSeismicAxisValue,
  formatSeismicCanvasFont,
  resolveSeismicPresentationProfile,
  resolveSeismicSampleAxisTitle,
  resolveSeismicSectionTitle
} from "@ophiolite/charts-core";
import {
  applyCanvasSurfaceTransform,
  createRasterSurfaceMetrics,
  resizeCanvasBackingStore,
  scaleRasterRect,
  type RasterSurfaceMetrics
} from "../../internal/rasterSurface";
import type { RendererAdapter } from "../adapter";
import {
  buildOverlaySpatialIndex,
  createBaseRenderState,
  createOverlayRenderState,
  diffRenderStates,
  prepareHeatmapData,
  prepareWiggleData,
  prepareWiggleInstances,
  visibleAmplitudeMaxAbs,
  type BaseRenderState,
  type OverlayRenderState,
  type PreparedHeatmapData,
  type PreparedWiggleInstances
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
import { createRendererTelemetryEvent, type RendererTelemetryListener } from "../../telemetry";

interface GLResources {
  heatmapProgram: WebGLProgram;
  wiggleProgram: WebGLProgram;
  heatmapQuadBuffer: WebGLBuffer;
  wiggleInstanceBuffer: WebGLBuffer;
  amplitudeTexture: WebGLTexture;
  secondaryAmplitudeTexture: WebGLTexture;
  overlayTexture: WebGLTexture;
  lutTexture: WebGLTexture;
  traceInstanceCount: number;
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

const SEISMIC_PRESENTATION = resolveSeismicPresentationProfile("standard");
const SEISMIC_TICK_FONT = formatSeismicCanvasFont(SEISMIC_PRESENTATION.typography.tick);
const SEISMIC_AXIS_LABEL_FONT = formatSeismicCanvasFont(SEISMIC_PRESENTATION.typography.axisLabel);
const SEISMIC_TITLE_FONT = formatSeismicCanvasFont(SEISMIC_PRESENTATION.typography.title);
const SEISMIC_OVERLAY_FONT = formatSeismicCanvasFont(SEISMIC_PRESENTATION.typography.overlay);
const SEISMIC_ANNOTATION_FONT = formatSeismicCanvasFont(SEISMIC_PRESENTATION.typography.annotation);

const scalarOverlayImageCache = new WeakMap<
  SectionScalarOverlay,
  {
    key: string;
    canvas: HTMLCanvasElement;
  }
>();

type SeismicBaseRendererKind = "auto" | "worker-webgl" | "local-webgl" | "local-canvas";

interface MockCanvasRendererOptions {
  axisChrome?: "canvas" | "none";
}

export class MockCanvasRenderer implements RendererAdapter {
  private mountContainer: HTMLElement | null = null;
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
  private preparedWiggles: PreparedWiggleInstances | null = null;
  private wiggleScaleCache = new WeakMap<SectionPayload, Map<string, number>>();
  private lastUploadedSection: SectionPayload | null = null;
  private lastUploadedSecondarySection: SectionPayload | null = null;
  private lastUploadedOverlay: RenderFrame["state"]["overlay"] = null;
  private worker: Worker | null = null;
  private workerMode = false;
  private workerReady = false;
  private workerInitTimeout: number | null = null;
  private forcedBaseRenderer: SeismicBaseRendererKind = "auto";
  private readonly axisChrome: "canvas" | "none";
  private telemetryListener: RendererTelemetryListener | null = null;
  private activeBackend: "canvas-2d" | "webgl" | null = null;

  constructor(options: MockCanvasRendererOptions = {}) {
    this.axisChrome = options.axisChrome ?? "canvas";
  }

  setTelemetryListener(listener: RendererTelemetryListener | null): void {
    this.telemetryListener = listener;
  }

  mount(container: HTMLElement): void {
    this.mountContainer = container;
    this.activeBackend = null;
    this.forcedBaseRenderer = resolveForcedBaseRendererKind();
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
    if (!this.overlayContext) {
      this.emitTelemetry({
        kind: "mount-failed",
        phase: "mount",
        backend: null,
        recoverable: false,
        message: "Seismic renderer could not acquire an overlay 2D canvas context."
      });
      throw new Error("MockCanvasRenderer could not acquire an overlay 2D canvas context.");
    }

    if (
      (this.forcedBaseRenderer === "auto" || this.forcedBaseRenderer === "worker-webgl") &&
      canUseOffscreenWorkerRenderer(this.baseCanvas)
    ) {
      this.startWorkerRenderer();
    } else {
      this.initLocalBaseRenderer();
    }
  }

  render(frame: RenderFrame): void {
    try {
      if (!this.baseCanvas || !this.overlayCanvas || !this.overlayContext || !this.host) {
        return;
      }

      const surface = this.ensureCanvasSize();
      const plotRect = getPlotRect(surface.cssWidth, surface.cssHeight);
      const nextBaseState = createBaseRenderState(frame, plotRect, surface.cssWidth, surface.cssHeight, surface.pixelRatio);
      const nextOverlayState = createOverlayRenderState(
        frame,
        plotRect,
        surface.cssWidth,
        surface.cssHeight,
        surface.pixelRatio
      );
      if (
        nextBaseState.comparisonMode === "split" &&
        nextBaseState.displayTransform.renderMode === "heatmap" &&
        nextBaseState.secondarySection &&
        this.workerMode
      ) {
        this.fallbackToLocalRenderer("Seismic renderer fell back to a local backend because split compare is not supported in the worker path.");
      }
      const invalidation = diffRenderStates(this.lastBaseState, nextBaseState, this.lastOverlayState, nextOverlayState);

      if (invalidation.baseChanged) {
        if (this.workerMode && this.worker) {
          this.renderBaseWorker(nextBaseState, invalidation);
        } else if (this.gl && this.resources) {
          this.renderBaseWebGl(nextBaseState, invalidation, surface);
        } else if (this.baseContext2d) {
          this.renderBaseCanvas(nextBaseState, surface);
        }
      }

      if (invalidation.overlayNeedsDraw || invalidation.baseChanged) {
        this.renderOverlay(nextOverlayState, surface);
      }

      this.lastBaseState = nextBaseState;
      this.lastOverlayState = nextOverlayState;
    } catch (error) {
      this.emitTelemetry({
        kind: "frame-failed",
        phase: "render",
        backend: this.activeBackend,
        recoverable: this.activeBackend !== "canvas-2d",
        message: error instanceof Error ? error.message : String(error),
        detail: "Seismic renderer failed while drawing a frame."
      });
      throw error;
    }
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
      gl.deleteProgram(resources.wiggleProgram);
      gl.deleteBuffer(resources.heatmapQuadBuffer);
      gl.deleteBuffer(resources.wiggleInstanceBuffer);
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
    this.wiggleScaleCache = new WeakMap<SectionPayload, Map<string, number>>();
    this.lastUploadedSection = null;
    this.lastUploadedSecondarySection = null;
    this.lastUploadedOverlay = null;
    this.workerMode = false;
    this.workerReady = false;
    this.activeBackend = null;
    this.mountContainer = null;
  }

  private ensureCanvasSize(): RasterSurfaceMetrics {
    const surface = createRasterSurfaceMetrics(this.host?.clientWidth || 1, this.host?.clientHeight || 1);

    for (const canvas of [this.baseCanvas, this.overlayCanvas]) {
      if (canvas) {
        resizeCanvasBackingStore(canvas, surface);
      }
    }

    return surface;
  }

  private visibleWiggleAmplitudeMaxAbs(section: SectionPayload, viewport: NonNullable<BaseRenderState["viewport"]>): number {
    let sectionCache = this.wiggleScaleCache.get(section);
    if (!sectionCache) {
      sectionCache = new Map<string, number>();
      this.wiggleScaleCache.set(section, sectionCache);
    }

    const key = `${viewport.traceStart}:${viewport.traceEnd}:${viewport.sampleStart}:${viewport.sampleEnd}`;
    const cached = sectionCache.get(key);
    if (cached !== undefined) {
      return cached;
    }

    const value = visibleAmplitudeMaxAbs(
      section,
      viewport.traceStart,
      viewport.traceEnd,
      viewport.sampleStart,
      viewport.sampleEnd
    );
    sectionCache.set(key, value);
    return value;
  }

  private startWorkerRenderer(): void {
    if (!this.baseCanvas) {
      return;
    }

    try {
      this.setBaseRendererKind("worker-pending");
      const surface = this.ensureCanvasSize();
      const plotRect = getPlotRect(surface.cssWidth, surface.cssHeight);
      const offscreen = this.baseCanvas.transferControlToOffscreen();
      this.worker = new Worker(new URL("./baseRenderWorker.ts", import.meta.url), {
        type: "module"
      });
      this.worker.onmessage = (event: MessageEvent<WorkerOutgoingMessage>) => this.handleWorkerMessage(event.data);
      this.worker.onerror = (event) => {
        console.error(event);
        this.fallbackToLocalRenderer("Seismic worker renderer failed and fell back to a local backend.");
      };
      this.worker.postMessage(
        {
          type: "init",
          canvas: offscreen,
          state: this.createWorkerState(surface, plotRect)
        },
        [offscreen]
      );
      this.workerMode = true;
      this.workerInitTimeout = window.setTimeout(() => {
        if (!this.workerReady) {
          this.fallbackToLocalRenderer("Seismic worker renderer did not become ready and fell back to a local backend.");
        }
      }, 750);
    } catch (error) {
      console.error(error);
      this.fallbackToLocalRenderer(
        `Seismic worker renderer initialization failed and fell back to a local backend: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }

  private initLocalBaseRenderer(): void {
    if (!this.baseCanvas) {
      return;
    }

    if (this.forcedBaseRenderer === "local-canvas") {
      this.baseContext2d = this.baseCanvas.getContext("2d");
      if (!this.baseContext2d) {
        this.emitTelemetry({
          kind: "mount-failed",
          phase: "mount",
          backend: "canvas-2d",
          recoverable: false,
          message: "Seismic renderer could not acquire a 2D canvas context."
        });
        throw new Error("MockCanvasRenderer could not acquire a 2D canvas context.");
      }
      this.setBaseRendererKind("local-canvas");
      this.selectBackend("canvas-2d", "mount", "Seismic renderer selected the canvas-2d backend.");
      return;
    }

    this.gl = this.baseCanvas.getContext("webgl2", {
      antialias: true,
      alpha: true,
      premultipliedAlpha: false
    });
    if (this.gl) {
      try {
        this.resources = createGlResources(this.gl);
        this.baseContext2d = null;
        this.setBaseRendererKind("local-webgl");
        this.selectBackend("webgl", "mount", "Seismic renderer selected the WebGL backend.");
        return;
      } catch (error) {
        this.emitTelemetry({
          kind: "fallback-used",
          phase: "mount",
          backend: "canvas-2d",
          previousBackend: "webgl",
          recoverable: true,
          message: "Seismic renderer fell back to canvas after WebGL initialization failed.",
          detail: error instanceof Error ? error.message : String(error)
        });
        this.gl = null;
        this.resources = null;
      }
    }

    this.baseContext2d = this.baseCanvas.getContext("2d");
    if (!this.baseContext2d) {
      this.emitTelemetry({
        kind: "mount-failed",
        phase: "mount",
        backend: "canvas-2d",
        recoverable: false,
        message: "Seismic renderer could not acquire a 2D canvas context."
      });
      throw new Error("MockCanvasRenderer could not acquire a 2D canvas context.");
    }
    this.setBaseRendererKind("local-canvas");
    this.selectBackend("canvas-2d", "mount", "Seismic renderer selected the canvas-2d backend.");
  }

  private renderBaseWorker(baseState: BaseRenderState, invalidation: ReturnType<typeof diffRenderStates>): void {
    if (!this.worker) {
      return;
    }

    if (invalidation.sizeChanged) {
      const surface = this.ensureCanvasSize();
      this.worker.postMessage({
        type: "resize",
        state: this.createWorkerState(surface, baseState.plotRect)
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
          this.fallbackToLocalRenderer("Seismic worker renderer reported that WebGL2 is unavailable.");
        } else {
          this.setBaseRendererKind("worker-webgl");
          this.selectBackend("webgl", "worker", "Seismic renderer selected the worker WebGL backend.");
        }
        break;
      case "error":
        console.error(message.message);
        this.fallbackToLocalRenderer(`Seismic worker renderer failed: ${message.message}`);
        break;
      case "frameRendered":
        break;
    }
  }

  private fallbackToLocalRenderer(detailMessage: string): void {
    if (!this.host || !this.overlayCanvas) {
      return;
    }

    const previousBackend = this.activeBackend ?? "webgl";

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
    const surface = this.ensureCanvasSize();
    this.initLocalBaseRenderer();
    if (this.activeBackend) {
      this.emitTelemetry({
        kind: "fallback-used",
        phase: "worker",
        backend: this.activeBackend,
        previousBackend,
        recoverable: true,
        message: `Seismic renderer fell back to ${this.activeBackend}.`,
        detail: detailMessage
      });
    }

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
        }, surface);
      } else if (this.baseContext2d) {
        this.renderBaseCanvas(this.lastBaseState, surface);
      }
    }
  }

  private createWorkerState(surface: RasterSurfaceMetrics, plotRect: PlotRect): WorkerBaseStatePayload {
    return {
      cssWidth: surface.cssWidth,
      cssHeight: surface.cssHeight,
      pixelRatio: surface.pixelRatio,
      pixelWidth: surface.pixelWidth,
      pixelHeight: surface.pixelHeight,
      plotRect
    };
  }

  private setBaseRendererKind(kind: string): void {
    this.host?.setAttribute("data-base-renderer", kind);
    this.mountContainer?.setAttribute("data-base-renderer", kind);
  }

  private selectBackend(
    backend: "canvas-2d" | "webgl",
    phase: "mount" | "worker",
    message: string
  ): void {
    this.activeBackend = backend;
    this.emitTelemetry({
      kind: "backend-selected",
      phase,
      backend,
      recoverable: true,
      message
    });
  }

  private emitTelemetry(event: Parameters<typeof createRendererTelemetryEvent>[0]): void {
    this.telemetryListener?.(createRendererTelemetryEvent(event));
  }

  private renderBaseWebGl(
    baseState: BaseRenderState,
    invalidation: ReturnType<typeof diffRenderStates>,
    surface: RasterSurfaceMetrics
  ): void {
    if (!this.gl || !this.resources || !this.baseCanvas) {
      return;
    }

    const { gl, resources } = this;
    const pixelPlotRect = scaleRasterRect(baseState.plotRect, surface.pixelRatio);
    const pixelState: BaseRenderState = {
      ...baseState,
      plotRect: pixelPlotRect,
      width: surface.pixelWidth,
      height: surface.pixelHeight
    };
    gl.viewport(0, 0, pixelState.width, pixelState.height);

    if (!pixelState.section || !pixelState.viewport) {
      clearGl(gl, [0.016, 0.075, 0.114, 1]);
      return;
    }

    if (invalidation.dataChanged || this.lastUploadedSection !== pixelState.section) {
      uploadAmplitudeTexture(gl, resources.amplitudeTexture, pixelState.section);
      this.lastUploadedSection = pixelState.section;
    }

    if (pixelState.secondarySection && this.lastUploadedSecondarySection !== pixelState.secondarySection) {
      uploadAmplitudeTexture(gl, resources.secondaryAmplitudeTexture, pixelState.secondarySection);
      this.lastUploadedSecondarySection = pixelState.secondarySection;
    } else if (!pixelState.secondarySection) {
      this.lastUploadedSecondarySection = null;
    }

    if ((invalidation.dataChanged || invalidation.overlayChanged) && pixelState.overlay) {
      uploadOverlayTexture(gl, resources.overlayTexture, pixelState.overlay);
      this.lastUploadedOverlay = pixelState.overlay;
    } else if (!pixelState.overlay) {
      this.lastUploadedOverlay = null;
    }

    if (
      invalidation.styleChanged ||
      invalidation.dataChanged ||
      invalidation.viewportChanged ||
      invalidation.overlayChanged ||
      invalidation.sizeChanged
    ) {
      if (pixelState.displayTransform.renderMode === "heatmap") {
        this.preparedHeatmap = prepareHeatmapData(
          pixelState.section,
          pixelState.viewport,
          pixelState.displayTransform,
          pixelState.overlay,
          pixelState.comparisonMode === "split" ? pixelState.secondarySection : null
        );
        uploadLutTexture(gl, resources.lutTexture, this.preparedHeatmap.lut);
      } else {
        this.preparedWiggles = prepareWiggleInstances(
          pixelState.section,
          pixelState.viewport,
          pixelState.displayTransform,
          pixelState.plotRect,
          pixelState.width,
          {
            visibleAmplitudeMaxAbs: this.visibleWiggleAmplitudeMaxAbs(pixelState.section, pixelState.viewport)
          }
        );
        uploadWiggleInstances(gl, resources.wiggleInstanceBuffer, this.preparedWiggles);
        resources.traceInstanceCount = this.preparedWiggles.traceIndices.length;
      }
    }

    if (invalidation.sizeChanged || invalidation.viewportChanged || invalidation.styleChanged) {
      gl.bindBuffer(gl.ARRAY_BUFFER, resources.heatmapQuadBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, buildPlotQuadVertices(pixelState.plotRect, pixelState.width, pixelState.height), gl.DYNAMIC_DRAW);
    }

    const palette = paletteForMode(pixelState.displayTransform.renderMode);
    clearGl(gl, hexToRgbaArray(palette.shellBackground));
    clearPlotGl(gl, pixelState.plotRect, pixelState.height, hexToRgbaArray(palette.plotBackground));

    if (pixelState.displayTransform.renderMode === "heatmap" && this.preparedHeatmap) {
      const splitHeatmapEnabled =
        pixelState.comparisonMode === "split" && Boolean(pixelState.secondarySection);
      if (splitHeatmapEnabled && pixelState.secondarySection) {
        const splitX = Math.round(pixelState.plotRect.x + pixelState.plotRect.width * pixelState.splitPosition);
        drawHeatmapGl(gl, resources, pixelState, this.preparedHeatmap, resources.amplitudeTexture, {
          x: pixelState.plotRect.x,
          width: Math.max(0, splitX - pixelState.plotRect.x)
        });
        drawHeatmapGl(
          gl,
          resources,
          { ...pixelState, section: pixelState.secondarySection, overlay: null },
          this.preparedHeatmap,
          resources.secondaryAmplitudeTexture,
          {
            x: splitX,
            width: Math.max(0, pixelState.plotRect.x + pixelState.plotRect.width - splitX)
          }
        );
      } else {
        drawHeatmapGl(gl, resources, pixelState, this.preparedHeatmap, resources.amplitudeTexture);
      }
      return;
    }

    if (pixelState.displayTransform.renderMode === "wiggle" && this.preparedWiggles) {
      drawWigglesGl(gl, resources, pixelState, this.preparedWiggles, palette);
    }
  }

  private renderBaseCanvas(baseState: BaseRenderState, surface: RasterSurfaceMetrics): void {
    if (!this.baseContext2d) {
      return;
    }
    applyCanvasSurfaceTransform(this.baseContext2d, surface);

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

  private renderOverlay(overlayState: OverlayRenderState, surface: RasterSurfaceMetrics): void {
    if (!this.overlayContext) {
      return;
    }

    const ctx = this.overlayContext;
    applyCanvasSurfaceTransform(ctx, surface);
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

    if (this.axisChrome === "canvas") {
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
    }

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
  const wiggleProgram = createProgram(gl, WIGGLE_VERTEX_SHADER, WIGGLE_FRAGMENT_SHADER);
  const heatmapQuadBuffer = createBuffer(gl);
  const wiggleInstanceBuffer = createBuffer(gl);
  const amplitudeTexture = createTexture(gl);
  const secondaryAmplitudeTexture = createTexture(gl);
  const overlayTexture = createTexture(gl);
  const lutTexture = createTexture(gl);

  return {
    heatmapProgram,
    wiggleProgram,
    heatmapQuadBuffer,
    wiggleInstanceBuffer,
    amplitudeTexture,
    secondaryAmplitudeTexture,
    overlayTexture,
    lutTexture,
    traceInstanceCount: 0
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
  setSectionTextureUniforms(gl, resources.heatmapProgram, baseState.section);
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

function setSectionTextureUniforms(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  section: SectionPayload,
  sectionSizeUniform = "uSectionSize",
  loadedWindowUniform = "uLoadedWindow"
): void {
  const window = resolveLoadedSectionWindow(section);
  gl.uniform2f(
    gl.getUniformLocation(program, sectionSizeUniform),
    section.dimensions.samples,
    section.dimensions.traces
  );
  gl.uniform4f(
    gl.getUniformLocation(program, loadedWindowUniform),
    window.traceStart,
    window.traceEnd,
    window.sampleStart,
    window.sampleEnd
  );
}

function uploadWiggleInstances(
  gl: WebGL2RenderingContext,
  buffer: WebGLBuffer,
  prepared: PreparedWiggleInstances
): void {
  const interleaved = new Float32Array(prepared.traceIndices.length * 3);
  for (let index = 0; index < prepared.traceIndices.length; index += 1) {
    const offset = index * 3;
    interleaved[offset] = prepared.traceIndices[index]!;
    interleaved[offset + 1] = prepared.baselineClipX[index]!;
    interleaved[offset + 2] = prepared.amplitudeScaleClip[index]!;
  }

  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, interleaved, gl.DYNAMIC_DRAW);
}

function drawWigglesGl(
  gl: WebGL2RenderingContext,
  resources: GLResources,
  baseState: BaseRenderState,
  prepared: PreparedWiggleInstances,
  palette: PlotPalette
): void {
  if (!baseState.viewport || !baseState.section || resources.traceInstanceCount === 0) {
    return;
  }

  const pixelPlotRect = baseState.plotRect;
  gl.enable(gl.SCISSOR_TEST);
  gl.scissor(
    pixelPlotRect.x,
    baseState.height - pixelPlotRect.y - pixelPlotRect.height,
    pixelPlotRect.width,
    pixelPlotRect.height
  );
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.useProgram(resources.wiggleProgram);
  bindTexture(gl, resources.amplitudeTexture, 0);
  gl.uniform1i(gl.getUniformLocation(resources.wiggleProgram, "uAmplitude"), 0);
  setSectionTextureUniforms(gl, resources.wiggleProgram, baseState.section, "uTextureSize", "uLoadedWindow");
  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uGain"), baseState.displayTransform.gain);
  gl.uniform1f(
    gl.getUniformLocation(resources.wiggleProgram, "uPolaritySign"),
    baseState.displayTransform.polarity === "reversed" ? -1 : 1
  );
  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uPlotY"), pixelPlotRect.y);
  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uPlotHeight"), pixelPlotRect.height);
  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uCanvasHeight"), baseState.height);
  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uSampleStart"), prepared.sampleStart);
  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uSampleCount"), prepared.sampleCount);

  gl.bindBuffer(gl.ARRAY_BUFFER, resources.wiggleInstanceBuffer);
  const traceIndexLocation = gl.getAttribLocation(resources.wiggleProgram, "aTraceIndex");
  const baselineLocation = gl.getAttribLocation(resources.wiggleProgram, "aBaselineClipX");
  const amplitudeScaleLocation = gl.getAttribLocation(resources.wiggleProgram, "aAmplitudeScaleClip");
  gl.enableVertexAttribArray(traceIndexLocation);
  gl.vertexAttribPointer(traceIndexLocation, 1, gl.FLOAT, false, 12, 0);
  gl.vertexAttribDivisor(traceIndexLocation, 1);
  gl.enableVertexAttribArray(baselineLocation);
  gl.vertexAttribPointer(baselineLocation, 1, gl.FLOAT, false, 12, 4);
  gl.vertexAttribDivisor(baselineLocation, 1);
  gl.enableVertexAttribArray(amplitudeScaleLocation);
  gl.vertexAttribPointer(amplitudeScaleLocation, 1, gl.FLOAT, false, 12, 8);
  gl.vertexAttribDivisor(amplitudeScaleLocation, 1);

  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uFillMode"), 1);
  gl.uniform4fv(gl.getUniformLocation(resources.wiggleProgram, "uColor"), hexToRgbaArray(palette.fillColor));
  gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, prepared.sampleCount * 2, resources.traceInstanceCount);

  gl.uniform1f(gl.getUniformLocation(resources.wiggleProgram, "uFillMode"), 0);
  gl.uniform4fv(gl.getUniformLocation(resources.wiggleProgram, "uColor"), hexToRgbaArray(palette.traceColor));
  gl.drawArraysInstanced(gl.LINE_STRIP, 0, prepared.sampleCount, resources.traceInstanceCount);

  gl.vertexAttribDivisor(traceIndexLocation, 0);
  gl.vertexAttribDivisor(baselineLocation, 0);
  gl.vertexAttribDivisor(amplitudeScaleLocation, 0);
  gl.disable(gl.BLEND);
  gl.disable(gl.SCISSOR_TEST);
}

function drawEmptyState(ctx: CanvasRenderingContext2D, width: number, height: number): void {
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = SEISMIC_PRESENTATION.palette.shellBackground;
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = SEISMIC_PRESENTATION.palette.title;
  ctx.font = SEISMIC_OVERLAY_FONT;
  ctx.textBaseline = "top";
  ctx.fillText("No section loaded", 32, 40);
}

function resolveForcedBaseRendererKind(): SeismicBaseRendererKind {
  const forced = (globalThis as { __OPHIOLITE_FORCE_SEISMIC_BASE_RENDERER__?: unknown })
    .__OPHIOLITE_FORCE_SEISMIC_BASE_RENDERER__;
  switch (forced) {
    case "worker-webgl":
    case "local-webgl":
    case "local-canvas":
      return forced;
    default:
      return "auto";
  }
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

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const amplitude = sectionAmplitudeAt(section, trace, sample);
      if (amplitude === null) {
        continue;
      }
      const value = amplitude * displayTransform.gain;
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
  }

  const clipMin = forcedClipMin ?? displayTransform.clipMin ?? min;
  const clipMax = forcedClipMax ?? displayTransform.clipMax ?? max;
  if (!Number.isFinite(clipMin) || !Number.isFinite(clipMax)) {
    return;
  }
  const denominator = Math.max(1e-6, clipMax - clipMin);
  const symmetricExtent = Math.max(Math.abs(clipMin), Math.abs(clipMax), 1e-6);

  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      const amplitude = sectionAmplitudeAt(section, trace, sample);
      if (amplitude === null) {
        continue;
      }
      const source = amplitude * displayTransform.gain;
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
    ctx.font = SEISMIC_ANNOTATION_FONT;
    ctx.textBaseline = "middle";
    ctx.lineWidth = 3;
    ctx.strokeStyle = SEISMIC_PRESENTATION.palette.annotationHalo;
    ctx.strokeText(overlay.name, labelPoint.x + SEISMIC_PRESENTATION.frame.annotationOffsetX, labelPoint.y);
    ctx.fillText(overlay.name, labelPoint.x + SEISMIC_PRESENTATION.frame.annotationOffsetX, labelPoint.y);
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
  ctx.font = SEISMIC_TICK_FONT;
  ctx.textAlign = "center";
  ctx.textBaseline = "bottom";

  const visibleCoords = [];
  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    const coordinate = sectionHorizontalCoordinateAt(section, trace);
    if (coordinate !== null) {
      visibleCoords.push(coordinate);
    }
  }
  const coordMin = Math.min(...visibleCoords);
  const coordMax = Math.max(...visibleCoords);
  const topTicks = buildSeismicTickIndices(traceStart, traceEnd, 12);
  const topAxisRows = buildSeismicTopAxisRows(section);
  for (const traceIndex of topTicks) {
    const x =
      renderMode === "wiggle"
        ? mapCoordinateToPlotX(
            sectionHorizontalCoordinateAt(section, traceIndex) ?? traceIndex,
            coordMin,
            coordMax,
            plotRect
          )
        : plotRect.x + ((traceIndex - traceStart) / Math.max(1, traceEnd - traceStart - 1)) * plotRect.width;
    ctx.beginPath();
    ctx.moveTo(x, plotRect.y);
    ctx.lineTo(x, plotRect.y - SEISMIC_PRESENTATION.frame.topTickLength);
    ctx.stroke();
    for (const [rowIndex, row] of topAxisRows.entries()) {
      const localTraceIndex = globalTraceToLocalIndex(section, traceIndex);
      if (localTraceIndex === null) {
        continue;
      }
      ctx.fillText(
        formatSeismicAxisValue(row.values[localTraceIndex]!),
        x,
        plotRect.y -
          SEISMIC_PRESENTATION.frame.topTickOffset -
          rowIndex * SEISMIC_PRESENTATION.frame.topAxisRowSpacing
      );
    }
  }

  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  const leftTicks = buildSeismicTickIndices(sampleStart, sampleEnd, 14);
  for (const sampleIndex of leftTicks) {
    const ratio = (sampleIndex - sampleStart) / Math.max(1, sampleEnd - sampleStart - 1);
    const y = plotRect.y + ratio * plotRect.height;
    ctx.beginPath();
    ctx.moveTo(plotRect.x, y);
    ctx.lineTo(plotRect.x - SEISMIC_PRESENTATION.frame.leftTickLength, y);
    ctx.stroke();
    ctx.fillText(
      formatSeismicAxisValue(sectionSampleValueAt(section, sampleIndex) ?? sampleIndex),
      plotRect.x - SEISMIC_PRESENTATION.frame.leftTickOffset,
      y
    );
  }

  ctx.fillStyle = palette.metaText;
  ctx.font = SEISMIC_TITLE_FONT;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  ctx.fillText(resolveSeismicSectionTitle(section), plotRect.x + plotRect.width / 2, SEISMIC_PRESENTATION.frame.titleY);

  ctx.font = SEISMIC_AXIS_LABEL_FONT;
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  for (const [rowIndex, row] of topAxisRows.entries()) {
    ctx.fillText(
      row.label,
      SEISMIC_PRESENTATION.frame.topAxisLabelX,
      plotRect.y -
        SEISMIC_PRESENTATION.frame.topAxisRowLabelOffset -
        rowIndex * SEISMIC_PRESENTATION.frame.topAxisRowSpacing
    );
  }
  ctx.save();
  ctx.translate(SEISMIC_PRESENTATION.frame.yAxisLabelX, plotRect.y + plotRect.height / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.textAlign = "center";
  ctx.fillText(resolveSeismicSampleAxisTitle(section), 0, 0);
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
  const ticks = buildSeismicTickIndices(sampleStart, sampleEnd, 14);
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
      shellBackground: SEISMIC_PRESENTATION.palette.shellBackground,
      plotBackground: "#ffffff",
      traceColor: "#101418",
      fillColor: "#101418aa",
      guideColor: "rgba(120, 128, 138, 0.24)",
      axisStroke: SEISMIC_PRESENTATION.palette.axisStroke,
      axisLabel: SEISMIC_PRESENTATION.palette.axisLabel,
      metaText: SEISMIC_PRESENTATION.palette.title
    };
  }

  return {
    shellBackground: SEISMIC_PRESENTATION.palette.shellBackground,
    plotBackground: "#f7fafc",
    traceColor: "#0b0f12",
    fillColor: "#0b0f1299",
    guideColor: "rgba(120, 140, 155, 0.22)",
    axisStroke: SEISMIC_PRESENTATION.palette.axisStroke,
    axisLabel: SEISMIC_PRESENTATION.palette.axisLabel,
    metaText: SEISMIC_PRESENTATION.palette.title
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
uniform vec4 uLoadedWindow;
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
  if (
    traceIndex < uLoadedWindow.x ||
    traceIndex >= uLoadedWindow.y ||
    sampleIndex < uLoadedWindow.z ||
    sampleIndex >= uLoadedWindow.w
  ) {
    discard;
  }
  vec2 sampleUv = vec2(
    ((sampleIndex - uLoadedWindow.z) + 0.5) / uSectionSize.x,
    ((traceIndex - uLoadedWindow.x) + 0.5) / uSectionSize.y
  );
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

const WIGGLE_VERTEX_SHADER = `#version 300 es
precision highp float;
in float aTraceIndex;
in float aBaselineClipX;
in float aAmplitudeScaleClip;
uniform sampler2D uAmplitude;
uniform vec2 uTextureSize;
uniform vec4 uLoadedWindow;
uniform float uGain;
uniform float uPolaritySign;
uniform float uPlotY;
uniform float uPlotHeight;
uniform float uCanvasHeight;
uniform float uSampleStart;
uniform float uSampleCount;
uniform float uFillMode;
void main() {
  float logicalVertex = float(gl_VertexID);
  float sampleOffset = uFillMode > 0.5 ? floor(logicalVertex / 2.0) : logicalVertex;
  float sampleIndex = uSampleStart + sampleOffset;
  float localSampleIndex = sampleIndex - uLoadedWindow.z;
  float localTraceIndex = aTraceIndex - uLoadedWindow.x;
  float amplitude = 0.0;
  if (
    localSampleIndex >= 0.0 &&
    localSampleIndex < uTextureSize.x &&
    localTraceIndex >= 0.0 &&
    localTraceIndex < uTextureSize.y
  ) {
    amplitude = texelFetch(uAmplitude, ivec2(int(localSampleIndex), int(localTraceIndex)), 0).r;
  }
  float offset = amplitude * uGain * aAmplitudeScaleClip * uPolaritySign;
  if (uFillMode > 0.5) {
    float side = mod(logicalVertex, 2.0);
    offset = max(offset, 0.0) * side;
  }
  float yPx = uPlotY + (sampleOffset / max(uSampleCount - 1.0, 1.0)) * uPlotHeight;
  float yClip = 1.0 - (yPx / uCanvasHeight) * 2.0;
  gl_Position = vec4(aBaselineClipX + offset, yClip, 0.0, 1.0);
}
`;

const WIGGLE_FRAGMENT_SHADER = `#version 300 es
precision mediump float;
uniform vec4 uColor;
out vec4 outColor;

void main() {
  outColor = uColor;
}
`;
