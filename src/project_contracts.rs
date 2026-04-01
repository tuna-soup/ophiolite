use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const WELL_PANEL_CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelRequestDto {
    pub schema_version: u32,
    pub wellbore_ids: Vec<String>,
    pub depth_min: Option<f64>,
    pub depth_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelDepthSampleDto {
    pub native_depth: f64,
    pub panel_depth: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelLogCurveDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    pub curve_name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub semantic_type: String,
    pub depths: Vec<f64>,
    pub values: Vec<Option<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelTrajectoryRowDto {
    pub measured_depth: f64,
    pub true_vertical_depth: Option<f64>,
    pub azimuth_deg: Option<f64>,
    pub inclination_deg: Option<f64>,
    pub northing_offset: Option<f64>,
    pub easting_offset: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelTrajectoryDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    pub rows: Vec<WellPanelTrajectoryRowDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelTopRowDto {
    pub name: String,
    pub top_depth: f64,
    pub base_depth: Option<f64>,
    pub source: Option<String>,
    pub depth_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelTopSetDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    pub rows: Vec<WellPanelTopRowDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelPressureObservationDto {
    pub measured_depth: Option<f64>,
    pub pressure: f64,
    pub phase: Option<String>,
    pub test_kind: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelPressureSetDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    pub rows: Vec<WellPanelPressureObservationDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelDrillingObservationDto {
    pub measured_depth: Option<f64>,
    pub event_kind: String,
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub timestamp: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelDrillingSetDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    pub rows: Vec<WellPanelDrillingObservationDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedWellPanelWellDto {
    pub well_id: String,
    pub wellbore_id: String,
    pub name: String,
    pub native_depth_datum: String,
    pub panel_depth_mapping: Vec<WellPanelDepthSampleDto>,
    pub logs: Vec<WellPanelLogCurveDto>,
    pub trajectories: Vec<WellPanelTrajectoryDto>,
    pub top_sets: Vec<WellPanelTopSetDto>,
    pub pressure_observations: Vec<WellPanelPressureSetDto>,
    pub drilling_observations: Vec<WellPanelDrillingSetDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedWellPanelSourceDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub wells: Vec<ResolvedWellPanelWellDto>,
}
