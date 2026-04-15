---
title: Repo Structure
description: The intended maintenance structure for the Ophiolite platform repo.
draft: false
---

The Ophiolite repo is the platform monorepo.

## Top-level layout

- `crates/` for Rust platform crates
- `contracts/` for shared schema and TypeScript export surfaces
- `charts/` for Ophiolite Charts packages and playgrounds
- `apps/ophiolite-docs` for the public docs site

## Practical implication

The core platform and chart SDK now evolve together in one repo because they are one product family with one public boundary.

Applications that consume the platform can still live separately and move at their own cadence.
