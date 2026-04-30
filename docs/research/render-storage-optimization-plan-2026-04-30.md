# Render And Storage Optimization Plan (2026-04-30)

Date: 2026-04-30

Scope:

- Ophiolite seismic runtime storage and section access
- Ophiolite Charts seismic renderer and future 3D seismic viewer paths
- TraceBoost demo packed transport, viewport tile cache, and app-side integration

This note follows the memory lesson from the vertex/texture discussion: optimize by representation and access pattern, not by assuming that one asset class is inherently cheaper. A million well-packed vertices can be cheaper than one full-resolution float texture; a full-fidelity seismic tile can be correct for compute while a smaller display texture is enough for an interactive view.

## External References Checked

Cloned into `/Users/sc/dev/ophiolite-optimization-references`:

| Repo | Local checkout | Useful pattern |
| --- | --- | --- |
| `equinor/seismic-zfp` | `seismic-zfp @ 5c8a121` | seismic-specific chunk cache, compressed chunk reads, subplane/trace-range access |
| `google/neuroglancer` | `neuroglancer @ 2343c14` | visible chunk prioritization, worker/GPU chunk lifecycle, 3D volume bricking |
| `Kitware/vtk-js` | `vtk-js @ dfbee4c` | WebGL image/volume mapper structure and texture LOD download path |
| `niivue/niivue` | `niivue @ 503c207` | browser volume viewer texture/upload patterns |
| `zeux/meshoptimizer` | `meshoptimizer @ c835967` | vertex/index buffer packing, meshlets, simplification, mesh payload codecs |

Already present in `/Users/sc/dev`:

| Repo | Local checkout | Useful pattern |
| --- | --- | --- |
| `c-blosc2` | `c-blosc2 @ ede15dde` | chunked compressed frame design, shuffle/bitshuffle filters, indexed chunk offsets |
| `zarrs` | `zarrs @ 4ff63e77` | chunk grids, sharding, partial subchunk decode/cache |
| `TileDB` | `TileDB @ 28aeb653d` | dense array storage architecture and query planning ideas |
| `arrow-rs` | `arrow-rs @ d49f017fe` | typed buffer ownership, zero-copy IPC-style data movement |

Reference URLs:

- https://github.com/equinor/seismic-zfp
- https://github.com/google/neuroglancer
- https://github.com/Kitware/vtk-js
- https://github.com/niivue/niivue
- https://github.com/zeux/meshoptimizer
- https://github.com/Blosc/c-blosc2
- https://github.com/zarrs/zarrs

## Local Baseline Snapshot

Commands were run from `/Users/sc/dev/ophiolite` with release binaries.

Captured result note:

- [articles/benchmarking/results/2026-04-30-render-storage-optimization-baseline.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-04-30-render-storage-optimization-baseline.md)

Repeatable commands:

- `bun run bench:render-storage-baseline`
- `bun run traceboost:bench:packed-section`
- `bun run charts:bench:texture-upload`
- `bun run charts:bench:wiggle-geometry`

