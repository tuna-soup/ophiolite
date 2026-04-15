# ADR-0014: Seismic CRS Native, Effective, and Display Boundary

## Status

Accepted

## Decision

`ophiolite` will treat seismic coordinate-reference metadata as a canonical backend concern rather than an app-local convenience.

The durable boundary is:

- every seismic asset/store has a native coordinate-reference state owned by `ophiolite`
- native coordinate-reference state distinguishes:
  - detected native CRS
  - effective native CRS
  - provenance/source of the effective CRS
- `VolumeDescriptor.spatial` remains asset-native spatial geometry
- survey-map resolution accepts an optional requested display CRS
- survey-map resolution returns native spatial and display spatial separately
- `TraceBoost` owns only the workspace display-CRS preference
- `Ophiolite Charts` remains a rendering consumer of resolved geometry and does not infer, assign, or transform CRS values

Unknown CRS remains explicit. The system must not silently treat a workspace display CRS as the asset-native CRS.

## Why

The current seismic ingest path can derive stable projected XY geometry from `CDP_X/CDP_Y + SCALCO`, but it still cannot name the corresponding CRS reliably from SEG-Y alone.

That creates two separate truths:

- geometric truth: we can fit the survey map transform
- reference truth: we cannot yet always say whether those coordinates are `EPSG:23031`, `EPSG:32631`, or some local operator system

If this distinction is not modeled canonically in `ophiolite`, app layers will drift into inventing their own CRS assumptions, overrides, and reprojection rules.

## Consequences

- seismic stores and descriptors gain explicit CRS metadata beyond the current `SurveySpatialDescriptor`
- user overrides become canonical store/project metadata, not TraceBoost-only session state
- survey-map resolution becomes request-context aware because display CRS is no longer identical to native CRS by default
- mixed-native-CRS workspaces are supported in the model without pretending that overlay is safe until reprojection exists

## Exact Contract Changes

### 1. Shared seismic contract

Current problem:

- `SurveySpatialDescriptor` currently contains both `native_coordinate_reference` and `display_coordinate_reference`
- that shape overloads one object with two coordinate spaces

Phase-1 target:

```rust
pub enum CoordinateReferenceSource {
    Header,
    ImportManifest,
    UserOverride,
    Unknown,
}

pub struct CoordinateReferenceBinding {
    pub detected: Option<CoordinateReferenceDescriptor>,
    pub effective: Option<CoordinateReferenceDescriptor>,
    pub source: CoordinateReferenceSource,
    pub notes: Vec<String>,
}

pub struct SurveySpatialDescriptor {
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub grid_transform: Option<SurveyGridTransform>,
    pub footprint: Option<ProjectedPolygon2>,
    pub availability: SurveySpatialAvailability,
    pub notes: Vec<String>,
}

pub struct VolumeDescriptor {
    pub id: DatasetId,
    pub label: String,
    pub shape: [usize; 3],
    pub chunk_shape: [usize; 3],
    pub sample_interval_ms: f32,
    pub geometry: GeometryDescriptor,
    pub coordinate_reference_binding: Option<CoordinateReferenceBinding>,
    pub spatial: Option<SurveySpatialDescriptor>,
}
```

Rules:

- `VolumeDescriptor.spatial` is always native-space geometry for the effective native CRS
- `coordinate_reference_binding.detected` preserves what ingest or import discovered
- `coordinate_reference_binding.effective` is what downstream apps should use as the asset-native CRS
- `coordinate_reference_binding.source` indicates whether the effective CRS came from ingest/import or from a user override

### 2. Store/runtime metadata

The same canonical CRS binding must be persisted in:

- `VolumeMetadata`
- `StoreManifest`
- any manifest/metadata path that currently persists `spatial`

That makes CRS overrides durable and app-agnostic.

Phase-1 runtime behavior:

- SEG-Y ingest populates `detected` when it can name the CRS
- SEG-Y ingest still derives native XY geometry even when `detected` is unknown
- `effective` equals `detected` unless a user override exists
- user override updates only the effective binding; it does not erase the detected binding

### 3. Ophiolite project DTOs

Current DTOs in `src/project_contracts.rs`:

- `SurveyMapRequestDto`
- `ProjectSurveyMapRequestDto`
- `SurveyMapSpatialDescriptorDto`
- `ResolvedSurveyMapSurveyDto`

