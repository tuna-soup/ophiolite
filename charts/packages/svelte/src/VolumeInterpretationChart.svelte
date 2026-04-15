<svelte:options runes={true} />

<script lang="ts">
  import { VolumeInterpretationController } from "@ophiolite/charts-domain";
  import {
    VolumeInterpretationPlaceholderRenderer,
    VolumeInterpretationVtkRenderer
  } from "@ophiolite/charts-renderer";
  import { resolveVolumeInterpretationStageSize, scaleVolumeInterpretationStageSize } from "./volume-interpretation-stage";
  import {
    VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES,
    type VolumeInterpretationChartInteractionState,
    type VolumeInterpretationChartProps,
    type VolumeInterpretationChartRenderer,
    type VolumeInterpretationChartTool
  } from "./types";

  interface DragState {
    kind: "orbit" | "pan" | "slice-drag";
    pointerId: number;
    lastX: number;
    lastY: number;
  }

  let {
    chartId,
    model = null,
    tool = "pointer",
    renderer = "vtk",
    interactions = undefined,
    loading = false,
    emptyMessage = "No interpretation scene selected.",
    errorMessage = null,
    resetToken = null,
    stageTopLeft = undefined,
    plotTopCenter = undefined,
    plotTopRight = undefined,
    plotBottomRight = undefined,
    plotBottomLeft = undefined,
    stageScale = 1,
    onProbeChange,
    onSelectionChange,
    onViewStateChange,
    onInteractionStateChange,
    onInteractionEvent,
    onInterpretationRequest
  }: VolumeInterpretationChartProps = $props();

  let controller: VolumeInterpretationController | null = null;
  let lastModel: VolumeInterpretationChartProps["model"] = null;
  let lastResetToken: string | number | null = null;
  let lastProbeKey = "";
  let lastSelectionKey = "";
  let lastViewKey = "";
  let lastInteractionStateKey = "";
  let drag = $state.raw<DragState | null>(null);
  let effectiveTool = $derived<VolumeInterpretationChartTool>(resolveRequestedTool());
  let stageSize = $derived(
    scaleVolumeInterpretationStageSize(resolveVolumeInterpretationStageSize(model), stageScale)
  );
  let hostCursor = $derived.by(() => {
    if (drag?.kind === "orbit" || drag?.kind === "pan" || drag?.kind === "slice-drag") {
      return "grabbing";
    }
    if (effectiveTool === "orbit" || effectiveTool === "pan" || effectiveTool === "slice-drag") {
      return "grab";
    }
    if (effectiveTool === "interpret-seed") {
      return "crosshair";
    }
    return null;
  });

  function attachChartHost(element: HTMLDivElement): () => void {
    const activeController = new VolumeInterpretationController(createRenderer(renderer));
    controller = activeController;
    lastModel = null;

    const unsubscribeStateChange = activeController.onStateChange((state) => {
      const nextProbeKey = JSON.stringify(state.probe);
      if (nextProbeKey !== lastProbeKey) {
        lastProbeKey = nextProbeKey;
        onProbeChange?.({
          chartId,
          probe: state.probe
        });
      }

      const nextSelectionKey = JSON.stringify(state.selection);
      if (nextSelectionKey !== lastSelectionKey) {
        lastSelectionKey = nextSelectionKey;
        onSelectionChange?.({
          chartId,
          selection: state.selection
        });
      }

      const nextViewKey = JSON.stringify(state.view);
      if (nextViewKey !== lastViewKey) {
        lastViewKey = nextViewKey;
        onViewStateChange?.({
          chartId,
          view: state.view
        });
      }

      const nextInteractionState = createInteractionState(state.tool);
      const nextInteractionStateKey = JSON.stringify(nextInteractionState);
      if (nextInteractionStateKey !== lastInteractionStateKey) {
        lastInteractionStateKey = nextInteractionStateKey;
        onInteractionStateChange?.(nextInteractionState);
      }
    });
    const unsubscribeInteractionEvent = activeController.onInteractionEvent((event) => {
      onInteractionEvent?.({
        chartId,
        event
      });
    });
    const unsubscribeInterpretationRequest = activeController.onInterpretationRequest((request) => {
      onInterpretationRequest?.({
        chartId,
        request
      });
    });

    const resizeObserver = new ResizeObserver(() => {
      activeController.refresh();
    });
    const onPointerDown = (event: PointerEvent) => handlePointerDown(event);
    const onPointerMove = (event: PointerEvent) => handlePointerMove(event);
    const onPointerUp = (event: PointerEvent) => handlePointerUp(event);
    const onPointerCancel = (event: PointerEvent) => handlePointerCancel(event);
    const onPointerLeave = () => handlePointerLeave();
    const onWheel = (event: WheelEvent) => handleWheel(event);
    const onKeyDown = (event: KeyboardEvent) => handleKeyDown(event);
    const onFocus = () => controller?.interactions.setFocused(true);
    const onBlur = () => {
      drag = null;
      controller?.interactions.setFocused(false);
      controller?.clearPointer();
    };

    activeController.mount(element);
    resizeObserver.observe(element);
    element.addEventListener("pointerdown", onPointerDown);
    element.addEventListener("pointermove", onPointerMove);
    element.addEventListener("pointerup", onPointerUp);
    element.addEventListener("pointercancel", onPointerCancel);
    element.addEventListener("pointerleave", onPointerLeave);
    element.addEventListener("wheel", onWheel, { passive: false });
    element.addEventListener("keydown", onKeyDown);
    element.addEventListener("focus", onFocus);
    element.addEventListener("blur", onBlur);

    $effect(() => {
      syncController(activeController);
    });

    return () => {
      unsubscribeInterpretationRequest();
      unsubscribeInteractionEvent();
      unsubscribeStateChange();
      resizeObserver.disconnect();
      element.removeEventListener("pointerdown", onPointerDown);
      element.removeEventListener("pointermove", onPointerMove);
      element.removeEventListener("pointerup", onPointerUp);
      element.removeEventListener("pointercancel", onPointerCancel);
      element.removeEventListener("pointerleave", onPointerLeave);
      element.removeEventListener("wheel", onWheel);
      element.removeEventListener("keydown", onKeyDown);
      element.removeEventListener("focus", onFocus);
      element.removeEventListener("blur", onBlur);
      if (controller === activeController) {
        controller = null;
      }
      activeController.dispose();
    };
  }

  export function fitToData(): void {
    controller?.fitToData();
  }

  export function resetView(): void {
    controller?.resetView();
  }

  export function centerSelection(): void {
    controller?.centerSelection();
  }

  export function zoomBy(factor: number): void {
    controller?.zoom(factor);
  }

  export function orbitBy(deltaYawDeg: number, deltaPitchDeg: number): void {
    controller?.orbit(deltaYawDeg, deltaPitchDeg);
  }

  export function panBy(deltaX: number, deltaY: number): void {
    controller?.pan(deltaX, deltaY);
  }

  function syncController(activeController: VolumeInterpretationController): void {
    activeController.setTool(effectiveTool);

    const modelChanged = model !== lastModel;
    const resetChanged = resetToken !== lastResetToken;
    lastResetToken = resetToken;

    if (modelChanged) {
      activeController.setModel(model);
      lastModel = model;
    }
    if (resetChanged) {
      activeController.resetView();
    }
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!controller) {
      return;
    }
    const element = event.currentTarget;
    if (!(element instanceof HTMLDivElement) || event.button !== 0) {
      return;
    }
    element.setPointerCapture(event.pointerId);
    const point = pointerPoint(event, element);
    controller.updatePointer(point.x, point.y);
    if (effectiveTool === "orbit") {
      drag = { kind: "orbit", pointerId: event.pointerId, lastX: point.x, lastY: point.y };
      return;
    }
    if (effectiveTool === "pan" || effectiveTool === "crop") {
      drag = { kind: "pan", pointerId: event.pointerId, lastX: point.x, lastY: point.y };
      return;
    }
    if (effectiveTool === "slice-drag") {
      drag = { kind: "slice-drag", pointerId: event.pointerId, lastX: point.x, lastY: point.y };
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
    const point = pointerPoint(event, element);
    if (drag && drag.pointerId === event.pointerId) {
      const deltaX = point.x - drag.lastX;
      const deltaY = point.y - drag.lastY;
      if (drag.kind === "orbit") {
        controller.orbit(deltaX * 0.3, deltaY * 0.18);
      } else if (drag.kind === "pan") {
        controller.pan(deltaX, deltaY);
      } else {
        const span = Math.max(1, model ? model.sceneBounds.maxX - model.sceneBounds.minX : 1);
        controller.moveActiveSlice((deltaX - deltaY) * span * 0.0015);
      }
      drag = {
        ...drag,
        lastX: point.x,
        lastY: point.y
      };
      controller.updatePointer(point.x, point.y);
      return;
    }
    controller.updatePointer(point.x, point.y);
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
    if (!drag || drag.pointerId !== event.pointerId) {
      controller.handlePrimaryAction(point.x, point.y);
    }
    drag = null;
    if (element.hasPointerCapture(event.pointerId)) {
      element.releasePointerCapture(event.pointerId);
    }
    controller.updatePointer(point.x, point.y);
  }

  function handlePointerCancel(event: PointerEvent): void {
    const element = event.currentTarget;
    if (element instanceof HTMLDivElement && element.hasPointerCapture(event.pointerId)) {
      element.releasePointerCapture(event.pointerId);
    }
    drag = null;
  }

  function handlePointerLeave(): void {
    if (!drag) {
      controller?.clearPointer();
    }
  }

  function handleWheel(event: WheelEvent): void {
    if (!controller) {
      return;
    }
    controller.zoom(event.deltaY < 0 ? 1.08 : 0.92);
    event.preventDefault();
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (!controller) {
      return;
    }
    switch (event.key) {
      case "Escape":
        drag = null;
        controller.clearPointer();
        event.preventDefault();
        break;
      case "f":
      case "F":
        controller.fitToData();
        event.preventDefault();
        break;
      case "r":
      case "R":
        controller.resetView();
        event.preventDefault();
        break;
      case "c":
      case "C":
        controller.centerSelection();
        event.preventDefault();
        break;
    }
  }

  function resolveRequestedTool(): VolumeInterpretationChartTool {
    return interactions?.tool ?? tool;
  }

  function createInteractionState(toolName: VolumeInterpretationChartTool): VolumeInterpretationChartInteractionState {
    return {
      capabilities: {
        tools: [...VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.tools],
        actions: [...VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.actions]
      },
      tool: toolName
    };
  }

  function pointerPoint(event: PointerEvent | WheelEvent, element: HTMLDivElement): { x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
    };
  }

  function createRenderer(rendererName: VolumeInterpretationChartRenderer) {
    return rendererName === "placeholder"
      ? new VolumeInterpretationPlaceholderRenderer()
      : new VolumeInterpretationVtkRenderer();
  }
