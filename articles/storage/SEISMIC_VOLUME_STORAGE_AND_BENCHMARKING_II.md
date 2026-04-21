# Seismic Volume Storage, Benchmarking, and Backend Conclusions II

## Why this follow-up exists

Part I explained why TraceBoost/Ophiolite moved to tiled uncompressed `tbvol` for active seismic compute.

This follow-up asks a narrower question:

- can we shrink seismic volumes exactly, with no sample loss, in a way that is still useful for the real runtime?

The answer is partly yes, but not in the way the more specialized floating-point compression literature first suggested.

## The plain-language conclusion

If you strip away the codec names and benchmark details, the findings are simple.

1. `tbvol` is still the right active processing format.
2. Exact compression can save a meaningful amount of disk space.
3. That same compression also adds read and compute overhead.
4. The practical winners were not the fanciest compressors. The best trade-off came from `bitshuffle + lz4` and `bitshuffle + zstd`.
5. That points toward a two-tier future:
   - uncompressed `tbvol` for active processing
   - optionally compressed exact storage for colder or more storage-sensitive workflows

In other words:

- for speed, keep today's `tbvol`
- for smaller exact storage, add a compressed tier later if it is worth the engineering complexity

## What we tested

The compression study stayed exact-lossless throughout. No quantization, no discarded detail, no silent approximation.

Real datasets used:

- F3 runtime store already materialized as `tbvol`
- `3D-Waihapa.sgy` converted to a benchmark-only dense trace-order proxy because standard 3D headers were zeroed
- `3D-Waipuku.sgy` converted the same way

Candidate families tested:

- `fpzip`
- `blosc2-lz4-bitshuffle`
- `blosc2-zstd-bitshuffle`
- a benchmark-only reversible trace-wise XOR preconditioner ahead of the same Blosc pipelines
- tile-wise Blosc compression that mirrors the current `tbvol` full-trace tile layout

What was not proven here:

- `ndzip` was not benchmarked on this machine because the available Windows environment lacked the Clang and Boost prerequisites it expects
- OpenVDS wavelet compression remained out of scope because the public build available locally does not support compressed VDS creation

## What we found

### 1. Synthetic results were misleading for `fpzip`

On smooth synthetic cubes, `fpzip` looked excellent.

On real seismic data, it did not hold up consistently.

- On F3, `fpzip` compressed worse than the Blosc baselines and was dramatically slower to read back.
- On Waihapa and Waipuku it sometimes improved ratio, but the decompression cost remained much higher than the practical alternatives.

That means `fpzip` is not the right front-runner for this workload even though it looked attractive in the first synthetic pass.

### 2. The boring exact baselines won

Across the real datasets, the best operational candidates were:

- `blosc2-lz4-bitshuffle`
- `blosc2-zstd-bitshuffle`

The split is straightforward:

- `lz4` is the speed-oriented option
- `zstd` is the storage-oriented option

### 3. A simple custom reversible transform did not justify itself

A benchmark-only trace-wise XOR transform was tested before Blosc compression.

It was exact and reversible, but in practice:

- compression ratio improved only slightly
- decompression and apply costs got materially worse

That is not a strong basis for custom codec investment.

### 4. Compression survives `tbvol`-style tiling

The most important practical finding is that the exact storage benefit did not disappear when compression was applied tile-by-tile in a `tbvol`-like layout.

That matters because whole-volume compression numbers are less relevant to the real runtime than tile-local numbers.

For F3, tile-wise compression using the current `tbvol` tile geometry still landed near the same ratio band as whole-volume compression:

- `tile-blosc2-lz4-bitshuffle`: about `1.97x`
- `tile-blosc2-zstd-bitshuffle`: about `2.11x`

So the storage gain is real even when measured in a layout that looks like the store the runtime actually uses.

### 5. Tile-shape tuning was not the main lever

A small F3 sweep was run around the current full-trace tile layout:

- current F3 tile shape: `[82, 56, 462]`
- alternates tested: `[32, 32, 462]`, `[64, 64, 462]`, `[96, 48, 462]`

The result:

- the compression ratio stayed effectively unchanged at about `1.966x` for tile-wise `lz4`

That suggests the current full-sample-axis tile philosophy is the important design choice. Fine-grained tile retuning is not the main source of storage gain here.

## The most useful real numbers

### F3 real `tbvol`

Logical shape:

- `[651, 951, 462]`

Key exact results:

| Codec | Ratio | Decompress ms | Apply ms |
| --- | ---: | ---: | ---: |
| `blosc2-lz4-bitshuffle` | `1.966x` | `793` | `1247` |
| `blosc2-zstd-bitshuffle` | `2.106x` | `1120` | `1564` |
| `fpzip` | `1.210x` | `13071` | `15969` |

Tile-wise `tbvol`-style simulation on the same dataset:

| Codec | Ratio | Decompress ms | Apply ms |
| --- | ---: | ---: | ---: |
| `tile-blosc2-lz4-bitshuffle` | `1.966x` | `951` | `1155` |
| `tile-blosc2-zstd-bitshuffle` | `2.107x` | `1086` | `1628` |

### Waihapa trace-order proxy

Logical shape:

- `[227, 305, 2501]`

Key exact results:

| Codec | Ratio | Decompress ms | Apply ms |
| --- | ---: | ---: | ---: |
| `blosc2-lz4-bitshuffle` | `1.169x` | `574` | `786` |
| `blosc2-zstd-bitshuffle` | `1.246x` | `775` | `1099` |
| `fpzip` | `1.371x` | `8978` | `7720` |

### Waipuku trace-order proxy

Logical shape:

- `[148, 312, 2001]`

Key exact results:

| Codec | Ratio | Decompress ms | Apply ms |
| --- | ---: | ---: | ---: |
| `blosc2-lz4-bitshuffle` | `1.814x` | `326` | `352` |
| `blosc2-zstd-bitshuffle` | `1.924x` | `295` | `396` |
| `fpzip` | `1.867x` | `3139` | `3351` |

## What this means for `tbvol`

This is the most important practical section.

### What `tbvol` is good at today

Today's `tbvol` is good because it is simple and predictable.

- every tile spans the full sample axis
- the runtime can memory-map one binary amplitude file
- the compute path touches dense contiguous trace data
- preview and full apply use the same simple tile iteration model

That is why it works well as the active processing substrate.

### What compression could improve

Compression could help `tbvol` in one clear way:

- it can reduce disk footprint meaningfully, often toward roughly half-size on the tested F3-style workload

That has obvious value for:

- laptops with limited SSD capacity
- duplicated derived stores
- cold storage of intermediate versions
- shipping or syncing exact datasets between machines

### What compression would cost

Compression also changes the character of the active store.

Instead of directly touching numeric tiles in place, the runtime would have to:

1. read the compressed bytes
2. decompress the tile
3. then run the operator pipeline

That means:

- more CPU before compute even starts
- more temporary allocations or scratch buffers
- more implementation complexity
- more room for performance regressions in preview workflows

So even when compression is exact, it is not free.

### The practical impact on current processing

For the current processing pipeline, the findings suggest:

- do not replace uncompressed active `tbvol` by default
- if compression is introduced, it should begin as an optional storage mode, not as the universal runtime default

In plain terms:

- the current store is optimized to let the CPU get to the numbers quickly
- a compressed store is optimized to make the file smaller
- those are related goals, but not the same goal

## Recommended product direction

### Near-term recommendation

Keep current uncompressed `tbvol` as the default runtime processing store.

Reason:

- it still fits the preview/materialization model best
- none of the tested exact alternatives justified changing the hot path

### If storage pressure matters

If the product needs a smaller exact storage tier, the most credible candidates are:

- `tile-blosc2-lz4-bitshuffle` for a speed-friendlier exact compressed store
- `tile-blosc2-zstd-bitshuffle` for a smaller but slower exact compressed store

The most plausible shape is not "replace `tbvol`."

It is:

- keep active `tbvol` uncompressed
- allow optional exact compressed export, cache, archive, or cold-store variants

The concrete follow-up proposal is documented in `TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md`.

### What not to do next

Based on the evidence so far, the following should be deprioritized:

- treating `fpzip` as the main path
- building a custom codec immediately
- adopting the trace-wise XOR preconditioner in anything like its current form

## Final summary

In layman's terms:

- the current `tbvol` layout is still the right working format for processing
- exact compression can reduce storage enough to be interesting
- the best exact answers were simple chunked byte-level compressors, not the fancier scientific compressors we initially suspected
- that means the likely future is a split between a fast working store and a smaller exact storage tier, not one magic format that is best at everything

That is a useful outcome.

It means the system does not need a heroic reinvention. It needs a disciplined decision about whether storage savings are important enough to justify an optional compressed tier around the `tbvol` model that already works.
