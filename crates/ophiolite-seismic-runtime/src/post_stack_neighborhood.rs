use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ophiolite_seismic::contracts::{LocalVolumeStatistic, NeighborhoodDipOutput};

use crate::compute::{
    MaterializeOptions, PreviewSectionPrefixReuse, PreviewSectionSession,
    materialize_processing_volume, preview_processing_section_plane,
};
use crate::error::SeismicStoreError;
use crate::metadata::{DatasetKind, ProcessingLineage, VolumeMetadata, generate_store_id};
use crate::segy_export::copy_store_segy_export;
use crate::storage::section_assembler;
use crate::storage::tbvol::{TbvolReader, TbvolWriter};
use crate::storage::tile_geometry::TileCoord;
use crate::storage::volume_store::{VolumeStoreReader, VolumeStoreWriter};
use crate::store::{SectionPlane, StoreHandle, open_store};
use crate::{
    PostStackNeighborhoodProcessingOperation, PostStackNeighborhoodProcessingPipeline,
    PostStackNeighborhoodWindow, ProcessingPipelineSpec, SectionAxis, SectionView, SeismicLayout,
};
use ophiolite_seismic::ProcessingArtifactRole;

const MAX_POST_STACK_NEIGHBORHOOD_GATE_MS: f32 = 10_000.0;
const SIMILARITY_EPSILON: f32 = 1.0e-8;
const DIP_FIT_EPSILON: f64 = 1.0e-12;
const RUNTIME_VERSION: &str = "ophiolite-seismic-runtime-0.1.0";

struct NeighborhoodTraceMatrix {
    inline_count: usize,
    xline_count: usize,
    samples: usize,
    center_inline_offset: usize,
    center_xline_offset: usize,
    amplitudes: Vec<f32>,
    occupancy: Option<Vec<u8>>,
}

struct LoadedSourceTile {
    amplitudes: Vec<f32>,
    occupancy: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct DipObservationAccumulator {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct DipEstimate {
    inline_ms_per_trace: f32,
    xline_ms_per_trace: f32,
}

#[derive(Debug, Clone, Copy)]
struct MatrixDipNeighbor {
    trace_offset: usize,
    delta_inline: f64,
    delta_xline: f64,
}

#[derive(Debug, Clone, Copy)]
struct SectionDipNeighbor {
    section_index: usize,
    trace_offset: usize,
    delta_inline: f64,
    delta_xline: f64,
}

impl DipObservationAccumulator {
    fn add_observation(&mut self, delta_inline: f64, delta_xline: f64, lag_ms: f64, weight: f64) {
        if !weight.is_finite() || weight <= 0.0 {
            return;
        }
        self.a += weight * delta_inline * delta_inline;
        self.b += weight * delta_inline * delta_xline;
        self.c += weight * delta_xline * delta_xline;
        self.d += weight * delta_inline * lag_ms;
        self.e += weight * delta_xline * lag_ms;
    }

    fn solve(&self) -> DipEstimate {
        let determinant = self.a * self.c - self.b * self.b;
        if determinant.abs() > DIP_FIT_EPSILON {
            let inline = (self.d * self.c - self.b * self.e) / determinant;
            let xline = (self.a * self.e - self.b * self.d) / determinant;
            return DipEstimate {
                inline_ms_per_trace: inline as f32,
                xline_ms_per_trace: xline as f32,
            };
        }
        if self.a.abs() > DIP_FIT_EPSILON {
            return DipEstimate {
                inline_ms_per_trace: (self.d / self.a) as f32,
                xline_ms_per_trace: 0.0,
            };
        }
        if self.c.abs() > DIP_FIT_EPSILON {
            return DipEstimate {
                inline_ms_per_trace: 0.0,
                xline_ms_per_trace: (self.e / self.c) as f32,
            };
        }
        DipEstimate::default()
    }
}

pub fn validate_post_stack_neighborhood_processing_pipeline(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> Result<(), SeismicStoreError> {
    validate_post_stack_neighborhood_processing_pipeline_for_layout(
        pipeline,
        SeismicLayout::PostStack3D,
    )
}

pub fn validate_post_stack_neighborhood_processing_pipeline_for_layout(
    pipeline: &PostStackNeighborhoodProcessingPipeline,
    layout: SeismicLayout,
) -> Result<(), SeismicStoreError> {
    if !matches!(
        layout,
        SeismicLayout::PostStack3D | SeismicLayout::PostStack2D
    ) {
        return Err(SeismicStoreError::Message(format!(
            "post-stack neighborhood processing requires post-stack layout, found {:?}",
            layout
        )));
    }
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        crate::compute::validate_processing_pipeline_for_layout(trace_local_pipeline, layout)?;
    }
    if pipeline.operations.len() != 1 {
        return Err(SeismicStoreError::Message(format!(
            "post-stack neighborhood processing currently requires exactly one operator, found {}",
            pipeline.operations.len()
        )));
    }
    for operation in &pipeline.operations {
        let compatibility = operation.compatibility();
        if !compatibility.supports_layout(layout) {
            return Err(SeismicStoreError::Message(format!(
                "post-stack neighborhood operator '{}' requires {}, found layout {:?}",
                operation.operator_id(),
                compatibility.label(),
                layout
            )));
        }
        match operation {
            PostStackNeighborhoodProcessingOperation::Similarity { window } => {
                validate_window(window)?;
            }
            PostStackNeighborhoodProcessingOperation::LocalVolumeStats { window, .. } => {
                validate_window(window)?;
            }
            PostStackNeighborhoodProcessingOperation::Dip { window, .. } => {
                validate_window(window)?;
            }
        }
    }
    Ok(())
}

pub fn preview_post_stack_neighborhood_processing_section_view(
    store_root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> Result<SectionView, SeismicStoreError> {
    validate_post_stack_neighborhood_processing_pipeline(pipeline)?;
    let handle = open_store(&store_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let (plane, _) = preview_post_stack_neighborhood_processing_section_plane_with_loader(
        axis,
        index,
        pipeline,
        match axis {
            SectionAxis::Inline => reader.volume().shape[0],
            SectionAxis::Xline => reader.volume().shape[1],
        },
        |section_index, trace_local_pipeline| {
            Ok((
                load_preview_section_plane(
                    store_root.as_ref(),
                    &reader,
                    axis,
                    section_index,
                    trace_local_pipeline,
                )?,
                PreviewSectionPrefixReuse::default(),
            ))
        },
    )?;

    Ok(handle.section_view_from_plane(&plane))
}

pub fn preview_post_stack_neighborhood_processing_section_view_with_prefix_cache(
    session: &mut PreviewSectionSession,
    axis: SectionAxis,
    index: usize,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> Result<(SectionView, PreviewSectionPrefixReuse), SeismicStoreError> {
    validate_post_stack_neighborhood_processing_pipeline(pipeline)?;
    let section_count = session.section_count(axis);
    let (plane, reuse) = preview_post_stack_neighborhood_processing_section_plane_with_loader(
        axis,
        index,
        pipeline,
        section_count,
        |section_index, trace_local_pipeline| match trace_local_pipeline {
            Some(prefix) => session.preview_processing_section_plane_with_prefix_cache(
                axis,
                section_index,
                prefix,
            ),
            None => Ok((
                session.read_section_plane(axis, section_index)?,
                PreviewSectionPrefixReuse::default(),
            )),
        },
    )?;
    Ok((session.section_view_from_plane(&plane), reuse))
}

pub fn materialize_post_stack_neighborhood_processing_volume(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
    options: MaterializeOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    materialize_post_stack_neighborhood_processing_volume_with_progress(
        input_store_root,
        output_store_root,
        pipeline,
        options,
        |_, _| Ok(()),
    )
}

pub fn materialize_post_stack_neighborhood_processing_volume_with_progress<
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    input_store_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
    options: MaterializeOptions,
    mut on_progress: F,
) -> Result<StoreHandle, SeismicStoreError> {
    validate_post_stack_neighborhood_processing_pipeline(pipeline)?;

    let prepared_input = if let Some(prefix) = pipeline.trace_local_pipeline.as_ref() {
        let temp_root = unique_temp_store_path(output_store_root.as_ref(), "prefix");
        let result =
            materialize_processing_volume(&input_store_root, &temp_root, prefix, options.clone());
        match result {
            Ok(handle) => handle,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&temp_root);
                return Err(error);
            }
        }
    } else {
        open_store(&input_store_root)?
    };

    let prepared_root = prepared_input.root.clone();
    let result = materialize_post_stack_neighborhood_without_prefix_with_progress(
        &prepared_root,
        output_store_root,
        pipeline,
        options,
        &mut on_progress,
    );

    if pipeline.trace_local_pipeline.is_some() {
        let _ = std::fs::remove_dir_all(&prepared_root);
    }

    result
}

fn materialize_post_stack_neighborhood_without_prefix_with_progress<
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
>(
    prepared_input_root: impl AsRef<Path>,
    output_store_root: impl AsRef<Path>,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
    options: MaterializeOptions,
    on_progress: &mut F,
) -> Result<StoreHandle, SeismicStoreError> {
    let handle = open_store(&prepared_input_root)?;
    let reader = TbvolReader::open(&handle.root)?;
    let volume = derived_post_stack_neighborhood_volume_metadata(
        reader.volume(),
        prepared_input_root.as_ref(),
        pipeline,
        options.created_by.clone(),
    );
    let chunk_shape = resolve_chunk_shape(options.chunk_shape, volume.shape);
    let has_occupancy = reader_has_occupancy(&reader)?;
    let writer = TbvolWriter::create(&output_store_root, volume, chunk_shape, has_occupancy)?;
    let output_root = writer.root().to_path_buf();
    let operation = pipeline
        .operations
        .first()
        .ok_or_else(|| SeismicStoreError::Message("missing neighborhood operator".to_string()))?;
    let sample_interval_ms = reader.volume().source.sample_interval_us as f32 / 1000.0;
    let tile_shape = reader.tile_geometry().tile_shape();
    let total_tiles = reader.tile_geometry().tile_count();
    let mut completed_tiles = 0usize;

    for tile in reader.tile_geometry().iter_tiles() {
        let effective = reader.tile_geometry().effective_tile_shape(tile);
        let origin = reader.tile_geometry().tile_origin(tile);
        let occupancy = reader
            .read_tile_occupancy(tile)?
            .map(|value| value.into_owned());
        let amplitudes = match operation {
            PostStackNeighborhoodProcessingOperation::Similarity { window } => {
                let neighborhood = load_neighborhood_trace_matrix(
                    &reader,
                    origin[0],
                    origin[1],
                    [effective[0], effective[1]],
                    window,
                )?;
                similarity_tile_amplitudes(
                    &neighborhood,
                    window,
                    sample_interval_ms,
                    tile_shape,
                    [effective[0], effective[1]],
                )?
            }
            PostStackNeighborhoodProcessingOperation::LocalVolumeStats { window, statistic } => {
                let neighborhood = load_neighborhood_trace_matrix(
                    &reader,
                    origin[0],
                    origin[1],
                    [effective[0], effective[1]],
                    window,
                )?;
                local_volume_stats_tile_amplitudes(
                    &neighborhood,
                    window,
                    sample_interval_ms,
                    tile_shape,
                    [effective[0], effective[1]],
                    *statistic,
                )?
            }
            PostStackNeighborhoodProcessingOperation::Dip { window, output } => {
                let neighborhood = load_neighborhood_trace_matrix(
                    &reader,
                    origin[0],
                    origin[1],
                    [effective[0], effective[1]],
                    window,
                )?;
                dip_tile_amplitudes(
                    &neighborhood,
                    window,
                    sample_interval_ms,
                    tile_shape,
                    [effective[0], effective[1]],
                    *output,
                )?
            }
        };
        writer.write_tile(tile, &amplitudes)?;
        if let Some(mask) = occupancy.as_deref() {
            writer.write_tile_occupancy(tile, mask)?;
        }
        completed_tiles += 1;
        on_progress(completed_tiles, total_tiles)?;
    }

    writer.finalize()?;
    copy_store_segy_export(prepared_input_root.as_ref(), &output_root)?;
    open_store(output_root)
}

fn validate_window(window: &PostStackNeighborhoodWindow) -> Result<(), SeismicStoreError> {
    if !window.gate_ms.is_finite() || window.gate_ms <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "post-stack neighborhood gate_ms must be finite and > 0, found {}",
            window.gate_ms
        )));
    }
    if window.gate_ms > MAX_POST_STACK_NEIGHBORHOOD_GATE_MS {
        return Err(SeismicStoreError::Message(format!(
            "post-stack neighborhood gate_ms must be <= {MAX_POST_STACK_NEIGHBORHOOD_GATE_MS}, found {}",
            window.gate_ms
        )));
    }
    if window.inline_stepout == 0 && window.xline_stepout == 0 {
        return Err(SeismicStoreError::Message(
            "post-stack neighborhood stepout must be non-zero on at least one lateral axis"
                .to_string(),
        ));
    }
    Ok(())
}

