# Lithos Harness

Internal Tauri capability harness for exercising the `lithos_las` SDK end to end.

## Purpose

This app is intentionally thin. It exists to test:

- package inspection
- session open/close
- metadata and catalog reads
- windowed curve reads
- metadata edits
- curve edits
- save / save-as
- structured validation and conflict reporting

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

The frontend is intentionally utilitarian. Its job is to exercise the SDK contract end to end, not to act as the final desktop UI.

## Short Acceptance Workflow

1. Run `bun install` in `apps/lithos-harness`.
2. Run `bun run test` to verify the mocked app-boundary smoke tests.
3. Run `cargo test --manifest-path apps/lithos-harness/src-tauri/Cargo.toml` to verify the Rust command layer.
4. Run `bun tauri dev`.
5. Open a known `.laspkg` directory and walk the checklist below.

## Manual Acceptance Checklist

1. Open a package path and inspect summary, metadata, and validation output.
2. Open a shared session and confirm session id, root, and dirty state are visible.
3. Query a curve window and confirm the JSON result updates.
4. Apply a metadata edit and confirm session dirty-state changes.
5. Apply a curve edit and confirm the session remains usable.
6. Save and verify the package reopens with the persisted changes.
7. Save-as to a new package path and confirm the session root rebinds.
8. Trigger at least one validation or save failure and confirm the structured error renders.
