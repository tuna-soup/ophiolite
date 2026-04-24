import type { OphioliteEncodedSectionView } from "../../packages/data-models/src/ophiolite-seismic-adapter";
import type { CursorProbe, SectionPayload, SectionViewport } from "../../packages/data-models/src/seismic";

export interface EncodedSectionFixtureOptions {
  axis?: "inline" | "xline";
  traces?: number;
  samples?: number;
  coordinate?: {
    index: number;
    value: number;
  };
  logicalDimensions?: {
    traces: number;
    samples: number;
  };
  window?: {
    traceStart: number;
    traceEnd: number;
    sampleStart: number;
    sampleEnd: number;
    lod?: number;
  };
  displayDefaults?: {
    gain: number;
    clipMin?: number;
    clipMax?: number;
    renderMode?: "heatmap" | "wiggle";
    colormap?: "grayscale" | "red_white_blue";
    polarity?: "normal" | "reversed";
  };
  units?: {
    horizontal?: string;
    sample?: string;
    amplitude?: string;
  };
  metadata?: {
    storeId?: string;
    derivedFrom?: string;
    notes?: string[];
  };
  horizontalAxis?: number[];
  inlineAxis?: number[] | null;
  xlineAxis?: number[] | null;
  sampleAxis?: number[];
  amplitudes?: number[];
}

export interface SectionPayloadFixtureOptions {
  axis?: "inline" | "xline";
  traces?: number;
  samples?: number;
  coordinate?: {
    index: number;
    value: number;
  };
  logicalDimensions?: {
    traces: number;
    samples: number;
  };
  window?: {
    traceStart: number;
    traceEnd: number;
    sampleStart: number;
    sampleEnd: number;
    lod?: number;
  };
  displayDefaults?: SectionPayload["displayDefaults"];
  units?: SectionPayload["units"];
  metadata?: SectionPayload["metadata"];
  horizontalAxis?: number[];
  inlineAxis?: number[] | null;
  xlineAxis?: number[] | null;
  sampleAxis?: number[];
  amplitudes?: number[];
}

export function encodeFloat32(values: number[]): Uint8Array {
  return new Uint8Array(new Float32Array(values).buffer.slice(0));
}

export function encodeFloat64(values: number[]): Uint8Array {
  return new Uint8Array(new Float64Array(values).buffer.slice(0));
}

export function createEncodedSectionView(options: EncodedSectionFixtureOptions = {}): OphioliteEncodedSectionView {
  const traces = options.traces ?? 4;
  const samples = options.samples ?? 5;
  const axis = options.axis ?? "inline";
  const coordinate = options.coordinate ?? { index: 12, value: 1820.5 };
  const horizontalAxis = options.horizontalAxis ?? sequence(traces, 1000, 12.5);
  const inlineAxis = options.inlineAxis ?? (axis === "inline" ? sequence(traces, 1820, 1) : null);
  const xlineAxis = options.xlineAxis ?? (axis === "xline" ? sequence(traces, 940, 1) : null);
  const sampleAxis = options.sampleAxis ?? sequence(samples, 1500, 4);
  const amplitudes = options.amplitudes ?? sequence(traces * samples, -0.45, 0.1);
  const displayDefaults = options.displayDefaults;

  return {
    dataset_id: "fixture-dataset",
    axis,
    coordinate,
    traces,
    samples,
    horizontal_axis_f64le: encodeFloat64(horizontalAxis),
    inline_axis_f64le: inlineAxis ? encodeFloat64(inlineAxis) : null,
    xline_axis_f64le: xlineAxis ? encodeFloat64(xlineAxis) : null,
    sample_axis_f32le: encodeFloat32(sampleAxis),
    amplitudes_f32le: encodeFloat32(amplitudes),
    units: options.units
      ? {
          horizontal: options.units.horizontal ?? null,
          sample: options.units.sample ?? null,
          amplitude: options.units.amplitude ?? null
        }
      : null,
    metadata: options.metadata
      ? {
          store_id: options.metadata.storeId ?? null,
          derived_from: options.metadata.derivedFrom ?? null,
          notes: options.metadata.notes ?? []
        }
      : null,
    logical_dimensions: options.logicalDimensions
      ? {
          traces: options.logicalDimensions.traces,
          samples: options.logicalDimensions.samples
        }
      : undefined,
    window: options.window
      ? {
          trace_start: options.window.traceStart,
          trace_end: options.window.traceEnd,
          sample_start: options.window.sampleStart,
          sample_end: options.window.sampleEnd,
          lod: options.window.lod
        }
      : undefined,
    display_defaults: displayDefaults
      ? {
          gain: displayDefaults.gain,
          clip_min: displayDefaults.clipMin ?? null,
          clip_max: displayDefaults.clipMax ?? null,
          render_mode: displayDefaults.renderMode ?? "heatmap",
          colormap: displayDefaults.colormap ?? "grayscale",
          polarity: displayDefaults.polarity ?? "normal"
        }
      : null
  };
}

export function createSectionPayload(options: SectionPayloadFixtureOptions = {}): SectionPayload {
  const traces = options.traces ?? 4;
  const samples = options.samples ?? 5;
  const axis = options.axis ?? "inline";
  const coordinate = options.coordinate ?? { index: 12, value: 1820.5 };
  const horizontalAxis = options.horizontalAxis ?? Float64Array.from(sequence(traces, 1000, 12.5));
  const inlineAxisValues = options.inlineAxis ?? (axis === "inline" ? sequence(traces, 1820, 1) : null);
  const xlineAxisValues = options.xlineAxis ?? (axis === "xline" ? sequence(traces, 940, 1) : null);
  const sampleAxis = options.sampleAxis ?? sequence(samples, 1500, 4);
  const amplitudes = options.amplitudes ?? sequence(traces * samples, -0.45, 0.1);

  return {
    axis,
    coordinate,
    horizontalAxis,
    inlineAxis: inlineAxisValues ? Float64Array.from(inlineAxisValues) : undefined,
    xlineAxis: xlineAxisValues ? Float64Array.from(xlineAxisValues) : undefined,
    sampleAxis: Float32Array.from(sampleAxis),
    amplitudes: Float32Array.from(amplitudes),
    dimensions: { traces, samples },
    logicalDimensions: options.logicalDimensions,
    window: options.window,
    displayDefaults: options.displayDefaults,
    units: options.units,
    metadata: options.metadata
  };
}

export function createArbitrarySectionPayload(): SectionPayload {
  return {
    ...createSectionPayload({
      axis: "inline",
      coordinate: { index: 5, value: 42.5 },
      units: { sample: "ms" }
    }),
    inlineAxis: Float64Array.from([1800, 1801, 1803, 1806]),
    xlineAxis: Float64Array.from([940, 943, 947, 952]),
    presentation: {
      title: "Arbitrary Traverse",
      sampleAxisLabel: "TWT"
    }
  };
}

export function createSectionViewport(): SectionViewport {
  return {
    traceStart: 10,
    traceEnd: 30,
    sampleStart: 4,
    sampleEnd: 28
  };
}

export function createCursorProbe(): CursorProbe {
  return {
    traceIndex: 14,
    traceCoordinate: 1042.5,
    inlineCoordinate: 1822,
    xlineCoordinate: 946,
    sampleIndex: 9,
    sampleValue: 1536,
    amplitude: -0.12,
    screenX: 412,
    screenY: 184
  };
}

function sequence(count: number, start: number, step: number): number[] {
  return Array.from({ length: count }, (_, index) => start + index * step);
}
