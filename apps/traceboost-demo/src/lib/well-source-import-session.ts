import type {
  ProjectWellFolderImportPreview,
  ProjectWellSourceAsciiLogImportRequest,
  ProjectWellSourceImportCanonicalDraft,
  ProjectWellSourceImportPreview,
  ProjectWellSourceTrajectoryDraftRow,
  WellSourceCoordinateReferenceSelectionMode
} from "./bridge";

export interface WellSourceImportDraft {
  wellName: string;
  wellboreName: string;
  uwi: string;
  api: string;
  importLogs: boolean;
  importTopsMarkers: boolean;
  importTrajectory: boolean;
  topsDepthReference: string;
  fieldName: string;
  blockName: string;
  country: string;
  provinceState: string;
  locationText: string;
  interestType: string;
  sourceCrsMode: WellSourceCoordinateReferenceSelectionMode;
  detectedCandidateId: string;
  manualSourceCrsId: string;
  manualSourceCrsName: string;
  surfaceX: string;
  surfaceY: string;
  wellboreStatus: string;
  wellborePurpose: string;
  trajectoryType: string;
  parentWellboreId: string;
  serviceCompanyName: string;
  wellboreLocationText: string;
}

export interface WellSourceImportEditableTopRow {
  name: string;
  topDepth: string;
  baseDepth: string;
  anomaly: string;
  quality: string;
  note: string;
}

export interface WellSourceImportEditableAsciiCurve {
  sourceColumn: string;
  enabled: boolean;
  mnemonic: string;
  unit: string;
}

export interface WellSourceImportEditableAsciiLogImport {
  sourcePath: string;
  enabled: boolean;
  depthColumn: string;
  nullValue: string;
  curves: WellSourceImportEditableAsciiCurve[];
}

export interface WellSourceImportEditableTrajectoryRow {
  measuredDepth: string;
  inclinationDeg: string;
  azimuthDeg: string;
  trueVerticalDepth: string;
  xOffset: string;
  yOffset: string;
}

export interface StoredAsciiCurveMemory {
  enabled?: boolean;
  mnemonic?: string;
  unit?: string;
}

export interface StoredAsciiLogMemory {
  enabled?: boolean;
  depthColumn?: string;
  nullValue?: string;
  curves?: Record<string, StoredAsciiCurveMemory>;
}

export interface WellImportMemory {
  asciiLogs?: Record<string, StoredAsciiLogMemory>;
}

export interface WellSourceImportSessionState {
  preview: ProjectWellFolderImportPreview;
  suggestedDraft: ProjectWellSourceImportCanonicalDraft;
  draft: WellSourceImportDraft;
  topsRows: WellSourceImportEditableTopRow[];
  selectedLogSourcePaths: string[];
  asciiLogDrafts: WellSourceImportEditableAsciiLogImport[];
  trajectoryRows: WellSourceImportEditableTrajectoryRow[];
}

export function emptyWellSourceImportDraft(): WellSourceImportDraft {
  return {
    wellName: "",
    wellboreName: "",
    uwi: "",
    api: "",
    importLogs: true,
    importTopsMarkers: true,
    importTrajectory: false,
    topsDepthReference: "md",
    fieldName: "",
    blockName: "",
    country: "",
    provinceState: "",
    locationText: "",
    interestType: "",
    sourceCrsMode: "unresolved",
    detectedCandidateId: "",
    manualSourceCrsId: "",
    manualSourceCrsName: "",
    surfaceX: "",
    surfaceY: "",
    wellboreStatus: "",
    wellborePurpose: "",
    trajectoryType: "",
    parentWellboreId: "",
    serviceCompanyName: "",
    wellboreLocationText: ""
  };
}

