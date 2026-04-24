<svelte:options runes={true} />

<script lang="ts">
  import {
    formatSeismicCssFont,
    formatSeismicAxisValue,
    isArbitrarySeismicSection,
    resolveProbePanelPresentation,
    resolveSeismicPresentationProfile
  } from "@ophiolite/charts-core";
  import { getChartDefinition } from "@ophiolite/charts-data-models";
  import type {
    ChartRendererTelemetryEvent,
    SectionHorizonOverlay,
    SectionScalarOverlay,
    SectionWellOverlay
  } from "@ophiolite/charts-data-models";
  import { SeismicViewerController } from "@ophiolite/charts-domain";
  import { MockCanvasRenderer, PLOT_MARGIN, getPlotRect } from "@ophiolite/charts-renderer";
  import ProbePanel from "./ProbePanel.svelte";
  import SeismicAxisOverlay from "./SeismicAxisOverlay.svelte";
  import { emitRendererStatusForChart } from "./renderer-status";
  import { resolveSeismicStageSize, scaleSeismicStageSize } from "./seismic-stage";
  import {
    decodeSectionView,
    canReuseSectionViewport,
    shouldIgnoreExternalSectionViewport,
    mergeDisplayTransform
  } from "./contracts";
  import {
    SEISMIC_CHART_INTERACTION_CAPABILITIES,
    type SeismicChartInteractionState,
    type SeismicProbe,
    type SeismicSectionChartProps,
    type SeismicViewport
  } from "./types";

  type ScrollbarAxis = "horizontal" | "vertical";
  const EMPTY_SECTION_SCALAR_OVERLAYS: readonly SectionScalarOverlay[] = [];
  const EMPTY_SECTION_HORIZONS: readonly SectionHorizonOverlay[] = [];
  const EMPTY_SECTION_WELL_OVERLAYS: readonly SectionWellOverlay[] = [];
  const BROWSE_AXIS_LABEL: Record<"inline" | "xline", string> = {
    inline: "Inline",
    xline: "Xline"
  };
  const ANALYSIS_KIND_LABEL = {
    "amplitude-spectrum": "Amplitude spectrum",
    "amplitude-distribution": "Amplitude distribution"
  } as const;
  const ANALYSIS_KIND_ORDER = ["amplitude-spectrum", "amplitude-distribution"] as const;
  const ANALYSIS_SELECTION_MODE_LABEL = {
    "whole-section": "Section",
    viewport: "View"
  } as const;

  interface ScrollbarDragState {
    axis: ScrollbarAxis;
    pointerId: number;
    offsetPx: number;
    totalSpan: number;
    visibleSpan: number;
  }

  const seismicPresentation = resolveSeismicPresentationProfile("standard");
  const seismicOverlayFont = formatSeismicCssFont(seismicPresentation.typography.overlay);
  const chartDefinition = getChartDefinition("seismic-section");

  let {
    chartId,
    viewId,
    section = null,
    secondarySection = null,
    dataSource = null,
    renderer = undefined,
    sectionScalarOverlays = EMPTY_SECTION_SCALAR_OVERLAYS,
    sectionHorizons = EMPTY_SECTION_HORIZONS,
    sectionWellOverlays = EMPTY_SECTION_WELL_OVERLAYS,
    viewport = null,
    displayTransform = undefined,
    interactions = undefined,
    browse = undefined,
    analysis = undefined,
    compareMode = "single",
    splitPosition = 0.5,
    crosshairEnabled = true,
    primaryMode = "cursor",
    loading = false,
    loadingMessage = "Loading section...",
    emptyMessage = "No section selected.",
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
    onDataSourceStateChange,
    onRendererStatusChange,
    onRendererTelemetry,
    onSplitPositionChange
  }: SeismicSectionChartProps = $props();

  let controller: SeismicViewerController | null = null;
  let currentProbe = $state.raw<SeismicProbe | null>(null);
  let currentViewport = $state.raw<SeismicViewport | null>(null);
  let lastSection = $state.raw<typeof section>(null);
  let lastSecondarySection = $state.raw<typeof secondarySection>(null);
  let lastSectionScalarOverlays: readonly SectionScalarOverlay[] = EMPTY_SECTION_SCALAR_OVERLAYS;
  let lastSectionHorizons: readonly SectionHorizonOverlay[] = EMPTY_SECTION_HORIZONS;
  let lastSectionWellOverlays: readonly SectionWellOverlay[] = EMPTY_SECTION_WELL_OVERLAYS;
  let lastDataSource = $state.raw<typeof dataSource>(null);
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let lastRendererStatusKey = "";
  let lastRendererTelemetryEvent = $state.raw<ChartRendererTelemetryEvent | null>(null);
  let ignoredExternalViewportKey: string | null = null;
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | "zoomRect" | null = null;
  let lastPanPoint: { x: number; y: number } | null = null;
  let splitDragPointerId: number | null = null;
  let scrollbarDrag = $state.raw<ScrollbarDragState | null>(null);
  let rendererErrorMessage = $state<string | null>(null);
  let shellElement: HTMLDivElement | null = null;
  let lastRequestedTool = resolveRequestedTool();
  let effectiveTool = $state(lastRequestedTool);
  let resolvedDisplayTransform = $derived(mergeDisplayTransform(section, displayTransform));
  let decodedSectionPayload = $derived(section ? decodeSectionView(section) : null);
  let sectionMetrics = $derived(
    decodedSectionPayload
      ? {
          traces: decodedSectionPayload.dimensions.traces,
          samples: decodedSectionPayload.dimensions.samples
        }
      : null
  );
  const seismicProbePanelInset = resolveProbePanelPresentation("light", "standard").frame.insetPx;
  let stageSize = $derived(
    scaleSeismicStageSize(
      resolveSeismicStageSize("section", sectionMetrics?.traces, sectionMetrics?.samples, resolvedDisplayTransform.renderMode),
      stageScale
    )
  );
  let browseRequestContext = $derived.by(() => {
    if (
      !browse?.enabled ||
      !browse.current ||
      !browse.onRequest ||
      !section ||
      loading ||
      errorMessage ||
      rendererErrorMessage ||
      !decodedSectionPayload ||
      isArbitrarySeismicSection(decodedSectionPayload)
    ) {
      return null;
    }

    return {
      current: browse.current,
      canStepBackward: browse.canStepBackward === true && !browse.pending,
      canStepForward: browse.canStepForward === true && !browse.pending,
      canSwitchAxis: browse.canSwitchAxis !== false && !browse.pending
    };
  });
  let browseChromeVisible = $derived(Boolean(browseRequestContext && browse?.showChrome !== false));
  let browseCurrentLabel = $derived(
    browseRequestContext
      ? `${BROWSE_AXIS_LABEL[browseRequestContext.current.axis]} ${formatSeismicAxisValue(browseRequestContext.current.value)}`
      : ""
  );
  let analysisRequestContext = $derived.by(() => {
    if (!analysis?.enabled || !analysis.onRequest || !decodedSectionPayload || loading || errorMessage || rendererErrorMessage) {
      return null;
    }

    const availableKinds = ANALYSIS_KIND_ORDER.filter((kind) =>
      kind === "amplitude-spectrum" ? analysis.spectrumEnabled !== false : analysis.distributionEnabled !== false
    );
    if (availableKinds.length === 0) {
      return null;
    }

    const selectionModes = (analysis.selectionModes?.length
      ? analysis.selectionModes
      : [analysis.selectionMode ?? "whole-section"]) as readonly ("whole-section" | "viewport")[];
    const selectionMode = selectionModes.includes(analysis.selectionMode ?? "whole-section")
      ? (analysis.selectionMode ?? "whole-section")
      : selectionModes[0] ?? "whole-section";
    const canRequestSelection = selectionMode === "whole-section" || currentViewport !== null;

    return {
      current: {
        axis: decodedSectionPayload.axis,
        index: decodedSectionPayload.coordinate.index,
        value: decodedSectionPayload.coordinate.value
      },
      selectionMode,
      selectionModes,
      canRequestSelection,
      availableKinds,
      openKinds: new Set(analysis.openKinds ?? []),
      pendingKinds: new Set(analysis.pendingKinds ?? [])
    };
  });
  let analysisChromeVisible = $derived(Boolean(analysisRequestContext && analysis?.showChrome !== false));
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
  let scrollbarState = $derived.by(() => {
    if (!sectionMetrics || !currentViewport) {
      return null;
    }

    const totalTraces = Math.max(1, sectionMetrics.traces);
    const totalSamples = Math.max(1, sectionMetrics.samples);
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
    const activeController = new SeismicViewerController(new MockCanvasRenderer({ axisChrome: "none" }), {
      chartId,
      viewId,
      dataSource,
      onDataSourceStateChange: (state) => {
        onDataSourceStateChange?.({
          chartId,
          viewId,
          state
        });
      }
    });
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
    const onContextMenu = (event: MouseEvent) => {
      handleContextMenu(event);
    };

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
          detail: "Seismic section wrapper observed a controller initialization failure.",
          timestampMs: performance.now()
        });
      }
      console.error("SeismicSectionChart initialization failed.", error);
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
        restoreRendererTelemetry("render");
      } catch (error) {
        handleRendererTelemetry({
          kind: "frame-failed",
          phase: "render",
          backend: null,
          recoverable: true,
          message: error instanceof Error ? error.message : String(error),
          detail: "Seismic section wrapper observed a controller sync failure.",
          timestampMs: performance.now()
        });
        console.error("SeismicSectionChart sync failed.", error);
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
    if (controller && section) {
      controller.setSection(decodeSectionView(section));
      applyDisplayProps(controller);
    }
  }

  export function setViewport(nextViewport: NonNullable<SeismicSectionChartProps["viewport"]>): void {
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

  export function setSplitRatio(nextSplitPosition: number): void {
    splitPosition = nextSplitPosition;
    controller?.setSplitPosition(nextSplitPosition);
  }

  function syncController(activeController: SeismicViewerController, forceReset = false): void {
    emitRendererStatus();
    const requestedTool = resolveRequestedTool();
    if (requestedTool !== lastRequestedTool) {
      lastRequestedTool = requestedTool;
      effectiveTool = requestedTool;
    }

    const sectionChanged = section !== lastSection;
    const secondarySectionChanged = secondarySection !== lastSecondarySection;
    const sectionScalarOverlaysChanged = sectionScalarOverlays !== lastSectionScalarOverlays;
    const sectionHorizonsChanged = sectionHorizons !== lastSectionHorizons;
    const sectionWellOverlaysChanged = sectionWellOverlays !== lastSectionWellOverlays;
    const canReuseViewportAcrossSections = canReuseSectionViewport(lastSection, section);
    const shouldReset =
      forceReset ||
      resetToken !== lastResetToken ||
      (sectionChanged && !canReuseViewportAcrossSections);
    const externalViewportKey = viewport ? `${viewId}:${JSON.stringify(viewport)}` : null;
    const shouldIgnoreExternalViewport = shouldIgnoreExternalSectionViewport(
      lastSection,
      section,
      externalViewportKey,
      ignoredExternalViewportKey
    );

    lastResetToken = resetToken;

    if (dataSource !== lastDataSource) {
      lastDataSource = dataSource;
      activeController.setDataSource(dataSource ?? null);
    }
    activeController.setDataSourceStateListener(
      onDataSourceStateChange
        ? (state) => {
            onDataSourceStateChange({
              chartId,
              viewId,
              state
            });
          }
        : null
    );

    if (section && (sectionChanged || forceReset)) {
      const previousViewport = activeController.getState().viewport;
      activeController.setSection(decodeSectionView(section));
      if (!shouldReset && previousViewport) {
        activeController.setViewport(previousViewport);
      }
      lastSection = section;
    } else if (!section) {
      lastSection = null;
      ignoredExternalViewportKey = null;
    }

    if ((secondarySectionChanged || forceReset) && section) {
      lastSecondarySection = secondarySection;
      activeController.setSecondarySection(secondarySection ? decodeSectionView(secondarySection) : null);
    } else if (!section) {
      lastSecondarySection = null;
      activeController.setSecondarySection(null);
    }

    if (sectionScalarOverlaysChanged || forceReset) {
      lastSectionScalarOverlays = sectionScalarOverlays;
      activeController.setSectionScalarOverlays(sectionScalarOverlays);
    }

    if (sectionHorizonsChanged || forceReset) {
      lastSectionHorizons = sectionHorizons;
      activeController.setSectionHorizonOverlays(sectionHorizons);
    }

    if (sectionWellOverlaysChanged || forceReset) {
      lastSectionWellOverlays = sectionWellOverlays;
      activeController.setSectionWellOverlays(sectionWellOverlays);
    }

    applyDisplayProps(activeController);
    activeController.setComparisonMode(compareMode);
    activeController.setSplitPosition(splitPosition);

    if (viewport && section && !shouldIgnoreExternalViewport) {
      ignoredExternalViewportKey = null;
      currentViewport = viewport;
      activeController.setViewport(viewport);
    } else if (viewport && shouldIgnoreExternalViewport) {
      ignoredExternalViewportKey = externalViewportKey;
    } else if (!section) {
      currentViewport = null;
      ignoredExternalViewportKey = null;
    }

    applyTool(activeController, effectiveTool);
  }

  function emitRendererStatus(): void {
    lastRendererStatusKey = emitRendererStatusForChart(
      chartDefinition.id,
      {
        chartId,
        viewId,
        renderer: rendererErrorMessage ? { ...(renderer ?? {}), runtimeErrorMessage: rendererErrorMessage } : renderer,
        telemetryEvent: lastRendererTelemetryEvent
      },
      lastRendererStatusKey,
      onRendererStatusChange
    );
  }

  function handleRendererTelemetry(event: ChartRendererTelemetryEvent): void {
    lastRendererTelemetryEvent = { ...event };
    if (event.kind === "mount-failed" || event.kind === "frame-failed" || event.kind === "context-lost") {
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

  function restoreRendererTelemetry(phase: ChartRendererTelemetryEvent["phase"]): void {
    if (
      lastRendererTelemetryEvent?.kind !== "mount-failed" &&
      lastRendererTelemetryEvent?.kind !== "frame-failed" &&
      lastRendererTelemetryEvent?.kind !== "context-lost"
    ) {
      return;
    }
    handleRendererTelemetry({
      kind: "context-restored",
      phase,
      backend: lastRendererTelemetryEvent.backend,
      recoverable: true,
      message: "Renderer recovered after a transient failure.",
      detail: "Seismic section wrapper observed a successful controller sync after a prior renderer failure.",
      timestampMs: performance.now()
    });
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

  function requestBrowseStep(direction: -1 | 1): void {
    if (!browseRequestContext) {
      return;
    }

    if ((direction < 0 && !browseRequestContext.canStepBackward) || (direction > 0 && !browseRequestContext.canStepForward)) {
      return;
    }

    browse?.onRequest?.({
      kind: "step",
      direction,
      current: browseRequestContext.current,
      viewport: currentViewport ? { ...currentViewport } : null,
      preserveViewport: true
    });
  }

  function requestBrowseAxisSwitch(axis: "inline" | "xline"): void {
    if (!browseRequestContext || !browseRequestContext.canSwitchAxis || browseRequestContext.current.axis === axis) {
      return;
    }

    browse?.onRequest?.({
      kind: "switch-axis",
      axis,
      current: browseRequestContext.current,
      viewport: currentViewport ? { ...currentViewport } : null,
      preserveViewport: true
    });
  }

  function requestAnalysis(kind: "amplitude-spectrum" | "amplitude-distribution"): void {
    if (!analysisRequestContext || !analysisRequestContext.availableKinds.includes(kind)) {
      return;
    }

    if (analysisRequestContext.pendingKinds.has(kind) || !analysisRequestContext.canRequestSelection) {
      return;
    }

    const selection =
      analysisRequestContext.selectionMode === "viewport" && currentViewport
        ? {
            kind: "viewport" as const,
            viewport: { ...currentViewport }
          }
        : ({
            kind: "whole-section" as const
          });

    analysis?.onRequest?.({
      kind,
      selection,
      current: analysisRequestContext.current,
      viewport: currentViewport ? { ...currentViewport } : null
    });
  }

  function requestAnalysisSelectionMode(mode: "whole-section" | "viewport"): void {
    if (!analysisRequestContext || analysisRequestContext.selectionMode === mode) {
      return;
    }

    analysis?.onSelectionModeChange?.(mode);
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!controller || splitDragPointerId !== null) {
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
    controller.updatePointer(
      point.x,
      point.y,
      element.clientWidth,
      element.clientHeight
    );
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!controller || splitDragPointerId !== null) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    if (event.button !== 0) {
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
    if (!controller || splitDragPointerId !== null) {
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
    splitDragPointerId = null;
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

    if (!activeDragKind && !scrollbarDrag && browseRequestContext) {
      if (event.key === "ArrowLeft") {
        requestBrowseStep(-1);
        event.preventDefault();
        return;
      }

      if (event.key === "ArrowRight") {
        requestBrowseStep(1);
        event.preventDefault();
        return;
      }
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

  function handleSplitDividerPointerDown(event: PointerEvent): void {
    if (
      event.button !== 0 ||
      !controller ||
      compareMode !== "split" ||
      resolvedDisplayTransform.renderMode !== "heatmap" ||
      !section ||
      !secondarySection
    ) {
      return;
    }

    const divider = event.currentTarget;
    if (!(divider instanceof HTMLDivElement)) {
      return;
    }

    splitDragPointerId = event.pointerId;
    divider.setPointerCapture(event.pointerId);
    updateSplitPositionFromEvent(event);
    event.preventDefault();
    event.stopPropagation();
  }

  function handleSplitDividerPointerMove(event: PointerEvent): void {
    if (splitDragPointerId !== event.pointerId) {
      return;
    }

    updateSplitPositionFromEvent(event);
    event.preventDefault();
    event.stopPropagation();
  }

  function handleSplitDividerPointerUp(event: PointerEvent): void {
    if (splitDragPointerId !== event.pointerId) {
      return;
    }

    const divider = event.currentTarget;
    if (divider instanceof HTMLDivElement && divider.hasPointerCapture(event.pointerId)) {
      divider.releasePointerCapture(event.pointerId);
    }

    splitDragPointerId = null;
    event.preventDefault();
    event.stopPropagation();
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

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }

  function updateSplitPositionFromEvent(event: PointerEvent): void {
    if (!shellElement) {
      return;
    }

    const plotRect = getPlotRect(shellElement.clientWidth, shellElement.clientHeight);
    const rect = shellElement.getBoundingClientRect();
    const ratio = clamp((event.clientX - rect.left - plotRect.x) / Math.max(1, plotRect.width), 0.05, 0.95);
    splitPosition = ratio;
    controller?.setSplitPosition(ratio);
    onSplitPositionChange?.(ratio);
  }

  function handleScrollbarPointerDown(axis: ScrollbarAxis, event: PointerEvent): void {
    if (event.button !== 0 || !controller || !section || !currentViewport) {
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

  function sectionProbeRows(): Array<{ label: string; value: string }> {
    if (!currentProbe) {
      return [];
    }

    const rows = [
      {
        label: "trace",
        value: `${currentProbe.traceIndex} (${currentProbe.traceCoordinate.toFixed(1)})`
      }
    ];

    if (currentProbe.inlineCoordinate !== null && currentProbe.inlineCoordinate !== undefined) {
      rows.push({
        label: "IL",
        value: currentProbe.inlineCoordinate.toFixed(1)
      });
    }

    if (currentProbe.xlineCoordinate !== null && currentProbe.xlineCoordinate !== undefined) {
      rows.push({
        label: "XL",
        value: currentProbe.xlineCoordinate.toFixed(1)
      });
    }

    rows.push(
      {
        label: "sample",
        value: `${currentProbe.sampleIndex} (${currentProbe.sampleValue.toFixed(1)})`
      },
      {
        label: "amplitude",
        value: currentProbe.amplitude.toFixed(4)
      }
    );

    return rows;
  }

  function getScrollbarMetrics(axis: ScrollbarAxis): {
    trackStart: number;
    trackLength: number;
    totalSpan: number;
    visibleSpan: number;
    start: number;
  } | null {
    if (!sectionMetrics || !currentViewport || !shellElement) {
      return null;
    }

    const plotRect = getPlotRect(shellElement.clientWidth, shellElement.clientHeight);
    const shellRect = shellElement.getBoundingClientRect();

    if (axis === "horizontal") {
      return {
        trackStart: shellRect.left + plotRect.x,
        trackLength: plotRect.width,
        totalSpan: Math.max(1, sectionMetrics.traces),
        visibleSpan: Math.max(1, currentViewport.traceEnd - currentViewport.traceStart),
        start: currentViewport.traceStart
      };
    }

    return {
      trackStart: shellRect.top + plotRect.y,
      trackLength: plotRect.height,
      totalSpan: Math.max(1, sectionMetrics.samples),
      visibleSpan: Math.max(1, currentViewport.sampleEnd - currentViewport.sampleStart),
      start: currentViewport.sampleStart
    };
  }

  function updateScrollbarViewport(axis: ScrollbarAxis, pointerPosition: number, drag: ScrollbarDragState): void {
    if (!controller || !section || !currentViewport) {
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
    const nextStart =
      maxThumbStartPx === 0 ? 0 : Math.round((thumbStartPx / maxThumbStartPx) * maxStart);

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
      bind:this={shellElement}
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
        aria-label="Seismic section chart"
        aria-busy={loading}
        style:cursor={hostCursor ?? undefined}
        {@attach attachChartHost}
      ></div>
      <SeismicAxisOverlay
        section={decodedSectionPayload}
        viewport={overlayViewport}
        renderMode={resolvedDisplayTransform.renderMode}
        stageWidth={stageSize.width}
        stageHeight={stageSize.height}
      />
      {#if loading}
        <div class="ophiolite-charts-overlay">{loadingMessage}</div>
      {:else if errorMessage || rendererErrorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">
          {errorMessage ?? rendererErrorMessage}
        </div>
      {:else if !section}
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
      {#if browseChromeVisible || analysisChromeVisible}
        <div class="ophiolite-charts-seismic-tools">
          {#if browseChromeVisible && browseRequestContext}
            <div class="ophiolite-charts-seismic-browse" data-pending={browse?.pending ? "true" : "false"}>
              <div class="ophiolite-charts-seismic-browse-row">
                <button
                  type="button"
                  class="ophiolite-charts-seismic-browse-button"
                  disabled={!browseRequestContext.canStepBackward}
                  onclick={() => requestBrowseStep(-1)}
                  aria-label={`Browse previous ${browseRequestContext.current.axis}`}
                >
                  Prev
                </button>
                <div class="ophiolite-charts-seismic-browse-current">
                  <span class="ophiolite-charts-seismic-browse-current-label">{browseCurrentLabel}</span>
                  {#if browse?.pending}
                    <span class="ophiolite-charts-seismic-browse-status">Loading</span>
                  {/if}
                </div>
                <button
                  type="button"
                  class="ophiolite-charts-seismic-browse-button"
                  disabled={!browseRequestContext.canStepForward}
                  onclick={() => requestBrowseStep(1)}
                  aria-label={`Browse next ${browseRequestContext.current.axis}`}
                >
                  Next
                </button>
              </div>
              <div class="ophiolite-charts-seismic-browse-row ophiolite-charts-seismic-browse-axis-row">
                <button
                  type="button"
                  class={[
                    "ophiolite-charts-seismic-browse-axis-button",
                    browseRequestContext.current.axis === "inline" && "ophiolite-charts-seismic-browse-axis-button-active"
                  ]}
                  aria-pressed={browseRequestContext.current.axis === "inline"}
                  disabled={browseRequestContext.current.axis === "inline" || !browseRequestContext.canSwitchAxis}
                  onclick={() => requestBrowseAxisSwitch("inline")}
                >
                  Inline
                </button>
                <button
                  type="button"
                  class={[
                    "ophiolite-charts-seismic-browse-axis-button",
                    browseRequestContext.current.axis === "xline" && "ophiolite-charts-seismic-browse-axis-button-active"
                  ]}
                  aria-pressed={browseRequestContext.current.axis === "xline"}
                  disabled={browseRequestContext.current.axis === "xline" || !browseRequestContext.canSwitchAxis}
                  onclick={() => requestBrowseAxisSwitch("xline")}
                >
                  Xline
                </button>
              </div>
            </div>
          {/if}
          {#if analysisChromeVisible && analysisRequestContext}
            <div class="ophiolite-charts-seismic-analysis">
              {#if analysisRequestContext.selectionModes.length > 1}
                <div class="ophiolite-charts-seismic-analysis-selection">
                  {#each analysisRequestContext.selectionModes as mode (mode)}
                    <button
                      type="button"
                      class={[
                        "ophiolite-charts-seismic-analysis-selection-button",
                        analysisRequestContext.selectionMode === mode &&
                          "ophiolite-charts-seismic-analysis-selection-button-active"
                      ]}
                      aria-pressed={analysisRequestContext.selectionMode === mode}
                      onclick={() => requestAnalysisSelectionMode(mode)}
                    >
                      {ANALYSIS_SELECTION_MODE_LABEL[mode]}
                    </button>
                  {/each}
                </div>
              {/if}
              {#if analysisRequestContext.availableKinds.includes("amplitude-spectrum")}
                <button
                  type="button"
                  class={[
                    "ophiolite-charts-seismic-analysis-button",
                    analysisRequestContext.openKinds.has("amplitude-spectrum") &&
                      "ophiolite-charts-seismic-analysis-button-active"
                  ]}
                  disabled={
                    analysisRequestContext.pendingKinds.has("amplitude-spectrum") ||
                    !analysisRequestContext.canRequestSelection
                  }
                  onclick={() => requestAnalysis("amplitude-spectrum")}
                  aria-label={ANALYSIS_KIND_LABEL["amplitude-spectrum"]}
                  title={ANALYSIS_KIND_LABEL["amplitude-spectrum"]}
                >
                  <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8">
                    <path d="M4 18.5V13.5" />
                    <path d="M8 18.5V8.5" />
                    <path d="M12 18.5V5.5" />
                    <path d="M16 18.5V10.5" />
                    <path d="M20 18.5V7.5" />
                    <path d="M3 18.5h18" />
                  </svg>
                </button>
              {/if}
              {#if analysisRequestContext.availableKinds.includes("amplitude-distribution")}
                <button
                  type="button"
                  class={[
                    "ophiolite-charts-seismic-analysis-button",
                    analysisRequestContext.openKinds.has("amplitude-distribution") &&
                      "ophiolite-charts-seismic-analysis-button-active"
                  ]}
                  disabled={
                    analysisRequestContext.pendingKinds.has("amplitude-distribution") ||
                    !analysisRequestContext.canRequestSelection
                  }
                  onclick={() => requestAnalysis("amplitude-distribution")}
                  aria-label={ANALYSIS_KIND_LABEL["amplitude-distribution"]}
                  title={ANALYSIS_KIND_LABEL["amplitude-distribution"]}
                >
                  <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8">
                    <path d="M3 18.5h18" />
                    <path d="M6 18.5V14.5" />
                    <path d="M10 18.5V8.5" />
                    <path d="M14 18.5V5.5" />
                    <path d="M18 18.5V11.5" />
                  </svg>
                </button>
              {/if}
            </div>
          {/if}
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
      {#if currentProbe && !loading && !errorMessage && !rendererErrorMessage && section}
        <ProbePanel
          theme="light"
          size="standard"
          right={`${PLOT_MARGIN.right + seismicProbePanelInset}px`}
          bottom={`${PLOT_MARGIN.bottom + seismicProbePanelInset}px`}
          rows={sectionProbeRows()}
        />
      {/if}
      {#if compareMode === "split" && resolvedDisplayTransform.renderMode === "heatmap" && section && secondarySection}
        <div
          class="ophiolite-charts-split-divider"
          style:left={`calc(${PLOT_MARGIN.left}px + ((100% - ${PLOT_MARGIN.left + PLOT_MARGIN.right}px) * ${splitPosition}))`}
          style:top={`${PLOT_MARGIN.top}px`}
          style:bottom={`${PLOT_MARGIN.bottom}px`}
          onpointerdown={handleSplitDividerPointerDown}
          onpointermove={handleSplitDividerPointerMove}
          onpointerup={handleSplitDividerPointerUp}
          onpointercancel={handleSplitDividerPointerUp}
          aria-hidden="true"
        >
          <div class="ophiolite-charts-split-divider-line"></div>
          <div class="ophiolite-charts-split-divider-handle"></div>
        </div>
      {/if}
      {#if scrollbarState && !loading && !errorMessage && !rendererErrorMessage && section}
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

  .ophiolite-charts-seismic-tools {
    position: absolute;
    top: calc(var(--ophiolite-charts-plot-top) + var(--ophiolite-charts-overlay-pad));
    right: calc(var(--ophiolite-charts-plot-right) + var(--ophiolite-charts-overlay-pad));
    z-index: 5;
    display: grid;
    gap: 8px;
    justify-items: end;
  }

  .ophiolite-charts-seismic-browse {
    min-width: 184px;
    display: grid;
    gap: 6px;
    padding: 8px;
    border: 1px solid rgba(176, 212, 238, 0.74);
    border-radius: 10px;
    background: rgba(250, 252, 253, 0.96);
    box-shadow:
      0 10px 22px rgba(42, 64, 84, 0.16),
      inset 0 0 0 1px rgba(255, 255, 255, 0.72);
    backdrop-filter: blur(10px);
  }

  .ophiolite-charts-seismic-analysis {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    padding: 8px;
    border: 1px solid rgba(176, 212, 238, 0.74);
    border-radius: 10px;
    background: rgba(250, 252, 253, 0.96);
    box-shadow:
      0 10px 22px rgba(42, 64, 84, 0.16),
      inset 0 0 0 1px rgba(255, 255, 255, 0.72);
    backdrop-filter: blur(10px);
  }

  .ophiolite-charts-seismic-browse-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 6px;
  }

  .ophiolite-charts-seismic-browse-axis-row {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .ophiolite-charts-seismic-browse-button,
  .ophiolite-charts-seismic-browse-axis-button {
    min-width: 0;
    min-height: 30px;
    padding: 0 10px;
    border: 1px solid rgba(153, 186, 208, 0.76);
    border-radius: 7px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(231, 239, 245, 0.98));
    color: #284052;
    font: var(--ophiolite-chart-overlay-font, 500 14px/1.4 sans-serif);
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
    cursor: pointer;
  }

  .ophiolite-charts-seismic-analysis-button {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(153, 186, 208, 0.76);
    border-radius: 8px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(231, 239, 245, 0.98));
    color: #284052;
    cursor: pointer;
  }

  .ophiolite-charts-seismic-analysis-selection {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-right: 2px;
  }

  .ophiolite-charts-seismic-analysis-selection-button {
    min-height: 28px;
    padding: 0 9px;
    border: 1px solid rgba(153, 186, 208, 0.76);
    border-radius: 7px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(231, 239, 245, 0.98));
    color: #284052;
    font: var(--ophiolite-chart-overlay-font, 500 14px/1.4 sans-serif);
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
    cursor: pointer;
  }

  .ophiolite-charts-seismic-browse-button:disabled,
  .ophiolite-charts-seismic-browse-axis-button:disabled,
  .ophiolite-charts-seismic-analysis-button:disabled,
  .ophiolite-charts-seismic-analysis-selection-button:disabled {
    cursor: default;
    opacity: 0.54;
  }

  .ophiolite-charts-seismic-browse-axis-button-active {
    background: linear-gradient(180deg, rgba(193, 224, 239, 0.98), rgba(166, 203, 223, 0.98));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.7),
      0 0 0 1px rgba(89, 123, 145, 0.12);
  }

  .ophiolite-charts-seismic-analysis-button-active {
    background: linear-gradient(180deg, rgba(193, 224, 239, 0.98), rgba(166, 203, 223, 0.98));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.7),
      0 0 0 1px rgba(89, 123, 145, 0.12);
  }

  .ophiolite-charts-seismic-analysis-selection-button-active {
    background: linear-gradient(180deg, rgba(193, 224, 239, 0.98), rgba(166, 203, 223, 0.98));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.7),
      0 0 0 1px rgba(89, 123, 145, 0.12);
  }

  .ophiolite-charts-seismic-browse-current {
    min-width: 0;
    display: grid;
    justify-items: center;
    gap: 3px;
    padding: 0 6px;
    color: #284052;
    text-align: center;
  }

  .ophiolite-charts-seismic-browse-current-label {
    font: var(--ophiolite-chart-overlay-font, 500 14px/1.4 sans-serif);
    font-size: 12px;
    font-weight: 700;
    line-height: 1.15;
    white-space: nowrap;
  }

  .ophiolite-charts-seismic-browse-status {
    font: var(--ophiolite-chart-overlay-font, 500 14px/1.4 sans-serif);
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: rgba(70, 98, 119, 0.84);
  }

  .ophiolite-charts-split-divider {
    position: absolute;
    width: 18px;
    margin-left: -9px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: col-resize;
    touch-action: none;
    z-index: 2;
  }

  .ophiolite-charts-split-divider-line {
    width: 2px;
    height: 100%;
    background: rgba(126, 159, 181, 0.72);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.45),
      0 0 12px rgba(42, 64, 84, 0.14);
  }

  .ophiolite-charts-split-divider-handle {
    position: absolute;
    width: 14px;
    height: 36px;
    border: 1px solid rgba(176, 212, 238, 0.78);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.96);
    box-shadow: 0 4px 10px rgba(42, 64, 84, 0.14);
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
