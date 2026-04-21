use std::path::PathBuf;

use ophiolite_seismic::{
    CoordinateReferenceBinding, ProcessingArtifactRole, ProcessingPipelineSpec,
    SampleDataConversionKind, SampleDataFidelity, SampleValuePreservation, SurveySpatialDescriptor,
    TimeDepthDomain,
};
use ophiolite_seismic_io::SampleFormat;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    #[serde(default)]
    pub sample_data_fidelity: SampleDataFidelity,
    #[serde(default = "default_source_endianness")]
    pub endianness: String,
    #[serde(default)]
    pub revision_raw: u16,
    #[serde(default = "default_fixed_length_trace_flag_raw")]
    pub fixed_length_trace_flag_raw: u16,
    #[serde(default)]
    pub extended_textual_headers: i16,
    pub geometry: GeometryProvenance,
    pub regularization: Option<RegularizationProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegyExportDescriptor {
    pub schema_version: u32,
    pub text_headers_path: String,
    pub binary_header_path: String,
    pub trace_headers_path: String,
    pub trace_index_path: String,
    pub trace_count: usize,
    pub textual_header_count: usize,
    pub endianness: String,
    pub contains_synthetic_traces: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAxes {
    pub ilines: Vec<f64>,
    pub xlines: Vec<f64>,
    #[serde(default = "default_sample_axis_domain")]
    pub sample_axis_domain: TimeDepthDomain,
    #[serde(default = "default_sample_axis_unit")]
    pub sample_axis_unit: String,
    pub sample_axis_ms: Vec<f32>,
}

impl VolumeAxes {
    pub fn from_time_axis(ilines: Vec<f64>, xlines: Vec<f64>, sample_axis_ms: Vec<f32>) -> Self {
        Self {
            ilines,
            xlines,
            sample_axis_domain: default_sample_axis_domain(),
            sample_axis_unit: default_sample_axis_unit(),
            sample_axis_ms,
        }
    }

    pub fn with_vertical_axis(
        ilines: Vec<f64>,
        xlines: Vec<f64>,
        sample_axis_domain: TimeDepthDomain,
        sample_axis_unit: String,
        sample_axis_ms: Vec<f32>,
    ) -> Self {
        Self {
            ilines,
            xlines,
            sample_axis_domain,
            sample_axis_unit,
            sample_axis_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedFrom {
    pub parent_store: PathBuf,
    pub method: InterpMethod,
    pub scale: u8,
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
    pub parent_store_id: String,
    pub artifact_role: ProcessingArtifactRole,
    pub pipeline: ProcessingPipelineSpec,
    pub runtime_version: String,
    pub created_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetadata {
    pub kind: DatasetKind,
    #[serde(default)]
    pub store_id: String,
    pub source: SourceIdentity,
    pub shape: [usize; 3],
    pub axes: VolumeAxes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segy_export: Option<SegyExportDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_binding: Option<CoordinateReferenceBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SurveySpatialDescriptor>,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_lineage: Option<ProcessingLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreManifest {
    pub version: u32,
    #[serde(default)]
    pub store_id: String,
    pub kind: DatasetKind,
    pub source: SourceIdentity,
    pub shape: [usize; 3],
    pub chunk_shape: [usize; 3],
    pub axes: VolumeAxes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segy_export: Option<SegyExportDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_binding: Option<CoordinateReferenceBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SurveySpatialDescriptor>,
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
            store_id: value.store_id.clone(),
            source: value.source.clone(),
            shape: value.shape,
            axes: value.axes.clone(),
            segy_export: value.segy_export.clone(),
            coordinate_reference_binding: value.coordinate_reference_binding.clone(),
            spatial: value.spatial.clone(),
            created_by: value.created_by.clone(),
            processing_lineage: value.processing_lineage.clone(),
        }
    }
}

pub fn generate_store_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn segy_sample_data_fidelity(sample_format_code: u16) -> SampleDataFidelity {
    match SampleFormat::from_code(sample_format_code) {
        Some(SampleFormat::IeeeFloat32) => SampleDataFidelity {
            source_sample_type: "ieee_float32".to_string(),
            working_sample_type: "f32".to_string(),
            conversion: SampleDataConversionKind::Identity,
            preservation: SampleValuePreservation::Exact,
            notes: vec![
                "SEG-Y IEEE float32 samples are stored in the working volume without numeric narrowing."
                    .to_string(),
            ],
        },
        Some(SampleFormat::IbmFloat32) => SampleDataFidelity {
            source_sample_type: "ibm_float32".to_string(),
            working_sample_type: "f32".to_string(),
            conversion: SampleDataConversionKind::FormatTranscode,
            preservation: SampleValuePreservation::PotentiallyLossy,
            notes: vec![
                "SEG-Y IBM float32 samples are transcoded into IEEE f32 for the working store."
                    .to_string(),
            ],
        },
        Some(SampleFormat::Int16) => exact_cast_fidelity("int16"),
        Some(SampleFormat::UInt16) => exact_cast_fidelity("uint16"),
        Some(SampleFormat::Int24) => exact_cast_fidelity("int24"),
        Some(SampleFormat::UInt24) => exact_cast_fidelity("uint24"),
        Some(SampleFormat::Int8) => exact_cast_fidelity("int8"),
        Some(SampleFormat::UInt8) => exact_cast_fidelity("uint8"),
        Some(SampleFormat::Int32) => potentially_lossy_cast_fidelity(
            "int32",
            "Large 32-bit integers can exceed the exact integer range of IEEE f32.",
        ),
        Some(SampleFormat::UInt32) => potentially_lossy_cast_fidelity(
            "uint32",
            "Large 32-bit integers can exceed the exact integer range of IEEE f32.",
        ),
        Some(SampleFormat::Int64) => potentially_lossy_cast_fidelity(
            "int64",
            "64-bit integers are generally not exactly representable in IEEE f32.",
        ),
        Some(SampleFormat::UInt64) => potentially_lossy_cast_fidelity(
            "uint64",
            "64-bit integers are generally not exactly representable in IEEE f32.",
        ),
        Some(SampleFormat::IeeeFloat64) => potentially_lossy_cast_fidelity(
            "ieee_float64",
            "IEEE float64 samples are narrowed to IEEE f32 for the working store.",
        ),
        Some(SampleFormat::FixedPoint32) => potentially_lossy_cast_fidelity(
            "fixed_point32",
            "Fixed-point 32-bit samples do not map exactly to the working IEEE f32 representation.",
        ),
        None => SampleDataFidelity {
            source_sample_type: format!("unknown_code_{sample_format_code}"),
            working_sample_type: "f32".to_string(),
            conversion: SampleDataConversionKind::Cast,
            preservation: SampleValuePreservation::PotentiallyLossy,
            notes: vec![format!(
                "SEG-Y sample format code {sample_format_code} is not recognized; fidelity to the working f32 store could not be classified."
            )],
        },
    }
}

pub fn normalize_source_identity(source: &mut SourceIdentity) -> bool {
    let needs_update = source.sample_data_fidelity.source_sample_type == "unknown"
        || source
            .sample_data_fidelity
            .working_sample_type
            .trim()
            .is_empty();
    if needs_update {
        source.sample_data_fidelity = segy_sample_data_fidelity(source.sample_format_code);
    }
    needs_update
}

pub fn normalize_volume_axes(axes: &mut VolumeAxes) -> bool {
    let mut changed = false;
    if axes.sample_axis_unit.trim().is_empty() {
        axes.sample_axis_unit = default_sample_axis_unit_for_domain(axes.sample_axis_domain);
        changed = true;
    }
    changed
}

pub fn validate_vertical_axis(
    values: &[f32],
    expected_count: usize,
    axis_label: &str,
) -> Result<(), String> {
    if values.len() != expected_count {
        return Err(format!(
            "{axis_label} length mismatch: expected {expected_count}, found {}",
            values.len()
        ));
    }
    if values.is_empty() {
        return Err(format!(
            "{axis_label} must contain at least one sample coordinate"
        ));
    }
    for (index, value) in values.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(format!(
                "{axis_label} contains non-finite coordinate at sample index {index}"
            ));
        }
        if index > 0 && value <= values[index - 1] {
            return Err(format!(
                "{axis_label} must be strictly increasing; sample index {index} has {value} after {}",
                values[index - 1]
            ));
        }
    }
    Ok(())
}

fn default_source_endianness() -> String {
    "big".to_string()
}

fn default_fixed_length_trace_flag_raw() -> u16 {
    1
}

fn default_sample_axis_domain() -> TimeDepthDomain {
    TimeDepthDomain::Time
}

fn default_sample_axis_unit() -> String {
    "ms".to_string()
}

pub fn default_sample_axis_unit_for_domain(domain: TimeDepthDomain) -> String {
    match domain {
        TimeDepthDomain::Time => "ms".to_string(),
        TimeDepthDomain::Depth => "m".to_string(),
    }
}

fn exact_cast_fidelity(source_sample_type: &str) -> SampleDataFidelity {
    SampleDataFidelity {
        source_sample_type: source_sample_type.to_string(),
        working_sample_type: "f32".to_string(),
        conversion: SampleDataConversionKind::Cast,
        preservation: SampleValuePreservation::Exact,
        notes: vec![
            "This source sample type is exactly representable in the working IEEE f32 store."
                .to_string(),
        ],
    }
}

fn potentially_lossy_cast_fidelity(source_sample_type: &str, note: &str) -> SampleDataFidelity {
    SampleDataFidelity {
        source_sample_type: source_sample_type.to_string(),
        working_sample_type: "f32".to_string(),
        conversion: SampleDataConversionKind::Cast,
        preservation: SampleValuePreservation::PotentiallyLossy,
        notes: vec![note.to_string()],
    }
}
