import { createContext, tick } from "svelte";
import type {
  AmplitudeSpectrumRequest,
  AmplitudeSpectrumResponse,
  PreviewTraceLocalProcessingRequest as PreviewProcessingRequest,
  ProcessingJobStatus,
  SubvolumeCropOperation,
  SubvolumeProcessingPipeline,
  TraceLocalProcessingOperation as ProcessingOperation,
  TraceLocalProcessingPipeline as ProcessingPipeline,
  TraceLocalProcessingStep as ProcessingStep,
  TraceLocalProcessingPreset as ProcessingPreset,
  RunTraceLocalProcessingRequest as RunProcessingRequest,
  RunSubvolumeProcessingRequest,
  SectionView,
  WorkspacePipelineEntry
} from "@traceboost/seis-contracts";
import {
  cancelProcessingJob,
  defaultProcessingStorePath,
  defaultSubvolumeProcessingStorePath,
  deletePipelinePreset,
  emitFrontendDiagnosticsEvent,
  fetchAmplitudeSpectrum,
  getProcessingJob,
  listPipelinePresets,
  previewProcessing,
  runProcessing,
  runSubvolumeProcessing,
  savePipelinePreset,
  type TransportSectionView
} from "./bridge";
import { confirmOverwriteStore, pickOutputStorePath } from "./file-dialog";
import type { ViewerModel } from "./viewer-model.svelte";

