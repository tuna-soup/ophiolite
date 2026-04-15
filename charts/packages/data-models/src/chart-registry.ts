export type ChartFamilyId = "seismic" | "well-panel" | "survey-map" | "rock-physics";

export type ChartDefinitionId =
  | "seismic-section"
  | "seismic-gather"
  | "well-correlation-panel"
  | "survey-map"
  | "rock-physics-crossplot";

export type ChartRendererKernelId = "raster-trace" | "well-panel" | "survey-map" | "point-cloud";

export type ChartPublicSurfaceId =
  | "SeismicSectionChart"
  | "SeismicGatherChart"
  | "WellCorrelationPanelChart"
  | "SurveyMapChart"
  | "RockPhysicsCrossplotChart";

export type ChartCanonicalBoundaryId =
  | "ophiolite-section-view"
  | "ophiolite-gather-view"
  | "ophiolite-well-panel-source"
  | "ophiolite-survey-map-source"
  | "ophiolite-rock-physics-crossplot-source";

export type ChartAssetFamilyId =
  | "seismic-section-amplitudes"
  | "seismic-gather-amplitudes"
  | "seismic-section-overlay-grid"
  | "seismic-horizon-overlay"
  | "seismic-well-overlay"
  | "well-log-curve"
  | "well-trajectory"
  | "well-top-set"
  | "well-pressure-observation"
  | "well-drilling-observation"
  | "well-seismic-trace-set"
  | "well-seismic-section"
  | "survey-outline"
  | "survey-well-location"
  | "survey-plan-trajectory"
  | "survey-scalar-field"
  | "rock-physics-log-samples"
  | "rock-physics-template-lines"
  | "rock-physics-categorical-color-binding"
  | "rock-physics-continuous-color-binding";

export type ChartInteractionToolId = "pointer" | "crosshair" | "pan";
export type ChartInteractionActionId = "fitToData";

export interface ChartInteractionProfile {
  tools: readonly ChartInteractionToolId[];
  actions: readonly ChartInteractionActionId[];
}

export interface ChartDefinition {
  id: ChartDefinitionId;
  familyId: ChartFamilyId;
  label: string;
  summary: string;
  publicSurface: ChartPublicSurfaceId;
  rendererKernel: ChartRendererKernelId;
  canonicalBoundaries: readonly ChartCanonicalBoundaryId[];
  allowedAssetFamilies: readonly ChartAssetFamilyId[];
  interactionProfile: ChartInteractionProfile;
  adapterEntryPoints: readonly string[];
  validationEntryPoints: readonly string[];
  constraints: readonly string[];
}

export interface ChartFamilyDefinition {
  id: ChartFamilyId;
  label: string;
  summary: string;
  chartIds: readonly ChartDefinitionId[];
  rendererKernels: readonly ChartRendererKernelId[];
  canonicalBoundaries: readonly ChartCanonicalBoundaryId[];
}

const SEISMIC_INTERACTION_PROFILE = {
  tools: ["pointer", "crosshair", "pan"],
  actions: ["fitToData"]
} as const satisfies ChartInteractionProfile;

const WELL_PANEL_INTERACTION_PROFILE = {
  tools: ["pointer", "crosshair", "pan"],
  actions: ["fitToData"]
} as const satisfies ChartInteractionProfile;

const SURVEY_MAP_INTERACTION_PROFILE = {
  tools: ["pointer", "pan"],
  actions: ["fitToData"]
} as const satisfies ChartInteractionProfile;

const ROCK_PHYSICS_INTERACTION_PROFILE = {
  tools: ["pointer", "crosshair", "pan"],
  actions: ["fitToData"]
} as const satisfies ChartInteractionProfile;

