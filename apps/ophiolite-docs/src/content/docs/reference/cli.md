---
title: CLI
description: The operational command surface for Ophiolite.
draft: false
---

**Audience:** automation and CI builders  
**Status:** Preview

The CLI is the operational command boundary for Ophiolite.

## Current commands

The current public command set includes:

- `operation-catalog`
- `create-project`
- `open-project`
- `project-summary`
- `list-project-wells`
- `list-project-wellbores`
- `project-operator-lock`
- `install-operator-package`
- `list-project-compute-catalog`
- `run-project-compute`
- `import`
- `inspect-file`
- `summary`
- `list-curves`
- `examples`
- `generate-fixture-packages`

## What the CLI is for

- repeatable local commands
- JSON output for scripts
- CI validation
- package inspection and creation
- project admin-style tasks

## Design rule

If a behavior is part of the public platform contract, it should be visible as an intentional command, not hidden behind app-specific glue.

## Example

```powershell
cargo run --quiet --manifest-path Cargo.toml -p ophiolite-cli -- project-summary .\demo-project
```

Next: [CLI automation guide](/docs/build-workflows/cli-automation/)
