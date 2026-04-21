<svelte:options runes={true} />

<script lang="ts">
  import {
    ALL_ROCK_PHYSICS_TEMPLATE_IDS,
    createMockAvoChiProjectionModel,
    createMockAvoCrossplotModel,
    createMockAvoResponseModel,
    MOCK_SECTION_VELOCITY_MODEL_LABEL,
    STANDARD_ROCK_PHYSICS_TEMPLATE_IDS,
    type AvoCartesianViewport,
    type CartesianAxisId,
    type CartesianAxisOverrides,
    type AvoChiProjectionModel,
    type AvoCrossplotModel,
    type AvoCrossplotProbe,
    type AvoHistogramProbe,
    type AvoResponseModel,
    type AvoResponseProbe,
    createMockVolumeInterpretationModel,
    createMockRockPhysicsCrossplotModel,
    createMockGatherView,
    createMockSurveyMap,
    createMockWellPanel,
    createMockSection,
    createMockSectionHorizons,
    createMockSectionScalarOverlays,
    createMockSectionWellOverlays,
    getDefaultRockPhysicsMockColorMode,
    getRockPhysicsMockColorModes,
    getRockPhysicsTemplateSpec,
    type MockSectionDomain,
    type MockSectionKind,
    type RockPhysicsCrossplotModel,
    type RockPhysicsCrossplotProbe,
    type RockPhysicsTemplateId,
    type RockPhysicsCrossplotViewport,
    type RockPhysicsMockColorMode,
    type RockPhysicsMockOptions,
    ROCK_PHYSICS_TEMPLATE_GROUPS,
    type SectionHorizonOverlay,
    type SectionScalarOverlay,
    type SectionScalarOverlayColorMap,
    type SectionWellOverlay,
    type SurveyMapModel,
    type SurveyMapProbe,
    type SurveyMapViewport,
    type VolumeInterpretationAxis,
    type VolumeInterpretationColorMap,
    type VolumeInterpretationEditRequest,
    type VolumeInterpretationDeleteRequest,
    type VolumeInterpretationInterpretationRequest,
    type VolumeInterpretationModel,
    type VolumeInterpretationProbe,
    type VolumeInterpretationSelection,
    type VolumeInterpretationView,
    type WellPanelModel,
    type WellCorrelationProbe,
    type WellCorrelationViewport
  } from "@ophiolite/charts-data-models";
  import { PLOT_MARGIN } from "@ophiolite/charts-renderer";
  import {
    AVO_CHART_INTERACTION_CAPABILITIES,
    AvoChiProjectionHistogramChart,
    AvoInterceptGradientCrossplotChart,
    AvoResponseChart,
    ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES,
    SEISMIC_CHART_INTERACTION_CAPABILITIES,
    SURVEY_MAP_CHART_INTERACTION_CAPABILITIES,
    VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES,
    WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES,
    RockPhysicsCrossplotChart,
    SeismicGatherChart,
    SeismicSectionChart,
    SurveyMapChart,
    VolumeInterpretationChart,
    WellCorrelationPanelChart,
    type AvoChartAction,
    type AvoChiProjectionHistogramChartHandle,
    type AvoChartInteractionConfig,
    type AvoChartInteractionState,
    type AvoInterceptGradientCrossplotChartHandle,
    type AvoResponseChartHandle,
    type AvoChartTool,
    type CartesianAxisContextRequestPayload,
    type RockPhysicsCrossplotChartHandle,
    type RockPhysicsCrossplotChartAction,
    type RockPhysicsCrossplotChartInteractionConfig,
    type RockPhysicsCrossplotChartInteractionState,
    type RockPhysicsCrossplotChartTool,
    type SeismicGatherChartHandle,
    type SeismicSectionChartHandle,
    type SeismicChartAction,
    type SeismicChartInteractionConfig,
    type SeismicChartInteractionState,
    type SeismicChartTool,
    type SurveyMapChartHandle,
    type SurveyMapChartAction,
    type SurveyMapChartInteractionConfig,
    type SurveyMapChartInteractionState,
    type SurveyMapChartTool,
    type VolumeInterpretationChartAction,
    type VolumeInterpretationChartHandle,
    type VolumeInterpretationDebugPickPayload,
    type VolumeInterpretationChartInteractionConfig,
    type VolumeInterpretationChartInteractionState,
    type VolumeInterpretationChartRenderer,
    type VolumeInterpretationChartTool,
    type WellCorrelationPanelChartHandle,
    type WellCorrelationChartAction,
    type WellCorrelationDebugSnapshot,
    type WellCorrelationChartInteractionConfig,
    type WellCorrelationChartInteractionState,
    type WellCorrelationChartTool
  } from "@ophiolite/charts";
  import {
    ChartInteractionToolbar,
    type ToolbarIconName,
    type ChartToolbarActionItem,
    type ChartToolbarToolItem
  } from "@ophiolite/charts-toolbar";
  import type {
    GatherProbeChanged,
    GatherView,
    GatherViewportChanged,
    SectionColorMap,
    SectionRenderMode,
    SectionView as OphioliteSectionView,
    SectionViewportChanged
  } from "@ophiolite/contracts";
  import { demoCssVars } from "./demo-presentation";

  type DemoSeismicBaseRenderer = "auto" | "worker-webgl" | "local-webgl" | "local-canvas";
  type AxisEditorChartKey = "rockPhysics" | "avoResponse" | "avoCrossplot" | "avoChi";

  interface AxisEditorFormState {
    label: string;
    unit: string;
    min: string;
    max: string;
    tickCount: string;
    tickFormat: string;
  }

  interface AxisEditorState {
    chartKey: AxisEditorChartKey;
    chartLabel: string;
    axis: CartesianAxisId;
    clientX: number;
    clientY: number;
    form: AxisEditorFormState;
  }

  type DemoRoute = "seismic" | "gather" | "survey-map" | "rock-physics" | "volume" | "avo" | "well-panel";
  type DemoMode = "playground" | "public" | "embed";

  const DEMO_LABELS: Record<DemoRoute, string> = {
    seismic: "Seismic Section",
    gather: "Prestack Gather",
    "survey-map": "Survey Map",
    "rock-physics": "Rock Physics",
    volume: "Volume Interpretation",
    avo: "AVO",
    "well-panel": "Well Panel"
  };

  const LAUNCH_DEMO_ROUTES = [
    "seismic",
    "gather",
    "survey-map",
    "rock-physics",
    "well-panel"
  ] as const satisfies readonly DemoRoute[];

  const PUBLIC_DEMO_META: Record<
    DemoRoute,
    {
      eyebrow: string;
      title: string;
      summary: string;
    }
  > = {
    seismic: {
      eyebrow: "Early Access Example",
      title: "Seismic Section",
      summary: "A focused seismic section example with shared probe behavior, overlays, and heatmap or wiggle presentation."
    },
    gather: {
      eyebrow: "Early Access Example",
      title: "Prestack Gather",
      summary: "A prestack gather example over the shared seismic interaction model, centered on view control and probe readout."
    },
    "survey-map": {
      eyebrow: "Early Access Example",
      title: "Survey Map",
      summary: "A plan-view chart for survey footprints, well locations, trajectories, and optional scalar context."
    },
    "rock-physics": {
      eyebrow: "Early Access Example",
      title: "Rock Physics Crossplot",
      summary: "A dense point-cloud example with template-scoped semantics, probe callbacks, and host-owned axis workflows."
    },
    volume: {
      eyebrow: "Preview Example",
      title: "Volume Interpretation",
      summary: "A 3D scene and interpretation preview that remains outside the launch product story while the public boundary hardens."
    },
    avo: {
      eyebrow: "Preview Example",
      title: "AVO",
      summary: "A collection of AVO examples aligned to the shared cartesian wrapper direction but not part of the launch marketing core."
    },
    "well-panel": {
      eyebrow: "Early Access Example",
      title: "Well Correlation Panel",
      summary: "A depth-aligned multi-well example with explicit panel semantics for scientific interpretation workflows."
    }
  };

  let displayMode = $state<DemoMode>(getDemoMode());
  let seismicChart = $state.raw<SeismicSectionChartHandle | null>(null);
  let gatherChart = $state.raw<SeismicGatherChartHandle | null>(null);
  let surveyMapChart = $state.raw<SurveyMapChartHandle | null>(null);
  let correlationChart = $state.raw<WellCorrelationPanelChartHandle | null>(null);
  let rockPhysicsChart = $state.raw<RockPhysicsCrossplotChartHandle | null>(null);
  let volumeChart = $state.raw<VolumeInterpretationChartHandle | null>(null);
  let avoResponseChart = $state.raw<AvoResponseChartHandle | null>(null);
  let avoCrossplotChart = $state.raw<AvoInterceptGradientCrossplotChartHandle | null>(null);
  let avoHistogramChart = $state.raw<AvoChiProjectionHistogramChartHandle | null>(null);
  let activeDemo = $state<DemoRoute>(getDemoRoute());
  let publicMode = $derived(displayMode !== "playground");
  let embedMode = $derived(displayMode === "embed");
  let activePublicDemoMeta = $derived(PUBLIC_DEMO_META[activeDemo]);

  let sectionKind = $state<MockSectionKind>("inline");
  let sectionDomain = $state<MockSectionDomain>("time");
  let section = $state.raw<OphioliteSectionView | null>(toContractSectionView());
  let sectionHorizons = $state.raw<readonly SectionHorizonOverlay[]>(createMockSectionHorizons("inline", "time"));
  let sectionWellOverlays = $state.raw<readonly SectionWellOverlay[]>(createMockSectionWellOverlays("inline", "time"));
  let showVelocityOverlay = $state(true);
  let velocityOverlayOpacity = $state(0.58);
  let velocityOverlayColorMap = $state<SectionScalarOverlayColorMap>("turbo");
  let sectionScalarOverlays = $state.raw<readonly SectionScalarOverlay[]>(createSectionScalarOverlays());
  let renderMode = $state<"heatmap" | "wiggle">("heatmap");
  let colormap = $state<"grayscale" | "red-white-blue">("grayscale");
  let resetToken = $state(0);
  let lastViewport = $state.raw<SectionViewportChanged | null>(null);
  let viewId = $derived(section ? `${section.axis}:${section.coordinate.index}` : "empty");
  let seismicInteractions = $state.raw<SeismicChartInteractionConfig>({
    tool: "pointer"
  });
  let lastSeismicInteractionState = $state.raw<SeismicChartInteractionState>({
    capabilities: {
      tools: [...SEISMIC_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...SEISMIC_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "pointer"
  });
  let lastSeismicEvent = $state("none");
  let seismicBaseRenderer = $state<DemoSeismicBaseRenderer>("auto");
  let seismicRendererEpoch = $state(0);
  let activeSeismicBaseRenderer = $state("unknown");

  let gather = $state.raw<GatherView | null>(createMockGatherView());
  let gatherRenderMode = $state<"heatmap" | "wiggle">("wiggle");
  let gatherColormap = $state<"grayscale" | "red-white-blue">("red-white-blue");
  let gatherResetToken = $state(0);
  let lastGatherViewport = $state.raw<GatherViewportChanged | null>(null);
  let gatherViewId = $derived(gather ? `${gather.gather_axis_kind}:${gather.label}` : "empty");
  let gatherInteractions = $state.raw<SeismicChartInteractionConfig>({
    tool: "pointer"
  });
  let lastGatherInteractionState = $state.raw<SeismicChartInteractionState>({
    capabilities: {
      tools: [...SEISMIC_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...SEISMIC_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "pointer"
  });
  let lastGatherEvent = $state("none");
  let lastGatherProbe = $state.raw<GatherProbeChanged["probe"]>(null);
  let activeGatherBaseRenderer = $state("unknown");

  let surveyMap = $state.raw<SurveyMapModel | null>(createMockSurveyMap());
  let surveyMapResetToken = $state(0);
  let surveyMapInteractions = $state.raw<SurveyMapChartInteractionConfig>({
    tool: "pointer"
  });
  let lastSurveyMapInteractionState = $state.raw<SurveyMapChartInteractionState>({
    capabilities: {
      tools: [...SURVEY_MAP_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...SURVEY_MAP_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "pointer"
  });
  let lastSurveyMapEvent = $state("none");
  let lastSurveyMapViewport = $state.raw<SurveyMapViewport | null>(null);
  let lastSurveyMapProbe = $state.raw<SurveyMapProbe | null>(null);
  let selectedSurveyMapWellId = $state<string | null>(null);

  let correlationPanel = $state.raw<WellPanelModel | null>(createMockWellPanel());
  let correlationResetToken = $state(0);
  let correlationInteractions = $state.raw<WellCorrelationChartInteractionConfig>({
    tool: "crosshair"
  });
  let lastCorrelationInteractionState = $state.raw<WellCorrelationChartInteractionState>({
    capabilities: {
      tools: [...WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "crosshair"
  });
  let lastCorrelationEvent = $state("none");
  let lastCorrelationViewport = $state.raw<WellCorrelationViewport | null>(null);
  let lastCorrelationProbe = $state.raw<WellCorrelationProbe | null>(null);
  let lastCorrelationDebug = $state.raw<WellCorrelationDebugSnapshot | null>(null);

  let rockPhysicsTemplateId = $state<RockPhysicsTemplateId>("vp-vs-vs-ai");
  let rockPhysicsColorMode = $state<RockPhysicsMockColorMode>(getDefaultRockPhysicsMockColorMode("vp-vs-vs-ai"));
  let rockPhysicsDense = $state(false);
  let rockPhysicsModel = $state.raw<RockPhysicsCrossplotModel | null>(createRockPhysicsModel());
  let rockPhysicsResetToken = $state(0);
  let rockPhysicsInteractions = $state.raw<RockPhysicsCrossplotChartInteractionConfig>({
    tool: "crosshair"
  });
  let lastRockPhysicsInteractionState = $state.raw<RockPhysicsCrossplotChartInteractionState>({
    capabilities: {
      tools: [...ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "crosshair"
  });
  let lastRockPhysicsEvent = $state("none");
  let lastRockPhysicsViewport = $state.raw<RockPhysicsCrossplotViewport | null>(null);
  let lastRockPhysicsProbe = $state.raw<RockPhysicsCrossplotProbe | null>(null);
  let rockPhysicsTemplateSpec = $derived(getRockPhysicsTemplateSpec(rockPhysicsTemplateId));
  let rockPhysicsColorModes = $derived(getRockPhysicsMockColorModes(rockPhysicsTemplateId));
  let rockPhysicsAxisOverrides = $state.raw<CartesianAxisOverrides>({});

  let volumeColormap = $state<VolumeInterpretationColorMap>("red-white-blue");
  let volumeSliceOpacity = $state(0.94);
  let volumeContours = $state(true);
  let volumeRenderer = $state<VolumeInterpretationChartRenderer>("vtk");
  let volumeModel = $state.raw<VolumeInterpretationModel | null>(createVolumeSceneModel());
  let volumeResetToken = $state(0);
  let volumeSliceSerial = $state(0);
  let volumeInteractions = $state.raw<VolumeInterpretationChartInteractionConfig>({
    tool: "pointer"
  });
  let lastVolumeInteractionState = $state.raw<VolumeInterpretationChartInteractionState>({
    capabilities: {
      tools: [...VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "pointer",
    selectionContext: null
  });
  let lastVolumeEvent = $state("none");
  let lastVolumeProbe = $state.raw<VolumeInterpretationProbe | null>(null);
  let lastVolumeSelection = $state.raw<VolumeInterpretationSelection | null>(null);
  let lastVolumeClickedSelection = $state.raw<VolumeInterpretationSelection | null>(null);
  let lastVolumeDebugPick = $state.raw<VolumeInterpretationDebugPickPayload | null>(null);
  let volumeDebugHistory = $state.raw<VolumeInterpretationDebugPickPayload[]>([]);
  let lastVolumeView = $state.raw<VolumeInterpretationView | null>(null);
  let lastVolumeInterpretationMessage = $state("Right-click a slice or horizon to remove it.");

  let avoDense = $state(false);
  let avoChiAngleDeg = $state(35);
  let avoResponseModel = $state.raw<AvoResponseModel | null>(createAvoResponseModel());
  let avoCrossplotModel = $state.raw<AvoCrossplotModel | null>(createAvoCrossplotModel());
  let avoChiModel = $state.raw<AvoChiProjectionModel | null>(createAvoChiModel());
  let avoResetToken = $state(0);
  let avoInteractions = $state.raw<AvoChartInteractionConfig>({
    tool: "crosshair"
  });
  let lastAvoInteractionState = $state.raw<AvoChartInteractionState>({
    capabilities: {
      tools: [...AVO_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...AVO_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "crosshair"
  });
  let lastAvoEvent = $state("none");
  let lastAvoResponseViewport = $state.raw<AvoCartesianViewport | null>(null);
  let lastAvoCrossplotViewport = $state.raw<AvoCartesianViewport | null>(null);
  let lastAvoChiViewport = $state.raw<AvoCartesianViewport | null>(null);
  let lastAvoResponseProbe = $state.raw<AvoResponseProbe | null>(null);
  let lastAvoCrossplotProbe = $state.raw<AvoCrossplotProbe | null>(null);
  let lastAvoChiProbe = $state.raw<AvoHistogramProbe | null>(null);
  let avoResponseAxisOverrides = $state.raw<CartesianAxisOverrides>({});
  let avoCrossplotAxisOverrides = $state.raw<CartesianAxisOverrides>({});
  let avoChiAxisOverrides = $state.raw<CartesianAxisOverrides>({});
  let axisEditor = $state.raw<AxisEditorState | null>(null);

  let seismicToolbarTools = $derived.by<ChartToolbarToolItem<SeismicChartTool>[]>(() =>
    SEISMIC_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: toolLabel(tool),
      icon: tool,
      active: lastSeismicInteractionState.tool === tool,
      disabled: !section
    }))
  );
  let seismicToolbarActions = $derived.by<ChartToolbarActionItem<SeismicChartAction>[]>(() =>
    SEISMIC_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit To Data" : action,
      icon: "fitToData",
      disabled: !section
    }))
  );
  let gatherToolbarTools = $derived.by<ChartToolbarToolItem<SeismicChartTool>[]>(() =>
    SEISMIC_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: toolLabel(tool),
      icon: tool,
      active: lastGatherInteractionState.tool === tool,
      disabled: !gather
    }))
  );
  let gatherToolbarActions = $derived.by<ChartToolbarActionItem<SeismicChartAction>[]>(() =>
    SEISMIC_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit To Data" : action,
      icon: "fitToData",
      disabled: !gather
    }))
  );
  let surveyMapToolbarTools = $derived.by<ChartToolbarToolItem<SurveyMapChartTool>[]>(() =>
    SURVEY_MAP_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: tool === "pointer" ? "Pointer" : "Pan",
      icon: tool === "pointer" ? "pointer" : "pan",
      active: lastSurveyMapInteractionState.tool === tool,
      disabled: !surveyMap
    }))
  );
  let surveyMapToolbarActions = $derived.by<ChartToolbarActionItem<SurveyMapChartAction>[]>(() =>
    SURVEY_MAP_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit To Data" : action,
      icon: "fitToData",
      disabled: !surveyMap
    }))
  );
  let correlationToolbarTools = $derived.by<ChartToolbarToolItem<WellCorrelationChartTool>[]>(() =>
    WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: toolLabel(tool),
      icon: tool,
      active: correlationInteractions.tool === tool,
      disabled: !correlationPanel
    }))
  );
  let correlationToolbarActions = $derived.by<ChartToolbarActionItem<WellCorrelationChartAction>[]>(() =>
    WELL_CORRELATION_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit To Data" : action,
      icon: "fitToData",
      disabled: !correlationPanel
    }))
  );
  let rockPhysicsToolbarTools = $derived.by<ChartToolbarToolItem<RockPhysicsCrossplotChartTool>[]>(() =>
    ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: toolLabel(tool),
      icon: tool,
      active: lastRockPhysicsInteractionState.tool === tool,
      disabled: !rockPhysicsModel
    }))
  );
  let rockPhysicsToolbarActions = $derived.by<ChartToolbarActionItem<RockPhysicsCrossplotChartAction>[]>(() =>
    ROCK_PHYSICS_CROSSPLOT_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit To Data" : action,
      icon: "fitToData",
      disabled: !rockPhysicsModel
    }))
  );
  let volumeToolbarTools = $derived.by<ChartToolbarToolItem<VolumeInterpretationChartTool>[]>(() =>
    VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: volumeToolLabel(tool),
      icon: volumeToolIcon(tool),
      active: lastVolumeInteractionState.tool === tool,
      disabled: !volumeModel
    }))
  );
  let volumeToolbarActions = $derived.by<ChartToolbarActionItem<VolumeInterpretationChartAction>[]>(() =>
    VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label:
        action === "fitToData"
          ? "Fit"
          : action === "topView"
            ? "Top"
            : action === "sideView"
              ? "Side"
              : "Center",
      icon:
        action === "fitToData"
          ? "fitToData"
          : action === "topView"
            ? "topView"
            : action === "sideView"
              ? "sideView"
              : "centerSelection",
      disabled: !volumeModel || (action === "centerSelection" && !lastVolumeSelection)
    }))
  );
  let avoToolbarTools = $derived.by<ChartToolbarToolItem<AvoChartTool>[]>(() =>
    AVO_CHART_INTERACTION_CAPABILITIES.tools.map((tool) => ({
      id: tool,
      label: toolLabel(tool),
      icon: tool,
      active: lastAvoInteractionState.tool === tool,
      disabled: !avoResponseModel && !avoCrossplotModel && !avoChiModel
    }))
  );
  let avoToolbarActions = $derived.by<ChartToolbarActionItem<AvoChartAction>[]>(() =>
    AVO_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit To Data" : action,
      icon: "fitToData",
      disabled: !avoResponseModel && !avoCrossplotModel && !avoChiModel
    }))
  );

  const seismicToolbarTop = `${PLOT_MARGIN.top}px`;
  const seismicToolbarLeft = `${PLOT_MARGIN.left}px`;
  const seismicToolbarRight = `${PLOT_MARGIN.right}px`;

  $effect(() => {
    const seismicWindow = getSeismicRendererWindow();
    if (seismicWindow) {
      seismicWindow.__OPHIOLITE_FORCE_SEISMIC_BASE_RENDERER__ = seismicBaseRenderer;
    }
  });

  function refreshMockSection() {
    section = toContractSectionView();
    sectionHorizons = createMockSectionHorizons(sectionKind, sectionDomain);
    sectionWellOverlays = createMockSectionWellOverlays(sectionKind, sectionDomain);
    refreshSectionScalarOverlays();
    resetToken += 1;
  }

  function toggleSectionKind() {
    sectionKind = sectionKind === "inline" ? "arbitrary" : "inline";
    refreshMockSection();
  }

  function toggleSectionDomain() {
    sectionDomain = sectionDomain === "time" ? "depth" : "time";
    refreshMockSection();
  }

  function clearSection() {
    section = null;
    sectionHorizons = [];
    sectionWellOverlays = [];
    sectionScalarOverlays = [];
    lastViewport = null;
  }

  function toggleVelocityOverlay() {
    showVelocityOverlay = !showVelocityOverlay;
    refreshSectionScalarOverlays();
  }

  function toggleVelocityOverlayColorMap() {
    velocityOverlayColorMap = velocityOverlayColorMap === "turbo" ? "viridis" : "turbo";
    refreshSectionScalarOverlays();
  }

  function refreshSectionScalarOverlays() {
    sectionScalarOverlays = createSectionScalarOverlays();
  }

  function createSectionScalarOverlays() {
    if (!showVelocityOverlay || !section) {
      return [];
    }
    return createMockSectionScalarOverlays(sectionKind, sectionDomain, {
      opacity: velocityOverlayOpacity,
      colorMap: velocityOverlayColorMap
    });
  }

  function setSeismicTool(tool: string) {
    seismicInteractions = {
      ...seismicInteractions,
      tool: tool as SeismicChartTool
    };
  }

  function runSeismicAction(action: string) {
    if (action === "fitToData") {
      fitSeismicToData();
    }
  }

  function handleSeismicInteractionStateChange(event: SeismicChartInteractionState) {
    lastSeismicInteractionState = event;
    seismicInteractions = {
      ...seismicInteractions,
      tool: event.tool
    };
  }

  function toggleRenderMode() {
    renderMode = renderMode === "heatmap" ? "wiggle" : "heatmap";
  }

  function toggleColormap() {
    colormap = colormap === "grayscale" ? "red-white-blue" : "grayscale";
  }

  function fitSeismicToData() {
    seismicChart?.fitToData?.();
  }

  function setSeismicBaseRendererPreference(nextRenderer: DemoSeismicBaseRenderer): void {
    seismicBaseRenderer = nextRenderer;
    const seismicWindow = getSeismicRendererWindow();
    if (seismicWindow) {
      seismicWindow.__OPHIOLITE_FORCE_SEISMIC_BASE_RENDERER__ = nextRenderer;
    }
    activeSeismicBaseRenderer = "pending";
    activeGatherBaseRenderer = "pending";
    seismicRendererEpoch += 1;
    queueSeismicRendererKindSync();
  }

  function getSeismicRendererWindow():
    | (Window & { __OPHIOLITE_FORCE_SEISMIC_BASE_RENDERER__?: DemoSeismicBaseRenderer })
    | null {
    return typeof window === "undefined"
      ? null
      : (window as Window & {
          __OPHIOLITE_FORCE_SEISMIC_BASE_RENDERER__?: DemoSeismicBaseRenderer;
        });
  }

  function refreshGather() {
    gather = createMockGatherView();
    gatherResetToken += 1;
  }

  function clearGather() {
    gather = null;
    lastGatherViewport = null;
    lastGatherProbe = null;
  }

  function setGatherTool(tool: string) {
    gatherInteractions = {
      ...gatherInteractions,
      tool: tool as SeismicChartTool
    };
  }

  function runGatherAction(action: string) {
    if (action === "fitToData") {
      fitGatherToData();
    }
  }

  function handleGatherInteractionStateChange(event: SeismicChartInteractionState) {
    lastGatherInteractionState = event;
    gatherInteractions = {
      ...gatherInteractions,
      tool: event.tool
    };
  }

  function toggleGatherRenderMode() {
    gatherRenderMode = gatherRenderMode === "heatmap" ? "wiggle" : "heatmap";
  }

  function toggleGatherColormap() {
    gatherColormap = gatherColormap === "grayscale" ? "red-white-blue" : "grayscale";
  }

  function fitGatherToData() {
    gatherChart?.fitToData?.();
  }

  function queueSeismicRendererKindSync(): void {
    if (typeof window === "undefined") {
      return;
    }
    window.requestAnimationFrame(() => {
      syncSeismicRendererKinds();
      window.setTimeout(syncSeismicRendererKinds, 250);
    });
  }

  function syncSeismicRendererKinds(): void {
    if (typeof document === "undefined") {
      return;
    }
    activeSeismicBaseRenderer =
      document.querySelector<HTMLElement>('[aria-label="Seismic section chart"]')?.dataset.baseRenderer ?? "unknown";
    activeGatherBaseRenderer =
      document.querySelector<HTMLElement>('[aria-label="Seismic gather chart"]')?.dataset.baseRenderer ?? "unknown";
  }

  function refreshSurveyMap() {
    surveyMap = createMockSurveyMap();
    surveyMapResetToken += 1;
    selectedSurveyMapWellId = null;
  }

  function clearSurveyMap() {
    surveyMap = null;
    lastSurveyMapViewport = null;
    lastSurveyMapProbe = null;
    selectedSurveyMapWellId = null;
  }

  function setSurveyMapTool(tool: string) {
    surveyMapInteractions = {
      ...surveyMapInteractions,
      tool: tool as SurveyMapChartTool
    };
  }

  function runSurveyMapAction(action: string) {
    if (action === "fitToData") {
      fitSurveyMapToData();
    }
  }

  function handleSurveyMapInteractionStateChange(event: SurveyMapChartInteractionState) {
    lastSurveyMapInteractionState = event;
    surveyMapInteractions = {
      ...surveyMapInteractions,
      tool: event.tool
    };
  }

  function fitSurveyMapToData() {
    surveyMapChart?.fitToData?.();
  }

  function refreshCorrelationPanel() {
    correlationPanel = createMockWellPanel();
    correlationResetToken += 1;
  }

  function clearCorrelationPanel() {
    correlationPanel = null;
    lastCorrelationViewport = null;
    lastCorrelationProbe = null;
  }

  function setCorrelationTool(tool: string) {
    correlationInteractions = {
      ...correlationInteractions,
      tool: tool as WellCorrelationChartTool
    };
  }

  async function copyCorrelationDebug(): Promise<void> {
    if (!lastCorrelationDebug || typeof navigator === "undefined" || !navigator.clipboard) {
      return;
    }
    await navigator.clipboard.writeText(JSON.stringify(lastCorrelationDebug, null, 2));
  }

  function captureCorrelationDebug(): void {
    lastCorrelationDebug = correlationChart?.getDebugSnapshot?.() ?? null;
  }

  function runCorrelationAction(action: string) {
    if (action === "fitToData") {
      fitCorrelationToData();
    }
  }

  function handleCorrelationInteractionStateChange(event: WellCorrelationChartInteractionState) {
    lastCorrelationInteractionState = event;
    if (correlationInteractions.tool === event.tool) {
      return;
    }
    correlationInteractions = {
      ...correlationInteractions,
      tool: event.tool
    };
  }

  function fitCorrelationToData() {
    correlationChart?.fitToData?.();
  }

  function zoomCorrelation(factor: number) {
    correlationChart?.zoomBy?.(factor);
  }

  function panCorrelation(deltaDepth: number) {
    correlationChart?.panBy?.(deltaDepth);
  }

  function createRockPhysicsModel() {
    return createMockRockPhysicsCrossplotModel({
      templateId: rockPhysicsTemplateId,
      pointCount: rockPhysicsDense ? 120_000 : 8_000,
      wellCount: rockPhysicsDense ? 10 : 6,
      colorMode: rockPhysicsColorMode
    });
  }

  function refreshRockPhysics() {
    rockPhysicsModel = createRockPhysicsModel();
    rockPhysicsAxisOverrides = {};
    closeAxisEditorForChart("rockPhysics");
    rockPhysicsResetToken += 1;
  }

  function clearRockPhysics() {
    rockPhysicsModel = null;
    rockPhysicsAxisOverrides = {};
    closeAxisEditorForChart("rockPhysics");
    lastRockPhysicsViewport = null;
    lastRockPhysicsProbe = null;
  }

  function toggleRockPhysicsColorMode() {
    const modes = rockPhysicsColorModes;
    const currentIndex = Math.max(0, modes.indexOf(rockPhysicsColorMode));
    rockPhysicsColorMode = modes[(currentIndex + 1) % modes.length]!;
    refreshRockPhysics();
  }

  function setRockPhysicsTemplate(templateId: RockPhysicsTemplateId) {
    rockPhysicsTemplateId = templateId;
    rockPhysicsColorMode = getDefaultRockPhysicsMockColorMode(templateId);
    refreshRockPhysics();
  }

  function cycleRockPhysicsTemplate() {
    const currentIndex = ALL_ROCK_PHYSICS_TEMPLATE_IDS.indexOf(rockPhysicsTemplateId);
    const nextIndex = (currentIndex + 1) % ALL_ROCK_PHYSICS_TEMPLATE_IDS.length;
    setRockPhysicsTemplate(ALL_ROCK_PHYSICS_TEMPLATE_IDS[nextIndex]!);
  }

  function toggleRockPhysicsDensity() {
    rockPhysicsDense = !rockPhysicsDense;
    refreshRockPhysics();
  }

  function setRockPhysicsTool(tool: string) {
    rockPhysicsInteractions = {
      ...rockPhysicsInteractions,
      tool: tool as RockPhysicsCrossplotChartTool
    };
    lastRockPhysicsInteractionState = {
      ...lastRockPhysicsInteractionState,
      tool: tool as RockPhysicsCrossplotChartTool
    };
  }

  function runRockPhysicsAction(action: string) {
    if (action === "fitToData") {
      fitRockPhysicsToData();
    }
  }

  function handleRockPhysicsInteractionStateChange(event: RockPhysicsCrossplotChartInteractionState) {
    lastRockPhysicsInteractionState = event;
    rockPhysicsInteractions = {
      ...rockPhysicsInteractions,
      tool: event.tool
    };
  }

  function fitRockPhysicsToData() {
    rockPhysicsChart?.fitToData?.();
  }

  function zoomRockPhysics(factor: number) {
    rockPhysicsChart?.zoomBy?.(factor);
  }

  function panRockPhysics(deltaX: number, deltaY: number) {
    rockPhysicsChart?.panBy?.(deltaX, deltaY);
  }

  function formatRockPhysicsNumber(value: number): string {
    if (Math.abs(value) >= 1_000) {
      return Math.round(value).toString();
    }
    if (Math.abs(value) >= 100) {
      return value.toFixed(1).replace(/\.0$/, "");
    }
    if (Math.abs(value) >= 10) {
      return value.toFixed(2).replace(/\.00$/, "");
    }
    return value.toFixed(3).replace(/\.000$/, "");
  }

  function createAxisEditorForm(
    overrides: CartesianAxisOverrides,
    axis: CartesianAxisId
  ): AxisEditorFormState {
    const axisOverride = overrides[axis];
    return {
      label: axisOverride?.label ?? "",
      unit: axisOverride?.unit ?? "",
      min: formatAxisEditorNumber(axisOverride?.min),
      max: formatAxisEditorNumber(axisOverride?.max),
      tickCount: axisOverride?.tickCount ? String(axisOverride.tickCount) : "",
      tickFormat: axisOverride?.tickFormat ?? "auto"
    };
  }

  function formatAxisEditorNumber(value: number | undefined): string {
    return value === undefined || !Number.isFinite(value) ? "" : String(value);
  }

  function parseAxisEditorNumber(value: string): number | undefined {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      return undefined;
    }
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : undefined;
  }

  function parseAxisEditorInteger(value: string): number | undefined {
    const parsed = parseAxisEditorNumber(value);
    if (parsed === undefined) {
      return undefined;
    }
    const rounded = Math.round(parsed);
    return rounded >= 2 ? rounded : undefined;
  }

  function buildAxisOverrideFromForm(form: AxisEditorFormState): CartesianAxisOverrides[CartesianAxisId] {
    const next = {
      label: form.label.trim() || undefined,
      unit: form.unit.trim() || undefined,
      min: parseAxisEditorNumber(form.min),
      max: parseAxisEditorNumber(form.max),
      tickCount: parseAxisEditorInteger(form.tickCount),
      tickFormat: form.tickFormat === "auto" ? undefined : form.tickFormat
    };
    if (
      next.label === undefined &&
      next.unit === undefined &&
      next.min === undefined &&
      next.max === undefined &&
      next.tickCount === undefined &&
      next.tickFormat === undefined
    ) {
      return undefined;
    }
    return next;
  }

  function getAxisOverridesForChart(chartKey: AxisEditorChartKey): CartesianAxisOverrides {
    switch (chartKey) {
      case "rockPhysics":
        return rockPhysicsAxisOverrides;
      case "avoResponse":
        return avoResponseAxisOverrides;
      case "avoCrossplot":
        return avoCrossplotAxisOverrides;
      case "avoChi":
        return avoChiAxisOverrides;
    }
  }

  function setAxisOverridesForChart(chartKey: AxisEditorChartKey, overrides: CartesianAxisOverrides): void {
    switch (chartKey) {
      case "rockPhysics":
        rockPhysicsAxisOverrides = overrides;
        break;
      case "avoResponse":
        avoResponseAxisOverrides = overrides;
        break;
      case "avoCrossplot":
        avoCrossplotAxisOverrides = overrides;
        break;
      case "avoChi":
        avoChiAxisOverrides = overrides;
        break;
    }
  }

  function openAxisEditor(
    chartKey: AxisEditorChartKey,
    chartLabel: string,
    event: CartesianAxisContextRequestPayload,
    overrides: CartesianAxisOverrides
  ): void {
    axisEditor = {
      chartKey,
      chartLabel,
      axis: event.axis,
      clientX: event.clientX,
      clientY: event.clientY,
      form: createAxisEditorForm(overrides, event.axis)
    };
  }

  function updateAxisEditorField(field: keyof AxisEditorFormState, value: string): void {
    if (!axisEditor) {
      return;
    }
    axisEditor = {
      ...axisEditor,
      form: {
        ...axisEditor.form,
        [field]: value
      }
    };
  }

  function closeAxisEditor(): void {
    axisEditor = null;
  }

  function applyAxisEditor(): void {
    if (!axisEditor) {
      return;
    }
    const nextAxisOverride = buildAxisOverrideFromForm(axisEditor.form);
    const nextOverrides = {
      ...getAxisOverridesForChart(axisEditor.chartKey)
    };
    if (nextAxisOverride) {
      nextOverrides[axisEditor.axis] = nextAxisOverride;
    } else {
      delete nextOverrides[axisEditor.axis];
    }
    setAxisOverridesForChart(axisEditor.chartKey, nextOverrides);
    closeAxisEditor();
  }

  function resetAxisEditorAxis(): void {
    if (!axisEditor) {
      return;
    }
    const nextOverrides = {
      ...getAxisOverridesForChart(axisEditor.chartKey)
    };
    delete nextOverrides[axisEditor.axis];
    setAxisOverridesForChart(axisEditor.chartKey, nextOverrides);
    closeAxisEditor();
  }

  function closeAxisEditorForChart(chartKey: AxisEditorChartKey): void {
    if (axisEditor?.chartKey === chartKey) {
      axisEditor = null;
    }
  }

  function createVolumeSceneModel() {
    const base = createMockVolumeInterpretationModel();
    return {
      ...base,
      slicePlanes: base.slicePlanes.map((plane) => ({
        ...plane,
        style: {
          ...plane.style,
          colormap: volumeColormap,
          opacity: volumeSliceOpacity
        }
      })),
      horizons: base.horizons.map((horizon) => ({
        ...horizon,
        style: {
          ...horizon.style,
          showContours: volumeContours
        }
      }))
    };
  }

  function refreshVolumeScene() {
    volumeModel = createVolumeSceneModel();
    volumeResetToken += 1;
    lastVolumeSelection = null;
    lastVolumeClickedSelection = null;
    lastVolumeInterpretationMessage = "Scene refreshed.";
  }

  function clearVolumeScene() {
    volumeModel = null;
    lastVolumeProbe = null;
    lastVolumeSelection = null;
    lastVolumeClickedSelection = null;
    lastVolumeDebugPick = null;
    lastVolumeView = null;
    lastVolumeInterpretationMessage = "Scene cleared.";
  }

  function applyVolumeStyles() {
    if (!volumeModel) {
      return;
    }
    volumeModel = {
      ...volumeModel,
      slicePlanes: volumeModel.slicePlanes.map((plane) => ({
        ...plane,
        style: {
          ...plane.style,
          colormap: volumeColormap,
          opacity: volumeSliceOpacity
        }
      })),
      horizons: volumeModel.horizons.map((horizon) => ({
        ...horizon,
        style: {
          ...horizon.style,
          showContours: volumeContours
        }
      }))
    };
  }

  function toggleVolumeRenderer() {
    volumeRenderer = volumeRenderer === "vtk" ? "placeholder" : "vtk";
  }

  function toggleVolumeColormap() {
    volumeColormap = volumeColormap === "red-white-blue" ? "grayscale" : "red-white-blue";
    applyVolumeStyles();
  }

  function toggleVolumeContours() {
    volumeContours = !volumeContours;
    applyVolumeStyles();
  }

  function setVolumeTool(tool: string) {
    volumeInteractions = {
      ...volumeInteractions,
      tool: tool as VolumeInterpretationChartTool
    };
  }

  function runVolumeAction(action: string) {
    if (action === "fitToData") {
      fitVolumeToData();
    } else if (action === "topView") {
      setVolumeTopView();
    } else if (action === "sideView") {
      setVolumeSideView();
    } else if (action === "centerSelection") {
      centerVolumeSelection();
    }
  }

  function handleVolumeInteractionStateChange(event: VolumeInterpretationChartInteractionState) {
    lastVolumeInteractionState = event;
    volumeInteractions = {
      ...volumeInteractions,
      tool: event.tool
    };
  }

  function fitVolumeToData() {
    volumeChart?.fitToData?.();
  }

  function setVolumeTopView() {
    volumeChart?.topView?.();
  }

  function setVolumeSideView() {
    volumeChart?.sideView?.();
  }

  function centerVolumeSelection() {
    volumeChart?.centerSelection?.();
  }

  function zoomVolume(factor: number) {
    volumeChart?.zoomBy?.(factor);
  }

  function orbitVolume(deltaYawDeg: number, deltaPitchDeg: number) {
    volumeChart?.orbitBy?.(deltaYawDeg, deltaPitchDeg);
  }

  function panVolume(deltaX: number, deltaY: number) {
    volumeChart?.panBy?.(deltaX, deltaY);
  }

  function handleVolumeDeleteRequest(request: VolumeInterpretationDeleteRequest) {
    if (!volumeModel) {
      return;
    }

    if (request.kind === "delete-slice-plane") {
      const removedSlice = volumeModel.slicePlanes.find((plane) => plane.id === request.itemId);
      if (!removedSlice) {
        return;
      }
      volumeModel = {
        ...volumeModel,
        slicePlanes: volumeModel.slicePlanes.filter((plane) => plane.id !== request.itemId)
      };
      if (lastVolumeClickedSelection?.itemId === request.itemId) {
        lastVolumeClickedSelection = null;
      }
      lastVolumeInterpretationMessage = `Removed ${removedSlice.name}.`;
      return;
    }

    const removedHorizon = volumeModel.horizons.find((horizon) => horizon.id === request.itemId);
    if (!removedHorizon) {
      return;
    }
    volumeModel = {
      ...volumeModel,
      horizons: volumeModel.horizons.filter((horizon) => horizon.id !== request.itemId)
    };
    if (lastVolumeClickedSelection?.itemId === request.itemId) {
      lastVolumeClickedSelection = null;
    }
    lastVolumeInterpretationMessage = `Removed ${removedHorizon.name}.`;
  }

  function handleVolumeSelectionChange(selection: VolumeInterpretationSelection | null) {
    lastVolumeSelection = selection;
    if (selection) {
      lastVolumeClickedSelection = selection;
    }
  }

  function describeVolumeSelection(selection: VolumeInterpretationSelection | null): string {
    if (!selection) {
      return "Last selected: none";
    }
    const label =
      selection.kind === "slice-plane"
        ? "Slice"
        : selection.kind === "horizon-surface"
          ? "Horizon"
          : selection.kind === "well-trajectory"
            ? "Well trajectory"
            : selection.kind === "well-marker"
              ? "Well marker"
              : "Annotation";
    return `${label}: ${selection.itemName ?? selection.itemId}`;
  }

  function handleVolumeDebugPick(payload: VolumeInterpretationDebugPickPayload) {
    lastVolumeDebugPick = payload;
    volumeDebugHistory = [...volumeDebugHistory.slice(-199), payload];
    const actualLabel = payload.snapshot.actualWinner?.itemName ?? payload.snapshot.actualWinner?.itemId ?? "none";
    const syntheticLabel =
      payload.snapshot.syntheticWinner?.itemName ?? payload.snapshot.syntheticWinner?.itemId ?? "none";
    const syntheticSuffix =
      syntheticLabel !== "none" && syntheticLabel !== actualLabel ? ` synthetic=${syntheticLabel}` : "";
    console.groupCollapsed(
      `[volume-debug] ${payload.phase} (${payload.stageX.toFixed(1)}, ${payload.stageY.toFixed(1)}) actual=${actualLabel}${syntheticSuffix} picked=${payload.snapshot.actualPickedCount} matchedBy=${payload.snapshot.actualMatchedBy ?? "none"} render=(${payload.snapshot.renderPointerX.toFixed(1)}, ${payload.snapshot.renderPointerY.toFixed(1)}) scale=${payload.snapshot.renderScaleX.toFixed(2)}x${payload.snapshot.renderScaleY.toFixed(2)}`
    );
    console.log("snapshot", payload.snapshot);
    console.table(
      payload.snapshot.candidates.map((candidate) => ({
        hit: candidate.hit,
        score: candidate.score === null ? "miss" : candidate.score.toFixed(2),
        depth: candidate.depth.toFixed(4),
        targetType: candidate.targetType,
        kind: candidate.kind,
        item: candidate.itemName ?? candidate.itemId,
        screen: `${candidate.screenX.toFixed(1)}, ${candidate.screenY.toFixed(1)}`,
        world: `${formatVolumeCoordinate(candidate.worldX)}, ${formatVolumeCoordinate(candidate.worldY)}, ${formatVolumeCoordinate(candidate.worldZ)}`
      }))
    );
    console.groupEnd();
  }

  function formatVolumeDebugJson(payloads: VolumeInterpretationDebugPickPayload[]): string {
    return JSON.stringify(payloads, null, 2);
  }

  async function copyVolumeDebugHistory(): Promise<void> {
    if (volumeDebugHistory.length === 0 || typeof navigator === "undefined" || !navigator.clipboard) {
      return;
    }
    await navigator.clipboard.writeText(formatVolumeDebugJson(volumeDebugHistory));
    lastVolumeInterpretationMessage = `Copied ${volumeDebugHistory.length} volume debug snapshots.`;
  }

  function downloadVolumeDebugHistory(): void {
    if (volumeDebugHistory.length === 0 || typeof document === "undefined") {
      return;
    }
    const blob = new Blob([formatVolumeDebugJson(volumeDebugHistory)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `volume-pick-debug-${new Date().toISOString().replaceAll(":", "-")}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    lastVolumeInterpretationMessage = `Downloaded ${volumeDebugHistory.length} volume debug snapshots.`;
  }

  function formatVolumeDebugCandidates(payload: VolumeInterpretationDebugPickPayload | null): string {
    if (!payload) {
      return "Click in the viewer to inspect pick candidates.";
    }
    return payload.snapshot.candidates
      .map((candidate, index) => {
        const label = candidate.itemName ?? candidate.itemId;
        const score = candidate.score === null ? "miss" : candidate.score.toFixed(2);
        return `${index + 1}. ${candidate.hit ? "*" : "-"} ${candidate.kind} ${label}
   score ${score} depth ${candidate.depth.toFixed(4)}
   screen ${candidate.screenX.toFixed(1)}, ${candidate.screenY.toFixed(1)}
   world ${formatVolumeCoordinate(candidate.worldX)}, ${formatVolumeCoordinate(candidate.worldY)}, ${formatVolumeCoordinate(candidate.worldZ)}`;
      })
      .join("\n");
  }

  function handleVolumeEditRequest(request: VolumeInterpretationEditRequest) {
    if (!volumeModel) {
      return;
    }
    if (request.kind === "move-slice-plane") {
      const updatedName = formatVolumeSliceName(request.axis, request.position, volumeModel.sampleDomain);
      volumeModel = {
        ...volumeModel,
        slicePlanes: volumeModel.slicePlanes.map((plane) =>
          plane.id === request.itemId
            ? {
                ...plane,
                name: updatedName,
                position: request.position
              }
            : plane
        )
      };
      lastVolumeInterpretationMessage =
        request.phase === "commit"
          ? `Moved ${updatedName} to ${formatVolumeCoordinate(request.position)}.`
          : `Moving ${updatedName}...`;
      return;
    }
    handleVolumeDeleteRequest(request);
  }

  function addRandomVolumeSlice(axis: VolumeInterpretationAxis) {
    if (!volumeModel) {
      return;
    }
    const volume = volumeModel.volumes[0];
    if (!volume) {
      return;
    }

    volumeSliceSerial += 1;
    const position = randomSlicePosition(axis, volume.bounds);
    const name = formatVolumeSliceName(axis, position, volumeModel.sampleDomain);

    volumeModel = {
      ...volumeModel,
      slicePlanes: [
        ...volumeModel.slicePlanes,
        {
          id: `slice-${axis}-added-${volumeSliceSerial}`,
          name,
          volumeId: volume.id,
          axis,
          position,
          visible: true,
          style: {
            colormap: volumeColormap,
            gain: volume.displayDefaults?.gain ?? 1,
            opacity: volumeSliceOpacity,
            showBorder: true
          }
        }
      ]
    };
    lastVolumeInterpretationMessage = `Added ${name}.`;
  }

  function handleVolumeInterpretationRequest(request: VolumeInterpretationInterpretationRequest) {
    if (!volumeModel) {
      return;
    }

    lastVolumeInterpretationMessage = `Request ${request.kind} @ ${formatVolumeCoordinate(request.worldX)}, ${formatVolumeCoordinate(request.worldY)}, ${formatVolumeCoordinate(request.worldZ)}`;
    const targetHorizonId = request.targetHorizonId ?? volumeModel.horizons[0]?.id;
    if (!targetHorizonId) {
      return;
    }

    volumeModel = {
      ...volumeModel,
      horizons: volumeModel.horizons.map((horizon) =>
        horizon.id === targetHorizonId
          ? {
              ...horizon,
              points: Float32Array.from(horizon.points, (value, index) =>
                index % 3 === 2
                  ? value + Math.sin(request.worldX * 0.002 + request.worldY * 0.001 + index * 0.03) * 18
                  : value
              )
            }
          : horizon
      )
    };
  }

  function formatVolumeCoordinate(value: number): string {
    return value.toFixed(0);
  }

  function formatVolumeSliceName(
    axis: VolumeInterpretationAxis,
    position: number,
    sampleDomain: VolumeInterpretationModel["sampleDomain"]
  ): string {
    const axisLabel =
      axis === "inline"
        ? "Inline"
        : axis === "xline"
          ? "Xline"
          : sampleDomain === "depth"
            ? "Depth Slice"
            : "Sample Slice";
    return `${axisLabel} ${Math.round(position)}`;
  }

  function randomSlicePosition(axis: VolumeInterpretationAxis, bounds: VolumeInterpretationModel["sceneBounds"]): number {
    const [min, max] =
      axis === "inline"
        ? [bounds.minX, bounds.maxX]
        : axis === "xline"
          ? [bounds.minY, bounds.maxY]
          : [bounds.minZ, bounds.maxZ];
    const padding = Math.max(12, (max - min) * 0.1);
    return min + padding + Math.random() * Math.max(1, max - min - padding * 2);
  }

  function createAvoResponseModel() {
    return createMockAvoResponseModel({
      sampleCountPerInterface: avoDense ? 4_500 : 620,
      chiAngleDeg: avoChiAngleDeg
    });
  }

  function createAvoCrossplotModel() {
    return createMockAvoCrossplotModel({
      sampleCountPerInterface: avoDense ? 4_500 : 620,
      chiAngleDeg: avoChiAngleDeg
    });
  }

  function createAvoChiModel() {
    return createMockAvoChiProjectionModel({
      sampleCountPerInterface: avoDense ? 4_500 : 620,
      chiAngleDeg: avoChiAngleDeg
    });
  }

  function refreshAvo() {
    avoResponseModel = createAvoResponseModel();
    avoCrossplotModel = createAvoCrossplotModel();
    avoChiModel = createAvoChiModel();
    avoResponseAxisOverrides = {};
    avoCrossplotAxisOverrides = {};
    avoChiAxisOverrides = {};
    closeAxisEditorForChart("avoResponse");
    closeAxisEditorForChart("avoCrossplot");
    closeAxisEditorForChart("avoChi");
    avoResetToken += 1;
  }

  function clearAvo() {
    avoResponseModel = null;
    avoCrossplotModel = null;
    avoChiModel = null;
    avoResponseAxisOverrides = {};
    avoCrossplotAxisOverrides = {};
    avoChiAxisOverrides = {};
    closeAxisEditorForChart("avoResponse");
    closeAxisEditorForChart("avoCrossplot");
    closeAxisEditorForChart("avoChi");
    lastAvoResponseViewport = null;
    lastAvoCrossplotViewport = null;
    lastAvoChiViewport = null;
    lastAvoResponseProbe = null;
    lastAvoCrossplotProbe = null;
    lastAvoChiProbe = null;
  }

  function toggleAvoDensity() {
    avoDense = !avoDense;
    refreshAvo();
  }

  function cycleAvoChiAngle() {
    avoChiAngleDeg = avoChiAngleDeg === 35 ? 25 : avoChiAngleDeg === 25 ? 45 : 35;
    refreshAvo();
  }

  function setAvoTool(tool: string) {
    avoInteractions = {
      ...avoInteractions,
      tool: tool as AvoChartTool
    };
  }

  function runAvoAction(action: string) {
    if (action === "fitToData") {
      fitAvoToData();
    }
  }

  function handleAvoInteractionStateChange(event: AvoChartInteractionState) {
    lastAvoInteractionState = event;
    avoInteractions = {
      ...avoInteractions,
      tool: event.tool
    };
  }

  function fitAvoToData() {
    avoResponseChart?.fitToData?.();
    avoCrossplotChart?.fitToData?.();
    avoHistogramChart?.fitToData?.();
  }

  function toContractSectionView(): OphioliteSectionView {
    const source = createMockSection(sectionKind, sectionDomain);
    const displayDefaults = source.displayDefaults;

    return {
      dataset_id: "mock-svelte-playground",
      axis: source.axis,
      coordinate: source.coordinate,
      traces: source.dimensions.traces,
      samples: source.dimensions.samples,
      horizontal_axis_f64le: encodeFloat64(source.horizontalAxis),
      inline_axis_f64le: source.inlineAxis ? encodeFloat64(source.inlineAxis) : null,
      xline_axis_f64le: source.xlineAxis ? encodeFloat64(source.xlineAxis) : null,
      sample_axis_f32le: encodeFloat32(source.sampleAxis),
      amplitudes_f32le: encodeFloat32(source.amplitudes),
      units: source.units
        ? {
            horizontal: source.units.horizontal ?? null,
            sample: source.units.sample ?? null,
            amplitude: source.units.amplitude ?? null
          }
        : null,
      metadata: source.metadata
        ? {
            store_id: source.metadata.storeId ?? null,
            derived_from: source.metadata.derivedFrom ?? null,
            notes: source.metadata.notes ?? []
          }
        : null,
      display_defaults: {
        gain: displayDefaults?.gain ?? 1,
        clip_min: displayDefaults?.clipMin ?? null,
        clip_max: displayDefaults?.clipMax ?? null,
        render_mode: toContractRenderMode(displayDefaults?.renderMode),
        colormap: toContractColormap(displayDefaults?.colormap),
        polarity: displayDefaults?.polarity ?? "normal"
      }
    };
  }

  function encodeFloat32(values: Float32Array): number[] {
    return Array.from(new Uint8Array(values.buffer.slice(0)));
  }

  function encodeFloat64(values: Float64Array): number[] {
    return Array.from(new Uint8Array(values.buffer.slice(0)));
  }

  function toContractRenderMode(value: "heatmap" | "wiggle" | undefined): SectionRenderMode {
    return value === "wiggle" ? "wiggle" : "heatmap";
  }

  function toContractColormap(value: "grayscale" | "red-white-blue" | undefined): SectionColorMap {
    return value === "red-white-blue" ? "red_white_blue" : "grayscale";
  }

  function toolLabel(tool: "pointer" | "crosshair" | "pan"): string {
    return tool === "pointer" ? "Pointer" : tool === "crosshair" ? "Crosshair" : "Pan";
  }

  function volumeToolLabel(tool: VolumeInterpretationChartTool): string {
    switch (tool) {
      case "pointer":
        return "Pointer";
      case "orbit":
        return "Orbit";
      case "pan":
        return "Pan";
      case "slice-drag":
        return "Slice";
      case "crop":
        return "Crop";
      case "select":
        return "Select";
      default:
        return "Seed";
    }
  }

  function volumeToolIcon(tool: VolumeInterpretationChartTool): ToolbarIconName {
    switch (tool) {
      case "pointer":
        return "pointer";
      case "orbit":
        return "orbit";
      case "pan":
        return "pan";
      case "slice-drag":
        return "sliceDrag";
      case "interpret-seed":
        return "crosshair";
      default:
        return "pointer";
    }
  }

  function setDemoRoute(next: DemoRoute) {
    activeDemo = next;
    closeAxisEditor();
    if (next === "seismic" || next === "gather") {
      queueSeismicRendererKindSync();
    }
    if (typeof window !== "undefined") {
      window.location.hash =
        next === "seismic"
          ? "#/seismic"
          : next === "gather"
            ? "#/gather"
            : next === "survey-map"
              ? "#/survey-map"
              : next === "rock-physics"
                ? "#/rock-physics"
                : next === "volume"
                  ? "#/volume"
                : next === "avo"
                  ? "#/avo"
              : "#/well-panel";
    }
  }

  function handleLocationChange() {
    activeDemo = getDemoRoute();
    displayMode = getDemoMode();
  }

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape" && axisEditor) {
      closeAxisEditor();
    }
  }

  function getDemoRoute(): DemoRoute {
    if (typeof window === "undefined") {
      return "seismic";
    }
    if (window.location.hash === "#/survey-map") {
      return "survey-map";
    }
    if (window.location.hash === "#/rock-physics") {
      return "rock-physics";
    }
    if (window.location.hash === "#/volume") {
      return "volume";
    }
    if (window.location.hash === "#/avo") {
      return "avo";
    }
    if (window.location.hash === "#/well-panel") {
      return "well-panel";
    }
    if (window.location.hash === "#/gather") {
      return "gather";
    }
    return "seismic";
  }

  function getDemoMode(): DemoMode {
    if (typeof window === "undefined") {
      return "playground";
    }
    const mode = new URLSearchParams(window.location.search).get("mode");
    if (mode === "embed") {
      return "embed";
    }
    if (mode === "public") {
      return "public";
    }
    return "playground";
  }
</script>

<svelte:head>
  <title>{publicMode ? `${activePublicDemoMeta.title} Example | Ophiolite Charts` : "Ophiolite Charts Svelte Playground"}</title>
</svelte:head>

<svelte:window onhashchange={handleLocationChange} onpopstate={handleLocationChange} onkeydown={handleWindowKeyDown} />

{#snippet avoToolbarOverlay()}
  <ChartInteractionToolbar
    label="AVO interaction toolbar"
    tools={avoToolbarTools}
    actions={avoToolbarActions}
    onToolSelect={setAvoTool}
    onActionSelect={runAvoAction}
    variant="overlay"
    iconOnly={true}
  />
{/snippet}

{#snippet correlationToolbarOverlay()}
  <ChartInteractionToolbar
    label="Correlation interaction toolbar"
    tools={correlationToolbarTools}
    actions={correlationToolbarActions}
    onToolSelect={setCorrelationTool}
    onActionSelect={runCorrelationAction}
    variant="overlay"
    iconOnly={true}
  />
{/snippet}

<div class={["layout", publicMode && "layout-public", embedMode && "layout-embed"]} style={demoCssVars}>
  {#if !publicMode}
    <aside class="sidebar">
      <div>
        <h1>Ophiolite Charts / svelte</h1>
        <p>
          Wrapper-level playground for <code>@ophiolite/charts</code>. The demos are split by chart family so
          seismic and well-panel behavior can be evaluated independently.
        </p>
      </div>

      <section class="group">
        <h2>Demo</h2>
        <button class={["demo-button", activeDemo === "seismic" && "active-demo"]} onclick={() => setDemoRoute("seismic")}>
          Seismic Section
        </button>
        <button class={["demo-button", activeDemo === "gather" && "active-demo"]} onclick={() => setDemoRoute("gather")}>
          Prestack Gather
        </button>
        <button class={["demo-button", activeDemo === "survey-map" && "active-demo"]} onclick={() => setDemoRoute("survey-map")}>
          Survey Map
        </button>
        <button class={["demo-button", activeDemo === "rock-physics" && "active-demo"]} onclick={() => setDemoRoute("rock-physics")}>
          Rock Physics
        </button>
        <button class={["demo-button", activeDemo === "volume" && "active-demo"]} onclick={() => setDemoRoute("volume")}>
          Volume Interpretation
        </button>
        <button class={["demo-button", activeDemo === "avo" && "active-demo"]} onclick={() => setDemoRoute("avo")}>
          AVO
        </button>
        <button class={["demo-button", activeDemo === "well-panel" && "active-demo"]} onclick={() => setDemoRoute("well-panel")}>
          Well Panel
        </button>
      </section>

      {#if activeDemo === "seismic"}
      <section class="group">
        <h2>Renderer</h2>
        <label class="demo-field">
          <span>Base Renderer</span>
          <select
            value={seismicBaseRenderer}
            onchange={(event) =>
              setSeismicBaseRendererPreference(event.currentTarget.value as DemoSeismicBaseRenderer)}
          >
            <option value="auto">Auto</option>
            <option value="worker-webgl">Worker WebGL</option>
            <option value="local-webgl">Local WebGL</option>
            <option value="local-canvas">Local Canvas</option>
          </select>
        </label>
      </section>

      <section class="group">
        <h2>Seismic Controls</h2>
        <button onclick={toggleSectionKind}>
          {sectionKind === "inline" ? "Switch To Arbitrary Mock" : "Switch To Inline 111 Mock"}
        </button>
        <button onclick={refreshMockSection}>Refresh Mock Section</button>
        <button onclick={clearSection}>Clear Section</button>
        <button onclick={fitSeismicToData} disabled={!section}>Fit To Data</button>
        <button onclick={toggleSectionDomain} disabled={!section}>
          {sectionDomain === "time" ? "Switch To Depth" : "Switch To TWT"}
        </button>
        <button onclick={toggleRenderMode} disabled={!section}>
          {renderMode === "heatmap" ? "Switch To Wiggle" : "Switch To Heatmap"}
        </button>
        <button onclick={toggleColormap} disabled={!section}>
          {colormap === "grayscale" ? "Switch To R/W/B" : "Switch To Grayscale"}
        </button>
        <button onclick={toggleVelocityOverlay} disabled={!section}>
          {showVelocityOverlay ? "Hide Velocity Overlay" : "Show Velocity Overlay"}
        </button>
        <button onclick={toggleVelocityOverlayColorMap} disabled={!section || !showVelocityOverlay}>
          {velocityOverlayColorMap === "turbo" ? "Use Viridis Overlay" : "Use Turbo Overlay"}
        </button>
        <label class="range-control">
          <span>Velocity Overlay Alpha {Math.round(velocityOverlayOpacity * 100)}%</span>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            bind:value={velocityOverlayOpacity}
            disabled={!section || !showVelocityOverlay}
            oninput={refreshSectionScalarOverlays}
          />
        </label>
      </section>

      <section class="group">
        <h2>Seismic Status</h2>
        <div class="readout">
          chart bound: {seismicChart ? "yes" : "no"}
          section loaded: {section ? "yes" : "no"}
          render mode: {renderMode}
          colormap: {colormap}
          mock view: {sectionKind}
          sample domain: {sectionDomain}
          velocity model: {MOCK_SECTION_VELOCITY_MODEL_LABEL}
          velocity overlay: {showVelocityOverlay ? "on" : "off"}
          velocity colormap: {velocityOverlayColorMap}
          velocity alpha: {Math.round(velocityOverlayOpacity * 100)}%
          base renderer requested: {seismicBaseRenderer}
          base renderer active: {activeSeismicBaseRenderer}
          tool: {lastSeismicInteractionState.tool}
          last event: {lastSeismicEvent}
          {#if section}
            traces: {section.traces}
            samples: {section.samples}
            scalar overlays: {sectionScalarOverlays.length}
            horizon overlays: {sectionHorizons.length}
            well overlays: {sectionWellOverlays.length}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Seismic Viewport</h2>
        <div class="readout">
          {#if lastViewport}
            traces {lastViewport.viewport.trace_start}..{lastViewport.viewport.trace_end}
            samples {lastViewport.viewport.sample_start}..{lastViewport.viewport.sample_end}
          {:else}
            Viewport callbacks will appear after the chart initializes.
          {/if}
        </div>
      </section>
    {:else if activeDemo === "gather"}
      <section class="group">
        <h2>Renderer</h2>
        <label class="demo-field">
          <span>Base Renderer</span>
          <select
            value={seismicBaseRenderer}
            onchange={(event) =>
              setSeismicBaseRendererPreference(event.currentTarget.value as DemoSeismicBaseRenderer)}
          >
            <option value="auto">Auto</option>
            <option value="worker-webgl">Worker WebGL</option>
            <option value="local-webgl">Local WebGL</option>
            <option value="local-canvas">Local Canvas</option>
          </select>
        </label>
      </section>

      <section class="group">
        <h2>Gather Controls</h2>
        <button onclick={refreshGather}>Refresh Mock Gather</button>
        <button onclick={clearGather}>Clear Gather</button>
        <button onclick={fitGatherToData} disabled={!gather}>Fit To Data</button>
        <button onclick={toggleGatherRenderMode} disabled={!gather}>
          {gatherRenderMode === "heatmap" ? "Switch To Wiggle" : "Switch To Heatmap"}
        </button>
        <button onclick={toggleGatherColormap} disabled={!gather}>
          {gatherColormap === "grayscale" ? "Switch To R/W/B" : "Switch To Grayscale"}
        </button>
      </section>

      <section class="group">
        <h2>Gather Status</h2>
        <div class="readout">
          chart bound: {gatherChart ? "yes" : "no"}
          gather loaded: {gather ? "yes" : "no"}
          render mode: {gatherRenderMode}
          colormap: {gatherColormap}
          base renderer requested: {seismicBaseRenderer}
          base renderer active: {activeGatherBaseRenderer}
          tool: {lastGatherInteractionState.tool}
          last event: {lastGatherEvent}
          {#if gather}
            axis: {gather.gather_axis_kind}
            traces: {gather.traces}
            samples: {gather.samples}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Gather Readout</h2>
        <div class="readout">
          Probe readout is rendered inside the chart.

          {#if lastGatherViewport}
            traces {lastGatherViewport.viewport.trace_start}..{lastGatherViewport.viewport.trace_end}
            samples {lastGatherViewport.viewport.sample_start}..{lastGatherViewport.viewport.sample_end}
          {/if}
        </div>
      </section>
    {:else if activeDemo === "survey-map"}
      <section class="group">
        <h2>Survey Map Controls</h2>
        <button onclick={refreshSurveyMap}>Refresh Mock Survey Map</button>
        <button onclick={clearSurveyMap}>Clear Survey Map</button>
        <button onclick={fitSurveyMapToData} disabled={!surveyMap}>Fit To Data</button>
        <button onclick={() => surveyMapChart?.zoomBy?.(1.25)} disabled={!surveyMap}>Zoom In</button>
        <button onclick={() => surveyMapChart?.zoomBy?.(0.82)} disabled={!surveyMap}>Zoom Out</button>
      </section>

      <section class="group">
        <h2>Survey Map Status</h2>
        <div class="readout">
          chart bound: {surveyMapChart ? "yes" : "no"}
          map loaded: {surveyMap ? "yes" : "no"}
          tool: {lastSurveyMapInteractionState.tool}
          last event: {lastSurveyMapEvent}
          selected well: {selectedSurveyMapWellId ?? "none"}
          {#if surveyMap}
            surveys: {surveyMap.surveys.length}
            wells: {surveyMap.wells.length}
            scalar grid: {surveyMap.scalarField?.columns ?? 0} x {surveyMap.scalarField?.rows ?? 0}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Survey Map Readout</h2>
        <div class="readout">
          Probe readout is rendered inside the chart.

          {#if lastSurveyMapViewport}
            x {lastSurveyMapViewport.xMin.toFixed(0)}..{lastSurveyMapViewport.xMax.toFixed(0)}
            y {lastSurveyMapViewport.yMin.toFixed(0)}..{lastSurveyMapViewport.yMax.toFixed(0)}
          {/if}
        </div>
      </section>
    {:else if activeDemo === "rock-physics"}
      <section class="group">
        <h2>Rock Physics Controls</h2>
        <button onclick={refreshRockPhysics}>Refresh Crossplot</button>
        <button onclick={clearRockPhysics}>Clear Crossplot</button>
        <button onclick={fitRockPhysicsToData} disabled={!rockPhysicsModel}>Fit To Data</button>
        <button onclick={() => zoomRockPhysics(1.25)} disabled={!rockPhysicsModel}>Zoom In</button>
        <button onclick={() => zoomRockPhysics(0.82)} disabled={!rockPhysicsModel}>Zoom Out</button>
        <button onclick={() => panRockPhysics(-120, 0)} disabled={!rockPhysicsModel}>Pan Left</button>
        <button onclick={() => panRockPhysics(120, 0)} disabled={!rockPhysicsModel}>Pan Right</button>
        <button onclick={cycleRockPhysicsTemplate} disabled={!rockPhysicsModel}>
          Next Template: {rockPhysicsTemplateSpec.title}
        </button>
        {#each ROCK_PHYSICS_TEMPLATE_GROUPS as group (group.id)}
          <div class="template-picker">
            <div class="template-picker-label">{group.label}</div>
            {#each group.templateIds as templateId (templateId)}
              <button
                class:active-demo={rockPhysicsTemplateId === templateId}
                onclick={() => setRockPhysicsTemplate(templateId)}
                disabled={rockPhysicsTemplateId === templateId && rockPhysicsModel !== null}
              >
                {getRockPhysicsTemplateSpec(templateId).title}
              </button>
            {/each}
          </div>
        {/each}
        <button onclick={toggleRockPhysicsColorMode} disabled={!rockPhysicsModel}>
          Next Color: {rockPhysicsModel?.colorBinding.label ?? rockPhysicsColorMode}
        </button>
        <button onclick={toggleRockPhysicsDensity}>
          {rockPhysicsDense ? "Load Standard Point Set" : "Load Dense Point Set"}
        </button>
      </section>

      <section class="group">
        <h2>Rock Physics Status</h2>
        <div class="readout">
          chart bound: {rockPhysicsChart ? "yes" : "no"}
          model loaded: {rockPhysicsModel ? "yes" : "no"}
          tool: {lastRockPhysicsInteractionState.tool}
          last event: {lastRockPhysicsEvent}
          template label: {rockPhysicsTemplateSpec.title}
          color mode: {rockPhysicsColorMode}
          density mode: {rockPhysicsDense ? "dense" : "standard"}
          {#if rockPhysicsModel}
            template: {rockPhysicsModel.templateId}
            points: {rockPhysicsModel.pointCount}
            wells: {rockPhysicsModel.wells.length}
            color binding: {rockPhysicsModel.colorBinding.label}
            guides: {rockPhysicsModel.templateOverlays?.length ?? rockPhysicsModel.templateLines?.length ?? 0}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Rock Physics Readout</h2>
        <div class="readout">
          {#if lastRockPhysicsProbe}
            well {lastRockPhysicsProbe.wellName}
            x {formatRockPhysicsNumber(lastRockPhysicsProbe.xValue)}
            y {formatRockPhysicsNumber(lastRockPhysicsProbe.yValue)}
            depth {lastRockPhysicsProbe.sampleDepthM.toFixed(1)} m
            color {lastRockPhysicsProbe.colorValue !== undefined
              ? formatRockPhysicsNumber(lastRockPhysicsProbe.colorValue)
              : lastRockPhysicsProbe.colorCategoryLabel ?? "n/a"}
          {:else}
            Probe callbacks will appear after you move over the crossplot.
          {/if}

          {#if lastRockPhysicsViewport}
            x {formatRockPhysicsNumber(lastRockPhysicsViewport.xMin)}..{formatRockPhysicsNumber(lastRockPhysicsViewport.xMax)}
            y {formatRockPhysicsNumber(lastRockPhysicsViewport.yMin)}..{formatRockPhysicsNumber(lastRockPhysicsViewport.yMax)}
          {/if}
        </div>
      </section>
    {:else if activeDemo === "volume"}
      <section class="group">
        <h2>Volume Controls</h2>
        <button onclick={refreshVolumeScene}>Refresh Scene</button>
        <button onclick={clearVolumeScene}>Clear Scene</button>
        <button onclick={() => addRandomVolumeSlice("inline")} disabled={!volumeModel}>Add Inline Slice</button>
        <button onclick={() => addRandomVolumeSlice("xline")} disabled={!volumeModel}>Add Xline Slice</button>
        <button onclick={() => addRandomVolumeSlice("sample")} disabled={!volumeModel}>
          Add {volumeModel?.sampleDomain === "depth" ? "Depth" : "Sample"} Slice
        </button>
        <button onclick={fitVolumeToData} disabled={!volumeModel}>Fit To Data</button>
        <button onclick={setVolumeTopView} disabled={!volumeModel}>Top View</button>
        <button onclick={setVolumeSideView} disabled={!volumeModel}>Side View</button>
        <button onclick={centerVolumeSelection} disabled={!volumeModel}>Center Selection</button>
        <button onclick={() => zoomVolume(1.12)} disabled={!volumeModel}>Zoom In</button>
        <button onclick={() => zoomVolume(0.9)} disabled={!volumeModel}>Zoom Out</button>
        <button onclick={() => orbitVolume(-12, 0)} disabled={!volumeModel}>Orbit Left</button>
        <button onclick={() => orbitVolume(12, 0)} disabled={!volumeModel}>Orbit Right</button>
        <button onclick={() => panVolume(-40, 0)} disabled={!volumeModel}>Pan Left</button>
        <button onclick={() => panVolume(40, 0)} disabled={!volumeModel}>Pan Right</button>
        <button onclick={toggleVolumeRenderer}>
          Renderer: {volumeRenderer === "vtk" ? "VTK" : "Placeholder"}
        </button>
        <button onclick={toggleVolumeColormap} disabled={!volumeModel}>
          Colormap: {volumeColormap === "red-white-blue" ? "R/W/B" : "Grayscale"}
        </button>
        <button onclick={toggleVolumeContours} disabled={!volumeModel}>
          Contours: {volumeContours ? "On" : "Off"}
        </button>
        <label class="range-control">
          <span>Slice Opacity {Math.round(volumeSliceOpacity * 100)}%</span>
          <input
            type="range"
            min="0.2"
            max="1"
            step="0.05"
            bind:value={volumeSliceOpacity}
            disabled={!volumeModel}
            oninput={applyVolumeStyles}
          />
        </label>
      </section>

      <section class="group">
        <h2>Volume Status</h2>
        <div class="readout">
          chart bound: {volumeChart ? "yes" : "no"}
          scene loaded: {volumeModel ? "yes" : "no"}
          renderer: {volumeRenderer}
          tool: {lastVolumeInteractionState.tool}
          last event: {lastVolumeEvent}
          colormap: {volumeColormap}
          contours: {volumeContours ? "on" : "off"}
          slice opacity: {Math.round(volumeSliceOpacity * 100)}%
          {#if volumeModel}
            volumes: {volumeModel.volumes.length}
            slice planes: {volumeModel.slicePlanes.length}
            horizons: {volumeModel.horizons.length}
            wells: {volumeModel.wells.length}
            markers: {volumeModel.markers.length}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Volume Readout</h2>
        <div class="readout">
          {#if lastVolumeProbe}
            target {lastVolumeProbe.target.itemName ?? lastVolumeProbe.target.itemId ?? lastVolumeProbe.target.kind}
            world {formatVolumeCoordinate(lastVolumeProbe.worldX)}, {formatVolumeCoordinate(lastVolumeProbe.worldY)}, {formatVolumeCoordinate(lastVolumeProbe.worldZ)}
          {:else}
            Probe callbacks will appear after you move over the volume scene.
          {/if}

          {#if lastVolumeSelection}
            selection {lastVolumeSelection.kind} {lastVolumeSelection.itemName ?? lastVolumeSelection.itemId}
          {/if}

          {#if lastVolumeInteractionState.selectionContext}
            actions {lastVolumeInteractionState.selectionContext.allowedGestures.join(", ")}
          {/if}

          {#if lastVolumeView}
            yaw {lastVolumeView.yawDeg.toFixed(1)} pitch {lastVolumeView.pitchDeg.toFixed(1)} zoom {lastVolumeView.zoom.toFixed(2)}
          {/if}

          {lastVolumeInterpretationMessage}
        </div>
      </section>

      <section class="group">
        <h2>Volume Pick Debug</h2>
        <button onclick={copyVolumeDebugHistory} disabled={volumeDebugHistory.length === 0}>Copy Debug JSON</button>
        <button onclick={downloadVolumeDebugHistory} disabled={volumeDebugHistory.length === 0}>Download Debug JSON</button>
        <div class="readout">
          {#if lastVolumeDebugPick}
            phase {lastVolumeDebugPick.phase}
            pointer {lastVolumeDebugPick.stageX.toFixed(1)}, {lastVolumeDebugPick.stageY.toFixed(1)}
            render pointer {lastVolumeDebugPick.snapshot.renderPointerX.toFixed(1)}, {lastVolumeDebugPick.snapshot.renderPointerY.toFixed(1)}
            render scale {lastVolumeDebugPick.snapshot.renderScaleX.toFixed(2)} x {lastVolumeDebugPick.snapshot.renderScaleY.toFixed(2)}
            winner {lastVolumeDebugPick.snapshot.winner?.itemName ?? lastVolumeDebugPick.snapshot.winner?.itemId ?? "none"}
            winner target {lastVolumeDebugPick.snapshot.winner?.kind ?? "none"}
            actual winner {lastVolumeDebugPick.snapshot.actualWinner?.itemName ?? lastVolumeDebugPick.snapshot.actualWinner?.itemId ?? "none"}
            actual picked props {lastVolumeDebugPick.snapshot.actualPickedCount}
            actual matched by {lastVolumeDebugPick.snapshot.actualMatchedBy ?? "none"}
            {#if (lastVolumeDebugPick.snapshot.syntheticWinner?.itemName ?? lastVolumeDebugPick.snapshot.syntheticWinner?.itemId ?? "none") !== "none"
              && (lastVolumeDebugPick.snapshot.syntheticWinner?.itemName ?? lastVolumeDebugPick.snapshot.syntheticWinner?.itemId)
                !== (lastVolumeDebugPick.snapshot.actualWinner?.itemName ?? lastVolumeDebugPick.snapshot.actualWinner?.itemId)}
              debug synthetic {lastVolumeDebugPick.snapshot.syntheticWinner?.itemName ?? lastVolumeDebugPick.snapshot.syntheticWinner?.itemId}
            {/if}
            winner world {lastVolumeDebugPick.snapshot.winner
              ? `${formatVolumeCoordinate(lastVolumeDebugPick.snapshot.winner.worldX)}, ${formatVolumeCoordinate(lastVolumeDebugPick.snapshot.winner.worldY)}, ${formatVolumeCoordinate(lastVolumeDebugPick.snapshot.winner.worldZ)}`
              : "n/a"}
          {/if}

          {formatVolumeDebugCandidates(lastVolumeDebugPick)}
        </div>
      </section>
    {:else if activeDemo === "avo"}
      <section class="group">
        <h2>AVO Controls</h2>
        <button onclick={refreshAvo}>Refresh AVO Models</button>
        <button onclick={clearAvo}>Clear AVO Models</button>
        <button onclick={fitAvoToData} disabled={!avoResponseModel && !avoCrossplotModel && !avoChiModel}>Fit All Charts</button>
        <button onclick={cycleAvoChiAngle} disabled={!avoResponseModel && !avoCrossplotModel && !avoChiModel}>
          Cycle Chi Angle ({avoChiAngleDeg} deg)
        </button>
        <button onclick={toggleAvoDensity}>
          {avoDense ? "Load Standard Populations" : "Load Dense Populations"}
        </button>
      </section>

      <section class="group">
        <h2>AVO Status</h2>
        <div class="readout">
          response loaded: {avoResponseModel ? "yes" : "no"}
          crossplot loaded: {avoCrossplotModel ? "yes" : "no"}
          chi projection loaded: {avoChiModel ? "yes" : "no"}
          tool: {lastAvoInteractionState.tool}
          last event: {lastAvoEvent}
          chi angle: {avoChiAngleDeg} deg
          density mode: {avoDense ? "dense" : "standard"}
        </div>
      </section>

      <section class="group">
        <h2>AVO Readout</h2>
        <div class="readout">
          {#if lastAvoResponseProbe}
            response angle {lastAvoResponseProbe.angleDeg.toFixed(1)} deg
          {:else}
            Response probe: n/a
          {/if}

          {#if lastAvoCrossplotProbe}
            crossplot {lastAvoCrossplotProbe.interfaceLabel}
            intercept {lastAvoCrossplotProbe.intercept.toFixed(3)}
            gradient {lastAvoCrossplotProbe.gradient.toFixed(3)}
          {:else}
            Crossplot probe: n/a
          {/if}

          {#if lastAvoChiProbe}
            weighted stack {lastAvoChiProbe.binStart.toFixed(3)}..{lastAvoChiProbe.binEnd.toFixed(3)}
          {:else}
            Chi probe: n/a
          {/if}
        </div>
      </section>
    {:else}
      <section class="group">
        <h2>Well Panel Controls</h2>
        <button onclick={refreshCorrelationPanel}>Refresh Correlation Panel</button>
        <button onclick={clearCorrelationPanel}>Clear Correlation Panel</button>
        <button onclick={fitCorrelationToData} disabled={!correlationPanel}>Fit To Data</button>
        <button onclick={() => zoomCorrelation(1.35)} disabled={!correlationPanel}>Zoom In</button>
        <button onclick={() => zoomCorrelation(0.74)} disabled={!correlationPanel}>Zoom Out</button>
        <button onclick={() => panCorrelation(-30)} disabled={!correlationPanel}>Pan Up</button>
        <button onclick={() => panCorrelation(30)} disabled={!correlationPanel}>Pan Down</button>
      </section>

      <section class="group">
        <h2>Well Panel Status</h2>
        <div class="readout">
            chart bound: {correlationChart ? "yes" : "no"}
            panel loaded: {correlationPanel ? "yes" : "no"}
            tool: {correlationInteractions.tool}
            last event: {lastCorrelationEvent}
          {#if correlationPanel}
            wells: {correlationPanel.wells.length}
            tracks per well: {correlationPanel.wells[0]?.tracks.length ?? 0}
            scalar/trace/section: {correlationPanel.wells[0]?.tracks.filter((track) => track.kind === "scalar").length ?? 0}/
              {correlationPanel.wells[0]?.tracks.filter((track) => track.kind === "seismic-trace").length ?? 0}/
              {correlationPanel.wells[0]?.tracks.filter((track) => track.kind === "seismic-section").length ?? 0}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Well Panel Readout</h2>
        <div class="readout">
          {#if lastCorrelationProbe}
            {lastCorrelationProbe.wellName} / {lastCorrelationProbe.trackTitle}
            panel depth {lastCorrelationProbe.panelDepth.toFixed(1)}
            native depth {lastCorrelationProbe.nativeDepth.toFixed(1)}
            value {lastCorrelationProbe.markerName ?? (lastCorrelationProbe.value?.toFixed(3) ?? "n/a")}
          {:else}
            Probe callbacks will appear after you move over the correlation panel.
          {/if}

          {#if lastCorrelationViewport}
            depth {lastCorrelationViewport.depthStart.toFixed(1)}..{lastCorrelationViewport.depthEnd.toFixed(1)}
          {/if}
        </div>
      </section>

      <section class="group">
        <h2>Well Panel Debug</h2>
        <button onclick={captureCorrelationDebug} disabled={!correlationChart}>Refresh Debug JSON</button>
        <button onclick={copyCorrelationDebug} disabled={!lastCorrelationDebug}>Copy Debug JSON</button>
        <div class="readout">
          {#if lastCorrelationDebug}
            {JSON.stringify(lastCorrelationDebug, null, 2)}
          {:else}
            Debug snapshot will appear after the well panel chart mounts.
          {/if}
        </div>
      </section>
      {/if}
    </aside>
  {/if}

  <main class={["content", publicMode && "content-public", embedMode && "content-embed"]}>
    {#if publicMode && !embedMode}
      <section class="public-intro">
        <div class="public-copy">
          <p class="public-eyebrow">{activePublicDemoMeta.eyebrow}</p>
          <h1>{activePublicDemoMeta.title}</h1>
          <p>{activePublicDemoMeta.summary}</p>
        </div>
        <div class="public-nav">
          {#each LAUNCH_DEMO_ROUTES as route (route)}
            <button
              class={["demo-button", "public-nav-button", activeDemo === route && "active-demo"]}
              onclick={() => setDemoRoute(route)}
            >
              {DEMO_LABELS[route]}
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if activeDemo === "seismic"}
      <section class="card">
        <header>
          <div>
            <h2>Seismic Section Wrapper</h2>
            <p>
              This uses the public <code>SeismicSectionChart</code> wrapper, while the playground
              toggles between TWT and a synthetic depth-domain section generated from a spatially
              varying velocity field that also renders as a transparent scalar overlay.
            </p>
          </div>
        </header>
        <div class="viewer viewer-seismic">
          <div
            class="viewer-toolbar viewer-toolbar-seismic"
            style:top={seismicToolbarTop}
            style:--plot-left={seismicToolbarLeft}
            style:--plot-right={seismicToolbarRight}
          >
            <ChartInteractionToolbar
              label="Seismic interaction toolbar"
              tools={seismicToolbarTools}
              actions={seismicToolbarActions}
              onToolSelect={setSeismicTool}
              onActionSelect={runSeismicAction}
              variant="overlay"
              iconOnly={true}
            />
          </div>
          {#key `seismic:${seismicRendererEpoch}`}
            <SeismicSectionChart
              bind:this={seismicChart}
              chartId="svelte-playground-seismic"
              viewId={viewId}
              {section}
              sectionScalarOverlays={sectionScalarOverlays}
              sectionHorizons={sectionHorizons}
              sectionWellOverlays={sectionWellOverlays}
              displayTransform={{
                gain: 1.15,
                renderMode,
                colormap,
                polarity: "normal"
              }}
              interactions={seismicInteractions}
              resetToken={resetToken}
              onInteractionEvent={(payload) => {
                lastSeismicEvent = payload.event.type;
                syncSeismicRendererKinds();
              }}
              onInteractionStateChange={(event) => {
                handleSeismicInteractionStateChange(event);
                syncSeismicRendererKinds();
              }}
              onViewportChange={(event) => {
                lastViewport = event;
                syncSeismicRendererKinds();
              }}
            />
          {/key}
        </div>
      </section>
    {:else if activeDemo === "gather"}
      <section class="card">
        <header>
          <div>
            <h2>Prestack Gather Wrapper</h2>
            <p>
              This uses a public <code>SeismicGatherChart</code> wrapper with a canonical
              <code>GatherView</code> contract and the same underlying wiggle/heatmap rendering core.
            </p>
          </div>
        </header>
        <div class="viewer viewer-seismic">
          <div
            class="viewer-toolbar viewer-toolbar-seismic"
            style:top={seismicToolbarTop}
            style:--plot-left={seismicToolbarLeft}
            style:--plot-right={seismicToolbarRight}
          >
            <ChartInteractionToolbar
              label="Gather interaction toolbar"
              tools={gatherToolbarTools}
              actions={gatherToolbarActions}
              onToolSelect={setGatherTool}
              onActionSelect={runGatherAction}
              variant="overlay"
              iconOnly={true}
            />
          </div>
          {#key `gather:${seismicRendererEpoch}`}
            <SeismicGatherChart
              bind:this={gatherChart}
              chartId="svelte-playground-gather"
              viewId={gatherViewId}
              {gather}
              displayTransform={{
                gain: 1,
                renderMode: gatherRenderMode,
                colormap: gatherColormap,
                polarity: "normal"
              }}
              interactions={gatherInteractions}
              resetToken={gatherResetToken}
              onInteractionEvent={(payload) => {
                lastGatherEvent = payload.event.type;
                syncSeismicRendererKinds();
              }}
              onInteractionStateChange={(event) => {
                handleGatherInteractionStateChange(event);
                syncSeismicRendererKinds();
              }}
              onProbeChange={(event) => {
                lastGatherProbe = event.probe;
                syncSeismicRendererKinds();
              }}
              onViewportChange={(event) => {
                lastGatherViewport = event;
                syncSeismicRendererKinds();
              }}
            />
          {/key}
        </div>
      </section>
    {:else if activeDemo === "survey-map"}
      <section class="card">
        <header>
          <div>
            <h2>Survey Map Wrapper</h2>
            <p>
              This uses a dedicated <code>SurveyMapChart</code> wrapper over a canonical
              survey/well map source with scalar background, survey footprint, and well trajectories.
            </p>
          </div>
        </header>
        <div class="viewer viewer-map">
          <div
            class="viewer-toolbar viewer-toolbar-seismic"
            style:top={seismicToolbarTop}
            style:--plot-left={seismicToolbarLeft}
            style:--plot-right={seismicToolbarRight}
          >
            <ChartInteractionToolbar
              label="Survey map interaction toolbar"
              tools={surveyMapToolbarTools}
              actions={surveyMapToolbarActions}
              onToolSelect={setSurveyMapTool}
              onActionSelect={runSurveyMapAction}
              variant="overlay"
              iconOnly={true}
            />
          </div>
          <SurveyMapChart
            bind:this={surveyMapChart}
            chartId="svelte-playground-survey-map"
            map={surveyMap}
            interactions={surveyMapInteractions}
            resetToken={surveyMapResetToken}
            onInteractionEvent={(payload) => (lastSurveyMapEvent = payload.event.type)}
            onInteractionStateChange={handleSurveyMapInteractionStateChange}
            onProbeChange={(event) => (lastSurveyMapProbe = event.probe)}
            onSelectionChange={(event) => (selectedSurveyMapWellId = event.wellId)}
            onViewportChange={(event) => (lastSurveyMapViewport = event.viewport)}
          />
        </div>
      </section>
    {:else if activeDemo === "rock-physics"}
      <section class="card">
        <header>
          <div>
            <h2>Rock Physics Crossplot Wrapper</h2>
            <p>
              This uses one dedicated <code>RockPhysicsCrossplotChart</code> wrapper over the
              canonical rock-physics model, with strict standardized templates for Vp/Vs vs AI,
              AI vs SI, Vp vs Vs, porosity vs Vp, lambda-rho vs mu-rho, and neutron porosity vs bulk density.
            </p>
          </div>
        </header>
        <div class="viewer viewer-rock-physics">
          <div
            class="viewer-toolbar viewer-toolbar-seismic"
            style:top={seismicToolbarTop}
            style:--plot-left={seismicToolbarLeft}
            style:--plot-right={seismicToolbarRight}
          >
            <ChartInteractionToolbar
              label="Rock physics interaction toolbar"
              tools={rockPhysicsToolbarTools}
              actions={rockPhysicsToolbarActions}
              onToolSelect={setRockPhysicsTool}
              onActionSelect={runRockPhysicsAction}
              variant="overlay"
              iconOnly={true}
            />
          </div>
          <RockPhysicsCrossplotChart
            bind:this={rockPhysicsChart}
            chartId="svelte-playground-rock-physics"
            model={rockPhysicsModel}
            interactions={rockPhysicsInteractions}
            resetToken={rockPhysicsResetToken}
            onInteractionEvent={(payload) => (lastRockPhysicsEvent = payload.event.type)}
            onProbeChange={(event) => (lastRockPhysicsProbe = event.probe)}
            onViewportChange={(event) => (lastRockPhysicsViewport = event.viewport)}
          />
        </div>
      </section>
    {:else if activeDemo === "volume"}
      <section class="card">
        <header>
          <div>
            <h2>Volume Interpretation Wrapper</h2>
            <p>
              This exposes the public <code>VolumeInterpretationChart</code> wrapper over the resolved
              scene DTO, with VTK and placeholder renderer options, orthogonal seismic slices, horizons,
              wells, semantic interaction modes, and a stub interpretation request loop.
            </p>
          </div>
        </header>
        <div class="viewer viewer-volume">
          <div class="viewer-selection-overlay viewer-selection-overlay-volume">
            {describeVolumeSelection(lastVolumeClickedSelection)}
          </div>
          <div
            class="viewer-toolbar viewer-toolbar-seismic"
            style:top={seismicToolbarTop}
            style:--plot-left={seismicToolbarLeft}
            style:--plot-right={seismicToolbarRight}
          >
            <ChartInteractionToolbar
              label="Volume interaction toolbar"
              tools={volumeToolbarTools}
              actions={volumeToolbarActions}
              onToolSelect={setVolumeTool}
              onActionSelect={runVolumeAction}
              variant="overlay"
              iconOnly={true}
            />
          </div>
          <VolumeInterpretationChart
            bind:this={volumeChart}
            chartId="svelte-playground-volume"
            model={volumeModel}
            renderer={volumeRenderer}
            interactions={volumeInteractions}
            resetToken={volumeResetToken}
            onInteractionEvent={(payload) => (lastVolumeEvent = payload.event.type)}
            onInteractionStateChange={handleVolumeInteractionStateChange}
            onProbeChange={(event) => (lastVolumeProbe = event.probe)}
            onSelectionChange={(event) => handleVolumeSelectionChange(event.selection)}
            onViewStateChange={(event) => (lastVolumeView = event.view)}
            onDebugPick={handleVolumeDebugPick}
            onEditRequest={(event) => handleVolumeEditRequest(event.request)}
            onInterpretationRequest={(event) => handleVolumeInterpretationRequest(event.request)}
          />
        </div>
      </section>
    {:else if activeDemo === "avo"}
      <section class="card">
        <header>
          <div>
            <h2>AVO Response Wrapper</h2>
            <p>
              This line chart keeps modeled interface response semantics separate from raw compute transport,
              while sharing the same toolbar and viewport conventions as the other chart families.
            </p>
          </div>
        </header>
        <div class="viewer viewer-avo-response">
          <AvoResponseChart
            bind:this={avoResponseChart}
            chartId="svelte-playground-avo-response"
            model={avoResponseModel}
            axisOverrides={avoResponseAxisOverrides}
            interactions={avoInteractions}
            resetToken={avoResetToken}
            stageTopLeft={avoToolbarOverlay as never}
            onAxisContextRequest={(event) =>
              openAxisEditor("avoResponse", "AVO Response", event, avoResponseAxisOverrides)}
            onAxisOverridesChange={(event) => (avoResponseAxisOverrides = event.axisOverrides)}
            onInteractionEvent={(payload) => (lastAvoEvent = payload.event.type)}
            onInteractionStateChange={handleAvoInteractionStateChange}
            onProbeChange={(event) => (lastAvoResponseProbe = event.probe)}
            onViewportChange={(event) => (lastAvoResponseViewport = event.viewport)}
          />
        </div>
      </section>

      <section class="card">
        <header>
          <div>
            <h2>AVO Intercept-Gradient Crossplot</h2>
            <p>
              This is the first production AVO renderer path because it aligns directly with the existing
              point-cloud direction and the current backend maturity for intercept and gradient outputs.
            </p>
          </div>
        </header>
        <div class="viewer viewer-avo-crossplot">
          <AvoInterceptGradientCrossplotChart
            bind:this={avoCrossplotChart}
            chartId="svelte-playground-avo-crossplot"
            model={avoCrossplotModel}
            axisOverrides={avoCrossplotAxisOverrides}
            interactions={avoInteractions}
            resetToken={avoResetToken}
            stageTopLeft={avoToolbarOverlay as never}
            onAxisContextRequest={(event) =>
              openAxisEditor("avoCrossplot", "AVO Intercept-Gradient Crossplot", event, avoCrossplotAxisOverrides)}
            onAxisOverridesChange={(event) => (avoCrossplotAxisOverrides = event.axisOverrides)}
            onInteractionEvent={(payload) => (lastAvoEvent = payload.event.type)}
            onInteractionStateChange={handleAvoInteractionStateChange}
            onProbeChange={(event) => (lastAvoCrossplotProbe = event.probe)}
            onViewportChange={(event) => (lastAvoCrossplotViewport = event.viewport)}
          />
        </div>
      </section>

      <section class="card">
        <header>
          <div>
            <h2>AVO Weighted-Stack / Chi Projection</h2>
            <p>
              The weighted-stack view is treated as a chi-projection histogram so the public model stays explicit
              about what is being binned and compared.
            </p>
          </div>
        </header>
        <div class="viewer viewer-avo-histogram">
          <AvoChiProjectionHistogramChart
            bind:this={avoHistogramChart}
            chartId="svelte-playground-avo-chi"
            model={avoChiModel}
            axisOverrides={avoChiAxisOverrides}
            interactions={avoInteractions}
            resetToken={avoResetToken}
            stageTopLeft={avoToolbarOverlay as never}
            onAxisContextRequest={(event) =>
              openAxisEditor("avoChi", "AVO Weighted-Stack / Chi Projection", event, avoChiAxisOverrides)}
            onAxisOverridesChange={(event) => (avoChiAxisOverrides = event.axisOverrides)}
            onInteractionEvent={(payload) => (lastAvoEvent = payload.event.type)}
            onInteractionStateChange={handleAvoInteractionStateChange}
            onProbeChange={(event) => (lastAvoChiProbe = event.probe)}
            onViewportChange={(event) => (lastAvoChiViewport = event.viewport)}
          />
        </div>
      </section>
    {:else}
      <section class="card">
        <header>
          <div>
            <h2>Well Correlation Wrapper</h2>
            <p>
              This now uses the layered <code>WellPanelModel</code> path with scalar tracks, point overlays,
              multi-trace wiggles, and seismic section lanes.
            </p>
          </div>
        </header>
        <div class="viewer viewer-correlation">
          <WellCorrelationPanelChart
            bind:this={correlationChart}
            chartId="svelte-playground-correlation"
            panel={correlationPanel}
            interactions={correlationInteractions}
            resetToken={correlationResetToken}
            stageTopLeft={correlationToolbarOverlay as never}
            onInteractionEvent={(payload) => (lastCorrelationEvent = payload.event.type)}
            onInteractionStateChange={handleCorrelationInteractionStateChange}
            onProbeChange={(event) => (lastCorrelationProbe = event.probe)}
            onViewportChange={(event) => (lastCorrelationViewport = event.viewport)}
          />
        </div>
      </section>
    {/if}
  </main>

  {#if axisEditor}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="axis-editor-backdrop" onclick={closeAxisEditor}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="axis-editor-panel"
        style:left={`${Math.max(16, axisEditor.clientX - 140)}px`}
        style:top={`${Math.max(16, axisEditor.clientY - 12)}px`}
        onclick={(event) => event.stopPropagation()}
      >
        <div class="axis-editor-header">
          <div>
            <h2>{axisEditor.chartLabel}</h2>
            <p>{axisEditor.axis.toUpperCase()} axis</p>
          </div>
          <button type="button" class="axis-editor-close" onclick={closeAxisEditor}>Close</button>
        </div>

        <label class="axis-editor-field">
          <span>Label</span>
          <input
            type="text"
            value={axisEditor.form.label}
            oninput={(event) => updateAxisEditorField("label", event.currentTarget.value)}
          />
        </label>

        <label class="axis-editor-field">
          <span>Unit</span>
          <input
            type="text"
            value={axisEditor.form.unit}
            oninput={(event) => updateAxisEditorField("unit", event.currentTarget.value)}
          />
        </label>

        <div class="axis-editor-grid">
          <label class="axis-editor-field">
            <span>Min</span>
            <input
              type="number"
              step="any"
              value={axisEditor.form.min}
              oninput={(event) => updateAxisEditorField("min", event.currentTarget.value)}
            />
          </label>
          <label class="axis-editor-field">
            <span>Max</span>
            <input
              type="number"
              step="any"
              value={axisEditor.form.max}
              oninput={(event) => updateAxisEditorField("max", event.currentTarget.value)}
            />
          </label>
        </div>

        <div class="axis-editor-grid">
          <label class="axis-editor-field">
            <span>Tick Count</span>
            <input
              type="number"
              min="2"
              step="1"
              value={axisEditor.form.tickCount}
              oninput={(event) => updateAxisEditorField("tickCount", event.currentTarget.value)}
            />
          </label>
          <label class="axis-editor-field">
            <span>Tick Format</span>
            <select
              value={axisEditor.form.tickFormat}
              onchange={(event) => updateAxisEditorField("tickFormat", event.currentTarget.value)}
            >
              <option value="auto">Auto</option>
              <option value="fixed:0">Fixed 0</option>
              <option value="fixed:1">Fixed 1</option>
              <option value="fixed:2">Fixed 2</option>
              <option value="fixed:3">Fixed 3</option>
              <option value="fixed:4">Fixed 4</option>
              <option value="scientific">Scientific</option>
            </select>
          </label>
        </div>

        <div class="axis-editor-actions">
          <button type="button" class="axis-editor-secondary" onclick={resetAxisEditorAxis}>Reset Axis</button>
          <div class="axis-editor-action-group">
            <button type="button" class="axis-editor-secondary" onclick={closeAxisEditor}>Cancel</button>
            <button type="button" onclick={applyAxisEditor}>Apply</button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
    margin: 0;
  }

  :global(body) {
    background: #0b1720;
    color: #edf2f5;
  }

  .layout {
    display: grid;
    grid-template-columns: 340px 1fr;
    min-height: 100%;
    font-family: var(--demo-font-family);
    background: var(--demo-shell-bg);
    color: var(--demo-shell-text);
  }

  .sidebar {
    padding: 22px;
    border-right: 1px solid var(--demo-sidebar-border);
    background: var(--demo-sidebar-bg);
    display: grid;
    align-content: start;
    gap: 18px;
  }

  .layout-public {
    grid-template-columns: 1fr;
    min-height: 100vh;
  }

  .layout-embed {
    min-height: auto;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    font-size: 28px;
  }

  p {
    color: var(--demo-text-muted);
    line-height: 1.45;
  }

  code {
    font-family: "SF Mono", "Menlo", monospace;
    font-size: 0.92em;
  }

  .group {
    display: grid;
    gap: 8px;
  }

  .group h2 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--demo-group-title);
  }

  button {
    padding: 10px 12px;
    border-radius: var(--demo-radius-md);
    border: 1px solid var(--demo-button-border);
    font: inherit;
    background: var(--demo-button-bg);
    color: var(--demo-button-text);
    font-weight: 600;
    cursor: pointer;
    box-shadow: var(--demo-button-shadow);
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease,
      opacity 120ms ease;
  }

  button.active-demo {
    background: var(--demo-button-bg-active);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
    background: var(--demo-button-bg-disabled);
    box-shadow: none;
  }

  .range-control {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: var(--demo-shell-text);
  }

  .demo-field {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: var(--demo-shell-text);
  }

  .demo-field select {
    min-width: 0;
    padding: 10px 12px;
    border-radius: var(--demo-radius-md);
    border: 1px solid var(--demo-button-border);
    background: var(--demo-button-bg);
    color: var(--demo-button-text);
    font: inherit;
    box-shadow: var(--demo-button-shadow);
  }

  .demo-field select:focus {
    outline: none;
    border-color: var(--demo-input-border-focus);
    box-shadow: 0 0 0 3px rgba(134, 178, 203, 0.18);
  }

  .range-control input {
    width: 100%;
  }

  .readout {
    padding: 10px 12px;
    border-radius: var(--demo-radius-md);
    border: 1px solid var(--demo-readout-border);
    background: var(--demo-readout-bg);
    white-space: pre-wrap;
    font-size: 12px;
    line-height: 1.45;
  }

  .template-picker {
    display: grid;
    gap: 8px;
    margin-top: 4px;
  }

  .template-picker-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--demo-text-muted);
  }

  .content {
    padding: 22px;
    display: grid;
    gap: 22px;
  }

  .content-public {
    width: min(1440px, 100%);
    margin: 0 auto;
    padding: 28px;
    gap: 18px;
  }

  .content-embed {
    width: 100%;
    margin: 0;
    padding: 0;
    gap: 0;
  }

  .public-intro {
    display: grid;
    gap: 18px;
    padding: 10px 0 2px;
  }

  .public-copy {
    display: grid;
    gap: 8px;
    max-width: 760px;
  }

  .public-eyebrow {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--demo-group-title);
  }

  .public-nav {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .public-nav-button {
    min-width: 0;
  }

  .card {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 10px;
    min-height: 480px;
  }

  .content-public .card {
    min-height: 0;
  }

  .content-embed .card {
    min-height: 0;
    gap: 0;
  }

  .content-embed .card > header {
    display: none;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
    flex-wrap: wrap;
  }

  .card h2 {
    font-size: 22px;
  }

  .viewer {
    position: relative;
    min-height: 520px;
    border-radius: var(--demo-radius-md);
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--demo-viewer-border);
  }

  .viewer-seismic {
    background: rgba(3, 9, 14, 0.5);
  }

  .viewer-correlation {
    background: rgba(241, 233, 221, 0.76);
  }

  .viewer-map {
    background: rgba(241, 236, 228, 0.88);
  }

  .viewer-rock-physics {
    background: rgba(6, 20, 28, 0.9);
  }

  .viewer-volume {
    background: rgba(8, 16, 24, 0.96);
  }

  .viewer-avo-response {
    background: rgba(235, 239, 245, 0.96);
  }

  .viewer-avo-crossplot {
    background: rgba(235, 241, 229, 0.96);
  }

  .viewer-avo-histogram {
    background: rgba(243, 239, 228, 0.96);
  }

  .viewer-toolbar {
    position: absolute;
    z-index: 2;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .viewer-toolbar :global(.toolbar-shell) {
    pointer-events: auto;
  }

  .viewer-toolbar-seismic {
    left: var(--plot-left);
    right: var(--plot-right);
  }

  .viewer-selection-overlay {
    position: absolute;
    top: 16px;
    left: 16px;
    z-index: 2;
    max-width: min(320px, calc(100% - 32px));
    padding: 9px 11px;
    border: 1px solid var(--demo-selection-border);
    border-radius: var(--demo-radius-md);
    background: var(--demo-selection-bg);
    color: var(--demo-selection-text);
    font: 600 12px/1.35 var(--demo-font-family);
    pointer-events: none;
  }

  .viewer-selection-overlay-volume {
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.2);
  }

  .axis-editor-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    background: var(--demo-modal-backdrop);
  }

  .axis-editor-panel {
    position: fixed;
    width: min(336px, calc(100vw - 32px));
    display: grid;
    gap: 14px;
    padding: 14px;
    border-radius: var(--demo-radius-md);
    border: 1px solid var(--demo-modal-border);
    background: var(--demo-modal-bg);
    box-shadow: var(--demo-modal-shadow);
    color: var(--demo-modal-text);
  }

  .axis-editor-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .axis-editor-header h2 {
    font-size: 14px;
    line-height: 1.1;
  }

  .axis-editor-header p {
    margin-top: 4px;
    font-size: 11px;
    line-height: 1.2;
    color: var(--demo-modal-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .axis-editor-close {
    background: var(--demo-secondary-bg);
    border-color: var(--demo-secondary-border);
    color: var(--demo-secondary-text);
    box-shadow: none;
  }

  .axis-editor-grid {
    display: grid;
    gap: 10px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .axis-editor-field {
    display: grid;
    gap: 6px;
    font-size: 11px;
    line-height: 1.2;
    color: var(--demo-modal-muted);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .axis-editor-field input,
  .axis-editor-field select {
    min-width: 0;
    padding: 9px 10px;
    border-radius: var(--demo-radius-sm);
    border: 1px solid var(--demo-input-border);
    background: var(--demo-input-bg);
    color: var(--demo-input-text);
    font: inherit;
    font-size: 12px;
    line-height: 1.25;
    box-sizing: border-box;
  }

  .axis-editor-field input::placeholder {
    color: var(--demo-input-placeholder);
  }

  .axis-editor-field input:focus,
  .axis-editor-field select:focus {
    outline: none;
    border-color: var(--demo-input-border-focus);
    box-shadow: 0 0 0 3px rgba(134, 178, 203, 0.18);
  }

  .axis-editor-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .axis-editor-action-group {
    display: flex;
    gap: 8px;
  }

  .axis-editor-secondary {
    background: var(--demo-secondary-bg);
    border-color: var(--demo-secondary-border);
    color: var(--demo-secondary-text);
    box-shadow: none;
  }

  .axis-editor-actions button:not(.axis-editor-secondary) {
    background: var(--demo-primary-bg);
    border-color: var(--demo-primary-border);
    color: var(--demo-primary-text);
    box-shadow: none;
  }

  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }

    header {
      align-items: stretch;
    }

    .viewer {
      min-height: 420px;
    }

    .content-public {
      padding: 18px;
    }

    .axis-editor-grid,
    .axis-editor-actions {
      grid-template-columns: 1fr;
      justify-items: stretch;
    }

    .axis-editor-actions,
    .axis-editor-action-group {
      display: grid;
    }
  }
</style>
