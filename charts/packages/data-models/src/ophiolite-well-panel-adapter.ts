import type {
  ResolvedWellPanelSourceDto,
  ResolvedWellPanelWellDto,
  WellPanelDrillingObservationDto,
  WellPanelDrillingSetDto,
  WellPanelLogCurveDto,
  WellPanelPressureObservationDto,
  WellPanelPressureSetDto,
  WellPanelTopRowDto,
  WellPanelTopSetDto,
  WellPanelTrajectoryDto,
  WellPanelTrajectoryRowDto
} from "@ophiolite/contracts";
import type {
  PointObservationFamily,
  PointObservationSample,
  TrajectoryStation,
  WellPanelCurveData,
  WellPanelDataCatalog,
  WellPanelModel,
  WellPanelPointObservationData,
  WellPanelSeismicSectionData,
  WellPanelSeismicTraceData,
  WellPanelTop,
  WellPanelTopSetData,
  WellPanelTrack,
  WellPanelTrajectoryData
} from "./well-panel";
import { validateSectionPayload } from "./ophiolite-seismic-adapter";
import type { SectionPayload } from "./seismic";
import type { DepthDatum, DepthDomain, DepthMappingSample, TrackAxis } from "./well-correlation";

export type OphioliteResolvedLogCurve = WellPanelLogCurveDto;
export type OphioliteResolvedTrajectoryRow = WellPanelTrajectoryRowDto;
export type OphioliteResolvedTrajectoryAsset = WellPanelTrajectoryDto;
export type OphioliteResolvedTopRow = WellPanelTopRowDto;
export type OphioliteResolvedTopSetAsset = WellPanelTopSetDto;
export type OphioliteResolvedPressureObservationRow = WellPanelPressureObservationDto;
export type OphioliteResolvedPressureObservationAsset = WellPanelPressureSetDto;
export type OphioliteResolvedDrillingObservationRow = WellPanelDrillingObservationDto;
export type OphioliteResolvedDrillingObservationAsset = WellPanelDrillingSetDto;

export interface OphioliteResolvedSeismicTraceSeries {
  id: string;
  name: string;
  amplitudes: ArrayLike<number>;
  metadata?: Record<string, string | number | boolean | null>;
}

export interface OphioliteResolvedSeismicTraceSetAsset {
  id: string;
  name: string;
  nativeDepths: ArrayLike<number>;
  panelDepths?: ArrayLike<number>;
  amplitudeUnit?: string;
  traces: OphioliteResolvedSeismicTraceSeries[];
}

export interface OphioliteResolvedSeismicSectionAsset {
  id: string;
  name: string;
  section: SectionPayload;
  panelDepths: ArrayLike<number>;
  nativeDepths?: ArrayLike<number>;
  wellTraceIndex?: number;
}

export interface OphioliteResolvedWellPanelColumn extends ResolvedWellPanelWellDto {
  header_note?: string;
  depth_axis?: TrackAxis;
  pressure_axis?: TrackAxis;
  drilling_axis?: TrackAxis;
  seismic_trace_sets?: OphioliteResolvedSeismicTraceSetAsset[];
  seismic_sections?: OphioliteResolvedSeismicSectionAsset[];
}

export interface OphioliteResolvedWellPanelSource extends Omit<ResolvedWellPanelSourceDto, "wells"> {
  depth_domain?: DepthDomain;
  wells: OphioliteResolvedWellPanelColumn[];
  background?: string;
}

export interface OphioliteWellPanelLayoutWell {
  wellId: string;
  tracks: WellPanelTrack[];
  headerNote?: string;
}

export interface OphioliteWellPanelLayout {
  wells: OphioliteWellPanelLayoutWell[];
  background?: string;
}

export interface OphioliteWellPanelAdapterOptions {
  topColorByName?: Record<string, string>;
  pointFamilyByAssetId?: Record<string, PointObservationFamily>;
}

export interface WellPanelValidationIssue {
  code: string;
  path: string;
  message: string;
}

export class OphioliteWellPanelValidationError extends Error {
  readonly issues: WellPanelValidationIssue[];

  constructor(issues: WellPanelValidationIssue[]) {
    super([
      "Well-panel source validation failed.",
      ...issues.map((entry) => `- [${entry.code}] ${entry.path}: ${entry.message}`)
    ].join("\n"));
    this.name = "OphioliteWellPanelValidationError";
    this.issues = issues;
  }
}

