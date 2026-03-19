# Ophiolite Harness

Internal Tauri desktop shell for exercising the `ophiolite` SDK end to end.

## Purpose

This is now a project-first multi-asset desktop harness, not just a package/session demo.

It exists to validate that the current SDK can support a real local-first subsurface workflow:

- open a `OphioliteProject`
- browse wells and wellbores
- inspect asset collections and typed assets
- import LAS and structured CSV assets
- open selected log assets into live package sessions when editing is needed
- open selected trajectory, tops, pressure, and drilling assets into typed structured edit sessions
- list and run eligible compute/UDF functions for selected log and structured assets

The app is still intentionally utilitarian. It is a capability harness, not the final product UI.

## Core Model

- `Project`
  - the multi-well root on disk
  - contains `catalog.sqlite` and `assets/`
- `Asset package`
  - one authoritative storage unit for one asset
  - log assets use `metadata.json + curves.parquet`
  - structured non-log assets use `metadata.json + data.parquet + asset_manifest.json`
- `Session`
  - the live editable SDK state for one selected log asset package
- `Workspace`
  - the app shell around one open project

## Current Workflow

1. Create a new `OphioliteProject` root or open an existing one.
2. Browse wells, wellbores, asset collections, and assets from the project catalog.
3. Import:
   - LAS logs
   - trajectory CSV
   - tops CSV
   - pressure observation CSV
   - drilling observation CSV
4. Select an asset to inspect it:
   - logs open the package/session-backed log viewer
   - non-log assets render typed tabular views and can enter structured edit mode
5. Run depth-range coverage queries for the selected wellbore.
6. Run available compute functions for the selected asset, adjust bindings/parameters when needed, and inspect the derived sibling asset.
7. Save or Save As when a log asset session is open, or explicitly save structured non-log edits from the editor panel.

## UI Structure

- `Home`
  - create project
  - open project
  - recent projects
- `Workspace`
  - left well browser
  - left-middle wellbore / collection / asset browser
  - center project panels:
    - overview
    - imports
    - depth coverage
    - selected asset viewer
  - right inspector

## Native Menu

The Tauri shell exposes:

- `File > New Project`
- `File > Open Project...`
- `File > Import Asset...`
- `File > Save`
- `File > Save As...`
- `File > Close Workspace`

`Save` and `Save As` apply to the currently selected log asset session when one is open.
Structured non-log assets use explicit save/discard controls inside the asset editor rather than `Save As`.

## Running

Rust-side harness tests:

```powershell
cargo test --manifest-path apps/ophiolite-harness/src-tauri/Cargo.toml
```

Frontend smoke tests:

```powershell
cd apps/ophiolite-harness
bun run test
```

Frontend production build:

```powershell
cd apps/ophiolite-harness
bun run build
```

Interactive app:

```powershell
cd apps/ophiolite-harness
bun install
bun tauri dev
```

## Short Acceptance Workflow

1. Run `bun install` in `apps/ophiolite-harness`.
2. Run `bun run test`.
3. Run `cargo test --manifest-path apps/ophiolite-harness/src-tauri/Cargo.toml`.
4. Run `bun tauri dev`.
5. In the app:
   - create or open a project
   - import a LAS log asset
   - import one structured non-log asset
   - browse the resulting well / wellbore / asset hierarchy
   - open the log asset and confirm the package/session-backed viewer loads
   - open a structured asset, enter edit mode, change one row, and save it
   - run one available compute function and confirm a derived asset appears in the same asset family
   - run a depth-range coverage query and confirm multiple assets can be opened from the result list

## Manual Acceptance Checklist

1. Start on `Home` and confirm create/open/recent project actions are visible.
2. Create a project and confirm the workspace opens on the project root.
3. Import a LAS file and confirm a log asset appears under the selected wellbore.
4. Import a trajectory, tops, pressure, or drilling CSV and confirm it is visible as a separate typed asset.
5. Select the log asset and confirm session-backed package inspection works.
6. Select a trajectory, tops, pressure, or drilling asset, enter edit mode, change one row, and save it.
7. Use `Save` and `Save As` with a selected log asset session.
8. Run an available compute function from a selected asset and confirm the derived asset appears in a derived collection.
9. Open `Depth Coverage`, enter a range, and confirm matching assets are listed.
