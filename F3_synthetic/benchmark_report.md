# F3 Synthetic Horizon Conversion Benchmark

- store: `/Users/sc/dev/TraceBoost/sandbox/f3_dataset_regularized.tbvol`
- survey package: `/Users/sc/dev/TraceBoost/sandbox/f3_synthetic_f3_crs`
- transform: `f3-paired-horizon-survey-transform`
- control profiles: `110`

## Summary
- twt_to_depth_m / full_grid: mean RMSE 0.097407 m, mean MAE 0.082624 m, worst P95 0.249499 m, worst max 0.331677 m
- twt_to_depth_m / control_profiles: mean RMSE 0.099790 m, mean MAE 0.085152 m, worst P95 0.254233 m, worst max 0.282573 m
- depth_to_twt_ms / full_grid: mean RMSE 0.084240 ms, mean MAE 0.071469 ms, worst P95 0.238164 ms, worst max 0.288172 ms
- depth_to_twt_ms / control_profiles: mean RMSE 0.085782 ms, mean MAE 0.073151 ms, worst P95 0.252302 ms, worst max 0.266757 ms

## Notes
- This benchmark reflects the paired-horizon transform path, so the remaining error is dominated by ASCII quantization and piecewise-linear resampling against the stored survey sample axis.
- Control-profile statistics are reported for the authored velocity-function locations, but they are only a reporting subset for this paired-horizon transform.
- Import fidelity checks compare stored canonical horizon grids against the original ASCII source rows.
