use std::fs::{self, File};
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};

use crate::error::SeismicStoreError;
use crate::metadata::VolumeMetadata;

use super::tile_geometry::{TileCoord, TileGeometry};
use super::volume_store::{OccupancyTile, TileBuffer, VolumeStoreReader, VolumeStoreWriter};

const MANIFEST_FILE: &str = "manifest.json";
const AMPLITUDE_FILE: &str = "amplitude.bin";
const OCCUPANCY_FILE: &str = "occupancy.bin";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbvolManifest {
    pub format: String,
    pub version: u32,
    pub volume: VolumeMetadata,
    pub tile_shape: [usize; 3],
    pub tile_grid_shape: [usize; 2],
    pub sample_type: String,
    pub endianness: String,
    pub has_occupancy: bool,
    pub amplitude_tile_bytes: u64,
    pub occupancy_tile_bytes: Option<u64>,
}

impl TbvolManifest {
    pub fn new(volume: VolumeMetadata, tile_shape: [usize; 3], has_occupancy: bool) -> Self {
        let geometry = TileGeometry::new(volume.shape, tile_shape);
        Self {
            format: "tbvol".to_string(),
            version: 1,
            volume,
            tile_shape: geometry.tile_shape(),
            tile_grid_shape: geometry.tile_grid_shape(),
            sample_type: "f32".to_string(),
            endianness: "little".to_string(),
            has_occupancy,
            amplitude_tile_bytes: geometry.amplitude_tile_bytes(),
            occupancy_tile_bytes: has_occupancy.then(|| geometry.occupancy_tile_bytes()),
        }
    }

    pub fn tile_geometry(&self) -> TileGeometry {
        TileGeometry::new(self.volume.shape, self.tile_shape)
    }
}

pub fn recommended_tbvol_tile_shape(shape: [usize; 3], tile_target_mib: u16) -> [usize; 3] {
    let [ilines, xlines, samples] = shape;
    let ilines = ilines.max(1);
    let xlines = xlines.max(1);
    let samples = samples.max(1);
    let bytes_per_trace = (samples * std::mem::size_of::<f32>()) as u64;
    let target_bytes = tile_target_mib as u64 * 1024 * 1024;
    let target_traces = (target_bytes / bytes_per_trace).max(1) as usize;
    let total_traces = ilines * xlines;

    if target_traces >= total_traces {
        return [ilines, xlines, samples];
    }

    let mut best_shape = [1, 1, samples];
    let mut best_score = f64::INFINITY;

    for ci in 1..=ilines {
        let grid_i = ilines.div_ceil(ci);
        let stored_i = grid_i * ci;
        for cx in 1..=xlines {
            let grid_x = xlines.div_ceil(cx);
            let stored_x = grid_x * cx;
            let tile_traces = ci * cx;
            let stored_traces = stored_i * stored_x;
            let padding_traces = stored_traces.saturating_sub(total_traces);
            let target_delta =
                ((tile_traces as f64 - target_traces as f64).abs()) / target_traces as f64;
            let padding_ratio = padding_traces as f64 / stored_traces.max(1) as f64;
            let aspect_penalty = ((ci as f64 / cx as f64).ln()).abs();
            let tile_count = (grid_i * grid_x) as f64;
            let score =
                (target_delta * 5.0) + (padding_ratio * 4.0) + (aspect_penalty * 0.05) + (tile_count * 0.001);

            let better = score < best_score
                || ((score - best_score).abs() < f64::EPSILON
                    && tile_traces > best_shape[0] * best_shape[1]);
            if better {
                best_score = score;
                best_shape = [ci, cx, samples];
            }
        }
    }

    best_shape
}

pub struct TbvolReader {
    _root: PathBuf,
    manifest: TbvolManifest,
    geometry: TileGeometry,
    amplitude_map: Mmap,
    occupancy_map: Option<Mmap>,
}

impl TbvolReader {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SeismicStoreError> {
        let root = root.as_ref().to_path_buf();
        let manifest: TbvolManifest =
            serde_json::from_slice(&fs::read(root.join(MANIFEST_FILE))?)?;
        validate_manifest(&manifest)?;
        let geometry = manifest.tile_geometry();

        let amplitude_file = File::open(root.join(AMPLITUDE_FILE))?;
        let amplitude_len = amplitude_file.metadata()?.len();
        let expected_amplitude = geometry.tile_count() as u64 * geometry.amplitude_tile_bytes();
        if amplitude_len != expected_amplitude {
            return Err(SeismicStoreError::Message(format!(
                "tbvol amplitude size mismatch: expected {expected_amplitude}, found {amplitude_len}"
            )));
        }
        let amplitude_map = unsafe { MmapOptions::new().map(&amplitude_file)? };

