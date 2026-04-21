# Geospatial CRS Architecture, Survey Maps, and Backend Design Conclusions

## Audience and intent

This note is for senior software engineers working on seismic runtimes, local-first interpretation applications, and subsurface data platforms that need to place seismic surveys and wells correctly on a map.

It documents the geospatial architecture now implemented across `TraceBoost`, `Ophiolite`, and `Ophiolite Charts`, why that architecture is a good fit for this stack, what background problems it is trying to solve, and how it compares to several widely used open-source geospatial systems and libraries.

This is not a user guide. It is an engineering writeup about coordinate-reference-system ownership, survey-map resolution, reprojection policy, cache boundaries, and why those choices are more appropriate for this product than several tempting alternatives.

## Problem statement

The seismic runtime can now derive stable plan-view map geometry from imported SEG-Y and runtime stores:

- survey footprint
- inline/xline to projected XY affine geometry
- future well overlays

That solves only half of the geospatial problem.

The harder half is coordinate-reference identity and transformation:

- a survey can have usable XY coordinates but no trustworthy CRS identifier
- different surveys in one workspace may use different native CRSs
- wells and seismic cannot be overlaid safely if one asset's CRS is assumed incorrectly
- a frontend can draw something plausible long before it can draw something auditable

That creates the core architectural risk:

> if CRS truth is not modeled canonically in the backend, application layers start inventing their own spatial rules.

That drift is expensive. It produces maps that look correct until the first mixed-survey workspace, well overlay, GIS export, or basemap alignment task exposes the hidden assumptions.

## What the system needs to support

The stack needs to support five distinct requirements:

1. A survey should retain its native geospatial truth even when the UI wants to display it in another CRS.
2. A workspace should be able to choose one display CRS for map composition.
3. Unknown native CRS must remain explicit until it is assigned deliberately.
4. Reprojection must be traceable, diagnosable, and cacheable.
5. Rendering code should consume resolved geometry, not own CRS policy.

Those requirements sound straightforward, but they imply a specific ownership model.

## The architectural boundary we implemented

The implemented design is:

- `Ophiolite` owns native CRS, effective CRS, reprojection, transform diagnostics, and derived display geometry.
- `TraceBoost` owns only workspace display CRS and the user workflow around overrides and warnings.
- `Ophiolite Charts` stays CRS-agnostic and renders resolved geometry only.

More concretely:

- seismic assets now carry canonical coordinate-reference binding metadata
- that binding distinguishes:
  - detected native CRS
  - effective native CRS
  - provenance of the effective CRS
- survey-map resolution accepts an optional requested display CRS
- survey-map results return:
  - native spatial geometry
  - display spatial geometry
  - transform status
  - transform diagnostics
- display geometry is treated as a derived artifact, not as asset truth

This is the central design choice. Everything else follows from it.

## Native, effective, and display CRS

The design deliberately separates three concepts that many systems blur together.

### Native CRS

This is the CRS the survey or well is actually authored in.

If it is known, it belongs to the asset.
If it is not known, that uncertainty also belongs to the asset.

### Effective native CRS

This is the native CRS the backend should use operationally.

Usually:

- `effective = detected`

But if a user assigns a correction:

- `effective = user override`
- `detected` is still preserved for auditability

This avoids the common failure mode where a UI correction destroys the ingest truth.

### Display CRS

This is a workspace choice.

It is not the survey's truth. It is the coordinate system the application wants to use for cross-asset map display.

That distinction matters because it lets us support:

- mixed-native-CRS workspaces
- native-space debugging
- display-space overlays
- future export policies with stricter transform requirements

## Why this is a good fit for TraceBoost and Ophiolite

This architecture fits the product for several reasons.

### 1. It matches the local-first runtime model

`TraceBoost` is not a thin GIS client over a remote geospatial service. It is a local desktop application with a local seismic runtime and local project state.

That means:

- asset metadata should live with the assets/runtime
- workspace display preferences should live with the workspace
- transform caches should stay near the runtime, not inside the renderer

The current design follows that naturally.