| Dataset | Shape | Store size | Case | Median |
| --- | ---: | ---: | --- | ---: |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | inline full section | `0.142 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | xline full section | `0.300 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | inline focus LOD 1 | `0.114 ms` |
| Poseidon ROI `.tbvol` | `256 x 256 x 256` | `64M` | xline focus LOD 1 | `0.196 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline full section | `7.808 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline overview fit, LOD 2 | `1.602 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline focus LOD 0 | `0.273 ms` |
| F3 small `DATR12I-021.tbvol` | `1 x 3826 x 2000` | `30M` | inline focus LOD 1 | `0.220 ms` |

Compression baseline:

| Source | Output | Encode | Decode | Exactness |
| --- | ---: | ---: | ---: | --- |
| Poseidon ROI `64M` `.tbvol` | `59M` `.tbvolc` | `0.81 s` | `0.24 s` | `amplitude.bin` and `occupancy.bin` byte-exact |
| F3 small `30M` `.tbvol` | `27M` `.tbvolc` | `0.12 s` | not run | not checked |

Interpretation:

- Current mmap-backed `.tbvol` section reads are already fast on these small/medium local stores.
- Current exact `.tbvolc` compression is useful but modest on these samples. It should be expanded only with benchmark evidence from larger volumes and direct compressed-read paths.
- The app-layer packed-section benchmark confirmed that typed-array adaptation copies were worth removing: the copy path moved about `30.7 MB` for a full F3-small section, while the typed-view path copied `0` bytes in the synthetic benchmark.
- The texture-upload benchmark showed that display packing is not automatically a win. `R8 + scale/bias` reduces GPU bytes to 25% and full-section upload time from about `4.1 ms` to `0.7 ms`, but JavaScript packing raises total time to about `17.2 ms`. Naive JavaScript `R16F` packing is too slow for hot-path use.
- The wiggle-geometry benchmark was convincing enough for a production change: the instanced path reduced `zoomed-f3-512x512` wiggle upload bytes from about `3.1 MB` to about `2 KB` and first-draw preparation time from about `7.8 ms` to about `1.5 ms`. Cached redraw preparation for the full F3-small section is about `0.13 ms` after the visible amplitude max has been measured once.
- The more obvious remaining near-term optimization area is not raw section read speed. It is bytes between app model, worker, and GPU upload.

## Existing Ophiolite Touchpoints

| Area | Current state | Optimization signal |
| --- | --- | --- |
| `.tbvol` runtime store | mmap-backed f32 amplitudes, full sample-axis tiles, borrowed tile slices in `TbvolReader` | good baseline; preserve as exact compute/reference format |
| `.tbvolc` archive store | native lossless `lz4 + bitshuffle_g8`, tile index, exact round trip | candidate for codec variants, direct preview reads, scratch-buffer pooling |
| Packed section transport | binary header plus typed byte payloads | already avoids JSON float arrays; can reduce app-side typed-array copies |
| Viewer app model | viewport-driven tile fetch and tile diagnostics exist | add benchmarkable cache/prefetch policy and copy counters |
| Charts heatmap renderer | uploads amplitudes as WebGL2 `R32F` texture | display-only `R16F`, `R16_SNORM`, or `R8 + scale/bias` experiments can cut upload/GPU bytes |
| Charts main wiggle renderer | local and worker WebGL paths use instanced wiggle rendering with visible-scale cache | next target is parity screenshots and carrying max-abs metadata from upstream tiles |
| Future 3D seismic viewer | currently not a Neuroglancer-style chunk manager | adopt bricked 3D texture cache and priority states before large-volume rendering |
| Horizon/well/mesh overlays | likely smaller than seismic volumes but can grow | meshoptimizer is relevant for 3D geometry payloads, not for amplitude samples |

## Optimization Plan

### Phase 0: Benchmark Gates First

Add one script or documented target set that runs:

- `section_tile_bench` on Poseidon ROI, F3 small, and a scheduled larger F3 stress store.
- `.tbvol -> .tbvolc -> .tbvol` transcode timing and byte-exact compare.
- A browser/app benchmark that records section payload bytes, typed-array adaptation time, GPU upload bytes, and frame time.
- A renderer benchmark scene for heatmap and wiggle modes with fixed dimensions and trace/sample counts.

Acceptance:

- Patches must include before/after numbers for the touched path.
- Exact paths must remain byte-exact unless the experiment is explicitly display-only or lossy-preview-only.
- Hot `.tbvol` section reads must not regress by more than 5% median on the same machine and dataset.

### Phase 1: Low-Risk Bytes And Copies

Target:

- `apps/traceboost-demo/src/lib/transport/packed-sections.ts`
- `apps/traceboost-demo/src/lib/viewer-model.svelte.ts`
- chart worker transfer boundaries

Experiments:

- Replace `source.buffer.slice(...)` typed-array decoding with zero-copy typed views when the `Uint8Array` alignment and full-buffer ownership permit it.
- Preserve transfer ownership across worker boundaries when possible instead of cloning typed-array backing buffers.
- Add diagnostics for decode/adapt time, payload bytes, and copied bytes.

Why first:

- The storage baseline is already fast, but the app currently copies amplitude and axis buffers while adapting binary transport into chart data.
- This is exact and reversible; no numeric compromise is involved.

Benchmarks:

- Browser microbench for `parsePackedSectionTileResponse -> adaptTransportWindowedSectionToChartData`.
- App viewport tile load timing already emitted by the viewer model.
- Correctness check comparing decoded arrays against the current copy-based path.

### Phase 2: Display Texture Packing

Target:

- `charts/packages/renderer/src/seismic/mock/MockCanvasRenderer.ts`
- `charts/packages/renderer/src/seismic/mock/baseRenderWorker.ts`

Experiments:

- Keep runtime and chart contract amplitudes as `Float32Array`.
- Add renderer-internal display packing modes:
  - baseline: `R32F`
  - candidate: `R16F`
  - candidate: signed normalized `R16` with per-section scale/bias
  - candidate: `R8` with per-window scale/bias for overview-only displays
- Move color mapping and normalization into shader uniforms where it reduces CPU work and upload bytes.

Current evidence:

- `R32F` remains the default full-fidelity path because it has no browser-side packing cost.
- JavaScript-packed `R16F` should not be used in the hot path without a faster packer or pre-packed source.
- `R8 + scale/bias` is promising only as an overview/LOD/cache format where reduced upload bytes matter more than packing cost and bounded display error is acceptable.

Why:

- A 4k `R32F` texture is 64 MB before considering mips or duplicates. `R16` halves that; `R8` quarters it.
- This is the closest direct application of the tweet, but it must remain display-only because seismic compute should not silently quantize source amplitudes.

Benchmarks:

- GPU upload bytes and upload time.
- Frame time on heatmap pan/zoom.
- Pixel-difference/PSNR check against `R32F`.
- Visual acceptance screenshots for representative dynamic range cases.

### Phase 3: Wiggle Geometry Instancing

Target:

- Main `MockCanvasRenderer` path that previously prepared and uploaded CPU line/fill vertices.
- Existing worker instanced wiggle path as the model.

Experiments:

- Replace per-sample CPU-expanded line/fill buffers with trace instances plus amplitude texture sampling.
- Keep a fallback for unsupported contexts.

Why:

- The current main path expands wiggles into large dynamic vertex buffers. The worker path already proves an instanced representation with roughly per-trace metadata plus texture sampling.
- This mirrors the vertex-packing lesson: store compact instance parameters and let the shader reconstruct the display geometry.

Benchmarks:

- CPU preparation time.
- Dynamic buffer upload bytes.
- Frame time for `256 x 256`, `1200 x 900`, and full F3 small views.
- Visual parity for normal/reversed polarity and gain changes.

Current status:

- Implemented for the main local WebGL renderer.
- Visible max-amplitude caching is implemented in both local and worker WebGL paths.
- Remaining work is visual parity validation and carrying section/window max-abs metadata from tile assembly so first-draw scale computation can avoid a full viewport scan.

### Phase 4: `.tbvolc` Codec And Direct-Read Experiments

Target:

- `crates/ophiolite-seismic-runtime/src/storage/tbvolc.rs`
- `crates/ophiolite-seismic-runtime/src/bin/section_tile_bench.rs`
- `crates/ophiolite-seismic-runtime/src/bin/tbvolc_transcode.rs`

Experiments:

- Add codec parameterization for lossless variants:
  - current `lz4 + bitshuffle_g8`
  - `zstd + bitshuffle_g8`
  - optional no-filter controls
- Add scratch-buffer pooling for repeated compressed tile decode.
- Add direct compressed-store section preview benchmarks before making `.tbvolc` an interactive path.
- Consider a separate lossy preview tier only after exact `.tbvolc` evidence is understood. If tried, name it differently and keep compute paths exact.

Why:

- Blosc/Zarr-style chunk indexes and filters are relevant, but current ratios are only 8-10% on the quick samples. The idea may still win on larger stores or different amplitude distributions; it needs measured proof.

Benchmarks:

- Archive size fraction.
- Encode/decode throughput.
- Section read latency from compressed source.
- Allocations per tile read.
- Exact round-trip compare.

### Phase 5: 3D Seismic Viewer Chunk Manager

Target:

- Future 3D seismic viewer/view model.
- Ophiolite runtime section/brick serving APIs.
- Chart renderer internals only for rendering, not for `.tbvol` ownership.

Experiments:

- Use a Neuroglancer-style chunk lifecycle:
  - queued
  - downloading/reading
  - CPU memory
  - worker memory
  - GPU memory
  - evictable recent/prefetch
- Use bricked 3D textures with anisotropic chunk options:
  - `64 x 64 x 64` for true 3D volume interaction
  - section-friendly slabs for inline/xline/time-slice interaction
- Prioritize visible chunks, lower-resolution fallback chunks, then prefetch.

Why:

- A 3D seismic viewer will fail by trying to upload "the volume" as a single texture. The correct unit is a visible brick with explicit memory budgeting.

Benchmarks:

- Time to first visible section.
- Orbit/pan frame latency.
- GPU memory residency.
- Chunk hit/miss rate.
- Eviction churn under scripted navigation.

### Phase 6: Mesh Payload Packing

Target:

- Horizon surfaces, well trajectories, fault meshes, and dense interpretation overlays.

Experiments:

- Use meshoptimizer for vertex/index reordering, quantization, simplification, and optional meshlet generation.
- Apply only to geometry payloads, not seismic amplitude arrays.

Why:

- The tweet's vertex half applies most directly here: geometry should be packed as geometry, not over-promoted to huge textures or verbose JSON arrays.

Benchmarks:

- Mesh bytes before/after.
- Decode time.
- Render time for representative horizon/well scenes.
- Picking accuracy after quantization.

## Priority Recommendation

1. Treat Phase 1 zero-copy packed-section adaptation as implemented for the TraceBoost demo path. Real viewport-tile timing and copy counters now exist in the running app; next capture those counters into a named benchmark artifact from a real desktop dataset.
2. Treat Phase 3 local WebGL wiggle instancing and visible-scale caching as implemented. A Svelte playground local-WebGL wiggle screenshot baseline now exists; next extend it to worker WebGL, normal/reversed polarity, and gain changes.
3. Keep Phase 2 display texture packing experimental. `R8 + scale/bias` is promising for overview/LOD caches, but the current JavaScript packing cost does not justify replacing the default `R32F` hot path.
4. Carry precomputed min/max or max-abs metadata from tile assembly into chart payloads where it can remove first-draw visible-scale scans without changing canonical `f32` amplitudes.
5. Keep Phase 4 `.tbvolc` work behind benchmark gates until larger-volume ratios and direct compressed-section reads justify more codec work.
6. Design the 3D viewer around Phase 5 from the start; retrofitting chunk lifecycle after a monolithic texture renderer would be expensive.
7. Use meshoptimizer opportunistically for interpretation geometry, not as a seismic volume optimization.

## Non-Goals

- Do not change canonical runtime amplitude samples from `f32` to a display-packed format.
- Do not put dense seismic payloads into chart manifests.
- Do not make Ophiolite Charts own `.tbvol`, `.tbvolc`, or application cache policy.
- Do not introduce lossy seismic compression without a separate format/contract and visible user-facing semantics.
