# Lithos Harness

Internal Tauri desktop shell for exercising the `lithos_las` SDK end to end.

## Purpose

This is no longer just a single-screen command harness. It now behaves like a small desktop app:

- `Home` page for:
  - create package
  - open existing package
  - reopen recent packages
- `Workspace` page for:
  - overview
  - metadata inspector
  - curve catalog and editable sample table
  - LAS import/preview
  - diagnostics
  - read-only package file views

The app still exists to prove SDK capability coverage rather than product polish.

## Core Model

- `Package`
  - the saved folder on disk
  - contains `metadata.json` and `curves.parquet`
- `Session`
  - the live editable SDK state for one open package
- `Workspace`
  - the app shell around either:
    - a draft package folder with no session yet
    - a live package-backed session

Current workflow:

1. Create a package folder or open an existing package.
2. Creating a package immediately prompts for a LAS import.
3. If a LAS file is chosen, package files are written and a live SDK session opens immediately.
4. If LAS import is skipped, the app falls back to a draft workspace on the `Imports` view.
5. Inspect or edit metadata and curves.
6. Save or Save As from the toolbar or native File menu.

The current curve workspace is depth-range first:

- the UI prefers depth-range reads against the package/session index curve
- regular-step depth logs can resolve ranges very cheaply from package metadata
- row-window reads still exist underneath as the lower-level fallback/query primitive

## UI Structure

- `Home`
  - create package
  - open package
  - recent packages
  - terminology/workflow explanation
- `Workspace`
  - left package browser
  - center inspector/table surface
  - right-side session inspector
- `Curves`
  - selected curve
  - depth min / depth max controls
  - depth-range-backed sample table
- `Package Files`
  - read-only `metadata.json`
  - parquet/storage column summary
- `Imports`
  - raw LAS summary, metadata, curve catalog, validation, and window preview
  - import into draft or current workspace

## Native Menu

The Tauri shell exposes:

- `File > New Package`
- `File > Open Package...`
- `File > Import LAS...`
- `File > Save`
- `File > Save As...`
- `File > Close Workspace`

## Running

Rust-side harness tests:

```powershell
cargo test --manifest-path apps/lithos-harness/src-tauri/Cargo.toml
```

Frontend smoke tests:

```powershell
cd apps/lithos-harness
bun run test
```

Frontend production build:

```powershell
cd apps/lithos-harness
bun run build
```

Interactive app:

```powershell
cd apps/lithos-harness
bun install
bun tauri dev
```

The frontend is intentionally utilitarian. It is meant to feel like a desktop inspector, not the final product UI.

## Short Acceptance Workflow

1. Run `bun install` in `apps/lithos-harness`.
2. Run `bun run test` to verify the mocked app-boundary smoke tests.
3. Run `cargo test --manifest-path apps/lithos-harness/src-tauri/Cargo.toml` to verify the Rust command layer.
4. Run `bun tauri dev`.
5. Use the app flow:
   - create a package folder or open an existing one
   - import a LAS file if starting from a draft
   - inspect/edit/save in the workspace

## Manual Acceptance Checklist

1. Start on `Home` and confirm create/open/recent package actions are visible.
2. Create a draft workspace from a folder.
3. Choose a real `.las` file when prompted and confirm the workspace becomes a live session with session id, root, revision, and dirty state visible.
4. Open `Metadata`, change the company value or OTHER text, and apply the edit.
5. Open `Curves`, edit one or more loaded sample values, and apply the curve edit.
6. Use `Save` and confirm the session remains open and clean afterward.
7. Use `Save As` and confirm the session remains the same logical workspace but rebounds to the new root.
8. Open `Diagnostics` and `Package Files` to verify structured issues and read-only storage views render correctly.
9. In `Curves`, adjust the depth range and confirm the table reloads over that interval rather than only showing a fixed row window.
