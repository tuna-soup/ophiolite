mod asset;
mod parser;
mod storage;

pub use asset::{
    CanonicalAlias, Curve, CurveDescriptor, CurveWindow, HeaderItem, HeaderSection,
    IndexDescriptor, IndexKind, IngestIssue, IssueSeverity, LasAsset, LasAssetSummary, Provenance,
};
pub use parser::import_las_file;
pub use storage::{StoredLasAsset, write_bundle};

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;

pub type Result<T> = std::result::Result<T, LasError>;

#[derive(Debug)]
pub enum LasError {
    Io(io::Error),
    Parse(String),
    Unsupported(String),
    Storage(String),
    Serialization(serde_json::Error),
}

impl Display for LasError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Parse(err) => write!(f, "Parse error: {err}"),
            Self::Unsupported(err) => write!(f, "Unsupported LAS input: {err}"),
            Self::Storage(err) => write!(f, "Storage error: {err}"),
            Self::Serialization(err) => write!(f, "Serialization error: {err}"),
        }
    }
}

impl Error for LasError {}

impl From<io::Error> for LasError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LasError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}
