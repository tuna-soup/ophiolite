use crate::error::SeismicStoreError;
use crate::metadata::VolumeMetadata;
use ndarray::{Array2, Array3};

use super::tile_geometry::{TileCoord, TileGeometry};

pub enum TileBuffer<'a> {
    Borrowed(&'a [f32]),
    Owned(Vec<f32>),
}

impl<'a> TileBuffer<'a> {
    pub fn as_slice(&self) -> &[f32] {
        match self {
            Self::Borrowed(values) => values,
            Self::Owned(values) => values,
        }
    }

    pub fn into_owned(self) -> Vec<f32> {
        match self {
            Self::Borrowed(values) => values.to_vec(),
            Self::Owned(values) => values,
        }
    }
}

pub enum OccupancyTile<'a> {
    Borrowed(&'a [u8]),
    Owned(Vec<u8>),
}

impl<'a> OccupancyTile<'a> {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Borrowed(values) => values,
            Self::Owned(values) => values,
        }
    }

    pub fn into_owned(self) -> Vec<u8> {
        match self {
            Self::Borrowed(values) => values.to_vec(),
            Self::Owned(values) => values,
        }
    }
}

pub trait VolumeStoreReader {
    fn volume(&self) -> &VolumeMetadata;
    fn tile_geometry(&self) -> &TileGeometry;
    fn read_tile<'a>(&'a self, tile: TileCoord) -> Result<TileBuffer<'a>, SeismicStoreError>;
    fn read_tile_occupancy<'a>(
        &'a self,
        tile: TileCoord,
    ) -> Result<Option<OccupancyTile<'a>>, SeismicStoreError>;
}

pub trait VolumeStoreWriter {
    fn volume(&self) -> &VolumeMetadata;
    fn tile_geometry(&self) -> &TileGeometry;
    fn write_tile(&self, tile: TileCoord, amplitudes: &[f32]) -> Result<(), SeismicStoreError>;
    fn write_tile_occupancy(
        &self,
        tile: TileCoord,
        occupancy: &[u8],
    ) -> Result<(), SeismicStoreError>;
    fn finalize(self) -> Result<(), SeismicStoreError>
    where
        Self: Sized;
}

pub fn write_dense_volume<W: VolumeStoreWriter>(
    writer: &W,
    data: &Array3<f32>,
    occupancy: Option<&Array2<u8>>,
) -> Result<(), SeismicStoreError> {
    let geometry = writer.tile_geometry().clone();
    let tile_shape = geometry.tile_shape();
    for tile in geometry.iter_tiles() {
        let effective = geometry.effective_tile_shape(tile);
        let origin = geometry.tile_origin(tile);
        let mut amplitudes = vec![0.0_f32; geometry.amplitude_tile_len()];
        for local_i in 0..effective[0] {
            for local_x in 0..effective[1] {
                let dst = ((local_i * tile_shape[1]) + local_x) * tile_shape[2];
                for sample in 0..effective[2] {
                    amplitudes[dst + sample] =
                        data[[origin[0] + local_i, origin[1] + local_x, sample]];
                }
            }
        }
        writer.write_tile(tile, &amplitudes)?;

        if let Some(mask) = occupancy {
            let mut occupancy_tile = vec![0_u8; geometry.occupancy_tile_len()];
            for local_i in 0..effective[0] {
                for local_x in 0..effective[1] {
                    let dst = local_i * tile_shape[1] + local_x;
                    occupancy_tile[dst] = mask[[origin[0] + local_i, origin[1] + local_x]];
                }
            }
            writer.write_tile_occupancy(tile, &occupancy_tile)?;
        }
    }

    Ok(())
}

pub fn read_dense_volume<R: VolumeStoreReader>(
    reader: &R,
) -> Result<Array3<f32>, SeismicStoreError> {
    let shape = reader.volume().shape;
    let mut data = Array3::<f32>::zeros((shape[0], shape[1], shape[2]));
    let tile_shape = reader.tile_geometry().tile_shape();

    for tile in reader.tile_geometry().iter_tiles() {
        let tile_data = reader.read_tile(tile)?;
        let effective = reader.tile_geometry().effective_tile_shape(tile);
        let origin = reader.tile_geometry().tile_origin(tile);
        let tile_samples = tile_data.as_slice();
        for local_i in 0..effective[0] {
            for local_x in 0..effective[1] {
                let src = ((local_i * tile_shape[1]) + local_x) * tile_shape[2];
                for sample in 0..effective[2] {
                    data[[origin[0] + local_i, origin[1] + local_x, sample]] =
                        tile_samples[src + sample];
                }
            }
        }
    }

    Ok(data)
}

pub fn read_dense_occupancy<R: VolumeStoreReader>(
    reader: &R,
) -> Result<Option<Array2<u8>>, SeismicStoreError> {
    let shape = reader.volume().shape;
    let mut occupancy = Array2::<u8>::zeros((shape[0], shape[1]));
    let tile_shape = reader.tile_geometry().tile_shape();
    let mut has_any = false;

    for tile in reader.tile_geometry().iter_tiles() {
        let Some(tile_occupancy) = reader.read_tile_occupancy(tile)? else {
            continue;
        };
        has_any = true;
        let effective = reader.tile_geometry().effective_tile_shape(tile);
        let origin = reader.tile_geometry().tile_origin(tile);
        let tile_values = tile_occupancy.as_slice();
        for local_i in 0..effective[0] {
            for local_x in 0..effective[1] {
                let src = local_i * tile_shape[1] + local_x;
                occupancy[[origin[0] + local_i, origin[1] + local_x]] = tile_values[src];
            }
        }
    }

    Ok(has_any.then_some(occupancy))
}
