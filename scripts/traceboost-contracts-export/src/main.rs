use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::schema_for;
use ts_rs::{Config, ExportError, TS};

macro_rules! public_contracts {
    ($callback:ident, $($args:tt)*) => {
        $callback! {
            $($args)*
            {
                "DatasetId" => seis_contracts_core::DatasetId,
                "AxisSummaryF32" => seis_contracts_core::AxisSummaryF32,
                "AxisSummaryI32" => seis_contracts_core::AxisSummaryI32,
                "GeometryDescriptor" => seis_contracts_core::GeometryDescriptor,
                "GeometryProvenanceSummary" => seis_contracts_core::GeometryProvenanceSummary,
                "GeometrySummary" => seis_contracts_core::GeometrySummary,
                "ProcessingArtifactRole" => seis_contracts_core::ProcessingArtifactRole,
                "ProcessingLineageSummary" => seis_contracts_core::ProcessingLineageSummary,
                "SampleDataConversionKind" => seis_contracts_core::SampleDataConversionKind,
                "SampleDataFidelity" => seis_contracts_core::SampleDataFidelity,
                "SampleValuePreservation" => seis_contracts_core::SampleValuePreservation,
                "VolumeDescriptor" => seis_contracts_core::VolumeDescriptor,
                "SectionAxis" => seis_contracts_core::SectionAxis,
                "SectionRequest" => seis_contracts_core::SectionRequest,
                "GatherRequest" => seis_contracts_core::GatherRequest,
                "GatherSelector" => seis_contracts_core::GatherSelector,
                "SectionTileRequest" => seis_contracts_core::SectionTileRequest,
                "FrequencyPhaseMode" => seis_contracts_core::FrequencyPhaseMode,
                "FrequencyWindowShape" => seis_contracts_core::FrequencyWindowShape,
                "VelocityFunctionSource" => seis_contracts_core::VelocityFunctionSource,
                "VelocityQuantityKind" => seis_contracts_core::VelocityQuantityKind,
                "GatherInterpolationMode" => seis_contracts_core::GatherInterpolationMode,
                "SectionSpectrumSelection" => seis_contracts_core::SectionSpectrumSelection,
                "AmplitudeSpectrumCurve" => seis_contracts_core::AmplitudeSpectrumCurve,
                "AmplitudeSpectrumRequest" => seis_contracts_core::AmplitudeSpectrumRequest,
                "AmplitudeSpectrumResponse" => seis_contracts_core::AmplitudeSpectrumResponse,
                "TraceLocalProcessingOperation" => seis_contracts_core::TraceLocalProcessingOperation,
                "TraceLocalProcessingPipeline" => seis_contracts_core::TraceLocalProcessingPipeline,
                "TraceLocalProcessingStep" => seis_contracts_core::TraceLocalProcessingStep,
                "PostStackNeighborhoodWindow" => seis_contracts_core::processing::PostStackNeighborhoodWindow,
                "LocalVolumeStatistic" => seis_contracts_core::processing::LocalVolumeStatistic,
                "NeighborhoodDipOutput" => seis_contracts_core::processing::NeighborhoodDipOutput,
                "PostStackNeighborhoodProcessingOperation" => seis_contracts_core::processing::PostStackNeighborhoodProcessingOperation,
                "PostStackNeighborhoodProcessingPipeline" => seis_contracts_core::processing::PostStackNeighborhoodProcessingPipeline,
                "SubvolumeCropOperation" => seis_contracts_core::processing::SubvolumeCropOperation,
                "SubvolumeProcessingPipeline" => seis_contracts_core::processing::SubvolumeProcessingPipeline,
                "TraceLocalVolumeArithmeticOperator" => seis_contracts_core::TraceLocalVolumeArithmeticOperator,
                "GatherProcessingOperation" => seis_contracts_core::GatherProcessingOperation,
                "GatherProcessingPipeline" => seis_contracts_core::GatherProcessingPipeline,
                "ProcessingPipelineFamily" => seis_contracts_core::ProcessingPipelineFamily,
                "ProcessingPipelineSpec" => seis_contracts_core::ProcessingPipelineSpec,
                "ProcessingJobState" => seis_contracts_core::ProcessingJobState,
                "ProcessingJobProgress" => seis_contracts_core::ProcessingJobProgress,
                "ProcessingJobArtifactKind" => seis_contracts_core::ProcessingJobArtifactKind,
                "ProcessingJobArtifact" => seis_contracts_core::ProcessingJobArtifact,
                "ProcessingJobStageExecutionSummary" => seis_contracts_core::processing::ProcessingJobStageExecutionSummary,
                "ProcessingJobChunkPlanSummary" => seis_contracts_core::processing::ProcessingJobChunkPlanSummary,
                "ProcessingJobExecutionSummary" => seis_contracts_core::processing::ProcessingJobExecutionSummary,
                "ProcessingJobPlanSummary" => seis_contracts_core::processing::ProcessingJobPlanSummary,
                "ProcessingJobStatus" => seis_contracts_core::ProcessingJobStatus,
                "ProcessingJobQueueClass" => seis_contracts_core::ProcessingJobQueueClass,
                "ProcessingJobRuntimeSnapshot" => seis_contracts_core::ProcessingJobRuntimeSnapshot,
                "ProcessingJobRuntimeState" => seis_contracts_core::ProcessingJobRuntimeState,
                "ProcessingRuntimePolicyDivergence" => seis_contracts_core::processing::ProcessingRuntimePolicyDivergence,
                "ProcessingRuntimePolicyDivergenceField" => seis_contracts_core::processing::ProcessingRuntimePolicyDivergenceField,
                "ProcessingJobWaitReason" => seis_contracts_core::ProcessingJobWaitReason,
                "ProcessingRuntimeEvent" => seis_contracts_core::ProcessingRuntimeEvent,
                "ProcessingRuntimeEventDetails" => seis_contracts_core::ProcessingRuntimeEventDetails,
                "ProcessingRuntimeEventKind" => seis_contracts_core::ProcessingRuntimeEventKind,
                "ProcessingRuntimeState" => seis_contracts_core::ProcessingRuntimeState,
                "ProcessingStageRuntimeSnapshot" => seis_contracts_core::ProcessingStageRuntimeSnapshot,
                "SectionAssemblyArtifactKind" => seis_contracts_core::SectionAssemblyArtifactKind,
                "SectionAssemblyDebugRecord" => seis_contracts_core::SectionAssemblyDebugRecord,
                "SectionAssemblyDebugSourceTile" => seis_contracts_core::SectionAssemblyDebugSourceTile,
                "InspectableArtifactDerivation" => seis_contracts_core::InspectableArtifactDerivation,
                "InspectableArtifactKey" => seis_contracts_core::InspectableArtifactKey,
                "InspectableArtifactLifetimeClass" => seis_contracts_core::InspectableArtifactLifetimeClass,
                "InspectableBoundaryReason" => seis_contracts_core::InspectableBoundaryReason,
                "InspectableCacheMode" => seis_contracts_core::InspectableCacheMode,
                "InspectableChunkGridSpec" => seis_contracts_core::InspectableChunkGridSpec,
                "InspectableChunkShapePolicy" => seis_contracts_core::InspectableChunkShapePolicy,
                "InspectableCostClass" => seis_contracts_core::InspectableCostClass,
                "InspectableCostEstimate" => seis_contracts_core::InspectableCostEstimate,
                "InspectableDecisionFactor" => seis_contracts_core::InspectableDecisionFactor,
                "InspectableExclusiveScope" => seis_contracts_core::InspectableExclusiveScope,
                "InspectableExecutionArtifactRole" => seis_contracts_core::InspectableExecutionArtifactRole,
                "InspectableExecutionPipelineSegment" => seis_contracts_core::InspectableExecutionPipelineSegment,
                "InspectableExecutionPlan" => seis_contracts_core::InspectableExecutionPlan,
                "InspectableExecutionPlanSummary" => seis_contracts_core::InspectableExecutionPlanSummary,
                "InspectableExecutionPriorityClass" => seis_contracts_core::InspectableExecutionPriorityClass,
                "InspectableExecutionQueueClass" => seis_contracts_core::InspectableExecutionQueueClass,
                "InspectableExecutionStage" => seis_contracts_core::InspectableExecutionStage,
                "InspectableExecutionStageKind" => seis_contracts_core::InspectableExecutionStageKind,
                "InspectableGeometryFingerprints" => seis_contracts_core::InspectableGeometryFingerprints,
                "InspectableHaloSpec" => seis_contracts_core::InspectableHaloSpec,
                "InspectableLogicalDomain" => seis_contracts_core::InspectableLogicalDomain,
                "InspectableMaterializationClass" => seis_contracts_core::InspectableMaterializationClass,
                "InspectableParallelEfficiencyClass" => seis_contracts_core::InspectableParallelEfficiencyClass,
                "InspectablePartitionDomain" => seis_contracts_core::InspectablePartitionDomain,
                "InspectablePartitionFamily" => seis_contracts_core::InspectablePartitionFamily,
                "InspectablePartitionOrdering" => seis_contracts_core::InspectablePartitionOrdering,
                "InspectablePartitionPlan" => seis_contracts_core::InspectablePartitionPlan,
                "InspectablePlanDecision" => seis_contracts_core::InspectablePlanDecision,
                "InspectablePlanDecisionKind" => seis_contracts_core::InspectablePlanDecisionKind,
                "InspectablePlanDecisionSubjectKind" => seis_contracts_core::InspectablePlanDecisionSubjectKind,
                "InspectablePlannerDiagnostics" => seis_contracts_core::InspectablePlannerDiagnostics,
                "InspectablePlannerPassId" => seis_contracts_core::InspectablePlannerPassId,
                "InspectablePlannerPassSnapshot" => seis_contracts_core::InspectablePlannerPassSnapshot,
                "InspectablePlannedArtifact" => seis_contracts_core::InspectablePlannedArtifact,
                "InspectablePlanningMode" => seis_contracts_core::InspectablePlanningMode,
                "InspectablePlanSource" => seis_contracts_core::InspectablePlanSource,
                "InspectableProcessingPlan" => seis_contracts_core::InspectableProcessingPlan,
                "InspectableProgressGranularity" => seis_contracts_core::InspectableProgressGranularity,
                "InspectableProgressUnits" => seis_contracts_core::InspectableProgressUnits,
                "InspectableRetryGranularity" => seis_contracts_core::InspectableRetryGranularity,
                "InspectableRetryPolicy" => seis_contracts_core::InspectableRetryPolicy,
                "InspectableReuseClass" => seis_contracts_core::InspectableReuseClass,
                "InspectableReuseDecision" => seis_contracts_core::InspectableReuseDecision,
                "InspectableReuseDecisionEvidence" => seis_contracts_core::InspectableReuseDecisionEvidence,
                "InspectableReuseDecisionOutcome" => seis_contracts_core::InspectableReuseDecisionOutcome,
                "InspectableSchedulerHints" => seis_contracts_core::InspectableSchedulerHints,
                "InspectableSectionDomain" => seis_contracts_core::InspectableSectionDomain,
                "InspectableSectionWindowDomain" => seis_contracts_core::InspectableSectionWindowDomain,
                "InspectableSemanticPlan" => seis_contracts_core::InspectableSemanticPlan,
                "InspectableSemanticRootNode" => seis_contracts_core::InspectableSemanticRootNode,
                "InspectableSpillabilityClass" => seis_contracts_core::InspectableSpillabilityClass,
                "InspectableStageClassification" => seis_contracts_core::InspectableStageClassification,
                "InspectableStageMemoryProfile" => seis_contracts_core::InspectableStageMemoryProfile,
                "InspectableStagePlanningDecision" => seis_contracts_core::InspectableStagePlanningDecision,
                "InspectableStageResourceEnvelope" => seis_contracts_core::InspectableStageResourceEnvelope,
                "InspectableTileDomain" => seis_contracts_core::InspectableTileDomain,
                "InspectableTraceLocalSegment" => seis_contracts_core::InspectableTraceLocalSegment,
                "InspectableTraceLocalSemanticPlan" => seis_contracts_core::InspectableTraceLocalSemanticPlan,
                "InspectableValidationReport" => seis_contracts_core::InspectableValidationReport,
                "InspectableVolumeDomain" => seis_contracts_core::InspectableVolumeDomain,
                "ProcessingBatchState" => seis_contracts_core::processing::ProcessingBatchState,
                "ProcessingBatchProgress" => seis_contracts_core::processing::ProcessingBatchProgress,
                "ProcessingBatchItemStatus" => seis_contracts_core::processing::ProcessingBatchItemStatus,
                "ProcessingBatchStatus" => seis_contracts_core::processing::ProcessingBatchStatus,
                "ProcessingExecutionMode" => seis_contracts_core::processing::ProcessingExecutionMode,
                "ProcessingSchedulerReason" => seis_contracts_core::processing::ProcessingSchedulerReason,
                "ProcessingPreset" => seis_contracts_core::ProcessingPreset,
                "InterpretationPoint" => seis_contracts_core::InterpretationPoint,
                "SectionColorMap" => seis_contracts_core::views::SectionColorMap,
                "SectionRenderMode" => seis_contracts_core::views::SectionRenderMode,
                "SectionPolarity" => seis_contracts_core::views::SectionPolarity,
                "SectionPrimaryMode" => seis_contracts_core::views::SectionPrimaryMode,
                "SectionCoordinate" => seis_contracts_core::views::SectionCoordinate,
                "SectionUnits" => seis_contracts_core::views::SectionUnits,
                "SectionMetadata" => seis_contracts_core::views::SectionMetadata,
                "SectionDisplayDefaults" => seis_contracts_core::views::SectionDisplayDefaults,
                "SectionView" => seis_contracts_core::views::SectionView,
                "SectionTimeDepthTransformMode" => seis_contracts_core::views::SectionTimeDepthTransformMode,
                "SectionTimeDepthDiagnostics" => seis_contracts_core::views::SectionTimeDepthDiagnostics,
                "SectionScalarOverlayColorMap" => seis_contracts_core::views::SectionScalarOverlayColorMap,
                "SectionScalarOverlayValueRange" => seis_contracts_core::views::SectionScalarOverlayValueRange,
                "SectionScalarOverlayView" => seis_contracts_core::views::SectionScalarOverlayView,
                "SectionHorizonLineStyle" => seis_contracts_core::views::SectionHorizonLineStyle,
                "SectionHorizonStyle" => seis_contracts_core::views::SectionHorizonStyle,
                "SectionHorizonSample" => seis_contracts_core::views::SectionHorizonSample,
                "SectionHorizonOverlayView" => seis_contracts_core::views::SectionHorizonOverlayView,
                "ResolvedSectionDisplayView" => seis_contracts_core::views::ResolvedSectionDisplayView,
                "GatherView" => seis_contracts_core::views::GatherView,
                "PreviewView" => seis_contracts_core::views::PreviewView,
                "ProjectSurveyMapRequestDto" => ophiolite_project::ProjectSurveyMapRequestDto,
                "GatherPreviewView" => seis_contracts_core::views::GatherPreviewView,
                "SectionViewport" => seis_contracts_core::views::SectionViewport,
                "GatherViewport" => seis_contracts_core::views::GatherViewport,
                "SectionProbe" => seis_contracts_core::views::SectionProbe,
                "GatherProbe" => seis_contracts_core::views::GatherProbe,
                "SectionProbeChanged" => seis_contracts_core::views::SectionProbeChanged,
                "GatherProbeChanged" => seis_contracts_core::views::GatherProbeChanged,
                "SectionViewportChanged" => seis_contracts_core::views::SectionViewportChanged,
                "GatherViewportChanged" => seis_contracts_core::views::GatherViewportChanged,
                "SectionInteractionChanged" => seis_contracts_core::views::SectionInteractionChanged,
                "SemblancePanel" => seis_contracts_core::SemblancePanel,
                "VelocityScanRequest" => seis_contracts_core::VelocityScanRequest,
                "VelocityScanResponse" => seis_contracts_core::VelocityScanResponse,
                "SegyHeaderValueType" => seis_contracts_operations::SegyHeaderValueType,
                "SegyHeaderField" => seis_contracts_operations::SegyHeaderField,
                "SegyGeometryOverride" => seis_contracts_operations::SegyGeometryOverride,
                "SegyGeometryCandidate" => seis_contracts_operations::SegyGeometryCandidate,
                "SuggestedImportAction" => seis_contracts_operations::SuggestedImportAction,
                "DatasetSummary" => seis_contracts_operations::DatasetSummary,
                "SurveyPreflightRequest" => seis_contracts_operations::SurveyPreflightRequest,
                "SurveyPreflightResponse" => seis_contracts_operations::SurveyPreflightResponse,
                "SegyImportWizardStage" => seis_contracts_operations::SegyImportWizardStage,
                "SegyImportIssueSeverity" => seis_contracts_operations::SegyImportIssueSeverity,
                "SegyImportIssueSection" => seis_contracts_operations::SegyImportIssueSection,
                "SegyImportSparseHandling" => seis_contracts_operations::SegyImportSparseHandling,
                "SegyImportPlanSource" => seis_contracts_operations::SegyImportPlanSource,
                "SegyImportRecipeScope" => seis_contracts_operations::SegyImportRecipeScope,
                "SegyImportPolicy" => seis_contracts_operations::SegyImportPolicy,
                "SegyImportSpatialPlan" => seis_contracts_operations::SegyImportSpatialPlan,
                "SegyImportProvenance" => seis_contracts_operations::SegyImportProvenance,
                "SegyImportPlan" => seis_contracts_operations::SegyImportPlan,
                "SegyImportIssue" => seis_contracts_operations::SegyImportIssue,
                "SegyImportRiskSummary" => seis_contracts_operations::SegyImportRiskSummary,
                "SegyImportResolvedDataset" => seis_contracts_operations::SegyImportResolvedDataset,
                "SegyImportResolvedSpatial" => seis_contracts_operations::SegyImportResolvedSpatial,
                "SegyImportFieldObservation" => seis_contracts_operations::SegyImportFieldObservation,
                "SegyImportCandidatePlan" => seis_contracts_operations::SegyImportCandidatePlan,
                "ScanSegyImportRequest" => seis_contracts_operations::ScanSegyImportRequest,
                "SegyImportScanResponse" => seis_contracts_operations::SegyImportScanResponse,
                "ValidateSegyImportPlanRequest" => seis_contracts_operations::ValidateSegyImportPlanRequest,
                "SegyImportValidationResponse" => seis_contracts_operations::SegyImportValidationResponse,
                "ImportSegyWithPlanRequest" => seis_contracts_operations::ImportSegyWithPlanRequest,
                "ImportSegyWithPlanResponse" => seis_contracts_operations::ImportSegyWithPlanResponse,
                "SegyImportRecipe" => seis_contracts_operations::SegyImportRecipe,
                "ListSegyImportRecipesRequest" => seis_contracts_operations::ListSegyImportRecipesRequest,
                "ListSegyImportRecipesResponse" => seis_contracts_operations::ListSegyImportRecipesResponse,
                "SaveSegyImportRecipeRequest" => seis_contracts_operations::SaveSegyImportRecipeRequest,
                "SaveSegyImportRecipeResponse" => seis_contracts_operations::SaveSegyImportRecipeResponse,
                "DeleteSegyImportRecipeRequest" => seis_contracts_operations::DeleteSegyImportRecipeRequest,
                "DeleteSegyImportRecipeResponse" => seis_contracts_operations::DeleteSegyImportRecipeResponse,
                "ImportDatasetRequest" => seis_contracts_operations::ImportDatasetRequest,
                "ImportDatasetResponse" => seis_contracts_operations::ImportDatasetResponse,
                "ExportSegyRequest" => seis_contracts_operations::ExportSegyRequest,
                "ExportSegyResponse" => seis_contracts_operations::ExportSegyResponse,
                "ImportedHorizonDescriptor" => seis_contracts_core::ImportedHorizonDescriptor,
                "ImportHorizonXyzRequest" => seis_contracts_operations::ImportHorizonXyzRequest,
                "ImportHorizonXyzResponse" => seis_contracts_operations::ImportHorizonXyzResponse,
                "LoadSectionHorizonsRequest" => seis_contracts_operations::LoadSectionHorizonsRequest,
                "LoadSectionHorizonsResponse" => seis_contracts_operations::LoadSectionHorizonsResponse,
                "OpenDatasetRequest" => seis_contracts_operations::OpenDatasetRequest,
                "OpenDatasetResponse" => seis_contracts_operations::OpenDatasetResponse,
                "PreviewCommand" => seis_contracts_operations::PreviewCommand,
                "PreviewResponse" => seis_contracts_operations::PreviewResponse,
                "PreviewTraceLocalProcessingRequest" => seis_contracts_operations::PreviewTraceLocalProcessingRequest,
                "PreviewTraceLocalProcessingResponse" => seis_contracts_operations::PreviewTraceLocalProcessingResponse,
                "PreviewPostStackNeighborhoodProcessingRequest" => seis_contracts_operations::PreviewPostStackNeighborhoodProcessingRequest,
                "PreviewPostStackNeighborhoodProcessingResponse" => seis_contracts_operations::PreviewPostStackNeighborhoodProcessingResponse,
                "PreviewSubvolumeProcessingRequest" => seis_contracts_operations::PreviewSubvolumeProcessingRequest,
                "PreviewSubvolumeProcessingResponse" => seis_contracts_operations::PreviewSubvolumeProcessingResponse,
                "RunTraceLocalProcessingRequest" => seis_contracts_operations::RunTraceLocalProcessingRequest,
                "RunTraceLocalProcessingResponse" => seis_contracts_operations::RunTraceLocalProcessingResponse,
                "ProcessingBatchItemRequest" => seis_contracts_operations::processing_ops::ProcessingBatchItemRequest,
                "SubmitProcessingBatchRequest" => seis_contracts_operations::SubmitProcessingBatchRequest,
                "SubmitProcessingBatchResponse" => seis_contracts_operations::SubmitProcessingBatchResponse,
                "SubmitTraceLocalProcessingBatchRequest" => seis_contracts_operations::processing_ops::SubmitTraceLocalProcessingBatchRequest,
                "SubmitTraceLocalProcessingBatchResponse" => seis_contracts_operations::processing_ops::SubmitTraceLocalProcessingBatchResponse,
                "RunPostStackNeighborhoodProcessingRequest" => seis_contracts_operations::RunPostStackNeighborhoodProcessingRequest,
                "RunPostStackNeighborhoodProcessingResponse" => seis_contracts_operations::RunPostStackNeighborhoodProcessingResponse,
                "RunSubvolumeProcessingRequest" => seis_contracts_operations::RunSubvolumeProcessingRequest,
                "RunSubvolumeProcessingResponse" => seis_contracts_operations::RunSubvolumeProcessingResponse,
                "PreviewGatherProcessingRequest" => seis_contracts_operations::PreviewGatherProcessingRequest,
                "PreviewGatherProcessingResponse" => seis_contracts_operations::PreviewGatherProcessingResponse,
                "RunGatherProcessingRequest" => seis_contracts_operations::RunGatherProcessingRequest,
                "RunGatherProcessingResponse" => seis_contracts_operations::RunGatherProcessingResponse,
                "GetProcessingDebugPlanRequest" => seis_contracts_operations::GetProcessingDebugPlanRequest,
                "GetProcessingDebugPlanResponse" => seis_contracts_operations::GetProcessingDebugPlanResponse,
                "GetProcessingJobRequest" => seis_contracts_operations::GetProcessingJobRequest,
                "GetProcessingJobResponse" => seis_contracts_operations::GetProcessingJobResponse,
                "GetProcessingRuntimeStateRequest" => seis_contracts_operations::GetProcessingRuntimeStateRequest,
                "GetProcessingRuntimeStateResponse" => seis_contracts_operations::GetProcessingRuntimeStateResponse,
                "ListProcessingRuntimeEventsRequest" => seis_contracts_operations::ListProcessingRuntimeEventsRequest,
                "ListProcessingRuntimeEventsResponse" => seis_contracts_operations::ListProcessingRuntimeEventsResponse,
                "CancelProcessingJobRequest" => seis_contracts_operations::CancelProcessingJobRequest,
                "CancelProcessingJobResponse" => seis_contracts_operations::CancelProcessingJobResponse,
                "GetProcessingBatchRequest" => seis_contracts_operations::processing_ops::GetProcessingBatchRequest,
                "GetProcessingBatchResponse" => seis_contracts_operations::processing_ops::GetProcessingBatchResponse,
                "CancelProcessingBatchRequest" => seis_contracts_operations::processing_ops::CancelProcessingBatchRequest,
                "CancelProcessingBatchResponse" => seis_contracts_operations::processing_ops::CancelProcessingBatchResponse,
                "ListPipelinePresetsResponse" => seis_contracts_operations::ListPipelinePresetsResponse,
                "SavePipelinePresetRequest" => seis_contracts_operations::SavePipelinePresetRequest,
                "SavePipelinePresetResponse" => seis_contracts_operations::SavePipelinePresetResponse,
                "DeletePipelinePresetRequest" => seis_contracts_operations::DeletePipelinePresetRequest,
                "DeletePipelinePresetResponse" => seis_contracts_operations::DeletePipelinePresetResponse,
                "DatasetRegistryStatus" => seis_contracts_operations::DatasetRegistryStatus,
                "WorkspacePipelineEntry" => seis_contracts_operations::WorkspacePipelineEntry,
                "DatasetRegistryEntry" => seis_contracts_operations::DatasetRegistryEntry,
                "WorkspaceSession" => seis_contracts_operations::WorkspaceSession,
                "LoadWorkspaceStateResponse" => seis_contracts_operations::LoadWorkspaceStateResponse,
                "UpsertDatasetEntryRequest" => seis_contracts_operations::UpsertDatasetEntryRequest,
                "UpsertDatasetEntryResponse" => seis_contracts_operations::UpsertDatasetEntryResponse,
                "RemoveDatasetEntryRequest" => seis_contracts_operations::RemoveDatasetEntryRequest,
                "RemoveDatasetEntryResponse" => seis_contracts_operations::RemoveDatasetEntryResponse,
                "SetActiveDatasetEntryRequest" => seis_contracts_operations::SetActiveDatasetEntryRequest,
                "SetActiveDatasetEntryResponse" => seis_contracts_operations::SetActiveDatasetEntryResponse,
                "SaveWorkspaceSessionRequest" => seis_contracts_operations::SaveWorkspaceSessionRequest,
                "SaveWorkspaceSessionResponse" => seis_contracts_operations::SaveWorkspaceSessionResponse,
                "DescribeVelocityVolumeRequest" => seis_contracts_operations::DescribeVelocityVolumeRequest,
                "DescribeVelocityVolumeResponse" => seis_contracts_operations::DescribeVelocityVolumeResponse,
                "IngestVelocityVolumeRequest" => seis_contracts_operations::IngestVelocityVolumeRequest,
                "IngestVelocityVolumeResponse" => seis_contracts_operations::IngestVelocityVolumeResponse,
                "SetDatasetNativeCoordinateReferenceRequest" => seis_contracts_operations::SetDatasetNativeCoordinateReferenceRequest,
                "SetDatasetNativeCoordinateReferenceResponse" => seis_contracts_operations::SetDatasetNativeCoordinateReferenceResponse,
                "ResolvedSurveyMapSourceDto" => seis_contracts_operations::ResolvedSurveyMapSourceDto,
                "ResolveSurveyMapRequest" => seis_contracts_operations::ResolveSurveyMapRequest,
                "ResolveSurveyMapResponse" => seis_contracts_operations::ResolveSurveyMapResponse,
                "BuildSurveyTimeDepthTransformRequest" => seis_contracts_operations::BuildSurveyTimeDepthTransformRequest,
                "LayeredVelocityModel" => seis_contracts_operations::LayeredVelocityModel,
                "LayeredVelocityInterval" => seis_contracts_operations::LayeredVelocityInterval,
                "VelocityIntervalTrend" => seis_contracts_operations::VelocityIntervalTrend,
                "StratigraphicBoundaryReference" => seis_contracts_operations::StratigraphicBoundaryReference,
                "LateralInterpolationMethod" => seis_contracts_operations::LateralInterpolationMethod,
                "VerticalInterpolationMethod" => seis_contracts_operations::VerticalInterpolationMethod,
                "TimeDepthDomain" => seis_contracts_operations::TimeDepthDomain,
                "TravelTimeReference" => seis_contracts_operations::TravelTimeReference,
                "DepthReferenceKind" => seis_contracts_operations::DepthReferenceKind,
                "SurveyTimeDepthTransform3D" => seis_contracts_operations::SurveyTimeDepthTransform3D,
                "LoadVelocityModelsRequest" => seis_contracts_operations::LoadVelocityModelsRequest,
                "LoadVelocityModelsResponse" => seis_contracts_operations::LoadVelocityModelsResponse,
            }
        }
    };
}

