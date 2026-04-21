# ADR-0026: Vendor Project Import Adapters

## Status

Accepted

## Context

Ophiolite already imports open and industry-standard formats such as LAS and SEG-Y. Interpretation
projects from vendor ecosystems add another layer: a project directory can contain native seismic
volumes, interpreted horizons, well artifacts, pick sets, random lines, and vendor-specific session
state. Those artifacts matter because some useful data products only exist in the vendor-native
layout even when an open source input is also present.

The immediate pilot is OpendTect using the F3 Demo 2023 project. The broader requirement is to
support multiple vendor ecosystems over time, including Petrel, without forcing every vendor format
through ad hoc one-off commands.

## Decision

Ophiolite adds a vendor-project import scaffold with an explicit three-step flow:

- `scan`
- `plan`
- `commit`

The first implementation targets OpendTect project folders and stays read-only.

### Adapter boundary

Vendor project ingestion is modeled as a separate adapter protocol, not as a compute operator
payload and not as an extension of LAS file inspection. The adapter owns:

- vendor project discovery
- vendor object inventory
- canonical target recommendations
- blocking issues such as unresolved coordinate reference handling

### Flow contract

`scan` returns a schema-versioned preview of:

- vendor project metadata
- detected survey metadata
- detected coordinate reference when available
- vendor objects with stable ids, source paths, target recommendations, and default selections

`plan` freezes a user selection against the scanned project and produces:

- planned canonical imports
- bridge requests
- warnings
- blocking issues

`commit` consumes a frozen plan. In phase one, commit supports:

- dry-run validation for the full scanned inventory
- canonical persistence for OpendTect well-family objects
- canonical persistence for vendor objects that already expose an open companion format, such as
  OpendTect `Rawdata/Seismic_data.sgy`
- canonical persistence for bridge-backed vendor objects when the caller supplies a compatible
  bridge output
- raw-source preservation for vendor objects intentionally mapped to `RawSourceBundle`

Other vendor-native canonical targets remain planning-only until their payload bridges are added.

### Canonical mapping rules

The OpendTect pilot uses these target families:

- `.cbvs` seismic volumes -> `SeismicTraceData`
- `.well` trajectory/well geometry -> `Trajectory`
- `.wll` well logs -> `Log`
- `.wlm` markers -> `WellMarkerSet`
- `.wlt` and `.csmdl` -> `WellTimeDepthModel`
- `.hor`, `.hcs`, `.hci` -> survey-store horizon attachments
- `.flt`, `.body`, `.pck`, `.rdl`, and session-like artifacts -> `RawSourceBundle` until a better
  canonical family exists
- open inputs already present in the vendor project, such as SEG-Y or shapefiles, remain preferred
  import routes when they satisfy the need directly

### Bridge contract rule

When a vendor-native payload cannot be imported directly, `plan` may emit one or more bridge
requests. A bridge request names:

- the vendor object id
- the vendor-native object id when one is discoverable
- the bridge kind
- the recommended output format
- the accepted output formats for phase-one commit
- the output formats that currently support automatic execution
- the runtime requirements needed to execute the bridge automatically

`commit` may then consume a bridge output supplied by the caller. For the OpendTect pilot, CBVS
volumes use:

- bridge kind: `opendtect_cbvs_volume_export`
- recommended output format: `segy`
- accepted output formats: `segy`, `tbvol_store`, `zarr_store`, `open_vds_store`
- automatic execution formats: `segy`
- runtime requirements: `vendor_batch_executable`, `vendor_project_data_root`

Ophiolite also exposes a `vendor-bridge-run` step for bridge kinds that can be prepared
programmatically. For the OpendTect CBVS pilot, that runner:

- discovers the OpendTect native storage id from vendor sidecars such as `Proc/*.par` or
  `Attribs/*.attr`
- generates an OpendTect batch parameter file for `od_process_segyio`
- resolves the OpendTect batch executable from an explicit path, an installation root, `DTECT_APPL`,
  or `PATH`
- returns a bridge output contract compatible with `commit`
- optionally executes the batch export when an OpendTect binary is available in the caller's
  environment

This keeps the canonical import surface stable while still allowing bridge execution to remain
optional and vendor-specific.

For callers that want a single workflow boundary, Ophiolite also exposes a chained
`vendor-bridge-commit` step that composes `plan -> vendor-bridge-run -> commit` for a selected
vendor object. In environments without an installed vendor binary, this still supports dry-run
validation when the bridge output path has already been materialized externally.

Ophiolite also exposes `vendor-bridge-capabilities <vendor>` to return the registered bridge
capability catalog for a vendor. This is the current discovery boundary for bridge kinds, accepted
target formats, automatic-execution support, and runtime prerequisites.

Ophiolite also exposes `vendor-connector-contract <vendor>` to return the vendor-level connector
contract: phased execution boundaries, supported runtime kinds, bridge capabilities, and
provenance guarantees. This is the reusable backend surface future vendor adapters should conform
to before vendor-specific discovery or extraction logic is added. As of 2026-04-17, both
`opendtect` and `petrel` are registered through this boundary. The current Petrel implementation
supports export-bundle discovery and planning plus phase-one canonical commit for single-well log,
trajectory, tops, and checkshot imports. Petrel runtime probe and bridge execution remain
unimplemented, and horizon-point exports currently preserve as raw source bundles. The follow-on
catalog owner-model migration for survey- and project-scoped assets is tracked in ADR-0027.

