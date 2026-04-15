---
title: First Asset
description: Start with one typed asset and grow into the broader platform from there.
draft: false
---

The simplest Ophiolite workflow still starts from a single asset.

Today, the easiest concrete example is a LAS-backed log import, but that is only one entry path into the broader platform.

## Core flow

1. Read or import one typed asset.
2. Inspect its canonical metadata and samples.
3. Persist it into a package or register it inside a project.
4. Open the appropriate query or editing surface for the next workflow step.

```rust
use ophiolite::read_path;

fn main() -> Result<(), ophiolite::LasError> {
    let file = read_path("test_data/logs/6038187_v1.2_short.las", &Default::default())?;
    println!("Curves: {:?}", file.curve_names());
    Ok(())
}
```

## What you get

- canonical asset access through a domain-first API
- package-backed or project-backed storage optimized for query-style reads
- typed boundaries that can later feed application DTOs and runtime services
- revision-aware saves when an editable asset is changed

If you want the bigger application model, continue with [First Project](/getting-started/first-project/).
