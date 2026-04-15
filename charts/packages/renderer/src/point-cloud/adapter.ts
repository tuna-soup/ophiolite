import type {
  InteractionState,
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotProbe,
  RockPhysicsCrossplotViewport
} from "@ophiolite/charts-data-models";

export interface RockPhysicsCrossplotViewState {
  model: RockPhysicsCrossplotModel | null;
  viewport: RockPhysicsCrossplotViewport | null;
  probe: RockPhysicsCrossplotProbe | null;
  interactions: InteractionState;
}

export interface RockPhysicsCrossplotRenderFrame {
  state: RockPhysicsCrossplotViewState;
}

export interface RockPhysicsCrossplotRendererAdapter {
  mount(container: HTMLElement): void;
  render(frame: RockPhysicsCrossplotRenderFrame): void;
  dispose(): void;
}
