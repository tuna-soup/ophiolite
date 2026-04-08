use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex, num_complex::Complex32};

use crate::error::SeismicStoreError;
use crate::metadata::{DatasetKind, ProcessingLineage, VolumeMetadata};
use crate::storage::section_assembler;
use crate::storage::tbvol::{
    TbvolReader, TbvolWriter, recommended_default_tbvol_tile_target_mib,
    recommended_tbvol_tile_shape,
};
use crate::storage::tile_geometry::TileCoord;
use crate::storage::volume_store::{VolumeStoreReader, VolumeStoreWriter};
use crate::store::{SectionPlane, StoreHandle, open_store};
use crate::{
    AmplitudeSpectrumCurve, FrequencyPhaseMode, FrequencyWindowShape, ProcessingOperation,
    ProcessingPipeline, ProcessingPipelineSpec, SectionAxis, SectionSpectrumSelection, SectionView,
    SeismicLayout, TraceLocalVolumeArithmeticOperator,
};

const MAX_SCALAR_FACTOR: f32 = 10.0;
const MAX_PHASE_ROTATION_DEGREES: f32 = 180.0;
const MAX_AGC_WINDOW_MS: f32 = 10_000.0;
const RMS_EPSILON: f32 = 1.0e-8;
const MAX_RMS_GAIN: f32 = 1.0e6;
const SPECTRUM_EPSILON: f32 = 1.0e-12;
const DIVISION_EPSILON: f32 = 1.0e-8;
const RUNTIME_VERSION: &str = "ophiolite-seismic-runtime-0.1.0";

#[derive(Debug, Clone)]
pub struct MaterializeOptions {
    pub chunk_shape: [usize; 3],
    pub created_by: String,
}

impl Default for MaterializeOptions {
    fn default() -> Self {
        Self {
            chunk_shape: [0, 0, 0],
            created_by: RUNTIME_VERSION.to_string(),
        }
    }
}

#[derive(Debug)]
struct SecondaryTraceMatrix {
    amplitudes: Vec<f32>,
    occupancy: Option<Vec<u8>>,
}

pub fn validate_pipeline(pipeline: &[ProcessingOperation]) -> Result<(), SeismicStoreError> {
    validate_pipeline_for_layout(pipeline, SeismicLayout::PostStack3D)
}

pub fn validate_pipeline_for_layout(
    pipeline: &[ProcessingOperation],
    layout: SeismicLayout,
) -> Result<(), SeismicStoreError> {
    if pipeline.is_empty() {
        return Err(SeismicStoreError::Message(
            "processing pipeline must contain at least one operator".to_string(),
        ));
    }

    for operation in pipeline {
        if let ProcessingOperation::AmplitudeScalar { factor } = operation
            && (!(0.0..=MAX_SCALAR_FACTOR).contains(factor) || factor.is_nan())
        {
            return Err(SeismicStoreError::Message(format!(
                "amplitude scalar factor must be in [0.0, {MAX_SCALAR_FACTOR}], found {factor}"
            )));
        }

        if let ProcessingOperation::BandpassFilter {
            f1_hz,
            f2_hz,
            f3_hz,
            f4_hz,
            ..
        } = operation
        {
            validate_bandpass_corners(*f1_hz, *f2_hz, *f3_hz, *f4_hz)?;
        }

        if let ProcessingOperation::LowpassFilter { f3_hz, f4_hz, .. } = operation {
            validate_lowpass_corners(*f3_hz, *f4_hz)?;
        }

        if let ProcessingOperation::HighpassFilter { f1_hz, f2_hz, .. } = operation {
            validate_highpass_corners(*f1_hz, *f2_hz)?;
        }

        if let ProcessingOperation::AgcRms { window_ms } = operation {
            validate_agc_window(*window_ms)?;
        }

        if let ProcessingOperation::PhaseRotation { angle_degrees } = operation {
            validate_phase_rotation_angle(*angle_degrees)?;
        }

        if let ProcessingOperation::VolumeArithmetic {
            secondary_store_path,
            ..
        } = operation
        {
            validate_secondary_store_path(secondary_store_path)?;
        }

        let compatibility = operation.compatibility();
        if !compatibility.supports_layout(layout) {
            return Err(SeismicStoreError::Message(format!(
                "processing operator '{}' requires {}, found layout {:?}",
                operation.operator_id(),
                compatibility.label(),
                layout
            )));
        }
    }

    Ok(())
}

fn validate_secondary_store_path(secondary_store_path: &str) -> Result<(), SeismicStoreError> {
    if secondary_store_path.trim().is_empty() {
        return Err(SeismicStoreError::Message(
            "volume arithmetic secondary_store_path must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn pipeline_requires_external_volume_inputs(pipeline: &[ProcessingOperation]) -> bool {
    pipeline
        .iter()
        .any(|operation| matches!(operation, ProcessingOperation::VolumeArithmetic { .. }))
}

fn secondary_store_paths(pipeline: &[ProcessingOperation]) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for operation in pipeline {
        if let ProcessingOperation::VolumeArithmetic {
            secondary_store_path,
            ..
        } = operation
        {
            let trimmed = secondary_store_path.trim();
            if !trimmed.is_empty() {
                paths.insert(trimmed.to_string());
            }
        }
    }
    paths.into_iter().collect()
}

fn validate_secondary_store_compatibility(
    primary_handle: &StoreHandle,
    secondary_handle: &StoreHandle,
    secondary_store_path: &str,
) -> Result<(), SeismicStoreError> {
    let primary_geometry = primary_handle.volume_descriptor().geometry;
    let secondary_geometry = secondary_handle.volume_descriptor().geometry;
    if secondary_geometry.compare_family != primary_geometry.compare_family {
        return Err(SeismicStoreError::Message(format!(
            "volume arithmetic secondary store '{}' compare family '{}' does not match primary '{}'",
            secondary_store_path,
            secondary_geometry.compare_family,
            primary_geometry.compare_family
        )));
    }
    if secondary_geometry.fingerprint != primary_geometry.fingerprint {
        return Err(SeismicStoreError::Message(format!(
            "volume arithmetic secondary store '{}' fingerprint '{}' does not match primary '{}'",
            secondary_store_path, secondary_geometry.fingerprint, primary_geometry.fingerprint
        )));
    }
    Ok(())
}

fn open_secondary_store_readers(
    primary_handle: &StoreHandle,
    primary_tile_shape: Option<[usize; 3]>,
    pipeline: &[ProcessingOperation],
) -> Result<HashMap<String, TbvolReader>, SeismicStoreError> {
    let mut readers = HashMap::new();
    for secondary_store_path in secondary_store_paths(pipeline) {
        let secondary_handle = open_store(&secondary_store_path)?;
        validate_secondary_store_compatibility(
            primary_handle,
            &secondary_handle,
            &secondary_store_path,
        )?;
        let secondary_reader = TbvolReader::open(&secondary_handle.root)?;
        if let Some(tile_shape) = primary_tile_shape
            && secondary_reader.tile_geometry().tile_shape() != tile_shape
        {
            return Err(SeismicStoreError::Message(format!(
                "volume arithmetic secondary store '{}' tile shape {:?} does not match primary {:?}",
                secondary_store_path,
                secondary_reader.tile_geometry().tile_shape(),
                tile_shape
            )));
        }
        readers.insert(secondary_store_path, secondary_reader);
    }
    Ok(readers)
}

fn load_secondary_section_inputs(
    axis: SectionAxis,
    index: usize,
    readers: &HashMap<String, TbvolReader>,
) -> Result<HashMap<String, SecondaryTraceMatrix>, SeismicStoreError> {
    let mut inputs = HashMap::with_capacity(readers.len());
    for (store_path, reader) in readers {
        let plane = section_assembler::read_section_plane(reader, axis, index)?;
        inputs.insert(
            store_path.clone(),
            SecondaryTraceMatrix {
                amplitudes: plane.amplitudes,
                occupancy: plane.occupancy,
            },
        );
    }
    Ok(inputs)
}

fn load_secondary_tile_inputs(
    tile: TileCoord,
    readers: &HashMap<String, TbvolReader>,
) -> Result<HashMap<String, SecondaryTraceMatrix>, SeismicStoreError> {
    let mut inputs = HashMap::with_capacity(readers.len());
    for (store_path, reader) in readers {
        inputs.insert(
            store_path.clone(),
            SecondaryTraceMatrix {
                amplitudes: reader.read_tile(tile)?.into_owned(),
                occupancy: reader
                    .read_tile_occupancy(tile)?
                    .map(|occupancy| occupancy.into_owned()),
            },
        );
    }
    Ok(inputs)
}

pub fn validate_processing_pipeline(
    pipeline: &ProcessingPipeline,
) -> Result<(), SeismicStoreError> {
    validate_processing_pipeline_for_layout(pipeline, SeismicLayout::PostStack3D)
}

pub fn validate_processing_pipeline_for_layout(
    pipeline: &ProcessingPipeline,
    layout: SeismicLayout,
) -> Result<(), SeismicStoreError> {
    if pipeline.operations.is_empty() {
        return Err(SeismicStoreError::Message(
            "processing pipeline must contain at least one operator".to_string(),
        ));
    }
    validate_pipeline_for_layout(&pipeline.operations, layout)
}

fn validate_pipeline_for_sample_interval(
    pipeline: &[ProcessingOperation],
    sample_interval_ms: f32,
) -> Result<(), SeismicStoreError> {
    if !pipeline_requires_sample_interval(pipeline) {
        return Ok(());
    }

    let nyquist_hz = nyquist_hz_for_sample_interval_ms(sample_interval_ms)?;
    for operation in pipeline {
        match operation {
            ProcessingOperation::BandpassFilter { f4_hz, .. } if *f4_hz > nyquist_hz => {
                return Err(SeismicStoreError::Message(format!(
                    "bandpass high corner f4_hz must be <= Nyquist ({nyquist_hz:.3} Hz), found {f4_hz}"
                )));
            }
            ProcessingOperation::LowpassFilter { f4_hz, .. } if *f4_hz > nyquist_hz => {
                return Err(SeismicStoreError::Message(format!(
                    "lowpass high corner f4_hz must be <= Nyquist ({nyquist_hz:.3} Hz), found {f4_hz}"
                )));
            }
            ProcessingOperation::HighpassFilter { f2_hz, .. } if *f2_hz > nyquist_hz => {
                return Err(SeismicStoreError::Message(format!(
                    "highpass pass corner f2_hz must be <= Nyquist ({nyquist_hz:.3} Hz), found {f2_hz}"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

fn pipeline_requires_sample_interval(pipeline: &[ProcessingOperation]) -> bool {
    pipeline.iter().any(|operation| {
        matches!(
            operation,
            ProcessingOperation::AgcRms { .. }
                | ProcessingOperation::LowpassFilter { .. }
                | ProcessingOperation::HighpassFilter { .. }
                | ProcessingOperation::BandpassFilter { .. }
        )
    })
}

fn pipeline_requires_spectral_workspace(pipeline: &[ProcessingOperation]) -> bool {
    pipeline.iter().any(|operation| {
        matches!(
            operation,
            ProcessingOperation::PhaseRotation { .. }
                | ProcessingOperation::LowpassFilter { .. }
                | ProcessingOperation::HighpassFilter { .. }
                | ProcessingOperation::BandpassFilter { .. }
        )
    })
}

fn validate_bandpass_corners(
    f1_hz: f32,
    f2_hz: f32,
    f3_hz: f32,
    f4_hz: f32,
) -> Result<(), SeismicStoreError> {
    for (label, value) in [
        ("f1_hz", f1_hz),
        ("f2_hz", f2_hz),
        ("f3_hz", f3_hz),
        ("f4_hz", f4_hz),
    ] {
        if !value.is_finite() {
            return Err(SeismicStoreError::Message(format!(
                "bandpass corner {label} must be finite, found {value}"
            )));
        }
    }

    if f1_hz < 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "bandpass corner f1_hz must be >= 0.0, found {f1_hz}"
        )));
    }

    if !(f1_hz <= f2_hz && f2_hz <= f3_hz && f3_hz <= f4_hz) {
        return Err(SeismicStoreError::Message(format!(
            "bandpass corners must satisfy f1 <= f2 <= f3 <= f4, found [{f1_hz}, {f2_hz}, {f3_hz}, {f4_hz}]"
        )));
    }

    Ok(())
}

fn validate_lowpass_corners(f3_hz: f32, f4_hz: f32) -> Result<(), SeismicStoreError> {
    for (label, value) in [("f3_hz", f3_hz), ("f4_hz", f4_hz)] {
        if !value.is_finite() {
            return Err(SeismicStoreError::Message(format!(
                "lowpass corner {label} must be finite, found {value}"
            )));
        }
    }

    if f3_hz < 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "lowpass corner f3_hz must be >= 0.0, found {f3_hz}"
        )));
    }

    if f3_hz > f4_hz {
        return Err(SeismicStoreError::Message(format!(
            "lowpass corners must satisfy f3 <= f4, found [{f3_hz}, {f4_hz}]"
        )));
    }

    Ok(())
}

fn validate_highpass_corners(f1_hz: f32, f2_hz: f32) -> Result<(), SeismicStoreError> {
    for (label, value) in [("f1_hz", f1_hz), ("f2_hz", f2_hz)] {
        if !value.is_finite() {
            return Err(SeismicStoreError::Message(format!(
                "highpass corner {label} must be finite, found {value}"
            )));
        }
    }

    if f1_hz < 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "highpass corner f1_hz must be >= 0.0, found {f1_hz}"
        )));
    }

    if f1_hz > f2_hz {
        return Err(SeismicStoreError::Message(format!(
            "highpass corners must satisfy f1 <= f2, found [{f1_hz}, {f2_hz}]"
        )));
    }

    Ok(())
}

