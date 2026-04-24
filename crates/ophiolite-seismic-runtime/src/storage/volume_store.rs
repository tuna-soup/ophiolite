use crate::error::SeismicStoreError;
use crate::metadata::{
    VolumeMetadata, generate_store_id, normalize_source_identity, normalize_volume_axes,
    validate_vertical_axis,
};
use ndarray::{Array2, Array3};

use super::tile_geometry::{TileCoord, TileGeometry};

#[derive(Debug, Clone)]
pub(crate) struct PostStackStoreEnvelope {
    pub format: String,
    pub version: u32,
    pub volume: VolumeMetadata,
    pub tile_shape: [usize; 3],
    pub tile_grid_shape: [usize; 2],
    pub sample_type: String,
    pub endianness: String,
    pub has_occupancy: bool,
}

impl PostStackStoreEnvelope {
    pub fn tile_geometry(&self) -> TileGeometry {
        TileGeometry::new(self.volume.shape, self.tile_shape)
    }

    pub fn store_format_version(&self) -> String {
        format!("{}@{}", self.format, self.version)
    }

    pub fn parent_artifact_key(&self) -> Option<String> {
        self.volume
            .processing_lineage
            .as_ref()
            .and_then(|lineage| lineage.artifact_key.as_ref())
            .map(|artifact_key| artifact_key.cache_key.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostStackStoreCompatibilityReport {
    pub exact_compatible: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum StoreBuffer<'a, T> {
    Borrowed(&'a [T]),
    Owned(Vec<T>),
}

pub type TileAmplitudeBuffer<'a> = StoreBuffer<'a, f32>;
pub type TileOccupancyBuffer<'a> = StoreBuffer<'a, u8>;
pub type TileBuffer<'a> = TileAmplitudeBuffer<'a>;
pub type OccupancyTile<'a> = TileOccupancyBuffer<'a>;

pub(crate) fn normalize_poststack_volume_metadata(volume: &mut VolumeMetadata) -> bool {
    let mut changed = false;
    if volume.store_id.trim().is_empty() {
        volume.store_id = generate_store_id();
        changed = true;
    }
    if normalize_source_identity(&mut volume.source) {
        changed = true;
    }
    if normalize_volume_axes(&mut volume.axes) {
        changed = true;
    }
    changed
}

pub(crate) fn validate_poststack_store_envelope(
    envelope: &PostStackStoreEnvelope,
    expected_format: &str,
) -> Result<(), SeismicStoreError> {
    if envelope.format != expected_format {
        return Err(SeismicStoreError::Message(format!(
            "unsupported {expected_format} format marker: {}",
            envelope.format
        )));
    }
    if envelope.version == 0 {
        return Err(SeismicStoreError::Message(format!(
            "{expected_format} version must be >= 1"
        )));
    }
    if envelope.volume.store_id.trim().is_empty() {
        return Err(SeismicStoreError::Message(format!(
            "{expected_format} store_id must not be empty"
        )));
    }
    if envelope.endianness != "little" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported {expected_format} endianness: {}",
            envelope.endianness
        )));
    }
    if envelope.sample_type != "f32" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported {expected_format} sample type: {}",
            envelope.sample_type
        )));
    }

    let geometry = envelope.tile_geometry();
    if envelope.tile_shape[2] != envelope.volume.shape[2] {
        return Err(SeismicStoreError::Message(format!(
            "{expected_format} tiles must span the full sample axis"
        )));
    }
    if envelope.tile_grid_shape != geometry.tile_grid_shape() {
        return Err(SeismicStoreError::Message(format!(
            "{expected_format} tile grid shape mismatch: expected {:?}, found {:?}",
            geometry.tile_grid_shape(),
            envelope.tile_grid_shape
        )));
    }
    validate_vertical_axis(
        &envelope.volume.axes.sample_axis_ms,
        envelope.volume.shape[2],
        "sample axis",
    )
    .map_err(SeismicStoreError::Message)?;
    Ok(())
}