fn load_preview_section_plane(
    store_root: &Path,
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    trace_local_pipeline: Option<&crate::ProcessingPipeline>,
) -> Result<SectionPlane, SeismicStoreError> {
    match trace_local_pipeline {
        Some(pipeline) => preview_processing_section_plane(store_root, axis, index, pipeline),
        None => section_assembler::read_section_plane(reader, axis, index),
    }
}

fn preview_post_stack_neighborhood_processing_section_plane_with_loader<F>(
    axis: SectionAxis,
    index: usize,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
    section_count: usize,
    mut load_section: F,
) -> Result<(SectionPlane, PreviewSectionPrefixReuse), SeismicStoreError>
where
    F: FnMut(
        usize,
        Option<&crate::ProcessingPipeline>,
    ) -> Result<(SectionPlane, PreviewSectionPrefixReuse), SeismicStoreError>,
{
    if index >= section_count {
        return Err(SeismicStoreError::Message(format!(
            "section index {index} is out of bounds for {:?} axis with {section_count} sections",
            axis
        )));
    }

    let operation = pipeline
        .operations
        .first()
        .ok_or_else(|| SeismicStoreError::Message("missing neighborhood operator".to_string()))?;
    let trace_local_pipeline = pipeline.trace_local_pipeline.as_ref();

    match operation {
        PostStackNeighborhoodProcessingOperation::Similarity { window } => {
            let (sections, center_section_offset, reuse) = load_preview_sections(
                axis,
                index,
                section_count,
                window,
                trace_local_pipeline,
                &mut load_section,
            )?;
            Ok((
                similarity_section_plane(axis, sections, center_section_offset, window)?,
                reuse,
            ))
        }
        PostStackNeighborhoodProcessingOperation::LocalVolumeStats { window, statistic } => {
            let (sections, center_section_offset, reuse) = load_preview_sections(
                axis,
                index,
                section_count,
                window,
                trace_local_pipeline,
                &mut load_section,
            )?;
            Ok((
                local_volume_stats_section_plane(
                    axis,
                    sections,
                    center_section_offset,
                    window,
                    *statistic,
                )?,
                reuse,
            ))
        }
        PostStackNeighborhoodProcessingOperation::Dip { window, output } => {
            let (sections, center_section_offset, reuse) = load_preview_sections(
                axis,
                index,
                section_count,
                window,
                trace_local_pipeline,
                &mut load_section,
            )?;
            Ok((
                dip_section_plane(axis, sections, center_section_offset, window, *output)?,
                reuse,
            ))
        }
    }
}

fn load_preview_sections<F>(
    axis: SectionAxis,
    center_index: usize,
    section_count: usize,
    window: &PostStackNeighborhoodWindow,
    trace_local_pipeline: Option<&crate::ProcessingPipeline>,
    load_section: &mut F,
) -> Result<(Vec<SectionPlane>, usize, PreviewSectionPrefixReuse), SeismicStoreError>
where
    F: FnMut(
        usize,
        Option<&crate::ProcessingPipeline>,
    ) -> Result<(SectionPlane, PreviewSectionPrefixReuse), SeismicStoreError>,
{
    let (section_radius, _) = section_and_trace_radii(axis, window);
    let section_start = center_index.saturating_sub(section_radius);
    let section_end = center_index
        .saturating_add(section_radius)
        .min(section_count.saturating_sub(1));
    let mut sections = Vec::with_capacity(section_end - section_start + 1);
    let mut reuse = PreviewSectionPrefixReuse::default();
    for section_index in section_start..=section_end {
        let (section, section_reuse) = load_section(section_index, trace_local_pipeline)?;
        reuse.cache_hit |= section_reuse.cache_hit;
        reuse.reused_prefix_operations = reuse
            .reused_prefix_operations
            .max(section_reuse.reused_prefix_operations);
        sections.push(section);
    }
    Ok((sections, center_index - section_start, reuse))
}

fn similarity_section_plane(
    axis: SectionAxis,
    sections: Vec<SectionPlane>,
    center_section_offset: usize,
    window: &PostStackNeighborhoodWindow,
) -> Result<SectionPlane, SeismicStoreError> {
    let center = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("similarity preview center section was not loaded".to_string())
    })?;
    let sample_interval_ms = sample_interval_ms_from_axis(&center.sample_axis_ms)?;
    let (_, trace_radius) = section_and_trace_radii(axis, window);
    let gate_half_samples = gate_half_samples(sample_interval_ms, window.gate_ms);
    let mut amplitudes = vec![0.0_f32; center.amplitudes.len()];

    for trace_index in 0..center.traces {
        if is_unoccupied(center.occupancy.as_deref(), trace_index) {
            continue;
        }
        let center_trace = plane_trace_slice(center, trace_index)?;
        for sample_index in 0..center.samples {
            amplitudes[trace_index * center.samples + sample_index] = similarity_at_section_sample(
                &sections,
                center_section_offset,
                trace_index,
                sample_index,
                trace_radius,
                gate_half_samples,
                center_trace,
            )?;
        }
    }

    Ok(SectionPlane {
        axis,
        coordinate_index: center.coordinate_index,
        coordinate_value: center.coordinate_value,
        traces: center.traces,
        samples: center.samples,
        horizontal_axis: center.horizontal_axis.clone(),
        sample_axis_ms: center.sample_axis_ms.clone(),
        amplitudes,
        occupancy: center.occupancy.clone(),
    })
}