export function adaptOphioliteWellPanelToChart(
  source: OphioliteResolvedWellPanelSource,
  layout: OphioliteWellPanelLayout,
  options: OphioliteWellPanelAdapterOptions = {}
): WellPanelModel {
  const issues = validateOphioliteWellPanelSource(source, layout);
  if (issues.length > 0) {
    throw new OphioliteWellPanelValidationError(issues);
  }

  const tracksByWellId = new Map(layout.wells.map((well) => [well.wellId, well]));

  return {
    id: source.id,
    name: source.name,
    depthDomain: source.depth_domain ?? deriveDepthDomain(source.wells),
    background: layout.background ?? source.background,
    wells: source.wells.map((well) => {
      const layoutWell = tracksByWellId.get(well.wellbore_id);
      return {
        id: well.wellbore_id,
        name: well.name,
        nativeDepthDatum: mapDepthDatum(well.native_depth_datum),
        panelDepthMapping: well.panel_depth_mapping.map((sample) => ({
          nativeDepth: sample.native_depth,
          panelDepth: sample.panel_depth
        })),
        headerNote: layoutWell?.headerNote ?? well.header_note,
        data: adaptCatalog(well, options),
        tracks: layoutWell ? structuredClone(layoutWell.tracks) : []
      };
    })
  };
}

function adaptCatalog(
  well: OphioliteResolvedWellPanelColumn,
  options: OphioliteWellPanelAdapterOptions
): WellPanelDataCatalog {
  return {
    curves: well.logs?.map(adaptLogCurve),
    trajectories: well.trajectories?.map(adaptTrajectoryAsset),
    topSets: well.top_sets?.map((asset) => adaptTopSetAsset(asset, options.topColorByName)),
    pointObservations: [
      ...(well.pressure_observations?.map((asset) =>
        adaptPressureObservationAsset(asset, well.pressure_axis, options.pointFamilyByAssetId?.[asset.asset_id])
      ) ?? []),
      ...(well.drilling_observations?.map((asset) =>
        adaptDrillingObservationAsset(asset, well.drilling_axis, options.pointFamilyByAssetId?.[asset.asset_id])
      ) ?? [])
    ],
    seismicTraces: well.seismic_trace_sets?.map(adaptSeismicTraceSetAsset),
    seismicSections: well.seismic_sections?.map(adaptSeismicSectionAsset)
  };
}

function adaptLogCurve(curve: WellPanelLogCurveDto): WellPanelCurveData {
  const nativeDepths: number[] = [];
  const values: number[] = [];
  const count = Math.min(curve.depths.length, curve.values.length);

  for (let index = 0; index < count; index += 1) {
    const depth = curve.depths[index];
    const value = curve.values[index];
    if (!Number.isFinite(depth) || typeof value !== "number" || !Number.isFinite(value)) {
      continue;
    }
    nativeDepths.push(depth);
    values.push(value);
  }

  return {
    kind: "log-curve",
    id: curve.asset_id,
    name: curve.curve_name,
    mnemonic: curve.original_mnemonic ?? curve.curve_name,
    unit: curve.unit ?? undefined,
    semantics: curve.semantic_type,
    nativeDepths: Float32Array.from(nativeDepths),
    values: Float32Array.from(values),
    metadata: {
      logicalAssetId: curve.logical_asset_id,
      assetName: curve.asset_name
    }
  };
}

function adaptTrajectoryAsset(asset: WellPanelTrajectoryDto): WellPanelTrajectoryData {
  return {
    kind: "trajectory",
    id: asset.asset_id,
    name: asset.asset_name,
    stations: asset.rows.map((row, index) => ({
      id: `${asset.asset_id}:station:${index}`,
      md: row.measured_depth,
      tvd: normalizeOptionalNumber(row.true_vertical_depth),
      inclination: normalizeOptionalNumber(row.inclination_deg),
      azimuth: normalizeOptionalNumber(row.azimuth_deg),
      northing: normalizeOptionalNumber(row.northing_offset),
      easting: normalizeOptionalNumber(row.easting_offset)
    } satisfies TrajectoryStation))
  };
}

function adaptTopSetAsset(
  asset: WellPanelTopSetDto,
  topColorByName: Record<string, string> | undefined
): WellPanelTopSetData {
  return {
    kind: "top-set",
    id: asset.asset_id,
    name: asset.asset_name,
    tops: asset.rows.map((row, index) => ({
      id: `${asset.asset_id}:top:${index}`,
      name: row.name,
      nativeDepth: row.top_depth,
      color: topColorByName?.[row.name] ?? colorForLabel(row.name),
      source: row.source ? "imported" : "picked",
      metadata: {
        baseDepth: normalizeOptionalNumber(row.base_depth) ?? null,
        source: row.source ?? null,
        depthReference: row.source_depth_reference ?? null
      }
    } satisfies WellPanelTop))
  };
}

