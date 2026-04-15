---
title: Package, Session, and Workspace
description: Understand the key runtime terms Ophiolite uses for desktop workflows.
draft: false
---

These terms matter because they operate at different levels.

## Package

A package is an optimized local saved asset.

For log assets, that usually means:

```text
asset.laspkg/
  metadata.json
  curves.parquet
```

Other families may use different physical layouts or runtime stores. Ophiolite owns the canonical access semantics above those details.

## Session

A session is the editable in-memory working state for one package-backed asset.

Today:

- log assets use `PackageSession`
- structured assets use typed edit sessions inside `OphioliteProject`
- runtime-oriented seismic flows use family-specific stores and query surfaces rather than forcing everything through one log-shaped session

## Workspace

The app workspace is the UI shell around those sessions:

- project browser
- inspectors
- viewers
- edits
- compute actions
- save actions

Packages are storage, sessions are working state, and workspaces are application context.
