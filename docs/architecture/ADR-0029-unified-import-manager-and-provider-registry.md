# ADR-0029: Unified Import Manager and Provider Registry

## Status

Accepted

## Context

TraceBoost already has substantial import capability, but it is organized as a set of parallel,
asset-specific flows instead of one integrated intake architecture.

Current examples in `apps/traceboost-demo`:

- `Open Volume...` and `Import -> Seismic Volume...` overlap semantically
- `App.svelte` owns multiple menu-event handlers and dialog booleans for separate import families
- `viewer-model.svelte.ts` mixes direct open, one-shot import, SEG-Y preflight, geometry recovery,
  and post-import activation behavior
- the backend already exposes multiple strong but uneven workflows:
  - SEG-Y: `scan -> validate plan -> import`
  - horizons: `preview -> suggested canonical draft -> commit`
  - well sources: `preview -> suggested canonical draft -> commit`
  - time-depth assets: `preview -> suggested canonical draft -> commit`

This means the repo already has domain-aware import logic, but the product shell does not yet have
one durable orchestration boundary.

Local inspection of QGIS shows a useful pattern:

- one centralized `QgsDataSourceManagerDialog`
- one `QgsSourceSelectProviderRegistry`
- one common provider-facing widget contract
- provider-specific payload semantics kept separate under the shared shell

That pattern fits TraceBoost better than either:

- leaving each import family as a bespoke vertical slice forever
- flattening all import families into one generic import DTO that erases domain meaning

## Decision

TraceBoost adopts a unified import architecture with:

- one centralized frontend `Import Manager`
- one app-local backend `ImportProviderRegistry`
- one shared external import lifecycle
- provider-specific typed preview, draft, validation, and commit payloads underneath that

The design rule is:

- genericize orchestration
- do not genericize canonical domain models

### Entry-surface rule

TraceBoost should converge toward two top-level data intake verbs:

- `Open Volume...`
- `Import Data...`

Semantics:

- `Open Volume...` opens existing Ophiolite-managed runtime artifacts such as `.tbvol`
- `Import Data...` is the centralized route for external source ingestion
- provider-specific menu items may remain temporarily, but they must deep-link into the centralized
  import manager with a preselected provider instead of launching bespoke top-level flows
- when a user selects an external file from an open-style surface, the app reroutes into import
  instead of silently materializing new managed data behind an `Open` verb

### Shared lifecycle rule

All providers present the same external lifecycle:

- `inspect`
- `preview`
- `suggested_draft`
- `validate`
- `commit`

Providers may implement internal shortcuts as long as the external lifecycle remains uniform.

Examples:

- SEG-Y may continue to use richer internal `scan` and validated-plan mechanics
- horizons and well imports may continue to use canonical draft generation directly from parsed
  source evidence

### Provider registry rule

The first registry lives in `apps/traceboost-demo/src-tauri`.

It is intentionally app-local in phase one. Shared extraction should wait until multiple providers
actually fit the same abstraction cleanly.

Expected provider families include:

- `seismic_volume`
- `horizons`
- `well_sources`
- `velocity_functions`
- `checkshot_vsp`
- `manual_time_depth_picks`
- `well_time_depth_authored_model`
- `well_time_depth_model`
- `vendor_project`

### Domain typing rule

Provider-specific canonical draft types remain typed and domain-owned.

Examples already present in the repo:

- `SegyImportPlan`
- `HorizonSourceImportCanonicalDraft`
- `ProjectWellSourceImportCanonicalDraft`
- `ProjectWellTimeDepthImportCanonicalDraft`

The shared import framework must not collapse these into one generic canonical draft shape.

### Canonical translation rule

Canonical translation is an explicit backend concept, not hidden app glue.

The backend is responsible for:

- parsing source evidence
- generating a suggested canonical draft
- validating what is canonically defensible
- enforcing canonical commit gates such as CRS, geometry, identity, and destination requirements

The frontend is responsible for:

- presenting preview and review state
- letting the user edit or confirm the draft
- choosing conflict and activation policies

### Partial commit rule

