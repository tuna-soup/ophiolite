use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use seis_contracts_operations::datasets::{
    DatasetRegistryEntry, DatasetRegistryStatus, DatasetSummary, LoadWorkspaceStateResponse,
    RemoveDatasetEntryRequest, RemoveDatasetEntryResponse, SetActiveDatasetEntryRequest,
    SetActiveDatasetEntryResponse, UpsertDatasetEntryRequest, UpsertDatasetEntryResponse,
};
use seis_contracts_operations::resolve::IPC_SCHEMA_VERSION;
use seis_contracts_operations::workspace::{
    PostStackNeighborhoodProcessingPipeline, ProcessingPipelineFamily, SaveWorkspaceSessionRequest,
    SaveWorkspaceSessionResponse, SectionAxis, SubvolumeCropOperation, WorkspacePipelineEntry,
    WorkspaceSession,
};
use seis_runtime::TraceLocalProcessingPipeline;
use serde::{Deserialize, Serialize};

use crate::processing::unix_timestamp_s;

const WORKSPACE_SIGNED_DOCUMENT_FORMAT: &str = "traceboost-workspace-v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DatasetRegistryDocument {
    entries: Vec<DatasetRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedWorkspaceDocument<T> {
    format: String,
    checksum: String,
    payload: T,
}

pub struct WorkspaceState {
    registry_path: PathBuf,
    session_path: PathBuf,
    entries: Mutex<Vec<DatasetRegistryEntry>>,
    session: Mutex<WorkspaceSession>,
}

impl WorkspaceState {
    pub fn initialize(
        registry_path: impl AsRef<Path>,
        session_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let registry_path = registry_path.as_ref().to_path_buf();
        let session_path = session_path.as_ref().to_path_buf();
        if let Some(parent) = registry_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        if let Some(parent) = session_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let mut entries = load_registry(&registry_path)?;
        ensure_registry_has_unique_store_ids(&entries)?;
        entries.sort_by(|left, right| right.updated_at_unix_s.cmp(&left.updated_at_unix_s));
        let mut session = load_session(&session_path)?;
        session.active_store_path = None;
        session.project_root = None;
        session.native_engineering_accepted_store_paths.clear();
        persist_session(&session_path, &session)?;

        Ok(Self {
            registry_path,
            session_path,
            entries: Mutex::new(entries),
            session: Mutex::new(session),
        })
    }

    pub fn load_state(&self) -> Result<LoadWorkspaceStateResponse, String> {
        let entries = self.snapshot_entries()?;
        ensure_registry_has_unique_store_ids(&entries)?;
        let session = self.snapshot_session()?;
        Ok(LoadWorkspaceStateResponse {
            schema_version: IPC_SCHEMA_VERSION,
            entries,
            session,
        })
    }

