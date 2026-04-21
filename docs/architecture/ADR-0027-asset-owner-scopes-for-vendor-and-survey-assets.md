# ADR-0027: Asset Owner Scopes for Vendor and Survey Assets

## Status

Accepted

## Date

2026-04-17

## Context

Ophiolite's current project catalog binds every asset and asset collection to a `well_id` and
`wellbore_id`. That model works for canonical well assets such as logs, trajectories, tops, marker
sets, and well time-depth assets, but it is a poor fit for:

- survey-scoped seismic assets,
- survey-scoped interpretation assets such as imported horizons,
- project-scoped preserved source bundles,
- vendor-native objects that do not belong to a single well.

The current Petrel and OpendTect connector work exposed the mismatch directly. Phase-one Petrel
horizon-point exports and OpendTect interpretation objects can be preserved, but only through a
compatibility lane that routes them into a system-owned archive wellbore.

That compatibility lane is useful, but it is not the end state. It keeps provenance intact and
avoids pretending preserved source bundles belong to a user well, yet it still encodes non-well
assets through well ownership because the underlying schema requires it.

Modern ingestion systems generally separate:

- source discovery and planning,
- resource-specific ingestion configuration,
- execution and scheduling,
- destination object ownership and lifecycle state,
- source provenance and sync state.

This is the same general shape visible in current Databricks Lakeflow Connect and Snowflake Native
SDK for Connectors flows, where connectors manage resources or pipelines explicitly instead of
forcing every ingested object through one physical ownership model.

## Decision

Ophiolite will evolve from implicit wellbore-only ownership to explicit asset owner scopes.

The target owner scopes are:

1. `wellbore`
2. `survey`
3. `project`

The compatibility rule is:

- keep the current archive wellbore path for phase-one preserved vendor bundles,
- do not expand that compatibility path into the long-term owner model,
- migrate future survey and project assets onto first-class non-well owners.

The implementation rule is:

- provenance remains separate from owner scope,
- vendor object ids remain provenance, not owner ids,
- owner scope is a catalog concern, not a connector-specific concern.

## Design Direction

### Phase 1: Internal owner abstraction

Before any schema migration, Ophiolite should stop threading raw `well/wellbore` pairs directly
through new import code. Import paths should resolve an internal owner handle first and let asset
creation consume that handle.

This phase reduces migration blast radius without changing persisted schema yet.

### Phase 2: Catalog owner model

The project catalog should add a first-class owner abstraction for assets and collections. The
minimal target shape is:

- an owner scope enum,
- an owner id,
- owner metadata specific to the chosen scope,
- compatibility support for legacy well/wellbore-owned rows.

Possible physical designs include:

- adding nullable `survey_asset_id` and `project_scope_key` columns directly to `assets` and
  `asset_collections`, or
- introducing an `asset_owners` table that normalizes owner scope and owner identity.

Recommended answer: prefer a normalized owner table if multiple non-well scopes are expected to
grow. Prefer direct columns only if the scope set remains very small and stable.

Initial implementation status as of 2026-04-17:

- a normalized `asset_owners` registry is in place,
- `asset_collection_owners` can bind collections to `wellbore`, `survey`, or `project` owners,
- collection owner lookups fall back to inferred wellbore ownership for legacy rows,
- raw preserved bundles without binding register as `project` owned,
- seismic trace collections register as `survey` owned,
- Petrel horizon-point commits can append canonical horizons into an existing survey-owned seismic
  asset when `targetSurveyAssetId` is supplied,
- asset rows still retain legacy `well_id` and `wellbore_id` columns for compatibility.

### Phase 3: Survey and project canonicalization

Once owner scope is first-class:

- seismic trace stores should be survey-owned,
- imported horizons should be survey-owned,
- preserved raw vendor bundles with no well binding should be project-owned,
- well assets stay wellbore-owned.

### Phase 4: UI and API surfacing

CLI, backend DTOs, and application UIs should expose owner scope explicitly instead of inferring it
from wellbore attachment.

## Consequences

### Positive

- vendor connectors stop abusing well ownership for non-well artifacts,
- seismic and interpretation assets get a catalog model that matches their actual scope,
- provenance stays stable even when canonical ownership changes,
- future Petrel horizon, fault, body, and interpretation imports become straightforward,
- backend APIs can express ownership clearly for application and automation clients.

### Negative

- catalog migration work is unavoidable,
- several project queries currently assume wellbore ownership and will need to be generalized,
- some existing DTOs and inventory views will need owner-aware revisions.

## Near-Term Rules

Until asset rows fully migrate beyond legacy well ownership:

- raw-source-only vendor commits without binding should continue using the system-owned archive
  wellbore,
- canonical well assets should continue requiring explicit binding,
- new connector work should avoid introducing more fake ownership shortcuts,
- new inventory and API work should prefer explicit owner scope when available.

## References

- Databricks Lakeflow Connect ingestion overview:
  https://docs.databricks.com/aws/en/ingestion/
- Snowflake Native SDK for Connectors overview:
  https://docs.snowflake.com/en/developer-guide/native-apps/connector-sdk/about-connector-sdk
- Snowflake ingestion management overview:
  https://docs.snowflake.com/en/developer-guide/native-apps/connector-sdk/flow/ingestion-management/overview
