//! App-local processing authoring adapter for TraceBoost desktop.
//!
//! This module is intentionally not part of the public Ophiolite SDK surface.
//! It owns desktop-facing authoring glue until a second real consumer justifies
//! extraction into a shared crate.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use ophiolite::{
    IPC_SCHEMA_VERSION, OpenDatasetRequest, OperatorAvailability, OperatorCatalogEntry,
    OperatorFamily, OperatorParameterDoc,
};
use seis_contracts_operations::SubvolumeCropOperation;
use seis_contracts_operations::datasets::{DatasetRegistryEntry, DatasetSummary};
use seis_contracts_operations::workspace::{WorkspacePipelineEntry, WorkspaceSession};
use seis_runtime::{
    FrequencyPhaseMode, FrequencyWindowShape, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, PostStackNeighborhoodWindow, ProcessingPipelineFamily,
    ProcessingPipelineSpec, SubvolumeProcessingPipeline, TraceLocalProcessingOperation,
    TraceLocalProcessingPipeline, TraceLocalVolumeArithmeticOperator,
};
use serde::{Deserialize, Serialize};
use traceboost_app::{dataset_operator_catalog, open_dataset_summary};

use crate::app_paths::AppPaths;
use crate::processing::unix_timestamp_s;
use crate::workspace::WorkspaceState;
use crate::{
    default_post_stack_neighborhood_processing_store_path, default_processing_store_path,
    default_subvolume_processing_store_path,
};

const DEFAULT_SAMPLE_INTERVAL_MS: f32 = 2.0;
const PALETTE_PROVIDER_FALLBACK: &str = "traceboost-demo";
const PALETTE_SOURCE_CANONICAL: &str = "canonical";
const PALETTE_SOURCE_FALLBACK: &str = "fallback";

