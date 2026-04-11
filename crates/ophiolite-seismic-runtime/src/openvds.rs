use std::path::Path;

use crate::error::SeismicStoreError;
use crate::ingest::IngestOptions;
use crate::store::StoreHandle;

pub fn looks_like_openvds_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("vds"))
        .unwrap_or(false)
}

pub fn ingest_openvds_store(
    input_root: impl AsRef<Path>,
    _store_root: impl AsRef<Path>,
    _options: IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    Err(SeismicStoreError::Message(format!(
        "OpenVDS import is not wired into this build yet: {}. The format boundary is reserved so the next adapter can map OpenVDS metadata and samples into tbvol without changing the import surface.",
        input_root.as_ref().display()
    )))
}