fn similarity_at_section_sample(
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    sample_index: usize,
    trace_radius: usize,
    gate_half_samples: usize,
    center_trace: &[f32],
) -> Result<f32, SeismicStoreError> {
    let center_section = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("missing center section for similarity".to_string())
    })?;
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, center_section.samples, gate_half_samples);
    let center_window = &center_trace[sample_start..sample_end];
    let center_window_norm = signal_window_l2_norm(center_window);
    if center_window_norm <= SIMILARITY_EPSILON {
        return Ok(0.0);
    }
    let mut sum = 0.0_f32;
    let mut count = 0usize;

    for (section_offset, section) in sections.iter().enumerate() {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if section_offset == center_section_offset && neighbor_trace_index == center_trace_index
            {
                continue;
            }
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let neighbor_trace = plane_trace_slice(section, neighbor_trace_index)?;
            let neighbor_window = &neighbor_trace[sample_start..sample_end];
            sum += normalized_cross_correlation_abs_with_left_norm(
                center_window,
                center_window_norm,
                neighbor_window,
            );
            count += 1;
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        Ok((sum / count as f32).clamp(0.0, 1.0))
    }
}

fn local_volume_stats_section_plane(
    axis: SectionAxis,
    sections: Vec<SectionPlane>,
    center_section_offset: usize,
    window: &PostStackNeighborhoodWindow,
    statistic: LocalVolumeStatistic,
) -> Result<SectionPlane, SeismicStoreError> {
    match statistic {
        LocalVolumeStatistic::Mean => {
            local_volume_stats_section_plane_with(axis, sections, center_section_offset, window, {
                local_volume_stats_mean_at_section_sample
            })
        }
        LocalVolumeStatistic::Rms => {
            local_volume_stats_section_plane_with(axis, sections, center_section_offset, window, {
                local_volume_stats_rms_at_section_sample
            })
        }
        LocalVolumeStatistic::Variance => local_volume_stats_section_plane_with(
            axis,
            sections,
            center_section_offset,
            window,
            local_volume_stats_variance_at_section_sample,
        ),
        LocalVolumeStatistic::Minimum => local_volume_stats_section_plane_with(
            axis,
            sections,
            center_section_offset,
            window,
            local_volume_stats_minimum_at_section_sample,
        ),
        LocalVolumeStatistic::Maximum => local_volume_stats_section_plane_with(
            axis,
            sections,
            center_section_offset,
            window,
            local_volume_stats_maximum_at_section_sample,
        ),
    }
}

fn local_volume_stats_section_plane_with(
    axis: SectionAxis,
    sections: Vec<SectionPlane>,
    center_section_offset: usize,
    window: &PostStackNeighborhoodWindow,
    sample_fn: fn(
        &[SectionPlane],
        usize,
        usize,
        usize,
        usize,
        usize,
    ) -> Result<f32, SeismicStoreError>,
) -> Result<SectionPlane, SeismicStoreError> {
    let center = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message(
            "local volume stats preview center section was not loaded".to_string(),
        )
    })?;
    let sample_interval_ms = sample_interval_ms_from_axis(&center.sample_axis_ms)?;
    let (_, trace_radius) = section_and_trace_radii(axis, window);
    let gate_half_samples = gate_half_samples(sample_interval_ms, window.gate_ms);
    let mut amplitudes = vec![0.0_f32; center.amplitudes.len()];

    for trace_index in 0..center.traces {
        if is_unoccupied(center.occupancy.as_deref(), trace_index) {
            continue;
        }
        for sample_index in 0..center.samples {
            amplitudes[trace_index * center.samples + sample_index] = sample_fn(
                &sections,
                center_section_offset,
                trace_index,
                sample_index,
                trace_radius,
                gate_half_samples,
            )?;
        }
    }

    Ok(SectionPlane {
        axis,
        coordinate_index: center.coordinate_index,
        coordinate_value: center.coordinate_value,
        traces: center.traces,
        samples: center.samples,
        horizontal_axis: center.horizontal_axis.clone(),
        sample_axis_ms: center.sample_axis_ms.clone(),
        amplitudes,
        occupancy: center.occupancy.clone(),
    })
}

fn local_volume_stats_mean_at_section_sample(
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    sample_index: usize,
    trace_radius: usize,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let center_section = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("missing center section for local volume stats".to_string())
    })?;
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, center_section.samples, gate_half_samples);
    let mut count = 0usize;
    let mut sum = 0.0_f64;

    for section in sections {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let neighbor_trace = plane_trace_slice(section, neighbor_trace_index)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                count += 1;
                sum += f64::from(sample);
            }
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        Ok((sum / count as f64) as f32)
    }
}

fn local_volume_stats_rms_at_section_sample(
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    sample_index: usize,
    trace_radius: usize,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let center_section = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("missing center section for local volume stats".to_string())
    })?;
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, center_section.samples, gate_half_samples);
    let mut count = 0usize;
    let mut sum_squares = 0.0_f64;

    for section in sections {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let neighbor_trace = plane_trace_slice(section, neighbor_trace_index)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                count += 1;
                let sample = f64::from(sample);
                sum_squares += sample * sample;
            }
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        Ok((sum_squares / count as f64).sqrt() as f32)
    }
}

fn local_volume_stats_variance_at_section_sample(
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    sample_index: usize,
    trace_radius: usize,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let center_section = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("missing center section for local volume stats".to_string())
    })?;
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, center_section.samples, gate_half_samples);
    let mut count = 0usize;
    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;

    for section in sections {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let neighbor_trace = plane_trace_slice(section, neighbor_trace_index)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                let sample = f64::from(sample);
                count += 1;
                sum += sample;
                sum_squares += sample * sample;
            }
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        let mean = sum / count as f64;
        Ok(((sum_squares / count as f64) - (mean * mean)).max(0.0) as f32)
    }
}

fn local_volume_stats_minimum_at_section_sample(
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    sample_index: usize,
    trace_radius: usize,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let center_section = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("missing center section for local volume stats".to_string())
    })?;
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, center_section.samples, gate_half_samples);
    let mut minimum = None::<f32>;

    for section in sections {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let neighbor_trace = plane_trace_slice(section, neighbor_trace_index)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                minimum = Some(match minimum {
                    Some(current) => current.min(sample),
                    None => sample,
                });
            }
        }
    }

    Ok(minimum.unwrap_or(0.0))
}

fn local_volume_stats_maximum_at_section_sample(
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    sample_index: usize,
    trace_radius: usize,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let center_section = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("missing center section for local volume stats".to_string())
    })?;
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, center_section.samples, gate_half_samples);
    let mut maximum = None::<f32>;

    for section in sections {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let neighbor_trace = plane_trace_slice(section, neighbor_trace_index)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                maximum = Some(match maximum {
                    Some(current) => current.max(sample),
                    None => sample,
                });
            }
        }
    }

    Ok(maximum.unwrap_or(0.0))
}

fn load_neighborhood_trace_matrix(
    reader: &TbvolReader,
    output_inline_start: usize,
    output_xline_start: usize,
    effective_trace_shape: [usize; 2],
    window: &PostStackNeighborhoodWindow,
) -> Result<NeighborhoodTraceMatrix, SeismicStoreError> {
    let inline_start = output_inline_start.saturating_sub(window.inline_stepout);
    let xline_start = output_xline_start.saturating_sub(window.xline_stepout);
    let inline_end = output_inline_start
        .saturating_add(effective_trace_shape[0])
        .saturating_add(window.inline_stepout)
        .min(reader.volume().shape[0]);
    let xline_end = output_xline_start
        .saturating_add(effective_trace_shape[1])
        .saturating_add(window.xline_stepout)
        .min(reader.volume().shape[1]);
    let trace_shape = [
        inline_end.saturating_sub(inline_start),
        xline_end.saturating_sub(xline_start),
    ];
    let samples = reader.volume().shape[2];
    let raw =
        assemble_source_trace_matrix(reader, inline_start, xline_start, trace_shape, 0, samples)?;
    Ok(NeighborhoodTraceMatrix {
        inline_count: trace_shape[0],
        xline_count: trace_shape[1],
        samples,
        center_inline_offset: output_inline_start.saturating_sub(inline_start),
        center_xline_offset: output_xline_start.saturating_sub(xline_start),
        amplitudes: raw.amplitudes,
        occupancy: raw.occupancy,
    })
}

fn similarity_tile_amplitudes(
    neighborhood: &NeighborhoodTraceMatrix,
    window: &PostStackNeighborhoodWindow,
    sample_interval_ms: f32,
    tile_shape: [usize; 3],
    effective_trace_shape: [usize; 2],
) -> Result<Vec<f32>, SeismicStoreError> {
    let gate_half_samples = gate_half_samples(sample_interval_ms, window.gate_ms);
    let mut amplitudes = vec![0.0_f32; tile_shape[0] * tile_shape[1] * tile_shape[2]];

    for local_i in 0..effective_trace_shape[0] {
        for local_x in 0..effective_trace_shape[1] {
            let center_i = neighborhood.center_inline_offset + local_i;
            let center_x = neighborhood.center_xline_offset + local_x;
            if neighborhood.is_unoccupied(center_i, center_x) {
                continue;
            }
            let center_trace = neighborhood.trace_slice(center_i, center_x)?;
            let destination_trace_index = (local_i * tile_shape[1]) + local_x;
            let output_trace = &mut amplitudes[destination_trace_index * tile_shape[2]
                ..(destination_trace_index + 1) * tile_shape[2]];
            for sample_index in 0..tile_shape[2] {
                output_trace[sample_index] = similarity_at_matrix_sample(
                    neighborhood,
                    center_i,
                    center_x,
                    sample_index,
                    window,
                    gate_half_samples,
                    center_trace,
                )?;
            }
        }
    }

    Ok(amplitudes)
}

