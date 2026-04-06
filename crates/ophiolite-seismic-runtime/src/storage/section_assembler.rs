use crate::SectionAxis;
use crate::error::SeismicStoreError;
use crate::metadata::VolumeMetadata;
use crate::store::SectionPlane;

use super::tile_geometry::{TileCoord, TileGeometry};
use super::volume_store::VolumeStoreReader;

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
