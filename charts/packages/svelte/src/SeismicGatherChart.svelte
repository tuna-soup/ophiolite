<svelte:options runes={true} />

<script lang="ts">
  import {
    formatSeismicCssFont,
    resolveProbePanelPresentation,
    resolveSeismicPresentationProfile
  } from "@ophiolite/charts-core";
  import { gatherToSectionPayload } from "@ophiolite/charts-data-models";
  import type { ChartRendererTelemetryEvent } from "@ophiolite/charts-data-models";
  import { SeismicViewerController } from "@ophiolite/charts-domain";
  import { MockCanvasRenderer, PLOT_MARGIN, getPlotRect } from "@ophiolite/charts-renderer";
  import ProbePanel from "./ProbePanel.svelte";
  import SeismicAxisOverlay from "./SeismicAxisOverlay.svelte";
  import { emitRendererStatusForChart } from "./renderer-status";
  import { resolveSeismicStageSize, scaleSeismicStageSize } from "./seismic-stage";
  import {
    decodeGatherView,
    isCompatibleGatherIdentity,
    mergeGatherDisplayTransform
  } from "./contracts";
  import {
    SEISMIC_CHART_INTERACTION_CAPABILITIES,
    type SeismicChartInteractionState,
    type SeismicGatherChartProps,
    type SeismicProbe,
    type SeismicViewport
  } from "./types";

  type ScrollbarAxis = "horizontal" | "vertical";

  interface ScrollbarDragState {
    axis: ScrollbarAxis;
    pointerId: number;
    offsetPx: number;
    totalSpan: number;
    visibleSpan: number;
  }

  const seismicPresentation = resolveSeismicPresentationProfile("standard");
  const seismicOverlayFont = formatSeismicCssFont(seismicPresentation.typography.overlay);

  let {
    chartId,
    viewId,
    gather = null,
    renderer = undefined,
    viewport = null,
    displayTransform = undefined,
    interactions = undefined,
    crosshairEnabled = true,
    primaryMode = "cursor",
    loading = false,
    emptyMessage = "No gather selected.",
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
    onInteractionChange,
    onInteractionStateChange,
    onInteractionEvent,
    onRendererStatusChange,
    onRendererTelemetry
  }: SeismicGatherChartProps = $props();

  let controller: SeismicViewerController | null = null;
  let currentProbe = $state.raw<SeismicProbe | null>(null);
  let currentViewport = $state.raw<SeismicViewport | null>(null);
  let lastGather = $state.raw<typeof gather>(null);
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let lastRendererStatusKey = "";
  let lastRendererTelemetryEvent = $state.raw<ChartRendererTelemetryEvent | null>(null);
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | "zoomRect" | null = null;
  let lastPanPoint: { x: number; y: number } | null = null;
  let rendererErrorMessage = $state<string | null>(null);
  let scrollbarDrag = $state.raw<ScrollbarDragState | null>(null);
  let hostElement = $state.raw<HTMLDivElement | null>(null);
  let lastRequestedTool = resolveRequestedTool();
  let effectiveTool = $state(lastRequestedTool);
  let resolvedDisplayTransform = $derived(mergeGatherDisplayTransform(gather, displayTransform));
  let decodedGatherModel = $derived(gather ? decodeGatherView(gather) : null);
  const seismicProbePanelInset = resolveProbePanelPresentation("light", "standard").frame.insetPx;
  let stageSize = $derived(
    scaleSeismicStageSize(
      resolveSeismicStageSize(
        "gather",
        decodedGatherModel?.dimensions.traces,
        decodedGatherModel?.dimensions.samples,
        resolvedDisplayTransform.renderMode
      ),
      stageScale
    )
  );
  let decodedGatherPayload = $derived(decodedGatherModel ? gatherToSectionPayload(decodedGatherModel) : null);
  let overlayViewport = $derived(
    currentViewport
      ? {
          traceStart: currentViewport.traceStart,
          traceEnd: currentViewport.traceEnd,
          sampleStart: currentViewport.sampleStart,
          sampleEnd: currentViewport.sampleEnd
        }
      : null
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
  $effect(() => {
    lastRendererStatusKey = emitRendererStatusForChart(
      "seismic-gather",
      {
        chartId,
        viewId,
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
      viewId,
      event: { ...event }
    });
  }
  let probeHorizontalLabel = $derived(horizontalLabel(decodedGatherModel?.gatherAxisKind ?? null));
  let scrollbarState = $derived.by(() => {
    if (!decodedGatherModel || !currentViewport) {
      return null;
    }

    const totalTraces = Math.max(1, decodedGatherModel.dimensions.traces);
    const totalSamples = Math.max(1, decodedGatherModel.dimensions.samples);
    const traceStart = clamp(currentViewport.traceStart, 0, totalTraces - 1);
    const traceEnd = clamp(currentViewport.traceEnd, traceStart + 1, totalTraces);
    const sampleStart = clamp(currentViewport.sampleStart, 0, totalSamples - 1);
    const sampleEnd = clamp(currentViewport.sampleEnd, sampleStart + 1, totalSamples);

    return {
      horizontalStart: `${(traceStart / totalTraces) * 100}%`,
      horizontalSize: `${((traceEnd - traceStart) / totalTraces) * 100}%`,
      verticalStart: `${(sampleStart / totalSamples) * 100}%`,
      verticalSize: `${((sampleEnd - sampleStart) / totalSamples) * 100}%`,
      horizontalZoomed: traceEnd - traceStart < totalTraces,
      verticalZoomed: sampleEnd - sampleStart < totalSamples
    };
  });

  function attachChartHost(element: HTMLDivElement): () => void {
    rendererErrorMessage = null;
    lastRendererTelemetryEvent = null;
    const activeController = new SeismicViewerController(new MockCanvasRenderer({ axisChrome: "none" }));
    controller = activeController;
    currentProbe = null;

    const unsubscribeStateChange = activeController.onStateChange((state) => {
      if (!state.viewport) {
        currentViewport = null;
        currentProbe = null;
        return;
      }

      const nextViewport = {
        chartId,
        viewId,
        viewport: state.viewport
      };
      const nextViewportKey = JSON.stringify(nextViewport);
      if (nextViewportKey !== lastViewportKey) {
        lastViewportKey = nextViewportKey;
        currentViewport = nextViewport.viewport;
        onViewportChange?.(nextViewport);
      }

      const nextProbe = {
        chartId,
        viewId,
        probe: state.probe
      };
      const nextProbeKey = JSON.stringify(nextProbe);
      if (nextProbeKey !== lastProbeKey) {
        lastProbeKey = nextProbeKey;
        currentProbe = nextProbe.probe;
        onProbeChange?.(nextProbe);
      }

      const nextTool = controllerModeToTool(
        state.interactions.primaryMode,
        state.interactions.modifiers.includes("crosshair")
      );
      const nextInteraction = {
        chartId,
        viewId,
        primaryMode: toLegacyPrimaryMode(state.interactions.primaryMode),
        crosshairEnabled: state.interactions.modifiers.includes("crosshair"),
        tool: nextTool
      };
      const nextInteractionKey = JSON.stringify(nextInteraction);
      if (nextInteractionKey !== lastInteractionKey) {
        lastInteractionKey = nextInteractionKey;
        onInteractionChange?.(nextInteraction);
      }

      const nextInteractionState = createInteractionState(nextTool);
      const nextInteractionStateKey = JSON.stringify(nextInteractionState);
      if (nextInteractionStateKey !== lastInteractionStateKey) {
        lastInteractionStateKey = nextInteractionStateKey;
        onInteractionStateChange?.(nextInteractionState);
      }
    });

    const unsubscribeInteractionEvent = activeController.onInteractionEvent((event) => {
      onInteractionEvent?.({
        chartId,
        viewId,
        event
      });
    });
    const unsubscribeRendererTelemetry = activeController.onRendererTelemetry((event) => {
      handleRendererTelemetry(event);
    });

    const resizeObserver = new ResizeObserver(() => {
      syncController(activeController);
    });

    const onPointerDown = (event: PointerEvent) => handlePointerDown(event);
    const onPointerMove = (event: PointerEvent) => handlePointerMove(event);
    const onPointerUp = (event: PointerEvent) => handlePointerUp(event);
    const onPointerCancel = (event: PointerEvent) => handlePointerCancel(event);
    const onPointerLeave = () => handlePointerLeave();
    const onFocus = () => handleFocus();
    const onBlur = () => handleBlur();
    const onKeyDown = (event: KeyboardEvent) => handleKeyDown(event);
    const onContextMenu = (event: MouseEvent) => handleContextMenu(event);

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
          detail: "Seismic gather wrapper observed a controller initialization failure.",
          timestampMs: performance.now()
        });
      }
      console.error("SeismicGatherChart initialization failed.", error);
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
    element.addEventListener("contextmenu", onContextMenu);

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
          detail: "Seismic gather wrapper observed a controller sync failure.",
          timestampMs: performance.now()
        });
        console.error("SeismicGatherChart sync failed.", error);
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
      element.removeEventListener("contextmenu", onContextMenu);
      if (controller === activeController) {
        controller = null;
      }
      currentProbe = null;
      activeController.dispose();
    };
  }

  export function fitToData(): void {
    controller?.fitToData();
  }

  export function resetView(): void {
    if (controller && gather) {
      controller.setSection(gatherToSectionPayload(decodeGatherView(gather)));
      applyDisplayProps(controller);
    }
  }

  export function setViewport(nextViewport: NonNullable<SeismicGatherChartProps["viewport"]>): void {
    viewport = nextViewport;
    currentViewport = nextViewport;
    if (controller) {
      controller.setViewport(nextViewport);
    }
  }

  export function zoomBy(factor: number): void {
    controller?.zoom(factor);
  }

  export function panBy(deltaTrace: number, deltaSample: number): void {
    controller?.pan(deltaTrace, deltaSample);
  }

  function syncController(activeController: SeismicViewerController, forceReset = false): void {
    const requestedTool = resolveRequestedTool();
    if (requestedTool !== lastRequestedTool) {
      lastRequestedTool = requestedTool;
      effectiveTool = requestedTool;
    }

    const gatherChanged = gather !== lastGather;
    const shouldReset =
      forceReset ||
      resetToken !== lastResetToken ||
      (gatherChanged && !isCompatibleGatherIdentity(lastGather, gather));

    lastResetToken = resetToken;

    if (gather && (gatherChanged || forceReset)) {
      const previousViewport = activeController.getState().viewport;
      activeController.setSection(gatherToSectionPayload(decodeGatherView(gather)));
      if (!shouldReset && previousViewport) {
        activeController.setViewport(previousViewport);
      }
      lastGather = gather;
    } else if (!gather) {
      lastGather = null;
    }

    applyDisplayProps(activeController);

    if (viewport) {
      currentViewport = viewport;
      activeController.setViewport(viewport);
    } else if (!gather) {
      currentViewport = null;
    }

    applyTool(activeController, effectiveTool);
  }

  function applyTool(activeController: SeismicViewerController, tool: "pointer" | "crosshair" | "pan"): void {
    activeController.setPrimaryMode(tool === "pan" ? "panZoom" : "cursor");
    const enabled = activeController.getState().interactions.modifiers.includes("crosshair");
    if (enabled !== (tool === "crosshair")) {
      activeController.toggleCrosshair();
    }
  }

  function applyDisplayProps(activeController: SeismicViewerController): void {
    activeController.setDisplayTransform({
      gain: resolvedDisplayTransform.gain,
      clipMin: resolvedDisplayTransform.clipMin,
      clipMax: resolvedDisplayTransform.clipMax,
      renderMode: resolvedDisplayTransform.renderMode,
      colormap: resolvedDisplayTransform.colormap,
      polarity: resolvedDisplayTransform.polarity
    });
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    if (activeDragKind === "pan") {
      if (lastPanPoint) {
        panByScreenDelta(element, point.x - lastPanPoint.x, point.y - lastPanPoint.y);
      }
      lastPanPoint = point;
      return;
    }
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
    const point = pointerPoint(event, element);
    if (event.shiftKey) {
      activeDragKind = controller.beginZoomRect(point.x, point.y, element.clientWidth, element.clientHeight)
        ? "zoomRect"
        : null;
      return;
    }
    if (effectiveTool === "pan") {
      activeDragKind = "pan";
      lastPanPoint = point;
      controller.clearPointer();
      return;
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
    if (activeDragKind === "zoomRect") {
      const zoomed = controller.commitZoomRect(element.clientWidth, element.clientHeight);
      activeDragKind = null;
      controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
    } else if (activeDragKind === "pan") {
      activeDragKind = null;
      lastPanPoint = null;
    }
    releasePointerCapture(element, event.pointerId);
  }

  function handlePointerCancel(event: PointerEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    activeDragKind = null;
    lastPanPoint = null;
    controller.cancelInteractionSession();
    releasePointerCapture(element, event.pointerId);
  }

  function handlePointerLeave(): void {
    if (activeDragKind || scrollbarDrag) {
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
      controller.cancelInteractionSession();
      setEffectiveTool("pointer");
      event.preventDefault();
      return;
    }

    const state = controller.getState();
    if (!state.section || !state.viewport) {
      return;
    }

    const traceSpan = state.viewport.traceEnd - state.viewport.traceStart;
    const sampleSpan = state.viewport.sampleEnd - state.viewport.sampleStart;
    const canPanHorizontally = traceSpan < state.section.dimensions.traces;
    const canPanVertically = sampleSpan < state.section.dimensions.samples;
    const traceStep = Math.max(1, Math.round(traceSpan * 0.1));
    const sampleStep = Math.max(1, Math.round(sampleSpan * 0.1));

    switch (event.key) {
      case "ArrowLeft":
        if (canPanHorizontally) {
          controller.pan(-traceStep, 0);
          event.preventDefault();
        }
        break;
      case "ArrowRight":
        if (canPanHorizontally) {
          controller.pan(traceStep, 0);
          event.preventDefault();
        }
        break;
      case "ArrowUp":
        if (canPanVertically) {
          controller.pan(0, -sampleStep);
          event.preventDefault();
        }
        break;
      case "ArrowDown":
        if (canPanVertically) {
          controller.pan(0, sampleStep);
          event.preventDefault();
        }
        break;
    }
  }

  function handleContextMenu(event: MouseEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    const zoomed = controller.zoomAt(point.x, point.y, element.clientWidth, element.clientHeight, 0.7);
    if (zoomed) {
      controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
    }
    event.preventDefault();
  }

  function resolveRequestedTool(): "pointer" | "crosshair" | "pan" {
    if (interactions?.tool) {
      return interactions.tool;
    }
    if (primaryMode === "panZoom") {
      return "pan";
    }
    if (crosshairEnabled) {
      return "crosshair";
    }
    return "pointer";
  }

  function setEffectiveTool(tool: "pointer" | "crosshair" | "pan"): void {
    effectiveTool = tool;
    if (controller) {
      applyTool(controller, tool);
    }
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

  function toLegacyPrimaryMode(mode: "cursor" | "panZoom" | "zoomRect" | "topEdit" | "lassoSelect"): "cursor" | "panZoom" {
    return mode === "panZoom" ? "panZoom" : "cursor";
  }

  function createInteractionState(tool: "pointer" | "crosshair" | "pan"): SeismicChartInteractionState {
    return {
      capabilities: {
        tools: [...SEISMIC_CHART_INTERACTION_CAPABILITIES.tools],
        actions: [...SEISMIC_CHART_INTERACTION_CAPABILITIES.actions]
      },
      tool
    };
  }

  function pointerPoint(event: MouseEvent | PointerEvent, element: HTMLDivElement): { x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
    };
  }

  function panByScreenDelta(element: HTMLDivElement, deltaX: number, deltaY: number): void {
    if (!controller) {
      return;
    }
    const state = controller.getState();
    if (!state.section || !state.viewport) {
      return;
    }
    const plotRect = getPlotRect(element.clientWidth, element.clientHeight);
    const visibleTraces = Math.max(1, state.viewport.traceEnd - state.viewport.traceStart);
    const visibleSamples = Math.max(1, state.viewport.sampleEnd - state.viewport.sampleStart);
    const traceDelta = Math.round((deltaX / Math.max(1, plotRect.width)) * visibleTraces);
    const sampleDelta = Math.round((deltaY / Math.max(1, plotRect.height)) * visibleSamples);
    if (traceDelta === 0 && sampleDelta === 0) {
      return;
    }
    controller.pan(-traceDelta, -sampleDelta);
  }

  function releasePointerCapture(element: HTMLDivElement, pointerId: number): void {
    if (activePointerId === pointerId) {
      activePointerId = null;
    }
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  }

  function handleScrollbarPointerDown(axis: ScrollbarAxis, event: PointerEvent): void {
    if (event.button !== 0 || !controller || !gather || !currentViewport) {
      return;
    }
    const track = event.currentTarget;
    if (!(track instanceof HTMLDivElement)) {
      return;
    }

    const metrics = getScrollbarMetrics(axis);
    if (!metrics) {
      return;
    }

    const pointerPosition = axis === "horizontal" ? event.clientX - metrics.trackStart : event.clientY - metrics.trackStart;
    const thumbStartPx = (metrics.start / metrics.totalSpan) * metrics.trackLength;
    const thumbSizePx = (metrics.visibleSpan / metrics.totalSpan) * metrics.trackLength;
    const target = event.target;
    const clickedThumb =
      target instanceof HTMLElement && target.closest(".ophiolite-charts-scrollbar-thumb") instanceof HTMLElement;
    const offsetPx =
      clickedThumb && pointerPosition >= thumbStartPx && pointerPosition <= thumbStartPx + thumbSizePx
        ? pointerPosition - thumbStartPx
        : thumbSizePx / 2;

    scrollbarDrag = {
      axis,
      pointerId: event.pointerId,
      offsetPx,
      totalSpan: metrics.totalSpan,
      visibleSpan: metrics.visibleSpan
    };

    activeDragKind = null;
    lastPanPoint = null;
    controller.cancelInteractionSession();
    controller.clearPointer();
    track.setPointerCapture(event.pointerId);
    updateScrollbarViewport(axis, pointerPosition, scrollbarDrag);
    event.preventDefault();
    event.stopPropagation();
  }

  function handleScrollbarPointerMove(axis: ScrollbarAxis, event: PointerEvent): void {
    if (!scrollbarDrag || scrollbarDrag.axis !== axis || scrollbarDrag.pointerId !== event.pointerId) {
      return;
    }

    const metrics = getScrollbarMetrics(axis);
    if (!metrics) {
      return;
    }

    const pointerPosition = axis === "horizontal" ? event.clientX - metrics.trackStart : event.clientY - metrics.trackStart;
    updateScrollbarViewport(axis, pointerPosition, scrollbarDrag);
    event.preventDefault();
    event.stopPropagation();
  }

  function handleScrollbarPointerUp(axis: ScrollbarAxis, event: PointerEvent): void {
    if (!scrollbarDrag || scrollbarDrag.axis !== axis || scrollbarDrag.pointerId !== event.pointerId) {
      return;
    }

    const track = event.currentTarget;
    if (track instanceof HTMLDivElement && track.hasPointerCapture(event.pointerId)) {
      track.releasePointerCapture(event.pointerId);
    }

    scrollbarDrag = null;
    event.preventDefault();
    event.stopPropagation();
  }

  function gatherProbeRows(): Array<{ label: string; value: string }> {
    if (!currentProbe) {
      return [];
    }

    return [
      {
        label: probeHorizontalLabel,
        value: `${currentProbe.traceIndex} (${currentProbe.traceCoordinate.toFixed(2)})`
      },
      {
        label: "sample",
        value: `${currentProbe.sampleIndex} (${currentProbe.sampleValue.toFixed(1)})`
      },
      {
        label: "amplitude",
        value: currentProbe.amplitude.toFixed(4)
      }
    ];
  }

  function getScrollbarMetrics(axis: ScrollbarAxis): {
    trackStart: number;
    trackLength: number;
    totalSpan: number;
    visibleSpan: number;
    start: number;
  } | null {
    if (!decodedGatherModel || !currentViewport || !hostElement) {
      return null;
    }

    const plotRect = getPlotRect(hostElement.clientWidth, hostElement.clientHeight);
    const shellRect = hostElement.getBoundingClientRect();

    if (axis === "horizontal") {
      return {
        trackStart: shellRect.left + plotRect.x,
        trackLength: plotRect.width,
        totalSpan: Math.max(1, decodedGatherModel.dimensions.traces),
        visibleSpan: Math.max(1, currentViewport.traceEnd - currentViewport.traceStart),
        start: currentViewport.traceStart
      };
    }

    return {
      trackStart: shellRect.top + plotRect.y,
      trackLength: plotRect.height,
      totalSpan: Math.max(1, decodedGatherModel.dimensions.samples),
      visibleSpan: Math.max(1, currentViewport.sampleEnd - currentViewport.sampleStart),
      start: currentViewport.sampleStart
    };
  }

  function updateScrollbarViewport(axis: ScrollbarAxis, pointerPosition: number, drag: ScrollbarDragState): void {
    if (!controller || !gather || !currentViewport) {
      return;
    }

    const metrics = getScrollbarMetrics(axis);
    if (!metrics) {
      return;
    }

    const thumbSizePx = (drag.visibleSpan / drag.totalSpan) * metrics.trackLength;
    const maxThumbStartPx = Math.max(0, metrics.trackLength - thumbSizePx);
    const thumbStartPx = clamp(pointerPosition - drag.offsetPx, 0, maxThumbStartPx);
    const maxStart = Math.max(0, drag.totalSpan - drag.visibleSpan);
    const nextStart = maxThumbStartPx === 0 ? 0 : Math.round((thumbStartPx / maxThumbStartPx) * maxStart);

    const nextViewport =
      axis === "horizontal"
        ? {
            ...currentViewport,
            traceStart: nextStart,
            traceEnd: nextStart + drag.visibleSpan
          }
        : {
            ...currentViewport,
            sampleStart: nextStart,
            sampleEnd: nextStart + drag.visibleSpan
          };

    currentViewport = nextViewport;
    controller.setViewport(nextViewport);
  }

  function horizontalLabel(kind: import("@ophiolite/charts-data-models").GatherAxisKind | null): string {
    switch (kind) {
      case "angle":
        return "angle";
      case "offset":
        return "offset";
      case "azimuth":
        return "azimuth";
      case "shot":
        return "shot";
      case "receiver":
        return "receiver";
      case "cmp":
        return "cmp";
      case "trace-ordinal":
        return "trace";
      default:
        return "trace";
    }
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }
</script>

<div
  class="ophiolite-charts-svelte-chart-shell"
  style:--ophiolite-chart-shell-bg={seismicPresentation.palette.shellBackground}
  style:--ophiolite-chart-overlay-bg={seismicPresentation.palette.overlayBackground}
  style:--ophiolite-chart-overlay-text={seismicPresentation.palette.overlayText}
  style:--ophiolite-chart-overlay-error={seismicPresentation.palette.overlayError}
  style:--ophiolite-chart-overlay-font={seismicOverlayFont}
  style:--ophiolite-chart-scrollbar-track-bg={seismicPresentation.palette.scrollbarTrack}
  style:--ophiolite-chart-scrollbar-track-border={seismicPresentation.palette.scrollbarTrackBorder}
  style:--ophiolite-chart-scrollbar-thumb-start={seismicPresentation.palette.scrollbarThumbStart}
  style:--ophiolite-chart-scrollbar-thumb-end={seismicPresentation.palette.scrollbarThumbEnd}
  style:--ophiolite-chart-scrollbar-thumb-active-start={seismicPresentation.palette.scrollbarThumbActiveStart}
  style:--ophiolite-chart-scrollbar-thumb-active-end={seismicPresentation.palette.scrollbarThumbActiveEnd}
  style:--ophiolite-chart-scrollbar-thumb-inner-border={seismicPresentation.palette.scrollbarThumbInnerBorder}
  style:--ophiolite-chart-scrollbar-thumb-outer-border={seismicPresentation.palette.scrollbarThumbOuterBorder}
>
  <div class="ophiolite-charts-svelte-chart-lane">
    <div
      class="ophiolite-charts-svelte-chart-stage"
      bind:this={hostElement}
      style:width={`${stageSize.width}px`}
      style:height={`${stageSize.height}px`}
      style:--ophiolite-charts-plot-top={`${PLOT_MARGIN.top}px`}
      style:--ophiolite-charts-plot-right={`${PLOT_MARGIN.right}px`}
      style:--ophiolite-charts-plot-bottom={`${PLOT_MARGIN.bottom}px`}
      style:--ophiolite-charts-plot-left={`${PLOT_MARGIN.left}px`}
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="ophiolite-charts-svelte-chart-host"
        tabindex="0"
        role="application"
        aria-label="Seismic gather chart"
        aria-busy={loading}
        style:cursor={hostCursor ?? undefined}
        {@attach attachChartHost}
      ></div>
      <SeismicAxisOverlay
        section={decodedGatherPayload}
        viewport={overlayViewport}
        renderMode={resolvedDisplayTransform.renderMode}
        stageWidth={stageSize.width}
        stageHeight={stageSize.height}
      />
      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage || rendererErrorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">
          {errorMessage ?? rendererErrorMessage}
        </div>
      {:else if !gather}
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
      {#if currentProbe && !loading && !errorMessage && !rendererErrorMessage && gather}
        <ProbePanel
          theme="light"
          size="standard"
          right={`${PLOT_MARGIN.right + seismicProbePanelInset}px`}
          bottom={`${PLOT_MARGIN.bottom + seismicProbePanelInset}px`}
          rows={gatherProbeRows()}
        />
      {/if}
      {#if scrollbarState && !loading && !errorMessage && !rendererErrorMessage && gather}
        <div
          class="ophiolite-charts-scrollbar ophiolite-charts-scrollbar-horizontal"
          class:ophiolite-charts-scrollbar-active={scrollbarState.horizontalZoomed}
          class:ophiolite-charts-scrollbar-dragging={scrollbarDrag?.axis === "horizontal"}
          style:left={`${PLOT_MARGIN.left}px`}
          style:right={`${PLOT_MARGIN.right}px`}
          style:height={`${PLOT_MARGIN.bottom}px`}
          onpointerdown={(event) => handleScrollbarPointerDown("horizontal", event)}
          onpointermove={(event) => handleScrollbarPointerMove("horizontal", event)}
          onpointerup={(event) => handleScrollbarPointerUp("horizontal", event)}
          onpointercancel={(event) => handleScrollbarPointerUp("horizontal", event)}
          aria-hidden="true"
        >
          <div
            class="ophiolite-charts-scrollbar-thumb ophiolite-charts-scrollbar-thumb-horizontal"
            style:left={scrollbarState.horizontalStart}
            style:width={scrollbarState.horizontalSize}
          ></div>
        </div>
        <div
          class="ophiolite-charts-scrollbar ophiolite-charts-scrollbar-vertical"
          class:ophiolite-charts-scrollbar-active={scrollbarState.verticalZoomed}
          class:ophiolite-charts-scrollbar-dragging={scrollbarDrag?.axis === "vertical"}
          style:top={`${PLOT_MARGIN.top}px`}
          style:bottom={`${PLOT_MARGIN.bottom}px`}
          style:width={`${PLOT_MARGIN.right}px`}
          onpointerdown={(event) => handleScrollbarPointerDown("vertical", event)}
          onpointermove={(event) => handleScrollbarPointerMove("vertical", event)}
          onpointerup={(event) => handleScrollbarPointerUp("vertical", event)}
          onpointercancel={(event) => handleScrollbarPointerUp("vertical", event)}
          aria-hidden="true"
        >
          <div
            class="ophiolite-charts-scrollbar-thumb ophiolite-charts-scrollbar-thumb-vertical"
            style:top={scrollbarState.verticalStart}
            style:height={scrollbarState.verticalSize}
          ></div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .ophiolite-charts-svelte-chart-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 280px;
    display: flex;
    overflow-x: auto;
    overflow-y: auto;
    background: var(--ophiolite-chart-shell-bg, #f4f7f9);
  }

  .ophiolite-charts-svelte-chart-lane {
    min-width: 100%;
    min-height: 100%;
    width: max-content;
    height: max-content;
    display: grid;
    place-items: start;
  }

  .ophiolite-charts-svelte-chart-stage {
    position: relative;
    min-height: 280px;
    flex: 0 0 auto;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-svelte-chart-host {
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
    background: var(--ophiolite-chart-overlay-bg, rgba(244, 247, 249, 0.88));
    color: var(--ophiolite-chart-overlay-text, #284052);
    font: var(--ophiolite-chart-overlay-font, 500 14px/1.4 sans-serif);
    pointer-events: none;
  }

  .ophiolite-charts-overlay-error {
    color: var(--ophiolite-chart-overlay-error, #8f3c3c);
  }

  .ophiolite-charts-chart-anchor {
    position: absolute;
    z-index: 4;
    pointer-events: none;
  }

  .ophiolite-charts-chart-anchor-content {
    pointer-events: auto;
  }

  .ophiolite-charts-chart-anchor-top-center {
    top: calc(var(--ophiolite-charts-plot-top) + var(--ophiolite-charts-overlay-pad));
    left: calc(var(--ophiolite-charts-plot-left) + ((100% - var(--ophiolite-charts-plot-left) - var(--ophiolite-charts-plot-right)) / 2));
    transform: translateX(-50%);
  }

  .ophiolite-charts-chart-anchor-stage-top-left {
    top: var(--ophiolite-charts-overlay-pad);
    left: var(--ophiolite-charts-overlay-pad);
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
    pointer-events: auto;
    touch-action: none;
    background: var(--ophiolite-chart-scrollbar-track-bg, rgba(228, 236, 241, 0.92));
    box-shadow: inset 0 0 0 1px var(--ophiolite-chart-scrollbar-track-border, rgba(176, 212, 238, 0.68));
    cursor: grab;
  }

  .ophiolite-charts-scrollbar-horizontal {
    bottom: 0;
  }

  .ophiolite-charts-scrollbar-vertical {
    right: 0;
  }

  .ophiolite-charts-scrollbar-thumb {
    position: absolute;
    background: linear-gradient(
      180deg,
      var(--ophiolite-chart-scrollbar-thumb-start, rgba(245, 249, 252, 0.96)),
      var(--ophiolite-chart-scrollbar-thumb-end, rgba(190, 208, 219, 0.94))
    );
    box-shadow:
      inset 0 0 0 1px var(--ophiolite-chart-scrollbar-thumb-inner-border, rgba(255, 255, 255, 0.72)),
      0 0 0 1px var(--ophiolite-chart-scrollbar-thumb-outer-border, rgba(69, 93, 112, 0.2));
  }

  .ophiolite-charts-scrollbar-dragging {
    cursor: grabbing;
  }

  .ophiolite-charts-scrollbar-thumb-horizontal {
    top: 4px;
    bottom: 4px;
    min-width: 18px;
  }

  .ophiolite-charts-scrollbar-thumb-vertical {
    left: 4px;
    right: 4px;
    min-height: 18px;
  }

  .ophiolite-charts-scrollbar-active .ophiolite-charts-scrollbar-thumb {
    background: linear-gradient(
      180deg,
      var(--ophiolite-chart-scrollbar-thumb-active-start, rgba(186, 215, 232, 0.94)),
      var(--ophiolite-chart-scrollbar-thumb-active-end, rgba(149, 186, 208, 0.94))
    );
  }
</style>
