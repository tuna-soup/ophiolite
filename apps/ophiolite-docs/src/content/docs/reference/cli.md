---
title: CLI
description: The operational command surface for Ophiolite.
draft: false
---

**Audience:** automation and CI builders  
**Status:** Preview

The CLI is the operational command boundary for Ophiolite.

It is the platform command surface, not the TraceBoost desktop command table.

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

The inverse is also true:

- TraceBoost desktop command names are app-local adapter details
- the CLI should teach platform meanings directly
- app transport quirks should not be documented as if they were stable CLI API

## Relationship to TraceBoost desktop

TraceBoost desktop uses its own internal command boundary for frontend-to-backend transport.

That boundary may call the same shared runtime and contract layers as the CLI, but it is still an application shell concern:

- the command list is not a public Ophiolite API commitment
- desktop commands should stay thin and delegate to shared or app-framework behavior
- compatibility shims can exist in the desktop layer without expanding the platform promise

Use the CLI when you want a platform-owned operational surface. Use TraceBoost desktop when you want the first-party workflow application.

## Example

```powershell
cargo run --quiet --manifest-path Cargo.toml -p ophiolite-cli -- project-summary .\demo-project
```

Next: [CLI automation guide](/docs/build-workflows/cli-automation/)