    pub fn upsert_entry(
        &self,
        request: UpsertDatasetEntryRequest,
    ) -> Result<UpsertDatasetEntryResponse, String> {
        let now = unix_timestamp_s();
        let mut entries = self
            .entries
            .lock()
            .expect("workspace entries mutex poisoned");
        let mut session = self
            .session
            .lock()
            .expect("workspace session mutex poisoned");

        let match_index = if let Some(entry_id) = request.entry_id.as_ref() {
            entries.iter().position(|entry| &entry.entry_id == entry_id)
        } else {
            find_matching_entry(&entries, &request)
        };
        let existing_entry_id =
            match_index.and_then(|index| entries.get(index).map(|entry| entry.entry_id.as_str()));
        ensure_unique_store_identity(&entries, &request, existing_entry_id)?;
        let entry_count = entries.len();

        let entry = if let Some(index) = match_index {
            let entry = &mut entries[index];
            if let Some(display_name) = request
                .display_name
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                entry.display_name = display_name.trim().to_string();
            }
            if let Some(source_path) = request
                .source_path
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                entry.source_path = Some(source_path.trim().to_string());
            }
            if let Some(preferred_store_path) = request
                .preferred_store_path
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                entry.preferred_store_path = Some(preferred_store_path.trim().to_string());
            }
            if let Some(imported_store_path) = request
                .imported_store_path
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                entry.imported_store_path = Some(imported_store_path.trim().to_string());
                entry.last_imported_at_unix_s = Some(now);
            }
            if let Some(dataset) = request.dataset.as_ref() {
                entry.last_dataset = Some(dataset.clone());
                if entry.imported_store_path.is_none() {
                    entry.imported_store_path = Some(dataset.store_path.clone());
                }
                entry.last_imported_at_unix_s = Some(now);
            }
            if let Some(session_pipelines) = request.session_pipelines.as_ref() {
                entry.session_pipelines = session_pipelines.clone();
            }
            if request.active_session_pipeline_id.is_some() || request.session_pipelines.is_some() {
                entry.active_session_pipeline_id =
                    normalize_optional_string(request.active_session_pipeline_id.as_deref());
            }
            if entry.display_name.trim().is_empty() {
                entry.display_name = derive_display_name(
                    request.display_name.as_deref(),
                    entry.last_dataset.as_ref(),
                    entry.source_path.as_deref(),
                    entry
                        .imported_store_path
                        .as_deref()
                        .or(entry.preferred_store_path.as_deref()),
                    entry_count + 1,
                );
            }
            entry.updated_at_unix_s = now;
            apply_status(entry);
            entry.clone()
        } else {
            let display_name = derive_display_name(
                request.display_name.as_deref(),
                request.dataset.as_ref(),
                request.source_path.as_deref(),
                request
                    .imported_store_path
                    .as_deref()
                    .or(request.preferred_store_path.as_deref()),
                entry_count + 1,
            );
            let mut entry = DatasetRegistryEntry {
                entry_id: request
                    .entry_id
                    .clone()
                    .unwrap_or_else(|| format!("dataset-{now}-{:03}", entry_count + 1)),
                display_name,
                source_path: normalize_optional_path(request.source_path.as_deref()),
                preferred_store_path: normalize_optional_path(
                    request.preferred_store_path.as_deref(),
                ),
                imported_store_path: normalize_optional_path(
                    request.imported_store_path.as_deref(),
                ),
                last_dataset: request.dataset.clone(),
                session_pipelines: request.session_pipelines.clone().unwrap_or_default(),
                active_session_pipeline_id: normalize_optional_string(
                    request.active_session_pipeline_id.as_deref(),
                ),
                status: DatasetRegistryStatus::Linked,
                last_opened_at_unix_s: None,
                last_imported_at_unix_s: if request.dataset.is_some()
                    || request.imported_store_path.is_some()
                {
                    Some(now)
                } else {
                    None
                },
                updated_at_unix_s: now,
            };
            apply_status(&mut entry);
            entries.push(entry.clone());
            entry
        };

        if request.make_active {
            set_session_active_entry(&mut session, &entry);
        }

        sort_entries(&mut entries);
        persist_registry(&self.registry_path, &entries)?;
        persist_session(&self.session_path, &session)?;

        Ok(UpsertDatasetEntryResponse {
            schema_version: IPC_SCHEMA_VERSION,
            entry,
            session: session.clone(),
        })
    }

    pub fn remove_entry(
        &self,
        request: RemoveDatasetEntryRequest,
    ) -> Result<RemoveDatasetEntryResponse, String> {
        let mut entries = self
            .entries
            .lock()
            .expect("workspace entries mutex poisoned");
        let mut session = self
            .session
            .lock()
            .expect("workspace session mutex poisoned");
        let original_len = entries.len();
        entries.retain(|entry| entry.entry_id != request.entry_id);
        let deleted = entries.len() != original_len;

        if session.active_entry_id.as_deref() == Some(request.entry_id.as_str()) {
            session.active_entry_id = None;
            session.active_store_path = None;
        }

        persist_registry(&self.registry_path, &entries)?;
        persist_session(&self.session_path, &session)?;

        Ok(RemoveDatasetEntryResponse {
            schema_version: IPC_SCHEMA_VERSION,
            deleted,
            session: session.clone(),
        })
    }

    pub fn set_active_entry(
        &self,
        request: SetActiveDatasetEntryRequest,
    ) -> Result<SetActiveDatasetEntryResponse, String> {
        let mut entries = self
            .entries
            .lock()
            .expect("workspace entries mutex poisoned");
        let mut session = self
            .session
            .lock()
            .expect("workspace session mutex poisoned");

        let index = entries
            .iter()
            .position(|entry| entry.entry_id == request.entry_id)
            .ok_or_else(|| format!("Unknown dataset entry: {}", request.entry_id))?;

        let now = unix_timestamp_s();
        let entry = &mut entries[index];
        entry.last_opened_at_unix_s = Some(now);
        entry.updated_at_unix_s = now;
        apply_status(entry);
        let snapshot = entry.clone();
        set_session_active_entry(&mut session, &snapshot);

        sort_entries(&mut entries);
        persist_registry(&self.registry_path, &entries)?;
        persist_session(&self.session_path, &session)?;

        Ok(SetActiveDatasetEntryResponse {
            schema_version: IPC_SCHEMA_VERSION,
            entry: snapshot,
            session: session.clone(),
        })
    }

    pub fn save_session(
        &self,
        request: SaveWorkspaceSessionRequest,
    ) -> Result<SaveWorkspaceSessionResponse, String> {
        let now = unix_timestamp_s();
        let mut entries = self
            .entries
            .lock()
            .expect("workspace entries mutex poisoned");
        let mut session = self
            .session
            .lock()
            .expect("workspace session mutex poisoned");

        *session = WorkspaceSession {
            active_entry_id: request.active_entry_id.clone(),
            active_store_path: normalize_optional_path(request.active_store_path.as_deref()),
            active_axis: request.active_axis,
            active_index: request.active_index,
            selected_preset_id: normalize_optional_string(request.selected_preset_id.as_deref()),
            display_coordinate_reference_id: normalize_optional_string(
                request.display_coordinate_reference_id.as_deref(),
            ),
            active_velocity_model_asset_id: normalize_optional_string(
                request.active_velocity_model_asset_id.as_deref(),
            ),
            project_root: normalize_optional_path(request.project_root.as_deref()),
            project_survey_asset_id: normalize_optional_string(
                request.project_survey_asset_id.as_deref(),
            ),
            project_wellbore_id: normalize_optional_string(request.project_wellbore_id.as_deref()),
            project_section_tolerance_m: request
                .project_section_tolerance_m
                .filter(|value| value.is_finite() && *value > 0.0),
            selected_project_well_time_depth_model_asset_id: normalize_optional_string(
                request
                    .selected_project_well_time_depth_model_asset_id
                    .as_deref(),
            ),
            native_engineering_accepted_store_paths: normalize_string_list(
                &request.native_engineering_accepted_store_paths,
            ),
        };

        if let Some(active_entry_id) = session.active_entry_id.as_ref() {
            if let Some(entry) = entries
                .iter_mut()
                .find(|entry| &entry.entry_id == active_entry_id)
            {
                entry.last_opened_at_unix_s = Some(now);
                entry.updated_at_unix_s = now;
                if entry.imported_store_path.is_none() {
                    entry.imported_store_path = session.active_store_path.clone();
                }
                apply_status(entry);
            }
            sort_entries(&mut entries);
            persist_registry(&self.registry_path, &entries)?;
        }

        persist_session(&self.session_path, &session)?;

        Ok(SaveWorkspaceSessionResponse {
            schema_version: IPC_SCHEMA_VERSION,
            session: session.clone(),
        })
    }

    pub fn save_processing_session_pipelines(
        &self,
        entry_id: &str,
        session_pipelines: Vec<WorkspacePipelineEntry>,
        active_session_pipeline_id: Option<String>,
    ) -> Result<UpsertDatasetEntryResponse, String> {
        let now = unix_timestamp_s();
        let mut entries = self
            .entries
            .lock()
            .expect("workspace entries mutex poisoned");
        let session = self
            .session
            .lock()
            .expect("workspace session mutex poisoned");

        let index = entries
            .iter()
            .position(|entry| entry.entry_id == entry_id)
            .ok_or_else(|| format!("Unknown dataset entry: {entry_id}"))?;
        let entry = &mut entries[index];
        entry.session_pipelines = normalize_workspace_pipeline_entries(session_pipelines);
        entry.active_session_pipeline_id =
            normalize_active_pipeline_id(&entry.session_pipelines, active_session_pipeline_id);
        entry.updated_at_unix_s = now;
        apply_status(entry);
        let snapshot = entry.clone();

        sort_entries(&mut entries);
        persist_registry(&self.registry_path, &entries)?;

        Ok(UpsertDatasetEntryResponse {
            schema_version: IPC_SCHEMA_VERSION,
            entry: snapshot,
            session: session.clone(),
        })
    }

    fn snapshot_entries(&self) -> Result<Vec<DatasetRegistryEntry>, String> {
        let mut entries = self
            .entries
            .lock()
            .expect("workspace entries mutex poisoned")
            .clone();
        for entry in &mut entries {
            apply_status(entry);
        }
        sort_entries(&mut entries);
        Ok(entries)
    }

    fn snapshot_session(&self) -> Result<WorkspaceSession, String> {
        Ok(self
            .session
            .lock()
            .expect("workspace session mutex poisoned")
            .clone())
    }
}

