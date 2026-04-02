mod contracts;

use serde::{Deserialize, Serialize};

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
    CancelProcessingJobRequest, CancelProcessingJobResponse, DatasetId, DatasetSummary,
    DeletePipelinePresetRequest, DeletePipelinePresetResponse, GetProcessingJobRequest,
    GetProcessingJobResponse, IPC_SCHEMA_VERSION, ImportDatasetRequest, ImportDatasetResponse,
    InterpretationPoint, ListPipelinePresetsResponse, OpenDatasetRequest, OpenDatasetResponse,
    PreviewCommand, PreviewProcessingRequest, PreviewProcessingResponse, PreviewResponse,
    PreviewView, ProcessingJobProgress, ProcessingJobState, ProcessingJobStatus,
    ProcessingOperation, ProcessingPipeline, ProcessingPreset, SectionAxis, SectionColorMap,
    SectionCoordinate, SectionDisplayDefaults, SectionInteractionChanged, SectionMetadata,
    SectionPolarity, SectionPrimaryMode, SectionProbe, SectionProbeChanged, SectionRenderMode,
    SectionRequest, SectionTileRequest, SectionUnits, SectionView, SectionViewport,
    SectionViewportChanged, SuggestedImportAction, SurveyPreflightRequest,
    SurveyPreflightResponse, RunProcessingRequest, RunProcessingResponse,
    SavePipelinePresetRequest, SavePipelinePresetResponse, VolumeDescriptor,
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
