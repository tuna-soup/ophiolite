import { manifestCatalog, type SupportTier } from "./generated/manifest-catalog";

export const navigation = [
  { href: "#start-here", label: "Start Here" },
  { href: "#support-tiers", label: "Support Tiers" },
  { href: "#recipes", label: "Recipes" },
  { href: "#examples", label: "Examples" },
  { href: "#traceboost", label: "TraceBoost" },
  { href: "#manifests", label: "Manifests" }
] as const;

export const launchFamilies = [
  {
    id: "seismic-section",
    title: "Seismic Section",
    summary: "2D seismic sections with viewport control, overlays, and interpretation-grade interaction."
  },
  {
    id: "seismic-gather",
    title: "Prestack Gather",
    summary: "Gather-native seismic displays without forcing consumers through a generic series DSL."
  },
  {
    id: "survey-map",
    title: "Survey Map",
    summary: "Plan-view charts for survey outlines, scalar grids, well locations, and trajectories."
  },
  {
    id: "rock-physics",
    title: "Rock Physics Crossplot",
    summary: "Template-scoped crossplots for petrophysical and rock-physics interpretation."
  },
  {
    id: "well-panel",
    title: "Well Correlation Panel",
    summary: "Track-oriented well panels for multi-well comparison and correlation workflows."
  }
] as const;

export const startHereSteps = [
  {
    title: "Start with @ophiolite/charts",
    summary: "Use the wrapper package and public model types as the default embedding path."
  },
  {
    title: "Keep Ophiolite adapters explicit",
    summary: "Add @ophiolite/charts/adapters/ophiolite only when the app input already begins as Ophiolite DTOs."
  },
  {
    title: "Treat preview as opt-in",
    summary: "Preview charts and extras stay behind explicit subpaths rather than widening the default product promise."
  },
  {
    title: "Use TraceBoost as a reference consumer",
    summary: "The first-party app should consume public packages honestly instead of reaching into lower-level internals."
  }
] as const;

const supportTierCopy = {
  "public-launch": {
    summary: "Default documented surfaces for embedders.",
    examples: "@ophiolite/charts, @ophiolite/charts-toolbar"
  },
  "public-adapter": {
    summary: "Explicit Ophiolite integration helpers documented after the launch path.",
    examples: "@ophiolite/charts/adapters/ophiolite"
  },
  preview: {
    summary: "Opt-in surfaces with narrower guarantees.",
    examples: "@ophiolite/charts/preview, @ophiolite/charts/extras"
  },
  internal: {
    summary: "Implementation packages behind the public SDK boundary.",
    examples: "@ophiolite/charts-data-models, @ophiolite/charts-core, @ophiolite/charts-renderer, @ophiolite/charts-domain"
  }
} satisfies Record<SupportTier, { summary: string; examples: string }>;

export const supportTiers = manifestCatalog.supportTiers.map((entry) => ({
  ...entry,
  summary: supportTierCopy[entry.tier].summary,
  examples: supportTierCopy[entry.tier].examples
}));

export const surfaceCatalog = manifestCatalog.surfaceCatalog;
export const packageManifestCatalog = manifestCatalog.packageCatalog;

export const docsCatalog = [
  {
    group: "Start Here",
    title: "Default public entry path",
    path: "docs/start-here.md",
    summary: "Learn the default wrapper path before adapters, preview, or internal packages."
  },
  {
    group: "Reference",
    title: "Support tiers and compatibility intent",
    path: "docs/support-tiers.md",
    summary: "Defines public-launch, public-adapter, preview, and internal."
  },
  {
    group: "Reference",
    title: "Generated manifest catalog",
    path: "manifests/generated/module-catalog.json",
    summary: "Normalized catalog emitted by the manifest validator for docs and future tooling."
  },
  {
    group: "Recipes",
    title: "Launch embedding patterns",
    path: "docs/recipes/embedding-launch-charts.md",
    summary: "Covers the default root-package embedding path."
  },
  {
    group: "Recipes",
    title: "Ophiolite adapter boundary",
    path: "docs/recipes/ophiolite-adapters.md",
    summary: "Shows where Ophiolite-specific DTO adaptation should live."
  },
  {
    group: "Recipes",
    title: "TraceBoost as reference consumer",
    path: "docs/recipes/traceboost-reference-consumer.md",
    summary: "Explains the ownership split between charts and the first-party app shell."
  },
  {
    group: "Examples",
    title: "Per-family simple and production examples",
    path: "docs/examples/*.md",
    summary: "One simple and one production example for each launch family."
  }
] as const;

export const exampleCatalog = [
  {
    family: "Seismic Section",
    simple: "docs/examples/seismic-section-simple.md",
    production: "docs/examples/seismic-section-production.md"
  },
  {
    family: "Prestack Gather",
    simple: "docs/examples/seismic-gather-simple.md",
    production: "docs/examples/seismic-gather-production.md"
  },
  {
    family: "Survey Map",
    simple: "docs/examples/survey-map-simple.md",
    production: "docs/examples/survey-map-production.md"
  },
  {
    family: "Rock Physics Crossplot",
    simple: "docs/examples/rock-physics-crossplot-simple.md",
    production: "docs/examples/rock-physics-crossplot-production.md"
  },
  {
    family: "Well Correlation Panel",
    simple: "docs/examples/well-correlation-panel-simple.md",
    production: "docs/examples/well-correlation-panel-production.md"
  }
] as const;

export const sectionExample = `import { SeismicSectionChart, type SeismicSectionData } from "@ophiolite/charts";

const section: SeismicSectionData = {
  axis: "inline",
  coordinate: { index: 111, value: 111 },
  horizontalAxis: Float64Array.from([875, 876, 877, 878]),
  sampleAxis: Float32Array.from([0, 4, 8, 12]),
  amplitudes: Float32Array.from([
    0.2, -0.1, 0.4, 0.3,
    0.1, 0.0, -0.2, 0.5,
    -0.3, 0.2, 0.1, -0.1,
    0.0, 0.4, 0.2, -0.2
  ]),
  dimensions: { traces: 4, samples: 4 }
};`;

export const adapterExample = `import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";

const section = adaptOphioliteSectionViewToSeismicSectionData(sectionView);`;

export const manifestSnippet = `{
  "id": "@ophiolite/charts",
  "supportTier": "public-launch",
  "entrypoints": [
    { "subpath": ".", "tier": "public-launch" },
    { "subpath": "./adapters/ophiolite", "tier": "public-adapter" },
    { "subpath": "./preview", "tier": "preview" }
  ]
}`;

export const traceboostFlow = `TraceBoost app shell
  -> @ophiolite/charts
  -> @ophiolite/charts/adapters/ophiolite when the input begins as Ophiolite DTOs
  -> @ophiolite/charts/preview only by explicit opt-in

Not the default path:
TraceBoost app shell
  -> charts-core / renderer / domain internals`;
