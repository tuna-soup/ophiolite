import {
  clampRockPhysicsViewport,
  fitRockPhysicsViewport,
  getRockPhysicsCrossplotPlotRect,
  rockPhysicsScreenToValue,
  valueToRockPhysicsScreenX,
  valueToRockPhysicsScreenY
} from "@ophiolite/charts-core";
import { InteractionManager } from "@ophiolite/charts-core";
import type {
  InteractionCapabilities,
  InteractionEvent,
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotProbe,
  RockPhysicsCrossplotViewport
} from "@ophiolite/charts-data-models";
import type { RockPhysicsCrossplotRendererAdapter, RockPhysicsCrossplotViewState } from "@ophiolite/charts-renderer";

const ROCK_PHYSICS_INTERACTION_CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom"],
  modifiers: ["crosshair"]
};

const POINT_HIT_RADIUS_PX = 10;

export class RockPhysicsCrossplotController {
  private container: HTMLElement | null = null;
  private readonly renderer: RockPhysicsCrossplotRendererAdapter;
  readonly interactions = new InteractionManager(ROCK_PHYSICS_INTERACTION_CAPABILITIES, "cursor");
  private readonly listeners = new Set<(state: RockPhysicsCrossplotViewState) => void>();
  private readonly interactionEventListeners = new Set<(event: InteractionEvent) => void>();
  private state: RockPhysicsCrossplotViewState = {
    model: null,
    viewport: null,
    probe: null,
    interactions: this.interactions.getState()
  };

  constructor(renderer: RockPhysicsCrossplotRendererAdapter) {
    this.renderer = renderer;
    this.interactions.on((event) => {
      this.state.interactions = this.interactions.getState();
      this.render();
      for (const listener of this.interactionEventListeners) {
        listener(cloneInteractionEvent(event));
      }
    });
  }

  mount(container: HTMLElement): void {
    this.container = container;
    this.renderer.mount(container);
    this.render();
  }

  setModel(model: RockPhysicsCrossplotModel | null): void {
    this.state.model = model;
    this.state.viewport = fitRockPhysicsViewport(model);
    this.state.probe = null;
    this.interactions.setHoverTarget(null);
    this.render();
  }

  fitToData(): void {
    this.state.viewport = fitRockPhysicsViewport(this.state.model);
    this.render();
  }

  setViewport(viewport: RockPhysicsCrossplotViewport | null): void {
    this.state.viewport = clampRockPhysicsViewport(this.state.model, viewport);
    this.render();
  }

  zoom(factor: number): void {
    if (!this.state.viewport || factor <= 0) {
      return;
    }
    const centerX = (this.state.viewport.xMin + this.state.viewport.xMax) / 2;
    const centerY = (this.state.viewport.yMin + this.state.viewport.yMax) / 2;
    this.zoomAround(centerX, centerY, factor);
  }

  zoomAround(x: number, y: number, factor: number): void {
    if (!this.state.viewport || factor <= 0) {
      return;
    }

    const spanX = (this.state.viewport.xMax - this.state.viewport.xMin) / factor;
    const spanY = (this.state.viewport.yMax - this.state.viewport.yMin) / factor;
    const ratioX = (x - this.state.viewport.xMin) / Math.max(1e-6, this.state.viewport.xMax - this.state.viewport.xMin);
    const ratioY = (y - this.state.viewport.yMin) / Math.max(1e-6, this.state.viewport.yMax - this.state.viewport.yMin);

    this.setViewport({
      xMin: x - ratioX * spanX,
      xMax: x + (1 - ratioX) * spanX,
      yMin: y - ratioY * spanY,
      yMax: y + (1 - ratioY) * spanY
    });
  }

  pan(deltaX: number, deltaY: number): void {
    if (!this.state.viewport) {
      return;
    }
    this.setViewport({
      xMin: this.state.viewport.xMin + deltaX,
      xMax: this.state.viewport.xMax + deltaX,
      yMin: this.state.viewport.yMin + deltaY,
      yMax: this.state.viewport.yMax + deltaY
    });
  }

  focus(): void {
    this.interactions.setFocused(true);
  }

  blur(): void {
    this.interactions.setFocused(false);
    this.clearPointer();
  }

  setPrimaryMode(mode: "cursor" | "panZoom"): void {
    this.interactions.setPrimaryMode(mode);
  }

  toggleCrosshair(): void {
    this.interactions.toggleModifier("crosshair");
  }