fn find_matching_entry(
    entries: &[DatasetRegistryEntry],
    request: &UpsertDatasetEntryRequest,
) -> Option<usize> {
    let source_path = normalize_optional_path(request.source_path.as_deref());
    let imported_store_path = normalize_optional_path(request.imported_store_path.as_deref());

    entries.iter().position(|entry| {
        source_path
            .as_ref()
            .is_some_and(|value| entry.source_path.as_ref() == Some(value))
            || imported_store_path
                .as_ref()
                .is_some_and(|value| entry.imported_store_path.as_ref() == Some(value))
    })
}

fn entry_identity_store_id(entry: &DatasetRegistryEntry) -> Option<&str> {
    entry
        .last_dataset
        .as_ref()
        .map(|dataset| dataset.descriptor.store_id.trim())
        .filter(|store_id| !store_id.is_empty())
}

fn entry_identity_store_path(entry: &DatasetRegistryEntry) -> Option<String> {
    normalize_optional_path(
        entry
            .last_dataset
            .as_ref()
            .map(|dataset| dataset.store_path.as_str())
            .or(entry.imported_store_path.as_deref())
            .or(entry.preferred_store_path.as_deref()),
    )
}

fn ensure_unique_store_identity(
    entries: &[DatasetRegistryEntry],
    request: &UpsertDatasetEntryRequest,
    existing_entry_id: Option<&str>,
) -> Result<(), String> {
    let Some(dataset) = request.dataset.as_ref() else {
        return Ok(());
    };

    let candidate_store_id = dataset.descriptor.store_id.trim();
    if candidate_store_id.is_empty() {
        return Err("Dataset is missing a required store_id".to_string());
    }
    let candidate_store_path = normalize_optional_path(Some(dataset.store_path.as_str()))
        .unwrap_or_else(|| dataset.store_path.clone());

    for entry in entries {
        if existing_entry_id.is_some_and(|entry_id| entry.entry_id == entry_id) {
            continue;
        }
        let Some(existing_store_id) = entry_identity_store_id(entry) else {
            continue;
        };
        if existing_store_id != candidate_store_id {
            continue;
        }

        let existing_store_path =
            entry_identity_store_path(entry).unwrap_or_else(|| "<unknown store path>".to_string());
        if existing_store_path == candidate_store_path {
            continue;
        }

        return Err(format!(
            "Refusing to register duplicate store identity '{}' for '{}' because it is already used by '{}' at '{}'. This usually means a store folder was copied outside TraceBoost.",
            candidate_store_id, candidate_store_path, entry.display_name, existing_store_path
        ));
    }

    Ok(())
}

