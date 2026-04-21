# ADR-0024: Authored Marker-Set Assets and Canonical Marker Precedence

## Status

Accepted

## Context

ADR-0023 introduced canonical well markers as a wellbore-owned catalog layer synchronized from the
definitive `TopSet` asset. That gave Ophiolite a usable canonical marker surface, but it still left
marker authoring coupled to tops assets.

OSDU treats marker sets as separate wellbore child work-product-components. Ophiolite needs a
similar authored path without removing the existing tops bridge.

## Decision

Ophiolite adds a first-class `WellMarkerSet` structured asset kind.

### Asset model

`WellMarkerSet` is stored like other structured assets and carries authored marker rows with:

- name
- optional marker kind
- top depth
- optional base depth
- optional source
- optional depth-reference text
- optional note

### Wellbore selection

`WellboreRecord` gains:

- `definitive_marker_set_asset_id: Option<AssetId>`

Canonical marker synchronization now follows this precedence:

1. definitive marker set, when present
2. otherwise definitive top set, when present
3. otherwise no canonical markers

### Synchronization rules

- the first imported `WellMarkerSet` becomes the definitive marker set if none is selected yet
- overwriting the definitive marker set resynchronizes canonical markers
- clearing the definitive marker set falls back to the definitive top set if one exists
- top-set synchronization remains as a compatibility bridge and is ignored when a definitive marker
  set is selected

## Consequences

### Positive

- Ophiolite now has a separate authored marker-set asset path closer to the OSDU model
- existing tops-based ingestion still works
- the canonical marker API remains stable while the authored source evolves

### Tradeoffs

- marker sets and top sets can both exist for the same wellbore, so users need one selected
  interpretation
- well-panel DTOs still expose marker-set assets through the existing top-set shape rather than a
  dedicated marker DTO family

## Non-goals

This ADR does not add:

- compute operators for marker-set assets
- dedicated marker visualization DTOs
- interpreter/provenance history beyond source asset tracking
