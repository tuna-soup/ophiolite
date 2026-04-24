# Chart/Runtime Extension Lifecycle Sketch

This note maps the current chart manifest and runtime shape onto the shared capability lifecycle introduced in `docs/architecture/ADR-0035-boundary-manifest-and-capability-registry.md`.

It is intentionally narrow:

- Ophiolite Charts does not have a dynamic plugin system today
- chart packages are still first-party workspace code, bundled normally
- the goal is to name the current discovery, validation, and activation seams without overstating them

## Current Discovery Metadata In Charts

Today the charts workspace has three concrete discovery sources:

| Current source | What it describes | Current files |
| --- | --- | --- |
| package/module manifests | package identity, support tier, entrypoints, dependency roles, consumer guarantees, required test suites | `charts/packages/*/ophiolite.module.json`, `charts/manifests/schemas/ophiolite-charts-module.schema.json` |
| generated manifest catalog | normalized, docs/tooling-friendly view of those manifests | `charts/manifests/generated/module-catalog.json`, `charts/apps/public-docs/src/lib/generated/manifest-catalog.ts` |
| chart definition registry | chart ids, families, public surfaces, canonical boundaries, renderer kernels, allowed asset families, adapter/validation entrypoints, consumer guarantees | `charts/packages/data-models/src/chart-registry.ts` |

The manifest layer is validated by `charts/scripts/validate-module-manifests.ts`. The runtime-facing renderer vocabulary lives beside the chart registry in `charts/packages/data-models/src/renderer-capabilities.ts`.

## Mapping Current Pieces To The Capability Model

The useful mapping is conceptual, not yet a shared `CapabilityRegistry` implementation for charts.

| Capability concept from ADR-0035 | Current chart/runtime equivalent | Notes |
| --- | --- | --- |
| discovery | `ophiolite.module.json` files and `CHART_DEFINITIONS` / `CHART_FAMILIES` | This is where the repo currently says "these chart surfaces exist" and "these are their declared boundaries". |
| validation | manifest validation, chart constraints, adapter entrypoints, validation entrypoints, support-tier tests | `charts/scripts/validate-module-manifests.ts` validates package metadata; `charts/packages/data-models/src/chart-registry.ts` records semantic constraints such as canonical boundaries and allowed asset families. |
| activation | renderer backend selection and runtime status resolution | `charts/packages/renderer/src/capabilities.ts` and `charts/packages/svelte/src/renderer-status.ts` separate "supported by this chart" from "available in this host right now" and from "runtime failed after mount". |

That separation matters because it already matches the capability lifecycle rule in `crates/ophiolite-capabilities`:

- discovery says a chart surface or renderer path exists
- validation says whether the package shape, payload shape, and requested backend are acceptable
- activation says which backend actually became active, or whether runtime failure occurred

The current chart code already avoids the main architectural mistake ADR-0035 warns about: discovery does not automatically imply successful activation.

## How Chart Definitions, Manifests, And Renderer Checks Line Up

`charts/packages/*/ophiolite.module.json` is the package-level discovery layer. It answers:

- which chart packages are public, preview, or internal
- which entrypoints are intentionally exposed
- which dependencies are runtime, adapter-runtime, peer, or preview-only

`charts/packages/data-models/src/chart-registry.ts` is the chart-family and chart-instance discovery layer. It answers:

- which chart definitions exist
- which public wrapper surface owns each definition
- which canonical source boundary is expected
- which adapter and validation entrypoints are relevant
- which renderer kernel and backends the chart claims to support

`charts/packages/renderer/src/capabilities.ts` plus `charts/packages/svelte/src/renderer-status.ts` is the activation seam. It answers:

- whether a requested backend is supported by the chart at all
- whether that backend is available in the current host
- whether the system fell back to a different backend
- whether a runtime failure happened after an initially valid selection

That is enough to treat current charts as built-in capabilities with an explicit lifecycle, even though no dynamic loading exists.

## What Remains Platform-Owned Vs Chart-Owned

Chart-owned concerns today:

- chart ids, chart families, and wrapper entrypoints
- chart-native payload constraints and interaction profiles
- renderer-kernel declarations and supported backend lists
- support-tier declarations inside the chart workspace

Platform-owned or app-owned concerns today:

- canonical subsurface DTO ownership and semantic contracts outside the chart boundary
- application assembly, transport, and workflow state
- whether preview or internal chart surfaces are exposed in a given app
- bundling and shipping optional runtime dependencies such as preview renderer stacks
- host/runtime facts such as available backends, browser capabilities, worker policy, and failure reporting

In capability-model terms, charts own most of the durable discovery vocabulary, while the host application still owns activation policy and actual environment compatibility.

## Why This Is Inspired By ParaView/GDAL, But Kept Minimal

The ParaView/GDAL lesson in `ADR-0035` is not "copy their plugin systems". The useful lesson is smaller:

- keep discovery metadata explicit
- keep validation explicit
- keep activation separate from discovery
- keep first-party application assembly outside the reusable capability vocabulary

For Ophiolite Charts, that translates to a deliberately minimal shape:

- static manifests instead of a large module loader
- typed chart-definition registries instead of a broad runtime reflection layer
- backend-status resolution instead of a generalized plugin activation framework

That is the right level for the current repo because charts are still first-party packages in one workspace, not independently shipped third-party binaries with a stable plugin ABI.

## Proposed Next Slice After The Chart Worktree Stabilizes

The next implementation slice should stay non-dynamic and build-time first:

1. generate a chart capability catalog from `charts/manifests/generated/module-catalog.json` plus `charts/packages/data-models/src/chart-registry.ts`
2. mark those records as effectively `built_in` capability sources when mirrored into shared vocabulary
3. add a small host-side lifecycle probe that converts renderer availability into explicit validation/activation records for each chart definition

Concretely, that likely means adding one generated artifact near the existing manifest catalog, not inventing a loader:

- input: module manifests, chart definitions, renderer backend declarations
- output: a stable "chart capability catalog" that apps can read to decide what is discoverable, what is valid in the current host, and what actually activated

That would let Ophiolite and TraceBoost talk about chart/runtime extensions using the same lifecycle language as import providers and operators, without claiming that charts are already hot-loadable plugins.
