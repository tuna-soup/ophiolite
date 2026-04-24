import {
  InteractionManager,
  buildWellCorrelationLayoutCache,
  hitTestWellTrack,
  mapNativeDepthToPanelDepth,
  mapPanelDepthToNativeDepth,
  normalizeWellPanelModel,
  trackHitsForWell,
  type WellCorrelationLayoutCache,
  type NormalizedCurveLayer,
  type NormalizedPointLayer,
  type NormalizedReferenceTrack,
  type NormalizedScalarTrack,
  type NormalizedSeismicSectionLayer,
  type NormalizedSeismicSectionTrack,
  type NormalizedSeismicTraceLayer,
  type NormalizedSeismicTraceTrack,
  type NormalizedTopOverlayLayer,
  type NormalizedTrack,
  type NormalizedWellColumn,
  type NormalizedWellPanelModel
} from "@ophiolite/charts-core";
import type {
  ChartInteractionStyle,
  ChartRendererTelemetryEvent,
  CorrelationMarkerLink,
  InteractionCapabilities,
  InteractionEvent,
  InteractionTarget,
  InteractionTrigger,
  LassoPoint,
  LassoSelectionResult,
  PrimaryInteractionMode,
  TrackAxis,
  WellCorrelationPanelModel,
  WellPanelModel,
  WellCorrelationProbe,
  WellCorrelationViewport
} from "@ophiolite/charts-data-models";
import type { WellCorrelationRendererAdapter, WellCorrelationViewState } from "@ophiolite/charts-renderer";

const DEFAULT_MARKER_COLOR = "#cc4d4d";
const WELL_CORRELATION_INTERACTION_STYLE: ChartInteractionStyle = {
  id: "well-panel-navigation",
  label: "Well Panel Navigation",
  manipulators: ["viewport-navigation", "crosshair-probe", "top-edit", "lasso-selection"],
  bindings: [
    {
      trigger: "pointer-primary",
      primaryMode: "panZoom",
      command: "viewport.pan"
    },
    {
      trigger: "pointer-primary",
      primaryMode: "topEdit",
      command: "topEdit.begin"
    },
    {
      trigger: "pointer-primary",
      primaryMode: "lassoSelect",
      command: "lasso.begin"
    },
    {
      trigger: "wheel",
      command: "viewport.pan"
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
    },
    {
      trigger: "keyboard",
      key: "Escape",
      command: "session.cancel"
    }
  ]
};

export class WellCorrelationController {
  private container: HTMLElement | null = null;
  private readonly renderer: WellCorrelationRendererAdapter;
  private layoutCache: WellCorrelationLayoutCache | null = null;
  readonly interactions = new InteractionManager(
    WELL_CORRELATION_INTERACTION_CAPABILITIES,
    "cursor",
    ["crosshair"],
    WELL_CORRELATION_INTERACTION_STYLE
  );
  private readonly listeners = new Set<(state: WellCorrelationViewState) => void>();
  private readonly interactionEventListeners = new Set<(event: InteractionEvent) => void>();
  private readonly rendererTelemetryListeners = new Set<(event: ChartRendererTelemetryEvent) => void>();
  private state: WellCorrelationViewState = {
    panel: null,
    viewport: null,
    probe: null,
    interactions: this.interactions.getState(),
    activeMarkerName: "Correlation Marker",
    activeMarkerColor: DEFAULT_MARKER_COLOR,
    correlationLines: [],
    previewCorrelationLines: null,
    previewTop: null
  };