</script>

<div class="ophiolite-charts-volume-shell">
  <div class="ophiolite-charts-volume-lane">
    <div
      class="ophiolite-charts-volume-stage"
      style:width={`${stageSize.width}px`}
      style:height={`${stageSize.height}px`}
    >
      {#key renderer}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="ophiolite-charts-volume-host"
          tabindex="0"
          role="application"
          aria-label="Volume interpretation chart"
          aria-busy={loading}
          style:cursor={hostCursor ?? undefined}
          {@attach attachChartHost}
        ></div>
      {/key}
      {#if loading}
        <div class="ophiolite-charts-overlay">{emptyMessage}</div>
      {:else if errorMessage}
        <div class="ophiolite-charts-overlay ophiolite-charts-overlay-error">{errorMessage}</div>
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
    </div>
  </div>
</div>

<style>
  .ophiolite-charts-volume-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 360px;
    display: flex;
    overflow-x: auto;
    overflow-y: auto;
    background: #0d1822;
  }

  .ophiolite-charts-volume-lane {
    min-width: 100%;
    min-height: 100%;
    width: max-content;
    height: max-content;
    display: grid;
    place-items: start;
  }

  .ophiolite-charts-volume-stage {
    position: relative;
    min-height: 360px;
    flex: 0 0 auto;
    --ophiolite-charts-overlay-pad: 8px;
  }

  .ophiolite-charts-volume-host {
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
    background: rgba(13, 24, 34, 0.74);
    color: #ddeaf0;
    font: 600 14px/1.4 sans-serif;
    pointer-events: none;
  }

  .ophiolite-charts-overlay-error {
    color: #ffb6b6;
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
    top: var(--ophiolite-charts-overlay-pad);
    left: 50%;
    transform: translateX(-50%);
  }

  .ophiolite-charts-chart-anchor-top-right {
    top: var(--ophiolite-charts-overlay-pad);
    right: var(--ophiolite-charts-overlay-pad);
  }

  .ophiolite-charts-chart-anchor-bottom-right {
    right: var(--ophiolite-charts-overlay-pad);
    bottom: var(--ophiolite-charts-overlay-pad);
  }

  .ophiolite-charts-chart-anchor-bottom-left {
    left: var(--ophiolite-charts-overlay-pad);
    bottom: var(--ophiolite-charts-overlay-pad);
  }
</style>
