mod compute;
mod error;
mod ingest;
mod metadata;
mod preflight;
mod render;
mod storage;
mod store;

pub use compute::{
    MaterializeOptions, amplitude_spectrum_from_plane, amplitude_spectrum_from_reader,
    apply_pipeline_to_plane, apply_pipeline_to_traces,
    materialize_from_reader_writer, materialize_from_reader_writer_with_progress,
    materialize_processing_volume, materialize_processing_volume_with_progress, materialize_volume,
    preview_processing_section_plane, preview_processing_section_view, preview_section_from_reader,
    preview_section_plane, preview_section_view, validate_pipeline, validate_pipeline_for_layout,
    validate_processing_pipeline, validate_processing_pipeline_for_layout,
};
pub use error::SeismicStoreError;
pub use ingest::{
    IngestOptions, SeisGeometryOptions, SourceVolume, SparseSurveyPolicy, ingest_segy,
    load_source_volume, load_source_volume_with_options, recommended_chunk_shape,
};
pub use metadata::{
    CompressionKind, DatasetKind, GeometryProvenance, HeaderFieldSpec, InterpMethod,
    ProcessingLineage, RegularizationProvenance, SourceIdentity, StorageLayout, StoreManifest,
    VolumeAxes, VolumeMetadata,
};
pub use ophiolite_seismic::{
    AmplitudeSpectrumCurve, AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, AxisSummaryF32,
    AxisSummaryI32, CancelProcessingJobRequest, CancelProcessingJobResponse, DatasetId,
    DeletePipelinePresetRequest, DeletePipelinePresetResponse, FrequencyPhaseMode,
    FrequencyWindowShape, GeometryDescriptor, GeometryProvenanceSummary, GeometrySummary,
    GetProcessingJobRequest, GetProcessingJobResponse, InterpretationPoint,
    ListPipelinePresetsResponse, PreviewProcessingRequest, PreviewProcessingResponse,
    PreviewResponse, PreviewView, ProcessingJobProgress, ProcessingJobState,
    ProcessingJobStatus, ProcessingLayoutCompatibility, ProcessingOperation, ProcessingPipeline,
    ProcessingPreset, RunProcessingRequest, RunProcessingResponse, SavePipelinePresetRequest,
    SavePipelinePresetResponse, SectionAxis, SectionCoordinate, SectionDisplayDefaults,
    SectionMetadata, SectionProbe, SectionProbeChanged, SectionRenderMode, SectionRequest,
    SectionTileRequest, SectionUnits, SectionView, SectionViewport, SectionViewportChanged,
    SectionSpectrumSelection, SeismicLayout, VolumeDescriptor,
};
pub use preflight::{PreflightAction, PreflightGeometry, SurveyPreflight, preflight_segy};
pub use render::{render_section_csv, render_section_csv_for_request};
pub use storage::section_assembler::read_section_plane as assemble_section_plane;
pub use storage::tbvol::{TbvolManifest, TbvolReader, TbvolWriter, recommended_tbvol_tile_shape};
pub use storage::tile_geometry::{TileCoord, TileGeometry};
pub use storage::volume_store::{
    OccupancyTile, TileBuffer, VolumeStoreReader, VolumeStoreWriter, write_dense_volume,
};
pub use storage::zarr::{ZarrVolumeStoreReader, ZarrVolumeStoreWriter};
pub use store::{
    SectionPlane, StoreHandle, create_tbvol_store, describe_store, load_array, load_occupancy,
    open_store, read_section_plane, section_view,
};

use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SegyInspection {
    pub file_size: u64,
    pub trace_count: u64,
    pub samples_per_trace: u16,
    pub sample_interval_us: u16,
    pub sample_format_code: u16,
    pub fixed_length_trace: Option<bool>,
    pub endianness: String,
    pub warnings: Vec<String>,
}

pub fn inspect_segy(path: impl AsRef<Path>) -> Result<SegyInspection, SeismicStoreError> {
    let summary = ophiolite_seismic_io::inspect_file(path)?;
    Ok(SegyInspection {
        file_size: summary.file_size,
        trace_count: summary.trace_count,
        samples_per_trace: summary.samples_per_trace,
        sample_interval_us: summary.sample_interval_us,
        sample_format_code: summary.sample_format_code,
        fixed_length_trace: summary.fixed_length_trace,
        endianness: format!("{:?}", summary.endianness),
        warnings: summary
            .warnings
            .iter()
            .map(|warning| format!("{warning:?}"))
            .collect(),
    })
}
