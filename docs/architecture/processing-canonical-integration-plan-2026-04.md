# Processing Canonical Integration Plan (2026-04)

## Status

Active implementation plan

## Purpose

This document turns the current processing architecture landing into an execution plan.

It is intentionally repo-specific. It assumes the current baseline is already in place:

- `ophiolite-seismic` owns the stable inspectable processing-plan and debug contract surfaces
- planner hints are backend-owned metadata next to operator definitions
- planner structure is organized as named passes with diagnostics snapshots
- runtime owns checkpoint, reuse, lineage, artifact identity, execution policy, and packaging mechanics
- persisted reuse carries explicit requirement/resolution metadata and is validated against canonical lineage and version context
- execution policy includes queue classes, admission control, reservation hints, exclusive scopes, cancellation-before-dispatch, and runtime snapshots
- artifact identity is modeled explicitly with canonical path-independent identity, artifact keys, logical domains, chunk-grid metadata, geometry fingerprints, materialization classes, reuse classes, and live-set metadata
- store and prestack lineage emit the same canonical semantic envelope in intent, but not yet through one fully unified implementation path
- reproducible derived-output packaging exists, but validation and reproducibility guarantees are not yet strong enough
- TraceBoost consumes the canonical debug model directly, including Explain, Why, Runtime, Lineage, and Sections

The goal of this phase is not another redesign. The goal is to freeze one canonical derivation path, harden compatibility policy, close correctness gaps, and only then reduce representation duplication.

## Locked Decisions

The current implementation plan is based on these fixed decisions:

- `ophiolite-seismic-runtime` is the canonical owner of artifact-key construction, geometry fingerprint recipes, lineage digesting, reuse comparison, and compatibility classification
- `ophiolite-seismic` owns the stable serialized contracts, not the canonical derivation logic
- `artifact_key` is the authoritative canonical artifact identity
- readable identity mirrors such as `logical_domain`, `chunk_grid_spec`, and `geometry_fingerprints` may remain denormalized for ergonomics, but they are validated mirrors rather than independent state
- compatibility remains explicit:
  - `Canonical` means readable and reusable
  - `NormalizedLegacyReadable` means readable, optionally rewritable, not silently reusable
  - `LegacyReadableNoCanonicalReuse` means readable only
- canonical identity changes invalidate by default through explicit semantics/version bumps
- cache validation tightens for every family in one cut
- persisted rewrite of legacy metadata is explicit; normalize-on-read is allowed for interpretation, not for silent reuse
- package validation becomes a strict canonicality check rather than a presence check
- package reproducibility targets deterministic package-tree contents first; archive/container reproducibility is a later concern
- runtime snapshots represent executed truth; the plan represents intended truth
- runtime-vs-plan divergence must be recorded structurally, not only as strings
- inspectable/app-facing plan models stay thin projections over runtime truth rather than becoming a second execution truth
- generated TS/schema artifacts are blocking contract outputs and must be enforced

## Current Gaps This Plan Closes

The main open problems are:

1. canonical identity is still computed by different recipes in planning, store lineage writing, prestack lineage writing, lineage rewriting, packaging, and desktop cache validation
2. family-aware cache validation is incomplete
3. package canonicality checks are too weak
4. app-side processing schema/version envelopes still have local hardcoded authority in a few paths
5. compatibility policy exists in code behavior, but not yet as one explicit validator/normalizer contract
6. runtime/debug surfaces do not yet record divergence between planned execution policy and executed runtime state as a structured first-class concept
7. runtime and inspectable plan representations remain too close to one another for long-term maintainability

## Workstreams

The implementation is organized into one serialized foundation wave and several follow-on workstreams.

### Workstream A: Canonical Foundation

This workstream defines the correctness boundary for everything else. It must be completed first.

#### Task A1: Extract canonical artifact identity helper surface

