use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::SeismicStoreError;
use crate::metadata::SegyExportDescriptor;
use crate::storage::tile_geometry::TileCoord;
use crate::storage::volume_store::VolumeStoreReader;
use crate::store::{StoreHandle, open_store};
use crate::{TbvolManifest, TbvolReader};
use ophiolite_seismic_io::{Endianness, FileSummary, GeometryReport, SegyReader, TraceSelection};

const SEGY_EXPORT_DIR: &str = "segy-export";
const TEXT_HEADERS_FILE: &str = "text-headers.bin";
const BINARY_HEADER_FILE: &str = "binary-header.bin";
const TRACE_HEADERS_FILE: &str = "trace-headers.bin";
const TRACE_INDEX_FILE: &str = "trace-index.bin";
const TRACE_HEADER_SIZE: usize = 240;
const TEXT_HEADER_SIZE: usize = 3200;
const BINARY_HEADER_SIZE: usize = 400;

const BIN_SAMPLE_INTERVAL_OFFSET: usize = 16;
const BIN_SAMPLE_COUNT_OFFSET: usize = 20;
const BIN_FORMAT_CODE_OFFSET: usize = 24;

const TR_DELAY_RECORDING_TIME_OFFSET: usize = 108;
const TR_TRACE_SAMPLE_COUNT_OFFSET: usize = 114;
const TR_TRACE_SAMPLE_INTERVAL_OFFSET: usize = 116;
const TR_DELAY_SCALAR_OFFSET: usize = 214;

#[derive(Debug, Clone, Copy)]
struct TraceIndexEntry {
    inline_index: u32,
    xline_index: u32,
}

#[derive(Debug, Clone)]
struct SegyExportBundle {
    text_headers: Vec<u8>,
    binary_header: Vec<u8>,
    trace_headers: Vec<u8>,
    trace_index: Vec<TraceIndexEntry>,
    descriptor: SegyExportDescriptor,
}

pub fn capture_store_segy_export(
    segy_path: &Path,
    summary: &FileSummary,
    reader: &SegyReader,
    geometry_report: &GeometryReport,
    contains_synthetic_traces: bool,
) -> Result<SegyExportDescriptor, SeismicStoreError> {
    let bundle = build_export_bundle(
        segy_path,
        summary,
        reader,
        geometry_report,
        contains_synthetic_traces,
    )?;
    Ok(bundle.descriptor)
}

pub fn attach_store_segy_export(
    store_root: &Path,
    segy_path: &Path,
    summary: &FileSummary,
    reader: &SegyReader,
    geometry_report: &GeometryReport,
    contains_synthetic_traces: bool,
) -> Result<SegyExportDescriptor, SeismicStoreError> {
    let bundle = build_export_bundle(
        segy_path,
        summary,
        reader,
        geometry_report,
        contains_synthetic_traces,
    )?;
    write_export_bundle(store_root, &bundle)?;
    Ok(bundle.descriptor)
}

pub fn copy_store_segy_export(
    input_store_root: &Path,
    output_store_root: &Path,
) -> Result<Option<SegyExportDescriptor>, SeismicStoreError> {
    let Some(bundle) = read_export_bundle(input_store_root)? else {
        return Ok(None);
    };
    write_export_bundle(output_store_root, &bundle)?;
    Ok(Some(bundle.descriptor))
}

pub fn crop_store_segy_export(
    input_store_root: &Path,
    output_store_root: &Path,
    inline_start: usize,
    inline_end_exclusive: usize,
    xline_start: usize,
    xline_end_exclusive: usize,
) -> Result<Option<SegyExportDescriptor>, SeismicStoreError> {
    let Some(bundle) = read_export_bundle(input_store_root)? else {
        return Ok(None);
    };

    let mut cropped_trace_index = Vec::new();
    let mut cropped_trace_headers = Vec::with_capacity(
        bundle
            .trace_headers
            .len()
            .min(bundle.trace_index.len() * TRACE_HEADER_SIZE),
    );
    for (trace_idx, entry) in bundle.trace_index.iter().enumerate() {
        let inline_index = entry.inline_index as usize;
        let xline_index = entry.xline_index as usize;
        if inline_index < inline_start
            || inline_index >= inline_end_exclusive
            || xline_index < xline_start
            || xline_index >= xline_end_exclusive
        {
            continue;
        }
        cropped_trace_index.push(TraceIndexEntry {
            inline_index: (inline_index - inline_start) as u32,
            xline_index: (xline_index - xline_start) as u32,
        });
        let start = trace_idx * TRACE_HEADER_SIZE;
        let end = start + TRACE_HEADER_SIZE;
        cropped_trace_headers.extend_from_slice(&bundle.trace_headers[start..end]);
    }

    let mut descriptor = bundle.descriptor.clone();
    descriptor.trace_count = cropped_trace_index.len();
    let cropped = SegyExportBundle {
        text_headers: bundle.text_headers,
        binary_header: bundle.binary_header,
        trace_headers: cropped_trace_headers,
        trace_index: cropped_trace_index,
        descriptor,
    };
    write_export_bundle(output_store_root, &cropped)?;
    Ok(Some(cropped.descriptor))
}

