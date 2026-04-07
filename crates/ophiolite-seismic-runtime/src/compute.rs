use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use realfft::{ComplexToReal, RealFftPlanner, RealToComplex, num_complex::Complex32};
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use crate::error::SeismicStoreError;
use crate::metadata::{DatasetKind, ProcessingLineage, VolumeMetadata};
use crate::storage::section_assembler;
use crate::storage::tbvol::{TbvolReader, TbvolWriter, recommended_tbvol_tile_shape};
use crate::storage::tile_geometry::TileCoord;
use crate::storage::volume_store::{VolumeStoreReader, VolumeStoreWriter};
use crate::store::{SectionPlane, StoreHandle, open_store};
use crate::{
    AmplitudeSpectrumCurve, FrequencyPhaseMode, FrequencyWindowShape, ProcessingOperation,
    ProcessingPipeline, SectionAxis, SectionSpectrumSelection, SectionView, SeismicLayout,
};

const MAX_SCALAR_FACTOR: f32 = 10.0;
const RMS_EPSILON: f32 = 1.0e-8;
const MAX_RMS_GAIN: f32 = 1.0e6;
const SPECTRUM_EPSILON: f32 = 1.0e-12;
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
        if let ProcessingOperation::BandpassFilter { f4_hz, .. } = operation
            && *f4_hz > nyquist_hz
        {
            return Err(SeismicStoreError::Message(format!(
                "bandpass high corner f4_hz must be <= Nyquist ({nyquist_hz:.3} Hz), found {f4_hz}"
            )));
        }
    }
    Ok(())
}

fn pipeline_requires_sample_interval(pipeline: &[ProcessingOperation]) -> bool {
    pipeline
        .iter()
        .any(|operation| matches!(operation, ProcessingOperation::BandpassFilter { .. }))
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
    preview_section_from_reader(&reader, axis, index, pipeline)
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
    let plane = preview_section_from_reader(&reader, axis, index, pipeline)?;
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
    materialize_from_reader_writer(&reader, writer, pipeline)?;
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
    materialize_from_reader_writer_with_progress(
        &reader,
        writer,
        &pipeline.operations,
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
        apply_pipeline_to_traces(
            &mut amplitudes,
            traces,
            samples,
            sample_interval_ms,
            occupancy.as_deref(),
            pipeline,
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

pub fn apply_pipeline_to_plane(
    plane: &mut SectionPlane,
    pipeline: &[ProcessingOperation],
) -> Result<(), SeismicStoreError> {
    validate_pipeline(pipeline)?;
    let sample_interval_ms = sample_interval_ms_from_axis(&plane.sample_axis_ms)?;
    validate_pipeline_for_sample_interval(pipeline, sample_interval_ms)?;
    apply_pipeline_to_traces(
        &mut plane.amplitudes,
        plane.traces,
        plane.samples,
        sample_interval_ms,
        plane.occupancy.as_deref(),
        pipeline,
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
    validate_pipeline_for_sample_interval(pipeline, sample_interval_ms)?;
    let needs_fft = pipeline_requires_sample_interval(pipeline);

    compute_pool().install(|| {
        data.par_chunks_mut(samples)
            .enumerate()
            .try_for_each_init(
                || TraceComputeState::new(samples, needs_fft),
                |state, (trace_index, trace)| {
                if trace_index >= traces {
                    return Ok(());
                }
                if occupancy.is_some_and(|mask| mask.get(trace_index).copied().unwrap_or(1) == 0) {
                    return Ok(());
                }
                for operation in pipeline {
                    apply_operation_to_trace(trace, sample_interval_ms, state, operation)?;
                }
                Ok(())
            },
        )
    })
}

fn apply_operation_to_trace(
    trace: &mut [f32],
    sample_interval_ms: f32,
    state: &mut TraceComputeState,
    operation: &ProcessingOperation,
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
        ProcessingOperation::BandpassFilter {
            f1_hz,
            f2_hz,
            f3_hz,
            f4_hz,
            phase,
            window,
        } => {
            state
                .bandpass
                .as_mut()
                .expect("bandpass workspace should exist when bandpass operators are present")
                .apply(
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
    }

    Ok(())
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
        Some(operations) if !operations.is_empty() => preview_section_from_reader(reader, axis, index, operations)?,
        _ => section_assembler::read_section_plane(reader, axis, index)?,
    };
    amplitude_spectrum_from_plane(&plane, selection)
}

struct TraceComputeState {
    bandpass: Option<BandpassWorkspace>,
}

impl TraceComputeState {
    fn new(samples: usize, needs_bandpass: bool) -> Self {
        Self {
            bandpass: needs_bandpass.then(|| BandpassWorkspace::new(samples)),
        }
    }
}

struct BandpassWorkspace {
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
    input: Vec<f32>,
    output: Vec<f32>,
    spectrum: Vec<Complex32>,
}

impl BandpassWorkspace {
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

    fn apply(
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
            .map_err(|error| SeismicStoreError::Message(format!("bandpass forward FFT failed: {error}")))?;
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
            .map_err(|error| SeismicStoreError::Message(format!("bandpass inverse FFT failed: {error}")))?;

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
        accumulate_single_sided_amplitudes(&self.spectrum, trace.len(), sample_interval_ms, destination)
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
        return recommended_tbvol_tile_shape(shape, 4);
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
            pipeline: pipeline.clone(),
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
    use crate::SectionAxis;

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
    fn pipeline_validation_rejects_incompatible_layout() {
        let result = validate_pipeline_for_layout(
            &[ProcessingOperation::TraceRmsNormalize],
            SeismicLayout::PreStack3DOffset,
        );
        let error = result.expect_err("prestack layout should be rejected for current operators");
        assert!(error.to_string().contains("requires post-stack only"));
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
            sample_axis_ms: (0..samples).map(|index| index as f32 * sample_interval_ms).collect(),
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
            sample_axis_ms: (0..samples).map(|index| index as f32 * sample_interval_ms).collect(),
            amplitudes: trace,
            occupancy: None,
        };

        let original_spectrum = amplitude_spectrum_from_plane(
            &original_plane,
            &SectionSpectrumSelection::WholeSection,
        )
        .unwrap();
        let filtered_spectrum = amplitude_spectrum_from_plane(
            &filtered_plane,
            &SectionSpectrumSelection::WholeSection,
        )
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
            amplitudes: vec![
                0.0, 1.0, 0.0, -1.0, 0.0, 0.0,
                0.0, 0.5, 0.0, -0.5, 0.0, 0.0,
            ],
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
}
