use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

const LEGACY_TRACEBOOST_LOGS_DIR_NAME: &str = "TraceBoost";

pub fn preferred_traceboost_logs_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Logs")
                .join(LEGACY_TRACEBOOST_LOGS_DIR_NAME)
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn resolve_logs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(logs_dir) = preferred_traceboost_logs_dir() {
        return Ok(logs_dir);
    }

    app.path().app_log_dir().map_err(|error| error.to_string())
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    logs_dir: PathBuf,
    pipeline_presets_dir: PathBuf,
    segy_import_recipes_dir: PathBuf,
    imported_volumes_dir: PathBuf,
    imported_gathers_dir: PathBuf,
    derived_volumes_dir: PathBuf,
    derived_gathers_dir: PathBuf,
    map_transform_cache_dir: PathBuf,
    processing_cache_dir: PathBuf,
    processing_cache_volumes_dir: PathBuf,
    processing_cache_index_path: PathBuf,
    dataset_registry_path: PathBuf,
    workspace_session_path: PathBuf,
    settings_path: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> Result<Self, String> {
        let logs_dir = resolve_logs_dir(app)?;
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        let pipeline_presets_dir = app_data_dir.join("processing-pipelines");
        let segy_import_recipes_dir = app_data_dir.join("segy-import-recipes");
        let imported_volumes_dir = app_data_dir.join("volumes");
        let imported_gathers_dir = app_data_dir.join("gathers");
        let derived_volumes_dir = app_data_dir.join("derived-volumes");
        let derived_gathers_dir = app_data_dir.join("derived-gathers");
        let map_transform_cache_dir = app_data_dir.join("map-transform-cache");
        let processing_cache_dir = app_data_dir.join("processing-cache");
        let processing_cache_volumes_dir = processing_cache_dir.join("volumes");
        let processing_cache_index_path = processing_cache_dir.join("index.sqlite");
        let dataset_registry_path = app_data_dir.join("workspace").join("dataset-registry.json");
        let workspace_session_path = app_data_dir.join("workspace").join("session.json");
        let settings_path = app_data_dir.join("settings.json");
        Ok(Self {
            logs_dir,
            pipeline_presets_dir,
            segy_import_recipes_dir,
            imported_volumes_dir,
            imported_gathers_dir,
            derived_volumes_dir,
            derived_gathers_dir,
            map_transform_cache_dir,
            processing_cache_dir,
            processing_cache_volumes_dir,
            processing_cache_index_path,
            dataset_registry_path,
            workspace_session_path,
            settings_path,
        })
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn pipeline_presets_dir(&self) -> &Path {
        &self.pipeline_presets_dir
    }

    pub fn segy_import_recipes_dir(&self) -> &Path {
        &self.segy_import_recipes_dir
    }

    pub fn imported_volumes_dir(&self) -> &Path {
        &self.imported_volumes_dir
    }

    pub fn imported_gathers_dir(&self) -> &Path {
        &self.imported_gathers_dir
    }

    pub fn derived_volumes_dir(&self) -> &Path {
        &self.derived_volumes_dir
    }

    pub fn derived_gathers_dir(&self) -> &Path {
        &self.derived_gathers_dir
    }

    pub fn map_transform_cache_dir(&self) -> &Path {
        &self.map_transform_cache_dir
    }

    pub fn processing_cache_dir(&self) -> &Path {
        &self.processing_cache_dir
    }

    pub fn processing_cache_volumes_dir(&self) -> &Path {
        &self.processing_cache_volumes_dir
    }

    pub fn processing_cache_index_path(&self) -> &Path {
        &self.processing_cache_index_path
    }

    pub fn dataset_registry_path(&self) -> &Path {
        &self.dataset_registry_path
    }

    pub fn workspace_session_path(&self) -> &Path {
        &self.workspace_session_path
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }
}
