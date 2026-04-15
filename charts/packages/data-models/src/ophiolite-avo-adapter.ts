import {
  AVO_ANALYSIS_CONTRACT_VERSION,
  type ResolvedAvoChiProjectionSourceDto,
  type ResolvedAvoCrossplotSourceDto,
  type ResolvedAvoResponseSourceDto
} from "@ophiolite/contracts";
import type {
  AvoAxisModel,
  AvoBackgroundRegionModel,
  AvoChiProjectionModel,
  AvoChiProjectionSeriesModel,
  AvoCrossplotModel,
  AvoInterfaceDescriptor,
  AvoReferenceLineModel,
  AvoResponseModel,
  AvoResponseSeriesModel
} from "./avo";

export type OphioliteResolvedAvoResponseSource = ResolvedAvoResponseSourceDto;
export type OphioliteResolvedAvoCrossplotSource = ResolvedAvoCrossplotSourceDto;
export type OphioliteResolvedAvoChiProjectionSource = ResolvedAvoChiProjectionSourceDto;
type OphioliteAvoReferenceLine = NonNullable<OphioliteResolvedAvoCrossplotSource["reference_lines"]>[number];
type OphioliteAvoBackgroundRegion = NonNullable<OphioliteResolvedAvoCrossplotSource["background_regions"]>[number];

export interface AvoValidationIssue {
  code: string;
  path: string;
  message: string;
}

export class OphioliteAvoValidationError extends Error {
  readonly issues: AvoValidationIssue[];

  constructor(issues: AvoValidationIssue[]) {
    super(formatValidationMessage(issues));
    this.name = "OphioliteAvoValidationError";
    this.issues = issues;
  }
}

export function validateOphioliteAvoResponseSource(source: OphioliteResolvedAvoResponseSource): AvoValidationIssue[] {
  const issues: AvoValidationIssue[] = [];
  const interfaceIds = new Set(source.interfaces.map((entry) => entry.id));

  validateSchemaVersion(source.schema_version, "schema_version", issues);
  validateInterfaces(source.interfaces, "interfaces", issues);

  if (source.series.length === 0) {
    issues.push(issue("missing-series", "series", "At least one AVO response series is required."));
  }

  source.series.forEach((series, index) => {
    if (!interfaceIds.has(series.interface_id)) {
      issues.push(
        issue(
          "unknown-interface",
          `series[${index}].interface_id`,
          `Series '${series.id}' references unknown interface '${series.interface_id}'.`
        )
      );
    }
    if (series.incidence_angles_deg.length !== series.values.length) {
      issues.push(
        issue(
          "length-mismatch",
          `series[${index}]`,
          `Series '${series.id}' angle/value lengths differ (${series.incidence_angles_deg.length} vs ${series.values.length}).`
        )
      );
    }
    validateFiniteArray(series.incidence_angles_deg, `series[${index}].incidence_angles_deg`, issues);
    validateFiniteArray(series.values, `series[${index}].values`, issues);
  });

  return issues;
}

export function validateOphioliteAvoCrossplotSource(source: OphioliteResolvedAvoCrossplotSource): AvoValidationIssue[] {
  const issues: AvoValidationIssue[] = [];
  const interfaceIds = new Set(source.interfaces.map((entry) => entry.id));

  validateSchemaVersion(source.schema_version, "schema_version", issues);
  validateInterfaces(source.interfaces, "interfaces", issues);

  if (source.points.length === 0) {
    issues.push(issue("missing-points", "points", "At least one AVO crossplot point is required."));
  }

  source.points.forEach((point, index) => {
    if (!interfaceIds.has(point.interface_id)) {
      issues.push(
        issue(
          "unknown-interface",
          `points[${index}].interface_id`,
          `Point ${index} references unknown interface '${point.interface_id}'.`
        )
      );
    }
    validateFiniteNumber(point.intercept, `points[${index}].intercept`, issues);
    validateFiniteNumber(point.gradient, `points[${index}].gradient`, issues);
    if (point.chi_projection != null) {
      validateFiniteNumber(point.chi_projection, `points[${index}].chi_projection`, issues);
    }
  });

  source.reference_lines?.forEach((line, index) => {
    validateFiniteNumber(line.x1, `reference_lines[${index}].x1`, issues);
    validateFiniteNumber(line.y1, `reference_lines[${index}].y1`, issues);
    validateFiniteNumber(line.x2, `reference_lines[${index}].x2`, issues);
    validateFiniteNumber(line.y2, `reference_lines[${index}].y2`, issues);
  });

  source.background_regions?.forEach((region, index) => {
    validateFiniteNumber(region.x_min, `background_regions[${index}].x_min`, issues);
    validateFiniteNumber(region.x_max, `background_regions[${index}].x_max`, issues);
    validateFiniteNumber(region.y_min, `background_regions[${index}].y_min`, issues);
    validateFiniteNumber(region.y_max, `background_regions[${index}].y_max`, issues);
  });

  return issues;
}

