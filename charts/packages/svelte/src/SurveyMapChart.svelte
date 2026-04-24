<svelte:options runes={true} />

<script lang="ts">
  import {
    getSurveyMapPlotRect,
    resolveProbePanelPresentation,
    resolveSurveyMapViewMetrics,
    screenToWorld,
    SURVEY_MAP_MARGIN
  } from "@ophiolite/charts-core";
  import type { ChartRendererTelemetryEvent } from "@ophiolite/charts-data-models";
  import { SurveyMapController } from "@ophiolite/charts-domain";
  import { SurveyMapCanvasRenderer } from "@ophiolite/charts-renderer";
  import ProbePanel from "./ProbePanel.svelte";
  import { emitRendererStatusForChart } from "./renderer-status";
  import { adaptSurveyMapInputToModel } from "./survey-map-public-model";
  import { resolveSurveyMapStageSize, scaleSurveyMapStageSize } from "./survey-map-stage";
  import {
    SURVEY_MAP_CHART_INTERACTION_CAPABILITIES,
    type SurveyMapChartInteractionState,
    type SurveyMapChartProps
  } from "./types";

  interface PanDragPoint {
    clientX: number;
    clientY: number;
  }

  let {
    chartId,
    map = null,
    renderer = undefined,
    viewport = null,
    interactions = undefined,
    loading = false,
    emptyMessage = "No survey map selected.",
    errorMessage = null,
    resetToken = null,
    stageTopLeft = undefined,
    plotTopCenter = undefined,
    plotTopRight = undefined,
    plotBottomRight = undefined,
    plotBottomLeft = undefined,
    stageScale = 1,
    onViewportChange,
    onProbeChange,
    onSelectionChange,
    onInteractionStateChange,
    onInteractionEvent,
    onRendererStatusChange,
    onRendererTelemetry
  }: SurveyMapChartProps = $props();

  let controller: SurveyMapController | null = null;
  let currentProbe = $state.raw<import("@ophiolite/charts-data-models").SurveyMapProbe | null>(null);
  let currentViewport = $state.raw<import("@ophiolite/charts-data-models").SurveyMapViewport | null>(null);
  let lastMap = $state.raw<import("@ophiolite/charts-data-models").SurveyMapModel | null>(null);
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastSelectionKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let lastRendererStatusKey = "";
  let lastRendererTelemetryEvent = $state.raw<ChartRendererTelemetryEvent | null>(null);
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | null = null;
  let lastPanPoint = $state.raw<PanDragPoint | null>(null);
  let rendererErrorMessage = $state<string | null>(null);
  let hostElement = $state.raw<HTMLDivElement | null>(null);
  let lastRequestedTool = resolveRequestedTool();
  let effectiveTool = $state(lastRequestedTool);
  let normalizedMap = $derived(adaptSurveyMapInputToModel(map));
  const surveyMapProbePanelInset = resolveProbePanelPresentation("light", "compact").frame.insetPx;
  let stageSize = $derived(
    scaleSurveyMapStageSize(resolveSurveyMapStageSize(normalizedMap), stageScale)
  );
  let surveyMapProbePanelPosition = $derived.by(() => {
    if (!currentViewport) {
      return null;
    }
    const stageWidth = hostElement?.clientWidth ?? stageSize.width;
    const stageHeight = hostElement?.clientHeight ?? stageSize.height;
    const plotRect = getSurveyMapPlotRect(stageWidth, stageHeight);
    const drawRect = resolveSurveyMapViewMetrics(currentViewport, plotRect).drawRect;
    return {
      left: `${Math.round(drawRect.x + surveyMapProbePanelInset)}px`,
      bottom: `${Math.round(stageHeight - (drawRect.y + drawRect.height) + surveyMapProbePanelInset)}px`
    };
  });
  let hostCursor = $derived.by(() => {
    if (activeDragKind === "pan") {
      return "grabbing";
    }
    if (effectiveTool === "pan") {
      return "grab";
    }
    return null;
  });
  $effect(() => {
    lastRendererStatusKey = emitRendererStatusForChart(
      "survey-map",
      {
        chartId,
        renderer: rendererErrorMessage ? { ...(renderer ?? {}), runtimeErrorMessage: rendererErrorMessage } : renderer,
        telemetryEvent: lastRendererTelemetryEvent
      },
      lastRendererStatusKey,
      onRendererStatusChange
    );
  });

  function handleRendererTelemetry(event: ChartRendererTelemetryEvent): void {
    lastRendererTelemetryEvent = { ...event };
    if (
      event.kind === "mount-failed" ||
      event.kind === "frame-failed" ||
      event.kind === "context-lost"
    ) {
      rendererErrorMessage = event.message;
    } else if (
      event.kind === "backend-selected" ||
      event.kind === "fallback-used" ||
      event.kind === "context-restored"
    ) {
      rendererErrorMessage = null;
    }
    onRendererTelemetry?.({
      chartId,
      event: { ...event }
    });
  }

  function attachChartHost(element: HTMLDivElement): () => void {
    rendererErrorMessage = null;
    lastRendererTelemetryEvent = null;
    const activeController = new SurveyMapController(new SurveyMapCanvasRenderer());
    controller = activeController;
    currentProbe = null;
    currentViewport = null;

    const unsubscribeStateChange = activeController.onStateChange((state) => {
      const nextViewportKey = JSON.stringify(state.viewport);
      if (nextViewportKey !== lastViewportKey) {
        lastViewportKey = nextViewportKey;
        currentViewport = state.viewport ? { ...state.viewport } : null;
        onViewportChange?.({
          chartId,
          viewport: currentViewport
        });
      }

      const nextProbeKey = JSON.stringify(state.probe);
      if (nextProbeKey !== lastProbeKey) {
        lastProbeKey = nextProbeKey;
        currentProbe = state.probe ? { ...state.probe } : null;
        onProbeChange?.({
          chartId,
          probe: currentProbe
        });
      }

      const nextSelectionKey = state.selectedWellId ?? "";
      if (nextSelectionKey !== lastSelectionKey) {
        lastSelectionKey = nextSelectionKey;
        onSelectionChange?.({
          chartId,
          wellId: state.selectedWellId
        });
      }

      const nextInteractionState = createInteractionState(
        controllerModeToTool(state.interactions.primaryMode)
      );
      const nextInteractionStateKey = JSON.stringify(nextInteractionState);
      if (nextInteractionStateKey !== lastInteractionStateKey) {
        lastInteractionStateKey = nextInteractionStateKey;
        onInteractionStateChange?.(nextInteractionState);
      }
    });
    const unsubscribeInteractionEvent = activeController.onInteractionEvent((event) => {
      const nextInteractionKey = JSON.stringify(event);
      if (nextInteractionKey !== lastInteractionKey) {
        lastInteractionKey = nextInteractionKey;
        onInteractionEvent?.({
          chartId,
          event
        });
      }
    });
    const unsubscribeRendererTelemetry = activeController.onRendererTelemetry((event) => {
      handleRendererTelemetry(event);
    });

    const resizeObserver = new ResizeObserver(() => {
      activeController.refresh();
    });
    const onPointerDown = (event: PointerEvent) => handlePointerDown(event);
    const onPointerMove = (event: PointerEvent) => handlePointerMove(event);
    const onPointerUp = (event: PointerEvent) => handlePointerUp(event);
    const onPointerCancel = (event: PointerEvent) => handlePointerCancel(event);
    const onPointerLeave = () => handlePointerLeave();
    const onFocus = () => handleFocus();
    const onBlur = () => handleBlur();
    const onKeyDown = (event: KeyboardEvent) => handleKeyDown(event);
    const onWheel = (event: WheelEvent) => handleWheel(event);

    try {
      activeController.mount(element);
    } catch (error) {
      if (!lastRendererTelemetryEvent) {
        handleRendererTelemetry({
          kind: "mount-failed",
          phase: "mount",
          backend: null,
          recoverable: false,
          message: error instanceof Error ? error.message : String(error),
          detail: "Survey map wrapper observed a controller initialization failure.",
          timestampMs: performance.now()
        });
      }
      console.error("SurveyMapChart initialization failed.", error);
      unsubscribeRendererTelemetry();
      unsubscribeInteractionEvent();
      unsubscribeStateChange();
      resizeObserver.disconnect();
      if (controller === activeController) {
        controller = null;
      }
      currentProbe = null;
      currentViewport = null;
      activeController.dispose();
      return () => {};
    }
    resizeObserver.observe(element);
    element.addEventListener("pointerdown", onPointerDown);
    element.addEventListener("pointermove", onPointerMove);
    element.addEventListener("pointerup", onPointerUp);
    element.addEventListener("pointercancel", onPointerCancel);
    element.addEventListener("pointerleave", onPointerLeave);
    element.addEventListener("focus", onFocus);
    element.addEventListener("blur", onBlur);
    element.addEventListener("keydown", onKeyDown);
    element.addEventListener("wheel", onWheel, { passive: false });

    $effect(() => {
      try {
        syncController(activeController);
      } catch (error) {
        handleRendererTelemetry({
          kind: "frame-failed",
          phase: "render",
          backend: null,
          recoverable: true,
          message: error instanceof Error ? error.message : String(error),
          detail: "Survey map wrapper observed a controller sync failure.",
          timestampMs: performance.now()
        });
        console.error("SurveyMapChart sync failed.", error);
      }
    });

    return () => {
      unsubscribeRendererTelemetry();
      unsubscribeInteractionEvent();
      unsubscribeStateChange();
      resizeObserver.disconnect();
      element.removeEventListener("pointerdown", onPointerDown);
      element.removeEventListener("pointermove", onPointerMove);
      element.removeEventListener("pointerup", onPointerUp);
      element.removeEventListener("pointercancel", onPointerCancel);
      element.removeEventListener("pointerleave", onPointerLeave);
      element.removeEventListener("focus", onFocus);
      element.removeEventListener("blur", onBlur);
      element.removeEventListener("keydown", onKeyDown);
      element.removeEventListener("wheel", onWheel);
      if (controller === activeController) {
        controller = null;
      }
      currentProbe = null;
      currentViewport = null;
      activeController.dispose();
    };
  }

  export function fitToData(): void {
    controller?.fitToData();
  }

  export function setViewport(nextViewport: NonNullable<SurveyMapChartProps["viewport"]>): void {
    viewport = nextViewport;
    currentViewport = nextViewport;
    controller?.setViewport(nextViewport);
  }

  export function zoomBy(factor: number): void {
    controller?.zoom(factor);
  }

  function syncController(activeController: SurveyMapController): void {
    const requestedTool = resolveRequestedTool();
    if (requestedTool !== lastRequestedTool) {
      lastRequestedTool = requestedTool;
      effectiveTool = requestedTool;
    }

    const mapChanged = normalizedMap !== lastMap;
    const shouldReset = resetToken !== lastResetToken || mapChanged;
    lastResetToken = resetToken;

    if (shouldReset) {
      activeController.setMap(normalizedMap);
      lastMap = normalizedMap;
    }

    if (normalizedMap && viewport) {
      currentViewport = viewport;
      activeController.setViewport(viewport);
    } else if (!normalizedMap) {
      currentViewport = null;
    }

    activeController.setPrimaryMode(effectiveTool === "pan" ? "panZoom" : "cursor");
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    if (activeDragKind === "pan") {
      if (lastPanPoint) {
        panByScreenDelta(element, event.clientX - lastPanPoint.clientX, event.clientY - lastPanPoint.clientY);
      }
      lastPanPoint = {
        clientX: event.clientX,
        clientY: event.clientY
      };
      return;
    }
    const point = pointerPoint(event, element);
    controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement) || event.button !== 0) {
      return;
    }
    activePointerId = event.pointerId;
    element.setPointerCapture(event.pointerId);
    if (controller.handlePrimaryPointerDown() === "pan") {
      activeDragKind = "pan";
      lastPanPoint = {
        clientX: event.clientX,
        clientY: event.clientY
      };
      controller.clearPointer();
    }
  }

  function handlePointerUp(event: PointerEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    controller.handlePrimaryPointerUp(
      point.x,
      point.y,
      element.clientWidth,
      element.clientHeight,
      activeDragKind === "pan"
    );
    activeDragKind = null;
    lastPanPoint = null;
    releasePointerCapture(element, event.pointerId);
  }

  function handlePointerCancel(event: PointerEvent): void {
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    activeDragKind = null;
    lastPanPoint = null;
    controller?.blur();
    releasePointerCapture(element, event.pointerId);
  }

  function handlePointerLeave(): void {
    if (activeDragKind) {
      return;
    }
    controller?.clearPointer();
  }

  function handleFocus(): void {
    controller?.focus();
  }

  function handleBlur(): void {
    activeDragKind = null;
    lastPanPoint = null;
    controller?.blur();
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (!controller) {
      return;
    }
    if (event.key === "Escape") {
      activeDragKind = null;
      lastPanPoint = null;
    }
    if (controller.handleKeyboardNavigation(event.key)) {
      event.preventDefault();
    }
  }

  function handleWheel(event: WheelEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    if (controller.handleWheelAt(point.x, point.y, element.clientWidth, element.clientHeight, event.deltaY)) {
      event.preventDefault();
    }
  }

  function resolveRequestedTool(): "pointer" | "pan" {
    return interactions?.tool ?? "pointer";
  }

  function controllerModeToTool(mode: "cursor" | "panZoom" | "zoomRect" | "topEdit" | "lassoSelect"): "pointer" | "pan" {
    return mode === "panZoom" ? "pan" : "pointer";
  }

  function createInteractionState(tool: "pointer" | "pan"): SurveyMapChartInteractionState {
    return {
      capabilities: {
        tools: [...SURVEY_MAP_CHART_INTERACTION_CAPABILITIES.tools],
        actions: [...SURVEY_MAP_CHART_INTERACTION_CAPABILITIES.actions]
      },
      tool
    };
  }

  function pointerPoint(event: PointerEvent | WheelEvent, element: HTMLDivElement): { x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
    };
  }

  function panByScreenDelta(element: HTMLDivElement, deltaX: number, deltaY: number): void {
    if (!controller || !currentViewport) {
      return;
    }
    const plotRect = getSurveyMapPlotRect(element.clientWidth, element.clientHeight);
    const metrics = resolveSurveyMapViewMetrics(currentViewport, plotRect);
    controller.pan(-deltaX / Math.max(metrics.scale, 1e-6), deltaY / Math.max(metrics.scale, 1e-6));
  }

  function surveyMapProbeRows(): Array<{ label: string; value: string }> {
    if (!currentProbe) {
      return [];
    }

    const rows = [
      { label: "x", value: currentProbe.x.toFixed(0) },
      { label: "y", value: currentProbe.y.toFixed(0) }
    ];

    if (currentProbe.scalarValue !== undefined) {
      rows.unshift({
        label: currentProbe.scalarName?.toLowerCase() ?? "value",
        value: currentProbe.scalarValue.toFixed(1)
      });
    }

    return rows;
  }

  function releasePointerCapture(element: HTMLDivElement, pointerId: number): void {
    if (activePointerId === pointerId) {
      activePointerId = null;
    }
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  }
</script>

