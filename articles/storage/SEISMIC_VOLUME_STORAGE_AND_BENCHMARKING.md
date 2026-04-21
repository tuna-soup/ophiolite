# Seismic Volume Storage, Benchmarking, and Backend Conclusions

Status note, April 2026:

- This article is Part I of the storage story.
- It explains why the runtime converged on uncompressed tiled `tbvol` as the active compute store.
- The exact-lossless compression follow-up is documented separately in `SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md`.

## Audience and intent

This note is for senior software engineers working on seismic runtime systems, local-first scientific applications, and storage-backed compute pipelines. It documents why the TraceBoost/Ophiolite seismic stack moved away from a Zarr-first runtime design for compute-heavy workflows, what storage shapes were evaluated, how the benchmark was structured, and why `tbvol` is now the preferred runtime backend.

This is not a product overview. It is an engineering writeup about physical layout, logical layout, I/O costs, compute costs, and the consequences of those choices for section preview and full-volume materialization.

## Problem statement

The application needs to support two closely related workflows:

1. Fast preview of an operator pipeline on the currently displayed 2D seismic section.
2. Materialization of that same pipeline over the entire imported 3D seismic volume.

The first requirement is latency-sensitive. The second is throughput-sensitive. The same operator kernels must serve both paths, otherwise preview and full apply diverge semantically and operationally.

That leads to three backend constraints:

- Section preview cannot require loading the full volume.
- Full-volume apply cannot require whole-volume in-memory materialization.
- The persistent runtime store must support both selective reads and high-throughput streamed writes.

The initial runtime implementation used Zarr-backed arrays. That was a reasonable starting point for a multidimensional volume store, but it was not obvious that Zarr would remain the best substrate once the runtime started doing real compute instead of mostly ingest and display.

## The logical data model

The seismic runtime uses a dense regularized cube as its canonical compute substrate:

- logical shape: `[iline, xline, sample]`
- value type: `f32`
- optional occupancy mask for sparse/empty bins

This is a deliberate design choice. Source SEG-Y may be irregular, sparse, or ambiguous in geometry. That complexity belongs in ingest and regularization. Once compute begins, the runtime wants a predictable, rectangular, cacheable volume model.

The operator class tested so far is trace-local:

- `amplitude_scalar { factor }`
- `trace_rms_normalize`

Both operators require access to a full trace along the sample axis. That detail matters because storage layout that splits traces across sample-axis chunks introduces avoidable read amplification and more complicated compute loops.

## The storage candidates that were tested

Four runtime-store shapes were benchmarked through the same compute executor.

### 1. Zarr

Zarr was the original general-purpose array store. In this codebase it was tested in several variants:

- uncompressed, unsharded
- Blosc/LZ4, unsharded
- Zstd, unsharded
- sharded variants of the above

Why Zarr was attractive:

- mature multidimensional array abstraction
- natural subset reads
- explicit chunking
- good fit for scientific data at the metadata/API level

Why Zarr was suspect for this workload:

- nontrivial storage metadata overhead
- many small objects/files in some configurations
- additional codec and chunk-management overhead even when compression is disabled
- a chunk API that is generic enough to be convenient, but not necessarily optimal for a local desktop seismic runtime doing repeated trace-wise operators

### 2. Flat binary control

The benchmark also included a flat-binary control. This was not meant to be production-ready. It existed to answer a narrower question:

> If we remove most container abstraction overhead and store the cube in a very direct binary layout, how much headroom is available?

This control is useful because it separates two concerns:

- logical model quality
- implementation overhead

If Zarr lost badly to a naive contiguous layout, then the system was likely paying too much for generality.

### 3. `tbvol`

`tbvol` is the production-grade custom runtime format that emerged from the benchmark work. It is a dense tiled binary container optimized for local desktop compute.
It is specific to the TraceBoost/Ophiolite runtime stack, not a broadly adopted industry interchange standard like SEG-Y or an increasingly recognized ecosystem format like OpenVDS.

Current on-disk shape:

- `manifest.json`
- `amplitude.bin`
- `occupancy.bin` optional

Key properties:

- fixed-layout tiles
- uncompressed `f32` amplitude payload
- little-endian
- full sample axis in every tile
- mmap-backed reads
- positioned writes into preallocated files
- derived-store lineage stored in metadata

The point of `tbvol` is not novelty. It is to retain the low-overhead behavior that made the flat-binary control attractive, while restoring the locality and partial-read behavior needed for section assembly and streamed full-volume apply.

## What `mmap` means in this design

