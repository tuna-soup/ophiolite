mod backend;
mod backend_commands;
mod command_service;

pub use backend::{PackageBackend, dto_contract_version};
pub use backend_commands::PackageBackendState;
pub use command_service::PackageCommandService;
pub use ophiolite_compute::{
    AssetSemanticFamily, ComputeAvailability, ComputeBindingCandidate, ComputeCatalog,
    ComputeCatalogEntry, ComputeExecutionManifest, ComputeFunctionMetadata, ComputeInputBinding,
    ComputeInputSpec, ComputeParameterDefinition, ComputeParameterValue, ComputeRegistry,
    ComputedCurve, CurveBindingCandidate, CurveSemanticDescriptor, CurveSemanticSource,
    CurveSemanticType, DrillingObservationDataRow, LogCurveData, PressureObservationDataRow,
    TopDataRow, TrajectoryDataRow, classify_curve_semantic, default_curve_semantics,
};
pub use ophiolite_core::{
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
pub use ophiolite_ingest::{
    import_drilling_csv_asset, import_las_asset, import_pressure_csv_asset,
    import_seismic_trace_data_store_asset, import_tops_csv_asset, import_trajectory_csv_asset,
};
pub use ophiolite_package::{
    CurveValueDiffSummary, PackageBlobRef, PackageDiffSummary, PackageRevisionRecord,
    PackageSession, PackageSessionStore, StoredLasFile, list_package_revisions, open_package,
    open_package_metadata, open_package_summary, validate_package, write_bundle, write_package,
    write_package_overwrite,
};
pub use ophiolite_parser::examples;
pub use ophiolite_parser::{
    DType, DTypeSpec, DecodedText, NullPolicy, NullRule, ParsedHeaderLine, ReadOptions, ReadPolicy,
    decode_bytes, import_las_file, parse_header_line, read_path, read_reader, read_string,
};
pub use ophiolite_project::{
    AssetBindingInput, AssetColumnMetadata, AssetColumnType, AssetTableMetadata, DepthRangeQuery,
    DrillingObservationRow, PressureObservationRow, TopRow, TrajectoryRow,
};
pub use ophiolite_project::{
    AssetBlobRef, AssetCollectionId, AssetCollectionRecord, AssetCollectionSummary,
    AssetDiffSummary, AssetExtent, AssetId, AssetKind, AssetManifest, AssetRecord,
    AssetReferenceMetadata, AssetRevisionId, AssetRevisionRecord, AssetStatus, BulkDataDescriptor,
    CoordinateReference, CurveValueChangeSummary, DepthReference, DirectoryAssetDiffSummary,
    ImportResolution, LogAssetDiffSummary, LogAssetImportResult, OphioliteProject,
    OphioliteProjectManifest, ProjectAssetImportResult, ProjectAssetSummary,
    ProjectComputeRunRequest, ProjectComputeRunResult, ProjectSummary,
    ProjectSurveyAssetInventoryItem, ProjectWellOverlayInventory, ProjectWellboreInventoryItem,
    SeismicAssetImportResult, SeismicAssetMetadata, SourceArtifactRef, StructuredAssetDiffSummary,
    UnitSystem, VerticalDatum, WellId, WellIdentifierSet, WellRecord, WellSummary, WellboreId,
    WellboreRecord, WellboreSummary, resolve_dataset_summary_survey_map_source,
};
pub use ophiolite_project::{
    CoordinateReferenceBindingDto, CoordinateReferenceDto, CoordinateReferenceSourceDto,
    ProjectSurveyMapRequestDto, ProjectedPoint2Dto, ProjectedPolygon2Dto, ProjectedVector2Dto,
    ResolveSectionWellOverlaysResponse, ResolvedSectionWellOverlayDto, ResolvedSurveyMapHorizonDto,
    ResolvedSurveyMapSourceDto, ResolvedSurveyMapSurveyDto, ResolvedSurveyMapWellDto,
    ResolvedWellPanelSourceDto, ResolvedWellPanelWellDto, SECTION_WELL_OVERLAY_CONTRACT_VERSION,
    SURVEY_MAP_CONTRACT_VERSION, SectionWellOverlayDomainDto, SectionWellOverlayRequestDto,
    SectionWellOverlaySampleDto, SectionWellOverlaySegmentDto, SurveyIndexAxisDto,
    SurveyIndexGridDto, SurveyMapGridTransformDto, SurveyMapRequestDto, SurveyMapScalarFieldDto,
    SurveyMapSpatialAvailabilityDto, SurveyMapSpatialDescriptorDto, SurveyMapTrajectoryDto,
    SurveyMapTrajectoryStationDto, SurveyMapTransformDiagnosticsDto, SurveyMapTransformPolicyDto,
    SurveyMapTransformStatusDto, WELL_PANEL_CONTRACT_VERSION, WellPanelDepthSampleDto,
    WellPanelDrillingObservationDto, WellPanelDrillingSetDto, WellPanelLogCurveDto,
    WellPanelPressureObservationDto, WellPanelPressureSetDto, WellPanelRequestDto,
    WellPanelTopRowDto, WellPanelTopSetDto, WellPanelTrajectoryDto, WellPanelTrajectoryRowDto,
};
pub use ophiolite_project::{
    DrillingObservationEditRequest, DrillingObservationRowPatch,
    OpenStructuredAssetEditSessionRequest, OptionalFieldPatch, PressureObservationEditRequest,
    PressureObservationRowPatch, StructuredAssetEditSessionId, StructuredAssetEditSessionStore,
    StructuredAssetEditSessionSummary, StructuredAssetSaveResult, StructuredAssetSessionRequest,
    TopRowPatch, TopSetEditRequest, TrajectoryEditRequest, TrajectoryRowPatch,
};
pub use ophiolite_project::{
    SyntheticProjectAssetIds, SyntheticProjectFixture, SyntheticProjectSourcePaths,
    generate_synthetic_project_fixture,
};
pub use ophiolite_seismic::{
    BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest,
    CheckshotVspObservationSet1D, CompiledWellTimeDepthLineage, CoordinateReferenceBinding,
    CoordinateReferenceDescriptor, CoordinateReferenceSource, DatasetId, DatasetSummary,
    DepthReferenceKind, GatherAxisKind, GatherInteractionChanged, GatherPreviewView, GatherProbe,
    GatherProbeChanged, GatherProcessingOperation, GatherProcessingPipeline, GatherRequest,
    GatherSampleDomain, GatherSelector, GatherView, GatherViewport, GatherViewportChanged,
    IPC_SCHEMA_VERSION, ImportDatasetRequest, ImportDatasetResponse,
    ImportPrestackOffsetDatasetRequest, ImportPrestackOffsetDatasetResponse,
    ImportedHorizonDescriptor, InterpretationPoint, LateralInterpolationMethod,
    LayeredVelocityInterval, LayeredVelocityModel, ManualTimeDepthPickSet1D, OpenDatasetRequest,
    OpenDatasetResponse, PrestackThirdAxisField, PreviewCommand, PreviewGatherProcessingRequest,
    PreviewGatherProcessingResponse, PreviewResponse, PreviewTraceLocalProcessingRequest,
    PreviewTraceLocalProcessingResponse, PreviewView, ProcessingJobProgress, ProcessingJobState,
    ProcessingJobStatus, ProcessingLayoutCompatibility, ProcessingOperation,
    ProcessingOperatorScope, ProcessingPipeline, ProcessingPipelineFamily, ProcessingPipelineSpec,
    ProcessingPreset, ProjectedPoint2, ProjectedPolygon2, ProjectedVector2,
    ResolvedSectionDisplayView, ResolvedTrajectoryGeometry, ResolvedTrajectoryStation,
    RunGatherProcessingRequest, RunGatherProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SectionAxis, SectionColorMap, SectionCoordinate,
    SectionDisplayDefaults, SectionHorizonLineStyle, SectionHorizonOverlayView,
    SectionHorizonSample, SectionHorizonStyle, SectionInteractionChanged, SectionMetadata,
    SectionPolarity, SectionPrimaryMode, SectionProbe, SectionProbeChanged, SectionRenderMode,
    SectionRequest, SectionScalarOverlayColorMap, SectionScalarOverlayValueRange,
    SectionScalarOverlayView, SectionTileRequest, SectionTimeDepthDiagnostics,
    SectionTimeDepthTransformMode, SectionUnits, SectionView, SectionViewport,
    SectionViewportChanged, SegyGeometryCandidate, SegyGeometryOverride, SegyHeaderField,
    SegyHeaderValueType, SemblancePanel, SpatialCoverageRelationship, SpatialCoverageSummary,
    StratigraphicBoundaryReference, SuggestedImportAction, SurveyGridTransform,
    SurveyPreflightRequest, SurveyPreflightResponse, SurveyPropertyField3D,
    SurveySpatialAvailability, SurveySpatialDescriptor, SurveyTimeDepthTransform3D,
    TimeDepthDomain, TimeDepthSample1D, TimeDepthTransformSourceKind,
    TraceLocalProcessingOperation, TraceLocalProcessingPipeline, TraceLocalProcessingPreset,
    TraceLocalProcessingStep, TrajectoryInputSchemaKind, TrajectoryValueOrigin,
    TravelTimeReference, VelocityAutopickParameters, VelocityControlProfile,
    VelocityControlProfileSample, VelocityControlProfileSet, VelocityFunctionEstimate,
    VelocityFunctionSource, VelocityIntervalTrend, VelocityPickStrategy, VelocityQuantityKind,
    VelocityScanRequest, VelocityScanResponse, VelocitySource3D, VerticalAxisDescriptor,
    VerticalInterpolationMethod, VolumeDescriptor, WellAzimuthReferenceKind, WellTieAnalysis1D,
    WellTieCurve1D, WellTieLogCurveSource, WellTieLogSelection1D, WellTieObservationSet1D,
    WellTieSectionWindow, WellTieTrace1D, WellTieVelocitySourceKind, WellTieWavelet,
    WellTimeDepthAssumptionInterval, WellTimeDepthAssumptionKind, WellTimeDepthAuthoredModel1D,
    WellTimeDepthModel1D, WellTimeDepthObservationSample, WellTimeDepthSourceBinding,
    WellboreAnchorKind, WellboreAnchorReference, WellboreGeometry,
};
pub use ophiolite_seismic::{
    SeismicAssetFamily, SeismicAssetId, SeismicAxisRole, SeismicBinGridDescriptor, SeismicColorMap,
    SeismicDescriptorConversionError, SeismicDimensionDescriptor, SeismicDisplayDefaults,
    SeismicGatherAxisKind, SeismicIndexAxis, SeismicInterpretationPoint, SeismicLayout,
    SeismicOrganization, SeismicPolarity, SeismicProbe, SeismicProcessingParameters,
    SeismicRenderMode, SeismicSampleAxis, SeismicSampleDomain, SeismicSectionAxis,
    SeismicSectionCoordinate, SeismicSectionRequest, SeismicSectionTileRequest, SeismicSectionView,
    SeismicStackingState, SeismicTrace, SeismicTraceDataDescriptor, SeismicTraceDescriptor,
    SeismicTraceSetDescriptor, SeismicTraceSetView, SeismicUnits, SeismicVolumeDescriptor,
    SeismicVolumeGeometry,
};
pub use ophiolite_seismic_io::{
    ChunkProcessingError, ChunkReadConfig, Cube, Endianness, FileSummary, FixtureCase,
    GeometryClassification, GeometryCoordinate, GeometryOptions, GeometryReport, Hdf5CubeLayout,
    Hdf5CubeWriteError, Hdf5CubeWriter, HeaderColumn, HeaderField, HeaderLoadConfig, HeaderMapping,
    HeaderTable, HeaderValueType, InspectError, InspectOptions, IntervalOptions, IoStrategy,
    PrimaryTraceHeader, ReadError, ReaderOptions, SampleFormat, SampleIntervalSource,
    SampleIntervalUnit, SegyReader, SegyRevision, SegyWarning, TextualHeader,
    TextualHeaderEncoding, TraceBlock, TraceBlockInfo, TraceChunk, TraceChunkIter, TraceChunkRef,
    TraceSelection, ValidationMode, curated_fixtures, inspect_file, inspect_file_with_options,
    load_trace_headers, load_trace_headers_with_config, open,
};
pub use ophiolite_seismic_runtime::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, InterpMethod, PreflightAction,
    PreflightGeometry, PrestackStoreHandle, ProcessingLineage, RegularizationProvenance,
    SegyInspection, SeisGeometryOptions, SeismicStoreError, SourceIdentity, SourceVolume,
    SparseSurveyPolicy, StoreHandle, SurveyPreflight, TbgathManifest, TbgathReader, TbgathWriter,
    TbvolManifest, VolumeAxes, VolumeMetadata, build_survey_property_field,
    build_survey_time_depth_transform, create_tbgath_store, create_tbvol_store,
    describe_prestack_store, describe_store, ingest_prestack_offset_segy, ingest_segy,
    inspect_segy, load_array, load_occupancy, load_source_volume, load_source_volume_with_options,
    materialize_gather_processing_store, materialize_gather_processing_store_with_progress,
    open_prestack_store, open_store, preflight_segy, prestack_gather_view,
    preview_gather_processing_view, read_prestack_gather_plane, read_section_plane,
    recommended_chunk_shape, render_section_csv, render_section_csv_for_request, section_view,
    velocity_scan,
};
