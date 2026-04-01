pub use ophiolite_compute::{
    AssetSemanticFamily, ComputeAvailability, ComputeBindingCandidate, ComputeCatalog,
    ComputeCatalogEntry, ComputeExecutionManifest, ComputeFunctionMetadata, ComputeInputBinding,
    ComputeInputSpec, ComputeParameterDefinition, ComputeParameterValue, ComputeRegistry,
    ComputedCurve, CurveBindingCandidate, CurveSemanticDescriptor, CurveSemanticSource,
    CurveSemanticType, DrillingObservationDataRow, LogCurveData, PressureObservationDataRow,
    TopDataRow, TrajectoryDataRow, classify_curve_semantic, default_curve_semantics,
};
pub use ophiolite_core::{
    IndexKind, IngestIssue, LasError, LasFile, Provenance, Result, WellInfo, package_metadata_for,
    revision_token_for_bytes,
};
pub use ophiolite_package::write_package_overwrite;
pub use ophiolite_parser::read_path;

#[path = "../../../src/project.rs"]
mod project;
#[path = "../../../src/project_contracts.rs"]
mod project_contracts;
#[path = "../../../src/project_assets.rs"]
mod project_assets;
#[path = "../../../src/project_edit.rs"]
mod project_edit;
#[path = "../../../src/synthetic_fixtures.rs"]
mod synthetic_fixtures;

pub use project::{
    AssetBlobRef, AssetCollectionId, AssetCollectionRecord, AssetCollectionSummary,
    AssetDiffSummary, AssetExtent, AssetId, AssetKind, AssetManifest, AssetRecord,
    AssetReferenceMetadata, AssetRevisionId, AssetRevisionRecord, AssetStatus, BulkDataDescriptor,
    CoordinateReference, CurveValueChangeSummary, DepthReference, DirectoryAssetDiffSummary,
    ImportResolution, LogAssetDiffSummary, LogAssetImportResult, OphioliteProject,
    OphioliteProjectManifest, ProjectAssetImportResult, ProjectAssetSummary,
    ProjectComputeRunRequest, ProjectComputeRunResult, ProjectSummary, SeismicAssetImportResult,
    SeismicAssetMetadata, SourceArtifactRef, StructuredAssetDiffSummary, UnitSystem,
    VerticalDatum, WellId, WellIdentifierSet, WellRecord, WellSummary, WellboreId, WellboreRecord,
    WellboreSummary,
};
pub use project_assets::{
    AssetBindingInput, AssetColumnMetadata, AssetColumnType, AssetTableMetadata, DepthRangeQuery,
    DrillingObservationRow, PressureObservationRow, TopRow, TrajectoryRow,
};
pub use project_contracts::{
    ResolvedWellPanelSourceDto, ResolvedWellPanelWellDto, WELL_PANEL_CONTRACT_VERSION,
    WellPanelDepthSampleDto, WellPanelDrillingObservationDto, WellPanelDrillingSetDto,
    WellPanelLogCurveDto, WellPanelPressureObservationDto, WellPanelPressureSetDto,
    WellPanelRequestDto, WellPanelTopRowDto, WellPanelTopSetDto, WellPanelTrajectoryDto,
    WellPanelTrajectoryRowDto,
};
pub use project_edit::{
    DrillingObservationEditRequest, DrillingObservationRowPatch,
    OpenStructuredAssetEditSessionRequest, OptionalFieldPatch, PressureObservationEditRequest,
    PressureObservationRowPatch, StructuredAssetEditSessionId, StructuredAssetEditSessionStore,
    StructuredAssetEditSessionSummary, StructuredAssetSaveResult, StructuredAssetSessionRequest,
    TopRowPatch, TopSetEditRequest, TrajectoryEditRequest, TrajectoryRowPatch,
};
pub use synthetic_fixtures::{
    SyntheticProjectAssetIds, SyntheticProjectFixture, SyntheticProjectSourcePaths,
    generate_synthetic_project_fixture,
};