Phase-1 target:

```rust
pub struct SurveyMapRequestDto {
    pub schema_version: u32,
    pub survey_asset_ids: Vec<String>,
    pub wellbore_ids: Vec<String>,
    pub display_coordinate_reference_id: Option<String>,
}

pub struct ProjectSurveyMapRequestDto {
    pub schema_version: u32,
    pub survey_asset_ids: Vec<String>,
    pub wellbore_ids: Vec<String>,
    pub display_coordinate_reference_id: String,
}

pub enum CoordinateReferenceSourceDto {
    Header,
    ImportManifest,
    UserOverride,
    Unknown,
}

pub struct CoordinateReferenceBindingDto {
    pub detected: Option<CoordinateReferenceDto>,
    pub effective: Option<CoordinateReferenceDto>,
    pub source: CoordinateReferenceSourceDto,
    pub notes: Vec<String>,
}

pub struct SurveyMapSpatialDescriptorDto {
    pub coordinate_reference: Option<CoordinateReferenceDto>,
    pub grid_transform: Option<SurveyMapGridTransformDto>,
    pub footprint: Option<ProjectedPolygon2Dto>,
    pub availability: SurveyMapSpatialAvailabilityDto,
    pub notes: Vec<String>,
}

pub struct ResolvedSurveyMapSurveyDto {
    pub asset_id: String,
    pub logical_asset_id: String,
    pub name: String,
    pub index_grid: SurveyIndexGridDto,
    pub coordinate_reference_binding: Option<CoordinateReferenceBindingDto>,
    pub native_spatial: SurveyMapSpatialDescriptorDto,
    pub display_spatial: Option<SurveyMapSpatialDescriptorDto>,
    pub notes: Vec<String>,
}
```

Rules:

- `native_spatial` is always resolved from the effective native CRS
- `display_spatial` is:
  - `None` when no display CRS was requested
  - `Some(...)` when display CRS equals effective native CRS
  - `None` plus explanatory survey notes when a different display CRS was requested but transformation is unavailable

Phase-1 does not require a richer display-status enum yet. Survey-level `notes` are sufficient.

### 4. Well DTO scope in phase 1

`ResolvedSurveyMapWellDto` remains intentionally limited in phase 1.

No new well-map promises are made beyond:

- preserving well/trajectory coordinate-reference metadata when known
- explicitly reporting that trajectory offsets are relative and require an absolute surface origin before map placement is authoritative

Absolute well map placement is deferred until:

- well surface location becomes canonical
- well native CRS is explicit
- native-to-display transformation exists

## Exact Command/Additive API Work Needed

Phase-1 requires one new override-oriented backend operation in addition to the survey-map request change.

Recommended shared/runtime-facing shape:

```rust
pub struct SetDatasetNativeCoordinateReferenceRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub coordinate_reference_id: Option<String>,
    pub coordinate_reference_name: Option<String>,
}

pub struct SetDatasetNativeCoordinateReferenceResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}
```

Semantics:

- `coordinate_reference_id = None` clears the user override
- clearing an override restores `effective = detected`
- a successful response returns a refreshed dataset descriptor so app caches can update immediately

## Phase-1 Validation Rules

- accepted CRS identifiers should be canonical strings such as `EPSG:23031`
- malformed identifiers are rejected at write time
- semantic CRS-registry validation is deferred until reprojection infrastructure exists
- workspace display CRS must never be copied into `effective` automatically

## Phase-1 Non-Goals

Phase 1 does not include:

- native-to-display reprojection for different CRSs
- PROJ or other CRS-transformation infrastructure
- local survey correction transforms
- authoritative absolute well map placement
- basemap integration

## Implementation Order

1. Add shared `CoordinateReferenceBinding` and source enum to the seismic contracts.
2. Persist those fields in runtime/store metadata.
3. Update project survey-map DTOs to split native/display spatial.
4. Add the dataset native-CRS override command surface.
5. Let `TraceBoost` adopt workspace display CRS and override UX on top of those canonical changes.

## Follow-On Work

The next phase after this ADR lands should add:

- CRS registry-backed validation
- native-to-display reprojection in `ophiolite`
- well surface location and well CRS identity
- display-space overlay support across mixed-native-CRS surveys
