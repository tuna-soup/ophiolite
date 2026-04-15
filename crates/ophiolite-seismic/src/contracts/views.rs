use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{SeismicColorMap, SeismicPolarity, SeismicRenderMode};

use super::domain::{
    CoordinateReferenceDescriptor, DatasetId, GatherAxisKind, GatherSampleDomain, SectionAxis,
};
use super::models::{
    SpatialCoverageRelationship, TimeDepthDomain, TimeDepthTransformSourceKind,
    VelocityQuantityKind,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionScalarOverlayColorMap {
    Grayscale,
    Viridis,
    Turbo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionTimeDepthTransformMode {
    None,
    Global1d,
    Survey3d,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionTimeDepthDiagnostics {
    pub display_domain: TimeDepthDomain,
    pub transform_mode: SectionTimeDepthTransformMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<TimeDepthTransformSourceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity_kind: Option<VelocityQuantityKind>,
    pub trace_varying: bool,
    pub coverage_relationship: SpatialCoverageRelationship,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionScalarOverlayValueRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionScalarOverlayView {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub width: usize,
    pub height: usize,
    pub values_f32le: Vec<u8>,
    pub color_map: SectionScalarOverlayColorMap,
    pub opacity: f32,
    pub value_range: SectionScalarOverlayValueRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionHorizonLineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionHorizonStyle {
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_width: Option<f32>,
    pub line_style: SectionHorizonLineStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionHorizonSample {
    pub trace_index: usize,
    pub sample_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_value: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionHorizonOverlayView {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub style: SectionHorizonStyle,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<SectionHorizonSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ResolvedSectionDisplayView {
    pub section: SectionView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_depth_diagnostics: Option<SectionTimeDepthDiagnostics>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scalar_overlays: Vec<SectionScalarOverlayView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub horizon_overlays: Vec<SectionHorizonOverlayView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportedHorizonDescriptor {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub point_count: usize,
    pub mapped_point_count: usize,
    pub missing_cell_count: usize,
    pub vertical_domain: TimeDepthDomain,
    pub vertical_unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aligned_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub transformed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub style: SectionHorizonStyle,
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
pub struct GatherPreviewView {
    pub gather: GatherView,
    pub processing_label: String,
    pub preview_ready: bool,
}

impl GatherPreviewView {
    pub fn pending(gather: GatherView, processing_label: impl Into<String>) -> Self {
        Self {
            gather,
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
