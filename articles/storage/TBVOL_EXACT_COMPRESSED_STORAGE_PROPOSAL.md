# Exact Compressed tbvol Storage Proposal

## Executive summary

The benchmark result is now clear enough to support a concrete product proposal.

Do not replace active `tbvol`.

Instead:

- keep today's uncompressed `tbvol` as the default hot working store
- add a separate optional exact compressed sibling format for colder storage-sensitive workflows
- start with tile-wise `bitshuffle + lz4` and `bitshuffle + zstd`

In plain terms:

- when the user wants the fastest local preview and apply loop, keep using `tbvol`
- when the user wants the same data exactly but smaller on disk, offer an optional compressed variant

That gives the product a storage lever without destabilizing the runtime path that already works.

## Implementation status

Phase 1 now has a first runtime prototype.

Implemented shape:

- a new exact compressed sibling store `tbvolc`
- offline `tbvol -> tbvolc` and `tbvolc -> tbvol` transcode
- per-tile amplitude compression with `lz4`
- a runtime-owned grouped bitshuffle preconditioner recorded as `bitshuffle_g8`

This is intentionally close to the benchmark direction, but not a direct Blosc2 binding.

Reason:

- the current Rust runtime workspace did not already expose a clean native Blosc2 path
- Phase 1 is about proving the exact transcode and file-shape story first

So the immediate goal is correctness and product fit, not exact codec parity with the external benchmark harness.

## Why this proposal exists

The storage benchmark work now supports three important conclusions.

1. Uncompressed tiled `tbvol` is still the right active compute substrate.
2. Exact compression is useful enough to be interesting.
3. Exact compression is not free enough to make it the default hot path.

On the real F3 `tbvol`, the strongest exact candidates were:

- `tile-blosc2-lz4-bitshuffle`: about `1.97x`
- `tile-blosc2-zstd-bitshuffle`: about `2.11x`

Those are real savings, but they still introduce decompression cost before compute.

That points to a two-tier design rather than one format that tries to be best at everything.

## The product decision

### What we should build

Build an optional exact compressed storage tier adjacent to `tbvol`.

### What we should not build

Do not:

- replace default active `tbvol`
- redesign the processing pipeline around always-compressed tiles
- start a custom codec project
- make the first version depend on OpenVDS+, SGZ, or any lossy path

## Recommended shape

### Separate sibling format, not `tbvol` mutation

The cleanest first move is a separate sibling format rather than changing what `tbvol` means.

Reason:

- current `tbvol` semantics are simple: fixed-size full-trace tiles, direct mmap, predictable offsets
- compressed tiles need a tile index and variable payload sizes
- mixing those semantics into the existing active store would make the hot path harder to reason about

Recommended format marker:

- `tbvolc`

Meaning:

- "tbvol-compatible geometry and metadata"
- "compressed amplitude payloads"
- "exact-lossless only in v1"

This keeps the runtime contract clean:

- `tbvol` means direct fixed-stride active working store
- `tbvolc` means exact compressed sibling optimized for storage, import/export, and colder access

### Shared logical model

`tbvolc` should preserve the same logical volume model as `tbvol`:

- same volume metadata
- same axes
- same tile shape philosophy
- same tile grid
- same full-sample-axis tile requirement
- same occupancy semantics when present

That keeps round-trip and transcode behavior simple.

## Proposed on-disk structure

Directory layout:

```text
example.tbvolc/
  manifest.json
  amplitude.index.bin
  amplitude.bin
  occupancy.bin            optional, still raw if present in v1
```

### Manifest additions

The manifest should be explicit about encoding.

Suggested shape:

```json
{
  "format": "tbvolc",
  "version": 1,
  "volume": { "...": "same logical metadata as tbvol" },
  "tile_shape": [82, 56, 462],
  "tile_grid_shape": [8, 17],
  "sample_type": "f32",
  "endianness": "little",
  "has_occupancy": false,
  "amplitude_encoding": {
    "codec": "native",
    "compressor": "lz4",
    "filters": ["bitshuffle_g8"],
    "compression_level": null,
    "lossless": true
  },
  "amplitude_tile_sample_count": 2121504,
  "tile_count": 136
}
```

### Tile index

Compressed tiles are variable sized, so the store needs an index.

Recommended index entry:

```text
offset_u64
length_u32
stored_ci_u16
stored_cx_u16
reserved_u32
```

This supports:

- direct seek to one tile payload
- edge tiles with logical shapes smaller than the padded tile shape
- future metadata expansion without redoing the file structure immediately

