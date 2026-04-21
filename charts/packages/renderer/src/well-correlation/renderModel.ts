import type { WellCorrelationRenderFrame } from "./adapter";

export interface BaseRenderState {
  panel: WellCorrelationRenderFrame["state"]["panel"];
  viewport: WellCorrelationRenderFrame["state"]["viewport"];
  width: number;
  height: number;
  pixelRatio: number;
  scrollLeft: number;
  viewportWidth: number;
}

export interface OverlayRenderState {
  panel: WellCorrelationRenderFrame["state"]["panel"];
  viewport: WellCorrelationRenderFrame["state"]["viewport"];
  probe: WellCorrelationRenderFrame["state"]["probe"];
  interactions: WellCorrelationRenderFrame["state"]["interactions"];
  correlationLines: WellCorrelationRenderFrame["state"]["correlationLines"];
  previewCorrelationLines: WellCorrelationRenderFrame["state"]["previewCorrelationLines"];
  width: number;
  height: number;
  pixelRatio: number;
  scrollLeft: number;
  viewportWidth: number;
}

export interface RenderInvalidation {
  baseChanged: boolean;
  overlayNeedsDraw: boolean;
}

export function createBaseRenderState(
  frame: WellCorrelationRenderFrame,
  width: number,
  height: number,
  pixelRatio: number,
  scrollLeft: number,
  viewportWidth: number
): BaseRenderState {
  return {
    panel: frame.state.panel,
    viewport: frame.state.viewport,
    width,
    height,
    pixelRatio,
    scrollLeft,
    viewportWidth
  };
}

export function createOverlayRenderState(
  frame: WellCorrelationRenderFrame,
  width: number,
  height: number,
  pixelRatio: number,
  scrollLeft: number,
  viewportWidth: number
): OverlayRenderState {
  return {
    panel: frame.state.panel,
    viewport: frame.state.viewport,
    probe: frame.state.probe,
    interactions: frame.state.interactions,
    correlationLines: frame.state.correlationLines,
    previewCorrelationLines: frame.state.previewCorrelationLines,
    width,
    height,
    pixelRatio,
    scrollLeft,
    viewportWidth
  };
}

export function diffRenderStates(
  previousBase: BaseRenderState | null,
  nextBase: BaseRenderState,
  previousOverlay: OverlayRenderState | null,
  nextOverlay: OverlayRenderState
): RenderInvalidation {
  const baseChanged =
    previousBase?.panel !== nextBase.panel ||
    viewportChanged(previousBase?.viewport ?? null, nextBase.viewport) ||
    previousBase?.width !== nextBase.width ||
    previousBase?.height !== nextBase.height ||
    previousBase?.pixelRatio !== nextBase.pixelRatio ||
    previousBase?.scrollLeft !== nextBase.scrollLeft ||
    previousBase?.viewportWidth !== nextBase.viewportWidth;

  const overlayNeedsDraw =
    baseChanged ||
    previousOverlay?.probe !== nextOverlay.probe ||
    previousOverlay?.interactions !== nextOverlay.interactions ||
    previousOverlay?.correlationLines !== nextOverlay.correlationLines ||
    previousOverlay?.previewCorrelationLines !== nextOverlay.previewCorrelationLines ||
    previousOverlay?.scrollLeft !== nextOverlay.scrollLeft ||
    previousOverlay?.viewportWidth !== nextOverlay.viewportWidth ||
    previousOverlay?.width !== nextOverlay.width ||
    previousOverlay?.height !== nextOverlay.height ||
    previousOverlay?.pixelRatio !== nextOverlay.pixelRatio;

  return {
    baseChanged,
    overlayNeedsDraw
  };
}

function viewportChanged(
  previous: WellCorrelationRenderFrame["state"]["viewport"],
  next: WellCorrelationRenderFrame["state"]["viewport"]
): boolean {
  return (
    previous?.depthStart !== next?.depthStart ||
    previous?.depthEnd !== next?.depthEnd
  );
}
