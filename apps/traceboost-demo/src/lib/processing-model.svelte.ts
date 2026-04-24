import { createContext, tick } from "svelte";
import type { SeismicSectionAnalysisSelectionMode } from "@ophiolite/charts";
import type {
  AmplitudeSpectrumRequest,
  AmplitudeSpectrumResponse,
  InspectableProcessingPlan,
  LocalVolumeStatistic,
  NeighborhoodDipOutput,
  PostStackNeighborhoodProcessingOperation,
  PostStackNeighborhoodProcessingPipeline,
  PostStackNeighborhoodWindow,
  PreviewPostStackNeighborhoodProcessingRequest,
  PreviewTraceLocalProcessingRequest as PreviewProcessingRequest,
  ProcessingExecutionMode,
  ProcessingBatchStatus,
  ProcessingJobRuntimeState,
  ProcessingPreset,
  ProcessingPipelineFamily,
  ProcessingPipelineSpec,
  ProcessingJobStatus,
  ProcessingRuntimeEvent,
  RunPostStackNeighborhoodProcessingRequest,
  SubvolumeCropOperation,
  SubvolumeProcessingPipeline,
  TraceLocalProcessingOperation as ProcessingOperation,
  TraceLocalProcessingPipeline as ProcessingPipeline,
  TraceLocalProcessingStep as ProcessingStep,
  RunTraceLocalProcessingRequest as RunProcessingRequest,
  RunSubvolumeProcessingRequest,
  SectionView,
  WorkspacePipelineEntry
} from "@traceboost/seis-contracts";
import {
  SCHEMA_VERSION,
  cancelProcessingJob,
  cancelProcessingBatch,
  deletePipelinePreset,
  emitFrontendDiagnosticsEvent,
  fetchAmplitudeSpectrum,
  getProcessingDebugPlan,
  getProcessingBatch,
  getProcessingJob,
  getProcessingRuntimeState,
  listProcessingRuntimeEvents,
  listPipelinePresets,
  previewPostStackNeighborhoodProcessing,
  previewProcessing,
  resolveProcessingAuthoringPalette,
  resolveProcessingRunOutput,
  runPostStackNeighborhoodProcessing,
  runProcessing,
  runSubvolumeProcessing,
  savePipelinePreset,
  submitProcessingBatch,
  type ProcessingAuthoringPaletteItem,
  type ResolveProcessingAuthoringPaletteResponse,
  type TransportSectionView
} from "./bridge";
import { confirmOverwriteStore, pickOutputStorePath } from "./file-dialog";
import {
  buildAnalysisSelectionKey,
  buildAnalysisSelectionSummary,
  selectionFromMode,
  toSpectrumSelection
} from "./seismic-analysis-selection";
import type { CompareCandidate, ViewerModel } from "./viewer-model.svelte";

type PreviewState = "raw" | "preview" | "stale";
type SpectrumAmplitudeScale = "db" | "linear";
type VolumeArithmeticOperator = "add" | "subtract" | "multiply" | "divide";
export type ProcessingWorkspaceFamily = "trace_local" | "post_stack_neighborhood";
type BatchExecutionModeSelection = Exclude<ProcessingExecutionMode, "custom">;
type DisplaySectionView = SectionView | TransportSectionView;
export type NeighborhoodOperation = PostStackNeighborhoodProcessingOperation;
export interface SourceSubvolumeBounds {
  inlineMin: number;
  inlineMax: number;
  xlineMin: number;
  xlineMax: number;
  zMinMs: number;
  zMaxMs: number;
  zUnits: string | null;
}
export interface BatchProcessingCandidate {
  storePath: string;
  displayName: string;
  isActive: boolean;
}
export interface ProcessingPlanSummaryView {
  overview: string;
  detail: string | null;
  stages: string[];
}
export interface ProcessingExecutionSummaryView {
  overview: string;
  detail: string | null;
  stages: string[];
}
export interface RecentProcessingJobEntry {
  kind: "job";
  job: ProcessingJobStatus;
  familyLabel: string;
  title: string;
}
export interface RecentProcessingBatchEntry {
  kind: "batch";
  batch: ProcessingBatchStatus;
  familyLabel: string;
  title: string;
}
export type RecentProcessingEntry = RecentProcessingJobEntry | RecentProcessingBatchEntry;
interface ProcessingJobPlanSummaryViewModel {
  planning_mode: string;
  stage_count: number;
  stage_labels: string[];
  expected_partition_count: number | null;
  max_active_partitions: number | null;
  stage_partition_summaries: string[];
}
interface ProcessingJobStageExecutionSummaryViewModel {
  stage_label: string;
  completed_partitions: number;
  total_partitions: number | null;
  retry_count: number;
}
interface ProcessingJobExecutionSummaryViewModel {
  completed_partitions: number;
  total_partitions: number | null;
  active_partitions: number;
  peak_active_partitions: number;
  retry_count: number;
  stages: ProcessingJobStageExecutionSummaryViewModel[];
}
export type WorkspaceOperation =
  | ProcessingOperation
  | {
      crop_subvolume: SubvolumeCropOperation;
    };
const SESSION_PIPELINE_PERSIST_DEBOUNCE_MS = 200;
const RUN_OUTPUT_PATH_REFRESH_DEBOUNCE_MS = 150;
export type OperatorCatalogId =
  | "amplitude_scalar"
  | "trace_rms_normalize"
  | "agc_rms"
  | "phase_rotation"
  | "envelope"
  | "instantaneous_phase"
  | "instantaneous_frequency"
  | "sweetness"
  | "volume_subtract"
  | "volume_add"
  | "volume_multiply"
  | "volume_divide"
  | "crop_subvolume"
  | "lowpass_filter"
  | "highpass_filter"
  | "bandpass_filter";

interface CopiedSessionPipeline {
  family: ProcessingWorkspaceFamily;
  pipeline: ProcessingPipeline;
  subvolumeCrop: SubvolumeCropOperation | null;
  postStackNeighborhoodPipeline: PostStackNeighborhoodProcessingPipeline | null;
}
export type OperatorCatalogItem = ProcessingAuthoringPaletteItem;

function operatorCatalogIdForOperation(operation: WorkspaceOperation): OperatorCatalogId {
  if (isCropSubvolume(operation)) {
    return "crop_subvolume";
  }
  if (typeof operation === "string") {
    switch (operation) {
      case "trace_rms_normalize":
        return "trace_rms_normalize";
      case "envelope":
        return "envelope";
      case "instantaneous_phase":
        return "instantaneous_phase";
      case "instantaneous_frequency":
        return "instantaneous_frequency";
      case "sweetness":
        return "sweetness";
      default:
        return "trace_rms_normalize";
    }
  }
  if ("amplitude_scalar" in operation) {
    return "amplitude_scalar";
  }
  if ("agc_rms" in operation) {
    return "agc_rms";
  }
  if ("phase_rotation" in operation) {
    return "phase_rotation";
  }
  if ("lowpass_filter" in operation) {
    return "lowpass_filter";
  }
  if ("highpass_filter" in operation) {
    return "highpass_filter";
  }
  if ("bandpass_filter" in operation) {
    return "bandpass_filter";
  }
  if ("volume_arithmetic" in operation) {
    switch (operation.volume_arithmetic.operator) {
      case "subtract":
        return "volume_subtract";
      case "add":
        return "volume_add";
      case "multiply":
        return "volume_multiply";
      case "divide":
        return "volume_divide";
    }
  }
  return "trace_rms_normalize";
}

export function findOperatorCatalogItemForOperation(
  operation: WorkspaceOperation,
  items: readonly OperatorCatalogItem[]
): OperatorCatalogItem | null {
  const operatorId = operatorCatalogIdForOperation(operation);
  return items.find((item) => item.item_id === operatorId) ?? null;
}

function workspaceOperationFromPaletteItem(item: OperatorCatalogItem): WorkspaceOperation {
  return item.insertable.kind === "subvolume_crop"
    ? { crop_subvolume: { ...item.insertable.crop } }
    : cloneOperation(item.insertable.operation);
}

function createEmptyPipeline(): ProcessingPipeline {
  return {
    schema_version: 2,
    revision: 1,
    preset_id: null,
    name: null,
    description: null,
    steps: []
  };
}

function createEmptyPostStackNeighborhoodPipeline(): PostStackNeighborhoodProcessingPipeline {
  return {
    schema_version: 1,
    revision: 1,
    preset_id: null,
    name: null,
    description: null,
    trace_local_pipeline: null,
    operations: [defaultNeighborhoodSimilarity()]
  };
}

function pipelineName(pipeline: ProcessingPipeline): string {
  return pipeline.name?.trim() || "Untitled pipeline";
}

function postStackNeighborhoodPipelineName(pipeline: PostStackNeighborhoodProcessingPipeline): string {
  return pipeline.name?.trim() || "Untitled neighborhood pipeline";
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function nextAnimationFrame(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame(() => resolve());
      return;
    }
    setTimeout(resolve, 16);
  });
}

function estimateSectionPayloadBytes(section: DisplaySectionView): number {
  return (
    section.horizontal_axis_f64le.length +
    (section.inline_axis_f64le?.length ?? 0) +
    (section.xline_axis_f64le?.length ?? 0) +
    section.sample_axis_f32le.length +
    section.amplitudes_f32le.length
  );
}

function previewOperationIds(pipeline: ProcessingPipeline): string[] {
  return pipeline.steps.map(({ operation }) => {
    if (typeof operation === "string") {
      return operation;
    }
    return Object.keys(operation)[0] ?? "unknown";
  });
}

function sectionKey(viewerModel: ViewerModel): string {
  return `${viewerModel.activeStorePath}:${viewerModel.axis}:${viewerModel.index}:${viewerModel.sectionDomain}:${viewerModel.depthVelocityKind}:${Math.round(viewerModel.depthVelocityMPerS)}`;
}

function clonePipeline(pipeline: ProcessingPipeline): ProcessingPipeline {
  return {
    schema_version: pipeline.schema_version,
    revision: pipeline.revision,
    preset_id: pipeline.preset_id,
    name: pipeline.name,
    description: pipeline.description,
    steps: pipeline.steps.map((step) => cloneStep(step))
  };
}

function clonePostStackNeighborhoodWindow(window: PostStackNeighborhoodWindow): PostStackNeighborhoodWindow {
  return {
    gate_ms: window.gate_ms,
    inline_stepout: window.inline_stepout,
    xline_stepout: window.xline_stepout
  };
}

function cloneNeighborhoodOperation(operation: NeighborhoodOperation): NeighborhoodOperation {
  return "similarity" in operation
    ? {
        similarity: {
          window: clonePostStackNeighborhoodWindow(operation.similarity.window)
        }
      }
    : "local_volume_stats" in operation
      ? {
          local_volume_stats: {
            window: clonePostStackNeighborhoodWindow(operation.local_volume_stats.window),
            statistic: operation.local_volume_stats.statistic
          }
        }
      : "dip" in operation
        ? {
            dip: {
              window: clonePostStackNeighborhoodWindow(operation.dip.window),
              output: operation.dip.output
            }
          }
      : operation;
}

function clonePostStackNeighborhoodPipeline(
  pipeline: PostStackNeighborhoodProcessingPipeline
): PostStackNeighborhoodProcessingPipeline {
  return {
    schema_version: pipeline.schema_version,
    revision: pipeline.revision,
    preset_id: pipeline.preset_id,
    name: pipeline.name,
    description: pipeline.description,
    trace_local_pipeline: pipeline.trace_local_pipeline
      ? clonePipeline(pipeline.trace_local_pipeline)
      : null,
    operations: pipeline.operations.map((operation) => cloneNeighborhoodOperation(operation))
  };
}

function cloneStep(step: ProcessingStep): ProcessingStep {
  return {
    operation: cloneOperation(step.operation),
    checkpoint: step.checkpoint
  };
}

function createStep(operation: ProcessingOperation, checkpoint = false): ProcessingStep {
  return {
    operation: cloneOperation(operation),
    checkpoint
  };
}

function cloneOperation(operation: ProcessingOperation): ProcessingOperation {
  if (typeof operation === "string") {
    return operation;
  }
  if ("amplitude_scalar" in operation) {
    return { amplitude_scalar: { ...operation.amplitude_scalar } };
  }
  if ("agc_rms" in operation) {
    return { agc_rms: { ...operation.agc_rms } };
  }
  if ("phase_rotation" in operation) {
    return { phase_rotation: { ...operation.phase_rotation } };
  }
  if ("lowpass_filter" in operation) {
    return { lowpass_filter: { ...operation.lowpass_filter } };
  }
  if ("highpass_filter" in operation) {
    return { highpass_filter: { ...operation.highpass_filter } };
  }
  if ("volume_arithmetic" in operation) {
    return { volume_arithmetic: { ...operation.volume_arithmetic } };
  }
  return {
    bandpass_filter: {
      ...operation.bandpass_filter
    }
  };
}

function cloneSubvolumeCrop(crop: SubvolumeCropOperation | null | undefined): SubvolumeCropOperation | null {
  return crop ? { ...crop } : null;
}

function cloneWorkspacePipelineEntry(entry: WorkspacePipelineEntry): WorkspacePipelineEntry {
  return {
    pipeline_id: entry.pipeline_id,
    family: entry.family,
    pipeline: entry.pipeline ? clonePipeline(entry.pipeline) : null,
    subvolume_crop: cloneSubvolumeCrop(entry.subvolume_crop),
    post_stack_neighborhood_pipeline: entry.post_stack_neighborhood_pipeline
      ? clonePostStackNeighborhoodPipeline(entry.post_stack_neighborhood_pipeline)
      : null,
    updated_at_unix_s: entry.updated_at_unix_s
  };
}

function createFallbackSessionPipelineEntry(
  family: ProcessingWorkspaceFamily
): WorkspacePipelineEntry {
  return {
    pipeline_id: `session-pipeline-fallback-${family}`,
    family: family === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local",
    pipeline:
      family === "post_stack_neighborhood"
        ? null
        : {
            ...createEmptyPipeline(),
            name: "Pipeline"
          },
    subvolume_crop: null,
    post_stack_neighborhood_pipeline:
      family === "post_stack_neighborhood"
        ? {
            ...createEmptyPostStackNeighborhoodPipeline(),
            name: "Neighborhood"
          }
        : null,
    updated_at_unix_s: pipelineTimestamp()
  };
}

function cloneWorkspaceOperation(operation: WorkspaceOperation): WorkspaceOperation {
  return isCropSubvolume(operation) ? { crop_subvolume: { ...operation.crop_subvolume } } : cloneOperation(operation);
}

function canCheckpointStepIndex(
  pipeline: ProcessingPipeline,
  index: number,
  subvolumeCrop: SubvolumeCropOperation | null
): boolean {
  return index >= 0 && index < pipeline.steps.length && (index < pipeline.steps.length - 1 || subvolumeCrop !== null);
}

