import type { SectionDimensions, SectionViewport } from "@ophiolite/charts-data-models";

export function fullViewport(dimensions: SectionDimensions): SectionViewport {
  return {
    traceStart: 0,
    traceEnd: dimensions.traces,
    sampleStart: 0,
    sampleEnd: dimensions.samples
  };
}

export function clampViewport(
  viewport: SectionViewport,
  dimensions: SectionDimensions
): SectionViewport {
  const width = Math.max(1, Math.min(dimensions.traces, viewport.traceEnd - viewport.traceStart));
  const height = Math.max(1, Math.min(dimensions.samples, viewport.sampleEnd - viewport.sampleStart));

  const traceStart = clamp(viewport.traceStart, 0, Math.max(0, dimensions.traces - width));
  const sampleStart = clamp(viewport.sampleStart, 0, Math.max(0, dimensions.samples - height));

  return {
    traceStart,
    traceEnd: traceStart + width,
    sampleStart,
    sampleEnd: sampleStart + height
  };
}

export function zoomViewport(
  viewport: SectionViewport,
  dimensions: SectionDimensions,
  factor: number
): SectionViewport {
  return zoomViewportAt(
    viewport,
    dimensions,
    factor,
    (viewport.traceStart + viewport.traceEnd) / 2,
    (viewport.sampleStart + viewport.sampleEnd) / 2
  );
}

export function zoomViewportAt(
  viewport: SectionViewport,
  dimensions: SectionDimensions,
  factor: number,
  centerTrace: number,
  centerSample: number
): SectionViewport {
  const width = Math.max(1, Math.round((viewport.traceEnd - viewport.traceStart) / factor));
  const height = Math.max(1, Math.round((viewport.sampleEnd - viewport.sampleStart) / factor));

  return clampViewport(
    {
      traceStart: Math.round(centerTrace - width / 2),
      traceEnd: Math.round(centerTrace + width / 2),
      sampleStart: Math.round(centerSample - height / 2),
      sampleEnd: Math.round(centerSample + height / 2)
    },
    dimensions
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
