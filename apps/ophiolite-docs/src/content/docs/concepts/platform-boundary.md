---
title: Product Boundary
description: What Ophiolite owns at the platform boundary, and what it does not.
draft: false
---

Ophiolite is the canonical subsurface platform at the product boundary.

## What Ophiolite owns

- canonical subsurface contracts and DTO meaning
- typed domain models for wellbore and seismic data
- local-first package, catalog, and runtime foundations
- query, edit, compute, and derived-asset primitives
- app-boundary payloads that products and embedders can rely on

## What Ophiolite does not own

- commercial workflow UX and session orchestration
- customer-facing product automation flows
- chart rendering and chart-native interaction behavior

## How it links to higher layers

- application shells turn the core into end-user workflows
- automation surfaces sit above the core and expose product-shaped commands
- Ophiolite Charts renders contract-backed views without owning the underlying domain meaning

## Practical implication

If a type is reusable subsurface meaning, Ophiolite should own it first.

If a feature is a product workflow, preset, session rule, or customer-facing automation recipe, it belongs in the application layer.

If a feature is chart rendering, viewport behavior, or wrapper-layer adaptation for embedders, it belongs in the visualization layer.
