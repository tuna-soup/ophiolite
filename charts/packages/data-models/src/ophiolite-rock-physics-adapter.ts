import {
  ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION,
  type ResolvedRockPhysicsCrossplotSourceDto,
  type RockPhysicsAxisDto,
  type RockPhysicsCategoricalColorBindingDto,
  type RockPhysicsCategoryDto,
  type RockPhysicsColorBindingDto,
  type RockPhysicsContinuousColorBindingDto,
  type RockPhysicsSampleDto,
  type RockPhysicsSourceBindingDto,
  type RockPhysicsTemplateLineDto,
  type RockPhysicsTemplateOverlayDto,
  type RockPhysicsWellDto
} from "@ophiolite/contracts";
import type {
  RockPhysicsCategoricalSemantic,
  RockPhysicsColorBinding,
  RockPhysicsCurveSemantic,
  RockPhysicsCrossplotModel,
  RockPhysicsTemplateLine,
  RockPhysicsTemplateOverlay,
  RockPhysicsWellDescriptor
} from "./rock-physics-crossplot";
import { ROCK_PHYSICS_TEMPLATE_SPECS } from "./rock-physics-template-catalog";
import type { RockPhysicsTemplateSpec } from "./rock-physics-template-catalog";

const DEFAULT_WELL_PALETTE = [
  "#d94841",
  "#2563eb",
  "#eab308",
  "#0f9d58",
  "#8b5cf6",
  "#0ea5e9",
  "#ef6c00",
  "#7c3aed"
] as const;

const DEFAULT_CONTINUOUS_PALETTE = ["#1d4ed8", "#0ea5e9", "#67e8f9", "#facc15", "#f97316", "#dc2626"] as const;

export type OphioliteResolvedRockPhysicsAxisDto = RockPhysicsAxisDto;
export type OphioliteResolvedRockPhysicsCategoryDto = RockPhysicsCategoryDto;
export type OphioliteResolvedRockPhysicsCategoricalColorBindingDto = RockPhysicsCategoricalColorBindingDto;
export type OphioliteResolvedRockPhysicsContinuousColorBindingDto = RockPhysicsContinuousColorBindingDto;
export type OphioliteResolvedRockPhysicsColorBindingDto = RockPhysicsColorBindingDto;
export type OphioliteResolvedRockPhysicsWellDto = RockPhysicsWellDto;
export type OphioliteResolvedRockPhysicsSourceBindingDto = RockPhysicsSourceBindingDto;
export type OphioliteResolvedRockPhysicsSampleDto = RockPhysicsSampleDto;
export type OphioliteResolvedRockPhysicsCrossplotSource = ResolvedRockPhysicsCrossplotSourceDto;

export interface RockPhysicsValidationIssue {
  code: string;
  path: string;
  message: string;
}

export class OphioliteRockPhysicsValidationError extends Error {
  readonly issues: RockPhysicsValidationIssue[];

  constructor(issues: RockPhysicsValidationIssue[]) {
    super(formatValidationMessage(issues));
    this.name = "OphioliteRockPhysicsValidationError";
    this.issues = issues;
  }
}