pub fn export_store_to_segy(
    store_root: &Path,
    output_path: &Path,
    overwrite_existing: bool,
) -> Result<(), SeismicStoreError> {
    prepare_output_path(output_path, overwrite_existing)?;
    let handle = open_store(store_root)?;
    let reader = TbvolReader::open(store_root)?;
    let bundle = read_export_bundle(store_root)?.ok_or_else(|| {
        SeismicStoreError::Message("store does not carry SEG-Y export provenance".to_string())
    })?;
    if bundle.descriptor.contains_synthetic_traces {
        return Err(SeismicStoreError::Message(
            "store contains synthetic or regularized traces; phase 1 SEG-Y export rejects this dataset"
                .to_string(),
        ));
    }

    let sample_interval_us = resolve_sample_interval_us(&handle);
    let sample_count = handle.manifest.volume.shape[2];
    let sample_start_ms = handle
        .manifest
        .volume
        .axes
        .sample_axis_ms
        .first()
        .copied()
        .unwrap_or_default();
    let output_sample_format_code = resolved_output_sample_format_code(&handle);
    let endianness = parse_endianness(&bundle.descriptor.endianness)?;

    let mut output = File::create(output_path)?;
    output.write_all(&bundle.text_headers)?;
    let mut binary_header = bundle.binary_header.clone();
    put_u16(
        &mut binary_header,
        BIN_SAMPLE_INTERVAL_OFFSET,
        sample_interval_us,
        endianness,
    );
    put_u16(
        &mut binary_header,
        BIN_SAMPLE_COUNT_OFFSET,
        sample_count as u16,
        endianness,
    );
    put_u16(
        &mut binary_header,
        BIN_FORMAT_CODE_OFFSET,
        output_sample_format_code,
        endianness,
    );
    output.write_all(&binary_header)?;

    let mut tile_cache = TraceTileCache::default();
    let patch_trace_sample_count = sample_count != handle.manifest.volume.source.samples_per_trace;
    let patch_trace_sample_interval =
        sample_interval_us != handle.manifest.volume.source.sample_interval_us;
    let patch_delay = if bundle.trace_headers.len() >= TRACE_HEADER_SIZE {
        let original_start_ms =
            decode_delay_ms(&bundle.trace_headers[..TRACE_HEADER_SIZE], endianness);
        (sample_start_ms - original_start_ms).abs() > 0.001
    } else {
        sample_start_ms.abs() > 0.001
    };

    for (trace_idx, entry) in bundle.trace_index.iter().enumerate() {
        let mut trace_header = bundle.trace_headers
            [trace_idx * TRACE_HEADER_SIZE..(trace_idx + 1) * TRACE_HEADER_SIZE]
            .to_vec();
        if patch_trace_sample_count {
            put_u16(
                &mut trace_header,
                TR_TRACE_SAMPLE_COUNT_OFFSET,
                sample_count as u16,
                endianness,
            );
        }
        if patch_trace_sample_interval {
            put_u16(
                &mut trace_header,
                TR_TRACE_SAMPLE_INTERVAL_OFFSET,
                sample_interval_us,
                endianness,
            );
        }
        if patch_delay {
            let delay_ms = rounded_i16(sample_start_ms)?;
            put_i16(
                &mut trace_header,
                TR_DELAY_RECORDING_TIME_OFFSET,
                delay_ms,
                endianness,
            );
            put_i16(&mut trace_header, TR_DELAY_SCALAR_OFFSET, 1, endianness);
        }
        output.write_all(&trace_header)?;
        let trace = tile_cache.trace(
            &reader,
            entry.inline_index as usize,
            entry.xline_index as usize,
        )?;
        write_encoded_trace(&mut output, trace, output_sample_format_code, endianness)?;
    }

    output.flush()?;
    Ok(())
}

