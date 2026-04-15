import { InteractionManager } from "@ophiolite/charts-core";
import {
  clampSlicePlanePosition,
  createDefaultVolumeInterpretationView,
  sceneSpan
} from "@ophiolite/charts-data-models";
import type {
  InteractionCapabilities,
  InteractionEvent,
  VolumeInterpretationInterpretationRequest,
  VolumeInterpretationModel,
  VolumeInterpretationProbe,
  VolumeInterpretationSelection,
  VolumeInterpretationTool,
  VolumeInterpretationView
} from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationPickResult,
  VolumeInterpretationRendererAdapter,
  VolumeInterpretationViewState
} from "@ophiolite/charts-renderer";

const VOLUME_INTERPRETATION_INTERACTION_CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom"],
  modifiers: []
};

export class VolumeInterpretationController {
  private container: HTMLElement | null = null;
  private readonly renderer: VolumeInterpretationRendererAdapter;
  readonly interactions = new InteractionManager(VOLUME_INTERPRETATION_INTERACTION_CAPABILITIES, "cursor");
  private readonly listeners = new Set<(state: VolumeInterpretationViewState) => void>();
  private readonly interactionEventListeners = new Set<(event: InteractionEvent) => void>();
  private readonly interpretationRequestListeners = new Set<(request: VolumeInterpretationInterpretationRequest) => void>();
  private state: VolumeInterpretationViewState = {
    model: null,
    view: null,
    tool: "pointer",
    probe: null,
    selection: null,
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
    const nextModel = model ? structuredClone(model) : null;
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
        ? { ...previousSelection }
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
      pitchDeg: clamp(this.state.view.pitchDeg + deltaPitchDeg, 6, 72)
    };
    this.render();
  }

  pan(deltaX: number, deltaY: number): void {
    if (!this.state.view || !this.state.model) {
      return;
    }
    const span = sceneSpan(this.state.model.sceneBounds) / Math.max(0.35, this.state.view.zoom);
    this.state.view = {
      ...this.state.view,
      focusX: this.state.view.focusX - deltaX * span * 0.0009,
      focusY: this.state.view.focusY + deltaY * span * 0.0009
    };
    this.render();
  }

  moveActiveSlice(deltaWorld: number): void {
    if (!this.state.model) {
      return;
    }
    const activeSliceId =
      this.state.selection?.kind === "slice-plane"
        ? this.state.selection.itemId
        : this.state.probe?.target.kind === "slice-plane"
          ? this.state.probe.target.itemId
          : null;
    if (!activeSliceId) {
      return;
    }
    this.state.model = {
      ...this.state.model,
      slicePlanes: this.state.model.slicePlanes.map((plane) =>
        plane.id === activeSliceId
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
    this.render();
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
      model: this.state.model ? structuredClone(this.state.model) : null,
      view: this.state.view ? { ...this.state.view } : null,
      tool: this.state.tool,
      probe: this.state.probe ? { ...this.state.probe, target: { ...this.state.probe.target } } : null,
      selection: this.state.selection ? { ...this.state.selection } : null,
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

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function cloneInteractionEvent(event: InteractionEvent): InteractionEvent {
  return JSON.parse(JSON.stringify(event)) as InteractionEvent;
}
