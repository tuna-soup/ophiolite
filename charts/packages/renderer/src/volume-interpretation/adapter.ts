import type {
  VolumeInterpretationSelectionContext,
  InteractionState,
  VolumeInterpretationModel,
  VolumeInterpretationProbe,
  VolumeInterpretationSelection,
  VolumeInterpretationTool,
  VolumeInterpretationView,
} from "@ophiolite/charts-data-models";

export interface VolumeInterpretationViewState {
  model: VolumeInterpretationModel | null;
  view: VolumeInterpretationView | null;
  tool: VolumeInterpretationTool;
  probe: VolumeInterpretationProbe | null;
  selection: VolumeInterpretationSelection | null;
  selectionContext: VolumeInterpretationSelectionContext | null;
  interactions: InteractionState;
}

export interface VolumeInterpretationRenderFrame {
  state: VolumeInterpretationViewState;
}

export interface VolumeInterpretationPickResult {
  kind:
    | "slice-plane"
    | "slice-sample"
    | "horizon-surface"
    | "horizon-contour"
    | "well-trajectory"
    | "well-marker"
    | "annotation";
  itemId: string;
  itemName?: string;
  worldX: number;
  worldY: number;
  worldZ: number;
  screenX: number;
  screenY: number;
}

export interface VolumeInterpretationPickDebugCandidate {
  targetType: "polygon" | "polyline" | "point";
  kind: VolumeInterpretationPickResult["kind"];
  itemId: string;
  itemName?: string;
  hit: boolean;
  score: number | null;
  depth: number;
  screenX: number;
  screenY: number;
  worldX: number;
  worldY: number;
  worldZ: number;
}

export interface VolumeInterpretationPickDebugSnapshot {
  pointerX: number;
  pointerY: number;
  renderPointerX: number;
  renderPointerY: number;
  renderScaleX: number;
  renderScaleY: number;
  actualWinner: VolumeInterpretationPickResult | null;
  actualPickedCount: number;
  actualMatchedBy: "prop" | "mapper" | null;
  syntheticWinner: VolumeInterpretationPickResult | null;
  winner: VolumeInterpretationPickResult | null;
  candidates: VolumeInterpretationPickDebugCandidate[];
}

export interface VolumeInterpretationRendererAdapter {
  mount(container: HTMLElement): void;
  render(frame: VolumeInterpretationRenderFrame): void;
  pick(screenX: number, screenY: number): VolumeInterpretationPickResult | null;
  projectWorldToScreen(
    worldX: number,
    worldY: number,
    worldZ: number,
  ): { x: number; y: number } | null;
  debugPick(
    screenX: number,
    screenY: number,
  ): VolumeInterpretationPickDebugSnapshot;
  dispose(): void;
}
