import { resolveCartesianPresentationProfile, resolveCartesianStageLayout } from "./cartesian-presentation";
import type { AvoCartesianViewport, AvoCrossplotModel } from "@ophiolite/charts-data-models";

export interface AvoCrossplotRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const AVO_CROSSPLOT_MARGIN = resolveCartesianPresentationProfile("avo").frame.plotInsets;

export function getAvoCrossplotPlotRect(width: number, height: number): AvoCrossplotRect {
  return resolveCartesianStageLayout(width, height, "avo").plotRect;
}

export function fitAvoCrossplotViewport(model: AvoCrossplotModel | null, paddingRatio = 0.08): AvoCartesianViewport | null {
  if (!model || model.pointCount <= 0) {
    return null;
  }

  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (let index = 0; index < model.pointCount; index += 1) {
    const x = model.columns.intercept[index];
    const y = model.columns.gradient[index];
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

  return clampAvoCrossplotViewport(model, {
    xMin: minX - spanX * paddingRatio,
    xMax: maxX + spanX * paddingRatio,
    yMin: minY - spanY * paddingRatio,
    yMax: maxY + spanY * paddingRatio
  });
}

export function clampAvoCrossplotViewport(
  model: AvoCrossplotModel | null,
  viewport: AvoCartesianViewport | null
): AvoCartesianViewport | null {
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

export function valueToAvoCrossplotScreenX(
  value: number,
  viewport: AvoCartesianViewport,
  plotRect: AvoCrossplotRect
): number {
  const ratio = (value - viewport.xMin) / Math.max(1e-6, viewport.xMax - viewport.xMin);
  return plotRect.x + clamp(ratio, 0, 1) * plotRect.width;
}

export function valueToAvoCrossplotScreenY(
  value: number,
  viewport: AvoCartesianViewport,
  plotRect: AvoCrossplotRect
): number {
  const ratio = (value - viewport.yMin) / Math.max(1e-6, viewport.yMax - viewport.yMin);
  return plotRect.y + plotRect.height - clamp(ratio, 0, 1) * plotRect.height;
}

export function avoCrossplotScreenToValue(
  x: number,
  y: number,
  viewport: AvoCartesianViewport,
  plotRect: AvoCrossplotRect
): { x: number; y: number } {
  const xRatio = clamp((x - plotRect.x) / Math.max(1, plotRect.width), 0, 1);
  const yRatio = clamp((plotRect.y + plotRect.height - y) / Math.max(1, plotRect.height), 0, 1);
  return {
    x: viewport.xMin + xRatio * (viewport.xMax - viewport.xMin),
    y: viewport.yMin + yRatio * (viewport.yMax - viewport.yMin)
  };
}

function axisViewport(model: AvoCrossplotModel): AvoCartesianViewport {
  return {
    xMin: model.xAxis.range?.min ?? -0.3,
    xMax: model.xAxis.range?.max ?? 0.3,
    yMin: model.yAxis.range?.min ?? -0.8,
    yMax: model.yAxis.range?.max ?? 0.8
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
