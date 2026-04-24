import { describe, expect, it } from "bun:test";
import { createMockCorrelationPanel } from "@ophiolite/charts-data-models";
import type {
  RendererTelemetryListener,
  WellCorrelationRenderFrame,
  WellCorrelationRendererAdapter
} from "@ophiolite/charts-renderer";
import { WellCorrelationController } from "../src/well-correlation/controller";

class WellCorrelationTestRenderer implements WellCorrelationRendererAdapter {
  setTelemetryListener(_listener: RendererTelemetryListener | null): void {}
  mount(_container: HTMLElement): void {}
  render(_frame: WellCorrelationRenderFrame): void {}
  dispose(): void {}
}

describe("WellCorrelationController interactions", () => {
  it("attaches the well-panel interaction style and resolves pan mode through it", () => {
    const controller = new WellCorrelationController(new WellCorrelationTestRenderer());
    controller.setPanel(createMockCorrelationPanel());

    expect(controller.interactions.getStyle()?.id).toBe("well-panel-navigation");
    controller.setPrimaryMode("panZoom");
    expect(controller.handlePrimaryPointerDown(280, 220, 1200, 720)).toBe("pan");
  });

  it("routes keyboard, wheel, and session cancel through controller-owned handlers", () => {
    const controller = new WellCorrelationController(new WellCorrelationTestRenderer());
    controller.setPanel(createMockCorrelationPanel());

    expect(controller.handleWheelAt(280, -120, 1200, 720, true)).toBe(true);
    const zoomedViewport = controller.getState().viewport!;
    const zoomedSpan = zoomedViewport.depthEnd - zoomedViewport.depthStart;
    expect(zoomedSpan).toBeLessThan(
      controller.getState().panel!.depthDomain.end - controller.getState().panel!.depthDomain.start
    );

    expect(controller.handleKeyboardNavigation("ArrowDown")).toBe(true);
    const pannedViewport = controller.getState().viewport!;
    expect(pannedViewport.depthStart).toBeGreaterThan(zoomedViewport.depthStart);

    controller.setPrimaryMode("lassoSelect");
    expect(controller.handlePrimaryPointerDown(320, 260, 1200, 720)).toBe("session");
    expect(controller.getState().interactions.session?.kind).toBe("lasso");
    expect(controller.handleKeyboardNavigation("Escape")).toBe(true);
    expect(controller.getState().interactions.session).toBeNull();
  });
});
