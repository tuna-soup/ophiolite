---
title: Asset Editing
description: How Ophiolite edits and saves package-backed assets today.
draft: false
---

The most mature package-backed editing path currently uses `PackageSession` for log assets.

That is the current implementation center, not the public scope limit for the platform.

## Characteristics

- in-memory edits against `LasFile`
- dirty-state tracking
- metadata edits and curve edits
- explicit `save` and `save_as`
- revision-aware persistence

Other families use their own typed edit surfaces where that is a better fit than forcing everything through one session model.

## Save behavior

- accepted edits mutate the session state in memory
- `save` writes a new canonical revision snapshot
- the visible package root is rematerialized from that new head
- the session stays bound to the same package unless `save_as` rebases it
