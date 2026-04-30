# Evidence-Backed Subsurface Platform Direction

Date: 2026-04-30

## Purpose

This note turns the current Ophiolite / Ophiolite Charts / TraceBoost direction
into an operating model.

The architectural center should not be "SDK plus demo apps". The center should
be reproducible subsurface workflows backed by canonical contracts, real-data
adapters, validation fixtures, benchmark evidence, and chart views that prove the
stack works on messy data.

## Reference Repos Cloned For Study

The following public repositories were cloned under
`/Users/sc/dev/reference-repos` as implementation references:

| Repo | Local path | Use as inspiration for |
|---|---|---|
| `equinor/segyio` | `/Users/sc/dev/reference-repos/segyio` | SEG-Y strict/lenient import behavior, trace/header access, format quirks as reader evidence |
| `TGSAI/mdio-python` | `/Users/sc/dev/reference-repos/mdio-python` | MDIO/Zarr chunked multidimensional seismic import, templates, cloud/local source separation |
| `ahay/src` | `/Users/sc/dev/reference-repos/madagascar-src` | Reproducible geophysical processing histories and compact recipe thinking |
| `nextflow-io/nf-prov` | `/Users/sc/dev/reference-repos/nf-prov` | Event collection separated from provenance/report renderers |
| `cornerstonejs/cornerstone3D` | `/Users/sc/dev/reference-repos/cornerstone3D` | Specialized medical imaging viewport/tool boundaries and downstream validation |
| `JintaoLee-Roger/cigvis` | `/Users/sc/dev/reference-repos/cigvis` | Domain-native geophysical visualization nodes/layers |

These repos are references only. Ophiolite should adapt architectural patterns,
not copy implementation code or public API shape.

## Product Boundary

Ophiolite owns canonical subsurface meaning:

- asset and project contracts
- runtime and processing identity
- reusable import/preflight evidence
- operator/runtime behavior
- thin CLI/Python automation over Rust-owned behavior
- benchmark and validation surfaces that make sense without TraceBoost

Ophiolite Charts owns specialized visualization:

- launch chart-family components and public model types
- chart-native interaction, viewport, overlay, and handle contracts
- explicit Ophiolite adapters
- visual regression and chart-performance evidence

TraceBoost owns reference workflows:

- app-local recipes and workflow orchestration
- file grants, dialogs, recent sessions, and workspace activation
- proof-harness runs over real or curated datasets
- report rendering and product-facing workflow shortcuts

The dependency direction remains:

```text
TraceBoost -> Ophiolite core + Ophiolite Charts
```

## Readiness Rule

A capability is not product-ready until the relevant proof loop exists:

1. canonical contract or clearly app-local DTO
2. Rust-owned runtime or adapter behavior
3. CLI/app control surface over the same behavior
4. fixture or curated real-data manifest
5. validation checks with warnings/blockers where applicable
6. benchmark or timing evidence where performance is part of the claim
7. chart/view path when the capability is visual
8. recipe/report artifact when the capability is workflow-level

Use `docs/development/proof-readiness-checklist.md` as the lightweight template
for recording those answers before treating a capability as public or
product-facing.

This rule intentionally favors fewer, better-proven capabilities over broad
surface expansion.

## Immediate Architecture Program

### 1. Shared Dataset Preflight And Import Evidence

Create a format-neutral import/preflight/report boundary in the seismic runtime
before adding more format-specific public flows.

Target shape:

- `DatasetAdapter`
- `DatasetPreflightRequest`
- `DatasetPreflightResponse`
- `DatasetImportPlan`
- `ImportWarning`
- `ImportBlocker`
- adapter-specific `adapter_detail`

SEG-Y byte mappings, MDIO Zarr paths, portal quirks, strict/lenient reader
policy, source chunking, and source template names stay in adapter evidence.
Canonical descriptors expose subsurface meaning such as layout, stacking state,
organization, shape, sample axis, coordinate binding, fidelity, and storage
estimate.

### 2. TraceBoost Workflow Recipes And Reports

TraceBoost should become the reference workflow proof harness. Its recipes should
be typed JSON/TOML structures, not shell scripts.

The canonical report should be JSON. Markdown, HTML, Mermaid, or screenshots are
derived renderers.

Recipes should reference existing request payloads wherever possible:

- preflight/import plans
- processing pipeline specs
- processing preview/run/batch requests
- export requests
- assertion checks

Reports should link each recipe step to source fingerprints, request digests,
runtime job ids, inspectable plans, runtime events, artifacts, lineage checks,
assertions, and benchmark/timing summaries.

### 3. Public Fixture Manifests Instead Of Vendored Data

Real-data proof should use small manifests and optional tiny derived snapshots,
not large committed datasets.

Target location:

- `test_data/seismic/public-fixtures/`

Each manifest should identify source URI, adapter, subset, license note, expected
canonical metadata, expected warnings/blockers, and fetch policy.

### 4. Ophiolite Charts Public Boundary And Evidence

Finish the public SDK hardening before adding more chart families.

Priority:

- split launch and preview public types
- keep `@ophiolite/charts` root narrow
- keep Ophiolite contract decoding in `@ophiolite/charts/adapters/ophiolite`
- keep interaction profiles declarative
- add example-level visual regression for every launch chart family
- make benchmark fixtures explicit and reproducible

## Explicit Non-Goals

- no generic SaaS dashboard direction
- no generic charting DSL
- no broad plugin ABI before proof workflows exist
- no public workflow API that is just TraceBoost command names
- no canonical contracts that expose SEG-Y byte offsets or MDIO internal paths
- no bulk public seismic data committed to the repo
- no app-local recreation of canonical processing identity, runtime events, or
  lineage semantics

## First Useful Milestones

1. Land the architecture decision for evidence-backed workflow boundaries.
2. Add TraceBoost workflow recipe/report design docs.
3. Extract shared dataset preflight/import evidence from TraceBoost-heavy flows
   into runtime-owned operations.
4. Add one public fixture manifest for a Poseidon MDIO ROI and one SEG-Y fixture
   manifest.
5. Add `traceboost workflow validate/run/render-report` CLI design and then
   implementation.
6. Complete the Ophiolite Charts public model split and export-surface tests.
7. Add example-level visual regression and fixture-driven chart benchmarks.
