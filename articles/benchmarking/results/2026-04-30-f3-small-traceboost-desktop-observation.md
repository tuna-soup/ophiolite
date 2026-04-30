# F3 Small TraceBoost Desktop Observation (2026-04-30)

Status: local desktop observation, not an authoritative benchmark

Source:

- `/Users/sc/Library/Logs/TraceBoost/traceboost-session-1777558935191-47502.log`
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

## Observed Frontend Timings

From the five `Frontend section load timings recorded` entries:

| Metric | Min | Median | Max |
| --- | ---: | ---: | ---: |
| backend await / frontend await | `44 ms` | `48 ms` | `52 ms` |
| commit to second frame | `27 ms` | `35 ms` | `37 ms` |
| total frontend load to second frame | `75 ms` | `81 ms` | `89 ms` |

## Interpretation

This is a useful desktop smoke observation for full-section browsing on F3 small. It is not the benchmark needed for the typed-array view optimization because no viewport tile request was captured.

To capture the zero-copy viewport path, the overlay should show a loaded tile window instead of `Full section`, nonzero viewport fetches, and populated `Adapt` / `Views` fields. The expected successful signal is:

- `copied` near `0 MiB`
- `views` close to the tile payload size
- nonzero viewed buffer count
- an `adapt` time for the last fetched viewport tile