For runtime-backed connector diagnostics, Ophiolite also exposes `vendor-runtime-probe` with an
explicit request payload. The first implementation targets OpendTect's ODBind Python runtime and is
intended to answer a separate question from metadata scan: whether the vendor runtime can see the
survey and open the bridgeable objects that metadata discovery already found. `vendor-plan` may
also carry an optional `runtimeProbe` request and embed the structured runtime result inline so
planners and orchestrators can consume bridgeability and runtime viability in one call.

### Provenance rule

Vendor identity and vendor object ids are preserved as provenance and source metadata. They do not
become canonical Ophiolite asset ids.

### Coordinate-reference rule

When the vendor project declares a coordinate reference explicitly, Ophiolite carries it forward as
the detected default. Geometry-bearing canonical imports remain blocked when no effective
coordinate-reference decision is available.

## Consequences

### Positive

- Ophiolite gets a reusable boundary for vendor ecosystems instead of format-specific sprawl
- the same `scan -> plan -> commit` workflow can be extended to Petrel and other project layouts
- the F3 OpendTect project becomes a concrete pilot for native seismic, interpretation, and well
  object mapping
- read-only discovery and dry-run planning reduce risk while broader canonical mappings mature
- the F3 pilot now exercises real canonical import for trajectories, logs, marker sets, and
  time-depth models
- the same contract now previews Petrel export bundles as trajectories, logs, tops, checkshots,
  and horizon point exports without changing the API shape
- Petrel now proves the same contract can land real canonical assets for a single selected well
  without adding vendor-specific CLI surface area
- Petrel horizon point exports now have a survey-targeted canonical path when callers supply a
  `targetSurveyAssetId`, while still retaining the original Petrel export files as provenance
- raw-source-only vendor commits no longer require callers to fabricate a dummy binding; Ophiolite
  now routes preserved source bundles into a stable system-owned project archive wellbore while the
  project model still requires wellbore ownership

### Tradeoffs

- the first phase materializes the OpendTect well family, open companion seismic volumes, and
  bridge-backed CBVS imports when a compatible bridge output is provided
- Petrel canonical commit is intentionally narrow in phase one: one well per request for logs,
  trajectory, tops, and checkshots only
- some targets, especially interpreted surfaces and structural bodies, still map to preservation
  bundles rather than rich canonical families
- CBVS and other vendor-native payloads still require bridge implementations before they can commit
  from the original vendor bytes alone
- Petrel checkshot sign conventions are inferred from the export values today
- Petrel horizon-point exports currently rely on a Petrel-specific normalization step that rewrites
  `id id x y z name` rows into canonical XYZ before survey-horizon import; richer interpretation
  families such as faults and bodies still need their own canonical mappings and ownership rules
- the current project asset model is still wellbore-owned, so the project archive remains an
  explicit compatibility lane rather than a true project/survey-scoped asset owner

## Industry Pattern Fit

The current Ophiolite design is closest to a materialized connector-and-staging pattern, not to a
query federation or zero-copy sharing pattern.

Comparable patterns in current data platforms:

- query federation:
  Databricks Lakehouse Federation and Snowflake external tables expose foreign data through a
  governed catalog boundary while minimizing data movement
- governed sharing:
  Delta Sharing exposes an open protocol so providers share once and many consumers connect without
  a custom export tool per recipient
- materialized connectors:
  platforms such as Airbyte use connector SDKs, capability-specific sync logic, and connector test
  suites to materialize source-native data into a canonical destination contract

For vendor interpretation projects such as OpendTect or Petrel, Ophiolite should primarily follow
the materialized connector pattern because:

- the source assets are often file-backed and workstation-native rather than remotely queryable
- canonical Ophiolite assets need durable local persistence, not just transient read access
- the same vendor object may need different bridge routes depending on the target, such as SEG-Y,
  tbvol, zarr, or OpenVDS

This means the bridge architecture should be judged against modern connector systems rather than
against warehouse federation systems.

## Follow-on Standards

To stay aligned with modern connector practice as the vendor surface grows, follow-on work should
add:

- a connector capability registry:
  implemented in phase one for bridge kinds, accepted targets, and execution prerequisites; extend
  it next to cover lossiness, provenance policy, and future vendor families
- a bridge artifact manifest:
  every executed bridge should emit source ids, source paths, output paths, hashes, executable
  identity, bridge version, and execution timestamps for replay and audit
- runtime isolation:
  vendor binaries should run in a dedicated worker boundary, ideally with controlled environment
  setup instead of relying on ambient workstation state
- phase separation inside a connector:
  keep discovery and lightweight inspection independent from payload extraction so metadata can stay
  available even when a vendor export runtime is missing or unstable
- supported-surface preference:
  prefer vendor-supported library or SDK surfaces for discovery and inspection, and reserve batch
  executables or UI automation for payload extraction when no better programmatic surface exists
- conformance fixtures:
  each vendor adapter should have fixture-backed scan, plan, bridge, and commit tests plus at least
  one real acceptance dataset such as F3
- dual-mode strategy:
  prefer zero-copy or open-protocol integration when a vendor ecosystem actually exposes one, and
  fall back to materialized bridges only where canonical persistence requires it

The present `scan -> plan -> vendor-bridge-run -> commit` structure remains a sound base for this.
The main gap is not the staged workflow itself, but the absence of a richer connector manifest and
runtime contract around it.

## Non-goals

This ADR does not add:

- executable vendor-native payload readers in every supported language immediately
- a generic plugin marketplace for arbitrary vendor import code
- canonical ids derived from vendor ids
- automatic CRS inference when the vendor project does not declare one explicitly