`mmap` does not mean "load the whole volume into RAM". It means the runtime asks the operating system to map `amplitude.bin` into the process address space and then touches only the byte ranges it needs.

In practical terms:

- the canonical `tbvol` payload stays on the local SSD at rest
- opening a reader creates a virtual-memory mapping for `amplitude.bin` and optional `occupancy.bin`
- when preview or materialization touches a tile range, the OS pages the corresponding file-backed memory into main RAM on demand
- the CPU then consumes the active subset through the normal cache hierarchy: L3, then L2, then L1
- recently touched pages may remain in the OS page cache, but this is still not the same thing as eagerly materializing the whole volume into application-owned buffers

The important runtime consequence is that `tbvol` can preserve a very small explicit working set in application code while still giving the compute path direct access to dense binary data.

### 4. OpenVDS

OpenVDS is increasingly relevant because it is part of the OSDU seismic data stack and is showing up more often as teams standardize around OSDU-aligned interchange and storage workflows.

For this benchmark, OpenVDS was treated as a local desktop data format, not as a cloud/object-store service layer:

- local file-backed `.vds`
- uncompressed amplitude channel
- benchmarked through local subset reads and full-volume page iteration
- brick-size sweep over `32`, `64`, and `128`

Why OpenVDS is worth testing:

- it has a real seismic-specific storage model
- it supports local-file and object-storage-backed VDS assets
- it offers subset reads, page-based access, and format-level metadata
- it is materially more relevant to seismic interoperability than a generic array container

Why OpenVDS was still suspect for this workload:

- the physical unit is still a brick/page layout, not a full-trace tile
- the sample axis is therefore split across multiple bricks for `1024`-sample traces
- that mismatch is expensive for trace-local operators like `trace_rms_normalize`
- it is a stronger interoperability format than it is a guaranteed optimal local compute substrate

## Why "shape" matters more than "format"

The benchmark was never really about brand names like Zarr versus not-Zarr. The primary issue was physical shape.

The logical volume remains `[iline, xline, sample]`, but physical layout controls:

- how many bytes must be touched to preview one section
- whether a trace-local operator can run on contiguous memory
- how much padding or fragmentation is introduced
- how many files and metadata lookups are involved
- whether read amplification occurs at tile boundaries

For the current operator set, the decisive rule is:

- tiles must span the full sample axis

That means the physical tile shape is:

- `[ci, cx, samples]`

This is not arbitrary. `trace_rms_normalize` computes RMS over an entire trace. If the store splits the sample axis across chunks, then one logical trace is no longer one physical unit. The runtime must either stitch partial traces together or perform multi-pass logic with more I/O and more temporary state.

For trace-local operators, that is wasted work.

## Section preview and full apply are different I/O problems

One mistake in storage discussions is treating all access as "random read". In practice the runtime has at least two distinct hot paths.

### Section preview

Preview reads only the tiles intersecting the requested inline or xline section, assembles a numeric `SectionPlane`, and runs the same operator pipeline used for full apply.

The important performance characteristics are:

- low fixed overhead
- limited touched surface area
- minimal allocations
- minimal format decode overhead
- fast assembly from intersecting tiles

This path is latency-sensitive. Small overheads matter.

### Full-volume apply

Full apply streams tile-by-tile, runs the operator pipeline, and writes a new derived store. This path is throughput-sensitive. It is dominated by:

- tile iteration cost
- decode/encode overhead
- allocation churn
- write path efficiency
- whether the store shape fits the operator class

The runtime does not mutate the source store. It always materializes a new derived store with stored lineage. That is a correctness and reproducibility decision, not just an API decision.

## Concrete execution flow

The shared runtime uses the same operator kernels for preview and full apply, but the data movement pattern is intentionally different.

### Preview section flow

When a user previews one inline or xline section from a `tbvol`:

1. The reader opens `manifest.json` and memory-maps `amplitude.bin`.
2. The runtime computes which `[ci, cx, samples]` tiles intersect the requested section.
3. It reads only those tiles and copies the relevant traces into one dense in-memory `SectionPlane`.
4. The OS pages only the touched file regions into RAM as the mapped bytes are accessed.
5. The operator pipeline runs against that one section buffer.
6. The preview result is returned without writing a new persistent store.

This path is optimized for low latency and limited touched surface area.

### Full-volume materialization flow

When a user materializes the pipeline over the entire volume:

1. The source `tbvol` is opened for mapped reads.
2. A new derived `tbvol` is created and its output files are preallocated.
3. The runtime iterates tile-by-tile over the full tile grid.
4. For each tile it:
   - reads one tile from the mapped source
   - pages the touched range into RAM on demand
   - applies the same operator pipeline used for preview
   - writes the processed tile to the deterministic output offset
