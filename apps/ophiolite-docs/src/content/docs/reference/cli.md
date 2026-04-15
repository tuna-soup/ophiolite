---
title: CLI
description: The current and intended CLI boundary for Ophiolite.
draft: false
---

The Ophiolite CLI is currently strongest as a developer and validation surface.

It is most useful for fixture generation, inspection, and repo-local workflows around the core platform.

It is not meant to own product-local session behavior or opinionated app workflows.

The intended direction is a stable platform CLI for import, inspect, open, export, and validation workflows that should remain meaningful across multiple applications built on Ophiolite.

Examples:

```powershell
cargo run -- generate-fixture-packages test_data/logs test_data/logs/packages
```

```powershell
cargo run -- generate-synthetic-project test_data/projects/synthetic_well_project
```

These are especially useful for:

- testing package generation
- generating a coherent multi-asset demo project
- validating the app and SDK against stable local fixtures
- exercising the core outside the product shell
