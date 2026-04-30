# ADR-0037: Evidence-Backed Workflow And Import Proof Boundaries

## Status

Accepted

## Context

Ophiolite already has the right high-level stack boundary:

```text
TraceBoost -> Ophiolite core + Ophiolite Charts
```

The remaining risk is not mainly that the architecture is pointed in the wrong
direction. The risk is that the system looks like a generic SDK, chart package,
and demo application instead of a defensible subsurface workflow platform.

The durable moat is evidence:

- canonical geoscience semantics
- real-data preflight/import behavior
- processing identity, lineage, and runtime events
- validation fixtures and public-data manifests
- benchmark evidence with provenance
- specialized chart families with visual and interaction regression coverage
- reproducible workflow recipes and run reports

## Decision

Ophiolite treats proof generation as part of architecture, not only marketing or
demo material.

The working rule is:

> A capability is not product-ready until it has the appropriate contract,
> runtime behavior, control surface, fixture, validation, benchmark/report, and
> chart/view proof for its role.

### Platform-Owned Evidence

`ophiolite` owns reusable evidence that makes sense without TraceBoost attached:

- canonical subsurface contracts and DTO meaning
- processing identity, artifact identity, runtime events, and lineage semantics
- format-neutral dataset preflight/import evidence
- import warnings, blockers, fingerprints, storage estimates, and canonical
  previews
- CLI/Python control surfaces over Rust-owned behavior
- reusable fixture manifests and benchmark policies

Format-specific details stay behind adapter evidence. SEG-Y byte locations,
sample format quirks, MDIO Zarr paths, source chunking, portal behavior, and
reader policy do not become canonical domain fields.

### TraceBoost-Owned Proof Harness

TraceBoost owns reference workflow composition:

- typed workflow recipes
- app-local orchestration and session activation
- file dialogs, grants, recent datasets, and desktop transport
- workflow run reports
- report renderers such as Markdown, HTML, Mermaid, and screenshots

TraceBoost reports may consume canonical Ophiolite processing evidence, but they
must not recreate canonical processing identity, runtime-event, or lineage
models locally.

### Ophiolite Charts-Owned Visual Evidence

Ophiolite Charts owns chart-family proof:

- stable launch chart models and wrapper props
- chart-native viewport, interaction, overlay, probe, and handle contracts
- explicit Ophiolite adapters
- visual regression baselines for public examples
- fixture-driven chart benchmarks

Charts does not own app workflow state, backend transport, data fetching, import
policy, or product dialogs.

## Implementation Shape

### Dataset Import Evidence

Add a format-neutral import/preflight/report layer before expanding public
dataset workflows.

Initial target shape:

```text
source ref
  -> adapter detection
  -> preflight evidence
  -> import plan
  -> validation warnings/blockers
  -> commit/materialization
  -> stable report
```

The shared model should include:

- `DatasetAdapter`
- `DatasetPreflightRequest`
- `DatasetPreflightResponse`
- `DatasetImportPlan`
- `ImportWarning`
- `ImportBlocker`
- stable report serialization for CLI, Python, TraceBoost, and tests

Adapter-specific detail may include existing SEG-Y import plans or MDIO subset
and template details. Canonical preview fields remain format-neutral.

### Workflow Recipes And Reports

TraceBoost recipes are typed app-level workflow descriptions. They should not be
raw shell pipelines.

Recipe records should include:

- `schema_version`
- `recipe_id`
- `name`
- `dataset_inputs`
- `steps`
- stable `step_id`
- `depends_on`
- request payloads using existing contracts where possible
- expected outputs and assertions

Workflow reports are append-only run evidence. The canonical report is JSON.
Renderers derive Markdown, HTML, Mermaid, or other display formats from that
JSON report.

Report records should include:

- `run_id`
- `recipe_id`
- `recipe_digest`
- started/completed timestamps
- app/runtime/contract versions
- environment summary
- source fingerprints
- request and response digests
- runtime job ids
- inspectable plans
- runtime events
- output artifacts and digests
- lineage/package compatibility results
- assertions
- benchmark/timing summaries where relevant

### Public Fixture Manifests

Public real-data validation uses manifests, not vendored bulk datasets.

Fixture manifests describe:

- fixture id
- source URI
- license/access note
- adapter id
- subset policy
- expected canonical preview
- expected warnings/blockers
- optional checksums or tiny derived snapshots

Bulk fetches must be opt-in and should not run as normal unit-test dependencies.

## Consequences

Accepted consequences:

- some work shifts from broad feature expansion to proof loops
- TraceBoost becomes more important as a reference workflow harness, but remains
  app-owned
- public import flows need stable evidence/report types instead of only
  app-shaped request/response DTOs
- chart launch families should harden before more preview families are promoted
- benchmark and visual-regression evidence become part of release readiness

Rejected shapes:

- no TraceBoost recipe or command name becomes the public platform API by
  accident
- no app-local reconstruction of canonical processing identity or lineage
- no generic charting DSL as the public Charts story
- no SEG-Y/MDIO/source-portal quirks in canonical contracts
- no committed large public seismic datasets

## Validation

This decision is working when:

- a real dataset can be preflighted with reusable evidence and clear
  warnings/blockers
- the same import evidence is consumable from CLI, Python, TraceBoost, and tests
- TraceBoost can run a typed recipe and produce a JSON report that links recipe
  steps to canonical runtime evidence and artifacts
- chart examples have visual regression coverage and benchmark fixtures
- public docs can point to executable proof instead of claims alone

## Follow-On Documents

- `docs/strategy/evidence-backed-subsurface-platform.md`
- `docs/architecture/traceboost-reference-workflow-proof-harness.md`
- `docs/architecture/ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`
- `charts/docs/public-sdk-roadmap.md`
- `docs/research/public-seismic-open-data-api-candidates-2026-04.md`
