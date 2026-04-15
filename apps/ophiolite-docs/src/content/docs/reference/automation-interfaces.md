---
title: Automation Surfaces
description: How Ophiolite approaches CLI and Python interfaces.
draft: false
---

Ophiolite should expose automation surfaces that make sense at the platform boundary.

## Current shape

Today the CLI is still developer-heavy. It is strongest for fixture generation, package inspection, and repo-local validation.

Ophiolite now keeps a checked-in operation catalog for the platform-owned automation slice. That catalog is intentionally narrower than the full platform surface because more reusable capability still exists as Rust APIs than as polished end-user commands.

There is now also an initial thin Python wrapper in `python/ophiolite_automation`. It shells out to the same CLI commands rather than creating a second platform backend.

Those two surfaces are meant to stay mechanically aligned. The operation catalog, the Python conformance check, and CI verification all exist to keep the wrapper thin and keep naming drift visible.

## Intended shape

The long-term platform interfaces are:

- a stable Ophiolite CLI for core ingest, inspect, open, export, and validation workflows
- a thin Python package over those same platform workflows

Those interfaces should wrap existing platform primitives. They should not create a second backend or duplicate application orchestration.

## Boundary rule

- platform-stable automation belongs in Ophiolite
- application-specific workflow recipes belong in the application layer built on top of Ophiolite

## Ownership split

Examples that belong in `Ophiolite`:

- project creation and project inventory queries
- package inspection and validation
- canonical ingest and typed compute
- reusable well-panel, survey-map, and geometry resolution

Examples that belong in an application layer:

- opinionated import workflows around a specific product shell
- product-local export and demo preparation flows
- orchestration exposed as a workflow rather than a reusable platform primitive

The practical rule is simple: if an operation still makes sense when a specific app disappears, it should trend toward the Ophiolite catalog. If it mainly packages Ophiolite capability into one app workflow, it should stay in that app.