export function validateOphioliteRockPhysicsCrossplotSource(
  source: OphioliteResolvedRockPhysicsCrossplotSource
): RockPhysicsValidationIssue[] {
  const issues: RockPhysicsValidationIssue[] = [];
  const template = ROCK_PHYSICS_TEMPLATE_SPECS[source.template_id];
  const wellsById = new Map<string, OphioliteResolvedRockPhysicsWellDto>();
  const bindingsById = new Map<string, OphioliteResolvedRockPhysicsSourceBindingDto>();
  const bindingsByWellId = new Map<string, OphioliteResolvedRockPhysicsSourceBindingDto[]>();

  if (source.schema_version !== ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION) {
    issues.push(
      issue(
        "unsupported-schema-version",
        "schema_version",
        `Expected rock-physics crossplot schema version ${ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION}, got ${source.schema_version}.`
      )
    );
  }

  if (source.wells.length === 0) {
    issues.push(issue("missing-wells", "wells", "At least one well is required."));
  }
  if (source.samples.length === 0) {
    issues.push(issue("missing-samples", "samples", "At least one materialized sample is required."));
  }

  if (!template.xSemantics.includes(source.x_axis.semantic)) {
    issues.push(
      issue(
        "invalid-x-semantic",
        "x_axis.semantic",
        `Template '${source.template_id}' requires one of x-axis semantics '${template.xSemantics.join(", ")}', got '${source.x_axis.semantic}'.`
      )
    );
  }
  if (!template.ySemantics.includes(source.y_axis.semantic)) {
    issues.push(
      issue(
        "invalid-y-semantic",
        "y_axis.semantic",
        `Template '${source.template_id}' requires one of y-axis semantics '${template.ySemantics.join(", ")}', got '${source.y_axis.semantic}'.`
      )
    );
  }

  validateColorBinding(source, template, issues);

  source.wells.forEach((well, index) => {
    if (wellsById.has(well.well_id)) {
      issues.push(issue("duplicate-well", `wells[${index}].well_id`, `Duplicate well id '${well.well_id}'.`));
      return;
    }
    wellsById.set(well.well_id, well);
  });

  source.source_bindings.forEach((binding, index) => {
    if (bindingsById.has(binding.id)) {
      issues.push(issue("duplicate-source-binding", `source_bindings[${index}].id`, `Duplicate source binding id '${binding.id}'.`));
      return;
    }
    const well = wellsById.get(binding.well_id);
    if (!well) {
      issues.push(
        issue(
          "unknown-binding-well",
          `source_bindings[${index}].well_id`,
          `Source binding '${binding.id}' references unknown well '${binding.well_id}'.`
        )
      );
    } else if (well.wellbore_id !== binding.wellbore_id) {
      issues.push(
        issue(
          "binding-wellbore-mismatch",
          `source_bindings[${index}].wellbore_id`,
          `Source binding '${binding.id}' wellbore '${binding.wellbore_id}' does not match well '${binding.well_id}'.`
        )
      );
    }
    bindingsById.set(binding.id, binding);
    const bindings = bindingsByWellId.get(binding.well_id) ?? [];
    bindings.push(binding);
    bindingsByWellId.set(binding.well_id, bindings);
  });

  const faciesCategoryIds =
    source.color_binding.kind === "categorical" && source.color_binding.semantic === "facies"
      ? new Set((source.color_binding.categories ?? []).map((category) => category.id))
      : null;

  source.samples.forEach((sample, index) => {
    const well = wellsById.get(sample.well_id);
    if (!well) {
      issues.push(
        issue(
          "unknown-sample-well",
          `samples[${index}].well_id`,
          `Sample ${index} references unknown well '${sample.well_id}'.`
        )
      );
    } else if (sample.wellbore_id !== null && sample.wellbore_id !== well.wellbore_id) {
      issues.push(
        issue(
          "sample-wellbore-mismatch",
          `samples[${index}].wellbore_id`,
          `Sample ${index} wellbore '${sample.wellbore_id}' does not match well '${sample.well_id}'.`
        )
      );
    }

    validateFiniteNumber(sample.x_value, `samples[${index}].x_value`, issues);
    validateFiniteNumber(sample.y_value, `samples[${index}].y_value`, issues);
    validateFiniteNumber(sample.sample_depth_m, `samples[${index}].sample_depth_m`, issues);

    if (source.color_binding.kind === "continuous") {
      validateFiniteNumber(sample.color_value, `samples[${index}].color_value`, issues);
    }
    if (
      source.color_binding.kind === "categorical" &&
      source.color_binding.semantic === "facies" &&
      !faciesCategoryIds?.has(sample.color_category_id ?? Number.NaN)
    ) {
      issues.push(
        issue(
          "unknown-facies-category",
          `samples[${index}].color_category_id`,
          `Sample ${index} references an unknown facies category '${sample.color_category_id ?? "undefined"}'.`
        )
      );
    }

    if (sample.source_binding_id) {
      const binding = bindingsById.get(sample.source_binding_id);
      if (!binding) {
        issues.push(
          issue(
            "unknown-source-binding",
            `samples[${index}].source_binding_id`,
            `Sample ${index} references unknown source binding '${sample.source_binding_id}'.`
          )
        );
      } else if (binding.well_id !== sample.well_id) {
        issues.push(
          issue(
            "source-binding-well-mismatch",
            `samples[${index}].source_binding_id`,
            `Sample ${index} binding '${sample.source_binding_id}' does not belong to well '${sample.well_id}'.`
          )
        );
      }
    } else if (source.source_bindings.length > 0) {
      const candidateBindings = bindingsByWellId.get(sample.well_id) ?? [];
      if (candidateBindings.length > 1) {
        issues.push(
          issue(
            "ambiguous-source-binding",
            `samples[${index}].source_binding_id`,
            `Sample ${index} requires an explicit source binding because well '${sample.well_id}' has multiple bindings.`
          )
        );
      }
    }
  });

  if (
    source.color_binding.kind === "categorical" &&
    source.color_binding.semantic === "facies" &&
    (source.color_binding.categories?.length ?? 0) === 0
  ) {
    issues.push(
      issue("missing-facies-categories", "color_binding.categories", "Facies color mode requires explicit categories.")
    );
  }

  return issues;
}

