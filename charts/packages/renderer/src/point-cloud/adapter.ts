import type {
  CartesianAxisOverrides,
  InteractionState,
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotProbe,
  RockPhysicsCrossplotViewport
} from "@ophiolite/charts-data-models";

export interface RockPhysicsCrossplotViewState {
  model: RockPhysicsCrossplotModel | null;
  viewport: RockPhysicsCrossplotViewport | null;
  probe: RockPhysicsCrossplotProbe | null;
  axisOverrides: CartesianAxisOverrides;
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
