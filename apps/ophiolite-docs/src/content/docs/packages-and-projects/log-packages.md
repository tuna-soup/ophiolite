---
title: Log Packages
description: The single-asset package format for log data.
draft: false
---

Ophiolite log packages are optimized local storage units for one log asset.

## Layout

```text
log_asset.laspkg/
  metadata.json
  curves.parquet
```

## Why Parquet

Curve data is stored column-wise to support:

- projected reads
- depth-window access
- efficient package persistence

Ophiolite does not patch Parquet in place. Saves rewrite the payload into a new immutable revision snapshot and rematerialize the visible package head from that revision.