function checkpointAfterOperationIndexes(
  pipeline: ProcessingPipeline,
  subvolumeCrop: SubvolumeCropOperation | null
): number[] {
  return pipeline.steps.flatMap((step, index) => (step.checkpoint && canCheckpointStepIndex(pipeline, index, subvolumeCrop) ? [index] : []));
}

function normalizePresetId(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function parsePositiveInteger(value: string): number | null {
  const normalized = value.trim();
  if (!normalized) {
    return null;
  }
  const parsed = Number.parseInt(normalized, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

function isExistingOutputStoreError(message: string): boolean {
  return message.toLowerCase().includes("output processing store already exists:");
}

function pipelineTimestamp(): number {
  return Math.floor(Date.now() / 1000);
}

function pipelineRunOutputSignature(pipeline: ProcessingPipeline): string {
  return JSON.stringify({
    name: pipeline.name ?? null,
    operations: pipeline.steps.map(({ operation }) =>
      typeof operation === "string"
        ? operation
        : "amplitude_scalar" in operation
          ? { amplitude_scalar: { factor: operation.amplitude_scalar.factor } }
          : "agc_rms" in operation
            ? { agc_rms: { window_ms: operation.agc_rms.window_ms } }
          : "phase_rotation" in operation
            ? {
                phase_rotation: {
                  angle_degrees: operation.phase_rotation.angle_degrees
                }
              }
            : "lowpass_filter" in operation
              ? {
                  lowpass_filter: {
                    f3_hz: operation.lowpass_filter.f3_hz,
                    f4_hz: operation.lowpass_filter.f4_hz,
                    phase: operation.lowpass_filter.phase,
                    window: operation.lowpass_filter.window
                  }
                }
              : "highpass_filter" in operation
              ? {
                  highpass_filter: {
                    f1_hz: operation.highpass_filter.f1_hz,
                    f2_hz: operation.highpass_filter.f2_hz,
                    phase: operation.highpass_filter.phase,
                    window: operation.highpass_filter.window
                  }
                }
                : "volume_arithmetic" in operation
                  ? {
                      volume_arithmetic: {
                        operator: operation.volume_arithmetic.operator,
                        secondary_store_path: operation.volume_arithmetic.secondary_store_path
                      }
                    }
                : {
              bandpass_filter: {
                f1_hz: operation.bandpass_filter.f1_hz,
                f2_hz: operation.bandpass_filter.f2_hz,
                f3_hz: operation.bandpass_filter.f3_hz,
                f4_hz: operation.bandpass_filter.f4_hz,
                phase: operation.bandpass_filter.phase,
                window: operation.bandpass_filter.window
              }
            }
    )
  });
}

function workspaceRunOutputSignature(
  pipeline: ProcessingPipeline,
  subvolumeCrop: SubvolumeCropOperation | null
): string {
  return JSON.stringify({
    pipeline: JSON.parse(pipelineRunOutputSignature(pipeline)),
    subvolume_crop: subvolumeCrop
  });
}

function postStackNeighborhoodPipelineRunOutputSignature(
  pipeline: PostStackNeighborhoodProcessingPipeline
): string {
  return JSON.stringify({
    name: pipeline.name ?? null,
    trace_local_pipeline: pipeline.trace_local_pipeline
      ? JSON.parse(pipelineRunOutputSignature(pipeline.trace_local_pipeline))
      : null,
    operations: pipeline.operations.map((operation) =>
      "similarity" in operation
        ? {
            similarity: {
              window: clonePostStackNeighborhoodWindow(operation.similarity.window)
            }
          }
        : "local_volume_stats" in operation
          ? {
              local_volume_stats: {
                window: clonePostStackNeighborhoodWindow(operation.local_volume_stats.window),
                statistic: operation.local_volume_stats.statistic
              }
            }
          : "dip" in operation
            ? {
                dip: {
                  window: clonePostStackNeighborhoodWindow(operation.dip.window),
                  output: operation.dip.output
                }
              }
          : operation
    )
  });
}

function defaultNeighborhoodSimilarity(): NeighborhoodOperation {
  return {
    similarity: {
      window: {
        gate_ms: 24,
        inline_stepout: 1,
        xline_stepout: 1
      }
    }
  };
}

function defaultNeighborhoodLocalVolumeStats(
  window: PostStackNeighborhoodWindow = {
    gate_ms: 24,
    inline_stepout: 1,
    xline_stepout: 1
  }
): NeighborhoodOperation {
  return {
    local_volume_stats: {
      window: clonePostStackNeighborhoodWindow(window),
      statistic: "mean"
    }
  };
}

function defaultNeighborhoodDip(
  window: PostStackNeighborhoodWindow = {
    gate_ms: 24,
    inline_stepout: 1,
    xline_stepout: 1
  }
): NeighborhoodOperation {
  return {
    dip: {
      window: clonePostStackNeighborhoodWindow(window),
      output: "inline"
    }
  };
}

function neighborhoodWindowForOperation(operation: NeighborhoodOperation): PostStackNeighborhoodWindow {
  if ("similarity" in operation) {
    return clonePostStackNeighborhoodWindow(operation.similarity.window);
  }
  if ("local_volume_stats" in operation) {
    return clonePostStackNeighborhoodWindow(operation.local_volume_stats.window);
  }
  if ("dip" in operation) {
    return clonePostStackNeighborhoodWindow(operation.dip.window);
  }
  return {
    gate_ms: 24,
    inline_stepout: 1,
    xline_stepout: 1
  };
}

function volumeStoreLabel(storePath: string): string {
  const normalizedPath = storePath.trim();
  const separatorIndex = Math.max(normalizedPath.lastIndexOf("/"), normalizedPath.lastIndexOf("\\"));
  const filename = separatorIndex >= 0 ? normalizedPath.slice(separatorIndex + 1) : normalizedPath;
  return filename.replace(/\.[^.]+$/, "") || "volume";
}

function volumeArithmeticSecondaryOptions(viewerModel: ViewerModel): { storePath: string; label: string }[] {
  const primaryChunkShape = viewerModel.dataset?.descriptor.chunk_shape ?? null;
  return viewerModel.compatibleSecondaryCompareCandidates
    .filter((candidate) => {
      if (!primaryChunkShape) {
        return true;
      }
      const entry = viewerModel.workspaceEntries.find((workspaceEntry) => workspaceEntry.entry_id === candidate.entryId);
      const secondaryChunkShape = entry?.last_dataset?.descriptor.chunk_shape ?? null;
      return !!secondaryChunkShape && secondaryChunkShape.every((value, index) => value === primaryChunkShape[index]);
    })
    .map((candidate) => ({
      storePath: candidate.storePath,
      label: candidate.displayName || volumeStoreLabel(candidate.storePath)
    }));
}

function buildSubvolumeProcessingPipeline(
  pipeline: ProcessingPipeline,
  crop: SubvolumeCropOperation
): SubvolumeProcessingPipeline {
  return {
    schema_version: pipeline.schema_version,
    revision: pipeline.revision,
    preset_id: pipeline.preset_id,
    name: pipeline.name,
    description: pipeline.description,
    trace_local_pipeline: pipeline.steps.length ? clonePipeline(pipeline) : null,
    crop: { ...crop }
  };
}

function workspaceOperations(
  pipeline: ProcessingPipeline,
  subvolumeCrop: SubvolumeCropOperation | null
): WorkspaceOperation[] {
  const operations: WorkspaceOperation[] = pipeline.steps.map(({ operation }) => cloneOperation(operation));
  if (subvolumeCrop) {
    operations.push({ crop_subvolume: { ...subvolumeCrop } });
  }
  return operations;
}

function batchPipelineSpecForWorkspace(
  family: ProcessingWorkspaceFamily,
  pipeline: ProcessingPipeline,
  postStackNeighborhoodPipeline: PostStackNeighborhoodProcessingPipeline,
  subvolumeCrop: SubvolumeCropOperation | null
): ProcessingPipelineSpec {
  if (family === "post_stack_neighborhood") {
    return {
      post_stack_neighborhood: {
        pipeline: clonePostStackNeighborhoodPipeline(postStackNeighborhoodPipeline)
      }
    };
  }
  if (subvolumeCrop) {
    return {
      subvolume: {
        pipeline: buildSubvolumeProcessingPipeline(pipeline, subvolumeCrop)
      }
    };
  }
  return {
    trace_local: {
      pipeline: clonePipeline(pipeline)
    }
  };
}

function processingPipelineSpecFamily(pipeline: ProcessingPipelineSpec): ProcessingPipelineFamily {
  if ("trace_local" in pipeline) {
    return "trace_local";
  }
  if ("post_stack_neighborhood" in pipeline) {
    return "post_stack_neighborhood";
  }
  if ("subvolume" in pipeline) {
    return "subvolume";
  }
  return "gather";
}

function workspaceEntryPresetId(entry: WorkspacePipelineEntry | null | undefined): string | null {
  if (!entry) {
    return null;
  }
  if (entry.family === "post_stack_neighborhood") {
    return entry.post_stack_neighborhood_pipeline?.preset_id ?? null;
  }
  return entry.pipeline?.preset_id ?? null;
}

function withPresetIdOnPipelineSpec(pipeline: ProcessingPipelineSpec, presetId: string): ProcessingPipelineSpec {
  const next = structuredClone(pipeline) as ProcessingPipelineSpec;
  if ("trace_local" in next) {
    next.trace_local.pipeline.preset_id = presetId;
    return next;
  }
  if ("post_stack_neighborhood" in next) {
    next.post_stack_neighborhood.pipeline.preset_id = presetId;
    if (next.post_stack_neighborhood.pipeline.trace_local_pipeline) {
      next.post_stack_neighborhood.pipeline.trace_local_pipeline.preset_id = presetId;
    }
    return next;
  }
  if ("subvolume" in next) {
    next.subvolume.pipeline.preset_id = presetId;
    if (next.subvolume.pipeline.trace_local_pipeline) {
      next.subvolume.pipeline.trace_local_pipeline.preset_id = presetId;
    }
    return next;
  }
  next.gather.pipeline.preset_id = presetId;
  if (next.gather.pipeline.trace_local_pipeline) {
    next.gather.pipeline.trace_local_pipeline.preset_id = presetId;
  }
  return next;
}

function batchPipelineFamilyLabel(
  family: ProcessingWorkspaceFamily,
  subvolumeCrop: SubvolumeCropOperation | null
): string {
  if (family === "post_stack_neighborhood") {
    return "post-stack neighborhood";
  }
  if (subvolumeCrop) {
    return "subvolume";
  }
  return "trace-local";
}

function processingPipelineFamilyLabel(pipeline: ProcessingPipelineSpec): string {
  if ("trace_local" in pipeline) {
    return "trace-local";
  }
  if ("subvolume" in pipeline) {
    return "subvolume";
  }
  if ("post_stack_neighborhood" in pipeline) {
    return "post-stack neighborhood";
  }
  return "gather";
}

function summarizeStorePathLabel(path: string | null | undefined): string | null {
  const normalized = path?.trim();
  if (!normalized) {
    return null;
  }
  const parts = normalized.replace(/\\/g, "/").split("/").filter((part) => part.length > 0);
  return parts.at(-1) ?? normalized;
}

function recentJobTitle(job: ProcessingJobStatus): string {
  return `${processingPipelineFamilyLabel(job.pipeline)} · ${summarizeStorePathLabel(job.input_store_path) ?? job.job_id}`;
}

function recentBatchTitle(batch: ProcessingBatchStatus): string {
  return `${processingPipelineFamilyLabel(batch.pipeline)} batch · ${batch.progress.total_jobs} datasets`;
}

function isActiveJobState(state: ProcessingJobStatus["state"]): boolean {
  return state === "queued" || state === "running";
}

function isActiveBatchState(state: ProcessingBatchStatus["state"]): boolean {
  return state === "queued" || state === "running";
}

function workspaceOperationAt(
  pipeline: ProcessingPipeline,
  subvolumeCrop: SubvolumeCropOperation | null,
  index: number
): WorkspaceOperation | null {
  if (index < pipeline.steps.length) {
    return pipeline.steps[index]?.operation ?? null;
  }
  if (subvolumeCrop && index === pipeline.steps.length) {
    return { crop_subvolume: { ...subvolumeCrop } };
  }
  return null;
}

export interface ProcessingModelOptions {
  viewerModel: ViewerModel;
}

export class ProcessingModel {
  readonly viewerModel: ViewerModel;

  pipelineFamily = $state<ProcessingWorkspaceFamily>("trace_local");
  pipeline = $state<ProcessingPipeline>(createEmptyPipeline());
  postStackNeighborhoodPipeline = $state<PostStackNeighborhoodProcessingPipeline>(
    createEmptyPostStackNeighborhoodPipeline()
  );
  subvolumeCrop = $state<SubvolumeCropOperation | null>(null);
  sessionPipelines = $state.raw<WorkspacePipelineEntry[]>([]);
  activeSessionPipelineId = $state<string | null>(null);
  selectedStepIndex = $state(0);
  editingParams = $state(false);
  previewState = $state<PreviewState>("raw");
  previewSection = $state.raw<DisplaySectionView | null>(null);
  previewLabel = $state<string | null>(null);
  previewedSectionKey = $state<string | null>(null);
  previewBusy = $state(false);
  spectrumInspectorOpen = $state(false);
  spectrumSelectionMode = $state<SeismicSectionAnalysisSelectionMode>("whole-section");
  spectrumAmplitudeScale = $state<SpectrumAmplitudeScale>("db");
  spectrumBusy = $state(false);
  spectrumStale = $state(false);
  spectrumError = $state<string | null>(null);
  rawSpectrum = $state.raw<AmplitudeSpectrumResponse | null>(null);
  processedSpectrum = $state.raw<AmplitudeSpectrumResponse | null>(null);
  spectrumSectionKey = $state<string | null>(null);
  spectrumSelectionKey = $state<string | null>(null);
  runBusy = $state(false);
  batchSubmitting = $state(false);
  error = $state<string | null>(null);
  authoringPalette = $state.raw<ResolveProcessingAuthoringPaletteResponse | null>(null);
  authoringPaletteLoading = $state(false);
  authoringPaletteError = $state<string | null>(null);
  presets = $state.raw<ProcessingPreset[]>([]);
  activeJob = $state<ProcessingJobStatus | null>(null);
  activeDebugPlan = $state.raw<InspectableProcessingPlan | null>(null);
  activeRuntimeState = $state.raw<ProcessingJobRuntimeState | null>(null);
  activeRuntimeEvents = $state.raw<ProcessingRuntimeEvent[]>([]);
  activeBatch = $state.raw<ProcessingBatchStatus | null>(null);
  recentJobs = $state.raw<RecentProcessingJobEntry[]>([]);
  recentBatches = $state.raw<RecentProcessingBatchEntry[]>([]);
  selectedBatchStorePaths = $state.raw<string[]>([]);
  batchExecutionMode = $state<BatchExecutionModeSelection>("auto");
  batchMaxActiveJobs = $state("");
  loadingPresets = $state(false);
  runOutputSettingsOpen = $state(false);
  runOutputPathMode = $state<"default" | "custom">("default");
  customRunOutputPath = $state("");
  overwriteExistingRunOutput = $state(false);
  defaultRunOutputPath = $state<string | null>(null);
  resolvingRunOutputPath = $state(false);

  #jobPollTimer: number | null = null;
  #batchPollTimer: number | null = null;
  #presetCounter = 0;
  #hydratedDatasetEntryId: string | null = null;
  #runOutputPathRequestId = 0;
  #copiedSessionPipeline: CopiedSessionPipeline | null = null;
  #copiedOperation: WorkspaceOperation | null = null;
  #persistSessionPipelinesTimer: number | null = null;
  #runOutputPathRefreshTimer: number | null = null;
  #operatorCatalogRequestId = 0;
  #activeDebugJobId: string | null = null;
  #latestRuntimeEventSeq = 0;

  constructor(options: ProcessingModelOptions) {
    this.viewerModel = options.viewerModel;

    $effect(() => {
      const key = sectionKey(this.viewerModel);
      const currentSection = this.viewerModel.section;
      const activeStorePath = this.viewerModel.activeStorePath;
      if (!activeStorePath || !currentSection) {
        this.previewSection = null;
        this.previewState = "raw";
        this.previewedSectionKey = null;
        this.spectrumInspectorOpen = false;
        this.clearSpectrumState();
        return;
      }

      if (this.previewedSectionKey && this.previewedSectionKey !== key) {
        this.previewState = "stale";
      }
      if (this.spectrumSectionKey && this.spectrumSectionKey !== key) {
        this.clearSpectrumState();
        return;
      }

      if (
        this.spectrumSelectionKey &&
        this.activeSpectrumSelection &&
        this.spectrumSelectionKey !== buildAnalysisSelectionKey(key, this.activeSpectrumSelection)
      ) {
        this.spectrumStale = true;
        this.spectrumError = null;
      }
    });

    $effect(() => {
      const activeStorePath = this.viewerModel.activeStorePath.trim();
      const family =
        this.pipelineFamily === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local";
      const secondaryStorePaths = this.volumeArithmeticSecondaryOptions.map((option) => option.storePath);
      const secondarySignature = secondaryStorePaths.join("\n");
      void secondarySignature;

      const requestId = ++this.#operatorCatalogRequestId;
      this.authoringPaletteLoading = true;
      this.authoringPaletteError = null;

      void resolveProcessingAuthoringPalette({
        schema_version: SCHEMA_VERSION,
        store_path: activeStorePath || null,
        family,
        secondary_store_paths: secondaryStorePaths
      })
        .then((palette) => {
          if (this.#operatorCatalogRequestId !== requestId) {
            return;
          }
          this.authoringPalette = palette;
        })
        .catch((error) => {
          if (this.#operatorCatalogRequestId !== requestId) {
            return;
          }
          this.authoringPalette = null;
          this.authoringPaletteError = errorMessage(
            error,
            "Failed to resolve the processing authoring palette."
          );
        })
        .finally(() => {
          if (this.#operatorCatalogRequestId !== requestId) {
            return;
          }
          this.authoringPaletteLoading = false;
        });
    });

    $effect(() => {
      const activeEntryId = this.viewerModel.activeEntryId;
      const activeEntry = this.viewerModel.activeDatasetEntry;

      if (!activeEntryId || !activeEntry) {
        this.#hydratedDatasetEntryId = null;
        if (!this.sessionPipelines.length) {
          const fallback = createFallbackSessionPipelineEntry(this.pipelineFamily);
          this.sessionPipelines = [fallback];
          this.activeSessionPipelineId = fallback.pipeline_id;
          this.pipeline = clonePipeline(fallback.pipeline ?? createEmptyPipeline());
          this.postStackNeighborhoodPipeline = clonePostStackNeighborhoodPipeline(
            fallback.post_stack_neighborhood_pipeline ?? createEmptyPostStackNeighborhoodPipeline()
          );
          this.subvolumeCrop = cloneSubvolumeCrop(fallback.subvolume_crop);
        }
        return;
      }

      if (this.#hydratedDatasetEntryId === activeEntryId) {
        return;
      }
      this.#hydratedDatasetEntryId = activeEntryId;

      if (activeEntry.session_pipelines.length === 0) {
        void this.runSessionAction(
          {
            action: "ensure_family_pipeline",
            family: this.activeProcessingPipelineFamily
          },
          "Failed to initialize a processing session pipeline."
        );
        return;
      }

      this.applyAuthoringSessionEntry(
        activeEntry.session_pipelines,
        activeEntry.active_session_pipeline_id ?? null
      );
    });

    $effect(() => {
      const runOutputSettingsOpen = this.runOutputSettingsOpen;
      const runOutputPathMode = this.runOutputPathMode;
      const activeStorePath = this.viewerModel.activeStorePath;
      const signature =
        this.pipelineFamily === "post_stack_neighborhood"
          ? postStackNeighborhoodPipelineRunOutputSignature(this.postStackNeighborhoodPipeline)
          : workspaceRunOutputSignature(this.pipeline, this.subvolumeCrop);

      if (!activeStorePath) {
        this.defaultRunOutputPath = null;
        this.resolvingRunOutputPath = false;
        if (this.#runOutputPathRefreshTimer !== null && typeof window !== "undefined") {
          window.clearTimeout(this.#runOutputPathRefreshTimer);
          this.#runOutputPathRefreshTimer = null;
        }
        return;
      }

      if (!runOutputSettingsOpen || runOutputPathMode !== "default") {
        return;
      }

      this.scheduleDefaultRunOutputPathRefresh(
        activeStorePath,
        clonePipeline(this.pipeline),
        clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline),
        cloneSubvolumeCrop(this.subvolumeCrop),
        signature
      );
    });

    $effect(() => {
      const candidates = this.batchCandidates;
      const nextSelection = this.selectedBatchStorePaths.filter((storePath) =>
        candidates.some((candidate) => candidate.storePath === storePath)
      );
      if (
        nextSelection.length !== this.selectedBatchStorePaths.length ||
        nextSelection.some((storePath, index) => storePath !== this.selectedBatchStorePaths[index])
      ) {
        this.selectedBatchStorePaths = nextSelection;
      }
    });
  }

  mount = (): (() => void) => {
    void this.refreshPresets();
    return () => {
      if (this.#persistSessionPipelinesTimer !== null && typeof window !== "undefined") {
        window.clearTimeout(this.#persistSessionPipelinesTimer);
        this.#persistSessionPipelinesTimer = null;
        void this.persistSessionPipelinesNow();
      }
      if (this.#runOutputPathRefreshTimer !== null && typeof window !== "undefined") {
        window.clearTimeout(this.#runOutputPathRefreshTimer);
        this.#runOutputPathRefreshTimer = null;
      }
      if (this.#jobPollTimer !== null && typeof window !== "undefined") {
        window.clearTimeout(this.#jobPollTimer);
      }
      this.#jobPollTimer = null;
      if (this.#batchPollTimer !== null && typeof window !== "undefined") {
        window.clearTimeout(this.#batchPollTimer);
      }
      this.#batchPollTimer = null;
    };
  };

  get selectedOperation(): WorkspaceOperation | null {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      const prefix = this.postStackNeighborhoodPipeline.trace_local_pipeline;
      if (!prefix || this.selectedStepIndex < 0 || this.selectedStepIndex >= prefix.steps.length) {
        return null;
      }
      return prefix.steps[this.selectedStepIndex]?.operation ?? null;
    }
    return workspaceOperationAt(this.pipeline, this.subvolumeCrop, this.selectedStepIndex);
  }

  get activeSessionPipeline(): WorkspacePipelineEntry | null {
    const activeEntry =
      this.sessionPipelines.find((entry) => entry.pipeline_id === this.activeSessionPipelineId) ?? null;
    if (!activeEntry) {
      return null;
    }
    return activeEntry.family === this.activeProcessingPipelineFamily ? activeEntry : null;
  }

  get activeProcessingPipelineFamily(): ProcessingPipelineFamily {
    return this.pipelineFamily === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local";
  }

  get activeDatasetIsGatherNative(): boolean {
    return Boolean(this.viewerModel.dataset?.descriptor.geometry.summary.gather_axis_kind);
  }

  get batchCandidates(): BatchProcessingCandidate[] {
    if (this.activeDatasetIsGatherNative) {
      return [];
    }

    return this.viewerModel.compatibleCompareCandidates.map((candidate: CompareCandidate) => ({
      storePath: candidate.storePath,
      displayName: candidate.displayName,
      isActive: candidate.isPrimary
    }));
  }

  get batchBusy(): boolean {
    if (this.batchSubmitting) {
      return true;
    }
    return this.activeBatch?.state === "queued" || this.activeBatch?.state === "running";
  }

  get canRunBatch(): boolean {
    return (
      this.canRun &&
      this.batchCandidates.length > 0 &&
      this.selectedBatchStorePaths.length > 0 &&
      !this.runBusy &&
      !this.batchBusy
    );
  }

  get recentActivityEntries(): RecentProcessingEntry[] {
    return [...this.recentJobs, ...this.recentBatches].sort((left, right) => {
      const rightUpdatedAt = right.kind === "job" ? right.job.updated_at_unix_s : right.batch.updated_at_unix_s;
      const leftUpdatedAt = left.kind === "job" ? left.job.updated_at_unix_s : left.batch.updated_at_unix_s;
      return rightUpdatedAt - leftUpdatedAt;
    });
  }

  get hasClearableRecentActivity(): boolean {
    return (
      this.recentJobs.some((entry) => !isActiveJobState(entry.job.state)) ||
      this.recentBatches.some((entry) => !isActiveBatchState(entry.batch.state))
    );
  }

  get availableOperatorCatalogItems(): readonly OperatorCatalogItem[] {
    if (this.activeDatasetIsGatherNative) {
      return [];
    }

    return this.authoringPalette?.items ?? [];
  }

  get operatorCatalogSourceLabel(): string {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      return "Trace-local neighborhood prefix";
    }
    if (this.activeDatasetIsGatherNative) {
      return "Gather-native dataset";
    }
    return this.authoringPalette?.source_label ?? "Processing authoring palette";
  }

  get operatorCatalogSourceDetail(): string {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      return "These trace-local steps run before the terminal neighborhood operator. Prefix checkpoints stay hidden in v1.";
    }
    if (this.activeDatasetIsGatherNative) {
      return "This demo remains section-centric. Gather processing and velocity scans are backend-wired but not exposed here yet.";
    }
    return this.authoringPalette?.source_detail ??
      (this.authoringPaletteError ?? "Using the backend processing authoring palette.");
  }

  get operatorCatalogEmptyMessage(): string {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      return "No trace-local prefix operators are available for this dataset.";
    }
    if (this.activeDatasetIsGatherNative) {
      return "Gather-native authoring is not exposed in this section viewer yet.";
    }
    if (this.authoringPaletteLoading) {
      return "Loading processing authoring options for the active dataset...";
    }
    return this.authoringPalette?.empty_message ?? "No processing authoring options are available for this dataset.";
  }

  get selectedNeighborhoodOperation(): NeighborhoodOperation | null {
    if (this.pipelineFamily !== "post_stack_neighborhood") {
      return null;
    }
    const prefixSteps = this.postStackNeighborhoodPipeline.trace_local_pipeline?.steps.length ?? 0;
    return this.postStackNeighborhoodPipeline.operations[this.selectedStepIndex - prefixSteps] ?? null;
  }

  private traceOperationIndexForDisplayIndex(displayIndex: number): number | null {
    const traceStepCount =
      this.pipelineFamily === "post_stack_neighborhood"
        ? this.postStackNeighborhoodPipeline.trace_local_pipeline?.steps.length ?? 0
        : this.pipeline.steps.length;
    if (displayIndex < 0 || displayIndex >= traceStepCount) {
      return null;
    }
    return displayIndex;
  }

  private nextTraceInsertIndexAfterSelection(): number {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    if (selectedTraceOperationIndex === null) {
      return this.pipelineFamily === "post_stack_neighborhood"
        ? this.postStackNeighborhoodPipeline.trace_local_pipeline?.steps.length ?? 0
        : this.pipeline.steps.length;
    }
    return selectedTraceOperationIndex + 1;
  }

  private selectedTraceOperationIndex(): number | null {
    return this.traceOperationIndexForDisplayIndex(this.selectedStepIndex);
  }

  get checkpointAfterOperationIndexes(): number[] {
    return checkpointAfterOperationIndexes(this.pipeline, this.subvolumeCrop);
  }

  get checkpointWarning(): string | null {
    const checkpointCount = this.checkpointAfterOperationIndexes.length;
    return checkpointCount > 5
      ? "More than 5 checkpoints will materially increase full-volume run time."
      : null;
  }

  get sessionPipelineItems(): WorkspacePipelineEntry[] {
    return this.sessionPipelines.filter((entry) => entry.family === this.activeProcessingPipelineFamily);
  }

  get hasOperations(): boolean {
    return this.pipelineFamily === "post_stack_neighborhood"
      ? (this.postStackNeighborhoodPipeline.trace_local_pipeline?.steps.length ?? 0) +
          this.postStackNeighborhoodPipeline.operations.length >
          0
      : this.pipeline.steps.length > 0 || this.subvolumeCrop !== null;
  }

  get selectedStepLabel(): string | null {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      if (this.selectedOperation) {
        return describeOperation(this.selectedOperation);
      }
      return this.selectedNeighborhoodOperation
        ? describeNeighborhoodOperation(this.selectedNeighborhoodOperation)
        : null;
    }
    return this.selectedOperatorCatalogItem?.label ?? (this.selectedOperation ? describeOperation(this.selectedOperation) : null);
  }

  get selectedOperatorCatalogItem(): OperatorCatalogItem | null {
    return this.selectedOperation
      ? findOperatorCatalogItemForOperation(this.selectedOperation, this.availableOperatorCatalogItems)
      : null;
  }

  get displaySection(): DisplaySectionView | null {
    if (this.previewState === "preview" && this.previewSection) {
      return this.previewSection;
    }
    return this.viewerModel.section;
  }

  get displaySectionMode(): PreviewState {
    return this.previewState;
  }

  get displayResetToken(): string {
    return `${this.viewerModel.resetToken}:${this.previewState}:${this.previewedSectionKey ?? "raw"}`;
  }

  get canPreview(): boolean {
    return this.pipelineFamily === "post_stack_neighborhood"
      ? this.postStackNeighborhoodPipeline.operations.length > 0 &&
          Boolean(this.viewerModel.section && this.viewerModel.activeStorePath)
      : this.pipeline.steps.length > 0 && Boolean(this.viewerModel.section && this.viewerModel.activeStorePath);
  }

  get canRun(): boolean {
    return this.hasOperations && Boolean(this.viewerModel.activeStorePath);
  }

  get canInspectSpectrum(): boolean {
    return Boolean(
      this.viewerModel.section &&
        this.viewerModel.activeStorePath &&
        this.viewerModel.dataset &&
        this.activeSpectrumSelection
    );
  }

  get spectrumSelectionSummary(): string {
    if (!this.viewerModel.section || !this.activeSpectrumSelection) {
      return "Open a dataset and load a section to inspect spectra.";
    }

    return buildAnalysisSelectionSummary(this.viewerModel.section, this.activeSpectrumSelection);
  }

  get activeSpectrumSelection() {
    return selectionFromMode(this.spectrumSelectionMode, this.viewerModel.displayedViewport);
  }

  get pipelineDirty(): boolean {
    return this.previewState !== "preview";
  }

  get pipelineTitle(): string {
    return this.pipelineFamily === "post_stack_neighborhood"
      ? postStackNeighborhoodPipelineName(this.postStackNeighborhoodPipeline)
      : pipelineName(this.pipeline);
  }

  get activePresetFamily(): ProcessingPipelineFamily {
    return processingPipelineSpecFamily(
      batchPipelineSpecForWorkspace(
        this.pipelineFamily,
        this.pipeline,
        this.postStackNeighborhoodPipeline,
        this.subvolumeCrop
      )
    );
  }

  get visiblePresets(): ProcessingPreset[] {
    return this.presets.filter((preset) => processingPipelineSpecFamily(preset.pipeline) === this.activePresetFamily);
  }

  get activePrimaryVolumeLabel(): string {
    return this.viewerModel.activeDatasetDisplayName;
  }

  get volumeArithmeticSecondaryOptions(): { storePath: string; label: string }[] {
    return volumeArithmeticSecondaryOptions(this.viewerModel);
  }

  get sourceSubvolumeBounds(): SourceSubvolumeBounds | null {
    const summary = this.viewerModel.dataset?.descriptor.geometry?.summary;
    if (!summary) {
      return null;
    }
    return {
      inlineMin: summary.inline_axis.first,
      inlineMax: summary.inline_axis.last,
      xlineMin: summary.xline_axis.first,
      xlineMax: summary.xline_axis.last,
      zMinMs: summary.sample_axis.first,
      zMaxMs: summary.sample_axis.last,
      zUnits: summary.sample_axis.units
    };
  }

  get workspaceOperations(): WorkspaceOperation[] {
    return workspaceOperations(this.pipeline, this.subvolumeCrop);
  }

  get neighborhoodOperations(): NeighborhoodOperation[] {
    return this.postStackNeighborhoodPipeline.operations.map((operation) => cloneNeighborhoodOperation(operation));
  }

  get neighborhoodTraceLocalOperations(): WorkspaceOperation[] {
    return this.postStackNeighborhoodPipeline.trace_local_pipeline
      ? workspaceOperations(this.postStackNeighborhoodPipeline.trace_local_pipeline, null)
      : [];
  }

  get hasSubvolumeCrop(): boolean {
    return this.subvolumeCrop !== null;
  }

  get canMoveSelectedUp(): boolean {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    return selectedTraceOperationIndex !== null && selectedTraceOperationIndex > 0;
  }

  get canMoveSelectedDown(): boolean {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    const traceStepCount =
      this.pipelineFamily === "post_stack_neighborhood"
        ? this.postStackNeighborhoodPipeline.trace_local_pipeline?.steps.length ?? 0
        : this.pipeline.steps.length;
    return selectedTraceOperationIndex !== null && selectedTraceOperationIndex < traceStepCount - 1;
  }

  get canRemoveSessionPipeline(): boolean {
    return this.sessionPipelineItems.length > 1;
  }

  get canToggleSelectedCheckpoint(): boolean {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      return false;
    }
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    return selectedTraceOperationIndex !== null && canCheckpointStepIndex(this.pipeline, selectedTraceOperationIndex, this.subvolumeCrop);
  }

  get selectedStepCheckpoint(): boolean {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      return false;
    }
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    return selectedTraceOperationIndex !== null
      ? this.pipeline.steps[selectedTraceOperationIndex]?.checkpoint ?? false
      : false;
  }

  get resolvedRunOutputPath(): string | null {
    if (this.runOutputPathMode === "custom") {
      const nextPath = this.customRunOutputPath.trim();
      return nextPath.length > 0 ? nextPath : null;
    }
    return this.defaultRunOutputPath;
  }

  sessionPipelineLabel = (entry: WorkspacePipelineEntry, index: number): string => {
    if (entry.family === "post_stack_neighborhood") {
      return postStackNeighborhoodPipelineName(
        entry.post_stack_neighborhood_pipeline ?? createEmptyPostStackNeighborhoodPipeline()
      ) || `Neighborhood ${index + 1}`;
    }
    return pipelineName(entry.pipeline ?? createEmptyPipeline()) || `Pipeline ${index + 1}`;
  };

  sessionPipelineSummary = (entry: WorkspacePipelineEntry): string => {
    if (entry.family === "post_stack_neighborhood") {
      const count = entry.post_stack_neighborhood_pipeline?.operations.length ?? 0;
      return `${count} neighborhood step${count === 1 ? "" : "s"}`;
    }
    const stepCount = (entry.pipeline?.steps.length ?? 0) + (entry.subvolume_crop ? 1 : 0);
    return `${stepCount} step${stepCount === 1 ? "" : "s"}`;
  };

  private applyAuthoringSessionEntry(
    sessionPipelines: WorkspacePipelineEntry[],
    activeSessionPipelineId: string | null
  ): void {
    const nextSessionPipelines =
      sessionPipelines.length > 0
        ? sessionPipelines.map((entry) => cloneWorkspacePipelineEntry(entry))
        : [createFallbackSessionPipelineEntry(this.pipelineFamily)];
    const nextActivePipelineId =
      activeSessionPipelineId &&
      nextSessionPipelines.some((entry) => entry.pipeline_id === activeSessionPipelineId)
        ? activeSessionPipelineId
        : nextSessionPipelines[0]?.pipeline_id ?? null;
    const activePipeline =
      nextSessionPipelines.find((entry) => entry.pipeline_id === nextActivePipelineId) ??
      nextSessionPipelines[0] ??
      null;

    this.sessionPipelines = nextSessionPipelines;
    this.pipelineFamily = activePipeline?.family === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local";
    this.activeSessionPipelineId = activePipeline?.pipeline_id ?? null;
    this.pipeline = clonePipeline(activePipeline?.pipeline ?? createEmptyPipeline());
    this.postStackNeighborhoodPipeline = clonePostStackNeighborhoodPipeline(
      activePipeline?.post_stack_neighborhood_pipeline ?? createEmptyPostStackNeighborhoodPipeline()
    );
    this.subvolumeCrop = cloneSubvolumeCrop(activePipeline?.subvolume_crop);
    this.viewerModel.setSelectedPresetId(workspaceEntryPresetId(activePipeline));
    this.selectedStepIndex = 0;
    this.editingParams = false;
    this.clearPreviewState();
  }

  private async runSessionAction(
    action:
      | {
          action: "ensure_family_pipeline";
          family: ProcessingPipelineFamily;
        }
      | {
          action: "create_pipeline";
          family: ProcessingPipelineFamily;
        }
      | {
          action: "duplicate_pipeline";
          pipeline_id?: string | null;
        }
      | {
          action: "activate_pipeline";
          pipeline_id: string;
        }
      | {
          action: "remove_pipeline";
          pipeline_id: string;
        }
      | {
          action: "replace_active_from_pipeline_spec";
          pipeline: ProcessingPipelineSpec;
        },
    fallbackMessage: string
  ): Promise<boolean> {
    try {
      const response = await this.viewerModel.applyProcessingAuthoringSessionAction(action);
      this.applyAuthoringSessionEntry(
        response.entry.session_pipelines,
        response.entry.active_session_pipeline_id ?? null
      );
      return true;
    } catch (error) {
      this.error = errorMessage(error, fallbackMessage);
      this.viewerModel.note(fallbackMessage, "backend", "error", this.error);
      return false;
    }
  }

  private copiedSessionPipelineSpec(copied: CopiedSessionPipeline): ProcessingPipelineSpec {
    return batchPipelineSpecForWorkspace(
      copied.family,
      copied.pipeline,
      copied.postStackNeighborhoodPipeline ?? createEmptyPostStackNeighborhoodPipeline(),
      copied.subvolumeCrop
    );
  }

  setPipelineFamily = (family: ProcessingWorkspaceFamily): void => {
    if (this.pipelineFamily === family) {
      return;
    }
    void this.runSessionAction(
      {
        action: "ensure_family_pipeline",
        family: family === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local"
      },
      "Failed to switch the active processing workflow."
    );
  };

  setRunOutputSettingsOpen = (open: boolean): void => {
    this.runOutputSettingsOpen = open;
    if (open && this.viewerModel.activeStorePath && !this.defaultRunOutputPath && !this.resolvingRunOutputPath) {
      this.scheduleDefaultRunOutputPathRefresh(
        this.viewerModel.activeStorePath,
        clonePipeline(this.pipeline),
        clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline),
        cloneSubvolumeCrop(this.subvolumeCrop),
        this.pipelineFamily === "post_stack_neighborhood"
          ? postStackNeighborhoodPipelineRunOutputSignature(this.postStackNeighborhoodPipeline)
          : workspaceRunOutputSignature(this.pipeline, this.subvolumeCrop)
      );
    }
  };

  setRunOutputPathMode = (mode: "default" | "custom"): void => {
    this.runOutputPathMode = mode;
  };

  setCustomRunOutputPath = (value: string): void => {
    this.customRunOutputPath = value;
  };

  resetRunOutputPath = (): void => {
    this.runOutputPathMode = "default";
    this.customRunOutputPath = "";
  };

  browseRunOutputPath = async (): Promise<void> => {
    const selected = await pickOutputStorePath(this.resolvedRunOutputPath ?? this.defaultRunOutputPath ?? "processed.tbvol");
    if (!selected) {
      return;
    }
    this.runOutputPathMode = "custom";
    this.customRunOutputPath = selected;
  };

  setOverwriteExistingRunOutput = (value: boolean): void => {
    this.overwriteExistingRunOutput = value;
  };

  refreshPresets = async (): Promise<void> => {
    this.loadingPresets = true;
    try {
      const response = await listPipelinePresets();
      this.presets = response.presets;
    } catch (error) {
      this.error = errorMessage(error, "Failed to load pipeline presets.");
      this.viewerModel.note("Failed to load pipeline presets.", "backend", "error", this.error);
    } finally {
      this.loadingPresets = false;
    }
  };

  createSessionPipeline = (): void => {
    void this.runSessionAction(
      {
        action: "create_pipeline",
        family: this.activeProcessingPipelineFamily
      },
      "Failed to create a processing session pipeline."
    );
  };

  duplicateActiveSessionPipeline = (): void => {
    void this.runSessionAction(
      {
        action: "duplicate_pipeline",
        pipeline_id: this.activeSessionPipelineId
      },
      "Failed to duplicate the active processing session pipeline."
    );
  };

  activateSessionPipeline = (pipelineId: string): void => {
    void this.runSessionAction(
      {
        action: "activate_pipeline",
        pipeline_id: pipelineId
      },
      "Failed to activate the selected processing session pipeline."
    );
  };

  removeActiveSessionPipeline = (): void => {
    const activePipelineId = this.activeSessionPipelineId;
    if (!activePipelineId) {
      return;
    }

    this.removeSessionPipeline(activePipelineId);
  };

  removeSessionPipeline = (pipelineId: string): void => {
    if (this.sessionPipelines.length <= 1) {
      return;
    }
    void this.runSessionAction(
      {
        action: "remove_pipeline",
        pipeline_id: pipelineId
      },
      "Failed to remove the selected processing session pipeline."
    );
  };

  copyActiveSessionPipeline = (): void => {
    const activePipeline = this.activeSessionPipeline;
    if (!activePipeline) {
      return;
    }
    this.#copiedSessionPipeline = {
      family: activePipeline.family === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local",
      pipeline: clonePipeline(activePipeline.pipeline ?? createEmptyPipeline()),
      subvolumeCrop: cloneSubvolumeCrop(activePipeline.subvolume_crop),
      postStackNeighborhoodPipeline: activePipeline.post_stack_neighborhood_pipeline
        ? clonePostStackNeighborhoodPipeline(activePipeline.post_stack_neighborhood_pipeline)
        : null
    };
    this.viewerModel.note(
      "Copied active session pipeline.",
      "ui",
      "info",
      this.sessionPipelineLabel(activePipeline, 0)
    );
  };

  pasteCopiedSessionPipeline = (): void => {
    if (!this.#copiedSessionPipeline) {
      return;
    }
    const copied = this.#copiedSessionPipeline;
    void (async () => {
      const created = await this.runSessionAction(
        {
          action: "create_pipeline",
          family: copied.family === "post_stack_neighborhood" ? "post_stack_neighborhood" : "trace_local"
        },
        "Failed to create a session pipeline for the pasted processing workflow."
      );
      if (!created) {
        return;
      }
      const replaced = await this.runSessionAction(
        {
          action: "replace_active_from_pipeline_spec",
          pipeline: this.copiedSessionPipelineSpec(copied)
        },
        "Failed to paste the copied processing workflow."
      );
      if (replaced) {
        this.viewerModel.setSelectedPresetId(null);
      }
    })();
  };

  copySelectedOperation = (): void => {
    const selectedOperation = this.selectedOperation;
    if (!selectedOperation) {
      return;
    }

    this.#copiedOperation = cloneWorkspaceOperation(selectedOperation);
    this.viewerModel.note("Copied selected pipeline step.", "ui", "info", describeOperation(selectedOperation));
  };

  pasteCopiedOperation = (): void => {
    if (!this.#copiedOperation) {
      return;
    }

    this.insertOperation(this.#copiedOperation);
  };

  toggleCheckpointAfterOperation = (index: number): void => {
    if (!canCheckpointStepIndex(this.pipeline, index, this.subvolumeCrop)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const step = next.steps[index];
    if (!step) {
      return;
    }
    step.checkpoint = !step.checkpoint;
    next.revision += 1;
    this.updateActiveSessionPipeline(next, this.subvolumeCrop);
  };

  setSelectedCheckpoint = (value: boolean): void => {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    if (selectedTraceOperationIndex === null || !canCheckpointStepIndex(this.pipeline, selectedTraceOperationIndex, this.subvolumeCrop)) {
      return;
    }
    const next = clonePipeline(this.pipeline);
    const step = next.steps[selectedTraceOperationIndex];
    if (!step || step.checkpoint === value) {
      return;
    }
    step.checkpoint = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next, this.subvolumeCrop);
  };

  openProcessingArtifact = async (storePath: string): Promise<void> => {
    if (!storePath.trim()) {
      return;
    }
    await this.viewerModel.openDerivedDatasetAt(storePath, this.viewerModel.axis, this.viewerModel.index);
  };

  private persistSessionPipelinesNow(): Promise<void> {
    return this.viewerModel.updateActiveEntryPipelines(
      this.sessionPipelines.map((entry) => ({
        pipeline_id: entry.pipeline_id,
        updated_at_unix_s: entry.updated_at_unix_s,
        family: entry.family,
        pipeline: entry.pipeline ? clonePipeline(entry.pipeline) : null,
        subvolume_crop: cloneSubvolumeCrop(entry.subvolume_crop),
        post_stack_neighborhood_pipeline: entry.post_stack_neighborhood_pipeline
          ? clonePostStackNeighborhoodPipeline(entry.post_stack_neighborhood_pipeline)
          : null
      })),
      this.activeSessionPipelineId
    );
  }

  private schedulePersistSessionPipelines(): void {
    if (typeof window === "undefined") {
      void this.persistSessionPipelinesNow();
      return;
    }

    if (this.#persistSessionPipelinesTimer !== null) {
      window.clearTimeout(this.#persistSessionPipelinesTimer);
    }

    this.#persistSessionPipelinesTimer = window.setTimeout(() => {
      this.#persistSessionPipelinesTimer = null;
      void this.persistSessionPipelinesNow();
    }, SESSION_PIPELINE_PERSIST_DEBOUNCE_MS);
  }

  private updateActiveSessionPipeline(
    nextPipeline: ProcessingPipeline,
    nextSubvolumeCrop: SubvolumeCropOperation | null = this.subvolumeCrop
  ): void {
    if (this.pipelineFamily !== "trace_local") {
      return;
    }
    const activePipelineId = this.activeSessionPipelineId;
    const snapshot = clonePipeline(nextPipeline);
    this.pipeline = snapshot;
    this.subvolumeCrop = cloneSubvolumeCrop(nextSubvolumeCrop);

    if (!activePipelineId) {
      return;
    }

    this.sessionPipelines = this.sessionPipelines.map((entry) =>
      entry.pipeline_id === activePipelineId
        ? {
            pipeline_id: entry.pipeline_id,
            family: entry.family,
            pipeline: clonePipeline(snapshot),
            subvolume_crop: cloneSubvolumeCrop(nextSubvolumeCrop),
            post_stack_neighborhood_pipeline: null,
            updated_at_unix_s: pipelineTimestamp()
          }
        : entry
    );
    this.schedulePersistSessionPipelines();
  }

  private updateActivePostStackNeighborhoodPipeline(
    nextPipeline: PostStackNeighborhoodProcessingPipeline
  ): void {
    if (this.pipelineFamily !== "post_stack_neighborhood") {
      return;
    }
    const activePipelineId = this.activeSessionPipelineId;
    const snapshot = clonePostStackNeighborhoodPipeline(nextPipeline);
    this.postStackNeighborhoodPipeline = snapshot;

    if (!activePipelineId) {
      return;
    }

    this.sessionPipelines = this.sessionPipelines.map((entry) =>
      entry.pipeline_id === activePipelineId
        ? {
            pipeline_id: entry.pipeline_id,
            family: "post_stack_neighborhood",
            pipeline: null,
            subvolume_crop: null,
            post_stack_neighborhood_pipeline: clonePostStackNeighborhoodPipeline(snapshot),
            updated_at_unix_s: pipelineTimestamp()
          }
        : entry
    );
    this.schedulePersistSessionPipelines();
  }

  private cloneEditableTraceLocalPipeline(): ProcessingPipeline {
    return this.pipelineFamily === "post_stack_neighborhood"
      ? clonePipeline(this.postStackNeighborhoodPipeline.trace_local_pipeline ?? createEmptyPipeline())
      : clonePipeline(this.pipeline);
  }

  private commitEditedTraceLocalPipeline(
    nextPipeline: ProcessingPipeline,
    nextSubvolumeCrop: SubvolumeCropOperation | null = this.subvolumeCrop
  ): void {
    nextPipeline.revision += 1;
    if (this.pipelineFamily === "post_stack_neighborhood") {
      const nextNeighborhood = clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline);
      nextNeighborhood.trace_local_pipeline = nextPipeline.steps.length > 0 ? nextPipeline : null;
      nextNeighborhood.revision += 1;
      this.updateActivePostStackNeighborhoodPipeline(nextNeighborhood);
      return;
    }
    this.updateActiveSessionPipeline(nextPipeline, nextSubvolumeCrop);
  }

  private clearPreviewState(): void {
    this.previewState = "raw";
    this.previewSection = null;
    this.previewLabel = null;
    this.previewedSectionKey = null;
  }

  private clearSpectrumState(): void {
    this.rawSpectrum = null;
    this.processedSpectrum = null;
    this.spectrumStale = false;
    this.spectrumError = null;
    this.spectrumSectionKey = null;
    this.spectrumSelectionKey = null;
  }

  openSpectrumInspector = (): void => {
    this.spectrumInspectorOpen = true;
  };

  closeSpectrumInspector = (): void => {
    this.spectrumInspectorOpen = false;
  };

  toggleSpectrumInspector = (): void => {
    this.spectrumInspectorOpen = !this.spectrumInspectorOpen;
  };

  setSpectrumAmplitudeScale = (scale: SpectrumAmplitudeScale): void => {
    this.spectrumAmplitudeScale = scale;
  };

  setSpectrumSelectionMode = (mode: SeismicSectionAnalysisSelectionMode): void => {
    this.spectrumSelectionMode = mode;
    if (this.rawSpectrum || this.processedSpectrum) {
      this.spectrumStale = true;
      this.spectrumError = null;
    }
  };

  selectStep = (index: number): void => {
    const operationCount = this.workspaceOperations.length;
    if (operationCount === 0) {
      this.selectedStepIndex = 0;
      return;
    }
    this.selectedStepIndex = Math.max(0, Math.min(index, operationCount - 1));
  };

  selectNextStep = (): void => {
    this.selectStep(this.selectedStepIndex + 1);
  };

  selectPreviousStep = (): void => {
    this.selectStep(this.selectedStepIndex - 1);
  };

  addAmplitudeScalarAfterSelected = (): void => {
    this.insertOperatorById("amplitude_scalar");
  };

  addTraceRmsNormalizeAfterSelected = (): void => {
    this.insertOperatorById("trace_rms_normalize");
  };

  addAgcRmsAfterSelected = (): void => {
    this.insertOperatorById("agc_rms");
  };

  addPhaseRotationAfterSelected = (): void => {
    this.insertOperatorById("phase_rotation");
  };

  addLowpassAfterSelected = (): void => {
    this.insertOperatorById("lowpass_filter");
  };

  addHighpassAfterSelected = (): void => {
    this.insertOperatorById("highpass_filter");
  };

  addBandpassAfterSelected = (): void => {
    this.insertOperatorById("bandpass_filter");
  };

  addVolumeArithmeticAfterSelected = (): void => {
    this.insertOperatorById("volume_subtract");
  };

  addCropSubvolumeAfterSelected = (): void => {
    this.insertOperatorById("crop_subvolume");
  };

  insertOperatorById = (operatorId: OperatorCatalogId): void => {
    const item = this.availableOperatorCatalogItems.find((candidate) => candidate.item_id === operatorId);
    if (!item) {
      this.error = `Operator '${operatorId}' is not available for the active dataset.`;
      return;
    }
    if (item.insertable.kind === "subvolume_crop") {
      this.insertCropSubvolume(item.insertable.crop);
      return;
    }
    this.insertOperation(item.insertable.operation);
  };

  insertOperation = (operation: WorkspaceOperation): void => {
    if (isCropSubvolume(operation)) {
      if (this.pipelineFamily === "post_stack_neighborhood") {
        this.error = "Neighborhood prefixes do not support crop subvolume steps.";
        return;
      }
      this.insertCropSubvolume(operation.crop_subvolume);
      return;
    }
    const next = this.cloneEditableTraceLocalPipeline();
    const insertIndex = this.nextTraceInsertIndexAfterSelection();
    const insertDisplayIndex = insertIndex;
    next.steps.splice(insertIndex, 0, createStep(operation));
    this.commitEditedTraceLocalPipeline(next);
    this.selectedStepIndex = insertDisplayIndex;
    this.editingParams = true;
    this.invalidatePreview();
  };

  insertCropSubvolume = (crop: SubvolumeCropOperation): void => {
    if (this.subvolumeCrop) {
      this.selectedStepIndex = this.pipeline.steps.length;
      this.editingParams = true;
      return;
    }
    this.updateActiveSessionPipeline(clonePipeline(this.pipeline), crop);
    this.selectedStepIndex = this.pipeline.steps.length;
    this.editingParams = true;
    this.invalidatePreview();
  };

  removeSelected = (): void => {
    this.removeOperationAt(this.selectedStepIndex);
  };

  removeOperationAt = (index: number): void => {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      const traceOperationIndex = this.traceOperationIndexForDisplayIndex(index);
      const prefixPipeline = this.postStackNeighborhoodPipeline.trace_local_pipeline;
      if (traceOperationIndex === null || !prefixPipeline?.steps[traceOperationIndex]) {
        return;
      }

      const removedSelectedOperation = index === this.selectedStepIndex;
      const next = clonePipeline(prefixPipeline);
      next.steps.splice(traceOperationIndex, 1);
      this.commitEditedTraceLocalPipeline(next);
      const nextWorkspaceOperationCount =
        next.steps.length + this.postStackNeighborhoodPipeline.operations.length;
      if (next.steps.length === 0) {
        this.selectedStepIndex = 0;
      } else if (index < this.selectedStepIndex) {
        this.selectedStepIndex -= 1;
      } else if (index === this.selectedStepIndex) {
        this.selectedStepIndex = Math.min(index, nextWorkspaceOperationCount - 1);
      }
      if (removedSelectedOperation || next.steps.length === 0) {
        this.editingParams = false;
      }
      this.invalidatePreview();
      return;
    }

    if (this.subvolumeCrop && index === this.pipeline.steps.length) {
      this.updateActiveSessionPipeline(clonePipeline(this.pipeline), null);
      this.selectedStepIndex = Math.max(0, Math.min(index - 1, this.pipeline.steps.length - 1));
      this.editingParams = this.pipeline.steps.length > 0;
      this.invalidatePreview();
      return;
    }

    const traceOperationIndex = this.traceOperationIndexForDisplayIndex(index);
    if (traceOperationIndex === null || !this.pipeline.steps[traceOperationIndex]) {
      return;
    }

    const removedSelectedOperation = index === this.selectedStepIndex;
    const next = this.cloneEditableTraceLocalPipeline();
    next.steps.splice(traceOperationIndex, 1);
    this.commitEditedTraceLocalPipeline(next, this.subvolumeCrop);
    const nextWorkspaceOperationCount = next.steps.length + (this.subvolumeCrop ? 1 : 0);
    if (next.steps.length === 0) {
      this.selectedStepIndex = this.subvolumeCrop ? 0 : 0;
    } else if (index < this.selectedStepIndex) {
      this.selectedStepIndex -= 1;
    } else if (index === this.selectedStepIndex) {
      this.selectedStepIndex = Math.min(index, nextWorkspaceOperationCount - 1);
    }
    if (removedSelectedOperation || next.steps.length === 0) {
      this.editingParams = false;
    }
    this.invalidatePreview();
  };

  moveSelectedUp = (): void => {
    if (!this.canMoveSelectedUp || !this.selectedOperation) {
      return;
    }
    const fromIndex = this.selectedTraceOperationIndex();
    if (fromIndex === null) {
      return;
    }
    const toIndex = fromIndex - 1;
    const next = this.cloneEditableTraceLocalPipeline();
    const [step] = next.steps.splice(fromIndex, 1);
    next.steps.splice(toIndex, 0, step);
    this.commitEditedTraceLocalPipeline(next, this.subvolumeCrop);
    this.selectedStepIndex -= 1;
    this.invalidatePreview();
  };

  moveSelectedDown = (): void => {
    if (!this.canMoveSelectedDown || !this.selectedOperation) {
      return;
    }
    const fromIndex = this.selectedTraceOperationIndex();
    if (fromIndex === null) {
      return;
    }
    const toIndex = fromIndex + 1;
    const next = this.cloneEditableTraceLocalPipeline();
    const [step] = next.steps.splice(fromIndex, 1);
    next.steps.splice(toIndex, 0, step);
    this.commitEditedTraceLocalPipeline(next, this.subvolumeCrop);
    this.selectedStepIndex += 1;
    this.invalidatePreview();
  };

  beginParamEdit = (): void => {
    this.editingParams = Boolean(this.selectedOperation);
  };

  endParamEdit = (): void => {
    this.editingParams = false;
  };

  setPipelineName = (value: string): void => {
    if (this.pipelineFamily === "post_stack_neighborhood") {
      this.updateActivePostStackNeighborhoodPipeline({
        ...clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline),
        name: value.trim() || null
      });
      return;
    }

    this.updateActiveSessionPipeline({
      ...clonePipeline(this.pipeline),
      name: value.trim() || null
    });
  };

  setSelectedNeighborhoodWindow = (field: keyof PostStackNeighborhoodWindow, value: number): void => {
    const selected = this.selectedNeighborhoodOperation;
    if (!selected || !Number.isFinite(value)) {
      return;
    }

    const next = clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline);
    const operation = next.operations[0];
    if (!operation) {
      return;
    }

    const window =
      "similarity" in operation
        ? operation.similarity.window
        : "local_volume_stats" in operation
          ? operation.local_volume_stats.window
          : "dip" in operation
            ? operation.dip.window
          : null;
    if (!window) {
      return;
    }

    if (field === "inline_stepout" || field === "xline_stepout") {
      window[field] = Math.max(0, Math.round(value));
    } else {
      window[field] = value;
    }
    next.revision += 1;
    this.updateActivePostStackNeighborhoodPipeline(next);
    this.invalidatePreview();
  };

  setSelectedNeighborhoodStatistic = (statistic: LocalVolumeStatistic): void => {
    const selected = this.selectedNeighborhoodOperation;
    if (!selected || !("local_volume_stats" in selected)) {
      return;
    }

    const next = clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline);
    const operation = next.operations[0];
    if (!operation || !("local_volume_stats" in operation)) {
      return;
    }

    operation.local_volume_stats.statistic = statistic;
    next.revision += 1;
    this.updateActivePostStackNeighborhoodPipeline(next);
    this.invalidatePreview();
  };

  setSelectedNeighborhoodDipOutput = (output: NeighborhoodDipOutput): void => {
    const selected = this.selectedNeighborhoodOperation;
    if (!selected || !("dip" in selected)) {
      return;
    }

    const next = clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline);
    const operation = next.operations[0];
    if (!operation || !("dip" in operation)) {
      return;
    }

    operation.dip.output = output;
    next.revision += 1;
    this.updateActivePostStackNeighborhoodPipeline(next);
    this.invalidatePreview();
  };

  setSelectedNeighborhoodOperatorKind = (
    kind: "similarity" | "local_volume_stats" | "dip"
  ): void => {
    const selected = this.selectedNeighborhoodOperation;
    if (!selected) {
      return;
    }
    if (
      (kind === "similarity" && "similarity" in selected) ||
      (kind === "local_volume_stats" && "local_volume_stats" in selected) ||
      (kind === "dip" && "dip" in selected)
    ) {
      return;
    }

    const next = clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline);
    const current = next.operations[0];
    if (!current) {
      return;
    }
    const window = neighborhoodWindowForOperation(current);
    next.operations[0] =
      kind === "similarity"
        ? {
            similarity: {
              window
            }
          }
        : kind === "local_volume_stats"
          ? defaultNeighborhoodLocalVolumeStats(window)
          : defaultNeighborhoodDip(window);
    next.revision += 1;
    this.updateActivePostStackNeighborhoodPipeline(next);
    this.invalidatePreview();
  };

  setSelectedAmplitudeScalarFactor = (value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isAmplitudeScalar(selected)) {
      return;
    }
    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isAmplitudeScalar(operation)) {
      return;
    }
    operation.amplitude_scalar.factor = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedAgcWindow = (value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isAgcRms(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isAgcRms(operation)) {
      return;
    }

    operation.agc_rms.window_ms = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedLowpassCorner = (corner: "f3_hz" | "f4_hz", value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isLowpassFilter(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isLowpassFilter(operation)) {
      return;
    }

    operation.lowpass_filter[corner] = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedHighpassCorner = (corner: "f1_hz" | "f2_hz", value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isHighpassFilter(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isHighpassFilter(operation)) {
      return;
    }

    operation.highpass_filter[corner] = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedBandpassCorner = (
    corner: "f1_hz" | "f2_hz" | "f3_hz" | "f4_hz",
    value: number
  ): void => {
    const selected = this.selectedOperation;
    if (!selected || !isBandpassFilter(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isBandpassFilter(operation)) {
      return;
    }

    operation.bandpass_filter[corner] = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedPhaseRotationAngle = (value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isPhaseRotation(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isPhaseRotation(operation)) {
      return;
    }

    operation.phase_rotation.angle_degrees = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedVolumeArithmeticOperator = (value: VolumeArithmeticOperator): void => {
    const selected = this.selectedOperation;
    if (!selected || !isVolumeArithmetic(selected)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isVolumeArithmetic(operation)) {
      return;
    }

    operation.volume_arithmetic.operator = value;
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedVolumeArithmeticSecondaryStorePath = (value: string): void => {
    const selected = this.selectedOperation;
    if (!selected || isCropSubvolume(selected) || !isVolumeArithmetic(selected)) {
      return;
    }

    const next = this.cloneEditableTraceLocalPipeline();
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isVolumeArithmetic(operation)) {
      return;
    }

    operation.volume_arithmetic.secondary_store_path = value.trim();
    this.commitEditedTraceLocalPipeline(next);
    this.invalidatePreview();
  };

  setSelectedSubvolumeCropBound = (
    bound:
      | "inline_min"
      | "inline_max"
      | "xline_min"
      | "xline_max"
      | "z_min_ms"
      | "z_max_ms",
    value: number
  ): void => {
    const selected = this.selectedOperation;
    if (!selected || !isCropSubvolume(selected) || !Number.isFinite(value)) {
      return;
    }

    const nextCrop = {
      ...selected.crop_subvolume,
      [bound]: value
    };
    this.updateActiveSessionPipeline(clonePipeline(this.pipeline), nextCrop);
    this.invalidatePreview();
  };

  private async applyPresetPipelineSpec(pipeline: ProcessingPipelineSpec): Promise<boolean> {
    if ("gather" in pipeline) {
      this.error = "Gather library templates cannot be applied in this workspace.";
      this.viewerModel.note("Failed to apply library template.", "ui", "warn", this.error);
      return false;
    }
    const applied = await this.runSessionAction(
      {
        action: "replace_active_from_pipeline_spec",
        pipeline
      },
      "Failed to apply the selected library template."
    );
    if (applied) {
      this.invalidatePreview();
    }
    return applied;
  }

  loadPreset = async (preset: ProcessingPreset): Promise<void> => {
    if (processingPipelineSpecFamily(preset.pipeline) !== this.activePresetFamily) {
      this.error = "Library template family does not match the active processing workflow.";
      this.viewerModel.note("Failed to apply library template.", "ui", "warn", this.error);
      return;
    }
    if (!(await this.applyPresetPipelineSpec(preset.pipeline))) {
      return;
    }
    this.viewerModel.setSelectedPresetId(preset.preset_id);
    this.viewerModel.note("Applied library template to the active pipeline.", "ui", "info", preset.preset_id);
  };

  savePreset = async (): Promise<void> => {
    const presetId =
      normalizePresetId(this.pipelineTitle ?? this.pipeline.preset_id ?? `pipeline-${++this.#presetCounter}`) ||
      `pipeline-${++this.#presetCounter}`;
    const preset: ProcessingPreset = {
      preset_id: presetId,
      pipeline: withPresetIdOnPipelineSpec(
        batchPipelineSpecForWorkspace(
          this.pipelineFamily,
          this.pipeline,
          this.postStackNeighborhoodPipeline,
          this.subvolumeCrop
        ),
        presetId
      ),
      created_at_unix_s: 0,
      updated_at_unix_s: 0
    };
    try {
      const response = await savePipelinePreset(preset);
      await this.applyPresetPipelineSpec(response.preset.pipeline);
      this.viewerModel.setSelectedPresetId(response.preset.preset_id);
      await this.refreshPresets();
      this.viewerModel.note("Saved pipeline as a library template.", "ui", "info", response.preset.preset_id);
    } catch (error) {
      this.error = errorMessage(error, "Failed to save library template.");
      this.viewerModel.note("Failed to save library template.", "backend", "error", this.error);
    }
  };

  deletePreset = async (presetId: string): Promise<void> => {
    try {
      const deleted = await deletePipelinePreset(presetId);
      if (deleted) {
        if (this.viewerModel.selectedPresetId === presetId) {
          this.viewerModel.setSelectedPresetId(null);
        }
        await this.refreshPresets();
        this.viewerModel.note("Deleted library template.", "ui", "warn", presetId);
      }
    } catch (error) {
      this.error = errorMessage(error, "Failed to delete library template.");
      this.viewerModel.note("Failed to delete library template.", "backend", "error", this.error);
    }
  };

  previewCurrentSection = async (): Promise<void> => {
    if (!this.canPreview || !this.viewerModel.dataset || !this.viewerModel.activeStorePath) {
      this.error =
        this.pipelineFamily === "trace_local" && this.pipeline.steps.length === 0 && this.subvolumeCrop
          ? "Crop Subvolume runs only on full-volume execution. Add a processing step to preview."
          : "Open a dataset and load a section before previewing.";
      return;
    }

    this.previewBusy = true;
    this.error = null;
    const previewStartedMs = nowMs();
    const section = {
      dataset_id: this.viewerModel.dataset.descriptor.id,
      axis: this.viewerModel.axis,
      index: this.viewerModel.index
    };
    const storePath = this.viewerModel.activeStorePath;
    const operatorIds =
      this.pipelineFamily === "post_stack_neighborhood"
        ? this.postStackNeighborhoodPipeline.operations.map((operation) =>
            "similarity" in operation
              ? "similarity"
              : "local_volume_stats" in operation
                ? "local_volume_stats"
                : "dip" in operation
                  ? "dip"
                  : "neighborhood"
          )
        : previewOperationIds(this.pipeline);
    const previewMode = this.pipelineFamily;
    try {
      const response =
        this.pipelineFamily === "post_stack_neighborhood"
          ? await previewPostStackNeighborhoodProcessing({
              schema_version: SCHEMA_VERSION,
              store_path: storePath,
              section,
              pipeline: clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline)
            } satisfies PreviewPostStackNeighborhoodProcessingRequest)
          : await previewProcessing({
              schema_version: SCHEMA_VERSION,
              store_path: storePath,
              section,
              pipeline: clonePipeline(this.pipeline)
            } satisfies PreviewProcessingRequest);
      const previewResolvedMs = nowMs();
      this.previewSection = response.preview.section;
      const stateAssignedMs = nowMs();
      this.previewState = "preview";
      this.previewLabel = response.preview.processing_label;
      this.previewedSectionKey = sectionKey(this.viewerModel);
      await tick();
      const afterTickMs = nowMs();
      await nextAnimationFrame();
      const afterFirstFrameMs = nowMs();
      await nextAnimationFrame();
      const afterSecondFrameMs = nowMs();
      const previewSection = response.preview.section;
      void emitFrontendDiagnosticsEvent({
        stage: "preview_current_section",
        level: "info",
        message: "Frontend preview pipeline timings recorded",
        fields: {
          previewMode,
          storePath,
          datasetId: section.dataset_id,
          axis: section.axis,
          index: section.index,
          pipelineRevision:
            this.pipelineFamily === "post_stack_neighborhood"
              ? this.postStackNeighborhoodPipeline.revision
              : this.pipeline.revision,
          pipelineName:
            this.pipelineFamily === "post_stack_neighborhood"
              ? postStackNeighborhoodPipelineName(this.postStackNeighborhoodPipeline)
              : pipelineName(this.pipeline),
          operatorCount: operatorIds.length,
          operatorIds,
          hasRunOnlySubvolumeCrop: this.pipelineFamily === "trace_local" && this.subvolumeCrop !== null,
          previewReady: response.preview.preview_ready,
          processingLabel: response.preview.processing_label,
          traces: previewSection.traces,
          samples: previewSection.samples,
          payloadBytes: estimateSectionPayloadBytes(previewSection),
          frontendAwaitMs: previewResolvedMs - previewStartedMs,
          frontendStateAssignMs: stateAssignedMs - previewResolvedMs,
          frontendTickMs: afterTickMs - stateAssignedMs,
          frontendFirstFrameMs: afterFirstFrameMs - afterTickMs,
          frontendSecondFrameMs: afterSecondFrameMs - afterFirstFrameMs,
          frontendCommitToSecondFrameMs: afterSecondFrameMs - stateAssignedMs,
          frontendTotalMs: afterSecondFrameMs - previewStartedMs
        }
      }).catch((error) => {
        this.viewerModel.note(
          "Failed to record frontend preview timings.",
          "backend",
          "warn",
          error instanceof Error ? error.message : String(error)
        );
      });
      this.viewerModel.note("Processing preview generated.", "backend", "info", this.previewLabel);
    } catch (error) {
      this.error = errorMessage(
        error,
        this.pipelineFamily === "post_stack_neighborhood"
          ? "Failed to preview neighborhood processing."
          : "Failed to preview processing pipeline."
      );
      this.viewerModel.note("Processing preview failed.", "backend", "error", this.error);
    } finally {
      this.previewBusy = false;
    }
  };

  refreshSpectrum = async (): Promise<void> => {
    const currentSection = this.viewerModel.section;
    const currentSelection = this.activeSpectrumSelection;
    if (
      !this.canInspectSpectrum ||
      !this.viewerModel.dataset ||
      !this.viewerModel.activeStorePath ||
      !currentSection ||
      !currentSelection
    ) {
      this.spectrumError = "Open a dataset and load a section before inspecting the spectrum.";
      return;
    }

    this.spectrumBusy = true;
    this.spectrumError = null;
    try {
      const baseRequest: AmplitudeSpectrumRequest = {
        schema_version: SCHEMA_VERSION,
        store_path: this.viewerModel.activeStorePath,
        section: {
          dataset_id: this.viewerModel.dataset.descriptor.id,
          axis: this.viewerModel.axis,
          index: this.viewerModel.index
        },
        selection: toSpectrumSelection(currentSelection),
        pipeline: null
      };

      const rawResponse = await fetchAmplitudeSpectrum(baseRequest);
      this.rawSpectrum = rawResponse;

      if (this.pipeline.steps.length > 0) {
        this.processedSpectrum = await fetchAmplitudeSpectrum({
          ...baseRequest,
          pipeline: clonePipeline(this.pipeline)
        });
      } else {
        this.processedSpectrum = null;
      }

      this.spectrumStale = false;
      this.spectrumSectionKey = sectionKey(this.viewerModel);
      this.spectrumSelectionKey = buildAnalysisSelectionKey(this.spectrumSectionKey, currentSelection);
      this.viewerModel.note("Amplitude spectrum generated.", "backend", "info", this.spectrumSelectionSummary);
    } catch (error) {
      this.spectrumError = errorMessage(error, "Failed to inspect amplitude spectrum.");
      this.viewerModel.note("Amplitude spectrum failed.", "backend", "error", this.spectrumError);
    } finally {
      this.spectrumBusy = false;
    }
  };

  showRawSection = (): void => {
    this.previewState = this.previewedSectionKey === sectionKey(this.viewerModel) ? "stale" : "raw";
  };

  setBatchMaxActiveJobs = (value: string): void => {
    this.batchMaxActiveJobs = value.replace(/[^0-9]/g, "");
  };

  setBatchExecutionMode = (value: BatchExecutionModeSelection): void => {
    this.batchExecutionMode = value;
  };

  toggleBatchStorePath = (storePath: string): void => {
    const normalizedStorePath = storePath.trim();
    if (!normalizedStorePath) {
      return;
    }
    const selectedStorePaths = this.selectedBatchStorePaths.includes(normalizedStorePath)
      ? this.selectedBatchStorePaths.filter((candidateStorePath) => candidateStorePath !== normalizedStorePath)
      : [...this.selectedBatchStorePaths, normalizedStorePath];
    this.selectedBatchStorePaths = this.batchCandidates
      .map((candidate) => candidate.storePath)
      .filter((candidateStorePath) => selectedStorePaths.includes(candidateStorePath));
  };

  selectAllBatchCandidates = (): void => {
    this.selectedBatchStorePaths = this.batchCandidates.map((candidate) => candidate.storePath);
  };

  clearBatchSelection = (): void => {
    this.selectedBatchStorePaths = [];
  };

  runOnVolume = async (): Promise<void> => {
    if (!this.canRun || !this.viewerModel.activeStorePath) {
      this.error = "Open a dataset before running processing on the full volume.";
      return;
    }
    this.runBusy = true;
    this.error = null;
    try {
      const outputStorePath =
        this.runOutputPathMode === "custom"
          ? this.customRunOutputPath.trim()
          : await this.resolveDefaultRunOutputPathForState(
              this.viewerModel.activeStorePath,
              clonePipeline(this.pipeline),
              clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline),
              cloneSubvolumeCrop(this.subvolumeCrop)
            );
      if (!outputStorePath) {
        this.error = "Select an output runtime store path before running the full volume.";
        this.runBusy = false;
        return;
      }
      await this.startRunOnVolume(outputStorePath, this.overwriteExistingRunOutput);
    } catch (error) {
      this.error = errorMessage(error, "Failed to start processing job.");
      if (!this.overwriteExistingRunOutput && isExistingOutputStoreError(this.error)) {
        const confirmed = await confirmOverwriteStore(
          this.resolvedRunOutputPath ?? this.customRunOutputPath.trim()
        );
        if (confirmed) {
          this.overwriteExistingRunOutput = true;
          const outputStorePath =
            this.resolvedRunOutputPath ??
            (this.viewerModel.activeStorePath
              ? await this.resolveDefaultRunOutputPathForState(
                  this.viewerModel.activeStorePath,
                  clonePipeline(this.pipeline),
                  clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline),
                  cloneSubvolumeCrop(this.subvolumeCrop)
                )
              : null);
          if (outputStorePath) {
            try {
              await this.startRunOnVolume(outputStorePath, true);
              return;
            } catch (retryError) {
              this.error = errorMessage(retryError, "Failed to start processing job.");
            }
          }
        }
      }
      this.runBusy = false;
      this.viewerModel.note("Failed to start processing job.", "backend", "error", this.error);
    }
  };

  runBatchOnVolumes = async (): Promise<void> => {
    const selectedStorePaths = this.batchCandidates
      .map((candidate) => candidate.storePath)
      .filter((storePath) => this.selectedBatchStorePaths.includes(storePath));
    if (!selectedStorePaths.length) {
      this.error = "Select at least one compatible dataset before starting a batch run.";
      return;
    }

    this.batchSubmitting = true;
    this.error = null;
    try {
      const familyLabel = batchPipelineFamilyLabel(this.pipelineFamily, this.subvolumeCrop);
      const response = await submitProcessingBatch({
        schema_version: SCHEMA_VERSION,
        items: selectedStorePaths.map((store_path) => ({
          store_path,
          output_store_path: null
        })),
        overwrite_existing: this.overwriteExistingRunOutput,
        execution_mode: this.batchExecutionMode,
        max_active_jobs: parsePositiveInteger(this.batchMaxActiveJobs),
        pipeline: batchPipelineSpecForWorkspace(
          this.pipelineFamily,
          this.pipeline,
          this.postStackNeighborhoodPipeline,
          this.subvolumeCrop
        )
      });
      this.setActiveBatchStatus(response.batch);
      this.viewerModel.note(
        "Started processing batch.",
        "backend",
        "info",
        `${response.batch.progress.total_jobs} ${familyLabel} dataset runs`
      );
      this.scheduleBatchPoll();
    } catch (error) {
      this.error = errorMessage(error, "Failed to start processing batch.");
      this.viewerModel.note("Failed to start processing batch.", "backend", "error", this.error);
    } finally {
      this.batchSubmitting = false;
    }
  };

  cancelActiveJob = async (): Promise<void> => {
    if (!this.activeJob) {
      return;
    }
    try {
      const response = await cancelProcessingJob(this.activeJob.job_id);
      this.setActiveJobStatus(response.job);
      this.viewerModel.note("Requested processing job cancellation.", "ui", "warn", response.job.job_id);
    } catch (error) {
      this.error = errorMessage(error, "Failed to cancel processing job.");
    }
  };

  cancelActiveBatch = async (): Promise<void> => {
    if (!this.activeBatch) {
      return;
    }
    try {
      const response = await cancelProcessingBatch(this.activeBatch.batch_id);
      this.setActiveBatchStatus(response.batch);
      this.viewerModel.note(
        "Requested processing batch cancellation.",
        "ui",
        "warn",
        response.batch.batch_id
      );
    } catch (error) {
      this.error = errorMessage(error, "Failed to cancel processing batch.");
    }
  };

  focusRecentJob = (jobId: string): void => {
    const match = this.recentJobs.find((entry) => entry.job.job_id === jobId);
    if (!match) {
      return;
    }
    this.setActiveJobStatus(match.job);
    if (match.job.state === "queued" || match.job.state === "running") {
      this.runBusy = true;
      this.scheduleJobPoll();
    }
  };

  focusRecentBatch = (batchId: string): void => {
    const match = this.recentBatches.find((entry) => entry.batch.batch_id === batchId);
    if (!match) {
      return;
    }
    this.setActiveBatchStatus(match.batch);
    if (match.batch.state === "queued" || match.batch.state === "running") {
      this.scheduleBatchPoll();
    }
  };

  clearFinishedRecentActivity = (): void => {
    this.recentJobs = this.recentJobs.filter((entry) => isActiveJobState(entry.job.state));
    this.recentBatches = this.recentBatches.filter((entry) => isActiveBatchState(entry.batch.state));
  };

  handleKeydown = async (event: KeyboardEvent): Promise<void> => {
    const target = event.target as HTMLElement | null;
    const tagName = target?.tagName?.toLowerCase();
    const editingText = Boolean(
      target?.isContentEditable ||
        tagName === "input" ||
        tagName === "textarea" ||
        tagName === "select"
    );
    if (editingText && !event.ctrlKey && !event.metaKey && event.key !== "Escape") {
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (this.pipelineFamily === "trace_local") {
          await this.savePreset();
        }
      }
      return;
    }

    if (this.pipelineFamily === "post_stack_neighborhood") {
      switch (event.key) {
        case "j":
          event.preventDefault();
          this.selectNextStep();
          break;
        case "k":
          event.preventDefault();
          this.selectPreviousStep();
          break;
        case "p":
          event.preventDefault();
          await this.previewCurrentSection();
          break;
        case "r":
          event.preventDefault();
          await this.runOnVolume();
          break;
      }
      return;
    }

    switch (event.key) {
      case "j":
        event.preventDefault();
        this.selectNextStep();
        break;
      case "k":
        event.preventDefault();
        this.selectPreviousStep();
        break;
      case "J":
        event.preventDefault();
        this.moveSelectedDown();
        break;
      case "K":
        event.preventDefault();
        this.moveSelectedUp();
        break;
      case "a":
        event.preventDefault();
        this.addAmplitudeScalarAfterSelected();
        break;
      case "n":
        event.preventDefault();
        this.addTraceRmsNormalizeAfterSelected();
        break;
      case "g":
        event.preventDefault();
        this.addAgcRmsAfterSelected();
        break;
      case "h":
        event.preventDefault();
        this.addPhaseRotationAfterSelected();
        break;
      case "l":
        event.preventDefault();
        this.addLowpassAfterSelected();
        break;
      case "i":
        event.preventDefault();
        this.addHighpassAfterSelected();
        break;
      case "b":
        event.preventDefault();
        this.addBandpassAfterSelected();
        break;
      case "v":
        event.preventDefault();
        this.addVolumeArithmeticAfterSelected();
        break;
      case "c":
        event.preventDefault();
        this.addCropSubvolumeAfterSelected();
        break;
      case "x":
      case "Delete":
        event.preventDefault();
        this.removeSelected();
        break;
      case "Enter":
        event.preventDefault();
        this.beginParamEdit();
        break;
      case "Escape":
        event.preventDefault();
        this.endParamEdit();
        break;
      case "p":
        event.preventDefault();
        await this.previewCurrentSection();
        break;
      case "s":
        event.preventDefault();
        this.openSpectrumInspector();
        if (!this.rawSpectrum && !this.spectrumBusy) {
          await this.refreshSpectrum();
        }
        break;
      case "r":
        event.preventDefault();
        await this.runOnVolume();
        break;
      case "F9":
        event.preventDefault();
        this.toggleCheckpointAfterOperation(this.selectedStepIndex);
        break;
    }
  };

  private scheduleJobPoll(): void {
    if (!this.activeJob || typeof window === "undefined") {
      return;
    }
    if (this.#jobPollTimer !== null) {
      window.clearTimeout(this.#jobPollTimer);
    }
    this.#jobPollTimer = window.setTimeout(() => {
      void this.pollActiveJob();
    }, 500);
  }

  private scheduleBatchPoll(): void {
    if (!this.activeBatch || typeof window === "undefined") {
      return;
    }
    if (this.#batchPollTimer !== null) {
      window.clearTimeout(this.#batchPollTimer);
    }
    this.#batchPollTimer = window.setTimeout(() => {
      void this.pollActiveBatch();
    }, 750);
  }

  private async pollActiveJob(): Promise<void> {
    if (!this.activeJob) {
      this.runBusy = false;
      return;
    }
    try {
      const response = await getProcessingJob(this.activeJob.job_id);
      this.setActiveJobStatus(response.job);
      await this.refreshActiveJobDebug(response.job.job_id);
      switch (response.job.state) {
        case "queued":
        case "running":
          this.runBusy = true;
          this.scheduleJobPoll();
          break;
        case "completed":
          this.runBusy = false;
          {
            const finalOutputStorePath =
              response.job.output_store_path ??
              response.job.artifacts.find((artifact) => artifact.kind === "final_output")?.store_path ??
              null;
            await this.viewerModel.refreshWorkspaceState();
            this.viewerModel.note(
              "Processing job completed.",
              "backend",
              "info",
              finalOutputStorePath ?? response.job.job_id
            );
          }
          break;
        case "cancelled":
          this.runBusy = false;
          this.viewerModel.note("Processing job cancelled.", "backend", "warn", response.job.job_id);
          break;
        case "failed":
          this.runBusy = false;
          this.error = response.job.error_message ?? "Processing job failed.";
          this.viewerModel.note("Processing job failed.", "backend", "error", this.error);
          break;
      }
    } catch (error) {
      this.runBusy = false;
      this.error = errorMessage(error, "Failed to poll processing job.");
      this.viewerModel.note("Processing job polling failed.", "backend", "error", this.error);
    }
  }

  private async pollActiveBatch(): Promise<void> {
    if (!this.activeBatch) {
      return;
    }
    try {
      const response = await getProcessingBatch(this.activeBatch.batch_id);
      this.setActiveBatchStatus(response.batch);
      switch (response.batch.state) {
        case "queued":
        case "running":
          this.scheduleBatchPoll();
          break;
        case "completed":
          await this.viewerModel.refreshWorkspaceState();
          this.viewerModel.note(
            "Processing batch completed.",
            "backend",
            "info",
            `${response.batch.progress.total_jobs} datasets`
          );
          break;
        case "completed_with_errors": {
          await this.viewerModel.refreshWorkspaceState();
          const failedCount = response.batch.items.filter((item) => item.state === "failed").length;
          this.error =
            failedCount > 0
              ? `${failedCount} batch item${failedCount === 1 ? "" : "s"} failed.`
              : "Processing batch completed with errors.";
          this.viewerModel.note("Processing batch completed with errors.", "backend", "warn", this.error);
          break;
        }
        case "cancelled":
          this.viewerModel.note(
            "Processing batch cancelled.",
            "backend",
            "warn",
            response.batch.batch_id
          );
          break;
      }
    } catch (error) {
      this.error = errorMessage(error, "Failed to poll processing batch.");
      this.viewerModel.note("Processing batch polling failed.", "backend", "error", this.error);
    }
  }

  private setActiveJobStatus(job: ProcessingJobStatus | null): void {
    const previousJobId = this.#activeDebugJobId;
    this.activeJob = job;
    if (!job) {
      this.#activeDebugJobId = null;
      this.#latestRuntimeEventSeq = 0;
      this.activeDebugPlan = null;
      this.activeRuntimeState = null;
      this.activeRuntimeEvents = [];
      return;
    }
    if (job.job_id !== previousJobId) {
      this.#activeDebugJobId = job.job_id;
      this.#latestRuntimeEventSeq = 0;
      this.activeDebugPlan = job.inspectable_plan ?? null;
      this.activeRuntimeState = null;
      this.activeRuntimeEvents = [];
      void this.refreshActiveJobDebug(job.job_id, true);
    } else if (job.inspectable_plan) {
      this.activeDebugPlan = job.inspectable_plan;
    }
    if (job) {
      this.upsertRecentJob(job);
    }
  }

  private async refreshActiveJobDebug(
    jobId: string,
    includePlan = false
  ): Promise<void> {
    try {
      const [planResponse, runtimeResponse, eventsResponse] = await Promise.all([
        includePlan || !this.activeDebugPlan
          ? getProcessingDebugPlan(jobId)
          : Promise.resolve({ schema_version: SCHEMA_VERSION, plan: this.activeDebugPlan }),
        getProcessingRuntimeState(jobId),
        listProcessingRuntimeEvents(jobId, this.#latestRuntimeEventSeq || null)
      ]);
      if (this.#activeDebugJobId !== jobId) {
        return;
      }
      this.activeDebugPlan = planResponse.plan ?? this.activeDebugPlan;
      this.activeRuntimeState = runtimeResponse.runtime;
      const events = eventsResponse.events ?? [];
      if (events.length > 0) {
        this.activeRuntimeEvents = [...this.activeRuntimeEvents, ...events].slice(-128);
        this.#latestRuntimeEventSeq =
          events[events.length - 1]?.seq ?? this.#latestRuntimeEventSeq;
      } else {
        this.#latestRuntimeEventSeq =
          runtimeResponse.runtime.latest_event_seq ?? this.#latestRuntimeEventSeq;
      }
    } catch (error) {
      this.viewerModel.note(
        "Processing debug refresh failed.",
        "backend",
        "warn",
        errorMessage(error, "Failed to refresh processing debug state.")
      );
    }
  }

  private setActiveBatchStatus(batch: ProcessingBatchStatus | null): void {
    this.activeBatch = batch;
    if (batch) {
      this.upsertRecentBatch(batch);
    }
  }

  private upsertRecentJob(job: ProcessingJobStatus): void {
    const entry: RecentProcessingJobEntry = {
      kind: "job",
      job,
      familyLabel: processingPipelineFamilyLabel(job.pipeline),
      title: recentJobTitle(job)
    };
    this.recentJobs = [
      entry,
      ...this.recentJobs.filter((candidate) => candidate.job.job_id !== job.job_id)
    ]
      .sort((left, right) => right.job.updated_at_unix_s - left.job.updated_at_unix_s)
      .slice(0, 8);
  }

  private upsertRecentBatch(batch: ProcessingBatchStatus): void {
    const entry: RecentProcessingBatchEntry = {
      kind: "batch",
      batch,
      familyLabel: processingPipelineFamilyLabel(batch.pipeline),
      title: recentBatchTitle(batch)
    };
    this.recentBatches = [
      entry,
      ...this.recentBatches.filter((candidate) => candidate.batch.batch_id !== batch.batch_id)
    ]
      .sort((left, right) => right.batch.updated_at_unix_s - left.batch.updated_at_unix_s)
      .slice(0, 8);
  }

  private invalidatePreview(): void {
    if (this.previewSection) {
      this.previewState = "stale";
    } else {
      this.previewState = "raw";
    }
    if (this.rawSpectrum || this.processedSpectrum) {
      this.spectrumStale = true;
      this.spectrumError = null;
    }
  }

  private async refreshDefaultRunOutputPath(
    activeStorePath: string,
    pipeline: ProcessingPipeline,
    postStackNeighborhoodPipeline: PostStackNeighborhoodProcessingPipeline,
    subvolumeCrop: SubvolumeCropOperation | null,
    signature: string
  ): Promise<void> {
    const requestId = ++this.#runOutputPathRequestId;
    this.resolvingRunOutputPath = true;
    try {
      const nextPath = await this.resolveDefaultRunOutputPathForState(
        activeStorePath,
        pipeline,
        postStackNeighborhoodPipeline,
        subvolumeCrop
      );
      if (
        requestId !== this.#runOutputPathRequestId ||
        activeStorePath !== this.viewerModel.activeStorePath ||
        signature !==
          (this.pipelineFamily === "post_stack_neighborhood"
            ? postStackNeighborhoodPipelineRunOutputSignature(this.postStackNeighborhoodPipeline)
            : workspaceRunOutputSignature(this.pipeline, this.subvolumeCrop))
      ) {
        return;
      }
      this.defaultRunOutputPath = nextPath;
    } catch {
      if (requestId !== this.#runOutputPathRequestId) {
        return;
      }
      this.defaultRunOutputPath = null;
    } finally {
      if (requestId === this.#runOutputPathRequestId) {
        this.resolvingRunOutputPath = false;
      }
    }
  }

  private scheduleDefaultRunOutputPathRefresh(
    activeStorePath: string,
    pipeline: ProcessingPipeline,
    postStackNeighborhoodPipeline: PostStackNeighborhoodProcessingPipeline,
    subvolumeCrop: SubvolumeCropOperation | null,
    signature: string
  ): void {
    if (typeof window === "undefined") {
      void this.refreshDefaultRunOutputPath(
        activeStorePath,
        pipeline,
        postStackNeighborhoodPipeline,
        subvolumeCrop,
        signature
      );
      return;
    }

    if (this.#runOutputPathRefreshTimer !== null) {
      window.clearTimeout(this.#runOutputPathRefreshTimer);
    }

    this.#runOutputPathRefreshTimer = window.setTimeout(() => {
      this.#runOutputPathRefreshTimer = null;
      void this.refreshDefaultRunOutputPath(
        activeStorePath,
        pipeline,
        postStackNeighborhoodPipeline,
        subvolumeCrop,
        signature
      );
    }, RUN_OUTPUT_PATH_REFRESH_DEBOUNCE_MS);
  }

  private async resolveDefaultRunOutputPathForState(
    activeStorePath: string,
    pipeline: ProcessingPipeline,
    postStackNeighborhoodPipeline: PostStackNeighborhoodProcessingPipeline,
    subvolumeCrop: SubvolumeCropOperation | null
  ): Promise<string> {
    const response = await resolveProcessingRunOutput({
      schema_version: SCHEMA_VERSION,
      store_path: activeStorePath,
      family:
        this.pipelineFamily === "post_stack_neighborhood"
          ? "post_stack_neighborhood"
          : subvolumeCrop
            ? "subvolume"
            : "trace_local",
      pipeline,
      subvolume_crop: subvolumeCrop,
      post_stack_neighborhood_pipeline:
        this.pipelineFamily === "post_stack_neighborhood" ? postStackNeighborhoodPipeline : null
    });
    return response.output_store_path;
  }

  private async startRunOnVolume(outputStorePath: string, overwriteExisting: boolean): Promise<void> {
    if (!this.viewerModel.activeStorePath) {
      throw new Error("Open a dataset before running processing on the full volume.");
    }

    const response =
      this.pipelineFamily === "post_stack_neighborhood"
        ? await runPostStackNeighborhoodProcessing({
            schema_version: SCHEMA_VERSION,
            store_path: this.viewerModel.activeStorePath,
            output_store_path: outputStorePath,
            overwrite_existing: overwriteExisting,
            pipeline: clonePostStackNeighborhoodPipeline(this.postStackNeighborhoodPipeline)
          } satisfies RunPostStackNeighborhoodProcessingRequest)
        : this.subvolumeCrop
          ? await runSubvolumeProcessing({
              schema_version: SCHEMA_VERSION,
              store_path: this.viewerModel.activeStorePath,
              output_store_path: outputStorePath,
              overwrite_existing: overwriteExisting,
              pipeline: buildSubvolumeProcessingPipeline(this.pipeline, this.subvolumeCrop)
            } satisfies RunSubvolumeProcessingRequest)
          : await runProcessing({
              schema_version: SCHEMA_VERSION,
              store_path: this.viewerModel.activeStorePath,
              output_store_path: outputStorePath,
              overwrite_existing: overwriteExisting,
              pipeline: clonePipeline(this.pipeline)
            } satisfies RunProcessingRequest);
    this.setActiveJobStatus(response.job);
    this.viewerModel.note(
      "Started full-volume processing job.",
      "backend",
      "info",
      response.job.output_store_path ?? response.job.job_id
    );
    this.scheduleJobPoll();
  }
}