5. The writer finalizes the new derived store and records lineage.

This path is optimized for throughput and bounded working-set size. It does not require whole-volume in-memory materialization.

## The benchmark methodology

The benchmark was designed to avoid one common failure mode in storage evaluation: measuring a synthetic microbenchmark that does not resemble the real runtime path.

The benchmark therefore runs through the same compute substrate the product uses:

- storage-neutral reader/writer interfaces
- shared tile geometry
- shared section assembly
- real operator kernels
- preview execution path
- full-volume materialization path

OpenVDS was added as an external comparison path through a small standalone C++ runner in [`scripts/openvds_storage_bench.cpp`](/Users/sc/dev/TraceBoost/scripts/openvds_storage_bench.cpp). The runner uses the same synthetic dataset generator and the same two-operator pipeline as the Rust benchmark, but it talks directly to the official OpenVDS API because the shared runtime does not currently ship an OpenVDS backend.

The current benchmark exercises:

- inline section read
- xline section read
- preview `amplitude_scalar`
- preview `trace_rms_normalize`
- preview of both operators in sequence
- full-volume apply of each operator
- full-volume apply of both operators in sequence

It runs against:

- a small real control dataset: `test-data/f3.sgy`
- medium synthetic volume: `256 x 256 x 1024`
- large synthetic volume: `384 x 384 x 1024`

The small real cube is useful for smoke-testing realism. It is not large enough to drive the architecture decision because container overhead dominates at that scale.

## Representative measured results

The numbers below are from the current rerun on April 7, 2026. For `tbvol`, flat binary, and Zarr, the benchmark used the shared Rust runtime with `4 MiB` full-trace tiles. For OpenVDS, the benchmark used local-file `.vds` stores with a brick-size sweep over `32`, `64`, and `128`; the best-balanced OpenVDS result for each dataset is listed, with sweep notes underneath.

### Medium synthetic: `256 x 256 x 1024`

- `zarr_uncompressed_unsharded`
  - preview pipeline: `5.796 ms`
  - full apply pipeline: `560.661 ms`
  - file count: `67`
- `tbvol`
  - preview pipeline: `0.286 ms`
  - full apply pipeline: `648.866 ms`
  - file count: `2`
- `flat_binary_control`
  - preview pipeline: `0.477 ms`
  - full apply pipeline: `1466.344 ms`
  - file count: `1`
- `openvds` best balanced result
  - preview pipeline: `2.960 ms`
  - full apply pipeline: `558.635 ms`
  - file count: `1`

OpenVDS sweep notes for the medium dataset:

- `brick_size=64` was the best balance for preview plus apply
- `brick_size=128` reduced full apply slightly further to `537.918 ms`, but preview regressed to `5.692 ms`
- `brick_size=32` gave `3.184 ms` preview and `749.788 ms` full apply

### Large synthetic: `384 x 384 x 1024`

- `zarr_uncompressed_unsharded`
  - preview pipeline: `9.612 ms`
  - full apply pipeline: `900.962 ms`
  - file count: `147`
- `tbvol`
  - preview pipeline: `0.229 ms`
  - full apply pipeline: `526.317 ms`
  - file count: `2`
- `flat_binary_control`
  - preview pipeline: `0.567 ms`
  - full apply pipeline: `970.066 ms`
  - file count: `1`
- `openvds` best result
  - preview pipeline: `4.759 ms`
  - full apply pipeline: `1033.193 ms`
  - file count: `1`

OpenVDS sweep notes for the large dataset:

- `brick_size=32` was best on both measured workloads
- `brick_size=64` gave `5.087 ms` preview and `1148.289 ms` full apply
- `brick_size=128` gave `10.659 ms` preview and `1314.584 ms` full apply

These numbers matter for three reasons.

First, `tbvol` still dominates preview latency by a wide margin. That remains important because preview is the interactive hot path.

Second, OpenVDS is meaningfully better than generic uncompressed Zarr on file count and often on preview latency, so it is a real comparison target rather than an academic omission.

Third, OpenVDS does not displace `tbvol` as the preferred local compute backend. It can be competitive on the medium synthetic full-apply case, but it falls behind once the volume gets larger and the brick/sample-axis mismatch shows up more clearly.

## Why `tbvol` wins

`tbvol` is not winning because it is magical. It wins because its implementation is aligned with the operator class and deployment model.

### 1. Fixed-layout tiles reduce bookkeeping

Every tile has a deterministic byte offset:

