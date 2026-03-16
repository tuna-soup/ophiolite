# lithos

`lithos` is a Rust-first LAS asset SDK for subsurface log data.

The current implementation focuses on LAS 1.2 and LAS 2.0 as ingest formats, then converts them into a local per-file asset bundle that is easier for desktop applications to inspect and query than raw LAS text files.

## What It Does

- Imports LAS 1.2 and 2.0 files into an in-memory asset model.
- Preserves raw header sections, curve metadata, provenance, and ingest issues.
- Exposes an app-facing model for asset summaries, index metadata, curve metadata, and selective curve reads.
- Writes a per-file optimized bundle with a JSON manifest and binary curve columns.
- Re-opens stored bundles for metadata inspection and windowed curve reads.
- Provides a small CLI for import and inspection workflows.

## CLI

```bash
cargo run -- import <input.las> <bundle_dir>
cargo run -- inspect-file <input.las>
cargo run -- summary <bundle_dir>
cargo run -- list-curves <bundle_dir>
cargo run -- read-curve <bundle_dir> <curve_id> [start end]
```

## Roadmap

### Already Implemented

- Rust crate with a LAS parser, asset model, storage bundle layer, and CLI.
- Tolerant parsing for common LAS 1.2/2.0 shapes including wrapped and unwrapped data.
- Structured ingest warnings for imperfect but importable files.
- Additive canonical aliases for common curve mnemonics without overwriting source values.
- Binary column storage plus manifest-based bundle metadata.
- Integration tests against the sample LAS files in `test_data/`.

### Next

- Integrate the crate behind Tauri commands for a Rust backend and JS frontend.
- Add richer metadata/query DTOs shaped for UI consumption.
- Support metadata-only open paths that avoid touching curve payloads entirely.
- Improve unit and mnemonic normalization beyond the current small alias set.
- Add more validation and repair diagnostics for malformed LAS inputs.

### Later

- LAS 3 support with proper handling of section groups and multi-dataset files.
- More storage/indexing options for larger local libraries of imported assets.
- Export flows and controlled round-tripping back to LAS where needed.
- Broader subsurface abstractions only after the LAS asset layer is stable.

## Verification

```bash
cargo fmt --check
cargo test
```