export function describeOperation(operation: WorkspaceOperation): string {
  if (isCropSubvolume(operation)) {
    const { inline_min, inline_max, xline_min, xline_max, z_min_ms, z_max_ms } = operation.crop_subvolume;
    return `crop subvolume (IL ${inline_min}-${inline_max}, XL ${xline_min}-${xline_max}, Z ${z_min_ms}-${z_max_ms} ms)`;
  }
  if (isAmplitudeScalar(operation)) {
    return `amplitude scalar (${operation.amplitude_scalar.factor})`;
  }
  if (isTraceRmsNormalize(operation)) {
    return "trace RMS normalize";
  }
  if (isAgcRms(operation)) {
    return `RMS AGC (${operation.agc_rms.window_ms} ms)`;
  }
  if (isPhaseRotation(operation)) {
    return `phase rotation (${operation.phase_rotation.angle_degrees} deg)`;
  }
  if (isEnvelope(operation)) {
    return "envelope";
  }
  if (isInstantaneousPhase(operation)) {
    return "instantaneous phase";
  }
  if (isInstantaneousFrequency(operation)) {
    return "instantaneous frequency";
  }
  if (isSweetness(operation)) {
    return "sweetness";
  }
  if (isLowpassFilter(operation)) {
    const { f3_hz, f4_hz } = operation.lowpass_filter;
    return `lowpass (${f3_hz}/${f4_hz} Hz)`;
  }
  if (isHighpassFilter(operation)) {
    const { f1_hz, f2_hz } = operation.highpass_filter;
    return `highpass (${f1_hz}/${f2_hz} Hz)`;
  }
  if (isBandpassFilter(operation)) {
    const { f1_hz, f2_hz, f3_hz, f4_hz } = operation.bandpass_filter;
    return `bandpass (${f1_hz}/${f2_hz}/${f3_hz}/${f4_hz} Hz)`;
  }
  if (isVolumeArithmetic(operation)) {
    return `${operation.volume_arithmetic.operator} volume (${volumeStoreLabel(operation.volume_arithmetic.secondary_store_path)})`;
  }
  return "trace-local";
}

