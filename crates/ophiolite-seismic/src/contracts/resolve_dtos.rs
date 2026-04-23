use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
    pub transform_status: SurveyMapTransformStatusDto,
    pub transform_diagnostics: SurveyMapTransformDiagnosticsDto,
    pub surface_location: Option<ProjectedPoint2Dto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_trajectory: Vec<ProjectedPoint2Dto>,
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
