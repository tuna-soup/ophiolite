# OpendTect F3 Bridge Assessment

Date: 2026-04-17

## Goal

Assess whether Ophiolite should bridge OpendTect projects through:

1. file-level metadata parsing,
2. vendor-supported runtime APIs,
3. vendor batch export tools, or
4. a future native reader for vendor file formats such as CBVS.

The working dataset is `/Users/sc/Downloads/F3_Demo_2023` from the public F3 Demo 2023 release.

## What worked

- Ophiolite's file-level scan of OpendTect project metadata is reliable enough to discover the survey and plan bridge work from `.omf` and related sidecar files.
- The shipped `odbind` Python layer can see the survey when it is mounted under a proper OpendTect data root and can return survey metadata and object names.
- The public OpendTect source and documentation confirm that `ODBind` is a supported plugin surface and that `Seismic3D` is intended to read rectangular subvolumes from existing 3D seismic data.

## What failed

- Running `od_process_segyio` directly against the F3 survey export parameter file ended in `SIGSEGV`.
- Re-running after an installed-style `setup.od` bootstrap still ended in `SIGSEGV`.
- Re-running against a fully copied data root instead of a symlinked survey still failed.
- `odbind` could list 3D seismic object names, but opening `Seismic3D(survey, "7a AI Cube Std")` failed with `IO object read error`.
- `survey.has_object(...)` and `survey.get_object_info(...)` did not agree with `survey.get_object_names("Seismic Data")` for this bundle and dataset.

## Interpretation

The practical split is:

- discovery and lightweight inspection can be done without launching a brittle export path,
- actual seismic payload extraction is still blocked on vendor runtime behavior on this macOS setup.

That means the backend should not model "vendor bridge" as one monolithic operation. It should model separate phases:

1. discovery,
2. capability negotiation,
3. extraction,
4. canonical ingest.

## What the public OpendTect surfaces suggest

- The OpendTect source tree exposes `plugins/ODBind/odseismic_3d.cc` and Python examples for `Seismic3D`, which indicates the vendor intends a library-style access path for existing volumes, not only batch export.
- The ODBind docs describe `Seismic3D` as able to read rectangular 3D subvolumes and return them as arrays/Xarray datasets.
- The OpendTect repository is open and plugin-oriented, which makes selective source-level investigation of CBVS and translator behavior feasible later.

## Comparison to modern connector practice

The current direction in Ophiolite is broadly right, but it should be tightened around the patterns used by modern data platforms:

- Databricks Lakeflow Connect separates connection/runtime concerns from ingestion state. It emphasizes incremental ingestion, explicit networking/deployment choices, cursor-based failure recovery, and operational monitoring.
- Snowflake's Native SDK for Connectors models ingestion around resources, ingestion definitions, and ingestion processes. It also explicitly separates pull and push patterns, with an agent for environments where direct access is not appropriate.
- Airbyte standardizes connector behavior behind a protocol and tests connectors through stable phases like `spec`, `check`, `discover`, and `read`/`write`.

The common pattern is not "one importer per format". It is:

- a machine-readable capability contract,
- explicit runtime requirements,
- a discovery phase,
- a validated extraction phase,
- canonical landing with state/provenance,
- isolated execution for vendor-specific runtimes.

## Recommended direction for Ophiolite

1. Keep the capability registry and extend it.
   Add lossiness markers, provenance fields, and runtime transport details alongside accepted output formats.

2. Split OpendTect into at least two adapters.
   One adapter should own metadata discovery and inspection.
   A separate adapter should own payload extraction.

3. Treat vendor runtimes as external workers.
   Batch executables, Python plugin stacks, or future native vendor SDKs should run out-of-process with explicit environment contracts.

4. Preserve canonical ingest as a separate boundary.
   The bridge should produce an artifact plus provenance, not silently collapse vendor extraction and canonical materialization into one opaque step.

5. Prefer vendor-supported library surfaces over GUI/batch automation when possible.
   For OpendTect, `odbind` is the first candidate to keep probing before committing to reverse-engineering CBVS directly.

6. Design the next vendor adapters from the same template.
   Petrel, Kingdom, or other vendor imports should plug into the same phases and capability model rather than adding bespoke one-off code paths.

## Immediate next steps

1. Keep `opendtect_cbvs_volume_export` as a capability-gated bridge, but mark automatic execution as runtime-dependent rather than assumed.
2. Add an `odbind`-backed discovery path for surveys, object names, and lightweight metadata capture.
3. Embed optional runtime probing into `vendor-plan` so orchestration can receive discovery, bridge requirements, and runtime-open diagnostics in one response.
4. Record extraction provenance and failure mode in bridge results so vendor-runtime instability is visible in the project history.
5. Defer a native CBVS reader until either:
   - `odbind` can actually open seismic payloads reliably on a supported runtime, or
   - source-level CBVS inspection shows a stable read path that is worth owning ourselves.

## Current backend status

As of 2026-04-17, Ophiolite now supports:

- a vendor-level connector contract endpoint,
- a vendor bridge capability registry,
- a standalone `vendor-runtime-probe` command,
- an optional `runtimeProbe` payload on `vendor-plan` that embeds runtime visibility and object-open results inline,
- a Petrel export-bundle discovery path that previews trajectories, logs, tops, checkshots, and horizon point exports through the same contract,
- a phase-one Petrel canonical commit path for a single selected well's LAS logs, `.dev`
  trajectory, tops exports, and checkshots.

Running `vendor-scan petrel '/Users/sc/Downloads/Petrel Data'` against the public F3 Petrel export
bundle currently returns:

- 48 preview objects,
- an inferred coordinate-reference hint of `ED50-UTM31`,
- 4 horizon-point previews that are not default-selected and now preserve as raw source bundles in phase one,
- 2 non-blocking issues on the sample bundle:
  `petrel_duplicate_tops_exports` and `petrel_export_unclassified`.

