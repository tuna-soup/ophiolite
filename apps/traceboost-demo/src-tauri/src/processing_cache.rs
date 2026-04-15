use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const CACHE_SCHEMA_VERSION: i64 = 1;
const SETTINGS_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingCacheSettings {
    pub schema_version: u32,
    #[serde(default)]
    pub enabled: bool,
}

impl Default for ProcessingCacheSettings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactArtifactHit {
    pub artifact_key: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixArtifactHit {
    pub artifact_key: String,
    pub path: String,
    pub prefix_len: usize,
}

pub struct ProcessingCacheState {
    settings: Mutex<ProcessingCacheSettings>,
    connection: Mutex<Connection>,
    #[cfg(test)]
    volumes_dir: std::path::PathBuf,
}

impl ProcessingCacheState {
    pub fn initialize(
        cache_dir: &Path,
        volumes_dir: &Path,
        index_path: &Path,
        settings_path: &Path,
    ) -> Result<Self, String> {
        fs::create_dir_all(cache_dir).map_err(|error| error.to_string())?;
        fs::create_dir_all(volumes_dir).map_err(|error| error.to_string())?;
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let settings = load_settings(settings_path)?;
        let connection = Connection::open(index_path).map_err(|error| error.to_string())?;
        initialize_schema(&connection)?;

        Ok(Self {
            settings: Mutex::new(settings),
            connection: Mutex::new(connection),
            #[cfg(test)]
            volumes_dir: volumes_dir.to_path_buf(),
        })
    }

    pub fn settings(&self) -> ProcessingCacheSettings {
        self.settings
            .lock()
            .expect("processing cache settings mutex poisoned")
            .clone()
    }

    pub fn enabled(&self) -> bool {
        self.settings().enabled
    }

    pub fn lookup_exact_visible_output(
        &self,
        family: &str,
        source_fingerprint: &str,
        full_pipeline_hash: &str,
    ) -> Result<Option<ExactArtifactHit>, String> {
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        let mut statement = connection
            .prepare(
                "SELECT artifact_key, path
                 FROM artifacts
                 WHERE valid = 1
                   AND kind = 'visible_final'
                   AND family = ?1
                   AND source_fingerprint = ?2
                   AND full_pipeline_hash = ?3
                 ORDER BY last_accessed_at_unix_s DESC, created_at_unix_s DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![family, source_fingerprint, full_pipeline_hash],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| error.to_string())?;

        let mut stale_keys = Vec::new();
        for row in rows {
            let (artifact_key, path) = row.map_err(|error| error.to_string())?;
            if Path::new(&path).exists() {
                connection
                    .execute(
                        "UPDATE artifacts
                         SET last_accessed_at_unix_s = ?2
                         WHERE artifact_key = ?1",
                        params![artifact_key, unix_timestamp_s() as i64],
                    )
                    .map_err(|error| error.to_string())?;
                return Ok(Some(ExactArtifactHit { artifact_key, path }));
            }
            stale_keys.push(artifact_key);
        }

        for artifact_key in stale_keys {
            connection
                .execute(
                    "UPDATE artifacts SET valid = 0 WHERE artifact_key = ?1",
                    params![artifact_key],
                )
                .map_err(|error| error.to_string())?;
        }

        Ok(None)
    }

