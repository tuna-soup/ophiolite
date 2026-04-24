use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

#[cfg(unix)]
use std::{ffi::CString, os::unix::ffi::OsStrExt};

use ophiolite::{
    OperatorCatalog, SeismicLayout, SeismicTraceDataDescriptor, operator_catalog_for_trace_data,
    resolve_dataset_summary_survey_map_source,
};
use ophiolite_seismic_execution::ProcessingExecutionService;
use seis_contracts_operations::datasets::{
    DatasetSummary, OpenDatasetRequest, OpenDatasetResponse,
};
use seis_contracts_operations::import_ops::{
    ExportSegyRequest, ExportSegyResponse, ImportDatasetRequest, ImportDatasetResponse,
    ImportHorizonXyzRequest, ImportHorizonXyzResponse, ImportPrestackOffsetDatasetRequest,
    ImportPrestackOffsetDatasetResponse, ImportSegyWithPlanRequest, ImportSegyWithPlanResponse,
    LoadSectionHorizonsRequest, LoadSectionHorizonsResponse, PrestackThirdAxisField,
    ScanSegyImportRequest, SegyGeometryCandidate, SegyGeometryOverride, SegyHeaderField,
    SegyHeaderValueType, SegyImportCandidatePlan, SegyImportFieldObservation, SegyImportIssue,
    SegyImportIssueSection, SegyImportIssueSeverity, SegyImportPlan, SegyImportPlanSource,
    SegyImportPolicy, SegyImportProvenance, SegyImportResolvedDataset, SegyImportResolvedSpatial,
    SegyImportRiskSummary, SegyImportScanResponse, SegyImportSparseHandling, SegyImportSpatialPlan,
    SegyImportValidationResponse, SegyImportWizardStage, SuggestedImportAction,
    SurveyPreflightRequest, SurveyPreflightResponse, ValidateSegyImportPlanRequest,
};
use seis_contracts_operations::processing_ops::{
    AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, GatherProcessingPipeline, GatherRequest,
    GatherView, NeighborhoodDipOutput, PostStackNeighborhoodProcessingPipeline,
    PreviewGatherProcessingRequest, PreviewGatherProcessingResponse,
    PreviewPostStackNeighborhoodProcessingRequest, PreviewPostStackNeighborhoodProcessingResponse,
    PreviewSubvolumeProcessingRequest, PreviewSubvolumeProcessingResponse,
    PreviewTraceLocalProcessingRequest, PreviewTraceLocalProcessingResponse,
    RunGatherProcessingRequest, RunPostStackNeighborhoodProcessingRequest,
    RunSubvolumeProcessingRequest, RunTraceLocalProcessingRequest, SubvolumeProcessingPipeline,
    VelocityFunctionSource, VelocityScanRequest, VelocityScanResponse,
};
use seis_contracts_operations::resolve::IPC_SCHEMA_VERSION;
use seis_contracts_operations::resolve::{
    ResolveSurveyMapRequest, ResolveSurveyMapResponse, SetDatasetNativeCoordinateReferenceRequest,
    SetDatasetNativeCoordinateReferenceResponse,
};
use seis_contracts_operations::workspace::{
    DescribeVelocityVolumeRequest, DescribeVelocityVolumeResponse, IngestVelocityVolumeRequest,
    IngestVelocityVolumeResponse, LoadVelocityModelsResponse,
};
use seis_io::HeaderField;
use seis_runtime::{
    BuildSurveyTimeDepthTransformRequest, DepthReferenceKind, ExecutionPriorityClass,
    GatherInterpolationMode, HorizonImportPreview, ImportedHorizonDescriptor, IngestOptions,
    LateralInterpolationMethod, LayeredVelocityInterval, LayeredVelocityModel, MaterializeOptions,
    PartitionExecutionProgress, PlanProcessingRequest, PlanningMode, PreflightAction, PreviewView,
    ProcessingBatchItemRequest, ProcessingBatchState, ProcessingExecutionMode,
    ProcessingJobChunkPlanSummary, ProcessingJobExecutionSummary, ProcessingJobState,
    ProcessingPipelineSpec, ProcessingSchedulerReason, ProjectedPoint2, ResolvedSectionDisplayView,
    SeisGeometryOptions, SparseSurveyPolicy, SpatialCoverageRelationship, SpatialCoverageSummary,
    StratigraphicBoundaryReference, SurveyTimeDepthTransform3D, TileGeometry, TimeDepthDomain,
    TimeDepthTransformSourceKind, TraceLocalProcessingOperation, TraceLocalProcessingPipeline,
    TraceLocalProcessingStep, TravelTimeReference, VelocityControlProfile,
    VelocityControlProfileSample, VelocityControlProfileSet, VelocityIntervalTrend,
    VelocityQuantityKind, VelocitySource3D, VerticalAxisDescriptor, VerticalInterpolationMethod,
    VolumeImportFormat, amplitude_spectrum_from_store, build_execution_plan,
    build_survey_time_depth_transform, build_survey_time_depth_transform_from_horizon_pairs,
    convert_horizon_vertical_domain_with_transform, depth_converted_section_view,
    describe_prestack_store, describe_store, detect_volume_import_format,
    estimate_mdio_tbvol_storage, export_store_to_segy, export_store_to_zarr,
    import_horizon_xyzs_with_vertical_domain, ingest_prestack_offset_segy, ingest_volume,
    load_horizon_grids, load_survey_time_depth_transforms, materialize_gather_processing_store,
    materialize_post_stack_neighborhood_processing_volume, materialize_processing_volume,
    materialize_processing_volume_with_partition_progress, materialize_subvolume_processing_volume,
    open_prestack_store, open_store, preflight_segy, prestack_gather_view,
    preview_gather_processing_view, preview_horizon_xyzs_with_vertical_domain,
    preview_post_stack_neighborhood_processing_section_view, preview_processing_section_view,
    preview_subvolume_processing_section_view, recommended_default_tbvol_tile_target_mib,
    recommended_tbvol_tile_shape, resolve_trace_local_materialize_options,
    resolved_section_display_view, section_horizon_overlays,
    set_any_store_native_coordinate_reference, set_store_vertical_axis,
    store_survey_time_depth_transform, velocity_scan,
};
use serde::Serialize;

const DEFAULT_SPARSE_FILL_VALUE: f32 = 0.0;
const DEMO_SURVEY_TIME_DEPTH_TRANSFORM_ID: &str = "demo-survey-3d-transform";
const DEMO_SURVEY_TIME_DEPTH_TRANSFORM_NAME: &str = "Synthetic Survey 3D Time-Depth Transform";
const IMPORT_FREE_SPACE_RESERVE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct TraceBoostWorkflowService;

