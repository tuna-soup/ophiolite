import type { InteractionCapabilities } from "./interactions";
import type { DisplayTransform, RenderMode, SectionPayload } from "./seismic";
import type { CorrelationMarkerLink, CurveFillStop, DepthDatum, DepthDomain, DepthMappingSample, TrackAxis } from "./well-correlation";

export type WellPanelDataFamily =
  | "log-curve"
  | "point-observation"
  | "top-set"
  | "trajectory"
  | "seismic-trace"
  | "seismic-section";

export type WellPanelTrackKind = "reference" | "scalar" | "seismic-trace" | "seismic-section";

export type WellPanelLayerKind = "curve" | "point-observation" | "top-overlay" | "seismic-trace" | "seismic-section";

export type PointObservationFamily = "pressure-observation" | "drilling-observation" | (string & {});

export type PointSymbolShape = "circle" | "square" | "diamond" | "triangle" | "cross" | "x";

export type CurveFillMode = "baseline" | "between-curves";

export type SeismicTraceNormalization = "shared-domain" | "per-trace";

export type WellPanelInteractionTargetKind =
  | "curve-sample"
  | "curve-fill-region"
  | "point-observation"
  | "top-marker"
  | "top-line"
  | "seismic-trace-sample"
  | "seismic-trace-event"
  | "seismic-section-sample"
  | "seismic-section-horizon-anchor";

export interface LayerRuntimeState {
  visible?: boolean;
  opacity?: number;
  zIndex?: number;
}

export interface LayerInteractionBinding {
  targets: WellPanelInteractionTargetKind[];
  capabilities?: Partial<InteractionCapabilities>;
  editable?: boolean;
  selectable?: boolean;
}

export interface WellPanelCurveFillStyle {
  mode: CurveFillMode;
  color: string;
  gradientStops?: CurveFillStop[];
  baseline?: number;
  targetCurveId?: string;
  fillWhen?: "leftOf" | "rightOf" | "greaterThan" | "lessThan";
  visible?: boolean;
  opacity?: number;
}

export interface CurveStyle {
  color: string;
  lineWidth?: number;
  lineDash?: number[];
  fill?: WellPanelCurveFillStyle;
}

export interface PointSymbolStyle {
  shape: PointSymbolShape;
  size: number;
  fillColor: string;
  strokeColor?: string;
  strokeWidth?: number;
}

export interface TopOverlayStyle {
  color: string;
  lineWidth?: number;
  labelColor?: string;
  showLabels?: boolean;
  editable?: boolean;
}

export interface SeismicTraceStyle {
  positiveFill: string;
  negativeFill?: string;
  lineColor?: string;
  lineWidth?: number;
  fillOpacity?: number;
}

export interface SeismicSectionStyle {
  transform: Partial<DisplayTransform> & {
    renderMode: RenderMode;
  };
}

export interface BaseWellPanelData<TFamily extends WellPanelDataFamily> {
  kind: TFamily;
  id: string;
  name: string;
}

export interface WellPanelCurveData extends BaseWellPanelData<"log-curve"> {
  mnemonic?: string;
  nativeDepths: Float32Array;
  values: Float32Array;
  unit?: string;
  axis?: TrackAxis;
  semantics?: string;
  metadata?: Record<string, string | number | boolean | null>;
}

export interface PointObservationSample {
  id: string;
  nativeDepth: number;
  value: number;
  label?: string;
  metadata?: Record<string, string | number | boolean | null>;
}

export interface WellPanelPointObservationData extends BaseWellPanelData<"point-observation"> {
  family: PointObservationFamily;
  unit?: string;
  axis?: TrackAxis;
  points: PointObservationSample[];
}

export interface WellPanelTop {
  id: string;
  name: string;
  nativeDepth: number;
  color: string;
  source: "picked" | "imported";
  metadata?: Record<string, string | number | boolean | null>;
}

export interface WellPanelTopSetData extends BaseWellPanelData<"top-set"> {
  tops: WellPanelTop[];
}

export interface TrajectoryStation {
  id: string;
  md: number;
  tvd?: number;
  tvdss?: number;
  inclination?: number;
  azimuth?: number;
  northing?: number;
  easting?: number;
}

