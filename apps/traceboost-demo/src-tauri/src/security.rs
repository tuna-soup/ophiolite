use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::app_paths::AppPaths;

const STORE_HANDLE_PREFIX: &str = "storeh:";
const PROJECT_HANDLE_PREFIX: &str = "projh:";
const OUTPUT_GRANT_PREFIX: &str = "outg:";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputGrantPurpose {
    RuntimeStoreOutput,
    GatherStoreOutput,
    SegyExport,
    ZarrExport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantedPathSelection {
    pub path: String,
    pub handle_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputPathGrantSelection {
    pub path: String,
    pub grant_id: String,
}

#[derive(Debug)]
struct OutputGrant {
    path: PathBuf,
    purpose: OutputGrantPurpose,
}

#[derive(Debug, Default)]
struct SecurityRegistry {
    next_id: u64,
    store_handles: HashMap<String, PathBuf>,
    project_handles: HashMap<String, PathBuf>,
    output_grants: HashMap<String, OutputGrant>,
}

pub struct SecurityState {
    registry: Mutex<SecurityRegistry>,
}

impl Default for SecurityState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(SecurityRegistry::default()),
        }
    }
}

impl SecurityState {
    pub fn grant_store_handle(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<GrantedPathSelection, String> {
        let canonical = canonicalize_existing_path(path.as_ref())?;
        let path_string = canonical.display().to_string();
        let mut registry = self
            .registry
            .lock()
            .expect("security registry mutex poisoned");
        let handle_id = next_token(&mut registry, STORE_HANDLE_PREFIX);
        registry.store_handles.insert(handle_id.clone(), canonical);
        Ok(GrantedPathSelection {
            path: path_string,
            handle_id,
        })
    }

    pub fn grant_project_handle(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<GrantedPathSelection, String> {
        let canonical = canonicalize_existing_dir(path.as_ref())?;
        let path_string = canonical.display().to_string();
        let mut registry = self
            .registry
            .lock()
            .expect("security registry mutex poisoned");
        let handle_id = next_token(&mut registry, PROJECT_HANDLE_PREFIX);
        registry
            .project_handles
            .insert(handle_id.clone(), canonical);
        Ok(GrantedPathSelection {
            path: path_string,
            handle_id,
        })
    }

    pub fn authorize_managed_store(
        &self,
        app_paths: &AppPaths,
        path: impl AsRef<Path>,
    ) -> Result<GrantedPathSelection, String> {
        let canonical = canonicalize_existing_path(path.as_ref())?;
        if !is_within_any(
            &canonical,
            &[
                app_paths.imported_volumes_dir(),
                app_paths.imported_gathers_dir(),
                app_paths.derived_volumes_dir(),
                app_paths.derived_gathers_dir(),
            ],
        ) {
            return Err(
                "Store path is not authorized for this session. Reopen it through the native picker."
                    .to_string(),
            );
        }
        self.grant_store_handle(canonical)
    }

    pub fn grant_output_path(
        &self,
        path: impl AsRef<Path>,
        purpose: OutputGrantPurpose,
    ) -> Result<OutputPathGrantSelection, String> {
        let normalized = normalize_output_path(path.as_ref())?;
        let path_string = normalized.display().to_string();
        let mut registry = self
            .registry
            .lock()
            .expect("security registry mutex poisoned");
        let grant_id = next_token(&mut registry, OUTPUT_GRANT_PREFIX);
        registry.output_grants.insert(
            grant_id.clone(),
            OutputGrant {
                path: normalized,
                purpose,
            },
        );
        Ok(OutputPathGrantSelection {
            path: path_string,
            grant_id,
        })
    }

    pub fn authorize_managed_output(
        &self,
        app_paths: &AppPaths,
        path: impl AsRef<Path>,
        purpose: OutputGrantPurpose,
    ) -> Result<OutputPathGrantSelection, String> {
        let normalized = normalize_output_path(path.as_ref())?;
        let allowed = match purpose {
            OutputGrantPurpose::RuntimeStoreOutput => is_within_any(
                &normalized,
                &[
                    app_paths.imported_volumes_dir(),
                    app_paths.derived_volumes_dir(),
                ],
            ),
            OutputGrantPurpose::GatherStoreOutput => is_within_any(
                &normalized,
                &[
                    app_paths.imported_gathers_dir(),
                    app_paths.derived_gathers_dir(),
                ],
            ),
            OutputGrantPurpose::SegyExport | OutputGrantPurpose::ZarrExport => false,
        };
        if !allowed {
            return Err(
                "Output path is not authorized for this session. Use the native save dialog to choose it."
                    .to_string(),
            );
        }
        self.grant_output_path(normalized, purpose)
    }

    pub fn resolve_store_path(&self, handle: &str) -> Result<PathBuf, String> {
        if !handle.starts_with(STORE_HANDLE_PREFIX) {
            return Err(
                "Runtime store access requires a session handle. Reopen the store through the native picker."
                    .to_string(),
            );
        }
        let registry = self
            .registry
            .lock()
            .expect("security registry mutex poisoned");
        registry
            .store_handles
            .get(handle)
            .cloned()
            .ok_or_else(|| "Unknown or expired runtime store handle.".to_string())
    }

    pub fn resolve_project_root(&self, handle: &str) -> Result<PathBuf, String> {
        if !handle.starts_with(PROJECT_HANDLE_PREFIX) {
            return Err(
                "Project access requires a session handle. Re-select the project root through the native picker."
                    .to_string(),
            );
        }
        let registry = self
            .registry
            .lock()
            .expect("security registry mutex poisoned");
        registry
            .project_handles
            .get(handle)
            .cloned()
            .ok_or_else(|| "Unknown or expired project handle.".to_string())
    }

    pub fn consume_output_path(
        &self,
        grant_id: &str,
        purpose: OutputGrantPurpose,
    ) -> Result<PathBuf, String> {
        if !grant_id.starts_with(OUTPUT_GRANT_PREFIX) {
            return Err(
                "Output writes require a fresh save grant. Use the native save dialog before running this action."
                    .to_string(),
            );
        }
        let mut registry = self
            .registry
            .lock()
            .expect("security registry mutex poisoned");
        let grant = registry
            .output_grants
            .remove(grant_id)
            .ok_or_else(|| "Unknown or expired output path grant.".to_string())?;
        if grant.purpose != purpose {
            return Err("Output grant purpose does not match the requested action.".to_string());
        }
        Ok(grant.path)
    }

    pub fn pick_runtime_store(
        &self,
        app: &AppHandle,
    ) -> Result<Option<GrantedPathSelection>, String> {
        let selected = app
            .dialog()
            .file()
            .set_title("Open Volume")
            .add_filter("Runtime Stores", &["tbvol"])
            .add_filter("All Files", &["*"])
            .blocking_pick_file();
        match selected {
            Some(path) => {
                let path = path.into_path().map_err(|error| error.to_string())?;
                self.grant_store_handle(path).map(Some)
            }
            None => Ok(None),
        }
    }

    pub fn pick_project_root(
        &self,
        app: &AppHandle,
        title: Option<&str>,
    ) -> Result<Option<GrantedPathSelection>, String> {
        let dialog = app
            .dialog()
            .file()
            .set_title(title.unwrap_or("Select Ophiolite Project Root"));
        #[cfg(desktop)]
        {
            let selected = dialog.blocking_pick_folder();
            return match selected {
                Some(path) => {
                    let path = path.into_path().map_err(|error| error.to_string())?;
                    self.grant_project_handle(path).map(Some)
                }
                None => Ok(None),
            };
        }
        #[allow(unreachable_code)]
        Ok(None)
    }

    pub fn pick_output_path(
        &self,
        app: &AppHandle,
        default_path: &str,
        purpose: OutputGrantPurpose,
    ) -> Result<Option<OutputPathGrantSelection>, String> {
        let mut dialog = app
            .dialog()
            .file()
            .set_file_name(file_name_from_default(default_path));
        if let Some(directory) = parent_dir_from_default(default_path) {
            dialog = dialog.set_directory(directory);
        }
        dialog = match purpose {
            OutputGrantPurpose::RuntimeStoreOutput => dialog
                .set_title("Set Runtime Store Output Path")
                .add_filter("Runtime Store", &["tbvol"])
                .add_filter("All Files", &["*"]),
            OutputGrantPurpose::GatherStoreOutput => dialog
                .set_title("Set Gather Output Path")
                .add_filter("Gather Store", &["tbgath"])
                .add_filter("All Files", &["*"]),
            OutputGrantPurpose::SegyExport => dialog
                .set_title("Export SEG-Y")
                .add_filter("SEG-Y", &["sgy", "segy"])
                .add_filter("All Files", &["*"]),
            OutputGrantPurpose::ZarrExport => dialog
                .set_title("Export Zarr")
                .add_filter("Zarr Store", &["zarr"])
                .add_filter("All Files", &["*"]),
        };
        let selected = dialog.blocking_save_file();
        match selected {
            Some(path) => {
                let path = path.into_path().map_err(|error| error.to_string())?;
                self.grant_output_path(path, purpose).map(Some)
            }
            None => Ok(None),
        }
    }
}

fn next_token(registry: &mut SecurityRegistry, prefix: &str) -> String {
    registry.next_id += 1;
    format!("{prefix}{:016x}", registry.next_id)
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    path.canonicalize().map_err(|error| error.to_string())
}

fn canonicalize_existing_dir(path: &Path) -> Result<PathBuf, String> {
    let canonical = canonicalize_existing_path(path)?;
    if !canonical.is_dir() {
        return Err(format!("Directory does not exist: {}", canonical.display()));
    }
    Ok(canonical)
}

fn normalize_output_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path.file_name().ok_or_else(|| {
        format!(
            "Output path is missing a file or directory name: {}",
            path.display()
        )
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Err(format!(
            "Output parent directory does not exist: {}",
            parent.display()
        ));
    }
    let canonical_parent = parent.canonicalize().map_err(|error| error.to_string())?;
    Ok(canonical_parent.join(file_name))
}

fn is_within_any(candidate: &Path, roots: &[&Path]) -> bool {
    roots.iter().any(|root| is_within(candidate, root))
}

fn is_within(candidate: &Path, root: &Path) -> bool {
    root.canonicalize()
        .ok()
        .is_some_and(|canonical_root| candidate.starts_with(&canonical_root))
}

fn file_name_from_default(default_path: &str) -> String {
    Path::new(default_path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("output")
        .to_string()
}

fn parent_dir_from_default(default_path: &str) -> Option<PathBuf> {
    let parent = Path::new(default_path).parent()?;
    if parent.as_os_str().is_empty() {
        return None;
    }
    Some(parent.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("traceboost-security-{label}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn store_handles_require_known_session_handle() {
        let root = temp_dir("store-handle");
        let store_path = root.join("demo.tbvol");
        fs::write(&store_path, b"demo").expect("write temp store");

        let security = SecurityState::default();
        let selection = security
            .grant_store_handle(&store_path)
            .expect("grant store handle");

        assert!(selection.handle_id.starts_with(STORE_HANDLE_PREFIX));
        assert_eq!(
            security
                .resolve_store_path(&selection.handle_id)
                .expect("resolve handle"),
            store_path.canonicalize().expect("canonicalize store")
        );
        assert!(
            security
                .resolve_store_path(store_path.to_string_lossy().as_ref())
                .is_err()
        );
        assert!(security.resolve_store_path("storeh:deadbeef").is_err());
    }

    #[test]
    fn project_handles_require_directories() {
        let root = temp_dir("project-handle");
        let file_path = root.join("not-a-directory.txt");
        fs::write(&file_path, b"demo").expect("write temp file");

        let security = SecurityState::default();
        let selection = security
            .grant_project_handle(&root)
            .expect("grant project handle");

        assert!(selection.handle_id.starts_with(PROJECT_HANDLE_PREFIX));
        assert_eq!(
            security
                .resolve_project_root(&selection.handle_id)
                .expect("resolve handle"),
            root.canonicalize().expect("canonicalize root")
        );
        assert!(security.grant_project_handle(&file_path).is_err());
    }

    #[test]
    fn output_grants_are_one_time_and_purpose_bound() {
        let root = temp_dir("output-grant");
        let output_path = root.join("derived.tbvol");
        let security = SecurityState::default();

        let selection = security
            .grant_output_path(&output_path, OutputGrantPurpose::RuntimeStoreOutput)
            .expect("grant output path");
        let normalized_output_path =
            normalize_output_path(&output_path).expect("normalize output path");

        assert!(selection.grant_id.starts_with(OUTPUT_GRANT_PREFIX));
        assert_eq!(
            security
                .consume_output_path(&selection.grant_id, OutputGrantPurpose::RuntimeStoreOutput)
                .expect("consume output grant"),
            normalized_output_path
        );
        assert!(
            security
                .consume_output_path(&selection.grant_id, OutputGrantPurpose::RuntimeStoreOutput)
                .is_err()
        );

        let second = security
            .grant_output_path(&output_path, OutputGrantPurpose::RuntimeStoreOutput)
            .expect("grant second output path");
        assert!(
            security
                .consume_output_path(&second.grant_id, OutputGrantPurpose::SegyExport)
                .is_err()
        );
        assert!(
            security
                .consume_output_path(
                    output_path.to_string_lossy().as_ref(),
                    OutputGrantPurpose::RuntimeStoreOutput
                )
                .is_err()
        );
    }
}
