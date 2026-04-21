# External Import Acceptance

Run the real-data acceptance pass against the Netherlands well-source corpus and horizon XYZ corpus:

```bash
scripts/external-import-acceptance.sh
```

Optional overrides:

```bash
F02_WELLS_ROOT=/path/to/F02_wells_data \
HORIZONS_ROOT=/path/to/horizons \
HORIZON_STORE_PATH=/path/to/active-survey.tbvol \
scripts/external-import-acceptance.sh /tmp/external-import-acceptance
```

Artifacts:

- `<first-well-folder>-source-preview.json`
- `<first-well-folder>-source-summary.json`
- `<second-well-folder>-source-preview.json`
- `<second-well-folder>-source-summary.json`
- `horizons-inspect.json`
- `horizons-summary.json`
- `horizons-source-preview.json` when `HORIZON_STORE_PATH` is set
- `horizons-source-summary.json` when `HORIZON_STORE_PATH` is set

What this covers:

- selected well-source preview parsing for the first two discovered well folders under `F02_WELLS_ROOT`, covering metadata, LAS logs, NLOG ASCII tables, tops, trajectory, and preserved unsupported sources
- default log-selection hints for likely duplicate LAS families
- horizon XYZ parse-only inspection without requiring a survey store
- optional horizon source preview against an active survey store using the draft lifecycle

TraceBoost demo import scope:

- users select specific source files, not a full source folder as a required unit of upload
- import preview is best-effort and incomplete files are still accepted into the review flow when they yield usable partial slices
- canonical translation is bounded by the parsed evidence plus explicit user confirmation, supplementation, and edits in the confirmation dialog
- unmapped or incomplete source content stays preserved as source-only rather than blocking the entire import
- geometry does not silently fall back to WGS84 or any other default CRS; missing CRS keeps geometry unresolved until the user confirms survey CRS or enters a source CRS
- horizon XYZ files support parse-only review without an active survey store, while final import remains gated on an explicit survey-aligned CRS path

Current expectation:

- `F02-A-02` should preview with selected LAS plus three mapped NLOG ASCII tables
- `F02-A-02-S1` should preview with selected LAS, tops, and preserved DLIS sources
- horizon XYZ inspection should parse the selected files with zero invalid rows
- horizon source preview should only be expected when a compatible survey store is available locally
