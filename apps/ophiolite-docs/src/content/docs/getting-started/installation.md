---
title: Installation
description: Set up Ophiolite locally and understand the repo layout.
draft: false
---

Ophiolite is currently developed as a Rust monorepo with supporting Bun-based frontend apps.

## Prerequisites

- Rust toolchain
- Bun
- Git

## Repo shape

The main workspace crates are:

- `ophiolite-core`
- `ophiolite-parser`
- `ophiolite-table`
- `ophiolite-package`
- `ophiolite-project`
- `ophiolite-ingest`
- `ophiolite-compute`
- `ophiolite-seismic`
- `ophiolite-seismic-io`
- `ophiolite-seismic-runtime`
- `ophiolite-cli`

The compatibility facade is:

- `ophiolite`

The generated contract surface is:

- `contracts/`

The internal app surface is:

- `apps/ophiolite-harness`

## Local commands

Rust:

```powershell
cargo test
```

Harness:

```powershell
cd apps/ophiolite-harness
bun install
bun run test
bun run build
```

Next:

- [Open your first asset](/getting-started/first-asset/)
- [Open your first project](/getting-started/first-project/)
