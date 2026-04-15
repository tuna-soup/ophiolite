---
title: Revisions and History
description: How Ophiolite keeps edits simple while still preserving history.
draft: false
---

Ophiolite uses a simple local save model in the UI, but saves are revision-aware under the hood.

## User-facing behavior

- edit in memory
- save explicitly
- latest save becomes the active head

## Persistence behavior

- every successful save creates a new immutable revision
- hidden revision stores are canonical
- the visible package or asset root is materialized from the active head
- revisions record parent linkage, blob refs, machine diffs, and a readable summary

This keeps desktop workflows simple without giving up lineage, inspection, or future sync hooks.

That same model applies across package-backed assets, project-managed structured assets, and runtime outputs that need durable provenance.