pub type PersistProcessingSessionPipelinesRequest = SaveProcessingAuthoringSessionRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveProcessingAuthoringPaletteRequest {
    pub schema_version: u32,
    pub family: ProcessingPipelineFamily,
    #[serde(default)]
    pub store_path: Option<String>,
    #[serde(default)]
    pub secondary_store_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveProcessingAuthoringPaletteResponse {
    pub schema_version: u32,
    pub family: ProcessingPipelineFamily,
    pub items: Vec<ProcessingAuthoringPaletteItem>,
    pub source_label: String,
    pub source_detail: String,
    pub empty_message: String,
    #[serde(default)]
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessingAuthoringPaletteItem {
    pub item_id: String,
    pub label: String,
    pub description: String,
    pub short_help: String,
    #[serde(default)]
    pub help_markdown: Option<String>,
    #[serde(default)]
    pub help_url: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
    pub canonical_id: String,
    pub canonical_name: String,
    pub group: String,
    pub group_id: String,
    pub provider: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameter_docs: Vec<OperatorParameterDoc>,
    #[serde(default)]
    pub alias_label: Option<String>,
    pub source: String,
    pub insertable: ProcessingAuthoringInsertable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProcessingAuthoringInsertable {
    TraceLocalOperation {
        operation: TraceLocalProcessingOperation,
    },
    SubvolumeCrop {
        crop: SubvolumeCropOperation,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProcessingAuthoringSessionRequest {
    pub schema_version: u32,
    pub entry_id: String,
    #[serde(default)]
    pub session_pipelines: Vec<WorkspacePipelineEntry>,
    #[serde(default)]
    pub active_session_pipeline_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingAuthoringSessionResponse {
    pub schema_version: u32,
    pub entry: DatasetRegistryEntry,
    pub session: WorkspaceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyProcessingAuthoringSessionActionRequest {
    pub schema_version: u32,
    pub entry_id: String,
    #[serde(flatten)]
    pub action: ProcessingAuthoringSessionAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ProcessingAuthoringSessionAction {
    EnsureFamilyPipeline {
        family: ProcessingPipelineFamily,
    },
    CreatePipeline {
        family: ProcessingPipelineFamily,
    },
    DuplicatePipeline {
        #[serde(default)]
        pipeline_id: Option<String>,
    },
    ActivatePipeline {
        pipeline_id: String,
    },
    RemovePipeline {
        pipeline_id: String,
    },
    ReplaceActiveFromPipelineSpec {
        pipeline: ProcessingPipelineSpec,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveProcessingRunOutputRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub family: ProcessingPipelineFamily,
    #[serde(default)]
    pub pipeline: Option<TraceLocalProcessingPipeline>,
    #[serde(default)]
    pub subvolume_crop: Option<SubvolumeCropOperation>,
    #[serde(default)]
    pub post_stack_neighborhood_pipeline: Option<PostStackNeighborhoodProcessingPipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveProcessingRunOutputResponse {
    pub schema_version: u32,
    pub output_store_path: String,
}

#[derive(Debug, Clone)]
struct PaletteDefinition {
    item_id: &'static str,
    canonical_id: &'static str,
    canonical_family: OperatorFamily,
    fallback_label: &'static str,
    alias_label: Option<&'static str>,
    local_keywords: &'static [&'static str],
    shortcut: Option<&'static str>,
}

const PALETTE_DEFINITIONS: &[PaletteDefinition] = &[
    PaletteDefinition {
        item_id: "amplitude_scalar",
        canonical_id: "amplitude_scalar",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Amplitude Scalar",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("a"),
    },
    PaletteDefinition {
        item_id: "trace_rms_normalize",
        canonical_id: "trace_rms_normalize",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Trace RMS Normalize",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("n"),
    },
    PaletteDefinition {
        item_id: "agc_rms",
        canonical_id: "agc_rms",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "RMS AGC",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("g"),
    },
    PaletteDefinition {
        item_id: "phase_rotation",
        canonical_id: "phase_rotation",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Phase Rotation",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("h"),
    },
    PaletteDefinition {
        item_id: "envelope",
        canonical_id: "envelope",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Envelope",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("e"),
    },
    PaletteDefinition {
        item_id: "instantaneous_phase",
        canonical_id: "instantaneous_phase",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Instantaneous Phase",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("p"),
    },
    PaletteDefinition {
        item_id: "instantaneous_frequency",
        canonical_id: "instantaneous_frequency",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Instantaneous Frequency",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("f"),
    },
    PaletteDefinition {
        item_id: "sweetness",
        canonical_id: "sweetness",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Sweetness",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("s"),
    },
    PaletteDefinition {
        item_id: "volume_subtract",
        canonical_id: "volume_arithmetic",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Subtract Volume",
        alias_label: Some("Subtract Volume"),
        local_keywords: &["subtract", "difference", "minus"],
        shortcut: Some("v"),
    },
    PaletteDefinition {
        item_id: "volume_add",
        canonical_id: "volume_arithmetic",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Add Volume",
        alias_label: Some("Add Volume"),
        local_keywords: &["add", "sum", "plus"],
        shortcut: None,
    },
    PaletteDefinition {
        item_id: "volume_multiply",
        canonical_id: "volume_arithmetic",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Multiply Volumes",
        alias_label: Some("Multiply Volumes"),
        local_keywords: &["multiply", "product", "times"],
        shortcut: None,
    },
    PaletteDefinition {
        item_id: "volume_divide",
        canonical_id: "volume_arithmetic",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Divide Volumes",
        alias_label: Some("Divide Volumes"),
        local_keywords: &["divide", "ratio", "quotient"],
        shortcut: None,
    },
    PaletteDefinition {
        item_id: "crop_subvolume",
        canonical_id: "crop",
        canonical_family: OperatorFamily::SubvolumeProcessing,
        fallback_label: "Crop Subvolume",
        alias_label: Some("Crop Subvolume"),
        local_keywords: &["crop", "subvolume", "subset", "window"],
        shortcut: Some("c"),
    },
    PaletteDefinition {
        item_id: "lowpass_filter",
        canonical_id: "lowpass_filter",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Lowpass Filter",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("l"),
    },
    PaletteDefinition {
        item_id: "highpass_filter",
        canonical_id: "highpass_filter",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Highpass Filter",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("i"),
    },
    PaletteDefinition {
        item_id: "bandpass_filter",
        canonical_id: "bandpass_filter",
        canonical_family: OperatorFamily::TraceLocalProcessing,
        fallback_label: "Bandpass Filter",
        alias_label: None,
        local_keywords: &[],
        shortcut: Some("b"),
    },
];

pub fn persist_processing_session_pipelines(
    workspace: &WorkspaceState,
    request: PersistProcessingSessionPipelinesRequest,
) -> Result<seis_contracts_operations::datasets::UpsertDatasetEntryResponse, String> {
    validate_schema(request.schema_version)?;
    workspace.save_processing_session_pipelines(
        &request.entry_id,
        request.session_pipelines,
        request.active_session_pipeline_id,
    )
}

pub fn resolve_processing_authoring_palette(
    request: ResolveProcessingAuthoringPaletteRequest,
) -> Result<ResolveProcessingAuthoringPaletteResponse, String> {
    validate_schema(request.schema_version)?;

    let store_path = normalize_optional_string(request.store_path.as_deref());
    let secondary_store_paths = normalize_string_list(&request.secondary_store_paths);
    let mut sample_interval_ms = DEFAULT_SAMPLE_INTERVAL_MS;
    let mut default_crop = empty_subvolume_crop();
    let mut fallback_reasons = Vec::new();

    if let Some(store_path) = store_path.as_ref() {
        match open_dataset_summary(OpenDatasetRequest {
            schema_version: IPC_SCHEMA_VERSION,
            store_path: store_path.clone(),
        }) {
            Ok(response) => {
                sample_interval_ms = response.dataset.descriptor.sample_interval_ms;
                default_crop = default_subvolume_crop_from_dataset(&response.dataset);
            }
            Err(error) => fallback_reasons.push(format!("Failed to load dataset summary: {error}")),
        }
    }

    let catalog = match store_path.as_ref() {
        Some(store_path) => match dataset_operator_catalog(store_path) {
            Ok(catalog) => Some(catalog),
            Err(error) => {
                fallback_reasons.push(format!("Failed to load dataset operator catalog: {error}"));
                None
            }
        },
        None => None,
    };

    let items = PALETTE_DEFINITIONS
        .iter()
        .filter(|definition| definition_supports_family(definition, request.family))
        .map(|definition| {
            let catalog_entry = catalog.as_ref().and_then(|catalog| {
                catalog.operators.iter().find(|entry| {
                    entry.family == definition.canonical_family
                        && entry.id == definition.canonical_id
                        && matches!(entry.availability, OperatorAvailability::Available)
                })
            });
            palette_item_from_definition(
                definition,
                catalog_entry,
                sample_interval_ms,
                &default_crop,
                &secondary_store_paths,
            )
        })
        .collect();

    let fallback_reason = if fallback_reasons.is_empty() {
        None
    } else {
        Some(fallback_reasons.join(" "))
    };
    let (source_label, source_detail, empty_message) = palette_metadata(
        request.family,
        catalog.is_some(),
        fallback_reason.as_deref(),
    );

    Ok(ResolveProcessingAuthoringPaletteResponse {
        schema_version: IPC_SCHEMA_VERSION,
        family: request.family,
        items,
        source_label,
        source_detail,
        empty_message,
        fallback_reason,
    })
}

pub fn apply_processing_authoring_session_action(
    workspace: &WorkspaceState,
    request: ApplyProcessingAuthoringSessionActionRequest,
) -> Result<ProcessingAuthoringSessionResponse, String> {
    validate_schema(request.schema_version)?;
    let entry_id = normalize_required_entry_id(&request.entry_id)?;
    let entry = load_entry(workspace, &entry_id)?;
    let active_pipeline_id = normalize_active_pipeline_id(
        &entry.session_pipelines,
        entry.active_session_pipeline_id.clone(),
    );

    let mut state = SessionState {
        pipelines: normalize_session_pipelines(entry.session_pipelines)?,
        active_pipeline_id,
    };
    state.active_pipeline_id =
        normalize_active_pipeline_id(&state.pipelines, state.active_pipeline_id.clone());

    match request.action {
        ProcessingAuthoringSessionAction::EnsureFamilyPipeline { family } => {
            if let Some(existing_id) = find_pipeline_id_for_family(&state.pipelines, family) {
                state.active_pipeline_id = Some(existing_id);
            } else {
                let entry = create_session_pipeline_entry(
                    next_empty_session_pipeline_name(&state.pipelines, family),
                    storage_family(family),
                );
                state.active_pipeline_id = Some(entry.pipeline_id.clone());
                state.pipelines.push(entry);
            }
        }
        ProcessingAuthoringSessionAction::CreatePipeline { family } => {
            let entry = create_session_pipeline_entry(
                next_empty_session_pipeline_name(&state.pipelines, family),
                storage_family(family),
            );
            state.active_pipeline_id = Some(entry.pipeline_id.clone());
            state.pipelines.push(entry);
        }
        ProcessingAuthoringSessionAction::DuplicatePipeline { pipeline_id } => {
            let source = select_pipeline(
                &state.pipelines,
                pipeline_id
                    .as_deref()
                    .or(state.active_pipeline_id.as_deref()),
            )?
            .clone();
            let duplicate = duplicate_pipeline_entry(&state.pipelines, &source);
            state.active_pipeline_id = Some(duplicate.pipeline_id.clone());
            state.pipelines.push(duplicate);
        }
        ProcessingAuthoringSessionAction::ActivatePipeline { pipeline_id } => {
            let pipeline_id = normalize_required_pipeline_id(&pipeline_id)?;
            ensure_pipeline_exists(&state.pipelines, &pipeline_id)?;
            state.active_pipeline_id = Some(pipeline_id);
        }
        ProcessingAuthoringSessionAction::RemovePipeline { pipeline_id } => {
            let pipeline_id = normalize_required_pipeline_id(&pipeline_id)?;
            let remove_index = state
                .pipelines
                .iter()
                .position(|entry| entry.pipeline_id == pipeline_id)
                .ok_or_else(|| format!("Unknown session pipeline: {pipeline_id}"))?;
            if state.pipelines.len() > 1 {
                state.pipelines.remove(remove_index);
                if state.active_pipeline_id.as_deref() == Some(pipeline_id.as_str()) {
                    let fallback_index = remove_index.saturating_sub(1);
                    state.active_pipeline_id = state
                        .pipelines
                        .get(fallback_index)
                        .or_else(|| state.pipelines.first())
                        .map(|entry| entry.pipeline_id.clone());
                }
            }
        }
        ProcessingAuthoringSessionAction::ReplaceActiveFromPipelineSpec { pipeline } => {
            let replacement = pipeline_entry_from_spec(pipeline)?;
            if let Some(active_pipeline_id) =
                normalize_active_pipeline_id(&state.pipelines, state.active_pipeline_id.clone())
            {
                let active_index = state
                    .pipelines
                    .iter()
                    .position(|entry| entry.pipeline_id == active_pipeline_id)
                    .ok_or_else(|| format!("Unknown session pipeline: {active_pipeline_id}"))?;
                let replacement_id = state.pipelines[active_index].pipeline_id.clone();
                state.pipelines[active_index] = WorkspacePipelineEntry {
                    pipeline_id: replacement_id.clone(),
                    updated_at_unix_s: pipeline_timestamp(),
                    ..replacement
                };
                state.active_pipeline_id = Some(replacement_id);
            } else {
                let mut entry = replacement;
                entry.pipeline_id = generate_session_pipeline_id();
                entry.updated_at_unix_s = pipeline_timestamp();
                state.active_pipeline_id = Some(entry.pipeline_id.clone());
                state.pipelines.push(entry);
            }
        }
    }

    save_processing_authoring_session(
        workspace,
        SaveProcessingAuthoringSessionRequest {
            schema_version: IPC_SCHEMA_VERSION,
            entry_id,
            session_pipelines: state.pipelines,
            active_session_pipeline_id: state.active_pipeline_id,
        },
    )
}

pub fn save_processing_authoring_session(
    workspace: &WorkspaceState,
    request: SaveProcessingAuthoringSessionRequest,
) -> Result<ProcessingAuthoringSessionResponse, String> {
    validate_schema(request.schema_version)?;
    let entry_id = normalize_required_entry_id(&request.entry_id)?;
    let session_pipelines = normalize_session_pipelines(request.session_pipelines)?;
    let active_session_pipeline_id =
        normalize_active_pipeline_id(&session_pipelines, request.active_session_pipeline_id);
    let response = workspace.save_processing_session_pipelines(
        &entry_id,
        session_pipelines,
        active_session_pipeline_id,
    )?;
    Ok(ProcessingAuthoringSessionResponse {
        schema_version: IPC_SCHEMA_VERSION,
        entry: response.entry,
        session: response.session,
    })
}

pub fn resolve_processing_run_output(
    app_paths: &AppPaths,
    request: ResolveProcessingRunOutputRequest,
) -> Result<ResolveProcessingRunOutputResponse, String> {
    validate_schema(request.schema_version)?;

    let output_store_path = match request.family {
        ProcessingPipelineFamily::PostStackNeighborhood => {
            let pipeline = request
                .post_stack_neighborhood_pipeline
                .ok_or_else(|| "Missing post_stack_neighborhood_pipeline".to_string())?;
            default_post_stack_neighborhood_processing_store_path(
                app_paths,
                &request.store_path,
                &pipeline,
            )?
        }
        ProcessingPipelineFamily::Subvolume => {
            let crop = request
                .subvolume_crop
                .ok_or_else(|| "Missing subvolume_crop".to_string())?;
            let pipeline = SubvolumeProcessingPipeline {
                schema_version: 2,
                revision: request
                    .pipeline
                    .as_ref()
                    .map_or(1, |pipeline| pipeline.revision),
                preset_id: request
                    .pipeline
                    .as_ref()
                    .and_then(|pipeline| pipeline.preset_id.clone()),
                name: request
                    .pipeline
                    .as_ref()
                    .and_then(|pipeline| pipeline.name.clone()),
                description: request
                    .pipeline
                    .as_ref()
                    .and_then(|pipeline| pipeline.description.clone()),
                trace_local_pipeline: request.pipeline,
                crop,
            };
            default_subvolume_processing_store_path(app_paths, &request.store_path, &pipeline)?
        }
        _ => {
            let pipeline = request
                .pipeline
                .ok_or_else(|| "Missing pipeline".to_string())?;
            default_processing_store_path(app_paths, &request.store_path, &pipeline)?
        }
    };

    Ok(ResolveProcessingRunOutputResponse {
        schema_version: IPC_SCHEMA_VERSION,
        output_store_path,
    })
}

#[derive(Debug, Clone)]
struct SessionState {
    pipelines: Vec<WorkspacePipelineEntry>,
    active_pipeline_id: Option<String>,
}

fn validate_schema(schema_version: u32) -> Result<(), String> {
    if schema_version != IPC_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported processing authoring schema version: {schema_version}"
        ));
    }
    Ok(())
}

fn definition_supports_family(
    definition: &PaletteDefinition,
    family: ProcessingPipelineFamily,
) -> bool {
    match family {
        ProcessingPipelineFamily::PostStackNeighborhood => {
            definition.canonical_family == OperatorFamily::TraceLocalProcessing
        }
        ProcessingPipelineFamily::Gather => false,
        ProcessingPipelineFamily::Subvolume | ProcessingPipelineFamily::TraceLocal => true,
    }
}

fn palette_item_from_definition(
    definition: &PaletteDefinition,
    catalog_entry: Option<&OperatorCatalogEntry>,
    sample_interval_ms: f32,
    default_crop: &SubvolumeCropOperation,
    secondary_store_paths: &[String],
) -> ProcessingAuthoringPaletteItem {
    let canonical_name = catalog_entry
        .map(|entry| entry.name.clone())
        .unwrap_or_else(|| definition.fallback_label.to_string());
    let label = definition
        .alias_label
        .unwrap_or(canonical_name.as_str())
        .to_string();
    let short_help = catalog_entry
        .map(|entry| entry.documentation.short_help.clone())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            catalog_entry
                .map(|entry| entry.description.clone())
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| label.clone());

    let mut keywords = Vec::new();
    let mut seen = HashSet::new();
    for keyword in definition.canonical_id.split('_') {
        push_keyword(&mut keywords, &mut seen, keyword);
    }
    for keyword in definition.fallback_label.split_whitespace() {
        push_keyword(&mut keywords, &mut seen, keyword);
    }
    for keyword in definition.local_keywords {
        push_keyword(&mut keywords, &mut seen, keyword);
    }
    if let Some(entry) = catalog_entry {
        for tag in &entry.tags {
            push_keyword(&mut keywords, &mut seen, tag);
        }
        for parameter in &entry.parameter_docs {
            push_keyword(&mut keywords, &mut seen, &parameter.name);
            push_keyword(&mut keywords, &mut seen, &parameter.label);
            push_keyword(&mut keywords, &mut seen, &parameter.description);
            for option in &parameter.options {
                push_keyword(&mut keywords, &mut seen, option);
            }
        }
    }

    ProcessingAuthoringPaletteItem {
        item_id: definition.item_id.to_string(),
        label,
        description: short_help.clone(),
        short_help,
        help_markdown: catalog_entry.and_then(|entry| entry.documentation.help_markdown.clone()),
        help_url: catalog_entry.and_then(|entry| entry.documentation.help_url.clone()),
        keywords,
        shortcut: definition.shortcut.map(ToOwned::to_owned),
        canonical_id: definition.canonical_id.to_string(),
        canonical_name,
        group: catalog_entry
            .map(|entry| entry.group.clone())
            .unwrap_or_else(|| fallback_group(definition).to_string()),
        group_id: catalog_entry
            .map(|entry| entry.group_id.clone())
            .unwrap_or_else(|| fallback_group_id(definition).to_string()),
        provider: catalog_entry
            .map(|entry| entry.provider.clone())
            .unwrap_or_else(|| PALETTE_PROVIDER_FALLBACK.to_string()),
        tags: catalog_entry
            .map(|entry| entry.tags.clone())
            .unwrap_or_default(),
        parameter_docs: catalog_entry
            .map(|entry| entry.parameter_docs.clone())
            .unwrap_or_default(),
        alias_label: definition.alias_label.map(ToOwned::to_owned),
        source: if catalog_entry.is_some() {
            PALETTE_SOURCE_CANONICAL.to_string()
        } else {
            PALETTE_SOURCE_FALLBACK.to_string()
        },
        insertable: default_insertable(
            definition,
            sample_interval_ms,
            default_crop,
            secondary_store_paths,
        ),
    }
}

fn default_insertable(
    definition: &PaletteDefinition,
    sample_interval_ms: f32,
    default_crop: &SubvolumeCropOperation,
    secondary_store_paths: &[String],
) -> ProcessingAuthoringInsertable {
    match definition.item_id {
        "amplitude_scalar" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::AmplitudeScalar { factor: 1.0 },
        },
        "trace_rms_normalize" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::TraceRmsNormalize,
        },
        "agc_rms" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::AgcRms { window_ms: 250.0 },
        },
        "phase_rotation" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::PhaseRotation { angle_degrees: 0.0 },
        },
        "envelope" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::Envelope,
        },
        "instantaneous_phase" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::InstantaneousPhase,
        },
        "instantaneous_frequency" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::InstantaneousFrequency,
        },
        "sweetness" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::Sweetness,
        },
        "volume_subtract" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_volume_arithmetic(
                secondary_store_paths,
                TraceLocalVolumeArithmeticOperator::Subtract,
            ),
        },
        "volume_add" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_volume_arithmetic(
                secondary_store_paths,
                TraceLocalVolumeArithmeticOperator::Add,
            ),
        },
        "volume_multiply" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_volume_arithmetic(
                secondary_store_paths,
                TraceLocalVolumeArithmeticOperator::Multiply,
            ),
        },
        "volume_divide" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_volume_arithmetic(
                secondary_store_paths,
                TraceLocalVolumeArithmeticOperator::Divide,
            ),
        },
        "crop_subvolume" => ProcessingAuthoringInsertable::SubvolumeCrop {
            crop: default_crop.clone(),
        },
        "lowpass_filter" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_lowpass_filter(sample_interval_ms),
        },
        "highpass_filter" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_highpass_filter(sample_interval_ms),
        },
        "bandpass_filter" => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: default_bandpass_filter(sample_interval_ms),
        },
        _ => ProcessingAuthoringInsertable::TraceLocalOperation {
            operation: TraceLocalProcessingOperation::TraceRmsNormalize,
        },
    }
}