#[derive(Debug, Clone, Serialize)]
pub struct ExportZarrResponse {
    pub store_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportVelocityFunctionsModelResponse {
    pub schema_version: u32,
    pub input_path: String,
    pub velocity_kind: VelocityQuantityKind,
    pub profile_count: usize,
    pub sample_count: usize,
    pub model: SurveyTimeDepthTransform3D,
}

#[derive(Debug, Clone)]
pub struct PrepareSurveyDemoRequest {
    pub store_path: String,
    pub display_coordinate_reference_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrepareSurveyDemoResponse {
    pub store_path: String,
    pub ensured_time_depth_transform_id: String,
    pub velocity_models: LoadVelocityModelsResponse,
    pub survey_map: ResolveSurveyMapResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceLocalBenchmarkScenario {
    Scalar,
    Agc,
    Analytic,
    Bandpass,
}

#[derive(Debug, Clone)]
pub struct TraceLocalBenchmarkRequest {
    pub store_path: String,
    pub output_root: Option<String>,
    pub scenario: TraceLocalBenchmarkScenario,
    pub partition_target_mib: Vec<u64>,
    pub adaptive_partition_target: bool,
    pub include_serial: bool,
    pub repeat_count: usize,
    pub keep_outputs: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkStageClassificationSummary {
    pub stage_id: String,
    pub stage_label: String,
    pub stage_kind: seis_runtime::ExecutionStageKind,
    pub partition_family: seis_runtime::PartitionFamily,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_partition_count: Option<usize>,
    pub classification: seis_runtime::StageExecutionClassification,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalChunkPlanBenchmarkRecommendation {
    pub target_bytes: u64,
    pub bytes_per_tile: u64,
    pub total_tiles: usize,
    pub preferred_partition_count: usize,
    pub recommended_partition_count: usize,
    pub recommended_max_active_partitions: usize,
    pub tiles_per_partition: usize,
    pub resident_partition_bytes: u64,
    pub global_worker_workspace_bytes: u64,
    pub estimated_peak_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_memory_bytes: Option<u64>,
    pub reserved_memory_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usable_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBenchmarkRunResult {
    pub label: String,
    pub scenario: TraceLocalBenchmarkScenario,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_target_bytes: Option<u64>,
    pub repeat_index: usize,
    pub elapsed_ms: f64,
    pub total_tiles: usize,
    pub completed_tiles: usize,
    pub total_partitions: usize,
    pub completed_partitions: usize,
    pub peak_active_partitions: usize,
    pub retry_count: usize,
    pub output_bytes: u64,
    pub output_store_path: String,
    pub output_retained: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBenchmarkVariantSummary {
    pub label: String,
    pub scenario: TraceLocalBenchmarkScenario,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_target_bytes: Option<u64>,
    pub run_count: usize,
    pub avg_elapsed_ms: f64,
    pub min_elapsed_ms: f64,
    pub max_elapsed_ms: f64,
    pub total_tiles: usize,
    pub avg_total_partitions: f64,
    pub avg_peak_active_partitions: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBenchmarkResponse {
    pub store_path: String,
    pub scenario: TraceLocalBenchmarkScenario,
    pub source_shape: [usize; 3],
    pub source_chunk_shape: [usize; 3],
    pub adaptive_partition_target: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_recommendation: Option<TraceLocalChunkPlanBenchmarkRecommendation>,
    pub pipeline: TraceLocalProcessingPipeline,
    pub plan_summary: seis_runtime::ExecutionPlanSummary,
    pub stage_classifications: Vec<BenchmarkStageClassificationSummary>,
    pub runs: Vec<TraceLocalBenchmarkRunResult>,
    pub variants: Vec<TraceLocalBenchmarkVariantSummary>,
}

#[derive(Debug, Clone)]
pub struct TraceLocalBatchBenchmarkRequest {
    pub store_path: String,
    pub output_root: Option<String>,
    pub scenario: TraceLocalBenchmarkScenario,
    pub job_count: usize,
    pub max_active_jobs: Vec<usize>,
    pub execution_mode: Option<ProcessingExecutionMode>,
    pub partition_target_mib: u64,
    pub adaptive_partition_target: bool,
    pub repeat_count: usize,
    pub keep_outputs: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBatchBenchmarkJobResult {
    pub job_label: String,
    pub job_id: String,
    pub state: String,
    pub queue_wait_ms: f64,
    pub elapsed_ms: f64,
    pub total_tiles: usize,
    pub completed_tiles: usize,
    pub total_partitions: usize,
    pub completed_partitions: usize,
    pub peak_active_partitions: usize,
    pub retry_count: usize,
    pub output_bytes: u64,
    pub output_store_path: String,
    pub output_retained: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBatchBenchmarkVariantResult {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_max_active_jobs: Option<usize>,
    pub effective_max_active_jobs: usize,
    pub execution_mode: ProcessingExecutionMode,
    pub scheduler_reason: ProcessingSchedulerReason,
    pub worker_budget: usize,
    pub global_cap: usize,
    pub max_memory_cost_class: seis_runtime::MemoryCostClass,
    pub max_cpu_cost_class: seis_runtime::CpuCostClass,
    pub max_io_cost_class: seis_runtime::IoCostClass,
    pub min_parallel_efficiency_class: seis_runtime::ParallelEfficiencyClass,
    pub max_estimated_peak_memory_bytes: u64,
    pub combined_cpu_weight: f32,
    pub combined_io_weight: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_expected_partition_count: Option<usize>,
    pub partition_target_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_recommendation: Option<TraceLocalChunkPlanBenchmarkRecommendation>,
    pub repeat_index: usize,
    pub batch_elapsed_ms: f64,
    pub completed_jobs: usize,
    pub total_jobs: usize,
    pub avg_queue_wait_ms: f64,
    pub max_queue_wait_ms: f64,
    pub avg_job_elapsed_ms: f64,
    pub min_job_elapsed_ms: f64,
    pub max_job_elapsed_ms: f64,
    pub avg_total_partitions: f64,
    pub avg_peak_active_partitions: f64,
    pub jobs: Vec<TraceLocalBatchBenchmarkJobResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBatchBenchmarkSummary {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_max_active_jobs: Option<usize>,
    pub effective_max_active_jobs: usize,
    pub execution_mode: ProcessingExecutionMode,
    pub scheduler_reason: ProcessingSchedulerReason,
    pub run_count: usize,
    pub avg_batch_elapsed_ms: f64,
    pub min_batch_elapsed_ms: f64,
    pub max_batch_elapsed_ms: f64,
    pub avg_queue_wait_ms: f64,
    pub avg_job_elapsed_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceLocalBatchBenchmarkResponse {
    pub store_path: String,
    pub scenario: TraceLocalBenchmarkScenario,
    pub source_shape: [usize; 3],
    pub source_chunk_shape: [usize; 3],
    pub job_count: usize,
    pub partition_target_bytes: u64,
    pub adaptive_partition_target: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_recommendation: Option<TraceLocalChunkPlanBenchmarkRecommendation>,
    pub pipeline: TraceLocalProcessingPipeline,
    pub plan_summary: seis_runtime::ExecutionPlanSummary,
    pub stage_classifications: Vec<BenchmarkStageClassificationSummary>,
    pub variants: Vec<TraceLocalBatchBenchmarkVariantResult>,
    pub summaries: Vec<TraceLocalBatchBenchmarkSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStackNeighborhoodBenchmarkOperator {
    Similarity,
    LocalVolumeStatsMean,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PostStackNeighborhoodBenchmarkWindow {
    pub gate_ms: f32,
    pub inline_stepout: usize,
    pub xline_stepout: usize,
}

#[derive(Debug, Clone)]
pub struct PostStackNeighborhoodPreviewBenchmarkRequest {
    pub store_path: String,
    pub operator: PostStackNeighborhoodBenchmarkOperator,
    pub gate_ms: f32,
    pub inline_stepout: usize,
    pub xline_stepout: usize,
    pub section_axis: seis_runtime::SectionAxis,
    pub section_index: usize,
    pub include_trace_local_prefix: bool,
    pub repeat_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostStackNeighborhoodPreviewBenchmarkRunResult {
    pub label: String,
    pub operator: PostStackNeighborhoodBenchmarkOperator,
    pub repeat_index: usize,
    pub elapsed_ms: f64,
    pub time_to_first_result_ms: f64,
    pub section_axis: seis_runtime::SectionAxis,
    pub section_index: usize,
    pub trace_local_prefix_applied: bool,
    pub preview_ready: bool,
    pub traces: usize,
    pub samples: usize,
    pub amplitude_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostStackNeighborhoodPreviewBenchmarkSummary {
    pub label: String,
    pub run_count: usize,
    pub avg_elapsed_ms: f64,
    pub min_elapsed_ms: f64,
    pub max_elapsed_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostStackNeighborhoodPreviewBenchmarkResponse {
    pub store_path: String,
    pub operator: PostStackNeighborhoodBenchmarkOperator,
    pub window: PostStackNeighborhoodBenchmarkWindow,
    pub source_shape: [usize; 3],
    pub source_chunk_shape: [usize; 3],
    pub section_axis: seis_runtime::SectionAxis,
    pub section_index: usize,
    pub include_trace_local_prefix: bool,
    pub pipeline: PostStackNeighborhoodProcessingPipeline,
    pub plan_summary: seis_runtime::ExecutionPlanSummary,
    pub stage_classifications: Vec<BenchmarkStageClassificationSummary>,
    pub runs: Vec<PostStackNeighborhoodPreviewBenchmarkRunResult>,
    pub summaries: Vec<PostStackNeighborhoodPreviewBenchmarkSummary>,
}

#[derive(Debug, Clone)]
pub struct PostStackNeighborhoodProcessingBenchmarkRequest {
    pub store_path: String,
    pub output_root: Option<String>,
    pub operator: PostStackNeighborhoodBenchmarkOperator,
    pub gate_ms: f32,
    pub inline_stepout: usize,
    pub xline_stepout: usize,
    pub include_trace_local_prefix: bool,
    pub repeat_count: usize,
    pub keep_outputs: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostStackNeighborhoodProcessingBenchmarkRunResult {
    pub label: String,
    pub operator: PostStackNeighborhoodBenchmarkOperator,
    pub repeat_index: usize,
    pub elapsed_ms: f64,
    pub trace_local_prefix_applied: bool,
    pub output_bytes: u64,
    pub output_store_path: String,
    pub output_retained: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostStackNeighborhoodProcessingBenchmarkSummary {
    pub label: String,
    pub run_count: usize,
    pub avg_elapsed_ms: f64,
    pub min_elapsed_ms: f64,
    pub max_elapsed_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostStackNeighborhoodProcessingBenchmarkResponse {
    pub store_path: String,
    pub operator: PostStackNeighborhoodBenchmarkOperator,
    pub window: PostStackNeighborhoodBenchmarkWindow,
    pub source_shape: [usize; 3],
    pub source_chunk_shape: [usize; 3],
    pub include_trace_local_prefix: bool,
    pub pipeline: PostStackNeighborhoodProcessingPipeline,
    pub plan_summary: seis_runtime::ExecutionPlanSummary,
    pub stage_classifications: Vec<BenchmarkStageClassificationSummary>,
    pub runs: Vec<PostStackNeighborhoodProcessingBenchmarkRunResult>,
    pub summaries: Vec<PostStackNeighborhoodProcessingBenchmarkSummary>,
}

#[derive(Debug, Clone)]
struct ParsedVelocityFunctions {
    profiles: Vec<VelocityControlProfile>,
    sample_count: usize,
}

#[derive(Debug, Clone)]
struct ParsedVelocityProfileRow {
    x: f64,
    y: f64,
    sample: VelocityControlProfileSample,
}

#[derive(Debug, Clone)]
struct VelocityVolumeDescriptorOptions {
    vertical_domain: TimeDepthDomain,
    vertical_unit: String,
    vertical_start: Option<f32>,
    vertical_step: Option<f32>,
}

impl Default for VelocityVolumeDescriptorOptions {
    fn default() -> Self {
        Self {
            vertical_domain: TimeDepthDomain::Time,
            vertical_unit: "ms".to_string(),
            vertical_start: None,
            vertical_step: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct VelocityNavigationIndex {
    points_by_line: HashMap<String, HashMap<i32, ProjectedPoint2>>,
}

#[derive(Debug, Clone)]
struct GeometryCandidateSpec {
    label: &'static str,
    inline: (u16, SegyHeaderValueType),
    crossline: (u16, SegyHeaderValueType),
}

const GEOMETRY_CANDIDATE_SPECS: [GeometryCandidateSpec; 7] = [
    GeometryCandidateSpec {
        label: "Legacy EP / trace-in-record (17/13)",
        inline: (17, SegyHeaderValueType::I32),
        crossline: (13, SegyHeaderValueType::I32),
    },
    GeometryCandidateSpec {
        label: "CDP / trace-in-record (21/13)",
        inline: (21, SegyHeaderValueType::I32),
        crossline: (13, SegyHeaderValueType::I32),
    },
    GeometryCandidateSpec {
        label: "EP / trace-sequence-file (17/9)",
        inline: (17, SegyHeaderValueType::I32),
        crossline: (9, SegyHeaderValueType::I32),
    },
    GeometryCandidateSpec {
        label: "CDP / trace-sequence-file (21/9)",
        inline: (21, SegyHeaderValueType::I32),
        crossline: (9, SegyHeaderValueType::I32),
    },
    GeometryCandidateSpec {
        label: "Trace-sequence-file / trace-in-record (9/13)",
        inline: (9, SegyHeaderValueType::I32),
        crossline: (13, SegyHeaderValueType::I32),
    },
    GeometryCandidateSpec {
        label: "Trace-sequence-line / trace-in-record (1/13)",
        inline: (1, SegyHeaderValueType::I32),
        crossline: (13, SegyHeaderValueType::I32),
    },
    GeometryCandidateSpec {
        label: "Trace-sequence-line / trace-sequence-file (1/9)",
        inline: (1, SegyHeaderValueType::I32),
        crossline: (9, SegyHeaderValueType::I32),
    },
];

impl TraceBoostWorkflowService {
    pub fn backend_info(&self) -> serde_json::Value {
        serde_json::json!({
            "backend_repo_hint": "monorepo: runtime/",
            "backend_local_path_hint": "../../runtime",
            "current_default_method_policy": "keep linear as default unless a stronger method wins on every validation dataset",
            "current_geometry_policy": "dense surveys ingest directly; sparse regular post-stack surveys require explicit regularization; duplicate-heavy surveys still stop for review",
            "current_scope": "monorepo app shell with preflight and ingest routing; Tauri app not started yet",
        })
    }

    pub fn preflight_dataset(
        &self,
        request: SurveyPreflightRequest,
    ) -> Result<SurveyPreflightResponse, Box<dyn std::error::Error>> {
        preflight_dataset(request)
    }

    pub fn import_dataset(
        &self,
        request: ImportDatasetRequest,
    ) -> Result<ImportDatasetResponse, Box<dyn std::error::Error>> {
        import_dataset(request)
    }

    pub fn scan_segy_import(
        &self,
        request: ScanSegyImportRequest,
    ) -> Result<SegyImportScanResponse, Box<dyn std::error::Error>> {
        scan_segy_import(request)
    }

    pub fn validate_segy_import_plan(
        &self,
        request: ValidateSegyImportPlanRequest,
    ) -> Result<SegyImportValidationResponse, Box<dyn std::error::Error>> {
        validate_segy_import_plan(request)
    }

    pub fn import_segy_with_plan(
        &self,
        request: ImportSegyWithPlanRequest,
    ) -> Result<ImportSegyWithPlanResponse, Box<dyn std::error::Error>> {
        import_segy_with_plan(request)
    }

    pub fn import_prestack_offset_dataset(
        &self,
        request: ImportPrestackOffsetDatasetRequest,
    ) -> Result<ImportPrestackOffsetDatasetResponse, Box<dyn std::error::Error>> {
        import_prestack_offset_dataset(request)
    }

    pub fn open_dataset_summary(
        &self,
        request: OpenDatasetRequest,
    ) -> Result<OpenDatasetResponse, Box<dyn std::error::Error>> {
        open_dataset_summary(request)
    }

    pub fn dataset_operator_catalog(
        &self,
        store_path: String,
    ) -> Result<OperatorCatalog, Box<dyn std::error::Error>> {
        dataset_operator_catalog(store_path)
    }

    pub fn set_dataset_native_coordinate_reference(
        &self,
        request: SetDatasetNativeCoordinateReferenceRequest,
    ) -> Result<SetDatasetNativeCoordinateReferenceResponse, Box<dyn std::error::Error>> {
        set_dataset_native_coordinate_reference(request)
    }

    pub fn resolve_survey_map(
        &self,
        request: ResolveSurveyMapRequest,
    ) -> Result<ResolveSurveyMapResponse, Box<dyn std::error::Error>> {
        resolve_survey_map(request)
    }

    pub fn export_dataset_segy(
        &self,
        request: ExportSegyRequest,
    ) -> Result<ExportSegyResponse, Box<dyn std::error::Error>> {
        export_dataset_segy(request)
    }

    pub fn export_dataset_zarr(
        &self,
        store_path: String,
        output_path: String,
        overwrite_existing: bool,
    ) -> Result<ExportZarrResponse, Box<dyn std::error::Error>> {
        export_dataset_zarr(store_path, output_path, overwrite_existing)
    }

    pub fn import_horizon_xyz(
        &self,
        request: ImportHorizonXyzRequest,
    ) -> Result<ImportHorizonXyzResponse, Box<dyn std::error::Error>> {
        import_horizon_xyz(request)
    }

    pub fn preview_horizon_xyz_import(
        &self,
        request: ImportHorizonXyzRequest,
    ) -> Result<HorizonImportPreview, Box<dyn std::error::Error>> {
        preview_horizon_xyz_import(request)
    }

    pub fn load_section_horizons(
        &self,
        request: LoadSectionHorizonsRequest,
    ) -> Result<LoadSectionHorizonsResponse, Box<dyn std::error::Error>> {
        load_section_horizons(request)
    }

    pub fn load_velocity_models(
        &self,
        store_path: String,
    ) -> Result<LoadVelocityModelsResponse, Box<dyn std::error::Error>> {
        load_velocity_models(store_path)
    }

    pub fn describe_velocity_volume_store(
        &self,
        store_path: String,
        velocity_kind: VelocityQuantityKind,
        vertical_domain: TimeDepthDomain,
        vertical_unit: Option<String>,
        vertical_start: Option<f32>,
        vertical_step: Option<f32>,
    ) -> Result<VelocitySource3D, Box<dyn std::error::Error>> {
        describe_velocity_volume_store_with_options(
            store_path,
            velocity_kind,
            VelocityVolumeDescriptorOptions {
                vertical_domain,
                vertical_unit: vertical_unit
                    .unwrap_or_else(|| default_vertical_axis_unit(vertical_domain)),
                vertical_start,
                vertical_step,
            },
        )
    }

    pub fn describe_velocity_volume(
        &self,
        request: DescribeVelocityVolumeRequest,
    ) -> Result<DescribeVelocityVolumeResponse, Box<dyn std::error::Error>> {
        describe_velocity_volume(request)
    }

    pub fn ingest_velocity_volume(
        &self,
        input_path: String,
        output_store_path: String,
        velocity_kind: VelocityQuantityKind,
        vertical_domain: TimeDepthDomain,
        vertical_unit: Option<String>,
        vertical_start: Option<f32>,
        vertical_step: Option<f32>,
        overwrite_existing: bool,
        delete_input_on_success: bool,
    ) -> Result<IngestVelocityVolumeResponse, Box<dyn std::error::Error>> {
        ingest_velocity_volume_with_options(
            input_path,
            output_store_path,
            velocity_kind,
            vertical_domain,
            vertical_unit,
            vertical_start,
            vertical_step,
            overwrite_existing,
            delete_input_on_success,
            None,
        )
    }

    pub fn ingest_velocity_volume_request(
        &self,
        request: IngestVelocityVolumeRequest,
    ) -> Result<IngestVelocityVolumeResponse, Box<dyn std::error::Error>> {
        ingest_velocity_volume(request)
    }

    pub fn ensure_demo_survey_time_depth_transform(
        &self,
        store_path: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        ensure_demo_survey_time_depth_transform(store_path)
    }

    pub fn build_paired_horizon_transform(
        &self,
        store_path: String,
        time_horizon_ids: Vec<String>,
        depth_horizon_ids: Vec<String>,
        output_id: Option<String>,
        output_name: Option<String>,
    ) -> Result<SurveyTimeDepthTransform3D, Box<dyn std::error::Error>> {
        build_paired_horizon_transform(
            store_path,
            time_horizon_ids,
            depth_horizon_ids,
            output_id,
            output_name,
        )
    }

    pub fn convert_horizon_domain(
        &self,
        store_path: String,
        source_horizon_id: String,
        transform_id: String,
        target_domain: TimeDepthDomain,
        output_id: Option<String>,
        output_name: Option<String>,
    ) -> Result<ImportedHorizonDescriptor, Box<dyn std::error::Error>> {
        convert_horizon_domain(
            store_path,
            source_horizon_id,
            transform_id,
            target_domain,
            output_id,
            output_name,
        )
    }

    pub fn import_velocity_functions_model(
        &self,
        store_path: String,
        input_path: String,
        velocity_kind: VelocityQuantityKind,
    ) -> Result<ImportVelocityFunctionsModelResponse, Box<dyn std::error::Error>> {
        import_velocity_functions_model(store_path, input_path, velocity_kind)
    }

    pub fn prepare_survey_demo(
        &self,
        request: PrepareSurveyDemoRequest,
    ) -> Result<PrepareSurveyDemoResponse, Box<dyn std::error::Error>> {
        let ensured_time_depth_transform_id =
            self.ensure_demo_survey_time_depth_transform(request.store_path.clone())?;
        let velocity_models = self.load_velocity_models(request.store_path.clone())?;
        let survey_map = self.resolve_survey_map(ResolveSurveyMapRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: request.store_path.clone(),
            display_coordinate_reference_id: request.display_coordinate_reference_id,
        })?;

        Ok(PrepareSurveyDemoResponse {
            store_path: request.store_path,
            ensured_time_depth_transform_id,
            velocity_models,
            survey_map,
        })
    }
}

fn materialize_options_for_store(
    input_store_path: &str,
) -> Result<MaterializeOptions, Box<dyn std::error::Error>> {
    let chunk_shape = open_store(input_store_path)?.manifest.tile_shape;
    Ok(MaterializeOptions {
        chunk_shape,
        ..MaterializeOptions::default()
    })
}

#[cfg(target_os = "windows")]
fn available_system_memory_bytes() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status = unsafe { std::mem::zeroed::<MEMORYSTATUSEX>() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        None
    } else {
        Some(status.ullAvailPhys)
    }
}

#[cfg(all(unix, target_vendor = "apple"))]
fn available_system_memory_bytes() -> Option<u64> {
    fn sysctl_u64(name: &str) -> Option<u64> {
        let c_name = CString::new(name).ok()?;
        let mut value: u64 = 0;
        let mut size = std::mem::size_of::<u64>();
        let rc = unsafe {
            libc::sysctlbyname(
                c_name.as_ptr(),
                &mut value as *mut u64 as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if rc == 0 { Some(value) } else { None }
    }

    let page_size = sysctl_u64("hw.pagesize")?;
    let free_pages = sysctl_u64("vm.page_free_count")?;
    Some(page_size.saturating_mul(free_pages))
}

#[cfg(all(unix, not(target_vendor = "apple")))]
fn available_system_memory_bytes() -> Option<u64> {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let available_pages = unsafe { libc::sysconf(libc::_SC_AVPHYS_PAGES) };
    if page_size <= 0 || available_pages <= 0 {
        None
    } else {
        Some((page_size as u64).saturating_mul(available_pages as u64))
    }
}

#[cfg(not(any(target_os = "windows", unix)))]
fn available_system_memory_bytes() -> Option<u64> {
    None
}

fn benchmark_worker_count() -> usize {
    std::env::var("OPHIOLITE_BENCHMARK_WORKERS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(4)
        })
        .max(1)
}

#[derive(Debug, Clone, Default)]
struct BatchBenchmarkJobMetrics {
    queue_wait_ms: Option<f64>,
    elapsed_ms: Option<f64>,
    total_tiles: usize,
    completed_tiles: usize,
    total_partitions: usize,
    completed_partitions: usize,
    peak_active_partitions: usize,
    retry_count: usize,
    output_bytes: u64,
    error_message: Option<String>,
}

pub fn benchmark_trace_local_processing(
    request: TraceLocalBenchmarkRequest,
) -> Result<TraceLocalBenchmarkResponse, Box<dyn std::error::Error>> {
    let handle = open_store(&request.store_path)?;
    let source_shape = handle.manifest.volume.shape;
    let source_chunk_shape = handle.manifest.tile_shape;
    let pipeline = benchmark_pipeline(request.scenario);
    let pipeline_spec = ProcessingPipelineSpec::TraceLocal {
        pipeline: pipeline.clone(),
    };
    let representative_plan = build_execution_plan(&PlanProcessingRequest {
        store_path: request.store_path.clone(),
        layout: SeismicLayout::PostStack3D,
        source_shape: Some(source_shape),
        source_chunk_shape: Some(source_chunk_shape),
        pipeline: pipeline_spec.clone(),
        output_store_path: None,
        planning_mode: PlanningMode::ForegroundMaterialize,
        max_active_partitions: None,
    })
    .map_err(|error| format!("failed to build execution plan: {error}"))?;
    let output_root = request
        .output_root
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ophiolite-trace-local-benchmarks"));
    fs::create_dir_all(&output_root)?;

    let adaptive_options_resolution = resolve_trace_local_materialize_options(
        Some(&representative_plan),
        source_chunk_shape,
        request.adaptive_partition_target,
        None,
        benchmark_worker_count(),
        available_system_memory_bytes(),
        1,
    );
    let adaptive_recommendation = adaptive_options_resolution
        .chunk_plan_resolution
        .as_ref()
        .map(|recommendation| chunk_plan_benchmark_recommendation(recommendation.clone()));
    let variants = benchmark_variant_targets(&request, adaptive_recommendation.as_ref());
    let repeat_count = request.repeat_count.max(1);
    let mut runs = Vec::new();

    for partition_target_bytes in variants {
        let label =
            benchmark_variant_label(partition_target_bytes, request.adaptive_partition_target);
        for repeat_index in 0..repeat_count {
            let output_store_path = output_root.join(format!(
                "{}-{}-run-{:02}.tbvol",
                dataset_slug(&request.store_path),
                benchmark_scenario_slug(request.scenario),
                benchmark_variant_output_slug(
                    partition_target_bytes,
                    request.adaptive_partition_target,
                    repeat_index,
                ),
            ));
            prepare_processing_output_store(&output_store_path, true)?;

            let mut completed_tiles = 0usize;
            let mut total_tiles = 0usize;
            let mut partition_progress = PartitionExecutionProgress {
                completed_partitions: 0,
                total_partitions: 0,
                active_partitions: 0,
                peak_active_partitions: 0,
                retry_count: 0,
            };
            let materialize_options = resolve_trace_local_materialize_options(
                Some(&representative_plan),
                source_chunk_shape,
                request.adaptive_partition_target && partition_target_bytes.is_some(),
                partition_target_bytes,
                benchmark_worker_count(),
                available_system_memory_bytes(),
                1,
            )
            .options;
            let started = Instant::now();
            let _derived = materialize_processing_volume_with_partition_progress(
                &request.store_path,
                &output_store_path,
                &pipeline,
                materialize_options,
                |completed, total| {
                    completed_tiles = completed;
                    total_tiles = total;
                    Ok(())
                },
                |progress| {
                    partition_progress = progress;
                    Ok(())
                },
            )?;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            let output_bytes = path_size_bytes(&output_store_path)?;
            let output_store_path_string = output_store_path.to_string_lossy().into_owned();
            let output_retained = request.keep_outputs;
            if !request.keep_outputs {
                remove_store_path_if_exists(&output_store_path)?;
            }

            runs.push(TraceLocalBenchmarkRunResult {
                label: label.clone(),
                scenario: request.scenario,
                partition_target_bytes,
                repeat_index: repeat_index + 1,
                elapsed_ms,
                total_tiles,
                completed_tiles,
                total_partitions: partition_progress.total_partitions,
                completed_partitions: partition_progress.completed_partitions,
                peak_active_partitions: partition_progress.peak_active_partitions,
                retry_count: partition_progress.retry_count,
                output_bytes,
                output_store_path: output_store_path_string,
                output_retained,
            });
        }
    }

    Ok(TraceLocalBenchmarkResponse {
        store_path: request.store_path,
        scenario: request.scenario,
        source_shape,
        source_chunk_shape,
        adaptive_partition_target: request.adaptive_partition_target,
        adaptive_recommendation,
        pipeline,
        plan_summary: representative_plan.plan_summary.clone(),
        stage_classifications: benchmark_stage_classifications(&representative_plan),
        variants: summarize_benchmark_variants(&runs),
        runs,
    })
}

pub fn benchmark_trace_local_batch_processing(
    request: TraceLocalBatchBenchmarkRequest,
) -> Result<TraceLocalBatchBenchmarkResponse, Box<dyn std::error::Error>> {
    let dataset = dataset_summary_for_path(&request.store_path)?;
    let trace_descriptor = SeismicTraceDataDescriptor::from(&dataset.descriptor);
    let source_shape = dataset.descriptor.shape;
    let source_chunk_shape = dataset.descriptor.chunk_shape;
    let pipeline = benchmark_pipeline(request.scenario);
    let pipeline_spec = ProcessingPipelineSpec::TraceLocal {
        pipeline: pipeline.clone(),
    };
    let output_root = request
        .output_root
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ophiolite-trace-local-batch-benchmarks"));
    fs::create_dir_all(&output_root)?;
    let service_workers = benchmark_worker_count();
    let repeat_count = request.repeat_count.max(1);
    let job_count = request.job_count.max(1);
    let representative_plan = build_execution_plan(&PlanProcessingRequest {
        store_path: request.store_path.clone(),
        layout: trace_descriptor.layout,
        source_shape: Some(source_shape),
        source_chunk_shape: Some(source_chunk_shape),
        pipeline: pipeline_spec.clone(),
        output_store_path: None,
        planning_mode: PlanningMode::BackgroundBatch,
        max_active_partitions: None,
    })
    .map_err(|error| format!("failed to build execution plan: {error}"))?;
    let mut response_adaptive_recommendation = None;
    let mut response_partition_target_bytes = request.partition_target_mib.max(1) * 1024 * 1024;
    let mut variants = Vec::new();

    let requested_levels = if request.execution_mode.is_some() && request.max_active_jobs.is_empty()
    {
        vec![None]
    } else if request.max_active_jobs.is_empty() {
        vec![Some(1), Some(2), Some(4)]
    } else {
        request
            .max_active_jobs
            .iter()
            .copied()
            .map(|value| value.max(1))
            .map(Some)
            .collect::<Vec<_>>()
    };

    for requested_max_active_jobs in requested_levels {
        for repeat_index in 0..repeat_count {
            let service = ProcessingExecutionService::new(service_workers);
            let batch_policy = service.resolve_batch_execution_policy(
                requested_max_active_jobs,
                request.execution_mode,
                &pipeline_spec,
                Some(&representative_plan),
                ExecutionPriorityClass::BackgroundBatch,
            );
            let materialize_options_resolution = resolve_trace_local_materialize_options(
                Some(&representative_plan),
                source_chunk_shape,
                request.adaptive_partition_target,
                Some(request.partition_target_mib.max(1) * 1024 * 1024),
                service_workers,
                available_system_memory_bytes(),
                batch_policy.effective_max_active_jobs,
            );
            let adaptive_recommendation = materialize_options_resolution
                .chunk_plan_resolution
                .as_ref()
                .map(|recommendation| chunk_plan_benchmark_recommendation(recommendation.clone()));
            let partition_target_bytes = materialize_options_resolution
                .resolved_partition_target_bytes
                .unwrap_or(request.partition_target_mib.max(1) * 1024 * 1024);
            if response_adaptive_recommendation.is_none() {
                response_adaptive_recommendation = adaptive_recommendation.clone();
                response_partition_target_bytes = partition_target_bytes;
            }
            let batch_gate = service.create_batch_gate(batch_policy.effective_max_active_jobs);
            let batch_started = Instant::now();
            let materialize_options = materialize_options_resolution.options.clone();
            let resolved_chunk_plan_summary =
                materialize_options_resolution.resolved_chunk_plan.clone();
            let shared_metrics = Arc::new(Mutex::new(
                HashMap::<String, BatchBenchmarkJobMetrics>::new(),
            ));
            let mut items = Vec::with_capacity(job_count);
            let mut job_labels_by_id = HashMap::with_capacity(job_count);
            let mut job_ids = Vec::with_capacity(job_count);

            for job_index in 0..job_count {
                let job_label = format!("job-{:02}", job_index + 1);
                let output_store_path = output_root.join(format!(
                    "{}-{}-batch-{}-run-{:02}-{}.tbvol",
                    dataset_slug(&request.store_path),
                    benchmark_scenario_slug(request.scenario),
                    benchmark_batch_requested_slug(
                        requested_max_active_jobs,
                        batch_policy.execution_mode,
                        request.adaptive_partition_target,
                    ),
                    repeat_index + 1,
                    job_label,
                ));
                prepare_processing_output_store(&output_store_path, true)?;
                let output_store_path_string = output_store_path.to_string_lossy().into_owned();
                let item = ProcessingBatchItemRequest {
                    store_path: request.store_path.clone(),
                    output_store_path: Some(output_store_path_string.clone()),
                };
                let queue_started = Instant::now();
                let metrics = Arc::clone(&shared_metrics);
                let input_store_path = request.store_path.clone();
                let benchmark_pipeline = pipeline.clone();
                let output_store_path_for_task = output_store_path.clone();
                let job_label_for_task = job_label.clone();
                let keep_outputs = request.keep_outputs;
                let materialize_options_for_task = materialize_options.clone();
                let resolved_chunk_plan_summary_for_task = resolved_chunk_plan_summary.clone();
                let record_plan = build_execution_plan(&PlanProcessingRequest {
                    store_path: request.store_path.clone(),
                    layout: trace_descriptor.layout,
                    source_shape: Some(source_shape),
                    source_chunk_shape: Some(source_chunk_shape),
                    pipeline: pipeline_spec.clone(),
                    output_store_path: Some(output_store_path_string.clone()),
                    planning_mode: PlanningMode::BackgroundBatch,
                    max_active_partitions: None,
                })
                .map_err(|error| format!("failed to build execution plan: {error}"))?;
                let status = service.enqueue_job(
                    request.store_path.clone(),
                    Some(output_store_path_string.clone()),
                    pipeline_spec.clone(),
                    Some(record_plan),
                    ExecutionPriorityClass::BackgroundBatch,
                    Some(batch_gate.clone()),
                    move |record| {
                        let started = Instant::now();
                        record.mark_running(Some("Trace-local batch benchmark".to_string()));
                        let queue_wait_ms =
                            started.duration_since(queue_started).as_secs_f64() * 1000.0;
                        {
                            let mut all_metrics =
                                metrics.lock().expect("benchmark metrics mutex poisoned");
                            all_metrics
                                .entry(job_label_for_task.clone())
                                .or_default()
                                .queue_wait_ms = Some(queue_wait_ms);
                        }
                        let mut completed_tiles = 0usize;
                        let mut total_tiles = 0usize;
                        let mut partition_progress = PartitionExecutionProgress {
                            completed_partitions: 0,
                            total_partitions: 0,
                            active_partitions: 0,
                            peak_active_partitions: 0,
                            retry_count: 0,
                        };
                        let result = materialize_processing_volume_with_partition_progress(
                            &input_store_path,
                            &output_store_path_for_task,
                            &benchmark_pipeline,
                            materialize_options_for_task,
                            |completed, total| {
                                completed_tiles = completed;
                                total_tiles = total;
                                let _ = record.mark_progress(
                                    completed,
                                    total,
                                    Some("Trace-local batch benchmark"),
                                );
                                Ok(())
                            },
                            |progress| {
                                partition_progress = progress;
                                let _ =
                                    record.set_execution_summary(processing_job_execution_summary(
                                        progress,
                                        resolved_chunk_plan_summary_for_task.clone(),
                                    ));
                                Ok(())
                            },
                        );
                        match result {
                            Ok(_) => {
                                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                                let output_bytes = path_size_bytes(&output_store_path_for_task)
                                    .unwrap_or_default();
                                {
                                    let mut all_metrics =
                                        metrics.lock().expect("benchmark metrics mutex poisoned");
                                    all_metrics.insert(
                                        job_label_for_task.clone(),
                                        BatchBenchmarkJobMetrics {
                                            queue_wait_ms: Some(queue_wait_ms),
                                            elapsed_ms: Some(elapsed_ms),
                                            total_tiles,
                                            completed_tiles,
                                            total_partitions: partition_progress.total_partitions,
                                            completed_partitions: partition_progress
                                                .completed_partitions,
                                            peak_active_partitions: partition_progress
                                                .peak_active_partitions,
                                            retry_count: partition_progress.retry_count,
                                            output_bytes,
                                            error_message: None,
                                        },
                                    );
                                }
                                if !keep_outputs {
                                    let _ =
                                        remove_store_path_if_exists(&output_store_path_for_task);
                                }
                                let _ = record.mark_completed(
                                    output_store_path_for_task.to_string_lossy().into_owned(),
                                );
                            }
                            Err(error) => {
                                let error_message = error.to_string();
                                {
                                    let mut all_metrics =
                                        metrics.lock().expect("benchmark metrics mutex poisoned");
                                    all_metrics.insert(
                                        job_label_for_task.clone(),
                                        BatchBenchmarkJobMetrics {
                                            queue_wait_ms: Some(queue_wait_ms),
                                            elapsed_ms: Some(
                                                started.elapsed().as_secs_f64() * 1000.0,
                                            ),
                                            total_tiles,
                                            completed_tiles,
                                            total_partitions: partition_progress.total_partitions,
                                            completed_partitions: partition_progress
                                                .completed_partitions,
                                            peak_active_partitions: partition_progress
                                                .peak_active_partitions,
                                            retry_count: partition_progress.retry_count,
                                            output_bytes: 0,
                                            error_message: Some(error_message.clone()),
                                        },
                                    );
                                }
                                let _ = remove_store_path_if_exists(&output_store_path_for_task);
                                let _ = record.mark_failed(error_message);
                            }
                        }
                    },
                );
                items.push(item);
                job_labels_by_id.insert(status.job_id.clone(), job_label);
                job_ids.push(status.job_id);
            }

            let batch = service
                .register_batch(pipeline_spec.clone(), items, job_ids.clone(), &batch_policy)
                .map_err(|error| format!("failed to register benchmark batch: {error}"))?;

            loop {
                let status = service
                    .batch_status(&batch.batch_id)
                    .map_err(|error| format!("failed to poll benchmark batch: {error}"))?;
                if matches!(
                    status.state,
                    ProcessingBatchState::Completed
                        | ProcessingBatchState::CompletedWithErrors
                        | ProcessingBatchState::Cancelled
                ) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let batch_elapsed_ms = batch_started.elapsed().as_secs_f64() * 1000.0;
            let final_batch = service
                .batch_status(&batch.batch_id)
                .map_err(|error| format!("failed to fetch final benchmark batch: {error}"))?;
            let all_metrics = shared_metrics
                .lock()
                .expect("benchmark metrics mutex poisoned");
            let mut jobs = Vec::with_capacity(final_batch.items.len());
            for item in &final_batch.items {
                let job_label = job_labels_by_id
                    .get(&item.job_id)
                    .cloned()
                    .unwrap_or_else(|| item.job_id.clone());
                let metrics = all_metrics.get(&job_label).cloned().unwrap_or_default();
                jobs.push(TraceLocalBatchBenchmarkJobResult {
                    job_label,
                    job_id: item.job_id.clone(),
                    state: match item.state {
                        ProcessingJobState::Queued => "queued".to_string(),
                        ProcessingJobState::Running => "running".to_string(),
                        ProcessingJobState::Completed => "completed".to_string(),
                        ProcessingJobState::Failed => "failed".to_string(),
                        ProcessingJobState::Cancelled => "cancelled".to_string(),
                    },
                    queue_wait_ms: metrics.queue_wait_ms.unwrap_or(0.0),
                    elapsed_ms: metrics.elapsed_ms.unwrap_or(0.0),
                    total_tiles: metrics.total_tiles,
                    completed_tiles: metrics.completed_tiles,
                    total_partitions: metrics.total_partitions,
                    completed_partitions: metrics.completed_partitions,
                    peak_active_partitions: metrics.peak_active_partitions,
                    retry_count: metrics.retry_count,
                    output_bytes: metrics.output_bytes,
                    output_store_path: item.output_store_path.clone().unwrap_or_default(),
                    output_retained: request.keep_outputs,
                    error_message: metrics.error_message.or_else(|| item.error_message.clone()),
                });
            }
            jobs.sort_by(|left, right| left.job_label.cmp(&right.job_label));
            let avg_queue_wait_ms = average_f64(jobs.iter().map(|job| job.queue_wait_ms));
            let max_queue_wait_ms = jobs
                .iter()
                .map(|job| job.queue_wait_ms)
                .fold(0.0_f64, f64::max);
            let avg_job_elapsed_ms = average_f64(jobs.iter().map(|job| job.elapsed_ms));
            let min_job_elapsed_ms = jobs
                .iter()
                .map(|job| job.elapsed_ms)
                .fold(f64::INFINITY, f64::min);
            let max_job_elapsed_ms = jobs
                .iter()
                .map(|job| job.elapsed_ms)
                .fold(f64::NEG_INFINITY, f64::max);
            let avg_total_partitions =
                average_f64(jobs.iter().map(|job| job.total_partitions as f64));
            let avg_peak_active_partitions =
                average_f64(jobs.iter().map(|job| job.peak_active_partitions as f64));
            variants.push(TraceLocalBatchBenchmarkVariantResult {
                label: format!(
                    "{}-requested-{}-effective-{}",
                    match batch_policy.execution_mode {
                        ProcessingExecutionMode::Auto => "auto",
                        ProcessingExecutionMode::Conservative => "conservative",
                        ProcessingExecutionMode::Throughput => "throughput",
                        ProcessingExecutionMode::Custom => "custom",
                    },
                    requested_max_active_jobs
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "auto".to_string()),
                    batch_policy.effective_max_active_jobs
                ),
                requested_max_active_jobs,
                effective_max_active_jobs: batch_policy.effective_max_active_jobs,
                execution_mode: batch_policy.execution_mode,
                scheduler_reason: batch_policy.scheduler_reason,
                worker_budget: batch_policy.worker_budget,
                global_cap: batch_policy.global_cap,
                max_memory_cost_class: batch_policy.max_memory_cost_class,
                max_cpu_cost_class: representative_plan.plan_summary.max_cpu_cost_class,
                max_io_cost_class: representative_plan.plan_summary.max_io_cost_class,
                min_parallel_efficiency_class: representative_plan
                    .plan_summary
                    .min_parallel_efficiency_class,
                max_estimated_peak_memory_bytes: batch_policy.max_estimated_peak_memory_bytes,
                combined_cpu_weight: representative_plan.plan_summary.combined_cpu_weight,
                combined_io_weight: representative_plan.plan_summary.combined_io_weight,
                max_expected_partition_count: batch_policy.max_expected_partition_count,
                partition_target_bytes,
                adaptive_recommendation: adaptive_recommendation.clone(),
                repeat_index: repeat_index + 1,
                batch_elapsed_ms,
                completed_jobs: final_batch.progress.completed_jobs,
                total_jobs: final_batch.progress.total_jobs,
                avg_queue_wait_ms,
                max_queue_wait_ms,
                avg_job_elapsed_ms,
                min_job_elapsed_ms,
                max_job_elapsed_ms,
                avg_total_partitions,
                avg_peak_active_partitions,
                jobs,
            });
        }
    }

    Ok(TraceLocalBatchBenchmarkResponse {
        store_path: request.store_path,
        scenario: request.scenario,
        source_shape,
        source_chunk_shape,
        job_count,
        partition_target_bytes: response_partition_target_bytes,
        adaptive_partition_target: request.adaptive_partition_target,
        adaptive_recommendation: response_adaptive_recommendation,
        pipeline,
        plan_summary: representative_plan.plan_summary.clone(),
        stage_classifications: benchmark_stage_classifications(&representative_plan),
        summaries: summarize_batch_benchmark_variants(&variants),
        variants,
    })
}

pub fn benchmark_post_stack_neighborhood_preview(
    request: PostStackNeighborhoodPreviewBenchmarkRequest,
) -> Result<PostStackNeighborhoodPreviewBenchmarkResponse, Box<dyn std::error::Error>> {
    let dataset = dataset_summary_for_path(&request.store_path)?;
    let source_shape = dataset.descriptor.shape;
    let source_chunk_shape = dataset.descriptor.chunk_shape;
    let pipeline = post_stack_neighborhood_benchmark_pipeline(
        request.operator,
        request.gate_ms,
        request.inline_stepout,
        request.xline_stepout,
        request.include_trace_local_prefix,
    );
    let plan = build_execution_plan(&PlanProcessingRequest {
        store_path: request.store_path.clone(),
        layout: SeismicLayout::PostStack3D,
        source_shape: Some(source_shape),
        source_chunk_shape: Some(source_chunk_shape),
        pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
            pipeline: pipeline.clone(),
        },
        output_store_path: None,
        planning_mode: PlanningMode::InteractivePreview,
        max_active_partitions: None,
    })
    .map_err(|error| format!("failed to build execution plan: {error}"))?;
    let repeat_count = request.repeat_count.max(1);
    let label = post_stack_neighborhood_benchmark_label(
        request.operator,
        request.gate_ms,
        request.inline_stepout,
        request.xline_stepout,
        request.include_trace_local_prefix,
    );
    let dataset_id = dataset.descriptor.id.clone();
    let mut runs = Vec::with_capacity(repeat_count);

    for repeat_index in 0..repeat_count {
        let started = Instant::now();
        let response = preview_post_stack_neighborhood_processing(
            PreviewPostStackNeighborhoodProcessingRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: request.store_path.clone(),
                section: seis_runtime::SectionRequest {
                    dataset_id: dataset_id.clone(),
                    axis: request.section_axis,
                    index: request.section_index,
                },
                pipeline: pipeline.clone(),
            },
        )?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        runs.push(PostStackNeighborhoodPreviewBenchmarkRunResult {
            label: label.clone(),
            operator: request.operator,
            repeat_index: repeat_index + 1,
            elapsed_ms,
            time_to_first_result_ms: elapsed_ms,
            section_axis: request.section_axis,
            section_index: request.section_index,
            trace_local_prefix_applied: request.include_trace_local_prefix,
            preview_ready: response.preview.preview_ready,
            traces: response.preview.section.traces,
            samples: response.preview.section.samples,
            amplitude_bytes: response.preview.section.amplitudes_f32le.len(),
        });
    }

    Ok(PostStackNeighborhoodPreviewBenchmarkResponse {
        store_path: request.store_path,
        operator: request.operator,
        window: PostStackNeighborhoodBenchmarkWindow {
            gate_ms: request.gate_ms,
            inline_stepout: request.inline_stepout,
            xline_stepout: request.xline_stepout,
        },
        source_shape,
        source_chunk_shape,
        section_axis: request.section_axis,
        section_index: request.section_index,
        include_trace_local_prefix: request.include_trace_local_prefix,
        pipeline,
        plan_summary: plan.plan_summary.clone(),
        stage_classifications: benchmark_stage_classifications(&plan),
        summaries: summarize_post_stack_neighborhood_preview_runs(&runs),
        runs,
    })
}

pub fn benchmark_post_stack_neighborhood_processing(
    request: PostStackNeighborhoodProcessingBenchmarkRequest,
) -> Result<PostStackNeighborhoodProcessingBenchmarkResponse, Box<dyn std::error::Error>> {
    let dataset = dataset_summary_for_path(&request.store_path)?;
    let source_shape = dataset.descriptor.shape;
    let source_chunk_shape = dataset.descriptor.chunk_shape;
    let pipeline = post_stack_neighborhood_benchmark_pipeline(
        request.operator,
        request.gate_ms,
        request.inline_stepout,
        request.xline_stepout,
        request.include_trace_local_prefix,
    );
    let plan = build_execution_plan(&PlanProcessingRequest {
        store_path: request.store_path.clone(),
        layout: SeismicLayout::PostStack3D,
        source_shape: Some(source_shape),
        source_chunk_shape: Some(source_chunk_shape),
        pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
            pipeline: pipeline.clone(),
        },
        output_store_path: None,
        planning_mode: PlanningMode::ForegroundMaterialize,
        max_active_partitions: None,
    })
    .map_err(|error| format!("failed to build execution plan: {error}"))?;
    let output_root = request
        .output_root
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join("ophiolite-post-stack-neighborhood-benchmarks")
        });
    fs::create_dir_all(&output_root)?;
    let repeat_count = request.repeat_count.max(1);
    let label = post_stack_neighborhood_benchmark_label(
        request.operator,
        request.gate_ms,
        request.inline_stepout,
        request.xline_stepout,
        request.include_trace_local_prefix,
    );
    let mut runs = Vec::with_capacity(repeat_count);

    for repeat_index in 0..repeat_count {
        let output_store_path = output_root.join(format!(
            "{}-{}-run-{:02}.tbvol",
            dataset_slug(&request.store_path),
            post_stack_neighborhood_benchmark_slug(
                request.operator,
                request.gate_ms,
                request.inline_stepout,
                request.xline_stepout,
                request.include_trace_local_prefix,
            ),
            repeat_index + 1
        ));
        prepare_processing_output_store(&output_store_path, true)?;
        let started = Instant::now();
        let _derived = materialize_post_stack_neighborhood_processing_volume(
            &request.store_path,
            &output_store_path,
            &pipeline,
            materialize_options_for_store(&request.store_path)?,
        )?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let output_bytes = path_size_bytes(&output_store_path)?;
        let output_store_path_string = output_store_path.to_string_lossy().into_owned();
        let output_retained = request.keep_outputs;
        if !request.keep_outputs {
            remove_store_path_if_exists(&output_store_path)?;
        }
        runs.push(PostStackNeighborhoodProcessingBenchmarkRunResult {
            label: label.clone(),
            operator: request.operator,
            repeat_index: repeat_index + 1,
            elapsed_ms,
            trace_local_prefix_applied: request.include_trace_local_prefix,
            output_bytes,
            output_store_path: output_store_path_string,
            output_retained,
        });
    }

    Ok(PostStackNeighborhoodProcessingBenchmarkResponse {
        store_path: request.store_path,
        operator: request.operator,
        window: PostStackNeighborhoodBenchmarkWindow {
            gate_ms: request.gate_ms,
            inline_stepout: request.inline_stepout,
            xline_stepout: request.xline_stepout,
        },
        source_shape,
        source_chunk_shape,
        include_trace_local_prefix: request.include_trace_local_prefix,
        pipeline,
        plan_summary: plan.plan_summary.clone(),
        stage_classifications: benchmark_stage_classifications(&plan),
        summaries: summarize_post_stack_neighborhood_processing_runs(&runs),
        runs,
    })
}

fn chunk_plan_benchmark_recommendation(
    recommendation: seis_runtime::TraceLocalChunkPlanResolution,
) -> TraceLocalChunkPlanBenchmarkRecommendation {
    TraceLocalChunkPlanBenchmarkRecommendation {
        target_bytes: recommendation.target_bytes(),
        bytes_per_tile: recommendation.bytes_per_tile,
        total_tiles: recommendation.total_tiles,
        preferred_partition_count: recommendation.preferred_partition_count,
        recommended_partition_count: recommendation.recommended_partition_count(),
        recommended_max_active_partitions: recommendation.recommended_max_active_partitions(),
        tiles_per_partition: recommendation.tiles_per_partition(),
        resident_partition_bytes: recommendation.resident_partition_bytes(),
        global_worker_workspace_bytes: recommendation.global_worker_workspace_bytes(),
        estimated_peak_bytes: recommendation.estimated_peak_bytes(),
        available_memory_bytes: recommendation.available_memory_bytes,
        reserved_memory_bytes: recommendation.reserved_memory_bytes,
        usable_memory_bytes: recommendation.usable_memory_bytes,
    }
}

fn processing_job_execution_summary(
    progress: PartitionExecutionProgress,
    resolved_chunk_plan: Option<ProcessingJobChunkPlanSummary>,
) -> ProcessingJobExecutionSummary {
    ProcessingJobExecutionSummary {
        completed_partitions: progress.completed_partitions,
        total_partitions: Some(progress.total_partitions),
        active_partitions: progress.active_partitions,
        peak_active_partitions: progress.peak_active_partitions,
        retry_count: progress.retry_count,
        resolved_chunk_plan,
        stages: Vec::new(),
    }
}

fn benchmark_stage_classifications(
    plan: &seis_runtime::ExecutionPlan,
) -> Vec<BenchmarkStageClassificationSummary> {
    plan.stages
        .iter()
        .map(|stage| BenchmarkStageClassificationSummary {
            stage_id: stage.stage_id.clone(),
            stage_label: benchmark_stage_label(stage),
            stage_kind: stage.stage_kind,
            partition_family: stage.partition_spec.family,
            expected_partition_count: stage.expected_partition_count,
            classification: stage.classification.clone(),
        })
        .collect()
}

fn benchmark_stage_label(stage: &seis_runtime::ExecutionStage) -> String {
    let action = match stage.stage_kind {
        seis_runtime::ExecutionStageKind::Compute => "Compute",
        seis_runtime::ExecutionStageKind::Checkpoint => "Checkpoint",
        seis_runtime::ExecutionStageKind::ReuseArtifact => "Reuse Artifact",
        seis_runtime::ExecutionStageKind::FinalizeOutput => "Finalize Output",
    };
    match stage.pipeline_segment.as_ref() {
        Some(segment) => {
            let family = match segment.family {
                seis_runtime::ProcessingPipelineFamily::TraceLocal => "trace-local",
                seis_runtime::ProcessingPipelineFamily::PostStackNeighborhood => {
                    "post-stack neighborhood"
                }
                seis_runtime::ProcessingPipelineFamily::Subvolume => "subvolume",
                seis_runtime::ProcessingPipelineFamily::Gather => "gather",
            };
            let steps = if segment.start_step_index == segment.end_step_index {
                format!("step {}", segment.end_step_index + 1)
            } else {
                format!(
                    "steps {}-{}",
                    segment.start_step_index + 1,
                    segment.end_step_index + 1
                )
            };
            format!("{action}: {family} {steps}")
        }
        None => format!("{action}: {}", stage.output_artifact_id),
    }
}

fn benchmark_variant_targets(
    request: &TraceLocalBenchmarkRequest,
    adaptive_recommendation: Option<&TraceLocalChunkPlanBenchmarkRecommendation>,
) -> Vec<Option<u64>> {
    let mut variants = Vec::new();
    if request.include_serial {
        variants.push(None);
    }
    if let Some(recommendation) = adaptive_recommendation {
        variants.push(Some(recommendation.target_bytes));
        return variants;
    }
    let configured_partition_targets = if request.partition_target_mib.is_empty() {
        vec![256]
    } else {
        request.partition_target_mib.clone()
    };
    for target_mib in configured_partition_targets {
        let target_mib = target_mib.max(1);
        let target_bytes = target_mib.saturating_mul(1024 * 1024);
        if !variants.contains(&Some(target_bytes)) {
            variants.push(Some(target_bytes));
        }
    }
    if variants.is_empty() {
        variants.push(Some(256 * 1024 * 1024));
    }
    variants
}

fn benchmark_variant_label(
    partition_target_bytes: Option<u64>,
    adaptive_partition_target: bool,
) -> String {
    match partition_target_bytes {
        Some(bytes) if adaptive_partition_target => format!("adaptive-{}mib", bytes / 1024 / 1024),
        Some(bytes) => format!("partitioned-{}mib", bytes / 1024 / 1024),
        None => "serial".to_string(),
    }
}

fn benchmark_variant_output_slug(
    partition_target_bytes: Option<u64>,
    adaptive_partition_target: bool,
    repeat_index: usize,
) -> String {
    match partition_target_bytes {
        Some(bytes) if adaptive_partition_target => format!(
            "adaptive-{}mib-run-{:02}",
            bytes / 1024 / 1024,
            repeat_index + 1
        ),
        Some(bytes) => format!(
            "partitioned-{}mib-run-{:02}",
            bytes / 1024 / 1024,
            repeat_index + 1
        ),
        None => format!("serial-run-{:02}", repeat_index + 1),
    }
}

fn benchmark_batch_requested_slug(
    requested_max_active_jobs: Option<usize>,
    execution_mode: ProcessingExecutionMode,
    adaptive_partition_target: bool,
) -> String {
    let base = requested_max_active_jobs
        .map(|value| value.to_string())
        .unwrap_or_else(|| match execution_mode {
            ProcessingExecutionMode::Auto => "auto".to_string(),
            ProcessingExecutionMode::Conservative => "conservative".to_string(),
            ProcessingExecutionMode::Throughput => "throughput".to_string(),
            ProcessingExecutionMode::Custom => "custom".to_string(),
        });
    if adaptive_partition_target {
        format!("adaptive-{base}")
    } else {
        base
    }
}

fn benchmark_scenario_slug(scenario: TraceLocalBenchmarkScenario) -> &'static str {
    match scenario {
        TraceLocalBenchmarkScenario::Scalar => "scalar",
        TraceLocalBenchmarkScenario::Agc => "agc",
        TraceLocalBenchmarkScenario::Analytic => "analytic",
        TraceLocalBenchmarkScenario::Bandpass => "bandpass",
    }
}

fn benchmark_pipeline(scenario: TraceLocalBenchmarkScenario) -> TraceLocalProcessingPipeline {
    let steps = match scenario {
        TraceLocalBenchmarkScenario::Scalar => vec![TraceLocalProcessingStep {
            operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 1.25 },
            checkpoint: false,
        }],
        TraceLocalBenchmarkScenario::Agc => vec![
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                checkpoint: false,
            },
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::AgcRms { window_ms: 250.0 },
                checkpoint: false,
            },
        ],
        TraceLocalBenchmarkScenario::Analytic => vec![
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                checkpoint: false,
            },
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::Envelope,
                checkpoint: false,
            },
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::InstantaneousPhase,
                checkpoint: false,
            },
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::InstantaneousFrequency,
                checkpoint: false,
            },
            TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::Sweetness,
                checkpoint: false,
            },
        ],
        TraceLocalBenchmarkScenario::Bandpass => vec![TraceLocalProcessingStep {
            operation: TraceLocalProcessingOperation::BandpassFilter {
                f1_hz: 8.0,
                f2_hz: 12.0,
                f3_hz: 48.0,
                f4_hz: 56.0,
                phase: seis_runtime::FrequencyPhaseMode::Zero,
                window: seis_runtime::FrequencyWindowShape::CosineTaper,
            },
            checkpoint: false,
        }],
    };

    TraceLocalProcessingPipeline {
        schema_version: IPC_SCHEMA_VERSION,
        revision: 1,
        preset_id: None,
        name: Some(format!("benchmark-{}", benchmark_scenario_slug(scenario))),
        description: Some("Headless trace-local benchmark pipeline".to_string()),
        steps,
    }
}

fn post_stack_neighborhood_benchmark_pipeline(
    operator: PostStackNeighborhoodBenchmarkOperator,
    gate_ms: f32,
    inline_stepout: usize,
    xline_stepout: usize,
    include_trace_local_prefix: bool,
) -> PostStackNeighborhoodProcessingPipeline {
    let window = seis_runtime::PostStackNeighborhoodWindow {
        gate_ms,
        inline_stepout,
        xline_stepout,
    };
    let operations = match operator {
        PostStackNeighborhoodBenchmarkOperator::Similarity => {
            vec![seis_runtime::PostStackNeighborhoodProcessingOperation::Similarity { window }]
        }
        PostStackNeighborhoodBenchmarkOperator::LocalVolumeStatsMean => vec![
            seis_runtime::PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
                window,
                statistic: seis_runtime::LocalVolumeStatistic::Mean,
            },
        ],
    };
    PostStackNeighborhoodProcessingPipeline {
        schema_version: 1,
        revision: 1,
        preset_id: None,
        name: Some(post_stack_neighborhood_benchmark_label(
            operator,
            gate_ms,
            inline_stepout,
            xline_stepout,
            include_trace_local_prefix,
        )),
        description: None,
        trace_local_pipeline: include_trace_local_prefix.then(|| TraceLocalProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: Some("trace-rms-prefix".to_string()),
            description: None,
            steps: vec![TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::TraceRmsNormalize,
                checkpoint: false,
            }],
        }),
        operations,
    }
}

fn post_stack_neighborhood_benchmark_label(
    operator: PostStackNeighborhoodBenchmarkOperator,
    gate_ms: f32,
    inline_stepout: usize,
    xline_stepout: usize,
    include_trace_local_prefix: bool,
) -> String {
    format!(
        "{}-gate-{gate_ms:.0}ms-il-{inline_stepout}-xl-{xline_stepout}{}",
        match operator {
            PostStackNeighborhoodBenchmarkOperator::Similarity => "similarity",
            PostStackNeighborhoodBenchmarkOperator::LocalVolumeStatsMean => "local-mean",
        },
        if include_trace_local_prefix {
            "-with-prefix"
        } else {
            "-no-prefix"
        }
    )
}

fn post_stack_neighborhood_benchmark_slug(
    operator: PostStackNeighborhoodBenchmarkOperator,
    gate_ms: f32,
    inline_stepout: usize,
    xline_stepout: usize,
    include_trace_local_prefix: bool,
) -> String {
    post_stack_neighborhood_benchmark_label(
        operator,
        gate_ms,
        inline_stepout,
        xline_stepout,
        include_trace_local_prefix,
    )
    .replace('.', "_")
}

fn summarize_benchmark_variants(
    runs: &[TraceLocalBenchmarkRunResult],
) -> Vec<TraceLocalBenchmarkVariantSummary> {
    let mut summaries = Vec::new();
    let mut labels = runs
        .iter()
        .map(|run| run.label.as_str())
        .collect::<Vec<_>>();
    labels.sort_unstable();
    labels.dedup();
    for label in labels {
        let variant_runs = runs
            .iter()
            .filter(|run| run.label == label)
            .collect::<Vec<_>>();
        if variant_runs.is_empty() {
            continue;
        }
        let run_count = variant_runs.len();
        let elapsed_values = variant_runs
            .iter()
            .map(|run| run.elapsed_ms)
            .collect::<Vec<_>>();
        let elapsed_sum = elapsed_values.iter().sum::<f64>();
        let total_partitions_sum = variant_runs
            .iter()
            .map(|run| run.total_partitions as f64)
            .sum::<f64>();
        let peak_active_sum = variant_runs
            .iter()
            .map(|run| run.peak_active_partitions as f64)
            .sum::<f64>();
        summaries.push(TraceLocalBenchmarkVariantSummary {
            label: label.to_string(),
            scenario: variant_runs[0].scenario,
            partition_target_bytes: variant_runs[0].partition_target_bytes,
            run_count,
            avg_elapsed_ms: elapsed_sum / run_count as f64,
            min_elapsed_ms: elapsed_values.iter().copied().fold(f64::INFINITY, f64::min),
            max_elapsed_ms: elapsed_values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
            total_tiles: variant_runs[0].total_tiles,
            avg_total_partitions: total_partitions_sum / run_count as f64,
            avg_peak_active_partitions: peak_active_sum / run_count as f64,
        });
    }
    summaries.sort_by(|left, right| left.label.cmp(&right.label));
    summaries
}

fn summarize_post_stack_neighborhood_preview_runs(
    runs: &[PostStackNeighborhoodPreviewBenchmarkRunResult],
) -> Vec<PostStackNeighborhoodPreviewBenchmarkSummary> {
    let mut grouped: HashMap<String, Vec<&PostStackNeighborhoodPreviewBenchmarkRunResult>> =
        HashMap::new();
    for run in runs {
        grouped.entry(run.label.clone()).or_default().push(run);
    }
    let mut summaries = Vec::new();
    for (label, grouped_runs) in grouped {
        let elapsed: Vec<f64> = grouped_runs.iter().map(|run| run.elapsed_ms).collect();
        summaries.push(PostStackNeighborhoodPreviewBenchmarkSummary {
            label,
            run_count: elapsed.len(),
            avg_elapsed_ms: average_f64(elapsed.iter().copied()),
            min_elapsed_ms: elapsed.iter().copied().fold(f64::INFINITY, f64::min),
            max_elapsed_ms: elapsed.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        });
    }
    summaries.sort_by(|left, right| left.label.cmp(&right.label));
    summaries
}

fn summarize_batch_benchmark_variants(
    variants: &[TraceLocalBatchBenchmarkVariantResult],
) -> Vec<TraceLocalBatchBenchmarkSummary> {
    let mut summaries = Vec::new();
    let mut labels = variants
        .iter()
        .map(|variant| variant.label.as_str())
        .collect::<Vec<_>>();
    labels.sort_unstable();
    labels.dedup();
    for label in labels {
        let group = variants
            .iter()
            .filter(|variant| variant.label == label)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }
        summaries.push(TraceLocalBatchBenchmarkSummary {
            label: label.to_string(),
            requested_max_active_jobs: group[0].requested_max_active_jobs,
            effective_max_active_jobs: group[0].effective_max_active_jobs,
            execution_mode: group[0].execution_mode,
            scheduler_reason: group[0].scheduler_reason,
            run_count: group.len(),
            avg_batch_elapsed_ms: average_f64(group.iter().map(|variant| variant.batch_elapsed_ms)),
            min_batch_elapsed_ms: group
                .iter()
                .map(|variant| variant.batch_elapsed_ms)
                .fold(f64::INFINITY, f64::min),
            max_batch_elapsed_ms: group
                .iter()
                .map(|variant| variant.batch_elapsed_ms)
                .fold(f64::NEG_INFINITY, f64::max),
            avg_queue_wait_ms: average_f64(group.iter().map(|variant| variant.avg_queue_wait_ms)),
            avg_job_elapsed_ms: average_f64(group.iter().map(|variant| variant.avg_job_elapsed_ms)),
        });
    }
    summaries.sort_by(|left, right| left.label.cmp(&right.label));
    summaries
}

fn summarize_post_stack_neighborhood_processing_runs(
    runs: &[PostStackNeighborhoodProcessingBenchmarkRunResult],
) -> Vec<PostStackNeighborhoodProcessingBenchmarkSummary> {
    let mut grouped: HashMap<String, Vec<&PostStackNeighborhoodProcessingBenchmarkRunResult>> =
        HashMap::new();
    for run in runs {
        grouped.entry(run.label.clone()).or_default().push(run);
    }
    let mut summaries = Vec::new();
    for (label, grouped_runs) in grouped {
        let elapsed: Vec<f64> = grouped_runs.iter().map(|run| run.elapsed_ms).collect();
        summaries.push(PostStackNeighborhoodProcessingBenchmarkSummary {
            label,
            run_count: elapsed.len(),
            avg_elapsed_ms: average_f64(elapsed.iter().copied()),
            min_elapsed_ms: elapsed.iter().copied().fold(f64::INFINITY, f64::min),
            max_elapsed_ms: elapsed.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        });
    }
    summaries.sort_by(|left, right| left.label.cmp(&right.label));
    summaries
}

fn average_f64<I>(values: I) -> f64
where
    I: Iterator<Item = f64>,
{
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn dataset_slug(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(slugify)
        .unwrap_or_else(|| "dataset".to_string())
}

fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn remove_store_path_if_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn preflight_dataset(
    request: SurveyPreflightRequest,
) -> Result<SurveyPreflightResponse, Box<dyn std::error::Error>> {
    let geometry_override = request.geometry_override.clone();
    let input_path = request.input_path.clone();
    let preflight = preflight_segy(
        &request.input_path,
        &ingest_options_from_geometry_override(geometry_override.as_ref()),
    )?;
    let candidates = if geometry_override.is_none()
        && matches!(
            preflight.recommended_action,
            seis_runtime::PreflightAction::ReviewGeometryMapping
        ) {
        discover_geometry_candidates(&request.input_path, &preflight)
    } else {
        Vec::new()
    };
    let suggested_geometry_override = preferred_geometry_override(&candidates);
    Ok(preflight_response(
        input_path,
        &preflight,
        suggested_geometry_override,
        candidates,
    ))
}

pub fn scan_segy_import(
    request: ScanSegyImportRequest,
) -> Result<SegyImportScanResponse, Box<dyn std::error::Error>> {
    let input_path = request.input_path.trim().to_string();
    let source_fingerprint = source_fingerprint_for_input(&input_path)?;
    let output_store_path = default_tbvol_output_path(&input_path);
    let preflight = preflight_segy(&input_path, &IngestOptions::default())?;
    let geometry_candidates = if matches!(
        preflight.recommended_action,
        PreflightAction::ReviewGeometryMapping
    ) {
        discover_geometry_candidates(&input_path, &preflight)
    } else {
        Vec::new()
    };
    let default_plan = build_segy_import_plan(
        &input_path,
        &source_fingerprint,
        &output_store_path,
        geometry_override_from_preflight(&preflight),
        SegyImportPlanSource::ScanDefault,
        None,
        None,
        None,
    );
    let default_validation = validate_segy_import_plan(ValidateSegyImportPlanRequest {
        schema_version: request.schema_version,
        plan: default_plan.clone(),
    })?;

    let candidate_plans = geometry_candidates
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| {
            let candidate_id = format!("candidate-{:02}", index + 1);
            let plan = build_segy_import_plan(
                &input_path,
                &source_fingerprint,
                &output_store_path,
                candidate.geometry.clone(),
                SegyImportPlanSource::Candidate,
                Some(candidate_id.clone()),
                None,
                None,
            );
            let validation = validate_segy_import_plan(ValidateSegyImportPlanRequest {
                schema_version: request.schema_version,
                plan: plan.clone(),
            })?;
            Ok::<SegyImportCandidatePlan, Box<dyn std::error::Error>>(SegyImportCandidatePlan {
                candidate_id,
                label: candidate.label,
                plan_patch: plan,
                resolved_dataset: validation.resolved_dataset,
                risk_summary: validation.risk_summary,
                issues: validation.issues,
                auto_selectable: candidate.auto_selectable,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SegyImportScanResponse {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
        source_fingerprint,
        file_size: preflight.inspection.file_size,
        trace_count: preflight.inspection.trace_count,
        samples_per_trace: preflight.inspection.samples_per_trace as usize,
        sample_interval_us: preflight.inspection.sample_interval_us,
        sample_format_code: preflight.inspection.sample_format_code,
        endianness: preflight.inspection.endianness.clone(),
        default_plan,
        candidate_plans,
        field_observations: field_observations_from_preflight(&preflight),
        risk_summary: default_validation.risk_summary,
        issues: default_validation.issues,
        recommended_next_stage: default_validation.recommended_next_stage,
    })
}

pub fn validate_segy_import_plan(
    request: ValidateSegyImportPlanRequest,
) -> Result<SegyImportValidationResponse, Box<dyn std::error::Error>> {
    let validated_plan = normalize_segy_import_plan(request.plan);
    let preflight = preflight_segy(
        &validated_plan.input_path,
        &ingest_options_from_geometry_override(Some(&validated_plan.header_mapping)),
    )?;
    let issues = validate_segy_plan_issues(&validated_plan, &preflight);
    let requires_acknowledgement = issues
        .iter()
        .any(|issue| issue.severity == SegyImportIssueSeverity::Warning);
    let can_import = !issues
        .iter()
        .any(|issue| issue.severity == SegyImportIssueSeverity::Blocking)
        && (!requires_acknowledgement || validated_plan.policy.acknowledge_warnings);
    let recommended_next_stage = if issues.iter().any(|issue| {
        issue.severity == SegyImportIssueSeverity::Blocking
            && matches!(
                issue.section,
                SegyImportIssueSection::Structure | SegyImportIssueSection::Scan
            )
    }) {
        SegyImportWizardStage::Structure
    } else if issues.iter().any(|issue| {
        issue.severity == SegyImportIssueSeverity::Blocking
            && issue.section == SegyImportIssueSection::Spatial
    }) {
        SegyImportWizardStage::Spatial
    } else if can_import {
        SegyImportWizardStage::Import
    } else {
        SegyImportWizardStage::Review
    };

    Ok(SegyImportValidationResponse {
        schema_version: IPC_SCHEMA_VERSION,
        validation_fingerprint: validation_fingerprint_for_plan(&validated_plan)?,
        resolved_dataset: resolved_dataset_from_preflight(&preflight),
        resolved_spatial: resolved_spatial_from_plan(&validated_plan),
        risk_summary: risk_summary_for_preflight(
            &validated_plan.input_path,
            &validated_plan.header_mapping,
            &preflight,
        )?,
        issues,
        can_import,
        requires_acknowledgement,
        recommended_next_stage,
        validated_plan,
    })
}

pub fn import_segy_with_plan(
    request: ImportSegyWithPlanRequest,
) -> Result<ImportSegyWithPlanResponse, Box<dyn std::error::Error>> {
    let validation = validate_segy_import_plan(ValidateSegyImportPlanRequest {
        schema_version: request.schema_version,
        plan: request.plan,
    })?;
    if validation.validation_fingerprint != request.validation_fingerprint {
        return Err("The import plan changed. Validate again before importing."
            .to_string()
            .into());
    }
    if !validation.can_import {
        return Err("SEG-Y import plan is not ready to import."
            .to_string()
            .into());
    }

    let response = import_dataset(ImportDatasetRequest {
        schema_version: request.schema_version,
        input_path: validation.validated_plan.input_path.clone(),
        output_store_path: validation.validated_plan.policy.output_store_path.clone(),
        geometry_override: Some(validation.validated_plan.header_mapping.clone()),
        overwrite_existing: validation.validated_plan.policy.overwrite_existing,
    })?;

    Ok(ImportSegyWithPlanResponse {
        schema_version: IPC_SCHEMA_VERSION,
        dataset: response.dataset,
    })
}

pub fn import_dataset(
    request: ImportDatasetRequest,
) -> Result<ImportDatasetResponse, Box<dyn std::error::Error>> {
    let input = PathBuf::from(&request.input_path);
    let output = PathBuf::from(&request.output_store_path);
    ensure_import_capacity(
        &input,
        &output,
        [0, 0, 0],
        request.overwrite_existing,
        request.geometry_override.as_ref(),
    )?;
    prepare_output_store(&input, &output, request.overwrite_existing)?;
    let handle = ingest_volume(
        &input,
        &output,
        IngestOptions {
            geometry: geometry_override_to_seis_options(request.geometry_override.as_ref()),
            sparse_survey_policy: SparseSurveyPolicy::RegularizeToDense {
                fill_value: DEFAULT_SPARSE_FILL_VALUE,
            },
            ..IngestOptions::default()
        },
    )?;
    Ok(ImportDatasetResponse {
        schema_version: IPC_SCHEMA_VERSION,
        dataset: dataset_summary_for_path(&handle.root)?,
    })
}

fn build_segy_import_plan(
    input_path: &str,
    source_fingerprint: &str,
    output_store_path: &str,
    header_mapping: SegyGeometryOverride,
    plan_source: SegyImportPlanSource,
    selected_candidate_id: Option<String>,
    recipe_id: Option<String>,
    recipe_name: Option<String>,
) -> SegyImportPlan {
    SegyImportPlan {
        input_path: input_path.to_string(),
        source_fingerprint: source_fingerprint.to_string(),
        header_mapping,
        spatial: SegyImportSpatialPlan {
            x_field: None,
            y_field: None,
            coordinate_scalar_field: None,
            coordinate_units: None,
            coordinate_reference_id: None,
            coordinate_reference_name: None,
        },
        policy: SegyImportPolicy {
            sparse_handling: SegyImportSparseHandling::BlockImport,
            output_store_path: output_store_path.to_string(),
            overwrite_existing: false,
            acknowledge_warnings: false,
        },
        provenance: SegyImportProvenance {
            plan_source,
            selected_candidate_id,
            recipe_id,
            recipe_name,
        },
    }
}

fn normalize_segy_import_plan(mut plan: SegyImportPlan) -> SegyImportPlan {
    plan.input_path = plan.input_path.trim().to_string();
    plan.source_fingerprint = plan.source_fingerprint.trim().to_string();
    plan.policy.output_store_path = plan.policy.output_store_path.trim().to_string();
    plan.spatial.coordinate_units =
        normalized_optional_string(plan.spatial.coordinate_units.take());
    plan.spatial.coordinate_reference_id =
        normalized_optional_string(plan.spatial.coordinate_reference_id.take());
    plan.spatial.coordinate_reference_name =
        normalized_optional_string(plan.spatial.coordinate_reference_name.take());
    plan.provenance.selected_candidate_id =
        normalized_optional_string(plan.provenance.selected_candidate_id.take());
    plan.provenance.recipe_id = normalized_optional_string(plan.provenance.recipe_id.take());
    plan.provenance.recipe_name = normalized_optional_string(plan.provenance.recipe_name.take());
    plan
}

fn normalized_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn validate_segy_plan_issues(
    plan: &SegyImportPlan,
    preflight: &seis_runtime::SurveyPreflight,
) -> Vec<SegyImportIssue> {
    let mut issues = Vec::new();

    if plan.input_path.is_empty() {
        issues.push(import_issue(
            SegyImportIssueSeverity::Blocking,
            "input_path_required",
            "Choose a SEG-Y file before continuing.",
            SegyImportIssueSection::Scan,
        ));
    }
    if plan.policy.output_store_path.is_empty() {
        issues.push(import_issue(
            SegyImportIssueSeverity::Blocking,
            "output_store_path_required",
            "Choose a runtime store output path before importing.",
            SegyImportIssueSection::Import,
        ));
    }
    if plan.header_mapping.inline_3d.is_none() || plan.header_mapping.crossline_3d.is_none() {
        issues.push(import_issue(
            SegyImportIssueSeverity::Blocking,
            "header_mapping_incomplete",
            "Inline and crossline header mappings are required before import.",
            SegyImportIssueSection::Structure,
        ));
    }

    match preflight.recommended_action {
        PreflightAction::DirectDenseIngest => {}
        PreflightAction::RegularizeSparseSurvey => {
            if plan.policy.sparse_handling != SegyImportSparseHandling::RegularizeToDense {
                issues.push(SegyImportIssue {
                    severity: SegyImportIssueSeverity::Blocking,
                    code: "sparse_policy_required".to_string(),
                    message: "This mapping expands the survey into a dense grid. Review structure before import.".to_string(),
                    field_path: Some("policy.sparse_handling".to_string()),
                    section: SegyImportIssueSection::Structure,
                    source_path: Some(plan.input_path.clone()),
                    suggested_fix: Some("Set sparse handling to regularize to dense after reviewing the footprint estimate.".to_string()),
                });
            } else {
                issues.push(SegyImportIssue {
                    severity: SegyImportIssueSeverity::Warning,
                    code: "sparse_regularization".to_string(),
                    message: "This mapping expands the survey into a dense grid. Review structure before import.".to_string(),
                    field_path: Some("policy.sparse_handling".to_string()),
                    section: SegyImportIssueSection::Structure,
                    source_path: Some(plan.input_path.clone()),
                    suggested_fix: Some("Confirm the grid shape and storage estimate, then acknowledge the remaining warnings before importing.".to_string()),
                });
            }
        }
        PreflightAction::ReviewGeometryMapping => issues.push(SegyImportIssue {
            severity: SegyImportIssueSeverity::Blocking,
            code: "review_geometry_mapping".to_string(),
            message: "The current header mapping still produces duplicate or ambiguous bins. Adjust the structure mapping before importing.".to_string(),
            field_path: Some("header_mapping".to_string()),
            section: SegyImportIssueSection::Structure,
            source_path: Some(plan.input_path.clone()),
            suggested_fix: Some("Choose a candidate mapping or enter different inline and crossline header bytes.".to_string()),
        }),
        PreflightAction::UnsupportedInV1 => issues.push(SegyImportIssue {
            severity: SegyImportIssueSeverity::Blocking,
            code: "unsupported_in_v1".to_string(),
            message: "This SEG-Y layout is not supported by the current import path.".to_string(),
            field_path: Some("header_mapping".to_string()),
            section: SegyImportIssueSection::Structure,
            source_path: Some(plan.input_path.clone()),
            suggested_fix: Some("Inspect the raw trace layout or repair the source survey before retrying.".to_string()),
        }),
    }

    if preflight.geometry.duplicate_coordinate_count > 0 {
        issues.push(SegyImportIssue {
            severity: SegyImportIssueSeverity::Info,
            code: "duplicate_coordinates_observed".to_string(),
            message: format!(
                "{} duplicate coordinate tuples were observed while scanning the current mapping.",
                preflight.geometry.duplicate_coordinate_count
            ),
            field_path: Some("header_mapping".to_string()),
            section: SegyImportIssueSection::Structure,
            source_path: Some(plan.input_path.clone()),
            suggested_fix: None,
        });
    }
    if preflight.geometry.missing_bin_count > 0 {
        issues.push(SegyImportIssue {
            severity: SegyImportIssueSeverity::Info,
            code: "missing_bins_observed".to_string(),
            message: format!(
                "{} dense-grid bins would be empty under the current mapping.",
                preflight.geometry.missing_bin_count
            ),
            field_path: Some("header_mapping".to_string()),
            section: SegyImportIssueSection::Structure,
            source_path: Some(plan.input_path.clone()),
            suggested_fix: None,
        });
    }

    for warning in &preflight.inspection.warnings {
        issues.push(SegyImportIssue {
            severity: SegyImportIssueSeverity::Info,
            code: "inspection_warning".to_string(),
            message: warning.clone(),
            field_path: None,
            section: SegyImportIssueSection::Scan,
            source_path: Some(plan.input_path.clone()),
            suggested_fix: None,
        });
    }

    issues
}

fn import_issue(
    severity: SegyImportIssueSeverity,
    code: &str,
    message: &str,
    section: SegyImportIssueSection,
) -> SegyImportIssue {
    SegyImportIssue {
        severity,
        code: code.to_string(),
        message: message.to_string(),
        field_path: None,
        section,
        source_path: None,
        suggested_fix: None,
    }
}

fn resolved_dataset_from_preflight(
    preflight: &seis_runtime::SurveyPreflight,
) -> SegyImportResolvedDataset {
    SegyImportResolvedDataset {
        classification: preflight.geometry.classification.clone(),
        stacking_state: preflight.geometry.stacking_state.clone(),
        organization: preflight.geometry.organization.clone(),
        layout: preflight.geometry.layout.clone(),
        gather_axis_kind: preflight.geometry.gather_axis_kind.clone(),
        inline_count: preflight.geometry.inline_count,
        crossline_count: preflight.geometry.crossline_count,
        third_axis_count: preflight.geometry.third_axis_count,
        trace_count: preflight.inspection.trace_count,
        samples_per_trace: preflight.inspection.samples_per_trace as usize,
        sample_data_fidelity: preflight.sample_data_fidelity.clone(),
    }
}

fn resolved_spatial_from_plan(plan: &SegyImportPlan) -> SegyImportResolvedSpatial {
    let mut notes = Vec::new();
    if plan.spatial.coordinate_reference_id.is_some()
        || plan.spatial.coordinate_reference_name.is_some()
    {
        notes.push("Import will retain the supplied coordinate reference metadata for later survey map setup.".to_string());
    }
    SegyImportResolvedSpatial {
        x_field: plan.spatial.x_field.clone(),
        y_field: plan.spatial.y_field.clone(),
        coordinate_scalar_field: plan.spatial.coordinate_scalar_field.clone(),
        coordinate_units: plan.spatial.coordinate_units.clone(),
        coordinate_reference_id: plan.spatial.coordinate_reference_id.clone(),
        coordinate_reference_name: plan.spatial.coordinate_reference_name.clone(),
        notes,
    }
}

fn risk_summary_for_preflight(
    input_path: &str,
    geometry_override: &SegyGeometryOverride,
    preflight: &seis_runtime::SurveyPreflight,
) -> Result<SegyImportRiskSummary, Box<dyn std::error::Error>> {
    let estimate = estimate_sparse_segy_tbvol_storage(
        Path::new(input_path),
        [0, 0, 0],
        Some(geometry_override),
    )?;
    let observed_trace_count = preflight.geometry.observed_trace_count as u64;
    let expected_trace_count = preflight.geometry.expected_trace_count as u64;
    let blowup_ratio = if observed_trace_count > 0 {
        expected_trace_count as f64 / observed_trace_count as f64
    } else {
        0.0
    };
    Ok(SegyImportRiskSummary {
        observed_trace_count,
        expected_trace_count,
        completeness_ratio: preflight.geometry.completeness_ratio,
        blowup_ratio,
        estimated_amplitude_bytes: estimate.as_ref().map_or(0, |value| value.amplitude_bytes),
        estimated_occupancy_bytes: estimate.as_ref().map_or(0, |value| value.occupancy_bytes),
        estimated_total_bytes: estimate.as_ref().map_or(0, |value| value.total_bytes),
        classification: preflight.geometry.classification.clone(),
        suggested_action: suggested_action(preflight.recommended_action),
    })
}

fn field_observations_from_preflight(
    preflight: &seis_runtime::SurveyPreflight,
) -> Vec<SegyImportFieldObservation> {
    let mut fields = Vec::new();
    fields.push(SegyImportFieldObservation {
        field: contract_header_field_from_spec(&preflight.geometry.inline_field),
        label: "Resolved inline".to_string(),
        unique_count: preflight.geometry.inline_count,
        min_value: None,
        max_value: None,
        zero_count: 0,
        nonzero_count: preflight.geometry.observed_trace_count,
    });
    fields.push(SegyImportFieldObservation {
        field: contract_header_field_from_spec(&preflight.geometry.crossline_field),
        label: "Resolved crossline".to_string(),
        unique_count: preflight.geometry.crossline_count,
        min_value: None,
        max_value: None,
        zero_count: 0,
        nonzero_count: preflight.geometry.observed_trace_count,
    });
    if let Some(field) = preflight.geometry.third_axis_field.as_ref() {
        fields.push(SegyImportFieldObservation {
            field: contract_header_field_from_spec(field),
            label: "Resolved third axis".to_string(),
            unique_count: preflight.geometry.third_axis_count,
            min_value: None,
            max_value: None,
            zero_count: 0,
            nonzero_count: preflight.geometry.observed_trace_count,
        });
    }
    fields
}

fn source_fingerprint_for_input(input_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(input_path);
    let metadata = fs::metadata(path)?;
    let canonical_path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let modified_s = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |value| value.as_secs());
    Ok(format!(
        "segy:{}:{}:{}",
        canonical_path,
        metadata.len(),
        modified_s
    ))
}

fn validation_fingerprint_for_plan(
    plan: &SegyImportPlan,
) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec(plan)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn default_tbvol_output_path(input_path: &str) -> String {
    let path = Path::new(input_path);
    path.with_extension("tbvol").to_string_lossy().into_owned()
}

fn ensure_import_capacity(
    input_path: &Path,
    output_path: &Path,
    chunk_shape: [usize; 3],
    overwrite_existing: bool,
    geometry_override: Option<&SegyGeometryOverride>,
) -> Result<(), Box<dyn std::error::Error>> {
    match detect_volume_import_format(input_path)? {
        VolumeImportFormat::MdioStore => {}
        VolumeImportFormat::Segy => {
            let Some(estimate) =
                estimate_sparse_segy_tbvol_storage(input_path, chunk_shape, geometry_override)?
            else {
                return Ok(());
            };

            let probe_path = filesystem_probe_path(output_path.parent().unwrap_or(output_path));
            let Some(available_bytes) = filesystem_available_space_bytes(&probe_path)? else {
                return Ok(());
            };
            let effective_available_bytes =
                effective_available_import_bytes(available_bytes, output_path, overwrite_existing)?;
            let required_free_bytes = estimate
                .total_bytes
                .saturating_add(IMPORT_FREE_SPACE_RESERVE_BYTES);
            if effective_available_bytes >= required_free_bytes {
                return Ok(());
            }

            return Err(sparse_segy_import_capacity_error(
                &estimate,
                effective_available_bytes,
                &probe_path,
                input_path,
                output_path,
            )
            .into());
        }
        _ => return Ok(()),
    }

    let estimate = estimate_mdio_tbvol_storage(input_path, chunk_shape, None)?;
    let probe_path = filesystem_probe_path(output_path.parent().unwrap_or(output_path));
    let Some(available_bytes) = filesystem_available_space_bytes(&probe_path)? else {
        return Ok(());
    };
    let effective_available_bytes =
        effective_available_import_bytes(available_bytes, output_path, overwrite_existing)?;
    let required_free_bytes = estimate
        .total_bytes
        .saturating_add(IMPORT_FREE_SPACE_RESERVE_BYTES);
    if effective_available_bytes >= required_free_bytes {
        return Ok(());
    }

    Err(mdio_import_capacity_error(
        &estimate,
        effective_available_bytes,
        &probe_path,
        input_path,
        output_path,
    )
    .into())
}

#[derive(Debug, Clone, Copy)]
struct SegyRegularizedTbvolStorageEstimate {
    shape: [usize; 3],
    tile_shape: [usize; 3],
    observed_trace_count: usize,
    expected_trace_count: usize,
    completeness_ratio: f64,
    amplitude_bytes: u64,
    occupancy_bytes: u64,
    total_bytes: u64,
}

fn estimate_sparse_segy_tbvol_storage(
    input_path: &Path,
    chunk_shape: [usize; 3],
    geometry_override: Option<&SegyGeometryOverride>,
) -> Result<Option<SegyRegularizedTbvolStorageEstimate>, Box<dyn std::error::Error>> {
    let preflight = preflight_segy(
        input_path,
        &IngestOptions {
            geometry: geometry_override_to_seis_options(geometry_override),
            ..IngestOptions::default()
        },
    )?;
    if preflight.recommended_action != PreflightAction::RegularizeSparseSurvey
        || preflight.geometry.layout != "post_stack_3d"
        || preflight.geometry.third_axis_count > 1
    {
        return Ok(None);
    }

    let shape = [
        preflight.geometry.inline_count,
        preflight.geometry.crossline_count,
        preflight.inspection.samples_per_trace as usize,
    ];
    let tile_shape = resolve_import_chunk_shape(chunk_shape, shape);
    let geometry = TileGeometry::new(shape, tile_shape);
    let tile_count = geometry.tile_count() as u64;
    let amplitude_bytes = tile_count.saturating_mul(geometry.amplitude_tile_bytes());
    let occupancy_bytes = tile_count.saturating_mul(geometry.occupancy_tile_bytes());

    Ok(Some(SegyRegularizedTbvolStorageEstimate {
        shape,
        tile_shape,
        observed_trace_count: preflight.geometry.observed_trace_count,
        expected_trace_count: preflight.geometry.expected_trace_count,
        completeness_ratio: preflight.geometry.completeness_ratio,
        amplitude_bytes,
        occupancy_bytes,
        total_bytes: amplitude_bytes.saturating_add(occupancy_bytes),
    }))
}

fn effective_available_import_bytes(
    available_bytes: u64,
    output_path: &Path,
    overwrite_existing: bool,
) -> Result<u64, Box<dyn std::error::Error>> {
    let reclaimable_output_bytes = if overwrite_existing && output_path.exists() {
        path_size_bytes(output_path)?
    } else {
        0
    };
    let stale_temp_path = output_path.with_extension("tbvol.tmp");
    let reclaimable_temp_bytes = if stale_temp_path.exists() {
        path_size_bytes(&stale_temp_path)?
    } else {
        0
    };
    Ok(available_bytes
        .saturating_add(reclaimable_output_bytes)
        .saturating_add(reclaimable_temp_bytes))
}

fn mdio_import_capacity_error(
    estimate: &seis_runtime::MdioTbvolStorageEstimate,
    available_bytes: u64,
    probe_path: &Path,
    input_path: &Path,
    output_path: &Path,
) -> String {
    let occupancy_label = if estimate.has_occupancy {
        "with occupancy"
    } else {
        "without occupancy"
    };
    format!(
        "Importing MDIO store '{}' to '{}' needs about {} free for the TBVOL output ({:?} samples, tile {:?}, {}) plus a {} safety reserve, but only {} is available on '{}'. Use a smaller ROI/subset import or free disk space before retrying.",
        input_path.display(),
        output_path.display(),
        format_bytes(estimate.total_bytes),
        estimate.shape,
        estimate.tile_shape,
        occupancy_label,
        format_bytes(IMPORT_FREE_SPACE_RESERVE_BYTES),
        format_bytes(available_bytes),
        probe_path.display()
    )
}

fn sparse_segy_import_capacity_error(
    estimate: &SegyRegularizedTbvolStorageEstimate,
    available_bytes: u64,
    probe_path: &Path,
    input_path: &Path,
    output_path: &Path,
) -> String {
    format!(
        "Importing SEG-Y '{}' to '{}' would regularize {} observed traces into a dense {:?} TBVOL grid ({}, tile {:?}, amplitudes {}, occupancy {}) with completeness {:.4}% and needs about {} free plus a {} safety reserve, but only {} is available on '{}'. Review the inline/crossline mapping or choose a smaller survey before retrying.",
        input_path.display(),
        output_path.display(),
        estimate.observed_trace_count,
        estimate.shape,
        estimate.expected_trace_count,
        estimate.tile_shape,
        format_bytes(estimate.amplitude_bytes),
        format_bytes(estimate.occupancy_bytes),
        estimate.completeness_ratio * 100.0,
        format_bytes(estimate.total_bytes),
        format_bytes(IMPORT_FREE_SPACE_RESERVE_BYTES),
        format_bytes(available_bytes),
        probe_path.display()
    )
}

pub fn import_prestack_offset_dataset(
    request: ImportPrestackOffsetDatasetRequest,
) -> Result<ImportPrestackOffsetDatasetResponse, Box<dyn std::error::Error>> {
    let input = PathBuf::from(&request.input_path);
    let output = PathBuf::from(&request.output_store_path);
    prepare_output_store(&input, &output, request.overwrite_existing)?;
    let handle = ingest_prestack_offset_segy(
        &input,
        &output,
        IngestOptions {
            geometry: SeisGeometryOptions {
                third_axis_field: Some(prestack_third_axis_field(request.third_axis_field)),
                ..SeisGeometryOptions::default()
            },
            ..IngestOptions::default()
        },
    )?;
    Ok(ImportPrestackOffsetDatasetResponse {
        schema_version: IPC_SCHEMA_VERSION,
        dataset: dataset_summary_for_path(&handle.root)?,
    })
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit_index = 0usize;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{bytes} {}", UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

fn resolve_import_chunk_shape(chunk_shape: [usize; 3], shape: [usize; 3]) -> [usize; 3] {
    if chunk_shape.iter().all(|value| *value == 0) {
        return recommended_tbvol_tile_shape(
            shape,
            recommended_default_tbvol_tile_target_mib(shape),
        );
    }

    [
        chunk_shape[0].max(1).min(shape[0].max(1)),
        chunk_shape[1].max(1).min(shape[1].max(1)),
        chunk_shape[2].max(1).min(shape[2].max(1)),
    ]
}

fn filesystem_probe_path(path: &Path) -> PathBuf {
    let mut candidate = if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    };
    while !candidate.exists() {
        let Some(parent) = candidate.parent() else {
            return PathBuf::from(".");
        };
        candidate = parent.to_path_buf();
    }
    candidate
}

fn path_size_bytes(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_dir() {
        return Ok(metadata.len());
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        total = total.saturating_add(path_size_bytes(&entry.path())?);
    }
    Ok(total)
}

#[cfg(unix)]
fn filesystem_available_space_bytes(
    path: &Path,
) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let probe_path = filesystem_probe_path(path);
    let c_path = CString::new(probe_path.as_os_str().as_bytes())?;
    let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
    let result = unsafe { libc::statfs(c_path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let stats = unsafe { stats.assume_init() };
    let block_size = stats.f_bsize as u128;
    let available_blocks = stats.f_bavail as u128;
    let available_bytes = block_size
        .saturating_mul(available_blocks)
        .min(u64::MAX as u128) as u64;
    Ok(Some(available_bytes))
}

#[cfg(not(unix))]
fn filesystem_available_space_bytes(
    _path: &Path,
) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    Ok(None)
}

fn prepare_output_store(
    input_path: &Path,
    output_path: &Path,
    overwrite_existing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !overwrite_existing || !output_path.exists() {
        return Ok(());
    }

    let input_path = input_path
        .canonicalize()
        .unwrap_or_else(|_| input_path.to_path_buf());
    let output_path = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.to_path_buf());

    if input_path == output_path {
        return Err("Output store path cannot overwrite the input dataset path.".into());
    }

    let metadata = fs::symlink_metadata(&output_path)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(&output_path)?;
    } else {
        fs::remove_file(&output_path)?;
    }

    Ok(())
}

fn delete_input_path_after_success(
    input_path: &Path,
    output_path: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let canonical_input = input_path
        .canonicalize()
        .unwrap_or_else(|_| input_path.to_path_buf());
    let canonical_output = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.to_path_buf());

    if canonical_input == canonical_output || canonical_output.starts_with(&canonical_input) {
        return Err(format!(
            "Refusing to delete '{}' because it resolves to or contains the output store '{}'.",
            input_path.display(),
            output_path.display()
        )
        .into());
    }

    let metadata = fs::symlink_metadata(input_path)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(input_path)?;
    } else {
        fs::remove_file(input_path)?;
    }

    Ok(true)
}

pub fn open_dataset_summary(
    request: OpenDatasetRequest,
) -> Result<OpenDatasetResponse, Box<dyn std::error::Error>> {
    let store_path = request.store_path;
    Ok(OpenDatasetResponse {
        schema_version: IPC_SCHEMA_VERSION,
        dataset: dataset_summary_for_path(&store_path)?,
    })
}

pub fn dataset_operator_catalog(
    store_path: impl AsRef<Path>,
) -> Result<OperatorCatalog, Box<dyn std::error::Error>> {
    let dataset = dataset_summary_for_path(store_path)?;
    let descriptor = SeismicTraceDataDescriptor::from(&dataset.descriptor);
    Ok(operator_catalog_for_trace_data(&descriptor))
}

pub fn set_dataset_native_coordinate_reference(
    request: SetDatasetNativeCoordinateReferenceRequest,
) -> Result<SetDatasetNativeCoordinateReferenceResponse, Box<dyn std::error::Error>> {
    set_any_store_native_coordinate_reference(
        &request.store_path,
        request.coordinate_reference_id.as_deref(),
        request.coordinate_reference_name.as_deref(),
    )?;
    let dataset = open_dataset_summary(OpenDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: request.store_path,
    })?
    .dataset;
    Ok(SetDatasetNativeCoordinateReferenceResponse {
        schema_version: IPC_SCHEMA_VERSION,
        dataset,
    })
}

pub fn resolve_survey_map(
    request: ResolveSurveyMapRequest,
) -> Result<ResolveSurveyMapResponse, Box<dyn std::error::Error>> {
    let dataset = open_dataset_summary(OpenDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: request.store_path.clone(),
    })?
    .dataset;
    let survey_map = resolve_dataset_summary_survey_map_source(
        &dataset,
        request.display_coordinate_reference_id.as_deref(),
        None,
        Some(Path::new(&request.store_path)),
    )?;
    Ok(ResolveSurveyMapResponse {
        schema_version: IPC_SCHEMA_VERSION,
        survey_map,
    })
}

