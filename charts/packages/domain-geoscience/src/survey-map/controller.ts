import {
  clampSurveyMapViewport,
  computeSurveyMapBounds,
  fitSurveyMapViewport,
  getSurveyMapPlotRect,
  pointInRect,
  resolveSurveyMapViewMetrics,
  screenToWorld,
  worldToScreen,
  type SurveyMapBounds
} from "@ophiolite/charts-core";
import { InteractionManager } from "@ophiolite/charts-core";
import type {
  ChartInteractionStyle,
  ChartRendererTelemetryEvent,
  InteractionCapabilities,
  InteractionTrigger,
  InteractionEvent,
  SurveyMapModel,
  SurveyMapPoint,
  SurveyMapProbe,
  SurveyMapScalarField,
  SurveyMapViewport
} from "@ophiolite/charts-data-models";
import type { SurveyMapRendererAdapter, SurveyMapViewState } from "@ophiolite/charts-renderer";

const SURVEY_MAP_INTERACTION_CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom"],
  modifiers: []
};
const SURVEY_MAP_INTERACTION_STYLE: ChartInteractionStyle = {
  id: "survey-map-navigation",
  label: "Survey Map Navigation",
  manipulators: ["viewport-navigation"],
  bindings: [
    {
      trigger: "pointer-primary",
      primaryMode: "panZoom",
      command: "viewport.pan"
    },
    {
      trigger: "pointer-primary",
      primaryMode: "cursor",
      command: "selection.primary"
    },
    {
      trigger: "wheel",
      command: "viewport.zoomAtCursor"
    },
    {
      trigger: "keyboard",
      key: "ArrowLeft",
      command: "viewport.panLeft"
    },
    {
      trigger: "keyboard",
      key: "ArrowRight",
      command: "viewport.panRight"
    },
    {
      trigger: "keyboard",
      key: "ArrowUp",
      command: "viewport.panUp"
    },
    {
      trigger: "keyboard",
      key: "ArrowDown",
      command: "viewport.panDown"
    }
  ]
};
const SURVEY_MAP_FIT_PADDING = 0.08;

export class SurveyMapController {
  private container: HTMLElement | null = null;
  private readonly renderer: SurveyMapRendererAdapter;
  readonly interactions = new InteractionManager(
    SURVEY_MAP_INTERACTION_CAPABILITIES,
    "cursor",
    [],
    SURVEY_MAP_INTERACTION_STYLE
  );
  private readonly listeners = new Set<(state: SurveyMapViewState) => void>();
  private readonly interactionEventListeners = new Set<(event: InteractionEvent) => void>();
  private readonly rendererTelemetryListeners = new Set<(event: ChartRendererTelemetryEvent) => void>();
  private state: SurveyMapViewState = {
    map: null,
    viewport: null,
    probe: null,
    selectedWellId: null,
    interactions: this.interactions.getState()
  };
  private worldBounds: SurveyMapBounds | null = null;