pub(crate) fn compare_exact_poststack_store_envelopes(
    working: &PostStackStoreEnvelope,
    archive: &PostStackStoreEnvelope,
) -> Result<PostStackStoreCompatibilityReport, SeismicStoreError> {
    let mut warnings = Vec::new();

    if archive.volume.store_id != working.volume.store_id {
        warnings.push(format!(
            "store_id mismatch: working={}, archive={}",
            working.volume.store_id, archive.volume.store_id
        ));
    }
    if archive.volume.shape != working.volume.shape {
        warnings.push(format!(
            "shape mismatch: working={:?}, archive={:?}",
            working.volume.shape, archive.volume.shape
        ));
    }
    if archive.tile_shape != working.tile_shape {
        warnings.push(format!(
            "tile shape mismatch: working={:?}, archive={:?}",
            working.tile_shape, archive.tile_shape
        ));
    }
    if archive.has_occupancy != working.has_occupancy {
        warnings.push(format!(
            "occupancy flag mismatch: working={}, archive={}",
            working.has_occupancy, archive.has_occupancy
        ));
    }
    if archive.sample_type != working.sample_type {
        warnings.push(format!(
            "sample type mismatch: working={}, archive={}",
            working.sample_type, archive.sample_type
        ));
    }
    if archive.endianness != working.endianness {
        warnings.push(format!(
            "endianness mismatch: working={}, archive={}",
            working.endianness, archive.endianness
        ));
    }
    if serde_json::to_vec(&archive.volume.axes)? != serde_json::to_vec(&working.volume.axes)? {
        warnings.push("volume axes mismatch between working store and archive".to_string());
    }
    if serde_json::to_vec(&archive.volume.source)? != serde_json::to_vec(&working.volume.source)? {
        warnings.push("source identity mismatch between working store and archive".to_string());
    }

    Ok(PostStackStoreCompatibilityReport {
        exact_compatible: warnings.is_empty(),
        warnings,
    })
}

impl<'a, T> StoreBuffer<'a, T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Borrowed(values) => values,
            Self::Owned(values) => values,
        }
    }
}

impl<'a, T: Clone> StoreBuffer<'a, T> {
    pub fn into_owned(self) -> Vec<T> {
        match self {
            Self::Borrowed(values) => values.to_vec(),
            Self::Owned(values) => values,
        }
    }
}

impl<'a, T> StoreBuffer<'a, T> {
    pub fn borrowed(values: &'a [T]) -> Self {
        Self::Borrowed(values)
    }

    pub fn owned(values: Vec<T>) -> Self {
        Self::Owned(values)
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
    let layout = geometry.trace_major_layout();
    for tile in geometry.iter_tiles() {
        let extent = geometry.tile_extent(tile);
        let mut amplitudes = vec![0.0_f32; geometry.amplitude_tile_len()];
        for local_i in 0..extent.trace_shape[0] {
            for local_x in 0..extent.trace_shape[1] {
                let trace_start = layout.amplitude_trace_offset(local_i, local_x);
                for sample in 0..extent.samples {
                    amplitudes[trace_start + sample] = data[[
                        extent.origin[0] + local_i,
                        extent.origin[1] + local_x,
                        sample,
                    ]];
                }
            }
        }
        writer.write_tile(tile, &amplitudes)?;

        if let Some(mask) = occupancy {
            let mut occupancy_tile = vec![0_u8; geometry.occupancy_tile_len()];
            for local_i in 0..extent.trace_shape[0] {
                for local_x in 0..extent.trace_shape[1] {
                    let dst = layout.occupancy_index(local_i, local_x);
                    occupancy_tile[dst] =
                        mask[[extent.origin[0] + local_i, extent.origin[1] + local_x]];
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
    let geometry = reader.tile_geometry().clone();
    let layout = geometry.trace_major_layout();

    for tile in geometry.iter_tiles() {
        let tile_data = reader.read_tile(tile)?;
        let extent = geometry.tile_extent(tile);
        let tile_samples = tile_data.as_slice();
        for local_i in 0..extent.trace_shape[0] {
            for local_x in 0..extent.trace_shape[1] {
                let trace_start = layout.amplitude_trace_offset(local_i, local_x);
                for sample in 0..extent.samples {
                    data[[
                        extent.origin[0] + local_i,
                        extent.origin[1] + local_x,
                        sample,
                    ]] = tile_samples[trace_start + sample];
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
    let geometry = reader.tile_geometry().clone();
    let layout = geometry.trace_major_layout();
    let mut has_any = false;

    for tile in geometry.iter_tiles() {
        let Some(tile_occupancy) = reader.read_tile_occupancy(tile)? else {
            continue;
        };
        has_any = true;
        let extent = geometry.tile_extent(tile);
        let tile_values = tile_occupancy.as_slice();
        for local_i in 0..extent.trace_shape[0] {
            for local_x in 0..extent.trace_shape[1] {
                let src = layout.occupancy_index(local_i, local_x);
                occupancy[[extent.origin[0] + local_i, extent.origin[1] + local_x]] =
                    tile_values[src];
            }
        }
    }

    Ok(has_any.then_some(occupancy))
}
