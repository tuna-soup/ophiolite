import { createContext, tick } from "svelte";
import { SvelteSet } from "svelte/reactivity";
import type {
  SectionHorizonOverlay as ChartSectionHorizonOverlay,
  SectionScalarOverlay as ChartSectionScalarOverlay,
  SectionWellOverlay as ChartSectionWellOverlay
} from "@ophiolite/charts-data-models";
import type { SeismicChartInteractionState, SeismicChartTool } from "@ophiolite/charts";
import type {
  BuildSurveyTimeDepthTransformRequest,
  DatasetRegistryEntry,
  DatasetSummary,
  ExportSegyResponse,
  ImportDatasetResponse,
  ImportedHorizonDescriptor,
  ImportSegyWithPlanResponse,
  ResolvedSurveyMapSourceDto,
  SegyGeometryCandidate,
  SegyGeometryOverride,
  SegyHeaderField,
  SegyHeaderValueType,
  SegyImportPlan,
  SectionAxis,
  SectionInteractionChanged,
  SectionHorizonOverlayView,
  SectionTimeDepthDiagnostics,
  SectionProbeChanged,
  SectionView,
  SectionViewportChanged,
  SurveyTimeDepthTransform3D,
  SurveyPreflightResponse,
  VelocityFunctionSource,
  VelocityQuantityKind,
  WorkspacePipelineEntry,
  WorkspaceSession
} from "@traceboost/seis-contracts";
import type {
  ResolveSectionWellOverlaysResponse,
  SectionWellOverlayRequestDto,
  WellTimeDepthModel1D
} from "@ophiolite/contracts";
import type {
  AcceptProjectWellTieRequest,
  AcceptProjectWellTieResponse,
  AnalyzeProjectWellTieRequest,
  ComputeProjectWellMarkerResidualRequest,
  ComputeProjectWellMarkerResidualResponse,
  ProjectWellTieAnalysisResponse,
  CompileProjectWellTimeDepthAuthoredModelRequest,
  DatasetExportCapabilitiesResponse,
  DiagnosticsEvent,
  DiagnosticsStatus,
  FrontendDiagnosticsEventRequest,
  HorizonSourceImportCanonicalDraft,
  ExportZarrResponse,
  HorizonImportCoordinateReferenceOptions,
  ImportProjectWellTimeDepthAssetRequest,
  CommitProjectWellTimeDepthImportRequest,
  ImportProjectWellTimeDepthModelRequest,
  ImportProjectWellTimeDepthModelResponse,
  ProjectWellTimeDepthImportCanonicalDraft,
  ProjectSurveyAssetDescriptor,
  ProjectWellMarkerDescriptor,
  ProjectWellMarkerHorizonResidualDescriptor,
  ProjectSurveyDisplayCompatibility,
  ProjectDisplayCoordinateReference,
  ProjectGeospatialSettings,
  ProjectWellboreDisplayCompatibility,
  ProjectWellTimeDepthAuthoredModelDescriptor,
  ProjectWellboreInventoryItem,
  ProjectWellOverlayInventoryResponse,
  ProjectWellTimeDepthModelDescriptor,
  ProjectWellTimeDepthObservationDescriptor,
  TransportResolvedSectionDisplayView,
  TransportSectionScalarOverlayView,
  TransportSectionTileView,
  TransportSectionView,
  TransportWindowedSectionView
} from "./bridge";
import {
  acceptProjectWellTie,
  analyzeProjectWellTie,
  compileProjectWellTimeDepthAuthoredModel,
  convertHorizonDomain,
  exportDatasetSegy,
  exportDatasetZarr,
  defaultImportStorePath,
  buildVelocityModelTransform,
  ensureDemoSurveyTimeDepthTransform,
  emitFrontendDiagnosticsEvent,
  fetchDepthConvertedSectionView,
  fetchResolvedSectionDisplay,
  fetchSectionTileView,
  fetchSectionView,
  getDatasetExportCapabilities,
  getDiagnosticsStatus,
  commitHorizonSourceImport,
  importDataset,
  importSegyWithPlan,
  importHorizonXyz,
  importProjectWellTimeDepthAsset,
  importProjectWellTimeDepthModel,
  importVelocityFunctionsModel,
  computeProjectWellMarkerResidual,
  commitProjectWellTimeDepthImport,
  listProjectWellOverlayInventory,
  listProjectSurveyHorizons,
  listProjectWellMarkerResidualInventory,
  listProjectWellTimeDepthInventory,
  loadHorizonAssets,
  loadProjectGeospatialSettings,
  loadVelocityModels,
  loadWorkspaceState,
  listenToDiagnosticsEvents,
  openDataset,
  preflightImport,
  readProjectWellTimeDepthModel,
  removeDatasetEntry,
  resolveProjectSurveyMap,
  resolveProjectSectionWellOverlays,
  resolveSurveyMap,
  saveProjectGeospatialSettings,
  saveWorkspaceSession,
  setProjectActiveWellTimeDepthModel,
  setActiveDatasetEntry,
  setDatasetNativeCoordinateReference,
  upsertDatasetEntry,
  setDiagnosticsVerbosity,
  validateSegyImportPlan
} from "./bridge";
import {
  confirmOverwriteSegy,
  confirmOverwriteStore,
  confirmOverwriteZarr,
  pickSegyExportPath,
  pickZarrExportPath
} from "./file-dialog";
import { buildWorkspaceCoordinateReferenceWarnings } from "./coordinate-reference-warnings";
import { shouldPromptForMissingNativeCoordinateReference } from "./missing-native-coordinate-reference-prompt";
import {
  describeProjectDisplayCompatibilityBlockingReasonCode,
  describeProjectSurveyDisplayCompatibility,
  describeProjectWellboreDisplayCompatibility,
  projectSurveyDisplayCompatibilityStatusLabel,
  projectWellboreDisplayCompatibilityStatusLabel
} from "./project-display-compatibility";
import {
  buildViewerSessionKey,
  buildViewportMemoryKey,
  resolveViewerResetReason,
  type ViewerDisplayDomain,
  type ViewerResetReason
} from "./viewer-session-keys";

type DisplaySectionView = SectionView | TransportSectionView | TransportWindowedSectionView;
type SectionDisplayDomain = "time" | "depth";
type SampleDataFidelity =
  | DatasetSummary["descriptor"]["sample_data_fidelity"]
  | SurveyPreflightResponse["sample_data_fidelity"];

const DEMO_SURVEY_3D_TRANSFORM_ID = "demo-survey-3d-transform";
const SECTION_WELL_OVERLAY_COLORS = [
  "#f97316",
  "#22c55e",
  "#38bdf8",
  "#facc15",
  "#f472b6",
  "#a78bfa"
] as const;
const PROJECT_SURVEY_SELECTION_GROUPS = [
  { key: "ready", label: "Ready" },
  { key: "degraded", label: "Degraded" },
  { key: "unavailable", label: "Unavailable" }
] as const;
const SECTION_TILE_CACHE_BUDGET_BYTES = 96 * 1024 * 1024;
const SECTION_TILE_BUCKET_TRACES = 256;
const SECTION_TILE_BUCKET_SAMPLES = 512;
const SECTION_TILE_HALO_FACTOR = 0.35;
const SECTION_TILE_VIEWPORT_DEBOUNCE_MS = 90;

interface SectionTileWindowRequest {
  traceRange: [number, number];
  sampleRange: [number, number];
  lod: number;
}

interface SectionTileCacheEntry {
  key: string;
  view: TransportWindowedSectionView;
  bytes: number;
  lastUsedAt: number;
}

interface SectionTileStats {
  viewportRequests: number;
  cacheHits: number;
  fetches: number;
  fetchErrors: number;
  prefetchRequests: number;
  prefetchCacheHits: number;
  prefetchErrors: number;
  evictions: number;
  cachedBytes: number;
}

export interface ViewerActivity {
  id: number;
  timestamp: string;
  source: "ui" | "backend" | "viewer";
  level: "info" | "warn" | "error";
  message: string;
  detail: string | null;
}

export interface ViewerModelOptions {
  tauriRuntime: boolean;
}

interface SetActiveDatasetNativeCoordinateReferenceResult {
  applied: boolean;
  requestedCoordinateReferenceId: string | null;
  requestedCoordinateReferenceName: string | null;
  effectiveCoordinateReferenceId: string | null;
  effectiveCoordinateReferenceName: string | null;
  exactMatch: boolean;
  error: string | null;
}

interface MissingNativeCoordinateReferencePromptState {
  storePath: string;
  datasetDisplayName: string;
  sourcePath: string | null;
  displayCoordinateReferenceId: string | null;
  displayCoordinateReferenceName: string | null;
  triggeredBy: "open" | "import";
}

interface OpenDatasetOptions {
  entryId?: string | null;
  displayName?: string | null;
  sourcePath?: string | null;
  sessionPipelines?: WorkspacePipelineEntry[] | null;
  activeSessionPipelineId?: string | null;
  makeActive?: boolean;
  loadSection?: boolean;
  promptForMissingNativeCoordinateReference?: boolean;
}

interface ImportDatasetOptions extends OpenDatasetOptions {
  inputPath?: string;
  outputStorePath?: string;
  reuseExistingStore?: boolean;
  geometryOverride?: SegyGeometryOverride | null;
}

interface ProjectWellTieDraftSeed {
  observationAssetId: string | null;
  sourceModelAssetId: string | null;
  tieName: string;
  tieStartMs: string;
  tieEndMs: string;
  searchRadiusM: string;
  summary: string | null;
}

interface GeometryOverrideDraft {
  inlineByte: string;
  inlineType: SegyHeaderValueType;
  crosslineByte: string;
  crosslineType: SegyHeaderValueType;
  thirdAxisByte: string;
  thirdAxisType: SegyHeaderValueType;
}

interface ImportGeometryRecoveryState {
  inputPath: string;
  outputStorePath: string;
  preflight: SurveyPreflightResponse;
  importOptions: ImportDatasetOptions;
  mode: "candidate" | "manual";
  selectedCandidateIndex: number;
  draft: GeometryOverrideDraft;
  working: boolean;
  error: string | null;
}

type DatasetExportFormat = "segy" | "zarr";

interface DatasetExportFormatState {
  selected: boolean;
  available: boolean;
  reason: string | null;
  path: string;
}

interface DatasetExportDialogState {
  entryId: string | null;
  displayName: string;
  storePath: string;
  working: boolean;
  error: string | null;
  formats: {
    segy: DatasetExportFormatState;
    zarr: DatasetExportFormatState;
  };
}

type ProjectSurveySelectionGroupKey = (typeof PROJECT_SURVEY_SELECTION_GROUPS)[number]["key"];
type ProjectWellboreSelectionGroupKey = (typeof PROJECT_SURVEY_SELECTION_GROUPS)[number]["key"];

interface ProjectSurveySelectionGroup {
  label: string;
  surveys: ProjectSurveyAssetDescriptor[];
}

interface ProjectWellboreSelectionGroup {
  label: string;
  wellbores: ProjectWellboreInventoryItem[];
}

function projectSurveySelectionGroupKey(
  survey: ProjectSurveyAssetDescriptor
): ProjectSurveySelectionGroupKey {
  if (survey.displayCompatibility.canResolveProjectMap) {
    return survey.displayCompatibility.transformStatus === "display_degraded"
      ? "degraded"
      : "ready";
  }
  return "unavailable";
}

function projectSurveyReadinessRank(survey: ProjectSurveyAssetDescriptor): number {
  if (survey.displayCompatibility.canResolveProjectMap) {
    return survey.displayCompatibility.transformStatus === "display_equivalent"
      ? 0
      : survey.displayCompatibility.transformStatus === "display_transformed"
        ? 1
        : 2;
  }
  return 3;
}

function pickPreferredProjectSurveyAssetId(surveys: ProjectSurveyAssetDescriptor[]): string {
  let preferredSurvey: ProjectSurveyAssetDescriptor | null = null;

  for (const survey of surveys) {
    if (!preferredSurvey || projectSurveyReadinessRank(survey) < projectSurveyReadinessRank(preferredSurvey)) {
      preferredSurvey = survey;
    }
  }

  return preferredSurvey?.assetId ?? "";
}

function projectWellboreSelectionGroupKey(
  wellbore: ProjectWellboreInventoryItem
): ProjectWellboreSelectionGroupKey {
  if (wellbore.displayCompatibility.canResolveProjectMap) {
    return wellbore.displayCompatibility.transformStatus === "display_degraded"
      ? "degraded"
      : "ready";
  }
  return "unavailable";
}

export type CompareCompatibilityReason =
  | "primary_unset"
  | "missing_store_path"
  | "missing_dataset"
  | "missing_geometry_descriptor"
  | "compare_family_mismatch"
  | "geometry_fingerprint_mismatch";

export interface CompareCandidate {
  entryId: string;
  displayName: string;
  storePath: string;
  datasetId: string | null;
  compareFamily: string | null;
  fingerprint: string | null;
  compatible: boolean;
  isPrimary: boolean;
  reason: CompareCompatibilityReason | null;
}

export interface ComparePoolState {
  primaryStorePath: string | null;
  primaryDatasetId: string | null;
  primaryLabel: string | null;
  compareFamily: string | null;
  fingerprint: string | null;
  candidates: CompareCandidate[];
  compatibleStorePaths: string[];
  compatibleSecondaryStorePaths: string[];
}

function timestampLabel(): string {
  return new Date().toLocaleTimeString("en-GB", { hour12: false });
}

function capEntries<T>(entries: T[], next: T, limit: number): T[] {
  return [next, ...entries].slice(0, limit);
}

function errorMessage(error: unknown, fallback: string): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return fallback;
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function nextAnimationFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function bytePayloadLength(bytes: Array<number> | Uint8Array | null | undefined): number {
  if (!bytes) {
    return 0;
  }
  return bytes instanceof Uint8Array ? bytes.byteLength : bytes.length;
}

function estimateSectionPayloadBytes(section: DisplaySectionView): number {
  return (
    bytePayloadLength(section.horizontal_axis_f64le) +
    bytePayloadLength(section.inline_axis_f64le) +
    bytePayloadLength(section.xline_axis_f64le) +
    bytePayloadLength(section.sample_axis_f32le) +
    bytePayloadLength(section.amplitudes_f32le)
  );
}

function isWindowedSectionView(section: DisplaySectionView | null): section is TransportWindowedSectionView {
  return Boolean(section && "window" in section && section.window && "logical_dimensions" in section);
}

function sectionLogicalDimensions(
  section: DisplaySectionView
): { traces: number; samples: number } {
  if (isWindowedSectionView(section)) {
    return section.logical_dimensions;
  }
  return { traces: section.traces, samples: section.samples };
}

function clampRange(start: number, end: number, total: number): [number, number] {
  const width = Math.max(1, Math.min(total, end - start));
  const clampedStart = Math.max(0, Math.min(start, Math.max(0, total - width)));
  return [clampedStart, clampedStart + width];
}

function expandAndSnapRange(
  start: number,
  end: number,
  total: number,
  bucket: number,
  haloFactor: number
): [number, number] {
  const width = Math.max(1, end - start);
  const halo = Math.max(1, Math.round(width * haloFactor));
  const expandedStart = Math.max(0, start - halo);
  const expandedEnd = Math.min(total, end + halo);
  const snappedStart = Math.max(0, Math.floor(expandedStart / bucket) * bucket);
  const snappedEnd = Math.min(total, Math.ceil(expandedEnd / bucket) * bucket);
  return clampRange(snappedStart, snappedEnd, total);
}

function chooseSectionTileLod(
  viewport: SectionViewportChanged["viewport"],
  chartWidthPx = 1600,
  chartHeightPx = 900
): number {
  const traceSpan = Math.max(1, viewport.trace_end - viewport.trace_start);
  const sampleSpan = Math.max(1, viewport.sample_end - viewport.sample_start);
  let lod = 0;
  while (
    lod < 6 &&
    (traceSpan / Math.max(1, chartWidthPx) > 1.35 * (1 << lod) ||
      sampleSpan / Math.max(1, chartHeightPx) > 1.35 * (1 << lod))
  ) {
    lod += 1;
  }
  return lod;
}

function buildSectionTileRequest(
  section: DisplaySectionView,
  viewport: SectionViewportChanged["viewport"]
): SectionTileWindowRequest {
  const logical = sectionLogicalDimensions(section);
  return {
    traceRange: expandAndSnapRange(
      viewport.trace_start,
      viewport.trace_end,
      logical.traces,
      SECTION_TILE_BUCKET_TRACES,
      SECTION_TILE_HALO_FACTOR
    ),
    sampleRange: expandAndSnapRange(
      viewport.sample_start,
      viewport.sample_end,
      logical.samples,
      SECTION_TILE_BUCKET_SAMPLES,
      SECTION_TILE_HALO_FACTOR
    ),
    lod: chooseSectionTileLod(viewport)
  };
}

function tileCacheKey(
  storePath: string,
  axis: SectionAxis,
  index: number,
  request: SectionTileWindowRequest
): string {
  return [
    storePath,
    axis,
    index,
    request.traceRange[0],
    request.traceRange[1],
    request.sampleRange[0],
    request.sampleRange[1],
    request.lod
  ].join(":");
}

function tileViewToWindowedSection(
  tile: TransportSectionTileView,
  logical: { traces: number; samples: number }
): TransportWindowedSectionView {
  return {
    ...tile.section,
    logical_dimensions: logical,
    window: {
      trace_start: tile.trace_range[0],
      trace_end: tile.trace_range[1],
      sample_start: tile.sample_range[0],
      sample_end: tile.sample_range[1],
      lod: tile.lod
    }
  };
}

function decodeF32Le(bytes: Array<number> | Uint8Array | null | undefined): Float32Array {
  if (!bytes) {
    return new Float32Array(0);
  }
  const source = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  if (source.byteLength % Float32Array.BYTES_PER_ELEMENT !== 0) {
    throw new Error(`Expected f32 little-endian bytes, found ${source.byteLength} bytes.`);
  }
  return new Float32Array(source.buffer.slice(source.byteOffset, source.byteOffset + source.byteLength));
}

function adaptSectionHorizonOverlays(
  overlays: SectionHorizonOverlayView[]
): ChartSectionHorizonOverlay[] {
  return overlays.map((overlay) => ({
    id: overlay.id,
    name: overlay.name ?? undefined,
    color: overlay.style.color,
    lineWidth: overlay.style.line_width ?? undefined,
    lineStyle: overlay.style.line_style,
    opacity: overlay.style.opacity ?? undefined,
    samples: overlay.samples.map((sample) => ({
      traceIndex: sample.trace_index,
      sampleIndex: sample.sample_index,
      sampleValue: sample.sample_value ?? undefined
    }))
  }));
}

function adaptSectionScalarOverlays(
  overlays: TransportSectionScalarOverlayView[],
  opacityOverride?: number
): ChartSectionScalarOverlay[] {
  return overlays.map((overlay) => ({
    id: overlay.id,
    name: overlay.name ?? undefined,
    width: overlay.width,
    height: overlay.height,
    values: decodeF32Le(overlay.values_f32le),
    colorMap: overlay.color_map,
    opacity: opacityOverride ?? overlay.opacity,
    valueRange: overlay.value_range,
    units: overlay.units ?? undefined
  }));
}

function adaptSectionWellOverlays(
  response: ResolveSectionWellOverlaysResponse
): ChartSectionWellOverlay[] {
  return response.overlays.map((overlay, overlayIndex) => ({
    id: overlay.wellbore_id,
    name: overlay.name || overlay.wellbore_id,
    color: SECTION_WELL_OVERLAY_COLORS[overlayIndex % SECTION_WELL_OVERLAY_COLORS.length]!,
    lineWidth: 2.5,
    lineStyle: overlay.display_domain === "time" ? "dashed" : "solid",
    opacity: 0.95,
    diagnostics: [...overlay.diagnostics],
    segments: overlay.segments.map((segment) => ({
      notes: [...segment.notes],
      samples: segment.samples.map((sample) => ({
        traceIndex: sample.trace_index,
        traceCoordinate: sample.trace_coordinate,
        sampleIndex: sample.sample_index ?? undefined,
        sampleValue: sample.sample_value ?? undefined,
        measuredDepthM: sample.measured_depth_m,
        trueVerticalDepthM: sample.true_vertical_depth_m ?? undefined,
        trueVerticalDepthSubseaM: sample.true_vertical_depth_subsea_m ?? undefined,
        twtMs: sample.twt_ms ?? undefined
      }))
    }))
  }));
}

function isExistingStoreError(message: string): boolean {
  return message.toLowerCase().includes("store root already exists:");
}

function isExistingSegyExportError(message: string): boolean {
  return message.toLowerCase().includes("output seg-y path already exists:");
}

function isExistingZarrExportError(message: string): boolean {
  return message.toLowerCase().includes("store root already exists:");
}

function describePreflight(preflight: SurveyPreflightResponse): string {
  const gather = preflight.gather_axis_kind ? `, gather axis ${preflight.gather_axis_kind}` : "";
  return `${preflight.classification} (${preflight.stacking_state}, ${preflight.layout}${gather})`;
}

function sampleTypeShort(sampleType: string | null | undefined): string {
  const normalized = sampleType?.trim();
  return normalized && normalized.length > 0 ? normalized : "unknown";
}

function sampleDataConversionLabel(
  conversion: SampleDataFidelity["conversion"] | null | undefined
): string {
  switch (conversion) {
    case "identity":
      return "native storage";
    case "format_transcode":
      return "format transcode";
    case "cast":
    default:
      return "numeric cast";
  }
}

function sampleDataFidelityNeedsWarning(fidelity: SampleDataFidelity | null | undefined): boolean {
  return fidelity?.preservation === "potentially_lossy";
}

function describeSampleDataFidelity(fidelity: SampleDataFidelity | null | undefined): string | null {
  if (!fidelity) {
    return null;
  }

  const source = sampleTypeShort(fidelity.source_sample_type);
  const working = sampleTypeShort(fidelity.working_sample_type);
  if (fidelity.conversion === "identity" && source === working) {
    return `${working} native`;
  }

  return `${source} -> ${working}`;
}

function describeSampleDataFidelityDetail(fidelity: SampleDataFidelity | null | undefined): string | null {
  if (!fidelity) {
    return null;
  }

  const source = sampleTypeShort(fidelity.source_sample_type);
  const working = sampleTypeShort(fidelity.working_sample_type);
  const preservation = sampleDataFidelityNeedsWarning(fidelity) ? "Potentially lossy" : "Exact";
  const notes = fidelity.notes.filter((note) => note.trim().length > 0);
  const base =
    fidelity.conversion === "identity" && source === working
      ? `Source samples stay ${working} in the working store.`
      : `Source samples import as ${working} from ${source} via ${sampleDataConversionLabel(fidelity.conversion)}.`;

  if (notes.length > 0) {
    return `${base} ${preservation} relative to the source values. ${notes.join(" ")}`;
  }

  return `${base} ${preservation} relative to the source values.`;
}

function canAutoImportPreflight(preflight: SurveyPreflightResponse): boolean {
  return (
    preflight.suggested_action === "direct_dense_ingest" ||
    preflight.suggested_action === "regularize_sparse_survey"
  );
}

function sameHeaderField(
  left: SegyHeaderField | null | undefined,
  right: SegyHeaderField | null | undefined
): boolean {
  if (!left && !right) {
    return true;
  }
  if (!left || !right) {
    return false;
  }
  return left.start_byte === right.start_byte && left.value_type === right.value_type;
}

function sameGeometryOverride(
  left: SegyGeometryOverride | null | undefined,
  right: SegyGeometryOverride | null | undefined
): boolean {
  if (!left && !right) {
    return true;
  }
  if (!left || !right) {
    return false;
  }
  return (
    sameHeaderField(left.inline_3d, right.inline_3d) &&
    sameHeaderField(left.crossline_3d, right.crossline_3d) &&
    sameHeaderField(left.third_axis, right.third_axis)
  );
}

function describeHeaderField(field: SegyHeaderField | null | undefined): string {
  if (!field) {
    return "unset";
  }
  return `${field.start_byte} (${field.value_type.toUpperCase()})`;
}

function describeGeometryOverride(geometry: SegyGeometryOverride | null | undefined): string {
  if (!geometry) {
    return "default SEG-Y mapping";
  }
  return `inline ${describeHeaderField(geometry.inline_3d)}, crossline ${describeHeaderField(geometry.crossline_3d)}`;
}

function geometryOverrideDraft(
  geometry: SegyGeometryOverride | null | undefined
): GeometryOverrideDraft {
  return {
    inlineByte: geometry?.inline_3d?.start_byte ? String(geometry.inline_3d.start_byte) : "",
    inlineType: geometry?.inline_3d?.value_type ?? "i32",
    crosslineByte: geometry?.crossline_3d?.start_byte ? String(geometry.crossline_3d.start_byte) : "",
    crosslineType: geometry?.crossline_3d?.value_type ?? "i32",
    thirdAxisByte: geometry?.third_axis?.start_byte ? String(geometry.third_axis.start_byte) : "",
    thirdAxisType: geometry?.third_axis?.value_type ?? "i32"
  };
}

function geometryOverrideFromDraft(draft: GeometryOverrideDraft): SegyGeometryOverride | null {
  const parseField = (startByteText: string, valueType: SegyHeaderValueType): SegyHeaderField | null => {
    const trimmed = startByteText.trim();
    if (!trimmed) {
      return null;
    }
    const parsed = Number.parseInt(trimmed, 10);
    if (!Number.isInteger(parsed) || parsed <= 0) {
      return null;
    }
    return { start_byte: parsed, value_type: valueType };
  };

  const geometry: SegyGeometryOverride = {
    inline_3d: parseField(draft.inlineByte, draft.inlineType),
    crossline_3d: parseField(draft.crosslineByte, draft.crosslineType),
    third_axis: parseField(draft.thirdAxisByte, draft.thirdAxisType)
  };

  if (!geometry.inline_3d && !geometry.crossline_3d && !geometry.third_axis) {
    return null;
  }
  return geometry;
}

function canRecoverPreflight(preflight: SurveyPreflightResponse): boolean {
  return Boolean(preflight.suggested_geometry_override) || preflight.geometry_candidates.length > 0;
}

function suggestedCandidateIndex(preflight: SurveyPreflightResponse): number {
  if (!preflight.suggested_geometry_override) {
    return preflight.geometry_candidates.length > 0 ? 0 : -1;
  }
  return preflight.geometry_candidates.findIndex((candidate) =>
    sameGeometryOverride(candidate.geometry, preflight.suggested_geometry_override)
  );
}

function trimPath(value: string): string {
  return value.trim();
}

function normalizeCoordinateReferenceId(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? "";
  return normalized || null;
}

function uniqueStringsInOrder(values: string[]): string[] {
  const uniqueValues: string[] = [];
  for (const value of values) {
    if (!uniqueValues.includes(value)) {
      uniqueValues.push(value);
    }
  }
  return uniqueValues;
}

function coordinateReferenceSelectionId(selection: ProjectDisplayCoordinateReference): string | null {
  if (selection.kind !== "authority_code") {
    return null;
  }
  return normalizeCoordinateReferenceId(selection.authId);
}

