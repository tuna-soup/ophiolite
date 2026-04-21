import type {
  ComparisonMode,
  DisplayTransform,
  Horizon,
  HorizonSnapMode,
  InteractionCapabilities,
  InteractionEvent,
  InteractionMode,
  LassoPoint,
  OverlayPayload,
  PrimaryInteractionMode,
  SectionHorizonOverlay,
  SectionWellOverlay,
  SectionScalarOverlay,
  SectionPayload,
  SectionViewport,
  ViewerState
} from "@ophiolite/charts-data-models";
import { resolveLogicalSectionDimensions } from "@ophiolite/charts-data-models";
import { InteractionManager } from "@ophiolite/charts-core";
import { clampViewport, fullViewport, zoomViewport, zoomViewportAt } from "./viewport";
import { getPlotRect, resolveNearestTraceIndex, type RendererAdapter } from "@ophiolite/charts-renderer";
import { buildCursorProbe, createHorizon, parseHorizons, recomputeHorizon, serializeHorizons, upsertAnchor } from "./interpretation";

const DEFAULT_DISPLAY: DisplayTransform = {
  gain: 1,
  renderMode: "heatmap",
  colormap: "grayscale",
  polarity: "normal"
};

export class SeismicViewerController {
  private container: HTMLElement | null = null;
  private readonly renderer: RendererAdapter;
  readonly interactions = new InteractionManager(SEISMIC_INTERACTION_CAPABILITIES, "cursor", ["crosshair"]);
  private readonly listeners = new Set<(state: ViewerState) => void>();
  private readonly interactionEventListeners = new Set<(event: InteractionEvent) => void>();
  private state: ViewerState = {
    section: null,
    secondarySection: null,
    viewport: null,
    displayTransform: { ...DEFAULT_DISPLAY },
    overlay: null,
    comparisonMode: "single",
    splitPosition: 0.5,
    interactionMode: "navigate",
    interactions: this.interactions.getState(),
    probe: null,
    sectionScalarOverlays: [],
    sectionHorizonOverlays: [],
    sectionWellOverlays: [],
    horizons: [],
    activeHorizonId: null
  };

