mod asset;
pub mod examples;
mod metadata;
mod parser;
mod storage;
mod table;

pub use asset::{
    CanonicalAlias, CurveItem, CurveSelector, HeaderItem, IndexDescriptor, IndexKind, IngestIssue,
    IssueSeverity, LasFile, LasFileSummary, LasValue, MnemonicCase, Provenance, SectionItems,
};
pub use metadata::{
    CanonicalMetadata, CurveColumnMetadata, CurveInfo, IndexInfo, PACKAGE_METADATA_SCHEMA_VERSION,
    PackageMetadata, ParameterInfo, RawMetadataSections, VersionInfo, WellInfo,
};
pub use parser::{
    DType, DTypeSpec, DecodedText, NullPolicy, NullRule, ParsedHeaderLine, ReadOptions, ReadPolicy,
    decode_bytes, import_las_file, parse_header_line, read_path, read_reader, read_string,
};
pub use storage::{StoredLasFile, open_package, write_bundle, write_package};
pub use table::{CurveColumn, CurveColumnDescriptor, CurveStorageKind, CurveTable};

use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, LasError>;

#[derive(Debug, Error)]
pub enum LasError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unsupported LAS input: {0}")]
    Unsupported(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
}
