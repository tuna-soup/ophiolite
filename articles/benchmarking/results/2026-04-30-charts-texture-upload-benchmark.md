# Charts Texture Upload Benchmark (2026-04-30)

Status: local exploratory baseline

Raw artifact:

- [2026-04-30-charts-texture-upload-benchmark.json](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-charts-texture-upload-benchmark.json)

Related command:

```bash
bun run charts:bench:texture-upload
```

## Purpose

This benchmark isolates the WebGL2 texture-upload part of seismic heatmap rendering. It compares the current `R32F` upload strategy with display-only packed candidates:

- `r32f`: current full-precision float texture, 4 bytes/sample
- `r16f`: half-float texture, 2 bytes/sample, packed in JavaScript for this benchmark
- `r8`: unsigned byte texture with per-section scale/bias, 1 byte/sample, packed in JavaScript for this benchmark

This is not a production renderer change. It is a measurement of the tradeoff before changing renderer behavior.

## Results

| Case | Mode | GPU bytes | Byte fraction | Pack median | Upload median | Total median | Max abs error | RMSE |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `focus-256x256` | `r32f` | `262144` | `1.00` | `0.000 ms` | `0.000 ms` | `0.000 ms` | `0` | `0` |
| `focus-256x256` | `r16f` | `131072` | `0.50` | `1.600 ms` | `0.100 ms` | `1.700 ms` | `0.000488` | `0.000103` |
| `focus-256x256` | `r8` | `65536` | `0.25` | `0.200 ms` | `0.100 ms` | `0.300 ms` | `0.003778` | `0.002179` |
| `overview-957x500` | `r32f` | `1914000` | `1.00` | `0.000 ms` | `0.200 ms` | `0.200 ms` | `0` | `0` |
| `overview-957x500` | `r16f` | `957000` | `0.50` | `11.600 ms` | `0.250 ms` | `11.800 ms` | `0.000488` | `0.000111` |
| `overview-957x500` | `r8` | `478500` | `0.25` | `1.100 ms` | `0.100 ms` | `1.200 ms` | `0.004402` | `0.002542` |
| `full-f3-small-3826x2000` | `r32f` | `30608000` | `1.00` | `0.000 ms` | `4.100 ms` | `4.100 ms` | `0` | `0` |
| `full-f3-small-3826x2000` | `r16f` | `15304000` | `0.50` | `178.200 ms` | `2.500 ms` | `181.650 ms` | `0.000488` | `0.000106` |
| `full-f3-small-3826x2000` | `r8` | `7652000` | `0.25` | `16.650 ms` | `0.700 ms` | `17.200 ms` | `0.004627` | `0.002671` |

## Interpretation

The current `R32F` path is still the best immediate full-fidelity path when the renderer receives `Float32Array` amplitudes and uploads them directly. There is no CPU packing cost, and upload time is acceptable for these fixtures.

`R16F` halves GPU bytes and slightly reduces upload time, but naive JavaScript half-float packing is far too expensive for interactive full-section updates. It is not viable as a hot-path browser-side conversion in this form. It could become interesting only if the data arrives pre-packed, is packed in native/WASM/SIMD code, or is generated as a GPU-side/display cache.

`R8 + scale/bias` cuts upload bytes to 25% and significantly reduces upload time on large sections, but CPU packing still costs more than uploading `R32F` for the full F3-small case. This is a plausible overview/LOD/display-cache format, not a replacement for canonical amplitudes.

## Recommendation

Do not switch the main heatmap renderer from `R32F` to packed textures yet.

The next useful implementation should be an optional display-cache path:

- keep chart inputs as `Float32Array`
- keep `R32F` as the exact/default renderer path
- add an opt-in `R8 + scale/bias` path only for overview/LOD tiles or cached display textures
- measure visual error with screenshots before enabling it by default
- avoid JavaScript `R16F` packing unless a faster packer exists
