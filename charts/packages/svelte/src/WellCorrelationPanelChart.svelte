<svelte:options runes={true} />

<script lang="ts">
  import { onMount, untrack } from "svelte";
  import {
    buildWellCorrelationChromeModel,
    resolveProbePanelPresentation,
    type NormalizedWellPanelModel
  } from "@ophiolite/charts-core";
  import { WellCorrelationController } from "@ophiolite/charts-domain";
  import { WellCorrelationCanvasRenderer } from "@ophiolite/charts-renderer";
  import type { WellCorrelationProbe, WellCorrelationViewport } from "@ophiolite/charts-data-models";
  import ProbePanel from "./ProbePanel.svelte";
  import WellCorrelationAxisOverlay from "./WellCorrelationAxisOverlay.svelte";
  import {
    WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES,
    type WellCorrelationDebugSnapshot,
    type WellCorrelationChartInteractionState,
    type WellCorrelationPanelChartProps
  } from "./types";

  interface PanDragPoint {
    clientX: number;
    clientY: number;
  }

  interface ScrollbarDragState {
    pointerId: number;
    offsetPx: number;
  }

  interface CorrelationRendererDebugState {
    renderCount: number;
    lastRenderMs: number | null;
    baseChanged: boolean | null;
    overlayDraw: boolean | null;
    contentWidth: number | null;
    viewportWidth: number | null;
  }

  const PANEL_PADDING = 12;
  const LEFT_LABEL_GUTTER = 96;
  const RIGHT_LABEL_GUTTER = 116;
  const SCROLLBAR_GUTTER = 18;
  const DEFAULT_STAGE_HEIGHT = 560;
  const DEFAULT_STAGE_WIDTH = 1400;

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
  let currentPanel = $state.raw<NormalizedWellPanelModel | null>(null);
  let currentProbe = $state.raw<WellCorrelationProbe | null>(null);
  let currentViewport = $state.raw<WellCorrelationViewport | null>(null);
  let lastPanel: WellCorrelationPanelChartProps["panel"] = null;
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | null = null;
  let lastPanPoint = $state.raw<PanDragPoint | null>(null);
  let stageWidthPx = $state(DEFAULT_STAGE_WIDTH);
  let stageElement = $state.raw<HTMLDivElement | null>(null);
  let hostElement = $state.raw<HTMLDivElement | null>(null);
  let scrollViewportElement = $state.raw<HTMLDivElement | null>(null);
  let scrollbarDrag = $state.raw<ScrollbarDragState | null>(null);
  let debugSessionId = 0;
  let debugStartedAtMs = 0;
  let firstViewportAtMs: number | null = null;
  let overlayReadyAtMs: number | null = null;
  let syncCount = 0;
  let lastSyncDurationMs: number | null = null;
  let totalSyncDurationMs = 0;
  let scrollLeft = 0;
  let rendererDebug: CorrelationRendererDebugState = {
    renderCount: 0,
    lastRenderMs: null,
    baseChanged: null,
    overlayDraw: null,
    contentWidth: null,
    viewportWidth: null
  };
  let callbacksEnabled = false;
  const probeInsetPx = resolveProbePanelPresentation("light", "standard").frame.insetPx;

  let safeScale = $derived(Number.isFinite(stageScale) && stageScale > 0 ? stageScale : 1);
  let stageHeightPx = $derived(Math.max(1, Math.round(DEFAULT_STAGE_HEIGHT * safeScale)));
  let overlayViewport = $derived(currentViewport ?? viewport ?? null);
  let chromeModel = $derived.by(() => {
    if (!currentPanel || !overlayViewport) {
      return null;
    }
    return buildWellCorrelationChromeModel(currentPanel, overlayViewport, Math.max(1, stageWidthPx), stageHeightPx);
  });
  let contentWidthPx = $derived(chromeModel?.layout.contentWidth ?? Math.max(1, stageWidthPx - SCROLLBAR_GUTTER));
  let plotTopPx = $derived(chromeModel?.layout.plotRect.y ?? PANEL_PADDING + 28);
  let plotLeftPx = $derived(chromeModel?.layout.plotRect.x ?? PANEL_PADDING + LEFT_LABEL_GUTTER);
  let plotBottomPx = $derived(
    chromeModel ? stageHeightPx - chromeModel.layout.plotRect.y - chromeModel.layout.plotRect.height : PANEL_PADDING
  );
  let plotRightPx = $derived(PANEL_PADDING + RIGHT_LABEL_GUTTER + SCROLLBAR_GUTTER);
  let requestedTool = $derived(resolveRequestedTool());
  let hostCursor = $derived.by(() => {
    if (activeDragKind === "pan") {
      return "grabbing";
    }
    if (requestedTool === "pan") {
      return "grab";
    }
    return null;
  });
  let verticalScrollbarState = $derived.by(() => {
    if (!currentPanel || !currentViewport) {
      return null;
    }
    const fullSpan = Math.max(1e-6, currentPanel.depthDomain.end - currentPanel.depthDomain.start);
    const viewportSpan = Math.max(1e-6, currentViewport.depthEnd - currentViewport.depthStart);
    const zoomed = viewportSpan < fullSpan;
    const size = `${Math.min(100, (viewportSpan / fullSpan) * 100)}%`;
    const availableSpan = Math.max(1e-6, fullSpan - viewportSpan);
    const start = zoomed ? `${((currentViewport.depthStart - currentPanel.depthDomain.start) / availableSpan) * 100}%` : "0%";
    return {
      start,
      size,
      zoomed
    };
  });

  function nowMs(): number {
    return typeof performance !== "undefined" ? performance.now() : Date.now();
  }

  function resetDebugState(): void {
    debugSessionId = Math.floor(Date.now() + Math.random() * 1000);
    debugStartedAtMs = nowMs();
    firstViewportAtMs = null;
    overlayReadyAtMs = null;
    syncCount = 0;
    lastSyncDurationMs = null;
    totalSyncDurationMs = 0;
    scrollLeft = scrollViewportElement?.scrollLeft ?? 0;
    rendererDebug = {
      renderCount: 0,
      lastRenderMs: null,
      baseChanged: null,
      overlayDraw: null,
      contentWidth: null,
      viewportWidth: null
    };
  }

  function buildDebugSnapshot(reason: string): WellCorrelationDebugSnapshot | null {
    if (!debugStartedAtMs) {
      return null;
    }
    return {
      sessionId: debugSessionId,
      reason,
      mount: {
        startedAtMs: debugStartedAtMs,
        firstViewportAtMs,
        overlayReadyAtMs,
        ageMs: nowMs() - debugStartedAtMs
      },
      sync: {
        count: syncCount,
        lastDurationMs: lastSyncDurationMs,
        totalDurationMs: totalSyncDurationMs
      },
      overlay: {
        normalizeCount: 0,
        lastNormalizeMs: null,
        layoutCount: chromeModel ? 1 : 0,
        lastLayoutMs: null,
        viewportReady: Boolean(overlayViewport),
        columns: chromeModel?.columns.length ?? 0,
        clipWidth: chromeModel?.layout.viewportWidth ?? null
      },
      renderer: {
        renderCount: rendererDebug.renderCount,
        lastRenderMs: rendererDebug.lastRenderMs,
        baseChanged: rendererDebug.baseChanged,
        overlayDraw: rendererDebug.overlayDraw,
        contentWidth: rendererDebug.contentWidth,
        viewportWidth: rendererDebug.viewportWidth
      },
      state: {
        wells: currentPanel?.wells.length ?? 0,
        viewport: currentViewport,
        probeTrackId: currentProbe?.trackId ?? null,
        scrollLeft
      }
    };
  }

  function attachStage(element: HTMLDivElement): () => void {
    stageElement = element;
    stageWidthPx = Math.max(1, Math.round(element.clientWidth || DEFAULT_STAGE_WIDTH));
    return () => {
      if (stageElement === element) {
        stageElement = null;
      }
    };
  }

  function mountChartHost(element: HTMLDivElement): () => void {
    const activeController = new WellCorrelationController(new WellCorrelationCanvasRenderer({ axisChrome: "none" }));
    controller = activeController;
    currentPanel = null;
    currentProbe = null;
    currentViewport = null;
    lastViewportKey = "";
    lastProbeKey = "";
    lastInteractionKey = "";
    lastInteractionStateKey = "";
    callbacksEnabled = false;
    resetDebugState();

    syncController(activeController);

    const unsubscribeStateChange = activeController.onStateChange((state) => {
      currentPanel = state.panel;

      const nextViewportKey = JSON.stringify(state.viewport);
      if (nextViewportKey !== lastViewportKey) {
        lastViewportKey = nextViewportKey;
        currentViewport = state.viewport ? { ...state.viewport } : null;
        if (currentViewport && firstViewportAtMs === null) {
          firstViewportAtMs = nowMs() - debugStartedAtMs;
        }
        if (callbacksEnabled) {
          onViewportChange?.({
            chartId,
            viewport: currentViewport
          });
        }
      }

      const nextProbeKey = JSON.stringify(state.probe);
      if (nextProbeKey !== lastProbeKey) {
        lastProbeKey = nextProbeKey;
        currentProbe = state.probe ? { ...state.probe } : null;
        if (callbacksEnabled) {
          onProbeChange?.({
            chartId,
            probe: currentProbe
          });
        }
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
        if (callbacksEnabled && nextInteractionState.tool !== requestedTool) {
          onInteractionStateChange?.(nextInteractionState);
        }
      }
    });

    const unsubscribeInteractionEvent = activeController.onInteractionEvent((event) => {
      const nextInteractionKey = JSON.stringify(event);
      if (nextInteractionKey !== lastInteractionKey) {
        lastInteractionKey = nextInteractionKey;
        if (callbacksEnabled) {
          onInteractionEvent?.({
            chartId,
            event
          });
        }
      }
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
    const onRenderDebug = (event: Event) => {
      const detail = (event as CustomEvent<{
        renderMs: number;
        baseChanged: boolean;
        overlayDraw: boolean;
        contentWidth: number;
        viewportWidth: number;
      }>).detail;
      rendererDebug = {
        renderCount: rendererDebug.renderCount + 1,
        lastRenderMs: detail.renderMs,
        baseChanged: detail.baseChanged,
        overlayDraw: detail.overlayDraw,
        contentWidth: detail.contentWidth,
        viewportWidth: detail.viewportWidth
      };
    };

    activeController.mount(element);
    element.addEventListener("pointerdown", onPointerDown);
    element.addEventListener("pointermove", onPointerMove);
    element.addEventListener("pointerup", onPointerUp);
    element.addEventListener("pointercancel", onPointerCancel);
    element.addEventListener("pointerleave", onPointerLeave);
    element.addEventListener("focus", onFocus);
    element.addEventListener("blur", onBlur);
    element.addEventListener("keydown", onKeyDown);
    element.addEventListener("wheel", onWheel, { passive: false });
    element.addEventListener("ophiolite-charts:correlation-render-debug", onRenderDebug);
    callbacksEnabled = true;

    return () => {
      unsubscribeInteractionEvent();
      unsubscribeStateChange();
      element.removeEventListener("pointerdown", onPointerDown);
      element.removeEventListener("pointermove", onPointerMove);
      element.removeEventListener("pointerup", onPointerUp);
      element.removeEventListener("pointercancel", onPointerCancel);
      element.removeEventListener("pointerleave", onPointerLeave);
      element.removeEventListener("focus", onFocus);
      element.removeEventListener("blur", onBlur);
      element.removeEventListener("keydown", onKeyDown);
      element.removeEventListener("wheel", onWheel);
      element.removeEventListener("ophiolite-charts:correlation-render-debug", onRenderDebug);
      if (controller === activeController) {
        controller = null;
      }
      currentPanel = null;
      currentProbe = null;
      currentViewport = null;
      activeDragKind = null;
      activePointerId = null;
      lastPanPoint = null;
      scrollbarDrag = null;
      activeController.dispose();
      debugStartedAtMs = 0;
    };
  }

  onMount(() => {
    const element = hostElement;
    if (!element) {
      return;
    }
    return mountChartHost(element);
  });

  $effect(() => {
    if (!controller) {
      return;
    }
    syncController(controller);
  });

  $effect(() => {
    if (chromeModel && overlayReadyAtMs === null && debugStartedAtMs) {
      overlayReadyAtMs = nowMs() - debugStartedAtMs;
    }
  });

  export function getDebugSnapshot(): WellCorrelationDebugSnapshot | null {
    return buildDebugSnapshot("manual");
  }

  export function fitToData(): void {
    controller?.fitToData();
  }

  export function setViewport(nextViewport: NonNullable<WellCorrelationPanelChartProps["viewport"]>): void {
    viewport = nextViewport;
    controller?.setViewport(nextViewport);
  }

  export function zoomBy(factor: number): void {
    controller?.zoomVertical(factor);
  }

  export function panBy(deltaDepth: number): void {
    controller?.panVertical(deltaDepth);
  }

  function syncController(activeController: WellCorrelationController): void {
    const controllerState = activeController.getState();
    const panelChanged = panel !== lastPanel;
    const shouldReset = resetToken !== lastResetToken || panelChanged || (panel !== null && controllerState.panel === null);
    lastResetToken = resetToken;

    if (shouldReset) {
      untrack(() => {
        activeController.setPanel(panel);
      });
      lastPanel = panel;
    }

    const nextViewport = viewport;
    if (panel && nextViewport) {
      untrack(() => {
        activeController.setViewport(nextViewport);
      });
    }

    untrack(() => {
      applyTool(activeController, requestedTool);
    });
  }

  function runControllerSync(activeController: WellCorrelationController, reason: string): void {
    try {
      const syncStart = nowMs();
      syncController(activeController);
      syncCount += 1;
      lastSyncDurationMs = nowMs() - syncStart;
      totalSyncDurationMs += lastSyncDurationMs;
    } catch (error) {
      console.error(`WellCorrelationPanelChart sync failed during ${reason}.`, error);
    }
  }

  function applyTool(activeController: WellCorrelationController, tool: "pointer" | "crosshair" | "pan"): void {
    const interactionState = activeController.getState().interactions;
    const nextPrimaryMode = tool === "pan" ? "panZoom" : "cursor";
    if (interactionState.primaryMode !== nextPrimaryMode) {
      activeController.setPrimaryMode(nextPrimaryMode);
    }
    const enabled = interactionState.modifiers.includes("crosshair");
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
        panByScreenDelta(event.clientX - lastPanPoint.clientX, event.clientY - lastPanPoint.clientY);
      }
      lastPanPoint = {
        clientX: event.clientX,
        clientY: event.clientY
      };
      return;
    }
    const point = pointerPoint(event, element);
    controller.updatePointer(point.x, point.y, Math.max(1, stageWidthPx), stageHeightPx);
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
    if (requestedTool === "pan") {
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

    if (event.shiftKey && scrollViewportElement) {
      scrollViewportElement.scrollLeft += event.deltaY + event.deltaX;
      event.preventDefault();
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      const point = pointerPoint(event, element);
      const panelDepth = controller.getPanelDepthAtViewY(point.y, Math.max(1, stageWidthPx), stageHeightPx);
      if (panelDepth !== null) {
        controller.zoomVerticalAround(panelDepth, event.deltaY < 0 ? 1.12 : 0.89);
        event.preventDefault();
      }
      return;
    }

    controller.panVertical(event.deltaY * 0.35);
    event.preventDefault();
  }

  function handleScrollViewportScroll(): void {
    scrollLeft = scrollViewportElement?.scrollLeft ?? 0;
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
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
    };
  }

  function panByScreenDelta(deltaX: number, deltaY: number): void {
    if (!controller) {
      return;
    }
    const state = controller.getState();
    if (!state.panel || !state.viewport) {
      return;
    }
    if (scrollViewportElement) {
      scrollViewportElement.scrollLeft -= deltaX;
    }
    const depthSpan = state.viewport.depthEnd - state.viewport.depthStart;
    const depthDelta = (deltaY / Math.max(1, stageHeightPx)) * depthSpan;
    if (depthDelta !== 0) {
      controller.panVertical(-depthDelta);
    }
  }

  function handleVerticalScrollbarPointerDown(event: PointerEvent): void {
    const element = event.currentTarget;
    if (
      !controller ||
      !currentPanel ||
      !currentViewport ||
      !(element instanceof HTMLDivElement)
    ) {
      return;
    }

    const thumb = element.querySelector<HTMLElement>(".ophiolite-charts-scrollbar-thumb-vertical");
    const thumbRect = thumb?.getBoundingClientRect();
    scrollbarDrag = {
      pointerId: event.pointerId,
      offsetPx:
        thumbRect && event.target instanceof HTMLElement && event.target.closest(".ophiolite-charts-scrollbar-thumb-vertical")
          ? event.clientY - thumbRect.top
          : (thumbRect?.height ?? 18) / 2
    };
    element.setPointerCapture(event.pointerId);
    updateVerticalScrollbarViewport(element, event.clientY, scrollbarDrag.offsetPx);
    event.preventDefault();
  }

  function handleVerticalScrollbarPointerMove(event: PointerEvent): void {
    const element = event.currentTarget;
    if (
      !scrollbarDrag ||
      scrollbarDrag.pointerId !== event.pointerId ||
      !(element instanceof HTMLDivElement)
    ) {
      return;
    }
    updateVerticalScrollbarViewport(element, event.clientY, scrollbarDrag.offsetPx);
    event.preventDefault();
  }

  function handleVerticalScrollbarPointerUp(event: PointerEvent): void {
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    if (scrollbarDrag?.pointerId === event.pointerId) {
      scrollbarDrag = null;
    }
    if (element.hasPointerCapture(event.pointerId)) {
      element.releasePointerCapture(event.pointerId);
    }
  }

  function updateVerticalScrollbarViewport(element: HTMLDivElement, clientY: number, offsetPx: number): void {
    if (!controller || !currentPanel || !currentViewport) {
      return;
    }
    const trackRect = element.getBoundingClientRect();
    const fullStart = currentPanel.depthDomain.start;
    const fullEnd = currentPanel.depthDomain.end;
    const fullSpan = fullEnd - fullStart;
    const viewportSpan = currentViewport.depthEnd - currentViewport.depthStart;
    if (fullSpan <= viewportSpan) {
      return;
    }
    const thumbHeight = Math.max(18, trackRect.height * (viewportSpan / Math.max(1e-6, fullSpan)));
    const available = Math.max(1, trackRect.height - thumbHeight);
    const top = clamp(clientY - trackRect.top - offsetPx, 0, available);
    const depthStart = fullStart + (top / available) * (fullSpan - viewportSpan);
    controller.setViewport({
      depthStart,
      depthEnd: depthStart + viewportSpan
    });
  }

  function releasePointerCapture(element: HTMLDivElement, pointerId: number): void {
    if (activePointerId === pointerId) {
      activePointerId = null;
    }
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  }

  function correlationProbeRows(): Array<{ label: string; value: string }> {
    if (!currentProbe) {
      return [];
    }

    return [
      { label: "well", value: currentProbe.wellName },
      { label: "track", value: currentProbe.trackTitle },
      { label: "panel", value: currentProbe.panelDepth.toFixed(1) },
      { label: "native", value: currentProbe.nativeDepth.toFixed(1) },
      {
        label: "value",
        value: currentProbe.markerName ?? (currentProbe.value?.toFixed(3) ?? "n/a")
      }
    ];
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }
</script>