fn similarity_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
    center_trace: &[f32],
) -> Result<f32, SeismicStoreError> {
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, matrix.samples, gate_half_samples);
    let center_window = &center_trace[sample_start..sample_end];
    let center_window_norm = signal_window_l2_norm(center_window);
    if center_window_norm <= SIMILARITY_EPSILON {
        return Ok(0.0);
    }
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let mut sum = 0.0_f32;
    let mut count = 0usize;

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if neighbor_i == center_i && neighbor_x == center_x {
                continue;
            }
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            let neighbor_trace = matrix.trace_slice(neighbor_i, neighbor_x)?;
            let neighbor_window = &neighbor_trace[sample_start..sample_end];
            sum += normalized_cross_correlation_abs_with_left_norm(
                center_window,
                center_window_norm,
                neighbor_window,
            );
            count += 1;
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        Ok((sum / count as f32).clamp(0.0, 1.0))
    }
}

fn local_volume_stats_tile_amplitudes(
    neighborhood: &NeighborhoodTraceMatrix,
    window: &PostStackNeighborhoodWindow,
    sample_interval_ms: f32,
    tile_shape: [usize; 3],
    effective_trace_shape: [usize; 2],
    statistic: LocalVolumeStatistic,
) -> Result<Vec<f32>, SeismicStoreError> {
    match statistic {
        LocalVolumeStatistic::Mean => local_volume_stats_tile_amplitudes_with(
            neighborhood,
            window,
            sample_interval_ms,
            tile_shape,
            effective_trace_shape,
            local_volume_stats_mean_at_matrix_sample,
        ),
        LocalVolumeStatistic::Rms => local_volume_stats_tile_amplitudes_with(
            neighborhood,
            window,
            sample_interval_ms,
            tile_shape,
            effective_trace_shape,
            local_volume_stats_rms_at_matrix_sample,
        ),
        LocalVolumeStatistic::Variance => local_volume_stats_tile_amplitudes_with(
            neighborhood,
            window,
            sample_interval_ms,
            tile_shape,
            effective_trace_shape,
            local_volume_stats_variance_at_matrix_sample,
        ),
        LocalVolumeStatistic::Minimum => local_volume_stats_tile_amplitudes_with(
            neighborhood,
            window,
            sample_interval_ms,
            tile_shape,
            effective_trace_shape,
            local_volume_stats_minimum_at_matrix_sample,
        ),
        LocalVolumeStatistic::Maximum => local_volume_stats_tile_amplitudes_with(
            neighborhood,
            window,
            sample_interval_ms,
            tile_shape,
            effective_trace_shape,
            local_volume_stats_maximum_at_matrix_sample,
        ),
    }
}

fn local_volume_stats_tile_amplitudes_with(
    neighborhood: &NeighborhoodTraceMatrix,
    window: &PostStackNeighborhoodWindow,
    sample_interval_ms: f32,
    tile_shape: [usize; 3],
    effective_trace_shape: [usize; 2],
    sample_fn: fn(
        &NeighborhoodTraceMatrix,
        usize,
        usize,
        usize,
        &PostStackNeighborhoodWindow,
        usize,
    ) -> Result<f32, SeismicStoreError>,
) -> Result<Vec<f32>, SeismicStoreError> {
    let gate_half_samples = gate_half_samples(sample_interval_ms, window.gate_ms);
    let mut amplitudes = vec![0.0_f32; tile_shape[0] * tile_shape[1] * tile_shape[2]];

    for local_i in 0..effective_trace_shape[0] {
        for local_x in 0..effective_trace_shape[1] {
            let center_i = neighborhood.center_inline_offset + local_i;
            let center_x = neighborhood.center_xline_offset + local_x;
            if neighborhood.is_unoccupied(center_i, center_x) {
                continue;
            }
            let destination_trace_index = (local_i * tile_shape[1]) + local_x;
            let output_trace = &mut amplitudes[destination_trace_index * tile_shape[2]
                ..(destination_trace_index + 1) * tile_shape[2]];
            for sample_index in 0..tile_shape[2] {
                output_trace[sample_index] = sample_fn(
                    neighborhood,
                    center_i,
                    center_x,
                    sample_index,
                    window,
                    gate_half_samples,
                )?;
            }
        }
    }

    Ok(amplitudes)
}

fn local_volume_stats_mean_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, matrix.samples, gate_half_samples);
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let mut count = 0usize;
    let mut sum = 0.0_f64;

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            let neighbor_trace = matrix.trace_slice(neighbor_i, neighbor_x)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                count += 1;
                sum += f64::from(sample);
            }
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        Ok((sum / count as f64) as f32)
    }
}

fn local_volume_stats_rms_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, matrix.samples, gate_half_samples);
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let mut count = 0usize;
    let mut sum_squares = 0.0_f64;

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            let neighbor_trace = matrix.trace_slice(neighbor_i, neighbor_x)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                count += 1;
                let sample = f64::from(sample);
                sum_squares += sample * sample;
            }
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        Ok((sum_squares / count as f64).sqrt() as f32)
    }
}

fn local_volume_stats_variance_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, matrix.samples, gate_half_samples);
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let mut count = 0usize;
    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            let neighbor_trace = matrix.trace_slice(neighbor_i, neighbor_x)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                let sample = f64::from(sample);
                count += 1;
                sum += sample;
                sum_squares += sample * sample;
            }
        }
    }

    if count == 0 {
        Ok(0.0)
    } else {
        let mean = sum / count as f64;
        Ok(((sum_squares / count as f64) - (mean * mean)).max(0.0) as f32)
    }
}

fn local_volume_stats_minimum_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, matrix.samples, gate_half_samples);
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let mut minimum = None::<f32>;

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            let neighbor_trace = matrix.trace_slice(neighbor_i, neighbor_x)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                minimum = Some(match minimum {
                    Some(current) => current.min(sample),
                    None => sample,
                });
            }
        }
    }

    Ok(minimum.unwrap_or(0.0))
}

fn local_volume_stats_maximum_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
) -> Result<f32, SeismicStoreError> {
    let (sample_start, sample_end) =
        symmetric_gate_bounds(sample_index, matrix.samples, gate_half_samples);
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let mut maximum = None::<f32>;

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            let neighbor_trace = matrix.trace_slice(neighbor_i, neighbor_x)?;
            for &sample in &neighbor_trace[sample_start..sample_end] {
                maximum = Some(match maximum {
                    Some(current) => current.max(sample),
                    None => sample,
                });
            }
        }
    }

    Ok(maximum.unwrap_or(0.0))
}

#[cfg(test)]
fn local_volume_stats_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    sample_index: usize,
    window: &PostStackNeighborhoodWindow,
    gate_half_samples: usize,
    statistic: LocalVolumeStatistic,
) -> Result<f32, SeismicStoreError> {
    match statistic {
        LocalVolumeStatistic::Mean => local_volume_stats_mean_at_matrix_sample(
            matrix,
            center_i,
            center_x,
            sample_index,
            window,
            gate_half_samples,
        ),
        LocalVolumeStatistic::Rms => local_volume_stats_rms_at_matrix_sample(
            matrix,
            center_i,
            center_x,
            sample_index,
            window,
            gate_half_samples,
        ),
        LocalVolumeStatistic::Variance => local_volume_stats_variance_at_matrix_sample(
            matrix,
            center_i,
            center_x,
            sample_index,
            window,
            gate_half_samples,
        ),
        LocalVolumeStatistic::Minimum => local_volume_stats_minimum_at_matrix_sample(
            matrix,
            center_i,
            center_x,
            sample_index,
            window,
            gate_half_samples,
        ),
        LocalVolumeStatistic::Maximum => local_volume_stats_maximum_at_matrix_sample(
            matrix,
            center_i,
            center_x,
            sample_index,
            window,
            gate_half_samples,
        ),
    }
}

