---
title: Platform Core
description: What the Ophiolite platform core owns and why it exists.
draft: false
---

Ophiolite is the platform core for the stack.

## What it owns

- canonical subsurface contracts and DTO meaning
- local-first package, project, and runtime foundations
- reusable ingest, query, edit, processing, and export primitives
- automation surfaces that should stay valid across multiple applications

## What it does not own

- one specific commercial workflow shell
- app-local workspace and session behavior
- chart-native rendering and viewport behavior

## Why this matters

If the platform boundary is clean, one application can demonstrate the workflow without becoming the source of truth for the underlying domain model.

That is the role of Ophiolite today: own the meaning first, then let applications compose it.