export function draftFromSuggestedDraft(
  nextPreview: ProjectWellFolderImportPreview,
  nextSuggestedDraft: ProjectWellSourceImportCanonicalDraft
): WellSourceImportDraft {
  const wellMetadata = nextSuggestedDraft.wellMetadata ?? {};
  const wellboreMetadata = nextSuggestedDraft.wellboreMetadata ?? {};
  const surfaceLocation = wellMetadata.surface_location;
  const selectedLogSourcePaths = new Set(nextSuggestedDraft.importPlan.selectedLogSourcePaths ?? []);
  const hasSelectedAsciiImports = (nextSuggestedDraft.importPlan.asciiLogImports?.length ?? 0) > 0;
  const topsMarkersDraft = nextSuggestedDraft.importPlan.topsMarkers;
  const trajectoryDraft = nextSuggestedDraft.importPlan.trajectory;
  const suggestedSourceCrsMode =
    nextSuggestedDraft.sourceCoordinateReference.mode === "detected"
      ? "unresolved"
      : nextSuggestedDraft.sourceCoordinateReference.mode;
  return {
    wellName: nextSuggestedDraft.binding.well_name,
    wellboreName: nextSuggestedDraft.binding.wellbore_name,
    uwi: nextSuggestedDraft.binding.uwi ?? "",
    api: nextSuggestedDraft.binding.api ?? "",
    importLogs: selectedLogSourcePaths.size > 0 || hasSelectedAsciiImports,
    importTopsMarkers: !!topsMarkersDraft && nextPreview.topsMarkers.commitEnabled,
    importTrajectory:
      !!trajectoryDraft && trajectoryDraft.enabled && nextPreview.trajectory.commitEnabled,
    topsDepthReference:
      topsMarkersDraft?.depthReference ?? nextPreview.topsMarkers.preferredDepthReference ?? "md",
    fieldName: wellMetadata.field_name ?? "",
    blockName: wellMetadata.block_name ?? "",
    country: wellMetadata.country ?? "",
    provinceState: wellMetadata.province_state ?? "",
    locationText: wellMetadata.location_text ?? "",
    interestType: wellMetadata.interest_type ?? "",
    sourceCrsMode: suggestedSourceCrsMode,
    detectedCandidateId: nextSuggestedDraft.sourceCoordinateReference.candidateId ?? "",
    manualSourceCrsId: surfaceLocation?.coordinate_reference?.id ?? "",
    manualSourceCrsName: surfaceLocation?.coordinate_reference?.name ?? "",
    surfaceX: surfaceLocation ? String(surfaceLocation.point.x) : "",
    surfaceY: surfaceLocation ? String(surfaceLocation.point.y) : "",
    wellboreStatus: wellboreMetadata.status ?? "",
    wellborePurpose: wellboreMetadata.purpose ?? "",
    trajectoryType: wellboreMetadata.trajectory_type ?? "",
    parentWellboreId: wellboreMetadata.parent_wellbore_id ?? "",
    serviceCompanyName: wellboreMetadata.service_company_name ?? "",
    wellboreLocationText: wellboreMetadata.location_text ?? ""
  };
}

export function editableTopsRows(
  rows: ProjectWellFolderImportPreview["topsMarkers"]["rows"]
): WellSourceImportEditableTopRow[] {
  return rows.map((row) => ({
    name: row.name ?? "",
    topDepth: row.topDepth === null || row.topDepth === undefined ? "" : String(row.topDepth),
    baseDepth: row.baseDepth === null || row.baseDepth === undefined ? "" : String(row.baseDepth),
    anomaly: row.anomaly ?? "",
    quality: row.quality ?? "",
    note: row.note ?? ""
  }));
}

export function defaultSelectedLogSourcePaths(
  nextSuggestedDraft: ProjectWellSourceImportCanonicalDraft
): string[] {
  return nextSuggestedDraft.importPlan.selectedLogSourcePaths ?? [];
}

