<svelte:options runes={true} />

<script lang="ts">
  import {
    RockPhysicsCrossplotChart,
    SeismicGatherChart,
    SeismicSectionChart,
    SurveyMapChart,
    WellCorrelationPanelChart
  } from "@ophiolite/charts";
  import { gather, rockPhysics, section, surveyMap, wellPanel } from "./lib/demo-data";
  import {
    adapterExample,
    docsCatalog,
    exampleCatalog,
    launchFamilies,
    manifestSnippet,
    navigation,
    packageManifestCatalog,
    sectionExample,
    startHereSteps,
    supportTiers,
    surfaceCatalog,
    traceboostFlow
  } from "./lib/site-data";
</script>

<svelte:head>
  <title>Ophiolite Charts Docs</title>
  <meta
    name="description"
    content="Public docs surface for Ophiolite Charts, the embeddable chart SDK inside Ophiolite."
  />
</svelte:head>

<div class="docs-shell">
  <header class="topbar">
    <p class="brand">Ophiolite Charts</p>
    <nav aria-label="Primary">
      {#each navigation as item (item.href)}
        <a href={item.href}>{item.label}</a>
      {/each}
    </nav>
  </header>

  <section class="hero" data-testid="hero">
    <div class="hero-copy">
      <p class="eyebrow">Embeddable Subsurface SDK</p>
      <h1>Teach charts first. Keep adapters explicit. Treat TraceBoost as a reference consumer.</h1>
      <p class="summary">
        Ophiolite Charts is the embeddable visualization SDK inside Ophiolite. The default docs
        path starts with <code>@ophiolite/charts</code>, adds adapters only when the source data
        begins as Ophiolite DTOs, and keeps preview and internal surfaces clearly labeled.
      </p>
      <div class="hero-actions">
        <a href="#start-here">Start Here</a>
        <a href="#recipes">Recipes</a>
        <a href="#manifests">Module Manifests</a>
      </div>
    </div>
    <div class="hero-note">
      <p class="eyebrow">Boundary Snapshot</p>
      <ul>
        <li><code>@ophiolite/charts</code> is the launch surface.</li>
        <li><code>@ophiolite/charts/adapters/ophiolite</code> is explicit integration glue.</li>
        <li><code>@ophiolite/charts/preview</code> stays opt-in.</li>
        <li>Lower-level packages remain implementation layers.</li>
      </ul>
      <div class="callout">
        <p class="callout-label">Validator</p>
        <code>bun scripts/validate-module-manifests.ts</code>
      </div>
    </div>
  </section>

  <main class="content">
    <section id="start-here" class="panel" data-testid="getting-started">
      <div class="section-heading">
        <p class="eyebrow">Start Here</p>
        <h2>The default path is wrapper first, adapters second, TraceBoost third.</h2>
      </div>
      <div class="start-grid">
        <div class="step-grid">
          {#each startHereSteps as step (step.title)}
            <article class="step-card">
              <h3>{step.title}</h3>
              <p>{step.summary}</p>
            </article>
          {/each}
        </div>
        <div class="code-stack">
          <article class="code-card">
            <h3>Root package first</h3>
            <pre><code>{sectionExample}</code></pre>
          </article>
          <article class="code-card">
            <h3>Adapters only when needed</h3>
            <pre><code>{adapterExample}</code></pre>
          </article>
        </div>
      </div>
    </section>

    <section id="support-tiers" class="panel">
      <div class="section-heading">
        <p class="eyebrow">Support Tiers</p>
        <h2>Small vocabulary, explicit boundaries.</h2>
      </div>
      <div class="tier-grid">
        {#each supportTiers as item (item.tier)}
          <article class="tier-card">
            <div class="tier-header">
              <h3>{item.tier}</h3>
              <p class="tier-stats">{item.packageCount} packages, {item.entrypointCount} entrypoints</p>
            </div>
            <p>{item.summary}</p>
            <p class="tier-examples">{item.examples}</p>
            {#if item.packages.length > 0}
              <p class="tier-packages">{item.packages.join(", ")}</p>
            {/if}
          </article>
        {/each}
      </div>
      <div class="surface-table" role="table" aria-label="Documented package surfaces">
        <div class="surface-row surface-head" role="row">
          <span role="columnheader">Surface</span>
          <span role="columnheader">Tier</span>
          <span role="columnheader">Role</span>
        </div>
        {#each surfaceCatalog as surface (surface.surface)}
          <div class="surface-row" role="row">
            <span role="cell"><code>{surface.surface}</code></span>
            <span role="cell"><span class="badge">{surface.tier}</span></span>
            <span role="cell">{surface.role}</span>
          </div>
        {/each}
      </div>
    </section>

    <section id="recipes" class="panel" data-testid="examples">
      <div class="section-heading">
        <p class="eyebrow">Docs Map</p>
        <h2>Start Here, Recipes, then family examples.</h2>
      </div>
      <div class="docs-grid">
        {#each docsCatalog as item (item.path)}
          <article class="docs-card">
            <p class="docs-group">{item.group}</p>
            <h3>{item.title}</h3>
            <p>{item.summary}</p>
            <p class="docs-path"><code>{item.path}</code></p>
          </article>
        {/each}
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
        <h2>Live examples built from neutral public data models.</h2>
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
        <h2>Focused chart families instead of a generic chart kitchen sink.</h2>
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
        <p class="eyebrow">Launch Preview</p>
        <h2>Other launch families stay in the same public-first story.</h2>
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

    <section id="traceboost" class="panel">
      <div class="section-heading">
        <p class="eyebrow">TraceBoost</p>
        <h2>The first-party app is a reference consumer, not the SDK boundary.</h2>
      </div>
      <div class="traceboost-grid">
        <div>
          <p>
            The intended consumption model is: TraceBoost owns workflow and transport, while
            charts owns chart-native rendering, viewport behavior, and wrapper APIs.
          </p>
          <p>
            That is why the docs teach <code>@ophiolite/charts</code> first and only point to
            TraceBoost after the public surfaces are already clear.
          </p>
        </div>
        <pre class="flow-card"><code>{traceboostFlow}</code></pre>
      </div>
    </section>

    <section id="manifests" class="panel">
      <div class="section-heading">
        <p class="eyebrow">Module Manifests</p>
        <h2>Small package manifests make support tiers and public intent visible.</h2>
      </div>
      <p class="manifest-summary">
        The validator emits a normalized catalog at
        <code>manifests/generated/module-catalog.json</code> and a docs-facing TypeScript mirror at
        <code>apps/public-docs/src/lib/generated/manifest-catalog.ts</code>.
      </p>
      <div class="manifest-grid">
        <article class="code-card">
          <h3>Manifest shape</h3>
          <pre><code>{manifestSnippet}</code></pre>
        </article>
        <article class="manifest-card">
          <h3>Package catalog</h3>
          <div class="manifest-list">
            {#each packageManifestCatalog as item (item.packageName)}
              <div class="manifest-item">
                <div>
                  <p class="manifest-name">{item.packageName}</p>
                  <p class="manifest-path"><code>{item.manifestPath}</code></p>
                  <p class="manifest-description">{item.description}</p>
                </div>
                <div class="manifest-meta">
                  <span class="badge">{item.tier}</span>
                  <span class="meta-chip">{item.layer}</span>
                </div>
              </div>
            {/each}
          </div>
        </article>
      </div>
    </section>
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    background:
      radial-gradient(circle at top left, rgba(202, 223, 217, 0.72), transparent 34%),
      linear-gradient(180deg, #f7f4ed 0%, #eee6d6 100%);
    color: #15322d;
    font-family: "Aptos", "Segoe UI", sans-serif;
  }

  :global(code),
  :global(pre) {
    font-family: "IBM Plex Mono", "Consolas", monospace;
  }

  .docs-shell {
    min-height: 100vh;
    padding: 24px;
  }

  .topbar {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 16px 24px;
    align-items: center;
    max-width: 1380px;
    margin: 0 auto 22px;
    padding: 16px 22px;
    border: 1px solid rgba(21, 50, 45, 0.1);
    border-radius: 22px;
    background: rgba(255, 251, 243, 0.75);
    box-shadow: 0 16px 40px rgba(53, 73, 54, 0.06);
    backdrop-filter: blur(10px);
  }

  .topbar nav {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .topbar a,
  .hero-actions a {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 10px 14px;
    border-radius: 999px;
    color: inherit;
    text-decoration: none;
  }

  .topbar a {
    background: rgba(21, 50, 45, 0.06);
  }

  .brand {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .hero {
    display: grid;
    gap: 24px;
    grid-template-columns: minmax(0, 1.8fr) minmax(280px, 0.95fr);
    align-items: start;
    max-width: 1380px;
    margin: 0 auto 28px;
  }

  .hero-copy,
  .hero-note,
  .panel {
    border: 1px solid rgba(21, 50, 45, 0.12);
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
  p,
  ul {
    margin: 0;
  }

  h1 {
    max-width: 12ch;
    font-size: clamp(2.6rem, 5.8vw, 5rem);
    line-height: 0.95;
    letter-spacing: -0.05em;
  }

  h2 {
    font-size: clamp(1.8rem, 3vw, 2.6rem);
    line-height: 1;
    letter-spacing: -0.04em;
  }

  h3 {
    font-size: 1.08rem;
    line-height: 1.2;
  }

  .summary {
    max-width: 60ch;
    margin-top: 18px;
    color: rgba(21, 50, 45, 0.86);
    font-size: 1.04rem;
    line-height: 1.65;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 24px;
  }

  .hero-actions a {
    background: #21483f;
    color: #fff7ea;
    font-weight: 600;
  }

  .hero-note ul {
    padding-left: 18px;
    display: grid;
    gap: 10px;
    color: rgba(21, 50, 45, 0.84);
    line-height: 1.55;
  }

  .callout {
    margin-top: 18px;
    padding: 14px 16px;
    border-radius: 18px;
    background: rgba(143, 76, 23, 0.08);
  }

  .callout-label {
    margin-bottom: 8px;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .content {
    display: grid;
    gap: 24px;
    max-width: 1380px;
    margin: 0 auto;
  }

  .panel {
    padding: 30px;
  }

  .section-heading {
    display: grid;
    gap: 8px;
    margin-bottom: 22px;
  }

  .start-grid,
  .traceboost-grid,
  .manifest-grid {
    display: grid;
    gap: 20px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .step-grid,
  .code-stack,
  .tier-grid,
  .docs-grid,
  .example-matrix,
  .family-grid,
  .launch-preview-grid {
    display: grid;
    gap: 16px;
  }

  .step-grid,
  .docs-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .tier-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin-bottom: 18px;
  }

  .family-grid,
  .launch-preview-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .example-matrix {
    grid-template-columns: repeat(5, minmax(0, 1fr));
    margin-top: 18px;
  }

  .showcase-grid {
    display: grid;
    gap: 18px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .step-card,
  .code-card,
  .tier-card,
  .docs-card,
  .example-card,
  .family-card,
  .showcase-card,
  .mini-card,
  .manifest-card {
    padding: 18px;
    border: 1px solid rgba(21, 50, 45, 0.1);
    border-radius: 22px;
    background: rgba(255, 255, 255, 0.68);
  }

  .step-card,
  .tier-card,
  .docs-card,
  .family-card,
  .mini-card {
    display: grid;
    gap: 10px;
  }

  .docs-group {
    color: #8f4c17;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .docs-path,
  .tier-examples,
  .manifest-path {
    color: rgba(21, 50, 45, 0.72);
  }

  .tier-stats,
  .manifest-summary,
  .manifest-description,
  .tier-packages {
    color: rgba(21, 50, 45, 0.78);
  }

  pre {
    overflow: auto;
    margin-top: 12px;
    padding: 16px;
    border-radius: 18px;
    background: #112520;
    color: #edf4ef;
    font-size: 0.88rem;
    line-height: 1.5;
  }

  .surface-table {
    border: 1px solid rgba(21, 50, 45, 0.1);
    border-radius: 22px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.62);
  }

  .surface-row {
    display: grid;
    gap: 12px;
    grid-template-columns: minmax(0, 1.2fr) 170px minmax(0, 1.5fr);
    align-items: center;
    padding: 14px 18px;
    border-top: 1px solid rgba(21, 50, 45, 0.08);
  }

  .surface-row:first-child {
    border-top: 0;
  }

  .surface-head {
    background: rgba(21, 50, 45, 0.06);
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .badge,
  .meta-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 6px 10px;
    border-radius: 999px;
    font-size: 0.78rem;
    font-weight: 700;
  }

  .badge {
    background: rgba(143, 76, 23, 0.12);
    color: #7d420f;
  }

  .meta-chip {
    background: rgba(21, 50, 45, 0.08);
    color: #21483f;
  }

  .traceboost-grid {
    align-items: start;
  }

  .flow-card {
    margin: 0;
  }

  .manifest-list {
    display: grid;
    gap: 12px;
  }

  .manifest-item {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: start;
    padding-top: 12px;
    border-top: 1px solid rgba(21, 50, 45, 0.08);
  }

  .manifest-item:first-child {
    padding-top: 0;
    border-top: 0;
  }

  .manifest-name {
    font-weight: 700;
  }

  .manifest-description {
    margin-top: 8px;
  }

  .manifest-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
  }

  @media (max-width: 1120px) {
    .hero,
    .start-grid,
    .traceboost-grid,
    .manifest-grid,
    .showcase-grid {
      grid-template-columns: 1fr;
    }

    .tier-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .example-matrix {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 840px) {
    .docs-shell {
      padding: 16px;
    }

    .hero-copy,
    .hero-note,
    .panel {
      border-radius: 22px;
    }

    .hero-copy,
    .hero-note,
    .panel {
      padding: 22px;
    }

    .step-grid,
    .docs-grid,
    .family-grid,
    .launch-preview-grid,
    .tier-grid,
    .example-matrix {
      grid-template-columns: 1fr;
    }

    .surface-row {
      grid-template-columns: 1fr;
      align-items: start;
    }
  }
</style>
