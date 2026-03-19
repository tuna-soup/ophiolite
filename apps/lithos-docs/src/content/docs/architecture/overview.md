---
title: Architecture Overview
description: The current layered architecture of Lithos.
draft: false
---

Lithos separates source-file import, domain modeling, runtime/query access, storage, and compute.

## Current layers

```text
source artifacts
  -> LAS / CSV importers
  -> canonical log + typed asset models
  -> single-asset packages
  -> LithosProject catalog + linked assets
  -> type-safe compute / derived assets
  -> app/query/edit workflows
```

## Workspace crates

- `lithos-core`
- `lithos-parser`
- `lithos-table`
- `lithos-package`
- `lithos-project`
- `lithos-ingest`
- `lithos-compute`
- `lithos-cli`

For the durable architecture record, use the ADRs.
