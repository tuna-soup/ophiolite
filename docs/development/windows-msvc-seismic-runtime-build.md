# Windows MSVC Build Notes for Seismic Runtime

The seismic runtime currently needs a sanitized MSVC build shell on Windows when
`proj-sys` is built from source.

## Why

- `proj-sys` can accidentally pick up MSYS / MinGW headers and tools from
  `PATH`, which breaks the bundled PROJ build.
- The bundled PROJ build also needs a native SQLite toolchain:
  - `sqlite3.h`
  - `sqlite3.lib` / `libsqlite3.a`
  - `sqlite3.exe`
- Static PROJ on Windows also needs `ole32` and `shell32` during final link.

The runtime crate handles the Windows system libraries via
[`build.rs`](/C:/Users/crooijmanss/dev/ophiolite/crates/ophiolite-seismic-runtime/build.rs).

## Helper Script

Use [`windows-msvc-cargo.cmd`](/C:/Users/crooijmanss/dev/ophiolite/scripts/windows-msvc-cargo.cmd)
to run cargo in a cleaned MSVC environment.

Required environment variables:

- `OPHIOLITE_SQLITE_INCLUDE`: directory containing `sqlite3.h`
- `OPHIOLITE_SQLITE_LIB_DIR`: directory containing `sqlite3.lib` or `libsqlite3.a`

Optional environment variables:

- `OPHIOLITE_SQLITE_BIN_DIR`: directory containing `sqlite3.exe`
- `OPHIOLITE_VSDEVCMD`: override path to `VsDevCmd.bat`

## Example

```powershell
$env:OPHIOLITE_SQLITE_INCLUDE = 'C:\path\to\sqlite\include'
$env:OPHIOLITE_SQLITE_LIB_DIR = 'C:\path\to\sqlite\lib'
$env:OPHIOLITE_SQLITE_BIN_DIR = 'C:\path\to\sqlite\bin'

.\scripts\windows-msvc-cargo.cmd test -p ophiolite-seismic-runtime tbvolc --lib -- --nocapture
```

## Verified Commands

```powershell
.\scripts\windows-msvc-cargo.cmd test -p ophiolite-seismic-runtime tbvolc --lib -- --nocapture
.\scripts\windows-msvc-cargo.cmd run -p ophiolite-seismic-runtime --bin tbvolc_transcode -- encode C:\path\to\input.tbvol C:\path\to\output.tbvolc
.\scripts\windows-msvc-cargo.cmd run -p ophiolite-seismic-runtime --bin tbvolc_transcode -- decode C:\path\to\input.tbvolc C:\path\to\output.tbvol
```

## Current Verification Snapshot

- `tbvolc` unit tests pass under the wrapper.
- A real Waipuku `tbvol -> tbvolc -> tbvol` smoke test completed successfully.
- The restored `manifest.json` and `amplitude.bin` matched the source byte-for-byte.
- The compressed `tbvolc` output was about `1.775x` smaller than the source `tbvol`
  store on that dataset.
