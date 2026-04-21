use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::import_ops::SegyGeometryOverride;
pub use ophiolite_seismic::{
    DatasetSummary, SectionAxis, SubvolumeCropOperation, SurveyTimeDepthTransform3D,
    TimeDepthDomain, VelocityQuantityKind, VelocitySource3D,
};
use seis_contracts_core::TraceLocalProcessingPipeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DatasetRegistryStatus {
    Linked,
    Imported,
    MissingSource,
    MissingStore,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct WorkspacePipelineEntry {
    pub pipeline_id: String,
    pub pipeline: TraceLocalProcessingPipeline,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subvolume_crop: Option<SubvolumeCropOperation>,
    #[ts(type = "number")]
    pub updated_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct DatasetRegistryEntry {
    pub entry_id: String,
    pub display_name: String,
    pub source_path: Option<String>,
    pub preferred_store_path: Option<String>,
    pub imported_store_path: Option<String>,
    pub last_dataset: Option<DatasetSummary>,
    #[serde(default)]
    pub session_pipelines: Vec<WorkspacePipelineEntry>,
    #[serde(default)]
    pub active_session_pipeline_id: Option<String>,
    pub status: DatasetRegistryStatus,
    pub last_opened_at_unix_s: Option<u64>,
    pub last_imported_at_unix_s: Option<u64>,
    pub updated_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct WorkspaceSession {
    pub active_entry_id: Option<String>,
    pub active_store_path: Option<String>,
    pub active_axis: SectionAxis,
    pub active_index: usize,
    pub selected_preset_id: Option<String>,
    pub display_coordinate_reference_id: Option<String>,
    pub active_velocity_model_asset_id: Option<String>,
    pub project_root: Option<String>,
    pub project_survey_asset_id: Option<String>,
    pub project_wellbore_id: Option<String>,
    pub project_section_tolerance_m: Option<f64>,
    pub selected_project_well_time_depth_model_asset_id: Option<String>,
    #[serde(default)]
    pub native_engineering_accepted_store_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct LoadWorkspaceStateResponse {
    pub schema_version: u32,
    pub entries: Vec<DatasetRegistryEntry>,
    pub session: WorkspaceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct UpsertDatasetEntryRequest {
    pub schema_version: u32,
    pub entry_id: Option<String>,
    pub display_name: Option<String>,
    pub source_path: Option<String>,
    pub preferred_store_path: Option<String>,
    pub imported_store_path: Option<String>,
    pub dataset: Option<DatasetSummary>,
    #[serde(default)]
    pub session_pipelines: Option<Vec<WorkspacePipelineEntry>>,
    #[serde(default)]
    pub active_session_pipeline_id: Option<String>,
    pub make_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct UpsertDatasetEntryResponse {
    pub schema_version: u32,
    pub entry: DatasetRegistryEntry,
    pub session: WorkspaceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct RemoveDatasetEntryRequest {
    pub schema_version: u32,
    pub entry_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct RemoveDatasetEntryResponse {
    pub schema_version: u32,
    pub deleted: bool,
    pub session: WorkspaceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SetActiveDatasetEntryRequest {
    pub schema_version: u32,
    pub entry_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SetActiveDatasetEntryResponse {
    pub schema_version: u32,
    pub entry: DatasetRegistryEntry,
    pub session: WorkspaceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SaveWorkspaceSessionRequest {
    pub schema_version: u32,
    pub active_entry_id: Option<String>,
    pub active_store_path: Option<String>,
    pub active_axis: SectionAxis,
    pub active_index: usize,
    pub selected_preset_id: Option<String>,
    pub display_coordinate_reference_id: Option<String>,
    pub active_velocity_model_asset_id: Option<String>,
    pub project_root: Option<String>,
    pub project_survey_asset_id: Option<String>,
    pub project_wellbore_id: Option<String>,
    pub project_section_tolerance_m: Option<f64>,
    pub selected_project_well_time_depth_model_asset_id: Option<String>,
    #[serde(default)]
    pub native_engineering_accepted_store_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SaveWorkspaceSessionResponse {
    pub schema_version: u32,
    pub session: WorkspaceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct LoadVelocityModelsRequest {
    pub schema_version: u32,
    pub store_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct LoadVelocityModelsResponse {
    pub schema_version: u32,
    pub models: Vec<SurveyTimeDepthTransform3D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct DescribeVelocityVolumeRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub velocity_kind: VelocityQuantityKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_domain: Option<TimeDepthDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_step: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct DescribeVelocityVolumeResponse {
    pub schema_version: u32,
    pub volume: VelocitySource3D,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct IngestVelocityVolumeRequest {
    pub schema_version: u32,
    pub input_path: String,
    pub output_store_path: String,
    pub velocity_kind: VelocityQuantityKind,
    pub vertical_domain: TimeDepthDomain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_step: Option<f32>,
    #[serde(default)]
    pub overwrite_existing: bool,
    #[serde(default)]
    pub delete_input_on_success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_override: Option<SegyGeometryOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct IngestVelocityVolumeResponse {
    pub schema_version: u32,
    pub input_path: String,
    pub store_path: String,
    pub deleted_input: bool,
    pub volume: VelocitySource3D,
}