fn validate_agc_window(window_ms: f32) -> Result<(), SeismicStoreError> {
    if !window_ms.is_finite() {
        return Err(SeismicStoreError::Message(format!(
            "AGC window_ms must be finite, found {window_ms}"
        )));
    }

    if !(0.0..=MAX_AGC_WINDOW_MS).contains(&window_ms) || window_ms <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "AGC window_ms must be in (0.0, {MAX_AGC_WINDOW_MS}], found {window_ms}"
        )));
    }

    Ok(())
}

fn validate_phase_rotation_angle(angle_degrees: f32) -> Result<(), SeismicStoreError> {
    if !angle_degrees.is_finite() {
        return Err(SeismicStoreError::Message(format!(
            "phase rotation angle_degrees must be finite, found {angle_degrees}"
        )));
    }

    if !(-MAX_PHASE_ROTATION_DEGREES..=MAX_PHASE_ROTATION_DEGREES).contains(&angle_degrees) {
        return Err(SeismicStoreError::Message(format!(
            "phase rotation angle_degrees must be in [-{MAX_PHASE_ROTATION_DEGREES}, {MAX_PHASE_ROTATION_DEGREES}], found {angle_degrees}"
        )));
    }

    Ok(())
}

fn nyquist_hz_for_sample_interval_ms(sample_interval_ms: f32) -> Result<f32, SeismicStoreError> {
    if !sample_interval_ms.is_finite() || sample_interval_ms <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "sample interval must be finite and > 0 ms, found {sample_interval_ms}"
        )));
    }

    Ok(500.0 / sample_interval_ms)
}

