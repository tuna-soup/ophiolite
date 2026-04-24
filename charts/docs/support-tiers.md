# Support Tiers

This document defines the small support-tier vocabulary used by the package manifests in `packages/*/ophiolite.module.json`.

The goal is to keep the public SDK story explicit without introducing a large module system.

The checked, normalized catalog emitted from those manifests lives at `manifests/generated/module-catalog.json`.

## Tier meanings

### `public-launch`

The default documented surface for embedders.

Use this tier for:

- launch chart wrappers
- public model types
- public handle types
- companion packages that are safe to document beside the launch wrappers

### `public-adapter`

The explicit integration layer for Ophiolite-shaped inputs.

Use this tier for:

- adapter entrypoints that translate Ophiolite DTOs into neutral chart models
- integration guidance that should appear after the default launch path, not before it

### `preview`

The explicit opt-in tier for narrower guarantees.

Use this tier for:

- preview chart families
- adjacent widgets that are useful but should not widen the default product promise
- preview rendering or domain entrypoints that exist to support those surfaces

### `internal`

The implementation layer behind the public SDK.

Use this tier for:

- data-model, core, renderer, and domain packages
- internal compatibility surfaces
- lower-level APIs that should remain free to change while the public product hardens

## Current package map

### Public first

- `@ophiolite/charts`
  `public-launch` package with `public-launch`, `public-adapter`, `preview`, and `internal` subpaths.
- `@ophiolite/charts-toolbar`
  `public-launch` companion package for chart interaction chrome.

### Public but explicit

- `@ophiolite/charts/adapters/ophiolite`
  `public-adapter` subpath on the main wrapper package.

### Opt-in

- `@ophiolite/charts/preview`
- `@ophiolite/charts/extras`

### Internal implementation layers

- `@ophiolite/charts-data-models`
- `@ophiolite/charts-core`
- `@ophiolite/charts-renderer`
- `@ophiolite/charts-domain`

## Compatibility promise

The practical compatibility promise in this slice is:

- document and stabilize `public-launch` surfaces first
- document `public-adapter` surfaces explicitly but narrowly
- require opt-in for `preview` surfaces
- avoid implying compatibility guarantees for `internal` packages

## Why TraceBoost matters here

`traceboost-demo` is useful because it forces the public boundary to stay honest.

The intended rule is:

- TraceBoost uses the same approved public packages that an external consumer would use
- charts internals stay behind those package boundaries

That is why support tiers and package manifests matter even before the system is fully wired into all checks.

## Manifest files

Package manifests live at:

- `packages/data-models/ophiolite.module.json`
- `packages/chart-core/ophiolite.module.json`
- `packages/renderer/ophiolite.module.json`
- `packages/domain-geoscience/ophiolite.module.json`
- `packages/svelte/ophiolite.module.json`
- `packages/svelte-toolbar/ophiolite.module.json`

Validate them with:

```bash
bun run validate:manifests
```