macro_rules! wrapper_files {
    ($callback:ident, $($args:tt)*) => {
        $callback! {
            $($args)*
            {
                "BuildSurveyTimeDepthTransformRequest" => seis_contracts_operations::BuildSurveyTimeDepthTransformRequest,
                "CoordinateReferenceBindingDto" => seis_contracts_operations::CoordinateReferenceBindingDto,
                "CoordinateReferenceDto" => seis_contracts_operations::CoordinateReferenceDto,
                "CoordinateReferenceSourceDto" => seis_contracts_operations::CoordinateReferenceSourceDto,
                "DepthReferenceKind" => seis_contracts_operations::DepthReferenceKind,
                "GatherAxisKind" => seis_contracts_core::domain::GatherAxisKind,
                "GatherInteractionChanged" => seis_contracts_core::views::GatherInteractionChanged,
                "GatherPreviewView" => seis_contracts_core::views::GatherPreviewView,
                "GatherProbe" => seis_contracts_core::views::GatherProbe,
                "GatherProbeChanged" => seis_contracts_core::views::GatherProbeChanged,
                "GatherSampleDomain" => seis_contracts_core::domain::GatherSampleDomain,
                "GatherView" => seis_contracts_core::views::GatherView,
                "GatherViewport" => seis_contracts_core::views::GatherViewport,
                "GatherViewportChanged" => seis_contracts_core::views::GatherViewportChanged,
                "ImportedHorizonDescriptor" => seis_contracts_core::ImportedHorizonDescriptor,
                "LateralInterpolationMethod" => seis_contracts_operations::LateralInterpolationMethod,
                "LayeredVelocityInterval" => seis_contracts_operations::LayeredVelocityInterval,
                "LayeredVelocityModel" => seis_contracts_operations::LayeredVelocityModel,
                "PreviewView" => seis_contracts_core::views::PreviewView,
                "ProjectSurveyMapRequestDto" => ophiolite_project::ProjectSurveyMapRequestDto,
                "ProjectedPoint2Dto" => seis_contracts_operations::ProjectedPoint2Dto,
                "ProjectedPolygon2Dto" => seis_contracts_operations::ProjectedPolygon2Dto,
                "ProjectedVector2Dto" => seis_contracts_operations::ProjectedVector2Dto,
                "ResolvedSectionDisplayView" => seis_contracts_core::views::ResolvedSectionDisplayView,
                "ResolvedSurveyMapSourceDto" => seis_contracts_operations::ResolvedSurveyMapSourceDto,
                "ResolvedSurveyMapSurveyDto" => seis_contracts_operations::ResolvedSurveyMapSurveyDto,
                "ResolvedSurveyMapWellDto" => seis_contracts_operations::ResolvedSurveyMapWellDto,
                "SectionColorMap" => seis_contracts_core::views::SectionColorMap,
                "SectionCoordinate" => seis_contracts_core::views::SectionCoordinate,
                "SectionDisplayDefaults" => seis_contracts_core::views::SectionDisplayDefaults,
                "SectionHorizonLineStyle" => seis_contracts_core::views::SectionHorizonLineStyle,
                "SectionHorizonOverlayView" => seis_contracts_core::views::SectionHorizonOverlayView,
                "SectionHorizonSample" => seis_contracts_core::views::SectionHorizonSample,
                "SectionHorizonStyle" => seis_contracts_core::views::SectionHorizonStyle,
                "SectionInteractionChanged" => seis_contracts_core::views::SectionInteractionChanged,
                "SectionMetadata" => seis_contracts_core::views::SectionMetadata,
                "SectionPolarity" => seis_contracts_core::views::SectionPolarity,
                "SectionPrimaryMode" => seis_contracts_core::views::SectionPrimaryMode,
                "SectionProbe" => seis_contracts_core::views::SectionProbe,
                "SectionProbeChanged" => seis_contracts_core::views::SectionProbeChanged,
                "SectionRenderMode" => seis_contracts_core::views::SectionRenderMode,
                "SectionScalarOverlayColorMap" => seis_contracts_core::views::SectionScalarOverlayColorMap,
                "SectionScalarOverlayValueRange" => seis_contracts_core::views::SectionScalarOverlayValueRange,
                "SectionScalarOverlayView" => seis_contracts_core::views::SectionScalarOverlayView,
                "SectionTimeDepthDiagnostics" => seis_contracts_core::views::SectionTimeDepthDiagnostics,
                "SectionTimeDepthTransformMode" => seis_contracts_core::views::SectionTimeDepthTransformMode,
                "SectionUnits" => seis_contracts_core::views::SectionUnits,
                "SectionView" => seis_contracts_core::views::SectionView,
                "SectionViewport" => seis_contracts_core::views::SectionViewport,
                "SectionViewportChanged" => seis_contracts_core::views::SectionViewportChanged,
                "StratigraphicBoundaryReference" => seis_contracts_operations::StratigraphicBoundaryReference,
                "SurveyIndexAxisDto" => seis_contracts_operations::SurveyIndexAxisDto,
                "SurveyIndexGridDto" => seis_contracts_operations::SurveyIndexGridDto,
                "SurveyMapGridTransformDto" => seis_contracts_operations::SurveyMapGridTransformDto,
                "SurveyMapSpatialAvailabilityDto" => seis_contracts_operations::SurveyMapSpatialAvailabilityDto,
                "SurveyMapSpatialDescriptorDto" => seis_contracts_operations::SurveyMapSpatialDescriptorDto,
                "SurveyMapTrajectoryDto" => seis_contracts_operations::SurveyMapTrajectoryDto,
                "SurveyMapTrajectoryStationDto" => seis_contracts_operations::SurveyMapTrajectoryStationDto,
                "SurveyTimeDepthTransform3D" => seis_contracts_operations::SurveyTimeDepthTransform3D,
                "TimeDepthDomain" => seis_contracts_operations::TimeDepthDomain,
                "TravelTimeReference" => seis_contracts_operations::TravelTimeReference,
                "VelocityIntervalTrend" => seis_contracts_operations::VelocityIntervalTrend,
                "VelocityQuantityKind" => seis_contracts_core::VelocityQuantityKind,
                "VerticalInterpolationMethod" => seis_contracts_operations::VerticalInterpolationMethod,
            }
        }
    };
}

