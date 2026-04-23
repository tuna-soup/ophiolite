<svelte:options runes={true} />

<script lang="ts">
  import {
    RockPhysicsCrossplotChart,
    SeismicGatherChart,
    SeismicSectionChart,
    SurveyMapChart,
    WellCorrelationPanelChart,
    type RockPhysicsCrossplotData,
    type SeismicGatherData,
    type SeismicSectionData,
    type SurveyMapData,
    type WellCorrelationPanelData
  } from "@ophiolite/charts";

  const launchFamilies = [
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

  const exampleCatalog = [
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

  const previewFamilies = [
    "Volume Interpretation",
    "AVO Response",
    "AVO Intercept-Gradient Crossplot",
    "AVO Chi Projection Histogram"
  ] as const;

  const section: SeismicSectionData = {
    axis: "inline",
    coordinate: {
      index: 111,
      value: 111
    },
    horizontalAxis: Float64Array.from([875, 876, 877, 878, 879, 880]),
    sampleAxis: Float32Array.from([0, 4, 8, 12, 16, 20]),
    amplitudes: Float32Array.from([
      0.18, -0.05, 0.34, 0.26, -0.16, 0.09,
      0.1, 0.02, -0.14, 0.42, 0.16, -0.22,
      -0.22, 0.18, 0.12, -0.08, 0.28, 0.05,
      0.03, 0.32, 0.2, -0.14, 0.08, -0.18,
      -0.15, 0.04, 0.24, 0.31, -0.1, 0.14,
      0.06, -0.11, 0.19, 0.27, 0.12, -0.06
    ]),
    dimensions: {
      traces: 6,
      samples: 6
    },
    units: {
      horizontal: "xline",
      sample: "ms",
      amplitude: "arb"
    },
    presentation: {
      title: "Inline 111",
      sampleAxisLabel: "Time"
    }
  };

  const gather: SeismicGatherData = {
    label: "Gather 042",
    gatherAxisKind: "offset",
    sampleDomain: "time",
    horizontalAxis: Float64Array.from([250, 500, 750, 1000, 1250, 1500]),
    sampleAxis: Float32Array.from([0, 4, 8, 12, 16, 20]),
    amplitudes: Float32Array.from([
      0.08, 0.16, 0.24, 0.18, 0.1, 0.02,
      -0.04, 0.12, 0.3, 0.44, 0.36, 0.18,
      -0.14, 0.05, 0.22, 0.38, 0.28, 0.12,
      -0.18, -0.04, 0.16, 0.31, 0.22, 0.08,
      -0.22, -0.08, 0.1, 0.24, 0.2, 0.06,
      -0.16, -0.02, 0.08, 0.17, 0.12, 0.02
    ]),
    dimensions: {
      traces: 6,
      samples: 6
    },
    units: {
      horizontal: "m",
      sample: "ms",
      amplitude: "arb"
    },
    displayDefaults: {
      gain: 1.4
    }
  };

  const surveyMap: SurveyMapData = {
    name: "North Survey",
    xLabel: "Easting",
    yLabel: "Northing",
    coordinateUnit: "m",
    background: "#f4f2ee",
    areas: [
      {
        name: "North Survey",
        points: [
          { x: 120, y: 160 },
          { x: 2060, y: 180 },
          { x: 2120, y: 1540 },
          { x: 180, y: 1620 }
        ],
        stroke: "rgba(39, 79, 68, 0.9)",
        fill: "rgba(39, 79, 68, 0.08)"
      }
    ],
    wells: [
      {
        name: "Well A",
        position: { x: 420, y: 480 },
        trajectory: [
          { x: 420, y: 480 },
          { x: 520, y: 620 },
          { x: 610, y: 760 }
        ],
        color: "#0e7490"
      },
      {
        name: "Well B",
        position: { x: 1240, y: 760 },
        trajectory: [
          { x: 1240, y: 760 },
          { x: 1320, y: 860 },
          { x: 1400, y: 980 }
        ],
        color: "#9a3412"
      },
      {
        name: "Well C",
        position: { x: 1680, y: 1180 },
        trajectory: [
          { x: 1680, y: 1180 },
          { x: 1750, y: 1280 },
          { x: 1820, y: 1390 }
        ],
        color: "#4d7c0f"
      }
    ]
  };

  const rockPhysics: RockPhysicsCrossplotData = {
    templateId: "vp-vs-vs-ai",
    title: "Vp/Vs vs AI",
    subtitle: "Small public model example",
    groups: [
      { name: "Well A", color: "#0f766e" },
      { name: "Well B", color: "#b45309", symbol: "diamond" }
    ],
    points: [
      { x: 5850, y: 1.62, group: "Well A", depthM: 2410 },
      { x: 6120, y: 1.68, group: "Well A", depthM: 2422 },
      { x: 6490, y: 1.74, group: "Well A", depthM: 2436 },
      { x: 6820, y: 1.81, group: "Well B", depthM: 2448 },
      { x: 7180, y: 1.89, group: "Well B", depthM: 2462 },
      { x: 7560, y: 1.96, group: "Well B", depthM: 2474 }
    ]
  };

  const wellPanel: WellCorrelationPanelData = {
    name: "Well Correlation",
    depthDomain: {
      start: 1500,
      end: 1620,
      unit: "m",
      label: "MD"
    },
    background: "#faf7f2",
    wells: [
      {
        name: "Well A",
        depthDatum: "md",
        curves: [
          {
            name: "GR",
            color: "#1f2937",
            values: Float32Array.from([72, 86, 102, 118, 94, 88, 76]),
            depths: Float32Array.from([1500, 1520, 1540, 1560, 1580, 1600, 1620]),
            unit: "API",
            axis: {
              min: 0,
              max: 180,
              label: "GR",
              unit: "API"
            }
          }
        ],
        tops: [
          {
            name: "Reservoir Top",
            depth: 1540,
            color: "#b45309",
            source: "picked"
          }
        ]
      },
      {
        name: "Well B",
        depthDatum: "md",
        curves: [
          {
            name: "GR",
            color: "#334155",
            values: Float32Array.from([64, 78, 92, 108, 116, 104, 90]),
            depths: Float32Array.from([1500, 1520, 1540, 1560, 1580, 1600, 1620]),
            unit: "API",
            axis: {
              min: 0,
              max: 180,
              label: "GR",
              unit: "API"
            }
          }
        ],
        tops: [
          {
            name: "Reservoir Top",
            depth: 1546,
            color: "#b45309",
            source: "picked"
          }
        ]
      }
    ]
  };

  const sectionExample = `import { SeismicSectionChart, type SeismicSectionData } from "@ophiolite/charts";

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

  const adapterExample = `import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";

const section = adaptOphioliteSectionViewToSeismicSectionData(sectionView);`;
</script>

<svelte:head>
  <title>Ophiolite Charts Docs</title>
  <meta
    name="description"
    content="Public docs surface for Ophiolite Charts, a Svelte-first SDK for specialized subsurface chart components."
  />
</svelte:head>

<div class="docs-shell">
  <header class="hero" data-testid="hero">
    <div class="hero-copy">
      <p class="eyebrow">Ophiolite Charts</p>
      <h1>Svelte-first charts for subsurface applications.</h1>
      <p class="summary">
        Specialized chart components for seismic, survey-map, rock-physics, and well-panel
        workflows. Public models stay domain-specific and readable. Ophiolite transport adapters
        remain explicit.
      </p>
      <div class="hero-actions">
        <a href="#getting-started">Getting Started</a>
        <a href="#examples">Examples</a>
        <a href="#families">Chart Families</a>
      </div>
    </div>
    <div class="hero-note">
      <p>Public boundary</p>
      <ul>
        <li>`@ophiolite/charts` for launch surfaces</li>
        <li>`@ophiolite/charts/adapters/ophiolite` for DTO adaptation</li>
        <li>No Ophiolite transport fields in the default API story</li>
      </ul>
    </div>
  </header>

  <main class="content">
    <section id="getting-started" class="panel" data-testid="getting-started">
      <div class="section-heading">
        <p class="eyebrow">Getting Started</p>
        <h2>One package for the default path</h2>
      </div>
      <div class="copy-grid">
        <div>
          <p>
            Start with `@ophiolite/charts` and the neutral public model types. The launch API is
            chart-family specific on purpose. It should feel like a subsurface SDK, not like a
            generic chart config DSL.
          </p>
          <pre><code>{sectionExample}</code></pre>
        </div>
        <div>
          <p>
            If your source data begins as an Ophiolite DTO, keep that concern explicit. The adapter
            should emit the same neutral public model that a non-Ophiolite consumer could author by
            hand.
          </p>
          <pre><code>{adapterExample}</code></pre>
        </div>
      </div>
    </section>

    <section id="examples" class="panel" data-testid="examples">
      <div class="section-heading">
        <p class="eyebrow">Canonical Examples</p>
        <h2>Every launch family ships with simple and production docs</h2>
      </div>
      <div class="example-matrix">
        {#each exampleCatalog as item (item.family)}
          <article class="example-card">
            <h3>{item.family}</h3>
            <p><code>{item.simple}</code></p>
            <p><code>{item.production}</code></p>
          </article>
        {/each}
      </div>
    </section>

    <section class="panel" data-testid="seismic-showcase">
      <div class="section-heading">
        <p class="eyebrow">Launch Surface</p>
        <h2>Live examples using neutral public data models</h2>
      </div>
      <div class="showcase-grid">
        <div class="showcase-card">
          <h3>Seismic Section</h3>
          <p>Readable field names, typed arrays where density matters, one wrapper import.</p>
          <SeismicSectionChart chartId="docs-example-section" viewId="inline:111" {section} />
        </div>
        <div class="showcase-card">
          <h3>Prestack Gather</h3>
          <p>Gather-native payloads stay simple without exposing encoded contract DTOs.</p>
          <SeismicGatherChart chartId="docs-example-gather" viewId="gather:042" {gather} />
        </div>
      </div>
    </section>

    <section id="families" class="panel" data-testid="families">
      <div class="section-heading">
        <p class="eyebrow">Launch Families</p>
        <h2>Focused chart families, not a generic chart kitchen sink</h2>
      </div>
      <div class="family-grid">
        {#each launchFamilies as family (family.id)}
          <article class="family-card">
            <h3>{family.title}</h3>
            <p>{family.summary}</p>
          </article>
        {/each}
      </div>
    </section>

    <section class="panel" data-testid="family-previews">
      <div class="section-heading">
        <p class="eyebrow">Family Previews</p>
        <h2>Launch families beyond seismic</h2>
      </div>
      <div class="launch-preview-grid">
        <div class="mini-card">
          <h3>Survey Map</h3>
          <SurveyMapChart chartId="docs-survey-map" map={surveyMap} />
        </div>
        <div class="mini-card">
          <h3>Rock Physics</h3>
          <RockPhysicsCrossplotChart chartId="docs-rock-physics" model={rockPhysics} />
        </div>
        <div class="mini-card">
          <h3>Well Correlation</h3>
          <WellCorrelationPanelChart chartId="docs-well-panel" panel={wellPanel} />
        </div>
      </div>
    </section>

    <section class="panel" data-testid="preview-surface">
      <div class="section-heading">
        <p class="eyebrow">Preview</p>
        <h2>Preview families move behind explicit subpaths</h2>
      </div>
      <p class="preview-copy">
        Preview charts such as volume interpretation and AVO stay available, but they should not
        widen the default product promise. Consume them from `@ophiolite/charts/preview`.
      </p>
      <ul class="preview-list">
        {#each previewFamilies as family (family)}
          <li>{family}</li>
        {/each}
      </ul>
    </section>
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    background:
      radial-gradient(circle at top left, rgba(204, 230, 221, 0.7), transparent 35%),
      linear-gradient(180deg, #f6f4ee 0%, #efe8d8 100%);
    color: #16332d;
    font-family: "Aptos", "Segoe UI", sans-serif;
  }

  :global(code),
  :global(pre) {
    font-family: "IBM Plex Mono", "Consolas", monospace;
  }

  .docs-shell {
    min-height: 100vh;
    padding: 32px;
  }

  .hero {
    display: grid;
    gap: 24px;
    grid-template-columns: minmax(0, 1.8fr) minmax(280px, 0.9fr);
    align-items: start;
    margin: 0 auto 28px;
    max-width: 1380px;
  }

  .hero-copy,
  .hero-note,
  .panel {
    border: 1px solid rgba(22, 51, 45, 0.12);
    border-radius: 28px;
    background: rgba(255, 252, 246, 0.88);
    box-shadow: 0 20px 48px rgba(53, 73, 54, 0.08);
  }

  .hero-copy {
    padding: 38px 40px;
  }

  .hero-note {
    padding: 28px 30px;
  }

  .eyebrow {
    margin: 0 0 12px;
    color: #8f4c17;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  h1,
  h2,
  h3,
  p {
    margin: 0;
  }

  h1 {
    max-width: 12ch;
    font-size: clamp(2.7rem, 6vw, 5.1rem);
    line-height: 0.95;
    letter-spacing: -0.05em;
  }

  .summary {
    max-width: 58ch;
    margin-top: 18px;
    color: rgba(22, 51, 45, 0.86);
    font-size: 1.05rem;
    line-height: 1.6;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 24px;
  }

  .hero-actions a {
    padding: 12px 18px;
    border-radius: 999px;
    background: #16332d;
    color: #f7f3eb;
    text-decoration: none;
    font-weight: 600;
  }

  .hero-note p {
    font-size: 0.95rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }

  .hero-note ul {
    margin: 16px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 10px;
    line-height: 1.5;
  }

  .content {
    display: grid;
    gap: 24px;
    max-width: 1380px;
    margin: 0 auto;
  }

  .panel {
    padding: 28px 30px;
  }

  .section-heading {
    margin-bottom: 18px;
  }

  .section-heading h2 {
    font-size: clamp(1.7rem, 3vw, 2.4rem);
    line-height: 1.05;
  }

  .copy-grid,
  .showcase-grid,
  .launch-preview-grid,
  .example-matrix {
    display: grid;
    gap: 18px;
  }

  .copy-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .copy-grid p,
  .preview-copy,
  .family-card p,
  .showcase-card p,
  .example-card p {
    color: rgba(22, 51, 45, 0.82);
    line-height: 1.6;
  }

  pre {
    overflow: auto;
    margin-top: 16px;
    padding: 18px;
    border-radius: 22px;
    background: #1b2926;
    color: #edf4ea;
    font-size: 0.88rem;
    line-height: 1.55;
  }

  .example-matrix,
  .family-grid,
  .launch-preview-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .showcase-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .showcase-card,
  .mini-card,
  .example-card {
    padding: 16px;
    border-radius: 24px;
    background: rgba(240, 235, 225, 0.72);
  }

  .family-card {
    padding: 20px;
    border-radius: 24px;
    background:
      linear-gradient(160deg, rgba(255, 249, 240, 0.95), rgba(240, 235, 225, 0.9)),
      linear-gradient(120deg, rgba(182, 141, 76, 0.12), rgba(13, 88, 74, 0.08));
    border: 1px solid rgba(22, 51, 45, 0.08);
  }

  .family-card h3,
  .mini-card h3,
  .showcase-card h3,
  .example-card h3 {
    margin-bottom: 10px;
    font-size: 1.05rem;
  }

  .preview-copy {
    max-width: 70ch;
  }

  .preview-list {
    margin: 16px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 8px;
  }

  @media (max-width: 960px) {
    .docs-shell {
      padding: 18px;
    }

    .hero,
    .copy-grid,
    .showcase-grid {
      grid-template-columns: 1fr;
    }

    .hero-copy,
    .hero-note,
    .panel {
      padding: 22px;
    }

    h1 {
      max-width: none;
    }
  }
</style>