### Payload policy

For v1:

- amplitude tiles are compressed independently
- tile payload order matches tile-grid iteration order
- occupancy remains raw if present

Keeping occupancy raw in v1 avoids introducing compression work where the space win is likely minor.

## Runtime behavior

### Open behavior

Opening `tbvolc` should:

1. read `manifest.json`
2. map or read the tile index
3. lazily decompress only requested tiles

### Preview behavior

For preview, the runtime should:

- read only the tiles that intersect the requested inline or xline
- decompress each tile into a reusable scratch buffer
- assemble the requested section exactly as today

### Full apply behavior

For full apply, the first productized version should not run in-place over compressed output.

Recommended v1 behavior:

- input may be `tbvol` or `tbvolc`
- output of active processing remains uncompressed `tbvol`

That preserves the current processing assumptions and makes compressed input support low risk.

### Transcode behavior

Add explicit transcode commands:

- `tbvol -> tbvolc`
- `tbvolc -> tbvol`

That is the simplest useful feature boundary.

## Recommended codecs for v1

### Primary

- `blosc2 + bitshuffle + lz4`

Use this as the first implementation because it best matches the speed-sensitive side of the benchmark result.

### Secondary

- `blosc2 + bitshuffle + zstd`

Add this once the basic storage tier works, for users who care more about disk reduction than decode speed.

### What to exclude from v1

- `fpzip`
- custom predictors
- lossy options
- codec auto-selection

## User-facing product shape

### Suggested workflows

1. Import to active `tbvol`
2. Optionally archive or duplicate to exact compressed `tbvolc`
3. Rehydrate back to `tbvol` when the user wants a fast working copy

### Candidate UI language

- `Optimize storage (exact)`
- `Restore fast working copy`

Avoid calling this "quantization" or "advanced compression" in the product.

In user terms, the feature is:

- smaller exact storage
- slower direct access than the fast working copy

## Acceptance bar

The feature should not ship just because the benchmark can produce smaller files.

Recommended acceptance gates:

1. On at least two real regularized 3D volumes, `tbvolc-lz4` should achieve about `1.5x` or better exact reduction.
2. `tbvolc-zstd` should beat `tbvolc-lz4` on ratio by a useful margin.
3. Opening and previewing directly from `tbvolc-lz4` should stay operationally acceptable for colder workflows.
4. `tbvol -> tbvolc -> tbvol` round-trips must be byte-exact for amplitudes and metadata-stable where expected.
5. Processing from `tbvolc` input must produce the same derived amplitudes as `tbvol` input.

## Implementation plan

### Phase 1: file format and offline transcoder

Scope:

- add `tbvolc` manifest type
- add tile index writer and reader
- add `tbvol -> tbvolc`
- add `tbvolc -> tbvol`
- support only `lz4` first

Success criterion:

- exact round-trip on F3 and at least one more real dataset

### Phase 2: runtime read support

Scope:

- add `TbvolcReader`
- allow section preview from `tbvolc`
- allow full apply with `tbvolc` input and `tbvol` output
- keep a scratch-buffer pool to avoid repeated allocation

Success criterion:

- direct `tbvolc` preview works correctly and remains acceptable for colder workflows

### Phase 3: second codec and product plumbing

Scope:

- add `zstd`
- add app actions for optimize-storage and restore-working-copy
- add volume metadata labels that show working-store versus compressed-store role

Success criterion:

- the user can manage exact compressed copies without touching the hot path by accident

## Risks and how to control them

### Risk: people accidentally process from compressed storage and blame the runtime

Mitigation:

- keep `tbvol` as the default output of ingest and processing
- label `tbvolc` clearly as storage-optimized

### Risk: index and payload logic add complexity

Mitigation:

- keep `tbvolc` as a sibling format with a narrow initial scope
- do not overload current `TbvolReader`

### Risk: compression benefit varies by survey

Mitigation:

- expose measured estimated savings before conversion when possible
- validate on more than one real dataset before product commitment

## What this means in layman's terms

The simplest description is:

- keep the current fast working copy exactly as it is
- add a smaller exact packed copy only for when disk space matters more than direct speed

That is the balanced move.

It gives TraceBoost a credible storage feature without making the existing processing path slower or more fragile for everyone.

## Recommendation

Proceed with Phase 1 only.

Do not build a compressed-by-default runtime store.

Do not start custom codec research from here.

Build a narrow exact compressed sibling format, prove the transcode and preview story, and then decide whether it is valuable enough to expose in the product.
