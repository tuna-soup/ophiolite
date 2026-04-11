use ophiolite_seismic::{DepthReferenceKind, SectionAxis, TravelTimeReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const WELL_PANEL_CONTRACT_VERSION: u32 = 1;
pub const SURVEY_MAP_CONTRACT_VERSION: u32 = 2;
pub const SECTION_WELL_OVERLAY_CONTRACT_VERSION: u32 = 1;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_subsea: Option<f64>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapRequestDto {
    pub schema_version: u32,
    pub survey_asset_ids: Vec<String>,
    pub wellbore_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_coordinate_reference_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CoordinateReferenceDto {
    pub id: Option<String>,
    pub name: Option<String>,
    pub geodetic_datum: Option<String>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CoordinateReferenceSourceDto {
    Header,
    ImportManifest,
    UserOverride,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CoordinateReferenceBindingDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected: Option<CoordinateReferenceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective: Option<CoordinateReferenceDto>,
    pub source: CoordinateReferenceSourceDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProjectedPoint2Dto {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProjectedVector2Dto {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProjectedPolygon2Dto {
    pub exterior: Vec<ProjectedPoint2Dto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapGridTransformDto {
    pub origin: ProjectedPoint2Dto,
    pub inline_basis: ProjectedVector2Dto,
    pub xline_basis: ProjectedVector2Dto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SurveyMapSpatialAvailabilityDto {
    Available,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapSpatialDescriptorDto {
    pub coordinate_reference: Option<CoordinateReferenceDto>,
    pub grid_transform: Option<SurveyMapGridTransformDto>,
    pub footprint: Option<ProjectedPolygon2Dto>,
    pub availability: SurveyMapSpatialAvailabilityDto,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapScalarFieldDto {
    pub id: String,
    pub name: String,
    pub columns: usize,
    pub rows: usize,
    pub values: Vec<f32>,
    pub origin: ProjectedPoint2Dto,
    pub step: ProjectedPoint2Dto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SurveyMapTransformStatusDto {
    NativeOnly,
    DisplayEquivalent,
    DisplayTransformed,
    DisplayDegraded,
    DisplayUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SurveyMapTransformPolicyDto {
    BestAvailable,
    BestOrFail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapTransformDiagnosticsDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_coordinate_reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_coordinate_reference_id: Option<String>,
    pub policy: SurveyMapTransformPolicyDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accuracy_meters: Option<f64>,
    pub degraded: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyIndexAxisDto {
    pub count: usize,
    pub first: i32,
    pub last: i32,
    pub step: Option<i32>,
    pub regular: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyIndexGridDto {
    pub inline_axis: SurveyIndexAxisDto,
    pub xline_axis: SurveyIndexAxisDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedSurveyMapSurveyDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub name: String,
    pub index_grid: SurveyIndexGridDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_binding: Option<CoordinateReferenceBindingDto>,
    pub native_spatial: SurveyMapSpatialDescriptorDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_spatial: Option<SurveyMapSpatialDescriptorDto>,
    pub transform_status: SurveyMapTransformStatusDto,
    pub transform_diagnostics: SurveyMapTransformDiagnosticsDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapTrajectoryStationDto {
    pub measured_depth: f64,
    pub true_vertical_depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_subsea: Option<f64>,
    pub azimuth_deg: Option<f64>,
    pub inclination_deg: Option<f64>,
    pub northing_offset: Option<f64>,
    pub easting_offset: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyMapTrajectoryDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    pub rows: Vec<SurveyMapTrajectoryStationDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedSurveyMapWellDto {
    pub well_id: String,
    pub wellbore_id: String,
    pub name: String,
    pub coordinate_reference: Option<CoordinateReferenceDto>,
    pub surface_location: Option<ProjectedPoint2Dto>,
    pub trajectories: Vec<SurveyMapTrajectoryDto>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedSurveyMapHorizonDto {
    pub id: String,
    pub survey_asset_id: String,
    pub name: String,
    pub source_path: String,
    pub point_count: usize,
    pub mapped_point_count: usize,
    pub missing_cell_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_coordinate_reference: Option<CoordinateReferenceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aligned_coordinate_reference: Option<CoordinateReferenceDto>,
    #[serde(default)]
    pub transformed: bool,
    pub preview_available: bool,
    pub preview_status: SurveyMapTransformStatusDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedSurveyMapSourceDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub surveys: Vec<ResolvedSurveyMapSurveyDto>,
    pub wells: Vec<ResolvedSurveyMapWellDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub horizons: Vec<ResolvedSurveyMapHorizonDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scalar_field: Option<SurveyMapScalarFieldDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scalar_field_horizon_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionWellOverlayDomainDto {
    Depth,
    Time,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionWellOverlayRequestDto {
    pub schema_version: u32,
    pub project_root: String,
    pub survey_asset_id: String,
    pub wellbore_ids: Vec<String>,
    pub axis: SectionAxis,
    pub index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_m: Option<f64>,
    pub display_domain: SectionWellOverlayDomainDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_well_model_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionWellOverlaySampleDto {
    pub trace_index: usize,
    pub trace_coordinate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_value: Option<f64>,
    pub x: f64,
    pub y: f64,
    pub measured_depth_m: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_subsea_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twt_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionWellOverlaySegmentDto {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<SectionWellOverlaySampleDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedSectionWellOverlayDto {
    pub well_id: String,
    pub wellbore_id: String,
    pub name: String,
    pub display_domain: SectionWellOverlayDomainDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<SectionWellOverlaySegmentDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_reference: Option<DepthReferenceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub travel_time_reference: Option<TravelTimeReference>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolveSectionWellOverlaysResponse {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<ResolvedSectionWellOverlayDto>,
}