Running `vendor-plan` with `runtimeProbe` against the public F3 dataset confirmed the same behavior as the standalone probe:

- survey discovery succeeds,
- runtime object listing succeeds for 15 CBVS volumes,
- every bridgeable CBVS volume fails to open through `odbind.seismic3d` with `translator group not found` and `IO object read error -`.

That is the right shape for a modern connector boundary: plan first, runtime-validate second, and only then attempt extraction.

Running Petrel against the public export bundle now also shows the intended phase-one boundary:

- planning the default selected Petrel objects blocks on
  `petrel_multi_well_selection_unsupported`,
- planning a focused `A10` request for `petrel-log:A10`, `petrel-tops:A10`,
  `petrel-trajectory:A10`, and `petrel-checkshot:A10` succeeds when the inferred `ED50-UTM31`
  coordinate reference is carried forward,
- committing that focused plan into a fresh Ophiolite project imports 4 canonical assets, with
  validation reports showing 1 LAS-backed log collection, 7 tops rows, 6011 trajectory rows, and
  7 checkshot observations,
- planning and committing a selected Petrel horizon-point export now succeeds through a
  survey-targeted canonical import path when `targetSurveyAssetId` is supplied; the commit
  normalizes Petrel's `id id x y z name` rows into canonical XYZ while preserving the original
  export file as provenance,
- raw-source-only vendor commits can now omit `binding`; Ophiolite routes preserved source bundles
  into a stable system-owned project archive wellbore while the underlying project asset model
  still requires wellbore ownership; the target owner-model migration is now captured in
  `docs/architecture/ADR-0027-asset-owner-scopes-for-vendor-and-survey-assets.md`,
- the public `CheckShots1.txt` export carries negative time/depth values after the origin sample,
  so the current canonical mapping infers Petrel's exported sign convention and normalizes those
  samples before commit; that assumption should still be validated against a Petrel-side reference
  before broadening scope.

## Decision points

1. Should Ophiolite treat vendor import as one opaque importer or as phased connectors?
   Recommended answer: phased connectors.
   Reason: this matches the Databricks, Snowflake, and Airbyte pattern of separating discovery, configuration, execution, and monitoring.

2. Should runtime validation be out-of-band or part of planning?
   Recommended answer: both, with `vendor-runtime-probe` standalone and `vendor-plan.runtimeProbe` inline.
   Reason: operators sometimes want an explicit diagnostic tool, while orchestrators usually want one structured planning response.

3. Should Ophiolite reverse-engineer CBVS now?
   Recommended answer: not yet.
   Reason: the vendor-supported runtime surface exists and should be exhausted first; native format ownership is expensive and should be justified by clear extraction blockers on supported runtimes.

4. Should future Petrel and other vendor integrations reuse the same backend contract?
   Recommended answer: yes.
   Reason: capability registry plus phased execution is the reusable part; only discovery and extraction workers should vary by vendor.

5. Should canonical ingest own vendor provenance?
   Recommended answer: yes.
   Reason: modern connector systems retain source identifiers, sync state, and failure evidence instead of collapsing everything into anonymous imported assets.

6. What should the next implementation priority be?
   Recommended answer: keep Petrel family-by-family and add the next blocked families deliberately.
   Reason: logs, tops, trajectory, and checkshots now land canonically on the real F3 export,
   while horizon points now have a preservation path but still need explicit policy and target-shape
   decisions before survey-aware canonical import.

7. What backend model change would make vendor preservation cleaner?
   Recommended answer: add project- or survey-scoped asset ownership for non-well artifacts.
   Reason: the current fallback still attaches preserved vendor artifacts to an inferred archive
   wellbore because the project schema is wellbore-owned end to end.

## Repro notes

- Temporary macOS batch install extracted from:
  `https://download.opendtect.org/relman/0.0.0/basebatch/8.1.0-20260214/basebatch_mac.zip`
- Data package extracted from:
  `https://download.opendtect.org/relman/0.0.0/basedata/8.1.0-20260214/basedata_mac.zip`
- Public source inspected from:
  `https://github.com/OpendTect/OpendTect`

Useful local probe:

```bash
export DTECT_APPL='/tmp/opendtect-batch/OpendTect 0.0.0.app/Contents'
export DYLD_LIBRARY_PATH='/tmp/opendtect-batch/OpendTect 0.0.0.app/Contents/Frameworks'
python3 scripts/validation/opendtect_odbind_probe.py \
  --odbind-root '/tmp/opendtect-batch/OpendTect 0.0.0.app/Contents/Resources/bin/python' \
  --basedir /tmp/opendtect-root-f3-copy \
  --survey F3_Demo_2023
```

For a `--volume` probe, use a Python environment that also has `xarray` installed because `odbind.seismic3d` imports it eagerly.

## Sources

- Databricks Lakeflow Connect:
  https://docs.databricks.com/aws/en/ingestion/lakeflow-connect
- Snowflake Native SDK for Connectors:
  https://docs.snowflake.com/en/developer-guide/native-apps/connector-sdk/about-connector-sdk
- Snowflake ingestion management:
  https://docs.snowflake.com/en/developer-guide/native-apps/connector-sdk/flow/ingestion-management/overview
- OpendTect ODBind docs:
  https://doc.opendtect.org/7.0.0/doc/odbind/autoapi/odbind/seismic3d/index.html
- OpendTect public source:
  https://github.com/OpendTect/OpendTect
- Airbyte protocol and connector testing references:
  https://github.com/airbytehq/airbyte-protocol
  https://airbyte.com/blog/how-we-test-airbyte-and-marketplace-connectors
