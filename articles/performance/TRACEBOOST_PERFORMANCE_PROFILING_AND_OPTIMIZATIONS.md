# TraceBoost Performance Profiling And Optimizations

## Purpose

This note records the performance investigation and optimization work that was done around:

- section loading
- trace-local preview
- full-volume processing runs
- frontend/backend payload transfer
- session-log profiling

It is intended to explain:

- what was slow
- how it was profiled
- what changed in Rust, Tauri, Svelte, and Ophiolite Charts
- what materially improved
- what still remains worth investigating

## Initial symptom

The app felt slow during preview and dataset interaction, even when the backend-reported compute times were already low.

At the beginning of the investigation, the key contradiction was:

- Rust/backend timings were often in the tens of milliseconds
- the UI still felt like it was blocking for seconds

That meant the bottleneck was likely outside raw seismic compute.

## Profiling approach

Two profiling layers were used.

### 1. Always-on structured app diagnostics

TraceBoost already had session logs under:

- `C:\Users\crooijmanss\AppData\Local\com.traceboost.app\logs`

We expanded those logs to include:

- backend compute duration
- preview cache hit state
- reused prefix count
- frontend await/commit/frame timings
- section payload size
- full-run stage/job timings

This became the main repeatable measurement tool because it is:

- always on
- lightweight
- stored automatically
- comparable across runs

### 2. DevTools timeline profiling

We also used WebView/Edge DevTools `Performance` recordings to identify the main-thread hotspot.

That trace showed the dominant frontend work was in:

- `syncController`
- `decodeSectionView`
- `decodeFloat32`

In other words, the main cost was not seismic math. It was:

- payload transport
- JS-side decode
- extra copies
- main-thread chart update

## Baseline findings

### Preview before the frontend fixes

From the early session logs:

- backend compute: about `37-95 ms`
- frontend await: about `333-410 ms`
- frontend post-assign/render path: about `2175-2543 ms`
- total frontend time: about `2587-2878 ms`

This established the original shape of the problem:

- compute was not dominant
- the real stall was frontend-side after the response arrived

### DevTools finding

The DevTools trace showed the expensive path was in frontend decode and chart synchronization, not layout/paint.

The main issue was that dense section payloads were still effectively moving through the app as JSON-style arrays and then being copied again during decode.

## Major optimizations implemented

## 1. Same-session preview prefix reuse

We first improved backend preview computation itself for same-section repeated edits.

Changes:

- added same-session in-memory preview prefix reuse in the seismic runtime
- reduced cache-key overhead
- reran only the suffix of the unchanged pipeline prefix on cache hit

Measured runtime benchmark results on the large F3 `tbvol` showed:

- inline late scalar edit: `4.488 ms -> 1.447 ms`
- inline late filter edit: `3.872 ms -> 2.665 ms`
- inline late AGC edit: `3.628 ms -> 1.730 ms`
- xline late scalar edit: `2.170 ms -> 1.128 ms`
- xline late filter edit: `2.447 ms -> 1.772 ms`
- xline late AGC edit: `1.868 ms -> 1.141 ms`

That made same-session preview prefix reuse worth integrating for desktop preview.

## 2. Move preview execution off the blocking command path

Preview compute was moved onto a blocking worker from the Tauri command path so the GUI thread was not tied directly to seismic compute execution.

This improved responsiveness and made the log timings more representative of the real split between compute and UI work.

## 3. Add frontend timing instrumentation

We added structured frontend timing events for preview and later for normal section loads.

The key fields now emitted are:

- `frontendAwaitMs`
- `frontendStateAssignMs`
- `frontendPersistWorkspaceMs`
- `frontendTickMs`
- `frontendFirstFrameMs`
- `frontendSecondFrameMs`
- `frontendCommitToSecondFrameMs`
- `frontendTotalMs`
- `payloadBytes`

This made it possible to separate:

- backend compute
- IPC/transport/deserialization
- Svelte state assignment
- UI commit/frame cost

## 4. Remove deep Svelte proxying for large section payloads

Large section payloads were being held in deep reactive state even though they are replaced wholesale rather than mutated incrementally.

We switched the large holders to `$state.raw(...)` in the frontend models for:

- active section
- background/compare section
- preview section

This removed a large amount of unnecessary reactive proxy overhead.

## 5. Cache decoded section payloads in Ophiolite Charts

`Ophiolite Charts` was repeatedly decoding the same incoming section object.

We added object-identity decode caches using `WeakMap`, so the decoded typed-array payload can be reused as long as the same section object instance is still in play.

This specifically reduced repeated `decodeSectionView(...)` work in chart synchronization.

## 6. Remove an extra copy during float decode

The original JS-side decode path did this pattern:

- construct `Uint8Array.from(bytes)`
- then `.buffer.slice(...)`
- then build `Float32Array`

That introduced an unnecessary extra copy.

The decode path now uses direct typed-array views where possible, especially for `Uint8Array`-backed transport payloads.

This materially reduced the main-thread decode cost seen in DevTools.

## 7. Switch preview transport from JSON-style arrays to packed binary

This was the next big step after the frontend decode/render stall was fixed.

Instead of returning preview sections through the old JSON-heavy command path, desktop preview now returns:

- a small JSON header
- packed binary buffers for axes and amplitudes

The frontend unpacks those buffers and passes them through as `Uint8Array`-backed payloads.

This avoids the earlier `number[]` expansion cost for dense preview sections.

## 8. Apply the same binary transport to normal section loads

Once preview was on the binary path, normal section loading was still paying the older transport cost.

That was fixed by adding a packed binary `load_section_binary_command` and moving ordinary viewer section loads to the same packed transport pattern.