macro_rules! compatibility_wrapper_contracts {
    ($callback:ident, $($args:tt)*) => {
        $callback! {
            $($args)*
            {
                "OperatorAvailability" => ophiolite_project::OperatorAvailability,
                "OperatorCatalog" => ophiolite_project::OperatorCatalog,
                "OperatorCatalogEntry" => ophiolite_project::OperatorCatalogEntry,
                "OperatorCatalogOutputLifecycle" => ophiolite_project::OperatorCatalogOutputLifecycle,
                "OperatorCatalogStability" => ophiolite_project::OperatorCatalogStability,
                "OperatorContractRef" => ophiolite_project::OperatorContractRef,
                "OperatorDetail" => ophiolite_project::OperatorDetail,
                "OperatorDocumentation" => ophiolite_project::OperatorDocumentation,
                "OperatorExecutionKind" => ophiolite_project::OperatorExecutionKind,
                "OperatorFamily" => ophiolite_project::OperatorFamily,
                "OperatorParameterDoc" => ophiolite_project::OperatorParameterDoc,
                "OperatorSubjectKind" => ophiolite_project::OperatorSubjectKind,
            }
        }
    };
}

macro_rules! export_types {
    ($output_dir:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( export_all_to::<$ty>($output_dir)?; )*
    }};
}