type PreviewState = "raw" | "preview" | "stale";
type SpectrumAmplitudeScale = "db" | "linear";
type VolumeArithmeticOperator = "add" | "subtract" | "multiply" | "divide";
type DisplaySectionView = SectionView | TransportSectionView;
export interface SourceSubvolumeBounds {
  inlineMin: number;
  inlineMax: number;
  xlineMin: number;
  xlineMax: number;
  zMinMs: number;
  zMaxMs: number;
  zUnits: string | null;
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
  | "volume_subtract"
  | "volume_add"
  | "volume_multiply"
  | "volume_divide"
  | "crop_subvolume"
  | "lowpass_filter"
  | "highpass_filter"
  | "bandpass_filter";

interface OperatorCatalogDefinition {
  id: OperatorCatalogId;
  label: string;
  description: string;
  keywords: string[];
  shortcut: "a" | "n" | "g" | "h" | "l" | "i" | "b" | "v" | "c" | null;
  create: (viewerModel: ViewerModel) => WorkspaceOperation;
}

interface CopiedSessionPipeline {
  pipeline: ProcessingPipeline;
  subvolumeCrop: SubvolumeCropOperation | null;
}

const OPERATOR_CATALOG: readonly OperatorCatalogDefinition[] = [
  {
    id: "amplitude_scalar",
    label: "Amplitude Scalar",
    description: "Scale trace amplitudes by a constant factor.",
    keywords: ["scalar", "scale", "gain", "amplitude"],
    shortcut: "a",
    create: () => ({ amplitude_scalar: { factor: 1 } })
  },
  {
    id: "trace_rms_normalize",
    label: "Trace RMS Normalize",
    description: "Normalize each trace to unit RMS amplitude.",
    keywords: ["normalize", "rms", "trace", "balance"],
    shortcut: "n",
    create: () => "trace_rms_normalize"
  },
  {
    id: "agc_rms",
    label: "RMS AGC",
    description: "Centered moving-window RMS automatic gain control.",
    keywords: ["agc", "gain", "window", "rms", "balance", "automatic gain control"],
    shortcut: "g",
    create: () => defaultAgcRms()
  },
  {
    id: "phase_rotation",
    label: "Phase Rotation",
    description: "Constant trace phase rotation in degrees.",
    keywords: ["phase", "rotation", "rotate", "constant phase", "quadrature", "hilbert"],
    shortcut: "h",
    create: () => defaultPhaseRotation()
  },
  {
    id: "volume_subtract",
    label: "Subtract Volume",
    description: "Subtract a compatible workspace volume from the active volume.",
    keywords: ["volume", "arithmetic", "subtract", "difference", "minus", "cube"],
    shortcut: "v",
    create: (viewerModel) => defaultVolumeArithmetic(viewerModel, "subtract")
  },
  {
    id: "volume_add",
    label: "Add Volume",
    description: "Add a compatible workspace volume to the active volume sample-by-sample.",
    keywords: ["volume", "arithmetic", "add", "sum", "plus", "cube"],
    shortcut: null,
    create: (viewerModel) => defaultVolumeArithmetic(viewerModel, "add")
  },
  {
    id: "volume_multiply",
    label: "Multiply Volumes",
    description: "Multiply the active volume by another compatible workspace volume.",
    keywords: ["volume", "arithmetic", "multiply", "product", "times", "cube"],
    shortcut: null,
    create: (viewerModel) => defaultVolumeArithmetic(viewerModel, "multiply")
  },
  {
    id: "volume_divide",
    label: "Divide Volumes",
    description: "Divide the active volume by another compatible workspace volume.",
    keywords: ["volume", "arithmetic", "divide", "ratio", "quotient", "cube"],
    shortcut: null,
    create: (viewerModel) => defaultVolumeArithmetic(viewerModel, "divide")
  },
  {
    id: "crop_subvolume",
    label: "Crop Subvolume",
    description: "Write a strict subvolume bounded by inline, xline, and time windows.",
    keywords: ["crop", "subvolume", "subset", "window", "inline", "xline", "time", "cube"],
    shortcut: "c",
    create: (viewerModel) => ({ crop_subvolume: defaultSubvolumeCrop(viewerModel) })
  },
  {
    id: "lowpass_filter",
    label: "Lowpass Filter",
    description: "Zero-phase FFT lowpass with a cosine high-cut taper.",
    keywords: ["lowpass", "filter", "frequency", "spectral", "highcut", "noise"],
    shortcut: "l",
    create: (viewerModel) => defaultLowpassFilter(viewerModel.section)
  },
  {
    id: "highpass_filter",
    label: "Highpass Filter",
    description: "Zero-phase FFT highpass with a cosine low-cut taper.",
    keywords: ["highpass", "filter", "frequency", "spectral", "lowcut", "drift"],
    shortcut: "i",
    create: (viewerModel) => defaultHighpassFilter(viewerModel.section)
  },
  {
    id: "bandpass_filter",
    label: "Bandpass Filter",
    description: "Zero-phase FFT bandpass with cosine tapers.",
    keywords: ["bandpass", "filter", "frequency", "spectral", "highcut", "lowcut"],
    shortcut: "b",
    create: (viewerModel) => defaultBandpassFilter(viewerModel.section)
  }
] as const;

export interface OperatorCatalogItem {
  id: OperatorCatalogId;
  label: string;
  description: string;
  keywords: string[];
  shortcut: "a" | "n" | "g" | "h" | "l" | "i" | "b" | "v" | "c" | null;
}

export const operatorCatalogItems: readonly OperatorCatalogItem[] = OPERATOR_CATALOG.map(
  ({ id, label, description, keywords, shortcut }) => ({
    id,
    label,
    description,
    keywords,
    shortcut
  })
);

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

function pipelineName(pipeline: ProcessingPipeline): string {
  return pipeline.name?.trim() || "Untitled pipeline";
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

function nextDuplicateName(sourceName: string, existingNames: string[]): string {
  const trimmedSourceName = sourceName.trim() || "Pipeline";
  const sourceMatch = /^(.*?)(?:_(\d+))?$/.exec(trimmedSourceName);
  const baseName = sourceMatch?.[1]?.trim() || trimmedSourceName;
  const lowerBaseName = baseName.toLowerCase();
  let maxSuffix = 0;

  for (const existingName of existingNames) {
    const trimmedExistingName = existingName.trim();
    if (!trimmedExistingName) {
      continue;
    }

    const existingMatch = /^(.*?)(?:_(\d+))?$/.exec(trimmedExistingName);
    const existingBaseName = existingMatch?.[1]?.trim() || trimmedExistingName;
    if (existingBaseName.toLowerCase() !== lowerBaseName) {
      continue;
    }

    const suffix = existingMatch?.[2] ? Number(existingMatch[2]) : 0;
    if (Number.isFinite(suffix)) {
      maxSuffix = Math.max(maxSuffix, suffix);
    }
  }

  return `${baseName}_${maxSuffix + 1}`;
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

function defaultPhaseRotation(): ProcessingOperation {
  return {
    phase_rotation: {
      angle_degrees: 0
    }
  };
}

function sectionNyquistHz(section: DisplaySectionView | null): number {
  const sampleAxis = section?.sample_axis_f32le ?? [];
  const sampleIntervalMs =
    sampleAxis.length >= 2 ? Math.abs((sampleAxis[1] ?? 0) - (sampleAxis[0] ?? 0)) : 2;
  const safeSampleIntervalMs =
    Number.isFinite(sampleIntervalMs) && sampleIntervalMs > 0 ? sampleIntervalMs : 2;
  return 500.0 / safeSampleIntervalMs;
}

function defaultAgcRms(): ProcessingOperation {
  return {
    agc_rms: {
      window_ms: 250
    }
  };
}

function defaultLowpassFilter(section: DisplaySectionView | null): ProcessingOperation {
  const nyquistHz = sectionNyquistHz(section);
  const f3_hz = Math.max(20, nyquistHz * 0.12);
  const f4_hz = Math.min(nyquistHz, Math.max(f3_hz + 8, nyquistHz * 0.18));

  return {
    lowpass_filter: {
      f3_hz: Number(f3_hz.toFixed(1)),
      f4_hz: Number(f4_hz.toFixed(1)),
      phase: "zero",
      window: "cosine_taper"
    }
  };
}

function defaultHighpassFilter(section: DisplaySectionView | null): ProcessingOperation {
  const nyquistHz = sectionNyquistHz(section);
  const f1_hz = Math.max(2, nyquistHz * 0.015);
  const f2_hz = Math.min(nyquistHz, Math.max(f1_hz + 2, nyquistHz * 0.04));

  return {
    highpass_filter: {
      f1_hz: Number(f1_hz.toFixed(1)),
      f2_hz: Number(f2_hz.toFixed(1)),
      phase: "zero",
      window: "cosine_taper"
    }
  };
}

function defaultBandpassFilter(section: DisplaySectionView | null): ProcessingOperation {
  const nyquistHz = sectionNyquistHz(section);
  const f1_hz = Math.max(4, nyquistHz * 0.06);
  const f2_hz = Math.max(f1_hz + 1, nyquistHz * 0.1);
  const f4_hz = Math.min(nyquistHz, Math.max(f2_hz + 6, nyquistHz * 0.45));
  const f3_hz = Math.min(f4_hz, Math.max(f2_hz + 4, nyquistHz * 0.32));

  return {
    bandpass_filter: {
      f1_hz: Number(f1_hz.toFixed(1)),
      f2_hz: Number(f2_hz.toFixed(1)),
      f3_hz: Number(f3_hz.toFixed(1)),
      f4_hz: Number(f4_hz.toFixed(1)),
      phase: "zero",
      window: "cosine_taper"
    }
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

function defaultVolumeArithmetic(
  viewerModel: ViewerModel,
  operator: VolumeArithmeticOperator = "subtract"
): ProcessingOperation {
  const secondaryOptions = volumeArithmeticSecondaryOptions(viewerModel);
  return {
    volume_arithmetic: {
      operator,
      secondary_store_path: secondaryOptions[0]?.storePath ?? ""
    }
  };
}

function defaultSubvolumeCrop(viewerModel: ViewerModel): SubvolumeCropOperation {
  const summary = viewerModel.dataset?.descriptor.geometry?.summary;
  const inlineAxis = summary?.inline_axis;
  const xlineAxis = summary?.xline_axis;
  const sampleAxis = summary?.sample_axis;
  return {
    inline_min: inlineAxis?.first ?? 0,
    inline_max: inlineAxis?.last ?? 0,
    xline_min: xlineAxis?.first ?? 0,
    xline_max: xlineAxis?.last ?? 0,
    z_min_ms: sampleAxis?.first ?? 0,
    z_max_ms: sampleAxis?.last ?? 0
  };
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

  pipeline = $state<ProcessingPipeline>(createEmptyPipeline());
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
  spectrumAmplitudeScale = $state<SpectrumAmplitudeScale>("db");
  spectrumBusy = $state(false);
  spectrumStale = $state(false);
  spectrumError = $state<string | null>(null);
  rawSpectrum = $state.raw<AmplitudeSpectrumResponse | null>(null);
  processedSpectrum = $state.raw<AmplitudeSpectrumResponse | null>(null);
  spectrumSectionKey = $state<string | null>(null);
  runBusy = $state(false);
  error = $state<string | null>(null);
  presets = $state.raw<ProcessingPreset[]>([]);
  activeJob = $state<ProcessingJobStatus | null>(null);
  loadingPresets = $state(false);
  runOutputSettingsOpen = $state(false);
  runOutputPathMode = $state<"default" | "custom">("default");
  customRunOutputPath = $state("");
  overwriteExistingRunOutput = $state(false);
  defaultRunOutputPath = $state<string | null>(null);
  resolvingRunOutputPath = $state(false);

  #jobPollTimer: number | null = null;
  #presetCounter = 0;
  #sessionPipelineCounter = 0;
  #hydratedDatasetEntryId: string | null = null;
  #runOutputPathRequestId = 0;
  #copiedSessionPipeline: CopiedSessionPipeline | null = null;
  #copiedOperation: WorkspaceOperation | null = null;
  #persistSessionPipelinesTimer: number | null = null;
  #runOutputPathRefreshTimer: number | null = null;

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
      }
    });

    $effect(() => {
      const activeEntryId = this.viewerModel.activeEntryId;
      const activeEntry = this.viewerModel.activeDatasetEntry;

      if (!activeEntryId || !activeEntry) {
        this.#hydratedDatasetEntryId = null;
        if (!this.sessionPipelines.length) {
          const fallback = this.createSessionPipelineEntry(this.nextEmptySessionPipelineName());
          this.sessionPipelines = [fallback];
          this.activeSessionPipelineId = fallback.pipeline_id;
          this.pipeline = clonePipeline(fallback.pipeline);
          this.subvolumeCrop = cloneSubvolumeCrop(fallback.subvolume_crop);
        }
        return;
      }

       if (this.#hydratedDatasetEntryId === activeEntryId) {
        return;
      }
      this.#hydratedDatasetEntryId = activeEntryId;

      const nextSessionPipelines =
        activeEntry.session_pipelines.length > 0
          ? activeEntry.session_pipelines.map((entry) => ({
              pipeline_id: entry.pipeline_id,
              pipeline: clonePipeline(entry.pipeline),
              subvolume_crop: cloneSubvolumeCrop(entry.subvolume_crop),
              updated_at_unix_s: entry.updated_at_unix_s
            }))
          : [this.createSessionPipelineEntry("Pipeline 1")];
      const activePipelineId =
        activeEntry.active_session_pipeline_id &&
        nextSessionPipelines.some((entry) => entry.pipeline_id === activeEntry.active_session_pipeline_id)
          ? activeEntry.active_session_pipeline_id
          : nextSessionPipelines[0]?.pipeline_id ?? null;
      const activePipeline =
        nextSessionPipelines.find((entry) => entry.pipeline_id === activePipelineId) ?? nextSessionPipelines[0];

      this.sessionPipelines = nextSessionPipelines;
      this.activeSessionPipelineId = activePipeline?.pipeline_id ?? null;
      this.pipeline = clonePipeline(activePipeline?.pipeline ?? createEmptyPipeline());
      this.subvolumeCrop = cloneSubvolumeCrop(activePipeline?.subvolume_crop);
      this.selectedStepIndex = 0;
      this.editingParams = false;
      this.clearPreviewState();
    });

    $effect(() => {
      const runOutputSettingsOpen = this.runOutputSettingsOpen;
      const runOutputPathMode = this.runOutputPathMode;
      const activeStorePath = this.viewerModel.activeStorePath;
      const signature = workspaceRunOutputSignature(this.pipeline, this.subvolumeCrop);

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
        cloneSubvolumeCrop(this.subvolumeCrop),
        signature
      );
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
    };
  };

  get selectedOperation(): WorkspaceOperation | null {
    return workspaceOperationAt(this.pipeline, this.subvolumeCrop, this.selectedStepIndex);
  }

  get activeSessionPipeline(): WorkspacePipelineEntry | null {
    return this.sessionPipelines.find((entry) => entry.pipeline_id === this.activeSessionPipelineId) ?? null;
  }

  private traceOperationIndexForDisplayIndex(displayIndex: number): number | null {
    if (displayIndex < 0 || displayIndex >= this.pipeline.steps.length) {
      return null;
    }
    return displayIndex;
  }

  private nextTraceInsertIndexAfterSelection(): number {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    if (selectedTraceOperationIndex === null) {
      return this.pipeline.steps.length;
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
    return this.sessionPipelines;
  }

  get hasOperations(): boolean {
    return this.pipeline.steps.length > 0 || this.subvolumeCrop !== null;
  }

  get selectedStepLabel(): string | null {
    return this.selectedOperation ? describeOperation(this.selectedOperation) : null;
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
    return this.pipeline.steps.length > 0 && Boolean(this.viewerModel.section && this.viewerModel.activeStorePath);
  }

  get canRun(): boolean {
    return this.hasOperations && Boolean(this.viewerModel.activeStorePath);
  }

  get canInspectSpectrum(): boolean {
    return Boolean(this.viewerModel.section && this.viewerModel.activeStorePath && this.viewerModel.dataset);
  }

  get spectrumSelectionSummary(): string {
    const section = this.viewerModel.section;
    if (!section) {
      return "Open a dataset and load a section to inspect spectra.";
    }

    return `Whole ${this.viewerModel.axis} section ${this.viewerModel.index} · ${section.traces} traces × ${section.samples} samples`;
  }

  get pipelineDirty(): boolean {
    return this.previewState !== "preview";
  }

  get pipelineTitle(): string {
    return pipelineName(this.pipeline);
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

  get hasSubvolumeCrop(): boolean {
    return this.subvolumeCrop !== null;
  }

  get canMoveSelectedUp(): boolean {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    return selectedTraceOperationIndex !== null && selectedTraceOperationIndex > 0;
  }

  get canMoveSelectedDown(): boolean {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    return selectedTraceOperationIndex !== null && selectedTraceOperationIndex < this.pipeline.steps.length - 1;
  }

  get canRemoveSessionPipeline(): boolean {
    return this.sessionPipelines.length > 1;
  }

  get canToggleSelectedCheckpoint(): boolean {
    const selectedTraceOperationIndex = this.selectedTraceOperationIndex();
    return selectedTraceOperationIndex !== null && canCheckpointStepIndex(this.pipeline, selectedTraceOperationIndex, this.subvolumeCrop);
  }

  get selectedStepCheckpoint(): boolean {
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
    return pipelineName(entry.pipeline) || `Pipeline ${index + 1}`;
  };

  setRunOutputSettingsOpen = (open: boolean): void => {
    this.runOutputSettingsOpen = open;
    if (open && this.viewerModel.activeStorePath && !this.defaultRunOutputPath && !this.resolvingRunOutputPath) {
      this.scheduleDefaultRunOutputPathRefresh(
        this.viewerModel.activeStorePath,
        clonePipeline(this.pipeline),
        cloneSubvolumeCrop(this.subvolumeCrop),
        workspaceRunOutputSignature(this.pipeline, this.subvolumeCrop)
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
    const nextEntry = this.createSessionPipelineEntry(this.nextEmptySessionPipelineName());
    this.sessionPipelines = [...this.sessionPipelines, nextEntry];
    this.activeSessionPipelineId = nextEntry.pipeline_id;
    this.pipeline = clonePipeline(nextEntry.pipeline);
    this.subvolumeCrop = cloneSubvolumeCrop(nextEntry.subvolume_crop);
    this.viewerModel.setSelectedPresetId(null);
    this.selectedStepIndex = 0;
    this.editingParams = false;
    this.clearPreviewState();
    this.schedulePersistSessionPipelines();
  };

  duplicateActiveSessionPipeline = (): void => {
    const source = this.activeSessionPipeline;
    if (!source) {
      return;
    }
    const duplicate = this.createCopiedSessionPipelineEntry(
      source.pipeline,
      source.subvolume_crop ?? null
    );
    this.sessionPipelines = [...this.sessionPipelines, duplicate];
    this.activeSessionPipelineId = duplicate.pipeline_id;
    this.pipeline = clonePipeline(duplicate.pipeline);
    this.subvolumeCrop = cloneSubvolumeCrop(duplicate.subvolume_crop);
    this.viewerModel.setSelectedPresetId(null);
    this.selectedStepIndex = 0;
    this.editingParams = false;
    this.clearPreviewState();
    this.schedulePersistSessionPipelines();
  };

  activateSessionPipeline = (pipelineId: string): void => {
    const entry = this.sessionPipelines.find((candidate) => candidate.pipeline_id === pipelineId);
    if (!entry) {
      return;
    }

    this.activeSessionPipelineId = pipelineId;
    this.pipeline = clonePipeline(entry.pipeline);
    this.subvolumeCrop = cloneSubvolumeCrop(entry.subvolume_crop);
    this.viewerModel.setSelectedPresetId(entry.pipeline.preset_id ?? null);
    this.selectedStepIndex = 0;
    this.editingParams = false;
    this.clearPreviewState();
    this.schedulePersistSessionPipelines();
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

    const activeIndex = this.sessionPipelines.findIndex((entry) => entry.pipeline_id === pipelineId);
    if (activeIndex < 0) {
      return;
    }

    const removingActivePipeline = this.activeSessionPipelineId === pipelineId;
    const nextSessionPipelines = this.sessionPipelines.filter((entry) => entry.pipeline_id !== pipelineId);
    this.sessionPipelines = nextSessionPipelines;

    if (removingActivePipeline) {
      const fallbackEntry = nextSessionPipelines[Math.max(0, activeIndex - 1)] ?? nextSessionPipelines[0];
      this.activeSessionPipelineId = fallbackEntry?.pipeline_id ?? null;
      this.pipeline = clonePipeline(fallbackEntry?.pipeline ?? createEmptyPipeline());
      this.subvolumeCrop = cloneSubvolumeCrop(fallbackEntry?.subvolume_crop);
      this.viewerModel.setSelectedPresetId(fallbackEntry?.pipeline.preset_id ?? null);
      this.selectedStepIndex = 0;
      this.editingParams = false;
      this.clearPreviewState();
    }

    this.schedulePersistSessionPipelines();
  };

  private createSessionPipelineEntry(
    suggestedName: string,
    template: ProcessingPipeline = createEmptyPipeline(),
    subvolumeCrop: SubvolumeCropOperation | null = null
  ): WorkspacePipelineEntry {
    this.#sessionPipelineCounter += 1;
    const pipeline = clonePipeline(template);
    pipeline.name = pipeline.name?.trim() || suggestedName;
    return {
      pipeline_id: `session-pipeline-${Date.now()}-${this.#sessionPipelineCounter}`,
      pipeline,
      subvolume_crop: cloneSubvolumeCrop(subvolumeCrop),
      updated_at_unix_s: pipelineTimestamp()
    };
  }

  private nextEmptySessionPipelineName(): string {
    const existingNames = this.sessionPipelines.map((entry) => pipelineName(entry.pipeline).trim().toLowerCase());
    if (!existingNames.includes("pipeline")) {
      return "Pipeline";
    }

    let index = 2;
    while (existingNames.includes(`pipeline ${index}`)) {
      index += 1;
    }
    return `Pipeline ${index}`;
  }

  private createCopiedSessionPipelineEntry(
    source: ProcessingPipeline,
    subvolumeCrop: SubvolumeCropOperation | null
  ): WorkspacePipelineEntry {
    const pipeline = clonePipeline(source);
    pipeline.preset_id = null;
    pipeline.name = nextDuplicateName(
      pipelineName(source),
      this.sessionPipelines.map((entry) => pipelineName(entry.pipeline))
    );
    return this.createSessionPipelineEntry(
      pipeline.name,
      pipeline,
      subvolumeCrop
    );
  }

  copyActiveSessionPipeline = (): void => {
    const activePipeline = this.activeSessionPipeline;
    if (!activePipeline) {
      return;
    }
    this.#copiedSessionPipeline = {
      pipeline: clonePipeline(activePipeline.pipeline),
      subvolumeCrop: cloneSubvolumeCrop(activePipeline.subvolume_crop)
    };
    this.viewerModel.note("Copied active session pipeline.", "ui", "info", pipelineName(activePipeline.pipeline));
  };

  pasteCopiedSessionPipeline = (): void => {
    if (!this.#copiedSessionPipeline) {
      return;
    }

    const duplicate = this.createCopiedSessionPipelineEntry(
      this.#copiedSessionPipeline.pipeline,
      this.#copiedSessionPipeline.subvolumeCrop
    );
    this.sessionPipelines = [...this.sessionPipelines, duplicate];
    this.activeSessionPipelineId = duplicate.pipeline_id;
    this.pipeline = clonePipeline(duplicate.pipeline);
    this.subvolumeCrop = cloneSubvolumeCrop(duplicate.subvolume_crop);
    this.viewerModel.setSelectedPresetId(null);
    this.selectedStepIndex = 0;
    this.editingParams = false;
    this.clearPreviewState();
    this.schedulePersistSessionPipelines();
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
        pipeline: clonePipeline(entry.pipeline),
        subvolume_crop: cloneSubvolumeCrop(entry.subvolume_crop)
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
            pipeline: clonePipeline(snapshot),
            subvolume_crop: cloneSubvolumeCrop(nextSubvolumeCrop),
            updated_at_unix_s: pipelineTimestamp()
          }
        : entry
    );
    this.schedulePersistSessionPipelines();
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
    const operator = OPERATOR_CATALOG.find((candidate) => candidate.id === operatorId);
    if (!operator) {
      return;
    }
    this.insertOperation(operator.create(this.viewerModel));
  };

  insertOperation = (operation: WorkspaceOperation): void => {
    if (isCropSubvolume(operation)) {
      this.insertCropSubvolume(operation.crop_subvolume);
      return;
    }
    const next = clonePipeline(this.pipeline);
    const insertIndex = this.nextTraceInsertIndexAfterSelection();
    const insertDisplayIndex = insertIndex;
    next.steps.splice(insertIndex, 0, createStep(operation));
    next.revision += 1;
    this.updateActiveSessionPipeline(next, this.subvolumeCrop);
    this.selectedStepIndex = insertDisplayIndex;
    this.editingParams = true;
    this.invalidatePreview();
  };

  insertCropSubvolume = (crop: SubvolumeCropOperation = defaultSubvolumeCrop(this.viewerModel)): void => {
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
    const next = clonePipeline(this.pipeline);
    next.steps.splice(traceOperationIndex, 1);
    next.revision += 1;
    this.updateActiveSessionPipeline(next, this.subvolumeCrop);
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
    const next = clonePipeline(this.pipeline);
    const [step] = next.steps.splice(fromIndex, 1);
    next.steps.splice(toIndex, 0, step);
    next.revision += 1;
    this.updateActiveSessionPipeline(next, this.subvolumeCrop);
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
    const next = clonePipeline(this.pipeline);
    const [step] = next.steps.splice(fromIndex, 1);
    next.steps.splice(toIndex, 0, step);
    next.revision += 1;
    this.updateActiveSessionPipeline(next, this.subvolumeCrop);
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
    this.updateActiveSessionPipeline({
      ...clonePipeline(this.pipeline),
      name: value.trim() || null
    });
  };

  setSelectedAmplitudeScalarFactor = (value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isAmplitudeScalar(selected)) {
      return;
    }
    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isAmplitudeScalar(operation)) {
      return;
    }
    operation.amplitude_scalar.factor = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
    this.invalidatePreview();
  };

