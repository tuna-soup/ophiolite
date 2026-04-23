import type { Snippet } from "svelte";
import type {
  GatherAxisKind,
  GatherInteractionChanged,
  GatherProbeChanged,
  GatherView,
  GatherViewport,
  GatherViewportChanged,
  SectionColorMap,
  SectionInteractionChanged,
  SectionPolarity,
  SectionPrimaryMode,
  SectionProbeChanged,
  SectionRenderMode,
  SectionView,
  SectionViewport,
  SectionViewportChanged
} from "@ophiolite/contracts";
import type {
  AvoCartesianViewport,
  CartesianAxisContextTrigger,
  CartesianAxisId,
  CartesianAxisOverrides,
  AvoChiProjectionModel,
  AvoCrossplotModel,
  AvoCrossplotProbe,
  AvoHistogramProbe,
  AvoResponseModel,
  AvoResponseProbe,
  ChartInteractionActionId,
  ChartInteractionToolId,
  InteractionEvent,
  RockPhysicsCrossplotModel,
  RockPhysicsCrossplotProbe,
  RockPhysicsCrossplotViewport,
  SectionHorizonOverlay,
  SectionWellOverlay,
  SectionScalarOverlay,
  VolumeInterpretationEditRequest,
  VolumeInterpretationDeleteRequest,
  VolumeInterpretationInterpretationRequest,
  VolumeInterpretationModel,
  VolumeInterpretationProbe,
  VolumeInterpretationSelectionContext,
  VolumeInterpretationSelection,
  VolumeInterpretationTool,
  VolumeInterpretationView,
  SurveyMapModel,
  SurveyMapProbe,
  SurveyMapViewport,
  WellCorrelationPanelModel,
  WellCorrelationProbe,
  WellCorrelationViewport,
  WellPanelModel
} from "@ophiolite/charts-data-models";
import type { VolumeInterpretationPickDebugSnapshot } from "@ophiolite/charts-renderer";
import { getChartDefinition } from "@ophiolite/charts-data-models";

export type SeismicChartPrimaryMode = "cursor" | "panZoom";
export type SeismicChartColorMap = "grayscale" | "red-white-blue";
export type SeismicChartRenderMode = "heatmap" | "wiggle";
export type SeismicChartPolarity = "normal" | "reversed";
export type SeismicChartTool = "pointer" | "crosshair" | "pan";
export type SeismicChartAction = "fitToData";
export type SeismicChartCompareMode = "single" | "split";
export type SeismicBrowseAxis = "inline" | "xline";
export type SeismicSectionAnalysisKind = "amplitude-spectrum" | "amplitude-distribution";
export type SeismicSectionAnalysisSelectionKind = "whole-section" | "viewport" | "rectangle";
export type SeismicSectionAnalysisSelectionMode = "whole-section" | "viewport";
export type SpectrumAmplitudeScale = "db" | "linear";
export type EncodedSectionBytes = number[] | Uint8Array;
export type EncodedGatherBytes = number[] | Uint8Array;

export interface SpectrumCurve {
  frequenciesHz: number[];
  amplitudes: number[];
}

export interface SpectrumResponseLike {
  curve: SpectrumCurve;
  sampleIntervalMs: number;
  processingLabel?: string | null;
}

export interface DerivedWavelet {
  assumption: "zero_phase";
  dominantFrequencyHz: number | null;
  timesMs: number[];
  amplitudes: number[];
}

export interface AmplitudeDistributionBin {
  start: number;
  end: number;
  count: number;
}

export interface AmplitudeDistributionMarker {
  id: string;
  value: number;
  label?: string;
  color?: string;
}

export interface AmplitudeDistributionResult {
  bins: AmplitudeDistributionBin[];
  count: number;
  min: number;
  max: number;
  mean: number;
  standardDeviation: number;
  median: number;
  rms: number;
}

export interface SectionViewLike extends Omit<
  SectionView,
  "horizontal_axis_f64le" | "inline_axis_f64le" | "xline_axis_f64le" | "sample_axis_f32le" | "amplitudes_f32le"
