---
title: Synthetic Project Fixture
description: Generate a coherent demo project with multiple wellbore asset families.
draft: false
---

Ophiolite can generate a deterministic synthetic project fixture for testing and demos.

## What it includes

- one well
- one wellbore
- one synthetic LAS log
- one trajectory CSV
- one tops CSV
- one pressure CSV
- one drilling CSV

These are imported through the real project APIs so the result exercises:

- ingest
- catalog creation
- asset packaging
- typed reads
- compute
- structured edits

## Generate it

```powershell
cargo run -- generate-synthetic-project test_data/projects/synthetic_well_project
```