function projectDisplaySelectionFromCoordinateReferenceId(
  coordinateReferenceId: string | null,
  name: string | null = null
): ProjectDisplayCoordinateReference | null {
  const normalizedCoordinateReferenceId = normalizeCoordinateReferenceId(coordinateReferenceId);
  if (!normalizedCoordinateReferenceId) {
    return null;
  }
  const [authority, code] = normalizedCoordinateReferenceId.split(":", 2);
  if (!authority || !code) {
    return null;
  }
  return {
    kind: "authority_code",
    authority: authority.trim().toUpperCase(),
    code: code.trim(),
    authId: normalizedCoordinateReferenceId.trim().toUpperCase(),
    name: name?.trim() || null
  };
}

function deriveStorePathFromInput(inputPath: string): string {
  const normalizedPath = trimPath(inputPath);
  if (!normalizedPath) {
    return "";
  }

  const separatorIndex = Math.max(normalizedPath.lastIndexOf("/"), normalizedPath.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalizedPath.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalizedPath.slice(separatorIndex + 1) : normalizedPath;
  const basename = filename.replace(/\.[^.]+$/, "");
  if (!basename) {
    return "";
  }

  return `${directory}${basename}.tbvol`;
}

function deriveSegyExportPathFromStore(storePath: string): string {
  const normalizedPath = trimPath(storePath);
  if (!normalizedPath) {
    return "";
  }

  const separatorIndex = Math.max(normalizedPath.lastIndexOf("/"), normalizedPath.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalizedPath.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalizedPath.slice(separatorIndex + 1) : normalizedPath;
  const basename = filename.replace(/\.[^.]+$/, "");
  if (!basename) {
    return "";
  }

  return `${directory}${basename}.export.sgy`;
}

function deriveZarrExportPathFromStore(storePath: string): string {
  const normalizedPath = trimPath(storePath);
  if (!normalizedPath) {
    return "";
  }

  const separatorIndex = Math.max(normalizedPath.lastIndexOf("/"), normalizedPath.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalizedPath.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalizedPath.slice(separatorIndex + 1) : normalizedPath;
  const basename = filename.replace(/\.[^.]+$/, "");
  if (!basename) {
    return "";
  }

  return `${directory}${basename}.export.zarr`;
}

function fileExtension(filePath: string): string {
  const normalized = trimPath(filePath);
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  const filename = separatorIndex >= 0 ? normalized.slice(separatorIndex + 1) : normalized;
  const extensionIndex = filename.lastIndexOf(".");
  return extensionIndex >= 0 ? filename.slice(extensionIndex).toLowerCase() : "";
}

function isSegyVolumeExtension(extension: string): boolean {
  return extension === ".sgy" || extension === ".segy";
}

function isDirectImportVolumeExtension(extension: string): boolean {
  return extension === ".zarr" || extension === ".mdio";
}

function isSupportedImportVolumeExtension(extension: string): boolean {
  return isSegyVolumeExtension(extension) || isDirectImportVolumeExtension(extension);
}

function describeImportVolumeType(extension: string): string {
  switch (extension) {
    case ".mdio":
      return "MDIO store";
    case ".zarr":
      return "Zarr store";
    case ".sgy":
    case ".segy":
      return "SEG-Y survey";
    default:
      return "source volume";
  }
}

function fileStem(filePath: string | null | undefined): string {
  const normalized = trimPath(filePath ?? "");
  if (!normalized) {
    return "";
  }
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  const filename = separatorIndex >= 0 ? normalized.slice(separatorIndex + 1) : normalized;
  return filename.replace(/\.[^.]+$/, "");
}

function stripGeneratedHashSuffix(value: string): string {
  return value.replace(/-[0-9a-f]{16}$/i, "");
}

function userVisibleDatasetName(
  displayName: string | null | undefined,
  sourcePath: string | null | undefined,
  storePath: string | null | undefined,
  fallbackId: string
): string {
  const trimmedDisplayName = trimPath(displayName ?? "");
  if (trimmedDisplayName) {
    return stripGeneratedHashSuffix(trimmedDisplayName);
  }

  const sourceStem = fileStem(sourcePath);
  if (sourceStem) {
    return sourceStem;
  }

  const storeStem = fileStem(storePath);
  if (storeStem) {
    return stripGeneratedHashSuffix(storeStem);
  }

  return fallbackId;
}

function nextDuplicateName(sourceName: string, existingNames: string[]): string {
  const trimmedSourceName = sourceName.trim() || "Dataset";
  const sourceMatch = /^(.*?)(?:_(\d+))?$/.exec(trimmedSourceName);
  const baseName = sourceMatch?.[1]?.trim() || trimmedSourceName;
  const lowerBaseName = baseName.toLowerCase();
  let maxSuffix = 0;

  for (const existingName of existingNames) {
    const trimmedExistingName = existingName.trim();
    if (!trimmedExistingName) {
      continue;
    }

    const existingMatch = /^(.*?)(?:_(\d+))?$/.exec(trimmedExistingName);
    const existingBaseName = existingMatch?.[1]?.trim() || trimmedExistingName;
    if (existingBaseName.toLowerCase() !== lowerBaseName) {
      continue;
    }

    const suffix = existingMatch?.[2] ? Number(existingMatch[2]) : 0;
    if (Number.isFinite(suffix)) {
      maxSuffix = Math.max(maxSuffix, suffix);
    }
  }

  return `${baseName}_${maxSuffix + 1}`;
}

function sortWorkspaceEntries(entries: DatasetRegistryEntry[]): DatasetRegistryEntry[] {
  return [...entries].sort((left, right) => {
    const leftName = userVisibleDatasetName(
      left.display_name,
      left.source_path,
      left.imported_store_path ?? left.preferred_store_path,
      left.entry_id
    );
    const rightName = userVisibleDatasetName(
      right.display_name,
      right.source_path,
      right.imported_store_path ?? right.preferred_store_path,
      right.entry_id
    );
    const byName = leftName.localeCompare(rightName, undefined, { sensitivity: "base", numeric: true });
    if (byName !== 0) {
      return byName;
    }
    return left.entry_id.localeCompare(right.entry_id, undefined, { sensitivity: "base", numeric: true });
  });
}

function mergeWorkspaceEntry(
  entries: DatasetRegistryEntry[],
  nextEntry: DatasetRegistryEntry
): DatasetRegistryEntry[] {
  const nextEntries = entries.filter((entry) => entry.entry_id !== nextEntry.entry_id);
  nextEntries.push(nextEntry);
  return sortWorkspaceEntries(nextEntries);
}

function entryStorePath(entry: DatasetRegistryEntry | null): string {
  return entry?.last_dataset?.store_path ?? entry?.imported_store_path ?? entry?.preferred_store_path ?? "";
}

function cloneSessionPipelines(
  entries: WorkspacePipelineEntry[] | null | undefined
): WorkspacePipelineEntry[] | null {
  return entries ? structuredClone(entries) : null;
}

function datasetCompareFamily(dataset: DatasetSummary | null): string | null {
  return dataset?.descriptor.geometry?.compare_family ?? null;
}

function datasetGeometryFingerprint(dataset: DatasetSummary | null): string | null {
  return dataset?.descriptor.geometry?.fingerprint ?? null;
}

function cloneViewport(
  viewport: SectionViewportChanged["viewport"]
): SectionViewportChanged["viewport"] {
  return {
    trace_start: viewport.trace_start,
    trace_end: viewport.trace_end,
    sample_start: viewport.sample_start,
    sample_end: viewport.sample_end
  };
}

function compareCandidateReason(
  primary: DatasetSummary | null,
  candidate: DatasetSummary | null,
  candidateStorePath: string,
  isPrimary: boolean
): CompareCompatibilityReason | null {
  if (isPrimary) {
    return null;
  }

  if (!primary) {
    return "primary_unset";
  }

  if (!candidateStorePath) {
    return "missing_store_path";
  }

  if (!candidate) {
    return "missing_dataset";
  }

  const primaryGeometry = primary.descriptor.geometry;
  const candidateGeometry = candidate.descriptor.geometry;

  if (!primaryGeometry || !candidateGeometry) {
    return "missing_geometry_descriptor";
  }

  if (candidateGeometry.compare_family !== primaryGeometry.compare_family) {
    return "compare_family_mismatch";
  }

  if (candidateGeometry.fingerprint !== primaryGeometry.fingerprint) {
    return "geometry_fingerprint_mismatch";
  }

  return null;
}

export class ViewerModel {
  readonly tauriRuntime: boolean;

  inputPath = $state("");
  outputStorePath = $state("");
  activeStorePath = $state("");
  dataset = $state<DatasetSummary | null>(null);
  preflight = $state<SurveyPreflightResponse | null>(null);
  importGeometryRecovery = $state.raw<ImportGeometryRecoveryState | null>(null);
  datasetExportDialog = $state.raw<DatasetExportDialogState | null>(null);
  axis = $state<SectionAxis>("inline");
  index = $state(0);
  sectionDomain = $state<SectionDisplayDomain>("time");
  activeVelocityModelAssetId = $state<string | null>(null);
  depthVelocityMPerS = $state(2200);
  depthVelocityKind = $state<VelocityQuantityKind>("average");
  availableVelocityModels = $state.raw<SurveyTimeDepthTransform3D[]>([]);
  section = $state.raw<DisplaySectionView | null>(null);
  timeDepthDiagnostics = $state.raw<SectionTimeDepthDiagnostics | null>(null);
  sectionScalarOverlays = $state.raw<ChartSectionScalarOverlay[]>([]);
  sectionHorizons = $state.raw<ChartSectionHorizonOverlay[]>([]);
  sectionWellOverlays = $state.raw<ChartSectionWellOverlay[]>([]);
  importedHorizons = $state.raw<ImportedHorizonDescriptor[]>([]);
  projectSurveyHorizons = $state.raw<ImportedHorizonDescriptor[]>([]);
  projectWellMarkers = $state.raw<ProjectWellMarkerDescriptor[]>([]);
  projectResidualAssets = $state.raw<ProjectWellMarkerHorizonResidualDescriptor[]>([]);
  backgroundSection = $state.raw<DisplaySectionView | null>(null);
  showVelocityOverlay = $state(false);
  velocityOverlayOpacity = $state(0.52);
  velocityModelWorkbenchOpen = $state(false);
  velocityModelWorkbenchBuilding = $state(false);
  velocityModelWorkbenchError = $state<string | null>(null);
  residualWorkbenchOpen = $state(false);
  residualWorkbenchWorking = $state(false);
  residualWorkbenchError = $state<string | null>(null);
  depthConversionWorkbenchOpen = $state(false);
  depthConversionWorkbenchWorking = $state(false);
  depthConversionWorkbenchError = $state<string | null>(null);
  projectSettingsOpen = $state(false);
  wellTieWorkbenchOpen = $state(false);
  wellTieWorkbenchError = $state<string | null>(null);
  velocityModelsLoading = $state(false);
  loading = $state(false);
  backgroundLoading = $state(false);
  horizonImporting = $state(false);
  datasetExporting = $state(false);
  busyLabel = $state<string | null>(null);
  error = $state<string | null>(null);
  backgroundError = $state<string | null>(null);
  velocityModelsError = $state<string | null>(null);
  resetToken = $state("inline:0");
  displayTransform = $state({
    renderMode: "heatmap" as "heatmap" | "wiggle",
    colormap: "grayscale" as "grayscale" | "red-white-blue",
    gain: 1,
    polarity: "normal" as "normal" | "reversed",
    clipMin: undefined as number | undefined,
    clipMax: undefined as number | undefined
  });
  chartTool = $state<SeismicChartTool>("crosshair");
  lastProbe = $state<SectionProbeChanged | null>(null);
  lastInteraction = $state<SectionInteractionChanged | null>(null);
  displayStorePath = $state("");
  displayGeometryFingerprint = $state<string | null>(null);
  displayAxis = $state<SectionAxis>("inline");
  displayIndex = $state(0);
  displayDomain = $state<SectionDisplayDomain>("time");
  viewportMemoryRevision = $state(0);
  diagnosticsStatus = $state<DiagnosticsStatus | null>(null);
  verboseDiagnostics = $state(false);
  backendEvents = $state<DiagnosticsEvent[]>([]);
  recentActivity = $state<ViewerActivity[]>([]);
  lastImportedInputPath = $state("");
  lastImportedStorePath = $state("");
  workspaceEntries = $state.raw<DatasetRegistryEntry[]>([]);
  activeEntryId = $state<string | null>(null);
  selectedPresetId = $state<string | null>(null);
  displayCoordinateReferenceId = $state<string | null>(null);
  projectDisplayCoordinateReferenceMode = $state<"native_engineering" | "authority_code">(
    "native_engineering"
  );
  projectDisplayCoordinateReferenceIdDraft = $state("");
  projectGeospatialSettingsResolved = $state(true);
  projectGeospatialSettingsSource = $state<string | null>("temporary_workspace");
  projectGeospatialSettingsLoading = $state(false);
  projectGeospatialSettingsSaving = $state(false);
  surveyMapSource = $state.raw<ResolvedSurveyMapSourceDto | null>(null);
  surveyMapLoading = $state(false);
  surveyMapError = $state<string | null>(null);
  projectWellOverlayInventory = $state.raw<ProjectWellOverlayInventoryResponse | null>(null);
  projectWellOverlayInventoryLoading = $state(false);
  projectWellOverlayInventoryError = $state<string | null>(null);
  projectWellTimeDepthObservationSets = $state.raw<ProjectWellTimeDepthObservationDescriptor[]>([]);
  projectWellTimeDepthAuthoredModels = $state.raw<ProjectWellTimeDepthAuthoredModelDescriptor[]>([]);
  projectWellTimeDepthModels = $state.raw<ProjectWellTimeDepthModelDescriptor[]>([]);
  projectWellTimeDepthModelsLoading = $state(false);
  projectWellTimeDepthModelsError = $state<string | null>(null);
  projectSectionWellOverlays = $state.raw<ResolveSectionWellOverlaysResponse | null>(null);
  projectSectionWellOverlaysLoading = $state(false);
  projectSectionWellOverlaysError = $state<string | null>(null);
  projectRoot = $state("");
  projectSurveyAssetId = $state("");
  projectWellboreId = $state("");
  projectSectionToleranceM = $state(12.5);
  selectedProjectWellTimeDepthModelAssetId = $state<string | null>(null);
  selectedProjectWellTieObservationAssetId = $state<string | null>(null);
  selectedProjectWellMarkerName = $state("");
  selectedProjectResidualAssetId = $state<string | null>(null);
  selectedProjectHorizonId = $state("");
  projectWellTieDraftSeed = $state.raw<ProjectWellTieDraftSeed | null>(null);
  projectWellTieDraftSeedNonce = $state(0);
  nativeCoordinateReferenceOverrideIdDraft = $state("");
  nativeCoordinateReferenceOverrideNameDraft = $state("");
  missingNativeCoordinateReferencePrompt =
    $state.raw<MissingNativeCoordinateReferencePromptState | null>(null);
  workspaceReady = $state(false);
  restoringWorkspace = $state(false);
  compareBackgroundStorePath = $state<string | null>(null);
  compareSplitEnabled = $state(false);
  compareSplitPosition = $state(0.5);
  sectionTileStats = $state<SectionTileStats>({
    viewportRequests: 0,
    cacheHits: 0,
    fetches: 0,
    fetchErrors: 0,
    prefetchRequests: 0,
    prefetchCacheHits: 0,
    prefetchErrors: 0,
    evictions: 0,
    cachedBytes: 0
  });

  #activityCounter = 0;
  #diagnosticsUnlisten: (() => void) | null = null;
  #outputPathSource: "auto" | "manual" = "auto";
  #backgroundLoadRequestId = 0;
  #backgroundSectionKey: string | null = null;
  #sectionTileViewportTimer: ReturnType<typeof setTimeout> | null = null;
  #sectionTileLoadRequestId = 0;
  #sectionTilePrefetchRequestId = 0;
  #sectionTileCache = new Map<string, SectionTileCacheEntry>();
  #sectionTileCacheBytes = 0;
  #viewportMemory = new Map<string, SectionViewportChanged["viewport"]>();
  #surveyMapRequestId = 0;
  #projectWellOverlayInventoryRequestId = 0;
  #projectWellTimeDepthModelsRequestId = 0;
  #projectSurveyHorizonsRequestId = 0;
  #projectWellMarkerResidualInventoryRequestId = 0;
  #copiedWorkspaceEntry: DatasetRegistryEntry | null = null;
  #workspaceEntryCounter = 0;
  #acceptedNativeEngineeringStorePaths = new SvelteSet<string>();
  #lastCoordinateReferenceWarningSignature = "";

  constructor(options: ViewerModelOptions) {
    this.tauriRuntime = options.tauriRuntime;

    $effect(() => {
      const backgroundStorePath = this.compareBackgroundStorePath;
      const foregroundStorePath = this.comparePrimaryStorePath;
      const axis = this.axis;
      const index = this.index;
      const sectionDomain = this.sectionDomain;
      const activeVelocityModelAssetId = this.activeVelocityModelAssetId;
      const depthVelocityMPerS = this.depthVelocityMPerS;
      const depthVelocityKind = this.depthVelocityKind;

      if (!backgroundStorePath || !foregroundStorePath || backgroundStorePath === foregroundStorePath) {
        this.backgroundSection = null;
        this.backgroundError = null;
        this.backgroundLoading = false;
        this.#backgroundSectionKey = null;
        return;
      }

      const nextKey = `${backgroundStorePath}:${axis}:${index}:${sectionDomain}:${activeVelocityModelAssetId ?? "global1d"}:${depthVelocityKind}:${depthVelocityMPerS}`;
      if (this.#backgroundSectionKey === nextKey) {
        return;
      }

      void this.loadBackgroundSection(backgroundStorePath, axis, index);
    });

    $effect(() => {
      const splitAllowed =
        this.compareSplitEnabled &&
        !!this.activeBackgroundCompareCandidate &&
        this.displayTransform.renderMode === "heatmap";

      if (!splitAllowed && this.compareSplitEnabled) {
        this.compareSplitEnabled = false;
      }
    });

    $effect(() => {
      const storePath = trimPath(this.comparePrimaryStorePath ?? this.activeStorePath) || null;
      const warnings = this.workspaceCoordinateReferenceWarnings;
      const signature = JSON.stringify({ storePath, warnings });
      if (signature === this.#lastCoordinateReferenceWarningSignature) {
        return;
      }
      const previousSignature = this.#lastCoordinateReferenceWarningSignature;
      this.#lastCoordinateReferenceWarningSignature = signature;
      if (!this.tauriRuntime || !previousSignature) {
        return;
      }
      if (warnings.length > 0) {
        this.#emitCoordinateReferenceLifecycleDiagnostics(
          "warn",
          "Workspace CRS warning state changed.",
          {
            event: "crs_warning_emitted",
            storePath,
            warningCount: warnings.length,
            warnings
          }
        );
        return;
      }
      this.#emitCoordinateReferenceLifecycleDiagnostics("info", "Workspace CRS warnings cleared.", {
        event: "crs_warning_cleared",
        storePath
      });
    });
  }

  #nextActivityId(): number {
    this.#activityCounter += 1;
    return this.#activityCounter;
  }

  note = (
    message: string,
    source: ViewerActivity["source"] = "ui",
    level: ViewerActivity["level"] = "info",
    detail: string | null = null
  ): void => {
    this.recentActivity = capEntries(
      this.recentActivity,
      {
        id: this.#nextActivityId(),
        timestamp: timestampLabel(),
        source,
        level,
        message,
        detail
      },
      24
    );
  };

  #sectionTileDetail(request: SectionTileWindowRequest, sectionIndex: number = this.index): string {
    return `${this.axis}:${sectionIndex} T[${request.traceRange[0]}, ${request.traceRange[1]}) S[${request.sampleRange[0]}, ${request.sampleRange[1]}) LOD ${request.lod}`;
  }

  #sectionTileFields(
    request: SectionTileWindowRequest,
    fields: Record<string, unknown> = {},
    sectionIndex: number = this.index
  ): Record<string, unknown> {
    const viewport = this.displayedViewport;
    return {
      storePath: this.displayStorePath || this.activeStorePath,
      axis: this.displayAxis,
      index: sectionIndex,
      traceRange: request.traceRange,
      sampleRange: request.sampleRange,
      lod: request.lod,
      viewportTraceRange: viewport ? [viewport.trace_start, viewport.trace_end] : null,
      viewportSampleRange: viewport ? [viewport.sample_start, viewport.sample_end] : null,
      cacheBytes: this.#sectionTileCacheBytes,
      cacheHits: this.sectionTileStats.cacheHits,
      fetches: this.sectionTileStats.fetches,
      prefetchRequests: this.sectionTileStats.prefetchRequests,
      evictions: this.sectionTileStats.evictions,
      ...fields
    };
  }

  #emitSectionTileDiagnostics(
    level: "debug" | "info" | "warn" | "error",
    message: string,
    request: SectionTileWindowRequest,
    fields: Record<string, unknown> = {},
    options: {
      sectionIndex?: number;
      mirrorToActivity?: boolean;
    } = {}
  ): void {
    if (!this.tauriRuntime) {
      return;
    }
    const sectionIndex = options.sectionIndex ?? this.index;
    const detail = this.#sectionTileDetail(request, sectionIndex);
    if (options.mirrorToActivity) {
      this.note(
        message,
        "backend",
        level === "error" ? "error" : level === "warn" ? "warn" : "info",
        detail
      );
    }
    void emitFrontendDiagnosticsEvent({
      stage: "section_tile",
      level,
      message,
      fields: this.#sectionTileFields(request, fields, sectionIndex)
    }).catch((error) => {
      console.warn("Failed to record section tile diagnostics.", error);
    });
  }

  #displayResetToken(): string {
    return [
      trimPath(this.activeStorePath) || "no-store",
      this.sectionDomain,
      this.activeVelocityModelAssetId ?? "global1d",
      this.depthVelocityKind,
      Math.round(this.depthVelocityMPerS),
      this.showVelocityOverlay ? "overlay-on" : "overlay-off"
    ].join(":");
  }

  #currentDisplayedIdentity(): {
    storePath: string;
    geometryFingerprint: string | null;
    domain: ViewerDisplayDomain;
  } | null {
    const storePath = trimPath(this.displayStorePath);
    if (!storePath) {
      return null;
    }
    return {
      storePath,
      geometryFingerprint: this.displayGeometryFingerprint,
      domain: this.displayDomain
    };
  }

  #rememberDisplayedViewport(viewport: SectionViewportChanged["viewport"]): void {
    const key = this.displayedViewportMemoryKey;
    if (!key) {
      return;
    }
    this.#viewportMemory.set(key, cloneViewport(viewport));
    this.viewportMemoryRevision += 1;
  }

  #applyDisplayedSectionContext(
    storePath: string,
    section: DisplaySectionView,
    domain: SectionDisplayDomain
  ): ViewerResetReason | null {
    const nextIdentity = {
      storePath,
      geometryFingerprint: datasetGeometryFingerprint(this.dataset),
      domain
    } satisfies {
      storePath: string;
      geometryFingerprint: string | null;
      domain: ViewerDisplayDomain;
    };
    const reason = resolveViewerResetReason(this.#currentDisplayedIdentity(), nextIdentity);
    this.displayStorePath = storePath;
    this.displayGeometryFingerprint = nextIdentity.geometryFingerprint;
    this.displayAxis = section.axis;
    this.displayIndex = section.coordinate.index;
    this.displayDomain = domain;
    return reason;
  }

  #evictSectionTileCacheForStore(storePath: string): void {
    const normalizedStorePath = trimPath(storePath);
    if (!normalizedStorePath) {
      return;
    }
    const prefix = `${normalizedStorePath}:`;
    let freedBytes = 0;
    let removedEntries = 0;
    for (const [key, entry] of this.#sectionTileCache.entries()) {
      if (!key.startsWith(prefix)) {
        continue;
      }
      this.#sectionTileCache.delete(key);
      this.#sectionTileCacheBytes -= entry.bytes;
      freedBytes += entry.bytes;
      removedEntries += 1;
    }
    if (removedEntries === 0) {
      return;
    }
    this.sectionTileStats.cachedBytes = this.#sectionTileCacheBytes;
    if (!this.tauriRuntime) {
      return;
    }
    void emitFrontendDiagnosticsEvent({
      stage: "section_tile",
      level: "debug",
      message: "Evicted cached section tiles for an inactive store.",
      fields: {
        storePath: normalizedStorePath,
        removedEntries,
        freedBytes,
        cacheBytes: this.#sectionTileCacheBytes
      }
    }).catch((error) => {
      console.warn("Failed to record section tile cache eviction diagnostics.", error);
    });
  }

  #notePotentiallyLossySampleData(
    fidelity: SampleDataFidelity | null | undefined,
    context: "preflight" | "dataset"
  ): void {
    if (!sampleDataFidelityNeedsWarning(fidelity)) {
      return;
    }

    const detail = describeSampleDataFidelityDetail(fidelity);
    this.note(
      context === "preflight"
        ? "Preflight detected a potentially lossy sample conversion."
        : "Dataset uses a potentially lossy source-to-working sample conversion.",
      "backend",
      "warn",
      detail
    );
  }

  get activeDatasetEntry(): DatasetRegistryEntry | null {
    return this.workspaceEntries.find((entry) => entry.entry_id === this.activeEntryId) ?? null;
  }

  get activeDatasetDisplayName(): string {
    const activeEntry = this.activeDatasetEntry;
    return userVisibleDatasetName(
      activeEntry?.display_name ?? this.dataset?.descriptor.label ?? null,
      activeEntry?.source_path ?? null,
      activeEntry?.imported_store_path ?? activeEntry?.preferred_store_path ?? this.activeStorePath ?? null,
      activeEntry?.entry_id ?? this.dataset?.descriptor.id ?? "dataset"
    );
  }

  get sectionTileStatsSnapshot(): SectionTileStats {
    return { ...this.sectionTileStats };
  }

  get displayedViewport(): SectionViewportChanged["viewport"] | null {
    this.viewportMemoryRevision;
    const key = this.displayedViewportMemoryKey;
    return key ? this.#viewportMemory.get(key) ?? null : null;
  }

  get displayedViewportMemoryKey(): string | null {
    const storePath = trimPath(this.displayStorePath);
    if (!storePath) {
      return null;
    }
    return buildViewportMemoryKey({
      storePath,
      geometryFingerprint: this.displayGeometryFingerprint,
      axis: this.displayAxis,
      domain: this.displayDomain
    });
  }

  get displayedViewerSessionKey(): string {
    return buildViewerSessionKey({
      storePath: trimPath(this.displayStorePath),
      geometryFingerprint: this.displayGeometryFingerprint,
      domain: this.displayDomain
    });
  }

  get displayedViewId(): string {
    return `${this.displayedViewerSessionKey}:${this.displayAxis}:${this.displayIndex}`;
  }

  get datasetSampleDataFidelity(): SampleDataFidelity | null {
    return this.comparePrimaryDataset?.descriptor.sample_data_fidelity ?? null;
  }

  get datasetSampleDataFidelityLabel(): string | null {
    return describeSampleDataFidelity(this.datasetSampleDataFidelity);
  }

  get datasetSampleDataFidelityDetail(): string | null {
    return describeSampleDataFidelityDetail(this.datasetSampleDataFidelity);
  }

  get datasetSampleDataFidelityNeedsWarning(): boolean {
    return sampleDataFidelityNeedsWarning(this.datasetSampleDataFidelity);
  }

  preflightSampleDataFidelityLabel(preflight: SurveyPreflightResponse | null | undefined): string | null {
    return describeSampleDataFidelity(preflight?.sample_data_fidelity);
  }

  preflightSampleDataFidelityDetail(
    preflight: SurveyPreflightResponse | null | undefined
  ): string | null {
    return describeSampleDataFidelityDetail(preflight?.sample_data_fidelity);
  }

  preflightSampleDataFidelityNeedsWarning(preflight: SurveyPreflightResponse | null | undefined): boolean {
    return sampleDataFidelityNeedsWarning(preflight?.sample_data_fidelity);
  }

  get canOpenExportDialog(): boolean {
    return this.tauriRuntime && !!trimPath(this.activeStorePath) && !!this.dataset && !this.datasetExporting;
  }

  get activeVelocityModel(): VelocityFunctionSource | null {
    if (this.activeVelocityModelAssetId) {
      return {
        velocity_asset_reference: {
          asset_id: this.activeVelocityModelAssetId
        }
      };
    }

    return {
      constant_velocity: {
        velocity_m_per_s: this.depthVelocityMPerS
      }
    };
  }

  get canDisplayDepthSection(): boolean {
    if (!this.tauriRuntime || !trimPath(this.activeStorePath)) {
      return false;
    }

    if (this.activeVelocityModelAssetId) {
      return true;
    }

    return Number.isFinite(this.depthVelocityMPerS) && this.depthVelocityMPerS >= 1;
  }

  get activeVelocityModelDescriptor(): SurveyTimeDepthTransform3D | null {
    if (!this.activeVelocityModelAssetId) {
      return null;
    }
    return (
      this.availableVelocityModels.find((model) => model.id === this.activeVelocityModelAssetId) ?? null
    );
  }

  get canDisplayVelocityOverlay(): boolean {
    return this.tauriRuntime && !!trimPath(this.activeStorePath);
  }

  get availableHorizonAssets(): ImportedHorizonDescriptor[] {
    return this.importedHorizons;
  }

  get projectSurveyHorizonAssets(): ImportedHorizonDescriptor[] {
    return this.projectSurveyHorizons;
  }

  get depthConversionHorizonAssets(): ImportedHorizonDescriptor[] {
    return this.importedHorizons;
  }

  get selectedProjectHorizonAsset(): ImportedHorizonDescriptor | null {
    const selectedProjectHorizonId = trimPath(this.selectedProjectHorizonId);
    if (!selectedProjectHorizonId) {
      return null;
    }
    return (
      this.projectSurveyHorizons.find((horizon) => horizon.id === selectedProjectHorizonId) ?? null
    );
  }

  get selectedProjectWellMarker(): ProjectWellMarkerDescriptor | null {
    const selectedProjectWellMarkerName = trimPath(this.selectedProjectWellMarkerName);
    if (!selectedProjectWellMarkerName) {
      return null;
    }
    return (
      this.projectWellMarkers.find(
        (marker) => marker.name.trim() === selectedProjectWellMarkerName
      ) ?? null
    );
  }

  get selectedProjectResidualAsset(): ProjectWellMarkerHorizonResidualDescriptor | null {
    if (!this.selectedProjectResidualAssetId) {
      return null;
    }
    return (
      this.projectResidualAssets.find((asset) => asset.assetId === this.selectedProjectResidualAssetId) ??
      null
    );
  }

  get canOpenDepthConversionWorkbench(): boolean {
    return this.depthConversionBlocker === null;
  }

  get residualWorkbenchBlocker(): string | null {
    if (!this.tauriRuntime) {
      return "Residual computation is only available in the desktop runtime.";
    }
    if (!trimPath(this.projectRoot)) {
      return "Set the project root before computing residuals.";
    }
    if (!trimPath(this.projectSurveyAssetId)) {
      return "Select a project survey before computing residuals.";
    }
    if (!trimPath(this.projectWellboreId)) {
      return "Select a project wellbore before computing residuals.";
    }
    if (this.projectSurveyHorizons.length === 0) {
      return "Selected project survey does not have imported depth horizons.";
    }
    if (!this.selectedProjectHorizonAsset) {
      return "Select a project survey horizon before computing residuals.";
    }
    if (this.selectedProjectHorizonAsset.vertical_domain !== "depth") {
      return "Residual computation requires a depth-domain horizon.";
    }
    if (this.projectWellMarkers.length === 0) {
      return "Selected wellbore does not have canonical markers.";
    }
    if (!this.selectedProjectWellMarker) {
      return "Select a canonical well marker before computing residuals.";
    }
    return null;
  }

  get canOpenResidualWorkbench(): boolean {
    return this.residualWorkbenchBlocker === null;
  }

  get canResolveConfiguredProjectSectionWellOverlays(): boolean {
    return this.projectSectionWellOverlayResolveBlocker === null;
  }

  get canPrepareProjectWellTie(): boolean {
    return this.projectWellTiePreparationBlocker === null;
  }

  get canImportHorizons(): boolean {
    return this.horizonImportBlocker === null;
  }

  get canImportProjectWellAssets(): boolean {
    return this.projectWellAssetImportBlocker === null;
  }

  get canAnalyzeProjectWellTie(): boolean {
    return this.projectWellTieAnalysisBlocker === null;
  }

  get canAcceptProjectWellTie(): boolean {
    return this.projectWellTieAcceptBlocker === null;
  }

  get projectSectionWellOverlayResolveBlocker(): string | null {
    if (!this.tauriRuntime) {
      return "Project section-well overlays are only available in the desktop runtime.";
    }
    if (!trimPath(this.projectRoot)) {
      return "Set a project root before resolving section well overlays.";
    }
    if (!this.projectGeospatialSettingsResolved || !this.displayCoordinateReferenceId) {
      return "Choose a project display CRS identifier before resolving section well overlays.";
    }
    if (!trimPath(this.projectSurveyAssetId)) {
      return "Select a project survey before resolving section well overlays.";
    }
    if (!trimPath(this.projectWellboreId)) {
      return "Select a project wellbore before resolving section well overlays.";
    }
    const selectedProjectSurveyCompatibility = this.selectedProjectSurveyDisplayCompatibility;
    if (
      selectedProjectSurveyCompatibility &&
      !selectedProjectSurveyCompatibility.canResolveProjectMap
    ) {
      return (
        this.selectedProjectSurveyDisplayCompatibilityMessage ??
        "The selected project survey cannot be resolved in the current project display CRS."
      );
    }
    const selectedProjectWellboreCompatibility = this.selectedProjectWellboreDisplayCompatibility;
    if (
      selectedProjectWellboreCompatibility &&
      !selectedProjectWellboreCompatibility.canResolveProjectMap
    ) {
      return (
        this.selectedProjectWellboreDisplayCompatibilityMessage ??
        "The selected project wellbore cannot be resolved in the current project display CRS."
      );
    }
    return null;
  }

  get projectWellTiePreparationBlocker(): string | null {
    if (!this.tauriRuntime) {
      return "Project well ties are only available in the desktop runtime.";
    }
    if (!trimPath(this.activeStorePath)) {
      return "Open a seismic volume before preparing a project well tie.";
    }
    if (!trimPath(this.projectRoot)) {
      return "Set a project root before preparing a project well tie.";
    }
    if (!this.projectGeospatialSettingsResolved || !this.displayCoordinateReferenceId) {
      return "Choose a project display CRS identifier before preparing a project well tie.";
    }
    if (!trimPath(this.projectSurveyAssetId)) {
      return "Select a project survey before preparing a project well tie.";
    }
    const selectedProjectSurveyCompatibility = this.selectedProjectSurveyDisplayCompatibility;
    if (
      selectedProjectSurveyCompatibility &&
      !selectedProjectSurveyCompatibility.canResolveProjectMap
    ) {
      return (
        this.selectedProjectSurveyDisplayCompatibilityMessage ??
        "The selected project survey cannot be resolved in the current project display CRS."
      );
    }
    if (!trimPath(this.projectWellboreId)) {
      return "Select a project wellbore before preparing a project well tie.";
    }
    const selectedProjectWellboreCompatibility = this.selectedProjectWellboreDisplayCompatibility;
    if (
      selectedProjectWellboreCompatibility &&
      !selectedProjectWellboreCompatibility.canResolveProjectMap
    ) {
      return (
        this.selectedProjectWellboreDisplayCompatibilityMessage ??
        "The selected project wellbore cannot be resolved in the current project display CRS."
      );
    }
    return null;
  }

  get horizonImportBlocker(): string | null {
    if (!trimPath(this.activeStorePath)) {
      return "Open a seismic volume before importing horizons.";
    }
    return null;
  }

  get depthConversionBlocker(): string | null {
    if (!this.tauriRuntime) {
      return "Depth conversion workbench is only available in the desktop runtime.";
    }
    if (!trimPath(this.activeStorePath)) {
      return "Open a seismic volume before converting horizons.";
    }
    if (!this.depthConversionHorizonAssets.length) {
      return "Import or load horizons before converting between TWT and depth.";
    }
    if (!this.availableVelocityModels.length) {
      return "Create or import a survey velocity model before converting horizons.";
    }
    return null;
  }

  get horizonImportSurveyModeBlocker(): string | null {
    if (!this.activeEffectiveNativeCoordinateReferenceId) {
      return "Active survey native CRS is unknown. Specify the horizon source CRS explicitly or assign the survey CRS first.";
    }
    return null;
  }

  get horizonImportProjectAdvisory(): string | null {
    if (trimPath(this.projectRoot) && !this.projectGeospatialSettingsResolved) {
      return "Project display CRS is unresolved. Horizon import can continue, but project overlays and map composition remain blocked until you choose a project CRS.";
    }
    if (
      trimPath(this.projectRoot) &&
      this.displayCoordinateReferenceId &&
      this.activeEffectiveNativeCoordinateReferenceId &&
      this.displayCoordinateReferenceId.toLowerCase() !==
        this.activeEffectiveNativeCoordinateReferenceId.toLowerCase() &&
      this.activeSurveyMapSurvey?.transform_status === "display_unavailable"
    ) {
      return `Project display CRS ${this.displayCoordinateReferenceId} differs from active survey native CRS ${this.activeEffectiveNativeCoordinateReferenceId}, and no display transform is currently available. Horizon import can continue in survey coordinates, but project display composition remains unavailable.`;
    }
    return null;
  }

  get projectWellAssetImportBlocker(): string | null {
    if (!this.tauriRuntime) {
      return "Project well-asset import is only available in the desktop runtime.";
    }
    if (!trimPath(this.projectRoot)) {
      return "Set a project root before importing project well assets.";
    }
    if (!this.projectGeospatialSettingsResolved || !this.displayCoordinateReferenceId) {
      return "Choose a project display CRS identifier before importing project well assets.";
    }
    if (!trimPath(this.projectSurveyAssetId)) {
      return "Select a project survey before importing project well assets.";
    }
    const selectedProjectSurveyCompatibility = this.selectedProjectSurveyDisplayCompatibility;
    if (
      selectedProjectSurveyCompatibility &&
      !selectedProjectSurveyCompatibility.canResolveProjectMap
    ) {
      return (
        this.selectedProjectSurveyDisplayCompatibilityMessage ??
        "The selected project survey cannot be resolved in the current project display CRS."
      );
    }
    if (!trimPath(this.projectWellboreId)) {
      return "Select a project wellbore before importing project well assets.";
    }
    const selectedProjectWellboreCompatibility = this.selectedProjectWellboreDisplayCompatibility;
    if (
      selectedProjectWellboreCompatibility &&
      !selectedProjectWellboreCompatibility.canResolveProjectMap
    ) {
      return (
        this.selectedProjectWellboreDisplayCompatibilityMessage ??
        "The selected project wellbore cannot be resolved in the current project display CRS."
      );
    }
    return null;
  }

  get projectWellAssetImportAdvisory(): string | null {
    const selectedProjectSurveyCompatibility = this.selectedProjectSurveyDisplayCompatibility;
    if (
      selectedProjectSurveyCompatibility?.transformStatus === "display_degraded" &&
      this.selectedProjectSurveyDisplayCompatibilityMessage
    ) {
      return this.selectedProjectSurveyDisplayCompatibilityMessage;
    }
    const selectedProjectWellboreCompatibility = this.selectedProjectWellboreDisplayCompatibility;
    if (
      selectedProjectWellboreCompatibility?.transformStatus === "display_degraded" &&
      this.selectedProjectWellboreDisplayCompatibilityMessage
    ) {
      return this.selectedProjectWellboreDisplayCompatibilityMessage;
    }
    return null;
  }

  get projectWellTieCompatibilityAdvisory(): string | null {
    const selectedProjectWellboreCompatibility = this.selectedProjectWellboreDisplayCompatibility;
    if (
      selectedProjectWellboreCompatibility?.transformStatus === "display_degraded" &&
      this.selectedProjectWellboreDisplayCompatibilityMessage
    ) {
      return this.selectedProjectWellboreDisplayCompatibilityMessage;
    }
    return null;
  }

  get projectWellTieAnalysisBlocker(): string | null {
    const preparationBlocker = this.projectWellTiePreparationBlocker;
    if (preparationBlocker) {
      return preparationBlocker;
    }
    if (this.projectWellTieCompatibilityAdvisory) {
      return this.projectWellTieCompatibilityAdvisory;
    }
    if (!this.selectedProjectWellTimeDepthModelAssetId) {
      return "Select a compiled well time-depth model before analyzing a project well tie.";
    }
    return null;
  }

  get projectWellTieAcceptBlocker(): string | null {
    const preparationBlocker = this.projectWellTiePreparationBlocker;
    if (preparationBlocker) {
      return preparationBlocker;
    }
    if (this.projectWellTieCompatibilityAdvisory) {
      return this.projectWellTieCompatibilityAdvisory;
    }
    if (!this.selectedProjectWellTimeDepthModelAssetId) {
      return "Select a compiled well time-depth model before accepting a project well tie.";
    }
    return null;
  }

  get requiresProjectGeospatialSettingsSelection(): boolean {
    return (
      !!trimPath(this.projectRoot) &&
      !this.projectGeospatialSettingsLoading &&
      !this.projectGeospatialSettingsResolved &&
      this.projectDisplayCoordinateReferenceMode === "authority_code"
    );
  }

  get suggestedProjectDisplayCoordinateReferenceId(): string | null {
    const activeCoordinateReferenceId = normalizeCoordinateReferenceId(
      this.activeEffectiveNativeCoordinateReferenceId
    );
    if (activeCoordinateReferenceId) {
      return activeCoordinateReferenceId;
    }

    const workspaceCoordinateReferenceIds = this.workspaceEntries
      .map((entry) =>
        normalizeCoordinateReferenceId(
          entry.last_dataset?.descriptor.coordinate_reference_binding?.effective?.id ?? null
        )
      )
      .filter((value): value is string => !!value);
    const uniqueCoordinateReferenceIds = uniqueStringsInOrder(workspaceCoordinateReferenceIds);
    if (uniqueCoordinateReferenceIds.length === 1) {
      return uniqueCoordinateReferenceIds[0] ?? null;
    }
    return null;
  }

  get projectSurveyAssets(): ProjectSurveyAssetDescriptor[] {
    return this.projectWellOverlayInventory?.surveys ?? [];
  }

  get projectWellboreInventory(): ProjectWellboreInventoryItem[] {
    return this.projectWellOverlayInventory?.wellbores ?? [];
  }

  get selectedProjectSurveyAsset(): ProjectSurveyAssetDescriptor | null {
    const projectSurveyAssetId = trimPath(this.projectSurveyAssetId);
    if (!projectSurveyAssetId) {
      return null;
    }
    return this.projectSurveyAssets.find((survey) => survey.assetId === projectSurveyAssetId) ?? null;
  }

  get selectedProjectSurveyDisplayCompatibility(): ProjectSurveyDisplayCompatibility | null {
    return this.selectedProjectSurveyAsset?.displayCompatibility ?? null;
  }

  get selectedProjectSurveyDisplayCompatibilityMessage(): string | null {
    return describeProjectSurveyDisplayCompatibility(this.selectedProjectSurveyDisplayCompatibility);
  }

  get selectedProjectSurveyWellboreId(): string | null {
    return trimPath(this.selectedProjectSurveyAsset?.wellboreId ?? "") || null;
  }

  get compatibleProjectSurveyAssets(): ProjectSurveyAssetDescriptor[] {
    return this.projectSurveyAssets.filter((survey) => survey.displayCompatibility.canResolveProjectMap);
  }

  get projectSurveySelectionGroups(): ProjectSurveySelectionGroup[] {
    return PROJECT_SURVEY_SELECTION_GROUPS.map((group) => ({
      label: group.label,
      surveys: this.projectSurveyAssets.filter(
        (survey) => projectSurveySelectionGroupKey(survey) === group.key
      )
    })).filter((group) => group.surveys.length > 0);
  }

  get projectSurveyDisplayCompatibilitySummaryLine(): string | null {
    const summary = this.projectWellOverlayInventory?.displayCompatibility;
    if (!summary) {
      return null;
    }
    const totalSurveyCount = summary.compatibleSurveyCount + summary.incompatibleSurveyCount;
    if (totalSurveyCount === 0) {
      return "No project surveys are available.";
    }
    const displayCoordinateReferenceId = summary.displayCoordinateReferenceId ?? "unresolved CRS";
    return `${summary.compatibleSurveyCount} of ${totalSurveyCount} surveys are ready for project display CRS ${displayCoordinateReferenceId}.`;
  }

  projectSurveyOptionLabel = (survey: ProjectSurveyAssetDescriptor): string => {
    const status = projectSurveyDisplayCompatibilityStatusLabel(survey.displayCompatibility);
    return `${survey.name} | ${survey.wellboreName} | ${status}`;
  };

  get selectedProjectWellboreInventoryItem(): ProjectWellboreInventoryItem | null {
    const projectWellboreId = trimPath(this.projectWellboreId);
    if (!projectWellboreId) {
      return null;
    }
    return (
      this.projectWellboreInventory.find((wellbore) => wellbore.wellboreId === projectWellboreId) ??
      null
    );
  }

  get selectedProjectWellboreDisplayCompatibility(): ProjectWellboreDisplayCompatibility | null {
    return this.selectedProjectWellboreInventoryItem?.displayCompatibility ?? null;
  }

  get selectedProjectWellboreDisplayCompatibilityMessage(): string | null {
    return describeProjectWellboreDisplayCompatibility(this.selectedProjectWellboreDisplayCompatibility);
  }

  get projectWellboreSelectionGroups(): ProjectWellboreSelectionGroup[] {
    return PROJECT_SURVEY_SELECTION_GROUPS.map((group) => ({
      label: group.label,
      wellbores: this.projectWellboreInventory.filter(
        (wellbore) => projectWellboreSelectionGroupKey(wellbore) === group.key
      )
    })).filter((group) => group.wellbores.length > 0);
  }

  get projectWellboreDisplayCompatibilitySummaryLine(): string | null {
    const summary = this.projectWellOverlayInventory?.displayCompatibility;
    if (!summary) {
      return null;
    }
    const totalWellboreCount = summary.compatibleWellboreCount + summary.incompatibleWellboreCount;
    if (totalWellboreCount === 0) {
      return "No project wellbores are available.";
    }
    const displayCoordinateReferenceId = summary.displayCoordinateReferenceId ?? "unresolved CRS";
    return `${summary.compatibleWellboreCount} of ${totalWellboreCount} wellbores are ready for project display CRS ${displayCoordinateReferenceId}.`;
  }

  get projectDisplayCompatibilityBlockingMessages(): string[] {
    const summary = this.projectWellOverlayInventory?.displayCompatibility;
    if (!summary) {
      return [];
    }

    const blockingReasonCodes = summary.blockingReasonCodes ?? [];
    const blockingReasons = summary.blockingReasons ?? [];
    const messages = blockingReasonCodes.length
      ? blockingReasonCodes.map((reasonCode) =>
          describeProjectDisplayCompatibilityBlockingReasonCode(
            reasonCode,
            summary.displayCoordinateReferenceId ?? null
          )
        )
      : blockingReasons;
    return uniqueStringsInOrder(messages.filter((message) => !!message));
  }

  projectWellboreStatusLabel = (wellbore: ProjectWellboreInventoryItem): string => {
    const readiness = projectWellboreDisplayCompatibilityStatusLabel(wellbore.displayCompatibility);
    return wellbore.wellboreId === this.selectedProjectSurveyWellboreId
      ? `${readiness} - survey match`
      : readiness;
  }

  projectWellboreOptionLabel = (wellbore: ProjectWellboreInventoryItem): string => {
    return `${wellbore.wellName} | ${wellbore.wellboreName} | ${this.projectWellboreStatusLabel(wellbore)}`;
  };

  get selectedProjectWellTimeDepthModel(): ProjectWellTimeDepthModelDescriptor | null {
    if (!this.selectedProjectWellTimeDepthModelAssetId) {
      return null;
    }
    return (
      this.projectWellTimeDepthModels.find(
        (model) => model.assetId === this.selectedProjectWellTimeDepthModelAssetId
      ) ?? null
    );
  }

  get selectedProjectWellTieObservationSet(): ProjectWellTimeDepthObservationDescriptor | null {
    if (!this.selectedProjectWellTieObservationAssetId) {
      return null;
    }
    return (
      this.projectWellTimeDepthObservationSets.find(
        (asset) => asset.assetId === this.selectedProjectWellTieObservationAssetId
      ) ?? null
    );
  }

  get timeDepthStatusLabel(): string | null {
    const diagnostics = this.timeDepthDiagnostics;
    if (!diagnostics) {
      return null;
    }
    switch (diagnostics.transform_mode) {
      case "none":
        return diagnostics.display_domain === "time" ? "Native time" : "No transform";
      case "global1d":
        return "Global 1D";
      case "survey3d":
        return "Survey 3D";
      default:
        return null;
    }
  }

  get timeDepthStatusDetail(): string | null {
    const diagnostics = this.timeDepthDiagnostics;
    if (!diagnostics) {
      return null;
    }
    return diagnostics.notes[0] ?? null;
  }

  get comparePrimaryDataset(): DatasetSummary | null {
    return this.dataset ?? this.activeDatasetEntry?.last_dataset ?? null;
  }

  get comparePrimaryStorePath(): string | null {
    const activeStorePath = trimPath(this.activeStorePath);
    if (activeStorePath) {
      return activeStorePath;
    }

    const fallbackStorePath = trimPath(entryStorePath(this.activeDatasetEntry));
    return fallbackStorePath || null;
  }

  get comparePoolState(): ComparePoolState {
    const primary = this.comparePrimaryDataset;
    const primaryStorePath = this.comparePrimaryStorePath;
    const primaryFingerprint = datasetGeometryFingerprint(primary);
    const primaryCompareFamily = datasetCompareFamily(primary);

    const candidates = this.workspaceEntries.map((entry) => {
      const storePath = trimPath(entryStorePath(entry));
      const candidateDataset = entry.last_dataset;
      const isPrimary = !!primaryStorePath && storePath === primaryStorePath;
      const reason = compareCandidateReason(primary, candidateDataset, storePath, isPrimary);

      return {
        entryId: entry.entry_id,
        displayName: userVisibleDatasetName(
          entry.display_name,
          entry.source_path,
          entry.imported_store_path ?? entry.preferred_store_path,
          entry.entry_id
        ),
        storePath,
        datasetId: candidateDataset?.descriptor.id ?? null,
        compareFamily: datasetCompareFamily(candidateDataset),
        fingerprint: datasetGeometryFingerprint(candidateDataset),
        compatible: reason === null,
        isPrimary,
        reason
      } satisfies CompareCandidate;
    });

    const compatibleStorePaths = candidates
      .filter((candidate) => candidate.compatible)
      .map((candidate) => candidate.storePath);

    const compatibleSecondaryStorePaths = candidates
      .filter((candidate) => candidate.compatible && !candidate.isPrimary)
      .map((candidate) => candidate.storePath);

    return {
      primaryStorePath,
      primaryDatasetId: primary?.descriptor.id ?? null,
      primaryLabel: this.activeDatasetDisplayName,
      compareFamily: primaryCompareFamily,
      fingerprint: primaryFingerprint,
      candidates,
      compatibleStorePaths,
      compatibleSecondaryStorePaths
    };
  }

  get compareCandidates(): CompareCandidate[] {
    return this.comparePoolState.candidates;
  }

  get compatibleCompareCandidates(): CompareCandidate[] {
    return this.compareCandidates.filter((candidate) => candidate.compatible);
  }

  get compatibleSecondaryCompareCandidates(): CompareCandidate[] {
    return this.compareCandidates.filter(
      (candidate) => candidate.compatible && !candidate.isPrimary
    );
  }

  get activeForegroundCompareCandidate(): CompareCandidate | null {
    const primaryStorePath = this.comparePrimaryStorePath;
    return this.compareCandidates.find((candidate) => candidate.storePath === primaryStorePath) ?? null;
  }

  get activeBackgroundCompareCandidate(): CompareCandidate | null {
    if (!this.compareBackgroundStorePath) {
      return null;
    }

    return (
      this.compatibleSecondaryCompareCandidates.find(
        (candidate) => candidate.storePath === this.compareBackgroundStorePath
      ) ?? null
    );
  }

  get canCycleForegroundCompareSurvey(): boolean {
    return this.compatibleCompareCandidates.length > 1;
  }

  get canEnableCompareSplit(): boolean {
    return !!this.activeBackgroundCompareCandidate && this.displayTransform.renderMode === "heatmap";
  }

  get activeCoordinateReferenceBinding() {
    return this.comparePrimaryDataset?.descriptor.coordinate_reference_binding ?? null;
  }

  get activeDetectedNativeCoordinateReferenceId(): string | null {
    return this.activeCoordinateReferenceBinding?.detected?.id ?? null;
  }

  get activeDetectedNativeCoordinateReferenceName(): string | null {
    return this.activeCoordinateReferenceBinding?.detected?.name ?? null;
  }

  get activeEffectiveNativeCoordinateReferenceId(): string | null {
    return this.activeCoordinateReferenceBinding?.effective?.id ?? null;
  }

  get activeEffectiveNativeCoordinateReferenceName(): string | null {
    return this.activeCoordinateReferenceBinding?.effective?.name ?? null;
  }

  get activeSurveyMapSurvey() {
    return this.surveyMapSource?.surveys[0] ?? null;
  }

  get surveyMapWellTransformWarnings(): string[] {
    const displayCoordinateReferenceId = normalizeCoordinateReferenceId(this.displayCoordinateReferenceId);
    if (!displayCoordinateReferenceId) {
      return [];
    }

    const wells = this.surveyMapSource?.wells ?? [];
    if (wells.length === 0) {
      return [];
    }

    const unavailableWells = wells.filter((well) => well.transform_status === "display_unavailable");
    const degradedWells = wells.filter((well) => well.transform_status === "display_degraded");
    const warnings: string[] = [];

    if (unavailableWells.length > 0) {
      warnings.push(
        `${unavailableWells.length} well${unavailableWells.length === 1 ? "" : "s"} could not be projected into display CRS ${displayCoordinateReferenceId}.`
      );
    }

    if (degradedWells.length > 0) {
      warnings.push(
        `${degradedWells.length} well${degradedWells.length === 1 ? "" : "s"} use partial geometry in display CRS ${displayCoordinateReferenceId}.`
      );
    }

    return warnings;
  }

  get workspaceCoordinateReferenceWarnings(): string[] {
    const canEvaluateProjectDisplayCompatibility =
      this.projectGeospatialSettingsResolved && !!this.displayCoordinateReferenceId;
    const selectedProjectSurveyCompatibility = this.selectedProjectSurveyDisplayCompatibility;
    const selectedProjectWellboreCompatibility = this.selectedProjectWellboreDisplayCompatibility;

    return buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: this.requiresProjectGeospatialSettingsSelection,
      suggestedProjectDisplayCoordinateReferenceId: this.suggestedProjectDisplayCoordinateReferenceId,
      canEvaluateProjectDisplayCompatibility,
      hasProjectRoot: !!trimPath(this.projectRoot),
      projectDisplayCompatibilityBlockingWarnings: this.projectDisplayCompatibilityBlockingMessages,
      hasSelectedProjectSurvey: !!trimPath(this.projectSurveyAssetId),
      selectedProjectSurveyCanResolveProjectMap:
        selectedProjectSurveyCompatibility?.canResolveProjectMap ?? null,
      selectedProjectSurveyReason: this.selectedProjectSurveyDisplayCompatibilityMessage,
      hasSelectedProjectWellbore: !!trimPath(this.projectWellboreId),
      selectedProjectWellboreCanResolveProjectMap:
        selectedProjectWellboreCompatibility?.canResolveProjectMap ?? null,
      selectedProjectWellboreReason: this.selectedProjectWellboreDisplayCompatibilityMessage,
      hasActiveDataset: !!this.comparePrimaryDataset,
      activeStoreAcceptedInNativeEngineering: this.activeStoreAcceptedInNativeEngineering,
      displayCoordinateReferenceId: this.displayCoordinateReferenceId,
      activeEffectiveNativeCoordinateReferenceId: this.activeEffectiveNativeCoordinateReferenceId,
      activeSurveyMapTransformStatus: this.activeSurveyMapSurvey?.transform_status ?? null,
      surveyMapError: this.surveyMapError,
      surveyMapWellTransformWarnings: this.surveyMapWellTransformWarnings
    });
  }

  get activeStoreAcceptedInNativeEngineering(): boolean {
    const storePath = trimPath(this.comparePrimaryStorePath ?? this.activeStorePath);
    return !!storePath && this.#acceptedNativeEngineeringStorePaths.has(storePath);
  }

  selectCompareBackground = (storePath: string | null): void => {
    const normalizedStorePath = trimPath(storePath ?? "") || null;

    if (!normalizedStorePath) {
      this.compareBackgroundStorePath = null;
      this.backgroundSection = null;
      this.backgroundError = null;
      this.backgroundLoading = false;
      this.#backgroundSectionKey = null;
      this.compareSplitEnabled = false;
      return;
    }

    const selectedCandidate = this.compatibleSecondaryCompareCandidates.find(
      (candidate) => candidate.storePath === normalizedStorePath
    );

    this.compareBackgroundStorePath = selectedCandidate?.storePath ?? null;
  };

  setCompareSplitEnabled = (enabled: boolean): void => {
    if (!enabled) {
      this.compareSplitEnabled = false;
      return;
    }

    if (!this.canEnableCompareSplit) {
      return;
    }

    this.compareSplitEnabled = true;
  };

  setCompareSplitPosition = (position: number): void => {
    this.compareSplitPosition = Math.min(Math.max(position, 0.1), 0.9);
  };

  cycleForegroundCompareSurvey = async (direction: -1 | 1): Promise<void> => {
    const candidates = this.compatibleCompareCandidates;
    if (candidates.length <= 1 || this.loading) {
      return;
    }

    const primaryStorePath = this.comparePrimaryStorePath;
    const currentIndex = candidates.findIndex((candidate) => candidate.storePath === primaryStorePath);
    const baseIndex = currentIndex >= 0 ? currentIndex : 0;
    const nextIndex = (baseIndex + direction + candidates.length) % candidates.length;
    const nextCandidate = candidates[nextIndex];

    if (!nextCandidate || !nextCandidate.storePath || nextCandidate.storePath === primaryStorePath) {
      return;
    }

    this.note("Cycling compare foreground survey.", "ui", "info", nextCandidate.displayName);
    await this.activateDatasetEntry(nextCandidate.entryId);
  };

  refreshCompareSelection = (): void => {
    if (!this.compareBackgroundStorePath) {
      return;
    }

    const stillCompatible = this.compatibleSecondaryCompareCandidates.some(
      (candidate) => candidate.storePath === this.compareBackgroundStorePath
    );

    if (!stillCompatible) {
      this.compareBackgroundStorePath = null;
      this.compareSplitEnabled = false;
      this.backgroundSection = null;
      this.backgroundError = null;
      this.backgroundLoading = false;
      this.#backgroundSectionKey = null;
    }
  };

  refreshSurveyMap = async (): Promise<void> => {
    const requestId = ++this.#surveyMapRequestId;
    const projectRoot = trimPath(this.projectRoot);
    const projectSurveyAssetId = trimPath(this.projectSurveyAssetId);
    const projectWellboreId = trimPath(this.projectWellboreId);
    const storePath = this.comparePrimaryStorePath;

    if (projectRoot && projectSurveyAssetId) {
      if (!this.projectGeospatialSettingsResolved || !this.displayCoordinateReferenceId) {
        this.surveyMapSource = null;
        this.surveyMapError = null;
        this.surveyMapLoading = false;
        return;
      }

      const selectedProjectSurveyCompatibility = this.selectedProjectSurveyDisplayCompatibility;
      if (
        selectedProjectSurveyCompatibility &&
        !selectedProjectSurveyCompatibility.canResolveProjectMap
      ) {
        this.surveyMapSource = null;
        this.surveyMapError =
          this.selectedProjectSurveyDisplayCompatibilityMessage ??
          "The selected project survey cannot be resolved in the current project display CRS.";
        this.surveyMapLoading = false;
        return;
      }
    } else if (!storePath) {
      this.surveyMapSource = null;
      this.surveyMapError = null;
      this.surveyMapLoading = false;
      return;
    }

    if (!this.tauriRuntime) {
      this.surveyMapSource = null;
      this.surveyMapError = null;
      this.surveyMapLoading = false;
      return;
    }

    this.surveyMapLoading = true;
    this.surveyMapError = null;

    try {
      const response =
        projectRoot && projectSurveyAssetId
          ? await resolveProjectSurveyMap({
              projectRoot,
              surveyAssetId: projectSurveyAssetId,
              wellboreId: projectWellboreId || null,
              displayCoordinateReferenceId: this.displayCoordinateReferenceId!
            })
          : await resolveSurveyMap({
              schema_version: 1,
              store_path: storePath as string,
              display_coordinate_reference_id: this.displayCoordinateReferenceId
            });

      if (requestId !== this.#surveyMapRequestId) {
        return;
      }

      this.surveyMapSource = "survey_map" in response ? response.survey_map : response.surveyMap;
      this.surveyMapError = null;
    } catch (error) {
      if (requestId !== this.#surveyMapRequestId) {
        return;
      }

      this.surveyMapSource = null;
      this.surveyMapError = errorMessage(error, "Failed to resolve the active survey map.");
      this.note("Failed to resolve survey map geometry.", "backend", "warn", this.surveyMapError);
    } finally {
      if (requestId === this.#surveyMapRequestId) {
        this.surveyMapLoading = false;
      }
    }
  };

  refreshProjectWellOverlayInventory = async (
    projectRoot: string,
    displayCoordinateReferenceId: string | null = this.displayCoordinateReferenceId
  ): Promise<ProjectWellOverlayInventoryResponse | null> => {
    const normalizedProjectRoot = trimPath(projectRoot);
    if (!normalizedProjectRoot) {
      this.projectWellOverlayInventory = null;
      this.projectWellOverlayInventoryError = null;
      this.projectWellOverlayInventoryLoading = false;
      this.projectSurveyAssetId = "";
      this.projectWellboreId = "";
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.projectWellTimeDepthModelsError = null;
      this.selectedProjectWellTimeDepthModelAssetId = null;
      return null;
    }

    if (!this.tauriRuntime) {
      this.projectWellOverlayInventory = null;
      this.projectWellOverlayInventoryError = null;
      this.projectWellOverlayInventoryLoading = false;
      return null;
    }

    this.projectWellOverlayInventoryLoading = true;
    this.projectWellOverlayInventoryError = null;
    const requestId = ++this.#projectWellOverlayInventoryRequestId;
    const previousSurveyAssetId = this.projectSurveyAssetId;
    const previousWellboreId = this.projectWellboreId;

    try {
      const inventory = await listProjectWellOverlayInventory(
        normalizedProjectRoot,
        displayCoordinateReferenceId
      );
      if (requestId !== this.#projectWellOverlayInventoryRequestId) {
        return null;
      }
      this.projectWellOverlayInventory = inventory;
      const preferredSurveyAssetId = pickPreferredProjectSurveyAssetId(inventory.surveys);
      const previousSurvey = inventory.surveys.find((survey) => survey.assetId === previousSurveyAssetId) ?? null;
      const preferredSurvey =
        inventory.surveys.find((survey) => survey.assetId === preferredSurveyAssetId) ?? null;

      const nextSurveyAssetId =
        previousSurvey &&
        (!preferredSurvey ||
          projectSurveyReadinessRank(previousSurvey) <= projectSurveyReadinessRank(preferredSurvey))
          ? previousSurvey.assetId
          : preferredSurveyAssetId;
      const surveyMatchedWellboreId =
        inventory.surveys.find((survey) => survey.assetId === nextSurveyAssetId)?.wellboreId ?? "";
      const nextWellboreId =
        inventory.wellbores.find((wellbore) => wellbore.wellboreId === previousWellboreId)?.wellboreId ??
        inventory.wellbores.find((wellbore) => wellbore.wellboreId === surveyMatchedWellboreId)?.wellboreId ??
        inventory.wellbores[0]?.wellboreId ??
        "";

      this.projectSurveyAssetId = nextSurveyAssetId;
      this.projectWellboreId = nextWellboreId;

      if (previousSurveyAssetId !== nextSurveyAssetId || previousWellboreId !== nextWellboreId) {
        this.clearProjectSectionWellOverlays();
      }

      if (nextSurveyAssetId) {
        await this.refreshProjectSurveyHorizons(normalizedProjectRoot, nextSurveyAssetId);
      } else {
        this.projectSurveyHorizons = [];
        this.selectedProjectHorizonId = "";
      }

      if (nextWellboreId) {
        await this.refreshProjectWellTimeDepthModels(normalizedProjectRoot, nextWellboreId);
        await this.refreshProjectWellMarkerResidualInventory(normalizedProjectRoot, nextWellboreId);
      } else {
        this.projectWellTimeDepthObservationSets = [];
        this.projectWellTimeDepthAuthoredModels = [];
        this.projectWellTimeDepthModels = [];
        this.projectWellTimeDepthModelsError = null;
        this.selectedProjectWellTimeDepthModelAssetId = null;
        this.projectWellMarkers = [];
        this.projectResidualAssets = [];
        this.selectedProjectWellMarkerName = "";
        this.selectedProjectResidualAssetId = null;
      }

      if (
        this.workspaceReady &&
        (previousSurveyAssetId !== nextSurveyAssetId || previousWellboreId !== nextWellboreId)
      ) {
        void this.persistWorkspaceSession();
      }

      return inventory;
    } catch (error) {
      if (requestId !== this.#projectWellOverlayInventoryRequestId) {
        return null;
      }
      this.projectWellOverlayInventory = null;
      this.projectSurveyAssetId = "";
      this.projectWellboreId = "";
      this.projectSurveyHorizons = [];
      this.selectedProjectHorizonId = "";
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.projectWellTimeDepthModelsError = null;
      this.selectedProjectWellTimeDepthModelAssetId = null;
      this.projectWellMarkers = [];
      this.projectResidualAssets = [];
      this.selectedProjectWellMarkerName = "";
      this.selectedProjectResidualAssetId = null;
      this.projectWellOverlayInventoryError = errorMessage(
        error,
        "Failed to load project well-overlay inventory."
      );
      this.note(
        "Failed to load project well-overlay inventory.",
        "backend",
        "warn",
        this.projectWellOverlayInventoryError
      );
      if (this.workspaceReady) {
        void this.persistWorkspaceSession();
      }
      return null;
    } finally {
      if (requestId === this.#projectWellOverlayInventoryRequestId) {
        this.projectWellOverlayInventoryLoading = false;
      }
    }
  };

  refreshProjectWellTimeDepthModels = async (
    projectRoot: string,
    wellboreId: string
  ): Promise<ProjectWellTimeDepthModelDescriptor[]> => {
    if (!this.tauriRuntime) {
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.projectWellTimeDepthModelsError = null;
      this.projectWellTimeDepthModelsLoading = false;
      return [];
    }

    this.projectWellTimeDepthModelsLoading = true;
    this.projectWellTimeDepthModelsError = null;
    const requestId = ++this.#projectWellTimeDepthModelsRequestId;
    const previousSelectedAssetId = this.selectedProjectWellTimeDepthModelAssetId;

    try {
      const inventory = await listProjectWellTimeDepthInventory(projectRoot, wellboreId);
      if (requestId !== this.#projectWellTimeDepthModelsRequestId) {
        return [];
      }
      const models = inventory.compiledModels;
      this.projectWellTimeDepthObservationSets = inventory.observationSets;
      this.projectWellTimeDepthAuthoredModels = inventory.authoredModels;
      this.projectWellTimeDepthModels = models;
      if (
        this.selectedProjectWellTieObservationAssetId &&
        !inventory.observationSets.some(
          (asset) => asset.assetId === this.selectedProjectWellTieObservationAssetId
        )
      ) {
        this.selectedProjectWellTieObservationAssetId = null;
      }
      if (
        this.selectedProjectWellTimeDepthModelAssetId &&
        !models.some((model) => model.assetId === this.selectedProjectWellTimeDepthModelAssetId)
      ) {
        this.selectedProjectWellTimeDepthModelAssetId = null;
      }
      if (!this.selectedProjectWellTimeDepthModelAssetId && models.length > 0) {
        this.selectedProjectWellTimeDepthModelAssetId =
          models.find((model) => model.isActiveProjectModel)?.assetId ?? models[0]!.assetId;
      }
      if (this.workspaceReady && previousSelectedAssetId !== this.selectedProjectWellTimeDepthModelAssetId) {
        void this.persistWorkspaceSession();
      }
      return models;
    } catch (error) {
      if (requestId !== this.#projectWellTimeDepthModelsRequestId) {
        return [];
      }
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.selectedProjectWellTimeDepthModelAssetId = null;
      this.selectedProjectWellTieObservationAssetId = null;
      this.projectWellTimeDepthModelsError = errorMessage(
        error,
        "Failed to load project well time-depth models."
      );
      if (this.workspaceReady && previousSelectedAssetId !== this.selectedProjectWellTimeDepthModelAssetId) {
        void this.persistWorkspaceSession();
      }
      this.note(
        "Failed to load project well time-depth models.",
        "backend",
        "warn",
        this.projectWellTimeDepthModelsError
      );
      return [];
    } finally {
      if (requestId === this.#projectWellTimeDepthModelsRequestId) {
        this.projectWellTimeDepthModelsLoading = false;
      }
    }
  };

  refreshProjectSurveyHorizons = async (
    projectRoot: string,
    surveyAssetId: string
  ): Promise<ImportedHorizonDescriptor[]> => {
    if (!this.tauriRuntime || !trimPath(projectRoot) || !trimPath(surveyAssetId)) {
      this.projectSurveyHorizons = [];
      this.selectedProjectHorizonId = "";
      return [];
    }

    const requestId = ++this.#projectSurveyHorizonsRequestId;
    try {
      const horizons = await listProjectSurveyHorizons(projectRoot, surveyAssetId);
      if (requestId !== this.#projectSurveyHorizonsRequestId) {
        return [];
      }
      this.projectSurveyHorizons = horizons;
      if (!horizons.some((horizon) => horizon.id === this.selectedProjectHorizonId)) {
        this.selectedProjectHorizonId =
          horizons.find((horizon) => horizon.vertical_domain === "depth")?.id ??
          horizons[0]?.id ??
          "";
      }
      return horizons;
    } catch (error) {
      if (requestId !== this.#projectSurveyHorizonsRequestId) {
        return [];
      }
      this.projectSurveyHorizons = [];
      this.selectedProjectHorizonId = "";
      this.note(
        "Failed to load project survey horizons.",
        "backend",
        "warn",
        errorMessage(error, "Unknown project survey horizon error")
      );
      return [];
    }
  };

  refreshProjectWellMarkerResidualInventory = async (
    projectRoot: string,
    wellboreId: string
  ): Promise<ProjectWellMarkerHorizonResidualDescriptor[]> => {
    if (!this.tauriRuntime || !trimPath(projectRoot) || !trimPath(wellboreId)) {
      this.projectWellMarkers = [];
      this.projectResidualAssets = [];
      this.selectedProjectWellMarkerName = "";
      this.selectedProjectResidualAssetId = null;
      return [];
    }

    const requestId = ++this.#projectWellMarkerResidualInventoryRequestId;
    try {
      const inventory = await listProjectWellMarkerResidualInventory(projectRoot, wellboreId);
      if (requestId !== this.#projectWellMarkerResidualInventoryRequestId) {
        return [];
      }
      this.projectWellMarkers = inventory.markers;
      this.projectResidualAssets = inventory.residualAssets;
      if (
        !inventory.markers.some(
          (marker) => marker.name.trim() === this.selectedProjectWellMarkerName.trim()
        )
      ) {
        this.selectedProjectWellMarkerName = inventory.markers[0]?.name ?? "";
      }
      if (
        this.selectedProjectResidualAssetId &&
        !inventory.residualAssets.some((asset) => asset.assetId === this.selectedProjectResidualAssetId)
      ) {
        this.selectedProjectResidualAssetId = null;
      }
      return inventory.residualAssets;
    } catch (error) {
      if (requestId !== this.#projectWellMarkerResidualInventoryRequestId) {
        return [];
      }
      this.projectWellMarkers = [];
      this.projectResidualAssets = [];
      this.selectedProjectWellMarkerName = "";
      this.selectedProjectResidualAssetId = null;
      this.note(
        "Failed to load project marker/residual inventory.",
        "backend",
        "warn",
        errorMessage(error, "Unknown project residual inventory error")
      );
      return [];
    }
  };

  refreshConfiguredProjectWellTimeDepthModels = async (): Promise<ProjectWellTimeDepthModelDescriptor[]> => {
    const projectRoot = trimPath(this.projectRoot);
    const wellboreId = trimPath(this.projectWellboreId);
    if (!projectRoot || !wellboreId) {
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.selectedProjectWellTimeDepthModelAssetId = null;
      this.selectedProjectWellTieObservationAssetId = null;
      this.projectWellTimeDepthModelsError =
        "Set both the project root and wellbore id before loading well models.";
      if (this.workspaceReady) {
        void this.persistWorkspaceSession();
      }
      return [];
    }

    return this.refreshProjectWellTimeDepthModels(projectRoot, wellboreId);
  };

  setProjectRoot = async (projectRoot: string): Promise<void> => {
    this.projectRoot = projectRoot.trim();
    this.clearProjectSectionWellOverlays();
    if (!this.projectRoot) {
      this.#projectWellOverlayInventoryRequestId += 1;
      this.#projectWellTimeDepthModelsRequestId += 1;
      this.#projectSurveyHorizonsRequestId += 1;
      this.#projectWellMarkerResidualInventoryRequestId += 1;
      this.projectWellOverlayInventory = null;
      this.projectWellOverlayInventoryError = null;
      this.projectWellOverlayInventoryLoading = false;
      this.projectSurveyAssetId = "";
      this.projectWellboreId = "";
      this.projectSurveyHorizons = [];
      this.selectedProjectHorizonId = "";
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.projectWellTimeDepthModelsError = null;
      this.projectWellTimeDepthModelsLoading = false;
      this.selectedProjectWellTimeDepthModelAssetId = null;
      this.selectedProjectWellTieObservationAssetId = null;
      this.projectWellMarkers = [];
      this.projectResidualAssets = [];
      this.selectedProjectWellMarkerName = "";
      this.selectedProjectResidualAssetId = null;
      this.projectWellTieDraftSeed = null;
      this.projectWellTieDraftSeedNonce += 1;
      this.#applyTemporaryDisplaySelection(this.displayCoordinateReferenceId);
    } else if (this.tauriRuntime) {
      await this.loadProjectGeospatialSettings(this.projectRoot);
      await this.refreshProjectWellOverlayInventory(this.projectRoot, this.displayCoordinateReferenceId);
    } else {
      await this.loadProjectGeospatialSettings(this.projectRoot);
    }
    if (this.workspaceReady) {
      await this.persistWorkspaceSession();
    }
  };

  setProjectSurveyAssetId = (surveyAssetId: string): void => {
    this.projectSurveyAssetId = surveyAssetId.trim();
    const matchedWellboreId =
      this.projectSurveyAssets.find((survey) => survey.assetId === this.projectSurveyAssetId)?.wellboreId ?? "";
    const nextWellboreId = matchedWellboreId.trim();
    const projectRoot = trimPath(this.projectRoot);
    if (projectRoot && this.projectSurveyAssetId) {
      void this.refreshProjectSurveyHorizons(projectRoot, this.projectSurveyAssetId);
    } else {
      this.projectSurveyHorizons = [];
      this.selectedProjectHorizonId = "";
    }
    if (nextWellboreId && nextWellboreId !== this.projectWellboreId) {
      this.projectWellboreId = nextWellboreId;
      if (projectRoot) {
        void this.refreshProjectWellTimeDepthModels(projectRoot, nextWellboreId);
        void this.refreshProjectWellMarkerResidualInventory(projectRoot, nextWellboreId);
      }
    }
    this.clearProjectSectionWellOverlays();
    if (projectRoot) {
      void this.refreshSurveyMap();
    }
    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  setProjectWellboreId = (wellboreId: string): void => {
    this.projectWellboreId = wellboreId.trim();
    this.clearProjectSectionWellOverlays();
    const projectRoot = trimPath(this.projectRoot);
    if (projectRoot) {
      void this.refreshSurveyMap();
    }
    if (this.projectWellboreId && projectRoot) {
      void this.refreshProjectWellTimeDepthModels(projectRoot, this.projectWellboreId);
      void this.refreshProjectWellMarkerResidualInventory(projectRoot, this.projectWellboreId);
    } else {
      this.#projectWellTimeDepthModelsRequestId += 1;
      this.#projectWellMarkerResidualInventoryRequestId += 1;
      this.projectWellTimeDepthObservationSets = [];
      this.projectWellTimeDepthAuthoredModels = [];
      this.projectWellTimeDepthModels = [];
      this.projectWellTimeDepthModelsError = null;
      this.projectWellTimeDepthModelsLoading = false;
      this.selectedProjectWellTimeDepthModelAssetId = null;
      this.selectedProjectWellTieObservationAssetId = null;
      this.projectWellMarkers = [];
      this.projectResidualAssets = [];
      this.selectedProjectWellMarkerName = "";
      this.selectedProjectResidualAssetId = null;
      this.projectWellTieDraftSeed = null;
      this.projectWellTieDraftSeedNonce += 1;
    }
    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  setProjectSectionToleranceM = (toleranceM: number): void => {
    if (!Number.isFinite(toleranceM) || toleranceM <= 0) {
      return;
    }
    this.projectSectionToleranceM = toleranceM;
    this.clearProjectSectionWellOverlays();
    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  setSelectedProjectWellTimeDepthModelAssetId = (assetId: string | null): void => {
    const nextAssetId = assetId?.trim() || null;
    this.selectedProjectWellTimeDepthModelAssetId = nextAssetId;

    const projectRoot = trimPath(this.projectRoot);
    const wellboreId = trimPath(this.projectWellboreId);
    if (this.tauriRuntime && projectRoot && wellboreId) {
      void (async () => {
        try {
          await setProjectActiveWellTimeDepthModel(projectRoot, wellboreId, nextAssetId);
          this.projectWellTimeDepthModels = this.projectWellTimeDepthModels.map((model) => ({
            ...model,
            isActiveProjectModel: model.assetId === nextAssetId
          }));
          if (this.projectWellOverlayInventory) {
            this.projectWellOverlayInventory = {
              ...this.projectWellOverlayInventory,
              wellbores: this.projectWellOverlayInventory.wellbores.map((wellbore) =>
                wellbore.wellboreId === wellboreId
                  ? {
                      ...wellbore,
                      activeWellTimeDepthModelAssetId: nextAssetId
                    }
                  : wellbore
              )
            };
          }
          if (this.workspaceReady) {
            await this.persistWorkspaceSession();
          }
        } catch (error) {
          const message = errorMessage(
            error,
            "Failed to update the active project well time-depth model."
          );
          this.projectWellTimeDepthModelsError = message;
          this.note(
            "Failed to update the active project well time-depth model.",
            "backend",
            "warn",
            message
          );
        }
      })();
      return;
    }

    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  clearProjectSectionWellOverlays = (): void => {
    this.projectSectionWellOverlays = null;
    this.projectSectionWellOverlaysError = null;
    this.sectionWellOverlays = [];
  };

  importProjectWellTimeDepthModel = async (
    request: ImportProjectWellTimeDepthModelRequest
  ): Promise<ImportProjectWellTimeDepthModelResponse> => {
    const response = await importProjectWellTimeDepthModel(request);
    this.note(
      "Imported project well time-depth model.",
      "backend",
      "info",
      response.assetId
    );
    return response;
  };

  importProjectWellTimeDepthAsset = async (
    request: ImportProjectWellTimeDepthAssetRequest
  ): Promise<ImportProjectWellTimeDepthModelResponse> => {
    const response = await importProjectWellTimeDepthAsset(request);
    this.note(
      "Imported project well time-depth asset.",
      "backend",
      "info",
      `${request.assetKind}:${response.assetId}`
    );
    return response;
  };

  importProjectWellTimeDepthDraft = async (
    request: CommitProjectWellTimeDepthImportRequest
  ): Promise<ImportProjectWellTimeDepthModelResponse> => {
    const response = await commitProjectWellTimeDepthImport(request);
    this.note(
      "Imported project well time-depth asset.",
      "backend",
      "info",
      `${request.draft.assetKind}:${response.assetId}`
    );
    return response;
  };

  compileProjectWellTimeDepthAuthoredModel = async (
    request: CompileProjectWellTimeDepthAuthoredModelRequest
  ): Promise<ImportProjectWellTimeDepthModelResponse> => {
    const response = await compileProjectWellTimeDepthAuthoredModel(request);
    this.note(
      "Compiled project well time-depth authored model.",
      "backend",
      "info",
      response.assetId
    );
    return response;
  };

  analyzeProjectWellTie = async (
    request: AnalyzeProjectWellTieRequest
  ): Promise<ProjectWellTieAnalysisResponse> => {
    const blocker = this.projectWellTieAnalysisBlocker;
    if (blocker) {
      throw new Error(blocker);
    }
    const response = await analyzeProjectWellTie(request);
    this.note(
      "Analyzed project well tie.",
      "backend",
      "info",
      `${response.sourceModelName}:${response.analysis.synthetic_trace.amplitudes.length} synthetic samples`
    );
    return response;
  };

  acceptProjectWellTie = async (
    request: AcceptProjectWellTieRequest
  ): Promise<AcceptProjectWellTieResponse> => {
    const blocker = this.projectWellTieAcceptBlocker;
    if (blocker) {
      throw new Error(blocker);
    }
    const response = await acceptProjectWellTie(request);
    this.note(
      request.setActive ? "Accepted and activated project well tie." : "Accepted project well tie.",
      "backend",
      "info",
      response.compiledModelAssetId
    );
    await this.refreshProjectWellOverlayInventory(request.projectRoot, this.displayCoordinateReferenceId);
    this.selectedProjectWellTieObservationAssetId = response.observationAssetId;
    this.selectedProjectWellTimeDepthModelAssetId = response.compiledModelAssetId;
    return response;
  };

  readProjectWellTimeDepthModel = async (
    projectRoot: string,
    assetId: string
  ): Promise<WellTimeDepthModel1D> => {
    return readProjectWellTimeDepthModel(projectRoot, assetId);
  };

  resolveProjectSectionWellOverlays = async (
    request: SectionWellOverlayRequestDto
  ): Promise<ResolveSectionWellOverlaysResponse> => {
    if (!this.tauriRuntime) {
      this.projectSectionWellOverlays = null;
      this.sectionWellOverlays = [];
      this.projectSectionWellOverlaysError = null;
      this.projectSectionWellOverlaysLoading = false;
      throw new Error("Project section-well overlays are only available in the desktop runtime.");
    }

    this.projectSectionWellOverlaysLoading = true;
    this.projectSectionWellOverlaysError = null;

    try {
      const response = await resolveProjectSectionWellOverlays(request);
      this.projectSectionWellOverlays = response;
      this.sectionWellOverlays = adaptSectionWellOverlays(response);
      this.note(
        "Resolved project section well overlays.",
        "backend",
        "info",
        `${response.overlays.length} overlay${response.overlays.length === 1 ? "" : "s"}`
      );
      return response;
    } catch (error) {
      this.projectSectionWellOverlays = null;
      this.sectionWellOverlays = [];
      this.projectSectionWellOverlaysError = errorMessage(
        error,
        "Failed to resolve project section-well overlays."
      );
      this.note(
        "Failed to resolve project section-well overlays.",
        "backend",
        "warn",
        this.projectSectionWellOverlaysError
      );
      throw error;
    } finally {
      this.projectSectionWellOverlaysLoading = false;
    }
  };

  resolveConfiguredProjectSectionWellOverlays = async (): Promise<ResolveSectionWellOverlaysResponse> => {
    const projectRoot = trimPath(this.projectRoot);
    const surveyAssetId = trimPath(this.projectSurveyAssetId);
    const wellboreId = trimPath(this.projectWellboreId);
    const blocker = this.projectSectionWellOverlayResolveBlocker;
    if (blocker) {
      throw new Error(blocker);
    }

    const activeWellModelIds = this.selectedProjectWellTimeDepthModelAssetId
      ? [this.selectedProjectWellTimeDepthModelAssetId]
      : [];

    return this.resolveProjectSectionWellOverlays({
      schema_version: 1,
      project_root: projectRoot,
      survey_asset_id: surveyAssetId,
      wellbore_ids: [wellboreId],
      axis: this.axis,
      index: this.index,
      tolerance_m: this.projectSectionToleranceM,
      display_domain: this.sectionDomain,
      active_well_model_ids: activeWellModelIds
    });
  };

  private async loadResolvedSection(
    storePath: string,
    axis: SectionAxis,
    index: number
  ): Promise<DisplaySectionView> {
    await this.ensureActiveVelocityModelReady(storePath);
    if (this.sectionDomain === "depth") {
      if (!this.activeVelocityModel) {
        throw new Error("Depth display requires an active velocity model.");
      }
      return fetchDepthConvertedSectionView(
        storePath,
        axis,
        index,
        this.activeVelocityModel,
        this.depthVelocityKind
      );
    }
    return fetchSectionView(storePath, axis, index);
  }

  private async loadResolvedSectionDisplay(
    storePath: string,
    axis: SectionAxis,
    index: number
  ): Promise<TransportResolvedSectionDisplayView> {
    await this.ensureActiveVelocityModelReady(storePath);
    return fetchResolvedSectionDisplay(
      storePath,
      axis,
      index,
      this.sectionDomain,
      this.activeVelocityModel,
      this.depthVelocityKind,
      this.showVelocityOverlay
    );
  }

  private async ensureActiveVelocityModelReady(storePath: string): Promise<void> {
    if (!trimPath(storePath) || this.activeVelocityModelAssetId !== DEMO_SURVEY_3D_TRANSFORM_ID) {
      return;
    }

    const assetId = await ensureDemoSurveyTimeDepthTransform(storePath);
    if (assetId !== DEMO_SURVEY_3D_TRANSFORM_ID) {
      this.note(
        "The runtime returned an unexpected synthetic survey 3D transform id.",
        "backend",
        "warn",
        assetId
      );
    }
  }

  private async loadBackgroundSection(
    storePath: string,
    axis: SectionAxis,
    index: number
  ): Promise<void> {
    const requestId = ++this.#backgroundLoadRequestId;
    const sectionKey = `${storePath}:${axis}:${index}`;
    this.backgroundLoading = true;
    this.backgroundError = null;

    try {
      const section = await this.loadResolvedSection(storePath, axis, index);
      if (requestId !== this.#backgroundLoadRequestId) {
        return;
      }

      this.backgroundSection = section;
      this.#backgroundSectionKey = sectionKey;
    } catch (error) {
      if (requestId !== this.#backgroundLoadRequestId) {
        return;
      }

      this.backgroundSection = null;
      this.#backgroundSectionKey = null;
      this.backgroundError = errorMessage(error, "Failed to load compare background section.");
      this.compareSplitEnabled = false;
      this.note("Failed to load compare background section.", "backend", "warn", this.backgroundError);
    } finally {
      if (requestId === this.#backgroundLoadRequestId) {
        this.backgroundLoading = false;
      }
    }
  }

  setSectionDomain = async (domain: SectionDisplayDomain): Promise<void> => {
    if (domain === this.sectionDomain) {
      return;
    }
    if (domain === "depth" && !this.canDisplayDepthSection) {
      this.note(
        "Depth display currently requires the desktop runtime, an active store, and a valid velocity model.",
        "ui",
        "warn"
      );
      return;
    }

    this.sectionDomain = domain;
    this.note(
      domain === "depth" ? "Switched section display to depth." : "Switched section display to time.",
      "ui",
      "info",
      domain === "depth"
        ? this.activeVelocityModelDescriptor
          ? this.activeVelocityModelDescriptor.name
          : `Constant Vavg ${Math.round(this.depthVelocityMPerS)} m/s`
        : null
    );

    if (trimPath(this.activeStorePath)) {
      await this.load(this.axis, this.index);
    }
  };

  refreshVelocityModels = async (storePathOverride?: string): Promise<void> => {
    const storePath = trimPath(storePathOverride ?? this.activeStorePath);
    if (!this.tauriRuntime || !storePath) {
      this.availableVelocityModels = [];
      this.velocityModelsLoading = false;
      this.velocityModelsError = null;
      return;
    }

    this.velocityModelsLoading = true;
    this.velocityModelsError = null;
    try {
      const models = await loadVelocityModels(storePath);
      this.availableVelocityModels = models;
      if (
        this.activeVelocityModelAssetId &&
        !models.some((model) => model.id === this.activeVelocityModelAssetId)
      ) {
        this.activeVelocityModelAssetId = null;
      }
    } catch (error) {
      this.availableVelocityModels = [];
      this.velocityModelsError = errorMessage(error, "Failed to load velocity models.");
    } finally {
      this.velocityModelsLoading = false;
    }
  };

  refreshHorizonAssets = async (storePathOverride?: string): Promise<void> => {
    const storePath = trimPath(storePathOverride ?? this.activeStorePath);
    if (!this.tauriRuntime || !storePath) {
      this.importedHorizons = [];
      return;
    }

    try {
      this.importedHorizons = await loadHorizonAssets(storePath);
    } catch (error) {
      this.importedHorizons = [];
      this.note(
        "Failed to load horizon assets.",
        "backend",
        "warn",
        errorMessage(error, "Unknown horizon asset error")
      );
    }
  };

  activateVelocityModel = async (assetId: string | null): Promise<void> => {
    if (assetId === this.activeVelocityModelAssetId) {
      return;
    }

    this.activeVelocityModelAssetId = assetId;
    this.note(
      assetId
        ? "Activated velocity model for time-depth conversion."
        : "Cleared the active velocity model and fell back to the global 1D velocity.",
      "ui",
      "info",
      assetId ?? `Constant Vavg ${Math.round(this.depthVelocityMPerS)} m/s`
    );
    await this.persistWorkspaceSession();

    if (trimPath(this.activeStorePath)) {
      await this.load(this.axis, this.index);
    }
  };

  createDemoVelocityModel = async (): Promise<void> => {
    const storePath = trimPath(this.activeStorePath);
    if (!storePath) {
      this.note("Open a seismic volume before creating a demo velocity model.", "ui", "warn");
      return;
    }

    try {
      const assetId = await ensureDemoSurveyTimeDepthTransform(storePath);
      await this.refreshVelocityModels(storePath);
      await this.activateVelocityModel(assetId);
    } catch (error) {
      this.note(
        "Failed to create the synthetic survey 3D velocity model.",
        "backend",
        "error",
        errorMessage(error, "Unknown velocity model error")
      );
    }
  };

  openVelocityModelWorkbench = (): void => {
    this.velocityModelWorkbenchError = null;
    this.velocityModelWorkbenchOpen = true;
    this.note("Opened the experimental velocity-model workbench.", "ui", "info");
  };

  closeVelocityModelWorkbench = (): void => {
    this.velocityModelWorkbenchError = null;
    this.velocityModelWorkbenchOpen = false;
  };

  setSelectedProjectHorizonId = (horizonId: string): void => {
    this.selectedProjectHorizonId = horizonId.trim();
    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  setSelectedProjectWellMarkerName = (markerName: string): void => {
    this.selectedProjectWellMarkerName = markerName.trim();
    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  setSelectedProjectResidualAssetId = (assetId: string | null): void => {
    this.selectedProjectResidualAssetId = assetId?.trim() || null;
    if (this.workspaceReady) {
      void this.persistWorkspaceSession();
    }
  };

  openResidualWorkbench = (): void => {
    const blocker = this.residualWorkbenchBlocker;
    this.residualWorkbenchError = null;
    this.residualWorkbenchOpen = true;
    if (blocker) {
      this.note(
        "Opened the residual workbench with unresolved prerequisites.",
        "ui",
        "warn",
        blocker
      );
      return;
    }
    this.note("Opened the residual workbench.", "ui", "info");
  };

  closeResidualWorkbench = (): void => {
    this.residualWorkbenchError = null;
    this.residualWorkbenchOpen = false;
  };

  openDepthConversionWorkbench = (): void => {
    this.depthConversionWorkbenchError = null;
    this.depthConversionWorkbenchOpen = true;
    this.note("Opened the depth-conversion workbench.", "ui", "info");
  };

  closeDepthConversionWorkbench = (): void => {
    this.depthConversionWorkbenchError = null;
    this.depthConversionWorkbenchOpen = false;
  };

  openProjectSettings = (): void => {
    this.projectSettingsOpen = true;
  };

  closeProjectSettings = (): void => {
    this.projectSettingsOpen = false;
  };

  openWellTieWorkbench = (): void => {
    this.wellTieWorkbenchError = null;
    this.wellTieWorkbenchOpen = true;
    this.note("Opened the well-tie workbench.", "ui", "info");
  };

  resumeWellTieWorkbenchFromObservation = (assetId: string): void => {
    const observation = this.projectWellTimeDepthObservationSets.find(
      (candidate) => candidate.assetId === assetId
    );
    if (!observation || observation.assetKind !== "well_tie_observation_set") {
      this.note("Selected observation is not a resumable well tie.", "ui", "warn", assetId);
      return;
    }

    this.selectedProjectWellTieObservationAssetId = observation.assetId;
    const sourceModelAssetId = observation.sourceWellTimeDepthModelAssetId?.trim() || null;
    if (
      sourceModelAssetId &&
      this.projectWellTimeDepthModels.some((model) => model.assetId === sourceModelAssetId)
    ) {
      this.selectedProjectWellTimeDepthModelAssetId = sourceModelAssetId;
    } else if (sourceModelAssetId) {
      this.note(
        "The saved well-tie source model is not currently available in this wellbore inventory.",
        "ui",
        "warn",
        sourceModelAssetId
      );
    }

    this.projectWellTieDraftSeed = {
      observationAssetId: observation.assetId,
      sourceModelAssetId,
      tieName: observation.name,
      tieStartMs:
        observation.tieWindowStartMs !== null && observation.tieWindowStartMs !== undefined
          ? observation.tieWindowStartMs.toFixed(0)
          : "1100",
      tieEndMs:
        observation.tieWindowEndMs !== null && observation.tieWindowEndMs !== undefined
          ? observation.tieWindowEndMs.toFixed(0)
          : "2200",
      searchRadiusM:
        observation.traceSearchRadiusM !== null && observation.traceSearchRadiusM !== undefined
          ? observation.traceSearchRadiusM.toFixed(0)
          : "200",
      summary:
        observation.tieWindowStartMs !== null &&
        observation.tieWindowStartMs !== undefined &&
        observation.tieWindowEndMs !== null &&
        observation.tieWindowEndMs !== undefined
          ? `Resumed from accepted tie ${observation.tieWindowStartMs.toFixed(0)}-${observation.tieWindowEndMs.toFixed(0)} ms.`
          : `Resumed from accepted tie '${observation.name}'.`
    };
    this.projectWellTieDraftSeedNonce += 1;
    this.openWellTieWorkbench();
    this.note("Loaded an accepted well tie into the workbench.", "ui", "info", observation.name);
  };

  closeWellTieWorkbench = (): void => {
    this.wellTieWorkbenchError = null;
    this.wellTieWorkbenchOpen = false;
  };

  computeProjectResidual = async (
    request: ComputeProjectWellMarkerResidualRequest
  ): Promise<ComputeProjectWellMarkerResidualResponse> => {
    if (!this.tauriRuntime) {
      throw new Error("Project residual computation is only available in the desktop runtime.");
    }
    this.residualWorkbenchWorking = true;
    this.residualWorkbenchError = null;
    try {
      const response = await computeProjectWellMarkerResidual(request);
      this.selectedProjectResidualAssetId = response.assetId;
      await this.refreshProjectWellMarkerResidualInventory(request.projectRoot, request.wellboreId);
      this.note(
        "Computed and stored project residuals.",
        "backend",
        "info",
        `${response.collectionName} (${response.pointCount} point${response.pointCount === 1 ? "" : "s"})`
      );
      return response;
    } catch (error) {
      this.residualWorkbenchError = errorMessage(
        error,
        "Failed to compute the selected project residual."
      );
      this.note(
        "Failed to compute the selected project residual.",
        "backend",
        "warn",
        this.residualWorkbenchError
      );
      throw error;
    } finally {
      this.residualWorkbenchWorking = false;
    }
  };

  buildAuthoredVelocityModel = async (
    request: BuildSurveyTimeDepthTransformRequest,
    activate = true
  ): Promise<SurveyTimeDepthTransform3D> => {
    const storePath = trimPath(this.activeStorePath);
    if (!storePath) {
      throw new Error("Open a seismic volume before building a velocity model.");
    }
    if (!this.tauriRuntime) {
      throw new Error("Velocity-model building is only available in the desktop runtime right now.");
    }

    this.velocityModelWorkbenchBuilding = true;
    this.velocityModelWorkbenchError = null;
    try {
      const builtModel = await buildVelocityModelTransform({
        ...request,
        store_path: storePath
      });
      await this.refreshVelocityModels(storePath);
      if (activate) {
        await this.activateVelocityModel(builtModel.id);
      }
      this.note(
        activate
          ? "Built and activated authored velocity model."
          : "Built authored velocity model.",
        "backend",
        "info",
        builtModel.name
      );
      return builtModel;
    } catch (error) {
      const message = errorMessage(error, "Failed to build the velocity model.");
      this.velocityModelWorkbenchError = message;
      this.note("Velocity-model build failed.", "backend", "error", message);
      throw error;
    } finally {
      this.velocityModelWorkbenchBuilding = false;
    }
  };

  convertSurveyHorizonDomain = async (request: {
    sourceHorizonId: string;
    transformId: string;
    targetDomain: "time" | "depth";
    outputId?: string | null;
    outputName?: string | null;
  }): Promise<ImportedHorizonDescriptor> => {
    const storePath = trimPath(this.activeStorePath);
    if (!storePath) {
      throw new Error("Open a seismic volume before converting horizons.");
    }
    if (!this.tauriRuntime) {
      throw new Error("Survey horizon conversion is only available in the desktop runtime right now.");
    }

    this.depthConversionWorkbenchWorking = true;
    this.depthConversionWorkbenchError = null;
    try {
      const converted = await convertHorizonDomain(
        storePath,
        request.sourceHorizonId,
        request.transformId,
        request.targetDomain,
        request.outputId,
        request.outputName
      );
      await this.refreshHorizonAssets(storePath);
      await this.load(this.axis, this.index, storePath);
      await this.refreshSurveyMap();
      this.note(
        request.targetDomain === "depth"
          ? "Converted horizon from TWT to depth."
          : "Converted horizon from depth to TWT.",
        "backend",
        "info",
        converted.name
      );
      return converted;
    } catch (error) {
      const message = errorMessage(error, "Failed to convert the selected horizon.");
      this.depthConversionWorkbenchError = message;
      this.note("Depth conversion failed.", "backend", "error", message);
      throw error;
    } finally {
      this.depthConversionWorkbenchWorking = false;
    }
  };

  importVelocityFunctionsFile = async (
    inputPath: string,
    velocityKind: VelocityQuantityKind = "interval"
  ): Promise<void> => {
    const storePath = trimPath(this.activeStorePath);
    const normalizedInputPath = trimPath(inputPath);
    if (!storePath) {
      this.note("Open a seismic volume before importing velocity functions.", "ui", "warn");
      return;
    }
    if (!normalizedInputPath) {
      return;
    }

    this.velocityModelsLoading = true;
    this.velocityModelsError = null;
    try {
      const response = await importVelocityFunctionsModel(storePath, normalizedInputPath, velocityKind);
      await this.refreshVelocityModels(storePath);
      await this.activateVelocityModel(response.model.id);
      this.note(
        "Imported sparse velocity functions and compiled a survey transform.",
        "backend",
        "info",
        `${response.profile_count} profiles, ${response.sample_count} samples`
      );
    } catch (error) {
      const message = errorMessage(error, "Failed to import velocity functions.");
      this.velocityModelsError = message;
      this.note("Velocity-functions import failed.", "backend", "error", message);
    } finally {
      this.velocityModelsLoading = false;
    }
  };

  setDepthVelocityMPerS = async (velocityMPerS: number): Promise<void> => {
    if (!Number.isFinite(velocityMPerS) || velocityMPerS < 1) {
      this.note("Depth conversion velocity must be a finite value >= 1 m/s.", "ui", "warn");
      return;
    }

    this.depthVelocityMPerS = velocityMPerS;
    if (!this.activeVelocityModelAssetId && this.sectionDomain === "depth" && trimPath(this.activeStorePath)) {
      await this.load(this.axis, this.index);
    }
  };

  setShowVelocityOverlay = async (enabled: boolean): Promise<void> => {
    this.showVelocityOverlay = enabled;
    if (trimPath(this.activeStorePath)) {
      await this.load(this.axis, this.index);
    }
  };

  setVelocityOverlayOpacity = (opacity: number): void => {
    const clamped = Math.min(Math.max(opacity, 0), 1);
    this.velocityOverlayOpacity = clamped;
    this.sectionScalarOverlays = this.sectionScalarOverlays.map((overlay) => ({
      ...overlay,
      opacity: clamped
    }));
  };

  setSelectedPresetId = (presetId: string | null): void => {
    this.selectedPresetId = presetId?.trim() || null;
    if (!this.workspaceReady) {
      return;
    }
    void this.persistWorkspaceSession();
  };

  #applyProjectDisplaySelection = (
    selection: ProjectDisplayCoordinateReference,
    resolved: boolean,
    source: string | null
  ): void => {
    this.projectDisplayCoordinateReferenceMode = selection.kind;
    this.projectDisplayCoordinateReferenceIdDraft = coordinateReferenceSelectionId(selection) ?? "";
    this.projectGeospatialSettingsResolved = resolved;
    this.projectGeospatialSettingsSource = source;
    this.displayCoordinateReferenceId = resolved ? coordinateReferenceSelectionId(selection) : null;
  };

  #applyProjectGeospatialSettings = (settings: ProjectGeospatialSettings | null): void => {
    if (!settings) {
      this.#applyProjectDisplaySelection(
        { kind: "native_engineering" },
        true,
        trimPath(this.projectRoot) ? "default_native_engineering" : "temporary_workspace"
      );
      return;
    }
    this.#applyProjectDisplaySelection(settings.displayCoordinateReference, true, settings.source);
  };

  #applyTemporaryDisplaySelection = (coordinateReferenceId: string | null): void => {
    const normalizedCoordinateReferenceId = normalizeCoordinateReferenceId(coordinateReferenceId);
    const selection = projectDisplaySelectionFromCoordinateReferenceId(normalizedCoordinateReferenceId);
    this.#applyProjectDisplaySelection(
      selection ?? { kind: "native_engineering" },
      true,
      "temporary_workspace"
    );
  };

  loadProjectGeospatialSettings = async (
    projectRoot: string,
    options: { allowAutoSeed?: boolean; allowMigration?: boolean } = {}
  ): Promise<boolean> => {
    const normalizedProjectRoot = trimPath(projectRoot);
    if (!normalizedProjectRoot) {
      this.#applyTemporaryDisplaySelection(this.displayCoordinateReferenceId);
      return true;
    }

    const allowMigration = options.allowMigration !== false;
    this.projectGeospatialSettingsLoading = true;

    try {
      const settings = await loadProjectGeospatialSettings(normalizedProjectRoot);
      if (settings) {
        this.#applyProjectGeospatialSettings(settings);
        return true;
      }

      const legacyDisplayCoordinateReferenceId = allowMigration
        ? normalizeCoordinateReferenceId(this.displayCoordinateReferenceId)
        : null;
      if (legacyDisplayCoordinateReferenceId) {
        const legacySelection = projectDisplaySelectionFromCoordinateReferenceId(
          legacyDisplayCoordinateReferenceId
        );
        if (!legacySelection) {
          this.note(
            "Legacy project display CRS could not be migrated because the identifier is malformed.",
            "backend",
            "warn",
            legacyDisplayCoordinateReferenceId
          );
          this.#applyProjectGeospatialSettings(null);
          return false;
        }
        await this.saveProjectDisplaySettings("migrated", legacySelection);
        this.note(
          "Migrated the project display CRS from the legacy workspace session into project settings.",
          "backend",
          "info",
          legacyDisplayCoordinateReferenceId
        );
        return true;
      }

      this.#applyProjectGeospatialSettings(null);
      return true;
    } catch (error) {
      const message = errorMessage(error, "Failed to load the project geospatial settings.");
      this.#applyProjectGeospatialSettings(null);
      this.note("Failed to load project geospatial settings.", "backend", "warn", message);
      return false;
    } finally {
      this.projectGeospatialSettingsLoading = false;
      if (!this.workspaceReady) {
        void this.refreshSurveyMap();
      }
    }
  };

  saveProjectDisplaySettings = async (
    source = "user_selected",
    selection?: ProjectDisplayCoordinateReference
  ): Promise<boolean> => {
    const nextSelection =
      selection ??
      (this.projectDisplayCoordinateReferenceMode === "authority_code"
        ? projectDisplaySelectionFromCoordinateReferenceId(
            this.projectDisplayCoordinateReferenceIdDraft.trim()
          )
        : { kind: "native_engineering" });

    if (!nextSelection) {
      this.note("Enter a project display CRS identifier before applying it.", "ui", "warn");
      return false;
    }

    this.projectGeospatialSettingsSaving = true;

    try {
      const normalizedProjectRoot = trimPath(this.projectRoot);
      if (!normalizedProjectRoot) {
        this.#applyProjectDisplaySelection(nextSelection, true, "temporary_workspace");
        void this.refreshSurveyMap();
        if (this.workspaceReady) {
          void this.persistWorkspaceSession();
        }
        return true;
      }

      const settings = await saveProjectGeospatialSettings(
        normalizedProjectRoot,
        nextSelection,
        source
      );
      this.#applyProjectGeospatialSettings(settings);
      await this.refreshProjectWellOverlayInventory(
        normalizedProjectRoot,
        this.displayCoordinateReferenceId
      );
      void this.refreshSurveyMap();
      if (this.workspaceReady) {
        void this.persistWorkspaceSession();
      }
      return true;
    } catch (error) {
      this.note(
        "Failed to save project geospatial settings.",
        "backend",
        "warn",
        errorMessage(error, "Unknown project geospatial settings error")
      );
      return false;
    } finally {
      this.projectGeospatialSettingsSaving = false;
    }
  };

  setProjectDisplayCoordinateReferenceMode = (
    mode: "native_engineering" | "authority_code"
  ): void => {
    this.projectDisplayCoordinateReferenceMode = mode;
    if (mode === "native_engineering") {
      this.projectDisplayCoordinateReferenceIdDraft = "";
    }
  };

  setDisplayCoordinateReferenceId = (coordinateReferenceId: string | null): void => {
    const normalizedCoordinateReferenceId = normalizeCoordinateReferenceId(coordinateReferenceId);
    this.projectDisplayCoordinateReferenceMode = normalizedCoordinateReferenceId
      ? "authority_code"
      : "native_engineering";
    this.projectDisplayCoordinateReferenceIdDraft = normalizedCoordinateReferenceId ?? "";
    this.displayCoordinateReferenceId = normalizedCoordinateReferenceId;
    this.projectGeospatialSettingsResolved = true;
    this.projectGeospatialSettingsSource = trimPath(this.projectRoot) ? "legacy_session" : "temporary_workspace";
    if (!this.workspaceReady) {
      void this.refreshSurveyMap();
      return;
    }
    void this.refreshSurveyMap();
    void this.persistWorkspaceSession();
  };

  #applyWorkspaceSession = (session: WorkspaceSession): void => {
    this.activeEntryId = session.active_entry_id;
    this.selectedPresetId = session.selected_preset_id;
    this.displayCoordinateReferenceId = session.display_coordinate_reference_id;
    this.activeVelocityModelAssetId = session.active_velocity_model_asset_id;
    this.axis = session.active_axis;
    this.index = session.active_index;
    this.projectRoot = session.project_root ?? "";
    this.projectSurveyAssetId = session.project_survey_asset_id ?? "";
    this.projectWellboreId = session.project_wellbore_id ?? "";
    this.projectSectionToleranceM =
      session.project_section_tolerance_m && session.project_section_tolerance_m > 0
        ? session.project_section_tolerance_m
        : 12.5;
    this.selectedProjectWellTimeDepthModelAssetId =
      session.selected_project_well_time_depth_model_asset_id ?? null;
    this.#acceptedNativeEngineeringStorePaths = new SvelteSet(
      (session.native_engineering_accepted_store_paths ?? [])
        .map((value) => trimPath(value))
        .filter((value, index, values) => !!value && values.indexOf(value) === index)
    );
    if (!trimPath(this.projectRoot)) {
      this.#applyTemporaryDisplaySelection(session.display_coordinate_reference_id);
    } else {
      this.projectDisplayCoordinateReferenceMode = "native_engineering";
      this.projectDisplayCoordinateReferenceIdDraft = "";
      this.projectGeospatialSettingsResolved = false;
      this.projectGeospatialSettingsSource = null;
      this.displayCoordinateReferenceId = null;
    }
  };

  #applyWorkspaceEntry = (entry: DatasetRegistryEntry | null): void => {
    if (!entry) {
      this.availableVelocityModels = [];
      this.velocityModelsError = null;
      this.velocityModelsLoading = false;
      this.nativeCoordinateReferenceOverrideIdDraft = "";
      this.nativeCoordinateReferenceOverrideNameDraft = "";
      return;
    }

    const sourcePath = entry.source_path ?? "";
    const storePath = entryStorePath(entry);
    this.inputPath = sourcePath;
    this.outputStorePath = storePath;
    this.activeStorePath = entry.imported_store_path ?? this.activeStorePath;
    this.#outputPathSource = storePath ? "manual" : "auto";
    this.error = null;
    this.preflight = null;
    this.nativeCoordinateReferenceOverrideIdDraft =
      entry.last_dataset?.descriptor.coordinate_reference_binding?.effective?.id ?? "";
    this.nativeCoordinateReferenceOverrideNameDraft =
      entry.last_dataset?.descriptor.coordinate_reference_binding?.effective?.name ?? "";
    void this.refreshVelocityModels(storePath);
  };

  #clearLoadedDataset = (): void => {
    this.activeStorePath = "";
    this.dataset = null;
    this.missingNativeCoordinateReferencePrompt = null;
    this.surveyMapSource = null;
    this.surveyMapError = null;
    this.surveyMapLoading = false;
    this.section = null;
    this.timeDepthDiagnostics = null;
    this.sectionScalarOverlays = [];
    this.sectionHorizons = [];
    this.sectionWellOverlays = [];
    this.backgroundSection = null;
    this.lastProbe = null;
    this.lastInteraction = null;
    this.displayStorePath = "";
    this.displayGeometryFingerprint = null;
    this.displayAxis = "inline";
    this.displayIndex = 0;
    this.displayDomain = "time";
    this.resetToken = this.#displayResetToken();
    this.compareBackgroundStorePath = null;
    this.compareSplitEnabled = false;
    this.compareSplitPosition = 0.5;
    this.backgroundError = null;
    this.backgroundLoading = false;
    this.#backgroundSectionKey = null;
    this.availableVelocityModels = [];
    this.velocityModelsError = null;
    this.velocityModelsLoading = false;
    this.activeVelocityModelAssetId = null;
    this.nativeCoordinateReferenceOverrideIdDraft = "";
    this.nativeCoordinateReferenceOverrideNameDraft = "";
  };

  #syncWorkspaceState = (entries: DatasetRegistryEntry[], session: WorkspaceSession): void => {
    this.workspaceEntries = sortWorkspaceEntries(entries);
    this.#applyWorkspaceSession(session);
    this.#applyWorkspaceEntry(
      this.workspaceEntries.find((entry) => entry.entry_id === session.active_entry_id) ?? null
    );
    this.refreshCompareSelection();
    this.workspaceReady = true;
    void this.refreshSurveyMap();
    if (this.tauriRuntime && trimPath(this.projectRoot)) {
      void this.refreshProjectWellOverlayInventory(
        trimPath(this.projectRoot),
        this.displayCoordinateReferenceId
      );
    }
  };

  updateActiveEntryPipelines = async (
    sessionPipelines: WorkspacePipelineEntry[],
    activeSessionPipelineId: string | null
  ): Promise<void> => {
    const activeEntry = this.activeDatasetEntry;
    if (!activeEntry) {
      return;
    }

    try {
      const response = await upsertDatasetEntry({
        schema_version: 1,
        entry_id: activeEntry.entry_id,
        display_name: activeEntry.display_name,
        source_path: activeEntry.source_path,
        preferred_store_path: activeEntry.preferred_store_path,
        imported_store_path: activeEntry.imported_store_path,
        dataset: activeEntry.last_dataset,
        session_pipelines: sessionPipelines,
        active_session_pipeline_id: activeSessionPipelineId,
        make_active: true
      });
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, response.entry);
      this.#applyWorkspaceSession(response.session);
    } catch (error) {
      this.note(
        "Failed to persist session pipelines for the active dataset.",
        "backend",
        "warn",
        errorMessage(error, "Unknown pipeline workspace error")
      );
    }
  };

  refreshWorkspaceState = async (): Promise<void> => {
    const response = await loadWorkspaceState();
    this.#syncWorkspaceState(response.entries, response.session);
  };

  persistWorkspaceSession = async (): Promise<void> => {
    if (!this.workspaceReady) {
      return;
    }

    try {
      const response = await saveWorkspaceSession({
        schema_version: 1,
        active_entry_id: this.activeEntryId,
        active_store_path: trimPath(this.activeStorePath) || null,
        active_axis: this.axis,
        active_index: this.index,
        selected_preset_id: this.selectedPresetId,
        display_coordinate_reference_id: this.displayCoordinateReferenceId,
        active_velocity_model_asset_id: this.activeVelocityModelAssetId,
        project_root: trimPath(this.projectRoot) || null,
        project_survey_asset_id: trimPath(this.projectSurveyAssetId) || null,
        project_wellbore_id: trimPath(this.projectWellboreId) || null,
        project_section_tolerance_m:
          Number.isFinite(this.projectSectionToleranceM) && this.projectSectionToleranceM > 0
            ? this.projectSectionToleranceM
            : null,
        selected_project_well_time_depth_model_asset_id:
          this.selectedProjectWellTimeDepthModelAssetId,
        native_engineering_accepted_store_paths: [...this.#acceptedNativeEngineeringStorePaths]
      });
      this.#applyWorkspaceSession(response.session);
    } catch (error) {
      this.note(
        "Failed to persist workspace session state.",
        "backend",
        "warn",
        errorMessage(error, "Unknown workspace session error")
      );
    }
  };

  setActiveDatasetNativeCoordinateReference = async (
    coordinateReferenceId: string | null,
    coordinateReferenceName: string | null
  ): Promise<SetActiveDatasetNativeCoordinateReferenceResult> => {
    const storePath = this.comparePrimaryStorePath;
    const normalizedCoordinateReferenceId = coordinateReferenceId?.trim() || null;
    const normalizedCoordinateReferenceName = coordinateReferenceName?.trim() || null;
    if (!storePath) {
      this.note("Survey CRS assignment blocked because no active runtime store is available.", "ui", "warn");
      return {
        applied: false,
        requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
        requestedCoordinateReferenceName: normalizedCoordinateReferenceName,
        effectiveCoordinateReferenceId: this.activeEffectiveNativeCoordinateReferenceId,
        effectiveCoordinateReferenceName: this.activeEffectiveNativeCoordinateReferenceName,
        exactMatch: false,
        error: "Survey CRS assignment blocked because no active runtime store is available."
      };
    }

    this.#emitCoordinateReferenceLifecycleDiagnostics(
      "info",
      "Requested active dataset survey CRS assignment.",
      {
        event: "crs_assignment_requested",
        storePath,
        requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
        requestedCoordinateReferenceName: normalizedCoordinateReferenceName
      }
    );

    try {
      const response = await setDatasetNativeCoordinateReference({
        schema_version: 1,
        store_path: storePath,
        coordinate_reference_id: normalizedCoordinateReferenceId,
        coordinate_reference_name: normalizedCoordinateReferenceName
      });
      this.dataset = response.dataset;
      const activeEntry = this.activeDatasetEntry;
      if (activeEntry) {
        this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, {
          ...activeEntry,
          imported_store_path: activeEntry.imported_store_path ?? response.dataset.store_path,
          last_dataset: response.dataset
        });
      }
      this.nativeCoordinateReferenceOverrideIdDraft =
        response.dataset.descriptor.coordinate_reference_binding?.effective?.id ?? "";
      this.nativeCoordinateReferenceOverrideNameDraft =
        response.dataset.descriptor.coordinate_reference_binding?.effective?.name ?? "";
      const effectiveCoordinateReferenceId =
        response.dataset.descriptor.coordinate_reference_binding?.effective?.id ?? null;
      const effectiveCoordinateReferenceName =
        response.dataset.descriptor.coordinate_reference_binding?.effective?.name ?? null;
      const exactMatch =
        normalizedCoordinateReferenceId === null
          ? effectiveCoordinateReferenceId === null
          : effectiveCoordinateReferenceId?.toLowerCase() ===
              normalizedCoordinateReferenceId.toLowerCase();
      void this.refreshSurveyMap();
      if (!exactMatch && normalizedCoordinateReferenceId) {
        this.note(
          "Active dataset survey CRS assignment did not match the requested CRS.",
          "backend",
          "error",
          `${normalizedCoordinateReferenceId} -> ${effectiveCoordinateReferenceId ?? effectiveCoordinateReferenceName ?? "unknown"}`
        );
        this.#emitCoordinateReferenceLifecycleDiagnostics(
          "warn",
          "Applied active dataset survey CRS assignment with a different effective CRS.",
          {
            event: "crs_assignment_mismatched",
            storePath,
            requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
            effectiveCoordinateReferenceId,
            effectiveCoordinateReferenceName
          }
        );
      } else {
        this.note(
          normalizedCoordinateReferenceId
            ? "Updated active dataset survey CRS assignment."
            : "Cleared active dataset survey CRS assignment.",
          "backend",
          "info",
          normalizedCoordinateReferenceId || null
        );
        this.#emitCoordinateReferenceLifecycleDiagnostics(
          "info",
          normalizedCoordinateReferenceId
            ? "Applied active dataset survey CRS assignment."
            : "Cleared active dataset survey CRS assignment.",
          {
            event: normalizedCoordinateReferenceId
              ? "crs_assignment_applied"
              : "crs_assignment_cleared",
            storePath,
            requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
            effectiveCoordinateReferenceId,
            effectiveCoordinateReferenceName
          }
        );
      }
      return {
        applied: true,
        requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
        requestedCoordinateReferenceName: normalizedCoordinateReferenceName,
        effectiveCoordinateReferenceId,
        effectiveCoordinateReferenceName,
        exactMatch,
        error: null
      };
    } catch (error) {
      const message = errorMessage(error, "Unknown survey CRS assignment error");
      this.note(
        "Failed to update the active dataset survey CRS assignment.",
        "backend",
        "error",
        message
      );
      this.#emitCoordinateReferenceLifecycleDiagnostics(
        "error",
        "Failed to update the active dataset survey CRS assignment.",
        {
          event: "crs_assignment_failed",
          storePath,
          requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
          requestedCoordinateReferenceName: normalizedCoordinateReferenceName,
          error: message
        }
      );
      return {
        applied: false,
        requestedCoordinateReferenceId: normalizedCoordinateReferenceId,
        requestedCoordinateReferenceName: normalizedCoordinateReferenceName,
        effectiveCoordinateReferenceId: this.activeEffectiveNativeCoordinateReferenceId,
        effectiveCoordinateReferenceName: this.activeEffectiveNativeCoordinateReferenceName,
        exactMatch: false,
        error: message
      };
    }
  };

  dismissMissingNativeCoordinateReferencePrompt = (): void => {
    const prompt = this.missingNativeCoordinateReferencePrompt;
    if (!prompt) {
      return;
    }
    this.#acceptedNativeEngineeringStorePaths.add(prompt.storePath);
    this.missingNativeCoordinateReferencePrompt = null;
    this.note(
      "Continuing in native engineering coordinates for the active survey.",
      "ui",
      "info",
      prompt.storePath
    );
    this.#emitCoordinateReferenceLifecycleDiagnostics(
      "info",
      "Accepted native engineering coordinates for the active survey in this workspace session.",
      {
        event: "crs_prompt_keep_native_engineering",
        storePath: prompt.storePath,
        displayCoordinateReferenceId: prompt.displayCoordinateReferenceId ?? null,
        triggeredBy: prompt.triggeredBy
      }
    );
    void this.persistWorkspaceSession();
  };

  applyMissingNativeCoordinateReferencePromptSelection = async (
    coordinateReferenceId: string | null,
    coordinateReferenceName: string | null = null
  ): Promise<SetActiveDatasetNativeCoordinateReferenceResult> => {
    const promptStorePath = this.missingNativeCoordinateReferencePrompt?.storePath ?? null;
    const result = await this.setActiveDatasetNativeCoordinateReference(
      coordinateReferenceId,
      coordinateReferenceName
    );
    if (result.exactMatch) {
      if (promptStorePath) {
        this.#acceptedNativeEngineeringStorePaths.delete(promptStorePath);
        void this.persistWorkspaceSession();
      }
      this.missingNativeCoordinateReferencePrompt = null;
    }
    return result;
  };

  #maybeQueueMissingNativeCoordinateReferencePrompt = (
    dataset: DatasetSummary,
    sourcePath: string | null,
    trigger: "open" | "import",
    options: {
      makeActive: boolean;
      promptRequested: boolean;
      displayCoordinateReferenceName?: string | null;
    }
  ): void => {
    const storePath = trimPath(dataset.store_path);
    const effectiveCoordinateReference =
      dataset.descriptor.coordinate_reference_binding?.effective ?? null;
    const shouldPrompt = shouldPromptForMissingNativeCoordinateReference({
      makeActive: options.makeActive,
      promptRequested: options.promptRequested,
      restoringWorkspace: this.restoringWorkspace,
      storePath,
      effectiveCoordinateReferenceId: effectiveCoordinateReference?.id ?? null,
      effectiveCoordinateReferenceName: effectiveCoordinateReference?.name ?? null,
      acceptedNativeEngineeringStorePaths: this.#acceptedNativeEngineeringStorePaths
    });

    if (!shouldPrompt) {
      if (this.missingNativeCoordinateReferencePrompt?.storePath === storePath) {
        this.missingNativeCoordinateReferencePrompt = null;
      }
      return;
    }

    this.missingNativeCoordinateReferencePrompt = {
      storePath,
      datasetDisplayName: this.activeDatasetDisplayName,
      sourcePath: trimPath(sourcePath ?? "") || null,
      displayCoordinateReferenceId: normalizeCoordinateReferenceId(this.displayCoordinateReferenceId),
      displayCoordinateReferenceName: options.displayCoordinateReferenceName?.trim() || null,
      triggeredBy: trigger
    };
    this.#emitCoordinateReferenceLifecycleDiagnostics(
      "info",
      "Queued survey CRS prompt for an active dataset with no effective CRS.",
      {
        event: "crs_prompt_queued",
        storePath,
        sourcePath: trimPath(sourcePath ?? "") || null,
        displayCoordinateReferenceId: normalizeCoordinateReferenceId(this.displayCoordinateReferenceId),
        triggeredBy: trigger
      }
    );
  };

  #emitCoordinateReferenceLifecycleDiagnostics(
    level: FrontendDiagnosticsEventRequest["level"],
    message: string,
    fields: Record<string, unknown>
  ): void {
    if (!this.tauriRuntime) {
      return;
    }
    void emitFrontendDiagnosticsEvent({
      stage: "coordinate_reference",
      level,
      message,
      fields
    }).catch(() => {});
  }

  setInputPath = (inputPath: string): void => {
    const normalizedPath = trimPath(inputPath);
    const previousInputPath = this.inputPath;
    const previousOutputStorePath = this.outputStorePath;
    const suggestedStorePath = deriveStorePathFromInput(normalizedPath);
    const sourceVolumeType = describeImportVolumeType(fileExtension(normalizedPath));
    const shouldReplaceOutputPath =
      !previousOutputStorePath ||
      this.#outputPathSource === "auto" ||
      trimPath(previousOutputStorePath) === trimPath(this.lastImportedStorePath);

    this.inputPath = normalizedPath;
    this.preflight = null;
    this.importGeometryRecovery = null;
    this.error = null;

    if (shouldReplaceOutputPath && suggestedStorePath && suggestedStorePath !== previousOutputStorePath) {
      this.outputStorePath = suggestedStorePath;
      this.#outputPathSource = "auto";
      this.note(
        "Suggested runtime store output path from the selected source volume.",
        "ui",
        "info",
        suggestedStorePath
      );
    }

    if (
      previousInputPath &&
      previousInputPath !== normalizedPath &&
      previousOutputStorePath &&
      previousOutputStorePath === this.lastImportedStorePath
    ) {
      this.note(
        "Replaced the previous active store path with a new suggested output path for the selected source volume.",
        "ui",
        "info",
        `${previousOutputStorePath} -> ${this.outputStorePath}`
      );
    }

    this.note(`Selected ${sourceVolumeType} path.`, "ui", "info", normalizedPath);
  };

  openVolumePath = async (volumePath: string): Promise<void> => {
    const normalizedPath = trimPath(volumePath);
    if (!normalizedPath) {
      this.error = "Volume path is required.";
      this.note("Open-volume blocked because no usable path was provided.", "ui", "error");
      return;
    }

    const extension = fileExtension(normalizedPath);
    const hasActiveDataset = Boolean(this.dataset && trimPath(this.activeStorePath));
    const shouldActivateOpenedVolume = !hasActiveDataset;
    if (extension === ".tbvol") {
      const matchingEntry =
        this.workspaceEntries.find(
          (entry) =>
            trimPath(entry.imported_store_path ?? entry.preferred_store_path ?? "") === normalizedPath
        ) ?? null;
      await this.openDatasetAt(normalizedPath, "inline", 0, {
        entryId: matchingEntry?.entry_id ?? null,
        sourcePath: matchingEntry?.source_path ?? null,
        sessionPipelines: cloneSessionPipelines(matchingEntry?.session_pipelines),
        activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
        makeActive: shouldActivateOpenedVolume,
        loadSection: shouldActivateOpenedVolume
      });
      return;
    }

    if (!isSupportedImportVolumeExtension(extension)) {
      this.error = "TraceBoost currently supports opening .tbvol, .mdio, .zarr, .sgy, and .segy volumes.";
      this.note("Open-volume blocked because the selected file type is unsupported.", "ui", "error", normalizedPath);
      return;
    }

    const matchingEntry =
      this.workspaceEntries.find((entry) => trimPath(entry.source_path ?? "") === normalizedPath) ?? null;
    const existingImportedStore = trimPath(matchingEntry?.imported_store_path ?? "");
    if (existingImportedStore) {
      this.note("Reusing existing imported runtime store for the selected source volume.", "ui", "info", existingImportedStore);
      await this.openDatasetAt(existingImportedStore, "inline", 0, {
        entryId: matchingEntry?.entry_id ?? null,
        sourcePath: normalizedPath,
        sessionPipelines: cloneSessionPipelines(matchingEntry?.session_pipelines),
        activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
        makeActive: shouldActivateOpenedVolume,
        loadSection: shouldActivateOpenedVolume
      });
      return;
    }

    if (isDirectImportVolumeExtension(extension)) {
      const outputStorePath =
        trimPath(matchingEntry?.imported_store_path ?? matchingEntry?.preferred_store_path ?? "") ||
        (await defaultImportStorePath(normalizedPath));
      this.note(
        `Started one-shot import from ${describeImportVolumeType(extension).toLowerCase()}.`,
        "ui",
        "info",
        normalizedPath
      );
      await this.importDataset({
        inputPath: normalizedPath,
        outputStorePath,
        entryId: matchingEntry?.entry_id ?? null,
        sourcePath: normalizedPath,
        sessionPipelines: cloneSessionPipelines(matchingEntry?.session_pipelines),
        activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
        makeActive: shouldActivateOpenedVolume,
        loadSection: shouldActivateOpenedVolume,
        reuseExistingStore: true
      });
      return;
    }

    this.loading = true;
    this.busyLabel = "Inspecting volume";
    this.error = null;
    this.preflight = null;
    this.importGeometryRecovery = null;
    this.note("Started one-shot SEG-Y import.", "ui", "info", normalizedPath);

    try {
      const preflight = await preflightImport(normalizedPath);
      this.preflight = preflight;

      const outputStorePath =
        trimPath(matchingEntry?.imported_store_path ?? matchingEntry?.preferred_store_path ?? "") ||
        (await defaultImportStorePath(normalizedPath));

      if (!canAutoImportPreflight(preflight)) {
        this.loading = false;
        this.busyLabel = null;

        if (canRecoverPreflight(preflight)) {
          this.error = null;
          this.openImportGeometryRecovery(preflight, {
            inputPath: normalizedPath,
            outputStorePath,
            entryId: matchingEntry?.entry_id ?? null,
            sourcePath: normalizedPath,
            sessionPipelines: cloneSessionPipelines(matchingEntry?.session_pipelines),
            activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
            makeActive: shouldActivateOpenedVolume,
            loadSection: shouldActivateOpenedVolume,
            reuseExistingStore: true
          });
          this.note(
            "SEG-Y import requires geometry review; opened the mapping recovery dialog.",
            "ui",
            "warn",
            describePreflight(preflight)
          );
          return;
        }

        throw new Error(
          `This SEG-Y survey cannot be opened automatically yet. Resolved layout: ${describePreflight(preflight)}. Suggested action: ${preflight.suggested_action}.`
        );
      }
      this.loading = false;
      this.busyLabel = null;
      await this.importDataset({
        inputPath: normalizedPath,
        outputStorePath,
        entryId: matchingEntry?.entry_id ?? null,
        sourcePath: normalizedPath,
        sessionPipelines: cloneSessionPipelines(matchingEntry?.session_pipelines),
        activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
        makeActive: shouldActivateOpenedVolume,
        loadSection: shouldActivateOpenedVolume,
        reuseExistingStore: true
      });
    } catch (error) {
      this.loading = false;
      this.busyLabel = null;
      this.error = errorMessage(error, "Failed to open the selected volume.");
      this.note("One-shot volume open failed.", "backend", "error", this.error);
    }
  };

  openImportGeometryRecovery = (
    preflight: SurveyPreflightResponse,
    importOptions: ImportDatasetOptions
  ): void => {
    const preferredIndex = suggestedCandidateIndex(preflight);
    const initialGeometry =
      preferredIndex >= 0
        ? preflight.geometry_candidates[preferredIndex]?.geometry
        : preflight.suggested_geometry_override ?? preflight.resolved_geometry;
    this.importGeometryRecovery = {
      inputPath: trimPath(importOptions.inputPath ?? ""),
      outputStorePath: trimPath(importOptions.outputStorePath ?? ""),
      preflight,
      importOptions: {
        ...importOptions,
        inputPath: trimPath(importOptions.inputPath ?? ""),
        outputStorePath: trimPath(importOptions.outputStorePath ?? "")
      },
      mode: preferredIndex >= 0 ? "candidate" : "manual",
      selectedCandidateIndex: preferredIndex,
      draft: geometryOverrideDraft(initialGeometry),
      working: false,
      error: null
    };
  };

  closeImportGeometryRecovery = (): void => {
    if (this.importGeometryRecovery?.working) {
      return;
    }
    this.importGeometryRecovery = null;
  };

  selectImportGeometryCandidate = (candidateIndex: number): void => {
    const state = this.importGeometryRecovery;
    if (!state || !state.preflight.geometry_candidates[candidateIndex]) {
      return;
    }
    const candidate = state.preflight.geometry_candidates[candidateIndex];
    this.importGeometryRecovery = {
      ...state,
      mode: "candidate",
      selectedCandidateIndex: candidateIndex,
      draft: geometryOverrideDraft(candidate.geometry),
      error: null
    };
  };

  setImportGeometryRecoveryMode = (mode: "candidate" | "manual"): void => {
    const state = this.importGeometryRecovery;
    if (!state) {
      return;
    }
    this.importGeometryRecovery = {
      ...state,
      mode,
      error: null
    };
  };

  setImportGeometryRecoveryDraft = (
    field: keyof GeometryOverrideDraft,
    value: string | SegyHeaderValueType
  ): void => {
    const state = this.importGeometryRecovery;
    if (!state) {
      return;
    }
    this.importGeometryRecovery = {
      ...state,
      mode: "manual",
      draft: {
        ...state.draft,
        [field]: value
      },
      error: null
    };
  };

  confirmImportGeometryRecovery = async (): Promise<void> => {
    const state = this.importGeometryRecovery;
    if (!state) {
      return;
    }

    const selectedCandidate =
      state.mode === "candidate" && state.selectedCandidateIndex >= 0
        ? state.preflight.geometry_candidates[state.selectedCandidateIndex] ?? null
        : null;
    const geometryOverride =
      selectedCandidate?.geometry ?? geometryOverrideFromDraft(state.draft);

    if (!geometryOverride?.inline_3d || !geometryOverride.crossline_3d) {
      this.importGeometryRecovery = {
        ...state,
        error: "Both inline and crossline header mappings are required before import."
      };
      return;
    }

    this.importGeometryRecovery = {
      ...state,
      working: true,
      error: null
    };

    try {
      const validatedPreflight = await preflightImport(state.inputPath, geometryOverride);
      this.preflight = validatedPreflight;
      if (!canAutoImportPreflight(validatedPreflight)) {
        throw new Error(
          `The selected geometry mapping still resolves as ${describePreflight(validatedPreflight)}.`
        );
      }

      this.importGeometryRecovery = null;
      await this.importDataset({
        ...state.importOptions,
        inputPath: state.inputPath,
        outputStorePath: state.outputStorePath,
        geometryOverride
      });
    } catch (error) {
      const message = errorMessage(error, "Failed to validate the selected geometry mapping.");
      const current = this.importGeometryRecovery;
      if (current) {
        this.importGeometryRecovery = {
          ...current,
          working: false,
          error: message
        };
      }
      this.note("Geometry recovery import failed.", "backend", "error", message);
    }
  };

  selectInputPath = async (inputPath: string): Promise<void> => {
    this.setInputPath(inputPath);
    const normalizedInputPath = trimPath(this.inputPath);
    const existingEntry = this.activeDatasetEntry;
    const reuseActiveEntry = existingEntry?.source_path === normalizedInputPath;
    const matchingEntry =
      this.workspaceEntries.find((entry) => entry.source_path === normalizedInputPath) ?? null;

    if (!reuseActiveEntry) {
      const suggestedStorePath = entryStorePath(matchingEntry) || deriveStorePathFromInput(normalizedInputPath);
      this.outputStorePath = suggestedStorePath;
      this.#outputPathSource = matchingEntry && entryStorePath(matchingEntry) ? "manual" : "auto";
      this.#clearLoadedDataset();
    }

    try {
      const response = await upsertDatasetEntry({
        schema_version: 1,
        entry_id: reuseActiveEntry ? this.activeEntryId : matchingEntry?.entry_id ?? null,
        display_name: null,
        source_path: normalizedInputPath || null,
        preferred_store_path: trimPath(this.outputStorePath) || null,
        imported_store_path: reuseActiveEntry ? existingEntry?.imported_store_path ?? null : null,
        dataset: reuseActiveEntry ? existingEntry?.last_dataset ?? null : null,
        session_pipelines: reuseActiveEntry ? existingEntry?.session_pipelines ?? [] : matchingEntry?.session_pipelines ?? null,
        active_session_pipeline_id:
          reuseActiveEntry
            ? existingEntry?.active_session_pipeline_id ?? null
            : matchingEntry?.active_session_pipeline_id ?? null,
        make_active: true
      });
      this.activeEntryId = response.entry.entry_id;
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, response.entry);
      this.#applyWorkspaceSession(response.session);
      this.refreshCompareSelection();
    } catch (error) {
      this.note(
        "Failed to register the selected SEG-Y path in the workspace.",
        "backend",
        "error",
        errorMessage(error, "Unknown workspace registry error")
      );
    }
  };

  setOutputStorePath = (outputStorePath: string): void => {
    const normalizedPath = trimPath(outputStorePath);
    this.outputStorePath = normalizedPath;
    this.error = null;
    this.#outputPathSource = "manual";
    this.note("Selected runtime store output path.", "ui", "info", normalizedPath);
  };

  selectOutputStorePath = async (outputStorePath: string): Promise<void> => {
    this.setOutputStorePath(outputStorePath);
    if (!this.activeEntryId && !trimPath(this.inputPath)) {
      return;
    }

    try {
      const response = await upsertDatasetEntry({
        schema_version: 1,
        entry_id: this.activeEntryId,
        display_name: null,
        source_path: trimPath(this.inputPath) || null,
        preferred_store_path: trimPath(this.outputStorePath) || null,
        imported_store_path: this.activeDatasetEntry?.imported_store_path ?? null,
        dataset: this.activeDatasetEntry?.last_dataset ?? null,
        session_pipelines: this.activeDatasetEntry?.session_pipelines ?? null,
        active_session_pipeline_id: this.activeDatasetEntry?.active_session_pipeline_id ?? null,
        make_active: true
      });
      this.activeEntryId = response.entry.entry_id;
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, response.entry);
      this.#applyWorkspaceSession(response.session);
      this.refreshCompareSelection();
    } catch (error) {
      this.note(
        "Failed to persist the selected runtime store path.",
        "backend",
        "error",
        errorMessage(error, "Unknown workspace registry error")
      );
    }
  };

  get importIsRedundant(): boolean {
    return (
      trimPath(this.inputPath).length > 0 &&
      trimPath(this.outputStorePath).length > 0 &&
      trimPath(this.inputPath) === trimPath(this.lastImportedInputPath) &&
      trimPath(this.outputStorePath) === trimPath(this.lastImportedStorePath)
    );
  }

  get importDisabledReason(): string | null {
    if (!trimPath(this.inputPath) || !trimPath(this.outputStorePath)) {
      return "Select a source volume path and output store path.";
    }

    if (this.importIsRedundant) {
      return "This source volume is already imported to the selected runtime store. Change the input or output path to import again.";
    }

    return null;
  }

  setDiagnosticsStatus = (status: DiagnosticsStatus | null): void => {
    this.diagnosticsStatus = status;
    this.verboseDiagnostics = status?.verboseEnabled ?? this.verboseDiagnostics;
    if (status) {
      this.note("Connected to desktop diagnostics session.", "backend", "info", status.sessionLogPath);
    }
  };

  setVerboseDiagnostics = (enabled: boolean): void => {
    this.verboseDiagnostics = enabled;
  };

  addDiagnosticsEvent = (event: DiagnosticsEvent): void => {
    this.backendEvents = capEntries(this.backendEvents, event, 20);
  };

  setRenderMode = (renderMode: (typeof this.displayTransform)["renderMode"]): void => {
    this.displayTransform.renderMode = renderMode;
  };

  setColormap = (colormap: (typeof this.displayTransform)["colormap"]): void => {
    this.displayTransform.colormap = colormap;
  };

  setGain = (gain: number): void => {
    this.displayTransform.gain = gain;
  };

  setPolarity = (polarity: (typeof this.displayTransform)["polarity"]): void => {
    this.displayTransform.polarity = polarity;
  };

  setClipRange = (clipMin: number | undefined, clipMax: number | undefined): void => {
    this.displayTransform.clipMin = clipMin;
    this.displayTransform.clipMax = clipMax;
  };

  setChartTool = (tool: SeismicChartTool): void => {
    this.chartTool = tool;
  };

  setProbe = (event: SectionProbeChanged): void => {
    this.lastProbe = event;
  };

  setViewport = (event: SectionViewportChanged): void => {
    this.#rememberDisplayedViewport(event.viewport);
    this.scheduleSectionTileRefresh(event);
  };

  setInteraction = (event: SectionInteractionChanged): void => {
    this.lastInteraction = event;
  };

  setInteractionState = (state: SeismicChartInteractionState): void => {
    this.chartTool = state.tool;
  };

  private canUseSectionTiles(): boolean {
    return (
      this.tauriRuntime &&
      this.sectionDomain === "time" &&
      !this.compareSplitEnabled &&
      !this.showVelocityOverlay &&
      this.sectionScalarOverlays.length === 0 &&
      this.section !== null &&
      trimPath(this.activeStorePath).length > 0
    );
  }

  private scheduleSectionTileRefresh(event: SectionViewportChanged): void {
    if (this.#sectionTileViewportTimer !== null) {
      clearTimeout(this.#sectionTileViewportTimer);
      this.#sectionTileViewportTimer = null;
    }
    if (!this.canUseSectionTiles()) {
      return;
    }
    this.#sectionTileViewportTimer = setTimeout(() => {
      this.#sectionTileViewportTimer = null;
      void this.refreshSectionTileForViewport(event.viewport);
    }, SECTION_TILE_VIEWPORT_DEBOUNCE_MS);
  }

  private async refreshSectionTileForViewport(
    viewport: SectionViewportChanged["viewport"]
  ): Promise<void> {
    if (!this.canUseSectionTiles() || !this.section) {
      return;
    }

    this.sectionTileStats.viewportRequests += 1;
    const request = buildSectionTileRequest(this.section, viewport);
    const cacheKey = tileCacheKey(this.activeStorePath, this.axis, this.index, request);
    const cached = this.#sectionTileCache.get(cacheKey);
    if (cached) {
      cached.lastUsedAt = nowMs();
      this.sectionTileStats.cacheHits += 1;
      this.#emitSectionTileDiagnostics(
        "debug",
        "Viewport request satisfied from section tile cache.",
        request,
        {
          source: "cache_hit",
          cacheKey,
          payloadBytes: cached.bytes,
          hitRate:
            this.sectionTileStats.cacheHits + this.sectionTileStats.fetches > 0
              ? this.sectionTileStats.cacheHits / (this.sectionTileStats.cacheHits + this.sectionTileStats.fetches)
              : null
        }
      );
      this.section = cached.view;
      void this.prefetchNeighborSectionTiles(request);
      return;
    }

    const requestId = ++this.#sectionTileLoadRequestId;
    const fetchStartedMs = nowMs();
    try {
      const logical = sectionLogicalDimensions(this.section);
      this.sectionTileStats.fetches += 1;
      const tile = await fetchSectionTileView(
        this.activeStorePath,
        this.axis,
        this.index,
        request.traceRange,
        request.sampleRange,
        request.lod
      );
      if (requestId !== this.#sectionTileLoadRequestId) {
        return;
      }
      const windowed = tileViewToWindowedSection(tile, logical);
      this.storeSectionTileCacheEntry(cacheKey, windowed);
      this.section = windowed;
      const elapsedMs = nowMs() - fetchStartedMs;
      this.#emitSectionTileDiagnostics(
        "info",
        "Loaded section tile for the active viewport.",
        request,
        {
          source: "viewport_fetch",
          cacheKey,
          elapsedMs,
          payloadBytes: estimateSectionPayloadBytes(windowed),
          traceStep: tile.trace_step,
          sampleStep: tile.sample_step
        },
        { mirrorToActivity: true }
      );
      void this.prefetchNeighborSectionTiles(request);
    } catch (error) {
      this.sectionTileStats.fetchErrors += 1;
      this.note(
        "Section tile fetch fell back to the current section payload.",
        "backend",
        "warn",
        error instanceof Error ? error.message : String(error)
      );
      this.#emitSectionTileDiagnostics(
        "warn",
        "Section tile fetch fell back to the current section payload.",
        request,
        {
          source: "viewport_fetch_error",
          error: error instanceof Error ? error.message : String(error)
        }
      );
    }
  }

  private async prefetchNeighborSectionTiles(request: SectionTileWindowRequest): Promise<void> {
    if (!this.canUseSectionTiles() || !this.section) {
      return;
    }
    const logical = sectionLogicalDimensions(this.section);
    const requestId = ++this.#sectionTilePrefetchRequestId;
    const neighborIndices = [this.index - 1, this.index + 1].filter(
      (candidate) => candidate >= 0 && candidate < this.sectionCountForAxis(this.axis)
    );

    for (const neighborIndex of neighborIndices) {
      if (requestId !== this.#sectionTilePrefetchRequestId) {
        return;
      }
      const cacheKey = tileCacheKey(this.activeStorePath, this.axis, neighborIndex, request);
      if (this.#sectionTileCache.has(cacheKey)) {
        this.sectionTileStats.prefetchCacheHits += 1;
        this.#emitSectionTileDiagnostics(
          "debug",
          "Adjacent section tile already present in cache.",
          request,
          {
            source: "prefetch_cache_hit",
            cacheKey
          },
          {
            sectionIndex: neighborIndex
          }
        );
        continue;
      }
      try {
        this.sectionTileStats.prefetchRequests += 1;
        const prefetchStartedMs = nowMs();
        const tile = await fetchSectionTileView(
          this.activeStorePath,
          this.axis,
          neighborIndex,
          request.traceRange,
          request.sampleRange,
          request.lod
        );
        const windowed = tileViewToWindowedSection(tile, logical);
        this.storeSectionTileCacheEntry(cacheKey, windowed);
        this.#emitSectionTileDiagnostics(
          "debug",
          "Prefetched adjacent section tile.",
          request,
          {
            source: "prefetch_fetch",
            cacheKey,
            elapsedMs: nowMs() - prefetchStartedMs,
            payloadBytes: estimateSectionPayloadBytes(windowed),
            traceStep: tile.trace_step,
            sampleStep: tile.sample_step
          },
          {
            sectionIndex: neighborIndex
          }
        );
      } catch (error) {
        this.sectionTileStats.prefetchErrors += 1;
        this.#emitSectionTileDiagnostics(
          "debug",
          "Adjacent section tile prefetch failed.",
          request,
          {
            source: "prefetch_error",
            error: error instanceof Error ? error.message : String(error)
          },
          {
            sectionIndex: neighborIndex
          }
        );
        return;
      }
    }
  }

  private sectionCountForAxis(axis: SectionAxis): number {
    const summary = this.dataset?.descriptor;
    return axis === "inline" ? summary?.shape[0] ?? 0 : summary?.shape[1] ?? 0;
  }

  private storeSectionTileCacheEntry(key: string, view: TransportWindowedSectionView): void {
    const bytes = estimateSectionPayloadBytes(view);
    const existing = this.#sectionTileCache.get(key);
    if (existing) {
      this.#sectionTileCacheBytes -= existing.bytes;
    }
    this.#sectionTileCache.set(key, {
      key,
      view,
      bytes,
      lastUsedAt: nowMs()
    });
    this.#sectionTileCacheBytes += bytes;
    this.trimSectionTileCache();
    this.sectionTileStats.cachedBytes = this.#sectionTileCacheBytes;
  }

  private trimSectionTileCache(): void {
    if (this.#sectionTileCacheBytes <= SECTION_TILE_CACHE_BUDGET_BYTES) {
      return;
    }
    const bytesBeforeTrim = this.#sectionTileCacheBytes;
    let evictedEntries = 0;
    const entries = [...this.#sectionTileCache.values()].sort((left, right) => left.lastUsedAt - right.lastUsedAt);
    for (const entry of entries) {
      if (this.#sectionTileCacheBytes <= SECTION_TILE_CACHE_BUDGET_BYTES) {
        break;
      }
      this.#sectionTileCache.delete(entry.key);
      this.#sectionTileCacheBytes -= entry.bytes;
      this.sectionTileStats.evictions += 1;
      evictedEntries += 1;
    }
    this.sectionTileStats.cachedBytes = this.#sectionTileCacheBytes;
    if (evictedEntries > 0) {
      const viewport = this.displayedViewport;
      const request = viewport && this.section ? buildSectionTileRequest(this.section, viewport) : null;
      if (request) {
        this.#emitSectionTileDiagnostics(
          "debug",
          "Trimmed the section tile cache to the configured budget.",
          request,
          {
            source: "cache_trim",
            evictedEntries,
            bytesBeforeTrim,
            bytesAfterTrim: this.#sectionTileCacheBytes,
            bytesFreed: bytesBeforeTrim - this.#sectionTileCacheBytes
          }
        );
      }
    }
  }

  mountShell = (): (() => void) => {
    this.note(
      `App shell mounted in ${this.tauriRuntime ? "Tauri" : "browser"} mode.`,
      "viewer",
      "info"
    );

    if (!this.tauriRuntime) {
      return () => {};
    }

    let cancelled = false;

    void (async () => {
      const workspace = await loadWorkspaceState();
      if (cancelled) {
        return;
      }

      this.#syncWorkspaceState(workspace.entries, workspace.session);
      if (trimPath(workspace.session.project_root ?? "")) {
        await this.loadProjectGeospatialSettings(workspace.session.project_root ?? "");
        await this.refreshProjectWellOverlayInventory(
          workspace.session.project_root ?? "",
          this.displayCoordinateReferenceId
        );
      }
      if (workspace.session.active_store_path) {
        this.restoringWorkspace = true;
        this.note("Restoring previous workspace dataset.", "viewer", "info", workspace.session.active_store_path);
        try {
          await this.openDatasetAt(
            workspace.session.active_store_path,
            workspace.session.active_axis,
            workspace.session.active_index,
            {
              promptForMissingNativeCoordinateReference: false
            }
          );
        } catch (error) {
          this.note(
            "Failed to restore the previous active dataset automatically.",
            "backend",
            "warn",
            errorMessage(error, "Unknown workspace restore error")
          );
        } finally {
          this.restoringWorkspace = false;
        }
      }

      const status = await getDiagnosticsStatus();
      if (cancelled) {
        return;
      }

      this.setDiagnosticsStatus(status);
      const unlisten = await listenToDiagnosticsEvents((event) => {
        this.addDiagnosticsEvent(event);
      });

      if (cancelled) {
        unlisten();
        return;
      }

      this.#diagnosticsUnlisten = unlisten;
    })();

    return () => {
      cancelled = true;
      this.#diagnosticsUnlisten?.();
      this.#diagnosticsUnlisten = null;
    };
  };

  updateDiagnosticsVerbosity = async (enabled: boolean): Promise<void> => {
    this.setVerboseDiagnostics(enabled);
    this.note(
      enabled ? "Requested verbose backend diagnostics." : "Requested standard backend diagnostics.",
      "ui",
      "info"
    );

    try {
      await setDiagnosticsVerbosity(enabled);
    } catch (error) {
      this.setVerboseDiagnostics(!enabled);
      this.note(
        "Failed to update diagnostics verbosity.",
        "backend",
        "error",
        error instanceof Error ? error.message : "Unknown verbosity error"
      );
    }
  };

  activateDatasetEntry = async (entryId: string): Promise<void> => {
    try {
      const response = await setActiveDatasetEntry(entryId);
      this.activeEntryId = response.entry.entry_id;
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, response.entry);
      this.#applyWorkspaceSession(response.session);
      this.#applyWorkspaceEntry(response.entry);
      this.refreshCompareSelection();
      this.note("Activated dataset entry from the workspace list.", "ui", "info", response.entry.display_name);

      const storePath = trimPath(entryStorePath(response.entry));
      if (storePath) {
        await this.openDatasetAt(
          storePath,
          this.axis,
          this.index
        );
      }
    } catch (error) {
      this.error = errorMessage(error, "Failed to activate dataset entry.");
      this.note("Failed to activate dataset entry.", "backend", "error", this.error);
    }
  };

  removeWorkspaceEntry = async (entryId: string): Promise<void> => {
    try {
      const response = await removeDatasetEntry(entryId);
      const removedActive = this.activeEntryId === entryId;
      this.workspaceEntries = this.workspaceEntries.filter((entry) => entry.entry_id !== entryId);
      this.#applyWorkspaceSession(response.session);
      this.refreshCompareSelection();
      if (removedActive) {
        this.inputPath = "";
        this.outputStorePath = "";
        this.#clearLoadedDataset();
        this.preflight = null;
      }
      this.note("Removed dataset entry from the workspace list.", "ui", "warn", entryId);
    } catch (error) {
      this.error = errorMessage(error, "Failed to remove dataset entry.");
      this.note("Failed to remove dataset entry.", "backend", "error", this.error);
    }
  };

  copyActiveWorkspaceEntry = (): void => {
    const activeEntry = this.activeDatasetEntry;
    if (!activeEntry) {
      return;
    }

    this.#copiedWorkspaceEntry = structuredClone(activeEntry);
    this.note(
      "Copied active dataset entry.",
      "ui",
      "info",
      userVisibleDatasetName(
        activeEntry.display_name,
        activeEntry.source_path,
        activeEntry.imported_store_path ?? activeEntry.preferred_store_path,
        activeEntry.entry_id
      )
    );
  };

  pasteCopiedWorkspaceEntry = async (): Promise<void> => {
    const copiedEntry = this.#copiedWorkspaceEntry;
    if (!copiedEntry) {
      return;
    }

    const storePath = trimPath(entryStorePath(copiedEntry));
    if (!storePath) {
      this.note("Copied dataset entry has no runtime store path.", "ui", "warn", copiedEntry.entry_id);
      return;
    }

    const nextDisplayName = nextDuplicateName(
      userVisibleDatasetName(
        copiedEntry.display_name,
        copiedEntry.source_path,
        copiedEntry.imported_store_path ?? copiedEntry.preferred_store_path,
        copiedEntry.entry_id
      ),
      this.workspaceEntries.map((entry) =>
        userVisibleDatasetName(
          entry.display_name,
          entry.source_path,
          entry.imported_store_path ?? entry.preferred_store_path,
          entry.entry_id
        )
      )
    );

    await this.openDatasetAt(storePath, this.axis, this.index, {
      entryId: this.nextWorkspaceEntryId(),
      displayName: nextDisplayName,
      sourcePath: copiedEntry.source_path,
      sessionPipelines: cloneSessionPipelines(copiedEntry.session_pipelines),
      activeSessionPipelineId: copiedEntry.active_session_pipeline_id,
      makeActive: true,
      loadSection: true
    });
  };

  openDatasetAt = async (
    storePath: string,
    axis: SectionAxis = "inline",
    index = 0,
    options: OpenDatasetOptions = {}
  ): Promise<void> => {
    const normalizedStorePath = trimPath(storePath);
    if (!normalizedStorePath) {
      throw new Error("Store path is required.");
    }

    this.loading = true;
    this.busyLabel = this.restoringWorkspace ? "Restoring dataset" : "Opening dataset";
    this.error = null;
    this.note("Opening runtime store.", "ui", "info", normalizedStorePath);

    const hasOwnOption = (key: keyof OpenDatasetOptions): boolean =>
      Object.prototype.hasOwnProperty.call(options, key);
    const matchingActiveEntry =
      trimPath(entryStorePath(this.activeDatasetEntry)) === normalizedStorePath ? this.activeDatasetEntry : null;
    const nextEntryId: string | null = hasOwnOption("entryId")
      ? options.entryId ?? null
      : this.activeEntryId;
    const nextDisplayName: string | null = hasOwnOption("displayName")
      ? options.displayName ?? null
      : matchingActiveEntry?.display_name ?? null;
    const nextSourcePath: string = hasOwnOption("sourcePath")
      ? options.sourcePath ?? ""
      : matchingActiveEntry?.source_path ?? this.inputPath;
    const nextSessionPipelines: WorkspacePipelineEntry[] | null = hasOwnOption("sessionPipelines")
      ? cloneSessionPipelines(options.sessionPipelines)
      : this.activeDatasetEntry?.session_pipelines ?? null;
    const nextActiveSessionPipelineId: string | null = hasOwnOption("activeSessionPipelineId")
      ? options.activeSessionPipelineId ?? null
      : this.activeDatasetEntry?.active_session_pipeline_id ?? null;
    const makeActive = options.makeActive ?? true;
    const loadSection = options.loadSection ?? makeActive;
    const previousActiveStorePath = trimPath(this.activeStorePath);
    const promptForMissingNativeCoordinateReference =
      options.promptForMissingNativeCoordinateReference ?? true;

    try {
      const response = await openDataset(normalizedStorePath);

      const workspaceResponse = await upsertDatasetEntry({
        schema_version: 1,
        entry_id: nextEntryId,
        display_name:
          nextDisplayName?.trim() ||
          userVisibleDatasetName(
            response.dataset.descriptor.label,
            trimPath(nextSourcePath) || null,
            response.dataset.store_path,
            nextEntryId ?? response.dataset.descriptor.id
          ),
        source_path: trimPath(nextSourcePath) || null,
        preferred_store_path: response.dataset.store_path,
        imported_store_path: response.dataset.store_path,
        dataset: response.dataset,
        session_pipelines: nextSessionPipelines,
        active_session_pipeline_id: nextActiveSessionPipelineId,
        make_active: makeActive
      });
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, workspaceResponse.entry);
      this.refreshCompareSelection();

      if (makeActive) {
        if (
          previousActiveStorePath &&
          previousActiveStorePath !== response.dataset.store_path
        ) {
          this.#evictSectionTileCacheForStore(previousActiveStorePath);
        }
        this.dataset = response.dataset;
        this.activeStorePath = response.dataset.store_path;
        this.outputStorePath = response.dataset.store_path;
        this.#outputPathSource = "manual";
        this.inputPath = trimPath(nextSourcePath);
        this.error = null;
        this.activeEntryId = workspaceResponse.entry.entry_id;
        this.#applyWorkspaceSession(workspaceResponse.session);
        await this.refreshVelocityModels(response.dataset.store_path);
        await this.refreshHorizonAssets(response.dataset.store_path);
        void this.refreshSurveyMap();

        this.note(
          "Runtime store opened.",
          "backend",
          "info",
          `${response.dataset.descriptor.label} @ ${response.dataset.store_path}`
        );
        this.#notePotentiallyLossySampleData(response.dataset.descriptor.sample_data_fidelity, "dataset");
        if (loadSection) {
          await this.load(axis, index, response.dataset.store_path);
        } else {
          this.loading = false;
          this.busyLabel = null;
        }
        this.#maybeQueueMissingNativeCoordinateReferencePrompt(
          response.dataset,
          trimPath(nextSourcePath) || null,
          "open",
          {
            makeActive,
            promptRequested: promptForMissingNativeCoordinateReference
          }
        );
      } else {
        this.loading = false;
        this.busyLabel = null;
        this.error = null;
        this.note(
          "Volume added to the workspace without changing the active seismic view.",
          "backend",
          "info",
          `${response.dataset.descriptor.label} @ ${response.dataset.store_path}`
        );
      }
    } catch (error) {
      this.loading = false;
      this.busyLabel = null;
      this.error = errorMessage(error, "Unknown open-store error");
      this.note("Opening runtime store failed.", "backend", "error", this.error);
      throw error;
    }
  };

  openDerivedDatasetAt = async (
    storePath: string,
    axis: SectionAxis = "inline",
    index = 0
  ): Promise<void> => {
    const activeEntry = this.activeDatasetEntry;
    await this.openDatasetAt(storePath, axis, index, {
      entryId: null,
      sourcePath: null,
      sessionPipelines: cloneSessionPipelines(activeEntry?.session_pipelines),
      activeSessionPipelineId: activeEntry?.active_session_pipeline_id ?? null
    });
  };

  private nextWorkspaceEntryId(): string {
    this.#workspaceEntryCounter += 1;
    return `dataset-copy-${Date.now()}-${this.#workspaceEntryCounter}`;
  }

  runPreflight = async (): Promise<void> => {
    const inputPath = this.inputPath.trim();
    const extension = fileExtension(inputPath);
    if (!inputPath) {
      this.loading = false;
      this.busyLabel = null;
      this.error = "Input SEG-Y path is required.";
      this.note("Preflight blocked because no SEG-Y path was provided.", "ui", "error");
      return;
    }
    if (!isSegyVolumeExtension(extension)) {
      const volumeType = describeImportVolumeType(extension);
      this.error = `${volumeType} inputs do not use SEG-Y preflight. Import them directly.`;
      this.note(
        "Preflight blocked because the selected input is not a SEG-Y survey.",
        "ui",
        "warn",
        inputPath
      );
      return;
    }

    this.loading = true;
    this.busyLabel = "Preflighting survey";
    this.error = null;
    this.note("Started survey preflight.", "ui", "info", inputPath || null);

    try {
      const preflight = await preflightImport(inputPath);
      this.loading = false;
      this.busyLabel = null;
      this.preflight = preflight;
      this.error = null;
      this.note(
        `Preflight completed as ${describePreflight(preflight)}.`,
        "backend",
        preflight.suggested_action === "direct_dense_ingest" ? "info" : "warn",
        `Suggested action: ${preflight.suggested_action}`
      );
      this.#notePotentiallyLossySampleData(preflight.sample_data_fidelity, "preflight");
    } catch (error) {
      this.loading = false;
      this.busyLabel = null;
      this.error = error instanceof Error ? error.message : "Unknown preflight error";
      this.note(
        "Preflight failed.",
        "backend",
        "error",
        error instanceof Error ? error.message : "Unknown preflight error"
      );
    }
  };

  importDataset = async (options: ImportDatasetOptions = {}): Promise<void> => {
    const hasOwnOption = (key: keyof ImportDatasetOptions): boolean =>
      Object.prototype.hasOwnProperty.call(options, key);
    const inputPath = trimPath(hasOwnOption("inputPath") ? options.inputPath ?? "" : this.inputPath);
    const outputStorePath = trimPath(
      hasOwnOption("outputStorePath") ? options.outputStorePath ?? "" : this.outputStorePath
    );
    const nextEntryId: string | null = hasOwnOption("entryId") ? options.entryId ?? null : this.activeEntryId;
    const nextSourcePath = hasOwnOption("sourcePath") ? options.sourcePath ?? inputPath : inputPath;
    const nextSessionPipelines: WorkspacePipelineEntry[] | null = hasOwnOption("sessionPipelines")
      ? cloneSessionPipelines(options.sessionPipelines)
      : this.activeDatasetEntry?.session_pipelines ?? null;
    const nextActiveSessionPipelineId: string | null = hasOwnOption("activeSessionPipelineId")
      ? options.activeSessionPipelineId ?? null
      : this.activeDatasetEntry?.active_session_pipeline_id ?? null;
    const makeActive = options.makeActive ?? true;
    const loadSection = options.loadSection ?? makeActive;
    const promptForMissingNativeCoordinateReference =
      options.promptForMissingNativeCoordinateReference ?? true;
    const reuseExistingStore = options.reuseExistingStore ?? false;
    const geometryOverride = hasOwnOption("geometryOverride") ? options.geometryOverride ?? null : null;
    this.loading = true;
    this.busyLabel = "Importing survey";
    this.error = null;
    this.note(
      "Started dataset import.",
      "ui",
      "info",
      `${inputPath || "(missing input)"} -> ${outputStorePath || "(missing output)"}`
    );

    if (!inputPath || !outputStorePath) {
      this.loading = false;
      this.busyLabel = null;
      this.error = "Both input source volume path and output store path are required.";
      this.note("Import blocked because input or output path is missing.", "ui", "error");
      return;
    }

    try {
      let response: ImportDatasetResponse;

      try {
        response = await importDataset(inputPath, outputStorePath, false, geometryOverride);
      } catch (error) {
        const message = errorMessage(error, "Unknown import error");
        if (!isExistingStoreError(message)) {
          throw error;
        }

        if (reuseExistingStore) {
          this.loading = false;
          this.busyLabel = null;
          this.error = null;
          this.note(
            "An imported runtime store already exists for this source volume; reusing it instead of re-importing.",
            "backend",
            "info",
            outputStorePath
          );
          await this.openDatasetAt(outputStorePath, "inline", 0, {
            entryId: nextEntryId,
            sourcePath: nextSourcePath,
            sessionPipelines: nextSessionPipelines,
            activeSessionPipelineId: nextActiveSessionPipelineId,
            makeActive,
            loadSection
          });
          return;
        }

        this.loading = false;
        this.busyLabel = null;
        this.error = message;
        this.note(
          "Runtime store already exists; waiting for overwrite confirmation.",
          "backend",
          "warn",
          outputStorePath
        );

        const confirmed = await confirmOverwriteStore(outputStorePath);
        if (!confirmed) {
          this.error = "Import cancelled because the selected runtime store already exists.";
          this.note(
            "Overwrite of the existing runtime store was cancelled.",
            "ui",
            "warn",
            outputStorePath
          );
          return;
        }

        this.loading = true;
        this.busyLabel = "Overwriting runtime store";
        this.error = null;
        this.note("Confirmed overwrite of the existing runtime store.", "ui", "warn", outputStorePath);
        response = await importDataset(inputPath, outputStorePath, true, geometryOverride);
      }

      this.loading = false;
      this.busyLabel = null;
      this.lastImportedInputPath = inputPath;
      this.lastImportedStorePath = response.dataset.store_path;
      const workspaceResponse = await upsertDatasetEntry({
        schema_version: 1,
        entry_id: nextEntryId,
        display_name: userVisibleDatasetName(
          response.dataset.descriptor.label,
          trimPath(nextSourcePath) || null,
          response.dataset.store_path,
          nextEntryId ?? response.dataset.descriptor.id
        ),
        source_path: trimPath(nextSourcePath) || null,
        preferred_store_path: response.dataset.store_path,
        imported_store_path: response.dataset.store_path,
        dataset: response.dataset,
        session_pipelines: nextSessionPipelines,
        active_session_pipeline_id: nextActiveSessionPipelineId,
        make_active: makeActive
      });
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, workspaceResponse.entry);
      this.refreshCompareSelection();
      if (makeActive) {
        this.dataset = response.dataset;
        this.activeStorePath = response.dataset.store_path;
        this.outputStorePath = response.dataset.store_path;
        this.#outputPathSource = "manual";
        this.inputPath = inputPath;
        this.error = null;
        this.activeEntryId = workspaceResponse.entry.entry_id;
        this.#applyWorkspaceSession(workspaceResponse.session);
        void this.refreshSurveyMap();
        this.note(
          "Dataset import completed.",
          "backend",
          "info",
          `${response.dataset.descriptor.label} @ ${response.dataset.store_path}`
        );
        this.#notePotentiallyLossySampleData(response.dataset.descriptor.sample_data_fidelity, "dataset");
        if (loadSection) {
          await this.load("inline", 0, response.dataset.store_path);
        }
        this.#maybeQueueMissingNativeCoordinateReferencePrompt(response.dataset, inputPath, "import", {
          makeActive,
          promptRequested: promptForMissingNativeCoordinateReference
        });
      } else {
        this.error = null;
        this.note(
          "Survey import completed and the volume was added to the workspace without changing the active seismic view.",
          "backend",
          "info",
          `${response.dataset.descriptor.label} @ ${response.dataset.store_path}`
        );
        this.#notePotentiallyLossySampleData(response.dataset.descriptor.sample_data_fidelity, "dataset");
      }
    } catch (error) {
      this.loading = false;
      this.busyLabel = null;
      this.error = errorMessage(error, "Unknown import error");
      this.note(
        "Dataset import failed.",
        "backend",
        "error",
        errorMessage(error, "Unknown import error")
      );
    }
  };

  importSegySurveyPlan = async (
    plan: SegyImportPlan,
    validationFingerprint: string,
    options: ImportDatasetOptions = {}
  ): Promise<void> => {
    const hasOwnOption = (key: keyof ImportDatasetOptions): boolean =>
      Object.prototype.hasOwnProperty.call(options, key);
    const inputPath = trimPath(plan.input_path);
    const outputStorePath = trimPath(plan.policy.output_store_path);
    const nextEntryId: string | null = hasOwnOption("entryId") ? options.entryId ?? null : this.activeEntryId;
    const nextSourcePath = hasOwnOption("sourcePath") ? options.sourcePath ?? inputPath : inputPath;
    const nextSessionPipelines: WorkspacePipelineEntry[] | null = hasOwnOption("sessionPipelines")
      ? cloneSessionPipelines(options.sessionPipelines)
      : this.activeDatasetEntry?.session_pipelines ?? null;
    const nextActiveSessionPipelineId: string | null = hasOwnOption("activeSessionPipelineId")
      ? options.activeSessionPipelineId ?? null
      : this.activeDatasetEntry?.active_session_pipeline_id ?? null;
    const makeActive = options.makeActive ?? true;
    const loadSection = options.loadSection ?? makeActive;
    const promptForMissingNativeCoordinateReference =
      options.promptForMissingNativeCoordinateReference ?? true;
    const reuseExistingStore = options.reuseExistingStore ?? false;
    this.loading = true;
    this.busyLabel = "Importing survey";
    this.error = null;
    this.note(
      "Started validated SEG-Y import.",
      "ui",
      "info",
      `${inputPath || "(missing input)"} -> ${outputStorePath || "(missing output)"}`
    );

    if (!inputPath || !outputStorePath) {
      this.loading = false;
      this.busyLabel = null;
      this.error = "Both input source volume path and output store path are required.";
      this.note("Import blocked because input or output path is missing.", "ui", "error");
      return;
    }

    try {
      let response: ImportSegyWithPlanResponse;
      let currentPlan = structuredClone(plan);
      currentPlan.input_path = inputPath;
      currentPlan.policy.output_store_path = outputStorePath;
      let currentValidationFingerprint = validationFingerprint;

      try {
        response = await importSegyWithPlan(currentPlan, currentValidationFingerprint);
      } catch (error) {
        const message = errorMessage(error, "Unknown SEG-Y import error");
        if (!isExistingStoreError(message)) {
          throw error;
        }

        if (reuseExistingStore) {
          this.loading = false;
          this.busyLabel = null;
          this.error = null;
          this.note(
            "An imported runtime store already exists for this source volume; reusing it instead of re-importing.",
            "backend",
            "info",
            outputStorePath
          );
          await this.openDatasetAt(outputStorePath, "inline", 0, {
            entryId: nextEntryId,
            sourcePath: nextSourcePath,
            sessionPipelines: nextSessionPipelines,
            activeSessionPipelineId: nextActiveSessionPipelineId,
            makeActive,
            loadSection
          });
          return;
        }

        this.loading = false;
        this.busyLabel = null;
        this.error = message;
        this.note(
          "Runtime store already exists; waiting for overwrite confirmation.",
          "backend",
          "warn",
          outputStorePath
        );

        const confirmed = await confirmOverwriteStore(outputStorePath);
        if (!confirmed) {
          this.error = "Import cancelled because the selected runtime store already exists.";
          this.note(
            "Overwrite of the existing runtime store was cancelled.",
            "ui",
            "warn",
            outputStorePath
          );
          return;
        }

        this.loading = true;
        this.busyLabel = "Overwriting runtime store";
        this.error = null;
        this.note("Confirmed overwrite of the existing runtime store.", "ui", "warn", outputStorePath);
        currentPlan = {
          ...currentPlan,
          policy: {
            ...currentPlan.policy,
            overwrite_existing: true
          }
        };
        const validated = await validateSegyImportPlan(currentPlan);
        currentPlan = validated.validated_plan;
        currentValidationFingerprint = validated.validation_fingerprint;
        response = await importSegyWithPlan(currentPlan, currentValidationFingerprint);
      }

      this.loading = false;
      this.busyLabel = null;
      this.lastImportedInputPath = inputPath;
      this.lastImportedStorePath = response.dataset.store_path;
      const workspaceResponse = await upsertDatasetEntry({
        schema_version: 1,
        entry_id: nextEntryId,
        display_name: userVisibleDatasetName(
          response.dataset.descriptor.label,
          trimPath(nextSourcePath) || null,
          response.dataset.store_path,
          nextEntryId ?? response.dataset.descriptor.id
        ),
        source_path: trimPath(nextSourcePath) || null,
        preferred_store_path: response.dataset.store_path,
        imported_store_path: response.dataset.store_path,
        dataset: response.dataset,
        session_pipelines: nextSessionPipelines,
        active_session_pipeline_id: nextActiveSessionPipelineId,
        make_active: makeActive
      });
      this.workspaceEntries = mergeWorkspaceEntry(this.workspaceEntries, workspaceResponse.entry);
      this.refreshCompareSelection();
      if (makeActive) {
        this.dataset = response.dataset;
        this.activeStorePath = response.dataset.store_path;
        this.outputStorePath = response.dataset.store_path;
        this.#outputPathSource = "manual";
        this.inputPath = inputPath;
        this.error = null;
        this.activeEntryId = workspaceResponse.entry.entry_id;
        this.#applyWorkspaceSession(workspaceResponse.session);
        void this.refreshSurveyMap();
        this.note(
          "SEG-Y import completed.",
          "backend",
          "info",
          `${response.dataset.descriptor.label} @ ${response.dataset.store_path}`
        );
        this.#notePotentiallyLossySampleData(response.dataset.descriptor.sample_data_fidelity, "dataset");
        if (loadSection) {
          await this.load("inline", 0, response.dataset.store_path);
        }
        this.#maybeQueueMissingNativeCoordinateReferencePrompt(response.dataset, inputPath, "import", {
          makeActive,
          promptRequested: promptForMissingNativeCoordinateReference
        });
      } else {
        this.error = null;
        this.note(
          "SEG-Y import completed and the volume was added to the workspace without changing the active seismic view.",
          "backend",
          "info",
          `${response.dataset.descriptor.label} @ ${response.dataset.store_path}`
        );
        this.#notePotentiallyLossySampleData(response.dataset.descriptor.sample_data_fidelity, "dataset");
      }
    } catch (error) {
      this.loading = false;
      this.busyLabel = null;
      this.error = errorMessage(error, "Unknown SEG-Y import error");
      this.note(
        "SEG-Y import failed.",
        "backend",
        "error",
        errorMessage(error, "Unknown SEG-Y import error")
      );
    }
  };

  openDataset = async (): Promise<void> => {
    const storePath = this.outputStorePath.trim() || this.activeStorePath.trim();
    if (!storePath) {
      this.error = "Store path is required.";
      this.note("Open-store blocked because no runtime store path was provided.", "ui", "error");
      return;
    }

    try {
      await this.openDatasetAt(storePath, "inline", 0);
    } catch (error) {
      this.error = errorMessage(error, "Unknown open-store error");
    }
  };

  importHorizonFiles = async (
    inputPaths: string[],
    options: HorizonImportCoordinateReferenceOptions = {}
  ): Promise<ImportedHorizonDescriptor[] | null> => {
    const activeStorePath = this.activeStorePath.trim();
    const normalizedPaths = inputPaths.map(trimPath).filter((value) => value.length > 0);
    if (!activeStorePath) {
      this.error = "Open a runtime store before importing horizons.";
      this.note("Horizon import blocked because no active runtime store is open.", "ui", "error");
      return null;
    }
    if (normalizedPaths.length === 0) {
      return [];
    }

    this.horizonImporting = true;
    this.error = null;
    this.note(
      "Started horizon xyz import.",
      "ui",
      "info",
      `${normalizedPaths.length} file${normalizedPaths.length === 1 ? "" : "s"}`
    );

    try {
      const response = await importHorizonXyz(activeStorePath, normalizedPaths, options);
      return await this.#finalizeHorizonImport(activeStorePath, response.imported);
    } catch (error) {
      this.error = errorMessage(error, "Unknown horizon import error");
      this.note("Horizon import failed.", "backend", "error", this.error);
      return null;
    } finally {
      this.horizonImporting = false;
    }
  };

  importHorizonDraft = async (
    draft: HorizonSourceImportCanonicalDraft
  ): Promise<ImportedHorizonDescriptor[] | null> => {
    const activeStorePath = this.activeStorePath.trim();
    if (!activeStorePath) {
      this.error = "Open a runtime store before importing horizons.";
      this.note("Horizon import blocked because no active runtime store is open.", "ui", "error");
      return null;
    }

    const selectedPaths = draft.selectedSourcePaths
      .map(trimPath)
      .filter((value) => value.length > 0);
    if (selectedPaths.length === 0) {
      return [];
    }

    this.horizonImporting = true;
    this.error = null;
    this.note(
      "Started horizon source import.",
      "ui",
      "info",
      `${selectedPaths.length} file${selectedPaths.length === 1 ? "" : "s"}`
    );

    try {
      const response = await commitHorizonSourceImport({
        storePath: activeStorePath,
        draft: {
          ...draft,
          selectedSourcePaths: selectedPaths
        }
      });
      return await this.#finalizeHorizonImport(activeStorePath, response.imported);
    } catch (error) {
      this.error = errorMessage(error, "Unknown horizon import error");
      this.note("Horizon import failed.", "backend", "error", this.error);
      return null;
    } finally {
      this.horizonImporting = false;
    }
  };

  async #finalizeHorizonImport(
    storePath: string,
    imported: ImportedHorizonDescriptor[]
  ): Promise<ImportedHorizonDescriptor[]> {
    await this.refreshHorizonAssets(storePath);
    const display = await this.loadResolvedSectionDisplay(storePath, this.axis, this.index);
    this.section = display.section;
    this.timeDepthDiagnostics = display.time_depth_diagnostics;
    this.sectionScalarOverlays = adaptSectionScalarOverlays(
      display.scalar_overlays,
      this.velocityOverlayOpacity
    );
    this.sectionHorizons = adaptSectionHorizonOverlays(display.horizon_overlays);
    this.note(
      "Imported horizon xyz files into the active runtime store.",
      "backend",
      "info",
      imported.map((item) => item.name).join(", ")
    );
    return imported;
  }

  openActiveDatasetExportDialog = async (): Promise<void> => {
    await this.openDatasetExportDialog(this.activeEntryId);
  };

  openDatasetExportDialog = async (entryId: string | null): Promise<void> => {
    if (!this.tauriRuntime) {
      this.note("Dataset export is only available in the desktop app.", "ui", "warn");
      return;
    }

    const entry =
      (entryId ? this.workspaceEntries.find((candidate) => candidate.entry_id === entryId) : null) ??
      this.activeDatasetEntry;
    const storePath = trimPath(entryStorePath(entry) || this.activeStorePath);
    if (!storePath) {
      this.error = "Open or import a runtime store before exporting.";
      this.note("Dataset export blocked because no runtime store path is available.", "ui", "error");
      return;
    }

    try {
      const capabilities: DatasetExportCapabilitiesResponse = await getDatasetExportCapabilities(storePath);
      const displayName = userVisibleDatasetName(
        entry?.display_name ?? this.dataset?.descriptor.label ?? null,
        entry?.source_path ?? null,
        storePath,
        entry?.entry_id ?? this.dataset?.descriptor.id ?? "dataset"
      );
      this.datasetExportDialog = {
        entryId: entry?.entry_id ?? null,
        displayName,
        storePath,
        working: false,
        error: null,
        formats: {
          segy: {
            selected: capabilities.segy.available,
            available: capabilities.segy.available,
            reason: capabilities.segy.reason,
            path: capabilities.segy.defaultOutputPath || deriveSegyExportPathFromStore(storePath)
          },
          zarr: {
            selected: !capabilities.segy.available && capabilities.zarr.available,
            available: capabilities.zarr.available,
            reason: capabilities.zarr.reason,
            path: capabilities.zarr.defaultOutputPath || deriveZarrExportPathFromStore(storePath)
          }
        }
      };
    } catch (error) {
      this.error = errorMessage(error, "Failed to inspect dataset export capabilities.");
      this.note("Dataset export dialog failed to initialize.", "backend", "error", this.error);
    }
  };

  closeDatasetExportDialog = (): void => {
    if (this.datasetExportDialog?.working) {
      return;
    }
    this.datasetExportDialog = null;
  };

  setDatasetExportFormatSelected = (format: DatasetExportFormat, selected: boolean): void => {
    const dialog = this.datasetExportDialog;
    if (!dialog || !dialog.formats[format].available || dialog.working) {
      return;
    }
    this.datasetExportDialog = {
      ...dialog,
      error: null,
      formats: {
        ...dialog.formats,
        [format]: {
          ...dialog.formats[format],
          selected
        }
      }
    };
  };

  setDatasetExportPath = (format: DatasetExportFormat, path: string): void => {
    const dialog = this.datasetExportDialog;
    if (!dialog || dialog.working) {
      return;
    }
    this.datasetExportDialog = {
      ...dialog,
      error: null,
      formats: {
        ...dialog.formats,
        [format]: {
          ...dialog.formats[format],
          path
        }
      }
    };
  };

  browseDatasetExportPath = async (format: DatasetExportFormat): Promise<void> => {
    const dialog = this.datasetExportDialog;
    if (!dialog || dialog.working || !dialog.formats[format].available) {
      return;
    }

    const picker =
      format === "segy"
        ? pickSegyExportPath(dialog.formats.segy.path || "survey.export.sgy")
        : pickZarrExportPath(dialog.formats.zarr.path || "survey.export.zarr");
    const selectedPath = trimPath((await picker) ?? "");
    if (!selectedPath) {
      return;
    }
    this.setDatasetExportPath(format, selectedPath);
  };

  confirmDatasetExportDialog = async (): Promise<void> => {
    const dialog = this.datasetExportDialog;
    if (!dialog || dialog.working) {
      return;
    }

    const selectedFormats = (["segy", "zarr"] as const).filter(
      (format) => dialog.formats[format].available && dialog.formats[format].selected
    );
    if (selectedFormats.length === 0) {
      this.datasetExportDialog = {
        ...dialog,
        error: "Select at least one export format before continuing."
      };
      return;
    }

    for (const format of selectedFormats) {
      const outputPath = trimPath(dialog.formats[format].path);
      if (!outputPath) {
        this.datasetExportDialog = {
          ...dialog,
          error: `Choose an output path for ${format.toUpperCase()} export before continuing.`
        };
        return;
      }
    }

    this.datasetExportDialog = {
      ...dialog,
      working: true,
      error: null
    };
    this.datasetExporting = true;
    this.error = null;

    try {
      const exportedPaths: string[] = [];
      for (const format of selectedFormats) {
        const outputPath = trimPath(dialog.formats[format].path);

        if (format === "segy") {
          this.note("Started SEG-Y export.", "ui", "info", `${dialog.storePath} -> ${outputPath}`);
          let response: ExportSegyResponse;
          try {
            response = await exportDatasetSegy(dialog.storePath, outputPath, false);
          } catch (error) {
            const message = errorMessage(error, "Unknown SEG-Y export error");
            if (!isExistingSegyExportError(message)) {
              throw error;
            }

            this.note(
              "SEG-Y export target already exists; waiting for overwrite confirmation.",
              "backend",
              "warn",
              outputPath
            );
            const confirmed = await confirmOverwriteSegy(outputPath);
            if (!confirmed) {
              this.datasetExportDialog = {
                ...dialog,
                working: false,
                error: "SEG-Y export cancelled because overwrite was declined."
              };
              this.note("SEG-Y export overwrite was cancelled.", "ui", "warn", outputPath);
              return;
            }

            this.note("Confirmed overwrite of the existing SEG-Y export target.", "ui", "warn", outputPath);
            response = await exportDatasetSegy(dialog.storePath, outputPath, true);
          }

          exportedPaths.push(response.output_path);
          this.note("Exported dataset to SEG-Y.", "backend", "info", response.output_path);
          continue;
        }

        this.note("Started Zarr export.", "ui", "info", `${dialog.storePath} -> ${outputPath}`);
        let response: ExportZarrResponse;
        try {
          response = await exportDatasetZarr(dialog.storePath, outputPath, false);
        } catch (error) {
          const message = errorMessage(error, "Unknown Zarr export error");
          if (!isExistingZarrExportError(message)) {
            throw error;
          }

          this.note(
            "Zarr export target already exists; waiting for overwrite confirmation.",
            "backend",
            "warn",
            outputPath
          );
          const confirmed = await confirmOverwriteZarr(outputPath);
          if (!confirmed) {
            this.datasetExportDialog = {
              ...dialog,
              working: false,
              error: "Zarr export cancelled because overwrite was declined."
            };
            this.note("Zarr export overwrite was cancelled.", "ui", "warn", outputPath);
            return;
          }

          this.note("Confirmed overwrite of the existing Zarr export target.", "ui", "warn", outputPath);
          response = await exportDatasetZarr(dialog.storePath, outputPath, true);
        }

        exportedPaths.push(response.output_path);
        this.note("Exported dataset to Zarr.", "backend", "info", response.output_path);
      }

      this.datasetExportDialog = null;
      this.note(
        "Dataset export completed.",
        "backend",
        "info",
        exportedPaths.join(", ")
      );
    } catch (error) {
      const message = errorMessage(error, "Dataset export failed.");
      this.error = message;
      this.datasetExportDialog = {
        ...dialog,
        working: false,
        error: message
      };
      this.note("Dataset export failed.", "backend", "error", message);
    } finally {
      this.datasetExporting = false;
      if (this.datasetExportDialog) {
        this.datasetExportDialog = {
          ...this.datasetExportDialog,
          working: false
        };
      }
    }
  };

  load = async (axis: SectionAxis, index: number, storePathOverride?: string): Promise<void> => {
    const activeStorePath = (storePathOverride ?? this.activeStorePath).trim();
    this.activeStorePath = storePathOverride ?? this.activeStorePath;
    this.axis = axis;
    this.index = index;
    this.#sectionTileLoadRequestId += 1;
    this.#sectionTilePrefetchRequestId += 1;
    if (this.#sectionTileViewportTimer !== null) {
      clearTimeout(this.#sectionTileViewportTimer);
      this.#sectionTileViewportTimer = null;
    }
    this.loading = true;
    this.busyLabel = this.sectionDomain === "depth" ? "Converting section to depth" : "Loading section";
    this.error = null;
    this.note(
      "Requested section load.",
      "ui",
      "info",
      `${axis}:${index} (${this.sectionDomain === "depth" ? "depth" : "time"})`
    );

    if (!activeStorePath) {
      this.loading = false;
      this.busyLabel = null;
      this.error = "Open or import a dataset before loading sections.";
      this.timeDepthDiagnostics = null;
      this.sectionScalarOverlays = [];
      this.sectionHorizons = [];
      this.sectionWellOverlays = [];
      this.note("Section load blocked because no active store is open.", "ui", "error");
      return;
    }

    try {
      const loadStartedMs = nowMs();
      const display = await this.loadResolvedSectionDisplay(activeStorePath, axis, index);
      const loadResolvedMs = nowMs();
      this.axis = axis;
      this.index = index;
      this.section = display.section;
      const viewerResetReason = this.#applyDisplayedSectionContext(
        activeStorePath,
        display.section,
        this.sectionDomain
      );
      this.timeDepthDiagnostics = display.time_depth_diagnostics;
      this.sectionScalarOverlays = adaptSectionScalarOverlays(
        display.scalar_overlays,
        this.velocityOverlayOpacity
      );
      this.sectionHorizons = adaptSectionHorizonOverlays(display.horizon_overlays);
      this.sectionWellOverlays = [];
      const stateAssignedMs = nowMs();
      this.loading = false;
      this.busyLabel = null;
      this.error = null;
      this.resetToken = this.#displayResetToken();
      await this.persistWorkspaceSession();
      const afterPersistMs = nowMs();
      await tick();
      const afterTickMs = nowMs();
      await nextAnimationFrame();
      const afterFirstFrameMs = nowMs();
      await nextAnimationFrame();
      const afterSecondFrameMs = nowMs();
      void emitFrontendDiagnosticsEvent({
        stage: "load_section",
        level: "info",
        message: "Frontend section load timings recorded",
        fields: {
          storePath: activeStorePath,
          datasetId: display.section.dataset_id,
          axis,
          index,
          traces: display.section.traces,
          samples: display.section.samples,
          payloadBytes: estimateSectionPayloadBytes(display.section),
          scalarOverlayCount: display.scalar_overlays.length,
          horizonOverlayCount: display.horizon_overlays.length,
          frontendAwaitMs: loadResolvedMs - loadStartedMs,
          frontendStateAssignMs: stateAssignedMs - loadResolvedMs,
          frontendPersistWorkspaceMs: afterPersistMs - stateAssignedMs,
          frontendTickMs: afterTickMs - afterPersistMs,
          frontendFirstFrameMs: afterFirstFrameMs - afterTickMs,
          frontendSecondFrameMs: afterSecondFrameMs - afterFirstFrameMs,
          frontendCommitToSecondFrameMs: afterSecondFrameMs - stateAssignedMs,
          frontendTotalMs: afterSecondFrameMs - loadStartedMs,
          viewerResetReason,
          viewerSessionKey: this.displayedViewerSessionKey,
          frontendStage: "viewer_load_section"
        }
      }).catch((error) => {
        this.note(
          "Failed to record frontend section load timings.",
          "backend",
          "warn",
          error instanceof Error ? error.message : String(error)
        );
      });
      this.note(
        "Section payload loaded.",
        "backend",
        "info",
        `${axis}:${index} | traces=${display.section.traces} samples=${display.section.samples} coordinate=${display.section.coordinate.value} | domain=${this.sectionDomain}`
      );
    } catch (error) {
      this.axis = axis;
      this.index = index;
      this.loading = false;
      this.busyLabel = null;
      this.timeDepthDiagnostics = null;
      this.error = error instanceof Error ? error.message : "Unknown section load error";
      this.note(
        "Section load failed.",
        "backend",
        "error",
        error instanceof Error ? error.message : "Unknown section load error"
      );
    }
  };
}

const [internalGetViewerModelContext, internalSetViewerModelContext] = createContext<ViewerModel>();

export function getViewerModelContext(): ViewerModel {
  const viewerModel = internalGetViewerModelContext();

  if (!viewerModel) {
    throw new Error("Viewer model context not found");
  }

  return viewerModel;
}

export function setViewerModelContext(viewerModel: ViewerModel): ViewerModel {
  internalSetViewerModelContext(viewerModel);
  return viewerModel;
}
