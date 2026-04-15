F3 synthetic assets for the regularized F3 survey.

- `survey_package/`
  - Generated ASCII inputs:
    - `survey_spec.json`
    - `Velocity_functions.txt`
    - `horizon_*_twt_ms.xyz`
    - `horizon_*_depth_m.xyz`
    - `horizon_*_depth_ft.xyz`
- `f3_dataset_regularized.tbvol/`
  - Regularized survey store used for import and time-depth modeling.
- `benchmark_report.json`
  - Import fidelity and TWT<->depth conversion accuracy report.
- `benchmark_report.md`
  - Short human-readable summary of the benchmark.
- `derived_horizon_conversion_report.json`
  - Materialized-horizon comparison report for stored TWT->depth and depth->TWT outputs.
- `derived_horizon_conversion_report.md`
  - Short human-readable summary of the stored derived horizon accuracy.
- `validate_bundle.py`
  - Fast local validator for bundle completeness, expected store assets, transform ids, and report thresholds.
- `depth_velocity_cube/`
  - Depth-domain synthetic interval-velocity cube derived from `f3-paired-horizon-survey-transform`.
  - Regular depth axis: `start=0 m`, `step=50 m`, `count=51`, `extent=2451.98 m`.

Current canonical transform status:

- Sparse imported `Velocity_functions.txt` control profiles were ingested successfully, but the single-interval nearest-neighbor Vint build path was not accurate enough for horizon-domain conversion on F3.
- The survey now also contains `f3-paired-horizon-survey-transform`, built directly from the imported canonical TWT and depth horizon pairs.
- Benchmark result for `f3-paired-horizon-survey-transform`:
  - TWT -> depth full-grid mean RMSE: `0.097407 m`
  - Depth -> TWT full-grid mean RMSE: `0.084240 ms`
- The regularized store now also contains materialized converted horizons:
  - `horizon_01_twt_ms-derived_depth_m` ... `horizon_04_twt_ms-derived_depth_m`
  - `horizon_01_depth_m-derived_twt_ms` ... `horizon_04_depth_m-derived_twt_ms`
  - These payloads live under the store's `horizons/` directory and are indexed in `horizons/manifest.json`.
- Stored derived-horizon accuracy against the authored canonical pairs:
  - TWT -> depth full-grid mean RMSE: `0.097407 m`
  - Depth -> TWT full-grid mean RMSE: `0.084238 ms`
- The depth velocity cube is a regularized derivative for depth-domain workflows:
  - `start=0 m`
  - `step=50 m`
  - `count=51`
  - `max_depth=2401.98 m`
  - `extent=2451.98 m`
  - `invalid_trace_count=0`

Accuracy notes:

- For this F3 package, the paired-horizon transform is the accuracy anchor for horizon-domain conversion.
- The exported depth-domain Vint cube follows the same transform and starts at depth zero, extending beyond the deepest modeled horizon interval, which makes it suitable for synthetic survey-style cube consumers.
- Using the sparse ASCII velocity functions directly as the canonical conversion model was retained as an ingestion path, but not as the recommended conversion path for this survey.

The links point at the active generated assets under `/Users/sc/dev/TraceBoost/sandbox`.
