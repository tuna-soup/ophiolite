---
title: Python SDK
description: The primary builder surface for local-first Ophiolite workflows.
draft: false
---

**Audience:** workflow builders  
**Status:** Preview

The Python SDK is the main intended builder surface for Ophiolite.

It is designed to expose Ophiolite nouns such as `Project` and to keep lower-level transport, platform-admin, and operator-authoring details in explicit advanced namespaces instead of mirroring internal Rust modules directly.

## Stable public vocabulary

The top-level package should teach durable subsurface and workflow language first:

- `Project`
- `Survey`
- `Well`
- `Wellbore`
- `WellboreBinding`
- `TraceBoostApp`
- `SeismicDataset`
- `SectionSelection`
- `TraceProcessingPipeline`
- `BandpassFilter`
- `RmsAgc`
- `avo_reflectivity(...)`
- `rock_physics_attribute(...)`
- `avo_intercept_gradient_attribute(...)`

The main rule is:

- domain nouns and typed workflow helpers belong in `ophiolite_sdk`
- raw transport shapes, platform internals, and extension plumbing do not

Advanced namespaces:

- `ophiolite_sdk.analysis`
- `ophiolite_sdk.avo`
- `ophiolite_sdk.operators`
- `ophiolite_sdk.platform`
- `ophiolite_sdk.interop`

## Operator exposure

Built-in operators should usually be exposed through typed workflow surfaces instead of generic request objects.

For logs and AVO:

- `wellbore.elastic_log_set(bindings=...)`
- `elastic.run_avo(layering=..., experiment=...)`
- `result.response_source(...)`
- `result.crossplot_source(...)`

For seismic processing:

- `TraceBoostApp.open_dataset(...)`
- `TraceProcessingPipeline.named(...).bandpass(...).agc_rms(...)`
- `dataset.preview_processing(selection, pipeline)`
- `dataset.run_processing(pipeline, output_store_path=...)`

For external Python operator authoring:

- `ophiolite_sdk.operators.OperatorRegistry`
- `ophiolite_sdk.operators.OperatorRequest`
- `ophiolite_sdk.operators.computed_curve(...)`

This keeps built-in workflow composition and external operator authoring visible without collapsing them into one generic API shape.

## What it is good for

- local project lifecycle
- survey-backed seismic discovery
- well and wellbore navigation
- typed built-in operator composition
- operator package installation
- compute catalog lookup
- compute execution
- Python operator authoring

## What it is not yet

- a full mirror of every Rust capability
- a cloud API client
- a place to expose storage internals as the primary abstraction

## Namespace guidance

- use `ophiolite_sdk` for the main builder story
- use `Well`, `Wellbore`, and `Survey` for the main object graph and convenience helpers
- use `WellboreBinding` when a later ingest needs to attach explicitly to an existing canonical wellbore
- use `TraceBoostApp`, `SeismicDataset`, and `TraceProcessingPipeline` when the workflow is seismic and operator-chain oriented
- use `project.views` when you want explicit project-scoped well-panel, survey-map, and section-overlay resolution
- use `Wellbore.log_curves()` when you want the current well-log curves resolved through the project layer
- use `Wellbore.elastic_log_set(bindings=...)` when you want semantic elastic inputs for log-derived workflows such as AVO
- use `ophiolite_sdk.avo` for domain-first AVO spec objects such as `ElasticChannelBindings`, `LayeringSpec`, `AngleSampling`, and `AvoExperiment`
- use `ophiolite_sdk.analysis` for explicit kernel-style request/response APIs
- use `ophiolite_sdk.operators` for Python operator authoring helpers
- use `ophiolite_sdk.platform` for platform/admin introspection such as the operation catalog
- use `ophiolite_sdk.interop` for raw typed transport models and compatibility-facing shapes

## Object-first workflow shape

The intended builder flow is:

- `Project` opens or creates the local workspace
- `Project.import_las(...)` can seed canonical log assets directly, with `binding=wellbore.binding()` when vendor headers need explicit wellbore attachment
- `Project.wells()` and `Project.surveys()` discover the main domain objects
- `Well.wellbores()` and `Well.surveys()` continue the project graph without dropping back to raw ids
- `Well.panel()`, `Wellbore.panel()`, `Wellbore.trajectory()`, `Survey.map_view()`, and `Survey.section_well_overlays()` delegate to the same Rust-owned view resolvers exposed under `project.views`
- `Wellbore.log_curves()` and `Wellbore.elastic_log_set(bindings=...)` keep log and AVO preparation object-first without exposing storage internals as the primary abstraction
- `elastic.run_avo(layering=..., experiment=...)` applies domain-first AVO spec objects instead of requiring raw request payloads in the main builder story

