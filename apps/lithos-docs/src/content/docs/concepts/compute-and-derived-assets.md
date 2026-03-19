---
title: Compute and Derived Assets
description: Typed compute in Lithos is family-aware and asset-aware.
draft: false
---

Lithos compute is not a loose “run any UDF on any column” model.

## Key rules

- compute is type-safe
- function eligibility depends on asset family or curve semantics
- outputs are usually persisted as derived sibling assets

Examples:

- `VShale` is valid for gamma ray curves
- trajectory transforms apply to trajectory assets
- structured compute stays within the same family

## Why it is modeled this way

- better UX for function discovery
- fewer invalid workflows
- cleaner provenance and derived-asset lineage
- room to grow beyond logs without redesigning the compute surface
