use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const TEXTUAL_HEADER_SIZE: usize = 3200;
const BINARY_HEADER_SIZE: usize = 400;
const FILE_HEADER_SIZE: usize = TEXTUAL_HEADER_SIZE + BINARY_HEADER_SIZE;
const TRACE_HEADER_SIZE: u64 = 240;

const BIN_SAMPLE_INTERVAL: usize = 3216;
const BIN_SAMPLE_COUNT: usize = 3220;
const BIN_FORMAT_CODE: usize = 3224;
const BIN_REVISION: usize = 3500;
const BIN_FIXED_LENGTH_FLAG: usize = 3502;
const BIN_EXTENDED_TEXT_HEADERS: usize = 3504;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Big,
    Little,
}

impl Endianness {
    fn read_u16(self, buffer: &[u8], offset: usize) -> u16 {
        let bytes = [buffer[offset], buffer[offset + 1]];
        match self {
            Self::Big => u16::from_be_bytes(bytes),
            Self::Little => u16::from_le_bytes(bytes),
        }
    }

    fn read_i16(self, buffer: &[u8], offset: usize) -> i16 {
        let bytes = [buffer[offset], buffer[offset + 1]];
        match self {
            Self::Big => i16::from_be_bytes(bytes),
            Self::Little => i16::from_le_bytes(bytes),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    IbmFloat32,
    Int32,
    Int16,
    FixedPoint32,
    IeeeFloat32,
    IeeeFloat64,
    Int24,
    Int8,
    Int64,
    UInt32,
    UInt16,
    UInt64,
    UInt24,
    UInt8,
}

impl SampleFormat {
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::IbmFloat32),
            2 => Some(Self::Int32),
            3 => Some(Self::Int16),
            4 => Some(Self::FixedPoint32),
            5 => Some(Self::IeeeFloat32),
            6 => Some(Self::IeeeFloat64),
            7 => Some(Self::Int24),
            8 => Some(Self::Int8),
            9 => Some(Self::Int64),
            10 => Some(Self::UInt32),
            11 => Some(Self::UInt16),
            12 => Some(Self::UInt64),
            15 => Some(Self::UInt24),
            16 => Some(Self::UInt8),
            _ => None,
        }
    }

    pub fn code(self) -> u16 {
        match self {
            Self::IbmFloat32 => 1,
            Self::Int32 => 2,
            Self::Int16 => 3,
            Self::FixedPoint32 => 4,
            Self::IeeeFloat32 => 5,
            Self::IeeeFloat64 => 6,
            Self::Int24 => 7,
            Self::Int8 => 8,
            Self::Int64 => 9,
            Self::UInt32 => 10,
            Self::UInt16 => 11,
            Self::UInt64 => 12,
            Self::UInt24 => 15,
            Self::UInt8 => 16,
        }
    }

    pub fn bytes_per_sample(self) -> u16 {
        match self {
            Self::IbmFloat32
            | Self::Int32
            | Self::FixedPoint32
            | Self::IeeeFloat32
            | Self::UInt32 => 4,
            Self::Int16 | Self::UInt16 => 2,
            Self::IeeeFloat64 | Self::Int64 | Self::UInt64 => 8,
            Self::Int24 | Self::UInt24 => 3,
            Self::Int8 | Self::UInt8 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegyRevision {
    pub major: u8,
    pub minor: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InspectOptions {
    pub endianness_override: Option<Endianness>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextualHeaderEncoding {
    Ascii,
    Ebcdic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextualHeader {
    pub index: usize,
    pub raw: Vec<u8>,
    pub decoded: String,
    pub encoding: TextualHeaderEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleIntervalSource {
    BinaryHeader,
    TraceHeader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegyWarning {
    NonAsciiTextHeader {
        header_index: usize,
        encoding: TextualHeaderEncoding,
    },
    LossyTextHeaderDecode {
        header_index: usize,
        encoding: TextualHeaderEncoding,
    },
    SuspiciousSampleInterval {
        source: SampleIntervalSource,
        raw_value: i32,
    },
    ConflictingSampleCount {
        binary_header: u16,
        trace_header: u16,
    },
    ConflictingSampleInterval {
        binary_header: i16,
        trace_header: i16,
        resolved_us: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSummary {
    pub path: PathBuf,
    pub file_size: u64,
    pub endianness: Endianness,
    pub sample_interval_us: u16,
    pub samples_per_trace: u16,
    pub sample_format: SampleFormat,
    pub sample_format_code: u16,
    pub sample_bytes: u16,
    pub revision_raw: u16,
    pub revision: Option<SegyRevision>,
    pub fixed_length_trace_flag_raw: u16,
    pub fixed_length_trace: Option<bool>,
    pub extended_textual_headers: i16,
    pub first_trace_offset: u64,
    pub trace_size_bytes: u64,
    pub trace_count: u64,
    pub textual_header_is_ascii_like: bool,
    pub textual_headers: Vec<TextualHeader>,
    pub warnings: Vec<SegyWarning>,
}

impl FileSummary {
    pub fn total_textual_headers(&self) -> u16 {
        1 + self.extended_textual_headers.max(0) as u16
    }
}

#[derive(Debug)]
pub enum InspectError {
    Io(std::io::Error),
    FileTooSmall {
        actual_size: u64,
    },
    UnsupportedSampleFormat {
        code: u16,
    },
    UnableToDetermineEndianness,
    InvalidFirstTraceOffset {
        offset: u64,
        file_size: u64,
    },
    TraceSizeOverflow,
    UnalignedTraceData {
        data_bytes: u64,
        trace_size_bytes: u64,
    },
}

impl Display for InspectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::FileTooSmall { actual_size } => {
                write!(
                    f,
                    "file is too small to be a SEG-Y file: {actual_size} bytes"
                )
            }
            Self::UnsupportedSampleFormat { code } => {
                write!(f, "unsupported SEG-Y sample format code {code}")
            }
            Self::UnableToDetermineEndianness => {
                write!(f, "unable to determine file endianness from binary header")
            }
            Self::InvalidFirstTraceOffset { offset, file_size } => {
                write!(
                    f,
                    "derived first trace offset {offset} is beyond file size {file_size}"
                )
            }
            Self::TraceSizeOverflow => write!(f, "trace size overflow while inspecting file"),
            Self::UnalignedTraceData {
                data_bytes,
                trace_size_bytes,
            } => write!(
                f,
                "trace data payload of {data_bytes} bytes is not aligned to trace size {trace_size_bytes}"
            ),
        }
    }
}

impl Error for InspectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for InspectError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn inspect_file(path: impl AsRef<Path>) -> Result<FileSummary, InspectError> {
    inspect_file_with_options(path, InspectOptions::default())
}

pub fn inspect_file_with_options(
    path: impl AsRef<Path>,
    options: InspectOptions,
) -> Result<FileSummary, InspectError> {
    let path = path.as_ref();
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    if file_size < FILE_HEADER_SIZE as u64 {
        return Err(InspectError::FileTooSmall {
            actual_size: file_size,
        });
    }

    let mut header = [0_u8; FILE_HEADER_SIZE];
    file.read_exact(&mut header)?;

    let textual_header_is_ascii_like = matches!(
        detect_textual_header_encoding(&header[..TEXTUAL_HEADER_SIZE]),
        TextualHeaderEncoding::Ascii
    );

    let mut summary = match options.endianness_override {
        Some(endianness) => build_summary(
            path,
            file_size,
            &header,
            endianness,
            textual_header_is_ascii_like,
        )?,
        None => autodetect_summary(path, file_size, &header, textual_header_is_ascii_like)?,
    };

    let (textual_headers, text_warnings) =
        read_textual_headers(&mut file, summary.total_textual_headers() as usize)?;
    summary.textual_headers = textual_headers;
    summary.warnings.extend(text_warnings);

    file.seek(SeekFrom::Start(summary.first_trace_offset))?;
    Ok(summary)
}

fn autodetect_summary(
    path: &Path,
    file_size: u64,
    header: &[u8; FILE_HEADER_SIZE],
    textual_header_is_ascii_like: bool,
) -> Result<FileSummary, InspectError> {
    let big = Candidate::new(file_size, header, Endianness::Big);
    let little = Candidate::new(file_size, header, Endianness::Little);

    match (big.score(), little.score()) {
        (0, 0) => Err(InspectError::UnableToDetermineEndianness),
        (big_score, little_score) if big_score > little_score => {
            big.into_summary(path.to_path_buf(), textual_header_is_ascii_like)
        }
        (big_score, little_score) if little_score > big_score => {
            little.into_summary(path.to_path_buf(), textual_header_is_ascii_like)
        }
        _ => {
            if big.sample_format.is_some() {
                big.into_summary(path.to_path_buf(), textual_header_is_ascii_like)
            } else if little.sample_format.is_some() {
                little.into_summary(path.to_path_buf(), textual_header_is_ascii_like)
            } else {
                Err(InspectError::UnableToDetermineEndianness)
            }
        }
    }
}

fn build_summary(
    path: &Path,
    file_size: u64,
    header: &[u8; FILE_HEADER_SIZE],
    endianness: Endianness,
    textual_header_is_ascii_like: bool,
) -> Result<FileSummary, InspectError> {
    Candidate::new(file_size, header, endianness)
        .into_summary(path.to_path_buf(), textual_header_is_ascii_like)
}

#[derive(Debug, Clone)]
struct Candidate {
    file_size: u64,
    endianness: Endianness,
    sample_interval_us: u16,
    samples_per_trace: u16,
    sample_format_code: u16,
    sample_format: Option<SampleFormat>,
    revision_raw: u16,
    fixed_length_trace_flag_raw: u16,
    extended_textual_headers: i16,
    first_trace_offset: u64,
    trace_size_bytes: Option<u64>,
    trace_count: Option<u64>,
}

impl Candidate {
    fn new(file_size: u64, header: &[u8; FILE_HEADER_SIZE], endianness: Endianness) -> Self {
        let sample_interval_us = endianness.read_u16(header, BIN_SAMPLE_INTERVAL);
        let samples_per_trace = endianness.read_u16(header, BIN_SAMPLE_COUNT);
        let sample_format_code = endianness.read_u16(header, BIN_FORMAT_CODE);
        let sample_format = SampleFormat::from_code(sample_format_code);
        let revision_raw = endianness.read_u16(header, BIN_REVISION);
        let fixed_length_trace_flag_raw = endianness.read_u16(header, BIN_FIXED_LENGTH_FLAG);
        let extended_textual_headers = endianness.read_i16(header, BIN_EXTENDED_TEXT_HEADERS);

        let first_trace_offset = FILE_HEADER_SIZE as u64
            + (extended_textual_headers.max(0) as u64 * TEXTUAL_HEADER_SIZE as u64);
        let trace_size_bytes = sample_format.and_then(|format| {
            TRACE_HEADER_SIZE
                .checked_add(samples_per_trace as u64 * format.bytes_per_sample() as u64)
        });
        let trace_count = trace_size_bytes.and_then(|trace_size_bytes| {
            (file_size >= first_trace_offset)
                .then_some(file_size - first_trace_offset)
                .and_then(|data_bytes| {
                    if trace_size_bytes == 0 || data_bytes % trace_size_bytes != 0 {
                        None
                    } else {
                        Some(data_bytes / trace_size_bytes)
                    }
                })
        });

        Self {
            file_size,
            endianness,
            sample_interval_us,
            samples_per_trace,
            sample_format_code,
            sample_format,
            revision_raw,
            fixed_length_trace_flag_raw,
            extended_textual_headers,
            first_trace_offset,
            trace_size_bytes,
            trace_count,
        }
    }

    fn score(&self) -> u8 {
        let mut score = 0;
        if self.sample_format.is_some() {
            score += 3;
        }
        if self.samples_per_trace > 0 {
            score += 2;
        }
        if self.sample_interval_us > 0 {
            score += 1;
        }
        if self.first_trace_offset <= self.file_size {
            score += 1;
        }
        if self.trace_count.is_some() {
            score += 2;
        }
        score
    }

    fn into_summary(
        self,
        path: PathBuf,
        textual_header_is_ascii_like: bool,
    ) -> Result<FileSummary, InspectError> {
        let sample_format = self
            .sample_format
            .ok_or(InspectError::UnsupportedSampleFormat {
                code: self.sample_format_code,
            })?;
        let trace_size_bytes = self
            .trace_size_bytes
            .ok_or(InspectError::TraceSizeOverflow)?;

        if self.first_trace_offset > self.file_size {
            return Err(InspectError::InvalidFirstTraceOffset {
                offset: self.first_trace_offset,
                file_size: self.file_size,
            });
        }

        let data_bytes = self.file_size - self.first_trace_offset;
        if data_bytes % trace_size_bytes != 0 {
            return Err(InspectError::UnalignedTraceData {
                data_bytes,
                trace_size_bytes,
            });
        }

        let revision = if self.revision_raw == 0 {
            None
        } else {
            Some(SegyRevision {
                major: (self.revision_raw >> 8) as u8,
                minor: (self.revision_raw & 0x00ff) as u8,
            })
        };

        let fixed_length_trace = match self.fixed_length_trace_flag_raw {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        };

        let mut warnings = Vec::new();
        if is_suspicious_interval_value(self.sample_interval_us as i32) {
            warnings.push(SegyWarning::SuspiciousSampleInterval {
                source: SampleIntervalSource::BinaryHeader,
                raw_value: self.sample_interval_us as i32,
            });
        }

        Ok(FileSummary {
            path,
            file_size: self.file_size,
            endianness: self.endianness,
            sample_interval_us: self.sample_interval_us,
            samples_per_trace: self.samples_per_trace,
            sample_format,
            sample_format_code: self.sample_format_code,
            sample_bytes: sample_format.bytes_per_sample(),
            revision_raw: self.revision_raw,
            revision,
            fixed_length_trace_flag_raw: self.fixed_length_trace_flag_raw,
            fixed_length_trace,
            extended_textual_headers: self.extended_textual_headers,
            first_trace_offset: self.first_trace_offset,
            trace_size_bytes,
            trace_count: data_bytes / trace_size_bytes,
            textual_header_is_ascii_like,
            textual_headers: Vec::new(),
            warnings,
        })
    }
}

fn is_suspicious_interval_value(value: i32) -> bool {
    (1..100).contains(&value)
}

fn read_textual_headers(
    file: &mut File,
    count: usize,
) -> Result<(Vec<TextualHeader>, Vec<SegyWarning>), InspectError> {
    file.seek(SeekFrom::Start(0))?;

    let mut headers = Vec::with_capacity(count);
    let mut warnings = Vec::new();
    let mut buffer = vec![0_u8; TEXTUAL_HEADER_SIZE];

    for index in 0..count {
        file.read_exact(&mut buffer)?;
        let raw = buffer.clone();
        let encoding = detect_textual_header_encoding(&raw);
        let (decoded, is_lossy) = if matches!(encoding, TextualHeaderEncoding::Ascii) {
            let decoded = String::from_utf8_lossy(&raw).into_owned();
            let is_lossy = decoded.contains('\u{FFFD}');
            (decoded, is_lossy)
        } else {
            let decoded: String = ibm1047::decode(&raw).collect();
            (decoded, false)
        };

        if matches!(encoding, TextualHeaderEncoding::Ebcdic) {
            warnings.push(SegyWarning::NonAsciiTextHeader {
                header_index: index,
                encoding,
            });
        }
        if is_lossy {
            warnings.push(SegyWarning::LossyTextHeaderDecode {
                header_index: index,
                encoding,
            });
        }

        headers.push(TextualHeader {
            index,
            raw,
            decoded,
            encoding,
        });
    }

    Ok((headers, warnings))
}

fn detect_textual_header_encoding(bytes: &[u8]) -> TextualHeaderEncoding {
    let ascii_printable = bytes
        .iter()
        .filter(|byte| matches!(byte, 0x20..=0x7e | b'\n' | b'\r' | 0))
        .count();
    let high_bit_bytes = bytes.iter().filter(|byte| **byte >= 0x80).count();

    if high_bit_bytes > bytes.len() / 20 || ascii_printable < bytes.len() * 9 / 10 {
        TextualHeaderEncoding::Ebcdic
    } else {
        TextualHeaderEncoding::Ascii
    }
}