function adaptPressureObservationAsset(
  asset: WellPanelPressureSetDto,
  axis: TrackAxis | undefined,
  familyOverride: PointObservationFamily | undefined
): WellPanelPointObservationData {
  const points = asset.rows.flatMap((row, index) => {
    if (!Number.isFinite(row.measured_depth) || !Number.isFinite(row.pressure)) {
      return [];
    }
    return [{
      id: `${asset.asset_id}:pressure:${index}`,
      nativeDepth: row.measured_depth!,
      value: row.pressure,
      label: row.test_kind ?? row.phase ?? undefined,
      metadata: {
        phase: row.phase ?? null,
        testKind: row.test_kind ?? null,
        timestamp: row.timestamp ?? null
      }
    } satisfies PointObservationSample];
  });

  return {
    kind: "point-observation",
    id: asset.asset_id,
    name: asset.asset_name,
    family: familyOverride ?? "pressure-observation",
    unit: "pressure",
    axis: axis ? { ...axis } : deriveNumericAxis(points, asset.asset_name, "pressure"),
    points
  };
}

function adaptDrillingObservationAsset(
  asset: WellPanelDrillingSetDto,
  axis: TrackAxis | undefined,
  familyOverride: PointObservationFamily | undefined
): WellPanelPointObservationData {
  const points = asset.rows.flatMap((row, index) => {
    if (!Number.isFinite(row.measured_depth) || !Number.isFinite(row.value)) {
      return [];
    }
    return [{
      id: `${asset.asset_id}:drilling:${index}`,
      nativeDepth: row.measured_depth!,
      value: row.value!,
      label: row.event_kind,
      metadata: {
        eventKind: row.event_kind,
        unit: row.unit ?? null,
        timestamp: row.timestamp ?? null,
        comment: row.comment ?? null
      }
    } satisfies PointObservationSample];
  });

  return {
    kind: "point-observation",
    id: asset.asset_id,
    name: asset.asset_name,
    family: familyOverride ?? "drilling-observation",
    unit: firstDefinedString(asset.rows.map((row) => row.unit)) ?? "value",
    axis: axis ? { ...axis } : deriveNumericAxis(points, asset.asset_name, "value"),
    points
  };
}

function adaptSeismicTraceSetAsset(asset: OphioliteResolvedSeismicTraceSetAsset): WellPanelSeismicTraceData {
  return {
    kind: "seismic-trace",
    id: asset.id,
    name: asset.name,
    nativeDepths: Float32Array.from(toNumberArray(asset.nativeDepths)),
    panelDepths: asset.panelDepths ? Float32Array.from(toNumberArray(asset.panelDepths)) : undefined,
    amplitudeUnit: asset.amplitudeUnit,
    traces: asset.traces.map((trace) => ({
      id: trace.id,
      name: trace.name,
      amplitudes: Float32Array.from(toNumberArray(trace.amplitudes)),
      metadata: trace.metadata ? { ...trace.metadata } : undefined
    }))
  };
}

function adaptSeismicSectionAsset(asset: OphioliteResolvedSeismicSectionAsset): WellPanelSeismicSectionData {
  return {
    kind: "seismic-section",
    id: asset.id,
    name: asset.name,
    section: cloneSectionPayload(asset.section),
    panelDepths: Float32Array.from(toNumberArray(asset.panelDepths)),
    nativeDepths: asset.nativeDepths ? Float32Array.from(toNumberArray(asset.nativeDepths)) : undefined,
    wellTraceIndex: asset.wellTraceIndex
  };
}

function deriveDepthDomain(wells: OphioliteResolvedWellPanelColumn[]): DepthDomain {
  const panelDepths = wells.flatMap((well) => well.panel_depth_mapping.map((sample) => sample.panel_depth));
  const start = panelDepths.length ? Math.min(...panelDepths) : 0;
  const end = panelDepths.length ? Math.max(...panelDepths) : 1;
  return {
    start,
    end,
    unit: "m",
    label: "Correlation Depth"
  };
}

function mapDepthDatum(value: string): DepthDatum {
  switch (value) {
    case "tvd":
      return "tvd";
    case "tvdss":
      return "tvdss";
    case "md":
    case "measured_depth":
      return "md";
    default:
      return "md";
  }
}

function deriveNumericAxis(points: PointObservationSample[], label: string, unit?: string): TrackAxis {
  const values = points.map((point) => point.value).filter(Number.isFinite);
  if (values.length === 0) {
    return {
      min: 0,
      max: 1,
      label,
      unit,
      tickCount: 4
    };
  }
  const min = Math.min(...values);
  const max = Math.max(...values);
  const pad = Math.max((max - min) * 0.08, 1e-6);
  return {
    min: min - pad,
    max: max + pad,
    label,
    unit,
    tickCount: 4
  };
}

function cloneSectionPayload(section: SectionPayload): SectionPayload {
  return {
    axis: section.axis,
    coordinate: { ...section.coordinate },
    horizontalAxis: new Float64Array(section.horizontalAxis),
    inlineAxis: section.inlineAxis ? new Float64Array(section.inlineAxis) : undefined,
    xlineAxis: section.xlineAxis ? new Float64Array(section.xlineAxis) : undefined,
    sampleAxis: new Float32Array(section.sampleAxis),
    amplitudes: new Float32Array(section.amplitudes),
    dimensions: { ...section.dimensions },
    units: section.units ? { ...section.units } : undefined,
    metadata: section.metadata ? { ...section.metadata, notes: section.metadata.notes ? [...section.metadata.notes] : undefined } : undefined,
    displayDefaults: section.displayDefaults ? { ...section.displayDefaults } : undefined,
    overlay: section.overlay
      ? {
          ...section.overlay,
          values: new Uint8Array(section.overlay.values)
        }
      : undefined
  };
}

