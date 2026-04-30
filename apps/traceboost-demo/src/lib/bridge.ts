import type {
  AmplitudeSpectrumRequest,
  AmplitudeSpectrumResponse,
  BuildSurveyTimeDepthTransformRequest,
  ExportSegyResponse,
  CancelProcessingBatchResponse,
  CancelProcessingJobResponse,
  DescribeVelocityVolumeRequest,
  DescribeVelocityVolumeResponse,
  DatasetRegistryEntry,
  DatasetRegistryStatus,
  GetProcessingDebugPlanResponse,
  GetProcessingJobResponse,
  GetProcessingBatchResponse,
  GetProcessingRuntimeStateResponse,
  IngestVelocityVolumeRequest,
  IngestVelocityVolumeResponse,
  ImportDatasetResponse,
  ImportHorizonXyzResponse,
  ImportSegyWithPlanResponse,
  ImportedHorizonDescriptor,
  LoadVelocityModelsResponse,
  LoadWorkspaceStateResponse,
  ListPipelinePresetsResponse,
  ListProcessingRuntimeEventsResponse,
  ListSegyImportRecipesResponse,
  OpenDatasetResponse,
  OperatorParameterDoc,
  PostStackNeighborhoodProcessingPipeline,
  ProcessingPipelineSpec,
  ProcessingPipelineFamily,
  PreviewSubvolumeProcessingRequest,
  PreviewSubvolumeProcessingResponse,
  PreviewPostStackNeighborhoodProcessingRequest,
  PreviewPostStackNeighborhoodProcessingResponse,
  PreviewTraceLocalProcessingResponse as PreviewProcessingResponse,
  ProcessingPreset,
  RemoveDatasetEntryResponse,
  RunPostStackNeighborhoodProcessingRequest,
  RunPostStackNeighborhoodProcessingResponse,
  RunSubvolumeProcessingRequest,
  RunSubvolumeProcessingResponse,
  RunTraceLocalProcessingResponse as RunProcessingResponse,
  TraceLocalProcessingOperation,
  TraceLocalProcessingPipeline,
  SubmitProcessingBatchRequest,
  SubmitProcessingBatchResponse,
  SubmitTraceLocalProcessingBatchRequest,
  SubmitTraceLocalProcessingBatchResponse,
  SaveWorkspaceSessionRequest,
  SaveWorkspaceSessionResponse,
  SavePipelinePresetResponse,
  SaveSegyImportRecipeResponse,
  SegyImportScanResponse,
  SectionAxis,
  SectionHorizonOverlayView,
  SegyGeometryOverride,
  SegyImportPlan,
  SegyImportRecipe,
  SegyImportValidationResponse,
  SetDatasetNativeCoordinateReferenceRequest,
  SetDatasetNativeCoordinateReferenceResponse,
  SetActiveDatasetEntryResponse,
  VelocityFunctionSource,
  VelocityQuantityKind,
  SurveyTimeDepthTransform3D,
  SectionView,
  SurveyPreflightResponse,
  PreviewTraceLocalProcessingRequest as PreviewProcessingRequest,
  ResolveSurveyMapRequest,
  ResolveSurveyMapResponse,
  RunTraceLocalProcessingRequest as RunProcessingRequest,
  SubvolumeCropOperation,
  SubvolumeProcessingPipeline,
  UpsertDatasetEntryRequest,
  UpsertDatasetEntryResponse,
  WorkspacePipelineEntry,
  WorkspaceSession
} from "@traceboost/seis-contracts";
import { IPC_SCHEMA_VERSION as SCHEMA_VERSION } from "@traceboost/seis-contracts";
export { SCHEMA_VERSION };
import type {
  CheckshotVspObservationSet1D,
  CoordinateReferenceDescriptor,
  ResolvedSurveyMapSourceDto,
  ManualTimeDepthPickSet1D,
  ResolveSectionWellOverlaysResponse,
  SurveyMapTransformStatusDto,
  SectionWellOverlayRequestDto,
  WellTieAnalysis1D,
  WellTieObservationSet1D,
  WellTimeDepthAuthoredModel1D,
  WellTimeDepthModel1D
} from "@ophiolite/contracts";
import {
  parsePackedPreviewProcessingResponse,
  parsePackedSectionDisplayResponse,
  parsePackedSectionTileResponse,
  parsePackedSectionViewResponse
} from "./transport/packed-sections";
import {
  desktopBridgeCommands,
  desktopCommandMetadata,
  type DesktopBridgeCommandName
} from "./generated/desktop-bridge-stubs";
import type {
  TransportPreviewProcessingResponse,
  TransportResolvedSectionDisplayView,
  TransportSectionTileView,
  TransportSectionView,
  TransportWindowedSectionView
} from "./transport/packed-sections";
export type {
  SectionBytePayload,
  TransportPreviewProcessingResponse,
  TransportPreviewView,
  TransportResolvedSectionDisplayView,
  TransportSectionScalarOverlayView,
  TransportSectionTileView,
  TransportSectionView,
  TransportWindowedSectionView
} from "./transport/packed-sections";

export interface DiagnosticsStatus {
  sessionId: string;
  sessionStartedAt: string;
  verboseEnabled: boolean;
  sessionLogPath: string;
}

export interface DiagnosticsEvent {
  sessionId: string;
  operationId: string;
  command: string;
  stage: string;
  level: string;
  timestamp: string;
  message: string;
  durationMs?: number | null;
  fields?: Record<string, unknown> | null;
}

interface GrantedPathSelection {
  path: string;
  handleId: string;
}

interface OutputPathGrantSelection {
  path: string;
  grantId: string;
}

export type OutputGrantPurpose =
  | "runtime_store_output"
  | "gather_store_output"
  | "segy_export"
  | "zarr_export";

export interface FrontendDiagnosticsEventRequest {
  stage: string;
  level: "debug" | "info" | "warn" | "error";
  message: string;
  fields?: Record<string, unknown> | null;
}

export interface ResolveProcessingAuthoringPaletteRequest {
  schema_version: number;
  store_path?: string | null;
  family: ProcessingPipelineFamily;
  secondary_store_paths?: string[];
}

export type ProcessingAuthoringInsertable =
  | {
      kind: "trace_local_operation";
      operation: TraceLocalProcessingOperation;
    }
  | {
      kind: "subvolume_crop";
      crop: SubvolumeCropOperation;
    };

export interface ProcessingAuthoringPaletteItem {
  item_id: string;
  label: string;
  description: string;
  short_help: string;
  help_markdown: string | null;
  help_url: string | null;
  keywords: string[];
  shortcut: string | null;
  canonical_id: string;
  canonical_name: string;
  group: string;
  group_id: string;
  provider: string;
  tags: string[];
  parameter_docs: readonly OperatorParameterDoc[];
  alias_label: string | null;
  source: string;
  insertable: ProcessingAuthoringInsertable;
}

export interface ResolveProcessingAuthoringPaletteResponse {
  schema_version: number;
  family: ProcessingPipelineFamily;
  items: ProcessingAuthoringPaletteItem[];
  source_label: string;
  source_detail: string;
  empty_message: string;
  fallback_reason?: string | null;
}

export interface SaveProcessingAuthoringSessionRequest {
  schema_version: number;
  entry_id: string;
  session_pipelines: WorkspacePipelineEntry[];
  active_session_pipeline_id?: string | null;
}

export type SaveProcessingAuthoringSessionResponse = ProcessingAuthoringSessionResponse;

export type ProcessingAuthoringSessionAction =
  | {
      action: "ensure_family_pipeline";
      family: ProcessingPipelineFamily;
    }
  | {
      action: "create_pipeline";
      family: ProcessingPipelineFamily;
    }
  | {
      action: "duplicate_pipeline";
      pipeline_id?: string | null;
    }
  | {
      action: "activate_pipeline";
      pipeline_id: string;
    }
  | {
      action: "remove_pipeline";
      pipeline_id: string;
    }
  | {
      action: "replace_active_from_pipeline_spec";
      pipeline: ProcessingPipelineSpec;
    };

export interface ApplyProcessingAuthoringSessionActionRequest {
  schema_version: number;
  entry_id: string;
  action: ProcessingAuthoringSessionAction;
}

export interface ProcessingAuthoringSessionResponse {
  schema_version: number;
  entry: DatasetRegistryEntry;
  session: WorkspaceSession;
}

export interface ResolveProcessingRunOutputRequest {
  schema_version: number;
  store_path: string;
  family: ProcessingPipelineFamily;
  pipeline?: TraceLocalProcessingPipeline | null;
  subvolume_crop?: SubvolumeCropOperation | null;
  post_stack_neighborhood_pipeline?: PostStackNeighborhoodProcessingPipeline | null;
}

export interface ResolveProcessingRunOutputResponse {
  schema_version: number;
  output_store_path: string;
}

export type ImportProviderId =
  | "seismic_volume"
  | "horizons"
  | "well_sources"
  | "velocity_functions"
  | "checkshot_vsp"
  | "manual_picks"
  | "authored_model"
  | "compiled_model"
  | "vendor_project";

export interface ImportProviderDescriptor {
  providerId: ImportProviderId;
  label: string;
  description: string;
  iconId: string;
  group: string;
  ordering: number;
  destinationKind: "runtime_store" | "project_asset" | string;
  selectionMode: "single_file" | "multi_file" | string;
  supportedExtensions: string[];
  supportsDirectory: boolean;
  requiresActiveStore: boolean;
  requiresProjectRoot: boolean;
  requiresProjectWellBinding: boolean;
  supportsDragDrop: boolean;
  supportsDeepLink: boolean;
  implemented: boolean;
}

export interface ImportSessionDiagnostic {
  level: string;
  message: string;
}

export interface ImportSessionEnvelope {
  sessionId: string;
  providerId: ImportProviderId;
  sourceRefs: string[];
  destinationKind: string;
  destinationRef?: string | null;
  activationIntent: string;
  status: string;
  diagnostics: ImportSessionDiagnostic[];
}

export interface BeginImportSessionRequest {
  providerId: ImportProviderId;
  sourceRefs?: string[] | null;
  destinationRef?: string | null;
  activationIntent?: string | null;
}

export interface ListImportProvidersResponse {
  providers: ImportProviderDescriptor[];
}

export interface RunSectionBrowsingBenchmarkRequest {
  storePath: string;
  axis: SectionAxis;
  sectionIndex: number;
  traceRange: [number, number];
  sampleRange: [number, number];
  lod: number;
  iterations?: number;
  includeFullSectionBaseline?: boolean;
  stepOffsets?: number[];
  switchAxis?: SectionAxis;
  switchSectionIndex?: number;
}

export interface SectionBrowsingBenchmarkCase {
  scenario: string;
  axis: string;
  index: number;
  traceRange: [number, number];
  sampleRange: [number, number];
  lod: number;
  traceStep: number;
  sampleStep: number;
  outputTraces: number;
  outputSamples: number;
  payloadBytes: number;
  iterationMs: number[];
  medianMs: number;
  meanMs: number;
}

export interface RunSectionBrowsingBenchmarkResponse {
  sessionLogPath: string;
  storePath: string;
  datasetId: string;
  shape: [number, number, number];
  tileShape: [number, number, number];
  axis: string;
  sectionIndex: number;
  traceRange: [number, number];
  sampleRange: [number, number];
  lod: number;
  iterations: number;
  includeFullSectionBaseline: boolean;
  stepOffsets: number[];
  switchAxis?: string | null;
  switchSectionIndex?: number | null;
  cases: SectionBrowsingBenchmarkCase[];
}

export interface HorizonImportCoordinateReferenceOptions {
  sourceCoordinateReferenceId?: string | null;
  sourceCoordinateReferenceName?: string | null;
  assumeSameAsSurvey?: boolean;
  verticalDomain?: ImportedHorizonDescriptor["vertical_domain"] | null;
  verticalUnit?: string | null;
}

export interface HorizonImportPreviewFile {
  source_path: string;
  name: string;
  parsed_point_count: number;
  invalid_row_count: number;
  x_min: number | null;
  x_max: number | null;
  y_min: number | null;
  y_max: number | null;
  z_min: number | null;
  z_max: number | null;
  estimated_mapped_point_count: number | null;
  estimated_missing_cell_count: number | null;
  can_commit: boolean;
  issues: string[];
}

export interface HorizonXyzFilePreview {
  source_path: string;
  name: string;
  parsed_point_count: number;
  invalid_row_count: number;
  x_min: number | null;
  x_max: number | null;
  y_min: number | null;
  y_max: number | null;
  z_min: number | null;
  z_max: number | null;
  issues: string[];
}

export interface HorizonImportPreview {
  files: HorizonImportPreviewFile[];
  source_coordinate_reference: CoordinateReferenceDescriptor | null;
  aligned_coordinate_reference: CoordinateReferenceDescriptor | null;
  transformed: boolean;
  can_commit: boolean;
  notes: string[];
  issues: string[];
}

export interface HorizonSourceImportCanonicalDraft {
  selectedSourcePaths: string[];
  verticalDomain: ImportedHorizonDescriptor["vertical_domain"];
  verticalUnit?: string | null;
  sourceCoordinateReference?: CoordinateReferenceDescriptor | null;
  assumeSameAsSurvey: boolean;
}

export interface HorizonSourceImportPreview {
  parsed: HorizonImportPreview;
  suggestedDraft: HorizonSourceImportCanonicalDraft;
}

export interface PreviewHorizonSourceImportRequest {
  storePath: string;
  inputPaths: string[];
  draft?: HorizonSourceImportCanonicalDraft | null;
}

export interface CommitHorizonSourceImportRequest {
  storePath: string;
  draft: HorizonSourceImportCanonicalDraft;
}

export interface CoordinateReferenceCatalogEntry {
  authority: string;
  code: string;
  authId: string;
  name: string;
  deprecated: boolean;
  areaName?: string | null;
  coordinateReferenceType: string;
}

export type CoordinateReferenceSelection =
  | {
      kind: "authority_code";
      authority: string;
      code: string;
      authId: string;
      name?: string | null;
    }
  | {
      kind: "local_engineering";
      label: string;
    }
  | {
      kind: "unresolved";
    };

export type ProjectDisplayCoordinateReference =
  | {
      kind: "native_engineering";
    }
  | {
      kind: "authority_code";
      authority: string;
      code: string;
      authId: string;
      name?: string | null;
    };

export interface ProjectGeospatialSettings {
  schemaVersion: number;
  displayCoordinateReference: ProjectDisplayCoordinateReference;
  source: string;
  createdAtUnixS: number;
  updatedAtUnixS: number;
}

export interface ExportZarrResponse {
  store_path: string;
  output_path: string;
}

export interface ImportVelocityFunctionsModelResponse {
  schema_version: number;
  input_path: string;
  velocity_kind: VelocityQuantityKind;
  profile_count: number;
  sample_count: number;
  model: SurveyTimeDepthTransform3D;
}

export interface DatasetExportFormatCapability {
  available: boolean;
  reason: string | null;
  defaultOutputPath: string;
}

export interface DatasetExportCapabilitiesResponse {
  storePath: string;
  segy: DatasetExportFormatCapability;
  zarr: DatasetExportFormatCapability;
}

export interface ProjectAssetBindingInput {
  well_name: string;
  wellbore_name: string;
  uwi?: string | null;
  api?: string | null;
  operator_aliases: string[];
}

export interface ProjectOperatorAssignment {
  organisation_name?: string | null;
  organisation_id?: string | null;
  effective_at?: string | null;
  terminated_at?: string | null;
  source?: string | null;
  note?: string | null;
}

export interface ProjectExternalReference {
  system: string;
  id: string;
  kind?: string | null;
  note?: string | null;
}

export interface ProjectProjectedPoint2 {
  x: number;
  y: number;
}

export interface ProjectLocatedPoint {
  coordinate_reference?: CoordinateReferenceDescriptor | null;
  point: ProjectProjectedPoint2;
  recorded_at?: string | null;
  source?: string | null;
  note?: string | null;
}

export interface ProjectVerticalMeasurement {
  measurement_id?: string | null;
  value: number;
  unit?: string | null;
  path: string;
  coordinate_reference?: CoordinateReferenceDescriptor | null;
  reference_measurement_id?: string | null;
  reference_entity_id?: string | null;
  source?: string | null;
  description?: string | null;
}

export interface ProjectWellMetadata {
  field_name?: string | null;
  block_name?: string | null;
  basin_name?: string | null;
  country?: string | null;
  province_state?: string | null;
  location_text?: string | null;
  interest_type?: string | null;
  operator_history?: ProjectOperatorAssignment[];
  surface_location?: ProjectLocatedPoint | null;
  default_vertical_measurement_id?: string | null;
  default_vertical_coordinate_reference?: CoordinateReferenceDescriptor | null;
  vertical_measurements?: ProjectVerticalMeasurement[];
  external_references?: ProjectExternalReference[];
  notes?: string[];
}

