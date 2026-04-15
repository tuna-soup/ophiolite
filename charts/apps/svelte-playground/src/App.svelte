<svelte:options runes={true} />

<script lang="ts">
  import {
    createMockAvoChiProjectionModel,
    createMockAvoCrossplotModel,
    createMockAvoResponseModel,
    MOCK_SECTION_VELOCITY_MODEL_LABEL,
    STANDARD_ROCK_PHYSICS_TEMPLATE_IDS,
    type AvoCartesianViewport,
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
    type RockPhysicsCrossplotViewport,
    type RockPhysicsMockColorMode,
    type RockPhysicsMockOptions,
    type SectionHorizonOverlay,
    type SectionScalarOverlay,
    type SectionScalarOverlayColorMap,
    type SectionWellOverlay,
    type SurveyMapModel,
    type SurveyMapProbe,
    type SurveyMapViewport,
    type VolumeInterpretationColorMap,
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
    type AvoChartInteractionConfig,
    type AvoChartInteractionState,
    type AvoChartTool,
    type RockPhysicsCrossplotChartAction,
    type RockPhysicsCrossplotChartInteractionConfig,
    type RockPhysicsCrossplotChartInteractionState,
    type RockPhysicsCrossplotChartTool,
    type SeismicChartAction,
    type SeismicChartInteractionConfig,
    type SeismicChartInteractionState,
    type SeismicChartTool,
    type SurveyMapChartAction,
    type SurveyMapChartInteractionConfig,
    type SurveyMapChartInteractionState,
    type SurveyMapChartTool,
    type VolumeInterpretationChartAction,
    type VolumeInterpretationChartInteractionConfig,
    type VolumeInterpretationChartInteractionState,
    type VolumeInterpretationChartRenderer,
    type VolumeInterpretationChartTool,
    type WellCorrelationChartAction,
    type WellCorrelationChartInteractionConfig,
    type WellCorrelationChartInteractionState,
    type WellCorrelationChartTool
  } from "@ophiolite/charts";
  import {
    ChartInteractionToolbar,
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

  interface SeismicChartHandle {
    fitToData?: () => void;
  }

  interface GatherChartHandle {
    fitToData?: () => void;
  }

  interface CorrelationChartHandle {
    fitToData?: () => void;
    zoomBy?: (factor: number) => void;
    panBy?: (deltaDepth: number) => void;
  }

  interface SurveyMapChartHandle {
    fitToData?: () => void;
    zoomBy?: (factor: number) => void;
  }

  interface RockPhysicsChartHandle {
    fitToData?: () => void;
    zoomBy?: (factor: number) => void;
    panBy?: (deltaX: number, deltaY: number) => void;
  }

  interface VolumeInterpretationChartHandle {
    fitToData?: () => void;
    resetView?: () => void;
    centerSelection?: () => void;
    zoomBy?: (factor: number) => void;
    orbitBy?: (deltaYawDeg: number, deltaPitchDeg: number) => void;
    panBy?: (deltaX: number, deltaY: number) => void;
  }

  interface AvoChartHandle {
    fitToData?: () => void;
    zoomBy?: (factor: number) => void;
    panBy?: (deltaX: number, deltaY: number) => void;
  }

  type DemoRoute = "seismic" | "gather" | "survey-map" | "rock-physics" | "volume" | "avo" | "well-panel";

  let seismicChart = $state.raw<SeismicChartHandle | null>(null);
  let gatherChart = $state.raw<GatherChartHandle | null>(null);
  let surveyMapChart = $state.raw<SurveyMapChartHandle | null>(null);
  let correlationChart = $state.raw<CorrelationChartHandle | null>(null);
  let rockPhysicsChart = $state.raw<RockPhysicsChartHandle | null>(null);
  let volumeChart = $state.raw<VolumeInterpretationChartHandle | null>(null);
  let avoResponseChart = $state.raw<AvoChartHandle | null>(null);
  let avoCrossplotChart = $state.raw<AvoChartHandle | null>(null);
  let avoHistogramChart = $state.raw<AvoChartHandle | null>(null);
  let activeDemo = $state<DemoRoute>(getDemoRoute());

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

  let rockPhysicsTemplateId = $state<(typeof STANDARD_ROCK_PHYSICS_TEMPLATE_IDS)[number]>("vp-vs-vs-ai");
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

  let volumeColormap = $state<VolumeInterpretationColorMap>("red-white-blue");
  let volumeSliceOpacity = $state(0.94);
  let volumeContours = $state(true);
  let volumeRenderer = $state<VolumeInterpretationChartRenderer>("vtk");
  let volumeModel = $state.raw<VolumeInterpretationModel | null>(createVolumeSceneModel());
  let volumeResetToken = $state(0);
  let volumeInteractions = $state.raw<VolumeInterpretationChartInteractionConfig>({
    tool: "pointer"
  });
  let lastVolumeInteractionState = $state.raw<VolumeInterpretationChartInteractionState>({
    capabilities: {
      tools: [...VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.tools],
      actions: [...VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.actions]
    },
    tool: "pointer"
  });
  let lastVolumeEvent = $state("none");
  let lastVolumeProbe = $state.raw<VolumeInterpretationProbe | null>(null);
  let lastVolumeSelection = $state.raw<VolumeInterpretationSelection | null>(null);
  let lastVolumeView = $state.raw<VolumeInterpretationView | null>(null);
  let lastVolumeInterpretationMessage = $state("No interpretation request yet.");

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
      active: lastCorrelationInteractionState.tool === tool,
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
      icon:
        tool === "interpret-seed"
          ? "crosshair"
          : tool === "orbit"
            ? "orbit"
            : tool === "slice-drag" || tool === "crop"
            ? "pan"
            : tool === "select"
              ? "pointer"
              : tool,
      active: lastVolumeInteractionState.tool === tool,
      disabled: !volumeModel
    }))
  );
  let volumeToolbarActions = $derived.by<ChartToolbarActionItem<VolumeInterpretationChartAction>[]>(() =>
    VOLUME_INTERPRETATION_CHART_INTERACTION_CAPABILITIES.actions.map((action) => ({
      id: action,
      label: action === "fitToData" ? "Fit" : action === "resetView" ? "Reset" : "Center",
      icon: "fitToData",
      disabled: !volumeModel
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

  function runCorrelationAction(action: string) {
    if (action === "fitToData") {
      fitCorrelationToData();
    }
  }

  function handleCorrelationInteractionStateChange(event: WellCorrelationChartInteractionState) {
    lastCorrelationInteractionState = event;
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
    rockPhysicsResetToken += 1;
  }

  function clearRockPhysics() {
    rockPhysicsModel = null;
    lastRockPhysicsViewport = null;
    lastRockPhysicsProbe = null;
  }

  function toggleRockPhysicsColorMode() {
    const modes = rockPhysicsColorModes;
    const currentIndex = Math.max(0, modes.indexOf(rockPhysicsColorMode));
    rockPhysicsColorMode = modes[(currentIndex + 1) % modes.length]!;
    refreshRockPhysics();
  }

  function cycleRockPhysicsTemplate() {
    const currentIndex = STANDARD_ROCK_PHYSICS_TEMPLATE_IDS.indexOf(rockPhysicsTemplateId);
    const nextIndex = (currentIndex + 1) % STANDARD_ROCK_PHYSICS_TEMPLATE_IDS.length;
    rockPhysicsTemplateId = STANDARD_ROCK_PHYSICS_TEMPLATE_IDS[nextIndex]!;
    rockPhysicsColorMode = getDefaultRockPhysicsMockColorMode(rockPhysicsTemplateId);
    refreshRockPhysics();
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
    lastVolumeInterpretationMessage = "Scene refreshed.";
  }

  function clearVolumeScene() {
    volumeModel = null;
    lastVolumeProbe = null;
    lastVolumeSelection = null;
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
    } else if (action === "resetView") {
      resetVolumeView();
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

  function resetVolumeView() {
    volumeChart?.resetView?.();
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
    avoResetToken += 1;
  }

  function clearAvo() {
    avoResponseModel = null;
    avoCrossplotModel = null;
    avoChiModel = null;
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

  function setDemoRoute(next: DemoRoute) {
    activeDemo = next;
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

  function handleHashChange() {
    activeDemo = getDemoRoute();
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
</script>

<svelte:head>
  <title>Ophiolite Charts Svelte Playground</title>
</svelte:head>

<svelte:window onhashchange={handleHashChange} />

<div class="layout">
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
          {#if lastGatherProbe}
            trace {lastGatherProbe.trace_index} ({lastGatherProbe.trace_coordinate.toFixed(2)})
            sample {lastGatherProbe.sample_index} ({lastGatherProbe.sample_value.toFixed(1)})
            amplitude {lastGatherProbe.amplitude.toFixed(4)}
          {:else}
            Probe callbacks will appear after you move over the gather.
          {/if}

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
          {#if lastSurveyMapProbe}
            x {lastSurveyMapProbe.x.toFixed(0)}
            y {lastSurveyMapProbe.y.toFixed(0)}
            well {lastSurveyMapProbe.wellName ?? "n/a"}
            value {lastSurveyMapProbe.scalarValue?.toFixed(1) ?? "n/a"}
          {:else}
            Probe callbacks will appear after you move over the survey map.
          {/if}

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
        <button onclick={fitVolumeToData} disabled={!volumeModel}>Fit To Data</button>
        <button onclick={resetVolumeView} disabled={!volumeModel}>Reset View</button>
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

          {#if lastVolumeView}
            yaw {lastVolumeView.yawDeg.toFixed(1)} pitch {lastVolumeView.pitchDeg.toFixed(1)} zoom {lastVolumeView.zoom.toFixed(2)}
          {/if}

          {lastVolumeInterpretationMessage}
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
            tool: {lastCorrelationInteractionState.tool}
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
    {/if}
  </aside>

  <main class="content">
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
            onInteractionEvent={(payload) => (lastSeismicEvent = payload.event.type)}
            onInteractionStateChange={handleSeismicInteractionStateChange}
            onViewportChange={(event) => (lastViewport = event)}
          />
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
            onInteractionEvent={(payload) => (lastGatherEvent = payload.event.type)}
            onInteractionStateChange={handleGatherInteractionStateChange}
            onProbeChange={(event) => (lastGatherProbe = event.probe)}
            onViewportChange={(event) => (lastGatherViewport = event)}
          />
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
            onInteractionStateChange={handleRockPhysicsInteractionStateChange}
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
            onSelectionChange={(event) => (lastVolumeSelection = event.selection)}
            onViewStateChange={(event) => (lastVolumeView = event.view)}
            onInterpretationRequest={(event) => handleVolumeInterpretationRequest(event.request)}
          />
        </div>
      </section>
    {:else if activeDemo === "avo"}
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
            interactions={avoInteractions}
            resetToken={avoResetToken}
            stageTopLeft={avoToolbarOverlay}
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
            interactions={avoInteractions}
            resetToken={avoResetToken}
            stageTopLeft={avoToolbarOverlay}
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
            interactions={avoInteractions}
            resetToken={avoResetToken}
            stageTopLeft={avoToolbarOverlay}
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
        <div class="viewer viewer-correlation">
          <WellCorrelationPanelChart
            bind:this={correlationChart}
            chartId="svelte-playground-correlation"
            panel={correlationPanel}
            interactions={correlationInteractions}
            resetToken={correlationResetToken}
            stageTopLeft={correlationToolbarOverlay}
            onInteractionEvent={(payload) => (lastCorrelationEvent = payload.event.type)}
            onInteractionStateChange={handleCorrelationInteractionStateChange}
            onProbeChange={(event) => (lastCorrelationProbe = event.probe)}
            onViewportChange={(event) => (lastCorrelationViewport = event.viewport)}
          />
        </div>
      </section>
    {/if}
  </main>
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
    margin: 0;
  }

  :global(body) {
    font-family: "Segoe UI", sans-serif;
    background: linear-gradient(160deg, #07131b 0%, #102534 45%, #17384c 100%);
    color: #edf2f5;
  }

  .layout {
    display: grid;
    grid-template-columns: 340px 1fr;
    min-height: 100%;
  }

  .sidebar {
    padding: 22px;
    border-right: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(5, 10, 16, 0.48);
    display: grid;
    align-content: start;
    gap: 18px;
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
    color: #c7d6de;
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
    color: #bfd0da;
  }

  button {
    padding: 10px 12px;
    border-radius: 10px;
    border: 0;
    font: inherit;
    background: #e8eff3;
    color: #07131b;
    font-weight: 600;
    cursor: pointer;
  }

  button.active-demo {
    background: #9dd9ff;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .range-control {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: #d6e4eb;
  }

  .range-control input {
    width: 100%;
  }

  .readout {
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.22);
    white-space: pre-wrap;
    font-size: 12px;
    line-height: 1.45;
  }

  .content {
    padding: 22px;
    display: grid;
    gap: 22px;
  }

  .card {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 10px;
    min-height: 480px;
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
    border-radius: 18px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
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
  }
</style>