  constructor(renderer: WellCorrelationRendererAdapter) {
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

  setPanel(panel: WellCorrelationPanelModel | WellPanelModel | null): void {
    if (!panel) {
      this.state.panel = null;
      this.state.viewport = null;
      this.state.probe = null;
      this.layoutCache = null;
      this.state.previewCorrelationLines = null;
      this.state.previewTop = null;
      this.state.correlationLines = [];
      this.interactions.cancelSession();
      this.interactions.setHoverTarget(null);
      this.render();
      return;
    }
    this.state.panel = normalizeWellPanelModel(panel);
    this.layoutCache = null;
    this.state.viewport = {
      depthStart: panel.depthDomain.start,
      depthEnd: panel.depthDomain.end
    };
    this.state.probe = null;
    this.state.previewCorrelationLines = null;
    this.state.previewTop = null;
    this.interactions.cancelSession();
    this.interactions.setHoverTarget(null);
    this.recomputeCorrelationLines();
    this.render();
  }

  fitToData(): void {
    if (!this.state.panel) {
      return;
    }
    this.state.viewport = {
      depthStart: this.state.panel.depthDomain.start,
      depthEnd: this.state.panel.depthDomain.end
    };
    this.render();
  }

  zoomVertical(factor: number): void {
    if (!this.state.panel || !this.state.viewport || factor <= 0) {
      return;
    }
    const center = (this.state.viewport.depthStart + this.state.viewport.depthEnd) / 2;
    const span = (this.state.viewport.depthEnd - this.state.viewport.depthStart) / factor;
    this.setViewport({
      depthStart: center - span / 2,
      depthEnd: center + span / 2
    });
  }

  zoomVerticalAround(panelDepth: number, factor: number): void {
    if (!this.state.panel || !this.state.viewport || factor <= 0) {
      return;
    }
    const currentSpan = this.state.viewport.depthEnd - this.state.viewport.depthStart;
    const nextSpan = Math.max(20, Math.min(this.state.panel.depthDomain.end - this.state.panel.depthDomain.start, currentSpan / factor));
    const ratio =
      currentSpan <= 0 ? 0.5 : (panelDepth - this.state.viewport.depthStart) / Math.max(1e-6, currentSpan);
    this.setViewport({
      depthStart: panelDepth - ratio * nextSpan,
      depthEnd: panelDepth + (1 - ratio) * nextSpan
    });
  }

  panVertical(deltaDepth: number): void {
    if (!this.state.viewport) {
      return;
    }
    this.setViewport({
      depthStart: this.state.viewport.depthStart + deltaDepth,
      depthEnd: this.state.viewport.depthEnd + deltaDepth
    });
  }

  setViewport(viewport: WellCorrelationViewport): void {
    if (!this.state.panel) {
      return;
    }
    const fullStart = this.state.panel.depthDomain.start;
    const fullEnd = this.state.panel.depthDomain.end;
    const span = Math.max(20, Math.min(fullEnd - fullStart, viewport.depthEnd - viewport.depthStart));
    const depthStart = clamp(viewport.depthStart, fullStart, fullEnd - span);
    this.state.viewport = {
      depthStart,
      depthEnd: depthStart + span
    };
    this.render();
  }

  setActiveMarker(name: string, color: string = DEFAULT_MARKER_COLOR): void {
    this.state.activeMarkerName = name;
    this.state.activeMarkerColor = color;
    this.render();
  }

  focus(): void {
    this.interactions.setFocused(true);
  }

  blur(): void {
    this.interactions.setFocused(false);
    this.clearPointer();
  }

  setPrimaryMode(mode: PrimaryInteractionMode): void {
    this.cancelPreviewIfNeeded();
    this.interactions.setPrimaryMode(mode);
  }

  toggleCrosshair(): void {
    this.interactions.toggleModifier("crosshair");
  }

  updatePointer(x: number, y: number, width: number, height: number): void {
    this.handlePointerMove(x, y, width, height);
  }

  handlePrimaryPointerDown(x: number, y: number, width: number, height: number): "pan" | "session" | null {
    this.focus();
    switch (this.resolveTriggerCommand("pointer-primary")) {
      case "viewport.pan":
        return this.state.viewport ? "pan" : null;
      case "topEdit.begin":
        this.beginTopEditAt(x, y, width, height);
        return this.interactions.getState().session ? "session" : null;
      case "lasso.begin":
        this.beginLassoAt(x, y);
        return this.interactions.getState().session ? "session" : null;
      default:
        return null;
    }
  }

  handlePrimaryPointerUp(): void {
    const session = this.interactions.getState().session;
    if (!session) {
      return;
    }
    if (session.kind === "topEdit" && this.state.panel && this.state.previewTop) {
      this.state.panel = applyTopPreview(this.state.panel, this.state.previewTop);
      this.recomputeCorrelationLines();
      this.state.previewCorrelationLines = null;
      this.state.previewTop = null;
      this.interactions.commitSession();
      this.render();
      return;
    }
    if (session.kind === "lasso") {
      this.interactions.commitSession();
      this.render();
    }
  }

  handleKeyboardNavigation(key: string): boolean {
    if (!this.state.viewport) {
      return false;
    }

    const step = Math.max(10, (this.state.viewport.depthEnd - this.state.viewport.depthStart) * 0.1);
    switch (this.resolveTriggerCommand("keyboard", key)) {
      case "viewport.panUp":
        this.panVertical(-step);
        return true;
      case "viewport.panDown":
        this.panVertical(step);
        return true;
      case "session.cancel":
        this.cancelPreviewIfNeeded();
        this.interactions.cancelSession();
        this.render();
        return true;
      default:
        return false;
    }
  }

  handleWheelAt(
    y: number,
    deltaY: number,
    width: number,
    height: number,
    zoomAroundPointer: boolean
  ): boolean {
    if (this.resolveTriggerCommand("wheel") !== "viewport.pan") {
      return false;
    }
    if (zoomAroundPointer) {
      const panelDepth = this.getPanelDepthAtViewY(y, width, height);
      if (panelDepth === null) {
        return false;
      }
      this.zoomVerticalAround(panelDepth, deltaY < 0 ? 1.12 : 0.89);
      return true;
    }
    this.panVertical(deltaY * 0.35);
    return true;
  }

  handlePointerMove(x: number, y: number, width: number, height: number): void {
    if (!this.state.panel || !this.state.viewport) {
      return;
    }
    const layoutCache = this.getLayoutCache(this.state.panel, width, height);
    this.state.probe = buildProbe(layoutCache, this.state.viewport, x, y);
    const interactionState = this.interactions.getState();
    const topTarget =
      interactionState.primaryMode === "topEdit"
        ? hitTestTopLine(layoutCache, this.state.viewport, x, y)
        : null;
    this.interactions.setHoverTarget(topTarget ?? targetFromProbe(this.state.panel.id, this.state.probe));

    if (interactionState.session?.kind === "topEdit") {
      const previewTarget = interactionState.session.target;
      if (previewTarget.wellId && previewTarget.entityId) {
        const preview = buildTopPreview(
          layoutCache,
          this.state.viewport,
          previewTarget.wellId,
          previewTarget.entityId,
          y
        );
        if (preview) {
          this.state.previewTop = preview;
          this.state.previewCorrelationLines = deriveCorrelationLines(applyTopPreview(this.state.panel, preview));
          this.interactions.updateSession({
            ...interactionState.session,
            previewNativeDepth: preview.nativeDepth,
            previewPanelDepth: preview.panelDepth
          });
        }
      }
    } else if (interactionState.session?.kind === "lasso") {
      const nextPoints = appendLassoPoint(interactionState.session.points, { x, y });
      const selection = buildLassoSelection(layoutCache, this.state.viewport, nextPoints);
      this.interactions.updateSession({
        kind: "lasso",
        points: nextPoints,
        selection
      });
    }
    this.render();
  }

  clearPointer(): void {
    this.state.probe = null;
    if (!this.interactions.getState().session) {
      this.interactions.setHoverTarget(null);
    }
    this.render();
  }

  handlePointerDown(x: number, y: number, width: number, height: number): void {
    this.handlePrimaryPointerDown(x, y, width, height);
  }

  handlePointerUp(): void {
    this.handlePrimaryPointerUp();
  }

  handleKeyDown(key: string): void {
    this.handleKeyboardNavigation(key);
  }

  getPanelDepthAtViewY(y: number, width: number, height: number): number | null {
    if (!this.state.panel || !this.state.viewport) {
      return null;
    }
    const layoutCache = this.getLayoutCache(this.state.panel, width, height);
    if (y < layoutCache.layout.plotRect.y || y > layoutCache.layout.plotRect.y + layoutCache.layout.plotRect.height) {
      return null;
    }
    return screenYToDepth(layoutCache.layout.plotRect, this.state.viewport, y);
  }

  pickMarker(x: number, y: number, width: number, height: number): void {
    if (!this.state.panel || !this.state.viewport || !this.state.activeMarkerName) {
      return;
    }

    const panel = clonePanel(this.state.panel);
    const layoutCache = this.getLayoutCache(panel, width, height);
    const hit = hitTestWellTrack(layoutCache, x, y);
    if (!hit) {
      return;
    }

    const panelDepth = screenYToDepth(hit.column.bodyRect, this.state.viewport, y);
    const nativeDepth = mapPanelDepthToNativeDepth(hit.well.panelDepthMapping, panelDepth);
    const overlay = topOverlaysForTrack(hit.track)[0] ?? hit.well.tracks.flatMap((track) => topOverlaysForTrack(track))[0];
    if (!overlay) {
      return;
    }
    const existing = overlay.tops.find((top) => top.name === this.state.activeMarkerName);
    const pickedTop = {
      id: existing?.id ?? `${hit.well.id}-${slug(this.state.activeMarkerName)}`,
      name: this.state.activeMarkerName,
      nativeDepth,
      color: this.state.activeMarkerColor,
      source: "picked" as const
    };

    overlay.tops = [
      ...overlay.tops.filter((top) => top.name !== this.state.activeMarkerName),
      pickedTop
    ].sort((left, right) => left.nativeDepth - right.nativeDepth);
    this.state.panel = panel;
    this.layoutCache = null;
    this.recomputeCorrelationLines();
    this.state.probe = buildProbe(this.getLayoutCache(panel, width, height), this.state.viewport, x, y);
    this.render();
  }

  getState(): WellCorrelationViewState {
    return {
      panel: this.state.panel,
      viewport: this.state.viewport ? { ...this.state.viewport } : null,
      probe: this.state.probe ? { ...this.state.probe } : null,
      interactions: this.interactions.getState(),
      activeMarkerName: this.state.activeMarkerName,
      activeMarkerColor: this.state.activeMarkerColor,
      correlationLines: this.state.correlationLines,
      previewCorrelationLines: this.state.previewCorrelationLines ? this.state.previewCorrelationLines.map(cloneCorrelationLine) : null,
      previewTop: this.state.previewTop ? { ...this.state.previewTop } : null
    };
  }

  refresh(): void {
    this.render();
  }

  onStateChange(listener: (state: WellCorrelationViewState) => void): () => void {
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
    this.layoutCache = null;
  }

  private recomputeCorrelationLines(): void {
    this.state.correlationLines = this.state.panel ? deriveCorrelationLines(this.state.panel) : [];
  }

  private cancelPreviewIfNeeded(): void {
    this.state.previewCorrelationLines = null;
    this.state.previewTop = null;
  }

  private beginTopEditAt(x: number, y: number, width: number, height: number): void {
    if (!this.state.panel || !this.state.viewport) {
      return;
    }
    const layoutCache = this.getLayoutCache(this.state.panel, width, height);
    const target = hitTestTopLine(layoutCache, this.state.viewport, x, y);
    if (!target || !target.wellId || !target.entityId) {
      return;
    }
    const preview = buildTopPreview(layoutCache, this.state.viewport, target.wellId, target.entityId, y);
    if (!preview) {
      return;
    }
    this.state.previewTop = preview;
    this.state.previewCorrelationLines = deriveCorrelationLines(applyTopPreview(this.state.panel, preview));
    this.interactions.beginSession({
      kind: "topEdit",
      target,
      originalNativeDepth: preview.nativeDepth,
      previewNativeDepth: preview.nativeDepth,
      previewPanelDepth: preview.panelDepth
    });
    this.render();
  }

  private beginLassoAt(x: number, y: number): void {
    if (!this.state.panel || !this.state.viewport) {
      return;
    }
    this.interactions.beginSession({
      kind: "lasso",
      points: [{ x, y }],
      selection: null
    });
    this.render();
  }

  private getLayoutCache(
    panel: NormalizedWellPanelModel,
    width: number,
    height: number
  ): WellCorrelationLayoutCache {
    if (
      this.layoutCache &&
      this.layoutCache.panel === panel &&
      this.layoutCache.width === width &&
      this.layoutCache.height === height
    ) {
      return this.layoutCache;
    }
    this.layoutCache = buildWellCorrelationLayoutCache(panel, width, height);
    return this.layoutCache;
  }

  private resolveTriggerCommand(type: InteractionTrigger["type"], key?: string) {
    return this.interactions.resolveTriggerCommand({
      type,
      primaryMode: this.interactions.getState().primaryMode,
      modifiers: this.interactions.getState().modifiers,
      key
    })?.command ?? null;
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
        detail: "Well correlation controller observed a renderer frame failure.",
        timestampMs: nowMs()
      });
    }
    for (const listener of this.listeners) {
      listener(state);
    }
  }

  private emitRendererTelemetry(event: ChartRendererTelemetryEvent): void {
    const clonedEvent = { ...event };
    for (const listener of this.rendererTelemetryListeners) {
      listener(clonedEvent);
    }
  }
}

