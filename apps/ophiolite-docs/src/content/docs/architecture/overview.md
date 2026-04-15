---
title: Architecture Overview
description: The current layered architecture of Ophiolite.
draft: false
---

Ophiolite separates ingest, canonical subsurface meaning, runtime/storage, application-boundary DTOs, and higher-level product workflows.

It is the core of the stack, not the commercial app shell and not the chart SDK.

## Current layers

```text
source artifacts
  -> import and normalization
  -> canonical subsurface contracts and typed asset models
  -> package, catalog, and runtime stores
  -> query, edit, compute, and derived-asset services
  -> app-boundary DTOs for sections, gathers, maps, wells, and time-depth
  -> product workflows and embedders above the core
```

## Workspace crates

- `ophiolite-core`
- `ophiolite-parser`
- `ophiolite-table`
- `ophiolite-package`
- `ophiolite-project`
- `ophiolite-ingest`
- `ophiolite-compute`
- `ophiolite-seismic`
- `ophiolite-seismic-io`
- `ophiolite-seismic-runtime`
- `ophiolite-cli`

## Practical boundary

- Ophiolite owns canonical subsurface semantics, contracts, and runtime primitives.
- product shells own workflow orchestration and external automation surfaces built on those primitives.
- visualization SDKs own chart rendering and interaction wrappers that consume Ophiolite-backed contracts.

For the durable architecture record, use the ADRs.
