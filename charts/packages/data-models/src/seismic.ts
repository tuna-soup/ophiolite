export type SectionAxis = "inline" | "xline";
export type ColorMapName = "grayscale" | "red-white-blue";
export type RenderMode = "heatmap" | "wiggle";
export type InteractionMode = "navigate" | "interpret";
export type HorizonSnapMode = "peak" | "trough";
export type ComparisonMode = "single" | "split";
export type SectionHorizonLineStyle = "solid" | "dashed" | "dotted";
export type SectionScalarOverlayColorMap = "grayscale" | "viridis" | "turbo";

export interface SectionCoordinate {
  index: number;
  value: number;
}

export interface SectionDimensions {
  traces: number;
  samples: number;
}

export interface SectionWindow {
  traceStart: number;
  traceEnd: number;
  sampleStart: number;
  sampleEnd: number;
  lod?: number;
}

export interface DisplayTransform {
  gain: number;
  clipMin?: number;
  clipMax?: number;
  renderMode: RenderMode;
  colormap: ColorMapName;
  polarity: "normal" | "reversed";
}

export interface SectionViewport {
  traceStart: number;
  traceEnd: number;
  sampleStart: number;
  sampleEnd: number;
}

export interface OverlayPayload {
  kind: "occupancy" | "mask" | "qc";
  width: number;
  height: number;
  values: Uint8Array;
  opacity?: number;
}

export interface CursorProbe {
  traceIndex: number;
  traceCoordinate: number;
  inlineCoordinate?: number;
  xlineCoordinate?: number;
  sampleIndex: number;
  sampleValue: number;
  amplitude: number;
  screenX: number;
  screenY: number;
}

export interface HorizonAnchor {
  id: string;
  traceIndex: number;
  sampleIndex: number;
}

export interface HorizonPick {
  traceIndex: number;
  traceCoordinate: number;
  sampleIndex: number;
  sampleValue: number;
  amplitude: number;
}

export interface SectionHorizonSample {
  traceIndex: number;
  sampleIndex: number;
  sampleValue?: number;
}

export interface AxisPresentationRow {
  label: string;
  values: Float64Array;
}

export interface SeismicPresentation {
  title?: string;
  sampleAxisLabel?: string;
  topAxisRows?: AxisPresentationRow[];
}

export interface Horizon {
  id: string;
  name: string;
  color: string;
  snapMode: HorizonSnapMode;
  anchors: HorizonAnchor[];
  picks: HorizonPick[];
}

export interface SectionHorizonOverlay {
  id: string;
  name?: string;
  color: string;
  lineWidth?: number;
  lineStyle?: SectionHorizonLineStyle;
  opacity?: number;
  samples: SectionHorizonSample[];
}

export interface SectionWellOverlaySample {
  traceIndex: number;
  traceCoordinate?: number;
  sampleIndex?: number;
  sampleValue?: number;
  measuredDepthM?: number;
  trueVerticalDepthM?: number;
  trueVerticalDepthSubseaM?: number;
  twtMs?: number;
}

export interface SectionWellOverlaySegment {
  samples: SectionWellOverlaySample[];
  notes?: string[];
}

export interface SectionWellOverlay {
  id: string;
  name?: string;
  color: string;
  lineWidth?: number;
  lineStyle?: SectionHorizonLineStyle;
  opacity?: number;
  segments: SectionWellOverlaySegment[];
  diagnostics?: string[];
}

export interface SectionScalarOverlay {
  id: string;
  name?: string;
  width: number;
  height: number;
  values: Float32Array;
  colorMap?: SectionScalarOverlayColorMap;
  opacity?: number;
  valueRange?: {
    min: number;
    max: number;
  };
  units?: string;
  noDataValue?: number;
}

export interface SectionPayload {
  axis: SectionAxis;
  coordinate: SectionCoordinate;
  horizontalAxis: Float64Array;
  inlineAxis?: Float64Array;
  xlineAxis?: Float64Array;
  sampleAxis: Float32Array;
  amplitudes: Float32Array;
  dimensions: SectionDimensions;
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
  logicalDimensions?: SectionDimensions;
  window?: SectionWindow;
  displayDefaults?: Partial<DisplayTransform>;
  overlay?: OverlayPayload;
  presentation?: SeismicPresentation;
}

export interface SectionTilePayload extends SectionPayload {
  lod: number;
  tile: {
    traceStart: number;
    traceEnd: number;
    sampleStart: number;
    sampleEnd: number;
  };
}

