import type {
  DatasetId,
  SectionAxis,
  SectionCoordinate,
  SectionDisplayDefaults,
  SectionHorizonLineStyle,
  SectionHorizonOverlayView,
  SectionMetadata,
  SectionTimeDepthDiagnostics,
  SectionUnits,
  SectionView
} from "@traceboost/seis-contracts";

export type SectionBytePayload = Array<number> | Uint8Array;

export interface TransportSectionView
  extends Omit<
    SectionView,
    "horizontal_axis_f64le" | "inline_axis_f64le" | "xline_axis_f64le" | "sample_axis_f32le" | "amplitudes_f32le"
  > {
  horizontal_axis_f64le: SectionBytePayload;
  inline_axis_f64le: SectionBytePayload | null;
  xline_axis_f64le: SectionBytePayload | null;
  sample_axis_f32le: SectionBytePayload;
  amplitudes_f32le: SectionBytePayload;
}

export interface TransportSectionScalarOverlayView {
  id: string;
  name: string | null;
  width: number;
  height: number;
  values_f32le: SectionBytePayload;
  color_map: "grayscale" | "viridis" | "turbo";
  opacity: number;
  value_range: {
    min: number;
    max: number;
  };
  units: string | null;
}

export interface TransportResolvedSectionDisplayView {
  section: TransportSectionView;
  time_depth_diagnostics: SectionTimeDepthDiagnostics | null;
  scalar_overlays: TransportSectionScalarOverlayView[];
  horizon_overlays: SectionHorizonOverlayView[];
}

export interface TransportPreviewView {
  section: TransportSectionView;
  processing_label: string;
  preview_ready: boolean;
}

export interface TransportPreviewProcessingResponse {
  preview: TransportPreviewView;
}

interface PackedPreviewSectionHeader {
  datasetId: DatasetId;
  axis: SectionAxis;
  coordinate: SectionCoordinate;
  traces: number;
  samples: number;
  horizontalAxisBytes: number;
  inlineAxisBytes: number | null;
  xlineAxisBytes: number | null;
  sampleAxisBytes: number;
  amplitudesBytes: number;
  units: SectionUnits | null;
  metadata: SectionMetadata | null;
  displayDefaults: SectionDisplayDefaults | null;
}

interface PackedPreviewResponseHeader {
  section: PackedPreviewSectionHeader;
  previewReady: boolean;
  processingLabel: string;
}

interface PackedSectionResponseHeader {
  section: PackedPreviewSectionHeader;
}

interface PackedSectionScalarOverlayHeader {
  id: string;
  name: string | null;
  width: number;
  height: number;
  valuesBytes: number;
  colorMap: "grayscale" | "viridis" | "turbo";
  opacity: number;
  valueRange: {
    min: number;
    max: number;
  };
  units: string | null;
}

interface PackedSectionDisplayResponseHeader {
  section: PackedPreviewSectionHeader;
  timeDepthDiagnostics: SectionTimeDepthDiagnostics | null;
  scalarOverlays: PackedSectionScalarOverlayHeader[];
  horizonOverlays: Array<{
    id: string;
    name: string | null;
    style: {
      color: string;
      line_width: number | null;
      line_style: SectionHorizonLineStyle;
      opacity: number | null;
    };
    samples: SectionHorizonOverlayView["samples"];
  }>;
}

function nextBytesReader(bytes: Uint8Array, startOffset: number): (length: number | null) => Uint8Array | null {
  let cursor = startOffset;
  return (length: number | null) => {
    if (length === null) {
      return null;
    }
    const next = bytes.subarray(cursor, cursor + length);
    cursor += length;
    return next;
  };
}

function parseHeader<T>(bytes: Uint8Array, magic: string, errorLabel: string): { header: T; nextBytes: (length: number | null) => Uint8Array | null } {
  if (bytes.byteLength < 16) {
    throw new Error(`${errorLabel} is too small.`);
  }

  const actualMagic = new TextDecoder().decode(bytes.subarray(0, 8));
  if (actualMagic !== magic) {
    throw new Error(`Unexpected ${errorLabel} magic: ${actualMagic}`);
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const headerLength = view.getUint32(8, true);
  const dataOffset = view.getUint32(12, true);
  const headerStart = 16;
  const headerEnd = headerStart + headerLength;
  if (headerEnd > bytes.byteLength || dataOffset > bytes.byteLength || dataOffset < headerEnd) {
    throw new Error(`${errorLabel} header is invalid.`);
  }

  const header = JSON.parse(
    new TextDecoder().decode(bytes.subarray(headerStart, headerEnd))
  ) as T;

  return {
    header,
    nextBytes: nextBytesReader(bytes, dataOffset)
  };
}

function sectionFromPackedHeader(
  header: PackedPreviewSectionHeader,
  nextBytes: (length: number | null) => Uint8Array | null
): TransportSectionView {
  return {
    dataset_id: header.datasetId,
    axis: header.axis,
    coordinate: header.coordinate,
    traces: header.traces,
    samples: header.samples,
    horizontal_axis_f64le: nextBytes(header.horizontalAxisBytes) ?? new Uint8Array(0),
    inline_axis_f64le: nextBytes(header.inlineAxisBytes),
    xline_axis_f64le: nextBytes(header.xlineAxisBytes),
    sample_axis_f32le: nextBytes(header.sampleAxisBytes) ?? new Uint8Array(0),
    amplitudes_f32le: nextBytes(header.amplitudesBytes) ?? new Uint8Array(0),
    units: header.units,
    metadata: header.metadata,
    display_defaults: header.displayDefaults
  };
}

export function parsePackedPreviewProcessingResponse(bytes: Uint8Array): TransportPreviewProcessingResponse {
  const { header, nextBytes } = parseHeader<PackedPreviewResponseHeader>(bytes, "TBPRV001", "packed preview response");
  return {
    preview: {
      preview_ready: header.previewReady,
      processing_label: header.processingLabel,
      section: sectionFromPackedHeader(header.section, nextBytes)
    }
  };
}

export function parsePackedSectionViewResponse(bytes: Uint8Array): TransportSectionView {
  const { header, nextBytes } = parseHeader<PackedSectionResponseHeader>(bytes, "TBSEC001", "packed section response");
  return sectionFromPackedHeader(header.section, nextBytes);
}

export function parsePackedSectionDisplayResponse(bytes: Uint8Array): TransportResolvedSectionDisplayView {
  const { header, nextBytes } = parseHeader<PackedSectionDisplayResponseHeader>(
    bytes,
    "TBSDP001",
    "packed section display response"
  );

  const section = sectionFromPackedHeader(header.section, nextBytes);
  const scalar_overlays = header.scalarOverlays.map((overlay) => ({
    id: overlay.id,
    name: overlay.name,
    width: overlay.width,
    height: overlay.height,
    values_f32le: nextBytes(overlay.valuesBytes) ?? new Uint8Array(0),
    color_map: overlay.colorMap,
    opacity: overlay.opacity,
    value_range: overlay.valueRange,
    units: overlay.units
  }));

  return {
    section,
    time_depth_diagnostics: header.timeDepthDiagnostics,
    scalar_overlays,
    horizon_overlays: header.horizonOverlays.map((overlay) => ({
      id: overlay.id,
      name: overlay.name ?? null,
      style: {
        color: overlay.style.color,
        line_width: overlay.style.line_width ?? null,
        line_style: overlay.style.line_style,
        opacity: overlay.style.opacity ?? null
      },
      samples: overlay.samples
    }))
  };
}