export { WellCorrelationController as WellCorrelationPanel };

const WELL_CORRELATION_INTERACTION_CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom", "topEdit", "lassoSelect"],
  modifiers: ["crosshair"]
};

function buildProbe(
  layoutCache: WellCorrelationLayoutCache,
  viewport: WellCorrelationViewport,
  x: number,
  y: number
): WellCorrelationProbe | null {
  const hit = hitTestWellTrack(layoutCache, x, y);
  if (!hit) {
    return null;
  }

  const panelDepth = screenYToDepth(hit.trackFrame.bodyRect, viewport, y);
  const nativeDepth = mapPanelDepthToNativeDepth(hit.well.panelDepthMapping, panelDepth);
  const marker = nearestTop(topsForTrack(hit.track), nativeDepth);

  if (marker && Math.abs(mapNativeDepthToPanelDepth(hit.well.panelDepthMapping, marker.nativeDepth) - panelDepth) < 6) {
    return {
      wellId: hit.well.id,
      wellName: hit.well.name,
      trackId: hit.trackFrame.trackId,
      trackTitle: hit.trackFrame.title,
      panelDepth,
      nativeDepth,
      markerName: marker.name,
      screenX: x,
      screenY: y
    };
  }

  if (hit.track.kind === "scalar") {
    const nearestPoint = nearestPointObservation(hit.track, hit.well.panelDepthMapping, viewport, x, y, hit.trackFrame.bodyRect);
    if (nearestPoint) {
      return {
        wellId: hit.well.id,
        wellName: hit.well.name,
        trackId: hit.trackFrame.trackId,
        trackTitle: hit.trackFrame.title,
        panelDepth,
        nativeDepth: nearestPoint.nativeDepth,
        seriesName: nearestPoint.layerName,
        value: nearestPoint.value,
        markerName: nearestPoint.label,
        screenX: x,
        screenY: y,
        kind: "point-observation",
        entityId: nearestPoint.pointId
      };
    }
    const nearest = nearestCurveSample(hit.track, hit.well.panelDepthMapping, panelDepth);
    return {
      wellId: hit.well.id,
      wellName: hit.well.name,
      trackId: hit.trackFrame.trackId,
      trackTitle: hit.trackFrame.title,
      panelDepth,
      nativeDepth,
      seriesName: nearest?.seriesName,
      value: nearest?.value,
      screenX: x,
      screenY: y,
      kind: "curve-sample"
    };
  }

  if (hit.track.kind === "seismic-trace") {
    const sample = nearestSeismicTraceSample(hit.track, hit.trackFrame.bodyRect, viewport, x, y);
    return {
      wellId: hit.well.id,
      wellName: hit.well.name,
      trackId: hit.trackFrame.trackId,
      trackTitle: hit.trackFrame.title,
      panelDepth,
      nativeDepth,
      seriesName: sample?.traceName,
      value: sample?.amplitude,
      screenX: x,
      screenY: y,
      kind: "seismic-trace-sample",
      traceIndex: sample?.traceIndex,
      sampleIndex: sample?.sampleIndex
    };
  }

  if (hit.track.kind === "seismic-section") {
    const sample = nearestSeismicSectionSample(hit.track, hit.trackFrame.bodyRect, viewport, x, y);
    return {
      wellId: hit.well.id,
      wellName: hit.well.name,
      trackId: hit.trackFrame.trackId,
      trackTitle: hit.trackFrame.title,
      panelDepth,
      nativeDepth,
      value: sample?.amplitude,
      screenX: x,
      screenY: y,
      kind: "seismic-section-sample",
      traceIndex: sample?.traceIndex,
      sampleIndex: sample?.sampleIndex
    };
  }

  return {
    wellId: hit.well.id,
    wellName: hit.well.name,
    trackId: hit.trackFrame.trackId,
    trackTitle: hit.trackFrame.title,
    panelDepth,
    nativeDepth,
    screenX: x,
    screenY: y,
    kind: "reference"
  };
}

