# F3 Materialized Horizon Conversion Report

- store: `/Users/sc/dev/TraceBoost/sandbox/f3_dataset_regularized.tbvol`
- transform: `f3-paired-horizon-survey-transform`
- compared pairs: `4`

## Summary
- twt_to_depth_m: mean RMSE 0.097407 m, mean MAE 0.082620 m, worst P95 0.249512 m, worst max 0.331665 m, invalid cells 0
- depth_to_twt_ms: mean RMSE 0.084238 ms, mean MAE 0.071466 ms, worst P95 0.238159 ms, worst max 0.288147 ms, invalid cells 0

## Notes
- These figures compare the stored derived horizons against the authored canonical horizons already imported into the same regularized F3 store.
- For this F3 package, the materialized-horizon numbers match the paired-horizon transform benchmark, which is expected because both paths use the same piecewise-linear time-depth model.
- The exported depth velocity cube remains a derived convenience product on a regular depth axis starting at 0 m; it is not the accuracy anchor for horizon-to-horizon conversion.
