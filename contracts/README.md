# contracts

`contracts/` is the shared app-boundary schema layer for `ophiolite`.

It is the canonical contract surface for reusable subsurface meaning in the current stack.

## Stack And Formats

- Rust 2024 DTOs owned by `ophiolite`
- `serde` for JSON serialization
- `schemars` for JSON Schema export
- `ts-rs` for generated TypeScript types
- generated frontend package at `ts/ophiolite-contracts`

This layer defines typed payloads that cross:

- `ophiolite` backend/core <-> frontend apps
- canonical project/catalog queries <-> chart/application adapters

The current exported slice spans multiple subsurface workflows, including:

- well, wellbore, and typed supporting rows for logs, trajectory, tops, pressure, and drilling
- well-panel request and resolved-source DTOs
- survey-map request and resolved-source DTOs
- canonical section and gather view families
- well-on-section overlay DTOs
- time-depth, CRS, and related display/runtime boundary types

Regenerate the TypeScript artifact from the repo root with:

```powershell
.\scripts\generate-ts-contracts.ps1
```

The generated output currently lives under:

- `ts/ophiolite-contracts/src/generated/`
- `ts/ophiolite-contracts/schemas/ophiolite-contracts.schema.json`

## Non-Goals

This layer must not own:

- chart renderer internals
- storage/runtime implementation details
- product workflow logic
- product-specific session/orchestration transport
