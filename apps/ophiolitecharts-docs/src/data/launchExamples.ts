export interface LaunchExample {
  key: "seismic" | "gather" | "well-panel" | "survey-map" | "rock-physics";
  title: string;
  description: string;
  href: string;
  docsHref: string;
  embedRoute: string;
  imageSrc: string;
  imageAlt: string;
  eyebrow: string;
}

export const launchExamples: LaunchExample[] = [
  {
    key: "seismic",
    title: "Seismic Section",
    description:
      "Shared viewport control, probes, overlays, and heatmap or wiggle presentation for section-first interpretation tooling.",
    href: "/live/?mode=public#/seismic",
    docsHref: "/docs/chart-families/seismic-section/",
    embedRoute: "/live/?mode=embed#/seismic",
    imageSrc: "/images/examples/seismic-section.png",
    imageAlt: "Seismic section example from Ophiolite Charts",
    eyebrow: "Launch Family"
  },
  {
    key: "gather",
    title: "Prestack Gather",
    description:
      "A focused gather surface over the same seismic interaction model, tuned for trace-by-trace exploration.",
    href: "/live/?mode=public#/gather",
    docsHref: "/docs/chart-families/prestack-gather/",
    embedRoute: "/live/?mode=embed#/gather",
    imageSrc: "/images/examples/prestack-gather.png",
    imageAlt: "Prestack gather example from Ophiolite Charts",
    eyebrow: "Launch Family"
  },
  {
    key: "well-panel",
    title: "Well Correlation Panel",
    description:
      "Depth-aligned multi-well layouts with explicit panel semantics instead of generic dashboard abstractions.",
    href: "/live/?mode=public#/well-panel",
    docsHref: "/docs/chart-families/well-correlation-panel/",
    embedRoute: "/live/?mode=embed#/well-panel",
    imageSrc: "/images/examples/well-correlation-panel.png",
    imageAlt: "Well correlation panel example from Ophiolite Charts",
    eyebrow: "Launch Family"
  },
  {
    key: "survey-map",
    title: "Survey Map",
    description:
      "Plan-view context for wells, trajectories, outlines, and geospatial overlays that belong beside scientific data.",
    href: "/live/?mode=public#/survey-map",
    docsHref: "/docs/chart-families/survey-map/",
    embedRoute: "/live/?mode=embed#/survey-map",
    imageSrc: "/images/examples/survey-map.png",
    imageAlt: "Survey map example from Ophiolite Charts",
    eyebrow: "Launch Family"
  },
  {
    key: "rock-physics",
    title: "Rock Physics Crossplot",
    description:
      "Dense point-cloud interaction with typed models and host-owned axis workflows for scientific applications.",
    href: "/live/?mode=public#/rock-physics",
    docsHref: "/docs/chart-families/rock-physics-crossplot/",
    embedRoute: "/live/?mode=embed#/rock-physics",
    imageSrc: "/images/examples/rock-physics-crossplot.png",
    imageAlt: "Rock physics crossplot example from Ophiolite Charts",
    eyebrow: "Launch Family"
  }
];
