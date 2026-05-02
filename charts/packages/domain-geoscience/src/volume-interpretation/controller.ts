import { InteractionManager } from "@ophiolite/charts-core";
import {
  clampSlicePlanePosition,
  cloneVolumeInterpretationModel,
  createDefaultVolumeInterpretationView,
  sceneSpan
} from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationAxis,
  InteractionCapabilities,
  VolumeInterpretationEditRequest,
  InteractionEvent,
  VolumeInterpretationDeleteRequest,
  VolumeInterpretationInterpretationRequest,
  VolumeInterpretationModel,
  VolumeInterpretationProbe,
  VolumeInterpretationSelection,
  VolumeInterpretationSelectionContext,
  VolumeInterpretationTool,
  VolumeInterpretationView
} from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationPickDebugSnapshot,
  VolumeInterpretationPickResult,
  VolumeInterpretationRendererAdapter,
  VolumeInterpretationViewState
} from "@ophiolite/charts-renderer";

const VOLUME_INTERPRETATION_INTERACTION_CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom"],
  modifiers: []
};

export class VolumeInterpretationController {
  private sliceMoveSession: {
    itemId: string;
    itemName?: string;
    axis: VolumeInterpretationAxis;
    volumeId: string;
    originalPosition: number;
    currentPosition: number;
    anchorX: number;
    anchorY: number;
    anchorZ: number;
  } | null = null;
  private container: HTMLElement | null = null;
  private readonly renderer: VolumeInterpretationRendererAdapter;
  readonly interactions = new InteractionManager(VOLUME_INTERPRETATION_INTERACTION_CAPABILITIES, "cursor");
  private readonly listeners = new Set<(state: VolumeInterpretationViewState) => void>();
  private readonly interactionEventListeners = new Set<(event: InteractionEvent) => void>();
  private readonly editRequestListeners = new Set<(request: VolumeInterpretationEditRequest) => void>();
  private readonly deleteRequestListeners = new Set<(request: VolumeInterpretationDeleteRequest) => void>();
  private readonly interpretationRequestListeners = new Set<(request: VolumeInterpretationInterpretationRequest) => void>();
  private state: VolumeInterpretationViewState = {
    model: null,
    view: null,
    tool: "pointer",
    probe: null,
    selection: null,
    selectionContext: null,
    interactions: this.interactions.getState()
  };

