# 15-Point Optimization Plan

Date: 2026-04-30

## Purpose

This document turns the current product direction into a concrete optimization
program for Ophiolite, Ophiolite Charts, and TraceBoost.

The goal is not to optimize only for speed or UI polish. The goal is to optimize
the whole stack for defensibility:

- real subsurface semantics
- real-data ingestion and validation
- reproducible workflow evidence
- benchmark-backed claims
- specialized visualization
- future agentic operation boundaries

The public positioning should become:

> Ophiolite is evidence-backed subsurface workflow infrastructure. Ophiolite
> Charts is the specialized visualization layer. TraceBoost is the reference
> workflow proof harness.

Related documents:

- `docs/development/proof-readiness-checklist.md`
- `docs/strategy/evidence-backed-subsurface-platform.md`
- `docs/architecture/ADR-0037-evidence-backed-workflow-and-import-proof-boundaries.md`
- `docs/architecture/traceboost-reference-workflow-proof-harness.md`
- `STACK-CONTEXT.md`
- `charts/docs/public-sdk-roadmap.md`

## How To Read This Plan

Each numbered topic carries an area-level status in the heading:

- **Not Started:** no meaningful repo work has landed yet.
- **Started:** direction, scaffolding, or partial implementation exists, but the
  area is not yet product-ready.
- **Finished:** the area has implementation, tests, docs, and public proof
  artifacts sufficient for the current product bar.

Each area is then split into four practical questions:

- **What is already fulfilled?** Capabilities that exist today or are mostly
  represented in the current repo.
- **What is partially fulfilled?** The direction exists, but the behavior is
  incomplete, scattered, app-local, or not yet reusable.
- **What is missing?** Structural gaps that prevent the area from being
  product-ready.
- **What should we focus on?** The recommended implementation focus.

So "near-term work" does not always mean "nothing exists today". In several
areas, the core idea already exists, but needs to be consolidated, exposed,
reported, tested, or made reusable.

## 1. Evidence-Backed Platform Core (Started)

### Objective

Make Ophiolite prove platform capabilities rather than merely expose APIs.

The core should answer:

- What is this subsurface asset?
- Where did it come from?
- What validation risks or blockers were found?
- What operation produced this result?
- Can the result be reproduced?
- Is this artifact compatible with current runtime expectations?

### What Is Already Fulfilled

Ophiolite already answers part of this today.

The repo has canonical contracts, asset/project concepts, package metadata,
provenance concepts, processing identity work, lineage/cache compatibility
policy, runtime debug contracts, and a clear Rust-first ownership model.
Existing architecture docs such as `ADR-0034` and `ADR-0037` already define the
right direction.

The platform also already has meaningful domain depth: LAS/log foundations,
typed project-managed assets, seismic runtime paths, processing operators,
contract generation, and chart-facing DTO boundaries.

The proof-readiness checklist now exists as a lightweight process artifact at
`docs/development/proof-readiness-checklist.md`.

### What Is Partially Fulfilled

The answers are not yet uniformly available as one reusable evidence chain.

For some asset families and processing paths, Ophiolite can answer what the
asset is, where it came from, and what produced a derived output. But this is
not yet consistently surfaced across CLI, Python, TraceBoost, tests, and public
docs.

Some validation and preflight behavior exists, but too much of the practical
workflow evidence is still app-local or path-specific.

### What Is Missing

The missing piece is not a new grand architecture. The missing piece is applying
the product-readiness discipline consistently:

- every capability needs a proof path
- every risky workflow needs warnings and blockers
- every derived output needs reportable identity and lineage
- every public claim needs executable evidence

The shared template exists, but it still needs to be attached to real capability
designs and used as a gate.

### What Should We Focus On

Add proof-readiness as a required design step using
`docs/development/proof-readiness-checklist.md`.

For every new platform capability, explicitly decide:

- canonical contract or app-local DTO
- runtime owner
- CLI/Python/app control surface
- fixture or public-data manifest
- validation warnings and blockers
- report shape
- benchmark need
- chart/view need

### Near-Term Work

