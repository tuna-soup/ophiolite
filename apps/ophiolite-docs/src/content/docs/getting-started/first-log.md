---
title: First Log
description: Start with Ophiolite as a LAS and log package SDK.
draft: false
---

The simplest Ophiolite workflow starts from a LAS file and produces a log package.

## Core flow

1. Read a LAS file into `LasFile`.
2. Inspect metadata and curves.
3. Save to a package with `metadata.json + curves.parquet`.
4. Open a `PackageSession` for editing or windowed reads.

```rust
use ophiolite::read_path;

fn main() -> Result<(), ophiolite::LasError> {
    let file = read_path("test_data/logs/6038187_v1.2_short.las", &Default::default())?;
    println!("Curves: {:?}", file.curve_names());
    Ok(())
}
```

## What you get

- canonical log-domain access through `LasFile`
- package-backed storage optimized for query-style reads
- depth-window and row-window access patterns
- revision-aware saves when a package is edited

If you want the bigger application model, continue with [First Project](/getting-started/first-project/).