function colorForLabel(label: string): string {
  const palette = ["#b64f4f", "#c56156", "#c97d71", "#9c3e3e", "#7c2d2d", "#8d5c2e", "#5d4ea3", "#3a6e9f"];
  let hash = 0;
  for (let index = 0; index < label.length; index += 1) {
    hash = (hash * 31 + label.charCodeAt(index)) >>> 0;
  }
  return palette[hash % palette.length]!;
}

function toNumberArray(values: ArrayLike<number>): number[] {
  const next = new Array<number>(values.length);
  for (let index = 0; index < values.length; index += 1) {
    next[index] = values[index] ?? 0;
  }
  return next;
}

function normalizeOptionalNumber(value: number | null | undefined): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function firstDefinedString(values: Array<string | null | undefined>): string | undefined {
  return values.find((value): value is string => typeof value === "string" && value.length > 0);
}

export function validateOphioliteWellPanelSource(
  source: OphioliteResolvedWellPanelSource,
  layout: OphioliteWellPanelLayout
): WellPanelValidationIssue[] {
  const issues: WellPanelValidationIssue[] = [];
  const wellsByWellboreId = new Map<string, OphioliteResolvedWellPanelColumn>();
  const layoutByWellId = new Map<string, OphioliteWellPanelLayoutWell>();

  if (source.wells.length === 0) {
    issues.push(issue("missing-wells", "wells", "At least one resolved well column is required."));
  }

  source.wells.forEach((well, wellIndex) => {
    if (wellsByWellboreId.has(well.wellbore_id)) {
      issues.push(
        issue("duplicate-wellbore", `wells[${wellIndex}].wellbore_id`, `Duplicate wellbore id '${well.wellbore_id}'.`)
      );
    }
    wellsByWellboreId.set(well.wellbore_id, well);

    validateDepthMapping(well.panel_depth_mapping, `wells[${wellIndex}].panel_depth_mapping`, issues);
    validateTrackAxis(well.depth_axis, `wells[${wellIndex}].depth_axis`, issues);
    validateTrackAxis(well.pressure_axis, `wells[${wellIndex}].pressure_axis`, issues);
    validateTrackAxis(well.drilling_axis, `wells[${wellIndex}].drilling_axis`, issues);
    validateResolvedWellCatalog(well, wellIndex, issues);
  });

  layout.wells.forEach((layoutWell, layoutWellIndex) => {
    if (layoutByWellId.has(layoutWell.wellId)) {
      issues.push(
        issue("duplicate-layout-well", `layout.wells[${layoutWellIndex}].wellId`, `Duplicate layout well '${layoutWell.wellId}'.`)
      );
    }
    layoutByWellId.set(layoutWell.wellId, layoutWell);

    const sourceWell = wellsByWellboreId.get(layoutWell.wellId);
    if (!sourceWell) {
      issues.push(
        issue(
          "unknown-layout-well",
          `layout.wells[${layoutWellIndex}].wellId`,
          `Layout references unknown wellbore '${layoutWell.wellId}'.`
        )
      );
      return;
    }

    validateLayoutWell(layoutWell, layoutWellIndex, sourceWell, issues);
  });

  return issues;
}

