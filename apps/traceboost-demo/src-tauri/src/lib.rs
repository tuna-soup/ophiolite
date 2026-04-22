mod app_paths;
mod crs_registry;
mod diagnostics;
mod import_manager;
mod preview_session;
mod processing;
mod processing_cache;
mod project_settings;
mod segy_import_recipes;
mod workspace;

#[cfg(test)]
mod preview_session_bench;
#[cfg(test)]
mod processing_cache_bench;

use ophiolite::{
    AssetBindingInput, AssetKind, AssetStatus, CheckshotVspObservationSet1D, ComputeParameterValue,
    ManualTimeDepthPickSet1D, OphioliteProject, ProjectComputeRunRequest,
    ProjectSurveyMapRequestDto, ResolveSectionWellOverlaysResponse, SURVEY_MAP_CONTRACT_VERSION,
    SectionWellOverlayRequestDto, SurveyMapTransformStatusDto,
    WellMarkerHorizonResidualPointRecord, WellTieAnalysis1D, WellTieObservationSet1D,
    WellTimeDepthAuthoredModel1D, WellTimeDepthModel1D, resolve_dataset_summary_survey_map_source,
};
use seis_contracts_operations::datasets::{
    LoadWorkspaceStateResponse, OpenDatasetRequest, OpenDatasetResponse, RemoveDatasetEntryRequest,
    RemoveDatasetEntryResponse, SetActiveDatasetEntryRequest, SetActiveDatasetEntryResponse,
    UpsertDatasetEntryRequest, UpsertDatasetEntryResponse,
};
use seis_contracts_operations::import_ops::{
    DeleteSegyImportRecipeRequest, DeleteSegyImportRecipeResponse, ExportSegyRequest,
    ExportSegyResponse, ImportDatasetRequest, ImportDatasetResponse, ImportHorizonXyzRequest,
    ImportHorizonXyzResponse, ImportPrestackOffsetDatasetRequest,
    ImportPrestackOffsetDatasetResponse, ImportSegyWithPlanRequest, ImportSegyWithPlanResponse,
    ListSegyImportRecipesRequest, ListSegyImportRecipesResponse, LoadSectionHorizonsResponse,
    SaveSegyImportRecipeRequest, SaveSegyImportRecipeResponse, ScanSegyImportRequest,
    SegyImportScanResponse, SegyImportValidationResponse, SurveyPreflightRequest,
    SurveyPreflightResponse, ValidateSegyImportPlanRequest,
};
use seis_contracts_operations::processing_ops::{
    AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, CancelProcessingJobRequest,
    CancelProcessingJobResponse, DeletePipelinePresetRequest, DeletePipelinePresetResponse,
    GatherProcessingPipeline, GatherRequest, GatherView, GetProcessingJobRequest,
    GetProcessingJobResponse, ListPipelinePresetsResponse, PreviewGatherProcessingRequest,
    PreviewGatherProcessingResponse, PreviewSubvolumeProcessingRequest,
    PreviewSubvolumeProcessingResponse, PreviewTraceLocalProcessingRequest,
    PreviewTraceLocalProcessingResponse, RunGatherProcessingRequest, RunGatherProcessingResponse,
    RunSubvolumeProcessingRequest, RunSubvolumeProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SavePipelinePresetRequest, SavePipelinePresetResponse,
    VelocityScanRequest, VelocityScanResponse,
};
use seis_contracts_operations::resolve::{
    BuildSurveyTimeDepthTransformRequest, IPC_SCHEMA_VERSION, ResolveSurveyMapRequest,
    ResolveSurveyMapResponse, SetDatasetNativeCoordinateReferenceResponse,
};
use seis_contracts_operations::workspace::{
    DescribeVelocityVolumeRequest, DescribeVelocityVolumeResponse, IngestVelocityVolumeRequest,
    IngestVelocityVolumeResponse, LoadVelocityModelsResponse, SaveWorkspaceSessionRequest,
    SaveWorkspaceSessionResponse,
};
use seis_runtime::{
    ImportedHorizonDescriptor, MaterializeOptions, ProcessingArtifactRole, ProcessingJobArtifact,
    ProcessingJobArtifactKind, ProcessingPipelineSpec, SectionAxis, SectionHorizonOverlayView,
    SectionTileView, SectionView, SubvolumeProcessingPipeline, TbvolManifest, TimeDepthDomain,
    TraceLocalProcessingPipeline, VelocityFunctionSource, VelocityQuantityKind,
    materialize_gather_processing_store_with_progress, materialize_processing_volume_with_progress,
    materialize_subvolume_processing_volume_with_progress, open_store,
    set_any_store_native_coordinate_reference,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::Instant,
};
use tauri::{
    AppHandle, Emitter, Manager, State,
    ipc::Response,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
};
use traceboost_app::{
    ExportZarrResponse, TraceBoostWorkflowService, amplitude_spectrum,
    build_velocity_model_transform, convert_horizon_domain, default_export_segy_path,
    default_export_zarr_path, export_dataset_zarr, import_horizon_xyz,
    load_depth_converted_section, load_gather, load_horizon_assets, load_resolved_section_display,
    open_dataset_summary, preview_gather_processing, preview_subvolume_processing,
    run_velocity_scan,
};

use crate::app_paths::{AppPaths, preferred_traceboost_logs_dir};
use crate::crs_registry::{
    CoordinateReferenceCatalogEntry, ResolveCoordinateReferenceRequest,
    SearchCoordinateReferencesRequest, SearchCoordinateReferencesResponse,
    resolve_coordinate_reference, search_coordinate_references,
};
use crate::diagnostics::{DiagnosticsState, ExportBundleResponse, build_fields, json_value};
use crate::import_manager::{
    BeginImportSessionRequest, ImportManagerState, ImportSessionEnvelope,
    ListImportProvidersResponse,
};
use crate::preview_session::PreviewSessionState;
use crate::processing::{JobRecord, ProcessingState};
use crate::processing_cache::ProcessingCacheState;
use crate::project_settings::{
    ProjectDisplayCoordinateReference, ProjectGeospatialSettings, load_project_geospatial_settings,
    save_project_geospatial_settings,
};
use crate::segy_import_recipes::SegyImportRecipeState;
use crate::workspace::WorkspaceState;

const FILE_OPEN_VOLUME_MENU_ID: &str = "file.open_volume";
const FILE_OPEN_VOLUME_MENU_EVENT: &str = "menu:file-open-volume";
const FILE_IMPORT_DATA_MENU_ID: &str = "file.import_data";
const FILE_IMPORT_DATA_MENU_EVENT: &str = "menu:file-import-data";
const APP_SETTINGS_MENU_ID: &str = "app.settings";
const APP_SETTINGS_MENU_EVENT: &str = "menu:app-settings";
const APP_VELOCITY_MODEL_MENU_ID: &str = "app.velocity_model";
const APP_VELOCITY_MODEL_MENU_EVENT: &str = "menu:app-velocity-model";
const APP_RESIDUALS_MENU_ID: &str = "app.residuals";
const APP_RESIDUALS_MENU_EVENT: &str = "menu:app-residuals";
const APP_DEPTH_CONVERSION_MENU_ID: &str = "app.depth_conversion";
const APP_DEPTH_CONVERSION_MENU_EVENT: &str = "menu:app-depth-conversion";
const APP_WELL_TIE_MENU_ID: &str = "app.well_tie";
const APP_WELL_TIE_MENU_EVENT: &str = "menu:app-well-tie";
const FILE_IMPORT_SEISMIC_MENU_ID: &str = "file.import_seismic";
const FILE_IMPORT_SEISMIC_MENU_EVENT: &str = "menu:file-import-seismic";
const FILE_IMPORT_HORIZONS_MENU_ID: &str = "file.import_horizons";
const FILE_IMPORT_HORIZONS_MENU_EVENT: &str = "menu:file-import-horizons";
const FILE_IMPORT_WELL_SOURCES_MENU_ID: &str = "file.import_well_sources";
const FILE_IMPORT_WELL_SOURCES_MENU_EVENT: &str = "menu:file-import-well-sources";
const FILE_IMPORT_VELOCITY_FUNCTIONS_MENU_ID: &str = "file.import_velocity_functions";
const FILE_IMPORT_VELOCITY_FUNCTIONS_MENU_EVENT: &str = "menu:file-import-velocity-functions";
const FILE_IMPORT_CHECKSHOT_MENU_ID: &str = "file.import_checkshot";
const FILE_IMPORT_CHECKSHOT_MENU_EVENT: &str = "menu:file-import-checkshot";
const FILE_IMPORT_MANUAL_PICKS_MENU_ID: &str = "file.import_manual_picks";
const FILE_IMPORT_MANUAL_PICKS_MENU_EVENT: &str = "menu:file-import-manual-picks";
const FILE_IMPORT_AUTHORED_WELL_MODEL_MENU_ID: &str = "file.import_authored_well_model";
const FILE_IMPORT_AUTHORED_WELL_MODEL_MENU_EVENT: &str = "menu:file-import-authored-well-model";
const FILE_IMPORT_COMPILED_WELL_MODEL_MENU_ID: &str = "file.import_compiled_well_model";
const FILE_IMPORT_COMPILED_WELL_MODEL_MENU_EVENT: &str = "menu:file-import-compiled-well-model";
const TRACE_LOCAL_CACHE_FAMILY: &str = "trace_local";
const TBVOL_STORE_FORMAT_VERSION: &str = "tbvol-v1";
const PROCESSING_CACHE_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

fn workflow_service() -> TraceBoostWorkflowService {
    TraceBoostWorkflowService
}

#[tauri::command]
fn list_import_providers_command(
    import_manager: State<'_, ImportManagerState>,
) -> Result<ListImportProvidersResponse, String> {
    Ok(import_manager.list_providers())
}

#[tauri::command]
fn begin_import_session_command(
    import_manager: State<'_, ImportManagerState>,
    request: BeginImportSessionRequest,
) -> Result<ImportSessionEnvelope, String> {
    import_manager.begin_session(request)
}

