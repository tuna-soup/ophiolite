# Section Tiling, Viewport Caching, and Interactive Section Browsing Optimizations

## Purpose

This note records the next performance layer added on top of the earlier TraceBoost/Ophiolite work on:

- packed binary section transport
- frontend decode and chart synchronization costs
- preview prefix reuse
- processing-cache exact rerun reuse
- `tbvol` as the active seismic runtime format

The focus here is narrower:

- make `SeismicSection` browsing in the real TraceBoost desktop app feel faster
- avoid moving full-section payloads when the user is only inspecting a small viewport
- prefetch adjacent slices because interpreters usually browse locally coherent neighborhoods
- preserve viewport state while stepping between lines

This article also maps the changes back to the concepts discussed in `technical_architecture_deep_dive.md` so it is clear what has now been addressed, what has only been partially addressed, and what still remains open.

## Why this work mattered

Even after the earlier binary transport and frontend decode fixes, one expensive pattern remained:

- open an inline or xline
- zoom into a smaller area
- step to the next nearby line
- still move or reconstruct more section payload than the current view really needs

That is exactly the kind of interaction where a mature subsurface viewer should stop thinking in terms of "full section every time" and start thinking in terms of:

- viewport-sized working sets
- snapped tile windows
- adjacent-slice prefetch
- small in-process caches

For a local-first desktop app, the goal is not abstract elegance. The goal is for the user to feel that line flicking is immediate enough that they stay in interpretation flow.

## What changed in this iteration

## 1. Viewport-driven section tile requests

TraceBoost now requests section tiles from the runtime based on the current chart viewport rather than always relying on the full section payload.

The request shape is built from:

- the visible trace range
- the visible sample range
- a halo around the viewport
- snapped bucket boundaries
- a request-time LOD

The resulting section tile is represented as a windowed section payload rather than a full-section payload.

## 2. Section-tile binary backend path

The desktop backend now exposes a packed binary section-tile command instead of routing viewport requests through the older full-section path.

That means the runtime can return:

- only the requested trace/sample window
- only the requested decimated sampling step for the chosen LOD
- only the bytes needed for the current interaction

This keeps the hot interactive path aligned with the earlier packed binary transport work rather than regressing to array-heavy payloads.

## 3. In-process section tile cache with LRU-style eviction

The TraceBoost app now keeps recently used section tiles in memory under a size budget and reuses them when the next viewport request falls inside the same snapped window.

This is intentionally an in-process local cache, not an external cache service.

That matches the product shape:

- local desktop app
- local files
- small interaction locality windows
- no need to pay network or service complexity to cache a few recent section tiles

## 4. Adjacent-slice prefetch

When a viewport tile is fetched for the active line, the app immediately begins prefetching the same tile window for the neighboring line indices.

That is a domain-specific optimization, not a generic one. It is based on the real browsing pattern:

- interpreters often move one inline or xline at a time
- they usually keep looking in the same neighborhood
- the next likely request is often `index - 1` or `index + 1`

## 5. Request-time LOD for section tiles

The viewport request now chooses a decimation level from the visible density of traces and samples on screen.

This is not yet a precomputed multiresolution pyramid on disk. It is request-time downsampling from the active `tbvol` using a stride derived from the requested `lod`.

That distinction matters:

- it gives us a practical first step toward multires behavior
- it does not yet give us the import-time multiresolution hierarchy recommended in the original architecture memo

## 6. Viewport preservation across line changes

Before this fix, stepping to a new inline or xline could reset the chart to full extents.

That was wrong for interpretation flow. The user has already expressed intent by zooming or panning. The viewer should not discard that context just because the section index changed.

The behavior is now:

- keep the current viewport when the incoming section has compatible logical dimensions
- only force a full chart reset when the real display context changes, such as dataset/domain/velocity-overlay state

## 7. Always-on diagnostics for section-tiling behavior

The desktop session log now records:

- active viewport tile fetches
- cache hits
- adjacent-line prefetches
- prefetch failures
- cache trimming events
- payload size
- elapsed time
- tile ranges
- viewport ranges
- cache counters

This is temporary engineering instrumentation, but it is always on for this work because hidden diagnostics are much less useful during iteration.

## Where the code lives

The placement follows the repo boundaries reasonably well.

### Runtime / Ophiolite

- `crates/ophiolite-seismic-runtime`
- tile-window section reads
- request-time LOD sampling
- section-tile benchmark binary

### Charts

- `charts/packages/svelte`
- preserve viewport when compatible section payloads are swapped in
- reuse decoded payloads where possible

### TraceBoost app shell

- `apps/traceboost-demo`
- viewport-to-tile request orchestration
- section-tile cache
- adjacent-slice prefetch policy
- diagnostics logging
- chart overlay diagnostics

That division is important. The chart package should not own survey-specific cache policy or desktop-only prefetch orchestration. The app owns that.

## Measurement environment

The measurements in this note were taken on:

