import type {
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotViewport
} from "@ophiolite/charts-data-models";

export interface RockPhysicsCrossplotRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RockPhysicsDataPoint {
  x: number;
  y: number;
}

export const ROCK_PHYSICS_CROSSPLOT_MARGIN = {
  top: 56,
  right: 28,
  bottom: 56,
  left: 72
} as const;

export function getRockPhysicsCrossplotPlotRect(
  width: number,
  height: number
): RockPhysicsCrossplotRect {
  return {
    x: ROCK_PHYSICS_CROSSPLOT_MARGIN.left,
    y: ROCK_PHYSICS_CROSSPLOT_MARGIN.top,
    width: Math.max(1, width - ROCK_PHYSICS_CROSSPLOT_MARGIN.left - ROCK_PHYSICS_CROSSPLOT_MARGIN.right),
    height: Math.max(1, height - ROCK_PHYSICS_CROSSPLOT_MARGIN.top - ROCK_PHYSICS_CROSSPLOT_MARGIN.bottom)
  };
}

export function fitRockPhysicsViewport(
  model: RockPhysicsCrossplotModel | null,
  paddingRatio = 0.06
): RockPhysicsCrossplotViewport | null {
  if (!model || model.pointCount <= 0) {
    return null;
  }

  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (let index = 0; index < model.pointCount; index += 1) {
    const x = model.columns.x[index];
    const y = model.columns.y[index];
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      continue;
    }
    minX = Math.min(minX, x);
    maxX = Math.max(maxX, x);
    minY = Math.min(minY, y);
    maxY = Math.max(maxY, y);
  }

  if (!Number.isFinite(minX) || !Number.isFinite(maxX) || !Number.isFinite(minY) || !Number.isFinite(maxY)) {
    return axisViewport(model);
  }

  const spanX = Math.max(1e-6, maxX - minX);
  const spanY = Math.max(1e-6, maxY - minY);
  const padX = spanX * paddingRatio;
  const padY = spanY * paddingRatio;

  return clampRockPhysicsViewport(model, {
    xMin: minX - padX,
    xMax: maxX + padX,
    yMin: minY - padY,
    yMax: maxY + padY
  });
}

export function clampRockPhysicsViewport(
  model: RockPhysicsCrossplotModel | null,
  viewport: RockPhysicsCrossplotViewport | null
): RockPhysicsCrossplotViewport | null {
  if (!model || !viewport) {
    return viewport;
  }

  const bounds = axisViewport(model);
  const fullSpanX = Math.max(1e-6, bounds.xMax - bounds.xMin);
  const fullSpanY = Math.max(1e-6, bounds.yMax - bounds.yMin);
  const requestedSpanX = clamp(viewport.xMax - viewport.xMin, fullSpanX * 0.01, fullSpanX);
  const requestedSpanY = clamp(viewport.yMax - viewport.yMin, fullSpanY * 0.01, fullSpanY);
  const xMin = clamp(viewport.xMin, bounds.xMin, bounds.xMax - requestedSpanX);
  const yMin = clamp(viewport.yMin, bounds.yMin, bounds.yMax - requestedSpanY);

  return {
    xMin,
    xMax: xMin + requestedSpanX,
    yMin,
    yMax: yMin + requestedSpanY
  };
}

export function valueToRockPhysicsScreenX(
  value: number,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: RockPhysicsCrossplotRect
): number {
  const ratio = (value - viewport.xMin) / Math.max(1e-6, viewport.xMax - viewport.xMin);
  return plotRect.x + clamp(ratio, 0, 1) * plotRect.width;
}

export function valueToRockPhysicsScreenY(
  value: number,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: RockPhysicsCrossplotRect
): number {
  const ratio = (value - viewport.yMin) / Math.max(1e-6, viewport.yMax - viewport.yMin);
  return plotRect.y + plotRect.height - clamp(ratio, 0, 1) * plotRect.height;
}

export function rockPhysicsScreenToValue(
  x: number,
  y: number,
  viewport: RockPhysicsCrossplotViewport,
  plotRect: RockPhysicsCrossplotRect
): RockPhysicsDataPoint {
  const xRatio = clamp((x - plotRect.x) / Math.max(1, plotRect.width), 0, 1);
  const yRatio = clamp((plotRect.y + plotRect.height - y) / Math.max(1, plotRect.height), 0, 1);
  return {
    x: viewport.xMin + xRatio * (viewport.xMax - viewport.xMin),
    y: viewport.yMin + yRatio * (viewport.yMax - viewport.yMin)
  };
}

function axisViewport(model: RockPhysicsCrossplotModel): RockPhysicsCrossplotViewport {
  return {
    xMin: model.xAxis.range.min,
    xMax: model.xAxis.range.max,
    yMin: model.yAxis.range.min,
    yMax: model.yAxis.range.max
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
