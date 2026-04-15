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
