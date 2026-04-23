import { getRockPhysicsTemplateSpec } from "@ophiolite/charts-data-models";
import type {
  RockPhysicsAxisDirection,
  RockPhysicsCrossplotModel,
  RockPhysicsPointSymbol,
  RockPhysicsTemplateId
} from "@ophiolite/charts-data-models";
import type {
  RockPhysicsCrossplotData,
  RockPhysicsCrossplotSimpleData,
  RockPhysicsSimpleGroup,
  RockPhysicsSimplePoint
} from "./types";

const rockPhysicsCache = new WeakMap<object, RockPhysicsCrossplotModel>();
const DEFAULT_GROUP_COLORS = ["#0f766e", "#b45309", "#1d4ed8", "#7c3aed", "#be123c", "#0f766e"];

export function adaptRockPhysicsCrossplotInputToModel(
  input: RockPhysicsCrossplotData | null
): RockPhysicsCrossplotModel | null {
  if (!input) {
    return null;
  }
  if (isRockPhysicsCrossplotModel(input)) {
    return input;
  }

  const cached = rockPhysicsCache.get(input);
  if (cached) {
    return cached;
  }

  const spec = getRockPhysicsTemplateSpec(input.templateId);
  const groups = resolveGroups(input);
  const categories = groups.map((group, index) => ({
    id: index,
    label: group.name,
    color: group.color ?? DEFAULT_GROUP_COLORS[index % DEFAULT_GROUP_COLORS.length]!,
    symbol: group.symbol ?? "circle"
  }));
  const groupIndexByKey = new Map(groups.map((group, index) => [group.id, index]));

  const xValues = new Float32Array(input.points.length);
  const yValues = new Float32Array(input.points.length);
  const colorCategoryIds = new Uint16Array(input.points.length);
  const symbolCategoryIds = new Uint16Array(input.points.length);
  const wellIndices = new Uint16Array(input.points.length);
  const sampleDepthsM = new Float32Array(input.points.length);

  input.points.forEach((point, index) => {
    const groupId = resolvePointGroupId(point);
    const categoryIndex = groupIndexByKey.get(groupId) ?? 0;
    xValues[index] = point.x;
    yValues[index] = point.y;
    colorCategoryIds[index] = categoryIndex;
    symbolCategoryIds[index] = categoryIndex;
    wellIndices[index] = categoryIndex;
    sampleDepthsM[index] = point.depthM ?? index;
  });

  const xRange = resolveAxisRange(
    xValues,
    input.xAxis?.min,
    input.xAxis?.max
  );
  const yRange = resolveAxisRange(
    yValues,
    input.yAxis?.min,
    input.yAxis?.max
  );

  const normalized: RockPhysicsCrossplotModel = {
    id: input.id ?? slugify(input.title || input.name || spec.title, "rock-physics"),
    name: input.name ?? input.title ?? spec.title,
    templateId: input.templateId,
    title: input.title ?? spec.title,
    subtitle: input.subtitle,
    pointCount: input.points.length,
    xAxis: {
      label: input.xAxis?.label ?? spec.xLabel,
      unit: input.xAxis?.unit ?? spec.xUnit,
      semantic: spec.xSemantics[0]!,
      range: xRange,
      direction: input.xAxis?.direction ?? spec.xDirection
    },
    yAxis: {
      label: input.yAxis?.label ?? spec.yLabel,
      unit: input.yAxis?.unit ?? spec.yUnit,
      semantic: spec.ySemantics[0]!,
      range: yRange,
      direction: input.yAxis?.direction ?? spec.yDirection
    },
    colorBinding: {
      kind: "categorical",
      label: input.groupLabel ?? "Group",
      semantic: "well",
      categories
    },
    columns: {
      x: xValues,
      y: yValues,
      colorCategoryIds,
      symbolCategoryIds,
      wellIndices,
      sampleDepthsM
    },
    wells: groups.map((group, index) => ({
      id: group.wellId ?? group.id,
      wellboreId: group.wellboreId ?? group.wellId ?? group.id,
      name: group.name,
      color: categories[index]!.color
    })),
    sourceBindings: groups.map((group) => ({
      wellId: group.wellId ?? group.id,
      wellboreId: group.wellboreId ?? group.wellId ?? group.id,
      xCurveId: `${group.id}:x`,
      yCurveId: `${group.id}:y`
    })),
    templateLines: spec.templateLines ? [...spec.templateLines] : undefined,
    templateOverlays: spec.templateOverlays ? [...spec.templateOverlays] : undefined
  };

  rockPhysicsCache.set(input, normalized);
  return normalized;
}

function isRockPhysicsCrossplotModel(input: RockPhysicsCrossplotData): input is RockPhysicsCrossplotModel {
  return "columns" in input;
}

function resolveGroups(input: RockPhysicsCrossplotSimpleData): Array<Required<RockPhysicsSimpleGroup>> {
  const groups = new Map<string, Required<RockPhysicsSimpleGroup>>();

  input.groups?.forEach((group, index) => {
    const id = group.id ?? slugify(group.name, `group-${index + 1}`);
    groups.set(id, {
      id,
      name: group.name,
      color: group.color ?? DEFAULT_GROUP_COLORS[index % DEFAULT_GROUP_COLORS.length]!,
      symbol: group.symbol ?? "circle",
      wellId: group.wellId ?? id,
      wellboreId: group.wellboreId ?? group.wellId ?? id
    });
  });

  input.points.forEach((point, index) => {
    const id = resolvePointGroupId(point);
    if (!groups.has(id)) {
      groups.set(id, {
        id,
        name: resolvePointGroupName(point, index),
        color: point.color ?? DEFAULT_GROUP_COLORS[groups.size % DEFAULT_GROUP_COLORS.length]!,
        symbol: "circle",
        wellId: point.wellId ?? id,
        wellboreId: point.wellboreId ?? point.wellId ?? id
      });
    }
  });

  if (groups.size === 0) {
    groups.set("samples", {
      id: "samples",
      name: "Samples",
      color: DEFAULT_GROUP_COLORS[0]!,
      symbol: "circle",
      wellId: "samples",
      wellboreId: "samples"
    });
  }

  return [...groups.values()];
}

function resolvePointGroupId(point: RockPhysicsSimplePoint): string {
  return point.groupId ?? slugify(point.group ?? point.wellId ?? point.wellboreId ?? "samples", "samples");
}

function resolvePointGroupName(point: RockPhysicsSimplePoint, index: number): string {
  return point.group ?? point.wellId ?? point.wellboreId ?? `Group ${index + 1}`;
}

function resolveAxisRange(
  values: Float32Array,
  minOverride: number | undefined,
  maxOverride: number | undefined
): { min: number; max: number } {
  if (minOverride !== undefined && maxOverride !== undefined) {
    return {
      min: minOverride,
      max: maxOverride
    };
  }

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (!Number.isFinite(value)) {
      continue;
    }
    min = Math.min(min, value);
    max = Math.max(max, value);
  }

  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return {
      min: minOverride ?? 0,
      max: maxOverride ?? 1
    };
  }

  const span = Math.max(1e-6, max - min);
  const pad = span * 0.06;
  return {
    min: minOverride ?? min - pad,
    max: maxOverride ?? max + pad
  };
}

function slugify(value: string, fallback: string): string {
  const normalized = value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || fallback;
}
