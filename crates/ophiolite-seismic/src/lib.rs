mod contracts;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SeismicAssetId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicAssetFamily {
    Volume,
    Section,
    TraceSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicSampleDomain {
    Time,
    Depth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicStackingState {
    PostStack,
    PreStack,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicOrganization {
    BinnedGrid,
    GatherCollection,
    Unstructured,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SeismicLayout {
    PostStack3D,
    PostStack2D,
    PreStack3DOffset,
    PreStack3DAngle,
    PreStack3DAzimuth,
    PreStack3DUnknownAxis,
    PreStack2DOffset,
    ShotGatherSet,
    ReceiverGatherSet,
    CmpGatherSet,
    UnstructuredTraceCollection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicGatherAxisKind {
    Offset,
    Angle,
    Azimuth,
    Shot,
    Receiver,
    Cmp,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicAxisRole {
    Inline,
    Crossline,
    Sample,
    Offset,
    Angle,
    Azimuth,
    Shot,
    Receiver,
    Cmp,
    TraceOrdinal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicSectionAxis {
    Inline,
    Xline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicRenderMode {
    Heatmap,
    Wiggle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicColorMap {
    Grayscale,
    RedWhiteBlue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeismicPolarity {
    Normal,
    Reversed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeismicUnits {
    pub sample: String,
    pub amplitude: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicSampleAxis {
    pub domain: SeismicSampleDomain,
    pub start: f32,
    pub step: f32,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeismicIndexAxis {
    pub start: i32,
    pub step: i32,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicVolumeGeometry {
    pub inline: SeismicIndexAxis,
    pub xline: SeismicIndexAxis,
    pub sample: SeismicSampleAxis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicVolumeDescriptor {
    pub id: SeismicAssetId,
    pub label: String,
    pub family: SeismicAssetFamily,
    pub shape: [usize; 3],
    pub chunk_shape: [usize; 3],
    pub geometry: SeismicVolumeGeometry,
    pub units: SeismicUnits,
}

impl SeismicVolumeDescriptor {
    pub fn inline_count(&self) -> usize {
        self.shape[0]
    }

    pub fn xline_count(&self) -> usize {
        self.shape[1]
    }

    pub fn sample_count(&self) -> usize {
        self.shape[2]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicDimensionDescriptor {
    pub role: SeismicAxisRole,
    pub label: String,
    pub start: Option<f64>,
    pub step: Option<f64>,
    pub count: usize,
    pub values: Option<Vec<f64>>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicBinGridDescriptor {
    pub inline_axis: Option<SeismicDimensionDescriptor>,
    pub crossline_axis: Option<SeismicDimensionDescriptor>,
    pub coordinate_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicTraceDataDescriptor {
    pub id: SeismicAssetId,
    pub label: String,
    pub stacking_state: SeismicStackingState,
    pub organization: SeismicOrganization,
    pub layout: SeismicLayout,
    pub gather_axis_kind: Option<SeismicGatherAxisKind>,
    pub dimensions: Vec<SeismicDimensionDescriptor>,
    pub chunk_shape: Option<Vec<usize>>,
    pub sample_domain: SeismicSampleDomain,
    pub units: SeismicUnits,
    pub bin_grid: Option<SeismicBinGridDescriptor>,
}

impl SeismicTraceDataDescriptor {
    pub fn dimension(&self, role: SeismicAxisRole) -> Option<&SeismicDimensionDescriptor> {
        self.dimensions
            .iter()
            .find(|dimension| dimension.role == role)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SeismicDescriptorConversionError {
    IncompatibleLayout { layout: SeismicLayout },
    MissingDimension { role: SeismicAxisRole },
    MissingStart { role: SeismicAxisRole },
    MissingStep { role: SeismicAxisRole },
    InvalidChunkShape { actual_len: usize },
    ValueOutOfRange { role: SeismicAxisRole, value: f64 },
}

impl Display for SeismicDescriptorConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompatibleLayout { layout } => {
                write!(
                    f,
                    "descriptor layout {layout:?} is not compatible with SeismicVolumeDescriptor"
                )
            }
            Self::MissingDimension { role } => {
                write!(f, "missing required {:?} dimension", role)
            }
            Self::MissingStart { role } => {
                write!(f, "missing start value for {:?} dimension", role)
            }
            Self::MissingStep { role } => {
                write!(f, "missing step value for {:?} dimension", role)
            }
            Self::InvalidChunkShape { actual_len } => {
                write!(f, "expected 3 chunk-shape dimensions, found {actual_len}")
            }
            Self::ValueOutOfRange { role, value } => {
                write!(f, "value {value} for {:?} dimension is out of range", role)
            }
        }
    }
}

impl Error for SeismicDescriptorConversionError {}

impl From<&SeismicVolumeDescriptor> for SeismicTraceDataDescriptor {
    fn from(value: &SeismicVolumeDescriptor) -> Self {
        let inline_dimension = SeismicDimensionDescriptor {
            role: SeismicAxisRole::Inline,
            label: "inline".to_string(),
            start: Some(value.geometry.inline.start as f64),
            step: Some(value.geometry.inline.step as f64),
            count: value.geometry.inline.count,
            values: None,
            unit: None,
        };
        let crossline_dimension = SeismicDimensionDescriptor {
            role: SeismicAxisRole::Crossline,
            label: "crossline".to_string(),
            start: Some(value.geometry.xline.start as f64),
            step: Some(value.geometry.xline.step as f64),
            count: value.geometry.xline.count,
            values: None,
            unit: None,
        };
        let sample_dimension = SeismicDimensionDescriptor {
            role: SeismicAxisRole::Sample,
            label: "sample".to_string(),
            start: Some(value.geometry.sample.start as f64),
            step: Some(value.geometry.sample.step as f64),
            count: value.geometry.sample.count,
            values: None,
            unit: Some(value.units.sample.clone()),
        };

        Self {
            id: value.id.clone(),
            label: value.label.clone(),
            stacking_state: SeismicStackingState::PostStack,
            organization: SeismicOrganization::BinnedGrid,
            layout: SeismicLayout::PostStack3D,
            gather_axis_kind: None,
            dimensions: vec![
                inline_dimension.clone(),
                crossline_dimension.clone(),
                sample_dimension,
            ],
            chunk_shape: Some(value.chunk_shape.to_vec()),
            sample_domain: value.geometry.sample.domain.clone(),
            units: value.units.clone(),
            bin_grid: Some(SeismicBinGridDescriptor {
                inline_axis: Some(inline_dimension),
                crossline_axis: Some(crossline_dimension),
                coordinate_reference: None,
            }),
        }
    }
}

impl From<&contracts::VolumeDescriptor> for SeismicTraceDataDescriptor {
    fn from(value: &contracts::VolumeDescriptor) -> Self {
        let layout = value
            .geometry
            .summary
            .layout
            .unwrap_or(SeismicLayout::PostStack3D);
        let stacking_state = match layout {
            SeismicLayout::PostStack3D | SeismicLayout::PostStack2D => {
                SeismicStackingState::PostStack
            }
            SeismicLayout::PreStack3DOffset
            | SeismicLayout::PreStack3DAngle
            | SeismicLayout::PreStack3DAzimuth
            | SeismicLayout::PreStack3DUnknownAxis
            | SeismicLayout::PreStack2DOffset
            | SeismicLayout::ShotGatherSet
            | SeismicLayout::ReceiverGatherSet
            | SeismicLayout::CmpGatherSet => SeismicStackingState::PreStack,
            SeismicLayout::UnstructuredTraceCollection => SeismicStackingState::Unknown,
        };
        let organization = match layout {
            SeismicLayout::PostStack3D | SeismicLayout::PostStack2D => {
                SeismicOrganization::BinnedGrid
            }
            SeismicLayout::PreStack3DOffset
            | SeismicLayout::PreStack3DAngle
            | SeismicLayout::PreStack3DAzimuth
            | SeismicLayout::PreStack3DUnknownAxis
            | SeismicLayout::PreStack2DOffset
            | SeismicLayout::ShotGatherSet
            | SeismicLayout::ReceiverGatherSet
            | SeismicLayout::CmpGatherSet => SeismicOrganization::GatherCollection,
            SeismicLayout::UnstructuredTraceCollection => SeismicOrganization::Unstructured,
        };
        let gather_axis_kind = value
            .geometry
            .summary
            .gather_axis_kind
            .map(|kind| match kind {
                contracts::GatherAxisKind::Offset => SeismicGatherAxisKind::Offset,
                contracts::GatherAxisKind::Angle => SeismicGatherAxisKind::Angle,
                contracts::GatherAxisKind::Azimuth => SeismicGatherAxisKind::Azimuth,
                contracts::GatherAxisKind::Shot => SeismicGatherAxisKind::Shot,
                contracts::GatherAxisKind::Receiver => SeismicGatherAxisKind::Receiver,
                contracts::GatherAxisKind::Cmp => SeismicGatherAxisKind::Cmp,
                contracts::GatherAxisKind::TraceOrdinal | contracts::GatherAxisKind::Unknown => {
                    SeismicGatherAxisKind::Unknown
                }
            });
        let inline_dimension = SeismicDimensionDescriptor {
            role: SeismicAxisRole::Inline,
            label: "inline".to_string(),
            start: Some(value.geometry.summary.inline_axis.first as f64),
            step: value
                .geometry
                .summary
                .inline_axis
                .step
                .map(|step| step as f64),
            count: value.geometry.summary.inline_axis.count,
            values: None,
            unit: None,
        };
        let crossline_dimension = SeismicDimensionDescriptor {
            role: SeismicAxisRole::Crossline,
            label: "crossline".to_string(),
            start: Some(value.geometry.summary.xline_axis.first as f64),
            step: value
                .geometry
                .summary
                .xline_axis
                .step
                .map(|step| step as f64),
            count: value.geometry.summary.xline_axis.count,
            values: None,
            unit: None,
        };
        let sample_dimension = SeismicDimensionDescriptor {
            role: SeismicAxisRole::Sample,
            label: "sample".to_string(),
            start: Some(value.geometry.summary.sample_axis.first as f64),
            step: value
                .geometry
                .summary
                .sample_axis
                .step
                .map(|step| step as f64),
            count: value.geometry.summary.sample_axis.count,
            values: None,
            unit: value.geometry.summary.sample_axis.units.clone(),
        };

        Self {
            id: SeismicAssetId(value.id.0.clone()),
            label: value.label.clone(),
            stacking_state,
            organization,
            layout,
            gather_axis_kind,
            dimensions: vec![
                inline_dimension.clone(),
                crossline_dimension.clone(),
                sample_dimension,
            ],
            chunk_shape: Some(value.chunk_shape.to_vec()),
            sample_domain: SeismicSampleDomain::Time,
            units: SeismicUnits {
                sample: value
                    .geometry
                    .summary
                    .sample_axis
                    .units
                    .clone()
                    .unwrap_or_else(|| "ms".to_string()),
                amplitude: None,
            },
            bin_grid: Some(SeismicBinGridDescriptor {
                inline_axis: Some(inline_dimension),
                crossline_axis: Some(crossline_dimension),
                coordinate_reference: None,
            }),
        }
    }
}

impl TryFrom<&SeismicTraceDataDescriptor> for SeismicVolumeDescriptor {
    type Error = SeismicDescriptorConversionError;

    fn try_from(value: &SeismicTraceDataDescriptor) -> Result<Self, Self::Error> {
        if value.layout != SeismicLayout::PostStack3D
            || value.organization != SeismicOrganization::BinnedGrid
            || value.stacking_state != SeismicStackingState::PostStack
        {
            return Err(SeismicDescriptorConversionError::IncompatibleLayout {
                layout: value.layout.clone(),
            });
        }

        let inline = required_index_axis(value, SeismicAxisRole::Inline)?;
        let xline = required_index_axis(value, SeismicAxisRole::Crossline)?;
        let sample = required_sample_axis(value, SeismicAxisRole::Sample)?;

        let chunk_shape = match value.chunk_shape.as_deref() {
            Some([inline, xline, sample]) => [*inline, *xline, *sample],
            Some(other) => {
                return Err(SeismicDescriptorConversionError::InvalidChunkShape {
                    actual_len: other.len(),
                });
            }
            None => [inline.count, xline.count, sample.count],
        };

        Ok(Self {
            id: value.id.clone(),
            label: value.label.clone(),
            family: SeismicAssetFamily::Volume,
            shape: [inline.count, xline.count, sample.count],
            chunk_shape,
            geometry: SeismicVolumeGeometry {
                inline,
                xline,
                sample,
            },
            units: value.units.clone(),
        })
    }
}

fn required_dimension<'a>(
    descriptor: &'a SeismicTraceDataDescriptor,
    role: SeismicAxisRole,
) -> Result<&'a SeismicDimensionDescriptor, SeismicDescriptorConversionError> {
    descriptor
        .dimension(role.clone())
        .ok_or(SeismicDescriptorConversionError::MissingDimension { role })
}

fn required_index_axis(
    descriptor: &SeismicTraceDataDescriptor,
    role: SeismicAxisRole,
) -> Result<SeismicIndexAxis, SeismicDescriptorConversionError> {
    let dimension = required_dimension(descriptor, role.clone())?;
    let start = dimension
        .start
        .ok_or_else(|| SeismicDescriptorConversionError::MissingStart { role: role.clone() })?;
    let step = dimension
        .step
        .ok_or_else(|| SeismicDescriptorConversionError::MissingStep { role: role.clone() })?;

    Ok(SeismicIndexAxis {
        start: f64_to_i32(start, role.clone())?,
        step: f64_to_i32(step, role)?,
        count: dimension.count,
    })
}

fn required_sample_axis(
    descriptor: &SeismicTraceDataDescriptor,
    role: SeismicAxisRole,
) -> Result<SeismicSampleAxis, SeismicDescriptorConversionError> {
    let dimension = required_dimension(descriptor, role.clone())?;
    let start = dimension
        .start
        .ok_or_else(|| SeismicDescriptorConversionError::MissingStart { role: role.clone() })?;
    let step = dimension
        .step
        .ok_or_else(|| SeismicDescriptorConversionError::MissingStep { role: role.clone() })?;

    Ok(SeismicSampleAxis {
        domain: descriptor.sample_domain.clone(),
        start: start as f32,
        step: step as f32,
        count: dimension.count,
    })
}

fn f64_to_i32(value: f64, role: SeismicAxisRole) -> Result<i32, SeismicDescriptorConversionError> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i32::MIN as f64
        || value > i32::MAX as f64
    {
        return Err(SeismicDescriptorConversionError::ValueOutOfRange { role, value });
    }
    Ok(value as i32)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicSectionCoordinate {
    pub index: usize,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeismicSectionRequest {
    pub asset_id: SeismicAssetId,
    pub axis: SeismicSectionAxis,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeismicSectionTileRequest {
    pub section: SeismicSectionRequest,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub lod: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicDisplayDefaults {
    pub gain: f32,
    pub clip_min: Option<f32>,
    pub clip_max: Option<f32>,
    pub render_mode: SeismicRenderMode,
    pub color_map: SeismicColorMap,
    pub polarity: SeismicPolarity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicProbe {
    pub trace_index: usize,
    pub trace_coordinate: f64,
    pub sample_index: usize,
    pub sample_value: f32,
    pub amplitude: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicSectionView {
    pub descriptor: SeismicVolumeDescriptor,
    pub axis: SeismicSectionAxis,
    pub coordinate: SeismicSectionCoordinate,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub sample_axis: Vec<f32>,
    pub amplitudes: Vec<f32>,
    pub display_defaults: Option<SeismicDisplayDefaults>,
    pub probe: Option<SeismicProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicTraceDescriptor {
    pub id: String,
    pub label: String,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicTrace {
    pub descriptor: SeismicTraceDescriptor,
    pub amplitudes: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicTraceSetDescriptor {
    pub id: SeismicAssetId,
    pub label: String,
    pub family: SeismicAssetFamily,
    pub sample_axis: SeismicSampleAxis,
    pub trace_count: usize,
    pub amplitude_unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicTraceSetView {
    pub descriptor: SeismicTraceSetDescriptor,
    pub traces: Vec<SeismicTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicProcessingParameters {
    pub algorithm: String,
    pub gain: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeismicInterpretationPoint {
    pub trace_index: usize,
    pub sample_index: usize,
}

pub use contracts::{
    AmplitudeSpectrumCurve, AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, AxisSummaryF32,
    AxisSummaryI32, CancelProcessingJobRequest, CancelProcessingJobResponse, DatasetId,
    DatasetSummary, DeletePipelinePresetRequest, DeletePipelinePresetResponse, FrequencyPhaseMode,
    FrequencyWindowShape, GatherAxisKind, GatherInteractionChanged, GatherInterpolationMode,
    GatherPreviewView, GatherProbe, GatherProbeChanged, GatherProcessingOperation,
    GatherProcessingPipeline, GatherRequest, GatherSampleDomain, GatherSelector, GatherView,
    GatherViewport, GatherViewportChanged, GeometryDescriptor, GeometryProvenanceSummary,
    GeometrySummary, GetProcessingJobRequest, GetProcessingJobResponse, IPC_SCHEMA_VERSION,
    ImportDatasetRequest, ImportDatasetResponse, ImportPrestackOffsetDatasetRequest,
    ImportPrestackOffsetDatasetResponse, InterpretationPoint, ListPipelinePresetsResponse,
    OpenDatasetRequest, OpenDatasetResponse, PrestackThirdAxisField, PreviewCommand,
    PreviewGatherProcessingRequest, PreviewGatherProcessingResponse, PreviewProcessingRequest,
    PreviewProcessingResponse, PreviewResponse, PreviewTraceLocalProcessingRequest,
    PreviewTraceLocalProcessingResponse, PreviewView, ProcessingJobProgress, ProcessingJobState,
    ProcessingJobStatus, ProcessingLayoutCompatibility, ProcessingOperation,
    ProcessingOperatorScope, ProcessingPipeline, ProcessingPipelineFamily, ProcessingPipelineSpec,
    ProcessingPreset, RunGatherProcessingRequest, RunGatherProcessingResponse,
    RunProcessingRequest, RunProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SavePipelinePresetRequest, SavePipelinePresetResponse,
    SectionAxis, SectionColorMap, SectionCoordinate, SectionDisplayDefaults,
    SectionInteractionChanged, SectionMetadata, SectionPolarity, SectionPrimaryMode, SectionProbe,
    SectionProbeChanged, SectionRenderMode, SectionRequest, SectionSpectrumSelection,
    SectionTileRequest, SectionUnits, SectionView, SectionViewport, SectionViewportChanged,
    SemblancePanel, SuggestedImportAction, SurveyPreflightRequest, SurveyPreflightResponse,
    TraceLocalProcessingOperation, TraceLocalProcessingPipeline, TraceLocalProcessingPreset,
    TraceLocalVolumeArithmeticOperator, VelocityAutopickParameters, VelocityFunctionEstimate,
    VelocityFunctionSource, VelocityPickStrategy, VelocityScanRequest, VelocityScanResponse,
    VolumeDescriptor,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_descriptor() -> SeismicVolumeDescriptor {
        SeismicVolumeDescriptor {
            id: SeismicAssetId("asset-demo".to_string()),
            label: "Demo survey".to_string(),
            family: SeismicAssetFamily::Volume,
            shape: [32, 48, 256],
            chunk_shape: [8, 8, 64],
            geometry: SeismicVolumeGeometry {
                inline: SeismicIndexAxis {
                    start: 1000,
                    step: 2,
                    count: 32,
                },
                xline: SeismicIndexAxis {
                    start: 2000,
                    step: 1,
                    count: 48,
                },
                sample: SeismicSampleAxis {
                    domain: SeismicSampleDomain::Time,
                    start: 0.0,
                    step: 4.0,
                    count: 256,
                },
            },
            units: SeismicUnits {
                sample: "ms".to_string(),
                amplitude: Some("arb".to_string()),
            },
        }
    }

    #[test]
    fn volume_descriptor_reports_axis_counts() {
        let descriptor = sample_descriptor();
        assert_eq!(descriptor.inline_count(), 32);
        assert_eq!(descriptor.xline_count(), 48);
        assert_eq!(descriptor.sample_count(), 256);
    }

    #[test]
    fn volume_descriptor_upcasts_to_trace_data_descriptor() {
        let descriptor = sample_descriptor();
        let trace_descriptor = SeismicTraceDataDescriptor::from(&descriptor);

        assert_eq!(trace_descriptor.layout, SeismicLayout::PostStack3D);
        assert_eq!(
            trace_descriptor.stacking_state,
            SeismicStackingState::PostStack
        );
        assert_eq!(
            trace_descriptor.organization,
            SeismicOrganization::BinnedGrid
        );
        assert_eq!(
            trace_descriptor
                .dimension(SeismicAxisRole::Inline)
                .unwrap()
                .count,
            32
        );
        assert_eq!(trace_descriptor.chunk_shape, Some(vec![8, 8, 64]));
    }

    #[test]
    fn post_stack_trace_data_descriptor_downcasts_to_volume_descriptor() {
        let descriptor = sample_descriptor();
        let trace_descriptor = SeismicTraceDataDescriptor::from(&descriptor);
        let restored = SeismicVolumeDescriptor::try_from(&trace_descriptor).unwrap();

        assert_eq!(restored, descriptor);
    }

    #[test]
    fn prestack_trace_data_descriptor_rejects_volume_downcast() {
        let descriptor = sample_descriptor();
        let mut trace_descriptor = SeismicTraceDataDescriptor::from(&descriptor);
        trace_descriptor.layout = SeismicLayout::PreStack3DOffset;
        trace_descriptor.stacking_state = SeismicStackingState::PreStack;
        trace_descriptor.gather_axis_kind = Some(SeismicGatherAxisKind::Offset);
        trace_descriptor.dimensions.insert(
            2,
            SeismicDimensionDescriptor {
                role: SeismicAxisRole::Offset,
                label: "offset".to_string(),
                start: Some(0.0),
                step: Some(25.0),
                count: 8,
                values: None,
                unit: Some("m".to_string()),
            },
        );

        let error = SeismicVolumeDescriptor::try_from(&trace_descriptor).unwrap_err();
        assert_eq!(
            error,
            SeismicDescriptorConversionError::IncompatibleLayout {
                layout: SeismicLayout::PreStack3DOffset,
            }
        );
    }

    #[test]
    fn section_request_round_trips_through_json() {
        let request = SeismicSectionRequest {
            asset_id: SeismicAssetId("asset-demo".to_string()),
            axis: SeismicSectionAxis::Inline,
            index: 12,
        };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: SeismicSectionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, request);
    }
}
