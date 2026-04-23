# Public SDK Roadmap

This document turns the current SDK audit and product-direction decisions into an execution plan for making `Ophiolite Charts` usable as a public commercial SDK.

Related context:

- [README.md](../README.md)
- [docs/public-sdk-audit.md](./public-sdk-audit.md)
- [packages/svelte/src/types.ts](../packages/svelte/src/types.ts)
- [packages/svelte/src/index.ts](../packages/svelte/src/index.ts)
- [packages/data-models/src/index.ts](../packages/data-models/src/index.ts)

## Product Position

`Ophiolite Charts` should be:

- a Svelte-first commercial SDK for specialized subsurface charts
- usable without `@ophiolite/contracts`
- domain-specific rather than generic-chart-config driven
- public at the root package, with Ophiolite integration as an explicit adapter layer

It should not be:

- a generic charting library
- an Ophiolite-internal frontend package with a public wrapper
- a multi-framework product on day one

## Target Public Boundary

### Default package

Use `@ophiolite/charts` as the only default documented surface.

Public root exports should contain only:

- stable launch chart components
- stable public chart props and handle types
- stable family-specific model types
- a small set of stable helper types and enums

### Explicit adapter package

Use `@ophiolite/charts/adapters/ophiolite` for:

- Ophiolite contract DTO adaptation
- contract-specific decoding
- Ophiolite-specific naming and transport concerns

### Internal packages

Treat these as internal or advanced until intentionally promoted:

- `@ophiolite/charts-data-models`
- `@ophiolite/charts-core`
- `@ophiolite/charts-renderer`
- `@ophiolite/charts-domain`

Do not lead public docs or examples with those packages.

## Main Problems To Fix

### 1. Public types leak Ophiolite contracts

Current issue:

- [packages/svelte/src/types.ts](../packages/svelte/src/types.ts) imports `@ophiolite/contracts`
- public props still expose transport-shaped data such as `horizontal_axis_f64le`, `sample_axis_f32le`, and `amplitudes_f32le`

Target:

- public chart props use neutral, chart-family-specific models such as `SeismicSectionData`, `SeismicGatherData`, `SurveyMapData`, `RockPhysicsCrossplotData`, and `WellCorrelationPanelData`
- Ophiolite adapters translate contracts into those public models

### 2. Root exports are too broad

Current issue:

- [packages/svelte/src/index.ts](../packages/svelte/src/index.ts) exports launch charts together with inspectors and adjacent widgets
- [packages/data-models/src/index.ts](../packages/data-models/src/index.ts) exports mocks, adapters, registries, and models from a single root

Target:

- narrow `@ophiolite/charts` root exports to the supported public launch surface
- move experimental, adjacent, or internal-ish exports off the main path

### 3. Public examples are still internal playgrounds

Current issue:

- [apps/svelte-playground/src/App.svelte](../apps/svelte-playground/src/App.svelte) is both lab and public-demo surface
- [apps/demo-playground/src/main.ts](../apps/demo-playground/src/main.ts) teaches controllers and renderers directly

Target:

- separate internal playgrounds from public examples and docs
- public examples must teach the root package, not the internal architecture

### 4. Benchmarks are not yet publishable evidence

Current issue:

- [apps/benchmark-app/src/main.ts](../apps/benchmark-app/src/main.ts) is useful as a smoke benchmark but does not establish publishable performance evidence

Target:

- a documented benchmark methodology with fixture definitions, repetition policy, machine metadata, renderer mode, and raw results

### 5. Test and release gates are too light

Current issue:

- automated coverage is narrow relative to the size of the public surface

Target:

- unit coverage for core math and adapters
- public API contract tests
- visual regression coverage for launch families
- benchmark methodology before benchmark marketing

## Public Model Rules

Public models should follow these rules:

- chart-family specific rather than one universal options object
- plain JS objects and arrays by default
- typed arrays allowed for dense scientific data
- semantic domain names, not transport-layer names
- easy to author by hand in simple examples

Public models should not expose:

- `_f32le` or `_f64le` transport fields
- `dataset_id` style transport naming
- Ophiolite DTO callback payload shapes

### Example direction

This is the level of clarity to target for public examples:

```ts
const section = {
  traces: [1000, 1001, 1002, 1003],
  samplesMs: [0, 4, 8, 12],
  amplitudes: new Float32Array([
    0.2, -0.1, 0.4, 0.3,
    0.1, 0.0, -0.2, 0.5,
    -0.3, 0.2, 0.1, -0.1,
    0.0, 0.4, 0.2, -0.2
  ])
};
```

Not this:

```ts
horizontal_axis_f64le
sample_axis_f32le
amplitudes_f32le
display_defaults
```

## Launch Scope

### Launch chart families

Prioritize these as the public early-access launch set:

- `SeismicSectionChart`
- `SeismicGatherChart`
- `SurveyMapChart`
- `RockPhysicsCrossplotChart`
- `WellCorrelationPanelChart`

### Preview or experimental families

Keep these outside the core launch story until the boundary is harder:

- `VolumeInterpretationChart`
- AVO family charts
- inspectors and adjacent analysis widgets

## Documentation Strategy

Build a public docs surface organized by buyer intent, not repo architecture.

Target IA:

- Getting Started
- Chart Families
- Examples
- API Reference
- Adapters
- Performance
- FAQ
- Migration

For each launch chart family, provide:

- what problem the chart solves
- a minimal `simple` example
- a `production` example
- performance notes
- integration notes

### Example policy

Each launch chart family must ship with exactly two public examples:

- `simple`
- `production`

`simple` example rules:

- roughly 30-60 lines
- obvious naming
- plain object literals where possible
- no internal package imports
- no Ophiolite contract knowledge required

`production` example rules:

- typed arrays where appropriate
- controlled viewport or state callbacks
- overlays, handles, and interaction hooks when relevant
- realistic embedding guidance

## Support And Compatibility Policy

At launch, the compatibility promise should apply only to:

- `@ophiolite/charts` root exports
- documented public props
- documented public handle methods
- documented public model types
- documented adapter entry points

Undocumented lower-level packages remain free to change.

Document a deprecation policy before broader public rollout.

## Explicit Non-Features

Define these early to keep the product sharp:

- no generic all-purpose chart DSL
- no Ophiolite transport DTOs in the default public API
- no built-in product workflow dialogs
- no data fetching or backend transport in the SDK
- no promise of multiple framework wrappers on day one
- no arbitrary composition system until the public chart-family APIs are mature

## Release Readiness Bar

Do not call the product public early access until all of the following exist:

- stable root exports for launch families
- neutral public model types for launch families
- explicit Ophiolite adapter package
- one canonical getting-started path
- one `simple` example and one `production` example per launch family
- public API reference for launch surfaces
- benchmark methodology document
- visual regression coverage for launch families
- changelog and migration notes

## Phased Execution Plan

### Phase 0: Boundary spec

Goal:

- freeze the target public SDK rules before more implementation work expands the wrong surface

Tasks:

- adopt this roadmap and [docs/public-sdk-audit.md](./public-sdk-audit.md) as the working spec
- define the launch chart families and preview chart families explicitly
- define which current root exports are in or out

Deliverables:

- approved product boundary doc
- approved launch-scope list

### Phase 1: Public model split

Goal:

- separate neutral public chart models from Ophiolite contracts

Tasks:

- design public model types for each launch family
- design adapter output types to match those public models exactly
- remove `@ophiolite/contracts` from the default public types path
- keep transport decoding inside the adapter layer

Deliverables:

- new public family model types
- explicit Ophiolite adapter entrypoints
- migration notes from current contract-shaped props

### Phase 2: Package and export hardening

Goal:

- make the root package look like a product, not a workspace mirror

Tasks:

- narrow [packages/svelte/src/index.ts](../packages/svelte/src/index.ts) to stable public exports
- stop leading public usage through [packages/data-models/src/index.ts](../packages/data-models/src/index.ts)
- move non-launch and experimental surfaces off the main public path
- add export-surface tests

Deliverables:

- cleaned root export surface
- package entrypoint policy
- export regression tests

### Phase 3: Docs and examples split

Goal:

- build a real public learning surface

Tasks:

- create a docs app or site separate from playgrounds
- add getting-started docs based only on `@ophiolite/charts`
- add `simple` and `production` examples for each launch family
- mark internal playgrounds as development tooling

Deliverables:

- public docs app
- launch-family example pages
- internal playground labeling cleanup

### Phase 4: Quality gates

Goal:

- support a public commercial readiness floor

Tasks:

- unit tests for geometry, viewport math, registry rules, and adapters
- visual regression tests for launch chart families
- API contract tests for public props and handles
- typed example validation in CI

Deliverables:

- public API test suite
- visual regression baseline
- example build and typecheck coverage

### Phase 5: Benchmark discipline

Goal:

- move from smoke benchmarks to decision-grade benchmark evidence

Tasks:

- define benchmark modes: `smoke`, `development`, `authoritative`
- define fixed benchmark fixtures per launch family
- capture machine, browser, renderer mode, and repetition data
- store raw results
- write benchmark methodology docs before publishing benchmark claims

Deliverables:

- benchmark methodology doc
- reproducible benchmark harness
- raw results format and storage policy

### Phase 6: Internal refactors after API freeze

Goal:

- reduce wrapper and renderer complexity without destabilizing the product boundary

Tasks:

- break down large wrappers such as [packages/svelte/src/SeismicSectionChart.svelte](../packages/svelte/src/SeismicSectionChart.svelte) and [packages/svelte/src/WellCorrelationPanelChart.svelte](../packages/svelte/src/WellCorrelationPanelChart.svelte)
- extract shared wrapper-family kernels where appropriate
- keep refactors behind the now-hardened public API

Deliverables:

- smaller wrapper units
- reduced duplication across chart families
- stable public behavior through refactor

## Suggested 30/60/90-Day Order

### First 30 days

- freeze launch scope
- define public model types
- design adapter boundary
- decide which root exports survive

### Next 30 days

- implement public model split for launch families
- clean root exports
- create the docs/examples app skeleton
- add one polished example per launch family

### Following 30 days

- complete `simple` and `production` examples
- add public API tests and visual regression
- formalize benchmark methodology
- publish early-access docs and migration notes

## Immediate Next Tasks

If starting implementation now, do these first:

1. Write the target public model types for the five launch families.
2. Design the exact `@ophiolite/charts/adapters/ophiolite` entrypoints.
3. Audit every current export in [packages/svelte/src/index.ts](../packages/svelte/src/index.ts) into `launch`, `preview`, or `internal`.
4. Draft the first `simple` example for `SeismicSectionChart` using only neutral public types.
5. Add an export-surface test so the public boundary stops drifting.
