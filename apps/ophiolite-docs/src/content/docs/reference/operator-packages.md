---
title: Operator Packages
description: Manifest and runtime expectations for external operator packages.
draft: false
---

**Audience:** operator authors  
**Status:** Preview

Ophiolite should treat built-in compute and external extensions as one operator system.

## Operator model

An operator package should declare:

- package name
- package version
- runtime kind such as `python` or `rust`
- compatible Ophiolite version range
- exported operator ids
- entrypoint information
- supported asset families

## Execution model

External Python operators should execute out of process by default so that:

- crashes stay isolated
- Python dependencies stay isolated
- upgrades are easier to manage
- future sandboxing remains possible

## Current implementation

The repo now supports:

- per-project installation
- project operator locking
- package-local `.venv` creation for Python runtimes
- manifest-driven catalog exposure
- Rust-owned validation and provenance
- derived-asset persistence through the normal project flow

## Authoring helpers

Use `OperatorRegistry`, `OperatorRequest`, and `computed_curve(...)` from `ophiolite_sdk` in Python entrypoints.

See [Write your first operator](/operators/write-your-first-operator/) for the end-to-end example.