fn build_export_bundle(
    segy_path: &Path,
    summary: &FileSummary,
    reader: &SegyReader,
    geometry_report: &GeometryReport,
    contains_synthetic_traces: bool,
) -> Result<SegyExportBundle, SeismicStoreError> {
    let text_headers = summary
        .textual_headers
        .iter()
        .flat_map(|header| header.raw.iter().copied())
        .collect::<Vec<_>>();
    let binary_header = read_binary_header(segy_path)?;
    let trace_headers = read_trace_headers(segy_path, summary)?;
    let trace_index = build_trace_index(reader, geometry_report)?;
    if trace_index.len() * TRACE_HEADER_SIZE != trace_headers.len() {
        return Err(SeismicStoreError::Message(format!(
            "captured trace header byte length {} does not match trace index length {}",
            trace_headers.len(),
            trace_index.len()
        )));
    }
    let descriptor = SegyExportDescriptor {
        schema_version: 1,
        text_headers_path: export_relative_path(TEXT_HEADERS_FILE),
        binary_header_path: export_relative_path(BINARY_HEADER_FILE),
        trace_headers_path: export_relative_path(TRACE_HEADERS_FILE),
        trace_index_path: export_relative_path(TRACE_INDEX_FILE),
        trace_count: trace_index.len(),
        textual_header_count: summary.textual_headers.len(),
        endianness: match summary.endianness {
            Endianness::Big => "big".to_string(),
            Endianness::Little => "little".to_string(),
        },
        contains_synthetic_traces,
    };

    Ok(SegyExportBundle {
        text_headers,
        binary_header,
        trace_headers,
        trace_index,
        descriptor,
    })
}

fn write_export_bundle(
    store_root: &Path,
    bundle: &SegyExportBundle,
) -> Result<(), SeismicStoreError> {
    let export_root = store_root.join(SEGY_EXPORT_DIR);
    if export_root.exists() {
        fs::remove_dir_all(&export_root)?;
    }
    fs::create_dir_all(&export_root)?;
    fs::write(export_root.join(TEXT_HEADERS_FILE), &bundle.text_headers)?;
    fs::write(export_root.join(BINARY_HEADER_FILE), &bundle.binary_header)?;
    fs::write(export_root.join(TRACE_HEADERS_FILE), &bundle.trace_headers)?;
    fs::write(
        export_root.join(TRACE_INDEX_FILE),
        encode_trace_index(&bundle.trace_index),
    )?;
    patch_store_manifest_descriptor(store_root, Some(bundle.descriptor.clone()))
}

fn read_export_bundle(store_root: &Path) -> Result<Option<SegyExportBundle>, SeismicStoreError> {
    let manifest_path = store_root.join("manifest.json");
    let manifest = serde_json::from_slice::<TbvolManifest>(&fs::read(&manifest_path)?)?;
    let Some(descriptor) = manifest.volume.segy_export.clone() else {
        return Ok(None);
    };
    let text_headers = fs::read(store_root.join(&descriptor.text_headers_path))?;
    let binary_header = fs::read(store_root.join(&descriptor.binary_header_path))?;
    let trace_headers = fs::read(store_root.join(&descriptor.trace_headers_path))?;
    let trace_index =
        decode_trace_index(&fs::read(store_root.join(&descriptor.trace_index_path))?)?;
    if trace_index.len() != descriptor.trace_count {
        return Err(SeismicStoreError::Message(format!(
            "captured SEG-Y trace index count {} does not match manifest descriptor count {}",
            trace_index.len(),
            descriptor.trace_count
        )));
    }
    if trace_headers.len() != descriptor.trace_count * TRACE_HEADER_SIZE {
        return Err(SeismicStoreError::Message(format!(
            "captured SEG-Y trace header byte length {} does not match expected {}",
            trace_headers.len(),
            descriptor.trace_count * TRACE_HEADER_SIZE
        )));
    }

    Ok(Some(SegyExportBundle {
        text_headers,
        binary_header,
        trace_headers,
        trace_index,
        descriptor,
    }))
}

