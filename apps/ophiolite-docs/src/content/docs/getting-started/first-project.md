---
title: First Project
description: Work with multiple linked wellbore asset families under OphioliteProject.
draft: false
---

`OphioliteProject` is the multi-asset entry point above individual packages.

## Project responsibilities

- store wells and wellbores in a local catalog
- register collections and assets
- keep asset packages single-purpose and typed
- provide typed reads, editing flows, compute, and revision history

## Typical flow

1. Create or open a project root.
2. Import a LAS file as a log asset.
3. Import trajectory, tops, pressure, or drilling data from CSV.
4. Query and edit those assets through the project surface.

```rust
use ophiolite::OphioliteProject;

fn main() -> Result<(), ophiolite::LasError> {
    let project = OphioliteProject::open("test_data/projects/synthetic_well_project")?;
    let wells = project.list_wells()?;
    println!("Wells: {}", wells.len());
    Ok(())
}
```

Continue with:

- [Well Domain Model](/concepts/well-domain-model/)
- [OphioliteProject](/packages-and-projects/ophiolite-project/)
