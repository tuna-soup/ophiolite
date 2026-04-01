# contracts

`contracts/` is the shared app-boundary schema layer for `ophiolite`.

## Stack And Formats

- Rust 2024 DTOs owned by `ophiolite`
- `serde` for JSON serialization
- `schemars` for JSON Schema export
- `ts-rs` for generated TypeScript types
- generated frontend package at `ts/ophiolite-contracts`

This layer defines typed payloads that cross:

- `ophiolite` backend/core <-> frontend apps
- canonical project/catalog queries <-> chart/application adapters

The current exported slice is intentionally narrow:

- well-panel request DTOs
- resolved well-panel source DTOs
- typed supporting rows for logs, trajectory, tops, pressure, and drilling

Regenerate the TypeScript artifact from the repo root with:

```powershell
.\scripts\generate-ts-contracts.ps1
```

The generated output currently lives under:

- `ts/ophiolite-contracts/src/generated/`
- `ts/ophiolite-contracts/schemas/ophiolite-contracts.schema.json`

## Non-Goals

This layer must not own:

- chart/view models from `geoviz`
- storage/runtime implementation details
- product workflow logic
