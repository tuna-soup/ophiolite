use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use ophiolite_seismic::{
    BuildSurveyTimeDepthTransformRequest, CoordinateReferenceBindingDto, CoordinateReferenceDto,
    CoordinateReferenceSourceDto, DatasetSummary, DepthReferenceKind, IPC_SCHEMA_VERSION,
    LateralInterpolationMethod, LayeredVelocityInterval, LayeredVelocityModel, ProjectedPoint2Dto,
    ProjectedPolygon2Dto, ProjectedVector2Dto, ResolvedSurveyMapHorizonDto,
    ResolvedSurveyMapSourceDto, ResolvedSurveyMapSurveyDto, ResolvedSurveyMapWellDto,
    StratigraphicBoundaryReference, SurveyIndexAxisDto, SurveyIndexGridDto,
    SurveyMapGridTransformDto, SurveyMapScalarFieldDto, SurveyMapSpatialAvailabilityDto,
    SurveyMapSpatialDescriptorDto, SurveyMapTrajectoryDto, SurveyMapTrajectoryStationDto,
    SurveyMapTransformDiagnosticsDto, SurveyMapTransformPolicyDto, SurveyMapTransformStatusDto,
    SurveyTimeDepthTransform3D, TimeDepthDomain, TravelTimeReference, VerticalInterpolationMethod,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SetDatasetNativeCoordinateReferenceRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub coordinate_reference_id: Option<String>,
    pub coordinate_reference_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SetDatasetNativeCoordinateReferenceResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct ResolveSurveyMapRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub display_coordinate_reference_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct ResolveSurveyMapResponse {
    pub schema_version: u32,
    pub survey_map: ResolvedSurveyMapSourceDto,
}