        let occupancy_map = if manifest.has_occupancy {
            let occupancy_file = File::open(root.join(OCCUPANCY_FILE))?;
            let occupancy_len = occupancy_file.metadata()?.len();
            let expected = geometry.tile_count() as u64 * geometry.occupancy_tile_bytes();
            if occupancy_len != expected {
                return Err(SeismicStoreError::Message(format!(
                    "tbvol occupancy size mismatch: expected {expected}, found {occupancy_len}"
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
            occupancy_map,
        })
    }
}

impl VolumeStoreReader for TbvolReader {
    fn volume(&self) -> &VolumeMetadata {
        &self.manifest.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn read_tile<'a>(&'a self, tile: TileCoord) -> Result<TileBuffer<'a>, SeismicStoreError> {
        let offset = self.geometry.amplitude_offset(tile) as usize;
        let end = offset + self.geometry.amplitude_tile_bytes() as usize;
        let bytes = &self.amplitude_map[offset..end];
        Ok(TileBuffer::Borrowed(bytes_as_f32_slice(bytes)?))
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

pub struct TbvolWriter {
    final_root: PathBuf,
    temp_root: PathBuf,
    manifest: TbvolManifest,
    geometry: TileGeometry,
    amplitude_file: File,
    occupancy_file: Option<File>,
}

impl TbvolWriter {
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
        let temp_root = final_root.with_extension("tbvol.tmp");
        if temp_root.exists() {
            fs::remove_dir_all(&temp_root)?;
        }
        fs::create_dir_all(&temp_root)?;

        let manifest = TbvolManifest::new(volume, tile_shape, has_occupancy);
        validate_manifest(&manifest)?;
        let geometry = manifest.tile_geometry();

        let amplitude_path = temp_root.join(AMPLITUDE_FILE);
        let amplitude_file = File::create(&amplitude_path)?;
        amplitude_file.set_len(geometry.tile_count() as u64 * geometry.amplitude_tile_bytes())?;

        let occupancy_file = if has_occupancy {
            let file = File::create(temp_root.join(OCCUPANCY_FILE))?;
            file.set_len(geometry.tile_count() as u64 * geometry.occupancy_tile_bytes())?;
            Some(file)
        } else {
            None
        };

        Ok(Self {
            final_root,
            temp_root,
            manifest,
            geometry,
            amplitude_file,
            occupancy_file,
        })
    }

    pub fn root(&self) -> &Path {
        &self.final_root
    }
}

impl VolumeStoreWriter for TbvolWriter {
    fn volume(&self) -> &VolumeMetadata {
        &self.manifest.volume
    }

    fn tile_geometry(&self) -> &TileGeometry {
        &self.geometry
    }

    fn write_tile(&self, tile: TileCoord, amplitudes: &[f32]) -> Result<(), SeismicStoreError> {
        if amplitudes.len() != self.geometry.amplitude_tile_len() {
            return Err(SeismicStoreError::Message(format!(
                "tbvol tile length mismatch: expected {}, found {}",
                self.geometry.amplitude_tile_len(),
                amplitudes.len()
            )));
        }
        write_all_at(
            &self.amplitude_file,
            f32_slice_as_bytes(amplitudes),
            self.geometry.amplitude_offset(tile),
        )?;
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
                "tbvol occupancy tile length mismatch: expected {}, found {}",
                self.geometry.occupancy_tile_len(),
                occupancy.len()
            )));
        }
        write_all_at(file, occupancy, self.geometry.occupancy_offset(tile))?;
        Ok(())
    }

    fn finalize(self) -> Result<(), SeismicStoreError> {
        self.amplitude_file.sync_all()?;
        if let Some(file) = &self.occupancy_file {
            file.sync_all()?;
        }
        let final_root = self.final_root.clone();
        let temp_root = self.temp_root.clone();
        let manifest = self.manifest.clone();
        let geometry = self.geometry.clone();
        fs::write(
            temp_root.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        let amplitude_len = fs::metadata(temp_root.join(AMPLITUDE_FILE))?.len();
        let expected_amplitude = geometry.tile_count() as u64 * geometry.amplitude_tile_bytes();
        if amplitude_len != expected_amplitude {
            return Err(SeismicStoreError::Message(format!(
                "tbvol amplitude finalize size mismatch: expected {expected_amplitude}, found {amplitude_len}"
            )));
        }
        if manifest.has_occupancy {
            let occupancy_len = fs::metadata(temp_root.join(OCCUPANCY_FILE))?.len();
            let expected_occupancy =
                geometry.tile_count() as u64 * geometry.occupancy_tile_bytes();
            if occupancy_len != expected_occupancy {
                return Err(SeismicStoreError::Message(format!(
                    "tbvol occupancy finalize size mismatch: expected {expected_occupancy}, found {occupancy_len}"
                )));
            }
        }
        drop(self);
        fs::rename(&temp_root, &final_root)?;
        Ok(())
    }
}

fn validate_manifest(manifest: &TbvolManifest) -> Result<(), SeismicStoreError> {
    if manifest.format != "tbvol" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvol format marker: {}",
            manifest.format
        )));
    }
    if manifest.endianness != "little" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvol endianness: {}",
            manifest.endianness
        )));
    }
    if manifest.sample_type != "f32" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbvol sample type: {}",
            manifest.sample_type
        )));
    }
    if manifest.tile_shape[2] != manifest.volume.shape[2] {
        return Err(SeismicStoreError::Message(
            "tbvol tiles must span the full sample axis".to_string(),
        ));
    }
    Ok(())
}