function hitTestTopLine(
  layoutCache: WellCorrelationLayoutCache,
  viewport: WellCorrelationViewport,
  x: number,
  y: number
): InteractionTarget | null {
  const hit = hitTestWellTrack(layoutCache, x, y);
  if (!hit) {
    return null;
  }
  let bestTop: ReturnType<typeof topsForTrack>[number] | null = null;
  let bestDistance = 6;
  for (const top of topsForTrack(hit.track)) {
    const panelDepth = mapNativeDepthToPanelDepth(hit.well.panelDepthMapping, top.nativeDepth);
    const lineY = depthToScreenY(hit.trackFrame.bodyRect, viewport, panelDepth);
    const distance = Math.abs(lineY - y);
    if (distance <= bestDistance) {
      bestDistance = distance;
      bestTop = top;
    }
  }
  if (!bestTop) {
    return null;
  }
  return {
    kind: "top-line",
    chartId: layoutCache.panel.id,
    wellId: hit.well.id,
    trackId: hit.trackFrame.trackId,
    entityId: bestTop.id,
    nativeDepth: bestTop.nativeDepth,
    panelDepth: mapNativeDepthToPanelDepth(hit.well.panelDepthMapping, bestTop.nativeDepth)
  };
}

function buildTopPreview(
  layoutCache: WellCorrelationLayoutCache,
  viewport: WellCorrelationViewport,
  wellId: string,
  topId: string,
  y: number
): { wellId: string; topId: string; nativeDepth: number; panelDepth: number } | null {
  const wellHits = trackHitsForWell(layoutCache, wellId);
  const column = wellHits[0]?.column;
  const well = wellHits[0]?.well;
  const hit = wellHits.find((candidate) => topsForTrack(candidate.track).some((item) => item.id === topId));
  const top = well?.tracks.flatMap((track) => topsForTrack(track)).find((candidate) => candidate.id === topId);
  if (!column || !well || !top || !hit) {
    return null;
  }
  const panelDepth = screenYToDepth(hit.trackFrame.bodyRect, viewport, y);
  const nativeDepth = mapPanelDepthToNativeDepth(well.panelDepthMapping, panelDepth);
  return { wellId, topId, nativeDepth, panelDepth };
}

