---
title: CLI
description: Utility commands for generating and inspecting Ophiolite fixtures.
draft: false
---

The CLI is currently most useful for fixture generation and inspection workflows.

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
