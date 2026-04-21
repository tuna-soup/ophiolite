# ADR-0023: Canonical Well Markers Phase One

## Status

Accepted

## Context

ADR-0022 added canonical well and wellbore metadata plus a definitive trajectory pointer, but
well markers still only existed as rows inside imported `TopSet` assets. That left Ophiolite
without a canonical wellbore-owned marker surface for:

- API and SDK consumers that want one marker list per wellbore
- selecting one preferred marker interpretation when multiple top sets exist
- carrying marker rows forward without forcing every consumer to understand asset packages

OSDU models wellbore markers as first-class wellbore records with measurement context and
provenance. Ophiolite does not need the entire OSDU envelope yet, but it does need the same basic
ownership boundary.

## Decision

Ophiolite adds a phase-one canonical well marker model owned by the project catalog.

### Catalog shape

`WellboreRecord` gains:

- `definitive_top_set_asset_id: Option<AssetId>`

The catalog adds `WellMarkerRecord`, which captures:

- wellbore ownership
- optional source asset id
- marker name
- sequence number
- marker kind
- top measurement
- optional base measurement
- raw depth reference text
- source text
- optional external references and notes

Markers are stored in a dedicated `well_markers` catalog table rather than embedded inside
`WellboreMetadata`.

### Sync behavior

Phase one canonical markers are synchronized from the wellbore's definitive `TopSet` asset.

- the first imported `TopSet` asset becomes the definitive top set if none is selected yet
- setting a new definitive top set replaces the canonical markers for that wellbore
- clearing the definitive top set clears the canonical markers
- overwriting the definitive top set asset resynchronizes the canonical markers

Imported top rows are translated into canonical marker measurements using the row
`depth_reference` field when present. Unknown or missing depth-reference strings are preserved as
raw text and mapped to the `unknown` measurement path.

## Consequences

### Positive

- Ophiolite now has a single wellbore-level marker surface close to the OSDU ownership model
- existing tops import and edit workflows remain intact
- canonical markers stay additive and can evolve without breaking the `TopSet` asset format

### Tradeoffs

- phase one still assumes canonical markers come from one preferred tops asset
- canonical markers are synchronized data, not yet independently authored records
- marker semantics such as stratigraphic relationships, confidence, and interpreter identity remain
  future work

## Non-goals

This ADR does not add:

- a separate authored well-marker asset family
- stratigraphic hierarchy or formation relationship graphs
- interpreter audit history beyond source asset provenance
- frontend editing for canonical markers