fn bytes_as_f32_slice(bytes: &[u8]) -> Result<&[f32], SeismicStoreError> {
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "tbvol amplitude byte length is not f32 aligned: {}",
            bytes.len()
        )));
    }
    let (prefix, aligned, suffix) = unsafe { bytes.align_to::<f32>() };
    if !prefix.is_empty() || !suffix.is_empty() {
        return Err(SeismicStoreError::Message(
            "tbvol amplitude mapping is not aligned to f32".to_string(),
        ));
    }
    Ok(aligned)
}

fn f32_slice_as_bytes(values: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            values.as_ptr().cast::<u8>(),
            std::mem::size_of_val(values),
        )
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
                "failed to write tbvol bytes",
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
                "failed to write tbvol bytes",
            ));
        }
        bytes = &bytes[written..];
        offset += written as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
    };
    use crate::storage::section_assembler;
    use crate::SectionAxis;

    use super::*;

    #[test]
    fn tbvol_roundtrip_reads_tiles_and_sections() {
        let root = unique_test_root("tbvol-roundtrip");
        let volume = test_volume_metadata([3, 4, 2]);
        let tile_shape = [2, 2, 2];
        let geometry = TileGeometry::new(volume.shape, tile_shape);
        let writer = TbvolWriter::create(&root, volume.clone(), tile_shape, true).unwrap();

        for tile in geometry.iter_tiles() {
            let origin = geometry.tile_origin(tile);
            let effective = geometry.effective_tile_shape(tile);
            let mut amplitudes = vec![0.0_f32; geometry.amplitude_tile_len()];
            let mut occupancy = vec![0_u8; geometry.occupancy_tile_len()];
            for local_i in 0..effective[0] {
                for local_x in 0..effective[1] {
                    let dst = (local_i * tile_shape[1]) + local_x;
                    occupancy[dst] =
                        if origin[0] + local_i == 1 && origin[1] + local_x == 2 { 0 } else { 1 };
                    let trace_start = dst * tile_shape[2];
                    for sample in 0..effective[2] {
                        amplitudes[trace_start + sample] = amplitude_value(
                            origin[0] + local_i,
                            origin[1] + local_x,
                            sample,
                        );
                    }
                }
            }
            writer.write_tile(tile, &amplitudes).unwrap();
            writer.write_tile_occupancy(tile, &occupancy).unwrap();
        }
        writer.finalize().unwrap();

        let reader = TbvolReader::open(&root).unwrap();
        let tile = reader
            .read_tile(TileCoord {
                tile_i: 1,
                tile_x: 1,
            })
            .unwrap()
            .into_owned();
        assert_eq!(tile[0], amplitude_value(2, 2, 0));
        assert_eq!(tile[1], amplitude_value(2, 2, 1));
        assert_eq!(tile[2], amplitude_value(2, 3, 0));
        assert_eq!(tile[3], amplitude_value(2, 3, 1));

        let inline = section_assembler::read_section_plane(&reader, SectionAxis::Inline, 1).unwrap();
        assert_eq!(inline.traces, 4);
        assert_eq!(inline.samples, 2);
        assert_eq!(inline.amplitudes[4], amplitude_value(1, 2, 0));
        assert_eq!(inline.amplitudes[5], amplitude_value(1, 2, 1));
        assert_eq!(inline.occupancy.as_ref().unwrap()[2], 0);

        let xline = section_assembler::read_section_plane(&reader, SectionAxis::Xline, 3).unwrap();
        assert_eq!(xline.traces, 3);
        assert_eq!(xline.samples, 2);
        assert_eq!(xline.amplitudes[4], amplitude_value(2, 3, 0));
        assert_eq!(xline.amplitudes[5], amplitude_value(2, 3, 1));

        drop(reader);
        let _ = fs::remove_dir_all(&root);
    }

    fn amplitude_value(iline: usize, xline: usize, sample: usize) -> f32 {
        (iline as f32 * 100.0) + (xline as f32 * 10.0) + sample as f32
    }

    fn test_volume_metadata(shape: [usize; 3]) -> VolumeMetadata {
        VolumeMetadata {
            kind: DatasetKind::Source,
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://tbvol-test"),
                file_size: 0,
                trace_count: (shape[0] * shape[1]) as u64,
                samples_per_trace: shape[2],
                sample_interval_us: 2000,
                sample_format_code: 5,
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
            created_by: "tbvol-test".to_string(),
            processing_lineage: None,
        }
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("traceboost-{label}-{suffix}.tbvol"))
    }

    #[test]
    fn recommends_padding_aware_full_trace_tiles() {
        assert_eq!(
            recommended_tbvol_tile_shape([256, 256, 1024], 4),
            [32, 32, 1024]
        );
        assert_eq!(
            recommended_tbvol_tile_shape([23, 18, 75], 1),
            [23, 18, 75]
        );
    }
}