Import is not all-or-nothing by default.

The commit policy is:

- commit what is canonically defensible
- preserve the rest as source
- never silently manufacture missing meaning

Every committed canonical field must come from:

- parsed evidence
- explicit user confirmation
- explicit user edit

Unresolved content is classified explicitly as:

- `committed_canonical`
- `preserved_source_only`
- `dropped_with_explicit_reason`

### Strictness rule

Preview may proceed with incomplete information.

Commit must remain strict where canonical truth would otherwise be fabricated.

Examples:

- geometry-bearing canonical assets remain blocked when CRS is unresolved
- well imports remain blocked when well/wellbore identity is not confirmed
- seismic volume import remains blocked when destination or geometry mapping is unresolved

Source-only preservation may still be allowed when canonical commit is blocked, but only as an
explicit user-selected outcome.

### Recipe and session rule

TraceBoost distinguishes:

- session recovery
- durable recipes

Session recovery:

- stores in-progress draft work for one import session
- preserves review effort and partially edited canonical drafts

Recipes:

- are reusable provider-specific defaults
- apply as visible suggestions in preview
- never act as silent authority

Recipe scopes should include at least:

- `global`
- `project`
- `source_fingerprint`

Provider-specific recipe schemas remain separate.

### Conflict rule

Conflict handling becomes part of the import contract rather than a collection of ad hoc prompts.

The framework must support explicit policies such as:

- `create_new`
- `reuse_existing`
- `replace_existing`
- `merge_append`
- `merge_patch`
- `skip`

There is no hidden replacement.

Each provider defines safe defaults, but conflicts requiring user intent remain explicit.

### Normalized result rule

All providers return one normalized import result schema with provider-specific detail attached.

Minimum normalized result concepts:

- `provider_id`
- `session_id`
- `outcome`
- `previewable`
- `committable`
- `activatable`
- `canonical_assets[]`
- `preserved_sources[]`
- `dropped_items[]`
- `blockers[]`
- `warnings[]`
- `diagnostics[]`
- `activation_effects[]`
- `refresh_scopes[]`
- `provider_detail`

Recommended outcomes:

- `preview_only`
- `source_only_committed`
- `partial_canonical_commit`
- `canonical_commit`
- `commit_failed`

## Consequences

### Positive

- TraceBoost gets one coherent intake surface instead of a growing set of menu-specific flows
- backend orchestration becomes explicit without flattening provider semantics
- existing strong preview/draft/commit work for wells and horizons becomes the pattern rather than
  a special case
- SEG-Y import can keep its richer planning logic while still fitting one shared product shell
- canonical translation, source preservation, blockers, conflicts, and post-commit effects become
  platform-level behaviors
- import outcomes become reviewable and auditable instead of binary success/fail strings

### Tradeoffs

- the app-local Tauri layer becomes more opinionated before any shared-crate extraction
- the first pass adds one more abstraction layer on top of existing import commands
- old direct dialog paths will need a staged migration instead of immediate removal
- some existing commands may temporarily remain as compatibility adapters while the new manager
  takes ownership of orchestration

## Implementation Order

1. Introduce an app-local import provider registry and shared import session envelope in
   `apps/traceboost-demo/src-tauri`.
2. Add a centralized frontend Import Manager that routes provider-specific entry points through one
   shared shell.
3. Adapt existing import families to the shared external lifecycle without flattening their typed
   domain payloads.
4. Normalize blockers, conflicts, outcomes, preserved-source reporting, and refresh scopes.
5. Converge `Open Volume...` on managed-artifact open semantics and reroute external formats into
   the import manager.
6. Extend recipe/session recovery beyond SEG-Y once the shared shell is stable.
7. Extract only the proven shared parts into broader Ophiolite crates after at least two or three
   providers demonstrate a stable common abstraction.

## Non-goals

This ADR does not require:

- one generic import DTO that replaces provider-specific domain types
- immediate migration of all import logic into shared crates
- immediate removal of every existing provider-specific command surface
- relaxing canonical CRS, identity, or geometry boundaries
