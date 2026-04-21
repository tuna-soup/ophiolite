# Log-to-AVO Golden Path

This example is the Python-first workflow sandbox for the canonical LAS/log to AVO path.

It is intentionally separate from `Ophiolite Charts` and the Svelte playground:

- `ophiolite` owns ingest, canonical well/log meaning, interval modeling, and AVO compute
- `Ophiolite Charts` owns rendering of chart-ready DTOs

The script and notebook here demonstrate the same workflow:

1. create or open a project
2. ingest LAS into a well/wellbore context
3. resolve canonical elastic channels (`Vp`, `Vs`, `Rho`)
4. optionally materialize derived velocity logs for downstream reuse
5. build elastic layers from fixed bins, explicit edges, or top-set intervals
6. run Zoeppritz AVO
7. write chart-ready AVO response source JSON

## Data Modes

### 1. Real LAS pair

The script will use the following environment variables when present:

- `OPHIOLITE_LOG_AVO_DSI_LAS`
- `OPHIOLITE_LOG_AVO_DENSITY_LAS`

If they are unset, the script falls back to the current local F3 F02-A-05 candidates when those files exist:

- `/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/f02a05_20030103-1-06_a5_main_log_dsi_030lup.las`
- `/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/f02a05_20030103-1-06_lwd2259a5rt.las`

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

## Outputs

Each run writes:

- `workflow_summary.json`
- `avo_fixed_interval_source.json`
- `avo_explicit_edges_source.json`
- `avo_top_set_source.json` when a top set is available

The JSON outputs are chart-ready DTOs for the AVO chart family. They are meant to be consumed by chart wrappers, not generated inside them.

## Notebook

`log_avo_workflow.ipynb` reuses the script helpers and is the recommended customer-facing walkthrough surface.
