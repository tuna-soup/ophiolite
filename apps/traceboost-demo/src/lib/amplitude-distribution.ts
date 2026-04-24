import type { SeismicSectionAnalysisSelection } from "@ophiolite/charts";
import type { SectionView } from "@traceboost/seis-contracts";
import type { TransportSectionView } from "./bridge";

export type DisplaySectionView = SectionView | TransportSectionView;

export interface AmplitudeDistributionBin {
  start: number;
  end: number;
  count: number;
}

export interface AmplitudeDistributionResult {
  bins: AmplitudeDistributionBin[];
  count: number;
  min: number;
  max: number;
  mean: number;
  standardDeviation: number;
  median: number;
  rms: number;
}

function decodeF32Le(bytes: Array<number> | Uint8Array | null | undefined): Float32Array {
  if (!bytes) {
    return new Float32Array(0);
  }

  const source = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  if (source.byteLength % Float32Array.BYTES_PER_ELEMENT !== 0) {
    throw new Error(`Expected f32 little-endian bytes, found ${source.byteLength} bytes.`);
  }

  return new Float32Array(source.buffer.slice(source.byteOffset, source.byteOffset + source.byteLength));
}

export function buildAmplitudeDistribution(
  section: DisplaySectionView,
  preferredBinCount = 48,
  selection: SeismicSectionAnalysisSelection = { kind: "whole-section" }
): AmplitudeDistributionResult {
  const amplitudes = sliceAmplitudeSelection(decodeF32Le(section.amplitudes_f32le), section, selection);
  if (amplitudes.length === 0) {
    return {
      bins: [],
      count: 0,
      min: 0,
      max: 0,
      mean: 0,
      standardDeviation: 0,
      median: 0,
      rms: 0
    };
  }

  let min = Infinity;
  let max = -Infinity;
  let sum = 0;
  let sumSquares = 0;

  for (let index = 0; index < amplitudes.length; index += 1) {
    const value = amplitudes[index] ?? 0;
    if (value < min) {
      min = value;
    }
    if (value > max) {
      max = value;
    }
    sum += value;
    sumSquares += value * value;
  }

  const count = amplitudes.length;
  const mean = sum / count;
  const rms = Math.sqrt(sumSquares / count);
  const variance = Math.max(0, sumSquares / count - mean * mean);
  const standardDeviation = Math.sqrt(variance);
  const sorted = Float32Array.from(amplitudes);
  sorted.sort();
  const medianIndex = Math.floor(sorted.length / 2);
  const median =
    sorted.length % 2 === 0
      ? ((sorted[medianIndex - 1] ?? 0) + (sorted[medianIndex] ?? 0)) / 2
      : (sorted[medianIndex] ?? 0);

  const binCount = Math.max(1, preferredBinCount);
  if (min === max) {
    const halfWidth = Math.max(1, Math.abs(min) * 0.05);
    return {
      bins: [
        {
          start: min - halfWidth,
          end: max + halfWidth,
          count
        }
      ],
      count,
      min,
      max,
      mean,
      standardDeviation,
      median,
      rms
    };
  }

  const width = (max - min) / binCount;
  const counts = new Array<number>(binCount).fill(0);
  for (let index = 0; index < amplitudes.length; index += 1) {
    const value = amplitudes[index] ?? 0;
    const rawIndex = Math.floor((value - min) / width);
    const clampedIndex = Math.max(0, Math.min(binCount - 1, rawIndex));
    counts[clampedIndex] = (counts[clampedIndex] ?? 0) + 1;
  }

  return {
    bins: counts.map((entry, index) => ({
      start: min + width * index,
      end: index === binCount - 1 ? max : min + width * (index + 1),
      count: entry
    })),
    count,
    min,
    max,
    mean,
    standardDeviation,
    median,
    rms
  };
}

function sliceAmplitudeSelection(
  amplitudes: Float32Array,
  section: DisplaySectionView,
  selection: SeismicSectionAnalysisSelection
): Float32Array {
  const traces = Math.max(0, section.traces);
  const samples = Math.max(0, section.samples);
  if (traces === 0 || samples === 0 || amplitudes.length === 0) {
    return new Float32Array(0);
  }

  if (selection.kind === "whole-section") {
    return amplitudes;
  }

  const rect =
    selection.kind === "viewport"
      ? {
          left: selection.viewport.traceStart,
          right: selection.viewport.traceEnd,
          top: selection.viewport.sampleStart,
          bottom: selection.viewport.sampleEnd
        }
      : selection.rectangle;

  const traceStart = clamp(Math.floor(rect.left), 0, traces);
  const traceEnd = clamp(Math.ceil(rect.right), traceStart, traces);
  const sampleStart = clamp(Math.floor(rect.top), 0, samples);
  const sampleEnd = clamp(Math.ceil(rect.bottom), sampleStart, samples);

  const selected = new Float32Array((traceEnd - traceStart) * (sampleEnd - sampleStart));
  let targetIndex = 0;
  for (let traceIndex = traceStart; traceIndex < traceEnd; traceIndex += 1) {
    const traceOffset = traceIndex * samples;
    for (let sampleIndex = sampleStart; sampleIndex < sampleEnd; sampleIndex += 1) {
      selected[targetIndex] = amplitudes[traceOffset + sampleIndex] ?? 0;
      targetIndex += 1;
    }
  }
  return selected;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
