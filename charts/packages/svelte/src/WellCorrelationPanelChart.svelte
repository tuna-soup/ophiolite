<svelte:options runes={true} />

<script lang="ts">
  import { WellCorrelationController } from "@ophiolite/charts-domain";
  import { WellCorrelationCanvasRenderer } from "@ophiolite/charts-renderer";
  import type { WellCorrelationProbe, WellCorrelationViewport } from "@ophiolite/charts-data-models";
  import { resolveWellCorrelationStageMetrics } from "./well-correlation-stage";
  import {
    WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES,
    type WellCorrelationChartInteractionState,
    type WellCorrelationPanelChartProps
  } from "./types";

  interface PanDragPoint {
    clientX: number;
    clientY: number;
  }

  let {
    chartId,
    panel = null,
    viewport = null,
    interactions = undefined,
    loading = false,
    emptyMessage = "No correlation panel selected.",
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
    onInteractionStateChange,
    onInteractionEvent
  }: WellCorrelationPanelChartProps = $props();

  let controller: WellCorrelationController | null = null;
  let currentProbe = $state.raw<WellCorrelationProbe | null>(null);
  let currentViewport = $state.raw<WellCorrelationViewport | null>(null);
  let lastPanel = $state.raw<WellCorrelationPanelChartProps["panel"]>(null);
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | null = null;
  let lastPanPoint = $state.raw<PanDragPoint | null>(null);
  let lastRequestedTool = resolveRequestedTool();
  let effectiveTool = $state(lastRequestedTool);
  let stageMetrics = $derived(resolveWellCorrelationStageMetrics(panel, stageScale));
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
    const activeController = new WellCorrelationController(new WellCorrelationCanvasRenderer());
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

      const nextInteractionState = createInteractionState(
        controllerModeToTool(
          state.interactions.primaryMode,
          state.interactions.modifiers.includes("crosshair")
        )
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
    const onPointerDown = (event: PointerEvent) => {
      handlePointerDown(event);
    };
    const onPointerMove = (event: PointerEvent) => {
      handlePointerMove(event);
    };
    const onPointerUp = (event: PointerEvent) => {
      handlePointerUp(event);
    };
    const onPointerCancel = (event: PointerEvent) => {
      handlePointerCancel(event);
    };
    const onPointerLeave = () => {
      handlePointerLeave();
    };
    const onFocus = () => {
      handleFocus();
    };
    const onBlur = () => {
      handleBlur();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      handleKeyDown(event);
    };
    const onWheel = (event: WheelEvent) => {
      handleWheel(event);
    };
    const onViewportRequest = (event: Event) => {
      const detail = (event as CustomEvent<WellCorrelationViewport>).detail;
      activeController.setViewport(detail);
    };

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
    element.addEventListener("ophiolite-charts:correlation-viewport-request", onViewportRequest);

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
      element.removeEventListener("ophiolite-charts:correlation-viewport-request", onViewportRequest);
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

  export function setViewport(nextViewport: NonNullable<WellCorrelationPanelChartProps["viewport"]>): void {
    viewport = nextViewport;
    currentViewport = nextViewport;
    controller?.setViewport(nextViewport);
  }

  export function zoomBy(factor: number): void {
    controller?.zoomVertical(factor);
  }

  export function panBy(deltaDepth: number): void {
    controller?.panVertical(deltaDepth);
  }

  function syncController(activeController: WellCorrelationController, forceReset = false): void {
    const requestedTool = resolveRequestedTool();
    if (requestedTool !== lastRequestedTool) {
      lastRequestedTool = requestedTool;
      effectiveTool = requestedTool;
    }

    const panelChanged = panel !== lastPanel;
    const shouldReset = forceReset || resetToken !== lastResetToken || panelChanged;
    lastResetToken = resetToken;

    if (shouldReset) {
      activeController.setPanel(panel);
      lastPanel = panel;
    }

    if (panel && viewport) {
      currentViewport = viewport;
      activeController.setViewport(viewport);
    } else if (!panel) {
      currentViewport = null;
    }

    applyTool(activeController, effectiveTool);
  }

  function applyTool(activeController: WellCorrelationController, tool: "pointer" | "crosshair" | "pan"): void {
    activeController.setPrimaryMode(tool === "pan" ? "panZoom" : "cursor");
    const enabled = activeController.getState().interactions.modifiers.includes("crosshair");
    if (enabled !== (tool === "crosshair")) {
      activeController.toggleCrosshair();
    }
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
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
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
    if (!controller) {
      return;
    }
    if (event.key === "Escape") {
      activeDragKind = null;
      lastPanPoint = null;
      controller.blur();
      event.preventDefault();
      return;
    }

    const state = controller.getState();
    if (!state.panel || !state.viewport) {
      return;
    }

    const step = Math.max(10, (state.viewport.depthEnd - state.viewport.depthStart) * 0.1);
    if (event.key === "ArrowUp") {
      controller.panVertical(-step);
      event.preventDefault();
    } else if (event.key === "ArrowDown") {
      controller.panVertical(step);
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
    const scrollHost = getCorrelationScrollHost(element);
    if (event.shiftKey) {
      scrollHost.scrollLeft += event.deltaY + event.deltaX;
      event.preventDefault();
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      const point = pointerPoint(event, element);
      const panelDepth = controller.getPanelDepthAtViewY(point.y, element.clientWidth, element.clientHeight);
      if (panelDepth !== null) {
        controller.zoomVerticalAround(panelDepth, event.deltaY < 0 ? 1.12 : 0.89);
        event.preventDefault();
      }
      return;
    }

    controller.panVertical(event.deltaY * 0.35);
    event.preventDefault();
  }

  function resolveRequestedTool(): "pointer" | "crosshair" | "pan" {
    return interactions?.tool ?? "pointer";
  }

  function controllerModeToTool(
    mode: "cursor" | "panZoom" | "zoomRect" | "topEdit" | "lassoSelect",
    crosshair: boolean
  ): "pointer" | "crosshair" | "pan" {
    if (mode === "panZoom") {
      return "pan";
    }
    return crosshair ? "crosshair" : "pointer";
  }

  function createInteractionState(tool: "pointer" | "crosshair" | "pan"): WellCorrelationChartInteractionState {
    return {
      capabilities: {
        tools: [...WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES.tools],
        actions: [...WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES.actions]
      },
      tool
    };
  }

  function pointerPoint(
    event: PointerEvent | WheelEvent,
    element: HTMLDivElement
  ): { x: number; y: number } {
    const rect = element.getBoundingClientRect();
    const scrollHost = getCorrelationScrollHost(element);
    return {
      x: event.clientX - rect.left + scrollHost.scrollLeft,
      y: event.clientY - rect.top
    };
  }

  function panByScreenDelta(element: HTMLDivElement, deltaX: number, deltaY: number): void {
    if (!controller) {
      return;
    }
    const state = controller.getState();
    if (!state.panel || !state.viewport) {
      return;
    }
    const scrollHost = getCorrelationScrollHost(element);
    scrollHost.scrollLeft -= deltaX;
    const depthSpan = state.viewport.depthEnd - state.viewport.depthStart;
    const depthDelta = (deltaY / Math.max(1, element.clientHeight)) * depthSpan;
    if (depthDelta !== 0) {
      controller.panVertical(-depthDelta);
    }
  }

  function releasePointerCapture(element: HTMLDivElement, pointerId: number): void {
    if (activePointerId === pointerId) {
      activePointerId = null;
    }
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  }

  function getCorrelationScrollHost(element: HTMLDivElement): HTMLElement {
    return element.querySelector<HTMLElement>(".ophiolite-charts-correlation-scroll-host") ?? element;
  }
</script>

<div class="ophiolite-charts-correlation-shell">
  <div class="ophiolite-charts-correlation-lane">
    <div
      class="ophiolite-charts-correlation-stage"
      style:width={`${stageMetrics.width}px`}
      style:height={`${stageMetrics.height}px`}
      style:--ophiolite-charts-plot-top={`${stageMetrics.plotTop}px`}
      style:--ophiolite-charts-plot-right={`${stageMetrics.plotRight}px`}
      style:--ophiolite-charts-plot-bottom={`${stageMetrics.plotBottom}px`}
      style:--ophiolite-charts-plot-left={`${stageMetrics.plotLeft}px`}
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="ophiolite-charts-correlation-host"
        tabindex="0"
        role="application"
        aria-label="Well correlation panel"
        aria-busy={loading}
        style:cursor={hostCursor ?? undefined}
        {@attach attachChartHost}
      ></div>
      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">{errorMessage}</div>
      {:else if !panel}
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
      {#if currentProbe && !loading && !errorMessage && panel}
        <div
          class="ophiolite-charts-probe-panel"
          style:top={`${stageMetrics.plotTop}px`}
          style:right={`${stageMetrics.plotRight}px`}
        >
          <div class="ophiolite-charts-probe-panel-row">
            <span>well</span>
            <span>{currentProbe.wellName}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>track</span>
            <span>{currentProbe.trackTitle}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>panel</span>
            <span>{currentProbe.panelDepth.toFixed(1)}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>native</span>
            <span>{currentProbe.nativeDepth.toFixed(1)}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>value</span>
            <span>{currentProbe.markerName ?? (currentProbe.value?.toFixed(3) ?? "n/a")}</span>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .ophiolite-charts-correlation-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 320px;
    display: flex;
    overflow-x: auto;
    overflow-y: auto;
    background: #efe8db;
  }

  .ophiolite-charts-correlation-lane {
    min-width: 100%;
    min-height: 100%;
    width: max-content;
    height: max-content;
    display: grid;
    place-items: start;
  }

  .ophiolite-charts-correlation-stage {
    position: relative;
    min-height: 320px;
    flex: 0 0 auto;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-correlation-host {
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
    background: rgba(239, 232, 219, 0.86);
    color: #3f3527;
    font: 600 14px/1.4 sans-serif;
    pointer-events: none;
  }

  .ophiolite-charts-overlay-error {
    color: #8a2e2a;
  }

  .ophiolite-charts-chart-anchor {
    position: absolute;
    z-index: 2;
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
    left: var(--ophiolite-charts-plot-left);
    right: var(--ophiolite-charts-plot-right);
    display: flex;
    justify-content: center;
  }

  .ophiolite-charts-chart-anchor-top-right {
    top: calc(var(--ophiolite-charts-plot-top) + var(--ophiolite-charts-overlay-pad));
    right: var(--ophiolite-charts-plot-right);
  }

  .ophiolite-charts-chart-anchor-bottom-right {
    right: var(--ophiolite-charts-plot-right);
    bottom: calc(var(--ophiolite-charts-plot-bottom) + var(--ophiolite-charts-overlay-pad));
  }

  .ophiolite-charts-chart-anchor-bottom-left {
    left: var(--ophiolite-charts-plot-left);
    bottom: calc(var(--ophiolite-charts-plot-bottom) + var(--ophiolite-charts-overlay-pad));
  }

  .ophiolite-charts-probe-panel {
    position: absolute;
    z-index: 2;
    padding: 8px 10px;
    border: 1px solid rgba(87, 69, 44, 0.18);
    background: rgba(255, 252, 247, 0.94);
    box-shadow: 0 10px 24px rgba(45, 31, 14, 0.12);
    color: #2c2419;
    pointer-events: none;
  }

  .ophiolite-charts-probe-panel-row {
    display: grid;
    grid-template-columns: 52px auto;
    column-gap: 8px;
    align-items: baseline;
    font: 500 12px/1.25 sans-serif;
    white-space: nowrap;
  }

  .ophiolite-charts-probe-panel-row span:first-child {
    color: #7c6850;
    text-transform: lowercase;
  }

  .ophiolite-charts-probe-panel-row span:last-child {
    color: #2c2419;
  }
</style>
