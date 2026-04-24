# @traceboost/seis-contracts

This package is the TraceBoost-facing TypeScript compatibility artifact
generated from the Rust `seis-contracts-*` crates in the Ophiolite workspace.

It exists to keep the demo/app boundary typed during the current migration
window. It should not be described as the canonical platform-owned frontend
contract surface. Canonical cross-language contract ownership stays with the
platform-owned Rust contracts and the root `contracts/ts/ophiolite-contracts`
distribution, and `seis-contracts-interop` remains a compatibility lane that
stays outside the `ophiolite-sdk` facade.

Contents:

- generated TypeScript type bindings under `src/generated/`
- a JSON schema bundle under `schemas/seis-contracts.schema.json`

Regenerate from the repo root with:

```powershell
.\scripts\generate-ts-contracts.ps1
```

The TraceBoost package itself is emitted by:

```sh
cargo run -p traceboost-contracts-export
```

Consumers should treat this package as a TraceBoost compatibility surface for
demo IPC payloads, shared dataset/view models, desktop workspace/session DTOs,
and survey preflight metadata such as resolved stacking and layout
classification.
