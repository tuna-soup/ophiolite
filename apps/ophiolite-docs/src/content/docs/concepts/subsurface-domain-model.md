---
title: Subsurface Domain Model
description: How Ophiolite models linked subsurface assets, not just isolated files.
draft: false
---

Ophiolite uses a subsurface domain graph rather than a file-first API.

## Main entities

- `Well`
- `Wellbore`
- `AssetCollection`
- `Asset`

Those entities remain important because many workflows are still organized around well and wellbore identity.

Assets are typed. Current first-class families are:

- log
- trajectory
- tops
- pressure observations
- drilling observations
- seismic trace data and related runtime descriptors

The core also owns adjacent canonical DTO families for:

- survey maps
- section and gather views
- well overlays
- time-depth and velocity-model workflows

## Why this matters

Real subsurface workflows need linked data, not isolated files. A wellbore may have:

- one or more log assets
- one trajectory
- multiple tops sets
- pressure observations
- drilling observations
- linked seismic context and display/runtime projections

`OphioliteProject` gives those assets shared identity and query context while keeping each saved asset package focused and independent.
