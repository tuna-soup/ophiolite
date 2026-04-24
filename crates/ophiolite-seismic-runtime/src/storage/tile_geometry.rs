use std::ops::Range;

use crate::SectionAxis;
use crate::error::SeismicStoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub tile_i: usize,
    pub tile_x: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileExtent {
    pub coord: TileCoord,
    pub origin: [usize; 2],
    pub trace_shape: [usize; 2],
    pub samples: usize,
}

impl TileExtent {
    pub fn shape(&self) -> [usize; 3] {
        [self.trace_shape[0], self.trace_shape[1], self.samples]
    }

    pub fn inline_range(&self) -> Range<usize> {
        self.origin[0]..self.origin[0] + self.trace_shape[0]
    }

    pub fn xline_range(&self) -> Range<usize> {
        self.origin[1]..self.origin[1] + self.trace_shape[1]
    }

    pub fn trace_range(&self, axis: SectionAxis) -> Range<usize> {
        match axis {
            SectionAxis::Inline => self.xline_range(),
            SectionAxis::Xline => self.inline_range(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceMajorTileLayout {
    tile_x_capacity: usize,
    sample_count: usize,
}

impl TraceMajorTileLayout {
    pub fn new(tile_shape: [usize; 3]) -> Self {
        Self {
            tile_x_capacity: tile_shape[1],
            sample_count: tile_shape[2],
        }
    }

    pub fn samples(&self) -> usize {
        self.sample_count
    }

    pub fn trace_index(&self, local_i: usize, local_x: usize) -> usize {
        (local_i * self.tile_x_capacity) + local_x
    }

    pub fn amplitude_trace_offset(&self, local_i: usize, local_x: usize) -> usize {
        self.trace_index(local_i, local_x) * self.sample_count
    }

    pub fn amplitude_index(&self, local_i: usize, local_x: usize, sample: usize) -> usize {
        self.amplitude_trace_offset(local_i, local_x) + sample
    }

    pub fn occupancy_index(&self, local_i: usize, local_x: usize) -> usize {
        self.trace_index(local_i, local_x)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionTileIntersection {
    pub tile: TileCoord,
    pub section_local_offset: usize,
    pub section_trace_range: [usize; 2],
}

impl SectionTileIntersection {
    pub fn trace_len(&self) -> usize {
        self.section_trace_range[1] - self.section_trace_range[0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionWindowIntersection {
    pub section_local_offset: usize,
    pub tile_trace_range: [usize; 2],
    pub section_trace_range: [usize; 2],
    pub window_trace_range: [usize; 2],
}

impl SectionWindowIntersection {
    pub fn trace_len(&self) -> usize {
        self.section_trace_range[1] - self.section_trace_range[0]
    }
}

#[derive(Debug, Clone)]
pub struct TileGeometry {
    volume_shape: [usize; 3],
    tile_shape: [usize; 3],
    tile_grid_shape: [usize; 2],
    amplitude_tile_len: usize,
    amplitude_tile_bytes: u64,
    occupancy_tile_len: usize,
    occupancy_tile_bytes: u64,
}

impl TileGeometry {
    pub fn new(volume_shape: [usize; 3], tile_shape: [usize; 3]) -> Self {
        let tile_shape = [
            tile_shape[0].max(1).min(volume_shape[0].max(1)),
            tile_shape[1].max(1).min(volume_shape[1].max(1)),
            tile_shape[2].max(1).min(volume_shape[2].max(1)),
        ];
        let tile_grid_shape = [
            volume_shape[0].div_ceil(tile_shape[0]),
            volume_shape[1].div_ceil(tile_shape[1]),
        ];
        let amplitude_tile_len = tile_shape[0] * tile_shape[1] * tile_shape[2];
        let occupancy_tile_len = tile_shape[0] * tile_shape[1];

        Self {
            volume_shape,
            tile_shape,
            tile_grid_shape,
            amplitude_tile_len,
            amplitude_tile_bytes: (amplitude_tile_len * std::mem::size_of::<f32>()) as u64,
            occupancy_tile_len,
            occupancy_tile_bytes: occupancy_tile_len as u64,
        }
    }

    pub fn volume_shape(&self) -> [usize; 3] {
        self.volume_shape
    }

    pub fn tile_shape(&self) -> [usize; 3] {
        self.tile_shape
    }

    pub fn tile_grid_shape(&self) -> [usize; 2] {
        self.tile_grid_shape
    }

    pub fn amplitude_tile_len(&self) -> usize {
        self.amplitude_tile_len
    }

    pub fn amplitude_tile_bytes(&self) -> u64 {
        self.amplitude_tile_bytes
    }

    pub fn occupancy_tile_len(&self) -> usize {
        self.occupancy_tile_len
    }

    pub fn occupancy_tile_bytes(&self) -> u64 {
        self.occupancy_tile_bytes
    }

    pub fn tile_count(&self) -> usize {
        self.tile_grid_shape[0] * self.tile_grid_shape[1]
    }

    pub fn tile_linear_index(&self, coord: TileCoord) -> usize {
        coord.tile_i * self.tile_grid_shape[1] + coord.tile_x
    }

    pub fn amplitude_byte_range(&self, coord: TileCoord) -> Range<usize> {
        let start = self.amplitude_offset(coord) as usize;
        start..start + self.amplitude_tile_bytes as usize
    }

    pub fn amplitude_offset(&self, coord: TileCoord) -> u64 {
        self.tile_linear_index(coord) as u64 * self.amplitude_tile_bytes
    }

    pub fn occupancy_byte_range(&self, coord: TileCoord) -> Range<usize> {
        let start = self.occupancy_offset(coord) as usize;
        start..start + self.occupancy_tile_bytes as usize
    }

    pub fn occupancy_offset(&self, coord: TileCoord) -> u64 {
        self.tile_linear_index(coord) as u64 * self.occupancy_tile_bytes
    }

    pub fn trace_major_layout(&self) -> TraceMajorTileLayout {
        TraceMajorTileLayout::new(self.tile_shape)
    }

    pub fn tile_extent(&self, coord: TileCoord) -> TileExtent {
        let origin = [
            coord.tile_i * self.tile_shape[0],
            coord.tile_x * self.tile_shape[1],
        ];
        TileExtent {
            coord,
            origin,
            trace_shape: [
                self.volume_shape[0]
                    .saturating_sub(origin[0])
                    .min(self.tile_shape[0]),
                self.volume_shape[1]
                    .saturating_sub(origin[1])
                    .min(self.tile_shape[1]),
            ],
            samples: self.volume_shape[2],
        }
    }

    pub fn tile_origin(&self, coord: TileCoord) -> [usize; 2] {
        self.tile_extent(coord).origin
    }

    pub fn effective_tile_shape(&self, coord: TileCoord) -> [usize; 3] {
        self.tile_extent(coord).shape()
    }

    pub fn section_tiles(&self, axis: SectionAxis, index: usize) -> Vec<TileCoord> {
        match axis {
            SectionAxis::Inline => {
                let tile_i = index / self.tile_shape[0];
                (0..self.tile_grid_shape[1])
                    .map(|tile_x| TileCoord { tile_i, tile_x })
                    .collect()
            }
            SectionAxis::Xline => {
                let tile_x = index / self.tile_shape[1];
                (0..self.tile_grid_shape[0])
                    .map(|tile_i| TileCoord { tile_i, tile_x })
                    .collect()
            }
        }
    }

    pub fn iter_tiles(&self) -> impl Iterator<Item = TileCoord> + '_ {
        (0..self.tile_grid_shape[0]).flat_map(|tile_i| {
            (0..self.tile_grid_shape[1]).map(move |tile_x| TileCoord { tile_i, tile_x })
        })
    }

    pub fn section_intersection(
        &self,
        axis: SectionAxis,
        index: usize,
        tile: TileCoord,
    ) -> Option<SectionTileIntersection> {
        let extent = self.tile_extent(tile);
        match axis {
            SectionAxis::Inline => {
                extent
                    .inline_range()
                    .contains(&index)
                    .then_some(SectionTileIntersection {
                        tile,
                        section_local_offset: index - extent.origin[0],
                        section_trace_range: [
                            extent.origin[1],
                            extent.origin[1] + extent.trace_shape[1],
                        ],
                    })
            }
            SectionAxis::Xline => {
                extent
                    .xline_range()
                    .contains(&index)
                    .then_some(SectionTileIntersection {
                        tile,
                        section_local_offset: index - extent.origin[1],
                        section_trace_range: [
                            extent.origin[0],
                            extent.origin[0] + extent.trace_shape[0],
                        ],
                    })
            }
        }
    }

    pub fn section_window_intersection(
        &self,
        axis: SectionAxis,
        index: usize,
        tile: TileCoord,
        trace_range: [usize; 2],
    ) -> Option<SectionWindowIntersection> {
        let intersection = self.section_intersection(axis, index, tile)?;
        let section_trace_start = intersection.section_trace_range[0].max(trace_range[0]);
        let section_trace_end = intersection.section_trace_range[1].min(trace_range[1]);
        (section_trace_start < section_trace_end).then_some(SectionWindowIntersection {
            section_local_offset: intersection.section_local_offset,
            tile_trace_range: [
                section_trace_start - intersection.section_trace_range[0],
                section_trace_end - intersection.section_trace_range[0],
            ],
            section_trace_range: [section_trace_start, section_trace_end],
            window_trace_range: [
                section_trace_start - trace_range[0],
                section_trace_end - trace_range[0],
            ],
        })
    }
}

pub fn section_lod_step(lod: u8) -> Result<usize, SeismicStoreError> {
    1usize.checked_shl(lod as u32).ok_or_else(|| {
        SeismicStoreError::Message(format!(
            "section tile lod {lod} exceeds the supported stride width"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_major_layout_indexes_padded_tiles() {
        let geometry = TileGeometry::new([5, 4, 3], [2, 3, 3]);
        let extent = geometry.tile_extent(TileCoord {
            tile_i: 2,
            tile_x: 1,
        });
        let layout = geometry.trace_major_layout();

        assert_eq!(extent.origin, [4, 3]);
        assert_eq!(extent.trace_shape, [1, 1]);
        assert_eq!(extent.samples, 3);
        assert_eq!(layout.trace_index(1, 2), 5);
        assert_eq!(layout.amplitude_trace_offset(1, 2), 15);
        assert_eq!(layout.amplitude_index(1, 2, 1), 16);
        assert_eq!(layout.occupancy_index(1, 2), 5);
    }

    #[test]
    fn section_window_intersection_clips_edge_tiles() {
        let geometry = TileGeometry::new([5, 6, 8], [2, 4, 8]);
        let clipped = geometry
            .section_window_intersection(
                SectionAxis::Inline,
                3,
                TileCoord {
                    tile_i: 1,
                    tile_x: 1,
                },
                [5, 6],
            )
            .expect("inline section should intersect edge tile");

        assert_eq!(clipped.section_local_offset, 1);
        assert_eq!(clipped.tile_trace_range, [1, 2]);
        assert_eq!(clipped.section_trace_range, [5, 6]);
        assert_eq!(clipped.window_trace_range, [0, 1]);
    }

    #[test]
    fn section_lod_step_rejects_overflow() {
        assert_eq!(section_lod_step(0).expect("lod 0"), 1);
        assert!(section_lod_step(usize::BITS as u8).is_err());
    }
}