export function editableTrajectoryRows(
  nextPreview: ProjectWellFolderImportPreview,
  nextSuggestedDraft: ProjectWellSourceImportCanonicalDraft
): WellSourceImportEditableTrajectoryRow[] {
  const seedRows =
    nextSuggestedDraft.importPlan.trajectory?.rows && nextSuggestedDraft.importPlan.trajectory.rows.length > 0
      ? nextSuggestedDraft.importPlan.trajectory.rows
      : nextPreview.trajectory.draftRows.length > 0
        ? nextPreview.trajectory.draftRows
        : nextPreview.trajectory.sampleRows;
  const rows = seedRows.map((row) => ({
    measuredDepth:
      row.measuredDepth === null || row.measuredDepth === undefined ? "" : String(row.measuredDepth),
    inclinationDeg:
      row.inclinationDeg === null || row.inclinationDeg === undefined ? "" : String(row.inclinationDeg),
    azimuthDeg: row.azimuthDeg === null || row.azimuthDeg === undefined ? "" : String(row.azimuthDeg),
    trueVerticalDepth:
      row.trueVerticalDepth === null || row.trueVerticalDepth === undefined
        ? ""
        : String(row.trueVerticalDepth),
    xOffset: row.xOffset === null || row.xOffset === undefined ? "" : String(row.xOffset),
    yOffset: row.yOffset === null || row.yOffset === undefined ? "" : String(row.yOffset)
  }));
  if (nextPreview.trajectory.sourcePath && rows.length < 2) {
    while (rows.length < 2) {
      rows.push(emptyTrajectoryRow());
    }
  }
  return rows;
}

export function editableAsciiLogDrafts(
  nextPreview: ProjectWellFolderImportPreview,
  nextSuggestedDraft: ProjectWellSourceImportCanonicalDraft
): WellSourceImportEditableAsciiLogImport[] {
  const asciiImportBySourcePath = new Map(
    (nextSuggestedDraft.importPlan.asciiLogImports ?? []).map((entry) => [entry.sourcePath, entry])
  );
  return nextPreview.asciiLogs.files.map((file) => ({
    sourcePath: file.sourcePath,
    enabled: asciiImportBySourcePath.has(file.sourcePath),
    depthColumn:
      asciiImportBySourcePath.get(file.sourcePath)?.depthColumn ??
      file.defaultDepthColumn ??
      file.columns[0]?.name ??
      "",
    nullValue: String(asciiImportBySourcePath.get(file.sourcePath)?.nullValue ?? -999.25),
    curves: file.columns
      .filter((column) => column.name !== file.defaultDepthColumn)
      .map((column) => ({
        sourceColumn: column.name,
        enabled:
          asciiImportBySourcePath
            .get(file.sourcePath)
            ?.valueColumns.some((valueColumn) => valueColumn.sourceColumn === column.name) ??
          file.defaultValueColumns.includes(column.name),
        mnemonic:
          asciiImportBySourcePath
            .get(file.sourcePath)
            ?.valueColumns.find((valueColumn) => valueColumn.sourceColumn === column.name)
            ?.mnemonic ?? column.name,
        unit:
          asciiImportBySourcePath
            .get(file.sourcePath)
            ?.valueColumns.find((valueColumn) => valueColumn.sourceColumn === column.name)
            ?.unit ?? ""
      }))
  }));
}

export function createWellSourceImportSession(
  preview: ProjectWellSourceImportPreview
): WellSourceImportSessionState {
  return {
    preview: preview.parsed,
    suggestedDraft: preview.suggestedDraft,
    draft: draftFromSuggestedDraft(preview.parsed, preview.suggestedDraft),
    topsRows: editableTopsRows(preview.parsed.topsMarkers.rows),
    selectedLogSourcePaths: defaultSelectedLogSourcePaths(preview.suggestedDraft),
    asciiLogDrafts: editableAsciiLogDrafts(preview.parsed, preview.suggestedDraft),
    trajectoryRows: editableTrajectoryRows(preview.parsed, preview.suggestedDraft)
  };
}

