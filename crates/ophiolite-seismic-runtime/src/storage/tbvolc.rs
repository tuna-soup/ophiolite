use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lz4_flex::block;
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};

use crate::error::SeismicStoreError;
use crate::metadata::VolumeMetadata;

use super::tbvol::{TbvolReader, TbvolWriter};
use super::tile_geometry::{TileCoord, TileGeometry};
use super::volume_store::{OccupancyTile, TileBuffer, VolumeStoreReader, VolumeStoreWriter};

const MANIFEST_FILE: &str = "manifest.json";
const AMPLITUDE_INDEX_FILE: &str = "amplitude.index.bin";
const AMPLITUDE_FILE: &str = "amplitude.bin";
const OCCUPANCY_FILE: &str = "occupancy.bin";
const INDEX_ENTRY_BYTES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TbvolcAmplitudeEncoding {
    pub codec: String,
    pub compressor: String,
    pub filters: Vec<String>,
    pub compression_level: Option<u8>,
    pub lossless: bool,
}

impl Default for TbvolcAmplitudeEncoding {
    fn default() -> Self {
        Self {
            codec: "native".to_string(),
            compressor: "lz4".to_string(),
            filters: vec!["bitshuffle_g8".to_string()],
            compression_level: None,
            lossless: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbvolcManifest {
    pub format: String,
    pub version: u32,
    pub volume: VolumeMetadata,
    pub tile_shape: [usize; 3],
    pub tile_grid_shape: [usize; 2],
    pub sample_type: String,
    pub endianness: String,
    pub has_occupancy: bool,
    pub amplitude_encoding: TbvolcAmplitudeEncoding,
    pub amplitude_tile_sample_count: usize,
    pub tile_count: usize,
}

impl TbvolcManifest {
    pub fn new(
        volume: VolumeMetadata,
        tile_shape: [usize; 3],
        has_occupancy: bool,
        amplitude_encoding: TbvolcAmplitudeEncoding,
    ) -> Self {
        let geometry = TileGeometry::new(volume.shape, tile_shape);
        Self {
            format: "tbvolc".to_string(),
            version: 1,
            volume,
            tile_shape: geometry.tile_shape(),
            tile_grid_shape: geometry.tile_grid_shape(),
            sample_type: "f32".to_string(),
            endianness: "little".to_string(),
            has_occupancy,
            amplitude_encoding,
            amplitude_tile_sample_count: geometry.amplitude_tile_len(),
            tile_count: geometry.tile_count(),
        }
    }

    pub fn tile_geometry(&self) -> TileGeometry {
        TileGeometry::new(self.volume.shape, self.tile_shape)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TbvolcTileIndexEntry {
    offset: u64,
    length: u32,
    stored_ci: u16,
    stored_cx: u16,
    reserved: u32,
}

impl TbvolcTileIndexEntry {
    fn to_bytes(self) -> [u8; INDEX_ENTRY_BYTES] {
        let mut bytes = [0_u8; INDEX_ENTRY_BYTES];
        bytes[0..8].copy_from_slice(&self.offset.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.length.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.stored_ci.to_le_bytes());
        bytes[14..16].copy_from_slice(&self.stored_cx.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.reserved.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, SeismicStoreError> {
        if bytes.len() != INDEX_ENTRY_BYTES {
            return Err(SeismicStoreError::Message(format!(
                "tbvolc index entry size mismatch: expected {INDEX_ENTRY_BYTES}, found {}",
                bytes.len()
            )));
        }
        let mut offset = [0_u8; 8];
        offset.copy_from_slice(&bytes[0..8]);
        let mut length = [0_u8; 4];
        length.copy_from_slice(&bytes[8..12]);
        let mut stored_ci = [0_u8; 2];
        stored_ci.copy_from_slice(&bytes[12..14]);
        let mut stored_cx = [0_u8; 2];
        stored_cx.copy_from_slice(&bytes[14..16]);
        let mut reserved = [0_u8; 4];
        reserved.copy_from_slice(&bytes[16..20]);
        Ok(Self {
            offset: u64::from_le_bytes(offset),
            length: u32::from_le_bytes(length),
            stored_ci: u16::from_le_bytes(stored_ci),
            stored_cx: u16::from_le_bytes(stored_cx),
            reserved: u32::from_le_bytes(reserved),
        })
    }
}

pub(crate) fn load_tbvolc_manifest(
    manifest_path: &Path,
) -> Result<TbvolcManifest, SeismicStoreError> {
    let bytes = fs::read(manifest_path)?;
    let mut manifest = serde_json::from_slice::<TbvolcManifest>(&bytes)?;
    if crate::metadata::normalize_source_identity(&mut manifest.volume.source) {
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    }
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub struct TbvolcReader {
    _root: PathBuf,
    manifest: TbvolcManifest,
    geometry: TileGeometry,
    amplitude_map: Mmap,
    index: Vec<TbvolcTileIndexEntry>,
    occupancy_map: Option<Mmap>,
}

impl TbvolcReader {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SeismicStoreError> {
        let root = root.as_ref().to_path_buf();
        let manifest = load_tbvolc_manifest(&root.join(MANIFEST_FILE))?;
        let geometry = manifest.tile_geometry();

        let index_bytes = fs::read(root.join(AMPLITUDE_INDEX_FILE))?;
        let expected_index_len = geometry.tile_count() * INDEX_ENTRY_BYTES;
        if index_bytes.len() != expected_index_len {
            return Err(SeismicStoreError::Message(format!(
                "tbvolc index size mismatch: expected {expected_index_len}, found {}",
                index_bytes.len()
            )));
        }
        let mut index = Vec::with_capacity(geometry.tile_count());
        for chunk in index_bytes.chunks_exact(INDEX_ENTRY_BYTES) {
            index.push(TbvolcTileIndexEntry::from_bytes(chunk)?);
        }

        let amplitude_file = File::open(root.join(AMPLITUDE_FILE))?;
        let amplitude_len = amplitude_file.metadata()?.len();
        validate_index_bounds(&index, &geometry, amplitude_len)?;
        let amplitude_map = unsafe { MmapOptions::new().map(&amplitude_file)? };

        let occupancy_map = if manifest.has_occupancy {
            let occupancy_file = File::open(root.join(OCCUPANCY_FILE))?;
            let occupancy_len = occupancy_file.metadata()?.len();
            let expected = geometry.tile_count() as u64 * geometry.occupancy_tile_bytes();
            if occupancy_len != expected {
                return Err(SeismicStoreError::Message(format!(
                    "tbvolc occupancy size mismatch: expected {expected}, found {occupancy_len}"
                )));
            }
            Some(unsafe { MmapOptions::new().map(&occupancy_file)? })
        } else {
            None
        };

        Ok(Self {
            _root: root,
            manifest,
            geometry,
            amplitude_map,
            index,
            occupancy_map,
        })
    }
}

impl VolumeStoreReader for TbvolcReader {
    fn volume(&self) -> &VolumeMetadata {
        &self.manifest.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn read_tile<'a>(&'a self, tile: TileCoord) -> Result<TileBuffer<'a>, SeismicStoreError> {
        let linear = self.geometry.tile_linear_index(tile);
        let entry = self.index[linear];
        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        let compressed = &self.amplitude_map[start..end];
        let shuffled = block::decompress(compressed, self.geometry.amplitude_tile_bytes() as usize)
            .map_err(|error| {
                SeismicStoreError::Message(format!("tbvolc lz4 decompress failed: {error}"))
            })?;
        let unshuffled = bitunshuffle_f32_groups8(&shuffled, self.geometry.amplitude_tile_len())?;
        Ok(TileBuffer::Owned(bytes_to_f32_vec(&unshuffled)?))
    }

    fn read_tile_occupancy<'a>(
        &'a self,
        tile: TileCoord,
    ) -> Result<Option<OccupancyTile<'a>>, SeismicStoreError> {
        let Some(map) = &self.occupancy_map else {
            return Ok(None);
        };
        let offset = self.geometry.occupancy_offset(tile) as usize;
        let end = offset + self.geometry.occupancy_tile_bytes() as usize;
        Ok(Some(OccupancyTile::Borrowed(&map[offset..end])))
    }
}

struct TbvolcWriterState {
    amplitude_file: File,
    next_offset: u64,
    index: Vec<Option<TbvolcTileIndexEntry>>,
}

pub struct TbvolcWriter {
    final_root: PathBuf,
    temp_root: PathBuf,
    manifest: TbvolcManifest,
    geometry: TileGeometry,
    state: Mutex<TbvolcWriterState>,
    occupancy_file: Option<File>,
}

impl TbvolcWriter {
    pub fn create(
        root: impl AsRef<Path>,
        volume: VolumeMetadata,
        tile_shape: [usize; 3],
        has_occupancy: bool,
    ) -> Result<Self, SeismicStoreError> {
        let final_root = root.as_ref().to_path_buf();
        if final_root.exists() {
            return Err(SeismicStoreError::StoreAlreadyExists(final_root));
        }
        let temp_root = final_root.with_extension("tbvolc.tmp");
        if temp_root.exists() {
            fs::remove_dir_all(&temp_root)?;
        }
        fs::create_dir_all(&temp_root)?;

        let manifest = TbvolcManifest::new(
            volume,
            tile_shape,
            has_occupancy,
            TbvolcAmplitudeEncoding::default(),
        );
        validate_manifest(&manifest)?;
        let geometry = manifest.tile_geometry();

        let amplitude_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(temp_root.join(AMPLITUDE_FILE))?;

        let occupancy_file = if has_occupancy {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(temp_root.join(OCCUPANCY_FILE))?;
            file.set_len(geometry.tile_count() as u64 * geometry.occupancy_tile_bytes())?;
            Some(file)
        } else {
            None
        };

        Ok(Self {
            final_root,
            temp_root,
            manifest,
            geometry: geometry.clone(),
            state: Mutex::new(TbvolcWriterState {
                amplitude_file,
                next_offset: 0,
                index: vec![None; geometry.tile_count()],
            }),
            occupancy_file,
        })
    }
}

impl VolumeStoreWriter for TbvolcWriter {
    fn volume(&self) -> &VolumeMetadata {
        &self.manifest.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn write_tile(&self, tile: TileCoord, amplitudes: &[f32]) -> Result<(), SeismicStoreError> {
        if amplitudes.len() != self.geometry.amplitude_tile_len() {
            return Err(SeismicStoreError::Message(format!(
                "tbvolc tile length mismatch: expected {}, found {}",
                self.geometry.amplitude_tile_len(),
                amplitudes.len()
            )));
        }

        let shuffled = bitshuffle_f32_groups8(amplitudes);
        let compressed = block::compress(&shuffled);
        let effective = self.geometry.effective_tile_shape(tile);
        let stored_ci = u16::try_from(effective[0]).map_err(|_| {
            SeismicStoreError::Message(format!(
                "tbvolc tile inline span exceeds u16: {}",
                effective[0]
            ))
        })?;
        let stored_cx = u16::try_from(effective[1]).map_err(|_| {
            SeismicStoreError::Message(format!(
                "tbvolc tile xline span exceeds u16: {}",
                effective[1]
            ))
        })?;
        let length = u32::try_from(compressed.len()).map_err(|_| {
            SeismicStoreError::Message(format!(
                "tbvolc compressed tile payload exceeds u32: {}",
                compressed.len()
            ))
        })?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| SeismicStoreError::Message("tbvolc writer mutex poisoned".to_string()))?;
        let offset = state.next_offset;
        state.amplitude_file.write_all(&compressed)?;
        state.next_offset += compressed.len() as u64;
        let linear = self.geometry.tile_linear_index(tile);
        state.index[linear] = Some(TbvolcTileIndexEntry {
            offset,
            length,
            stored_ci,
            stored_cx,
            reserved: 0,
        });
        Ok(())
    }

    fn write_tile_occupancy(
        &self,
        tile: TileCoord,
        occupancy: &[u8],
    ) -> Result<(), SeismicStoreError> {
        let Some(file) = &self.occupancy_file else {
            return Ok(());
        };
        if occupancy.len() != self.geometry.occupancy_tile_len() {
            return Err(SeismicStoreError::Message(format!(
                "tbvolc occupancy tile length mismatch: expected {}, found {}",
                self.geometry.occupancy_tile_len(),
                occupancy.len()
            )));
        }
        write_all_at(file, occupancy, self.geometry.occupancy_offset(tile))?;
        Ok(())
    }

    fn finalize(self) -> Result<(), SeismicStoreError> {
        let TbvolcWriter {
            final_root,
            temp_root,
            manifest,
            geometry,
            state,
            occupancy_file,
        } = self;
        let state = state
            .into_inner()
            .map_err(|_| SeismicStoreError::Message("tbvolc writer mutex poisoned".to_string()))?;
        let TbvolcWriterState {
            amplitude_file,
            next_offset: _,
            index,
        } = state;
        let mut index_bytes = Vec::with_capacity(geometry.tile_count() * INDEX_ENTRY_BYTES);
        for entry in index {
            let entry = entry.ok_or_else(|| {
                SeismicStoreError::Message(
                    "tbvolc finalize missing one or more tile index entries".to_string(),
                )
            })?;
            index_bytes.extend_from_slice(&entry.to_bytes());
        }

        amplitude_file.sync_all()?;
        if let Some(file) = &occupancy_file {
            file.sync_all()?;
        }
        drop(amplitude_file);
        drop(occupancy_file);
        fs::write(temp_root.join(AMPLITUDE_INDEX_FILE), index_bytes)?;
        fs::write(
            temp_root.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        fs::rename(&temp_root, &final_root)?;
        Ok(())
    }
}

pub fn transcode_tbvol_to_tbvolc(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<(), SeismicStoreError> {
    let reader = TbvolReader::open(input_root)?;
    let writer = TbvolcWriter::create(
        output_root,
        reader.volume().clone(),
        reader.tile_geometry().tile_shape(),
        reader
            .read_tile_occupancy(TileCoord {
                tile_i: 0,
                tile_x: 0,
            })?
            .is_some(),
    )?;
    copy_tiles(&reader, &writer)?;
    writer.finalize()
}

pub fn transcode_tbvolc_to_tbvol(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<(), SeismicStoreError> {
    let reader = TbvolcReader::open(input_root)?;
    let writer = TbvolWriter::create(
        output_root,
        reader.volume().clone(),
        reader.tile_geometry().tile_shape(),
        reader
            .read_tile_occupancy(TileCoord {
                tile_i: 0,
                tile_x: 0,
            })?
            .is_some(),
    )?;
    copy_tiles(&reader, &writer)?;
    writer.finalize()
}

fn copy_tiles<R: VolumeStoreReader, W: VolumeStoreWriter>(
    reader: &R,
    writer: &W,
) -> Result<(), SeismicStoreError> {
    for tile in reader.tile_geometry().iter_tiles() {
        let amplitudes = reader.read_tile(tile)?.into_owned();
        writer.write_tile(tile, &amplitudes)?;
        if let Some(occupancy) = reader.read_tile_occupancy(tile)? {
            let occupancy = occupancy.into_owned();
            writer.write_tile_occupancy(tile, &occupancy)?;
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &TbvolcManifest) -> Result<(), SeismicStoreError> {
    if manifest.format != "tbvolc" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvolc format marker: {}",
            manifest.format
        )));
    }
    if manifest.endianness != "little" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvolc endianness: {}",
            manifest.endianness
        )));
    }
    if manifest.sample_type != "f32" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvolc sample type: {}",
            manifest.sample_type
        )));
    }
    if manifest.tile_shape[2] != manifest.volume.shape[2] {
        return Err(SeismicStoreError::Message(
            "tbvolc tiles must span the full sample axis".to_string(),
        ));
    }
    if !manifest.amplitude_encoding.lossless {
        return Err(SeismicStoreError::Message(
            "tbvolc only supports lossless amplitude encoding".to_string(),
        ));
    }
    if manifest.amplitude_encoding.codec != "native" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvolc codec family: {}",
            manifest.amplitude_encoding.codec
        )));
    }
    if manifest.amplitude_encoding.compressor != "lz4" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvolc compressor: {}",
            manifest.amplitude_encoding.compressor
        )));
    }
    if manifest.amplitude_encoding.filters.as_slice() != ["bitshuffle_g8"] {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvolc filters: {:?}",
            manifest.amplitude_encoding.filters
        )));
    }
    Ok(())
}

