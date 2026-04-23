use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::default_pipeline_schema_version;
use super::domain::{CoordinateReferenceDescriptor, ProjectedPoint2, SurveyGridTransform};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum VelocityFunctionSource {
    ConstantVelocity {
        velocity_m_per_s: f32,
    },
    TimeVelocityPairs {
        times_ms: Vec<f32>,
        velocities_m_per_s: Vec<f32>,
    },
    VelocityAssetReference {
        asset_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TimeDepthDomain {
    Time,
    Depth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TimeDepthTransformSourceKind {
    ConstantVelocity,
    VelocityFunction1D,
    VelocityGrid3D,
    CheckshotModel1D,
    SonicLog1D,
    VpLog1D,
    HorizonLayerModel,
    WellTieObservationSet1D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum VelocityQuantityKind {
    Interval,
    Rms,
    Average,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TravelTimeReference {
    OneWay,
    TwoWay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DepthReferenceKind {
    MeasuredDepth,
    TrueVerticalDepth,
    TrueVerticalDepthSubsea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum WellboreAnchorKind {
    Surface,
    ParentTieOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum WellAzimuthReferenceKind {
    TrueNorth,
    GridNorth,
    MagneticNorth,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellboreAnchorReference {
    pub kind: WellboreAnchorKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub location: ProjectedPoint2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_wellbore_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_measured_depth_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellboreGeometry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<WellboreAnchorReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_datum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_unit: Option<String>,
    pub azimuth_reference: WellAzimuthReferenceKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TrajectoryInputSchemaKind {
    MdIncAzi,
    MdTvdIncAzi,
    MdTvdssIncAzi,
    MdOffsetTvd,
    MdOffsetTvdss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TrajectoryValueOrigin {
    Imported,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedTrajectoryStation {
    pub measured_depth_m: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_subsea_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub northing_offset_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easting_offset_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute_xy: Option<ProjectedPoint2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inclination_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azimuth_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_origin: Option<TrajectoryValueOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_vertical_depth_subsea_origin: Option<TrajectoryValueOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub northing_offset_origin: Option<TrajectoryValueOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easting_offset_origin: Option<TrajectoryValueOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inclination_origin: Option<TrajectoryValueOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azimuth_origin: Option<TrajectoryValueOrigin>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedTrajectoryGeometry {
    pub id: String,
    pub wellbore_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_asset_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stations: Vec<ResolvedTrajectoryStation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SpatialCoverageRelationship {
    Exact,
    Contains,
    PartialOverlap,
    Disjoint,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VerticalAxisDescriptor {
    pub domain: TimeDepthDomain,
    pub unit: String,
    pub start: f32,
    pub step: f32,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SpatialCoverageSummary {
    pub relationship: SpatialCoverageRelationship,
    #[serde(default)]
    pub source_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub target_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocitySource3D {
    pub id: String,
    pub name: String,
    pub source_kind: TimeDepthTransformSourceKind,
    pub velocity_kind: VelocityQuantityKind,
    pub vertical_domain: TimeDepthDomain,
    pub velocity_unit: String,
    #[serde(default)]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub grid_transform: Option<SurveyGridTransform>,
    pub vertical_axis: VerticalAxisDescriptor,
    pub inline_count: usize,
    pub xline_count: usize,
    pub coverage: SpatialCoverageSummary,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyPropertyField3D {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub derived_from: Vec<String>,
    pub property_name: String,
    pub property_unit: String,
    #[serde(default)]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub grid_transform: Option<SurveyGridTransform>,
    pub vertical_axis: VerticalAxisDescriptor,
    pub inline_count: usize,
    pub xline_count: usize,
    pub sample_count: usize,
    pub coverage: SpatialCoverageSummary,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TimeDepthSample1D {
    pub time_ms: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTimeDepthObservationSample {
    pub depth_m: f64,
    pub time_ms: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CheckshotVspObservationSet1D {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub wellbore_id: Option<String>,
    pub depth_reference: DepthReferenceKind,
    pub travel_time_reference: TravelTimeReference,
    #[serde(default)]
    pub samples: Vec<WellTimeDepthObservationSample>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ManualTimeDepthPickSet1D {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub wellbore_id: Option<String>,
    pub depth_reference: DepthReferenceKind,
    pub travel_time_reference: TravelTimeReference,
    #[serde(default)]
    pub samples: Vec<WellTimeDepthObservationSample>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieObservationSet1D {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub wellbore_id: Option<String>,
    pub depth_reference: DepthReferenceKind,
    pub travel_time_reference: TravelTimeReference,
    #[serde(default)]
    pub samples: Vec<WellTimeDepthObservationSample>,
    #[serde(default)]
    pub source_well_time_depth_model_asset_id: Option<String>,
    #[serde(default)]
    pub tie_window_start_ms: Option<f64>,
    #[serde(default)]
    pub tie_window_end_ms: Option<f64>,
    #[serde(default)]
    pub trace_search_radius_m: Option<f32>,
    #[serde(default)]
    pub bulk_shift_ms: Option<f32>,
    #[serde(default)]
    pub stretch_factor: Option<f32>,
    #[serde(default)]
    pub trace_search_offset_m: Option<f32>,
    #[serde(default)]
    pub correlation: Option<f32>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTimeDepthSourceBinding {
    pub source_kind: TimeDepthTransformSourceKind,
    pub asset_id: String,
    pub enabled: bool,
    pub priority: u32,
    #[serde(default)]
    pub valid_from_depth_m: Option<f64>,
    #[serde(default)]
    pub valid_to_depth_m: Option<f64>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum WellTimeDepthAssumptionKind {
    ConstantVelocity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTimeDepthAssumptionInterval {
    #[serde(default)]
    pub from_depth_m: Option<f64>,
    #[serde(default)]
    pub to_depth_m: Option<f64>,
    pub kind: WellTimeDepthAssumptionKind,
    #[serde(default)]
    pub velocity_m_per_s: Option<f64>,
    #[serde(default)]
    pub overwrite_existing_source_coverage: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTimeDepthAuthoredModel1D {
    pub id: String,
    pub name: String,
    pub wellbore_id: String,
    pub resolved_trajectory_fingerprint: String,
    pub depth_reference: DepthReferenceKind,
    pub travel_time_reference: TravelTimeReference,
    #[serde(default)]
    pub source_bindings: Vec<WellTimeDepthSourceBinding>,
    #[serde(default)]
    pub assumption_intervals: Vec<WellTimeDepthAssumptionInterval>,
    #[serde(default)]
    pub sampling_step_m: Option<f64>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CompiledWellTimeDepthLineage {
    pub authored_model_id: String,
    pub resolved_trajectory_fingerprint: String,
    #[serde(default)]
    pub source_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTimeDepthModel1D {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub wellbore_id: Option<String>,
    pub source_kind: TimeDepthTransformSourceKind,
    pub depth_reference: DepthReferenceKind,
    pub travel_time_reference: TravelTimeReference,
    #[serde(default)]
    pub samples: Vec<TimeDepthSample1D>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyTimeDepthTransform3D {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub derived_from: Vec<String>,
    pub source_kind: TimeDepthTransformSourceKind,
    #[serde(default)]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub grid_transform: Option<SurveyGridTransform>,
    pub time_axis: VerticalAxisDescriptor,
    pub depth_unit: String,
    pub inline_count: usize,
    pub xline_count: usize,
    pub sample_count: usize,
    pub coverage: SpatialCoverageSummary,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum StratigraphicBoundaryReference {
    SurveyTop,
    HorizonAsset { horizon_id: String },
    SurveyBase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum LateralInterpolationMethod {
    Nearest,
    Linear,
    InverseDistance,
    MinimumCurvature,
    Kriging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum VerticalInterpolationMethod {
    Step,
    Linear,
    MonotonicCubic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum VelocityIntervalTrend {
    Constant {
        velocity_m_per_s: f32,
    },
    LinearWithDepth {
        velocity_at_top_m_per_s: f32,
        gradient_m_per_s_per_m: f32,
    },
    LinearWithTime {
        velocity_at_top_m_per_s: f32,
        gradient_m_per_s_per_ms: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityControlProfileSample {
    pub time_ms: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_m: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vrms_m_per_s: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vint_m_per_s: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vavg_m_per_s: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityControlProfile {
    pub id: String,
    pub location: ProjectedPoint2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wellbore_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<VelocityControlProfileSample>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityControlProfileSet {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub travel_time_reference: TravelTimeReference,
    pub depth_reference: DepthReferenceKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<VelocityControlProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct LayeredVelocityInterval {
    pub id: String,
    pub name: String,
    pub top_boundary: StratigraphicBoundaryReference,
    pub base_boundary: StratigraphicBoundaryReference,
    pub trend: VelocityIntervalTrend,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_profile_set_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_profile_velocity_kind: Option<VelocityQuantityKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lateral_interpolation: Option<LateralInterpolationMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_interpolation: Option<VerticalInterpolationMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_blend_weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct LayeredVelocityModel {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_transform: Option<SurveyGridTransform>,
    pub vertical_domain: TimeDepthDomain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub travel_time_reference: Option<TravelTimeReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_reference: Option<DepthReferenceKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intervals: Vec<LayeredVelocityInterval>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct BuildSurveyTimeDepthTransformRequest {
    #[serde(default = "default_pipeline_schema_version")]
    pub schema_version: u32,
    pub store_path: String,
    pub model: LayeredVelocityModel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_profile_sets: Vec<VelocityControlProfileSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_velocity_kind: Option<VelocityQuantityKind>,
    pub output_depth_unit: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct BuildSurveyPropertyFieldRequest {
    #[serde(default = "default_pipeline_schema_version")]
    pub schema_version: u32,
    pub store_path: String,
    pub model: LayeredVelocityModel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_profile_sets: Vec<VelocityControlProfileSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_name: Option<String>,
    pub property_name: String,
    pub property_unit: String,
    pub preferred_velocity_kind: VelocityQuantityKind,
    pub output_vertical_domain: TimeDepthDomain,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vertical_axis() -> VerticalAxisDescriptor {
        VerticalAxisDescriptor {
            domain: TimeDepthDomain::Time,
            unit: "ms".to_string(),
            start: 0.0,
            step: 4.0,
            count: 10,
        }
    }

    fn coverage() -> SpatialCoverageSummary {
        SpatialCoverageSummary {
            relationship: SpatialCoverageRelationship::Unknown,
            source_coordinate_reference: None,
            target_coordinate_reference: None,
            notes: Vec::new(),
        }
    }

    #[test]
    fn survey_time_depth_transform_serializes_presence_stable_fields() {
        let model = SurveyTimeDepthTransform3D {
            id: "vel-1".to_string(),
            name: "Velocity".to_string(),
            derived_from: Vec::new(),
            source_kind: TimeDepthTransformSourceKind::VelocityGrid3D,
            coordinate_reference: None,
            grid_transform: None,
            time_axis: vertical_axis(),
            depth_unit: "m".to_string(),
            inline_count: 8,
            xline_count: 9,
            sample_count: 10,
            coverage: coverage(),
            notes: Vec::new(),
        };

        let value = serde_json::to_value(model).expect("transform should serialize");
        assert_eq!(value["derived_from"], json!([]));
        assert_eq!(value["coordinate_reference"], serde_json::Value::Null);
        assert_eq!(value["grid_transform"], serde_json::Value::Null);
        assert_eq!(value["notes"], json!([]));
        assert_eq!(value["coverage"]["notes"], json!([]));
    }

    #[test]
    fn well_tie_observation_set_serializes_samples_and_nulls() {
        let observation_set = WellTieObservationSet1D {
            id: "tie-1".to_string(),
            name: "Tie".to_string(),
            wellbore_id: None,
            depth_reference: DepthReferenceKind::MeasuredDepth,
            travel_time_reference: TravelTimeReference::TwoWay,
            samples: Vec::new(),
            source_well_time_depth_model_asset_id: None,
            tie_window_start_ms: None,
            tie_window_end_ms: None,
            trace_search_radius_m: None,
            bulk_shift_ms: None,
            stretch_factor: None,
            trace_search_offset_m: None,
            correlation: None,
            notes: Vec::new(),
        };

        let value =
            serde_json::to_value(observation_set).expect("observation set should serialize");
        assert_eq!(value["samples"], json!([]));
        assert_eq!(
            value["source_well_time_depth_model_asset_id"],
            serde_json::Value::Null
        );
        assert_eq!(value["notes"], json!([]));
    }
}