export function selectedAsciiImportDrafts(
  draft: WellSourceImportDraft,
  asciiLogDrafts: WellSourceImportEditableAsciiLogImport[],
  parseOptionalNumber: (value: string) => number | null,
  blankToNull: (value: string) => string | null
): ProjectWellSourceAsciiLogImportRequest[] {
  return draft.importLogs
    ? asciiLogDrafts
        .filter((entry) => entry.enabled)
        .map((entry) => ({
          sourcePath: entry.sourcePath,
          depthColumn: entry.depthColumn,
          nullValue: parseOptionalNumber(entry.nullValue),
          valueColumns: entry.curves
            .filter((curve) => curve.enabled && curve.mnemonic.trim().length > 0)
            .map((curve) => ({
              sourceColumn: curve.sourceColumn,
              mnemonic: curve.mnemonic.trim(),
              unit: blankToNull(curve.unit)
            }))
        }))
        .filter((entry) => entry.depthColumn.trim().length > 0 && entry.valueColumns.length > 0)
    : [];
}

export function emptyTrajectoryRow(): WellSourceImportEditableTrajectoryRow {
  return {
    measuredDepth: "",
    inclinationDeg: "",
    azimuthDeg: "",
    trueVerticalDepth: "",
    xOffset: "",
    yOffset: ""
  };
}

export function selectedTrajectoryDraftRows(
  rows: WellSourceImportEditableTrajectoryRow[],
  parseOptionalNumber: (value: string) => number | null
): ProjectWellSourceTrajectoryDraftRow[] {
  return rows
    .map((row) => ({
      measuredDepth: parseOptionalNumber(row.measuredDepth),
      inclinationDeg: parseOptionalNumber(row.inclinationDeg),
      azimuthDeg: parseOptionalNumber(row.azimuthDeg),
      trueVerticalDepth: parseOptionalNumber(row.trueVerticalDepth),
      xOffset: parseOptionalNumber(row.xOffset),
      yOffset: parseOptionalNumber(row.yOffset)
    }))
    .filter(
      (row) =>
        row.measuredDepth !== null ||
        row.inclinationDeg !== null ||
        row.azimuthDeg !== null ||
        row.trueVerticalDepth !== null ||
        row.xOffset !== null ||
        row.yOffset !== null
    );
}

export interface WellSourceTrajectoryDraftSupport {
  measuredDepthCount: number;
  inclinationDegCount: number;
  azimuthDegCount: number;
  trueVerticalDepthCount: number;
  xOffsetCount: number;
  yOffsetCount: number;
  committableRowCount: number;
  commitEnabled: boolean;
}

export function summarizeTrajectoryDraftRows(
  rows: ProjectWellSourceTrajectoryDraftRow[]
): WellSourceTrajectoryDraftSupport {
  const measuredDepthCount = rows.filter((row) => row.measuredDepth !== null && row.measuredDepth !== undefined)
    .length;
  const inclinationDegCount = rows.filter(
    (row) => row.inclinationDeg !== null && row.inclinationDeg !== undefined
  ).length;
  const azimuthDegCount = rows.filter((row) => row.azimuthDeg !== null && row.azimuthDeg !== undefined)
    .length;
  const trueVerticalDepthCount = rows.filter(
    (row) => row.trueVerticalDepth !== null && row.trueVerticalDepth !== undefined
  ).length;
  const xOffsetCount = rows.filter((row) => row.xOffset !== null && row.xOffset !== undefined).length;
  const yOffsetCount = rows.filter((row) => row.yOffset !== null && row.yOffset !== undefined).length;
  const hasMdIncAzi =
    measuredDepthCount >= 2 && inclinationDegCount >= 2 && azimuthDegCount >= 2;
  const hasMdTvdXy =
    measuredDepthCount >= 2 &&
    trueVerticalDepthCount >= 2 &&
    xOffsetCount >= 2 &&
    yOffsetCount >= 2;
  return {
    measuredDepthCount,
    inclinationDegCount,
    azimuthDegCount,
    trueVerticalDepthCount,
    xOffsetCount,
    yOffsetCount,
    committableRowCount: measuredDepthCount,
    commitEnabled: hasMdIncAzi || hasMdTvdXy
  };
}