export const CHART_DEFINITIONS = [
  {
    id: "seismic-section",
    familyId: "seismic",
    label: "Seismic Section",
    summary: "2D seismic section renderer with section overlays, well overlays, and shared probe interactions.",
    publicSurface: "SeismicSectionChart",
    rendererKernel: "raster-trace",
    canonicalBoundaries: ["ophiolite-section-view"],
    allowedAssetFamilies: [
      "seismic-section-amplitudes",
      "seismic-section-overlay-grid",
      "seismic-horizon-overlay",
      "seismic-well-overlay"
    ],
    interactionProfile: SEISMIC_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteSectionViewToPayload"],
    validationEntryPoints: ["validateSectionPayload"],
    constraints: [
      "Accepts only Ophiolite section-view contracts adapted into chart payloads.",
      "Allows section overlays, horizon overlays, and well overlays tied to the section domain.",
      "Rejects gathers, survey-map sources, arbitrary well-log curves, pressure tables, and generic tabular samples."
    ]
  },
  {
    id: "seismic-gather",
    familyId: "seismic",
    label: "Seismic Gather",
    summary: "Prestack gather renderer over the same raster-trace kernel used for sections.",
    publicSurface: "SeismicGatherChart",
    rendererKernel: "raster-trace",
    canonicalBoundaries: ["ophiolite-gather-view"],
    allowedAssetFamilies: ["seismic-gather-amplitudes"],
    interactionProfile: SEISMIC_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteGatherViewToPayload"],
    validationEntryPoints: ["validateGatherPayload"],
    constraints: [
      "Accepts only Ophiolite gather-view contracts adapted into gather payloads.",
      "Shares the raster-trace kernel and interaction profile with seismic sections.",
      "Rejects section overlays, well-log tracks, survey assets, and non-seismic sample tables."
    ]
  },
  {
    id: "well-correlation-panel",
    familyId: "well-panel",
    label: "Well Correlation Panel",
    summary: "Multi-well panel with explicit track taxonomy for logs, tops, trajectories, point observations, and embedded seismic.",
    publicSurface: "WellCorrelationPanelChart",
    rendererKernel: "well-panel",
    canonicalBoundaries: ["ophiolite-well-panel-source"],
    allowedAssetFamilies: [
      "well-log-curve",
      "well-trajectory",
      "well-top-set",
      "well-pressure-observation",
      "well-drilling-observation",
      "well-seismic-trace-set",
      "well-seismic-section"
    ],
    interactionProfile: WELL_PANEL_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteWellPanelToChart"],
    validationEntryPoints: ["validateOphioliteWellPanelSource", "validateSectionPayload"],
    constraints: [
      "Accepts only resolved Ophiolite well-panel sources plus an explicit chart layout.",
      "Track families constrain what may be rendered: scalar tracks for curves and point observations, seismic-trace tracks for trace sets, and seismic-section tracks for embedded sections.",
      "Embedded seismic sections must also satisfy the seismic section payload validator.",
      "Rejects survey maps, standalone gathers, and unconstrained generic x/y sample tables."
    ]
  },
  {
    id: "survey-map",
    familyId: "survey-map",
    label: "Survey Map",
    summary: "Plan-view map for survey outlines, wells, trajectories, and optional scalar grids.",
    publicSurface: "SurveyMapChart",
    rendererKernel: "survey-map",
    canonicalBoundaries: ["ophiolite-survey-map-source"],
    allowedAssetFamilies: ["survey-outline", "survey-well-location", "survey-plan-trajectory", "survey-scalar-field"],
    interactionProfile: SURVEY_MAP_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteSurveyMapToChart"],
    validationEntryPoints: ["validateOphioliteSurveyMapSource"],
    constraints: [
      "Accepts only resolved Ophiolite survey-map sources.",
      "Allows survey outlines, well surface locations, plan trajectories, and optional scalar grids in map coordinates.",
      "Rejects seismic amplitude payloads, well-panel track payloads, and rock-physics sample clouds."
    ]
  },
  {
    id: "rock-physics-crossplot",
    familyId: "rock-physics",
    label: "Rock Physics Crossplot",
    summary: "Point-cloud crossplot for well-log-derived rock-physics samples with template-scoped axis semantics and color bindings.",
    publicSurface: "RockPhysicsCrossplotChart",
    rendererKernel: "point-cloud",
    canonicalBoundaries: ["ophiolite-rock-physics-crossplot-source"],
    allowedAssetFamilies: [
      "rock-physics-log-samples",
      "rock-physics-template-lines",
      "rock-physics-categorical-color-binding",
      "rock-physics-continuous-color-binding"
    ],
    interactionProfile: ROCK_PHYSICS_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteRockPhysicsCrossplotToChart"],
    validationEntryPoints: ["validateOphioliteRockPhysicsCrossplotSource"],
    constraints: [
      "Accepts only resolved Ophiolite rock-physics crossplot sources materialized from well-log samples.",
      "Axis semantics are template-scoped, so a Vp/Vs vs AI template cannot bind arbitrary log types such as resistivity on the x or y axis.",
      "Color bindings are restricted to the template-approved continuous and categorical semantics.",
      "Rejects seismic contracts, survey-map sources, pressure tables, and generic non-log sample payloads."
    ]
  }
] as const satisfies readonly ChartDefinition[];

export const CHART_FAMILIES = [
  {
    id: "seismic",
    label: "Seismic",
    summary: "Raster-trace charts for section and prestack seismic views.",
    chartIds: ["seismic-section", "seismic-gather"],
    rendererKernels: ["raster-trace"],
    canonicalBoundaries: ["ophiolite-section-view", "ophiolite-gather-view"]
  },
  {
    id: "well-panel",
    label: "Well Panel",
    summary: "Track-based well interpretation charts with explicit layer and asset constraints.",
    chartIds: ["well-correlation-panel"],
    rendererKernels: ["well-panel"],
    canonicalBoundaries: ["ophiolite-well-panel-source"]
  },
  {
    id: "survey-map",
    label: "Survey Map",
    summary: "Map-space charts for outlines, well locations, and scalar grids.",
    chartIds: ["survey-map"],
    rendererKernels: ["survey-map"],
    canonicalBoundaries: ["ophiolite-survey-map-source"]
  },
  {
    id: "rock-physics",
    label: "Rock Physics",
    summary: "Point-cloud analysis charts for template-driven crossplots derived from log samples.",
    chartIds: ["rock-physics-crossplot"],
    rendererKernels: ["point-cloud"],
    canonicalBoundaries: ["ophiolite-rock-physics-crossplot-source"]
  }
] as const satisfies readonly ChartFamilyDefinition[];

const CHART_DEFINITION_MAP = new Map(CHART_DEFINITIONS.map((definition) => [definition.id, definition]));
const CHART_FAMILY_MAP = new Map(CHART_FAMILIES.map((family) => [family.id, family]));

export function getChartDefinition(id: ChartDefinitionId): ChartDefinition {
  const definition = CHART_DEFINITION_MAP.get(id);
  if (!definition) {
    throw new Error(`Unknown chart definition '${id}'.`);
  }
  return definition;
}

export function getChartFamilyDefinition(id: ChartFamilyId): ChartFamilyDefinition {
  const definition = CHART_FAMILY_MAP.get(id);
  if (!definition) {
    throw new Error(`Unknown chart family '${id}'.`);
  }
  return definition;
}

export function listChartDefinitionsByFamily(familyId: ChartFamilyId): ChartDefinition[] {
  return CHART_DEFINITIONS.filter((definition) => definition.familyId === familyId);
}
