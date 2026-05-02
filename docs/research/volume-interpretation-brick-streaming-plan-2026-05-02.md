# Volume Interpretation Brick Streaming Plan (2026-05-02)

Date: 2026-05-02

Scope:

- Ophiolite Charts `VolumeInterpretationChart`
- chart-side volume/slice data-source contracts
- future TraceBoost/Ophiolite runtime feeding of real `.tbvol` slices/bricks into vtk.js
- benchmark gates for 3D volume interaction

This plan applies the same lesson as the 2D section tiling work: do not make a renderer move the whole dataset when the user is looking at a small part of it. For the 3D volume chart, the correct unit is not a full `Float32Array` inside `VolumeInterpretationModel`; it is a lightweight semantic scene plus explicit handles that can load visible slices or bricks on demand.

## Reference Repos

Local checkouts:

| Repo | Local checkout | Commit | Pattern to borrow |
| --- | --- | --- | --- |
| `google/neuroglancer` | `/Users/sc/dev/neuroglancer` | `339c24f` | visible chunk priority, CPU/GPU chunk lifecycle, multiscale volume chunks, worker separation |
| `cornerstonejs/cornerstone3D` | `/Users/sc/dev/cornerstone3D` | `ad34316` | volume loader/cache handles, cacheable byte accounting, image/volume identity, viewport-driven volume assignment |
| `pyvista/pyvista` | `/Users/sc/dev/pyvista` | `cd891530a` | active scalar metadata and explicit shallow/deep copy semantics around VTK arrays |

Reference URLs:

- https://github.com/google/neuroglancer
- https://github.com/cornerstonejs/cornerstone3D
- https://github.com/pyvista/pyvista

## What We Implemented Now

Implemented the architectural foundation in Ophiolite Charts:

- Added `VolumeInterpretationScalarField`, `activeFieldId`, and `resolveActiveVolumeScalarField` so the chart can distinguish amplitude, velocity, impedance, and attribute fields without changing volume geometry.
- Added `VolumeInterpretationDataSource` with `loadSlice` and `loadBrick` hooks. This is intentionally a chart-side handle, not a `.tbvol` DTO and not a dense sample array embedded in the model.
- Added explicit `VolumeInterpretationBufferOwnership = "view" | "copy" | "transfer"` to make buffer movement visible at API boundaries.
- Added `adaptOphioliteVolumeInterpretationToChart` and a resolved-source shape for volume interpretation scenes, matching the existing survey-map/rock-physics pattern of `resolved source -> adapter -> chart model`.
- Replaced `structuredClone(model)` in `VolumeInterpretationController` with `cloneVolumeInterpretationModel`, which clones semantic objects but preserves typed-array payloads and data-source handles by reference.
- Updated the mock volume to generate a synthetic resolved Ophiolite source, pass it through the adapter, declare an active `amplitude` scalar field, and expose a slice data source. The Svelte playground volume demo now exercises `VolumeInterpretationDataSource.loadSlice` with slice-sized `f32` payloads.
- Updated the VTK renderer to render cached slice payloads from `VolumeInterpretationDataSource.loadSlice` and to name/range synthetic scalar data from the active scalar metadata.

This is a defensive change. The current demo still synthesizes sample values in the renderer, but the controller no longer has the footgun that would clone large typed arrays or fail on data-source functions once real slice/brick feeds are attached.

Smoke benchmark:

- [articles/benchmarking/results/2026-05-02-volume-interpretation-clone-boundary.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-volume-interpretation-clone-boundary.md)
- On a model with about `6.86 MiB` of typed horizon/well buffers, `cloneVolumeInterpretationModel` measured `0.0047 ms` median while `structuredClone(model)` measured `1.0956 ms` median.
- `structuredClone(modelWithDataSource)` threw `DataCloneError` because the data source contains a loader function.
- [articles/benchmarking/results/2026-05-02-volume-interpretation-slice-source-smoke.md](/Users/sc/dev/ophiolite/articles/benchmarking/results/2026-05-02-volume-interpretation-slice-source-smoke.md)
- The mock slice source returns `81,920` to `163,840` byte `f32` slice payloads with `ownership: "view"` in about `0.83` to `1.61 ms` median on this machine.

## Why This Fits Ophiolite

`VolumeInterpretationModel` should describe an interpretation scene:

- volume identity and dimensions
- sample domain and bounds
- slice planes
- horizons
- wells
- markers
- active scalar field
- optional data-source handles

It should not own large seismic sample buffers. Large buffers belong behind app/runtime data sources, cache managers, worker transfer boundaries, or renderer-internal texture resources.

The boundary is:

- Ophiolite/TraceBoost runtime owns `.tbvol`, `.tbvolc`, memory maps, and transport packing.
- The consuming app adapts runtime payloads into chart-native slice/brick payloads.
- Ophiolite Charts consumes `VolumeInterpretationDataSource` handles and normalized typed arrays.
- The renderer decides how to upload those arrays to vtk.js/WebGL textures.

