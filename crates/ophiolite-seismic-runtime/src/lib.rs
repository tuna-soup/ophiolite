mod avo_analysis;
mod compute;
mod error;
mod execution;
mod gather_processing;
mod horizons;
mod ingest;
mod mdio;
mod metadata;
mod openvds;
mod planner;
mod post_stack_neighborhood;
mod preflight;
mod prestack_analysis;
mod prestack_store;
mod render;
mod rock_physics;
mod segy_export;
mod storage;
mod store;
mod survey_time_depth;
mod time_depth;
mod trace_local_chunk_planning;
mod zarr_export;

pub use avo_analysis::avo_reflectivity;
pub use compute::{
    MaterializeOptions, PartitionExecutionProgress, PreviewSectionPrefixCache,
    PreviewSectionPrefixReuse, PreviewSectionSession, TraceLocalMaterializeOptionsResolution,
    amplitude_spectrum_from_plane, amplitude_spectrum_from_reader, amplitude_spectrum_from_store,
    apply_pipeline_to_plane, apply_pipeline_to_traces, materialize_from_reader_writer,
    materialize_from_reader_writer_with_progress, materialize_processing_volume,
    materialize_processing_volume_with_partition_progress,
    materialize_processing_volume_with_progress, materialize_subvolume_processing_volume,
    materialize_subvolume_processing_volume_with_progress, materialize_volume,
    preview_processing_section_plane, preview_processing_section_view,
    preview_processing_section_view_with_prefix_cache, preview_section_from_reader,
    preview_section_plane, preview_section_view, preview_section_view_with_prefix_cache,
    preview_subvolume_processing_section_view, resolve_trace_local_materialize_options,
    validate_pipeline, validate_pipeline_for_layout, validate_processing_pipeline,
    validate_processing_pipeline_for_layout, validate_subvolume_processing_pipeline,
    validate_subvolume_processing_pipeline_for_layout,
};
pub use error::SeismicStoreError;
pub use execution::{
    ArtifactDescriptor, CacheMode, ChunkPlanningMode, ChunkShapePolicy, Chunkability, CostEstimate,
    CpuCostClass, ExecutionArtifactRole, ExecutionMemoryBudget, ExecutionOperatorScope,
    ExecutionPipelineSegment, ExecutionPlan, ExecutionPlanSummary, ExecutionPriorityClass,
    ExecutionSourceDescriptor, ExecutionSpatialDependency, ExecutionStage, ExecutionStageKind,
    HaloSpec, IoCostClass, MemoryCostClass, OperatorExecutionTraits, ParallelEfficiencyClass,
    PartitionFamily, PartitionOrdering, PartitionSpec, PipelineDescriptor, PlanningMode,
    PreferredPartitioning, ProgressUnits, RetryPolicy, SampleHaloRequirement, SchedulerHints,
    StageExecutionClassification, StageMemoryProfile, TraceLocalChunkPlanRecommendation,
    ValidationReport, operator_execution_traits_for_pipeline_spec,
};
pub use gather_processing::{
    GatherPlane, apply_gather_processing_pipeline, apply_trace_local_pipeline_to_gather,
    validate_gather_processing_pipeline, validate_gather_processing_pipeline_for_layout,
};
pub use horizons::{
    HorizonImportPreview, HorizonImportPreviewFile, HorizonSourceImportCanonicalDraft,
    HorizonSourceImportPreview, HorizonXyzFilePreview, ImportedHorizonGrid,
    build_suggested_horizon_source_import_draft, convert_horizon_vertical_domain_with_transform,
    import_horizon_xyzs, import_horizon_xyzs_from_draft, import_horizon_xyzs_with_vertical_domain,
    inspect_horizon_xyz_files, load_horizon_grids, preview_horizon_source_import,
    preview_horizon_xyzs, preview_horizon_xyzs_with_vertical_domain, section_horizon_overlays,
};
pub use ingest::{
    IngestOptions, SeisGeometryOptions, SourceVolume, SparseSurveyPolicy, VolumeImportFormat,
    detect_volume_import_format, ingest_prestack_offset_segy, ingest_segy, ingest_volume,
    ingest_zarr_store, load_source_volume, load_source_volume_with_options,
    normalize_volume_import_path, recommended_chunk_shape,
};
pub use mdio::{
    MdioTbvolStorageEstimate, VolumeSubset, estimate_mdio_tbvol_storage, ingest_mdio_store,
    looks_like_mdio_path,
};
pub use metadata::{
    CompressionKind, DatasetKind, GeometryProvenance, HeaderFieldSpec, InterpMethod,
    ProcessingLineage, RegularizationProvenance, SegyExportDescriptor, SourceIdentity,
    StorageLayout, StoreManifest, VolumeAxes, VolumeMetadata, generate_store_id,
};
pub use openvds::{ingest_openvds_store, looks_like_openvds_path};
pub use ophiolite_seismic::{
    AmplitudeSpectrumCurve, AmplitudeSpectrumRequest, AmplitudeSpectrumResponse,
    AvoInterceptGradientAttributeMethod, AvoInterceptGradientAttributeRequest,
    AvoInterceptGradientAttributeResponse, AvoReflectivityMethod, AvoReflectivityRequest,
    AvoReflectivityResponse, AxisSummaryF32, AxisSummaryI32, CancelProcessingJobRequest,
    CancelProcessingJobResponse, CoordinateReferenceBinding, CoordinateReferenceDescriptor,
    CoordinateReferenceSource, DatasetId, DeletePipelinePresetRequest,
    DeletePipelinePresetResponse, FrequencyPhaseMode, FrequencyWindowShape, GatherAxisKind,
    GatherInterpolationMode, GatherPreviewView, GatherProbe, GatherProbeChanged,
    GatherProcessingOperation, GatherProcessingPipeline, GatherRequest, GatherSampleDomain,
    GatherSelector, GatherView, GatherViewport, GatherViewportChanged, GeometryDescriptor,
    GeometryProvenanceSummary, GeometrySummary, GetProcessingJobRequest, GetProcessingJobResponse,
    ImportHorizonXyzRequest, ImportHorizonXyzResponse, ImportedHorizonDescriptor,
    InterpretationPoint, ListPipelinePresetsResponse, LoadSectionHorizonsRequest,
    LoadSectionHorizonsResponse, LocalVolumeStatistic, NeighborhoodDipOutput,
    PostStackNeighborhoodProcessingOperation, PostStackNeighborhoodProcessingPipeline,
    PostStackNeighborhoodWindow, PreviewGatherProcessingRequest, PreviewGatherProcessingResponse,
    PreviewPostStackNeighborhoodProcessingRequest, PreviewPostStackNeighborhoodProcessingResponse,
    PreviewProcessingRequest, PreviewProcessingResponse, PreviewResponse,
    PreviewSubvolumeProcessingRequest, PreviewSubvolumeProcessingResponse,
    PreviewTraceLocalProcessingRequest, PreviewTraceLocalProcessingResponse, PreviewView,
    ProcessingArtifactRole, ProcessingJobChunkPlanSummary, ProcessingJobExecutionSummary,
    ProcessingJobPlanSummary, ProcessingJobProgress, ProcessingJobStageExecutionSummary,
    ProcessingJobState, ProcessingJobStatus, ProcessingLayoutCompatibility, ProcessingOperation,
    ProcessingOperatorDependencyProfile, ProcessingOperatorScope, ProcessingPipeline,
    ProcessingPipelineFamily, ProcessingPipelineSpec, ProcessingPreset, ProcessingSampleDependency,
    ProcessingSpatialDependency, ProjectedPoint2, ProjectedPolygon2, ProjectedVector2,
    RockPhysicsAttributeMethod, RockPhysicsAttributeRequest, RockPhysicsAttributeResponse,
    RunGatherProcessingRequest, RunGatherProcessingResponse,
    RunPostStackNeighborhoodProcessingRequest, RunPostStackNeighborhoodProcessingResponse,
    RunProcessingRequest, RunProcessingResponse, RunSubvolumeProcessingRequest,
    RunSubvolumeProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SavePipelinePresetRequest, SavePipelinePresetResponse,
    SectionAxis, SectionCoordinate, SectionDisplayDefaults, SectionHorizonLineStyle,
    SectionHorizonOverlayView, SectionHorizonSample, SectionHorizonStyle, SectionMetadata,
    SectionProbe, SectionProbeChanged, SectionRenderMode, SectionRequest, SectionSpectrumSelection,
    SectionTileRequest, SectionUnits, SectionView, SectionViewport, SectionViewportChanged,
    SeismicLayout, SemblancePanel, SubvolumeCropOperation, SubvolumeProcessingPipeline,
    SurveyGridTransform, SurveyPropertyField3D, SurveySpatialAvailability, SurveySpatialDescriptor,
    TraceLocalProcessingOperation, TraceLocalProcessingPipeline,
    TraceLocalVolumeArithmeticOperator, VelocityAutopickParameters, VelocityFunctionEstimate,
    VelocityFunctionSource, VelocityPickStrategy, VelocityQuantityKind, VelocityScanRequest,
    VelocityScanResponse, VolumeDescriptor,
};
pub use planner::{
    AdaptivePartitionTargetRecommendation, PlanProcessingRequest, build_execution_plan,
    recommend_adaptive_partition_target, recommend_adaptive_partition_target_for_job_concurrency,
    recommend_trace_local_chunk_plan_for_execution,
};
pub use post_stack_neighborhood::{
    materialize_post_stack_neighborhood_processing_volume,
    materialize_post_stack_neighborhood_processing_volume_with_progress,
    preview_post_stack_neighborhood_processing_section_view,
    preview_post_stack_neighborhood_processing_section_view_with_prefix_cache,
    validate_post_stack_neighborhood_processing_pipeline,
    validate_post_stack_neighborhood_processing_pipeline_for_layout,
};
pub use preflight::{PreflightAction, PreflightGeometry, SurveyPreflight, preflight_segy};
pub use prestack_analysis::velocity_scan;
pub use prestack_store::{
    PrestackStoreHandle, TbgathManifest, TbgathReader, TbgathWriter, create_tbgath_store,
    describe_prestack_store, gather_view as prestack_gather_view,
    materialize_gather_processing_store, materialize_gather_processing_store_with_progress,
    open_prestack_store, preview_gather_processing_view,
    read_gather_plane as read_prestack_gather_plane,
    set_prestack_store_native_coordinate_reference,
};
pub use render::{render_section_csv, render_section_csv_for_request};
pub use rock_physics::{avo_intercept_gradient_attribute, rock_physics_attribute};
pub use segy_export::{
    attach_store_segy_export, copy_store_segy_export, crop_store_segy_export, export_store_to_segy,
};
pub use storage::section_assembler::read_section_plane as assemble_section_plane;
pub use storage::tbvol::{
    TbvolManifest, TbvolReader, TbvolWriter, recommended_default_tbvol_tile_target_mib,
    recommended_tbvol_tile_shape,
};
pub use storage::tbvolc::{
    TbvolArchiveSiblingStatus, TbvolcAmplitudeEncoding, TbvolcManifest, TbvolcReader, TbvolcWriter,
    describe_tbvol_archive_sibling, suggested_tbvol_restore_path, suggested_tbvolc_archive_path,
    transcode_tbvol_to_tbvolc, transcode_tbvolc_to_tbvol,
};
pub use storage::tile_geometry::{TileCoord, TileGeometry};
pub use storage::volume_store::{
    OccupancyTile, TileBuffer, VolumeStoreReader, VolumeStoreWriter, write_dense_volume,
};
pub use storage::zarr::{ZarrVolumeStoreReader, ZarrVolumeStoreWriter};
pub use store::{
    SectionPlane, SectionTileView, StoreHandle, create_tbvol_store, describe_store, load_array,
    load_occupancy, open_store, read_section_plane, section_tile_view, section_view,
    set_store_native_coordinate_reference, set_store_vertical_axis,
};
pub use survey_time_depth::{
    SectionSurveyTimeDepthTransformSlice, StoredSurveyPropertyField,
    StoredSurveyTimeDepthTransform, build_survey_property_field, build_survey_time_depth_transform,
    build_survey_time_depth_transform_from_horizon_pairs, load_survey_property_field,
    load_survey_property_fields, load_survey_time_depth_transform,
    load_survey_time_depth_transforms, section_time_depth_transform_slice,
    store_survey_property_field, store_survey_time_depth_transform,
};
pub use time_depth::{
    convert_section_view_to_depth, depth_converted_section_view, resolved_section_display_view,
};
pub use zarr_export::{
    default_zarr_storage_layout, export_store_to_zarr, export_store_to_zarr_with_layout,
};