### 2. It keeps domain truth out of the chart layer

It is tempting to let the map widget infer coordinates, remember overrides, or do reprojection itself.

That is the wrong boundary.

`Ophiolite Charts` should not need to know:

- whether a CRS was detected or overridden
- whether a transform was degraded
- whether the display geometry was cached
- how PROJ selected an operation

It should receive resolved geometry and render it.

That keeps the rendering layer simple and the geospatial semantics centralized.

### 3. It preserves auditability

The design stores:

- detected CRS
- effective CRS
- source of the effective CRS
- transform diagnostics

That makes it possible to answer real engineering questions later:

- Was this survey map shown in native or display coordinates?
- Did the user override the CRS?
- Which source and target CRS were used?
- Was the transform degraded?
- Was the displayed geometry loaded from cache?

Those questions matter in subsurface workflows, where "close enough" map behavior is often not good enough.

### 4. It supports gradual rollout

The architecture allows staged implementation.

Phase 1:

- native/effective/display contract split
- override workflow
- native-space display when needed

Phase 2:

- PROJ-backed reprojection
- transform diagnostics
- display-geometry cache

Phase 3:

- wells with absolute surface location and CRS
- real survey plus well overlays across mixed native CRSs

That is materially safer than trying to solve all geospatial concerns in one step.

## The background seismic issue this design addresses

The immediate seismic problem is not only geometry extraction. It is geometry extraction plus missing or unreliable CRS identity.

In practice:

- SEG-Y often contains usable XY coordinates
- those coordinates may require `SCALCO` handling
- the file may still not contain trustworthy CRS identity
- some workflows also involve local or operator-specific coordinate systems

That means a survey can be mappable before it is fully geodetically identified.

The system therefore needs to support this state honestly:

- geometry available
- CRS identity unknown or user-assigned later

The implemented design does exactly that. It does not block all map functionality on perfect metadata, but it also does not pretend uncertain metadata is authoritative.

## Why we did not make the project CRS the asset CRS

One alternative was to let the TraceBoost workspace define the seismic CRS by default.

That would be simpler in the short term.
It would also be wrong.

Why it was rejected:

- it turns a display preference into domain truth
- it hides mixed-CRS datasets instead of modeling them
- it makes audits and support harder
- it makes reuse of the same store in another app ambiguous
- it encourages silent misalignment between surveys and wells

In other words, it reduces UI friction by introducing semantic debt into the data model.

That is the wrong trade.

## Why reprojection belongs in Ophiolite

Another alternative was to let TraceBoost or `Ophiolite Charts` do reprojection directly.

That was also rejected.

Why:

- reprojection is not presentation logic
- transform policy needs access to provenance and native/effective CRS binding
- diagnostics and cache identity belong with the runtime
- other consumers besides TraceBoost will need the same behavior

If the transform engine lived in the app layer:

- each client would need to duplicate CRS policy
- transform diagnostics would fragment
- caches would diverge
- bugs would become client-specific instead of backend-fixable

Putting reprojection in `Ophiolite` avoids all of that.

## Why display geometry is cached separately

Display geometry is derived from:

- native geometry
- effective native CRS
- requested display CRS
- transform policy
- PROJ engine and data version

That means it is request-dependent derived data, not canonical asset metadata.

So the current design treats it as a small derived cache artifact rather than mutating the source manifest or store metadata with display-space coordinates.

That is the correct fit because:

- native truth stays clean
- cache invalidation becomes key-based
- backend diagnostics can travel with the artifact
- the cache stays tiny and inspectable

This is also more consistent with the rest of the local runtime design than pushing ephemeral display results into immutable or semi-canonical store metadata.

## Comparison to popular open-source systems

The current design is not novel for novelty's sake. It is aligned with how mature open-source geospatial systems separate concerns.

### PROJ

PROJ is the reference engine for coordinate operations. Its model is:

- explicit source and target CRS
- late selection of the actual coordinate operation
- optional area-of-interest-aware operation choice
- support for grid-based datum shifts and transformation resources