- Scope:
  - add one runtime-owned helper module for canonical artifact identity, geometry fingerprints, canonical lineage envelope construction, lineage digesting, and reuse comparison
  - keep semantic identity ownership in `identity.rs`
- Primary files:
  - `crates/ophiolite-seismic-runtime/src/identity.rs`
  - `crates/ophiolite-seismic-runtime/src/lib.rs`
  - new runtime helper module under `crates/ophiolite-seismic-runtime/src/`
- Owner:
  - runtime
- Depends on:
  - none
- Acceptance criteria:
  - there is one obvious canonical API for artifact identity and lineage construction
  - new code paths do not need to know store-specific or family-specific hashing recipes to construct canonical identity
  - the helper API returns canonical envelopes or validated fragments, not only low-level hash ingredients

#### Task A2: Route all canonical call sites through shared helpers

- Scope:
  - remove divergent canonical derivation logic from planning, store/prestack store lineage writing, lineage rewriting, package validation, and desktop cache validation
- Primary files:
  - [planner.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/planner.rs:1)
  - [store.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/store.rs:1)
  - [prestack_store.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/prestack_store.rs:1)
  - [processing_runtime.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_runtime.rs:1)
  - [processing_package.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_package.rs:1)
  - [processing_cache.rs](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src-tauri/src/processing_cache.rs:1)
- Owner:
  - runtime with desktop cache cooperation
- Depends on:
  - A1
- Acceptance criteria:
  - planner-emitted artifact identity matches stored lineage identity for the same produced output
  - `tbvol`, `tbvolc`, and `tbgath` canonical envelopes are built from shared helpers
  - cache validation for every family resolves through the same canonical comparison rules

#### Task A3: Fix planner diagnostics snapshot coherence

- Scope:
  - correct named-pass snapshot ids and ordering where they drift from the actual pass pipeline
- Primary files:
  - [planner.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/planner.rs:1)
- Owner:
  - runtime
- Depends on:
  - none
- Acceptance criteria:
  - planner diagnostics snapshots carry the correct pass ids
  - tests assert exact pass ordering and exact pass ids for the named-pass debug contract

#### Task A4: Tighten package canonicality validation

- Scope:
  - move package validation from presence checks to equality checks across redundant metadata copies
  - verify that package config, copied store metadata, and canonical lineage agree
- Primary files:
  - [processing_package.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_package.rs:1)
  - related metadata readers in runtime store code
- Owner:
  - runtime
- Depends on:
  - A1
- Acceptance criteria:
  - package open/validation fails on mismatched canonical mirrors
  - package validation distinguishes malformed canonical metadata from merely legacy-readable metadata
  - tamper tests cover canonical mismatch cases

#### Task A5: Remove app-local processing schema-version authority

- Scope:
  - eliminate hardcoded processing schema versions and synthetic local versioned envelopes in TraceBoost
- Primary files:
  - [bridge.ts](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src/lib/bridge.ts:1)
  - [processing-model.svelte.ts](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src/lib/processing-model.svelte.ts:1)
- Owner:
  - app and contracts
- Depends on:
  - none
- Acceptance criteria:
  - generated contract authority is the only source of processing schema versions in the app
  - cached app state may reuse prior responses, but it does not invent its own versioned debug envelope

#### Task A6: Add Phase 0 invariants test set

- Scope:
  - add the minimum integration coverage that freezes canonical behavior before cleanup continues
- Primary files:
  - runtime, execution, desktop, and packaging test modules
- Owner:
  - runtime, execution, desktop
- Depends on:
  - A2, A3, A4, A5
- Acceptance criteria:
  - exact planner pass ordering/pass-id tests exist
  - canonical identity equivalence tests span plan, stored lineage, cache lookup, and package metadata
  - cache invalidation tests cover every supported family
  - compatibility-state tests cover `Canonical`, `NormalizedLegacyReadable`, and `LegacyReadableNoCanonicalReuse`

### Workstream B: Compatibility Policy

