---
title: Core Types
description: The main public concepts exposed by Ophiolite today.
draft: false
---

Important public concepts include:

- `LasFile`
- `CurveTable`
- `PackageSession`
- `OphioliteProject`
- `AssetKind`
- typed row models for structured assets
- canonical section, gather, and survey-map DTO families
- time-depth and velocity-model boundary types
- compute-related descriptors and execution request types

These types are intentionally domain-first. Storage implementation details such as SQLite, Arrow, and Parquet are not the primary public abstraction.

The practical rule is simple: if the type expresses reusable subsurface meaning, it should live here before products or chart SDKs adapt it.
