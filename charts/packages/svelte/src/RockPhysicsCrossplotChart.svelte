<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import {
    applyViewportToAxisOverrides,
    cloneCartesianAxisOverrides,
    getRockPhysicsCrossplotPlotRect,
    hitTestCartesianAxisBand,
    ROCK_PHYSICS_CROSSPLOT_MARGIN,
    rockPhysicsScreenToValue
  } from "@ophiolite/charts-core";
  import { RockPhysicsCrossplotController } from "@ophiolite/charts-domain";
  import { PointCloudSpikeRenderer } from "@ophiolite/charts-renderer";
  import type {
    CartesianAxisOverrides,
    RockPhysicsCrossplotProbe,
    RockPhysicsCrossplotViewport
  } from "@ophiolite/charts-data-models";
  import ProbePanel from "./ProbePanel.svelte";
  import { resolveRockPhysicsStageSize, scaleRockPhysicsStageSize } from "./rock-physics-stage";
  import {
    ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES,
    type RockPhysicsCrossplotChartInteractionState,
    type RockPhysicsCrossplotChartProps
  } from "./types";

  interface PanDragPoint {
    clientX: number;
    clientY: number;
  }

  let {
    chartId,
    model = null,
    viewport = null,
    axisOverrides = undefined,
    interactions = undefined,
    loading = false,
    emptyMessage = "No rock physics crossplot selected.",
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
  }: RockPhysicsCrossplotChartProps = $props();

  let controller: RockPhysicsCrossplotController | null = null;
  let currentProbe = $state.raw<RockPhysicsCrossplotProbe | null>(null);
  let currentViewport = $state.raw<RockPhysicsCrossplotViewport | null>(null);
  let currentAxisOverrides = $state.raw<CartesianAxisOverrides>({});
  let lastModel: RockPhysicsCrossplotChartProps["model"] = null;
  let lastResetToken: string | number | null = null;
  let lastViewportKey = "";
  let lastProbeKey = "";
  let lastAxisOverridesKey = "";
  let lastRequestedAxisOverridesKey = "";
  let lastInteractionKey = "";
  let lastInteractionStateKey = "";
  let activePointerId: number | null = null;
  let activeDragKind: "pan" | "zoomRect" | null = null;
  let lastPanPoint = $state.raw<PanDragPoint | null>(null);
  let rendererErrorMessage = $state<string | null>(null);
  let hostElement = $state.raw<HTMLDivElement | null>(null);
  let requestedTool = $derived(resolveRequestedTool());
  let stageSize = $derived(
    scaleRockPhysicsStageSize(resolveRockPhysicsStageSize(), stageScale)
  );
  let hostCursor = $derived.by(() => {
    if (activeDragKind === "zoomRect") {
      return "crosshair";
    }
    if (activeDragKind === "pan") {
      return "grabbing";
    }
    if (requestedTool === "pan") {
      return "grab";
    }
    return null;
  });

  onMount(() => {
    const element = hostElement;
    if (!element) {
      return;
    }
    rendererErrorMessage = null;
    const activeController = new RockPhysicsCrossplotController(new PointCloudSpikeRenderer());
    controller = activeController;
    currentProbe = null;
    currentViewport = null;
    currentAxisOverrides = cloneCartesianAxisOverrides(axisOverrides);

    try {
      syncController(activeController);

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

        const nextAxisOverrides = applyViewportToAxisOverrides(state.axisOverrides, state.viewport);
        const nextAxisOverridesKey = JSON.stringify(nextAxisOverrides);
        if (nextAxisOverridesKey !== lastAxisOverridesKey) {
          lastAxisOverridesKey = nextAxisOverridesKey;
          currentAxisOverrides = nextAxisOverrides;
          onAxisOverridesChange?.({
            chartId,
            axisOverrides: currentAxisOverrides
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
      const onPointerDown = (event: PointerEvent) => handlePointerDown(event);
      const onPointerMove = (event: PointerEvent) => handlePointerMove(event);
      const onPointerUp = (event: PointerEvent) => handlePointerUp(event);
      const onPointerCancel = (event: PointerEvent) => handlePointerCancel(event);
      const onPointerLeave = () => handlePointerLeave();
      const onFocus = () => handleFocus();
      const onBlur = () => handleBlur();
      const onKeyDown = (event: KeyboardEvent) => handleKeyDown(event);
      const onWheel = (event: WheelEvent) => handleWheel(event);
      const onContextMenu = (event: MouseEvent) => handleContextMenu(event);

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
      element.addEventListener("contextmenu", onContextMenu);

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
        element.removeEventListener("contextmenu", onContextMenu);
        if (controller === activeController) {
          controller = null;
        }
        currentProbe = null;
        currentViewport = null;
        activeController.dispose();
      };
    } catch (error) {
      rendererErrorMessage = error instanceof Error ? error.message : String(error);
      console.error("RockPhysicsCrossplotChart initialization failed.", error);
      controller = null;
      currentProbe = null;
      currentViewport = null;
      activeController.dispose();
      return () => {};
    }
  });

  $effect(() => {
    if (!controller) {
      return;
    }
    syncController(controller);
  });

  export function fitToData(): void {
    controller?.fitToData();
  }

  export function setViewport(nextViewport: NonNullable<RockPhysicsCrossplotChartProps["viewport"]>): void {
    viewport = nextViewport;
    currentViewport = nextViewport;
    controller?.setViewport(nextViewport);
  }

  export function zoomBy(factor: number): void {
    controller?.zoom(factor);
  }

  export function panBy(deltaX: number, deltaY: number): void {
    controller?.pan(deltaX, deltaY);
  }

  function syncController(activeController: RockPhysicsCrossplotController): void {
    const modelChanged = model !== lastModel;
    const shouldReset = resetToken !== lastResetToken || modelChanged;
    lastResetToken = resetToken;

    if (shouldReset) {
      activeController.setModel(model);
      lastModel = model;
    }

    const requestedAxisOverrides = cloneCartesianAxisOverrides(axisOverrides);
    const requestedAxisOverridesKey = JSON.stringify(requestedAxisOverrides);
    if (requestedAxisOverridesKey !== lastRequestedAxisOverridesKey) {
      lastRequestedAxisOverridesKey = requestedAxisOverridesKey;
      activeController.setAxisOverrides(requestedAxisOverrides);
      if (!viewport) {
        const nextViewport = viewportFromAxisOverrides(activeController.getState().viewport, requestedAxisOverrides);
        if (nextViewport) {
          activeController.setViewport(nextViewport);
        }
      }
    } else {
      activeController.setAxisOverrides(requestedAxisOverrides);
    }

    if (model && viewport) {
      currentViewport = viewport;
      activeController.setViewport(viewport);
    } else if (!model) {
      currentViewport = null;
    }

    applyTool(activeController, requestedTool);
  }

  function applyTool(
    activeController: RockPhysicsCrossplotController,
    tool: "pointer" | "crosshair" | "pan"
  ): void {
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
    if (activeDragKind === "zoomRect") {
      const point = pointerPoint(event, element);
      controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
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
    const point = pointerPoint(event, element);
    if (event.shiftKey) {
      activeDragKind = controller.beginZoomRect(point.x, point.y, element.clientWidth, element.clientHeight)
        ? "zoomRect"
        : null;
      return;
    }
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
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    if (activeDragKind === "zoomRect") {
      controller.commitZoomRect(element.clientWidth, element.clientHeight);
      activeDragKind = null;
      controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
    } else {
      activeDragKind = null;
      lastPanPoint = null;
      controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
    }
    releasePointerCapture(element, event.pointerId);
  }

  function handlePointerCancel(event: PointerEvent): void {
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    activeDragKind = null;
    lastPanPoint = null;
    controller?.cancelInteractionSession();
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
      controller.cancelInteractionSession();
      controller.blur();
      event.preventDefault();
      return;
    }

    const stepX = (currentViewport.xMax - currentViewport.xMin) * 0.08;
    const stepY = (currentViewport.yMax - currentViewport.yMin) * 0.08;
    const yPanDirection = currentYAxisDirection() === "reversed" ? -1 : 1;

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
        controller.pan(0, stepY * yPanDirection);
        event.preventDefault();
        break;
      case "ArrowDown":
        controller.pan(0, -stepY * yPanDirection);
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
    const plotRect = getRockPhysicsCrossplotPlotRect(element.clientWidth, element.clientHeight);
    const value = rockPhysicsScreenToValue(point.x, point.y, currentViewport, plotRect, currentYAxisDirection());
    controller.zoomAround(value.x, value.y, event.deltaY < 0 ? 1.12 : 0.89);
    event.preventDefault();
  }

  function handleContextMenu(event: MouseEvent): void {
    if (!controller || !currentViewport) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement)) {
      return;
    }
    const point = pointerPoint(event, element);
    const plotRect = getRockPhysicsCrossplotPlotRect(element.clientWidth, element.clientHeight);
    const axis = hitTestCartesianAxisBand(point.x, point.y, plotRect, element.clientWidth, element.clientHeight);
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
    const withinPlot =
      point.x >= plotRect.x &&
      point.x <= plotRect.x + plotRect.width &&
      point.y >= plotRect.y &&
      point.y <= plotRect.y + plotRect.height;
    if (!withinPlot) {
      return;
    }
    const value = rockPhysicsScreenToValue(point.x, point.y, currentViewport, plotRect, currentYAxisDirection());
    controller.zoomAround(value.x, value.y, 0.7);
    controller.updatePointer(point.x, point.y, element.clientWidth, element.clientHeight);
    event.preventDefault();
  }

  function resolveRequestedTool(): "pointer" | "crosshair" | "pan" {
    return interactions?.tool ?? "crosshair";
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

  function createInteractionState(
    tool: "pointer" | "crosshair" | "pan"
  ): RockPhysicsCrossplotChartInteractionState {
    return {
      capabilities: {
        tools: [...ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES.tools],
        actions: [...ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES.actions]
      },
      tool
    };
  }

  function pointerPoint(
    event: PointerEvent | WheelEvent | MouseEvent,
    element: HTMLDivElement
  ): { x: number; y: number } {
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
    const plotRect = getRockPhysicsCrossplotPlotRect(element.clientWidth, element.clientHeight);
    const dataDeltaX =
      (-deltaX / Math.max(1, plotRect.width)) * (currentViewport.xMax - currentViewport.xMin);
    const dataDeltaY =
      ((currentYAxisDirection() === "reversed" ? -deltaY : deltaY) / Math.max(1, plotRect.height)) *
      (currentViewport.yMax - currentViewport.yMin);
    controller.pan(dataDeltaX, dataDeltaY);
  }

  function currentYAxisDirection(): "normal" | "reversed" {
    return model?.yAxis.direction ?? "normal";
  }

  function rockPhysicsProbeRows(): Array<{ label: string; value: string }> {
    if (!currentProbe) {
      return [];
    }

    const rows = [
      { label: "well", value: currentProbe.wellName },
      { label: "x", value: currentProbe.xValue.toFixed(0) },
      { label: "y", value: currentProbe.yValue.toFixed(3) },
      { label: "depth", value: `${currentProbe.sampleDepthM.toFixed(1)} m` }
    ];

    if (currentProbe.colorValue !== undefined) {
      rows.push({ label: "color", value: currentProbe.colorValue.toFixed(3) });
    } else if (currentProbe.colorCategoryLabel) {
      rows.push({ label: "color", value: currentProbe.colorCategoryLabel });
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

  function viewportFromAxisOverrides(
    source: RockPhysicsCrossplotViewport | null,
    overrides: CartesianAxisOverrides
  ): RockPhysicsCrossplotViewport | null {
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
</script>

<div class="ophiolite-charts-rock-physics-shell">
  <div class="ophiolite-charts-rock-physics-lane">
    <div
      class="ophiolite-charts-rock-physics-stage"
      style:width={`${stageSize.width}px`}
      style:height={`${stageSize.height}px`}
      style:--ophiolite-charts-plot-top={`${ROCK_PHYSICS_CROSSPLOT_MARGIN.top}px`}
      style:--ophiolite-charts-plot-right={`${ROCK_PHYSICS_CROSSPLOT_MARGIN.right}px`}
      style:--ophiolite-charts-plot-bottom={`${ROCK_PHYSICS_CROSSPLOT_MARGIN.bottom}px`}
      style:--ophiolite-charts-plot-left={`${ROCK_PHYSICS_CROSSPLOT_MARGIN.left}px`}
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="ophiolite-charts-rock-physics-host"
        bind:this={hostElement}
        tabindex="0"
        role="application"
        aria-label="Rock physics crossplot"
        aria-busy={loading}
        style:cursor={hostCursor ?? undefined}
      ></div>
      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage || rendererErrorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">{errorMessage ?? rendererErrorMessage}</div>
      {:else if !model}
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
      {#if currentProbe && !loading && !errorMessage && model}
        <ProbePanel
          theme="light"
          size="standard"
          right={`${ROCK_PHYSICS_CROSSPLOT_MARGIN.right}px`}
          bottom={`${ROCK_PHYSICS_CROSSPLOT_MARGIN.bottom}px`}
          rows={rockPhysicsProbeRows()}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .ophiolite-charts-rock-physics-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 320px;
    display: flex;
    overflow-x: auto;
    overflow-y: auto;
    background: #06141c;
  }

  .ophiolite-charts-rock-physics-lane {
    min-width: 100%;
    min-height: 100%;
    width: max-content;
    height: max-content;
    display: grid;
    place-items: start;
  }

  .ophiolite-charts-rock-physics-stage {
    position: relative;
    min-height: 320px;
    flex: 0 0 auto;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-rock-physics-host {
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
    background: rgba(6, 20, 28, 0.88);
    color: #d9e7ee;
    font: 600 14px/1.4 sans-serif;
    pointer-events: none;
  }

  .ophiolite-charts-overlay-error {
    color: #f29e93;
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
