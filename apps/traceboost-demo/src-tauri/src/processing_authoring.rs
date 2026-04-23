use ophiolite::IPC_SCHEMA_VERSION;
use seis_contracts_operations::datasets::UpsertDatasetEntryResponse;
use seis_contracts_operations::workspace::{
    PostStackNeighborhoodProcessingPipeline, ProcessingPipelineFamily, SubvolumeCropOperation,
    WorkspacePipelineEntry,
};
use seis_runtime::TraceLocalProcessingPipeline;
use serde::{Deserialize, Serialize};

use crate::app_paths::AppPaths;
use crate::workspace::WorkspaceState;
use crate::{
    default_post_stack_neighborhood_processing_store_path, default_processing_store_path,
    default_subvolume_processing_store_path,
};
use seis_runtime::SubvolumeProcessingPipeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistProcessingSessionPipelinesRequest {
    pub schema_version: u32,
    pub entry_id: String,
    #[serde(default)]
    pub session_pipelines: Vec<WorkspacePipelineEntry>,
    #[serde(default)]
    pub active_session_pipeline_id: Option<String>,
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

pub fn persist_processing_session_pipelines(
    workspace: &WorkspaceState,
    request: PersistProcessingSessionPipelinesRequest,
) -> Result<UpsertDatasetEntryResponse, String> {
    if request.schema_version != IPC_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported processing authoring schema version: {}",
            request.schema_version
        ));
    }
    workspace.save_processing_session_pipelines(
        &request.entry_id,
        request.session_pipelines,
        request.active_session_pipeline_id,
    )
}

pub fn resolve_processing_run_output(
    app_paths: &AppPaths,
    request: ResolveProcessingRunOutputRequest,
) -> Result<ResolveProcessingRunOutputResponse, String> {
    if request.schema_version != IPC_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported processing authoring schema version: {}",
            request.schema_version
        ));
    }

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
