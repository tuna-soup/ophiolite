use crate::SectionAxis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub tile_i: usize,
    pub tile_x: usize,
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

    pub fn amplitude_offset(&self, coord: TileCoord) -> u64 {
        self.tile_linear_index(coord) as u64 * self.amplitude_tile_bytes
    }

    pub fn occupancy_offset(&self, coord: TileCoord) -> u64 {
        self.tile_linear_index(coord) as u64 * self.occupancy_tile_bytes
    }

    pub fn tile_origin(&self, coord: TileCoord) -> [usize; 2] {
        [
            coord.tile_i * self.tile_shape[0],
            coord.tile_x * self.tile_shape[1],
        ]
    }

    pub fn effective_tile_shape(&self, coord: TileCoord) -> [usize; 3] {
        let origin = self.tile_origin(coord);
        [
            (self.volume_shape[0] - origin[0]).min(self.tile_shape[0]),
            (self.volume_shape[1] - origin[1]).min(self.tile_shape[1]),
            self.volume_shape[2],
        ]
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
}
