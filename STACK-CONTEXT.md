# Subsurface Stack Context

Use this as the crisp engineering context for the platform end state.

## Mental Model

Use this simple model:

- `ophiolite` is the Rust engine and platform boundary
- `Ophiolite Charts` is the reusable display layer
- `TraceBoost` is the first workflow app and reference proof harness assembled from those lower layers
- CLI, Python, and desktop commands are control surfaces over Rust-owned behavior, not separate backend implementations

That is the baseline way to explain how the stack hangs together.

## What The Repos Are

### `ophiolite`

What it is:

- the platform repo
- the canonical owner of subsurface contracts, DTO meaning, runtime primitives, package and project foundations, and automation surfaces that should exist independent of any one app
- the home of `Ophiolite Charts`, the embeddable visualization SDK

What it is not:

- not a single end-user workflow app
- not a dumping ground for app-local workspace state, presets, or product-specific transport quirks

### `TraceBoost`

What it is:

- the first-party seismic workflow application built on top of Ophiolite
- the place where ingestion, viewing, processing, demo packaging, and app-local orchestration are assembled into one opinionated workflow
- the reference implementation that proves the platform can support a commercial-quality desktop product
- the reference proof harness for typed workflow recipes, reproducible run reports, and curated real-data workflow validation

What it is not:

- not the canonical source of reusable subsurface meaning
- not the platform brand
- not the chart SDK
- not the canonical owner of processing identity, import evidence, runtime events, or lineage semantics

### `Ophiolite Charts`

What it is:

- the embeddable chart SDK inside the Ophiolite umbrella
- the owner of chart-native rendering, viewport behavior, anchors, and wrapper APIs
- the layer that consumes canonical contracts or normalized chart payloads and turns them into reusable interactive views

What it is not:

- not the owner of survey semantics, processing semantics, or product workflow state
- not a backend transport layer

## Dependency Direction

The intended dependency direction is:

`TraceBoost -> Ophiolite core + Ophiolite Charts`

More concretely:

1. `ophiolite` defines canonical domain meaning and runtime primitives.
2. `Ophiolite Charts` adapts canonical or chart-normalized payloads into embeddable views.
3. `TraceBoost` composes those lower layers into a first-party workflow application.

## Control Surfaces

- `ophiolite` should expose platform-owned Rust APIs first, then thin CLI and Python wrappers where that makes sense for external automation.
- `TraceBoost` should expose app-owned workflow services in Rust first, then thin CLI, Python, and desktop command wrappers over those workflows.
- The point is shared ownership of behavior in Rust, not parallel reimplementation in each surface.

## Contracts And Flow

The intended flow is:

1. canonical contracts and DTO meaning are defined in `ophiolite`
2. generated TypeScript contracts are exported from that canonical meaning
3. `Ophiolite Charts` consumes canonical or normalized view payloads and turns them into reusable rendering behavior
4. `TraceBoost` consumes the same platform contracts and runtime capabilities to implement workflow UX and product automation

## Customer-Facing Story

The public platform story should be:

- `Ophiolite` is the subsurface platform.
- `Ophiolite Charts` is the embeddable charting layer within that platform.
- `TraceBoost` is one first-party application built on top of that platform.

That means Ophiolite and Ophiolite Charts should not need to explain TraceBoost in their own product docs. TraceBoost can explain how it uses them.

## Ownership Rules

- Put canonical DTO meaning and reusable runtime behavior in `ophiolite`.
- Put reusable chart behavior and embedder APIs in `Ophiolite Charts`.
- Put workflow composition, app-local session behavior, presets, and product automation recipes in `TraceBoost`.
- If a feature can be sold as platform capability independent of TraceBoost, it should move toward `ophiolite`.
- If a feature only makes sense as a TraceBoost workflow opinion, keep it in `TraceBoost`.

## Automation Ownership

- `ophiolite` should own automation that makes sense without TraceBoost attached:
  project creation/opening, package inspection and validation, canonical ingest, reusable preflight/import evidence, compute, map/well DTO resolution, processing identity/runtime evidence, lineage, and reusable geometry/query functions
- `TraceBoost` should own workflow automation that composes those lower-level capabilities into one seismic product flow:
  survey preflight/import composition, export recipes, app-local demo preparation, processing orchestration, typed workflow recipes, run-report rendering, and product-facing workflow shortcuts
- both repos can expose catalogs of declared operations, but they should describe different ownership layers rather than duplicate the same workflow names
