---
title: Overview
description: The main ideas behind Ophiolite.
draft: false
---

Ophiolite is built around a few durable concepts:

- `LasFile` for canonical log-domain access
- single-asset packages for optimized local persistence
- `PackageSession` for log editing
- `OphioliteProject` for multi-asset wellbore workflows
- typed asset families instead of generic blobs or arbitrary tables
- overwrite-oriented editing with immutable revision history

This means Ophiolite is not only a parser and not only a storage format. It is a layered SDK for:

- ingest
- modeling
- query
- editing
- compute
- revision-aware persistence

The rest of the docs explain how those layers fit together.