pub fn export_dataset_segy(
    request: ExportSegyRequest,
) -> Result<ExportSegyResponse, Box<dyn std::error::Error>> {
    let store_path = PathBuf::from(&request.store_path);
    let output_path = PathBuf::from(&request.output_path);
    prepare_export_output_path(
        &store_path,
        &output_path,
        request.overwrite_existing,
        "SEG-Y file",
    )?;
    export_store_to_segy(&store_path, &output_path, request.overwrite_existing)?;
    Ok(ExportSegyResponse {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: request.store_path,
        output_path: request.output_path,
    })
}

pub fn export_dataset_zarr(
    store_path: String,
    output_path: String,
    overwrite_existing: bool,
) -> Result<ExportZarrResponse, Box<dyn std::error::Error>> {
    let store_path_buf = PathBuf::from(&store_path);
    let output_path_buf = PathBuf::from(&output_path);
    prepare_export_output_path(
        &store_path_buf,
        &output_path_buf,
        overwrite_existing,
        "Zarr store",
    )?;
    export_store_to_zarr(&store_path_buf, &output_path_buf, overwrite_existing)?;
    Ok(ExportZarrResponse {
        store_path,
        output_path,
    })
}

pub fn import_horizon_xyz(
    request: ImportHorizonXyzRequest,
) -> Result<ImportHorizonXyzResponse, Box<dyn std::error::Error>> {
    let imported = import_horizon_xyzs_with_vertical_domain(
        &request.store_path,
        &request
            .input_paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
        request.vertical_domain.unwrap_or(TimeDepthDomain::Time),
        request.vertical_unit.as_deref(),
        request.source_coordinate_reference_id.as_deref(),
        request.source_coordinate_reference_name.as_deref(),
        request.assume_same_as_survey,
    )?;
    Ok(ImportHorizonXyzResponse {
        schema_version: IPC_SCHEMA_VERSION,
        imported,
    })
}