This workstream starts after the canonical helper cut has stabilized enough to define one compatibility contract.

#### Task B1: Write the checked compatibility note

- Scope:
  - add a short in-tree note that defines readable/reusable/rewritable/reject behavior
- Primary files:
  - new doc under `docs/architecture/`
  - references from processing architecture docs as needed
- Owner:
  - runtime/contracts
- Depends on:
  - A1, A2
- Acceptance criteria:
  - engineers can answer compatibility questions from one repo-local document
  - the compatibility note matches executable behavior in tests

#### Task B2: Centralize validator/normalizer behavior

- Scope:
  - implement one validator/normalizer path used by runtime, packaging, and desktop cache lookup
- Primary files:
  - runtime canonical helper module
  - [processing_package.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_package.rs:1)
  - [processing_cache.rs](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src-tauri/src/processing_cache.rs:1)
- Owner:
  - runtime with desktop cache cooperation
- Depends on:
  - B1
- Acceptance criteria:
  - compatibility classification is not reimplemented independently in multiple subsystems
  - newly produced outputs fail closed on mismatch
  - legacy-readable artifacts can still be inspected according to the documented policy

### Workstream C: Runtime-vs-Plan Divergence Diagnostics

This workstream can proceed after Workstream A is in place.

#### Task C1: Add structured divergence diagnostics to backend models

- Scope:
  - represent runtime-vs-plan policy divergence as structured data rather than free-form text only
- Primary files:
  - [processing.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic/src/contracts/processing.rs:1)
  - [execution.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/execution.rs:1)
  - [lib.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-execution/src/lib.rs:1)
  - generated TraceBoost contract outputs
- Owner:
  - contracts, runtime, execution
- Depends on:
  - A6
- Acceptance criteria:
  - runtime snapshots can record divergence from planned queue class, reservation, exclusivity, or cancellation intent
  - mismatch state survives serialization to generated TS contracts
  - debug consumers do not need to parse strings to detect divergence

#### Task C2: Surface divergence and legacy-state distinctions in TraceBoost

- Scope:
  - show `legacy readable but not reusable` and runtime-vs-plan divergence explicitly in the processing debug UI
- Primary files:
  - [processing-model.svelte.ts](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src/lib/processing-model.svelte.ts:1)
  - [ProcessingDebugPanel.svelte](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src/lib/components/ProcessingDebugPanel.svelte:1)
- Owner:
  - app
- Depends on:
  - C1
- Acceptance criteria:
  - the UI distinguishes canonical reusable outputs from readable-only legacy outputs
  - divergence diagnostics render from structured fields rather than inferred strings

### Workstream D: Package Reproducibility Hardening

This workstream can proceed after package canonicality rules are stable.

#### Task D1: Normalize package output determinism

- Scope:
  - normalize deterministic package-tree metadata such as ordering and writer-controlled timestamps
- Primary files:
  - [processing_package.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_package.rs:1)
- Owner:
  - runtime
- Depends on:
  - A4
- Acceptance criteria:
  - repeated packaging of unchanged output produces byte-stable package-tree contents under supported environments
  - determinism requirements are tested on Windows, where file metadata handling tends to drift

#### Task D2: Add payload integrity verification where feasible

- Scope:
  - verify copied payload content hashes when the package format has enough information to do so cheaply
- Primary files:
  - [processing_package.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/processing_package.rs:1)
  - related package manifest/config structures
- Owner:
  - runtime
- Depends on:
  - D1
- Acceptance criteria:
  - package validation can detect payload tampering in addition to metadata mismatch
  - content verification rules are explicit and covered by tests

### Workstream E: Contract Export and App Tooling Hygiene

This workstream can run in parallel after the current canonical contract wave is stable enough for generated outputs to stop churning daily.

#### Task E1: Add generated-contract drift gates

- Scope:
  - keep the allowlist export model, but make stale generated output and missing intended public types visible in automation