This keeps the public story domain-first while still leaving lower-level interop, admin, and authoring shapes available in explicit advanced namespaces.

## Log and AVO workflow notes

`Wellbore.elastic_log_set()` is semantic rather than LAS-specific.

The helper resolves the best current elastic inputs in this order:

- `PVelocity`, otherwise `Sonic -> Vp`
- `SVelocity`, otherwise `ShearSonic -> Vs`
- `BulkDensity`

For interval-based AVO workflows, the intended progression is:

- `Wellbore.elastic_log_set(bindings=ElasticChannelBindings(...))`
- `Wellbore.top_set(...)` when the workflow should follow authored intervals
- `LayeringSpec.fixed_interval(...)` or `LayeringSpec.from_edges(...)` or `LayeringSpec.from_top_set(...)` or `top_set.layering(...)`
- `AvoExperiment.zoeppritz(...)`
- `elastic.run_avo(layering=..., experiment=...)`
- `result.response_source(...)`

Users are not limited to fixed bin lengths. The domain-first AVO surface supports:

- fixed intervals with `LayeringSpec.fixed_interval(...)`
- explicit depth edges with `LayeringSpec.from_edges([...])`
- named top-set intervals with `LayeringSpec.from_top_set(...)`
- exact interval-set selectors with `top_set.layering(selectors=[...])`

The stable log-type and interval-set story is meant to stay domain-first:

```python
from ophiolite_sdk.avo import (
    AngleSampling,
    AvoExperiment,
    ElasticChannelBindings,
    LayeringSpec,
)

elastic = wellbore.elastic_log_set(
    bindings=ElasticChannelBindings(vp="Dt", vs="Dts", density="Rho")
)

top_set = wellbore.top_set("lithostrat-tops") or wellbore.top_set()
layering = (
    top_set.layering(selectors=top_set.interval_selectors[:2])
    if top_set is not None and len(top_set.interval_selectors) >= 2
    else LayeringSpec.fixed_interval(20, unit="ft")
)

result = elastic.run_avo(
    layering=layering,
    experiment=AvoExperiment.zoeppritz(
        angles=AngleSampling.range(0, 40, 5)
    ),
)
```

That lets the public API read in canonical subsurface nouns even when the source LAS carried vendor mnemonics such as `DTCO`, `DTSM`, and `BDCX`, or when imported tops contain repeated labels that need exact selectors such as `NLLFC#1`.

The lower-level layering helpers remain available for advanced users, but the main SDK story should now prefer the `ophiolite_sdk.avo` spec objects.

That keeps LAS ingest canonical and reusable. A sonic curve can remain the source of truth while a later workflow chooses whether to keep `Vp` and `Vs` virtual or materialize derived direct curves with `elastic.materialize_missing_channels(...)`.

## Seismic operator workflow notes

The same domain-first rule applies to seismic processing. The public story should read as typed datasets and typed operators, not raw command plumbing:

```python
from ophiolite_sdk import SectionSelection, TraceBoostApp, TraceProcessingPipeline

app = TraceBoostApp()
dataset = app.open_dataset("input.tbvol")
selection = SectionSelection.inline(120)
pipeline = (
    TraceProcessingPipeline.named(
        "Bandpass + RMS AGC",
        description="Trace-local seismic golden path.",
    )
    .bandpass(8.0, 12.0, 45.0, 60.0)
    .agc_rms(40.0)
)

preview = dataset.preview_processing(selection, pipeline)
processed = dataset.run_processing(
    pipeline,
    output_store_path="input_bandpass_agc.tbvol",
)
```

That exposes operators as explicit workflow objects while still keeping the runtime contract strongly typed.

## Relationship to the CLI

The Python SDK and CLI should expose the same platform meanings. The SDK is the preferred builder surface. The CLI remains useful for scripting, CI, and operational tasks.

## Deprecations

Some preview aliases still exist for compatibility, but the docs should always teach the preferred current names first.

The public placeholder for that migration ledger is [Python SDK deprecations](/docs/advanced/python-sdk-deprecations/).
