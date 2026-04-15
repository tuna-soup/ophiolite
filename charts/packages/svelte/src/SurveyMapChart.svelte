<svelte:options runes={true} />

<script lang="ts">
  import {
    getSurveyMapPlotRect,
    resolveSurveyMapViewMetrics,
    screenToWorld,
    SURVEY_MAP_MARGIN
  } from "@ophiolite/charts-core";
  import { SurveyMapController } from "@ophiolite/charts-domain";
  import { SurveyMapCanvasRenderer } from "@ophiolite/charts-renderer";
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
    onInteractionEvent
  }: SurveyMapChartProps = $props();

  let controller: SurveyMapController | null = null;
  let currentProbe = $state.raw<import("@ophiolite/charts-data-models").SurveyMapProbe | null>(null);
  let currentViewport = $state.raw<import("@ophiolite/charts-data-models").SurveyMapViewport | null>(null);
  let lastMap = $state.raw<SurveyMapChartProps["map"]>(null);
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastSelectionKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | null = null;
  let lastPanPoint = $state.raw<PanDragPoint | null>(null);
  let hostElement = $state.raw<HTMLDivElement | null>(null);
  let lastRequestedTool = resolveRequestedTool();
  let effectiveTool = $state(lastRequestedTool);
  let stageSize = $derived(
    scaleSurveyMapStageSize(resolveSurveyMapStageSize(map), stageScale)
  );
  let hostCursor = $derived.by(() => {
    if (activeDragKind === "pan") {
      return "grabbing";
    }
    if (effectiveTool === "pan") {
      return "grab";
    }
    return null;
  });

  function attachChartHost(element: HTMLDivElement): () => void {
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

    activeController.mount(element);
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
      syncController(activeController);
    });

    return () => {
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

    const mapChanged = map !== lastMap;
    const shouldReset = resetToken !== lastResetToken || mapChanged;
    lastResetToken = resetToken;

    if (shouldReset) {
      activeController.setMap(map);
      lastMap = map;
    }

    if (map && viewport) {
      currentViewport = viewport;
      activeController.setViewport(viewport);
    } else if (!map) {
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
    controller.focus();
    if (effectiveTool === "pan") {
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
    if (!activeDragKind && effectiveTool === "pointer") {
      const point = pointerPoint(event, element);
      controller.selectAt(point.x, point.y, element.clientWidth, element.clientHeight);
      controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
    }
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
    if (!controller || !currentViewport) {
      return;
    }
    if (event.key === "Escape") {
      activeDragKind = null;
      lastPanPoint = null;
      event.preventDefault();
      return;
    }

    const stepX = (currentViewport.xMax - currentViewport.xMin) * 0.08;
    const stepY = (currentViewport.yMax - currentViewport.yMin) * 0.08;

    switch (event.key) {
      case "ArrowLeft":
        controller.pan(-stepX, 0);
        event.preventDefault();
        break;
      case "ArrowRight":
        controller.pan(stepX, 0);
        event.preventDefault();
        break;
      case "ArrowUp":
        controller.pan(0, stepY);
        event.preventDefault();
        break;
      case "ArrowDown":
        controller.pan(0, -stepY);
        event.preventDefault();
        break;
    }
  }

  function handleWheel(event: WheelEvent): void {
    if (!controller || !currentViewport) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    const plotRect = getSurveyMapPlotRect(element.clientWidth, element.clientHeight);
    const world = screenToWorld(point.x, point.y, currentViewport, plotRect);
    controller.zoomAround(world.x, world.y, event.deltaY < 0 ? 1.12 : 0.9);
    event.preventDefault();
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

  function releasePointerCapture(element: HTMLDivElement, pointerId: number): void {
    if (activePointerId === pointerId) {
      activePointerId = null;
    }
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  }
</script>

<div class="ophiolite-charts-survey-map-shell" style:background={map?.background ?? "#f4f2ee"}>
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
      {:else if errorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">{errorMessage}</div>
      {:else if !map}
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
      {#if currentProbe && !loading && !errorMessage && map}
        <div
          class="ophiolite-charts-probe-panel"
          style:right={`${SURVEY_MAP_MARGIN.right}px`}
          style:bottom={`${SURVEY_MAP_MARGIN.bottom}px`}
        >
          {#if currentProbe.wellName}
            <div class="ophiolite-charts-probe-panel-row">
              <span>well</span>
              <span>{currentProbe.wellName}</span>
            </div>
          {/if}
          {#if currentProbe.scalarValue !== undefined}
            <div class="ophiolite-charts-probe-panel-row">
              <span>{currentProbe.scalarName?.toLowerCase() ?? "value"}</span>
              <span>{currentProbe.scalarValue.toFixed(1)}</span>
            </div>
          {/if}
          <div class="ophiolite-charts-probe-panel-row">
            <span>x</span>
            <span>{currentProbe.x.toFixed(0)}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>y</span>
            <span>{currentProbe.y.toFixed(0)}</span>
          </div>
        </div>
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

  .ophiolite-charts-probe-panel {
    position: absolute;
    z-index: 3;
    padding: 6px 8px;
    border: 1px solid rgba(76, 66, 49, 0.18);
    background: rgba(255, 252, 247, 0.94);
    box-shadow: 0 8px 22px rgba(44, 33, 16, 0.12);
    color: #2f271d;
    pointer-events: none;
  }

  .ophiolite-charts-probe-panel-row {
    display: grid;
    grid-template-columns: 44px auto;
    column-gap: 8px;
    align-items: baseline;
    font: 500 12px/1.2 sans-serif;
    white-space: nowrap;
  }

  .ophiolite-charts-probe-panel-row span:first-child {
    color: #776754;
    text-transform: lowercase;
  }

  .ophiolite-charts-probe-panel-row span:last-child {
    color: #2f271d;
  }
</style>
