import { describe, expect, it } from "bun:test";
import { createMockSurveyMap } from "@ophiolite/charts-data-models";
import type {
  RendererTelemetryListener,
  SurveyMapRenderFrame,
  SurveyMapRendererAdapter
} from "@ophiolite/charts-renderer";
import { SurveyMapController } from "../src/survey-map/controller";

class SurveyMapTestRenderer implements SurveyMapRendererAdapter {
  setTelemetryListener(_listener: RendererTelemetryListener | null): void {}
  mount(_container: HTMLElement): void {}
  render(_frame: SurveyMapRenderFrame): void {}
  dispose(): void {}
}

describe("SurveyMapController interactions", () => {
  it("attaches the survey-map interaction style and resolves primary actions from it", () => {
    const controller = new SurveyMapController(new SurveyMapTestRenderer());
    controller.setMap(createMockSurveyMap());

    expect(controller.interactions.getStyle()?.id).toBe("survey-map-navigation");
    expect(controller.handlePrimaryPointerDown()).toBe("select");

    controller.setPrimaryMode("panZoom");
    expect(controller.handlePrimaryPointerDown()).toBe("pan");
  });

  it("applies keyboard and wheel navigation through controller-owned handlers", () => {
    const controller = new SurveyMapController(new SurveyMapTestRenderer());
    controller.setMap(createMockSurveyMap());

    const initialViewport = controller.getState().viewport!;
    expect(controller.handleKeyboardNavigation("ArrowRight")).toBe(true);
    const pannedViewport = controller.getState().viewport!;
    expect(pannedViewport.xMin).toBeGreaterThan(initialViewport.xMin);

    const pannedSpan = pannedViewport.xMax - pannedViewport.xMin;
    expect(controller.handleWheelAt(640, 360, 1280, 720, -120)).toBe(true);
    const zoomedViewport = controller.getState().viewport!;
    expect(zoomedViewport.xMax - zoomedViewport.xMin).toBeLessThan(pannedSpan);
  });
});
