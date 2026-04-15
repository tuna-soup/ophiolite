---
title: Asset Packages
description: The local package layer for typed subsurface assets.
draft: false
---

Ophiolite uses single-asset packages and family-specific runtime stores to keep local persistence typed and explicit.

## One concrete package layout

Log assets remain the simplest example:

```text
log_asset.laspkg/
  metadata.json
  curves.parquet
```

## Why the package layer matters

- storage stays local and explicit
- each asset family can keep a fit-for-purpose physical layout
- higher layers can depend on canonical semantics instead of file quirks

For log packages specifically, curve data is stored column-wise to support:

- projected reads
- depth-window access
- efficient package persistence

Ophiolite does not patch payloads in place. Saves rewrite the payload into a new immutable revision snapshot and rematerialize the visible head from that revision.