fn fallback_group(definition: &PaletteDefinition) -> &'static str {
    if definition.canonical_family == OperatorFamily::SubvolumeProcessing {
        "Subvolume"
    } else {
        "Trace Local"
    }
}

fn fallback_group_id(definition: &PaletteDefinition) -> &'static str {
    if definition.canonical_family == OperatorFamily::SubvolumeProcessing {
        "subvolume"
    } else {
        "trace_local"
    }
}

fn palette_metadata(
    family: ProcessingPipelineFamily,
    has_catalog: bool,
    fallback_reason: Option<&str>,
) -> (String, String, String) {
    match family {
        ProcessingPipelineFamily::PostStackNeighborhood => (
            "Trace-local neighborhood prefix".to_string(),
            "These trace-local steps run before the terminal neighborhood operator. Prefix checkpoints stay hidden in v1.".to_string(),
            "No trace-local prefix operators are available for this dataset.".to_string(),
        ),
        ProcessingPipelineFamily::Gather => (
            "Gather-native dataset".to_string(),
            "This demo remains section-centric. Gather processing and velocity scans are backend-wired but not exposed here yet.".to_string(),
            "Gather-native authoring is not exposed in this section viewer yet.".to_string(),
        ),
        _ if has_catalog => (
            "Canonical registry-backed".to_string(),
            "Operators are filtered from the core-owned dataset catalog based on the active dataset layout.".to_string(),
            "No catalog-backed operators are available for this dataset.".to_string(),
        ),
        _ => (
            "Demo fallback catalog".to_string(),
            fallback_reason
                .map(|reason| format!("{reason} Showing the demo fallback list instead."))
                .unwrap_or_else(|| {
                    "Using the demo fallback list until the canonical catalog is available."
                        .to_string()
                }),
            "No catalog-backed operators are available for this dataset.".to_string(),
        ),
    }
}

