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

export function adaptOphioliteWellPanelToChart(
  source: OphioliteResolvedWellPanelSource,
  layout: OphioliteWellPanelLayout,
  options: OphioliteWellPanelAdapterOptions = {}
): WellPanelModel {
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
        depthReference: row.depth_reference ?? null
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