pub fn preview_section_plane(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
) -> Result<SectionPlane, SeismicStoreError> {
    let handle = open_store(store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let secondary_readers = open_secondary_store_readers(&handle, None, pipeline)?;
    preview_section_from_tbvol_reader(&reader, axis, index, pipeline, &secondary_readers)
}

pub fn preview_processing_section_plane(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &ProcessingPipeline,
) -> Result<SectionPlane, SeismicStoreError> {
    validate_processing_pipeline(pipeline)?;
    preview_section_plane(store_root, axis, index, &pipeline.operations)
}

pub fn preview_section_view(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
) -> Result<SectionView, SeismicStoreError> {
    validate_pipeline(pipeline)?;
    let handle = open_store(store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let secondary_readers = open_secondary_store_readers(&handle, None, pipeline)?;
    let plane =
        preview_section_from_tbvol_reader(&reader, axis, index, pipeline, &secondary_readers)?;
    Ok(handle.section_view_from_plane(&plane))
}

pub fn preview_processing_section_view(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &ProcessingPipeline,
) -> Result<SectionView, SeismicStoreError> {
    validate_processing_pipeline(pipeline)?;
    preview_section_view(store_root, axis, index, &pipeline.operations)
}

pub fn materialize_volume(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &[ProcessingOperation],
    options: MaterializeOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    validate_pipeline(pipeline)?;
    let pipeline_spec = pipeline_from_operations(pipeline);
    let handle = open_store(&input_store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let volume = derived_volume_metadata(
        reader.volume(),
        input_store_root.as_ref(),
        &pipeline_spec,
        options.created_by.clone(),
    );
    let chunk_shape = resolve_chunk_shape(options.chunk_shape, volume.shape);
    let has_occupancy = reader_has_occupancy(&reader)?;
    let writer = TbvolWriter::create(&output_store_root, volume, chunk_shape, has_occupancy)?;
    let output_root = writer.root().to_path_buf();
    let secondary_readers =
        open_secondary_store_readers(&handle, Some(reader.tile_geometry().tile_shape()), pipeline)?;
    materialize_from_tbvol_reader_writer_with_progress(
        &reader,
        writer,
        pipeline,
        &secondary_readers,
        |_, _| Ok(()),
    )?;
    open_store(output_root)
}

pub fn materialize_processing_volume(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &ProcessingPipeline,
    options: MaterializeOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    materialize_processing_volume_with_progress(
        input_store_root,
        output_store_root,
        pipeline,
        options,
        |_, _| Ok(()),
    )
}

pub fn materialize_processing_volume_with_progress<
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &ProcessingPipeline,
    options: MaterializeOptions,
    mut on_progress: F,
) -> Result<StoreHandle, SeismicStoreError> {
    validate_processing_pipeline(pipeline)?;
    let handle = open_store(&input_store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let volume = derived_volume_metadata(
        reader.volume(),
        input_store_root.as_ref(),
        pipeline,
        options.created_by.clone(),
    );
    let chunk_shape = resolve_chunk_shape(options.chunk_shape, volume.shape);
    let has_occupancy = reader_has_occupancy(&reader)?;
    let writer = TbvolWriter::create(&output_store_root, volume, chunk_shape, has_occupancy)?;
    let output_root = writer.root().to_path_buf();
    let secondary_readers = open_secondary_store_readers(
        &handle,
        Some(reader.tile_geometry().tile_shape()),
        &pipeline.operations,
    )?;
    materialize_from_tbvol_reader_writer_with_progress(
        &reader,
        writer,
        &pipeline.operations,
        &secondary_readers,
        |completed, total| on_progress(completed, total),
    )?;
    open_store(output_root)
}

pub fn preview_section_from_reader<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
) -> Result<SectionPlane, SeismicStoreError> {
    validate_pipeline(pipeline)?;
    if pipeline_requires_external_volume_inputs(pipeline) {
        return Err(SeismicStoreError::Message(
            "volume arithmetic preview requires a store-backed preview path".to_string(),
        ));
    }
    validate_pipeline_for_sample_interval(
        pipeline,
        reader.volume().source.sample_interval_us as f32 / 1000.0,
    )?;
    let mut plane = section_assembler::read_section_plane(reader, axis, index)?;
    apply_pipeline_to_plane(&mut plane, pipeline)?;
    Ok(plane)
}

pub fn materialize_from_reader_writer<R: VolumeStoreReader, W: VolumeStoreWriter>(
    reader: &R,
    writer: W,
    pipeline: &[ProcessingOperation],
) -> Result<(), SeismicStoreError> {
    materialize_from_reader_writer_with_progress(reader, writer, pipeline, |_, _| Ok(()))
}

pub fn materialize_from_reader_writer_with_progress<
    R: VolumeStoreReader,
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    reader: &R,
    writer: W,
    pipeline: &[ProcessingOperation],
    mut on_progress: F,
) -> Result<(), SeismicStoreError> {
    if pipeline_requires_external_volume_inputs(pipeline) {
        return Err(SeismicStoreError::Message(
            "volume arithmetic materialization requires a store-backed materialization path"
                .to_string(),
        ));
    }
    materialize_from_reader_writer_internal(reader, writer, pipeline, None, &mut on_progress)
}

pub fn apply_pipeline_to_plane(
    plane: &mut SectionPlane,
    pipeline: &[ProcessingOperation],
) -> Result<(), SeismicStoreError> {
    validate_pipeline(pipeline)?;
    if pipeline_requires_external_volume_inputs(pipeline) {
        return Err(SeismicStoreError::Message(
            "volume arithmetic requires store-backed secondary inputs".to_string(),
        ));
    }
    let sample_interval_ms = sample_interval_ms_from_axis(&plane.sample_axis_ms)?;
    validate_pipeline_for_sample_interval(pipeline, sample_interval_ms)?;
    apply_pipeline_to_traces_internal(
        &mut plane.amplitudes,
        plane.traces,
        plane.samples,
        sample_interval_ms,
        plane.occupancy.as_deref(),
        pipeline,
        None,
    )
}

pub fn apply_pipeline_to_traces(
    data: &mut [f32],
    traces: usize,
    samples: usize,
    sample_interval_ms: f32,
    occupancy: Option<&[u8]>,
    pipeline: &[ProcessingOperation],
) -> Result<(), SeismicStoreError> {
    if traces == 0 || samples == 0 || data.is_empty() || pipeline.is_empty() {
        return Ok(());
    }

    validate_pipeline(pipeline)?;
    if pipeline_requires_external_volume_inputs(pipeline) {
        return Err(SeismicStoreError::Message(
            "volume arithmetic requires store-backed secondary inputs".to_string(),
        ));
    }
    apply_pipeline_to_traces_internal(
        data,
        traces,
        samples,
        sample_interval_ms,
        occupancy,
        pipeline,
        None,
    )
}

fn preview_section_from_tbvol_reader(
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
) -> Result<SectionPlane, SeismicStoreError> {
    validate_pipeline(pipeline)?;
    let mut plane = section_assembler::read_section_plane(reader, axis, index)?;
    let sample_interval_ms = sample_interval_ms_from_axis(&plane.sample_axis_ms)?;
    let secondary_inputs = if pipeline_requires_external_volume_inputs(pipeline) {
        Some(load_secondary_section_inputs(
            axis,
            index,
            secondary_readers,
        )?)
    } else {
        None
    };
    apply_pipeline_to_traces_internal(
        &mut plane.amplitudes,
        plane.traces,
        plane.samples,
        sample_interval_ms,
        plane.occupancy.as_deref(),
        pipeline,
        secondary_inputs.as_ref(),
    )?;
    Ok(plane)
}

fn materialize_from_tbvol_reader_writer_with_progress<
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    reader: &TbvolReader,
    writer: W,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
    mut on_progress: F,
) -> Result<(), SeismicStoreError> {
    materialize_from_reader_writer_internal(
        reader,
        writer,
        pipeline,
        Some(secondary_readers),
        &mut on_progress,
    )
}

fn materialize_from_reader_writer_internal<
    R: VolumeStoreReader,
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    reader: &R,
    writer: W,
    pipeline: &[ProcessingOperation],
    secondary_readers: Option<&HashMap<String, TbvolReader>>,
    on_progress: &mut F,
) -> Result<(), SeismicStoreError> {
    validate_pipeline(pipeline)?;
    if reader.tile_geometry().tile_shape() != writer.tile_geometry().tile_shape()
        || reader.volume().shape != writer.volume().shape
    {
        return Err(SeismicStoreError::Message(
            "reader and writer geometry mismatch".to_string(),
        ));
    }

    let tile_shape = reader.tile_geometry().tile_shape();
    let traces = tile_shape[0] * tile_shape[1];
    let samples = tile_shape[2];
    let sample_interval_ms = reader.volume().source.sample_interval_us as f32 / 1000.0;
    validate_pipeline_for_sample_interval(pipeline, sample_interval_ms)?;
    let total_tiles = reader.tile_geometry().tile_count();
    let mut completed_tiles = 0;
    for tile in reader.tile_geometry().iter_tiles() {
        let mut amplitudes = reader.read_tile(tile)?.into_owned();
        let occupancy = reader
            .read_tile_occupancy(tile)?
            .map(|value| value.into_owned());
        let secondary_inputs = match secondary_readers {
            Some(readers) if pipeline_requires_external_volume_inputs(pipeline) => {
                Some(load_secondary_tile_inputs(tile, readers)?)
            }
            _ => None,
        };
        apply_pipeline_to_traces_internal(
            &mut amplitudes,
            traces,
            samples,
            sample_interval_ms,
            occupancy.as_deref(),
            pipeline,
            secondary_inputs.as_ref(),
        )?;
        writer.write_tile(tile, &amplitudes)?;
        if let Some(mask) = occupancy.as_deref() {
            writer.write_tile_occupancy(tile, mask)?;
        }
        completed_tiles += 1;
        on_progress(completed_tiles, total_tiles)?;
    }
    writer.finalize()
}

fn apply_pipeline_to_traces_internal(
    data: &mut [f32],
    traces: usize,
    samples: usize,
    sample_interval_ms: f32,
    occupancy: Option<&[u8]>,
    pipeline: &[ProcessingOperation],
    secondary_inputs: Option<&HashMap<String, SecondaryTraceMatrix>>,
) -> Result<(), SeismicStoreError> {
    if traces == 0 || samples == 0 || data.is_empty() || pipeline.is_empty() {
        return Ok(());
    }

    validate_pipeline(pipeline)?;
    validate_pipeline_for_sample_interval(pipeline, sample_interval_ms)?;
    if pipeline_requires_external_volume_inputs(pipeline) && secondary_inputs.is_none() {
        return Err(SeismicStoreError::Message(
            "volume arithmetic requires secondary trace matrices".to_string(),
        ));
    }
    let needs_spectral = pipeline_requires_spectral_workspace(pipeline);

    compute_pool().install(|| {
        data.par_chunks_mut(samples).enumerate().try_for_each_init(
            || TraceComputeState::new(samples, needs_spectral),
            |state, (trace_index, trace)| {
                if trace_index >= traces {
                    return Ok(());
                }
                if occupancy.is_some_and(|mask| mask.get(trace_index).copied().unwrap_or(1) == 0) {
                    return Ok(());
                }
                for operation in pipeline {
                    apply_operation_to_trace(
                        trace,
                        trace_index,
                        samples,
                        sample_interval_ms,
                        state,
                        operation,
                        secondary_inputs,
                    )?;
                }
                Ok(())
            },
        )
    })
}