- **Done:** add the proof-readiness checklist.
- Attach the proof-readiness checklist to capability design notes.
- Update implementation plans to call out fixture, report, and validation paths.
- Keep `docs/architecture/ADR-0037-evidence-backed-workflow-and-import-proof-boundaries.md` as the default product-readiness reference.
- Start using "proof-ready" as a gate before calling any capability public or product-facing.

### Validation

This area is improving when public docs can link to executable evidence rather
than claims alone.

## 2. Shared Dataset Preflight And Import Evidence (Started)

### Objective

Create a format-neutral preflight/import layer that works across SEG-Y, MDIO,
and future seismic data sources.

The layer should produce reusable evidence:

- source fingerprint
- adapter identity
- source kind
- geometry evidence
- coordinate warnings
- storage estimate
- canonical dataset preview
- warnings and blockers
- adapter-specific detail

### What Is Already Fulfilled

The repo already has working pieces for dataset import and inspection.

TraceBoost has SEG-Y preflight/import flows. Ophiolite has seismic runtime and
IO paths. The MDIO research and runtime code already show that the project can
read MDIO-style metadata and map it toward runtime concepts.

The architecture already recognizes that raw source quirks should not become
canonical domain meaning.

### What Is Partially Fulfilled

Preflight evidence exists, but it is not yet a shared platform surface.

The current import/preflight model is still too tied to specific app flows,
specific request/response DTOs, or individual format assumptions. SEG-Y plans,
MDIO source structure, CRS warnings, geometry evidence, and storage estimates
are not yet unified behind a reusable `DatasetPreflight` concept.

### What Is Missing

The missing piece is a format-neutral evidence wrapper.

Ophiolite needs a shared model that can say:

- this source looks like SEG-Y, MDIO, or another supported source
- this is the canonical dataset preview
- these are adapter-specific details
- these warnings are recoverable
- these blockers prevent import
- this is the storage/runtime plan

### What Should We Focus On

Design and implement the shared preflight/import/report boundary first, then
move format-specific logic behind adapters.

Do not expose SEG-Y byte locations, MDIO Zarr paths, template names, or portal
quirks as canonical domain fields. Keep them as evidence.

### Near-Term Work

- **Done:** document the shared preflight evidence model in
  `docs/architecture/shared-dataset-preflight-evidence-model.md`.
- Design `DatasetPreflightRequest`, `DatasetPreflightResponse`, and `DatasetImportPlan`.
- Extract reusable warning/blocker logic from TraceBoost-heavy flows into runtime-owned operations.
- Add stable serialized report output for CLI, Python, TraceBoost, and tests.
- Keep `ingest_volume()` as a convenience path, but back it with explicit `preflight -> plan -> validate -> commit` behavior.

### Validation

Run the same preflight through CLI, TraceBoost, and tests and verify that the
same warnings, blockers, fingerprints, and canonical preview appear.

## 3. Real/Public Data Fixture Manifests (Started)

### Objective

Build a durable real-data validation surface without committing huge public
datasets into the repository.

### What Is Already Fulfilled

The repo already contains synthetic fixtures and research notes about public
seismic data candidates.

There is already a useful shortlist of public/open sources, including Poseidon
MDIO, F3-style datasets, NLOG, NZP&M, SODIR/Diskos, and NOPIMS-style sources.

The first fixture-manifest scaffold now exists under
`test_data/seismic/public-fixtures/`, with placeholder Poseidon MDIO and F3
SEG-Y manifests plus a structural validator.

### What Is Partially Fulfilled

The research exists, and the manifest structure is now wired into a basic
validator, but it is not yet wired into real preflight/import proof workflows.

The project can talk about public data candidates, but cannot yet consistently
run a manifest-driven workflow that fetches or references a subset, preflights
it, imports it, validates expected warnings, and writes a report.

### What Is Missing

The missing pieces are authoritative manifests and runtime integration.

The repo now has placeholder manifests, but the actual dataset subsets,
expected canonical previews, expected warnings, checksums, and fetch behavior
still need domain review.

### What Should We Focus On

Start with a small number of high-value public-data fixtures:

- one Poseidon MDIO ROI
- one familiar SEG-Y/F3-style case
- one metadata-only discovery case from a public portal

Do not try to cover every public dataset source at once.

### Near-Term Work

