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
- compute-related descriptors and execution request types

These types are intentionally domain-first. Storage implementation details such as SQLite, Arrow, and Parquet are not the primary public abstraction.
