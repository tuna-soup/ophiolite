use crate::project_assets::normalize_depth_semantics;
use crate::{
    AssetBindingInput, CoordinateReference, ExternalReference, IndexKind, IngestIssue, LasError,
    LasFile, LogAssetImportResult, OperatorAssignment, OphioliteProject, Provenance, Result,
    TopRow, TrajectoryRow, VerticalMeasurement, VerticalMeasurementPath, WellMetadata,
    WellboreMetadata, revision_token_for_bytes,
};
use ophiolite_core::{
    CurveItem, HeaderItem, IndexDescriptor, IssueSeverity, LasFileSummary, LasValue, MnemonicCase,
    SectionItems,
};
use ophiolite_parser::read_path;
use ophiolite_seismic::{CoordinateReferenceDescriptor, ProjectedPoint2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const WELL_FOLDER_IMPORT_SCHEMA_VERSION: u32 = 1;
const DEFAULT_TOPS_DEPTH_REFERENCE: &str = "md";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WellFolderImportStatus {
    NotPresent,
    Parsed,
    ParsedWithIssues,
    Unsupported,
    NotViableForCommit,
    ReadyForCommit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WellFolderImportIssueSeverity {
    Info,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WellFolderCoordinateReferenceCandidateConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WellFolderCoordinateReferenceSelectionMode {
    Detected,
    AssumeSameAsSurvey,
    Manual,
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderCoordinateReferenceCandidate {
    pub coordinate_reference: CoordinateReferenceDescriptor,
    pub confidence: WellFolderCoordinateReferenceCandidateConfidence,
    pub evidence: String,
    pub rationale: String,
    pub supports_geometry_commit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderCoordinateReferencePreview {
    pub required_for_surface_location: bool,
    pub required_for_trajectory: bool,
    pub recommended_candidate_id: Option<String>,
    pub candidates: Vec<WellFolderCoordinateReferenceCandidate>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderCoordinateReferenceSelection {
    pub mode: WellFolderCoordinateReferenceSelectionMode,
    pub candidate_id: Option<String>,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderImportIssue {
    pub severity: WellFolderImportIssueSeverity,
    pub code: String,
    pub message: String,
    pub slice: Option<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WellFolderImportOmissionKind {
    SurfaceLocation,
    Trajectory,
    TopsRows,
    Log,
    AsciiLog,
    UnsupportedSources,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WellFolderImportOmissionReasonCode {
    SourceCrsUnresolved,
    TrajectoryNotCommitted,
    TopsRowsIncomplete,
    LogUnselected,
    AsciiLogUnselected,
    UnsupportedPreservedAsSource,
    UnsupportedPreservedAsRawBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderImportOmission {
    pub kind: WellFolderImportOmissionKind,
    pub slice: String,
    pub reason_code: WellFolderImportOmissionReasonCode,
    pub message: String,
    pub source_path: Option<String>,
    pub row_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderImportBindingDraft {
    pub well_name: String,
    pub wellbore_name: String,
    pub uwi: Option<String>,
    pub api: Option<String>,
    pub operator_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderDetectedSource {
    pub source_path: String,
    pub file_name: String,
    pub status: WellFolderImportStatus,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderMetadataSlicePreview {
    pub status: WellFolderImportStatus,
    pub commit_enabled: bool,
    pub source_path: Option<String>,
    pub well_metadata: Option<WellMetadata>,
    pub wellbore_metadata: Option<WellboreMetadata>,
    pub detected_coordinate_references: Vec<CoordinateReferenceDescriptor>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderLogFilePreview {
    pub source_path: String,
    pub file_name: String,
    pub status: WellFolderImportStatus,
    pub row_count: usize,
    pub curve_count: usize,
    pub index_curve_name: String,
    pub curve_names: Vec<String>,
    pub detected_well_name: Option<String>,
    pub issue_count: usize,
    pub default_selected: bool,
    pub selection_reason: Option<String>,
    pub duplicate_group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderLogsSlicePreview {
    pub status: WellFolderImportStatus,
    pub commit_enabled: bool,
    pub files: Vec<WellFolderLogFilePreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderAsciiLogColumnPreview {
    pub name: String,
    pub numeric_count: usize,
    pub null_count: usize,
    pub sample_values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderAsciiLogFilePreview {
    pub source_path: String,
    pub file_name: String,
    pub status: WellFolderImportStatus,
    pub row_count: usize,
    pub column_count: usize,
    pub default_depth_column: Option<String>,
    pub default_value_columns: Vec<String>,
    pub columns: Vec<WellFolderAsciiLogColumnPreview>,
    pub issue_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderAsciiLogsSlicePreview {
    pub status: WellFolderImportStatus,
    pub commit_enabled: bool,
    pub files: Vec<WellFolderAsciiLogFilePreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderTopDraftRow {
    pub name: Option<String>,
    pub top_depth: Option<f64>,
    pub base_depth: Option<f64>,
    pub anomaly: Option<String>,
    pub quality: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderTopsSlicePreview {
    pub status: WellFolderImportStatus,
    pub commit_enabled: bool,
    pub source_path: Option<String>,
    pub row_count: usize,
    pub committable_row_count: usize,
    pub preferred_depth_reference: Option<String>,
    pub source_name: Option<String>,
    pub rows: Vec<WellFolderTopDraftRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderTrajectoryDraftRow {
    pub measured_depth: Option<f64>,
    pub inclination_deg: Option<f64>,
    pub azimuth_deg: Option<f64>,
    pub true_vertical_depth: Option<f64>,
    pub x_offset: Option<f64>,
    pub y_offset: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderTrajectorySlicePreview {
    pub status: WellFolderImportStatus,
    pub commit_enabled: bool,
    pub source_path: Option<String>,
    pub row_count: usize,
    pub committable_row_count: usize,
    pub non_empty_column_count: BTreeMap<String, usize>,
    pub draft_rows: Vec<WellFolderTrajectoryDraftRow>,
    pub sample_rows: Vec<WellFolderTrajectoryDraftRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellFolderImportPreview {
    pub schema_version: u32,
    pub folder_path: String,
    pub folder_name: String,
    pub binding: WellFolderImportBindingDraft,
    pub source_coordinate_reference: WellFolderCoordinateReferencePreview,
    pub metadata: WellFolderMetadataSlicePreview,
    pub logs: WellFolderLogsSlicePreview,
    pub ascii_logs: WellFolderAsciiLogsSlicePreview,
    pub tops_markers: WellFolderTopsSlicePreview,
    pub trajectory: WellFolderTrajectorySlicePreview,
    pub unsupported_sources: Vec<WellFolderDetectedSource>,
    pub issues: Vec<WellFolderImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellFolderImportCommitRequest {
    pub folder_path: String,
    pub source_paths: Option<Vec<String>>,
    pub binding: AssetBindingInput,
    pub well_metadata: Option<WellMetadata>,
    pub wellbore_metadata: Option<WellboreMetadata>,
    pub source_coordinate_reference: WellFolderCoordinateReferenceSelection,
    pub import_logs: bool,
    pub selected_log_source_paths: Option<Vec<String>>,
    pub import_tops_markers: bool,
    pub import_trajectory: bool,
    pub tops_depth_reference: Option<String>,
    pub tops_rows: Option<Vec<WellFolderTopDraftRow>>,
    pub trajectory_rows: Option<Vec<WellFolderTrajectoryDraftRow>>,
    pub ascii_log_imports: Option<Vec<WellFolderAsciiLogImportRequest>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderAsciiLogCurveMapping {
    pub source_column: String,
    pub mnemonic: String,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellFolderAsciiLogImportRequest {
    pub source_path: String,
    pub depth_column: String,
    pub value_columns: Vec<WellFolderAsciiLogCurveMapping>,
    pub null_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellFolderImportedAsset {
    pub asset_kind: String,
    pub source_path: String,
    pub asset_id: String,
    pub collection_id: String,
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellFolderImportCommitResponse {
    pub schema_version: u32,
    pub well_id: String,
    pub wellbore_id: String,
    pub created_well: bool,
    pub created_wellbore: bool,
    pub source_coordinate_reference_mode: WellFolderCoordinateReferenceSelectionMode,
    pub source_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub imported_assets: Vec<ProjectWellFolderImportedAsset>,
    pub omissions: Vec<WellFolderImportOmission>,
    pub issues: Vec<WellFolderImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellSourceImportTopsCanonicalDraft {
    pub depth_reference: Option<String>,
    pub rows: Vec<WellFolderTopDraftRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellSourceImportTrajectoryCanonicalDraft {
    pub enabled: bool,
    pub rows: Option<Vec<WellFolderTrajectoryDraftRow>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellSourceImportPlanCanonicalDraft {
    pub selected_log_source_paths: Option<Vec<String>>,
    pub ascii_log_imports: Option<Vec<WellFolderAsciiLogImportRequest>>,
    pub tops_markers: Option<ProjectWellSourceImportTopsCanonicalDraft>,
    pub trajectory: Option<ProjectWellSourceImportTrajectoryCanonicalDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellSourceImportCanonicalDraft {
    pub binding: AssetBindingInput,
    pub source_coordinate_reference: WellFolderCoordinateReferenceSelection,
    pub well_metadata: Option<WellMetadata>,
    pub wellbore_metadata: Option<WellboreMetadata>,
    pub import_plan: ProjectWellSourceImportPlanCanonicalDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellSourceImportPreview {
    pub parsed: ProjectWellFolderImportPreview,
    pub suggested_draft: ProjectWellSourceImportCanonicalDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTopsSourceImportResult {
    pub schema_version: u32,
    pub source_path: String,
    pub source_name: Option<String>,
    pub reported_well_name: Option<String>,
    pub reported_depth_reference: Option<String>,
    pub resolved_source_depth_reference: Option<String>,
    pub resolved_depth_domain: Option<String>,
    pub resolved_depth_datum: Option<String>,
    pub source_row_count: usize,
    pub imported_row_count: usize,
    pub omitted_row_count: usize,
    pub import_result: crate::ProjectAssetImportResult,
    pub issues: Vec<WellFolderImportIssue>,
    pub omissions: Vec<WellFolderImportOmission>,
}

pub type WellSourceImportStatus = WellFolderImportStatus;
pub type WellSourceImportIssueSeverity = WellFolderImportIssueSeverity;
pub type WellSourceCoordinateReferenceCandidateConfidence =
    WellFolderCoordinateReferenceCandidateConfidence;
pub type WellSourceCoordinateReferenceSelectionMode = WellFolderCoordinateReferenceSelectionMode;
pub type WellSourceCoordinateReferenceCandidate = WellFolderCoordinateReferenceCandidate;
pub type WellSourceCoordinateReferencePreview = WellFolderCoordinateReferencePreview;
pub type WellSourceCoordinateReferenceSelection = WellFolderCoordinateReferenceSelection;
pub type WellSourceImportIssue = WellFolderImportIssue;
pub type WellSourceImportOmissionKind = WellFolderImportOmissionKind;
pub type WellSourceImportOmissionReasonCode = WellFolderImportOmissionReasonCode;
pub type WellSourceImportOmission = WellFolderImportOmission;
pub type WellSourceImportBindingDraft = WellFolderImportBindingDraft;
pub type WellSourceDetectedSource = WellFolderDetectedSource;
pub type WellSourceMetadataSlicePreview = WellFolderMetadataSlicePreview;
pub type WellSourceLogFilePreview = WellFolderLogFilePreview;
pub type WellSourceLogsSlicePreview = WellFolderLogsSlicePreview;
pub type WellSourceAsciiLogColumnPreview = WellFolderAsciiLogColumnPreview;
pub type WellSourceAsciiLogFilePreview = WellFolderAsciiLogFilePreview;
pub type WellSourceAsciiLogsSlicePreview = WellFolderAsciiLogsSlicePreview;
pub type WellSourceTopDraftRow = WellFolderTopDraftRow;
pub type WellSourceTopsSlicePreview = WellFolderTopsSlicePreview;
pub type WellSourceTrajectoryDraftRow = WellFolderTrajectoryDraftRow;
pub type WellSourceTrajectorySlicePreview = WellFolderTrajectorySlicePreview;
pub type ProjectWellSourceImportCommitRequest = ProjectWellFolderImportCommitRequest;
pub type WellSourceAsciiLogCurveMapping = WellFolderAsciiLogCurveMapping;
pub type WellSourceAsciiLogImportRequest = WellFolderAsciiLogImportRequest;
pub type ProjectWellSourceImportedAsset = ProjectWellFolderImportedAsset;
pub type ProjectWellSourceImportCommitResponse = ProjectWellFolderImportCommitResponse;

fn suggested_source_coordinate_reference_selection(
    preview: &ProjectWellFolderImportPreview,
) -> WellFolderCoordinateReferenceSelection {
    let candidate_id = preview
        .source_coordinate_reference
        .recommended_candidate_id
        .clone()
        .or_else(|| {
            preview
                .source_coordinate_reference
                .candidates
                .first()
                .and_then(|candidate| candidate.coordinate_reference.id.clone())
        });
    if candidate_id.is_some() {
        WellFolderCoordinateReferenceSelection {
            mode: WellFolderCoordinateReferenceSelectionMode::Detected,
            candidate_id,
            coordinate_reference: None,
        }
    } else {
        WellFolderCoordinateReferenceSelection {
            mode: WellFolderCoordinateReferenceSelectionMode::Unresolved,
            candidate_id: None,
            coordinate_reference: None,
        }
    }
}

fn resolved_preview_source_coordinate_reference(
    preview: &ProjectWellFolderImportPreview,
    selection: &WellFolderCoordinateReferenceSelection,
) -> Option<CoordinateReferenceDescriptor> {
    match selection.mode {
        WellFolderCoordinateReferenceSelectionMode::Detected => {
            selection
                .candidate_id
                .as_deref()
                .and_then(|candidate_id| {
                    preview
                        .source_coordinate_reference
                        .candidates
                        .iter()
                        .find(|candidate| {
                            candidate
                                .coordinate_reference
                                .id
                                .as_deref()
                                .map(|value| value.eq_ignore_ascii_case(candidate_id))
                                .unwrap_or(false)
                        })
                })
                .or_else(|| {
                    preview
                        .source_coordinate_reference
                        .recommended_candidate_id
                        .as_deref()
                        .and_then(|candidate_id| {
                            preview.source_coordinate_reference.candidates.iter().find(
                                |candidate| {
                                    candidate
                                        .coordinate_reference
                                        .id
                                        .as_deref()
                                        .map(|value| value.eq_ignore_ascii_case(candidate_id))
                                        .unwrap_or(false)
                                },
                            )
                        })
                })
                .or_else(|| preview.source_coordinate_reference.candidates.first())
                .map(|candidate| candidate.coordinate_reference.clone())
        }
        WellFolderCoordinateReferenceSelectionMode::AssumeSameAsSurvey
        | WellFolderCoordinateReferenceSelectionMode::Manual => {
            selection.coordinate_reference.clone()
        }
        WellFolderCoordinateReferenceSelectionMode::Unresolved => None,
    }
}

fn build_suggested_well_source_import_draft(
    preview: &ProjectWellFolderImportPreview,
) -> ProjectWellSourceImportCanonicalDraft {
    let source_coordinate_reference = suggested_source_coordinate_reference_selection(preview);
    let resolved_source_coordinate_reference =
        resolved_preview_source_coordinate_reference(preview, &source_coordinate_reference);
    let mut well_metadata = preview.metadata.well_metadata.clone();
    if let Some(metadata) = well_metadata.as_mut() {
        if let Some(reference) = resolved_source_coordinate_reference.as_ref() {
            if let Some(surface_location) = metadata.surface_location.as_mut() {
                surface_location.coordinate_reference = Some(reference.clone());
            }
        } else {
            metadata.surface_location = None;
        }
    }
    let selected_log_source_paths = preview
        .logs
        .files
        .iter()
        .filter(|file| file.default_selected)
        .map(|file| file.source_path.clone())
        .collect::<Vec<_>>();
    let ascii_log_imports = preview
        .ascii_logs
        .files
        .iter()
        .filter_map(|file| {
            let depth_column = file.default_depth_column.clone()?;
            if file.default_value_columns.is_empty() {
                return None;
            }
            Some(WellFolderAsciiLogImportRequest {
                source_path: file.source_path.clone(),
                depth_column,
                value_columns: file
                    .default_value_columns
                    .iter()
                    .map(|column| WellFolderAsciiLogCurveMapping {
                        source_column: column.clone(),
                        mnemonic: column.clone(),
                        unit: None,
                    })
                    .collect(),
                null_value: Some(-999.25),
            })
        })
        .collect::<Vec<_>>();
    let tops_rows = preview
        .tops_markers
        .rows
        .iter()
        .filter(|row| row.name.is_some() && row.top_depth.is_some())
        .cloned()
        .collect::<Vec<_>>();
    ProjectWellSourceImportCanonicalDraft {
        binding: AssetBindingInput {
            well_name: preview.binding.well_name.clone(),
            wellbore_name: preview.binding.wellbore_name.clone(),
            uwi: preview.binding.uwi.clone(),
            api: preview.binding.api.clone(),
            operator_aliases: preview.binding.operator_aliases.clone(),
        },
        source_coordinate_reference,
        well_metadata,
        wellbore_metadata: preview.metadata.wellbore_metadata.clone(),
        import_plan: ProjectWellSourceImportPlanCanonicalDraft {
            selected_log_source_paths: (!selected_log_source_paths.is_empty())
                .then_some(selected_log_source_paths),
            ascii_log_imports: (!ascii_log_imports.is_empty()).then_some(ascii_log_imports),
            tops_markers: (preview.tops_markers.commit_enabled && !tops_rows.is_empty()).then_some(
                ProjectWellSourceImportTopsCanonicalDraft {
                    depth_reference: preview
                        .tops_markers
                        .preferred_depth_reference
                        .clone()
                        .or_else(|| Some(DEFAULT_TOPS_DEPTH_REFERENCE.to_string())),
                    rows: tops_rows,
                },
            ),
            trajectory: (preview.trajectory.commit_enabled
                && resolved_source_coordinate_reference.is_some())
            .then_some(ProjectWellSourceImportTrajectoryCanonicalDraft {
                enabled: true,
                rows: None,
            }),
        },
    }
}

pub fn preview_well_folder_import(folder_path: &Path) -> Result<ProjectWellFolderImportPreview> {
    Ok(parse_well_folder(folder_path)?.to_preview())
}

pub fn preview_well_source_import(root_path: &Path) -> Result<ProjectWellSourceImportPreview> {
    let parsed = preview_well_folder_import(root_path)?;
    let suggested_draft = build_suggested_well_source_import_draft(&parsed);
    Ok(ProjectWellSourceImportPreview {
        parsed,
        suggested_draft,
    })
}

pub fn preview_well_source_import_sources(
    source_paths: &[PathBuf],
    root_hint: Option<&Path>,
) -> Result<ProjectWellSourceImportPreview> {
    let parsed = preview_well_import_sources(source_paths, root_hint)?;
    let suggested_draft = build_suggested_well_source_import_draft(&parsed);
    Ok(ProjectWellSourceImportPreview {
        parsed,
        suggested_draft,
    })
}

pub fn preview_well_import_sources(
    source_paths: &[PathBuf],
    root_hint: Option<&Path>,
) -> Result<ProjectWellFolderImportPreview> {
    Ok(parse_selected_well_sources(source_paths, root_hint)?.to_preview())
}

pub fn import_tops_source(
    project: &mut OphioliteProject,
    source_path: &Path,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
    depth_reference: Option<&str>,
) -> Result<ProjectTopsSourceImportResult> {
    let parsed = parse_tops_source(source_path)?;
    let resolved_source_depth_reference = depth_reference
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| parsed.reported_depth_reference.clone());
    let resolved_depth_semantics =
        normalize_depth_semantics(resolved_source_depth_reference.as_deref(), None, None);

    let rows = parsed
        .rows
        .iter()
        .filter_map(|row| {
            Some(TopRow {
                name: row.name.clone()?,
                top_depth: row.top_depth?,
                base_depth: row.base_depth,
                source: parsed.source_name.clone(),
                source_depth_reference: resolved_depth_semantics.source_depth_reference.clone(),
                depth_domain: resolved_depth_semantics.depth_domain.clone(),
                depth_datum: resolved_depth_semantics.depth_datum.clone(),
            })
        })
        .collect::<Vec<_>>();

    let omitted_row_count = parsed.rows.len().saturating_sub(rows.len());
    let mut omissions = Vec::new();
    if omitted_row_count > 0 {
        omissions.push(omission(
            WellFolderImportOmissionKind::TopsRows,
            "tops_markers",
            WellFolderImportOmissionReasonCode::TopsRowsIncomplete,
            "Some tops rows were omitted because they did not include both a stratigraphic name and a top depth.",
            Some(path_string(&parsed.path)),
            Some(omitted_row_count),
        ));
    }

    if rows.is_empty() {
        return Err(LasError::Validation(format!(
            "tops source '{}' did not contain any committable rows",
            path_string(source_path)
        )));
    }

    let import_result = project.import_tops_rows_with_supporting_sources(
        &parsed.path,
        binding,
        collection_name,
        &rows,
        &[],
    )?;

    Ok(ProjectTopsSourceImportResult {
        schema_version: WELL_FOLDER_IMPORT_SCHEMA_VERSION,
        source_path: path_string(&parsed.path),
        source_name: parsed.source_name,
        reported_well_name: parsed.reported_well_name,
        reported_depth_reference: parsed.reported_depth_reference,
        resolved_source_depth_reference: resolved_depth_semantics.source_depth_reference,
        resolved_depth_domain: resolved_depth_semantics.depth_domain,
        resolved_depth_datum: resolved_depth_semantics.depth_datum,
        source_row_count: parsed.rows.len(),
        imported_row_count: rows.len(),
        omitted_row_count,
        import_result,
        issues: parsed.issues,
        omissions,
    })
}

pub fn commit_well_source_import(
    project: &mut OphioliteProject,
    request: &ProjectWellSourceImportCommitRequest,
) -> Result<ProjectWellSourceImportCommitResponse> {
    commit_well_folder_import(project, request)
}

pub fn commit_well_folder_import(
    project: &mut OphioliteProject,
    request: &ProjectWellFolderImportCommitRequest,
) -> Result<ProjectWellFolderImportCommitResponse> {
    let parsed = if let Some(source_paths) = request
        .source_paths
        .as_ref()
        .filter(|paths| !paths.is_empty())
    {
        let normalized_paths = source_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
        parse_selected_well_sources(&normalized_paths, Some(Path::new(&request.folder_path)))?
    } else {
        parse_well_folder(Path::new(&request.folder_path))?
    };
    let metadata_support_path = parsed.metadata.as_ref().map(|value| value.path.as_path());
    let selected_log_source_paths = request.selected_log_source_paths.as_ref().map(|paths| {
        paths
            .iter()
            .map(|value| normalize_requested_source_path(value))
            .collect::<BTreeSet<_>>()
    });
    let mut requested_ascii_imports = request.ascii_log_imports.clone().unwrap_or_default();
    for ascii_import in &mut requested_ascii_imports {
        ascii_import.source_path = normalize_requested_source_path(&ascii_import.source_path);
    }
    let requested_ascii_source_paths = requested_ascii_imports
        .iter()
        .map(|value| value.source_path.clone())
        .collect::<BTreeSet<_>>();
    let mut pending_unsupported_source_paths = parsed
        .unsupported_source_paths
        .iter()
        .map(PathBuf::as_path)
        .collect::<Vec<_>>();
    pending_unsupported_source_paths.extend(
        parsed
            .logs
            .iter()
            .filter(|log| {
                !request.import_logs
                    || selected_log_source_paths
                        .as_ref()
                        .map(|paths| !paths.contains(&path_string(&log.path)))
                        .unwrap_or(false)
            })
            .map(|log| log.path.as_path()),
    );
    pending_unsupported_source_paths.extend(
        parsed
            .ascii_logs
            .iter()
            .filter(|log| !requested_ascii_source_paths.contains(&path_string(&log.path)))
            .map(|log| log.path.as_path()),
    );
    let resolved_source_coordinate_reference = resolve_source_coordinate_reference(
        &parsed.source_coordinate_reference,
        &request.source_coordinate_reference,
    )?;
    let mut issues = parsed.issues.clone();
    issues.extend(issues_for_coordinate_reference_resolution(
        &parsed.source_coordinate_reference,
        resolved_source_coordinate_reference.as_ref(),
        request.source_coordinate_reference.mode,
        "source_coordinate_reference",
    ));
    let resolution = project.ensure_well_binding(&request.binding)?;
    let mut omissions = Vec::new();
    if let Some(mut metadata) = request.well_metadata.clone() {
        let had_surface_location = metadata.surface_location.is_some();
        apply_surface_location_policy(
            &mut metadata,
            resolved_source_coordinate_reference.as_ref(),
            request.source_coordinate_reference.mode,
            &mut issues,
        );
        if had_surface_location && metadata.surface_location.is_none() {
            omissions.push(omission(
                WellFolderImportOmissionKind::SurfaceLocation,
                "metadata",
                WellFolderImportOmissionReasonCode::SourceCrsUnresolved,
                "Surface location coordinates were parsed but withheld because the source CRS remains unresolved.",
                None,
                None,
            ));
        }
        project.set_well_metadata(&resolution.well_id, Some(metadata))?;
    }
    if let Some(mut metadata) = request.wellbore_metadata.clone() {
        append_coordinate_reference_note(
            &mut metadata.notes,
            resolved_source_coordinate_reference.as_ref(),
            request.source_coordinate_reference.mode,
        );
        project.set_wellbore_metadata(&resolution.wellbore_id, Some(metadata))?;
    }

    let mut imported_assets = Vec::new();
    let mut unsupported_source_artifacts_reported = false;

    if request.import_logs {
        let selected_logs = parsed
            .logs
            .iter()
            .filter(|log| {
                selected_log_source_paths
                    .as_ref()
                    .map(|paths| paths.contains(&path_string(&log.path)))
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        let omitted_logs = parsed
            .logs
            .iter()
            .filter(|log| {
                !selected_logs
                    .iter()
                    .any(|selected| selected.path == log.path)
            })
            .collect::<Vec<_>>();
        for log in selected_logs {
            let mut supporting_sources = Vec::new();
            if let Some(metadata_path) = metadata_support_path {
                supporting_sources.push(metadata_path);
            }
            let preserved_unsupported = take_pending_unsupported_source_paths(
                &mut supporting_sources,
                &mut pending_unsupported_source_paths,
            );
            let result = project.import_las_with_binding_and_supporting_sources(
                &log.path,
                &request.binding,
                None,
                &supporting_sources,
            )?;
            if preserved_unsupported {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Info,
                    "unsupported.preserved",
                    "Unsupported well-folder source files were preserved as source artifacts on the imported log asset.",
                    Some("unsupported_sources"),
                    Some(path_string(&log.path)),
                ));
                if !unsupported_source_artifacts_reported
                    && !parsed.unsupported_source_paths.is_empty()
                {
                    omissions.extend(parsed.unsupported_source_paths.iter().map(|path| {
                        omission(
                            WellFolderImportOmissionKind::UnsupportedSources,
                            "unsupported_sources",
                            WellFolderImportOmissionReasonCode::UnsupportedPreservedAsSource,
                            "Unsupported sidecar file was preserved as a source artifact on an imported canonical asset.",
                            Some(path_string(path)),
                            None,
                        )
                    }));
                    unsupported_source_artifacts_reported = true;
                }
            }
            imported_assets.push(imported_asset_from_log_result(&log.path, result));
        }
        if !omitted_logs.is_empty() {
            issues.push(issue(
                WellFolderImportIssueSeverity::Info,
                "logs.partial_selection",
                "Some LAS files were preserved as source only because they were left unchecked in the review dialog.",
                Some("logs"),
                None,
            ));
            omissions.extend(omitted_logs.into_iter().map(|log| {
                omission(
                    WellFolderImportOmissionKind::Log,
                    "logs",
                    WellFolderImportOmissionReasonCode::LogUnselected,
                    "LAS file was left unchecked and was not translated into a canonical log asset.",
                    Some(path_string(&log.path)),
                    None,
                )
            }));
        }
    } else if !parsed.logs.is_empty() {
        issues.push(issue(
            WellFolderImportIssueSeverity::Info,
            "logs.skipped",
            "Log files were detected but skipped at commit time.",
            Some("logs"),
            None,
        ));
        omissions.extend(parsed.logs.iter().map(|log| {
            omission(
                WellFolderImportOmissionKind::Log,
                "logs",
                WellFolderImportOmissionReasonCode::LogUnselected,
                "LAS file was detected but left out of the canonical import plan.",
                Some(path_string(&log.path)),
                None,
            )
        }));
    }

    let omitted_ascii_logs = parsed
        .ascii_logs
        .iter()
        .filter(|log| !requested_ascii_source_paths.contains(&path_string(&log.path)))
        .collect::<Vec<_>>();
    omissions.extend(omitted_ascii_logs.into_iter().map(|log| {
        omission(
            WellFolderImportOmissionKind::AsciiLog,
            "ascii_logs",
            WellFolderImportOmissionReasonCode::AsciiLogUnselected,
            "ASCII log table was not mapped for import and remains preserved as source only.",
            Some(path_string(&log.path)),
            None,
        )
    }));

    for ascii_import in &requested_ascii_imports {
        let Some(source) = parsed
            .ascii_logs
            .iter()
            .find(|value| path_string(&value.path) == ascii_import.source_path)
        else {
            issues.push(issue(
                WellFolderImportIssueSeverity::Blocking,
                "ascii_logs.source_missing",
                "Requested ASCII log import source was not found in the parsed folder preview.",
                Some("ascii_logs"),
                Some(ascii_import.source_path.clone()),
            ));
            continue;
        };

        if ascii_import.value_columns.is_empty() {
            issues.push(issue(
                WellFolderImportIssueSeverity::Blocking,
                "ascii_logs.no_mapped_curves",
                "ASCII log import requires at least one mapped value column.",
                Some("ascii_logs"),
                Some(ascii_import.source_path.clone()),
            ));
            continue;
        }

        let ascii_file = build_ascii_log_file(source, ascii_import, &request.binding)?;
        let mut supporting_sources = Vec::new();
        if let Some(metadata_path) = metadata_support_path {
            supporting_sources.push(metadata_path);
        }
        let preserved_unsupported = take_pending_unsupported_source_paths(
            &mut supporting_sources,
            &mut pending_unsupported_source_paths,
        );
        let result = project.import_log_file_with_binding_and_supporting_sources(
            ascii_file,
            &request.binding,
            None,
            &supporting_sources,
        )?;
        if preserved_unsupported {
            issues.push(issue(
                WellFolderImportIssueSeverity::Info,
                "unsupported.preserved",
                "Unmapped well-folder source files were preserved as source artifacts on the imported ASCII log asset.",
                Some("unsupported_sources"),
                Some(path_string(&source.path)),
            ));
            if !unsupported_source_artifacts_reported && !parsed.unsupported_source_paths.is_empty()
            {
                omissions.extend(parsed.unsupported_source_paths.iter().map(|path| {
                    omission(
                        WellFolderImportOmissionKind::UnsupportedSources,
                        "unsupported_sources",
                        WellFolderImportOmissionReasonCode::UnsupportedPreservedAsSource,
                        "Unsupported sidecar file was preserved as a source artifact on an imported canonical asset.",
                        Some(path_string(path)),
                        None,
                    )
                }));
                unsupported_source_artifacts_reported = true;
            }
        }
        imported_assets.push(imported_asset_from_project_result(
            "log_curve_set",
            &source.path,
            result.asset.id.0.clone(),
            result.collection.id.0.clone(),
            result.collection.name.clone(),
        ));
    }

    if request.import_tops_markers {
        if let Some(tops) = &parsed.tops {
            let source_depth_reference = request
                .tops_depth_reference
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| tops.reported_depth_reference.clone());
            let depth_semantics =
                normalize_depth_semantics(source_depth_reference.as_deref(), None, None);
            let rows = if let Some(requested_tops_rows) =
                request.tops_rows.as_ref().filter(|rows| !rows.is_empty())
            {
                requested_tops_rows
                    .iter()
                    .filter_map(|row| {
                        Some(TopRow {
                            name: row.name.clone()?,
                            top_depth: row.top_depth?,
                            base_depth: row.base_depth,
                            source: tops.source_name.clone(),
                            source_depth_reference: depth_semantics.source_depth_reference.clone(),
                            depth_domain: depth_semantics.depth_domain.clone(),
                            depth_datum: depth_semantics.depth_datum.clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                tops.rows
                    .iter()
                    .filter_map(|row| {
                        Some(TopRow {
                            name: row.name.clone()?,
                            top_depth: row.top_depth?,
                            base_depth: row.base_depth,
                            source: tops.source_name.clone(),
                            source_depth_reference: depth_semantics.source_depth_reference.clone(),
                            depth_domain: depth_semantics.depth_domain.clone(),
                            depth_datum: depth_semantics.depth_datum.clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            };
            let source_row_count = request
                .tops_rows
                .as_ref()
                .filter(|rows| !rows.is_empty())
                .map(|rows| rows.len())
                .unwrap_or(tops.rows.len());
            let omitted_row_count = source_row_count.saturating_sub(rows.len());
            if omitted_row_count > 0 {
                omissions.push(omission(
                    WellFolderImportOmissionKind::TopsRows,
                    "tops_markers",
                    WellFolderImportOmissionReasonCode::TopsRowsIncomplete,
                    "Some tops rows were omitted because they did not include both a name and a top depth.",
                    Some(path_string(&tops.path)),
                    Some(omitted_row_count),
                ));
            }
            if rows.is_empty() {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Blocking,
                    "tops.no_committable_rows",
                    "No tops rows contained both a stratigraphic name and a top depth.",
                    Some("tops_markers"),
                    Some(path_string(&tops.path)),
                ));
            } else {
                let mut supporting_sources = Vec::new();
                if let Some(metadata_path) = metadata_support_path {
                    supporting_sources.push(metadata_path);
                }
                let preserved_unsupported = take_pending_unsupported_source_paths(
                    &mut supporting_sources,
                    &mut pending_unsupported_source_paths,
                );
                let result = project.import_tops_rows_with_supporting_sources(
                    &tops.path,
                    &request.binding,
                    None,
                    &rows,
                    &supporting_sources,
                )?;
                if preserved_unsupported {
                    issues.push(issue(
                        WellFolderImportIssueSeverity::Info,
                        "unsupported.preserved",
                        "Unsupported well-folder source files were preserved as source artifacts on the imported top-set asset.",
                        Some("unsupported_sources"),
                        Some(path_string(&tops.path)),
                    ));
                    if !unsupported_source_artifacts_reported
                        && !parsed.unsupported_source_paths.is_empty()
                    {
                        omissions.extend(parsed.unsupported_source_paths.iter().map(|path| {
                            omission(
                                WellFolderImportOmissionKind::UnsupportedSources,
                                "unsupported_sources",
                                WellFolderImportOmissionReasonCode::UnsupportedPreservedAsSource,
                                "Unsupported sidecar file was preserved as a source artifact on an imported canonical asset.",
                                Some(path_string(path)),
                                None,
                            )
                        }));
                        unsupported_source_artifacts_reported = true;
                    }
                }
                imported_assets.push(imported_asset_from_project_result(
                    "top_set",
                    &tops.path,
                    result.asset.id.0.clone(),
                    result.collection.id.0.clone(),
                    result.collection.name.clone(),
                ));
            }
        }
    } else if parsed.tops.is_some() {
        issues.push(issue(
            WellFolderImportIssueSeverity::Info,
            "tops.skipped",
            "Top markers were detected but skipped at commit time.",
            Some("tops_markers"),
            None,
        ));
    }

    if request.import_trajectory {
        if let Some(trajectory) = &parsed.trajectory {
            if resolved_source_coordinate_reference.is_none() {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Blocking,
                    "trajectory.crs_unresolved",
                    "Trajectory import requires an explicit source CRS decision. Choose a detected CRS, use the active survey CRS, or enter one manually before committing trajectory geometry.",
                    Some("trajectory"),
                    Some(path_string(&trajectory.path)),
                ));
                omissions.push(omission(
                    WellFolderImportOmissionKind::Trajectory,
                    "trajectory",
                    WellFolderImportOmissionReasonCode::SourceCrsUnresolved,
                    "Trajectory geometry was withheld because the source CRS remains unresolved.",
                    Some(path_string(&trajectory.path)),
                    None,
                ));
            } else {
                let requested_trajectory_rows = request
                    .trajectory_rows
                    .as_ref()
                    .filter(|rows| !rows.is_empty())
                    .cloned();
                let trajectory_rows = requested_trajectory_rows
                    .as_deref()
                    .unwrap_or(trajectory.rows.as_slice());
                let trajectory_schema = summarize_trajectory_rows(trajectory_rows);
                let rows = committable_trajectory_rows(trajectory_rows);
                if !trajectory_schema.commit_enabled {
                    issues.push(issue(
                        WellFolderImportIssueSeverity::Blocking,
                        "trajectory.not_viable",
                        "Trajectory stations still do not form a viable commit schema. Supply at least two stations with measured depth plus inclination/azimuth or measured depth plus TVD and XY offsets.",
                        Some("trajectory"),
                        Some(path_string(&trajectory.path)),
                    ));
                } else if rows.len() < 2 {
                    issues.push(issue(
                        WellFolderImportIssueSeverity::Blocking,
                        "trajectory.insufficient_rows",
                        "Trajectory file did not produce at least two committable stations.",
                        Some("trajectory"),
                        Some(path_string(&trajectory.path)),
                    ));
                } else {
                    let mut supporting_sources = Vec::new();
                    if let Some(metadata_path) = metadata_support_path {
                        supporting_sources.push(metadata_path);
                    }
                    let preserved_unsupported = take_pending_unsupported_source_paths(
                        &mut supporting_sources,
                        &mut pending_unsupported_source_paths,
                    );
                    let result = project
                        .import_trajectory_rows_with_coordinate_reference_and_supporting_sources(
                            &trajectory.path,
                            &request.binding,
                            None,
                            &rows,
                            resolved_source_coordinate_reference
                                .as_ref()
                                .map(project_coordinate_reference_from_descriptor),
                            &supporting_sources,
                        )?;
                    if preserved_unsupported {
                        issues.push(issue(
                            WellFolderImportIssueSeverity::Info,
                            "unsupported.preserved",
                            "Unsupported well-folder source files were preserved as source artifacts on the imported trajectory asset.",
                            Some("unsupported_sources"),
                            Some(path_string(&trajectory.path)),
                        ));
                        if !unsupported_source_artifacts_reported
                            && !parsed.unsupported_source_paths.is_empty()
                        {
                            omissions.extend(parsed.unsupported_source_paths.iter().map(|path| {
                                omission(
                                    WellFolderImportOmissionKind::UnsupportedSources,
                                    "unsupported_sources",
                                    WellFolderImportOmissionReasonCode::UnsupportedPreservedAsSource,
                                    "Unsupported sidecar file was preserved as a source artifact on an imported canonical asset.",
                                    Some(path_string(path)),
                                    None,
                                )
                            }));
                            unsupported_source_artifacts_reported = true;
                        }
                    }
                    imported_assets.push(imported_asset_from_project_result(
                        "trajectory",
                        &trajectory.path,
                        result.asset.id.0.clone(),
                        result.collection.id.0.clone(),
                        result.collection.name.clone(),
                    ));
                }
            }
        }
    } else if parsed.trajectory.is_some() {
        issues.push(issue(
            WellFolderImportIssueSeverity::Info,
            "trajectory.skipped",
            "Trajectory file was detected but skipped at commit time.",
            Some("trajectory"),
            None,
        ));
        if let Some(trajectory) = &parsed.trajectory {
            omissions.push(omission(
                WellFolderImportOmissionKind::Trajectory,
                "trajectory",
                WellFolderImportOmissionReasonCode::TrajectoryNotCommitted,
                "Trajectory file was detected but left out of the canonical import plan.",
                Some(path_string(&trajectory.path)),
                None,
            ));
        }
    }

    if !pending_unsupported_source_paths.is_empty() {
        let result = project.import_raw_source_bundle_with_binding(
            &pending_unsupported_source_paths,
            &request.binding,
            Some("Unsupported well-folder sources"),
        )?;
        issues.push(issue(
            WellFolderImportIssueSeverity::Info,
            "unsupported.preserved_raw_bundle",
            "Unsupported well-folder source files were preserved in a raw source bundle because no canonical well asset was imported from the folder.",
            Some("unsupported_sources"),
            Some(path_string(&parsed.folder_path)),
        ));
        if !unsupported_source_artifacts_reported && !parsed.unsupported_source_paths.is_empty() {
            omissions.extend(parsed.unsupported_source_paths.iter().map(|path| {
                omission(
                    WellFolderImportOmissionKind::UnsupportedSources,
                    "unsupported_sources",
                    WellFolderImportOmissionReasonCode::UnsupportedPreservedAsRawBundle,
                    "Unsupported sidecar file was preserved in a raw source bundle because no canonical asset imported it.",
                    Some(path_string(path)),
                    None,
                )
            }));
        }
        imported_assets.push(imported_asset_from_project_result(
            "raw_source_bundle",
            &parsed.folder_path,
            result.asset.id.0.clone(),
            result.collection.id.0.clone(),
            result.collection.name.clone(),
        ));
    }

    Ok(ProjectWellFolderImportCommitResponse {
        schema_version: WELL_FOLDER_IMPORT_SCHEMA_VERSION,
        well_id: resolution.well_id.0,
        wellbore_id: resolution.wellbore_id.0,
        created_well: resolution.created_well,
        created_wellbore: resolution.created_wellbore,
        source_coordinate_reference_mode: request.source_coordinate_reference.mode,
        source_coordinate_reference: resolved_source_coordinate_reference,
        imported_assets,
        omissions,
        issues,
    })
}

#[derive(Debug, Clone)]
struct ParsedWellFolder {
    folder_path: PathBuf,
    binding: WellFolderImportBindingDraft,
    source_coordinate_reference: ParsedWellFolderCoordinateReference,
    metadata: Option<ParsedMetadataSource>,
    logs: Vec<ParsedLogSource>,
    ascii_logs: Vec<ParsedAsciiLogSource>,
    tops: Option<ParsedTopsSource>,
    trajectory: Option<ParsedTrajectorySource>,
    unsupported_sources: Vec<WellFolderDetectedSource>,
    unsupported_source_paths: Vec<PathBuf>,
    issues: Vec<WellFolderImportIssue>,
}

#[derive(Debug, Clone)]
struct ParsedMetadataSource {
    path: PathBuf,
    well_name: Option<String>,
    uwi: Option<String>,
    well_metadata: WellMetadata,
    wellbore_metadata: WellboreMetadata,
    coordinate_reference_candidates: Vec<WellFolderCoordinateReferenceCandidate>,
    detected_coordinate_references: Vec<CoordinateReferenceDescriptor>,
    notes: Vec<String>,
    issues: Vec<WellFolderImportIssue>,
}

#[derive(Debug, Clone)]
struct ParsedWellFolderCoordinateReference {
    required_for_surface_location: bool,
    required_for_trajectory: bool,
    recommended_candidate_id: Option<String>,
    candidates: Vec<WellFolderCoordinateReferenceCandidate>,
    notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedLogSource {
    path: PathBuf,
    preview: WellFolderLogFilePreview,
}

#[derive(Debug, Clone)]
struct ParsedAsciiLogSource {
    path: PathBuf,
    headers: Vec<String>,
    rows: Vec<Vec<Option<f64>>>,
    preview: WellFolderAsciiLogFilePreview,
    issues: Vec<WellFolderImportIssue>,
}

#[derive(Debug, Clone)]
struct ParsedTopRow {
    name: Option<String>,
    top_depth: Option<f64>,
    base_depth: Option<f64>,
    anomaly: Option<String>,
    quality: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedTopsSource {
    path: PathBuf,
    source_name: Option<String>,
    reported_well_name: Option<String>,
    reported_depth_reference: Option<String>,
    rows: Vec<ParsedTopRow>,
    issues: Vec<WellFolderImportIssue>,
}

#[derive(Debug, Clone)]
struct ParsedTrajectorySource {
    path: PathBuf,
    rows: Vec<WellFolderTrajectoryDraftRow>,
    non_empty_column_count: BTreeMap<String, usize>,
    commit_enabled: bool,
    issues: Vec<WellFolderImportIssue>,
}

impl ParsedWellFolder {
    fn to_preview(&self) -> ProjectWellFolderImportPreview {
        let metadata_issues = self
            .metadata
            .as_ref()
            .map(|value| value.issues.len())
            .unwrap_or(0);
        let metadata = if let Some(metadata) = &self.metadata {
            WellFolderMetadataSlicePreview {
                status: if metadata_issues > 0 {
                    WellFolderImportStatus::ParsedWithIssues
                } else {
                    WellFolderImportStatus::ReadyForCommit
                },
                commit_enabled: true,
                source_path: Some(path_string(&metadata.path)),
                well_metadata: Some(metadata.well_metadata.clone()),
                wellbore_metadata: Some(metadata.wellbore_metadata.clone()),
                detected_coordinate_references: metadata.detected_coordinate_references.clone(),
                notes: metadata.notes.clone(),
            }
        } else {
            WellFolderMetadataSlicePreview {
                status: WellFolderImportStatus::NotPresent,
                commit_enabled: false,
                source_path: None,
                well_metadata: None,
                wellbore_metadata: None,
                detected_coordinate_references: Vec::new(),
                notes: Vec::new(),
            }
        };

        let logs_have_issues = self
            .logs
            .iter()
            .any(|log| log.preview.status == WellFolderImportStatus::ParsedWithIssues);
        let logs = WellFolderLogsSlicePreview {
            status: if self.logs.is_empty() {
                WellFolderImportStatus::NotPresent
            } else if logs_have_issues {
                WellFolderImportStatus::ParsedWithIssues
            } else {
                WellFolderImportStatus::ReadyForCommit
            },
            commit_enabled: !self.logs.is_empty(),
            files: self
                .logs
                .iter()
                .map(|value| value.preview.clone())
                .collect(),
        };

        let ascii_logs_have_issues = self
            .ascii_logs
            .iter()
            .any(|log| log.preview.status != WellFolderImportStatus::ReadyForCommit);
        let ascii_logs = WellFolderAsciiLogsSlicePreview {
            status: if self.ascii_logs.is_empty() {
                WellFolderImportStatus::NotPresent
            } else if ascii_logs_have_issues {
                WellFolderImportStatus::ParsedWithIssues
            } else {
                WellFolderImportStatus::ReadyForCommit
            },
            commit_enabled: self.ascii_logs.iter().any(|log| {
                log.preview.default_depth_column.is_some()
                    && !log.preview.default_value_columns.is_empty()
            }),
            files: self
                .ascii_logs
                .iter()
                .map(|value| value.preview.clone())
                .collect(),
        };

        let tops = if let Some(tops) = &self.tops {
            let committable_row_count = tops
                .rows
                .iter()
                .filter(|row| row.name.is_some() && row.top_depth.is_some())
                .count();
            WellFolderTopsSlicePreview {
                status: if committable_row_count == 0 {
                    WellFolderImportStatus::NotViableForCommit
                } else if tops.issues.is_empty() {
                    WellFolderImportStatus::ReadyForCommit
                } else {
                    WellFolderImportStatus::ParsedWithIssues
                },
                commit_enabled: committable_row_count > 0,
                source_path: Some(path_string(&tops.path)),
                row_count: tops.rows.len(),
                committable_row_count,
                preferred_depth_reference: Some(DEFAULT_TOPS_DEPTH_REFERENCE.to_string()),
                source_name: tops.source_name.clone(),
                rows: tops
                    .rows
                    .iter()
                    .map(|row| WellFolderTopDraftRow {
                        name: row.name.clone(),
                        top_depth: row.top_depth,
                        base_depth: row.base_depth,
                        anomaly: row.anomaly.clone(),
                        quality: row.quality.clone(),
                        note: row.note.clone(),
                    })
                    .collect(),
            }
        } else {
            WellFolderTopsSlicePreview {
                status: WellFolderImportStatus::NotPresent,
                commit_enabled: false,
                source_path: None,
                row_count: 0,
                committable_row_count: 0,
                preferred_depth_reference: None,
                source_name: None,
                rows: Vec::new(),
            }
        };

        let trajectory = if let Some(trajectory) = &self.trajectory {
            let committable_row_count = trajectory
                .rows
                .iter()
                .filter(|row| row.measured_depth.is_some())
                .count();
            WellFolderTrajectorySlicePreview {
                status: if trajectory.commit_enabled {
                    if trajectory.issues.is_empty() {
                        WellFolderImportStatus::ReadyForCommit
                    } else {
                        WellFolderImportStatus::ParsedWithIssues
                    }
                } else {
                    WellFolderImportStatus::NotViableForCommit
                },
                commit_enabled: trajectory.commit_enabled,
                source_path: Some(path_string(&trajectory.path)),
                row_count: trajectory.rows.len(),
                committable_row_count,
                non_empty_column_count: trajectory.non_empty_column_count.clone(),
                draft_rows: trajectory.rows.clone(),
                sample_rows: trajectory.rows.iter().take(5).cloned().collect(),
            }
        } else {
            WellFolderTrajectorySlicePreview {
                status: WellFolderImportStatus::NotPresent,
                commit_enabled: false,
                source_path: None,
                row_count: 0,
                committable_row_count: 0,
                non_empty_column_count: BTreeMap::new(),
                draft_rows: Vec::new(),
                sample_rows: Vec::new(),
            }
        };

        let folder_name = self
            .folder_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.folder_path.to_string_lossy().into_owned());

        ProjectWellFolderImportPreview {
            schema_version: WELL_FOLDER_IMPORT_SCHEMA_VERSION,
            folder_path: path_string(&self.folder_path),
            folder_name,
            binding: self.binding.clone(),
            source_coordinate_reference: WellFolderCoordinateReferencePreview {
                required_for_surface_location: self
                    .source_coordinate_reference
                    .required_for_surface_location,
                required_for_trajectory: self.source_coordinate_reference.required_for_trajectory,
                recommended_candidate_id: self
                    .source_coordinate_reference
                    .recommended_candidate_id
                    .clone(),
                candidates: self.source_coordinate_reference.candidates.clone(),
                notes: self.source_coordinate_reference.notes.clone(),
            },
            metadata,
            logs,
            ascii_logs,
            tops_markers: tops,
            trajectory,
            unsupported_sources: self.unsupported_sources.clone(),
            issues: self.issues.clone(),
        }
    }
}

fn parse_well_folder(folder_path: &Path) -> Result<ParsedWellFolder> {
    let folder_path = normalize_existing_path(folder_path)?;
    let files = collect_files(&folder_path)?;
    parse_well_import_sources_from_files(folder_path, files)
}

fn parse_selected_well_sources(
    source_paths: &[PathBuf],
    root_hint: Option<&Path>,
) -> Result<ParsedWellFolder> {
    if source_paths.is_empty() {
        return Err(LasError::Validation(
            "well import requires at least one selected source file".to_string(),
        ));
    }
    let mut files = source_paths
        .iter()
        .map(PathBuf::as_path)
        .map(normalize_existing_file_path)
        .collect::<Result<Vec<_>>>()?;
    files.sort();
    files.dedup();
    let folder_path = resolve_selected_source_root(&files, root_hint)?;
    parse_well_import_sources_from_files(folder_path, files)
}

fn parse_well_import_sources_from_files(
    folder_path: PathBuf,
    files: Vec<PathBuf>,
) -> Result<ParsedWellFolder> {
    let mut issues = Vec::new();
    let mut metadata_path = None;
    let mut tops_path = None;
    let mut trajectory_path = None;
    let mut las_paths = Vec::new();
    let mut ascii_log_paths = Vec::new();
    let mut unsupported_sources = Vec::new();
    let mut unsupported_source_paths = Vec::new();

    for path in files {
        let file_name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let lowercase_name = file_name.to_ascii_lowercase();
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if lowercase_name == "basisgegevens.txt" {
            if metadata_path.is_some() {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Warning,
                    "metadata.duplicate",
                    "Multiple basisgegevens files were found. The first file will be used.",
                    Some("metadata"),
                    Some(path_string(&path)),
                ));
                unsupported_sources.push(WellFolderDetectedSource {
                    source_path: path_string(&path),
                    file_name,
                    status: WellFolderImportStatus::Unsupported,
                    reason: "Duplicate metadata source".to_string(),
                });
                unsupported_source_paths.push(path);
            } else {
                metadata_path = Some(path);
            }
            continue;
        }

        if lowercase_name == "lithostratigrafie.txt" {
            if tops_path.is_some() {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Warning,
                    "tops.duplicate",
                    "Multiple lithostratigrafie files were found. The first file will be used.",
                    Some("tops_markers"),
                    Some(path_string(&path)),
                ));
                unsupported_sources.push(WellFolderDetectedSource {
                    source_path: path_string(&path),
                    file_name,
                    status: WellFolderImportStatus::Unsupported,
                    reason: "Duplicate tops source".to_string(),
                });
                unsupported_source_paths.push(path);
            } else {
                tops_path = Some(path);
            }
            continue;
        }

        if lowercase_name == "deviatie.txt" {
            if trajectory_path.is_some() {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Warning,
                    "trajectory.duplicate",
                    "Multiple deviatie files were found. The first file will be used.",
                    Some("trajectory"),
                    Some(path_string(&path)),
                ));
                unsupported_sources.push(WellFolderDetectedSource {
                    source_path: path_string(&path),
                    file_name,
                    status: WellFolderImportStatus::Unsupported,
                    reason: "Duplicate trajectory source".to_string(),
                });
                unsupported_source_paths.push(path);
            } else {
                trajectory_path = Some(path);
            }
            continue;
        }

        if extension == "las" {
            las_paths.push(path);
            continue;
        }

        if extension == "asc" {
            ascii_log_paths.push(path);
            continue;
        }

        if extension == "dlis" {
            unsupported_sources.push(WellFolderDetectedSource {
                source_path: path_string(&path),
                file_name,
                status: WellFolderImportStatus::Unsupported,
                reason: "Wave one does not support DLIS yet.".to_string(),
            });
            unsupported_source_paths.push(path);
        }
    }

    las_paths.sort();
    ascii_log_paths.sort();

    let metadata = metadata_path
        .as_deref()
        .map(parse_metadata_source)
        .transpose()?;
    if let Some(metadata) = &metadata {
        issues.extend(metadata.issues.iter().cloned());
    }

    let tops = tops_path.as_deref().map(parse_tops_source).transpose()?;
    if let Some(tops) = &tops {
        issues.extend(tops.issues.iter().cloned());
    }

    let trajectory = trajectory_path
        .as_deref()
        .map(parse_trajectory_source)
        .transpose()?;
    if let Some(trajectory) = &trajectory {
        issues.extend(trajectory.issues.iter().cloned());
    }

    let mut logs = Vec::new();
    for path in las_paths {
        match parse_log_source(&path) {
            Ok(log) => logs.push(log),
            Err(error) => {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Warning,
                    "logs.parse_failed",
                    &format!("Failed to parse LAS file: {error}"),
                    Some("logs"),
                    Some(path_string(&path)),
                ));
                unsupported_sources.push(WellFolderDetectedSource {
                    source_path: path_string(&path),
                    file_name: path
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned()),
                    status: WellFolderImportStatus::Unsupported,
                    reason: format!("LAS parse failed: {error}"),
                });
                unsupported_source_paths.push(path);
            }
        }
    }
    apply_log_default_selection_hints(&mut logs);

    let mut ascii_logs = Vec::new();
    for path in ascii_log_paths {
        match parse_ascii_log_source(&path) {
            Ok(log) => {
                issues.extend(log.issues.iter().cloned());
                ascii_logs.push(log);
            }
            Err(error) => {
                issues.push(issue(
                    WellFolderImportIssueSeverity::Warning,
                    "ascii_logs.parse_failed",
                    &format!("Failed to parse NLOG ASCII table: {error}"),
                    Some("ascii_logs"),
                    Some(path_string(&path)),
                ));
                unsupported_sources.push(WellFolderDetectedSource {
                    source_path: path_string(&path),
                    file_name: path
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned()),
                    status: WellFolderImportStatus::Unsupported,
                    reason: format!("NLOG ASCII parse failed: {error}"),
                });
                unsupported_source_paths.push(path);
            }
        }
    }

    let folder_name = folder_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder_path.to_string_lossy().into_owned());
    let binding = build_binding_draft(&folder_name, metadata.as_ref(), logs.as_slice());
    let source_coordinate_reference =
        build_source_coordinate_reference_preview(metadata.as_ref(), trajectory.as_ref());

    Ok(ParsedWellFolder {
        folder_path,
        binding,
        source_coordinate_reference,
        metadata,
        logs,
        ascii_logs,
        tops,
        trajectory,
        unsupported_sources,
        unsupported_source_paths,
        issues,
    })
}

fn resolve_selected_source_root(
    source_paths: &[PathBuf],
    root_hint: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(root_hint) = root_hint {
        let normalized_root = normalize_existing_path(root_hint)?;
        if source_paths
            .iter()
            .all(|path| path.starts_with(&normalized_root))
        {
            return Ok(normalized_root);
        }
    }

    let first_parent = source_paths
        .first()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            LasError::Validation(
                "selected well import sources must live inside an accessible directory".to_string(),
            )
        })?;
    let mut candidate = first_parent.clone();
    for path in source_paths.iter().skip(1) {
        while !path.starts_with(&candidate) {
            if !candidate.pop() {
                return Ok(first_parent);
            }
        }
    }
    Ok(candidate)
}

fn take_pending_unsupported_source_paths<'a>(
    supporting_sources: &mut Vec<&'a Path>,
    pending_unsupported_source_paths: &mut Vec<&'a Path>,
) -> bool {
    if pending_unsupported_source_paths.is_empty() {
        return false;
    }
    supporting_sources.extend(std::mem::take(pending_unsupported_source_paths));
    true
}

fn parse_metadata_source(path: &Path) -> Result<ParsedMetadataSource> {
    let text = fs::read_to_string(path)?;
    let values = parse_key_value_lines(&text);
    let mut notes = Vec::new();
    let mut issues = Vec::new();

    let latitude = parse_optional_f64(metadata_value(&values, "Latitude (WGS84)"));
    let longitude = parse_optional_f64(metadata_value(&values, "Longitude (WGS84)"));
    let ed50_x = parse_optional_f64(metadata_value(&values, "X Coord (ED50 UTM31)"));
    let ed50_y = parse_optional_f64(metadata_value(&values, "Y Coord (ED50 UTM31)"));
    let rd_x = parse_optional_f64(metadata_value(&values, "X Coord (RD)"));
    let rd_y = parse_optional_f64(metadata_value(&values, "Y Coord (RD)"));

    let mut coordinate_reference_candidates = Vec::new();
    let mut detected_coordinate_references = Vec::new();
    if latitude.is_some() && longitude.is_some() {
        let coordinate_reference = CoordinateReferenceDescriptor {
            id: Some("EPSG:4326".to_string()),
            name: Some("WGS 84".to_string()),
            geodetic_datum: Some("WGS84".to_string()),
            unit: Some("degree".to_string()),
        };
        detected_coordinate_references.push(coordinate_reference.clone());
        coordinate_reference_candidates.push(WellFolderCoordinateReferenceCandidate {
            coordinate_reference,
            confidence: WellFolderCoordinateReferenceCandidateConfidence::Low,
            evidence: "Latitude (WGS84) / Longitude (WGS84)".to_string(),
            rationale: "Geographic coordinates were detected in basisgegevens. This is useful supporting metadata, but it is not the preferred native projected CRS for absolute XY import.".to_string(),
            supports_geometry_commit: false,
        });
    }
    if ed50_x.is_some() && ed50_y.is_some() {
        let coordinate_reference = CoordinateReferenceDescriptor {
            id: Some("EPSG:23031".to_string()),
            name: Some("ED50 / UTM zone 31N".to_string()),
            geodetic_datum: Some("ED50".to_string()),
            unit: Some("m".to_string()),
        };
        detected_coordinate_references.push(coordinate_reference.clone());
        coordinate_reference_candidates.push(WellFolderCoordinateReferenceCandidate {
            coordinate_reference,
            confidence: WellFolderCoordinateReferenceCandidateConfidence::High,
            evidence: "X Coord (ED50 UTM31) / Y Coord (ED50 UTM31)".to_string(),
            rationale: "Projected ED50 UTM31 coordinates were detected directly in basisgegevens. This is the strongest default candidate for NLOG-style offshore well folders.".to_string(),
            supports_geometry_commit: true,
        });
    }
    if rd_x.is_some() && rd_y.is_some() {
        let coordinate_reference = CoordinateReferenceDescriptor {
            id: Some("EPSG:28992".to_string()),
            name: Some("Amersfoort / RD New".to_string()),
            geodetic_datum: Some("Amersfoort".to_string()),
            unit: Some("m".to_string()),
        };
        detected_coordinate_references.push(coordinate_reference.clone());
        coordinate_reference_candidates.push(WellFolderCoordinateReferenceCandidate {
            coordinate_reference,
            confidence: WellFolderCoordinateReferenceCandidateConfidence::Medium,
            evidence: "X Coord (RD) / Y Coord (RD)".to_string(),
            rationale: "Projected RD coordinates were detected in basisgegevens. This is a plausible native CRS candidate when the folder uses Dutch national grid coordinates.".to_string(),
            supports_geometry_commit: true,
        });
    }

    let surface_location = if let (Some(x), Some(y)) = (ed50_x, ed50_y) {
        Some(crate::LocatedPoint {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: Some("EPSG:23031".to_string()),
                name: Some("ED50 / UTM zone 31N".to_string()),
                geodetic_datum: Some("ED50".to_string()),
                unit: Some("m".to_string()),
            }),
            point: ProjectedPoint2 { x, y },
            recorded_at: None,
            source: Some("basisgegevens".to_string()),
            note: None,
        })
    } else if let (Some(x), Some(y)) = (rd_x, rd_y) {
        Some(crate::LocatedPoint {
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: Some("EPSG:28992".to_string()),
                name: Some("Amersfoort / RD New".to_string()),
                geodetic_datum: Some("Amersfoort".to_string()),
                unit: Some("m".to_string()),
            }),
            point: ProjectedPoint2 { x, y },
            recorded_at: None,
            source: Some("basisgegevens".to_string()),
            note: None,
        })
    } else {
        None
    };

    if surface_location.is_none() && (latitude.is_some() || longitude.is_some()) {
        notes.push("Only geographic coordinates were detected. Absolute XY commit still needs a projected CRS or a user-supplied anchor.".to_string());
    }
    if values
        .get(&normalize_key_value_line_key("Coord System"))
        .map(|value| value.eq_ignore_ascii_case("ED50-GEOGR"))
        .unwrap_or(false)
    {
        notes.push("Metadata reports ED50-GEOGR while projected ED50 UTM31 coordinates are also present. Surface location defaults to ED50 / UTM zone 31N when available.".to_string());
    }

    let vertical_reference =
        normalize_optional_text(metadata_value(&values, "DRP Datum")).map(|value| {
            CoordinateReferenceDescriptor {
                id: None,
                name: Some(value),
                geodetic_datum: None,
                unit: Some("m".to_string()),
            }
        });

    let mut well_external_references = Vec::new();
    let uwi = normalize_optional_text(metadata_value(&values, "UWI"));
    if let Some(uwi_value) = uwi.clone() {
        well_external_references.push(ExternalReference {
            system: "nlog".to_string(),
            id: uwi_value,
            kind: Some("uwi".to_string()),
            note: None,
        });
    }
    if let Some(nitg_number) = normalize_optional_text(metadata_value(&values, "NITG Number")) {
        well_external_references.push(ExternalReference {
            system: "nlog".to_string(),
            id: nitg_number,
            kind: Some("nitg_number".to_string()),
            note: None,
        });
    }
    if let Some(field_code) = normalize_optional_text(metadata_value(&values, "Field Code")) {
        well_external_references.push(ExternalReference {
            system: "nlog".to_string(),
            id: field_code,
            kind: Some("field_code".to_string()),
            note: None,
        });
    }

    let operator_name = normalize_optional_text(metadata_value(&values, "Legal Owner"))
        .or_else(|| normalize_optional_text(metadata_value(&values, "Client")));
    let operator_history = operator_name
        .map(|name| {
            vec![OperatorAssignment {
                organisation_name: Some(name),
                organisation_id: None,
                effective_at: None,
                terminated_at: None,
                source: Some("basisgegevens".to_string()),
                note: None,
            }]
        })
        .unwrap_or_default();

    let mut vertical_measurements = Vec::new();
    push_vertical_measurement(
        &mut vertical_measurements,
        "end_ah_depth_m",
        parse_optional_f64(metadata_value(&values, "End AH Depth (m)")),
        VerticalMeasurementPath::MeasuredDepth,
        vertical_reference.clone(),
        "basisgegevens",
    );
    push_vertical_measurement(
        &mut vertical_measurements,
        "tvd_m",
        parse_optional_f64(metadata_value(&values, "TVD (m)")),
        VerticalMeasurementPath::TrueVerticalDepth,
        vertical_reference.clone(),
        "basisgegevens",
    );
    push_vertical_measurement(
        &mut vertical_measurements,
        "tvd_nap_m",
        parse_optional_f64(metadata_value(&values, "TVD NAP (m)")),
        VerticalMeasurementPath::TrueVerticalDepthSubsea,
        vertical_reference.clone(),
        "basisgegevens",
    );
    push_vertical_measurement(
        &mut vertical_measurements,
        "drp_height_m",
        parse_optional_f64(metadata_value(&values, "DRP Height (m)")),
        VerticalMeasurementPath::Elevation,
        vertical_reference.clone(),
        "basisgegevens",
    );

    let well_metadata = WellMetadata {
        field_name: normalize_optional_text(metadata_value(&values, "Field Name")),
        block_name: normalize_optional_text(metadata_value(&values, "Block")),
        basin_name: None,
        country: Some("Netherlands".to_string()),
        province_state: normalize_optional_text(metadata_value(&values, "Province")),
        location_text: normalize_optional_text(metadata_value(&values, "On/Offshore")).map(
            |value| {
                if value.eq_ignore_ascii_case("OFF") {
                    "Offshore".to_string()
                } else if value.eq_ignore_ascii_case("ON") {
                    "Onshore".to_string()
                } else {
                    value
                }
            },
        ),
        interest_type: normalize_optional_text(metadata_value(&values, "Type")),
        operator_history: operator_history.clone(),
        surface_location,
        default_vertical_measurement_id: vertical_measurements
            .first()
            .and_then(|value| value.measurement_id.clone()),
        default_vertical_coordinate_reference: vertical_reference.clone(),
        vertical_measurements: vertical_measurements.clone(),
        external_references: well_external_references,
        notes: build_metadata_notes(&values),
    };

    let mut wellbore_external_references = Vec::new();
    if let Some(result) = normalize_optional_text(metadata_value(&values, "Result")) {
        wellbore_external_references.push(ExternalReference {
            system: "nlog".to_string(),
            id: result,
            kind: Some("result".to_string()),
            note: None,
        });
    }

    let parent_wellbore_id = normalize_optional_text(metadata_value(&values, "Parent Borehole"))
        .filter(|value| value != "[object Object]");
    if parent_wellbore_id.is_none()
        && values
            .get(&normalize_key_value_line_key("Parent Borehole"))
            .map(|value| value == "[object Object]")
            .unwrap_or(false)
    {
        issues.push(issue(
            WellFolderImportIssueSeverity::Warning,
            "metadata.parent_wellbore_unusable",
            "Parent Borehole field was present but unusable.",
            Some("metadata"),
            Some(path_string(path)),
        ));
    }

    let wellbore_metadata = WellboreMetadata {
        sequence_number: None,
        status: normalize_optional_text(metadata_value(&values, "Status")),
        purpose: normalize_optional_text(metadata_value(&values, "Purpose")),
        trajectory_type: normalize_optional_text(metadata_value(&values, "Trajectory Shape")),
        parent_wellbore_id,
        target_formation: None,
        primary_material: None,
        location_text: normalize_optional_text(metadata_value(&values, "Facility"))
            .or_else(|| normalize_optional_text(metadata_value(&values, "On/Offshore"))),
        service_company_name: normalize_optional_text(metadata_value(&values, "Drilling Company"))
            .or_else(|| normalize_optional_text(metadata_value(&values, "Rig Name"))),
        operator_history,
        bottom_hole_location: None,
        default_vertical_measurement_id: vertical_measurements
            .first()
            .and_then(|value| value.measurement_id.clone()),
        default_vertical_coordinate_reference: vertical_reference,
        vertical_measurements,
        external_references: wellbore_external_references,
        notes: Vec::new(),
    };

    Ok(ParsedMetadataSource {
        path: path.to_path_buf(),
        well_name: normalize_optional_text(metadata_value(&values, "Well Name"))
            .or_else(|| normalize_optional_text(metadata_value(&values, "Short Name"))),
        uwi,
        well_metadata,
        wellbore_metadata,
        coordinate_reference_candidates,
        detected_coordinate_references,
        notes,
        issues,
    })
}

fn parse_tops_source(path: &Path) -> Result<ParsedTopsSource> {
    let text = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    let mut source_name = None;
    let mut reported_well_name = None;
    let mut reported_depth_reference = None;
    let mut in_table = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Well Name:") {
            reported_well_name =
                normalize_optional_text(Some(trimmed.trim_start_matches("Well Name:").trim()));
            continue;
        }
        if trimmed.starts_with("Depth Ref Point:") {
            reported_depth_reference = normalize_optional_text(Some(
                trimmed.trim_start_matches("Depth Ref Point:").trim(),
            ));
            continue;
        }
        if trimmed.starts_with("Source:") {
            source_name =
                normalize_optional_text(Some(trimmed.trim_start_matches("Source:").trim()));
            continue;
        }
        if trimmed.starts_with("Top(m)") {
            in_table = true;
            continue;
        }
        if !in_table || trimmed.is_empty() {
            continue;
        }

        let fields = line.split('\t').map(str::trim).collect::<Vec<_>>();
        if fields.len() < 6 {
            continue;
        }
        let name = normalize_optional_text(fields.get(2).copied());
        let top_depth = parse_optional_f64(fields.first().copied());
        let base_depth = parse_optional_f64(fields.get(1).copied());
        if name.is_none() {
            issues.push(issue(
                WellFolderImportIssueSeverity::Warning,
                "tops.name_missing",
                "A tops row is missing the stratigraphic unit name.",
                Some("tops_markers"),
                Some(path_string(path)),
            ));
        }
        if top_depth.is_none() {
            issues.push(issue(
                WellFolderImportIssueSeverity::Warning,
                "tops.top_depth_missing",
                "A tops row is missing top depth and will be preview-only until the user supplements it.",
                Some("tops_markers"),
                Some(path_string(path)),
            ));
        }
        rows.push(ParsedTopRow {
            name,
            top_depth,
            base_depth,
            anomaly: normalize_optional_text(fields.get(3).copied()),
            quality: normalize_optional_text(fields.get(4).copied()),
            note: normalize_optional_text(fields.get(5).copied()),
        });
    }

    Ok(ParsedTopsSource {
        path: path.to_path_buf(),
        source_name,
        reported_well_name,
        reported_depth_reference,
        rows,
        issues,
    })
}

fn parse_trajectory_source(path: &Path) -> Result<ParsedTrajectorySource> {
    let text = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    let mut in_table = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Depth(m)") {
            in_table = true;
            continue;
        }
        if !in_table || trimmed.is_empty() {
            continue;
        }
        let fields = line.split('\t').map(str::trim).collect::<Vec<_>>();
        rows.push(WellFolderTrajectoryDraftRow {
            measured_depth: parse_optional_f64(fields.first().copied()),
            inclination_deg: parse_optional_f64(fields.get(1).copied()),
            azimuth_deg: parse_optional_f64(fields.get(2).copied()),
            true_vertical_depth: parse_optional_f64(fields.get(3).copied()),
            x_offset: parse_optional_f64(fields.get(4).copied()),
            y_offset: parse_optional_f64(fields.get(5).copied()),
        });
    }

    let summary = summarize_trajectory_rows(&rows);

    if !summary.commit_enabled {
        issues.push(issue(
            WellFolderImportIssueSeverity::Warning,
            "trajectory.schema_incomplete",
            &format!(
                "Deviation survey columns are too incomplete for commit (md={}, inc={}, azi={}, tvd={}, x={}, y={}).",
                summary.measured_depth_count,
                summary.inclination_deg_count,
                summary.azimuth_deg_count,
                summary.true_vertical_depth_count,
                summary.x_offset_count,
                summary.y_offset_count,
            ),
            Some("trajectory"),
            Some(path_string(path)),
        ));
    }

    Ok(ParsedTrajectorySource {
        path: path.to_path_buf(),
        rows,
        non_empty_column_count: summary.non_empty_column_count,
        commit_enabled: summary.commit_enabled,
        issues,
    })
}

#[derive(Debug, Clone)]
struct TrajectoryDraftSummary {
    non_empty_column_count: BTreeMap<String, usize>,
    measured_depth_count: usize,
    inclination_deg_count: usize,
    azimuth_deg_count: usize,
    true_vertical_depth_count: usize,
    x_offset_count: usize,
    y_offset_count: usize,
    commit_enabled: bool,
}

fn summarize_trajectory_rows(rows: &[WellFolderTrajectoryDraftRow]) -> TrajectoryDraftSummary {
    let mut non_empty_column_count = BTreeMap::from([
        ("measured_depth".to_string(), 0_usize),
        ("inclination_deg".to_string(), 0_usize),
        ("azimuth_deg".to_string(), 0_usize),
        ("true_vertical_depth".to_string(), 0_usize),
        ("x_offset".to_string(), 0_usize),
        ("y_offset".to_string(), 0_usize),
    ]);
    for row in rows {
        increment_non_empty(
            &mut non_empty_column_count,
            "measured_depth",
            row.measured_depth,
        );
        increment_non_empty(
            &mut non_empty_column_count,
            "inclination_deg",
            row.inclination_deg,
        );
        increment_non_empty(&mut non_empty_column_count, "azimuth_deg", row.azimuth_deg);
        increment_non_empty(
            &mut non_empty_column_count,
            "true_vertical_depth",
            row.true_vertical_depth,
        );
        increment_non_empty(&mut non_empty_column_count, "x_offset", row.x_offset);
        increment_non_empty(&mut non_empty_column_count, "y_offset", row.y_offset);
    }
    let measured_depth_count = *non_empty_column_count.get("measured_depth").unwrap_or(&0);
    let inclination_deg_count = *non_empty_column_count.get("inclination_deg").unwrap_or(&0);
    let azimuth_deg_count = *non_empty_column_count.get("azimuth_deg").unwrap_or(&0);
    let true_vertical_depth_count = *non_empty_column_count
        .get("true_vertical_depth")
        .unwrap_or(&0);
    let x_offset_count = *non_empty_column_count.get("x_offset").unwrap_or(&0);
    let y_offset_count = *non_empty_column_count.get("y_offset").unwrap_or(&0);
    let has_md_inc_azi =
        measured_depth_count >= 2 && inclination_deg_count >= 2 && azimuth_deg_count >= 2;
    let has_md_tvd_xy = measured_depth_count >= 2
        && true_vertical_depth_count >= 2
        && x_offset_count >= 2
        && y_offset_count >= 2;
    TrajectoryDraftSummary {
        non_empty_column_count,
        measured_depth_count,
        inclination_deg_count,
        azimuth_deg_count,
        true_vertical_depth_count,
        x_offset_count,
        y_offset_count,
        commit_enabled: has_md_inc_azi || has_md_tvd_xy,
    }
}

fn committable_trajectory_rows(rows: &[WellFolderTrajectoryDraftRow]) -> Vec<TrajectoryRow> {
    rows.iter()
        .filter_map(|row| {
            Some(TrajectoryRow {
                measured_depth: row.measured_depth?,
                true_vertical_depth: row.true_vertical_depth,
                true_vertical_depth_subsea: None,
                azimuth_deg: row.azimuth_deg,
                inclination_deg: row.inclination_deg,
                northing_offset: row.y_offset,
                easting_offset: row.x_offset,
            })
        })
        .collect()
}

fn parse_log_source(path: &Path) -> Result<ParsedLogSource> {
    let file = read_path(path, &Default::default())?;
    let well_info = file.well_info();
    let preview = WellFolderLogFilePreview {
        source_path: path_string(path),
        file_name: path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned()),
        status: if file.issues.is_empty() {
            WellFolderImportStatus::ReadyForCommit
        } else {
            WellFolderImportStatus::ParsedWithIssues
        },
        row_count: file.summary.row_count,
        curve_count: file.summary.curve_count,
        index_curve_name: file.index.curve_id.clone(),
        curve_names: file.curve_names().into_iter().take(16).collect(),
        detected_well_name: normalize_optional_text(well_info.well.as_deref()),
        issue_count: file.summary.issue_count,
        default_selected: true,
        selection_reason: None,
        duplicate_group_id: None,
    };
    Ok(ParsedLogSource {
        path: path.to_path_buf(),
        preview,
    })
}

fn apply_log_default_selection_hints(logs: &mut [ParsedLogSource]) {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, log) in logs.iter().enumerate() {
        groups
            .entry(log_duplicate_signature(&log.preview))
            .or_default()
            .push(index);
    }

    for (group_index, indexes) in groups
        .values()
        .filter(|indexes| indexes.len() > 1)
        .enumerate()
    {
        let duplicate_group_id = format!("las-duplicate-group-{}", group_index + 1);
        for (offset, index) in indexes.iter().enumerate() {
            let preview = &mut logs[*index].preview;
            preview.duplicate_group_id = Some(duplicate_group_id.clone());
            if offset == 0 {
                preview.default_selected = true;
                preview.selection_reason =
                    Some("Selected by default from a duplicate LAS family.".to_string());
            } else {
                preview.default_selected = false;
                preview.selection_reason =
                    Some("Looks duplicate with another LAS file in this folder.".to_string());
            }
        }
    }
}

fn log_duplicate_signature(preview: &WellFolderLogFilePreview) -> String {
    let well_name = preview
        .detected_well_name
        .as_deref()
        .map(normalize_log_token)
        .unwrap_or_default();
    let curve_names = preview
        .curve_names
        .iter()
        .map(|value| normalize_log_token(value))
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{well_name}|{}|{}|{}",
        preview.row_count, preview.curve_count, curve_names
    )
}

fn normalize_log_token(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '_' && *character != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

fn parse_ascii_log_source(path: &Path) -> Result<ParsedAsciiLogSource> {
    let text = fs::read_to_string(path)?;
    let mut issues = Vec::new();
    let mut header_line = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        header_line = Some(trimmed.to_string());
        break;
    }

    let header_line =
        header_line.ok_or_else(|| LasError::Parse("ASCII table is empty".to_string()))?;
    let headers = split_ascii_fields(&header_line);
    if headers.len() < 2 {
        return Err(LasError::Parse(
            "ASCII table header must contain at least depth and one value column".to_string(),
        ));
    }

    let mut rows = Vec::new();
    let mut invalid_row_count = 0usize;
    let mut data_started = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == header_line {
            continue;
        }
        if !data_started {
            data_started = true;
        }
        let fields = split_ascii_fields(trimmed);
        if fields.len() != headers.len() {
            invalid_row_count += 1;
            continue;
        }
        let mut row = Vec::with_capacity(fields.len());
        let mut row_parse_failed = false;
        for field in fields {
            match field.parse::<f64>() {
                Ok(value) => row.push(Some(value)),
                Err(_) => {
                    row_parse_failed = true;
                    break;
                }
            }
        }
        if row_parse_failed {
            invalid_row_count += 1;
            continue;
        }
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(LasError::Parse(
            "ASCII table did not contain any numeric data rows".to_string(),
        ));
    }

    let depth_index = headers
        .iter()
        .position(|name| is_depth_like_ascii_column(name))
        .or(Some(0));

    let mut columns = Vec::with_capacity(headers.len());
    for (column_index, name) in headers.iter().enumerate() {
        let mut numeric_count = 0usize;
        let mut null_count = 0usize;
        let mut sample_values = Vec::new();
        for row in &rows {
            if let Some(value) = row[column_index] {
                numeric_count += 1;
                if is_ascii_null_value(value) {
                    null_count += 1;
                } else if sample_values.len() < 4 {
                    sample_values.push(value);
                }
            }
        }
        columns.push(WellFolderAsciiLogColumnPreview {
            name: name.clone(),
            numeric_count,
            null_count,
            sample_values,
        });
    }

    let default_depth_column = depth_index.map(|index| headers[index].clone());
    let default_value_columns = headers
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            if Some(index) == depth_index {
                return None;
            }
            let column = columns.get(index)?;
            (column.numeric_count > column.null_count).then(|| name.clone())
        })
        .collect::<Vec<_>>();

    if invalid_row_count > 0 {
        issues.push(issue(
            WellFolderImportIssueSeverity::Warning,
            "ascii_logs.invalid_rows",
            &format!(
                "Skipped {invalid_row_count} rows that did not match the detected NLOG ASCII schema."
            ),
            Some("ascii_logs"),
            Some(path_string(path)),
        ));
    }
    if default_depth_column.is_none() {
        issues.push(issue(
            WellFolderImportIssueSeverity::Warning,
            "ascii_logs.depth_unresolved",
            "Could not confidently identify a depth column. Review the mapping before commit.",
            Some("ascii_logs"),
            Some(path_string(path)),
        ));
    }
    if default_value_columns.is_empty() {
        issues.push(issue(
            WellFolderImportIssueSeverity::Warning,
            "ascii_logs.no_value_columns",
            "No numeric log columns were detected beyond the depth column.",
            Some("ascii_logs"),
            Some(path_string(path)),
        ));
    }

    let status = if default_depth_column.is_some() && !default_value_columns.is_empty() {
        if issues.is_empty() {
            WellFolderImportStatus::ReadyForCommit
        } else {
            WellFolderImportStatus::ParsedWithIssues
        }
    } else {
        WellFolderImportStatus::NotViableForCommit
    };

    let row_count = rows.len();

    Ok(ParsedAsciiLogSource {
        path: path.to_path_buf(),
        headers,
        rows,
        preview: WellFolderAsciiLogFilePreview {
            source_path: path_string(path),
            file_name: path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned()),
            status,
            row_count,
            column_count: columns.len(),
            default_depth_column,
            default_value_columns,
            columns,
            issue_count: issues.len(),
        },
        issues,
    })
}

fn split_ascii_fields(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}

fn is_depth_like_ascii_column(name: &str) -> bool {
    matches!(
        normalize_log_token(name).as_str(),
        "DEPTH" | "DEPT" | "MD" | "MEASUREDDEPTH"
    )
}

fn is_ascii_null_value(value: f64) -> bool {
    (value + 999.25).abs() < 1e-6 || (value + 999.0).abs() < 1e-6
}

fn build_ascii_log_file(
    source: &ParsedAsciiLogSource,
    request: &WellFolderAsciiLogImportRequest,
    binding: &AssetBindingInput,
) -> Result<LasFile> {
    let depth_index = source
        .headers
        .iter()
        .position(|value| value.eq_ignore_ascii_case(&request.depth_column))
        .ok_or_else(|| {
            LasError::Parse(format!(
                "Depth column '{}' was not found",
                request.depth_column
            ))
        })?;
    let value_indexes = request
        .value_columns
        .iter()
        .map(|mapping| {
            source
                .headers
                .iter()
                .position(|value| value.eq_ignore_ascii_case(&mapping.source_column))
                .map(|index| (mapping, index))
                .ok_or_else(|| {
                    LasError::Parse(format!(
                        "ASCII log column '{}' was not found",
                        mapping.source_column
                    ))
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let null_value = request.null_value.unwrap_or(-999.25);

    let mut depth_values = Vec::new();
    let mut mapped_curve_values = vec![Vec::new(); value_indexes.len()];
    let mut skipped_null_depth_rows = 0usize;
    for row in &source.rows {
        let Some(depth_value) = row[depth_index] else {
            continue;
        };
        if is_ascii_null_value(depth_value) {
            skipped_null_depth_rows += 1;
            continue;
        }
        depth_values.push(LasValue::Number(depth_value));
        for (mapped_index, (_, index)) in value_indexes.iter().enumerate() {
            mapped_curve_values[mapped_index].push(match row[*index] {
                Some(value) if !is_ascii_null_value(value) => LasValue::Number(value),
                _ => LasValue::Empty,
            });
        }
    }

    if depth_values.is_empty() {
        return Err(LasError::Parse(
            "ASCII log mapping did not yield any committable depth samples".to_string(),
        ));
    }

    let depth_numbers = depth_values
        .iter()
        .filter_map(LasValue::as_f64)
        .collect::<Vec<_>>();
    let start = *depth_numbers.first().unwrap_or(&0.0);
    let stop = *depth_numbers.last().unwrap_or(&start);
    let step = depth_numbers
        .windows(2)
        .find_map(|pair| {
            let delta = pair[1] - pair[0];
            (delta.abs() > f64::EPSILON).then_some(delta)
        })
        .unwrap_or(0.0);
    let source_bytes = fs::read(&source.path)?;
    let imported_at_unix_seconds = now_unix_seconds();
    let provenance = Provenance::from_path(
        &source.path,
        source_fingerprint_for_bytes(&source_bytes),
        imported_at_unix_seconds,
    );

    let depth_mnemonic = sanitize_las_mnemonic(&request.depth_column);
    let mut curves = Vec::with_capacity(value_indexes.len() + 1);
    curves.push(CurveItem::new(
        depth_mnemonic.clone(),
        "",
        LasValue::Empty,
        "Depth".to_string(),
        depth_values,
    ));
    for (mapped_index, (mapping, _)) in value_indexes.iter().enumerate() {
        curves.push(CurveItem::new(
            sanitize_las_mnemonic(&mapping.mnemonic),
            mapping.unit.clone().unwrap_or_default(),
            LasValue::Empty,
            mapping.source_column.clone(),
            mapped_curve_values[mapped_index].clone(),
        ));
    }

    let mut issues = source
        .issues
        .iter()
        .map(ascii_issue_to_ingest_issue)
        .collect::<Vec<_>>();
    if skipped_null_depth_rows > 0 {
        issues.push(IngestIssue {
            severity: IssueSeverity::Warning,
            code: "ASCII_NULL_DEPTH_ROWS_SKIPPED".to_string(),
            message: format!(
                "Skipped {skipped_null_depth_rows} rows because the selected depth column resolved to the configured null value."
            ),
            line: None,
        });
    }

    Ok(LasFile {
        summary: LasFileSummary {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
            las_version: "2.0".to_string(),
            wrap_mode: "NO".to_string(),
            delimiter: "space".to_string(),
            row_count: depth_numbers.len(),
            curve_count: curves.len(),
            issue_count: issues.len(),
        },
        provenance,
        encoding: None,
        index: IndexDescriptor {
            curve_id: depth_mnemonic.clone(),
            raw_mnemonic: request.depth_column.clone(),
            unit: String::new(),
            kind: IndexKind::Depth,
        },
        version: SectionItems::from_items(
            vec![
                HeaderItem::new("VERS", "", 2.0, "CWLS log ASCII Standard"),
                HeaderItem::new("WRAP", "", "NO", "One line per depth step"),
            ],
            MnemonicCase::Upper,
        ),
        well: SectionItems::from_items(
            vec![
                HeaderItem::new("STRT", "", start, "Start depth"),
                HeaderItem::new("STOP", "", stop, "Stop depth"),
                HeaderItem::new("STEP", "", step, "Step size"),
                HeaderItem::new("NULL", "", null_value, "Null value"),
                HeaderItem::new("WELL", "", binding.well_name.clone(), "Well name"),
                HeaderItem::new(
                    "UWI",
                    "",
                    binding.uwi.clone().unwrap_or_default(),
                    "Unique well identifier",
                ),
                HeaderItem::new(
                    "API",
                    "",
                    binding.api.clone().unwrap_or_default(),
                    "API identifier",
                ),
            ],
            MnemonicCase::Upper,
        ),
        params: SectionItems::new(MnemonicCase::Upper),
        curves: SectionItems::from_items(curves, MnemonicCase::Upper),
        other: String::new(),
        extra_sections: BTreeMap::new(),
        issues,
        index_unit: None,
    })
}

fn sanitize_las_mnemonic(value: &str) -> String {
    let mut mnemonic = value
        .trim()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .flat_map(char::to_uppercase)
        .collect::<String>();
    if mnemonic.is_empty() {
        mnemonic = "CURVE".to_string();
    }
    mnemonic.truncate(16);
    mnemonic
}

fn ascii_issue_to_ingest_issue(issue: &WellFolderImportIssue) -> IngestIssue {
    IngestIssue {
        severity: match issue.severity {
            WellFolderImportIssueSeverity::Info | WellFolderImportIssueSeverity::Warning => {
                IssueSeverity::Warning
            }
            WellFolderImportIssueSeverity::Blocking => IssueSeverity::Error,
        },
        code: issue.code.to_ascii_uppercase(),
        message: issue.message.clone(),
        line: None,
    }
}

fn source_fingerprint_for_bytes(bytes: &[u8]) -> String {
    let checksum = bytes.iter().fold(0u64, |acc, byte| {
        acc.wrapping_mul(16777619).wrapping_add(u64::from(*byte))
    });
    revision_token_for_bytes("source", &format!("{}:{checksum}", bytes.len())).0
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn build_source_coordinate_reference_preview(
    metadata: Option<&ParsedMetadataSource>,
    trajectory: Option<&ParsedTrajectorySource>,
) -> ParsedWellFolderCoordinateReference {
    let mut notes = Vec::new();
    let candidates = metadata
        .map(|value| value.coordinate_reference_candidates.clone())
        .unwrap_or_default();
    let recommended_candidate_id = candidates
        .iter()
        .find(|candidate| candidate.supports_geometry_commit)
        .and_then(|candidate| candidate.coordinate_reference.id.clone())
        .or_else(|| {
            candidates
                .first()
                .and_then(|candidate| candidate.coordinate_reference.id.clone())
        });
    let required_for_surface_location = metadata
        .and_then(|value| value.well_metadata.surface_location.as_ref())
        .is_some();
    let required_for_trajectory = trajectory.is_some();

    if required_for_surface_location || required_for_trajectory {
        if candidates.is_empty() {
            notes.push(
                "No source CRS could be detected from the folder contents. Geometry-bearing slices can still be previewed, but surface location and trajectory commit require an explicit CRS decision."
                    .to_string(),
            );
        } else {
            let projected_candidate_count = candidates
                .iter()
                .filter(|candidate| candidate.supports_geometry_commit)
                .count();
            if projected_candidate_count > 1 {
                notes.push(
                    "Multiple projected CRS candidates were detected. Review and confirm the intended source CRS before committing geometry."
                        .to_string(),
                );
            } else if projected_candidate_count == 0 {
                notes.push(
                    "Only geographic CRS evidence was detected. That is useful for review, but an explicit projected CRS decision may still be needed for absolute XY workflows."
                        .to_string(),
                );
            }
        }
    }

    ParsedWellFolderCoordinateReference {
        required_for_surface_location,
        required_for_trajectory,
        recommended_candidate_id,
        candidates,
        notes,
    }
}

fn resolve_source_coordinate_reference(
    preview: &ParsedWellFolderCoordinateReference,
    selection: &WellFolderCoordinateReferenceSelection,
) -> Result<Option<CoordinateReferenceDescriptor>> {
    match selection.mode {
        WellFolderCoordinateReferenceSelectionMode::Unresolved => Ok(None),
        WellFolderCoordinateReferenceSelectionMode::Detected => Ok(selection
            .candidate_id
            .as_deref()
            .and_then(|candidate_id| {
                preview.candidates.iter().find(|candidate| {
                    candidate
                        .coordinate_reference
                        .id
                        .as_deref()
                        .map(|value| value.eq_ignore_ascii_case(candidate_id))
                        .unwrap_or(false)
                })
            })
            .or_else(|| {
                preview
                    .recommended_candidate_id
                    .as_deref()
                    .and_then(|candidate_id| {
                        preview.candidates.iter().find(|candidate| {
                            candidate
                                .coordinate_reference
                                .id
                                .as_deref()
                                .map(|value| value.eq_ignore_ascii_case(candidate_id))
                                .unwrap_or(false)
                        })
                    })
            })
            .or_else(|| preview.candidates.first())
            .map(|candidate| candidate.coordinate_reference.clone())),
        WellFolderCoordinateReferenceSelectionMode::AssumeSameAsSurvey
        | WellFolderCoordinateReferenceSelectionMode::Manual => {
            let coordinate_reference = selection.coordinate_reference.clone().filter(|value| {
                value
                    .id
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|id| !id.is_empty())
                    || value
                        .name
                        .as_deref()
                        .map(str::trim)
                        .is_some_and(|name| !name.is_empty())
            });
            if coordinate_reference.is_none() {
                Err(crate::LasError::Validation(
                    "well-folder import source CRS selection requires a CRS identifier or label"
                        .to_string(),
                ))
            } else {
                Ok(coordinate_reference)
            }
        }
    }
}

fn issues_for_coordinate_reference_resolution(
    preview: &ParsedWellFolderCoordinateReference,
    resolved: Option<&CoordinateReferenceDescriptor>,
    mode: WellFolderCoordinateReferenceSelectionMode,
    slice: &str,
) -> Vec<WellFolderImportIssue> {
    let mut issues = Vec::new();
    match (mode, resolved) {
        (WellFolderCoordinateReferenceSelectionMode::Detected, Some(reference)) => {
            issues.push(issue(
                WellFolderImportIssueSeverity::Info,
                "crs.detected_selected",
                &format!(
                    "Using detected source CRS {} for geometry-bearing import.",
                    coordinate_reference_label(reference)
                ),
                Some(slice),
                None,
            ));
        }
        (WellFolderCoordinateReferenceSelectionMode::Detected, None)
            if preview.required_for_surface_location || preview.required_for_trajectory =>
        {
            issues.push(issue(
                WellFolderImportIssueSeverity::Warning,
                "crs.detected_missing",
                "Detected source CRS was requested, but no CRS candidate was available from the parsed folder.",
                Some(slice),
                None,
            ));
        }
        (WellFolderCoordinateReferenceSelectionMode::AssumeSameAsSurvey, Some(reference)) => {
            issues.push(issue(
                WellFolderImportIssueSeverity::Info,
                "crs.assumed_same_as_survey",
                &format!(
                    "Using the active survey CRS {} for geometry-bearing import.",
                    coordinate_reference_label(reference)
                ),
                Some(slice),
                None,
            ));
        }
        (WellFolderCoordinateReferenceSelectionMode::Manual, Some(reference)) => {
            issues.push(issue(
                WellFolderImportIssueSeverity::Info,
                "crs.manual_override",
                &format!(
                    "Using a manually supplied source CRS {} for geometry-bearing import.",
                    coordinate_reference_label(reference)
                ),
                Some(slice),
                None,
            ));
        }
        (WellFolderCoordinateReferenceSelectionMode::Unresolved, _)
            if preview.required_for_surface_location || preview.required_for_trajectory =>
        {
            issues.push(issue(
                WellFolderImportIssueSeverity::Warning,
                "crs.unresolved",
                "Source CRS remains unresolved. Geometry-bearing slices will be withheld until a CRS is confirmed.",
                Some(slice),
                None,
            ));
        }
        _ => {}
    }
    issues
}

fn apply_surface_location_policy(
    metadata: &mut WellMetadata,
    resolved: Option<&CoordinateReferenceDescriptor>,
    mode: WellFolderCoordinateReferenceSelectionMode,
    issues: &mut Vec<WellFolderImportIssue>,
) {
    let had_surface_location = metadata.surface_location.is_some();
    if let Some(reference) = resolved {
        if let Some(surface_location) = metadata.surface_location.as_mut() {
            surface_location.coordinate_reference = Some(reference.clone());
        }
    } else {
        metadata.surface_location = None;
    }
    append_coordinate_reference_note(&mut metadata.notes, resolved, mode);
    if had_surface_location && resolved.is_none() {
        issues.push(issue(
            WellFolderImportIssueSeverity::Warning,
            "metadata.surface_location_omitted",
            "Surface location coordinates were parsed, but they were omitted from commit because the source CRS remains unresolved.",
            Some("metadata"),
            None,
        ));
    }
}

fn append_coordinate_reference_note(
    notes: &mut Vec<String>,
    resolved: Option<&CoordinateReferenceDescriptor>,
    mode: WellFolderCoordinateReferenceSelectionMode,
) {
    let note = match (mode, resolved) {
        (WellFolderCoordinateReferenceSelectionMode::Detected, Some(reference)) => Some(format!(
            "Well-folder source CRS confirmed from detected candidate: {}.",
            coordinate_reference_label(reference)
        )),
        (
            WellFolderCoordinateReferenceSelectionMode::AssumeSameAsSurvey,
            Some(reference),
        ) => Some(format!(
            "Well-folder source CRS was confirmed from the active survey CRS: {}.",
            coordinate_reference_label(reference)
        )),
        (WellFolderCoordinateReferenceSelectionMode::Manual, Some(reference)) => Some(format!(
            "Well-folder source CRS was entered manually by the user: {}.",
            coordinate_reference_label(reference)
        )),
        (WellFolderCoordinateReferenceSelectionMode::Unresolved, None) => Some(
            "Well-folder source CRS remains unresolved. Surface and trajectory geometry were not committed.".to_string(),
        ),
        _ => None,
    };
    if let Some(note) = note {
        if !notes.iter().any(|existing| existing == &note) {
            notes.push(note);
        }
    }
}

fn coordinate_reference_label(reference: &CoordinateReferenceDescriptor) -> String {
    match (
        reference
            .id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        reference
            .name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ) {
        (Some(id), Some(name)) => format!("{id} ({name})"),
        (Some(id), None) => id.to_string(),
        (None, Some(name)) => name.to_string(),
        (None, None) => "unlabeled CRS".to_string(),
    }
}

fn project_coordinate_reference_from_descriptor(
    reference: &CoordinateReferenceDescriptor,
) -> CoordinateReference {
    CoordinateReference {
        id: reference.id.clone(),
        name: reference.name.clone(),
        geodetic_datum: reference.geodetic_datum.clone(),
    }
}

fn build_binding_draft(
    folder_name: &str,
    metadata: Option<&ParsedMetadataSource>,
    logs: &[ParsedLogSource],
) -> WellFolderImportBindingDraft {
    let fallback_log_name = logs
        .first()
        .and_then(|log| log.preview.detected_well_name.clone());
    let proposed_name = metadata
        .and_then(|value| value.well_name.clone())
        .or(fallback_log_name)
        .unwrap_or_else(|| folder_name.to_string());
    WellFolderImportBindingDraft {
        well_name: proposed_name.clone(),
        wellbore_name: proposed_name,
        uwi: metadata.and_then(|value| value.uwi.clone()),
        api: None,
        operator_aliases: metadata
            .map(|value| {
                value
                    .well_metadata
                    .operator_history
                    .iter()
                    .filter_map(|assignment| assignment.organisation_name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn normalize_existing_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        Ok(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
    } else {
        Err(crate::LasError::Validation(format!(
            "well folder '{}' does not exist",
            path.to_string_lossy()
        )))
    }
}

fn normalize_existing_file_path(path: &Path) -> Result<PathBuf> {
    let normalized = normalize_existing_path(path)?;
    if normalized.is_file() {
        Ok(normalized)
    } else {
        Err(crate::LasError::Validation(format!(
            "well import source '{}' is not a file",
            path.to_string_lossy()
        )))
    }
}

fn parse_key_value_lines(text: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = normalize_key_value_line_key(key);
            let value = value.trim();
            if !key.is_empty() && !value.is_empty() {
                values.insert(key, value.to_string());
            }
        }
    }
    values
}

fn normalize_key_value_line_key(key: &str) -> String {
    key.trim()
        .trim_start_matches('\u{feff}')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn metadata_value<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    values
        .get(&normalize_key_value_line_key(key))
        .map(String::as_str)
}

fn parse_optional_f64(value: Option<&str>) -> Option<f64> {
    normalize_optional_text(value).and_then(|value| value.parse::<f64>().ok())
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    let normalized = value?.trim();
    if normalized.is_empty()
        || normalized.eq_ignore_ascii_case("n/a")
        || normalized.eq_ignore_ascii_case("null")
    {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn build_metadata_notes(values: &BTreeMap<String, String>) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(date) = normalize_optional_text(metadata_value(values, "Confidentiality Date")) {
        notes.push(format!("Confidentiality date: {date}"));
    }
    if let Some(coord_system) = normalize_optional_text(metadata_value(values, "Coord System")) {
        notes.push(format!("Reported coordinate system: {coord_system}"));
    }
    notes
}

fn push_vertical_measurement(
    measurements: &mut Vec<VerticalMeasurement>,
    measurement_id: &str,
    value: Option<f64>,
    path: VerticalMeasurementPath,
    coordinate_reference: Option<CoordinateReferenceDescriptor>,
    source: &str,
) {
    let Some(value) = value else {
        return;
    };
    measurements.push(VerticalMeasurement {
        measurement_id: Some(measurement_id.to_string()),
        value,
        unit: Some("m".to_string()),
        path,
        coordinate_reference,
        reference_measurement_id: None,
        reference_entity_id: None,
        source: Some(source.to_string()),
        description: None,
    });
}

fn increment_non_empty(counts: &mut BTreeMap<String, usize>, key: &str, value: Option<f64>) {
    if value.is_some() {
        let entry = counts.entry(key.to_string()).or_insert(0);
        *entry += 1;
    }
}

fn imported_asset_from_log_result(
    path: &Path,
    result: LogAssetImportResult,
) -> ProjectWellFolderImportedAsset {
    imported_asset_from_project_result(
        "log",
        path,
        result.asset.id.0,
        result.collection.id.0,
        result.collection.name,
    )
}

fn imported_asset_from_project_result(
    asset_kind: &str,
    path: &Path,
    asset_id: String,
    collection_id: String,
    collection_name: String,
) -> ProjectWellFolderImportedAsset {
    ProjectWellFolderImportedAsset {
        asset_kind: asset_kind.to_string(),
        source_path: path_string(path),
        asset_id,
        collection_id,
        collection_name,
    }
}

fn issue(
    severity: WellFolderImportIssueSeverity,
    code: &str,
    message: &str,
    slice: Option<&str>,
    source_path: Option<String>,
) -> WellFolderImportIssue {
    WellFolderImportIssue {
        severity,
        code: code.to_string(),
        message: message.to_string(),
        slice: slice.map(str::to_string),
        source_path,
    }
}

fn omission(
    kind: WellFolderImportOmissionKind,
    slice: &str,
    reason_code: WellFolderImportOmissionReasonCode,
    message: &str,
    source_path: Option<String>,
    row_count: Option<usize>,
) -> WellFolderImportOmission {
    WellFolderImportOmission {
        kind,
        slice: slice.to_string(),
        reason_code,
        message: message.to_string(),
        source_path,
        row_count,
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn normalize_requested_source_path(path: &str) -> String {
    normalize_existing_file_path(Path::new(path))
        .map(|normalized| path_string(&normalized))
        .unwrap_or_else(|_| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_basisgegevens_metadata_preview() {
        let path = write_temp_file(
            "basisgegevens.txt",
            r#"=== BASISGEGEVENS (Basic Data) ===

Well Name: F02-A-02
Short Name: F02-A-02
UWI: 8514
NITG Number: BF020271
Block: F02
Field Name: F02a Hanze
Field Code: F02-HAN
Latitude (WGS84): 54.94482677
Longitude (WGS84): 4.57209585
X Coord (ED50 UTM31): 600791
Y Coord (ED50 UTM31): 6089995
X Coord (RD): 102743
Y Coord (RD): 773811
On/Offshore: OFF
Legal Owner: Dana Petroleum Netherlands B.V.
Status: Sidetracked
Purpose: Ontwikkeling koolwaterstof
Type: BRH
End AH Depth (m): 1724
TVD (m): 1438.75
TVD NAP (m): 1384.65
DRP Height (m): 54
DRP Datum: MSL
Trajectory Shape: Gedevieerd
Coord System: ED50-GEOGR
"#,
        );
        let parsed = parse_metadata_source(&path).expect("metadata should parse");
        assert_eq!(parsed.well_name.as_deref(), Some("F02-A-02"));
        assert_eq!(parsed.uwi.as_deref(), Some("8514"));
        assert_eq!(parsed.well_metadata.block_name.as_deref(), Some("F02"));
        assert_eq!(
            parsed
                .well_metadata
                .surface_location
                .as_ref()
                .and_then(|value| value.coordinate_reference.as_ref())
                .and_then(|value| value.id.as_deref()),
            Some("EPSG:23031")
        );
        assert_eq!(
            parsed.wellbore_metadata.status.as_deref(),
            Some("Sidetracked")
        );
        assert!(!parsed.detected_coordinate_references.is_empty());
    }

    #[test]
    fn parses_tops_with_preview_only_rows() {
        let path = write_temp_file(
            "lithostratigrafie.txt",
            r#"Top(m)	Bottom(m)	Strat Unit	Anomaly	Quality	Remark
	97.75	NU			
97.75	693	NUMS			
"#,
        );
        let parsed = parse_tops_source(&path).expect("tops should parse");
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(parsed.rows[0].top_depth, None);
        assert_eq!(parsed.rows[1].top_depth, Some(97.75));
        assert!(!parsed.issues.is_empty());
    }

    #[test]
    fn imports_tops_source_and_reports_omitted_rows() {
        let path = write_temp_file(
            "lithostratigrafie.txt",
            r#"Well Name: F02-A-05
Depth Ref Point: Kelly Bushing
Source: EINDSLIP

Top(m)	Bottom(m)	Strat Unit	Anomaly	Quality	Remark
	689.9	NN			
689.9	690	NUMS			
690	1209	NUOT			
"#,
        );
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let result = import_tops_source(
            &mut project,
            &path,
            &AssetBindingInput {
                well_name: "F02-A-05".to_string(),
                wellbore_name: "F02-A-05".to_string(),
                uwi: None,
                api: None,
                operator_aliases: Vec::new(),
            },
            Some("lithostrat-tops"),
            None,
        )
        .expect("tops source import should succeed");

        assert_eq!(result.source_row_count, 3);
        assert_eq!(result.imported_row_count, 2);
        assert_eq!(result.omitted_row_count, 1);
        assert_eq!(
            result.reported_depth_reference.as_deref(),
            Some("Kelly Bushing")
        );
        assert_eq!(
            result.resolved_source_depth_reference.as_deref(),
            Some("Kelly Bushing")
        );
        assert_eq!(result.resolved_depth_domain.as_deref(), Some("md"));
        assert_eq!(result.resolved_depth_datum.as_deref(), Some("kb"));
        assert_eq!(result.import_result.collection.name, "lithostrat-tops");
        let rows = project
            .read_tops(&result.import_result.asset.id)
            .expect("imported tops should be readable");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].source_depth_reference.as_deref(),
            Some("Kelly Bushing")
        );
        assert_eq!(rows[0].depth_domain.as_deref(), Some("md"));
        assert_eq!(rows[0].depth_datum.as_deref(), Some("kb"));
        assert_eq!(result.omissions.len(), 1);
        assert_eq!(
            result.omissions[0].reason_code,
            WellFolderImportOmissionReasonCode::TopsRowsIncomplete
        );
    }

    #[test]
    fn marks_sparse_deviation_as_not_committable() {
        let path = write_temp_file(
            "deviatie.txt",
            r#"Survey Points:
Depth(m)	Inclination(deg)	Azimuth(deg)	TVD(m)	X-offset(m)	Y-offset(m)
		226.42			
		145.8			
"#,
        );
        let parsed = parse_trajectory_source(&path).expect("trajectory should parse");
        assert_eq!(parsed.rows.len(), 2);
        assert!(!parsed.commit_enabled);
        assert_eq!(
            parsed.non_empty_column_count.get("azimuth_deg").copied(),
            Some(2)
        );
    }

    #[test]
    fn parses_nlog_ascii_preview() {
        let path = write_temp_file(
            "f02a02_lwd.asc",
            "DEPTH BDCX DRHX GRAX\n1433.00 -999.25 12.5 84.2\n1433.50 1.25 12.7 85.1\n",
        );
        let parsed = parse_ascii_log_source(&path).expect("ascii log should parse");
        assert_eq!(parsed.preview.row_count, 2);
        assert_eq!(parsed.preview.column_count, 4);
        assert_eq!(
            parsed.preview.default_depth_column.as_deref(),
            Some("DEPTH")
        );
        assert_eq!(
            parsed.preview.default_value_columns,
            vec!["BDCX".to_string(), "DRHX".to_string(), "GRAX".to_string()]
        );
        assert_eq!(
            parsed.preview.status,
            WellFolderImportStatus::ReadyForCommit
        );
    }

    #[test]
    fn previews_selected_well_sources_without_folder_scan() {
        let folder = write_temp_dir();
        let metadata_path = folder.join("basisgegevens.txt");
        let ascii_path = folder.join("f02a02_lwd.asc");
        fs::write(
            &metadata_path,
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            &ascii_path,
            "DEPTH BDCX DRHX GRAX\n1433.00 -999.25 12.5 84.2\n1433.50 1.25 12.7 85.1\n",
        )
        .expect("ascii log should be written");

        let preview = preview_well_import_sources(
            &[metadata_path.clone(), ascii_path.clone()],
            Some(&folder),
        )
        .expect("selected sources should preview");
        let metadata_source_path = path_string(
            &normalize_existing_file_path(&metadata_path).expect("path should normalize"),
        );

        assert_eq!(preview.logs.files.len(), 0);
        assert_eq!(preview.ascii_logs.files.len(), 1);
        assert_eq!(preview.binding.well_name, "F02-A-02");
        assert_eq!(
            preview.metadata.source_path.as_deref(),
            Some(metadata_source_path.as_str())
        );
        assert_eq!(
            preview.ascii_logs.files[0].source_path,
            path_string(&normalize_existing_file_path(&ascii_path).expect("path should normalize"))
        );
    }

    #[test]
    fn commit_imports_ascii_logs_with_original_source_provenance() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("basisgegevens.txt"),
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            folder.join("f02a02_lwd.asc"),
            "DEPTH BDCX DRHX GRAX\n1433.00 -999.25 12.5 84.2\n1433.50 1.25 12.7 85.1\n",
        )
        .expect("ascii log should be written");
        fs::write(folder.join("trace.dlis"), "").expect("unsupported source should be written");

        let preview = preview_well_folder_import(&folder).expect("preview should parse");
        let ascii = preview
            .ascii_logs
            .files
            .first()
            .expect("ascii preview should be present");
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: preview.binding.well_name,
                    wellbore_name: preview.binding.wellbore_name,
                    uwi: preview.binding.uwi,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: preview.metadata.well_metadata,
                wellbore_metadata: preview.metadata.wellbore_metadata,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Detected,
                    candidate_id: Some("EPSG:23031".to_string()),
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: false,
                import_trajectory: false,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: None,
                ascii_log_imports: Some(vec![WellFolderAsciiLogImportRequest {
                    source_path: ascii.source_path.clone(),
                    depth_column: "DEPTH".to_string(),
                    value_columns: vec![
                        WellFolderAsciiLogCurveMapping {
                            source_column: "BDCX".to_string(),
                            mnemonic: "BDCX".to_string(),
                            unit: Some("in".to_string()),
                        },
                        WellFolderAsciiLogCurveMapping {
                            source_column: "GRAX".to_string(),
                            mnemonic: "GR".to_string(),
                            unit: Some("gapi".to_string()),
                        },
                    ],
                    null_value: Some(-999.25),
                }]),
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        let asset = project
            .asset_by_id(&crate::AssetId(
                response.imported_assets[0].asset_id.clone(),
            ))
            .expect("asset should be readable");
        assert_eq!(asset.manifest.provenance.source_path, ascii.source_path);
        assert_eq!(asset.manifest.source_artifacts.len(), 3);
        assert!(
            Path::new(&asset.manifest.source_artifacts[0].source_path)
                .ends_with("basisgegevens.txt")
        );
        assert!(
            Path::new(&asset.manifest.source_artifacts[1].source_path).ends_with("f02a02_lwd.asc")
        );
        assert!(Path::new(&asset.manifest.source_artifacts[2].source_path).ends_with("trace.dlis"));
        assert!(
            asset
                .manifest
                .bulk_data_descriptors
                .iter()
                .any(|descriptor| descriptor.relative_path == "sources/basisgegevens.txt")
        );
        assert!(
            asset
                .manifest
                .bulk_data_descriptors
                .iter()
                .any(|descriptor| descriptor.relative_path == "sources/trace.dlis")
        );
    }

    #[test]
    fn commit_selected_well_sources_uses_explicit_paths() {
        let folder = write_temp_dir();
        let metadata_path = folder.join("basisgegevens.txt");
        let ascii_path = folder.join("f02a02_lwd.asc");
        fs::write(
            &metadata_path,
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            &ascii_path,
            "DEPTH BDCX DRHX GRAX\n1433.00 -999.25 12.5 84.2\n1433.50 1.25 12.7 85.1\n",
        )
        .expect("ascii log should be written");

        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: Some(vec![
                    metadata_path.to_string_lossy().into_owned(),
                    ascii_path.to_string_lossy().into_owned(),
                ]),
                binding: AssetBindingInput {
                    well_name: "F02-A-02".to_string(),
                    wellbore_name: "F02-A-02".to_string(),
                    uwi: Some("8514".to_string()),
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: None,
                wellbore_metadata: None,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Detected,
                    candidate_id: Some("EPSG:23031".to_string()),
                    coordinate_reference: None,
                },
                import_logs: true,
                selected_log_source_paths: Some(Vec::new()),
                import_tops_markers: false,
                import_trajectory: false,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: None,
                ascii_log_imports: Some(vec![WellFolderAsciiLogImportRequest {
                    source_path: path_string(&ascii_path),
                    depth_column: "DEPTH".to_string(),
                    value_columns: vec![WellFolderAsciiLogCurveMapping {
                        source_column: "GRAX".to_string(),
                        mnemonic: "GR".to_string(),
                        unit: Some("gapi".to_string()),
                    }],
                    null_value: Some(-999.25),
                }]),
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        assert_eq!(
            response.imported_assets[0].source_path,
            path_string(&normalize_existing_file_path(&ascii_path).expect("path should normalize"))
        );
    }

    #[test]
    fn previews_folder_from_supported_sources() {
        let root = write_temp_dir();
        fs::write(
            root.join("basisgegevens.txt"),
            r#"Well Name: F02-A-02
UWI: 8514
Block: F02
Field Name: F02a Hanze
X Coord (ED50 UTM31): 600791
Y Coord (ED50 UTM31): 6089995
DRP Datum: MSL
"#,
        )
        .expect("metadata should be written");
        fs::write(
            root.join("lithostratigrafie.txt"),
            "Top(m)\tBottom(m)\tStrat Unit\tAnomaly\tQuality\tRemark\n97.75\t693\tNUMS\t\t\t\n",
        )
        .expect("tops should be written");
        fs::write(
            root.join("deviatie.txt"),
            "Survey Points:\nDepth(m)\tInclination(deg)\tAzimuth(deg)\tTVD(m)\tX-offset(m)\tY-offset(m)\n\t\t226.42\t\t\t\n",
        )
        .expect("trajectory should be written");
        fs::write(root.join("trace.dlis"), "").expect("unsupported source should be written");

        let preview = preview_well_folder_import(&root).expect("folder preview should parse");
        assert_eq!(preview.binding.well_name, "F02-A-02");
        assert_eq!(
            preview
                .source_coordinate_reference
                .recommended_candidate_id
                .as_deref(),
            Some("EPSG:23031")
        );
        assert_eq!(preview.tops_markers.committable_row_count, 1);
        assert_eq!(
            preview.trajectory.status,
            WellFolderImportStatus::NotViableForCommit
        );
        assert_eq!(preview.unsupported_sources.len(), 1);
    }

    #[test]
    fn commit_preserves_supporting_source_artifacts() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("basisgegevens.txt"),
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            folder.join("lithostratigrafie.txt"),
            "Top(m)\tBottom(m)\tStrat Unit\tAnomaly\tQuality\tRemark\n97.75\t693\tNUMS\t\t\t\n",
        )
        .expect("tops should be written");
        fs::write(folder.join("trace.dlis"), "").expect("unsupported source should be written");

        let preview = preview_well_folder_import(&folder).expect("preview should parse");
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: preview.binding.well_name,
                    wellbore_name: preview.binding.wellbore_name,
                    uwi: preview.binding.uwi,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: preview.metadata.well_metadata,
                wellbore_metadata: preview.metadata.wellbore_metadata,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Detected,
                    candidate_id: Some("EPSG:23031".to_string()),
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: true,
                import_trajectory: false,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: None,
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        let asset = project
            .asset_by_id(&crate::AssetId(
                response.imported_assets[0].asset_id.clone(),
            ))
            .expect("asset should be readable");
        assert_eq!(asset.manifest.source_artifacts.len(), 3);
        assert!(
            asset
                .manifest
                .bulk_data_descriptors
                .iter()
                .any(|descriptor| descriptor.relative_path == "sources/basisgegevens.txt")
        );
        assert!(
            asset
                .manifest
                .bulk_data_descriptors
                .iter()
                .any(|descriptor| descriptor.relative_path == "sources/trace.dlis")
        );
        assert!(
            response
                .issues
                .iter()
                .any(|issue| issue.code == "unsupported.preserved")
        );
    }

    #[test]
    fn commit_preserves_unsupported_sources_in_raw_bundle_when_no_canonical_asset_imports() {
        let folder = write_temp_dir();
        fs::write(folder.join("trace.dlis"), "").expect("unsupported source should be written");

        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: "F02-A-02".to_string(),
                    wellbore_name: "F02-A-02".to_string(),
                    uwi: None,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: None,
                wellbore_metadata: None,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Unresolved,
                    candidate_id: None,
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: false,
                import_trajectory: false,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: None,
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        assert_eq!(response.imported_assets[0].asset_kind, "raw_source_bundle");
        let asset = project
            .asset_by_id(&crate::AssetId(
                response.imported_assets[0].asset_id.clone(),
            ))
            .expect("raw bundle asset should be readable");
        assert_eq!(asset.asset_kind, crate::AssetKind::RawSourceBundle);
        assert!(
            asset
                .manifest
                .bulk_data_descriptors
                .iter()
                .any(|descriptor| descriptor.relative_path == "sources/trace.dlis")
        );
        assert!(
            response
                .issues
                .iter()
                .any(|issue| issue.code == "unsupported.preserved_raw_bundle")
        );
        assert!(response.omissions.iter().any(|omission| {
            omission.kind == WellFolderImportOmissionKind::UnsupportedSources
                && omission.reason_code
                    == WellFolderImportOmissionReasonCode::UnsupportedPreservedAsRawBundle
        }));
    }

    #[test]
    fn unresolved_crs_omits_surface_location_and_skips_trajectory() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("basisgegevens.txt"),
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            folder.join("deviatie.txt"),
            "Survey Points:\nDepth(m)\tInclination(deg)\tAzimuth(deg)\tTVD(m)\tX-offset(m)\tY-offset(m)\n100\t0.1\t45\t99\t0\t0\n200\t1.1\t47\t198\t2\t3\n",
        )
        .expect("trajectory should be written");

        let preview = preview_well_folder_import(&folder).expect("preview should parse");
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: preview.binding.well_name,
                    wellbore_name: preview.binding.wellbore_name,
                    uwi: preview.binding.uwi,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: preview.metadata.well_metadata,
                wellbore_metadata: preview.metadata.wellbore_metadata,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Unresolved,
                    candidate_id: None,
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: false,
                import_trajectory: true,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: None,
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert!(response.imported_assets.is_empty());
        assert!(
            response
                .issues
                .iter()
                .any(|issue| issue.code == "trajectory.crs_unresolved")
        );
        assert!(response.omissions.iter().any(|omission| {
            omission.kind == WellFolderImportOmissionKind::SurfaceLocation
                && omission.reason_code == WellFolderImportOmissionReasonCode::SourceCrsUnresolved
        }));
        assert!(response.omissions.iter().any(|omission| {
            omission.kind == WellFolderImportOmissionKind::Trajectory
                && omission.reason_code == WellFolderImportOmissionReasonCode::SourceCrsUnresolved
        }));
        let wells = project.list_wells().expect("wells should be readable");
        assert_eq!(wells.len(), 1);
        assert_eq!(
            wells[0]
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.surface_location.as_ref()),
            None
        );
        assert!(
            wells[0]
                .metadata
                .as_ref()
                .map(|metadata| {
                    metadata
                        .notes
                        .iter()
                        .any(|note| note.contains("source CRS remains unresolved"))
                })
                .unwrap_or(false)
        );
    }

    #[test]
    fn trajectory_commit_persists_selected_coordinate_reference() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("basisgegevens.txt"),
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            folder.join("deviatie.txt"),
            "Survey Points:\nDepth(m)\tInclination(deg)\tAzimuth(deg)\tTVD(m)\tX-offset(m)\tY-offset(m)\n100\t0.1\t45\t99\t0\t0\n200\t1.1\t47\t198\t2\t3\n",
        )
        .expect("trajectory should be written");

        let preview = preview_well_folder_import(&folder).expect("preview should parse");
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: preview.binding.well_name,
                    wellbore_name: preview.binding.wellbore_name,
                    uwi: preview.binding.uwi,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: preview.metadata.well_metadata,
                wellbore_metadata: preview.metadata.wellbore_metadata,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Detected,
                    candidate_id: Some("EPSG:23031".to_string()),
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: false,
                import_trajectory: true,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: None,
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        let asset = project
            .asset_by_id(&crate::AssetId(
                response.imported_assets[0].asset_id.clone(),
            ))
            .expect("asset should be readable");
        assert_eq!(
            asset
                .manifest
                .reference_metadata
                .coordinate_reference
                .as_ref()
                .and_then(|reference| reference.id.as_deref()),
            Some("EPSG:23031")
        );
        assert_eq!(
            asset
                .manifest
                .reference_metadata
                .coordinate_reference
                .as_ref()
                .and_then(|reference| reference.name.as_deref()),
            Some("ED50 / UTM zone 31N")
        );
    }

    #[test]
    fn trajectory_commit_accepts_supplemented_station_rows_when_source_file_is_incomplete() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("basisgegevens.txt"),
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            folder.join("deviatie.txt"),
            "Survey Points:\nDepth(m)\tInclination(deg)\tAzimuth(deg)\tTVD(m)\tX-offset(m)\tY-offset(m)\n\t\t226.42\t\t\t\n\t\t145.8\t\t\t\n",
        )
        .expect("trajectory should be written");

        let preview = preview_well_folder_import(&folder).expect("preview should parse");
        assert!(!preview.trajectory.commit_enabled);
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: preview.binding.well_name,
                    wellbore_name: preview.binding.wellbore_name,
                    uwi: preview.binding.uwi,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: preview.metadata.well_metadata,
                wellbore_metadata: preview.metadata.wellbore_metadata,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Detected,
                    candidate_id: Some("EPSG:23031".to_string()),
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: false,
                import_trajectory: true,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: Some(vec![
                    WellFolderTrajectoryDraftRow {
                        measured_depth: Some(100.0),
                        inclination_deg: Some(0.5),
                        azimuth_deg: Some(226.42),
                        true_vertical_depth: None,
                        x_offset: None,
                        y_offset: None,
                    },
                    WellFolderTrajectoryDraftRow {
                        measured_depth: Some(200.0),
                        inclination_deg: Some(1.2),
                        azimuth_deg: Some(145.8),
                        true_vertical_depth: None,
                        x_offset: None,
                        y_offset: None,
                    },
                ]),
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        assert_eq!(response.imported_assets[0].asset_kind, "trajectory");
        let rows = project
            .read_trajectory_rows(
                &crate::AssetId(response.imported_assets[0].asset_id.clone()),
                None,
            )
            .expect("trajectory rows should be readable");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].measured_depth, 100.0);
        assert_eq!(rows[1].azimuth_deg, Some(145.8));
    }

    #[test]
    fn trajectory_commit_uses_row_overrides_even_when_source_file_is_already_viable() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("basisgegevens.txt"),
            "Well Name: F02-A-02\nUWI: 8514\nX Coord (ED50 UTM31): 600791\nY Coord (ED50 UTM31): 6089995\nDRP Datum: MSL\n",
        )
        .expect("metadata should be written");
        fs::write(
            folder.join("deviatie.txt"),
            "Survey Points:\nDepth(m)\tInclination(deg)\tAzimuth(deg)\tTVD(m)\tX-offset(m)\tY-offset(m)\n100\t0.1\t45\t99\t0\t0\n200\t1.1\t47\t198\t2\t3\n",
        )
        .expect("trajectory should be written");

        let preview = preview_well_folder_import(&folder).expect("preview should parse");
        assert!(preview.trajectory.commit_enabled);
        assert_eq!(preview.trajectory.draft_rows.len(), 2);
        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: preview.binding.well_name,
                    wellbore_name: preview.binding.wellbore_name,
                    uwi: preview.binding.uwi,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: preview.metadata.well_metadata,
                wellbore_metadata: preview.metadata.wellbore_metadata,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Detected,
                    candidate_id: Some("EPSG:23031".to_string()),
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: false,
                import_trajectory: true,
                tops_depth_reference: Some("md".to_string()),
                tops_rows: None,
                trajectory_rows: Some(vec![
                    WellFolderTrajectoryDraftRow {
                        measured_depth: Some(100.0),
                        inclination_deg: Some(0.1),
                        azimuth_deg: Some(55.0),
                        true_vertical_depth: Some(99.0),
                        x_offset: Some(0.0),
                        y_offset: Some(0.0),
                    },
                    WellFolderTrajectoryDraftRow {
                        measured_depth: Some(200.0),
                        inclination_deg: Some(1.1),
                        azimuth_deg: Some(57.0),
                        true_vertical_depth: Some(198.0),
                        x_offset: Some(2.0),
                        y_offset: Some(3.0),
                    },
                ]),
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        let rows = project
            .read_trajectory_rows(
                &crate::AssetId(response.imported_assets[0].asset_id.clone()),
                None,
            )
            .expect("trajectory rows should be readable");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].azimuth_deg, Some(55.0));
        assert_eq!(rows[1].azimuth_deg, Some(57.0));
    }

    #[test]
    fn commit_uses_tops_row_overrides() {
        let folder = write_temp_dir();
        fs::write(
            folder.join("lithostratigrafie.txt"),
            "Top(m)\tBottom(m)\tStrat Unit\tAnomaly\tQuality\tRemark\n97.75\t693\tNUMS\t\t\t\n",
        )
        .expect("tops should be written");

        let project_root = write_temp_dir();
        let mut project =
            OphioliteProject::create(&project_root).expect("project should be created");
        let response = commit_well_folder_import(
            &mut project,
            &ProjectWellFolderImportCommitRequest {
                folder_path: folder.to_string_lossy().into_owned(),
                source_paths: None,
                binding: AssetBindingInput {
                    well_name: "F02-A-02".to_string(),
                    wellbore_name: "F02-A-02".to_string(),
                    uwi: None,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                well_metadata: None,
                wellbore_metadata: None,
                source_coordinate_reference: WellFolderCoordinateReferenceSelection {
                    mode: WellFolderCoordinateReferenceSelectionMode::Unresolved,
                    candidate_id: None,
                    coordinate_reference: None,
                },
                import_logs: false,
                selected_log_source_paths: None,
                import_tops_markers: true,
                import_trajectory: false,
                tops_depth_reference: Some("tvd".to_string()),
                tops_rows: Some(vec![WellFolderTopDraftRow {
                    name: Some("Edited Top".to_string()),
                    top_depth: Some(123.45),
                    base_depth: Some(150.0),
                    anomaly: None,
                    quality: None,
                    note: None,
                }]),
                trajectory_rows: None,
                ascii_log_imports: None,
            },
        )
        .expect("commit should succeed");

        assert_eq!(response.imported_assets.len(), 1);
        let rows = project
            .read_tops(&crate::AssetId(
                response.imported_assets[0].asset_id.clone(),
            ))
            .expect("tops should be readable");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Edited Top");
        assert_eq!(rows[0].top_depth, 123.45);
        assert_eq!(rows[0].base_depth, Some(150.0));
        assert_eq!(rows[0].source_depth_reference.as_deref(), Some("tvd"));
        assert_eq!(rows[0].depth_domain.as_deref(), Some("tvd"));
        assert!(!response.omissions.iter().any(|omission| {
            omission.kind == WellFolderImportOmissionKind::TopsRows
                && omission.reason_code == WellFolderImportOmissionReasonCode::TopsRowsIncomplete
        }));
    }

    fn write_temp_file(name: &str, contents: &str) -> PathBuf {
        let root = write_temp_dir();
        let path = root.join(name);
        fs::write(&path, contents).expect("temp file should be written");
        path
    }

    fn write_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ophiolite-well-folder-import-{unique}"));
        fs::create_dir_all(&root).expect("temp dir should be created");
        root
    }
}
