export type AvoReflectivityModel =
  | "shuey_two_term"
  | "shuey_three_term"
  | "aki_richards"
  | "aki_richards_alt"
  | "fatti"
  | "bortfeld"
  | "hilterman"
  | "approx_zoeppritz_pp"
  | "zoeppritz"
  | "ruger";

export type AvoAnisotropyMode = "isotropic" | "vti" | "hti";

export type AvoCurveStyle = "solid" | "dashed";

export interface AvoAxisModel {
  label?: string;
  unit?: string;
  range?: {
    min: number;
    max: number;
  };
}

export interface AvoCartesianViewport {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

export interface AvoInterfaceDescriptor {
  id: string;
  label: string;
  color: string;
  reservoirLabel?: string;
}

export interface AvoResponseSeriesModel {
  id: string;
  interfaceId: string;
  label: string;
  color: string;
  style: AvoCurveStyle;
  reflectivityModel: AvoReflectivityModel;
  anisotropyMode: AvoAnisotropyMode;
  incidenceAnglesDeg: Float32Array;
  values: Float32Array;
}

export interface AvoResponseModel {
  id: string;
  name: string;
  title: string;
  subtitle?: string;
  xAxis: AvoAxisModel;
  yAxis: AvoAxisModel;
  interfaces: AvoInterfaceDescriptor[];
  series: AvoResponseSeriesModel[];
}

export interface AvoResponseProbeSeriesValue {
  seriesId: string;
  interfaceId: string;
  label: string;
  color: string;
  value: number;
}

export interface AvoResponseProbe {
  angleDeg: number;
  screenX: number;
  screenY: number;
  seriesValues: AvoResponseProbeSeriesValue[];
}

export interface AvoReferenceLineModel {
  id: string;
  label?: string;
  color: string;
  style: AvoCurveStyle;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export interface AvoBackgroundRegionModel {
  id: string;
  label?: string;
  fillColor: string;
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

export interface AvoCrossplotModel {
  id: string;
  name: string;
  title: string;
  subtitle?: string;
  xAxis: AvoAxisModel;
  yAxis: AvoAxisModel;
  pointCount: number;
  interfaces: AvoInterfaceDescriptor[];
  columns: {
    intercept: Float32Array;
    gradient: Float32Array;
    interfaceIndices: Uint16Array;
    chiProjection?: Float32Array;
    simulationIds?: Uint32Array;
  };
  referenceLines: AvoReferenceLineModel[];
  backgroundRegions: AvoBackgroundRegionModel[];
}

export interface AvoCrossplotProbe {
  pointIndex: number;
  interfaceId: string;
  interfaceLabel: string;
  intercept: number;
  gradient: number;
  chiProjection?: number;
  simulationId?: number;
  screenX: number;
  screenY: number;
}

export interface AvoChiProjectionSeriesModel {
  id: string;
  interfaceId: string;
  label: string;
  color: string;
  projectedValues: Float32Array;
  meanValue?: number;
}

export interface AvoChiProjectionModel {
  id: string;
  name: string;
  title: string;
  subtitle?: string;
  chiAngleDeg: number;
  projectionLabel?: string;
  xAxis: AvoAxisModel;
  interfaces: AvoInterfaceDescriptor[];
  series: AvoChiProjectionSeriesModel[];
  preferredBinCount?: number;
}

export interface AvoHistogramProbeSeriesValue {
  seriesId: string;
  interfaceId: string;
  label: string;
  color: string;
  count: number;
}

export interface AvoHistogramProbe {
  xValue: number;
  binStart: number;
  binEnd: number;
  screenX: number;
  screenY: number;
  seriesValues: AvoHistogramProbeSeriesValue[];
}
