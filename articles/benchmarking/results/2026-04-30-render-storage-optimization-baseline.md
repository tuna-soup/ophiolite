# Render And Storage Optimization Baseline (2026-04-30)

Status: local exploratory baseline

Related plan:

- [docs/research/render-storage-optimization-plan-2026-04-30.md](/Users/sc/dev/ophiolite/docs/research/render-storage-optimization-plan-2026-04-30.md)

Raw artifacts:

- [2026-04-30-poseidon-section-tile-baseline.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-poseidon-section-tile-baseline.json)
- [2026-04-30-f3-small-section-tile-baseline.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-f3-small-section-tile-baseline.json)
- [2026-04-30-tbvolc-transcode-smoke.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-tbvolc-transcode-smoke.json)
- [2026-04-30-packed-section-adaptation-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-packed-section-adaptation-benchmark.json)
- [2026-04-30-charts-texture-upload-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-texture-upload-benchmark.json)
- [2026-04-30-charts-wiggle-geometry-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-wiggle-geometry-benchmark.json)

Related observations:

- [2026-04-30-f3-small-traceboost-desktop-observation.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-f3-small-traceboost-desktop-observation.md)

Purpose:

- establish a small baseline after reviewing texture, mesh, chunked-volume, and compressed-storage optimization strategies
- decide whether the immediate optimization target should be `.tbvol` reads, `.tbvolc` compression, app/transport copies, renderer upload size, or 3D chunk management

## Capture Commands

Repeat the runtime/storage capture with:

```bash
bun run bench:render-storage-baseline
```

Repeat the app packed-section adapter benchmark with:

```bash
bun run traceboost:bench:packed-section
```

Repeat the charts texture upload benchmark with:

```bash
bun run charts:bench:texture-upload
```

Repeat the charts wiggle geometry benchmark with:

```bash
bun run charts:bench:wiggle-geometry
```

## Underlying Runtime Commands

Runtime section read baseline:

```bash
target/release/section_tile_bench \
  --store /Users/sc/Downloads/SubsurfaceData/poseidon/far_stack_roi_inline0_xline128_sample128_v3.tbvol \
  --axis both \
  --iterations 7 \
  --screen-traces 1200 \
  --screen-samples 900 \
  --focus-traces 256 \
  --focus-samples 256 \
  --focus-lod 0,1 \
  --format json
```

```bash
target/release/section_tile_bench \
  --store /Users/sc/Downloads/SubsurfaceData/blocks/F3/seismic/tbvol/DATR12I-021.tbvol \
  --axis both \
  --iterations 7 \
  --screen-traces 1200 \
  --screen-samples 900 \
  --focus-traces 256 \
  --focus-samples 256 \
  --focus-lod 0,1 \
  --format json
```

Exact archive transcode smoke baseline:

```bash
/usr/bin/time -p target/release/tbvolc_transcode encode \
  /Users/sc/Downloads/SubsurfaceData/poseidon/far_stack_roi_inline0_xline128_sample128_v3.tbvol \
  /tmp/poseidon-v3.tbvolc
```

```bash
/usr/bin/time -p target/release/tbvolc_transcode decode \
  /tmp/poseidon-v3.tbvolc \
  /tmp/poseidon-v3-roundtrip.tbvol
```

```bash
/usr/bin/time -p target/release/tbvolc_transcode encode \
  /Users/sc/Downloads/SubsurfaceData/blocks/F3/seismic/tbvol/DATR12I-021.tbvol \
  /tmp/DATR12I-021.tbvolc
```

Poseidon round trip was checked with `cmp -s` for both `amplitude.bin` and `occupancy.bin`.

## Section Read Results

