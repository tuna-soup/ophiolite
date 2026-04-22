# TraceBoost Import Manager Integration Plan

This document maps the unified import architecture from `ADR-0029` onto the current
`apps/traceboost-demo` frontend and Tauri shell.

## Current frontend situation

Current import routing is split across:

- menu event listeners in `src/App.svelte`
- file pickers in `src/lib/file-dialog.ts`
- runtime-store open/import logic in `src/lib/viewer-model.svelte.ts`
- provider-specific modal dialogs such as:
  - `SegyImportDialog.svelte`
  - `HorizonImportDialog.svelte`
  - `WellFolderImportDialog.svelte`
  - `WellTimeDepthImportDialog.svelte`
  - `VendorProjectImportDialog.svelte`

This works, but it leaves `App.svelte` acting as a manual import router and state owner.

## Target frontend shape

Recommended additions:

- one top-level `ImportManagerDialog.svelte`
- one app-scoped import coordinator module, likely `import-manager-model.svelte.ts`
- one provider registry descriptor list for UI metadata
- one normalized import review/result panel used across providers

Recommended retained pieces:

- provider-specific edit panes and review editors
- existing specialized dialogs can be refactored into provider pages instead of rewritten from
  scratch

## Menu convergence

Target native menu direction:

- `File -> Open Volume...`
- `File -> Import Data...`

Transitional compatibility:

- keep existing provider-specific menu items for now
- route them into `ImportManagerDialog` with the matching provider preselected

Examples:

- `menu:file-import-seismic` opens Import Manager with `provider_id = seismic_volume`
- `menu:file-import-horizons` opens Import Manager with `provider_id = horizons`
- `menu:file-import-well-sources` opens Import Manager with `provider_id = well_sources`

## Suggested frontend state split

Current `App.svelte` owns booleans such as:

- `segyImportDialogOpen`
- `horizonImportDialogOpen`
- `wellSourceImportDialogOpen`
- `wellTimeDepthImportDialogOpen`
- `vendorProjectImportDialogOpen`

Recommended replacement:

```ts
interface ImportManagerUiState {
  open: boolean;
  providerId: ImportProviderId | null;
  sessionId: string | null;
  sourceRefs: string[];
  pendingAction: "open" | "import" | "deep_link";
}
```

`App.svelte` should own only:

- whether the centralized import manager is visible
- which provider is preselected
- any initial source refs passed from menu or file picker

Provider-local editing state should move under the import manager model or the provider page.

## Open vs import behavior changes

`viewer-model.svelte.ts` currently lets `openVolumePath(...)` do all of these:

- open `.tbvol`
- reuse an existing imported store for an external source
- one-shot import direct formats
- preflight SEG-Y
- open geometry recovery UI when needed

Recommended target:

- `openManagedVolumePath(path)` only handles managed runtime artifacts such as `.tbvol`
- `requestImportFromSource(path)` routes external formats into Import Manager
- Import Manager owns external-source inspection, preview, validation, commit, and activation

This is the most important behavioral cleanup because it gives `Open Volume...` a stable meaning.

## Provider-page migration plan

### Phase 1

Wrap existing dialogs rather than rewriting them.

Recommended first move:

- `ImportManagerDialog` hosts provider selection and shared session header
- existing provider dialogs become embedded provider panels where feasible
- keep their internal domain editing behavior largely intact

### Phase 2

Extract shared review widgets:

- blockers
- warnings
- conflicts
- preserved sources
- canonical assets to be created
- result summary

This should reduce duplicated review logic across horizons, wells, and time-depth assets.

### Phase 3

Move provider-specific session orchestration out of `App.svelte` and into the manager model.

At that point, `App.svelte` should no longer know about per-provider draft booleans or pending
import payload fragments.

## Recommended initial rollout order

1. Introduce `ImportManagerDialog` and route existing provider-specific menu events into it.
2. Move seismic-volume import first because it currently overlaps most with `Open`.
3. Move horizons and well sources next because they already match the preview/draft/commit pattern.
4. Move time-depth asset imports next.
5. Fold vendor project import in after the base manager and normalized result panel stabilize.

## Shared UX pieces to standardize

The Import Manager should render these consistently for all providers:

- source summary
- destination summary
- preview status
- blockers
- warnings
- conflict choices
- canonical assets to be created or updated
- preserved sources
- explicit dropped items with reasons
- activation choice
- post-commit result summary

Provider pages should render only the domain-specific editing surface:

- SEG-Y geometry mapping and plan review
- horizon CRS and domain controls
- well identity, ASCII mapping, tops, and trajectory editing
- time-depth JSON preview and collection naming
- vendor project object selection and bridge choices

## Concrete codebase steps

Recommended first code changes after this design phase:

1. Add an `ImportManagerDialog.svelte` shell and open/preselect plumbing in `App.svelte`.
2. Add an `import-manager-model.svelte.ts` app-scoped session owner.
3. Replace direct external-format handling in `openVolumePath(...)` with reroute-to-import behavior.
4. Add backend Tauri commands for session start, provider list, and normalized result envelopes.
5. Adapt `SegyImportDialog.svelte`, `HorizonImportDialog.svelte`, and
   `WellFolderImportDialog.svelte` into provider pages or manager-hosted panes.

## Non-goals

This rollout does not require:

- deleting all existing provider dialogs immediately
- redesigning all provider-specific editing surfaces before the manager exists
- extracting the import manager into shared crates before the app-local abstraction proves out