- **Done:** add `test_data/seismic/public-fixtures/`.
- **Done:** add placeholder Poseidon MDIO and F3 SEG-Y manifests.
- **Done:** add a structural manifest validator.
- Replace placeholder manifests with reviewed fixture subsets.
- Add expected warnings such as unresolved CRS, sparse geometry, or source chunking mismatch after domain review.
- Add a fetch-gated fixture check command or test.

### Validation

Public fixture checks should not depend on large downloads by default. They
should be opt-in and should clearly report skipped, fetched, validated, and
failed states.

## 4. TraceBoost Reference Workflow Harness (Started)

### Objective

Reposition TraceBoost from a demo consumer into the reference workflow proof
harness for the stack.

TraceBoost should show complete workflows:

- preflight
- import
- open
- inspect
- process
- render
- export
- report

### What Is Already Fulfilled

TraceBoost already has a real app shape.

It can preflight/import SEG-Y, open runtime stores, load sections, run processing
preview/materialization paths, resolve maps, and render Ophiolite Charts. It is
already the best in-repo proof that the platform can support a real workflow.

### What Is Partially Fulfilled

TraceBoost proves workflows interactively, and now has initial recipe/report
schema modules plus a stub workflow runner, but it does not yet execute real
domain operations as reproducible proof artifacts.

The app can perform actions, recipe/report data structures exist, runner/CLI
scaffolding exists, Markdown/Mermaid renderers exist, and a placeholder golden
recipe exists. The missing part is real dispatch from recipe steps into
TraceBoost workflow operations.

The first golden workflow direction is also decided: technical
buyer/developer-evaluator audience, F3-style SEG-Y or existing small post-stack
fixture first, `preflight -> import -> open -> section view -> AGC preview ->
materialized output -> report`, AGC RMS as the first real operator, and
engineering correctness rather than interpretation correctness as the first
claim level.

### What Is Missing

TraceBoost still needs first-class workflow execution.

Without real step dispatch and real evidence collection, TraceBoost remains a
strong app demo plus proof-harness scaffolding, not yet a repeatable proof
harness.

### What Should We Focus On

Make TraceBoost workflow execution scriptable and reportable while keeping it
app-owned.

TraceBoost should compose platform and chart capabilities into user-facing
workflows, but it should not own canonical subsurface meaning, processing
identity, runtime events, or chart rendering internals.

### Near-Term Work

- **Done:** add workflow recipe and report schema modules under `traceboost/app/traceboost-app`.
- **Done:** add stub workflow runner, CLI scaffolding, Markdown/Mermaid report renderers, and first placeholder golden recipe.
- Add a small set of golden workflow recipes.
- Replace stub step execution with real dispatch for the first post-stack workflow.
- Keep browser endpoints and Tauri commands thin over the same app workflow service.

### Validation

A new user should be able to run one recipe and get a report that explains the
dataset, operations, warnings, outputs, and timings.

## 5. Workflow Recipes (Started)

### Objective

Define typed workflow recipes that can be run by TraceBoost, inspected by
developers, and eventually proposed or modified by agents.

### What Is Already Fulfilled

TraceBoost already has workflow actions and some app-local recipe-like behavior,
especially around import and demo preparation.

Existing request/response payloads already provide pieces that recipes should
reuse instead of inventing a second workflow language.

### What Is Partially Fulfilled

The workflow exists as code and UI behavior, and a portable recipe schema now
exists.

The schema has stable ids, step dependencies, dataset refs, JSON request
payloads, expected outputs, assertions, schema versioning, and validation tests.
It now has one placeholder golden recipe for the decided post-stack AGC RMS
workflow. That recipe is not yet authoritative because fixture paths and
expected evidence are placeholders.

The first artifact bundle target is clear: `recipe.json`, `report.json`,
`report.md`, a chart screenshot, and timing notes.

### What Is Missing

Authoritative golden recipes are missing.

The schema and placeholder recipe exist, but the project still needs
domain-reviewed recipe files that point at reviewed fixtures and use real
request payloads.

### What Should We Focus On

Start with a small schema that covers the workflows TraceBoost already performs.

Recipe fields should include:

- `schema_version`
- `recipe_id`
- `name`
- `description`
- `dataset_inputs`
- `steps`
- `depends_on`
- request payloads
- expected outputs
- assertions

