export interface PlotRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WigglePoint {
  x: number;
  y: number;
  offset: number;
}

export interface WiggleTraceGeometry {
  traceIndex: number;
  baselineX: number;
  amplitudeScale: number;
  stroke: WigglePoint[];
  positiveFillSegments: WigglePoint[][];
}

export interface WigglePanelGeometry {
  traceStride: number;
  coordMin: number;
  coordMax: number;
  traces: WiggleTraceGeometry[];
}

export interface WigglePanelGeometryArgs {
  horizontalAxis: Float64Array;
  amplitudes: Float32Array;
  samplesPerTrace: number;
  traceStart: number;
  traceEnd: number;
  sampleStart: number;
  sampleEnd: number;
  gain: number;
  polarity: "normal" | "reversed";
  plotRect: PlotRect;
  minTraceSpacingPx: number;
  amplitudeRatio: number;
}

export function buildWigglePanelGeometry(args: WigglePanelGeometryArgs): WigglePanelGeometry {
  const {
    horizontalAxis,
    amplitudes,
    samplesPerTrace,
    traceStart,
    traceEnd,
    sampleStart,
    sampleEnd,
    gain,
    polarity,
    plotRect,
    minTraceSpacingPx,
    amplitudeRatio
  } = args;

  const visibleTraceCount = Math.max(1, traceEnd - traceStart);
  const maxReadableTraces = Math.max(1, Math.floor(plotRect.width / minTraceSpacingPx));
  const traceStride = Math.max(1, Math.ceil(visibleTraceCount / maxReadableTraces));
  const drawnTraceIndices = buildTraceIndices(traceStart, traceEnd, traceStride);
  const visibleCoords = Array.from(horizontalAxis.slice(traceStart, traceEnd));
  const coordMin = Math.min(...visibleCoords);
  const coordMax = Math.max(...visibleCoords);
  const globalScale = visibleAmplitudeScale(
    amplitudes,
    samplesPerTrace,
    traceStart,
    traceEnd,
    sampleStart,
    sampleEnd,
    gain
  );

  const baselines = drawnTraceIndices.map((traceIndex) =>
    mapCoordinateToPlotX(horizontalAxis[traceIndex], coordMin, coordMax, plotRect)
  );

  const traces = drawnTraceIndices.map((traceIndex, index) => {
    const baselineX = baselines[index];
    const localSpacing = localBaselineSpacing(baselines, index, plotRect.width);
    const amplitudeScale = Math.max(localSpacing * amplitudeRatio, 1);
    const stroke = buildTracePoints(
      amplitudes,
      samplesPerTrace,
      traceIndex,
      sampleStart,
      sampleEnd,
      plotRect,
      baselineX,
      amplitudeScale,
      globalScale,
      gain,
      polarity
    );

    return {
      traceIndex,
      baselineX,
      amplitudeScale,
      stroke,
      positiveFillSegments: buildPositiveFillSegments(stroke)
    };
  });

  return {
    traceStride,
    coordMin,
    coordMax,
    traces
  };
}

export function mapCoordinateToPlotX(
  coordinate: number,
  coordMin: number,
  coordMax: number,
  plotRect: PlotRect
): number {
  if (coordMax === coordMin) {
    return plotRect.x + plotRect.width / 2;
  }
  const ratio = (coordinate - coordMin) / (coordMax - coordMin);
  return plotRect.x + ratio * plotRect.width;
}

export function buildTracePoints(
  amplitudes: Float32Array,
  samplesPerTrace: number,
  traceIndex: number,
  sampleStart: number,
  sampleEnd: number,
  plotRect: PlotRect,
  baselineX: number,
  amplitudeScale: number,
  globalScale: number,
  gain: number,
  polarity: "normal" | "reversed"
): WigglePoint[] {
  const points: WigglePoint[] = [];
  const sampleCount = Math.max(1, sampleEnd - sampleStart);

  for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
    const offset = sampleAmplitudeOffset(
      amplitudes[traceIndex * samplesPerTrace + sample],
      gain,
      globalScale,
      amplitudeScale,
      polarity
    );
    const yRatio = sampleCount === 1 ? 0 : (sample - sampleStart) / (sampleCount - 1);
    const y = plotRect.y + yRatio * plotRect.height;
    const current: WigglePoint = {
      x: baselineX + offset,
      y,
      offset
    };

    points.push(current);

    if (sample === sampleEnd - 1) {
      continue;
    }

    const nextOffset = sampleAmplitudeOffset(
      amplitudes[traceIndex * samplesPerTrace + sample + 1],
      gain,
      globalScale,
      amplitudeScale,
      polarity
    );
    if (offset === 0 || nextOffset === 0 || Math.sign(offset) === Math.sign(nextOffset)) {
      continue;
    }

    const fraction = offset / (offset - nextOffset);
    const nextYRatio = sampleCount === 1 ? 0 : (sample + 1 - sampleStart) / (sampleCount - 1);
    const nextY = plotRect.y + nextYRatio * plotRect.height;
    points.push({
      x: baselineX,
      y: y + fraction * (nextY - y),
      offset: 0
    });
  }

  return points;
}

export function buildPositiveFillSegments(points: WigglePoint[]): WigglePoint[][] {
  const segments: WigglePoint[][] = [];
  let current: WigglePoint[] = [];

  for (const point of points) {
    if (point.offset >= 0) {
      current.push(point);
      continue;
    }

    if (current.length > 1) {
      segments.push(current);
    }
    current = [];
  }

  if (current.length > 1) {
    segments.push(current);
  }

  return segments;
}

export function visibleAmplitudeScale(
  amplitudes: Float32Array,
  samplesPerTrace: number,
  traceStart: number,
  traceEnd: number,
  sampleStart: number,
  sampleEnd: number,
  gain: number
): number {
  let maxAbs = 0;
  for (let trace = traceStart; trace < traceEnd; trace += 1) {
    for (let sample = sampleStart; sample < sampleEnd; sample += 1) {
      maxAbs = Math.max(maxAbs, Math.abs(amplitudes[trace * samplesPerTrace + sample] * gain));
    }
  }
  return Math.max(maxAbs, 1e-6);
}

function buildTraceIndices(traceStart: number, traceEnd: number, traceStride: number): number[] {
  const indices: number[] = [];
  for (let trace = traceStart; trace < traceEnd; trace += traceStride) {
    indices.push(trace);
  }
  return indices;
}

function localBaselineSpacing(baselines: number[], index: number, fallback: number): number {
  const previous = index > 0 ? Math.abs(baselines[index] - baselines[index - 1]) : Number.POSITIVE_INFINITY;
  const next =
    index < baselines.length - 1 ? Math.abs(baselines[index + 1] - baselines[index]) : Number.POSITIVE_INFINITY;
  const spacing = Math.min(previous, next);
  if (!Number.isFinite(spacing) || spacing === 0) {
    return fallback;
  }
  return spacing;
}

function sampleAmplitudeOffset(
  amplitude: number,
  gain: number,
  globalScale: number,
  amplitudeScale: number,
  polarity: "normal" | "reversed"
): number {
  const signedAmplitude = polarity === "reversed" ? -amplitude : amplitude;
  return (signedAmplitude * gain * amplitudeScale) / globalScale;
}