fn patch_store_manifest_descriptor(
    store_root: &Path,
    descriptor: Option<SegyExportDescriptor>,
) -> Result<(), SeismicStoreError> {
    let manifest_path = store_root.join("manifest.json");
    let mut manifest = serde_json::from_slice::<TbvolManifest>(&fs::read(&manifest_path)?)?;
    manifest.volume.segy_export = descriptor;
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

fn build_trace_index(
    reader: &SegyReader,
    geometry_report: &GeometryReport,
) -> Result<Vec<TraceIndexEntry>, SeismicStoreError> {
    let headers = reader.load_trace_headers(
        &[
            geometry_report.inline_field,
            geometry_report.crossline_field,
        ],
        TraceSelection::All,
    )?;
    let inline_values = headers
        .column(geometry_report.inline_field)
        .ok_or_else(|| SeismicStoreError::Message("missing inline header column".to_string()))?;
    let xline_values = headers
        .column(geometry_report.crossline_field)
        .ok_or_else(|| SeismicStoreError::Message("missing crossline header column".to_string()))?;
    let inline_lookup = geometry_index_lookup(&geometry_report.inline_values);
    let xline_lookup = geometry_index_lookup(&geometry_report.crossline_values);
    let mut trace_index = Vec::with_capacity(headers.rows());
    for row in 0..headers.rows() {
        trace_index.push(TraceIndexEntry {
            inline_index: *inline_lookup.get(&inline_values[row]).ok_or_else(|| {
                SeismicStoreError::Message(format!(
                    "inline header value {} is outside resolved geometry",
                    inline_values[row]
                ))
            })? as u32,
            xline_index: *xline_lookup.get(&xline_values[row]).ok_or_else(|| {
                SeismicStoreError::Message(format!(
                    "crossline header value {} is outside resolved geometry",
                    xline_values[row]
                ))
            })? as u32,
        });
    }
    Ok(trace_index)
}

fn geometry_index_lookup(values: &[i64]) -> HashMap<i64, usize> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| (*value, index))
        .collect()
}

fn read_binary_header(path: &Path) -> Result<Vec<u8>, SeismicStoreError> {
    let mut file = File::open(path)?;
    let mut binary_header = vec![0_u8; BINARY_HEADER_SIZE];
    file.seek(SeekFrom::Start(TEXT_HEADER_SIZE as u64))?;
    file.read_exact(&mut binary_header)?;
    Ok(binary_header)
}

fn read_trace_headers(path: &Path, summary: &FileSummary) -> Result<Vec<u8>, SeismicStoreError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(summary.first_trace_offset))?;
    let trace_size = summary.trace_size_bytes as usize;
    let trace_count = summary.trace_count as usize;
    let mut raw_trace = vec![0_u8; trace_size];
    let mut trace_headers = vec![0_u8; trace_count * TRACE_HEADER_SIZE];
    for trace_index in 0..trace_count {
        file.read_exact(&mut raw_trace)?;
        let dst_start = trace_index * TRACE_HEADER_SIZE;
        trace_headers[dst_start..dst_start + TRACE_HEADER_SIZE]
            .copy_from_slice(&raw_trace[..TRACE_HEADER_SIZE]);
    }
    Ok(trace_headers)
}

fn encode_trace_index(entries: &[TraceIndexEntry]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(entries.len() * 8);
    for entry in entries {
        bytes.extend_from_slice(&entry.inline_index.to_le_bytes());
        bytes.extend_from_slice(&entry.xline_index.to_le_bytes());
    }
    bytes
}

fn decode_trace_index(bytes: &[u8]) -> Result<Vec<TraceIndexEntry>, SeismicStoreError> {
    if bytes.len() % 8 != 0 {
        return Err(SeismicStoreError::Message(format!(
            "invalid SEG-Y trace index byte length {}",
            bytes.len()
        )));
    }
    Ok(bytes
        .chunks_exact(8)
        .map(|chunk| TraceIndexEntry {
            inline_index: u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
            xline_index: u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
        })
        .collect())
}

fn export_relative_path(file_name: &str) -> String {
    format!("{SEGY_EXPORT_DIR}/{file_name}")
}