export interface ProjectWellboreMetadata {
  sequence_number?: number | null;
  status?: string | null;
  purpose?: string | null;
  trajectory_type?: string | null;
  parent_wellbore_id?: string | null;
  target_formation?: string | null;
  primary_material?: string | null;
  location_text?: string | null;
  service_company_name?: string | null;
  operator_history?: ProjectOperatorAssignment[];
  bottom_hole_location?: ProjectLocatedPoint | null;
  default_vertical_measurement_id?: string | null;
  default_vertical_coordinate_reference?: CoordinateReferenceDescriptor | null;
  vertical_measurements?: ProjectVerticalMeasurement[];
  external_references?: ProjectExternalReference[];
  notes?: string[];
}

export interface ProjectWellFolderImportIssue {
  severity: "info" | "warning" | "blocking";
  code: string;
  message: string;
  slice?: string | null;
  sourcePath?: string | null;
}

export type ProjectWellFolderImportOmissionKind =
  | "surface_location"
  | "trajectory"
  | "tops_rows"
  | "log"
  | "ascii_log"
  | "unsupported_sources";

export type ProjectWellFolderImportOmissionReasonCode =
  | "source_crs_unresolved"
  | "trajectory_not_committed"
  | "tops_rows_incomplete"
  | "log_unselected"
  | "ascii_log_unselected"
  | "unsupported_preserved_as_source"
  | "unsupported_preserved_as_raw_bundle";

export interface ProjectWellFolderImportOmission {
  kind: ProjectWellFolderImportOmissionKind;
  slice: string;
  reasonCode: ProjectWellFolderImportOmissionReasonCode;
  message: string;
  sourcePath?: string | null;
  rowCount?: number | null;
}

export interface ProjectWellFolderImportBindingDraft {
  wellName: string;
  wellboreName: string;
  uwi?: string | null;
  api?: string | null;
  operatorAliases: string[];
}

export interface ProjectWellFolderDetectedSource {
  sourcePath: string;
  fileName: string;
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  reason: string;
}

export interface ProjectWellFolderMetadataSlicePreview {
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  commitEnabled: boolean;
  sourcePath?: string | null;
  wellMetadata?: ProjectWellMetadata | null;
  wellboreMetadata?: ProjectWellboreMetadata | null;
  detectedCoordinateReferences: CoordinateReferenceDescriptor[];
  notes: string[];
}

export interface ProjectWellFolderLogFilePreview {
  sourcePath: string;
  fileName: string;
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  rowCount: number;
  curveCount: number;
  indexCurveName: string;
  curveNames: string[];
  detectedWellName?: string | null;
  issueCount: number;
  defaultSelected: boolean;
  selectionReason?: string | null;
  duplicateGroupId?: string | null;
}

export interface ProjectWellFolderLogsSlicePreview {
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  commitEnabled: boolean;
  files: ProjectWellFolderLogFilePreview[];
}

export interface ProjectWellFolderAsciiLogColumnPreview {
  name: string;
  numericCount: number;
  nullCount: number;
  sampleValues: number[];
}

export interface ProjectWellFolderAsciiLogFilePreview {
  sourcePath: string;
  fileName: string;
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  rowCount: number;
  columnCount: number;
  defaultDepthColumn?: string | null;
  defaultValueColumns: string[];
  columns: ProjectWellFolderAsciiLogColumnPreview[];
  issueCount: number;
}

export interface ProjectWellFolderAsciiLogsSlicePreview {
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  commitEnabled: boolean;
  files: ProjectWellFolderAsciiLogFilePreview[];
}

export interface ProjectWellFolderTopDraftRow {
  name?: string | null;
  topDepth?: number | null;
  baseDepth?: number | null;
  anomaly?: string | null;
  quality?: string | null;
  note?: string | null;
}

export interface ProjectWellFolderTopsSlicePreview {
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  commitEnabled: boolean;
  sourcePath?: string | null;
  rowCount: number;
  committableRowCount: number;
  preferredDepthReference?: string | null;
  sourceName?: string | null;
  rows: ProjectWellFolderTopDraftRow[];
}

export interface ProjectWellFolderTrajectoryDraftRow {
  measuredDepth?: number | null;
  inclinationDeg?: number | null;
  azimuthDeg?: number | null;
  trueVerticalDepth?: number | null;
  xOffset?: number | null;
  yOffset?: number | null;
}

export interface ProjectWellFolderTrajectorySlicePreview {
  status:
    | "not_present"
    | "parsed"
    | "parsed_with_issues"
    | "unsupported"
    | "not_viable_for_commit"
    | "ready_for_commit";
  commitEnabled: boolean;
  sourcePath?: string | null;
  rowCount: number;
  committableRowCount: number;
  nonEmptyColumnCount: Record<string, number>;
  draftRows: ProjectWellFolderTrajectoryDraftRow[];
  sampleRows: ProjectWellFolderTrajectoryDraftRow[];
}

export type WellFolderCoordinateReferenceCandidateConfidence = "high" | "medium" | "low";

export type WellFolderCoordinateReferenceSelectionMode =
  | "detected"
  | "assume_same_as_survey"
  | "manual"
  | "unresolved";

export interface WellFolderCoordinateReferenceCandidate {
  coordinateReference: CoordinateReferenceDescriptor;
  confidence: WellFolderCoordinateReferenceCandidateConfidence;
  evidence: string;
  rationale: string;
  supportsGeometryCommit: boolean;
}

export interface WellFolderCoordinateReferencePreview {
  requiredForSurfaceLocation: boolean;
  requiredForTrajectory: boolean;
  recommendedCandidateId?: string | null;
  candidates: WellFolderCoordinateReferenceCandidate[];
  notes: string[];
}

export interface WellFolderCoordinateReferenceSelection {
  mode: WellFolderCoordinateReferenceSelectionMode;
  candidateId?: string | null;
  coordinateReference?: CoordinateReferenceDescriptor | null;
}

export interface ProjectWellFolderImportPreview {
  schemaVersion: number;
  folderPath: string;
  folderName: string;
  binding: ProjectWellFolderImportBindingDraft;
  sourceCoordinateReference: WellFolderCoordinateReferencePreview;
  metadata: ProjectWellFolderMetadataSlicePreview;
  logs: ProjectWellFolderLogsSlicePreview;
  asciiLogs: ProjectWellFolderAsciiLogsSlicePreview;
  topsMarkers: ProjectWellFolderTopsSlicePreview;
  trajectory: ProjectWellFolderTrajectorySlicePreview;
  unsupportedSources: ProjectWellFolderDetectedSource[];
  issues: ProjectWellFolderImportIssue[];
}

export interface ProjectWellFolderImportedAsset {
  assetKind: string;
  sourcePath: string;
  assetId: string;
  collectionId: string;
  collectionName: string;
}

export interface CommitProjectWellImportRequest {
  projectRoot: string;
  folderPath: string;
  sourcePaths?: string[] | null;
  draft?: ProjectWellSourceImportCanonicalDraft | null;
  binding: ProjectAssetBindingInput;
  wellMetadata?: ProjectWellMetadata | null;
  wellboreMetadata?: ProjectWellboreMetadata | null;
  sourceCoordinateReference: WellFolderCoordinateReferenceSelection;
  importLogs: boolean;
  selectedLogSourcePaths?: string[] | null;
  importTopsMarkers: boolean;
  importTrajectory: boolean;
  topsDepthReference?: string | null;
  topsRows?: ProjectWellFolderTopDraftRow[] | null;
  trajectoryRows?: ProjectWellFolderTrajectoryDraftRow[] | null;
  asciiLogImports?: ProjectWellFolderAsciiLogImportRequest[] | null;
}

export interface PreviewProjectWellImportRequest {
  folderPath: string;
  sourcePaths?: string[] | null;
}

export interface ProjectWellFolderAsciiLogCurveMapping {
  sourceColumn: string;
  mnemonic: string;
  unit?: string | null;
}

export interface ProjectWellFolderAsciiLogImportRequest {
  sourcePath: string;
  depthColumn: string;
  valueColumns: ProjectWellFolderAsciiLogCurveMapping[];
  nullValue?: number | null;
}

export interface ProjectWellFolderImportCommitResponse {
  schemaVersion: number;
  wellId: string;
  wellboreId: string;
  createdWell: boolean;
  createdWellbore: boolean;
  sourceCoordinateReferenceMode: WellFolderCoordinateReferenceSelectionMode;
  sourceCoordinateReference?: CoordinateReferenceDescriptor | null;
  importedAssets: ProjectWellFolderImportedAsset[];
  omissions: ProjectWellFolderImportOmission[];
  issues: ProjectWellFolderImportIssue[];
}

export type ProjectWellSourceImportIssue = ProjectWellFolderImportIssue;
export type ProjectWellSourceImportOmissionKind = ProjectWellFolderImportOmissionKind;
export type ProjectWellSourceImportOmissionReasonCode = ProjectWellFolderImportOmissionReasonCode;
export type ProjectWellSourceImportOmission = ProjectWellFolderImportOmission;
export type ProjectWellSourceImportBindingDraft = ProjectWellFolderImportBindingDraft;
export type ProjectWellSourceDetectedSource = ProjectWellFolderDetectedSource;
export type ProjectWellSourceMetadataSlicePreview = ProjectWellFolderMetadataSlicePreview;
export type ProjectWellSourceLogFilePreview = ProjectWellFolderLogFilePreview;
export type ProjectWellSourceLogsSlicePreview = ProjectWellFolderLogsSlicePreview;
export type ProjectWellSourceAsciiLogColumnPreview = ProjectWellFolderAsciiLogColumnPreview;
export type ProjectWellSourceAsciiLogFilePreview = ProjectWellFolderAsciiLogFilePreview;
export type ProjectWellSourceAsciiLogsSlicePreview = ProjectWellFolderAsciiLogsSlicePreview;
export type ProjectWellSourceTopDraftRow = ProjectWellFolderTopDraftRow;
export type ProjectWellSourceTopsSlicePreview = ProjectWellFolderTopsSlicePreview;
export type ProjectWellSourceTrajectoryDraftRow = ProjectWellFolderTrajectoryDraftRow;
export type ProjectWellSourceTrajectorySlicePreview = ProjectWellFolderTrajectorySlicePreview;
export type WellSourceCoordinateReferenceCandidateConfidence =
  WellFolderCoordinateReferenceCandidateConfidence;
export type WellSourceCoordinateReferenceSelectionMode =
  WellFolderCoordinateReferenceSelectionMode;
export type WellSourceCoordinateReferenceCandidate = WellFolderCoordinateReferenceCandidate;
export type WellSourceCoordinateReferencePreview = WellFolderCoordinateReferencePreview;
export type WellSourceCoordinateReferenceSelection = WellFolderCoordinateReferenceSelection;
export type ProjectWellSourceImportedAsset = ProjectWellFolderImportedAsset;
export type ProjectWellSourceAsciiLogCurveMapping = ProjectWellFolderAsciiLogCurveMapping;
export type ProjectWellSourceAsciiLogImportRequest = ProjectWellFolderAsciiLogImportRequest;
export type ProjectWellSourceImportCommitResponse = ProjectWellFolderImportCommitResponse;

export interface ProjectWellSourceImportTopsCanonicalDraft {
  depthReference?: string | null;
  rows: ProjectWellSourceTopDraftRow[];
}

export interface ProjectWellSourceImportTrajectoryCanonicalDraft {
  enabled: boolean;
  rows?: ProjectWellSourceTrajectoryDraftRow[] | null;
}

export interface ProjectWellSourceImportPlanCanonicalDraft {
  selectedLogSourcePaths?: string[] | null;
  asciiLogImports?: ProjectWellSourceAsciiLogImportRequest[] | null;
  topsMarkers?: ProjectWellSourceImportTopsCanonicalDraft | null;
  trajectory?: ProjectWellSourceImportTrajectoryCanonicalDraft | null;
}

export interface ProjectWellSourceImportCanonicalDraft {
  binding: ProjectAssetBindingInput;
  sourceCoordinateReference: WellSourceCoordinateReferenceSelection;
  wellMetadata?: ProjectWellMetadata | null;
  wellboreMetadata?: ProjectWellboreMetadata | null;
  importPlan: ProjectWellSourceImportPlanCanonicalDraft;
}

export interface ProjectWellSourceImportPreview {
  parsed: ProjectWellFolderImportPreview;
  suggestedDraft: ProjectWellSourceImportCanonicalDraft;
}

export interface PreviewProjectWellSourceImportRequest {
  sourceRootPath: string;
  sourcePaths?: string[] | null;
}

export interface CommitProjectWellSourceImportRequest {
  projectRoot: string;
  sourceRootPath: string;
  sourcePaths?: string[] | null;
  draft?: ProjectWellSourceImportCanonicalDraft | null;
  binding?: ProjectAssetBindingInput;
  wellMetadata?: ProjectWellMetadata | null;
  wellboreMetadata?: ProjectWellboreMetadata | null;
  sourceCoordinateReference?: WellSourceCoordinateReferenceSelection;
  importLogs?: boolean;
  selectedLogSourcePaths?: string[] | null;
  importTopsMarkers?: boolean;
  importTrajectory?: boolean;
  topsDepthReference?: string | null;
  topsRows?: ProjectWellSourceTopDraftRow[] | null;
  trajectoryRows?: ProjectWellSourceTrajectoryDraftRow[] | null;
  asciiLogImports?: ProjectWellSourceAsciiLogImportRequest[] | null;
}

export interface ProjectWellTimeDepthModelDescriptor {
  assetId: string;
  wellId: string;
  wellboreId: string;
  status: string;
  name: string;
  sourceKind: WellTimeDepthModel1D["source_kind"];
  depthReference: WellTimeDepthModel1D["depth_reference"];
  travelTimeReference: WellTimeDepthModel1D["travel_time_reference"];
  sampleCount: number;
  isActiveProjectModel: boolean;
}

export interface ProjectWellTimeDepthObservationDescriptor {
  assetId: string;
  assetKind:
    | "checkshot_vsp_observation_set"
    | "manual_time_depth_pick_set"
    | "well_tie_observation_set";
  wellId: string;
  wellboreId: string;
  status: string;
  name: string;
  depthReference:
    | CheckshotVspObservationSet1D["depth_reference"]
    | ManualTimeDepthPickSet1D["depth_reference"]
    | WellTieObservationSet1D["depth_reference"];
  travelTimeReference:
    | CheckshotVspObservationSet1D["travel_time_reference"]
    | ManualTimeDepthPickSet1D["travel_time_reference"]
    | WellTieObservationSet1D["travel_time_reference"];
  sampleCount: number;
  sourceWellTimeDepthModelAssetId?: string | null;
  tieWindowStartMs?: number | null;
  tieWindowEndMs?: number | null;
  traceSearchRadiusM?: number | null;
  bulkShiftMs?: number | null;
  stretchFactor?: number | null;
  traceSearchOffsetM?: number | null;
  correlation?: number | null;
}

export interface ProjectWellTimeDepthAuthoredModelDescriptor {
  assetId: string;
  wellId: string;
  wellboreId: string;
  status: string;
  name: string;
  sourceBindingCount: number;
  assumptionIntervalCount: number;
  samplingStepM?: number | null;
  resolvedTrajectoryFingerprint: WellTimeDepthAuthoredModel1D["resolved_trajectory_fingerprint"];
}

export interface ProjectWellTimeDepthInventoryResponse {
  observationSets: ProjectWellTimeDepthObservationDescriptor[];
  authoredModels: ProjectWellTimeDepthAuthoredModelDescriptor[];
  compiledModels: ProjectWellTimeDepthModelDescriptor[];
}

export interface ProjectSurveyAssetDescriptor {
  assetId: string;
  name: string;
  status: string;
  wellId: string;
  wellName: string;
  wellboreId: string;
  wellboreName: string;
  effectiveCoordinateReferenceId?: string | null;
  effectiveCoordinateReferenceName?: string | null;
  displayCompatibility: ProjectSurveyDisplayCompatibility;
}

export interface ProjectWellboreInventoryItem {
  wellId: string;
  wellName: string;
  wellboreId: string;
  wellboreName: string;
  trajectoryAssetCount: number;
  wellTimeDepthModelCount: number;
  activeWellTimeDepthModelAssetId?: string | null;
  displayCompatibility: ProjectWellboreDisplayCompatibility;
}

export interface ProjectSurveyDisplayCompatibility {
  canResolveProjectMap: boolean;
  transformStatus: SurveyMapTransformStatusDto;
  sourceCoordinateReferenceId?: string | null;
  displayCoordinateReferenceId?: string | null;
  reasonCode?: ProjectSurveyDisplayReasonCode | null;
  reason?: string | null;
}

export type ProjectSurveyDisplayReasonCode =
  | "project_display_crs_unresolved"
  | "display_crs_unsupported"
  | "source_crs_unknown"
  | "source_crs_unsupported"
  | "display_equivalent"
  | "display_transformed";

export interface ProjectMapDisplayCompatibilitySummary {
  displayCoordinateReferenceId?: string | null;
  compatibleSurveyCount: number;
  incompatibleSurveyCount: number;
  compatibleWellboreCount: number;
  incompatibleWellboreCount: number;
  blockingReasonCodes?: ProjectDisplayCompatibilityBlockingReasonCode[] | null;
  blockingReasons?: string[] | null;
}