- no dynamic object enumeration
- no per-chunk metadata fetch during the hot path
- no codec negotiation in the read loop

This makes both tile reads and tile writes cheaper.

### 2. Full sample-axis tiles match trace-local operators

Because a tile always spans the full sample axis:

- one physical trace corresponds to one logical trace segment
- `trace_rms_normalize` can run in a straightforward loop
- there is no need to merge sample slabs from multiple chunks

This is the most important physical-layout decision in the whole system so far.

### 3. mmap-backed reads reduce preview overhead

For preview, `tbvol` can read directly from memory-mapped amplitude data and assemble only the requested section. That eliminates a layer of container decoding and reduces copies on the read side.

The effect is most visible in preview latency, where the gap versus Zarr is large.

### 4. Positioned writes fit materialization well

Full apply writes output tile-by-tile into preallocated files using positioned writes. That avoids append-order coupling and lets the writer treat tiles as deterministic addressable units.

That yields a simple streamed pipeline:

- read tile
- apply operators
- write tile

The writer does not need a generalized chunk container protocol to do useful work.

### 5. File count stays low

The Zarr path produced dozens to hundreds of files in the tested configurations. `tbvol` uses a tiny fixed file set. Lower file count is not the main reason `tbvol` is faster, but it is operationally cleaner and removes another source of overhead.

## Why the flat-binary control did not become the final answer

A monolithic contiguous array is attractive because it minimizes abstraction. But it has a structural weakness:

- it is good at being one large blob
- it is not good at being the unit of selective assembly and streamed compute scheduling

Once the dataset becomes large enough, the ability to operate on a sensible tile unit beats the simplicity of a single giant payload. `tbvol` keeps the low-overhead binary payload design while introducing a physical execution unit that matches both preview and apply.

This is a common systems lesson: the best production substrate is often neither the most generic format nor the most naive one. It is the simplest format that still exposes the right physical unit of work.

## Padding, tile size, and the current conclusion

Tile size is not a cosmetic tuning parameter. It changes:

- tile count
- padding waste on edge tiles
- I/O granularity
- per-tile loop overhead
- section assembly behavior

The benchmark found:

- `1 MiB` tiles were too small and increased tile-management overhead
- on the original synthetic benchmark corpus, `8 MiB` tiles were not consistently better and increased padding waste for non-divisible shapes
- the original practical sweet spot was around `2-4 MiB`

That conclusion is now qualified by the first focused real-volume sweep. On April 8, 2026, the new `sweep-tbvol` benchmark was run against `C:\Users\crooijmanss\Downloads\archive\f3_dataset.sgy`, a `651 x 951 x 462` regularized volume.

That run found:

- `1 MiB` won preview latency at `1.208 ms` but lost badly on full apply at `6907.067 ms`
- `2 MiB` reached `1.458 ms` preview and `1633.727 ms` full apply
- `4 MiB` reached `2.287 ms` preview and `1334.141 ms` full apply
- `8 MiB` reached `1.279 ms` preview, `1290.195 ms` full apply, and the best section-read I/O

So the more accurate current conclusion is:

- `1 MiB` is still too small for production-scale materialization
- `2-4 MiB` remains a strong synthetic/default regime
- `8 MiB` is now a proven contender, and was the best balanced choice on the first customer-scale sweep
- the shared runtime now uses a conservative adaptive fallback of `4 MiB` below roughly `768 MiB` dense `f32` volume size and `8 MiB` at or above that threshold when no explicit tile shape is supplied

That is why the current recommendation is not a generic "bigger chunks are better." It is:

- use padding-aware full-trace tiles
- treat `2`, `4`, and `8 MiB` as active benchmark candidates
- let dataset shape influence the exact `ci x cx` selection

This is implemented in the `tbvol` tile recommendation logic. The runtime scores candidate tile shapes by balancing:

- target-byte proximity
- padding ratio
- aspect ratio penalty
- tile count

That is a better policy than hardcoding one chunk shape or blindly maximizing chunk size.

## Tile size is about locality, not exact cache fitting

The current tile-size conclusion should not be read as "fit every tile into L2" or "fit every tile into L3". CPU cache sizes vary by workstation and operating system does not expose one universal tuning target anyway.

What the current policy is actually optimizing for is broader and more robust:

- contiguous useful reads from disk into RAM
- predictable section assembly from intersecting tiles
- low enough tile count to avoid excessive per-tile overhead
- small enough tile payloads to limit padding waste and keep the active working set tractable
- a physical unit of work that the CPU can consume with good locality once the tile is active

In other words, the runtime is tuned for storage locality, RAM locality, and compute-shape locality together. It is not trying to hand-pack the format to one specific workstation cache size.

