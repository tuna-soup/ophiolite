use crate::asset::{
    CanonicalAlias, HeaderItem, IngestIssue, LasFile, LasFileSummary, LasValue, Provenance,
    SectionItems, derive_canonical_alias,
};
use crate::{CurveStorageKind, IndexDescriptor, IndexKind, LasError, MnemonicCase, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PACKAGE_METADATA_SCHEMA_VERSION: &str = "0.2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub vers: Option<String>,
    pub wrap: Option<String>,
    pub delimiter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellInfo {
    pub well: Option<String>,
    pub company: Option<String>,
    pub field: Option<String>,
    pub location: Option<String>,
    pub province: Option<String>,
    pub service_company: Option<String>,
    pub date: Option<String>,
    pub uwi: Option<String>,
    pub api: Option<String>,
    pub start: Option<f64>,
    pub stop: Option<f64>,
    pub step: Option<f64>,
    pub null_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub original_mnemonic: String,
    pub canonical_name: String,
    pub unit: Option<String>,
    pub kind: IndexKind,
    pub row_count: usize,
    pub nullable: bool,
    pub storage_kind: CurveStorageKind,
    pub alias: CanonicalAlias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveInfo {
    pub name: String,
    pub original_mnemonic: String,
    pub canonical_name: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub header_value: Option<String>,
    pub nullable: bool,
    pub storage_kind: CurveStorageKind,
    pub row_count: usize,
    pub alias: CanonicalAlias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveColumnMetadata {
    pub name: String,
    pub canonical_name: String,
    pub original_mnemonic: String,
    pub unit: String,
    pub header_value: LasValue,
    pub description: String,
    pub storage_kind: CurveStorageKind,
    pub row_count: usize,
    pub nullable: bool,
    pub alias: CanonicalAlias,
    pub is_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalMetadata {
    pub version: VersionInfo,
    pub well: WellInfo,
    pub index: IndexInfo,
    pub curves: Vec<CurveInfo>,
    pub parameters: Vec<ParameterInfo>,
    pub other: Option<String>,
    pub issue_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMetadataSections {
    pub version: SectionItems<HeaderItem>,
    pub well: SectionItems<HeaderItem>,
    pub params: SectionItems<HeaderItem>,
    pub other: String,
    pub extra_sections: BTreeMap<String, String>,
    pub curve_mnemonic_case: MnemonicCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIdentityMetadata {
    pub package_version: u32,
    pub metadata_schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDocumentMetadata {
    pub summary: LasFileSummary,
    pub provenance: Provenance,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageStorageMetadata {
    pub index: IndexDescriptor,
    pub index_unit: Option<String>,
    pub curve_columns: Vec<CurveColumnMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDiagnosticsMetadata {
    pub issues: Vec<IngestIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub package: PackageIdentityMetadata,
    pub document: PackageDocumentMetadata,
    pub canonical: CanonicalMetadata,
    pub storage: PackageStorageMetadata,
    pub raw: RawMetadataSections,
    pub diagnostics: PackageDiagnosticsMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyPackageMetadata {
    pub package_version: u32,
    pub metadata_schema_version: String,
    pub summary: LasFileSummary,
    pub provenance: Provenance,
    pub encoding: Option<String>,
    pub index: IndexDescriptor,
    pub canonical: CanonicalMetadata,
    pub curve_columns: Vec<CurveColumnMetadata>,
    pub raw_sections: RawMetadataSections,
    pub issues: Vec<IngestIssue>,
    pub index_unit: Option<String>,
}

impl From<LegacyPackageMetadata> for PackageMetadata {
    fn from(value: LegacyPackageMetadata) -> Self {
        Self {
            package: PackageIdentityMetadata {
                package_version: value.package_version,
                metadata_schema_version: value.metadata_schema_version,
            },
            document: PackageDocumentMetadata {
                summary: value.summary,
                provenance: value.provenance,
                encoding: value.encoding,
            },
            canonical: value.canonical,
            storage: PackageStorageMetadata {
                index: value.index,
                index_unit: value.index_unit,
                curve_columns: value.curve_columns,
            },
            raw: value.raw_sections,
            diagnostics: PackageDiagnosticsMetadata {
                issues: value.issues,
            },
        }
    }
}

impl PackageMetadata {
    pub fn package_version(&self) -> u32 {
        self.package.package_version
    }
}

impl LasFile {
    pub fn metadata(&self) -> CanonicalMetadata {
        CanonicalMetadata {
            version: self.version_info(),
            well: self.well_info(),
            index: self.index_info(),
            curves: self
                .curves
                .iter()
                .map(|curve| CurveInfo {
                    name: curve.mnemonic.clone(),
                    original_mnemonic: curve.original_mnemonic.clone(),
                    canonical_name: curve.mnemonic.clone(),
                    unit: non_empty_string(&curve.unit),
                    description: non_empty_string(&curve.description),
                    header_value: las_value_option(&curve.value),
                    nullable: curve
                        .data
                        .iter()
                        .any(|value| value.is_empty() || value.is_nan()),
                    storage_kind: detect_storage_kind(&curve.data),
                    row_count: curve.data.len(),
                    alias: derive_canonical_alias(&curve.original_mnemonic, &curve.unit),
                })
                .collect(),
            parameters: self.parameter_infos(),
            other: non_empty_string(&self.other),
            issue_count: self.issues.len(),
        }
    }

    pub fn version_info(&self) -> VersionInfo {
        VersionInfo {
            vers: item_display_string(self.version.get("VERS")),
            wrap: item_display_string(self.version.get("WRAP")),
            delimiter: non_empty_string(&self.summary.delimiter),
        }
    }

    pub fn well_info(&self) -> WellInfo {
        WellInfo {
            well: item_display_string(self.well.get("WELL")),
            company: item_display_string(self.well.get("COMP")),
            field: item_display_string(self.well.get("FLD")),
            location: item_display_string(self.well.get("LOC")),
            province: item_display_string(self.well.get("PROV")),
            service_company: item_display_string(self.well.get("SRVC")),
            date: item_display_string(self.well.get("DATE")),
            uwi: item_display_string(self.well.get("UWI")),
            api: item_display_string(self.well.get("API")),
            start: item_numeric_value(self.well.get("STRT")),
            stop: item_numeric_value(self.well.get("STOP")),
            step: item_numeric_value(self.well.get("STEP")),
            null_value: item_numeric_value(self.well.get("NULL")),
        }
    }

    pub fn index_info(&self) -> IndexInfo {
        let curve = self.curves.get(&self.index.curve_id);
        let row_count = curve.map(|value| value.data.len()).unwrap_or(0);
        let nullable = curve
            .map(|value| {
                value
                    .data
                    .iter()
                    .any(|item| item.is_empty() || item.is_nan())
            })
            .unwrap_or(false);
        let storage_kind = curve
            .map(|value| detect_storage_kind(&value.data))
            .unwrap_or(CurveStorageKind::Numeric);
        IndexInfo {
            name: self.index.curve_id.clone(),
            original_mnemonic: self.index.raw_mnemonic.clone(),
            canonical_name: String::from("index"),
            unit: non_empty_string(&self.index.unit),
            kind: self.index.kind.clone(),
            row_count,
            nullable,
            storage_kind,
            alias: derive_canonical_alias(&self.index.raw_mnemonic, &self.index.unit),
        }
    }

    pub fn curve_infos(&self) -> Vec<CurveInfo> {
        self.metadata().curves
    }

    pub fn parameter_infos(&self) -> Vec<ParameterInfo> {
        self.params
            .iter()
            .map(|param| ParameterInfo {
                name: param.mnemonic.clone(),
                original_mnemonic: param.original_mnemonic.clone(),
                unit: non_empty_string(&param.unit),
                value: las_value_option(&param.value),
                description: non_empty_string(&param.description),
            })
            .collect()
    }
}

pub fn package_metadata_for(file: &LasFile, package_version: u32) -> PackageMetadata {
    let mut summary = file.summary.clone();
    summary.row_count = file.row_count();
    summary.curve_count = file.curves.len();
    summary.issue_count = file.issues.len();

    PackageMetadata {
        package: PackageIdentityMetadata {
            package_version,
            metadata_schema_version: String::from(PACKAGE_METADATA_SCHEMA_VERSION),
        },
        document: PackageDocumentMetadata {
            summary,
            provenance: file.provenance.clone(),
            encoding: file.encoding.clone(),
        },
        canonical: file.metadata(),
        storage: PackageStorageMetadata {
            index: file.index.clone(),
            index_unit: file.index_unit.clone(),
            curve_columns: file
                .curves
                .iter()
                .map(|curve| CurveColumnMetadata {
                    name: curve.mnemonic.clone(),
                    canonical_name: if curve.mnemonic == file.index.curve_id {
                        String::from("index")
                    } else {
                        curve.mnemonic.clone()
                    },
                    original_mnemonic: curve.original_mnemonic.clone(),
                    unit: curve.unit.clone(),
                    header_value: curve.value.clone(),
                    description: curve.description.clone(),
                    storage_kind: detect_storage_kind(&curve.data),
                    row_count: curve.data.len(),
                    nullable: curve
                        .data
                        .iter()
                        .any(|value| value.is_empty() || value.is_nan()),
                    alias: derive_canonical_alias(&curve.original_mnemonic, &curve.unit),
                    is_index: curve.mnemonic == file.index.curve_id,
                })
                .collect(),
        },
        raw: RawMetadataSections {
            version: file.version.clone(),
            well: file.well.clone(),
            params: file.params.clone(),
            other: file.other.clone(),
            extra_sections: file.extra_sections.clone(),
            curve_mnemonic_case: file.curves.mnemonic_case,
        },
        diagnostics: PackageDiagnosticsMetadata {
            issues: file.issues.clone(),
        },
    }
}

pub fn parse_package_metadata(
    text: &str,
) -> std::result::Result<PackageMetadata, serde_json::Error> {
    serde_json::from_str(text)
        .or_else(|_| serde_json::from_str::<LegacyPackageMetadata>(text).map(Into::into))
}

pub fn validate_canonical_metadata(file: &LasFile) -> Result<()> {
    if file.index.curve_id.trim().is_empty() {
        return Err(LasError::Validation(String::from(
            "index descriptor must reference a curve id",
        )));
    }
    if file.index.raw_mnemonic.trim().is_empty() {
        return Err(LasError::Validation(String::from(
            "index descriptor must preserve the original mnemonic",
        )));
    }

    let index_curve = file.curves.get(&file.index.curve_id).ok_or_else(|| {
        LasError::Validation(format!(
            "index curve '{}' is missing from LAS curves",
            file.index.curve_id
        ))
    })?;

    if detect_storage_kind(&index_curve.data) != CurveStorageKind::Numeric {
        return Err(LasError::Validation(format!(
            "index curve '{}' must remain numeric",
            file.index.curve_id
        )));
    }

    for curve in file.curves.iter() {
        if curve.mnemonic.trim().is_empty() {
            return Err(LasError::Validation(String::from(
                "curve mnemonics must not be empty",
            )));
        }
    }

    Ok(())
}

pub fn validate_package_metadata(metadata: &PackageMetadata) -> Result<()> {
    if metadata.package.metadata_schema_version.trim().is_empty() {
        return Err(LasError::Validation(String::from(
            "package metadata schema version must not be empty",
        )));
    }

    let summary = &metadata.document.summary;
    let index = &metadata.storage.index;
    let canonical = &metadata.canonical;
    let curve_columns = &metadata.storage.curve_columns;

    if canonical.index.name != index.curve_id {
        return Err(LasError::Validation(format!(
            "canonical index '{}' does not match storage index '{}'",
            canonical.index.name, index.curve_id
        )));
    }

    if canonical.index.original_mnemonic != index.raw_mnemonic {
        return Err(LasError::Validation(format!(
            "canonical index mnemonic '{}' does not match storage index mnemonic '{}'",
            canonical.index.original_mnemonic, index.raw_mnemonic
        )));
    }

    if curve_columns.len() != summary.curve_count {
        return Err(LasError::Validation(format!(
            "package metadata declares {} curve columns but summary expects {}",
            curve_columns.len(),
            summary.curve_count
        )));
    }

    if canonical.curves.len() != summary.curve_count {
        return Err(LasError::Validation(format!(
            "canonical metadata declares {} curves but summary expects {}",
            canonical.curves.len(),
            summary.curve_count
        )));
    }

    if canonical.index.row_count != summary.row_count {
        return Err(LasError::Validation(format!(
            "canonical index row count {} does not match summary row count {}",
            canonical.index.row_count, summary.row_count
        )));
    }

    let index_column = curve_columns.iter().find(|column| column.is_index);
    let Some(index_column) = index_column else {
        return Err(LasError::Validation(String::from(
            "package storage metadata must mark exactly one index column",
        )));
    };

    if index_column.name != index.curve_id {
        return Err(LasError::Validation(format!(
            "index column '{}' does not match storage index '{}'",
            index_column.name, index.curve_id
        )));
    }

    if curve_columns
        .iter()
        .filter(|column| column.is_index)
        .count()
        != 1
    {
        return Err(LasError::Validation(String::from(
            "package storage metadata must contain exactly one index column",
        )));
    }

    for column in curve_columns {
        if column.row_count != summary.row_count {
            return Err(LasError::Validation(format!(
                "curve column '{}' has {} rows but summary expects {}",
                column.name, column.row_count, summary.row_count
            )));
        }
    }

    for curve in &canonical.curves {
        if curve.row_count != summary.row_count {
            return Err(LasError::Validation(format!(
                "canonical curve '{}' has {} rows but summary expects {}",
                curve.name, curve.row_count, summary.row_count
            )));
        }
        if !curve_columns.iter().any(|column| column.name == curve.name) {
            return Err(LasError::Validation(format!(
                "canonical curve '{}' is missing from storage columns",
                curve.name
            )));
        }
    }

    Ok(())
}

fn item_display_string(item: Option<&HeaderItem>) -> Option<String> {
    item.and_then(|item| las_value_option(&item.value))
}

fn item_numeric_value(item: Option<&HeaderItem>) -> Option<f64> {
    item.and_then(|item| item.value.as_f64())
}

fn las_value_option(value: &LasValue) -> Option<String> {
    match value {
        LasValue::Empty => None,
        _ => Some(value.display_string()),
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn detect_storage_kind(values: &[LasValue]) -> CurveStorageKind {
    let has_numbers = values
        .iter()
        .any(|value| matches!(value, LasValue::Number(_)));
    let has_text = values
        .iter()
        .any(|value| matches!(value, LasValue::Text(_)));
    match (has_numbers, has_text) {
        (true, true) => CurveStorageKind::Mixed,
        (true, false) => CurveStorageKind::Numeric,
        (false, true) => CurveStorageKind::Text,
        (false, false) => CurveStorageKind::Numeric,
    }
}
