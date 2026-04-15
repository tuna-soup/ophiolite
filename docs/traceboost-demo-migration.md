# TraceBoost Demo Structure

`traceboost-demo` is now the first-party workflow demo for `Ophiolite Charts`.

The migration from the standalone `TraceBoost` repo is complete. This document describes the current design, not a future move.

## Current Repository Shape

```text
ophiolite/
  apps/
    ophiolite-docs/
    traceboost-demo/
  charts/
    packages/
    apps/
  contracts/
  crates/
  traceboost/
    contracts/
    io/
    runtime/
    app/traceboost-app/
```

## Product Boundary

`traceboost-demo` consumes the chart SDK through public package entry points.

Current approved Ophiolite surface for the demo:

- `@ophiolite/charts`
- `@ophiolite/charts-toolbar`
- `@ophiolite/charts-data-models`
- `@ophiolite/contracts`

If the demo needs more functionality, prefer promoting that capability into an intentional public package API rather than importing chart internals.

## Demo Support Stack

- frontend app: `apps/traceboost-demo`
- desktop shell crate: `apps/traceboost-demo/src-tauri`
- demo backend and CLI surface: `traceboost/app/traceboost-app`
- demo runtime and IO: `traceboost/runtime`, `traceboost/io`
- demo contracts: `traceboost/contracts`

These `traceboost/` crates remain inside the Ophiolite repo because the demo still needs them, but they are demo support code, not the commercial chart product surface.

## Dev-Time Behavior

- browser dev mode in `apps/traceboost-demo/vite.config.ts` shells out to `cargo run -p traceboost-app`
- desktop mode in `apps/traceboost-demo/src-tauri` depends directly on `traceboost-app`, `seis-runtime`, and the demo contract crates
- `bun run tauri:dev` chooses a free frontend port dynamically and overrides Tauri `devUrl` for that run

## Validation

At minimum, keep these green:

1. `bun install` in `charts/`
2. `bun run typecheck` in `charts/`
3. `bun install` in `apps/traceboost-demo`
4. `bun run doctor` in `apps/traceboost-demo`
5. `bun run typecheck` in `apps/traceboost-demo`
6. `bun run build` in `apps/traceboost-demo`
7. `cargo check -p traceboost-app`
8. `cargo check -p traceboost-desktop`

## Rule

Treat `traceboost-demo` as a harsh consumer of `Ophiolite Charts`.

If the demo needs a new capability, add it to an intentional public Ophiolite surface. Do not let the demo bypass the product boundary through raw source imports or ad hoc compatibility wiring.
