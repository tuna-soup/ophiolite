use crate::asset::{CurveItem, HeaderItem, LasFile, LasFileSummary, LasValue};
use crate::metadata::{CanonicalMetadata, CurveInfo, IndexInfo};
use crate::{CanonicalAlias, CurveStorageKind, IngestIssue, LasError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSummaryDto {
    pub summary: LasFileSummary,
    pub encoding: Option<String>,
    pub index: IndexInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePackageResultDto {
    pub root: String,
    pub overwritten: bool,
    pub summary: AssetSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    pub metadata: CanonicalMetadata,
    pub issues: Vec<IngestIssue>,
    pub extra_sections: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReportDto {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveCatalogEntryDto {
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
    for item in &request.items {
        let header = HeaderItem::new(
            item.mnemonic.clone(),
            item.unit.clone(),
            item.value.clone(),
            item.description.clone(),
        );
        match item.section {
            MetadataSectionDto::Version => file.version.set_item(&item.mnemonic, header),
            MetadataSectionDto::Well => file.well.set_item(&item.mnemonic, header),
            MetadataSectionDto::Parameters => file.params.set_item(&item.mnemonic, header),
        }
    }

    if let Some(other) = &request.other {
        file.other = other.clone();
    }

    refresh_summary(file);
    Ok(())
}

pub fn apply_curve_edit(file: &mut LasFile, request: &CurveEditRequest) -> Result<()> {
    match request {
        CurveEditRequest::Upsert(update) => {
            let expected_row_count = file.row_count();
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
            file.replace_curve_item(&update.mnemonic, curve);
        }
        CurveEditRequest::Remove { mnemonic } => {
            if file.index.curve_id == *mnemonic {
                return Err(LasError::Validation(format!(
                    "cannot remove index curve '{}'",
                    mnemonic
                )));
            }
            file.delete_curve_by_mnemonic(mnemonic).ok_or_else(|| {
                LasError::Validation(format!("curve '{mnemonic}' not found in LAS file"))
            })?;
        }
    }

    validate_edit_state(file)?;
    refresh_summary(file);
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
            valid: true,
            errors: Vec::new(),
        },
        Err(LasError::Validation(message)) => ValidationReportDto {
            valid: false,
            errors: vec![message],
        },
        Err(other) => ValidationReportDto {
            valid: false,
            errors: vec![other.to_string()],
        },
    }
}

fn catalog_entry(curve: &CurveInfo, item: &CurveItem) -> CurveCatalogEntryDto {
    CurveCatalogEntryDto {
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