function validateResolvedWellCatalog(
  well: OphioliteResolvedWellPanelColumn,
  wellIndex: number,
  issues: WellPanelValidationIssue[]
): void {
  validateUniqueIds(well.logs ?? [], (item) => item.asset_id, `wells[${wellIndex}].logs`, "duplicate-log-asset", issues);
  validateUniqueIds(well.top_sets ?? [], (item) => item.asset_id, `wells[${wellIndex}].top_sets`, "duplicate-top-set", issues);
  validateUniqueIds(
    well.pressure_observations ?? [],
    (item) => item.asset_id,
    `wells[${wellIndex}].pressure_observations`,
    "duplicate-pressure-set",
    issues
  );
  validateUniqueIds(
    well.drilling_observations ?? [],
    (item) => item.asset_id,
    `wells[${wellIndex}].drilling_observations`,
    "duplicate-drilling-set",
    issues
  );
  validateUniqueIds(
    [...(well.pressure_observations ?? []), ...(well.drilling_observations ?? [])],
    (item) => item.asset_id,
    `wells[${wellIndex}].point_observations`,
    "duplicate-point-observation-set",
    issues
  );
  validateUniqueIds(
    well.seismic_trace_sets ?? [],
    (item) => item.id,
    `wells[${wellIndex}].seismic_trace_sets`,
    "duplicate-seismic-trace-set",
    issues
  );
  validateUniqueIds(
    well.seismic_sections ?? [],
    (item) => item.id,
    `wells[${wellIndex}].seismic_sections`,
    "duplicate-seismic-section",
    issues
  );

  well.logs?.forEach((curve, curveIndex) => validateLogCurve(curve, `wells[${wellIndex}].logs[${curveIndex}]`, issues));
  well.top_sets?.forEach((asset, assetIndex) => validateTopSet(asset, `wells[${wellIndex}].top_sets[${assetIndex}]`, issues));
  well.pressure_observations?.forEach((asset, assetIndex) =>
    validatePressureSet(asset, `wells[${wellIndex}].pressure_observations[${assetIndex}]`, issues)
  );
  well.drilling_observations?.forEach((asset, assetIndex) =>
    validateDrillingSet(asset, `wells[${wellIndex}].drilling_observations[${assetIndex}]`, issues)
  );
  well.seismic_trace_sets?.forEach((asset, assetIndex) =>
    validateSeismicTraceSet(asset, `wells[${wellIndex}].seismic_trace_sets[${assetIndex}]`, issues)
  );
  well.seismic_sections?.forEach((asset, assetIndex) =>
    validateSeismicSectionAsset(asset, `wells[${wellIndex}].seismic_sections[${assetIndex}]`, issues)
  );
}

