---
title: Architecture Overview
description: The current layered architecture of Ophiolite.
draft: false
---

Ophiolite separates source-file import, domain modeling, runtime/query access, storage, and compute.

## Current layers

```text
source artifacts
  -> LAS / CSV importers
  -> canonical log + typed asset models
  -> single-asset packages
  -> OphioliteProject catalog + linked assets
  -> type-safe compute / derived assets
  -> app/query/edit workflows
```

## Workspace crates

- `ophiolite-core`
- `ophiolite-parser`
- `ophiolite-table`
- `ophiolite-package`
- `ophiolite-project`
- `ophiolite-ingest`
- `ophiolite-compute`
- `ophiolite-cli`

For the durable architecture record, use the ADRs.
