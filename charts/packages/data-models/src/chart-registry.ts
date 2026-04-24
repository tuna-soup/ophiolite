export type ChartFamilyId = "seismic" | "well-panel" | "survey-map" | "rock-physics" | "avo" | "volume-interpretation";

export type ChartDefinitionId =
  | "seismic-section"
  | "seismic-gather"
  | "well-correlation-panel"
  | "survey-map"
  | "rock-physics-crossplot"
  | "volume-interpretation"
  | "avo-response-plot"
  | "avo-intercept-gradient-crossplot"
  | "avo-chi-projection-histogram";

export type ChartRendererKernelId =
  | "raster-trace"
  | "well-panel"
  | "survey-map"
  | "point-cloud"
  | "volume-interpretation"
  | "cartesian-line"
  | "histogram";

export type ChartPublicSurfaceId =
  | "SeismicSectionChart"
  | "SeismicGatherChart"
  | "WellCorrelationPanelChart"
  | "SurveyMapChart"
  | "RockPhysicsCrossplotChart"
  | "VolumeInterpretationChart"
  | "AvoResponseChart"
  | "AvoInterceptGradientCrossplotChart"
  | "AvoChiProjectionHistogramChart";

export type ChartCanonicalBoundaryId =
  | "ophiolite-section-view"
  | "ophiolite-gather-view"
  | "ophiolite-well-panel-source"
  | "ophiolite-survey-map-source"
  | "ophiolite-rock-physics-crossplot-source"
  | "ophiolite-volume-interpretation-source"
  | "ophiolite-avo-response-source"
  | "ophiolite-avo-crossplot-source"
  | "ophiolite-avo-chi-projection-source";

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
  | "rock-physics-continuous-color-binding"
  | "seismic-volume"
  | "volume-slice-plane"
  | "horizon-surface"
  | "well-trajectory-3d"
  | "well-marker-3d"
  | "avo-response-series"
  | "avo-interface-model"
  | "avo-crossplot-point"
  | "avo-crossplot-reference-line"
  | "avo-crossplot-background-region"
  | "avo-chi-projection-series";

export type ChartInteractionToolId =
  | "pointer"
  | "crosshair"
  | "pan"
  | "orbit"
  | "slice-drag"
  | "crop"
  | "select"
  | "interpret-seed";
export type ChartInteractionActionId = "fitToData" | "topView" | "sideView" | "centerSelection";

import type {
  ChartRendererBackendContract,
  ChartRendererConsumerGuarantee,
  ChartSupportTier
} from "./renderer-capabilities";

export interface ChartInteractionProfile {
  tools: readonly ChartInteractionToolId[];
  actions: readonly ChartInteractionActionId[];
}

export interface ChartDefinition {
  id: ChartDefinitionId;
  familyId: ChartFamilyId;
  label: string;
  summary: string;
  supportTier: ChartSupportTier;
  publicSurface: ChartPublicSurfaceId;
  rendererKernel: ChartRendererKernelId;
  rendererBackends: readonly ChartRendererBackendContract[];
  canonicalBoundaries: readonly ChartCanonicalBoundaryId[];
  allowedAssetFamilies: readonly ChartAssetFamilyId[];
  interactionProfile: ChartInteractionProfile;
  adapterEntryPoints: readonly string[];
  validationEntryPoints: readonly string[];
  consumerGuarantees: readonly ChartRendererConsumerGuarantee[];
  constraints: readonly string[];
}

export interface ChartFamilyDefinition {
  id: ChartFamilyId;
  label: string;
  summary: string;
  supportTier: ChartSupportTier;
  chartIds: readonly ChartDefinitionId[];
  rendererKernels: readonly ChartRendererKernelId[];
  canonicalBoundaries: readonly ChartCanonicalBoundaryId[];
}

