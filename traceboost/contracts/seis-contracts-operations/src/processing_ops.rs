pub use ophiolite_seismic::contracts::operations::{
    AmplitudeSpectrumRequest, AmplitudeSpectrumResponse, CancelProcessingBatchRequest,
    CancelProcessingBatchResponse, CancelProcessingJobRequest, CancelProcessingJobResponse,
    DeletePipelinePresetRequest, DeletePipelinePresetResponse, GetProcessingBatchRequest,
    GetProcessingBatchResponse, GetProcessingJobRequest, GetProcessingJobResponse,
    ListPipelinePresetsResponse, PreviewCommand, PreviewGatherProcessingRequest,
    PreviewGatherProcessingResponse, PreviewPostStackNeighborhoodProcessingRequest,
    PreviewPostStackNeighborhoodProcessingResponse, PreviewResponse,
    PreviewSubvolumeProcessingRequest, PreviewSubvolumeProcessingResponse,
    PreviewTraceLocalProcessingRequest, PreviewTraceLocalProcessingResponse,
    RunGatherProcessingRequest, RunGatherProcessingResponse,
    RunPostStackNeighborhoodProcessingRequest, RunPostStackNeighborhoodProcessingResponse,
    RunSubvolumeProcessingRequest, RunSubvolumeProcessingResponse, RunTraceLocalProcessingRequest,
    RunTraceLocalProcessingResponse, SavePipelinePresetRequest, SavePipelinePresetResponse,
    SubmitProcessingBatchRequest, SubmitProcessingBatchResponse,
    SubmitTraceLocalProcessingBatchRequest, SubmitTraceLocalProcessingBatchResponse,
    VelocityScanRequest, VelocityScanResponse,
};
pub use ophiolite_seismic::{
    GatherProcessingPipeline, GatherRequest, GatherView, LocalVolumeStatistic,
    NeighborhoodDipOutput, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, PostStackNeighborhoodWindow,
    ProcessingBatchItemRequest, ProcessingBatchItemStatus, ProcessingBatchProgress,
    ProcessingBatchState, ProcessingBatchStatus, ProcessingJobArtifact, ProcessingJobArtifactKind,
    SubvolumeCropOperation, SubvolumeProcessingPipeline, VelocityAutopickParameters,
    VelocityFunctionEstimate, VelocityFunctionSource, VelocityIntervalTrend, VelocityPickStrategy,
    VelocityQuantityKind,
};

pub fn encode_preview_command(command: &PreviewCommand) -> serde_json::Result<String> {
    serde_json::to_string(command)
}

pub fn decode_preview_command(json: &str) -> serde_json::Result<PreviewCommand> {
    serde_json::from_str(json)
}