## Borrowed Patterns

### Neuroglancer

Neuroglancer separates visible chunk decisions from chunk loading and GPU residency. The useful pattern for us is a chunk state machine:

- requested
- reading/fetching
- CPU memory
- GPU memory
- recent/prefetch
- evictable

For seismic interpretation, visible priority should come from active slice planes, camera view, crop box, and the selected scalar field.

### Cornerstone3D

Cornerstone keeps volume identity and loader/cache behavior explicit. The useful pattern for us is:

- stable volume ids
- cache byte accounting
- viewport-driven assignment of a cached volume to a renderer
- loader objects that can be cancelled/decached

For TraceBoost, the equivalent is a cache keyed by volume id, field id, LOD, slice/brick index, and source revision.

### PyVista

PyVista makes active arrays first-class and exposes shallow/deep copy behavior. The useful pattern for us is:

- field metadata separate from geometry
- one active scalar field
- copy behavior is explicit

For Ophiolite, that means scalar metadata lives in the chart model, while sample buffers carry `"view"`, `"copy"`, or `"transfer"` ownership.

## Product Plan

### Phase 1: Real Slice Feed Prototype

Goal:

- Feed one real inline/xline/sample slice into `VolumeInterpretationChart` through `VolumeInterpretationDataSource.loadSlice`.

Approach:

- Keep `VolumeInterpretationModel` metadata-only.
- Add a renderer-internal slice cache keyed by `volumeId:fieldId:axis:position:lod`.
- Convert a loaded slice payload into `vtkImageData` for `vtkImageMapper`.
- Keep the existing synthetic renderer path as fallback.

Benchmark:

- first slice load latency
- bytes read from runtime
- bytes copied while adapting
- vtk.js texture/upload time where measurable
- interaction frame time after cache hit

Acceptance:

- No dense sample arrays in `VolumeInterpretationModel`.
- No `structuredClone` of sample buffers.
- A cached slice move should reuse data when returning to a recent plane.

### Phase 2: Brick Feed Prototype

Goal:

- Load 3D bricks around visible slice planes and crop box instead of a full volume texture.

Approach:

- Add a brick cache keyed by `volumeId:fieldId:lod:inlineBrick:xlineBrick:sampleBrick`.
- Start with `64 x 64 x 64` bricks and one lower-resolution overview LOD.
- Prioritize bricks intersecting visible slice planes first, then crop-box interior, then camera-adjacent prefetch.
- Track CPU bytes and GPU bytes separately.

Benchmark:

- first interactive frame time
- number of bricks requested for a default scene
- cache hit rate while orbiting
- cache hit rate while dragging slice planes
- GPU upload bytes per interaction
- eviction count under a fixed memory budget

Acceptance:

- Dragging one slice plane should request only newly crossed bricks.
- Orbiting without changing slice/crop should not refetch CPU data.
- The renderer can show an overview before full-resolution bricks arrive.

### Phase 3: Worker Decode And Transfer

Goal:

- Keep decode/packing off the UI thread.

Approach:

- Use `"transfer"` ownership when the app no longer needs the buffer.
- Use `"view"` ownership for memory-mapped or shared backing buffers that must remain owned by the source.
- Avoid structured clone for large buffers.
- Add diagnostics for copied bytes, transferred bytes, and viewed bytes.

Benchmark:

- main-thread blocked time
- worker decode time
- copied/transferred/viewed byte counters
- UI frame time during slice dragging

Acceptance:

- Slice dragging remains interactive while payloads load.
- Ownership counters explain every large buffer movement.

### Phase 4: Display Format Experiments

Goal:

- Reduce GPU bytes for display without changing canonical compute data.

Approach:

- Keep source amplitudes exact where needed.
- Experiment with renderer-internal display formats:
  - `R32F` baseline
  - `R16F` if packing is cheap or source-prepacked
  - `R8 + scale/bias` for overview/preview only

Benchmark:

- upload bytes
- upload time
- packing time
- visual error against `R32F`

Acceptance:

- `R32F` remains the fallback/default until display packing wins end-to-end.
- Any lossy path is labelled display-only.

## Current Non-Goals

- Do not push TraceBoost section-tile DTOs into `VolumeInterpretationChart`.
- Do not put `.tbvol` paths or binary transport headers in Ophiolite Charts data models.
- Do not change product defaults until the real-slice prototype has benchmark evidence.
- Do not make the controller own or clone dense volume buffers.

## Next Benchmark Artifact To Capture

Add a result note once Phase 1 exists:

- `articles/benchmarking/results/YYYY-MM-DD-volume-interpretation-slice-feed.md`

Minimum contents:

- dataset path/logical id
- slice axis/index
- source format
- payload bytes
- copied/viewed/transferred bytes
- load/adapt/render timings
- cache hit behavior for returning to the same slice
