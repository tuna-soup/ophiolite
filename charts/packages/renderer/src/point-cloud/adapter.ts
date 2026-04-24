import type {
  CartesianAxisOverrides,
  InteractionState,
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotProbe,
  RockPhysicsCrossplotViewport
} from "@ophiolite/charts-data-models";
import type { RendererTelemetrySource } from "../telemetry";

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

export interface RockPhysicsCrossplotRendererAdapter extends RendererTelemetrySource {
  mount(container: HTMLElement): void;
  render(frame: RockPhysicsCrossplotRenderFrame): void;
  dispose(): void;
}
