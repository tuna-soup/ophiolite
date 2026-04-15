---
title: Contracts and DTOs
description: The canonical app-boundary types Ophiolite exports.
draft: false
---

Ophiolite owns the canonical app-boundary contracts for reusable subsurface meaning.

## What lives here

- frontend-safe DTOs exported from `contracts/`
- generated TypeScript contracts for embedders and app shells
- canonical request and response shapes for sections, gathers, maps, well overlays, and related workflows

## What does not live here

- chart renderer internals
- product workflow/session rules
- app-specific transport wrappers that only make sense inside one product shell

## Why this matters

The contract layer is the point where applications, SDK wrappers, and backend services agree on meaning.

That lets:

- Ophiolite stay the canonical source of subsurface DTO meaning
- product shells build workflows without redefining those semantics
- embedders such as Ophiolite Charts adapt canonical payloads for rendering without owning the domain model
