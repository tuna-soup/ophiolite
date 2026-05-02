# Benchmark Results

This folder stores benchmark outputs and short result notes that are useful as future comparison baselines.

Most files are raw JSON artifacts from runtime or application benchmark commands. Markdown files in this folder should be concise summaries of runs that were exploratory but still decision-relevant.

## Current Notes

- [2026-04-30-render-storage-optimization-baseline.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-render-storage-optimization-baseline.md): quick baseline for section tile reads and `.tbvolc` transcode behavior after reviewing external render/storage optimization strategies.
- [2026-04-30-f3-small-traceboost-desktop-observation.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-f3-small-traceboost-desktop-observation.md): desktop F3/F3-small observations from TraceBoost logs and screenshots, including forced viewport-tile zero-copy evidence, cache-reuse smoke data, and the follow-up need to remeasure narrower `128`-sample tile windows.
- [2026-04-30-charts-texture-upload-benchmark.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-texture-upload-benchmark.md): WebGL2 upload benchmark comparing `R32F`, JavaScript-packed `R16F`, and `R8 + scale/bias` seismic display textures.
- [2026-04-30-charts-wiggle-geometry-benchmark.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-wiggle-geometry-benchmark.md): CPU preparation and upload-size benchmark comparing expanded wiggle vertices, instanced wiggle rendering, and warm-cache instanced redraws.
- [2026-05-02-volume-interpretation-clone-boundary.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-volume-interpretation-clone-boundary.md): smoke benchmark showing why the 3D volume controller should preserve typed-array/data-source handles instead of using `structuredClone(model)`.
- [2026-05-02-volume-interpretation-slice-source-smoke.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-volume-interpretation-slice-source-smoke.md): smoke benchmark for the mock `VolumeInterpretationDataSource.loadSlice` path now used by the Svelte playground volume demo.

## Optimization Evidence Map

These results are not only compression tests:

- Storage/IO: `.tbvol` section reads and `.tbvolc` exact archive transcodes.
- App transport: packed-section typed-array views that avoid app-side buffer copies.
- Interactive viewport tiling: zoomed section views can request smaller trace/sample windows, reuse cached tiles while panning, and preserve viewport state across line browsing.
- Display upload: renderer-internal texture packing candidates for display-only paths.
- Renderer geometry: instanced wiggle rendering and visible-scale caching.
- 3D volume interpretation groundwork: metadata-only volume scenes, scalar-field handles, explicit buffer ownership, and controller clone boundaries that preserve data-source handles instead of cloning large typed arrays.

Canonical seismic amplitudes remain `f32` in the compute/runtime path unless a future experiment explicitly defines a separate display-only or lossy-preview contract.

The 3D volume benchmark plan is tracked in [docs/research/volume-interpretation-brick-streaming-plan-2026-05-02.md](/Users/sc/dev/ophiolite/docs/research/volume-interpretation-brick-streaming-plan-2026-05-02.md). Once real slice or brick feeding lands, add a dedicated result note with payload bytes, copied/viewed/transferred bytes, cache hit behavior, and render timings.

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