const PUBLIC_GUARANTEES = [
  {
    id: "public-package-entrypoint",
    summary: "Embedders should consume this chart family through published package entrypoints."
  },
  {
    id: "traceboost-demo-consumer",
    summary: "TraceBoost demo must remain a valid first-party external-style consumer."
  }
] as const satisfies readonly ChartRendererConsumerGuarantee[];

const NEUTRAL_MODEL_GUARANTEE = {
  id: "neutral-data-model",
  summary: "The chart accepts a neutral chart-native data model at the public SDK boundary."
} as const satisfies ChartRendererConsumerGuarantee;

const OPHIOLITE_ADAPTER_GUARANTEE = {
  id: "ophiolite-adapter-subpath",
  summary: "An Ophiolite adapter path exists for canonical platform contracts."
} as const satisfies ChartRendererConsumerGuarantee;

const DOCS_GUARANTEE = {
  id: "public-docs-coverage",
  summary: "The chart family should be represented in public docs or examples."
} as const satisfies ChartRendererConsumerGuarantee;

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

const AVO_INTERACTION_PROFILE = {
  tools: ["pointer", "crosshair", "pan"],
  actions: ["fitToData"]
} as const satisfies ChartInteractionProfile;

const VOLUME_INTERPRETATION_INTERACTION_PROFILE = {
  tools: ["pointer", "orbit", "pan", "interpret-seed"],
  actions: ["fitToData", "topView", "sideView", "centerSelection"]
} as const satisfies ChartInteractionProfile;

