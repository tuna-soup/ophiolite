import type {
  CursorProbe,
  Horizon,
  HorizonAnchor,
  HorizonPick,
  HorizonSnapMode,
  RenderMode,
  SectionPayload,
  SectionViewport
} from "@ophiolite/charts-data-models";
import {
  sectionAmplitudeAt,
  sectionHorizontalCoordinateAt,
  sectionInlineCoordinateAt,
  sectionSampleValueAt,
  sectionXlineCoordinateAt
} from "@ophiolite/charts-data-models";
import { getPlotRect, resolveNearestTraceIndex, sampleIndexToScreenY, traceIndexToScreenX } from "@ophiolite/charts-renderer";

const DEFAULT_SNAP_WINDOW = 8;

export function buildCursorProbe(
  section: SectionPayload,
  viewport: SectionViewport,
  renderMode: RenderMode,
  viewWidth: number,
  viewHeight: number,
  x: number,
  y: number
): CursorProbe | null {
  const plotRect = getPlotRect(viewWidth, viewHeight);
  if (x < plotRect.x || x > plotRect.x + plotRect.width || y < plotRect.y || y > plotRect.y + plotRect.height) {
    return null;
  }

  const traceIndex = resolveNearestTraceIndex(section, viewport, renderMode, plotRect, x);
  const sampleIndex = clamp(
    Math.round(((y - plotRect.y) / Math.max(1, plotRect.height)) * Math.max(1, viewport.sampleEnd - viewport.sampleStart - 1)) +
      viewport.sampleStart,
    viewport.sampleStart,
    viewport.sampleEnd - 1
  );

  return pickAt(section, viewport, renderMode, plotRect, traceIndex, sampleIndex);
}

export function upsertAnchor(
  section: SectionPayload,
  horizon: Horizon,
  anchor: HorizonAnchor,
  snapWindow: number = DEFAULT_SNAP_WINDOW
): Horizon {
  const anchors = [...horizon.anchors];
  const existingIndex = anchors.findIndex((candidate) => candidate.id === anchor.id || candidate.traceIndex === anchor.traceIndex);
  if (existingIndex >= 0) {
    anchors[existingIndex] = anchor;
  } else {
    anchors.push(anchor);
  }

  const sortedAnchors = anchors.sort((left, right) => left.traceIndex - right.traceIndex);
  return {
    ...horizon,
    anchors: sortedAnchors,
    picks: recomputeHorizonPicks(section, sortedAnchors, horizon.snapMode, snapWindow)
  };
}

export function removeAnchor(
  section: SectionPayload,
  horizon: Horizon,
  anchorId: string,
  snapWindow: number = DEFAULT_SNAP_WINDOW
): Horizon {
  const anchors = horizon.anchors.filter((anchor) => anchor.id !== anchorId);
  return {
    ...horizon,
    anchors,
    picks: recomputeHorizonPicks(section, anchors, horizon.snapMode, snapWindow)
  };
}

export function recomputeHorizon(
  section: SectionPayload,
  horizon: Horizon,
  snapWindow: number = DEFAULT_SNAP_WINDOW
): Horizon {
  return {
    ...horizon,
    picks: recomputeHorizonPicks(section, horizon.anchors, horizon.snapMode, snapWindow)
  };
}

