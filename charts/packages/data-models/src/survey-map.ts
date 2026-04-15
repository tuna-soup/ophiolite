export interface SurveyMapPoint {
  x: number;
  y: number;
}

export interface SurveyMapViewport {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

export interface SurveyMapScalarField {
  id: string;
  name: string;
  columns: number;
  rows: number;
  values: Float32Array;
  origin: SurveyMapPoint;
  step: SurveyMapPoint;
  unit?: string;
  minValue?: number;
  maxValue?: number;
}

export interface SurveyMapSurveyArea {
  id: string;
  name: string;
  outline: SurveyMapPoint[];
  stroke?: string;
  fill?: string;
}

export interface SurveyMapWell {
  id: string;
  name: string;
  wellboreId?: string;
  surface: SurveyMapPoint;
  trajectory?: SurveyMapPoint[];
  color?: string;
}

export interface SurveyMapModel {
  id: string;
  name: string;
  xLabel?: string;
  yLabel?: string;
  coordinateUnit?: string;
  background?: string;
  surveys: SurveyMapSurveyArea[];
  wells: SurveyMapWell[];
  scalarField?: SurveyMapScalarField | null;
}

export interface SurveyMapProbe {
  x: number;
  y: number;
  scalarValue?: number;
  scalarName?: string;
  wellId?: string;
  wellName?: string;
  surveyId?: string;
  surveyName?: string;
  screenX: number;
  screenY: number;
}

