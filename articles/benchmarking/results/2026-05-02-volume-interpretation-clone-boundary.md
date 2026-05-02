# Volume Interpretation Clone Boundary Smoke Benchmark (2026-05-02)

Date: 2026-05-02

Command:

```bash
bun --eval '<inline clone-boundary benchmark>'
```

Purpose:

- Check the new `cloneVolumeInterpretationModel` path against the previous `structuredClone(model)` pattern in the 3D volume controller.
- Measure clone overhead for a metadata scene with about `6.86 MiB` of typed horizon/well buffers.
- Confirm that function-backed `VolumeInterpretationDataSource` handles cannot safely pass through `structuredClone`.

Result:

| Case | Median |
| --- | ---: |
| `cloneVolumeInterpretationModel` | `0.0047 ms` |
| `structuredClone(model)` | `1.0956 ms` |

Observed behavior:

- The custom clone preserved typed-array buffer identity for horizon and well geometry.
- `structuredClone(model)` duplicated the typed arrays.
- `structuredClone(modelWithDataSource)` threw `DataCloneError` because the data source contains a `loadSlice` function.

Interpretation:

This is a small smoke benchmark, not a rendering benchmark. It supports the architectural change: the 3D volume scene should remain metadata plus data-source handles, and the controller should not structured-clone models that may contain large typed-array views or loader functions.

Next benchmark:

- Real slice feed into `VolumeInterpretationChart`.
- Capture source payload bytes, copied/viewed/transferred bytes, adapt time, and VTK render/update time.