This means normal browsing and preview now share the same transport strategy.

## 9. Add stage/job timing for full-volume runs

Full-volume run logging was extended so the session log now includes:

- `processing_job_started`
- `processing_job_stage_started`
- `processing_job_stage_completed`
- `jobDurationMs`
- per-stage `materializeDurationMs`
- `lineageRewriteDurationMs`
- `artifactRegisterDurationMs`

Equivalent total-duration logging was also added for subvolume and gather processing jobs.

This makes full runs diagnosable from session logs instead of relying only on queue/completed timestamps.

## Measured improvements

## Preview path

### Before frontend decode/render fixes

- backend compute: `37-95 ms`
- frontend await: `333-410 ms`
- frontend UI/post-assign: `2175-2543 ms`
- frontend total: `2587-2878 ms`

### After `$state.raw`, decode cache, and less-copy decode

- backend compute: `16-32 ms`
- frontend await: `349-402 ms`
- frontend post-assign/render: roughly `4-31 ms`
- frontend total: `355-433 ms`

This removed the huge multi-second frontend stall. The dominant cost then became transport.

### After packed binary preview transport

Representative measured preview result:

- backend compute: `57 ms`
- frontend await: `141.1 ms`
- frontend commit-to-second-frame: `14.9 ms`
- frontend total: `156 ms`

That is the key end-to-end win:

- the multi-second stall was removed first
- then the remaining transport cost dropped by about `2.5x`

## Normal section load path

After moving normal section loads onto the binary path, the latest measured section-load example was:

- backend load: `22 ms`
- payload size: `1,766,904 bytes`
- frontend await: `119.2 ms`
- frontend persist-workspace: `15.3 ms`
- frontend commit-to-second-frame: `56.7 ms`
- frontend total: `175.9 ms`

This is a much better baseline for normal browsing than the previous JSON-heavy path.

## Full-volume rerun behavior

Separate earlier benchmarking already showed:

- exact full-pipeline rerun reuse is clearly worth keeping
- automatic hidden whole-volume prefix checkpoints are not worth keeping

On the real larger F3 volume:

- exact rerun dropped from about `14.1 s` to about `38 ms`
- automatic hidden intermediate caching did not provide a strong enough late-edit rerun win and was removed as a default strategy

That led to the current architecture choice:

- keep exact-result reuse
- keep explicit checkpoint outputs
- do not rely on automatic hidden whole-volume intermediate caching
- use same-session preview prefix reuse for interactive preview
- keep the production reuse layer narrow rather than exposing multiple hidden-cache policy modes

## What the latest session log shows

The supplied session log:

- `traceboost-session-1775721894760-50764.log`

does show:

- one full-volume trace-local processing completion
- one binary section load after opening the final output
- one frontend section-load timing event

It does **not** show completed preview timing events in that specific file.

So from that session log specifically, the strongest confirmed measurements are:

- full run completion with `jobDurationMs`
- normal section load on the new binary path

The absence of preview entries in that file means preview was either not completed in that session, or the file is not the one containing those preview events.

## Checkpoint output investigation

At the end of the session, a pipeline had explicit checkpoint markers on step `5` and step `6`, so the expected outputs were:

- checkpoint after step `5`
- checkpoint after step `6`
- final output after step `7`

Investigation result:

- all three stores were created on disk
- all three stores were registered in the workspace registry

Concrete filesystem evidence:

- `...step-05-bandpass_filter.tbvol`
- `...step-06-amplitude_scalar.tbvol`
- final `...pipeline2-dotted....tbvol`

Concrete registry evidence:

- `workspace/dataset-registry.json` contains entries for both step `5` and step `6` checkpoints and the final output

So the issue is **not** missing materialization.

The likely issue is discoverability/visibility in the UI:

- the outputs exist
- the registry knows about them
- but they are easy to miss because they are not presented as a grouped “run outputs” set

This suggests a future UX improvement rather than a backend correctness fix.

## Remaining opportunities

The current bottlenecks are much smaller than before, but there are still useful next steps.

### 1. Reduce `frontendPersistWorkspaceMs`

Normal section load logs now show a noticeable workspace-persist component.

That is not catastrophic, but it is now visible because the larger transport/render costs were removed.

### 2. Consider event-driven job updates

The logs still show repeated `get_processing_job_command` polling during long runs.

This is not the dominant performance problem, but it is noisy and could eventually be replaced with push-style updates.

### 3. Improve UI presentation of checkpoint outputs

The system currently creates and registers checkpoint outputs correctly, but the user can still reasonably think one is missing.

That means the next likely improvement there is:

- better grouping
- clearer run artifact presentation
- explicit checkpoint output surfacing in the UI

### 4. Keep using session logs as the primary comparison tool

The structured session log is now good enough to compare:

- preview
- ordinary section load
- full-volume runs

without requiring a manual DevTools trace for each run.

Manual DevTools recordings are still useful for occasional deep frontend investigations, but they are no longer needed for everyday performance comparison.

## Final takeaway

The performance work has been successful because it followed measurement rather than intuition.

The key lessons were:

- the initial pain was mostly not in Rust compute
- the biggest frontend stall was in JS-side decode/copy/reactivity
- fixing state ownership and decode behavior removed the multi-second UI stall
- moving preview and section transport to packed binary removed most of the remaining wait
- same-session preview prefix reuse is worth keeping
- exact full-result reuse is worth keeping
- automatic hidden whole-volume prefix caching is not worth keeping in the current design

The app is now much closer to the right performance shape:

- low backend compute cost
- bounded transport cost
- low frontend commit cost
- logs that make future regressions measurable