pub fn preview_horizon_xyz_import(
    request: ImportHorizonXyzRequest,
) -> Result<HorizonImportPreview, Box<dyn std::error::Error>> {
    preview_horizon_xyzs_with_vertical_domain(
        &request.store_path,
        &request
            .input_paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
        request.vertical_domain.unwrap_or(TimeDepthDomain::Time),
        request.vertical_unit.as_deref(),
        request.source_coordinate_reference_id.as_deref(),
        request.source_coordinate_reference_name.as_deref(),
        request.assume_same_as_survey,
    )
    .map_err(Into::into)
}

pub fn load_section_horizons(
    request: LoadSectionHorizonsRequest,
) -> Result<LoadSectionHorizonsResponse, Box<dyn std::error::Error>> {
    let overlays = section_horizon_overlays(&request.store_path, request.axis, request.index)?;
    Ok(LoadSectionHorizonsResponse {
        schema_version: IPC_SCHEMA_VERSION,
        overlays,
    })
}

pub fn load_horizon_assets(
    store_path: String,
) -> Result<Vec<seis_runtime::ImportedHorizonDescriptor>, Box<dyn std::error::Error>> {
    let horizons = load_horizon_grids(&store_path)?
        .into_iter()
        .map(|grid| grid.descriptor)
        .collect::<Vec<_>>();
    Ok(horizons)
}

pub fn load_depth_converted_section(
    store_path: String,
    axis: seis_runtime::SectionAxis,
    index: usize,
    velocity_model: VelocityFunctionSource,
    velocity_kind: seis_runtime::VelocityQuantityKind,
) -> Result<seis_runtime::SectionView, Box<dyn std::error::Error>> {
    let handle = open_store(&store_path)?;
    let section =
        depth_converted_section_view(&store_path, axis, index, &velocity_model, velocity_kind)?;
    ensure_dataset_matches(&handle, &section.dataset_id.0)?;
    Ok(section)
}

pub fn load_resolved_section_display(
    store_path: String,
    axis: seis_runtime::SectionAxis,
    index: usize,
    domain: TimeDepthDomain,
    velocity_model: Option<VelocityFunctionSource>,
    velocity_kind: Option<seis_runtime::VelocityQuantityKind>,
    include_velocity_overlay: bool,
) -> Result<ResolvedSectionDisplayView, Box<dyn std::error::Error>> {
    let handle = open_store(&store_path)?;
    let display = resolved_section_display_view(
        &store_path,
        axis,
        index,
        domain,
        velocity_model.as_ref(),
        velocity_kind,
        include_velocity_overlay,
    )?;
    ensure_dataset_matches(&handle, &display.section.dataset_id.0)?;
    Ok(display)
}

pub fn ensure_demo_survey_time_depth_transform(
    store_path: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let handle = open_store(&store_path)?;
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;
    if sample_axis_ms.is_empty() {
        return Err(
            "Cannot create a survey time-depth transform for a store without a sample axis.".into(),
        );
    }

    let shape = handle.manifest.volume.shape;
    let inline_count = shape[0];
    let xline_count = shape[1];
    let sample_count = shape[2];
    if inline_count == 0 || xline_count == 0 || sample_count == 0 {
        return Err("Cannot create a survey time-depth transform for an empty survey grid.".into());
    }

    let time_axis = VerticalAxisDescriptor {
        domain: TimeDepthDomain::Time,
        unit: "ms".to_string(),
        start: sample_axis_ms[0],
        step: inferred_sample_interval_ms(sample_axis_ms),
        count: sample_axis_ms.len(),
    };
    let descriptor = SurveyTimeDepthTransform3D {
        id: DEMO_SURVEY_TIME_DEPTH_TRANSFORM_ID.to_string(),
        name: DEMO_SURVEY_TIME_DEPTH_TRANSFORM_NAME.to_string(),
        derived_from: vec![handle.dataset_id().0.clone()],
        source_kind: TimeDepthTransformSourceKind::VelocityGrid3D,
        coordinate_reference: handle
            .manifest
            .volume
            .coordinate_reference_binding
            .as_ref()
            .and_then(|binding| binding.effective.clone()),
        grid_transform: handle
            .manifest
            .volume
            .spatial
            .as_ref()
            .and_then(|spatial| spatial.grid_transform.clone()),
        time_axis,
        depth_unit: "m".to_string(),
        inline_count,
        xline_count,
        sample_count,
        coverage: SpatialCoverageSummary {
            relationship: SpatialCoverageRelationship::Exact,
            source_coordinate_reference: handle
                .manifest
                .volume
                .coordinate_reference_binding
                .as_ref()
                .and_then(|binding| binding.effective.clone()),
            target_coordinate_reference: handle
                .manifest
                .volume
                .coordinate_reference_binding
                .as_ref()
                .and_then(|binding| binding.effective.clone()),
            notes: vec![
                "Synthetic survey-aligned trace-varying transform for time-depth demo workflows."
                    .to_string(),
            ],
        },
        notes: vec![
            "This transform is synthetic demo data, not an imported velocity model.".to_string(),
            "It is survey-aligned and spatially varying so TraceBoost can exercise the survey-3D section conversion path.".to_string(),
        ],
    };

    let cell_count = inline_count * xline_count * sample_count;
    let mut depths_m = vec![0.0_f32; cell_count];
    let validity = vec![1_u8; cell_count];
    for inline_index in 0..inline_count {
        for xline_index in 0..xline_count {
            let mut cumulative_depth_m = 0.0_f32;
            let inline_ratio = normalized_index(inline_index, inline_count);
            let xline_ratio = normalized_index(xline_index, xline_count);
            let structural_uplift =
                (-(distance_squared(inline_ratio, xline_ratio, 0.58, 0.46) / 0.035)).exp() * 14.0;
            let layer_one = 0.18 + f32::sin(inline_ratio * std::f32::consts::TAU * 1.15) * 0.035;
            let layer_two =
                0.36 + f32::sin(xline_ratio * std::f32::consts::TAU * 1.35 + 0.55) * 0.045;
            let layer_three =
                0.56 + f32::sin((inline_ratio + xline_ratio) * std::f32::consts::PI * 1.4) * 0.05;
            let layer_four = 0.74
                + f32::cos((inline_ratio * 0.7 + xline_ratio * 1.3) * std::f32::consts::PI * 1.6)
                    * 0.055;

            let mut previous_time_ms = 0.0_f32;
            for sample_index in 0..sample_count {
                let offset =
                    ((inline_index * xline_count + xline_index) * sample_count) + sample_index;
                let time_ms = sample_axis_ms[sample_index];
                let dt_ms = if sample_index == 0 {
                    time_ms.max(0.0)
                } else {
                    (time_ms - previous_time_ms).max(0.0)
                };
                previous_time_ms = time_ms;

                let vertical_ratio = normalized_index(sample_index, sample_count)
                    - structural_uplift / sample_count as f32;
                let layer_index = if vertical_ratio < layer_one {
                    0
                } else if vertical_ratio < layer_two {
                    1
                } else if vertical_ratio < layer_three {
                    2
                } else if vertical_ratio < layer_four {
                    3
                } else {
                    4
                };
                let base_velocity_m_per_s =
                    [1525.0_f32, 1810.0, 2225.0, 2735.0, 3320.0][layer_index];
                let lateral_trend = f32::sin(inline_ratio * std::f32::consts::TAU * 1.3) * 130.0
                    + f32::cos(xline_ratio * std::f32::consts::TAU * 1.1) * 95.0;
                let local_variation = f32::sin(sample_index as f32 / 17.0 + inline_ratio * 4.8)
                    * 36.0
                    + f32::cos(sample_index as f32 / 23.0 + xline_ratio * 5.6) * 28.0;
                let deepening_trend = normalized_index(sample_index, sample_count) * 260.0;
                let interval_velocity_m_per_s =
                    (base_velocity_m_per_s + lateral_trend + local_variation + deepening_trend)
                        .clamp(1450.0, 3900.0);

                cumulative_depth_m += interval_velocity_m_per_s * (dt_ms * 0.001) * 0.5;
                depths_m[offset] = cumulative_depth_m;
            }
        }
    }

    let stored = store_survey_time_depth_transform(&store_path, descriptor, &depths_m, &validity)?;
    Ok(stored.id)
}

pub fn load_velocity_models(
    store_path: String,
) -> Result<LoadVelocityModelsResponse, Box<dyn std::error::Error>> {
    let models = load_survey_time_depth_transforms(&store_path)?
        .into_iter()
        .map(|transform| transform.descriptor)
        .collect::<Vec<_>>();
    Ok(LoadVelocityModelsResponse {
        schema_version: IPC_SCHEMA_VERSION,
        models,
    })
}

pub fn describe_velocity_volume_store(
    store_path: String,
    velocity_kind: VelocityQuantityKind,
) -> Result<VelocitySource3D, Box<dyn std::error::Error>> {
    let handle = open_store(&store_path)?;
    let options = native_velocity_volume_descriptor_options_for_axes(&handle.manifest.volume.axes);
    describe_velocity_volume_store_handle_with_options(handle, velocity_kind, options)
}

pub fn describe_velocity_volume(
    request: DescribeVelocityVolumeRequest,
) -> Result<DescribeVelocityVolumeResponse, Box<dyn std::error::Error>> {
    let handle = open_store(&request.store_path)?;
    let native = native_velocity_volume_descriptor_options_for_axes(&handle.manifest.volume.axes);
    let vertical_domain = request.vertical_domain.unwrap_or(native.vertical_domain);
    let volume = describe_velocity_volume_store_handle_with_options(
        handle,
        request.velocity_kind,
        VelocityVolumeDescriptorOptions {
            vertical_domain,
            vertical_unit: request.vertical_unit.unwrap_or_else(|| {
                if vertical_domain == native.vertical_domain {
                    native.vertical_unit.clone()
                } else {
                    default_vertical_axis_unit(vertical_domain)
                }
            }),
            vertical_start: request.vertical_start,
            vertical_step: request.vertical_step,
        },
    )?;

    Ok(DescribeVelocityVolumeResponse {
        schema_version: IPC_SCHEMA_VERSION,
        volume,
    })
}

fn describe_velocity_volume_store_with_options(
    store_path: String,
    velocity_kind: VelocityQuantityKind,
    options: VelocityVolumeDescriptorOptions,
) -> Result<VelocitySource3D, Box<dyn std::error::Error>> {
    let handle = open_store(&store_path)?;
    describe_velocity_volume_store_handle_with_options(handle, velocity_kind, options)
}

fn describe_velocity_volume_store_handle_with_options(
    handle: seis_runtime::StoreHandle,
    velocity_kind: VelocityQuantityKind,
    options: VelocityVolumeDescriptorOptions,
) -> Result<VelocitySource3D, Box<dyn std::error::Error>> {
    let label = handle.volume_descriptor().label;
    let coordinate_reference = handle
        .manifest
        .volume
        .coordinate_reference_binding
        .as_ref()
        .and_then(|binding| binding.effective.clone());
    let grid_transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.clone());
    let shape = handle.manifest.volume.shape;
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;
    let (vertical_start, vertical_step, vertical_count) =
        summarize_vertical_axis(sample_axis_ms, shape[2], &options)?;

    let mut notes = vec![
        "Canonical dense velocity source derived directly from an imported volume store."
            .to_string(),
        format!(
            "Velocity quantity kind is user-declared as {:?}.",
            velocity_kind
        ),
        format!(
            "The vertical axis is interpreted for canonical use as {:?} in {}.",
            options.vertical_domain, options.vertical_unit
        ),
    ];
    if let Some(regularization) = handle.manifest.volume.source.regularization.as_ref() {
        notes.push(format!(
            "Regularized sparse source survey: {} missing bins were filled with {}.",
            regularization.missing_bin_count, regularization.fill_value
        ));
    }
    notes.extend(
        handle
            .manifest
            .volume
            .source
            .sample_data_fidelity
            .notes
            .iter()
            .cloned(),
    );
    if coordinate_reference.is_none() {
        notes.push(
            "Coordinate reference is not resolved on the store; apply a native CRS override if known."
                .to_string(),
        );
    }
    if grid_transform.is_none() {
        notes.push(
            "Grid transform is not resolved on the store; map-space alignment will remain unavailable."
                .to_string(),
        );
    }

    let coverage_relationship = handle
        .manifest
        .volume
        .source
        .regularization
        .as_ref()
        .filter(|regularization| regularization.missing_bin_count > 0)
        .map(|_| SpatialCoverageRelationship::PartialOverlap)
        .unwrap_or(SpatialCoverageRelationship::Exact);

    Ok(VelocitySource3D {
        id: handle.dataset_id().0,
        name: label,
        source_kind: TimeDepthTransformSourceKind::VelocityGrid3D,
        velocity_kind,
        vertical_domain: options.vertical_domain,
        velocity_unit: "m/s".to_string(),
        coordinate_reference: coordinate_reference.clone(),
        grid_transform,
        vertical_axis: VerticalAxisDescriptor {
            domain: options.vertical_domain,
            unit: options.vertical_unit.clone(),
            start: vertical_start,
            step: vertical_step,
            count: vertical_count,
        },
        inline_count: shape[0],
        xline_count: shape[1],
        coverage: SpatialCoverageSummary {
            relationship: coverage_relationship,
            source_coordinate_reference: coordinate_reference,
            target_coordinate_reference: None,
            notes: Vec::new(),
        },
        notes,
    })
}