export interface ProjectWellboreDisplayCompatibility {
  canResolveProjectMap: boolean;
  transformStatus: SurveyMapTransformStatusDto;
  sourceCoordinateReferenceId?: string | null;
  displayCoordinateReferenceId?: string | null;
  reasonCode?: ProjectWellboreDisplayReasonCode | null;
  reason?: string | null;
}

export type ProjectWellboreDisplayReasonCode =
  | "project_display_crs_unresolved"
  | "resolved_geometry_missing"
  | "display_equivalent"
  | "display_transformed"
  | "display_degraded"
  | "display_unavailable"
  | "resolution_error";

export type ProjectDisplayCompatibilityBlockingReasonCode =
  | "project_display_crs_unresolved"
  | "display_crs_unsupported"
  | "source_crs_unknown"
  | "source_crs_unsupported"
  | "resolved_geometry_missing"
  | "display_unavailable"
  | "resolution_error";

export interface ProjectWellOverlayInventoryResponse {
  surveys: ProjectSurveyAssetDescriptor[];
  wellbores: ProjectWellboreInventoryItem[];
  displayCompatibility: ProjectMapDisplayCompatibilitySummary;
}

export interface ProjectWellMarkerDescriptor {
  name: string;
  markerKind?: string | null;
  sourceAssetId?: string | null;
  topDepth: number;
  baseDepth?: number | null;
  depthReference?: string | null;
  source?: string | null;
  note?: string | null;
}

export interface ProjectWellMarkerHorizonResidualPointDescriptor {
  markerName: string;
  markerKind?: string | null;
  x: number;
  y: number;
  z: number;
  horizonDepth: number;
  residual: number;
  status: string;
  note?: string | null;
}

export interface ProjectWellMarkerHorizonResidualDescriptor {
  assetId: string;
  sourceAssetId?: string | null;
  surveyAssetId?: string | null;
  horizonId?: string | null;
  markerName?: string | null;
  wellId: string;
  wellboreId: string;
  status: string;
  name: string;
  rowCount: number;
  pointCount: number;
  markerNames: string[];
  points: ProjectWellMarkerHorizonResidualPointDescriptor[];
}

export interface ProjectWellMarkerResidualInventoryResponse {
  markers: ProjectWellMarkerDescriptor[];
  residualAssets: ProjectWellMarkerHorizonResidualDescriptor[];
}

export type VendorProjectImportVendor = "opendtect" | "petrel";

export type VendorProjectCanonicalTargetKind =
  | "seismic_trace_data"
  | "survey_store_horizon"
  | "log"
  | "trajectory"
  | "top_set"
  | "well_marker_set"
  | "well_time_depth_model"
  | "checkshot_vsp_observation_set"
  | "raw_source_bundle"
  | "external_open_format"
  | "none";

export type VendorProjectImportDisposition = "canonical" | "canonical_with_loss" | "raw_source_only";

export type VendorProjectImportIssueSeverity = "info" | "warning" | "blocking";

export interface VendorProjectImportIssue {
  severity: VendorProjectImportIssueSeverity;
  code: string;
  message: string;
  sourcePath?: string | null;
  vendorObjectId?: string | null;
}

export interface VendorProjectCoordinateReferenceDescriptor {
  id?: string | null;
  name?: string | null;
  geodetic_datum?: string | null;
  unit?: string | null;
}

export interface VendorProjectSurveyMetadata {
  name?: string | null;
  surveyDataType?: string | null;
  inlineRange?: [number, number, number] | null;
  crosslineRange?: [number, number, number] | null;
  zRange?: [number, number, number] | null;
  zDomain?: string | null;
  coordinateReference?: VendorProjectCoordinateReferenceDescriptor | null;
  coordinateReferenceSourcePath?: string | null;
  notes: string[];
}

export interface VendorProjectObjectPreview {
  vendorObjectId: string;
  vendorKind: string;
  displayName: string;
  sourcePaths: string[];
  canonicalTargetKind: VendorProjectCanonicalTargetKind;
  disposition: VendorProjectImportDisposition;
  requiresCrsDecision: boolean;
  defaultSelected: boolean;
  notes: string[];
}

export interface VendorProjectScanRequest {
  vendor: VendorProjectImportVendor;
  projectRoot: string;
}

export interface VendorProjectScanResponse {
  schemaVersion: number;
  vendor: VendorProjectImportVendor;
  projectRoot: string;
  vendorProject?: string | null;
  surveyMetadata: VendorProjectSurveyMetadata;
  objects: VendorProjectObjectPreview[];
  issues: VendorProjectImportIssue[];
}

export interface VendorProjectPlanSurveyCandidate {
  asset_id: string;
  logical_asset_id: string;
  collection_id: string;
  name: string;
  status: string;
  owner_scope: string;
  owner_id: string;
  owner_name: string;
  well_id: string;
  well_name: string;
  wellbore_id: string;
  wellbore_name: string;
  effective_coordinate_reference_id?: string | null;
  effective_coordinate_reference_name?: string | null;
}

export interface VendorProjectPlannedImport {
  vendorObjectId: string;
  displayName: string;
  canonicalTargetKind: VendorProjectCanonicalTargetKind;
  disposition: VendorProjectImportDisposition;
  requiresTargetSurveyAsset: boolean;
  sourcePaths: string[];
  notes: string[];
}

export interface VendorProjectPlanRequest {
  vendor: VendorProjectImportVendor;
  projectRoot: string;
  selectedVendorObjectIds: string[];
  targetProjectRoot?: string | null;
  targetSurveyAssetId?: string | null;
  binding?: ProjectAssetBindingInput | null;
  coordinateReference?: VendorProjectCoordinateReferenceDescriptor | null;
}

export interface VendorProjectPlanResponse {
  schemaVersion: number;
  vendor: VendorProjectImportVendor;
  projectRoot: string;
  plannedImports: VendorProjectPlannedImport[];
  bridgeRequests: Array<Record<string, unknown>>;
  targetSurveyAssetRequired: boolean;
  targetSurveyAssetCandidates: VendorProjectPlanSurveyCandidate[];
  selectedTargetSurveyAsset?: VendorProjectPlanSurveyCandidate | null;
  runtimeProbe?: Record<string, unknown> | null;
  blockingIssues: VendorProjectImportIssue[];
  warnings: VendorProjectImportIssue[];
}

export interface VendorProjectCommittedAsset {
  vendorObjectId: string;
  displayName: string;
  canonicalTargetKind: VendorProjectCanonicalTargetKind;
  disposition: VendorProjectImportDisposition;
  assetId?: string | null;
  collectionId?: string | null;
  collectionName?: string | null;
  sourcePaths: string[];
  notes: string[];
}

export interface VendorProjectValidationReport {
  vendorObjectId: string;
  displayName: string;
  checks: string[];
  notes: string[];
}

export interface VendorProjectCommitRequest {
  plan: VendorProjectPlanResponse;
  targetProjectRoot?: string | null;
  binding?: ProjectAssetBindingInput | null;
  targetSurveyAssetId?: string | null;
  coordinateReference?: VendorProjectCoordinateReferenceDescriptor | null;
  bridgeOutputs?: Array<Record<string, unknown>>;
  dryRun: boolean;
}

export interface VendorProjectCommitResponse {
  schemaVersion: number;
  vendor: VendorProjectImportVendor;
  projectRoot: string;
  targetProjectRoot?: string | null;
  importedAssets: VendorProjectCommittedAsset[];
  preservedRawSources: VendorProjectCommittedAsset[];
  validationReports: VendorProjectValidationReport[];
  issues: VendorProjectImportIssue[];
}

export interface ResolveProjectSurveyMapRequest {
  projectRoot: string;
  surveyAssetId: string;
  wellboreId?: string | null;
  displayCoordinateReferenceId: string;
}

export interface ResolveProjectSurveyMapResponse {
  surveyMap: ResolvedSurveyMapSourceDto;
}

export interface ComputeProjectWellMarkerResidualRequest {
  projectRoot: string;
  wellboreId: string;
  surveyAssetId: string;
  horizonId: string;
  markerName: string;
  outputCollectionName?: string | null;
}

export interface ComputeProjectWellMarkerResidualResponse {
  assetId: string;
  collectionId: string;
  collectionName: string;
  wellId: string;
  wellboreId: string;
  markerName: string;
  horizonId: string;
  pointCount: number;
}

export interface ImportProjectWellTimeDepthModelRequest {
  projectRoot: string;
  jsonPath: string;
  binding: ProjectAssetBindingInput;
  collectionName?: string | null;
}

export interface ImportProjectWellTimeDepthModelResponse {
  assetId: string;
  wellId: string;
  wellboreId: string;
  createdWell: boolean;
  createdWellbore: boolean;
}

export interface ImportProjectWellTimeDepthAssetRequest {
  projectRoot: string;
  jsonPath: string;
  jsonPayload?: string | null;
  binding: ProjectAssetBindingInput;
  collectionName?: string | null;
  assetKind:
    | "checkshot_vsp_observation_set"
    | "manual_time_depth_pick_set"
    | "well_tie_observation_set"
    | "well_time_depth_authored_model"
    | "well_time_depth_model";
}

export interface ProjectWellTimeDepthPreviewIssue {
  severity: "info" | "warning" | "blocking";
  code: string;
  message: string;
}

export interface PreviewProjectWellTimeDepthAssetRequest {
  jsonPath: string;
  jsonPayload?: string | null;
  assetKind:
    | "checkshot_vsp_observation_set"
    | "manual_time_depth_pick_set"
    | "well_tie_observation_set"
    | "well_time_depth_authored_model"
    | "well_time_depth_model";
}

export interface ProjectWellTimeDepthAssetPreview {
  assetKind: PreviewProjectWellTimeDepthAssetRequest["assetKind"];
  jsonPath: string;
  canImport: boolean;
  id?: string | null;
  name?: string | null;
  wellboreId?: string | null;
  depthReference?: string | null;
  travelTimeReference?: string | null;
  sampleCount?: number | null;
  noteCount?: number | null;
  sourceKind?: string | null;
  sourceBindingCount?: number | null;
  assumptionIntervalCount?: number | null;
  samplingStepM?: number | null;
  resolvedTrajectoryFingerprint?: string | null;
  sourceWellTimeDepthModelAssetId?: string | null;
  tieWindowStartMs?: number | null;
  tieWindowEndMs?: number | null;
  traceSearchRadiusM?: number | null;
  bulkShiftMs?: number | null;
  stretchFactor?: number | null;
  traceSearchOffsetM?: number | null;
  correlation?: number | null;
  issues: ProjectWellTimeDepthPreviewIssue[];
  rawJson: string;
}

export interface ProjectWellTimeDepthImportCanonicalDraft {
  assetKind: PreviewProjectWellTimeDepthAssetRequest["assetKind"];
  jsonPayload: string;
  collectionName?: string | null;
}

export interface ProjectWellTimeDepthImportPreview {
  parsed: ProjectWellTimeDepthAssetPreview;
  suggestedDraft: ProjectWellTimeDepthImportCanonicalDraft;
}

export interface PreviewProjectWellTimeDepthImportRequest {
  jsonPath: string;
  draft?: ProjectWellTimeDepthImportCanonicalDraft | null;
  assetKind: PreviewProjectWellTimeDepthAssetRequest["assetKind"];
}

export interface CommitProjectWellTimeDepthImportRequest {
  projectRoot: string;
  jsonPath: string;
  binding: ProjectAssetBindingInput;
  draft: ProjectWellTimeDepthImportCanonicalDraft;
}

export interface CompileProjectWellTimeDepthAuthoredModelRequest {
  projectRoot: string;
  assetId: string;
  outputCollectionName?: string | null;
  setActive: boolean;
}

export interface AnalyzeProjectWellTieRequest {
  projectRoot: string;
  sourceModelAssetId: string;
  tieName: string;
  tieStartMs: number;
  tieEndMs: number;
  searchRadiusM: number;
  storePath: string;
  surveyAssetId: string;
  displayCoordinateReferenceId: string;
}

export interface ProjectWellTieAnalysisResponse {
  draftObservationSet: WellTieObservationSet1D;
  analysis: WellTieAnalysis1D;
  sourceModelAssetId: string;
  sourceModelName: string;
}

export interface AcceptProjectWellTieRequest extends AnalyzeProjectWellTieRequest {
  binding: ProjectAssetBindingInput;
  outputCollectionName?: string | null;
  setActive: boolean;
}

export interface AcceptProjectWellTieResponse {
  observationAssetId: string;
  authoredModelAssetId: string;
  compiledModelAssetId: string;
}

export function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const DATASET_REGISTRY_STORAGE_KEY = "traceboost.dataset-registry";
const WORKSPACE_SESSION_STORAGE_KEY = "traceboost.workspace-session";
const PROJECT_GEOSPATIAL_SETTINGS_STORAGE_KEY_PREFIX = "traceboost.project-geospatial-settings:";
const storeHandleCache = new Map<string, string>();
const projectHandleCache = new Map<string, string>();
const outputGrantCache = new Map<string, string>();

function normalizeAuthorizedPath(path: string): string {
  return path.trim();
}

function outputGrantCacheKey(path: string, purpose: OutputGrantPurpose): string {
  return `${purpose}:${normalizeAuthorizedPath(path)}`;
}

function rememberStoreHandle(selection: GrantedPathSelection): void {
  const normalizedPath = normalizeAuthorizedPath(selection.path);
  const normalizedHandle = selection.handleId.trim();
  if (!normalizedPath || !normalizedHandle) {
    return;
  }
  storeHandleCache.set(normalizedPath, normalizedHandle);
}

function rememberProjectHandle(selection: GrantedPathSelection): void {
  const normalizedPath = normalizeAuthorizedPath(selection.path);
  const normalizedHandle = selection.handleId.trim();
  if (!normalizedPath || !normalizedHandle) {
    return;
  }
  projectHandleCache.set(normalizedPath, normalizedHandle);
}

function rememberOutputGrant(path: string, purpose: OutputGrantPurpose, grantId: string): void {
  const normalizedPath = normalizeAuthorizedPath(path);
  const normalizedGrant = grantId.trim();
  if (!normalizedPath || !normalizedGrant) {
    return;
  }
  outputGrantCache.set(outputGrantCacheKey(normalizedPath, purpose), normalizedGrant);
}

const secureTauriArgs = (
  command: DesktopBridgeCommandName,
  args: Record<string, unknown>
): Promise<Record<string, unknown>> => secureTauriArgsImpl(command, args);

async function invokeTauri<T>(
  command: DesktopBridgeCommandName,
  args: Record<string, unknown>
): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, await secureTauriArgs(command, args));
}

async function invokeTauriRaw(
  command: DesktopBridgeCommandName,
  args: Record<string, unknown>
): Promise<Uint8Array> {
  const { invoke } = await import("@tauri-apps/api/core");
  const response = await invoke<Uint8Array | ArrayBuffer>(command, await secureTauriArgs(command, args));
  return response instanceof Uint8Array ? response : new Uint8Array(response);
}

type DesktopGrantedPathCommandName =
  | typeof desktopBridgeCommands.pickRuntimeStore
  | typeof desktopBridgeCommands.pickProjectRoot;

async function pickGrantedPath(
  command: DesktopGrantedPathCommandName,
  args: Record<string, unknown> = {}
): Promise<GrantedPathSelection | null> {
  const { invoke } = await import("@tauri-apps/api/core");
  const selection = await invoke<GrantedPathSelection | null>(command, args);
  if (selection) {
    if (command === desktopBridgeCommands.pickRuntimeStore) {
      rememberStoreHandle(selection);
    } else {
      rememberProjectHandle(selection);
    }
  }
  return selection;
}

async function pickOutputGrant(
  defaultPath: string,
  purpose: OutputGrantPurpose
): Promise<OutputPathGrantSelection | null> {
  const { invoke } = await import("@tauri-apps/api/core");
  const selection = await invoke<OutputPathGrantSelection | null>(desktopBridgeCommands.pickOutputPath, {
    defaultPath,
    purpose
  });
  if (selection) {
    rememberOutputGrant(selection.path, purpose, selection.grantId);
  }
  return selection;
}

async function authorizeManagedStore(path: string): Promise<string | null> {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    const selection = await invoke<GrantedPathSelection>(desktopBridgeCommands.authorizeManagedStore, { path });
    rememberStoreHandle(selection);
    return selection.handleId;
  } catch {
    return null;
  }
}

async function authorizeRuntimeStore(path: string): Promise<string> {
  const { invoke } = await import("@tauri-apps/api/core");
  const selection = await invoke<GrantedPathSelection>(desktopBridgeCommands.authorizeRuntimeStore, {
    path
  });
  rememberStoreHandle(selection);
  return selection.path;
}

async function takeOutputGrant(path: string, purpose: OutputGrantPurpose): Promise<string> {
  const normalizedPath = normalizeAuthorizedPath(path);
  const cacheKey = outputGrantCacheKey(normalizedPath, purpose);
  const cached = outputGrantCache.get(cacheKey);
  if (cached) {
    outputGrantCache.delete(cacheKey);
    return cached;
  }

  if (purpose === "runtime_store_output" || purpose === "gather_store_output") {
    const { invoke } = await import("@tauri-apps/api/core");
    const selection = await invoke<OutputPathGrantSelection>(desktopBridgeCommands.authorizeManagedOutput, {
      path: normalizedPath,
      purpose
    });
    return selection.grantId;
  }

  throw new Error("Output path is not authorized for this session. Use Browse to choose a save location first.");
}

