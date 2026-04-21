import type { RenderMode, SectionPayload, SectionViewport } from "@ophiolite/charts-data-models";
import {
  sectionHorizontalCoordinateAt,
  sectionSampleValueAt
} from "@ophiolite/charts-data-models";
import { mapCoordinateToPlotX, type PlotRect } from "./wiggleGeometry";

export const PLOT_MARGIN = {
  top: 96,
  right: 24,
  bottom: 28,
  left: 68
} as const;

export function getPlotRect(width: number, height: number): PlotRect {
  return {
    x: PLOT_MARGIN.left,
    y: PLOT_MARGIN.top,
    width: Math.max(1, width - PLOT_MARGIN.left - PLOT_MARGIN.right),
    height: Math.max(1, height - PLOT_MARGIN.top - PLOT_MARGIN.bottom)
  };
}

export function traceIndexToScreenX(
  section: SectionPayload,
  viewport: SectionViewport,
  renderMode: RenderMode,
  plotRect: PlotRect,
  traceIndex: number
): number {
  if (renderMode === "wiggle") {
    const coords = [];
    for (let trace = viewport.traceStart; trace < viewport.traceEnd; trace += 1) {
      const coordinate = sectionHorizontalCoordinateAt(section, trace);
      if (coordinate !== null) {
        coords.push(coordinate);
      }
    }
    if (coords.length === 0) {
      return plotRect.x + plotRect.width / 2;
    }
    const traceCoordinate = sectionHorizontalCoordinateAt(section, traceIndex) ?? traceIndex;
    return mapCoordinateToPlotX(
      traceCoordinate,
      Math.min(...coords),
      Math.max(...coords),
      plotRect
    );
  }

  return (
    plotRect.x +
    ((traceIndex - viewport.traceStart) / Math.max(1, viewport.traceEnd - viewport.traceStart - 1)) * plotRect.width
  );
}

export function sampleIndexToScreenY(
  viewport: SectionViewport,
  plotRect: PlotRect,
  sampleIndex: number
): number {
  return (
    plotRect.y +
    ((sampleIndex - viewport.sampleStart) / Math.max(1, viewport.sampleEnd - viewport.sampleStart - 1)) * plotRect.height
  );
}

export function sampleValueToScreenY(
  section: SectionPayload,
  viewport: SectionViewport,
  plotRect: PlotRect,
  sampleValue: number
): number | null {
  const start = viewport.sampleStart;
  const end = viewport.sampleEnd;
  if (start < 0 || end <= start) {
    return null;
  }
  const first = sectionSampleValueAt(section, start);
  const last = sectionSampleValueAt(section, end - 1);
  if (first === null || last === null) {
    return null;
  }
  const ascending = last >= first;
  const minValue = ascending ? first : last;
  const maxValue = ascending ? last : first;
  if (sampleValue < minValue || sampleValue > maxValue) {
    return null;
  }

  let lower = start;
  let upper = end - 1;
  while (lower <= upper) {
    const middle = Math.floor((lower + upper) / 2);
    const current = sectionSampleValueAt(section, middle);
    if (current === null) {
      return null;
    }
    if (Math.abs(current - sampleValue) <= 1e-6) {
      return sampleIndexToScreenY(viewport, plotRect, middle);
    }
    if (ascending ? current < sampleValue : current > sampleValue) {
      lower = middle + 1;
    } else {
      upper = middle - 1;
    }
  }

  const nextIndex = clamp(lower, start + 1, end - 1);
  const previousIndex = nextIndex - 1;
  const previousValue = sectionSampleValueAt(section, previousIndex);
  const nextValue = sectionSampleValueAt(section, nextIndex);
  if (previousValue === null || nextValue === null) {
    return null;
  }
  const denominator = nextValue - previousValue;
  if (Math.abs(denominator) <= 1e-6) {
    return sampleIndexToScreenY(viewport, plotRect, previousIndex);
  }

  const ratio = (sampleValue - previousValue) / denominator;
  const previousY = sampleIndexToScreenY(viewport, plotRect, previousIndex);
  const nextY = sampleIndexToScreenY(viewport, plotRect, nextIndex);
  return previousY + (nextY - previousY) * ratio;
}

export function resolveNearestTraceIndex(
  section: SectionPayload,
  viewport: SectionViewport,
  renderMode: RenderMode,
  plotRect: PlotRect,
  x: number
): number {
  if (renderMode === "wiggle") {
    const visibleCoords = [];
    for (let trace = viewport.traceStart; trace < viewport.traceEnd; trace += 1) {
      const coordinate = sectionHorizontalCoordinateAt(section, trace);
      if (coordinate !== null) {
        visibleCoords.push(coordinate);
      }
    }
    if (visibleCoords.length === 0) {
      return viewport.traceStart;
    }
    const coordMin = Math.min(...visibleCoords);
    const coordMax = Math.max(...visibleCoords);
    const ratio = clamp((x - plotRect.x) / Math.max(1, plotRect.width), 0, 1);
    const targetCoord = coordMin + ratio * (coordMax - coordMin);
    return nearestCoordinateIndex(Float64Array.from(visibleCoords), targetCoord) + viewport.traceStart;
  }

  const ratio = clamp((x - plotRect.x) / Math.max(1, plotRect.width), 0, 1);
  return clamp(
    Math.round(ratio * Math.max(1, viewport.traceEnd - viewport.traceStart - 1)) + viewport.traceStart,
    viewport.traceStart,
    viewport.traceEnd - 1
  );
}

function nearestCoordinateIndex(coordinates: Float64Array, target: number): number {
  let left = 0;
  let right = coordinates.length - 1;
  while (left < right) {
    const middle = Math.floor((left + right) / 2);
    if (coordinates[middle] < target) {
      left = middle + 1;
    } else {
      right = middle;
    }
  }

  if (left === 0) {
    return 0;
  }

  const previous = coordinates[left - 1];
  const current = coordinates[left];
  return Math.abs(previous - target) <= Math.abs(current - target) ? left - 1 : left;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