- Primary files:
  - [traceboost-contracts-export/src/main.rs](/C:/Users/crooijmanss/dev/ophiolite/scripts/traceboost-contracts-export/src/main.rs:1)
  - TraceBoost generated contract package scripts/config
- Owner:
  - contracts/tooling
- Depends on:
  - A6
- Acceptance criteria:
  - CI fails if generated TS/schema artifacts are stale
  - local regeneration/check commands are cheap enough to run in normal development
  - allowlist drift is treated as a deliberate public-surface decision

#### Task E2: Clean up the processing debug panel implementation

- Scope:
  - remove obvious local implementation debt in the debug panel while preserving backend ownership of semantics
- Primary files:
  - [ProcessingDebugPanel.svelte](/C:/Users/crooijmanss/dev/ophiolite/apps/traceboost-demo/src/lib/components/ProcessingDebugPanel.svelte:1)
- Owner:
  - app
- Depends on:
  - C1 when touching divergence display, otherwise may start earlier
- Acceptance criteria:
  - keyed iteration and Svelte-native reactive containers are used where required
  - the panel remains a projection over canonical contract data rather than a semantic reimplementation

### Workstream F: Representation Dedupe

This is last on purpose. It should start only after Workstreams A through E stop moving the canonical rules.

#### Task F1: Thin the inspectable projection boundary

- Scope:
  - reduce drift risk between runtime execution structures and inspectable/public plan structures
- Primary files:
  - [inspectable_processing_plan.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic/src/contracts/inspectable_processing_plan.rs:1)
  - [execution.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/src/execution.rs:1)
  - [lib.rs](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-execution/src/lib.rs:1)
- Owner:
  - contracts, runtime, execution
- Depends on:
  - A6, C1, E1
- Acceptance criteria:
  - inspectable types are more obviously view/projection oriented
  - runtime retains internal freedom without making inspectable types a competing source of truth
  - existing generated TS type names and wire compatibility are preserved in the first dedupe wave

## Serialization And Parallelism

### Must Be Serialized

These changes define correctness boundaries and should not be split across competing implementations:

1. A1
2. A2
3. A4
4. A6
5. B1
6. B2

### Can Run In Parallel After Workstream A Stabilizes

These work items operate on top of the frozen canonical helper and compatibility boundary:

- C1 and D1 may run in parallel
- E1 may run in parallel once generated contracts are expected to settle
- A5 can happen during Workstream A because it removes app-local authority without changing canonical identity recipes
- C2 and E2 may run in parallel after backend contract fields exist

### Explicit Ordering

The preferred implementation order is:

1. A1
2. A2
3. A3
4. A4
5. A5
6. A6
7. B1
8. B2
9. C1
10. D1
11. E1
12. C2
13. E2
14. D2
15. F1

## Definition Of Done For Phase 0

Phase 0 is complete only when all of the following are true:

- every canonical identity call site uses the shared helper path
- family-aware cache validation is enabled for all supported families
- package validation checks deep canonical agreement rather than field presence
- app-local processing schema-version authority is gone
- planner pass snapshots are coherent and tested
- integration tests prove canonical identity equivalence across plan, stored lineage, cache lookup, and package metadata

No representation-deduping work should start before this definition of done is met.

## Suggested Issue Breakdown

The fastest path to execution is to create issues matching the task ids above.

Recommended first issue batch:

1. A1 and A2 as one ownership bundle for runtime canonical helper extraction and call-site migration
2. A3 as a small isolated correctness fix
3. A4 as package canonicality hardening
4. A5 as app/contract authority cleanup
5. A6 as the first invariant test lane

That issue batch is enough to start implementation without reopening design.

## Related Docs

- [ADR-0032: Processing Authority and Thin-Client Migration](./ADR-0032-processing-authority-and-thin-client-migration.md)
- [Processing Authority Matrix](./processing-authority-matrix.md)
