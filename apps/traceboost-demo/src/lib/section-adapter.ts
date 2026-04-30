import type {
  SectionHorizonOverlay as ChartSectionHorizonOverlay,
  SectionScalarOverlay as ChartSectionScalarOverlay
} from "@ophiolite/charts-data-models";
import type { SeismicSectionData } from "@ophiolite/charts";
import type { SectionHorizonOverlayView } from "@traceboost/seis-contracts";
import type {
  TransportSectionScalarOverlayView,
  TransportWindowedSectionView
} from "./bridge";

export type DecodeCopyMode = "copy" | "view";

export interface DecodeStats {
  copiedBuffers: number;
  copiedBytes: number;
  viewedBuffers: number;
  viewedBytes: number;
}

export interface DecodeOptions {
  copyMode?: DecodeCopyMode;
  stats?: DecodeStats;
}

export function createDecodeStats(): DecodeStats {
  return {
    copiedBuffers: 0,
    copiedBytes: 0,
    viewedBuffers: 0,
    viewedBytes: 0
  };
}

function recordCopy(stats: DecodeStats | undefined, byteLength: number): void {
  if (!stats) {
    return;
  }
  stats.copiedBuffers += 1;
  stats.copiedBytes += byteLength;
}

function recordView(stats: DecodeStats | undefined, byteLength: number): void {
  if (!stats) {
    return;
  }
  stats.viewedBuffers += 1;
  stats.viewedBytes += byteLength;
}

function sourceBytes(bytes: Array<number> | Uint8Array): { source: Uint8Array; copied: boolean } {
  if (bytes instanceof Uint8Array) {
    return { source: bytes, copied: false };
  }
  return { source: Uint8Array.from(bytes), copied: true };
}

export function decodeF32Le(
  bytes: Array<number> | Uint8Array | null | undefined,
  options: DecodeOptions = {}
): Float32Array {
  if (!bytes) {
    return new Float32Array(0);
  }

  const { source, copied } = sourceBytes(bytes);
  if (source.byteLength % Float32Array.BYTES_PER_ELEMENT !== 0) {
    throw new Error(`Expected f32 little-endian bytes, found ${source.byteLength} bytes.`);
  }

  if (
    options.copyMode !== "copy" &&
    !copied &&
    source.byteOffset % Float32Array.BYTES_PER_ELEMENT === 0
  ) {
    recordView(options.stats, source.byteLength);
    return new Float32Array(
      source.buffer,
      source.byteOffset,
      source.byteLength / Float32Array.BYTES_PER_ELEMENT
    );
  }

  recordCopy(options.stats, source.byteLength);
  return new Float32Array(source.buffer.slice(source.byteOffset, source.byteOffset + source.byteLength));
}

export function decodeF64Le(
  bytes: Array<number> | Uint8Array | null | undefined,
  options: DecodeOptions = {}
): Float64Array {
  if (!bytes) {
    return new Float64Array(0);
  }

  const { source, copied } = sourceBytes(bytes);
  if (source.byteLength % Float64Array.BYTES_PER_ELEMENT !== 0) {
    throw new Error(`Expected f64 little-endian bytes, found ${source.byteLength} bytes.`);
  }

  if (
    options.copyMode !== "copy" &&
    !copied &&
    source.byteOffset % Float64Array.BYTES_PER_ELEMENT === 0
  ) {
    recordView(options.stats, source.byteLength);
    return new Float64Array(
      source.buffer,
      source.byteOffset,
      source.byteLength / Float64Array.BYTES_PER_ELEMENT
    );
  }

  recordCopy(options.stats, source.byteLength);
  return new Float64Array(source.buffer.slice(source.byteOffset, source.byteOffset + source.byteLength));
}

export function adaptTransportWindowedSectionToChartData(
  section: TransportWindowedSectionView,
  options: DecodeOptions = {}
): SeismicSectionData {
  return {
    axis: section.axis,
    coordinate: {
      index: section.coordinate.index,
      value: section.coordinate.value
    },
    horizontalAxis: decodeF64Le(section.horizontal_axis_f64le, options),
    inlineAxis: section.inline_axis_f64le ? decodeF64Le(section.inline_axis_f64le, options) : undefined,
    xlineAxis: section.xline_axis_f64le ? decodeF64Le(section.xline_axis_f64le, options) : undefined,
    sampleAxis: decodeF32Le(section.sample_axis_f32le, options),
    amplitudes: decodeF32Le(section.amplitudes_f32le, options),
    dimensions: {
      traces: section.traces,
      samples: section.samples
    },
    units: section.units
      ? {
          horizontal: section.units.horizontal ?? undefined,
          sample: section.units.sample ?? undefined,
          amplitude: section.units.amplitude ?? undefined
        }
      : undefined,
    metadata: section.metadata
      ? {
          storeId: section.metadata.store_id ?? undefined,
          derivedFrom: section.metadata.derived_from ?? undefined,
          notes: section.metadata.notes
        }
      : undefined,
    logicalDimensions: section.logical_dimensions
      ? {
          traces: section.logical_dimensions.traces,
          samples: section.logical_dimensions.samples
        }
      : undefined,
    window: section.window
      ? {
          traceStart: section.window.trace_start,
          traceEnd: section.window.trace_end,
          sampleStart: section.window.sample_start,
          sampleEnd: section.window.sample_end,
          lod: section.window.lod ?? undefined
        }
      : undefined,
    displayDefaults: section.display_defaults
      ? {
          gain: section.display_defaults.gain,
          clipMin: section.display_defaults.clip_min ?? undefined,
          clipMax: section.display_defaults.clip_max ?? undefined,
          renderMode: section.display_defaults.render_mode === "wiggle" ? "wiggle" : "heatmap",
          colormap:
            section.display_defaults.colormap === "red_white_blue" ? "red-white-blue" : "grayscale",
          polarity: section.display_defaults.polarity === "reversed" ? "reversed" : "normal"
        }
      : undefined
  };
}

export function adaptSectionHorizonOverlays(
  overlays: SectionHorizonOverlayView[]
): ChartSectionHorizonOverlay[] {
  return overlays.map((overlay) => ({
    id: overlay.id,
    name: overlay.name ?? undefined,
    color: overlay.style.color,
    lineWidth: overlay.style.line_width ?? undefined,
    lineStyle: overlay.style.line_style,
    opacity: overlay.style.opacity ?? undefined,
    samples: overlay.samples.map((sample) => ({
      traceIndex: sample.trace_index,
      sampleIndex: sample.sample_index,
      sampleValue: sample.sample_value ?? undefined
    }))
  }));
}

export function adaptSectionScalarOverlays(
  overlays: TransportSectionScalarOverlayView[],
  opacityOverride?: number,
  options: DecodeOptions = {}
): ChartSectionScalarOverlay[] {
  return overlays.map((overlay) => ({
    id: overlay.id,
    name: overlay.name ?? undefined,
    width: overlay.width,
    height: overlay.height,
    values: decodeF32Le(overlay.values_f32le, options),
    colorMap: overlay.color_map,
    opacity: opacityOverride ?? overlay.opacity,
    valueRange: overlay.value_range,
    units: overlay.units ?? undefined
  }));
}