export const CHART_DEFINITIONS = [
  {
    id: "seismic-section",
    familyId: "seismic",
    label: "Seismic Section",
    summary: "2D seismic section renderer with section overlays, well overlays, and shared probe interactions.",
    supportTier: "public-launch",
    publicSurface: "SeismicSectionChart",
    rendererKernel: "raster-trace",
    rendererBackends: [{ id: "canvas-2d", default: true }],
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
    consumerGuarantees: [...PUBLIC_GUARANTEES, NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE, DOCS_GUARANTEE],
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
    supportTier: "public-launch",
    publicSurface: "SeismicGatherChart",
    rendererKernel: "raster-trace",
    rendererBackends: [{ id: "canvas-2d", default: true }],
    canonicalBoundaries: ["ophiolite-gather-view"],
    allowedAssetFamilies: ["seismic-gather-amplitudes"],
    interactionProfile: SEISMIC_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteGatherViewToPayload"],
    validationEntryPoints: ["validateGatherPayload"],
    consumerGuarantees: [...PUBLIC_GUARANTEES, NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE, DOCS_GUARANTEE],
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
    supportTier: "public-launch",
    publicSurface: "WellCorrelationPanelChart",
    rendererKernel: "well-panel",
    rendererBackends: [{ id: "canvas-2d", default: true }],
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
    consumerGuarantees: [...PUBLIC_GUARANTEES, NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE, DOCS_GUARANTEE],
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
    supportTier: "public-launch",
    publicSurface: "SurveyMapChart",
    rendererKernel: "survey-map",
    rendererBackends: [{ id: "canvas-2d", default: true }],
    canonicalBoundaries: ["ophiolite-survey-map-source"],
    allowedAssetFamilies: ["survey-outline", "survey-well-location", "survey-plan-trajectory", "survey-scalar-field"],
    interactionProfile: SURVEY_MAP_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteSurveyMapToChart"],
    validationEntryPoints: ["validateOphioliteSurveyMapSource"],
    consumerGuarantees: [...PUBLIC_GUARANTEES, NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE, DOCS_GUARANTEE],
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
    supportTier: "public-launch",
    publicSurface: "RockPhysicsCrossplotChart",
    rendererKernel: "point-cloud",
    rendererBackends: [{ id: "canvas-2d", default: true }],
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
    consumerGuarantees: [...PUBLIC_GUARANTEES, NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE, DOCS_GUARANTEE],
    constraints: [
      "Accepts only resolved Ophiolite rock-physics crossplot sources materialized from well-log samples.",
      "Axis semantics are template-scoped, so a Vp/Vs vs AI template cannot bind arbitrary log types such as resistivity on the x or y axis.",
      "Color bindings are restricted to the template-approved continuous and categorical semantics.",
      "Rejects seismic contracts, survey-map sources, pressure tables, and generic non-log sample payloads."
    ]
  },
  {
    id: "volume-interpretation",
    familyId: "volume-interpretation",
    label: "Volume Interpretation",
    summary: "3D interpretation workspace for seismic slice planes, horizon surfaces, well trajectories, and interpretation seeds.",
    supportTier: "preview",
    publicSurface: "VolumeInterpretationChart",
    rendererKernel: "volume-interpretation",
    rendererBackends: [{ id: "vtkjs", default: true }, { id: "webgl" }],
    canonicalBoundaries: ["ophiolite-volume-interpretation-source"],
    allowedAssetFamilies: [
      "seismic-volume",
      "volume-slice-plane",
      "horizon-surface",
      "well-trajectory-3d",
      "well-marker-3d"
    ],
    interactionProfile: VOLUME_INTERPRETATION_INTERACTION_PROFILE,
    adapterEntryPoints: [],
    validationEntryPoints: [],
    consumerGuarantees: [...PUBLIC_GUARANTEES, OPHIOLITE_ADAPTER_GUARANTEE],
    constraints: [
      "Accepts only resolved volume-interpretation scene sources rather than raw canonical assets.",
      "Treats orthogonal slice planes as the precision interaction surface, with optional volumetric context remaining secondary.",
      "Keeps runtime styling, selection, and interpretation requests chart-native while backend layers own interpretation math and display-space resolution."
    ]
  },
  {
    id: "avo-response-plot",
    familyId: "avo",
    label: "AVO Response Plot",
    summary: "Angle-versus-response line chart for modeled isotropic or anisotropic interface reflectivity.",
    supportTier: "preview",
    publicSurface: "AvoResponseChart",
    rendererKernel: "cartesian-line",
    rendererBackends: [{ id: "canvas-2d", default: true }],
    canonicalBoundaries: ["ophiolite-avo-response-source"],
    allowedAssetFamilies: ["avo-response-series", "avo-interface-model"],
    interactionProfile: AVO_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteAvoResponseToChart"],
    validationEntryPoints: ["validateOphioliteAvoResponseSource"],
    consumerGuarantees: [NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE],
    constraints: [
      "Accepts only analysis DTOs for modeled AVO interface responses.",
      "Series stay tied to explicit interface ids plus reflectivity-model and anisotropy semantics.",
      "Rejects generic line payloads, rock-physics sample clouds, and persisted seismic section contracts."
    ]
  },
  {
    id: "avo-intercept-gradient-crossplot",
    familyId: "avo",
    label: "AVO Intercept-Gradient Crossplot",
    summary: "Point-cloud crossplot for intercept-gradient analysis with optional chi projection, background regions, and trend lines.",
    supportTier: "preview",
    publicSurface: "AvoInterceptGradientCrossplotChart",
    rendererKernel: "point-cloud",
    rendererBackends: [{ id: "canvas-2d", default: true }],
    canonicalBoundaries: ["ophiolite-avo-crossplot-source"],
    allowedAssetFamilies: [
      "avo-interface-model",
      "avo-crossplot-point",
      "avo-crossplot-reference-line",
      "avo-crossplot-background-region"
    ],
    interactionProfile: AVO_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteAvoCrossplotToChart"],
    validationEntryPoints: ["validateOphioliteAvoCrossplotSource"],
    consumerGuarantees: [NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE],
    constraints: [
      "Accepts only analysis DTOs for AVO intercept-gradient point clouds.",
      "Points stay bound to explicit interface ids and optional chi projections or Monte Carlo simulation ids.",
      "Rejects unconstrained generic scatter data and non-AVO sample semantics."
    ]
  },
  {
    id: "avo-chi-projection-histogram",
    familyId: "avo",
    label: "AVO Chi Projection Histogram",
    summary: "Histogram-oriented analysis chart for chi-angle projections used to compare interface separability.",
    supportTier: "preview",
    publicSurface: "AvoChiProjectionHistogramChart",
    rendererKernel: "histogram",
    rendererBackends: [{ id: "canvas-2d", default: true }],
    canonicalBoundaries: ["ophiolite-avo-chi-projection-source"],
    allowedAssetFamilies: ["avo-interface-model", "avo-chi-projection-series"],
    interactionProfile: AVO_INTERACTION_PROFILE,
    adapterEntryPoints: ["adaptOphioliteAvoChiProjectionToChart"],
    validationEntryPoints: ["validateOphioliteAvoChiProjectionSource"],
    consumerGuarantees: [NEUTRAL_MODEL_GUARANTEE, OPHIOLITE_ADAPTER_GUARANTEE],
    constraints: [
      "Accepts only analysis DTOs for chi-angle projections or weighted-stack feasibility studies.",
      "Series stay tied to explicit interface ids and preserve raw projected samples for histogramming.",
      "Rejects generic bar charts, seismic volumes, and unconstrained categorical counts."
    ]
  }
] as const satisfies readonly ChartDefinition[];

