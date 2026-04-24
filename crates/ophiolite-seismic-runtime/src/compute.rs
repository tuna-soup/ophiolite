use std::collections::{BTreeSet, HashMap, hash_map::DefaultHasher};
use std::hash::Hasher;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::SeismicStoreError;
use crate::execution::{
    ArtifactKey, ChunkGridSpec, ExecutionPlan, GeometryFingerprints, LogicalDomain,
    MaterializationClass, SectionDomain, TraceLocalChunkPlanRecommendation,
};
use crate::identity::{CURRENT_RUNTIME_SEMANTICS_VERSION, CURRENT_STORE_WRITER_SEMANTICS_VERSION};
use crate::metadata::{DatasetKind, ProcessingLineage, VolumeMetadata, generate_store_id};
use crate::planner::{
    TraceLocalChunkPlanResolution, recommend_trace_local_chunk_plan_for_execution,
};
use crate::segy_export::{copy_store_segy_export, crop_store_segy_export};
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
    SeismicLayout, SubvolumeCropOperation, SubvolumeProcessingPipeline,
    TraceLocalVolumeArithmeticOperator,
};
use ophiolite_seismic::{ProcessingArtifactRole, ProcessingJobChunkPlanSummary};
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex, num_complex::Complex32};

const MAX_SCALAR_FACTOR: f32 = 10.0;
const MAX_PHASE_ROTATION_DEGREES: f32 = 180.0;
const MAX_AGC_WINDOW_MS: f32 = 10_000.0;
const RMS_EPSILON: f32 = 1.0e-8;
const MAX_RMS_GAIN: f32 = 1.0e6;
const SPECTRUM_EPSILON: f32 = 1.0e-12;
const DIVISION_EPSILON: f32 = 1.0e-8;
const INSTANTANEOUS_FREQUENCY_EPSILON: f32 = 1.0e-8;
const SWEETNESS_FREQUENCY_FLOOR_HZ: f32 = 1.0;
const RUNTIME_VERSION: &str = "ophiolite-seismic-runtime-0.1.0";
const DEFAULT_PREVIEW_SECTION_PREFIX_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct MaterializeOptions {
    pub chunk_shape: [usize; 3],
    pub partition_target_bytes: Option<u64>,
    pub max_active_partitions: Option<usize>,
    pub trace_local_chunk_plan: Option<TraceLocalChunkPlanRecommendation>,
    pub created_by: String,
}

