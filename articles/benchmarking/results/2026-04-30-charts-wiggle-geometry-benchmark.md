# Charts Wiggle Geometry Benchmark (2026-04-30)

Status: local exploratory baseline

Raw artifact:

- [2026-04-30-charts-wiggle-geometry-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-wiggle-geometry-benchmark.json)

Related command:

```bash
bun run charts:bench:wiggle-geometry
```

## Purpose

This benchmark compares two wiggle rendering representations:

- `expanded`: the previous main local WebGL renderer path, which prepared CPU line/fill vertices and uploaded large dynamic buffers
- `instanced`: the worker-renderer representation, which uploads compact per-trace instance data and samples amplitudes from the existing texture in the shader
- `instanced-cached`: the same instanced path with the visible amplitude max already cached for the current section/window

## Results

| Case | Mode | Prepare median | Wiggle upload bytes | Drawn traces | Cache prime |
| --- | --- | ---: | ---: | ---: | ---: |
| `focus-256x256` | `expanded` | `2.998 ms` | `1695488` | `128` | n/a |
| `focus-256x256` | `instanced` | `0.426 ms` | `1536` | `128` | n/a |
| `focus-256x256` | `instanced-cached` | `0.031 ms` | `1536` | `128` | `0.427 ms` |
| `overview-957x500` | `expanded` | `11.060 ms` | `3784144` | `192` | n/a |
| `overview-957x500` | `instanced` | `2.896 ms` | `2304` | `192` | n/a |
| `overview-957x500` | `instanced-cached` | `0.058 ms` | `2304` | `192` | `2.775 ms` |
| `full-f3-small-3826x2000` | `expanded` | `86.390 ms` | `18214880` | `226` | n/a |
| `full-f3-small-3826x2000` | `instanced` | `43.629 ms` | `2712` | `226` | n/a |
| `full-f3-small-3826x2000` | `instanced-cached` | `0.130 ms` | `2712` | `226` | `44.302 ms` |
| `zoomed-f3-512x512` | `expanded` | `7.801 ms` | `3085520` | `171` | n/a |
| `zoomed-f3-512x512` | `instanced` | `1.535 ms` | `2052` | `171` | n/a |
| `zoomed-f3-512x512` | `instanced-cached` | `0.042 ms` | `2052` | `171` | `1.475 ms` |

## Interpretation

The instanced representation is a clear production win for the local WebGL path. It reduces wiggle geometry upload from megabytes to a few kilobytes and improves preparation time in every measured first-draw case.

The full-section first draw still pays for a visible-amplitude scan to compute the global wiggle scale. The cached mode represents repeated redraws of the same section/window, where that scan is reused: the full F3-small case drops from `43.629 ms` to `0.130 ms` while keeping the same `2712` byte instance upload.

## Production Decision

The main local WebGL renderer was updated to use instanced wiggle rendering, matching the worker renderer's representation. The local and worker WebGL paths now also cache visible wiggle amplitude max by section/window, so repeated redraws do not rescan the same viewport just to preserve scale.

Follow-up optimization candidates:

- compute min/max or max-abs during section tile assembly and transport it as display metadata
- add a browser visual regression check for wiggle mode parity between local WebGL and worker WebGL
