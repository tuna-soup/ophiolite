use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    SeismicAssetId, SeismicColorMap, SeismicLayout, SeismicPolarity, SeismicRenderMode,
    SeismicSectionAxis,
};

fn default_pipeline_schema_version() -> u32 {
    1
}

fn default_pipeline_revision() -> u32 {
    1
}

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
    pub label: String,
    pub shape: [usize; 3],
    pub chunk_shape: [usize; 3],
    pub sample_interval_ms: f32,
    pub geometry: GeometryDescriptor,
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
    pub provenance: GeometryProvenanceSummary,
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

impl From<crate::SeismicGatherAxisKind> for GatherAxisKind {
    fn from(value: crate::SeismicGatherAxisKind) -> Self {
        match value {
            crate::SeismicGatherAxisKind::Offset => Self::Offset,
            crate::SeismicGatherAxisKind::Angle => Self::Angle,
            crate::SeismicGatherAxisKind::Azimuth => Self::Azimuth,
            crate::SeismicGatherAxisKind::Shot => Self::Shot,
            crate::SeismicGatherAxisKind::Receiver => Self::Receiver,
            crate::SeismicGatherAxisKind::Cmp => Self::Cmp,
            crate::SeismicGatherAxisKind::Unknown => Self::Unknown,
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

impl From<crate::SeismicSampleDomain> for GatherSampleDomain {
    fn from(value: crate::SeismicSampleDomain) -> Self {
        match value {
            crate::SeismicSampleDomain::Time => Self::Time,
            crate::SeismicSampleDomain::Depth => Self::Depth,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionRequest {
    pub dataset_id: DatasetId,
    pub axis: SectionAxis,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionTileRequest {
    pub section: SectionRequest,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub lod: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingOperation {
    AmplitudeScalar { factor: f32 },
    TraceRmsNormalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingLayoutCompatibility {
    PostStackOnly,
    PreStackOffsetOnly,
    AnyTraceMatrix,
}

impl ProcessingLayoutCompatibility {
    pub fn supports_layout(self, layout: SeismicLayout) -> bool {
        match self {
            Self::PostStackOnly => matches!(
                layout,
                SeismicLayout::PostStack3D | SeismicLayout::PostStack2D
            ),
            Self::PreStackOffsetOnly => matches!(
                layout,
                SeismicLayout::PreStack3DOffset | SeismicLayout::PreStack2DOffset
            ),
            Self::AnyTraceMatrix => matches!(
                layout,
                SeismicLayout::PostStack3D
                    | SeismicLayout::PostStack2D
                    | SeismicLayout::PreStack3DOffset
                    | SeismicLayout::PreStack3DAngle
                    | SeismicLayout::PreStack3DAzimuth
                    | SeismicLayout::PreStack3DUnknownAxis
                    | SeismicLayout::PreStack2DOffset
                    | SeismicLayout::ShotGatherSet
                    | SeismicLayout::ReceiverGatherSet
                    | SeismicLayout::CmpGatherSet
            ),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PostStackOnly => "post-stack only",
            Self::PreStackOffsetOnly => "prestack offset only",
            Self::AnyTraceMatrix => "any trace matrix",
        }
    }
}

impl ProcessingOperation {
    pub fn operator_id(&self) -> &'static str {
        match self {
            Self::AmplitudeScalar { .. } => "amplitude_scalar",
            Self::TraceRmsNormalize => "trace_rms_normalize",
        }
    }

    pub fn compatibility(&self) -> ProcessingLayoutCompatibility {
        match self {
            Self::AmplitudeScalar { .. } => ProcessingLayoutCompatibility::PostStackOnly,
            Self::TraceRmsNormalize => ProcessingLayoutCompatibility::PostStackOnly,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingPipeline {
    #[serde(default = "default_pipeline_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_pipeline_revision")]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub operations: Vec<ProcessingOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingJobProgress {
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingJobStatus {
    pub job_id: String,
    pub state: ProcessingJobState,
    pub progress: ProcessingJobProgress,
    pub input_store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_store_path: Option<String>,
    pub pipeline: ProcessingPipeline,
    #[ts(type = "number")]
    pub created_at_unix_s: u64,
    #[ts(type = "number")]
    pub updated_at_unix_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingPreset {
    pub preset_id: String,
    pub pipeline: ProcessingPipeline,
    #[ts(type = "number")]
    pub created_at_unix_s: u64,
    #[ts(type = "number")]
    pub updated_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct InterpretationPoint {
    pub trace_index: usize,
    pub sample_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionColorMap {
    Grayscale,
    RedWhiteBlue,
}

impl From<SeismicColorMap> for SectionColorMap {
    fn from(value: SeismicColorMap) -> Self {
        match value {
            SeismicColorMap::Grayscale => Self::Grayscale,
            SeismicColorMap::RedWhiteBlue => Self::RedWhiteBlue,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionRenderMode {
    Heatmap,
    Wiggle,
}

impl From<SeismicRenderMode> for SectionRenderMode {
    fn from(value: SeismicRenderMode) -> Self {
        match value {
            SeismicRenderMode::Heatmap => Self::Heatmap,
            SeismicRenderMode::Wiggle => Self::Wiggle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionPolarity {
    Normal,
    Reversed,
}

impl From<SeismicPolarity> for SectionPolarity {
    fn from(value: SeismicPolarity) -> Self {
        match value {
            SeismicPolarity::Normal => Self::Normal,
            SeismicPolarity::Reversed => Self::Reversed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionPrimaryMode {
    Cursor,
    PanZoom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionCoordinate {
    pub index: usize,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionUnits {
    pub horizontal: Option<String>,
    pub sample: Option<String>,
    pub amplitude: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionMetadata {
    pub store_id: Option<String>,
    pub derived_from: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionDisplayDefaults {
    pub gain: f32,
    pub clip_min: Option<f32>,
    pub clip_max: Option<f32>,
    pub render_mode: SectionRenderMode,
    pub colormap: SectionColorMap,
    pub polarity: SectionPolarity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionView {
    pub dataset_id: DatasetId,
    pub axis: SectionAxis,
    pub coordinate: SectionCoordinate,
    pub traces: usize,
    pub samples: usize,
    pub horizontal_axis_f64le: Vec<u8>,
    pub inline_axis_f64le: Option<Vec<u8>>,
    pub xline_axis_f64le: Option<Vec<u8>>,
    pub sample_axis_f32le: Vec<u8>,
    pub amplitudes_f32le: Vec<u8>,
    pub units: Option<SectionUnits>,
    pub metadata: Option<SectionMetadata>,
    pub display_defaults: Option<SectionDisplayDefaults>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherView {
    pub dataset_id: DatasetId,
    pub label: String,
    pub gather_axis_kind: GatherAxisKind,
    pub sample_domain: GatherSampleDomain,
    pub traces: usize,
    pub samples: usize,
    pub horizontal_axis_f64le: Vec<u8>,
    pub sample_axis_f32le: Vec<u8>,
    pub amplitudes_f32le: Vec<u8>,
    pub units: Option<SectionUnits>,
    pub metadata: Option<SectionMetadata>,
    pub display_defaults: Option<SectionDisplayDefaults>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewView {
    pub section: SectionView,
    pub processing_label: String,
    pub preview_ready: bool,
}

impl PreviewView {
    pub fn pending(section: SectionView, processing_label: impl Into<String>) -> Self {
        Self {
            section,
            processing_label: processing_label.into(),
            preview_ready: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionViewport {
    pub trace_start: usize,
    pub trace_end: usize,
    pub sample_start: usize,
    pub sample_end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherViewport {
    pub trace_start: usize,
    pub trace_end: usize,
    pub sample_start: usize,
    pub sample_end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionProbe {
    pub trace_index: usize,
    pub trace_coordinate: f64,
    pub inline_coordinate: Option<f64>,
    pub xline_coordinate: Option<f64>,
    pub sample_index: usize,
    pub sample_value: f32,
    pub amplitude: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherProbe {
    pub trace_index: usize,
    pub trace_coordinate: f64,
    pub sample_index: usize,
    pub sample_value: f32,
    pub amplitude: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionProbeChanged {
    pub chart_id: String,
    pub view_id: String,
    pub probe: Option<SectionProbe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherProbeChanged {
    pub chart_id: String,
    pub view_id: String,
    pub probe: Option<GatherProbe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionViewportChanged {
    pub chart_id: String,
    pub view_id: String,
    pub viewport: SectionViewport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherViewportChanged {
    pub chart_id: String,
    pub view_id: String,
    pub viewport: GatherViewport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionInteractionChanged {
    pub chart_id: String,
    pub view_id: String,
    pub primary_mode: SectionPrimaryMode,
    pub crosshair_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherInteractionChanged {
    pub chart_id: String,
    pub view_id: String,
    pub primary_mode: SectionPrimaryMode,
    pub crosshair_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SuggestedImportAction {
    DirectDenseIngest,
    RegularizeSparseSurvey,
    ReviewGeometryMapping,
    UnsupportedInV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DatasetSummary {
    pub store_path: String,
    pub descriptor: VolumeDescriptor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyPreflightRequest {
    pub schema_version: u32,
    pub input_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyPreflightResponse {
    pub schema_version: u32,
    pub input_path: String,
    pub trace_count: u64,
    pub samples_per_trace: usize,
    pub classification: String,
    pub stacking_state: String,
    pub organization: String,
    pub layout: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_axis_kind: Option<String>,
    pub suggested_action: SuggestedImportAction,
    pub observed_trace_count: usize,
    pub expected_trace_count: usize,
    pub completeness_ratio: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportDatasetRequest {
    pub schema_version: u32,
    pub input_path: String,
    pub output_store_path: String,
    #[serde(default)]
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportDatasetResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct OpenDatasetRequest {
    pub schema_version: u32,
    pub store_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct OpenDatasetResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewCommand {
    pub schema_version: u32,
    pub request: SectionRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewResponse {
    pub schema_version: u32,
    pub preview: PreviewView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub section: SectionRequest,
    pub pipeline: ProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewProcessingResponse {
    pub schema_version: u32,
    pub preview: PreviewView,
    pub pipeline: ProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_store_path: Option<String>,
    #[serde(default)]
    pub overwrite_existing: bool,
    pub pipeline: ProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunProcessingResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GetProcessingJobRequest {
    pub schema_version: u32,
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GetProcessingJobResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CancelProcessingJobRequest {
    pub schema_version: u32,
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CancelProcessingJobResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ListPipelinePresetsResponse {
    pub schema_version: u32,
    pub presets: Vec<ProcessingPreset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SavePipelinePresetRequest {
    pub schema_version: u32,
    pub preset: ProcessingPreset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SavePipelinePresetResponse {
    pub schema_version: u32,
    pub preset: ProcessingPreset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeletePipelinePresetRequest {
    pub schema_version: u32,
    pub preset_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeletePipelinePresetResponse {
    pub schema_version: u32,
    pub deleted: bool,
}

pub const IPC_SCHEMA_VERSION: u32 = 1;
