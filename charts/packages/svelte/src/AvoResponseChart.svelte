<svelte:options runes={true} />

<script lang="ts">
  import {
    applyViewportToAxisOverrides,
    buildCartesianTicks,
    cloneCartesianAxisOverrides,
    formatCartesianCssFont,
    formatCartesianTick,
    hitTestCartesianAxisBand,
    resolveCartesianPresentationProfile,
    resolveCartesianStageLayout,
    resolveCartesianAxisTitle,
    resolveCartesianTickCount
  } from "@ophiolite/charts-core";
  import type {
    InteractionEvent,
    InteractionTarget,
    AvoCartesianViewport,
    AvoResponseProbe,
    CartesianAxisOverrides
  } from "@ophiolite/charts-data-models";
  import ProbePanel from "./ProbePanel.svelte";
  import { scaleAvoStageSize, resolveAvoStageSize } from "./avo-stage";
  import {
    AVO_CHART_INTERACTION_CAPABILITIES,
    type AvoChartInteractionState,
    type AvoResponseChartProps
  } from "./types";

  interface ScreenPoint {
    x: number;
    y: number;
  }

  const PRESENTATION = resolveCartesianPresentationProfile("avo");
  const MARGIN = PRESENTATION.frame.plotInsets;
  const TICK_FONT = formatCartesianCssFont(PRESENTATION.typography.tick);
  const TITLE_FONT = formatCartesianCssFont(PRESENTATION.typography.title);
  const SUBTITLE_FONT = formatCartesianCssFont(PRESENTATION.typography.subtitle);
  const AXIS_LABEL_FONT = formatCartesianCssFont(PRESENTATION.typography.axisLabel);

  let {
    chartId,
    model = null,
    viewport = null,
    axisOverrides = undefined,
    interactions = undefined,
    loading = false,
    emptyMessage = "No AVO response selected.",
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
    onInteractionEvent,
    onAxisOverridesChange,
    onAxisContextRequest
  }: AvoResponseChartProps = $props();

  let host = $state.raw<HTMLDivElement | null>(null);
  let currentViewport = $state.raw<AvoCartesianViewport | null>(null);
  let currentProbe = $state.raw<AvoResponseProbe | null>(null);
  let currentAxisOverrides = $state.raw<CartesianAxisOverrides>({});
  let activePointerId = $state.raw<number | null>(null);
  let activeDragKind = $state<"pan" | "zoomRect" | null>(null);
  let lastPanPoint = $state.raw<ScreenPoint | null>(null);
  let zoomRectSession = $state.raw<{ origin: ScreenPoint; current: ScreenPoint } | null>(null);
  let lastModel = $state.raw<AvoResponseChartProps["model"]>(null);
  let lastResetToken = $state.raw<string | number | null>(null);
  let lastInteractionStateKey = "";
  let lastHoverKey = "";
  let lastAxisOverridesKey = "";
  let lastRequestedAxisOverridesKey = "";
  let stageSize = $derived(scaleAvoStageSize(resolveAvoStageSize(), stageScale));
  let layout = $derived(resolveCartesianStageLayout(stageSize.width, stageSize.height, PRESENTATION));
  let requestedTool = $derived(interactions?.tool ?? "crosshair");
  let plotClipId = $derived(`ophiolite-charts-avo-response-clip-${sanitizeDomId(chartId)}`);
  let plotWidth = $derived(layout.plotRect.width);
  let plotHeight = $derived(layout.plotRect.height);
  let xTicks = $derived(
    buildCartesianTicks(currentViewport?.xMin ?? 0, currentViewport?.xMax ?? 40, resolveCartesianTickCount(currentAxisOverrides.x))
  );
  let yTicks = $derived(
    buildCartesianTicks(currentViewport?.yMin ?? -0.3, currentViewport?.yMax ?? 0.2, resolveCartesianTickCount(currentAxisOverrides.y))
  );
  let hostCursor = $derived.by(() => {
    if (activeDragKind === "zoomRect") {
      return "crosshair";
    }
    if (requestedTool === "pan") {
      return activePointerId == null ? "grab" : "grabbing";
    }
    return "crosshair";
  });

  $effect(() => {
    const nextViewport = viewport ?? fitViewport(model);
    const shouldReset = model !== lastModel || resetToken !== lastResetToken;
    lastResetToken = resetToken;
    if (shouldReset) {
      lastModel = model;
      currentAxisOverrides = cloneCartesianAxisOverrides(axisOverrides);
      setViewportState(nextViewport, false);
      currentProbe = null;
      notifyProbeChange();
      return;
    }
    if (viewport && !sameViewport(viewport, currentViewport)) {
      setViewportState(viewport, false);
    }
  });

  $effect(() => {
    const requested = cloneCartesianAxisOverrides(axisOverrides);
    const key = JSON.stringify(requested);
    if (key === lastRequestedAxisOverridesKey) {
      return;
    }
    lastRequestedAxisOverridesKey = key;
    const baseViewport = currentViewport ?? fitViewport(model);
    const nextViewport = !viewport ? viewportFromAxisOverrides(baseViewport, requested) : baseViewport;
    currentAxisOverrides = nextViewport ? applyViewportToAxisOverrides(requested, nextViewport) : requested;
    notifyAxisOverridesChange();
    if (!viewport && nextViewport && !sameViewport(nextViewport, currentViewport)) {
      setViewportState(clampViewport(model, nextViewport), true, false);
    }
  });

  $effect(() => {
    const nextState: AvoChartInteractionState = {
      capabilities: {
        tools: [...AVO_CHART_INTERACTION_CAPABILITIES.tools],
        actions: [...AVO_CHART_INTERACTION_CAPABILITIES.actions]
      },
      tool: requestedTool
    };
    const key = JSON.stringify(nextState);
    if (key !== lastInteractionStateKey) {
      lastInteractionStateKey = key;
      onInteractionStateChange?.(nextState);
      emitInteractionEvent({
        type: "modeChange",
        primaryMode: requestedTool === "pan" ? "panZoom" : "cursor"
      });
      emitInteractionEvent({
        type: "modifierChange",
        modifier: "crosshair",
        enabled: requestedTool === "crosshair"
      });
    }
  });

  export function fitToData(): void {
    setViewportState(fitViewport(model), true);
  }

  export function setViewport(nextViewport: NonNullable<AvoResponseChartProps["viewport"]>): void {
    setViewportState(clampViewport(model, nextViewport), true);
  }

  export function zoomBy(factor: number): void {
    if (!currentViewport || factor <= 0) {
      return;
    }
    zoomAround(
      (currentViewport.xMin + currentViewport.xMax) / 2,
      (currentViewport.yMin + currentViewport.yMax) / 2,
      factor
    );
  }

  export function panBy(deltaX: number, deltaY: number): void {
    if (!currentViewport) {
      return;
    }
    setViewportState(
      clampViewport(model, {
        xMin: currentViewport.xMin + deltaX,
        xMax: currentViewport.xMax + deltaX,
        yMin: currentViewport.yMin + deltaY,
        yMax: currentViewport.yMax + deltaY
      }),
      true
    );
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!host) {
      return;
    }
    host.focus();
    if (event.button !== 0) {
      return;
    }
    if (event.shiftKey) {
      const point = pointerPoint(event);
      if (!pointInPlot(point)) {
        return;
      }
      activePointerId = event.pointerId;
      activeDragKind = "zoomRect";
      zoomRectSession = { origin: point, current: point };
      currentProbe = null;
      notifyProbeChange();
      emitHoverTarget(null);
      host.setPointerCapture(event.pointerId);
      emitInteractionEvent({
        type: "zoomRectStart",
        session: { kind: "zoomRect", origin: point, current: point }
      });
      return;
    }
    if (requestedTool !== "pan") {
      updateProbeFromPointer(event);
      return;
    }
    activePointerId = event.pointerId;
    activeDragKind = "pan";
    lastPanPoint = pointerPoint(event);
    host.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!host) {
      return;
    }
    if (activeDragKind === "zoomRect" && activePointerId === event.pointerId && zoomRectSession) {
      const point = clampPointToPlot(pointerPoint(event));
      zoomRectSession = {
        origin: zoomRectSession.origin,
        current: point
      };
      emitInteractionEvent({
        type: "zoomRectPreview",
        session: { kind: "zoomRect", origin: zoomRectSession.origin, current: point }
      });
      return;
    }
    if (requestedTool === "pan" && activePointerId === event.pointerId && lastPanPoint && currentViewport) {
      const point = pointerPoint(event);
      const deltaX = point.x - lastPanPoint.x;
      const deltaY = point.y - lastPanPoint.y;
      lastPanPoint = point;
      panByScreenDelta(deltaX, deltaY);
      return;
    }
    updateProbeFromPointer(event);
  }

  function handlePointerUp(event: PointerEvent): void {
    if (!host) {
      return;
    }
    if (activeDragKind === "zoomRect" && zoomRectSession) {
      const session = zoomRectSession;
      const nextViewport = viewportFromZoomRect(session);
      activeDragKind = null;
      zoomRectSession = null;
      releasePointerCapture(event.pointerId);
      lastPanPoint = null;
      if (nextViewport) {
        setViewportState(clampViewport(model, nextViewport), true);
        emitInteractionEvent({
          type: "zoomRectCommit",
          session: { kind: "zoomRect", origin: session.origin, current: session.current }
        });
      } else {
        emitInteractionEvent({
          type: "zoomRectCancel",
          session: { kind: "zoomRect", origin: session.origin, current: session.current }
        });
      }
      return;
    }
    activeDragKind = null;
    releasePointerCapture(event.pointerId);
    lastPanPoint = null;
  }

  function handlePointerCancel(event: PointerEvent): void {
    if (!host) {
      return;
    }
    if (activeDragKind === "zoomRect" && zoomRectSession) {
      emitInteractionEvent({
        type: "zoomRectCancel",
        session: { kind: "zoomRect", origin: zoomRectSession.origin, current: zoomRectSession.current }
      });
    }
    activeDragKind = null;
    zoomRectSession = null;
    releasePointerCapture(event.pointerId);
    lastPanPoint = null;
  }

  function handlePointerLeave(): void {
    currentProbe = null;
    notifyProbeChange();
    emitHoverTarget(null);
  }

  function handleWheel(event: WheelEvent): void {
    if (activeDragKind === "zoomRect") {
      return;
    }
    if (!currentViewport || !pointInPlot(pointerPoint(event))) {
      return;
    }
    const value = screenToValue(pointerPoint(event));
    zoomAround(value.x, value.y, event.deltaY < 0 ? 1.12 : 0.89);
    event.preventDefault();
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (!currentViewport) {
      return;
    }
    if (event.key === "Escape") {
      if (activeDragKind === "zoomRect" && zoomRectSession) {
        emitInteractionEvent({
          type: "zoomRectCancel",
          session: { kind: "zoomRect", origin: zoomRectSession.origin, current: zoomRectSession.current }
        });
      }
      activeDragKind = null;
      zoomRectSession = null;
      currentProbe = null;
      notifyProbeChange();
      emitHoverTarget(null);
      event.preventDefault();
      return;
    }

    const stepX = (currentViewport.xMax - currentViewport.xMin) * 0.08;
    const stepY = (currentViewport.yMax - currentViewport.yMin) * 0.08;

    switch (event.key) {
      case "ArrowLeft":
        panBy(-stepX, 0);
        event.preventDefault();
        break;
      case "ArrowRight":
        panBy(stepX, 0);
        event.preventDefault();
        break;
      case "ArrowUp":
        panBy(0, stepY);
        event.preventDefault();
        break;
      case "ArrowDown":
        panBy(0, -stepY);
        event.preventDefault();
        break;
    }
  }

  function updateProbeFromPointer(event: PointerEvent | WheelEvent | MouseEvent): void {
    if (!model || !currentViewport) {
      return;
    }
    const point = pointerPoint(event);
    if (!pointInPlot(point)) {
      currentProbe = null;
      notifyProbeChange();
      emitHoverTarget(null);
      return;
    }

    const probe = buildProbe(model, currentViewport, point);
    currentProbe = probe;
    notifyProbeChange();
    emitHoverTarget(
      probe
        ? {
            kind: "curve-sample",
            chartId,
            entityId: probe.seriesValues[0]?.seriesId
          }
        : null
    );
  }

  function notifyProbeChange(): void {
    onProbeChange?.({
      chartId,
      probe: currentProbe
    });
  }

  function avoResponseProbeRows(): Array<{ label: string; value: string }> {
    if (!currentProbe) {
      return [];
    }

    return [
      { label: "angle", value: `${currentProbe.angleDeg.toFixed(1)} deg` },
      ...currentProbe.seriesValues.map((entry) => ({
        label: entry.label,
        value: entry.value.toFixed(3)
      }))
    ];
  }

  function setViewportState(nextViewport: AvoCartesianViewport | null, notify = true, syncAxis = true): void {
    currentViewport = nextViewport;
    if (syncAxis) {
      syncAxisOverridesWithViewport();
    }
    if (notify) {
      onViewportChange?.({
        chartId,
        viewport: currentViewport
      });
    }
  }

  function emitHoverTarget(target: InteractionTarget | null): void {
    const key = JSON.stringify(target);
    if (key === lastHoverKey) {
      return;
    }
    lastHoverKey = key;
    emitInteractionEvent({
      type: "hoverTargetChange",
      target
    });
  }

  function emitInteractionEvent(event: InteractionEvent): void {
    onInteractionEvent?.({
      chartId,
      event
    });
  }

  function notifyAxisOverridesChange(): void {
    const key = JSON.stringify(currentAxisOverrides);
    if (key === lastAxisOverridesKey) {
      return;
    }
    lastAxisOverridesKey = key;
    onAxisOverridesChange?.({
      chartId,
      axisOverrides: currentAxisOverrides
    });
  }

  function syncAxisOverridesWithViewport(): void {
    currentAxisOverrides = applyViewportToAxisOverrides(currentAxisOverrides, currentViewport);
    notifyAxisOverridesChange();
  }

  function zoomAround(x: number, y: number, factor: number): void {
    if (!currentViewport || factor <= 0) {
      return;
    }
    const spanX = (currentViewport.xMax - currentViewport.xMin) / factor;
    const spanY = (currentViewport.yMax - currentViewport.yMin) / factor;
    const ratioX = (x - currentViewport.xMin) / Math.max(1e-6, currentViewport.xMax - currentViewport.xMin);
    const ratioY = (y - currentViewport.yMin) / Math.max(1e-6, currentViewport.yMax - currentViewport.yMin);
    setViewportState(
      clampViewport(model, {
        xMin: x - ratioX * spanX,
        xMax: x + (1 - ratioX) * spanX,
        yMin: y - ratioY * spanY,
        yMax: y + (1 - ratioY) * spanY
      }),
      true
    );
  }

  function panByScreenDelta(deltaX: number, deltaY: number): void {
    if (!currentViewport) {
      return;
    }
    const dataDeltaX = (-deltaX / Math.max(1, plotWidth)) * (currentViewport.xMax - currentViewport.xMin);
    const dataDeltaY = (deltaY / Math.max(1, plotHeight)) * (currentViewport.yMax - currentViewport.yMin);
    panBy(dataDeltaX, dataDeltaY);
  }

  function pointInPlot(point: ScreenPoint): boolean {
    return (
      point.x >= MARGIN.left &&
      point.x <= MARGIN.left + plotWidth &&
      point.y >= MARGIN.top &&
      point.y <= MARGIN.top + plotHeight
    );
  }

  function clampPointToPlot(point: ScreenPoint): ScreenPoint {
    return {
      x: clamp(point.x, MARGIN.left, MARGIN.left + plotWidth),
      y: clamp(point.y, MARGIN.top, MARGIN.top + plotHeight)
    };
  }

  function pointerPoint(event: PointerEvent | WheelEvent | MouseEvent): ScreenPoint {
    const rect = host?.getBoundingClientRect();
    if (!rect) {
      return { x: 0, y: 0 };
    }
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
    };
  }

  function screenToValue(point: ScreenPoint): { x: number; y: number } {
    const viewportState = currentViewport ?? fitViewport(model);
    if (!viewportState) {
      return { x: 0, y: 0 };
    }
    const xRatio = (point.x - MARGIN.left) / Math.max(1, plotWidth);
    const yRatio = (MARGIN.top + plotHeight - point.y) / Math.max(1, plotHeight);
    return {
      x: viewportState.xMin + clamp(xRatio, 0, 1) * (viewportState.xMax - viewportState.xMin),
      y: viewportState.yMin + clamp(yRatio, 0, 1) * (viewportState.yMax - viewportState.yMin)
    };
  }

  function releasePointerCapture(pointerId: number): void {
    if (activePointerId === pointerId) {
      activePointerId = null;
    }
    if (host?.hasPointerCapture(pointerId)) {
      host.releasePointerCapture(pointerId);
    }
  }

  function handleContextMenu(event: MouseEvent): void {
    const point = pointerPoint(event);
    const axis = hitTestCartesianAxisBand(
      point.x,
      point.y,
      { x: MARGIN.left, y: MARGIN.top, width: plotWidth, height: plotHeight },
      stageSize.width,
      stageSize.height
    );
    if (axis) {
      onAxisContextRequest?.({
        chartId,
        axis,
        trigger: "contextmenu",
        clientX: event.clientX,
        clientY: event.clientY,
        stageX: point.x,
        stageY: point.y
      });
      event.preventDefault();
      return;
    }
    if (!currentViewport || !pointInPlot(point)) {
      return;
    }
    const value = screenToValue(point);
    zoomAround(value.x, value.y, 0.7);
    updateProbeFromPointer(event);
    event.preventDefault();
  }

  function viewportFromAxisOverrides(
    source: AvoCartesianViewport | null,
    overrides: CartesianAxisOverrides
  ): AvoCartesianViewport | null {
    if (!source) {
      return null;
    }
    return {
      xMin: overrides.x?.min ?? source.xMin,
      xMax: overrides.x?.max ?? source.xMax,
      yMin: overrides.y?.min ?? source.yMin,
      yMax: overrides.y?.max ?? source.yMax
    };
  }

  function viewportFromZoomRect(session: { origin: ScreenPoint; current: ScreenPoint }): AvoCartesianViewport | null {
    const left = Math.min(session.origin.x, session.current.x);
    const right = Math.max(session.origin.x, session.current.x);
    const top = Math.min(session.origin.y, session.current.y);
    const bottom = Math.max(session.origin.y, session.current.y);
    if (right - left < 4 || bottom - top < 4) {
      return null;
    }
    const topLeft = screenToValue({ x: left, y: top });
    const bottomRight = screenToValue({ x: right, y: bottom });
    return {
      xMin: topLeft.x,
      xMax: bottomRight.x,
      yMin: bottomRight.y,
      yMax: topLeft.y
    };
  }

  function fitViewport(source: AvoResponseChartProps["model"]): AvoCartesianViewport | null {
    if (!source || source.series.length === 0) {
      return null;
    }
    let minX = source.xAxis.range?.min ?? Number.POSITIVE_INFINITY;
    let maxX = source.xAxis.range?.max ?? Number.NEGATIVE_INFINITY;
    let minY = source.yAxis.range?.min ?? Number.POSITIVE_INFINITY;
    let maxY = source.yAxis.range?.max ?? Number.NEGATIVE_INFINITY;

    for (const series of source.series) {
      for (let index = 0; index < series.incidenceAnglesDeg.length; index += 1) {
        const x = series.incidenceAnglesDeg[index] ?? 0;
        const y = series.values[index] ?? 0;
        if (Number.isFinite(x)) {
          minX = Math.min(minX, x);
          maxX = Math.max(maxX, x);
        }
        if (Number.isFinite(y)) {
          minY = Math.min(minY, y);
          maxY = Math.max(maxY, y);
        }
      }
    }

    if (!Number.isFinite(minX) || !Number.isFinite(maxX) || !Number.isFinite(minY) || !Number.isFinite(maxY)) {
      return {
        xMin: 0,
        xMax: 40,
        yMin: -0.3,
        yMax: 0.2
      };
    }

    const padX = Math.max(1, maxX - minX) * 0.05;
    const padY = Math.max(1e-3, maxY - minY) * 0.08;
    return clampViewport(source, {
      xMin: minX - padX,
      xMax: maxX + padX,
      yMin: minY - padY,
      yMax: maxY + padY
    });
  }

  function clampViewport(source: AvoResponseChartProps["model"], nextViewport: AvoCartesianViewport | null): AvoCartesianViewport | null {
    if (!source || !nextViewport) {
      return nextViewport;
    }
    const bounds = {
      xMin: source.xAxis.range?.min ?? nextViewport.xMin,
      xMax: source.xAxis.range?.max ?? nextViewport.xMax,
      yMin: source.yAxis.range?.min ?? nextViewport.yMin,
      yMax: source.yAxis.range?.max ?? nextViewport.yMax
    };
    const fullSpanX = Math.max(1e-6, bounds.xMax - bounds.xMin);
    const fullSpanY = Math.max(1e-6, bounds.yMax - bounds.yMin);
    const spanX = clamp(nextViewport.xMax - nextViewport.xMin, fullSpanX * 0.02, fullSpanX);
    const spanY = clamp(nextViewport.yMax - nextViewport.yMin, fullSpanY * 0.05, fullSpanY);
    const xMin = clamp(nextViewport.xMin, bounds.xMin, bounds.xMax - spanX);
    const yMin = clamp(nextViewport.yMin, bounds.yMin, bounds.yMax - spanY);
    return {
      xMin,
      xMax: xMin + spanX,
      yMin,
      yMax: yMin + spanY
    };
  }

  function buildProbe(
    source: NonNullable<AvoResponseChartProps["model"]>,
    viewportState: AvoCartesianViewport,
    point: ScreenPoint
  ): AvoResponseProbe | null {
    const plotRatio = clamp((point.x - MARGIN.left) / Math.max(1, plotWidth), 0, 1);
    const xValue = viewportState.xMin + plotRatio * (viewportState.xMax - viewportState.xMin);
    const anchorSeries = source.series[0];
    if (!anchorSeries || anchorSeries.incidenceAnglesDeg.length === 0) {
      return null;
    }
    const nearestIndex = nearestValueIndex(anchorSeries.incidenceAnglesDeg, xValue);
    const angleDeg = anchorSeries.incidenceAnglesDeg[nearestIndex] ?? xValue;
    const seriesValues = source.series
      .map((series) => {
        const sampleIndex = nearestValueIndex(series.incidenceAnglesDeg, angleDeg);
        const value = series.values[sampleIndex];
        if (!Number.isFinite(value)) {
          return null;
        }
        return {
          seriesId: series.id,
          interfaceId: series.interfaceId,
          label: series.label,
          color: series.color,
          value
        };
      })
      .filter((entry) => entry !== null);
    if (seriesValues.length === 0) {
      return null;
    }
    const firstValue = seriesValues[0]?.value ?? 0;
    return {
      angleDeg,
      screenX: valueToScreenX(angleDeg, viewportState),
      screenY: valueToScreenY(firstValue, viewportState),
      seriesValues
    };
  }

  function nearestValueIndex(values: ArrayLike<number>, target: number): number {
    let bestIndex = 0;
    let bestDistance = Number.POSITIVE_INFINITY;
    for (let index = 0; index < values.length; index += 1) {
      const value = values[index] ?? 0;
      const distance = Math.abs(value - target);
      if (distance < bestDistance) {
        bestDistance = distance;
        bestIndex = index;
      }
    }
    return bestIndex;
  }

  function valueToScreenX(value: number, viewportState: AvoCartesianViewport): number {
    return MARGIN.left + ((value - viewportState.xMin) / Math.max(1e-6, viewportState.xMax - viewportState.xMin)) * plotWidth;
  }

  function valueToScreenY(value: number, viewportState: AvoCartesianViewport): number {
    const ratio = (value - viewportState.yMin) / Math.max(1e-6, viewportState.yMax - viewportState.yMin);
    return MARGIN.top + plotHeight - clamp(ratio, 0, 1) * plotHeight;
  }

  function linePath(angles: Float32Array, values: Float32Array, viewportState: AvoCartesianViewport): string {
    let path = "";
    for (let index = 0; index < angles.length; index += 1) {
      const x = angles[index];
      const y = values[index];
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        continue;
      }
      path += `${path === "" ? "M" : " L"}${valueToScreenX(x, viewportState).toFixed(2)} ${valueToScreenY(y, viewportState).toFixed(2)}`;
    }
    return path;
  }

  function sanitizeDomId(value: string): string {
    return value.replace(/[^A-Za-z0-9_-]/g, "-");
  }

  function sameViewport(left: AvoCartesianViewport | null, right: AvoCartesianViewport | null): boolean {
    return (
      left?.xMin === right?.xMin &&
      left?.xMax === right?.xMax &&
      left?.yMin === right?.yMin &&
      left?.yMax === right?.yMax
    );
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }
</script>