fn validate_index_bounds(
    index: &[TbvolcTileIndexEntry],
    geometry: &TileGeometry,
    amplitude_len: u64,
) -> Result<(), SeismicStoreError> {
    if index.len() != geometry.tile_count() {
        return Err(SeismicStoreError::Message(format!(
            "tbvolc index entry count mismatch: expected {}, found {}",
            geometry.tile_count(),
            index.len()
        )));
    }
    let tile_grid_shape = geometry.tile_grid_shape();
    for (linear, entry) in index.iter().enumerate() {
        let end = entry
            .offset
            .checked_add(entry.length as u64)
            .ok_or_else(|| {
                SeismicStoreError::Message("tbvolc tile payload overflow".to_string())
            })?;
        if end > amplitude_len {
            return Err(SeismicStoreError::Message(format!(
                "tbvolc tile payload exceeds amplitude file: end {end}, file {amplitude_len}"
            )));
        }
        let tile = TileCoord {
            tile_i: linear / tile_grid_shape[1],
            tile_x: linear % tile_grid_shape[1],
        };
        let effective = geometry.effective_tile_shape(tile);
        if usize::from(entry.stored_ci) != effective[0]
            || usize::from(entry.stored_cx) != effective[1]
        {
            return Err(SeismicStoreError::Message(format!(
                "tbvolc stored tile span mismatch at ({}, {}): expected [{}, {}], found [{}, {}]",
                tile.tile_i,
                tile.tile_x,
                effective[0],
                effective[1],
                entry.stored_ci,
                entry.stored_cx
            )));
        }
    }
    Ok(())
}

