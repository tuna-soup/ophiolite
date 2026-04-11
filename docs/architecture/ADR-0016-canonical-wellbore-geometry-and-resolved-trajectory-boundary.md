# ADR-0016: Canonical Wellbore Geometry and Resolved Trajectory Boundary

## Status

Accepted

## Decision

`ophiolite` will treat wellbore placement and trajectory resolution as canonical backend concerns rather than app-local conventions.

The durable boundary is:

- `Well` remains an identity/catalog object
- `Wellbore` remains the owner of canonical absolute placement metadata
- trajectory imports remain separate typed wellbore-linked source assets
- raw trajectory imports and resolved trajectory geometry become distinct contract families
- authoritative map/section placement depends on canonical wellbore geometry, not on frontend guesses from relative offsets alone

This means a trajectory does relate to a wellbore, but it is not embedded on the well or wellbore record as an opaque blob. The wellbore owns the anchor and reference metadata that make trajectory geometry authoritative. The trajectory asset owns the imported station data and provenance.

## Why

The current project layer is structurally close, but not canonical enough for real well placement and section projection:

- `WellRecord` and `WellboreRecord` are identity containers today
- `TrajectoryRow` is too thin for multiple import schemas and authoritative geometry
- `ResolvedSurveyMapWellDto` still reports `surface_location: None` and treats offsets as relative-only
- dataset-only survey-map resolution returns no wells at all

That is adequate for early cataloging and simple well-panel display, but it is not adequate for:

- absolute survey-map placement
- seismic section overlays
- sidetracks and tie-on anchors
- depth and TWT projection along deviated well paths
- auditable raw-versus-derived trajectory handling

If `ophiolite` does not own this boundary, app layers will drift into inventing their own anchor, CRS, azimuth, and minimum-curvature rules.

## Consequences

- `Wellbore` gains canonical geometry metadata beyond identity and naming
- trajectory import contracts expand from one CSV shape to multiple supported schema families
- trajectory resolution becomes a first-class backend step with explicit validation and derivation reporting
- survey-map and section workflows can rely on one authoritative resolved well geometry path
- frontend packages remain rendering and workflow consumers rather than geometry owners

## Exact Boundary

### 1. Canonical wellbore geometry metadata

`Wellbore` must gain a canonical geometry object that is authoritative for absolute placement.

Recommended shared shape:

```rust
pub enum WellboreAnchorKind {
    Surface,
    ParentTieOn,
}

pub enum WellAzimuthReferenceKind {
    TrueNorth,
    GridNorth,
    MagneticNorth,
    Unknown,
}

pub struct WellboreAnchorReference {
    pub kind: WellboreAnchorKind,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub location: ProjectedPoint2,
    pub parent_wellbore_id: Option<String>,
    pub parent_measured_depth_m: Option<f64>,
    pub notes: Vec<String>,
}

pub struct WellboreGeometry {
    pub anchor: Option<WellboreAnchorReference>,
    pub vertical_datum: Option<String>,
    pub depth_unit: Option<String>,
    pub azimuth_reference: WellAzimuthReferenceKind,
    pub notes: Vec<String>,
}
```

Rules:

- `WellRecord` stays focused on well identity
- `Wellbore` owns absolute placement and directional-reference metadata
- a sidetrack-ready model requires both `surface` and `parent_tie_on` anchor semantics, even if initial runtime support starts with `surface`
- the authoritative CRS belongs on the wellbore geometry boundary, not only on the trajectory asset

### 2. Raw trajectory import versus resolved trajectory geometry

Raw trajectory ingest and resolved geometry must be modeled separately.

Recommended raw import family:

```rust
pub enum TrajectoryInputSchemaKind {
    MdIncAzi,
    MdTvdIncAzi,
    MdTvdssIncAzi,
    MdOffsetTvd,
    MdOffsetTvdss,
}

pub struct RawTrajectoryImport {
    pub asset_id: String,
    pub wellbore_id: String,
    pub source_path: String,
    pub schema_kind: TrajectoryInputSchemaKind,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub azimuth_reference: Option<WellAzimuthReferenceKind>,
    pub rows: Vec<RawTrajectoryRow>,
    pub notes: Vec<String>,
}
```