- OS: `macOS 26.3.1`
- architecture: `arm64`
- CPU: `Apple M1 Pro`
- RAM: `16 GiB`

The GUI measurements were taken in the actual Tauri desktop TraceBoost app, not in a browser dev server.

## Datasets used

The main dataset used for the new section-tiling measurements was the real imported F3 runtime store:

- source survey: `f3_dataset.sgy`
- active runtime store shape: `[651, 951, 462]`
- `tbvol` tile shape: `[82, 56, 462]`

That shape matters because it reflects the existing `tbvol` choice:

- tiles span the full sample axis
- trace-local and section assembly operations do not need to stitch partial traces along sample depth

## Measurement method

Two measurement paths were used.

## 1. Actual desktop GUI session logs

We used structured TraceBoost session logs from real browsing sessions in the Tauri desktop app.

The useful fields were:

- `elapsedMs`
- `duration_ms`
- `payloadBytes`
- `traceRange`
- `sampleRange`
- `viewportTraceRange`
- `viewportSampleRange`
- `cacheHits`
- `fetches`
- `prefetchRequests`

These logs are the best source for user-facing interaction timing because they include:

- frontend orchestration
- Tauri IPC
- runtime load
- prefetch behavior

## 2. Runtime-only `section_tile_bench`

We also ran:

```bash
cargo run -p ophiolite-seismic-runtime --bin section_tile_bench --release -- \
  --store '<f3 tbvol path>' \
  --axis both \
  --iterations 7 \
  --focus-traces 256 \
  --focus-samples 256 \
  --screen-traces 1200 \
  --screen-samples 900 \
  --focus-lod 0,1 \
  --format json
```

This isolates runtime section assembly from desktop frontend overhead and gives a better picture of the storage/read side alone.

## Benchmarks

## A. Desktop GUI session: full-extent first view versus zoomed viewport

Two session logs are worth separating:

### Full-extent initial viewport

From `traceboost-session-1776694694020-65280.log`:

- active viewport fetch elapsed time: about `47 ms`
- backend section-tile load: about `9-10 ms`
- payload: `1,766,904 bytes`
- viewport range: full inline extent `[0, 951) x [0, 462)`
- adjacent prefetch elapsed time: about `42-48 ms`

This is roughly the "cold full-screen section" case.

### Zoomed viewport on the same dataset

From `traceboost-session-1776694354830-62861.log`:

- active viewport fetch elapsed time: about `15 ms`
- backend section-tile load: about `3 ms`
- payload: `476,984 bytes`
- viewport range: `[88, 209) x [119, 205)` inside a snapped tile window `[0, 256) x [0, 462)`
- adjacent prefetch elapsed time: about `29-30 ms`

Relative to the full-extent inline payload:

- payload dropped from `1,766,904` bytes to `476,984` bytes
- that is about a `73%` reduction in transferred payload
- or about a `3.7x` smaller payload

That is the main practical win of this work. Once the user zooms into a local area, the app no longer needs to keep behaving as if the full section is equally important.

## B. Runtime-only release benchmark on the same imported F3 store

### Inline median timings

| Scenario | Output size | Payload fraction of full | Median time |
| --- | ---: | ---: | ---: |
| Full section | `951 x 462` | `100%` | `0.490 ms` |
| Focus tile LOD 0 | `256 x 256` | `15.0%` | `0.197 ms` |
| Focus tile LOD 1 | `128 x 128` | `3.8%` | `0.186 ms` |

### Xline median timings

| Scenario | Output size | Payload fraction of full | Median time |
| --- | ---: | ---: | ---: |
| Full section | `651 x 462` | `100%` | `1.040 ms` |
| Focus tile LOD 0 | `256 x 256` | `21.9%` | `0.353 ms` |
| Focus tile LOD 1 | `128 x 128` | `5.5%` | `0.251 ms` |

These numbers are much smaller than the desktop GUI numbers because:

- the benchmark is release-mode runtime only
- there is no Tauri IPC
- there is no frontend state update
- repeated iterations are warm

That does not make them less useful. It tells us the runtime path itself is already cheap enough that most of the user-visible cost is now in the app-layer interaction path rather than raw tile assembly.

## Engineering interpretation

The combination of the desktop logs and runtime benchmark suggests:

1. The runtime tile-read path is already fast on local `tbvol`.
2. The main practical user-facing win comes from reducing payload size and avoiding unnecessary full-section work.
3. Adjacent-slice prefetch is justified because the next line usually arrives before the user changes location entirely.
4. A zoomed viewport now moves materially less data than a full section and feels correspondingly faster.

## What concepts from the original architecture memo have now been addressed?

## Addressed

### Chunking

Yes.

`tbvol` already uses a tiled physical layout, and the active F3 store used here has tile shape `[82, 56, 462]`.

### Tiling

Yes.

This work added viewport-sized section tiles on top of the existing store tiles, including halo expansion and snapped bucket ranges.

