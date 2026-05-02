# F3 Small TraceBoost Desktop Observation (2026-04-30)

Status: local desktop observation, not an authoritative benchmark

Source:

- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777558935191-47502.log`
- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777561186143-59926.log`
- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777561436275-60636.log`
- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777561870993-63353.log`
- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777562015472-63855.log`
- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777574448571-85816.log`
- User screenshots of the TraceBoost Section Tiling overlay for F3 small

## Context

Dataset:

- `f3_dataset-5017566849517f13.tbvol`
- shape: `651 x 951 x 462`
- loaded inline indices: `46, 47, 48, 49, 50`
- section shape per loaded inline: `951 traces x 462 samples`
- section payload bytes: `1,766,904`

The screenshots show the Section Tiling overlay, but the captured state is still the full-section path:

- loaded: `Full section`
- fetch: `0 viewport · 0 prefetch`
- adapt: `pending`
- views: `pending`

That means this run confirms the overlay is visible, but it does not yet prove the new viewport-tile zero-copy adaptation counters.

Follow-up forced-tiling sessions did exercise the viewport tile path:

- F3 small (`/Users/sc/segyio/test-data/f3.sgy`, runtime store `f3-1be6aa4b777bf7ed.tbvol`) loaded forced viewport tiles without crashing. The captured tile payloads were `5,844` bytes with `copiedBytes: 0`, `viewedBytes: 5,844`, and `viewedBuffers: 3`.
- Large F3 (`f3_dataset`, derived from `/Users/sc/Downloads/SubsurfaceData/blocks/F3/f3_dataset.sgy`) loaded forced tiles after the tile-intersection underflow fix. A full-section forced tile carried `1,766,904` bytes with `copiedBytes: 0` and `viewedBytes: 1,766,904`.
- Zoom/pan forced-tiling sessions showed cache reuse in the overlay around `93-98%`, with no reported tile errors. The backend payloads still showed large sample windows before the `SECTION_TILE_BUCKET_SAMPLES` reduction from `512` to `128`, so follow-up runs should confirm smaller vertical-window payloads when zoomed in by time/depth.
- The later `1777574448571` session confirmed the `128`-sample buckets in production logs. Zoomed viewport requests such as `T [373,396] S [183,209]` loaded backend windows like `T [256,512] S [128,256]` with `133,632` bytes, and wider padded windows like `T [256,512] S [128,384]` with `265,216` bytes. That is roughly `8-15%` of the `1,766,904` byte full-section payload. Adaptation stayed zero-copy (`copiedBytes: 0`, `viewedBuffers: 3`), and the screenshot showed `97%` tile-cache reuse.

## Observed Frontend Timings

From the five `Frontend section load timings recorded` entries:

| Metric | Min | Median | Max |
| --- | ---: | ---: | ---: |
| backend await / frontend await | `44 ms` | `48 ms` | `52 ms` |
| commit to second frame | `27 ms` | `35 ms` | `37 ms` |
| total frontend load to second frame | `75 ms` | `81 ms` | `89 ms` |

## Interpretation

This is a useful desktop smoke observation for full-section browsing on F3 small. It is not the benchmark needed for the typed-array view optimization because no viewport tile request was captured.

To capture the zero-copy viewport path on either F3 fixture, enable `Force viewport tiles` in the Section Tiling overlay. The overlay should then show a loaded tile window instead of `Full section`, nonzero viewport fetches, and populated `Adapt` / `Views` fields. The expected successful signal is:

- `copied` near `0 MiB`
- `views` close to the tile payload size
- nonzero viewed buffer count
- an `adapt` time for the last fetched viewport tile
- zoomed/panned views should keep cache hits high and should now request narrower sample windows because the backend tile request snaps samples to `128`-sample buckets instead of always covering F3's whole `462`-sample vertical range

Current limitation:

- Force viewport tiles still runs after the app has loaded the full section during line browsing. In the `1777574448571` session, stepping inline `95 -> 100` loaded full `1,766,904` byte sections with frontend totals around `73-103 ms`, then fetched the smaller viewport tile for the active line. This makes forced tiles useful evidence and useful while panning/zooming, but not yet the production default for rapid line flicking.

Follow-up implementation:

- TraceBoost now has a viewport-first line-browsing path for zoomed views. When the requested tile window is less than `45%` of the full logical section area after halo and bucket snapping, Next/Prev and axis-switch browsing can display the viewport tile directly and skip the initial full-section fetch.
- This keeps `Force viewport tiles` as a diagnostic switch instead of making it the production default. The production path should now be judged by new logs that contain `viewer_load_section_viewport_first`, `fullSectionSkipped: true`, and payload sizes close to the active tile window rather than the full `1,766,904` byte section.
- The next benchmark should repeat the same zoomed inline flick workflow and compare section-change totals against the `73-103 ms` full-section line-browse baseline above.