export function validateOphioliteAvoChiProjectionSource(
  source: OphioliteResolvedAvoChiProjectionSource
): AvoValidationIssue[] {
  const issues: AvoValidationIssue[] = [];
  const interfaceIds = new Set(source.interfaces.map((entry) => entry.id));

  validateSchemaVersion(source.schema_version, "schema_version", issues);
  validateInterfaces(source.interfaces, "interfaces", issues);
  validateFiniteNumber(source.chi_angle_deg, "chi_angle_deg", issues);

  if (source.series.length === 0) {
    issues.push(issue("missing-series", "series", "At least one chi-projection series is required."));
  }

  source.series.forEach((series, index) => {
    if (!interfaceIds.has(series.interface_id)) {
      issues.push(
        issue(
          "unknown-interface",
          `series[${index}].interface_id`,
          `Series '${series.id}' references unknown interface '${series.interface_id}'.`
        )
      );
    }
    if (series.mean_value != null) {
      validateFiniteNumber(series.mean_value, `series[${index}].mean_value`, issues);
    }
    validateFiniteArray(series.projected_values, `series[${index}].projected_values`, issues);
  });

  return issues;
}

export function adaptOphioliteAvoResponseToChart(source: OphioliteResolvedAvoResponseSource): AvoResponseModel {
  const issues = validateOphioliteAvoResponseSource(source);
  if (issues.length > 0) {
    throw new OphioliteAvoValidationError(issues);
  }

  return {
    id: source.id,
    name: source.name,
    title: source.title ?? source.name,
    subtitle: source.subtitle ?? undefined,
    xAxis: adaptAxis(source.x_axis),
    yAxis: adaptAxis(source.y_axis),
    interfaces: source.interfaces.map(adaptInterface),
    series: source.series.map(adaptResponseSeries)
  };
}

export function adaptOphioliteAvoCrossplotToChart(source: OphioliteResolvedAvoCrossplotSource): AvoCrossplotModel {
  const issues = validateOphioliteAvoCrossplotSource(source);
  if (issues.length > 0) {
    throw new OphioliteAvoValidationError(issues);
  }

  const interfaces = source.interfaces.map(adaptInterface);
  const interfaceIndexById = new Map(interfaces.map((entry, index) => [entry.id, index]));
  const pointCount = source.points.length;
  const intercept = new Float32Array(pointCount);
  const gradient = new Float32Array(pointCount);
  const interfaceIndices = new Uint16Array(pointCount);
  const hasChiProjection = source.points.some((point) => point.chi_projection != null);
  const chiProjection = hasChiProjection ? new Float32Array(pointCount) : undefined;
  const hasSimulationIds = source.points.some((point) => point.simulation_id != null);
  const simulationIds = hasSimulationIds ? new Uint32Array(pointCount) : undefined;

  source.points.forEach((point, index) => {
    intercept[index] = point.intercept;
    gradient[index] = point.gradient;
    interfaceIndices[index] = interfaceIndexById.get(point.interface_id) ?? 0;
    if (chiProjection) {
      chiProjection[index] = point.chi_projection ?? Number.NaN;
    }
    if (simulationIds) {
      simulationIds[index] = point.simulation_id ?? 0;
    }
  });

  return {
    id: source.id,
    name: source.name,
    title: source.title ?? source.name,
    subtitle: source.subtitle ?? undefined,
    xAxis: adaptAxis(source.x_axis),
    yAxis: adaptAxis(source.y_axis),
    pointCount,
    interfaces,
    columns: {
      intercept,
      gradient,
      interfaceIndices,
      chiProjection,
      simulationIds
    },
    referenceLines: (source.reference_lines ?? []).map(adaptReferenceLine),
    backgroundRegions: (source.background_regions ?? []).map(adaptBackgroundRegion)
  };
}

