# contracts

`traceboost/contracts` is the shared schema layer for the TraceBoost demo support stack inside the Ophiolite repo.

These contracts are demo-facing compatibility surfaces. They describe what the TraceBoost demo needs to move across its own runtime, app-shell, and frontend boundaries, while canonical reusable subsurface meaning remains owned by Ophiolite.

## Stack And Formats

- Rust 2024 crates:
  - `seis-contracts-core`
  - `seis-contracts-views`
  - `seis-contracts-operations`
  - `seis-contracts-interop`
- `serde` for JSON serialization
- `schemars` for JSON Schema export
- `ts-rs` for generated TypeScript types
- generated frontend package at `ts/seis-contracts`

The contracts layer defines the typed payloads that cross:

- runtime <-> app/backend
- app/backend <-> Tauri frontend
- demo runtime <-> demo frontend consumers

Current architectural direction:

- the `seis-contracts-core`, `seis-contracts-views`, `seis-contracts-operations`, and `seis-contracts-interop` split is a compatibility layer over Ophiolite-owned seismic taxonomy
- Ophiolite contract ownership is now split by concern under `ophiolite-seismic/src/contracts/`:
  - `domain.rs`
  - `processing.rs`
  - `models.rs`
  - `views.rs`
  - `operations.rs`
- TraceBoost crates now expose matching compatibility namespaces:
  - `seis-contracts-core::{domain, processing, models, operations, views}`
  - `seis-contracts-views::{section, gather}`
  - `seis-contracts-operations::{datasets, import_ops, processing_ops, workspace, resolve}`
  - `seis-contracts-interop::*` as a compatibility re-export of `seis-contracts-operations`
- the owning Rust source for app/workflow operations now lives in `seis-contracts-operations`; `seis-contracts-interop` remains only to avoid a breaking rename across downstream consumers
- packed frontend section transport is now explicit in `../apps/traceboost-demo/src/lib/transport/packed-sections.ts` instead of living only as bridge-local helpers

## Implemented

- dataset and volume descriptors
- section-axis and section-view contracts
- preview/view request-response contracts
- survey preflight request-response contracts
  - includes resolved stacking/layout metadata so apps can distinguish post-stack vs prestack before ingest
- dataset import request-response contracts
- dataset open/summary request-response contracts
- dataset registry and workspace-session payloads for the desktop shell
- schema-versioned IPC types for the first desktop workflow

Regenerate the TypeScript artifact from the repo root with:

```powershell
.\scripts\generate-ts-contracts.ps1
```

That script now runs both generators:

- `cargo run -p contracts-export`
- `cargo run -p traceboost-contracts-export`

The generated output currently lives under:

- `ts/seis-contracts/src/generated/`
- `ts/seis-contracts/schemas/seis-contracts.schema.json`

## Roadmap

1. Keep this layer narrow and demo-specific.
2. Push reusable meaning down into Ophiolite before growing demo compatibility DTOs.
3. Add only the next demo-facing contracts that the desktop workflow actually needs.
4. Avoid growing a second canonical contract model inside the demo stack.

## Non-Goals

This layer must not own:

- SEG-Y parsing
- runtime-store layout or chunk access
- processing kernels
- product workflow logic