export function describeNeighborhoodOperation(operation: NeighborhoodOperation): string {
  if ("similarity" in operation) {
    const { gate_ms, inline_stepout, xline_stepout } = operation.similarity.window;
    return `similarity (${gate_ms} ms, il ${inline_stepout}, xl ${xline_stepout})`;
  }
  if ("local_volume_stats" in operation) {
    const { gate_ms, inline_stepout, xline_stepout } = operation.local_volume_stats.window;
    return `${operation.local_volume_stats.statistic} stats (${gate_ms} ms, il ${inline_stepout}, xl ${xline_stepout})`;
  }
  if ("dip" in operation) {
    const { gate_ms, inline_stepout, xline_stepout } = operation.dip.window;
    return `dip ${operation.dip.output} (${gate_ms} ms, il ${inline_stepout}, xl ${xline_stepout})`;
  }
  return "neighborhood";
}

function pluralizeCount(value: number, noun: string): string {
  return `${value} ${noun}${value === 1 ? "" : "s"}`;
}

function humanizePlanningMode(mode: string): string {
  return mode
    .split("_")
    .filter((token) => token.length > 0)
    .join(" ");
}

export function summarizeProcessingPlan(
  summary: ProcessingJobPlanSummaryViewModel | null | undefined
): ProcessingPlanSummaryView | null {
  if (!summary) {
    return null;
  }

  const overviewParts = [pluralizeCount(summary.stage_count, "stage")];
  if (summary.expected_partition_count !== null) {
    overviewParts.push(`~${pluralizeCount(summary.expected_partition_count, "partition")}`);
  }

  const detailParts = [`Mode: ${humanizePlanningMode(summary.planning_mode)}`];
  if (summary.max_active_partitions !== null) {
    detailParts.push(`max ${pluralizeCount(summary.max_active_partitions, "active partition")}`);
  }

  const stages = summary.stage_labels.map((label, index) => {
    const partitionSummary = summary.stage_partition_summaries[index];
    return partitionSummary ? `${label}: ${partitionSummary}` : label;
  });

  return {
    overview: overviewParts.join(", "),
    detail: detailParts.join(" · "),
    stages
  };
}