export function adaptOphioliteAvoChiProjectionToChart(
  source: OphioliteResolvedAvoChiProjectionSource
): AvoChiProjectionModel {
  const issues = validateOphioliteAvoChiProjectionSource(source);
  if (issues.length > 0) {
    throw new OphioliteAvoValidationError(issues);
  }

  return {
    id: source.id,
    name: source.name,
    title: source.title ?? source.name,
    subtitle: source.subtitle ?? undefined,
    chiAngleDeg: source.chi_angle_deg,
    projectionLabel: source.projection_label ?? undefined,
    xAxis: adaptAxis(source.x_axis),
    interfaces: source.interfaces.map(adaptInterface),
    series: source.series.map(adaptChiSeries),
    preferredBinCount: source.preferred_bin_count ?? undefined
  };
}

function validateSchemaVersion(value: number, path: string, issues: AvoValidationIssue[]): void {
  if (value !== AVO_ANALYSIS_CONTRACT_VERSION) {
    issues.push(
      issue(
        "unsupported-schema-version",
        path,
        `Expected AVO analysis schema version ${AVO_ANALYSIS_CONTRACT_VERSION}, got ${value}.`
      )
    );
  }
}

function validateInterfaces(
  interfaces: OphioliteResolvedAvoResponseSource["interfaces"],
  path: string,
  issues: AvoValidationIssue[]
): void {
  if (interfaces.length === 0) {
    issues.push(issue("missing-interfaces", path, "At least one AVO interface is required."));
    return;
  }

  const seen = new Set<string>();
  interfaces.forEach((entry, index) => {
    if (seen.has(entry.id)) {
      issues.push(issue("duplicate-interface", `${path}[${index}].id`, `Duplicate interface id '${entry.id}'.`));
      return;
    }
    seen.add(entry.id);
  });
}

function validateFiniteArray(values: ArrayLike<number>, path: string, issues: AvoValidationIssue[]): void {
  for (let index = 0; index < values.length; index += 1) {
    if (!Number.isFinite(values[index])) {
      issues.push(issue("invalid-number", `${path}[${index}]`, `Expected a finite number at '${path}[${index}]'.`));
      break;
    }
  }
}

function validateFiniteNumber(value: number, path: string, issues: AvoValidationIssue[]): void {
  if (!Number.isFinite(value)) {
    issues.push(issue("invalid-number", path, `Expected a finite number at '${path}'.`));
  }
}

function adaptAxis(axis: OphioliteResolvedAvoResponseSource["x_axis"]): AvoAxisModel {
  return {
    label: axis.label ?? undefined,
    unit: axis.unit ?? undefined,
    range:
      axis.min_value != null && axis.max_value != null
        ? {
            min: axis.min_value,
            max: axis.max_value
          }
        : undefined
  };
}

function adaptInterface(entry: OphioliteResolvedAvoResponseSource["interfaces"][number]): AvoInterfaceDescriptor {
  return {
    id: entry.id,
    label: entry.label,
    color: entry.color,
    reservoirLabel: entry.reservoir_label ?? undefined
  };
}

function adaptResponseSeries(series: OphioliteResolvedAvoResponseSource["series"][number]): AvoResponseSeriesModel {
  return {
    id: series.id,
    interfaceId: series.interface_id,
    label: series.label,
    color: series.color,
    style: series.style,
    reflectivityModel: series.reflectivity_model,
    anisotropyMode: series.anisotropy_mode,
    incidenceAnglesDeg: Float32Array.from(series.incidence_angles_deg),
    values: Float32Array.from(series.values)
  };
}

function adaptReferenceLine(line: OphioliteAvoReferenceLine): AvoReferenceLineModel {
  return {
    id: line.id,
    label: line.label ?? undefined,
    color: line.color,
    style: line.style,
    x1: line.x1,
    y1: line.y1,
    x2: line.x2,
    y2: line.y2
  };
}

function adaptBackgroundRegion(region: OphioliteAvoBackgroundRegion): AvoBackgroundRegionModel {
  return {
    id: region.id,
    label: region.label ?? undefined,
    fillColor: region.fill_color,
    xMin: region.x_min,
    xMax: region.x_max,
    yMin: region.y_min,
    yMax: region.y_max
  };
}

function adaptChiSeries(series: OphioliteResolvedAvoChiProjectionSource["series"][number]): AvoChiProjectionSeriesModel {
  return {
    id: series.id,
    interfaceId: series.interface_id,
    label: series.label,
    color: series.color,
    projectedValues: Float32Array.from(series.projected_values),
    meanValue: series.mean_value ?? undefined
  };
}

function issue(code: string, path: string, message: string): AvoValidationIssue {
  return { code, path, message };
}

function formatValidationMessage(issues: AvoValidationIssue[]): string {
  return [
    "AVO source validation failed.",
    ...issues.map((entry) => `- [${entry.code}] ${entry.path}: ${entry.message}`)
  ].join("\n");
}
