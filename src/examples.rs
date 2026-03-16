use crate::{LasFile, ReadOptions, Result, read_path};
use std::path::{Path, PathBuf};

pub fn path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(relative)
}

pub fn open(relative: &str, options: &ReadOptions) -> Result<LasFile> {
    read_path(path(relative), options)
}