### Near-Term Work

- **Done:** define the first recipe schema in a TraceBoost-owned module.
- **Done:** add schema versioning and validation tests.
- **Done:** add the first placeholder post-stack AGC RMS golden recipe.
- Extend or refine step kinds as needed for:
  - `preflight`
  - `import`
  - `open`
  - `processing_preview`
  - `processing_run`
  - `export`
  - `assert`
- Replace placeholder request payloads with real request payloads for the first
  configured fixture.
- **Done:** add recipe loading and CLI validation.

### Validation

Invalid recipes should fail before running. Valid recipes should produce stable
step ids, request digests, and report records.

## 6. Workflow Run Reports (Started)

### Objective

Make workflow evidence inspectable, reproducible, and renderable.

### What Is Already Fulfilled

Ophiolite already has canonical processing identity, runtime events, lineage,
and debug concepts. TraceBoost already receives enough workflow responses to
show useful user-facing state.

The ingredients exist.

### What Is Partially Fulfilled

The ingredients are now partially assembled into a durable report schema.

The JSON report data model exists with run ids, recipe ids, status, versions,
environment, source fingerprints, step records, warnings/blockers, runtime
evidence placeholders, artifacts, assertions, timings, and validation tests.
The stub runner now populates synthetic reports from recipe structure. It does
not yet populate reports from real preflight/import/process execution.

### What Is Missing

TraceBoost needs real report production.

The JSON report model, stub report production, and Markdown/Mermaid renderers
exist. TraceBoost still needs code that collects evidence from real workflow
execution. HTML and screenshots should remain later derived renderers.

### What Should We Focus On

Build the JSON report first and keep renderers secondary.

Reports should record:

- run id
- recipe id
- recipe digest
- timestamps
- app/runtime/contract versions
- environment summary
- source fingerprints
- request and response digests
- warnings and blockers
- runtime job ids
- inspectable processing plans
- runtime events
- output artifacts
- lineage checks
- assertions
- timings and benchmark summaries

### Near-Term Work

- **Done:** add a `workflow_report` module in TraceBoost.
- **Done:** add JSON-serializable report structs and validation tests.
- **Done:** add stub JSON report writing through `traceboost-app workflow run`.
- **Done:** add Markdown and Mermaid renderers.
- Link every processing step to canonical Ophiolite runtime evidence.

### Validation

Reports should be append-only run evidence. They should be stable enough to
compare across runs and useful enough to debug failures without opening the app.

## 7. Processing Identity, Lineage, And Runtime Events (Started)

### Objective

Keep hardening the canonical answer to:

- What produced this artifact?
- Can it be reused?
- Is it compatible?
- What actually happened at runtime?

### What Is Already Fulfilled

This is one of the stronger areas of the current architecture.

The repo already has `ADR-0034`, processing identity concepts, lineage/cache
compatibility policy, inspectable plan/debug direction, and shared runtime event
thinking. The right ownership boundary is already established: Ophiolite owns
canonical processing identity and TraceBoost consumes it.

### What Is Partially Fulfilled

The canonical model exists, but not every workflow/report surface fully consumes
it yet.

Some paths may still expose enough information for the app to work without
fully tying every result back to canonical evidence in a report.

### What Is Missing

The missing piece is end-to-end propagation into proof artifacts.

Processing identity should appear consistently in runtime metadata, package
lineage, cache checks, debug views, TraceBoost reports, and generated contracts.

### What Should We Focus On

Do not reinvent processing identity for recipes or reports. Connect reports to
the existing canonical evidence.

### Near-Term Work

- Keep planner-produced artifact identity authoritative.
- Ensure reports link to existing inspectable plans and runtime events.
- Expand compatibility checks only in shared runtime code.
- Keep generated TypeScript contracts aligned with Rust-owned meaning.

### Validation

The same artifact should have the same identity across planner, runtime, cache,
package, debug view, and TraceBoost report.

## 8. Operator Catalog And Narrow Operator Playbook (Started)

### Objective

Avoid a broad, shallow operator list. Build a small operator playbook that is
well explained, validated, benchmarked, and visible in workflows.

