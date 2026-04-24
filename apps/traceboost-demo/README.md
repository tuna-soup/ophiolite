# TraceBoost Demo

`traceboost-demo` is the flagship demo consumer application for `Ophiolite Charts`.

It lives inside the Ophiolite repository, but it must behave like an external consumer of the chart SDK.

The current design is:

- `Ophiolite Charts` is the standalone embeddable product surface
- `traceboost-demo` is the first-party demo application that consumes that surface
- `traceboost/` contains the demo-only backend, IO, runtime, and contract support crates

## Stack

- Svelte 5
- Vite
- Bun
- generated `@traceboost/seis-contracts` compatibility package
- external `@ophiolite/charts`
- Tauri 2 desktop shell under `src-tauri`

## Data Boundary

- inputs from the user:
  - SEG-Y file path
  - runtime-store output path
  - existing runtime-store path
- backend responses:
  - JSON payloads typed from the TraceBoost compatibility package `@traceboost/seis-contracts`
- rendered data:
  - section views returned by `traceboost-app` / `seis-runtime`

## Current layout

- frontend app: `apps/traceboost-demo`
- desktop shell crate: `apps/traceboost-demo/src-tauri`
- demo backend and CLI surface: `traceboost/app/traceboost-app`
- demo runtime and IO: `traceboost/runtime`, `traceboost/io`
- demo compatibility contracts: `traceboost/contracts`

The old standalone `TraceBoost` repo is no longer part of the build path.

## Implemented

- form-driven workflow for:
  - preflighting a SEG-Y file
  - importing into the runtime store
  - opening an existing runtime store
  - loading inline/xline sections
- shared frontend bridge that can call:
  - Vite dev endpoints in browser mode
  - Tauri commands in desktop mode
- embedded Ophiolite Charts section rendering
- typechecked/generated contract consumption through the compatibility package

## Development

From `apps/traceboost-demo`:

```powershell
bun install
bun run dev
```

If you want an explicit prerequisite check before install:

```powershell
bun run doctor
```

Additional checks:

```powershell
bun run typecheck
bun run build
```

Run the desktop shell:

```powershell
bun run tauri:dev
```

`bun run tauri:dev` picks a free frontend port automatically if `1420` is already taken and passes the matching `devUrl` override to Tauri for that run.

## Local Prerequisites

- use the repo-pinned Rust toolchain from `../../rust-toolchain.toml`

In browser dev mode, Vite exposes app-oriented endpoints that shell out to `traceboost-app` for:

- `/api/preflight`
- `/api/import`
- `/api/open`
- `/api/section`

## Boundary Rule

`@traceboost/seis-contracts` is a TraceBoost compatibility surface for the
demo/app boundary. It is not the canonical platform-owned frontend contract
layer, and it should not be treated as part of the `ophiolite-sdk` public
facade.

This demo may import only the approved public Ophiolite packages:

- `@ophiolite/charts`
- `@ophiolite/charts-toolbar`
- `@ophiolite/charts-data-models`
- `@ophiolite/contracts`

It must not import chart internals from raw source paths as part of its application code.
