use crate::asset::{CurveItem, HeaderItem, LasFile, LasFileSummary, LasValue};
use crate::metadata::{CanonicalMetadata, CurveInfo, IndexInfo};
use crate::{CanonicalAlias, CurveStorageKind, IngestIssue, LasError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const DTO_CONTRACT_VERSION: &str = "0.1.0";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReportDto {
    pub dto_contract_version: String,
    pub kind: ValidationKind,
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveCatalogEntryDto {
    pub curve_id: String,
    pub name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub row_count: usize,
    pub nullable: bool,
    pub storage_kind: CurveStorageKind,
    pub alias: CanonicalAlias,
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
    pub dirty: DirtyStateDto,
    pub summary: AssetSummaryDto,
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
pub struct CurveWindowColumnDto {
    pub name: String,
    pub storage_kind: CurveStorageKind,
    pub values: Vec<LasValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveWindowDto {
    pub start_row: usize,
    pub row_count: usize,
    pub columns: Vec<CurveWindowColumnDto>,
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
                name: curve.mnemonic.clone(),
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
        start_row: request.start_row,
        row_count: actual_row_count,
        columns,
    })
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

    refresh_summary(&mut candidate);
    *file = candidate;
    Ok(())
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

    Ok(())
}

pub fn validation_report_dto(file: &LasFile) -> ValidationReportDto {
    match validate_edit_state(file) {
        Ok(()) => ValidationReportDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            kind: ValidationKind::Edit,
            valid: true,
            errors: Vec::new(),
        },
        Err(LasError::Validation(message)) => ValidationReportDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            kind: ValidationKind::Edit,
            valid: false,
            errors: vec![message],
        },
        Err(other) => ValidationReportDto {
            dto_contract_version: String::from(DTO_CONTRACT_VERSION),
            kind: ValidationKind::Edit,
            valid: false,
            errors: vec![other.to_string()],
        },
    }
}

fn catalog_entry(curve: &CurveInfo, item: &CurveItem) -> CurveCatalogEntryDto {
    CurveCatalogEntryDto {
        curve_id: stable_curve_id(&curve.name),
        name: curve.name.clone(),
        original_mnemonic: curve.original_mnemonic.clone(),
        unit: curve.unit.clone(),
        description: curve.description.clone(),
        row_count: item.data.len(),
        nullable: curve.nullable,
        storage_kind: curve.storage_kind,
        alias: curve.alias.clone(),
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
    has_unsaved_changes: bool,
    summary: AssetSummaryDto,
) -> SessionSummaryDto {
    SessionSummaryDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        package_id: package_id.clone(),
        session_id: session_id.clone(),
        revision: revision.clone(),
        dirty: dirty_state_dto(package_id, session_id, revision, has_unsaved_changes),
        summary,
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
    ValidationReportDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        kind: ValidationKind::Package,
        valid: errors.is_empty(),
        errors,
    }
}

pub fn save_validation_report(errors: Vec<String>) -> ValidationReportDto {
    ValidationReportDto {
        dto_contract_version: String::from(DTO_CONTRACT_VERSION),
        kind: ValidationKind::Save,
        valid: errors.is_empty(),
        errors,
    }
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

fn stable_curve_id(name: &str) -> String {
    stable_id("curve", name)
}

fn stable_id(scope: &str, value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    value.hash(&mut hasher);
    format!("{scope}-{:016x}", hasher.finish())
}
