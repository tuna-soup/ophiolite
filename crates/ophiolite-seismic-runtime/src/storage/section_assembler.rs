use crate::SectionAxis;
use crate::error::SeismicStoreError;
use crate::execution::{SectionDomain, SectionWindowDomain};
use crate::metadata::VolumeMetadata;
use crate::store::SectionPlane;

use super::tile_geometry::{TileCoord, TileGeometry};
use super::volume_store::VolumeStoreReader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionAssemblyPlan {
    pub axis: SectionAxis,
    pub section_index: usize,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub lod: u8,
    pub source_tiles: Vec<TileCoord>,
    pub output_shape: [usize; 2],
}

#[derive(Debug, Clone)]
pub struct SectionWindowArtifact {
    pub domain: SectionWindowDomain,
    pub assembly_plan: SectionAssemblyPlan,
    pub plane: SectionPlane,
}

#[derive(Debug, Clone)]
pub struct AssembledSectionArtifact {
    pub domain: SectionDomain,
    pub assembly_plan: SectionAssemblyPlan,
    pub plane: SectionPlane,
}

pub fn read_section_plane<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
) -> Result<SectionPlane, SeismicStoreError> {
    let volume = reader.volume();
    let geometry = reader.tile_geometry();
    validate_section_index(volume, axis, index)?;

    let traces = match axis {
        SectionAxis::Inline => volume.shape[1],
        SectionAxis::Xline => volume.shape[0],
    };
    let mut amplitudes = vec![0.0_f32; traces * volume.shape[2]];
    let mut occupancy = volume_has_occupancy(reader).then(|| vec![0_u8; traces]);

    for tile in geometry.section_tiles(axis, index) {
        let tile_values = reader.read_tile(tile)?;
        let tile_values = tile_values.as_slice();
        let tile_occupancy = reader.read_tile_occupancy(tile)?;
        copy_tile_into_section(
            geometry,
            axis,
            index,
            tile,
            tile_values,
            tile_occupancy.as_ref().map(|value| value.as_slice()),
            &mut amplitudes,
            occupancy.as_deref_mut(),
        );
    }

    let (horizontal_axis, coordinate_value) = match axis {
        SectionAxis::Inline => (volume.axes.xlines.clone(), volume.axes.ilines[index]),
        SectionAxis::Xline => (volume.axes.ilines.clone(), volume.axes.xlines[index]),
    };

    Ok(SectionPlane {
        axis,
        coordinate_index: index,
        coordinate_value,
        traces,
        samples: volume.shape[2],
        horizontal_axis,
        sample_axis_ms: volume.axes.sample_axis_ms.clone(),
        amplitudes,
        occupancy,
    })
}

pub fn read_section_tile_plane<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
) -> Result<SectionPlane, SeismicStoreError> {
    let volume = reader.volume();
    let geometry = reader.tile_geometry();
    validate_section_index(volume, axis, index)?;

    let total_traces = match axis {
        SectionAxis::Inline => volume.shape[1],
        SectionAxis::Xline => volume.shape[0],
    };
    validate_tile_window(total_traces, volume.shape[2], trace_range, sample_range)?;

    let trace_step = lod_step(lod)?;
    let sample_step = lod_step(lod)?;
    let traces = (trace_range[1] - trace_range[0]).div_ceil(trace_step);
    let samples = (sample_range[1] - sample_range[0]).div_ceil(sample_step);
    let mut amplitudes = vec![0.0_f32; traces * samples];
    let mut occupancy = volume_has_occupancy(reader).then(|| vec![0_u8; traces]);

    for tile in geometry.section_tiles(axis, index) {
        let origin = geometry.tile_origin(tile);
        let effective_shape = geometry.effective_tile_shape(tile);
        let (tile_trace_start, tile_trace_end) = match axis {
            SectionAxis::Inline => (origin[1], origin[1] + effective_shape[1]),
            SectionAxis::Xline => (origin[0], origin[0] + effective_shape[0]),
        };
        if tile_trace_end <= trace_range[0] || tile_trace_start >= trace_range[1] {
            continue;
        }

        let tile_values = reader.read_tile(tile)?;
        let tile_occupancy = reader.read_tile_occupancy(tile)?;
        copy_tile_into_section_tile(
            geometry,
            axis,
            index,
            tile,
            tile_values.as_slice(),
            tile_occupancy.as_ref().map(|value| value.as_slice()),
            trace_range,
            sample_range,
            trace_step,
            sample_step,
            samples,
            &mut amplitudes,
            occupancy.as_deref_mut(),
        );
    }

    let (horizontal_axis, coordinate_value) = match axis {
        SectionAxis::Inline => (
            sampled_axis_f64(&volume.axes.xlines, trace_range, trace_step),
            volume.axes.ilines[index],
        ),
        SectionAxis::Xline => (
            sampled_axis_f64(&volume.axes.ilines, trace_range, trace_step),
            volume.axes.xlines[index],
        ),
    };

    Ok(SectionPlane {
        axis,
        coordinate_index: index,
        coordinate_value,
        traces,
        samples,
        horizontal_axis,
        sample_axis_ms: sampled_axis_f32(&volume.axes.sample_axis_ms, sample_range, sample_step),
        amplitudes,
        occupancy,
    })
}