  constructor(renderer: RendererAdapter) {
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

  setSection(section: SectionPayload): void {
    this.state.section = section;
    this.state.viewport = fullViewport(resolveLogicalSectionDimensions(section));
    this.state.overlay = section.overlay ?? null;
    this.state.sectionScalarOverlays = this.state.sectionScalarOverlays.filter((overlay) =>
      isCompatibleScalarOverlay(section, overlay)
    );
    if (!isCompatibleSection(section, this.state.secondarySection)) {
      this.state.secondarySection = null;
      this.state.comparisonMode = "single";
    }
    this.state.probe = null;
    this.state.displayTransform = {
      ...DEFAULT_DISPLAY,
      ...section.displayDefaults
    };
    this.render();
  }

  setSecondarySection(section: SectionPayload | null): void {
    this.state.secondarySection = isCompatibleSection(this.state.section, section) ? section : null;
    if (!this.state.secondarySection) {
      this.state.comparisonMode = "single";
    }
    this.state.probe = null;
    this.render();
  }

  setComparisonMode(mode: ComparisonMode): void {
    this.state.comparisonMode =
      mode === "split" && this.state.displayTransform.renderMode === "heatmap" && this.state.secondarySection
        ? "split"
        : "single";
    this.state.probe = null;
    this.render();
  }

  setSplitPosition(position: number): void {
    this.state.splitPosition = clamp(position, 0.05, 0.95);
    this.render();
  }

  setViewport(viewport: SectionViewport): void {
    if (!this.state.section) {
      return;
    }
    this.state.viewport = clampViewport(viewport, resolveLogicalSectionDimensions(this.state.section));
    this.render();
  }

  fitToData(): void {
    if (!this.state.section) {
      return;
    }
    this.state.viewport = fullViewport(resolveLogicalSectionDimensions(this.state.section));
    this.render();
  }

  zoom(factor: number): void {
    if (!this.state.section || !this.state.viewport || factor <= 0) {
      return;
    }
    this.state.viewport = zoomViewport(
      this.state.viewport,
      resolveLogicalSectionDimensions(this.state.section),
      factor
    );
    this.render();
  }

  pan(deltaTrace: number, deltaSample: number): void {
    if (!this.state.section || !this.state.viewport) {
      return;
    }
    this.setViewport({
      traceStart: this.state.viewport.traceStart + deltaTrace,
      traceEnd: this.state.viewport.traceEnd + deltaTrace,
      sampleStart: this.state.viewport.sampleStart + deltaSample,
      sampleEnd: this.state.viewport.sampleEnd + deltaSample
    });
  }

  setDisplayTransform(patch: Partial<DisplayTransform>): void {
    this.state.displayTransform = {
      ...this.state.displayTransform,
      ...patch
    };
    if (this.state.displayTransform.renderMode !== "heatmap" && this.state.comparisonMode === "split") {
      this.state.comparisonMode = "single";
    }
    this.render();
  }

  setOverlay(overlay: OverlayPayload | null): void {
    this.state.overlay = overlay;
    this.render();
  }

  setInteractionMode(mode: InteractionMode): void {
    this.state.interactionMode = mode;
    this.render();
  }

  setSectionHorizonOverlays(overlays: readonly SectionHorizonOverlay[]): void {
    this.state.sectionHorizonOverlays = overlays.map(cloneSectionHorizonOverlay);
    this.render();
  }

  setSectionWellOverlays(overlays: readonly SectionWellOverlay[]): void {
    this.state.sectionWellOverlays = overlays.map(cloneSectionWellOverlay);
    this.render();
  }

  setSectionScalarOverlays(overlays: readonly SectionScalarOverlay[]): void {
    this.state.sectionScalarOverlays = this.state.section
      ? overlays.filter((overlay) => isCompatibleScalarOverlay(this.state.section!, overlay)).map(cloneSectionScalarOverlay)
      : [];
    this.render();
  }

  updatePointer(x: number, y: number, viewWidth: number, viewHeight: number): void {
    const activeSection = activeProbeSection(this.state, x, viewWidth, viewHeight);
    if (!activeSection || !this.state.viewport) {
      return;
    }
    const plotRect = getPlotRect(viewWidth, viewHeight);
    const interactionState = this.interactions.getState();
    if (interactionState.session?.kind === "zoomRect") {
      this.interactions.updateSession({
        kind: "zoomRect",
        origin: interactionState.session.origin,
        current: clampPointToPlot(x, y, plotRect)
      });
    }
    this.state.probe = buildCursorProbe(
      activeSection,
      this.state.viewport,
      this.state.displayTransform.renderMode,
      viewWidth,
      viewHeight,
      x,
      y
    );
    this.interactions.setHoverTarget(
      this.state.probe
        ? {
            kind: "curve-sample",
            chartId: activeSection.axis,
            traceIndex: this.state.probe.traceIndex,
            sampleIndex: this.state.probe.sampleIndex
          }
        : { kind: "empty-plot", chartId: activeSection.axis }
    );
    this.render();
  }

  clearPointer(): void {
    this.state.probe = null;
    this.interactions.setHoverTarget(null);
    this.render();
  }

  focus(): void {
    this.interactions.setFocused(true);
  }

  blur(): void {
    this.interactions.setFocused(false);
    this.interactions.cancelSession();
    this.clearPointer();
  }

  setPrimaryMode(mode: PrimaryInteractionMode): void {
    this.interactions.setPrimaryMode(mode);
  }

  toggleCrosshair(): void {
    this.interactions.toggleModifier("crosshair");
  }

  beginZoomRect(x: number, y: number, viewWidth: number, viewHeight: number): boolean {
    this.focus();
    if (!this.state.section || !this.state.viewport) {
      return false;
    }

    const plotRect = getPlotRect(viewWidth, viewHeight);
    if (!pointInRect({ x, y }, plotRect)) {
      return false;
    }

    const origin = clampPointToPlot(x, y, plotRect);
    this.interactions.beginSession({
      kind: "zoomRect",
      origin,
      current: origin
    });
    return true;
  }

  commitZoomRect(viewWidth: number, viewHeight: number): boolean {
    if (!this.state.section || !this.state.viewport) {
      return false;
    }
    const session = this.interactions.getState().session;
    if (session?.kind !== "zoomRect") {
      return false;
    }

    const nextViewport = viewportFromZoomRect(
      this.state.section,
      this.state.viewport,
      this.state.displayTransform.renderMode,
      viewWidth,
      viewHeight,
      session.origin,
      session.current
    );
    let changed = false;
    if (nextViewport) {
      this.state.viewport = clampViewport(
        nextViewport,
        resolveLogicalSectionDimensions(this.state.section)
      );
      changed = true;
    }

    this.interactions.commitSession();
    return changed;
  }

  zoomAt(x: number, y: number, viewWidth: number, viewHeight: number, factor: number): boolean {
    if (!this.state.section || !this.state.viewport || factor <= 0) {
      return false;
    }

    const plotRect = getPlotRect(viewWidth, viewHeight);
    if (!pointInRect({ x, y }, plotRect)) {
      return false;
    }

    const centerTrace = resolveNearestTraceIndex(
      this.state.section,
      this.state.viewport,
      this.state.displayTransform.renderMode,
      plotRect,
      x
    );
    const centerSample = sampleIndexFromScreenY(this.state.viewport, plotRect, y);
    this.state.viewport = zoomViewportAt(
      this.state.viewport,
      resolveLogicalSectionDimensions(this.state.section),
      factor,
      centerTrace,
      centerSample
    );
    this.render();
    return true;
  }

  cancelInteractionSession(): void {
    this.interactions.cancelSession();
  }

  onStateChange(listener: (state: ViewerState) => void): () => void {
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

  createNewHorizon(name: string, color: string, snapMode: HorizonSnapMode): string {
    const horizon = createHorizon(name, color, snapMode);
    this.state.horizons = [...this.state.horizons, horizon];
    this.state.activeHorizonId = horizon.id;
    this.render();
    return horizon.id;
  }

  setActiveHorizon(id: string | null): void {
    this.state.activeHorizonId = id;
    this.render();
  }

  setActiveHorizonSnapMode(mode: HorizonSnapMode): void {
    if (!this.state.section || !this.state.activeHorizonId) {
      return;
    }
    this.state.horizons = this.state.horizons.map((horizon) =>
      horizon.id === this.state.activeHorizonId ? recomputeHorizon(this.state.section!, { ...horizon, snapMode: mode }) : horizon
    );
    this.render();
  }

  handlePrimaryAction(x: number, y: number, viewWidth: number, viewHeight: number): void {
    if (this.state.interactionMode !== "interpret" || !this.state.section || !this.state.viewport) {
      return;
    }
    const probe = buildCursorProbe(
      this.state.section,
      this.state.viewport,
      this.state.displayTransform.renderMode,
      viewWidth,
      viewHeight,
      x,
      y
    );
    if (!probe) {
      return;
    }

    if (!this.state.activeHorizonId) {
      this.createNewHorizon(`Horizon ${this.state.horizons.length + 1}`, "#ef4444", "peak");
    }

    this.state.probe = probe;
    this.state.horizons = this.state.horizons.map((horizon) => {
      if (horizon.id !== this.state.activeHorizonId) {
        return horizon;
      }
      return upsertAnchor(this.state.section!, horizon, {
        id: `anchor-${probe.traceIndex}`,
        traceIndex: probe.traceIndex,
        sampleIndex: probe.sampleIndex
      });
    });
    this.render();
  }

  exportHorizons(): string {
    return serializeHorizons(this.state.horizons);
  }

  importHorizons(json: string): void {
    const horizons = parseHorizons(json);
    this.state.horizons = this.state.section
      ? horizons.map((horizon) => recomputeHorizon(this.state.section!, horizon))
      : horizons;
    this.state.activeHorizonId = this.state.horizons[0]?.id ?? null;
    this.render();
  }

  getState(): ViewerState {
    return {
      section: this.state.section,
      secondarySection: this.state.secondarySection,
      viewport: this.state.viewport ? { ...this.state.viewport } : null,
      displayTransform: { ...this.state.displayTransform },
      overlay: this.state.overlay,
      comparisonMode: this.state.comparisonMode,
      splitPosition: this.state.splitPosition,
      interactionMode: this.state.interactionMode,
      interactions: this.interactions.getState(),
      probe: this.state.probe ? { ...this.state.probe } : null,
      sectionScalarOverlays: [...this.state.sectionScalarOverlays],
      sectionHorizonOverlays: this.state.sectionHorizonOverlays.map(cloneSectionHorizonOverlay),
      sectionWellOverlays: this.state.sectionWellOverlays.map(cloneSectionWellOverlay),
      horizons: this.state.horizons.map(cloneHorizon),
      activeHorizonId: this.state.activeHorizonId
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

export { SeismicViewerController as SeismicSectionChart };

export const SEISMIC_INTERACTION_CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom"],
  modifiers: ["crosshair"]
};

function viewportFromZoomRect(
  section: SectionPayload,
  viewport: SectionViewport,
  renderMode: DisplayTransform["renderMode"],
  viewWidth: number,
  viewHeight: number,
  origin: LassoPoint,
  current: LassoPoint
): SectionViewport | null {
  const plotRect = getPlotRect(viewWidth, viewHeight);
  const left = Math.max(plotRect.x, Math.min(origin.x, current.x));
  const right = Math.min(plotRect.x + plotRect.width, Math.max(origin.x, current.x));
  const top = Math.max(plotRect.y, Math.min(origin.y, current.y));
  const bottom = Math.min(plotRect.y + plotRect.height, Math.max(origin.y, current.y));

  if (right - left < 6 || bottom - top < 6) {
    return null;
  }

  const traceStart = resolveNearestTraceIndex(section, viewport, renderMode, plotRect, left);
  const traceEnd = resolveNearestTraceIndex(section, viewport, renderMode, plotRect, right) + 1;
  const sampleStart = sampleIndexFromScreenY(viewport, plotRect, top);
  const sampleEnd = sampleIndexFromScreenY(viewport, plotRect, bottom) + 1;

  if (traceEnd - traceStart < 2 || sampleEnd - sampleStart < 2) {
    return null;
  }

  return {
    traceStart: Math.min(traceStart, traceEnd - 1),
    traceEnd: Math.max(traceStart + 1, traceEnd),
    sampleStart: Math.min(sampleStart, sampleEnd - 1),
    sampleEnd: Math.max(sampleStart + 1, sampleEnd)
  };
}

function sampleIndexFromScreenY(viewport: SectionViewport, plotRect: ReturnType<typeof getPlotRect>, y: number): number {
  const ratio = clamp((y - plotRect.y) / Math.max(1, plotRect.height), 0, 1);
  return clamp(
    Math.round(ratio * Math.max(1, viewport.sampleEnd - viewport.sampleStart - 1)) + viewport.sampleStart,
    viewport.sampleStart,
    viewport.sampleEnd - 1
  );
}

function clampPointToPlot(x: number, y: number, plotRect: ReturnType<typeof getPlotRect>): LassoPoint {
  return {
    x: clamp(x, plotRect.x, plotRect.x + plotRect.width),
    y: clamp(y, plotRect.y, plotRect.y + plotRect.height)
  };
}

function pointInRect(point: LassoPoint, plotRect: ReturnType<typeof getPlotRect>): boolean {
  return (
    point.x >= plotRect.x &&
    point.x <= plotRect.x + plotRect.width &&
    point.y >= plotRect.y &&
    point.y <= plotRect.y + plotRect.height
  );
}

function activeProbeSection(
  state: ViewerState,
  _x: number,
  _viewWidth: number,
  _viewHeight: number
): SectionPayload | null {
  return state.section;
}

function isCompatibleSection(primary: SectionPayload | null, secondary: SectionPayload | null): boolean {
  if (!primary || !secondary) {
    return false;
  }

  return (
    primary.axis === secondary.axis &&
    primary.coordinate.index === secondary.coordinate.index &&
    approximatelyEqual(primary.coordinate.value, secondary.coordinate.value) &&
    resolveLogicalSectionDimensions(primary).traces === resolveLogicalSectionDimensions(secondary).traces &&
    resolveLogicalSectionDimensions(primary).samples === resolveLogicalSectionDimensions(secondary).samples
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function cloneHorizon(horizon: Horizon): Horizon {
  return {
    ...horizon,
    anchors: horizon.anchors.map((anchor) => ({ ...anchor })),
    picks: horizon.picks.map((pick) => ({ ...pick }))
  };
}

function cloneSectionHorizonOverlay(overlay: SectionHorizonOverlay): SectionHorizonOverlay {
  return {
    ...overlay,
    samples: overlay.samples.map((sample) => ({ ...sample }))
  };
}

function cloneSectionWellOverlay(overlay: SectionWellOverlay): SectionWellOverlay {
  return {
    ...overlay,
    diagnostics: overlay.diagnostics ? [...overlay.diagnostics] : undefined,
    segments: overlay.segments.map((segment) => ({
      samples: segment.samples.map((sample) => ({ ...sample })),
      notes: segment.notes ? [...segment.notes] : undefined
    }))
  };
}

function cloneSectionScalarOverlay(overlay: SectionScalarOverlay): SectionScalarOverlay {
  return {
    ...overlay,
    values: new Float32Array(overlay.values),
    valueRange: overlay.valueRange ? { ...overlay.valueRange } : undefined
  };
}

function cloneInteractionEvent(event: InteractionEvent): InteractionEvent {
  return JSON.parse(JSON.stringify(event)) as InteractionEvent;
}

function isCompatibleScalarOverlay(section: SectionPayload, overlay: SectionScalarOverlay): boolean {
  return (
    overlay.width === section.dimensions.traces &&
    overlay.height === section.dimensions.samples &&
    overlay.values.length === overlay.width * overlay.height
  );
}

function approximatelyEqual(left: number, right: number): boolean {
  return Math.abs(left - right) <= 1e-6;
}