fn native_velocity_volume_descriptor_options_for_axes(
    axes: &seis_runtime::VolumeAxes,
) -> VelocityVolumeDescriptorOptions {
    let native_unit = axes.sample_axis_unit.trim();
    let vertical_domain = axes.sample_axis_domain;
    let vertical_unit = if native_unit.is_empty() {
        default_vertical_axis_unit(vertical_domain)
    } else {
        native_unit.to_string()
    };
    VelocityVolumeDescriptorOptions {
        vertical_domain,
        vertical_unit,
        vertical_start: None,
        vertical_step: None,
    }
}

fn summarize_vertical_axis(
    sample_axis_ms: &[f32],
    expected_count: usize,
    options: &VelocityVolumeDescriptorOptions,
) -> Result<(f32, f32, usize), Box<dyn std::error::Error>> {
    if sample_axis_ms.len() != expected_count {
        return Err(format!(
            "Volume sample axis length mismatch: expected {expected_count}, found {}",
            sample_axis_ms.len()
        )
        .into());
    }
    let inferred_start = sample_axis_ms.first().copied().unwrap_or(0.0);
    let inferred_step = if sample_axis_ms.len() >= 2 {
        sample_axis_ms[1] - sample_axis_ms[0]
    } else {
        0.0
    };
    Ok((
        options.vertical_start.unwrap_or(inferred_start),
        options.vertical_step.unwrap_or(inferred_step),
        sample_axis_ms.len(),
    ))
}

pub fn ingest_velocity_volume(
    request: IngestVelocityVolumeRequest,
) -> Result<IngestVelocityVolumeResponse, Box<dyn std::error::Error>> {
    ingest_velocity_volume_with_options(
        request.input_path,
        request.output_store_path,
        request.velocity_kind,
        request.vertical_domain,
        request.vertical_unit,
        request.vertical_start,
        request.vertical_step,
        request.overwrite_existing,
        request.delete_input_on_success,
        request.geometry_override.as_ref(),
    )
}

pub fn ingest_velocity_volume_with_options(
    input_path: String,
    output_store_path: String,
    velocity_kind: VelocityQuantityKind,
    vertical_domain: TimeDepthDomain,
    vertical_unit: Option<String>,
    vertical_start: Option<f32>,
    vertical_step: Option<f32>,
    overwrite_existing: bool,
    delete_input_on_success: bool,
    geometry_override: Option<&SegyGeometryOverride>,
) -> Result<IngestVelocityVolumeResponse, Box<dyn std::error::Error>> {
    let input = PathBuf::from(&input_path);
    let output = PathBuf::from(&output_store_path);
    prepare_output_store(&input, &output, overwrite_existing)?;
    let handle = ingest_volume(
        &input,
        &output,
        IngestOptions {
            geometry: geometry_override_to_seis_options(geometry_override),
            sparse_survey_policy: SparseSurveyPolicy::RegularizeToDense {
                fill_value: DEFAULT_SPARSE_FILL_VALUE,
            },
            ..IngestOptions::default()
        },
    )?;

    set_store_vertical_axis(
        &handle.root,
        vertical_domain,
        vertical_unit.as_deref(),
        vertical_start,
        vertical_step,
    )?;
    let descriptor =
        describe_velocity_volume_store(handle.root.to_string_lossy().into_owned(), velocity_kind)?;

    let deleted_input = if delete_input_on_success {
        delete_input_path_after_success(&input, &handle.root)?
    } else {
        false
    };

    Ok(IngestVelocityVolumeResponse {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
        store_path: handle.root.to_string_lossy().into_owned(),
        deleted_input,
        volume: descriptor,
    })
}

fn default_vertical_axis_unit(domain: TimeDepthDomain) -> String {
    match domain {
        TimeDepthDomain::Time => "ms".to_string(),
        TimeDepthDomain::Depth => "m".to_string(),
    }
}

pub fn build_velocity_model_transform(
    request: BuildSurveyTimeDepthTransformRequest,
) -> Result<SurveyTimeDepthTransform3D, Box<dyn std::error::Error>> {
    let model = build_survey_time_depth_transform(&request)?;
    Ok(model)
}

pub fn build_paired_horizon_transform(
    store_path: String,
    time_horizon_ids: Vec<String>,
    depth_horizon_ids: Vec<String>,
    output_id: Option<String>,
    output_name: Option<String>,
) -> Result<SurveyTimeDepthTransform3D, Box<dyn std::error::Error>> {
    let model = build_survey_time_depth_transform_from_horizon_pairs(
        &store_path,
        &time_horizon_ids,
        &depth_horizon_ids,
        output_id,
        output_name,
        &vec![
            "Built directly from paired canonical TWT and depth horizons.".to_string(),
            "Recommended when synthetic or interpreted horizon pairs define the target structural geometry more accurately than sparse Vint control profiles alone.".to_string(),
        ],
    )?;
    Ok(model)
}

pub fn convert_horizon_domain(
    store_path: String,
    source_horizon_id: String,
    transform_id: String,
    target_domain: TimeDepthDomain,
    output_id: Option<String>,
    output_name: Option<String>,
) -> Result<seis_runtime::ImportedHorizonDescriptor, Box<dyn std::error::Error>> {
    let descriptor = convert_horizon_vertical_domain_with_transform(
        &store_path,
        &source_horizon_id,
        &transform_id,
        target_domain,
        output_id,
        output_name,
    )?;
    Ok(descriptor)
}

pub fn import_velocity_functions_model(
    store_path: String,
    input_path: String,
    velocity_kind: VelocityQuantityKind,
) -> Result<ImportVelocityFunctionsModelResponse, Box<dyn std::error::Error>> {
    let parsed = parse_velocity_control_profiles_file(Path::new(&input_path), velocity_kind)?;
    if parsed.profiles.is_empty() {
        return Err("Velocity control profile file did not contain any control profiles.".into());
    }

    let handle = open_store(&store_path)?;
    let coordinate_reference = handle
        .manifest
        .volume
        .coordinate_reference_binding
        .as_ref()
        .and_then(|binding| binding.effective.clone());
    let grid_transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.clone());
    let source_stem = file_stem_from_path(&input_path);
    let output_slug = slugify(&format!(
        "{}-{}",
        source_stem,
        velocity_quantity_kind_slug(velocity_kind)
    ));
    let control_profile_set_id = format!("{output_slug}-control-profiles");
    let model = LayeredVelocityModel {
        id: format!("{output_slug}-layered-model"),
        name: format!(
            "{} {} Control Profiles",
            display_name_from_stem(&source_stem),
            velocity_quantity_kind_label(velocity_kind)
        ),
        derived_from: vec![handle.dataset_id().0.clone(), input_path.clone()],
        coordinate_reference: coordinate_reference.clone(),
        grid_transform: grid_transform.clone(),
        vertical_domain: TimeDepthDomain::Time,
        travel_time_reference: Some(TravelTimeReference::TwoWay),
        depth_reference: Some(DepthReferenceKind::TrueVerticalDepth),
        intervals: vec![LayeredVelocityInterval {
            id: format!("{output_slug}-survey-interval"),
            name: "Survey interval".to_string(),
            top_boundary: StratigraphicBoundaryReference::SurveyTop,
            base_boundary: StratigraphicBoundaryReference::SurveyBase,
            trend: VelocityIntervalTrend::Constant {
                velocity_m_per_s: 1500.0,
            },
            control_profile_set_id: Some(control_profile_set_id.clone()),
            control_profile_velocity_kind: Some(velocity_kind),
            lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
            vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
            control_blend_weight: Some(1.0),
            notes: vec![
                "Built from sparse velocity control profiles imported from text.".to_string(),
            ],
        }],
        notes: vec![
            "Single-interval authored model compiled from sparse control profiles.".to_string(),
            "Current builder path uses nearest lateral interpolation and linear vertical interpolation."
                .to_string(),
        ],
    };
    let request = BuildSurveyTimeDepthTransformRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: store_path.clone(),
        model,
        control_profile_sets: vec![VelocityControlProfileSet {
            id: control_profile_set_id,
            name: format!(
                "{} {} Profiles",
                display_name_from_stem(&source_stem),
                velocity_quantity_kind_label(velocity_kind)
            ),
            derived_from: vec![input_path.clone()],
            coordinate_reference,
            travel_time_reference: TravelTimeReference::TwoWay,
            depth_reference: DepthReferenceKind::TrueVerticalDepth,
            profiles: parsed.profiles.clone(),
            notes: vec!["Imported from sparse velocity control profile text file.".to_string()],
        }],
        output_id: Some(format!("{output_slug}-survey-transform")),
        output_name: Some(format!(
            "{} {} Transform",
            display_name_from_stem(&source_stem),
            velocity_quantity_kind_label(velocity_kind)
        )),
        preferred_velocity_kind: Some(velocity_kind),
        output_depth_unit: "m".to_string(),
        notes: vec![
            format!("Imported from {}", Path::new(&input_path).display()),
            "Compiled from sparse control profiles through the authored-model builder.".to_string(),
        ],
    };
    let model = build_survey_time_depth_transform(&request)?;

    Ok(ImportVelocityFunctionsModelResponse {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
        velocity_kind,
        profile_count: parsed.profiles.len(),
        sample_count: parsed.sample_count,
        model,
    })
}

fn normalized_index(index: usize, count: usize) -> f32 {
    if count <= 1 {
        0.0
    } else {
        index as f32 / (count - 1) as f32
    }
}

fn inferred_sample_interval_ms(sample_axis_ms: &[f32]) -> f32 {
    if sample_axis_ms.len() >= 2 {
        sample_axis_ms[1] - sample_axis_ms[0]
    } else {
        0.0
    }
}

fn distance_squared(x: f32, y: f32, center_x: f32, center_y: f32) -> f32 {
    let dx = x - center_x;
    let dy = y - center_y;
    dx * dx + dy * dy
}

#[cfg(test)]
fn parse_velocity_functions_file(
    input_path: &Path,
) -> Result<ParsedVelocityFunctions, Box<dyn std::error::Error>> {
    parse_velocity_control_profiles_file(input_path, VelocityQuantityKind::Interval)
}

fn parse_velocity_control_profiles_file(
    input_path: &Path,
    velocity_kind: VelocityQuantityKind,
) -> Result<ParsedVelocityFunctions, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(input_path)?;
    let navigation_index = try_load_velocity_navigation_index(input_path)?;
    let mut rows_by_profile = HashMap::<(u64, u64), Vec<ParsedVelocityProfileRow>>::new();
    let mut sample_count = 0_usize;
    let mut active_navigation_line = None::<String>;

    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if should_skip_velocity_control_profile_line(line) {
            continue;
        }

        if let Some(parsed_row) = try_parse_nlog_fixed_width_velocity_profile_row(
            raw_line,
            line_index + 1,
            velocity_kind,
        )? {
            rows_by_profile
                .entry((parsed_row.x.to_bits(), parsed_row.y.to_bits()))
                .or_default()
                .push(parsed_row);
            sample_count += 1;
            continue;
        }

        let columns = line
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();

        if let Some(navigation_line_key) =
            parse_velocity_navigation_section_header(&columns, navigation_index.as_ref())
        {
            active_navigation_line = Some(navigation_line_key);
            continue;
        }

        if let Some(parsed_row) = try_parse_navigation_velocity_profile_row(
            &columns,
            line_index + 1,
            velocity_kind,
            navigation_index.as_ref(),
            active_navigation_line.as_deref(),
        )? {
            rows_by_profile
                .entry((parsed_row.x.to_bits(), parsed_row.y.to_bits()))
                .or_default()
                .push(parsed_row);
            sample_count += 1;
            continue;
        }

        if let Some(parsed_row) =
            try_parse_nlog_ascii_velocity_profile_row(&columns, line_index + 1, velocity_kind)?
        {
            rows_by_profile
                .entry((parsed_row.x.to_bits(), parsed_row.y.to_bits()))
                .or_default()
                .push(parsed_row);
            sample_count += 1;
            continue;
        }

        let Some(numeric_columns) =
            parse_velocity_control_profile_numeric_columns(&columns, line_index + 1)?
        else {
            continue;
        };

        let parsed_row = match numeric_columns.len() {
            count if count >= 7 => parse_full_velocity_profile_row(&numeric_columns, line_index + 1)?,
            4 => parse_single_velocity_profile_row(&numeric_columns, line_index + 1, velocity_kind)?,
            3 => {
                return Err(format!(
                    "Velocity control profile row {} has 3 numeric columns. Headerless 3-column rows need line/navigation mapping before import; expected X Y Time Velocity for direct import.",
                    line_index + 1
                )
                .into())
            }
            count => {
                return Err(format!(
                    "Velocity control profile row {} is invalid: unsupported numeric layout with {} columns.",
                    line_index + 1,
                    count
                )
                .into())
            }
        };

        rows_by_profile
            .entry((parsed_row.x.to_bits(), parsed_row.y.to_bits()))
            .or_default()
            .push(parsed_row);
        sample_count += 1;
    }

    let mut profiles = rows_by_profile
        .into_values()
        .enumerate()
        .map(|(profile_index, mut rows)| {
            rows.sort_by(|left, right| left.sample.time_ms.total_cmp(&right.sample.time_ms));
            let first = rows
                .first()
                .ok_or_else(|| "Velocity profile group was unexpectedly empty.".to_string())?;
            Ok::<VelocityControlProfile, Box<dyn std::error::Error>>(VelocityControlProfile {
                id: format!("control-profile-{:05}", profile_index + 1),
                location: ProjectedPoint2 {
                    x: first.x,
                    y: first.y,
                },
                wellbore_id: None,
                samples: rows.into_iter().map(|row| row.sample).collect(),
                notes: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    profiles.sort_by(|left, right| {
        left.location
            .x
            .total_cmp(&right.location.x)
            .then(left.location.y.total_cmp(&right.location.y))
    });

    Ok(ParsedVelocityFunctions {
        profiles,
        sample_count,
    })
}

fn should_skip_velocity_control_profile_line(line: &str) -> bool {
    line.is_empty()
        || line.starts_with('#')
        || line.starts_with("This data contains")
        || line.starts_with("CDP-X")
        || line
            == "0        1         2         3         4         5         6         7         8"
        || line
            == "12345678901234567890123456789012345678901234567890123456789012345678901234567890"
}

fn parse_velocity_control_profile_numeric_columns(
    columns: &[&str],
    line_number: usize,
) -> Result<Option<Vec<f64>>, Box<dyn std::error::Error>> {
    let mut numeric_columns = Vec::with_capacity(columns.len());
    let mut saw_non_numeric = false;

    for column in columns {
        match column.parse::<f64>() {
            Ok(value) => numeric_columns.push(value),
            Err(_) => saw_non_numeric = true,
        }
    }

    if saw_non_numeric {
        if numeric_columns.is_empty() {
            return Ok(None);
        }
        if numeric_columns.len() < 3 {
            return Ok(None);
        }
        return Err(format!(
            "Velocity control profile row {} mixes numeric and non-numeric fields. Line/CDP keyed files need navigation mapping before import.",
            line_number
        )
        .into());
    }

    Ok(Some(numeric_columns))
}

fn try_load_velocity_navigation_index(
    input_path: &Path,
) -> Result<Option<VelocityNavigationIndex>, Box<dyn std::error::Error>> {
    for candidate in velocity_navigation_candidate_paths(input_path)? {
        let index = parse_velocity_navigation_index(&candidate)?;
        if !index.points_by_line.is_empty() {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn velocity_navigation_candidate_paths(
    input_path: &Path,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut candidates = Vec::<PathBuf>::new();
    let mut seen = HashSet::<PathBuf>::new();
    let Some(parent) = input_path.parent() else {
        return Ok(candidates);
    };

    collect_velocity_navigation_candidates(parent, &mut candidates, &mut seen)?;
    collect_velocity_navigation_candidates(&parent.join("navigation"), &mut candidates, &mut seen)?;
    Ok(candidates)
}

fn collect_velocity_navigation_candidates(
    directory: &Path,
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !directory.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(directory)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.ends_with(".hdr.sgn"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    entries.sort();

    for entry in entries {
        if seen.insert(entry.clone()) {
            candidates.push(entry);
        }
    }
    Ok(())
}

fn parse_velocity_navigation_index(
    input_path: &Path,
) -> Result<VelocityNavigationIndex, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(input_path)?;
    let mut index = VelocityNavigationIndex::default();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 4 {
            continue;
        }

        let raw_qc_line_key = columns[0];
        if !raw_qc_line_key.starts_with('Q') {
            continue;
        }
        let raw_cdp_and_latitude = columns[1];
        if raw_cdp_and_latitude.len() <= 10 {
            continue;
        }
        let raw_xy = columns[columns.len() - 2];
        if raw_xy.len() <= 9 {
            continue;
        }
        let raw_original_line_key = columns[columns.len() - 1];

        let cdp = raw_cdp_and_latitude[..raw_cdp_and_latitude.len() - 10]
            .parse::<i32>()
            .map_err(|error| {
                format!(
                    "Navigation row in '{}' has invalid CDP token '{}': {error}",
                    input_path.display(),
                    raw_cdp_and_latitude
                )
            })?;
        let x = raw_xy[..raw_xy.len() - 9].parse::<f64>().map_err(|error| {
            format!(
                "Navigation row in '{}' has invalid X token '{}': {error}",
                input_path.display(),
                raw_xy
            )
        })?;
        let y = raw_xy[raw_xy.len() - 9..].parse::<f64>().map_err(|error| {
            format!(
                "Navigation row in '{}' has invalid Y token '{}': {error}",
                input_path.display(),
                raw_xy
            )
        })?;

        let point = ProjectedPoint2 { x, y };
        index.insert(raw_qc_line_key, cdp, point.clone());
        index.insert(raw_qc_line_key.trim_start_matches('Q'), cdp, point.clone());
        index.insert(raw_original_line_key, cdp, point);
    }

    Ok(index)
}

fn parse_velocity_navigation_section_header(
    columns: &[&str],
    navigation_index: Option<&VelocityNavigationIndex>,
) -> Option<String> {
    let navigation_index = navigation_index?;
    if columns.len() != 1 || !columns[0].ends_with(':') {
        return None;
    }

    let line_key = normalize_velocity_navigation_line_key(columns[0]);
    navigation_index
        .contains_line(&line_key)
        .then_some(line_key)
}

fn try_parse_navigation_velocity_profile_row(
    columns: &[&str],
    line_number: usize,
    velocity_kind: VelocityQuantityKind,
    navigation_index: Option<&VelocityNavigationIndex>,
    active_navigation_line: Option<&str>,
) -> Result<Option<ParsedVelocityProfileRow>, Box<dyn std::error::Error>> {
    let Some(navigation_index) = navigation_index else {
        return Ok(None);
    };

    if columns.len() == 4 && !token_is_numeric(columns[0]) {
        return Ok(Some(parse_navigation_velocity_profile_row(
            columns[0],
            columns[1],
            columns[2],
            columns[3],
            line_number,
            velocity_kind,
            navigation_index,
        )?));
    }

    if columns.len() == 4
        && navigation_index.contains_line(columns[0])
        && token_is_integer_like(columns[1])?
    {
        return Ok(Some(parse_navigation_velocity_profile_row(
            columns[0],
            columns[1],
            columns[2],
            columns[3],
            line_number,
            velocity_kind,
            navigation_index,
        )?));
    }

    if columns.len() == 3
        && active_navigation_line.is_some()
        && columns.iter().all(|column| token_is_numeric(column))
    {
        return Ok(Some(parse_navigation_velocity_profile_row(
            active_navigation_line.expect("checked above"),
            columns[0],
            columns[1],
            columns[2],
            line_number,
            velocity_kind,
            navigation_index,
        )?));
    }

    Ok(None)
}

fn try_parse_nlog_ascii_velocity_profile_row(
    columns: &[&str],
    line_number: usize,
    velocity_kind: VelocityQuantityKind,
) -> Result<Option<ParsedVelocityProfileRow>, Box<dyn std::error::Error>> {
    if columns.len() < 8 || !columns[0].eq_ignore_ascii_case("V2") {
        return Ok(None);
    }
    if token_is_numeric(columns[1]) || !token_is_integer_like(columns[2])? {
        return Ok(None);
    }

    let numeric_tail = columns[columns.len() - 5..]
        .iter()
        .map(|value| {
            value.parse::<f64>().map_err(|error| {
                format!(
                    "Velocity control profile row {} has invalid NLOG ASCII field '{}': {error}",
                    line_number, value
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if numeric_tail.len() != 5 {
        return Ok(None);
    }

    let time_ms = numeric_tail[0] as f32;
    let velocity_m_per_s = numeric_tail[2] as f32;
    let x = numeric_tail[3];
    let y = numeric_tail[4];

    validate_velocity_control_profile_coordinate(x, "X", line_number)?;
    validate_velocity_control_profile_coordinate(y, "Y", line_number)?;
    validate_velocity_control_profile_time(time_ms, line_number)?;
    validate_velocity_control_profile_value(
        velocity_m_per_s,
        velocity_quantity_kind_label(velocity_kind),
        line_number,
    )?;

    let mut sample = VelocityControlProfileSample {
        time_ms,
        depth_m: None,
        vrms_m_per_s: None,
        vint_m_per_s: None,
        vavg_m_per_s: None,
    };
    match velocity_kind {
        VelocityQuantityKind::Interval => sample.vint_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Rms => sample.vrms_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Average => sample.vavg_m_per_s = Some(velocity_m_per_s),
    }

    Ok(Some(ParsedVelocityProfileRow { x, y, sample }))
}

fn try_parse_nlog_fixed_width_velocity_profile_row(
    raw_line: &str,
    line_number: usize,
    velocity_kind: VelocityQuantityKind,
) -> Result<Option<ParsedVelocityProfileRow>, Box<dyn std::error::Error>> {
    let line = raw_line.trim_end();
    if !line.starts_with("V2") || line.len() < 76 {
        return Ok(None);
    }

    let profile_key_a = line
        .get(2..7)
        .ok_or_else(|| format!("Velocity control profile row {} is too short.", line_number))?
        .trim();
    let profile_key_b = line
        .get(7..15)
        .ok_or_else(|| format!("Velocity control profile row {} is too short.", line_number))?
        .trim();
    let profile_key_c = line
        .get(15..21)
        .ok_or_else(|| format!("Velocity control profile row {} is too short.", line_number))?
        .trim();
    if profile_key_a.is_empty()
        || profile_key_b.is_empty()
        || profile_key_c.is_empty()
        || !profile_key_a
            .chars()
            .all(|character| character.is_ascii_digit())
        || !profile_key_b
            .chars()
            .all(|character| character.is_ascii_digit())
        || !profile_key_c
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Ok(None);
    }

    let time_ms = line[35..40].trim().parse::<f32>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid fixed-width time '{}': {error}",
            line_number,
            line[35..40].trim()
        )
    })?;
    let velocity_m_per_s = line[57..61].trim().parse::<f32>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid fixed-width velocity '{}': {error}",
            line_number,
            line[57..61].trim()
        )
    })?;
    let first_tail_coordinate = line[62..69].trim().parse::<f64>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid fixed-width coordinate '{}': {error}",
            line_number,
            line[62..69].trim()
        )
    })?;
    let second_tail_coordinate = line[70..76].trim().parse::<f64>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid fixed-width coordinate '{}': {error}",
            line_number,
            line[70..76].trim()
        )
    })?;

    // NLOG's fixed-width 3D V2/ESSOV2XY rows store projected coordinates as northing then easting.
    let x = second_tail_coordinate;
    let y = first_tail_coordinate;

    validate_velocity_control_profile_coordinate(x, "X", line_number)?;
    validate_velocity_control_profile_coordinate(y, "Y", line_number)?;
    validate_velocity_control_profile_time(time_ms, line_number)?;
    validate_velocity_control_profile_value(
        velocity_m_per_s,
        velocity_quantity_kind_label(velocity_kind),
        line_number,
    )?;

    let mut sample = VelocityControlProfileSample {
        time_ms,
        depth_m: None,
        vrms_m_per_s: None,
        vint_m_per_s: None,
        vavg_m_per_s: None,
    };
    match velocity_kind {
        VelocityQuantityKind::Interval => sample.vint_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Rms => sample.vrms_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Average => sample.vavg_m_per_s = Some(velocity_m_per_s),
    }

    Ok(Some(ParsedVelocityProfileRow { x, y, sample }))
}

fn parse_navigation_velocity_profile_row(
    line_key: &str,
    cdp_token: &str,
    time_token: &str,
    velocity_token: &str,
    line_number: usize,
    velocity_kind: VelocityQuantityKind,
    navigation_index: &VelocityNavigationIndex,
) -> Result<ParsedVelocityProfileRow, Box<dyn std::error::Error>> {
    let cdp = parse_velocity_navigation_cdp(cdp_token, line_number)?;
    let point = navigation_index.lookup(line_key, cdp).ok_or_else(|| {
        format!(
            "Velocity control profile row {} references line '{}' CDP {}, but the navigation sidecar does not contain that location.",
            line_number, line_key, cdp
        )
    })?;
    let time_ms = time_token.parse::<f32>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid time '{}': {error}",
            line_number, time_token
        )
    })?;
    let velocity_m_per_s = velocity_token.parse::<f32>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid velocity '{}': {error}",
            line_number, velocity_token
        )
    })?;

    validate_velocity_control_profile_coordinate(point.x, "X", line_number)?;
    validate_velocity_control_profile_coordinate(point.y, "Y", line_number)?;
    validate_velocity_control_profile_time(time_ms, line_number)?;
    validate_velocity_control_profile_value(
        velocity_m_per_s,
        velocity_quantity_kind_label(velocity_kind),
        line_number,
    )?;

    let mut sample = VelocityControlProfileSample {
        time_ms,
        depth_m: None,
        vrms_m_per_s: None,
        vint_m_per_s: None,
        vavg_m_per_s: None,
    };
    match velocity_kind {
        VelocityQuantityKind::Interval => sample.vint_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Rms => sample.vrms_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Average => sample.vavg_m_per_s = Some(velocity_m_per_s),
    }

    Ok(ParsedVelocityProfileRow {
        x: point.x,
        y: point.y,
        sample,
    })
}

fn parse_velocity_navigation_cdp(
    value: &str,
    line_number: usize,
) -> Result<i32, Box<dyn std::error::Error>> {
    let parsed = value.parse::<f64>().map_err(|error| {
        format!(
            "Velocity control profile row {} has invalid CDP '{}': {error}",
            line_number, value
        )
    })?;
    let rounded = parsed.round();
    if !parsed.is_finite() || parsed < 0.0 || (parsed - rounded).abs() > 1e-3 {
        return Err(format!(
            "Velocity control profile row {} has non-integer CDP '{}'.",
            line_number, value
        )
        .into());
    }
    Ok(rounded as i32)
}

fn normalize_velocity_navigation_line_key(line_key: &str) -> String {
    line_key
        .trim()
        .trim_end_matches(':')
        .trim_start_matches('Q')
        .to_ascii_uppercase()
}

fn token_is_numeric(value: &str) -> bool {
    value.parse::<f64>().is_ok()
}

fn token_is_integer_like(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let parsed = value.parse::<f64>()?;
    Ok(parsed.is_finite() && (parsed - parsed.round()).abs() <= 1e-3)
}

impl VelocityNavigationIndex {
    fn insert(&mut self, line_key: &str, cdp: i32, point: ProjectedPoint2) {
        self.points_by_line
            .entry(normalize_velocity_navigation_line_key(line_key))
            .or_default()
            .insert(cdp, point);
    }

    fn contains_line(&self, line_key: &str) -> bool {
        self.points_by_line
            .contains_key(&normalize_velocity_navigation_line_key(line_key))
    }

    fn lookup(&self, line_key: &str, cdp: i32) -> Option<ProjectedPoint2> {
        self.points_by_line
            .get(&normalize_velocity_navigation_line_key(line_key))
            .and_then(|points| points.get(&cdp))
            .cloned()
    }
}

fn parse_full_velocity_profile_row(
    numeric_columns: &[f64],
    line_number: usize,
) -> Result<ParsedVelocityProfileRow, Box<dyn std::error::Error>> {
    let x = numeric_columns[0];
    let y = numeric_columns[1];
    let time_ms = numeric_columns[2] as f32;
    let vrms_m_per_s = numeric_columns[3] as f32;
    let vint_m_per_s = numeric_columns[4] as f32;
    let vavg_m_per_s = numeric_columns[5] as f32;
    let depth_m = numeric_columns[6] as f32;

    validate_velocity_control_profile_coordinate(x, "X", line_number)?;
    validate_velocity_control_profile_coordinate(y, "Y", line_number)?;
    validate_velocity_control_profile_time(time_ms, line_number)?;
    validate_velocity_control_profile_value(vrms_m_per_s, "Vrms", line_number)?;
    validate_velocity_control_profile_value(vint_m_per_s, "Vint", line_number)?;
    validate_velocity_control_profile_value(vavg_m_per_s, "Vavg", line_number)?;
    validate_velocity_control_profile_depth(depth_m, line_number)?;

    Ok(ParsedVelocityProfileRow {
        x,
        y,
        sample: VelocityControlProfileSample {
            time_ms,
            depth_m: Some(depth_m),
            vrms_m_per_s: Some(vrms_m_per_s),
            vint_m_per_s: Some(vint_m_per_s),
            vavg_m_per_s: Some(vavg_m_per_s),
        },
    })
}

