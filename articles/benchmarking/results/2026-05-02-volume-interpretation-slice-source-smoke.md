# Volume Interpretation Slice Source Smoke Benchmark (2026-05-02)

Date: 2026-05-02

Command:

```bash
bun --eval '<inline mock VolumeInterpretationDataSource.loadSlice benchmark>'
```

Purpose:

- Confirm that the Svelte playground mock volume now feeds slice payloads through `VolumeInterpretationDataSource.loadSlice`.
- Confirm that the synthetic demo source follows the production-shaped path: resolved Ophiolite source -> `adaptOphioliteVolumeInterpretationToChart` -> `VolumeInterpretationChart`.
- Establish the current demo payload sizes before wiring real TraceBoost/Ophiolite runtime slices into the same contract.

Result:

| Axis | Dimensions | Payload | Median load |
| --- | ---: | ---: | ---: |
| inline | `128 x 256` | `131,072 bytes` | `1.32 ms` |
| xline | `160 x 256` | `163,840 bytes` | `1.61 ms` |
| sample | `160 x 128` | `81,920 bytes` | `0.83 ms` |

All payloads used `ownership: "view"` and `sampleFormat: "f32"`.

Interpretation:

This is a demo/data-contract smoke benchmark, not a production seismic benchmark. It proves the chart can now receive slice-sized payloads through a data-source handle instead of requiring the renderer to synthesize or own a full dense volume. The next production benchmark should replace the mock source with a TraceBoost/Ophiolite adapter and record real source bytes, copied/viewed/transferred bytes, cache hits, and vtk.js update time.