This strongly supports our decision to keep reprojection in the backend and to attach diagnostics to the resolved result rather than treating reprojection as a trivial frontend math step.

### GDAL / OGR

GDAL's coordinate transformation model also separates:

- dataset CRS metadata
- transformation setup
- transformation execution

It treats reprojection as an explicit operation, not as an implicit side effect of rendering.

That supports the same boundary we chose:

- assets own native CRS
- requests specify target/display CRS
- transformations are executed by infrastructure that understands CRS semantics

### pyproj

`pyproj` exposes practical controls such as:

- reusable transformers
- `always_xy`
- area of interest
- "best" versus alternative operations

That matches the operational direction of our implementation:

- normalize axis-order behavior at the backend boundary
- support transform policy explicitly
- keep diagnostics about the chosen operation

### QGIS

QGIS is especially relevant because its product model matches our workspace/display distinction:

- layers keep native CRS
- the project has a project CRS
- reprojection is done on the fly for display

That is very close to the design we implemented:

- assets keep native truth
- TraceBoost workspace chooses display CRS
- Ophiolite resolves display geometry on demand

The important lesson from QGIS is not that all systems need a "project CRS" setting. It is that project/display CRS is not the same thing as layer/asset CRS.

### OpenVDS

OpenVDS is relevant because it explicitly models survey coordinate metadata and transformations from annotation coordinates such as inline/crossline into XY or XYZ coordinate systems.

That is helpful for seismic data modeling, but OpenVDS is primarily a volumetric storage and metadata system, not the owner of TraceBoost workspace policy or user override workflow.

Our design is compatible with that lesson:

- keep survey-grid-to-map transforms as first-class metadata
- keep CRS metadata explicit
- still separate asset truth from project display policy

### OpendTect

OpendTect treats survey information and the inline/xline to XY transformation as first-class survey state.

That is one of the clearest seismic-specific precedents for our design.

The lesson from OpendTect is that seismic applications need both:

- grid coordinates
- world/map coordinates

and that the transformation between them is a core part of the survey model, not an afterthought.

### GeoTools

GeoTools exposes CRS decoding and multiple possible transforms between the same CRS pair.

That reinforces two of our choices:

- CRS should be modeled explicitly, not as free text hidden in UI settings
- transformation is a backend concern that may require policy and diagnostics, not just one hardcoded formula

### XTGeo

XTGeo's cube model keeps origin, increments, and rotation as core geometry fields.

That is closely aligned with our decision to model survey-map grid transforms and footprints directly instead of reducing everything to only polygons or only inline/xline ranges.

## Why this is more appropriate than the main alternatives

The alternatives we considered were not all equally bad. Some are actively useful in other systems. They are just worse fits here.

### Alternative 1: frontend-owned reprojection

Rejected because:

- it duplicates policy
- it hides diagnostics
- it fragments behavior across clients

### Alternative 2: project CRS silently becomes native CRS

Rejected because:

- it corrupts asset truth
- it makes mixed-native-CRS support dishonest

### Alternative 3: one spatial descriptor whose meaning changes between native and display

Rejected because:

- it makes debugging ambiguous
- it weakens auditability
- it invites accidental reuse of display-space geometry as if it were native truth

### Alternative 4: store only a footprint, not a grid transform

Rejected because:

- footprint alone is too weak for future overlays and map interactions
- a seismic survey is not just a polygon; it is a grid in map space

### Alternative 5: block all map behavior until CRS metadata is perfect

Rejected because:

- it prevents useful native-space workflows
- it confuses "unknown CRS" with "no geometry"

The current architecture is the middle path:

- permissive where geometry is real
- strict where CRS truth is unknown

That is the right shape for a practical seismic desktop runtime.

## Why PROJ is the right transformation backend

The chosen long-term transformation engine is PROJ, not custom affine-only math and not a browser projection library.

Why PROJ fits:

- it is the standard open-source CRS transformation engine
- it supports real coordinate operations, not just simple projection formulas
- it can select among multiple candidate operations
- it supports grid-based datum shifts
- it exposes enough metadata to support diagnostics and policy

