---
title: Compute and Derived Assets
description: Typed compute in Ophiolite is family-aware and asset-aware.
draft: false
---

Ophiolite compute is not a loose “run any UDF on any column” model.

## Key rules

- compute is type-safe
- function eligibility depends on asset family or curve semantics
- outputs are usually persisted as derived sibling assets
- display DTOs and analysis APIs stay distinct from materialized outputs when they have different lifecycle rules

Examples:

- `VShale` is valid for gamma ray curves
- trajectory transforms apply to trajectory assets
- structured compute stays within the same family
- seismic processing operators apply to canonical seismic runtime inputs
- time-depth and velocity-model workflows can expose analysis or display payloads without pretending they are saved assets

## Why it is modeled this way

- better UX for function discovery
- fewer invalid workflows
- cleaner provenance and derived-asset lineage
- room to grow across well, seismic, map, and time-depth workflows without redesigning the compute surface