<div class="ophiolite-charts-avo-shell">
  <div class="ophiolite-charts-avo-lane">
    <div
      class="ophiolite-charts-avo-stage"
      style:width={`${stageSize.width}px`}
      style:height={`${stageSize.height}px`}
      style:--ophiolite-charts-plot-top={`${MARGIN.top}px`}
      style:--ophiolite-charts-plot-right={`${MARGIN.right}px`}
      style:--ophiolite-charts-plot-bottom={`${MARGIN.bottom}px`}
      style:--ophiolite-charts-plot-left={`${MARGIN.left}px`}
      style:--ophiolite-charts-cartesian-tick-font={TICK_FONT}
      style:--ophiolite-charts-cartesian-title-font={TITLE_FONT}
      style:--ophiolite-charts-cartesian-subtitle-font={SUBTITLE_FONT}
      style:--ophiolite-charts-cartesian-axis-font={AXIS_LABEL_FONT}
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        bind:this={host}
        class="ophiolite-charts-avo-host"
        tabindex="0"
        role="application"
        aria-label="AVO response chart"
        aria-busy={loading}
        style:cursor={hostCursor}
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        onpointercancel={handlePointerCancel}
        onpointerleave={handlePointerLeave}
        onwheel={handleWheel}
        onkeydown={handleKeyDown}
        oncontextmenu={handleContextMenu}
      >
        <svg viewBox={`0 0 ${stageSize.width} ${stageSize.height}`} class="ophiolite-charts-avo-svg" aria-hidden="true">
          <defs>
            <clipPath id={plotClipId} clipPathUnits="userSpaceOnUse">
              <rect x={layout.plotRect.x} y={layout.plotRect.y} width={layout.plotRect.width} height={layout.plotRect.height} />
            </clipPath>
          </defs>
          <rect x="0" y="0" width={stageSize.width} height={stageSize.height} fill="#f7f8fb" />
          <rect
            x={layout.plotRect.x}
            y={layout.plotRect.y}
            width={layout.plotRect.width}
            height={layout.plotRect.height}
            fill="#ffffff"
            stroke="rgba(119, 138, 158, 0.28)"
          />
          {#if currentViewport}
            {#each xTicks as tick (tick)}
              <line
                x1={valueToScreenX(tick, currentViewport)}
                y1={layout.plotRect.y}
                x2={valueToScreenX(tick, currentViewport)}
                y2={layout.plotRect.y + layout.plotRect.height}
                stroke="rgba(130, 148, 166, 0.18)"
              />
              <text class="tick x" x={valueToScreenX(tick, currentViewport)} y={layout.xTickY}>
                {formatCartesianTick(tick, currentAxisOverrides.x?.tickFormat)}
              </text>
            {/each}
            {#each yTicks as tick (tick)}
              <line
                x1={layout.plotRect.x}
                y1={valueToScreenY(tick, currentViewport)}
                x2={layout.plotRect.x + layout.plotRect.width}
                y2={valueToScreenY(tick, currentViewport)}
                stroke="rgba(130, 148, 166, 0.18)"
              />
              <text class="tick y" x={layout.yTickX} y={valueToScreenY(tick, currentViewport) + 4}>
                {formatCartesianTick(tick, currentAxisOverrides.y?.tickFormat)}
              </text>
            {/each}
            {#if model}
              <g clip-path={`url(#${plotClipId})`}>
                {#each model.series as series (series.id)}
                  <path
                    d={linePath(series.incidenceAnglesDeg, series.values, currentViewport)}
                    fill="none"
                    stroke={series.color}
                    stroke-width="2.25"
                    stroke-dasharray={series.style === "dashed" ? "9 6" : undefined}
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                {/each}
              </g>
            {/if}
            {#if currentProbe && requestedTool === "crosshair"}
              <g clip-path={`url(#${plotClipId})`}>
                <line
                  x1={currentProbe.screenX}
                  y1={layout.plotRect.y}
                  x2={currentProbe.screenX}
                  y2={layout.plotRect.y + layout.plotRect.height}
                  stroke="rgba(64, 78, 93, 0.42)"
                  stroke-dasharray="6 5"
                />
                {#each currentProbe.seriesValues as entry (entry.seriesId)}
                  <circle
                    cx={currentProbe.screenX}
                    cy={valueToScreenY(entry.value, currentViewport)}
                    r="3"
                    fill={entry.color}
                    stroke="#ffffff"
                    stroke-width="1.2"
                  />
                {/each}
              </g>
            {/if}
            {#if zoomRectSession}
              <rect
                x={Math.max(layout.plotRect.x, Math.min(zoomRectSession.origin.x, zoomRectSession.current.x))}
                y={Math.max(layout.plotRect.y, Math.min(zoomRectSession.origin.y, zoomRectSession.current.y))}
                width={Math.max(0, Math.min(layout.plotRect.x + layout.plotRect.width, Math.max(zoomRectSession.origin.x, zoomRectSession.current.x)) - Math.max(layout.plotRect.x, Math.min(zoomRectSession.origin.x, zoomRectSession.current.x)))}
                height={Math.max(0, Math.min(layout.plotRect.y + layout.plotRect.height, Math.max(zoomRectSession.origin.y, zoomRectSession.current.y)) - Math.max(layout.plotRect.y, Math.min(zoomRectSession.origin.y, zoomRectSession.current.y)))}
                fill="rgba(180, 214, 232, 0.12)"
                stroke="rgba(223, 232, 238, 0.88)"
                stroke-dasharray="5 4"
              />
            {/if}
          {/if}
          {#if model}
            <text class="title" x={layout.title.x} y={layout.title.y}>{model.title}</text>
            {#if model.subtitle}
              <text class="subtitle" x={layout.subtitle.x} y={layout.subtitle.y}>{model.subtitle}</text>
            {/if}
            <text class="axis-label x" x={layout.plotRect.x + layout.plotRect.width / 2} y={layout.xAxisLabelY}>
              {resolveCartesianAxisTitle("Angle", model.xAxis.label, model.xAxis.unit, currentAxisOverrides.x)}
            </text>
            <text
              class="axis-label y"
              x={layout.yAxisLabelX}
              y={layout.plotRect.y + layout.plotRect.height / 2}
              transform={`rotate(-90 ${layout.yAxisLabelX} ${layout.plotRect.y + layout.plotRect.height / 2})`}
            >
              {resolveCartesianAxisTitle("Response", model.yAxis.label, model.yAxis.unit, currentAxisOverrides.y)}
            </text>
          {/if}
        </svg>
      </div>

      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">{errorMessage}</div>
      {:else if !model}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {/if}

      {#if model}
        <div class="ophiolite-charts-legend" style:right={`${layout.legendRight}px`} style:top={`${layout.legendTop}px`}>
          {#each model.series as series (series.id)}
            <div class="ophiolite-charts-legend-row">
              <span class="swatch" style:background={series.color}></span>
              <span>{series.label}</span>
            </div>
          {/each}
        </div>
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

      {#if currentProbe && !loading && !errorMessage}
        <ProbePanel
          theme="light"
          size="standard"
          left={`${layout.plotRect.x + layout.probePanelInset}px`}
          bottom={`${MARGIN.bottom + layout.probePanelInset}px`}
          rows={avoResponseProbeRows()}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .ophiolite-charts-avo-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 320px;
    display: flex;
    overflow-x: auto;
    overflow-y: auto;
    background: #edf1f6;
  }

  .ophiolite-charts-avo-lane {
    min-width: 100%;
    min-height: 100%;
    width: max-content;
    height: max-content;
    display: grid;
    place-items: start;
  }

  .ophiolite-charts-avo-stage {
    position: relative;
    min-height: 320px;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-avo-host {
    width: 100%;
    height: 100%;
    outline: none;
  }

  .ophiolite-charts-avo-svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  .tick {
    fill: #5b6d7f;
    font: var(--ophiolite-charts-cartesian-tick-font);
  }

  .tick.x {
    text-anchor: middle;
  }

  .tick.y {
    text-anchor: end;
  }

  .title {
    fill: #324355;
    font: var(--ophiolite-charts-cartesian-title-font);
  }

  .subtitle {
    fill: #6c7f91;
    font: var(--ophiolite-charts-cartesian-subtitle-font);
  }

  .axis-label {
    fill: #425567;
    font: var(--ophiolite-charts-cartesian-axis-font);
    text-anchor: middle;
  }

  .axis-label.x {
    text-anchor: middle;
  }

  .ophiolite-charts-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(243, 245, 249, 0.92);
    color: #324355;
    font: 600 14px/1.4 sans-serif;
    pointer-events: none;
  }

  .ophiolite-charts-overlay-error {
    color: #b65248;
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

  .ophiolite-charts-legend {
    position: absolute;
    z-index: 2;
    display: grid;
    gap: 6px;
    max-width: 180px;
    padding: 10px 12px;
    border: 1px solid rgba(123, 142, 161, 0.26);
    background: rgba(255, 255, 255, 0.94);
    color: #324355;
    font: 600 11px/1.25 sans-serif;
  }

  .ophiolite-charts-legend-row {
    display: grid;
    grid-template-columns: 10px 1fr;
    gap: 8px;
    align-items: center;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

</style>
