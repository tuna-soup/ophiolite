mod error;
mod ingest;
mod metadata;
mod preflight;
mod render;
mod store;
mod upscale;
mod validation;

pub use error::SeisRefineError;
pub use ingest::{
    IngestOptions, SeisGeometryOptions, SourceVolume, SparseSurveyPolicy, VolumeImportFormat,
    detect_volume_import_format, estimate_mdio_tbvol_storage, ingest_mdio_store, ingest_segy,
    ingest_volume, ingest_zarr_store, load_source_volume, load_source_volume_with_options,
    looks_like_mdio_path, normalize_volume_import_path, recommended_chunk_shape,
};
pub use metadata::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, InterpMethod, ProcessingLineage,
    RegularizationProvenance, SourceIdentity, TbvolManifest, VolumeAxes, VolumeMetadata,
};
pub use ophiolite_seismic::contracts::{
    InspectableProcessingPlan, ReuseArtifactKind, ReuseBoundaryKind, ReuseMissReason,
    ReuseRequirement, ReuseResolution,
};
pub use ophiolite_seismic::{
    AmplitudeSpectrumCurve, AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, AxisSummaryF32,
    AxisSummaryI32, BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest,
    CancelProcessingBatchRequest, CancelProcessingBatchResponse, CancelProcessingJobRequest,
    CancelProcessingJobResponse, CoordinateReferenceBinding, CoordinateReferenceDescriptor,
    CoordinateReferenceSource, DatasetId, DeletePipelinePresetRequest,
    DeletePipelinePresetResponse, DepthReferenceKind, FrequencyPhaseMode, FrequencyWindowShape,
    GatherInterpolationMode, GatherPreviewView, GatherProcessingOperation,
    GatherProcessingPipeline, GatherRequest, GatherSelector, GeometryDescriptor,
    GeometryProvenanceSummary, GeometrySummary, GetProcessingBatchRequest,
    GetProcessingBatchResponse, GetProcessingDebugPlanRequest, GetProcessingDebugPlanResponse,
    GetProcessingJobRequest, GetProcessingJobResponse, GetProcessingRuntimeStateRequest,
    GetProcessingRuntimeStateResponse, ImportHorizonXyzRequest, ImportHorizonXyzResponse,
    ImportPrestackOffsetDatasetRequest, ImportPrestackOffsetDatasetResponse,
    ImportedHorizonDescriptor, InterpretationPoint, LateralInterpolationMethod,
    LayeredVelocityInterval, LayeredVelocityModel, ListPipelinePresetsResponse,
    ListProcessingRuntimeEventsRequest, ListProcessingRuntimeEventsResponse,
    LoadSectionHorizonsRequest, LoadSectionHorizonsResponse, LocalVolumeStatistic,
    NeighborhoodDipOutput, OperatorSetIdentity, PipelineArtifactIdentity, PipelineSemanticIdentity,
    PlannerProfileIdentity, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, PostStackNeighborhoodWindow, PrestackThirdAxisField,
    PreviewGatherProcessingRequest, PreviewGatherProcessingResponse,
    PreviewPostStackNeighborhoodProcessingRequest, PreviewPostStackNeighborhoodProcessingResponse,
    PreviewProcessingRequest, PreviewProcessingResponse, PreviewTraceLocalProcessingRequest,
    PreviewTraceLocalProcessingResponse, ProcessingArtifactRole, ProcessingBatchItemRequest,
    ProcessingBatchItemStatus, ProcessingBatchProgress, ProcessingBatchState,
    ProcessingBatchStatus, ProcessingExecutionMode, ProcessingJobArtifact,
    ProcessingJobArtifactKind, ProcessingJobChunkPlanSummary, ProcessingJobExecutionSummary,
    ProcessingJobProgress, ProcessingJobRuntimeState, ProcessingJobStageExecutionSummary,
    ProcessingJobState, ProcessingJobStatus, ProcessingOperation, ProcessingPipeline,
    ProcessingPipelineFamily, ProcessingPipelineSpec, ProcessingPreset, ProcessingRuntimeEvent,
    ProcessingSchedulerReason, ProjectedPoint2, ProjectedVector2, ResolvedSectionDisplayView,
    RunGatherProcessingRequest, RunGatherProcessingResponse,
    RunPostStackNeighborhoodProcessingRequest, RunPostStackNeighborhoodProcessingResponse,
    RunProcessingRequest, RunProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SampleDataConversionKind, SampleDataFidelity,
    SampleValuePreservation, SavePipelinePresetRequest, SavePipelinePresetResponse, SectionAxis,
    SectionHorizonLineStyle, SectionHorizonOverlayView, SectionHorizonSample, SectionHorizonStyle,
    SectionRequest, SectionSpectrumSelection, SectionTileRequest, SemblancePanel,
    SourceArtifactIdentity, SourceSemanticIdentity, SpatialCoverageRelationship,
    SpatialCoverageSummary, StoreFormatIdentity, StratigraphicBoundaryReference,
    SubmitProcessingBatchRequest, SubmitProcessingBatchResponse,
    SubmitTraceLocalProcessingBatchRequest, SubmitTraceLocalProcessingBatchResponse,
    SubvolumeProcessingPipeline, SurveyGridTransform, SurveyPropertyField3D,
    SurveySpatialAvailability, SurveySpatialDescriptor, SurveyTimeDepthTransform3D,
    TimeDepthDomain, TimeDepthTransformSourceKind, TraceLocalProcessingOperation,
    TraceLocalProcessingPipeline, TraceLocalProcessingStep, TraceLocalVolumeArithmeticOperator,
    TravelTimeReference, VelocityAutopickParameters, VelocityControlProfile,
    VelocityControlProfileSample, VelocityControlProfileSet, VelocityFunctionEstimate,
    VelocityFunctionSource, VelocityIntervalTrend, VelocityPickStrategy, VelocityQuantityKind,
    VelocityScanRequest, VelocityScanResponse, VelocitySource3D, VerticalAxisDescriptor,
    VerticalInterpolationMethod, VolumeDescriptor,
};
pub use ophiolite_seismic::{PreviewView, SectionView};
pub use ophiolite_seismic_runtime::{
    ArtifactBoundaryReason, ArtifactDescriptor, ArtifactKey, ArtifactLifetimeClass, CacheMode,
    ChunkGridSpec, ChunkShapePolicy, CostEstimate, CpuCostClass, ExecutionArtifactRole,
    ExecutionOperatorScope, ExecutionPipelineSegment, ExecutionPlan, ExecutionPlanSummary,
    ExecutionPriorityClass, ExecutionSourceDescriptor, ExecutionSpatialDependency, ExecutionStage,
    ExecutionStageKind, GeometryFingerprints, HaloSpec, IoCostClass, LogicalDomain,
    MaterializationClass, MemoryCostClass, OperatorExecutionTraits, ParallelEfficiencyClass,
    PartitionFamily, PartitionOrdering, PartitionSpec, PipelineDescriptor, PlanProcessingRequest,
    PlanningMode, PreferredPartitioning, ProgressUnits, RetryPolicy,
    ReuseDecisionEvidence as RuntimeReuseDecisionEvidence,
    ReuseDecisionOutcome as RuntimeReuseDecisionOutcome, SampleHaloRequirement, SchedulerHints,
    StageExecutionClassification, TraceLocalChunkPlanRecommendation, TraceLocalChunkPlanResolution,
    TraceLocalMaterializeOptionsResolution, ValidationReport, VolumeDomain,
    operator_execution_traits_for_pipeline_spec, recommend_adaptive_partition_target,
    recommend_adaptive_partition_target_for_job_concurrency,
    recommend_trace_local_chunk_plan_for_execution,
};
pub use ophiolite_seismic_runtime::{
    CURRENT_RUNTIME_SEMANTICS_VERSION, CURRENT_STORE_WRITER_SEMANTICS_VERSION,
    CanonicalIdentityStatus, canonical_artifact_identity, canonical_processing_lineage_validation,
    operator_set_identity_for_pipeline, pipeline_semantic_identity,
    planner_profile_identity_for_pipeline, source_identity_digest,
};
pub use ophiolite_seismic_runtime::{
    GatherExecutionObserver, GatherJobStartedEvent, PostStackNeighborhoodExecutionObserver,
    PostStackNeighborhoodJobStartedEvent, ProcessingCacheFingerprint,
    ProcessingExecutionSummaryState, ReusedTraceLocalCheckpoint,
    SubvolumeCheckpointStageCompletedEvent, SubvolumeCheckpointStageStartedEvent,
    SubvolumeExecutionObserver, SubvolumeFinalStageStartedEvent, SubvolumeJobStartedEvent,
    TraceLocalCheckpointLookupKey, TraceLocalExecutionObserver, TraceLocalJobStartedEvent,
    TraceLocalProcessingStagePlan, TraceLocalStageCompletedEvent, TraceLocalStageStartedEvent,
    build_trace_local_checkpoint_stages_from_pipeline,
    build_trace_local_processing_stages_from_plan, checkpoint_output_store_path,
    execute_gather_processing_job, execute_post_stack_neighborhood_processing_job,
    execute_subvolume_processing_job, execute_trace_local_processing_job,
    rewrite_tbgath_processing_lineage, rewrite_tbvol_processing_lineage,
};
pub use ophiolite_seismic_runtime::{
    HorizonImportPreview, HorizonImportPreviewFile, HorizonSourceImportCanonicalDraft,
    HorizonSourceImportPreview,
};
pub use ophiolite_seismic_runtime::{
    MaterializeOptions, PartitionExecutionProgress, PreviewSectionPrefixCache,
    PreviewSectionPrefixReuse, PreviewSectionSession, SeismicStoreError,
    amplitude_spectrum_from_plane, amplitude_spectrum_from_reader, amplitude_spectrum_from_store,
    apply_pipeline_to_plane, apply_pipeline_to_traces, build_execution_plan,
    build_survey_property_field, build_survey_time_depth_transform,
    build_survey_time_depth_transform_from_horizon_pairs, convert_section_view_to_depth,
    default_zarr_storage_layout, depth_converted_section_view, export_store_to_segy,
    export_store_to_zarr, export_store_to_zarr_with_layout, load_survey_property_fields,
    load_survey_time_depth_transforms, materialize_from_reader_writer,
    materialize_from_reader_writer_with_progress,
    materialize_post_stack_neighborhood_processing_volume,
    materialize_post_stack_neighborhood_processing_volume_with_progress,
    materialize_processing_volume, materialize_processing_volume_with_partition_progress,
    materialize_processing_volume_with_progress, materialize_subvolume_processing_volume,
    materialize_subvolume_processing_volume_with_progress, materialize_volume,
    preview_post_stack_neighborhood_processing_section_view,
    preview_post_stack_neighborhood_processing_section_view_with_prefix_cache,
    preview_processing_section_plane, preview_processing_section_view,
    preview_processing_section_view_with_prefix_cache, preview_section_from_reader,
    preview_section_plane, preview_section_view, preview_section_view_with_prefix_cache,
    preview_subvolume_processing_section_view, resolve_reused_trace_local_checkpoint,
    resolve_trace_local_checkpoint_indexes, resolve_trace_local_materialize_options,
    resolved_section_display_view, store_survey_property_field, store_survey_time_depth_transform,
    trace_local_pipeline_hash, trace_local_pipeline_prefix, trace_local_pipeline_segment,
    trace_local_source_fingerprint,
};
pub use ophiolite_seismic_runtime::{MdioTbvolStorageEstimate, VolumeSubset};
pub use ophiolite_seismic_runtime::{
    OccupancyTile, PrestackStoreHandle, TbgathManifest, TbgathReader, TbgathWriter, TbvolReader,
    TbvolWriter, TileBuffer, TileCoord, TileGeometry, VolumeStoreReader, VolumeStoreWriter,
    assemble_section_plane, build_suggested_horizon_source_import_draft,
    convert_horizon_vertical_domain_with_transform, create_tbgath_store, describe_prestack_store,
    describe_tbvol_archive_sibling, import_horizon_xyzs_from_draft,
    import_horizon_xyzs_with_vertical_domain, ingest_openvds_store, ingest_prestack_offset_segy,
    load_horizon_grids, looks_like_openvds_path, open_prestack_store, prestack_gather_view,
    preview_gather_processing_view, preview_horizon_source_import,
    preview_horizon_xyzs_with_vertical_domain, read_prestack_gather_plane,
    recommended_default_tbvol_tile_target_mib, recommended_tbvol_tile_shape,
    set_any_store_native_coordinate_reference, set_store_vertical_axis,
    suggested_tbvol_restore_path, suggested_tbvolc_archive_path, transcode_tbvol_to_tbvolc,
    transcode_tbvolc_to_tbvol,
};
pub use ophiolite_seismic_runtime::{
    PROCESSING_OUTPUT_PACKAGE_CONFIG_SCHEMA_VERSION, PROCESSING_OUTPUT_PACKAGE_SCHEMA_VERSION,
    ProcessingOutputPackage, ProcessingOutputPackageBlobRef, ProcessingOutputPackageConfig,
    ProcessingOutputPackageManifest, open_processing_output_package, package_processing_output,
};
pub use ophiolite_seismic_runtime::{ReuseDecisionEvidence, ReuseDecisionOutcome};
pub use ophiolite_seismic_runtime::{
    TbvolArchiveSiblingStatus, TbvolcAmplitudeEncoding, TbvolcManifest,
};
pub use ophiolite_seismic_runtime::{
    materialize_gather_processing_store, materialize_gather_processing_store_with_progress,
};
pub use ophiolite_seismic_runtime::{
    validate_pipeline, validate_post_stack_neighborhood_processing_pipeline,
    validate_post_stack_neighborhood_processing_pipeline_for_layout, validate_processing_pipeline,
    velocity_scan,
};
pub use preflight::{PreflightAction, PreflightGeometry, SurveyPreflight, preflight_segy};
pub use render::{render_section_csv, render_section_csv_for_request};
pub use store::{
    SectionPlane, SectionTileView, StoreHandle, create_tbvol_store, describe_store,
    import_horizon_xyzs, load_array, load_occupancy, open_store, read_section_plane,
    section_horizon_overlays, section_tile_view, section_view,
};
pub use upscale::{UpscaleOptions, upscale_2x, upscale_cubic_2x, upscale_linear_2x, upscale_store};
pub use validation::{
    ValidationDatasetReport, ValidationMethodReport, ValidationMetrics, ValidationOptions,
    ValidationSummary, run_validation, validate_dataset,
};

pub use ophiolite_seismic_runtime::{SegyInspection, inspect_segy};
