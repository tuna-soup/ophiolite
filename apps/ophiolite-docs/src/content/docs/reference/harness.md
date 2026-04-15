---
title: Harness
description: The internal Tauri app used to validate Ophiolite workflows.
draft: false
---

`apps/ophiolite-harness` is the internal desktop validation surface for Ophiolite.

It currently exercises:

- project browsing
- log and structured asset inspection
- typed compute execution
- structured editing
- package/session editing for log assets
- app-facing DTO and query patterns that need a real desktop shell before they are treated as stable

The harness is not just a demo. It is the main proving ground for application-facing workflows before they are treated as stable SDK patterns.
