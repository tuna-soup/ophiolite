# ADR-0007: Canonical Schema Target and Roadmap Placement

## Status

Accepted

## Decision

`lithos` keeps a target-state canonical schema direction, but does not claim full conformance to that target yet.

The current implementation already provides:

- canonical domain semantics through `LasFile`
- typed metadata views
- runtime table access through `CurveTable`
- package persistence through `metadata.json + curves.parquet`

The longer-term canonical schema target is:

- one canonical sample-axis column plus one column per curve
- stable canonical handling of the index axis
- nulls represented canonically in the sample table rather than LAS sentinel values
- metadata, provenance, units, original mnemonics, and diagnostics stored outside the sample matrix
- tighter alignment between package schema and canonical metadata descriptors

Arrow/Parquet may support this target internally, but they do not define the public domain model.

## Why

- the project needs a durable record of the intended end-state without pretending the current implementation is already there
- the target helps sequence work on package rules, query contracts, and future runtime changes
- the canonical schema direction needs to live in durable architecture docs rather than a drifting root note

## Consequences

- documentation should describe current state separately from target state
- roadmap items that mention canonical-schema alignment should refer to this target direction
- package/runtime changes should not claim full canonical alignment until index rules, nullability rules, and schema guarantees are actually stable

## Prerequisites Before Full Alignment

Before `lithos` should claim fuller canonical-schema conformance, it should first stabilize:

1. package schema and version guarantees
2. Tauri/backend DTO and query contracts
3. nullability, index, and curve descriptor rules
4. editable-session loading behavior where it materially improves the desktop workflow

Only after those settle should the project tighten runtime and package behavior toward the stricter canonical sample-table target.