async function ensureStoreHandle(storePath: string): Promise<string> {
  const normalizedPath = normalizeAuthorizedPath(storePath);
  if (!normalizedPath) {
    throw new Error("Runtime store path is required.");
  }
  if (normalizedPath.startsWith("storeh:")) {
    return normalizedPath;
  }
  const cached = storeHandleCache.get(normalizedPath);
  if (cached) {
    return cached;
  }
  const authorized = await authorizeManagedStore(normalizedPath);
  if (authorized) {
    return authorized;
  }
  throw new Error("Runtime store is not authorized for this session. Reopen it through the native picker.");
}

async function ensureProjectHandle(projectRoot: string): Promise<string> {
  const normalizedPath = normalizeAuthorizedPath(projectRoot);
  if (!normalizedPath) {
    throw new Error("Project root is required.");
  }
  if (normalizedPath.startsWith("projh:")) {
    return normalizedPath;
  }
  const cached = projectHandleCache.get(normalizedPath);
  if (cached) {
    return cached;
  }
  throw new Error("Project root is not authorized for this session. Re-select it through the native picker.");
}

async function secureTraceLocalPipeline<T extends { steps: Array<{ operation: unknown }> }>(
  pipeline: T
): Promise<T> {
  const nextSteps = await Promise.all(
    pipeline.steps.map(async (step) => {
      const operation = step.operation as Record<string, unknown>;
      if (
        operation &&
        typeof operation === "object" &&
        "volume_arithmetic" in operation &&
        operation.volume_arithmetic &&
        typeof operation.volume_arithmetic === "object"
      ) {
        const volumeArithmetic = operation.volume_arithmetic as Record<string, unknown>;
        return {
          ...step,
          operation: {
            ...operation,
            volume_arithmetic: {
              ...volumeArithmetic,
              secondary_store_path: await ensureStoreHandle(String(volumeArithmetic.secondary_store_path ?? ""))
            }
          }
        };
      }
      return step;
    })
  );
  return {
    ...pipeline,
    steps: nextSteps
  };
}

async function secureSubvolumePipeline<T extends { trace_local_pipeline?: unknown | null }>(
  pipeline: T
): Promise<T> {
  if (!pipeline.trace_local_pipeline || typeof pipeline.trace_local_pipeline !== "object") {
    return pipeline;
  }
  return {
    ...pipeline,
    trace_local_pipeline: await secureTraceLocalPipeline(
      pipeline.trace_local_pipeline as { steps: Array<{ operation: unknown }> }
    )
  };
}

async function securePostStackNeighborhoodPipeline<T extends { trace_local_pipeline?: unknown | null }>(
  pipeline: T
): Promise<T> {
  if (!pipeline.trace_local_pipeline || typeof pipeline.trace_local_pipeline !== "object") {
    return pipeline;
  }
  return {
    ...pipeline,
    trace_local_pipeline: await secureTraceLocalPipeline(
      pipeline.trace_local_pipeline as { steps: Array<{ operation: unknown }> }
    )
  };
}

async function secureGatherPipeline<T extends { trace_local_pipeline?: unknown | null }>(
  pipeline: T
): Promise<T> {
  if (!pipeline.trace_local_pipeline || typeof pipeline.trace_local_pipeline !== "object") {
    return pipeline;
  }
  return {
    ...pipeline,
    trace_local_pipeline: await secureTraceLocalPipeline(
      pipeline.trace_local_pipeline as { steps: Array<{ operation: unknown }> }
    )
  };
}

async function secureProcessingPipelineSpec(pipeline: ProcessingPipelineSpec): Promise<ProcessingPipelineSpec> {
  if ("trace_local" in pipeline) {
    return {
      trace_local: {
        ...pipeline.trace_local,
        pipeline: await secureTraceLocalPipeline(pipeline.trace_local.pipeline)
      }
    };
  }
  if ("subvolume" in pipeline) {
    return {
      subvolume: {
        ...pipeline.subvolume,
        pipeline: await secureSubvolumePipeline(pipeline.subvolume.pipeline)
      }
    };
  }
  if ("post_stack_neighborhood" in pipeline) {
    return {
      post_stack_neighborhood: {
        ...pipeline.post_stack_neighborhood,
        pipeline: await securePostStackNeighborhoodPipeline(
          pipeline.post_stack_neighborhood.pipeline
        )
      }
    };
  }
  return {
    gather: {
      ...pipeline.gather,
      pipeline: await secureGatherPipeline(pipeline.gather.pipeline)
    }
  };
}

async function secureTauriArgsImpl(
  command: DesktopBridgeCommandName,
  args: Record<string, unknown>
): Promise<Record<string, unknown>> {
  switch (command) {
    case desktopBridgeCommands.openDataset:
    case desktopBridgeCommands.datasetOperatorCatalog:
    case desktopBridgeCommands.getDatasetExportCapabilities:
    case desktopBridgeCommands.ensureDemoSurveyTimeDepthTransform:
    case desktopBridgeCommands.loadVelocityModels:
    case desktopBridgeCommands.loadHorizonAssets:
      return { ...args, storePath: await ensureStoreHandle(String(args.storePath ?? "")) };
    case desktopBridgeCommands.defaultProcessingStorePath:
    case desktopBridgeCommands.defaultSubvolumeProcessingStorePath:
    case desktopBridgeCommands.defaultPostStackNeighborhoodProcessingStorePath:
    case desktopBridgeCommands.defaultGatherProcessingStorePath:
      return { ...args, storePath: await ensureStoreHandle(String(args.storePath ?? "")) };
    case desktopBridgeCommands.exportDatasetSegy:
      return {
        ...args,
        storePath: await ensureStoreHandle(String(args.storePath ?? "")),
        outputPath: await takeOutputGrant(String(args.outputPath ?? ""), "segy_export")
      };
    case desktopBridgeCommands.exportDatasetZarr:
      return {
        ...args,
        storePath: await ensureStoreHandle(String(args.storePath ?? "")),
        outputPath: await takeOutputGrant(String(args.outputPath ?? ""), "zarr_export")
      };
    case desktopBridgeCommands.previewHorizonXyzImport:
    case desktopBridgeCommands.importHorizonXyz:
      return { ...args, storePath: await ensureStoreHandle(String(args.storePath ?? "")) };
    case desktopBridgeCommands.commitHorizonSourceImport: {
      const request = args.request as Record<string, unknown>;
      return {
        request: {
          ...request,
          store_path: await ensureStoreHandle(String(request.store_path ?? ""))
        }
      };
    }
    case desktopBridgeCommands.loadSectionHorizons:
    case desktopBridgeCommands.loadSection:
    case desktopBridgeCommands.loadSectionBinary:
    case desktopBridgeCommands.loadSectionTileBinary:
    case desktopBridgeCommands.loadDepthConvertedSectionBinary:
    case desktopBridgeCommands.loadResolvedSectionDisplayBinary:
    case desktopBridgeCommands.loadGather:
      return { ...args, storePath: await ensureStoreHandle(String(args.storePath ?? "")) };
    case desktopBridgeCommands.describeVelocityVolume:
    case desktopBridgeCommands.buildVelocityModelTransform:
    case desktopBridgeCommands.setDatasetNativeCoordinateReference:
    case desktopBridgeCommands.resolveSurveyMap: {
      const request = args.request as Record<string, unknown>;
      return {
        request: {
          ...request,
          store_path: await ensureStoreHandle(String(request.store_path ?? request.storePath ?? ""))
        }
      };
    }
    case desktopBridgeCommands.ingestVelocityVolume: {
      const request = args.request as Record<string, unknown>;
      return {
        request: {
          ...request,
          output_store_path: await takeOutputGrant(
            String(request.output_store_path ?? ""),
            "runtime_store_output"
          )
        }
      };
    }
    case desktopBridgeCommands.importDataset:
      return {
        ...args,
        outputStorePath: await takeOutputGrant(String(args.outputStorePath ?? ""), "runtime_store_output")
      };
    case desktopBridgeCommands.importSegyWithPlan: {
      const request = structuredClone(args.request as Record<string, unknown>);
      const plan = request.plan as Record<string, unknown>;
      const policy = plan.policy as Record<string, unknown>;
      policy.output_store_path = await takeOutputGrant(
        String(policy.output_store_path ?? ""),
        "runtime_store_output"
      );
      return { request };
    }
    case desktopBridgeCommands.previewProcessing:
    case desktopBridgeCommands.previewProcessingBinary: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await secureTraceLocalPipeline(
        request.pipeline as { steps: Array<{ operation: unknown }> }
      );
      return { request };
    }
    case desktopBridgeCommands.previewSubvolumeProcessing:
    case desktopBridgeCommands.previewSubvolumeProcessingBinary: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await secureSubvolumePipeline(
        request.pipeline as { trace_local_pipeline?: unknown | null }
      );
      return { request };
    }
    case desktopBridgeCommands.previewPostStackNeighborhoodProcessing:
    case desktopBridgeCommands.previewPostStackNeighborhoodProcessingBinary: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await securePostStackNeighborhoodPipeline(
        request.pipeline as { trace_local_pipeline?: unknown | null }
      );
      return { request };
    }
    case desktopBridgeCommands.previewGatherProcessing:
    case desktopBridgeCommands.runGatherProcessing: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await secureGatherPipeline(
        request.pipeline as { trace_local_pipeline?: unknown | null }
      );
      if (command === desktopBridgeCommands.runGatherProcessing && request.output_store_path) {
        request.output_store_path = await takeOutputGrant(
          String(request.output_store_path),
          "gather_store_output"
        );
      }
      return { request };
    }
    case desktopBridgeCommands.amplitudeSpectrum:
    case desktopBridgeCommands.velocityScan: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      return { request };
    }
    case desktopBridgeCommands.runProcessing: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await secureTraceLocalPipeline(
        request.pipeline as { steps: Array<{ operation: unknown }> }
      );
      if (request.output_store_path) {
        request.output_store_path = await takeOutputGrant(
          String(request.output_store_path),
          "runtime_store_output"
        );
      }
      return { request };
    }
    case desktopBridgeCommands.submitTraceLocalProcessingBatch: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.pipeline = await secureTraceLocalPipeline(
        request.pipeline as { steps: Array<{ operation: unknown }> }
      );
      request.items = await Promise.all(
        (request.items as Array<Record<string, unknown>>).map(async (item) => {
          const nextItem: Record<string, unknown> = {
            ...item,
            store_path: await ensureStoreHandle(String(item.store_path ?? ""))
          };
          if (item.output_store_path) {
            nextItem.output_store_path = await takeOutputGrant(
              String(item.output_store_path),
              "runtime_store_output"
            );
          }
          return nextItem;
        })
      );
      return { request };
    }
    case desktopBridgeCommands.submitProcessingBatch: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.pipeline = await secureProcessingPipelineSpec(
        request.pipeline as ProcessingPipelineSpec
      );
      const outputGrantPurpose: OutputGrantPurpose =
        "gather" in (request.pipeline as ProcessingPipelineSpec)
          ? "gather_store_output"
          : "runtime_store_output";
      request.items = await Promise.all(
        (request.items as Array<Record<string, unknown>>).map(async (item) => {
          const nextItem: Record<string, unknown> = {
            ...item,
            store_path: await ensureStoreHandle(String(item.store_path ?? ""))
          };
          if (item.output_store_path) {
            nextItem.output_store_path = await takeOutputGrant(
              String(item.output_store_path),
              outputGrantPurpose
            );
          }
          return nextItem;
        })
      );
      return { request };
    }
    case desktopBridgeCommands.runSubvolumeProcessing: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await secureSubvolumePipeline(
        request.pipeline as { trace_local_pipeline?: unknown | null }
      );
      if (request.output_store_path) {
        request.output_store_path = await takeOutputGrant(
          String(request.output_store_path),
          "runtime_store_output"
        );
      }
      return { request };
    }
    case desktopBridgeCommands.runPostStackNeighborhoodProcessing: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.store_path = await ensureStoreHandle(String(request.store_path ?? ""));
      request.pipeline = await securePostStackNeighborhoodPipeline(
        request.pipeline as { trace_local_pipeline?: unknown | null }
      );
      if (request.output_store_path) {
        request.output_store_path = await takeOutputGrant(
          String(request.output_store_path),
          "runtime_store_output"
        );
      }
      return { request };
    }
    case desktopBridgeCommands.saveWorkspaceSession: {
      const request = structuredClone(args.request as Record<string, unknown>);
      if (request.active_store_path) {
        request.active_store_path = await ensureStoreHandle(String(request.active_store_path));
      }
      if (request.project_root) {
        request.project_root = await ensureProjectHandle(String(request.project_root));
      }
      if (Array.isArray(request.native_engineering_accepted_store_paths)) {
        request.native_engineering_accepted_store_paths = await Promise.all(
          request.native_engineering_accepted_store_paths.map((path) => ensureStoreHandle(String(path)))
        );
      }
      return { request };
    }
    case desktopBridgeCommands.upsertDatasetEntry: {
      const request = structuredClone(args.request as Record<string, unknown>);
      if (request.preferred_store_path) {
        request.preferred_store_path = await ensureStoreHandle(String(request.preferred_store_path));
      }
      if (request.imported_store_path) {
        request.imported_store_path = await ensureStoreHandle(String(request.imported_store_path));
      }
      if (request.dataset && typeof request.dataset === "object") {
        (request.dataset as Record<string, unknown>).store_path = await ensureStoreHandle(
          String((request.dataset as Record<string, unknown>).store_path ?? "")
        );
      }
      return { request };
    }
    case desktopBridgeCommands.loadProjectGeospatialSettings:
    case desktopBridgeCommands.saveProjectGeospatialSettings: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.projectRoot = await ensureProjectHandle(String(request.projectRoot ?? ""));
      return { request };
    }
    case desktopBridgeCommands.resolveProjectSurveyMap:
    case desktopBridgeCommands.listProjectWellTimeDepthModels:
    case desktopBridgeCommands.listProjectWellTimeDepthInventory:
    case desktopBridgeCommands.listProjectWellOverlayInventory:
    case desktopBridgeCommands.listProjectSurveyHorizons:
    case desktopBridgeCommands.listProjectWellMarkerResidualInventory:
    case desktopBridgeCommands.computeProjectWellMarkerResidual:
    case desktopBridgeCommands.setProjectActiveWellTimeDepthModel:
    case desktopBridgeCommands.importProjectWellTimeDepthModel:
    case desktopBridgeCommands.importProjectWellTimeDepthAsset:
    case desktopBridgeCommands.commitProjectWellTimeDepthImport:
    case desktopBridgeCommands.compileProjectWellTimeDepthAuthoredModel:
    case desktopBridgeCommands.readProjectWellTimeDepthModel:
    case desktopBridgeCommands.commitProjectWellSources:
    case desktopBridgeCommands.commitProjectWellImport: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.projectRoot = await ensureProjectHandle(String(request.projectRoot ?? ""));
      return { request };
    }
    case desktopBridgeCommands.analyzeProjectWellTie:
    case desktopBridgeCommands.acceptProjectWellTie: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.projectRoot = await ensureProjectHandle(String(request.projectRoot ?? ""));
      request.storePath = await ensureStoreHandle(String(request.storePath ?? ""));
      return { request };
    }
    case desktopBridgeCommands.resolveProjectSectionWellOverlays: {
      const request = structuredClone(args.request as Record<string, unknown>);
      request.project_root = await ensureProjectHandle(String(request.project_root ?? ""));
      return { request };
    }
    case desktopBridgeCommands.scanVendorProject:
    case desktopBridgeCommands.planVendorProjectImport:
    case desktopBridgeCommands.commitVendorProjectImport: {
      const request = structuredClone(args.request as Record<string, unknown>);
      if (request.project_root) {
        request.project_root = await ensureProjectHandle(String(request.project_root));
      }
      if (request.target_project_root) {
        request.target_project_root = await ensureProjectHandle(String(request.target_project_root));
      }
      return { request };
    }
    default:
      return args;
  }
}

export async function pickDesktopRuntimeStore(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return null;
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Open Volume",
    filters: [
      { name: "Runtime Stores", extensions: ["tbvol"] },
      { name: "All Files", extensions: ["*"] }
    ],
    multiple: false,
    directory: false
  });
  const path = typeof result === "string" ? result.trim() : "";
  if (!path) {
    return null;
  }
  return authorizeRuntimeStore(path);
}

export async function pickDesktopProjectRoot(title: string): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return null;
  }
  const selection = await pickGrantedPath(desktopBridgeCommands.pickProjectRoot, { title });
  return selection?.path ?? null;
}

export async function pickDesktopOutputPath(
  defaultPath: string,
  purpose: OutputGrantPurpose
): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return null;
  }
  const selection = await pickOutputGrant(defaultPath, purpose);
  return selection?.path ?? null;
}

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || "Backend request failed");
  }
  return response.json() as Promise<T>;
}