export const CHART_FAMILIES = [
  {
    id: "seismic",
    label: "Seismic",
    summary: "Raster-trace charts for section and prestack seismic views.",
    supportTier: "public-launch",
    chartIds: ["seismic-section", "seismic-gather"],
    rendererKernels: ["raster-trace"],
    canonicalBoundaries: ["ophiolite-section-view", "ophiolite-gather-view"]
  },
  {
    id: "well-panel",
    label: "Well Panel",
    summary: "Track-based well interpretation charts with explicit layer and asset constraints.",
    supportTier: "public-launch",
    chartIds: ["well-correlation-panel"],
    rendererKernels: ["well-panel"],
    canonicalBoundaries: ["ophiolite-well-panel-source"]
  },
  {
    id: "survey-map",
    label: "Survey Map",
    summary: "Map-space charts for outlines, well locations, and scalar grids.",
    supportTier: "public-launch",
    chartIds: ["survey-map"],
    rendererKernels: ["survey-map"],
    canonicalBoundaries: ["ophiolite-survey-map-source"]
  },
  {
    id: "rock-physics",
    label: "Rock Physics",
    summary: "Point-cloud analysis charts for template-driven crossplots derived from log samples.",
    supportTier: "public-launch",
    chartIds: ["rock-physics-crossplot"],
    rendererKernels: ["point-cloud"],
    canonicalBoundaries: ["ophiolite-rock-physics-crossplot-source"]
  },
  {
    id: "volume-interpretation",
    label: "Volume Interpretation",
    summary: "3D interpretation charts for slice planes, horizons, trajectories, and interpretation seeds.",
    supportTier: "preview",
    chartIds: ["volume-interpretation"],
    rendererKernels: ["volume-interpretation"],
    canonicalBoundaries: ["ophiolite-volume-interpretation-source"]
  },
  {
    id: "avo",
    label: "AVO",
    summary: "Analysis charts for modeled AVO responses, intercept-gradient crossplots, and chi-angle projections.",
    supportTier: "preview",
    chartIds: ["avo-response-plot", "avo-intercept-gradient-crossplot", "avo-chi-projection-histogram"],
    rendererKernels: ["cartesian-line", "point-cloud", "histogram"],
    canonicalBoundaries: [
      "ophiolite-avo-response-source",
      "ophiolite-avo-crossplot-source",
      "ophiolite-avo-chi-projection-source"
    ]
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

export function listChartDefinitionsBySupportTier(supportTier: ChartSupportTier): ChartDefinition[] {
  return CHART_DEFINITIONS.filter((definition) => definition.supportTier === supportTier);
}