fn dip_section_plane(
    axis: SectionAxis,
    sections: Vec<SectionPlane>,
    center_section_offset: usize,
    window: &PostStackNeighborhoodWindow,
    output: NeighborhoodDipOutput,
) -> Result<SectionPlane, SeismicStoreError> {
    let center = sections.get(center_section_offset).ok_or_else(|| {
        SeismicStoreError::Message("dip preview center section was not loaded".to_string())
    })?;
    let sample_interval_ms = sample_interval_ms_from_axis(&center.sample_axis_ms)?;
    let (_, trace_radius) = section_and_trace_radii(axis, window);
    let gate_half_samples = gate_half_samples(sample_interval_ms, window.gate_ms);
    let lag_half_samples = gate_half_samples.max(1);
    let mut amplitudes = vec![0.0_f32; center.amplitudes.len()];

    for trace_index in 0..center.traces {
        if is_unoccupied(center.occupancy.as_deref(), trace_index) {
            continue;
        }
        let center_trace = plane_trace_slice(center, trace_index)?;
        let neighbors = section_dip_neighbors(
            axis,
            &sections,
            center_section_offset,
            trace_index,
            trace_radius,
        );
        for sample_index in 0..center.samples {
            let estimate = dip_estimate_at_section_sample(
                &sections,
                sample_index,
                gate_half_samples,
                lag_half_samples,
                sample_interval_ms,
                center_trace,
                &neighbors,
            )?;
            amplitudes[trace_index * center.samples + sample_index] =
                dip_output_value(estimate, output);
        }
    }

    Ok(SectionPlane {
        axis,
        coordinate_index: center.coordinate_index,
        coordinate_value: center.coordinate_value,
        traces: center.traces,
        samples: center.samples,
        horizontal_axis: center.horizontal_axis.clone(),
        sample_axis_ms: center.sample_axis_ms.clone(),
        amplitudes,
        occupancy: center.occupancy.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn dip_estimate_at_section_sample(
    sections: &[SectionPlane],
    sample_index: usize,
    gate_half_samples: usize,
    lag_half_samples: usize,
    sample_interval_ms: f32,
    center_trace: &[f32],
    neighbors: &[SectionDipNeighbor],
) -> Result<DipEstimate, SeismicStoreError> {
    let mut accumulator = DipObservationAccumulator::default();
    let (center_start, center_end) =
        symmetric_gate_bounds(sample_index, center_trace.len(), gate_half_samples);
    let center_window = center_trace
        .get(center_start..center_end)
        .filter(|window| !window.is_empty())
        .ok_or_else(|| {
            SeismicStoreError::Message("dip preview center gate is out of bounds".to_string())
        })?;
    let center_window_norm = signal_window_l2_norm(center_window);
    if center_window_norm <= SIMILARITY_EPSILON {
        return Ok(accumulator.solve());
    }

    for neighbor in neighbors {
        let section = sections.get(neighbor.section_index).ok_or_else(|| {
            SeismicStoreError::Message("dip preview neighbor section is out of bounds".to_string())
        })?;
        let neighbor_trace = plane_trace_slice_at_offset(section, neighbor.trace_offset);
        let Some((lag_samples, weight)) = best_lag_and_weight(
            center_window,
            center_start,
            neighbor_trace,
            lag_half_samples,
            center_window_norm,
        ) else {
            continue;
        };
        accumulator.add_observation(
            neighbor.delta_inline,
            neighbor.delta_xline,
            f64::from(lag_samples * sample_interval_ms),
            f64::from(weight),
        );
    }

    Ok(accumulator.solve())
}

fn matrix_dip_neighbors(
    matrix: &NeighborhoodTraceMatrix,
    center_i: usize,
    center_x: usize,
    window: &PostStackNeighborhoodWindow,
) -> Vec<MatrixDipNeighbor> {
    let inline_start = center_i.saturating_sub(window.inline_stepout);
    let inline_end = center_i
        .saturating_add(window.inline_stepout)
        .min(matrix.inline_count.saturating_sub(1));
    let xline_start = center_x.saturating_sub(window.xline_stepout);
    let xline_end = center_x
        .saturating_add(window.xline_stepout)
        .min(matrix.xline_count.saturating_sub(1));
    let capacity = inline_end
        .saturating_sub(inline_start)
        .saturating_add(1)
        .saturating_mul(xline_end.saturating_sub(xline_start).saturating_add(1))
        .saturating_sub(1);
    let mut neighbors = Vec::with_capacity(capacity);

    for neighbor_i in inline_start..=inline_end {
        for neighbor_x in xline_start..=xline_end {
            if neighbor_i == center_i && neighbor_x == center_x {
                continue;
            }
            if matrix.is_unoccupied(neighbor_i, neighbor_x) {
                continue;
            }
            neighbors.push(MatrixDipNeighbor {
                trace_offset: matrix.trace_offset(neighbor_i, neighbor_x),
                delta_inline: neighbor_i as f64 - center_i as f64,
                delta_xline: neighbor_x as f64 - center_x as f64,
            });
        }
    }

    neighbors
}

fn section_dip_neighbors(
    axis: SectionAxis,
    sections: &[SectionPlane],
    center_section_offset: usize,
    center_trace_index: usize,
    trace_radius: usize,
) -> Vec<SectionDipNeighbor> {
    let mut capacity = 0usize;
    for section in sections {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        capacity += trace_end.saturating_sub(trace_start).saturating_add(1);
    }
    capacity = capacity.saturating_sub(1);
    let mut neighbors = Vec::with_capacity(capacity);

    for (section_index, section) in sections.iter().enumerate() {
        let trace_start = center_trace_index.saturating_sub(trace_radius);
        let trace_end = center_trace_index
            .saturating_add(trace_radius)
            .min(section.traces.saturating_sub(1));
        for neighbor_trace_index in trace_start..=trace_end {
            if section_index == center_section_offset && neighbor_trace_index == center_trace_index
            {
                continue;
            }
            if is_unoccupied(section.occupancy.as_deref(), neighbor_trace_index) {
                continue;
            }
            let (delta_inline, delta_xline) = match axis {
                SectionAxis::Inline => (
                    section_index as f64 - center_section_offset as f64,
                    neighbor_trace_index as f64 - center_trace_index as f64,
                ),
                SectionAxis::Xline => (
                    neighbor_trace_index as f64 - center_trace_index as f64,
                    section_index as f64 - center_section_offset as f64,
                ),
            };
            neighbors.push(SectionDipNeighbor {
                section_index,
                trace_offset: plane_trace_offset(section, neighbor_trace_index),
                delta_inline,
                delta_xline,
            });
        }
    }

    neighbors
}

fn dip_tile_amplitudes(
    neighborhood: &NeighborhoodTraceMatrix,
    window: &PostStackNeighborhoodWindow,
    sample_interval_ms: f32,
    tile_shape: [usize; 3],
    effective_trace_shape: [usize; 2],
    output: NeighborhoodDipOutput,
) -> Result<Vec<f32>, SeismicStoreError> {
    let gate_half_samples = gate_half_samples(sample_interval_ms, window.gate_ms);
    let lag_half_samples = gate_half_samples.max(1);
    let mut amplitudes = vec![0.0_f32; tile_shape[0] * tile_shape[1] * tile_shape[2]];

    for local_i in 0..effective_trace_shape[0] {
        for local_x in 0..effective_trace_shape[1] {
            let center_i = neighborhood.center_inline_offset + local_i;
            let center_x = neighborhood.center_xline_offset + local_x;
            if neighborhood.is_unoccupied(center_i, center_x) {
                continue;
            }
            let center_trace = neighborhood.trace_slice(center_i, center_x)?;
            let neighbors = matrix_dip_neighbors(neighborhood, center_i, center_x, window);
            let destination_trace_index = (local_i * tile_shape[1]) + local_x;
            let output_trace = &mut amplitudes[destination_trace_index * tile_shape[2]
                ..(destination_trace_index + 1) * tile_shape[2]];
            for sample_index in 0..tile_shape[2] {
                let estimate = dip_estimate_at_matrix_sample(
                    neighborhood,
                    sample_index,
                    gate_half_samples,
                    lag_half_samples,
                    sample_interval_ms,
                    center_trace,
                    &neighbors,
                )?;
                output_trace[sample_index] = dip_output_value(estimate, output);
            }
        }
    }

    Ok(amplitudes)
}

#[allow(clippy::too_many_arguments)]
fn dip_estimate_at_matrix_sample(
    matrix: &NeighborhoodTraceMatrix,
    sample_index: usize,
    gate_half_samples: usize,
    lag_half_samples: usize,
    sample_interval_ms: f32,
    center_trace: &[f32],
    neighbors: &[MatrixDipNeighbor],
) -> Result<DipEstimate, SeismicStoreError> {
    let mut accumulator = DipObservationAccumulator::default();
    let (center_start, center_end) =
        symmetric_gate_bounds(sample_index, center_trace.len(), gate_half_samples);
    let center_window = center_trace
        .get(center_start..center_end)
        .filter(|window| !window.is_empty())
        .ok_or_else(|| {
            SeismicStoreError::Message("dip matrix center gate is out of bounds".to_string())
        })?;
    let center_window_norm = signal_window_l2_norm(center_window);
    if center_window_norm <= SIMILARITY_EPSILON {
        return Ok(accumulator.solve());
    }

    for neighbor in neighbors {
        let neighbor_trace = matrix.trace_slice_at_offset(neighbor.trace_offset);
        let Some((lag_samples, weight)) = best_lag_and_weight(
            center_window,
            center_start,
            neighbor_trace,
            lag_half_samples,
            center_window_norm,
        ) else {
            continue;
        };
        accumulator.add_observation(
            neighbor.delta_inline,
            neighbor.delta_xline,
            f64::from(lag_samples * sample_interval_ms),
            f64::from(weight),
        );
    }

    Ok(accumulator.solve())
}

impl NeighborhoodTraceMatrix {
    fn trace_offset(&self, inline_index: usize, xline_index: usize) -> usize {
        debug_assert!(inline_index < self.inline_count);
        debug_assert!(xline_index < self.xline_count);
        ((inline_index * self.xline_count) + xline_index) * self.samples
    }

    fn trace_slice_at_offset(&self, trace_offset: usize) -> &[f32] {
        let end = trace_offset + self.samples;
        debug_assert!(end <= self.amplitudes.len());
        &self.amplitudes[trace_offset..end]
    }

    fn trace_slice(
        &self,
        inline_index: usize,
        xline_index: usize,
    ) -> Result<&[f32], SeismicStoreError> {
        if inline_index >= self.inline_count || xline_index >= self.xline_count {
            return Err(SeismicStoreError::Message(format!(
                "neighborhood trace ({inline_index}, {xline_index}) is out of bounds for [{}, {}]",
                self.inline_count, self.xline_count
            )));
        }
        Ok(self.trace_slice_at_offset(self.trace_offset(inline_index, xline_index)))
    }

    fn is_unoccupied(&self, inline_index: usize, xline_index: usize) -> bool {
        let trace_index = match inline_index
            .checked_mul(self.xline_count)
            .and_then(|value| value.checked_add(xline_index))
        {
            Some(value) => value,
            None => return true,
        };
        is_unoccupied(self.occupancy.as_deref(), trace_index)
    }
}

fn assemble_source_trace_matrix(
    reader: &TbvolReader,
    source_inline_start: usize,
    source_xline_start: usize,
    trace_shape: [usize; 2],
    sample_start: usize,
    sample_end_exclusive: usize,
) -> Result<LoadedSourceTile, SeismicStoreError> {
    let sample_count = sample_end_exclusive.saturating_sub(sample_start);
    let trace_count = trace_shape[0] * trace_shape[1];
    let mut amplitudes = vec![0.0_f32; trace_count * sample_count];
    let mut occupancy = vec![0_u8; trace_count];
    let mut has_occupancy = false;
    let mut cache = HashMap::<TileCoord, LoadedSourceTile>::new();
    let source_tile_shape = reader.tile_geometry().tile_shape();

    for local_i in 0..trace_shape[0] {
        for local_x in 0..trace_shape[1] {
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
            let destination_trace_index = (local_i * trace_shape[1]) + local_x;
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

fn plane_trace_slice<'a>(
    plane: &'a SectionPlane,
    trace_index: usize,
) -> Result<&'a [f32], SeismicStoreError> {
    if trace_index >= plane.traces {
        return Err(SeismicStoreError::Message(format!(
            "section trace {trace_index} is out of bounds for trace count {}",
            plane.traces
        )));
    }
    Ok(plane_trace_slice_at_offset(
        plane,
        plane_trace_offset(plane, trace_index),
    ))
}

fn plane_trace_offset(plane: &SectionPlane, trace_index: usize) -> usize {
    debug_assert!(trace_index < plane.traces);
    trace_index * plane.samples
}

fn plane_trace_slice_at_offset(plane: &SectionPlane, trace_offset: usize) -> &[f32] {
    let end = trace_offset + plane.samples;
    debug_assert!(end <= plane.amplitudes.len());
    &plane.amplitudes[trace_offset..end]
}

fn section_and_trace_radii(
    axis: SectionAxis,
    window: &PostStackNeighborhoodWindow,
) -> (usize, usize) {
    match axis {
        SectionAxis::Inline => (window.inline_stepout, window.xline_stepout),
        SectionAxis::Xline => (window.xline_stepout, window.inline_stepout),
    }
}

fn gate_half_samples(sample_interval_ms: f32, gate_ms: f32) -> usize {
    let window_samples = ((gate_ms / sample_interval_ms).round() as usize).max(1);
    window_samples / 2
}

fn symmetric_gate_bounds(
    center_sample: usize,
    sample_count: usize,
    half_window: usize,
) -> (usize, usize) {
    let start = center_sample.saturating_sub(half_window);
    let end = center_sample
        .saturating_add(half_window)
        .saturating_add(1)
        .min(sample_count);
    (start, end)
}

#[cfg(test)]
fn normalized_cross_correlation_abs(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let left_norm = signal_window_l2_norm(left);
    normalized_cross_correlation_abs_with_left_norm(left, left_norm, right)
}

fn signal_window_l2_norm(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    let mut energy = 0.0_f64;
    for &value in signal {
        let value = f64::from(value);
        energy += value * value;
    }
    energy.sqrt() as f32
}

fn normalized_cross_correlation_abs_with_left_norm(
    left: &[f32],
    left_norm: f32,
    right: &[f32],
) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut right_energy = 0.0_f64;
    for (&lhs, &rhs) in left.iter().zip(right.iter()) {
        let lhs = f64::from(lhs);
        let rhs = f64::from(rhs);
        dot += lhs * rhs;
        right_energy += rhs * rhs;
    }
    let denominator = left_norm * right_energy.sqrt() as f32;
    if denominator <= SIMILARITY_EPSILON {
        0.0
    } else {
        ((dot as f32 / denominator).abs()).clamp(0.0, 1.0)
    }
}

fn best_lag_and_weight(
    center_window: &[f32],
    center_start: usize,
    neighbor_trace: &[f32],
    lag_half_samples: usize,
    center_window_norm: f32,
) -> Option<(f32, f32)> {
    let center_len = center_window.len();
    if center_len == 0 {
        return None;
    }
    let max_neighbor_start = neighbor_trace.len().checked_sub(center_len)?;
    let center_start = center_start as isize;
    let lag_half_samples = lag_half_samples as isize;
    let min_lag = (-center_start).max(-lag_half_samples);
    let max_lag = (max_neighbor_start as isize - center_start).min(lag_half_samples);
    if min_lag > max_lag {
        return None;
    }

    let mut best_lag = 0_isize;
    let mut best_weight = 0.0_f32;

    for lag in min_lag..=max_lag {
        let neighbor_start = (center_start + lag) as usize;
        let neighbor_window = &neighbor_trace[neighbor_start..neighbor_start + center_len];
        let weight = normalized_cross_correlation_abs_with_left_norm(
            center_window,
            center_window_norm,
            neighbor_window,
        );
        let replace = weight > best_weight + SIMILARITY_EPSILON
            || ((weight - best_weight).abs() <= SIMILARITY_EPSILON
                && lag.unsigned_abs() < best_lag.unsigned_abs());
        if replace {
            best_lag = lag;
            best_weight = weight;
        }
    }

    if best_weight <= SIMILARITY_EPSILON {
        None
    } else {
        Some((best_lag as f32, best_weight))
    }
}

fn dip_output_value(estimate: DipEstimate, output: NeighborhoodDipOutput) -> f32 {
    match output {
        NeighborhoodDipOutput::Inline => estimate.inline_ms_per_trace,
        NeighborhoodDipOutput::Xline => estimate.xline_ms_per_trace,
        NeighborhoodDipOutput::Azimuth => {
            if estimate.inline_ms_per_trace.abs() <= SIMILARITY_EPSILON
                && estimate.xline_ms_per_trace.abs() <= SIMILARITY_EPSILON
            {
                0.0
            } else {
                estimate
                    .xline_ms_per_trace
                    .atan2(estimate.inline_ms_per_trace)
                    .to_degrees()
                    .rem_euclid(360.0)
            }
        }
        NeighborhoodDipOutput::AbsDip => estimate
            .inline_ms_per_trace
            .hypot(estimate.xline_ms_per_trace),
    }
}

fn sample_interval_ms_from_axis(sample_axis_ms: &[f32]) -> Result<f32, SeismicStoreError> {
    if sample_axis_ms.len() < 2 {
        return Err(SeismicStoreError::Message(
            "sample axis must contain at least two samples".to_string(),
        ));
    }
    let interval = (sample_axis_ms[1] - sample_axis_ms[0]).abs();
    if !interval.is_finite() || interval <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "sample interval must be finite and > 0, found {interval}"
        )));
    }
    Ok(interval)
}