impl Default for MaterializeOptions {
    fn default() -> Self {
        Self {
            chunk_shape: [0, 0, 0],
            partition_target_bytes: None,
            max_active_partitions: None,
            trace_local_chunk_plan: None,
            created_by: RUNTIME_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceLocalMaterializeOptionsResolution {
    pub options: MaterializeOptions,
    pub chunk_plan_resolution: Option<TraceLocalChunkPlanResolution>,
    pub resolved_chunk_plan: Option<ProcessingJobChunkPlanSummary>,
    pub resolved_partition_target_bytes: Option<u64>,
}

pub fn resolve_trace_local_materialize_options(
    plan: Option<&ExecutionPlan>,
    chunk_shape: [usize; 3],
    adaptive_partition_target: bool,
    fallback_partition_target_bytes: Option<u64>,
    worker_count: usize,
    available_memory_bytes: Option<u64>,
    concurrent_job_count: usize,
) -> TraceLocalMaterializeOptionsResolution {
    let chunk_plan_resolution = if adaptive_partition_target {
        plan.and_then(|plan| {
            recommend_trace_local_chunk_plan_for_execution(
                plan,
                worker_count,
                available_memory_bytes,
                concurrent_job_count,
            )
        })
    } else {
        None
    };
    let chunk_plan = chunk_plan_resolution
        .as_ref()
        .map(|recommendation| recommendation.trace_local_chunk_plan());
    let resolved_chunk_plan = chunk_plan
        .as_ref()
        .map(processing_job_chunk_plan_summary)
        .or_else(|| {
            fallback_partition_target_bytes.and_then(|target_bytes| {
                fixed_partition_target_summary(plan, target_bytes, worker_count)
            })
        });
    let resolved_partition_target_bytes = chunk_plan_resolution
        .as_ref()
        .map(|recommendation| recommendation.target_bytes())
        .or(fallback_partition_target_bytes);
    let options = MaterializeOptions {
        chunk_shape,
        partition_target_bytes: if chunk_plan.is_some() {
            None
        } else {
            fallback_partition_target_bytes
        },
        max_active_partitions: None,
        trace_local_chunk_plan: chunk_plan,
        ..MaterializeOptions::default()
    };

    TraceLocalMaterializeOptionsResolution {
        options,
        chunk_plan_resolution,
        resolved_chunk_plan,
        resolved_partition_target_bytes,
    }
}

fn processing_job_chunk_plan_summary(
    plan: &TraceLocalChunkPlanRecommendation,
) -> ProcessingJobChunkPlanSummary {
    ProcessingJobChunkPlanSummary {
        partition_count: plan.partition_count,
        max_active_partitions: plan.max_active_partitions,
        tiles_per_partition: plan.tiles_per_partition,
        compatibility_target_bytes: plan.compatibility_target_bytes,
        estimated_peak_bytes: plan.estimated_peak_bytes,
    }
}

fn fixed_partition_target_summary(
    plan: Option<&ExecutionPlan>,
    partition_target_bytes: u64,
    worker_count: usize,
) -> Option<ProcessingJobChunkPlanSummary> {
    let plan = plan?;
    let source_shape = plan.source.shape?;
    let source_chunk_shape = plan.source.chunk_shape?;
    let chunk_inline = source_chunk_shape[0].max(1).min(source_shape[0].max(1));
    let chunk_xline = source_chunk_shape[1].max(1).min(source_shape[1].max(1));
    let chunk_samples = source_chunk_shape[2].max(1).min(source_shape[2].max(1));
    let total_tiles =
        source_shape[0].div_ceil(chunk_inline) * source_shape[1].div_ceil(chunk_xline);
    let bytes_per_tile = (chunk_inline as u64 * chunk_xline as u64 * chunk_samples as u64 * 4)
        .saturating_add(chunk_inline as u64 * chunk_xline as u64)
        .max(1);
    let tiles_per_partition = target_tile_group_size(
        chunk_inline as u64 * chunk_xline as u64 * chunk_samples as u64 * 4,
        chunk_inline as u64 * chunk_xline as u64,
        partition_target_bytes,
    )
    .max(1);
    let partition_count = total_tiles.div_ceil(tiles_per_partition).max(1);
    let max_active_partitions = worker_count.max(1).min(partition_count);
    let resident_partition_bytes = bytes_per_tile.saturating_mul(tiles_per_partition as u64);
    Some(ProcessingJobChunkPlanSummary {
        partition_count,
        max_active_partitions,
        tiles_per_partition,
        compatibility_target_bytes: partition_target_bytes,
        estimated_peak_bytes: resident_partition_bytes.saturating_mul(max_active_partitions as u64),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionExecutionProgress {
    pub completed_partitions: usize,
    pub total_partitions: usize,
    pub active_partitions: usize,
    pub peak_active_partitions: usize,
    pub retry_count: usize,
}

#[derive(Debug)]
struct SecondaryTraceMatrix {
    amplitudes: Vec<f32>,
    occupancy: Option<Vec<u8>>,
}

#[derive(Debug)]
struct LoadedSourceTile {
    amplitudes: Vec<f32>,
    occupancy: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CropIndexBounds {
    inline_start: usize,
    inline_end_exclusive: usize,
    xline_start: usize,
    xline_end_exclusive: usize,
    sample_start: usize,
    sample_end_exclusive: usize,
}

impl CropIndexBounds {
    fn output_shape(self) -> [usize; 3] {
        [
            self.inline_end_exclusive - self.inline_start,
            self.xline_end_exclusive - self.xline_start,
            self.sample_end_exclusive - self.sample_start,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreviewSectionPrefixReuse {
    pub cache_hit: bool,
    pub reused_prefix_operations: usize,
}

#[derive(Debug, Clone)]
pub struct PreviewSectionPrefixCache {
    max_bytes: usize,
    current_bytes: usize,
    access_counter: u64,
    entries: HashMap<PreviewSectionPrefixCacheKey, PreviewSectionPrefixCacheEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PreviewSectionPrefixCacheKey {
    artifact_cache_key: String,
}

#[derive(Debug, Clone)]
struct PreviewSectionPrefixCacheEntry {
    plane: SectionPlane,
    bytes: usize,
    prefix_operations: usize,
    last_access: u64,
}

pub struct PreviewSectionSession {
    handle: StoreHandle,
    reader: TbvolReader,
    store_root_hash: u64,
    prefix_cache: PreviewSectionPrefixCache,
}

impl Default for PreviewSectionPrefixCache {
    fn default() -> Self {
        Self::new(DEFAULT_PREVIEW_SECTION_PREFIX_CACHE_BYTES)
    }
}

impl PreviewSectionPrefixCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            current_bytes: 0,
            access_counter: 0,
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.current_bytes
    }

    fn next_access(&mut self) -> u64 {
        self.access_counter = self.access_counter.saturating_add(1);
        self.access_counter
    }

    fn longest_prefix_hit(
        &mut self,
        artifact_cache_keys: &[String],
        max_prefix_len: usize,
    ) -> Result<Option<(SectionPlane, usize)>, SeismicStoreError> {
        for prefix_len in (1..=max_prefix_len.min(artifact_cache_keys.len())).rev() {
            let key = preview_section_prefix_cache_key(&artifact_cache_keys[prefix_len - 1]);
            let access = self.next_access();
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.last_access = access;
                return Ok(Some((entry.plane.clone(), entry.prefix_operations)));
            }
        }
        Ok(None)
    }

    fn store_prefix(
        &mut self,
        prefix_len: usize,
        artifact_cache_key: &str,
        plane: &SectionPlane,
    ) -> Result<(), SeismicStoreError> {
        if prefix_len == 0 {
            return Ok(());
        }
        let key = preview_section_prefix_cache_key(artifact_cache_key);
        let bytes = preview_section_plane_bytes(plane);
        if bytes > self.max_bytes {
            return Ok(());
        }
        let access = self.next_access();
        if let Some(existing) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(existing.bytes);
        }
        self.evict_until_fits(bytes);
        self.entries.insert(
            key,
            PreviewSectionPrefixCacheEntry {
                plane: plane.clone(),
                bytes,
                prefix_operations: prefix_len,
                last_access: access,
            },
        );
        self.current_bytes = self.current_bytes.saturating_add(bytes);
        Ok(())
    }

    fn evict_until_fits(&mut self, incoming_bytes: usize) {
        while self.current_bytes.saturating_add(incoming_bytes) > self.max_bytes {
            let Some((key, bytes)) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(key, entry)| (key.clone(), entry.bytes))
            else {
                break;
            };
            self.entries.remove(&key);
            self.current_bytes = self.current_bytes.saturating_sub(bytes);
        }
    }
}

impl PreviewSectionSession {
    pub fn open(store_root: impl AsRef<Path>) -> Result<Self, SeismicStoreError> {
        Self::open_with_cache_bytes(store_root, DEFAULT_PREVIEW_SECTION_PREFIX_CACHE_BYTES)
    }

    pub fn open_with_cache_bytes(
        store_root: impl AsRef<Path>,
        cache_bytes: usize,
    ) -> Result<Self, SeismicStoreError> {
        let handle = open_store(store_root)?;
        let reader = TbvolReader::open(&handle.root)?;
        Ok(Self {
            store_root_hash: preview_store_root_hash(&handle.root),
            handle,
            reader,
            prefix_cache: PreviewSectionPrefixCache::new(cache_bytes),
        })
    }

    pub fn preview_section_view(
        &self,
        axis: SectionAxis,
        index: usize,
        pipeline: &[ProcessingOperation],
    ) -> Result<SectionView, SeismicStoreError> {
        validate_pipeline(pipeline)?;
        let secondary_readers = open_secondary_store_readers(&self.handle, None, pipeline)?;
        let plane = preview_section_from_tbvol_reader(
            &self.reader,
            axis,
            index,
            pipeline,
            &secondary_readers,
        )?;
        Ok(self.handle.section_view_from_plane(&plane))
    }

    pub fn preview_section_view_with_prefix_cache(
        &mut self,
        axis: SectionAxis,
        index: usize,
        pipeline: &[ProcessingOperation],
    ) -> Result<(SectionView, PreviewSectionPrefixReuse), SeismicStoreError> {
        validate_pipeline(pipeline)?;
        let secondary_readers = open_secondary_store_readers(&self.handle, None, pipeline)?;
        let (plane, reuse) = preview_section_from_tbvol_reader_with_prefix_cache(
            &self.handle,
            &self.reader,
            self.store_root_hash,
            axis,
            index,
            pipeline,
            &secondary_readers,
            &mut self.prefix_cache,
        )?;
        Ok((self.handle.section_view_from_plane(&plane), reuse))
    }

    pub fn read_section_plane(
        &self,
        axis: SectionAxis,
        index: usize,
    ) -> Result<SectionPlane, SeismicStoreError> {
        section_assembler::read_section_plane(&self.reader, axis, index)
    }

    pub fn preview_processing_section_plane_with_prefix_cache(
        &mut self,
        axis: SectionAxis,
        index: usize,
        pipeline: &ProcessingPipeline,
    ) -> Result<(SectionPlane, PreviewSectionPrefixReuse), SeismicStoreError> {
        validate_processing_pipeline(pipeline)?;
        let operations = trace_local_operations(pipeline);
        let secondary_readers = open_secondary_store_readers(&self.handle, None, &operations)?;
        preview_section_from_tbvol_reader_with_prefix_cache(
            &self.handle,
            &self.reader,
            self.store_root_hash,
            axis,
            index,
            &operations,
            &secondary_readers,
            &mut self.prefix_cache,
        )
    }

    pub fn section_view_from_plane(&self, plane: &SectionPlane) -> SectionView {
        self.handle.section_view_from_plane(plane)
    }

    pub fn section_count(&self, axis: SectionAxis) -> usize {
        match axis {
            SectionAxis::Inline => self.reader.volume().shape[0],
            SectionAxis::Xline => self.reader.volume().shape[1],
        }
    }

    pub fn cache_entry_count(&self) -> usize {
        self.prefix_cache.len()
    }

    pub fn cache_total_bytes(&self) -> usize {
        self.prefix_cache.total_bytes()
    }

    pub fn dataset_id(&self) -> ophiolite_seismic::DatasetId {
        self.handle.dataset_id()
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
    if pipeline.operation_count() == 0 {
        return Err(SeismicStoreError::Message(
            "processing pipeline must contain at least one operator".to_string(),
        ));
    }
    let operations = trace_local_operations(pipeline);
    validate_pipeline_for_layout(&operations, layout)
}

pub fn validate_subvolume_processing_pipeline(
    pipeline: &SubvolumeProcessingPipeline,
) -> Result<(), SeismicStoreError> {
    validate_subvolume_processing_pipeline_for_layout(pipeline, SeismicLayout::PostStack3D)
}

pub fn validate_subvolume_processing_pipeline_for_layout(
    pipeline: &SubvolumeProcessingPipeline,
    layout: SeismicLayout,
) -> Result<(), SeismicStoreError> {
    if !matches!(
        layout,
        SeismicLayout::PostStack3D | SeismicLayout::PostStack2D
    ) {
        return Err(SeismicStoreError::Message(format!(
            "subvolume processing requires post-stack layout, found {:?}",
            layout
        )));
    }

    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        validate_processing_pipeline_for_layout(trace_local_pipeline, layout)?;
    }

    validate_subvolume_crop_operation(&pipeline.crop)
}

fn trace_local_operations(pipeline: &ProcessingPipeline) -> Vec<ProcessingOperation> {
    pipeline.operations().cloned().collect()
}

fn validate_subvolume_crop_operation(
    crop: &SubvolumeCropOperation,
) -> Result<(), SeismicStoreError> {
    if crop.inline_min > crop.inline_max {
        return Err(SeismicStoreError::Message(format!(
            "crop inline_min must be <= inline_max, found [{}, {}]",
            crop.inline_min, crop.inline_max
        )));
    }
    if crop.xline_min > crop.xline_max {
        return Err(SeismicStoreError::Message(format!(
            "crop xline_min must be <= xline_max, found [{}, {}]",
            crop.xline_min, crop.xline_max
        )));
    }
    if !crop.z_min_ms.is_finite() || !crop.z_max_ms.is_finite() {
        return Err(SeismicStoreError::Message(
            "crop z_min_ms and z_max_ms must be finite".to_string(),
        ));
    }
    if crop.z_min_ms > crop.z_max_ms {
        return Err(SeismicStoreError::Message(format!(
            "crop z_min_ms must be <= z_max_ms, found [{}, {}]",
            crop.z_min_ms, crop.z_max_ms
        )));
    }
    Ok(())
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
                | ProcessingOperation::InstantaneousFrequency
                | ProcessingOperation::Sweetness
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
                | ProcessingOperation::Envelope
                | ProcessingOperation::InstantaneousPhase
                | ProcessingOperation::InstantaneousFrequency
                | ProcessingOperation::Sweetness
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
    let operations = trace_local_operations(pipeline);
    preview_section_plane(store_root, axis, index, &operations)
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
    let operations = trace_local_operations(pipeline);
    preview_section_view(store_root, axis, index, &operations)
}

pub fn preview_section_view_with_prefix_cache(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
    cache: &mut PreviewSectionPrefixCache,
) -> Result<(SectionView, PreviewSectionPrefixReuse), SeismicStoreError> {
    validate_pipeline(pipeline)?;
    let handle = open_store(store_root.as_ref())?;
    let reader = TbvolReader::open(&handle.root)?;
    let secondary_readers = open_secondary_store_readers(&handle, None, pipeline)?;
    let (plane, reuse) = preview_section_from_tbvol_reader_with_prefix_cache(
        &handle,
        &reader,
        preview_store_root_hash(&handle.root),
        axis,
        index,
        pipeline,
        &secondary_readers,
        cache,
    )?;
    Ok((handle.section_view_from_plane(&plane), reuse))
}

pub fn preview_processing_section_view_with_prefix_cache(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &ProcessingPipeline,
    cache: &mut PreviewSectionPrefixCache,
) -> Result<(SectionView, PreviewSectionPrefixReuse), SeismicStoreError> {
    validate_processing_pipeline(pipeline)?;
    let operations = trace_local_operations(pipeline);
    preview_section_view_with_prefix_cache(store_root, axis, index, &operations, cache)
}

pub fn preview_subvolume_processing_section_view(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &SubvolumeProcessingPipeline,
) -> Result<SectionView, SeismicStoreError> {
    validate_subvolume_processing_pipeline(pipeline)?;
    let handle = open_store(store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let crop_bounds = resolve_crop_bounds(reader.volume(), &pipeline.crop)?;
    let plane = preview_subvolume_section_plane_from_tbvol_reader(
        &reader,
        axis,
        index,
        pipeline.trace_local_pipeline.as_ref(),
        &crop_bounds,
        &handle,
    )?;
    Ok(handle.section_view_from_plane(&plane))
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
        options.partition_target_bytes,
        options.max_active_partitions,
        options.trace_local_chunk_plan.as_ref(),
        |_, _| Ok(()),
        |_| Ok(()),
    )?;
    copy_store_segy_export(input_store_root.as_ref(), &output_root)?;
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
    on_progress: F,
) -> Result<StoreHandle, SeismicStoreError> {
    materialize_processing_volume_with_partition_progress(
        input_store_root,
        output_store_root,
        pipeline,
        options,
        on_progress,
        |_| Ok(()),
    )
}

pub fn materialize_processing_volume_with_partition_progress<
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
    P: FnMut(PartitionExecutionProgress) -> Result<(), SeismicStoreError>,
>(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &ProcessingPipeline,
    options: MaterializeOptions,
    mut on_progress: F,
    mut on_partition_progress: P,
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
    let operations = trace_local_operations(pipeline);
    let secondary_readers = open_secondary_store_readers(
        &handle,
        Some(reader.tile_geometry().tile_shape()),
        &operations,
    )?;
    materialize_from_tbvol_reader_writer_with_progress(
        &reader,
        writer,
        &operations,
        &secondary_readers,
        options.partition_target_bytes,
        options.max_active_partitions,
        options.trace_local_chunk_plan.as_ref(),
        |completed, total| on_progress(completed, total),
        |progress| on_partition_progress(progress),
    )?;
    copy_store_segy_export(input_store_root.as_ref(), &output_root)?;
    open_store(output_root)
}

pub fn materialize_subvolume_processing_volume(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &SubvolumeProcessingPipeline,
    options: MaterializeOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    materialize_subvolume_processing_volume_with_progress(
        input_store_root,
        output_store_root,
        pipeline,
        options,
        |_, _| Ok(()),
    )
}

pub fn materialize_subvolume_processing_volume_with_progress<
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &SubvolumeProcessingPipeline,
    options: MaterializeOptions,
    mut on_progress: F,
) -> Result<StoreHandle, SeismicStoreError> {
    validate_subvolume_processing_pipeline(pipeline)?;
    let handle = open_store(&input_store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let crop_bounds = resolve_crop_bounds(reader.volume(), &pipeline.crop)?;
    let volume = derived_subvolume_volume_metadata(
        reader.volume(),
        input_store_root.as_ref(),
        pipeline,
        crop_bounds,
        options.created_by.clone(),
    );
    let chunk_shape = resolve_chunk_shape(options.chunk_shape, volume.shape);
    let has_occupancy = reader_has_occupancy(&reader)?;
    let writer = TbvolWriter::create(&output_store_root, volume, chunk_shape, has_occupancy)?;
    let output_root = writer.root().to_path_buf();
    let secondary_readers = match pipeline.trace_local_pipeline.as_ref() {
        Some(trace_local_pipeline) => {
            let operations = trace_local_operations(trace_local_pipeline);
            open_secondary_store_readers(
                &handle,
                Some(reader.tile_geometry().tile_shape()),
                &operations,
            )?
        }
        None => HashMap::new(),
    };
    materialize_subvolume_from_tbvol_reader_writer_with_progress(
        &reader,
        writer,
        pipeline.trace_local_pipeline.as_ref(),
        crop_bounds,
        &secondary_readers,
        |completed, total| on_progress(completed, total),
    )?;
    crop_store_segy_export(
        input_store_root.as_ref(),
        &output_root,
        crop_bounds.inline_start,
        crop_bounds.inline_end_exclusive,
        crop_bounds.xline_start,
        crop_bounds.xline_end_exclusive,
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

fn preview_subvolume_section_plane_from_tbvol_reader(
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    trace_local_pipeline: Option<&ProcessingPipeline>,
    crop_bounds: &CropIndexBounds,
    handle: &StoreHandle,
) -> Result<SectionPlane, SeismicStoreError> {
    let mut plane = match trace_local_pipeline {
        Some(pipeline) => {
            let operations = trace_local_operations(pipeline);
            let secondary_readers = open_secondary_store_readers(handle, None, &operations)?;
            preview_section_from_tbvol_reader(reader, axis, index, &operations, &secondary_readers)?
        }
        None => section_assembler::read_section_plane(reader, axis, index)?,
    };
    crop_section_plane(&mut plane, *crop_bounds)?;
    Ok(plane)
}

fn crop_section_plane(
    plane: &mut SectionPlane,
    crop_bounds: CropIndexBounds,
) -> Result<(), SeismicStoreError> {
    let (section_start, section_end_exclusive, horizontal_start, horizontal_end_exclusive) =
        match plane.axis {
            SectionAxis::Inline => (
                crop_bounds.inline_start,
                crop_bounds.inline_end_exclusive,
                crop_bounds.xline_start,
                crop_bounds.xline_end_exclusive,
            ),
            SectionAxis::Xline => (
                crop_bounds.xline_start,
                crop_bounds.xline_end_exclusive,
                crop_bounds.inline_start,
                crop_bounds.inline_end_exclusive,
            ),
        };

    if plane.coordinate_index < section_start || plane.coordinate_index >= section_end_exclusive {
        return Err(SeismicStoreError::Message(format!(
            "current {:?} section lies outside the crop window",
            plane.axis
        )));
    }

    let output_traces = horizontal_end_exclusive - horizontal_start;
    let output_samples = crop_bounds.sample_end_exclusive - crop_bounds.sample_start;
    let mut cropped = vec![0.0_f32; output_traces * output_samples];
    for output_trace_index in 0..output_traces {
        let source_trace_index = horizontal_start + output_trace_index;
        let source_trace_start = source_trace_index * plane.samples;
        let output_trace_start = output_trace_index * output_samples;
        cropped[output_trace_start..output_trace_start + output_samples].copy_from_slice(
            &plane.amplitudes[source_trace_start + crop_bounds.sample_start
                ..source_trace_start + crop_bounds.sample_end_exclusive],
        );
    }

    plane.coordinate_index -= section_start;
    plane.traces = output_traces;
    plane.samples = output_samples;
    plane.horizontal_axis =
        plane.horizontal_axis[horizontal_start..horizontal_end_exclusive].to_vec();
    plane.sample_axis_ms =
        plane.sample_axis_ms[crop_bounds.sample_start..crop_bounds.sample_end_exclusive].to_vec();
    plane.amplitudes = cropped;
    plane.occupancy = plane
        .occupancy
        .as_ref()
        .map(|mask| mask[horizontal_start..horizontal_end_exclusive].to_vec());
    Ok(())
}

fn resolve_crop_bounds(
    volume: &VolumeMetadata,
    crop: &SubvolumeCropOperation,
) -> Result<CropIndexBounds, SeismicStoreError> {
    validate_subvolume_crop_operation(crop)?;
    let (inline_start, inline_end_exclusive) = resolve_i32_axis_bounds(
        "inline",
        &volume.axes.ilines,
        crop.inline_min,
        crop.inline_max,
    )?;
    let (xline_start, xline_end_exclusive) =
        resolve_i32_axis_bounds("xline", &volume.axes.xlines, crop.xline_min, crop.xline_max)?;
    let (sample_start, sample_end_exclusive) = resolve_f32_axis_bounds(
        "sample",
        &volume.axes.sample_axis_ms,
        crop.z_min_ms,
        crop.z_max_ms,
    )?;

    if inline_start == 0
        && inline_end_exclusive == volume.shape[0]
        && xline_start == 0
        && xline_end_exclusive == volume.shape[1]
        && sample_start == 0
        && sample_end_exclusive == volume.shape[2]
    {
        return Err(SeismicStoreError::Message(
            "crop window must be a strict subset of the source volume".to_string(),
        ));
    }

    Ok(CropIndexBounds {
        inline_start,
        inline_end_exclusive,
        xline_start,
        xline_end_exclusive,
        sample_start,
        sample_end_exclusive,
    })
}

fn resolve_i32_axis_bounds(
    label: &str,
    axis: &[f64],
    min_value: i32,
    max_value: i32,
) -> Result<(usize, usize), SeismicStoreError> {
    if axis.is_empty() {
        return Err(SeismicStoreError::Message(format!("{label} axis is empty")));
    }

    let min_index = axis
        .iter()
        .position(|value| (*value).round() as i32 == min_value)
        .ok_or_else(|| {
            SeismicStoreError::Message(format!(
                "crop {label}_min {min_value} does not match a source axis value"
            ))
        })?;
    let max_index = axis
        .iter()
        .position(|value| (*value).round() as i32 == max_value)
        .ok_or_else(|| {
            SeismicStoreError::Message(format!(
                "crop {label}_max {max_value} does not match a source axis value"
            ))
        })?;

    Ok((
        min_index.min(max_index),
        min_index.max(max_index).saturating_add(1),
    ))
}

fn resolve_f32_axis_bounds(
    label: &str,
    axis: &[f32],
    min_value: f32,
    max_value: f32,
) -> Result<(usize, usize), SeismicStoreError> {
    if axis.is_empty() {
        return Err(SeismicStoreError::Message(format!("{label} axis is empty")));
    }

    let axis_min = axis.iter().copied().fold(f32::INFINITY, f32::min);
    let axis_max = axis.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    if min_value < axis_min || max_value > axis_max {
        return Err(SeismicStoreError::Message(format!(
            "crop {label} bounds [{min_value}, {max_value}] exceed source extent [{axis_min}, {axis_max}]"
        )));
    }

    let contained_indexes = axis
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            ((*value >= min_value) && (*value <= max_value)).then_some(index)
        })
        .collect::<Vec<_>>();
    let Some(start) = contained_indexes.first().copied() else {
        return Err(SeismicStoreError::Message(format!(
            "crop {label} bounds [{min_value}, {max_value}] do not contain any source samples"
        )));
    };
    let end_exclusive = contained_indexes
        .last()
        .copied()
        .expect("contained indexes should have at least one element")
        + 1;
    Ok((start, end_exclusive))
}

fn derived_subvolume_volume_metadata(
    input: &VolumeMetadata,
    parent_store: &Path,
    pipeline: &SubvolumeProcessingPipeline,
    crop_bounds: CropIndexBounds,
    created_by: String,
) -> VolumeMetadata {
    VolumeMetadata {
        kind: DatasetKind::Derived,
        store_id: generate_store_id(),
        source: input.source.clone(),
        shape: crop_bounds.output_shape(),
        axes: crate::metadata::VolumeAxes {
            ilines: input.axes.ilines[crop_bounds.inline_start..crop_bounds.inline_end_exclusive]
                .to_vec(),
            xlines: input.axes.xlines[crop_bounds.xline_start..crop_bounds.xline_end_exclusive]
                .to_vec(),
            sample_axis_domain: input.axes.sample_axis_domain,
            sample_axis_unit: input.axes.sample_axis_unit.clone(),
            sample_axis_ms: input.axes.sample_axis_ms
                [crop_bounds.sample_start..crop_bounds.sample_end_exclusive]
                .to_vec(),
        },
        segy_export: None,
        coordinate_reference_binding: input.coordinate_reference_binding.clone(),
        spatial: input.spatial.clone(),
        created_by,
        processing_lineage: Some(ProcessingLineage {
            schema_version: 1,
            parent_store: parent_store.to_path_buf(),
            parent_store_id: input.store_id.clone(),
            artifact_role: ProcessingArtifactRole::FinalOutput,
            pipeline: ProcessingPipelineSpec::Subvolume {
                pipeline: pipeline.clone(),
            },
            pipeline_identity: None,
            operator_set_identity: None,
            planner_profile_identity: None,
            source_identity: None,
            runtime_semantics_version: CURRENT_RUNTIME_SEMANTICS_VERSION.to_string(),
            store_writer_semantics_version: CURRENT_STORE_WRITER_SEMANTICS_VERSION.to_string(),
            runtime_version: RUNTIME_VERSION.to_string(),
            created_at_unix_s: unix_timestamp_s(),
            artifact_key: None,
            input_artifact_keys: Vec::new(),
            produced_by_stage_id: None,
            boundary_reason: None,
            logical_domain: None,
            chunk_grid_spec: None,
            geometry_fingerprints: None,
        }),
    }
}

fn materialize_subvolume_from_tbvol_reader_writer_with_progress<
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    reader: &TbvolReader,
    writer: W,
    trace_local_pipeline: Option<&ProcessingPipeline>,
    crop_bounds: CropIndexBounds,
    secondary_readers: &HashMap<String, TbvolReader>,
    mut on_progress: F,
) -> Result<(), SeismicStoreError> {
    if let Some(pipeline) = trace_local_pipeline {
        validate_processing_pipeline(pipeline)?;
    }

    let output_geometry = writer.tile_geometry().clone();
    let output_tile_shape = output_geometry.tile_shape();
    let source_sample_count = reader.volume().shape[2];
    let sample_interval_ms = reader.volume().source.sample_interval_us as f32 / 1000.0;
    let total_tiles = output_geometry.tile_count();
    let trace_count = output_tile_shape[0] * output_tile_shape[1];
    let cropped_sample_count = output_tile_shape[2];
    let mut completed_tiles = 0;

    for tile in output_geometry.iter_tiles() {
        let effective = output_geometry.effective_tile_shape(tile);
        let origin = output_geometry.tile_origin(tile);
        let source_inline_start = crop_bounds.inline_start + origin[0];
        let source_xline_start = crop_bounds.xline_start + origin[1];

        let primary_full = assemble_source_trace_matrix(
            reader,
            source_inline_start,
            source_xline_start,
            output_tile_shape,
            [effective[0], effective[1]],
            0,
            source_sample_count,
        )?;

        let occupancy = primary_full.occupancy.clone();
        let amplitudes = if let Some(pipeline) = trace_local_pipeline {
            let operations = trace_local_operations(pipeline);
            let mut full_trace_amplitudes = primary_full.amplitudes;
            let secondary_inputs = if pipeline_requires_external_volume_inputs(&operations) {
                Some(load_secondary_trace_matrices_for_crop_tile(
                    secondary_readers,
                    source_inline_start,
                    source_xline_start,
                    output_tile_shape,
                    [effective[0], effective[1]],
                    0,
                    source_sample_count,
                )?)
            } else {
                None
            };
            apply_pipeline_to_traces_internal(
                &mut full_trace_amplitudes,
                trace_count,
                source_sample_count,
                sample_interval_ms,
                occupancy.as_deref(),
                &operations,
                secondary_inputs.as_ref(),
            )?;
            crop_trace_matrix_samples(
                &full_trace_amplitudes,
                trace_count,
                source_sample_count,
                crop_bounds.sample_start,
                crop_bounds.sample_end_exclusive,
            )
        } else {
            crop_trace_matrix_samples(
                &primary_full.amplitudes,
                trace_count,
                source_sample_count,
                crop_bounds.sample_start,
                crop_bounds.sample_end_exclusive,
            )
        };

        debug_assert_eq!(amplitudes.len(), trace_count * cropped_sample_count);
        writer.write_tile(tile, &amplitudes)?;
        if let Some(mask) = occupancy.as_deref() {
            writer.write_tile_occupancy(tile, mask)?;
        }
        completed_tiles += 1;
        on_progress(completed_tiles, total_tiles)?;
    }

    writer.finalize()
}

fn load_secondary_trace_matrices_for_crop_tile(
    readers: &HashMap<String, TbvolReader>,
    source_inline_start: usize,
    source_xline_start: usize,
    tile_shape: [usize; 3],
    effective_trace_shape: [usize; 2],
    sample_start: usize,
    sample_end_exclusive: usize,
) -> Result<HashMap<String, SecondaryTraceMatrix>, SeismicStoreError> {
    let mut inputs = HashMap::with_capacity(readers.len());
    for (store_path, reader) in readers {
        let matrix = assemble_source_trace_matrix(
            reader,
            source_inline_start,
            source_xline_start,
            tile_shape,
            effective_trace_shape,
            sample_start,
            sample_end_exclusive,
        )?;
        inputs.insert(
            store_path.clone(),
            SecondaryTraceMatrix {
                amplitudes: matrix.amplitudes,
                occupancy: matrix.occupancy,
            },
        );
    }
    Ok(inputs)
}

fn assemble_source_trace_matrix(
    reader: &TbvolReader,
    source_inline_start: usize,
    source_xline_start: usize,
    tile_shape: [usize; 3],
    effective_trace_shape: [usize; 2],
    sample_start: usize,
    sample_end_exclusive: usize,
) -> Result<LoadedSourceTile, SeismicStoreError> {
    let sample_count = sample_end_exclusive.saturating_sub(sample_start);
    let trace_count = tile_shape[0] * tile_shape[1];
    let mut amplitudes = vec![0.0_f32; trace_count * sample_count];
    let mut occupancy = vec![0_u8; trace_count];
    let mut has_occupancy = false;
    let mut cache = HashMap::<TileCoord, LoadedSourceTile>::new();
    let source_tile_shape = reader.tile_geometry().tile_shape();

    for local_i in 0..effective_trace_shape[0] {
        for local_x in 0..effective_trace_shape[1] {
            let source_inline = source_inline_start + local_i;
            let source_xline = source_xline_start + local_x;
            let source_tile = TileCoord {
                tile_i: source_inline / source_tile_shape[0],
                tile_x: source_xline / source_tile_shape[1],
            };
            let tile = if let Some(tile) = cache.get(&source_tile) {
                tile
            } else {
                let loaded = LoadedSourceTile {
                    amplitudes: reader.read_tile(source_tile)?.into_owned(),
                    occupancy: reader
                        .read_tile_occupancy(source_tile)?
                        .map(|mask| mask.into_owned()),
                };
                cache.entry(source_tile).or_insert(loaded)
            };

            let source_local_i = source_inline % source_tile_shape[0];
            let source_local_x = source_xline % source_tile_shape[1];
            let source_trace_start =
                ((source_local_i * source_tile_shape[1]) + source_local_x) * source_tile_shape[2];
            let destination_trace_index = (local_i * tile_shape[1]) + local_x;
            let destination_trace_start = destination_trace_index * sample_count;
            amplitudes[destination_trace_start..destination_trace_start + sample_count]
                .copy_from_slice(
                    &tile.amplitudes[source_trace_start + sample_start
                        ..source_trace_start + sample_end_exclusive],
                );
            has_occupancy |= tile.occupancy.is_some();
            occupancy[destination_trace_index] = tile
                .occupancy
                .as_ref()
                .and_then(|mask| mask.get(source_local_i * source_tile_shape[1] + source_local_x))
                .copied()
                .unwrap_or(1);
        }
    }

    Ok(LoadedSourceTile {
        amplitudes,
        occupancy: has_occupancy.then_some(occupancy),
    })
}

fn crop_trace_matrix_samples(
    source: &[f32],
    traces: usize,
    source_samples: usize,
    sample_start: usize,
    sample_end_exclusive: usize,
) -> Vec<f32> {
    let output_samples = sample_end_exclusive.saturating_sub(sample_start);
    let mut cropped = vec![0.0_f32; traces * output_samples];
    for trace_index in 0..traces {
        let source_trace_start = trace_index * source_samples;
        let destination_trace_start = trace_index * output_samples;
        cropped[destination_trace_start..destination_trace_start + output_samples].copy_from_slice(
            &source[source_trace_start + sample_start..source_trace_start + sample_end_exclusive],
        );
    }
    cropped
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
    materialize_from_reader_writer_internal(reader, writer, pipeline, &mut on_progress)
}

fn materialize_from_reader_writer_internal<
    R: VolumeStoreReader,
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    reader: &R,
    writer: W,
    pipeline: &[ProcessingOperation],
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
        apply_pipeline_to_traces_internal(
            &mut amplitudes,
            traces,
            samples,
            sample_interval_ms,
            occupancy.as_deref(),
            pipeline,
            None,
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

fn preview_section_from_tbvol_reader_with_prefix_cache(
    handle: &StoreHandle,
    reader: &TbvolReader,
    store_root_hash: u64,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
    cache: &mut PreviewSectionPrefixCache,
) -> Result<(SectionPlane, PreviewSectionPrefixReuse), SeismicStoreError> {
    validate_pipeline(pipeline)?;
    if pipeline_requires_external_volume_inputs(pipeline) {
        let plane =
            preview_section_from_tbvol_reader(reader, axis, index, pipeline, secondary_readers)?;
        return Ok((plane, PreviewSectionPrefixReuse::default()));
    }

    let max_cacheable_prefix_len = pipeline
        .iter()
        .take_while(|operation| {
            operation
                .dependency_profile()
                .same_section_ephemeral_reuse_safe
        })
        .count();
    let prefix_hashes = preview_prefix_hashes(pipeline);
    let prefix_cache_keys =
        preview_prefix_artifact_cache_keys(handle, store_root_hash, axis, index, &prefix_hashes)?;

    let mut reuse = PreviewSectionPrefixReuse::default();
    let mut plane = if let Some((cached_plane, prefix_len)) =
        cache.longest_prefix_hit(&prefix_cache_keys, max_cacheable_prefix_len)?
    {
        reuse.cache_hit = true;
        reuse.reused_prefix_operations = prefix_len;
        cached_plane
    } else {
        section_assembler::read_section_plane(reader, axis, index)?
    };

    if reuse.cache_hit {
        if reuse.reused_prefix_operations < pipeline.len() {
            apply_pipeline_to_plane(&mut plane, &pipeline[reuse.reused_prefix_operations..])?;
        }
    } else {
        for prefix_len in 1..=pipeline.len() {
            let operation = &pipeline[prefix_len - 1];
            apply_pipeline_to_plane(&mut plane, std::slice::from_ref(operation))?;
            if prefix_len <= max_cacheable_prefix_len {
                cache.store_prefix(prefix_len, &prefix_cache_keys[prefix_len - 1], &plane)?;
            }
        }
    }

    Ok((plane, reuse))
}

fn materialize_from_tbvol_reader_writer_with_progress<
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
    P: FnMut(PartitionExecutionProgress) -> Result<(), SeismicStoreError>,
>(
    reader: &TbvolReader,
    writer: W,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
    partition_target_bytes: Option<u64>,
    max_active_partitions: Option<usize>,
    trace_local_chunk_plan: Option<&TraceLocalChunkPlanRecommendation>,
    mut on_progress: F,
    mut on_partition_progress: P,
) -> Result<(), SeismicStoreError> {
    materialize_from_tbvol_reader_writer_internal(
        reader,
        writer,
        pipeline,
        secondary_readers,
        partition_target_bytes,
        max_active_partitions,
        trace_local_chunk_plan,
        &mut on_progress,
        &mut on_partition_progress,
    )
}

fn materialize_from_tbvol_reader_writer_internal<
    W: VolumeStoreWriter,
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
    P: FnMut(PartitionExecutionProgress) -> Result<(), SeismicStoreError>,
>(
    reader: &TbvolReader,
    writer: W,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
    partition_target_bytes: Option<u64>,
    max_active_partitions: Option<usize>,
    trace_local_chunk_plan: Option<&TraceLocalChunkPlanRecommendation>,
    on_progress: &mut F,
    on_partition_progress: &mut P,
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

    let chunk_execution = resolve_trace_local_chunk_execution(
        reader.tile_geometry(),
        partition_target_bytes,
        max_active_partitions,
        trace_local_chunk_plan,
    );

    if let Some(tiles_per_partition) = chunk_execution.tiles_per_partition {
        let tile_groups =
            partition_tile_groups_for_tile_count(reader.tile_geometry(), tiles_per_partition);
        if tile_groups.len() > 1 {
            let max_groups_per_batch = chunk_execution
                .max_active_partitions
                .unwrap_or_else(|| compute_pool().current_num_threads())
                .max(1)
                .min(compute_pool().current_num_threads().max(1));
            let total_partitions = tile_groups.len();
            let mut completed_partitions = 0usize;
            let mut peak_active_partitions = 0usize;
            for group_batch in tile_groups.chunks(max_groups_per_batch) {
                let active_partitions = group_batch.len();
                peak_active_partitions = peak_active_partitions.max(active_partitions);
                on_partition_progress(PartitionExecutionProgress {
                    completed_partitions,
                    total_partitions,
                    active_partitions,
                    peak_active_partitions,
                    retry_count: 0,
                })?;
                let processed_batches = compute_pool().install(|| {
                    group_batch
                        .par_iter()
                        .map(|group| {
                            process_tile_group(
                                reader,
                                pipeline,
                                secondary_readers,
                                traces,
                                samples,
                                sample_interval_ms,
                                group,
                            )
                        })
                        .collect::<Result<Vec<_>, SeismicStoreError>>()
                })?;
                for outputs in processed_batches {
                    for output in outputs {
                        writer.write_tile(output.tile, &output.amplitudes)?;
                        if let Some(mask) = output.occupancy.as_deref() {
                            writer.write_tile_occupancy(output.tile, mask)?;
                        }
                        completed_tiles += 1;
                        on_progress(completed_tiles, total_tiles)?;
                    }
                    completed_partitions += 1;
                    on_partition_progress(PartitionExecutionProgress {
                        completed_partitions,
                        total_partitions,
                        active_partitions: 0,
                        peak_active_partitions,
                        retry_count: 0,
                    })?;
                }
            }
            return writer.finalize();
        }
    }

    on_partition_progress(PartitionExecutionProgress {
        completed_partitions: 0,
        total_partitions: 1,
        active_partitions: 1,
        peak_active_partitions: 1,
        retry_count: 0,
    })?;
    for tile in reader.tile_geometry().iter_tiles() {
        let output = process_tile(
            reader,
            pipeline,
            secondary_readers,
            traces,
            samples,
            sample_interval_ms,
            tile,
        )?;
        writer.write_tile(output.tile, &output.amplitudes)?;
        if let Some(mask) = output.occupancy.as_deref() {
            writer.write_tile_occupancy(output.tile, mask)?;
        }
        completed_tiles += 1;
        on_progress(completed_tiles, total_tiles)?;
    }
    writer.finalize()?;
    on_partition_progress(PartitionExecutionProgress {
        completed_partitions: 1,
        total_partitions: 1,
        active_partitions: 0,
        peak_active_partitions: 1,
        retry_count: 0,
    })?;
    Ok(())
}

#[derive(Debug)]
struct ProcessedTileOutput {
    tile: TileCoord,
    amplitudes: Vec<f32>,
    occupancy: Option<Vec<u8>>,
}

fn process_tile_group(
    reader: &TbvolReader,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
    traces: usize,
    samples: usize,
    sample_interval_ms: f32,
    tiles: &[TileCoord],
) -> Result<Vec<ProcessedTileOutput>, SeismicStoreError> {
    tiles
        .iter()
        .copied()
        .map(|tile| {
            process_tile(
                reader,
                pipeline,
                secondary_readers,
                traces,
                samples,
                sample_interval_ms,
                tile,
            )
        })
        .collect()
}

fn process_tile(
    reader: &TbvolReader,
    pipeline: &[ProcessingOperation],
    secondary_readers: &HashMap<String, TbvolReader>,
    traces: usize,
    samples: usize,
    sample_interval_ms: f32,
    tile: TileCoord,
) -> Result<ProcessedTileOutput, SeismicStoreError> {
    let mut amplitudes = reader.read_tile(tile)?.into_owned();
    let occupancy = reader
        .read_tile_occupancy(tile)?
        .map(|value| value.into_owned());
    let secondary_inputs = if pipeline_requires_external_volume_inputs(pipeline) {
        Some(load_secondary_tile_inputs(tile, secondary_readers)?)
    } else {
        None
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
    Ok(ProcessedTileOutput {
        tile,
        amplitudes,
        occupancy,
    })
}

#[cfg(test)]
fn partition_tile_groups_for_target_bytes(
    geometry: &crate::storage::tile_geometry::TileGeometry,
    target_bytes: u64,
) -> Vec<Vec<TileCoord>> {
    let target_tiles = target_tile_group_size(
        geometry.amplitude_tile_bytes(),
        geometry.occupancy_tile_bytes(),
        target_bytes,
    );
    let tiles = geometry.iter_tiles().collect::<Vec<_>>();
    tiles
        .chunks(target_tiles.max(1))
        .map(|chunk| chunk.to_vec())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TraceLocalChunkExecution {
    tiles_per_partition: Option<usize>,
    max_active_partitions: Option<usize>,
}

fn resolve_trace_local_chunk_execution(
    geometry: &crate::storage::tile_geometry::TileGeometry,
    partition_target_bytes: Option<u64>,
    max_active_partitions: Option<usize>,
    trace_local_chunk_plan: Option<&TraceLocalChunkPlanRecommendation>,
) -> TraceLocalChunkExecution {
    if let Some(plan) = trace_local_chunk_plan {
        return TraceLocalChunkExecution {
            tiles_per_partition: Some(plan.tiles_per_partition.max(1)),
            max_active_partitions: Some(
                max_active_partitions
                    .unwrap_or(plan.max_active_partitions)
                    .max(1)
                    .min(plan.max_active_partitions.max(1)),
            ),
        };
    }

    TraceLocalChunkExecution {
        tiles_per_partition: partition_target_bytes.map(|target_bytes| {
            target_tile_group_size(
                geometry.amplitude_tile_bytes(),
                geometry.occupancy_tile_bytes(),
                target_bytes,
            )
        }),
        max_active_partitions,
    }
}

fn partition_tile_groups_for_tile_count(
    geometry: &crate::storage::tile_geometry::TileGeometry,
    tiles_per_partition: usize,
) -> Vec<Vec<TileCoord>> {
    let tiles = geometry.iter_tiles().collect::<Vec<_>>();
    tiles
        .chunks(tiles_per_partition.max(1))
        .map(|chunk| chunk.to_vec())
        .collect()
}

fn target_tile_group_size(
    amplitude_tile_bytes: u64,
    occupancy_tile_bytes: u64,
    target_bytes: u64,
) -> usize {
    let bytes_per_tile = amplitude_tile_bytes
        .saturating_add(occupancy_tile_bytes)
        .max(1);
    (target_bytes / bytes_per_tile).max(1) as usize
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
        ProcessingOperation::Envelope => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_envelope(trace)?;
        }
        ProcessingOperation::InstantaneousPhase => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_instantaneous_phase(trace)?;
        }
        ProcessingOperation::InstantaneousFrequency => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_instantaneous_frequency(trace, sample_interval_ms)?;
        }
        ProcessingOperation::Sweetness => {
            state
                .spectral
                .as_mut()
                .expect("spectral workspace should exist when spectral operators are present")
                .apply_sweetness(trace, sample_interval_ms)?;
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

fn preview_section_prefix_cache_key(artifact_cache_key: &str) -> PreviewSectionPrefixCacheKey {
    PreviewSectionPrefixCacheKey {
        artifact_cache_key: artifact_cache_key.to_string(),
    }
}

fn preview_store_root_hash(store_root: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(store_root.as_os_str().to_string_lossy().as_bytes());
    hasher.finish()
}

fn preview_prefix_hashes(pipeline: &[ProcessingOperation]) -> Vec<u64> {
    let mut hasher = DefaultHasher::new();
    let mut hashes = Vec::with_capacity(pipeline.len());
    for operation in pipeline {
        hash_processing_operation(&mut hasher, operation);
        hashes.push(hasher.finish());
    }
    hashes
}

fn preview_prefix_artifact_cache_keys(
    handle: &StoreHandle,
    store_root_hash: u64,
    axis: SectionAxis,
    index: usize,
    prefix_hashes: &[u64],
) -> Result<Vec<String>, SeismicStoreError> {
    let geometry_fingerprints = handle.geometry_fingerprints();
    let chunk_grid_spec = ChunkGridSpec::Regular {
        origin: [0, 0, 0],
        chunk_shape: handle.manifest.tile_shape,
    };
    prefix_hashes
        .iter()
        .enumerate()
        .map(|(prefix_index, prefix_hash)| {
            Ok(preview_section_artifact_key(
                store_root_hash,
                axis,
                index,
                prefix_index + 1,
                *prefix_hash,
                geometry_fingerprints.clone(),
                chunk_grid_spec.clone(),
            )?
            .cache_key)
        })
        .collect()
}

fn preview_section_artifact_key(
    store_root_hash: u64,
    axis: SectionAxis,
    index: usize,
    prefix_len: usize,
    prefix_hash: u64,
    geometry_fingerprints: GeometryFingerprints,
    chunk_grid_spec: ChunkGridSpec,
) -> Result<ArtifactKey, SeismicStoreError> {
    let logical_domain = LogicalDomain::Section {
        section: SectionDomain {
            axis,
            section_index: index,
        },
    };
    let lineage_digest = crate::ProcessingCacheFingerprint::fingerprint_json(&(
        store_root_hash,
        match axis {
            SectionAxis::Inline => 0_u8,
            SectionAxis::Xline => 1_u8,
        },
        index,
        prefix_len,
        prefix_hash,
    ))
    .map_err(SeismicStoreError::Message)?;
    let cache_key = crate::ProcessingCacheFingerprint::fingerprint_json(&(
        &lineage_digest,
        &geometry_fingerprints,
        &logical_domain,
        &chunk_grid_spec,
        MaterializationClass::EphemeralWindow,
    ))
    .map_err(SeismicStoreError::Message)?;
    Ok(ArtifactKey {
        lineage_digest,
        geometry_fingerprints,
        logical_domain,
        chunk_grid_spec,
        materialization_class: MaterializationClass::EphemeralWindow,
        cache_key,
    })
}

fn hash_processing_operation(hasher: &mut DefaultHasher, operation: &ProcessingOperation) {
    match operation {
        ProcessingOperation::AmplitudeScalar { factor } => {
            hasher.write_u8(0);
            hasher.write_u32(factor.to_bits());
        }
        ProcessingOperation::TraceRmsNormalize => {
            hasher.write_u8(1);
        }
        ProcessingOperation::AgcRms { window_ms } => {
            hasher.write_u8(2);
            hasher.write_u32(window_ms.to_bits());
        }
        ProcessingOperation::PhaseRotation { angle_degrees } => {
            hasher.write_u8(3);
            hasher.write_u32(angle_degrees.to_bits());
        }
        ProcessingOperation::Envelope => {
            hasher.write_u8(4);
        }
        ProcessingOperation::InstantaneousPhase => {
            hasher.write_u8(5);
        }
        ProcessingOperation::InstantaneousFrequency => {
            hasher.write_u8(6);
        }
        ProcessingOperation::Sweetness => {
            hasher.write_u8(7);
        }
        ProcessingOperation::LowpassFilter {
            f3_hz,
            f4_hz,
            phase,
            window,
        } => {
            hasher.write_u8(8);
            hasher.write_u32(f3_hz.to_bits());
            hasher.write_u32(f4_hz.to_bits());
            hasher.write_u8(match phase {
                FrequencyPhaseMode::Zero => 0,
            });
            hasher.write_u8(match window {
                FrequencyWindowShape::CosineTaper => 0,
            });
        }
        ProcessingOperation::HighpassFilter {
            f1_hz,
            f2_hz,
            phase,
            window,
        } => {
            hasher.write_u8(9);
            hasher.write_u32(f1_hz.to_bits());
            hasher.write_u32(f2_hz.to_bits());
            hasher.write_u8(match phase {
                FrequencyPhaseMode::Zero => 0,
            });
            hasher.write_u8(match window {
                FrequencyWindowShape::CosineTaper => 0,
            });
        }
        ProcessingOperation::BandpassFilter {
            f1_hz,
            f2_hz,
            f3_hz,
            f4_hz,
            phase,
            window,
        } => {
            hasher.write_u8(10);
            hasher.write_u32(f1_hz.to_bits());
            hasher.write_u32(f2_hz.to_bits());
            hasher.write_u32(f3_hz.to_bits());
            hasher.write_u32(f4_hz.to_bits());
            hasher.write_u8(match phase {
                FrequencyPhaseMode::Zero => 0,
            });
            hasher.write_u8(match window {
                FrequencyWindowShape::CosineTaper => 0,
            });
        }
        ProcessingOperation::VolumeArithmetic {
            operator,
            secondary_store_path,
        } => {
            hasher.write_u8(11);
            hasher.write_u8(match operator {
                TraceLocalVolumeArithmeticOperator::Add => 0,
                TraceLocalVolumeArithmeticOperator::Subtract => 1,
                TraceLocalVolumeArithmeticOperator::Multiply => 2,
                TraceLocalVolumeArithmeticOperator::Divide => 3,
            });
            hasher.write(secondary_store_path.as_bytes());
        }
    }
}

fn preview_section_plane_bytes(plane: &SectionPlane) -> usize {
    plane.amplitudes.len() * std::mem::size_of::<f32>()
        + plane.sample_axis_ms.len() * std::mem::size_of::<f32>()
        + plane.horizontal_axis.len() * std::mem::size_of::<f64>()
        + plane.occupancy.as_ref().map_or(0, Vec::len)
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
    analytic_hilbert: Vec<f32>,
    derivative_real: Vec<f32>,
    derivative_hilbert: Vec<f32>,
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
            analytic_hilbert: vec![0.0; samples],
            derivative_real: vec![0.0; samples],
            derivative_hilbert: vec![0.0; samples],
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

    fn apply_envelope(&mut self, trace: &mut [f32]) -> Result<(), SeismicStoreError> {
        self.compute_hilbert_transform(trace, "envelope")?;
        for (sample, hilbert) in trace.iter_mut().zip(self.output.iter()) {
            *sample = sample.hypot(*hilbert);
        }
        Ok(())
    }

    fn apply_instantaneous_phase(&mut self, trace: &mut [f32]) -> Result<(), SeismicStoreError> {
        self.compute_hilbert_transform(trace, "instantaneous phase")?;
        for (sample, hilbert) in trace.iter_mut().zip(self.output.iter()) {
            *sample = wrap_phase_degrees(hilbert.atan2(*sample).to_degrees());
        }
        Ok(())
    }

    fn apply_instantaneous_frequency(
        &mut self,
        trace: &mut [f32],
        sample_interval_ms: f32,
    ) -> Result<(), SeismicStoreError> {
        let dt_s = trace_sample_interval_seconds(sample_interval_ms)?;
        self.compute_hilbert_transform(trace, "instantaneous frequency")?;
        self.analytic_hilbert.copy_from_slice(&self.output);
        differentiate_trace(trace, dt_s, &mut self.derivative_real)?;
        differentiate_trace(&self.analytic_hilbert, dt_s, &mut self.derivative_hilbert)?;

        for index in 0..trace.len() {
            let real = trace[index];
            let hilbert = self.analytic_hilbert[index];
            let denominator = real * real + hilbert * hilbert + INSTANTANEOUS_FREQUENCY_EPSILON;
            let angular_frequency = (real * self.derivative_hilbert[index]
                - self.derivative_real[index] * hilbert)
                / denominator;
            self.output[index] = angular_frequency / (2.0 * std::f32::consts::PI);
        }
        trace.copy_from_slice(&self.output[..trace.len()]);
        Ok(())
    }

    fn apply_sweetness(
        &mut self,
        trace: &mut [f32],
        sample_interval_ms: f32,
    ) -> Result<(), SeismicStoreError> {
        let dt_s = trace_sample_interval_seconds(sample_interval_ms)?;
        self.compute_hilbert_transform(trace, "sweetness")?;
        self.analytic_hilbert.copy_from_slice(&self.output);
        differentiate_trace(trace, dt_s, &mut self.derivative_real)?;
        differentiate_trace(&self.analytic_hilbert, dt_s, &mut self.derivative_hilbert)?;

        for index in 0..trace.len() {
            let real = trace[index];
            let hilbert = self.analytic_hilbert[index];
            let envelope = real.hypot(hilbert);
            let denominator = real * real + hilbert * hilbert + INSTANTANEOUS_FREQUENCY_EPSILON;
            let angular_frequency = (real * self.derivative_hilbert[index]
                - self.derivative_real[index] * hilbert)
                / denominator;
            let frequency_hz = angular_frequency / (2.0 * std::f32::consts::PI);
            self.output[index] = envelope / frequency_hz.max(SWEETNESS_FREQUENCY_FLOOR_HZ).sqrt();
        }
        trace.copy_from_slice(&self.output[..trace.len()]);
        Ok(())
    }

    fn compute_hilbert_transform(
        &mut self,
        trace: &[f32],
        label: &str,
    ) -> Result<(), SeismicStoreError> {
        if trace.len() != self.input.len() {
            return Err(SeismicStoreError::Message(format!(
                "{label} workspace length mismatch: expected {}, found {}",
                self.input.len(),
                trace.len()
            )));
        }

        self.input.copy_from_slice(trace);
        self.forward
            .process(&mut self.input, &mut self.spectrum)
            .map_err(|error| {
                SeismicStoreError::Message(format!("{label} forward FFT failed: {error}"))
            })?;
        apply_hilbert_response(&mut self.spectrum, trace.len());
        self.inverse
            .process(&mut self.spectrum, &mut self.output)
            .map_err(|error| {
                SeismicStoreError::Message(format!("{label} inverse FFT failed: {error}"))
            })?;

        let inverse_scale = 1.0 / trace.len().max(1) as f32;
        for value in &mut self.output[..trace.len()] {
            *value *= inverse_scale;
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

fn apply_hilbert_response(spectrum: &mut [Complex32], samples: usize) {
    let multiplier = Complex32::new(0.0, -1.0);
    for (index, value) in spectrum.iter_mut().enumerate() {
        let is_dc = index == 0;
        let is_nyquist = samples % 2 == 0 && index == samples / 2;
        if is_dc || is_nyquist {
            *value = Complex32::default();
            continue;
        }
        *value *= multiplier;
    }
}

fn trace_sample_interval_seconds(sample_interval_ms: f32) -> Result<f32, SeismicStoreError> {
    if !sample_interval_ms.is_finite() || sample_interval_ms <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "sample interval must be finite and > 0 ms, found {sample_interval_ms}"
        )));
    }
    Ok(sample_interval_ms / 1000.0)
}

fn differentiate_trace(
    samples: &[f32],
    delta_seconds: f32,
    destination: &mut [f32],
) -> Result<(), SeismicStoreError> {
    if samples.len() != destination.len() {
        return Err(SeismicStoreError::Message(format!(
            "derivative workspace length mismatch: expected {}, found {}",
            samples.len(),
            destination.len()
        )));
    }
    if samples.is_empty() {
        return Ok(());
    }
    if samples.len() == 1 {
        destination[0] = 0.0;
        return Ok(());
    }

    destination[0] = (samples[1] - samples[0]) / delta_seconds;
    for index in 1..samples.len() - 1 {
        destination[index] = (samples[index + 1] - samples[index - 1]) / (2.0 * delta_seconds);
    }
    destination[samples.len() - 1] =
        (samples[samples.len() - 1] - samples[samples.len() - 2]) / delta_seconds;
    Ok(())
}

fn wrap_phase_degrees(angle_degrees: f32) -> f32 {
    (angle_degrees + 180.0).rem_euclid(360.0) - 180.0
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
        let threads = configured_compute_threads();
        ThreadPoolBuilder::new()
            .thread_name(|index| format!("ophiolite-seismic-compute-{index}"))
            .num_threads(threads)
            .build()
            .expect("compute pool should build")
    })
}

fn configured_compute_threads() -> usize {
    std::env::var("OPHIOLITE_BENCHMARK_WORKERS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|value| value.get())
                .unwrap_or(1)
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
        schema_version: 2,
        revision: 1,
        preset_id: None,
        name: None,
        description: None,
        steps: operations
            .iter()
            .cloned()
            .map(|operation| ophiolite_seismic::TraceLocalProcessingStep {
                operation,
                checkpoint: false,
            })
            .collect(),
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
        store_id: generate_store_id(),
        source: input.source.clone(),
        shape: input.shape,
        axes: input.axes.clone(),
        segy_export: None,
        coordinate_reference_binding: input.coordinate_reference_binding.clone(),
        spatial: input.spatial.clone(),
        created_by,
        processing_lineage: Some(ProcessingLineage {
            schema_version: 1,
            parent_store: parent_store.to_path_buf(),
            parent_store_id: input.store_id.clone(),
            artifact_role: ProcessingArtifactRole::FinalOutput,
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: pipeline.clone(),
            },
            pipeline_identity: None,
            operator_set_identity: None,
            planner_profile_identity: None,
            source_identity: None,
            runtime_semantics_version: CURRENT_RUNTIME_SEMANTICS_VERSION.to_string(),
            store_writer_semantics_version: CURRENT_STORE_WRITER_SEMANTICS_VERSION.to_string(),
            runtime_version: RUNTIME_VERSION.to_string(),
            created_at_unix_s: unix_timestamp_s(),
            artifact_key: None,
            input_artifact_keys: Vec::new(),
            produced_by_stage_id: None,
            boundary_reason: None,
            logical_domain: None,
            chunk_grid_spec: None,
            geometry_fingerprints: None,
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
    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
    };
    use crate::{
        PlanProcessingRequest, PlanningMode, ProcessingPipelineSpec, build_execution_plan,
    };
    use crate::{ProcessingOperatorScope, ProcessingSampleDependency, ProcessingSpatialDependency};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sinusoid_trace(
        samples: usize,
        sample_interval_ms: f32,
        cycles_per_trace: usize,
        amplitude: f32,
        phase_radians: f32,
    ) -> (Vec<f32>, f32) {
        let dt_s = sample_interval_ms / 1000.0;
        let frequency_hz = cycles_per_trace as f32 / (samples as f32 * dt_s);
        let trace = (0..samples)
            .map(|index| {
                let t = index as f32 * dt_s;
                amplitude * (2.0 * std::f32::consts::PI * frequency_hz * t + phase_radians).sin()
            })
            .collect::<Vec<_>>();
        (trace, frequency_hz)
    }

    fn mean_absolute_error(values: &[f32], expected: &[f32], trim: usize) -> f32 {
        let start = trim.min(values.len());
        let end = values.len().saturating_sub(trim).max(start);
        values[start..end]
            .iter()
            .zip(expected[start..end].iter())
            .map(|(actual, target)| (actual - target).abs())
            .sum::<f32>()
            / (end - start).max(1) as f32
    }

    fn discrete_central_difference_frequency_hz(frequency_hz: f32, sample_interval_ms: f32) -> f32 {
        let omega_dt = 2.0 * std::f32::consts::PI * frequency_hz * (sample_interval_ms / 1000.0);
        if omega_dt.abs() <= f32::EPSILON {
            frequency_hz
        } else {
            frequency_hz * (omega_dt.sin() / omega_dt)
        }
    }

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
    fn dependency_profiles_distinguish_pointwise_windowed_and_whole_trace_ops() {
        let scalar = ProcessingOperation::AmplitudeScalar { factor: 1.0 }.dependency_profile();
        assert_eq!(
            scalar.sample_dependency,
            ProcessingSampleDependency::Pointwise
        );
        assert_eq!(
            scalar.spatial_dependency,
            ProcessingSpatialDependency::SingleTrace
        );
        assert!(scalar.same_section_ephemeral_reuse_safe);

        let agc = ProcessingOperation::AgcRms { window_ms: 200.0 }.dependency_profile();
        assert_eq!(
            agc.sample_dependency,
            ProcessingSampleDependency::BoundedWindow {
                window_ms_hint: 200.0
            }
        );
        assert_eq!(
            agc.spatial_dependency,
            ProcessingSpatialDependency::SingleTrace
        );

        let bandpass = ProcessingOperation::BandpassFilter {
            f1_hz: 5.0,
            f2_hz: 10.0,
            f3_hz: 30.0,
            f4_hz: 40.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        }
        .dependency_profile();
        assert_eq!(
            bandpass.sample_dependency,
            ProcessingSampleDependency::WholeTrace
        );
        assert_eq!(
            bandpass.spatial_dependency,
            ProcessingSpatialDependency::SingleTrace
        );
    }

    #[test]
    fn volume_arithmetic_dependency_profile_marks_external_input() {
        let profile = ProcessingOperation::VolumeArithmetic {
            operator: TraceLocalVolumeArithmeticOperator::Add,
            secondary_store_path: "secondary.tbvol".to_string(),
        }
        .dependency_profile();
        assert_eq!(
            profile.sample_dependency,
            ProcessingSampleDependency::Pointwise
        );
        assert_eq!(
            profile.spatial_dependency,
            ProcessingSpatialDependency::ExternalVolumePointwise
        );
        assert!(profile.same_section_ephemeral_reuse_safe);
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
    fn envelope_matches_analytic_magnitude_definition() {
        let samples = 256usize;
        let sample_interval_ms = 2.0_f32;
        let amplitude = 3.5_f32;
        let (mut trace, _) = sinusoid_trace(
            samples,
            sample_interval_ms,
            16,
            amplitude,
            0.35 * std::f32::consts::PI,
        );

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::Envelope],
        )
        .unwrap();

        let expected = vec![amplitude; samples];
        assert!(mean_absolute_error(&trace, &expected, 4) < 2.0e-3);
    }

    #[test]
    fn instantaneous_phase_reconstructs_original_trace() {
        let samples = 256usize;
        let sample_interval_ms = 2.0_f32;
        let phase_offset = 0.2 * std::f32::consts::PI;
        let (original, _) = sinusoid_trace(samples, sample_interval_ms, 18, 2.0, phase_offset);
        let mut envelope = original.clone();
        let mut phase = original.clone();

        apply_pipeline_to_traces(
            &mut envelope,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::Envelope],
        )
        .unwrap();
        apply_pipeline_to_traces(
            &mut phase,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::InstantaneousPhase],
        )
        .unwrap();

        let reconstructed = envelope
            .iter()
            .zip(phase.iter())
            .map(|(magnitude, phase_degrees)| magnitude * phase_degrees.to_radians().cos())
            .collect::<Vec<_>>();
        assert!(mean_absolute_error(&reconstructed, &original, 4) < 2.5e-3);
    }

    #[test]
    fn instantaneous_frequency_tracks_periodic_sine_frequency() {
        let samples = 256usize;
        let sample_interval_ms = 2.0_f32;
        let (mut trace, frequency_hz) = sinusoid_trace(samples, sample_interval_ms, 20, 1.5, 0.0);

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::InstantaneousFrequency],
        )
        .unwrap();

        let expected =
            vec![
                discrete_central_difference_frequency_hz(frequency_hz, sample_interval_ms);
                samples
            ];
        let mae = mean_absolute_error(&trace, &expected, 4);
        assert!(mae < 0.2);
    }