  setSelectedAgcWindow = (value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isAgcRms(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isAgcRms(operation)) {
      return;
    }

    operation.agc_rms.window_ms = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
    this.invalidatePreview();
  };

  setSelectedLowpassCorner = (corner: "f3_hz" | "f4_hz", value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isLowpassFilter(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isLowpassFilter(operation)) {
      return;
    }

    operation.lowpass_filter[corner] = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
    this.invalidatePreview();
  };

  setSelectedHighpassCorner = (corner: "f1_hz" | "f2_hz", value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isHighpassFilter(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isHighpassFilter(operation)) {
      return;
    }

    operation.highpass_filter[corner] = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
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

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isBandpassFilter(operation)) {
      return;
    }

    operation.bandpass_filter[corner] = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
    this.invalidatePreview();
  };

  setSelectedPhaseRotationAngle = (value: number): void => {
    const selected = this.selectedOperation;
    if (!selected || !isPhaseRotation(selected) || !Number.isFinite(value)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isPhaseRotation(operation)) {
      return;
    }

    operation.phase_rotation.angle_degrees = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
    this.invalidatePreview();
  };

  setSelectedVolumeArithmeticOperator = (value: VolumeArithmeticOperator): void => {
    const selected = this.selectedOperation;
    if (!selected || !isVolumeArithmetic(selected)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isVolumeArithmetic(operation)) {
      return;
    }

    operation.volume_arithmetic.operator = value;
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
    this.invalidatePreview();
  };

  setSelectedVolumeArithmeticSecondaryStorePath = (value: string): void => {
    const selected = this.selectedOperation;
    if (!selected || isCropSubvolume(selected) || !isVolumeArithmetic(selected)) {
      return;
    }

    const next = clonePipeline(this.pipeline);
    const selectedIndex = this.selectedTraceOperationIndex();
    if (selectedIndex === null) {
      return;
    }
    const operation = next.steps[selectedIndex]?.operation;
    if (!isVolumeArithmetic(operation)) {
      return;
    }

    operation.volume_arithmetic.secondary_store_path = value.trim();
    next.revision += 1;
    this.updateActiveSessionPipeline(next);
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

  replacePipeline = (pipeline: ProcessingPipeline): void => {
    this.updateActiveSessionPipeline(clonePipeline(pipeline), null);
    this.selectedStepIndex = 0;
    this.editingParams = false;
    this.invalidatePreview();
  };

  loadPreset = (preset: ProcessingPreset): void => {
    this.replacePipeline(preset.pipeline);
    this.viewerModel.setSelectedPresetId(preset.preset_id);
    this.viewerModel.note("Applied library template to the active pipeline.", "ui", "info", preset.preset_id);
  };

  savePreset = async (): Promise<void> => {
    if (this.subvolumeCrop) {
      this.error = "Crop Subvolume pipelines cannot be saved as library templates.";
      this.viewerModel.note("Failed to save library template.", "ui", "warn", this.error);
      return;
    }
    const presetId =
      normalizePresetId(this.pipeline.preset_id ?? this.pipeline.name ?? `pipeline-${++this.#presetCounter}`) ||
      `pipeline-${++this.#presetCounter}`;
    const preset: ProcessingPreset = {
      preset_id: presetId,
      pipeline: {
        ...clonePipeline(this.pipeline),
        preset_id: presetId
      },
      created_at_unix_s: 0,
      updated_at_unix_s: 0
    };
    try {
      const response = await savePipelinePreset(preset);
      this.updateActiveSessionPipeline(clonePipeline(response.preset.pipeline));
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
        this.pipeline.steps.length === 0 && this.subvolumeCrop
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
    const operatorIds = previewOperationIds(this.pipeline);
    const previewMode = "trace_local";
    try {
      const response = await previewProcessing({
        schema_version: 1,
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
          pipelineRevision: this.pipeline.revision,
          pipelineName: pipelineName(this.pipeline),
          operatorCount: operatorIds.length,
          operatorIds,
          hasRunOnlySubvolumeCrop: this.subvolumeCrop !== null,
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
      this.error = errorMessage(error, "Failed to preview processing pipeline.");
      this.viewerModel.note("Processing preview failed.", "backend", "error", this.error);
    } finally {
      this.previewBusy = false;
    }
  };

  refreshSpectrum = async (): Promise<void> => {
    const currentSection = this.viewerModel.section;
    if (!this.canInspectSpectrum || !this.viewerModel.dataset || !this.viewerModel.activeStorePath || !currentSection) {
      this.spectrumError = "Open a dataset and load a section before inspecting the spectrum.";
      return;
    }

    this.spectrumBusy = true;
    this.spectrumError = null;
    try {
      const baseRequest: AmplitudeSpectrumRequest = {
        schema_version: 1,
        store_path: this.viewerModel.activeStorePath,
        section: {
          dataset_id: this.viewerModel.dataset.descriptor.id,
          axis: this.viewerModel.axis,
          index: this.viewerModel.index
        },
        selection: "whole_section",
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
          : this.subvolumeCrop
            ? await defaultSubvolumeProcessingStorePath(
                this.viewerModel.activeStorePath,
                buildSubvolumeProcessingPipeline(this.pipeline, this.subvolumeCrop)
              )
            : await defaultProcessingStorePath(this.viewerModel.activeStorePath, this.pipeline);
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
              ? this.subvolumeCrop
                ? await defaultSubvolumeProcessingStorePath(
                    this.viewerModel.activeStorePath,
                    buildSubvolumeProcessingPipeline(this.pipeline, this.subvolumeCrop)
                  )
                : await defaultProcessingStorePath(this.viewerModel.activeStorePath, this.pipeline)
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

  cancelActiveJob = async (): Promise<void> => {
    if (!this.activeJob) {
      return;
    }
    try {
      const response = await cancelProcessingJob(this.activeJob.job_id);
      this.activeJob = response.job;
      this.viewerModel.note("Requested processing job cancellation.", "ui", "warn", response.job.job_id);
    } catch (error) {
      this.error = errorMessage(error, "Failed to cancel processing job.");
    }
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
        await this.savePreset();
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

  private async pollActiveJob(): Promise<void> {
    if (!this.activeJob) {
      this.runBusy = false;
      return;
    }
    try {
      const response = await getProcessingJob(this.activeJob.job_id);
      this.activeJob = response.job;
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
    subvolumeCrop: SubvolumeCropOperation | null,
    signature: string
  ): Promise<void> {
    const requestId = ++this.#runOutputPathRequestId;
    this.resolvingRunOutputPath = true;
    try {
      const nextPath = subvolumeCrop
        ? await defaultSubvolumeProcessingStorePath(activeStorePath, buildSubvolumeProcessingPipeline(pipeline, subvolumeCrop))
        : await defaultProcessingStorePath(activeStorePath, pipeline);
      if (
        requestId !== this.#runOutputPathRequestId ||
        activeStorePath !== this.viewerModel.activeStorePath ||
        signature !== workspaceRunOutputSignature(this.pipeline, this.subvolumeCrop)
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
    subvolumeCrop: SubvolumeCropOperation | null,
    signature: string
  ): void {
    if (typeof window === "undefined") {
      void this.refreshDefaultRunOutputPath(activeStorePath, pipeline, subvolumeCrop, signature);
      return;
    }

    if (this.#runOutputPathRefreshTimer !== null) {
      window.clearTimeout(this.#runOutputPathRefreshTimer);
    }

    this.#runOutputPathRefreshTimer = window.setTimeout(() => {
      this.#runOutputPathRefreshTimer = null;
      void this.refreshDefaultRunOutputPath(activeStorePath, pipeline, subvolumeCrop, signature);
    }, RUN_OUTPUT_PATH_REFRESH_DEBOUNCE_MS);
  }

  private async startRunOnVolume(outputStorePath: string, overwriteExisting: boolean): Promise<void> {
    if (!this.viewerModel.activeStorePath) {
      throw new Error("Open a dataset before running processing on the full volume.");
    }

    const response = this.subvolumeCrop
      ? await runSubvolumeProcessing({
          schema_version: 1,
          store_path: this.viewerModel.activeStorePath,
          output_store_path: outputStorePath,
          overwrite_existing: overwriteExisting,
          pipeline: buildSubvolumeProcessingPipeline(this.pipeline, this.subvolumeCrop)
        } satisfies RunSubvolumeProcessingRequest)
      : await runProcessing({
          schema_version: 1,
          store_path: this.viewerModel.activeStorePath,
          output_store_path: outputStorePath,
          overwrite_existing: overwriteExisting,
          pipeline: clonePipeline(this.pipeline)
        } satisfies RunProcessingRequest);
    this.activeJob = response.job;
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
  if (isAgcRms(operation)) {
    return `RMS AGC (${operation.agc_rms.window_ms} ms)`;
  }
  if (isPhaseRotation(operation)) {
    return `phase rotation (${operation.phase_rotation.angle_degrees}°)`;
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
  return "trace RMS normalize";
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
