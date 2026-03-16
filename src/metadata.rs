use crate::asset::{
    CanonicalAlias, HeaderItem, IngestIssue, LasFile, LasFileSummary, LasValue, Provenance,
    SectionItems, derive_canonical_alias,
};
use crate::table::CurveStorageKind;
use crate::{IndexDescriptor, IndexKind, MnemonicCase};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PACKAGE_METADATA_SCHEMA_VERSION: &str = "0.1.0";

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
    pub alias: CanonicalAlias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveColumnMetadata {
    pub name: String,
    pub original_mnemonic: String,
    pub unit: String,
    pub header_value: LasValue,
    pub description: String,
    pub storage_kind: CurveStorageKind,
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
pub struct PackageMetadata {
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

impl LasFile {
    pub fn metadata(&self) -> CanonicalMetadata {
        let curve_descriptors = self.data().descriptors();
        CanonicalMetadata {
            version: self.version_info(),
            well: self.well_info(),
            index: self.index_info(),
            curves: self
                .curves
                .iter()
                .zip(curve_descriptors)
                .map(|(curve, descriptor)| CurveInfo {
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
                    storage_kind: descriptor.storage_kind,
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
        IndexInfo {
            name: self.index.curve_id.clone(),
            original_mnemonic: self.index.raw_mnemonic.clone(),
            canonical_name: String::from("index"),
            unit: non_empty_string(&self.index.unit),
            kind: self.index.kind.clone(),
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
    PackageMetadata {
        package_version,
        metadata_schema_version: String::from(PACKAGE_METADATA_SCHEMA_VERSION),
        summary: file.summary.clone(),
        provenance: file.provenance.clone(),
        encoding: file.encoding.clone(),
        index: file.index.clone(),
        canonical: file.metadata(),
        curve_columns: file
            .curves
            .iter()
            .zip(file.data().descriptors())
            .map(|(curve, descriptor)| CurveColumnMetadata {
                name: curve.mnemonic.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: curve.unit.clone(),
                header_value: curve.value.clone(),
                description: curve.description.clone(),
                storage_kind: descriptor.storage_kind,
            })
            .collect(),
        raw_sections: RawMetadataSections {
            version: file.version.clone(),
            well: file.well.clone(),
            params: file.params.clone(),
            other: file.other.clone(),
            extra_sections: file.extra_sections.clone(),
            curve_mnemonic_case: file.curves.mnemonic_case,
        },
        issues: file.issues.clone(),
        index_unit: file.index_unit.clone(),
    }
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