export function adaptOphioliteRockPhysicsCrossplotToChart(
  source: OphioliteResolvedRockPhysicsCrossplotSource
): RockPhysicsCrossplotModel {
  const issues = validateOphioliteRockPhysicsCrossplotSource(source);
  if (issues.length > 0) {
    throw new OphioliteRockPhysicsValidationError(issues);
  }

  const template = ROCK_PHYSICS_TEMPLATE_SPECS[source.template_id];
  const wells = adaptWells(source.wells);
  const wellIndexById = new Map(wells.map((well, index) => [well.id, index]));
  const bindings = source.source_bindings.map(adaptSourceBinding);
  const bindingIndexById = new Map(bindings.map((binding, index) => [source.source_bindings[index]!.id, index]));
  const fallbackBindingByWellId = new Map<string, number>();
  const categoricalContext = buildCategoricalContext(source, wells);

  source.source_bindings.forEach((binding, index) => {
    const existing = fallbackBindingByWellId.get(binding.well_id);
    fallbackBindingByWellId.set(binding.well_id, existing === undefined ? index : -1);
  });

  const pointCount = source.samples.length;
  const x = new Float32Array(pointCount);
  const y = new Float32Array(pointCount);
  const wellIndices = new Uint16Array(pointCount);
  const sampleDepthsM = new Float32Array(pointCount);
  const sourceBindingIndices = bindings.length > 0 ? new Uint16Array(pointCount) : undefined;
  const colorScalars = source.color_binding.kind === "continuous" ? new Float32Array(pointCount) : undefined;
  const colorCategoryIds = source.color_binding.kind === "categorical" ? new Uint16Array(pointCount) : undefined;
  const symbolCategoryIds = source.samples.some((sample) => sample.symbol_category_id != null)
    ? new Uint16Array(pointCount)
    : undefined;

  source.samples.forEach((sample, index) => {
    x[index] = sample.x_value;
    y[index] = sample.y_value;
    wellIndices[index] = wellIndexById.get(sample.well_id) ?? 0;
    sampleDepthsM[index] = sample.sample_depth_m;
    if (colorScalars) {
      colorScalars[index] = sample.color_value ?? 0;
    }
    if (colorCategoryIds) {
      colorCategoryIds[index] = resolveColorCategoryId(source, sample, categoricalContext, wellIndices[index]!);
    }
    if (symbolCategoryIds) {
      symbolCategoryIds[index] = sample.symbol_category_id ?? 0;
    }
    if (sourceBindingIndices) {
      sourceBindingIndices[index] = resolveSourceBindingIndex(sample, bindingIndexById, fallbackBindingByWellId);
    }
  });

  return {
    id: source.id,
    name: source.name,
    templateId: source.template_id,
    title: source.title ?? template.title,
    subtitle: source.subtitle ?? undefined,
    pointCount,
    xAxis: {
      label: source.x_axis.label ?? template.xLabel,
      unit: source.x_axis.unit ?? template.xUnit,
      semantic: source.x_axis.semantic,
      range: deriveRange(source.x_axis.min_value, source.x_axis.max_value, x)
    },
    yAxis: {
      label: source.y_axis.label ?? template.yLabel,
      unit: source.y_axis.unit ?? template.yUnit,
      semantic: source.y_axis.semantic,
      range: deriveRange(source.y_axis.min_value, source.y_axis.max_value, y)
    },
    colorBinding: adaptColorBinding(source, wells, colorScalars),
    columns: {
      x,
      y,
      colorScalars,
      colorCategoryIds,
      symbolCategoryIds,
      wellIndices,
      sourceBindingIndices,
      sampleDepthsM
    },
    wells,
    sourceBindings: bindings,
    templateLines:
      normalizeTemplateLines(source.template_lines) ??
      toTemplateLines(normalizeTemplateOverlays(source.template_overlays) ?? template.templateOverlays),
    templateOverlays:
      normalizeTemplateOverlays(source.template_overlays) ??
      toTemplateOverlays(source.template_lines) ??
      cloneTemplateOverlays(template.templateOverlays),
    interactionThresholds: source.interaction_thresholds
      ? {
          exactPointLimit: source.interaction_thresholds.exact_point_limit,
          progressivePointLimit: source.interaction_thresholds.progressive_point_limit
        }
      : undefined
  };
}