fn apply_operation_to_trace(
    trace: &mut [f32],
    trace_index: usize,
    samples: usize,
    sample_interval_ms: f32,
    state: &mut TraceComputeState,
    operation: &ProcessingOperation,
    secondary_inputs: Option<&HashMap<String, SecondaryTraceMatrix>>,
) -> Result<(), SeismicStoreError> {
    match operation {
        ProcessingOperation::AmplitudeScalar { factor } => {
            for sample in trace.iter_mut() {
                *sample *= *factor;
            }
        }
        ProcessingOperation::TraceRmsNormalize => {
            let rms = (trace
                .iter()
                .map(|value| f64::from(*value) * f64::from(*value))
                .sum::<f64>()
                / trace.len().max(1) as f64)
                .sqrt() as f32;
            let gain = (1.0 / rms.max(RMS_EPSILON)).min(MAX_RMS_GAIN);
            for sample in trace.iter_mut() {
                *sample *= gain;
            }
        }
        ProcessingOperation::AgcRms { window_ms } => {
            state.apply_agc_rms(trace, sample_interval_ms, *window_ms)?;
        }
        ProcessingOperation::PhaseRotation { angle_degrees } => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_phase_rotation(trace, *angle_degrees)?;
        }
        ProcessingOperation::LowpassFilter {
            f3_hz,
            f4_hz,
            phase,
            window,
        } => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_lowpass(trace, sample_interval_ms, *f3_hz, *f4_hz, *phase, *window)?;
        }
        ProcessingOperation::HighpassFilter {
            f1_hz,
            f2_hz,
            phase,
            window,
        } => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_highpass(trace, sample_interval_ms, *f1_hz, *f2_hz, *phase, *window)?;
        }
        ProcessingOperation::BandpassFilter {
            f1_hz,
            f2_hz,
            f3_hz,
            f4_hz,
            phase,
            window,
        } => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_bandpass(
                    trace,
                    sample_interval_ms,
                    *f1_hz,
                    *f2_hz,
                    *f3_hz,
                    *f4_hz,
                    *phase,
                    *window,
                )?;
        }
        ProcessingOperation::VolumeArithmetic {
            operator,
            secondary_store_path,
        } => {
            let secondary_inputs = secondary_inputs.ok_or_else(|| {
                SeismicStoreError::Message(
                    "volume arithmetic requires secondary trace matrices".to_string(),
                )
            })?;
            let secondary = secondary_inputs.get(secondary_store_path).ok_or_else(|| {
                SeismicStoreError::Message(format!(
                    "volume arithmetic secondary input '{}' was not loaded",
                    secondary_store_path
                ))
            })?;
            let secondary_trace = secondary.trace_slice(trace_index, samples)?;
            apply_volume_arithmetic(trace, secondary_trace, *operator);
        }
    }

    Ok(())
}

impl SecondaryTraceMatrix {
    fn trace_slice(
        &self,
        trace_index: usize,
        samples: usize,
    ) -> Result<Option<&[f32]>, SeismicStoreError> {
        if self
            .occupancy
            .as_deref()
            .is_some_and(|mask| mask.get(trace_index).copied().unwrap_or(1) == 0)
        {
            return Ok(None);
        }

        let start = trace_index
            .checked_mul(samples)
            .ok_or_else(|| SeismicStoreError::Message("trace offset overflow".to_string()))?;
        let end = start
            .checked_add(samples)
            .ok_or_else(|| SeismicStoreError::Message("trace slice overflow".to_string()))?;
        if end > self.amplitudes.len() {
            return Err(SeismicStoreError::Message(format!(
                "secondary trace matrix length {} is smaller than required end offset {}",
                self.amplitudes.len(),
                end
            )));
        }
        Ok(Some(&self.amplitudes[start..end]))
    }
}

fn apply_volume_arithmetic(
    trace: &mut [f32],
    secondary_trace: Option<&[f32]>,
    operator: TraceLocalVolumeArithmeticOperator,
) {
    match (operator, secondary_trace) {
        (TraceLocalVolumeArithmeticOperator::Add, Some(secondary_trace)) => {
            for (sample, other) in trace.iter_mut().zip(secondary_trace.iter()) {
                *sample += *other;
            }
        }
        (TraceLocalVolumeArithmeticOperator::Subtract, Some(secondary_trace)) => {
            for (sample, other) in trace.iter_mut().zip(secondary_trace.iter()) {
                *sample -= *other;
            }
        }
        (TraceLocalVolumeArithmeticOperator::Multiply, Some(secondary_trace)) => {
            for (sample, other) in trace.iter_mut().zip(secondary_trace.iter()) {
                *sample *= *other;
            }
        }
        (TraceLocalVolumeArithmeticOperator::Divide, Some(secondary_trace)) => {
            for (sample, other) in trace.iter_mut().zip(secondary_trace.iter()) {
                *sample = if other.abs() <= DIVISION_EPSILON {
                    0.0
                } else {
                    *sample / *other
                };
            }
        }
        (TraceLocalVolumeArithmeticOperator::Multiply, None)
        | (TraceLocalVolumeArithmeticOperator::Divide, None) => {
            trace.fill(0.0);
        }
        (TraceLocalVolumeArithmeticOperator::Add, None)
        | (TraceLocalVolumeArithmeticOperator::Subtract, None) => {}
    }
}

pub fn amplitude_spectrum_from_plane(
    plane: &SectionPlane,
    selection: &SectionSpectrumSelection,
) -> Result<AmplitudeSpectrumCurve, SeismicStoreError> {
    if plane.samples == 0 {
        return Err(SeismicStoreError::Message(
            "cannot compute amplitude spectrum for an empty section".to_string(),
        ));
    }

    let sample_interval_ms = sample_interval_ms_from_axis(&plane.sample_axis_ms)?;
    let selected = resolve_spectrum_selection(selection, plane.traces, plane.samples)?;
    let selected_samples = selected.sample_end - selected.sample_start;
    let mut workspace = SpectrumWorkspace::new(selected_samples);
    let frequencies_hz = frequency_bins_hz(selected_samples, sample_interval_ms);
    let mut amplitudes = vec![0.0_f32; frequencies_hz.len()];
    let mut contributing_traces = 0usize;

    for trace_index in selected.trace_start..selected.trace_end {
        if plane
            .occupancy
            .as_deref()
            .is_some_and(|mask| mask.get(trace_index).copied().unwrap_or(1) == 0)
        {
            continue;
        }

        let start = trace_index * plane.samples;
        workspace.accumulate_trace_spectrum(
            &plane.amplitudes[start + selected.sample_start..start + selected.sample_end],
            sample_interval_ms,
            &mut amplitudes,
        )?;
        contributing_traces += 1;
    }

    if contributing_traces == 0 {
        return Err(SeismicStoreError::Message(
            "selected traces do not contain any occupied samples for spectrum analysis".to_string(),
        ));
    }

    let normalization = contributing_traces as f32;
    for value in &mut amplitudes {
        *value /= normalization;
    }

    Ok(AmplitudeSpectrumCurve {
        frequencies_hz,
        amplitudes,
    })
}

pub fn amplitude_spectrum_from_reader<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
    pipeline: Option<&[ProcessingOperation]>,
    selection: &SectionSpectrumSelection,
) -> Result<AmplitudeSpectrumCurve, SeismicStoreError> {
    let plane = match pipeline {
        Some(operations) if !operations.is_empty() => {
            preview_section_from_reader(reader, axis, index, operations)?
        }
        _ => section_assembler::read_section_plane(reader, axis, index)?,
    };
    amplitude_spectrum_from_plane(&plane, selection)
}