    #[test]
    fn sweetness_uses_envelope_and_stabilized_frequency() {
        let samples = 256usize;
        let sample_interval_ms = 2.0_f32;
        let amplitude = 4.0_f32;
        let (mut trace, frequency_hz) =
            sinusoid_trace(samples, sample_interval_ms, 12, amplitude, 0.1);

        apply_pipeline_to_traces(
            &mut trace,
            1,
            samples,
            sample_interval_ms,
            None,
            &[ProcessingOperation::Sweetness],
        )
        .unwrap();

        let stabilized_frequency =
            discrete_central_difference_frequency_hz(frequency_hz, sample_interval_ms)
                .max(SWEETNESS_FREQUENCY_FLOOR_HZ);
        let expected = vec![amplitude / stabilized_frequency.sqrt(); samples];
        let mae = mean_absolute_error(&trace, &expected, 4);
        assert!(mae < 2.5e-3);
    }

    #[test]
    fn target_tile_group_size_respects_target_bytes() {
        let tiles = target_tile_group_size(4096, 128, 16_000);
        assert_eq!(tiles, 3);
        assert_eq!(target_tile_group_size(4096, 128, 1), 1);
    }

    #[test]
    fn partition_tile_groups_follow_storage_order() {
        let geometry = crate::storage::tile_geometry::TileGeometry::new([8, 6, 4], [2, 3, 4]);
        let groups = partition_tile_groups_for_target_bytes(&geometry, 220);
        assert_eq!(groups.len(), 4);
        assert!(groups.iter().all(|group| group.len() == 2));
        assert_eq!(
            groups[0],
            vec![
                TileCoord {
                    tile_i: 0,
                    tile_x: 0,
                },
                TileCoord {
                    tile_i: 0,
                    tile_x: 1,
                },
            ]
        );
        assert_eq!(
            groups[3],
            vec![
                TileCoord {
                    tile_i: 3,
                    tile_x: 0,
                },
                TileCoord {
                    tile_i: 3,
                    tile_x: 1,
                },
            ]
        );
    }