Recommended resolved family:

```rust
pub enum TrajectoryValueOrigin {
    Imported,
    Derived,
}

pub struct ResolvedTrajectoryStation {
    pub measured_depth_m: f64,
    pub true_vertical_depth_m: Option<f64>,
    pub true_vertical_depth_subsea_m: Option<f64>,
    pub northing_offset_m: Option<f64>,
    pub easting_offset_m: Option<f64>,
    pub absolute_xy: Option<ProjectedPoint2>,
    pub inclination_deg: Option<f64>,
    pub azimuth_deg: Option<f64>,
    pub true_vertical_depth_origin: Option<TrajectoryValueOrigin>,
    pub true_vertical_depth_subsea_origin: Option<TrajectoryValueOrigin>,
    pub northing_offset_origin: Option<TrajectoryValueOrigin>,
    pub easting_offset_origin: Option<TrajectoryValueOrigin>,
    pub inclination_origin: Option<TrajectoryValueOrigin>,
    pub azimuth_origin: Option<TrajectoryValueOrigin>,
}

pub struct ResolvedTrajectoryGeometry {
    pub id: String,
    pub wellbore_id: String,
    pub source_asset_ids: Vec<String>,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub anchor_fingerprint: Option<String>,
    pub stations: Vec<ResolvedTrajectoryStation>,
    pub notes: Vec<String>,
}
```

Rules:

- raw imports preserve what the user supplied and how it was interpreted
- resolved geometry is the canonical downstream shape for map display, section projection, time-depth compilation, and future export
- resolved stations always use monotonic MD
- unsupported partial input combinations such as `md+tvd` without direction or offsets are rejected rather than guessed

### 3. Supported raw schema families

Phase 1 trajectory ingest should support multiple explicit schema families rather than a single CSV header contract:

- `md + inc + azi`
- `md + tvd + inc + azi`
- `md + tvdss + inc + azi`
- `md + northing/easting + tvd`
- `md + northing/easting + tvdss`

The existing `TrajectoryRow` is a useful seed, but it should be treated as an early import-row shape rather than the final resolved geometry model.

### 4. Resolution rules

Trajectory resolution is backend-owned and must apply explicit, auditable rules:

- minimum curvature is the default resolver whenever `inc/azi` inputs are involved
- zero-dogleg handling must be numerically stable
- MD must be strictly monotonic
- angle and unit validation happens at import/resolution time, not later in display code
- resolved geometry preserves both relative offsets and absolute XY when a canonical anchor exists
- `tvdss` is a typed canonical field, not a note or free-text annotation

The expected mathematical reference is the standard minimum-curvature formulation used by mature wellpath libraries such as `wellpathpy` and `welleng`.

### 5. Survey-map implications

Once canonical wellbore geometry exists, `ResolvedSurveyMapWellDto` can evolve from a relative-only preview shape into an authoritative map placement DTO.

Phase 1 after this ADR:

- well survey-map resolution should rely on canonical wellbore anchor plus resolved trajectory geometry
- frontend code should no longer reconstruct authoritative placement from relative offsets alone
- dataset-only survey-map paths remain intentionally limited; project-aware well workflows are the target path

## Non-Goals

This ADR does not by itself introduce:

- well-local time/depth source assets
- authored well time/depth models
- section overlay DTOs
- CRS reprojection across different native/display systems
- full sidetrack inheritance runtime logic

Those are covered by follow-on ADRs.

## Implementation Order

1. Add canonical wellbore geometry metadata to the project/catalog layer.
2. Split raw trajectory import from resolved trajectory geometry.
3. Expand trajectory import to explicit schema-family handling.
4. Add minimum-curvature-based trajectory resolution and derivation diagnostics.
5. Update project-aware map/section queries to consume resolved geometry instead of raw rows directly.

## Follow-On Work

The next architecture records after this one should define:

- well time/depth source assets and authored models
- compiled well time/depth runtime outputs
- project-aware well-on-section overlay DTOs and backend projection rules