async function postJson<T>(url: string, body: Record<string, unknown>): Promise<T> {
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(body)
  });
  return readJson<T>(response);
}

function operationSlug(
  operation:
    | TraceLocalProcessingOperation
    | RunProcessingRequest["pipeline"]["steps"][number]["operation"]
): string {
  if (typeof operation === "string") {
    return "trace-rms-normalize";
  }
  if ("amplitude_scalar" in operation) {
    return `amplitude-scalar-${String(operation.amplitude_scalar.factor).replace(".", "_")}`;
  }
  if ("agc_rms" in operation) {
    return `agc-rms-${String(operation.agc_rms.window_ms).replace(".", "_")}`;
  }
  if ("phase_rotation" in operation) {
    return `phase-rotation-${String(operation.phase_rotation.angle_degrees).replace(".", "_")}`;
  }
  if ("lowpass_filter" in operation) {
    return `lowpass-${[operation.lowpass_filter.f3_hz, operation.lowpass_filter.f4_hz]
      .map((value) => String(value).replace(".", "_"))
      .join("-")}`;
  }
  if ("highpass_filter" in operation) {
    return `highpass-${[operation.highpass_filter.f1_hz, operation.highpass_filter.f2_hz]
      .map((value) => String(value).replace(".", "_"))
      .join("-")}`;
  }
  if ("volume_arithmetic" in operation) {
    const secondaryStem =
      fileStem(operation.volume_arithmetic.secondary_store_path)
        .toLowerCase()
        .replace(/[^a-z0-9_-]+/g, "-")
        .replace(/^-+|-+$/g, "") || "volume";
    return `volume-${operation.volume_arithmetic.operator}-${secondaryStem}`;
  }
  return `bandpass-${[
    operation.bandpass_filter.f1_hz,
    operation.bandpass_filter.f2_hz,
    operation.bandpass_filter.f3_hz,
    operation.bandpass_filter.f4_hz
  ]
    .map((value) => String(value).replace(".", "_"))
    .join("-")}`;
}

function defaultWorkspaceSession(): WorkspaceSession {
  return {
    active_entry_id: null,
    active_store_path: null,
    active_axis: "inline",
    active_index: 0,
    selected_preset_id: null,
    display_coordinate_reference_id: null,
    active_velocity_model_asset_id: null,
    project_root: null,
    project_survey_asset_id: null,
    project_wellbore_id: null,
    project_section_tolerance_m: null,
    selected_project_well_time_depth_model_asset_id: null,
    native_engineering_accepted_store_paths: []
  };
}

function storageAvailable(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function unixTimestampBigInt(): bigint {
  return BigInt(Math.floor(Date.now() / 1000));
}

function parseOptionalUnixTimestamp(value: unknown): bigint | null {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "bigint") {
    return value;
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    return BigInt(Math.trunc(value));
  }
  if (typeof value === "string" && /^-?\d+$/.test(value.trim())) {
    return BigInt(value.trim());
  }
  return null;
}

function parseRequiredUnixTimestamp(value: unknown): bigint {
  return parseOptionalUnixTimestamp(value) ?? 0n;
}

function serializeJsonWithBigInt(value: unknown): string {
  return JSON.stringify(value, (_key, nestedValue) =>
    typeof nestedValue === "bigint" ? nestedValue.toString() : nestedValue
  );
}

function loadLocalRegistry(): DatasetRegistryEntry[] {
  if (!storageAvailable()) {
    return [];
  }
  const stored = window.localStorage.getItem(DATASET_REGISTRY_STORAGE_KEY);
  if (!stored) {
    return [];
  }
  try {
    const parsed = JSON.parse(stored) as DatasetRegistryEntry[];
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.map((entry) => ({
      ...entry,
      last_opened_at_unix_s: parseOptionalUnixTimestamp(entry.last_opened_at_unix_s),
      last_imported_at_unix_s: parseOptionalUnixTimestamp(entry.last_imported_at_unix_s),
      updated_at_unix_s: parseRequiredUnixTimestamp(entry.updated_at_unix_s)
    }));
  } catch {
    return [];
  }
}

function loadLocalSession(): WorkspaceSession {
  const defaults = defaultWorkspaceSession();
  if (!storageAvailable()) {
    return defaults;
  }
  const stored = window.localStorage.getItem(WORKSPACE_SESSION_STORAGE_KEY);
  if (!stored) {
    return defaults;
  }
  try {
    const parsed = JSON.parse(stored) as Partial<WorkspaceSession>;
    return {
      ...defaults,
      ...parsed,
      native_engineering_accepted_store_paths: Array.isArray(
        parsed.native_engineering_accepted_store_paths
      )
        ? parsed.native_engineering_accepted_store_paths
            .map((value) => String(value).trim())
            .filter((value, index, values) => value.length > 0 && values.indexOf(value) === index)
        : []
    };
  } catch {
    return defaults;
  }
}

function saveLocalRegistry(entries: DatasetRegistryEntry[]): void {
  if (!storageAvailable()) {
    return;
  }
  window.localStorage.setItem(DATASET_REGISTRY_STORAGE_KEY, serializeJsonWithBigInt(entries));
}

function entryStoreIdentity(entry: DatasetRegistryEntry): { storeId: string; storePath: string } | null {
  const storeId = entry.last_dataset?.descriptor.store_id?.trim();
  const storePath =
    entry.last_dataset?.store_path?.trim() ||
    entry.imported_store_path?.trim() ||
    entry.preferred_store_path?.trim() ||
    "";
  if (!storeId || !storePath) {
    return null;
  }
  return { storeId, storePath };
}

function ensureUniqueStoreIdentityLocal(
  entries: DatasetRegistryEntry[],
  request: UpsertDatasetEntryRequest,
  existingIndex: number
): void {
  const storeId = request.dataset?.descriptor.store_id?.trim();
  const storePath = request.dataset?.store_path?.trim();
  if (!storeId || !storePath) {
    return;
  }

  for (let index = 0; index < entries.length; index += 1) {
    if (index === existingIndex) {
      continue;
    }
    const identity = entryStoreIdentity(entries[index]);
    if (!identity || identity.storeId !== storeId) {
      continue;
    }
    if (identity.storePath === storePath) {
      continue;
    }
    throw new Error(
      `Refusing to register duplicate store identity '${storeId}' for '${storePath}' because it is already used by '${entries[index].display_name}' at '${identity.storePath}'. This usually means a store folder was copied outside TraceBoost.`
    );
  }
}

function saveLocalSession(session: WorkspaceSession): void {
  if (!storageAvailable()) {
    return;
  }
  window.localStorage.setItem(WORKSPACE_SESSION_STORAGE_KEY, JSON.stringify(session));
}

function projectGeospatialSettingsStorageKey(projectRoot: string): string {
  return `${PROJECT_GEOSPATIAL_SETTINGS_STORAGE_KEY_PREFIX}${projectRoot.trim()}`;
}

function normalizeLocalProjectDisplayCoordinateReference(
  value: ProjectGeospatialSettings["displayCoordinateReference"]
): ProjectDisplayCoordinateReference | null {
  if (!value || typeof value !== "object" || typeof value.kind !== "string") {
    return null;
  }
  const legacyValue = value as { kind?: unknown; coordinateReferenceId?: unknown };
  if (value.kind === "native_engineering") {
    return { kind: "native_engineering" };
  }
  if (
    value.kind === "authority_code" &&
    typeof value.authority === "string" &&
    typeof value.code === "string" &&
    typeof value.authId === "string"
  ) {
    return {
      kind: "authority_code",
      authority: value.authority.trim().toUpperCase(),
      code: value.code.trim(),
      authId: value.authId.trim().toUpperCase(),
      name: typeof value.name === "string" ? value.name.trim() || null : null
    };
  }
  if (legacyValue.kind === "coordinate_reference_id" && typeof legacyValue.coordinateReferenceId === "string") {
    const authId = legacyValue.coordinateReferenceId.trim().toUpperCase();
    const [authority, code] = authId.split(":", 2);
    if (authority && code) {
      return {
        kind: "authority_code",
        authority,
        code,
        authId,
        name: null
      };
    }
  }
  return null;
}

function loadLocalProjectGeospatialSettings(projectRoot: string): ProjectGeospatialSettings | null {
  if (!storageAvailable()) {
    return null;
  }
  const key = projectGeospatialSettingsStorageKey(projectRoot);
  const stored = window.localStorage.getItem(key);
  if (!stored) {
    return null;
  }
  try {
    const parsed = JSON.parse(stored) as ProjectGeospatialSettings;
    const normalizedDisplayCoordinateReference = normalizeLocalProjectDisplayCoordinateReference(
      parsed.displayCoordinateReference
    );
    if (!normalizedDisplayCoordinateReference) {
      return null;
    }
    return {
      ...parsed,
      displayCoordinateReference: normalizedDisplayCoordinateReference
    };
  } catch {
    return null;
  }
}

function saveLocalProjectGeospatialSettings(projectRoot: string, settings: ProjectGeospatialSettings): void {
  if (!storageAvailable()) {
    return;
  }
  const key = projectGeospatialSettingsStorageKey(projectRoot);
  window.localStorage.setItem(key, JSON.stringify(settings));
}

function fileStem(filePath: string | null | undefined): string {
  const normalized = filePath?.trim() ?? "";
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

function userVisibleDatasetName(entry: DatasetRegistryEntry): string {
  const sourceStem = fileStem(entry.source_path);
  if (sourceStem) {
    return sourceStem;
  }
  const trimmedDisplayName = entry.display_name?.trim() ?? "";
  if (trimmedDisplayName) {
    return stripGeneratedHashSuffix(trimmedDisplayName);
  }
  const storeStem = fileStem(entry.imported_store_path ?? entry.preferred_store_path);
  if (storeStem) {
    return stripGeneratedHashSuffix(storeStem);
  }
  return entry.entry_id;
}

function sortEntries(entries: DatasetRegistryEntry[]): DatasetRegistryEntry[] {
  return [...entries].sort((left, right) => {
    const byName = userVisibleDatasetName(left).localeCompare(userVisibleDatasetName(right), undefined, {
      sensitivity: "base",
      numeric: true
    });
    if (byName !== 0) {
      return byName;
    }
    return left.entry_id.localeCompare(right.entry_id, undefined, { sensitivity: "base", numeric: true });
  });
}

function resolveEntryStatus(entry: DatasetRegistryEntry): DatasetRegistryStatus {
  if (entry.source_path) {
    return entry.imported_store_path ? "imported" : "linked";
  }
  return entry.imported_store_path ? "imported" : "linked";
}

export async function listImportProviders(): Promise<ImportProviderDescriptor[]> {
  if (isTauriEnvironment()) {
    const response = await invokeTauri<ListImportProvidersResponse>(desktopBridgeCommands.listImportProviders, {});
    return response.providers;
  }

  throw new Error("Import provider discovery is only available in the desktop runtime right now.");
}

export async function beginImportSession(
  request: BeginImportSessionRequest
): Promise<ImportSessionEnvelope> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportSessionEnvelope>(desktopBridgeCommands.beginImportSession, { request });
  }

  throw new Error("Import session orchestration is only available in the desktop runtime right now.");
}

export async function preflightImport(
  inputPath: string,
  geometryOverride: SegyGeometryOverride | null = null
): Promise<SurveyPreflightResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SurveyPreflightResponse>(desktopBridgeCommands.preflightImport, { inputPath, geometryOverride });
  }

  return postJson<SurveyPreflightResponse>("/api/preflight", { inputPath, geometryOverride });
}

export async function importDataset(
  inputPath: string,
  outputStorePath: string,
  overwriteExisting = false,
  geometryOverride: SegyGeometryOverride | null = null
): Promise<ImportDatasetResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportDatasetResponse>(desktopBridgeCommands.importDataset, {
      inputPath,
      outputStorePath,
      geometryOverride,
      overwriteExisting
    });
  }

  return postJson<ImportDatasetResponse>("/api/import", {
    inputPath,
    outputStorePath,
    geometryOverride,
    overwriteExisting
  });
}

export async function scanSegyImport(inputPath: string): Promise<SegyImportScanResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SegyImportScanResponse>(desktopBridgeCommands.scanSegyImport, { inputPath });
  }

  return postJson<SegyImportScanResponse>("/api/segy-import/scan", { inputPath });
}

export async function validateSegyImportPlan(
  plan: SegyImportPlan
): Promise<SegyImportValidationResponse> {
  const request = {
    schema_version: SCHEMA_VERSION,
    plan
  };
  if (isTauriEnvironment()) {
    return invokeTauri<SegyImportValidationResponse>(desktopBridgeCommands.validateSegyImportPlan, {
      request
    });
  }

  return postJson<SegyImportValidationResponse>("/api/segy-import/validate", request);
}

export async function importSegyWithPlan(
  plan: SegyImportPlan,
  validationFingerprint: string
): Promise<ImportSegyWithPlanResponse> {
  const request = {
    schema_version: SCHEMA_VERSION,
    plan,
    validation_fingerprint: validationFingerprint
  };
  if (isTauriEnvironment()) {
    return invokeTauri<ImportSegyWithPlanResponse>(desktopBridgeCommands.importSegyWithPlan, { request });
  }

  return postJson<ImportSegyWithPlanResponse>("/api/segy-import/import", request);
}

export async function listSegyImportRecipes(
  sourceFingerprint: string | null = null
): Promise<ListSegyImportRecipesResponse> {
  const request = {
    schema_version: SCHEMA_VERSION,
    source_fingerprint: sourceFingerprint
  };
  if (isTauriEnvironment()) {
    return invokeTauri<ListSegyImportRecipesResponse>(desktopBridgeCommands.listSegyImportRecipes, {
      request
    });
  }

  return postJson<ListSegyImportRecipesResponse>("/api/segy-import/recipes/list", request);
}

export async function saveSegyImportRecipe(
  recipe: SegyImportRecipe
): Promise<SaveSegyImportRecipeResponse> {
  const request = {
    schema_version: SCHEMA_VERSION,
    recipe
  };
  if (isTauriEnvironment()) {
    return invokeTauri<SaveSegyImportRecipeResponse>(desktopBridgeCommands.saveSegyImportRecipe, {
      request
    });
  }

  return postJson<SaveSegyImportRecipeResponse>("/api/segy-import/recipes/save", request);
}

export async function deleteSegyImportRecipe(recipeId: string): Promise<boolean> {
  const request = {
    schema_version: SCHEMA_VERSION,
    recipe_id: recipeId
  };
  if (isTauriEnvironment()) {
    const response = await invokeTauri<{ deleted: boolean }>(desktopBridgeCommands.deleteSegyImportRecipe, {
      request
    });
    return response.deleted;
  }

  const response = await postJson<{ deleted: boolean }>("/api/segy-import/recipes/delete", request);
  return response.deleted;
}

export async function defaultImportStorePath(inputPath: string): Promise<string> {
  if (isTauriEnvironment()) {
    return invokeTauri<string>(desktopBridgeCommands.defaultImportStorePath, { inputPath });
  }

  const normalized = inputPath.trim();
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalized.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalized.slice(separatorIndex + 1) : normalized;
  const basename = filename.replace(/\.[^.]+$/, "");
  return `${directory}${basename}.tbvol`;
}

export async function openDataset(storePath: string): Promise<OpenDatasetResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<OpenDatasetResponse>(desktopBridgeCommands.openDataset, { storePath });
  }

  return postJson<OpenDatasetResponse>("/api/open", { storePath });
}

export async function resolveProcessingAuthoringPalette(
  request: ResolveProcessingAuthoringPaletteRequest
): Promise<ResolveProcessingAuthoringPaletteResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ResolveProcessingAuthoringPaletteResponse>(
      desktopBridgeCommands.resolveProcessingAuthoringPalette,
      { request }
    );
  }

  throw new Error("Processing authoring palette resolution is only available in the desktop runtime right now.");
}

export async function applyProcessingAuthoringSessionAction(
  request: ApplyProcessingAuthoringSessionActionRequest
): Promise<ProcessingAuthoringSessionResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProcessingAuthoringSessionResponse>(
      desktopBridgeCommands.applyProcessingAuthoringSessionAction,
      { request }
    );
  }

  throw new Error("Processing authoring session actions are only available in the desktop runtime right now.");
}

export async function saveProcessingAuthoringSession(
  request: SaveProcessingAuthoringSessionRequest
): Promise<SaveProcessingAuthoringSessionResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SaveProcessingAuthoringSessionResponse>(
      desktopBridgeCommands.saveProcessingAuthoringSession,
      { request }
    );
  }

  throw new Error("Processing authoring persistence is only available in the desktop runtime right now.");
}

export async function resolveProcessingRunOutput(
  request: ResolveProcessingRunOutputRequest
): Promise<ResolveProcessingRunOutputResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ResolveProcessingRunOutputResponse>(desktopBridgeCommands.resolveProcessingRunOutput, {
      request
    });
  }

  throw new Error("Processing run output resolution is only available in the desktop runtime right now.");
}

