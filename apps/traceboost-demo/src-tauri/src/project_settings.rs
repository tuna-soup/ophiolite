use crate::crs_registry::resolve_coordinate_reference;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PROJECT_SETTINGS_SCHEMA_VERSION: u32 = 1;
const TRACEBOOST_PROJECT_DIR: &str = ".traceboost";
const PROJECT_SETTINGS_FILENAME: &str = "project-settings.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectDisplayCoordinateReference {
    NativeEngineering,
    AuthorityCode {
        authority: String,
        code: String,
        #[serde(rename = "authId")]
        auth_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(alias = "coordinate_reference_id")]
    CoordinateReferenceId {
        #[serde(rename = "coordinateReferenceId")]
        coordinate_reference_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGeospatialSettings {
    pub schema_version: u32,
    pub display_coordinate_reference: ProjectDisplayCoordinateReference,
    pub source: String,
    pub created_at_unix_s: u64,
    pub updated_at_unix_s: u64,
}

pub fn load_project_geospatial_settings(
    project_root: impl AsRef<Path>,
) -> Result<Option<ProjectGeospatialSettings>, String> {
    let path = project_settings_path(project_root);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let settings =
        serde_json::from_slice::<ProjectGeospatialSettings>(&bytes).map_err(|error| {
            format!(
                "failed to parse project settings '{}': {error}",
                path.display()
            )
        })?;
    Ok(Some(normalize_settings(settings)?))
}

pub fn save_project_geospatial_settings(
    project_root: impl AsRef<Path>,
    display_coordinate_reference: ProjectDisplayCoordinateReference,
    source: &str,
) -> Result<ProjectGeospatialSettings, String> {
    let project_root = project_root.as_ref();
    let path = project_settings_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let now = unix_timestamp_s();
    let created_at_unix_s = load_project_geospatial_settings(project_root)?
        .map(|existing| existing.created_at_unix_s)
        .unwrap_or(now);
    let settings = normalize_settings(ProjectGeospatialSettings {
        schema_version: PROJECT_SETTINGS_SCHEMA_VERSION,
        display_coordinate_reference,
        source: normalize_source(source),
        created_at_unix_s,
        updated_at_unix_s: now,
    })?;
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(settings)
}

pub fn project_settings_path(project_root: impl AsRef<Path>) -> PathBuf {
    project_root
        .as_ref()
        .join(TRACEBOOST_PROJECT_DIR)
        .join(PROJECT_SETTINGS_FILENAME)
}

fn normalize_settings(
    mut settings: ProjectGeospatialSettings,
) -> Result<ProjectGeospatialSettings, String> {
    settings.schema_version = PROJECT_SETTINGS_SCHEMA_VERSION;
    settings.source = normalize_source(&settings.source);
    if settings.created_at_unix_s == 0 {
        settings.created_at_unix_s = settings.updated_at_unix_s.max(unix_timestamp_s());
    }
    if settings.updated_at_unix_s == 0 {
        settings.updated_at_unix_s = settings.created_at_unix_s;
    }
    match &mut settings.display_coordinate_reference {
        ProjectDisplayCoordinateReference::NativeEngineering => {}
        ProjectDisplayCoordinateReference::AuthorityCode {
            authority,
            code,
            auth_id,
            name,
        } => {
            let resolved = resolve_coordinate_reference(
                crate::crs_registry::ResolveCoordinateReferenceRequest {
                    authority: Some(authority.clone()),
                    code: Some(code.clone()),
                    auth_id: Some(auth_id.clone()),
                },
            )?;
            *authority = resolved.authority;
            *code = resolved.code;
            *auth_id = resolved.auth_id;
            *name = Some(resolved.name);
        }
        ProjectDisplayCoordinateReference::CoordinateReferenceId {
            coordinate_reference_id,
        } => {
            let resolved = resolve_coordinate_reference(
                crate::crs_registry::ResolveCoordinateReferenceRequest {
                    authority: None,
                    code: None,
                    auth_id: Some(coordinate_reference_id.trim().to_string()),
                },
            )?;
            settings.display_coordinate_reference =
                ProjectDisplayCoordinateReference::AuthorityCode {
                    authority: resolved.authority,
                    code: resolved.code,
                    auth_id: resolved.auth_id,
                    name: Some(resolved.name),
                };
        }
    }
    Ok(settings)
}

fn normalize_source(source: &str) -> String {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return "user_selected".to_string();
    }
    trimmed.to_string()
}

fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        ProjectDisplayCoordinateReference, load_project_geospatial_settings, project_settings_path,
        save_project_geospatial_settings,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("traceboost-{name}-{unique}"));
        fs::create_dir_all(&root).expect("failed to create temp project root");
        root
    }

    #[test]
    fn project_settings_round_trip_preserves_selection() {
        let project_root = temp_project_root("project-settings-round-trip");
        let saved = save_project_geospatial_settings(
            &project_root,
            ProjectDisplayCoordinateReference::AuthorityCode {
                authority: " epsg ".to_string(),
                code: " 23031 ".to_string(),
                auth_id: " EPSG:23031 ".to_string(),
                name: None,
            },
            " auto_seeded ",
        )
        .expect("failed to save project settings");

        let loaded = load_project_geospatial_settings(&project_root)
            .expect("failed to load project settings")
            .expect("settings should exist");

        assert_eq!(
            saved.display_coordinate_reference,
            loaded.display_coordinate_reference
        );
        assert_eq!(loaded.source, "auto_seeded");
        assert_eq!(
            loaded.display_coordinate_reference,
            ProjectDisplayCoordinateReference::AuthorityCode {
                authority: "EPSG".to_string(),
                code: "23031".to_string(),
                auth_id: "EPSG:23031".to_string(),
                name: Some("ED50 / UTM zone 31N".to_string()),
            }
        );
        assert!(project_settings_path(&project_root).exists());

        fs::remove_dir_all(&project_root).expect("failed to clean temp project root");
    }

    #[test]
    fn project_settings_load_normalizes_legacy_coordinate_reference_id_variant() {
        let project_root = temp_project_root("project-settings-legacy");
        let path = project_settings_path(&project_root);
        fs::create_dir_all(path.parent().expect("parent should exist"))
            .expect("failed to create project settings parent");
        fs::write(
            &path,
            r#"{
  "schemaVersion": 1,
  "displayCoordinateReference": {
    "kind": "coordinate_reference_id",
    "coordinateReferenceId": "EPSG:4326"
  },
  "source": "legacy",
  "createdAtUnixS": 1,
  "updatedAtUnixS": 1
}"#,
        )
        .expect("failed to seed legacy settings");

        let loaded = load_project_geospatial_settings(&project_root)
            .expect("failed to load project settings")
            .expect("settings should exist");

        assert_eq!(
            loaded.display_coordinate_reference,
            ProjectDisplayCoordinateReference::AuthorityCode {
                authority: "EPSG".to_string(),
                code: "4326".to_string(),
                auth_id: "EPSG:4326".to_string(),
                name: Some("WGS 84".to_string()),
            }
        );

        fs::remove_dir_all(&project_root).expect("failed to clean temp project root");
    }
}
