# TBVOLC Direct Read Benchmark (2026-05-02)

Status: local development benchmark, not an authoritative regression threshold.

Purpose:

- test whether exact compressed `.tbvolc` should become an interactive input path
- compare direct section reads from `.tbvol` and `.tbvolc` with the same section-window scenarios
- verify `.tbvol -> .tbvolc -> .tbvol` remains byte-exact

Local run context:

- host: macOS workstation
- CPU: 10 physical / 10 logical cores
- disk headroom on `/Users/sc/dev` and `/tmp`: about `70 GiB`
- caveat: Chrome, WindowServer, and a Tauri dev process were active, so these numbers should be used for direction only
- binaries built with `CARGO_TARGET_DIR=/tmp/ophiolite-bench-target cargo build -p ophiolite-seismic-runtime --release --bin section_tile_bench --bin tbvolc_transcode`

Raw artifacts:

- [2026-05-02-poseidon-tbvol-section-direct-read.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-poseidon-tbvol-section-direct-read.json)
- [2026-05-02-poseidon-tbvolc-section-direct-read.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-poseidon-tbvolc-section-direct-read.json)
- [2026-05-02-f3-small-tbvol-section-direct-read.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-f3-small-tbvol-section-direct-read.json)
- [2026-05-02-f3-small-tbvolc-section-direct-read.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-f3-small-tbvolc-section-direct-read.json)
- [2026-05-02-tbvolc-direct-read-store-sizes.txt](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-tbvolc-direct-read-store-sizes.txt)

## Datasets

| Dataset | Shape | `.tbvol` size | `.tbvolc` size | Archive fraction | Exact round trip |
| --- | ---: | ---: | ---: | ---: | --- |
| Poseidon ROI | `256 x 256 x 256` | `64.1 MiB` | `59.2 MiB` | `92.3%` | yes, `amplitude.bin` and `occupancy.bin` |
| F3 small `DATR12I-021` | `1 x 3826 x 2000` | `30.2 MiB` | `27.1 MiB` | `89.8%` | yes, `amplitude.bin` |

Transcode timings:

| Dataset | Encode wall time | Decode wall time |
| --- | ---: | ---: |
| Poseidon ROI | `1.05 s` | `0.21 s` |
| F3 small | `0.20 s` | `0.11 s` |

## Direct Section Reads

Medians from 9 iterations.

### Poseidon ROI

| Scenario | `.tbvol` | `.tbvolc` |
| --- | ---: | ---: |
| inline full section | `0.018 ms` | `44.345 ms` |
| inline focus LOD 0 | `0.089 ms` | `44.904 ms` |
| inline focus LOD 1 | `0.027 ms` | `44.193 ms` |
| xline full section | `0.057 ms` | `44.427 ms` |
| xline focus LOD 0 | `0.090 ms` | `44.175 ms` |
| xline focus LOD 1 | `0.027 ms` | `48.512 ms` |

### F3 small

| Scenario | `.tbvol` | `.tbvolc` |
| --- | ---: | ---: |
| inline full section | `1.107 ms` | `82.710 ms` |
| inline overview fit | `0.371 ms` | `83.315 ms` |
| inline focus LOD 0 | `0.039 ms` | `11.568 ms` |
| inline focus LOD 1 | `0.012 ms` | `11.688 ms` |
| xline full section | `0.001 ms` | `11.562 ms` |
| xline focus LOD 0 | `0.001 ms` | `11.650 ms` |
| xline focus LOD 1 | `0.000 ms` | `11.606 ms` |

## Interpretation

The compressed store is exact, but it is not a good interactive input path in its current form.

The core reason is tile granularity. `.tbvol` can touch mapped `f32` samples directly. `.tbvolc` must read the compressed tile payload, decompress the full padded tile, unshuffle it, and then copy the requested section/window out of it. Even tiny output payloads inherit the full tile-decode cost.

On these two stores, `.tbvolc` saves only about `8-10%` of disk space while adding an obvious fixed decode cost to section reads. That is the wrong trade for line browsing and viewport tiles.

## Product Recommendation

Do not change production browsing or active processing to read from `.tbvolc` by default.

Keep the current product behavior:

- active working store: `.tbvol`
- optional exact archive/export sibling: `.tbvolc`
- rehydrate `.tbvolc` back to `.tbvol` before latency-sensitive interactive work

Future `.tbvolc` work should be gated behind larger-store evidence or an implementation change that reduces the fixed tile-decode cost, such as smaller compressed read units, scratch-buffer reuse, or codec variants. Based on this run, codec work alone is unlikely to justify switching production behavior.