function validateColorBinding(
  source: OphioliteResolvedRockPhysicsCrossplotSource,
  template: RockPhysicsTemplateSpec,
  issues: RockPhysicsValidationIssue[]
): void {
  if (source.color_binding.kind === "continuous") {
    if (!template.allowedContinuousColorSemantics.includes(source.color_binding.semantic)) {
      issues.push(
        issue(
          "invalid-continuous-color-semantic",
          "color_binding.semantic",
          `Template '${source.template_id}' does not allow continuous color semantic '${source.color_binding.semantic}'.`
        )
      );
    }
    if ((source.color_binding.palette?.length ?? 0) < 2) {
      if (source.color_binding.palette !== null) {
        issues.push(
          issue(
            "invalid-continuous-palette",
            "color_binding.palette",
            "Continuous color mode requires at least two palette colors."
          )
        );
      }
    }
    return;
  }

  if (!template.allowedCategoricalColorSemantics.includes(source.color_binding.semantic)) {
    issues.push(
      issue(
        "invalid-categorical-color-semantic",
        "color_binding.semantic",
        `Template '${source.template_id}' does not allow categorical color semantic '${source.color_binding.semantic}'.`
      )
    );
  }
}

function adaptWells(wells: OphioliteResolvedRockPhysicsWellDto[]): RockPhysicsWellDescriptor[] {
  return wells.map((well, index) => ({
    id: well.well_id,
    wellboreId: well.wellbore_id,
    name: well.name,
    color: well.color ?? DEFAULT_WELL_PALETTE[index % DEFAULT_WELL_PALETTE.length]!
  }));
}

function adaptSourceBinding(source: OphioliteResolvedRockPhysicsSourceBindingDto) {
  return {
    wellId: source.well_id,
    wellboreId: source.wellbore_id,
    xCurveId: source.x_curve_id,
    yCurveId: source.y_curve_id,
    colorCurveId: source.color_curve_id ?? undefined,
    derivedChannels: source.derived_channels ? [...source.derived_channels] : undefined
  };
}

function adaptColorBinding(
  source: OphioliteResolvedRockPhysicsCrossplotSource,
  wells: RockPhysicsWellDescriptor[],
  colorScalars: Float32Array | undefined
): RockPhysicsColorBinding {
  const categoricalContext = buildCategoricalContext(source, wells);
  if (source.color_binding.kind === "categorical") {
    return {
      kind: "categorical",
      label: source.color_binding.label ?? defaultCategoricalLabel(source.color_binding.semantic),
      semantic: source.color_binding.semantic,
      categories: categoricalContext.categories
    };
  }

  return {
    kind: "continuous",
    label: source.color_binding.label ?? defaultContinuousLabel(source.color_binding.semantic),
    semantic: source.color_binding.semantic,
    range: deriveRange(source.color_binding.min_value, source.color_binding.max_value, colorScalars),
    palette: source.color_binding.palette ? [...source.color_binding.palette] : [...DEFAULT_CONTINUOUS_PALETTE]
  };
}

function buildCategoricalContext(
  source: OphioliteResolvedRockPhysicsCrossplotSource,
  wells: RockPhysicsWellDescriptor[]
): {
  categories: Array<{
    id: number;
    label: string;
    color: string;
    symbol?: "circle" | "square" | "diamond" | "triangle";
  }>;
  categoryIdByWellId: Map<string, number>;
} {
  if (source.color_binding.kind !== "categorical") {
    return {
      categories: [],
      categoryIdByWellId: new Map()
    };
  }

  if (source.color_binding.semantic === "well") {
    return {
      categories: wells.map((well, index) => ({
        id: index,
        label: well.name,
        color: well.color,
        symbol: "circle" as const
      })),
      categoryIdByWellId: new Map(wells.map((well, index) => [well.id, index]))
    };
  }

  if (source.color_binding.semantic === "wellbore") {
    const seen = new Map<string, number>();
    const categories = wells.flatMap((well, index) => {
      const existing = seen.get(well.wellboreId);
      if (existing !== undefined) {
        return [];
      }
      seen.set(well.wellboreId, index);
      return [{
        id: index,
        label: well.wellboreId,
        color: well.color,
        symbol: "circle" as const
      }];
    });
    return {
      categories,
      categoryIdByWellId: new Map(wells.map((well) => [well.id, seen.get(well.wellboreId) ?? 0]))
    };
  }

  return {
    categories: (source.color_binding.categories ?? []).map((category) => ({
      id: category.id,
      label: category.label,
      color: category.color,
      symbol: category.symbol ?? undefined
    })),
    categoryIdByWellId: new Map()
  };
}

