---
title: Overview
description: The main ideas behind Ophiolite.
draft: false
---

Ophiolite is built around a few durable concepts:

- canonical subsurface contracts instead of product-specific transport types
- typed asset families instead of generic blobs or arbitrary tables
- single-asset packages and runtime stores for optimized local persistence
- `PackageSession` and typed edit/query surfaces for bounded working state
- `OphioliteProject` for multi-asset subsurface workflows
- family-aware compute, derived assets, and display DTOs
- overwrite-oriented editing with immutable revision history beneath a simple local workflow

This means Ophiolite is not only a parser and not only a storage format. It is the core layer for:

- ingest
- modeling
- canonical contracts
- query
- editing
- compute
- runtime projection
- revision-aware persistence
- application-facing DTOs

The rest of the docs explain how those layers fit together.
