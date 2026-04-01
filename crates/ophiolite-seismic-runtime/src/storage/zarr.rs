use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ndarray::Array2;
use zarrs::array::codec::{BloscCodec, ZstdCodec};
use zarrs::array::{Array, ArrayBuilder, ArraySubset, BytesToBytesCodecTraits, data_type};
use zarrs::filesystem::FilesystemStore;
use zarrs::group::GroupBuilder;
use zarrs::metadata_ext::codec::blosc::{
    BloscCompressionLevel, BloscCompressor, BloscShuffleMode,
};
use zarrs::storage::{
    ReadableWritableListableStorage, ReadableWritableListableStorageTraits,
};

use crate::error::SeismicStoreError;
use crate::metadata::{CompressionKind, StorageLayout, StoreManifest, VolumeMetadata};

use super::tile_geometry::{TileCoord, TileGeometry};
use super::volume_store::{OccupancyTile, TileBuffer, VolumeStoreReader, VolumeStoreWriter};

const ARRAY_PATH: &str = "/amplitude";
const OCCUPANCY_PATH: &str = "/occupancy";

pub struct ZarrVolumeStoreReader {
    _root: PathBuf,
    volume: VolumeMetadata,
    geometry: TileGeometry,
    array: Array<dyn ReadableWritableListableStorageTraits>,
    occupancy: Option<Array<dyn ReadableWritableListableStorageTraits>>,
}

impl ZarrVolumeStoreReader {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SeismicStoreError> {
        let root = root.as_ref().to_path_buf();
        let manifest: StoreManifest =
            serde_json::from_slice(&fs::read(root.join(StoreManifest::FILE_NAME))?)?;
        let geometry = TileGeometry::new(manifest.shape, manifest.chunk_shape);
        let array = open_array_at_path(&root, &manifest.array_path)?;
        let occupancy = manifest
            .occupancy_array_path
            .as_deref()
            .map(|path| open_array_at_path(&root, path))
            .transpose()?;
        Ok(Self {
            _root: root,
            volume: VolumeMetadata::from(&manifest),
            geometry,
            array,
            occupancy,
        })
    }
}

impl VolumeStoreReader for ZarrVolumeStoreReader {
    fn volume(&self) -> &VolumeMetadata {
        &self.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn read_tile<'a>(&'a self, tile: TileCoord) -> Result<TileBuffer<'a>, SeismicStoreError> {
        let subset = amplitude_subset(&self.geometry, tile);
        let effective = self.geometry.effective_tile_shape(tile);
        let raw = self.array.retrieve_array_subset::<Vec<f32>>(&subset)?;
        Ok(TileBuffer::Owned(pad_amplitude_tile(
            &self.geometry,
            effective,
            &raw,
        )))
    }

    fn read_tile_occupancy<'a>(
        &'a self,
        tile: TileCoord,
    ) -> Result<Option<OccupancyTile<'a>>, SeismicStoreError> {
        let Some(array) = &self.occupancy else {
            return Ok(None);
        };
        let subset = occupancy_subset(&self.geometry, tile);
        let effective = self.geometry.effective_tile_shape(tile);
        let raw = array.retrieve_array_subset::<Vec<u8>>(&subset)?;
        Ok(Some(OccupancyTile::Owned(pad_occupancy_tile(
            &self.geometry,
            effective,
            &raw,
        ))))
    }
}

pub struct ZarrVolumeStoreWriter {
    root: PathBuf,
    volume: VolumeMetadata,
    geometry: TileGeometry,
    array: Array<dyn ReadableWritableListableStorageTraits>,
    occupancy: Option<Array<dyn ReadableWritableListableStorageTraits>>,
}