export interface WellPanelTrajectoryData extends BaseWellPanelData<"trajectory"> {
  stations: TrajectoryStation[];
}

export interface SeismicTraceSeries {
  id: string;
  name: string;
  amplitudes: Float32Array;
  metadata?: Record<string, string | number | boolean | null>;
}

export interface WellPanelSeismicTraceData extends BaseWellPanelData<"seismic-trace"> {
  nativeDepths: Float32Array;
  panelDepths?: Float32Array;
  traces: SeismicTraceSeries[];
  amplitudeUnit?: string;
}

export interface WellPanelSeismicSectionData extends BaseWellPanelData<"seismic-section"> {
  section: SectionPayload;
  panelDepths: Float32Array;
  nativeDepths?: Float32Array;
  wellTraceIndex?: number;
}

export type WellPanelDataSource =
  | WellPanelCurveData
  | WellPanelPointObservationData
  | WellPanelTopSetData
  | WellPanelTrajectoryData
  | WellPanelSeismicTraceData
  | WellPanelSeismicSectionData;

export interface WellPanelDataCatalog {
  curves?: WellPanelCurveData[];
  pointObservations?: WellPanelPointObservationData[];
  topSets?: WellPanelTopSetData[];
  trajectories?: WellPanelTrajectoryData[];
  seismicTraces?: WellPanelSeismicTraceData[];
  seismicSections?: WellPanelSeismicSectionData[];
}

export interface BaseTrackLayer<TKind extends WellPanelLayerKind> extends LayerRuntimeState {
  kind: TKind;
  id: string;
  name?: string;
  interaction?: LayerInteractionBinding;
}

export interface CurveLayer extends BaseTrackLayer<"curve"> {
  dataId: string;
  style: CurveStyle;
}

export interface PointObservationLayer extends BaseTrackLayer<"point-observation"> {
  dataId: string;
  style: PointSymbolStyle;
}

export interface TopOverlayLayer extends BaseTrackLayer<"top-overlay"> {
  dataId: string;
  style: TopOverlayStyle;
}

export interface SeismicTraceLayer extends BaseTrackLayer<"seismic-trace"> {
  dataId: string;
  traceIds?: string[];
  normalization?: SeismicTraceNormalization;
  styleByTraceId?: Record<string, SeismicTraceStyle>;
}

export interface SeismicSectionLayer extends BaseTrackLayer<"seismic-section"> {
  dataId: string;
  style: SeismicSectionStyle;
}

export interface BaseWellPanelTrack<TKind extends WellPanelTrackKind> {
  kind: TKind;
  id: string;
  title: string;
  width: number;
  interactions?: Partial<InteractionCapabilities>;
}

export interface WellPanelReferenceTrack extends BaseWellPanelTrack<"reference"> {
  layers?: TopOverlayLayer[];
}

export interface WellPanelScalarTrack extends BaseWellPanelTrack<"scalar"> {
  xAxis: TrackAxis;
  layers: Array<CurveLayer | PointObservationLayer | TopOverlayLayer>;
}

export interface WellPanelSeismicTraceTrack extends BaseWellPanelTrack<"seismic-trace"> {
  layers: Array<SeismicTraceLayer | TopOverlayLayer>;
}

export interface WellPanelSeismicSectionTrack extends BaseWellPanelTrack<"seismic-section"> {
  layers: Array<SeismicSectionLayer | TopOverlayLayer>;
}

export type WellPanelTrack =
  | WellPanelReferenceTrack
  | WellPanelScalarTrack
  | WellPanelSeismicTraceTrack
  | WellPanelSeismicSectionTrack;

export interface WellPanelWellColumn {
  id: string;
  name: string;
  nativeDepthDatum: DepthDatum;
  panelDepthMapping: DepthMappingSample[];
  data: WellPanelDataCatalog;
  tracks: WellPanelTrack[];
  headerNote?: string;
}

export interface WellPanelModel {
  id: string;
  name: string;
  depthDomain: DepthDomain;
  wells: WellPanelWellColumn[];
  markers?: CorrelationMarkerLink[];
  background?: string;
}
