import type { SurveyMapModel, SurveyMapPoint, SurveyMapScalarField } from "@ophiolite/charts-data-models";
import type { SurveyMapData, SurveyMapSimpleArea, SurveyMapSimpleData, SurveyMapSimpleWell } from "./types";

const surveyMapCache = new WeakMap<object, SurveyMapModel>();

export function adaptSurveyMapInputToModel(input: SurveyMapData | null): SurveyMapModel | null {
  if (!input) {
    return null;
  }
  if (isSurveyMapModel(input)) {
    return input;
  }

  const cached = surveyMapCache.get(input);
  if (cached) {
    return cached;
  }

  const normalized: SurveyMapModel = {
    id: input.id ?? slugify(input.name, "survey-map"),
    name: input.name,
    xLabel: input.xLabel,
    yLabel: input.yLabel,
    coordinateUnit: input.coordinateUnit,
    background: input.background,
    surveys: (input.areas ?? []).map((area, index) => adaptArea(area, index)),
    wells: (input.wells ?? []).map((well, index) => adaptWell(well, index)),
    scalarField: input.scalarField ? adaptScalarField(input.scalarField) : null
  };

  surveyMapCache.set(input, normalized);
  return normalized;
}

function isSurveyMapModel(input: SurveyMapData): input is SurveyMapModel {
  return "surveys" in input;
}

function adaptArea(area: SurveyMapSimpleArea, index: number): SurveyMapModel["surveys"][number] {
  return {
    id: area.id ?? slugify(area.name, `area-${index + 1}`),
    name: area.name,
    outline: area.points.map(clonePoint),
    stroke: area.stroke,
    fill: area.fill
  };
}

function adaptWell(well: SurveyMapSimpleWell, index: number): SurveyMapModel["wells"][number] {
  return {
    id: well.id ?? slugify(well.name, `well-${index + 1}`),
    wellboreId: well.wellboreId,
    name: well.name,
    surface: clonePoint(well.position),
    trajectory: well.trajectory?.map(clonePoint),
    color: well.color
  };
}

function adaptScalarField(field: NonNullable<SurveyMapSimpleData["scalarField"]>): SurveyMapScalarField {
  return {
    id: field.id ?? slugify(field.name, "scalar-field"),
    name: field.name,
    columns: field.columns,
    rows: field.rows,
    values: Float32Array.from(toNumberArray(field.values)),
    origin: clonePoint(field.origin),
    step: clonePoint(field.step),
    unit: field.unit,
    minValue: field.minValue,
    maxValue: field.maxValue
  };
}

function clonePoint(point: SurveyMapPoint): SurveyMapPoint {
  return {
    x: point.x,
    y: point.y
  };
}

function toNumberArray(values: ArrayLike<number>): number[] {
  return Array.from({ length: values.length }, (_, index) => values[index] ?? 0);
}

function slugify(value: string, fallback: string): string {
  const normalized = value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || fallback;
}