fn load_entry(workspace: &WorkspaceState, entry_id: &str) -> Result<DatasetRegistryEntry, String> {
    workspace
        .load_state()?
        .entries
        .into_iter()
        .find(|entry| entry.entry_id == entry_id)
        .ok_or_else(|| format!("Unknown dataset entry: {entry_id}"))
}

fn normalize_session_pipelines(
    session_pipelines: Vec<WorkspacePipelineEntry>,
) -> Result<Vec<WorkspacePipelineEntry>, String> {
    session_pipelines
        .into_iter()
        .map(normalize_session_pipeline)
        .collect()
}

fn normalize_session_pipeline(
    entry: WorkspacePipelineEntry,
) -> Result<WorkspacePipelineEntry, String> {
    let pipeline_id = normalize_required_pipeline_id(&entry.pipeline_id)?;
    match entry.family {
        ProcessingPipelineFamily::PostStackNeighborhood => {
            let mut pipeline = entry
                .post_stack_neighborhood_pipeline
                .unwrap_or_else(empty_post_stack_neighborhood_pipeline);
            pipeline.name = normalize_optional_string(pipeline.name.as_deref());
            pipeline.description = normalize_optional_string(pipeline.description.as_deref());
            if pipeline.operations.is_empty() {
                pipeline.operations.push(default_neighborhood_similarity());
            }
            Ok(WorkspacePipelineEntry {
                pipeline_id,
                family: ProcessingPipelineFamily::PostStackNeighborhood,
                pipeline: None,
                subvolume_crop: None,
                post_stack_neighborhood_pipeline: Some(pipeline),
                updated_at_unix_s: entry.updated_at_unix_s,
            })
        }
        ProcessingPipelineFamily::TraceLocal | ProcessingPipelineFamily::Subvolume => {
            let mut pipeline = entry.pipeline.unwrap_or_else(empty_trace_local_pipeline);
            pipeline.name = normalize_optional_string(pipeline.name.as_deref());
            pipeline.description = normalize_optional_string(pipeline.description.as_deref());
            Ok(WorkspacePipelineEntry {
                pipeline_id,
                family: ProcessingPipelineFamily::TraceLocal,
                pipeline: Some(pipeline),
                subvolume_crop: normalize_subvolume_crop(entry.subvolume_crop),
                post_stack_neighborhood_pipeline: None,
                updated_at_unix_s: entry.updated_at_unix_s,
            })
        }
        ProcessingPipelineFamily::Gather => Err(
            "Gather authoring is not supported by the TraceBoost processing authoring boundary."
                .to_string(),
        ),
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

fn storage_family(family: ProcessingPipelineFamily) -> ProcessingPipelineFamily {
    match family {
        ProcessingPipelineFamily::PostStackNeighborhood => {
            ProcessingPipelineFamily::PostStackNeighborhood
        }
        _ => ProcessingPipelineFamily::TraceLocal,
    }
}

fn find_pipeline_id_for_family(
    pipelines: &[WorkspacePipelineEntry],
    family: ProcessingPipelineFamily,
) -> Option<String> {
    pipelines
        .iter()
        .find(|entry| entry.family == storage_family(family))
        .map(|entry| entry.pipeline_id.clone())
}

fn select_pipeline<'a>(
    pipelines: &'a [WorkspacePipelineEntry],
    preferred_pipeline_id: Option<&str>,
) -> Result<&'a WorkspacePipelineEntry, String> {
    if let Some(preferred_pipeline_id) = preferred_pipeline_id {
        let preferred_pipeline_id = normalize_required_pipeline_id(preferred_pipeline_id)?;
        if let Some(entry) = pipelines
            .iter()
            .find(|entry| entry.pipeline_id == preferred_pipeline_id)
        {
            return Ok(entry);
        }
    }
    pipelines
        .first()
        .ok_or_else(|| "No session pipelines are available for this dataset entry.".to_string())
}

