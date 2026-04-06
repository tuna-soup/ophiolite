use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::thread;

use memmap2::Mmap;
use ophiolite_seismic::{
    SeismicGatherAxisKind, SeismicLayout, SeismicOrganization, SeismicStackingState,
};
use rayon::prelude::*;

use crate::{
    Endianness, FileSummary, InspectError, InspectOptions, SampleFormat, SampleIntervalSource,
    SegyWarning, inspect_file_with_options,
};

const TRACE_HEADER_SIZE: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationMode {
    #[default]
    Strict,
    Lenient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderOptions {
    pub inspect_options: InspectOptions,
    pub validation_mode: ValidationMode,
    pub header_mapping: HeaderMapping,
    pub interval_options: IntervalOptions,
}

impl Default for ReaderOptions {
    fn default() -> Self {
        Self {
            inspect_options: InspectOptions::default(),
            validation_mode: ValidationMode::Strict,
            header_mapping: HeaderMapping::default(),
            interval_options: IntervalOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleIntervalUnit {
    Microseconds,
    Milliseconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IntervalOptions {
    pub unit_override: Option<SampleIntervalUnit>,
    pub enable_lenient_ms_guess: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoStrategy {
    #[default]
    Auto,
    Stream,
    Mmap,
}

#[derive(Debug)]
pub enum ReadError {
    Inspect(InspectError),
    Io(std::io::Error),
    InvalidSelection {
        start: u64,
        end: u64,
        trace_count: u64,
    },
    InvalidChunkSize,
    InvalidDestinationBuffer {
        actual_len: usize,
        expected_len: usize,
    },
    UnsupportedSampleFormat {
        sample_format: SampleFormat,
    },
    TraceCountOverflow,
    InconsistentSampleCount {
        binary_header: u16,
        trace_header: u16,
    },
    InconsistentSampleInterval {
        binary_header: i16,
        trace_header: i16,
    },
    UnableToResolveSampleInterval,
    SampleIntervalOverflow {
        raw_value: i16,
        unit: SampleIntervalUnit,
    },
    DuplicateTraceCoordinate {
        inline: i64,
        crossline: i64,
        offset: i64,
    },
    IrregularGeometry {
        trace_count: usize,
        expected_trace_count: usize,
    },
    MissingGeometryField {
        field: &'static str,
    },
}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inspect(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::InvalidSelection {
                start,
                end,
                trace_count,
            } => write!(
                f,
                "invalid trace selection [{start}, {end}) for file with {trace_count} traces"
            ),
            Self::InvalidChunkSize => write!(f, "chunk size must be greater than zero"),
            Self::InvalidDestinationBuffer {
                actual_len,
                expected_len,
            } => write!(
                f,
                "destination buffer has length {actual_len}, expected {expected_len}"
            ),
            Self::UnsupportedSampleFormat { sample_format } => {
                write!(
                    f,
                    "sample format {:?} is not yet supported for data reads",
                    sample_format
                )
            }
            Self::TraceCountOverflow => write!(f, "trace count overflow while materializing data"),
            Self::InconsistentSampleCount {
                binary_header,
                trace_header,
            } => write!(
                f,
                "binary-header sample count {binary_header} disagrees with first-trace sample count {trace_header}"
            ),
            Self::InconsistentSampleInterval {
                binary_header,
                trace_header,
            } => write!(
                f,
                "binary-header sample interval {binary_header} disagrees with first-trace interval {trace_header}"
            ),
            Self::UnableToResolveSampleInterval => {
                write!(
                    f,
                    "unable to resolve a positive sample interval from SEG-Y headers"
                )
            }
            Self::SampleIntervalOverflow { raw_value, unit } => {
                write!(
                    f,
                    "sample interval value {raw_value} in {:?} does not fit in the supported microsecond range",
                    unit
                )
            }
            Self::DuplicateTraceCoordinate {
                inline,
                crossline,
                offset,
            } => write!(
                f,
                "duplicate trace coordinate encountered at inline={inline}, crossline={crossline}, offset={offset}"
            ),
            Self::IrregularGeometry {
                trace_count,
                expected_trace_count,
            } => write!(
                f,
                "geometry is not a complete regular cube: found {trace_count} traces, expected {expected_trace_count}"
            ),
            Self::MissingGeometryField { field } => {
                write!(
                    f,
                    "missing geometry field {field} required for cube assembly"
                )
            }
        }
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inspect(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<InspectError> for ReadError {
    fn from(value: InspectError) -> Self {
        Self::Inspect(value)
    }
}

impl From<std::io::Error> for ReadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryTraceHeader {
    pub sample_count: u16,
    pub sample_interval_us: i16,
    pub delay_recording_time_ms: i16,
    pub delay_scalar: i16,
}

impl PrimaryTraceHeader {
    pub fn delay_scale(self) -> f32 {
        match self.delay_scalar {
            0 => 1.0,
            value if value > 0 => value as f32,
            value => 1.0 / (-(value as f32)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegyReader {
    path: PathBuf,
    summary: FileSummary,
    validation_mode: ValidationMode,
    header_mapping: HeaderMapping,
    primary_trace_header: PrimaryTraceHeader,
    resolved_sample_interval_us: u16,
    warnings: Vec<SegyWarning>,
    mapped: Option<Arc<Mmap>>,
}

impl SegyReader {
    pub fn open(path: impl AsRef<Path>, options: ReaderOptions) -> Result<Self, ReadError> {
        let path = path.as_ref();
        let summary = inspect_file_with_options(path, options.inspect_options)?;
        let primary_trace_header =
            read_primary_trace_header(path, summary.first_trace_offset, summary.endianness)?;
        let mut warnings = summary.warnings.clone();
        push_sample_count_warning(
            &mut warnings,
            summary.samples_per_trace,
            primary_trace_header.sample_count,
        );
        let resolved_sample_interval_us = resolve_sample_interval_us(
            summary.sample_interval_us as i16,
            primary_trace_header.sample_interval_us,
            options.validation_mode,
            options.interval_options,
            &mut warnings,
        )?;
        let mapped = map_file(path).ok();

        Ok(Self {
            path: path.to_path_buf(),
            summary,
            validation_mode: options.validation_mode,
            header_mapping: options.header_mapping,
            primary_trace_header,
            resolved_sample_interval_us,
            warnings,
            mapped,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn summary(&self) -> &FileSummary {
        &self.summary
    }

    pub fn validation_mode(&self) -> ValidationMode {
        self.validation_mode
    }

    pub fn header_mapping(&self) -> &HeaderMapping {
        &self.header_mapping
    }

    pub fn primary_trace_header(&self) -> PrimaryTraceHeader {
        self.primary_trace_header
    }

    pub fn resolved_sample_interval_us(&self) -> u16 {
        self.resolved_sample_interval_us
    }

    pub fn warnings(&self) -> &[SegyWarning] {
        &self.warnings
    }

    pub fn analyze_geometry(&self, options: GeometryOptions) -> Result<GeometryReport, ReadError> {
        let inline_field = options.resolve_inline_field(&self.header_mapping);
        let crossline_field = options.resolve_crossline_field(&self.header_mapping);

        let mut fields = vec![inline_field, crossline_field];
        if let Some(third_axis_field) = options.third_axis_field {
            fields.push(third_axis_field);
        }

        let headers = self.load_trace_headers(&fields, TraceSelection::All)?;
        analyze_geometry_headers(
            &headers,
            inline_field,
            crossline_field,
            options.third_axis_field,
        )
    }

    pub fn sample_axis_ms(&self) -> Vec<f32> {
        let start_ms = self.primary_trace_header.delay_recording_time_ms as f32
            * self.primary_trace_header.delay_scale().abs();
        let dt_ms = self.resolved_sample_interval_us as f32 / 1000.0;
        (0..self.summary.samples_per_trace as usize)
            .map(|i| start_ms + i as f32 * dt_ms)
            .collect()
    }

    pub fn load_trace_headers(
        &self,
        fields: &[HeaderField],
        selection: TraceSelection,
    ) -> Result<HeaderTable, ReadError> {
        self.load_trace_headers_with_config(
            fields,
            HeaderLoadConfig {
                selection,
                ..HeaderLoadConfig::default()
            },
        )
    }

    pub fn load_trace_headers_with_config(
        &self,
        fields: &[HeaderField],
        config: HeaderLoadConfig,
    ) -> Result<HeaderTable, ReadError> {
        load_trace_headers_from_reader(self, fields, config)
    }

    pub fn read_trace_chunks(&self, config: ChunkReadConfig) -> Result<TraceChunkIter, ReadError> {
        TraceChunkIter::new(
            self.path.clone(),
            self.summary.clone(),
            self.mapped.clone(),
            config,
        )
    }

    pub fn trace_block_layout(
        &self,
        selection: TraceSelection,
    ) -> Result<TraceBlockInfo, ReadError> {
        let (start, end) = selection.resolve(self.summary.trace_count)?;
        Ok(TraceBlockInfo {
            start_trace: start,
            trace_count: usize::try_from(end - start).map_err(|_| ReadError::TraceCountOverflow)?,
            samples_per_trace: self.summary.samples_per_trace as usize,
        })
    }

    pub fn read_all_traces_into(
        &self,
        config: ChunkReadConfig,
        dst: &mut [f32],
    ) -> Result<TraceBlockInfo, ReadError> {
        let layout = self.trace_block_layout(config.selection)?;
        let expected_len = layout.trace_count * layout.samples_per_trace;
        if dst.len() != expected_len {
            return Err(ReadError::InvalidDestinationBuffer {
                actual_len: dst.len(),
                expected_len,
            });
        }

        read_all_traces_into_buffer(self, config, dst)?;
        Ok(layout)
    }

    pub fn read_trace_into(
        &self,
        trace_index: u64,
        io_strategy: IoStrategy,
        dst: &mut [f32],
    ) -> Result<(), ReadError> {
        let samples_per_trace = self.summary.samples_per_trace as usize;
        if dst.len() != samples_per_trace {
            return Err(ReadError::InvalidDestinationBuffer {
                actual_len: dst.len(),
                expected_len: samples_per_trace,
            });
        }
        if trace_index >= self.summary.trace_count {
            return Err(ReadError::InvalidSelection {
                start: trace_index,
                end: trace_index + 1,
                trace_count: self.summary.trace_count,
            });
        }

        let config = ChunkReadConfig {
            traces_per_chunk: 1,
            selection: TraceSelection::Range {
                start: trace_index,
                end: trace_index + 1,
            },
            parallel_decode: false,
            io_strategy,
        };
        read_all_traces_into_buffer(self, config, dst)
    }

    pub fn read_trace(
        &self,
        trace_index: u64,
        io_strategy: IoStrategy,
    ) -> Result<Vec<f32>, ReadError> {
        let mut data = vec![0.0_f32; self.summary.samples_per_trace as usize];
        self.read_trace_into(trace_index, io_strategy, &mut data)?;
        Ok(data)
    }

    pub fn read_all_traces(&self, config: ChunkReadConfig) -> Result<TraceBlock<f32>, ReadError> {
        let layout = self.trace_block_layout(config.selection)?;
        let mut data = vec![0_f32; layout.trace_count.saturating_mul(layout.samples_per_trace)];
        self.read_all_traces_into(config, &mut data)?;

        Ok(TraceBlock {
            start_trace: layout.start_trace,
            trace_count: layout.trace_count,
            samples_per_trace: layout.samples_per_trace,
            data,
        })
    }

    pub fn process_trace_chunks_into<E, F>(
        &self,
        config: ChunkReadConfig,
        scratch: &mut [f32],
        mut process: F,
    ) -> Result<(), ChunkProcessingError<E>>
    where
        F: FnMut(TraceChunkRef<'_, f32>) -> Result<(), E>,
    {
        if config.traces_per_chunk == 0 {
            return Err(ChunkProcessingError::Read(ReadError::InvalidChunkSize));
        }

        let layout = self
            .trace_block_layout(config.selection)
            .map_err(ChunkProcessingError::Read)?;
        if layout.trace_count == 0 {
            return Ok(());
        }

        let scratch_traces = validate_chunk_scratch(scratch, layout.samples_per_trace)
            .map_err(ChunkProcessingError::Read)?;
        let chunk_traces = scratch_traces.min(config.traces_per_chunk);
        let chunk_config = ChunkReadConfig {
            traces_per_chunk: chunk_traces,
            ..config
        };

        if should_try_mmap(chunk_config.io_strategy) {
            let mut processed_traces = 0usize;
            while processed_traces < layout.trace_count {
                let traces_in_chunk = (layout.trace_count - processed_traces).min(chunk_traces);
                let chunk_start = layout.start_trace + processed_traces as u64;
                let dst = &mut scratch[..traces_in_chunk * layout.samples_per_trace];
                let mmap_result = with_trace_bytes(
                    self,
                    self.summary.first_trace_offset,
                    self.summary.trace_size_bytes,
                    chunk_start,
                    traces_in_chunk,
                    |raw| {
                        decode_trace_chunk_into(
                            raw,
                            self.summary.sample_format,
                            self.summary.endianness,
                            layout.samples_per_trace,
                            dst,
                            chunk_config.parallel_decode,
                        )
                    },
                );

                match mmap_result {
                    Ok(()) => {
                        process(TraceChunkRef {
                            start_trace: chunk_start,
                            trace_count: traces_in_chunk,
                            samples_per_trace: layout.samples_per_trace,
                            data: dst,
                        })
                        .map_err(ChunkProcessingError::Sink)?;
                        processed_traces += traces_in_chunk;
                    }
                    Err(error) if matches!(chunk_config.io_strategy, IoStrategy::Mmap) => {
                        return Err(ChunkProcessingError::Read(error));
                    }
                    Err(_) => break,
                }
            }

            if processed_traces == layout.trace_count {
                return Ok(());
            }
        }

        process_trace_chunks_streaming(self, chunk_config, layout, scratch, process)
    }

    pub fn assemble_cube(&self) -> Result<Cube<f32>, ReadError> {
        let inline_field = self.header_mapping.inline_3d();
        let crossline_field = self.header_mapping.crossline_3d();
        let offset_field = self.header_mapping.offset();
        let headers = self.load_trace_headers(
            &[inline_field, crossline_field, offset_field],
            TraceSelection::All,
        )?;
        let plan = build_cube_plan(&headers, inline_field, crossline_field, offset_field)?;
        let samples_per_trace = self.summary.samples_per_trace as usize;
        let mut cube_data = vec![0_f32; headers.rows() * samples_per_trace];
        fill_cube_data_from_reader(self, &plan.slot_indices, &mut cube_data)?;

        Ok(Cube {
            ilines: plan.ilines,
            xlines: plan.xlines,
            offsets: plan.offsets,
            samples_per_trace,
            sample_interval_us: self.resolved_sample_interval_us,
            sample_axis_ms: self.sample_axis_ms(),
            data: cube_data,
        })
    }
}

pub fn open(path: impl AsRef<Path>, options: ReaderOptions) -> Result<SegyReader, ReadError> {
    SegyReader::open(path, options)
}

pub fn load_trace_headers(
    path: impl AsRef<Path>,
    fields: &[HeaderField],
    selection: TraceSelection,
    options: ReaderOptions,
) -> Result<HeaderTable, ReadError> {
    load_trace_headers_with_config(
        path,
        fields,
        HeaderLoadConfig {
            selection,
            ..HeaderLoadConfig::default()
        },
        options,
    )
}

pub fn load_trace_headers_with_config(
    path: impl AsRef<Path>,
    fields: &[HeaderField],
    config: HeaderLoadConfig,
    options: ReaderOptions,
) -> Result<HeaderTable, ReadError> {
    let reader = open(path, options)?;
    load_trace_headers_from_reader(&reader, fields, config)
}

fn load_trace_headers_from_reader(
    reader: &SegyReader,
    fields: &[HeaderField],
    config: HeaderLoadConfig,
) -> Result<HeaderTable, ReadError> {
    if config.traces_per_chunk == 0 {
        return Err(ReadError::InvalidChunkSize);
    }
    let (start, end) = config.selection.resolve(reader.summary.trace_count)?;
    let selected = usize::try_from(end - start).map_err(|_| ReadError::TraceCountOverflow)?;
    let trace_size = usize::try_from(reader.summary.trace_size_bytes)
        .map_err(|_| ReadError::TraceCountOverflow)?;

    if should_try_mmap(config.io_strategy) {
        let mmap_result = with_trace_bytes(
            reader,
            reader.summary.first_trace_offset,
            reader.summary.trace_size_bytes,
            start,
            selected,
            |raw| {
                Ok(extract_headers_from_trace_bytes(
                    raw,
                    fields,
                    start,
                    trace_size,
                    reader.summary.endianness,
                    config.parallel_extract,
                ))
            },
        );

        match mmap_result {
            Ok(table) => return Ok(table),
            Err(error) if matches!(config.io_strategy, IoStrategy::Mmap) => {
                return Err(error);
            }
            Err(_) => {}
        }
    }

    let mut file = File::open(&reader.path)?;
    file.seek(SeekFrom::Start(
        reader.summary.first_trace_offset + start * reader.summary.trace_size_bytes,
    ))?;

    let mut columns = empty_header_columns(fields, selected);
    let mut trace_numbers = Vec::with_capacity(selected);
    let mut raw_buffer = vec![0_u8; config.traces_per_chunk.saturating_mul(trace_size)];
    let mut processed = 0usize;
    while processed < selected {
        let traces_in_chunk = (selected - processed).min(config.traces_per_chunk);
        let raw_len = traces_in_chunk * trace_size;
        file.read_exact(&mut raw_buffer[..raw_len])?;

        for column in &mut columns {
            let values = extract_header_values(
                &raw_buffer[..raw_len],
                trace_size,
                column.field,
                reader.summary.endianness,
                config.parallel_extract,
            );
            column.values.extend(values);
        }

        let chunk_start = start + processed as u64;
        trace_numbers.extend((0..traces_in_chunk).map(|i| chunk_start + i as u64));
        processed += traces_in_chunk;
    }

    Ok(HeaderTable {
        trace_numbers,
        columns,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderLoadConfig {
    pub selection: TraceSelection,
    pub traces_per_chunk: usize,
    pub parallel_extract: bool,
    pub io_strategy: IoStrategy,
}

impl Default for HeaderLoadConfig {
    fn default() -> Self {
        Self {
            selection: TraceSelection::All,
            traces_per_chunk: 4096,
            parallel_extract: true,
            io_strategy: IoStrategy::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderValueType {
    I16,
    I32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderField {
    pub name: &'static str,
    pub start_byte: u16,
    pub value_type: HeaderValueType,
}

impl HeaderField {
    pub const FIELD_RECORD: Self = Self::new_i32("FIELD_RECORD", 9);
    pub const CDP_X: Self = Self::new_i32("CDP_X", 181);
    pub const CDP_Y: Self = Self::new_i32("CDP_Y", 185);
    pub const INLINE_3D: Self = Self::new_i32("INLINE_3D", 189);
    pub const CROSSLINE_3D: Self = Self::new_i32("CROSSLINE_3D", 193);
    pub const OFFSET: Self = Self::new_i32("OFFSET", 37);
    pub const TRACE_SAMPLE_COUNT: Self = Self::new_i16("TRACE_SAMPLE_COUNT", 115);
    pub const TRACE_SAMPLE_INTERVAL: Self = Self::new_i16("TRACE_SAMPLE_INTERVAL", 117);
    pub const DELAY_RECORDING_TIME: Self = Self::new_i16("DELAY_RECORDING_TIME", 109);
    pub const DELAY_SCALAR: Self = Self::new_i16("DELAY_SCALAR", 215);

    pub const fn new_i16(name: &'static str, start_byte: u16) -> Self {
        Self {
            name,
            start_byte,
            value_type: HeaderValueType::I16,
        }
    }

    pub const fn new_i32(name: &'static str, start_byte: u16) -> Self {
        Self {
            name,
            start_byte,
            value_type: HeaderValueType::I32,
        }
    }

    fn read(self, header: &[u8; TRACE_HEADER_SIZE], endianness: Endianness) -> i64 {
        self.read_from_slice(header, endianness)
    }

    fn read_from_slice(self, header: &[u8], endianness: Endianness) -> i64 {
        let offset = self.start_byte as usize - 1;
        match self.value_type {
            HeaderValueType::I16 => {
                let bytes = [header[offset], header[offset + 1]];
                match endianness {
                    Endianness::Big => i16::from_be_bytes(bytes) as i64,
                    Endianness::Little => i16::from_le_bytes(bytes) as i64,
                }
            }
            HeaderValueType::I32 => {
                let bytes = [
                    header[offset],
                    header[offset + 1],
                    header[offset + 2],
                    header[offset + 3],
                ];
                match endianness {
                    Endianness::Big => i32::from_be_bytes(bytes) as i64,
                    Endianness::Little => i32::from_le_bytes(bytes) as i64,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeaderMapping {
    pub inline_3d: Option<HeaderField>,
    pub crossline_3d: Option<HeaderField>,
    pub offset: Option<HeaderField>,
    pub cdp_x: Option<HeaderField>,
    pub cdp_y: Option<HeaderField>,
    pub field_record: Option<HeaderField>,
    pub custom_fields: Vec<HeaderField>,
}

impl HeaderMapping {
    pub fn inline_3d(&self) -> HeaderField {
        self.inline_3d.unwrap_or(HeaderField::INLINE_3D)
    }

    pub fn crossline_3d(&self) -> HeaderField {
        self.crossline_3d.unwrap_or(HeaderField::CROSSLINE_3D)
    }

    pub fn offset(&self) -> HeaderField {
        self.offset.unwrap_or(HeaderField::OFFSET)
    }

    pub fn cdp_x(&self) -> HeaderField {
        self.cdp_x.unwrap_or(HeaderField::CDP_X)
    }

    pub fn cdp_y(&self) -> HeaderField {
        self.cdp_y.unwrap_or(HeaderField::CDP_Y)
    }

    pub fn field_record(&self) -> HeaderField {
        self.field_record.unwrap_or(HeaderField::FIELD_RECORD)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GeometryOptions {
    pub inline_field: Option<HeaderField>,
    pub crossline_field: Option<HeaderField>,
    pub third_axis_field: Option<HeaderField>,
}

impl GeometryOptions {
    fn resolve_inline_field(self, mapping: &HeaderMapping) -> HeaderField {
        self.inline_field.unwrap_or_else(|| mapping.inline_3d())
    }

    fn resolve_crossline_field(self, mapping: &HeaderMapping) -> HeaderField {
        self.crossline_field
            .unwrap_or_else(|| mapping.crossline_3d())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryClassification {
    RegularDense,
    RegularSparse,
    DuplicateCoordinates,
    NonCartesian,
    AmbiguousMapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeometryCoordinate {
    pub inline: i64,
    pub crossline: i64,
    pub third_axis: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryReport {
    pub inline_field: HeaderField,
    pub crossline_field: HeaderField,
    pub third_axis_field: Option<HeaderField>,
    pub stacking_state: SeismicStackingState,
    pub organization: SeismicOrganization,
    pub layout: SeismicLayout,
    pub gather_axis_kind: Option<SeismicGatherAxisKind>,
    pub inline_values: Vec<i64>,
    pub crossline_values: Vec<i64>,
    pub third_axis_values: Vec<i64>,
    pub observed_trace_count: usize,
    pub unique_coordinate_count: usize,
    pub expected_trace_count: usize,
    pub completeness_ratio: f64,
    pub missing_bin_count: usize,
    pub duplicate_coordinate_count: usize,
    pub duplicate_examples: Vec<GeometryCoordinate>,
    pub classification: GeometryClassification,
}

impl GeometryReport {
    pub fn is_dense_regular(&self) -> bool {
        matches!(self.classification, GeometryClassification::RegularDense)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderColumn {
    pub field: HeaderField,
    pub values: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderTable {
    pub trace_numbers: Vec<u64>,
    pub columns: Vec<HeaderColumn>,
}

impl HeaderTable {
    pub fn rows(&self) -> usize {
        self.trace_numbers.len()
    }

    pub fn column(&self, field: HeaderField) -> Option<&[i64]> {
        self.columns
            .iter()
            .find(|column| column.field == field)
            .map(|column| column.values.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceSelection {
    All,
    Range { start: u64, end: u64 },
}

impl Default for TraceSelection {
    fn default() -> Self {
        Self::All
    }
}

impl TraceSelection {
    pub(crate) fn resolve(self, trace_count: u64) -> Result<(u64, u64), ReadError> {
        let (start, end) = match self {
            Self::All => (0, trace_count),
            Self::Range { start, end } => (start, end),
        };

        if start > end || end > trace_count {
            return Err(ReadError::InvalidSelection {
                start,
                end,
                trace_count,
            });
        }

        Ok((start, end))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkReadConfig {
    pub traces_per_chunk: usize,
    pub selection: TraceSelection,
    pub parallel_decode: bool,
    pub io_strategy: IoStrategy,
}

impl Default for ChunkReadConfig {
    fn default() -> Self {
        Self {
            traces_per_chunk: 512,
            selection: TraceSelection::All,
            parallel_decode: true,
            io_strategy: IoStrategy::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceChunk<T> {
    pub start_trace: u64,
    pub samples_per_trace: usize,
    pub data: Vec<T>,
}

impl<T> TraceChunk<T> {
    pub fn trace_count(&self) -> usize {
        if self.samples_per_trace == 0 {
            0
        } else {
            self.data.len() / self.samples_per_trace
        }
    }

    pub fn trace(&self, trace_index: usize) -> &[T] {
        let start = trace_index * self.samples_per_trace;
        let end = start + self.samples_per_trace;
        &self.data[start..end]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceBlock<T> {
    pub start_trace: u64,
    pub trace_count: usize,
    pub samples_per_trace: usize,
    pub data: Vec<T>,
}

impl<T> TraceBlock<T> {
    pub fn trace(&self, trace_index: usize) -> &[T] {
        let start = trace_index * self.samples_per_trace;
        let end = start + self.samples_per_trace;
        &self.data[start..end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceBlockInfo {
    pub start_trace: u64,
    pub trace_count: usize,
    pub samples_per_trace: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceChunkRef<'a, T> {
    pub start_trace: u64,
    pub trace_count: usize,
    pub samples_per_trace: usize,
    pub data: &'a [T],
}

impl<'a, T> TraceChunkRef<'a, T> {
    pub fn trace(&self, trace_index: usize) -> &[T] {
        let start = trace_index * self.samples_per_trace;
        let end = start + self.samples_per_trace;
        &self.data[start..end]
    }
}

#[derive(Debug)]
pub enum ChunkProcessingError<E> {
    Read(ReadError),
    Sink(E),
}

impl<E: Display> Display for ChunkProcessingError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(f, "{error}"),
            Self::Sink(error) => write!(f, "{error}"),
        }
    }
}

impl<E: Error + 'static> Error for ChunkProcessingError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Sink(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cube<T> {
    pub ilines: Vec<i64>,
    pub xlines: Vec<i64>,
    pub offsets: Vec<i64>,
    pub samples_per_trace: usize,
    pub sample_interval_us: u16,
    pub sample_axis_ms: Vec<f32>,
    pub data: Vec<T>,
}

impl<T> Cube<T> {
    pub fn dimensions(&self) -> (usize, usize, usize, usize) {
        (
            self.ilines.len(),
            self.xlines.len(),
            self.offsets.len(),
            self.samples_per_trace,
        )
    }

    pub fn trace(&self, inline_index: usize, xline_index: usize, offset_index: usize) -> &[T] {
        let trace_index =
            ((inline_index * self.xlines.len() + xline_index) * self.offsets.len()) + offset_index;
        let start = trace_index * self.samples_per_trace;
        let end = start + self.samples_per_trace;
        &self.data[start..end]
    }
}

#[derive(Debug)]
pub struct TraceChunkIter {
    file: Option<File>,
    mapped: Option<Arc<Mmap>>,
    summary: FileSummary,
    next_trace: u64,
    end_trace: u64,
    traces_per_chunk: usize,
    parallel_decode: bool,
    raw_buffer: Vec<u8>,
}

impl TraceChunkIter {
    fn new(
        path: PathBuf,
        summary: FileSummary,
        mapped: Option<Arc<Mmap>>,
        config: ChunkReadConfig,
    ) -> Result<Self, ReadError> {
        if config.traces_per_chunk == 0 {
            return Err(ReadError::InvalidChunkSize);
        }
        ensure_supported_format(summary.sample_format)?;
        let (start, end) = config.selection.resolve(summary.trace_count)?;
        let (file, mapped) = if should_try_mmap(config.io_strategy) {
            match mapped {
                Some(mapped) => (None, Some(mapped)),
                None => match map_file(&path) {
                    Ok(mapped) => (None, Some(mapped)),
                    Err(error) if matches!(config.io_strategy, IoStrategy::Mmap) => {
                        return Err(error);
                    }
                    Err(_) => {
                        let mut file = File::open(path)?;
                        file.seek(SeekFrom::Start(
                            summary.first_trace_offset + start * summary.trace_size_bytes,
                        ))?;
                        (Some(file), None)
                    }
                },
            }
        } else {
            let mut file = File::open(path)?;
            file.seek(SeekFrom::Start(
                summary.first_trace_offset + start * summary.trace_size_bytes,
            ))?;
            (Some(file), None)
        };

        Ok(Self {
            file,
            mapped,
            summary,
            next_trace: start,
            end_trace: end,
            traces_per_chunk: config.traces_per_chunk,
            parallel_decode: config.parallel_decode,
            raw_buffer: Vec::new(),
        })
    }
}

impl Iterator for TraceChunkIter {
    type Item = Result<TraceChunk<f32>, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_trace >= self.end_trace {
            return None;
        }

        let remaining = (self.end_trace - self.next_trace) as usize;
        let traces_in_chunk = remaining.min(self.traces_per_chunk);
        let trace_size = self.summary.trace_size_bytes as usize;
        let raw_len = traces_in_chunk * trace_size;
        let start_trace = self.next_trace;
        let result = if let Some(mapped) = &self.mapped {
            slice_mapped_trace_bytes(
                mapped,
                self.summary.first_trace_offset,
                self.summary.trace_size_bytes,
                start_trace,
                traces_in_chunk,
                |raw| {
                    decode_trace_chunk(
                        raw,
                        self.summary.sample_format,
                        self.summary.endianness,
                        self.summary.samples_per_trace as usize,
                        start_trace,
                        self.parallel_decode,
                    )
                },
            )
        } else {
            self.raw_buffer.resize(raw_len, 0);
            match self.file.as_mut() {
                Some(file) => {
                    if let Err(error) = file.read_exact(&mut self.raw_buffer[..raw_len]) {
                        return Some(Err(ReadError::Io(error)));
                    }
                    decode_trace_chunk(
                        &self.raw_buffer[..raw_len],
                        self.summary.sample_format,
                        self.summary.endianness,
                        self.summary.samples_per_trace as usize,
                        start_trace,
                        self.parallel_decode,
                    )
                }
                None => Err(ReadError::TraceCountOverflow),
            }
        };

        self.next_trace += traces_in_chunk as u64;
        Some(result)
    }
}

fn ensure_supported_format(sample_format: SampleFormat) -> Result<(), ReadError> {
    match sample_format {
        SampleFormat::IbmFloat32
        | SampleFormat::Int32
        | SampleFormat::Int16
        | SampleFormat::IeeeFloat64
        | SampleFormat::Int24
        | SampleFormat::IeeeFloat32
        | SampleFormat::Int8 => Ok(()),
        SampleFormat::Int64
        | SampleFormat::UInt32
        | SampleFormat::UInt16
        | SampleFormat::UInt64
        | SampleFormat::UInt24
        | SampleFormat::UInt8 => Ok(()),
        _ => Err(ReadError::UnsupportedSampleFormat { sample_format }),
    }
}

fn read_primary_trace_header(
    path: &Path,
    first_trace_offset: u64,
    endianness: Endianness,
) -> Result<PrimaryTraceHeader, ReadError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(first_trace_offset))?;
    let mut header = [0_u8; TRACE_HEADER_SIZE];
    file.read_exact(&mut header)?;

    Ok(PrimaryTraceHeader {
        sample_count: HeaderField::TRACE_SAMPLE_COUNT.read(&header, endianness) as u16,
        sample_interval_us: HeaderField::TRACE_SAMPLE_INTERVAL.read(&header, endianness) as i16,
        delay_recording_time_ms: HeaderField::DELAY_RECORDING_TIME.read(&header, endianness) as i16,
        delay_scalar: HeaderField::DELAY_SCALAR.read(&header, endianness) as i16,
    })
}

fn push_sample_count_warning(
    warnings: &mut Vec<SegyWarning>,
    binary_header: u16,
    trace_header: u16,
) {
    if binary_header > 0 && trace_header > 0 && binary_header != trace_header {
        warnings.push(SegyWarning::ConflictingSampleCount {
            binary_header,
            trace_header,
        });
    }
}

fn resolve_sample_interval_us(
    binary_header: i16,
    trace_header: i16,
    validation_mode: ValidationMode,
    interval_options: IntervalOptions,
    warnings: &mut Vec<SegyWarning>,
) -> Result<u16, ReadError> {
    let (resolved_raw, resolved_source) =
        if binary_header > 0 && trace_header > 0 && binary_header != trace_header {
            match validation_mode {
                ValidationMode::Strict => {
                    return Err(ReadError::InconsistentSampleInterval {
                        binary_header,
                        trace_header,
                    });
                }
                ValidationMode::Lenient => (binary_header, SampleIntervalSource::BinaryHeader),
            }
        } else if binary_header > 0 {
            (binary_header, SampleIntervalSource::BinaryHeader)
        } else if trace_header > 0 {
            (trace_header, SampleIntervalSource::TraceHeader)
        } else {
            return Err(ReadError::UnableToResolveSampleInterval);
        };

    if matches!(validation_mode, ValidationMode::Lenient)
        && binary_header > 0
        && trace_header > 0
        && binary_header != trace_header
    {
        let resolved_us = finalize_sample_interval_us(
            resolved_raw,
            resolved_source,
            validation_mode,
            interval_options,
            warnings,
        )?;
        warnings.push(SegyWarning::ConflictingSampleInterval {
            binary_header,
            trace_header,
            resolved_us,
        });
        return Ok(resolved_us);
    }

    finalize_sample_interval_us(
        resolved_raw,
        resolved_source,
        validation_mode,
        interval_options,
        warnings,
    )
}

fn finalize_sample_interval_us(
    raw_value: i16,
    source: SampleIntervalSource,
    validation_mode: ValidationMode,
    interval_options: IntervalOptions,
    warnings: &mut Vec<SegyWarning>,
) -> Result<u16, ReadError> {
    if interval_value_is_suspicious(raw_value as i32) {
        warnings.push(SegyWarning::SuspiciousSampleInterval {
            source,
            raw_value: raw_value as i32,
        });
    }

    let unit = match interval_options.unit_override {
        Some(unit) => unit,
        None if matches!(validation_mode, ValidationMode::Lenient)
            && interval_options.enable_lenient_ms_guess
            && interval_value_is_suspicious(raw_value as i32) =>
        {
            SampleIntervalUnit::Milliseconds
        }
        None => SampleIntervalUnit::Microseconds,
    };

    let resolved = match unit {
        SampleIntervalUnit::Microseconds => raw_value as i32,
        SampleIntervalUnit::Milliseconds => (raw_value as i32) * 1000,
    };

    if !(1..=u16::MAX as i32).contains(&resolved) {
        return Err(ReadError::SampleIntervalOverflow { raw_value, unit });
    }

    Ok(resolved as u16)
}

fn interval_value_is_suspicious(value: i32) -> bool {
    (1..100).contains(&value)
}

#[derive(Debug)]
struct CubePlan {
    ilines: Vec<i64>,
    xlines: Vec<i64>,
    offsets: Vec<i64>,
    slot_indices: Vec<usize>,
}

fn map_file(path: &Path) -> Result<Arc<Mmap>, ReadError> {
    let file = File::open(path)?;
    Ok(Arc::new(unsafe { Mmap::map(&file)? }))
}

fn read_all_traces_into_buffer(
    reader: &SegyReader,
    config: ChunkReadConfig,
    dst: &mut [f32],
) -> Result<(), ReadError> {
    if config.traces_per_chunk == 0 {
        return Err(ReadError::InvalidChunkSize);
    }

    let layout = reader.trace_block_layout(config.selection)?;
    let expected_len = layout.trace_count * layout.samples_per_trace;
    if dst.len() != expected_len {
        return Err(ReadError::InvalidDestinationBuffer {
            actual_len: dst.len(),
            expected_len,
        });
    }

    let trace_size = reader.summary.trace_size_bytes as usize;
    if should_try_mmap(config.io_strategy) {
        let mmap_result = with_trace_bytes(
            reader,
            reader.summary.first_trace_offset,
            reader.summary.trace_size_bytes,
            layout.start_trace,
            layout.trace_count,
            |raw| {
                decode_trace_chunk_into(
                    raw,
                    reader.summary.sample_format,
                    reader.summary.endianness,
                    layout.samples_per_trace,
                    dst,
                    config.parallel_decode,
                )
            },
        );

        match mmap_result {
            Ok(()) => return Ok(()),
            Err(error) if matches!(config.io_strategy, IoStrategy::Mmap) => return Err(error),
            Err(_) => {}
        }
    }

    let mut file = File::open(&reader.path)?;
    file.seek(SeekFrom::Start(
        reader.summary.first_trace_offset + layout.start_trace * reader.summary.trace_size_bytes,
    ))?;

    let mut raw_buffer = vec![0_u8; config.traces_per_chunk.saturating_mul(trace_size)];
    let mut decoded_traces = 0usize;
    while decoded_traces < layout.trace_count {
        let traces_in_chunk = (layout.trace_count - decoded_traces).min(config.traces_per_chunk);
        let raw_len = traces_in_chunk * trace_size;
        file.read_exact(&mut raw_buffer[..raw_len])?;

        let dst_start = decoded_traces * layout.samples_per_trace;
        let dst_end = dst_start + traces_in_chunk * layout.samples_per_trace;
        decode_trace_chunk_into(
            &raw_buffer[..raw_len],
            reader.summary.sample_format,
            reader.summary.endianness,
            layout.samples_per_trace,
            &mut dst[dst_start..dst_end],
            config.parallel_decode,
        )?;
        decoded_traces += traces_in_chunk;
    }

    Ok(())
}

#[derive(Debug)]
struct RawTraceChunk {
    start_trace: u64,
    trace_count: usize,
    raw_buffer: Vec<u8>,
}

fn validate_chunk_scratch(scratch: &[f32], samples_per_trace: usize) -> Result<usize, ReadError> {
    if samples_per_trace == 0 {
        return Err(ReadError::TraceCountOverflow);
    }

    let scratch_traces = scratch.len() / samples_per_trace;
    if scratch_traces == 0 {
        return Err(ReadError::InvalidDestinationBuffer {
            actual_len: scratch.len(),
            expected_len: samples_per_trace,
        });
    }

    Ok(scratch_traces)
}

fn process_trace_chunks_streaming<E, F>(
    reader: &SegyReader,
    config: ChunkReadConfig,
    layout: TraceBlockInfo,
    scratch: &mut [f32],
    mut process: F,
) -> Result<(), ChunkProcessingError<E>>
where
    F: FnMut(TraceChunkRef<'_, f32>) -> Result<(), E>,
{
    let chunk_traces = validate_chunk_scratch(scratch, layout.samples_per_trace)
        .map_err(ChunkProcessingError::Read)?
        .min(config.traces_per_chunk);
    let trace_size = reader.summary.trace_size_bytes as usize;
    let raw_capacity = chunk_traces.saturating_mul(trace_size);

    if layout.trace_count > chunk_traces {
        let prefetched = process_trace_chunks_streaming_prefetched(
            reader,
            config,
            layout,
            scratch,
            raw_capacity,
            &mut process,
        );
        if prefetched.is_ok() {
            return Ok(());
        }
        if matches!(config.io_strategy, IoStrategy::Stream) {
            return prefetched;
        }
    }

    let mut file = File::open(&reader.path)
        .map_err(ReadError::Io)
        .map_err(ChunkProcessingError::Read)?;
    file.seek(SeekFrom::Start(
        reader.summary.first_trace_offset + layout.start_trace * reader.summary.trace_size_bytes,
    ))
    .map_err(ReadError::Io)
    .map_err(ChunkProcessingError::Read)?;
    let mut raw_buffer = vec![0_u8; raw_capacity];
    let mut processed_traces = 0usize;

    while processed_traces < layout.trace_count {
        let traces_in_chunk = (layout.trace_count - processed_traces).min(chunk_traces);
        let raw_len = traces_in_chunk * trace_size;
        file.read_exact(&mut raw_buffer[..raw_len])
            .map_err(ReadError::Io)
            .map_err(ChunkProcessingError::Read)?;

        let dst = &mut scratch[..traces_in_chunk * layout.samples_per_trace];
        decode_trace_chunk_into(
            &raw_buffer[..raw_len],
            reader.summary.sample_format,
            reader.summary.endianness,
            layout.samples_per_trace,
            dst,
            config.parallel_decode,
        )
        .map_err(ChunkProcessingError::Read)?;
        let start_trace = layout.start_trace + processed_traces as u64;
        process(TraceChunkRef {
            start_trace,
            trace_count: traces_in_chunk,
            samples_per_trace: layout.samples_per_trace,
            data: dst,
        })
        .map_err(ChunkProcessingError::Sink)?;
        processed_traces += traces_in_chunk;
    }

    Ok(())
}

fn process_trace_chunks_streaming_prefetched<E, F>(
    reader: &SegyReader,
    config: ChunkReadConfig,
    layout: TraceBlockInfo,
    scratch: &mut [f32],
    raw_capacity: usize,
    process: &mut F,
) -> Result<(), ChunkProcessingError<E>>
where
    F: FnMut(TraceChunkRef<'_, f32>) -> Result<(), E>,
{
    let chunk_traces = validate_chunk_scratch(scratch, layout.samples_per_trace)
        .map_err(ChunkProcessingError::Read)?
        .min(config.traces_per_chunk);
    let trace_size = reader.summary.trace_size_bytes as usize;
    let (buffer_tx, buffer_rx) = sync_channel::<Vec<u8>>(2);
    let (chunk_tx, chunk_rx) = sync_channel::<Result<RawTraceChunk, std::io::Error>>(2);

    buffer_tx
        .send(vec![0_u8; raw_capacity])
        .map_err(|_| ChunkProcessingError::Read(ReadError::Io(prefetch_channel_error())))?;
    buffer_tx
        .send(vec![0_u8; raw_capacity])
        .map_err(|_| ChunkProcessingError::Read(ReadError::Io(prefetch_channel_error())))?;

    let path = reader.path.clone();
    let first_trace_offset = reader.summary.first_trace_offset;
    let trace_size_bytes = reader.summary.trace_size_bytes;
    let start_trace = layout.start_trace;
    let total_traces = layout.trace_count;

    let handle = thread::spawn(move || -> Result<(), std::io::Error> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) => {
                let _ = chunk_tx.send(Err(error));
                return Ok(());
            }
        };
        if let Err(error) = file.seek(SeekFrom::Start(
            first_trace_offset + start_trace * trace_size_bytes,
        )) {
            let _ = chunk_tx.send(Err(error));
            return Ok(());
        }
        let mut sent_traces = 0usize;

        while sent_traces < total_traces {
            let mut raw_buffer = match buffer_rx.recv() {
                Ok(buffer) => buffer,
                Err(_) => return Ok(()),
            };
            let traces_in_chunk = (total_traces - sent_traces).min(chunk_traces);
            let raw_len = traces_in_chunk * trace_size;
            raw_buffer.resize(raw_len, 0);
            if let Err(error) = file.read_exact(&mut raw_buffer[..raw_len]) {
                let _ = chunk_tx.send(Err(error));
                return Ok(());
            }

            if chunk_tx
                .send(Ok(RawTraceChunk {
                    start_trace: start_trace + sent_traces as u64,
                    trace_count: traces_in_chunk,
                    raw_buffer,
                }))
                .is_err()
            {
                return Ok(());
            }

            sent_traces += traces_in_chunk;
        }

        Ok(())
    });

    let mut processed_traces = 0usize;
    let mut processing_result = Ok(());

    while processed_traces < total_traces {
        let raw_chunk = match chunk_rx.recv() {
            Ok(chunk) => match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    processing_result = Err(ChunkProcessingError::Read(ReadError::Io(error)));
                    break;
                }
            },
            Err(_) => {
                processing_result = Err(ChunkProcessingError::Read(ReadError::Io(
                    prefetch_channel_error(),
                )));
                break;
            }
        };

        let dst = &mut scratch[..raw_chunk.trace_count * layout.samples_per_trace];
        if let Err(error) = decode_trace_chunk_into(
            &raw_chunk.raw_buffer,
            reader.summary.sample_format,
            reader.summary.endianness,
            layout.samples_per_trace,
            dst,
            config.parallel_decode,
        ) {
            processing_result = Err(ChunkProcessingError::Read(error));
            break;
        }

        if let Err(error) = process(TraceChunkRef {
            start_trace: raw_chunk.start_trace,
            trace_count: raw_chunk.trace_count,
            samples_per_trace: layout.samples_per_trace,
            data: dst,
        }) {
            processing_result = Err(ChunkProcessingError::Sink(error));
            break;
        }

        processed_traces += raw_chunk.trace_count;
        let mut raw_buffer = raw_chunk.raw_buffer;
        raw_buffer.resize(raw_capacity, 0);
        let _ = buffer_tx.send(raw_buffer);
    }

    drop(buffer_tx);
    drop(chunk_rx);

    match handle.join() {
        Ok(Ok(())) => processing_result,
        Ok(Err(error)) => {
            if processing_result.is_ok() {
                Err(ChunkProcessingError::Read(ReadError::Io(error)))
            } else {
                processing_result
            }
        }
        Err(_) => Err(ChunkProcessingError::Read(ReadError::Io(
            std::io::Error::other("trace prefetch thread panicked"),
        ))),
    }
}

fn prefetch_channel_error() -> std::io::Error {
    std::io::Error::other("trace prefetch channel closed unexpectedly")
}

fn analyze_geometry_headers(
    headers: &HeaderTable,
    inline_field: HeaderField,
    crossline_field: HeaderField,
    third_axis_field: Option<HeaderField>,
) -> Result<GeometryReport, ReadError> {
    let ilines = headers
        .column(inline_field)
        .ok_or(ReadError::MissingGeometryField {
            field: inline_field.name,
        })?;
    let xlines = headers
        .column(crossline_field)
        .ok_or(ReadError::MissingGeometryField {
            field: crossline_field.name,
        })?;
    let third_axis_values = if let Some(field) = third_axis_field {
        Some(
            headers
                .column(field)
                .ok_or(ReadError::MissingGeometryField { field: field.name })?,
        )
    } else {
        None
    };

    if headers.rows() == 0 {
        return Ok(GeometryReport {
            inline_field,
            crossline_field,
            third_axis_field,
            stacking_state: SeismicStackingState::Unknown,
            organization: SeismicOrganization::Unstructured,
            layout: SeismicLayout::UnstructuredTraceCollection,
            gather_axis_kind: None,
            inline_values: Vec::new(),
            crossline_values: Vec::new(),
            third_axis_values: Vec::new(),
            observed_trace_count: 0,
            unique_coordinate_count: 0,
            expected_trace_count: 0,
            completeness_ratio: 0.0,
            missing_bin_count: 0,
            duplicate_coordinate_count: 0,
            duplicate_examples: Vec::new(),
            classification: GeometryClassification::NonCartesian,
        });
    }

    let unique_ilines = sorted_unique(ilines);
    let unique_xlines = sorted_unique(xlines);
    let unique_third_axis = third_axis_values.map(sorted_unique).unwrap_or_default();
    let third_axis_cardinality = unique_third_axis.len().max(1);
    let expected_trace_count = unique_ilines.len() * unique_xlines.len() * third_axis_cardinality;

    let mut occupancy = HashMap::with_capacity(headers.rows());
    for trace_index in 0..headers.rows() {
        let coordinate = GeometryCoordinate {
            inline: ilines[trace_index],
            crossline: xlines[trace_index],
            third_axis: third_axis_values.map(|values| values[trace_index]),
        };
        *occupancy.entry(coordinate).or_insert(0usize) += 1;
    }

    let unique_coordinate_count = occupancy.len();
    let missing_bin_count = expected_trace_count.saturating_sub(unique_coordinate_count);
    let completeness_ratio = if expected_trace_count == 0 {
        0.0
    } else {
        unique_coordinate_count as f64 / expected_trace_count as f64
    };

    let duplicate_examples = occupancy
        .iter()
        .filter_map(|(coordinate, count)| (*count > 1).then_some(*coordinate))
        .take(8)
        .collect::<Vec<_>>();
    let duplicate_coordinate_count = occupancy.values().filter(|count| **count > 1).count();

    let classification = if duplicate_coordinate_count == 0 && missing_bin_count == 0 {
        GeometryClassification::RegularDense
    } else if duplicate_coordinate_count == 0 {
        GeometryClassification::RegularSparse
    } else if missing_bin_count == 0 {
        GeometryClassification::DuplicateCoordinates
    } else {
        GeometryClassification::AmbiguousMapping
    };
    let gather_axis_kind = third_axis_field.map(classify_gather_axis_kind);
    let (stacking_state, organization, layout) =
        classify_survey_layout(third_axis_field, classification, gather_axis_kind);

    Ok(GeometryReport {
        inline_field,
        crossline_field,
        third_axis_field,
        stacking_state,
        organization,
        layout,
        gather_axis_kind,
        inline_values: unique_ilines,
        crossline_values: unique_xlines,
        third_axis_values: unique_third_axis,
        observed_trace_count: headers.rows(),
        unique_coordinate_count,
        expected_trace_count,
        completeness_ratio,
        missing_bin_count,
        duplicate_coordinate_count,
        duplicate_examples,
        classification,
    })
}

fn classify_gather_axis_kind(field: HeaderField) -> SeismicGatherAxisKind {
    if field == HeaderField::OFFSET {
        SeismicGatherAxisKind::Offset
    } else {
        SeismicGatherAxisKind::Unknown
    }
}

fn classify_survey_layout(
    third_axis_field: Option<HeaderField>,
    classification: GeometryClassification,
    gather_axis_kind: Option<SeismicGatherAxisKind>,
) -> (SeismicStackingState, SeismicOrganization, SeismicLayout) {
    if classification == GeometryClassification::NonCartesian {
        return (
            if third_axis_field.is_some() {
                SeismicStackingState::PreStack
            } else {
                SeismicStackingState::Unknown
            },
            SeismicOrganization::Unstructured,
            SeismicLayout::UnstructuredTraceCollection,
        );
    }

    match gather_axis_kind {
        Some(SeismicGatherAxisKind::Offset) => (
            SeismicStackingState::PreStack,
            SeismicOrganization::BinnedGrid,
            SeismicLayout::PreStack3DOffset,
        ),
        Some(SeismicGatherAxisKind::Angle) => (
            SeismicStackingState::PreStack,
            SeismicOrganization::BinnedGrid,
            SeismicLayout::PreStack3DAngle,
        ),
        Some(SeismicGatherAxisKind::Azimuth) => (
            SeismicStackingState::PreStack,
            SeismicOrganization::BinnedGrid,
            SeismicLayout::PreStack3DAzimuth,
        ),
        Some(_) => (
            SeismicStackingState::PreStack,
            SeismicOrganization::BinnedGrid,
            SeismicLayout::PreStack3DUnknownAxis,
        ),
        None => (
            SeismicStackingState::PostStack,
            SeismicOrganization::BinnedGrid,
            SeismicLayout::PostStack3D,
        ),
    }
}

fn build_cube_plan(
    headers: &HeaderTable,
    inline_field: HeaderField,
    crossline_field: HeaderField,
    offset_field: HeaderField,
) -> Result<CubePlan, ReadError> {
    let report =
        analyze_geometry_headers(headers, inline_field, crossline_field, Some(offset_field))?;
    match report.classification {
        GeometryClassification::RegularDense => {}
        GeometryClassification::DuplicateCoordinates => {
            let duplicate =
                report
                    .duplicate_examples
                    .first()
                    .copied()
                    .unwrap_or(GeometryCoordinate {
                        inline: 0,
                        crossline: 0,
                        third_axis: Some(0),
                    });
            return Err(ReadError::DuplicateTraceCoordinate {
                inline: duplicate.inline,
                crossline: duplicate.crossline,
                offset: duplicate.third_axis.unwrap_or_default(),
            });
        }
        _ => {
            return Err(ReadError::IrregularGeometry {
                trace_count: report.observed_trace_count,
                expected_trace_count: report.expected_trace_count,
            });
        }
    }

    let ilines = headers
        .column(inline_field)
        .ok_or(ReadError::MissingGeometryField {
            field: inline_field.name,
        })?;
    let xlines = headers
        .column(crossline_field)
        .ok_or(ReadError::MissingGeometryField {
            field: crossline_field.name,
        })?;
    let offsets = headers
        .column(offset_field)
        .ok_or(ReadError::MissingGeometryField {
            field: offset_field.name,
        })?;

    let unique_ilines = report.inline_values;
    let unique_xlines = report.crossline_values;
    let unique_offsets = report.third_axis_values;

    let inline_lookup = index_lookup(&unique_ilines);
    let xline_lookup = index_lookup(&unique_xlines);
    let offset_lookup = index_lookup(&unique_offsets);
    let mut slot_indices = vec![0usize; headers.rows()];
    let mut occupied = vec![false; headers.rows()];

    for trace_index in 0..headers.rows() {
        let il = ilines[trace_index];
        let xl = xlines[trace_index];
        let off = offsets[trace_index];
        let inline_index = inline_lookup[&il];
        let xline_index = xline_lookup[&xl];
        let offset_index = offset_lookup[&off];
        let slot = ((inline_index * unique_xlines.len() + xline_index) * unique_offsets.len())
            + offset_index;

        if occupied[slot] {
            return Err(ReadError::DuplicateTraceCoordinate {
                inline: il,
                crossline: xl,
                offset: off,
            });
        }

        occupied[slot] = true;
        slot_indices[trace_index] = slot;
    }

    Ok(CubePlan {
        ilines: unique_ilines,
        xlines: unique_xlines,
        offsets: unique_offsets,
        slot_indices,
    })
}

fn fill_cube_data_from_reader(
    reader: &SegyReader,
    slot_indices: &[usize],
    dst: &mut [f32],
) -> Result<(), ReadError> {
    let samples_per_trace = reader.summary.samples_per_trace as usize;
    let expected_len = slot_indices.len() * samples_per_trace;
    if dst.len() != expected_len {
        return Err(ReadError::InvalidDestinationBuffer {
            actual_len: dst.len(),
            expected_len,
        });
    }

    if should_try_mmap(IoStrategy::Auto) {
        let mmap_result = with_trace_bytes(
            reader,
            reader.summary.first_trace_offset,
            reader.summary.trace_size_bytes,
            0,
            slot_indices.len(),
            |raw| {
                decode_trace_chunk_scatter_into(
                    raw,
                    reader.summary.sample_format,
                    reader.summary.endianness,
                    samples_per_trace,
                    slot_indices,
                    dst,
                )
            },
        );

        if mmap_result.is_ok() {
            return Ok(());
        }
    }

    let mut file = File::open(&reader.path)?;
    file.seek(SeekFrom::Start(reader.summary.first_trace_offset))?;
    let trace_size = reader.summary.trace_size_bytes as usize;
    let chunk_traces = ChunkReadConfig::default().traces_per_chunk;
    let mut raw_buffer = vec![0_u8; chunk_traces.saturating_mul(trace_size)];
    let mut processed = 0usize;

    while processed < slot_indices.len() {
        let traces_in_chunk = (slot_indices.len() - processed).min(chunk_traces);
        let raw_len = traces_in_chunk * trace_size;
        file.read_exact(&mut raw_buffer[..raw_len])?;
        decode_trace_chunk_scatter_into(
            &raw_buffer[..raw_len],
            reader.summary.sample_format,
            reader.summary.endianness,
            samples_per_trace,
            &slot_indices[processed..processed + traces_in_chunk],
            dst,
        )?;
        processed += traces_in_chunk;
    }

    Ok(())
}

fn decode_trace_chunk_scatter_into(
    raw: &[u8],
    sample_format: SampleFormat,
    endianness: Endianness,
    samples_per_trace: usize,
    slot_indices: &[usize],
    dst: &mut [f32],
) -> Result<(), ReadError> {
    ensure_supported_format(sample_format)?;
    let sample_bytes = sample_format.bytes_per_sample() as usize;
    let trace_size = TRACE_HEADER_SIZE + samples_per_trace * sample_bytes;
    let trace_count = raw.len() / trace_size;

    if trace_count != slot_indices.len() {
        return Err(ReadError::TraceCountOverflow);
    }

    for (trace_raw, slot_index) in raw
        .chunks_exact(trace_size)
        .zip(slot_indices.iter().copied())
    {
        let dst_start = slot_index * samples_per_trace;
        let dst_end = dst_start + samples_per_trace;
        let sample_raw =
            &trace_raw[TRACE_HEADER_SIZE..TRACE_HEADER_SIZE + samples_per_trace * sample_bytes];
        decode_samples(
            sample_raw,
            &mut dst[dst_start..dst_end],
            sample_format,
            endianness,
        )?;
    }

    Ok(())
}

fn sorted_unique(values: &[i64]) -> Vec<i64> {
    let set = values.iter().copied().collect::<BTreeSet<_>>();
    set.into_iter().collect()
}

fn index_lookup(values: &[i64]) -> HashMap<i64, usize> {
    values
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| (value, index))
        .collect()
}

fn empty_header_columns(fields: &[HeaderField], capacity: usize) -> Vec<HeaderColumn> {
    fields
        .iter()
        .copied()
        .map(|field| HeaderColumn {
            field,
            values: Vec::with_capacity(capacity),
        })
        .collect()
}

fn extract_headers_from_trace_bytes(
    raw: &[u8],
    fields: &[HeaderField],
    start_trace: u64,
    trace_size: usize,
    endianness: Endianness,
    parallel_extract: bool,
) -> HeaderTable {
    let trace_count = raw.len() / trace_size;
    let mut columns = empty_header_columns(fields, trace_count);
    for column in &mut columns {
        column.values =
            extract_header_values(raw, trace_size, column.field, endianness, parallel_extract);
    }

    HeaderTable {
        trace_numbers: (0..trace_count).map(|i| start_trace + i as u64).collect(),
        columns,
    }
}

fn should_try_mmap(strategy: IoStrategy) -> bool {
    matches!(strategy, IoStrategy::Auto | IoStrategy::Mmap)
}

fn with_trace_bytes<T>(
    reader: &SegyReader,
    first_trace_offset: u64,
    trace_size_bytes: u64,
    start_trace: u64,
    trace_count: usize,
    f: impl FnOnce(&[u8]) -> Result<T, ReadError>,
) -> Result<T, ReadError> {
    if let Some(mapped) = reader.mapped.as_deref() {
        return slice_mapped_trace_bytes(
            mapped,
            first_trace_offset,
            trace_size_bytes,
            start_trace,
            trace_count,
            f,
        );
    }

    let mapped = map_file(&reader.path)?;
    slice_mapped_trace_bytes(
        &mapped,
        first_trace_offset,
        trace_size_bytes,
        start_trace,
        trace_count,
        f,
    )
}

fn slice_mapped_trace_bytes<T>(
    mapped: &Mmap,
    first_trace_offset: u64,
    trace_size_bytes: u64,
    start_trace: u64,
    trace_count: usize,
    f: impl FnOnce(&[u8]) -> Result<T, ReadError>,
) -> Result<T, ReadError> {
    let start = usize::try_from(first_trace_offset + start_trace * trace_size_bytes)
        .map_err(|_| ReadError::TraceCountOverflow)?;
    let len = usize::try_from(trace_size_bytes)
        .map_err(|_| ReadError::TraceCountOverflow)?
        .checked_mul(trace_count)
        .ok_or(ReadError::TraceCountOverflow)?;
    let end = start
        .checked_add(len)
        .ok_or(ReadError::TraceCountOverflow)?;
    f(&mapped[start..end])
}

fn extract_header_values(
    raw: &[u8],
    trace_size: usize,
    field: HeaderField,
    endianness: Endianness,
    parallel_extract: bool,
) -> Vec<i64> {
    if parallel_extract && raw.len() > trace_size {
        raw.par_chunks_exact(trace_size)
            .map(|trace| field.read_from_slice(&trace[..TRACE_HEADER_SIZE], endianness))
            .collect()
    } else {
        raw.chunks_exact(trace_size)
            .map(|trace| field.read_from_slice(&trace[..TRACE_HEADER_SIZE], endianness))
            .collect()
    }
}

fn decode_trace_chunk(
    raw: &[u8],
    sample_format: SampleFormat,
    endianness: Endianness,
    samples_per_trace: usize,
    start_trace: u64,
    parallel_decode: bool,
) -> Result<TraceChunk<f32>, ReadError> {
    ensure_supported_format(sample_format)?;
    let sample_bytes = sample_format.bytes_per_sample() as usize;
    let trace_size = TRACE_HEADER_SIZE + samples_per_trace * sample_bytes;
    let trace_count = raw.len() / trace_size;
    let mut data = vec![0_f32; trace_count * samples_per_trace];
    decode_trace_chunk_into(
        raw,
        sample_format,
        endianness,
        samples_per_trace,
        &mut data,
        parallel_decode,
    )?;

    Ok(TraceChunk {
        start_trace,
        samples_per_trace,
        data,
    })
}

fn decode_trace_chunk_into(
    raw: &[u8],
    sample_format: SampleFormat,
    endianness: Endianness,
    samples_per_trace: usize,
    dst: &mut [f32],
    parallel_decode: bool,
) -> Result<(), ReadError> {
    ensure_supported_format(sample_format)?;
    let sample_bytes = sample_format.bytes_per_sample() as usize;
    let trace_size = TRACE_HEADER_SIZE + samples_per_trace * sample_bytes;
    let trace_count = raw.len() / trace_size;

    if dst.len() != trace_count * samples_per_trace {
        return Err(ReadError::TraceCountOverflow);
    }

    if parallel_decode && trace_count > 1 {
        raw.par_chunks_exact(trace_size)
            .zip(dst.par_chunks_mut(samples_per_trace))
            .try_for_each(|(trace_raw, trace_dst)| {
                let sample_raw = &trace_raw
                    [TRACE_HEADER_SIZE..TRACE_HEADER_SIZE + samples_per_trace * sample_bytes];
                decode_samples(sample_raw, trace_dst, sample_format, endianness)
            })
    } else {
        for (trace_raw, trace_dst) in raw
            .chunks_exact(trace_size)
            .zip(dst.chunks_mut(samples_per_trace))
        {
            let sample_raw =
                &trace_raw[TRACE_HEADER_SIZE..TRACE_HEADER_SIZE + samples_per_trace * sample_bytes];
            decode_samples(sample_raw, trace_dst, sample_format, endianness)?;
        }
        Ok(())
    }
}

fn decode_samples(
    src: &[u8],
    dst: &mut [f32],
    sample_format: SampleFormat,
    endianness: Endianness,
) -> Result<(), ReadError> {
    match sample_format {
        SampleFormat::IbmFloat32 => {
            for (src_word, dst_sample) in src.chunks_exact(4).zip(dst.iter_mut()) {
                let word = match endianness {
                    Endianness::Big => {
                        u32::from_be_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                    Endianness::Little => {
                        u32::from_le_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                };
                *dst_sample = decode_ibm32(word);
            }
            Ok(())
        }
        SampleFormat::Int32 => {
            for (src_word, dst_sample) in src.chunks_exact(4).zip(dst.iter_mut()) {
                let value = match endianness {
                    Endianness::Big => {
                        i32::from_be_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                    Endianness::Little => {
                        i32::from_le_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                };
                *dst_sample = value as f32;
            }
            Ok(())
        }
        SampleFormat::Int16 => {
            for (src_word, dst_sample) in src.chunks_exact(2).zip(dst.iter_mut()) {
                let value = match endianness {
                    Endianness::Big => i16::from_be_bytes([src_word[0], src_word[1]]),
                    Endianness::Little => i16::from_le_bytes([src_word[0], src_word[1]]),
                };
                *dst_sample = value as f32;
            }
            Ok(())
        }
        SampleFormat::IeeeFloat32 => {
            for (src_word, dst_sample) in src.chunks_exact(4).zip(dst.iter_mut()) {
                let bits = match endianness {
                    Endianness::Big => {
                        u32::from_be_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                    Endianness::Little => {
                        u32::from_le_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                };
                *dst_sample = f32::from_bits(bits);
            }
            Ok(())
        }
        SampleFormat::IeeeFloat64 => {
            for (src_word, dst_sample) in src.chunks_exact(8).zip(dst.iter_mut()) {
                let bits = match endianness {
                    Endianness::Big => u64::from_be_bytes([
                        src_word[0],
                        src_word[1],
                        src_word[2],
                        src_word[3],
                        src_word[4],
                        src_word[5],
                        src_word[6],
                        src_word[7],
                    ]),
                    Endianness::Little => u64::from_le_bytes([
                        src_word[0],
                        src_word[1],
                        src_word[2],
                        src_word[3],
                        src_word[4],
                        src_word[5],
                        src_word[6],
                        src_word[7],
                    ]),
                };
                *dst_sample = f64::from_bits(bits) as f32;
            }
            Ok(())
        }
        SampleFormat::Int24 => {
            for (src_word, dst_sample) in src.chunks_exact(3).zip(dst.iter_mut()) {
                *dst_sample = decode_i24(src_word, endianness) as f32;
            }
            Ok(())
        }
        SampleFormat::Int8 => {
            for (src_byte, dst_sample) in src.iter().zip(dst.iter_mut()) {
                *dst_sample = (*src_byte as i8) as f32;
            }
            Ok(())
        }
        SampleFormat::Int64 => {
            for (src_word, dst_sample) in src.chunks_exact(8).zip(dst.iter_mut()) {
                let value = match endianness {
                    Endianness::Big => i64::from_be_bytes([
                        src_word[0],
                        src_word[1],
                        src_word[2],
                        src_word[3],
                        src_word[4],
                        src_word[5],
                        src_word[6],
                        src_word[7],
                    ]),
                    Endianness::Little => i64::from_le_bytes([
                        src_word[0],
                        src_word[1],
                        src_word[2],
                        src_word[3],
                        src_word[4],
                        src_word[5],
                        src_word[6],
                        src_word[7],
                    ]),
                };
                *dst_sample = value as f32;
            }
            Ok(())
        }
        SampleFormat::UInt32 => {
            for (src_word, dst_sample) in src.chunks_exact(4).zip(dst.iter_mut()) {
                let value = match endianness {
                    Endianness::Big => {
                        u32::from_be_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                    Endianness::Little => {
                        u32::from_le_bytes([src_word[0], src_word[1], src_word[2], src_word[3]])
                    }
                };
                *dst_sample = value as f32;
            }
            Ok(())
        }
        SampleFormat::UInt16 => {
            for (src_word, dst_sample) in src.chunks_exact(2).zip(dst.iter_mut()) {
                let value = match endianness {
                    Endianness::Big => u16::from_be_bytes([src_word[0], src_word[1]]),
                    Endianness::Little => u16::from_le_bytes([src_word[0], src_word[1]]),
                };
                *dst_sample = value as f32;
            }
            Ok(())
        }
        SampleFormat::UInt64 => {
            for (src_word, dst_sample) in src.chunks_exact(8).zip(dst.iter_mut()) {
                let value = match endianness {
                    Endianness::Big => u64::from_be_bytes([
                        src_word[0],
                        src_word[1],
                        src_word[2],
                        src_word[3],
                        src_word[4],
                        src_word[5],
                        src_word[6],
                        src_word[7],
                    ]),
                    Endianness::Little => u64::from_le_bytes([
                        src_word[0],
                        src_word[1],
                        src_word[2],
                        src_word[3],
                        src_word[4],
                        src_word[5],
                        src_word[6],
                        src_word[7],
                    ]),
                };
                *dst_sample = value as f32;
            }
            Ok(())
        }
        SampleFormat::UInt24 => {
            for (src_word, dst_sample) in src.chunks_exact(3).zip(dst.iter_mut()) {
                *dst_sample = decode_u24(src_word, endianness) as f32;
            }
            Ok(())
        }
        SampleFormat::UInt8 => {
            for (src_byte, dst_sample) in src.iter().zip(dst.iter_mut()) {
                *dst_sample = *src_byte as f32;
            }
            Ok(())
        }
        _ => Err(ReadError::UnsupportedSampleFormat { sample_format }),
    }
}

fn decode_ibm32(word: u32) -> f32 {
    if word == 0 {
        return 0.0;
    }

    let sign = if (word >> 31) & 0x1 == 0 { 1.0 } else { -1.0 };
    let exponent = ((word >> 24) & 0x7f) as i32 - 64;
    let fraction = (word & 0x00ff_ffff) as f64 / 16_777_216.0;
    (sign * fraction * 16_f64.powi(exponent)) as f32
}

fn decode_i24(src: &[u8], endianness: Endianness) -> i32 {
    let bytes = match endianness {
        Endianness::Big => [src[0], src[1], src[2]],
        Endianness::Little => [src[2], src[1], src[0]],
    };

    let mut value = ((bytes[0] as i32) << 16) | ((bytes[1] as i32) << 8) | bytes[2] as i32;
    if value & 0x0080_0000 != 0 {
        value -= 1 << 24;
    }
    value
}

fn decode_u24(src: &[u8], endianness: Endianness) -> u32 {
    let bytes = match endianness {
        Endianness::Big => [src[0], src[1], src[2]],
        Endianness::Little => [src[2], src[1], src[0]],
    };

    ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | bytes[2] as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_table(
        ilines: Vec<i64>,
        xlines: Vec<i64>,
        third_axis: Option<(HeaderField, Vec<i64>)>,
    ) -> HeaderTable {
        let mut columns = vec![
            HeaderColumn {
                field: HeaderField::INLINE_3D,
                values: ilines,
            },
            HeaderColumn {
                field: HeaderField::CROSSLINE_3D,
                values: xlines,
            },
        ];

        if let Some((field, values)) = third_axis {
            columns.push(HeaderColumn { field, values });
        }

        let row_count = columns
            .first()
            .map(|column| column.values.len())
            .unwrap_or_default();

        HeaderTable {
            trace_numbers: (0..row_count as u64).collect(),
            columns,
        }
    }

    #[test]
    fn geometry_report_marks_post_stack_binned_grid_without_third_axis() {
        let headers = header_table(vec![10, 10, 11, 11], vec![20, 21, 20, 21], None);
        let report = analyze_geometry_headers(
            &headers,
            HeaderField::INLINE_3D,
            HeaderField::CROSSLINE_3D,
            None,
        )
        .unwrap();

        assert_eq!(report.stacking_state, SeismicStackingState::PostStack);
        assert_eq!(report.organization, SeismicOrganization::BinnedGrid);
        assert_eq!(report.layout, SeismicLayout::PostStack3D);
        assert_eq!(report.gather_axis_kind, None);
    }

    #[test]
    fn geometry_report_marks_prestack_offset_grid_with_offset_axis() {
        let headers = header_table(
            vec![10, 10, 10, 10, 11, 11, 11, 11],
            vec![20, 20, 21, 21, 20, 20, 21, 21],
            Some((HeaderField::OFFSET, vec![1, 2, 1, 2, 1, 2, 1, 2])),
        );
        let report = analyze_geometry_headers(
            &headers,
            HeaderField::INLINE_3D,
            HeaderField::CROSSLINE_3D,
            Some(HeaderField::OFFSET),
        )
        .unwrap();

        assert_eq!(report.stacking_state, SeismicStackingState::PreStack);
        assert_eq!(report.organization, SeismicOrganization::BinnedGrid);
        assert_eq!(report.layout, SeismicLayout::PreStack3DOffset);
        assert_eq!(report.gather_axis_kind, Some(SeismicGatherAxisKind::Offset));
        assert_eq!(report.third_axis_values, vec![1, 2]);
    }
}
