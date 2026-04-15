---
title: Python SDK
description: The primary builder surface for local-first Ophiolite workflows.
draft: false
---

**Audience:** workflow builders  
**Status:** Preview

The Python SDK is the main intended builder surface for Ophiolite.

It is designed to expose Ophiolite nouns such as `Project`, operator catalogs, compute requests, and operator-package installation rather than mirroring internal Rust modules directly.

## Current API shape

Today the public package exposes:

- `Project`
- `ComputeRequest`
- `PlatformCatalog`
- `OperatorRegistry`
- `OperatorRequest`
- `computed_curve(...)`

## What it is good for

- local project lifecycle
- typed summaries and discovery
- operator package installation
- compute catalog lookup
- compute execution
- Python operator authoring

## What it is not yet

- a full mirror of every Rust capability
- a cloud API client
- a place to expose storage internals as the primary abstraction

## Relationship to the CLI

The Python SDK and CLI should expose the same platform meanings. The SDK is the preferred builder surface. The CLI remains useful for scripting, CI, and operational tasks.
