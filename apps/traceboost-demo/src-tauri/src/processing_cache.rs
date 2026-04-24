use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use seis_runtime::{
    ArtifactBoundaryReason, ArtifactKey, ChunkGridSpec, GeometryFingerprints, LogicalDomain,
    OperatorSetIdentity, PipelineSemanticIdentity, PlannerProfileIdentity, ProcessingArtifactRole,
    ProcessingLineage, ProcessingPipelineSpec, SourceSemanticIdentity,
    canonical_processing_lineage_validation, source_identity_digest,
};
use serde::{Deserialize, Serialize};

const CACHE_SCHEMA_VERSION: i64 = 2;
const SETTINGS_SCHEMA_VERSION: u32 = 3;

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

#[derive(Debug, Deserialize)]
struct CachedTbvolManifest {
    format: String,
    version: u32,
    #[serde(default)]
    tile_shape: Option<[usize; 3]>,
    volume: CachedTbvolVolume,
}

#[derive(Debug, Deserialize)]
struct CachedTbvolVolume {
    shape: [usize; 3],
    processing_lineage: Option<CachedProcessingLineage>,
}

#[derive(Debug, Deserialize)]
struct CachedProcessingLineage {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    parent_store: PathBuf,
    #[serde(default)]
    parent_store_id: String,
    artifact_role: ProcessingArtifactRole,
    pipeline: ProcessingPipelineSpec,
    #[serde(default)]
    pipeline_identity: Option<PipelineSemanticIdentity>,
    #[serde(default)]
    operator_set_identity: Option<OperatorSetIdentity>,
    #[serde(default)]
    planner_profile_identity: Option<PlannerProfileIdentity>,
    #[serde(default)]
    source_identity: Option<SourceSemanticIdentity>,
    #[serde(default)]
    runtime_semantics_version: String,
    #[serde(default)]
    store_writer_semantics_version: String,
    #[serde(default)]
    #[serde(rename = "runtime_version")]
    runtime_version: String,
    #[serde(default)]
    created_at_unix_s: u64,
    #[serde(default)]
    artifact_key: Option<ArtifactKey>,
    #[serde(default)]
    input_artifact_keys: Vec<ArtifactKey>,
    #[serde(default)]
    produced_by_stage_id: Option<String>,
    #[serde(default)]
    boundary_reason: Option<ArtifactBoundaryReason>,
    #[serde(default)]
    logical_domain: Option<LogicalDomain>,
    #[serde(default)]
    chunk_grid_spec: Option<ChunkGridSpec>,
    #[serde(default)]
    geometry_fingerprints: Option<GeometryFingerprints>,
}

impl From<CachedProcessingLineage> for ProcessingLineage {
    fn from(value: CachedProcessingLineage) -> Self {
        Self {
            schema_version: value.schema_version,
            parent_store: value.parent_store,
            parent_store_id: value.parent_store_id,
            artifact_role: value.artifact_role,
            pipeline: value.pipeline,
            pipeline_identity: value.pipeline_identity,
            operator_set_identity: value.operator_set_identity,
            planner_profile_identity: value.planner_profile_identity,
            source_identity: value.source_identity,
            runtime_semantics_version: value.runtime_semantics_version,
            store_writer_semantics_version: value.store_writer_semantics_version,
            runtime_version: value.runtime_version,
            created_at_unix_s: value.created_at_unix_s,
            artifact_key: value.artifact_key,
            input_artifact_keys: value.input_artifact_keys,
            produced_by_stage_id: value.produced_by_stage_id,
            boundary_reason: value.boundary_reason,
            logical_domain: value.logical_domain,
            chunk_grid_spec: value.chunk_grid_spec,
            geometry_fingerprints: value.geometry_fingerprints,
        }
    }
}