### What Is Already Fulfilled

The repo already has a narrow live operator family and architecture around the
operator catalog.

Existing live trace-local operators include amplitude scaling, RMS normalize,
AGC, phase rotation, filters, and same-geometry volume arithmetic. The
architecture already avoids forcing every seismic operation into one generic
operator bucket.

### What Is Partially Fulfilled

The operator implementation is ahead of the public/operator-playbook story.

Operators exist, but not all of them have the full product evidence package:
domain explanation, recipe example, numerical validation, benchmark case, chart
view, and report integration.

### What Is Missing

A small public/operator-facing playbook is missing.

Each public or product-facing operator should have:

- operator family
- domain assumptions
- parameter contract
- validation rules
- numerical tests
- benchmark cases
- workflow examples
- chart/report integration

### What Should We Focus On

Choose a small first playbook around existing trace-local operators and make
those excellent before adding more breadth.

### Near-Term Work

- **Done:** choose AGC RMS as the first real golden-workflow operator, with
  `amplitude_scalar` as a trivial baseline.
- Build the first operator playbook around that narrow post-stack path before
  broadening the public operator set.
- For each operator, document when to use it and when not to use it.
- Link operator runs into workflow reports.
- Add benchmark cases before making performance claims.

### Validation

An operator is ready when it can be run through a recipe, validated numerically,
benchmarked under known conditions, and explained in a report.

## 9. Ophiolite Charts Public SDK Hardening (Started)

### Objective

Make Ophiolite Charts feel like a focused commercial SDK for subsurface charts,
not an internal workspace exposed by accident.

### What Is Already Fulfilled

Ophiolite Charts already has the right product direction documented.

The launch families are identified:

- seismic section
- seismic gather
- survey map
- well correlation panel
- rock physics crossplot

The public SDK roadmap already calls for neutral public models, explicit
Ophiolite adapters, narrow root exports, examples, and benchmark methodology.

### What Is Partially Fulfilled

The current package surface is more hardened than before, but not fully done.

The public entrypoint test now guards against preview/extras component leaks,
wildcard root exports, wildcard package subpaths, and adapter contract-barrel
leaks. The adapter path no longer re-exports `../contracts`.

Some public types may still mix launch, preview, Ophiolite-specific, debug, and
internal concepts through the broader type surface. The launch surface is
better protected from drift, but the public type split is not complete.

### What Is Missing

Charts still needs a clean public type split.

The public root should teach only stable launch wrappers, launch model types,
props, handles, and explicit adapter entry points.

### What Should We Focus On

Harden the launch surface before adding more chart families.

### Near-Term Work

- Split launch-family public types from preview/internal types.
- **Done:** strengthen export-surface tests.
- **Done:** remove raw contract barrel re-export from `@ophiolite/charts/adapters/ophiolite`.
- Keep `@ophiolite/charts` root exports narrow.
- Keep Ophiolite-specific DTO decoding in `@ophiolite/charts/adapters/ophiolite`.
- Add simple and production examples for every launch family.

### Validation

Public examples should import only the intended public package paths and should
typecheck without internal package knowledge.

## 10. Chart Visual Regression And Benchmarks (Started)

### Objective

Prove chart behavior visually and performance-wise.

### What Is Already Fulfilled

There is already benchmark and visual-test thinking in the Charts workspace.

The repo has a benchmark app, public docs/playground surfaces, and a documented
benchmark methodology direction. The architecture recognizes that chart claims
need evidence.

### What Is Partially Fulfilled

Coverage is not yet complete enough for public confidence.

Static screenshots and smoke benchmarks are useful, but they are not enough to
prove chart-family behavior under realistic interactions. Public examples need
interaction-level visual regression, and benchmarks need fixed fixtures and
recorded conditions.

### What Is Missing

Example-level visual regression and fixture-driven benchmarks are missing for
the full launch set.

### What Should We Focus On

Make the launch chart families boringly reliable before expanding the chart
surface.

### Near-Term Work

- Add Playwright coverage for every launch chart example.
- Capture interaction states such as probe, pan/zoom, selection, overlays, and viewport changes.
- Make chart benchmark fixtures explicit.
- Record renderer mode, browser, machine metadata, dataset shape, viewport action, repetition policy, and raw results.

