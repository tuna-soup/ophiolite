---
title: Well Domain Model
description: How Ophiolite models wells, wellbores, collections, and assets.
draft: false
---

Ophiolite uses a well-domain graph rather than a file-first API.

## Main entities

- `Well`
- `Wellbore`
- `AssetCollection`
- `Asset`

Assets are typed. Current first-class families are:

- log
- trajectory
- tops
- pressure observations
- drilling observations

## Why this matters

Real subsurface workflows need linked data, not isolated files. A wellbore may have:

- one or more log assets
- one trajectory
- multiple tops sets
- pressure observations
- drilling observations

`OphioliteProject` gives those assets shared identity and query context while keeping each saved asset package focused and independent.