| Dataset | Shape | Store size | Scenario | Median |
| --- | ---: | ---: | --- | ---: |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | inline full section | `0.142 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | inline overview fit | `0.159 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | inline focus LOD 0 | `0.158 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | inline focus LOD 1 | `0.114 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | xline full section | `0.300 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | xline overview fit | `0.445 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | xline focus LOD 0 | `0.401 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | xline focus LOD 1 | `0.196 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline full section | `7.808 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline overview fit, LOD 2 | `1.602 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline focus LOD 0 | `0.273 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline focus LOD 1 | `0.220 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | xline full section | `0.145 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | xline overview fit, LOD 2 | `0.145 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | xline focus LOD 0 | `0.142 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | xline focus LOD 1 | `0.145 ms` |

## `.tbvolc` Results

| Source | Output | Encode wall time | Decode wall time | Exactness |
| --- | ---: | ---: | ---: | --- |
| Poseidon ROI `64M` `.tbvol` | `59M` `.tbvolc` | `0.81 s` | `0.24 s` | `amplitude.bin` and `occupancy.bin` byte-exact |
| F3 small `30M` `.tbvol` | `27M` `.tbvolc` | `0.12 s` | not run | not checked |

## Packed Section Adaptation Results

This synthetic app-layer benchmark compares the previous copy-style decode behavior with the new typed-view decode path used by `section-adapter.ts`.

| Case | Payload | Mode | Total median | Copied bytes | Viewed bytes |
| --- | ---: | --- | ---: | ---: | ---: |
| `focus-256x256` | `267680` bytes | copy | `0.029 ms` | `267264` | `0` |
| `focus-256x256` | `267680` bytes | view | `0.003 ms` | `0` | `267264` |
| `overview-957x500` | `1931728` bytes | copy | `0.115 ms` | `1931312` | `0` |
| `overview-957x500` | `1931728` bytes | view | `0.001 ms` | `0` | `1931312` |
| `full-f3-small-3826x2000` | `30677640` bytes | copy | `1.286 ms` | `30677216` | `0` |
| `full-f3-small-3826x2000` | `30677640` bytes | view | `0.002 ms` | `0` | `30677216` |

## Interpretation

In basic terms, the existing `.tbvol` reader is already very quick for these local stores. Reading a focused viewport tile is already in the sub-millisecond range, and even the larger F3 inline full-section case is under 8 ms.

The current exact `.tbvolc` archive path works and is fast to transcode on these samples, but the size reduction is modest: roughly `64M -> 59M` and `30M -> 27M`. That makes `.tbvolc` worth keeping and measuring, but it does not yet look like the first place to spend optimization effort for interactive browsing.

The packed-section benchmark confirms that the data movement after storage is worth optimizing. The storage read can be sub-millisecond, while a full-section app adaptation copy can move about `30.7 MB` before the chart sees the data. The typed-view path removes that copy when the packed response is aligned, which is the normal Tauri response shape.

The TraceBoost demo now records real viewport-tile fetch/adaptation metrics through the same path used by the running app. The Section Tiling overlay and `section_tile` diagnostics include fetch time, adaptation time, copied bytes, viewed bytes, and copied/viewed buffer counts for the most recent viewport tile. A debug-only `Force viewport tiles` control can force the tile path even when the currently loaded full section already covers the viewport.

The first F3-small desktop screenshots/logs captured after adding the overlay still show the full-section path rather than a viewport tile request: `Full section`, `0 viewport`, and `Adapt pending`. The log is still useful as a full-section desktop smoke observation: five inline loads had median `48 ms` frontend await time and median `81 ms` total frontend load-to-second-frame time for `951 x 462` sections.

The texture-upload benchmark adds a second constraint: reducing GPU bytes is useful only if packing does not dominate. `R8 + scale/bias` reduced full-section upload time from about `4.1 ms` to `0.7 ms`, but JavaScript packing made total time about `17.2 ms`. `R16F` upload size and error look attractive, but JavaScript half-float packing made it unusable as a hot-path conversion.

The wiggle benchmark is more actionable. The instanced representation cut `zoomed-f3-512x512` wiggle upload bytes from about `3.1 MB` to about `2 KB` and reduced first-draw preparation from about `7.8 ms` to about `1.5 ms`. For the full F3-small case it cut upload bytes from about `18.2 MB` to about `2.7 KB`; after caching the visible amplitude max for the current section/window, repeated redraw preparation drops from about `43.6 ms` to about `0.13 ms`.

The strongest next candidates are now:

- an optional `R8 + scale/bias` display cache for overview/LOD textures, not a default replacement for `R32F`
- carrying precomputed min/max or max-abs display metadata through section tile assembly and transport
- future 3D viewer chunk lifecycle and GPU residency policy

## Next Benchmark Gaps

- Capture and archive real desktop viewport timing from the new Section Tiling adaptation counters on F3 small with `Force viewport tiles` enabled.
- Add a renderer benchmark that compares `R32F` against display-only `R16` or `R8 + scale/bias` texture modes.
- Extend the new Svelte playground wiggle visual baseline to worker WebGL, reversed polarity, and gain-change cases.
- Add direct `.tbvolc` section-read benchmarking before treating compressed stores as interactive inputs.