function formatPartitionProgress(
  completed: number,
  total: number | null | undefined
): string {
  return total === null || total === undefined
    ? `${completed} partitions`
    : `${completed}/${total} partitions`;
}

export function summarizeProcessingExecution(
  summary: ProcessingJobExecutionSummaryViewModel | null | undefined
): ProcessingExecutionSummaryView | null {
  if (!summary) {
    return null;
  }

  const overviewParts = [
    formatPartitionProgress(summary.completed_partitions, summary.total_partitions)
  ];
  if (summary.active_partitions > 0) {
    overviewParts.push(`${summary.active_partitions} active`);
  }
  if (summary.peak_active_partitions > 0) {
    overviewParts.push(`peak ${summary.peak_active_partitions}`);
  }

  const detailParts: string[] = [];
  if (summary.retry_count > 0) {
    detailParts.push(`${pluralizeCount(summary.retry_count, "retry")}`);
  }

  const stages = summary.stages.map((stage) => {
    const parts = [
      `${stage.stage_label}: ${formatPartitionProgress(stage.completed_partitions, stage.total_partitions)}`
    ];
    if (stage.retry_count > 0) {
      parts.push(`${pluralizeCount(stage.retry_count, "retry")}`);
    }
    return parts.join(", ");
  });

  return {
    overview: overviewParts.join(", "),
    detail: detailParts.length > 0 ? detailParts.join(" · ") : null,
    stages
  };
}

