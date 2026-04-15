# ADR-0018: Project-Aware Well-On-Section Overlay DTOs and Backend Projection Rules

## Status

Accepted

## Decision

Well-on-section overlays for seismic views will be modeled as project-aware display DTOs produced by backend-owned projection logic.

The durable boundary is:

- `ophiolite` owns section-plane math, survey-grid math, trajectory densification, tolerance handling, and time/depth projection
- well-on-section overlays are display DTOs rather than canonical well or trajectory storage types
- `Ophiolite Charts` renders the overlay DTOs and does not infer CRS, section planes, or time-depth models
- `TraceBoost` owns workflow and activation, not the underlying geometry math

Depth and time overlays are separate display-domain views over the same wellbore geometry. Time overlays require an active compiled well time/depth model. Depth overlays require only canonical resolved trajectory geometry.

## Why

This workflow requires several domain-sensitive steps that should not be pushed into app code:

- survey inline/xline grid inversion
- physical distance to the section plane
- off-line tolerance handling in meters
- trajectory densification for deviated wells
- mapping from well stations to depth or TWT
- partial coverage handling when the active well model does not cover the full trajectory

If the frontend or chart layer owns those steps, different products will drift into different math and tolerance rules.

## Consequences

- section overlays become a first-class DTO family
- project/workspace context is required for real well section workflows
- `SeismicSectionChart` can stay rendering-oriented
- time overlays will not silently fall back to constant velocity when a compiled well model is missing
- partial and degraded coverage can be reported explicitly rather than hidden inside ad hoc rendering logic

## Exact Boundary

### 1. Project-aware request DTO

Recommended request shape:

```rust
pub enum SectionWellOverlayDomainDto {
    Depth,
    Time,
}

pub struct SectionWellOverlayRequestDto {
    pub schema_version: u32,
    pub project_root: String,
    pub survey_asset_id: String,
    pub wellbore_ids: Vec<String>,
    pub axis: SectionAxis,
    pub index: i32,
    pub tolerance_m: Option<f64>,
    pub display_domain: SectionWellOverlayDomainDto,
    pub active_well_model_ids: Vec<String>,
}
```

Rules:

- this is project-aware rather than dataset-only
- `tolerance_m` is measured in physical map distance to the section plane
- when omitted, the backend may default to a survey-aware half-bin style tolerance
- the active model ids are relevant only for time-domain overlays

### 2. Overlay DTO shape

Recommended response family:

```rust
pub struct SectionWellOverlaySampleDto {
    pub trace_index: usize,
    pub trace_coordinate: f64,
    pub sample_index: Option<usize>,
    pub sample_value: Option<f64>,
    pub x: f64,
    pub y: f64,
    pub measured_depth_m: f64,
    pub true_vertical_depth_m: Option<f64>,
    pub twt_ms: Option<f64>,
}

pub struct SectionWellOverlaySegmentDto {
    pub samples: Vec<SectionWellOverlaySampleDto>,
    pub notes: Vec<String>,
}

pub struct ResolvedSectionWellOverlayDto {
    pub well_id: String,
    pub wellbore_id: String,
    pub name: String,
    pub display_domain: SectionWellOverlayDomainDto,
    pub segments: Vec<SectionWellOverlaySegmentDto>,
    pub diagnostics: Vec<String>,
}

pub struct ResolveSectionWellOverlaysResponse {
    pub schema_version: u32,
    pub overlays: Vec<ResolvedSectionWellOverlayDto>,
}
```

Rules:

- overlay samples carry both section-space and map-space positions
- overlays are ordered polyline segments, not point clouds
- explicit segment breaks represent missing coverage, tolerance failures, or invalid spans
- both trace/sample coordinates and physical XY are preserved for debugging and future interactions

### 3. Backend projection rules

Projection is backend-owned and must use explicit rules:

- use the survey grid transform as the canonical bridge from XY to inline/xline space
- compute physical distance to the requested section plane in meters
- retain stations within a configurable tolerance ribbon, not only exact on-line hits
- compute fractional trace position before final chart sampling
- densify the resolved trajectory before projection so sparse surveys do not miss the section or render as kinked segments
- densification should be controlled by both MD-step and geometric-error style limits, not MD step alone

### 4. Domain-specific projection rules

Depth-domain overlays:

- require resolved trajectory geometry
- do not require a compiled time model

Time-domain overlays:

- require an active compiled `WellTimeDepthModel1D`
- convert stations using the compiled model on `TVD` or `TVDSS`
- never fall back silently to a constant-velocity shortcut

Partial coverage behavior:

- missing compiled-model coverage breaks the overlay into segments
- missing coverage should degrade gracefully with diagnostics
- fundamentally invalid anchor/CRS/trajectory states fail hard rather than inventing geometry

### 5. Renderer boundary

`Ophiolite Charts` and section charts remain rendering consumers.

Rules:

- no section-plane math in `Ophiolite Charts`
- no CRS inference in `Ophiolite Charts`
- no time-depth compilation in `Ophiolite Charts`
- chart packages receive preprojected overlays and render them

This stays consistent with the existing source/authored/runtime/display taxonomy.

## Caching Guidance

Overlay caching should be keyed by the geometry and modeling inputs that materially affect the result:

- survey geometry fingerprint
- section axis/index
- wellbore id
- resolved trajectory fingerprint
- compiled well-model fingerprint when applicable
- tolerance
- display domain

## Non-Goals

This ADR does not by itself define:

- the canonical wellbore geometry model
- source assets or authored well time/depth models
- chart-specific styling defaults
- mixed-CRS reprojection infrastructure beyond existing survey CRS boundaries

Those are covered by adjacent ADRs.

## Implementation Order

1. Add project-aware section well-overlay request/response DTOs.
2. Implement backend projection and densification on resolved trajectory geometry.
3. Add depth-domain overlays first.
4. Add time-domain overlays on top of compiled well models.
5. Extend `Ophiolite Charts` section charts to render the new DTO family.

## Follow-On Work

The next phase after this ADR should add:

- overlay labels and interaction metadata
- richer diagnostics enums instead of free-text notes
- optional station markers and pick annotations
- caching and incremental refresh paths in app/workspace layers