fn prepare_output_path(path: &Path, overwrite_existing: bool) -> Result<(), SeismicStoreError> {
    if !path.exists() {
        return Ok(());
    }
    if !overwrite_existing {
        return Err(SeismicStoreError::Message(format!(
            "output SEG-Y path already exists: {}",
            path.display()
        )));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn resolve_sample_interval_us(handle: &StoreHandle) -> u16 {
    let axis = &handle.manifest.volume.axes.sample_axis_ms;
    if axis.len() >= 2 {
        let delta_ms = axis[1] - axis[0];
        if delta_ms.is_finite() && delta_ms > 0.0 {
            return (delta_ms * 1000.0).round().clamp(0.0, u16::MAX as f32) as u16;
        }
    }
    handle.manifest.volume.source.sample_interval_us
}

fn resolved_output_sample_format_code(handle: &StoreHandle) -> u16 {
    let original = handle.manifest.volume.source.sample_format_code;
    match handle.manifest.volume.kind {
        crate::metadata::DatasetKind::Source => original,
        crate::metadata::DatasetKind::Derived => match original {
            1 | 5 => original,
            _ => 5,
        },
    }
}

fn parse_endianness(value: &str) -> Result<Endianness, SeismicStoreError> {
    match value {
        "big" => Ok(Endianness::Big),
        "little" => Ok(Endianness::Little),
        _ => Err(SeismicStoreError::Message(format!(
            "unsupported SEG-Y export endianness '{value}'"
        ))),
    }
}

fn put_u16(buffer: &mut [u8], offset: usize, value: u16, endianness: Endianness) {
    let bytes = match endianness {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    };
    buffer[offset..offset + 2].copy_from_slice(&bytes);
}

fn put_i16(buffer: &mut [u8], offset: usize, value: i16, endianness: Endianness) {
    let bytes = match endianness {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    };
    buffer[offset..offset + 2].copy_from_slice(&bytes);
}

fn read_i16(buffer: &[u8], offset: usize, endianness: Endianness) -> i16 {
    let bytes = [buffer[offset], buffer[offset + 1]];
    match endianness {
        Endianness::Big => i16::from_be_bytes(bytes),
        Endianness::Little => i16::from_le_bytes(bytes),
    }
}

fn decode_delay_ms(trace_header: &[u8], endianness: Endianness) -> f32 {
    let delay = read_i16(trace_header, TR_DELAY_RECORDING_TIME_OFFSET, endianness);
    let scalar = read_i16(trace_header, TR_DELAY_SCALAR_OFFSET, endianness);
    let scale = match scalar {
        0 => 1.0,
        value if value > 0 => value as f32,
        value => 1.0 / (-(value as f32)),
    };
    delay as f32 * scale.abs()
}

fn rounded_i16(value: f32) -> Result<i16, SeismicStoreError> {
    let rounded = value.round();
    if !rounded.is_finite() || rounded < i16::MIN as f32 || rounded > i16::MAX as f32 {
        return Err(SeismicStoreError::Message(format!(
            "SEG-Y delay recording time {value} ms does not fit in i16"
        )));
    }
    Ok(rounded as i16)
}

#[derive(Default)]
struct TraceTileCache {
    tile: Option<TileCoord>,
    values: Vec<f32>,
    tile_shape: [usize; 3],
}

impl TraceTileCache {
    fn trace<'a>(
        &'a mut self,
        reader: &TbvolReader,
        inline_index: usize,
        xline_index: usize,
    ) -> Result<&'a [f32], SeismicStoreError> {
        let tile_shape = reader.tile_geometry().tile_shape();
        let tile = TileCoord {
            tile_i: inline_index / tile_shape[0],
            tile_x: xline_index / tile_shape[1],
        };
        if self.tile != Some(tile) {
            self.values = reader.read_tile(tile)?.into_owned();
            self.tile = Some(tile);
            self.tile_shape = tile_shape;
        }
        let local_i = inline_index % self.tile_shape[0];
        let local_x = xline_index % self.tile_shape[1];
        let start = ((local_i * self.tile_shape[1]) + local_x) * self.tile_shape[2];
        let end = start + self.tile_shape[2];
        Ok(&self.values[start..end])
    }
}