use std::path::Path;

use serde::Serialize;

pub use metadata::segy_sample_data_fidelity;

#[derive(Debug, Clone, Serialize)]
pub struct SegyInspection {
    pub file_size: u64,
    pub trace_count: u64,
    pub samples_per_trace: u16,
    pub sample_interval_us: u16,
    pub sample_format_code: u16,
    pub fixed_length_trace: Option<bool>,
    pub endianness: String,
    pub warnings: Vec<String>,
}

pub fn inspect_segy(path: impl AsRef<Path>) -> Result<SegyInspection, SeismicStoreError> {
    let summary = ophiolite_seismic_io::inspect_file(path)?;
    Ok(SegyInspection {
        file_size: summary.file_size,
        trace_count: summary.trace_count,
        samples_per_trace: summary.samples_per_trace,
        sample_interval_us: summary.sample_interval_us,
        sample_format_code: summary.sample_format_code,
        fixed_length_trace: summary.fixed_length_trace,
        endianness: format!("{:?}", summary.endianness),
        warnings: summary
            .warnings
            .iter()
            .map(|warning| format!("{warning:?}"))
            .collect(),
    })
}

pub fn set_any_store_native_coordinate_reference(
    root: impl AsRef<Path>,
    coordinate_reference_id: Option<&str>,
    coordinate_reference_name: Option<&str>,
) -> Result<VolumeDescriptor, SeismicStoreError> {
    let root = root.as_ref();
    match set_store_native_coordinate_reference(
        root,
        coordinate_reference_id,
        coordinate_reference_name,
    ) {
        Ok(descriptor) => Ok(descriptor),
        Err(SeismicStoreError::Json(_)) | Err(SeismicStoreError::MissingManifest(_)) => {
            set_prestack_store_native_coordinate_reference(
                root,
                coordinate_reference_id,
                coordinate_reference_name,
            )
        }
        Err(error) => Err(error),
    }
}
