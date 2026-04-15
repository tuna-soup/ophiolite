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

export interface SurveyMapValidationIssue {
  code: string;
  path: string;
  message: string;
}

export class OphioliteSurveyMapValidationError extends Error {
  readonly issues: SurveyMapValidationIssue[];

  constructor(issues: SurveyMapValidationIssue[]) {
    super([
      "Survey-map source validation failed.",
      ...issues.map((entry) => `- [${entry.code}] ${entry.path}: ${entry.message}`)
    ].join("\n"));
    this.name = "OphioliteSurveyMapValidationError";
    this.issues = issues;
  }
}

export function validateOphioliteSurveyMapSource(source: OphioliteResolvedSurveyMapSource): SurveyMapValidationIssue[] {
  const issues: SurveyMapValidationIssue[] = [];
  const surveyIds = new Set<string>();
  const wellIds = new Set<string>();

  source.surveys.forEach((survey, index) => {
    if (surveyIds.has(survey.id)) {
      issues.push(issue("duplicate-survey", `surveys[${index}].id`, `Duplicate survey id '${survey.id}'.`));
    }
    surveyIds.add(survey.id);
    if (survey.outline.length < 3) {
      issues.push(
        issue("invalid-survey-outline", `surveys[${index}].outline`, `Survey '${survey.id}' requires at least three outline points.`)
      );
    }
    survey.outline.forEach((point, pointIndex) => {
      validatePoint(point, `surveys[${index}].outline[${pointIndex}]`, issues);
    });
  });

  source.wells.forEach((well, index) => {
    if (wellIds.has(well.well_id)) {
      issues.push(issue("duplicate-well", `wells[${index}].well_id`, `Duplicate well id '${well.well_id}'.`));
    }
    wellIds.add(well.well_id);
    const surface = well.surface_location ?? well.surface_position;
    if (!surface) {
      issues.push(
        issue("missing-surface", `wells[${index}]`, `Well '${well.well_id}' is missing a resolved surface location.`)
      );
    } else {
      validatePoint(surface, `wells[${index}].surface_location`, issues);
    }
    well.plan_trajectory?.forEach((point, pointIndex) => {
      validatePoint(point, `wells[${index}].plan_trajectory[${pointIndex}]`, issues);
    });
  });

  if (source.scalar_field) {
    const field = source.scalar_field;
    if (field.columns <= 0 || field.rows <= 0) {
      issues.push(
        issue("invalid-scalar-grid", "scalar_field", "Scalar field columns and rows must both be greater than zero.")
      );
    }
    if (field.values.length !== field.columns * field.rows) {
      issues.push(
        issue(
          "invalid-scalar-grid-shape",
          "scalar_field.values",
          `Scalar field value count ${field.values.length} does not match ${field.columns}x${field.rows}.`
        )
      );
    }
    validatePoint(field.origin, "scalar_field.origin", issues);
    validatePoint(field.step, "scalar_field.step", issues);
    for (let index = 0; index < field.values.length; index += 1) {
      if (!Number.isFinite(field.values[index])) {
        issues.push(
          issue("invalid-scalar-value", `scalar_field.values[${index}]`, `Scalar field value ${index} must be finite.`)
        );
        break;
      }
    }
  }

  return issues;
}

export function adaptOphioliteSurveyMapToChart(source: OphioliteResolvedSurveyMapSource): SurveyMapModel {
  const issues = validateOphioliteSurveyMapSource(source);
  if (issues.length > 0) {
    throw new OphioliteSurveyMapValidationError(issues);
  }

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

function validatePoint(
  point: OphioliteResolvedSurveyMapPointDto,
  path: string,
  issues: SurveyMapValidationIssue[]
): void {
  if (!Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    issues.push(issue("invalid-point", path, `Point '${path}' must have finite x and y coordinates.`));
  }
}

function issue(code: string, path: string, message: string): SurveyMapValidationIssue {
  return { code, path, message };
}