export function isNeighborhoodSimilarity(
  operation: NeighborhoodOperation | null | undefined
): operation is { similarity: { window: PostStackNeighborhoodWindow } } {
  return Boolean(operation && typeof operation === "object" && "similarity" in operation);
}

export function isNeighborhoodLocalVolumeStats(
  operation: NeighborhoodOperation | null | undefined
): operation is {
  local_volume_stats: { window: PostStackNeighborhoodWindow; statistic: LocalVolumeStatistic };
} {
  return Boolean(operation && typeof operation === "object" && "local_volume_stats" in operation);
}

export function isNeighborhoodDip(
  operation: NeighborhoodOperation | null | undefined
): operation is { dip: { window: PostStackNeighborhoodWindow; output: NeighborhoodDipOutput } } {
  return Boolean(operation && typeof operation === "object" && "dip" in operation);
}

export function isCropSubvolume(
  operation: WorkspaceOperation | null | undefined
): operation is { crop_subvolume: SubvolumeCropOperation } {
  return Boolean(operation && typeof operation === "object" && "crop_subvolume" in operation);
}

export function isAmplitudeScalar(
  operation: WorkspaceOperation
): operation is { amplitude_scalar: { factor: number } } {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "amplitude_scalar" in operation;
}

export function isTraceRmsNormalize(
  operation: WorkspaceOperation | null | undefined
): operation is "trace_rms_normalize" {
  return operation === "trace_rms_normalize";
}