export function createHorizon(name: string, color: string, snapMode: HorizonSnapMode): Horizon {
  return {
    id: `horizon-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name,
    color,
    snapMode,
    anchors: [],
    picks: []
  };
}

export function serializeHorizons(horizons: Horizon[]): string {
  return JSON.stringify(horizons, null, 2);
}

export function parseHorizons(json: string): Horizon[] {
  const parsed = JSON.parse(json) as Horizon[];
  return Array.isArray(parsed) ? parsed : [];
}

export function pickAt(
  section: SectionPayload,
  viewport: SectionViewport,
  renderMode: RenderMode,
  plotRect: ReturnType<typeof getPlotRect>,
  traceIndex: number,
  sampleIndex: number
): CursorProbe {
  const amplitude = sectionAmplitudeAt(section, traceIndex, sampleIndex) ?? 0;
  return {
    traceIndex,
    traceCoordinate: sectionHorizontalCoordinateAt(section, traceIndex) ?? traceIndex,
    inlineCoordinate: sectionInlineCoordinateAt(section, traceIndex) ?? undefined,
    xlineCoordinate: sectionXlineCoordinateAt(section, traceIndex) ?? undefined,
    sampleIndex,
    sampleValue: sectionSampleValueAt(section, sampleIndex) ?? sampleIndex,
    amplitude,
    screenX: traceIndexToScreenX(section, viewport, renderMode, plotRect, traceIndex),
    screenY: sampleIndexToScreenY(viewport, plotRect, sampleIndex)
  };
}

function recomputeHorizonPicks(
  section: SectionPayload,
  anchors: HorizonAnchor[],
  snapMode: HorizonSnapMode,
  snapWindow: number
): HorizonPick[] {
  if (anchors.length === 0) {
    return [];
  }

  if (anchors.length === 1) {
    const anchor = anchors[0];
    return [snapPick(section, anchor.traceIndex, anchor.sampleIndex, snapMode, snapWindow)];
  }

  const picks: HorizonPick[] = [];
  for (let anchorIndex = 0; anchorIndex < anchors.length - 1; anchorIndex += 1) {
    const left = anchors[anchorIndex];
    const right = anchors[anchorIndex + 1];
    const traceStart = left.traceIndex;
    const traceEnd = right.traceIndex;
    const leftCoord = sectionHorizontalCoordinateAt(section, left.traceIndex) ?? left.traceIndex;
    const rightCoord = sectionHorizontalCoordinateAt(section, right.traceIndex) ?? right.traceIndex;

    for (let traceIndex = traceStart; traceIndex <= traceEnd; traceIndex += 1) {
      if (anchorIndex > 0 && traceIndex === traceStart) {
        continue;
      }
      const coordinate = sectionHorizontalCoordinateAt(section, traceIndex) ?? traceIndex;
      const ratio =
        rightCoord === leftCoord ? 0 : (coordinate - leftCoord) / (rightCoord - leftCoord);
      const provisional = Math.round(left.sampleIndex + ratio * (right.sampleIndex - left.sampleIndex));
      picks.push(snapPick(section, traceIndex, provisional, snapMode, snapWindow));
    }
  }

  return picks;
}

function snapPick(
  section: SectionPayload,
  traceIndex: number,
  targetSampleIndex: number,
  snapMode: HorizonSnapMode,
  snapWindow: number
): HorizonPick {
  const sampleIndex = snapTraceSample(section, traceIndex, targetSampleIndex, snapMode, snapWindow);
  return {
    traceIndex,
    traceCoordinate: sectionHorizontalCoordinateAt(section, traceIndex) ?? traceIndex,
    sampleIndex,
    sampleValue: sectionSampleValueAt(section, sampleIndex) ?? sampleIndex,
    amplitude: sectionAmplitudeAt(section, traceIndex, sampleIndex) ?? 0
  };
}

function snapTraceSample(
  section: SectionPayload,
  traceIndex: number,
  targetSampleIndex: number,
  snapMode: HorizonSnapMode,
  snapWindow: number
): number {
  const samplesPerTrace = section.dimensions.samples;
  const start = clamp(targetSampleIndex - snapWindow, 1, samplesPerTrace - 2);
  const end = clamp(targetSampleIndex + snapWindow, 1, samplesPerTrace - 2);
  const candidates: number[] = [];

  for (let sampleIndex = start; sampleIndex <= end; sampleIndex += 1) {
    const previous = sectionAmplitudeAt(section, traceIndex, sampleIndex - 1) ?? 0;
    const current = sectionAmplitudeAt(section, traceIndex, sampleIndex) ?? 0;
    const next = sectionAmplitudeAt(section, traceIndex, sampleIndex + 1) ?? 0;
    const isPeak = current >= previous && current >= next;
    const isTrough = current <= previous && current <= next;
    if ((snapMode === "peak" && isPeak) || (snapMode === "trough" && isTrough)) {
      candidates.push(sampleIndex);
    }
  }

  if (candidates.length === 0) {
    return clamp(targetSampleIndex, 0, samplesPerTrace - 1);
  }

  return candidates.sort((left, right) => {
    const distance = Math.abs(left - targetSampleIndex) - Math.abs(right - targetSampleIndex);
    if (distance !== 0) {
      return distance;
    }
    const leftAmplitude = sectionAmplitudeAt(section, traceIndex, left) ?? 0;
    const rightAmplitude = sectionAmplitudeAt(section, traceIndex, right) ?? 0;
    return snapMode === "peak" ? rightAmplitude - leftAmplitude : leftAmplitude - rightAmplitude;
  })[0];
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