## Storage format versus metadata and lineage

One concern in moving away from a general-purpose array format is whether provenance and derivation tracking become weaker. In this design they do not.

Derived volumes persist processing lineage in store metadata, including the operator pipeline used to produce them. That means the system can answer:

- what parent store produced this volume
- which operators were applied
- with which parameters
- in which order

This is the correct place to model provenance for this application: in explicit domain/runtime metadata, not in a storage-engine abstraction that was not built to be the application's provenance system.

This is also why Apache Arrow and Spark were not adopted as the provenance answer here.

- Arrow is valuable as a columnar memory and interchange format, and it supports metadata, but it is not the right primary storage substrate for dense local seismic cube processing in this workflow.
- Spark lineage is lineage inside a compute engine, not persisted seismic asset provenance for a local desktop runtime.

The current design keeps provenance attached to the derived seismic object itself, which is the pragmatic and correct long-term choice.

## Architectural consequences

The benchmark did not merely recommend a faster file format. It forced a broader runtime architecture.

The shared seismic runtime now centers on:

- storage-neutral reader/writer interfaces
- shared tile geometry
- shared section assembly
- one compute executor for preview and full apply
- one canonical tiled binary runtime path for production use

That is the deeper engineering outcome. Once those abstractions exist, storage backends become measurable implementations instead of assumptions baked into business logic.

This also made it possible to move the backend into the shared Ophiolite seismic core, with TraceBoost consuming it rather than owning a parallel runtime stack.

## What remains true even if future operators change

The current conclusions are strong, but they are not universal laws.

They are valid for the current operator class and deployment model:

- trace-local CPU operators
- local filesystem
- local workstation
- preview on sections
- full-volume materialization to derived stores

If the system later emphasizes:

- wide spatial stencils
- heavy frequency-domain transforms
- distributed execution
- remote object storage
- GPU-first execution

then the optimal physical layout may change. That does not invalidate the current decision. It means storage decisions should continue to be benchmark-driven and operator-aware.

The important thing is that the runtime is no longer boxed into a storage abstraction it cannot question.

## What is still worth benchmarking further

The current storage decision is settled enough to stop debating lossy compression and generic chunk containers for the hot path, but some benchmark work is still worth doing.

Questions and recommended answers:

- Should active runtime work focus on `tbvol` rather than SGZ/ZFP-style compression: yes.
- Should we benchmark more `tbvol` tile shapes for preview versus full apply tradeoffs on larger real datasets: yes.
- Should we benchmark whether imported source stores and derived output stores should use the same tile policy: yes, but only if the benchmark is run through the same preview and materialization code paths.
- Should we benchmark section-read prefetch or read-ahead behavior for the preview path: yes.
- Should we benchmark overlapped read, compute, and write scheduling for full-volume materialization: yes, if apply throughput becomes a practical user bottleneck.
- Should we tune the format around exact L1, L2, or L3 cache sizes on individual machines: no.
- Should we pursue lossy compression for the active runtime store when amplitude fidelity must be exact: no.
- Should we redesign the hot path around whole-volume in-memory materialization: no.

The next worthwhile performance questions are therefore narrow and empirical:

- how sensitive is preview latency to tile shape on real customer-sized cubes
- how sensitive is materialization throughput to tile shape and output write scheduling
- whether a small amount of reader-side prefetch improves preview without complicating correctness
- whether the current default should settle at `2 MiB`, `4 MiB`, or `8 MiB` once more real datasets are in the benchmark corpus

## Final conclusion

For the current TraceBoost/Ophiolite seismic runtime, `tbvol` is still the right default backend.

Not because Zarr or OpenVDS are bad in general, and not because custom formats are inherently superior, but because:

- the operator class is trace-local
- preview and full apply need the same compute substrate
- the workload is local and latency-sensitive
- full-trace tiled binary layout minimizes overhead while preserving selective access
- mmap reads and positioned writes map cleanly onto the required execution paths
- the measured results still show a decisive preview advantage
- the larger synthetic full-apply case still favors `tbvol`

The benchmark therefore changed the architecture in a meaningful way:

- Zarr is no longer the optimization target for runtime compute
- OpenVDS is now a benchmarked interoperability/comparison format rather than a missing data point
- monolithic flat binary was useful as a control, but not as the final production answer
- `tbvol` remains the preferred runtime backend because it best matches the operator class and the local interactive workload

That is the main lesson of the exercise: in scientific compute systems, backend correctness and backend speed often come from the same design choice, namely choosing a physical layout that matches the unit of computation.
