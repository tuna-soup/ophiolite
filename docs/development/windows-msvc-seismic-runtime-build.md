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

Use [`verify-windows-benchmark-env.ps1`](/C:/Users/crooijmanss/dev/ophiolite/scripts/verify-windows-benchmark-env.ps1)
to validate the current machine state before building or benchmarking.

Use [`run-benchmark-gated.ps1`](/C:/Users/crooijmanss/dev/ophiolite/scripts/run-benchmark-gated.ps1)
to serialize heavy benchmark families, pin worker counts, and wait for quieter
machine conditions before authoritative runs.

Use the manual GitHub Actions workflow
[`benchmark.yml`](/C:/Users/crooijmanss/dev/ophiolite/.github/workflows/benchmark.yml)
to run benchmarks on a dedicated self-hosted Windows runner and retrieve the
result artifacts without using the shared workstation as the measurement host.

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

## Benchmark Prep

1. Launch a fresh Visual Studio Developer PowerShell or use `windows-msvc-cargo.cmd`.
2. Point the SQLite variables at a native Windows SQLite installation.
3. Verify the environment:

```powershell
$env:OPHIOLITE_SQLITE_INCLUDE = 'C:\path\to\sqlite\include'
$env:OPHIOLITE_SQLITE_LIB_DIR = 'C:\path\to\sqlite\lib'
$env:OPHIOLITE_SQLITE_BIN_DIR = 'C:\path\to\sqlite\bin'

.\scripts\verify-windows-benchmark-env.ps1
.\scripts\verify-windows-benchmark-env.ps1 -RunCargoCheck
```

4. Only run authoritative benchmarks after the verifier reports no errors and the machine is mostly idle.

Preferred launcher examples:

```powershell
.\scripts\run-benchmark-gated.ps1 `
  -Mode authoritative `
  -BenchmarkCommandLine '.\target\debug\traceboost-app.exe benchmark-trace-local-processing C:\Users\crooijmanss\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol --scenario agc --partition-target-mib 64 --include-serial --repeat-count 3'
```

```powershell
.\scripts\run-benchmark-gated.ps1 `
  -Mode development `
  -BenchmarkCommandLine 'cargo bench -p ophiolite-seismic-runtime'
```

For the current local Windows workstation, the launcher defaults to
`OPHIOLITE_BENCHMARK_WORKERS=8` for `development` and `authoritative` runs and
uses a single heavy-benchmark lane for F3-heavy or Criterion-style work.

## CI Benchmark Flow

For cleaner benchmark evidence, prefer a dedicated self-hosted Windows runner
with the labels:

- `self-hosted`
- `windows`
- `benchmark`

The benchmark workflow is manual by design. It runs:

1. benchmark-runner environment validation
2. `verify-windows-benchmark-env.ps1`
3. an optional MSVC compile gate for bench/test surfaces
4. `run-benchmark-gated.ps1`
5. artifact upload for benchmark logs and JSON outputs

Expected repository variables on the benchmark runner:

- `OPHIOLITE_SQLITE_INCLUDE`
- `OPHIOLITE_SQLITE_LIB_DIR`
- `OPHIOLITE_SQLITE_BIN_DIR`
- `OPHIOLITE_VSDEVCMD` if the default Visual Studio path does not apply

Recommended runner-local state:

- keep the F3 benchmark store on a fixed fast local SSD path
- avoid unrelated interactive work on that host
- use one authoritative workload at a time

Representative manual dispatch inputs:

```text
mode=authoritative
benchmark_command_line=.\target\debug\traceboost-app.exe benchmark-trace-local-processing C:\Users\runneradmin\AppData\Local\Temp\ophiolite-benchmarks\f3_dataset_regularized.tbvol --scenario agc --partition-target-mib 64 --include-serial --repeat-count 3
run_compile_gate=true
```

```text
mode=development
benchmark_command_line=cargo bench -p ophiolite-seismic-runtime --bench post_stack_neighborhood_kernels
run_compile_gate=true
```

The launcher now rewrites leading Windows `cargo ...` benchmark commands through
[`windows-msvc-cargo.cmd`](/C:/Users/crooijmanss/dev/ophiolite/scripts/windows-msvc-cargo.cmd),
so Criterion and bench/test compile surfaces inherit the sanitized MSVC / SQLite
environment instead of launching raw `cargo` from an arbitrary shell.

## Current Verification Snapshot

- `tbvolc` unit tests pass under the wrapper.
- A real Waipuku `tbvol -> tbvolc -> tbvol` smoke test completed successfully.
- The restored `manifest.json` and `amplitude.bin` matched the source byte-for-byte.
- The compressed `tbvolc` output was about `1.775x` smaller than the source `tbvol`
  store on that dataset.
