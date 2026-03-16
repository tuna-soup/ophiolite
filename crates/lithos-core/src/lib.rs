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
    AssetSummaryDto, CloseSessionResultDto, CommandErrorDto, CommandErrorKind, CommandGroup,
    CommandResponse, CurveCatalogDto, CurveCatalogEntryDto, CurveEditRequest, CurveUpdateRequest,
    CurveWindowColumnDto, CurveWindowDto, CurveWindowRequest, DTO_CONTRACT_VERSION,
    DiagnosticIssueDto, DiagnosticTargetDto, DiagnosticTargetKind, DirtyStateDto, HeaderItemUpdate,
    MetadataDto, MetadataSectionDto, MetadataUpdateRequest, PackageId, PackagePathRequest,
    RevisionToken, SaveConflictDto, SavePackageResultDto, SaveSessionResponseDto,
    SessionContextDto, SessionCurveEditRequest, SessionId, SessionMetadataDto,
    SessionMetadataEditRequest, SessionRequest, SessionSaveAsRequest, SessionSummaryDto,
    SessionWindowDto, SessionWindowRequest, ValidationKind, ValidationReportDto, apply_curve_edit,
    apply_metadata_update, asset_summary_dto, close_session_result_dto, command_error_dto,
    curve_catalog_dto, curve_catalog_result_dto, curve_window_dto, diagnostic_issue_dto,
    diagnostic_target_dto, dirty_state_dto, empty_validation_report, metadata_dto,
    package_id_for_path, package_validation_report, revision_token_for_bytes, save_conflict_dto,
    save_validation_report, session_context_dto, session_id_for_path, session_metadata_dto,
    session_summary_dto, session_window_dto, validate_edit_state, validation_issue_for_message,
    validation_report_dto, validation_report_from_issues, validation_report_from_messages,
};
pub use metadata::{
    CanonicalMetadata, CurveColumnMetadata, CurveInfo, IndexInfo, PACKAGE_METADATA_SCHEMA_VERSION,
    PackageDiagnosticsMetadata, PackageDocumentMetadata, PackageIdentityMetadata, PackageMetadata,
    PackageStorageMetadata, ParameterInfo, RawMetadataSections, VersionInfo, WellInfo,
    package_metadata_for, parse_package_metadata, validate_canonical_metadata,
    validate_package_metadata,
};
pub use table::{CurveColumn, CurveColumnDescriptor, CurveTable, detect_storage_kind};
