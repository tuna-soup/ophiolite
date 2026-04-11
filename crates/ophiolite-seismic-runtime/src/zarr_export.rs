use std::fs;
use std::path::Path;

use crate::error::SeismicStoreError;
use crate::metadata::{CompressionKind, StorageLayout};
use crate::storage::tbvol::TbvolReader;
use crate::storage::volume_store::{
    VolumeStoreWriter, read_dense_occupancy, read_dense_volume, write_dense_volume,
};
use crate::storage::zarr::ZarrVolumeStoreWriter;
use crate::store::open_store;

pub fn export_store_to_zarr(
    store_root: &Path,
    output_root: &Path,
    overwrite_existing: bool,
) -> Result<(), SeismicStoreError> {
    export_store_to_zarr_with_layout(
        store_root,
        output_root,
        overwrite_existing,
        &StorageLayout::default(),
    )
}

pub fn export_store_to_zarr_with_layout(
    store_root: &Path,
    output_root: &Path,
    overwrite_existing: bool,
    storage_layout: &StorageLayout,
) -> Result<(), SeismicStoreError> {
    prepare_output_root(store_root, output_root, overwrite_existing)?;

    let handle = open_store(store_root)?;
    let reader = TbvolReader::open(store_root)?;
    let data = read_dense_volume(&reader)?;
    let occupancy = read_dense_occupancy(&reader)?;
    let writer = ZarrVolumeStoreWriter::create(
        output_root,
        handle.manifest.volume.clone(),
        handle.manifest.tile_shape,
        storage_layout.clone(),
        occupancy.is_some(),
    )?;
    write_dense_volume(&writer, &data, occupancy.as_ref())?;
    writer.finalize()?;
    Ok(())
}

pub fn default_zarr_storage_layout() -> StorageLayout {
    StorageLayout {
        compression: CompressionKind::None,
        shard_shape: None,
    }
}

fn prepare_output_root(
    store_root: &Path,
    output_root: &Path,
    overwrite_existing: bool,
) -> Result<(), SeismicStoreError> {
    let store_root = store_root
        .canonicalize()
        .unwrap_or_else(|_| store_root.to_path_buf());
    let output_root = output_root
        .canonicalize()
        .unwrap_or_else(|_| output_root.to_path_buf());

    if store_root == output_root {
        return Err(SeismicStoreError::Message(
            "output Zarr store cannot overwrite the input tbvol store".to_string(),
        ));
    }

    if !output_root.exists() {
        return Ok(());
    }

    if !overwrite_existing {
        return Err(SeismicStoreError::StoreAlreadyExists(output_root));
    }

    let metadata = fs::symlink_metadata(&output_root)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(&output_root)?;
    } else {
        fs::remove_file(&output_root)?;
    }
    Ok(())
}