<div class="ophiolite-charts-survey-map-shell" style:background={normalizedMap?.background ?? "#f4f2ee"}>
  <div class="ophiolite-charts-survey-map-lane">
    <div
      class="ophiolite-charts-survey-map-stage"
      bind:this={hostElement}
      style:width={`${stageSize.width}px`}
      style:height={`${stageSize.height}px`}
      style:--ophiolite-charts-plot-top={`${SURVEY_MAP_MARGIN.top}px`}
      style:--ophiolite-charts-plot-right={`${SURVEY_MAP_MARGIN.right}px`}
      style:--ophiolite-charts-plot-bottom={`${SURVEY_MAP_MARGIN.bottom}px`}
      style:--ophiolite-charts-plot-left={`${SURVEY_MAP_MARGIN.left}px`}
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="ophiolite-charts-survey-map-host"
        tabindex="0"
        role="application"
        aria-label="Survey map chart"
        aria-busy={loading}
        style:cursor={hostCursor ?? undefined}
        {@attach attachChartHost}
      ></div>
      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage || rendererErrorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">
          {errorMessage ?? rendererErrorMessage}
        </div>
      {:else if !normalizedMap}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {/if}
      {#if stageTopLeft}
        <div class="ophiolite-charts-chart-anchor ophiolite-charts-chart-anchor-stage-top-left">
          <div class="ophiolite-charts-chart-anchor-content">
            {@render stageTopLeft()}
          </div>
        </div>
      {/if}
      {#if plotTopCenter}
        <div class="ophiolite-charts-chart-anchor ophiolite-charts-chart-anchor-top-center">
          <div class="ophiolite-charts-chart-anchor-content">
            {@render plotTopCenter()}
          </div>
        </div>
      {/if}
      {#if plotTopRight}
        <div class="ophiolite-charts-chart-anchor ophiolite-charts-chart-anchor-top-right">
          <div class="ophiolite-charts-chart-anchor-content">
            {@render plotTopRight()}
          </div>
        </div>
      {/if}
      {#if plotBottomRight}
        <div class="ophiolite-charts-chart-anchor ophiolite-charts-chart-anchor-bottom-right">
          <div class="ophiolite-charts-chart-anchor-content">
            {@render plotBottomRight()}
          </div>
        </div>
      {/if}
      {#if plotBottomLeft}
        <div class="ophiolite-charts-chart-anchor ophiolite-charts-chart-anchor-bottom-left">
          <div class="ophiolite-charts-chart-anchor-content">
            {@render plotBottomLeft()}
          </div>
        </div>
      {/if}
      {#if currentProbe && !loading && !errorMessage && !rendererErrorMessage && normalizedMap && surveyMapProbePanelPosition}
        <ProbePanel
          theme="light"
          size="compact"
          left={surveyMapProbePanelPosition.left}
          bottom={surveyMapProbePanelPosition.bottom}
          rows={surveyMapProbeRows()}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .ophiolite-charts-survey-map-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 240px;
    display: flex;
    overflow-x: auto;
    overflow-y: auto;
  }

  .ophiolite-charts-survey-map-lane {
    min-width: 100%;
    min-height: 100%;
    width: max-content;
    height: max-content;
    display: grid;
    place-items: start;
  }

  .ophiolite-charts-survey-map-stage {
    position: relative;
    min-height: 240px;
    flex: 0 0 auto;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-survey-map-host {
    width: 100%;
    height: 100%;
    outline: none;
  }

  .ophiolite-charts-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(244, 242, 238, 0.86);
    color: #40362b;
    font: 600 14px/1.4 sans-serif;
    pointer-events: none;
  }

  .ophiolite-charts-overlay-error {
    color: #8a3027;
  }

  .ophiolite-charts-chart-anchor {
    position: absolute;
    z-index: 3;
    pointer-events: none;
  }

  .ophiolite-charts-chart-anchor-content {
    pointer-events: auto;
  }

  .ophiolite-charts-chart-anchor-stage-top-left {
    top: var(--ophiolite-charts-overlay-pad);
    left: var(--ophiolite-charts-overlay-pad);
  }

  .ophiolite-charts-chart-anchor-top-center {
    top: calc(var(--ophiolite-charts-plot-top) + var(--ophiolite-charts-overlay-pad));
    left: calc(var(--ophiolite-charts-plot-left) + ((100% - var(--ophiolite-charts-plot-left) - var(--ophiolite-charts-plot-right)) / 2));
    transform: translateX(-50%);
  }

  .ophiolite-charts-chart-anchor-top-right {
    top: calc(var(--ophiolite-charts-plot-top) + var(--ophiolite-charts-overlay-pad));
    right: calc(var(--ophiolite-charts-plot-right) + var(--ophiolite-charts-overlay-pad));
  }

  .ophiolite-charts-chart-anchor-bottom-right {
    right: calc(var(--ophiolite-charts-plot-right) + var(--ophiolite-charts-overlay-pad));
    bottom: calc(var(--ophiolite-charts-plot-bottom) + var(--ophiolite-charts-overlay-pad));
  }

  .ophiolite-charts-chart-anchor-bottom-left {
    left: calc(var(--ophiolite-charts-plot-left) + var(--ophiolite-charts-overlay-pad));
    bottom: calc(var(--ophiolite-charts-plot-bottom) + var(--ophiolite-charts-overlay-pad));
  }

</style>