fn bitshuffle_f32_groups8(values: &[f32]) -> Vec<u8> {
    let input = f32_slice_as_bytes(values);
    let mut output = vec![0_u8; input.len()];
    let full_groups = values.len() / 8;
    let tail_values = values.len() % 8;

    for group in 0..full_groups {
        let input_base = group * 32;
        let output_base = group * 32;
        for byte_index in 0..4 {
            for bit_index in 0..8 {
                let mut packed = 0_u8;
                for lane in 0..8 {
                    let source = input[input_base + lane * 4 + byte_index];
                    let bit = (source >> bit_index) & 1;
                    packed |= bit << lane;
                }
                output[output_base + byte_index * 8 + bit_index] = packed;
            }
        }
    }

    let tail_byte_count = tail_values * 4;
    if tail_byte_count > 0 {
        let start = full_groups * 32;
        output[start..start + tail_byte_count]
            .copy_from_slice(&input[start..start + tail_byte_count]);
    }

    output
}

fn bitunshuffle_f32_groups8(
    input: &[u8],
    sample_count: usize,
) -> Result<Vec<u8>, SeismicStoreError> {
    let expected_len = sample_count * std::mem::size_of::<f32>();
    if input.len() != expected_len {
        return Err(SeismicStoreError::Message(format!(
            "tbvolc unshuffle byte length mismatch: expected {expected_len}, found {}",
            input.len()
        )));
    }
    let mut output = vec![0_u8; input.len()];
    let full_groups = sample_count / 8;
    let tail_values = sample_count % 8;

    for group in 0..full_groups {
        let input_base = group * 32;
        let output_base = group * 32;
        for byte_index in 0..4 {
            for bit_index in 0..8 {
                let packed = input[input_base + byte_index * 8 + bit_index];
                for lane in 0..8 {
                    let bit = (packed >> lane) & 1;
                    output[output_base + lane * 4 + byte_index] |= bit << bit_index;
                }
            }
        }
    }

    let tail_byte_count = tail_values * 4;
    if tail_byte_count > 0 {
        let start = full_groups * 32;
        output[start..start + tail_byte_count]
            .copy_from_slice(&input[start..start + tail_byte_count]);
    }

    Ok(output)
}