export interface ViewerState {
  section: SectionPayload | null;
  secondarySection: SectionPayload | null;
  viewport: SectionViewport | null;
  displayTransform: DisplayTransform;
  overlay: OverlayPayload | null;
  comparisonMode: ComparisonMode;
  splitPosition: number;
  interactionMode: InteractionMode;
  interactions: import("./interactions").InteractionState;
  probe: CursorProbe | null;
  sectionScalarOverlays: SectionScalarOverlay[];
  sectionHorizonOverlays: SectionHorizonOverlay[];
  sectionWellOverlays: SectionWellOverlay[];
  horizons: Horizon[];
  activeHorizonId: string | null;
}

export interface RenderFrame {
  state: ViewerState;
}

export function resolveLogicalSectionDimensions(section: SectionPayload): SectionDimensions {
  return section.logicalDimensions ?? section.dimensions;
}

export function resolveLoadedSectionWindow(section: SectionPayload): SectionWindow {
  return (
    section.window ?? {
      traceStart: 0,
      traceEnd: section.dimensions.traces,
      sampleStart: 0,
      sampleEnd: section.dimensions.samples,
      lod: 0
    }
  );
}

export function sectionWindowCoversViewport(section: SectionPayload, viewport: SectionViewport): boolean {
  const window = resolveLoadedSectionWindow(section);
  return (
    viewport.traceStart >= window.traceStart &&
    viewport.traceEnd <= window.traceEnd &&
    viewport.sampleStart >= window.sampleStart &&
    viewport.sampleEnd <= window.sampleEnd
  );
}

export function intersectViewportWithSectionWindow(
  section: SectionPayload,
  viewport: SectionViewport
): SectionViewport | null {
  const window = resolveLoadedSectionWindow(section);
  const traceStart = Math.max(viewport.traceStart, window.traceStart);
  const traceEnd = Math.min(viewport.traceEnd, window.traceEnd);
  const sampleStart = Math.max(viewport.sampleStart, window.sampleStart);
  const sampleEnd = Math.min(viewport.sampleEnd, window.sampleEnd);
  if (traceEnd <= traceStart || sampleEnd <= sampleStart) {
    return null;
  }
  return { traceStart, traceEnd, sampleStart, sampleEnd };
}

export function toLoadedSectionViewport(
  section: SectionPayload,
  viewport: SectionViewport
): SectionViewport | null {
  const visible = intersectViewportWithSectionWindow(section, viewport);
  if (!visible) {
    return null;
  }
  const window = resolveLoadedSectionWindow(section);
  return {
    traceStart: visible.traceStart - window.traceStart,
    traceEnd: visible.traceEnd - window.traceStart,
    sampleStart: visible.sampleStart - window.sampleStart,
    sampleEnd: visible.sampleEnd - window.sampleStart
  };
}

export function globalTraceToLocalIndex(section: SectionPayload, traceIndex: number): number | null {
  const window = resolveLoadedSectionWindow(section);
  if (traceIndex < window.traceStart || traceIndex >= window.traceEnd) {
    return null;
  }
  return traceIndex - window.traceStart;
}

export function globalSampleToLocalIndex(section: SectionPayload, sampleIndex: number): number | null {
  const window = resolveLoadedSectionWindow(section);
  if (sampleIndex < window.sampleStart || sampleIndex >= window.sampleEnd) {
    return null;
  }
  return sampleIndex - window.sampleStart;
}

export function sectionHorizontalCoordinateAt(section: SectionPayload, traceIndex: number): number | null {
  const local = globalTraceToLocalIndex(section, traceIndex);
  return local === null ? null : section.horizontalAxis[local] ?? null;
}

export function sectionInlineCoordinateAt(section: SectionPayload, traceIndex: number): number | null {
  const local = globalTraceToLocalIndex(section, traceIndex);
  return local === null ? null : section.inlineAxis?.[local] ?? null;
}

export function sectionXlineCoordinateAt(section: SectionPayload, traceIndex: number): number | null {
  const local = globalTraceToLocalIndex(section, traceIndex);
  return local === null ? null : section.xlineAxis?.[local] ?? null;
}

export function sectionSampleValueAt(section: SectionPayload, sampleIndex: number): number | null {
  const local = globalSampleToLocalIndex(section, sampleIndex);
  return local === null ? null : section.sampleAxis[local] ?? null;
}

export function sectionAmplitudeAt(
  section: SectionPayload,
  traceIndex: number,
  sampleIndex: number
): number | null {
  const localTrace = globalTraceToLocalIndex(section, traceIndex);
  const localSample = globalSampleToLocalIndex(section, sampleIndex);
  if (localTrace === null || localSample === null) {
    return null;
  }
  return section.amplitudes[localTrace * section.dimensions.samples + localSample] ?? null;
}