fn parse_single_velocity_profile_row(
    numeric_columns: &[f64],
    line_number: usize,
    velocity_kind: VelocityQuantityKind,
) -> Result<ParsedVelocityProfileRow, Box<dyn std::error::Error>> {
    let x = numeric_columns[0];
    let y = numeric_columns[1];
    let time_ms = numeric_columns[2] as f32;
    let velocity_m_per_s = numeric_columns[3] as f32;

    validate_velocity_control_profile_coordinate(x, "X", line_number)?;
    validate_velocity_control_profile_coordinate(y, "Y", line_number)?;
    validate_velocity_control_profile_time(time_ms, line_number)?;
    validate_velocity_control_profile_value(
        velocity_m_per_s,
        velocity_quantity_kind_label(velocity_kind),
        line_number,
    )?;

    let mut sample = VelocityControlProfileSample {
        time_ms,
        depth_m: None,
        vrms_m_per_s: None,
        vint_m_per_s: None,
        vavg_m_per_s: None,
    };
    match velocity_kind {
        VelocityQuantityKind::Interval => sample.vint_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Rms => sample.vrms_m_per_s = Some(velocity_m_per_s),
        VelocityQuantityKind::Average => sample.vavg_m_per_s = Some(velocity_m_per_s),
    }

    Ok(ParsedVelocityProfileRow { x, y, sample })
}

fn validate_velocity_control_profile_coordinate(
    value: f64,
    label: &str,
    line_number: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if !value.is_finite() {
        return Err(format!(
            "Velocity control profile row {} has invalid {label} coordinate {}.",
            line_number, value
        )
        .into());
    }
    Ok(())
}

fn validate_velocity_control_profile_time(
    time_ms: f32,
    line_number: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if !time_ms.is_finite() || time_ms < 0.0 {
        return Err(format!(
            "Velocity control profile row {} has invalid time {}.",
            line_number, time_ms
        )
        .into());
    }
    Ok(())
}

fn validate_velocity_control_profile_value(
    value: f32,
    label: &str,
    line_number: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!(
            "Velocity control profile row {} has invalid {label} value {}.",
            line_number, value
        )
        .into());
    }
    Ok(())
}

fn validate_velocity_control_profile_depth(
    depth_m: f32,
    line_number: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if !depth_m.is_finite() || depth_m < 0.0 {
        return Err(format!(
            "Velocity control profile row {} has invalid depth {}.",
            line_number, depth_m
        )
        .into());
    }
    Ok(())
}

fn velocity_quantity_kind_label(kind: VelocityQuantityKind) -> &'static str {
    match kind {
        VelocityQuantityKind::Interval => "Interval",
        VelocityQuantityKind::Rms => "RMS",
        VelocityQuantityKind::Average => "Average",
    }
}

fn velocity_quantity_kind_slug(kind: VelocityQuantityKind) -> &'static str {
    match kind {
        VelocityQuantityKind::Interval => "vint",
        VelocityQuantityKind::Rms => "vrms",
        VelocityQuantityKind::Average => "vavg",
    }
}

fn file_stem_from_path(file_path: &str) -> String {
    Path::new(file_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| "velocity-functions".to_string())
}

fn display_name_from_stem(stem: &str) -> String {
    stem.replace('_', " ").trim().to_string()
}

pub fn load_gather(
    store_path: String,
    request: GatherRequest,
) -> Result<GatherView, Box<dyn std::error::Error>> {
    let handle = open_prestack_store(&store_path)?;
    ensure_prestack_dataset_matches(&handle, &request.dataset_id.0)?;
    Ok(prestack_gather_view(&store_path, &request)?)
}

pub fn preview_processing(
    request: PreviewTraceLocalProcessingRequest,
) -> Result<PreviewTraceLocalProcessingResponse, Box<dyn std::error::Error>> {
    let handle = open_store(&request.store_path)?;
    ensure_dataset_matches(&handle, &request.section.dataset_id.0)?;
    let section = preview_processing_section_view(
        &request.store_path,
        request.section.axis,
        request.section.index,
        &request.pipeline,
    )?;
    Ok(PreviewTraceLocalProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        preview: PreviewView {
            section,
            processing_label: preview_processing_label(&request.pipeline),
            preview_ready: true,
        },
        pipeline: request.pipeline,
    })
}

pub fn preview_subvolume_processing(
    request: PreviewSubvolumeProcessingRequest,
) -> Result<PreviewSubvolumeProcessingResponse, Box<dyn std::error::Error>> {
    let handle = open_store(&request.store_path)?;
    ensure_dataset_matches(&handle, &request.section.dataset_id.0)?;
    let section = preview_subvolume_processing_section_view(
        &request.store_path,
        request.section.axis,
        request.section.index,
        &request.pipeline,
    )?;
    Ok(PreviewSubvolumeProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        preview: PreviewView {
            section,
            processing_label: preview_subvolume_processing_label(&request.pipeline),
            preview_ready: true,
        },
        pipeline: request.pipeline,
    })
}

pub fn preview_post_stack_neighborhood_processing(
    request: PreviewPostStackNeighborhoodProcessingRequest,
) -> Result<PreviewPostStackNeighborhoodProcessingResponse, Box<dyn std::error::Error>> {
    let handle = open_store(&request.store_path)?;
    ensure_dataset_matches(&handle, &request.section.dataset_id.0)?;
    let section = preview_post_stack_neighborhood_processing_section_view(
        &request.store_path,
        request.section.axis,
        request.section.index,
        &request.pipeline,
    )?;
    Ok(PreviewPostStackNeighborhoodProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        preview: PreviewView {
            section,
            processing_label: preview_post_stack_neighborhood_processing_label(&request.pipeline),
            preview_ready: true,
        },
        pipeline: request.pipeline,
    })
}

pub fn preview_gather_processing(
    request: PreviewGatherProcessingRequest,
) -> Result<PreviewGatherProcessingResponse, Box<dyn std::error::Error>> {
    let handle = open_prestack_store(&request.store_path)?;
    ensure_prestack_dataset_matches(&handle, &request.gather.dataset_id.0)?;
    let preview =
        preview_gather_processing_view(&request.store_path, &request.gather, &request.pipeline)?;
    Ok(PreviewGatherProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        preview,
        pipeline: request.pipeline,
    })
}

pub fn apply_processing(
    request: RunTraceLocalProcessingRequest,
) -> Result<DatasetSummary, Box<dyn std::error::Error>> {
    let pipeline = request.pipeline;
    let handle = open_store(&request.store_path)?;
    let source_shape = handle.manifest.volume.shape;
    let source_chunk_shape = handle.manifest.tile_shape;
    let output_store = request
        .output_store_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_store_path(&request.store_path, &pipeline));
    prepare_processing_output_store(&output_store, request.overwrite_existing)?;
    let execution_plan = build_execution_plan(&PlanProcessingRequest {
        store_path: request.store_path.clone(),
        layout: SeismicLayout::PostStack3D,
        source_shape: Some(source_shape),
        source_chunk_shape: Some(source_chunk_shape),
        pipeline: ProcessingPipelineSpec::TraceLocal {
            pipeline: pipeline.clone(),
        },
        output_store_path: Some(output_store.to_string_lossy().into_owned()),
        planning_mode: PlanningMode::ForegroundMaterialize,
        max_active_partitions: None,
    })
    .map_err(|error| format!("failed to build execution plan: {error}"))?;
    let materialize_options = resolve_trace_local_materialize_options(
        Some(&execution_plan),
        source_chunk_shape,
        false,
        None,
        1,
        None,
        1,
    )
    .options;
    let derived = materialize_processing_volume(
        &request.store_path,
        &output_store,
        &pipeline,
        materialize_options,
    )?;
    Ok(DatasetSummary {
        store_path: derived.root.to_string_lossy().into_owned(),
        descriptor: handle_for_summary(&derived)?,
    })
}

pub fn apply_subvolume_processing(
    request: RunSubvolumeProcessingRequest,
) -> Result<DatasetSummary, Box<dyn std::error::Error>> {
    let pipeline = request.pipeline;
    let output_store = request
        .output_store_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_subvolume_output_store_path(&request.store_path, &pipeline));
    prepare_processing_output_store(&output_store, request.overwrite_existing)?;
    let materialize_options = materialize_options_for_store(&request.store_path)?;
    let derived = materialize_subvolume_processing_volume(
        &request.store_path,
        &output_store,
        &pipeline,
        materialize_options,
    )?;
    Ok(DatasetSummary {
        store_path: derived.root.to_string_lossy().into_owned(),
        descriptor: handle_for_summary(&derived)?,
    })
}

pub fn apply_post_stack_neighborhood_processing(
    request: RunPostStackNeighborhoodProcessingRequest,
) -> Result<DatasetSummary, Box<dyn std::error::Error>> {
    let pipeline = request.pipeline;
    let output_store = request
        .output_store_path
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            default_post_stack_neighborhood_output_store_path(&request.store_path, &pipeline)
        });
    prepare_processing_output_store(&output_store, request.overwrite_existing)?;
    let materialize_options = materialize_options_for_store(&request.store_path)?;
    let derived = materialize_post_stack_neighborhood_processing_volume(
        &request.store_path,
        &output_store,
        &pipeline,
        materialize_options,
    )?;
    Ok(DatasetSummary {
        store_path: derived.root.to_string_lossy().into_owned(),
        descriptor: handle_for_summary(&derived)?,
    })
}

pub fn apply_gather_processing(
    request: RunGatherProcessingRequest,
) -> Result<DatasetSummary, Box<dyn std::error::Error>> {
    let pipeline = request.pipeline;
    let output_store = request
        .output_store_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_gather_output_store_path(&request.store_path, &pipeline));
    prepare_processing_output_store(&output_store, request.overwrite_existing)?;
    let derived =
        materialize_gather_processing_store(&request.store_path, &output_store, &pipeline)?;
    dataset_summary_for_path(&derived.root)
}

pub fn amplitude_spectrum(
    request: AmplitudeSpectrumRequest,
) -> Result<AmplitudeSpectrumResponse, Box<dyn std::error::Error>> {
    let handle = open_store(&request.store_path)?;
    ensure_dataset_matches(&handle, &request.section.dataset_id.0)?;
    let curve = amplitude_spectrum_from_store(
        &request.store_path,
        request.section.axis,
        request.section.index,
        request
            .pipeline
            .as_ref()
            .map(|pipeline| pipeline.operations().cloned().collect::<Vec<_>>())
            .as_deref(),
        &request.selection,
    )?;

    Ok(AmplitudeSpectrumResponse {
        schema_version: IPC_SCHEMA_VERSION,
        section: request.section,
        selection: request.selection,
        sample_interval_ms: handle.volume_descriptor().sample_interval_ms,
        curve,
        processing_label: request.pipeline.as_ref().map(preview_processing_label),
    })
}

pub fn run_velocity_scan(
    request: VelocityScanRequest,
) -> Result<VelocityScanResponse, Box<dyn std::error::Error>> {
    let handle = open_prestack_store(&request.store_path)?;
    ensure_prestack_dataset_matches(&handle, &request.gather.dataset_id.0)?;
    Ok(velocity_scan(request)?)
}

pub fn default_output_store_path(
    input_store_path: impl AsRef<Path>,
    pipeline: &TraceLocalProcessingPipeline,
) -> PathBuf {
    let input_store_path = input_store_path.as_ref();
    let parent = input_store_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_store_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("dataset");
    let suffix = pipeline_slug(pipeline);
    parent.join(format!("{stem}.{suffix}.tbvol"))
}

pub fn default_export_segy_path(input_store_path: impl AsRef<Path>) -> PathBuf {
    let input_store_path = input_store_path.as_ref();
    let parent = input_store_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_store_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("dataset");
    parent.join(format!("{stem}.export.sgy"))
}

pub fn default_export_zarr_path(input_store_path: impl AsRef<Path>) -> PathBuf {
    let input_store_path = input_store_path.as_ref();
    let parent = input_store_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_store_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("dataset");
    parent.join(format!("{stem}.export.zarr"))
}

pub fn default_subvolume_output_store_path(
    input_store_path: impl AsRef<Path>,
    pipeline: &SubvolumeProcessingPipeline,
) -> PathBuf {
    let input_store_path = input_store_path.as_ref();
    let parent = input_store_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_store_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("dataset");
    let suffix = subvolume_pipeline_slug(pipeline);
    parent.join(format!("{stem}.{suffix}.tbvol"))
}

pub fn default_post_stack_neighborhood_output_store_path(
    input_store_path: impl AsRef<Path>,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> PathBuf {
    let input_store_path = input_store_path.as_ref();
    let parent = input_store_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_store_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("dataset");
    let suffix = post_stack_neighborhood_pipeline_slug(pipeline);
    parent.join(format!("{stem}.{suffix}.tbvol"))
}

pub fn default_gather_output_store_path(
    input_store_path: impl AsRef<Path>,
    pipeline: &GatherProcessingPipeline,
) -> PathBuf {
    let input_store_path = input_store_path.as_ref();
    let parent = input_store_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_store_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("dataset");
    let suffix = gather_pipeline_slug(pipeline);
    parent.join(format!("{stem}.{suffix}.tbgath"))
}

fn dataset_summary_for_path(
    store_path: impl AsRef<Path>,
) -> Result<DatasetSummary, Box<dyn std::error::Error>> {
    let store_path = store_path.as_ref();
    let descriptor = match open_store(store_path) {
        Ok(_) => describe_store(store_path)?,
        Err(poststack_error) => match open_prestack_store(store_path) {
            Ok(_) => describe_prestack_store(store_path)?,
            Err(prestack_error) => {
                return Err(format!(
                    "failed to open dataset store as tbvol ({poststack_error}) or tbgath ({prestack_error})"
                )
                .into())
            }
        },
    };
    Ok(DatasetSummary {
        store_path: store_path.to_string_lossy().into_owned(),
        descriptor,
    })
}

fn prepare_export_output_path(
    input_store_path: &Path,
    output_path: &Path,
    overwrite_existing: bool,
    output_label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_store_path = input_store_path
        .canonicalize()
        .unwrap_or_else(|_| input_store_path.to_path_buf());
    let output_path = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.to_path_buf());

    if input_store_path == output_path {
        return Err(
            format!("Output {output_label} path cannot overwrite the input tbvol store.").into(),
        );
    }

    if !overwrite_existing || !output_path.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(&output_path)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(&output_path)?;
        return Ok(());
    }

    fs::remove_file(&output_path)?;
    Ok(())
}

fn suggested_action(action: seis_runtime::PreflightAction) -> SuggestedImportAction {
    match action {
        seis_runtime::PreflightAction::DirectDenseIngest => {
            SuggestedImportAction::DirectDenseIngest
        }
        seis_runtime::PreflightAction::RegularizeSparseSurvey => {
            SuggestedImportAction::RegularizeSparseSurvey
        }
        seis_runtime::PreflightAction::ReviewGeometryMapping => {
            SuggestedImportAction::ReviewGeometryMapping
        }
        seis_runtime::PreflightAction::UnsupportedInV1 => SuggestedImportAction::UnsupportedInV1,
    }
}

fn preflight_response(
    input_path: String,
    preflight: &seis_runtime::SurveyPreflight,
    suggested_geometry_override: Option<SegyGeometryOverride>,
    geometry_candidates: Vec<SegyGeometryCandidate>,
) -> SurveyPreflightResponse {
    let mut notes = preflight.notes.clone();
    if !geometry_candidates.is_empty() {
        notes.push("TraceBoost found one or more alternate header mappings that may allow import without manual SEG-Y repair.".to_string());
    }
    if suggested_geometry_override.is_some() {
        notes.push(
            "A single high-confidence alternate mapping was detected; review it before import."
                .to_string(),
        );
    }

    SurveyPreflightResponse {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
        trace_count: preflight.inspection.trace_count,
        samples_per_trace: preflight.inspection.samples_per_trace as usize,
        sample_data_fidelity: preflight.sample_data_fidelity.clone(),
        classification: preflight.geometry.classification.clone(),
        stacking_state: preflight.geometry.stacking_state.clone(),
        organization: preflight.geometry.organization.clone(),
        layout: preflight.geometry.layout.clone(),
        gather_axis_kind: preflight.geometry.gather_axis_kind.clone(),
        suggested_action: suggested_action(preflight.recommended_action),
        observed_trace_count: preflight.geometry.observed_trace_count,
        expected_trace_count: preflight.geometry.expected_trace_count,
        completeness_ratio: preflight.geometry.completeness_ratio,
        resolved_geometry: geometry_override_from_preflight(preflight),
        suggested_geometry_override,
        geometry_candidates,
        notes,
    }
}

fn discover_geometry_candidates(
    input_path: &str,
    baseline: &seis_runtime::SurveyPreflight,
) -> Vec<SegyGeometryCandidate> {
    let baseline_geometry = geometry_override_from_preflight(baseline);
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for spec in GEOMETRY_CANDIDATE_SPECS {
        let geometry = SegyGeometryOverride {
            inline_3d: Some(SegyHeaderField {
                start_byte: spec.inline.0,
                value_type: spec.inline.1.clone(),
            }),
            crossline_3d: Some(SegyHeaderField {
                start_byte: spec.crossline.0,
                value_type: spec.crossline.1.clone(),
            }),
            third_axis: None,
        };
        if geometry == baseline_geometry {
            continue;
        }

        let preflight = match preflight_segy(
            input_path,
            &ingest_options_from_geometry_override(Some(&geometry)),
        ) {
            Ok(preflight) => preflight,
            Err(_) => continue,
        };

        let action = suggested_action(preflight.recommended_action);
        if !matches!(
            action,
            SuggestedImportAction::DirectDenseIngest
                | SuggestedImportAction::RegularizeSparseSurvey
        ) {
            continue;
        }
        if !is_plausible_geometry_candidate(&preflight) {
            continue;
        }

        let geometry_key = (
            preflight.geometry.inline_field.start_byte,
            preflight.geometry.inline_field.value_type.clone(),
            preflight.geometry.crossline_field.start_byte,
            preflight.geometry.crossline_field.value_type.clone(),
            preflight
                .geometry
                .third_axis_field
                .as_ref()
                .map(|field| (field.start_byte, field.value_type.clone())),
        );
        if !seen.insert(geometry_key) {
            continue;
        }

        candidates.push(SegyGeometryCandidate {
            label: spec.label.to_string(),
            geometry: geometry_override_from_preflight(&preflight),
            classification: preflight.geometry.classification.clone(),
            stacking_state: preflight.geometry.stacking_state.clone(),
            organization: preflight.geometry.organization.clone(),
            layout: preflight.geometry.layout.clone(),
            gather_axis_kind: preflight.geometry.gather_axis_kind.clone(),
            suggested_action: action,
            inline_count: preflight.geometry.inline_count,
            crossline_count: preflight.geometry.crossline_count,
            third_axis_count: preflight.geometry.third_axis_count,
            observed_trace_count: preflight.geometry.observed_trace_count,
            expected_trace_count: preflight.geometry.expected_trace_count,
            completeness_ratio: preflight.geometry.completeness_ratio,
            auto_selectable: is_high_confidence_dense_candidate(&preflight),
            notes: preflight.notes.clone(),
        });
    }

    candidates.sort_by_key(|candidate| {
        (
            Reverse(geometry_candidate_rank(candidate)),
            Reverse(
                candidate
                    .inline_count
                    .saturating_mul(candidate.crossline_count),
            ),
            candidate.label.clone(),
        )
    });
    candidates
}

fn geometry_candidate_rank(candidate: &SegyGeometryCandidate) -> usize {
    let action_score = match candidate.suggested_action {
        SuggestedImportAction::DirectDenseIngest => 3,
        SuggestedImportAction::RegularizeSparseSurvey => 2,
        SuggestedImportAction::ReviewGeometryMapping => 1,
        SuggestedImportAction::UnsupportedInV1 => 0,
    };
    let auto_score = usize::from(candidate.auto_selectable);
    let axis_balance_score = candidate
        .inline_count
        .min(candidate.crossline_count)
        .min(10_000);
    (action_score * 10_000)
        + (auto_score * 1_000)
        + axis_balance_score
        + ((candidate.completeness_ratio * 100.0).round() as usize)
}

fn preferred_geometry_override(
    candidates: &[SegyGeometryCandidate],
) -> Option<SegyGeometryOverride> {
    let mut auto_candidates = candidates
        .iter()
        .filter(|candidate| candidate.auto_selectable);
    let first = auto_candidates.next()?;
    if auto_candidates.next().is_some() {
        return None;
    }
    Some(first.geometry.clone())
}

fn is_high_confidence_dense_candidate(preflight: &seis_runtime::SurveyPreflight) -> bool {
    matches!(
        preflight.recommended_action,
        seis_runtime::PreflightAction::DirectDenseIngest
    ) && preflight.geometry.observed_trace_count == preflight.geometry.expected_trace_count
        && preflight.geometry.inline_count > 1
        && preflight.geometry.crossline_count > 1
}

fn is_plausible_geometry_candidate(preflight: &seis_runtime::SurveyPreflight) -> bool {
    preflight.geometry.inline_count > 1 && preflight.geometry.crossline_count > 1
}

fn ingest_options_from_geometry_override(
    geometry_override: Option<&SegyGeometryOverride>,
) -> IngestOptions {
    IngestOptions {
        geometry: geometry_override_to_seis_options(geometry_override),
        ..IngestOptions::default()
    }
}

fn geometry_override_to_seis_options(
    geometry_override: Option<&SegyGeometryOverride>,
) -> SeisGeometryOptions {
    let mut geometry = SeisGeometryOptions::default();
    if let Some(geometry_override) = geometry_override {
        geometry.header_mapping.inline_3d = geometry_override
            .inline_3d
            .as_ref()
            .map(|field| contract_header_field_to_runtime("INLINE_3D", field));
        geometry.header_mapping.crossline_3d = geometry_override
            .crossline_3d
            .as_ref()
            .map(|field| contract_header_field_to_runtime("CROSSLINE_3D", field));
        geometry.third_axis_field = geometry_override
            .third_axis
            .as_ref()
            .map(|field| contract_header_field_to_runtime("THIRD_AXIS", field));
    }
    geometry
}

fn contract_header_field_to_runtime(name: &'static str, field: &SegyHeaderField) -> HeaderField {
    match field.value_type {
        SegyHeaderValueType::I16 => HeaderField::new_i16(name, field.start_byte),
        SegyHeaderValueType::I32 => HeaderField::new_i32(name, field.start_byte),
    }
}

fn geometry_override_from_preflight(
    preflight: &seis_runtime::SurveyPreflight,
) -> SegyGeometryOverride {
    SegyGeometryOverride {
        inline_3d: Some(contract_header_field_from_spec(
            &preflight.geometry.inline_field,
        )),
        crossline_3d: Some(contract_header_field_from_spec(
            &preflight.geometry.crossline_field,
        )),
        third_axis: preflight
            .geometry
            .third_axis_field
            .as_ref()
            .map(contract_header_field_from_spec),
    }
}

fn contract_header_field_from_spec(spec: &seis_runtime::HeaderFieldSpec) -> SegyHeaderField {
    SegyHeaderField {
        start_byte: spec.start_byte,
        value_type: contract_header_value_type(&spec.value_type),
    }
}

fn contract_header_value_type(value_type: &str) -> SegyHeaderValueType {
    match value_type {
        "I16" => SegyHeaderValueType::I16,
        _ => SegyHeaderValueType::I32,
    }
}

fn handle_for_summary(
    handle: &seis_runtime::StoreHandle,
) -> Result<seis_runtime::VolumeDescriptor, Box<dyn std::error::Error>> {
    Ok(describe_store(&handle.root)?)
}

fn ensure_dataset_matches(
    handle: &seis_runtime::StoreHandle,
    expected_dataset_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = handle.dataset_id().0;
    if actual != expected_dataset_id {
        return Err(format!(
            "Section request dataset mismatch: expected {expected_dataset_id}, found {actual}"
        )
        .into());
    }
    Ok(())
}

fn ensure_prestack_dataset_matches(
    handle: &seis_runtime::PrestackStoreHandle,
    expected_dataset_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = handle.dataset_id().0;
    if actual != expected_dataset_id {
        return Err(format!(
            "Gather request dataset mismatch: expected {expected_dataset_id}, found {actual}"
        )
        .into());
    }
    Ok(())
}

pub fn preview_processing_label(pipeline: &TraceLocalProcessingPipeline) -> String {
    pipeline
        .name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| pipeline_slug(pipeline))
}

pub fn preview_subvolume_processing_label(pipeline: &SubvolumeProcessingPipeline) -> String {
    pipeline
        .name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| subvolume_pipeline_slug(pipeline))
}

pub fn preview_post_stack_neighborhood_processing_label(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> String {
    pipeline
        .name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| post_stack_neighborhood_pipeline_slug(pipeline))
}

fn pipeline_slug(pipeline: &TraceLocalProcessingPipeline) -> String {
    let mut parts = Vec::with_capacity(pipeline.operation_count());
    for operation in pipeline.operations() {
        let label = match operation {
            seis_runtime::ProcessingOperation::AmplitudeScalar { factor } => {
                format!("amplitude-scalar-{}", format_factor(*factor))
            }
            seis_runtime::ProcessingOperation::TraceRmsNormalize => {
                "trace-rms-normalize".to_string()
            }
            seis_runtime::ProcessingOperation::AgcRms { window_ms } => {
                format!("agc-rms-{}", format_factor(*window_ms))
            }
            seis_runtime::ProcessingOperation::PhaseRotation { angle_degrees } => {
                format!("phase-rotation-{}", format_factor(*angle_degrees))
            }
            seis_runtime::ProcessingOperation::Envelope => "envelope".to_string(),
            seis_runtime::ProcessingOperation::InstantaneousPhase => {
                "instantaneous-phase".to_string()
            }
            seis_runtime::ProcessingOperation::InstantaneousFrequency => {
                "instantaneous-frequency".to_string()
            }
            seis_runtime::ProcessingOperation::Sweetness => "sweetness".to_string(),
            seis_runtime::ProcessingOperation::LowpassFilter { f3_hz, f4_hz, .. } => format!(
                "lowpass-{}-{}",
                format_factor(*f3_hz),
                format_factor(*f4_hz)
            ),
            seis_runtime::ProcessingOperation::HighpassFilter { f1_hz, f2_hz, .. } => format!(
                "highpass-{}-{}",
                format_factor(*f1_hz),
                format_factor(*f2_hz)
            ),
            seis_runtime::ProcessingOperation::BandpassFilter {
                f1_hz,
                f2_hz,
                f3_hz,
                f4_hz,
                ..
            } => format!(
                "bandpass-{}-{}-{}-{}",
                format_factor(*f1_hz),
                format_factor(*f2_hz),
                format_factor(*f3_hz),
                format_factor(*f4_hz)
            ),
            seis_runtime::ProcessingOperation::VolumeArithmetic {
                operator,
                secondary_store_path,
            } => format!(
                "volume-{}-{}",
                volume_arithmetic_operator_slug(*operator),
                store_path_slug(secondary_store_path)
            ),
        };
        parts.push(label);
    }
    if parts.is_empty() {
        "pipeline".to_string()
    } else {
        parts.join("__")
    }
}

fn subvolume_pipeline_slug(pipeline: &SubvolumeProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return name.replace(' ', "-").to_ascii_lowercase();
    }

    let mut parts = Vec::new();
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        parts.push(pipeline_slug(trace_local_pipeline));
    }
    parts.push(format!(
        "crop-il-{}-{}-xl-{}-{}-z-{}-{}",
        pipeline.crop.inline_min,
        pipeline.crop.inline_max,
        pipeline.crop.xline_min,
        pipeline.crop.xline_max,
        format_factor(pipeline.crop.z_min_ms),
        format_factor(pipeline.crop.z_max_ms)
    ));
    parts.join("__")
}

fn post_stack_neighborhood_pipeline_slug(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return name.replace(' ', "-").to_ascii_lowercase();
    }

    let mut parts = Vec::new();
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        parts.push(pipeline_slug(trace_local_pipeline));
    }
    for operation in &pipeline.operations {
        let label = match operation {
            seis_runtime::PostStackNeighborhoodProcessingOperation::Similarity { window } => {
                format!(
                    "similarity-g{}-il{}-xl{}",
                    format_factor(window.gate_ms),
                    window.inline_stepout,
                    window.xline_stepout
                )
            }
            seis_runtime::PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
                window,
                statistic,
            } => {
                format!(
                    "local-volume-stats-{}-g{}-il{}-xl{}",
                    local_volume_statistic_slug(*statistic),
                    format_factor(window.gate_ms),
                    window.inline_stepout,
                    window.xline_stepout
                )
            }
            seis_runtime::PostStackNeighborhoodProcessingOperation::Dip { window, output } => {
                format!(
                    "dip-{}-g{}-il{}-xl{}",
                    neighborhood_dip_output_slug(*output),
                    format_factor(window.gate_ms),
                    window.inline_stepout,
                    window.xline_stepout
                )
            }
        };
        parts.push(label);
    }
    if parts.is_empty() {
        "post-stack-neighborhood".to_string()
    } else {
        parts.join("__")
    }
}

fn local_volume_statistic_slug(statistic: seis_runtime::LocalVolumeStatistic) -> &'static str {
    match statistic {
        seis_runtime::LocalVolumeStatistic::Mean => "mean",
        seis_runtime::LocalVolumeStatistic::Rms => "rms",
        seis_runtime::LocalVolumeStatistic::Variance => "variance",
        seis_runtime::LocalVolumeStatistic::Minimum => "minimum",
        seis_runtime::LocalVolumeStatistic::Maximum => "maximum",
    }
}

fn neighborhood_dip_output_slug(output: NeighborhoodDipOutput) -> &'static str {
    match output {
        NeighborhoodDipOutput::Inline => "inline",
        NeighborhoodDipOutput::Xline => "xline",
        NeighborhoodDipOutput::Azimuth => "azimuth",
        NeighborhoodDipOutput::AbsDip => "abs-dip",
    }
}

fn gather_pipeline_slug(pipeline: &GatherProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return name.replace(' ', "-").to_ascii_lowercase();
    }

    let mut parts = Vec::new();
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        parts.push(pipeline_slug(trace_local_pipeline));
    }
    for operation in &pipeline.operations {
        let label = match operation {
            seis_runtime::GatherProcessingOperation::NmoCorrection {
                velocity_model,
                interpolation,
            } => format!(
                "nmo-{}-{}",
                velocity_model_slug(velocity_model),
                interpolation_slug(*interpolation)
            ),
            seis_runtime::GatherProcessingOperation::StretchMute {
                velocity_model,
                max_stretch_ratio,
            } => format!(
                "stretch-mute-{}-{}",
                velocity_model_slug(velocity_model),
                format_factor(*max_stretch_ratio)
            ),
            seis_runtime::GatherProcessingOperation::OffsetMute {
                min_offset,
                max_offset,
            } => format!(
                "offset-mute-{}-{}",
                optional_factor_slug(*min_offset),
                optional_factor_slug(*max_offset)
            ),
        };
        parts.push(label);
    }
    if parts.is_empty() {
        "gather-processing".to_string()
    } else {
        parts.join("__")
    }
}