pub fn section_assembly_plan<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
) -> Result<SectionAssemblyPlan, SeismicStoreError> {
    let volume = reader.volume();
    validate_section_index(volume, axis, index)?;
    validate_tile_window(
        match axis {
            SectionAxis::Inline => volume.shape[1],
            SectionAxis::Xline => volume.shape[0],
        },
        volume.shape[2],
        trace_range,
        sample_range,
    )?;
    let trace_step = lod_step(lod)?;
    let sample_step = lod_step(lod)?;
    let geometry = reader.tile_geometry();
    let source_tiles = geometry
        .section_tiles(axis, index)
        .into_iter()
        .filter(|tile| {
            let origin = geometry.tile_origin(*tile);
            let effective_shape = geometry.effective_tile_shape(*tile);
            let (tile_trace_start, tile_trace_end) = match axis {
                SectionAxis::Inline => (origin[1], origin[1] + effective_shape[1]),
                SectionAxis::Xline => (origin[0], origin[0] + effective_shape[0]),
            };
            !(tile_trace_end <= trace_range[0] || tile_trace_start >= trace_range[1])
        })
        .collect();
    Ok(SectionAssemblyPlan {
        axis,
        section_index: index,
        trace_range,
        sample_range,
        lod,
        source_tiles,
        output_shape: [
            (trace_range[1] - trace_range[0]).div_ceil(trace_step),
            (sample_range[1] - sample_range[0]).div_ceil(sample_step),
        ],
    })
}

pub fn read_assembled_section_artifact<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
) -> Result<AssembledSectionArtifact, SeismicStoreError> {
    let volume = reader.volume();
    let traces = match axis {
        SectionAxis::Inline => volume.shape[1],
        SectionAxis::Xline => volume.shape[0],
    };
    let plane = read_section_plane(reader, axis, index)?;
    Ok(AssembledSectionArtifact {
        domain: SectionDomain {
            axis,
            section_index: index,
        },
        assembly_plan: section_assembly_plan(
            reader,
            axis,
            index,
            [0, traces],
            [0, volume.shape[2]],
            0,
        )?,
        plane,
    })
}

pub fn read_section_window_artifact<R: VolumeStoreReader>(
    reader: &R,
    axis: SectionAxis,
    index: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
) -> Result<SectionWindowArtifact, SeismicStoreError> {
    let plane = read_section_tile_plane(reader, axis, index, trace_range, sample_range, lod)?;
    Ok(SectionWindowArtifact {
        domain: SectionWindowDomain {
            axis,
            section_index: index,
            trace_range,
            sample_range,
            lod,
        },
        assembly_plan: section_assembly_plan(reader, axis, index, trace_range, sample_range, lod)?,
        plane,
    })
}

fn validate_section_index(
    volume: &VolumeMetadata,
    axis: SectionAxis,
    index: usize,
) -> Result<(), SeismicStoreError> {
    let len = match axis {
        SectionAxis::Inline => volume.shape[0],
        SectionAxis::Xline => volume.shape[1],
    };
    if index >= len {
        return Err(SeismicStoreError::InvalidSectionIndex { index, len });
    }
    Ok(())
}

fn volume_has_occupancy<R: VolumeStoreReader>(reader: &R) -> bool {
    let first = TileCoord {
        tile_i: 0,
        tile_x: 0,
    };
    matches!(reader.read_tile_occupancy(first), Ok(Some(_)))
}

fn validate_tile_window(
    total_traces: usize,
    total_samples: usize,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
) -> Result<(), SeismicStoreError> {
    if trace_range[0] >= trace_range[1] || trace_range[1] > total_traces {
        return Err(SeismicStoreError::Message(format!(
            "invalid section tile trace range {:?} for axis length {}",
            trace_range, total_traces
        )));
    }
    if sample_range[0] >= sample_range[1] || sample_range[1] > total_samples {
        return Err(SeismicStoreError::Message(format!(
            "invalid section tile sample range {:?} for sample length {}",
            sample_range, total_samples
        )));
    }
    Ok(())
}

fn lod_step(lod: u8) -> Result<usize, SeismicStoreError> {
    1usize.checked_shl(lod as u32).ok_or_else(|| {
        SeismicStoreError::Message(format!(
            "section tile lod {lod} exceeds the supported stride width"
        ))
    })
}

fn sampled_axis_f64(values: &[f64], range: [usize; 2], step: usize) -> Vec<f64> {
    (range[0]..range[1])
        .step_by(step)
        .map(|index| values[index])
        .collect()
}