fn ensure_pipeline_exists(
    pipelines: &[WorkspacePipelineEntry],
    pipeline_id: &str,
) -> Result<(), String> {
    if pipelines
        .iter()
        .any(|entry| entry.pipeline_id == pipeline_id)
    {
        Ok(())
    } else {
        Err(format!("Unknown session pipeline: {pipeline_id}"))
    }
}

fn create_session_pipeline_entry(
    suggested_name: String,
    family: ProcessingPipelineFamily,
) -> WorkspacePipelineEntry {
    match family {
        ProcessingPipelineFamily::PostStackNeighborhood => {
            let mut pipeline = empty_post_stack_neighborhood_pipeline();
            pipeline.name = Some(suggested_name);
            WorkspacePipelineEntry {
                pipeline_id: generate_session_pipeline_id(),
                family: ProcessingPipelineFamily::PostStackNeighborhood,
                pipeline: None,
                subvolume_crop: None,
                post_stack_neighborhood_pipeline: Some(pipeline),
                updated_at_unix_s: pipeline_timestamp(),
            }
        }
        _ => {
            let mut pipeline = empty_trace_local_pipeline();
            pipeline.name = Some(suggested_name);
            WorkspacePipelineEntry {
                pipeline_id: generate_session_pipeline_id(),
                family: ProcessingPipelineFamily::TraceLocal,
                pipeline: Some(pipeline),
                subvolume_crop: None,
                post_stack_neighborhood_pipeline: None,
                updated_at_unix_s: pipeline_timestamp(),
            }
        }
    }
}