  constructor(renderer: SurveyMapRendererAdapter) {
    this.renderer = renderer;
    this.renderer.setTelemetryListener((event) => {
      this.emitRendererTelemetry(event);
    });
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

  setMap(map: SurveyMapModel | null): void {
    this.state.map = map ? cloneMap(map) : null;
    this.state.probe = null;
    this.state.selectedWellId = null;
    this.worldBounds = mapBoundsWithPadding(this.state.map);
    this.state.viewport = fitSurveyMapViewport(this.worldBounds, SURVEY_MAP_FIT_PADDING);
    this.interactions.setHoverTarget(null);
    this.render();
  }

  fitToData(): void {
    this.state.viewport = fitSurveyMapViewport(this.worldBounds, SURVEY_MAP_FIT_PADDING);
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

  setViewport(viewport: SurveyMapViewport | null): void {
    this.state.viewport = clampSurveyMapViewport(this.worldBounds, viewport);
    this.render();
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

  handlePrimaryPointerDown(): "pan" | "select" | null {
    this.focus();
    const command = this.resolveTriggerCommand("pointer-primary");
    if (command === "viewport.pan" && this.state.viewport) {
      return "pan";
    }
    if (command === "selection.primary" && this.state.map && this.state.viewport) {
      return "select";
    }
    return null;
  }

  handlePrimaryPointerUp(x: number, y: number, width: number, height: number, dragged: boolean): void {
    if (dragged) {
      return;
    }
    if (this.resolveTriggerCommand("pointer-primary") !== "selection.primary") {
      return;
    }
    this.selectAt(x, y, width, height);
    this.updatePointer(x, y, width, height);
  }

  handleKeyboardNavigation(key: string): boolean {
    if (!this.state.viewport) {
      return false;
    }

    const spanX = this.state.viewport.xMax - this.state.viewport.xMin;
    const spanY = this.state.viewport.yMax - this.state.viewport.yMin;
    const stepX = spanX * 0.08;
    const stepY = spanY * 0.08;

    switch (this.resolveTriggerCommand("keyboard", key)) {
      case "viewport.panLeft":
        this.pan(-stepX, 0);
        return true;
      case "viewport.panRight":
        this.pan(stepX, 0);
        return true;
      case "viewport.panUp":
        this.pan(0, stepY);
        return true;
      case "viewport.panDown":
        this.pan(0, -stepY);
        return true;
      default:
        return false;
    }
  }

  handleWheelAt(x: number, y: number, width: number, height: number, deltaY: number): boolean {
    if (!this.state.viewport || this.resolveTriggerCommand("wheel") !== "viewport.zoomAtCursor") {
      return false;
    }
    const plotRect = getSurveyMapPlotRect(width, height);
    const world = screenToWorld(x, y, this.state.viewport, plotRect);
    this.zoomAround(world.x, world.y, deltaY < 0 ? 1.12 : 0.9);
    return true;
  }

  updatePointer(x: number, y: number, width: number, height: number): void {
    if (!this.state.map || !this.state.viewport) {
      return;
    }

    const probe = buildProbe(this.state.map, this.state.viewport, width, height, x, y);
    this.state.probe = probe;
    this.interactions.setHoverTarget(targetFromProbe(this.state.map.id, probe));
    this.render();
  }

  clearPointer(): void {
    this.state.probe = null;
    this.interactions.setHoverTarget(null);
    this.render();
  }

  selectAt(x: number, y: number, width: number, height: number): void {
    if (!this.state.map || !this.state.viewport) {
      return;
    }

    const hit = hitTestWell(this.state.map, this.state.viewport, width, height, x, y);
    this.state.selectedWellId = hit?.id ?? null;
    this.render();
  }

  getState(): SurveyMapViewState {
    return {
      map: this.state.map ? cloneMap(this.state.map) : null,
      viewport: this.state.viewport ? { ...this.state.viewport } : null,
      probe: this.state.probe ? { ...this.state.probe } : null,
      selectedWellId: this.state.selectedWellId,
      interactions: this.interactions.getState()
    };
  }

  refresh(): void {
    this.render();
  }

  onStateChange(listener: (state: SurveyMapViewState) => void): () => void {
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

  onRendererTelemetry(listener: (event: ChartRendererTelemetryEvent) => void): () => void {
    this.rendererTelemetryListeners.add(listener);
    return () => {
      this.rendererTelemetryListeners.delete(listener);
    };
  }

  dispose(): void {
    this.renderer.setTelemetryListener(null);
    this.renderer.dispose();
    this.container = null;
  }

  private render(): void {
    if (!this.container) {
      return;
    }

    const state = this.getState();
    try {
      this.renderer.render({ state });
    } catch (error) {
      this.emitRendererTelemetry({
        kind: "frame-failed",
        phase: "render",
        backend: null,
        recoverable: true,
        message: error instanceof Error ? error.message : String(error),
        detail: "Survey map controller observed a renderer frame failure.",
        timestampMs: nowMs()
      });
    }
    for (const listener of this.listeners) {
      listener(state);
    }
  }

  private emitRendererTelemetry(event: ChartRendererTelemetryEvent): void {
    const clonedEvent = cloneRendererTelemetryEvent(event);
    for (const listener of this.rendererTelemetryListeners) {
      listener(clonedEvent);
    }
  }

  private resolveTriggerCommand(type: InteractionTrigger["type"], key?: string) {
    return this.interactions.resolveTriggerCommand({
      type,
      primaryMode: this.interactions.getState().primaryMode,
      modifiers: this.interactions.getState().modifiers,
      key
    })?.command ?? null;
  }
}

function buildProbe(
  map: SurveyMapModel,
  viewport: SurveyMapViewport,
  width: number,
  height: number,
  screenX: number,
  screenY: number
): SurveyMapProbe | null {
  const plotRect = getSurveyMapPlotRect(width, height);
  const drawRect = resolveSurveyMapViewMetrics(viewport, plotRect).drawRect;
  if (!pointInRect({ x: screenX, y: screenY }, drawRect)) {
    return null;
  }

  const world = screenToWorld(screenX, screenY, viewport, plotRect);
  const hitWell = hitTestWell(map, viewport, width, height, screenX, screenY);
  const survey = map.surveys.find((candidate) => pointInPolygon(world, candidate.outline)) ?? null;
  const scalarValue = map.scalarField ? sampleScalarField(map.scalarField, world) : undefined;

  return {
    x: world.x,
    y: world.y,
    scalarValue,
    scalarName: scalarValue !== undefined ? map.scalarField?.name : undefined,
    wellId: hitWell?.id,
    wellName: hitWell?.name,
    surveyId: survey?.id,
    surveyName: survey?.name,
    screenX,
    screenY
  };
}

function hitTestWell(
  map: SurveyMapModel,
  viewport: SurveyMapViewport,
  width: number,
  height: number,
  screenX: number,
  screenY: number
) {
  const plotRect = getSurveyMapPlotRect(width, height);
  let bestWell: SurveyMapModel["wells"][number] | null = null;
  let bestDistance = 9;

  for (const well of map.wells) {
    const surface = worldToScreen(well.surface, viewport, plotRect);
    bestDistance = bestWellDistance(surface, screenX, screenY, well, bestDistance, (nextDistance) => {
      bestWell = well;
      return nextDistance;
    });

    const trajectory = well.trajectory ?? [];
    for (let index = 1; index < trajectory.length; index += 1) {
      const left = worldToScreen(trajectory[index - 1]!, viewport, plotRect);
      const right = worldToScreen(trajectory[index]!, viewport, plotRect);
      const distance = pointToSegmentDistance({ x: screenX, y: screenY }, left, right);
      if (distance <= bestDistance) {
        bestDistance = distance;
        bestWell = well;
      }
    }
  }

  return bestWell;
}

function sampleScalarField(field: SurveyMapScalarField, world: SurveyMapPoint): number | undefined {
  const column = Math.round((world.x - field.origin.x) / Math.max(Math.abs(field.step.x), 1e-6));
  const row = Math.round((world.y - field.origin.y) / Math.max(Math.abs(field.step.y), 1e-6));
  if (column < 0 || column >= field.columns || row < 0 || row >= field.rows) {
    return undefined;
  }
  const value = field.values[row * field.columns + column];
  return Number.isFinite(value) ? value : undefined;
}

function targetFromProbe(chartId: string, probe: SurveyMapProbe | null) {
  if (!probe) {
    return null;
  }
  if (probe.wellId) {
    return {
      kind: "map-well" as const,
      chartId,
      wellId: probe.wellId
    };
  }
  if (probe.scalarValue !== undefined) {
    return {
      kind: "map-scalar-sample" as const,
      chartId
    };
  }
  if (probe.surveyId) {
    return {
      kind: "map-survey-outline" as const,
      chartId,
      entityId: probe.surveyId
    };
  }
  return null;
}

function bestWellDistance(
  point: SurveyMapPoint,
  screenX: number,
  screenY: number,
  well: SurveyMapModel["wells"][number],
  bestDistance: number,
  commit: (distance: number) => number
): number {
  const distance = Math.hypot(point.x - screenX, point.y - screenY);
  return distance <= bestDistance ? commit(distance) : bestDistance;
}

function pointToSegmentDistance(point: SurveyMapPoint, left: SurveyMapPoint, right: SurveyMapPoint): number {
  const dx = right.x - left.x;
  const dy = right.y - left.y;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared <= 1e-6) {
    return Math.hypot(point.x - left.x, point.y - left.y);
  }

  const t = clamp(((point.x - left.x) * dx + (point.y - left.y) * dy) / lengthSquared, 0, 1);
  const projectedX = left.x + t * dx;
  const projectedY = left.y + t * dy;
  return Math.hypot(point.x - projectedX, point.y - projectedY);
}

function pointInPolygon(point: SurveyMapPoint, polygon: SurveyMapPoint[]): boolean {
  let inside = false;
  for (let index = 0, previous = polygon.length - 1; index < polygon.length; previous = index, index += 1) {
    const left = polygon[index]!;
    const right = polygon[previous]!;
    const intersects =
      left.y > point.y !== right.y > point.y &&
      point.x < ((right.x - left.x) * (point.y - left.y)) / Math.max(1e-6, right.y - left.y) + left.x;
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}

function mapBoundsWithPadding(map: SurveyMapModel | null): SurveyMapBounds | null {
  return viewportToBounds(fitSurveyMapViewport(computeSurveyMapBounds(map)));
}

function viewportToBounds(viewport: SurveyMapViewport | null): SurveyMapBounds | null {
  return viewport
    ? {
        minX: viewport.xMin,
        maxX: viewport.xMax,
        minY: viewport.yMin,
        maxY: viewport.yMax
      }
    : null;
}

function cloneMap(map: SurveyMapModel): SurveyMapModel {
  return {
    ...map,
    surveys: map.surveys.map((survey) => ({
      ...survey,
      outline: survey.outline.map((point) => ({ ...point }))
    })),
    wells: map.wells.map((well) => ({
      ...well,
      surface: { ...well.surface },
      trajectory: well.trajectory?.map((point) => ({ ...point }))
    })),
    scalarField: map.scalarField
      ? {
          ...map.scalarField,
          origin: { ...map.scalarField.origin },
          step: { ...map.scalarField.step },
          values: new Float32Array(map.scalarField.values)
        }
      : null
  };
}

function cloneInteractionEvent(event: InteractionEvent): InteractionEvent {
  return JSON.parse(JSON.stringify(event)) as InteractionEvent;
}

function cloneRendererTelemetryEvent(event: ChartRendererTelemetryEvent): ChartRendererTelemetryEvent {
  return { ...event };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}