function validateLayoutWell(
  layoutWell: OphioliteWellPanelLayoutWell,
  layoutWellIndex: number,
  sourceWell: OphioliteResolvedWellPanelColumn,
  issues: WellPanelValidationIssue[]
): void {
  const trackIds = new Set<string>();
  const layerIds = new Set<string>();
  const catalogIds = buildCatalogIds(sourceWell);

  layoutWell.tracks.forEach((track, trackIndex) => {
    const trackPath = `layout.wells[${layoutWellIndex}].tracks[${trackIndex}]`;
    if (trackIds.has(track.id)) {
      issues.push(issue("duplicate-track-id", `${trackPath}.id`, `Duplicate track id '${track.id}'.`));
    }
    trackIds.add(track.id);

    if (!Number.isFinite(track.width) || track.width <= 0) {
      issues.push(issue("invalid-track-width", `${trackPath}.width`, `Track '${track.id}' must have a positive width.`));
    }

    if (track.kind === "reference") {
      track.layers?.forEach((layer, layerIndex) => {
        validateLayerId(layer.id, `${trackPath}.layers[${layerIndex}].id`, layerIds, issues);
        if (!catalogIds.topSets.has(layer.dataId)) {
          issues.push(
            issue(
              "invalid-reference-layer-data",
              `${trackPath}.layers[${layerIndex}].dataId`,
              `Reference track '${track.id}' requires top-set data, got '${layer.dataId}'.`
            )
          );
        }
      });
      return;
    }

    if (track.kind === "scalar") {
      validateTrackAxis(track.xAxis, `${trackPath}.xAxis`, issues);
      let primaryLayerCount = 0;
      const curveDataIds = new Set(
        track.layers.filter((layer) => layer.kind === "curve").map((layer) => layer.dataId)
      );

      track.layers.forEach((layer, layerIndex) => {
        const layerPath = `${trackPath}.layers[${layerIndex}]`;
        validateLayerId(layer.id, `${layerPath}.id`, layerIds, issues);

        if (layer.kind === "curve") {
          primaryLayerCount += 1;
          if (!catalogIds.curves.has(layer.dataId)) {
            issues.push(
              issue("invalid-curve-layer-data", `${layerPath}.dataId`, `Curve layer references unknown log curve '${layer.dataId}'.`)
            );
          }
          const fill = layer.style.fill;
          if (fill?.mode === "between-curves") {
            if (!fill.targetCurveId) {
              issues.push(
                issue("missing-between-curve-target", `${layerPath}.style.fill.targetCurveId`, "Between-curves fill requires a target curve id.")
              );
            } else if (!curveDataIds.has(fill.targetCurveId)) {
              issues.push(
                issue(
                  "unknown-between-curve-target",
                  `${layerPath}.style.fill.targetCurveId`,
                  `Between-curves fill target '${fill.targetCurveId}' is not present in scalar track '${track.id}'.`
                )
              );
            }
          }
          return;
        }

        if (layer.kind === "point-observation") {
          primaryLayerCount += 1;
          if (!catalogIds.pointObservations.has(layer.dataId)) {
            issues.push(
              issue(
                "invalid-point-layer-data",
                `${layerPath}.dataId`,
                `Point-observation layer references unknown point set '${layer.dataId}'.`
              )
            );
          }
          return;
        }

        if (!catalogIds.topSets.has(layer.dataId)) {
          issues.push(
            issue(
              "invalid-top-overlay-data",
              `${layerPath}.dataId`,
              `Top-overlay layer references unknown top set '${layer.dataId}'.`
            )
          );
        }
      });

      if (primaryLayerCount === 0) {
        issues.push(
          issue(
            "missing-scalar-primary-layer",
            `${trackPath}.layers`,
            `Scalar track '${track.id}' requires at least one curve or point-observation layer.`
          )
        );
      }
      return;
    }

    if (track.kind === "seismic-trace") {
      let primaryLayerCount = 0;
      track.layers.forEach((layer, layerIndex) => {
        const layerPath = `${trackPath}.layers[${layerIndex}]`;
        validateLayerId(layer.id, `${layerPath}.id`, layerIds, issues);

        if (layer.kind === "top-overlay") {
          if (!catalogIds.topSets.has(layer.dataId)) {
            issues.push(
              issue(
                "invalid-top-overlay-data",
                `${layerPath}.dataId`,
                `Top-overlay layer references unknown top set '${layer.dataId}'.`
              )
            );
          }
          return;
        }

        primaryLayerCount += 1;
        if (!catalogIds.seismicTraces.has(layer.dataId)) {
          issues.push(
            issue(
              "invalid-seismic-trace-layer-data",
              `${layerPath}.dataId`,
              `Seismic-trace layer references unknown seismic trace set '${layer.dataId}'.`
            )
          );
          return;
        }

        const traceSet = sourceWell.seismic_trace_sets?.find((item) => item.id === layer.dataId);
        if (traceSet && layer.traceIds?.some((traceId) => !traceSet.traces.some((trace) => trace.id === traceId))) {
          const missing = layer.traceIds.filter((traceId) => !traceSet.traces.some((trace) => trace.id === traceId));
          issues.push(
            issue(
              "unknown-trace-id",
              `${layerPath}.traceIds`,
              `Trace layer references unknown trace ids for '${layer.dataId}': ${missing.join(", ")}.`
            )
          );
        }
      });

      if (primaryLayerCount === 0) {
        issues.push(
          issue(
            "missing-seismic-trace-layer",
            `${trackPath}.layers`,
            `Seismic-trace track '${track.id}' requires at least one seismic-trace layer.`
          )
        );
      }
      return;
    }

    let primaryLayerCount = 0;
    track.layers.forEach((layer, layerIndex) => {
      const layerPath = `${trackPath}.layers[${layerIndex}]`;
      validateLayerId(layer.id, `${layerPath}.id`, layerIds, issues);

      if (layer.kind === "top-overlay") {
        if (!catalogIds.topSets.has(layer.dataId)) {
          issues.push(
            issue(
              "invalid-top-overlay-data",
              `${layerPath}.dataId`,
              `Top-overlay layer references unknown top set '${layer.dataId}'.`
            )
          );
        }
        return;
      }

      primaryLayerCount += 1;
      if (!catalogIds.seismicSections.has(layer.dataId)) {
        issues.push(
          issue(
            "invalid-seismic-section-layer-data",
            `${layerPath}.dataId`,
            `Seismic-section layer references unknown seismic section '${layer.dataId}'.`
          )
        );
      }
    });

    if (primaryLayerCount === 0) {
      issues.push(
        issue(
          "missing-seismic-section-layer",
          `${trackPath}.layers`,
          `Seismic-section track '${track.id}' requires at least one seismic-section layer.`
        )
      );
    }
  });
}

function buildCatalogIds(well: OphioliteResolvedWellPanelColumn) {
  return {
    curves: new Set((well.logs ?? []).map((item) => item.asset_id)),
    pointObservations: new Set([
      ...(well.pressure_observations ?? []).map((item) => item.asset_id),
      ...(well.drilling_observations ?? []).map((item) => item.asset_id)
    ]),
    topSets: new Set((well.top_sets ?? []).map((item) => item.asset_id)),
    seismicTraces: new Set((well.seismic_trace_sets ?? []).map((item) => item.id)),
    seismicSections: new Set((well.seismic_sections ?? []).map((item) => item.id))
  };
}

function validateDepthMapping(
  mapping: Array<{ native_depth: number; panel_depth: number }>,
  path: string,
  issues: WellPanelValidationIssue[]
): void {
  if (mapping.length === 0) {
    issues.push(issue("empty-depth-mapping", path, "Panel depth mapping must not be empty."));
    return;
  }

  for (let index = 0; index < mapping.length; index += 1) {
    const sample = mapping[index]!;
    validateFiniteNumber(sample.native_depth, `${path}[${index}].native_depth`, issues);
    validateFiniteNumber(sample.panel_depth, `${path}[${index}].panel_depth`, issues);
    if (index === 0) {
      continue;
    }
    const previous = mapping[index - 1]!;
    if (sample.native_depth <= previous.native_depth) {
      issues.push(
        issue(
          "non-monotonic-native-depth",
          `${path}[${index}].native_depth`,
          "Panel depth mapping native depths must be strictly increasing."
        )
      );
    }
    if (sample.panel_depth <= previous.panel_depth) {
      issues.push(
        issue(
          "non-monotonic-panel-depth",
          `${path}[${index}].panel_depth`,
          "Panel depth mapping panel depths must be strictly increasing."
        )
      );
    }
  }
}

