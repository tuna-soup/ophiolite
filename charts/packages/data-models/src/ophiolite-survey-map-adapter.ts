import type { SurveyMapModel, SurveyMapPoint, SurveyMapScalarField, SurveyMapSurveyArea, SurveyMapWell } from "./survey-map";

export interface OphioliteResolvedSurveyMapPointDto {
  x: number;
  y: number;
}

export interface OphioliteResolvedSurveyOutlineDto {
  id: string;
  name: string;
  outline: OphioliteResolvedSurveyMapPointDto[];
  stroke?: string;
  fill?: string;
}

export interface OphioliteResolvedSurveyMapWellDto {
  well_id: string;
  wellbore_id?: string;
  name: string;
  surface_location?: OphioliteResolvedSurveyMapPointDto;
  surface_position?: OphioliteResolvedSurveyMapPointDto;
  plan_trajectory?: OphioliteResolvedSurveyMapPointDto[];
  color?: string;
}

export interface OphioliteResolvedSurveyScalarFieldDto {
  id: string;
  name: string;
  columns: number;
  rows: number;
  values: ArrayLike<number>;
  origin: OphioliteResolvedSurveyMapPointDto;
  step: OphioliteResolvedSurveyMapPointDto;
  unit?: string;
  min_value?: number;
  max_value?: number;
}

export interface OphioliteResolvedSurveyMapSource {
  id: string;
  name: string;
  x_label?: string;
  y_label?: string;
  coordinate_unit?: string;
  background?: string;
  surveys: OphioliteResolvedSurveyOutlineDto[];
  wells: OphioliteResolvedSurveyMapWellDto[];
  scalar_field?: OphioliteResolvedSurveyScalarFieldDto | null;
}

export function adaptOphioliteSurveyMapToChart(source: OphioliteResolvedSurveyMapSource): SurveyMapModel {
  return {
    id: source.id,
    name: source.name,
    xLabel: source.x_label,
    yLabel: source.y_label,
    coordinateUnit: source.coordinate_unit,
    background: source.background,
    surveys: source.surveys.map(adaptSurvey),
    wells: source.wells.map(adaptWell),
    scalarField: source.scalar_field ? adaptScalarField(source.scalar_field) : null
  };
}

function adaptSurvey(source: OphioliteResolvedSurveyOutlineDto): SurveyMapSurveyArea {
  return {
    id: source.id,
    name: source.name,
    outline: source.outline.map(adaptPoint),
    stroke: source.stroke,
    fill: source.fill
  };
}

function adaptWell(source: OphioliteResolvedSurveyMapWellDto): SurveyMapWell {
  const surface = source.surface_location ?? source.surface_position;
  if (!surface) {
    throw new Error(`Survey-map well '${source.well_id}' is missing a resolved surface location`);
  }
  return {
    id: source.well_id,
    wellboreId: source.wellbore_id,
    name: source.name,
    surface: adaptPoint(surface),
    trajectory: source.plan_trajectory?.map(adaptPoint),
    color: source.color
  };
}

function adaptScalarField(source: OphioliteResolvedSurveyScalarFieldDto): SurveyMapScalarField {
  return {
    id: source.id,
    name: source.name,
    columns: source.columns,
    rows: source.rows,
    values: Float32Array.from(toNumberArray(source.values)),
    origin: adaptPoint(source.origin),
    step: adaptPoint(source.step),
    unit: source.unit,
    minValue: source.min_value,
    maxValue: source.max_value
  };
}

function adaptPoint(source: OphioliteResolvedSurveyMapPointDto): SurveyMapPoint {
  return { x: source.x, y: source.y };
}

function toNumberArray(values: ArrayLike<number>): number[] {
  const next = new Array<number>(values.length);
  for (let index = 0; index < values.length; index += 1) {
    next[index] = values[index] ?? 0;
  }
  return next;
}
