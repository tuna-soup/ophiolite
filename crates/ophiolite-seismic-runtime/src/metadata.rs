use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderFieldSpec {
    pub name: String,
    pub start_byte: u16,
    pub value_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryProvenance {
    pub inline_field: HeaderFieldSpec,
    pub crossline_field: HeaderFieldSpec,
    pub third_axis_field: Option<HeaderFieldSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegularizationProvenance {
    pub source_classification: String,
    pub fill_value: f32,
    pub observed_trace_count: usize,
    pub expected_trace_count: usize,
    pub missing_bin_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DatasetKind {
    Source,
    Derived,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterpMethod {
    Linear,
    Cubic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub source_path: PathBuf,
    pub file_size: u64,
    pub trace_count: u64,
    pub samples_per_trace: usize,
    pub sample_interval_us: u16,
    pub sample_format_code: u16,
    pub geometry: GeometryProvenance,
    pub regularization: Option<RegularizationProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAxes {
    pub ilines: Vec<f64>,
    pub xlines: Vec<f64>,
    pub sample_axis_ms: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedFrom {
    pub parent_store: PathBuf,
    pub method: InterpMethod,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingOperation {
    AmplitudeScalar { factor: f32 },
    TraceRmsNormalize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompressionKind {
    None,
    BloscLz4,
    Zstd,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageLayout {
    pub compression: CompressionKind,
    pub shard_shape: Option<[usize; 3]>,
}

impl Default for StorageLayout {
    fn default() -> Self {
        Self {
            compression: CompressionKind::None,
            shard_shape: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingLineage {
    pub parent_store: PathBuf,
    pub pipeline: Vec<ProcessingOperation>,
    pub runtime_version: String,
    pub created_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetadata {
    pub kind: DatasetKind,
    pub source: SourceIdentity,
    pub shape: [usize; 3],
    pub axes: VolumeAxes,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_lineage: Option<ProcessingLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreManifest {
    pub version: u32,
    pub kind: DatasetKind,
    pub source: SourceIdentity,
    pub shape: [usize; 3],
    pub chunk_shape: [usize; 3],
    pub axes: VolumeAxes,
    pub array_path: String,
    pub occupancy_array_path: Option<String>,
    pub created_by: String,
    pub derived_from: Option<DerivedFrom>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_lineage: Option<ProcessingLineage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_layout: Option<StorageLayout>,
}

impl StoreManifest {
    pub const FILE_NAME: &'static str = "seisrefine.manifest.json";
}

impl From<&StoreManifest> for VolumeMetadata {
    fn from(value: &StoreManifest) -> Self {
        Self {
            kind: value.kind.clone(),
            source: value.source.clone(),
            shape: value.shape,
            axes: value.axes.clone(),
            created_by: value.created_by.clone(),
            processing_lineage: value.processing_lineage.clone(),
        }
    }
}
