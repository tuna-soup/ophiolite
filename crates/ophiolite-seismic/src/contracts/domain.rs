use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    SeismicAssetId, SeismicGatherAxisKind, SeismicLayout, SeismicSampleDomain, SeismicSectionAxis,
};

use super::processing::ProcessingPipelineFamily;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[ts(rename = "DatasetId")]
pub struct DatasetId(pub String);

impl From<SeismicAssetId> for DatasetId {
    fn from(value: SeismicAssetId) -> Self {
        Self(value.0)
    }
}

impl From<&SeismicAssetId> for DatasetId {
    fn from(value: &SeismicAssetId) -> Self {
        Self(value.0.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VolumeDescriptor {
    pub id: DatasetId,
    pub store_id: String,
    pub label: String,
    pub shape: [usize; 3],
    pub chunk_shape: [usize; 3],
    pub sample_interval_ms: f32,
    pub sample_data_fidelity: SampleDataFidelity,
    pub geometry: GeometryDescriptor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_binding: Option<CoordinateReferenceBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SurveySpatialDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_lineage_summary: Option<ProcessingLineageSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SampleDataConversionKind {
    Identity,
    Cast,
    FormatTranscode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SampleValuePreservation {
    Exact,
    PotentiallyLossy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SampleDataFidelity {
    pub source_sample_type: String,
    pub working_sample_type: String,
    pub conversion: SampleDataConversionKind,
    pub preservation: SampleValuePreservation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Default for SampleDataFidelity {
    fn default() -> Self {
        Self {
            source_sample_type: "unknown".to_string(),
            working_sample_type: "f32".to_string(),
            conversion: SampleDataConversionKind::Cast,
            preservation: SampleValuePreservation::PotentiallyLossy,
            notes: vec![
                "Source sample fidelity metadata was not recorded in this manifest.".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingArtifactRole {
    FinalOutput,
    Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingLineageSummary {
    pub parent_store_path: String,
    pub parent_store_id: String,
    pub artifact_role: ProcessingArtifactRole,
    pub pipeline_family: ProcessingPipelineFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_name: Option<String>,
    pub pipeline_revision: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GeometryDescriptor {
    pub compare_family: String,
    pub fingerprint: String,
    pub summary: GeometrySummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GeometrySummary {
    pub inline_axis: AxisSummaryI32,
    pub xline_axis: AxisSummaryI32,
    pub sample_axis: AxisSummaryF32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<SeismicLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_axis_kind: Option<GatherAxisKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_axis: Option<AxisSummaryF32>,
    pub provenance: GeometryProvenanceSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CoordinateReferenceDescriptor {
    pub id: Option<String>,
    pub name: Option<String>,
    pub geodetic_datum: Option<String>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CoordinateReferenceSource {
    Header,
    ImportManifest,
    UserOverride,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CoordinateReferenceBinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective: Option<CoordinateReferenceDescriptor>,
    pub source: CoordinateReferenceSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProjectedPoint2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProjectedVector2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProjectedPolygon2 {
    pub exterior: Vec<ProjectedPoint2>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyGridTransform {
    pub origin: ProjectedPoint2,
    pub inline_basis: ProjectedVector2,
    pub xline_basis: ProjectedVector2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SurveySpatialAvailability {
    Available,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveySpatialDescriptor {
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub grid_transform: Option<SurveyGridTransform>,
    pub footprint: Option<ProjectedPolygon2>,
    pub availability: SurveySpatialAvailability,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AxisSummaryI32 {
    pub count: usize,
    pub first: i32,
    pub last: i32,
    pub step: Option<i32>,
    pub regular: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AxisSummaryF32 {
    pub count: usize,
    pub first: f32,
    pub last: f32,
    pub step: Option<f32>,
    pub regular: bool,
    pub units: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GeometryProvenanceSummary {
    Source,
    Derived,
    Regularized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionAxis {
    Inline,
    Xline,
}

impl From<SeismicSectionAxis> for SectionAxis {
    fn from(value: SeismicSectionAxis) -> Self {
        match value {
            SeismicSectionAxis::Inline => Self::Inline,
            SeismicSectionAxis::Xline => Self::Xline,
        }
    }
}

impl From<SectionAxis> for SeismicSectionAxis {
    fn from(value: SectionAxis) -> Self {
        match value {
            SectionAxis::Inline => Self::Inline,
            SectionAxis::Xline => Self::Xline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GatherAxisKind {
    Offset,
    Angle,
    Azimuth,
    Shot,
    Receiver,
    Cmp,
    TraceOrdinal,
    Unknown,
}

impl From<SeismicGatherAxisKind> for GatherAxisKind {
    fn from(value: SeismicGatherAxisKind) -> Self {
        match value {
            SeismicGatherAxisKind::Offset => Self::Offset,
            SeismicGatherAxisKind::Angle => Self::Angle,
            SeismicGatherAxisKind::Azimuth => Self::Azimuth,
            SeismicGatherAxisKind::Shot => Self::Shot,
            SeismicGatherAxisKind::Receiver => Self::Receiver,
            SeismicGatherAxisKind::Cmp => Self::Cmp,
            SeismicGatherAxisKind::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GatherSampleDomain {
    Time,
    Depth,
}

impl From<SeismicSampleDomain> for GatherSampleDomain {
    fn from(value: SeismicSampleDomain) -> Self {
        match value {
            SeismicSampleDomain::Time => Self::Time,
            SeismicSampleDomain::Depth => Self::Depth,
        }
    }
}