  updatePointer(x: number, y: number, width: number, height: number): void {
    const { model, viewport } = this.state;
    if (!model || !viewport) {
      return;
    }

    const plotRect = getRockPhysicsCrossplotPlotRect(width, height);
    if (!pointInRect(x, y, plotRect)) {
      this.state.probe = null;
      this.interactions.setHoverTarget(null);
      this.render();
      return;
    }

    const hit = findNearestPoint(model, viewport, plotRect, x, y);
    this.state.probe = hit ? buildProbe(model, hit.index, x, y) : null;
    this.interactions.setHoverTarget(
      hit
        ? {
            kind: "point-cloud-sample",
            chartId: model.id,
            wellId: hit.wellId,
            entityId: String(hit.index)
          }
        : {
            kind: "empty-plot",
            chartId: model.id
          }
    );
    this.render();
  }

  clearPointer(): void {
    this.state.probe = null;
    this.interactions.setHoverTarget(null);
    this.render();
  }

  getState(): RockPhysicsCrossplotViewState {
    return {
      model: this.state.model,
      viewport: this.state.viewport ? { ...this.state.viewport } : null,
      probe: this.state.probe ? { ...this.state.probe } : null,
      interactions: this.interactions.getState()
    };
  }

  refresh(): void {
    this.render();
  }

  onStateChange(listener: (state: RockPhysicsCrossplotViewState) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  onInteractionEvent(listener: (event: InteractionEvent) => void): () => void {
    this.interactionEventListeners.add(listener);
    return () => {
      this.interactionEventListeners.delete(listener);
    };
  }

  dispose(): void {
    this.renderer.dispose();
    this.container = null;
  }

  private render(): void {
    if (!this.container) {
      return;
    }

    const state = this.getState();
    this.renderer.render({ state });
    for (const listener of this.listeners) {
      listener(state);
    }
  }
}

function findNearestPoint(
  model: RockPhysicsCrossplotModel,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>,
  screenX: number,
  screenY: number
): { index: number; wellId: string } | null {
  const interactionThresholds = model.interactionThresholds;
  const exactPointLimit = interactionThresholds?.exactPointLimit ?? 100_000;
  const stride = model.pointCount <= exactPointLimit ? 1 : Math.ceil(model.pointCount / exactPointLimit);
  let bestIndex = -1;
  let bestDistance = POINT_HIT_RADIUS_PX;

  for (let index = 0; index < model.pointCount; index += stride) {
    const pointX = valueToRockPhysicsScreenX(model.columns.x[index] ?? 0, viewport, plotRect);
    const pointY = valueToRockPhysicsScreenY(model.columns.y[index] ?? 0, viewport, plotRect);
    const distance = Math.hypot(pointX - screenX, pointY - screenY);
    if (distance <= bestDistance) {
      bestDistance = distance;
      bestIndex = index;
    }
  }

  if (bestIndex < 0) {
    return null;
  }

  const wellIndex = model.columns.wellIndices[bestIndex] ?? 0;
  return {
    index: bestIndex,
    wellId: model.wells[wellIndex]?.id ?? ""
  };
}

function buildProbe(
  model: RockPhysicsCrossplotModel,
  pointIndex: number,
  screenX: number,
  screenY: number
): RockPhysicsCrossplotProbe {
  const wellIndex = model.columns.wellIndices[pointIndex] ?? 0;
  const well = model.wells[wellIndex];

  return {
    pointIndex,
    wellId: well?.id ?? "",
    wellName: well?.name ?? "Unknown well",
    xValue: model.columns.x[pointIndex] ?? 0,
    yValue: model.columns.y[pointIndex] ?? 0,
    colorValue: model.columns.colorScalars?.[pointIndex],
    colorCategoryLabel: resolveColorCategoryLabel(model, pointIndex),
    sampleDepthM: model.columns.sampleDepthsM[pointIndex] ?? 0,
    screenX,
    screenY
  };
}

function resolveColorCategoryLabel(model: RockPhysicsCrossplotModel, pointIndex: number): string | undefined {
  if (model.colorBinding.kind !== "categorical" || !model.columns.colorCategoryIds) {
    return undefined;
  }
  const categoryId = model.columns.colorCategoryIds[pointIndex];
  return model.colorBinding.categories.find((category) => category.id === categoryId)?.label;
}

function pointInRect(
  x: number,
  y: number,
  rect: ReturnType<typeof getRockPhysicsCrossplotPlotRect>
): boolean {
  return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
}

function cloneInteractionEvent(event: InteractionEvent): InteractionEvent {
  return JSON.parse(JSON.stringify(event)) as InteractionEvent;
}
