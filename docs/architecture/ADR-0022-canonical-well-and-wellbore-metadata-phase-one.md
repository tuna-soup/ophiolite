# ADR-0022: Canonical Well and Wellbore Metadata Phase One

## Status

Accepted

## Decision

`ophiolite` will keep the current project/catalog backbone:

- `WellRecord`
- `WellboreRecord`
- `AssetCollectionRecord`
- `AssetRecord`

and extend it additively rather than introducing a new parallel well model.

Phase one adds:

- canonical `WellMetadata` on `WellRecord`
- canonical `WellboreMetadata` on `WellboreRecord`
- a wellbore-level `definitive_trajectory_asset_id`

The existing asset hierarchy remains unchanged. Trajectory stays a wellbore-linked asset. Logs stay log assets. Tops stay a narrow structured asset family for now.

## Why

The current model is strong enough for local project organization, but too thin for real well data onboarding:

- `WellRecord` currently carries only name and a thin identifier set
- `WellboreRecord` currently carries only identity plus geometry and active well-model selection
- LAS headers preserve useful metadata, but that metadata does not have a durable project-level home
- trajectory workflows need one canonical wellbore-level pointer to the preferred survey
- marker semantics are broader than the current `TopSet` shape

We want to move toward OSDU-like captured scope without importing OSDU's full enterprise envelope.

## Consequences

- well and wellbore master metadata become first-class catalog fields
- project APIs can return richer well records without changing the asset hierarchy
- the preferred survey for a wellbore becomes explicit and durable
- LAS import can seed the first metadata values without overwriting curated metadata later
- follow-on marker and trajectory work can attach to these same records instead of inventing new side structures

## Exact Boundary

### 1. Well metadata

`WellRecord` remains the well identity object and gains optional `WellMetadata`.

Phase one metadata includes:

- field / basin / block style descriptive fields
- textual location context
- province/state/country style fields
- operator history
- optional structured surface location
- vertical measurement support
- external source references
- notes

This is intentionally lighter than OSDU:

- no OSDU resource envelope
- no ACL/legal/frame metadata
- no mandatory reference-data indirection for every enum-like field

### 2. Wellbore metadata

`WellboreRecord` remains the owner of canonical wellbore placement metadata and gains optional `WellboreMetadata`.

Phase one metadata includes:

- sequence / status / purpose / trajectory-type style fields
- parent wellbore linkage
- service-company and textual location context
- optional bottom-hole location
- vertical measurement support
- external source references
- notes

`WellboreGeometry` remains the owner of authoritative placement/anchor/CRS/azimuth context.

### 3. Definitive trajectory

Trajectory remains a separate asset family.

`WellboreRecord.definitive_trajectory_asset_id` identifies the preferred trajectory asset for downstream resolved geometry workflows.

Rules:

- the referenced asset must be a trajectory asset
- the referenced asset must belong to the same wellbore
- when present, resolved wellbore trajectory queries should prefer that asset over merging all current trajectory assets
- when absent, existing merge behavior may remain as a fallback

### 4. LAS seeding

LAS import may seed well/wellbore metadata on first creation from available header fields such as:

- field
- location text
- province
- operator/company
- service company

Rules:

- seeding happens only when the well or wellbore is first created
- later curated metadata is not overwritten by subsequent LAS imports
- LAS header content remains preserved in package metadata independently of project-level metadata

### 5. Marker follow-on

This ADR does not redefine `TopSet` into the final canonical marker model.

Follow-on work should introduce a broader wellbore-linked marker family that can represent:

- tops and bases
- contacts
- picks
- casing and operational markers
- depth/time-domain markers

`TopSet` remains a useful narrow asset family and potential import/conversion source for that later marker model.

## Non-Goals

This ADR does not by itself introduce:

- a full OSDU-style WellLog work-product model
- a full OSDU-style WellboreTrajectory schema
- a canonical well marker asset family
- frontend editing workflows for rich well metadata
- migration of every structured asset to richer metadata immediately

## Implementation Order

1. Add additive well and wellbore metadata fields to the project catalog.
2. Add a definitive trajectory pointer on wellbores.
3. Seed phase-one metadata from LAS on initial well/wellbore creation.
4. Make resolved trajectory queries prefer the definitive trajectory when present.
5. Add a broader well marker family in a follow-on ADR.
