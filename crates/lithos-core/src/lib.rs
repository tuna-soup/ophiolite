use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurveStorageKind {
    Numeric,
    Text,
    Mixed,
}

pub type Result<T> = std::result::Result<T, LasError>;

#[derive(Debug, Error)]
pub enum LasError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unsupported LAS input: {0}")]
    Unsupported(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[path = "../../../src/asset.rs"]
mod asset;
#[path = "../../../src/dto.rs"]
mod dto;
#[path = "../../../src/metadata.rs"]
mod metadata;
#[path = "../../../src/table.rs"]
mod table;

pub use asset::{
    CanonicalAlias, CurveItem, CurveSelector, HeaderItem, IndexDescriptor, IndexKind, IngestIssue,
    IssueSeverity, LasFile, LasFileSummary, LasValue, MnemonicCase, Provenance, SectionItem,
    SectionItems, bundle_manifest_path, derive_canonical_alias, derive_index_kind, mnemonic_match,
    natural_sort_key, normalized_depth_unit, useful_mnemonic,
};
pub use dto::{
    AssetSummaryDto, CloseSessionResultDto, CurveCatalogEntryDto, CurveEditRequest,
    CurveUpdateRequest, CurveWindowColumnDto, CurveWindowDto, CurveWindowRequest,
    DTO_CONTRACT_VERSION, DirtyStateDto, HeaderItemUpdate, MetadataDto, MetadataSectionDto,
    MetadataUpdateRequest, PackageId, PackagePathRequest, RevisionToken, SaveConflictDto,
    SavePackageResultDto, SaveSessionResponseDto, SessionCurveEditRequest, SessionId,
    SessionMetadataEditRequest, SessionRequest, SessionSaveAsRequest, SessionSummaryDto,
    SessionWindowRequest, ValidationKind, ValidationReportDto, apply_curve_edit,
    apply_metadata_update, asset_summary_dto, close_session_result_dto, curve_catalog_dto,
    curve_window_dto, dirty_state_dto, metadata_dto, package_id_for_path,
    package_validation_report, revision_token_for_bytes, save_conflict_dto, save_validation_report,
    session_id_for_path, session_summary_dto, validate_edit_state, validation_report_dto,
};
pub use metadata::{
    CanonicalMetadata, CurveColumnMetadata, CurveInfo, IndexInfo, PACKAGE_METADATA_SCHEMA_VERSION,
    PackageMetadata, ParameterInfo, RawMetadataSections, VersionInfo, WellInfo,
    package_metadata_for,
};
pub use table::{CurveColumn, CurveColumnDescriptor, CurveTable, detect_storage_kind};
