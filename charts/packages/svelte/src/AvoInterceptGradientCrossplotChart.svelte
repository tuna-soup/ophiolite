<svelte:options runes={true} />

<script lang="ts">
  import {
    AVO_CROSSPLOT_MARGIN,
    avoCrossplotScreenToValue,
    clampAvoCrossplotViewport,
    fitAvoCrossplotViewport,
    getAvoCrossplotPlotRect,
    valueToAvoCrossplotScreenX,
    valueToAvoCrossplotScreenY
  } from "@ophiolite/charts-core";
  import type {
    AvoCartesianViewport,
    AvoCrossplotProbe,
    AvoInterfaceDescriptor,
    InteractionEvent,
    InteractionTarget
  } from "@ophiolite/charts-data-models";
  import { scaleAvoStageSize, resolveAvoStageSize } from "./avo-stage";
  import {
    AVO_CHART_INTERACTION_CAPABILITIES,
    type AvoChartInteractionState,
    type AvoInterceptGradientCrossplotChartProps
  } from "./types";

  interface ScreenPoint {
    x: number;
    y: number;
  }

  const POINT_RADIUS_PX = 2.3;
  const HIT_RADIUS_PX = 10;

  let {
    chartId,
    model = null,
    viewport = null,
    interactions = undefined,
    loading = false,
    emptyMessage = "No AVO crossplot selected.",
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
  }: AvoInterceptGradientCrossplotChartProps = $props();

  let host = $state.raw<HTMLDivElement | null>(null);
  let canvas = $state.raw<HTMLCanvasElement | null>(null);
  let currentViewport = $state.raw<AvoCartesianViewport | null>(null);
  let currentProbe = $state.raw<AvoCrossplotProbe | null>(null);
  let activePointerId = $state.raw<number | null>(null);
  let lastPanPoint = $state.raw<ScreenPoint | null>(null);
  let lastModel = $state.raw<AvoInterceptGradientCrossplotChartProps["model"]>(null);
  let lastResetToken = $state.raw<string | number | null>(null);
  let lastInteractionStateKey = "";
  let lastHoverKey = "";
  let stageSize = $derived(scaleAvoStageSize(resolveAvoStageSize(), stageScale));
  let requestedTool = $derived(interactions?.tool ?? "crosshair");
  let plotRect = $derived(getAvoCrossplotPlotRect(stageSize.width, stageSize.height));

  $effect(() => {
    const nextViewport = viewport ?? fitAvoCrossplotViewport(model);
    const shouldReset = model !== lastModel || resetToken !== lastResetToken;
    lastResetToken = resetToken;
    if (shouldReset) {
      lastModel = model;
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

  $effect(() => {
    renderCanvas();
  });

  export function fitToData(): void {
    setViewportState(fitAvoCrossplotViewport(model), true);
  }

  export function setViewport(nextViewport: NonNullable<AvoInterceptGradientCrossplotChartProps["viewport"]>): void {
    setViewportState(clampAvoCrossplotViewport(model, nextViewport), true);
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
      clampAvoCrossplotViewport(model, {
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
    if (requestedTool !== "pan") {
      updateProbeFromPointer(event);
      return;
    }
    activePointerId = event.pointerId;
    lastPanPoint = pointerPoint(event);
    host.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!host) {
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
    releasePointerCapture(event.pointerId);
    lastPanPoint = null;
  }

  function handlePointerCancel(event: PointerEvent): void {
    if (!host) {
      return;
    }
    releasePointerCapture(event.pointerId);
    lastPanPoint = null;
  }

  function handlePointerLeave(): void {
    currentProbe = null;
    notifyProbeChange();
    emitHoverTarget(null);
  }

  function handleWheel(event: WheelEvent): void {
    if (!currentViewport) {
      return;
    }
    const point = pointerPoint(event);
    if (!pointInPlot(point)) {
      return;
    }
    const value = avoCrossplotScreenToValue(point.x, point.y, currentViewport, plotRect);
    zoomAround(value.x, value.y, event.deltaY < 0 ? 1.12 : 0.89);
    event.preventDefault();
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (!currentViewport) {
      return;
    }
    if (event.key === "Escape") {
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

  function updateProbeFromPointer(event: PointerEvent | WheelEvent): void {
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
    const probe = findNearestProbe(model, currentViewport, point);
    currentProbe = probe;
    notifyProbeChange();
    emitHoverTarget(
      probe
        ? {
            kind: "point-cloud-sample",
            chartId,
            entityId: String(probe.pointIndex),
            seriesId: probe.interfaceId
          }
        : null
    );
  }

  function renderCanvas(): void {
    const context = canvas?.getContext("2d");
    if (!canvas || !context) {
      return;
    }

    const pixelRatio = window.devicePixelRatio || 1;
    const cssWidth = stageSize.width;
    const cssHeight = stageSize.height;
    if (canvas.width !== Math.round(cssWidth * pixelRatio) || canvas.height !== Math.round(cssHeight * pixelRatio)) {
      canvas.width = Math.round(cssWidth * pixelRatio);
      canvas.height = Math.round(cssHeight * pixelRatio);
    }
    context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
    context.clearRect(0, 0, cssWidth, cssHeight);

    context.fillStyle = "#faf9f5";
    context.fillRect(0, 0, cssWidth, cssHeight);
    context.fillStyle = "#ffffff";
    context.fillRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);
    context.strokeStyle = "rgba(119, 138, 158, 0.3)";
    context.strokeRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);

    if (!currentViewport) {
      return;
    }

    drawCrossplotGrid(context, currentViewport);
    if (model) {
      drawBackgroundRegions(context, currentViewport, model.interfaces);
      drawReferenceLines(context, currentViewport);
      drawPoints(context, currentViewport);
      drawTitles(context);
      drawAxisLabels(context);
      if (currentProbe && requestedTool === "crosshair") {
        drawProbe(context, currentViewport);
      }
    }
  }

  function drawCrossplotGrid(context: CanvasRenderingContext2D, viewportState: AvoCartesianViewport): void {
    const xTicks = buildTicks(viewportState.xMin, viewportState.xMax, 6);
    const yTicks = buildTicks(viewportState.yMin, viewportState.yMax, 6);
    context.save();
    context.strokeStyle = "rgba(130, 148, 166, 0.18)";
    context.fillStyle = "#5b6d7f";
    context.font = "500 10px sans-serif";

    for (const tick of xTicks) {
      const x = valueToAvoCrossplotScreenX(tick, viewportState, plotRect);
      context.beginPath();
      context.moveTo(x, plotRect.y);
      context.lineTo(x, plotRect.y + plotRect.height);
      context.stroke();
      context.textAlign = "center";
      context.fillText(formatTick(tick), x, plotRect.y + plotRect.height + 22);
    }

    for (const tick of yTicks) {
      const y = valueToAvoCrossplotScreenY(tick, viewportState, plotRect);
      context.beginPath();
      context.moveTo(plotRect.x, y);
      context.lineTo(plotRect.x + plotRect.width, y);
      context.stroke();
      context.textAlign = "right";
      context.fillText(formatTick(tick), plotRect.x - 10, y + 4);
    }
    context.restore();
  }

  function drawBackgroundRegions(
    context: CanvasRenderingContext2D,
    viewportState: AvoCartesianViewport,
    interfaces: AvoInterfaceDescriptor[]
  ): void {
    if (!model) {
      return;
    }
    context.save();
    for (const region of model.backgroundRegions) {
      const x1 = valueToAvoCrossplotScreenX(region.xMin, viewportState, plotRect);
      const x2 = valueToAvoCrossplotScreenX(region.xMax, viewportState, plotRect);
      const y1 = valueToAvoCrossplotScreenY(region.yMax, viewportState, plotRect);
      const y2 = valueToAvoCrossplotScreenY(region.yMin, viewportState, plotRect);
      context.fillStyle = region.fillColor;
      context.fillRect(Math.min(x1, x2), Math.min(y1, y2), Math.abs(x2 - x1), Math.abs(y2 - y1));
      if (region.label) {
        context.fillStyle = "rgba(61, 73, 85, 0.74)";
        context.font = "600 13px sans-serif";
        context.textAlign = "left";
        context.fillText(region.label, Math.min(x1, x2) + 8, Math.min(y1, y2) + 16);
      }
    }
    context.restore();
  }

  function drawReferenceLines(context: CanvasRenderingContext2D, viewportState: AvoCartesianViewport): void {
    if (!model) {
      return;
    }
    context.save();
    context.lineWidth = 1.3;
    for (const line of model.referenceLines) {
      context.strokeStyle = line.color;
      context.setLineDash(line.style === "dashed" ? [8, 6] : []);
      context.beginPath();
      context.moveTo(valueToAvoCrossplotScreenX(line.x1, viewportState, plotRect), valueToAvoCrossplotScreenY(line.y1, viewportState, plotRect));
      context.lineTo(valueToAvoCrossplotScreenX(line.x2, viewportState, plotRect), valueToAvoCrossplotScreenY(line.y2, viewportState, plotRect));
      context.stroke();
    }
    context.restore();
  }

  function drawPoints(context: CanvasRenderingContext2D, viewportState: AvoCartesianViewport): void {
    if (!model) {
      return;
    }
    context.save();
    for (let index = 0; index < model.pointCount; index += 1) {
      const x = model.columns.intercept[index];
      const y = model.columns.gradient[index];
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        continue;
      }
      const screenX = valueToAvoCrossplotScreenX(x, viewportState, plotRect);
      const screenY = valueToAvoCrossplotScreenY(y, viewportState, plotRect);
      const interfaceIndex = model.columns.interfaceIndices[index] ?? 0;
      const interfaceDescriptor = model.interfaces[interfaceIndex];
      context.fillStyle = interfaceDescriptor?.color ?? "#5a7c9d";
      context.globalAlpha = currentProbe?.pointIndex === index ? 1 : 0.62;
      context.beginPath();
      context.arc(screenX, screenY, currentProbe?.pointIndex === index ? POINT_RADIUS_PX + 1.2 : POINT_RADIUS_PX, 0, Math.PI * 2);
      context.fill();
    }
    context.globalAlpha = 1;
    context.restore();
  }

  function drawProbe(context: CanvasRenderingContext2D, viewportState: AvoCartesianViewport): void {
    const probe = currentProbe;
    if (!probe) {
      return;
    }
    const x = valueToAvoCrossplotScreenX(probe.intercept, viewportState, plotRect);
    const y = valueToAvoCrossplotScreenY(probe.gradient, viewportState, plotRect);
    context.save();
    context.strokeStyle = "rgba(64, 78, 93, 0.42)";
    context.setLineDash([6, 5]);
    context.beginPath();
    context.moveTo(x, plotRect.y);
    context.lineTo(x, plotRect.y + plotRect.height);
    context.moveTo(plotRect.x, y);
    context.lineTo(plotRect.x + plotRect.width, y);
    context.stroke();
    context.setLineDash([]);
    context.fillStyle = model?.interfaces.find((entry) => entry.id === probe.interfaceId)?.color ?? "#334455";
    context.beginPath();
    context.arc(x, y, POINT_RADIUS_PX + 1.5, 0, Math.PI * 2);
    context.fill();
    context.restore();
  }

  function drawTitles(context: CanvasRenderingContext2D): void {
    if (!model) {
      return;
    }
    context.save();
    context.fillStyle = "#324355";
    context.font = "600 14px sans-serif";
    context.fillText(model.title, plotRect.x, 30);
    if (model.subtitle) {
      context.fillStyle = "#6c7f91";
      context.font = "500 11px sans-serif";
      context.fillText(model.subtitle, plotRect.x, 48);
    }
    context.restore();
  }

  function drawAxisLabels(context: CanvasRenderingContext2D): void {
    if (!model) {
      return;
    }
    context.save();
    context.fillStyle = "#425567";
    context.font = "600 11px sans-serif";
    context.textAlign = "center";
    context.fillText(`${model.xAxis.label ?? "Intercept"}${model.xAxis.unit ? ` (${model.xAxis.unit})` : ""}`, plotRect.x + plotRect.width / 2, stageSize.height - 10);
    context.translate(20, plotRect.y + plotRect.height / 2);
    context.rotate(-Math.PI / 2);
    context.fillText(`${model.yAxis.label ?? "Gradient"}${model.yAxis.unit ? ` (${model.yAxis.unit})` : ""}`, 0, 0);
    context.restore();
  }

  function findNearestProbe(
    source: NonNullable<AvoInterceptGradientCrossplotChartProps["model"]>,
    viewportState: AvoCartesianViewport,
    point: ScreenPoint
  ): AvoCrossplotProbe | null {
    const exactPointLimit = 100_000;
    const stride = source.pointCount <= exactPointLimit ? 1 : Math.ceil(source.pointCount / exactPointLimit);
    let bestIndex = -1;
    let bestDistance = HIT_RADIUS_PX;

    for (let index = 0; index < source.pointCount; index += stride) {
      const x = source.columns.intercept[index];
      const y = source.columns.gradient[index];
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        continue;
      }
      const screenX = valueToAvoCrossplotScreenX(x, viewportState, plotRect);
      const screenY = valueToAvoCrossplotScreenY(y, viewportState, plotRect);
      const distance = Math.hypot(screenX - point.x, screenY - point.y);
      if (distance <= bestDistance) {
        bestDistance = distance;
        bestIndex = index;
      }
    }

    if (bestIndex < 0) {
      return null;
    }

    const interfaceIndex = source.columns.interfaceIndices[bestIndex] ?? 0;
    const entry = source.interfaces[interfaceIndex];
    return {
      pointIndex: bestIndex,
      interfaceId: entry?.id ?? "",
      interfaceLabel: entry?.label ?? "Unknown interface",
      intercept: source.columns.intercept[bestIndex] ?? 0,
      gradient: source.columns.gradient[bestIndex] ?? 0,
      chiProjection: source.columns.chiProjection?.[bestIndex],
      simulationId: source.columns.simulationIds?.[bestIndex],
      screenX: point.x,
      screenY: point.y
    };
  }

  function buildTicks(min: number, max: number, count: number): number[] {
    if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min || count <= 1) {
      return [min];
    }
    return Array.from({ length: count }, (_, index) => min + ((max - min) * index) / (count - 1));
  }

  function formatTick(value: number): string {
    if (Math.abs(value) >= 10) {
      return value.toFixed(0);
    }
    if (Math.abs(value) >= 1) {
      return value.toFixed(2);
    }
    return value.toFixed(3);
  }

  function notifyProbeChange(): void {
    onProbeChange?.({
      chartId,
      probe: currentProbe
    });
  }

  function setViewportState(nextViewport: AvoCartesianViewport | null, notify = true): void {
    currentViewport = nextViewport;
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

  function zoomAround(x: number, y: number, factor: number): void {
    if (!currentViewport || factor <= 0) {
      return;
    }
    const spanX = (currentViewport.xMax - currentViewport.xMin) / factor;
    const spanY = (currentViewport.yMax - currentViewport.yMin) / factor;
    const ratioX = (x - currentViewport.xMin) / Math.max(1e-6, currentViewport.xMax - currentViewport.xMin);
    const ratioY = (y - currentViewport.yMin) / Math.max(1e-6, currentViewport.yMax - currentViewport.yMin);
    setViewportState(
      clampAvoCrossplotViewport(model, {
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
    const dataDeltaX = (-deltaX / Math.max(1, plotRect.width)) * (currentViewport.xMax - currentViewport.xMin);
    const dataDeltaY = (deltaY / Math.max(1, plotRect.height)) * (currentViewport.yMax - currentViewport.yMin);
    panBy(dataDeltaX, dataDeltaY);
  }

  function pointInPlot(point: ScreenPoint): boolean {
    return (
      point.x >= plotRect.x &&
      point.x <= plotRect.x + plotRect.width &&
      point.y >= plotRect.y &&
      point.y <= plotRect.y + plotRect.height
    );
  }

  function pointerPoint(event: PointerEvent | WheelEvent): ScreenPoint {
    const rect = host?.getBoundingClientRect();
    if (!rect) {
      return { x: 0, y: 0 };
    }
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
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

  function sameViewport(left: AvoCartesianViewport | null, right: AvoCartesianViewport | null): boolean {
    return (
      left?.xMin === right?.xMin &&
      left?.xMax === right?.xMax &&
      left?.yMin === right?.yMin &&
      left?.yMax === right?.yMax
    );
  }
</script>

<div class="ophiolite-charts-avo-shell">
  <div class="ophiolite-charts-avo-lane">
    <div
      class="ophiolite-charts-avo-stage"
      style:width={`${stageSize.width}px`}
      style:height={`${stageSize.height}px`}
      style:--ophiolite-charts-plot-top={`${AVO_CROSSPLOT_MARGIN.top}px`}
      style:--ophiolite-charts-plot-right={`${AVO_CROSSPLOT_MARGIN.right}px`}
      style:--ophiolite-charts-plot-bottom={`${AVO_CROSSPLOT_MARGIN.bottom}px`}
      style:--ophiolite-charts-plot-left={`${AVO_CROSSPLOT_MARGIN.left}px`}
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        bind:this={host}
        class="ophiolite-charts-avo-host"
        tabindex="0"
        role="application"
        aria-label="AVO intercept-gradient crossplot"
        aria-busy={loading}
        style:cursor={requestedTool === "pan" ? (activePointerId == null ? "grab" : "grabbing") : "crosshair"}
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        onpointercancel={handlePointerCancel}
        onpointerleave={handlePointerLeave}
        onwheel={handleWheel}
        onkeydown={handleKeyDown}
      >
        <canvas bind:this={canvas} class="ophiolite-charts-avo-canvas"></canvas>
      </div>

      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">{errorMessage}</div>
      {:else if !model}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {/if}

      {#if model}
        <div class="ophiolite-charts-legend" style:right={`${AVO_CROSSPLOT_MARGIN.right + 10}px`} style:top={`${AVO_CROSSPLOT_MARGIN.top + 10}px`}>
          {#each model.interfaces as entry (entry.id)}
            <div class="ophiolite-charts-legend-row">
              <span class="swatch" style:background={entry.color}></span>
              <span>{entry.label}</span>
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
        <div class="ophiolite-charts-probe-panel" style:left={`${AVO_CROSSPLOT_MARGIN.left + 10}px`} style:bottom={`${AVO_CROSSPLOT_MARGIN.bottom + 10}px`}>
          <div class="ophiolite-charts-probe-panel-row">
            <span>interface</span>
            <span>{currentProbe.interfaceLabel}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>intercept</span>
            <span>{currentProbe.intercept.toFixed(3)}</span>
          </div>
          <div class="ophiolite-charts-probe-panel-row">
            <span>gradient</span>
            <span>{currentProbe.gradient.toFixed(3)}</span>
          </div>
          {#if currentProbe.chiProjection !== undefined && Number.isFinite(currentProbe.chiProjection)}
            <div class="ophiolite-charts-probe-panel-row">
              <span>chi</span>
              <span>{currentProbe.chiProjection.toFixed(3)}</span>
            </div>
          {/if}
          {#if currentProbe.simulationId !== undefined && currentProbe.simulationId > 0}
            <div class="ophiolite-charts-probe-panel-row">
              <span>simulation</span>
              <span>{currentProbe.simulationId}</span>
            </div>
          {/if}
        </div>
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
    background: #e9efe7;
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

  .ophiolite-charts-avo-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .ophiolite-charts-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(244, 246, 240, 0.92);
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

  .ophiolite-charts-probe-panel {
    position: absolute;
    z-index: 3;
    padding: 8px 10px;
    border: 1px solid rgba(123, 142, 161, 0.24);
    background: rgba(255, 255, 255, 0.96);
    box-shadow: 0 10px 24px rgba(27, 39, 54, 0.12);
    color: #324355;
    pointer-events: none;
  }

  .ophiolite-charts-probe-panel-row {
    display: grid;
    grid-template-columns: 72px auto;
    column-gap: 8px;
    align-items: baseline;
    font: 500 12px/1.25 sans-serif;
    white-space: nowrap;
  }

  .ophiolite-charts-probe-panel-row span:first-child {
    color: #708396;
  }

  .ophiolite-charts-probe-panel-row span:last-child {
    color: #233445;
  }
</style>