fn resolve_chunk_shape(chunk_shape: [usize; 3], volume_shape: [usize; 3]) -> [usize; 3] {
    if chunk_shape.iter().all(|value| *value > 0) {
        [
            chunk_shape[0].min(volume_shape[0].max(1)),
            chunk_shape[1].min(volume_shape[1].max(1)),
            volume_shape[2],
        ]
    } else {
        [volume_shape[0], volume_shape[1], volume_shape[2]]
    }
}

fn reader_has_occupancy(reader: &TbvolReader) -> Result<bool, SeismicStoreError> {
    reader
        .read_tile_occupancy(TileCoord {
            tile_i: 0,
            tile_x: 0,
        })
        .map(|value| value.is_some())
}

fn derived_post_stack_neighborhood_volume_metadata(
    input: &VolumeMetadata,
    parent_store: &Path,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
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
            parent_store: parent_store.to_path_buf(),
            parent_store_id: input.store_id.clone(),
            artifact_role: ProcessingArtifactRole::FinalOutput,
            pipeline: ProcessingPipelineSpec::PostStackNeighborhood {
                pipeline: pipeline.clone(),
            },
            runtime_version: RUNTIME_VERSION.to_string(),
            created_at_unix_s: unix_timestamp_s(),
        }),
    }
}

fn unique_temp_store_path(output_store_root: &Path, label: &str) -> PathBuf {
    let parent = output_store_root.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_store_root
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("derived");
    let timestamp = unix_timestamp_s();
    parent.join(format!("{stem}.{label}.{timestamp}.tbvol"))
}

fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_unoccupied(occupancy: Option<&[u8]>, trace_index: usize) -> bool {
    occupancy
        .and_then(|mask| mask.get(trace_index).copied())
        .unwrap_or(1)
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_window_rejects_zero_stepout() {
        let error = validate_window(&PostStackNeighborhoodWindow {
            gate_ms: 24.0,
            inline_stepout: 0,
            xline_stepout: 0,
        })
        .expect_err("window should require a non-zero lateral stepout");

        assert!(
            error
                .to_string()
                .contains("stepout must be non-zero on at least one lateral axis")
        );
    }

    #[test]
    fn similarity_returns_zero_when_no_valid_neighbors_exist() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 1,
            samples: 5,
            center_inline_offset: 0,
            center_xline_offset: 0,
            amplitudes: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            occupancy: None,
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let center_trace = matrix.trace_slice(0, 0).expect("center trace");

        let similarity =
            similarity_at_matrix_sample(&matrix, 0, 0, 2, &window, 1, center_trace).unwrap();

        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn similarity_skips_unoccupied_neighbors() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 3,
            samples: 5,
            center_inline_offset: 0,
            center_xline_offset: 1,
            amplitudes: vec![
                50.0, -10.0, 25.0, 5.0, -2.0, //
                1.0, 2.0, 3.0, 4.0, 5.0, //
                1.0, 2.0, 3.0, 4.0, 5.0,
            ],
            occupancy: Some(vec![0, 1, 1]),
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 0,
            xline_stepout: 1,
        };
        let center_trace = matrix.trace_slice(0, 1).expect("center trace");

        let similarity =
            similarity_at_matrix_sample(&matrix, 0, 1, 2, &window, 1, center_trace).unwrap();

        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn normalized_cross_correlation_is_bounded_and_polarity_agnostic() {
        let similarity = normalized_cross_correlation_abs(&[1.0, -2.0, 3.0], &[-1.0, 2.0, -3.0]);
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn similarity_averages_mixed_continuity_neighbors() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 3,
            samples: 5,
            center_inline_offset: 0,
            center_xline_offset: 1,
            amplitudes: vec![
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, -1.0, 0.0, 1.0, -1.0,
            ],
            occupancy: Some(vec![1, 1, 1]),
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 0,
            xline_stepout: 1,
        };
        let gate_half_samples = gate_half_samples(2.0, window.gate_ms);
        let center_trace = matrix.trace_slice(0, 1).expect("center trace");

        let similarity =
            similarity_at_matrix_sample(&matrix, 0, 1, 2, &window, gate_half_samples, center_trace)
                .unwrap();

        assert!((similarity - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn local_volume_stats_include_center_trace_samples() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 3,
            samples: 5,
            center_inline_offset: 0,
            center_xline_offset: 1,
            amplitudes: vec![
                10.0, 10.0, 10.0, 10.0, 10.0, //
                1.0, 2.0, 3.0, 4.0, 5.0, //
                20.0, 20.0, 20.0, 20.0, 20.0,
            ],
            occupancy: Some(vec![0, 1, 0]),
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 0,
            xline_stepout: 1,
        };

        let mean = local_volume_stats_at_matrix_sample(
            &matrix,
            0,
            1,
            2,
            &window,
            1,
            LocalVolumeStatistic::Mean,
        )
        .unwrap();

        assert_eq!(mean, 3.0);
    }

    #[test]
    fn local_volume_stats_support_variance_and_skip_unoccupied_neighbors() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 3,
            samples: 5,
            center_inline_offset: 0,
            center_xline_offset: 1,
            amplitudes: vec![
                100.0, 100.0, 100.0, 100.0, 100.0, //
                1.0, 2.0, 3.0, 4.0, 5.0, //
                1.0, 2.0, 3.0, 4.0, 5.0,
            ],
            occupancy: Some(vec![0, 1, 1]),
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 0,
            xline_stepout: 1,
        };

        let variance = local_volume_stats_at_matrix_sample(
            &matrix,
            0,
            1,
            2,
            &window,
            1,
            LocalVolumeStatistic::Variance,
        )
        .unwrap();

        assert!((variance - (2.0_f32 / 3.0)).abs() < 1.0e-6);
    }

    #[test]
    fn local_volume_stats_respect_edge_gates_for_rms_and_extrema() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 2,
            samples: 5,
            center_inline_offset: 0,
            center_xline_offset: 0,
            amplitudes: vec![
                3.0, 4.0, 5.0, 6.0, 7.0, //
                0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            occupancy: Some(vec![1, 1]),
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 0,
            xline_stepout: 1,
        };

        let rms = local_volume_stats_at_matrix_sample(
            &matrix,
            0,
            0,
            0,
            &window,
            1,
            LocalVolumeStatistic::Rms,
        )
        .unwrap();
        let minimum = local_volume_stats_at_matrix_sample(
            &matrix,
            0,
            0,
            0,
            &window,
            1,
            LocalVolumeStatistic::Minimum,
        )
        .unwrap();
        let maximum = local_volume_stats_at_matrix_sample(
            &matrix,
            0,
            0,
            0,
            &window,
            1,
            LocalVolumeStatistic::Maximum,
        )
        .unwrap();

        assert!((rms - 2.5).abs() < 1.0e-6);
        assert_eq!(minimum, 0.0);
        assert_eq!(maximum, 4.0);
    }

    #[test]
    fn preview_section_similarity_matches_matrix_similarity_on_same_center_section() {
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 3,
            xline_count: 3,
            samples: 5,
            center_inline_offset: 1,
            center_xline_offset: 1,
            amplitudes: vec![
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0, //
                1.0, 2.0, 3.0, 2.0, 1.0,
            ],
            occupancy: Some(vec![1; 9]),
        };
        let gate_half_samples = gate_half_samples(2.0, window.gate_ms);
        let sections = vec![
            section_plane_from_matrix_inline(&matrix, 0),
            section_plane_from_matrix_inline(&matrix, 1),
            section_plane_from_matrix_inline(&matrix, 2),
        ];
        let preview = similarity_section_plane(SectionAxis::Inline, sections, 1, &window).unwrap();

        for xline_index in 0..matrix.xline_count {
            let center_trace = matrix.trace_slice(1, xline_index).unwrap();
            let preview_trace = plane_trace_slice(&preview, xline_index).unwrap();
            for sample_index in 0..matrix.samples {
                let matrix_value = similarity_at_matrix_sample(
                    &matrix,
                    1,
                    xline_index,
                    sample_index,
                    &window,
                    gate_half_samples,
                    center_trace,
                )
                .unwrap();
                assert!(
                    (preview_trace[sample_index] - matrix_value).abs() < 1.0e-6,
                    "mismatch at xline {xline_index} sample {sample_index}: preview={} matrix={matrix_value}",
                    preview_trace[sample_index]
                );
            }
        }
    }

    #[test]
    fn preview_section_local_volume_stats_matches_matrix_local_volume_stats() {
        let window = PostStackNeighborhoodWindow {
            gate_ms: 4.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 3,
            xline_count: 3,
            samples: 5,
            center_inline_offset: 1,
            center_xline_offset: 1,
            amplitudes: vec![
                1.0, 2.0, 3.0, 4.0, 5.0, //
                2.0, 3.0, 4.0, 5.0, 6.0, //
                3.0, 4.0, 5.0, 6.0, 7.0, //
                4.0, 5.0, 6.0, 7.0, 8.0, //
                5.0, 6.0, 7.0, 8.0, 9.0, //
                6.0, 7.0, 8.0, 9.0, 10.0, //
                7.0, 8.0, 9.0, 10.0, 11.0, //
                8.0, 9.0, 10.0, 11.0, 12.0, //
                9.0, 10.0, 11.0, 12.0, 13.0,
            ],
            occupancy: Some(vec![1; 9]),
        };
        let gate_half_samples = gate_half_samples(2.0, window.gate_ms);
        let sections = vec![
            section_plane_from_matrix_inline(&matrix, 0),
            section_plane_from_matrix_inline(&matrix, 1),
            section_plane_from_matrix_inline(&matrix, 2),
        ];
        let preview = local_volume_stats_section_plane(
            SectionAxis::Inline,
            sections,
            1,
            &window,
            LocalVolumeStatistic::Mean,
        )
        .unwrap();

        for xline_index in 0..matrix.xline_count {
            let preview_trace = plane_trace_slice(&preview, xline_index).unwrap();
            for sample_index in 0..matrix.samples {
                let matrix_value = local_volume_stats_at_matrix_sample(
                    &matrix,
                    1,
                    xline_index,
                    sample_index,
                    &window,
                    gate_half_samples,
                    LocalVolumeStatistic::Mean,
                )
                .unwrap();
                assert!(
                    (preview_trace[sample_index] - matrix_value).abs() < 1.0e-6,
                    "mismatch at xline {xline_index} sample {sample_index}: preview={} matrix={matrix_value}",
                    preview_trace[sample_index]
                );
            }
        }
    }

    #[test]
    fn dip_returns_zero_when_no_valid_neighbors_exist() {
        let matrix = NeighborhoodTraceMatrix {
            inline_count: 1,
            xline_count: 1,
            samples: 9,
            center_inline_offset: 0,
            center_xline_offset: 0,
            amplitudes: vec![0.0, 0.0, 0.0, 1.0, 2.0, 1.0, 0.0, 0.0, 0.0],
            occupancy: None,
        };
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let gate_half = gate_half_samples(2.0, window.gate_ms);
        let lag_half = gate_half.max(1);
        let center_trace = matrix.trace_slice(0, 0).expect("center trace");
        let neighbors = matrix_dip_neighbors(&matrix, 0, 0, &window);

        let estimate = dip_estimate_at_matrix_sample(
            &matrix,
            4,
            gate_half,
            lag_half,
            2.0,
            center_trace,
            &neighbors,
        )
        .unwrap();

        assert_eq!(estimate.inline_ms_per_trace, 0.0);
        assert_eq!(estimate.xline_ms_per_trace, 0.0);
    }

    #[test]
    fn dip_recovers_inline_and_xline_slopes_on_synthetic_event() {
        let matrix = synthetic_dip_matrix(2, 2, 17, 6, 1, 2);
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let gate_half = gate_half_samples(2.0, window.gate_ms);
        let lag_half = gate_half.max(1);
        let center_trace = matrix.trace_slice(2, 2).expect("center trace");
        let neighbors = matrix_dip_neighbors(&matrix, 2, 2, &window);

        let estimate = dip_estimate_at_matrix_sample(
            &matrix,
            6,
            gate_half,
            lag_half,
            2.0,
            center_trace,
            &neighbors,
        )
        .unwrap();

        assert!((estimate.inline_ms_per_trace - 2.0).abs() < 0.25);
        assert!((estimate.xline_ms_per_trace - 4.0).abs() < 0.25);
        assert!(
            (dip_output_value(estimate, NeighborhoodDipOutput::AbsDip) - 20.0_f32.sqrt()).abs()
                < 0.35
        );
        assert!(
            (dip_output_value(estimate, NeighborhoodDipOutput::Azimuth) - 63.43495).abs() < 1.0
        );
    }

    #[test]
    fn dip_output_variants_match_pure_inline_slope() {
        let matrix = synthetic_dip_matrix(5, 5, 21, 10, 1, 0);
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let gate_half = gate_half_samples(2.0, window.gate_ms);
        let lag_half = gate_half.max(1);
        let center_trace = matrix.trace_slice(2, 2).expect("center trace");
        let neighbors = matrix_dip_neighbors(&matrix, 2, 2, &window);

        let estimate = dip_estimate_at_matrix_sample(
            &matrix,
            10,
            gate_half,
            lag_half,
            2.0,
            center_trace,
            &neighbors,
        )
        .unwrap();

        assert!((estimate.inline_ms_per_trace - 2.0).abs() < 0.25);
        assert!(estimate.xline_ms_per_trace.abs() < 0.25);
        assert!((dip_output_value(estimate, NeighborhoodDipOutput::AbsDip) - 2.0).abs() < 0.25);
        assert!(dip_output_value(estimate, NeighborhoodDipOutput::Azimuth).abs() < 1.0);
    }

    #[test]
    fn dip_output_variants_match_pure_xline_slope() {
        let matrix = synthetic_dip_matrix(5, 5, 21, 10, 0, 2);
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let gate_half = gate_half_samples(2.0, window.gate_ms);
        let lag_half = gate_half.max(1);
        let center_trace = matrix.trace_slice(2, 2).expect("center trace");
        let neighbors = matrix_dip_neighbors(&matrix, 2, 2, &window);

        let estimate = dip_estimate_at_matrix_sample(
            &matrix,
            10,
            gate_half,
            lag_half,
            2.0,
            center_trace,
            &neighbors,
        )
        .unwrap();

        assert!(estimate.inline_ms_per_trace.abs() < 0.25);
        assert!((estimate.xline_ms_per_trace - 4.0).abs() < 0.25);
        assert!((dip_output_value(estimate, NeighborhoodDipOutput::AbsDip) - 4.0).abs() < 0.25);
        assert!((dip_output_value(estimate, NeighborhoodDipOutput::Azimuth) - 90.0).abs() < 1.0);
    }

    #[test]
    fn preview_section_dip_matches_matrix_dip_on_same_center_section() {
        let matrix = synthetic_dip_matrix(2, 2, 17, 6, 1, 2);
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let gate_half = gate_half_samples(2.0, window.gate_ms);
        let lag_half = gate_half.max(1);
        let sections = vec![
            section_plane_from_matrix_inline(&matrix, 1),
            section_plane_from_matrix_inline(&matrix, 2),
            section_plane_from_matrix_inline(&matrix, 3),
        ];
        let preview = dip_section_plane(
            SectionAxis::Inline,
            sections,
            1,
            &window,
            NeighborhoodDipOutput::Inline,
        )
        .unwrap();

        for xline_index in 0..matrix.xline_count {
            let center_trace = matrix.trace_slice(2, xline_index).unwrap();
            let neighbors = matrix_dip_neighbors(&matrix, 2, xline_index, &window);
            let preview_trace = plane_trace_slice(&preview, xline_index).unwrap();
            for sample_index in 0..matrix.samples {
                let matrix_value = dip_output_value(
                    dip_estimate_at_matrix_sample(
                        &matrix,
                        sample_index,
                        gate_half,
                        lag_half,
                        2.0,
                        center_trace,
                        &neighbors,
                    )
                    .unwrap(),
                    NeighborhoodDipOutput::Inline,
                );
                assert!(
                    (preview_trace[sample_index] - matrix_value).abs() < 1.0e-5,
                    "mismatch at xline {xline_index} sample {sample_index}: preview={} matrix={matrix_value}",
                    preview_trace[sample_index]
                );
            }
        }
    }

    #[test]
    fn preview_xline_dip_matches_matrix_dip_on_same_center_section() {
        let matrix = synthetic_dip_matrix(5, 5, 17, 6, 1, 2);
        let window = PostStackNeighborhoodWindow {
            gate_ms: 8.0,
            inline_stepout: 1,
            xline_stepout: 1,
        };
        let gate_half = gate_half_samples(2.0, window.gate_ms);
        let lag_half = gate_half.max(1);
        let sections = vec![
            section_plane_from_matrix_xline(&matrix, 1),
            section_plane_from_matrix_xline(&matrix, 2),
            section_plane_from_matrix_xline(&matrix, 3),
        ];
        let preview = dip_section_plane(
            SectionAxis::Xline,
            sections,
            1,
            &window,
            NeighborhoodDipOutput::Xline,
        )
        .unwrap();

        for inline_index in 0..matrix.inline_count {
            let center_trace = matrix.trace_slice(inline_index, 2).unwrap();
            let neighbors = matrix_dip_neighbors(&matrix, inline_index, 2, &window);
            let preview_trace = plane_trace_slice(&preview, inline_index).unwrap();
            for sample_index in 0..matrix.samples {
                let matrix_value = dip_output_value(
                    dip_estimate_at_matrix_sample(
                        &matrix,
                        sample_index,
                        gate_half,
                        lag_half,
                        2.0,
                        center_trace,
                        &neighbors,
                    )
                    .unwrap(),
                    NeighborhoodDipOutput::Xline,
                );
                assert!(
                    (preview_trace[sample_index] - matrix_value).abs() < 1.0e-5,
                    "mismatch at inline {inline_index} sample {sample_index}: preview={} matrix={matrix_value}",
                    preview_trace[sample_index]
                );
            }
        }
    }

    fn synthetic_dip_matrix(
        inline_count: usize,
        xline_count: usize,
        samples: usize,
        center_sample: usize,
        inline_shift_samples: isize,
        xline_shift_samples: isize,
    ) -> NeighborhoodTraceMatrix {
        let center_inline_offset = inline_count / 2;
        let center_xline_offset = xline_count / 2;
        let trace_count = inline_count * xline_count;
        let mut amplitudes = vec![0.0_f32; trace_count * samples];

        for inline_index in 0..inline_count {
            for xline_index in 0..xline_count {
                let delta_inline = inline_index as isize - center_inline_offset as isize;
                let delta_xline = xline_index as isize - center_xline_offset as isize;
                let shifted_center = center_sample as isize
                    + delta_inline * inline_shift_samples
                    + delta_xline * xline_shift_samples;
                let trace_index = (inline_index * xline_count) + xline_index;
                for (sample_offset, amplitude) in [
                    (-2, 0.25_f32),
                    (-1, 0.75_f32),
                    (0, 1.0_f32),
                    (1, 0.75_f32),
                    (2, 0.25_f32),
                ] {
                    let sample_index = shifted_center + sample_offset;
                    if (0..samples as isize).contains(&sample_index) {
                        amplitudes[trace_index * samples + sample_index as usize] = amplitude;
                    }
                }
            }
        }

        NeighborhoodTraceMatrix {
            inline_count,
            xline_count,
            samples,
            center_inline_offset,
            center_xline_offset,
            amplitudes,
            occupancy: Some(vec![1; trace_count]),
        }
    }

    fn section_plane_from_matrix_inline(
        matrix: &NeighborhoodTraceMatrix,
        inline_index: usize,
    ) -> SectionPlane {
        let mut amplitudes = Vec::with_capacity(matrix.xline_count * matrix.samples);
        let mut occupancy = Vec::with_capacity(matrix.xline_count);
        for xline_index in 0..matrix.xline_count {
            amplitudes.extend_from_slice(
                matrix
                    .trace_slice(inline_index, xline_index)
                    .expect("matrix trace should exist"),
            );
            occupancy.push(if matrix.is_unoccupied(inline_index, xline_index) {
                0
            } else {
                1
            });
        }

        SectionPlane {
            axis: SectionAxis::Inline,
            coordinate_index: inline_index,
            coordinate_value: inline_index as f64,
            traces: matrix.xline_count,
            samples: matrix.samples,
            horizontal_axis: (0..matrix.xline_count).map(|value| value as f64).collect(),
            sample_axis_ms: (0..matrix.samples)
                .map(|value| value as f32 * 2.0)
                .collect(),
            amplitudes,
            occupancy: Some(occupancy),
        }
    }

    fn section_plane_from_matrix_xline(
        matrix: &NeighborhoodTraceMatrix,
        xline_index: usize,
    ) -> SectionPlane {
        let mut amplitudes = Vec::with_capacity(matrix.inline_count * matrix.samples);
        let mut occupancy = Vec::with_capacity(matrix.inline_count);
        for inline_index in 0..matrix.inline_count {
            amplitudes.extend_from_slice(
                matrix
                    .trace_slice(inline_index, xline_index)
                    .expect("matrix trace should exist"),
            );
            occupancy.push(if matrix.is_unoccupied(inline_index, xline_index) {
                0
            } else {
                1
            });
        }

        SectionPlane {
            axis: SectionAxis::Xline,
            coordinate_index: xline_index,
            coordinate_value: xline_index as f64,
            traces: matrix.inline_count,
            samples: matrix.samples,
            horizontal_axis: (0..matrix.inline_count).map(|value| value as f64).collect(),
            sample_axis_ms: (0..matrix.samples)
                .map(|value| value as f32 * 2.0)
                .collect(),
            amplitudes,
            occupancy: Some(occupancy),
        }
    }
}