macro_rules! write_index_lines {
    ($buffer:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( $buffer.push_str(&format!("export type {{ {0} }} from \"./{0}\";\n", $name)); )*
    }};
}

macro_rules! insert_schema_entries {
    ($types:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( $types.insert($name.to_string(), serde_json::to_value(schema_for!($ty))?); )*
    }};
}

macro_rules! write_wrapper_files {
    ($output_dir:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( write_wrapper_file($output_dir, $name)?; )*
    }};
}

fn export_all_to<T>(output_dir: &Path) -> Result<(), ExportError>
where
    T: TS + 'static,
{
    let config = Config::default().with_out_dir(output_dir);
    T::export_all(&config)
}

fn main() -> Result<(), Box<dyn Error>> {
    let check_mode = std::env::args().skip(1).any(|arg| arg == "--check");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("scripts/traceboost-contracts-export should live two levels under repo root")
        .to_path_buf();

    let package_root = repo_root
        .join("traceboost")
        .join("contracts")
        .join("ts")
        .join("seis-contracts");
    if check_mode {
        let temp_root = std::env::temp_dir().join(format!(
            "traceboost-contracts-export-check-{}",
            std::process::id()
        ));
        if temp_root.exists() {
            fs::remove_dir_all(&temp_root)?;
        }
        let generated_dir = temp_root.join("src").join("generated");
        let schema_dir = temp_root.join("schemas");
        export_contracts(&generated_dir, &schema_dir)?;
        ensure_generated_outputs_match(
            &package_root.join("src").join("generated"),
            &generated_dir,
            "generated TypeScript contracts",
        )?;
        ensure_generated_outputs_match(
            &package_root.join("schemas"),
            &schema_dir,
            "generated schema bundle",
        )?;
        fs::remove_dir_all(&temp_root)?;
    } else {
        let generated_dir = package_root.join("src").join("generated");
        let schema_dir = package_root.join("schemas");
        export_contracts(&generated_dir, &schema_dir)?;
    }

    Ok(())
}

