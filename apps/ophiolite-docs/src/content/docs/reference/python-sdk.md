---
title: Python SDK
description: The primary builder surface for local-first Ophiolite workflows.
draft: false
---

**Audience:** workflow builders  
**Status:** Preview

The Python SDK is the main intended builder surface for Ophiolite.

It is designed to expose Ophiolite nouns such as `Project` and to keep lower-level transport, platform-admin, and operator-authoring details in explicit advanced namespaces instead of mirroring internal Rust modules directly.

It is also narrower than the full repo. The Python SDK is a platform surface, not a mirror of TraceBoost desktop commands or app-local workflow orchestration.

## Stable public vocabulary

The top-level package should teach durable subsurface and workflow language first:

- `Project`
- `Survey`
- `Well`
- `Wellbore`
- `WellboreBinding`
- `SectionSelection`
- `TraceLocalPipeline`
- `SubvolumePipeline`
- `GatherPipeline`
- `PostStackNeighborhoodPipeline`
- `VelocityScanSpec`
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

The rule for those advanced namespaces is:

- `analysis`, `avo`, `operators`, and `platform` remain platform-owned expert surfaces
- `interop` is an explicit compatibility and transport lane, not the primary teaching surface
- app-local TraceBoost transport names should not be copied into the Python SDK as public API

## Operator exposure

Built-in operators should usually be exposed through typed workflow surfaces instead of generic request objects.

For logs and AVO:

- `wellbore.elastic_log_set(bindings=...)`
- `elastic.run_avo(layering=..., experiment=...)`
- `result.response_source(...)`
- `result.crossplot_source(...)`

For seismic processing:

- `project.surveys()`
- `survey.operator_catalog()`
- `TraceLocalPipeline.named(...).bandpass(...).agc_rms(...)`
- `survey.preview_processing(selection, pipeline)`
- `survey.run_processing(pipeline, output_collection_name=...)`

For external Python operator authoring:

- `ophiolite_sdk.operators.OperatorRegistry`
- `ophiolite_sdk.operators.OperatorRequest`
- `ophiolite_sdk.operators.computed_curve(...)`

This keeps built-in workflow composition and external operator authoring visible without collapsing them into one generic API shape.

## What it is good for

- local project lifecycle
- survey-backed seismic discovery
- project-owned seismic execution
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
- use `Survey` plus typed seismic workflow objects when the data already lives in an `OphioliteProject`
- use `TraceBoostApp` and `SeismicDataset` only when the workflow is intentionally loose-store and outside project asset ownership
- use `project.views` when you want explicit project-scoped well-panel, survey-map, and section-overlay resolution
- use `Wellbore.log_curves()` when you want the current well-log curves resolved through the project layer
- use `Wellbore.elastic_log_set(bindings=...)` when you want semantic elastic inputs for log-derived workflows such as AVO
- use `ophiolite_sdk.avo` for domain-first AVO spec objects such as `ElasticChannelBindings`, `LayeringSpec`, `AngleSampling`, and `AvoExperiment`
- use `ophiolite_sdk.analysis` for explicit kernel-style request/response APIs
- use `ophiolite_sdk.operators` for Python operator authoring helpers
- use `ophiolite_sdk.platform` for platform/admin introspection such as the operation catalog
- use `ophiolite_sdk.interop` for raw typed transport models and compatibility-facing shapes

## Public versus compatibility lanes

Teach the Python SDK in this order:

- domain-first object graph and typed workflow helpers first
- explicit advanced platform namespaces second
- compatibility lanes only when the workflow genuinely needs them

The current compatibility lanes are:

- `ophiolite_sdk.interop` for raw transport-shaped models
- `TraceBoostApp` and `SeismicDataset` for intentionally loose-store workflows outside project ownership

Those are valid surfaces, but they are not the main public promise. They should stay clearly separated from the domain-first `ophiolite_sdk` story.

## Object-first workflow shape

The intended builder flow is:

- `Project` opens or creates the local workspace
- `Project.import_las(...)` can seed canonical log assets directly, with `binding=wellbore.binding()` when vendor headers need explicit wellbore attachment
- `Project.wells()` and `Project.surveys()` discover the main domain objects
- `Well.wellbores()` and `Well.surveys()` continue the project graph without dropping back to raw ids
- `Survey.operator_catalog()` discovers the available seismic operator families for that project-owned asset
- `Survey.preview_processing(...)`, `Survey.run_processing(...)`, `Survey.preview_subvolume(...)`, `Survey.run_subvolume(...)`, `Survey.preview_gather(...)`, `Survey.run_gather(...)`, `Survey.preview_post_stack_neighborhood(...)`, and `Survey.velocity_scan(...)` keep seismic execution on canonical asset ids instead of raw store paths
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

The same domain-first rule applies to seismic processing. For project-owned seismic assets, the public story should read as typed surveys and typed operators, not raw command plumbing:

```python
from ophiolite_sdk import Project, SectionSelection, TraceLocalPipeline

project = Project.open("demo-project")
survey = project.surveys()[0]
selection = SectionSelection.inline(120)
pipeline = (
    TraceLocalPipeline.named(
        "Bandpass + RMS AGC",
        description="Trace-local seismic golden path.",
    )
    .bandpass(8.0, 12.0, 45.0, 60.0)
    .agc_rms(40.0)
)

catalog = survey.operator_catalog()
preview = survey.preview_processing(selection, pipeline)
processed = survey.run_processing(
    pipeline,
    output_collection_name="derived-seismic",
)
```

That keeps discovery and execution on project-owned asset ids while still exposing operators as explicit workflow objects.

If the workflow is intentionally outside project ownership, `TraceBoostApp` and `SeismicDataset` remain available as the loose-store compatibility lane for direct `.tbvol` and `.tbgath` work.

## Relationship to the CLI

The Python SDK and CLI should expose the same platform meanings. The SDK is the preferred builder surface. The CLI remains useful for scripting, CI, and operational tasks.

Neither surface should be treated as a wrapper over TraceBoost desktop commands. All three may reach the same Rust-owned behavior, but the desktop command boundary remains app-local.

## Deprecations

Some preview aliases still exist for compatibility, but the docs should always teach the preferred current names first.

The public placeholder for that migration ledger is [Python SDK deprecations](/docs/advanced/python-sdk-deprecations/).