### Validation

No public chart claim should rely on a one-off screenshot or timing run. It
should point to repeatable visual and benchmark evidence.

## 11. Agent-Ready Operation Boundaries (Started)

### Objective

Prepare the stack for useful agentic operations without letting agents guess or
mutate state unsafely.

### What Is Already Fulfilled

The underlying direction is already agent-friendly.

Ophiolite has typed contracts, Rust-owned behavior, operation/catalog thinking,
validation concepts, generated frontend contracts, CLI/Python control surfaces,
and app boundaries. These are the right ingredients for constrained agentic
operation.

### What Is Partially Fulfilled

The project is not yet agent-operable as a product surface.

Operations are not consistently exposed as discoverable, typed, preflightable,
reportable actions with clear warnings, blockers, dry-run modes, and permission
boundaries.

The new TraceBoost recipe/report scaffolding is a useful first rail because it
turns workflow intent and workflow evidence into explicit artifacts. It is not
yet an agent-facing operation environment.

### What Is Missing

The missing piece is not "add a chatbot". The missing piece is an operation
environment where an agent can inspect available actions and constraints before
acting.

### What Should We Focus On

Build the rails first.

Useful agent tasks later:

- suggest a workflow recipe
- run preflight
- explain warnings
- choose a safe subset
- compare reports
- summarize benchmark evidence
- recommend next validated operations

### Near-Term Work

- Make operation catalogs explicit.
- Add warnings/blockers to every risky operation.
- Ensure operations have dry-run or preflight modes where needed.
- Make reports readable enough for agents and humans.

### Validation

An agent should be able to inspect available operations and constraints before
running anything destructive or expensive.

## 12. CLI And Python Thin Control Surfaces (Started)

### Objective

Expose platform behavior through CLI and Python without creating parallel
implementations.

### What Is Already Fulfilled

The architecture already has the right control-surface philosophy.

Ophiolite has CLI and Python automation surfaces, and the stack context already
says CLI, Python, and desktop commands should be thin control panels over
Rust-owned behavior.

### What Is Partially Fulfilled

Not every workflow has the right CLI/Python shape yet.

Some platform-level operations are not yet exposed through stable shared
commands, while some app-level flows live primarily inside TraceBoost. The
boundary is understood, but the command catalog is incomplete.

### What Is Missing

Shared dataset preflight/import evidence needs platform commands. TraceBoost
workflow recipes/reports need app commands.

TraceBoost now has initial workflow CLI scaffolding for recipe validation,
stubbed report generation, and report rendering. Ophiolite still needs the
shared platform preflight/import command surface.

### What Should We Focus On

Keep ownership separate:

- Ophiolite CLI/Python for reusable platform behavior
- TraceBoost CLI for app workflow recipes and reports

### Near-Term Work

- Add Ophiolite CLI commands for shared dataset preflight/import evidence.
- **Done:** add initial TraceBoost CLI commands for workflow recipe/report
  execution scaffolding.
- Replace stub TraceBoost CLI execution with real workflow dispatch.
- Keep Python wrappers focused on automation over platform operations.
- Validate command catalogs against ownership boundaries.

### Validation

The same operation should return the same evidence whether called from CLI,
Python, TraceBoost, or tests.

## 13. Public Documentation And Messaging (Started)

### Objective

Align public messaging with the actual moat.

The story should not be:

> SDK plus charts plus demo app.

The story should be:

> Evidence-backed subsurface workflow infrastructure with specialized charts and
> a reference workflow proof harness.

### What Is Already Fulfilled

The internal architecture docs now state the right direction.

The README, stack context, new strategy note, ADR-0037, TraceBoost proof-harness
design, and Charts roadmap now describe the stack as platform, charts, and
proof-harness layers.

### What Is Partially Fulfilled

The public-facing product story still needs executable artifacts to point to.

Messaging should not get ahead of implementation. The story becomes credible
when there are recipes, reports, charts, fixtures, and benchmark artifacts that
support it.

### What Is Missing

A public proof narrative is missing:

- proof artifacts page
- golden workflow examples
- report examples
- chart examples
- benchmark results with conditions
- real/public-data fixture documentation