function validateTrackAxis(
  axis: TrackAxis | undefined,
  path: string,
  issues: WellPanelValidationIssue[]
): void {
  if (!axis) {
    return;
  }
  validateFiniteNumber(axis.min, `${path}.min`, issues);
  validateFiniteNumber(axis.max, `${path}.max`, issues);
  if (axis.max <= axis.min) {
    issues.push(issue("invalid-axis-range", path, `Axis '${path}' must have max > min.`));
  }
  if (axis.scale === "log" && axis.min <= 0) {
    issues.push(issue("invalid-log-axis", path, `Log axis '${path}' requires min > 0.`));
  }
}

function validateLogCurve(curve: WellPanelLogCurveDto, path: string, issues: WellPanelValidationIssue[]): void {
  if (curve.depths.length !== curve.values.length) {
    issues.push(
      issue(
        "curve-length-mismatch",
        path,
        `Curve '${curve.asset_id}' depth/value lengths differ (${curve.depths.length} vs ${curve.values.length}).`
      )
    );
  }
  if (curve.depths.length === 0) {
    issues.push(issue("empty-curve", path, `Curve '${curve.asset_id}' has no samples.`));
  }
  let previousDepth = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < Math.min(curve.depths.length, curve.values.length); index += 1) {
    const depth = curve.depths[index]!;
    const value = curve.values[index];
    validateFiniteNumber(depth, `${path}.depths[${index}]`, issues);
    validateFiniteNumber(value, `${path}.values[${index}]`, issues);
    if (index > 0 && depth <= previousDepth) {
      issues.push(
        issue("non-monotonic-curve-depth", `${path}.depths[${index}]`, `Curve '${curve.asset_id}' depths must be strictly increasing.`)
      );
      break;
    }
    previousDepth = depth;
  }
}

function validateTopSet(asset: WellPanelTopSetDto, path: string, issues: WellPanelValidationIssue[]): void {
  asset.rows.forEach((row, rowIndex) => {
    validateFiniteNumber(row.top_depth, `${path}.rows[${rowIndex}].top_depth`, issues);
    if (row.base_depth !== null && row.base_depth !== undefined) {
      validateFiniteNumber(row.base_depth, `${path}.rows[${rowIndex}].base_depth`, issues);
      if (Number.isFinite(row.base_depth) && row.base_depth <= row.top_depth) {
        issues.push(
          issue(
            "invalid-top-interval",
            `${path}.rows[${rowIndex}].base_depth`,
            `Top row '${row.name}' base depth must be greater than top depth.`
          )
        );
      }
    }
  });
}

function validatePressureSet(asset: WellPanelPressureSetDto, path: string, issues: WellPanelValidationIssue[]): void {
  asset.rows.forEach((row, rowIndex) => {
    validateFiniteNumber(row.measured_depth, `${path}.rows[${rowIndex}].measured_depth`, issues);
    validateFiniteNumber(row.pressure, `${path}.rows[${rowIndex}].pressure`, issues);
  });
}

function validateDrillingSet(asset: WellPanelDrillingSetDto, path: string, issues: WellPanelValidationIssue[]): void {
  asset.rows.forEach((row, rowIndex) => {
    validateFiniteNumber(row.measured_depth, `${path}.rows[${rowIndex}].measured_depth`, issues);
    validateFiniteNumber(row.value, `${path}.rows[${rowIndex}].value`, issues);
  });
}

function validateSeismicTraceSet(
  asset: OphioliteResolvedSeismicTraceSetAsset,
  path: string,
  issues: WellPanelValidationIssue[]
): void {
  if (asset.nativeDepths.length === 0) {
    issues.push(issue("empty-seismic-trace-depths", `${path}.nativeDepths`, `Seismic trace set '${asset.id}' has no depth samples.`));
  }
  if (asset.panelDepths && asset.panelDepths.length !== asset.nativeDepths.length) {
    issues.push(
      issue(
        "seismic-trace-depth-length-mismatch",
        `${path}.panelDepths`,
        `Seismic trace set '${asset.id}' panelDepths length must match nativeDepths length.`
      )
    );
  }
  validateArrayLikeNumbers(asset.nativeDepths, `${path}.nativeDepths`, issues);
  validateArrayLikeNumbers(asset.panelDepths, `${path}.panelDepths`, issues);
  validateUniqueIds(asset.traces, (trace) => trace.id, `${path}.traces`, "duplicate-trace-series", issues);
  asset.traces.forEach((trace, traceIndex) => {
    if (trace.amplitudes.length !== asset.nativeDepths.length) {
      issues.push(
        issue(
          "trace-sample-length-mismatch",
          `${path}.traces[${traceIndex}].amplitudes`,
          `Trace '${trace.id}' amplitude count must match seismic trace depth count.`
        )
      );
    }
    validateArrayLikeNumbers(trace.amplitudes, `${path}.traces[${traceIndex}].amplitudes`, issues);
  });
}