function applyTopPreview(
  panel: NormalizedWellPanelModel,
  preview: { wellId: string; topId: string; nativeDepth: number }
): NormalizedWellPanelModel {
  const next = clonePanel(panel);
  const well = next.wells.find((candidate) => candidate.id === preview.wellId);
  if (!well) {
    return next;
  }
  for (const track of well.tracks) {
    for (const overlay of topOverlaysForTrack(track)) {
      const top = overlay.tops.find((candidate) => candidate.id === preview.topId);
      if (top) {
        top.nativeDepth = preview.nativeDepth;
        overlay.tops.sort((left, right) => left.nativeDepth - right.nativeDepth);
      }
    }
  }
  return next;
}

function appendLassoPoint(points: LassoPoint[], point: LassoPoint): LassoPoint[] {
  const previous = points[points.length - 1];
  if (previous && Math.hypot(previous.x - point.x, previous.y - point.y) < 4) {
    return points;
  }
  return [...points, point];
}

function buildLassoSelection(
  layoutCache: WellCorrelationLayoutCache,
  viewport: WellCorrelationViewport,
  points: LassoPoint[]
): LassoSelectionResult | null {
  if (points.length < 3) {
    return null;
  }
  const entities: LassoSelectionResult["entities"] = [];

  for (const hit of layoutCache.trackHits) {
    const { well, track, trackFrame } = hit;
    if (track.kind !== "scalar") {
      continue;
    }
    for (const layer of track.layers) {
        if (layer.kind !== "curve") {
          continue;
        }
        const series = layer.series;
        const axis = series.axis ?? track.xAxis;
        for (let index = 0; index < series.nativeDepths.length; index += 1) {
          const panelDepth = mapNativeDepthToPanelDepth(well.panelDepthMapping, series.nativeDepths[index]!);
          if (panelDepth < viewport.depthStart || panelDepth > viewport.depthEnd) {
            continue;
          }
          const samplePoint = {
            x: valueToTrackX(series.values[index]!, axis, trackFrame.bodyRect),
            y: depthToScreenY(trackFrame.bodyRect, viewport, panelDepth)
          };
          if (pointInPolygon(samplePoint, points)) {
            entities.push({
              kind: "curve-sample",
              chartId: layoutCache.panel.id,
              wellId: well.id,
              trackId: track.id,
              seriesId: layer.id,
              sourceIndex: index
            });
          }
        }
    }
  }

  return entities.length > 0
    ? {
        chartId: layoutCache.panel.id,
        targetKind: "curve-sample",
        entities
      }
    : null;
}