> {
  horizontal_axis_f64le: EncodedSectionBytes;
  inline_axis_f64le: EncodedSectionBytes | null;
  xline_axis_f64le: EncodedSectionBytes | null;
  sample_axis_f32le: EncodedSectionBytes;
  amplitudes_f32le: EncodedSectionBytes;
  logical_dimensions?: {
    traces: number;
    samples: number;
  };
  window?: {
    trace_start: number;
    trace_end: number;
    sample_start: number;
    sample_end: number;
    lod?: number;
  };
}

export interface GatherViewLike extends Omit<
  GatherView,
  "horizontal_axis_f64le" | "sample_axis_f32le" | "amplitudes_f32le" | "gather_axis_kind"
> {
  gather_axis_kind: GatherAxisKind | "trace_ordinal";
  horizontal_axis_f64le: EncodedGatherBytes;
  sample_axis_f32le: EncodedGatherBytes;
  amplitudes_f32le: EncodedGatherBytes;
}

export interface SeismicChartDisplayTransform {
  gain: number;
  clipMin?: number;
  clipMax?: number;
  renderMode: SeismicChartRenderMode;
  colormap: SeismicChartColorMap;
  polarity: SeismicChartPolarity;
}

export interface SeismicChartInteractionConfig {
  tool?: SeismicChartTool;
}

export interface SeismicChartInteractionCapabilities {
  tools: SeismicChartTool[];
  actions: SeismicChartAction[];
}

export const SEISMIC_CHART_INTERACTION_CAPABILITIES: SeismicChartInteractionCapabilities =
  chartInteractionCapabilities<SeismicChartTool, SeismicChartAction>("seismic-section");

export interface SeismicChartInteractionState {
  capabilities: SeismicChartInteractionCapabilities;
  tool: SeismicChartTool;
}

export interface SeismicChartInteractionEventPayload {
  chartId: string;
  viewId: string;
  event: InteractionEvent;
}

export interface SeismicSectionBrowseCurrent {
  axis: SeismicBrowseAxis;
  index: number;
  value: number;
}

export type SeismicSectionBrowseRequest =
  | {
      kind: "step";
      direction: -1 | 1;
      current: SeismicSectionBrowseCurrent;
      viewport: SectionViewport | null;
      preserveViewport: boolean;
    }
  | {
      kind: "switch-axis";
      axis: SeismicBrowseAxis;
      current: SeismicSectionBrowseCurrent;
      viewport: SectionViewport | null;
      preserveViewport: boolean;
    };

export interface SeismicSectionBrowseConfig {
  enabled: boolean;
  current: SeismicSectionBrowseCurrent | null;
  canStepBackward?: boolean;
  canStepForward?: boolean;
  canSwitchAxis?: boolean;
  pending?: boolean;
  showChrome?: boolean;
  onRequest?: SeismicSectionBrowseRequestHandler;
}