pub fn amplitude_spectrum_from_store(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: Option<&[ProcessingOperation]>,
    selection: &SectionSpectrumSelection,
) -> Result<AmplitudeSpectrumCurve, SeismicStoreError> {
    let handle = open_store(store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let operations = pipeline.unwrap_or(&[]);
    let secondary_readers = open_secondary_store_readers(&handle, None, operations)?;
    let plane = match pipeline {
        Some(operations) if !operations.is_empty() => {
            preview_section_from_tbvol_reader(&reader, axis, index, operations, &secondary_readers)?
        }
        _ => section_assembler::read_section_plane(&reader, axis, index)?,
    };
    amplitude_spectrum_from_plane(&plane, selection)
}

struct TraceComputeState {
    spectral: Option<SpectralWorkspace>,
    agc_output: Vec<f32>,
    agc_prefix_squares: Vec<f64>,
}

impl TraceComputeState {
    fn new(samples: usize, needs_spectral: bool) -> Self {
        Self {
            spectral: needs_spectral.then(|| SpectralWorkspace::new(samples)),
            agc_output: vec![0.0; samples],
            agc_prefix_squares: vec![0.0; samples + 1],
        }
    }

    fn apply_agc_rms(
        &mut self,
        trace: &mut [f32],
        sample_interval_ms: f32,
        window_ms: f32,
    ) -> Result<(), SeismicStoreError> {
        validate_agc_window(window_ms)?;
        let _ = nyquist_hz_for_sample_interval_ms(sample_interval_ms)?;

        if trace.len() != self.agc_output.len() || self.agc_prefix_squares.len() != trace.len() + 1
        {
            return Err(SeismicStoreError::Message(format!(
                "AGC workspace length mismatch: expected trace length {}, found {}",
                self.agc_output.len(),
                trace.len()
            )));
        }

        let window_samples = ((window_ms / sample_interval_ms).round() as usize).max(1);
        let half_window = window_samples / 2;
        self.agc_prefix_squares[0] = 0.0;
        for (index, sample) in trace.iter().enumerate() {
            self.agc_prefix_squares[index + 1] =
                self.agc_prefix_squares[index] + f64::from(*sample) * f64::from(*sample);
        }

        for (index, sample) in trace.iter().enumerate() {
            let start = index.saturating_sub(half_window);
            let end = (index + half_window + 1).min(trace.len());
            let energy = self.agc_prefix_squares[end] - self.agc_prefix_squares[start];
            let rms = (energy / (end - start).max(1) as f64).sqrt() as f32;
            let gain = (1.0 / rms.max(RMS_EPSILON)).min(MAX_RMS_GAIN);
            self.agc_output[index] = *sample * gain;
        }
        trace.copy_from_slice(&self.agc_output);

        Ok(())
    }
}

struct SpectralWorkspace {
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
    input: Vec<f32>,
    output: Vec<f32>,
    spectrum: Vec<Complex32>,
}

impl SpectralWorkspace {
    fn new(samples: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(samples);
        let inverse = planner.plan_fft_inverse(samples);
        let input = forward.make_input_vec();
        let output = inverse.make_output_vec();
        let spectrum = forward.make_output_vec();
        Self {
            forward,
            inverse,
            input,
            output,
            spectrum,
        }
    }

    fn apply_bandpass(
        &mut self,
        trace: &mut [f32],
        sample_interval_ms: f32,
        f1_hz: f32,
        f2_hz: f32,
        f3_hz: f32,
        f4_hz: f32,
        phase: FrequencyPhaseMode,
        window: FrequencyWindowShape,
    ) -> Result<(), SeismicStoreError> {
        if trace.len() != self.input.len() {
            return Err(SeismicStoreError::Message(format!(
                "bandpass workspace length mismatch: expected {}, found {}",
                self.input.len(),
                trace.len()
            )));
        }

        self.input.copy_from_slice(trace);
        remove_trace_mean(&mut self.input);

        self.forward
            .process(&mut self.input, &mut self.spectrum)
            .map_err(|error| {
                SeismicStoreError::Message(format!("bandpass forward FFT failed: {error}"))
            })?;
        apply_bandpass_response(
            &mut self.spectrum,
            trace.len(),
            sample_interval_ms,
            f1_hz,
            f2_hz,
            f3_hz,
            f4_hz,
            phase,
            window,
        )?;
        self.inverse
            .process(&mut self.spectrum, &mut self.output)
            .map_err(|error| {
                SeismicStoreError::Message(format!("bandpass inverse FFT failed: {error}"))
            })?;

        let inverse_scale = 1.0 / trace.len().max(1) as f32;
        for (sample, value) in trace.iter_mut().zip(self.output.iter()) {
            *sample = *value * inverse_scale;
        }

        Ok(())
    }

    fn apply_lowpass(
        &mut self,
        trace: &mut [f32],
        sample_interval_ms: f32,
        f3_hz: f32,
        f4_hz: f32,
        phase: FrequencyPhaseMode,
        window: FrequencyWindowShape,
    ) -> Result<(), SeismicStoreError> {
        if trace.len() != self.input.len() {
            return Err(SeismicStoreError::Message(format!(
                "lowpass workspace length mismatch: expected {}, found {}",
                self.input.len(),
                trace.len()
            )));
        }

        self.input.copy_from_slice(trace);
        remove_trace_mean(&mut self.input);

        self.forward
            .process(&mut self.input, &mut self.spectrum)
            .map_err(|error| {
                SeismicStoreError::Message(format!("lowpass forward FFT failed: {error}"))
            })?;
        apply_lowpass_response(
            &mut self.spectrum,
            trace.len(),
            sample_interval_ms,
            f3_hz,
            f4_hz,
            phase,
            window,
        )?;
        self.inverse
            .process(&mut self.spectrum, &mut self.output)
            .map_err(|error| {
                SeismicStoreError::Message(format!("lowpass inverse FFT failed: {error}"))
            })?;

        let inverse_scale = 1.0 / trace.len().max(1) as f32;
        for (sample, value) in trace.iter_mut().zip(self.output.iter()) {
            *sample = *value * inverse_scale;
        }

        Ok(())
    }

    fn apply_highpass(
        &mut self,
        trace: &mut [f32],
        sample_interval_ms: f32,
        f1_hz: f32,
        f2_hz: f32,
        phase: FrequencyPhaseMode,
        window: FrequencyWindowShape,
    ) -> Result<(), SeismicStoreError> {
        if trace.len() != self.input.len() {
            return Err(SeismicStoreError::Message(format!(
                "highpass workspace length mismatch: expected {}, found {}",
                self.input.len(),
                trace.len()
            )));
        }

        self.input.copy_from_slice(trace);
        remove_trace_mean(&mut self.input);

        self.forward
            .process(&mut self.input, &mut self.spectrum)
            .map_err(|error| {
                SeismicStoreError::Message(format!("highpass forward FFT failed: {error}"))
            })?;
        apply_highpass_response(
            &mut self.spectrum,
            trace.len(),
            sample_interval_ms,
            f1_hz,
            f2_hz,
            phase,
            window,
        )?;
        self.inverse
            .process(&mut self.spectrum, &mut self.output)
            .map_err(|error| {
                SeismicStoreError::Message(format!("highpass inverse FFT failed: {error}"))
            })?;

        let inverse_scale = 1.0 / trace.len().max(1) as f32;
        for (sample, value) in trace.iter_mut().zip(self.output.iter()) {
            *sample = *value * inverse_scale;
        }

        Ok(())
    }

    fn apply_phase_rotation(
        &mut self,
        trace: &mut [f32],
        angle_degrees: f32,
    ) -> Result<(), SeismicStoreError> {
        if trace.len() != self.input.len() {
            return Err(SeismicStoreError::Message(format!(
                "phase rotation workspace length mismatch: expected {}, found {}",
                self.input.len(),
                trace.len()
            )));
        }

        self.input.copy_from_slice(trace);
        self.forward
            .process(&mut self.input, &mut self.spectrum)
            .map_err(|error| {
                SeismicStoreError::Message(format!("phase rotation forward FFT failed: {error}"))
            })?;
        apply_phase_rotation_response(&mut self.spectrum, trace.len(), angle_degrees)?;
        self.inverse
            .process(&mut self.spectrum, &mut self.output)
            .map_err(|error| {
                SeismicStoreError::Message(format!("phase rotation inverse FFT failed: {error}"))
            })?;

        let inverse_scale = 1.0 / trace.len().max(1) as f32;
        for (sample, value) in trace.iter_mut().zip(self.output.iter()) {
            *sample = *value * inverse_scale;
        }

        Ok(())
    }
}

struct SpectrumWorkspace {
    forward: Arc<dyn RealToComplex<f32>>,
    input: Vec<f32>,
    spectrum: Vec<Complex32>,
}

impl SpectrumWorkspace {
    fn new(samples: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(samples);
        let input = forward.make_input_vec();
        let spectrum = forward.make_output_vec();
        Self {
            forward,
            input,
            spectrum,
        }
    }

    fn accumulate_trace_spectrum(
        &mut self,
        trace: &[f32],
        sample_interval_ms: f32,
        destination: &mut [f32],
    ) -> Result<(), SeismicStoreError> {
        if trace.len() != self.input.len() {
            return Err(SeismicStoreError::Message(format!(
                "spectrum workspace length mismatch: expected {}, found {}",
                self.input.len(),
                trace.len()
            )));
        }

        self.input.copy_from_slice(trace);
        remove_trace_mean(&mut self.input);
        self.forward
            .process(&mut self.input, &mut self.spectrum)
            .map_err(|error| SeismicStoreError::Message(format!("spectrum FFT failed: {error}")))?;
        accumulate_single_sided_amplitudes(
            &self.spectrum,
            trace.len(),
            sample_interval_ms,
            destination,
        )
    }
}

fn sample_interval_ms_from_axis(sample_axis_ms: &[f32]) -> Result<f32, SeismicStoreError> {
    if sample_axis_ms.len() < 2 {
        return Err(SeismicStoreError::Message(
            "sample axis must contain at least two entries to resolve sample interval".to_string(),
        ));
    }

    let step = (sample_axis_ms[1] - sample_axis_ms[0]).abs();
    if !step.is_finite() || step <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "sample axis step must be finite and > 0 ms, found {step}"
        )));
    }

    Ok(step)
}

fn remove_trace_mean(trace: &mut [f32]) {
    if trace.is_empty() {
        return;
    }

    let mean = trace.iter().sum::<f32>() / trace.len() as f32;
    for sample in trace.iter_mut() {
        *sample -= mean;
    }
}

fn apply_bandpass_response(
    spectrum: &mut [Complex32],
    samples: usize,
    sample_interval_ms: f32,
    f1_hz: f32,
    f2_hz: f32,
    f3_hz: f32,
    f4_hz: f32,
    phase: FrequencyPhaseMode,
    window: FrequencyWindowShape,
) -> Result<(), SeismicStoreError> {
    match phase {
        FrequencyPhaseMode::Zero => {}
    }
    match window {
        FrequencyWindowShape::CosineTaper => {}
    }

    let dt_s = sample_interval_ms / 1000.0;
    for (index, value) in spectrum.iter_mut().enumerate() {
        let frequency_hz = index as f32 / (samples.max(1) as f32 * dt_s);
        *value *= cosine_taper_bandpass_gain(frequency_hz, f1_hz, f2_hz, f3_hz, f4_hz);
    }

    Ok(())
}

fn apply_lowpass_response(
    spectrum: &mut [Complex32],
    samples: usize,
    sample_interval_ms: f32,
    f3_hz: f32,
    f4_hz: f32,
    phase: FrequencyPhaseMode,
    window: FrequencyWindowShape,
) -> Result<(), SeismicStoreError> {
    match phase {
        FrequencyPhaseMode::Zero => {}
    }
    match window {
        FrequencyWindowShape::CosineTaper => {}
    }

    let dt_s = sample_interval_ms / 1000.0;
    for (index, value) in spectrum.iter_mut().enumerate() {
        let frequency_hz = index as f32 / (samples.max(1) as f32 * dt_s);
        *value *= cosine_taper_lowpass_gain(frequency_hz, f3_hz, f4_hz);
    }

    Ok(())
}

