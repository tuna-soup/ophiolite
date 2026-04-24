import type { InteractionState, SurveyMapModel, SurveyMapProbe, SurveyMapViewport } from "@ophiolite/charts-data-models";
import type { RendererTelemetrySource } from "../telemetry";

export interface SurveyMapViewState {
  map: SurveyMapModel | null;
  viewport: SurveyMapViewport | null;
  probe: SurveyMapProbe | null;
  selectedWellId: string | null;
  interactions: InteractionState;
}

export interface SurveyMapRenderFrame {
  state: SurveyMapViewState;
}

export interface SurveyMapRendererAdapter extends RendererTelemetrySource {
  mount(container: HTMLElement): void;
  render(frame: SurveyMapRenderFrame): void;
  dispose(): void;
}