### Caching

Yes.

Multiple cache layers now exist in the stack:

- processing exact-rerun cache
- same-session preview prefix reuse
- chart decode cache
- app-level section tile cache
- implicit OS page cache through `mmap`

### Lazy loading

Yes.

The desktop app now requests section data on demand from the visible viewport rather than always leaning on full-section payloads.

### Prefetching

Yes.

Adjacent-slice prefetch is now part of the active desktop browsing path, and lower-level SEG-Y reader code already includes trace-chunk prefetch paths in the ingest layer.

### Compute-friendly versus visualization-friendly intermediate formats

Yes, at the current product level.

We are explicitly using:

- raw SEG-Y for interchange and provenance
- `tbvol` as the optimized local runtime/compute store
- viewport/windowed section payloads as the interaction-time representation

That is already a practical three-layer model rather than trying to make one format do everything.

## Partially addressed

### Multiresolution / LOD

Partially.

We now support request-time LOD for section tiles, but we do **not** yet build and persist true multiresolution pyramids during import.

So we have the beginnings of multires behavior, not a full multires storage system.

### Streaming

Partially.

The current desktop path is on-demand and binary, but it is not yet a true progressive streaming renderer that incrementally refines one viewport from coarse to fine chunks inside a single request cycle.

### GPU/rendering optimization

Partially.

This work reduces what reaches the renderer and keeps the existing window-aware rendering path in play, but it does **not** yet introduce:

- a GPU-resident tile atlas
- persistent texture cache management
- explicit GPU-side multires orchestration

### Spatial indexing

Partially.

Regular `tbvol` geometry and tile coordinates are enough for inline/xline section-window access, but we have not added a richer spatial index for arbitrary planes, horizon-constrained sampling, or more complex 3D lookup patterns.

## Not yet addressed in a serious way

### Import-time multiresolution pyramids

Still open.

This is the clearest missing architecture item if we later want faster overview navigation on very large surveys or remote/object-backed stores.

### Progressive coarse-to-fine rendering

Still open.

We currently fetch one chosen tile window at one chosen LOD. We do not yet show coarse data immediately and then refine the same viewport progressively.

### GPU-resident cache policy

Still open.

For the current local desktop product shape, CPU-side caching plus existing renderer behavior is good enough to move forward, but a commercial chart library will eventually want a more explicit GPU upload/cache strategy.

### Dedicated slice cache

Still mostly open.

Right now the app caches windowed section tiles. A higher-level ready-to-display slice cache may still help if repeated identical view states become common.

## How this lines up with the earlier articles

This work is consistent with the earlier conclusions rather than replacing them.

## From `SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md`

Still true:

- `tbvol` remains the right active local compute store
- full-sample-axis tiles remain a good choice for the current operator class

## From `SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md`

Still true:

- exact compressed storage remains attractive as an optional colder tier
- it should not replace the hot uncompressed active store by default

## From `PROCESSING_CACHE_ARCHITECTURE_AND_BENCHMARKING.md`

Still true:

- exact full-rerun reuse is worth keeping
- hidden whole-volume prefix checkpointing is not the right optimization target

The new work reinforces that lesson. The best locality wins are currently:

- viewport-local
- line-local
- session-local

not whole-volume hidden intermediates.

## Should we address more architecture right now?

My current answer is: mostly no, with two targeted follow-ups worth considering.

## Reasonable next priorities

### 1. Keep the current path and benchmark it more rigorously

We should add a repeatable benchmark harness for desktop browsing scenarios so we can compare builds directly rather than inferring regressions from ad hoc session logs.

This is the most important next step because the current behavior is already directionally good enough that accidental regressions become the bigger risk.

### 2. Consider import-time multires pyramids only if survey size or deployment shape demands it

If we move toward:

- larger surveys
- slower storage
- object storage
- remote data access
- much larger chart canvases

then import-time multiresolution levels become much more attractive.

For the current local `tbvol` desktop path, request-time LOD plus viewport tiling is a good first stopping point.

## Things I would *not* prioritize immediately

- replacing `tbvol`
- adding a remote cache service
- building a complex hidden persistent tile cache
- rewriting the renderer around a new GPU cache architecture before the current interaction path is benchmarked more thoroughly

## Bottom line

This iteration materially improved the local desktop `SeismicSection` browsing path in the place users feel it most:

- zoom into a smaller area
- step through nearby lines
- keep the same viewport
- move less data
- prefetch the next likely slices

That means we have now touched the most important concepts from the original architecture note:

- chunking
- tiling
- caching
- lazy loading
- prefetching
- partial LOD / multires behavior

The main architecture items still left open are not blockers for the current local desktop product:

- true import-time multires pyramids
- progressive streaming refinement
- richer GPU-resident cache policy
- more advanced spatial indexing

Those are worth revisiting later, but they are not prerequisites for shipping a meaningfully faster section-browsing experience today.
