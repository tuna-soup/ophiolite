---
title: LithosProject
description: The local-first multi-asset root for Lithos applications.
draft: false
---

`LithosProject` is the main multi-asset application surface.

## Responsibilities

- local catalog storage
- well and wellbore identity
- asset registration
- typed reads and edits
- compute execution
- revision history across assets

## On disk

A project owns:

- `catalog.sqlite`
- `assets/`
- hidden revision storage under `.lithos/`

Packages remain single-asset. The project is the unit that ties them together.