export async function exportDatasetSegy(
  storePath: string,
  outputPath: string,
  overwriteExisting = false
): Promise<ExportSegyResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ExportSegyResponse>(desktopBridgeCommands.exportDatasetSegy, {
      storePath,
      outputPath,
      overwriteExisting
    });
  }

  return postJson<ExportSegyResponse>("/api/export/segy", {
    storePath,
    outputPath,
    overwriteExisting
  });
}

export async function exportDatasetZarr(
  storePath: string,
  outputPath: string,
  overwriteExisting = false
): Promise<ExportZarrResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ExportZarrResponse>(desktopBridgeCommands.exportDatasetZarr, {
      storePath,
      outputPath,
      overwriteExisting
    });
  }

  return postJson<ExportZarrResponse>("/api/export/zarr", {
    storePath,
    outputPath,
    overwriteExisting
  });
}

export async function getDatasetExportCapabilities(
  storePath: string
): Promise<DatasetExportCapabilitiesResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<DatasetExportCapabilitiesResponse>(desktopBridgeCommands.getDatasetExportCapabilities, {
      storePath
    });
  }

  throw new Error("Dataset export capability lookup is only available in the desktop runtime.");
}

export async function importHorizonXyz(
  storePath: string,
  inputPaths: string[],
  options: HorizonImportCoordinateReferenceOptions = {}
): Promise<ImportHorizonXyzResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportHorizonXyzResponse>(desktopBridgeCommands.importHorizonXyz, {
      storePath,
      inputPaths,
      verticalDomain: options.verticalDomain ?? null,
      verticalUnit: options.verticalUnit ?? null,
      sourceCoordinateReferenceId: options.sourceCoordinateReferenceId ?? null,
      sourceCoordinateReferenceName: options.sourceCoordinateReferenceName ?? null,
      assumeSameAsSurvey: options.assumeSameAsSurvey === true
    });
  }

  return postJson<ImportHorizonXyzResponse>("/api/horizons/import", {
    storePath,
    inputPaths,
    verticalDomain: options.verticalDomain ?? null,
    verticalUnit: options.verticalUnit ?? null,
    sourceCoordinateReferenceId: options.sourceCoordinateReferenceId ?? null,
    sourceCoordinateReferenceName: options.sourceCoordinateReferenceName ?? null,
    assumeSameAsSurvey: options.assumeSameAsSurvey === true
  });
}

export async function previewHorizonXyzImport(
  storePath: string,
  inputPaths: string[],
  options: HorizonImportCoordinateReferenceOptions = {}
): Promise<HorizonImportPreview> {
  if (isTauriEnvironment()) {
    return invokeTauri<HorizonImportPreview>(desktopBridgeCommands.previewHorizonXyzImport, {
      storePath,
      inputPaths,
      verticalDomain: options.verticalDomain ?? null,
      verticalUnit: options.verticalUnit ?? null,
      sourceCoordinateReferenceId: options.sourceCoordinateReferenceId ?? null,
      sourceCoordinateReferenceName: options.sourceCoordinateReferenceName ?? null,
      assumeSameAsSurvey: options.assumeSameAsSurvey === true
    });
  }

  throw new Error("Horizon import preview is only available in the desktop runtime right now.");
}

export async function previewHorizonSourceImport(
  request: PreviewHorizonSourceImportRequest
): Promise<HorizonSourceImportPreview> {
  if (isTauriEnvironment()) {
    return invokeTauri<HorizonSourceImportPreview>(desktopBridgeCommands.previewHorizonSourceImport, {
      request
    });
  }

  throw new Error("Horizon source import preview is only available in the desktop runtime right now.");
}

export async function inspectHorizonXyzFiles(
  inputPaths: string[]
): Promise<HorizonXyzFilePreview[]> {
  if (isTauriEnvironment()) {
    return invokeTauri<HorizonXyzFilePreview[]>(desktopBridgeCommands.inspectHorizonXyzFiles, {
      inputPaths
    });
  }

  throw new Error("Horizon xyz inspection is only available in the desktop runtime right now.");
}

export async function commitHorizonSourceImport(
  request: CommitHorizonSourceImportRequest
): Promise<ImportHorizonXyzResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportHorizonXyzResponse>(desktopBridgeCommands.commitHorizonSourceImport, {
      request
    });
  }

  throw new Error("Horizon source import is only available in the desktop runtime right now.");
}

export async function fetchSectionHorizons(
  storePath: string,
  axis: SectionAxis,
  index: number
): Promise<SectionHorizonOverlayView[]> {
  if (isTauriEnvironment()) {
    const response = await invokeTauri<{ schema_version: number; overlays: SectionHorizonOverlayView[] }>(
      desktopBridgeCommands.loadSectionHorizons,
      {
        storePath,
        axis,
        index
      }
    );
    return response.overlays;
  }

  const response = await fetch(
    `/api/horizons/section?storePath=${encodeURIComponent(storePath)}&axis=${encodeURIComponent(axis)}&index=${encodeURIComponent(index)}`
  );
  const payload = await readJson<{ schema_version: number; overlays: SectionHorizonOverlayView[] }>(response);
  return payload.overlays;
}

export async function fetchSectionView(
  storePath: string,
  axis: SectionAxis,
  index: number
): Promise<TransportSectionView | SectionView> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.loadSectionBinary, {
      storePath,
      axis,
      index
    });
    return parsePackedSectionViewResponse(payload);
  }

  const response = await fetch(
    `/api/section?storePath=${encodeURIComponent(storePath)}&axis=${encodeURIComponent(axis)}&index=${encodeURIComponent(index)}`
  );
  return readJson<SectionView>(response);
}

export async function fetchSectionTileView(
  storePath: string,
  axis: SectionAxis,
  index: number,
  traceRange: [number, number],
  sampleRange: [number, number],
  lod: number
): Promise<TransportSectionTileView> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.loadSectionTileBinary, {
      storePath,
      axis,
      index,
      traceRange,
      sampleRange,
      lod
    });
    return parsePackedSectionTileResponse(payload);
  }

  throw new Error("Section tile loading is only available in the desktop runtime right now.");
}

export async function fetchDepthConvertedSectionView(
  storePath: string,
  axis: SectionAxis,
  index: number,
  velocityModel: VelocityFunctionSource,
  velocityKind: VelocityQuantityKind
): Promise<TransportSectionView | SectionView> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.loadDepthConvertedSectionBinary, {
      storePath,
      axis,
      index,
      velocityModel,
      velocityKind
    });
    return parsePackedSectionViewResponse(payload);
  }

  throw new Error("Depth-converted section loading is only available in the desktop runtime right now.");
}

export async function fetchResolvedSectionDisplay(
  storePath: string,
  axis: SectionAxis,
  index: number,
  domain: "time" | "depth",
  velocityModel: VelocityFunctionSource | null,
  velocityKind: VelocityQuantityKind | null,
  includeVelocityOverlay: boolean
): Promise<TransportResolvedSectionDisplayView> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.loadResolvedSectionDisplayBinary, {
      storePath,
      axis,
      index,
      domain,
      velocityModel,
      velocityKind,
      includeVelocityOverlay
    });
    return parsePackedSectionDisplayResponse(payload);
  }

  throw new Error("Resolved section display loading is only available in the desktop runtime right now.");
}

export async function ensureDemoSurveyTimeDepthTransform(storePath: string): Promise<string> {
  if (isTauriEnvironment()) {
    return invokeTauri<string>(desktopBridgeCommands.ensureDemoSurveyTimeDepthTransform, { storePath });
  }

  throw new Error("Synthetic survey 3D time-depth transforms are only available in the desktop runtime right now.");
}

export async function loadVelocityModels(storePath: string): Promise<SurveyTimeDepthTransform3D[]> {
  if (isTauriEnvironment()) {
    const response = await invokeTauri<LoadVelocityModelsResponse>(desktopBridgeCommands.loadVelocityModels, {
      storePath
    });
    return response.models;
  }

  return [];
}

export async function describeVelocityVolume(
  request: DescribeVelocityVolumeRequest
): Promise<DescribeVelocityVolumeResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<DescribeVelocityVolumeResponse>(desktopBridgeCommands.describeVelocityVolume, {
      request
    });
  }

  throw new Error("Velocity-volume description is only available in the desktop runtime right now.");
}

export async function ingestVelocityVolume(
  request: IngestVelocityVolumeRequest
): Promise<IngestVelocityVolumeResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<IngestVelocityVolumeResponse>(desktopBridgeCommands.ingestVelocityVolume, {
      request
    });
  }

  throw new Error("Velocity-volume ingest is only available in the desktop runtime right now.");
}

export async function loadHorizonAssets(storePath: string): Promise<ImportedHorizonDescriptor[]> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportedHorizonDescriptor[]>(desktopBridgeCommands.loadHorizonAssets, {
      storePath
    });
  }

  return [];
}

export async function importVelocityFunctionsModel(
  storePath: string,
  inputPath: string,
  velocityKind: VelocityQuantityKind
): Promise<ImportVelocityFunctionsModelResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportVelocityFunctionsModelResponse>(desktopBridgeCommands.importVelocityFunctionsModel, {
      storePath,
      inputPath,
      velocityKind
    });
  }

  throw new Error("Velocity-functions import is only available in the desktop runtime right now.");
}

export async function buildVelocityModelTransform(
  request: BuildSurveyTimeDepthTransformRequest
): Promise<SurveyTimeDepthTransform3D> {
  if (isTauriEnvironment()) {
    return invokeTauri<SurveyTimeDepthTransform3D>(desktopBridgeCommands.buildVelocityModelTransform, {
      request
    });
  }

  throw new Error("Authored velocity-model building is only available in the desktop runtime right now.");
}

export async function convertHorizonDomain(
  storePath: string,
  sourceHorizonId: string,
  transformId: string,
  targetDomain: "time" | "depth",
  outputId?: string | null,
  outputName?: string | null
): Promise<ImportedHorizonDescriptor> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportedHorizonDescriptor>(desktopBridgeCommands.convertHorizonDomain, {
      storePath,
      sourceHorizonId,
      transformId,
      targetDomain,
      outputId: outputId ?? null,
      outputName: outputName ?? null
    });
  }

  throw new Error("Survey horizon-domain conversion is only available in the desktop runtime right now.");
}

export async function previewProcessing(
  request: PreviewProcessingRequest
): Promise<TransportPreviewProcessingResponse> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.previewProcessingBinary, { request });
    return parsePackedPreviewProcessingResponse(payload);
  }

  return postJson<PreviewProcessingResponse>("/api/processing/preview", request as Record<string, unknown>);
}

export async function previewSubvolumeProcessing(
  request: PreviewSubvolumeProcessingRequest
): Promise<TransportPreviewProcessingResponse> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.previewSubvolumeProcessingBinary, { request });
    return parsePackedPreviewProcessingResponse(payload);
  }

  return postJson<PreviewSubvolumeProcessingResponse>("/api/processing/subvolume/preview", request as Record<string, unknown>);
}

export async function previewPostStackNeighborhoodProcessing(
  request: PreviewPostStackNeighborhoodProcessingRequest
): Promise<TransportPreviewProcessingResponse> {
  if (isTauriEnvironment()) {
    const payload = await invokeTauriRaw(desktopBridgeCommands.previewPostStackNeighborhoodProcessingBinary, {
      request
    });
    return parsePackedPreviewProcessingResponse(payload);
  }

  return postJson<PreviewPostStackNeighborhoodProcessingResponse>(
    "/api/processing/post-stack-neighborhood/preview",
    request as Record<string, unknown>
  );
}

export async function emitFrontendDiagnosticsEvent(request: FrontendDiagnosticsEventRequest): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }

  await invokeTauri<void>(desktopBridgeCommands.emitFrontendDiagnosticsEvent, { request });
}

export async function fetchAmplitudeSpectrum(
  request: AmplitudeSpectrumRequest
): Promise<AmplitudeSpectrumResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<AmplitudeSpectrumResponse>(desktopBridgeCommands.amplitudeSpectrum, { request });
  }

  return postJson<AmplitudeSpectrumResponse>("/api/processing/spectrum", request as Record<string, unknown>);
}

export async function runProcessing(
  request: RunProcessingRequest
): Promise<RunProcessingResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<RunProcessingResponse>(desktopBridgeCommands.runProcessing, { request });
  }

  return postJson<RunProcessingResponse>("/api/processing/run", request as Record<string, unknown>);
}

export async function submitTraceLocalProcessingBatch(
  request: SubmitTraceLocalProcessingBatchRequest
): Promise<SubmitTraceLocalProcessingBatchResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SubmitTraceLocalProcessingBatchResponse>(
      desktopBridgeCommands.submitTraceLocalProcessingBatch,
      { request }
    );
  }

  throw new Error("Trace-local processing batches are only available in the desktop runtime.");
}

export async function submitProcessingBatch(
  request: SubmitProcessingBatchRequest
): Promise<SubmitProcessingBatchResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SubmitProcessingBatchResponse>(desktopBridgeCommands.submitProcessingBatch, { request });
  }

  throw new Error("Processing batches are only available in the desktop runtime.");
}

export async function runSubvolumeProcessing(
  request: RunSubvolumeProcessingRequest
): Promise<RunSubvolumeProcessingResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<RunSubvolumeProcessingResponse>(desktopBridgeCommands.runSubvolumeProcessing, { request });
  }

  return postJson<RunSubvolumeProcessingResponse>("/api/processing/subvolume/run", request as Record<string, unknown>);
}

export async function runPostStackNeighborhoodProcessing(
  request: RunPostStackNeighborhoodProcessingRequest
): Promise<RunPostStackNeighborhoodProcessingResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<RunPostStackNeighborhoodProcessingResponse>(
      desktopBridgeCommands.runPostStackNeighborhoodProcessing,
      { request }
    );
  }

  return postJson<RunPostStackNeighborhoodProcessingResponse>(
    "/api/processing/post-stack-neighborhood/run",
    request as Record<string, unknown>
  );
}

export async function defaultProcessingStorePath(
  storePath: string,
  pipeline: TraceLocalProcessingPipeline | RunProcessingRequest["pipeline"]
): Promise<string> {
  if (isTauriEnvironment()) {
    return invokeTauri<string>(desktopBridgeCommands.defaultProcessingStorePath, {
      storePath,
      pipeline
    });
  }

  const normalizedStorePath = storePath.trim();
  const separatorIndex = Math.max(normalizedStorePath.lastIndexOf("/"), normalizedStorePath.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalizedStorePath.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalizedStorePath.slice(separatorIndex + 1) : normalizedStorePath;
  const sourceStem = filename.replace(/\.[^.]+$/, "") || "dataset";
  const namedPipeline = pipeline.name?.trim();
  const pipelineOperationSlug =
    pipeline.steps.map(({ operation }) => operationSlug(operation)).join("-") || "pipeline";
  const pipelineStem = (namedPipeline || pipelineOperationSlug)
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  const timestamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\..+$/, "")
    .replace("T", "-");
  return `${directory}${sourceStem}.${pipelineStem || "pipeline"}.${timestamp}.tbvol`;
}

export async function defaultSubvolumeProcessingStorePath(
  storePath: string,
  pipeline: SubvolumeProcessingPipeline
): Promise<string> {
  if (isTauriEnvironment()) {
    return invokeTauri<string>(desktopBridgeCommands.defaultSubvolumeProcessingStorePath, {
      storePath,
      pipeline
    });
  }

  const normalizedStorePath = storePath.trim();
  const separatorIndex = Math.max(normalizedStorePath.lastIndexOf("/"), normalizedStorePath.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalizedStorePath.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalizedStorePath.slice(separatorIndex + 1) : normalizedStorePath;
  const sourceStem = filename.replace(/\.[^.]+$/, "") || "dataset";
  const namedPipeline = pipeline.name?.trim();
  const prefixLabel =
    pipeline.trace_local_pipeline?.steps.map(({ operation }) => operationSlug(operation)).join("-") ?? "";
  const cropLabel = `crop-il-${pipeline.crop.inline_min}-${pipeline.crop.inline_max}-xl-${pipeline.crop.xline_min}-${pipeline.crop.xline_max}-z-${pipeline.crop.z_min_ms}-${pipeline.crop.z_max_ms}`;
  const pipelineStem = (namedPipeline || [prefixLabel, cropLabel].filter(Boolean).join("-") || "crop-subvolume")
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  const timestamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\..+$/, "")
    .replace("T", "-");
  return `${directory}${sourceStem}.${pipelineStem || "crop-subvolume"}.${timestamp}.tbvol`;
}