function targetFromProbe(chartId: string, probe: WellCorrelationProbe | null): InteractionTarget | null {
  if (!probe) {
    return null;
  }
  if (probe.markerName) {
    return {
      kind: "top-marker",
      chartId,
      wellId: probe.wellId,
      trackId: probe.trackId,
      entityId: probe.markerName,
      nativeDepth: probe.nativeDepth,
      panelDepth: probe.panelDepth
    };
  }
  if (probe.kind === "point-observation") {
    return {
      kind: "point-observation",
      chartId,
      wellId: probe.wellId,
      trackId: probe.trackId,
      entityId: probe.entityId,
      nativeDepth: probe.nativeDepth,
      panelDepth: probe.panelDepth
    };
  }
  if (probe.kind === "seismic-trace-sample") {
    return {
      kind: "seismic-trace-sample",
      chartId,
      wellId: probe.wellId,
      trackId: probe.trackId,
      nativeDepth: probe.nativeDepth,
      panelDepth: probe.panelDepth,
      traceIndex: probe.traceIndex,
      sampleIndex: probe.sampleIndex
    };
  }
  if (probe.kind === "seismic-section-sample") {
    return {
      kind: "seismic-section-sample",
      chartId,
      wellId: probe.wellId,
      trackId: probe.trackId,
      nativeDepth: probe.nativeDepth,
      panelDepth: probe.panelDepth,
      traceIndex: probe.traceIndex,
      sampleIndex: probe.sampleIndex
    };
  }
  return {
    kind: "curve-sample",
    chartId,
    wellId: probe.wellId,
    trackId: probe.trackId,
    panelDepth: probe.panelDepth,
    nativeDepth: probe.nativeDepth
  };
}

