---
title: Log Editing
description: How Lithos edits and saves log assets.
draft: false
---

Log editing currently uses `PackageSession`.

## Characteristics

- in-memory edits against `LasFile`
- dirty-state tracking
- metadata edits and curve edits
- explicit `save` and `save_as`
- revision-aware persistence

## Save behavior

- accepted edits mutate the session state in memory
- `save` writes a new canonical revision snapshot
- the visible package root is rematerialized from that new head
- the session stays bound to the same package unless `save_as` rebases it