fn bytes_to_f32_vec(bytes: &[u8]) -> Result<Vec<f32>, SeismicStoreError> {
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "tbvolc amplitude byte length is not f32 aligned: {}",
            bytes.len()
        )));
    }
    let mut values = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let mut raw = [0_u8; 4];
        raw.copy_from_slice(chunk);
        values.push(f32::from_le_bytes(raw));
    }
    Ok(values)
}

fn f32_slice_as_bytes(values: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

#[cfg(unix)]
fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;

    while !bytes.is_empty() {
        let written = file.write_at(bytes, offset)?;
        if written == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "failed to write tbvolc bytes",
            ));
        }
        bytes = &bytes[written..];
        offset += written as u64;
    }
    Ok(())
}

#[cfg(windows)]
fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;

    while !bytes.is_empty() {
        let written = file.seek_write(bytes, offset)?;
        if written == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "failed to write tbvolc bytes",
            ));
        }
        bytes = &bytes[written..];
        offset += written as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::SectionAxis;
    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        VolumeMetadata, generate_store_id,
    };
    use crate::storage::section_assembler;

    use super::*;

    #[test]
    fn tbvolc_roundtrip_reads_tiles_and_sections() {
        let root = unique_test_root("tbvolc-roundtrip", "tbvolc");
        let volume = test_volume_metadata([3, 4, 5]);
        let tile_shape = [2, 2, 5];
        let geometry = TileGeometry::new(volume.shape, tile_shape);
        let writer = TbvolcWriter::create(&root, volume.clone(), tile_shape, true).unwrap();

        for tile in geometry.iter_tiles() {
            let origin = geometry.tile_origin(tile);
            let effective = geometry.effective_tile_shape(tile);
            let mut amplitudes = vec![0.0_f32; geometry.amplitude_tile_len()];
            let mut occupancy = vec![0_u8; geometry.occupancy_tile_len()];
            for local_i in 0..effective[0] {
                for local_x in 0..effective[1] {
                    let dst = (local_i * tile_shape[1]) + local_x;
                    occupancy[dst] = if origin[0] + local_i == 1 && origin[1] + local_x == 2 {
                        0
                    } else {
                        1
                    };
                    let trace_start = dst * tile_shape[2];
                    for sample in 0..effective[2] {
                        amplitudes[trace_start + sample] =
                            amplitude_value(origin[0] + local_i, origin[1] + local_x, sample);
                    }
                }
            }
            writer.write_tile(tile, &amplitudes).unwrap();
            writer.write_tile_occupancy(tile, &occupancy).unwrap();
        }
        writer.finalize().unwrap();

        let reader = TbvolcReader::open(&root).unwrap();
        let tile = reader
            .read_tile(TileCoord {
                tile_i: 1,
                tile_x: 1,
            })
            .unwrap()
            .into_owned();
        assert_eq!(tile[0], amplitude_value(2, 2, 0));
        assert_eq!(tile[1], amplitude_value(2, 2, 1));
        assert_eq!(tile[5], amplitude_value(2, 3, 0));

        let inline =
            section_assembler::read_section_plane(&reader, SectionAxis::Inline, 1).unwrap();
        assert_eq!(inline.traces, 4);
        assert_eq!(inline.samples, 5);
        assert_eq!(inline.amplitudes[10], amplitude_value(1, 2, 0));
        assert_eq!(inline.occupancy.as_ref().unwrap()[2], 0);

        drop(reader);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tbvol_to_tbvolc_to_tbvol_roundtrip_preserves_tiles() {
        let source_root = unique_test_root("tbvol-source", "tbvol");
        let compressed_root = unique_test_root("tbvol-compressed", "tbvolc");
        let restored_root = unique_test_root("tbvol-restored", "tbvol");
        let volume = test_volume_metadata([3, 4, 5]);
        let tile_shape = [2, 2, 5];
        let geometry = TileGeometry::new(volume.shape, tile_shape);
        let writer = TbvolWriter::create(&source_root, volume.clone(), tile_shape, true).unwrap();

        for tile in geometry.iter_tiles() {
            let origin = geometry.tile_origin(tile);
            let effective = geometry.effective_tile_shape(tile);
            let mut amplitudes = vec![0.0_f32; geometry.amplitude_tile_len()];
            let mut occupancy = vec![0_u8; geometry.occupancy_tile_len()];
            for local_i in 0..effective[0] {
                for local_x in 0..effective[1] {
                    let dst = (local_i * tile_shape[1]) + local_x;
                    occupancy[dst] = 1;
                    let trace_start = dst * tile_shape[2];
                    for sample in 0..effective[2] {
                        amplitudes[trace_start + sample] =
                            amplitude_value(origin[0] + local_i, origin[1] + local_x, sample);
                    }
                }
            }
            writer.write_tile(tile, &amplitudes).unwrap();
            writer.write_tile_occupancy(tile, &occupancy).unwrap();
        }
        writer.finalize().unwrap();

        transcode_tbvol_to_tbvolc(&source_root, &compressed_root).unwrap();
        transcode_tbvolc_to_tbvol(&compressed_root, &restored_root).unwrap();

        let source_reader = TbvolReader::open(&source_root).unwrap();
        let restored_reader = TbvolReader::open(&restored_root).unwrap();
        for tile in source_reader.tile_geometry().iter_tiles() {
            assert_eq!(
                source_reader.read_tile(tile).unwrap().as_slice(),
                restored_reader.read_tile(tile).unwrap().as_slice()
            );
            assert_eq!(
                source_reader
                    .read_tile_occupancy(tile)
                    .unwrap()
                    .unwrap()
                    .as_slice(),
                restored_reader
                    .read_tile_occupancy(tile)
                    .unwrap()
                    .unwrap()
                    .as_slice()
            );
        }

        let _ = fs::remove_dir_all(&source_root);
        let _ = fs::remove_dir_all(&compressed_root);
        let _ = fs::remove_dir_all(&restored_root);
    }

    fn amplitude_value(iline: usize, xline: usize, sample: usize) -> f32 {
        (iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32
    }

    fn test_volume_metadata(shape: [usize; 3]) -> VolumeMetadata {
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://tbvolc-test"),
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
            axes: VolumeAxes {
                ilines: (0..shape[0]).map(|value| value as f64).collect(),
                xlines: (0..shape[1]).map(|value| value as f64).collect(),
                sample_axis_ms: (0..shape[2]).map(|value| value as f32 * 2.0).collect(),
            },
            segy_export: None,
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "tbvolc-test".to_string(),
            processing_lineage: None,
        }
    }

    fn unique_test_root(label: &str, extension: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("traceboost-{label}-{suffix}.{extension}"))
    }
}
