use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::SeismicLayout;

use super::{default_pipeline_revision, default_pipeline_schema_version};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TraceLocalProcessingOperation {
    AmplitudeScalar {
        factor: f32,
    },
    TraceRmsNormalize,
    AgcRms {
        window_ms: f32,
    },
    PhaseRotation {
        angle_degrees: f32,
    },
    LowpassFilter {
        f3_hz: f32,
        f4_hz: f32,
        phase: FrequencyPhaseMode,
        window: FrequencyWindowShape,
    },
    HighpassFilter {
        f1_hz: f32,
        f2_hz: f32,
        phase: FrequencyPhaseMode,
        window: FrequencyWindowShape,
    },
    BandpassFilter {
        f1_hz: f32,
        f2_hz: f32,
        f3_hz: f32,
        f4_hz: f32,
        phase: FrequencyPhaseMode,
        window: FrequencyWindowShape,
    },
    VolumeArithmetic {
        operator: TraceLocalVolumeArithmeticOperator,
        secondary_store_path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum TraceLocalVolumeArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum FrequencyPhaseMode {
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum FrequencyWindowShape {
    CosineTaper,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingOperatorScope {
    TraceLocal,
    SectionMatrix,
    GatherMatrix,
    InverseWavelet,
}

impl ProcessingOperatorScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::TraceLocal => "trace-local",
            Self::SectionMatrix => "section-matrix",
            Self::GatherMatrix => "gather-matrix",
            Self::InverseWavelet => "inverse-wavelet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessingSampleDependency {
    Pointwise,
    BoundedWindow { window_ms_hint: f32 },
    WholeTrace,
}

impl ProcessingSampleDependency {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pointwise => "pointwise",
            Self::BoundedWindow { .. } => "bounded_window",
            Self::WholeTrace => "whole_trace",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingSpatialDependency {
    SingleTrace,
    SectionNeighborhood,
    GatherNeighborhood,
    ExternalVolumePointwise,
    Global,
}

impl ProcessingSpatialDependency {
    pub fn label(self) -> &'static str {
        match self {
            Self::SingleTrace => "single_trace",
            Self::SectionNeighborhood => "section_neighborhood",
            Self::GatherNeighborhood => "gather_neighborhood",
            Self::ExternalVolumePointwise => "external_volume_pointwise",
            Self::Global => "global",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessingOperatorDependencyProfile {
    pub deterministic: bool,
    pub sample_dependency: ProcessingSampleDependency,
    pub spatial_dependency: ProcessingSpatialDependency,
    pub inline_radius: usize,
    pub crossline_radius: usize,
    pub same_section_ephemeral_reuse_safe: bool,
}

impl TraceLocalProcessingOperation {
    pub fn operator_id(&self) -> &'static str {
        match self {
            Self::AmplitudeScalar { .. } => "amplitude_scalar",
            Self::TraceRmsNormalize => "trace_rms_normalize",
            Self::AgcRms { .. } => "agc_rms",
            Self::PhaseRotation { .. } => "phase_rotation",
            Self::LowpassFilter { .. } => "lowpass_filter",
            Self::HighpassFilter { .. } => "highpass_filter",
            Self::BandpassFilter { .. } => "bandpass_filter",
            Self::VolumeArithmetic { .. } => "volume_arithmetic",
        }
    }

    pub fn scope(&self) -> ProcessingOperatorScope {
        match self {
            Self::AmplitudeScalar { .. }
            | Self::TraceRmsNormalize
            | Self::AgcRms { .. }
            | Self::PhaseRotation { .. }
            | Self::LowpassFilter { .. }
            | Self::HighpassFilter { .. }
            | Self::BandpassFilter { .. }
            | Self::VolumeArithmetic { .. } => ProcessingOperatorScope::TraceLocal,
        }
    }

    pub fn compatibility(&self) -> ProcessingLayoutCompatibility {
        match self {
            Self::AmplitudeScalar { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::TraceRmsNormalize => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::AgcRms { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::PhaseRotation { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::LowpassFilter { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::HighpassFilter { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::BandpassFilter { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
            Self::VolumeArithmetic { .. } => ProcessingLayoutCompatibility::AnyTraceMatrix,
        }
    }

    pub fn dependency_profile(&self) -> ProcessingOperatorDependencyProfile {
        match self {
            Self::AmplitudeScalar { .. } => ProcessingOperatorDependencyProfile {
                deterministic: true,
                sample_dependency: ProcessingSampleDependency::Pointwise,
                spatial_dependency: ProcessingSpatialDependency::SingleTrace,
                inline_radius: 0,
                crossline_radius: 0,
                same_section_ephemeral_reuse_safe: true,
            },
            Self::TraceRmsNormalize => ProcessingOperatorDependencyProfile {
                deterministic: true,
                sample_dependency: ProcessingSampleDependency::WholeTrace,
                spatial_dependency: ProcessingSpatialDependency::SingleTrace,
                inline_radius: 0,
                crossline_radius: 0,
                same_section_ephemeral_reuse_safe: true,
            },
            Self::AgcRms { window_ms } => ProcessingOperatorDependencyProfile {
                deterministic: true,
                sample_dependency: ProcessingSampleDependency::BoundedWindow {
                    window_ms_hint: *window_ms,
                },
                spatial_dependency: ProcessingSpatialDependency::SingleTrace,
                inline_radius: 0,
                crossline_radius: 0,
                same_section_ephemeral_reuse_safe: true,
            },
            Self::PhaseRotation { .. }
            | Self::LowpassFilter { .. }
            | Self::HighpassFilter { .. }
            | Self::BandpassFilter { .. } => ProcessingOperatorDependencyProfile {
                deterministic: true,
                sample_dependency: ProcessingSampleDependency::WholeTrace,
                spatial_dependency: ProcessingSpatialDependency::SingleTrace,
                inline_radius: 0,
                crossline_radius: 0,
                same_section_ephemeral_reuse_safe: true,
            },
            Self::VolumeArithmetic { .. } => ProcessingOperatorDependencyProfile {
                deterministic: true,
                sample_dependency: ProcessingSampleDependency::Pointwise,
                spatial_dependency: ProcessingSpatialDependency::ExternalVolumePointwise,
                inline_radius: 0,
                crossline_radius: 0,
                same_section_ephemeral_reuse_safe: true,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TraceLocalProcessingStep {
    pub operation: TraceLocalProcessingOperation,
    #[serde(default)]
    pub checkpoint: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
pub struct TraceLocalProcessingPipeline {
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
    pub steps: Vec<TraceLocalProcessingStep>,
}

#[derive(Debug, Deserialize)]
struct TraceLocalProcessingPipelineSerde {
    #[serde(default = "default_pipeline_schema_version")]
    schema_version: u32,
    #[serde(default = "default_pipeline_revision")]
    revision: u32,
    #[serde(default)]
    preset_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    steps: Vec<TraceLocalProcessingStep>,
    #[serde(default)]
    operations: Vec<TraceLocalProcessingOperation>,
}

impl<'de> Deserialize<'de> for TraceLocalProcessingPipeline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = TraceLocalProcessingPipelineSerde::deserialize(deserializer)?;
        let steps = if value.steps.is_empty() && !value.operations.is_empty() {
            value
                .operations
                .into_iter()
                .map(|operation| TraceLocalProcessingStep {
                    operation,
                    checkpoint: false,
                })
                .collect()
        } else {
            value.steps
        };

        Ok(Self {
            schema_version: value.schema_version,
            revision: value.revision,
            preset_id: value.preset_id,
            name: value.name,
            description: value.description,
            steps,
        })
    }
}

impl TraceLocalProcessingPipeline {
    pub fn operation_count(&self) -> usize {
        self.steps.len()
    }

    pub fn operations(&self) -> impl Iterator<Item = &TraceLocalProcessingOperation> {
        self.steps.iter().map(|step| &step.operation)
    }

    pub fn checkpoint_indexes(&self) -> Vec<usize> {
        self.steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| step.checkpoint.then_some(index))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SubvolumeCropOperation {
    pub inline_min: i32,
    pub inline_max: i32,
    pub xline_min: i32,
    pub xline_max: i32,
    pub z_min_ms: f32,
    pub z_max_ms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SubvolumeProcessingPipeline {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_local_pipeline: Option<TraceLocalProcessingPipeline>,
    pub crop: SubvolumeCropOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GatherInterpolationMode {
    Linear,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GatherProcessingOperation {
    NmoCorrection {
        velocity_model: super::models::VelocityFunctionSource,
        interpolation: GatherInterpolationMode,
    },
    StretchMute {
        velocity_model: super::models::VelocityFunctionSource,
        max_stretch_ratio: f32,
    },
    OffsetMute {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_offset: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_offset: Option<f32>,
    },
}

impl GatherProcessingOperation {
    pub fn operator_id(&self) -> &'static str {
        match self {
            Self::NmoCorrection { .. } => "nmo_correction",
            Self::StretchMute { .. } => "stretch_mute",
            Self::OffsetMute { .. } => "offset_mute",
        }
    }

    pub fn scope(&self) -> ProcessingOperatorScope {
        ProcessingOperatorScope::GatherMatrix
    }

    pub fn compatibility(&self) -> ProcessingLayoutCompatibility {
        ProcessingLayoutCompatibility::PreStackOffsetOnly
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherProcessingPipeline {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_local_pipeline: Option<TraceLocalProcessingPipeline>,
    pub operations: Vec<GatherProcessingOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingPipelineFamily {
    TraceLocal,
    Subvolume,
    Gather,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingPipelineSpec {
    TraceLocal {
        pipeline: TraceLocalProcessingPipeline,
    },
    Subvolume {
        pipeline: SubvolumeProcessingPipeline,
    },
    Gather {
        pipeline: GatherProcessingPipeline,
    },
}

impl ProcessingPipelineSpec {
    pub fn family(&self) -> ProcessingPipelineFamily {
        match self {
            Self::TraceLocal { .. } => ProcessingPipelineFamily::TraceLocal,
            Self::Subvolume { .. } => ProcessingPipelineFamily::Subvolume,
            Self::Gather { .. } => ProcessingPipelineFamily::Gather,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingJobArtifactKind {
    Checkpoint,
    FinalOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingJobArtifact {
    pub kind: ProcessingJobArtifactKind,
    pub step_index: usize,
    pub label: String,
    pub store_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ProcessingJobStatus {
    pub job_id: String,
    pub state: ProcessingJobState,
    pub progress: ProcessingJobProgress,
    pub input_store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_store_path: Option<String>,
    pub pipeline: ProcessingPipelineSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ProcessingJobArtifact>,
    #[ts(type = "number")]
    pub created_at_unix_s: u64,
    #[ts(type = "number")]
    pub updated_at_unix_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TraceLocalProcessingPreset {
    pub preset_id: String,
    pub pipeline: TraceLocalProcessingPipeline,
    #[ts(type = "number")]
    pub created_at_unix_s: u64,
    #[ts(type = "number")]
    pub updated_at_unix_s: u64,
}

pub type ProcessingOperation = TraceLocalProcessingOperation;
pub type ProcessingPipeline = TraceLocalProcessingPipeline;
pub type ProcessingPreset = TraceLocalProcessingPreset;