### What Should We Focus On

Wait to broadly update public-facing claims until the first proof workflows
exist, then lead with those artifacts.

### Near-Term Work

- Update top-level docs after the first proof workflows land.
- Add a "proof artifacts" page.
- Link public examples to recipes, reports, charts, and benchmark evidence.
- Avoid generic SaaS/productivity language.

### Validation

A technical evaluator should understand what is defensible about the stack in
five minutes: domain semantics, data evidence, workflow reports, benchmarks, and
specialized charts.

## 14. Open-Data Discovery And Dataset Qualification (Started)

### Objective

Use public/open datasets to build validation depth and future agent evaluation
substrate.

### What Is Already Fulfilled

The repo already has useful public-data research.

The shortlist includes several realistic sources and candid notes about their
limits: public metadata, portal friction, download constraints, MDIO vs SEG-Y
shape differences, authentication needs, and subset feasibility.

### What Is Partially Fulfilled

Discovery is ahead of qualification.

The repo can identify candidate sources, but not yet consistently turn them
into executable fixture manifests and repeatable workflows.

Placeholder Poseidon MDIO and F3 SEG-Y manifests now exist as structural
starting points. They are not yet qualified public datasets because paths,
checksums, expected previews, warnings, and fetch behavior still need domain
review.

### What Is Missing

Dataset qualification criteria need to become operational.

Datasets should be ranked by:

- access feasibility
- license clarity
- machine-readable metadata
- format relevance
- workflow relevance
- subset support
- validation value

### What Should We Focus On

Do not chase every dataset. Pick datasets that can become repeatable proof
workflows.

### Near-Term Work

- Continue the public seismic source shortlist.
- Prioritize Poseidon MDIO for chunked ROI workflows.
- Keep F3-style SEG-Y as a familiar baseline.
- Track NLOG, NZP&M, SODIR, and NOPIMS as discovery/metadata candidates.
- Add fixture manifests as sources become practical.

### Validation

A dataset is useful when it can support a repeatable preflight/import/report
workflow, not merely when it exists publicly.

## 15. Architecture Boundary Enforcement (Started)

### Objective

Keep the stack scalable by enforcing ownership boundaries as features expand.

### What Is Already Fulfilled

The main boundaries are now clear and documented.

Ophiolite owns canonical meaning and runtime behavior. Ophiolite Charts owns
visualization. TraceBoost owns workflow composition and app-local behavior.
Recent docs now explicitly add proof artifacts, recipes, reports, and visual
evidence to that boundary model.

The first boundary enforcement checks now exist for Charts public entrypoints
and public fixture manifests.

### What Is Partially Fulfilled

The boundary model is documented, but enforcement still depends heavily on
discipline.

There are manifest checks, command-boundary checks, and chart public-entrypoint
checks, but not every future drift mode has a test or automated guard.

### What Is Missing

More boundary enforcement should be automated where practical.

Risk areas:

- app DTOs recreating canonical meaning
- platform contracts encoding TraceBoost-only workflow choices
- chart APIs exposing backend transport or app session state
- command names drifting from ownership boundaries
- preview chart types leaking into public launch exports

### What Should We Focus On

Use this ownership rule:

- Ophiolite owns canonical meaning, reusable runtime behavior, import evidence,
  processing identity, lineage, and platform automation.
- Ophiolite Charts owns reusable visualization behavior, public chart models,
  chart interactions, adapters, and visual evidence.
- TraceBoost owns app workflow composition, recipes, reports, session behavior,
  desktop transport, and product presets.

### Near-Term Work

- Keep boundary docs current when new workflow/report/import work lands.
- **Done:** add fixture manifest validation.
- **Done:** add stronger Charts public entrypoint guardrails.
- Add more tests or manifest checks where boundaries can drift.
- Reject app-local DTOs that recreate canonical meaning.
- Reject platform contracts that encode TraceBoost-only workflow choices.
- Reject chart APIs that expose backend transport or app session state.

### Validation

When a new feature is proposed, the team should be able to place it quickly:

- canonical platform capability
- chart-family capability
- app workflow capability
- future/deferred ecosystem concern

If placement is ambiguous, write the boundary decision before implementing the
feature.