fn duplicate_pipeline_entry(
    existing_entries: &[WorkspacePipelineEntry],
    source: &WorkspacePipelineEntry,
) -> WorkspacePipelineEntry {
    match source.family {
        ProcessingPipelineFamily::PostStackNeighborhood => {
            let mut pipeline = source
                .post_stack_neighborhood_pipeline
                .clone()
                .unwrap_or_else(empty_post_stack_neighborhood_pipeline);
            pipeline.preset_id = None;
            if let Some(prefix) = pipeline.trace_local_pipeline.as_mut() {
                prefix.preset_id = None;
            }
            pipeline.name = Some(next_duplicate_name(
                pipeline.name.as_deref().unwrap_or("Neighborhood"),
                &existing_entries
                    .iter()
                    .filter(|entry| entry.family == ProcessingPipelineFamily::PostStackNeighborhood)
                    .map(session_label)
                    .collect::<Vec<_>>(),
            ));
            WorkspacePipelineEntry {
                pipeline_id: generate_session_pipeline_id(),
                family: ProcessingPipelineFamily::PostStackNeighborhood,
                pipeline: None,
                subvolume_crop: None,
                post_stack_neighborhood_pipeline: Some(pipeline),
                updated_at_unix_s: pipeline_timestamp(),
            }
        }
        _ => {
            let mut pipeline = source
                .pipeline
                .clone()
                .unwrap_or_else(empty_trace_local_pipeline);
            pipeline.preset_id = None;
            pipeline.name = Some(next_duplicate_name(
                pipeline.name.as_deref().unwrap_or("Pipeline"),
                &existing_entries
                    .iter()
                    .filter(|entry| entry.family == ProcessingPipelineFamily::TraceLocal)
                    .map(session_label)
                    .collect::<Vec<_>>(),
            ));
            WorkspacePipelineEntry {
                pipeline_id: generate_session_pipeline_id(),
                family: ProcessingPipelineFamily::TraceLocal,
                pipeline: Some(pipeline),
                subvolume_crop: source.subvolume_crop.clone(),
                post_stack_neighborhood_pipeline: None,
                updated_at_unix_s: pipeline_timestamp(),
            }
        }
    }
}

fn pipeline_entry_from_spec(
    pipeline: ProcessingPipelineSpec,
) -> Result<WorkspacePipelineEntry, String> {
    match pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => Ok(WorkspacePipelineEntry {
            pipeline_id: String::new(),
            family: ProcessingPipelineFamily::TraceLocal,
            pipeline: Some(pipeline),
            subvolume_crop: None,
            post_stack_neighborhood_pipeline: None,
            updated_at_unix_s: pipeline_timestamp(),
        }),
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => Ok(WorkspacePipelineEntry {
            pipeline_id: String::new(),
            family: ProcessingPipelineFamily::PostStackNeighborhood,
            pipeline: None,
            subvolume_crop: None,
            post_stack_neighborhood_pipeline: Some(pipeline),
            updated_at_unix_s: pipeline_timestamp(),
        }),
        ProcessingPipelineSpec::Subvolume { pipeline } => Ok(WorkspacePipelineEntry {
            pipeline_id: String::new(),
            family: ProcessingPipelineFamily::TraceLocal,
            pipeline: Some(trace_local_from_subvolume(&pipeline)),
            subvolume_crop: Some(pipeline.crop),
            post_stack_neighborhood_pipeline: None,
            updated_at_unix_s: pipeline_timestamp(),
        }),
        ProcessingPipelineSpec::Gather { .. } => Err(
            "Gather authoring is not supported by the TraceBoost processing authoring boundary."
                .to_string(),
        ),
    }
}

fn trace_local_from_subvolume(
    pipeline: &SubvolumeProcessingPipeline,
) -> TraceLocalProcessingPipeline {
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        let mut trace_local = trace_local_pipeline.clone();
        trace_local.schema_version = pipeline.schema_version;
        trace_local.revision = pipeline.revision;
        trace_local.preset_id = pipeline.preset_id.clone();
        trace_local.name = pipeline.name.clone();
        trace_local.description = pipeline.description.clone();
        return trace_local;
    }

    TraceLocalProcessingPipeline {
        schema_version: pipeline.schema_version,
        revision: pipeline.revision,
        preset_id: pipeline.preset_id.clone(),
        name: pipeline.name.clone(),
        description: pipeline.description.clone(),
        steps: Vec::new(),
    }
}

fn session_label(entry: &WorkspacePipelineEntry) -> String {
    match entry.family {
        ProcessingPipelineFamily::PostStackNeighborhood => entry
            .post_stack_neighborhood_pipeline
            .as_ref()
            .and_then(|pipeline| normalize_optional_string(pipeline.name.as_deref()))
            .unwrap_or_else(|| "Neighborhood".to_string()),
        _ => entry
            .pipeline
            .as_ref()
            .and_then(|pipeline| normalize_optional_string(pipeline.name.as_deref()))
            .unwrap_or_else(|| "Pipeline".to_string()),
    }
}

