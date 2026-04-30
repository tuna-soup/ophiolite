# Benchmark Results

This folder stores benchmark outputs and short result notes that are useful as future comparison baselines.

Most files are raw JSON artifacts from runtime or application benchmark commands. Markdown files in this folder should be concise summaries of runs that were exploratory but still decision-relevant.

## Current Notes

- [2026-04-30-render-storage-optimization-baseline.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-render-storage-optimization-baseline.md): quick baseline for section tile reads and `.tbvolc` transcode behavior after reviewing external render/storage optimization strategies.
- [2026-04-30-f3-small-traceboost-desktop-observation.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-f3-small-traceboost-desktop-observation.md): desktop F3-small full-section observation from TraceBoost logs and screenshots; useful smoke context, but not yet a viewport-tile adaptation benchmark.
- [2026-04-30-charts-texture-upload-benchmark.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-texture-upload-benchmark.md): WebGL2 upload benchmark comparing `R32F`, JavaScript-packed `R16F`, and `R8 + scale/bias` seismic display textures.
- [2026-04-30-charts-wiggle-geometry-benchmark.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-wiggle-geometry-benchmark.md): CPU preparation and upload-size benchmark comparing expanded wiggle vertices, instanced wiggle rendering, and warm-cache instanced redraws.

## Optimization Evidence Map

These results are not only compression tests:

- Storage/IO: `.tbvol` section reads and `.tbvolc` exact archive transcodes.
- App transport: packed-section typed-array views that avoid app-side buffer copies.
- Display upload: renderer-internal texture packing candidates for display-only paths.
- Renderer geometry: instanced wiggle rendering and visible-scale caching.

Canonical seismic amplitudes remain `f32` in the compute/runtime path unless a future experiment explicitly defines a separate display-only or lossy-preview contract.

## Raw Artifacts

- [2026-04-30-poseidon-section-tile-baseline.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-poseidon-section-tile-baseline.json)
- [2026-04-30-f3-small-section-tile-baseline.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-f3-small-section-tile-baseline.json)
- [2026-04-30-tbvolc-transcode-smoke.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-tbvolc-transcode-smoke.json)
- [2026-04-30-packed-section-adaptation-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-packed-section-adaptation-benchmark.json)
- [2026-04-30-charts-texture-upload-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-texture-upload-benchmark.json)
- [2026-04-30-charts-wiggle-geometry-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-wiggle-geometry-benchmark.json)

## Capture Guidelines

When adding a result note, include:

- date and machine context when known
- exact command or benchmark entry point
- dataset path or logical dataset name
- key metrics, preferably medians for latency
- interpretation in plain terms
- whether the result is authoritative, exploratory, or a local smoke baseline

Do not treat exploratory notes as regression thresholds until the same command, dataset, and environment have been repeated enough to establish normal variance.
