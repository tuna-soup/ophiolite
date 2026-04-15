import type { SurveyMapModel, SurveyMapPoint, SurveyMapScalarField, SurveyMapViewport } from "@ophiolite/charts-data-models";

export interface SurveyMapRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SurveyMapBounds {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
}

export interface SurveyMapViewMetrics {
  drawRect: SurveyMapRect;
  scale: number;
}

export const SURVEY_MAP_MARGIN = {
  top: 18,
  right: 58,
  bottom: 34,
  left: 48
} as const;

export function getSurveyMapPlotRect(width: number, height: number): SurveyMapRect {
  return {
    x: SURVEY_MAP_MARGIN.left,
    y: SURVEY_MAP_MARGIN.top,
    width: Math.max(1, width - SURVEY_MAP_MARGIN.left - SURVEY_MAP_MARGIN.right),
    height: Math.max(1, height - SURVEY_MAP_MARGIN.top - SURVEY_MAP_MARGIN.bottom)
  };
}

export function computeSurveyMapBounds(map: SurveyMapModel | null): SurveyMapBounds | null {
  if (!map) {
    return null;
  }

  const points: SurveyMapPoint[] = [
    ...map.surveys.flatMap((survey) => survey.outline),
    ...map.wells.map((well) => well.surface),
    ...map.wells.flatMap((well) => well.trajectory ?? [])
  ];
  const scalarBounds = map.scalarField ? scalarFieldBounds(map.scalarField) : null;
  if (scalarBounds) {
    points.push(
      { x: scalarBounds.minX, y: scalarBounds.minY },
      { x: scalarBounds.maxX, y: scalarBounds.maxY }
    );
  }

  if (points.length === 0) {
    return null;
  }

  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const point of points) {
    minX = Math.min(minX, point.x);
    maxX = Math.max(maxX, point.x);
    minY = Math.min(minY, point.y);
    maxY = Math.max(maxY, point.y);
  }

  return { minX, maxX, minY, maxY };
}

export function fitSurveyMapViewport(bounds: SurveyMapBounds | null, paddingRatio = 0.08): SurveyMapViewport | null {
  if (!bounds) {
    return null;
  }

  const spanX = Math.max(1, bounds.maxX - bounds.minX);
  const spanY = Math.max(1, bounds.maxY - bounds.minY);
  const padX = spanX * paddingRatio;
  const padY = spanY * paddingRatio;

  return {
    xMin: bounds.minX - padX,
    xMax: bounds.maxX + padX,
    yMin: bounds.minY - padY,
    yMax: bounds.maxY + padY
  };
}

export function clampSurveyMapViewport(
  bounds: SurveyMapBounds | null,
  viewport: SurveyMapViewport | null
): SurveyMapViewport | null {
  if (!bounds || !viewport) {
    return viewport;
  }

  const fullSpanX = Math.max(1, bounds.maxX - bounds.minX);
  const fullSpanY = Math.max(1, bounds.maxY - bounds.minY);
  const requestedSpanX = clamp(viewport.xMax - viewport.xMin, fullSpanX * 0.02, fullSpanX);
  const requestedSpanY = clamp(viewport.yMax - viewport.yMin, fullSpanY * 0.02, fullSpanY);
  const maxXMin = bounds.maxX - requestedSpanX;
  const maxYMin = bounds.maxY - requestedSpanY;
  const xMin = clamp(viewport.xMin, bounds.minX, maxXMin);
  const yMin = clamp(viewport.yMin, bounds.minY, maxYMin);

  return {
    xMin,
    xMax: xMin + requestedSpanX,
    yMin,
    yMax: yMin + requestedSpanY
  };
}

export function resolveSurveyMapViewMetrics(viewport: SurveyMapViewport, plotRect: SurveyMapRect): SurveyMapViewMetrics {
  const spanX = Math.max(1e-6, viewport.xMax - viewport.xMin);
  const spanY = Math.max(1e-6, viewport.yMax - viewport.yMin);
  const scale = Math.min(plotRect.width / spanX, plotRect.height / spanY);
  const drawWidth = spanX * scale;
  const drawHeight = spanY * scale;

  return {
    drawRect: {
      x: plotRect.x + (plotRect.width - drawWidth) / 2,
      y: plotRect.y + (plotRect.height - drawHeight) / 2,
      width: drawWidth,
      height: drawHeight
    },
    scale
  };
}

export function worldToScreen(point: SurveyMapPoint, viewport: SurveyMapViewport, plotRect: SurveyMapRect): SurveyMapPoint {
  const metrics = resolveSurveyMapViewMetrics(viewport, plotRect);
  return {
    x: metrics.drawRect.x + (point.x - viewport.xMin) * metrics.scale,
    y: metrics.drawRect.y + metrics.drawRect.height - (point.y - viewport.yMin) * metrics.scale
  };
}

export function screenToWorld(x: number, y: number, viewport: SurveyMapViewport, plotRect: SurveyMapRect): SurveyMapPoint {
  const metrics = resolveSurveyMapViewMetrics(viewport, plotRect);
  return {
    x: viewport.xMin + (x - metrics.drawRect.x) / Math.max(metrics.scale, 1e-6),
    y: viewport.yMin + (metrics.drawRect.y + metrics.drawRect.height - y) / Math.max(metrics.scale, 1e-6)
  };
}

export function pointInRect(point: SurveyMapPoint, rect: SurveyMapRect): boolean {
  return point.x >= rect.x && point.x <= rect.x + rect.width && point.y >= rect.y && point.y <= rect.y + rect.height;
}

function scalarFieldBounds(field: SurveyMapScalarField): SurveyMapBounds {
  const halfStepX = Math.abs(field.step.x) / 2;
  const halfStepY = Math.abs(field.step.y) / 2;
  return {
    minX: field.origin.x - halfStepX,
    maxX: field.origin.x + (field.columns - 1) * field.step.x + halfStepX,
    minY: field.origin.y - halfStepY,
    maxY: field.origin.y + (field.rows - 1) * field.step.y + halfStepY
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
