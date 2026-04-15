# ADR-0020: AVO Chart Family, Analysis Contracts, and the Initial Rendering Plan

## Status

Accepted

## Decision

`Ophiolite Charts` will add AVO as a strict chart family with analysis-specific display DTOs that stay separate from raw compute requests and responses.

The immediate AVO family shape is:

- `AvoResponseChart`
- `AvoInterceptGradientCrossplotChart`
- `AvoChiProjectionHistogramChart`

These charts consume resolved analysis DTOs, not raw seismic volumes, not generic XY tables, and not backend compute payloads such as `AvoReflectivityRequest` or `AvoReflectivityResponse`.

The current chart-side DTO boundary is:

- `ResolvedAvoResponseSourceDto`
- `ResolvedAvoCrossplotSourceDto`
- `ResolvedAvoChiProjectionSourceDto`

## Why

The backend is mature enough for the first AVO modeling slice, but not for the whole AVO product surface.

Today we already have runtime support for:

- modeled PP reflectivity curves
- multiple reflectivity methods:
  - `shuey_three_term`
  - `aki_richards`
  - `zoeppritz_pp`
- Shuey intercept and gradient outputs

Today we do **not** yet have a canonical AVO chart-source model for:

- Monte Carlo realizations as first-class analysis populations
- chi-angle projected populations
- weighted-stack feasibility outputs as persisted runtime assets
- anisotropy parameter sets beyond display metadata
- raw pre-stack seismic response sets adapted into chart-facing DTOs

That maturity line matters. If we jump straight to a generic chart or wire renderers directly to compute payloads, we will collapse three different concerns into one model:

- canonical Ophiolite semantics
- backend compute transport
- chart-native rendering payloads

The chart family should stay strict while the upstream analysis model is still settling.

## Decision Details

### 1. AVO stays a strict family, not a generic XY chart

The initial AVO charts are not instances of a universal scatter or line framework exposed at the public API level.

They are strict charts with explicit semantic boundaries:

- `AvoResponseChart` accepts modeled interface response series
- `AvoInterceptGradientCrossplotChart` accepts intercept-gradient sample clouds
- `AvoChiProjectionHistogramChart` accepts projected chi-angle sample populations

We may later add a generic crossplot family, but it does not replace these AVO-specific contracts.

### 2. Compute payloads are not chart payloads

`AvoReflectivityRequest` and `AvoReflectivityResponse` remain compute/runtime contracts.

They are useful for:

- backend kernels
- batch analysis
- service boundaries
- future authored analysis sessions

They are not sufficient as chart payloads because they do not carry:

- interface descriptors
- chart labels and axis policies
- crossplot regions or trend lines
- chi-projection series semantics
- display-ready population grouping

### 3. Weighted-stack plot maps to chi projection, not a separate first data model

The external AVO tools we reviewed describe weighted-stack plots as a complement to the intercept-gradient crossplot for finding an optimal chi angle.

For our architecture, that means the first canonical interpretation is:

- chi-angle projection study
- histogram or separability view over projected samples

So the chart family keeps `AvoChiProjectionHistogramChart` as the primary model name.

UI labels may later alias this as:

- `Weighted Stack`
- `Chi Projection`
- `EEI Feasibility`

But the data model should stay explicit about what the samples mean.

### 4. Reuse kernels through composition, not inheritance

We should not create a deep class hierarchy such as:

- `BaseChart`
- `CartesianChart`
- `AvoChart`
- `AvoInterceptGradientChart`

Instead:

- reuse the existing point-cloud kernel shape for the intercept-gradient crossplot
- add a narrow cartesian line kernel for response curves
- add a narrow histogram kernel for chi projections
- extract shared cartesian helpers only after the second family actually uses them

This keeps the codebase consistent with the existing seismic, survey-map, and rock-physics direction.

### 5. AVO crossplot should reuse the point-cloud path first

The intercept-gradient crossplot is the best first AVO renderer because it fits the current stack:

- point cloud data is already columnar
- the renderer already handles dense scatter efficiently
- pan, probe, and crosshair behavior already exist in a close neighbor chart

This means the first implementation should adapt the point-cloud kernel for:

- interface-colored populations
- optional background classification regions
- optional reference lines
- optional chi-projection / simulation id columns

We should not build a separate scatter engine for AVO.

### 6. AVO response should be a dedicated line chart, not retrofitted into seismic

The angle-versus-reflectivity response plot is a cartesian line chart with semantics that differ from seismic sections and gathers:

- x is angle, not trace or depth
- y is reflectivity, not amplitude sample position
- multiple interface series share the same axes
- anisotropic and isotropic curves may coexist

So it should get its own narrow line renderer and controller, with shared interaction vocabulary:

- `pointer`
- `crosshair`
- `pan`
- `fitToData`

### 7. Canonical upstream model should split authored analysis from resolved chart views

The eventual upstream shape should distinguish:

- authored AVO study inputs
- runtime compute outputs
- resolved chart view DTOs

The family likely wants upstream concepts along these lines:

- authored interface scenario
- resolved interface analysis set
- resolved intercept-gradient population
- resolved chi-angle projection study

The chart layer should continue consuming only the resolved chart-view DTOs.

### 8. Backend maturity gates the rollout order

Because current runtime support is strongest for modeled reflectivity and Shuey intercept/gradient, the rollout order should be:

1. `AvoInterceptGradientCrossplotChart`
2. `AvoResponseChart`
3. `AvoChiProjectionHistogramChart`

`AvoChiProjectionHistogramChart` can exist in the registry now, but production use should wait until upstream can materialize projected populations cleanly.

## Consequences

- AVO now has the same family-level boundary discipline as seismic, survey-map, well-panel, and rock physics
- the chart library avoids turning compute DTOs into de facto render models
- the intercept-gradient crossplot can ship quickly on top of the point-cloud path
- weighted-stack/chi views stay semantically correct instead of becoming a vague bar-chart abstraction
- future generic XY work can reuse cartesian helpers without weakening strict family contracts

## Immediate Implementation Plan

### 1. Stabilize the family boundary

Done in this slice:

- add AVO family entries to the chart registry
- add AVO display DTOs and versioning
- export generated TypeScript contracts
- add chart-native AVO models plus validation/adapters

### 2. Add mock AVO view models and a visible demo

Next implementation slice:

- create mock AVO response, crossplot, and chi-projection sources
- add an AVO route to the Svelte playground
- expose the same toolbar/interactions used by rock-physics where applicable

### 3. Ship the intercept-gradient crossplot first

First production renderer slice:

- adapt the point-cloud kernel for AVO
- add AVO-specific probe fields
- render interface regions and reference lines
- support exact vs progressive hit-testing at large sample counts

### 4. Add the response line chart second

Second renderer slice:

- implement a narrow cartesian line renderer
- keep typed-array series storage
- support mixed isotropic / anisotropic style variants

### 5. Delay weighted-stack histogram until upstream population semantics are ready

Do not fake this chart from loosely defined bins.

Production rollout should wait for an upstream resolver that can emit:

- projected chi-angle values per population
- population labels and colors
- optional summary statistics and preferred bin counts

## Non-Goals

This ADR does not yet define:

- the final authored AVO study asset types in core Ophiolite
- Monte Carlo uncertainty session models
- pre-stack seismic AVO response ingestion into the chart family
- anisotropy parameter authoring contracts
- a generic public Cartesian chart API

Those should follow after the first AVO chart slice is in use.
