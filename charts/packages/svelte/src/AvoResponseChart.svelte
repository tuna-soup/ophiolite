<svelte:options runes={true} />

<script lang="ts">
  import type { InteractionEvent, InteractionTarget, AvoCartesianViewport, AvoResponseProbe } from "@ophiolite/charts-data-models";
  import { scaleAvoStageSize, resolveAvoStageSize } from "./avo-stage";
  import {
    AVO_CHART_INTERACTION_CAPABILITIES,
    type AvoChartInteractionState,
    type AvoResponseChartProps
  } from "./types";

  interface PlotMargin {
    top: number;
    right: number;
    bottom: number;
    left: number;
  }

  interface ScreenPoint {
    x: number;
    y: number;
  }

  const MARGIN: PlotMargin = {
    top: 56,
    right: 228,
    bottom: 56,
    left: 72
  };

  let {
    chartId,
    model = null,
    viewport = null,
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
    onInteractionEvent
  }: AvoResponseChartProps = $props();

  let host = $state.raw<HTMLDivElement | null>(null);
  let currentViewport = $state.raw<AvoCartesianViewport | null>(null);
  let currentProbe = $state.raw<AvoResponseProbe | null>(null);
  let activePointerId = $state.raw<number | null>(null);
  let lastPanPoint = $state.raw<ScreenPoint | null>(null);
  let lastModel = $state.raw<AvoResponseChartProps["model"]>(null);
  let lastResetToken = $state.raw<string | number | null>(null);
  let lastInteractionStateKey = "";
  let lastHoverKey = "";
  let stageSize = $derived(scaleAvoStageSize(resolveAvoStageSize(), stageScale));
  let requestedTool = $derived(interactions?.tool ?? "crosshair");
  let plotWidth = $derived(Math.max(1, stageSize.width - MARGIN.left - MARGIN.right));
  let plotHeight = $derived(Math.max(1, stageSize.height - MARGIN.top - MARGIN.bottom));
  let xTicks = $derived(buildTicks(currentViewport?.xMin ?? 0, currentViewport?.xMax ?? 40, 6));
  let yTicks = $derived(buildTicks(currentViewport?.yMin ?? -0.3, currentViewport?.yMax ?? 0.2, 6));

  $effect(() => {
    const nextViewport = viewport ?? fitViewport(model);
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

  function buildTicks(min: number, max: number, count: number): number[] {
    if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min || count <= 1) {
      return [min];
    }
    return Array.from({ length: count }, (_, index) => min + ((max - min) * index) / (count - 1));
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

  function formatTick(value: number): string {
    if (Math.abs(value) >= 10) {
      return value.toFixed(0);
    }
    if (Math.abs(value) >= 1) {
      return value.toFixed(2);
    }
    return value.toFixed(3);
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
        style:cursor={requestedTool === "pan" ? (activePointerId == null ? "grab" : "grabbing") : "crosshair"}
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        onpointercancel={handlePointerCancel}
        onpointerleave={handlePointerLeave}
        onwheel={handleWheel}
        onkeydown={handleKeyDown}
      >
        <svg viewBox={`0 0 ${stageSize.width} ${stageSize.height}`} class="ophiolite-charts-avo-svg" aria-hidden="true">
          <rect x="0" y="0" width={stageSize.width} height={stageSize.height} fill="#f7f8fb" />
          <rect
            x={MARGIN.left}
            y={MARGIN.top}
            width={plotWidth}
            height={plotHeight}
            fill="#ffffff"
            stroke="rgba(119, 138, 158, 0.28)"
          />
          {#if currentViewport}
            {#each xTicks as tick (tick)}
              <line
                x1={valueToScreenX(tick, currentViewport)}
                y1={MARGIN.top}
                x2={valueToScreenX(tick, currentViewport)}
                y2={MARGIN.top + plotHeight}
                stroke="rgba(130, 148, 166, 0.18)"
              />
              <text class="tick x" x={valueToScreenX(tick, currentViewport)} y={stageSize.height - 18}>{formatTick(tick)}</text>
            {/each}
            {#each yTicks as tick (tick)}
              <line
                x1={MARGIN.left}
                y1={valueToScreenY(tick, currentViewport)}
                x2={MARGIN.left + plotWidth}
                y2={valueToScreenY(tick, currentViewport)}
                stroke="rgba(130, 148, 166, 0.18)"
              />
              <text class="tick y" x={MARGIN.left - 10} y={valueToScreenY(tick, currentViewport) + 4}>{formatTick(tick)}</text>
            {/each}
            {#if model}
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
            {/if}
            {#if currentProbe && requestedTool === "crosshair"}
              <line
                x1={currentProbe.screenX}
                y1={MARGIN.top}
                x2={currentProbe.screenX}
                y2={MARGIN.top + plotHeight}
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
            {/if}
          {/if}
          {#if model}
            <text class="title" x={MARGIN.left} y="30">{model.title}</text>
            {#if model.subtitle}
              <text class="subtitle" x={MARGIN.left} y="48">{model.subtitle}</text>
            {/if}
            <text class="axis-label x" x={MARGIN.left + plotWidth / 2} y={stageSize.height - 10}>
              {model.xAxis.label ?? "Angle"}{model.xAxis.unit ? ` (${model.xAxis.unit})` : ""}
            </text>
            <text
              class="axis-label y"
              x="20"
              y={MARGIN.top + plotHeight / 2}
              transform={`rotate(-90 20 ${MARGIN.top + plotHeight / 2})`}
            >
              {model.yAxis.label ?? "Response"}{model.yAxis.unit ? ` (${model.yAxis.unit})` : ""}
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
        <div class="ophiolite-charts-legend" style:right={`${MARGIN.right - 12}px`} style:top={`${MARGIN.top + 16}px`}>
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
        <div class="ophiolite-charts-probe-panel" style:left={`${MARGIN.left + 10}px`} style:bottom={`${MARGIN.bottom + 10}px`}>
          <div class="ophiolite-charts-probe-panel-row">
            <span>angle</span>
            <span>{currentProbe.angleDeg.toFixed(1)} deg</span>
          </div>
          {#each currentProbe.seriesValues as entry (entry.seriesId)}
            <div class="ophiolite-charts-probe-panel-row">
              <span>{entry.label}</span>
              <span>{entry.value.toFixed(3)}</span>
            </div>
          {/each}
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
    font: 500 10px/1 sans-serif;
  }

  .tick.x {
    text-anchor: middle;
  }

  .tick.y {
    text-anchor: end;
  }

  .title {
    fill: #324355;
    font: 600 14px/1.1 sans-serif;
  }

  .subtitle {
    fill: #6c7f91;
    font: 500 11px/1.1 sans-serif;
  }

  .axis-label {
    fill: #425567;
    font: 600 11px/1.1 sans-serif;
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
    grid-template-columns: 124px auto;
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
