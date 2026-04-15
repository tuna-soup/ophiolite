# SGYX Design

Historical note: this file was imported from the predecessor `sgyx` repo. It is
retained as background design context for the I/O layer, not as the canonical
architecture document for the current Ophiolite workspace.

## Purpose

`sgyx` is a Rust library for high-throughput SEG-Y ingest. It is not trying to
be a line-slicing clone of `segyio`. The primary use case is:

- inspect a SEG-Y file quickly,
- load selected trace-header fields efficiently,
- read all trace data in large chunks,
- assemble whole cubes when geometry is regular,
- hand data off to downstream conversion targets such as HDF5.

The first implementation target is read-only SEG-Y rev1 and rev2 support, with
the optimized path focused on fixed-length traces.

## Comparison

### `segyio`

`segyio` is a broad C library with Python and Matlab bindings. It exposes
low-level SEG-Y access and higher-level geometry-driven interfaces like
`trace`, `header`, `iline`, `xline`, `depth_slice`, and whole-cube helpers. It
supports read and write flows, geometry inference, and mmap-backed access.

This makes it strong as a general SEG-Y interaction library. It is less tightly
focused on the exact workflow `sgyx` cares about: fast metadata discovery,
whole-trace ingest, and downstream bulk conversion.

### `segfast`

`segfast` is a Python acceleration layer around `segyio`. It still uses
`segyio` for file metadata, then adds:

- faster bulk trace-header loading,
- memmap-based trace loading for fixed-length traces,
- multiprocessing for large header sweeps,
- conversion helpers.

It improves ingest speed substantially, but it remains Python-centric and still
exposes slice-oriented workflows such as depth-slice access.

### `sgyx`

`sgyx` overlaps with both libraries on the core tasks:

- textual and binary header inspection,
- trace-header loading,
- trace-sample loading,
- cube assembly from regular geometry.

The main differences are intentional:

- no public iline/xline/depth-slice API,
- minimal geometry inference only where needed for whole-cube assembly,
- Rust-native performance work around chunked reads, parallel decode, and
  allocation control,
- a design shaped around full-volume ingest and format conversion rather than
  interactive seismic browsing.

## SEG-Y Model

Both SEG-Y rev1 and rev2 share the same main structure:

- 3200-byte textual file header,
- 400-byte binary file header,
- optional extended textual headers,
- repeating trace records:
  - 240-byte trace header,
  - trace sample data.

The implementation needs to understand the revision differences that affect
fast ingest:

- rev1 common path:
  - fixed-length traces,
  - standard inline and crossline header positions,
  - common formats such as IBM float, IEEE `f32`, `i16`, `i32`.
- rev2 additions:
  - explicit revision fields,
  - fixed-length trace flag,
  - larger counters and offsets,
  - optional extra trace-header blocks,
  - data trailers,
  - variable-length traces.

Implementation rule:

- fixed-length rev1 and rev2 files are the optimized path,
- variable-length traces are detected early and either rejected for the fast
  cube-ingest path or routed to a slower compatibility path later.

## API Shape

The public API should stay narrow.

### Inspection

```rust
pub fn inspect_file(path: impl AsRef<Path>) -> Result<FileSummary, InspectError>;
pub fn inspect_file_with_options(
    path: impl AsRef<Path>,
    options: InspectOptions,
) -> Result<FileSummary, InspectError>;
```

`inspect_file` should report at least:

- endianness,
- sample interval,
- samples per trace,
- sample format,
- revision,
- fixed-length flag,
- extended textual header count,
- first trace offset,
- trace size,
- trace count.

### Reader

Planned reader surface:

```rust
pub fn open(path: impl AsRef<Path>, options: ReaderOptions) -> Result<SegyReader, ReaderError>;

impl SegyReader {
    pub fn load_trace_headers(
        &self,
        fields: &[HeaderField],
        selection: TraceSelection,
    ) -> Result<HeaderTable, ReaderError>;

    pub fn read_trace_chunks(
        &self,
        config: ChunkReadConfig,
    ) -> impl Iterator<Item = Result<TraceChunk<f32>, ReaderError>>;

    pub fn read_all_traces(&self, config: ReadConfig) -> Result<TraceBlock<f32>, ReaderError>;

    pub fn assemble_cube(&self, layout: CubeLayout) -> Result<Cube<f32>, ReaderError>;
}
```

The design choice is deliberate: `sgyx` optimizes the bulk path first and does
not expose general-purpose slice navigation.

### Export handoff

`sgyx` should also expose a writer-agnostic export layer for downstream
conversion flows. That layer should:

- describe selected trace ranges without forcing immediate format-specific I/O,
- expose stable chunk descriptors for regular cubes,
- preserve sample-axis and geometry metadata needed by HDF5 or similar targets,
- avoid coupling the core reader to a specific output dependency in v1.