fn next_empty_session_pipeline_name(
    entries: &[WorkspacePipelineEntry],
    family: ProcessingPipelineFamily,
) -> String {
    let (base, label) = if family == ProcessingPipelineFamily::PostStackNeighborhood {
        ("neighborhood", "Neighborhood")
    } else {
        ("pipeline", "Pipeline")
    };
    let existing = entries
        .iter()
        .filter(|entry| entry.family == storage_family(family))
        .map(|entry| session_label(entry).trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    if !existing.iter().any(|name| name == base) {
        return label.to_string();
    }
    let mut index = 2;
    loop {
        let candidate = format!("{base} {index}");
        if !existing.iter().any(|name| name == &candidate) {
            return format!("{label} {index}");
        }
        index += 1;
    }
}

fn next_duplicate_name(source_name: &str, existing_names: &[String]) -> String {
    let source =
        normalize_optional_string(Some(source_name)).unwrap_or_else(|| "Pipeline".to_string());
    let (base, _) = split_duplicate_suffix(&source);
    let lower_base = base.to_ascii_lowercase();
    let mut max_suffix = 0_u32;
    for existing in existing_names {
        let (existing_base, suffix) = split_duplicate_suffix(existing);
        if existing_base.to_ascii_lowercase() == lower_base {
            max_suffix = max_suffix.max(suffix.unwrap_or(0));
        }
    }
    format!("{base}_{}", max_suffix + 1)
}

fn split_duplicate_suffix(value: &str) -> (String, Option<u32>) {
    if let Some((base, suffix)) = value.rsplit_once('_') {
        if let Ok(number) = suffix.parse::<u32>() {
            let base = base.trim();
            if !base.is_empty() {
                return (base.to_string(), Some(number));
            }
        }
    }
    (value.trim().to_string(), None)
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
        operations: vec![default_neighborhood_similarity()],
    }
}

fn default_neighborhood_similarity() -> PostStackNeighborhoodProcessingOperation {
    PostStackNeighborhoodProcessingOperation::Similarity {
        window: PostStackNeighborhoodWindow {
            gate_ms: 24.0,
            inline_stepout: 1,
            xline_stepout: 1,
        },
    }
}

fn empty_subvolume_crop() -> SubvolumeCropOperation {
    SubvolumeCropOperation {
        inline_min: 0,
        inline_max: 0,
        xline_min: 0,
        xline_max: 0,
        z_min_ms: 0.0,
        z_max_ms: 0.0,
    }
}