impl ZarrVolumeStoreWriter {
    pub fn create(
        root: impl AsRef<Path>,
        volume: VolumeMetadata,
        tile_shape: [usize; 3],
        storage_layout: StorageLayout,
        has_occupancy: bool,
    ) -> Result<Self, SeismicStoreError> {
        let root = root.as_ref().to_path_buf();
        let geometry = TileGeometry::new(volume.shape, tile_shape);
        let manifest = StoreManifest {
            version: 1,
            kind: volume.kind.clone(),
            source: volume.source.clone(),
            shape: volume.shape,
            chunk_shape: tile_shape,
            axes: volume.axes.clone(),
            array_path: ARRAY_PATH.to_string(),
            occupancy_array_path: has_occupancy.then(|| OCCUPANCY_PATH.to_string()),
            created_by: volume.created_by.clone(),
            derived_from: None,
            processing_lineage: volume.processing_lineage.clone(),
            storage_layout: Some(storage_layout),
        };
        let occupancy_seed =
            has_occupancy.then(|| Array2::<u8>::zeros((volume.shape[0], volume.shape[1])));
        create_empty_store(&root, &manifest, occupancy_seed.as_ref())?;
        let array = open_array_at_path(&root, &manifest.array_path)?;
        let occupancy = manifest
            .occupancy_array_path
            .as_deref()
            .map(|path| open_array_at_path(&root, path))
            .transpose()?;

        Ok(Self {
            root,
            volume,
            geometry,
            array,
            occupancy,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl VolumeStoreWriter for ZarrVolumeStoreWriter {
    fn volume(&self) -> &VolumeMetadata {
        &self.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn write_tile(&self, tile: TileCoord, amplitudes: &[f32]) -> Result<(), SeismicStoreError> {
        let effective = self.geometry.effective_tile_shape(tile);
        let mut raw = Vec::with_capacity(effective[0] * effective[1] * effective[2]);
        let tile_shape = self.geometry.tile_shape();
        for local_i in 0..effective[0] {
            for local_x in 0..effective[1] {
                let src = ((local_i * tile_shape[1]) + local_x) * tile_shape[2];
                raw.extend_from_slice(&amplitudes[src..src + effective[2]]);
            }
        }
        self.array
            .store_array_subset(&amplitude_subset(&self.geometry, tile), &raw)?;
        Ok(())
    }

    fn write_tile_occupancy(
        &self,
        tile: TileCoord,
        occupancy: &[u8],
    ) -> Result<(), SeismicStoreError> {
        let Some(array) = &self.occupancy else {
            return Ok(());
        };
        let effective = self.geometry.effective_tile_shape(tile);
        let mut raw = Vec::with_capacity(effective[0] * effective[1]);
        let tile_shape = self.geometry.tile_shape();
        for local_i in 0..effective[0] {
            let src = local_i * tile_shape[1];
            raw.extend_from_slice(&occupancy[src..src + effective[1]]);
        }
        array.store_array_subset(&occupancy_subset(&self.geometry, tile), &raw)?;
        Ok(())
    }

    fn finalize(self) -> Result<(), SeismicStoreError> {
        Ok(())
    }
}

fn create_empty_store(
    root: &Path,
    manifest: &StoreManifest,
    occupancy: Option<&Array2<u8>>,
) -> Result<(), SeismicStoreError> {
    if root.exists() {
        return Err(SeismicStoreError::StoreAlreadyExists(root.to_path_buf()));
    }

    fs::create_dir_all(root)?;
    let store: ReadableWritableListableStorage = Arc::new(
        FilesystemStore::new(root).map_err(|error| SeismicStoreError::Message(error.to_string()))?,
    );
    GroupBuilder::new()
        .attributes(
            serde_json::json!({
                "producer": "ophiolite-seismic-runtime",
                "manifest": StoreManifest::FILE_NAME,
            })
            .as_object()
            .expect("object literal")
            .clone(),
        )
        .build(store.clone(), "/")
        .map_err(|error| SeismicStoreError::Message(error.to_string()))?
        .store_metadata()?;

    let array = build_amplitude_array(&store, ARRAY_PATH, manifest)?;
    array.store_metadata()?;

    if let Some(occupancy) = occupancy {
        let occupancy_array = ArrayBuilder::new(
            vec![manifest.shape[0] as u64, manifest.shape[1] as u64],
            vec![manifest.chunk_shape[0] as u64, manifest.chunk_shape[1] as u64],
            data_type::uint8(),
            0_u8,
        )
        .dimension_names(["iline", "xline"].into())
        .build(store, OCCUPANCY_PATH)
        .map_err(|error| SeismicStoreError::Message(error.to_string()))?;
        occupancy_array.store_metadata()?;
        occupancy_array.store_array_subset(
            &ArraySubset::new_with_ranges(&[
                0_u64..manifest.shape[0] as u64,
                0_u64..manifest.shape[1] as u64,
            ]),
            occupancy.to_owned(),
        )?;
    }

    fs::write(
        root.join(StoreManifest::FILE_NAME),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn open_array_at_path(
    root: &Path,
    path: &str,
) -> Result<Array<dyn ReadableWritableListableStorageTraits>, SeismicStoreError> {
    let store: ReadableWritableListableStorage = Arc::new(
        FilesystemStore::new(root).map_err(|error| SeismicStoreError::Message(error.to_string()))?,
    );
    Array::open(store, path).map_err(|error| SeismicStoreError::Message(error.to_string()))
}

fn build_amplitude_array(
    store: &ReadableWritableListableStorage,
    path: &str,
    manifest: &StoreManifest,
) -> Result<Array<dyn ReadableWritableListableStorageTraits>, SeismicStoreError> {
    let layout = manifest.storage_layout.clone().unwrap_or_default();
    let mut builder = ArrayBuilder::new(
        manifest
            .shape
            .iter()
            .map(|value| *value as u64)
            .collect::<Vec<_>>(),
        effective_array_chunk_shape(manifest, &layout),
        data_type::float32(),
        0.0f32,
    );
    builder.dimension_names(Some(["iline", "xline", "sample"]));

    if let Some(codec) = compression_codec(&layout.compression)? {
        builder.bytes_to_bytes_codecs(vec![codec]);
    }
    if let Some(shard_shape) = layout.shard_shape
        && shard_shape != manifest.chunk_shape
    {
        builder.subchunk_shape(
            manifest
                .chunk_shape
                .iter()
                .map(|value| *value as u64)
                .collect::<Vec<_>>(),
        );
    }

    builder
        .build(store.clone(), path)
        .map_err(|error| SeismicStoreError::Message(error.to_string()))
}

fn effective_array_chunk_shape(manifest: &StoreManifest, layout: &StorageLayout) -> Vec<u64> {
    layout
        .shard_shape
        .unwrap_or(manifest.chunk_shape)
        .iter()
        .map(|value| *value as u64)
        .collect()
}

fn compression_codec(
    compression: &CompressionKind,
) -> Result<Option<Arc<dyn BytesToBytesCodecTraits>>, SeismicStoreError> {
    let codec: Option<Arc<dyn BytesToBytesCodecTraits>> = match compression {
        CompressionKind::None => None,
        CompressionKind::BloscLz4 => Some(Arc::new(
            BloscCodec::new(
                BloscCompressor::LZ4,
                BloscCompressionLevel::try_from(5)
                    .map_err(|error| SeismicStoreError::Message(error.to_string()))?,
                None,
                BloscShuffleMode::BitShuffle,
                Some(2),
            )
            .map_err(|error| SeismicStoreError::Message(error.to_string()))?,
        )),
        CompressionKind::Zstd => Some(Arc::new(ZstdCodec::new(3, false))),
    };
    Ok(codec)
}

fn amplitude_subset(geometry: &TileGeometry, tile: TileCoord) -> ArraySubset {
    let origin = geometry.tile_origin(tile);
    let effective = geometry.effective_tile_shape(tile);
    ArraySubset::new_with_ranges(&[
        origin[0] as u64..(origin[0] + effective[0]) as u64,
        origin[1] as u64..(origin[1] + effective[1]) as u64,
        0..effective[2] as u64,
    ])
}

fn occupancy_subset(geometry: &TileGeometry, tile: TileCoord) -> ArraySubset {
    let origin = geometry.tile_origin(tile);
    let effective = geometry.effective_tile_shape(tile);
    ArraySubset::new_with_ranges(&[
        origin[0] as u64..(origin[0] + effective[0]) as u64,
        origin[1] as u64..(origin[1] + effective[1]) as u64,
    ])
}

fn pad_amplitude_tile(geometry: &TileGeometry, effective: [usize; 3], raw: &[f32]) -> Vec<f32> {
    let tile_shape = geometry.tile_shape();
    let mut out = vec![0.0_f32; geometry.amplitude_tile_len()];
    for local_i in 0..effective[0] {
        for local_x in 0..effective[1] {
            let src = ((local_i * effective[1]) + local_x) * effective[2];
            let dst = ((local_i * tile_shape[1]) + local_x) * tile_shape[2];
            out[dst..dst + effective[2]].copy_from_slice(&raw[src..src + effective[2]]);
        }
    }
    out
}

fn pad_occupancy_tile(geometry: &TileGeometry, effective: [usize; 3], raw: &[u8]) -> Vec<u8> {
    let tile_shape = geometry.tile_shape();
    let mut out = vec![0_u8; geometry.occupancy_tile_len()];
    for local_i in 0..effective[0] {
        let src = local_i * effective[1];
        let dst = local_i * tile_shape[1];
        out[dst..dst + effective[1]].copy_from_slice(&raw[src..src + effective[1]]);
    }
    out
}
