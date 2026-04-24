import type { RenderFrame } from "@ophiolite/charts-data-models";
import type { RendererTelemetrySource } from "../telemetry";

export interface RendererAdapter extends RendererTelemetrySource {
  mount(container: HTMLElement): void;
  render(frame: RenderFrame): void;
  dispose(): void;
}