export async function defaultPostStackNeighborhoodProcessingStorePath(
  storePath: string,
  pipeline: PostStackNeighborhoodProcessingPipeline
): Promise<string> {
  if (isTauriEnvironment()) {
    return invokeTauri<string>(desktopBridgeCommands.defaultPostStackNeighborhoodProcessingStorePath, {
      storePath,
      pipeline
    });
  }

  const normalizedStorePath = storePath.trim();
  const separatorIndex = Math.max(normalizedStorePath.lastIndexOf("/"), normalizedStorePath.lastIndexOf("\\"));
  const directory = separatorIndex >= 0 ? normalizedStorePath.slice(0, separatorIndex + 1) : "";
  const filename = separatorIndex >= 0 ? normalizedStorePath.slice(separatorIndex + 1) : normalizedStorePath;
  const sourceStem = filename.replace(/\.[^.]+$/, "") || "dataset";
  const namedPipeline = pipeline.name?.trim();
  const prefixLabel =
    pipeline.trace_local_pipeline?.steps.map(({ operation }) => operationSlug(operation)).join("-") ?? "";
  const neighborhoodLabel =
    pipeline.operations
      .map((operation) =>
        "similarity" in operation
          ? `similarity-g${String(operation.similarity.window.gate_ms).replace(".", "_")}-il${operation.similarity.window.inline_stepout}-xl${operation.similarity.window.xline_stepout}`
          : "local_volume_stats" in operation
            ? `local-volume-stats-${operation.local_volume_stats.statistic}-g${String(operation.local_volume_stats.window.gate_ms).replace(".", "_")}-il${operation.local_volume_stats.window.inline_stepout}-xl${operation.local_volume_stats.window.xline_stepout}`
            : "dip" in operation
              ? `dip-${operation.dip.output}-g${String(operation.dip.window.gate_ms).replace(".", "_")}-il${operation.dip.window.inline_stepout}-xl${operation.dip.window.xline_stepout}`
              : "neighborhood"
      )
      .join("-") || "post-stack-neighborhood";
  const pipelineStem = (namedPipeline || [prefixLabel, neighborhoodLabel].filter(Boolean).join("-") || "post-stack-neighborhood")
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  const timestamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\..+$/, "")
    .replace("T", "-");
  return `${directory}${sourceStem}.${pipelineStem}.${timestamp}.tbvol`;
}

export async function getProcessingJob(jobId: string): Promise<GetProcessingJobResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<GetProcessingJobResponse>(desktopBridgeCommands.getProcessingJob, {
      request: { schema_version: SCHEMA_VERSION, job_id: jobId }
    });
  }

  return postJson<GetProcessingJobResponse>("/api/processing/job", {
    schema_version: SCHEMA_VERSION,
    job_id: jobId
  });
}

export async function getProcessingDebugPlan(
  jobId: string
): Promise<GetProcessingDebugPlanResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<GetProcessingDebugPlanResponse>(desktopBridgeCommands.getProcessingDebugPlan, {
      request: { schema_version: SCHEMA_VERSION, job_id: jobId }
    });
  }

  throw new Error("Processing debug plan is only available in the desktop runtime.");
}

export async function getProcessingRuntimeState(
  jobId: string
): Promise<GetProcessingRuntimeStateResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<GetProcessingRuntimeStateResponse>(
        desktopBridgeCommands.getProcessingRuntimeState,
        {
        request: { schema_version: SCHEMA_VERSION, job_id: jobId }
        }
      );
  }

  throw new Error("Processing runtime state is only available in the desktop runtime.");
}

export async function listProcessingRuntimeEvents(
  jobId: string,
  afterSeq: number | null = null
): Promise<ListProcessingRuntimeEventsResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ListProcessingRuntimeEventsResponse>(
        desktopBridgeCommands.listProcessingRuntimeEvents,
        {
        request: { schema_version: SCHEMA_VERSION, job_id: jobId, after_seq: afterSeq }
        }
      );
  }

  throw new Error("Processing runtime events are only available in the desktop runtime.");
}

export async function cancelProcessingJob(jobId: string): Promise<CancelProcessingJobResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<CancelProcessingJobResponse>(desktopBridgeCommands.cancelProcessingJob, {
      request: { schema_version: SCHEMA_VERSION, job_id: jobId }
    });
  }

  return postJson<CancelProcessingJobResponse>("/api/processing/cancel", {
    schema_version: SCHEMA_VERSION,
    job_id: jobId
  });
}

export async function getProcessingBatch(batchId: string): Promise<GetProcessingBatchResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<GetProcessingBatchResponse>(desktopBridgeCommands.getProcessingBatch, {
      request: { schema_version: SCHEMA_VERSION, batch_id: batchId }
    });
  }

  throw new Error("Processing batch status is only available in the desktop runtime.");
}

export async function cancelProcessingBatch(
  batchId: string
): Promise<CancelProcessingBatchResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<CancelProcessingBatchResponse>(desktopBridgeCommands.cancelProcessingBatch, {
      request: { schema_version: SCHEMA_VERSION, batch_id: batchId }
    });
  }

  throw new Error("Processing batch cancellation is only available in the desktop runtime.");
}

export async function listPipelinePresets(): Promise<ListPipelinePresetsResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ListPipelinePresetsResponse>(desktopBridgeCommands.listPipelinePresets, {});
  }

  const response = await fetch("/api/processing/presets");
  return readJson<ListPipelinePresetsResponse>(response);
}

export async function savePipelinePreset(
  preset: ProcessingPreset
): Promise<SavePipelinePresetResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SavePipelinePresetResponse>(desktopBridgeCommands.savePipelinePreset, {
      request: { schema_version: SCHEMA_VERSION, preset }
    });
  }

  return postJson<SavePipelinePresetResponse>("/api/processing/presets/save", {
    schema_version: SCHEMA_VERSION,
    preset
  });
}

export async function deletePipelinePreset(presetId: string): Promise<boolean> {
  if (isTauriEnvironment()) {
    const response = await invokeTauri<{ schema_version: number; deleted: boolean }>(
        desktopBridgeCommands.deletePipelinePreset,
        {
        request: { schema_version: SCHEMA_VERSION, preset_id: presetId }
        }
      );
    return response.deleted;
  }

  const response = await postJson<{ schema_version: number; deleted: boolean }>(
      "/api/processing/presets/delete",
      {
      schema_version: SCHEMA_VERSION,
        preset_id: presetId
      }
    );
  return response.deleted;
}

export async function loadWorkspaceState(): Promise<LoadWorkspaceStateResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<LoadWorkspaceStateResponse>(desktopBridgeCommands.loadWorkspaceState, {});
  }

  return {
    schema_version: SCHEMA_VERSION,
    entries: sortEntries(loadLocalRegistry()),
    session: loadLocalSession()
  };
}

export async function upsertDatasetEntry(
  request: UpsertDatasetEntryRequest
): Promise<UpsertDatasetEntryResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<UpsertDatasetEntryResponse>(desktopBridgeCommands.upsertDatasetEntry, { request });
  }

  const entries = loadLocalRegistry();
  const explicitEntryId = request.entry_id?.trim() || null;
  const trimmedSource = request.source_path?.trim() || null;
  const trimmedPreferredStore = request.preferred_store_path?.trim() || null;
  const trimmedImportedStore = request.imported_store_path?.trim() || null;
  const existingIndex = explicitEntryId
    ? entries.findIndex((entry) => entry.entry_id === explicitEntryId)
    : entries.findIndex(
        (entry) =>
          (trimmedSource && entry.source_path === trimmedSource) ||
          (trimmedImportedStore && entry.imported_store_path === trimmedImportedStore)
      );
  ensureUniqueStoreIdentityLocal(entries, request, existingIndex);
  const now = unixTimestampBigInt();
  const entry: DatasetRegistryEntry =
    existingIndex >= 0
      ? {
          ...entries[existingIndex],
          display_name:
            request.display_name?.trim() ||
            entries[existingIndex].display_name,
          source_path: trimmedSource ?? entries[existingIndex].source_path,
          preferred_store_path: trimmedPreferredStore ?? entries[existingIndex].preferred_store_path,
          imported_store_path: trimmedImportedStore ?? entries[existingIndex].imported_store_path,
          last_dataset: request.dataset ?? entries[existingIndex].last_dataset,
          session_pipelines: request.session_pipelines ?? entries[existingIndex].session_pipelines,
          active_session_pipeline_id:
            request.active_session_pipeline_id ?? entries[existingIndex].active_session_pipeline_id,
          last_imported_at_unix_s:
            request.dataset || trimmedImportedStore ? now : entries[existingIndex].last_imported_at_unix_s,
          updated_at_unix_s: now,
          status: entries[existingIndex].status
        }
      : {
          entry_id: explicitEntryId ?? `dataset-${now.toString()}-${entries.length + 1}`,
          display_name:
            request.display_name?.trim() ||
            request.dataset?.descriptor.label ||
            trimmedSource?.split(/[\\/]/).pop() ||
            trimmedImportedStore?.split(/[\\/]/).pop() ||
            `Dataset ${entries.length + 1}`,
          source_path: trimmedSource,
          preferred_store_path: trimmedPreferredStore,
          imported_store_path: trimmedImportedStore,
          last_dataset: request.dataset ?? null,
          session_pipelines: request.session_pipelines ?? [],
          active_session_pipeline_id: request.active_session_pipeline_id ?? null,
          status: "linked",
          last_opened_at_unix_s: null,
          last_imported_at_unix_s: request.dataset || trimmedImportedStore ? now : null,
          updated_at_unix_s: now
        };
  entry.status = resolveEntryStatus(entry);

  const nextEntries = existingIndex >= 0 ? [...entries] : [...entries, entry];
  if (existingIndex >= 0) {
    nextEntries[existingIndex] = entry;
  }

  let session = loadLocalSession();
  if (request.make_active) {
    session = {
      ...session,
      active_entry_id: entry.entry_id,
      active_store_path: entry.imported_store_path ?? entry.preferred_store_path ?? null
    };
    saveLocalSession(session);
  }

  saveLocalRegistry(sortEntries(nextEntries));

  return {
    schema_version: SCHEMA_VERSION,
    entry,
    session
  };
}

export async function removeDatasetEntry(entryId: string): Promise<RemoveDatasetEntryResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<RemoveDatasetEntryResponse>(desktopBridgeCommands.removeDatasetEntry, {
      request: { schema_version: SCHEMA_VERSION, entry_id: entryId }
    });
  }

  const currentEntries = loadLocalRegistry();
  const entries = currentEntries.filter((entry) => entry.entry_id !== entryId);
  saveLocalRegistry(entries);
  const currentSession = loadLocalSession();
  const session =
    currentSession.active_entry_id === entryId
      ? { ...currentSession, active_entry_id: null, active_store_path: null }
      : currentSession;
  saveLocalSession(session);
  return { schema_version: SCHEMA_VERSION, deleted: entries.length !== currentEntries.length, session };
}

export async function setActiveDatasetEntry(entryId: string): Promise<SetActiveDatasetEntryResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SetActiveDatasetEntryResponse>(desktopBridgeCommands.setActiveDatasetEntry, {
      request: { schema_version: SCHEMA_VERSION, entry_id: entryId }
    });
  }

  const entries = loadLocalRegistry();
  const index = entries.findIndex((entry) => entry.entry_id === entryId);
  if (index < 0) {
    throw new Error(`Unknown dataset entry: ${entryId}`);
  }
  const now = unixTimestampBigInt();
  const entry = {
    ...entries[index],
    last_opened_at_unix_s: now,
    updated_at_unix_s: now
  };
  entries[index] = entry;
  saveLocalRegistry(sortEntries(entries));
  const session = {
    ...loadLocalSession(),
    active_entry_id: entry.entry_id,
    active_store_path: entry.imported_store_path ?? entry.preferred_store_path ?? null
  };
  saveLocalSession(session);
  return {
    schema_version: SCHEMA_VERSION,
    entry,
    session
  };
}

export async function saveWorkspaceSession(
  request: SaveWorkspaceSessionRequest
): Promise<SaveWorkspaceSessionResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SaveWorkspaceSessionResponse>(desktopBridgeCommands.saveWorkspaceSession, { request });
  }

  const session: WorkspaceSession = {
    active_entry_id: request.active_entry_id ?? null,
    active_store_path: request.active_store_path ?? null,
    active_axis: request.active_axis,
    active_index: request.active_index,
    selected_preset_id: request.selected_preset_id ?? null,
    display_coordinate_reference_id: request.display_coordinate_reference_id ?? null,
    active_velocity_model_asset_id: request.active_velocity_model_asset_id ?? null,
    project_root: request.project_root ?? null,
    project_survey_asset_id: request.project_survey_asset_id ?? null,
    project_wellbore_id: request.project_wellbore_id ?? null,
    project_section_tolerance_m: request.project_section_tolerance_m ?? null,
    selected_project_well_time_depth_model_asset_id:
      request.selected_project_well_time_depth_model_asset_id ?? null,
    native_engineering_accepted_store_paths: request.native_engineering_accepted_store_paths ?? []
  };
  saveLocalSession(session);
  return {
    schema_version: SCHEMA_VERSION,
    session
  };
}

export async function loadProjectGeospatialSettings(projectRoot: string): Promise<ProjectGeospatialSettings | null> {
  const normalizedProjectRoot = projectRoot.trim();
  if (!normalizedProjectRoot) {
    return null;
  }

  if (isTauriEnvironment()) {
    const response = await invokeTauri<{ settings: ProjectGeospatialSettings | null }>(
      desktopBridgeCommands.loadProjectGeospatialSettings,
      {
        request: {
          projectRoot: normalizedProjectRoot
        }
      }
    );
    return response.settings ?? null;
  }

  return loadLocalProjectGeospatialSettings(normalizedProjectRoot);
}

export async function searchCoordinateReferences(
  request: {
    query?: string | null;
    limit?: number | null;
    includeDeprecated?: boolean;
    projectedOnly?: boolean;
    includeGeographic?: boolean;
    includeVertical?: boolean;
  } = {}
): Promise<CoordinateReferenceCatalogEntry[]> {
  if (isTauriEnvironment()) {
    const response = await invokeTauri<{ entries: CoordinateReferenceCatalogEntry[] }>(
      desktopBridgeCommands.searchCoordinateReferences,
      {
        request: {
          query: request.query ?? null,
          limit: request.limit ?? null,
          includeDeprecated: request.includeDeprecated === true,
          projectedOnly: request.projectedOnly === true,
          includeGeographic: request.includeGeographic !== false,
          includeVertical: request.includeVertical === true
        }
      }
    );
    return response.entries ?? [];
  }

  const query = request.query?.trim().toUpperCase() ?? "";
  const common: CoordinateReferenceCatalogEntry[] = [
    {
      authority: "EPSG",
      code: "4326",
      authId: "EPSG:4326",
      name: "WGS 84",
      deprecated: false,
      areaName: "World",
      coordinateReferenceType: "geographic_2d"
    },
    {
      authority: "EPSG",
      code: "3857",
      authId: "EPSG:3857",
      name: "WGS 84 / Pseudo-Mercator",
      deprecated: false,
      areaName: "World",
      coordinateReferenceType: "projected"
    },
    {
      authority: "EPSG",
      code: "23031",
      authId: "EPSG:23031",
      name: "ED50 / UTM zone 31N",
      deprecated: false,
      areaName: "Europe - 0°E to 6°E",
      coordinateReferenceType: "projected"
    },
    {
      authority: "EPSG",
      code: "28992",
      authId: "EPSG:28992",
      name: "Amersfoort / RD New",
      deprecated: false,
      areaName: "Netherlands - onshore",
      coordinateReferenceType: "projected"
    }
  ];
  return common
    .filter((entry) =>
      !query
        ? true
        : entry.authId.includes(query) ||
          entry.name.toUpperCase().includes(query) ||
          entry.code.includes(query)
    )
    .slice(0, request.limit ?? 24);
}

export async function resolveCoordinateReference(request: {
  authority?: string | null;
  code?: string | null;
  authId?: string | null;
}): Promise<CoordinateReferenceCatalogEntry> {
  if (isTauriEnvironment()) {
    return invokeTauri<CoordinateReferenceCatalogEntry>(desktopBridgeCommands.resolveCoordinateReference, {
      request
    });
  }

  const authId = request.authId?.trim().toUpperCase() ?? "";
  const entries = await searchCoordinateReferences({ query: authId, limit: 1 });
  if (entries[0]) {
    return entries[0];
  }
  throw new Error(`Unknown coordinate reference '${authId || `${request.authority ?? ""}:${request.code ?? ""}`}'.`);
}

export async function saveProjectGeospatialSettings(
  projectRoot: string,
  displayCoordinateReference: ProjectDisplayCoordinateReference,
  source = "user_selected"
): Promise<ProjectGeospatialSettings> {
  const normalizedProjectRoot = projectRoot.trim();
  if (!normalizedProjectRoot) {
    throw new Error("Project root is required.");
  }

  if (isTauriEnvironment()) {
    return invokeTauri<ProjectGeospatialSettings>(desktopBridgeCommands.saveProjectGeospatialSettings, {
      request: {
        projectRoot: normalizedProjectRoot,
        displayCoordinateReference,
        source
      }
    });
  }

  const now = Math.floor(Date.now() / 1000);
  const existing = loadLocalProjectGeospatialSettings(normalizedProjectRoot);
  const settings: ProjectGeospatialSettings = {
    schemaVersion: 1,
    displayCoordinateReference,
    source,
    createdAtUnixS: existing?.createdAtUnixS ?? now,
    updatedAtUnixS: now
  };
  saveLocalProjectGeospatialSettings(normalizedProjectRoot, settings);
  return settings;
}