<div class="ophiolite-charts-correlation-shell">
  <div
    class="ophiolite-charts-correlation-stage"
    bind:this={stageElement}
    style:height={`${stageHeightPx}px`}
    style:--ophiolite-charts-plot-top={`${plotTopPx}px`}
    style:--ophiolite-charts-plot-right={`${plotRightPx}px`}
    style:--ophiolite-charts-plot-bottom={`${plotBottomPx}px`}
    style:--ophiolite-charts-plot-left={`${plotLeftPx}px`}
    {@attach attachStage}
  >
    <div
      class="ophiolite-charts-correlation-scroll-viewport"
      bind:this={scrollViewportElement}
      onscroll={handleScrollViewportScroll}
      style:right={`${SCROLLBAR_GUTTER}px`}
    >
      <div
        class="ophiolite-charts-correlation-scroll-content"
        style:width={`${contentWidthPx}px`}
        style:height={`${stageHeightPx}px`}
      >
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="ophiolite-charts-correlation-host"
          tabindex="0"
          role="application"
          aria-label="Well correlation panel"
          aria-busy={loading}
          bind:this={hostElement}
          style:width={`${contentWidthPx}px`}
          style:height={`${stageHeightPx}px`}
          style:cursor={hostCursor ?? undefined}
        ></div>
        <WellCorrelationAxisOverlay model={chromeModel} stageHeight={stageHeightPx} zIndex={1} />
      </div>
    </div>

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

    {#if currentProbe && !loading && !errorMessage && currentPanel}
      <ProbePanel
        theme="light"
        size="standard"
        right={`${SCROLLBAR_GUTTER + PANEL_PADDING + probeInsetPx}px`}
        bottom={`${PANEL_PADDING + probeInsetPx}px`}
        rows={correlationProbeRows()}
      />
    {/if}

    {#if verticalScrollbarState && !loading && !errorMessage && currentPanel}
      <div
        class="ophiolite-charts-scrollbar ophiolite-charts-scrollbar-vertical"
        class:ophiolite-charts-scrollbar-active={verticalScrollbarState.zoomed}
        class:ophiolite-charts-scrollbar-dragging={scrollbarDrag !== null}
        style:top={`${plotTopPx}px`}
        style:bottom={`${plotBottomPx}px`}
        style:width={`${SCROLLBAR_GUTTER}px`}
        onpointerdown={handleVerticalScrollbarPointerDown}
        onpointermove={handleVerticalScrollbarPointerMove}
        onpointerup={handleVerticalScrollbarPointerUp}
        onpointercancel={handleVerticalScrollbarPointerUp}
        aria-hidden="true"
      >
        <div
          class="ophiolite-charts-scrollbar-thumb ophiolite-charts-scrollbar-thumb-vertical"
          style:top={verticalScrollbarState.start}
          style:height={verticalScrollbarState.size}
        ></div>
      </div>
    {/if}
  </div>
</div>

<style>
  .ophiolite-charts-correlation-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 320px;
    background: #efe8db;
  }

  .ophiolite-charts-correlation-stage {
    position: relative;
    width: 100%;
    min-height: 320px;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-correlation-scroll-viewport {
    position: absolute;
    inset: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
  }

  .ophiolite-charts-correlation-scroll-content {
    position: relative;
    min-height: 100%;
  }

  .ophiolite-charts-correlation-host {
    position: absolute;
    inset: 0;
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
    z-index: 5;
  }

  .ophiolite-charts-overlay-error {
    color: #8a2e2a;
  }

  .ophiolite-charts-chart-anchor {
    position: absolute;
    z-index: 4;
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

  .ophiolite-charts-scrollbar {
    position: absolute;
    right: 0;
    pointer-events: auto;
    touch-action: none;
    background: rgba(228, 236, 241, 0.92);
    box-shadow: inset 0 0 0 1px rgba(176, 212, 238, 0.68);
    cursor: grab;
    z-index: 3;
  }

  .ophiolite-charts-scrollbar-thumb {
    position: absolute;
    background: linear-gradient(180deg, rgba(245, 249, 252, 0.96), rgba(190, 208, 219, 0.94));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.72),
      0 0 0 1px rgba(69, 93, 112, 0.2);
  }

  .ophiolite-charts-scrollbar-thumb-vertical {
    left: 4px;
    right: 4px;
    min-height: 18px;
  }

  .ophiolite-charts-scrollbar-dragging {
    cursor: grabbing;
  }

  .ophiolite-charts-scrollbar-active .ophiolite-charts-scrollbar-thumb {
    background: linear-gradient(180deg, rgba(186, 215, 232, 0.94), rgba(149, 186, 208, 0.94));
  }
</style>