fn interpolation_slug(mode: GatherInterpolationMode) -> &'static str {
    match mode {
        GatherInterpolationMode::Linear => "linear",
    }
}

fn velocity_model_slug(model: &VelocityFunctionSource) -> String {
    match model {
        VelocityFunctionSource::ConstantVelocity { velocity_m_per_s } => {
            format!("constant-{}", format_factor(*velocity_m_per_s))
        }
        VelocityFunctionSource::TimeVelocityPairs { .. } => "time-velocity-pairs".to_string(),
        VelocityFunctionSource::VelocityAssetReference { asset_id } => {
            format!(
                "velocity-asset-{}",
                asset_id.replace(' ', "-").to_ascii_lowercase()
            )
        }
    }
}

fn optional_factor_slug(value: Option<f32>) -> String {
    value
        .map(format_factor)
        .unwrap_or_else(|| "none".to_string())
}

fn volume_arithmetic_operator_slug(
    operator: seis_runtime::TraceLocalVolumeArithmeticOperator,
) -> &'static str {
    match operator {
        seis_runtime::TraceLocalVolumeArithmeticOperator::Add => "add",
        seis_runtime::TraceLocalVolumeArithmeticOperator::Subtract => "subtract",
        seis_runtime::TraceLocalVolumeArithmeticOperator::Multiply => "multiply",
        seis_runtime::TraceLocalVolumeArithmeticOperator::Divide => "divide",
    }
}

