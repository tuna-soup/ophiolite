---
title: First Project
description: Work with multiple linked subsurface assets under OphioliteProject.
draft: false
---

`OphioliteProject` is the multi-asset entry point above individual packages.

## Project responsibilities

- store wells and wellbores in a local catalog
- register collections and assets
- keep asset packages single-purpose and typed
- provide typed reads, editing flows, compute, and revision history
- host shared seismic and map/time-depth context through canonical contracts and runtime links where appropriate

## Typical flow

1. Create or open a project root.
2. Import one or more typed assets such as logs, trajectory, tops, pressure, drilling, or seismic data.
3. Resolve the linked project context through typed APIs and contracts.
4. Query, edit, compute, or project those assets through the project surface.

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

- [Subsurface Domain Model](/concepts/subsurface-domain-model/)
- [OphioliteProject](/packages-and-projects/ophiolite-project/)

The current synthetic examples are still strongest on the wellbore side, but the project boundary is broader than that.
