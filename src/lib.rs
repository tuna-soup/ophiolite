mod backend;
mod backend_commands;
mod command_service;

pub use backend::{PackageBackend, dto_contract_version};
pub use backend_commands::PackageBackendState;
pub use command_service::PackageCommandService;
pub use lithos_core::{
    AssetSummaryDto, CanonicalAlias, CanonicalMetadata, CloseSessionResultDto, CommandErrorDto,
    CommandErrorKind, CommandGroup, CommandResponse, CurveCatalogDto, CurveCatalogEntryDto,
    CurveColumn, CurveColumnDescriptor, CurveColumnMetadata, CurveEditRequest, CurveInfo,
    CurveItem, CurveSelector, CurveStorageKind, CurveTable, CurveUpdateRequest,
    CurveWindowColumnDto, CurveWindowDto, CurveWindowRequest, DTO_CONTRACT_VERSION,
    DepthWindowRequest, DiagnosticIssueDto, DiagnosticTargetDto, DiagnosticTargetKind,
    DirtyStateDto, HeaderItem, HeaderItemUpdate, IndexDescriptor, IndexInfo, IndexKind,
    IngestIssue, IssueSeverity, LasError, LasFile, LasFileSummary, LasValue, MetadataDto,
    MetadataSectionDto, MetadataUpdateRequest, MnemonicCase, PACKAGE_METADATA_SCHEMA_VERSION,
    PackageDiagnosticsMetadata, PackageDocumentMetadata, PackageId, PackageIdentityMetadata,
    PackageMetadata, PackagePathRequest, PackageStorageMetadata, ParameterInfo, Provenance,
    RawLasWindowRequest, RawMetadataSections, Result, RevisionToken, SavePackageResultDto,
    SaveSessionResponseDto, SectionItems, SessionContextDto, SessionCurveEditRequest,
    SessionDepthWindowRequest, SessionId, SessionMetadataDto, SessionMetadataEditRequest,
    SessionRequest, SessionSaveAsRequest, SessionSummaryDto, SessionWindowDto,
    SessionWindowRequest, ValidationKind, ValidationReportDto, VersionInfo, WellInfo,
    apply_curve_edit, apply_metadata_update, asset_summary_dto, close_session_result_dto,
    command_error_dto, curve_catalog_dto, curve_catalog_result_dto, curve_depth_window_dto,
    curve_window_dto, depth_window_request_for_values, diagnostic_issue_dto, diagnostic_target_dto,
    dirty_state_dto, empty_validation_report, metadata_dto, package_id_for_path,
    package_metadata_for, package_validation_report, parse_package_metadata,
    revision_token_for_bytes, save_validation_report, session_context_dto, session_id_for_path,
    session_metadata_dto, session_summary_dto, session_window_dto, validate_canonical_metadata,
    validate_edit_state, validate_package_metadata, validation_issue_for_message,
    validation_report_dto, validation_report_from_issues, validation_report_from_messages,
};
pub use lithos_ingest::{
    import_drilling_csv_asset, import_las_asset, import_pressure_csv_asset, import_tops_csv_asset,
    import_trajectory_csv_asset,
};
pub use lithos_package::{
    PackageSession, PackageSessionStore, StoredLasFile, open_package, open_package_metadata,
    open_package_summary, validate_package, write_bundle, write_package, write_package_overwrite,
};
pub use lithos_parser::examples;
pub use lithos_parser::{
    DType, DTypeSpec, DecodedText, NullPolicy, NullRule, ParsedHeaderLine, ReadOptions, ReadPolicy,
    decode_bytes, import_las_file, parse_header_line, read_path, read_reader, read_string,
};
pub use lithos_project::{
    AssetBindingInput, AssetColumnMetadata, AssetColumnType, AssetTableMetadata, DepthRangeQuery,
    DrillingObservationRow, PressureObservationRow, TopRow, TrajectoryRow,
};
pub use lithos_project::{
    AssetCollectionId, AssetCollectionRecord, AssetCollectionSummary, AssetExtent, AssetId,
    AssetKind, AssetManifest, AssetRecord, AssetReferenceMetadata, AssetStatus, BulkDataDescriptor,
    CoordinateReference, DepthReference, ImportResolution, LithosProject, LithosProjectManifest,
    LogAssetImportResult, ProjectAssetImportResult, ProjectAssetSummary, ProjectSummary,
    SourceArtifactRef, UnitSystem, VerticalDatum, WellId, WellIdentifierSet, WellRecord,
    WellSummary, WellboreId, WellboreRecord, WellboreSummary,
};
pub use lithos_project::{
    SyntheticProjectAssetIds, SyntheticProjectFixture, SyntheticProjectSourcePaths,
    generate_synthetic_project_fixture,
};
