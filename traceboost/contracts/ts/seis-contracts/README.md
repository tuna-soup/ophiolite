# @traceboost/seis-contracts

This package is the TypeScript-facing contract artifact generated from the Rust
`seis-contracts-*` crates in the Ophiolite workspace.

Contents:

- generated TypeScript type bindings under `src/generated/`
- a JSON schema bundle under `schemas/seis-contracts.schema.json`

Regenerate from the repo root with:

```powershell
.\scripts\generate-ts-contracts.ps1
```

Consumers should treat this package as the frontend contract surface for
TraceBoost demo IPC payloads, shared dataset/view models, desktop
workspace/session DTOs, and survey preflight metadata such as resolved stacking
and layout classification.