struct CachedArtifactValidation<'a> {
    family: &'a str,
    artifact_role: ProcessingArtifactRole,
    expected_pipeline_hash: &'a str,
    expected_runtime_semantics_version: &'a str,
    expected_store_writer_semantics_version: &'a str,
    expected_store_format_version: &'a str,
    expected_source_fingerprint: Option<&'a str>,
    expected_artifact_key: Option<&'a str>,
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

    #[allow(dead_code)]
    pub fn lookup_exact_visible_output(
        &self,
        family: &str,
        source_fingerprint: &str,
        full_pipeline_hash: &str,
        runtime_semantics_version: &str,
        store_writer_semantics_version: &str,
        store_format_version: &str,
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
                   AND runtime_version = ?4
                   AND store_format_version = ?5
                 ORDER BY last_accessed_at_unix_s DESC, created_at_unix_s DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![
                    family,
                    source_fingerprint,
                    full_pipeline_hash,
                    runtime_semantics_version,
                    store_format_version,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| error.to_string())?;

        let mut stale_keys = Vec::new();
        for row in rows {
            let (artifact_key, path) = row.map_err(|error| error.to_string())?;
            if validate_cached_artifact_manifest(
                &path,
                CachedArtifactValidation {
                    family,
                    artifact_role: ProcessingArtifactRole::FinalOutput,
                    expected_pipeline_hash: full_pipeline_hash,
                    expected_runtime_semantics_version: runtime_semantics_version,
                    expected_store_writer_semantics_version: store_writer_semantics_version,
                    expected_store_format_version: store_format_version,
                    expected_source_fingerprint: Some(source_fingerprint),
                    expected_artifact_key: Some(artifact_key.as_str()),
                },
            ) {
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

        mark_artifacts_invalid(&connection, &stale_keys)?;

        Ok(None)
    }

    pub fn lookup_exact_visible_output_by_artifact_key(
        &self,
        artifact_key: &str,
        family: &str,
        full_pipeline_hash: &str,
        runtime_semantics_version: &str,
        store_writer_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<Option<ExactArtifactHit>, String> {
        self.lookup_artifact_by_key(
            artifact_key,
            family,
            ProcessingArtifactRole::FinalOutput,
            full_pipeline_hash,
            runtime_semantics_version,
            store_writer_semantics_version,
            store_format_version,
        )
        .map(|hit| {
            hit.map(|hit| ExactArtifactHit {
                artifact_key: hit.artifact_key,
                path: hit.path,
            })
        })
    }

    #[allow(dead_code)]
    pub fn register_visible_output(
        &self,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        full_pipeline_hash: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<String, String> {
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
                    runtime_semantics_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(artifact_key)
    }

    pub fn register_visible_output_with_artifact_key(
        &self,
        artifact_key: &str,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        full_pipeline_hash: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<String, String> {
        self.register_visible_output_internal(
            artifact_key,
            family,
            path,
            source_fingerprint,
            full_pipeline_hash,
            prefix_hash,
            prefix_len,
            runtime_semantics_version,
            store_format_version,
            "visible_final",
            "protected_visible_output",
        )
    }

    #[allow(dead_code)]
    pub fn lookup_prefix_artifact(
        &self,
        family: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_writer_semantics_version: &str,
        store_format_version: &str,
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
                   AND runtime_version = ?5
                   AND store_format_version = ?6
                 ORDER BY last_accessed_at_unix_s DESC, created_at_unix_s DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![
                    family,
                    source_fingerprint,
                    prefix_hash,
                    prefix_len as i64,
                    runtime_semantics_version,
                    store_format_version,
                ],
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
            if validate_cached_artifact_manifest(
                &path,
                CachedArtifactValidation {
                    family,
                    artifact_role: ProcessingArtifactRole::Checkpoint,
                    expected_pipeline_hash: prefix_hash,
                    expected_runtime_semantics_version: runtime_semantics_version,
                    expected_store_writer_semantics_version: store_writer_semantics_version,
                    expected_store_format_version: store_format_version,
                    expected_source_fingerprint: Some(source_fingerprint),
                    expected_artifact_key: Some(artifact_key.as_str()),
                },
            ) {
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

        mark_artifacts_invalid(&connection, &stale_keys)?;

        Ok(None)
    }

    pub fn lookup_prefix_artifact_by_artifact_key(
        &self,
        artifact_key: &str,
        family: &str,
        prefix_hash: &str,
        runtime_semantics_version: &str,
        store_writer_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<Option<PrefixArtifactHit>, String> {
        self.lookup_artifact_by_key(
            artifact_key,
            family,
            ProcessingArtifactRole::Checkpoint,
            prefix_hash,
            runtime_semantics_version,
            store_writer_semantics_version,
            store_format_version,
        )
    }

    #[cfg(test)]
    pub fn lookup_any_prefix_artifact(
        &self,
        family: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_writer_semantics_version: &str,
        store_format_version: &str,
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
                   AND runtime_version = ?5
                   AND store_format_version = ?6
                 ORDER BY last_accessed_at_unix_s DESC, created_at_unix_s DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![
                    family,
                    source_fingerprint,
                    prefix_hash,
                    prefix_len as i64,
                    runtime_semantics_version,
                    store_format_version,
                ],
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
            if validate_cached_artifact_manifest(
                &path,
                CachedArtifactValidation {
                    family,
                    artifact_role: ProcessingArtifactRole::Checkpoint,
                    expected_pipeline_hash: prefix_hash,
                    expected_runtime_semantics_version: runtime_semantics_version,
                    expected_store_writer_semantics_version: store_writer_semantics_version,
                    expected_store_format_version: store_format_version,
                    expected_source_fingerprint: Some(source_fingerprint),
                    expected_artifact_key: Some(artifact_key.as_str()),
                },
            ) {
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

        mark_artifacts_invalid(&connection, &stale_keys)?;

        Ok(None)
    }

    #[allow(dead_code)]
    pub fn register_visible_checkpoint(
        &self,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<String, String> {
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
                    runtime_semantics_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(artifact_key)
    }

    pub fn register_visible_checkpoint_with_artifact_key(
        &self,
        artifact_key: &str,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<String, String> {
        self.register_visible_output_internal(
            artifact_key,
            family,
            path,
            source_fingerprint,
            prefix_hash,
            prefix_hash,
            prefix_len,
            runtime_semantics_version,
            store_format_version,
            "visible_checkpoint",
            "protected_checkpoint",
        )
    }
    #[allow(dead_code)]
    pub fn fingerprint_bytes(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    #[allow(dead_code)]
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
        runtime_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<String, String> {
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
                    runtime_semantics_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(artifact_key)
    }

    pub fn upsert_dataset_artifact_ref(
        &self,
        artifact_key: &str,
        dataset_entry_id: &str,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        connection
            .execute(
                "INSERT INTO artifact_refs (
                    artifact_key, dataset_entry_id, session_pipeline_id, ref_kind, updated_at_unix_s
                ) VALUES (?1, ?2, '', 'dataset_entry', ?3)
                ON CONFLICT(artifact_key, dataset_entry_id, session_pipeline_id, ref_kind)
                DO UPDATE SET updated_at_unix_s = excluded.updated_at_unix_s",
                params![artifact_key, dataset_entry_id, unix_timestamp_s() as i64],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn upsert_session_pipeline_artifact_ref(
        &self,
        artifact_key: &str,
        session_pipeline_id: &str,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        connection
            .execute(
                "INSERT INTO artifact_refs (
                    artifact_key, dataset_entry_id, session_pipeline_id, ref_kind, updated_at_unix_s
                ) VALUES (?1, '', ?2, 'session_pipeline', ?3)
                ON CONFLICT(artifact_key, dataset_entry_id, session_pipeline_id, ref_kind)
                DO UPDATE SET updated_at_unix_s = excluded.updated_at_unix_s",
                params![artifact_key, session_pipeline_id, unix_timestamp_s() as i64],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(test)]
    pub fn volumes_dir(&self) -> &Path {
        &self.volumes_dir
    }

    fn lookup_artifact_by_key(
        &self,
        artifact_key: &str,
        family: &str,
        artifact_role: ProcessingArtifactRole,
        expected_pipeline_hash: &str,
        runtime_semantics_version: &str,
        store_writer_semantics_version: &str,
        store_format_version: &str,
    ) -> Result<Option<PrefixArtifactHit>, String> {
        let connection = self
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        let row = connection
            .query_row(
                "SELECT artifact_key, path, COALESCE(prefix_len, 0)
                 FROM artifacts
                 WHERE valid = 1
                   AND artifact_key = ?1
                   AND family = ?2
                   AND runtime_version = ?3
                   AND store_format_version = ?4",
                params![
                    artifact_key,
                    family,
                    runtime_semantics_version,
                    store_format_version
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((artifact_key, path, prefix_len)) = row else {
            return Ok(None);
        };
        if validate_cached_artifact_manifest(
            &path,
            CachedArtifactValidation {
                family,
                artifact_role,
                expected_pipeline_hash,
                expected_runtime_semantics_version: runtime_semantics_version,
                expected_store_writer_semantics_version: store_writer_semantics_version,
                expected_store_format_version: store_format_version,
                expected_source_fingerprint: None,
                expected_artifact_key: Some(artifact_key.as_str()),
            },
        ) {
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
                prefix_len: prefix_len.max(0) as usize,
            }));
        }
        mark_artifacts_invalid(&connection, &[artifact_key])?;
        Ok(None)
    }

    fn register_visible_output_internal(
        &self,
        artifact_key: &str,
        family: &str,
        path: &str,
        source_fingerprint: &str,
        full_pipeline_hash: &str,
        prefix_hash: &str,
        prefix_len: usize,
        runtime_semantics_version: &str,
        store_format_version: &str,
        kind: &str,
        protection_class: &str,
    ) -> Result<String, String> {
        let bytes = file_size_bytes(Path::new(path))?;
        let now = unix_timestamp_s() as i64;
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
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10,
                    ?11, ?12, ?13, 1
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
                    kind,
                    family,
                    path,
                    source_fingerprint,
                    full_pipeline_hash,
                    prefix_hash,
                    prefix_len as i64,
                    bytes as i64,
                    now,
                    protection_class,
                    runtime_semantics_version,
                    store_format_version,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(artifact_key.to_string())
    }
}

fn mark_artifacts_invalid(connection: &Connection, artifact_keys: &[String]) -> Result<(), String> {
    for artifact_key in artifact_keys {
        connection
            .execute(
                "UPDATE artifacts SET valid = 0 WHERE artifact_key = ?1",
                params![artifact_key],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn validate_cached_artifact_manifest(path: &str, expected: CachedArtifactValidation<'_>) -> bool {
    let manifest_path = Path::new(path).join("manifest.json");
    let Ok(bytes) = fs::read(&manifest_path) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_slice::<CachedTbvolManifest>(&bytes) else {
        return false;
    };
    if format!("{}@{}", manifest.format, manifest.version) != expected.expected_store_format_version
    {
        return false;
    }
    let Some(lineage) = manifest.volume.processing_lineage else {
        return false;
    };
    let lineage: ProcessingLineage = lineage.into();
    let canonical_lineage = lineage.pipeline_identity.is_some()
        && lineage.operator_set_identity.is_some()
        && lineage.planner_profile_identity.is_some()
        && lineage.source_identity.is_some()
        && lineage.artifact_key.is_some()
        && !lineage.runtime_semantics_version.trim().is_empty()
        && !lineage.store_writer_semantics_version.trim().is_empty();
    if !canonical_lineage {
        return false;
    }
    if lineage.artifact_role != expected.artifact_role
        || lineage.runtime_semantics_version != expected.expected_runtime_semantics_version
        || lineage.store_writer_semantics_version
            != expected.expected_store_writer_semantics_version
    {
        return false;
    }
    let layout = match lineage.source_identity.as_ref() {
        Some(source_identity) => source_identity.layout,
        None => return false,
    };
    let chunk_shape = manifest.tile_shape.unwrap_or(manifest.volume.shape);
    let Ok(validation) = canonical_processing_lineage_validation(
        &lineage,
        layout,
        manifest.volume.shape,
        chunk_shape,
        Some(expected.artifact_role),
    ) else {
        return false;
    };
    let artifact_key = &validation.artifact_key;
    if let Some(expected_artifact_key) = expected.expected_artifact_key {
        if artifact_key.cache_key.as_str() != expected_artifact_key {
            return false;
        }
    }
    if let Some(expected_source_fingerprint) = expected.expected_source_fingerprint {
        let Some(source_identity) = lineage.source_identity.as_ref() else {
            return false;
        };
        let Ok(source_fingerprint) = source_identity_digest(source_identity) else {
            return false;
        };
        if source_fingerprint != expected_source_fingerprint {
            return false;
        }
    }

    let Some(pipeline_identity) = lineage.pipeline_identity.as_ref() else {
        return false;
    };
    if pipeline_identity.content_digest != expected.expected_pipeline_hash {
        return false;
    }

    match expected.family {
        "trace_local" => matches!(lineage.pipeline, ProcessingPipelineSpec::TraceLocal { .. }),
        "post_stack_neighborhood" => matches!(
            lineage.pipeline,
            ProcessingPipelineSpec::PostStackNeighborhood { .. }
        ),
        "subvolume" => matches!(lineage.pipeline, ProcessingPipelineSpec::Subvolume { .. }),
        "gather" => matches!(lineage.pipeline, ProcessingPipelineSpec::Gather { .. }),
        _ => false,
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

#[allow(dead_code)]
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
    let settings = match serde_json::from_slice::<ProcessingCacheSettings>(&bytes) {
        Ok(settings) if settings.schema_version == SETTINGS_SCHEMA_VERSION => return Ok(settings),
        Ok(settings) => {
            eprintln!(
                "Processing cache settings schema mismatch at {} (found {}, expected {}); disabling cache.",
                settings_path.display(),
                settings.schema_version,
                SETTINGS_SCHEMA_VERSION
            );
            ProcessingCacheSettings {
                schema_version: SETTINGS_SCHEMA_VERSION,
                enabled: false,
            }
        }
        Err(error) => {
            eprintln!(
                "Failed to parse processing cache settings at {}: {}; disabling cache.",
                settings_path.display(),
                error
            );
            ProcessingCacheSettings {
                schema_version: SETTINGS_SCHEMA_VERSION,
                enabled: false,
            }
        }
    };
    persist_settings(settings_path, &settings)?;
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
            .execute("DELETE FROM artifact_refs", [])
            .map_err(|error| error.to_string())?;
        connection
            .execute("DELETE FROM artifacts", [])
            .map_err(|error| error.to_string())?;
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
    use ophiolite::SeismicLayout;
    use seis_runtime::{
        CURRENT_RUNTIME_SEMANTICS_VERSION, CURRENT_STORE_WRITER_SEMANTICS_VERSION,
        CanonicalIdentityStatus, MaterializationClass, SourceSemanticIdentity, StoreFormatIdentity,
        TraceLocalProcessingOperation, TraceLocalProcessingPipeline, TraceLocalProcessingStep,
        canonical_artifact_identity, operator_set_identity_for_pipeline,
        pipeline_semantic_identity, planner_profile_identity_for_pipeline, source_identity_digest,
        trace_local_pipeline_hash,
    };
    use std::path::Path;
    use std::path::PathBuf;

    const TEST_STORE_FORMAT_VERSION: &str = "tbvol@2";

    fn sample_trace_local_pipeline() -> TraceLocalProcessingPipeline {
        TraceLocalProcessingPipeline {
            schema_version: 2,
            revision: 1,
            preset_id: None,
            name: Some("cache-test".to_string()),
            description: None,
            steps: vec![TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 1.25 },
                checkpoint: false,
            }],
        }
    }

    fn sample_source_identity() -> SourceSemanticIdentity {
        SourceSemanticIdentity {
            schema_version: 1,
            store_id: "source-store-id".to_string(),
            store_format: StoreFormatIdentity {
                schema_version: 1,
                store_kind: "tbvol".to_string(),
                store_format_version: TEST_STORE_FORMAT_VERSION.to_string(),
            },
            layout: SeismicLayout::PostStack3D,
            shape: Some([1, 1, 1]),
            chunk_shape: Some([1, 1, 1]),
            sample_type: Some("f32".to_string()),
            endianness: Some("little".to_string()),
            parent_artifact_key: None,
        }
    }

    fn sample_source_fingerprint() -> String {
        source_identity_digest(&sample_source_identity()).expect("source fingerprint")
    }

    fn write_trace_local_manifest(
        store: &Path,
        artifact_role: ProcessingArtifactRole,
        pipeline: &TraceLocalProcessingPipeline,
    ) -> String {
        let source_identity = sample_source_identity();
        let pipeline_spec = ProcessingPipelineSpec::TraceLocal {
            pipeline: pipeline.clone(),
        };
        let pipeline_identity =
            pipeline_semantic_identity(&pipeline_spec).expect("pipeline identity");
        let operator_set_identity =
            operator_set_identity_for_pipeline(&pipeline_spec).expect("operator set identity");
        let planner_profile_identity = planner_profile_identity_for_pipeline(&pipeline_spec)
            .expect("planner profile identity");
        let canonical_artifact = canonical_artifact_identity(
            &source_identity,
            CanonicalIdentityStatus::Canonical,
            &pipeline_identity,
            &operator_set_identity,
            &planner_profile_identity,
            SeismicLayout::PostStack3D,
            [1, 1, 1],
            [1, 1, 1],
            artifact_role,
            match artifact_role {
                ProcessingArtifactRole::Checkpoint => ArtifactBoundaryReason::AuthoredCheckpoint,
                ProcessingArtifactRole::FinalOutput => ArtifactBoundaryReason::FinalOutput,
            },
            match artifact_role {
                ProcessingArtifactRole::Checkpoint => MaterializationClass::Checkpoint,
                ProcessingArtifactRole::FinalOutput => MaterializationClass::PublishedOutput,
            },
            LogicalDomain::Volume {
                volume: seis_runtime::VolumeDomain { shape: [1, 1, 1] },
            },
        )
        .expect("canonical artifact identity")
        .expect("canonical artifact");
        let artifact_cache_key = canonical_artifact.artifact_key.cache_key.clone();
        let manifest = serde_json::json!({
            "format": "tbvol",
            "version": 2,
            "layout": "post_stack3_d",
            "tile_shape": [1, 1, 1],
            "volume": {
                "shape": [1, 1, 1],
                "processing_lineage": {
                    "schema_version": 2,
                    "artifact_role": artifact_role,
                    "parent_store": "C:\\cache-tests\\source.tbvol",
                    "parent_store_id": source_identity.store_id,
                    "pipeline": pipeline_spec,
                    "pipeline_identity": pipeline_identity,
                    "operator_set_identity": operator_set_identity,
                    "planner_profile_identity": planner_profile_identity,
                    "source_identity": source_identity,
                    "runtime_semantics_version": CURRENT_RUNTIME_SEMANTICS_VERSION,
                    "store_writer_semantics_version": CURRENT_STORE_WRITER_SEMANTICS_VERSION,
                    "runtime_version": "test-runtime",
                    "created_at_unix_s": 1,
                    "artifact_key": canonical_artifact.artifact_key,
                    "boundary_reason": match artifact_role {
                        ProcessingArtifactRole::Checkpoint => "authored_checkpoint",
                        ProcessingArtifactRole::FinalOutput => "final_output",
                    },
                    "logical_domain": canonical_artifact.logical_domain,
                    "chunk_grid_spec": canonical_artifact.chunk_grid_spec,
                    "geometry_fingerprints": canonical_artifact.geometry_fingerprints,
                }
            }
        });
        fs::write(
            store.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).expect("serialize test manifest"),
        )
        .expect("write manifest");
        artifact_cache_key
    }

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
        let pipeline = sample_trace_local_pipeline();
        let pipeline_hash = trace_local_pipeline_hash(&pipeline).expect("hash pipeline");
        let artifact_key =
            write_trace_local_manifest(&output, ProcessingArtifactRole::FinalOutput, &pipeline);

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        let _ = cache
            .register_visible_output_with_artifact_key(
                &artifact_key,
                "trace_local",
                &output.display().to_string(),
                &sample_source_fingerprint(),
                &pipeline_hash,
                &pipeline_hash,
                pipeline.operation_count(),
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("register visible output");

        let hit = cache
            .lookup_exact_visible_output(
                "trace_local",
                &sample_source_fingerprint(),
                &pipeline_hash,
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                CURRENT_STORE_WRITER_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("lookup visible output")
            .expect("expected exact hit");
        assert_eq!(hit.path, output.display().to_string());
    }

    #[test]
    fn canonical_artifact_key_lookup_prefers_registered_exact_output() {
        let root = temp_dir("canonical-exact");
        let output = root.join("derived.tbvol");
        fs::create_dir_all(&output).expect("create derived output");
        let pipeline = sample_trace_local_pipeline();
        let pipeline_hash = trace_local_pipeline_hash(&pipeline).expect("hash pipeline");
        let artifact_key =
            write_trace_local_manifest(&output, ProcessingArtifactRole::FinalOutput, &pipeline);

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        cache
            .register_visible_output_with_artifact_key(
                &artifact_key,
                "trace_local",
                &output.display().to_string(),
                &sample_source_fingerprint(),
                &pipeline_hash,
                &pipeline_hash,
                pipeline.operation_count(),
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("register canonical exact output");

        let hit = cache
            .lookup_exact_visible_output_by_artifact_key(
                &artifact_key,
                "trace_local",
                &pipeline_hash,
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                CURRENT_STORE_WRITER_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("lookup canonical exact output")
            .expect("cached output");
        assert_eq!(hit.artifact_key, artifact_key);
        assert_eq!(hit.path, output.display().to_string());
    }

    #[test]
    fn canonical_artifact_key_lookup_finds_registered_checkpoint() {
        let root = temp_dir("canonical-checkpoint");
        let output = root.join("checkpoint.tbvol");
        fs::create_dir_all(&output).expect("create checkpoint output");
        let pipeline = TraceLocalProcessingPipeline {
            schema_version: 2,
            revision: 1,
            preset_id: None,
            name: Some("cache-test-checkpoint".to_string()),
            description: None,
            steps: vec![TraceLocalProcessingStep {
                operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 1.25 },
                checkpoint: true,
            }],
        };
        let prefix_hash = trace_local_pipeline_hash(&pipeline).expect("hash pipeline");
        let artifact_key =
            write_trace_local_manifest(&output, ProcessingArtifactRole::Checkpoint, &pipeline);

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        cache
            .register_visible_checkpoint_with_artifact_key(
                &artifact_key,
                "trace_local",
                &output.display().to_string(),
                &sample_source_fingerprint(),
                &prefix_hash,
                pipeline.operation_count(),
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("register canonical checkpoint");

        let hit = cache
            .lookup_prefix_artifact_by_artifact_key(
                &artifact_key,
                "trace_local",
                &prefix_hash,
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                CURRENT_STORE_WRITER_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("lookup canonical checkpoint")
            .expect("cached checkpoint");
        assert_eq!(hit.artifact_key, artifact_key);
        assert_eq!(hit.path, output.display().to_string());
        assert_eq!(hit.prefix_len, pipeline.operation_count());
    }

    #[test]
    fn lookup_prefix_artifact_returns_visible_checkpoint() {
        let root = temp_dir("prefix");
        let checkpoint = root.join("checkpoint.tbvol");
        fs::create_dir_all(&checkpoint).expect("create checkpoint output");
        let pipeline = sample_trace_local_pipeline();
        let prefix_hash = trace_local_pipeline_hash(&pipeline).expect("hash prefix pipeline");
        let artifact_key =
            write_trace_local_manifest(&checkpoint, ProcessingArtifactRole::Checkpoint, &pipeline);

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        let _ = cache
            .register_visible_checkpoint_with_artifact_key(
                &artifact_key,
                "trace_local",
                &checkpoint.display().to_string(),
                &sample_source_fingerprint(),
                &prefix_hash,
                pipeline.operation_count(),
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("register visible checkpoint");

        let hit = cache
            .lookup_prefix_artifact(
                "trace_local",
                &sample_source_fingerprint(),
                &prefix_hash,
                pipeline.operation_count(),
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                CURRENT_STORE_WRITER_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("lookup prefix artifact")
            .expect("expected prefix hit");
        assert_eq!(hit.path, checkpoint.display().to_string());
        assert_eq!(hit.prefix_len, pipeline.operation_count());
    }

    #[test]
    fn upsert_artifact_refs_records_dataset_and_session_owners() {
        let root = temp_dir("refs");
        let output = root.join("derived.tbvol");
        fs::create_dir_all(&output).expect("create derived output");
        let pipeline = sample_trace_local_pipeline();
        let pipeline_hash = trace_local_pipeline_hash(&pipeline).expect("hash pipeline");
        let artifact_key =
            write_trace_local_manifest(&output, ProcessingArtifactRole::FinalOutput, &pipeline);

        let cache = ProcessingCacheState::initialize(
            &root.join("cache"),
            &root.join("cache").join("volumes"),
            &root.join("cache").join("index.sqlite"),
            &root.join("settings.json"),
        )
        .expect("initialize processing cache");

        cache
            .register_visible_output_with_artifact_key(
                &artifact_key,
                "trace_local",
                &output.display().to_string(),
                &sample_source_fingerprint(),
                &pipeline_hash,
                &pipeline_hash,
                pipeline.operation_count(),
                CURRENT_RUNTIME_SEMANTICS_VERSION,
                TEST_STORE_FORMAT_VERSION,
            )
            .expect("register visible output");
        cache
            .upsert_dataset_artifact_ref(&artifact_key, "dataset-1")
            .expect("pin dataset ref");
        cache
            .upsert_session_pipeline_artifact_ref(&artifact_key, "session-1")
            .expect("pin session ref");

        let connection = cache
            .connection
            .lock()
            .expect("processing cache connection mutex poisoned");
        let ref_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifact_refs WHERE artifact_key = ?1",
                params![artifact_key],
                |row| row.get(0),
            )
            .expect("count artifact refs");
        assert_eq!(ref_count, 2);
    }
}