function resolveColorCategoryId(
  source: OphioliteResolvedRockPhysicsCrossplotSource,
  sample: OphioliteResolvedRockPhysicsSampleDto,
  categoricalContext: ReturnType<typeof buildCategoricalContext>,
  wellIndex: number
): number {
  if (source.color_binding.kind !== "categorical") {
    return 0;
  }
  if (source.color_binding.semantic === "well" || source.color_binding.semantic === "wellbore") {
    return categoricalContext.categoryIdByWellId.get(sample.well_id) ?? wellIndex;
  }
  return sample.color_category_id ?? 0;
}

function resolveSourceBindingIndex(
  sample: OphioliteResolvedRockPhysicsSampleDto,
  bindingIndexById: Map<string, number>,
  fallbackBindingByWellId: Map<string, number>
): number {
  if (sample.source_binding_id) {
    return bindingIndexById.get(sample.source_binding_id) ?? 0;
  }
  return Math.max(0, fallbackBindingByWellId.get(sample.well_id) ?? 0);
}

function deriveRange(
  explicitMin: number | null | undefined,
  explicitMax: number | null | undefined,
  values: ArrayLike<number> | undefined
) {
  if (Number.isFinite(explicitMin) && Number.isFinite(explicitMax)) {
    return { min: explicitMin as number, max: explicitMax as number };
  }

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  if (values) {
    for (let index = 0; index < values.length; index += 1) {
      const value = values[index];
      if (!Number.isFinite(value)) {
        continue;
      }
      min = Math.min(min, value);
      max = Math.max(max, value);
    }
  }

  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return {
      min: Number.isFinite(explicitMin) ? (explicitMin as number) : 0,
      max: Number.isFinite(explicitMax) ? (explicitMax as number) : 1
    };
  }

  const span = Math.max(1e-6, max - min);
  const pad = span * 0.06;
  return {
    min: Number.isFinite(explicitMin) ? (explicitMin as number) : min - pad,
    max: Number.isFinite(explicitMax) ? (explicitMax as number) : max + pad
  };
}

function validateFiniteNumber(
  value: number | null | undefined,
  path: string,
  issues: RockPhysicsValidationIssue[]
): void {
  if (!Number.isFinite(value)) {
    issues.push(issue("invalid-number", path, `Expected a finite number at '${path}'.`));
  }
}

function defaultCategoricalLabel(semantic: RockPhysicsCategoricalSemantic): string {
  switch (semantic) {
    case "well":
      return "Well";
    case "wellbore":
      return "Wellbore";
    case "facies":
      return "Facies";
  }
}

function defaultContinuousLabel(semantic: RockPhysicsCurveSemantic): string {
  switch (semantic) {
    case "gamma-ray":
      return "Gamma Ray";
    case "water-saturation":
      return "Water Saturation";
    case "v-shale":
      return "V-Shale";
    case "bulk-density":
      return "Bulk Density";
    case "neutron-porosity":
      return "Neutron Porosity";
    case "effective-porosity":
      return "Effective Porosity";
    case "poissons-ratio":
      return "Poisson's Ratio";
    case "lambda-rho":
      return "Lambda-Rho";
    case "mu-rho":
      return "Mu-Rho";
    case "shear-impedance":
      return "Shear Impedance";
    case "p-velocity":
      return "Vp";
    case "s-velocity":
      return "Vs";
    case "vp-vs-ratio":
      return "Vp/Vs";
    case "acoustic-impedance":
      return "Acoustic Impedance";
    case "elastic-impedance":
      return "Elastic Impedance";
    case "extended-elastic-impedance":
      return "Extended Elastic Impedance";
    case "resistivity":
      return "Resistivity";
    default:
      return semantic;
  }
}