## Performance Strategy

### Fast metadata path

Open should start with a single small read:

1. Read 3600 bytes.
2. Parse textual and binary headers.
3. Resolve endianness, revision, sample format, sample count, fixed-length
   status, and extended textual header count.
4. Derive the first trace offset immediately.
5. If the file is fixed-length and the format is known, compute trace size and
   trace count from file size without scanning all traces.

### Bulk read path

The core trace-ingest pipeline should be:

1. positional chunk reads over contiguous trace ranges,
2. preallocated chunk buffers,
3. decode directly into `f32`,
4. parallel chunk decode with Rayon,
5. deterministic chunk order for cube assembly and export.

`mmap` should be treated as an optional fast path for profiled local-file cases,
not as the only data transport.

### Allocation and decode rules

- Reuse large chunk buffers.
- Avoid per-trace allocations.
- Use specialized decode kernels per sample format.
- Prioritize optimized support for:
  - IBM float,
  - IEEE `f32`,
  - `i16`,
  - `i32`.
- Keep the public API safe.
- Allow tightly scoped `unsafe` only in internal hotspots once benchmarks show
  it is justified.

### Geometry rules

Only infer the minimum geometry required for regular cube assembly:

- inline,
- crossline,
- offset when relevant,
- sample interval and scaling fields.

Do not build segyio-style mode objects or expose slice-axis APIs.

## Test Data Strategy

`sgyx` now carries the full `segyio/test-data` tree under `test-data/`. This is
the compatibility corpus, but not every `segyio` test should be ported.

The retained Rust validation set should focus on `sgyx` goals:

- fast inspection,
- fixed-length trace parsing,
- endianness handling,
- sample-format recognition,
- extended textual header handling,
- full-trace ingest building blocks,
- cube-assembly prerequisites.

The best candidates to reimplement in Rust are:

- `small.sgy` and `small-lsb.sgy`
- `f3.sgy` and `f3-lsb.sgy`
- `small-ps.sgy`
- `shot-gather.sgy`
- `multi-text.sgy`
- `text-embed-null.sgy`
- `long.sgy`
- the `multiformats/` fixtures
- interval and binary-header edge-case files

Lower priority or out of scope for now:

- segyio write-path tests,
- Matlab or Python binding tests,
- line/depth-slice behavior tests that `sgyx` will not expose,
- application-specific tests for `segyinfo`, `crop`, or related CLI tooling.

## Initial Bootstrap Delivered

This repo now includes:

- a Rust crate scaffold,
- a working metadata inspection API,
- a first `SegyReader` that can load selected trace headers and decode whole
  traces in chunks,
- regular post-stack and prestack cube assembly helpers,
- strict and lenient interval-validation behavior at open time,
- broader `multiformats` decode coverage across signed, unsigned, 24-bit,
  64-bit, and IEEE-double fixtures,
- a Criterion benchmark target for inspect, header loading, and bulk trace
  reads,
- reusable raw-buffer storage inside chunk iteration and direct decode into
  preallocated output buffers for full-trace reads,
- optional per-chunk parallel decode to start aligning runtime behavior with the
  high-throughput design target,
- configurable chunked header sweeps with contiguous reads and optional
  parallel field extraction,
- benchmark coverage for parallel vs sequential header scans and trace decode,
- explicit stream vs mmap backend selection for bulk trace reads and header
  sweeps, with `Auto` preferring the mmap fast path when available,
- a writer-agnostic export handoff layer for trace-range metadata, exported
  trace chunks, cube metadata, and cube chunk descriptors for downstream
  conversion,
- the copied `segyio` fixture corpus,
- curated Rust integration tests anchored to known `segyio` invariants.

Current explicit gap:

- format 4 remains unsupported for data reads until a justified decode strategy
  is chosen and validated against the corpus.

That is the base layer, not the full library.

## Completion Criteria

`sgyx` should not be considered finished until all of the following are true:

- the curated fixture set has Rust integration tests,
- Linux and Windows test runs pass,
- bulk read APIs exist for selected headers and chunked trace reads,
- the optimized fixed-length path works for the retained rev1 and rev2 cases,
- malformed and lenient-mode behavior is explicitly tested,
- performance regressions are tracked for inspect, header sweep, and bulk trace
  ingest.

## References

- `SEG-Y-Format-rev-1.pdf`
- `seg_y_rev2_0_mar2017.pdf`
- `../segyio/README.md`
- `../segyio/python/segyio/open.py`
- `../segyio/python/segyio/tools.py`
- `../segfast/README.md`
- `../segfast/segfast/segyio_loader.py`
- `../segfast/segfast/memmap_loader.py`