    pub fn register_visible_output(
        &self,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        full_pipeline_hash: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_version: &str,
        store_format_version: &str,
    ) -> Result<(), String> {
        let bytes = file_size_bytes(Path::new(path))?;
        let now = unix_timestamp_s() as i64;
        let artifact_key = Self::fingerprint_json(&serde_json::json!({
            "kind": "visible_final",
            "family": family,
            "path": normalized_path_key(path),
            "source_fingerprint": source_fingerprint,
            "full_pipeline_hash": full_pipeline_hash,
        }))?;
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        connection
            .execute(
                "INSERT INTO artifacts (
                    artifact_key, kind, family, path, source_fingerprint, full_pipeline_hash,
                    prefix_hash, prefix_len, bytes, created_at_unix_s, last_accessed_at_unix_s,
                    protection_class, runtime_version, store_format_version, valid
                ) VALUES (
                    ?1, 'visible_final', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9,
                    'protected_visible_output', ?10, ?11, 1
                )
                ON CONFLICT(artifact_key) DO UPDATE SET
                    path = excluded.path,
                    bytes = excluded.bytes,
                    last_accessed_at_unix_s = excluded.last_accessed_at_unix_s,
                    runtime_version = excluded.runtime_version,
                    store_format_version = excluded.store_format_version,
                    valid = 1",
                params![
                    artifact_key,
                    family,
                    path,
                    source_fingerprint,
                    full_pipeline_hash,
                    prefix_hash,
                    prefix_len as i64,
                    bytes as i64,
                    now,
                    runtime_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn lookup_prefix_artifact(
        &self,
        family: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
    ) -> Result<Option<PrefixArtifactHit>, String> {
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        let mut statement = connection
            .prepare(
                "SELECT artifact_key, path, prefix_len
                 FROM artifacts
                 WHERE valid = 1
                   AND family = ?1
                   AND source_fingerprint = ?2
                   AND prefix_hash = ?3
                   AND prefix_len = ?4
                   AND kind = 'visible_checkpoint'
                 ORDER BY last_accessed_at_unix_s DESC, created_at_unix_s DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![family, source_fingerprint, prefix_hash, prefix_len as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(|error| error.to_string())?;

        let mut stale_keys = Vec::new();
        for row in rows {
            let (artifact_key, path, stored_prefix_len) = row.map_err(|error| error.to_string())?;
            if Path::new(&path).exists() {
                connection
                    .execute(
                        "UPDATE artifacts
                         SET last_accessed_at_unix_s = ?2
                         WHERE artifact_key = ?1",
                        params![artifact_key, unix_timestamp_s() as i64],
                    )
                    .map_err(|error| error.to_string())?;
                return Ok(Some(PrefixArtifactHit {
                    artifact_key,
                    path,
                    prefix_len: stored_prefix_len.max(0) as usize,
                }));
            }
            stale_keys.push(artifact_key);
        }

        for artifact_key in stale_keys {
            connection
                .execute(
                    "UPDATE artifacts SET valid = 0 WHERE artifact_key = ?1",
                    params![artifact_key],
                )
                .map_err(|error| error.to_string())?;
        }

        Ok(None)
    }

    #[cfg(test)]
    pub fn lookup_any_prefix_artifact(
        &self,
        family: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
    ) -> Result<Option<PrefixArtifactHit>, String> {
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        let mut statement = connection
            .prepare(
                "SELECT artifact_key, path, prefix_len
                 FROM artifacts
                 WHERE valid = 1
                   AND family = ?1
                   AND source_fingerprint = ?2
                   AND prefix_hash = ?3
                   AND prefix_len = ?4
                   AND kind IN ('visible_checkpoint', 'hidden_prefix')
                 ORDER BY last_accessed_at_unix_s DESC, created_at_unix_s DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![family, source_fingerprint, prefix_hash, prefix_len as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(|error| error.to_string())?;

        let mut stale_keys = Vec::new();
        for row in rows {
            let (artifact_key, path, stored_prefix_len) = row.map_err(|error| error.to_string())?;
            if Path::new(&path).exists() {
                connection
                    .execute(
                        "UPDATE artifacts
                         SET last_accessed_at_unix_s = ?2
                         WHERE artifact_key = ?1",
                        params![artifact_key, unix_timestamp_s() as i64],
                    )
                    .map_err(|error| error.to_string())?;
                return Ok(Some(PrefixArtifactHit {
                    artifact_key,
                    path,
                    prefix_len: stored_prefix_len.max(0) as usize,
                }));
            }
            stale_keys.push(artifact_key);
        }

        for artifact_key in stale_keys {
            connection
                .execute(
                    "UPDATE artifacts SET valid = 0 WHERE artifact_key = ?1",
                    params![artifact_key],
                )
                .map_err(|error| error.to_string())?;
        }

        Ok(None)
    }

    pub fn register_visible_checkpoint(
        &self,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_version: &str,
        store_format_version: &str,
    ) -> Result<(), String> {
        let bytes = file_size_bytes(Path::new(path))?;
        let now = unix_timestamp_s() as i64;
        let artifact_key = Self::fingerprint_json(&serde_json::json!({
            "kind": "visible_checkpoint",
            "family": family,
            "path": normalized_path_key(path),
            "source_fingerprint": source_fingerprint,
            "prefix_hash": prefix_hash,
            "prefix_len": prefix_len,
        }))?;
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        connection
            .execute(
                "INSERT INTO artifacts (
                    artifact_key, kind, family, path, source_fingerprint, full_pipeline_hash,
                    prefix_hash, prefix_len, bytes, created_at_unix_s, last_accessed_at_unix_s,
                    protection_class, runtime_version, store_format_version, valid
                ) VALUES (
                    ?1, 'visible_checkpoint', ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?8,
                    'protected_checkpoint', ?9, ?10, 1
                )
                ON CONFLICT(artifact_key) DO UPDATE SET
                    path = excluded.path,
                    bytes = excluded.bytes,
                    last_accessed_at_unix_s = excluded.last_accessed_at_unix_s,
                    runtime_version = excluded.runtime_version,
                    store_format_version = excluded.store_format_version,
                    valid = 1",
                params![
                    artifact_key,
                    family,
                    path,
                    source_fingerprint,
                    prefix_hash,
                    prefix_len as i64,
                    bytes as i64,
                    now,
                    runtime_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn fingerprint_bytes(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    pub fn fingerprint_json<T: Serialize>(value: &T) -> Result<String, String> {
        let payload = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        Ok(Self::fingerprint_bytes(&payload))
    }

    #[cfg(test)]
    pub fn register_hidden_prefix(
        &self,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_version: &str,
        store_format_version: &str,
    ) -> Result<(), String> {
        let bytes = file_size_bytes(Path::new(path))?;
        let now = unix_timestamp_s() as i64;
        let artifact_key = Self::fingerprint_json(&serde_json::json!({
            "kind": "hidden_prefix",
            "family": family,
            "path": normalized_path_key(path),
            "source_fingerprint": source_fingerprint,
            "prefix_hash": prefix_hash,
            "prefix_len": prefix_len,
        }))?;
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        connection
            .execute(
                "INSERT INTO artifacts (
                    artifact_key, kind, family, path, source_fingerprint, full_pipeline_hash,
                    prefix_hash, prefix_len, bytes, created_at_unix_s, last_accessed_at_unix_s,
                    protection_class, runtime_version, store_format_version, valid
                ) VALUES (
                    ?1, 'hidden_prefix', ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?8,
                    'hidden_prefix', ?9, ?10, 1
                )
                ON CONFLICT(artifact_key) DO UPDATE SET
                    path = excluded.path,
                    bytes = excluded.bytes,
                    last_accessed_at_unix_s = excluded.last_accessed_at_unix_s,
                    runtime_version = excluded.runtime_version,
                    store_format_version = excluded.store_format_version,
                    valid = 1",
                params![
                    artifact_key,
                    family,
                    path,
                    source_fingerprint,
                    prefix_hash,
                    prefix_len as i64,
                    bytes as i64,
                    now,
                    runtime_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(test)]
    pub fn volumes_dir(&self) -> &Path {
        &self.volumes_dir
    }
}

fn file_size_bytes(path: &Path) -> Result<u64, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }

    let mut total = 0u64;
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        total = total
            .checked_add(file_size_bytes(&entry.path())?)
            .ok_or_else(|| {
                format!(
                    "Processing cache artifact size overflow: {}",
                    path.display()
                )
            })?;
    }
    Ok(total)
}

fn normalized_path_key(path: &str) -> String {
    path.trim().replace('/', "\\").to_ascii_lowercase()
}

fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_settings(settings_path: &Path) -> Result<ProcessingCacheSettings, String> {
    if !settings_path.exists() {
        let settings = ProcessingCacheSettings::default();
        persist_settings(settings_path, &settings)?;
        return Ok(settings);
    }

    let bytes = fs::read(settings_path).map_err(|error| error.to_string())?;
    let settings = serde_json::from_slice::<ProcessingCacheSettings>(&bytes)
        .unwrap_or_else(|_| ProcessingCacheSettings::default());
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        let normalized = ProcessingCacheSettings {
            schema_version: SETTINGS_SCHEMA_VERSION,
            ..settings
        };
        persist_settings(settings_path, &normalized)?;
        return Ok(normalized);
    }
    Ok(settings)
}

fn persist_settings(
    settings_path: &Path,
    settings: &ProcessingCacheSettings,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(settings_path, bytes).map_err(|error| error.to_string())
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS artifacts (
                artifact_key TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                family TEXT NOT NULL,
                path TEXT NOT NULL,
                source_fingerprint TEXT NOT NULL,
                full_pipeline_hash TEXT,
                prefix_hash TEXT,
                prefix_len INTEGER,
                bytes INTEGER NOT NULL DEFAULT 0,
                created_at_unix_s INTEGER NOT NULL,
                last_accessed_at_unix_s INTEGER NOT NULL,
                protection_class TEXT NOT NULL,
                runtime_version TEXT NOT NULL,
                store_format_version TEXT NOT NULL,
                valid INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS artifact_refs (
                artifact_key TEXT NOT NULL,
                dataset_entry_id TEXT,
                session_pipeline_id TEXT,
                ref_kind TEXT NOT NULL,
                updated_at_unix_s INTEGER NOT NULL,
                PRIMARY KEY (artifact_key, dataset_entry_id, session_pipeline_id, ref_kind),
                FOREIGN KEY (artifact_key) REFERENCES artifacts(artifact_key) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_artifacts_lookup
                ON artifacts (family, source_fingerprint, full_pipeline_hash, prefix_hash, valid);

            CREATE INDEX IF NOT EXISTS idx_artifacts_priority
                ON artifacts (valid, protection_class, last_accessed_at_unix_s);

            CREATE INDEX IF NOT EXISTS idx_artifact_refs_dataset
                ON artifact_refs (dataset_entry_id);

            CREATE INDEX IF NOT EXISTS idx_artifact_refs_pipeline
                ON artifact_refs (session_pipeline_id);
            ",
        )
        .map_err(|error| error.to_string())?;

    let current_version = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'cache_schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if current_version.as_deref() != Some(&CACHE_SCHEMA_VERSION.to_string()) {
        connection
            .execute(
                "INSERT INTO metadata (key, value)
                 VALUES ('cache_schema_version', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![CACHE_SCHEMA_VERSION.to_string()],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "traceboost-processing-cache-{}-{}",
            name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&base).expect("create temp processing cache dir");
        base
    }

    #[test]
    fn initialize_creates_settings_and_schema() {
        let root = temp_dir("init");
        let cache = ProcessingCacheState::initialize(
            &root,
            &root.join("volumes"),
            &root.join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        assert!(cache.enabled());
        assert!(root.join("volumes").exists());
    }

    #[test]
    fn fingerprint_json_is_stable() {
        #[derive(Serialize)]
        struct Payload<'a> {
            name: &'a str,
            values: &'a [u32],
        }

        let left = ProcessingCacheState::fingerprint_json(&Payload {
            name: "pipeline",
            values: &[1, 2, 3],
        })
        .expect("fingerprint left");
        let right = ProcessingCacheState::fingerprint_json(&Payload {
            name: "pipeline",
            values: &[1, 2, 3],
        })
        .expect("fingerprint right");
        assert_eq!(left, right);
    }

    #[test]
    fn lookup_exact_visible_output_returns_latest_existing_path() {
        let root = temp_dir("lookup");
        let output = root.join("derived.tbvol");
        fs::create_dir_all(&output).expect("create derived output");
        fs::write(output.join("manifest.json"), b"{}").expect("write manifest");

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        cache
            .register_visible_output(
                "trace_local",
                &output.display().to_string(),
                "source-a",
                "pipeline-a",
                "pipeline-a",
                4,
                "dev",
                "tbvol-v1",
            )
            .expect("register visible output");

        let hit = cache
            .lookup_exact_visible_output("trace_local", "source-a", "pipeline-a")
            .expect("lookup visible output")
            .expect("expected exact hit");
        assert_eq!(hit.path, output.display().to_string());
    }

    #[test]
    fn lookup_prefix_artifact_returns_visible_checkpoint() {
        let root = temp_dir("prefix");
        let checkpoint = root.join("checkpoint.tbvol");
        fs::create_dir_all(&checkpoint).expect("create checkpoint output");
        fs::write(checkpoint.join("manifest.json"), b"{}").expect("write checkpoint manifest");

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        cache
            .register_visible_checkpoint(
                "trace_local",
                &checkpoint.display().to_string(),
                "source-a",
                "prefix-a",
                3,
                "dev",
                "tbvol-v1",
            )
            .expect("register visible checkpoint");

        let hit = cache
            .lookup_prefix_artifact("trace_local", "source-a", "prefix-a", 3)
            .expect("lookup prefix artifact")
            .expect("expected prefix hit");
        assert_eq!(hit.path, checkpoint.display().to_string());
        assert_eq!(hit.prefix_len, 3);
    }
}