function screenYToDepth(
  rect: { x: number; y: number; width: number; height: number },
  viewport: WellCorrelationViewport,
  y: number
): number {
  const ratio = clamp((y - rect.y) / Math.max(1, rect.height), 0, 1);
  return viewport.depthStart + ratio * (viewport.depthEnd - viewport.depthStart);
}

function deriveCorrelationLines(panel: NormalizedWellPanelModel): CorrelationMarkerLink[] {
  const grouped = new Map<string, CorrelationMarkerLink>();
  for (const well of panel.wells) {
    const seenTopIds = new Set<string>();
    for (const top of allTopsForWell(well)) {
      if (seenTopIds.has(top.id)) {
        continue;
      }
      seenTopIds.add(top.id);
      const existing = grouped.get(top.name) ?? {
        name: top.name,
        color: top.color,
        points: []
      };
      existing.points.push({
        wellId: well.id,
        nativeDepth: top.nativeDepth,
        panelDepth: mapNativeDepthToPanelDepth(well.panelDepthMapping, top.nativeDepth)
      });
      grouped.set(top.name, existing);
    }
  }
  return [...grouped.values()];
}

function allTopsForWell(well: NormalizedWellColumn) {
  return well.tracks.flatMap((track) => topsForTrack(track));
}

function topOverlaysForTrack(track: NormalizedTrack): NormalizedTopOverlayLayer[] {
  if (track.kind === "reference") {
    return track.topOverlays;
  }
  return track.layers.filter((layer): layer is NormalizedTopOverlayLayer => layer.kind === "top-overlay");
}

function topsForTrack(track: NormalizedTrack) {
  return topOverlaysForTrack(track).flatMap((layer) => layer.tops);
}

function nearestTop(tops: ReturnType<typeof topsForTrack>, nativeDepth: number) {
  if (tops.length === 0) {
    return null;
  }
  return tops.reduce((best, candidate) =>
    Math.abs(candidate.nativeDepth - nativeDepth) < Math.abs(best.nativeDepth - nativeDepth) ? candidate : best
  );
}

function nearestCurveSample(
  track: NormalizedScalarTrack,
  mapping: { nativeDepth: number; panelDepth: number }[],
  panelDepth: number
): { seriesName: string; value: number } | null {
  let best: { seriesName: string; value: number } | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  const targetNativeDepth = mapPanelDepthToNativeDepth(mapping, panelDepth);
  for (const layer of track.layers) {
    if (layer.kind !== "curve") {
      continue;
    }
    const candidate = nearestSeriesSample(layer.series, mapping, panelDepth, targetNativeDepth);
    if (!candidate) {
      continue;
    }
    if (candidate.distance < bestDistance) {
      bestDistance = candidate.distance;
      best = {
        seriesName: candidate.seriesName,
        value: candidate.value
      };
    }
  }
  return best;
}

function nearestSeriesSample(
  series: NormalizedCurveLayer["series"],
  mapping: { nativeDepth: number; panelDepth: number }[],
  panelDepth: number,
  targetNativeDepth: number
): { seriesName: string; value: number; distance: number } | null {
  const bestIndex = nearestDepthIndex(series.nativeDepths, targetNativeDepth);
  if (bestIndex < 0) {
    return null;
  }
  return {
    seriesName: series.name,
    value: series.values[bestIndex]!,
    distance: Math.abs(mapNativeDepthToPanelDepth(mapping, series.nativeDepths[bestIndex]!) - panelDepth)
  };
}

function nearestDepthIndex(depths: ArrayLike<number>, target: number): number {
  if (depths.length === 0) {
    return -1;
  }

  let low = 0;
  let high = depths.length - 1;
  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    const value = depths[mid]!;
    if (value < target) {
      low = mid + 1;
    } else if (value > target) {
      high = mid - 1;
    } else {
      return mid;
    }
  }

  const rightIndex = clamp(low, 0, depths.length - 1);
  const leftIndex = clamp(rightIndex - 1, 0, depths.length - 1);
  return Math.abs(depths[leftIndex]! - target) <= Math.abs(depths[rightIndex]! - target) ? leftIndex : rightIndex;
}

