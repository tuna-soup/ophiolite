use crate::asset::{CurveItem, HeaderItem, LasFile, LasFileSummary, LasValue};
use crate::metadata::{CanonicalMetadata, CurveInfo, IndexInfo, validate_canonical_metadata};
use crate::{CanonicalAlias, CurveStorageKind, IngestIssue, IssueSeverity, LasError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const DTO_CONTRACT_VERSION: &str = "0.3.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RevisionToken(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSummaryDto {
    pub dto_contract_version: String,
    pub summary: LasFileSummary,
    pub encoding: Option<String>,
    pub index: IndexInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePackageResultDto {
    pub dto_contract_version: String,
    pub package_id: PackageId,
    pub session_id: SessionId,
    pub revision: RevisionToken,
    pub root: String,
    pub overwritten: bool,
    pub dirty_cleared: bool,
    pub summary: AssetSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    pub dto_contract_version: String,
    pub metadata: CanonicalMetadata,
    pub issues: Vec<IngestIssue>,
    pub extra_sections: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationKind {
    Package,
    Edit,
    Save,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticTargetKind {
    Package,
    Session,
    Curve,
    Field,
    Path,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticTargetDto {
    pub kind: DiagnosticTargetKind,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticIssueDto {
    pub code: String,
    pub severity: IssueSeverity,
    pub message: String,
    pub target: Option<DiagnosticTargetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReportDto {
    pub dto_contract_version: String,
    pub kind: ValidationKind,
    pub valid: bool,
    pub errors: Vec<String>,
    pub issues: Vec<DiagnosticIssueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveCatalogEntryDto {
    pub curve_id: String,
    pub name: String,
    pub canonical_name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub row_count: usize,
    pub nullable: bool,
    pub storage_kind: CurveStorageKind,
    pub alias: CanonicalAlias,
    pub is_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirtyStateDto {
    pub dto_contract_version: String,
    pub package_id: PackageId,
    pub session_id: SessionId,
    pub revision: RevisionToken,
    pub has_unsaved_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryDto {
    pub dto_contract_version: String,
    pub package_id: PackageId,
    pub session_id: SessionId,
    pub revision: RevisionToken,
    pub root: String,
    pub dirty: DirtyStateDto,
    pub summary: AssetSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContextDto {
    pub dto_contract_version: String,
    pub package_id: PackageId,
    pub session_id: SessionId,
    pub revision: RevisionToken,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadataDto {
    pub dto_contract_version: String,
    pub session: SessionContextDto,
    pub metadata: MetadataDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveCatalogDto {
    pub dto_contract_version: String,
    pub session: SessionContextDto,
    pub curves: Vec<CurveCatalogEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConflictDto {
    pub dto_contract_version: String,
    pub package_id: PackageId,
    pub session_id: SessionId,
    pub expected_revision: RevisionToken,
    pub actual_revision: RevisionToken,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandGroup {
    Inspect,
    Session,
    EditPersist,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandErrorKind {
    OpenFailed,
    ValidationFailed,
    SaveConflict,
    SessionNotFound,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandErrorDto {
    pub dto_contract_version: String,
    pub group: CommandGroup,
    pub kind: CommandErrorKind,
    pub message: String,
    pub session_id: Option<SessionId>,
    pub validation: Option<ValidationReportDto>,
    pub save_conflict: Option<SaveConflictDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResponse<T> {
    Ok(T),
    Err(CommandErrorDto),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaveSessionResponseDto {
    Saved(SavePackageResultDto),
    Conflict(SaveConflictDto),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSessionResultDto {
    pub dto_contract_version: String,
    pub package_id: PackageId,
    pub session_id: SessionId,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWindowRequest {
    pub session_id: SessionId,
    pub window: CurveWindowRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDepthWindowRequest {
    pub session_id: SessionId,
    pub window: DepthWindowRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLasWindowRequest {
    pub path: String,
    pub window: CurveWindowRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadataEditRequest {
    pub session_id: SessionId,
    pub update: MetadataUpdateRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCurveEditRequest {
    pub session_id: SessionId,
    pub edit: CurveEditRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSaveAsRequest {
    pub session_id: SessionId,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveWindowRequest {
    pub curve_names: Vec<String>,
    pub start_row: usize,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthWindowRequest {
    pub curve_names: Vec<String>,
    pub depth_min: f64,
    pub depth_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveWindowColumnDto {
    pub curve_id: String,
    pub name: String,
    pub canonical_name: String,
    pub is_index: bool,
    pub storage_kind: CurveStorageKind,
    pub values: Vec<LasValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveWindowDto {
    pub dto_contract_version: String,
    pub start_row: usize,
    pub row_count: usize,
    pub columns: Vec<CurveWindowColumnDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWindowDto {
    pub dto_contract_version: String,
    pub session: SessionContextDto,
    pub window: CurveWindowDto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetadataSectionDto {
    Version,
    Well,
    Parameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderItemUpdate {
    pub section: MetadataSectionDto,
    pub mnemonic: String,
    pub unit: String,
    pub value: LasValue,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataUpdateRequest {
    pub items: Vec<HeaderItemUpdate>,
    pub other: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveUpdateRequest {
    pub mnemonic: String,
    pub original_mnemonic: Option<String>,
    pub unit: String,
    pub header_value: LasValue,
    pub description: String,
    pub data: Vec<LasValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveEditRequest {
    Upsert(CurveUpdateRequest),
    Remove { mnemonic: String },
}

pub fn asset_summary_dto(file: &LasFile) -> AssetSummaryDto {
    AssetSummaryDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        summary: current_summary(file),
        encoding: file.encoding.clone(),
        index: file.index_info(),
    }
}

pub fn metadata_dto(file: &LasFile) -> MetadataDto {
    MetadataDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        metadata: file.metadata(),
        issues: file.issues.clone(),
        extra_sections: file.extra_sections.clone(),
    }
}

pub fn curve_catalog_dto(file: &LasFile) -> Vec<CurveCatalogEntryDto> {
    file.curve_infos()
        .into_iter()
        .zip(file.curves.iter())
        .map(|(curve, item)| catalog_entry(&curve, item))
        .collect()
}

pub fn curve_window_dto(file: &LasFile, request: &CurveWindowRequest) -> Result<CurveWindowDto> {
    if request.curve_names.is_empty() {
        return Err(LasError::Validation(String::from(
            "curve window request must include at least one curve",
        )));
    }

    let end_row = request.start_row.saturating_add(request.row_count);
    let columns = request
        .curve_names
        .iter()
        .map(|name| {
            let curve = file.curves.get(name).ok_or_else(|| {
                LasError::Validation(format!("curve '{name}' not found in LAS file"))
            })?;
            let descriptor = file
                .curve_infos()
                .into_iter()
                .find(|info| info.name == curve.mnemonic)
                .ok_or_else(|| {
                    LasError::Validation(format!(
                        "curve descriptor '{}' missing from canonical metadata",
                        curve.mnemonic
                    ))
                })?;
            Ok(CurveWindowColumnDto {
                curve_id: stable_curve_id(&curve.mnemonic),
                name: curve.mnemonic.clone(),
                canonical_name: descriptor.canonical_name.clone(),
                is_index: descriptor.canonical_name == "index",
                storage_kind: descriptor.storage_kind,
                values: curve
                    .data
                    .iter()
                    .skip(request.start_row)
                    .take(end_row.saturating_sub(request.start_row))
                    .cloned()
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let actual_row_count = columns
        .first()
        .map(|column| column.values.len())
        .unwrap_or(0);
    Ok(CurveWindowDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        start_row: request.start_row,
        row_count: actual_row_count,
        columns,
    })
}

pub fn curve_depth_window_dto(
    file: &LasFile,
    request: &DepthWindowRequest,
) -> Result<CurveWindowDto> {
    let index_curve = file.curve(&file.index.curve_id)?;
    let row_window =
        depth_window_request_for_values(&file.index.curve_id, &index_curve.data, request)?;
    curve_window_dto(file, &row_window)
}

pub fn apply_metadata_update(file: &mut LasFile, request: &MetadataUpdateRequest) -> Result<()> {
    let mut candidate = file.clone();
    for item in &request.items {
        let header = HeaderItem::new(
            item.mnemonic.clone(),
            item.unit.clone(),
            item.value.clone(),
            item.description.clone(),
        );
        match item.section {
            MetadataSectionDto::Version => candidate.version.set_item(&item.mnemonic, header),
            MetadataSectionDto::Well => candidate.well.set_item(&item.mnemonic, header),
            MetadataSectionDto::Parameters => candidate.params.set_item(&item.mnemonic, header),
        }
    }

    if let Some(other) = &request.other {
        candidate.other = other.clone();
    }

    validate_edit_state(&candidate)?;
    refresh_summary(&mut candidate);
    *file = candidate;
    Ok(())
}

pub fn depth_window_request_for_values(
    index_name: &str,
    index_values: &[LasValue],
    request: &DepthWindowRequest,
) -> Result<CurveWindowRequest> {
    if request.depth_min > request.depth_max {
        return Err(LasError::Validation(format!(
            "depth window for index '{index_name}' requires depth_min <= depth_max"
        )));
    }

    let numeric_values = index_values
        .iter()
        .map(|value| match value {
            LasValue::Number(number) if number.is_finite() => Ok(*number),
            LasValue::Number(_) | LasValue::Empty | LasValue::Text(_) => Err(LasError::Validation(
                format!(
                    "depth window for index '{index_name}' requires a finite monotonic numeric index"
                ),
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    let order = detect_monotonic_order(&numeric_values).ok_or_else(|| {
        LasError::Validation(format!(
            "depth window for index '{index_name}' requires a monotonic numeric index"
        ))
    })?;

    let (start_row, row_count) = depth_bounds_to_row_window(&numeric_values, request, order);
    Ok(CurveWindowRequest {
        curve_names: request.curve_names.clone(),
        start_row,
        row_count,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonotonicOrder {
    Ascending,
    Descending,
}

fn detect_monotonic_order(values: &[f64]) -> Option<MonotonicOrder> {
    if values.len() < 2 {
        return Some(MonotonicOrder::Ascending);
    }

    let non_decreasing = values.windows(2).all(|pair| pair[0] <= pair[1]);
    if non_decreasing {
        return Some(MonotonicOrder::Ascending);
    }

    let non_increasing = values.windows(2).all(|pair| pair[0] >= pair[1]);
    if non_increasing {
        return Some(MonotonicOrder::Descending);
    }

    None
}

fn depth_bounds_to_row_window(
    values: &[f64],
    request: &DepthWindowRequest,
    order: MonotonicOrder,
) -> (usize, usize) {
    match order {
        MonotonicOrder::Ascending => {
            let start_row = lower_bound(values, request.depth_min, |value, bound| value < bound);
            let end_row = lower_bound(values, request.depth_max, |value, bound| value <= bound);
            (start_row, end_row.saturating_sub(start_row))
        }
        MonotonicOrder::Descending => {
            let start_row = lower_bound(values, request.depth_max, |value, bound| value > bound);
            let end_row = lower_bound(values, request.depth_min, |value, bound| value >= bound);
            (start_row, end_row.saturating_sub(start_row))
        }
    }
}

fn lower_bound(values: &[f64], bound: f64, pred: impl Fn(f64, f64) -> bool) -> usize {
    let mut left = 0usize;
    let mut right = values.len();
    while left < right {
        let mid = left + (right - left) / 2;
        if pred(values[mid], bound) {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    left
}

pub fn apply_curve_edit(file: &mut LasFile, request: &CurveEditRequest) -> Result<()> {
    let mut candidate = file.clone();
    match request {
        CurveEditRequest::Upsert(update) => {
            let expected_row_count = candidate.row_count();
            if expected_row_count != 0 && update.data.len() != expected_row_count {
                return Err(LasError::Validation(format!(
                    "curve '{}' has {} rows but LAS file expects {}",
                    update.mnemonic,
                    update.data.len(),
                    expected_row_count
                )));
            }

            let mut curve = CurveItem::new(
                update
                    .original_mnemonic
                    .clone()
                    .unwrap_or_else(|| update.mnemonic.clone()),
                update.unit.clone(),
                update.header_value.clone(),
                update.description.clone(),
                update.data.clone(),
            );
            curve.rename(&update.mnemonic);
            candidate.replace_curve_item(&update.mnemonic, curve);
        }
        CurveEditRequest::Remove { mnemonic } => {
            if candidate.index.curve_id == *mnemonic {
                return Err(LasError::Validation(format!(
                    "cannot remove index curve '{}'",
                    mnemonic
                )));
            }
            candidate
                .delete_curve_by_mnemonic(mnemonic)
                .ok_or_else(|| {
                    LasError::Validation(format!("curve '{mnemonic}' not found in LAS file"))
                })?;
        }
    }

    validate_edit_state(&candidate)?;
    refresh_summary(&mut candidate);
    *file = candidate;
    Ok(())
}

pub fn validate_edit_state(file: &LasFile) -> Result<()> {
    if file.curves.is_empty() {
        return Err(LasError::Validation(String::from(
            "LAS file must contain at least one curve",
        )));
    }

    if !file.curves.contains(&file.index.curve_id) {
        return Err(LasError::Validation(format!(
            "index curve '{}' is missing from LAS curves",
            file.index.curve_id
        )));
    }

    let expected_row_count = file.row_count();
    for curve in file.curves.iter() {
        if curve.data.len() != expected_row_count {
            return Err(LasError::Validation(format!(
                "curve '{}' has {} rows but expected {}",
                curve.mnemonic,
                curve.data.len(),
                expected_row_count
            )));
        }
    }

    validate_canonical_metadata(file)?;
    Ok(())
}

pub fn validation_report_dto(file: &LasFile) -> ValidationReportDto {
    match validate_edit_state(file) {
        Ok(()) => empty_validation_report(ValidationKind::Edit),
        Err(LasError::Validation(message)) => {
            validation_report_from_messages(ValidationKind::Edit, vec![message])
        }
        Err(other) => {
            validation_report_from_messages(ValidationKind::Edit, vec![other.to_string()])
        }
    }
}

fn catalog_entry(curve: &CurveInfo, item: &CurveItem) -> CurveCatalogEntryDto {
    CurveCatalogEntryDto {
        curve_id: stable_curve_id(&curve.name),
        name: curve.name.clone(),
        canonical_name: curve.canonical_name.clone(),
        original_mnemonic: curve.original_mnemonic.clone(),
        unit: curve.unit.clone(),
        description: curve.description.clone(),
        row_count: item.data.len(),
        nullable: curve.nullable,
        storage_kind: curve.storage_kind,
        alias: curve.alias.clone(),
        is_index: curve.canonical_name == "index",
    }
}

fn refresh_summary(file: &mut LasFile) {
    file.summary.row_count = file.row_count();
    file.summary.curve_count = file.curves.len();
    file.summary.issue_count = file.issues.len();
}

fn current_summary(file: &LasFile) -> LasFileSummary {
    let mut summary = file.summary.clone();
    summary.row_count = file.row_count();
    summary.curve_count = file.curves.len();
    summary.issue_count = file.issues.len();
    summary
}

pub fn package_id_for_path(path: &str) -> PackageId {
    PackageId(stable_id("pkg", path))
}

pub fn session_id_for_path(path: &str) -> SessionId {
    SessionId(stable_id("session", path))
}

pub fn revision_token_for_bytes(scope: &str, payload: &str) -> RevisionToken {
    RevisionToken(stable_id(scope, payload))
}

pub fn dirty_state_dto(
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    has_unsaved_changes: bool,
) -> DirtyStateDto {
    DirtyStateDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id,
        session_id,
        revision,
        has_unsaved_changes,
    }
}

pub fn session_summary_dto(
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    root: String,
    has_unsaved_changes: bool,
    summary: AssetSummaryDto,
) -> SessionSummaryDto {
    SessionSummaryDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id: package_id.clone(),
        session_id: session_id.clone(),
        revision: revision.clone(),
        root,
        dirty: dirty_state_dto(package_id, session_id, revision, has_unsaved_changes),
        summary,
    }
}

pub fn session_context_dto(
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    root: String,
) -> SessionContextDto {
    SessionContextDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id,
        session_id,
        revision,
        root,
    }
}

pub fn session_metadata_dto(
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    root: String,
    metadata: MetadataDto,
) -> SessionMetadataDto {
    SessionMetadataDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        session: session_context_dto(package_id, session_id, revision, root),
        metadata,
    }
}

pub fn curve_catalog_result_dto(
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    root: String,
    curves: Vec<CurveCatalogEntryDto>,
) -> CurveCatalogDto {
    CurveCatalogDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        session: session_context_dto(package_id, session_id, revision, root),
        curves,
    }
}

pub fn session_window_dto(
    package_id: PackageId,
    session_id: SessionId,
    revision: RevisionToken,
    root: String,
    window: CurveWindowDto,
) -> SessionWindowDto {
    SessionWindowDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        session: session_context_dto(package_id, session_id, revision, root),
        window,
    }
}

pub fn save_conflict_dto(
    package_id: PackageId,
    session_id: SessionId,
    expected_revision: RevisionToken,
    actual_revision: RevisionToken,
    path: String,
) -> SaveConflictDto {
    SaveConflictDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id,
        session_id,
        expected_revision,
        actual_revision,
        path,
    }
}

pub fn package_validation_report(errors: Vec<String>) -> ValidationReportDto {
    validation_report_from_messages(ValidationKind::Package, errors)
}

pub fn save_validation_report(errors: Vec<String>) -> ValidationReportDto {
    validation_report_from_messages(ValidationKind::Save, errors)
}

pub fn empty_validation_report(kind: ValidationKind) -> ValidationReportDto {
    ValidationReportDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        kind,
        valid: true,
        errors: Vec::new(),
        issues: Vec::new(),
    }
}

pub fn validation_report_from_messages(
    kind: ValidationKind,
    messages: Vec<String>,
) -> ValidationReportDto {
    let issues = messages
        .into_iter()
        .map(|message| validation_issue_for_message(kind, message))
        .collect::<Vec<_>>();
    validation_report_from_issues(kind, issues)
}

pub fn validation_report_from_issues(
    kind: ValidationKind,
    issues: Vec<DiagnosticIssueDto>,
) -> ValidationReportDto {
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    ValidationReportDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        kind,
        valid: errors.is_empty(),
        errors,
        issues,
    }
}

pub fn diagnostic_target_dto(
    kind: DiagnosticTargetKind,
    value: impl Into<String>,
) -> DiagnosticTargetDto {
    DiagnosticTargetDto {
        kind,
        value: value.into(),
    }
}

pub fn diagnostic_issue_dto(
    code: impl Into<String>,
    severity: IssueSeverity,
    message: impl Into<String>,
    target: Option<DiagnosticTargetDto>,
) -> DiagnosticIssueDto {
    DiagnosticIssueDto {
        code: code.into(),
        severity,
        message: message.into(),
        target,
    }
}

pub fn validation_issue_for_message(
    kind: ValidationKind,
    message: impl Into<String>,
) -> DiagnosticIssueDto {
    let message = message.into();
    let (code, target) = match message.as_str() {
        "LAS file must contain at least one curve" => (
            "curve.missing_any",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Package,
                "las-file",
            )),
        ),
        "curve mnemonics must not be empty" => (
            "curve.mnemonic.empty",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "curve.mnemonic",
            )),
        ),
        "index descriptor must reference a curve id" => (
            "index.curve_id.missing",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "index.curve_id",
            )),
        ),
        "index descriptor must preserve the original mnemonic" => (
            "index.original_mnemonic.missing",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "index.raw_mnemonic",
            )),
        ),
        "package metadata schema version must not be empty" => (
            "package.schema_version.empty",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "package.metadata_schema_version",
            )),
        ),
        "package storage metadata must mark exactly one index column" => (
            "package.index_column.missing",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "storage.curve_columns",
            )),
        ),
        "package storage metadata must contain exactly one index column" => (
            "package.index_column.invalid",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "storage.curve_columns",
            )),
        ),
        other if other.starts_with("curve '") && other.contains("' not found in LAS file") => (
            "curve.not_found",
            extract_quoted_target(other, "curve '", DiagnosticTargetKind::Curve),
        ),
        other
            if other.starts_with("curve '")
                && (other.contains("rows but LAS file expects")
                    || other.contains("rows but expected")) =>
        {
            (
                "curve.row_count_mismatch",
                extract_quoted_target(other, "curve '", DiagnosticTargetKind::Curve),
            )
        }
        other
            if other.starts_with("curve column '")
                && other.contains("rows but summary expects") =>
        {
            (
                "storage.curve_row_count_mismatch",
                extract_quoted_target(other, "curve column '", DiagnosticTargetKind::Curve),
            )
        }
        other
            if other.starts_with("canonical curve '")
                && other.contains("rows but summary expects") =>
        {
            (
                "canonical.curve_row_count_mismatch",
                extract_quoted_target(other, "canonical curve '", DiagnosticTargetKind::Curve),
            )
        }
        other
            if other.starts_with("canonical curve '")
                && other.contains("is missing from storage columns") =>
        {
            (
                "canonical.curve_missing_from_storage",
                extract_quoted_target(other, "canonical curve '", DiagnosticTargetKind::Curve),
            )
        }
        other
            if other.starts_with("index curve '")
                && other.contains("is missing from LAS curves") =>
        {
            (
                "index.missing_curve",
                extract_quoted_target(other, "index curve '", DiagnosticTargetKind::Curve),
            )
        }
        other if other.starts_with("index curve '") && other.contains("must remain numeric") => (
            "index.must_be_numeric",
            extract_quoted_target(other, "index curve '", DiagnosticTargetKind::Curve),
        ),
        other if other.starts_with("cannot remove index curve '") => (
            "index.remove_forbidden",
            extract_quoted_target(
                other,
                "cannot remove index curve '",
                DiagnosticTargetKind::Curve,
            ),
        ),
        other
            if other.starts_with("canonical index '")
                && other.contains("does not match storage index") =>
        {
            (
                "package.index.mismatch",
                Some(diagnostic_target_dto(
                    DiagnosticTargetKind::Field,
                    "storage.index",
                )),
            )
        }
        other
            if other.starts_with("canonical index mnemonic '")
                && other.contains("does not match storage index mnemonic") =>
        {
            (
                "package.index_mnemonic.mismatch",
                Some(diagnostic_target_dto(
                    DiagnosticTargetKind::Field,
                    "storage.index.raw_mnemonic",
                )),
            )
        }
        other
            if other.starts_with("package metadata declares")
                && other.contains("curve columns but summary expects") =>
        {
            (
                "package.curve_count.mismatch",
                Some(diagnostic_target_dto(
                    DiagnosticTargetKind::Field,
                    "storage.curve_columns",
                )),
            )
        }
        other
            if other.starts_with("canonical metadata declares")
                && other.contains("curves but summary expects") =>
        {
            (
                "canonical.curve_count.mismatch",
                Some(diagnostic_target_dto(
                    DiagnosticTargetKind::Field,
                    "canonical.curves",
                )),
            )
        }
        other
            if other.starts_with("canonical index row count")
                && other.contains("does not match summary row count") =>
        {
            (
                "canonical.index_row_count.mismatch",
                Some(diagnostic_target_dto(
                    DiagnosticTargetKind::Field,
                    "canonical.index.row_count",
                )),
            )
        }
        other
            if other.starts_with("index column '")
                && other.contains("does not match storage index") =>
        {
            (
                "package.index_column.mismatch",
                extract_quoted_target(other, "index column '", DiagnosticTargetKind::Curve),
            )
        }
        other if other.starts_with("output directory '") && other.contains("already exists") => (
            "save.output_dir.exists",
            extract_quoted_target(other, "output directory '", DiagnosticTargetKind::Path),
        ),
        other if other.starts_with("unsupported package version ") => (
            "package.version.unsupported",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Field,
                "package.package_version",
            )),
        ),
        other if other.starts_with("invalid package metadata: ") => (
            "package.metadata.invalid",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Package,
                "metadata.json",
            )),
        ),
        other if other.contains("changed since session '") && other.contains("' was opened") => (
            "package.changed_since_session_open",
            Some(diagnostic_target_dto(
                DiagnosticTargetKind::Session,
                "session",
            )),
        ),
        other => (
            default_validation_code(kind),
            default_validation_target(kind, other),
        ),
    };

    diagnostic_issue_dto(code, IssueSeverity::Error, message, target)
}

pub fn close_session_result_dto(
    package_id: PackageId,
    session_id: SessionId,
    closed: bool,
) -> CloseSessionResultDto {
    CloseSessionResultDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id,
        session_id,
        closed,
    }
}

pub fn command_error_dto(
    group: CommandGroup,
    kind: CommandErrorKind,
    message: impl Into<String>,
) -> CommandErrorDto {
    CommandErrorDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        group,
        kind,
        message: message.into(),
        session_id: None,
        validation: None,
        save_conflict: None,
    }
}

fn stable_curve_id(name: &str) -> String {
    stable_id("curve", name)
}

fn extract_quoted_target(
    message: &str,
    prefix: &str,
    kind: DiagnosticTargetKind,
) -> Option<DiagnosticTargetDto> {
    let suffix = message.strip_prefix(prefix)?;
    let value = suffix.split_once('\'')?.0;
    Some(diagnostic_target_dto(kind, value))
}

fn default_validation_code(kind: ValidationKind) -> &'static str {
    match kind {
        ValidationKind::Package => "package.invalid",
        ValidationKind::Edit => "edit.invalid",
        ValidationKind::Save => "save.invalid",
    }
}

fn default_validation_target(kind: ValidationKind, _message: &str) -> Option<DiagnosticTargetDto> {
    match kind {
        ValidationKind::Package => Some(diagnostic_target_dto(
            DiagnosticTargetKind::Package,
            "package",
        )),
        ValidationKind::Edit => Some(diagnostic_target_dto(
            DiagnosticTargetKind::Session,
            "session",
        )),
        ValidationKind::Save => Some(diagnostic_target_dto(
            DiagnosticTargetKind::Path,
            "package-root",
        )),
    }
}

fn stable_id(scope: &str, value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    value.hash(&mut hasher);
    format!("{scope}-{:016x}", hasher.finish())
}