fn write_encoded_trace(
    output: &mut File,
    trace: &[f32],
    sample_format_code: u16,
    endianness: Endianness,
) -> Result<(), SeismicStoreError> {
    match sample_format_code {
        1 => {
            let mut bytes = vec![0_u8; trace.len() * 4];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                let encoded = encode_ibm32(sample);
                let src = match endianness {
                    Endianness::Big => encoded.to_be_bytes(),
                    Endianness::Little => encoded.to_le_bytes(),
                };
                let dst_start = sample_index * 4;
                bytes[dst_start..dst_start + 4].copy_from_slice(&src);
            }
            output.write_all(&bytes)?;
        }
        2 => {
            let mut bytes = vec![0_u8; trace.len() * 4];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                let encoded = clamped_round_i32(sample);
                let src = match endianness {
                    Endianness::Big => encoded.to_be_bytes(),
                    Endianness::Little => encoded.to_le_bytes(),
                };
                let dst_start = sample_index * 4;
                bytes[dst_start..dst_start + 4].copy_from_slice(&src);
            }
            output.write_all(&bytes)?;
        }
        3 => {
            let mut bytes = vec![0_u8; trace.len() * 2];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                let encoded = clamped_round_i16(sample);
                let src = match endianness {
                    Endianness::Big => encoded.to_be_bytes(),
                    Endianness::Little => encoded.to_le_bytes(),
                };
                let dst_start = sample_index * 2;
                bytes[dst_start..dst_start + 2].copy_from_slice(&src);
            }
            output.write_all(&bytes)?;
        }
        5 => {
            let mut bytes = vec![0_u8; trace.len() * 4];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                let src = match endianness {
                    Endianness::Big => sample.to_be_bytes(),
                    Endianness::Little => sample.to_le_bytes(),
                };
                let dst_start = sample_index * 4;
                bytes[dst_start..dst_start + 4].copy_from_slice(&src);
            }
            output.write_all(&bytes)?;
        }
        8 => {
            let mut bytes = vec![0_u8; trace.len()];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                bytes[sample_index] = clamped_round_i8(sample) as u8;
            }
            output.write_all(&bytes)?;
        }
        10 => {
            let mut bytes = vec![0_u8; trace.len() * 4];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                let encoded = clamped_round_u32(sample);
                let src = match endianness {
                    Endianness::Big => encoded.to_be_bytes(),
                    Endianness::Little => encoded.to_le_bytes(),
                };
                let dst_start = sample_index * 4;
                bytes[dst_start..dst_start + 4].copy_from_slice(&src);
            }
            output.write_all(&bytes)?;
        }
        11 => {
            let mut bytes = vec![0_u8; trace.len() * 2];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                let encoded = clamped_round_u16(sample);
                let src = match endianness {
                    Endianness::Big => encoded.to_be_bytes(),
                    Endianness::Little => encoded.to_le_bytes(),
                };
                let dst_start = sample_index * 2;
                bytes[dst_start..dst_start + 2].copy_from_slice(&src);
            }
            output.write_all(&bytes)?;
        }
        16 => {
            let mut bytes = vec![0_u8; trace.len()];
            for (sample_index, sample) in trace.iter().copied().enumerate() {
                bytes[sample_index] = clamped_round_u8(sample);
            }
            output.write_all(&bytes)?;
        }
        unsupported => {
            return Err(SeismicStoreError::Message(format!(
                "SEG-Y export does not support sample format code {unsupported}"
            )));
        }
    }
    Ok(())
}

fn clamped_round_i32(value: f32) -> i32 {
    value.round().clamp(i32::MIN as f32, i32::MAX as f32) as i32
}

fn clamped_round_i16(value: f32) -> i16 {
    value.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn clamped_round_i8(value: f32) -> i8 {
    value.round().clamp(i8::MIN as f32, i8::MAX as f32) as i8
}

fn clamped_round_u32(value: f32) -> u32 {
    value.round().clamp(0.0, u32::MAX as f32) as u32
}

fn clamped_round_u16(value: f32) -> u16 {
    value.round().clamp(0.0, u16::MAX as f32) as u16
}

fn clamped_round_u8(value: f32) -> u8 {
    value.round().clamp(0.0, u8::MAX as f32) as u8
}

fn encode_ibm32(value: f32) -> u32 {
    if value == 0.0 || !value.is_finite() {
        return 0;
    }
    let sign = if value.is_sign_negative() {
        0x8000_0000
    } else {
        0
    };
    let mut magnitude = value.abs() as f64;
    let mut exponent: i32 = 64;
    while magnitude < 0.0625 {
        magnitude *= 16.0;
        exponent -= 1;
    }
    while magnitude >= 1.0 {
        magnitude /= 16.0;
        exponent += 1;
    }
    let fraction = (magnitude * 16_777_216.0).round() as u32 & 0x00ff_ffff;
    sign | ((exponent as u32) << 24) | fraction
}