export async function setDatasetNativeCoordinateReference(
  request: SetDatasetNativeCoordinateReferenceRequest
): Promise<SetDatasetNativeCoordinateReferenceResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<SetDatasetNativeCoordinateReferenceResponse>(
      desktopBridgeCommands.setDatasetNativeCoordinateReference,
      {
        request: {
          storePath: request.store_path,
          coordinateReferenceId: request.coordinate_reference_id,
          coordinateReferenceName: request.coordinate_reference_name
        }
      }
    );
  }

  const entries = loadLocalRegistry();
  const index = entries.findIndex(
    (entry) =>
      entry.imported_store_path === request.store_path || entry.preferred_store_path === request.store_path
  );
  if (index >= 0 && entries[index]) {
    const entry = entries[index];
    entries[index] = {
      ...entry,
      last_dataset: entry.last_dataset
        ? {
            ...entry.last_dataset,
            descriptor: {
              ...entry.last_dataset.descriptor,
              spatial: entry.last_dataset.descriptor.spatial
                ? {
                    ...entry.last_dataset.descriptor.spatial,
                    coordinate_reference: request.coordinate_reference_id
                      ? {
                          ...(entry.last_dataset.descriptor.spatial.coordinate_reference ?? {
                            id: null,
                            name: null,
                            geodetic_datum: null,
                            unit: null
                          }),
                          id: request.coordinate_reference_id,
                          name:
                            request.coordinate_reference_name ??
                            entry.last_dataset.descriptor.spatial.coordinate_reference?.name ??
                            null
                        }
                      : request.coordinate_reference_name
                        ? {
                            ...(entry.last_dataset.descriptor.spatial.coordinate_reference ?? {
                              id: null,
                              name: null,
                              geodetic_datum: null,
                              unit: null
                            }),
                            id: null,
                            name: request.coordinate_reference_name
                          }
                      : entry.last_dataset.descriptor.coordinate_reference_binding?.detected ??
                        entry.last_dataset.descriptor.spatial.coordinate_reference
                  }
                : entry.last_dataset.descriptor.spatial,
              coordinate_reference_binding: entry.last_dataset.descriptor.coordinate_reference_binding
                ? {
                    ...entry.last_dataset.descriptor.coordinate_reference_binding,
                    effective: request.coordinate_reference_id
                      ? {
                          ...(entry.last_dataset.descriptor.coordinate_reference_binding.effective ??
                            entry.last_dataset.descriptor.coordinate_reference_binding.detected ?? {
                              id: null,
                              name: null,
                              geodetic_datum: null,
                              unit: null
                            }),
                          id: request.coordinate_reference_id,
                          name:
                            request.coordinate_reference_name ??
                            entry.last_dataset.descriptor.coordinate_reference_binding.effective?.name ??
                            entry.last_dataset.descriptor.coordinate_reference_binding.detected?.name ??
                            null
                        }
                      : request.coordinate_reference_name
                        ? {
                            ...(entry.last_dataset.descriptor.coordinate_reference_binding.effective ??
                              entry.last_dataset.descriptor.coordinate_reference_binding.detected ?? {
                                id: null,
                                name: null,
                                geodetic_datum: null,
                                unit: null
                              }),
                            id: null,
                            name: request.coordinate_reference_name
                          }
                      : entry.last_dataset.descriptor.coordinate_reference_binding.detected,
                    source:
                      request.coordinate_reference_id || request.coordinate_reference_name
                        ? "user_override"
                        : "header"
                  }
                : entry.last_dataset.descriptor.coordinate_reference_binding
            }
          }
        : null
    };
    saveLocalRegistry(sortEntries(entries));
  }

  const datasetResponse = await openDataset(request.store_path);
  return {
    schema_version: SCHEMA_VERSION,
    dataset: datasetResponse.dataset
  };
}

export async function resolveSurveyMap(
  request: ResolveSurveyMapRequest
): Promise<ResolveSurveyMapResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ResolveSurveyMapResponse>(desktopBridgeCommands.resolveSurveyMap, { request });
  }

  throw new Error("Survey map resolution is only available in the desktop runtime.");
}

export async function resolveProjectSurveyMap(
  request: ResolveProjectSurveyMapRequest
): Promise<ResolveProjectSurveyMapResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ResolveProjectSurveyMapResponse>(desktopBridgeCommands.resolveProjectSurveyMap, {
      request
    });
  }

  throw new Error("Project survey-map resolution is only available in the desktop runtime.");
}

export async function listProjectWellTimeDepthModels(
  projectRoot: string,
  wellboreId: string
): Promise<ProjectWellTimeDepthModelDescriptor[]> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellTimeDepthModelDescriptor[]>(
      desktopBridgeCommands.listProjectWellTimeDepthModels,
      {
        request: {
          projectRoot,
          wellboreId
        }
      }
    );
  }

  throw new Error("Project well-model listing is only available in the desktop runtime.");
}

export async function listProjectWellTimeDepthInventory(
  projectRoot: string,
  wellboreId: string
): Promise<ProjectWellTimeDepthInventoryResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellTimeDepthInventoryResponse>(
      desktopBridgeCommands.listProjectWellTimeDepthInventory,
      {
        request: {
          projectRoot,
          wellboreId
        }
      }
    );
  }

  throw new Error("Project well-model inventory is only available in the desktop runtime.");
}

export async function setProjectActiveWellTimeDepthModel(
  projectRoot: string,
  wellboreId: string,
  assetId: string | null
): Promise<void> {
  if (isTauriEnvironment()) {
    await invokeTauri<void>(desktopBridgeCommands.setProjectActiveWellTimeDepthModel, {
      request: {
        projectRoot,
        wellboreId,
        assetId
      }
    });
    return;
  }

  throw new Error("Project well-model updates are only available in the desktop runtime.");
}

export async function listProjectWellOverlayInventory(
  projectRoot: string,
  displayCoordinateReferenceId: string | null = null
): Promise<ProjectWellOverlayInventoryResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellOverlayInventoryResponse>(
      desktopBridgeCommands.listProjectWellOverlayInventory,
      {
        request: {
          projectRoot,
          displayCoordinateReferenceId
        }
      }
    );
  }

  throw new Error("Project well-overlay inventory is only available in the desktop runtime.");
}

export async function listProjectSurveyHorizons(
  projectRoot: string,
  assetId: string
): Promise<ImportedHorizonDescriptor[]> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportedHorizonDescriptor[]>(desktopBridgeCommands.listProjectSurveyHorizons, {
      request: {
        projectRoot,
        assetId
      }
    });
  }

  throw new Error("Project survey horizon listing is only available in the desktop runtime.");
}

export async function listProjectWellMarkerResidualInventory(
  projectRoot: string,
  wellboreId: string
): Promise<ProjectWellMarkerResidualInventoryResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellMarkerResidualInventoryResponse>(
      desktopBridgeCommands.listProjectWellMarkerResidualInventory,
      {
        request: {
          projectRoot,
          wellboreId
        }
      }
    );
  }

  throw new Error("Project well marker/residual inventory is only available in the desktop runtime.");
}

export async function scanVendorProject(
  request: VendorProjectScanRequest
): Promise<VendorProjectScanResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<VendorProjectScanResponse>(desktopBridgeCommands.scanVendorProject, { request });
  }

  throw new Error("Vendor project scanning is only available in the desktop runtime.");
}

export async function planVendorProjectImport(
  request: VendorProjectPlanRequest
): Promise<VendorProjectPlanResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<VendorProjectPlanResponse>(desktopBridgeCommands.planVendorProjectImport, { request });
  }

  throw new Error("Vendor project planning is only available in the desktop runtime.");
}

export async function commitVendorProjectImport(
  request: VendorProjectCommitRequest
): Promise<VendorProjectCommitResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<VendorProjectCommitResponse>(desktopBridgeCommands.commitVendorProjectImport, {
      request
    });
  }

  throw new Error("Vendor project commit is only available in the desktop runtime.");
}

export async function importProjectWellTimeDepthAsset(
  request: ImportProjectWellTimeDepthAssetRequest
): Promise<ImportProjectWellTimeDepthModelResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportProjectWellTimeDepthModelResponse>(
      desktopBridgeCommands.importProjectWellTimeDepthAsset,
      { request }
    );
  }

  throw new Error("Project well-model import is only available in the desktop runtime.");
}

export async function previewProjectWellTimeDepthAsset(
  request: PreviewProjectWellTimeDepthAssetRequest
): Promise<ProjectWellTimeDepthAssetPreview> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellTimeDepthAssetPreview>(
      desktopBridgeCommands.previewProjectWellTimeDepthAsset,
      { request }
    );
  }

  throw new Error("Project well-model preview is only available in the desktop runtime.");
}

export async function previewProjectWellTimeDepthImport(
  request: PreviewProjectWellTimeDepthImportRequest
): Promise<ProjectWellTimeDepthImportPreview> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellTimeDepthImportPreview>(
      desktopBridgeCommands.previewProjectWellTimeDepthImport,
      { request }
    );
  }

  throw new Error("Project well-model preview is only available in the desktop runtime.");
}

export async function previewProjectWellSourceImport(
  request: PreviewProjectWellSourceImportRequest
): Promise<ProjectWellSourceImportPreview> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellSourceImportPreview>(desktopBridgeCommands.previewProjectWellSources, {
      request
    });
  }

  throw new Error("Project well-source preview is only available in the desktop runtime.");
}

export async function previewProjectWellImport(
  request: PreviewProjectWellImportRequest
): Promise<ProjectWellFolderImportPreview> {
  const response = await previewProjectWellSourceImport({
    sourceRootPath: request.folderPath,
    sourcePaths: request.sourcePaths
  });
  return response.parsed;
}

function legacyWellSourceImportDraft(request: {
  binding?: ProjectAssetBindingInput;
  wellMetadata?: ProjectWellMetadata | null;
  wellboreMetadata?: ProjectWellboreMetadata | null;
  sourceCoordinateReference?: WellSourceCoordinateReferenceSelection;
  importLogs?: boolean;
  selectedLogSourcePaths?: string[] | null;
  importTopsMarkers?: boolean;
  importTrajectory?: boolean;
  topsDepthReference?: string | null;
  topsRows?: ProjectWellSourceTopDraftRow[] | null;
  trajectoryRows?: ProjectWellSourceTrajectoryDraftRow[] | null;
  asciiLogImports?: ProjectWellSourceAsciiLogImportRequest[] | null;
}): ProjectWellSourceImportCanonicalDraft {
  if (!request.binding) {
    throw new Error("Project well-source import requires either a canonical draft or binding data.");
  }
  if (!request.sourceCoordinateReference) {
    throw new Error(
      "Project well-source import requires either a canonical draft or source CRS selection data."
    );
  }

  return {
    binding: request.binding,
    sourceCoordinateReference: request.sourceCoordinateReference,
    wellMetadata: request.wellMetadata ?? null,
    wellboreMetadata: request.wellboreMetadata ?? null,
    importPlan: {
      selectedLogSourcePaths: request.importLogs ? (request.selectedLogSourcePaths ?? null) : null,
      asciiLogImports: request.importLogs ? (request.asciiLogImports ?? null) : null,
      topsMarkers: request.importTopsMarkers
        ? {
            depthReference: request.topsDepthReference ?? null,
            rows: request.topsRows ?? []
          }
        : null,
      trajectory: request.importTrajectory
        ? {
            enabled: true,
            rows: request.trajectoryRows ?? null
          }
        : null
    }
  };
}

export async function commitProjectWellSourceImport(
  request: CommitProjectWellSourceImportRequest
): Promise<ProjectWellSourceImportCommitResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellSourceImportCommitResponse>(
      desktopBridgeCommands.commitProjectWellSources,
      {
        request: {
          ...request,
          draft: request.draft ?? legacyWellSourceImportDraft(request)
        }
      }
    );
  }

  throw new Error("Project well-source import is only available in the desktop runtime.");
}

export async function commitProjectWellTimeDepthImport(
  request: CommitProjectWellTimeDepthImportRequest
): Promise<ImportProjectWellTimeDepthModelResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportProjectWellTimeDepthModelResponse>(
      desktopBridgeCommands.commitProjectWellTimeDepthImport,
      { request }
    );
  }

  throw new Error("Project well-model import is only available in the desktop runtime.");
}

export async function commitProjectWellImport(
  request: CommitProjectWellImportRequest
): Promise<ProjectWellFolderImportCommitResponse> {
  return commitProjectWellSourceImport({
    projectRoot: request.projectRoot,
    sourceRootPath: request.folderPath,
    sourcePaths: request.sourcePaths,
    draft: request.draft,
    binding: request.binding,
    wellMetadata: request.wellMetadata,
    wellboreMetadata: request.wellboreMetadata,
    sourceCoordinateReference: request.sourceCoordinateReference,
    importLogs: request.importLogs,
    selectedLogSourcePaths: request.selectedLogSourcePaths,
    importTopsMarkers: request.importTopsMarkers,
    importTrajectory: request.importTrajectory,
    topsDepthReference: request.topsDepthReference,
    topsRows: request.topsRows,
    asciiLogImports: request.asciiLogImports
  });
}

export async function importProjectWellTimeDepthModel(
  request: ImportProjectWellTimeDepthModelRequest
): Promise<ImportProjectWellTimeDepthModelResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportProjectWellTimeDepthModelResponse>(
      desktopBridgeCommands.importProjectWellTimeDepthModel,
      { request }
    );
  }

  throw new Error("Project well-model import is only available in the desktop runtime.");
}

export async function readProjectWellTimeDepthModel(
  projectRoot: string,
  assetId: string
): Promise<WellTimeDepthModel1D> {
  if (isTauriEnvironment()) {
    return invokeTauri<WellTimeDepthModel1D>(desktopBridgeCommands.readProjectWellTimeDepthModel, {
      request: {
        projectRoot,
        assetId
      }
    });
  }

  throw new Error("Project well-model reading is only available in the desktop runtime.");
}

export async function compileProjectWellTimeDepthAuthoredModel(
  request: CompileProjectWellTimeDepthAuthoredModelRequest
): Promise<ImportProjectWellTimeDepthModelResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ImportProjectWellTimeDepthModelResponse>(
      desktopBridgeCommands.compileProjectWellTimeDepthAuthoredModel,
      { request }
    );
  }

  throw new Error("Project well-model compilation is only available in the desktop runtime.");
}

export async function analyzeProjectWellTie(
  request: AnalyzeProjectWellTieRequest
): Promise<ProjectWellTieAnalysisResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ProjectWellTieAnalysisResponse>(desktopBridgeCommands.analyzeProjectWellTie, {
      request
    });
  }

  throw new Error("Project well-tie analysis is only available in the desktop runtime.");
}

export async function acceptProjectWellTie(
  request: AcceptProjectWellTieRequest
): Promise<AcceptProjectWellTieResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<AcceptProjectWellTieResponse>(desktopBridgeCommands.acceptProjectWellTie, {
      request
    });
  }

  throw new Error("Project well-tie acceptance is only available in the desktop runtime.");
}

export async function computeProjectWellMarkerResidual(
  request: ComputeProjectWellMarkerResidualRequest
): Promise<ComputeProjectWellMarkerResidualResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ComputeProjectWellMarkerResidualResponse>(
      desktopBridgeCommands.computeProjectWellMarkerResidual,
      { request }
    );
  }

  throw new Error("Project residual computation is only available in the desktop runtime.");
}

export async function resolveProjectSectionWellOverlays(
  request: SectionWellOverlayRequestDto
): Promise<ResolveSectionWellOverlaysResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<ResolveSectionWellOverlaysResponse>(
      desktopBridgeCommands.resolveProjectSectionWellOverlays,
      { request }
    );
  }

  throw new Error("Project section-well overlay resolution is only available in the desktop runtime.");
}

export async function getDiagnosticsStatus(): Promise<DiagnosticsStatus | null> {
  if (!isTauriEnvironment()) {
    return null;
  }

  return invokeTauri<DiagnosticsStatus>(desktopBridgeCommands.getDiagnosticsStatus, {});
}

export async function setDiagnosticsVerbosity(enabled: boolean): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }

  await invokeTauri<void>(desktopBridgeCommands.setDiagnosticsVerbosity, { enabled });
}

export async function runSectionBrowsingBenchmark(
  request: RunSectionBrowsingBenchmarkRequest
): Promise<RunSectionBrowsingBenchmarkResponse> {
  if (isTauriEnvironment()) {
    return invokeTauri<RunSectionBrowsingBenchmarkResponse>(
      desktopBridgeCommands.runSectionBrowsingBenchmark,
      { request }
    );
  }

  throw new Error("Section browsing benchmark is only available in the desktop runtime.");
}

export async function listenToDiagnosticsEvents(
  listener: (event: DiagnosticsEvent) => void
): Promise<() => void> {
  if (!isTauriEnvironment()) {
    return () => {};
  }

  const { listen } = await import("@tauri-apps/api/event");
  return listen<DiagnosticsEvent>("diagnostics:event", (event) => {
    listener(event.payload);
  });
}