function validateSeismicSectionAsset(
  asset: OphioliteResolvedSeismicSectionAsset,
  path: string,
  issues: WellPanelValidationIssue[]
): void {
  const { section } = asset;
  issues.push(...validateSectionPayload(section).map((entry) => ({
    ...entry,
    path: `${path}.section.${entry.path}`
  })));
  if (asset.panelDepths.length !== section.dimensions.samples) {
    issues.push(
      issue(
        "section-panel-depth-length-mismatch",
        `${path}.panelDepths`,
        `Seismic section '${asset.id}' panelDepths length must match section sample count.`
      )
    );
  }
  if (asset.nativeDepths && asset.nativeDepths.length !== section.dimensions.samples) {
    issues.push(
      issue(
        "section-native-depth-length-mismatch",
        `${path}.nativeDepths`,
        `Seismic section '${asset.id}' nativeDepths length must match section sample count.`
      )
    );
  }
  if (section.horizontalAxis.length !== section.dimensions.traces) {
    issues.push(
      issue(
        "section-horizontal-axis-length-mismatch",
        `${path}.section.horizontalAxis`,
        `Seismic section '${asset.id}' horizontal axis length must match trace count.`
      )
    );
  }
  if (section.sampleAxis.length !== section.dimensions.samples) {
    issues.push(
      issue(
        "section-sample-axis-length-mismatch",
        `${path}.section.sampleAxis`,
        `Seismic section '${asset.id}' sample axis length must match sample count.`
      )
    );
  }
  if (section.amplitudes.length !== section.dimensions.traces * section.dimensions.samples) {
    issues.push(
      issue(
        "section-amplitude-length-mismatch",
        `${path}.section.amplitudes`,
        `Seismic section '${asset.id}' amplitude count must equal traces * samples.`
      )
    );
  }
  if (
    asset.wellTraceIndex !== undefined &&
    (asset.wellTraceIndex < 0 || asset.wellTraceIndex >= section.dimensions.traces)
  ) {
    issues.push(
      issue(
        "invalid-well-trace-index",
        `${path}.wellTraceIndex`,
        `Seismic section '${asset.id}' wellTraceIndex must be within the trace range.`
      )
    );
  }
  validateArrayLikeNumbers(asset.panelDepths, `${path}.panelDepths`, issues);
  validateArrayLikeNumbers(asset.nativeDepths, `${path}.nativeDepths`, issues);
  validateArrayLikeNumbers(section.horizontalAxis, `${path}.section.horizontalAxis`, issues);
  validateArrayLikeNumbers(section.sampleAxis, `${path}.section.sampleAxis`, issues);
  validateArrayLikeNumbers(section.amplitudes, `${path}.section.amplitudes`, issues);
}

function validateArrayLikeNumbers(
  values: ArrayLike<number> | undefined,
  path: string,
  issues: WellPanelValidationIssue[]
): void {
  if (!values) {
    return;
  }
  for (let index = 0; index < values.length; index += 1) {
    if (!Number.isFinite(values[index])) {
      issues.push(issue("invalid-number", `${path}[${index}]`, `Expected a finite number at '${path}[${index}]'.`));
      break;
    }
  }
}

function validateUniqueIds<T>(
  items: T[],
  getId: (item: T) => string,
  path: string,
  code: string,
  issues: WellPanelValidationIssue[]
): void {
  const seen = new Set<string>();
  items.forEach((item, index) => {
    const id = getId(item);
    if (seen.has(id)) {
      issues.push(issue(code, `${path}[${index}]`, `Duplicate id '${id}'.`));
      return;
    }
    seen.add(id);
  });
}

function validateLayerId(
  layerId: string,
  path: string,
  layerIds: Set<string>,
  issues: WellPanelValidationIssue[]
): void {
  if (layerIds.has(layerId)) {
    issues.push(issue("duplicate-layer-id", path, `Duplicate layer id '${layerId}'.`));
    return;
  }
  layerIds.add(layerId);
}

function validateFiniteNumber(
  value: number | null | undefined,
  path: string,
  issues: WellPanelValidationIssue[]
): void {
  if (!Number.isFinite(value)) {
    issues.push(issue("invalid-number", path, `Expected a finite number at '${path}'.`));
  }
}

function issue(code: string, path: string, message: string): WellPanelValidationIssue {
  return { code, path, message };
}
