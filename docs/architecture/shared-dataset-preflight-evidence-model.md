# Shared Dataset Preflight Evidence Model

Date: 2026-04-30

## Purpose

This note sketches the implementation boundary for moving current SEG-Y-heavy
preflight/import evidence and MDIO source evidence into shared
`DatasetPreflightRequest`, `DatasetPreflightResponse`, and `DatasetImportPlan`
contracts.

The target is not a generic import manager. The target is a small
runtime-owned evidence model that CLI, Python, TraceBoost, fixtures, and future
reports can consume without knowing whether the source was SEG-Y, MDIO/Zarr, or
another adapter.

## Current Inputs To Preserve

Current SEG-Y flow already provides useful evidence:

- source identity and file fingerprint
- trace count, samples per trace, sample interval, sample format, endianness,
  inspection warnings
- resolved inline/crossline/third-axis mapping
- classification, stacking state, organization, layout, gather axis kind
- observed versus expected trace count, completeness, missing bins, duplicate
  coordinates
- sample-data fidelity and potentially lossy conversion notes
- candidate geometry mappings and plan provenance
- validation issues, recommended wizard stage, storage risk summary, and
  validation fingerprint

Current MDIO flow has source evidence but not the same shared preflight surface:

- source detection by `.mdio` extension or Zarr group with `/seismic`
- seismic array shape and axis names
- optional `/trace_mask` occupancy evidence
- subset resolution for inline, xline, and sample windows
- tile/chunk recommendation and TBVOL storage estimate
- coordinate arrays, sample-axis attributes, inferred time/depth domain, and
  optional spatial/CRS metadata

Domain-heavy choices requiring review: whether MDIO axis-name interpretation,
sample-axis unit inference, and spatial descriptor derivation are mature enough
to expose as canonical evidence instead of adapter detail.

## Proposed Types

Names are illustrative Rust contract names; JSON should use stable snake_case.

```rust
pub struct DatasetPreflightRequest {
    pub schema_version: u32,
    pub source: DatasetSourceRef,
    pub adapter_hint: Option<DatasetAdapterId>,
    pub requested_subset: Option<DatasetSubsetRequest>,
    pub import_intent: DatasetImportIntent,
    pub adapter_options: serde_json::Value,
}

pub struct DatasetSourceRef {
    pub uri: String,
    pub display_name: Option<String>,
    pub source_kind_hint: Option<String>,
}

pub struct DatasetPreflightResponse {
    pub schema_version: u32,
    pub source: DatasetSourceSummary,
    pub adapter: DatasetAdapterSummary,
    pub canonical_preview: DatasetCanonicalPreview,
    pub storage_estimate: Option<DatasetStorageEstimate>,
    pub import_plan: DatasetImportPlan,
    pub warnings: Vec<ImportWarning>,
    pub blockers: Vec<ImportBlocker>,
    pub evidence_fingerprint: String,
    pub adapter_detail: serde_json::Value,
}

pub struct DatasetImportPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub source_fingerprint: String,
    pub adapter_id: DatasetAdapterId,
    pub output: DatasetImportOutputPlan,
    pub subset: Option<DatasetSubsetPlan>,
    pub structure: DatasetStructurePlan,
    pub spatial: DatasetSpatialPlan,
    pub policy: DatasetImportPolicy,
    pub provenance: DatasetImportPlanProvenance,
    pub adapter_detail: serde_json::Value,
}
```

Recommended canonical preview fields:

- `dataset_kind`: initially `seismic_volume`
- `classification`: `regular_dense`, `regular_sparse`,
  `duplicate_coordinates`, `ambiguous_mapping`, `non_cartesian`, or
  adapter-equivalent structural class
- `stacking_state`: `post_stack`, `pre_stack`, `unknown`
- `organization`: `binned_grid`, `gather_collection`, `unstructured`
- `layout`: existing seismic layout vocabulary
- `gather_axis_kind`: optional existing gather-axis vocabulary
- `shape`: logical `[inline, crossline, sample]` or equivalent
- `sample_axis`: count, interval/coordinates summary, unit, domain
- `sample_data_fidelity`: existing fidelity object
- `coordinate_binding`: detected/effective/source summary, optional
- `trace_evidence`: observed count, expected count, completeness ratio,
  missing-bin count, duplicate-coordinate count, occupancy availability

Domain-heavy choices requiring review: names and thresholds for
`classification`, canonical handling of 2D and prestack gather sets, and whether
MDIO trace-mask presence should map directly to `occupancy_available`.

## Warnings And Blockers Taxonomy

The shared taxonomy should stay limited to structural engineering evidence. It
must not encode interpretive geoscience quality judgements.

```rust
pub enum ImportDiagnosticSeverity {
    Info,
    Warning,
    Blocking,
}

pub struct ImportWarning {
    pub code: String,
    pub message: String,
    pub target: Option<String>,
    pub evidence_ref: Option<String>,
}

pub struct ImportBlocker {
    pub code: String,
    pub message: String,
    pub target: Option<String>,
    pub required_action: ImportRequiredAction,
    pub evidence_ref: Option<String>,
}
```

Initial shared codes:

- `source_unreadable`
- `adapter_not_found`
- `source_fingerprint_failed`
- `unsupported_layout`
- `structure_mapping_incomplete`
- `structure_mapping_ambiguous`
- `duplicate_coordinates`
- `non_cartesian_structure`
- `sparse_regularization_requires_acknowledgement`
- `subset_out_of_bounds`
- `sample_axis_unresolved`
- `potentially_lossy_sample_conversion`
- `storage_estimate_exceeds_capacity`
- `output_destination_missing`
- `output_destination_exists`
- `validation_fingerprint_mismatch`

Severity is contextual. For example, sparse structure can be a blocker until
regularization is selected, then a warning requiring acknowledgement. Existing
SEG-Y `SegyImportIssue` values can map into this list while remaining available
in adapter detail.

Domain-heavy choices requiring review: whether CRS absence is structural enough
to warn in this model. A conservative first pass should expose CRS detection
state without blocking import unless a caller explicitly requires spatial
activation.

## Adapter Detail Policy

Canonical fields must describe dataset meaning and import readiness. Adapter
detail may describe how that evidence was derived.

Allowed in `adapter_detail`:

- SEG-Y header byte/value-type mappings, candidate mappings, field
  observations, endianness, sample format code, strict/lenient reader policy,
  scan-stage recommendations, and existing `SegyImportPlan` compatibility
  payloads
- MDIO/Zarr array paths, dimension names, chunk metadata, trace-mask path,
  source template names, cloud/local source hints, and subset index windows
- adapter version, detection confidence, and raw inspection warnings

Not allowed as canonical fields:

- SEG-Y byte offsets
- MDIO internal array paths
- portal-specific access quirks
- app wizard stages
- TraceBoost command names or desktop file-grant state

`adapter_detail` should be internally tagged by `adapter_id` and
`adapter_schema_version`. Consumers may log and render it, but import readiness
must be derivable from canonical preview, warnings, blockers, and plan policy.

## Ownership Boundaries

Ophiolite seismic runtime owns:

- adapter detection and dispatch
- preflight execution
- source fingerprints and evidence fingerprints
- canonical preview construction
- storage estimates
- shared warning/blocker taxonomy
- plan validation and validation fingerprints
- stable JSON serialization for CLI, Python, TraceBoost, tests, and reports

Format adapters own:

- source parsing and inspection
- SEG-Y mapping candidates and reader policy
- MDIO/Zarr layout, arrays, chunking, subset resolution, and template metadata
- adapter-specific `adapter_detail`
- conversion from adapter-native evidence into canonical preview fields

TraceBoost owns:

- import-manager sessions, dialogs, grants, recent sources, activation, and
  workspace registry updates
- app-local recipes and workflow reports
- mapping shared warnings/blockers into product UI
- compatibility with current SEG-Y review panels during migration

Ophiolite Charts owns no import policy. It may consume canonical previews or
committed dataset summaries after import.

## Consumers

CLI should expose direct evidence commands:

- `ophiolite dataset preflight <source>`
- `ophiolite dataset validate-plan <plan.json>`
- `ophiolite dataset import --plan <plan.json>`

Python should wrap the same Rust-owned behavior with typed return objects:

- `ophiolite.dataset.preflight(source, adapter_hint=None, subset=None)`
- `response.import_plan`
- `plan.validate()`
- `plan.import_to(output)`

TraceBoost should call the same preflight and plan-validation operations from
the Import Manager. SEG-Y pages can still render adapter detail for mapping
review, while MDIO can move from direct import toward the same evidence review
surface.

Workflow reports should persist request digests, response digests,
`evidence_fingerprint`, `validation_fingerprint`, warnings, blockers, storage
estimate, and final materialized dataset identity.

## Migration From Current SEG-Y Flow

1. Add shared dataset preflight contracts beside existing `SurveyPreflight*` and
   `SegyImport*` contracts.
2. Implement a SEG-Y adapter that internally calls existing preflight, scan, and
   validation functions, then maps their output into the shared response.
3. Preserve existing `SegyImportPlan` inside `DatasetImportPlan.adapter_detail`
   during the transition.
4. Teach TraceBoost Import Manager to request shared preflight first, then route
   `adapter_id = "segy"` responses to the existing SEG-Y review panel.
5. Add an MDIO adapter preflight that exposes layout, subset, occupancy, sample
   axis, spatial evidence, and storage estimates before direct import.
6. Move CLI/Python entry points to the shared model.
7. Deprecate direct public use of `SurveyPreflightRequest`,
   `SurveyPreflightResponse`, and SEG-Y-only validation after TraceBoost and
   tests consume the shared model.

During migration, no app-local code should recreate source fingerprints,
storage estimates, or structural validation rules.

## Non-Goals

- no code implementation in this note
- no generic import-manager session model
- no replacement for TraceBoost dialogs, activation, workspace state, or report
  rendering
- no public contract that exposes SEG-Y byte offsets or MDIO paths as canonical
  fields
- no geologic interpretation quality scoring
- no automatic CRS authority assignment beyond reporting detected/imported
  evidence
- no committed bulk seismic data
- no requirement that every adapter support every field on day one
