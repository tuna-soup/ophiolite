import type {
  InteractionState,
  VolumeInterpretationModel,
  VolumeInterpretationProbe,
  VolumeInterpretationSelection,
  VolumeInterpretationTool,
  VolumeInterpretationView
} from "@ophiolite/charts-data-models";

export interface VolumeInterpretationViewState {
  model: VolumeInterpretationModel | null;
  view: VolumeInterpretationView | null;
  tool: VolumeInterpretationTool;
  probe: VolumeInterpretationProbe | null;
  selection: VolumeInterpretationSelection | null;
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

export interface VolumeInterpretationRendererAdapter {
  mount(container: HTMLElement): void;
  render(frame: VolumeInterpretationRenderFrame): void;
  pick(screenX: number, screenY: number): VolumeInterpretationPickResult | null;
  dispose(): void;
}
