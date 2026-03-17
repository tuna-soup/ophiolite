import { invoke } from "@tauri-apps/api/core";

export type CommandErrorDto = {
  dto_contract_version?: string;
  kind: string;
  message: string;
  session_id?: string | null;
  validation?: ValidationReportDto | null;
  save_conflict?: unknown;
};

export type CommandResponse<T> = { Ok: T } | { Err: CommandErrorDto };

export type DiagnosticIssueDto = {
  code: string;
  message: string;
  severity: string;
};

export type ValidationReportDto = {
  dto_contract_version?: string;
  kind: string;
  valid: boolean;
  errors: string[];
  issues: DiagnosticIssueDto[];
};

export type AssetSummaryDto = {
  dto_contract_version?: string;
  summary?: {
    well_name?: string;
    curve_count?: number;
    row_count?: number;
    issue_count?: number;
  };
  index?: {
    name?: string;
    canonical_name?: string;
  };
};

export type MetadataDto = {
  dto_contract_version?: string;
  metadata: {
    version?: Record<string, unknown>;
    well: Record<string, unknown>;
    parameters?: Record<string, unknown>;
    curves?: Array<Record<string, unknown>>;
    other?: unknown;
  };
  extra_sections?: Record<string, string>;
  issues?: unknown[];
};

export type SessionSummaryDto = {
  dto_contract_version?: string;
  package_id?: string;
  session_id: string;
  revision: string;
  root: string;
  dirty: { has_unsaved_changes: boolean };
  summary?: AssetSummaryDto;
};

export type SessionMetadataDto = {
  dto_contract_version?: string;
  session: {
    session_id: string;
    root: string;
    revision: string;
  };
  metadata: MetadataDto;
};

export type CurveCatalogEntryDto = {
  curve_id?: string;
  name?: string;
  mnemonic?: string;
  canonical_name?: string;
  original_mnemonic?: string;
  unit?: string | null;
  description?: string | null;
  is_index?: boolean;
  row_count?: number;
  storage_kind?: string;
};

export type CurveCatalogDto = {
  dto_contract_version?: string;
  session: {
    session_id: string;
    root: string;
    revision: string;
  };
  curves: CurveCatalogEntryDto[];
};

export type CurveWindowRequest = {
  curve_names: string[];
  start_row: number;
  row_count: number;
};

export type DepthWindowRequest = {
  curve_names: string[];
  depth_min: number;
  depth_max: number;
};

export type CurveWindowDto = {
  dto_contract_version?: string;
  start_row: number;
  row_count: number;
  columns: Array<{
    curve_id?: string;
    name: string;
    canonical_name?: string;
    values: unknown[];
  }>;
};

export type SessionWindowDto = {
  dto_contract_version?: string;
  session: {
    session_id: string;
    root: string;
    revision: string;
  };
  window: CurveWindowDto;
};

export type DirtyStateDto = {
  dto_contract_version?: string;
  session_id: string;
  has_unsaved_changes: boolean;
};

export type SavePackageResultDto = {
  dto_contract_version?: string;
  session_id: string;
  revision: string;
  root: string;
  dirty_cleared: boolean;
  overwritten: boolean;
};

export type PackageFilesViewDto = {
  root: string;
  has_package_files: boolean;
  metadata_path: string;
  metadata_json?: string | null;
  parquet_path: string;
  parquet_exists: boolean;
  parquet_size_bytes?: number | null;
  row_count?: number | null;
  curve_count: number;
  index_name?: string | null;
  columns: Array<{
    name: string;
    canonical_name?: string;
    original_mnemonic?: string;
    unit?: string;
    storage_kind: string;
    row_count?: number;
    nullable?: boolean;
    is_index?: boolean;
  }>;
};

export type WellIdentifierSet = {
  primary_name?: string | null;
  uwi?: string | null;
  api?: string | null;
  operator_aliases?: string[];
};

export type ProjectSummaryDto = {
  root: string;
  catalog_path: string;
  manifest_path: string;
  well_count: number;
  asset_count: number;
};

export type WellRecord = {
  id: string;
  name: string;
  identifiers: WellIdentifierSet;
};

export type WellboreRecord = {
  id: string;
  well_id: string;
  name: string;
  identifiers: WellIdentifierSet;
};

export type AssetCollectionRecord = {
  id: string;
  wellbore_id: string;
  asset_kind: string;
  name: string;
  logical_asset_id: string;
  status: string;
};

export type AssetExtent = {
  index_kind?: string | null;
  start?: number | null;
  stop?: number | null;
  row_count?: number | null;
};

export type AssetManifest = {
  asset_kind: string;
  asset_schema_version: string;
  logical_asset_id: string;
  storage_asset_id: string;
  well_id: string;
  wellbore_id: string;
  asset_collection_id: string;
  extents: AssetExtent;
  reference_metadata?: {
    depth_reference?: string;
    vertical_datum?: string | null;
    unit_system?: {
      depth_unit?: string | null;
      coordinate_unit?: string | null;
      pressure_unit?: string | null;
    };
  };
  source_artifacts?: Array<{
    source_path: string;
    original_filename: string;
    source_fingerprint: string;
  }>;
};