fn normalize_workspace_pipeline_entries(
    session_pipelines: Vec<WorkspacePipelineEntry>,
) -> Vec<WorkspacePipelineEntry> {
    session_pipelines
        .into_iter()
        .filter_map(normalize_workspace_pipeline_entry)
        .collect()
}

fn normalize_workspace_pipeline_entry(
    entry: WorkspacePipelineEntry,
) -> Option<WorkspacePipelineEntry> {
    let pipeline_id = entry.pipeline_id.trim().to_string();
    if pipeline_id.is_empty() {
        return None;
    }

    match entry.family {
        ProcessingPipelineFamily::PostStackNeighborhood => Some(WorkspacePipelineEntry {
            pipeline_id,
            family: ProcessingPipelineFamily::PostStackNeighborhood,
            pipeline: None,
            subvolume_crop: None,
            post_stack_neighborhood_pipeline: Some(normalize_post_stack_neighborhood_pipeline(
                entry.post_stack_neighborhood_pipeline,
            )),
            updated_at_unix_s: entry.updated_at_unix_s,
        }),
        _ => Some(WorkspacePipelineEntry {
            pipeline_id,
            family: ProcessingPipelineFamily::TraceLocal,
            pipeline: Some(normalize_trace_local_pipeline(entry.pipeline)),
            subvolume_crop: normalize_subvolume_crop(entry.subvolume_crop),
            post_stack_neighborhood_pipeline: None,
            updated_at_unix_s: entry.updated_at_unix_s,
        }),
    }
}

fn normalize_active_pipeline_id(
    session_pipelines: &[WorkspacePipelineEntry],
    active_session_pipeline_id: Option<String>,
) -> Option<String> {
    let requested = normalize_optional_string(active_session_pipeline_id.as_deref());
    if let Some(requested) = requested {
        if session_pipelines
            .iter()
            .any(|entry| entry.pipeline_id == requested)
        {
            return Some(requested);
        }
    }

    session_pipelines
        .first()
        .map(|entry| entry.pipeline_id.clone())
}

fn normalize_trace_local_pipeline(
    pipeline: Option<TraceLocalProcessingPipeline>,
) -> TraceLocalProcessingPipeline {
    let mut pipeline = pipeline.unwrap_or_else(empty_trace_local_pipeline);
    pipeline.name = normalize_optional_string(pipeline.name.as_deref());
    pipeline.description = normalize_optional_string(pipeline.description.as_deref());
    pipeline
}

fn normalize_post_stack_neighborhood_pipeline(
    pipeline: Option<PostStackNeighborhoodProcessingPipeline>,
) -> PostStackNeighborhoodProcessingPipeline {
    let mut pipeline = pipeline.unwrap_or_else(empty_post_stack_neighborhood_pipeline);
    pipeline.name = normalize_optional_string(pipeline.name.as_deref());
    pipeline.description = normalize_optional_string(pipeline.description.as_deref());
    pipeline
}

fn normalize_subvolume_crop(
    crop: Option<SubvolumeCropOperation>,
) -> Option<SubvolumeCropOperation> {
    crop.map(|mut crop| {
        if crop.inline_min > crop.inline_max {
            std::mem::swap(&mut crop.inline_min, &mut crop.inline_max);
        }
        if crop.xline_min > crop.xline_max {
            std::mem::swap(&mut crop.xline_min, &mut crop.xline_max);
        }
        if crop.z_min_ms > crop.z_max_ms {
            std::mem::swap(&mut crop.z_min_ms, &mut crop.z_max_ms);
        }
        crop
    })
}

fn empty_trace_local_pipeline() -> TraceLocalProcessingPipeline {
    TraceLocalProcessingPipeline {
        schema_version: 2,
        revision: 1,
        preset_id: None,
        name: None,
        description: None,
        steps: Vec::new(),
    }
}

fn empty_post_stack_neighborhood_pipeline() -> PostStackNeighborhoodProcessingPipeline {
    PostStackNeighborhoodProcessingPipeline {
        schema_version: 2,
        revision: 1,
        preset_id: None,
        name: None,
        description: None,
        trace_local_pipeline: None,
        operations: Vec::new(),
    }
}

fn ensure_registry_has_unique_store_ids(entries: &[DatasetRegistryEntry]) -> Result<(), String> {
    for (index, entry) in entries.iter().enumerate() {
        let Some(store_id) = entry_identity_store_id(entry) else {
            continue;
        };
        let store_path =
            entry_identity_store_path(entry).unwrap_or_else(|| "<unknown store path>".to_string());
        for other in entries.iter().skip(index + 1) {
            let Some(other_store_id) = entry_identity_store_id(other) else {
                continue;
            };
            if other_store_id != store_id {
                continue;
            }
            let other_store_path = entry_identity_store_path(other)
                .unwrap_or_else(|| "<unknown store path>".to_string());
            if other_store_path == store_path {
                continue;
            }
            return Err(format!(
                "Workspace contains duplicate store identity '{}' at '{}' and '{}'. This usually means a store folder was copied outside TraceBoost.",
                store_id, store_path, other_store_path
            ));
        }
    }
    Ok(())
}