fn store_path_slug(store_path: &str) -> String {
    Path::new(store_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() {
                        ch.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
        })
        .map(|value| {
            value
                .split('-')
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
                .join("-")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "volume".to_string())
}

fn format_factor(value: f32) -> String {
    let mut formatted = format!("{value:.4}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted.replace('.', "_")
}

fn prestack_third_axis_field(field: PrestackThirdAxisField) -> HeaderField {
    match field {
        PrestackThirdAxisField::Offset => HeaderField::OFFSET,
    }
}

fn prepare_processing_output_store(
    output_path: &Path,
    overwrite_existing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !output_path.exists() {
        return Ok(());
    }
    if !overwrite_existing {
        return Err(format!(
            "Output processing store already exists: {}",
            output_path.display()
        )
        .into());
    }
    let metadata = fs::symlink_metadata(output_path)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(output_path)?;
    } else {
        fs::remove_file(output_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;
    use ophiolite::{GatherAxisKind, GatherProcessingOperation, GatherSelector};
    use seis_io::write_small_prestack_segy_fixture;
    use seis_runtime::{
        CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, SurveyGridTransform,
        SurveySpatialAvailability, SurveySpatialDescriptor, TbvolManifest, VolumeAxes,
        VolumeMetadata, create_tbvol_store,
    };
    use serde_json::Value;
    use std::sync::Arc;
    use tempfile::tempdir;
    use zarrs::array::{ArrayBuilder, DataType};
    use zarrs::filesystem::FilesystemStore;
    use zarrs::group::GroupBuilder;
    use zarrs::storage::ReadableWritableListableStorage;

    fn decode_f32le(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(std::mem::size_of::<f32>())
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn legacy_tbvol_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/f3.tbvol")
    }

    fn zarr_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/survey.zarr")
    }

    fn create_synthetic_mdio_fixture(root: &Path) {
        let store: ReadableWritableListableStorage =
            Arc::new(FilesystemStore::new(root).expect("mdio filesystem store"));
        GroupBuilder::new()
            .build(store.clone(), "/")
            .expect("build mdio root group");

        create_mdio_axis_u16(store.clone(), "/inline", "inline", &[100, 101]);
        create_mdio_axis_u16(store.clone(), "/crossline", "crossline", &[200, 201, 202]);
        create_mdio_axis_u16_with_attrs(
            store.clone(),
            "/time",
            "time",
            &[0, 4, 8, 12],
            Some(serde_json::json!({"unitsV1":{"time":"ms"}})),
        );

        let seismic = ArrayBuilder::new(vec![2, 3, 4], vec![2, 2, 4], DataType::Float32, 0.0_f32)
            .dimension_names(Some(["inline", "crossline", "time"]))
            .build(store, "/seismic")
            .expect("build seismic array");
        seismic.store_metadata().expect("store seismic metadata");
        seismic
            .store_array_subset_elements(
                &zarrs::array_subset::ArraySubset::new_with_ranges(&[0..2, 0..3, 0..4]),
                &(0..24).map(|value| value as f32).collect::<Vec<_>>(),
            )
            .expect("store seismic samples");
    }

    fn create_mdio_axis_u16(
        store: ReadableWritableListableStorage,
        path: &str,
        dimension_name: &str,
        values: &[u16],
    ) {
        create_mdio_axis_u16_with_attrs(store, path, dimension_name, values, None);
    }

    fn create_mdio_axis_u16_with_attrs(
        store: ReadableWritableListableStorage,
        path: &str,
        dimension_name: &str,
        values: &[u16],
        attributes: Option<serde_json::Value>,
    ) {
        let mut array = ArrayBuilder::new(
            vec![values.len() as u64],
            vec![values.len() as u64],
            DataType::UInt16,
            0_u16,
        )
        .dimension_names(Some([dimension_name]))
        .build(store, path)
        .expect("build mdio axis");
        if let Some(serde_json::Value::Object(map)) = attributes {
            for (key, value) in map {
                array.attributes_mut().insert(key, value);
            }
        }
        array.store_metadata().expect("store mdio axis metadata");
        array
            .store_array_subset_elements(
                &zarrs::array_subset::ArraySubset::new_with_ranges(&[0..values.len() as u64]),
                values,
            )
            .expect("store mdio axis values");
    }

    fn segy_fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test-data")
            .join(relative)
    }

    fn relocate_small_geometry_headers(path: &Path) {
        let mut bytes = fs::read(path).expect("read segy fixture");
        let first_trace_offset = 3600usize;
        let trace_size = 240 + (50 * 4);

        for trace_index in 0..25 {
            let trace_offset = first_trace_offset + trace_index * trace_size;
            let inline_src = trace_offset + 188;
            let crossline_src = trace_offset + 192;
            let inline_dst = trace_offset + 16;
            let crossline_dst = trace_offset + 24;

            let inline = bytes[inline_src..inline_src + 4].to_vec();
            let crossline = bytes[crossline_src..crossline_src + 4].to_vec();
            bytes[inline_dst..inline_dst + 4].copy_from_slice(&inline);
            bytes[crossline_dst..crossline_dst + 4].copy_from_slice(&crossline);
            bytes[inline_src..inline_src + 4].fill(0);
            bytes[crossline_src..crossline_src + 4].fill(0);
        }

        fs::write(path, bytes).expect("write relocated segy fixture");
    }

    fn create_test_store(root: &Path) {
        create_test_store_with_origin(root, 1_000.0, 2_000.0);
    }

    fn create_test_store_with_origin(root: &Path, origin_x: f64, origin_y: f64) {
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 4,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: seis_runtime::SampleDataFidelity {
                        source_sample_type: "ibm32".to_string(),
                        working_sample_type: "f32".to_string(),
                        conversion: seis_runtime::SampleDataConversionKind::Cast,
                        preservation: seis_runtime::SampleValuePreservation::PotentiallyLossy,
                        notes: Vec::new(),
                    },
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
                        inline_field: HeaderFieldSpec {
                            name: String::from("INLINE_3D"),
                            start_byte: 189,
                            value_type: String::from("I32"),
                        },
                        crossline_field: HeaderFieldSpec {
                            name: String::from("CROSSLINE_3D"),
                            start_byte: 193,
                            value_type: String::from("I32"),
                        },
                        third_axis_field: None,
                    },
                    regularization: None,
                },
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 10.0, 20.0, 30.0],
                ),
                coordinate_reference_binding: Some(CoordinateReferenceBinding {
                    detected: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:32631")),
                        name: Some(String::from("WGS 84 / UTM zone 31N")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    effective: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:32631")),
                        name: Some(String::from("WGS 84 / UTM zone 31N")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    source: CoordinateReferenceSource::Header,
                    notes: Vec::new(),
                }),
                spatial: Some(SurveySpatialDescriptor {
                    coordinate_reference: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:32631")),
                        name: Some(String::from("WGS 84 / UTM zone 31N")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    grid_transform: Some(SurveyGridTransform {
                        origin: ProjectedPoint2 {
                            x: origin_x,
                            y: origin_y,
                        },
                        inline_basis: seis_runtime::ProjectedVector2 { x: 10.0, y: 0.0 },
                        xline_basis: seis_runtime::ProjectedVector2 { x: 0.0, y: 20.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
                segy_export: None,
            },
            [2, 2, 4],
            false,
        );
        create_tbvol_store(root, manifest, &Array3::<f32>::zeros((2, 2, 4)), None)
            .expect("create store");
    }

    fn write_test_navigation_file(path: &Path) {
        fs::write(
            path,
            [
                "Q10000                  1545628.01N  437 1.82E 603600.06089568.0     DATR12I-021",
                "Q10000                  2545627.92N  437 1.54E 603600.06089588.0     DATR12I-021",
                "Q10002                  1545041.58N  42730.78E 603610.06089568.0     DATR12I-022",
                "Q10002                  2545041.77N  42730.78E 603610.06089588.0     DATR12I-022",
            ]
            .join("\n"),
        )
        .expect("write navigation sidecar");
    }

    fn format_test_nlog_3d_essov2xy_row(
        key_a: i32,
        key_b: i32,
        key_c: i32,
        date: i32,
        time_ms: i32,
        velocity_m_per_s: i32,
        y: i32,
        x: i32,
    ) -> String {
        format!(
            "V2{:>5}{:>8}{:>6}{:>14}{:>5}{:>21} {:>7} {:>6}    ",
            key_a, key_b, key_c, date, time_ms, velocity_m_per_s, y, x
        )
    }

    fn write_constant_horizon_xyz(path: &Path, value: f32) {
        let payload = [
            format!("1000 2000 {value}"),
            format!("1000 2020 {value}"),
            format!("1010 2000 {value}"),
            format!("1010 2020 {value}"),
        ]
        .join("\n");
        fs::write(path, payload).expect("write horizon xyz");
    }

    fn load_stored_horizon_grid(root: &Path, horizon_id: &str) -> (Vec<f32>, Vec<u8>) {
        let manifest_path = root.join("horizons").join("manifest.json");
        let manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).expect("read horizons manifest"))
                .expect("parse horizons manifest");
        let entry = manifest["horizons"]
            .as_array()
            .expect("horizon entries")
            .iter()
            .find(|entry| entry["id"].as_str() == Some(horizon_id))
            .expect("horizon manifest entry");
        let values_file = entry["values_file"]
            .as_str()
            .expect("horizon values file name");
        let validity_file = entry["validity_file"]
            .as_str()
            .expect("horizon validity file name");
        let values = decode_f32le(
            &fs::read(root.join("horizons").join(values_file)).expect("read horizon values"),
        );
        let validity =
            fs::read(root.join("horizons").join(validity_file)).expect("read horizon validity");
        (values, validity)
    }

    #[test]
    fn import_dataset_imports_zarr_fixture_to_tbvol_when_available() {
        let fixture = zarr_fixture_path();
        if !fixture.exists() {
            return;
        }

        let temp = tempdir().expect("temp dir");
        let output = temp.path().join("survey.tbvol");
        let response = import_dataset(ImportDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: fixture.display().to_string(),
            output_store_path: output.display().to_string(),
            geometry_override: None,
            overwrite_existing: false,
        })
        .expect("zarr fixture should import");

        assert_eq!(response.dataset.descriptor.shape, [23, 18, 75]);
        assert_eq!(response.dataset.descriptor.chunk_shape[2], 75);
    }

    #[test]
    fn import_dataset_imports_synthetic_mdio_fixture_to_tbvol() {
        let temp = tempdir().expect("temp dir");
        let fixture = temp.path().join("synthetic.mdio");
        let output = temp.path().join("synthetic.tbvol");
        create_synthetic_mdio_fixture(&fixture);

        let response = import_dataset(ImportDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: fixture.display().to_string(),
            output_store_path: output.display().to_string(),
            geometry_override: None,
            overwrite_existing: false,
        })
        .expect("mdio fixture should import");

        assert_eq!(response.dataset.descriptor.shape, [2, 3, 4]);
        assert_eq!(response.dataset.descriptor.chunk_shape[2], 4);
        assert_eq!(response.dataset.descriptor.sample_interval_ms, 4.0);
    }

    #[test]
    fn mdio_import_capacity_error_describes_required_space() {
        let message = mdio_import_capacity_error(
            &seis_runtime::MdioTbvolStorageEstimate {
                shape: [3437, 5053, 1501],
                tile_shape: [28, 31, 1501],
                tile_count: 594,
                has_occupancy: true,
                amplitude_bytes: 104 * 1024 * 1024 * 1024,
                occupancy_bytes: 17 * 1024 * 1024,
                total_bytes: 104 * 1024 * 1024 * 1024 + 17 * 1024 * 1024,
            },
            14 * 1024 * 1024 * 1024,
            Path::new("/Users/sc/Library/Application Support/com.traceboost.app/volumes"),
            Path::new("/Users/sc/Downloads/SubsurfaceData/poseidon/full_stack_agc.mdio"),
            Path::new(
                "/Users/sc/Library/Application Support/com.traceboost.app/volumes/full_stack_agc.tbvol",
            ),
        );

        assert!(message.contains("needs about"));
        assert!(message.contains("safety reserve"));
        assert!(message.contains("Use a smaller ROI/subset import"));
    }

    #[test]
    fn sparse_segy_import_capacity_error_describes_regularization_blowup() {
        let message = sparse_segy_import_capacity_error(
            &SegyRegularizedTbvolStorageEstimate {
                shape: [24_020, 27_013, 1501],
                tile_shape: [23, 18, 1501],
                observed_trace_count: 64_860,
                expected_trace_count: 648_859_440,
                completeness_ratio: 0.00009996001599360256,
                amplitude_bytes: 3_895_320_000_000,
                occupancy_bytes: 651_000_000,
                total_bytes: 3_895_971_000_000,
            },
            120 * 1024 * 1024 * 1024,
            Path::new("/Users/sc/Library/Application Support/com.traceboost.app/volumes"),
            Path::new(
                "/Users/sc/Downloads/SubsurfaceData/open-data/teapot-dome/seismic/filt_mig.sgy",
            ),
            Path::new(
                "/Users/sc/Library/Application Support/com.traceboost.app/volumes/filt_mig.tbvol",
            ),
        );

        assert!(message.contains("regularize 64860 observed traces"));
        assert!(message.contains("648859440"));
        assert!(message.contains("completeness 0.0100%"));
        assert!(message.contains("Review the inline/crossline mapping"));
    }

    #[test]
    fn effective_available_import_bytes_counts_reclaimable_output_and_temp() {
        let temp = tempdir().expect("temp dir");
        let output = temp.path().join("survey.tbvol");
        let stale_temp = temp.path().join("survey.tbvol.tmp");
        fs::write(&output, vec![0_u8; 16]).expect("write output");
        fs::write(&stale_temp, vec![0_u8; 32]).expect("write stale temp");

        let available = effective_available_import_bytes(64, &output, true)
            .expect("estimate effective available bytes");
        assert_eq!(available, 112);
    }

    #[test]
    fn export_dataset_zarr_roundtrips_legacy_tbvol_fixture() {
        let fixture = legacy_tbvol_fixture_path();
        if !fixture.exists() {
            return;
        }

        let temp = tempdir().expect("temp dir");
        let exported = temp.path().join("f3-export.zarr");
        let reimported = temp.path().join("f3-export-import.tbvol");

        let export_response = export_dataset_zarr(
            fixture.display().to_string(),
            exported.display().to_string(),
            false,
        )
        .expect("legacy tbvol fixture should export to zarr");
        assert_eq!(PathBuf::from(&export_response.output_path), exported);

        let import_response = import_dataset(ImportDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: exported.display().to_string(),
            output_store_path: reimported.display().to_string(),
            geometry_override: None,
            overwrite_existing: false,
        })
        .expect("exported zarr should import");

        let reopened = open_dataset_summary(OpenDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: reimported.display().to_string(),
        })
        .expect("reimported tbvol should open");

        assert_eq!(import_response.dataset.descriptor.shape, [23, 18, 75]);
        assert_eq!(
            reopened.dataset.descriptor.geometry.fingerprint,
            import_response.dataset.descriptor.geometry.fingerprint
        );
        assert_eq!(
            reopened.dataset.descriptor.sample_interval_ms,
            import_response.dataset.descriptor.sample_interval_ms
        );
    }

    #[test]
    fn parse_velocity_functions_file_groups_sparse_profiles() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("Velocity_functions.txt");
        fs::write(
            &input,
            [
                "This data contains example velocities, not measured velocities",
                "CDP-X       CDP-Y   Time(ms)  Vrms    Vint    Vavg   Depth(m)",
                " 605882.71  6073657.74   50.00 1500.00 1500.00 1500.00   37.50",
                " 605882.71  6073657.74  858.86 1936.22 1960.00 1933.22  830.19",
                " 606082.63  6073663.33   50.00 1500.00 1500.00 1500.00   37.50",
                " 606082.63  6073663.33  859.57 1936.24 1960.00 1933.24  830.88",
            ]
            .join("\n"),
        )
        .expect("write sample velocity functions file");

        let parsed = parse_velocity_functions_file(&input).expect("parse velocity functions");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].samples.len(), 2);
        assert_eq!(parsed.profiles[1].samples.len(), 2);
        assert_eq!(parsed.profiles[0].samples[0].vint_m_per_s, Some(1500.0));
        assert_eq!(parsed.profiles[0].samples[1].depth_m, Some(830.19));
    }

    #[test]
    fn parse_headerless_velocity_control_profiles_as_rms() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("DANA_F2_TRIBE_RMS_Lines_21-22.vels");
        fs::write(
            &input,
            [
                "1000 2000 10 1600",
                "1000 2000 0 1500",
                "1000 2020 10 1610",
                "1000 2020 0 1510",
            ]
            .join("\n"),
        )
        .expect("write rms velocity control profiles");

        let parsed = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Rms)
            .expect("parse rms velocity control profiles");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].samples.len(), 2);
        assert_eq!(parsed.profiles[0].samples[0].time_ms, 0.0);
        assert_eq!(parsed.profiles[0].samples[0].vrms_m_per_s, Some(1500.0));
        assert_eq!(parsed.profiles[0].samples[0].vint_m_per_s, None);
        assert_eq!(parsed.profiles[0].samples[1].time_ms, 10.0);
        assert_eq!(parsed.profiles[1].samples[0].vrms_m_per_s, Some(1510.0));
    }

    #[test]
    fn parse_headerless_three_column_velocity_rows_requires_navigation_mapping() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("line_velocity.vels");
        fs::write(&input, ["10000 0 1500", "10000 10 1600"].join("\n"))
            .expect("write unsupported velocity rows");

        let error = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Rms)
            .expect_err("three-column rows should require navigation mapping");
        let message = error.to_string();
        assert!(
            message.contains("need line/navigation mapping before import"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn prestack_import_load_preview_scan_and_materialize_work_from_synthetic_segy() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("small-ps.sgy");
        let store = temp.path().join("small-ps.tbgath");
        write_small_prestack_segy_fixture(&input).expect("write synthetic prestack segy");

        let imported = import_prestack_offset_dataset(ImportPrestackOffsetDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: input.display().to_string(),
            output_store_path: store.display().to_string(),
            third_axis_field: PrestackThirdAxisField::Offset,
            overwrite_existing: false,
        })
        .expect("import synthetic prestack segy");

        assert_eq!(imported.dataset.descriptor.shape, [4, 3, 10]);
        assert_eq!(
            imported
                .dataset
                .descriptor
                .geometry
                .summary
                .gather_axis_kind,
            Some(GatherAxisKind::Offset)
        );

        let gather_request = GatherRequest {
            dataset_id: imported.dataset.descriptor.id.clone(),
            selector: GatherSelector::Ordinal { index: 0 },
        };
        let gather = load_gather(store.display().to_string(), gather_request.clone())
            .expect("load first gather");
        assert_eq!(gather.traces, 2);
        assert_eq!(gather.samples, 10);

        let gather_amplitudes = decode_f32le(&gather.amplitudes_f32le);
        assert_eq!(gather_amplitudes.len(), 20);
        assert!(gather_amplitudes[10] > gather_amplitudes[0]);

        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: Some(String::from("Offset mute smoke")),
            description: None,
            trace_local_pipeline: None,
            operations: vec![GatherProcessingOperation::OffsetMute {
                min_offset: Some(2.0),
                max_offset: Some(2.0),
            }],
        };

        let preview = preview_gather_processing(PreviewGatherProcessingRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            gather: gather_request.clone(),
            pipeline: pipeline.clone(),
        })
        .expect("preview gather processing");
        assert!(preview.preview.preview_ready);
        assert_eq!(preview.preview.gather.traces, 2);

        let scan = run_velocity_scan(VelocityScanRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            gather: gather_request.clone(),
            trace_local_pipeline: None,
            min_velocity_m_per_s: 1_500.0,
            max_velocity_m_per_s: 2_000.0,
            velocity_step_m_per_s: 250.0,
            autopick: None,
        })
        .expect("run velocity scan");
        assert_eq!(
            scan.panel.velocities_m_per_s,
            vec![1_500.0, 1_750.0, 2_000.0]
        );
        assert_eq!(scan.panel.sample_axis_ms.len(), 10);
        assert_eq!(decode_f32le(&scan.panel.semblance_f32le).len(), 30);

        let derived = apply_gather_processing(RunGatherProcessingRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            output_store_path: Some(temp.path().join("offset-mute.tbgath").display().to_string()),
            overwrite_existing: false,
            pipeline,
        })
        .expect("materialize gather processing");
        assert_eq!(derived.descriptor.shape, [4, 3, 10]);
        assert_eq!(
            derived.descriptor.geometry.summary.gather_axis_kind,
            Some(GatherAxisKind::Offset)
        );

        let derived_gather = load_gather(
            derived.store_path.clone(),
            GatherRequest {
                dataset_id: derived.descriptor.id.clone(),
                selector: GatherSelector::Ordinal { index: 0 },
            },
        )
        .expect("load derived gather");
        let derived_amplitudes = decode_f32le(&derived_gather.amplitudes_f32le);
        assert!(
            derived_amplitudes[..10]
                .iter()
                .all(|value| value.abs() < 1.0e-6)
        );
        assert!(
            derived_amplitudes[10..]
                .iter()
                .any(|value| value.abs() > 0.0)
        );
    }

    #[test]
    fn parse_line_cdp_velocity_rows_with_navigation_sidecar() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("Z2DAN2012A_RMS_Vels.ivl");
        let navigation_dir = temp.path().join("navigation");
        fs::create_dir_all(&navigation_dir).expect("create navigation dir");
        write_test_navigation_file(&navigation_dir.join("Z2DAN2012A-1.hdr.sgn"));
        fs::write(
            &input,
            [
                "10000 1 10 1600",
                "10000 1 0 1500",
                "10002 2 10 1610",
                "10002 2 0 1510",
            ]
            .join("\n"),
        )
        .expect("write line/cdp velocity control profiles");

        let parsed = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Rms)
            .expect("parse navigation-backed velocity control profiles");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].location.x, 603600.0);
        assert_eq!(parsed.profiles[0].location.y, 6089568.0);
        assert_eq!(parsed.profiles[0].samples[0].vrms_m_per_s, Some(1500.0));
        assert_eq!(parsed.profiles[1].location.x, 603610.0);
        assert_eq!(parsed.profiles[1].location.y, 6089588.0);
        assert_eq!(parsed.profiles[1].samples[1].vrms_m_per_s, Some(1610.0));
    }

    #[test]
    fn parse_sectioned_line_velocity_rows_with_navigation_sidecar() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("Z2DAN2012A_RMS_Vels.ivl");
        write_test_navigation_file(&temp.path().join("Z2DAN2012A-1.hdr.sgn"));
        fs::write(
            &input,
            [
                "DATR12I-021:",
                "1 0 1500",
                "1 10 1600",
                "DATR12I-022:",
                "2 0 1510",
                "2 10 1610",
            ]
            .join("\n"),
        )
        .expect("write sectioned line/cdp velocity control profiles");

        let parsed = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Rms)
            .expect("parse sectioned navigation-backed velocity control profiles");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].samples[0].vrms_m_per_s, Some(1500.0));
        assert_eq!(parsed.profiles[1].samples[1].vrms_m_per_s, Some(1610.0));
    }

    #[test]
    fn parse_nlog_ascii_rms_velocity_rows_without_date_token() {
        let temp = tempdir().expect("temp dir");
        let input = temp
            .path()
            .join("L2EBN2019ASCAN004_RMS_velocities_stacking_ascii.txt");
        fs::write(
            &input,
            [
                "#LINE: L2EBN2019ASCAN004",
                "#08:15 2D Line name",
                "0        1         2         3         4         5         6         7         8",
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "V2     ASCAN004  400                  24          2          1465 166292 510384",
                "V2     ASCAN004  400                 203          2          1493 166292 510384",
                "V2     ASCAN004  800                  24          2          1464 166239 509385",
                "V2     ASCAN004  800                 206          2          1543 166239 509385",
            ]
            .join("\n"),
        )
        .expect("write nlog rms velocity control profiles");

        let parsed = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Rms)
            .expect("parse nlog rms velocity control profiles");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].location.x, 166239.0);
        assert_eq!(parsed.profiles[0].location.y, 509385.0);
        assert_eq!(parsed.profiles[0].samples[0].time_ms, 24.0);
        assert_eq!(parsed.profiles[0].samples[0].vrms_m_per_s, Some(1464.0));
        assert_eq!(parsed.profiles[1].samples[1].time_ms, 203.0);
        assert_eq!(parsed.profiles[1].samples[1].vrms_m_per_s, Some(1493.0));
    }

    #[test]
    fn parse_nlog_ascii_interval_velocity_rows_with_date_token() {
        let temp = tempdir().expect("temp dir");
        let input = temp
            .path()
            .join("L2EBN2019ASCAN004_PreSTM_velocities_migration_ascii.txt");
        fs::write(
            &input,
            [
                "#LINE: L2EBN2019ASCAN004",
                "V2     ASCAN004  200 PDADSTCK310720    0          2          1500 166324 510883",
                "V2     ASCAN004  200 PDADSTCK310720   24          2          1500 166324 510883",
                "V2     ASCAN004  400 PDADSTCK310720    0          2          1510 166292 510384",
                "V2     ASCAN004  400 PDADSTCK310720   24          2          1516 166292 510384",
            ]
            .join("\n"),
        )
        .expect("write nlog migration velocity control profiles");

        let parsed = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Interval)
            .expect("parse nlog migration velocity control profiles");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].location.x, 166292.0);
        assert_eq!(parsed.profiles[0].location.y, 510384.0);
        assert_eq!(parsed.profiles[0].samples[0].vint_m_per_s, Some(1510.0));
        assert_eq!(parsed.profiles[0].samples[1].time_ms, 24.0);
        assert_eq!(parsed.profiles[1].samples[0].vint_m_per_s, Some(1500.0));
    }

    #[test]
    fn parse_nlog_3d_essov2xy_rows_as_interval_profiles() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("NLDEF_PSTM_vels.essov2xy");
        fs::write(
            &input,
            [
                format_test_nlog_3d_essov2xy_row(630, 1030, 44880, 121018, 114, 1511, 2000, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 44880, 121018, 10000, 5053, 2000, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 45680, 121018, 140, 1546, 2020, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 45680, 121018, 1271, 2008, 2020, 1000),
            ]
            .join("\n"),
        )
        .expect("write essov2xy rows");

        let parsed = parse_velocity_control_profiles_file(&input, VelocityQuantityKind::Interval)
            .expect("parse essov2xy rows");
        assert_eq!(parsed.sample_count, 4);
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles[0].location.x, 1000.0);
        assert_eq!(parsed.profiles[0].location.y, 2000.0);
        assert_eq!(parsed.profiles[0].samples[0].time_ms, 114.0);
        assert_eq!(parsed.profiles[0].samples[0].vint_m_per_s, Some(1511.0));
        assert_eq!(parsed.profiles[0].samples[1].time_ms, 10000.0);
        assert_eq!(parsed.profiles[0].samples[1].vint_m_per_s, Some(5053.0));
        assert_eq!(parsed.profiles[1].location.y, 2020.0);
        assert_eq!(parsed.profiles[1].samples[1].vint_m_per_s, Some(2008.0));
    }

    #[test]
    fn import_nlog_3d_essov2xy_interval_model_builds_depth_transform_end_to_end() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("survey.tbvol");
        create_test_store(&store);

        let input = temp.path().join("NLDEF_PSTM_vels.essov2xy");
        fs::write(
            &input,
            [
                format_test_nlog_3d_essov2xy_row(630, 1030, 44880, 121018, 0, 2000, 2000, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 44880, 121018, 10, 2000, 2000, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 44880, 121018, 20, 2000, 2000, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 44880, 121018, 30, 2000, 2000, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 45680, 121018, 0, 2000, 2020, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 45680, 121018, 10, 2000, 2020, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 45680, 121018, 20, 2000, 2020, 1000),
                format_test_nlog_3d_essov2xy_row(630, 1030, 45680, 121018, 30, 2000, 2020, 1000),
                format_test_nlog_3d_essov2xy_row(631, 1030, 44880, 121018, 0, 2000, 2000, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 44880, 121018, 10, 2000, 2000, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 44880, 121018, 20, 2000, 2000, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 44880, 121018, 30, 2000, 2000, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 45680, 121018, 0, 2000, 2020, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 45680, 121018, 10, 2000, 2020, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 45680, 121018, 20, 2000, 2020, 1010),
                format_test_nlog_3d_essov2xy_row(631, 1030, 45680, 121018, 30, 2000, 2020, 1010),
            ]
            .join("\n"),
        )
        .expect("write essov2xy interval rows");

        let response = import_velocity_functions_model(
            store.display().to_string(),
            input.display().to_string(),
            VelocityQuantityKind::Interval,
        )
        .expect("import essov2xy interval velocity model");

        assert_eq!(response.profile_count, 4);
        assert_eq!(response.sample_count, 16);
        assert_eq!(response.model.inline_count, 2);
        assert_eq!(response.model.xline_count, 2);

        let display = resolved_section_display_view(
            &store,
            seis_runtime::SectionAxis::Inline,
            0,
            TimeDepthDomain::Depth,
            Some(&VelocityFunctionSource::VelocityAssetReference {
                asset_id: response.model.id.clone(),
            }),
            Some(VelocityQuantityKind::Interval),
            false,
        )
        .expect("resolve essov2xy depth display");
        let depth_axis = decode_f32le(&display.section.sample_axis_f32le);
        assert_eq!(depth_axis.len(), 4);
        for (actual, expected) in depth_axis.iter().zip([0.0_f32, 10.0, 20.0, 30.0]) {
            assert!(
                (actual - expected).abs() <= 1e-4,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn import_velocity_functions_model_builds_depth_transform_end_to_end() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("survey.tbvol");
        create_test_store(&store);

        let input = temp.path().join("Velocity_functions.txt");
        std::fs::write(
            &input,
            [
                "CDP-X       CDP-Y   Time(ms)  Vrms    Vint    Vavg   Depth(m)",
                "1000 2000 0 2000 2000 2000 0",
                "1000 2000 10 2000 2000 2000 10",
                "1000 2000 20 2000 2000 2000 20",
                "1000 2000 30 2000 2000 2000 30",
                "1000 2020 0 2000 2000 2000 0",
                "1000 2020 10 2000 2000 2000 10",
                "1000 2020 20 2000 2000 2000 20",
                "1000 2020 30 2000 2000 2000 30",
                "1010 2000 0 2000 2000 2000 0",
                "1010 2000 10 2000 2000 2000 10",
                "1010 2000 20 2000 2000 2000 20",
                "1010 2000 30 2000 2000 2000 30",
                "1010 2020 0 2000 2000 2000 0",
                "1010 2020 10 2000 2000 2000 10",
                "1010 2020 20 2000 2000 2000 20",
                "1010 2020 30 2000 2000 2000 30",
            ]
            .join("\n"),
        )
        .expect("write velocity functions");

        let response = import_velocity_functions_model(
            store.display().to_string(),
            input.display().to_string(),
            VelocityQuantityKind::Interval,
        )
        .expect("import velocity model");

        assert_eq!(response.profile_count, 4);
        assert_eq!(response.sample_count, 16);
        assert_eq!(response.model.inline_count, 2);
        assert_eq!(response.model.xline_count, 2);
        assert_eq!(response.model.depth_unit, "m");

        let display = resolved_section_display_view(
            &store,
            seis_runtime::SectionAxis::Inline,
            0,
            TimeDepthDomain::Depth,
            Some(&VelocityFunctionSource::VelocityAssetReference {
                asset_id: response.model.id.clone(),
            }),
            Some(VelocityQuantityKind::Interval),
            false,
        )
        .expect("resolve depth display");
        let depth_axis = decode_f32le(&display.section.sample_axis_f32le);
        assert_eq!(depth_axis.len(), 4);
        for (actual, expected) in depth_axis.iter().zip([0.0_f32, 10.0, 20.0, 30.0]) {
            assert!(
                (actual - expected).abs() <= 1e-4,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn import_rms_velocity_functions_model_builds_depth_transform_end_to_end() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("survey.tbvol");
        create_test_store(&store);

        let input = temp.path().join("DANA_F2_TRIBE_RMS_Lines_21-22.vels");
        std::fs::write(
            &input,
            [
                "1000 2000 0 2000",
                "1000 2000 10 2000",
                "1000 2000 20 2000",
                "1000 2000 30 2000",
                "1000 2020 0 2000",
                "1000 2020 10 2000",
                "1000 2020 20 2000",
                "1000 2020 30 2000",
                "1010 2000 0 2000",
                "1010 2000 10 2000",
                "1010 2000 20 2000",
                "1010 2000 30 2000",
                "1010 2020 0 2000",
                "1010 2020 10 2000",
                "1010 2020 20 2000",
                "1010 2020 30 2000",
            ]
            .join("\n"),
        )
        .expect("write rms velocity control profiles");

        let response = import_velocity_functions_model(
            store.display().to_string(),
            input.display().to_string(),
            VelocityQuantityKind::Rms,
        )
        .expect("import rms velocity model");

        assert_eq!(response.profile_count, 4);
        assert_eq!(response.sample_count, 16);
        assert_eq!(response.model.inline_count, 2);
        assert_eq!(response.model.xline_count, 2);
        assert_eq!(response.model.depth_unit, "m");

        let display = resolved_section_display_view(
            &store,
            seis_runtime::SectionAxis::Inline,
            0,
            TimeDepthDomain::Depth,
            Some(&VelocityFunctionSource::VelocityAssetReference {
                asset_id: response.model.id.clone(),
            }),
            Some(VelocityQuantityKind::Rms),
            false,
        )
        .expect("resolve rms depth display");
        let depth_axis = decode_f32le(&display.section.sample_axis_f32le);
        assert_eq!(depth_axis.len(), 4);
        for (actual, expected) in depth_axis.iter().zip([0.0_f32, 10.0, 20.0, 30.0]) {
            assert!(
                (actual - expected).abs() <= 1e-4,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn import_navigation_backed_rms_velocity_model_builds_depth_transform_end_to_end() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("survey.tbvol");
        create_test_store_with_origin(&store, 603_600.0, 6_089_568.0);

        let input = temp.path().join("Z2DAN2012A_RMS_Vels.ivl");
        write_test_navigation_file(&temp.path().join("Z2DAN2012A-1.hdr.sgn"));
        std::fs::write(
            &input,
            [
                "10000 1 0 2000",
                "10000 1 10 2000",
                "10000 1 20 2000",
                "10000 1 30 2000",
                "10000 2 0 2000",
                "10000 2 10 2000",
                "10000 2 20 2000",
                "10000 2 30 2000",
                "10002 1 0 2000",
                "10002 1 10 2000",
                "10002 1 20 2000",
                "10002 1 30 2000",
                "10002 2 0 2000",
                "10002 2 10 2000",
                "10002 2 20 2000",
                "10002 2 30 2000",
            ]
            .join("\n"),
        )
        .expect("write navigation-backed rms velocity control profiles");

        let response = import_velocity_functions_model(
            store.display().to_string(),
            input.display().to_string(),
            VelocityQuantityKind::Rms,
        )
        .expect("import navigation-backed rms velocity model");

        assert_eq!(response.profile_count, 4);
        assert_eq!(response.sample_count, 16);
        assert_eq!(response.model.inline_count, 2);
        assert_eq!(response.model.xline_count, 2);

        let display = resolved_section_display_view(
            &store,
            seis_runtime::SectionAxis::Inline,
            0,
            TimeDepthDomain::Depth,
            Some(&VelocityFunctionSource::VelocityAssetReference {
                asset_id: response.model.id.clone(),
            }),
            Some(VelocityQuantityKind::Rms),
            false,
        )
        .expect("resolve navigation-backed rms depth display");
        let depth_axis = decode_f32le(&display.section.sample_axis_f32le);
        assert_eq!(depth_axis.len(), 4);
        for (actual, expected) in depth_axis.iter().zip([0.0_f32, 10.0, 20.0, 30.0]) {
            assert!(
                (actual - expected).abs() <= 1e-4,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn describe_velocity_volume_store_builds_canonical_dense_source_descriptor() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("velocity.tbvol");
        create_test_store(&store);

        let descriptor = describe_velocity_volume_store(
            store.display().to_string(),
            VelocityQuantityKind::Interval,
        )
        .expect("describe dense velocity source");

        assert_eq!(
            descriptor.source_kind,
            TimeDepthTransformSourceKind::VelocityGrid3D
        );
        assert_eq!(descriptor.velocity_kind, VelocityQuantityKind::Interval);
        assert_eq!(descriptor.vertical_domain, TimeDepthDomain::Time);
        assert_eq!(descriptor.vertical_axis.unit, "ms");
        assert_eq!(descriptor.vertical_axis.start, 0.0);
        assert_eq!(descriptor.vertical_axis.step, 10.0);
        assert_eq!(descriptor.vertical_axis.count, 4);
        assert_eq!(descriptor.inline_count, 2);
        assert_eq!(descriptor.xline_count, 2);
        assert_eq!(
            descriptor.coverage.relationship,
            SpatialCoverageRelationship::Exact
        );
        assert!(
            descriptor
                .notes
                .iter()
                .any(|note| note.contains("Time in ms"))
        );
    }

    #[test]
    fn describe_velocity_volume_store_supports_depth_axis_override() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("velocity-depth.tbvol");
        create_test_store(&store);

        let descriptor = describe_velocity_volume_store_with_options(
            store.display().to_string(),
            VelocityQuantityKind::Interval,
            VelocityVolumeDescriptorOptions {
                vertical_domain: TimeDepthDomain::Depth,
                vertical_unit: "m".to_string(),
                vertical_start: Some(1000.0),
                vertical_step: Some(25.0),
            },
        )
        .expect("describe dense depth velocity source");

        assert_eq!(descriptor.vertical_domain, TimeDepthDomain::Depth);
        assert_eq!(descriptor.vertical_axis.domain, TimeDepthDomain::Depth);
        assert_eq!(descriptor.vertical_axis.unit, "m");
        assert_eq!(descriptor.vertical_axis.start, 1000.0);
        assert_eq!(descriptor.vertical_axis.step, 25.0);
        assert!(descriptor.notes.iter().any(|note| note.contains("Depth")));
    }

    #[test]
    fn describe_velocity_volume_request_returns_wrapped_response() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("velocity-depth.tbvol");
        create_test_store(&store);

        let response = describe_velocity_volume(DescribeVelocityVolumeRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            velocity_kind: VelocityQuantityKind::Interval,
            vertical_domain: Some(TimeDepthDomain::Depth),
            vertical_unit: Some("m".to_string()),
            vertical_start: Some(1000.0),
            vertical_step: Some(25.0),
        })
        .expect("describe wrapped velocity volume");

        assert_eq!(response.schema_version, IPC_SCHEMA_VERSION);
        assert_eq!(response.volume.vertical_domain, TimeDepthDomain::Depth);
        assert_eq!(response.volume.vertical_axis.domain, TimeDepthDomain::Depth);
        assert_eq!(response.volume.vertical_axis.unit, "m");
        assert_eq!(response.volume.vertical_axis.count, 4);
    }

    #[test]
    fn describe_velocity_volume_defaults_to_native_store_axis_metadata() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("velocity-native-depth.tbvol");
        create_test_store(&store);
        set_store_vertical_axis(
            &store,
            TimeDepthDomain::Depth,
            Some("m"),
            Some(1000.0),
            Some(25.0),
        )
        .expect("set native vertical axis");

        let descriptor = describe_velocity_volume_store(
            store.display().to_string(),
            VelocityQuantityKind::Interval,
        )
        .expect("describe native depth velocity source");
        assert_eq!(descriptor.vertical_domain, TimeDepthDomain::Depth);
        assert_eq!(descriptor.vertical_axis.domain, TimeDepthDomain::Depth);
        assert_eq!(descriptor.vertical_axis.unit, "m");
        assert_eq!(descriptor.vertical_axis.start, 1000.0);
        assert_eq!(descriptor.vertical_axis.step, 25.0);

        let response = describe_velocity_volume(DescribeVelocityVolumeRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            velocity_kind: VelocityQuantityKind::Interval,
            vertical_domain: None,
            vertical_unit: None,
            vertical_start: None,
            vertical_step: None,
        })
        .expect("describe wrapped native depth velocity volume");
        assert_eq!(response.volume.vertical_domain, TimeDepthDomain::Depth);
        assert_eq!(response.volume.vertical_axis.domain, TimeDepthDomain::Depth);
        assert_eq!(response.volume.vertical_axis.unit, "m");
        assert_eq!(response.volume.vertical_axis.start, 1000.0);
        assert_eq!(response.volume.vertical_axis.step, 25.0);
    }

    #[test]
    fn ingest_velocity_volume_accepts_geometry_override_for_nonstandard_headers() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("override-geometry-small.sgy");
        let failing_store = temp.path().join("velocity-fail.tbvol");
        let output_store = temp.path().join("velocity-ok.tbvol");
        fs::copy(segy_fixture_path("small.sgy"), &input).expect("copy segy fixture");
        relocate_small_geometry_headers(&input);

        let error = ingest_velocity_volume(IngestVelocityVolumeRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: input.display().to_string(),
            output_store_path: failing_store.display().to_string(),
            velocity_kind: VelocityQuantityKind::Interval,
            vertical_domain: TimeDepthDomain::Time,
            vertical_unit: None,
            vertical_start: None,
            vertical_step: None,
            overwrite_existing: false,
            delete_input_on_success: false,
            geometry_override: None,
        })
        .expect_err("default geometry should fail for relocated headers");
        let message = error.to_string();
        assert!(
            message.contains("geometry")
                || message.contains("duplicate")
                || message.contains("regular")
                || message.contains("inline"),
            "unexpected ingest error: {message}"
        );

        let response = ingest_velocity_volume(IngestVelocityVolumeRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: input.display().to_string(),
            output_store_path: output_store.display().to_string(),
            velocity_kind: VelocityQuantityKind::Interval,
            vertical_domain: TimeDepthDomain::Time,
            vertical_unit: None,
            vertical_start: None,
            vertical_step: None,
            overwrite_existing: false,
            delete_input_on_success: false,
            geometry_override: Some(SegyGeometryOverride {
                inline_3d: Some(SegyHeaderField {
                    start_byte: 17,
                    value_type: SegyHeaderValueType::I32,
                }),
                crossline_3d: Some(SegyHeaderField {
                    start_byte: 25,
                    value_type: SegyHeaderValueType::I32,
                }),
                third_axis: None,
            }),
        })
        .expect("ingest with geometry override");

        assert_eq!(response.volume.inline_count, 5);
        assert_eq!(response.volume.xline_count, 5);
        assert_eq!(response.volume.vertical_domain, TimeDepthDomain::Time);
        assert_eq!(response.volume.vertical_axis.count, 50);

        let opened = open_dataset_summary(OpenDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: output_store.display().to_string(),
        })
        .expect("open ingested velocity volume");
        assert_eq!(opened.dataset.descriptor.shape, [5, 5, 50]);
    }

    #[test]
    fn scan_segy_import_routes_relocated_headers_into_structure_review() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("wizard-geometry-small.sgy");
        let fixture = segy_fixture_path("small.sgy");
        if !fixture.exists() {
            return;
        }
        fs::copy(fixture, &input).expect("copy segy fixture");
        relocate_small_geometry_headers(&input);

        let response = scan_segy_import(ScanSegyImportRequest {
            schema_version: IPC_SCHEMA_VERSION,
            input_path: input.display().to_string(),
        })
        .expect("scan segy import");

        assert_eq!(
            response.recommended_next_stage,
            SegyImportWizardStage::Structure
        );
        assert!(!response.candidate_plans.is_empty());
        assert!(
            response
                .issues
                .iter()
                .any(|issue| issue.code == "review_geometry_mapping")
        );
    }

    #[test]
    fn validate_segy_import_plan_accepts_repaired_geometry_mapping() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("wizard-geometry-fixed.sgy");
        let fixture = segy_fixture_path("small.sgy");
        if !fixture.exists() {
            return;
        }
        fs::copy(fixture, &input).expect("copy segy fixture");
        relocate_small_geometry_headers(&input);
        let fingerprint =
            source_fingerprint_for_input(&input.display().to_string()).expect("source fingerprint");

        let response = validate_segy_import_plan(ValidateSegyImportPlanRequest {
            schema_version: IPC_SCHEMA_VERSION,
            plan: build_segy_import_plan(
                &input.display().to_string(),
                &fingerprint,
                &temp.path().join("wizard-fixed.tbvol").display().to_string(),
                SegyGeometryOverride {
                    inline_3d: Some(SegyHeaderField {
                        start_byte: 17,
                        value_type: SegyHeaderValueType::I32,
                    }),
                    crossline_3d: Some(SegyHeaderField {
                        start_byte: 25,
                        value_type: SegyHeaderValueType::I32,
                    }),
                    third_axis: None,
                },
                SegyImportPlanSource::Manual,
                None,
                None,
                None,
            ),
        })
        .expect("validate repaired geometry mapping");

        assert!(response.can_import);
        assert_eq!(
            response.recommended_next_stage,
            SegyImportWizardStage::Import
        );
        assert_eq!(response.resolved_dataset.layout, "post_stack_3d");
        assert!(
            !response
                .issues
                .iter()
                .any(|issue| issue.severity == SegyImportIssueSeverity::Blocking)
        );
    }

    #[test]
    fn delete_input_path_after_success_rejects_parent_of_output_store() {
        let temp = tempdir().expect("temp dir");
        let input = temp.path().join("raw-input");
        let output = input.join("nested.tbvol");
        fs::create_dir_all(&output).expect("create nested output");

        let error = delete_input_path_after_success(&input, &output)
            .expect_err("should reject deleting output parent");
        let message = error.to_string();
        assert!(message.contains("Refusing to delete"));
        assert!(input.exists());
        assert!(output.exists());
    }

    #[test]
    fn paired_horizon_cli_wrappers_materialize_matching_derived_horizons() {
        let temp = tempdir().expect("temp dir");
        let store = temp.path().join("survey.tbvol");
        create_test_store(&store);

        let anchor_time_top = temp.path().join("anchor_time_top.xyz");
        let anchor_time_base = temp.path().join("anchor_time_base.xyz");
        let anchor_depth_top = temp.path().join("anchor_depth_top.xyz");
        let anchor_depth_base = temp.path().join("anchor_depth_base.xyz");
        let mid_time = temp.path().join("mid_time.xyz");
        let mid_depth = temp.path().join("mid_depth.xyz");

        write_constant_horizon_xyz(&anchor_time_top, 10.0);
        write_constant_horizon_xyz(&anchor_time_base, 20.0);
        write_constant_horizon_xyz(&anchor_depth_top, 12.0);
        write_constant_horizon_xyz(&anchor_depth_base, 32.0);
        write_constant_horizon_xyz(&mid_time, 15.0);
        write_constant_horizon_xyz(&mid_depth, 22.0);

        let imported_time = import_horizon_xyz(ImportHorizonXyzRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            input_paths: vec![
                anchor_time_top.display().to_string(),
                anchor_time_base.display().to_string(),
                mid_time.display().to_string(),
            ],
            vertical_domain: Some(TimeDepthDomain::Time),
            vertical_unit: Some(String::from("ms")),
            source_coordinate_reference_id: None,
            source_coordinate_reference_name: None,
            assume_same_as_survey: true,
        })
        .expect("import time horizons")
        .imported;
        let imported_depth = import_horizon_xyz(ImportHorizonXyzRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store.display().to_string(),
            input_paths: vec![
                anchor_depth_top.display().to_string(),
                anchor_depth_base.display().to_string(),
                mid_depth.display().to_string(),
            ],
            vertical_domain: Some(TimeDepthDomain::Depth),
            vertical_unit: Some(String::from("m")),
            source_coordinate_reference_id: None,
            source_coordinate_reference_name: None,
            assume_same_as_survey: true,
        })
        .expect("import depth horizons")
        .imported;

        let transform = build_paired_horizon_transform(
            store.display().to_string(),
            vec![imported_time[0].id.clone(), imported_time[1].id.clone()],
            vec![imported_depth[0].id.clone(), imported_depth[1].id.clone()],
            Some(String::from("paired-wrapper-transform")),
            Some(String::from("Paired Wrapper Transform")),
        )
        .expect("build paired transform");
        assert_eq!(transform.id, "paired-wrapper-transform");
        assert_eq!(
            transform.source_kind,
            TimeDepthTransformSourceKind::HorizonLayerModel
        );

        let converted_depth = convert_horizon_domain(
            store.display().to_string(),
            imported_time[2].id.clone(),
            transform.id.clone(),
            TimeDepthDomain::Depth,
            Some(String::from("mid_time_to_depth")),
            Some(String::from("Mid Time To Depth")),
        )
        .expect("convert time horizon to depth");
        let converted_time = convert_horizon_domain(
            store.display().to_string(),
            imported_depth[2].id.clone(),
            transform.id.clone(),
            TimeDepthDomain::Time,
            Some(String::from("mid_depth_to_time")),
            Some(String::from("Mid Depth To Time")),
        )
        .expect("convert depth horizon to time");

        assert_eq!(converted_depth.id, "mid_time_to_depth");
        assert_eq!(converted_depth.vertical_domain, TimeDepthDomain::Depth);
        assert_eq!(converted_depth.vertical_unit, "m");
        assert_eq!(converted_time.id, "mid_depth_to_time");
        assert_eq!(converted_time.vertical_domain, TimeDepthDomain::Time);
        assert_eq!(converted_time.vertical_unit, "ms");

        let (depth_values, depth_validity) = load_stored_horizon_grid(&store, "mid_time_to_depth");
        let (time_values, time_validity) = load_stored_horizon_grid(&store, "mid_depth_to_time");

        assert_eq!(depth_validity, vec![1, 1, 1, 1]);
        assert_eq!(time_validity, vec![1, 1, 1, 1]);
        for actual in &depth_values {
            assert!(
                (actual - 22.0).abs() <= 1e-4,
                "expected 22.0 m, got {actual}"
            );
        }
        for actual in &time_values {
            assert!(
                (actual - 15.0).abs() <= 1e-4,
                "expected 15.0 ms, got {actual}"
            );
        }
    }

    #[test]
    fn benchmark_pipeline_builds_expected_agc_steps() {
        let pipeline = benchmark_pipeline(TraceLocalBenchmarkScenario::Agc);
        assert_eq!(pipeline.steps.len(), 2);
        assert!(matches!(
            pipeline.steps[0].operation,
            TraceLocalProcessingOperation::TraceRmsNormalize
        ));
        assert!(matches!(
            pipeline.steps[1].operation,
            TraceLocalProcessingOperation::AgcRms { window_ms }
            if (window_ms - 250.0).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn post_stack_neighborhood_benchmark_pipeline_includes_optional_prefix() {
        let pipeline = post_stack_neighborhood_benchmark_pipeline(
            PostStackNeighborhoodBenchmarkOperator::Similarity,
            24.0,
            1,
            2,
            true,
        );
        assert!(pipeline.trace_local_pipeline.is_some());
        assert_eq!(pipeline.operations.len(), 1);
        assert!(matches!(
            pipeline.operations[0],
            seis_runtime::PostStackNeighborhoodProcessingOperation::Similarity { .. }
        ));
    }

    #[test]
    fn benchmark_variant_targets_default_to_serial_and_partitioned() {
        let request = TraceLocalBenchmarkRequest {
            store_path: "demo.tbvol".to_string(),
            output_root: None,
            scenario: TraceLocalBenchmarkScenario::Scalar,
            partition_target_mib: Vec::new(),
            adaptive_partition_target: false,
            include_serial: true,
            repeat_count: 1,
            keep_outputs: false,
        };

        assert_eq!(
            benchmark_variant_targets(&request, None),
            vec![None, Some(256 * 1024 * 1024)]
        );
    }

    #[test]
    fn benchmark_variant_summary_aggregates_repeated_runs() {
        let summaries = summarize_benchmark_variants(&[
            TraceLocalBenchmarkRunResult {
                label: "serial".to_string(),
                scenario: TraceLocalBenchmarkScenario::Scalar,
                partition_target_bytes: None,
                repeat_index: 1,
                elapsed_ms: 12.0,
                total_tiles: 10,
                completed_tiles: 10,
                total_partitions: 1,
                completed_partitions: 1,
                peak_active_partitions: 1,
                retry_count: 0,
                output_bytes: 128,
                output_store_path: "serial-01.tbvol".to_string(),
                output_retained: false,
            },
            TraceLocalBenchmarkRunResult {
                label: "serial".to_string(),
                scenario: TraceLocalBenchmarkScenario::Scalar,
                partition_target_bytes: None,
                repeat_index: 2,
                elapsed_ms: 18.0,
                total_tiles: 10,
                completed_tiles: 10,
                total_partitions: 1,
                completed_partitions: 1,
                peak_active_partitions: 1,
                retry_count: 0,
                output_bytes: 128,
                output_store_path: "serial-02.tbvol".to_string(),
                output_retained: false,
            },
        ]);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].label, "serial");
        assert!((summaries[0].avg_elapsed_ms - 15.0).abs() < 1e-6);
        assert_eq!(summaries[0].min_elapsed_ms, 12.0);
        assert_eq!(summaries[0].max_elapsed_ms, 18.0);
    }

    #[test]
    fn batch_benchmark_summary_aggregates_and_sorts_variants() {
        let summaries = summarize_batch_benchmark_variants(&[
            TraceLocalBatchBenchmarkVariantResult {
                label: "custom-requested-4-effective-4".to_string(),
                requested_max_active_jobs: Some(4),
                effective_max_active_jobs: 4,
                execution_mode: ProcessingExecutionMode::Custom,
                scheduler_reason: ProcessingSchedulerReason::UserRequested,
                worker_budget: 8,
                global_cap: 8,
                max_memory_cost_class: seis_runtime::MemoryCostClass::Medium,
                max_cpu_cost_class: seis_runtime::CpuCostClass::Medium,
                max_io_cost_class: seis_runtime::IoCostClass::Low,
                min_parallel_efficiency_class: seis_runtime::ParallelEfficiencyClass::High,
                max_estimated_peak_memory_bytes: 192 * 1024 * 1024,
                combined_cpu_weight: 3.0,
                combined_io_weight: 2.0,
                max_expected_partition_count: Some(20),
                repeat_index: 1,
                batch_elapsed_ms: 24.0,
                completed_jobs: 4,
                total_jobs: 4,
                avg_queue_wait_ms: 0.2,
                max_queue_wait_ms: 0.4,
                avg_job_elapsed_ms: 18.0,
                min_job_elapsed_ms: 16.0,
                max_job_elapsed_ms: 20.0,
                avg_total_partitions: 20.0,
                avg_peak_active_partitions: 20.0,
                jobs: Vec::new(),
            },
            TraceLocalBatchBenchmarkVariantResult {
                label: "custom-requested-2-effective-2".to_string(),
                requested_max_active_jobs: Some(2),
                effective_max_active_jobs: 2,
                execution_mode: ProcessingExecutionMode::Custom,
                scheduler_reason: ProcessingSchedulerReason::UserRequested,
                worker_budget: 8,
                global_cap: 8,
                max_memory_cost_class: seis_runtime::MemoryCostClass::Medium,
                max_cpu_cost_class: seis_runtime::CpuCostClass::Medium,
                max_io_cost_class: seis_runtime::IoCostClass::Low,
                min_parallel_efficiency_class: seis_runtime::ParallelEfficiencyClass::High,
                max_estimated_peak_memory_bytes: 192 * 1024 * 1024,
                combined_cpu_weight: 3.0,
                combined_io_weight: 2.0,
                max_expected_partition_count: Some(20),
                repeat_index: 1,
                batch_elapsed_ms: 30.0,
                completed_jobs: 4,
                total_jobs: 4,
                avg_queue_wait_ms: 6.0,
                max_queue_wait_ms: 8.0,
                avg_job_elapsed_ms: 14.0,
                min_job_elapsed_ms: 12.0,
                max_job_elapsed_ms: 16.0,
                avg_total_partitions: 20.0,
                avg_peak_active_partitions: 20.0,
                jobs: Vec::new(),
            },
            TraceLocalBatchBenchmarkVariantResult {
                label: "custom-requested-2-effective-2".to_string(),
                requested_max_active_jobs: Some(2),
                effective_max_active_jobs: 2,
                execution_mode: ProcessingExecutionMode::Custom,
                scheduler_reason: ProcessingSchedulerReason::UserRequested,
                worker_budget: 8,
                global_cap: 8,
                max_memory_cost_class: seis_runtime::MemoryCostClass::Medium,
                max_cpu_cost_class: seis_runtime::CpuCostClass::Medium,
                max_io_cost_class: seis_runtime::IoCostClass::Low,
                min_parallel_efficiency_class: seis_runtime::ParallelEfficiencyClass::High,
                max_estimated_peak_memory_bytes: 192 * 1024 * 1024,
                combined_cpu_weight: 3.0,
                combined_io_weight: 2.0,
                max_expected_partition_count: Some(20),
                repeat_index: 2,
                batch_elapsed_ms: 34.0,
                completed_jobs: 4,
                total_jobs: 4,
                avg_queue_wait_ms: 4.0,
                max_queue_wait_ms: 6.0,
                avg_job_elapsed_ms: 15.0,
                min_job_elapsed_ms: 13.0,
                max_job_elapsed_ms: 17.0,
                avg_total_partitions: 20.0,
                avg_peak_active_partitions: 20.0,
                jobs: Vec::new(),
            },
        ]);

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].label, "custom-requested-2-effective-2");
        assert_eq!(summaries[0].requested_max_active_jobs, Some(2));
        assert_eq!(summaries[0].effective_max_active_jobs, 2);
        assert_eq!(summaries[0].execution_mode, ProcessingExecutionMode::Custom);
        assert_eq!(
            summaries[0].scheduler_reason,
            ProcessingSchedulerReason::UserRequested
        );
        assert_eq!(summaries[0].run_count, 2);
        assert!((summaries[0].avg_batch_elapsed_ms - 32.0).abs() < 1e-6);
        assert_eq!(summaries[0].min_batch_elapsed_ms, 30.0);
        assert_eq!(summaries[0].max_batch_elapsed_ms, 34.0);
        assert!((summaries[0].avg_queue_wait_ms - 5.0).abs() < 1e-6);
        assert!((summaries[0].avg_job_elapsed_ms - 14.5).abs() < 1e-6);

        assert_eq!(summaries[1].label, "custom-requested-4-effective-4");
        assert_eq!(summaries[1].requested_max_active_jobs, Some(4));
        assert_eq!(summaries[1].run_count, 1);
        assert_eq!(summaries[1].avg_batch_elapsed_ms, 24.0);
    }
}