fn apply_highpass_response(
    spectrum: &mut [Complex32],
    samples: usize,
    sample_interval_ms: f32,
    f1_hz: f32,
    f2_hz: f32,
    phase: FrequencyPhaseMode,
    window: FrequencyWindowShape,
) -> Result<(), SeismicStoreError> {
    match phase {
        FrequencyPhaseMode::Zero => {}
    }
    match window {
        FrequencyWindowShape::CosineTaper => {}
    }

    let dt_s = sample_interval_ms / 1000.0;
    for (index, value) in spectrum.iter_mut().enumerate() {
        let frequency_hz = index as f32 / (samples.max(1) as f32 * dt_s);
        *value *= cosine_taper_highpass_gain(frequency_hz, f1_hz, f2_hz);
    }

    Ok(())
}

fn apply_phase_rotation_response(
    spectrum: &mut [Complex32],
    samples: usize,
    angle_degrees: f32,
) -> Result<(), SeismicStoreError> {
    validate_phase_rotation_angle(angle_degrees)?;
    let angle_radians = angle_degrees.to_radians();
    let rotation = Complex32::new(angle_radians.cos(), angle_radians.sin());
    let dc_nyquist_scale = angle_radians.cos();

    for (index, value) in spectrum.iter_mut().enumerate() {
        let is_dc = index == 0;
        let is_nyquist = samples % 2 == 0 && index == samples / 2;
        if is_dc || is_nyquist {
            *value = Complex32::new(value.re * dc_nyquist_scale, 0.0);
            continue;
        }
        *value *= rotation;
    }

    Ok(())
}

fn cosine_taper_bandpass_gain(
    frequency_hz: f32,
    f1_hz: f32,
    f2_hz: f32,
    f3_hz: f32,
    f4_hz: f32,
) -> f32 {
    if frequency_hz <= f1_hz || frequency_hz >= f4_hz {
        return 0.0;
    }
    if frequency_hz >= f2_hz && frequency_hz <= f3_hz {
        return 1.0;
    }
    if frequency_hz < f2_hz {
        return cosine_ramp(frequency_hz, f1_hz, f2_hz);
    }
    cosine_ramp(frequency_hz, f4_hz, f3_hz)
}

fn cosine_taper_lowpass_gain(frequency_hz: f32, f3_hz: f32, f4_hz: f32) -> f32 {
    if frequency_hz <= f3_hz {
        return 1.0;
    }
    if frequency_hz >= f4_hz {
        return 0.0;
    }
    cosine_ramp(frequency_hz, f4_hz, f3_hz)
}

fn cosine_taper_highpass_gain(frequency_hz: f32, f1_hz: f32, f2_hz: f32) -> f32 {
    if frequency_hz <= f1_hz {
        return 0.0;
    }
    if frequency_hz >= f2_hz {
        return 1.0;
    }
    cosine_ramp(frequency_hz, f1_hz, f2_hz)
}

fn cosine_ramp(value: f32, edge0: f32, edge1: f32) -> f32 {
    let span = (edge1 - edge0).abs();
    if span <= SPECTRUM_EPSILON {
        return 1.0;
    }

    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    0.5 - 0.5 * (std::f32::consts::PI * t).cos()
}

struct ResolvedSpectrumSelection {
    trace_start: usize,
    trace_end: usize,
    sample_start: usize,
    sample_end: usize,
}

fn resolve_spectrum_selection(
    selection: &SectionSpectrumSelection,
    traces: usize,
    samples: usize,
) -> Result<ResolvedSpectrumSelection, SeismicStoreError> {
    match selection {
        SectionSpectrumSelection::WholeSection => Ok(ResolvedSpectrumSelection {
            trace_start: 0,
            trace_end: traces,
            sample_start: 0,
            sample_end: samples,
        }),
        SectionSpectrumSelection::TraceRange {
            trace_start,
            trace_end,
        } => {
            validate_trace_range(*trace_start, *trace_end, traces)?;
            Ok(ResolvedSpectrumSelection {
                trace_start: *trace_start,
                trace_end: *trace_end,
                sample_start: 0,
                sample_end: samples,
            })
        }
        SectionSpectrumSelection::RectWindow {
            trace_start,
            trace_end,
            sample_start,
            sample_end,
        } => {
            validate_trace_range(*trace_start, *trace_end, traces)?;
            validate_sample_range(*sample_start, *sample_end, samples)?;
            Ok(ResolvedSpectrumSelection {
                trace_start: *trace_start,
                trace_end: *trace_end,
                sample_start: *sample_start,
                sample_end: *sample_end,
            })
        }
    }
}

fn validate_trace_range(
    trace_start: usize,
    trace_end: usize,
    traces: usize,
) -> Result<(), SeismicStoreError> {
    if trace_start >= trace_end {
        return Err(SeismicStoreError::Message(format!(
            "spectrum trace range must satisfy trace_start < trace_end, found [{trace_start}, {trace_end})"
        )));
    }
    if trace_end > traces {
        return Err(SeismicStoreError::Message(format!(
            "spectrum trace range end {trace_end} exceeds trace count {traces}"
        )));
    }
    Ok(())
}

fn validate_sample_range(
    sample_start: usize,
    sample_end: usize,
    samples: usize,
) -> Result<(), SeismicStoreError> {
    if sample_start >= sample_end {
        return Err(SeismicStoreError::Message(format!(
            "spectrum sample range must satisfy sample_start < sample_end, found [{sample_start}, {sample_end})"
        )));
    }
    if sample_end > samples {
        return Err(SeismicStoreError::Message(format!(
            "spectrum sample range end {sample_end} exceeds sample count {samples}"
        )));
    }
    if sample_end - sample_start < 2 {
        return Err(SeismicStoreError::Message(
            "spectrum sample range must contain at least two samples".to_string(),
        ));
    }
    Ok(())
}

fn frequency_bins_hz(samples: usize, sample_interval_ms: f32) -> Vec<f32> {
    let dt_s = sample_interval_ms / 1000.0;
    let frequency_step = 1.0 / (samples.max(1) as f32 * dt_s);
    (0..=(samples / 2))
        .map(|index| index as f32 * frequency_step)
        .collect()
}

fn accumulate_single_sided_amplitudes(
    spectrum: &[Complex32],
    samples: usize,
    sample_interval_ms: f32,
    destination: &mut [f32],
) -> Result<(), SeismicStoreError> {
    let expected_bins = samples / 2 + 1;
    if destination.len() != expected_bins {
        return Err(SeismicStoreError::Message(format!(
            "spectrum accumulation buffer length mismatch: expected {expected_bins}, found {}",
            destination.len()
        )));
    }

    let _ = nyquist_hz_for_sample_interval_ms(sample_interval_ms)?;
    let normalization = samples.max(1) as f32;
    let nyquist_index = spectrum.len().saturating_sub(1);

    for (index, value) in spectrum.iter().enumerate() {
        let mut amplitude = value.norm() / normalization;
        if index != 0 && index != nyquist_index {
            amplitude *= 2.0;
        }
        destination[index] += amplitude;
    }

    Ok(())
}

fn compute_pool() -> &'static ThreadPool {
    static POOL: OnceLock<ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let threads = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);
        ThreadPoolBuilder::new()
            .thread_name(|index| format!("ophiolite-seismic-compute-{index}"))
            .num_threads(threads)
            .build()
            .expect("compute pool should build")
    })
}

fn resolve_chunk_shape(chunk_shape: [usize; 3], shape: [usize; 3]) -> [usize; 3] {
    if chunk_shape.iter().all(|value| *value == 0) {
        return recommended_tbvol_tile_shape(
            shape,
            recommended_default_tbvol_tile_target_mib(shape),
        );
    }

    [
        chunk_shape[0].max(1).min(shape[0].max(1)),
        chunk_shape[1].max(1).min(shape[1].max(1)),
        chunk_shape[2].max(1).min(shape[2].max(1)),
    ]
}

fn pipeline_from_operations(operations: &[ProcessingOperation]) -> ProcessingPipeline {
    ProcessingPipeline {
        schema_version: 1,
        revision: 1,
        preset_id: None,
        name: None,
        description: None,
        operations: operations.to_vec(),
    }
}

fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn derived_volume_metadata(
    input: &VolumeMetadata,
    parent_store: &Path,
    pipeline: &ProcessingPipeline,
    created_by: String,
) -> VolumeMetadata {
    VolumeMetadata {
        kind: DatasetKind::Derived,
        source: input.source.clone(),
        shape: input.shape,
        axes: input.axes.clone(),
        created_by,
        processing_lineage: Some(ProcessingLineage {
            parent_store: parent_store.to_path_buf(),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: pipeline.clone(),
            },
            runtime_version: RUNTIME_VERSION.to_string(),
            created_at_unix_s: unix_timestamp_s(),
        }),
    }
}

