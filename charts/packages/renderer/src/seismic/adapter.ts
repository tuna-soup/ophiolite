import type { RenderFrame } from "@ophiolite/charts-data-models";

export interface RendererAdapter {
  mount(container: HTMLElement): void;
  render(frame: RenderFrame): void;
  dispose(): void;
}