export type AssetRecord = {
  id: string;
  logical_asset_id: string;
  collection_id: string;
  well_id: string;
  wellbore_id: string;
  asset_kind: string;
  status: string;
  package_path: string;
  manifest: AssetManifest;
};

export type AssetBindingInput = {
  well_name: string;
  wellbore_name: string;
  uwi?: string | null;
  api?: string | null;
  operator_aliases: string[];
};

export type ProjectAssetImportResult = {
  resolution: {
    status: string;
    well_id: string;
    wellbore_id: string;
    created_well: boolean;
    created_wellbore: boolean;
  };
  collection: AssetCollectionRecord;
  asset: AssetRecord;
};

export type TrajectoryRow = {
  measured_depth: number;
  true_vertical_depth?: number | null;
  azimuth_deg?: number | null;
  inclination_deg?: number | null;
  northing_offset?: number | null;
  easting_offset?: number | null;
};

export type TopRow = {
  name: string;
  top_depth: number;
  base_depth?: number | null;
  source?: string | null;
  depth_reference?: string | null;
};

export type PressureObservationRow = {
  measured_depth?: number | null;
  pressure: number;
  phase?: string | null;
  test_kind?: string | null;
  timestamp?: string | null;
};

export type DrillingObservationRow = {
  measured_depth?: number | null;
  event_kind: string;
  value?: number | null;
  unit?: string | null;
  timestamp?: string | null;
  comment?: string | null;
};

export async function invokeCommand<T>(
  command: string,
  request: unknown
): Promise<CommandResponse<T>> {
  return invoke(command, { request });
}

async function unwrap<T>(command: string, request: unknown): Promise<T> {
  const response = await invokeCommand<T>(command, request);
  if ("Err" in response) {
    throw response.Err;
  }
  return response.Ok;
}