export function isAgcRms(
  operation: WorkspaceOperation
): operation is { agc_rms: { window_ms: number } } {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "agc_rms" in operation;
}

export function isBandpassFilter(
  operation: WorkspaceOperation
): operation is {
  bandpass_filter: {
    f1_hz: number;
    f2_hz: number;
    f3_hz: number;
    f4_hz: number;
    phase: "zero";
    window: "cosine_taper";
  };
} {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "bandpass_filter" in operation;
}

export function isLowpassFilter(
  operation: WorkspaceOperation
): operation is {
  lowpass_filter: {
    f3_hz: number;
    f4_hz: number;
    phase: "zero";
    window: "cosine_taper";
  };
} {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "lowpass_filter" in operation;
}

export function isHighpassFilter(
  operation: WorkspaceOperation
): operation is {
  highpass_filter: {
    f1_hz: number;
    f2_hz: number;
    phase: "zero";
    window: "cosine_taper";
  };
} {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "highpass_filter" in operation;
}

export function isPhaseRotation(
  operation: WorkspaceOperation
): operation is {
  phase_rotation: {
    angle_degrees: number;
  };
} {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "phase_rotation" in operation;
}

export function isEnvelope(
  operation: WorkspaceOperation | null | undefined
): operation is "envelope" {
  return operation === "envelope";
}

