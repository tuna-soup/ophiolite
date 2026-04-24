use crate::SectionAxis;
use crate::error::SeismicStoreError;
use crate::execution::{SectionDomain, SectionWindowDomain};
use crate::metadata::VolumeMetadata;
use crate::store::SectionPlane;

use super::tile_geometry::{
    SectionTileIntersection, SectionWindowIntersection, TileCoord, TraceMajorTileLayout,
    section_lod_step,
};
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
    let layout = geometry.trace_major_layout();
    let mut amplitudes = vec![0.0_f32; traces * volume.shape[2]];
    let mut occupancy = volume_has_occupancy(reader).then(|| vec![0_u8; traces]);

    for tile in geometry.section_tiles(axis, index) {
        let Some(intersection) = geometry.section_intersection(axis, index, tile) else {
            continue;
        };
        let tile_values = reader.read_tile(tile)?;
        let tile_occupancy = reader.read_tile_occupancy(tile)?;
        copy_tile_into_section(
            axis,
            &layout,
            intersection,
            tile_values.as_slice(),
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

    let trace_step = section_lod_step(lod)?;
    let sample_step = section_lod_step(lod)?;
    let layout = geometry.trace_major_layout();
    let traces = (trace_range[1] - trace_range[0]).div_ceil(trace_step);
    let samples = (sample_range[1] - sample_range[0]).div_ceil(sample_step);
    let mut amplitudes = vec![0.0_f32; traces * samples];
    let mut occupancy = volume_has_occupancy(reader).then(|| vec![0_u8; traces]);

    for tile in geometry.section_tiles(axis, index) {
        let Some(intersection) =
            geometry.section_window_intersection(axis, index, tile, trace_range)
        else {
            continue;
        };

        let tile_values = reader.read_tile(tile)?;
        let tile_occupancy = reader.read_tile_occupancy(tile)?;
        copy_tile_into_section_tile(
            axis,
            &layout,
            intersection,
            tile_values.as_slice(),
            tile_occupancy.as_ref().map(|value| value.as_slice()),
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
    let trace_step = section_lod_step(lod)?;
    let sample_step = section_lod_step(lod)?;
    let geometry = reader.tile_geometry();
    let source_tiles = geometry
        .section_tiles(axis, index)
        .into_iter()
        .filter(|tile| {
            geometry
                .section_window_intersection(axis, index, *tile, trace_range)
                .is_some()
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
    axis: SectionAxis,
    layout: &TraceMajorTileLayout,
    intersection: SectionTileIntersection,
    tile_values: &[f32],
    tile_occupancy: Option<&[u8]>,
    section_amplitudes: &mut [f32],
    mut section_occupancy: Option<&mut [u8]>,
) {
    let samples = layout.samples();

    match axis {
        SectionAxis::Inline => {
            let local_i = intersection.section_local_offset;
            for local_x in 0..intersection.trace_len() {
                let src_trace = layout.amplitude_trace_offset(local_i, local_x);
                let dst_trace = intersection.section_trace_range[0] + local_x;
                let dst_start = dst_trace * samples;
                section_amplitudes[dst_start..dst_start + samples]
                    .copy_from_slice(&tile_values[src_trace..src_trace + samples]);
                if let (Some(tile_mask), Some(section_mask)) =
                    (tile_occupancy, section_occupancy.as_deref_mut())
                {
                    section_mask[dst_trace] = tile_mask[layout.occupancy_index(local_i, local_x)];
                }
            }
        }
        SectionAxis::Xline => {
            let local_x = intersection.section_local_offset;
            for local_i in 0..intersection.trace_len() {
                let src_trace = layout.amplitude_trace_offset(local_i, local_x);
                let dst_trace = intersection.section_trace_range[0] + local_i;
                let dst_start = dst_trace * samples;
                section_amplitudes[dst_start..dst_start + samples]
                    .copy_from_slice(&tile_values[src_trace..src_trace + samples]);
                if let (Some(tile_mask), Some(section_mask)) =
                    (tile_occupancy, section_occupancy.as_deref_mut())
                {
                    section_mask[dst_trace] = tile_mask[layout.occupancy_index(local_i, local_x)];
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_tile_into_section_tile(
    axis: SectionAxis,
    layout: &TraceMajorTileLayout,
    intersection: SectionWindowIntersection,
    tile_values: &[f32],
    tile_occupancy: Option<&[u8]>,
    sample_range: [usize; 2],
    trace_step: usize,
    sample_step: usize,
    output_samples: usize,
    section_amplitudes: &mut [f32],
    mut section_occupancy: Option<&mut [u8]>,
) {
    let tile_samples = layout.samples();

    match axis {
        SectionAxis::Inline => {
            let local_i = intersection.section_local_offset;
            for offset in 0..intersection.trace_len() {
                let local_x = intersection.tile_trace_range[0] + offset;
                let relative_trace = intersection.window_trace_range[0] + offset;
                if relative_trace % trace_step != 0 {
                    continue;
                }
                let dst_trace = relative_trace / trace_step;
                let src_trace_index = layout.trace_index(local_i, local_x);
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
            let local_x = intersection.section_local_offset;
            for offset in 0..intersection.trace_len() {
                let local_i = intersection.tile_trace_range[0] + offset;
                let relative_trace = intersection.window_trace_range[0] + offset;
                if relative_trace % trace_step != 0 {
                    continue;
                }
                let dst_trace = relative_trace / trace_step;
                let src_trace_index = layout.trace_index(local_i, local_x);
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ndarray::{Array2, Array3};

    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        generate_store_id,
    };
    use crate::storage::tile_geometry::TileGeometry;
    use crate::storage::volume_store::{OccupancyTile, TileBuffer};

    use super::*;

    struct MockReader {
        volume: VolumeMetadata,
        geometry: TileGeometry,
        data: Array3<f32>,
        occupancy: Option<Array2<u8>>,
    }

    impl MockReader {
        fn new(shape: [usize; 3], tile_shape: [usize; 3]) -> Self {
            let mut data = Array3::<f32>::zeros((shape[0], shape[1], shape[2]));
            let mut occupancy = Array2::<u8>::from_elem((shape[0], shape[1]), 1);
            for iline in 0..shape[0] {
                for xline in 0..shape[1] {
                    for sample in 0..shape[2] {
                        data[[iline, xline, sample]] =
                            (iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32;
                    }
                }
            }
            occupancy[[1, 4]] = 0;
            Self {
                volume: test_volume_metadata(shape),
                geometry: TileGeometry::new(shape, tile_shape),
                data,
                occupancy: Some(occupancy),
            }
        }
    }

    impl VolumeStoreReader for MockReader {
        fn volume(&self) -> &VolumeMetadata {
            &self.volume
        }

        fn tile_geometry(&self) -> &TileGeometry {
            &self.geometry
        }

        fn read_tile<'a>(&'a self, tile: TileCoord) -> Result<TileBuffer<'a>, SeismicStoreError> {
            let extent = self.geometry.tile_extent(tile);
            let layout = self.geometry.trace_major_layout();
            let mut amplitudes = vec![0.0_f32; self.geometry.amplitude_tile_len()];
            for local_i in 0..extent.trace_shape[0] {
                for local_x in 0..extent.trace_shape[1] {
                    for sample in 0..extent.samples {
                        amplitudes[layout.amplitude_index(local_i, local_x, sample)] = self.data[[
                            extent.origin[0] + local_i,
                            extent.origin[1] + local_x,
                            sample,
                        ]];
                    }
                }
            }
            Ok(TileBuffer::owned(amplitudes))
        }

        fn read_tile_occupancy<'a>(
            &'a self,
            tile: TileCoord,
        ) -> Result<Option<OccupancyTile<'a>>, SeismicStoreError> {
            let Some(mask) = &self.occupancy else {
                return Ok(None);
            };
            let extent = self.geometry.tile_extent(tile);
            let layout = self.geometry.trace_major_layout();
            let mut occupancy = vec![0_u8; self.geometry.occupancy_tile_len()];
            for local_i in 0..extent.trace_shape[0] {
                for local_x in 0..extent.trace_shape[1] {
                    occupancy[layout.occupancy_index(local_i, local_x)] =
                        mask[[extent.origin[0] + local_i, extent.origin[1] + local_x]];
                }
            }
            Ok(Some(OccupancyTile::owned(occupancy)))
        }
    }

    #[test]
    fn read_section_tile_plane_clips_edge_tiles_with_lod() {
        let reader = MockReader::new([3, 5, 4], [2, 3, 4]);

        let plane = read_section_tile_plane(&reader, SectionAxis::Inline, 1, [2, 5], [1, 4], 1)
            .expect("assemble inline window");

        assert_eq!(plane.traces, 2);
        assert_eq!(plane.samples, 2);
        assert_eq!(plane.horizontal_axis, vec![2.0, 4.0]);
        assert_eq!(plane.sample_axis_ms, vec![2.0, 6.0]);
        assert_eq!(plane.amplitudes, vec![121.0, 123.0, 141.0, 143.0]);
        assert_eq!(plane.occupancy, Some(vec![1, 0]));
    }

    #[test]
    fn section_assembly_plan_only_lists_intersecting_tiles() {
        let reader = MockReader::new([3, 5, 4], [2, 3, 4]);

        let plan = section_assembly_plan(&reader, SectionAxis::Inline, 1, [2, 5], [0, 4], 0)
            .expect("build assembly plan");

        assert_eq!(
            plan.source_tiles,
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
        assert_eq!(plan.output_shape, [3, 4]);
    }

    fn test_volume_metadata(shape: [usize; 3]) -> VolumeMetadata {
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://section-assembler-test"),
                file_size: 0,
                trace_count: (shape[0] * shape[1]) as u64,
                samples_per_trace: shape[2],
                sample_interval_us: 2000,
                sample_format_code: 5,
                sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(5),
                endianness: "big".to_string(),
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
            created_by: "section-assembler-test".to_string(),
            processing_lineage: None,
        }
    }
}
