use ophiolite_seismic::{DepthReferenceKind, SectionAxis, TravelTimeReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const WELL_PANEL_CONTRACT_VERSION: u32 = 1;
pub const SURVEY_MAP_CONTRACT_VERSION: u32 = 2;
pub const SECTION_WELL_OVERLAY_CONTRACT_VERSION: u32 = 1;
pub const WELL_MARKER_HORIZON_RESIDUAL_CONTRACT_VERSION: u32 = 1;
pub const ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION: u32 = 1;
pub const AVO_ANALYSIS_CONTRACT_VERSION: u32 = 1;

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
    pub log_type: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depth_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_datum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellPanelTopSetDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub asset_name: String,
    #[serde(default = "default_well_panel_top_set_kind")]
    pub set_kind: String,
    pub rows: Vec<WellPanelTopRowDto>,
}

fn default_well_panel_top_set_kind() -> String {
    "top_set".to_string()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum RockPhysicsTemplateIdDto {
    VpVsVsAi,
    AiVsSi,
    VpVsVs,
    PorosityVsVp,
    LambdaRhoVsMuRho,
    NeutronPorosityVsBulkDensity,
    PhiVsAi,
    PrVsAi,
    VpVsDensity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum RockPhysicsCurveSemanticDto {
    PVelocity,
    SVelocity,
    VpVsRatio,
    AcousticImpedance,
    ElasticImpedance,
    ExtendedElasticImpedance,
    ShearImpedance,
    LambdaRho,
    MuRho,
    BulkDensity,
    Resistivity,
    Sonic,
    ShearSonic,
    PoissonsRatio,
    NeutronPorosity,
    EffectivePorosity,
    WaterSaturation,
    VShale,
    GammaRay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RockPhysicsCategoricalSemanticDto {
    Well,
    Wellbore,
    Facies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RockPhysicsPointSymbolDto {
    Circle,
    Square,
    Diamond,
    Triangle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsAxisDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub semantic: RockPhysicsCurveSemanticDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsCategoryDto {
    pub id: u32,
    pub label: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<RockPhysicsPointSymbolDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsCategoricalColorBindingDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub semantic: RockPhysicsCategoricalSemanticDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<RockPhysicsCategoryDto>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsContinuousColorBindingDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub semantic: RockPhysicsCurveSemanticDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(tag = "kind", rename_all = "snake_case")]
pub enum RockPhysicsColorBindingDto {
    Categorical(RockPhysicsCategoricalColorBindingDto),
    Continuous(RockPhysicsContinuousColorBindingDto),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsWellDto {
    pub well_id: String,
    pub wellbore_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsSourceBindingDto {
    pub id: String,
    pub well_id: String,
    pub wellbore_id: String,
    pub x_curve_id: String,
    pub y_curve_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_curve_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_channels: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsSampleDto {
    pub well_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wellbore_id: Option<String>,
    pub sample_depth_m: f64,
    pub x_value: f64,
    pub y_value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_category_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_category_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_binding_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsInteractionThresholdsDto {
    pub exact_point_limit: u32,
    pub progressive_point_limit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsTemplatePointDto {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsTemplateLineDto {
    pub id: String,
    pub label: String,
    pub color: String,
    pub points: Vec<RockPhysicsTemplatePointDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RockPhysicsTextAlignDto {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RockPhysicsTextBaselineDto {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsTemplatePolylineOverlayDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashed: Option<bool>,
    pub points: Vec<RockPhysicsTemplatePointDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsTemplatePolygonOverlayDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    pub fill_color: String,
    pub points: Vec<RockPhysicsTemplatePointDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_position: Option<RockPhysicsTemplatePointDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsTemplateTextOverlayDto {
    pub id: String,
    pub text: String,
    pub color: String,
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<RockPhysicsTextAlignDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<RockPhysicsTextBaselineDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(tag = "kind", rename_all = "snake_case")]
pub enum RockPhysicsTemplateOverlayDto {
    Polyline(RockPhysicsTemplatePolylineOverlayDto),
    Polygon(RockPhysicsTemplatePolygonOverlayDto),
    Text(RockPhysicsTemplateTextOverlayDto),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedRockPhysicsCrossplotSourceDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub template_id: RockPhysicsTemplateIdDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub x_axis: RockPhysicsAxisDto,
    pub y_axis: RockPhysicsAxisDto,
    pub color_binding: RockPhysicsColorBindingDto,
    pub wells: Vec<RockPhysicsWellDto>,
    pub samples: Vec<RockPhysicsSampleDto>,
    pub source_bindings: Vec<RockPhysicsSourceBindingDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_lines: Option<Vec<RockPhysicsTemplateLineDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_overlays: Option<Vec<RockPhysicsTemplateOverlayDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_thresholds: Option<RockPhysicsInteractionThresholdsDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsCategoricalColorRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub semantic: RockPhysicsCategoricalSemanticDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsContinuousColorRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub semantic: RockPhysicsCurveSemanticDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(tag = "kind", rename_all = "snake_case")]
pub enum RockPhysicsColorRequestDto {
    Categorical(RockPhysicsCategoricalColorRequestDto),
    Continuous(RockPhysicsContinuousColorRequestDto),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsCrossplotRequestDto {
    pub schema_version: u32,
    pub wellbore_ids: Vec<String>,
    pub template_id: RockPhysicsTemplateIdDto,
    pub x_semantic: RockPhysicsCurveSemanticDto,
    pub y_semantic: RockPhysicsCurveSemanticDto,
    pub color_binding: RockPhysicsColorRequestDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AvoReflectivityModelDto {
    ShueyTwoTerm,
    ShueyThreeTerm,
    AkiRichards,
    AkiRichardsAlt,
    Fatti,
    Bortfeld,
    Hilterman,
    ApproxZoeppritzPp,
    Zoeppritz,
    Ruger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AvoAnisotropyModeDto {
    Isotropic,
    Vti,
    Hti,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AvoCurveStyleDto {
    Solid,
    Dashed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoAxisDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoInterfaceDto {
    pub id: String,
    pub label: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reservoir_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoResponseSeriesDto {
    pub id: String,
    pub interface_id: String,
    pub label: String,
    pub color: String,
    pub style: AvoCurveStyleDto,
    pub reflectivity_model: AvoReflectivityModelDto,
    pub anisotropy_mode: AvoAnisotropyModeDto,
    pub incidence_angles_deg: Vec<f64>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedAvoResponseSourceDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub x_axis: AvoAxisDto,
    pub y_axis: AvoAxisDto,
    pub interfaces: Vec<AvoInterfaceDto>,
    pub series: Vec<AvoResponseSeriesDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoCrossplotPointDto {
    pub interface_id: String,
    pub intercept: f64,
    pub gradient: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chi_projection: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulation_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoReferenceLineDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub color: String,
    pub style: AvoCurveStyleDto,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoBackgroundRegionDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub fill_color: String,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedAvoCrossplotSourceDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub x_axis: AvoAxisDto,
    pub y_axis: AvoAxisDto,
    pub interfaces: Vec<AvoInterfaceDto>,
    pub points: Vec<AvoCrossplotPointDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_lines: Option<Vec<AvoReferenceLineDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_regions: Option<Vec<AvoBackgroundRegionDto>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoChiProjectionSeriesDto {
    pub id: String,
    pub interface_id: String,
    pub label: String,
    pub color: String,
    pub projected_values: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedAvoChiProjectionSourceDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub chi_angle_deg: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_label: Option<String>,
    pub x_axis: AvoAxisDto,
    pub interfaces: Vec<AvoInterfaceDto>,
    pub series: Vec<AvoChiProjectionSeriesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_bin_count: Option<u32>,
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
pub struct ProjectSurveyMapRequestDto {
    pub schema_version: u32,
    pub survey_asset_ids: Vec<String>,
    pub wellbore_ids: Vec<String>,
    pub display_coordinate_reference_id: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellMarkerHorizonResidualRequestDto {
    pub schema_version: u32,
    pub source_asset_id: String,
    pub survey_asset_id: String,
    pub horizon_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellMarkerHorizonResidualRowDto {
    pub marker_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_kind: Option<String>,
    pub source_depth: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depth_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depth_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depth_datum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured_depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_subsea: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon_depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon_inline_ordinal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon_xline_ordinal: Option<f64>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedWellMarkerHorizonResidualSourceDto {
    pub schema_version: u32,
    pub source_asset_id: String,
    pub source_asset_kind: String,
    pub survey_asset_id: String,
    pub horizon_id: String,
    pub horizon_name: String,
    pub well_id: String,
    pub wellbore_id: String,
    pub well_name: String,
    pub wellbore_name: String,
    pub residual_sign_convention: String,
    pub sampling_method: String,
    pub depth_reference_used: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trajectory_asset_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<WellMarkerHorizonResidualRowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}
