# Ophiolite Seismic Processing Golden Path

This example is the seismic analogue of the log-to-AVO golden path.

It stays out of the Svelte wrapper demo and instead exposes a Python-first SDK workflow that mirrors how a user would actually work with a seismic volume:

1. preflight a SEG-Y file
2. import it into a TraceBoost store
3. open the dataset summary
4. load a section view
5. preview a trace-local processing pipeline
6. materialize the processed dataset
7. load the processed section view

The public nouns are the ones a seismic user expects to reason about:

- `dataset`
- `section`
- `pipeline`
- `bandpass_filter`
- `agc_rms`

In Python those show up as:

- `TraceBoostApp`
- `SeismicDataset`
- `SectionSelection`
- `TraceProcessingPipeline`

```python
from ophiolite_sdk.seismic import TraceBoostApp, TraceProcessingPipeline

app = TraceBoostApp()
dataset = app.import_segy("small.sgy", "small.tbvol", overwrite_existing=True)
section = dataset.midpoint_section(axis="inline")
pipeline = (
    TraceProcessingPipeline.named("Bandpass + RMS AGC")
    .bandpass(8.0, 12.0, 45.0, 60.0)
    .agc_rms(40.0)
)
preview = dataset.preview_processing(section, pipeline)
processed = dataset.run_processing(
    pipeline,
    output_store_path="small_bandpass_agc.tbvol",
    overwrite_existing=True,
)
```

The chart handoff is also explicit: the workflow writes `SectionView` payloads that can be handed to the Ophiolite Charts `SeismicSection` surface. The preview payload is the same chart-ready section nested under `preview.section`.

## Run It

From the repo root:

```bash
python3 examples/golden_paths/seismic_processing/seismic_processing_workflow.py \
  --run-root examples/golden_paths/seismic_processing/.generated/small \
  --overwrite
```

By default the script looks for a local SEG-Y fixture in this order:

1. `OPHIOLITE_SEISMIC_GOLDEN_PATH_SEGY`
2. `test-data/small.sgy`
3. `test_data/small.sgy`
4. `../TraceBoost/test-data/small.sgy`
5. `/Users/sc/Downloads/SubsurfaceData/blocks/F3/f3_dataset.sgy`

Use a real dataset explicitly when needed:

```bash
python3 examples/golden_paths/seismic_processing/seismic_processing_workflow.py \
  --segy-path /Users/sc/Downloads/SubsurfaceData/blocks/F3/f3_dataset.sgy \
  --run-root examples/golden_paths/seismic_processing/.generated/f3 \
  --overwrite
```

## Outputs

The script writes:

- `preflight.json`
- `import.json`
- `dataset.json`
- `raw_inline_section.json` or `raw_xline_section.json`
- `pipeline.json`
- `preview_request.json`
- `preview_bandpass_agc_section.json`
- `run_request.json`
- `processed_dataset.json`
- `processed_inline_section.json` or `processed_xline_section.json`
- `workflow_summary.json`

`workflow_summary.json` is the quickest entry point. It captures:

- the resolved SEG-Y geometry
- the dataset descriptor
- the chosen section coordinate
- the processing pipeline name and operator ids
- amplitude statistics for the raw, preview, and processed sections
- the output paths for the chart payloads

## Notes

- The example uses `bandpass_filter` plus `agc_rms`. If someone says “AVC” for this step, the current implemented operator in TraceBoost is `agc_rms`.
- The script defaults to the midpoint section along the selected axis unless `--section-index` is supplied.
- The same workflow works with larger real volumes; the `small.sgy` fixture is there to keep verification fast.