  constructor(renderer: VolumeInterpretationRendererAdapter) {
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

  setModel(model: VolumeInterpretationModel | null): void {
    const nextModel = model ? cloneVolumeInterpretationModel(model) : null;
    const previousModel = this.state.model;
    const previousView = this.state.view;
    const previousSelection = this.state.selection;
    this.state.model = nextModel;
    this.state.view =
      nextModel && previousModel && previousView && sameBounds(previousModel.sceneBounds, nextModel.sceneBounds)
        ? { ...previousView }
        : nextModel
          ? createDefaultVolumeInterpretationView(nextModel.sceneBounds)
          : null;
    this.state.probe = null;
    this.state.selection =
      nextModel && previousSelection && selectionExists(nextModel, previousSelection)
        ? refreshSelection(nextModel, previousSelection)
        : null;
    this.state.selectionContext = selectionContextFromSelection(this.state.selection);
    this.sliceMoveSession =
      nextModel && this.sliceMoveSession && modelHasSlicePlane(nextModel, this.sliceMoveSession.itemId)
        ? this.sliceMoveSession
        : null;
    this.interactions.setHoverTarget(null);
    this.render();
  }

  setTool(tool: VolumeInterpretationTool): void {
    this.state.tool = tool;
    this.interactions.setPrimaryMode(tool === "pan" ? "panZoom" : "cursor");
    this.render();
  }

  fitToData(): void {
    if (!this.state.model) {
      return;
    }
    this.state.view = createDefaultVolumeInterpretationView(this.state.model.sceneBounds);
    this.render();
  }

  resetView(): void {
    this.fitToData();
  }

  setTopView(): void {
    this.applyViewPreset(-90, 84);
  }

  setSideView(): void {
    this.applyViewPreset(0, 10);
  }

  centerSelection(): void {
    if (!this.state.model || !this.state.view || !this.state.selection) {
      return;
    }
    const nextFocus = selectionFocus(this.state.model, this.state.selection);
    if (!nextFocus) {
      return;
    }
    this.state.view = {
      ...this.state.view,
      focusX: nextFocus.x,
      focusY: nextFocus.y,
      focusZ: nextFocus.z
    };
    this.render();
  }

  zoom(factor: number): void {
    if (!this.state.view || factor <= 0) {
      return;
    }
    this.state.view = {
      ...this.state.view,
      zoom: Math.min(4, Math.max(0.35, this.state.view.zoom * factor))
    };
    this.render();
  }

  orbit(deltaYawDeg: number, deltaPitchDeg: number): void {
    if (!this.state.view) {
      return;
    }
    this.state.view = {
      ...this.state.view,
      yawDeg: this.state.view.yawDeg + deltaYawDeg,
      pitchDeg: clamp(this.state.view.pitchDeg + deltaPitchDeg, 6, 84)
    };
    this.render();
  }

  pan(deltaX: number, deltaY: number): void {
    if (!this.state.view || !this.state.model) {
      return;
    }
    const span = sceneSpan(this.state.model.sceneBounds) / Math.max(0.35, this.state.view.zoom);
    const basis = cameraPanBasis(this.state.view);
    const scale = span * 0.0009;
    this.state.view = {
      ...this.state.view,
      focusX: this.state.view.focusX + basis.right.x * (-deltaX * scale) + basis.up.x * (deltaY * scale),
      focusY: this.state.view.focusY + basis.right.y * (-deltaX * scale) + basis.up.y * (deltaY * scale),
      focusZ: this.state.view.focusZ + basis.right.z * (-deltaX * scale) + basis.up.z * (deltaY * scale)
    };
    this.render();
  }

  moveActiveSlice(deltaWorld: number): void {
    if (!this.state.model || this.state.selection?.kind !== "slice-plane") {
      return;
    }
    this.state.model = {
      ...this.state.model,
      slicePlanes: this.state.model.slicePlanes.map((plane) =>
        plane.id === this.state.selection!.itemId
          ? clampSlicePlanePosition(
              {
                ...plane,
                position: plane.position + deltaWorld
              },
              this.state.model!
            )
          : plane
      )
    };
    this.render();
  }

  beginSelectedSliceMove(x: number, y: number): boolean {
    if (!this.state.model || this.state.selection?.kind !== "slice-plane") {
      this.sliceMoveSession = null;
      return false;
    }
    const plane = this.state.model.slicePlanes.find((candidate) => candidate.id === this.state.selection!.itemId);
    if (!plane) {
      this.sliceMoveSession = null;
      return false;
    }
    const focus =
      selectionFocus(this.state.model, this.state.selection) ??
      slicePlaneAnchorPoint(this.state.model.sceneBounds, plane.axis, plane.position, {
        x: (this.state.model.sceneBounds.minX + this.state.model.sceneBounds.maxX) / 2,
        y: (this.state.model.sceneBounds.minY + this.state.model.sceneBounds.maxY) / 2,
        z: (this.state.model.sceneBounds.minZ + this.state.model.sceneBounds.maxZ) / 2
      });
    const pick = this.renderer.pick(x, y);
    const anchor =
      pick?.kind === "slice-plane" && pick.itemId === plane.id
        ? { x: pick.worldX, y: pick.worldY, z: pick.worldZ }
        : focus;
    this.sliceMoveSession = {
      itemId: plane.id,
      itemName: plane.name,
      axis: plane.axis,
      volumeId: plane.volumeId,
      originalPosition: plane.position,
      currentPosition: plane.position,
      anchorX: anchor.x,
      anchorY: anchor.y,
      anchorZ: anchor.z
    };
    return true;
  }

  previewSelectedSliceMove(deltaWorld: number): void {
    if (!this.state.model || !this.sliceMoveSession) {
      return;
    }
    const plane = this.state.model.slicePlanes.find((candidate) => candidate.id === this.sliceMoveSession!.itemId);
    if (!plane) {
      return;
    }
    const clamped = clampSlicePlanePosition(
      {
        ...plane,
        position: this.sliceMoveSession.currentPosition + deltaWorld
      },
      this.state.model
    );
    this.sliceMoveSession.currentPosition = clamped.position;
    this.emitEditRequest({
      kind: "move-slice-plane",
      phase: "preview",
      itemId: plane.id,
      itemName: plane.name,
      axis: plane.axis,
      volumeId: plane.volumeId,
      originalPosition: this.sliceMoveSession.originalPosition,
      position: clamped.position,
      deltaWorld: clamped.position - this.sliceMoveSession.originalPosition
    });
  }

  previewSelectedSliceMoveFromScreenDelta(deltaX: number, deltaY: number): void {
    if (!this.state.model || !this.sliceMoveSession) {
      return;
    }
    const volume = this.state.model.volumes.find((candidate) => candidate.id === this.sliceMoveSession!.volumeId);
    if (!volume) {
      return;
    }
    const deltaWorld = projectScreenDeltaToAxisWorldDelta(
      this.renderer,
      volume.bounds,
      this.sliceMoveSession.axis,
      this.sliceMoveSession.currentPosition,
      {
        x: this.sliceMoveSession.anchorX,
        y: this.sliceMoveSession.anchorY,
        z: this.sliceMoveSession.anchorZ
      },
      deltaX,
      deltaY
    );
    if (!deltaWorld || !Number.isFinite(deltaWorld)) {
      return;
    }
    this.previewSelectedSliceMove(deltaWorld);
  }

  commitSelectedSliceMove(): void {
    if (!this.sliceMoveSession) {
      return;
    }
    const session = this.sliceMoveSession;
    this.sliceMoveSession = null;
    if (session.currentPosition === session.originalPosition) {
      return;
    }
    this.emitEditRequest({
      kind: "move-slice-plane",
      phase: "commit",
      itemId: session.itemId,
      itemName: session.itemName,
      axis: session.axis,
      volumeId: session.volumeId,
      originalPosition: session.originalPosition,
      position: session.currentPosition,
      deltaWorld: session.currentPosition - session.originalPosition
    });
  }

  cancelSelectedSliceMove(): void {
    this.sliceMoveSession = null;
  }

  deleteSelection(): boolean {
    const request = this.state.selection ? deleteRequestFromSelection(this.state.selection) : null;
    if (!request) {
      return false;
    }
    for (const listener of this.deleteRequestListeners) {
      listener(structuredClone(request));
    }
    this.emitEditRequest(request);
    return true;
  }

  debugPick(x: number, y: number): VolumeInterpretationPickDebugSnapshot {
    return this.renderer.debugPick(x, y);
  }

  updatePointer(x: number, y: number): void {
    const pick = this.renderer.pick(x, y);
    this.state.probe = pick ? probeFromPick(pick) : null;
    this.interactions.setHoverTarget(
      pick ? interactionTargetFromPick(this.state.model?.id, pick) : { kind: "empty-scene", chartId: this.state.model?.id }
    );
    this.render();
  }

  clearPointer(): void {
    this.state.probe = null;
    this.interactions.setHoverTarget(null);
    this.render();
  }

  handlePrimaryAction(x: number, y: number): void {
    const pick = this.renderer.pick(x, y);
    if (!pick) {
      this.state.selection = null;
      this.state.selectionContext = null;
      this.sliceMoveSession = null;
      this.render();
      return;
    }

    if (this.state.tool === "interpret-seed") {
      const request: VolumeInterpretationInterpretationRequest = {
        kind: "seed-horizon",
        targetHorizonId:
          this.state.selection?.kind === "horizon-surface"
            ? this.state.selection.itemId
            : this.state.model?.horizons[0]?.id,
        sourceVolumeId: this.state.model?.volumes[0]?.id,
        slicePlaneId:
          pick.kind === "slice-plane" || pick.kind === "slice-sample"
            ? pick.itemId
            : this.state.model?.slicePlanes[0]?.id,
        worldX: pick.worldX,
        worldY: pick.worldY,
        worldZ: pick.worldZ
      };
      for (const listener of this.interpretationRequestListeners) {
        listener(structuredClone(request));
      }
      return;
    }

    this.state.selection = selectionFromPick(pick);
    this.state.selectionContext = selectionContextFromSelection(this.state.selection);
    this.sliceMoveSession = null;
    this.render();
  }

  handleSecondaryAction(x: number, y: number): boolean {
    const pick = this.renderer.pick(x, y);
    const request = pick ? deleteRequestFromPick(pick) : null;
    if (!request) {
      return false;
    }

    for (const listener of this.deleteRequestListeners) {
      listener(structuredClone(request));
    }
    this.emitEditRequest(request);
    return true;
  }

  onStateChange(listener: (state: VolumeInterpretationViewState) => void): () => void {
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

  onEditRequest(listener: (request: VolumeInterpretationEditRequest) => void): () => void {
    this.editRequestListeners.add(listener);
    return () => {
      this.editRequestListeners.delete(listener);
    };
  }

  onDeleteRequest(listener: (request: VolumeInterpretationDeleteRequest) => void): () => void {
    this.deleteRequestListeners.add(listener);
    return () => {
      this.deleteRequestListeners.delete(listener);
    };
  }

  onInterpretationRequest(
    listener: (request: VolumeInterpretationInterpretationRequest) => void
  ): () => void {
    this.interpretationRequestListeners.add(listener);
    return () => {
      this.interpretationRequestListeners.delete(listener);
    };
  }

  getState(): VolumeInterpretationViewState {
    return {
      model: this.state.model ? cloneVolumeInterpretationModel(this.state.model) : null,
      view: this.state.view ? { ...this.state.view } : null,
      tool: this.state.tool,
      probe: this.state.probe ? { ...this.state.probe, target: { ...this.state.probe.target } } : null,
      selection: this.state.selection ? { ...this.state.selection } : null,
      selectionContext: this.state.selectionContext
        ? {
            selection: { ...this.state.selectionContext.selection },
            allowedGestures: [...this.state.selectionContext.allowedGestures]
          }
        : null,
      interactions: this.interactions.getState()
    };
  }

  dispose(): void {
    this.renderer.dispose();
    this.container = null;
  }

  refresh(): void {
    this.render();
  }

  private applyViewPreset(yawDeg: number, pitchDeg: number): void {
    if (!this.state.view) {
      return;
    }
    this.state.view = {
      ...this.state.view,
      yawDeg,
      pitchDeg: clamp(pitchDeg, 6, 84)
    };
    this.render();
  }

  private render(): void {
    if (!this.container) {
      return;
    }
    this.renderer.render({ state: this.state });
    const state = this.getState();
    for (const listener of this.listeners) {
      listener(state);
    }
  }

  private emitEditRequest(request: VolumeInterpretationEditRequest): void {
    for (const listener of this.editRequestListeners) {
      listener(structuredClone(request));
    }
  }
}

function probeFromPick(pick: VolumeInterpretationPickResult): VolumeInterpretationProbe {
  return {
    target: {
      kind: pick.kind,
      itemId: pick.itemId,
      itemName: pick.itemName
    },
    worldX: pick.worldX,
    worldY: pick.worldY,
    worldZ: pick.worldZ,
    screenX: pick.screenX,
    screenY: pick.screenY
  };
}

function selectionFromPick(pick: VolumeInterpretationPickResult): VolumeInterpretationSelection {
  return {
    kind:
      pick.kind === "slice-plane" || pick.kind === "slice-sample"
        ? "slice-plane"
        : pick.kind === "horizon-surface" || pick.kind === "horizon-contour"
          ? "horizon-surface"
          : pick.kind === "well-trajectory"
            ? "well-trajectory"
            : pick.kind === "well-marker"
              ? "well-marker"
              : "annotation",
    itemId: pick.itemId,
    itemName: pick.itemName
  };
}

function deleteRequestFromPick(pick: VolumeInterpretationPickResult): VolumeInterpretationDeleteRequest | null {
  if (pick.kind === "slice-plane" || pick.kind === "slice-sample") {
    return {
      kind: "delete-slice-plane",
      itemId: pick.itemId,
      itemName: pick.itemName
    };
  }
  if (pick.kind === "horizon-surface" || pick.kind === "horizon-contour") {
    return {
      kind: "delete-horizon-surface",
      itemId: pick.itemId,
      itemName: pick.itemName
    };
  }
  return null;
}

function deleteRequestFromSelection(selection: VolumeInterpretationSelection): VolumeInterpretationDeleteRequest | null {
  if (selection.kind === "slice-plane") {
    return {
      kind: "delete-slice-plane",
      itemId: selection.itemId,
      itemName: selection.itemName
    };
  }
  if (selection.kind === "horizon-surface") {
    return {
      kind: "delete-horizon-surface",
      itemId: selection.itemId,
      itemName: selection.itemName
    };
  }
  return null;
}

function selectionContextFromSelection(
  selection: VolumeInterpretationSelection | null
): VolumeInterpretationSelectionContext | null {
  if (!selection) {
    return null;
  }
  return {
    selection: { ...selection },
    allowedGestures:
      selection.kind === "slice-plane"
        ? ["shiftDragMove", "delete", "centerSelection"]
        : selection.kind === "horizon-surface"
          ? ["delete", "centerSelection"]
          : ["centerSelection"]
  };
}

function modelHasSlicePlane(model: VolumeInterpretationModel, itemId: string): boolean {
  return model.slicePlanes.some((candidate) => candidate.id === itemId);
}

function projectScreenDeltaToAxisWorldDelta(
  renderer: VolumeInterpretationRendererAdapter,
  bounds: VolumeInterpretationModel["sceneBounds"],
  axis: VolumeInterpretationAxis,
  position: number,
  anchor: { x: number; y: number; z: number },
  deltaX: number,
  deltaY: number
): number | null {
  const [min, max] = axisBounds(bounds, axis);
  const axisSpan = Math.max(1, max - min);
  const stepMagnitude = Math.max(axisSpan * 0.16, 18);
  const lowerPosition = clamp(position - stepMagnitude, min, max);
  const upperPosition = clamp(position + stepMagnitude, min, max);
  if (lowerPosition === upperPosition) {
    return null;
  }

  const lowerPoint = slicePlaneAnchorPoint(bounds, axis, lowerPosition, anchor);
  const upperPoint = slicePlaneAnchorPoint(bounds, axis, upperPosition, anchor);
  const lowerScreen = renderer.projectWorldToScreen(lowerPoint.x, lowerPoint.y, lowerPoint.z);
  const upperScreen = renderer.projectWorldToScreen(upperPoint.x, upperPoint.y, upperPoint.z);
  if (!lowerScreen || !upperScreen) {
    return null;
  }

  const screenVecX = upperScreen.x - lowerScreen.x;
  const screenVecY = upperScreen.y - lowerScreen.y;
  const screenLenSq = screenVecX * screenVecX + screenVecY * screenVecY;
  if (screenLenSq < 36) {
    return null;
  }

  return ((deltaX * screenVecX + deltaY * screenVecY) / screenLenSq) * (upperPosition - lowerPosition);
}

function axisBounds(
  bounds: VolumeInterpretationModel["sceneBounds"],
  axis: VolumeInterpretationAxis
): [number, number] {
  return axis === "inline"
    ? [bounds.minX, bounds.maxX]
    : axis === "xline"
      ? [bounds.minY, bounds.maxY]
      : [bounds.minZ, bounds.maxZ];
}

function axisCenterPoint(
  bounds: VolumeInterpretationModel["sceneBounds"],
  axis: VolumeInterpretationAxis,
  position: number
): { x: number; y: number; z: number } {
  const centerX = (bounds.minX + bounds.maxX) / 2;
  const centerY = (bounds.minY + bounds.maxY) / 2;
  const centerZ = (bounds.minZ + bounds.maxZ) / 2;
  return axis === "inline"
    ? { x: position, y: centerY, z: centerZ }
    : axis === "xline"
      ? { x: centerX, y: position, z: centerZ }
      : { x: centerX, y: centerY, z: position };
}

function slicePlaneAnchorPoint(
  bounds: VolumeInterpretationModel["sceneBounds"],
  axis: VolumeInterpretationAxis,
  position: number,
  anchor: { x: number; y: number; z: number }
): { x: number; y: number; z: number } {
  return axis === "inline"
    ? {
        x: position,
        y: clamp(anchor.y, bounds.minY, bounds.maxY),
        z: clamp(anchor.z, bounds.minZ, bounds.maxZ)
      }
    : axis === "xline"
      ? {
          x: clamp(anchor.x, bounds.minX, bounds.maxX),
          y: position,
          z: clamp(anchor.z, bounds.minZ, bounds.maxZ)
        }
      : {
          x: clamp(anchor.x, bounds.minX, bounds.maxX),
          y: clamp(anchor.y, bounds.minY, bounds.maxY),
          z: position
        };
}

function selectionFocus(
  model: VolumeInterpretationModel,
  selection: VolumeInterpretationSelection
): { x: number; y: number; z: number } | null {
  if (selection.kind === "slice-plane") {
    const plane = model.slicePlanes.find((candidate) => candidate.id === selection.itemId);
    const volume = plane ? model.volumes.find((candidate) => candidate.id === plane.volumeId) : null;
    if (!plane || !volume) {
      return null;
    }
    return plane.axis === "inline"
      ? { x: plane.position, y: (volume.bounds.minY + volume.bounds.maxY) / 2, z: (volume.bounds.minZ + volume.bounds.maxZ) / 2 }
      : plane.axis === "xline"
        ? { x: (volume.bounds.minX + volume.bounds.maxX) / 2, y: plane.position, z: (volume.bounds.minZ + volume.bounds.maxZ) / 2 }
        : { x: (volume.bounds.minX + volume.bounds.maxX) / 2, y: (volume.bounds.minY + volume.bounds.maxY) / 2, z: plane.position };
  }
  if (selection.kind === "horizon-surface") {
    const horizon = model.horizons.find((candidate) => candidate.id === selection.itemId);
    if (!horizon || horizon.points.length < 3) {
      return null;
    }
    const centerIndex = Math.floor(horizon.points.length / 6) * 3;
    return {
      x: horizon.points[centerIndex]!,
      y: horizon.points[centerIndex + 1]!,
      z: horizon.points[centerIndex + 2]!
    };
  }
  if (selection.kind === "well-trajectory") {
    const well = model.wells.find((candidate) => candidate.id === selection.itemId);
    if (!well || well.points.length < 3) {
      return null;
    }
    return {
      x: well.points[0]!,
      y: well.points[1]!,
      z: well.points[2]!
    };
  }
  if (selection.kind === "well-marker") {
    const marker = model.markers.find((candidate) => candidate.id === selection.itemId);
    if (!marker) {
      return null;
    }
    return { x: marker.x, y: marker.y, z: marker.z };
  }
  const annotation = model.annotations?.find((candidate) => candidate.id === selection.itemId);
  return annotation ? { x: annotation.x, y: annotation.y, z: annotation.z } : null;
}

function interactionTargetFromPick(chartId: string | undefined, pick: VolumeInterpretationPickResult) {
  return {
    kind:
      pick.kind === "slice-plane"
        ? "volume-slice-plane"
        : pick.kind === "slice-sample"
          ? "volume-slice-sample"
          : pick.kind === "horizon-surface"
            ? "volume-horizon-surface"
            : pick.kind === "horizon-contour"
              ? "volume-horizon-contour"
              : pick.kind === "well-trajectory"
                ? "volume-well-trajectory"
                : pick.kind === "well-marker"
                  ? "volume-well-marker"
                  : "volume-annotation",
    chartId,
    entityId: pick.itemId
  } as const;
}

function sameBounds(left: VolumeInterpretationModel["sceneBounds"], right: VolumeInterpretationModel["sceneBounds"]): boolean {
  return (
    left.minX === right.minX &&
    left.minY === right.minY &&
    left.minZ === right.minZ &&
    left.maxX === right.maxX &&
    left.maxY === right.maxY &&
    left.maxZ === right.maxZ
  );
}

function selectionExists(model: VolumeInterpretationModel, selection: VolumeInterpretationSelection): boolean {
  switch (selection.kind) {
    case "slice-plane":
      return model.slicePlanes.some((candidate) => candidate.id === selection.itemId);
    case "horizon-surface":
      return model.horizons.some((candidate) => candidate.id === selection.itemId);
    case "well-trajectory":
      return model.wells.some((candidate) => candidate.id === selection.itemId);
    case "well-marker":
      return model.markers.some((candidate) => candidate.id === selection.itemId);
    default:
      return (model.annotations ?? []).some((candidate) => candidate.id === selection.itemId);
  }
}

function refreshSelection(
  model: VolumeInterpretationModel,
  selection: VolumeInterpretationSelection
): VolumeInterpretationSelection {
  const itemName = selectionName(model, selection);
  return itemName === selection.itemName ? { ...selection } : { ...selection, itemName };
}

function selectionName(
  model: VolumeInterpretationModel,
  selection: VolumeInterpretationSelection
): string | undefined {
  switch (selection.kind) {
    case "slice-plane":
      return model.slicePlanes.find((candidate) => candidate.id === selection.itemId)?.name;
    case "horizon-surface":
      return model.horizons.find((candidate) => candidate.id === selection.itemId)?.name;
    case "well-trajectory":
      return model.wells.find((candidate) => candidate.id === selection.itemId)?.name;
    case "well-marker":
      return model.markers.find((candidate) => candidate.id === selection.itemId)?.name;
    default:
      return model.annotations?.find((candidate) => candidate.id === selection.itemId)?.text;
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function cameraPanBasis(view: VolumeInterpretationView): {
  right: { x: number; y: number; z: number };
  up: { x: number; y: number; z: number };
} {
  const yaw = (view.yawDeg * Math.PI) / 180;
  const pitch = (view.pitchDeg * Math.PI) / 180;
  const sinYaw = Math.sin(yaw);
  const cosYaw = Math.cos(yaw);
  const sinPitch = Math.sin(pitch);
  const cosPitch = Math.cos(pitch);

  return {
    right: {
      x: -sinYaw,
      y: cosYaw,
      z: 0
    },
    up: {
      x: -cosYaw * sinPitch,
      y: -sinYaw * sinPitch,
      z: cosPitch
    }
  };
}

function cloneInteractionEvent(event: InteractionEvent): InteractionEvent {
  return JSON.parse(JSON.stringify(event)) as InteractionEvent;
}