fn reader_has_occupancy<R: VolumeStoreReader>(reader: &R) -> Result<bool, SeismicStoreError> {
    reader
        .read_tile_occupancy(TileCoord {
            tile_i: 0,
            tile_x: 0,
        })
        .map(|value| value.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProcessingOperatorScope;
    use crate::SectionAxis;
    use std::collections::HashMap;

    #[test]
    fn pipeline_validation_rejects_invalid_scalar() {
        let result = validate_pipeline(&[ProcessingOperation::AmplitudeScalar { factor: 12.0 }]);
        assert!(result.is_err());
    }

    #[test]
    fn apply_pipeline_skips_unoccupied_traces() {
        let mut values = vec![1.0_f32, 2.0, 3.0, 4.0];
        apply_pipeline_to_traces(
            &mut values,
            2,
            2,
            2.0,
            Some(&[1, 0]),
            &[ProcessingOperation::AmplitudeScalar { factor: 2.0 }],
        )
        .unwrap();
        assert_eq!(values, vec![2.0, 4.0, 3.0, 4.0]);
    }

    #[test]
    fn preview_materialization_uses_same_normalization_kernel() {
        let mut plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples: 4,
            horizontal_axis: vec![0.0],
            sample_axis_ms: vec![0.0, 2.0, 4.0, 6.0],
            amplitudes: vec![1.0, 2.0, 3.0, 4.0],
            occupancy: None,
        };
        apply_pipeline_to_plane(&mut plane, &[ProcessingOperation::TraceRmsNormalize]).unwrap();
        let rms = (plane
            .amplitudes
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            / 4.0)
            .sqrt();
        assert!((rms - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn current_trace_local_operators_support_trace_matrix_layouts() {
        let operations = vec![
            ProcessingOperation::AmplitudeScalar { factor: 1.5 },
            ProcessingOperation::TraceRmsNormalize,
            ProcessingOperation::AgcRms { window_ms: 250.0 },
            ProcessingOperation::PhaseRotation {
                angle_degrees: 30.0,
            },
            ProcessingOperation::LowpassFilter {
                f3_hz: 30.0,
                f4_hz: 45.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            },
            ProcessingOperation::HighpassFilter {
                f1_hz: 4.0,
                f2_hz: 10.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            },
            ProcessingOperation::BandpassFilter {
                f1_hz: 5.0,
                f2_hz: 10.0,
                f3_hz: 30.0,
                f4_hz: 40.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            },
            ProcessingOperation::VolumeArithmetic {
                operator: TraceLocalVolumeArithmeticOperator::Subtract,
                secondary_store_path: "secondary.tbvol".to_string(),
            },
        ];

        for operation in operations {
            validate_pipeline_for_layout(&[operation], SeismicLayout::PreStack3DOffset)
                .expect("current trace-local operators should support trace-matrix layouts");
        }
    }

    #[test]
    fn current_operator_scope_is_trace_local() {
        let operations = vec![
            ProcessingOperation::AmplitudeScalar { factor: 1.0 },
            ProcessingOperation::TraceRmsNormalize,
            ProcessingOperation::AgcRms { window_ms: 200.0 },
            ProcessingOperation::PhaseRotation { angle_degrees: 0.0 },
            ProcessingOperation::LowpassFilter {
                f3_hz: 30.0,
                f4_hz: 40.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            },
            ProcessingOperation::HighpassFilter {
                f1_hz: 4.0,
                f2_hz: 8.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            },
            ProcessingOperation::BandpassFilter {
                f1_hz: 5.0,
                f2_hz: 10.0,
                f3_hz: 30.0,
                f4_hz: 40.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            },
            ProcessingOperation::VolumeArithmetic {
                operator: TraceLocalVolumeArithmeticOperator::Add,
                secondary_store_path: "secondary.tbvol".to_string(),
            },
        ];

        for operation in operations {
            assert_eq!(operation.scope(), ProcessingOperatorScope::TraceLocal);
        }
    }

    #[test]
    fn pipeline_validation_rejects_invalid_bandpass_order() {
        let result = validate_pipeline(&[ProcessingOperation::BandpassFilter {
            f1_hz: 20.0,
            f2_hz: 10.0,
            f3_hz: 30.0,
            f4_hz: 40.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_invalid_lowpass_order() {
        let result = validate_pipeline(&[ProcessingOperation::LowpassFilter {
            f3_hz: 45.0,
            f4_hz: 30.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_invalid_highpass_order() {
        let result = validate_pipeline(&[ProcessingOperation::HighpassFilter {
            f1_hz: 12.0,
            f2_hz: 8.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_bandpass_over_nyquist() {
        let result = validate_pipeline_for_sample_interval(
            &[ProcessingOperation::BandpassFilter {
                f1_hz: 5.0,
                f2_hz: 10.0,
                f3_hz: 40.0,
                f4_hz: 300.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            }],
            2.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_lowpass_over_nyquist() {
        let result = validate_pipeline_for_sample_interval(
            &[ProcessingOperation::LowpassFilter {
                f3_hz: 20.0,
                f4_hz: 300.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            }],
            2.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_highpass_over_nyquist() {
        let result = validate_pipeline_for_sample_interval(
            &[ProcessingOperation::HighpassFilter {
                f1_hz: 5.0,
                f2_hz: 300.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            }],
            2.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_invalid_agc_window() {
        let result = validate_pipeline(&[ProcessingOperation::AgcRms { window_ms: 0.0 }]);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_empty_volume_arithmetic_store_path() {
        let result = validate_pipeline(&[ProcessingOperation::VolumeArithmetic {
            operator: TraceLocalVolumeArithmeticOperator::Subtract,
            secondary_store_path: "   ".to_string(),
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_validation_rejects_phase_rotation_out_of_range() {
        let result = validate_pipeline(&[ProcessingOperation::PhaseRotation {
            angle_degrees: 270.0,
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn phase_rotation_zero_is_identity() {
        let mut trace = vec![1.0_f32, -2.0, 0.5, 4.0, -1.5, 0.0, 2.0, -0.25];
        let original = trace.clone();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            original.len(),
            2.0,
            None,
            &[ProcessingOperation::PhaseRotation { angle_degrees: 0.0 }],
        )
        .unwrap();

        for (actual, expected) in trace.iter().zip(original.iter()) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
    }

    #[test]
    fn phase_rotation_180_matches_polarity_reversal() {
        let mut trace = vec![1.0_f32, -2.0, 0.5, 4.0, -1.5, 0.0, 2.0, -0.25];
        let original = trace.clone();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            original.len(),
            2.0,
            None,
            &[ProcessingOperation::PhaseRotation {
                angle_degrees: 180.0,
            }],
        )
        .unwrap();

        for (actual, expected) in trace.iter().zip(original.iter()) {
            assert!((actual + expected).abs() < 1.0e-5);
        }
    }

    #[test]
    fn phase_rotation_90_turns_sine_into_cosine() {
        let sample_interval_ms = 2.0_f32;
        let samples = 256usize;
        let dt_s = sample_interval_ms / 1000.0;
        let frequency_hz = 16.0 / (samples as f32 * dt_s);
        let mut trace = (0..samples)
            .map(|index| {
                let t = index as f32 * dt_s;
                (2.0 * std::f32::consts::PI * frequency_hz * t).sin()
            })
            .collect::<Vec<_>>();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::PhaseRotation {
                angle_degrees: 90.0,
            }],
        )
        .unwrap();

        let mut max_error_positive = 0.0_f32;
        let mut max_error_negative = 0.0_f32;
        for (index, actual) in trace.iter().enumerate() {
            let t = index as f32 * dt_s;
            let expected = (2.0 * std::f32::consts::PI * frequency_hz * t).cos();
            max_error_positive = max_error_positive.max((actual - expected).abs());
            max_error_negative = max_error_negative.max((actual + expected).abs());
        }
        assert!(max_error_positive.min(max_error_negative) < 3.0e-3);
    }

    #[test]
    fn bandpass_preserves_in_band_energy_and_reduces_out_of_band_energy() {
        let sample_interval_ms = 2.0_f32;
        let samples = 256usize;
        let dt_s = sample_interval_ms / 1000.0;
        let mut trace = (0..samples)
            .map(|index| {
                let t = index as f32 * dt_s;
                (2.0 * std::f32::consts::PI * 20.0 * t).sin()
                    + 0.6 * (2.0 * std::f32::consts::PI * 90.0 * t).sin()
            })
            .collect::<Vec<_>>();
        let original = trace.clone();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::BandpassFilter {
                f1_hz: 10.0,
                f2_hz: 15.0,
                f3_hz: 30.0,
                f4_hz: 40.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            }],
        )
        .unwrap();

        let original_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: original,
            occupancy: None,
        };
        let filtered_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: trace,
            occupancy: None,
        };

        let original_spectrum =
            amplitude_spectrum_from_plane(&original_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();
        let filtered_spectrum =
            amplitude_spectrum_from_plane(&filtered_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();

        let amplitude_at = |curve: &AmplitudeSpectrumCurve, target_hz: f32| {
            let (index, _) = curve
                .frequencies_hz
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    (*left - target_hz)
                        .abs()
                        .partial_cmp(&(*right - target_hz).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("spectrum bins should exist");
            curve.amplitudes[index]
        };

        let original_20 = amplitude_at(&original_spectrum, 20.0);
        let filtered_20 = amplitude_at(&filtered_spectrum, 20.0);
        let original_90 = amplitude_at(&original_spectrum, 90.0);
        let filtered_90 = amplitude_at(&filtered_spectrum, 90.0);

        assert!(filtered_20 > original_20 * 0.7);
        assert!(filtered_90 < original_90 * 0.25);
    }

    #[test]
    fn lowpass_preserves_low_frequency_energy_and_reduces_high_frequency_energy() {
        let sample_interval_ms = 2.0_f32;
        let samples = 256usize;
        let dt_s = sample_interval_ms / 1000.0;
        let mut trace = (0..samples)
            .map(|index| {
                let t = index as f32 * dt_s;
                (2.0 * std::f32::consts::PI * 15.0 * t).sin()
                    + 0.65 * (2.0 * std::f32::consts::PI * 70.0 * t).sin()
            })
            .collect::<Vec<_>>();
        let original = trace.clone();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::LowpassFilter {
                f3_hz: 24.0,
                f4_hz: 36.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            }],
        )
        .unwrap();

        let original_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: original,
            occupancy: None,
        };
        let filtered_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: trace,
            occupancy: None,
        };

        let original_spectrum =
            amplitude_spectrum_from_plane(&original_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();
        let filtered_spectrum =
            amplitude_spectrum_from_plane(&filtered_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();

        let amplitude_at = |curve: &AmplitudeSpectrumCurve, target_hz: f32| {
            let (index, _) = curve
                .frequencies_hz
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    (*left - target_hz)
                        .abs()
                        .partial_cmp(&(*right - target_hz).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("spectrum bins should exist");
            curve.amplitudes[index]
        };

        let original_15 = amplitude_at(&original_spectrum, 15.0);
        let filtered_15 = amplitude_at(&filtered_spectrum, 15.0);
        let original_70 = amplitude_at(&original_spectrum, 70.0);
        let filtered_70 = amplitude_at(&filtered_spectrum, 70.0);

        assert!(filtered_15 > original_15 * 0.7);
        assert!(filtered_70 < original_70 * 0.25);
    }

    #[test]
    fn highpass_preserves_high_frequency_energy_and_reduces_low_frequency_energy() {
        let sample_interval_ms = 2.0_f32;
        let samples = 256usize;
        let dt_s = sample_interval_ms / 1000.0;
        let mut trace = (0..samples)
            .map(|index| {
                let t = index as f32 * dt_s;
                (2.0 * std::f32::consts::PI * 8.0 * t).sin()
                    + 0.65 * (2.0 * std::f32::consts::PI * 45.0 * t).sin()
            })
            .collect::<Vec<_>>();
        let original = trace.clone();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::HighpassFilter {
                f1_hz: 14.0,
                f2_hz: 22.0,
                phase: FrequencyPhaseMode::Zero,
                window: FrequencyWindowShape::CosineTaper,
            }],
        )
        .unwrap();

        let original_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: original,
            occupancy: None,
        };
        let filtered_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: trace,
            occupancy: None,
        };

        let original_spectrum =
            amplitude_spectrum_from_plane(&original_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();
        let filtered_spectrum =
            amplitude_spectrum_from_plane(&filtered_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();

        let amplitude_at = |curve: &AmplitudeSpectrumCurve, target_hz: f32| {
            let (index, _) = curve
                .frequencies_hz
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    (*left - target_hz)
                        .abs()
                        .partial_cmp(&(*right - target_hz).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("spectrum bins should exist");
            curve.amplitudes[index]
        };

        let original_8 = amplitude_at(&original_spectrum, 8.0);
        let filtered_8 = amplitude_at(&filtered_spectrum, 8.0);
        let original_45 = amplitude_at(&original_spectrum, 45.0);
        let filtered_45 = amplitude_at(&filtered_spectrum, 45.0);

        assert!(filtered_8 < original_8 * 0.25);
        assert!(filtered_45 > original_45 * 0.7);
    }

    #[test]
    fn agc_balances_low_and_high_energy_segments() {
        let sample_interval_ms = 2.0_f32;
        let samples = 400usize;
        let dt_s = sample_interval_ms / 1000.0;
        let mut trace = (0..samples)
            .map(|index| {
                let amplitude = if index < samples / 2 { 0.5 } else { 3.0 };
                let t = index as f32 * dt_s;
                amplitude * (2.0 * std::f32::consts::PI * 18.0 * t).sin()
            })
            .collect::<Vec<_>>();

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::AgcRms { window_ms: 120.0 }],
        )
        .unwrap();

        let region_rms = |start: usize, end: usize| {
            (trace[start..end]
                .iter()
                .map(|value| value * value)
                .sum::<f32>()
                / (end - start) as f32)
                .sqrt()
        };

        let early_rms = region_rms(40, 160);
        let late_rms = region_rms(240, 360);

        assert!((early_rms - 1.0).abs() < 0.2);
        assert!((late_rms - 1.0).abs() < 0.2);
        assert!((early_rms - late_rms).abs() < 0.15);
    }

    #[test]
    fn phase_rotation_preserves_amplitude_spectrum() {
        let sample_interval_ms = 2.0_f32;
        let samples = 256usize;
        let dt_s = sample_interval_ms / 1000.0;
        let original = (0..samples)
            .map(|index| {
                let t = index as f32 * dt_s;
                (2.0 * std::f32::consts::PI * 20.0 * t).sin()
                    + 0.35 * (2.0 * std::f32::consts::PI * 45.0 * t).cos()
            })
            .collect::<Vec<_>>();
        let mut rotated = original.clone();

        apply_pipeline_to_traces(
            &mut rotated,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::PhaseRotation {
                angle_degrees: 35.0,
            }],
        )
        .unwrap();

        let original_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: original,
            occupancy: None,
        };
        let rotated_plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 1,
            samples,
            horizontal_axis: vec![0.0],
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: rotated,
            occupancy: None,
        };

        let original_spectrum =
            amplitude_spectrum_from_plane(&original_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();
        let rotated_spectrum =
            amplitude_spectrum_from_plane(&rotated_plane, &SectionSpectrumSelection::WholeSection)
                .unwrap();

        for (left, right) in original_spectrum
            .amplitudes
            .iter()
            .zip(rotated_spectrum.amplitudes.iter())
        {
            assert!((left - right).abs() < 1.0e-3);
        }
    }

    #[test]
    fn amplitude_spectrum_averages_trace_range() {
        let plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 2,
            samples: 4,
            horizontal_axis: vec![0.0, 1.0],
            sample_axis_ms: vec![0.0, 2.0, 4.0, 6.0],
            amplitudes: vec![1.0, 0.0, -1.0, 0.0, 0.5, 0.0, -0.5, 0.0],
            occupancy: None,
        };

        let spectrum = amplitude_spectrum_from_plane(
            &plane,
            &SectionSpectrumSelection::TraceRange {
                trace_start: 0,
                trace_end: 2,
            },
        )
        .unwrap();

        assert_eq!(spectrum.frequencies_hz.len(), 3);
        assert_eq!(spectrum.amplitudes.len(), 3);
        assert!(spectrum.amplitudes.iter().any(|value| *value > 0.0));
    }

    #[test]
    fn amplitude_spectrum_supports_rect_window_selection() {
        let plane = SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: 0,
            coordinate_value: 0.0,
            traces: 2,
            samples: 6,
            horizontal_axis: vec![0.0, 1.0],
            sample_axis_ms: vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0],
            amplitudes: vec![0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.5, 0.0, -0.5, 0.0, 0.0],
            occupancy: None,
        };

        let spectrum = amplitude_spectrum_from_plane(
            &plane,
            &SectionSpectrumSelection::RectWindow {
                trace_start: 0,
                trace_end: 2,
                sample_start: 1,
                sample_end: 5,
            },
        )
        .unwrap();

        assert_eq!(spectrum.frequencies_hz.len(), 3);
        assert_eq!(spectrum.amplitudes.len(), 3);
        assert!(spectrum.amplitudes.iter().any(|value| *value > 0.0));
    }

    #[test]
    fn volume_arithmetic_adds_secondary_trace_samples() {
        let mut trace = vec![1.0_f32, 2.0, 3.0, 4.0];
        let secondary_inputs = HashMap::from([(
            "secondary.tbvol".to_string(),
            SecondaryTraceMatrix {
                amplitudes: vec![0.5, 1.0, 1.5, 2.0],
                occupancy: None,
            },
        )]);

        apply_pipeline_to_traces_internal(
            &mut trace,
            1,
            4,
            2.0,
            None,
            &[ProcessingOperation::VolumeArithmetic {
                operator: TraceLocalVolumeArithmeticOperator::Add,
                secondary_store_path: "secondary.tbvol".to_string(),
            }],
            Some(&secondary_inputs),
        )
        .unwrap();

        assert_eq!(trace, vec![1.5, 3.0, 4.5, 6.0]);
    }

    #[test]
    fn volume_arithmetic_requires_secondary_inputs_outside_store_backed_paths() {
        let mut trace = vec![1.0_f32, 2.0, 3.0, 4.0];

        let result = apply_pipeline_to_traces(
            &mut trace,
            1,
            4,
            2.0,
            None,
            &[ProcessingOperation::VolumeArithmetic {
                operator: TraceLocalVolumeArithmeticOperator::Subtract,
                secondary_store_path: "secondary.tbvol".to_string(),
            }],
        );

        assert!(result.is_err());
    }
}
