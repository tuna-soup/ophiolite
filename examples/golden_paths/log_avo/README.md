# Log-to-AVO Golden Path

This example is the Python-first workflow sandbox for the canonical LAS/log to AVO path.

It is intentionally separate from `Ophiolite Charts` and the Svelte playground:

- `ophiolite` owns ingest, canonical well/log meaning, interval modeling, and AVO compute
- `Ophiolite Charts` owns rendering of chart-ready DTOs

The script and notebook here demonstrate the same workflow:

1. create or open a project
2. ingest LAS into a well/wellbore context
3. resolve canonical elastic channels through `ElasticChannelBindings` using stable log-type selectors such as `Dt`, `Dts`, and `Rho`
4. optionally materialize derived velocity logs for downstream reuse
5. discover interval sets through `wellbore.top_set(...)` and `top_set.interval_selectors`
6. define interval layering with `LayeringSpec` or `top_set.layering(...)`
7. define the AVO experiment with `AngleSampling` and `AvoExperiment`
8. run Zoeppritz AVO through `elastic.run_avo(...)`
9. write chart-ready response and crossplot source JSON

The example uses:

- `zoeppritz` for the response curves
- `shuey_two_term` for the intercept-gradient crossplot handoff

## Data Modes

### 1. Real LAS pair

The script will use the following environment variables when present:

- `OPHIOLITE_LOG_AVO_DSI_LAS`
- `OPHIOLITE_LOG_AVO_DENSITY_LAS`
- `OPHIOLITE_LOG_AVO_TOPS_SOURCE`

If they are unset, the script falls back to the current local F3 F02-A-05 candidates when those files exist:

- `/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/f02a05_20030103-1-06_a5_main_log_dsi_030lup.las`
- `/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/f02a05_20030103-1-06_lwd2259a5rt.las`
- `/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/lithostratigrafie.txt`

When the tops source is available, the workflow imports it through `Project.import_tops_source(...)`
into a canonical top-set asset bound to the same wellbore as the LAS assets. The current F02-A-05
file reports `Kelly Bushing`; the workflow preserves that raw provenance and still treats it as a
measured-depth interval datum for log-layer AVO.

For the elastic channels, the Python surface exposes stable log types in addition to lower-level
curve semantics. Users can ask for `Dt`, `Dts`, and `Rho` without needing to know whether the LAS
actually carried `DTCO`, `DTSM`, or `BDCX`.

The `Wellbore` surface now also exposes:

- `wellbore.available_log_types()`
- `wellbore.log_curves_by_type("Dt")`
- `wellbore.preferred_log_curve("Rho")`
- `wellbore.available_top_sets()`
- `wellbore.top_set("lithostrat-tops")`
- `wellbore.marker_sets()`

The imported F02-A-05 tops include repeated labels, so `WellTopSet` also exposes stable interval
selectors for precise layering:

- `top_set.interval_selectors`
- `top_set.select_intervals(selectors=["NLLFC#1", "CKEK#1"])`
- `top_set.layering(selectors=["NLLFC#1", "CKEK#1"])`

### 2. Synthetic fallback

If no real LAS pair is available, the script generates a synthetic project fixture using the platform binary and runs the same workflow shape against that project. The synthetic fixture includes:

- `DT`
- `DTS`
- `RHOB`
- a top set named `synthetic-tops`

## Run

From the repo root:

```bash
python3 examples/golden_paths/log_avo/log_avo_workflow.py
```

To force real-data mode:

```bash
OPHIOLITE_LOG_AVO_DSI_LAS=/path/to/dsi.las \
OPHIOLITE_LOG_AVO_DENSITY_LAS=/path/to/density.las \
OPHIOLITE_LOG_AVO_TOPS_SOURCE=/path/to/lithostratigrafie.txt \
python3 examples/golden_paths/log_avo/log_avo_workflow.py --data-mode real
```

To force synthetic mode:

```bash
python3 examples/golden_paths/log_avo/log_avo_workflow.py --data-mode synthetic
```

To persist derived `VP` and `VS` logs as project assets:

```bash
python3 examples/golden_paths/log_avo/log_avo_workflow.py --materialize-derived-channels
```

To set custom interval edges:

```bash
python3 examples/golden_paths/log_avo/log_avo_workflow.py \
  --edge-depths-m 1881.25,1895.0,1910.0,1930.0
```

To drive the top-set path with exact interval selectors:

```bash
python3 examples/golden_paths/log_avo/log_avo_workflow.py \
  --top-set-asset-name lithostrat-tops \
  --top-set-selectors NLLFC#1,CKEK#1
```

## Outputs

Each run writes:

- `workflow_summary.json`
- `avo_fixed_interval_source.json`
- `avo_fixed_interval_crossplot.json`
- `avo_explicit_edges_source.json`
- `avo_top_set_source.json` when a top set is available

The JSON outputs are chart-ready DTOs for the AVO chart family. They are meant to be consumed by chart wrappers, not generated inside them.

## Notebook

`log_avo_workflow.ipynb` reuses the script helpers and is the recommended customer-facing walkthrough surface.