    #[test]
    fn partitioned_tbvol_materialization_matches_serial_output_with_secondary_inputs() {
        let source_root = unique_test_root("partitioned-trace-local-source");
        let secondary_root = unique_test_root("partitioned-trace-local-secondary");
        let serial_root = unique_test_root("partitioned-trace-local-serial");
        let partitioned_root = unique_test_root("partitioned-trace-local-parallel");
        let tile_shape = [2, 2, 8];
        let shape = [4, 4, 8];

        write_test_store(
            &source_root,
            shape,
            tile_shape,
            |iline, xline, sample| (iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32,
            |iline, xline| !(iline == 1 && xline == 2),
        );
        write_test_store(
            &secondary_root,
            shape,
            tile_shape,
            |iline, xline, sample| {
                ((iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32) * 0.25
            },
            |_, _| true,
        );

        let pipeline = ProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: Some("volume arithmetic add".to_string()),
            description: None,
            steps: vec![ophiolite_seismic::TraceLocalProcessingStep {
                operation: ProcessingOperation::VolumeArithmetic {
                    operator: TraceLocalVolumeArithmeticOperator::Add,
                    secondary_store_path: secondary_root.display().to_string(),
                },
                checkpoint: false,
            }],
        };

        let mut serial_progress = Vec::new();
        materialize_processing_volume_with_progress(
            &source_root,
            &serial_root,
            &pipeline,
            MaterializeOptions {
                chunk_shape: tile_shape,
                partition_target_bytes: None,
                ..MaterializeOptions::default()
            },
            |completed, total| {
                serial_progress.push((completed, total));
                Ok(())
            },
        )
        .expect("serial materialization should succeed");

        let mut partitioned_progress = Vec::new();
        let mut partition_execution_progress = Vec::new();
        materialize_processing_volume_with_partition_progress(
            &source_root,
            &partitioned_root,
            &pipeline,
            MaterializeOptions {
                chunk_shape: tile_shape,
                partition_target_bytes: Some(264),
                ..MaterializeOptions::default()
            },
            |completed, total| {
                partitioned_progress.push((completed, total));
                Ok(())
            },
            |progress| {
                partition_execution_progress.push(progress);
                Ok(())
            },
        )
        .expect("partitioned materialization should succeed");

        assert_eq!(serial_progress.last().copied(), Some((4, 4)));
        assert_eq!(partitioned_progress.last().copied(), Some((4, 4)));
        assert_eq!(
            partition_execution_progress.last().copied(),
            Some(PartitionExecutionProgress {
                completed_partitions: 2,
                total_partitions: 2,
                active_partitions: 0,
                peak_active_partitions: 2,
                retry_count: 0,
            })
        );

        let serial = TbvolReader::open(&serial_root).expect("open serial output");
        let partitioned = TbvolReader::open(&partitioned_root).expect("open partitioned output");

        for tile in serial.tile_geometry().iter_tiles() {
            assert_eq!(
                serial.read_tile(tile).expect("read serial tile").as_slice(),
                partitioned
                    .read_tile(tile)
                    .expect("read partitioned tile")
                    .as_slice()
            );
            assert_eq!(
                serial
                    .read_tile_occupancy(tile)
                    .expect("read serial occupancy")
                    .map(|mask| mask.into_owned()),
                partitioned
                    .read_tile_occupancy(tile)
                    .expect("read partitioned occupancy")
                    .map(|mask| mask.into_owned())
            );
        }

        let tile = partitioned
            .read_tile(TileCoord {
                tile_i: 0,
                tile_x: 0,
            })
            .expect("read partitioned verification tile");
        let tile = tile.as_slice();
        assert!((tile[0] - 0.0).abs() < 1.0e-6);
        assert!((tile[1] - 1.25).abs() < 1.0e-6);

        for root in [
            &source_root,
            &secondary_root,
            &serial_root,
            &partitioned_root,
        ] {
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn partitioned_tbvol_materialization_respects_max_active_partition_cap() {
        let source_root = unique_test_root("partition-cap-source");
        let output_root = unique_test_root("partition-cap-output");
        let tile_shape = [2, 2, 8];
        let shape = [4, 4, 8];

        write_test_store(
            &source_root,
            shape,
            tile_shape,
            |iline, xline, sample| (iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32,
            |_, _| true,
        );

        let pipeline = ProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: Some("scalar".to_string()),
            description: None,
            steps: vec![ophiolite_seismic::TraceLocalProcessingStep {
                operation: ProcessingOperation::AmplitudeScalar { factor: 2.0 },
                checkpoint: false,
            }],
        };

        let mut partition_execution_progress = Vec::new();
        materialize_processing_volume_with_partition_progress(
            &source_root,
            &output_root,
            &pipeline,
            MaterializeOptions {
                chunk_shape: tile_shape,
                partition_target_bytes: Some(264),
                max_active_partitions: Some(1),
                ..MaterializeOptions::default()
            },
            |_, _| Ok(()),
            |progress| {
                partition_execution_progress.push(progress);
                Ok(())
            },
        )
        .expect("partition-capped materialization should succeed");

        assert_eq!(
            partition_execution_progress.last().copied(),
            Some(PartitionExecutionProgress {
                completed_partitions: 2,
                total_partitions: 2,
                active_partitions: 0,
                peak_active_partitions: 1,
                retry_count: 0,
            })
        );

        for root in [&source_root, &output_root] {
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn partitioned_tbvol_materialization_uses_explicit_chunk_plan() {
        let source_root = unique_test_root("chunk-plan-source");
        let output_root = unique_test_root("chunk-plan-output");
        let tile_shape = [2, 2, 8];
        let shape = [4, 4, 8];

        write_test_store(
            &source_root,
            shape,
            tile_shape,
            |iline, xline, sample| (iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32,
            |_, _| true,
        );

        let pipeline = ProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: Some("scalar".to_string()),
            description: None,
            steps: vec![ophiolite_seismic::TraceLocalProcessingStep {
                operation: ProcessingOperation::AmplitudeScalar { factor: 2.0 },
                checkpoint: false,
            }],
        };

        let mut partition_execution_progress = Vec::new();
        materialize_processing_volume_with_partition_progress(
            &source_root,
            &output_root,
            &pipeline,
            MaterializeOptions {
                chunk_shape: tile_shape,
                partition_target_bytes: None,
                trace_local_chunk_plan: Some(TraceLocalChunkPlanRecommendation {
                    max_active_partitions: 1,
                    tiles_per_partition: 2,
                    partition_count: 2,
                    compatibility_target_bytes: 1,
                    resident_partition_bytes: 1,
                    global_worker_workspace_bytes: 0,
                    estimated_peak_bytes: 1,
                }),
                ..MaterializeOptions::default()
            },
            |_, _| Ok(()),
            |progress| {
                partition_execution_progress.push(progress);
                Ok(())
            },
        )
        .expect("chunk-plan materialization should succeed");

        assert_eq!(
            partition_execution_progress.last().copied(),
            Some(PartitionExecutionProgress {
                completed_partitions: 2,
                total_partitions: 2,
                active_partitions: 0,
                peak_active_partitions: 1,
                retry_count: 0,
            })
        );

        for root in [&source_root, &output_root] {
            let _ = fs::remove_dir_all(root);
        }
    }

    fn test_trace_local_execution_plan(
        shape: [usize; 3],
        chunk_shape: [usize; 3],
        planning_mode: PlanningMode,
    ) -> crate::execution::ExecutionPlan {
        build_execution_plan(&PlanProcessingRequest {
            store_path: "input.tbvol".to_string(),
            layout: SeismicLayout::PostStack3D,
            source_shape: Some(shape),
            source_chunk_shape: Some(chunk_shape),
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: ProcessingPipeline {
                    schema_version: 1,
                    revision: 1,
                    preset_id: None,
                    name: Some("adaptive-test".to_string()),
                    description: None,
                    steps: vec![
                        ophiolite_seismic::TraceLocalProcessingStep {
                            operation: ProcessingOperation::TraceRmsNormalize,
                            checkpoint: false,
                        },
                        ophiolite_seismic::TraceLocalProcessingStep {
                            operation: ProcessingOperation::AgcRms { window_ms: 250.0 },
                            checkpoint: false,
                        },
                    ],
                },
            },
            output_store_path: Some("output.tbvol".to_string()),
            planning_mode,
            max_active_partitions: None,
        })
        .expect("trace-local execution plan should build")
    }

    #[test]
    fn resolve_trace_local_materialize_options_uses_adaptive_chunk_plan() {
        let plan = test_trace_local_execution_plan(
            [64, 64, 256],
            [8, 8, 256],
            PlanningMode::ForegroundMaterialize,
        );

        let resolution = resolve_trace_local_materialize_options(
            Some(&plan),
            [8, 8, 256],
            true,
            Some(64 * 1024 * 1024),
            8,
            Some(8 * 1024 * 1024 * 1024),
            1,
        );

        let recommendation = resolution
            .chunk_plan_resolution
            .as_ref()
            .expect("adaptive recommendation should exist");
        let chunk_plan = resolution
            .options
            .trace_local_chunk_plan
            .as_ref()
            .expect("explicit chunk plan should be populated");
        let summary = resolution
            .resolved_chunk_plan
            .as_ref()
            .expect("resolved chunk-plan summary should be populated");

        assert_eq!(resolution.options.partition_target_bytes, None);
        assert_eq!(
            resolution.resolved_partition_target_bytes,
            Some(recommendation.target_bytes())
        );
        assert_eq!(
            chunk_plan.partition_count,
            recommendation.recommended_partition_count()
        );
        assert_eq!(
            summary.compatibility_target_bytes,
            recommendation.target_bytes()
        );
        assert_eq!(summary.partition_count, chunk_plan.partition_count);
    }

    #[test]
    fn resolve_trace_local_materialize_options_falls_back_when_adaptive_disabled() {
        let plan = test_trace_local_execution_plan(
            [64, 64, 256],
            [8, 8, 256],
            PlanningMode::ForegroundMaterialize,
        );
        let target_bytes = 96 * 1024 * 1024;

        let resolution = resolve_trace_local_materialize_options(
            Some(&plan),
            [8, 8, 256],
            false,
            Some(target_bytes),
            8,
            Some(8 * 1024 * 1024 * 1024),
            1,
        );

        assert!(resolution.chunk_plan_resolution.is_none());
        assert_eq!(
            resolution.options.partition_target_bytes,
            Some(target_bytes)
        );
        assert!(resolution.options.trace_local_chunk_plan.is_none());
        assert_eq!(
            resolution.resolved_partition_target_bytes,
            Some(target_bytes)
        );
        let summary = resolution
            .resolved_chunk_plan
            .as_ref()
            .expect("fixed partition target should still resolve a chunk summary");
        assert_eq!(summary.compatibility_target_bytes, target_bytes);
        assert!(summary.partition_count >= 1);
        assert!(summary.tiles_per_partition >= 1);
    }

    #[test]
    fn resolve_trace_local_materialize_options_handles_missing_plan() {
        let target_bytes = 32 * 1024 * 1024;

        let resolution = resolve_trace_local_materialize_options(
            None,
            [8, 8, 256],
            true,
            Some(target_bytes),
            8,
            Some(8 * 1024 * 1024 * 1024),
            1,
        );

        assert!(resolution.chunk_plan_resolution.is_none());
        assert_eq!(
            resolution.options.partition_target_bytes,
            Some(target_bytes)
        );
        assert!(resolution.options.trace_local_chunk_plan.is_none());
        assert_eq!(
            resolution.resolved_partition_target_bytes,
            Some(target_bytes)
        );
        assert!(resolution.resolved_chunk_plan.is_none());
    }

    #[test]
    fn resolve_trace_local_materialize_options_scales_for_batch_concurrency() {
        let plan = test_trace_local_execution_plan(
            [64, 64, 256],
            [8, 8, 256],
            PlanningMode::BackgroundBatch,
        );

        let single_job = resolve_trace_local_materialize_options(
            Some(&plan),
            [8, 8, 256],
            true,
            None,
            8,
            Some(8 * 1024 * 1024 * 1024),
            1,
        );
        let four_jobs = resolve_trace_local_materialize_options(
            Some(&plan),
            [8, 8, 256],
            true,
            None,
            8,
            Some(8 * 1024 * 1024 * 1024),
            4,
        );

        let single = single_job
            .chunk_plan_resolution
            .expect("single-job adaptive recommendation should exist");
        let batch = four_jobs
            .chunk_plan_resolution
            .expect("batch adaptive recommendation should exist");

        assert!(batch.target_bytes() <= single.target_bytes());
        assert!(batch.recommended_partition_count() >= single.recommended_partition_count());
    }

    fn write_test_store<F, O>(
        root: &Path,
        shape: [usize; 3],
        tile_shape: [usize; 3],
        amplitude: F,
        occupied: O,
    ) where
        F: Fn(usize, usize, usize) -> f32,
        O: Fn(usize, usize) -> bool,
    {
        let volume = test_volume_metadata(shape);
        let geometry = crate::storage::tile_geometry::TileGeometry::new(shape, tile_shape);
        let writer =
            TbvolWriter::create(root, volume, tile_shape, true).expect("create synthetic tbvol");

        for tile in geometry.iter_tiles() {
            let origin = geometry.tile_origin(tile);
            let effective = geometry.effective_tile_shape(tile);
            let mut amplitudes = vec![0.0_f32; geometry.amplitude_tile_len()];
            let mut occupancy = vec![0_u8; geometry.occupancy_tile_len()];
            for local_i in 0..effective[0] {
                for local_x in 0..effective[1] {
                    let global_i = origin[0] + local_i;
                    let global_x = origin[1] + local_x;
                    let trace_index = local_i * tile_shape[1] + local_x;
                    occupancy[trace_index] = u8::from(occupied(global_i, global_x));
                    let trace_start = trace_index * tile_shape[2];
                    for sample in 0..effective[2] {
                        amplitudes[trace_start + sample] = amplitude(global_i, global_x, sample);
                    }
                }
            }
            writer
                .write_tile(tile, &amplitudes)
                .expect("write amplitudes");
            writer
                .write_tile_occupancy(tile, &occupancy)
                .expect("write occupancy");
        }
        writer.finalize().expect("finalize synthetic tbvol");
    }

    fn test_volume_metadata(shape: [usize; 3]) -> VolumeMetadata {
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://compute-partition-test"),
                file_size: 0,
                trace_count: (shape[0] * shape[1]) as u64,
                samples_per_trace: shape[2],
                sample_interval_us: 2000,
                sample_format_code: 5,
                sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(5),
                endianness: "little".to_string(),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
                geometry: GeometryProvenance {
                    inline_field: HeaderFieldSpec {
                        name: "INLINE".to_string(),
                        start_byte: 189,
                        value_type: "I32".to_string(),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: "XLINE".to_string(),
                        start_byte: 193,
                        value_type: "I32".to_string(),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape,
            axes: VolumeAxes::from_time_axis(
                (0..shape[0]).map(|value| value as f64).collect(),
                (0..shape[1]).map(|value| value as f64).collect(),
                (0..shape[2]).map(|value| value as f32 * 2.0).collect(),
            ),
            segy_export: None,
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "compute-partition-test".to_string(),
            processing_lineage: None,
        }
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("traceboost-{label}-{suffix}.tbvol"))
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
