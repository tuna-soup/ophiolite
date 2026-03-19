---
title: Revisions and History
description: How Lithos keeps edits simple while still preserving history.
draft: false
---

Lithos uses a simple local save model in the UI, but saves are revision-aware under the hood.

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
