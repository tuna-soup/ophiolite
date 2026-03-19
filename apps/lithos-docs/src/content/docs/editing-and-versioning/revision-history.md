---
title: Revision History
description: Inspectable save history for packages and project assets.
draft: false
---

Revision history is currently append-only.

Each revision records:

- a revision id
- parent linkage
- immutable payload blob refs
- a typed machine diff
- a readable change summary

## Deferred work

The current design leaves room for:

- retention policies
- garbage collection
- compaction
- future sync/export flows

Those are later infrastructure concerns, not part of the current desktop editing model.
