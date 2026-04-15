export type DepthDatum = "md" | "tvd" | "tvdss";

export interface DepthMappingSample {
  nativeDepth: number;
  panelDepth: number;
}

export interface TrackAxis {
  min: number;
  max: number;
  label: string;
  unit?: string;
  tickCount?: number;
  scale?: "linear" | "log";
}

export interface CurveFillStop {
  offset: number;
  color: string;
}

export interface CurveFillStyle {
  direction: "left" | "right";
  baseline: number;
  color: string;
  gradientStops?: CurveFillStop[];
}

export interface CurveSeries {
  id: string;
  name: string;
  color: string;
  values: Float32Array;
  nativeDepths: Float32Array;
  lineWidth?: number;
  axis?: TrackAxis;
  fill?: CurveFillStyle;
}

export interface ReferenceTrack {
  kind: "reference";
  id: string;
  title: string;
  width: number;
}

export interface CurveTrack {
  kind: "curve";
  id: string;
  title: string;
  width: number;
  xAxis: TrackAxis;
  series: [CurveSeries];
}

export interface MultiCurveTrack {
  kind: "multi-curve";
  id: string;
  title: string;
  width: number;
  xAxis: TrackAxis;
  series: CurveSeries[];
  crossoverFill?: {
    leftSeriesId: string;
    rightSeriesId: string;
    color: string;
    fillWhen: "rightOf";
  };
}

export interface FilledCurveTrack {
  kind: "filled-curve";
  id: string;
  title: string;
  width: number;
  xAxis: TrackAxis;
  series: [CurveSeries];
  fill: CurveFillStyle;
}

export interface LithologyComponent {
  id: string;
  name: string;
  color: string;
  values: Float32Array;
}

export interface LithologyTrack {
  kind: "lithology";
  id: string;
  title: string;
  width: number;
  xAxis: TrackAxis;
  nativeDepths: Float32Array;
  components: LithologyComponent[];
}

export interface TopsTrack {
  kind: "tops";
  id: string;
  title: string;
  width: number;
}

export type WellTrack =
  | ReferenceTrack
  | CurveTrack
  | MultiCurveTrack
  | FilledCurveTrack
  | LithologyTrack
  | TopsTrack;

export interface WellTop {
  id: string;
  name: string;
  nativeDepth: number;
  color: string;
  source: "picked" | "imported";
}

export interface WellColumn {
  id: string;
  name: string;
  nativeDepthDatum: DepthDatum;
  panelDepthMapping: DepthMappingSample[];
  tracks: WellTrack[];
  tops: WellTop[];
  headerNote?: string;
}

export interface DepthDomain {
  start: number;
  end: number;
  unit: string;
  label: string;
}

export interface WellCorrelationPanelModel {
  id: string;
  name: string;
  depthDomain: DepthDomain;
  wells: WellColumn[];
  background?: string;
}

export interface WellCorrelationViewport {
  depthStart: number;
  depthEnd: number;
}

export interface CorrelationMarkerLink {
  name: string;
  color: string;
  points: Array<{
    wellId: string;
    nativeDepth: number;
    panelDepth: number;
  }>;
}

export interface WellCorrelationProbe {
  wellId: string;
  wellName: string;
  trackId: string;
  trackTitle: string;
  panelDepth: number;
  nativeDepth: number;
  kind?: "reference" | "curve-sample" | "point-observation" | "top-marker" | "seismic-trace-sample" | "seismic-section-sample";
  seriesName?: string;
  value?: number;
  markerName?: string;
  entityId?: string;
  traceIndex?: number;
  sampleIndex?: number;
  screenX: number;
  screenY: number;
}

export interface DepthBinnedPoint {
  depth: number;
  minValue: number;
  maxValue: number;
}