export interface SeismicSectionAnalysisRectangle {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface SeismicSectionWholeSectionAnalysisSelection {
  kind: "whole-section";
}

export interface SeismicSectionViewportAnalysisSelection {
  kind: "viewport";
  viewport: SectionViewport;
}

export interface SeismicSectionRectangleAnalysisSelection {
  kind: "rectangle";
  viewport: SectionViewport | null;
  rectangle: SeismicSectionAnalysisRectangle;
}

export type SeismicSectionAnalysisSelection =
  | SeismicSectionWholeSectionAnalysisSelection
  | SeismicSectionViewportAnalysisSelection
  | SeismicSectionRectangleAnalysisSelection;

export interface SeismicSectionAnalysisRequest {
  kind: SeismicSectionAnalysisKind;
  selection: SeismicSectionAnalysisSelection;
  current: SeismicSectionBrowseCurrent;
  viewport: SectionViewport | null;
}

export interface SeismicSectionAnalysisConfig {
  enabled: boolean;
  spectrumEnabled?: boolean;
  distributionEnabled?: boolean;
  selectionMode?: SeismicSectionAnalysisSelectionMode;
  selectionModes?: readonly SeismicSectionAnalysisSelectionMode[];
  openKinds?: readonly SeismicSectionAnalysisKind[];
  pendingKinds?: readonly SeismicSectionAnalysisKind[];
  showChrome?: boolean;
  onSelectionModeChange?: SeismicSectionAnalysisSelectionModeChangeHandler;
  onRequest?: SeismicSectionAnalysisRequestHandler;
}

export interface SeismicChartOverlayProps {
  stageTopLeft?: Snippet;
  plotTopCenter?: Snippet;
  plotTopRight?: Snippet;
  plotBottomRight?: Snippet;
  plotBottomLeft?: Snippet;
  stageScale?: number;
}

export interface SeismicSectionChartProps extends SeismicChartOverlayProps {
  chartId: string;
  viewId: string;
  section: SectionViewLike | null;
  secondarySection?: SectionViewLike | null;
  sectionScalarOverlays?: readonly SectionScalarOverlay[];
  sectionHorizons?: readonly SectionHorizonOverlay[];
  sectionWellOverlays?: readonly SectionWellOverlay[];
  viewport?: SectionViewport | null;
  displayTransform?: Partial<SeismicChartDisplayTransform>;
  interactions?: SeismicChartInteractionConfig;
  browse?: SeismicSectionBrowseConfig;
  analysis?: SeismicSectionAnalysisConfig;
  compareMode?: SeismicChartCompareMode;
  splitPosition?: number;
  crosshairEnabled?: boolean;
  primaryMode?: SeismicChartPrimaryMode;
  loading?: boolean;
  loadingMessage?: string;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: SeismicSectionViewportChangeHandler;
  onProbeChange?: SeismicSectionProbeChangeHandler;
  onInteractionChange?: SeismicSectionInteractionChangeHandler;
  onInteractionStateChange?: SeismicSectionInteractionStateChangeHandler;
  onInteractionEvent?: SeismicSectionInteractionEventHandler;
  onSplitPositionChange?: SeismicSectionSplitPositionChangeHandler;
}

export type SeismicSectionProbeChangeHandler = (payload: SectionProbeChanged) => void;
export type SeismicSectionViewportChangeHandler = (payload: SectionViewportChanged) => void;
export type SeismicSectionInteractionChangeHandler = (payload: SectionInteractionChanged) => void;
export type SeismicSectionInteractionStateChangeHandler = (payload: SeismicChartInteractionState) => void;
export type SeismicSectionInteractionEventHandler = (payload: SeismicChartInteractionEventPayload) => void;
export type SeismicSectionBrowseRequestHandler = (request: SeismicSectionBrowseRequest) => void;
export type SeismicSectionAnalysisRequestHandler = (request: SeismicSectionAnalysisRequest) => void;
export type SeismicSectionAnalysisSelectionModeChangeHandler = (
  mode: SeismicSectionAnalysisSelectionMode
) => void;
export type SeismicSectionSplitPositionChangeHandler = (splitPosition: number) => void;

export interface SeismicGatherChartProps extends SeismicChartOverlayProps {
  chartId: string;
  viewId: string;
  gather: GatherViewLike | null;
  viewport?: GatherViewport | null;
  displayTransform?: Partial<SeismicChartDisplayTransform>;
  interactions?: SeismicChartInteractionConfig;
  crosshairEnabled?: boolean;
  primaryMode?: SeismicChartPrimaryMode;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: SeismicGatherViewportChangeHandler;
  onProbeChange?: SeismicGatherProbeChangeHandler;
  onInteractionChange?: SeismicGatherInteractionChangeHandler;
  onInteractionStateChange?: SeismicGatherInteractionStateChangeHandler;
  onInteractionEvent?: SeismicGatherInteractionEventHandler;
}

export type SeismicGatherProbeChangeHandler = (payload: GatherProbeChanged) => void;
export type SeismicGatherViewportChangeHandler = (payload: GatherViewportChanged) => void;
export type SeismicGatherInteractionChangeHandler = (payload: GatherInteractionChanged) => void;
export type SeismicGatherInteractionStateChangeHandler = (payload: SeismicChartInteractionState) => void;
export type SeismicGatherInteractionEventHandler = (payload: SeismicChartInteractionEventPayload) => void;

export type WellCorrelationChartTool = "pointer" | "crosshair" | "pan";
export type WellCorrelationChartAction = "fitToData";

export interface WellCorrelationChartInteractionConfig {
  tool?: WellCorrelationChartTool;
}

export interface WellCorrelationChartInteractionCapabilities {
  tools: WellCorrelationChartTool[];
  actions: WellCorrelationChartAction[];
}

export const WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES: WellCorrelationChartInteractionCapabilities =
  chartInteractionCapabilities<WellCorrelationChartTool, WellCorrelationChartAction>("well-correlation-panel");

export interface WellCorrelationChartInteractionState {
  capabilities: WellCorrelationChartInteractionCapabilities;
  tool: WellCorrelationChartTool;
}

export interface WellCorrelationViewportChangePayload {
  chartId: string;
  viewport: WellCorrelationViewport | null;
}

export interface WellCorrelationProbeChangePayload {
  chartId: string;
  probe: WellCorrelationProbe | null;
}

export interface WellCorrelationInteractionEventPayload {
  chartId: string;
  event: InteractionEvent;
}

export interface WellCorrelationDebugSnapshot {
  sessionId: number;
  reason: string;
  mount: {
    startedAtMs: number;
    firstViewportAtMs: number | null;
    overlayReadyAtMs: number | null;
    ageMs: number;
  };
  sync: {
    count: number;
    lastDurationMs: number | null;
    totalDurationMs: number;
  };
  overlay: {
    normalizeCount: number;
    lastNormalizeMs: number | null;
    layoutCount: number;
    lastLayoutMs: number | null;
    viewportReady: boolean;
    columns: number;
    clipWidth: number | null;
  };
  renderer: {
    renderCount: number;
    lastRenderMs: number | null;
    baseChanged: boolean | null;
    overlayDraw: boolean | null;
    contentWidth: number | null;
    viewportWidth: number | null;
  };
  state: {
    wells: number;
    viewport: WellCorrelationViewport | null;
    probeTrackId: string | null;
    scrollLeft: number;
  };
}

export interface WellCorrelationChartOverlayProps {
  stageTopLeft?: Snippet;
  plotTopCenter?: Snippet;
  plotTopRight?: Snippet;
  plotBottomRight?: Snippet;
  plotBottomLeft?: Snippet;
  stageScale?: number;
}

export interface WellCorrelationPanelChartProps extends WellCorrelationChartOverlayProps {
  chartId: string;
  panel: WellCorrelationPanelModel | WellPanelModel | null;
  viewport?: WellCorrelationViewport | null;
  interactions?: WellCorrelationChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: WellCorrelationViewportChangeHandler;
  onProbeChange?: WellCorrelationProbeChangeHandler;
  onInteractionStateChange?: WellCorrelationInteractionStateChangeHandler;
  onInteractionEvent?: WellCorrelationInteractionEventHandler;
}

export type WellCorrelationViewportChangeHandler = (payload: WellCorrelationViewportChangePayload) => void;
export type WellCorrelationProbeChangeHandler = (payload: WellCorrelationProbeChangePayload) => void;
export type WellCorrelationInteractionStateChangeHandler = (payload: WellCorrelationChartInteractionState) => void;
export type WellCorrelationInteractionEventHandler = (payload: WellCorrelationInteractionEventPayload) => void;

export type SurveyMapChartTool = "pointer" | "pan";
export type SurveyMapChartAction = "fitToData";

export interface SurveyMapChartInteractionConfig {
  tool?: SurveyMapChartTool;
}

export interface SurveyMapChartInteractionCapabilities {
  tools: SurveyMapChartTool[];
  actions: SurveyMapChartAction[];
}

export const SURVEY_MAP_CHART_INTERACTION_CAPABILITIES: SurveyMapChartInteractionCapabilities =
  chartInteractionCapabilities<SurveyMapChartTool, SurveyMapChartAction>("survey-map");

export interface SurveyMapChartInteractionState {
  capabilities: SurveyMapChartInteractionCapabilities;
  tool: SurveyMapChartTool;
}

export interface SurveyMapViewportChangePayload {
  chartId: string;
  viewport: SurveyMapViewport | null;
}

export interface SurveyMapProbeChangePayload {
  chartId: string;
  probe: SurveyMapProbe | null;
}

export interface SurveyMapSelectionChangePayload {
  chartId: string;
  wellId: string | null;
}

export interface SurveyMapInteractionEventPayload {
  chartId: string;
  event: InteractionEvent;
}

export interface CartesianAxisContextRequestPayload {
  chartId: string;
  axis: CartesianAxisId;
  trigger: CartesianAxisContextTrigger;
  clientX: number;
  clientY: number;
  stageX: number;
  stageY: number;
}

export interface SurveyMapChartOverlayProps {
  stageTopLeft?: Snippet;
  plotTopCenter?: Snippet;
  plotTopRight?: Snippet;
  plotBottomRight?: Snippet;
  plotBottomLeft?: Snippet;
  stageScale?: number;
}

export interface SurveyMapChartProps extends SurveyMapChartOverlayProps {
  chartId: string;
  map: SurveyMapModel | null;
  viewport?: SurveyMapViewport | null;
  interactions?: SurveyMapChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: SurveyMapViewportChangeHandler;
  onProbeChange?: SurveyMapProbeChangeHandler;
  onSelectionChange?: SurveyMapSelectionChangeHandler;
  onInteractionStateChange?: SurveyMapInteractionStateChangeHandler;
  onInteractionEvent?: SurveyMapInteractionEventHandler;
}

export type SurveyMapViewportChangeHandler = (payload: SurveyMapViewportChangePayload) => void;
export type SurveyMapProbeChangeHandler = (payload: SurveyMapProbeChangePayload) => void;
export type SurveyMapSelectionChangeHandler = (payload: SurveyMapSelectionChangePayload) => void;
export type SurveyMapInteractionStateChangeHandler = (payload: SurveyMapChartInteractionState) => void;
export type SurveyMapInteractionEventHandler = (payload: SurveyMapInteractionEventPayload) => void;

export type RockPhysicsCrossplotChartTool = "pointer" | "crosshair" | "pan";
export type RockPhysicsCrossplotChartAction = "fitToData";

export interface RockPhysicsCrossplotChartInteractionConfig {
  tool?: RockPhysicsCrossplotChartTool;
}

export interface RockPhysicsCrossplotChartInteractionCapabilities {
  tools: RockPhysicsCrossplotChartTool[];
  actions: RockPhysicsCrossplotChartAction[];
}

export const ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES: RockPhysicsCrossplotChartInteractionCapabilities =
  chartInteractionCapabilities<RockPhysicsCrossplotChartTool, RockPhysicsCrossplotChartAction>("rock-physics-crossplot");

export interface RockPhysicsCrossplotChartInteractionState {
  capabilities: RockPhysicsCrossplotChartInteractionCapabilities;
  tool: RockPhysicsCrossplotChartTool;
}

export interface RockPhysicsCrossplotViewportChangePayload {
  chartId: string;
  viewport: RockPhysicsCrossplotViewport | null;
}

export interface RockPhysicsCrossplotProbeChangePayload {
  chartId: string;
  probe: RockPhysicsCrossplotProbe | null;
}

export interface RockPhysicsCrossplotInteractionEventPayload {
  chartId: string;
  event: InteractionEvent;
}

export interface RockPhysicsCrossplotAxisOverridesChangePayload {
  chartId: string;
  axisOverrides: CartesianAxisOverrides;
}

export interface RockPhysicsCrossplotChartOverlayProps {
  stageTopLeft?: Snippet;
  plotTopCenter?: Snippet;
  plotTopRight?: Snippet;
  plotBottomRight?: Snippet;
  plotBottomLeft?: Snippet;
  stageScale?: number;
}

export interface RockPhysicsCrossplotChartProps extends RockPhysicsCrossplotChartOverlayProps {
  chartId: string;
  model: RockPhysicsCrossplotModel | null;
  viewport?: RockPhysicsCrossplotViewport | null;
  axisOverrides?: CartesianAxisOverrides;
  interactions?: RockPhysicsCrossplotChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: RockPhysicsCrossplotViewportChangeHandler;
  onProbeChange?: RockPhysicsCrossplotProbeChangeHandler;
  onInteractionStateChange?: RockPhysicsCrossplotInteractionStateChangeHandler;
  onInteractionEvent?: RockPhysicsCrossplotInteractionEventHandler;
  onAxisOverridesChange?: RockPhysicsCrossplotAxisOverridesChangeHandler;
  onAxisContextRequest?: RockPhysicsCrossplotAxisContextRequestHandler;
}

export type RockPhysicsCrossplotViewportChangeHandler = (payload: RockPhysicsCrossplotViewportChangePayload) => void;
export type RockPhysicsCrossplotProbeChangeHandler = (payload: RockPhysicsCrossplotProbeChangePayload) => void;
export type RockPhysicsCrossplotInteractionStateChangeHandler = (payload: RockPhysicsCrossplotChartInteractionState) => void;
export type RockPhysicsCrossplotInteractionEventHandler = (payload: RockPhysicsCrossplotInteractionEventPayload) => void;
export type RockPhysicsCrossplotAxisOverridesChangeHandler = (
  payload: RockPhysicsCrossplotAxisOverridesChangePayload
) => void;
export type RockPhysicsCrossplotAxisContextRequestHandler = (
  payload: CartesianAxisContextRequestPayload
) => void;

export type VolumeInterpretationChartTool = VolumeInterpretationTool;
export type VolumeInterpretationChartAction = "fitToData" | "topView" | "sideView" | "centerSelection";
export type VolumeInterpretationChartRenderer = "vtk" | "placeholder";

export interface VolumeInterpretationChartInteractionConfig {
  tool?: VolumeInterpretationChartTool;
}

export interface VolumeInterpretationChartInteractionCapabilities {
  tools: VolumeInterpretationChartTool[];
  actions: VolumeInterpretationChartAction[];
}

export const VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES: VolumeInterpretationChartInteractionCapabilities =
  chartInteractionCapabilities<VolumeInterpretationChartTool, VolumeInterpretationChartAction>("volume-interpretation");

export interface VolumeInterpretationChartInteractionState {
  capabilities: VolumeInterpretationChartInteractionCapabilities;
  tool: VolumeInterpretationChartTool;
  selectionContext: VolumeInterpretationSelectionContext | null;
}

export interface VolumeInterpretationProbeChangePayload {
  chartId: string;
  probe: VolumeInterpretationProbe | null;
}

export interface VolumeInterpretationSelectionChangePayload {
  chartId: string;
  selection: VolumeInterpretationSelection | null;
}

export interface VolumeInterpretationViewStateChangePayload {
  chartId: string;
  view: VolumeInterpretationView | null;
}

export interface VolumeInterpretationInterpretationRequestPayload {
  chartId: string;
  request: VolumeInterpretationInterpretationRequest;
}

export interface VolumeInterpretationDeleteRequestPayload {
  chartId: string;
  request: VolumeInterpretationDeleteRequest;
}

export interface VolumeInterpretationEditRequestPayload {
  chartId: string;
  request: VolumeInterpretationEditRequest;
}

export interface VolumeInterpretationDebugPickPayload {
  chartId: string;
  phase: "primary" | "secondary";
  stageX: number;
  stageY: number;
  snapshot: VolumeInterpretationPickDebugSnapshot;
}

export interface VolumeInterpretationInteractionEventPayload {
  chartId: string;
  event: InteractionEvent;
}

export interface VolumeInterpretationChartOverlayProps {
  stageTopLeft?: Snippet;
  plotTopCenter?: Snippet;
  plotTopRight?: Snippet;
  plotBottomRight?: Snippet;
  plotBottomLeft?: Snippet;
  stageScale?: number;
}

export interface VolumeInterpretationChartProps extends VolumeInterpretationChartOverlayProps {
  chartId: string;
  model: VolumeInterpretationModel | null;
  tool?: VolumeInterpretationChartTool;
  renderer?: VolumeInterpretationChartRenderer;
  interactions?: VolumeInterpretationChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onProbeChange?: VolumeInterpretationProbeChangeHandler;
  onSelectionChange?: VolumeInterpretationSelectionChangeHandler;
  onViewStateChange?: VolumeInterpretationViewStateChangeHandler;
  onInteractionStateChange?: VolumeInterpretationInteractionStateChangeHandler;
  onInteractionEvent?: VolumeInterpretationInteractionEventHandler;
  onEditRequest?: VolumeInterpretationEditRequestHandler;
  onDeleteRequest?: VolumeInterpretationDeleteRequestHandler;
  onDebugPick?: VolumeInterpretationDebugPickHandler;
  onInterpretationRequest?: VolumeInterpretationInterpretationRequestHandler;
}

export type VolumeInterpretationProbeChangeHandler = (payload: VolumeInterpretationProbeChangePayload) => void;
export type VolumeInterpretationSelectionChangeHandler = (payload: VolumeInterpretationSelectionChangePayload) => void;
export type VolumeInterpretationViewStateChangeHandler = (payload: VolumeInterpretationViewStateChangePayload) => void;
export type VolumeInterpretationInteractionStateChangeHandler = (
  payload: VolumeInterpretationChartInteractionState
) => void;
export type VolumeInterpretationInteractionEventHandler = (payload: VolumeInterpretationInteractionEventPayload) => void;
export type VolumeInterpretationEditRequestHandler = (payload: VolumeInterpretationEditRequestPayload) => void;
export type VolumeInterpretationDeleteRequestHandler = (payload: VolumeInterpretationDeleteRequestPayload) => void;
export type VolumeInterpretationDebugPickHandler = (payload: VolumeInterpretationDebugPickPayload) => void;
export type VolumeInterpretationInterpretationRequestHandler = (
  payload: VolumeInterpretationInterpretationRequestPayload
) => void;

export type AvoChartTool = "pointer" | "crosshair" | "pan";
export type AvoChartAction = "fitToData";

export interface AvoChartInteractionConfig {
  tool?: AvoChartTool;
}

export interface AvoChartInteractionCapabilities {
  tools: AvoChartTool[];
  actions: AvoChartAction[];
}

export const AVO_CHART_INTERACTION_CAPABILITIES: AvoChartInteractionCapabilities =
  chartInteractionCapabilities<AvoChartTool, AvoChartAction>("avo-intercept-gradient-crossplot");

export interface AvoChartInteractionState {
  capabilities: AvoChartInteractionCapabilities;
  tool: AvoChartTool;
}

export interface AvoViewportChangePayload {
  chartId: string;
  viewport: AvoCartesianViewport | null;
}

export interface AvoCrossplotProbeChangePayload {
  chartId: string;
  probe: AvoCrossplotProbe | null;
}

export interface AvoResponseProbeChangePayload {
  chartId: string;
  probe: AvoResponseProbe | null;
}

export interface AvoHistogramProbeChangePayload {
  chartId: string;
  probe: AvoHistogramProbe | null;
}

export interface AvoInteractionEventPayload {
  chartId: string;
  event: InteractionEvent;
}

export interface AvoAxisOverridesChangePayload {
  chartId: string;
  axisOverrides: CartesianAxisOverrides;
}

export interface AvoChartOverlayProps {
  stageTopLeft?: Snippet;
  plotTopCenter?: Snippet;
  plotTopRight?: Snippet;
  plotBottomRight?: Snippet;
  plotBottomLeft?: Snippet;
  stageScale?: number;
}

export interface AvoInterceptGradientCrossplotChartProps extends AvoChartOverlayProps {
  chartId: string;
  model: AvoCrossplotModel | null;
  viewport?: AvoCartesianViewport | null;
  axisOverrides?: CartesianAxisOverrides;
  interactions?: AvoChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: AvoViewportChangeHandler;
  onProbeChange?: AvoCrossplotProbeChangeHandler;
  onInteractionStateChange?: AvoInteractionStateChangeHandler;
  onInteractionEvent?: AvoInteractionEventHandler;
  onAxisOverridesChange?: AvoAxisOverridesChangeHandler;
  onAxisContextRequest?: AvoAxisContextRequestHandler;
}

export interface AvoResponseChartProps extends AvoChartOverlayProps {
  chartId: string;
  model: AvoResponseModel | null;
  viewport?: AvoCartesianViewport | null;
  axisOverrides?: CartesianAxisOverrides;
  interactions?: AvoChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: AvoViewportChangeHandler;
  onProbeChange?: AvoResponseProbeChangeHandler;
  onInteractionStateChange?: AvoInteractionStateChangeHandler;
  onInteractionEvent?: AvoInteractionEventHandler;
  onAxisOverridesChange?: AvoAxisOverridesChangeHandler;
  onAxisContextRequest?: AvoAxisContextRequestHandler;
}

export interface AvoChiProjectionHistogramChartProps extends AvoChartOverlayProps {
  chartId: string;
  model: AvoChiProjectionModel | null;
  viewport?: AvoCartesianViewport | null;
  axisOverrides?: CartesianAxisOverrides;
  interactions?: AvoChartInteractionConfig;
  loading?: boolean;
  emptyMessage?: string;
  errorMessage?: string | null;
  resetToken?: string | number | null;
  onViewportChange?: AvoViewportChangeHandler;
  onProbeChange?: AvoHistogramProbeChangeHandler;
  onInteractionStateChange?: AvoInteractionStateChangeHandler;
  onInteractionEvent?: AvoInteractionEventHandler;
  onAxisOverridesChange?: AvoAxisOverridesChangeHandler;
  onAxisContextRequest?: AvoAxisContextRequestHandler;
}

export type AvoViewportChangeHandler = (payload: AvoViewportChangePayload) => void;
export type AvoCrossplotProbeChangeHandler = (payload: AvoCrossplotProbeChangePayload) => void;
export type AvoResponseProbeChangeHandler = (payload: AvoResponseProbeChangePayload) => void;
export type AvoHistogramProbeChangeHandler = (payload: AvoHistogramProbeChangePayload) => void;
export type AvoInteractionStateChangeHandler = (payload: AvoChartInteractionState) => void;
export type AvoInteractionEventHandler = (payload: AvoInteractionEventPayload) => void;
export type AvoAxisOverridesChangeHandler = (payload: AvoAxisOverridesChangePayload) => void;
export type AvoAxisContextRequestHandler = (payload: CartesianAxisContextRequestPayload) => void;

export interface SeismicSectionChartHandle {
  fitToData(): void;
  resetView(): void;
  setViewport(nextViewport: NonNullable<SeismicSectionChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaTrace: number, deltaSample: number): void;
  setSplitRatio(nextSplitPosition: number): void;
}

export interface SeismicGatherChartHandle {
  fitToData(): void;
  resetView(): void;
  setViewport(nextViewport: NonNullable<SeismicGatherChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaTrace: number, deltaSample: number): void;
}

export interface WellCorrelationPanelChartHandle {
  fitToData(): void;
  setViewport(nextViewport: NonNullable<WellCorrelationPanelChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaDepth: number): void;
  getDebugSnapshot(): WellCorrelationDebugSnapshot | null;
}

export interface SurveyMapChartHandle {
  fitToData(): void;
  setViewport(nextViewport: NonNullable<SurveyMapChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
}

export interface RockPhysicsCrossplotChartHandle {
  fitToData(): void;
  setViewport(nextViewport: NonNullable<RockPhysicsCrossplotChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaX: number, deltaY: number): void;
}

export interface AvoResponseChartHandle {
  fitToData(): void;
  setViewport(nextViewport: NonNullable<AvoResponseChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaX: number, deltaY: number): void;
}

export interface AvoInterceptGradientCrossplotChartHandle {
  fitToData(): void;
  setViewport(nextViewport: NonNullable<AvoInterceptGradientCrossplotChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaX: number, deltaY: number): void;
}

export interface AvoChiProjectionHistogramChartHandle {
  fitToData(): void;
  setViewport(nextViewport: NonNullable<AvoChiProjectionHistogramChartProps["viewport"]>): void;
  zoomBy(factor: number): void;
  panBy(deltaX: number, deltaY: number): void;
}

export interface VolumeInterpretationChartHandle {
  fitToData(): void;
  resetView(): void;
  topView(): void;
  sideView(): void;
  centerSelection(): void;
  zoomBy(factor: number): void;
  orbitBy(deltaYawDeg: number, deltaPitchDeg: number): void;
  panBy(deltaX: number, deltaY: number): void;
}

function chartInteractionCapabilities<TTool extends ChartInteractionToolId, TAction extends ChartInteractionActionId>(
  chartId:
    | "seismic-section"
    | "well-correlation-panel"
    | "survey-map"
    | "rock-physics-crossplot"
    | "volume-interpretation"
    | "avo-response-plot"
    | "avo-intercept-gradient-crossplot"
    | "avo-chi-projection-histogram"
): { tools: TTool[]; actions: TAction[] } {
  const { interactionProfile } = getChartDefinition(chartId);
  return {
    tools: [...interactionProfile.tools] as TTool[],
    actions: [...interactionProfile.actions] as TAction[]
  };
}