export const api = {
  createProject(path: string) {
    return unwrap<ProjectSummaryDto>("create_project", { path });
  },
  openProject(path: string) {
    return unwrap<ProjectSummaryDto>("open_project", { path });
  },
  listProjectWells(projectRoot: string) {
    return unwrap<WellRecord[]>("list_project_wells", { path: projectRoot });
  },
  listProjectWellbores(projectRoot: string, wellId: string) {
    return unwrap<WellboreRecord[]>("list_project_wellbores", {
      project_root: projectRoot,
      well_id: wellId
    });
  },
  listProjectAssetCollections(projectRoot: string, wellboreId: string) {
    return unwrap<AssetCollectionRecord[]>("list_project_asset_collections", {
      project_root: projectRoot,
      wellbore_id: wellboreId
    });
  },
  listProjectAssets(projectRoot: string, wellboreId: string, assetKind?: string | null) {
    return unwrap<AssetRecord[]>("list_project_assets", {
      project_root: projectRoot,
      wellbore_id: wellboreId,
      asset_kind: assetKind ? assetKind : null
    });
  },
  importProjectLas(projectRoot: string, lasPath: string, collectionName?: string | null) {
    return unwrap<ProjectAssetImportResult>("import_project_las", {
      project_root: projectRoot,
      las_path: lasPath,
      collection_name: collectionName ?? null
    });
  },
  importProjectTrajectoryCsv(
    projectRoot: string,
    csvPath: string,
    binding: AssetBindingInput,
    collectionName?: string | null
  ) {
    return unwrap<ProjectAssetImportResult>("import_project_trajectory_csv", {
      project_root: projectRoot,
      csv_path: csvPath,
      binding,
      collection_name: collectionName ?? null
    });
  },
  importProjectTopsCsv(
    projectRoot: string,
    csvPath: string,
    binding: AssetBindingInput,
    collectionName?: string | null
  ) {
    return unwrap<ProjectAssetImportResult>("import_project_tops_csv", {
      project_root: projectRoot,
      csv_path: csvPath,
      binding,
      collection_name: collectionName ?? null
    });
  },
  importProjectPressureCsv(
    projectRoot: string,
    csvPath: string,
    binding: AssetBindingInput,
    collectionName?: string | null
  ) {
    return unwrap<ProjectAssetImportResult>("import_project_pressure_csv", {
      project_root: projectRoot,
      csv_path: csvPath,
      binding,
      collection_name: collectionName ?? null
    });
  },
  importProjectDrillingCsv(
    projectRoot: string,
    csvPath: string,
    binding: AssetBindingInput,
    collectionName?: string | null
  ) {
    return unwrap<ProjectAssetImportResult>("import_project_drilling_csv", {
      project_root: projectRoot,
      csv_path: csvPath,
      binding,
      collection_name: collectionName ?? null
    });
  },
  projectAssetsCoveringDepthRange(projectRoot: string, wellboreId: string, depthMin: number, depthMax: number) {
    return unwrap<AssetRecord[]>("project_assets_covering_depth_range", {
      project_root: projectRoot,
      wellbore_id: wellboreId,
      depth_min: depthMin,
      depth_max: depthMax
    });
  },
  readProjectTrajectoryRows(projectRoot: string, assetId: string, depthMin?: number | null, depthMax?: number | null) {
    return unwrap<TrajectoryRow[]>("read_project_trajectory_rows", {
      project_root: projectRoot,
      asset_id: assetId,
      depth_min: depthMin ?? null,
      depth_max: depthMax ?? null
    });
  },
  readProjectTops(projectRoot: string, assetId: string) {
    return unwrap<TopRow[]>("read_project_tops", {
      project_root: projectRoot,
      asset_id: assetId
    });
  },
  readProjectPressureObservations(projectRoot: string, assetId: string, depthMin?: number | null, depthMax?: number | null) {
    return unwrap<PressureObservationRow[]>("read_project_pressure_observations", {
      project_root: projectRoot,
      asset_id: assetId,
      depth_min: depthMin ?? null,
      depth_max: depthMax ?? null
    });
  },
  readProjectDrillingObservations(projectRoot: string, assetId: string, depthMin?: number | null, depthMax?: number | null) {
    return unwrap<DrillingObservationRow[]>("read_project_drilling_observations", {
      project_root: projectRoot,
      asset_id: assetId,
      depth_min: depthMin ?? null,
      depth_max: depthMax ?? null
    });
  },
  inspectPackageSummary(path: string) {
    return unwrap<AssetSummaryDto>("inspect_package_summary", { path });
  },
  inspectPackageMetadata(path: string) {
    return unwrap<MetadataDto>("inspect_package_metadata", { path });
  },
  validatePackage(path: string) {
    return unwrap<ValidationReportDto>("validate_package", { path });
  },
  openPackageSession(path: string) {
    return unwrap<SessionSummaryDto>("open_package_session", { path });
  },
  sessionSummary(sessionId: string) {
    return unwrap<SessionSummaryDto>("session_summary", { session_id: sessionId });
  },
  sessionMetadata(sessionId: string) {
    return unwrap<SessionMetadataDto>("session_metadata", { session_id: sessionId });
  },
  sessionCurveCatalog(sessionId: string) {
    return unwrap<CurveCatalogDto>("session_curve_catalog", { session_id: sessionId });
  },
  readCurveWindow(sessionId: string, window: CurveWindowRequest) {
    return unwrap<SessionWindowDto>("read_curve_window", { session_id: sessionId, window });
  },
  readDepthWindow(sessionId: string, window: DepthWindowRequest) {
    return unwrap<SessionWindowDto>("read_depth_window", { session_id: sessionId, window });
  },
  dirtyState(sessionId: string) {
    return unwrap<DirtyStateDto>("dirty_state", { session_id: sessionId });
  },
  closeSession(sessionId: string) {
    return unwrap("close_session", { session_id: sessionId });
  },
  applyMetadataEdit(sessionId: string, update: unknown) {
    return unwrap<SessionSummaryDto>("apply_metadata_edit", { session_id: sessionId, update });
  },
  applyCurveEdit(sessionId: string, edit: unknown) {
    return unwrap<SessionSummaryDto>("apply_curve_edit", { session_id: sessionId, edit });
  },
  saveSession(sessionId: string) {
    return unwrap<SavePackageResultDto>("save_session", { session_id: sessionId });
  },
  saveSessionAs(sessionId: string, outputDir: string) {
    return unwrap<SavePackageResultDto>("save_session_as", {
      session_id: sessionId,
      output_dir: outputDir
    });
  },
  inspectLasSummary(path: string) {
    return unwrap<AssetSummaryDto>("inspect_las_summary", { path });
  },
  inspectLasMetadata(path: string) {
    return unwrap<MetadataDto>("inspect_las_metadata", { path });
  },
  inspectLasCurveCatalog(path: string) {
    return unwrap<CurveCatalogEntryDto[]>("inspect_las_curve_catalog", { path });
  },
  inspectLasWindow(path: string, window: CurveWindowRequest) {
    return unwrap<CurveWindowDto>("inspect_las_window", { path, window });
  },
  validateLas(path: string) {
    return unwrap<ValidationReportDto>("validate_las", { path });
  },
  importLasIntoWorkspace(packageRoot: string, lasPath: string, sessionId?: string | null) {
    return unwrap<SessionSummaryDto>("import_las_into_workspace", {
      package_root: packageRoot,
      las_path: lasPath,
      session_id: sessionId ?? null
    });
  },
  readPackageFiles(path: string) {
    return unwrap<PackageFilesViewDto>("read_package_files", { path });
  }
};
