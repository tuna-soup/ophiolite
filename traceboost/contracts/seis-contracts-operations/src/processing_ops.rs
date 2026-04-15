pub use ophiolite_seismic::contracts::operations::{
    AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, CancelProcessingJobRequest,
    CancelProcessingJobResponse, DeletePipelinePresetRequest, DeletePipelinePresetResponse,
    GetProcessingJobRequest, GetProcessingJobResponse, ListPipelinePresetsResponse, PreviewCommand,
    PreviewGatherProcessingRequest, PreviewGatherProcessingResponse, PreviewResponse,
    PreviewSubvolumeProcessingRequest, PreviewSubvolumeProcessingResponse,
    PreviewTraceLocalProcessingRequest, PreviewTraceLocalProcessingResponse,
    RunGatherProcessingRequest, RunGatherProcessingResponse, RunSubvolumeProcessingRequest,
    RunSubvolumeProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SavePipelinePresetRequest, SavePipelinePresetResponse,
    VelocityScanRequest, VelocityScanResponse,
};
pub use ophiolite_seismic::{
    GatherProcessingPipeline, GatherRequest, GatherView, ProcessingJobArtifact,
    ProcessingJobArtifactKind, SubvolumeCropOperation, SubvolumeProcessingPipeline,
    VelocityAutopickParameters, VelocityFunctionEstimate, VelocityFunctionSource,
    VelocityIntervalTrend, VelocityPickStrategy, VelocityQuantityKind,
};

pub fn encode_preview_command(command: &PreviewCommand) -> serde_json::Result<String> {
    serde_json::to_string(command)
}

pub fn decode_preview_command(json: &str) -> serde_json::Result<PreviewCommand> {
    serde_json::from_str(json)
}
