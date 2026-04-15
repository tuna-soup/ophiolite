---
title: OphioliteProject
description: The local-first multi-asset root for Ophiolite applications.
draft: false
---

`OphioliteProject` is the main multi-asset application surface.

## Responsibilities

- local catalog storage
- well and wellbore identity
- asset registration
- shared context for seismic and derived display/runtime outputs
- typed reads and edits
- compute execution
- revision history across assets

## On disk

A project owns:

- `catalog.sqlite`
- `assets/`
- hidden revision storage under `.ophiolite/`

Packages remain single-asset. The project is the unit that ties them together.

That is true whether the assets are wellbore-oriented source data, seismic runtime inputs, or derived outputs that need shared project context.