fn default_subvolume_crop_from_dataset(dataset: &DatasetSummary) -> SubvolumeCropOperation {
    let summary = &dataset.descriptor.geometry.summary;
    SubvolumeCropOperation {
        inline_min: summary.inline_axis.first,
        inline_max: summary.inline_axis.last,
        xline_min: summary.xline_axis.first,
        xline_max: summary.xline_axis.last,
        z_min_ms: summary.sample_axis.first,
        z_max_ms: summary.sample_axis.last,
    }
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

fn default_volume_arithmetic(
    secondary_store_paths: &[String],
    operator: TraceLocalVolumeArithmeticOperator,
) -> TraceLocalProcessingOperation {
    TraceLocalProcessingOperation::VolumeArithmetic {
        operator,
        secondary_store_path: secondary_store_paths.first().cloned().unwrap_or_default(),
    }
}

fn default_lowpass_filter(sample_interval_ms: f32) -> TraceLocalProcessingOperation {
    let nyquist = 500.0 / sample_interval_ms.max(0.001);
    let f3_hz = f32::max(20.0, nyquist * 0.12);
    let f4_hz = f32::min(nyquist, f32::max(f3_hz + 8.0, nyquist * 0.18));
    TraceLocalProcessingOperation::LowpassFilter {
        f3_hz: round_tenths(f3_hz),
        f4_hz: round_tenths(f4_hz),
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn default_highpass_filter(sample_interval_ms: f32) -> TraceLocalProcessingOperation {
    let nyquist = 500.0 / sample_interval_ms.max(0.001);
    let f1_hz = f32::max(2.0, nyquist * 0.015);
    let f2_hz = f32::min(nyquist, f32::max(f1_hz + 2.0, nyquist * 0.04));
    TraceLocalProcessingOperation::HighpassFilter {
        f1_hz: round_tenths(f1_hz),
        f2_hz: round_tenths(f2_hz),
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn default_bandpass_filter(sample_interval_ms: f32) -> TraceLocalProcessingOperation {
    let nyquist = 500.0 / sample_interval_ms.max(0.001);
    let f1_hz = f32::max(4.0, nyquist * 0.06);
    let f2_hz = f32::max(f1_hz + 1.0, nyquist * 0.10);
    let f4_hz = f32::min(nyquist, f32::max(f2_hz + 6.0, nyquist * 0.45));
    let f3_hz = f32::min(f4_hz, f32::max(f2_hz + 4.0, nyquist * 0.32));
    TraceLocalProcessingOperation::BandpassFilter {
        f1_hz: round_tenths(f1_hz),
        f2_hz: round_tenths(f2_hz),
        f3_hz: round_tenths(f3_hz),
        f4_hz: round_tenths(f4_hz),
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn round_tenths(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

fn generate_session_pipeline_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    format!("session-pipeline-{nanos}")
}

fn pipeline_timestamp() -> u64 {
    unix_timestamp_s()
}

fn push_keyword(keywords: &mut Vec<String>, seen: &mut HashSet<String>, raw: &str) {
    for token in raw
        .split(|ch: char| ch.is_whitespace() || ch == '_' || ch == '-')
        .filter(|token| !token.is_empty())
    {
        let normalized = token.trim().to_ascii_lowercase();
        if !normalized.is_empty() && seen.insert(normalized.clone()) {
            keywords.push(normalized);
        }
    }
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_required_entry_id(entry_id: &str) -> Result<String, String> {
    normalize_optional_string(Some(entry_id))
        .ok_or_else(|| "Processing authoring entry_id is required".to_string())
}

fn normalize_required_pipeline_id(pipeline_id: &str) -> Result<String, String> {
    normalize_optional_string(Some(pipeline_id))
        .ok_or_else(|| "Processing authoring pipeline_id is required".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::PathBuf;

    use seis_contracts_operations::datasets::UpsertDatasetEntryRequest;

    fn temp_file(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("traceboost-processing-authoring-test-{unique}"));
        fs::create_dir_all(&base).expect("create temp dir");
        base.join(name)
    }

    fn initialize_workspace() -> WorkspaceState {
        let registry = temp_file("registry.json");
        let session = temp_file("session.json");
        WorkspaceState::initialize(&registry, &session).expect("initialize workspace")
    }

    fn insert_workspace_entry(state: &WorkspaceState, entry_id: &str) {
        state
            .upsert_entry(UpsertDatasetEntryRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: Some(entry_id.to_string()),
                display_name: Some("Demo".to_string()),
                source_path: Some("C:/data/demo.segy".to_string()),
                preferred_store_path: Some("C:/data/demo.tbvol".to_string()),
                imported_store_path: Some("C:/data/demo.tbvol".to_string()),
                dataset: None,
                session_pipelines: None,
                active_session_pipeline_id: None,
                make_active: true,
            })
            .expect("insert workspace entry");
    }

    #[test]
    fn processing_authoring_palette_fallback_provides_defaults() {
        let response =
            resolve_processing_authoring_palette(ResolveProcessingAuthoringPaletteRequest {
                schema_version: IPC_SCHEMA_VERSION,
                family: ProcessingPipelineFamily::TraceLocal,
                store_path: None,
                secondary_store_paths: vec!["/tmp/secondary.tbvol".to_string()],
            })
            .expect("resolve palette");

        assert_eq!(response.source_label, "Demo fallback catalog");
        assert!(
            response
                .items
                .iter()
                .any(|item| item.item_id == "crop_subvolume")
        );

        let subtract = response
            .items
            .iter()
            .find(|item| item.item_id == "volume_subtract")
            .expect("subtract item");
        match &subtract.insertable {
            ProcessingAuthoringInsertable::TraceLocalOperation {
                operation:
                    TraceLocalProcessingOperation::VolumeArithmetic {
                        operator,
                        secondary_store_path,
                    },
            } => {
                assert_eq!(*operator, TraceLocalVolumeArithmeticOperator::Subtract);
                assert_eq!(secondary_store_path, "/tmp/secondary.tbvol");
            }
            other => panic!("unexpected insertable: {other:?}"),
        }
    }

    #[test]
    fn save_processing_authoring_session_normalizes_trace_local_entries() {
        let state = initialize_workspace();
        insert_workspace_entry(&state, "dataset-a");

        let saved =
            save_processing_authoring_session(
                &state,
                SaveProcessingAuthoringSessionRequest {
                    schema_version: IPC_SCHEMA_VERSION,
                    entry_id: "dataset-a".to_string(),
                    session_pipelines: vec![WorkspacePipelineEntry {
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
                        updated_at_unix_s: 10,
                    }],
                    active_session_pipeline_id: Some("missing".to_string()),
                },
            )
            .expect("save session");

        let pipeline = &saved.entry.session_pipelines[0];
        assert_eq!(pipeline.pipeline_id, "pipeline-1");
        assert_eq!(pipeline.family, ProcessingPipelineFamily::TraceLocal);
        assert!(pipeline.pipeline.is_some());
        assert!(pipeline.post_stack_neighborhood_pipeline.is_none());
        let crop = pipeline.subvolume_crop.as_ref().expect("crop");
        assert_eq!((crop.inline_min, crop.inline_max), (20, 40));
    }

    #[test]
    fn save_processing_authoring_session_rejects_gather_entries() {
        let state = initialize_workspace();
        insert_workspace_entry(&state, "dataset-b");

        let error = save_processing_authoring_session(
            &state,
            SaveProcessingAuthoringSessionRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: "dataset-b".to_string(),
                session_pipelines: vec![WorkspacePipelineEntry {
                    pipeline_id: "gather-1".to_string(),
                    family: ProcessingPipelineFamily::Gather,
                    pipeline: None,
                    subvolume_crop: None,
                    post_stack_neighborhood_pipeline: None,
                    updated_at_unix_s: 10,
                }],
                active_session_pipeline_id: Some("gather-1".to_string()),
            },
        )
        .err()
        .expect("gather should be rejected");

        assert!(error.contains("Gather authoring is not supported"));
    }

    #[test]
    fn processing_authoring_session_actions_create_duplicate_and_replace() {
        let state = initialize_workspace();
        insert_workspace_entry(&state, "dataset-c");

        let created = apply_processing_authoring_session_action(
            &state,
            ApplyProcessingAuthoringSessionActionRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: "dataset-c".to_string(),
                action: ProcessingAuthoringSessionAction::CreatePipeline {
                    family: ProcessingPipelineFamily::TraceLocal,
                },
            },
        )
        .expect("create pipeline");
        let active_pipeline_id = created
            .entry
            .active_session_pipeline_id
            .clone()
            .expect("active id");

        let duplicated = apply_processing_authoring_session_action(
            &state,
            ApplyProcessingAuthoringSessionActionRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: "dataset-c".to_string(),
                action: ProcessingAuthoringSessionAction::DuplicatePipeline {
                    pipeline_id: Some(active_pipeline_id.clone()),
                },
            },
        )
        .expect("duplicate pipeline");
        let duplicate_id = duplicated
            .entry
            .active_session_pipeline_id
            .clone()
            .expect("duplicate active id");
        assert_ne!(duplicate_id, active_pipeline_id);

        let replaced = apply_processing_authoring_session_action(
            &state,
            ApplyProcessingAuthoringSessionActionRequest {
                schema_version: IPC_SCHEMA_VERSION,
                entry_id: "dataset-c".to_string(),
                action: ProcessingAuthoringSessionAction::ReplaceActiveFromPipelineSpec {
                    pipeline: ProcessingPipelineSpec::Subvolume {
                        pipeline: SubvolumeProcessingPipeline {
                            schema_version: 2,
                            revision: 7,
                            preset_id: Some("preset-1".to_string()),
                            name: Some("Subset".to_string()),
                            description: Some("Subvolume preset".to_string()),
                            trace_local_pipeline: Some(TraceLocalProcessingPipeline {
                                schema_version: 2,
                                revision: 7,
                                preset_id: Some("preset-1".to_string()),
                                name: None,
                                description: None,
                                steps: vec![],
                            }),
                            crop: SubvolumeCropOperation {
                                inline_min: 10,
                                inline_max: 20,
                                xline_min: 30,
                                xline_max: 40,
                                z_min_ms: 50.0,
                                z_max_ms: 60.0,
                            },
                        },
                    },
                },
            },
        )
        .expect("replace pipeline");

        let replaced_entry = replaced
            .entry
            .session_pipelines
            .iter()
            .find(|entry| entry.pipeline_id == duplicate_id)
            .expect("replaced entry");
        assert_eq!(replaced_entry.family, ProcessingPipelineFamily::TraceLocal);
        assert!(replaced_entry.subvolume_crop.is_some());
    }
}