function toTemplateOverlays(lines: readonly RockPhysicsTemplateLine[] | null | undefined): RockPhysicsTemplateOverlay[] | undefined {
  if (!lines?.length) {
    return undefined;
  }
  return lines.map((line) => ({
    kind: "polyline",
    id: line.id,
    label: line.label,
    color: line.color,
    points: line.points.map((point) => ({ ...point }))
  }));
}

function cloneTemplateOverlays(
  overlays: readonly RockPhysicsTemplateOverlay[] | null | undefined
): RockPhysicsTemplateOverlay[] | undefined {
  if (!overlays?.length) {
    return undefined;
  }
  return overlays.map((overlay) => {
    if (overlay.kind === "text") {
      return { ...overlay };
    }
    if (overlay.kind === "polyline") {
      return {
        ...overlay,
        points: overlay.points.map((point) => ({ ...point }))
      };
    }
    return {
      ...overlay,
      points: overlay.points.map((point) => ({ ...point })),
      labelPosition: overlay.labelPosition ? { ...overlay.labelPosition } : undefined
    };
  });
}

function normalizeTemplateOverlays(
  overlays: readonly RockPhysicsTemplateOverlayDto[] | readonly RockPhysicsTemplateOverlay[] | null | undefined
): RockPhysicsTemplateOverlay[] | undefined {
  if (!overlays?.length) {
    return undefined;
  }
  return overlays.map((overlay) => {
    if (overlay.kind === "text") {
      const rotationDeg = "rotation_deg" in overlay ? overlay.rotation_deg : overlay.rotationDeg;
      return {
        kind: "text",
        id: overlay.id,
        text: overlay.text,
        color: overlay.color,
        x: overlay.x,
        y: overlay.y,
        rotationDeg: rotationDeg ?? undefined,
        align: overlay.align ?? undefined,
        baseline: overlay.baseline ?? undefined
      };
    }
    if (overlay.kind === "polyline") {
      return {
        kind: "polyline",
        id: overlay.id,
        label: overlay.label ?? undefined,
        color: overlay.color,
        width: overlay.width ?? undefined,
        dashed: overlay.dashed ?? undefined,
        points: overlay.points.map((point) => ({ x: point.x, y: point.y }))
      };
    }
    const strokeColor = "stroke_color" in overlay ? overlay.stroke_color : overlay.strokeColor;
    const fillColor = "fill_color" in overlay ? overlay.fill_color : overlay.fillColor;
    const labelPosition = "label_position" in overlay ? overlay.label_position : overlay.labelPosition;
    return {
      kind: "polygon",
      id: overlay.id,
      label: overlay.label ?? undefined,
      strokeColor: strokeColor ?? undefined,
      fillColor,
      points: overlay.points.map((point) => ({ x: point.x, y: point.y })),
      labelPosition: labelPosition ? { x: labelPosition.x, y: labelPosition.y } : undefined
    };
  });
}

function cloneTemplateLines(lines: readonly RockPhysicsTemplateLine[] | null | undefined): RockPhysicsTemplateLine[] | undefined {
  if (!lines?.length) {
    return undefined;
  }
  return lines.map((line) => ({
    ...line,
    points: line.points.map((point) => ({ ...point }))
  }));
}

function normalizeTemplateLines(
  lines: readonly RockPhysicsTemplateLineDto[] | readonly RockPhysicsTemplateLine[] | null | undefined
): RockPhysicsTemplateLine[] | undefined {
  if (!lines?.length) {
    return undefined;
  }
  return lines.map((line) => ({
    id: line.id,
    label: line.label,
    color: line.color,
    points: line.points.map((point) => ({ x: point.x, y: point.y }))
  }));
}

function toTemplateLines(overlays: readonly RockPhysicsTemplateOverlay[] | null | undefined): RockPhysicsTemplateLine[] | undefined {
  if (!overlays?.length) {
    return undefined;
  }
  const lines = overlays.flatMap((overlay) =>
    overlay.kind === "polyline"
      ? [{
          id: overlay.id,
          label: overlay.label ?? overlay.id,
          color: overlay.color,
          points: overlay.points.map((point) => ({ ...point }))
        }]
      : []
  );
  return lines.length > 0 ? lines : undefined;
}

function issue(code: string, path: string, message: string): RockPhysicsValidationIssue {
  return { code, path, message };
}

function formatValidationMessage(issues: RockPhysicsValidationIssue[]): string {
  return [
    "Rock-physics crossplot source validation failed.",
    ...issues.map((entry) => `- [${entry.code}] ${entry.path}: ${entry.message}`)
  ].join("\n");
}
