export type ChartInteractionTool =
  | "pointer"
  | "pan"
  | "zoomRect"
  | "topEdit"
  | "lassoSelect"
  | "orbit"
  | "slice-drag"
  | "crop"
  | "select"
  | "interpret-seed";
export type ChartInteractionToggle = "crosshair";
export type PrimaryInteractionMode = "cursor" | "panZoom" | "zoomRect" | "topEdit" | "lassoSelect";
export type SecondaryInteractionModifier = "crosshair";
export type ChartInteractionTriggerType = "pointer-primary" | "pointer-secondary" | "pointer-move" | "wheel" | "keyboard";
export type ChartInteractionManipulatorId =
  | "viewport-navigation"
  | "crosshair-probe"
  | "lasso-selection"
  | "top-edit"
  | "interpretation-seed";
export type ChartInteractionStyleId =
  | "seismic-navigation"
  | "survey-map-navigation"
  | "well-panel-navigation"
  | "volume-interpretation";
export type ChartInteractionCommandId =
  | "viewport.pan"
  | "viewport.panLeft"
  | "viewport.panRight"
  | "viewport.panUp"
  | "viewport.panDown"
  | "viewport.zoomAtCursor"
  | "selection.primary"
  | "probe.crosshair"
  | "topEdit.begin"
  | "lasso.begin"
  | "session.cancel";

export interface ChartInteractionBinding {
  trigger: ChartInteractionTriggerType;
  primaryMode?: PrimaryInteractionMode;
  modifier?: SecondaryInteractionModifier;
  key?: string;
  command: ChartInteractionCommandId;
}

export interface ChartInteractionCommand {
  type:
    | "setPrimaryMode"
    | "toggleModifier"
    | "beginSession"
    | "updateSession"
    | "commitSession"
    | "cancelSession"
    | "semanticAction";
  command: ChartInteractionCommandId;
  payload?: Record<string, unknown>;
}

export interface ChartInteractionStyle {
  id: ChartInteractionStyleId;
  label: string;
  manipulators: readonly ChartInteractionManipulatorId[];
  bindings: readonly ChartInteractionBinding[];
}

export interface InteractionTrigger {
  type: ChartInteractionTriggerType;
  primaryMode: PrimaryInteractionMode;
  modifiers: readonly SecondaryInteractionModifier[];
  key?: string;
}

export interface ChartInteractionCapabilities<
  TTool extends string = ChartInteractionTool,
  TToggle extends string = ChartInteractionToggle
> {
  tools: TTool[];
  toggles: TToggle[];
}

export interface InteractionCapabilities {
  primaryModes: PrimaryInteractionMode[];
  modifiers: SecondaryInteractionModifier[];
}

export interface InteractionTarget {
  kind:
    | "empty-plot"
    | "map-well"
    | "map-scalar-sample"
    | "map-survey-outline"
    | "curve-sample"
    | "curve-fill-region"
    | "curve-vertex"
    | "point-cloud-sample"
    | "point-observation"
    | "top-marker"
    | "top-line"
    | "seismic-trace-sample"
    | "seismic-trace-event"
    | "seismic-section-sample"
    | "seismic-section-horizon-anchor"
    | "horizon-anchor"
    | "lasso-selection"
    | "empty-scene"
    | "volume-slice-plane"
    | "volume-slice-sample"
    | "volume-horizon-surface"
    | "volume-horizon-contour"
    | "volume-well-trajectory"
    | "volume-well-marker"
    | "volume-annotation";
  chartId?: string;
  wellId?: string;
  trackId?: string;
  seriesId?: string;
  entityId?: string;
  traceIndex?: number;
  sampleIndex?: number;
  nativeDepth?: number;
  panelDepth?: number;
}

export interface LassoPoint {
  x: number;
  y: number;
}

export interface TopEditSession {
  kind: "topEdit";
  target: InteractionTarget;
  originalNativeDepth: number;
  previewNativeDepth: number;
  previewPanelDepth: number;
}

export interface LassoSelectionEntity {
  kind:
    | "curve-sample"
    | "curve-vertex"
    | "point-cloud-sample"
    | "point-observation"
    | "top-marker"
    | "horizon-anchor"
    | "seismic-trace-sample"
    | "seismic-section-sample";
  chartId?: string;
  wellId?: string;
  trackId?: string;
  seriesId?: string;
  entityId?: string;
  sourceIndex?: number;
}

export interface LassoSelectionResult {
  chartId?: string;
  targetKind:
    | "curve-sample"
    | "curve-vertex"
    | "point-cloud-sample"
    | "point-observation"
    | "top-marker"
    | "horizon-anchor"
    | "seismic-trace-sample"
    | "seismic-section-sample";
  entities: LassoSelectionEntity[];
}

export interface LassoSession {
  kind: "lasso";
  points: LassoPoint[];
  selection: LassoSelectionResult | null;
}

export interface ZoomRectSession {
  kind: "zoomRect";
  origin: LassoPoint;
  current: LassoPoint;
}

export type InteractionSession = TopEditSession | LassoSession | ZoomRectSession;

export interface InteractionState {
  capabilities: InteractionCapabilities;
  primaryMode: PrimaryInteractionMode;
  modifiers: SecondaryInteractionModifier[];
  focused: boolean;
  hoverTarget: InteractionTarget | null;
  session: InteractionSession | null;
}

export type InteractionEvent =
  | { type: "modeChange"; primaryMode: PrimaryInteractionMode }
  | { type: "modifierChange"; modifier: SecondaryInteractionModifier; enabled: boolean }
  | { type: "focusChange"; focused: boolean }
  | { type: "hoverTargetChange"; target: InteractionTarget | null }
  | { type: "topEditStart"; session: TopEditSession }
  | { type: "topEditPreview"; session: TopEditSession }
  | { type: "topEditCommit"; session: TopEditSession }
  | { type: "topEditCancel"; session: TopEditSession }
  | { type: "lassoStart"; session: LassoSession }
  | { type: "lassoPreview"; session: LassoSession }
  | { type: "lassoComplete"; session: LassoSession }
  | { type: "lassoCancel"; session: LassoSession }
  | { type: "zoomRectStart"; session: ZoomRectSession }
  | { type: "zoomRectPreview"; session: ZoomRectSession }
  | { type: "zoomRectCommit"; session: ZoomRectSession }
  | { type: "zoomRectCancel"; session: ZoomRectSession };

export function matchesInteractionBinding(binding: ChartInteractionBinding, trigger: InteractionTrigger): boolean {
  if (binding.trigger !== trigger.type) {
    return false;
  }
  if (binding.primaryMode && binding.primaryMode !== trigger.primaryMode) {
    return false;
  }
  if (binding.modifier && !trigger.modifiers.includes(binding.modifier)) {
    return false;
  }
  if (binding.key && binding.key !== trigger.key) {
    return false;
  }
  return true;
}