fn load_registry(path: &Path) -> Result<Vec<DatasetRegistryEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if let Ok(signed) =
        serde_json::from_slice::<SignedWorkspaceDocument<DatasetRegistryDocument>>(&bytes)
    {
        verify_signed_document(path, &signed)?;
        return Ok(signed.payload.entries);
    }
    serde_json::from_slice::<DatasetRegistryDocument>(&bytes)
        .map(|document| document.entries)
        .map_err(|error| {
            format!(
                "Workspace registry '{}' is invalid: {error}",
                path.display()
            )
        })
}

fn load_session(path: &Path) -> Result<WorkspaceSession, String> {
    if !path.exists() {
        return Ok(default_session());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if let Ok(signed) = serde_json::from_slice::<SignedWorkspaceDocument<WorkspaceSession>>(&bytes)
    {
        verify_signed_document(path, &signed)?;
        return Ok(signed.payload);
    }
    serde_json::from_slice::<WorkspaceSession>(&bytes)
        .map_err(|error| format!("Workspace session '{}' is invalid: {error}", path.display()))
}

fn persist_registry(path: &Path, entries: &[DatasetRegistryEntry]) -> Result<(), String> {
    let document = DatasetRegistryDocument {
        entries: entries.to_vec(),
    };
    let json = serialize_signed_document(&document)?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn persist_session(path: &Path, session: &WorkspaceSession) -> Result<(), String> {
    let json = serialize_signed_document(session)?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn serialize_signed_document<T>(payload: &T) -> Result<Vec<u8>, String>
where
    T: Serialize + Clone,
{
    let payload_value = serde_json::to_value(payload).map_err(|error| error.to_string())?;
    let payload_bytes = serde_json::to_vec(&payload_value).map_err(|error| error.to_string())?;
    let signed = SignedWorkspaceDocument {
        format: WORKSPACE_SIGNED_DOCUMENT_FORMAT.to_string(),
        checksum: blake3::hash(&payload_bytes).to_hex().to_string(),
        payload: payload.clone(),
    };
    serde_json::to_vec_pretty(&signed).map_err(|error| error.to_string())
}

fn verify_signed_document<T>(path: &Path, signed: &SignedWorkspaceDocument<T>) -> Result<(), String>
where
    T: Serialize,
{
    if signed.format != WORKSPACE_SIGNED_DOCUMENT_FORMAT {
        return Err(format!(
            "Workspace state '{}' uses unsupported format '{}'.",
            path.display(),
            signed.format
        ));
    }
    let payload_bytes = serde_json::to_vec(&signed.payload).map_err(|error| error.to_string())?;
    let expected = blake3::hash(&payload_bytes).to_hex().to_string();
    if expected != signed.checksum {
        return Err(format!(
            "Workspace state '{}' failed integrity verification.",
            path.display()
        ));
    }
    Ok(())
}

fn default_session() -> WorkspaceSession {
    WorkspaceSession {
        active_entry_id: None,
        active_store_path: None,
        active_axis: SectionAxis::Inline,
        active_index: 0,
        selected_preset_id: None,
        display_coordinate_reference_id: None,
        active_velocity_model_asset_id: None,
        project_root: None,
        project_survey_asset_id: None,
        project_wellbore_id: None,
        project_section_tolerance_m: None,
        selected_project_well_time_depth_model_asset_id: None,
        native_engineering_accepted_store_paths: Vec::new(),
    }
}

fn set_session_active_entry(session: &mut WorkspaceSession, entry: &DatasetRegistryEntry) {
    session.active_entry_id = Some(entry.entry_id.clone());
    session.active_store_path = entry
        .imported_store_path
        .clone()
        .or_else(|| entry.preferred_store_path.clone());
}

fn derive_display_name(
    explicit_name: Option<&str>,
    dataset: Option<&DatasetSummary>,
    source_path: Option<&str>,
    store_path: Option<&str>,
    sequence: usize,
) -> String {
    explicit_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| dataset.map(|dataset| dataset.descriptor.label.clone()))
        .or_else(|| source_path.and_then(path_basename))
        .or_else(|| store_path.and_then(path_basename))
        .unwrap_or_else(|| format!("Dataset {sequence}"))
}

fn path_basename(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_optional_path(value: Option<&str>) -> Option<String> {
    normalize_optional_string(value)
}

fn normalize_string_list(values: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        if let Some(value) = normalize_optional_string(Some(value.as_str())) {
            if !normalized.iter().any(|existing| existing == &value) {
                normalized.push(value);
            }
        }
    }
    normalized
}

fn apply_status(entry: &mut DatasetRegistryEntry) {
    entry.status = resolve_status(entry);
}

fn resolve_status(entry: &DatasetRegistryEntry) -> DatasetRegistryStatus {
    if entry
        .source_path
        .as_deref()
        .is_some_and(|value| !Path::new(value).exists())
    {
        return DatasetRegistryStatus::MissingSource;
    }

    if entry
        .imported_store_path
        .as_deref()
        .is_some_and(|value| !Path::new(value).exists())
    {
        return DatasetRegistryStatus::MissingStore;
    }

    if entry.imported_store_path.is_some() {
        return DatasetRegistryStatus::Imported;
    }

    DatasetRegistryStatus::Linked
}

fn sort_entries(entries: &mut [DatasetRegistryEntry]) {
    entries.sort_by(|left, right| right.updated_at_unix_s.cmp(&left.updated_at_unix_s));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("traceboost-workspace-test-{unique}"));
        fs::create_dir_all(&base).expect("create temp workspace dir");
        base.join(name)
    }

    fn sample_dataset_summary(store_path: &str, store_id: &str, label: &str) -> DatasetSummary {
        serde_json::from_value(serde_json::json!({
            "store_path": store_path,
            "descriptor": {
                "id": "dataset-id",
                "store_id": store_id,
                "label": label,
                "shape": [4, 4, 4],
                "chunk_shape": [4, 4, 4],
                "sample_interval_ms": 2.0,
                "sample_data_fidelity": {
                    "source_sample_type": "f32",
                    "working_sample_type": "f32",
                    "conversion": "identity",
                    "preservation": "exact",
                    "notes": []
                },
                "geometry": {
                    "compare_family": "seismic-grid:v1",
                    "fingerprint": "geom:test",
                    "summary": {
                        "inline_axis": { "count": 4, "first": 100, "last": 103, "step": 1, "regular": true },
                        "xline_axis": { "count": 4, "first": 200, "last": 203, "step": 1, "regular": true },
                        "sample_axis": { "count": 4, "first": 0.0, "last": 6.0, "step": 2.0, "regular": true, "units": "ms" },
                        "layout": null,
                        "gather_axis_kind": null,
                        "gather_axis": null,
                        "provenance": "source"
                    }
                },
                "coordinate_reference_binding": null,
                "spatial": null,
                "processing_lineage_summary": null
            }
        }))
        .expect("deserialize test dataset summary")
    }

    #[test]
    fn upsert_and_restore_workspace_state() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        let response = state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: None,
                display_name: Some("Demo survey".to_string()),
                source_path: Some("C:/data/demo.segy".to_string()),
                preferred_store_path: Some("C:/data/demo.tbvol".to_string()),
                imported_store_path: None,
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("upsert entry");
        assert_eq!(response.entry.display_name, "Demo survey");

        state
            .save_session(SaveWorkspaceSessionRequest {
                schema_version: IPC_SCHEMA_VERSION,
                active_entry_id: Some(response.entry.entry_id.clone()),
                active_store_path: Some("C:/data/demo.tbvol".to_string()),
                active_axis: SectionAxis::Xline,
                active_index: 17,
                selected_preset_id: Some("demo-preset".to_string()),
                display_coordinate_reference_id: Some("EPSG:23031".to_string()),
                active_velocity_model_asset_id: Some("velocity-asset-1".to_string()),
                project_root: Some("C:/data/project-root".to_string()),
                project_survey_asset_id: Some("survey-asset-1".to_string()),
                project_wellbore_id: Some("wellbore-1".to_string()),
                project_section_tolerance_m: Some(18.5),
                selected_project_well_time_depth_model_asset_id: Some("well-model-1".to_string()),
                native_engineering_accepted_store_paths: vec![
                    "C:/data/demo.tbvol".to_string(),
                    "C:/data/demo-secondary.tbvol".to_string(),
                ],
            })
            .expect("save session");

        let restored = WorkspaceState::initialize(&registry, &session)
            .expect("reinitialize workspace state")
            .load_state()
            .expect("load state");
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(
            restored.session.active_entry_id,
            Some(response.entry.entry_id)
        );
        assert_eq!(restored.session.active_index, 17);
        assert_eq!(
            restored.session.display_coordinate_reference_id.as_deref(),
            Some("EPSG:23031")
        );
        assert_eq!(
            restored.session.active_velocity_model_asset_id.as_deref(),
            Some("velocity-asset-1")
        );
        assert_eq!(restored.session.active_store_path, None);
        assert_eq!(restored.session.project_root, None);
        assert_eq!(
            restored.session.project_survey_asset_id.as_deref(),
            Some("survey-asset-1")
        );
        assert_eq!(
            restored.session.project_wellbore_id.as_deref(),
            Some("wellbore-1")
        );
        assert_eq!(restored.session.project_section_tolerance_m, Some(18.5));
        assert_eq!(
            restored
                .session
                .selected_project_well_time_depth_model_asset_id
                .as_deref(),
            Some("well-model-1")
        );
        assert_eq!(
            restored.session.native_engineering_accepted_store_paths,
            Vec::<String>::new()
        );
    }

    #[test]
    fn preferred_store_path_does_not_merge_distinct_sources() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: None,
                display_name: Some("First survey".to_string()),
                source_path: Some("C:/data/first.segy".to_string()),
                preferred_store_path: Some("C:/data/shared.tbvol".to_string()),
                imported_store_path: None,
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert first entry");

        state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: None,
                display_name: Some("Second survey".to_string()),
                source_path: Some("C:/data/second.segy".to_string()),
                preferred_store_path: Some("C:/data/shared.tbvol".to_string()),
                imported_store_path: None,
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert second entry");

        let restored = state.load_state().expect("load state");
        assert_eq!(restored.entries.len(), 2);
    }

    #[test]
    fn explicit_entry_id_allows_duplicate_entry_for_same_store() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-a".to_string()),
                display_name: Some("Demo".to_string()),
                source_path: Some("C:/data/demo.segy".to_string()),
                preferred_store_path: Some("C:/data/demo.tbvol".to_string()),
                imported_store_path: Some("C:/data/demo.tbvol".to_string()),
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert first entry");

        let response = state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-b".to_string()),
                display_name: Some("Demo_1".to_string()),
                source_path: Some("C:/data/demo.segy".to_string()),
                preferred_store_path: Some("C:/data/demo.tbvol".to_string()),
                imported_store_path: Some("C:/data/demo.tbvol".to_string()),
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert second entry");

        let restored = state.load_state().expect("load state");
        assert_eq!(restored.entries.len(), 2);
        assert_eq!(response.entry.entry_id, "dataset-b");
    }

    #[test]
    fn initialize_rejects_legacy_registry_without_geometry() {
        let registry = temp_file("legacy-registry.json");
        let session = temp_file("legacy-session.json");

        let legacy = serde_json::json!({
            "entries": [
                {
                    "entry_id": "dataset-legacy-001",
                    "display_name": "Legacy Dataset",
                    "source_path": null,
                    "preferred_store_path": "C:/missing/legacy.tbvol",
                    "imported_store_path": null,
                    "last_dataset": {
                        "store_path": "C:/missing/legacy.tbvol",
                        "descriptor": {
                            "id": "legacy",
                            "label": "Legacy Dataset",
                            "shape": [4, 4, 4],
                            "chunk_shape": [4, 4, 4],
                            "sample_interval_ms": 2.0
                        }
                    },
                    "session_pipelines": [],
                    "active_session_pipeline_id": null,
                    "status": "linked",
                    "last_opened_at_unix_s": null,
                    "last_imported_at_unix_s": null,
                    "updated_at_unix_s": 1
                }
            ]
        });

        fs::write(
            &registry,
            serde_json::to_vec_pretty(&legacy).expect("serialize legacy registry"),
        )
        .expect("write legacy registry");

        let error = WorkspaceState::initialize(&registry, &session)
            .err()
            .expect("legacy invalid registry should be rejected");
        assert!(error.contains("Workspace registry"));
        assert!(error.contains("missing field `store_id`"));
    }

    #[test]
    fn rejects_duplicate_store_id_with_different_store_path() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-a".to_string()),
                display_name: Some("Original".to_string()),
                source_path: None,
                preferred_store_path: Some("C:/data/original.tbvol".to_string()),
                imported_store_path: Some("C:/data/original.tbvol".to_string()),
                dataset: Some(sample_dataset_summary(
                    "C:/data/original.tbvol",
                    "store-123",
                    "Original",
                )),
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: false,
            })
            .expect("insert original entry");

        let error = state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-b".to_string()),
                display_name: Some("Copied".to_string()),
                source_path: None,
                preferred_store_path: Some("C:/data/copied.tbvol".to_string()),
                imported_store_path: Some("C:/data/copied.tbvol".to_string()),
                dataset: Some(sample_dataset_summary(
                    "C:/data/copied.tbvol",
                    "store-123",
                    "Copied",
                )),
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: false,
            })
            .expect_err("reject duplicate store id");

        assert!(error.contains("duplicate store identity"));
        assert!(error.contains("store-123"));
    }

    #[test]
    fn initialize_rejects_tampered_signed_session_state() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        state
            .save_session(SaveWorkspaceSessionRequest {
                schema_version: IPC_SCHEMA_VERSION,
                active_entry_id: Some("dataset-1".to_string()),
                active_store_path: Some("C:/data/demo.tbvol".to_string()),
                active_axis: SectionAxis::Inline,
                active_index: 12,
                selected_preset_id: None,
                display_coordinate_reference_id: None,
                active_velocity_model_asset_id: None,
                project_root: Some("C:/data/project-root".to_string()),
                project_survey_asset_id: None,
                project_wellbore_id: None,
                project_section_tolerance_m: None,
                selected_project_well_time_depth_model_asset_id: None,
                native_engineering_accepted_store_paths: vec!["C:/data/demo.tbvol".to_string()],
            })
            .expect("save signed session");

        let mut tampered: serde_json::Value =
            serde_json::from_slice(&fs::read(&session).expect("read signed session"))
                .expect("parse signed session");
        tampered["payload"]["active_index"] = serde_json::json!(99);
        fs::write(
            &session,
            serde_json::to_vec_pretty(&tampered).expect("serialize tampered session"),
        )
        .expect("write tampered session");

        let error = WorkspaceState::initialize(&registry, &session)
            .err()
            .expect("tampered session should be rejected");
        assert!(error.contains("failed integrity verification"));
    }

    #[test]
    fn initialize_rejects_tampered_signed_registry() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-a".to_string()),
                display_name: Some("Original".to_string()),
                source_path: Some("C:/data/original.segy".to_string()),
                preferred_store_path: Some("C:/data/original.tbvol".to_string()),
                imported_store_path: Some("C:/data/original.tbvol".to_string()),
                dataset: Some(sample_dataset_summary(
                    "C:/data/original.tbvol",
                    "store-123",
                    "Original",
                )),
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: false,
            })
            .expect("insert original entry");

        let mut tampered: serde_json::Value =
            serde_json::from_slice(&fs::read(&registry).expect("read signed registry"))
                .expect("parse signed registry");
        tampered["payload"]["entries"][0]["display_name"] = serde_json::json!("Tampered");
        fs::write(
            &registry,
            serde_json::to_vec_pretty(&tampered).expect("serialize tampered registry"),
        )
        .expect("write tampered registry");

        let error = WorkspaceState::initialize(&registry, &session)
            .err()
            .expect("tampered registry should be rejected");
        assert!(error.contains("failed integrity verification"));
    }

    #[test]
    fn save_processing_session_pipelines_normalizes_active_entry_payload() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        let response = state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-a".to_string()),
                display_name: Some("Demo".to_string()),
                source_path: Some("C:/data/demo.segy".to_string()),
                preferred_store_path: Some("C:/data/demo.tbvol".to_string()),
                imported_store_path: Some("C:/data/demo.tbvol".to_string()),
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert entry");

        let saved = state
            .save_processing_session_pipelines(
                &response.entry.entry_id,
                vec![
                    WorkspacePipelineEntry {
                        pipeline_id: "   ".to_string(),
                        family: ProcessingPipelineFamily::TraceLocal,
                        pipeline: Some(empty_trace_local_pipeline()),
                        subvolume_crop: None,
                        post_stack_neighborhood_pipeline: None,
                        updated_at_unix_s: 10,
                    },
                    WorkspacePipelineEntry {
                        pipeline_id: " pipeline-1 ".to_string(),
                        family: ProcessingPipelineFamily::Subvolume,
                        pipeline: None,
                        subvolume_crop: Some(SubvolumeCropOperation {
                            inline_min: 40,
                            inline_max: 20,
                            xline_min: 90,
                            xline_max: 70,
                            z_min_ms: 120.0,
                            z_max_ms: 80.0,
                        }),
                        post_stack_neighborhood_pipeline: Some(
                            empty_post_stack_neighborhood_pipeline(),
                        ),
                        updated_at_unix_s: 20,
                    },
                ],
                Some("missing".to_string()),
            )
            .expect("save processing pipelines");

        assert_eq!(saved.entry.session_pipelines.len(), 1);
        let pipeline = &saved.entry.session_pipelines[0];
        assert_eq!(pipeline.pipeline_id, "pipeline-1");
        assert_eq!(pipeline.family, ProcessingPipelineFamily::TraceLocal);
        assert!(pipeline.pipeline.is_some());
        assert!(pipeline.post_stack_neighborhood_pipeline.is_none());
        let crop = pipeline
            .subvolume_crop
            .as_ref()
            .expect("subvolume crop preserved");
        assert_eq!((crop.inline_min, crop.inline_max), (20, 40));
        assert_eq!((crop.xline_min, crop.xline_max), (70, 90));
        assert_eq!((crop.z_min_ms, crop.z_max_ms), (80.0, 120.0));
        assert_eq!(
            saved.entry.active_session_pipeline_id.as_deref(),
            Some("pipeline-1")
        );
    }

    #[test]
    fn save_processing_session_pipelines_normalizes_neighborhood_entries() {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        let state =
            WorkspaceState::initialize(&registry, &session).expect("initialize workspace state");

        let response = state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some("dataset-b".to_string()),
                display_name: Some("Neighborhood".to_string()),
                source_path: Some("C:/data/neighborhood.segy".to_string()),
                preferred_store_path: Some("C:/data/neighborhood.tbvol".to_string()),
                imported_store_path: Some("C:/data/neighborhood.tbvol".to_string()),
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert entry");

        let saved = state
            .save_processing_session_pipelines(
                &response.entry.entry_id,
                vec![WorkspacePipelineEntry {
                    pipeline_id: "neighborhood-1".to_string(),
                    family: ProcessingPipelineFamily::PostStackNeighborhood,
                    pipeline: Some(empty_trace_local_pipeline()),
                    subvolume_crop: Some(SubvolumeCropOperation {
                        inline_min: 1,
                        inline_max: 2,
                        xline_min: 3,
                        xline_max: 4,
                        z_min_ms: 5.0,
                        z_max_ms: 6.0,
                    }),
                    post_stack_neighborhood_pipeline: None,
                    updated_at_unix_s: 30,
                }],
                Some("neighborhood-1".to_string()),
            )
            .expect("save neighborhood pipelines");

        let pipeline = &saved.entry.session_pipelines[0];
        assert_eq!(
            pipeline.family,
            ProcessingPipelineFamily::PostStackNeighborhood
        );
        assert!(pipeline.pipeline.is_none());
        assert!(pipeline.subvolume_crop.is_none());
        assert!(pipeline.post_stack_neighborhood_pipeline.is_some());
    }
}