fn sampled_axis_f32(values: &[f32], range: [usize; 2], step: usize) -> Vec<f32> {
    (range[0]..range[1])
        .step_by(step)
        .map(|index| values[index])
        .collect()
}

fn copy_tile_into_section(
    geometry: &TileGeometry,
    axis: SectionAxis,
    index: usize,
    tile: TileCoord,
    tile_values: &[f32],
    tile_occupancy: Option<&[u8]>,
    section_amplitudes: &mut [f32],
    mut section_occupancy: Option<&mut [u8]>,
) {
    let tile_shape = geometry.tile_shape();
    let effective_shape = geometry.effective_tile_shape(tile);
    let origin = geometry.tile_origin(tile);
    let samples = tile_shape[2];

    match axis {
        SectionAxis::Inline => {
            let local_i = index - origin[0];
            for local_x in 0..effective_shape[1] {
                let src_trace = ((local_i * tile_shape[1]) + local_x) * samples;
                let dst_trace = origin[1] + local_x;
                let dst_start = dst_trace * samples;
                section_amplitudes[dst_start..dst_start + samples]
                    .copy_from_slice(&tile_values[src_trace..src_trace + samples]);
                if let (Some(tile_mask), Some(section_mask)) =
                    (tile_occupancy, section_occupancy.as_deref_mut())
                {
                    section_mask[dst_trace] = tile_mask[local_i * tile_shape[1] + local_x];
                }
            }
        }
        SectionAxis::Xline => {
            let local_x = index - origin[1];
            for local_i in 0..effective_shape[0] {
                let src_trace = ((local_i * tile_shape[1]) + local_x) * samples;
                let dst_trace = origin[0] + local_i;
                let dst_start = dst_trace * samples;
                section_amplitudes[dst_start..dst_start + samples]
                    .copy_from_slice(&tile_values[src_trace..src_trace + samples]);
                if let (Some(tile_mask), Some(section_mask)) =
                    (tile_occupancy, section_occupancy.as_deref_mut())
                {
                    section_mask[dst_trace] = tile_mask[local_i * tile_shape[1] + local_x];
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_tile_into_section_tile(
    geometry: &TileGeometry,
    axis: SectionAxis,
    index: usize,
    tile: TileCoord,
    tile_values: &[f32],
    tile_occupancy: Option<&[u8]>,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    trace_step: usize,
    sample_step: usize,
    output_samples: usize,
    section_amplitudes: &mut [f32],
    mut section_occupancy: Option<&mut [u8]>,
) {
    let tile_shape = geometry.tile_shape();
    let effective_shape = geometry.effective_tile_shape(tile);
    let origin = geometry.tile_origin(tile);
    let tile_samples = tile_shape[2];

    match axis {
        SectionAxis::Inline => {
            let local_i = index - origin[0];
            for local_x in 0..effective_shape[1] {
                let global_trace = origin[1] + local_x;
                if global_trace < trace_range[0] || global_trace >= trace_range[1] {
                    continue;
                }
                let relative_trace = global_trace - trace_range[0];
                if relative_trace % trace_step != 0 {
                    continue;
                }
                let dst_trace = relative_trace / trace_step;
                let src_trace_index = (local_i * tile_shape[1]) + local_x;
                let src_trace_start = src_trace_index * tile_samples;
                let dst_trace_start = dst_trace * output_samples;

                for (dst_sample, global_sample) in (sample_range[0]..sample_range[1])
                    .step_by(sample_step)
                    .enumerate()
                {
                    section_amplitudes[dst_trace_start + dst_sample] =
                        tile_values[src_trace_start + global_sample];
                }

                if let (Some(tile_mask), Some(section_mask)) =
                    (tile_occupancy, section_occupancy.as_deref_mut())
                {
                    section_mask[dst_trace] = tile_mask[src_trace_index];
                }
            }
        }
        SectionAxis::Xline => {
            let local_x = index - origin[1];
            for local_i in 0..effective_shape[0] {
                let global_trace = origin[0] + local_i;
                if global_trace < trace_range[0] || global_trace >= trace_range[1] {
                    continue;
                }
                let relative_trace = global_trace - trace_range[0];
                if relative_trace % trace_step != 0 {
                    continue;
                }
                let dst_trace = relative_trace / trace_step;
                let src_trace_index = (local_i * tile_shape[1]) + local_x;
                let src_trace_start = src_trace_index * tile_samples;
                let dst_trace_start = dst_trace * output_samples;

                for (dst_sample, global_sample) in (sample_range[0]..sample_range[1])
                    .step_by(sample_step)
                    .enumerate()
                {
                    section_amplitudes[dst_trace_start + dst_sample] =
                        tile_values[src_trace_start + global_sample];
                }

                if let (Some(tile_mask), Some(section_mask)) =
                    (tile_occupancy, section_occupancy.as_deref_mut())
                {
                    section_mask[dst_trace] = tile_mask[src_trace_index];
                }
            }
        }
    }
}