That makes it appropriate for:

- survey footprint reprojection
- survey grid-transform reprojection
- future well surface-location reprojection

It also keeps the stack aligned with the same geospatial core used by GDAL, pyproj, QGIS, and many other systems.

## Why the current implementation scope is intentionally limited

The current implementation does not try to solve every geospatial problem at once.

Today it supports:

- canonical CRS binding on seismic assets
- user override of effective native CRS
- workspace display CRS
- survey-map resolution with native and display spatial geometry
- compact transform status and detailed diagnostics
- PROJ-backed display reprojection for supported EPSG-to-EPSG cases
- derived display-geometry caching

It does not yet claim:

- complete CRS discovery from SEG-Y alone
- full WKT or PROJJSON authoring workflows
- full well absolute-location support
- arbitrary local correction transforms
- complete export-grade transformation policy

That restraint is deliberate. It keeps the implementation aligned with the actual product need instead of overfitting the data model to hypothetical future requirements.

## Remaining gaps

Three important gaps remain.

### 1. CRS identity is still sometimes unknown

We can derive map geometry more often than we can name the CRS confidently.

That is not a bug in the architecture.
It is exactly why the architecture needs detected versus effective CRS and explicit override provenance.

### 2. Wells are not fully map-ready yet

Trajectory offsets exist, but authoritative absolute well placement still depends on:

- absolute surface location
- well CRS identity

The current architecture already leaves room for that without forcing fake certainty now.

### 3. The frontend still has unrelated typing cleanup

The geospatial path itself is wired through the desktop app, but some existing pipeline-editor typing work is still being cleaned up separately. That does not change the geospatial design conclusion.

## Final conclusion

For the current TraceBoost and Ophiolite stack, the implemented geospatial architecture is the right one.

Not because it is the most abstract design, and not because it copies any one external system exactly, but because it matches the actual responsibilities in this codebase:

- `Ophiolite` is the canonical owner of asset geospatial truth and coordinate operations
- `TraceBoost` is the owner of workspace display intent and user correction workflow
- `Ophiolite Charts` is the owner of rendering, not geodesy

The resulting model is strong where it needs to be strong:

- explicit native truth
- explicit display intent
- explicit reprojection
- explicit diagnostics
- explicit uncertainty

That is more appropriate than pushing CRS assumptions into the UI, more appropriate than collapsing native and display coordinates into one ambiguous object, and more appropriate than pretending that a workspace map setting can stand in for asset metadata.

In short:

the chosen architecture treats geospatial alignment as a domain and runtime problem first, and a chart problem second.

That is the right priority for a seismic application.

## References

- PROJ transformation usage: https://proj.org/en/stable/usage/transformation.html
- PROJ resource files and grids: https://proj.org/en/stable/resource_files.html
- GDAL CRS and coordinate transformation tutorial: https://gdal.org/en/latest/tutorials/osr_api_tut.html
- pyproj `Transformer` API: https://pyproj4.github.io/pyproj/stable/api/transformer.html
- QGIS training manual on on-the-fly reprojection: https://docs.qgis.org/3.22/pdf/en/QGIS-3.22-TrainingManual-en.pdf
- QGIS processing and CRS mismatch notes: https://docs.qgis.org/latest/en/docs/user_manual/processing/configuration.html
- GeoTools CRS user guide: https://docs.geotools.org/latest/userguide/library/referencing/crs.html
- OpendTect `SurveyInfo` class reference: https://doc.opendtect.org/6.6.0/doc/Programmer/Generated/html/classSurveyInfo.html
- XTGeo data model notes for cube geometry: https://xtgeo.readthedocs.io/en/stable/datamodels.html
- OpenVDS metadata specification: https://osdu.pages.opengroup.org/platform/domain-data-mgmt-services/seismic/open-vds/vds/specification/Metadata.html
- OpenVDS known survey coordinate metadata: https://osdu.pages.opengroup.org/platform/domain-data-mgmt-services/seismic/open-vds/cppdoc/class/classOpenVDS_1_1KnownMetadata.html