export function isInstantaneousPhase(
  operation: WorkspaceOperation | null | undefined
): operation is "instantaneous_phase" {
  return operation === "instantaneous_phase";
}

export function isInstantaneousFrequency(
  operation: WorkspaceOperation | null | undefined
): operation is "instantaneous_frequency" {
  return operation === "instantaneous_frequency";
}

export function isSweetness(
  operation: WorkspaceOperation | null | undefined
): operation is "sweetness" {
  return operation === "sweetness";
}

export function isVolumeArithmetic(
  operation: WorkspaceOperation
): operation is {
  volume_arithmetic: {
    operator: VolumeArithmeticOperator;
    secondary_store_path: string;
  };
} {
  return !isCropSubvolume(operation) && typeof operation !== "string" && "volume_arithmetic" in operation;
}

const [internalGetProcessingModelContext, internalSetProcessingModelContext] =
  createContext<ProcessingModel>();

export function getProcessingModelContext(): ProcessingModel {
  const processingModel = internalGetProcessingModelContext();
  if (!processingModel) {
    throw new Error("Processing model context not found");
  }
  return processingModel;
}

export function setProcessingModelContext(processingModel: ProcessingModel): ProcessingModel {
  internalSetProcessingModelContext(processingModel);
  return processingModel;
}