function nearestPointObservation(
  track: NormalizedScalarTrack,
  mapping: { nativeDepth: number; panelDepth: number }[],
  viewport: WellCorrelationViewport,
  x: number,
  y: number,
  rect: { x: number; y: number; width: number; height: number }
): { layerName: string; value: number; nativeDepth: number; label?: string; pointId: string } | null {
  let best: { layerName: string; value: number; nativeDepth: number; label?: string; pointId: string } | null = null;
  let bestDistance = 10;
  for (const layer of track.layers) {
    if (layer.kind !== "point-observation") {
      continue;
    }
    for (const point of layer.points) {
      const pointPanelDepth = mapNativeDepthToPanelDepth(mapping, point.nativeDepth);
      const pointX = valueToTrackX(point.value, layer.axis, rect);
      const pointY = depthToScreenY(rect, viewport, pointPanelDepth);
      const distance = Math.hypot(pointX - x, pointY - y);
      if (distance <= bestDistance) {
        bestDistance = distance;
        best = {
          layerName: layer.name,
          value: point.value,
          nativeDepth: point.nativeDepth,
          label: point.label,
          pointId: point.id
        };
      }
    }
  }
  return best;
}

function nearestSeismicTraceSample(
  track: NormalizedSeismicTraceTrack,
  rect: { x: number; y: number; width: number; height: number },
  viewport: WellCorrelationViewport,
  x: number,
  y: number
): { traceIndex: number; traceName: string; sampleIndex: number; amplitude: number } | null {
  const layers = track.layers.filter((layer): layer is NormalizedSeismicTraceLayer => layer.kind === "seismic-trace");
  if (layers.length === 0) {
    return null;
  }
  const layer = layers[0]!;
  const traces = layer.traces;
  if (traces.length === 0) {
    return null;
  }
  const traceIndex = clamp(Math.floor(((x - rect.x) / Math.max(1, rect.width)) * traces.length), 0, traces.length - 1);
  const depths = layer.panelDepths ?? layer.nativeDepths;
  const sampleIndex = nearestDepthIndex(depths, screenYToDepth(rect, viewport, y));
  const trace = traces[traceIndex]!;
  return {
    traceIndex,
    traceName: trace.name,
    sampleIndex,
    amplitude: trace.amplitudes[sampleIndex] ?? 0
  };
}

function nearestSeismicSectionSample(
  track: NormalizedSeismicSectionTrack,
  rect: { x: number; y: number; width: number; height: number },
  viewport: WellCorrelationViewport,
  x: number,
  y: number
): { traceIndex: number; sampleIndex: number; amplitude: number } | null {
  const layers = track.layers.filter((layer): layer is NormalizedSeismicSectionLayer => layer.kind === "seismic-section");
  if (layers.length === 0) {
    return null;
  }
  const layer = layers[0]!;
  const traceIndex = clamp(
    Math.floor(((x - rect.x) / Math.max(1, rect.width)) * layer.section.dimensions.traces),
    0,
    layer.section.dimensions.traces - 1
  );
  const sampleIndex = nearestDepthIndex(layer.panelDepths, screenYToDepth(rect, viewport, y));
  const amplitude = layer.section.amplitudes[traceIndex * layer.section.dimensions.samples + sampleIndex] ?? 0;
  return { traceIndex, sampleIndex, amplitude };
}

function depthToScreenY(
  rect: { x: number; y: number; width: number; height: number },
  viewport: WellCorrelationViewport,
  depth: number
): number {
  return rect.y + ((depth - viewport.depthStart) / Math.max(1e-6, viewport.depthEnd - viewport.depthStart)) * rect.height;
}

function valueToTrackX(
  value: number,
  axis: TrackAxis,
  rect: { x: number; y: number; width: number; height: number }
): number {
  const ratio =
    axis.scale === "log"
      ? (Math.log10(Math.max(value, 1e-6)) - Math.log10(Math.max(axis.min, 1e-6))) /
        (Math.log10(Math.max(axis.max, axis.min * 1.0001)) - Math.log10(Math.max(axis.min, 1e-6)))
      : axis.max === axis.min
        ? 0.5
        : (value - axis.min) / (axis.max - axis.min);
  return rect.x + clamp(ratio, 0, 1) * rect.width;
}

function pointInPolygon(point: LassoPoint, polygon: LassoPoint[]): boolean {
  let inside = false;
  for (let left = 0, right = polygon.length - 1; left < polygon.length; right = left, left += 1) {
    const a = polygon[left]!;
    const b = polygon[right]!;
    const intersects =
      a.y > point.y !== b.y > point.y &&
      point.x < ((b.x - a.x) * (point.y - a.y)) / Math.max(1e-6, b.y - a.y) + a.x;
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}

function clonePanel(panel: NormalizedWellPanelModel): NormalizedWellPanelModel {
  return structuredClone(panel);
}

function slug(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "-");
}

function cloneCorrelationLine(line: CorrelationMarkerLink): CorrelationMarkerLink {
  return {
    ...line,
    points: line.points.map((point) => ({ ...point }))
  };
}

function cloneInteractionEvent(event: InteractionEvent): InteractionEvent {
  return JSON.parse(JSON.stringify(event)) as InteractionEvent;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}
