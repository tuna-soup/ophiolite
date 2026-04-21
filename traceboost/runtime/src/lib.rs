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
pub use ophiolite_seismic::{
    AmplitudeSpectrumCurve, AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, AxisSummaryF32,
    AxisSummaryI32, BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest,
    CancelProcessingJobRequest, CancelProcessingJobResponse, CoordinateReferenceBinding,
    CoordinateReferenceDescriptor, CoordinateReferenceSource, DatasetId,
    DeletePipelinePresetRequest, DeletePipelinePresetResponse, DepthReferenceKind,
    FrequencyPhaseMode, FrequencyWindowShape, GatherInterpolationMode, GatherPreviewView,
    GatherProcessingOperation, GatherProcessingPipeline, GatherRequest, GatherSelector,
    GeometryDescriptor, GeometryProvenanceSummary, GeometrySummary, GetProcessingJobRequest,
    GetProcessingJobResponse, ImportHorizonXyzRequest, ImportHorizonXyzResponse,
    ImportPrestackOffsetDatasetRequest, ImportPrestackOffsetDatasetResponse,
    ImportedHorizonDescriptor, InterpretationPoint, LateralInterpolationMethod,
    LayeredVelocityInterval, LayeredVelocityModel, ListPipelinePresetsResponse,
    LoadSectionHorizonsRequest, LoadSectionHorizonsResponse, PrestackThirdAxisField,
    PreviewGatherProcessingRequest, PreviewGatherProcessingResponse, PreviewProcessingRequest,
    PreviewProcessingResponse, PreviewTraceLocalProcessingRequest,
    PreviewTraceLocalProcessingResponse, ProcessingArtifactRole, ProcessingJobArtifact,
    ProcessingJobArtifactKind, ProcessingJobProgress, ProcessingJobState, ProcessingJobStatus,
    ProcessingOperation, ProcessingPipeline, ProcessingPipelineFamily, ProcessingPipelineSpec,
    ProcessingPreset, ProjectedPoint2, ProjectedVector2, ResolvedSectionDisplayView,
    RunGatherProcessingRequest, RunGatherProcessingResponse, RunProcessingRequest,
    RunProcessingResponse, RunTraceLocalProcessingRequest, RunTraceLocalProcessingResponse,
    SampleDataConversionKind, SampleDataFidelity, SampleValuePreservation,
    SavePipelinePresetRequest, SavePipelinePresetResponse, SectionAxis, SectionHorizonLineStyle,
    SectionHorizonOverlayView, SectionHorizonSample, SectionHorizonStyle, SectionRequest,
    SectionSpectrumSelection, SectionTileRequest, SemblancePanel, SpatialCoverageRelationship,
    SpatialCoverageSummary, StratigraphicBoundaryReference, SubvolumeProcessingPipeline,
    SurveyGridTransform, SurveyPropertyField3D, SurveySpatialAvailability, SurveySpatialDescriptor,
    SurveyTimeDepthTransform3D, TimeDepthDomain, TimeDepthTransformSourceKind,
    TraceLocalProcessingOperation, TraceLocalProcessingPipeline, TraceLocalProcessingPreset,
    TraceLocalProcessingStep, TraceLocalVolumeArithmeticOperator, TravelTimeReference,
    VelocityAutopickParameters, VelocityControlProfile, VelocityControlProfileSample,
    VelocityControlProfileSet, VelocityFunctionEstimate, VelocityFunctionSource,
    VelocityIntervalTrend, VelocityPickStrategy, VelocityQuantityKind, VelocityScanRequest,
    VelocityScanResponse, VelocitySource3D, VerticalAxisDescriptor, VerticalInterpolationMethod,
    VolumeDescriptor,
};
pub use ophiolite_seismic::{PreviewView, SectionView};
pub use ophiolite_seismic_runtime::{
    HorizonImportPreview, HorizonImportPreviewFile, HorizonSourceImportCanonicalDraft,
    HorizonSourceImportPreview,
};
pub use ophiolite_seismic_runtime::{
    MaterializeOptions, PreviewSectionPrefixCache, PreviewSectionPrefixReuse,
    PreviewSectionSession, SeismicStoreError, amplitude_spectrum_from_plane,
    amplitude_spectrum_from_reader, amplitude_spectrum_from_store, apply_pipeline_to_plane,
    apply_pipeline_to_traces, build_survey_property_field, build_survey_time_depth_transform,
    build_survey_time_depth_transform_from_horizon_pairs, convert_section_view_to_depth,
    default_zarr_storage_layout, depth_converted_section_view, export_store_to_segy,
    export_store_to_zarr, export_store_to_zarr_with_layout, load_survey_property_fields,
    load_survey_time_depth_transforms, materialize_from_reader_writer,
    materialize_from_reader_writer_with_progress, materialize_processing_volume,
    materialize_processing_volume_with_progress, materialize_subvolume_processing_volume,
    materialize_subvolume_processing_volume_with_progress, materialize_volume,
    preview_processing_section_plane, preview_processing_section_view,
    preview_processing_section_view_with_prefix_cache, preview_section_from_reader,
    preview_section_plane, preview_section_view, preview_section_view_with_prefix_cache,
    preview_subvolume_processing_section_view, resolved_section_display_view,
    store_survey_property_field, store_survey_time_depth_transform, validate_pipeline,
    validate_processing_pipeline, velocity_scan,
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
    TbvolArchiveSiblingStatus, TbvolcAmplitudeEncoding, TbvolcManifest,
};
pub use ophiolite_seismic_runtime::{
    materialize_gather_processing_store, materialize_gather_processing_store_with_progress,
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