fn export_contracts(generated_dir: &Path, schema_dir: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(generated_dir)?;
    fs::create_dir_all(schema_dir)?;
    clear_generated_ts(generated_dir)?;
    export_ts_types(generated_dir)?;
    write_generated_index(generated_dir)?;
    write_schema_bundle(schema_dir)?;
    Ok(())
}

fn clear_generated_ts(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("ts") {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn export_ts_types(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    public_contracts!(export_types, output_dir,);
    wrapper_files!(write_wrapper_files, output_dir,);
    compatibility_wrapper_contracts!(write_wrapper_files, output_dir,);

    fs::write(
        output_dir.join("ipc-schema-version.ts"),
        format!(
            "// Generated by `cargo run -p traceboost-contracts-export`\nexport const IPC_SCHEMA_VERSION = {} as const;\n",
            seis_contracts_operations::IPC_SCHEMA_VERSION
        ),
    )?;

    Ok(())
}

fn write_wrapper_file(output_dir: &Path, type_name: &str) -> Result<(), Box<dyn Error>> {
    fs::write(
        output_dir.join(format!("{type_name}.ts")),
        format!(
            "// Generated by `cargo run -p traceboost-contracts-export`\nexport type {{ {type_name} }} from \"@ophiolite/contracts\";\n"
        ),
    )?;

    Ok(())
}

fn write_generated_index(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut index = String::from("// Generated by `cargo run -p traceboost-contracts-export`\n");
    public_contracts!(write_index_lines, index,);
    compatibility_wrapper_contracts!(write_index_lines, index,);
    index.push_str("export { IPC_SCHEMA_VERSION } from \"./ipc-schema-version\";\n");
    fs::write(output_dir.join("index.ts"), index)?;
    Ok(())
}

fn write_schema_bundle(schema_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut types = BTreeMap::new();
    public_contracts!(insert_schema_entries, types,);
    compatibility_wrapper_contracts!(insert_schema_entries, types,);

    let schema = serde_json::json!({
        "ipcSchemaVersion": seis_contracts_operations::IPC_SCHEMA_VERSION,
        "types": types,
    });

    fs::write(
        schema_dir.join("seis-contracts.schema.json"),
        serde_json::to_string_pretty(&schema)?,
    )?;

    Ok(())
}

fn ensure_generated_outputs_match(
    expected_dir: &Path,
    actual_dir: &Path,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let expected = directory_snapshot(expected_dir)?;
    let actual = directory_snapshot(actual_dir)?;
    if expected != actual {
        return Err(
            format!("{label} are stale; run `cargo run -p traceboost-contracts-export`").into(),
        );
    }
    Ok(())
}

fn directory_snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let mut snapshot = BTreeMap::new();
    collect_directory_snapshot(root, root, &mut snapshot)?;
    Ok(snapshot)
}

fn collect_directory_snapshot(
    root: &Path,
    current: &Path,
    snapshot: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_string());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_directory_snapshot(root, &path, snapshot)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        snapshot.insert(relative, fs::read(path)?);
    }
    Ok(())
}