const PACKED_PREVIEW_MAGIC: &[u8; 8] = b"TBPRV001";
const PACKED_SECTION_MAGIC: &[u8; 8] = b"TBSEC001";
const PACKED_SECTION_TILE_MAGIC: &[u8; 8] = b"TBTIL001";
const PACKED_SECTION_DISPLAY_MAGIC: &[u8; 8] = b"TBSDP001";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrontendDiagnosticsEventRequest {
    stage: String,
    level: String,
    message: String,
    fields: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunSectionBrowsingBenchmarkRequest {
    store_path: String,
    axis: SectionAxis,
    section_index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    iterations: Option<usize>,
    include_full_section_baseline: Option<bool>,
    step_offsets: Option<Vec<isize>>,
    switch_axis: Option<SectionAxis>,
    switch_section_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SectionBrowsingBenchmarkCase {
    scenario: String,
    axis: String,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    trace_step: usize,
    sample_step: usize,
    output_traces: usize,
    output_samples: usize,
    payload_bytes: u64,
    iteration_ms: Vec<f64>,
    median_ms: f64,
    mean_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunSectionBrowsingBenchmarkResponse {
    session_log_path: String,
    store_path: String,
    dataset_id: String,
    shape: [usize; 3],
    tile_shape: [usize; 3],
    axis: String,
    section_index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    iterations: usize,
    include_full_section_baseline: bool,
    step_offsets: Vec<isize>,
    switch_axis: Option<String>,
    switch_section_index: Option<usize>,
    cases: Vec<SectionBrowsingBenchmarkCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellboreRequest {
    project_root: String,
    wellbore_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetProjectWellTimeDepthModelRequest {
    project_root: String,
    wellbore_id: String,
    asset_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRootRequest {
    project_root: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellOverlayInventoryRequest {
    project_root: String,
    display_coordinate_reference_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveProjectGeospatialSettingsRequest {
    project_root: String,
    display_coordinate_reference: ProjectDisplayCoordinateReference,
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadProjectGeospatialSettingsResponse {
    settings: Option<ProjectGeospatialSettings>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetDatasetNativeCoordinateReferenceSelectionRequest {
    store_path: String,
    coordinate_reference_id: Option<String>,
    coordinate_reference_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAssetRequest {
    project_root: String,
    asset_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveProjectSurveyMapRequest {
    project_root: String,
    survey_asset_id: String,
    wellbore_id: Option<String>,
    display_coordinate_reference_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveProjectSurveyMapResponse {
    survey_map: ophiolite::ResolvedSurveyMapSourceDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportProjectWellTimeDepthModelRequest {
    project_root: String,
    json_path: String,
    binding: AssetBindingInput,
    collection_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportProjectWellTimeDepthAssetRequest {
    project_root: String,
    json_path: String,
    json_payload: Option<String>,
    binding: AssetBindingInput,
    collection_name: Option<String>,
    asset_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewProjectWellTimeDepthAssetRequest {
    json_path: String,
    json_payload: Option<String>,
    asset_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellTimeDepthImportCanonicalDraft {
    asset_kind: String,
    json_payload: String,
    collection_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewProjectWellTimeDepthImportRequest {
    json_path: String,
    draft: Option<ProjectWellTimeDepthImportCanonicalDraft>,
    asset_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitProjectWellTimeDepthImportRequest {
    project_root: String,
    json_path: String,
    binding: AssetBindingInput,
    draft: ProjectWellTimeDepthImportCanonicalDraft,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewProjectWellImportRequest {
    folder_path: String,
    source_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewProjectWellSourceImportRequest {
    source_root_path: String,
    source_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewHorizonSourceImportRequest {
    store_path: String,
    input_paths: Vec<String>,
    draft: Option<seis_runtime::HorizonSourceImportCanonicalDraft>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitHorizonSourceImportRequest {
    store_path: String,
    draft: seis_runtime::HorizonSourceImportCanonicalDraft,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellSourceImportTopsCanonicalDraft {
    depth_reference: Option<String>,
    rows: Vec<ophiolite::WellSourceTopDraftRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellSourceImportTrajectoryCanonicalDraft {
    enabled: bool,
    rows: Option<Vec<ophiolite::WellSourceTrajectoryDraftRow>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellSourceImportPlanCanonicalDraft {
    selected_log_source_paths: Option<Vec<String>>,
    ascii_log_imports: Option<Vec<ophiolite::WellSourceAsciiLogImportRequest>>,
    tops_markers: Option<ProjectWellSourceImportTopsCanonicalDraft>,
    trajectory: Option<ProjectWellSourceImportTrajectoryCanonicalDraft>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellSourceImportCanonicalDraft {
    binding: AssetBindingInput,
    source_coordinate_reference: ophiolite::WellSourceCoordinateReferenceSelection,
    well_metadata: Option<ophiolite::WellMetadata>,
    wellbore_metadata: Option<ophiolite::WellboreMetadata>,
    import_plan: ProjectWellSourceImportPlanCanonicalDraft,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitProjectWellImportRequest {
    project_root: String,
    folder_path: String,
    source_paths: Option<Vec<String>>,
    draft: Option<ProjectWellSourceImportCanonicalDraft>,
    binding: AssetBindingInput,
    well_metadata: Option<ophiolite::WellMetadata>,
    wellbore_metadata: Option<ophiolite::WellboreMetadata>,
    source_coordinate_reference: ophiolite::WellFolderCoordinateReferenceSelection,
    import_logs: bool,
    selected_log_source_paths: Option<Vec<String>>,
    import_tops_markers: bool,
    import_trajectory: bool,
    tops_depth_reference: Option<String>,
    tops_rows: Option<Vec<ophiolite::WellFolderTopDraftRow>>,
    trajectory_rows: Option<Vec<ophiolite::WellFolderTrajectoryDraftRow>>,
    ascii_log_imports: Option<Vec<ophiolite::WellFolderAsciiLogImportRequest>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitProjectWellSourceImportRequest {
    project_root: String,
    source_root_path: String,
    source_paths: Option<Vec<String>>,
    draft: Option<ProjectWellSourceImportCanonicalDraft>,
    binding: Option<AssetBindingInput>,
    well_metadata: Option<ophiolite::WellMetadata>,
    wellbore_metadata: Option<ophiolite::WellboreMetadata>,
    source_coordinate_reference: Option<ophiolite::WellSourceCoordinateReferenceSelection>,
    import_logs: Option<bool>,
    selected_log_source_paths: Option<Vec<String>>,
    import_tops_markers: Option<bool>,
    import_trajectory: Option<bool>,
    tops_depth_reference: Option<String>,
    tops_rows: Option<Vec<ophiolite::WellSourceTopDraftRow>>,
    trajectory_rows: Option<Vec<ophiolite::WellSourceTrajectoryDraftRow>>,
    ascii_log_imports: Option<Vec<ophiolite::WellSourceAsciiLogImportRequest>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompileProjectWellTimeDepthAuthoredModelRequest {
    project_root: String,
    asset_id: String,
    output_collection_name: Option<String>,
    set_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeProjectWellTieRequest {
    project_root: String,
    source_model_asset_id: String,
    tie_name: String,
    tie_start_ms: f64,
    tie_end_ms: f64,
    search_radius_m: f64,
    store_path: String,
    survey_asset_id: String,
    display_coordinate_reference_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcceptProjectWellTieRequest {
    project_root: String,
    binding: AssetBindingInput,
    source_model_asset_id: String,
    tie_name: String,
    tie_start_ms: f64,
    tie_end_ms: f64,
    search_radius_m: f64,
    store_path: String,
    survey_asset_id: String,
    display_coordinate_reference_id: String,
    output_collection_name: Option<String>,
    set_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellTieAnalysisResponse {
    draft_observation_set: WellTieObservationSet1D,
    analysis: WellTieAnalysis1D,
    source_model_asset_id: String,
    source_model_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcceptProjectWellTieResponse {
    observation_asset_id: String,
    authored_model_asset_id: String,
    compiled_model_asset_id: String,
}

#[derive(Debug, Clone)]
struct StoreBackedWellTieSelection {
    axis: SectionAxis,
    section_index: usize,
    well_trace_index: usize,
    trace_spacing_m: f64,
}

#[derive(Debug, Clone)]
struct StoreBackedWellTieMatch {
    lateral_offset_m: f32,
    bulk_shift_samples: i32,
    stretch_factor: f32,
    correlation: f32,
    aligned_trace_amplitudes: Vec<f32>,
}

fn enrich_well_tie_analysis_with_store(
    project: &OphioliteProject,
    source_model_asset_id: &ophiolite::AssetId,
    analysis: &WellTieAnalysis1D,
    store_path: &str,
    survey_asset_id: &str,
    display_coordinate_reference_id: &str,
    search_radius_m: f64,
) -> Result<WellTieAnalysis1D, String> {
    let source_asset = project
        .asset_record(source_model_asset_id)
        .map_err(|error| error.to_string())?;
    let resolved = project
        .resolve_survey_map_source(&ProjectSurveyMapRequestDto {
            schema_version: SURVEY_MAP_CONTRACT_VERSION,
            survey_asset_ids: vec![survey_asset_id.to_string()],
            wellbore_ids: vec![source_asset.wellbore_id.0.clone()],
            display_coordinate_reference_id: display_coordinate_reference_id.to_string(),
        })
        .map_err(|error| error.to_string())?;
    let survey = resolved.surveys.first().ok_or_else(|| {
        format!("survey asset '{survey_asset_id}' did not resolve to a survey map")
    })?;
    let well = resolved
        .wells
        .iter()
        .find(|well| well.wellbore_id == source_asset.wellbore_id.0)
        .or_else(|| resolved.wells.first())
        .ok_or_else(|| {
            format!(
                "wellbore '{}' could not be resolved into the selected survey map",
                source_asset.wellbore_id.0
            )
        })?;
    let selection = select_store_backed_well_tie_section(survey, well)?;
    let handle = open_store(store_path).map_err(|error| error.to_string())?;
    let plane = handle
        .read_section_plane(selection.axis, selection.section_index)
        .map_err(|error| error.to_string())?;
    if plane.traces == 0 || plane.samples == 0 {
        return Err("selected store section does not contain any seismic samples".to_string());
    }

    let well_trace_index = selection
        .well_trace_index
        .min(plane.traces.saturating_sub(1));
    let half_window_traces = if search_radius_m.is_finite() && search_radius_m > 0.0 {
        ((search_radius_m / selection.trace_spacing_m.max(1.0)).ceil() as usize).clamp(4, 24)
    } else {
        4
    };
    let window_start = well_trace_index.saturating_sub(half_window_traces);
    let window_end = (well_trace_index + half_window_traces + 1).min(plane.traces);
    let tie_times_ms = analysis.synthetic_trace.times_ms.clone();
    let tie_amplitudes = analysis.synthetic_trace.amplitudes.clone();
    let sample_step_ms = mean_sample_step_ms(&tie_times_ms).unwrap_or(4.0).max(1.0);
    let max_lag_samples = ((48.0 / sample_step_ms).round() as usize).clamp(1, 16);

    let mut section_amplitudes =
        Vec::with_capacity((window_end - window_start) * tie_times_ms.len());
    let mut well_trace_amplitudes = Vec::new();
    let mut best_match: Option<StoreBackedWellTieMatch> = None;

    for trace_index in window_start..window_end {
        let resampled = resample_section_trace(
            &plane.sample_axis_ms,
            trace_slice(&plane.amplitudes, plane.samples, trace_index),
            &tie_times_ms,
        );
        if trace_index == well_trace_index {
            well_trace_amplitudes = resampled.clone();
        }

        let lateral_offset_m = ((trace_index as isize - well_trace_index as isize) as f32)
            * selection.trace_spacing_m as f32;
        let has_samples = plane
            .occupancy
            .as_ref()
            .and_then(|occupancy| occupancy.get(trace_index))
            .copied()
            .unwrap_or(1)
            != 0;
        if has_samples {
            if let Some(match_result) = best_shift_stretch_match(
                &tie_times_ms,
                &tie_amplitudes,
                &resampled,
                max_lag_samples,
                sample_step_ms,
            ) {
                let replace = best_match
                    .as_ref()
                    .map(|current| {
                        match_result.correlation > current.correlation + 0.001
                            || ((match_result.correlation - current.correlation).abs() <= 0.001
                                && (match_result.stretch_factor - 1.0).abs()
                                    < (current.stretch_factor - 1.0).abs())
                    })
                    .unwrap_or(true);
                if replace {
                    best_match = Some(StoreBackedWellTieMatch {
                        lateral_offset_m,
                        bulk_shift_samples: match_result.bulk_shift_samples,
                        stretch_factor: match_result.stretch_factor,
                        correlation: match_result.correlation,
                        aligned_trace_amplitudes: match_result.aligned_trace_amplitudes,
                    });
                }
            }
        }

        section_amplitudes.extend(resampled);
    }

    if well_trace_amplitudes.is_empty() {
        return Err("failed to extract the nearest seismic trace at the well location".to_string());
    }
    let Some(best_match) = best_match else {
        return Err(
            "failed to identify a seismic trace with usable correlation against the synthetic"
                .to_string(),
        );
    };

    let bulk_shift_ms = best_match.bulk_shift_samples as f32 * sample_step_ms;
    let stretch_factor = best_match.stretch_factor;
    let local_well_trace_index = well_trace_index - window_start;
    let section_label = format!(
        "Local {} {:.0}",
        match selection.axis {
            SectionAxis::Inline => "Inline",
            SectionAxis::Xline => "Xline",
        },
        plane.coordinate_value
    );
    let trace_offsets_m = (window_start..window_end)
        .map(|trace_index| {
            ((trace_index as isize - well_trace_index as isize) as f32)
                * selection.trace_spacing_m as f32
        })
        .collect::<Vec<_>>();

    let mut enriched = analysis.clone();
    let mut observation_set = enriched.draft_observation_set.clone();
    observation_set.bulk_shift_ms = Some(bulk_shift_ms);
    observation_set.stretch_factor = Some(stretch_factor);
    observation_set.trace_search_offset_m = Some(best_match.lateral_offset_m);
    observation_set.correlation = Some(best_match.correlation);
    let tie_midpoint_ms = time_axis_midpoint_ms(&tie_times_ms).unwrap_or_else(|| {
        let first = tie_times_ms.first().copied().unwrap_or_default();
        let last = tie_times_ms.last().copied().unwrap_or(first);
        (first + last) * 0.5
    });
    for (index, sample) in observation_set.samples.iter_mut().enumerate() {
        let base_time_ms = tie_times_ms
            .get(index)
            .copied()
            .map(f64::from)
            .unwrap_or(sample.time_ms);
        sample.time_ms = f64::from(tie_midpoint_ms)
            + (base_time_ms - f64::from(tie_midpoint_ms)) * f64::from(stretch_factor)
            + f64::from(bulk_shift_ms);
        sample.quality = Some(best_match.correlation);
    }
    observation_set.notes.push(format!(
        "Survey-backed seismic extracted from '{}' using survey asset '{}' and {} {:.0}.",
        store_path,
        survey_asset_id,
        match selection.axis {
            SectionAxis::Inline => "inline",
            SectionAxis::Xline => "xline",
        },
        plane.coordinate_value
    ));
    observation_set.notes.push(format!(
        "Best seismic match selected at {:+.0} m with {:.2} correlation, {:+.0} ms bulk shift, and {:.3}x stretch.",
        best_match.lateral_offset_m, best_match.correlation, bulk_shift_ms, stretch_factor
    ));

    let tie_length_ms = tie_times_ms
        .first()
        .zip(tie_times_ms.last())
        .map(|(start, end)| *end - *start)
        .unwrap_or(0.0)
        .abs();
    enriched.best_match_trace = ophiolite::WellTieTrace1D {
        id: "best-seismic".to_string(),
        label: "Best Seis".to_string(),
        times_ms: tie_times_ms.clone(),
        amplitudes: best_match.aligned_trace_amplitudes.clone(),
    };
    enriched.well_trace = ophiolite::WellTieTrace1D {
        id: "well-seismic".to_string(),
        label: "Well Seis".to_string(),
        times_ms: tie_times_ms.clone(),
        amplitudes: well_trace_amplitudes,
    };
    enriched.section_window = ophiolite::WellTieSectionWindow {
        id: "local-seismic-window".to_string(),
        label: section_label,
        times_ms: tie_times_ms.clone(),
        trace_offsets_m,
        amplitudes: section_amplitudes,
        trace_count: window_end - window_start,
        sample_count: enriched.synthetic_trace.amplitudes.len(),
        well_trace_index: local_well_trace_index,
    };
    if let Some(extracted_wavelet) = estimate_extracted_wavelet(
        &enriched.reflectivity_trace.amplitudes,
        &best_match.aligned_trace_amplitudes,
        sample_step_ms,
        tie_length_ms,
    ) {
        let extracted_synthetic = convolve_same_normalized(
            &enriched.reflectivity_trace.amplitudes,
            &extracted_wavelet.amplitudes,
        );
        if let Some(extracted_correlation) = correlation_with_synthetic_lag(
            &extracted_synthetic,
            &best_match.aligned_trace_amplitudes,
            0,
        ) {
            observation_set.correlation = Some(extracted_correlation);
            for sample in &mut observation_set.samples {
                sample.quality = Some(extracted_correlation);
            }
            enriched.notes.push(format!(
                "Least-squares wavelet extraction updated the synthetic/seismic correlation to {:.2}.",
                extracted_correlation
            ));
        } else {
            enriched.notes.push(
                "Least-squares wavelet extraction ran, but the updated synthetic correlation could not be scored."
                    .to_string(),
            );
        }
        enriched.wavelet = extracted_wavelet;
        enriched.synthetic_trace = ophiolite::WellTieTrace1D {
            id: "synthetic-extracted-wavelet".to_string(),
            label: "Syn".to_string(),
            times_ms: tie_times_ms.clone(),
            amplitudes: extracted_synthetic,
        };
        observation_set.notes.push(
            "Synthetic updated with a least-squares extracted wavelet estimated from the matched seismic trace."
                .to_string(),
        );
    } else {
        enriched.notes.push(
            "Wavelet extraction remained provisional because the least-squares estimate was unstable for this tie window."
                .to_string(),
        );
    }
    enriched
        .notes
        .retain(|note| !note.contains("remain provisional"));
    enriched.notes.push(format!(
        "Seismic traces and the local seismic window are extracted from the active store '{}' using survey asset '{}'.",
        store_path, survey_asset_id
    ));
    enriched.notes.push(format!(
        "Best-match trace preview is aligned with a solved {:+.0} ms bulk shift and {:.3}x stretch; the local seismic window remains in survey time.",
        bulk_shift_ms, stretch_factor
    ));
    enriched.draft_observation_set = observation_set;

    Ok(enriched)
}

fn select_store_backed_well_tie_section(
    survey: &ophiolite::ResolvedSurveyMapSurveyDto,
    well: &ophiolite::ResolvedSurveyMapWellDto,
) -> Result<StoreBackedWellTieSelection, String> {
    let grid_transform = survey
        .display_spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.as_ref())
        .or_else(|| survey.native_spatial.grid_transform.as_ref())
        .ok_or_else(|| {
            format!(
                "survey '{}' does not expose a grid transform in the selected display/native map space",
                survey.name
            )
        })?;
    let surface_location = well.surface_location.as_ref().ok_or_else(|| {
        format!(
            "wellbore '{}' does not expose a survey-map surface location",
            well.wellbore_id
        )
    })?;
    let (inline_ordinal, xline_ordinal) =
        invert_well_tie_grid_transform(grid_transform, surface_location.x, surface_location.y)
            .ok_or_else(|| {
                "survey grid transform could not be inverted at the well location".to_string()
            })?;
    let inline_spacing_m = vector_length_m(&grid_transform.inline_basis).max(1.0);
    let xline_spacing_m = vector_length_m(&grid_transform.xline_basis).max(1.0);

    let axis = if xline_spacing_m <= inline_spacing_m {
        SectionAxis::Inline
    } else {
        SectionAxis::Xline
    };
    let section_index = match axis {
        SectionAxis::Inline => {
            clamp_rounded_index(inline_ordinal, survey.index_grid.inline_axis.count)
        }
        SectionAxis::Xline => {
            clamp_rounded_index(xline_ordinal, survey.index_grid.xline_axis.count)
        }
    }
    .ok_or_else(|| "well location falls outside the survey section axis bounds".to_string())?;
    let well_trace_index = match axis {
        SectionAxis::Inline => {
            clamp_rounded_index(xline_ordinal, survey.index_grid.xline_axis.count)
        }
        SectionAxis::Xline => {
            clamp_rounded_index(inline_ordinal, survey.index_grid.inline_axis.count)
        }
    }
    .ok_or_else(|| "well location falls outside the survey trace axis bounds".to_string())?;
    let trace_spacing_m = match axis {
        SectionAxis::Inline => xline_spacing_m,
        SectionAxis::Xline => inline_spacing_m,
    };

    Ok(StoreBackedWellTieSelection {
        axis,
        section_index,
        well_trace_index,
        trace_spacing_m,
    })
}

fn invert_well_tie_grid_transform(
    transform: &ophiolite::SurveyMapGridTransformDto,
    x: f64,
    y: f64,
) -> Option<(f64, f64)> {
    let determinant = transform.inline_basis.x * transform.xline_basis.y
        - transform.inline_basis.y * transform.xline_basis.x;
    if determinant.abs() <= f64::EPSILON {
        return None;
    }

    let dx = x - transform.origin.x;
    let dy = y - transform.origin.y;
    let inline_ordinal =
        (dx * transform.xline_basis.y - dy * transform.xline_basis.x) / determinant;
    let xline_ordinal =
        (dy * transform.inline_basis.x - dx * transform.inline_basis.y) / determinant;
    Some((inline_ordinal, xline_ordinal))
}

fn clamp_rounded_index(value: f64, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let rounded = value.round();
    if !rounded.is_finite() || rounded < 0.0 {
        return None;
    }
    Some((rounded as usize).min(len.saturating_sub(1)))
}

fn vector_length_m(vector: &ophiolite::ProjectedVector2Dto) -> f64 {
    (vector.x * vector.x + vector.y * vector.y).sqrt()
}

fn trace_slice(amplitudes: &[f32], samples: usize, trace_index: usize) -> &[f32] {
    let start = trace_index * samples;
    &amplitudes[start..start + samples]
}

fn resample_section_trace(
    sample_axis_ms: &[f32],
    samples: &[f32],
    target_times_ms: &[f32],
) -> Vec<f32> {
    target_times_ms
        .iter()
        .map(|time_ms| interpolate_trace_sample(sample_axis_ms, samples, *time_ms))
        .collect()
}

fn interpolate_trace_sample(sample_axis_ms: &[f32], samples: &[f32], target_time_ms: f32) -> f32 {
    let Some(first_time) = sample_axis_ms.first().copied() else {
        return 0.0;
    };
    let Some(last_time) = sample_axis_ms.last().copied() else {
        return 0.0;
    };
    if target_time_ms <= first_time {
        return samples.first().copied().unwrap_or(0.0);
    }
    if target_time_ms >= last_time {
        return samples.last().copied().unwrap_or(0.0);
    }

    let upper_index = sample_axis_ms.partition_point(|value| *value < target_time_ms);
    if upper_index == 0 || upper_index >= sample_axis_ms.len() {
        return samples
            .get(upper_index.min(samples.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);
    }
    let lower_index = upper_index - 1;
    let start_time = sample_axis_ms[lower_index];
    let end_time = sample_axis_ms[upper_index];
    let start_value = samples.get(lower_index).copied().unwrap_or(0.0);
    let end_value = samples.get(upper_index).copied().unwrap_or(start_value);
    let weight = if (end_time - start_time).abs() <= f32::EPSILON {
        0.0
    } else {
        (target_time_ms - start_time) / (end_time - start_time)
    };
    start_value + (end_value - start_value) * weight
}

fn mean_sample_step_ms(times_ms: &[f32]) -> Option<f32> {
    if times_ms.len() < 2 {
        return None;
    }
    let mut sum = 0.0_f32;
    let mut count = 0_usize;
    for pair in times_ms.windows(2) {
        let delta = pair[1] - pair[0];
        if delta.is_finite() && delta > 0.0 {
            sum += delta;
            count += 1;
        }
    }
    (count > 0).then_some(sum / count as f32)
}

#[derive(Debug, Clone)]
struct ShiftStretchMatch {
    bulk_shift_samples: i32,
    stretch_factor: f32,
    correlation: f32,
    aligned_trace_amplitudes: Vec<f32>,
}

fn best_bulk_shift_match(
    synthetic: &[f32],
    seismic: &[f32],
    max_lag_samples: usize,
) -> Option<(f32, i32)> {
    let mut best: Option<(f32, i32)> = None;
    let max_lag = max_lag_samples as i32;
    for lag in -max_lag..=max_lag {
        let Some(correlation) = correlation_with_synthetic_lag(synthetic, seismic, lag) else {
            continue;
        };
        let replace = best
            .as_ref()
            .map(|(best_corr, _)| correlation > *best_corr)
            .unwrap_or(true);
        if replace {
            best = Some((correlation, lag));
        }
    }
    best
}

fn best_shift_stretch_match(
    times_ms: &[f32],
    synthetic: &[f32],
    seismic: &[f32],
    max_lag_samples: usize,
    sample_step_ms: f32,
) -> Option<ShiftStretchMatch> {
    let midpoint_ms = time_axis_midpoint_ms(times_ms)?;
    let (baseline_correlation, baseline_lag_samples) =
        best_bulk_shift_match(synthetic, seismic, max_lag_samples)?;
    let baseline = ShiftStretchMatch {
        bulk_shift_samples: baseline_lag_samples,
        stretch_factor: 1.0,
        correlation: baseline_correlation,
        aligned_trace_amplitudes: align_trace_with_affine_time_correction(
            times_ms,
            seismic,
            times_ms,
            midpoint_ms,
            1.0,
            baseline_lag_samples as f32 * sample_step_ms,
        ),
    };
    let mut stretch_factors = Vec::with_capacity(17);
    for step in -8..=8 {
        stretch_factors.push(1.0 + step as f32 * 0.005);
    }
    let candidate = best_affine_match_for_stretch_factors(
        times_ms,
        synthetic,
        seismic,
        max_lag_samples,
        sample_step_ms,
        &stretch_factors,
    )?;
    let material_improvement = candidate.correlation - baseline.correlation >= 0.01;
    let meaningful_stretch = (candidate.stretch_factor - 1.0).abs() >= 0.005;
    if material_improvement && meaningful_stretch {
        Some(candidate)
    } else {
        Some(baseline)
    }
}

fn best_affine_match_for_stretch_factors(
    times_ms: &[f32],
    synthetic: &[f32],
    seismic: &[f32],
    max_lag_samples: usize,
    sample_step_ms: f32,
    stretch_factors: &[f32],
) -> Option<ShiftStretchMatch> {
    let midpoint_ms = time_axis_midpoint_ms(times_ms)?;
    let max_lag = max_lag_samples as i32;
    let mut best: Option<ShiftStretchMatch> = None;
    for &stretch_factor in stretch_factors {
        for lag_samples in -max_lag..=max_lag {
            let bulk_shift_ms = lag_samples as f32 * sample_step_ms;
            let aligned_trace = align_trace_with_affine_time_correction(
                times_ms,
                seismic,
                times_ms,
                midpoint_ms,
                stretch_factor,
                bulk_shift_ms,
            );
            let Some(correlation) = correlation_with_synthetic_lag(synthetic, &aligned_trace, 0)
            else {
                continue;
            };
            let replace = best
                .as_ref()
                .map(|current| {
                    correlation > current.correlation + 0.0005
                        || ((correlation - current.correlation).abs() <= 0.0005
                            && (stretch_factor - 1.0).abs() < (current.stretch_factor - 1.0).abs())
                        || ((correlation - current.correlation).abs() <= 0.0005
                            && ((stretch_factor - 1.0).abs()
                                - (current.stretch_factor - 1.0).abs())
                            .abs()
                                <= 0.0005
                            && lag_samples.abs() < current.bulk_shift_samples.abs())
                })
                .unwrap_or(true);
            if replace {
                best = Some(ShiftStretchMatch {
                    bulk_shift_samples: lag_samples,
                    stretch_factor,
                    correlation,
                    aligned_trace_amplitudes: aligned_trace,
                });
            }
        }
    }
    best
}

fn correlation_with_synthetic_lag(
    synthetic: &[f32],
    seismic: &[f32],
    lag_samples: i32,
) -> Option<f32> {
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut sum_x2 = 0.0_f64;
    let mut sum_y2 = 0.0_f64;
    let mut sum_xy = 0.0_f64;
    let mut count = 0_usize;

    for (index, &seismic_value) in seismic.iter().enumerate() {
        let synthetic_index = index as i32 - lag_samples;
        if synthetic_index < 0 || synthetic_index >= synthetic.len() as i32 {
            continue;
        }
        let synthetic_value = synthetic[synthetic_index as usize];
        let x = f64::from(synthetic_value);
        let y = f64::from(seismic_value);
        sum_x += x;
        sum_y += y;
        sum_x2 += x * x;
        sum_y2 += y * y;
        sum_xy += x * y;
        count += 1;
    }

    if count < 8 {
        return None;
    }
    let count_f64 = count as f64;
    let numerator = count_f64 * sum_xy - sum_x * sum_y;
    let denominator_x = count_f64 * sum_x2 - sum_x * sum_x;
    let denominator_y = count_f64 * sum_y2 - sum_y * sum_y;
    let denominator = (denominator_x * denominator_y).sqrt();
    if denominator <= f64::EPSILON {
        return None;
    }
    Some((numerator / denominator) as f32)
}

fn estimate_extracted_wavelet(
    reflectivity: &[f32],
    seismic: &[f32],
    sample_step_ms: f32,
    tie_length_ms: f32,
) -> Option<ophiolite::WellTieWavelet> {
    if reflectivity.len() < 16 || seismic.len() < 16 || sample_step_ms <= 0.0 {
        return None;
    }
    let target_wavelet_length_ms = (tie_length_ms / 4.0).clamp(96.0, 160.0);
    let half_samples = ((target_wavelet_length_ms / sample_step_ms) * 0.5)
        .round()
        .max(8.0) as usize;
    let half_samples = half_samples.clamp(8, 24);
    let coefficient_count = half_samples * 2 + 1;
    let mut ata = vec![0.0_f64; coefficient_count * coefficient_count];
    let mut atb = vec![0.0_f64; coefficient_count];

    for sample_index in 0..seismic.len() {
        let target = f64::from(*seismic.get(sample_index).unwrap_or(&0.0));
        for column in 0..coefficient_count {
            let source_index = sample_index as isize + column as isize - half_samples as isize;
            let x_column = reflectivity
                .get(source_index.max(0) as usize)
                .copied()
                .filter(|_| source_index >= 0 && source_index < reflectivity.len() as isize)
                .unwrap_or(0.0) as f64;
            atb[column] += x_column * target;
            for row in column..coefficient_count {
                let other_index = sample_index as isize + row as isize - half_samples as isize;
                let x_row = reflectivity
                    .get(other_index.max(0) as usize)
                    .copied()
                    .filter(|_| other_index >= 0 && other_index < reflectivity.len() as isize)
                    .unwrap_or(0.0) as f64;
                ata[row * coefficient_count + column] += x_row * x_column;
                if row != column {
                    ata[column * coefficient_count + row] = ata[row * coefficient_count + column];
                }
            }
        }
    }

    let diagonal_mean = (0..coefficient_count)
        .map(|index| ata[index * coefficient_count + index])
        .sum::<f64>()
        / coefficient_count as f64;
    let ridge = diagonal_mean.max(1.0) * 1.0e-3;
    for index in 0..coefficient_count {
        ata[index * coefficient_count + index] += ridge;
    }
    let solved = solve_dense_linear_system(&mut ata, &mut atb, coefficient_count)?;
    let mut amplitudes = solved
        .into_iter()
        .map(|value| value as f32)
        .collect::<Vec<_>>();
    normalize_trace_in_place(&mut amplitudes);
    if amplitudes.iter().all(|value| value.abs() <= f32::EPSILON) {
        return None;
    }
    let times_ms = (0..coefficient_count)
        .map(|index| (index as isize - half_samples as isize) as f32 * sample_step_ms)
        .collect::<Vec<_>>();
    Some(ophiolite::WellTieWavelet {
        id: "extracted-wavelet".to_string(),
        label: "Extracted Wavelet".to_string(),
        times_ms,
        amplitudes,
    })
}

fn solve_dense_linear_system(
    matrix: &mut [f64],
    rhs: &mut [f64],
    dimension: usize,
) -> Option<Vec<f64>> {
    if matrix.len() != dimension * dimension || rhs.len() != dimension {
        return None;
    }
    for pivot_index in 0..dimension {
        let mut best_row = pivot_index;
        let mut best_value = matrix[pivot_index * dimension + pivot_index].abs();
        for row in (pivot_index + 1)..dimension {
            let candidate = matrix[row * dimension + pivot_index].abs();
            if candidate > best_value {
                best_value = candidate;
                best_row = row;
            }
        }
        if best_value <= f64::EPSILON {
            return None;
        }
        if best_row != pivot_index {
            for column in 0..dimension {
                matrix.swap(
                    pivot_index * dimension + column,
                    best_row * dimension + column,
                );
            }
            rhs.swap(pivot_index, best_row);
        }
        let pivot = matrix[pivot_index * dimension + pivot_index];
        for column in pivot_index..dimension {
            matrix[pivot_index * dimension + column] /= pivot;
        }
        rhs[pivot_index] /= pivot;
        for row in 0..dimension {
            if row == pivot_index {
                continue;
            }
            let factor = matrix[row * dimension + pivot_index];
            if factor.abs() <= f64::EPSILON {
                continue;
            }
            for column in pivot_index..dimension {
                matrix[row * dimension + column] -=
                    factor * matrix[pivot_index * dimension + column];
            }
            rhs[row] -= factor * rhs[pivot_index];
        }
    }
    Some(rhs.to_vec())
}

fn time_axis_midpoint_ms(times_ms: &[f32]) -> Option<f32> {
    let first = times_ms.first().copied()?;
    let last = times_ms.last().copied().unwrap_or(first);
    Some((first + last) * 0.5)
}

fn align_trace_with_affine_time_correction(
    source_times_ms: &[f32],
    source_amplitudes: &[f32],
    target_times_ms: &[f32],
    midpoint_ms: f32,
    stretch_factor: f32,
    bulk_shift_ms: f32,
) -> Vec<f32> {
    let safe_stretch = if stretch_factor.is_finite() && stretch_factor > 0.5 {
        stretch_factor
    } else {
        1.0
    };
    target_times_ms
        .iter()
        .map(|target_time_ms| {
            let source_time_ms =
                midpoint_ms + (*target_time_ms - bulk_shift_ms - midpoint_ms) / safe_stretch;
            interpolate_trace_sample(source_times_ms, source_amplitudes, source_time_ms)
        })
        .collect()
}

fn convolve_same_normalized(signal: &[f32], kernel: &[f32]) -> Vec<f32> {
    if signal.is_empty() || kernel.is_empty() {
        return Vec::new();
    }
    let kernel_half = kernel.len() / 2;
    let mut output = vec![0.0_f32; signal.len()];
    for output_index in 0..signal.len() {
        let mut sum = 0.0_f32;
        for (kernel_index, coefficient) in kernel.iter().enumerate() {
            let signal_index = output_index as isize + kernel_index as isize - kernel_half as isize;
            if signal_index >= 0 && (signal_index as usize) < signal.len() {
                sum += signal[signal_index as usize] * coefficient;
            }
        }
        output[output_index] = sum;
    }
    normalize_trace_in_place(&mut output);
    output
}

fn normalize_trace_in_place(values: &mut [f32]) {
    let max_abs = values
        .iter()
        .fold(0.0_f32, |current, value| current.max(value.abs()));
    if max_abs <= f32::EPSILON {
        return;
    }
    for value in values {
        *value /= max_abs;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellTimeDepthModelDescriptor {
    asset_id: String,
    well_id: String,
    wellbore_id: String,
    status: String,
    name: String,
    source_kind: ophiolite::TimeDepthTransformSourceKind,
    depth_reference: ophiolite::DepthReferenceKind,
    travel_time_reference: ophiolite::TravelTimeReference,
    sample_count: usize,
    is_active_project_model: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellTimeDepthObservationDescriptor {
    asset_id: String,
    asset_kind: String,
    well_id: String,
    wellbore_id: String,
    status: String,
    name: String,
    depth_reference: ophiolite::DepthReferenceKind,
    travel_time_reference: ophiolite::TravelTimeReference,
    sample_count: usize,
    source_well_time_depth_model_asset_id: Option<String>,
    tie_window_start_ms: Option<f64>,
    tie_window_end_ms: Option<f64>,
    trace_search_radius_m: Option<f32>,
    bulk_shift_ms: Option<f32>,
    stretch_factor: Option<f32>,
    trace_search_offset_m: Option<f32>,
    correlation: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellTimeDepthAuthoredModelDescriptor {
    asset_id: String,
    well_id: String,
    wellbore_id: String,
    status: String,
    name: String,
    source_binding_count: usize,
    assumption_interval_count: usize,
    sampling_step_m: Option<f64>,
    resolved_trajectory_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellTimeDepthInventoryResponse {
    observation_sets: Vec<ProjectWellTimeDepthObservationDescriptor>,
    authored_models: Vec<ProjectWellTimeDepthAuthoredModelDescriptor>,
    compiled_models: Vec<ProjectWellTimeDepthModelDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellMarkerDescriptor {
    name: String,
    marker_kind: Option<String>,
    source_asset_id: Option<String>,
    top_depth: f64,
    base_depth: Option<f64>,
    depth_reference: Option<String>,
    source: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellMarkerHorizonResidualPointDescriptor {
    marker_name: String,
    marker_kind: Option<String>,
    x: f64,
    y: f64,
    z: f64,
    horizon_depth: f64,
    residual: f64,
    status: String,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellMarkerHorizonResidualDescriptor {
    asset_id: String,
    source_asset_id: Option<String>,
    survey_asset_id: Option<String>,
    horizon_id: Option<String>,
    marker_name: Option<String>,
    well_id: String,
    wellbore_id: String,
    status: String,
    name: String,
    row_count: usize,
    point_count: usize,
    marker_names: Vec<String>,
    points: Vec<ProjectWellMarkerHorizonResidualPointDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellMarkerResidualInventoryResponse {
    markers: Vec<ProjectWellMarkerDescriptor>,
    residual_assets: Vec<ProjectWellMarkerHorizonResidualDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportProjectWellTimeDepthModelResponse {
    asset_id: String,
    well_id: String,
    wellbore_id: String,
    created_well: bool,
    created_wellbore: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSurveyAssetDescriptor {
    asset_id: String,
    name: String,
    status: String,
    well_id: String,
    well_name: String,
    wellbore_id: String,
    wellbore_name: String,
    effective_coordinate_reference_id: Option<String>,
    effective_coordinate_reference_name: Option<String>,
    display_compatibility: ProjectSurveyDisplayCompatibility,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellboreInventoryItem {
    well_id: String,
    well_name: String,
    wellbore_id: String,
    wellbore_name: String,
    trajectory_asset_count: usize,
    well_time_depth_model_count: usize,
    active_well_time_depth_model_asset_id: Option<String>,
    display_compatibility: ProjectWellboreDisplayCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProjectSurveyDisplayReasonCode {
    ProjectDisplayCrsUnresolved,
    DisplayCrsUnsupported,
    SourceCrsUnknown,
    SourceCrsUnsupported,
    DisplayEquivalent,
    DisplayTransformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProjectWellboreDisplayReasonCode {
    ProjectDisplayCrsUnresolved,
    ResolvedGeometryMissing,
    DisplayEquivalent,
    DisplayTransformed,
    DisplayDegraded,
    DisplayUnavailable,
    ResolutionError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProjectDisplayCompatibilityBlockingReasonCode {
    ProjectDisplayCrsUnresolved,
    DisplayCrsUnsupported,
    SourceCrsUnknown,
    SourceCrsUnsupported,
    ResolvedGeometryMissing,
    DisplayUnavailable,
    ResolutionError,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSurveyDisplayCompatibility {
    can_resolve_project_map: bool,
    transform_status: SurveyMapTransformStatusDto,
    source_coordinate_reference_id: Option<String>,
    display_coordinate_reference_id: Option<String>,
    reason_code: Option<ProjectSurveyDisplayReasonCode>,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellboreDisplayCompatibility {
    can_resolve_project_map: bool,
    transform_status: SurveyMapTransformStatusDto,
    source_coordinate_reference_id: Option<String>,
    display_coordinate_reference_id: Option<String>,
    reason_code: Option<ProjectWellboreDisplayReasonCode>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectMapDisplayCompatibilitySummary {
    display_coordinate_reference_id: Option<String>,
    compatible_survey_count: usize,
    incompatible_survey_count: usize,
    compatible_wellbore_count: usize,
    incompatible_wellbore_count: usize,
    blocking_reason_codes: Vec<ProjectDisplayCompatibilityBlockingReasonCode>,
    blocking_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWellOverlayInventoryResponse {
    surveys: Vec<ProjectSurveyAssetDescriptor>,
    wellbores: Vec<ProjectWellboreInventoryItem>,
    display_compatibility: ProjectMapDisplayCompatibilitySummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComputeProjectWellMarkerResidualRequest {
    project_root: String,
    wellbore_id: String,
    survey_asset_id: String,
    horizon_id: String,
    marker_name: String,
    output_collection_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComputeProjectWellMarkerResidualResponse {
    asset_id: String,
    collection_id: String,
    collection_name: String,
    well_id: String,
    wellbore_id: String,
    marker_name: String,
    horizon_id: String,
    point_count: usize,
}

fn asset_status_label(status: &AssetStatus) -> &'static str {
    match status {
        AssetStatus::Imported => "imported",
        AssetStatus::Validated => "validated",
        AssetStatus::Bound => "bound",
        AssetStatus::NeedsReview => "needs_review",
        AssetStatus::Rejected => "rejected",
        AssetStatus::Superseded => "superseded",
    }
}

fn normalized_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn supported_epsg_identifier(value: &str) -> bool {
    value.trim().to_ascii_uppercase().starts_with("EPSG:")
}

const PROJECT_SURVEY_DISPLAY_CRS_UNRESOLVED_REASON: &str =
    "Project display CRS is unresolved. Choose a project CRS before composing project maps.";
const PROJECT_WELLBORE_DISPLAY_CRS_UNRESOLVED_REASON: &str = "Project display CRS is unresolved. Choose a project CRS before resolving well trajectories in project display coordinates.";
const PROJECT_SURVEY_DISPLAY_EQUIVALENT_REASON: &str =
    "Survey native CRS already matches the project display CRS.";
const PROJECT_WELLBORE_DISPLAY_EQUIVALENT_REASON: &str =
    "Well trajectory native CRS already matches the project display CRS.";

fn project_survey_display_compatibility(
    source_coordinate_reference_id: Option<&str>,
    display_coordinate_reference_id: Option<&str>,
) -> ProjectSurveyDisplayCompatibility {
    let normalized_display_coordinate_reference_id =
        normalized_optional_string(display_coordinate_reference_id);
    let normalized_source_coordinate_reference_id =
        normalized_optional_string(source_coordinate_reference_id);

    let Some(display_coordinate_reference_id) =
        normalized_display_coordinate_reference_id.as_deref()
    else {
        return ProjectSurveyDisplayCompatibility {
            can_resolve_project_map: false,
            transform_status: SurveyMapTransformStatusDto::NativeOnly,
            source_coordinate_reference_id: normalized_source_coordinate_reference_id,
            display_coordinate_reference_id: None,
            reason_code: Some(ProjectSurveyDisplayReasonCode::ProjectDisplayCrsUnresolved),
            reason: Some(String::from(PROJECT_SURVEY_DISPLAY_CRS_UNRESOLVED_REASON)),
        };
    };

    if !supported_epsg_identifier(display_coordinate_reference_id) {
        return ProjectSurveyDisplayCompatibility {
            can_resolve_project_map: false,
            transform_status: SurveyMapTransformStatusDto::DisplayUnavailable,
            source_coordinate_reference_id: normalized_source_coordinate_reference_id,
            display_coordinate_reference_id: Some(display_coordinate_reference_id.to_string()),
            reason_code: Some(ProjectSurveyDisplayReasonCode::DisplayCrsUnsupported),
            reason: Some(format!(
                "Display CRS '{display_coordinate_reference_id}' is not yet supported for project map reprojection; phase 2 currently accepts only EPSG identifiers."
            )),
        };
    }

    let Some(source_coordinate_reference_id) = normalized_source_coordinate_reference_id.as_deref()
    else {
        return ProjectSurveyDisplayCompatibility {
            can_resolve_project_map: false,
            transform_status: SurveyMapTransformStatusDto::DisplayUnavailable,
            source_coordinate_reference_id: None,
            display_coordinate_reference_id: Some(display_coordinate_reference_id.to_string()),
            reason_code: Some(ProjectSurveyDisplayReasonCode::SourceCrsUnknown),
            reason: Some(String::from(
                "Survey effective native CRS is unknown, so project map reprojection is unavailable.",
            )),
        };
    };

    if !supported_epsg_identifier(source_coordinate_reference_id) {
        return ProjectSurveyDisplayCompatibility {
            can_resolve_project_map: false,
            transform_status: SurveyMapTransformStatusDto::DisplayUnavailable,
            source_coordinate_reference_id: Some(source_coordinate_reference_id.to_string()),
            display_coordinate_reference_id: Some(display_coordinate_reference_id.to_string()),
            reason_code: Some(ProjectSurveyDisplayReasonCode::SourceCrsUnsupported),
            reason: Some(format!(
                "Survey effective native CRS '{source_coordinate_reference_id}' is not yet supported for project map reprojection; phase 2 currently accepts only EPSG identifiers."
            )),
        };
    }

    let transform_status =
        if source_coordinate_reference_id.eq_ignore_ascii_case(display_coordinate_reference_id) {
            SurveyMapTransformStatusDto::DisplayEquivalent
        } else {
            SurveyMapTransformStatusDto::DisplayTransformed
        };
    let reason = if matches!(
        transform_status,
        SurveyMapTransformStatusDto::DisplayEquivalent
    ) {
        Some(String::from(PROJECT_SURVEY_DISPLAY_EQUIVALENT_REASON))
    } else {
        Some(format!(
            "Survey map geometry can be reprojected from {source_coordinate_reference_id} to {display_coordinate_reference_id}."
        ))
    };

    ProjectSurveyDisplayCompatibility {
        can_resolve_project_map: true,
        transform_status,
        source_coordinate_reference_id: Some(source_coordinate_reference_id.to_string()),
        display_coordinate_reference_id: Some(display_coordinate_reference_id.to_string()),
        reason_code: Some(
            if matches!(
                transform_status,
                SurveyMapTransformStatusDto::DisplayEquivalent
            ) {
                ProjectSurveyDisplayReasonCode::DisplayEquivalent
            } else {
                ProjectSurveyDisplayReasonCode::DisplayTransformed
            },
        ),
        reason,
    }
}

fn project_wellbore_display_reason(
    compatibility: &ProjectWellboreDisplayCompatibility,
) -> Option<String> {
    match compatibility.transform_status {
        SurveyMapTransformStatusDto::NativeOnly => {
            Some(String::from(PROJECT_WELLBORE_DISPLAY_CRS_UNRESOLVED_REASON))
        }
        SurveyMapTransformStatusDto::DisplayEquivalent => {
            Some(String::from(PROJECT_WELLBORE_DISPLAY_EQUIVALENT_REASON))
        }
        SurveyMapTransformStatusDto::DisplayTransformed => compatibility
            .source_coordinate_reference_id
            .as_deref()
            .zip(compatibility.display_coordinate_reference_id.as_deref())
            .map(|(source_coordinate_reference_id, display_coordinate_reference_id)| {
                format!(
                    "Well trajectory can be reprojected from {source_coordinate_reference_id} to {display_coordinate_reference_id}."
                )
            })
            .or_else(|| Some(String::from("Well trajectory can be resolved in the project display CRS."))),
        SurveyMapTransformStatusDto::DisplayDegraded => compatibility.reason.clone().or_else(|| {
            Some(String::from(
                "Well trajectory is only partially available in the project display CRS.",
            ))
        }),
        SurveyMapTransformStatusDto::DisplayUnavailable => compatibility.reason.clone().or_else(|| {
            Some(String::from(
                "Well trajectory cannot be resolved in the project display CRS.",
            ))
        }),
    }
}

fn project_wellbore_display_compatibility(
    project: &OphioliteProject,
    wellbore_id: &str,
    display_coordinate_reference_id: Option<&str>,
) -> ProjectWellboreDisplayCompatibility {
    let normalized_display_coordinate_reference_id =
        normalized_optional_string(display_coordinate_reference_id);
    let Some(display_coordinate_reference_id) =
        normalized_display_coordinate_reference_id.as_deref()
    else {
        return ProjectWellboreDisplayCompatibility {
            can_resolve_project_map: false,
            transform_status: SurveyMapTransformStatusDto::NativeOnly,
            source_coordinate_reference_id: None,
            display_coordinate_reference_id: None,
            reason_code: Some(ProjectWellboreDisplayReasonCode::ProjectDisplayCrsUnresolved),
            reason: Some(String::from(PROJECT_WELLBORE_DISPLAY_CRS_UNRESOLVED_REASON)),
        };
    };

    let resolution = project.resolve_survey_map_source(&ProjectSurveyMapRequestDto {
        schema_version: 1,
        survey_asset_ids: Vec::new(),
        wellbore_ids: vec![wellbore_id.to_string()],
        display_coordinate_reference_id: display_coordinate_reference_id.to_string(),
    });

    match resolution {
        Ok(source) => {
            let resolved_well = source
                .wells
                .into_iter()
                .find(|well| well.wellbore_id == wellbore_id);
            let Some(resolved_well) = resolved_well else {
                return ProjectWellboreDisplayCompatibility {
                    can_resolve_project_map: false,
                    transform_status: SurveyMapTransformStatusDto::DisplayUnavailable,
                    source_coordinate_reference_id: None,
                    display_coordinate_reference_id: Some(
                        display_coordinate_reference_id.to_string(),
                    ),
                    reason_code: Some(ProjectWellboreDisplayReasonCode::ResolvedGeometryMissing),
                    reason: Some(String::from(
                        "Wellbore survey-map geometry could not be resolved for the selected project display CRS.",
                    )),
                };
            };

            let mut compatibility = ProjectWellboreDisplayCompatibility {
                can_resolve_project_map: matches!(
                    resolved_well.transform_status,
                    SurveyMapTransformStatusDto::DisplayEquivalent
                        | SurveyMapTransformStatusDto::DisplayTransformed
                        | SurveyMapTransformStatusDto::DisplayDegraded
                ),
                transform_status: resolved_well.transform_status,
                source_coordinate_reference_id: normalized_optional_string(
                    resolved_well
                        .transform_diagnostics
                        .source_coordinate_reference_id
                        .as_deref(),
                ),
                display_coordinate_reference_id: normalized_optional_string(
                    resolved_well
                        .transform_diagnostics
                        .target_coordinate_reference_id
                        .as_deref(),
                )
                .or_else(|| Some(display_coordinate_reference_id.to_string())),
                reason_code: Some(match resolved_well.transform_status {
                    SurveyMapTransformStatusDto::NativeOnly => {
                        ProjectWellboreDisplayReasonCode::ProjectDisplayCrsUnresolved
                    }
                    SurveyMapTransformStatusDto::DisplayEquivalent => {
                        ProjectWellboreDisplayReasonCode::DisplayEquivalent
                    }
                    SurveyMapTransformStatusDto::DisplayTransformed => {
                        ProjectWellboreDisplayReasonCode::DisplayTransformed
                    }
                    SurveyMapTransformStatusDto::DisplayDegraded => {
                        ProjectWellboreDisplayReasonCode::DisplayDegraded
                    }
                    SurveyMapTransformStatusDto::DisplayUnavailable => {
                        ProjectWellboreDisplayReasonCode::DisplayUnavailable
                    }
                }),
                reason: resolved_well
                    .transform_diagnostics
                    .notes
                    .first()
                    .cloned()
                    .or_else(|| resolved_well.notes.first().cloned()),
            };
            compatibility.reason = project_wellbore_display_reason(&compatibility);
            compatibility
        }
        Err(error) => ProjectWellboreDisplayCompatibility {
            can_resolve_project_map: false,
            transform_status: SurveyMapTransformStatusDto::DisplayUnavailable,
            source_coordinate_reference_id: None,
            display_coordinate_reference_id: Some(display_coordinate_reference_id.to_string()),
            reason_code: Some(ProjectWellboreDisplayReasonCode::ResolutionError),
            reason: Some(error.to_string()),
        },
    }
}

fn project_survey_blocking_reason_code(
    reason_code: ProjectSurveyDisplayReasonCode,
) -> Option<ProjectDisplayCompatibilityBlockingReasonCode> {
    match reason_code {
        ProjectSurveyDisplayReasonCode::ProjectDisplayCrsUnresolved => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::ProjectDisplayCrsUnresolved)
        }
        ProjectSurveyDisplayReasonCode::DisplayCrsUnsupported => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::DisplayCrsUnsupported)
        }
        ProjectSurveyDisplayReasonCode::SourceCrsUnknown => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::SourceCrsUnknown)
        }
        ProjectSurveyDisplayReasonCode::SourceCrsUnsupported => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::SourceCrsUnsupported)
        }
        ProjectSurveyDisplayReasonCode::DisplayEquivalent
        | ProjectSurveyDisplayReasonCode::DisplayTransformed => None,
    }
}

fn project_wellbore_blocking_reason_code(
    reason_code: ProjectWellboreDisplayReasonCode,
) -> Option<ProjectDisplayCompatibilityBlockingReasonCode> {
    match reason_code {
        ProjectWellboreDisplayReasonCode::ProjectDisplayCrsUnresolved => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::ProjectDisplayCrsUnresolved)
        }
        ProjectWellboreDisplayReasonCode::ResolvedGeometryMissing => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::ResolvedGeometryMissing)
        }
        ProjectWellboreDisplayReasonCode::DisplayUnavailable => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::DisplayUnavailable)
        }
        ProjectWellboreDisplayReasonCode::ResolutionError => {
            Some(ProjectDisplayCompatibilityBlockingReasonCode::ResolutionError)
        }
        ProjectWellboreDisplayReasonCode::DisplayEquivalent
        | ProjectWellboreDisplayReasonCode::DisplayTransformed
        | ProjectWellboreDisplayReasonCode::DisplayDegraded => None,
    }
}

fn project_active_well_time_depth_model_asset_id(
    project: &OphioliteProject,
    wellbore_id: &str,
) -> Result<Option<String>, String> {
    let active_asset_id = project
        .project_well_overlay_inventory()
        .map_err(|error| error.to_string())?
        .wellbores
        .into_iter()
        .find(|wellbore| wellbore.wellbore_id.0 == wellbore_id)
        .and_then(|wellbore| wellbore.active_well_time_depth_model_asset_id)
        .map(|asset_id| asset_id.0);
    Ok(active_asset_id)
}

fn project_well_time_depth_model_descriptors(
    project: &OphioliteProject,
    wellbore_id: &str,
    active_asset_id: Option<&str>,
) -> Result<Vec<ProjectWellTimeDepthModelDescriptor>, String> {
    let assets = project
        .list_assets(
            &ophiolite::WellboreId(wellbore_id.to_string()),
            Some(AssetKind::WellTimeDepthModel),
        )
        .map_err(|error| error.to_string())?;

    assets
        .into_iter()
        .map(|asset| {
            let asset_id = asset.id.0.clone();
            let model = project
                .read_well_time_depth_model(&asset.id)
                .map_err(|error| error.to_string())?;
            Ok(ProjectWellTimeDepthModelDescriptor {
                asset_id: asset_id.clone(),
                well_id: asset.well_id.0,
                wellbore_id: asset.wellbore_id.0,
                status: asset_status_label(&asset.status).to_string(),
                name: model.name,
                source_kind: model.source_kind,
                depth_reference: model.depth_reference,
                travel_time_reference: model.travel_time_reference,
                sample_count: model.samples.len(),
                is_active_project_model: active_asset_id.is_some_and(|active| active == asset_id),
            })
        })
        .collect()
}

fn project_well_time_depth_observation_descriptors(
    project: &OphioliteProject,
    wellbore_id: &str,
) -> Result<Vec<ProjectWellTimeDepthObservationDescriptor>, String> {
    let mut descriptors = Vec::new();

    for asset_kind in [
        AssetKind::CheckshotVspObservationSet,
        AssetKind::ManualTimeDepthPickSet,
        AssetKind::WellTieObservationSet,
    ] {
        let assets = project
            .list_assets(
                &ophiolite::WellboreId(wellbore_id.to_string()),
                Some(asset_kind.clone()),
            )
            .map_err(|error| error.to_string())?;
        for asset in assets {
            let (
                name,
                depth_reference,
                travel_time_reference,
                sample_count,
                source_well_time_depth_model_asset_id,
                tie_window_start_ms,
                tie_window_end_ms,
                trace_search_radius_m,
                bulk_shift_ms,
                stretch_factor,
                trace_search_offset_m,
                correlation,
            ) = match asset_kind {
                AssetKind::CheckshotVspObservationSet => {
                    let source = project
                        .read_checkshot_vsp_observation_set(&asset.id)
                        .map_err(|error| error.to_string())?;
                    (
                        source.name,
                        source.depth_reference,
                        source.travel_time_reference,
                        source.samples.len(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                AssetKind::ManualTimeDepthPickSet => {
                    let source = project
                        .read_manual_time_depth_pick_set(&asset.id)
                        .map_err(|error| error.to_string())?;
                    (
                        source.name,
                        source.depth_reference,
                        source.travel_time_reference,
                        source.samples.len(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                AssetKind::WellTieObservationSet => {
                    let source = project
                        .read_well_tie_observation_set(&asset.id)
                        .map_err(|error| error.to_string())?;
                    (
                        source.name,
                        source.depth_reference,
                        source.travel_time_reference,
                        source.samples.len(),
                        source.source_well_time_depth_model_asset_id,
                        source.tie_window_start_ms,
                        source.tie_window_end_ms,
                        source.trace_search_radius_m,
                        source.bulk_shift_ms,
                        source.stretch_factor,
                        source.trace_search_offset_m,
                        source.correlation,
                    )
                }
                _ => continue,
            };
            descriptors.push(ProjectWellTimeDepthObservationDescriptor {
                asset_id: asset.id.0,
                asset_kind: match asset_kind {
                    AssetKind::CheckshotVspObservationSet => {
                        "checkshot_vsp_observation_set".to_string()
                    }
                    AssetKind::ManualTimeDepthPickSet => "manual_time_depth_pick_set".to_string(),
                    AssetKind::WellTieObservationSet => "well_tie_observation_set".to_string(),
                    _ => unreachable!("unexpected observation-set asset kind"),
                },
                well_id: asset.well_id.0,
                wellbore_id: asset.wellbore_id.0,
                status: asset_status_label(&asset.status).to_string(),
                name,
                depth_reference,
                travel_time_reference,
                sample_count,
                source_well_time_depth_model_asset_id,
                tie_window_start_ms,
                tie_window_end_ms,
                trace_search_radius_m,
                bulk_shift_ms,
                stretch_factor,
                trace_search_offset_m,
                correlation,
            });
        }
    }

    Ok(descriptors)
}

fn project_well_time_depth_authored_model_descriptors(
    project: &OphioliteProject,
    wellbore_id: &str,
) -> Result<Vec<ProjectWellTimeDepthAuthoredModelDescriptor>, String> {
    let assets = project
        .list_assets(
            &ophiolite::WellboreId(wellbore_id.to_string()),
            Some(AssetKind::WellTimeDepthAuthoredModel),
        )
        .map_err(|error| error.to_string())?;

    assets
        .into_iter()
        .map(|asset| {
            let model = project
                .read_well_time_depth_authored_model(&asset.id)
                .map_err(|error| error.to_string())?;
            Ok(ProjectWellTimeDepthAuthoredModelDescriptor {
                asset_id: asset.id.0,
                well_id: asset.well_id.0,
                wellbore_id: asset.wellbore_id.0,
                status: asset_status_label(&asset.status).to_string(),
                name: model.name,
                source_binding_count: model.source_bindings.len(),
                assumption_interval_count: model.assumption_intervals.len(),
                sampling_step_m: model.sampling_step_m,
                resolved_trajectory_fingerprint: model.resolved_trajectory_fingerprint,
            })
        })
        .collect()
}

fn project_well_marker_descriptors(
    project: &OphioliteProject,
    wellbore_id: &str,
) -> Result<Vec<ProjectWellMarkerDescriptor>, String> {
    Ok(project
        .list_well_markers(&ophiolite::WellboreId(wellbore_id.to_string()))
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|marker| ProjectWellMarkerDescriptor {
            name: marker.name,
            marker_kind: marker.marker_kind,
            source_asset_id: marker.source_asset_id.map(|asset_id| asset_id.0),
            top_depth: marker.top_measurement.value,
            base_depth: marker.base_measurement.map(|measurement| measurement.value),
            depth_reference: marker.depth_reference,
            source: marker.source,
            note: marker
                .notes
                .into_iter()
                .find(|value| !value.trim().is_empty()),
        })
        .collect::<Vec<_>>())
}

fn project_well_marker_horizon_residual_point_descriptors(
    points: Vec<WellMarkerHorizonResidualPointRecord>,
) -> Vec<ProjectWellMarkerHorizonResidualPointDescriptor> {
    points
        .into_iter()
        .map(|point| ProjectWellMarkerHorizonResidualPointDescriptor {
            marker_name: point.marker_name,
            marker_kind: point.marker_kind,
            x: point.x,
            y: point.y,
            z: point.z,
            horizon_depth: point.horizon_depth,
            residual: point.residual,
            status: point.status,
            note: point.note,
        })
        .collect()
}

fn compute_manifest_string_parameter(asset: &ophiolite::AssetRecord, name: &str) -> Option<String> {
    asset
        .manifest
        .compute_manifest
        .as_ref()?
        .parameters
        .get(name)
        .and_then(ComputeParameterValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn project_well_marker_horizon_residual_descriptors(
    project: &OphioliteProject,
    wellbore_id: &str,
) -> Result<Vec<ProjectWellMarkerHorizonResidualDescriptor>, String> {
    let collection_names = project
        .list_asset_collections(&ophiolite::WellboreId(wellbore_id.to_string()))
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|collection| (collection.id.0, collection.name))
        .collect::<std::collections::HashMap<_, _>>();
    let assets = project
        .list_assets(
            &ophiolite::WellboreId(wellbore_id.to_string()),
            Some(AssetKind::WellMarkerHorizonResidualSet),
        )
        .map_err(|error| error.to_string())?;

    assets
        .into_iter()
        .map(|asset| {
            let rows = project
                .read_well_marker_horizon_residual_rows(&asset.id)
                .map_err(|error| error.to_string())?;
            let points = project_well_marker_horizon_residual_point_descriptors(
                project
                    .read_well_marker_horizon_residual_points(&asset.id)
                    .map_err(|error| error.to_string())?,
            );
            let mut marker_names = rows
                .iter()
                .map(|row| row.marker_name.clone())
                .collect::<Vec<_>>();
            marker_names.sort();
            marker_names.dedup();
            Ok(ProjectWellMarkerHorizonResidualDescriptor {
                asset_id: asset.id.0.clone(),
                source_asset_id: asset
                    .manifest
                    .compute_manifest
                    .as_ref()
                    .map(|manifest| manifest.source_asset_id.clone())
                    .filter(|value| !value.trim().is_empty()),
                survey_asset_id: compute_manifest_string_parameter(&asset, "survey_asset_id"),
                horizon_id: compute_manifest_string_parameter(&asset, "horizon_id"),
                marker_name: compute_manifest_string_parameter(&asset, "marker_name"),
                well_id: asset.well_id.0,
                wellbore_id: asset.wellbore_id.0,
                status: asset_status_label(&asset.status).to_string(),
                name: collection_names
                    .get(&asset.collection_id.0)
                    .cloned()
                    .unwrap_or_else(|| asset.id.0.clone()),
                row_count: rows.len(),
                point_count: points.len(),
                marker_names,
                points,
            })
        })
        .collect()
}

fn well_time_depth_import_response(
    result: ophiolite::ProjectAssetImportResult,
) -> ImportProjectWellTimeDepthModelResponse {
    ImportProjectWellTimeDepthModelResponse {
        asset_id: result.asset.id.0,
        well_id: result.resolution.well_id.0,
        wellbore_id: result.resolution.wellbore_id.0,
        created_well: result.resolution.created_well,
        created_wellbore: result.resolution.created_wellbore,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedPreviewResponseHeader {
    preview_ready: bool,
    processing_label: String,
    section: PackedSectionHeader,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedSectionResponseHeader {
    section: PackedSectionHeader,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedSectionTileResponseHeader {
    section: PackedSectionHeader,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    trace_step: usize,
    sample_step: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedSectionDisplayResponseHeader {
    section: PackedSectionHeader,
    time_depth_diagnostics: Option<ophiolite::SectionTimeDepthDiagnostics>,
    scalar_overlays: Vec<PackedScalarOverlayHeader>,
    horizon_overlays: Vec<SectionHorizonOverlayView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedSectionHeader {
    dataset_id: String,
    axis: SectionAxis,
    coordinate: ophiolite::SectionCoordinate,
    traces: usize,
    samples: usize,
    horizontal_axis_bytes: usize,
    inline_axis_bytes: Option<usize>,
    xline_axis_bytes: Option<usize>,
    sample_axis_bytes: usize,
    amplitudes_bytes: usize,
    units: Option<ophiolite::SectionUnits>,
    metadata: Option<ophiolite::SectionMetadata>,
    display_defaults: Option<ophiolite::SectionDisplayDefaults>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedScalarOverlayHeader {
    id: String,
    name: Option<String>,
    width: usize,
    height: usize,
    values_bytes: usize,
    color_map: ophiolite::SectionScalarOverlayColorMap,
    opacity: f32,
    value_range: ophiolite::SectionScalarOverlayValueRange,
    units: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetExportFormatCapability {
    available: bool,
    reason: Option<String>,
    default_output_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetExportCapabilitiesResponse {
    store_path: String,
    segy: DatasetExportFormatCapability,
    zarr: DatasetExportFormatCapability,
}

fn align_up(value: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return value;
    }
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + (alignment - remainder)
    }
}

fn section_payload_bytes(section: &SectionView) -> u64 {
    (section.horizontal_axis_f64le.len()
        + section
            .inline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + section
            .xline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + section.sample_axis_f32le.len()
        + section.amplitudes_f32le.len()) as u64
}

fn section_tile_payload_bytes(tile: &SectionTileView) -> u64 {
    section_payload_bytes(&tile.section)
}

fn median_f64(values: &[f64]) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    ordered[ordered.len() / 2]
}

fn mean_f64(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn validate_benchmark_range(
    range: [usize; 2],
    total: usize,
    label: &str,
) -> Result<[usize; 2], String> {
    if range[0] >= range[1] || range[1] > total {
        return Err(format!(
            "invalid {label} range [{}, {}) for total length {total}",
            range[0], range[1]
        ));
    }
    Ok(range)
}

fn clamp_window_to_total(range: [usize; 2], total: usize) -> [usize; 2] {
    let width = (range[1].saturating_sub(range[0])).max(1).min(total.max(1));
    let start = range[0].min(total.saturating_sub(width));
    [start, start + width]
}

fn stepped_section_index(current: usize, offset: isize, axis_length: usize) -> usize {
    if axis_length <= 1 {
        return 0;
    }
    let max_index = axis_length - 1;
    if offset >= 0 {
        current.saturating_add(offset as usize).min(max_index)
    } else {
        current.saturating_sub(offset.unsigned_abs())
    }
}

fn axis_name(axis: SectionAxis) -> String {
    format!("{axis:?}").to_ascii_lowercase()
}

fn benchmark_iterations(requested: Option<usize>) -> usize {
    requested.unwrap_or(5).clamp(1, 50)
}

fn benchmark_step_offsets(requested: Option<Vec<isize>>) -> Vec<isize> {
    let offsets = requested.unwrap_or_else(|| vec![1, 1, -1]);
    if offsets.is_empty() {
        vec![1, 1, -1]
    } else {
        offsets
    }
}

fn measure_full_section_case(
    handle: &seis_runtime::StoreHandle,
    axis: SectionAxis,
    index: usize,
    iterations: usize,
) -> Result<SectionBrowsingBenchmarkCase, String> {
    let mut iteration_ms = Vec::with_capacity(iterations);
    let mut output_traces = 0usize;
    let mut output_samples = 0usize;
    let mut payload_bytes = 0u64;
    for _ in 0..iterations {
        let started = Instant::now();
        let section = handle
            .section_view(axis, index)
            .map_err(|error| error.to_string())?;
        iteration_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        output_traces = section.traces;
        output_samples = section.samples;
        payload_bytes = section_payload_bytes(&section);
    }

    Ok(SectionBrowsingBenchmarkCase {
        scenario: "full_section_baseline".to_string(),
        axis: axis_name(axis),
        index,
        trace_range: [0, section_axis_length(handle, axis)],
        sample_range: [0, handle.manifest.volume.shape[2]],
        lod: 0,
        trace_step: 1,
        sample_step: 1,
        output_traces,
        output_samples,
        payload_bytes,
        median_ms: median_f64(&iteration_ms),
        mean_ms: mean_f64(&iteration_ms),
        iteration_ms,
    })
}

fn measure_section_tile_case(
    handle: &seis_runtime::StoreHandle,
    scenario: String,
    axis: SectionAxis,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    iterations: usize,
) -> Result<SectionBrowsingBenchmarkCase, String> {
    let mut iteration_ms = Vec::with_capacity(iterations);
    let mut trace_step = 1usize;
    let mut sample_step = 1usize;
    let mut output_traces = 0usize;
    let mut output_samples = 0usize;
    let mut payload_bytes = 0u64;

    for _ in 0..iterations {
        let started = Instant::now();
        let tile = handle
            .section_tile_view(axis, index, trace_range, sample_range, lod)
            .map_err(|error| error.to_string())?;
        iteration_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        trace_step = tile.trace_step;
        sample_step = tile.sample_step;
        output_traces = tile.section.traces;
        output_samples = tile.section.samples;
        payload_bytes = section_tile_payload_bytes(&tile);
    }

    Ok(SectionBrowsingBenchmarkCase {
        scenario,
        axis: axis_name(axis),
        index,
        trace_range,
        sample_range,
        lod,
        trace_step,
        sample_step,
        output_traces,
        output_samples,
        payload_bytes,
        median_ms: median_f64(&iteration_ms),
        mean_ms: mean_f64(&iteration_ms),
        iteration_ms,
    })
}

fn pack_preview_section_response(
    preview_ready: bool,
    processing_label: String,
    section: SectionView,
) -> Result<Response, String> {
    let header = PackedPreviewResponseHeader {
        preview_ready,
        processing_label,
        section: packed_section_header(&section),
    };

    let header_bytes = serde_json::to_vec(&header).map_err(|error| error.to_string())?;
    let header_end = 16 + header_bytes.len();
    let data_offset = align_up(header_end, 8);
    let total_len = data_offset
        + section.horizontal_axis_f64le.len()
        + section
            .inline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + section
            .xline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + section.sample_axis_f32le.len()
        + section.amplitudes_f32le.len();

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(PACKED_PREVIEW_MAGIC);
    bytes.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(data_offset as u32).to_le_bytes());
    bytes.extend_from_slice(&header_bytes);
    bytes.resize(data_offset, 0);
    bytes.extend_from_slice(&section.horizontal_axis_f64le);
    if let Some(inline_axis) = section.inline_axis_f64le.as_ref() {
        bytes.extend_from_slice(inline_axis);
    }
    if let Some(xline_axis) = section.xline_axis_f64le.as_ref() {
        bytes.extend_from_slice(xline_axis);
    }
    bytes.extend_from_slice(&section.sample_axis_f32le);
    bytes.extend_from_slice(&section.amplitudes_f32le);
    Ok(Response::new(bytes))
}

fn pack_section_response(section: SectionView) -> Result<Response, String> {
    let header = PackedSectionResponseHeader {
        section: packed_section_header(&section),
    };

    let header_bytes = serde_json::to_vec(&header).map_err(|error| error.to_string())?;
    let header_end = 16 + header_bytes.len();
    let data_offset = align_up(header_end, 8);
    let total_len = data_offset
        + section.horizontal_axis_f64le.len()
        + section
            .inline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + section
            .xline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + section.sample_axis_f32le.len()
        + section.amplitudes_f32le.len();

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(PACKED_SECTION_MAGIC);
    bytes.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(data_offset as u32).to_le_bytes());
    bytes.extend_from_slice(&header_bytes);
    bytes.resize(data_offset, 0);
    bytes.extend_from_slice(&section.horizontal_axis_f64le);
    if let Some(inline_axis) = section.inline_axis_f64le.as_ref() {
        bytes.extend_from_slice(inline_axis);
    }
    if let Some(xline_axis) = section.xline_axis_f64le.as_ref() {
        bytes.extend_from_slice(xline_axis);
    }
    bytes.extend_from_slice(&section.sample_axis_f32le);
    bytes.extend_from_slice(&section.amplitudes_f32le);
    Ok(Response::new(bytes))
}

fn pack_section_tile_response(tile: SectionTileView) -> Result<Response, String> {
    let header = PackedSectionTileResponseHeader {
        section: packed_section_header(&tile.section),
        trace_range: tile.trace_range,
        sample_range: tile.sample_range,
        lod: tile.lod,
        trace_step: tile.trace_step,
        sample_step: tile.sample_step,
    };

    let header_bytes = serde_json::to_vec(&header).map_err(|error| error.to_string())?;
    let header_end = 16 + header_bytes.len();
    let data_offset = align_up(header_end, 8);
    let total_len = data_offset
        + tile.section.horizontal_axis_f64le.len()
        + tile
            .section
            .inline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + tile
            .section
            .xline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + tile.section.sample_axis_f32le.len()
        + tile.section.amplitudes_f32le.len();

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(PACKED_SECTION_TILE_MAGIC);
    bytes.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(data_offset as u32).to_le_bytes());
    bytes.extend_from_slice(&header_bytes);
    bytes.resize(data_offset, 0);
    bytes.extend_from_slice(&tile.section.horizontal_axis_f64le);
    if let Some(inline_axis) = tile.section.inline_axis_f64le.as_ref() {
        bytes.extend_from_slice(inline_axis);
    }
    if let Some(xline_axis) = tile.section.xline_axis_f64le.as_ref() {
        bytes.extend_from_slice(xline_axis);
    }
    bytes.extend_from_slice(&tile.section.sample_axis_f32le);
    bytes.extend_from_slice(&tile.section.amplitudes_f32le);
    Ok(Response::new(bytes))
}

fn pack_section_display_response(
    display: ophiolite::ResolvedSectionDisplayView,
) -> Result<Response, String> {
    let header = PackedSectionDisplayResponseHeader {
        section: packed_section_header(&display.section),
        time_depth_diagnostics: display.time_depth_diagnostics.clone(),
        scalar_overlays: display
            .scalar_overlays
            .iter()
            .map(|overlay| PackedScalarOverlayHeader {
                id: overlay.id.clone(),
                name: overlay.name.clone(),
                width: overlay.width,
                height: overlay.height,
                values_bytes: overlay.values_f32le.len(),
                color_map: overlay.color_map,
                opacity: overlay.opacity,
                value_range: overlay.value_range.clone(),
                units: overlay.units.clone(),
            })
            .collect(),
        horizon_overlays: display.horizon_overlays.clone(),
    };

    let header_bytes = serde_json::to_vec(&header).map_err(|error| error.to_string())?;
    let header_end = 16 + header_bytes.len();
    let data_offset = align_up(header_end, 8);
    let total_len = data_offset
        + display.section.horizontal_axis_f64le.len()
        + display
            .section
            .inline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + display
            .section
            .xline_axis_f64le
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + display.section.sample_axis_f32le.len()
        + display.section.amplitudes_f32le.len()
        + display
            .scalar_overlays
            .iter()
            .map(|overlay| overlay.values_f32le.len())
            .sum::<usize>();

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(PACKED_SECTION_DISPLAY_MAGIC);
    bytes.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(data_offset as u32).to_le_bytes());
    bytes.extend_from_slice(&header_bytes);
    bytes.resize(data_offset, 0);
    bytes.extend_from_slice(&display.section.horizontal_axis_f64le);
    if let Some(inline_axis) = display.section.inline_axis_f64le.as_ref() {
        bytes.extend_from_slice(inline_axis);
    }
    if let Some(xline_axis) = display.section.xline_axis_f64le.as_ref() {
        bytes.extend_from_slice(xline_axis);
    }
    bytes.extend_from_slice(&display.section.sample_axis_f32le);
    bytes.extend_from_slice(&display.section.amplitudes_f32le);
    for overlay in &display.scalar_overlays {
        bytes.extend_from_slice(&overlay.values_f32le);
    }
    Ok(Response::new(bytes))
}

fn packed_section_header(section: &SectionView) -> PackedSectionHeader {
    PackedSectionHeader {
        dataset_id: section.dataset_id.0.clone(),
        axis: section.axis,
        coordinate: section.coordinate.clone(),
        traces: section.traces,
        samples: section.samples,
        horizontal_axis_bytes: section.horizontal_axis_f64le.len(),
        inline_axis_bytes: section.inline_axis_f64le.as_ref().map(Vec::len),
        xline_axis_bytes: section.xline_axis_f64le.as_ref().map(Vec::len),
        sample_axis_bytes: section.sample_axis_f32le.len(),
        amplitudes_bytes: section.amplitudes_f32le.len(),
        units: section.units.clone(),
        metadata: section.metadata.clone(),
        display_defaults: section.display_defaults.clone(),
    }
}

fn sanitized_stem(value: &str, fallback: &str) -> String {
    let sanitized: String = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let collapsed = sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if collapsed.is_empty() {
        fallback.to_string()
    } else {
        collapsed
    }
}

fn section_axis_values(handle: &seis_runtime::StoreHandle, axis: SectionAxis) -> &[f64] {
    match axis {
        SectionAxis::Inline => &handle.manifest.volume.axes.ilines,
        SectionAxis::Xline => &handle.manifest.volume.axes.xlines,
    }
}

fn section_axis_length(handle: &seis_runtime::StoreHandle, axis: SectionAxis) -> usize {
    section_axis_values(handle, axis).len()
}

fn section_axis_range(handle: &seis_runtime::StoreHandle, axis: SectionAxis) -> Option<(f64, f64)> {
    let values = section_axis_values(handle, axis);
    Some((*values.first()?, *values.last()?))
}

fn sample_axis_range_ms(handle: &seis_runtime::StoreHandle) -> Option<(f32, f32)> {
    let values = &handle.manifest.volume.axes.sample_axis_ms;
    Some((*values.first()?, *values.last()?))
}

fn section_coordinate_value(
    handle: &seis_runtime::StoreHandle,
    axis: SectionAxis,
    index: usize,
) -> Option<f64> {
    section_axis_values(handle, axis).get(index).copied()
}

fn section_coordinate_within_crop(
    pipeline: &SubvolumeProcessingPipeline,
    axis: SectionAxis,
    coordinate_value: f64,
) -> bool {
    match axis {
        SectionAxis::Inline => {
            coordinate_value >= f64::from(pipeline.crop.inline_min)
                && coordinate_value <= f64::from(pipeline.crop.inline_max)
        }
        SectionAxis::Xline => {
            coordinate_value >= f64::from(pipeline.crop.xline_min)
                && coordinate_value <= f64::from(pipeline.crop.xline_max)
        }
    }
}

fn append_section_debug_fields(
    fields: &mut Vec<(&'static str, Value)>,
    handle: &seis_runtime::StoreHandle,
    axis: SectionAxis,
    index: usize,
    axis_length_key: &'static str,
    axis_range_key: &'static str,
    coordinate_key: &'static str,
) {
    fields.push((
        axis_length_key,
        json_value(section_axis_length(handle, axis)),
    ));
    if let Some((axis_min, axis_max)) = section_axis_range(handle, axis) {
        fields.push((axis_range_key, json_value([axis_min, axis_max])));
    }
    if let Some(coordinate_value) = section_coordinate_value(handle, axis, index) {
        fields.push((coordinate_key, json_value(coordinate_value)));
    }
}

fn append_subvolume_preview_debug_fields(
    fields: &mut Vec<(&'static str, Value)>,
    handle: &seis_runtime::StoreHandle,
    request: &PreviewSubvolumeProcessingRequest,
) {
    fields.push(("executionOrder", json_value("trace_local_then_crop")));
    fields.push(("sourceShape", json_value(handle.manifest.volume.shape)));
    if let Some((inline_min, inline_max)) = section_axis_range(handle, SectionAxis::Inline) {
        fields.push(("sourceInlineRange", json_value([inline_min, inline_max])));
    }
    if let Some((xline_min, xline_max)) = section_axis_range(handle, SectionAxis::Xline) {
        fields.push(("sourceXlineRange", json_value([xline_min, xline_max])));
    }
    if let Some((z_min, z_max)) = sample_axis_range_ms(handle) {
        fields.push(("sourceZRangeMs", json_value([z_min, z_max])));
    }
    append_section_debug_fields(
        fields,
        handle,
        request.section.axis,
        request.section.index,
        "sourceSectionAxisLength",
        "sourceSectionAxisRange",
        "requestedSectionCoordinateValue",
    );
    if let Some(coordinate_value) =
        section_coordinate_value(handle, request.section.axis, request.section.index)
    {
        fields.push((
            "sectionInsideCropWindow",
            json_value(section_coordinate_within_crop(
                &request.pipeline,
                request.section.axis,
                coordinate_value,
            )),
        ));
    }
}

fn pipeline_output_slug(pipeline: &seis_runtime::TraceLocalProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return sanitized_stem(name, "pipeline");
    }

    let mut parts = Vec::with_capacity(pipeline.operation_count());
    for operation in pipeline.operations() {
        let part = match operation {
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
                sanitized_stem(
                    Path::new(secondary_store_path)
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("volume"),
                    "volume",
                )
            ),
        };
        parts.push(part);
    }

    if parts.is_empty() {
        "pipeline".to_string()
    } else {
        sanitized_stem(&parts.join("-"), "pipeline")
    }
}

fn gather_pipeline_output_slug(pipeline: &seis_runtime::GatherProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return sanitized_stem(name, "gather-pipeline");
    }

    let mut parts = Vec::new();
    if let Some(trace_local) = &pipeline.trace_local_pipeline {
        parts.push(pipeline_output_slug(trace_local));
    }
    for operation in &pipeline.operations {
        let part = match operation {
            seis_runtime::GatherProcessingOperation::NmoCorrection {
                velocity_model,
                interpolation,
            } => format!(
                "nmo-{}-{}",
                velocity_model_output_slug(velocity_model),
                gather_interpolation_output_slug(*interpolation)
            ),
            seis_runtime::GatherProcessingOperation::StretchMute {
                velocity_model,
                max_stretch_ratio,
            } => format!(
                "stretch-mute-{}-{}",
                velocity_model_output_slug(velocity_model),
                format_factor(*max_stretch_ratio)
            ),
            seis_runtime::GatherProcessingOperation::OffsetMute {
                min_offset,
                max_offset,
            } => format!(
                "offset-mute-{}-{}",
                optional_factor_output_slug(*min_offset),
                optional_factor_output_slug(*max_offset)
            ),
        };
        parts.push(part);
    }

    if parts.is_empty() {
        "gather-pipeline".to_string()
    } else {
        sanitized_stem(&parts.join("-"), "gather-pipeline")
    }
}

fn subvolume_pipeline_output_slug(pipeline: &SubvolumeProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return sanitized_stem(name, "crop-subvolume");
    }

    let mut parts = Vec::new();
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        parts.push(pipeline_output_slug(trace_local_pipeline));
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
    sanitized_stem(&parts.join("-"), "crop-subvolume")
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

fn gather_interpolation_output_slug(mode: seis_runtime::GatherInterpolationMode) -> &'static str {
    match mode {
        seis_runtime::GatherInterpolationMode::Linear => "linear",
    }
}

fn velocity_model_output_slug(model: &seis_runtime::VelocityFunctionSource) -> String {
    match model {
        seis_runtime::VelocityFunctionSource::ConstantVelocity { velocity_m_per_s } => {
            format!("constant-{}", format_factor(*velocity_m_per_s))
        }
        seis_runtime::VelocityFunctionSource::TimeVelocityPairs { .. } => {
            "time-velocity-pairs".to_string()
        }
        seis_runtime::VelocityFunctionSource::VelocityAssetReference { asset_id } => {
            sanitized_stem(&format!("velocity-asset-{asset_id}"), "velocity-asset")
        }
    }
}

fn optional_factor_output_slug(value: Option<f32>) -> String {
    value
        .map(format_factor)
        .unwrap_or_else(|| "none".to_string())
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

fn source_store_stem(store_path: &str) -> String {
    let path = Path::new(store_path);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("dataset");
    sanitized_stem(stem, "dataset")
}

fn unique_store_candidate(dir: &Path, base_name: &str, extension: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{base_name}.{extension}"));
    let mut index = 2usize;
    while candidate.exists() {
        candidate = dir.join(format!("{base_name}-{index}.{extension}"));
        index += 1;
    }
    candidate
}

fn default_processing_store_path(
    app_paths: &AppPaths,
    input_store_path: &str,
    pipeline: &seis_runtime::TraceLocalProcessingPipeline,
) -> Result<String, String> {
    fs::create_dir_all(app_paths.derived_volumes_dir()).map_err(|error| error.to_string())?;
    let source_stem = source_store_stem(input_store_path);
    let pipeline_stem = pipeline_output_slug(pipeline);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let base_name = format!("{source_stem}.{pipeline_stem}.{timestamp}");
    Ok(
        unique_store_candidate(app_paths.derived_volumes_dir(), &base_name, "tbvol")
            .display()
            .to_string(),
    )
}

fn default_subvolume_processing_store_path(
    app_paths: &AppPaths,
    input_store_path: &str,
    pipeline: &SubvolumeProcessingPipeline,
) -> Result<String, String> {
    fs::create_dir_all(app_paths.derived_volumes_dir()).map_err(|error| error.to_string())?;
    let source_stem = source_store_stem(input_store_path);
    let pipeline_stem = subvolume_pipeline_output_slug(pipeline);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let base_name = format!("{source_stem}.{pipeline_stem}.{timestamp}");
    Ok(
        unique_store_candidate(app_paths.derived_volumes_dir(), &base_name, "tbvol")
            .display()
            .to_string(),
    )
}

fn default_gather_processing_store_path(
    app_paths: &AppPaths,
    input_store_path: &str,
    pipeline: &GatherProcessingPipeline,
) -> Result<String, String> {
    fs::create_dir_all(app_paths.derived_gathers_dir()).map_err(|error| error.to_string())?;
    let source_stem = source_store_stem(input_store_path);
    let pipeline_stem = gather_pipeline_output_slug(pipeline);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let base_name = format!("{source_stem}.{pipeline_stem}.{timestamp}");
    Ok(
        unique_store_candidate(app_paths.derived_gathers_dir(), &base_name, "tbgath")
            .display()
            .to_string(),
    )
}

#[derive(Debug, Clone)]
struct TraceLocalProcessingStage {
    segment_pipeline: TraceLocalProcessingPipeline,
    lineage_pipeline: TraceLocalProcessingPipeline,
    stage_label: String,
    artifact: ProcessingJobArtifact,
}

fn processing_operation_display_label(operation: &seis_runtime::ProcessingOperation) -> String {
    match operation {
        seis_runtime::ProcessingOperation::AmplitudeScalar { factor } => {
            format!("amplitude scalar ({factor})")
        }
        seis_runtime::ProcessingOperation::TraceRmsNormalize => "trace RMS normalize".to_string(),
        seis_runtime::ProcessingOperation::AgcRms { window_ms } => {
            format!("RMS AGC ({window_ms} ms)")
        }
        seis_runtime::ProcessingOperation::PhaseRotation { angle_degrees } => {
            format!("phase rotation ({angle_degrees} deg)")
        }
        seis_runtime::ProcessingOperation::LowpassFilter { f3_hz, f4_hz, .. } => {
            format!("lowpass ({f3_hz}/{f4_hz} Hz)")
        }
        seis_runtime::ProcessingOperation::HighpassFilter { f1_hz, f2_hz, .. } => {
            format!("highpass ({f1_hz}/{f2_hz} Hz)")
        }
        seis_runtime::ProcessingOperation::BandpassFilter {
            f1_hz,
            f2_hz,
            f3_hz,
            f4_hz,
            ..
        } => {
            format!("bandpass ({f1_hz}/{f2_hz}/{f3_hz}/{f4_hz} Hz)")
        }
        seis_runtime::ProcessingOperation::VolumeArithmetic {
            operator,
            secondary_store_path,
        } => format!(
            "{} volume ({})",
            volume_arithmetic_operator_slug(*operator),
            display_store_stem(secondary_store_path)
        ),
    }
}

fn preview_processing_operation_ids(pipeline: &TraceLocalProcessingPipeline) -> Vec<&'static str> {
    pipeline
        .operations()
        .map(seis_runtime::ProcessingOperation::operator_id)
        .collect()
}

fn preview_processing_operation_labels(pipeline: &TraceLocalProcessingPipeline) -> Vec<String> {
    pipeline
        .operations()
        .map(processing_operation_display_label)
        .collect()
}

fn display_store_stem(store_path: &str) -> String {
    Path::new(store_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("volume")
        .to_string()
}

fn trace_local_operations(
    pipeline: &TraceLocalProcessingPipeline,
) -> Vec<seis_runtime::ProcessingOperation> {
    pipeline.operations().cloned().collect()
}

fn clone_pipeline_with_steps(
    pipeline: &TraceLocalProcessingPipeline,
    steps: Vec<seis_runtime::TraceLocalProcessingStep>,
) -> TraceLocalProcessingPipeline {
    TraceLocalProcessingPipeline {
        schema_version: pipeline.schema_version,
        revision: pipeline.revision,
        preset_id: pipeline.preset_id.clone(),
        name: pipeline.name.clone(),
        description: pipeline.description.clone(),
        steps,
    }
}

fn pipeline_prefix(
    pipeline: &TraceLocalProcessingPipeline,
    end_operation_index: usize,
) -> TraceLocalProcessingPipeline {
    clone_pipeline_with_steps(pipeline, pipeline.steps[..=end_operation_index].to_vec())
}

fn pipeline_segment(
    pipeline: &TraceLocalProcessingPipeline,
    start_operation_index: usize,
    end_operation_index: usize,
) -> TraceLocalProcessingPipeline {
    clone_pipeline_with_steps(
        pipeline,
        pipeline.steps[start_operation_index..=end_operation_index].to_vec(),
    )
}

fn resolve_trace_local_checkpoint_indexes(
    pipeline: &TraceLocalProcessingPipeline,
    allow_final_checkpoint: bool,
) -> Result<Vec<usize>, String> {
    if pipeline.steps.is_empty() {
        return Ok(Vec::new());
    }

    let last_index = pipeline.steps.len() - 1;
    let indexes = pipeline.checkpoint_indexes();

    for index in &indexes {
        if *index >= pipeline.steps.len() {
            return Err(format!(
                "Checkpoint index {index} is out of range for a pipeline with {} steps.",
                pipeline.steps.len()
            ));
        }
        if *index == last_index && !allow_final_checkpoint {
            return Err(
                "Checkpoint markers cannot target the final step because the final output is emitted automatically."
                    .to_string(),
            );
        }
    }

    Ok(indexes)
}

fn checkpoint_output_store_path(
    final_output_store_path: &str,
    job_id: &str,
    step_index: usize,
    operation: &seis_runtime::ProcessingOperation,
) -> String {
    let output_path = Path::new(final_output_store_path);
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("processed");
    let job_stem = sanitized_stem(job_id, "job");
    let operation_stem = sanitized_stem(operation.operator_id(), "step");
    parent
        .join(format!(
            "{stem}.{job_stem}.step-{:02}-{operation_stem}.tbvol",
            step_index + 1
        ))
        .display()
        .to_string()
}

fn build_trace_local_processing_stages_from(
    request: &RunTraceLocalProcessingRequest,
    final_output_store_path: &str,
    job_id: &str,
    start_operation_index: usize,
) -> Result<Vec<TraceLocalProcessingStage>, String> {
    let checkpoint_indexes = resolve_trace_local_checkpoint_indexes(&request.pipeline, false)?;
    let mut stage_end_indexes = checkpoint_indexes;
    let final_step_index = request.pipeline.operation_count().saturating_sub(1);
    stage_end_indexes.push(final_step_index);
    stage_end_indexes.retain(|index| *index >= start_operation_index);

    let mut stages = Vec::with_capacity(stage_end_indexes.len());
    let mut segment_start = start_operation_index;
    for end_index in stage_end_indexes {
        let operation = request
            .pipeline
            .steps
            .get(end_index)
            .map(|step| &step.operation)
            .ok_or_else(|| format!("Missing operation at stage end index {end_index}"))?;
        let stage_label = format!(
            "Step {}: {}",
            end_index + 1,
            processing_operation_display_label(operation)
        );
        let artifact = ProcessingJobArtifact {
            kind: if end_index == final_step_index {
                ProcessingJobArtifactKind::FinalOutput
            } else {
                ProcessingJobArtifactKind::Checkpoint
            },
            step_index: end_index,
            label: stage_label.clone(),
            store_path: if end_index == final_step_index {
                final_output_store_path.to_string()
            } else {
                checkpoint_output_store_path(final_output_store_path, job_id, end_index, operation)
            },
        };
        stages.push(TraceLocalProcessingStage {
            segment_pipeline: pipeline_segment(&request.pipeline, segment_start, end_index),
            lineage_pipeline: pipeline_prefix(&request.pipeline, end_index),
            stage_label,
            artifact,
        });
        segment_start = end_index + 1;
    }

    Ok(stages)
}

fn build_trace_local_checkpoint_stages_from_pipeline(
    pipeline: &TraceLocalProcessingPipeline,
    final_output_store_path: &str,
    job_id: &str,
    start_operation_index: usize,
    allow_final_checkpoint: bool,
) -> Result<Vec<TraceLocalProcessingStage>, String> {
    let stage_end_indexes =
        resolve_trace_local_checkpoint_indexes(pipeline, allow_final_checkpoint)?
            .into_iter()
            .filter(|index| *index >= start_operation_index)
            .collect::<Vec<_>>();

    let mut stages = Vec::with_capacity(stage_end_indexes.len());
    let mut segment_start = start_operation_index;
    for end_index in stage_end_indexes {
        let operation = pipeline
            .steps
            .get(end_index)
            .map(|step| &step.operation)
            .ok_or_else(|| format!("Missing operation at checkpoint index {end_index}"))?;
        let stage_label = format!(
            "Step {}: {}",
            end_index + 1,
            processing_operation_display_label(operation)
        );
        let artifact = ProcessingJobArtifact {
            kind: ProcessingJobArtifactKind::Checkpoint,
            step_index: end_index,
            label: stage_label.clone(),
            store_path: checkpoint_output_store_path(
                final_output_store_path,
                job_id,
                end_index,
                operation,
            ),
        };
        stages.push(TraceLocalProcessingStage {
            segment_pipeline: pipeline_segment(pipeline, segment_start, end_index),
            lineage_pipeline: pipeline_prefix(pipeline, end_index),
            stage_label,
            artifact,
        });
        segment_start = end_index + 1;
    }

    Ok(stages)
}

#[derive(Debug, Clone)]
struct ReusedTraceLocalCheckpoint {
    after_operation_index: usize,
    path: String,
    artifact: ProcessingJobArtifact,
}

fn resolve_reused_trace_local_checkpoint(
    processing_cache: &ProcessingCacheState,
    request: &RunTraceLocalProcessingRequest,
    allow_final_checkpoint: bool,
) -> Result<Option<ReusedTraceLocalCheckpoint>, String> {
    if !processing_cache.enabled() {
        return Ok(None);
    }

    let checkpoint_indexes =
        resolve_trace_local_checkpoint_indexes(&request.pipeline, allow_final_checkpoint)?;
    if checkpoint_indexes.is_empty() {
        return Ok(None);
    }

    let source_fingerprint = trace_local_source_fingerprint(&request.store_path)?;
    for checkpoint_index in checkpoint_indexes.into_iter().rev() {
        let lineage_pipeline = pipeline_prefix(&request.pipeline, checkpoint_index);
        let prefix_hash = trace_local_pipeline_hash(&lineage_pipeline)?;
        if let Some(hit) = processing_cache.lookup_prefix_artifact(
            TRACE_LOCAL_CACHE_FAMILY,
            &source_fingerprint,
            &prefix_hash,
            checkpoint_index + 1,
        )? {
            let operation = request
                .pipeline
                .steps
                .get(checkpoint_index)
                .map(|step| &step.operation)
                .ok_or_else(|| {
                    format!("Missing operation at checkpoint index {checkpoint_index}")
                })?;
            let artifact = ProcessingJobArtifact {
                kind: ProcessingJobArtifactKind::Checkpoint,
                step_index: checkpoint_index,
                label: format!(
                    "Reused checkpoint after step {}: {}",
                    checkpoint_index + 1,
                    processing_operation_display_label(operation)
                ),
                store_path: hit.path.clone(),
            };
            return Ok(Some(ReusedTraceLocalCheckpoint {
                after_operation_index: checkpoint_index,
                path: hit.path,
                artifact,
            }));
        }
    }

    Ok(None)
}

fn rewrite_trace_local_processing_lineage(
    store_path: &str,
    pipeline: &TraceLocalProcessingPipeline,
    artifact_kind: ProcessingJobArtifactKind,
) -> Result<(), String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let mut manifest: TbvolManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let lineage = manifest
        .volume
        .processing_lineage
        .as_mut()
        .ok_or_else(|| format!("Derived store is missing processing lineage: {store_path}"))?;
    lineage.pipeline = ProcessingPipelineSpec::TraceLocal {
        pipeline: pipeline.clone(),
    };
    lineage.artifact_role = match artifact_kind {
        ProcessingJobArtifactKind::Checkpoint => ProcessingArtifactRole::Checkpoint,
        ProcessingJobArtifactKind::FinalOutput => ProcessingArtifactRole::FinalOutput,
    };
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn rewrite_subvolume_processing_lineage(
    store_path: &str,
    pipeline: &SubvolumeProcessingPipeline,
    artifact_kind: ProcessingJobArtifactKind,
) -> Result<(), String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let mut manifest: TbvolManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let lineage = manifest
        .volume
        .processing_lineage
        .as_mut()
        .ok_or_else(|| format!("Derived store is missing processing lineage: {store_path}"))?;
    lineage.pipeline = ProcessingPipelineSpec::Subvolume {
        pipeline: pipeline.clone(),
    };
    lineage.artifact_role = match artifact_kind {
        ProcessingJobArtifactKind::Checkpoint => ProcessingArtifactRole::Checkpoint,
        ProcessingJobArtifactKind::FinalOutput => ProcessingArtifactRole::FinalOutput,
    };
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn normalized_path_key(path: &str) -> String {
    path.trim().replace('/', "\\").to_ascii_lowercase()
}

#[derive(serde::Serialize)]
struct TraceLocalPipelineCacheIdentity<'a> {
    schema_version: u32,
    revision: u32,
    operations: &'a [seis_runtime::ProcessingOperation],
}

fn trace_local_source_fingerprint(store_path: &str) -> Result<String, String> {
    let manifest_path = Path::new(store_path).join("manifest.json");
    let manifest = fs::read(&manifest_path).map_err(|error| {
        format!(
            "Failed to read store manifest for cache fingerprint ({}): {error}",
            manifest_path.display()
        )
    })?;
    Ok(ProcessingCacheState::fingerprint_bytes(&manifest))
}

fn trace_local_pipeline_hash(pipeline: &TraceLocalProcessingPipeline) -> Result<String, String> {
    let operations = trace_local_operations(pipeline);
    ProcessingCacheState::fingerprint_json(&TraceLocalPipelineCacheIdentity {
        schema_version: pipeline.schema_version,
        revision: pipeline.revision,
        operations: &operations,
    })
}

fn materialize_options_for_store(input_store_path: &str) -> Result<MaterializeOptions, String> {
    let chunk_shape = open_store(input_store_path)
        .map_err(|error| error.to_string())?
        .manifest
        .tile_shape;
    Ok(MaterializeOptions {
        chunk_shape,
        ..MaterializeOptions::default()
    })
}

fn register_processing_store_artifact(
    app: &AppHandle,
    input_store_path: &str,
    artifact: &ProcessingJobArtifact,
) -> Result<(), String> {
    let workspace = match app.try_state::<WorkspaceState>() {
        Some(state) => state,
        None => return Ok(()),
    };
    let source_state = workspace.load_state()?;
    let source_key = normalized_path_key(input_store_path);
    let source_entry = source_state.entries.iter().find(|entry| {
        entry
            .imported_store_path
            .as_deref()
            .map(normalized_path_key)
            .as_deref()
            == Some(source_key.as_str())
            || entry
                .preferred_store_path
                .as_deref()
                .map(normalized_path_key)
                .as_deref()
                == Some(source_key.as_str())
            || entry
                .last_dataset
                .as_ref()
                .map(|dataset| normalized_path_key(&dataset.store_path))
                .as_deref()
                == Some(source_key.as_str())
    });
    let source_label = source_entry
        .map(|entry| entry.display_name.clone())
        .unwrap_or_else(|| display_store_stem(input_store_path));
    let dataset = open_dataset_summary(OpenDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: artifact.store_path.clone(),
    })
    .map_err(|error| error.to_string())?
    .dataset;

    workspace.upsert_entry(UpsertDatasetEntryRequest {
        schema_version: IPC_SCHEMA_VERSION,
        entry_id: None,
        display_name: Some(format!("{source_label} · {}", artifact.label)),
        source_path: None,
        preferred_store_path: Some(dataset.store_path.clone()),
        imported_store_path: Some(dataset.store_path.clone()),
        dataset: Some(dataset),
        session_pipelines: source_entry.map(|entry| entry.session_pipelines.clone()),
        active_session_pipeline_id: source_entry
            .and_then(|entry| entry.active_session_pipeline_id.clone()),
        make_active: false,
    })?;

    Ok(())
}

fn import_store_path_for_input(
    dir: &Path,
    input_path: &str,
    extension: &str,
) -> Result<String, String> {
    let input_path = input_path.trim();
    if input_path.is_empty() {
        return Err("Input path is required.".to_string());
    }

    fs::create_dir_all(dir).map_err(|error| error.to_string())?;

    let source = Path::new(input_path);
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("volume");
    let sanitized_stem = sanitized_stem(stem, "volume");

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input_path.to_ascii_lowercase().hash(&mut hasher);
    let fingerprint = hasher.finish();
    let store_name = format!("{sanitized_stem}-{fingerprint:016x}.{extension}");
    Ok(dir.join(store_name).display().to_string())
}

fn import_volume_store_path_for_input(
    app_paths: &AppPaths,
    input_path: &str,
) -> Result<String, String> {
    import_store_path_for_input(app_paths.imported_volumes_dir(), input_path, "tbvol")
}

fn import_prestack_store_path_for_input(
    app_paths: &AppPaths,
    input_path: &str,
) -> Result<String, String> {
    import_store_path_for_input(app_paths.imported_gathers_dir(), input_path, "tbgath")
}

#[tauri::command]
fn default_import_store_path_command(app: AppHandle, input_path: String) -> Result<String, String> {
    let app_paths = AppPaths::resolve(&app)?;
    import_volume_store_path_for_input(&app_paths, &input_path)
}

#[tauri::command]
fn default_import_prestack_store_path_command(
    app: AppHandle,
    input_path: String,
) -> Result<String, String> {
    let app_paths = AppPaths::resolve(&app)?;
    import_prestack_store_path_for_input(&app_paths, &input_path)
}

#[tauri::command]
fn default_processing_store_path_command(
    app: AppHandle,
    store_path: String,
    pipeline: seis_runtime::TraceLocalProcessingPipeline,
) -> Result<String, String> {
    let app_paths = AppPaths::resolve(&app)?;
    default_processing_store_path(&app_paths, &store_path, &pipeline)
}

#[tauri::command]
fn default_subvolume_processing_store_path_command(
    app: AppHandle,
    store_path: String,
    pipeline: SubvolumeProcessingPipeline,
) -> Result<String, String> {
    let app_paths = AppPaths::resolve(&app)?;
    default_subvolume_processing_store_path(&app_paths, &store_path, &pipeline)
}

#[tauri::command]
fn default_gather_processing_store_path_command(
    app: AppHandle,
    store_path: String,
    pipeline: seis_runtime::GatherProcessingPipeline,
) -> Result<String, String> {
    let app_paths = AppPaths::resolve(&app)?;
    default_gather_processing_store_path(&app_paths, &store_path, &pipeline)
}

fn build_app_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let settings = MenuItem::with_id(
        app,
        APP_SETTINGS_MENU_ID,
        "&Settings...",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let velocity_model = MenuItem::with_id(
        app,
        APP_VELOCITY_MODEL_MENU_ID,
        "&Velocity Model...",
        true,
        None::<&str>,
    )?;
    let residuals = MenuItem::with_id(
        app,
        APP_RESIDUALS_MENU_ID,
        "&Residuals...",
        true,
        None::<&str>,
    )?;
    let depth_conversion = MenuItem::with_id(
        app,
        APP_DEPTH_CONVERSION_MENU_ID,
        "&Depth Conversion...",
        true,
        None::<&str>,
    )?;
    let well_tie = MenuItem::with_id(
        app,
        APP_WELL_TIE_MENU_ID,
        "&Well Tie...",
        true,
        None::<&str>,
    )?;
    let open_volume = MenuItem::with_id(
        app,
        FILE_OPEN_VOLUME_MENU_ID,
        "&Open Volume...",
        true,
        None::<&str>,
    )?;
    let import_data = MenuItem::with_id(
        app,
        FILE_IMPORT_DATA_MENU_ID,
        "Import &Data...",
        true,
        None::<&str>,
    )?;
    let import_seismic = MenuItem::with_id(
        app,
        FILE_IMPORT_SEISMIC_MENU_ID,
        "&Seismic Volume...",
        true,
        None::<&str>,
    )?;
    let import_horizons = MenuItem::with_id(
        app,
        FILE_IMPORT_HORIZONS_MENU_ID,
        "&Horizons...",
        true,
        None::<&str>,
    )?;
    let import_well_sources = MenuItem::with_id(
        app,
        FILE_IMPORT_WELL_SOURCES_MENU_ID,
        "Well &Files...",
        true,
        None::<&str>,
    )?;
    let import_velocity_functions = MenuItem::with_id(
        app,
        FILE_IMPORT_VELOCITY_FUNCTIONS_MENU_ID,
        "&Velocity Functions...",
        true,
        None::<&str>,
    )?;
    let import_checkshot = MenuItem::with_id(
        app,
        FILE_IMPORT_CHECKSHOT_MENU_ID,
        "Well &Checkshot/VSP...",
        true,
        None::<&str>,
    )?;
    let import_manual_picks = MenuItem::with_id(
        app,
        FILE_IMPORT_MANUAL_PICKS_MENU_ID,
        "Well Manual &Picks...",
        true,
        None::<&str>,
    )?;
    let import_authored_well_model = MenuItem::with_id(
        app,
        FILE_IMPORT_AUTHORED_WELL_MODEL_MENU_ID,
        "Well &Authored Model...",
        true,
        None::<&str>,
    )?;
    let import_compiled_well_model = MenuItem::with_id(
        app,
        FILE_IMPORT_COMPILED_WELL_MODEL_MENU_ID,
        "Well C&ompiled Model...",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let close_window = PredefinedMenuItem::close_window(app, None)?;
    let import_separator = PredefinedMenuItem::separator(app)?;
    let import_submenu = Submenu::with_items(
        app,
        "&Import",
        true,
        &[
            &import_seismic,
            &import_horizons,
            &import_well_sources,
            &import_velocity_functions,
            &import_separator,
            &import_checkshot,
            &import_manual_picks,
            &import_authored_well_model,
            &import_compiled_well_model,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            &Submenu::with_items(
                app,
                "&TraceBoost",
                true,
                &[
                    &settings,
                    &velocity_model,
                    &residuals,
                    &depth_conversion,
                    &well_tie,
                ],
            )?,
            &Submenu::with_items(
                app,
                "&File",
                true,
                &[
                    &open_volume,
                    &import_data,
                    &import_submenu,
                    &separator,
                    &close_window,
                ],
            )?,
        ],
    )
}

#[tauri::command]
fn preflight_import_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    input_path: String,
    geometry_override: Option<seis_contracts_operations::SegyGeometryOverride>,
) -> Result<SurveyPreflightResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "preflight_import",
        "Starting survey preflight",
        Some(build_fields([
            ("inputPath", json_value(&input_path)),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.verbose_progress(
        &app,
        &operation,
        "Validated preflight inputs",
        Some(build_fields([("inputPath", json_value(&input_path))])),
    );

    diagnostics.progress(
        &app,
        &operation,
        "Inspecting SEG-Y survey metadata",
        Some(build_fields([("stage", json_value("inspect_segy"))])),
    );

    let result = workflow_service().preflight_dataset(SurveyPreflightRequest {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
        geometry_override,
    });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Survey preflight completed",
                Some(build_fields([
                    ("stage", json_value("summarize")),
                    ("classification", json_value(&response.classification)),
                    ("stackingState", json_value(&response.stacking_state)),
                    ("organization", json_value(&response.organization)),
                    ("layout", json_value(&response.layout)),
                    ("traceCount", json_value(response.trace_count)),
                    ("samplesPerTrace", json_value(response.samples_per_trace)),
                    ("completenessRatio", json_value(response.completeness_ratio)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Survey preflight failed",
                Some(build_fields([
                    ("stage", json_value("inspect_segy")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn scan_segy_import_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    input_path: String,
) -> Result<SegyImportScanResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "scan_segy_import",
        "Starting SEG-Y import scan",
        Some(build_fields([
            ("inputPath", json_value(&input_path)),
            ("stage", json_value("scan_input")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Inspecting SEG-Y layout and candidate mappings",
        Some(build_fields([("stage", json_value("inspect_segy"))])),
    );

    let result = workflow_service().scan_segy_import(ScanSegyImportRequest {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
    });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "SEG-Y import scan completed",
                Some(build_fields([
                    ("stage", json_value("summarize")),
                    ("traceCount", json_value(response.trace_count)),
                    ("candidateCount", json_value(response.candidate_plans.len())),
                    (
                        "recommendedNextStage",
                        json_value(format!("{:?}", response.recommended_next_stage)),
                    ),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "SEG-Y import scan failed",
                Some(build_fields([("error", json_value(&message))])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn validate_segy_import_plan_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: ValidateSegyImportPlanRequest,
) -> Result<SegyImportValidationResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "validate_segy_import_plan",
        "Validating SEG-Y import plan",
        Some(build_fields([
            ("inputPath", json_value(&request.plan.input_path)),
            ("stage", json_value("validate_plan")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Re-running SEG-Y validation with the selected plan",
        Some(build_fields([("stage", json_value("inspect_segy"))])),
    );

    let result = workflow_service().validate_segy_import_plan(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "SEG-Y import plan validated",
                Some(build_fields([
                    ("stage", json_value("summarize")),
                    ("canImport", json_value(response.can_import)),
                    (
                        "requiresAcknowledgement",
                        json_value(response.requires_acknowledgement),
                    ),
                    ("issueCount", json_value(response.issues.len())),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "SEG-Y import validation failed",
                Some(build_fields([("error", json_value(&message))])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn import_segy_with_plan_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: ImportSegyWithPlanRequest,
) -> Result<ImportSegyWithPlanResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "import_segy_with_plan",
        "Starting SEG-Y import from validated plan",
        Some(build_fields([
            ("inputPath", json_value(&request.plan.input_path)),
            (
                "outputStorePath",
                json_value(&request.plan.policy.output_store_path),
            ),
            ("stage", json_value("read_input")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Reading SEG-Y input and building runtime store",
        Some(build_fields([("stage", json_value("read_input"))])),
    );

    let result = workflow_service().import_segy_with_plan(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "SEG-Y import completed",
                Some(build_fields([
                    ("stage", json_value("persist_store")),
                    ("storePath", json_value(&response.dataset.store_path)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "SEG-Y import failed",
                Some(build_fields([
                    ("stage", json_value("read_input")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn import_dataset_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    input_path: String,
    output_store_path: String,
    geometry_override: Option<seis_contracts_operations::SegyGeometryOverride>,
    overwrite_existing: bool,
) -> Result<ImportDatasetResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "import_dataset",
        "Starting volume import",
        Some(build_fields([
            ("inputPath", json_value(&input_path)),
            ("outputStorePath", json_value(&output_store_path)),
            ("overwriteExisting", json_value(overwrite_existing)),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.verbose_progress(
        &app,
        &operation,
        "Validated import inputs",
        Some(build_fields([
            ("inputPath", json_value(&input_path)),
            ("outputStorePath", json_value(&output_store_path)),
            ("overwriteExisting", json_value(overwrite_existing)),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Reading input volume and building runtime store",
        Some(build_fields([("stage", json_value("read_input"))])),
    );

    let result = workflow_service().import_dataset(ImportDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        input_path,
        output_store_path,
        geometry_override,
        overwrite_existing,
    });

    match result {
        Ok(response) => {
            diagnostics.progress(
                &app,
                &operation,
                "Finalizing runtime store metadata",
                Some(build_fields([
                    ("stage", json_value("finalize_store")),
                    ("storePath", json_value(&response.dataset.store_path)),
                ])),
            );
            diagnostics.complete(
                &app,
                &operation,
                "Volume import completed",
                Some(build_fields([
                    ("storePath", json_value(&response.dataset.store_path)),
                    ("datasetId", json_value(&response.dataset.descriptor.id.0)),
                    (
                        "datasetLabel",
                        json_value(&response.dataset.descriptor.label),
                    ),
                    ("shape", json_value(response.dataset.descriptor.shape)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Volume import failed",
                Some(build_fields([
                    ("stage", json_value("build_store")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn list_segy_import_recipes_command(
    recipes: State<SegyImportRecipeState>,
    request: ListSegyImportRecipesRequest,
) -> Result<ListSegyImportRecipesResponse, String> {
    Ok(ListSegyImportRecipesResponse {
        schema_version: IPC_SCHEMA_VERSION,
        recipes: recipes.list_recipes(request.source_fingerprint.as_deref())?,
    })
}

#[tauri::command]
fn save_segy_import_recipe_command(
    recipes: State<SegyImportRecipeState>,
    request: SaveSegyImportRecipeRequest,
) -> Result<SaveSegyImportRecipeResponse, String> {
    Ok(SaveSegyImportRecipeResponse {
        schema_version: IPC_SCHEMA_VERSION,
        recipe: recipes.save_recipe(request.recipe)?,
    })
}

#[tauri::command]
fn delete_segy_import_recipe_command(
    recipes: State<SegyImportRecipeState>,
    request: DeleteSegyImportRecipeRequest,
) -> Result<DeleteSegyImportRecipeResponse, String> {
    Ok(DeleteSegyImportRecipeResponse {
        schema_version: IPC_SCHEMA_VERSION,
        deleted: recipes.delete_recipe(&request.recipe_id)?,
    })
}

#[tauri::command]
fn import_prestack_offset_dataset_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: ImportPrestackOffsetDatasetRequest,
) -> Result<ImportPrestackOffsetDatasetResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "import_prestack_offset_dataset",
        "Starting prestack SEG-Y import",
        Some(build_fields([
            ("inputPath", json_value(&request.input_path)),
            ("outputStorePath", json_value(&request.output_store_path)),
            ("overwriteExisting", json_value(request.overwrite_existing)),
            (
                "thirdAxisField",
                json_value(format!("{:?}", request.third_axis_field).to_ascii_lowercase()),
            ),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.verbose_progress(
        &app,
        &operation,
        "Validated prestack import inputs",
        Some(build_fields([
            ("inputPath", json_value(&request.input_path)),
            ("outputStorePath", json_value(&request.output_store_path)),
            ("overwriteExisting", json_value(request.overwrite_existing)),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Reading SEG-Y gather survey and building prestack runtime store",
        Some(build_fields([("stage", json_value("read_segy"))])),
    );

    let result = workflow_service().import_prestack_offset_dataset(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Prestack SEG-Y import completed",
                Some(build_fields([
                    ("storePath", json_value(&response.dataset.store_path)),
                    ("datasetId", json_value(&response.dataset.descriptor.id.0)),
                    (
                        "datasetLabel",
                        json_value(&response.dataset.descriptor.label),
                    ),
                    ("shape", json_value(response.dataset.descriptor.shape)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Prestack SEG-Y import failed",
                Some(build_fields([
                    ("stage", json_value("build_store")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn open_dataset_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
) -> Result<OpenDatasetResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "open_dataset",
        "Opening runtime store",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Loading runtime store summary",
        Some(build_fields([("stage", json_value("open_store"))])),
    );

    let result = workflow_service().open_dataset_summary(OpenDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path,
    });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Runtime store opened",
                Some(build_fields([
                    ("stage", json_value("summarize")),
                    ("storePath", json_value(&response.dataset.store_path)),
                    ("datasetId", json_value(&response.dataset.descriptor.id.0)),
                    (
                        "datasetLabel",
                        json_value(&response.dataset.descriptor.label),
                    ),
                    ("shape", json_value(response.dataset.descriptor.shape)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Opening runtime store failed",
                Some(build_fields([
                    ("stage", json_value("open_store")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn export_dataset_segy_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    output_path: String,
    overwrite_existing: bool,
) -> Result<ExportSegyResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "export_dataset_segy",
        "Starting SEG-Y export",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("outputPath", json_value(&output_path)),
            ("overwriteExisting", json_value(overwrite_existing)),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Reading tbvol export metadata and writing SEG-Y",
        Some(build_fields([("stage", json_value("write_segy"))])),
    );

    let result = workflow_service().export_dataset_segy(ExportSegyRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path,
        output_path,
        overwrite_existing,
    });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "SEG-Y export completed",
                Some(build_fields([
                    ("stage", json_value("complete")),
                    ("storePath", json_value(&response.store_path)),
                    ("outputPath", json_value(&response.output_path)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "SEG-Y export failed",
                Some(build_fields([
                    ("stage", json_value("write_segy")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn ensure_demo_survey_time_depth_transform_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
) -> Result<String, String> {
    let operation = diagnostics.start_operation(
        &app,
        "ensure_demo_survey_time_depth_transform",
        "Ensuring synthetic survey 3D time-depth transform",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Building or refreshing the synthetic survey-aligned transform asset",
        Some(build_fields([("stage", json_value("build_transform"))])),
    );

    match workflow_service().ensure_demo_survey_time_depth_transform(store_path.clone()) {
        Ok(asset_id) => {
            diagnostics.complete(
                &app,
                &operation,
                "Synthetic survey 3D transform is ready",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("assetId", json_value(&asset_id)),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(asset_id)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Ensuring synthetic survey 3D transform failed",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("stage", json_value("build_transform")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_velocity_models_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
) -> Result<LoadVelocityModelsResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "load_velocity_models",
        "Loading survey velocity models",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("stage", json_value("load_velocity_models")),
        ])),
    );

    match workflow_service().load_velocity_models(store_path.clone()) {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Velocity models loaded",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("modelCount", json_value(response.models.len())),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Loading velocity models failed",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("stage", json_value("load_velocity_models")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_horizon_assets_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
) -> Result<Vec<seis_runtime::ImportedHorizonDescriptor>, String> {
    let operation = diagnostics.start_operation(
        &app,
        "load_horizon_assets",
        "Loading survey horizon assets",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("stage", json_value("load_horizon_assets")),
        ])),
    );

    match load_horizon_assets(store_path.clone()) {
        Ok(horizons) => {
            diagnostics.complete(
                &app,
                &operation,
                "Horizon assets loaded",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("horizonCount", json_value(horizons.len())),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(horizons)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Loading horizon assets failed",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("stage", json_value("load_horizon_assets")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn import_velocity_functions_model_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    input_path: String,
    velocity_kind: VelocityQuantityKind,
) -> Result<traceboost_app::ImportVelocityFunctionsModelResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "import_velocity_functions_model",
        "Importing sparse velocity functions into a survey transform",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("inputPath", json_value(&input_path)),
            (
                "velocityKind",
                json_value(format!("{velocity_kind:?}").to_ascii_lowercase()),
            ),
            ("stage", json_value("parse_and_build")),
        ])),
    );

    match workflow_service().import_velocity_functions_model(
        store_path.clone(),
        input_path.clone(),
        velocity_kind,
    ) {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Velocity functions imported and compiled into a survey transform",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("inputPath", json_value(&input_path)),
                    ("assetId", json_value(&response.model.id)),
                    ("profileCount", json_value(response.profile_count)),
                    ("sampleCount", json_value(response.sample_count)),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Velocity functions import failed",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("inputPath", json_value(&input_path)),
                    ("stage", json_value("parse_and_build")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn describe_velocity_volume_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: DescribeVelocityVolumeRequest,
) -> Result<DescribeVelocityVolumeResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "describe_velocity_volume",
        "Describing velocity volume store",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            (
                "velocityKind",
                json_value(format!("{:?}", request.velocity_kind).to_ascii_lowercase()),
            ),
            (
                "verticalDomain",
                json_value(
                    request
                        .vertical_domain
                        .map(|domain| format!("{domain:?}").to_ascii_lowercase()),
                ),
            ),
            ("stage", json_value("describe_velocity_volume")),
        ])),
    );

    match workflow_service().describe_velocity_volume(request.clone()) {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Velocity volume described",
                Some(build_fields([
                    ("storePath", json_value(&request.store_path)),
                    (
                        "sampleCount",
                        json_value(response.volume.vertical_axis.count),
                    ),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Describing velocity volume failed",
                Some(build_fields([
                    ("storePath", json_value(&request.store_path)),
                    ("stage", json_value("describe_velocity_volume")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn ingest_velocity_volume_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: IngestVelocityVolumeRequest,
) -> Result<IngestVelocityVolumeResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "ingest_velocity_volume",
        "Ingesting velocity volume into tbvol",
        Some(build_fields([
            ("inputPath", json_value(&request.input_path)),
            ("outputStorePath", json_value(&request.output_store_path)),
            (
                "velocityKind",
                json_value(format!("{:?}", request.velocity_kind).to_ascii_lowercase()),
            ),
            (
                "deleteInputOnSuccess",
                json_value(request.delete_input_on_success),
            ),
            ("stage", json_value("ingest_velocity_volume")),
        ])),
    );

    match workflow_service().ingest_velocity_volume_request(request.clone()) {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Velocity volume ingested",
                Some(build_fields([
                    ("storePath", json_value(&response.store_path)),
                    ("deletedInput", json_value(response.deleted_input)),
                    (
                        "sampleCount",
                        json_value(response.volume.vertical_axis.count),
                    ),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Ingesting velocity volume failed",
                Some(build_fields([
                    ("inputPath", json_value(&request.input_path)),
                    ("outputStorePath", json_value(&request.output_store_path)),
                    ("stage", json_value("ingest_velocity_volume")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn build_velocity_model_transform_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: BuildSurveyTimeDepthTransformRequest,
) -> Result<seis_contracts_operations::SurveyTimeDepthTransform3D, String> {
    let operation = diagnostics.start_operation(
        &app,
        "build_velocity_model_transform",
        "Building authored velocity model into a survey transform",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            ("modelId", json_value(&request.model.id)),
            ("modelName", json_value(&request.model.name)),
            ("intervalCount", json_value(request.model.intervals.len())),
            ("stage", json_value("build_transform")),
        ])),
    );

    match build_velocity_model_transform(request) {
        Ok(model) => {
            diagnostics.complete(
                &app,
                &operation,
                "Velocity model compiled into a survey transform",
                Some(build_fields([
                    ("assetId", json_value(&model.id)),
                    ("modelName", json_value(&model.name)),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(model)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Velocity model build failed",
                Some(build_fields([
                    ("stage", json_value("build_transform")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn export_dataset_zarr_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    output_path: String,
    overwrite_existing: bool,
) -> Result<ExportZarrResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "export_dataset_zarr",
        "Starting Zarr export",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("outputPath", json_value(&output_path)),
            ("overwriteExisting", json_value(overwrite_existing)),
            ("stage", json_value("validate_input")),
        ])),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Reading tbvol data and writing Zarr",
        Some(build_fields([("stage", json_value("write_zarr"))])),
    );

    match export_dataset_zarr(store_path, output_path, overwrite_existing) {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Zarr export completed",
                Some(build_fields([
                    ("stage", json_value("complete")),
                    ("storePath", json_value(&response.store_path)),
                    ("outputPath", json_value(&response.output_path)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Zarr export failed",
                Some(build_fields([
                    ("stage", json_value("write_zarr")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn convert_horizon_domain_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    source_horizon_id: String,
    transform_id: String,
    target_domain: seis_runtime::TimeDepthDomain,
    output_id: Option<String>,
    output_name: Option<String>,
) -> Result<seis_runtime::ImportedHorizonDescriptor, String> {
    let target_domain_label = match &target_domain {
        seis_runtime::TimeDepthDomain::Time => "time",
        seis_runtime::TimeDepthDomain::Depth => "depth",
    };
    let operation = diagnostics.start_operation(
        &app,
        "convert_horizon_domain",
        "Converting survey horizon between time and depth",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("sourceHorizonId", json_value(&source_horizon_id)),
            ("transformId", json_value(&transform_id)),
            ("targetDomain", json_value(target_domain_label)),
            ("stage", json_value("convert_horizon")),
        ])),
    );

    match convert_horizon_domain(
        store_path.clone(),
        source_horizon_id.clone(),
        transform_id.clone(),
        target_domain,
        output_id,
        output_name,
    ) {
        Ok(descriptor) => {
            diagnostics.complete(
                &app,
                &operation,
                "Survey horizon converted",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("sourceHorizonId", json_value(&source_horizon_id)),
                    ("transformId", json_value(&transform_id)),
                    ("outputHorizonId", json_value(&descriptor.id)),
                    ("targetDomain", json_value(target_domain_label)),
                    ("stage", json_value("complete")),
                ])),
            );
            Ok(descriptor)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Survey horizon conversion failed",
                Some(build_fields([
                    ("storePath", json_value(&store_path)),
                    ("sourceHorizonId", json_value(&source_horizon_id)),
                    ("transformId", json_value(&transform_id)),
                    ("targetDomain", json_value(target_domain_label)),
                    ("stage", json_value("convert_horizon")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn get_dataset_export_capabilities_command(
    store_path: String,
) -> Result<DatasetExportCapabilitiesResponse, String> {
    let handle = open_store(&store_path).map_err(|error| error.to_string())?;
    let segy = match handle.manifest.volume.segy_export.as_ref() {
        Some(descriptor) if descriptor.contains_synthetic_traces => DatasetExportFormatCapability {
            available: false,
            reason: Some(
                "SEG-Y export is unavailable because this volume contains synthetic or regularized traces."
                    .to_string(),
            ),
            default_output_path: default_export_segy_path(&store_path)
                .display()
                .to_string(),
        },
        Some(_) => DatasetExportFormatCapability {
            available: true,
            reason: None,
            default_output_path: default_export_segy_path(&store_path)
                .display()
                .to_string(),
        },
        None => DatasetExportFormatCapability {
            available: false,
            reason: Some(
                "SEG-Y export is unavailable because this tbvol does not carry captured SEG-Y provenance."
                    .to_string(),
            ),
            default_output_path: default_export_segy_path(&store_path)
                .display()
                .to_string(),
        },
    };
    let zarr = DatasetExportFormatCapability {
        available: true,
        reason: None,
        default_output_path: default_export_zarr_path(&store_path).display().to_string(),
    };

    Ok(DatasetExportCapabilitiesResponse {
        store_path,
        segy,
        zarr,
    })
}

#[tauri::command]
fn preview_horizon_xyz_import_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    input_paths: Vec<String>,
    vertical_domain: Option<TimeDepthDomain>,
    vertical_unit: Option<String>,
    source_coordinate_reference_id: Option<String>,
    source_coordinate_reference_name: Option<String>,
    assume_same_as_survey: bool,
) -> Result<seis_runtime::HorizonImportPreview, String> {
    let operation = diagnostics.start_operation(
        &app,
        "preview_horizon_xyz_import",
        "Previewing horizon xyz import",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("inputPathCount", json_value(input_paths.len())),
            (
                "verticalDomain",
                json_value(vertical_domain.as_ref().map(|value| format!("{value:?}"))),
            ),
            ("verticalUnit", json_value(vertical_unit.as_deref())),
            (
                "sourceCoordinateReferenceId",
                json_value(source_coordinate_reference_id.as_deref()),
            ),
            ("assumeSameAsSurvey", json_value(assume_same_as_survey)),
            ("stage", json_value("validate_input")),
        ])),
    );

    let result = workflow_service().preview_horizon_xyz_import(ImportHorizonXyzRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path,
        input_paths,
        vertical_domain,
        vertical_unit,
        source_coordinate_reference_id,
        source_coordinate_reference_name,
        assume_same_as_survey,
    });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Horizon xyz import preview ready",
                Some(build_fields([
                    ("stage", json_value("preview_horizons")),
                    ("fileCount", json_value(response.files.len())),
                    ("canCommit", json_value(response.can_commit)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Horizon xyz import preview failed",
                Some(build_fields([
                    ("stage", json_value("preview_horizons")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn preview_horizon_source_import_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: PreviewHorizonSourceImportRequest,
) -> Result<seis_runtime::HorizonSourceImportPreview, String> {
    let operation = diagnostics.start_operation(
        &app,
        "preview_horizon_source_import",
        "Previewing horizon source import",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            ("inputPathCount", json_value(request.input_paths.len())),
            (
                "selectedSourcePathCount",
                json_value(
                    request
                        .draft
                        .as_ref()
                        .map(|draft| draft.selected_source_paths.len()),
                ),
            ),
            (
                "verticalDomain",
                json_value(
                    request
                        .draft
                        .as_ref()
                        .map(|draft| format!("{:?}", draft.vertical_domain)),
                ),
            ),
            (
                "assumeSameAsSurvey",
                json_value(
                    request
                        .draft
                        .as_ref()
                        .map(|draft| draft.assume_same_as_survey),
                ),
            ),
            ("stage", json_value("preview_horizons")),
        ])),
    );

    let input_paths = request
        .input_paths
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let result = seis_runtime::preview_horizon_source_import(
        &request.store_path,
        &input_paths,
        request.draft.as_ref(),
    );

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Horizon source import preview ready",
                Some(build_fields([
                    ("stage", json_value("preview_horizons")),
                    ("fileCount", json_value(response.parsed.files.len())),
                    ("canCommit", json_value(response.parsed.can_commit)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Horizon source import preview failed",
                Some(build_fields([
                    ("stage", json_value("preview_horizons")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn inspect_horizon_xyz_files_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    input_paths: Vec<String>,
) -> Result<Vec<ophiolite::HorizonXyzFilePreview>, String> {
    let operation = diagnostics.start_operation(
        &app,
        "inspect_horizon_xyz_files",
        "Inspecting horizon xyz files",
        Some(build_fields([
            ("inputPathCount", json_value(input_paths.len())),
            ("stage", json_value("preview_horizons")),
        ])),
    );
    let resolved_paths = input_paths
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let result =
        ophiolite::inspect_horizon_xyz_files(&resolved_paths).map_err(|error| error.to_string());
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Horizon xyz inspection ready",
                Some(build_fields([
                    ("stage", json_value("preview_horizons")),
                    ("fileCount", json_value(response.len())),
                    (
                        "invalidRowCount",
                        json_value(
                            response
                                .iter()
                                .map(|file| file.invalid_row_count)
                                .sum::<usize>(),
                        ),
                    ),
                ])),
            );
            Ok(response)
        }
        Err(message) => {
            diagnostics.fail(
                &app,
                &operation,
                "Horizon xyz inspection failed",
                Some(build_fields([
                    ("stage", json_value("preview_horizons")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn commit_horizon_source_import_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: CommitHorizonSourceImportRequest,
) -> Result<ImportHorizonXyzResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "commit_horizon_source_import",
        "Importing horizons from source draft",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            (
                "selectedSourcePathCount",
                json_value(request.draft.selected_source_paths.len()),
            ),
            (
                "verticalDomain",
                json_value(format!("{:?}", request.draft.vertical_domain)),
            ),
            (
                "assumeSameAsSurvey",
                json_value(request.draft.assume_same_as_survey),
            ),
            ("stage", json_value("import_horizons")),
        ])),
    );

    let result = seis_runtime::import_horizon_xyzs_from_draft(&request.store_path, &request.draft)
        .map(|imported| ImportHorizonXyzResponse {
            schema_version: IPC_SCHEMA_VERSION,
            imported,
        });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Horizon source import completed",
                Some(build_fields([
                    ("stage", json_value("import_horizons")),
                    ("importedCount", json_value(response.imported.len())),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Horizon source import failed",
                Some(build_fields([
                    ("stage", json_value("import_horizons")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn import_horizon_xyz_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    input_paths: Vec<String>,
    vertical_domain: Option<TimeDepthDomain>,
    vertical_unit: Option<String>,
    source_coordinate_reference_id: Option<String>,
    source_coordinate_reference_name: Option<String>,
    assume_same_as_survey: bool,
) -> Result<ImportHorizonXyzResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "import_horizon_xyz",
        "Importing horizon xyz files",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("inputPathCount", json_value(input_paths.len())),
            (
                "verticalDomain",
                json_value(vertical_domain.as_ref().map(|value| format!("{value:?}"))),
            ),
            ("verticalUnit", json_value(vertical_unit.as_deref())),
            (
                "sourceCoordinateReferenceId",
                json_value(source_coordinate_reference_id.as_deref()),
            ),
            ("assumeSameAsSurvey", json_value(assume_same_as_survey)),
            ("stage", json_value("validate_input")),
        ])),
    );

    let result = import_horizon_xyz(ImportHorizonXyzRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path,
        input_paths,
        vertical_domain,
        vertical_unit,
        source_coordinate_reference_id,
        source_coordinate_reference_name,
        assume_same_as_survey,
    });

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Horizon xyz files imported",
                Some(build_fields([
                    ("stage", json_value("import_horizons")),
                    ("horizonCount", json_value(response.imported.len())),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Horizon xyz import failed",
                Some(build_fields([
                    ("stage", json_value("import_horizons")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_section_horizons_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    axis: SectionAxis,
    index: usize,
) -> Result<LoadSectionHorizonsResponse, String> {
    let axis_name = format!("{axis:?}").to_ascii_lowercase();
    let operation = diagnostics.start_operation(
        &app,
        "load_section_horizons",
        "Loading section horizon overlays",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("axis", json_value(&axis_name)),
            ("index", json_value(index)),
            ("stage", json_value("validate_input")),
        ])),
    );

    let result = workflow_service().load_section_horizons(
        seis_contracts_operations::LoadSectionHorizonsRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path,
            axis,
            index,
        },
    );

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Section horizon overlays loaded",
                Some(build_fields([
                    ("stage", json_value("load_section_horizons")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("horizonCount", json_value(response.overlays.len())),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Loading section horizon overlays failed",
                Some(build_fields([
                    ("stage", json_value("load_section_horizons")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_section_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    axis: SectionAxis,
    index: usize,
) -> Result<SectionView, String> {
    let axis_name = format!("{axis:?}").to_ascii_lowercase();
    let handle = open_store(&store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&store_path)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(index)),
        ("stage", json_value("validate_input")),
    ];
    if let Some(handle) = handle.as_ref() {
        start_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
        start_fields.push(("shape", json_value(handle.manifest.volume.shape)));
        append_section_debug_fields(
            &mut start_fields,
            handle,
            axis,
            index,
            "axisLength",
            "axisRange",
            "requestedCoordinateValue",
        );
    }
    let operation = diagnostics.start_operation(
        &app,
        "load_section",
        "Loading section view",
        Some(build_fields(start_fields)),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Opening runtime store for section load",
        Some(build_fields([("stage", json_value("open_store"))])),
    );

    let result = match handle {
        Some(handle) => handle.section_view(axis, index),
        None => open_store(store_path.clone()).and_then(|handle| handle.section_view(axis, index)),
    };
    match result {
        Ok(section) => {
            diagnostics.complete(
                &app,
                &operation,
                "Section view loaded",
                Some(build_fields([
                    ("stage", json_value("load_section")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("traces", json_value(section.traces)),
                    ("samples", json_value(section.samples)),
                ])),
            );
            Ok(section)
        }
        Err(error) => {
            let message = error.to_string();
            let mut failure_fields = vec![
                ("stage", json_value("load_section")),
                ("axis", json_value(&axis_name)),
                ("index", json_value(index)),
                ("error", json_value(&message)),
            ];
            if let Some(handle) = open_store(&store_path).ok() {
                failure_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
                failure_fields.push(("shape", json_value(handle.manifest.volume.shape)));
                append_section_debug_fields(
                    &mut failure_fields,
                    &handle,
                    axis,
                    index,
                    "axisLength",
                    "axisRange",
                    "requestedCoordinateValue",
                );
            }
            diagnostics.fail(
                &app,
                &operation,
                "Section load failed",
                Some(build_fields(failure_fields)),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_section_binary_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    axis: SectionAxis,
    index: usize,
) -> Result<Response, String> {
    let axis_name = format!("{axis:?}").to_ascii_lowercase();
    let handle = open_store(&store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&store_path)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(index)),
        ("stage", json_value("validate_input")),
    ];
    if let Some(handle) = handle.as_ref() {
        start_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
        start_fields.push(("shape", json_value(handle.manifest.volume.shape)));
        append_section_debug_fields(
            &mut start_fields,
            handle,
            axis,
            index,
            "axisLength",
            "axisRange",
            "requestedCoordinateValue",
        );
    }
    let operation = diagnostics.start_operation(
        &app,
        "load_section_binary",
        "Loading section view (binary)",
        Some(build_fields(start_fields)),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Opening runtime store for binary section load",
        Some(build_fields([("stage", json_value("open_store"))])),
    );

    let result = match handle {
        Some(handle) => handle.section_view(axis, index),
        None => open_store(store_path.clone()).and_then(|handle| handle.section_view(axis, index)),
    };
    match result {
        Ok(section) => {
            let payload_bytes = section.horizontal_axis_f64le.len()
                + section
                    .inline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + section
                    .xline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + section.sample_axis_f32le.len()
                + section.amplitudes_f32le.len();
            diagnostics.complete(
                &app,
                &operation,
                "Section view loaded (binary)",
                Some(build_fields([
                    ("stage", json_value("load_section_binary")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("traces", json_value(section.traces)),
                    ("samples", json_value(section.samples)),
                    ("payloadBytes", json_value(payload_bytes)),
                ])),
            );
            pack_section_response(section)
        }
        Err(error) => {
            let message = error.to_string();
            let mut failure_fields = vec![
                ("stage", json_value("load_section_binary")),
                ("axis", json_value(&axis_name)),
                ("index", json_value(index)),
                ("error", json_value(&message)),
            ];
            if let Some(handle) = open_store(&store_path).ok() {
                failure_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
                failure_fields.push(("shape", json_value(handle.manifest.volume.shape)));
                append_section_debug_fields(
                    &mut failure_fields,
                    &handle,
                    axis,
                    index,
                    "axisLength",
                    "axisRange",
                    "requestedCoordinateValue",
                );
            }
            diagnostics.fail(
                &app,
                &operation,
                "Loading section view (binary) failed",
                Some(build_fields(failure_fields)),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_section_tile_binary_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    axis: SectionAxis,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
) -> Result<Response, String> {
    let axis_name = format!("{axis:?}").to_ascii_lowercase();
    let handle = open_store(&store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&store_path)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(index)),
        ("traceRange", json_value(trace_range)),
        ("sampleRange", json_value(sample_range)),
        ("lod", json_value(lod)),
        ("stage", json_value("validate_input")),
    ];
    if let Some(handle) = handle.as_ref() {
        start_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
        start_fields.push(("shape", json_value(handle.manifest.volume.shape)));
        append_section_debug_fields(
            &mut start_fields,
            handle,
            axis,
            index,
            "axisLength",
            "axisRange",
            "requestedCoordinateValue",
        );
    }
    let operation = diagnostics.start_operation(
        &app,
        "load_section_tile_binary",
        "Loading section tile view (binary)",
        Some(build_fields(start_fields)),
    );
    diagnostics.progress(
        &app,
        &operation,
        "Opening runtime store for binary section tile load",
        Some(build_fields([("stage", json_value("open_store"))])),
    );

    let result = match handle {
        Some(handle) => handle.section_tile_view(axis, index, trace_range, sample_range, lod),
        None => open_store(store_path.clone()).and_then(|handle| {
            handle.section_tile_view(axis, index, trace_range, sample_range, lod)
        }),
    };
    match result {
        Ok(tile) => {
            let payload_bytes = tile.section.horizontal_axis_f64le.len()
                + tile
                    .section
                    .inline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + tile
                    .section
                    .xline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + tile.section.sample_axis_f32le.len()
                + tile.section.amplitudes_f32le.len();
            diagnostics.complete(
                &app,
                &operation,
                "Section tile view loaded (binary)",
                Some(build_fields([
                    ("stage", json_value("load_section_tile_binary")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("traceRange", json_value(tile.trace_range)),
                    ("sampleRange", json_value(tile.sample_range)),
                    ("lod", json_value(tile.lod)),
                    ("traceStep", json_value(tile.trace_step)),
                    ("sampleStep", json_value(tile.sample_step)),
                    ("traces", json_value(tile.section.traces)),
                    ("samples", json_value(tile.section.samples)),
                    ("payloadBytes", json_value(payload_bytes)),
                ])),
            );
            pack_section_tile_response(tile)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Loading section tile view (binary) failed",
                Some(build_fields([
                    ("stage", json_value("load_section_tile_binary")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("traceRange", json_value(trace_range)),
                    ("sampleRange", json_value(sample_range)),
                    ("lod", json_value(lod)),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_depth_converted_section_binary_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    axis: SectionAxis,
    index: usize,
    velocity_model: VelocityFunctionSource,
    velocity_kind: VelocityQuantityKind,
) -> Result<Response, String> {
    let axis_name = format!("{axis:?}").to_ascii_lowercase();
    let velocity_kind_name = format!("{velocity_kind:?}").to_ascii_lowercase();
    let handle = open_store(&store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&store_path)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(index)),
        ("velocityKind", json_value(&velocity_kind_name)),
        ("stage", json_value("validate_input")),
    ];
    if let Some(handle) = handle.as_ref() {
        start_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
        start_fields.push(("shape", json_value(handle.manifest.volume.shape)));
        append_section_debug_fields(
            &mut start_fields,
            handle,
            axis,
            index,
            "axisLength",
            "axisRange",
            "requestedCoordinateValue",
        );
    }
    let operation = diagnostics.start_operation(
        &app,
        "load_depth_converted_section_binary",
        "Loading depth-converted section view (binary)",
        Some(build_fields(start_fields)),
    );

    let result = load_depth_converted_section(
        store_path.clone(),
        axis,
        index,
        velocity_model,
        velocity_kind,
    );
    match result {
        Ok(section) => {
            let payload_bytes = section.horizontal_axis_f64le.len()
                + section
                    .inline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + section
                    .xline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + section.sample_axis_f32le.len()
                + section.amplitudes_f32le.len();
            diagnostics.complete(
                &app,
                &operation,
                "Depth-converted section view loaded (binary)",
                Some(build_fields([
                    ("stage", json_value("load_depth_converted_section_binary")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("velocityKind", json_value(&velocity_kind_name)),
                    ("traces", json_value(section.traces)),
                    ("samples", json_value(section.samples)),
                    ("payloadBytes", json_value(payload_bytes)),
                ])),
            );
            pack_section_response(section)
        }
        Err(error) => {
            let message = error.to_string();
            let mut failure_fields = vec![
                ("stage", json_value("load_depth_converted_section_binary")),
                ("axis", json_value(&axis_name)),
                ("index", json_value(index)),
                ("velocityKind", json_value(&velocity_kind_name)),
                ("error", json_value(&message)),
            ];
            if let Some(handle) = open_store(&store_path).ok() {
                failure_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
                failure_fields.push(("shape", json_value(handle.manifest.volume.shape)));
                append_section_debug_fields(
                    &mut failure_fields,
                    &handle,
                    axis,
                    index,
                    "axisLength",
                    "axisRange",
                    "requestedCoordinateValue",
                );
            }
            diagnostics.fail(
                &app,
                &operation,
                "Loading depth-converted section view (binary) failed",
                Some(build_fields(failure_fields)),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_resolved_section_display_binary_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    axis: SectionAxis,
    index: usize,
    domain: TimeDepthDomain,
    velocity_model: Option<VelocityFunctionSource>,
    velocity_kind: Option<VelocityQuantityKind>,
    include_velocity_overlay: bool,
) -> Result<Response, String> {
    let axis_name = format!("{axis:?}").to_ascii_lowercase();
    let domain_name = format!("{domain:?}").to_ascii_lowercase();
    let velocity_kind_name = velocity_kind
        .map(|kind| format!("{kind:?}").to_ascii_lowercase())
        .unwrap_or_else(|| "none".to_string());
    let handle = open_store(&store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&store_path)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(index)),
        ("domain", json_value(&domain_name)),
        ("velocityKind", json_value(&velocity_kind_name)),
        (
            "includeVelocityOverlay",
            json_value(include_velocity_overlay),
        ),
        ("stage", json_value("validate_input")),
    ];
    if let Some(handle) = handle.as_ref() {
        start_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
        start_fields.push(("shape", json_value(handle.manifest.volume.shape)));
        append_section_debug_fields(
            &mut start_fields,
            handle,
            axis,
            index,
            "axisLength",
            "axisRange",
            "requestedCoordinateValue",
        );
    }
    let operation = diagnostics.start_operation(
        &app,
        "load_resolved_section_display_binary",
        "Loading resolved section display (binary)",
        Some(build_fields(start_fields)),
    );

    let result = load_resolved_section_display(
        store_path.clone(),
        axis,
        index,
        domain,
        velocity_model,
        velocity_kind,
        include_velocity_overlay,
    );
    match result {
        Ok(display) => {
            let payload_bytes = display.section.horizontal_axis_f64le.len()
                + display
                    .section
                    .inline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + display
                    .section
                    .xline_axis_f64le
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or_default()
                + display.section.sample_axis_f32le.len()
                + display.section.amplitudes_f32le.len()
                + display
                    .scalar_overlays
                    .iter()
                    .map(|overlay| overlay.values_f32le.len())
                    .sum::<usize>();
            diagnostics.complete(
                &app,
                &operation,
                "Resolved section display loaded (binary)",
                Some(build_fields([
                    ("stage", json_value("load_resolved_section_display_binary")),
                    ("axis", json_value(&axis_name)),
                    ("index", json_value(index)),
                    ("domain", json_value(&domain_name)),
                    ("velocityKind", json_value(&velocity_kind_name)),
                    (
                        "includeVelocityOverlay",
                        json_value(include_velocity_overlay),
                    ),
                    ("traces", json_value(display.section.traces)),
                    ("samples", json_value(display.section.samples)),
                    (
                        "scalarOverlayCount",
                        json_value(display.scalar_overlays.len()),
                    ),
                    (
                        "horizonOverlayCount",
                        json_value(display.horizon_overlays.len()),
                    ),
                    ("payloadBytes", json_value(payload_bytes)),
                ])),
            );
            pack_section_display_response(display)
        }
        Err(error) => {
            let message = error.to_string();
            let mut failure_fields = vec![
                ("stage", json_value("load_resolved_section_display_binary")),
                ("axis", json_value(&axis_name)),
                ("index", json_value(index)),
                ("domain", json_value(&domain_name)),
                ("velocityKind", json_value(&velocity_kind_name)),
                (
                    "includeVelocityOverlay",
                    json_value(include_velocity_overlay),
                ),
                ("error", json_value(&message)),
            ];
            if let Some(handle) = open_store(&store_path).ok() {
                failure_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
                failure_fields.push(("shape", json_value(handle.manifest.volume.shape)));
                append_section_debug_fields(
                    &mut failure_fields,
                    &handle,
                    axis,
                    index,
                    "axisLength",
                    "axisRange",
                    "requestedCoordinateValue",
                );
            }
            diagnostics.fail(
                &app,
                &operation,
                "Loading resolved section display (binary) failed",
                Some(build_fields(failure_fields)),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn load_gather_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    store_path: String,
    request: GatherRequest,
) -> Result<GatherView, String> {
    let operation = diagnostics.start_operation(
        &app,
        "load_gather",
        "Loading gather view",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("datasetId", json_value(&request.dataset_id.0)),
            ("stage", json_value("load_gather")),
        ])),
    );

    let result = load_gather(store_path, request);
    match result {
        Ok(gather) => {
            diagnostics.complete(
                &app,
                &operation,
                "Gather view loaded",
                Some(build_fields([
                    ("traces", json_value(gather.traces)),
                    ("samples", json_value(gather.samples)),
                    ("label", json_value(&gather.label)),
                ])),
            );
            Ok(gather)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Gather load failed",
                Some(build_fields([("error", json_value(&message))])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
async fn preview_processing_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    preview_sessions: State<'_, PreviewSessionState>,
    request: PreviewTraceLocalProcessingRequest,
) -> Result<PreviewTraceLocalProcessingResponse, String> {
    let axis_name = format!("{:?}", request.section.axis).to_ascii_lowercase();
    let operator_ids = preview_processing_operation_ids(&request.pipeline);
    let operator_labels = preview_processing_operation_labels(&request.pipeline);
    let pipeline_name = request.pipeline.name.clone();
    let pipeline_revision = request.pipeline.revision;
    let dataset_id = request.section.dataset_id.0.clone();
    let operation = diagnostics.start_operation(
        &app,
        "preview_processing",
        "Generating processing preview",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            ("datasetId", json_value(&dataset_id)),
            ("axis", json_value(&axis_name)),
            ("index", json_value(request.section.index)),
            (
                "operatorCount",
                json_value(request.pipeline.operation_count()),
            ),
            ("pipelineRevision", json_value(pipeline_revision)),
            ("pipelineName", json_value(&pipeline_name)),
            ("operatorIds", json_value(&operator_ids)),
            ("operatorLabels", json_value(&operator_labels)),
            ("stage", json_value("preview_section")),
        ])),
    );

    diagnostics.verbose_progress(
        &app,
        &operation,
        "Dispatching processing preview to runtime session",
        Some(build_fields([
            ("datasetId", json_value(&dataset_id)),
            ("axis", json_value(&axis_name)),
            ("index", json_value(request.section.index)),
            ("operatorCount", json_value(operator_ids.len())),
            ("pipelineRevision", json_value(pipeline_revision)),
            ("operatorIds", json_value(&operator_ids)),
        ])),
    );

    let preview_sessions = preview_sessions.inner().clone();
    let request_for_compute = request;
    let compute_started = Instant::now();
    let result = tauri::async_runtime::spawn_blocking(move || {
        preview_sessions.preview_processing(request_for_compute)
    })
    .await
    .map_err(|error| error.to_string())?;
    let compute_duration_ms = compute_started.elapsed().as_millis();

    match result {
        Ok((response, reuse)) => {
            diagnostics.complete(
                &app,
                &operation,
                "Processing preview ready",
                Some(build_fields([
                    ("pipelineRevision", json_value(pipeline_revision)),
                    ("pipelineName", json_value(&pipeline_name)),
                    ("operatorIds", json_value(&operator_ids)),
                    ("operatorLabels", json_value(&operator_labels)),
                    ("previewReady", json_value(response.preview.preview_ready)),
                    ("traces", json_value(response.preview.section.traces)),
                    ("samples", json_value(response.preview.section.samples)),
                    ("computeDurationMs", json_value(compute_duration_ms)),
                    ("cacheHit", json_value(reuse.cache_hit)),
                    (
                        "reusedPrefixOperations",
                        json_value(reuse.reused_prefix_operations),
                    ),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Processing preview failed",
                Some(build_fields([
                    ("pipelineRevision", json_value(pipeline_revision)),
                    ("pipelineName", json_value(&pipeline_name)),
                    ("operatorIds", json_value(&operator_ids)),
                    ("computeDurationMs", json_value(compute_duration_ms)),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
async fn preview_processing_binary_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    preview_sessions: State<'_, PreviewSessionState>,
    request: PreviewTraceLocalProcessingRequest,
) -> Result<Response, String> {
    let axis_name = format!("{:?}", request.section.axis).to_ascii_lowercase();
    let operator_ids = preview_processing_operation_ids(&request.pipeline);
    let operator_labels = preview_processing_operation_labels(&request.pipeline);
    let pipeline_name = request.pipeline.name.clone();
    let pipeline_revision = request.pipeline.revision;
    let dataset_id = request.section.dataset_id.0.clone();
    let operation = diagnostics.start_operation(
        &app,
        "preview_processing_binary",
        "Generating processing preview (binary)",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            ("datasetId", json_value(&dataset_id)),
            ("axis", json_value(&axis_name)),
            ("index", json_value(request.section.index)),
            (
                "operatorCount",
                json_value(request.pipeline.operation_count()),
            ),
            ("pipelineRevision", json_value(pipeline_revision)),
            ("pipelineName", json_value(&pipeline_name)),
            ("operatorIds", json_value(&operator_ids)),
            ("operatorLabels", json_value(&operator_labels)),
            ("stage", json_value("preview_section_binary")),
        ])),
    );

    let preview_sessions = preview_sessions.inner().clone();
    let request_for_compute = request;
    let compute_started = Instant::now();
    let result = tauri::async_runtime::spawn_blocking(move || {
        preview_sessions.preview_processing(request_for_compute)
    })
    .await
    .map_err(|error| error.to_string())?;
    let compute_duration_ms = compute_started.elapsed().as_millis();

    match result {
        Ok((response, reuse)) => {
            let traces = response.preview.section.traces;
            let samples = response.preview.section.samples;
            let packed = pack_preview_section_response(
                response.preview.preview_ready,
                response.preview.processing_label.clone(),
                response.preview.section,
            )?;
            diagnostics.complete(
                &app,
                &operation,
                "Processing preview ready (binary)",
                Some(build_fields([
                    ("pipelineRevision", json_value(pipeline_revision)),
                    ("pipelineName", json_value(&pipeline_name)),
                    ("operatorIds", json_value(&operator_ids)),
                    ("operatorLabels", json_value(&operator_labels)),
                    ("previewReady", json_value(true)),
                    ("traces", json_value(traces)),
                    ("samples", json_value(samples)),
                    ("computeDurationMs", json_value(compute_duration_ms)),
                    ("cacheHit", json_value(reuse.cache_hit)),
                    (
                        "reusedPrefixOperations",
                        json_value(reuse.reused_prefix_operations),
                    ),
                ])),
            );
            Ok(packed)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Processing preview failed (binary)",
                Some(build_fields([
                    ("pipelineRevision", json_value(pipeline_revision)),
                    ("pipelineName", json_value(&pipeline_name)),
                    ("operatorIds", json_value(&operator_ids)),
                    ("computeDurationMs", json_value(compute_duration_ms)),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn preview_subvolume_processing_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    request: PreviewSubvolumeProcessingRequest,
) -> Result<PreviewSubvolumeProcessingResponse, String> {
    let axis_name = format!("{:?}", request.section.axis).to_ascii_lowercase();
    let trace_local_count = request
        .pipeline
        .trace_local_pipeline
        .as_ref()
        .map(|pipeline| pipeline.operation_count())
        .unwrap_or(0);
    let source_handle = open_store(&request.store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&request.store_path)),
        ("datasetId", json_value(&request.section.dataset_id.0)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(request.section.index)),
        ("traceLocalOperatorCount", json_value(trace_local_count)),
        ("inlineMin", json_value(request.pipeline.crop.inline_min)),
        ("inlineMax", json_value(request.pipeline.crop.inline_max)),
        ("xlineMin", json_value(request.pipeline.crop.xline_min)),
        ("xlineMax", json_value(request.pipeline.crop.xline_max)),
        ("zMinMs", json_value(request.pipeline.crop.z_min_ms)),
        ("zMaxMs", json_value(request.pipeline.crop.z_max_ms)),
        ("stage", json_value("preview_subvolume")),
    ];
    if let Some(handle) = source_handle.as_ref() {
        append_subvolume_preview_debug_fields(&mut start_fields, handle, &request);
    }
    let preview_debug_fields = start_fields.clone();
    let operation = diagnostics.start_operation(
        &app,
        "preview_subvolume_processing",
        "Generating cropped processing preview",
        Some(build_fields(start_fields)),
    );

    let result = preview_subvolume_processing(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Subvolume processing preview ready",
                Some(build_fields([
                    ("previewReady", json_value(response.preview.preview_ready)),
                    ("traces", json_value(response.preview.section.traces)),
                    ("samples", json_value(response.preview.section.samples)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            let mut failure_fields = preview_debug_fields;
            failure_fields.push(("error", json_value(&message)));
            diagnostics.fail(
                &app,
                &operation,
                "Subvolume processing preview failed",
                Some(build_fields(failure_fields)),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn preview_subvolume_processing_binary_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    request: PreviewSubvolumeProcessingRequest,
) -> Result<Response, String> {
    let axis_name = format!("{:?}", request.section.axis).to_ascii_lowercase();
    let trace_local_count = request
        .pipeline
        .trace_local_pipeline
        .as_ref()
        .map(|pipeline| pipeline.operation_count())
        .unwrap_or(0);
    let source_handle = open_store(&request.store_path).ok();
    let mut start_fields = vec![
        ("storePath", json_value(&request.store_path)),
        ("datasetId", json_value(&request.section.dataset_id.0)),
        ("axis", json_value(&axis_name)),
        ("index", json_value(request.section.index)),
        ("traceLocalOperatorCount", json_value(trace_local_count)),
        ("inlineMin", json_value(request.pipeline.crop.inline_min)),
        ("inlineMax", json_value(request.pipeline.crop.inline_max)),
        ("xlineMin", json_value(request.pipeline.crop.xline_min)),
        ("xlineMax", json_value(request.pipeline.crop.xline_max)),
        ("zMinMs", json_value(request.pipeline.crop.z_min_ms)),
        ("zMaxMs", json_value(request.pipeline.crop.z_max_ms)),
        ("stage", json_value("preview_subvolume_binary")),
    ];
    if let Some(handle) = source_handle.as_ref() {
        append_subvolume_preview_debug_fields(&mut start_fields, handle, &request);
    }
    let preview_debug_fields = start_fields.clone();
    let operation = diagnostics.start_operation(
        &app,
        "preview_subvolume_processing_binary",
        "Generating cropped processing preview (binary)",
        Some(build_fields(start_fields)),
    );

    let result = preview_subvolume_processing(request);
    match result {
        Ok(response) => {
            let traces = response.preview.section.traces;
            let samples = response.preview.section.samples;
            let packed = pack_preview_section_response(
                response.preview.preview_ready,
                response.preview.processing_label.clone(),
                response.preview.section,
            )?;
            diagnostics.complete(
                &app,
                &operation,
                "Subvolume processing preview ready (binary)",
                Some(build_fields([
                    ("previewReady", json_value(true)),
                    ("traces", json_value(traces)),
                    ("samples", json_value(samples)),
                ])),
            );
            Ok(packed)
        }
        Err(error) => {
            let message = error.to_string();
            let mut failure_fields = preview_debug_fields;
            failure_fields.push(("error", json_value(&message)));
            diagnostics.fail(
                &app,
                &operation,
                "Subvolume processing preview failed (binary)",
                Some(build_fields(failure_fields)),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn preview_gather_processing_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: PreviewGatherProcessingRequest,
) -> Result<PreviewGatherProcessingResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "preview_gather_processing",
        "Generating gather processing preview",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            ("datasetId", json_value(&request.gather.dataset_id.0)),
            (
                "operatorCount",
                json_value(request.pipeline.operations.len()),
            ),
            (
                "traceLocalOperatorCount",
                json_value(
                    request
                        .pipeline
                        .trace_local_pipeline
                        .as_ref()
                        .map(|pipeline| pipeline.operation_count())
                        .unwrap_or(0),
                ),
            ),
            ("stage", json_value("preview_gather")),
        ])),
    );

    let result = preview_gather_processing(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Gather processing preview ready",
                Some(build_fields([
                    ("previewReady", json_value(response.preview.preview_ready)),
                    ("traces", json_value(response.preview.gather.traces)),
                    ("samples", json_value(response.preview.gather.samples)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Gather processing preview failed",
                Some(build_fields([("error", json_value(&message))])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn amplitude_spectrum_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: AmplitudeSpectrumRequest,
) -> Result<AmplitudeSpectrumResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "amplitude_spectrum",
        "Generating amplitude spectrum",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            (
                "axis",
                json_value(format!("{:?}", request.section.axis).to_ascii_lowercase()),
            ),
            ("index", json_value(request.section.index)),
            ("pipelineEnabled", json_value(request.pipeline.is_some())),
            ("stage", json_value("spectrum_analysis")),
        ])),
    );

    let result = amplitude_spectrum(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Amplitude spectrum ready",
                Some(build_fields([
                    ("bins", json_value(response.curve.frequencies_hz.len())),
                    ("sampleIntervalMs", json_value(response.sample_interval_ms)),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Amplitude spectrum failed",
                Some(build_fields([("error", json_value(&message))])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn velocity_scan_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: VelocityScanRequest,
) -> Result<VelocityScanResponse, String> {
    let operation = diagnostics.start_operation(
        &app,
        "velocity_scan",
        "Running prestack velocity scan",
        Some(build_fields([
            ("storePath", json_value(&request.store_path)),
            ("datasetId", json_value(&request.gather.dataset_id.0)),
            ("minVelocity", json_value(request.min_velocity_m_per_s)),
            ("maxVelocity", json_value(request.max_velocity_m_per_s)),
            ("velocityStep", json_value(request.velocity_step_m_per_s)),
            ("autopickEnabled", json_value(request.autopick.is_some())),
            ("stage", json_value("velocity_scan")),
        ])),
    );

    let result = run_velocity_scan(request);
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Velocity scan ready",
                Some(build_fields([
                    (
                        "velocityBins",
                        json_value(response.panel.velocities_m_per_s.len()),
                    ),
                    (
                        "sampleCount",
                        json_value(response.panel.sample_axis_ms.len()),
                    ),
                    (
                        "autopickCount",
                        json_value(
                            response
                                .autopicked_velocity_function
                                .as_ref()
                                .map(|estimate| estimate.times_ms.len())
                                .unwrap_or(0),
                        ),
                    ),
                ])),
            );
            Ok(response)
        }
        Err(error) => {
            let message = error.to_string();
            diagnostics.fail(
                &app,
                &operation,
                "Velocity scan failed",
                Some(build_fields([("error", json_value(&message))])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn run_processing_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    processing: State<ProcessingState>,
    processing_cache: State<ProcessingCacheState>,
    request: RunTraceLocalProcessingRequest,
) -> Result<RunTraceLocalProcessingResponse, String> {
    let pipeline_spec = seis_runtime::ProcessingPipelineSpec::TraceLocal {
        pipeline: request.pipeline.clone(),
    };
    let allow_exact_reuse = processing_cache.enabled()
        && request.output_store_path.is_none()
        && request.pipeline.checkpoint_indexes().is_empty();

    if allow_exact_reuse {
        let source_fingerprint = trace_local_source_fingerprint(&request.store_path)?;
        let full_pipeline_hash = trace_local_pipeline_hash(&request.pipeline)?;
        if let Some(hit) = processing_cache.lookup_exact_visible_output(
            TRACE_LOCAL_CACHE_FAMILY,
            &source_fingerprint,
            &full_pipeline_hash,
        )? {
            let final_artifact = ProcessingJobArtifact {
                kind: ProcessingJobArtifactKind::FinalOutput,
                step_index: request.pipeline.operation_count().saturating_sub(1),
                label: "Exact output reuse".to_string(),
                store_path: hit.path.clone(),
            };
            let reused = processing.enqueue_completed_job(
                request.store_path.clone(),
                hit.path.clone(),
                pipeline_spec.clone(),
                vec![final_artifact],
            );
            diagnostics.emit_session_event(
                &app,
                "processing_job_reused",
                log::Level::Info,
                "Processing job reused an existing derived output",
                Some(build_fields([
                    ("jobId", json_value(&reused.job_id)),
                    ("storePath", json_value(&request.store_path)),
                    ("outputStorePath", json_value(&hit.path)),
                ])),
            );
            return Ok(RunTraceLocalProcessingResponse {
                schema_version: IPC_SCHEMA_VERSION,
                job: reused,
            });
        }
    }

    let app_paths = AppPaths::resolve(&app)?;
    let output_store_path =
        request
            .output_store_path
            .clone()
            .unwrap_or(default_processing_store_path(
                &app_paths,
                &request.store_path,
                &request.pipeline,
            )?);
    let queued = processing.enqueue_job(
        request.store_path.clone(),
        Some(output_store_path.clone()),
        pipeline_spec,
    );
    let job_id = queued.job_id.clone();
    let record = processing.job_record(&job_id)?;

    diagnostics.emit_session_event(
        &app,
        "processing_job_queued",
        log::Level::Info,
        "Processing job queued",
        Some(build_fields([
            ("jobId", json_value(&job_id)),
            ("storePath", json_value(&request.store_path)),
            ("outputStorePath", json_value(&output_store_path)),
            (
                "operatorCount",
                json_value(request.pipeline.operation_count()),
            ),
        ])),
    );

    let worker_app = app.clone();
    let worker_request = RunTraceLocalProcessingRequest {
        output_store_path: Some(output_store_path.clone()),
        ..request
    };
    std::thread::spawn(move || {
        run_processing_job(&worker_app, &record, worker_request);
    });

    Ok(RunTraceLocalProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        job: queued,
    })
}

#[tauri::command]
fn run_subvolume_processing_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    processing: State<ProcessingState>,
    request: RunSubvolumeProcessingRequest,
) -> Result<RunSubvolumeProcessingResponse, String> {
    let app_paths = AppPaths::resolve(&app)?;
    let output_store_path =
        request
            .output_store_path
            .clone()
            .unwrap_or(default_subvolume_processing_store_path(
                &app_paths,
                &request.store_path,
                &request.pipeline,
            )?);
    let queued = processing.enqueue_job(
        request.store_path.clone(),
        Some(output_store_path.clone()),
        seis_runtime::ProcessingPipelineSpec::Subvolume {
            pipeline: request.pipeline.clone(),
        },
    );
    let job_id = queued.job_id.clone();
    let record = processing.job_record(&job_id)?;

    diagnostics.emit_session_event(
        &app,
        "subvolume_processing_job_queued",
        log::Level::Info,
        "Subvolume processing job queued",
        Some(build_fields([
            ("jobId", json_value(&job_id)),
            ("storePath", json_value(&request.store_path)),
            ("outputStorePath", json_value(&output_store_path)),
            (
                "traceLocalOperatorCount",
                json_value(
                    request
                        .pipeline
                        .trace_local_pipeline
                        .as_ref()
                        .map(|pipeline| pipeline.operation_count())
                        .unwrap_or(0),
                ),
            ),
        ])),
    );

    let worker_app = app.clone();
    let worker_request = RunSubvolumeProcessingRequest {
        output_store_path: Some(output_store_path.clone()),
        ..request
    };
    std::thread::spawn(move || {
        run_subvolume_processing_job(&worker_app, &record, worker_request);
    });

    Ok(RunSubvolumeProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        job: queued,
    })
}

#[tauri::command]
fn run_gather_processing_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    processing: State<ProcessingState>,
    request: RunGatherProcessingRequest,
) -> Result<RunGatherProcessingResponse, String> {
    let app_paths = AppPaths::resolve(&app)?;
    let output_store_path =
        request
            .output_store_path
            .clone()
            .unwrap_or(default_gather_processing_store_path(
                &app_paths,
                &request.store_path,
                &request.pipeline,
            )?);
    let queued = processing.enqueue_job(
        request.store_path.clone(),
        Some(output_store_path.clone()),
        seis_runtime::ProcessingPipelineSpec::Gather {
            pipeline: request.pipeline.clone(),
        },
    );
    let job_id = queued.job_id.clone();
    let record = processing.job_record(&job_id)?;

    diagnostics.emit_session_event(
        &app,
        "gather_processing_job_queued",
        log::Level::Info,
        "Gather processing job queued",
        Some(build_fields([
            ("jobId", json_value(&job_id)),
            ("storePath", json_value(&request.store_path)),
            ("outputStorePath", json_value(&output_store_path)),
            (
                "operatorCount",
                json_value(request.pipeline.operations.len()),
            ),
        ])),
    );

    let worker_app = app.clone();
    let worker_request = RunGatherProcessingRequest {
        output_store_path: Some(output_store_path.clone()),
        ..request
    };
    std::thread::spawn(move || {
        run_gather_processing_job(&worker_app, &record, worker_request);
    });

    Ok(RunGatherProcessingResponse {
        schema_version: IPC_SCHEMA_VERSION,
        job: queued,
    })
}

#[tauri::command]
fn get_processing_job_command(
    processing: State<ProcessingState>,
    request: GetProcessingJobRequest,
) -> Result<GetProcessingJobResponse, String> {
    Ok(GetProcessingJobResponse {
        schema_version: IPC_SCHEMA_VERSION,
        job: processing.job_status(&request.job_id)?,
    })
}

#[tauri::command]
fn cancel_processing_job_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    processing: State<ProcessingState>,
    request: CancelProcessingJobRequest,
) -> Result<CancelProcessingJobResponse, String> {
    let job = processing.cancel_job(&request.job_id)?;
    diagnostics.emit_session_event(
        &app,
        "processing_job_cancel_requested",
        log::Level::Warn,
        "Processing job cancellation requested",
        Some(build_fields([("jobId", json_value(&request.job_id))])),
    );
    Ok(CancelProcessingJobResponse {
        schema_version: IPC_SCHEMA_VERSION,
        job,
    })
}

#[tauri::command]
fn list_pipeline_presets_command(
    processing: State<ProcessingState>,
) -> Result<ListPipelinePresetsResponse, String> {
    Ok(ListPipelinePresetsResponse {
        schema_version: IPC_SCHEMA_VERSION,
        presets: processing.list_presets()?,
    })
}

#[tauri::command]
fn save_pipeline_preset_command(
    processing: State<ProcessingState>,
    request: SavePipelinePresetRequest,
) -> Result<SavePipelinePresetResponse, String> {
    Ok(SavePipelinePresetResponse {
        schema_version: IPC_SCHEMA_VERSION,
        preset: processing.save_preset(request.preset)?,
    })
}

#[tauri::command]
fn delete_pipeline_preset_command(
    processing: State<ProcessingState>,
    request: DeletePipelinePresetRequest,
) -> Result<DeletePipelinePresetResponse, String> {
    Ok(DeletePipelinePresetResponse {
        schema_version: IPC_SCHEMA_VERSION,
        deleted: processing.delete_preset(&request.preset_id)?,
    })
}

#[tauri::command]
fn load_workspace_state_command(
    workspace: State<WorkspaceState>,
) -> Result<LoadWorkspaceStateResponse, String> {
    workspace.load_state()
}

#[tauri::command]
fn upsert_dataset_entry_command(
    workspace: State<WorkspaceState>,
    request: UpsertDatasetEntryRequest,
) -> Result<UpsertDatasetEntryResponse, String> {
    workspace.upsert_entry(request)
}

#[tauri::command]
fn remove_dataset_entry_command(
    workspace: State<WorkspaceState>,
    request: RemoveDatasetEntryRequest,
) -> Result<RemoveDatasetEntryResponse, String> {
    workspace.remove_entry(request)
}

#[tauri::command]
fn set_active_dataset_entry_command(
    workspace: State<WorkspaceState>,
    request: SetActiveDatasetEntryRequest,
) -> Result<SetActiveDatasetEntryResponse, String> {
    workspace.set_active_entry(request)
}

#[tauri::command]
fn save_workspace_session_command(
    workspace: State<WorkspaceState>,
    request: SaveWorkspaceSessionRequest,
) -> Result<SaveWorkspaceSessionResponse, String> {
    workspace.save_session(request)
}

#[tauri::command]
fn load_project_geospatial_settings_command(
    request: ProjectRootRequest,
) -> Result<LoadProjectGeospatialSettingsResponse, String> {
    let settings = load_project_geospatial_settings(Path::new(&request.project_root))?;
    Ok(LoadProjectGeospatialSettingsResponse { settings })
}

#[tauri::command]
fn save_project_geospatial_settings_command(
    request: SaveProjectGeospatialSettingsRequest,
) -> Result<ProjectGeospatialSettings, String> {
    save_project_geospatial_settings(
        Path::new(&request.project_root),
        request.display_coordinate_reference,
        request.source.as_deref().unwrap_or("user_selected"),
    )
}

#[tauri::command]
fn search_coordinate_references_command(
    request: SearchCoordinateReferencesRequest,
) -> Result<SearchCoordinateReferencesResponse, String> {
    search_coordinate_references(request)
}

#[tauri::command]
fn resolve_coordinate_reference_command(
    request: ResolveCoordinateReferenceRequest,
) -> Result<CoordinateReferenceCatalogEntry, String> {
    resolve_coordinate_reference(request)
}

#[tauri::command]
fn set_dataset_native_coordinate_reference_command(
    request: SetDatasetNativeCoordinateReferenceSelectionRequest,
) -> Result<SetDatasetNativeCoordinateReferenceResponse, String> {
    let normalized_id = request
        .coordinate_reference_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let normalized_name = request
        .coordinate_reference_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let resolved = if let Some(auth_id) = normalized_id {
        let entry = resolve_coordinate_reference(ResolveCoordinateReferenceRequest {
            authority: None,
            code: None,
            auth_id: Some(auth_id.to_string()),
        })?;
        (Some(entry.auth_id), Some(entry.name))
    } else if let Some(label) = normalized_name {
        (None, Some(label.to_string()))
    } else {
        (None, None)
    };
    set_any_store_native_coordinate_reference(
        &request.store_path,
        resolved.0.as_deref(),
        resolved.1.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    let response = open_dataset_summary(OpenDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: request.store_path,
    })
    .map_err(|error| error.to_string())?;
    Ok(SetDatasetNativeCoordinateReferenceResponse {
        schema_version: IPC_SCHEMA_VERSION,
        dataset: response.dataset,
    })
}

#[tauri::command]
fn resolve_survey_map_command(
    app: AppHandle,
    request: ResolveSurveyMapRequest,
) -> Result<ResolveSurveyMapResponse, String> {
    let app_paths = AppPaths::resolve(&app)?;
    let store_path = request.store_path.clone();
    let dataset = open_dataset_summary(OpenDatasetRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path,
    })
    .map_err(|error| error.to_string())?
    .dataset;
    let survey_map = resolve_dataset_summary_survey_map_source(
        &dataset,
        request.display_coordinate_reference_id.as_deref(),
        Some(app_paths.map_transform_cache_dir()),
        Some(Path::new(&request.store_path)),
    )
    .map_err(|error| error.to_string())?;
    Ok(ResolveSurveyMapResponse {
        schema_version: IPC_SCHEMA_VERSION,
        survey_map,
    })
}

#[tauri::command]
fn resolve_project_survey_map_command(
    request: ResolveProjectSurveyMapRequest,
) -> Result<ResolveProjectSurveyMapResponse, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let survey_map = project
        .resolve_survey_map_source(&ProjectSurveyMapRequestDto {
            schema_version: 1,
            survey_asset_ids: vec![request.survey_asset_id],
            wellbore_ids: request.wellbore_id.into_iter().collect(),
            display_coordinate_reference_id: request.display_coordinate_reference_id,
        })
        .map_err(|error| error.to_string())?;
    Ok(ResolveProjectSurveyMapResponse { survey_map })
}

#[tauri::command]
fn list_project_well_time_depth_models_command(
    request: ProjectWellboreRequest,
) -> Result<Vec<ProjectWellTimeDepthModelDescriptor>, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let active_asset_id =
        project_active_well_time_depth_model_asset_id(&project, &request.wellbore_id)?;
    project_well_time_depth_model_descriptors(
        &project,
        &request.wellbore_id,
        active_asset_id.as_deref(),
    )
}

#[tauri::command]
fn list_project_well_time_depth_inventory_command(
    request: ProjectWellboreRequest,
) -> Result<ProjectWellTimeDepthInventoryResponse, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let active_asset_id =
        project_active_well_time_depth_model_asset_id(&project, &request.wellbore_id)?;
    Ok(ProjectWellTimeDepthInventoryResponse {
        observation_sets: project_well_time_depth_observation_descriptors(
            &project,
            &request.wellbore_id,
        )?,
        authored_models: project_well_time_depth_authored_model_descriptors(
            &project,
            &request.wellbore_id,
        )?,
        compiled_models: project_well_time_depth_model_descriptors(
            &project,
            &request.wellbore_id,
            active_asset_id.as_deref(),
        )?,
    })
}

#[tauri::command]
fn list_project_well_overlay_inventory_command(
    request: ProjectWellOverlayInventoryRequest,
) -> Result<ProjectWellOverlayInventoryResponse, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let display_coordinate_reference_id =
        normalized_optional_string(request.display_coordinate_reference_id.as_deref());
    let inventory = project
        .project_well_overlay_inventory()
        .map_err(|error| error.to_string())?;
    let surveys = inventory
        .surveys
        .into_iter()
        .map(|survey| {
            let display_compatibility = project_survey_display_compatibility(
                survey.effective_coordinate_reference_id.as_deref(),
                display_coordinate_reference_id.as_deref(),
            );
            ProjectSurveyAssetDescriptor {
                asset_id: survey.asset_id.0,
                name: survey.name,
                status: asset_status_label(&survey.status).to_string(),
                well_id: survey.well_id.0,
                well_name: survey.well_name,
                wellbore_id: survey.wellbore_id.0,
                wellbore_name: survey.wellbore_name,
                effective_coordinate_reference_id: survey.effective_coordinate_reference_id,
                effective_coordinate_reference_name: survey.effective_coordinate_reference_name,
                display_compatibility,
            }
        })
        .collect::<Vec<_>>();
    let wellbores = inventory
        .wellbores
        .into_iter()
        .map(|wellbore| {
            let display_compatibility = project_wellbore_display_compatibility(
                &project,
                &wellbore.wellbore_id.0,
                display_coordinate_reference_id.as_deref(),
            );
            ProjectWellboreInventoryItem {
                well_id: wellbore.well_id.0,
                well_name: wellbore.well_name,
                wellbore_id: wellbore.wellbore_id.0,
                wellbore_name: wellbore.wellbore_name,
                trajectory_asset_count: wellbore.trajectory_asset_count,
                well_time_depth_model_count: wellbore.well_time_depth_model_count,
                active_well_time_depth_model_asset_id: wellbore
                    .active_well_time_depth_model_asset_id
                    .map(|asset_id| asset_id.0),
                display_compatibility,
            }
        })
        .collect::<Vec<_>>();
    let compatible_survey_count = surveys
        .iter()
        .filter(|survey| survey.display_compatibility.can_resolve_project_map)
        .count();
    let incompatible_survey_count = surveys.len().saturating_sub(compatible_survey_count);
    let compatible_wellbore_count = wellbores
        .iter()
        .filter(|wellbore| wellbore.display_compatibility.can_resolve_project_map)
        .count();
    let incompatible_wellbore_count = wellbores.len().saturating_sub(compatible_wellbore_count);
    let blocking_reason_codes = surveys
        .iter()
        .filter(|survey| !survey.display_compatibility.can_resolve_project_map)
        .filter_map(|survey| survey.display_compatibility.reason_code)
        .filter_map(project_survey_blocking_reason_code)
        .chain(
            wellbores
                .iter()
                .filter(|wellbore| !wellbore.display_compatibility.can_resolve_project_map)
                .filter_map(|wellbore| wellbore.display_compatibility.reason_code)
                .filter_map(project_wellbore_blocking_reason_code),
        )
        .fold(
            Vec::<ProjectDisplayCompatibilityBlockingReasonCode>::new(),
            |mut codes, code| {
                if !codes.contains(&code) {
                    codes.push(code);
                }
                codes
            },
        );
    let blocking_reasons = surveys
        .iter()
        .filter(|survey| !survey.display_compatibility.can_resolve_project_map)
        .filter_map(|survey| survey.display_compatibility.reason.clone())
        .chain(
            wellbores
                .iter()
                .filter(|wellbore| !wellbore.display_compatibility.can_resolve_project_map)
                .filter_map(|wellbore| wellbore.display_compatibility.reason.clone()),
        )
        .fold(Vec::<String>::new(), |mut reasons, reason| {
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
            reasons
        });
    Ok(ProjectWellOverlayInventoryResponse {
        surveys,
        wellbores,
        display_compatibility: ProjectMapDisplayCompatibilitySummary {
            display_coordinate_reference_id,
            compatible_survey_count,
            incompatible_survey_count,
            compatible_wellbore_count,
            incompatible_wellbore_count,
            blocking_reason_codes,
            blocking_reasons,
        },
    })
}

#[tauri::command]
fn list_project_survey_horizons_command(
    request: ProjectAssetRequest,
) -> Result<Vec<ImportedHorizonDescriptor>, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let asset = project
        .asset_by_id(&ophiolite::AssetId(request.asset_id))
        .map_err(|error| error.to_string())?;
    if asset.asset_kind != AssetKind::SeismicTraceData {
        return Err(format!(
            "asset '{}' is not a seismic survey asset",
            asset.id.0
        ));
    }
    let store_path = Path::new(&asset.package_path).join("store");
    load_horizon_assets(store_path.to_string_lossy().to_string()).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_project_well_marker_residual_inventory_command(
    request: ProjectWellboreRequest,
) -> Result<ProjectWellMarkerResidualInventoryResponse, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    Ok(ProjectWellMarkerResidualInventoryResponse {
        markers: project_well_marker_descriptors(&project, &request.wellbore_id)?,
        residual_assets: project_well_marker_horizon_residual_descriptors(
            &project,
            &request.wellbore_id,
        )?,
    })
}

#[tauri::command]
fn scan_vendor_project_command(
    request: ophiolite::VendorProjectScanRequest,
) -> Result<ophiolite::VendorProjectScanResponse, String> {
    ophiolite::scan_vendor_project(&request).map_err(|error| error.to_string())
}

#[tauri::command]
fn plan_vendor_project_import_command(
    request: ophiolite::VendorProjectPlanRequest,
) -> Result<ophiolite::VendorProjectPlanResponse, String> {
    ophiolite::plan_vendor_project_import(&request).map_err(|error| error.to_string())
}

#[tauri::command]
fn commit_vendor_project_import_command(
    request: ophiolite::VendorProjectCommitRequest,
) -> Result<ophiolite::VendorProjectCommitResponse, String> {
    ophiolite::commit_vendor_project_import(&request).map_err(|error| error.to_string())
}

#[tauri::command]
fn compute_project_well_marker_residual_command(
    request: ComputeProjectWellMarkerResidualRequest,
) -> Result<ComputeProjectWellMarkerResidualResponse, String> {
    let mut project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let wellbore_id = ophiolite::WellboreId(request.wellbore_id.clone());
    let source_asset_id = project
        .definitive_marker_source_asset_id(&wellbore_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "wellbore '{}' does not have a definitive marker-set or top-set source",
                request.wellbore_id
            )
        })?;
    let marker_name = request.marker_name.trim();
    if marker_name.is_empty() {
        return Err("marker_name is required".to_string());
    }
    let result = project
        .run_compute(&ProjectComputeRunRequest {
            source_asset_id,
            function_id: "well_markers:depth_horizon_residuals".to_string(),
            curve_bindings: std::collections::BTreeMap::new(),
            parameters: std::collections::BTreeMap::from([
                (
                    "survey_asset_id".to_string(),
                    ComputeParameterValue::String(request.survey_asset_id.clone()),
                ),
                (
                    "horizon_id".to_string(),
                    ComputeParameterValue::String(request.horizon_id.clone()),
                ),
                (
                    "marker_name".to_string(),
                    ComputeParameterValue::String(marker_name.to_string()),
                ),
            ]),
            output_collection_name: request.output_collection_name.clone().or_else(|| {
                Some(format!(
                    "{} | {} Residual",
                    request.horizon_id.trim(),
                    marker_name
                ))
            }),
            output_mnemonic: None,
        })
        .map_err(|error| error.to_string())?;
    let points = project
        .read_well_marker_horizon_residual_points(&result.asset.id)
        .map_err(|error| error.to_string())?;
    Ok(ComputeProjectWellMarkerResidualResponse {
        asset_id: result.asset.id.0,
        collection_id: result.collection.id.0,
        collection_name: result.collection.name,
        well_id: result.asset.well_id.0,
        wellbore_id: result.asset.wellbore_id.0,
        marker_name: marker_name.to_string(),
        horizon_id: request.horizon_id,
        point_count: points.len(),
    })
}

#[tauri::command]
fn set_project_active_well_time_depth_model_command(
    request: SetProjectWellTimeDepthModelRequest,
) -> Result<(), String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    project
        .set_active_well_time_depth_model(
            &ophiolite::WellboreId(request.wellbore_id),
            request
                .asset_id
                .as_ref()
                .map(|value| ophiolite::AssetId(value.clone()))
                .as_ref(),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn import_project_well_time_depth_model_command(
    request: ImportProjectWellTimeDepthModelRequest,
) -> Result<ImportProjectWellTimeDepthModelResponse, String> {
    let mut project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let result = project
        .import_well_time_depth_model_json(
            Path::new(&request.json_path),
            request.binding,
            request.collection_name.as_deref(),
        )
        .map_err(|error| error.to_string())?;

    Ok(well_time_depth_import_response(result))
}

#[tauri::command]
fn preview_project_well_time_depth_asset_command(
    request: PreviewProjectWellTimeDepthAssetRequest,
) -> Result<ophiolite::ProjectWellTimeDepthAssetPreview, String> {
    let asset_kind = match request.asset_kind.as_str() {
        "checkshot_vsp_observation_set" => ophiolite::AssetKind::CheckshotVspObservationSet,
        "manual_time_depth_pick_set" => ophiolite::AssetKind::ManualTimeDepthPickSet,
        "well_tie_observation_set" => ophiolite::AssetKind::WellTieObservationSet,
        "well_time_depth_authored_model" => ophiolite::AssetKind::WellTimeDepthAuthoredModel,
        "well_time_depth_model" => ophiolite::AssetKind::WellTimeDepthModel,
        other => return Err(format!("unsupported well time-depth asset kind '{other}'")),
    };
    if let Some(json_payload) = request.json_payload.as_deref() {
        ophiolite::preview_well_time_depth_json_payload(
            Path::new(&request.json_path),
            json_payload,
            asset_kind,
        )
        .map_err(|error| error.to_string())
    } else {
        ophiolite::preview_well_time_depth_json_asset(Path::new(&request.json_path), asset_kind)
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn preview_project_well_time_depth_import_command(
    request: PreviewProjectWellTimeDepthImportRequest,
) -> Result<ophiolite::ProjectWellTimeDepthImportPreview, String> {
    let asset_kind = match request.asset_kind.as_str() {
        "checkshot_vsp_observation_set" => ophiolite::AssetKind::CheckshotVspObservationSet,
        "manual_time_depth_pick_set" => ophiolite::AssetKind::ManualTimeDepthPickSet,
        "well_tie_observation_set" => ophiolite::AssetKind::WellTieObservationSet,
        "well_time_depth_authored_model" => ophiolite::AssetKind::WellTimeDepthAuthoredModel,
        "well_time_depth_model" => ophiolite::AssetKind::WellTimeDepthModel,
        other => return Err(format!("unsupported well time-depth asset kind '{other}'")),
    };
    let draft = request.draft.as_ref();
    ophiolite::preview_well_time_depth_import_draft(
        Path::new(&request.json_path),
        draft.map(|value| value.json_payload.as_str()),
        asset_kind,
        draft.and_then(|value| value.collection_name.as_deref()),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_project_well_sources_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    request: PreviewProjectWellSourceImportRequest,
) -> Result<ophiolite::ProjectWellSourceImportPreview, String> {
    let selected_source_path_count = request
        .source_paths
        .as_ref()
        .map(|paths| paths.len())
        .unwrap_or(0);
    let operation = diagnostics.start_operation(
        &app,
        "preview_project_well_sources",
        "Previewing well source import",
        Some(build_fields([
            ("sourceRootPath", json_value(&request.source_root_path)),
            (
                "selectedSourcePathCount",
                json_value(selected_source_path_count),
            ),
            (
                "selectionMode",
                json_value(if selected_source_path_count > 0 {
                    "selected_sources"
                } else {
                    "source_root_scan"
                }),
            ),
            ("stage", json_value("preview_well_sources")),
        ])),
    );

    let result = if let Some(source_paths) = request.source_paths.filter(|paths| !paths.is_empty())
    {
        let normalized_paths = source_paths
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        ophiolite::preview_well_source_import_sources(
            &normalized_paths,
            Some(Path::new(&request.source_root_path)),
        )
        .map_err(|error| error.to_string())
    } else {
        ophiolite::preview_well_source_import(Path::new(&request.source_root_path))
            .map_err(|error| error.to_string())
    };

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Well source import preview ready",
                Some(build_fields([
                    ("stage", json_value("preview_well_sources")),
                    ("folderName", json_value(&response.parsed.folder_name)),
                    ("issueCount", json_value(response.parsed.issues.len())),
                    (
                        "blockingIssueCount",
                        json_value(
                            response
                                .parsed
                                .issues
                                .iter()
                                .filter(|issue| {
                                    issue.severity
                                        == ophiolite::WellFolderImportIssueSeverity::Blocking
                                })
                                .count(),
                        ),
                    ),
                    ("logFileCount", json_value(response.parsed.logs.files.len())),
                    (
                        "asciiLogFileCount",
                        json_value(response.parsed.ascii_logs.files.len()),
                    ),
                    (
                        "topsRowCount",
                        json_value(response.parsed.tops_markers.row_count),
                    ),
                    (
                        "trajectoryRowCount",
                        json_value(response.parsed.trajectory.row_count),
                    ),
                    (
                        "unsupportedSourceCount",
                        json_value(response.parsed.unsupported_sources.len()),
                    ),
                    (
                        "sourceCrsCandidateCount",
                        json_value(response.parsed.source_coordinate_reference.candidates.len()),
                    ),
                ])),
            );
            Ok(response)
        }
        Err(message) => {
            diagnostics.fail(
                &app,
                &operation,
                "Well source import preview failed",
                Some(build_fields([
                    ("stage", json_value("preview_well_sources")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn preview_project_well_import_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    request: PreviewProjectWellImportRequest,
) -> Result<ophiolite::ProjectWellFolderImportPreview, String> {
    let selected_source_path_count = request
        .source_paths
        .as_ref()
        .map(|paths| paths.len())
        .unwrap_or(0);
    let operation = diagnostics.start_operation(
        &app,
        "preview_project_well_import",
        "Previewing well import",
        Some(build_fields([
            ("sourceRootPath", json_value(&request.folder_path)),
            (
                "selectedSourcePathCount",
                json_value(selected_source_path_count),
            ),
            (
                "selectionMode",
                json_value(if selected_source_path_count > 0 {
                    "selected_sources"
                } else {
                    "source_root_scan"
                }),
            ),
            ("stage", json_value("preview_well_sources")),
        ])),
    );

    let result = if let Some(source_paths) = request.source_paths.filter(|paths| !paths.is_empty())
    {
        let normalized_paths = source_paths
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        ophiolite::preview_well_import_sources(
            &normalized_paths,
            Some(Path::new(&request.folder_path)),
        )
        .map_err(|error| error.to_string())
    } else {
        ophiolite::preview_well_folder_import(Path::new(&request.folder_path))
            .map_err(|error| error.to_string())
    };

    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Well import preview ready",
                Some(build_fields([
                    ("stage", json_value("preview_well_sources")),
                    ("folderName", json_value(&response.folder_name)),
                    ("issueCount", json_value(response.issues.len())),
                    (
                        "blockingIssueCount",
                        json_value(
                            response
                                .issues
                                .iter()
                                .filter(|issue| {
                                    issue.severity
                                        == ophiolite::WellFolderImportIssueSeverity::Blocking
                                })
                                .count(),
                        ),
                    ),
                    ("logFileCount", json_value(response.logs.files.len())),
                    (
                        "asciiLogFileCount",
                        json_value(response.ascii_logs.files.len()),
                    ),
                    ("topsRowCount", json_value(response.tops_markers.row_count)),
                    (
                        "trajectoryRowCount",
                        json_value(response.trajectory.row_count),
                    ),
                    (
                        "unsupportedSourceCount",
                        json_value(response.unsupported_sources.len()),
                    ),
                    (
                        "sourceCrsCandidateCount",
                        json_value(response.source_coordinate_reference.candidates.len()),
                    ),
                ])),
            );
            Ok(response)
        }
        Err(message) => {
            diagnostics.fail(
                &app,
                &operation,
                "Well import preview failed",
                Some(build_fields([
                    ("stage", json_value("preview_well_sources")),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

fn resolve_project_well_source_commit_request(
    request: CommitProjectWellSourceImportRequest,
) -> Result<(String, ophiolite::ProjectWellSourceImportCommitRequest), String> {
    let commit_request = if let Some(draft) = request.draft {
        let selected_log_source_paths = draft.import_plan.selected_log_source_paths;
        let ascii_log_imports = draft.import_plan.ascii_log_imports;
        let tops_markers = draft.import_plan.tops_markers;
        let trajectory = draft.import_plan.trajectory;
        let import_trajectory = trajectory
            .as_ref()
            .map(|value| value.enabled)
            .unwrap_or(false);
        let trajectory_rows = trajectory.as_ref().and_then(|value| value.rows.clone());
        ophiolite::ProjectWellSourceImportCommitRequest {
            folder_path: request.source_root_path,
            source_paths: request.source_paths,
            binding: draft.binding,
            well_metadata: draft.well_metadata,
            wellbore_metadata: draft.wellbore_metadata,
            source_coordinate_reference: draft.source_coordinate_reference,
            import_logs: selected_log_source_paths
                .as_ref()
                .map(|paths| !paths.is_empty())
                .unwrap_or(false)
                || ascii_log_imports
                    .as_ref()
                    .map(|imports| !imports.is_empty())
                    .unwrap_or(false),
            selected_log_source_paths,
            import_tops_markers: tops_markers.is_some(),
            import_trajectory,
            tops_depth_reference: tops_markers
                .as_ref()
                .and_then(|value| value.depth_reference.clone()),
            tops_rows: tops_markers.map(|value| value.rows),
            trajectory_rows,
            ascii_log_imports,
        }
    } else {
        ophiolite::ProjectWellSourceImportCommitRequest {
            folder_path: request.source_root_path,
            source_paths: request.source_paths,
            binding: request.binding.ok_or_else(|| {
                "well-source import requires either a draft or binding data".to_string()
            })?,
            well_metadata: request.well_metadata,
            wellbore_metadata: request.wellbore_metadata,
            source_coordinate_reference: request.source_coordinate_reference.ok_or_else(|| {
                "well-source import requires either a draft or source CRS selection data"
                    .to_string()
            })?,
            import_logs: request.import_logs.unwrap_or(false),
            selected_log_source_paths: request.selected_log_source_paths,
            import_tops_markers: request.import_tops_markers.unwrap_or(false),
            import_trajectory: request.import_trajectory.unwrap_or(false),
            tops_depth_reference: request.tops_depth_reference,
            tops_rows: request.tops_rows,
            trajectory_rows: request.trajectory_rows,
            ascii_log_imports: request.ascii_log_imports,
        }
    };
    Ok((request.project_root, commit_request))
}

#[tauri::command]
fn commit_project_well_sources_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    request: CommitProjectWellSourceImportRequest,
) -> Result<ophiolite::ProjectWellSourceImportCommitResponse, String> {
    let source_root_path = request.source_root_path.clone();
    let selected_source_path_count = request
        .source_paths
        .as_ref()
        .map(|paths| paths.len())
        .unwrap_or(0);
    let (project_root, commit_request) = resolve_project_well_source_commit_request(request)?;
    let operation = diagnostics.start_operation(
        &app,
        "commit_project_well_sources",
        "Importing well sources into project storage",
        Some(build_fields([
            ("projectRoot", json_value(&project_root)),
            ("sourceRootPath", json_value(&source_root_path)),
            (
                "selectedSourcePathCount",
                json_value(selected_source_path_count),
            ),
            ("importLogs", json_value(commit_request.import_logs)),
            (
                "selectedLogSourcePathCount",
                json_value(
                    commit_request
                        .selected_log_source_paths
                        .as_ref()
                        .map(|paths| paths.len())
                        .unwrap_or(0),
                ),
            ),
            (
                "asciiLogImportCount",
                json_value(
                    commit_request
                        .ascii_log_imports
                        .as_ref()
                        .map(|imports| imports.len())
                        .unwrap_or(0),
                ),
            ),
            (
                "topsRowCount",
                json_value(
                    commit_request
                        .tops_rows
                        .as_ref()
                        .map(|rows| rows.len())
                        .unwrap_or(0),
                ),
            ),
            (
                "importTrajectory",
                json_value(commit_request.import_trajectory),
            ),
            (
                "trajectoryRowCount",
                json_value(
                    commit_request
                        .trajectory_rows
                        .as_ref()
                        .map(|rows| rows.len())
                        .unwrap_or(0),
                ),
            ),
            (
                "sourceCrsMode",
                json_value(
                    format!("{:?}", commit_request.source_coordinate_reference.mode)
                        .to_ascii_lowercase(),
                ),
            ),
            ("stage", json_value("commit_well_sources")),
        ])),
    );
    let mut project =
        OphioliteProject::open(Path::new(&project_root)).map_err(|error| error.to_string())?;
    let result = ophiolite::commit_well_source_import(&mut project, &commit_request)
        .map_err(|error| error.to_string());
    match result {
        Ok(response) => {
            diagnostics.complete(
                &app,
                &operation,
                "Well source import committed",
                Some(build_fields([
                    ("stage", json_value("commit_well_sources")),
                    ("wellId", json_value(&response.well_id)),
                    ("wellboreId", json_value(&response.wellbore_id)),
                    ("createdWell", json_value(response.created_well)),
                    ("createdWellbore", json_value(response.created_wellbore)),
                    (
                        "importedAssetCount",
                        json_value(response.imported_assets.len()),
                    ),
                    ("omissionCount", json_value(response.omissions.len())),
                    ("issueCount", json_value(response.issues.len())),
                ])),
            );
            Ok(response)
        }
        Err(message) => {
            diagnostics.fail(
                &app,
                &operation,
                "Well source import failed",
                Some(build_fields([
                    ("stage", json_value("commit_well_sources")),
                    ("projectRoot", json_value(&project_root)),
                    ("error", json_value(&message)),
                ])),
            );
            Err(message)
        }
    }
}

#[tauri::command]
fn commit_project_well_import_command(
    app: AppHandle,
    diagnostics: State<'_, DiagnosticsState>,
    request: CommitProjectWellImportRequest,
) -> Result<ophiolite::ProjectWellFolderImportCommitResponse, String> {
    commit_project_well_sources_command(
        app,
        diagnostics,
        CommitProjectWellSourceImportRequest {
            project_root: request.project_root,
            source_root_path: request.folder_path,
            source_paths: request.source_paths,
            draft: request.draft,
            binding: Some(request.binding),
            well_metadata: request.well_metadata,
            wellbore_metadata: request.wellbore_metadata,
            source_coordinate_reference: Some(request.source_coordinate_reference),
            import_logs: Some(request.import_logs),
            selected_log_source_paths: request.selected_log_source_paths,
            import_tops_markers: Some(request.import_tops_markers),
            import_trajectory: Some(request.import_trajectory),
            tops_depth_reference: request.tops_depth_reference,
            tops_rows: request.tops_rows,
            trajectory_rows: request.trajectory_rows,
            ascii_log_imports: request.ascii_log_imports,
        },
    )
}

#[tauri::command]
fn import_project_well_time_depth_asset_command(
    request: ImportProjectWellTimeDepthAssetRequest,
) -> Result<ImportProjectWellTimeDepthModelResponse, String> {
    let mut project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let source_path = Path::new(&request.json_path);
    let result = match (request.asset_kind.as_str(), request.json_payload.as_deref()) {
        ("checkshot_vsp_observation_set", Some(json_payload)) => {
            let observation_set: CheckshotVspObservationSet1D = serde_json::from_str(json_payload)
                .map_err(|error| {
                    format!(
                        "failed to parse checkshot/VSP observation json '{}': {error}",
                        request.json_path
                    )
                })?;
            project.create_checkshot_vsp_observation_set(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
                &observation_set,
            )
        }
        ("checkshot_vsp_observation_set", None) => project
            .import_checkshot_vsp_observation_set_json(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
            ),
        ("manual_time_depth_pick_set", Some(json_payload)) => {
            let pick_set: ManualTimeDepthPickSet1D =
                serde_json::from_str(json_payload).map_err(|error| {
                    format!(
                        "failed to parse manual time-depth pick json '{}': {error}",
                        request.json_path
                    )
                })?;
            project.create_manual_time_depth_pick_set(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
                &pick_set,
            )
        }
        ("manual_time_depth_pick_set", None) => project.import_manual_time_depth_pick_set_json(
            source_path,
            request.binding,
            request.collection_name.as_deref(),
        ),
        ("well_tie_observation_set", Some(json_payload)) => {
            let observation_set: WellTieObservationSet1D = serde_json::from_str(json_payload)
                .map_err(|error| {
                    format!(
                        "failed to parse well tie observation json '{}': {error}",
                        request.json_path
                    )
                })?;
            project.create_well_tie_observation_set(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
                &observation_set,
            )
        }
        ("well_tie_observation_set", None) => {
            let observation_set: WellTieObservationSet1D =
                serde_json::from_slice(&fs::read(source_path).map_err(|error| error.to_string())?)
                    .map_err(|error| {
                        format!(
                            "failed to parse well tie observation json '{}': {error}",
                            request.json_path
                        )
                    })?;
            project.create_well_tie_observation_set(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
                &observation_set,
            )
        }
        ("well_time_depth_authored_model", Some(json_payload)) => {
            let model: WellTimeDepthAuthoredModel1D =
                serde_json::from_str(json_payload).map_err(|error| {
                    format!(
                        "failed to parse well time-depth authored model json '{}': {error}",
                        request.json_path
                    )
                })?;
            project.create_well_time_depth_authored_model(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
                &model,
            )
        }
        ("well_time_depth_authored_model", None) => project
            .import_well_time_depth_authored_model_json(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
            ),
        ("well_time_depth_model", Some(json_payload)) => {
            let model: WellTimeDepthModel1D =
                serde_json::from_str(json_payload).map_err(|error| {
                    format!(
                        "failed to parse well time-depth model json '{}': {error}",
                        request.json_path
                    )
                })?;
            project.create_well_time_depth_model(
                source_path,
                request.binding,
                request.collection_name.as_deref(),
                &model,
            )
        }
        ("well_time_depth_model", None) => project.import_well_time_depth_model_json(
            source_path,
            request.binding,
            request.collection_name.as_deref(),
        ),
        (other, _) => return Err(format!("unsupported well time-depth asset kind '{other}'")),
    }
    .map_err(|error| error.to_string())?;

    Ok(well_time_depth_import_response(result))
}

#[tauri::command]
fn commit_project_well_time_depth_import_command(
    request: CommitProjectWellTimeDepthImportRequest,
) -> Result<ImportProjectWellTimeDepthModelResponse, String> {
    import_project_well_time_depth_asset_command(ImportProjectWellTimeDepthAssetRequest {
        project_root: request.project_root,
        json_path: request.json_path,
        json_payload: Some(request.draft.json_payload),
        binding: request.binding,
        collection_name: request.draft.collection_name,
        asset_kind: request.draft.asset_kind,
    })
}

#[tauri::command]
fn analyze_project_well_tie_command(
    request: AnalyzeProjectWellTieRequest,
) -> Result<ProjectWellTieAnalysisResponse, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let source_model_asset_id = ophiolite::AssetId(request.source_model_asset_id.clone());
    let source_model = project
        .read_well_time_depth_model(&source_model_asset_id)
        .map_err(|error| error.to_string())?;
    let analysis = project
        .analyze_well_tie_from_model(
            &source_model_asset_id,
            &request.tie_name,
            request.tie_start_ms,
            request.tie_end_ms,
            request.search_radius_m,
        )
        .map_err(|error| error.to_string())?;
    let analysis = match enrich_well_tie_analysis_with_store(
        &project,
        &source_model_asset_id,
        &analysis,
        &request.store_path,
        &request.survey_asset_id,
        &request.display_coordinate_reference_id,
        request.search_radius_m,
    ) {
        Ok(enriched) => enriched,
        Err(error) => {
            let mut fallback = analysis;
            fallback.notes.push(format!(
                "Survey-backed seismic extraction fell back to the provisional preview: {error}"
            ));
            fallback
        }
    };
    Ok(ProjectWellTieAnalysisResponse {
        draft_observation_set: analysis.draft_observation_set.clone(),
        analysis,
        source_model_asset_id: request.source_model_asset_id,
        source_model_name: source_model.name,
    })
}

#[tauri::command]
fn accept_project_well_tie_command(
    request: AcceptProjectWellTieRequest,
) -> Result<AcceptProjectWellTieResponse, String> {
    let mut project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let source_model_asset_id = ophiolite::AssetId(request.source_model_asset_id.clone());
    let analysis = project
        .analyze_well_tie_from_model(
            &source_model_asset_id,
            &request.tie_name,
            request.tie_start_ms,
            request.tie_end_ms,
            request.search_radius_m,
        )
        .map_err(|error| error.to_string())?;
    let analysis = match enrich_well_tie_analysis_with_store(
        &project,
        &source_model_asset_id,
        &analysis,
        &request.store_path,
        &request.survey_asset_id,
        &request.display_coordinate_reference_id,
        request.search_radius_m,
    ) {
        Ok(enriched) => enriched,
        Err(error) => {
            let mut fallback = analysis;
            fallback.notes.push(format!(
                "Survey-backed seismic extraction fell back to the provisional preview: {error}"
            ));
            fallback
        }
    };
    let result = project
        .accept_well_tie_observation_set_from_model(
            &source_model_asset_id,
            request.binding,
            &request.tie_name,
            &analysis.draft_observation_set,
            request.output_collection_name.as_deref(),
            request.set_active,
        )
        .map_err(|error| error.to_string())?;
    Ok(AcceptProjectWellTieResponse {
        observation_asset_id: result.observation_result.asset.id.0,
        authored_model_asset_id: result.authored_result.asset.id.0,
        compiled_model_asset_id: result.compiled_result.asset.id.0,
    })
}

#[tauri::command]
fn compile_project_well_time_depth_authored_model_command(
    request: CompileProjectWellTimeDepthAuthoredModelRequest,
) -> Result<ImportProjectWellTimeDepthModelResponse, String> {
    let mut project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    let result = project
        .compile_well_time_depth_authored_model_to_asset(
            &ophiolite::AssetId(request.asset_id),
            request.output_collection_name.as_deref(),
            request.set_active,
        )
        .map_err(|error| error.to_string())?;
    Ok(well_time_depth_import_response(result))
}

#[tauri::command]
fn read_project_well_time_depth_model_command(
    request: ProjectAssetRequest,
) -> Result<WellTimeDepthModel1D, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    project
        .read_well_time_depth_model(&ophiolite::AssetId(request.asset_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn resolve_project_section_well_overlays_command(
    request: SectionWellOverlayRequestDto,
) -> Result<ResolveSectionWellOverlaysResponse, String> {
    let project = OphioliteProject::open(Path::new(&request.project_root))
        .map_err(|error| error.to_string())?;
    project
        .resolve_section_well_overlays(&request)
        .map_err(|error| error.to_string())
}

fn run_processing_job(
    app: &AppHandle,
    record: &JobRecord,
    request: RunTraceLocalProcessingRequest,
) {
    let app_paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            let _ = record.mark_failed(error.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "processing_job_failed",
                    log::Level::Error,
                    "Processing job failed before initialization",
                    Some(build_fields([("error", json_value(&error))])),
                );
            }
            return;
        }
    };
    let output_store_path = request.output_store_path.clone().unwrap_or_else(|| {
        default_processing_store_path(&app_paths, &request.store_path, &request.pipeline)
            .unwrap_or_else(|_| "derived-output.tbvol".to_string())
    });
    let job_id = record.snapshot().job_id;
    let reused_checkpoint = match app.try_state::<ProcessingCacheState>() {
        Some(processing_cache) => {
            match resolve_reused_trace_local_checkpoint(&processing_cache, &request, false) {
                Ok(value) => value,
                Err(error) => {
                    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                        diagnostics.emit_session_event(
                            app,
                            "processing_checkpoint_reuse_failed",
                            log::Level::Warn,
                            "Checkpoint reuse lookup failed; processing will continue from source",
                            Some(build_fields([
                                ("jobId", json_value(&job_id)),
                                ("error", json_value(error)),
                            ])),
                        );
                    }
                    None
                }
            }
        }
        None => None,
    };
    let source_fingerprint = match app.try_state::<ProcessingCacheState>() {
        Some(processing_cache) if processing_cache.enabled() => {
            match trace_local_source_fingerprint(&request.store_path) {
                Ok(value) => Some(value),
                Err(error) => {
                    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                        diagnostics.emit_session_event(
                            app,
                            "processing_cache_fingerprint_failed",
                            log::Level::Warn,
                            "Processing cache fingerprinting failed; processing will continue without prefix registration",
                            Some(build_fields([
                                ("jobId", json_value(&job_id)),
                                ("error", json_value(error)),
                            ])),
                        );
                    }
                    None
                }
            }
        }
        _ => None,
    };
    let stages = match build_trace_local_processing_stages_from(
        &request,
        &output_store_path,
        &job_id,
        reused_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.after_operation_index + 1)
            .unwrap_or(0),
    ) {
        Ok(stages) => stages,
        Err(error) => {
            let final_status = record.mark_failed(error);
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "processing_job_failed",
                    log::Level::Error,
                    "Processing job failed",
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        (
                            "error",
                            json_value(final_status.error_message.clone().unwrap_or_default()),
                        ),
                    ])),
                );
            }
            return;
        }
    };
    let job_started_at = Instant::now();
    let _ = record.mark_running(stages.first().map(|stage| stage.stage_label.clone()));
    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
        diagnostics.emit_session_event(
            app,
            "processing_job_started",
            log::Level::Info,
            "Processing job started",
            Some(build_fields([
                ("jobId", json_value(&job_id)),
                ("storePath", json_value(&request.store_path)),
                ("outputStorePath", json_value(&output_store_path)),
                ("stageCount", json_value(stages.len())),
                (
                    "operatorCount",
                    json_value(request.pipeline.operation_count()),
                ),
                ("reusedCheckpoint", json_value(reused_checkpoint.is_some())),
            ])),
        );
    }
    if let Err(error) = prepare_processing_output_store(
        &request.store_path,
        &output_store_path,
        request.overwrite_existing,
    ) {
        let final_status = record.mark_failed(error);
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "processing_job_failed",
                log::Level::Error,
                "Processing job failed",
                Some(build_fields([
                    ("jobId", json_value(&final_status.job_id)),
                    (
                        "error",
                        json_value(final_status.error_message.clone().unwrap_or_default()),
                    ),
                ])),
            );
        }
        return;
    }
    if let Some(reused_checkpoint) = reused_checkpoint.as_ref() {
        let _ = record.push_artifact(reused_checkpoint.artifact.clone());
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "processing_job_checkpoint_reused",
                log::Level::Info,
                "Processing job reused a cached checkpoint",
                Some(build_fields([
                    ("jobId", json_value(&job_id)),
                    ("storePath", json_value(&reused_checkpoint.path)),
                    (
                        "afterOperationIndex",
                        json_value(reused_checkpoint.after_operation_index),
                    ),
                ])),
            );
        }
    }
    let mut current_input_store_path = reused_checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.path.clone())
        .unwrap_or_else(|| request.store_path.clone());
    let result = stages.iter().try_for_each(|stage| {
        let stage_started_at = Instant::now();
        if record.cancel_requested() {
            return Err(seis_runtime::SeisRefineError::Message(
                "processing cancelled".to_string(),
            ));
        }
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "processing_job_stage_started",
                log::Level::Info,
                "Processing stage started",
                Some(build_fields([
                    ("jobId", json_value(&job_id)),
                    ("label", json_value(&stage.stage_label)),
                    ("inputStorePath", json_value(&current_input_store_path)),
                    ("outputStorePath", json_value(&stage.artifact.store_path)),
                    (
                        "artifactKind",
                        json_value(match stage.artifact.kind {
                            ProcessingJobArtifactKind::Checkpoint => "checkpoint",
                            ProcessingJobArtifactKind::FinalOutput => "final_output",
                        }),
                    ),
                    (
                        "segmentOperatorCount",
                        json_value(stage.segment_pipeline.operation_count()),
                    ),
                    (
                        "lineageOperatorCount",
                        json_value(stage.lineage_pipeline.operation_count()),
                    ),
                ])),
            );
        }
        if !matches!(stage.artifact.kind, ProcessingJobArtifactKind::FinalOutput) {
            prepare_processing_output_store(
                &current_input_store_path,
                &stage.artifact.store_path,
                false,
            )
            .map_err(seis_runtime::SeisRefineError::Message)?;
        }
        let materialize_options = materialize_options_for_store(&current_input_store_path)
            .map_err(seis_runtime::SeisRefineError::Message)?;
        let materialize_started_at = Instant::now();
        materialize_processing_volume_with_progress(
            &current_input_store_path,
            &stage.artifact.store_path,
            &stage.segment_pipeline,
            materialize_options,
            |completed, total| {
                if record.cancel_requested() {
                    return Err(seis_runtime::SeisRefineError::Message(
                        "processing cancelled".to_string(),
                    ));
                }
                let _ = record.mark_progress(completed, total, Some(&stage.stage_label));
                Ok(())
            },
        )?;
        let materialize_duration_ms = materialize_started_at.elapsed().as_millis() as u64;
        let lineage_rewrite_started_at = Instant::now();
        rewrite_trace_local_processing_lineage(
            &stage.artifact.store_path,
            &stage.lineage_pipeline,
            stage.artifact.kind,
        )
        .map_err(seis_runtime::SeisRefineError::Message)?;
        let lineage_rewrite_duration_ms =
            lineage_rewrite_started_at.elapsed().as_millis() as u64;
        let _ = record.push_artifact(stage.artifact.clone());
        let artifact_register_started_at = Instant::now();
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                if matches!(stage.artifact.kind, ProcessingJobArtifactKind::FinalOutput) {
                    "processing_job_output_emitted"
                } else {
                    "processing_job_checkpoint_emitted"
                },
                log::Level::Info,
                if matches!(stage.artifact.kind, ProcessingJobArtifactKind::FinalOutput) {
                    "Processing output emitted"
                } else {
                    "Processing checkpoint emitted"
                },
                Some(build_fields([
                    ("jobId", json_value(&job_id)),
                    ("storePath", json_value(&stage.artifact.store_path)),
                    ("label", json_value(&stage.artifact.label)),
                    (
                        "artifactKind",
                        json_value(match stage.artifact.kind {
                            ProcessingJobArtifactKind::Checkpoint => "checkpoint",
                            ProcessingJobArtifactKind::FinalOutput => "final_output",
                        }),
                    ),
                ])),
            );
        }
        if let Err(error) = register_processing_store_artifact(app, &request.store_path, &stage.artifact)
        {
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "processing_job_artifact_register_failed",
                    log::Level::Warn,
                    "Processing output emitted but workspace registration failed",
                    Some(build_fields([
                        ("jobId", json_value(&job_id)),
                        ("storePath", json_value(&stage.artifact.store_path)),
                        ("error", json_value(error)),
                    ])),
                );
            }
        }
        let artifact_register_duration_ms =
            artifact_register_started_at.elapsed().as_millis() as u64;
        if matches!(stage.artifact.kind, ProcessingJobArtifactKind::Checkpoint) {
            if let (Some(processing_cache), Some(source_fingerprint)) =
                (app.try_state::<ProcessingCacheState>(), source_fingerprint.as_ref())
            {
                if processing_cache.enabled() {
                    match trace_local_pipeline_hash(&stage.lineage_pipeline) {
                        Ok(prefix_hash) => {
                            if let Err(error) = processing_cache.register_visible_checkpoint(
                                TRACE_LOCAL_CACHE_FAMILY,
                                &stage.artifact.store_path,
                                source_fingerprint,
                                &prefix_hash,
                                stage.artifact.step_index + 1,
                                PROCESSING_CACHE_RUNTIME_VERSION,
                                TBVOL_STORE_FORMAT_VERSION,
                            ) {
                                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                                    diagnostics.emit_session_event(
                                        app,
                                        "processing_cache_checkpoint_register_failed",
                                        log::Level::Warn,
                                        "Processing checkpoint emitted but cache registration failed",
                                        Some(build_fields([
                                            ("jobId", json_value(&job_id)),
                                            ("storePath", json_value(&stage.artifact.store_path)),
                                            ("error", json_value(error)),
                                        ])),
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                                diagnostics.emit_session_event(
                                    app,
                                    "processing_cache_checkpoint_hash_failed",
                                    log::Level::Warn,
                                    "Processing checkpoint emitted but cache hashing failed",
                                    Some(build_fields([
                                        ("jobId", json_value(&job_id)),
                                        ("storePath", json_value(&stage.artifact.store_path)),
                                        ("error", json_value(error)),
                                    ])),
                                );
                            }
                        }
                    }
                }
            }
        }
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "processing_job_stage_completed",
                log::Level::Info,
                "Processing stage completed",
                Some(build_fields([
                    ("jobId", json_value(&job_id)),
                    ("label", json_value(&stage.stage_label)),
                    ("storePath", json_value(&stage.artifact.store_path)),
                    (
                        "artifactKind",
                        json_value(match stage.artifact.kind {
                            ProcessingJobArtifactKind::Checkpoint => "checkpoint",
                            ProcessingJobArtifactKind::FinalOutput => "final_output",
                        }),
                    ),
                    (
                        "stageDurationMs",
                        json_value(stage_started_at.elapsed().as_millis() as u64),
                    ),
                    ("materializeDurationMs", json_value(materialize_duration_ms)),
                    (
                        "lineageRewriteDurationMs",
                        json_value(lineage_rewrite_duration_ms),
                    ),
                    (
                        "artifactRegisterDurationMs",
                        json_value(artifact_register_duration_ms),
                    ),
                ])),
            );
        }
        current_input_store_path = stage.artifact.store_path.clone();
        Ok(())
    });

    match result {
        Ok(_) => {
            if let Some(processing_cache) = app.try_state::<ProcessingCacheState>() {
                if processing_cache.enabled() {
                    match (
                        source_fingerprint.as_ref(),
                        trace_local_pipeline_hash(&request.pipeline),
                    ) {
                        (Some(source_fingerprint), Ok(full_pipeline_hash)) => {
                            if let Err(error) = processing_cache.register_visible_output(
                                TRACE_LOCAL_CACHE_FAMILY,
                                &output_store_path,
                                source_fingerprint,
                                &full_pipeline_hash,
                                &full_pipeline_hash,
                                request.pipeline.operation_count(),
                                PROCESSING_CACHE_RUNTIME_VERSION,
                                TBVOL_STORE_FORMAT_VERSION,
                            ) {
                                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                                    diagnostics.emit_session_event(
                                        app,
                                        "processing_cache_register_failed",
                                        log::Level::Warn,
                                        "Processing output completed but cache registration failed",
                                        Some(build_fields([
                                            ("jobId", json_value(&job_id)),
                                            ("outputStorePath", json_value(&output_store_path)),
                                            ("error", json_value(error)),
                                        ])),
                                    );
                                }
                            }
                        }
                        (_, Err(error)) => {
                            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                                diagnostics.emit_session_event(
                                    app,
                                    "processing_cache_fingerprint_failed",
                                    log::Level::Warn,
                                    "Processing output completed but cache fingerprinting failed",
                                    Some(build_fields([
                                        ("jobId", json_value(&job_id)),
                                        ("outputStorePath", json_value(&output_store_path)),
                                        ("error", json_value(error)),
                                    ])),
                                );
                            }
                        }
                        (None, _) => {}
                    }
                }
            }
            let final_status = record.mark_completed(output_store_path.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "processing_job_completed",
                    log::Level::Info,
                    "Processing job completed",
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        ("outputStorePath", json_value(&output_store_path)),
                        (
                            "jobDurationMs",
                            json_value(job_started_at.elapsed().as_millis() as u64),
                        ),
                    ])),
                );
            }
        }
        Err(error) => {
            let final_status = if record.cancel_requested() {
                record.mark_cancelled()
            } else {
                record.mark_failed(error.to_string())
            };
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "processing_job_failed",
                    if matches!(
                        final_status.state,
                        seis_runtime::ProcessingJobState::Cancelled
                    ) {
                        log::Level::Warn
                    } else {
                        log::Level::Error
                    },
                    if matches!(
                        final_status.state,
                        seis_runtime::ProcessingJobState::Cancelled
                    ) {
                        "Processing job cancelled"
                    } else {
                        "Processing job failed"
                    },
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        (
                            "jobDurationMs",
                            json_value(job_started_at.elapsed().as_millis() as u64),
                        ),
                        (
                            "state",
                            json_value(format!("{:?}", final_status.state).to_ascii_lowercase()),
                        ),
                        (
                            "error",
                            json_value(final_status.error_message.clone().unwrap_or_default()),
                        ),
                    ])),
                );
            }
        }
    }
}

fn run_subvolume_processing_job(
    app: &AppHandle,
    record: &JobRecord,
    request: RunSubvolumeProcessingRequest,
) {
    let app_paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            let _ = record.mark_failed(error.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_job_failed",
                    log::Level::Error,
                    "Subvolume processing job failed before initialization",
                    Some(build_fields([("error", json_value(&error))])),
                );
            }
            return;
        }
    };

    let output_store_path = request.output_store_path.clone().unwrap_or_else(|| {
        default_subvolume_processing_store_path(&app_paths, &request.store_path, &request.pipeline)
            .unwrap_or_else(|_| "derived-output.tbvol".to_string())
    });
    let job_started_at = Instant::now();
    let job_id = record.snapshot().job_id;
    let prefix_request = request
        .pipeline
        .trace_local_pipeline
        .as_ref()
        .map(|pipeline| RunTraceLocalProcessingRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: request.store_path.clone(),
            output_store_path: None,
            overwrite_existing: false,
            pipeline: pipeline.clone(),
        });
    let reused_checkpoint = match (
        app.try_state::<ProcessingCacheState>(),
        prefix_request.as_ref(),
    ) {
        (Some(processing_cache), Some(prefix_request)) => {
            match resolve_reused_trace_local_checkpoint(&processing_cache, prefix_request, true) {
                Ok(value) => value,
                Err(error) => {
                    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                        diagnostics.emit_session_event(
                            app,
                            "subvolume_processing_checkpoint_reuse_failed",
                            log::Level::Warn,
                            "Checkpoint reuse lookup failed; subvolume processing will continue from source",
                            Some(build_fields([
                                ("jobId", json_value(&job_id)),
                                ("error", json_value(error)),
                            ])),
                        );
                    }
                    None
                }
            }
        }
        _ => None,
    };
    let source_fingerprint = match (
        app.try_state::<ProcessingCacheState>(),
        request.pipeline.trace_local_pipeline.as_ref(),
    ) {
        (Some(processing_cache), Some(_)) if processing_cache.enabled() => {
            match trace_local_source_fingerprint(&request.store_path) {
                Ok(value) => Some(value),
                Err(error) => {
                    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                        diagnostics.emit_session_event(
                            app,
                            "subvolume_processing_cache_fingerprint_failed",
                            log::Level::Warn,
                            "Processing cache fingerprinting failed; subvolume processing will continue without prefix registration",
                            Some(build_fields([
                                ("jobId", json_value(&job_id)),
                                ("error", json_value(error)),
                            ])),
                        );
                    }
                    None
                }
            }
        }
        _ => None,
    };
    let checkpoint_stages = match request.pipeline.trace_local_pipeline.as_ref() {
        Some(pipeline) => match build_trace_local_checkpoint_stages_from_pipeline(
            pipeline,
            &output_store_path,
            &job_id,
            reused_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.after_operation_index + 1)
                .unwrap_or(0),
            true,
        ) {
            Ok(stages) => stages,
            Err(error) => {
                let final_status = record.mark_failed(error);
                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                    diagnostics.emit_session_event(
                        app,
                        "subvolume_processing_job_failed",
                        log::Level::Error,
                        "Subvolume processing job failed",
                        Some(build_fields([
                            ("jobId", json_value(&final_status.job_id)),
                            (
                                "error",
                                json_value(final_status.error_message.clone().unwrap_or_default()),
                            ),
                        ])),
                    );
                }
                return;
            }
        },
        None => Vec::new(),
    };
    let _ = record.mark_running(
        checkpoint_stages
            .first()
            .map(|stage| stage.stage_label.clone())
            .or_else(|| Some("Crop Subvolume".to_string())),
    );
    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
        diagnostics.emit_session_event(
            app,
            "subvolume_processing_job_started",
            log::Level::Info,
            "Subvolume processing job started",
            Some(build_fields([
                ("jobId", json_value(&record.snapshot().job_id)),
                ("storePath", json_value(&request.store_path)),
                ("outputStorePath", json_value(&output_store_path)),
                (
                    "traceLocalOperatorCount",
                    json_value(
                        request
                            .pipeline
                            .trace_local_pipeline
                            .as_ref()
                            .map(|pipeline| pipeline.operation_count())
                            .unwrap_or(0),
                    ),
                ),
                ("checkpointCount", json_value(checkpoint_stages.len())),
                ("reusedCheckpoint", json_value(reused_checkpoint.is_some())),
            ])),
        );
    }
    if let Err(error) = prepare_processing_output_store(
        &request.store_path,
        &output_store_path,
        request.overwrite_existing,
    ) {
        let final_status = record.mark_failed(error);
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "subvolume_processing_job_failed",
                log::Level::Error,
                "Subvolume processing job failed",
                Some(build_fields([
                    ("jobId", json_value(&final_status.job_id)),
                    (
                        "error",
                        json_value(final_status.error_message.clone().unwrap_or_default()),
                    ),
                ])),
            );
        }
        return;
    }

    let final_materialize_options = match materialize_options_for_store(&request.store_path) {
        Ok(options) => options,
        Err(error) => {
            let final_status = record.mark_failed(error.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_job_failed",
                    log::Level::Error,
                    "Subvolume processing job failed",
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        ("error", json_value(&error)),
                    ])),
                );
            }
            return;
        }
    };
    if let Some(reused_checkpoint) = reused_checkpoint.as_ref() {
        let _ = record.push_artifact(reused_checkpoint.artifact.clone());
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "subvolume_processing_checkpoint_reused",
                log::Level::Info,
                "Subvolume processing reused a cached checkpoint",
                Some(build_fields([
                    ("jobId", json_value(&job_id)),
                    ("storePath", json_value(&reused_checkpoint.path)),
                    (
                        "afterOperationIndex",
                        json_value(reused_checkpoint.after_operation_index),
                    ),
                ])),
            );
        }
    }

    let mut current_input_store_path = reused_checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.path.clone())
        .unwrap_or_else(|| request.store_path.clone());
    let checkpoint_result: Result<(), seis_runtime::SeisRefineError> =
        checkpoint_stages.iter().try_for_each(|stage| {
            let stage_started_at = Instant::now();
            if record.cancel_requested() {
                return Err(seis_runtime::SeisRefineError::Message(
                    "processing cancelled".to_string(),
                ));
            }
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_checkpoint_stage_started",
                    log::Level::Info,
                    "Subvolume checkpoint stage started",
                    Some(build_fields([
                        ("jobId", json_value(&job_id)),
                        ("label", json_value(&stage.stage_label)),
                        ("inputStorePath", json_value(&current_input_store_path)),
                        ("outputStorePath", json_value(&stage.artifact.store_path)),
                        (
                            "segmentOperatorCount",
                            json_value(stage.segment_pipeline.operation_count()),
                        ),
                        (
                            "lineageOperatorCount",
                            json_value(stage.lineage_pipeline.operation_count()),
                        ),
                    ])),
                );
            }
            prepare_processing_output_store(
                &current_input_store_path,
                &stage.artifact.store_path,
                false,
            )
            .map_err(seis_runtime::SeisRefineError::Message)?;
            let stage_materialize_options = materialize_options_for_store(&current_input_store_path)
                .map_err(seis_runtime::SeisRefineError::Message)?;
            let materialize_started_at = Instant::now();
            materialize_processing_volume_with_progress(
                &current_input_store_path,
                &stage.artifact.store_path,
                &stage.segment_pipeline,
                stage_materialize_options,
                |completed, total| {
                    if record.cancel_requested() {
                        return Err(seis_runtime::SeisRefineError::Message(
                            "processing cancelled".to_string(),
                        ));
                    }
                    let _ = record.mark_progress(completed, total, Some(&stage.stage_label));
                    Ok(())
                },
            )?;
            let materialize_duration_ms = materialize_started_at.elapsed().as_millis() as u64;
            let lineage_rewrite_started_at = Instant::now();
            rewrite_trace_local_processing_lineage(
                &stage.artifact.store_path,
                &stage.lineage_pipeline,
                stage.artifact.kind,
            )
            .map_err(seis_runtime::SeisRefineError::Message)?;
            let lineage_rewrite_duration_ms =
                lineage_rewrite_started_at.elapsed().as_millis() as u64;
            let _ = record.push_artifact(stage.artifact.clone());
            let artifact_register_started_at = Instant::now();
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_checkpoint_emitted",
                    log::Level::Info,
                    "Subvolume checkpoint emitted",
                    Some(build_fields([
                        ("jobId", json_value(&job_id)),
                        ("storePath", json_value(&stage.artifact.store_path)),
                        ("label", json_value(&stage.artifact.label)),
                    ])),
                );
            }
            if let Err(error) =
                register_processing_store_artifact(app, &request.store_path, &stage.artifact)
            {
                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                    diagnostics.emit_session_event(
                        app,
                        "subvolume_processing_artifact_register_failed",
                        log::Level::Warn,
                        "Subvolume checkpoint emitted but workspace registration failed",
                        Some(build_fields([
                            ("jobId", json_value(&job_id)),
                            ("storePath", json_value(&stage.artifact.store_path)),
                            ("error", json_value(error)),
                        ])),
                    );
                }
            }
            let artifact_register_duration_ms =
                artifact_register_started_at.elapsed().as_millis() as u64;
            if let (Some(processing_cache), Some(source_fingerprint)) =
                (app.try_state::<ProcessingCacheState>(), source_fingerprint.as_ref())
            {
                if processing_cache.enabled() {
                    match trace_local_pipeline_hash(&stage.lineage_pipeline) {
                        Ok(prefix_hash) => {
                            if let Err(error) = processing_cache.register_visible_checkpoint(
                                TRACE_LOCAL_CACHE_FAMILY,
                                &stage.artifact.store_path,
                                source_fingerprint,
                                &prefix_hash,
                                stage.artifact.step_index + 1,
                                PROCESSING_CACHE_RUNTIME_VERSION,
                                TBVOL_STORE_FORMAT_VERSION,
                            ) {
                                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                                    diagnostics.emit_session_event(
                                        app,
                                        "subvolume_processing_cache_checkpoint_register_failed",
                                        log::Level::Warn,
                                        "Subvolume checkpoint emitted but cache registration failed",
                                        Some(build_fields([
                                            ("jobId", json_value(&job_id)),
                                            ("storePath", json_value(&stage.artifact.store_path)),
                                            ("error", json_value(error)),
                                        ])),
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                                diagnostics.emit_session_event(
                                    app,
                                    "subvolume_processing_cache_hash_failed",
                                    log::Level::Warn,
                                    "Subvolume checkpoint emitted but cache prefix hashing failed",
                                    Some(build_fields([
                                        ("jobId", json_value(&job_id)),
                                        ("storePath", json_value(&stage.artifact.store_path)),
                                        ("error", json_value(error)),
                                    ])),
                                );
                            }
                        }
                    }
                }
            }
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_checkpoint_stage_completed",
                    log::Level::Info,
                    "Subvolume checkpoint stage completed",
                    Some(build_fields([
                        ("jobId", json_value(&job_id)),
                        ("label", json_value(&stage.stage_label)),
                        (
                            "stageDurationMs",
                            json_value(stage_started_at.elapsed().as_millis() as u64),
                        ),
                        (
                            "materializeDurationMs",
                            json_value(materialize_duration_ms),
                        ),
                        (
                            "lineageRewriteDurationMs",
                            json_value(lineage_rewrite_duration_ms),
                        ),
                        (
                            "artifactRegisterDurationMs",
                            json_value(artifact_register_duration_ms),
                        ),
                    ])),
                );
            }
            current_input_store_path = stage.artifact.store_path.clone();
            Ok(())
        });

    let result = checkpoint_result.and_then(|_| {
        let remaining_trace_local_pipeline = request
            .pipeline
            .trace_local_pipeline
            .as_ref()
            .and_then(|pipeline| {
                let start_index = checkpoint_stages
                    .last()
                    .map(|stage| stage.artifact.step_index + 1)
                    .or_else(|| {
                        reused_checkpoint
                            .as_ref()
                            .map(|checkpoint| checkpoint.after_operation_index + 1)
                    })
                    .unwrap_or(0);
                (start_index < pipeline.operation_count()).then(|| {
                    pipeline_segment(pipeline, start_index, pipeline.operation_count() - 1)
                })
            });
        let execution_pipeline = SubvolumeProcessingPipeline {
            schema_version: request.pipeline.schema_version,
            revision: request.pipeline.revision,
            preset_id: request.pipeline.preset_id.clone(),
            name: request.pipeline.name.clone(),
            description: request.pipeline.description.clone(),
            trace_local_pipeline: remaining_trace_local_pipeline,
            crop: request.pipeline.crop.clone(),
        };
        materialize_subvolume_processing_volume_with_progress(
            &current_input_store_path,
            &output_store_path,
            &execution_pipeline,
            final_materialize_options,
            |completed, total| {
                if record.cancel_requested() {
                    return Err(seis_runtime::SeismicStoreError::Message(
                        "processing cancelled".to_string(),
                    ));
                }
                let _ = record.mark_progress(completed, total, Some("Crop Subvolume"));
                Ok(())
            },
        )
        .map_err(|error| seis_runtime::SeisRefineError::Message(error.to_string()))
    });

    match result {
        Ok(_) => {
            if let Err(error) = rewrite_subvolume_processing_lineage(
                &output_store_path,
                &request.pipeline,
                ProcessingJobArtifactKind::FinalOutput,
            ) {
                let final_status = record.mark_failed(error.clone());
                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                    diagnostics.emit_session_event(
                        app,
                        "subvolume_processing_job_failed",
                        log::Level::Error,
                        "Subvolume processing job failed",
                        Some(build_fields([
                            ("jobId", json_value(&final_status.job_id)),
                            ("error", json_value(&error)),
                        ])),
                    );
                }
                return;
            }
            let final_artifact = ProcessingJobArtifact {
                kind: ProcessingJobArtifactKind::FinalOutput,
                step_index: request
                    .pipeline
                    .trace_local_pipeline
                    .as_ref()
                    .map(|pipeline| pipeline.operation_count())
                    .unwrap_or(0),
                label: "Crop Subvolume".to_string(),
                store_path: output_store_path.clone(),
            };
            let _ = record.push_artifact(final_artifact.clone());
            if let Err(error) =
                register_processing_store_artifact(app, &request.store_path, &final_artifact)
            {
                if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                    diagnostics.emit_session_event(
                        app,
                        "subvolume_processing_artifact_register_failed",
                        log::Level::Warn,
                        "Subvolume output emitted but workspace registration failed",
                        Some(build_fields([
                            ("jobId", json_value(&job_id)),
                            ("storePath", json_value(&final_artifact.store_path)),
                            ("error", json_value(error)),
                        ])),
                    );
                }
            }
            let final_status = record.mark_completed(output_store_path.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                let mut completion_fields = vec![
                    ("jobId", json_value(&final_status.job_id)),
                    ("outputStorePath", json_value(&output_store_path)),
                    ("executionOrder", json_value("trace_local_then_crop")),
                    (
                        "jobDurationMs",
                        json_value(job_started_at.elapsed().as_millis() as u64),
                    ),
                    ("checkpointCount", json_value(checkpoint_stages.len())),
                ];
                if let Ok(handle) = open_store(&output_store_path) {
                    completion_fields.push(("datasetId", json_value(&handle.dataset_id().0)));
                    completion_fields.push(("shape", json_value(handle.manifest.volume.shape)));
                    if let Some((inline_min, inline_max)) =
                        section_axis_range(&handle, SectionAxis::Inline)
                    {
                        completion_fields
                            .push(("inlineRange", json_value([inline_min, inline_max])));
                    }
                    if let Some((xline_min, xline_max)) =
                        section_axis_range(&handle, SectionAxis::Xline)
                    {
                        completion_fields.push(("xlineRange", json_value([xline_min, xline_max])));
                    }
                    if let Some((z_min, z_max)) = sample_axis_range_ms(&handle) {
                        completion_fields.push(("zRangeMs", json_value([z_min, z_max])));
                    }
                }
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_job_completed",
                    log::Level::Info,
                    "Subvolume processing job completed",
                    Some(build_fields(completion_fields)),
                );
            }
        }
        Err(error) => {
            let final_status = if record.cancel_requested() {
                record.mark_cancelled()
            } else {
                record.mark_failed(error.to_string())
            };
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "subvolume_processing_job_failed",
                    if matches!(
                        final_status.state,
                        seis_runtime::ProcessingJobState::Cancelled
                    ) {
                        log::Level::Warn
                    } else {
                        log::Level::Error
                    },
                    if matches!(
                        final_status.state,
                        seis_runtime::ProcessingJobState::Cancelled
                    ) {
                        "Subvolume processing job cancelled"
                    } else {
                        "Subvolume processing job failed"
                    },
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        (
                            "jobDurationMs",
                            json_value(job_started_at.elapsed().as_millis() as u64),
                        ),
                        ("checkpointCount", json_value(checkpoint_stages.len())),
                        (
                            "state",
                            json_value(format!("{:?}", final_status.state).to_ascii_lowercase()),
                        ),
                        (
                            "error",
                            json_value(final_status.error_message.clone().unwrap_or_default()),
                        ),
                    ])),
                );
            }
        }
    }
}

fn run_gather_processing_job(
    app: &AppHandle,
    record: &JobRecord,
    request: RunGatherProcessingRequest,
) {
    let app_paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            let _ = record.mark_failed(error.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "gather_processing_job_failed",
                    log::Level::Error,
                    "Gather processing job failed before initialization",
                    Some(build_fields([("error", json_value(&error))])),
                );
            }
            return;
        }
    };
    let output_store_path = request.output_store_path.clone().unwrap_or_else(|| {
        default_gather_processing_store_path(&app_paths, &request.store_path, &request.pipeline)
            .unwrap_or_else(|_| "derived-output.tbgath".to_string())
    });
    let job_started_at = Instant::now();
    let _ = record.mark_running(None);
    if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
        diagnostics.emit_session_event(
            app,
            "gather_processing_job_started",
            log::Level::Info,
            "Gather processing job started",
            Some(build_fields([
                ("jobId", json_value(&record.snapshot().job_id)),
                ("storePath", json_value(&request.store_path)),
                ("outputStorePath", json_value(&output_store_path)),
                (
                    "operatorCount",
                    json_value(request.pipeline.operations.len()),
                ),
            ])),
        );
    }
    if let Err(error) = prepare_processing_output_store(
        &request.store_path,
        &output_store_path,
        request.overwrite_existing,
    ) {
        let final_status = record.mark_failed(error);
        if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
            diagnostics.emit_session_event(
                app,
                "gather_processing_job_failed",
                log::Level::Error,
                "Gather processing job failed",
                Some(build_fields([
                    ("jobId", json_value(&final_status.job_id)),
                    (
                        "error",
                        json_value(final_status.error_message.clone().unwrap_or_default()),
                    ),
                ])),
            );
        }
        return;
    }

    let result = materialize_gather_processing_store_with_progress(
        &request.store_path,
        &output_store_path,
        &request.pipeline,
        |completed, total| {
            if record.cancel_requested() {
                return Err(seis_runtime::SeisRefineError::Message(
                    "processing cancelled".to_string(),
                ));
            }
            let _ = record.mark_progress(completed, total, None);
            Ok(())
        },
    );

    match result {
        Ok(_) => {
            let final_status = record.mark_completed(output_store_path.clone());
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "gather_processing_job_completed",
                    log::Level::Info,
                    "Gather processing job completed",
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        ("outputStorePath", json_value(&output_store_path)),
                        (
                            "jobDurationMs",
                            json_value(job_started_at.elapsed().as_millis() as u64),
                        ),
                    ])),
                );
            }
        }
        Err(error) => {
            let final_status = if record.cancel_requested() {
                record.mark_cancelled()
            } else {
                record.mark_failed(error.to_string())
            };
            if let Some(diagnostics) = app.try_state::<DiagnosticsState>() {
                diagnostics.emit_session_event(
                    app,
                    "gather_processing_job_failed",
                    if matches!(
                        final_status.state,
                        seis_runtime::ProcessingJobState::Cancelled
                    ) {
                        log::Level::Warn
                    } else {
                        log::Level::Error
                    },
                    if matches!(
                        final_status.state,
                        seis_runtime::ProcessingJobState::Cancelled
                    ) {
                        "Gather processing job cancelled"
                    } else {
                        "Gather processing job failed"
                    },
                    Some(build_fields([
                        ("jobId", json_value(&final_status.job_id)),
                        (
                            "jobDurationMs",
                            json_value(job_started_at.elapsed().as_millis() as u64),
                        ),
                        (
                            "state",
                            json_value(format!("{:?}", final_status.state).to_ascii_lowercase()),
                        ),
                        (
                            "error",
                            json_value(final_status.error_message.clone().unwrap_or_default()),
                        ),
                    ])),
                );
            }
        }
    }
}

fn prepare_processing_output_store(
    input_store_path: &str,
    output_store_path: &str,
    overwrite_existing: bool,
) -> Result<(), String> {
    let input_path = std::path::Path::new(input_store_path);
    let output_path = std::path::Path::new(output_store_path);
    let input_canonical = input_path
        .canonicalize()
        .unwrap_or_else(|_| input_path.to_path_buf());
    let output_canonical = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.to_path_buf());
    if input_canonical == output_canonical {
        return Err("Output store path cannot overwrite the input store.".to_string());
    }
    if !output_path.exists() {
        return Ok(());
    }
    if !overwrite_existing {
        return Err(format!(
            "Output processing store already exists: {}",
            output_path.display()
        ));
    }
    let metadata = std::fs::symlink_metadata(output_path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(output_path).map_err(|error| error.to_string())?;
    } else {
        std::fs::remove_file(output_path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_diagnostics_status_command(
    diagnostics: State<DiagnosticsState>,
) -> Result<diagnostics::DiagnosticsStatus, String> {
    Ok(diagnostics.status())
}

#[tauri::command]
fn set_diagnostics_verbosity_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    enabled: bool,
) -> Result<(), String> {
    diagnostics.set_verbose_enabled(enabled);
    diagnostics.emit_session_event(
        &app,
        "config",
        if enabled {
            log::Level::Info
        } else {
            log::Level::Warn
        },
        if enabled {
            "Verbose diagnostics enabled for this session"
        } else {
            "Verbose diagnostics disabled for this session"
        },
        Some(build_fields([("verboseEnabled", json_value(enabled))])),
    );
    Ok(())
}

#[tauri::command]
fn export_diagnostics_bundle_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
) -> Result<ExportBundleResponse, String> {
    let bundle_path = diagnostics.export_bundle(&app)?;
    diagnostics.emit_session_event(
        &app,
        "exported",
        log::Level::Info,
        "Exported diagnostics bundle",
        Some(build_fields([(
            "bundlePath",
            json_value(bundle_path.display().to_string()),
        )])),
    );
    Ok(ExportBundleResponse {
        bundle_path: bundle_path.display().to_string(),
    })
}

#[tauri::command]
fn emit_frontend_diagnostics_event_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: FrontendDiagnosticsEventRequest,
) -> Result<(), String> {
    let level = match request.level.trim().to_ascii_lowercase().as_str() {
        "error" => log::Level::Error,
        "warn" | "warning" => log::Level::Warn,
        "debug" => log::Level::Debug,
        _ => log::Level::Info,
    };
    let mut fields = request.fields.unwrap_or_default();
    fields.insert("frontendStage".to_string(), json_value(request.stage));

    diagnostics.emit_session_event(
        &app,
        "frontend_profile",
        level,
        request.message,
        Some(fields),
    );
    Ok(())
}

#[tauri::command]
fn run_section_browsing_benchmark_command(
    app: AppHandle,
    diagnostics: State<DiagnosticsState>,
    request: RunSectionBrowsingBenchmarkRequest,
) -> Result<RunSectionBrowsingBenchmarkResponse, String> {
    let store_path = request.store_path.trim().to_string();
    if store_path.is_empty() {
        return Err("Store path is required.".to_string());
    }

    let iterations = benchmark_iterations(request.iterations);
    let include_full_section_baseline = request.include_full_section_baseline.unwrap_or(true);
    let step_offsets = benchmark_step_offsets(request.step_offsets);
    let primary_axis_name = axis_name(request.axis);

    let handle = open_store(&store_path).map_err(|error| error.to_string())?;
    if request.section_index >= section_axis_length(&handle, request.axis) {
        return Err(format!(
            "section index {} exceeds axis length {}",
            request.section_index,
            section_axis_length(&handle, request.axis)
        ));
    }

    let trace_range = validate_benchmark_range(
        request.trace_range,
        section_axis_length(&handle, request.axis),
        "trace",
    )?;
    let sample_range = validate_benchmark_range(
        request.sample_range,
        handle.manifest.volume.shape[2],
        "sample",
    )?;

    let operation = diagnostics.start_operation(
        &app,
        "run_section_browsing_benchmark",
        "Running section browsing benchmark scenario",
        Some(build_fields([
            ("storePath", json_value(&store_path)),
            ("datasetId", json_value(&handle.dataset_id().0)),
            ("shape", json_value(handle.manifest.volume.shape)),
            ("tileShape", json_value(handle.manifest.tile_shape)),
            ("axis", json_value(&primary_axis_name)),
            ("sectionIndex", json_value(request.section_index)),
            ("traceRange", json_value(trace_range)),
            ("sampleRange", json_value(sample_range)),
            ("lod", json_value(request.lod)),
            ("iterations", json_value(iterations)),
            (
                "includeFullSectionBaseline",
                json_value(include_full_section_baseline),
            ),
            ("stepOffsets", json_value(&step_offsets)),
        ])),
    );

    let mut cases = Vec::new();
    if include_full_section_baseline {
        let case =
            measure_full_section_case(&handle, request.axis, request.section_index, iterations)?;
        diagnostics.progress(
            &app,
            &operation,
            "Measured full-section baseline",
            Some(build_fields([
                ("scenario", json_value(&case.scenario)),
                ("axis", json_value(&case.axis)),
                ("index", json_value(case.index)),
                ("payloadBytes", json_value(case.payload_bytes)),
                ("medianMs", json_value(case.median_ms)),
            ])),
        );
        cases.push(case);
    }

    let active_case = measure_section_tile_case(
        &handle,
        "active_viewport_tile".to_string(),
        request.axis,
        request.section_index,
        trace_range,
        sample_range,
        request.lod,
        iterations,
    )?;
    diagnostics.progress(
        &app,
        &operation,
        "Measured active viewport tile case",
        Some(build_fields([
            ("scenario", json_value(&active_case.scenario)),
            ("axis", json_value(&active_case.axis)),
            ("index", json_value(active_case.index)),
            ("payloadBytes", json_value(active_case.payload_bytes)),
            ("medianMs", json_value(active_case.median_ms)),
        ])),
    );
    cases.push(active_case);

    let mut current_index = request.section_index;
    let axis_length = section_axis_length(&handle, request.axis);
    for (position, offset) in step_offsets.iter().enumerate() {
        current_index = stepped_section_index(current_index, *offset, axis_length);
        let case = measure_section_tile_case(
            &handle,
            format!("neighbor_step_{}_offset_{}", position + 1, offset),
            request.axis,
            current_index,
            trace_range,
            sample_range,
            request.lod,
            iterations,
        )?;
        diagnostics.progress(
            &app,
            &operation,
            "Measured neighboring section tile case",
            Some(build_fields([
                ("scenario", json_value(&case.scenario)),
                ("axis", json_value(&case.axis)),
                ("index", json_value(case.index)),
                ("payloadBytes", json_value(case.payload_bytes)),
                ("medianMs", json_value(case.median_ms)),
            ])),
        );
        cases.push(case);
    }

    let mut switch_axis_name = None;
    let mut switch_section_index = None;
    if let Some(switch_axis) = request.switch_axis {
        let target_axis_length = section_axis_length(&handle, switch_axis);
        let target_index = request
            .switch_section_index
            .unwrap_or(current_index)
            .min(target_axis_length.saturating_sub(1));
        let switch_trace_range = clamp_window_to_total(trace_range, target_axis_length);
        let case = measure_section_tile_case(
            &handle,
            "axis_switch_tile".to_string(),
            switch_axis,
            target_index,
            switch_trace_range,
            sample_range,
            request.lod,
            iterations,
        )?;
        diagnostics.progress(
            &app,
            &operation,
            "Measured switched-axis section tile case",
            Some(build_fields([
                ("scenario", json_value(&case.scenario)),
                ("axis", json_value(&case.axis)),
                ("index", json_value(case.index)),
                ("traceRange", json_value(case.trace_range)),
                ("payloadBytes", json_value(case.payload_bytes)),
                ("medianMs", json_value(case.median_ms)),
            ])),
        );
        switch_axis_name = Some(axis_name(switch_axis));
        switch_section_index = Some(target_index);
        cases.push(case);
    }

    let response = RunSectionBrowsingBenchmarkResponse {
        session_log_path: diagnostics.session_log_path().display().to_string(),
        store_path,
        dataset_id: handle.dataset_id().0.clone(),
        shape: handle.manifest.volume.shape,
        tile_shape: handle.manifest.tile_shape,
        axis: primary_axis_name,
        section_index: request.section_index,
        trace_range,
        sample_range,
        lod: request.lod,
        iterations,
        include_full_section_baseline,
        step_offsets,
        switch_axis: switch_axis_name,
        switch_section_index,
        cases,
    };

    diagnostics.complete(
        &app,
        &operation,
        "Section browsing benchmark scenario completed",
        Some(build_fields([
            ("datasetId", json_value(&response.dataset_id)),
            ("caseCount", json_value(response.cases.len())),
            ("sessionLogPath", json_value(&response.session_log_path)),
        ])),
    );

    Ok(response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let session_basename = DiagnosticsState::session_basename();
    let enable_devtools = cfg!(debug_assertions)
        && std::env::var("TRACEBOOST_ENABLE_DEVTOOLS")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

    let log_plugin_builder = tauri_plugin_log::Builder::default()
        .clear_targets()
        .level(log::LevelFilter::Info)
        .level_for("traceboost_desktop_lib", log::LevelFilter::Debug)
        .level_for(
            "traceboost_desktop_lib::diagnostics",
            log::LevelFilter::Debug,
        )
        .level_for("traceboost_app", log::LevelFilter::Debug)
        .level_for("seis_runtime", log::LevelFilter::Info)
        .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Stdout,
        ));
    let log_plugin_builder = if let Some(logs_dir) = preferred_traceboost_logs_dir() {
        log_plugin_builder.target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Folder {
                path: logs_dir,
                file_name: Some(session_basename.clone()),
            },
        ))
    } else {
        log_plugin_builder.target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::LogDir {
                file_name: Some(session_basename.clone()),
            },
        ))
    };
    let log_plugin = log_plugin_builder.build();

    let builder = tauri::Builder::default()
        .menu(build_app_menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            APP_SETTINGS_MENU_ID => {
                if let Err(error) = app.emit(APP_SETTINGS_MENU_EVENT, ()) {
                    log::warn!("failed to emit native settings menu event: {error}");
                }
            }
            APP_VELOCITY_MODEL_MENU_ID => {
                if let Err(error) = app.emit(APP_VELOCITY_MODEL_MENU_EVENT, ()) {
                    log::warn!("failed to emit native velocity-model menu event: {error}");
                }
            }
            APP_RESIDUALS_MENU_ID => {
                if let Err(error) = app.emit(APP_RESIDUALS_MENU_EVENT, ()) {
                    log::warn!("failed to emit native residuals menu event: {error}");
                }
            }
            APP_DEPTH_CONVERSION_MENU_ID => {
                if let Err(error) = app.emit(APP_DEPTH_CONVERSION_MENU_EVENT, ()) {
                    log::warn!("failed to emit native depth-conversion menu event: {error}");
                }
            }
            APP_WELL_TIE_MENU_ID => {
                if let Err(error) = app.emit(APP_WELL_TIE_MENU_EVENT, ()) {
                    log::warn!("failed to emit native well-tie menu event: {error}");
                }
            }
            FILE_OPEN_VOLUME_MENU_ID => {
                if let Err(error) = app.emit(FILE_OPEN_VOLUME_MENU_EVENT, ()) {
                    log::warn!("failed to emit native open-volume menu event: {error}");
                }
            }
            FILE_IMPORT_DATA_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_DATA_MENU_EVENT, ()) {
                    log::warn!("failed to emit native import-data menu event: {error}");
                }
            }
            FILE_IMPORT_SEISMIC_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_SEISMIC_MENU_EVENT, ()) {
                    log::warn!("failed to emit native import-seismic menu event: {error}");
                }
            }
            FILE_IMPORT_HORIZONS_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_HORIZONS_MENU_EVENT, ()) {
                    log::warn!("failed to emit native import-horizons menu event: {error}");
                }
            }
            FILE_IMPORT_WELL_SOURCES_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_WELL_SOURCES_MENU_EVENT, ()) {
                    log::warn!("failed to emit native import-well-sources menu event: {error}");
                }
            }
            FILE_IMPORT_VELOCITY_FUNCTIONS_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_VELOCITY_FUNCTIONS_MENU_EVENT, ()) {
                    log::warn!(
                        "failed to emit native import-velocity-functions menu event: {error}"
                    );
                }
            }
            FILE_IMPORT_CHECKSHOT_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_CHECKSHOT_MENU_EVENT, ()) {
                    log::warn!("failed to emit native import-checkshot menu event: {error}");
                }
            }
            FILE_IMPORT_MANUAL_PICKS_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_MANUAL_PICKS_MENU_EVENT, ()) {
                    log::warn!("failed to emit native import-manual-picks menu event: {error}");
                }
            }
            FILE_IMPORT_AUTHORED_WELL_MODEL_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_AUTHORED_WELL_MODEL_MENU_EVENT, ()) {
                    log::warn!(
                        "failed to emit native import-authored-well-model menu event: {error}"
                    );
                }
            }
            FILE_IMPORT_COMPILED_WELL_MODEL_MENU_ID => {
                if let Err(error) = app.emit(FILE_IMPORT_COMPILED_WELL_MODEL_MENU_EVENT, ()) {
                    log::warn!(
                        "failed to emit native import-compiled-well-model menu event: {error}"
                    );
                }
            }
            _ => {}
        })
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_paths = AppPaths::resolve(&app.handle().clone())?;
            let diagnostics =
                DiagnosticsState::initialize(app_paths.logs_dir(), session_basename.clone())?;
            let processing = ProcessingState::initialize(app_paths.pipeline_presets_dir())?;
            let segy_import_recipes =
                SegyImportRecipeState::initialize(app_paths.segy_import_recipes_dir())?;
            fs::create_dir_all(app_paths.map_transform_cache_dir())
                .map_err(|error| error.to_string())?;
            let processing_cache = ProcessingCacheState::initialize(
                app_paths.processing_cache_dir(),
                app_paths.processing_cache_volumes_dir(),
                app_paths.processing_cache_index_path(),
                app_paths.settings_path(),
            )?;
            let preview_sessions = PreviewSessionState::default();
            let workspace = WorkspaceState::initialize(
                app_paths.dataset_registry_path(),
                app_paths.workspace_session_path(),
            )?;
            let import_manager = ImportManagerState::initialize();
            diagnostics.emit_session_event(
                &app.handle().clone(),
                "started",
                log::Level::Info,
                "Diagnostics session started",
                Some(build_fields([
                    (
                        "sessionLogPath",
                        json_value(diagnostics.session_log_path().display().to_string()),
                    ),
                    ("verboseEnabled", json_value(false)),
                ])),
            );
            app.manage(diagnostics);
            app.manage(processing);
            app.manage(segy_import_recipes);
            app.manage(processing_cache);
            app.manage(preview_sessions);
            app.manage(workspace);
            app.manage(import_manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_import_providers_command,
            begin_import_session_command,
            preflight_import_command,
            scan_segy_import_command,
            validate_segy_import_plan_command,
            import_segy_with_plan_command,
            import_dataset_command,
            import_prestack_offset_dataset_command,
            open_dataset_command,
            ensure_demo_survey_time_depth_transform_command,
            load_velocity_models_command,
            describe_velocity_volume_command,
            ingest_velocity_volume_command,
            load_horizon_assets_command,
            import_velocity_functions_model_command,
            build_velocity_model_transform_command,
            convert_horizon_domain_command,
            get_dataset_export_capabilities_command,
            export_dataset_segy_command,
            export_dataset_zarr_command,
            preview_horizon_xyz_import_command,
            preview_horizon_source_import_command,
            inspect_horizon_xyz_files_command,
            commit_horizon_source_import_command,
            import_horizon_xyz_command,
            load_section_horizons_command,
            load_section_command,
            load_section_binary_command,
            load_section_tile_binary_command,
            load_depth_converted_section_binary_command,
            load_resolved_section_display_binary_command,
            load_gather_command,
            preview_processing_command,
            preview_processing_binary_command,
            preview_subvolume_processing_command,
            preview_subvolume_processing_binary_command,
            preview_gather_processing_command,
            amplitude_spectrum_command,
            velocity_scan_command,
            run_processing_command,
            run_subvolume_processing_command,
            run_gather_processing_command,
            get_processing_job_command,
            cancel_processing_job_command,
            list_pipeline_presets_command,
            save_pipeline_preset_command,
            delete_pipeline_preset_command,
            list_segy_import_recipes_command,
            save_segy_import_recipe_command,
            delete_segy_import_recipe_command,
            load_workspace_state_command,
            upsert_dataset_entry_command,
            remove_dataset_entry_command,
            set_active_dataset_entry_command,
            save_workspace_session_command,
            load_project_geospatial_settings_command,
            save_project_geospatial_settings_command,
            search_coordinate_references_command,
            resolve_coordinate_reference_command,
            set_dataset_native_coordinate_reference_command,
            resolve_survey_map_command,
            resolve_project_survey_map_command,
            list_project_survey_horizons_command,
            list_project_well_overlay_inventory_command,
            list_project_well_marker_residual_inventory_command,
            scan_vendor_project_command,
            plan_vendor_project_import_command,
            commit_vendor_project_import_command,
            list_project_well_time_depth_models_command,
            list_project_well_time_depth_inventory_command,
            compute_project_well_marker_residual_command,
            set_project_active_well_time_depth_model_command,
            import_project_well_time_depth_model_command,
            preview_project_well_time_depth_asset_command,
            preview_project_well_time_depth_import_command,
            preview_project_well_sources_command,
            preview_project_well_import_command,
            commit_project_well_sources_command,
            commit_project_well_import_command,
            import_project_well_time_depth_asset_command,
            commit_project_well_time_depth_import_command,
            analyze_project_well_tie_command,
            accept_project_well_tie_command,
            compile_project_well_time_depth_authored_model_command,
            read_project_well_time_depth_model_command,
            resolve_project_section_well_overlays_command,
            default_import_store_path_command,
            default_import_prestack_store_path_command,
            default_processing_store_path_command,
            default_subvolume_processing_store_path_command,
            default_gather_processing_store_path_command,
            get_diagnostics_status_command,
            set_diagnostics_verbosity_command,
            export_diagnostics_bundle_command,
            emit_frontend_diagnostics_event_command,
            run_section_browsing_benchmark_command
        ]);

    #[cfg(debug_assertions)]
    let builder = if enable_devtools {
        builder.plugin(tauri_plugin_devtools::init())
    } else {
        builder.plugin(log_plugin)
    };

    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(log_plugin);

    builder
        .run(tauri::generate_context!())
        .expect("error while running traceboost desktop shell");
}

#[cfg(test)]
mod tests {
    use super::{
        PROJECT_SURVEY_DISPLAY_CRS_UNRESOLVED_REASON, PROJECT_SURVEY_DISPLAY_EQUIVALENT_REASON,
        PROJECT_WELLBORE_DISPLAY_CRS_UNRESOLVED_REASON, PROJECT_WELLBORE_DISPLAY_EQUIVALENT_REASON,
        ProjectDisplayCompatibilityBlockingReasonCode, ProjectSurveyDisplayReasonCode,
        ProjectWellboreDisplayCompatibility, ProjectWellboreDisplayReasonCode,
        best_shift_stretch_match, convolve_same_normalized, correlation_with_synthetic_lag,
        estimate_extracted_wavelet, interpolate_trace_sample, project_survey_blocking_reason_code,
        project_survey_display_compatibility, project_wellbore_blocking_reason_code,
        project_wellbore_display_compatibility, time_axis_midpoint_ms,
    };
    use ophiolite::{
        AssetBindingInput, CoordinateReferenceDescriptor, OphioliteProject, ProjectedPoint2,
        SurveyMapTransformStatusDto, WellAzimuthReferenceKind, WellboreAnchorKind,
        WellboreAnchorReference, WellboreGeometry,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_project_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "traceboost-demo-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    #[test]
    fn project_wellbore_display_compatibility_requires_project_display_crs() {
        let root = temp_project_root("project-wellbore-display-requires-crs");
        fs::create_dir_all(&root).unwrap();
        let project = OphioliteProject::create(&root).unwrap();

        let compatibility = project_wellbore_display_compatibility(&project, "wellbore-1", None);

        assert!(!compatibility.can_resolve_project_map);
        assert_eq!(
            compatibility.transform_status,
            SurveyMapTransformStatusDto::NativeOnly
        );
        assert_eq!(
            compatibility.reason.as_deref(),
            Some(PROJECT_WELLBORE_DISPLAY_CRS_UNRESOLVED_REASON)
        );
        assert_eq!(
            compatibility.reason_code,
            Some(ProjectWellboreDisplayReasonCode::ProjectDisplayCrsUnresolved)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_survey_display_compatibility_requires_project_display_crs() {
        let compatibility = project_survey_display_compatibility(Some("EPSG:26917"), None);

        assert!(!compatibility.can_resolve_project_map);
        assert_eq!(
            compatibility.transform_status,
            SurveyMapTransformStatusDto::NativeOnly
        );
        assert_eq!(
            compatibility.source_coordinate_reference_id.as_deref(),
            Some("EPSG:26917")
        );
        assert_eq!(compatibility.display_coordinate_reference_id, None);
        assert_eq!(
            compatibility.reason.as_deref(),
            Some(PROJECT_SURVEY_DISPLAY_CRS_UNRESOLVED_REASON)
        );
        assert!(matches!(
            compatibility.reason_code,
            Some(ProjectSurveyDisplayReasonCode::ProjectDisplayCrsUnresolved)
        ));
    }

    #[test]
    fn project_survey_display_compatibility_reports_equivalent_epsg_match() {
        let compatibility =
            project_survey_display_compatibility(Some("EPSG:26917"), Some("EPSG:26917"));

        assert!(compatibility.can_resolve_project_map);
        assert_eq!(
            compatibility.transform_status,
            SurveyMapTransformStatusDto::DisplayEquivalent
        );
        assert_eq!(
            compatibility.reason.as_deref(),
            Some(PROJECT_SURVEY_DISPLAY_EQUIVALENT_REASON)
        );
        assert!(matches!(
            compatibility.reason_code,
            Some(ProjectSurveyDisplayReasonCode::DisplayEquivalent)
        ));
    }

    #[test]
    fn project_survey_display_compatibility_reports_transformed_epsg_path() {
        let compatibility =
            project_survey_display_compatibility(Some("EPSG:26917"), Some("EPSG:4326"));

        assert!(compatibility.can_resolve_project_map);
        assert_eq!(
            compatibility.transform_status,
            SurveyMapTransformStatusDto::DisplayTransformed
        );
        assert_eq!(
            compatibility.reason.as_deref(),
            Some("Survey map geometry can be reprojected from EPSG:26917 to EPSG:4326.")
        );
        assert!(matches!(
            compatibility.reason_code,
            Some(ProjectSurveyDisplayReasonCode::DisplayTransformed)
        ));
    }

    #[test]
    fn project_wellbore_display_compatibility_uses_resolved_well_geometry() {
        let root = temp_project_root("project-wellbore-display-uses-resolved-geometry");
        let csv_path = root.join("trajectory.csv");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &csv_path,
            "md,tvd,northing,easting,inclination,azimuth\n0,0,0,0,0,0\n100,90,20,10,12,45\n",
        )
        .unwrap();

        let mut project = OphioliteProject::create(&root).unwrap();
        let binding = AssetBindingInput {
            well_name: "Well A".to_string(),
            wellbore_name: "Well A".to_string(),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        };
        let import = project
            .import_trajectory_csv(&csv_path, &binding, Some("trajectory"))
            .unwrap();
        let geometry = WellboreGeometry {
            anchor: Some(WellboreAnchorReference {
                kind: WellboreAnchorKind::Surface,
                coordinate_reference: Some(CoordinateReferenceDescriptor {
                    id: Some("EPSG:23031".to_string()),
                    name: Some("ED50 / UTM zone 31N".to_string()),
                    geodetic_datum: Some("ED50".to_string()),
                    unit: Some("m".to_string()),
                }),
                location: ProjectedPoint2 {
                    x: 500_000.0,
                    y: 6_200_000.0,
                },
                parent_wellbore_id: None,
                parent_measured_depth_m: None,
                notes: Vec::new(),
            }),
            vertical_datum: Some("KB".to_string()),
            depth_unit: Some("m".to_string()),
            azimuth_reference: WellAzimuthReferenceKind::GridNorth,
            notes: Vec::new(),
        };
        project
            .set_wellbore_geometry(&import.resolution.wellbore_id, Some(geometry))
            .unwrap();

        let compatibility = project_wellbore_display_compatibility(
            &project,
            &import.resolution.wellbore_id.0,
            Some("EPSG:23031"),
        );

        assert_eq!(
            compatibility,
            ProjectWellboreDisplayCompatibility {
                can_resolve_project_map: true,
                transform_status: SurveyMapTransformStatusDto::DisplayEquivalent,
                source_coordinate_reference_id: Some("EPSG:23031".to_string()),
                display_coordinate_reference_id: Some("EPSG:23031".to_string()),
                reason_code: Some(ProjectWellboreDisplayReasonCode::DisplayEquivalent),
                reason: Some(String::from(PROJECT_WELLBORE_DISPLAY_EQUIVALENT_REASON)),
            }
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_display_compatibility_blocking_reason_codes_only_cover_blocking_states() {
        assert_eq!(
            project_survey_blocking_reason_code(
                ProjectSurveyDisplayReasonCode::ProjectDisplayCrsUnresolved
            ),
            Some(ProjectDisplayCompatibilityBlockingReasonCode::ProjectDisplayCrsUnresolved)
        );
        assert_eq!(
            project_survey_blocking_reason_code(ProjectSurveyDisplayReasonCode::DisplayEquivalent),
            None
        );
        assert_eq!(
            project_wellbore_blocking_reason_code(
                ProjectWellboreDisplayReasonCode::ResolvedGeometryMissing
            ),
            Some(ProjectDisplayCompatibilityBlockingReasonCode::ResolvedGeometryMissing)
        );
        assert_eq!(
            project_wellbore_blocking_reason_code(
                ProjectWellboreDisplayReasonCode::DisplayDegraded
            ),
            None
        );
    }

    #[test]
    fn best_shift_stretch_match_prefers_material_affine_improvement() {
        let times_ms = (0..160).map(|index| index as f32 * 4.0).collect::<Vec<_>>();
        let midpoint_ms = time_axis_midpoint_ms(&times_ms).unwrap();
        let synthetic = times_ms
            .iter()
            .map(|time_ms| {
                ((time_ms / 18.0).sin()
                    + 0.35 * (time_ms / 31.0).cos()
                    + 0.2 * (time_ms / 11.0).sin()) as f32
            })
            .collect::<Vec<_>>();
        let expected_shift_ms = 12.0_f32;
        let expected_stretch = 1.02_f32;
        let seismic = times_ms
            .iter()
            .map(|time_ms| {
                let warped_time_ms =
                    midpoint_ms + expected_stretch * (*time_ms - midpoint_ms) + expected_shift_ms;
                interpolate_trace_sample(&times_ms, &synthetic, warped_time_ms)
            })
            .collect::<Vec<_>>();

        let solved = best_shift_stretch_match(&times_ms, &synthetic, &seismic, 6, 4.0)
            .expect("solver should recover a usable affine match");
        let aligned_correlation =
            correlation_with_synthetic_lag(&synthetic, &solved.aligned_trace_amplitudes, 0)
                .expect("aligned correlation");

        assert!(aligned_correlation > 0.995, "{aligned_correlation}");
        assert_eq!(solved.bulk_shift_samples, 3);
        assert!((solved.stretch_factor - expected_stretch).abs() <= 0.01);
        assert!(solved.correlation > 0.99);
    }

    #[test]
    fn estimate_extracted_wavelet_recovers_known_wavelet_shape() {
        let reflectivity = (0..192)
            .map(|index| {
                let index = index as f32;
                (index / 9.0).sin() * 0.7 + (index / 17.0).cos() * 0.3
            })
            .collect::<Vec<_>>();
        let mut true_wavelet = vec![
            -0.12_f32, -0.28, -0.05, 0.62, 1.0, 0.62, -0.05, -0.28, -0.12,
        ];
        super::normalize_trace_in_place(&mut true_wavelet);
        let seismic = convolve_same_normalized(&reflectivity, &true_wavelet);

        let extracted = estimate_extracted_wavelet(
            &reflectivity,
            &seismic,
            4.0,
            reflectivity.len() as f32 * 4.0,
        )
        .expect("wavelet estimate should succeed");

        let recovered = extracted
            .amplitudes
            .iter()
            .skip(
                (extracted
                    .amplitudes
                    .len()
                    .saturating_sub(true_wavelet.len()))
                    / 2,
            )
            .take(true_wavelet.len())
            .copied()
            .collect::<Vec<_>>();
        let correlation = correlation_with_synthetic_lag(&true_wavelet, &recovered, 0)
            .expect("correlation should be defined");

        assert!(correlation > 0.9, "{correlation}");
    }
}
