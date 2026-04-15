import type { SectionColorMap, SectionPolarity, SectionPrimaryMode, SectionRenderMode } from "@ophiolite/contracts";
import type { SeismicChartDisplayTransform, SeismicChartPrimaryMode } from "./types";

export function decodeFloat32(bytes: number[] | Uint8Array): Float32Array {
  if (bytes.length === 0) {
    return new Float32Array(0);
  }

  const array = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  return new Float32Array(array.buffer, array.byteOffset, array.byteLength / Float32Array.BYTES_PER_ELEMENT);
}

export function decodeFloat64(bytes: number[] | Uint8Array): Float64Array {
  if (bytes.length === 0) {
    return new Float64Array(0);
  }

  const array = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  return new Float64Array(array.buffer, array.byteOffset, array.byteLength / Float64Array.BYTES_PER_ELEMENT);
}

export function decodeOptionalFloat64(bytes: number[] | Uint8Array | null | undefined): Float64Array | undefined {
  if (!bytes) {
    return undefined;
  }
  return decodeFloat64(bytes);
}

export function fromContractRenderMode(value: SectionRenderMode): SeismicChartDisplayTransform["renderMode"] {
  return value === "wiggle" ? "wiggle" : "heatmap";
}

export function fromContractColorMap(value: SectionColorMap): SeismicChartDisplayTransform["colormap"] {
  return value === "red_white_blue" ? "red-white-blue" : "grayscale";
}

export function fromContractPolarity(value: SectionPolarity): SeismicChartDisplayTransform["polarity"] {
  return value === "reversed" ? "reversed" : "normal";
}

export function toContractPrimaryMode(value: SeismicChartPrimaryMode): SectionPrimaryMode {
  return value === "panZoom" ? "pan_zoom" : "cursor";
}
