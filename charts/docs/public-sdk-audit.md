# Public SDK Audit

This document records the current gaps between the internal chart architecture and the public product surface needed for `ophiolitecharts.com`.

## Product Boundary

The launch product direction is:

- specialized scientific and subsurface chart SDK
- Svelte-first public wrappers
- JavaScript embedding guidance for the same wrappers
- early-access commercial rollout

The public story should center on `@ophiolite/charts`.

## Current Strengths

The codebase already has several properties that support a commercial SDK:

- layered package structure with wrapper, data-model, renderer, and controller separation
- explicit chart registry and per-family constraints
- shared interaction vocabulary across chart families
- existing wrapper handles like `fitToData`, `setViewport`, `zoomBy`, and `panBy`

Representative references:

- [README.md](../README.md)
- [packages/data-models/src/chart-registry.ts](../packages/data-models/src/chart-registry.ts)
- [packages/data-models/src/interactions.ts](../packages/data-models/src/interactions.ts)

## Current Gaps

### 1. Public types still leak Ophiolite-native contracts

Examples:

- `packages/svelte/src/types.ts` imports `@ophiolite/contracts`
- `SectionViewLike` and `GatherViewLike` expose encoded contract-style byte arrays
- wrapper props still use Ophiolite DTO naming and contract callback shapes

Consequence:

- external buyers will read the product as an internal Ophiolite frontend surface rather than a standalone chart SDK

### 2. Public exports mix product surfaces with internal helpers

Examples:

- `packages/data-models/src/index.ts` exports mocks, adapters, validators, and chart models from one surface

Consequence:

- the default import story is noisy
- examples and docs will tend to drift into internal implementation details

### 3. No single public extension model is documented

Current strengths exist, but there is not yet a simple buyer-facing answer to:

- how data is added
- how data is removed
- how overlays are added
- how interactions are configured
- what can be customized safely

Consequence:

- the product will look powerful in demos but ambiguous in real adoption decisions

### 4. Wrapper consistency is partial rather than explicit

Several wrappers already expose imperative handles, but the supported set is not yet defined as a public contract across launch charts.

Consequence:

- customers may rely on accidental similarities that later diverge

### 5. Public examples are still internal playgrounds

The current Svelte playground is strong for development, but it is not yet the public examples surface.

Consequence:

- too many controls
- too much internal language
- not enough narrative around what each chart solves

## Hardening Backlog

### A. Define the public package boundary

Target:

- `@ophiolite/charts` is the default documented surface
- lower-level packages are treated as advanced or internal until intentionally promoted

Actions:

- document the launch surface in `packages/svelte`
- stop leading examples with lower-level imports
- keep `@ophiolite/charts-data-models` out of first-pass getting-started docs

Progress:

- added a cleaner adapter path at `@ophiolite/charts/adapters/ophiolite`
- stopped re-exporting adapter utilities from the root `@ophiolite/charts` entry point

### B. Split neutral public models from Ophiolite adapters

Target:

- public chart models are neutral
- Ophiolite adapters are explicit integration helpers

Actions:

- identify per-family public model types
- move Ophiolite-specific adapter guidance under a distinct adapter namespace in docs
- reduce direct `@ophiolite/contracts` references in wrapper-facing docs

### C. Standardize imperative handles across launch charts

Target public handle set:

- `fitToData`
- `setViewport`
- `zoomBy`
- `panBy`

Actions:

- audit each launch wrapper
- document which handles are guaranteed per family
- avoid documenting chart-local extras until the family contract is stable

Progress:

- added explicit public handle interfaces to `packages/svelte/src/types.ts` for the launch families and adjacent families

### D. Document one interaction model

Target:

- one concise explanation of tools, modifiers, callbacks, and interaction events

Actions:

- map internal interaction terms to public docs language
- define what is stable in early access
- avoid surfacing every internal target kind before needed

### E. Separate examples from the internal playground

Target:

- public examples become curated demo pages

Actions:

- select one polished example per launch chart family
- strip internal toggles that are useful only for development
- link each example to the relevant docs and API section

### F. Raise the commercial readiness floor

Target before moving beyond early access:

- at least four polished examples
- one canonical getting-started path
- documented public model boundaries
- reproducible benchmark methodology
- basic wrapper and adapter regression coverage

## Immediate Work Items

1. Scaffold `apps/ophiolitecharts-docs`
2. Build the public IA around docs, examples, pricing, benchmarks, and contact
3. Curate launch chart families
4. Define the public handle contract for those families
5. Separate adapter guidance from the default getting-started flow
