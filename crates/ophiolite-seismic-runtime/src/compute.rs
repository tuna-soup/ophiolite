use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

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
use crate::{ProcessingOperation, ProcessingPipeline, SectionAxis, SectionView, SeismicLayout};

const MAX_SCALAR_FACTOR: f32 = 10.0;
const RMS_EPSILON: f32 = 1.0e-8;
const MAX_RMS_GAIN: f32 = 1.0e6;
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
            occupancy.as_deref(),
            pipeline,
        );
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
    apply_pipeline_to_traces(
        &mut plane.amplitudes,
        plane.traces,
        plane.samples,
        plane.occupancy.as_deref(),
        pipeline,
    );
    Ok(())
}

pub fn apply_pipeline_to_traces(
    data: &mut [f32],
    traces: usize,
    samples: usize,
    occupancy: Option<&[u8]>,
    pipeline: &[ProcessingOperation],
) {
    if traces == 0 || samples == 0 || data.is_empty() || pipeline.is_empty() {
        return;
    }

    compute_pool().install(|| {
        data.par_chunks_mut(samples)
            .enumerate()
            .for_each(|(trace_index, trace)| {
                if trace_index >= traces {
                    return;
                }
                if occupancy.is_some_and(|mask| mask.get(trace_index).copied().unwrap_or(1) == 0) {
                    return;
                }
                for operation in pipeline {
                    apply_operation_to_trace(trace, operation);
                }
            });
    });
}

fn apply_operation_to_trace(trace: &mut [f32], operation: &ProcessingOperation) {
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
    }
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
            Some(&[1, 0]),
            &[ProcessingOperation::AmplitudeScalar { factor: 2.0 }],
        );
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
}
