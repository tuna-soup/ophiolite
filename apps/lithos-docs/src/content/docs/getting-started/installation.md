---
title: Installation
description: Set up Lithos locally and understand the repo layout.
draft: false
---

Lithos is currently developed as a Rust monorepo with supporting Bun-based frontend apps.

## Prerequisites

- Rust toolchain
- Bun
- Git

## Repo shape

The main workspace crates are:

- `lithos-core`
- `lithos-parser`
- `lithos-table`
- `lithos-package`
- `lithos-project`
- `lithos-ingest`
- `lithos-compute`
- `lithos-cli`

The compatibility facade is:

- `lithos_las`

The internal app surface is:

- `apps/lithos-harness`

## Local commands

Rust:

```powershell
cargo test
```

Harness:

```powershell
cd apps/lithos-harness
bun install
bun run test
bun run build
```

Next:

- [Open your first log](/getting-started/first-log/)
- [Open your first project](/getting-started/first-project/)
