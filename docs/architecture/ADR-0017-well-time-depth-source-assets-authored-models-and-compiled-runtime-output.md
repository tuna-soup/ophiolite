# ADR-0017: Well Time-Depth Source Assets, Authored Models, and Compiled Runtime Output

## Status

Accepted

## Decision

`ophiolite` will model well-local time/depth conversion with three distinct layers:

- source assets
- authored models
- compiled runtime outputs

The canonical separation is:

- checkshot/VSP observations and manual time-depth picks are source assets
- sonic and Vp remain log assets, with later derived candidates built from those logs
- `WellTimeDepthAuthoredModel1D` becomes the editable per-well model that binds sources and assumptions
- existing `WellTimeDepthModel1D` remains the compiled/runtime 1D output used by downstream projection/query paths

The active well model for a wellbore is the authored model. The compiled runtime model is generated from it and invalidated when its dependencies change.

## Why

The current contracts already contain useful runtime-oriented pieces such as `WellTimeDepthModel1D`, `VelocityControlProfileSet`, `DepthReferenceKind`, and `TravelTimeReference`. But they are not sufficient as the only durable object for the workflow this platform needs.

The user workflow requires:

- multiple per-well source kinds
- explicit selection of the active source/model
- visible and editable gap-fill assumptions
- auditable source precedence
- compilation against a canonical resolved trajectory
- a stable runtime model that section and map workflows can trust

If these concerns are collapsed into one generic velocity object, the platform loses provenance, editability, and validation clarity.

## Consequences

- new well-local source asset families are introduced
- a new authored per-well model contract is introduced
- `WellTimeDepthModel1D` stays simple and runtime-oriented
- source conditioning and authored assumptions become explicit and inspectable
- frontend code selects and edits authored models rather than bypassing them with ad hoc constants

## Exact Boundary

### 1. Source asset families

Phase 1 source assets:

- `CheckshotVspObservationSet1D`
- `ManualTimeDepthPickSet1D`

Later source assets and derivatives:

- sonic logs remain log assets
- Vp logs remain log assets
- sonic-derived and Vp-derived candidates can later be persisted as accepted derived source assets rather than mutating the original log asset

Recommended shared source sample shape:

```rust
pub struct WellTimeDepthObservationSample {
    pub depth_m: f64,
    pub time_ms: f64,
    pub quality: Option<f32>,
    pub station_id: Option<String>,
    pub note: Option<String>,
}
```

Rules:

- samples are monotonic in both depth and time
- samples use depth/time semantics only, not direct velocity semantics
- phase 1 rejects direct velocity samples in these source asset families
- checkshot/VSP and manual picks share the same core sample meaning, but preserve distinct provenance and source kind

### 2. Authored per-well model

`WellTimeDepthAuthoredModel1D` is the editable, wellbore-bound, workflow-owned model.

Recommended shape:

```rust
pub struct WellTimeDepthSourceBinding {
    pub source_kind: TimeDepthTransformSourceKind,
    pub asset_id: String,
    pub enabled: bool,
    pub priority: u32,
    pub valid_from_depth_m: Option<f64>,
    pub valid_to_depth_m: Option<f64>,
    pub notes: Vec<String>,
}

pub enum WellTimeDepthAssumptionKind {
    ConstantVelocity,
}

pub struct WellTimeDepthAssumptionInterval {
    pub from_depth_m: Option<f64>,
    pub to_depth_m: Option<f64>,
    pub kind: WellTimeDepthAssumptionKind,
    pub velocity_m_per_s: Option<f64>,
    pub overwrite_existing_source_coverage: bool,
    pub notes: Vec<String>,
}

pub struct WellTimeDepthAuthoredModel1D {
    pub id: String,
    pub name: String,
    pub wellbore_id: String,
    pub resolved_trajectory_fingerprint: String,
    pub depth_reference: DepthReferenceKind,
    pub travel_time_reference: TravelTimeReference,
    pub source_bindings: Vec<WellTimeDepthSourceBinding>,
    pub assumption_intervals: Vec<WellTimeDepthAssumptionInterval>,
    pub sampling_step_m: Option<f64>,
    pub notes: Vec<String>,
}
```

Rules:

- one authored model binds to exactly one wellbore
- one authored model compiles against exactly one resolved trajectory fingerprint
- one authored model uses exactly one `DepthReferenceKind`
- one authored model uses exactly one `TravelTimeReference`
- assumptions are expressed on a vertical depth axis, never on MD
- overlap resolution in phase 1 uses explicit priority order rather than weighted blending
- assumptions fill uncovered zones by default and only overwrite covered zones when explicitly allowed

### 3. Compiled runtime output

`WellTimeDepthModel1D` remains the compiled/runtime output.

Rules:

- section overlays and other runtime queries depend on the compiled model, not directly on the authored model
- the compiled model stays a simple sampled curve
- compiled output must carry enough lineage to identify the authored model and resolved trajectory it came from

Recommended additions:

```rust
pub struct CompiledWellTimeDepthLineage {
    pub authored_model_id: String,
    pub resolved_trajectory_fingerprint: String,
    pub source_asset_ids: Vec<String>,
}
```

This may live inside `WellTimeDepthModel1D` or next to it in build metadata, but the lineage boundary must be durable.

### 4. Compilation diagnostics

Compilation must emit diagnostics, not just a final curve.

Recommended diagnostics include:

- coverage gaps
- extrapolated intervals
- assumption-filled intervals
- overlapping-source conflicts
- trajectory/model datum mismatches
- disabled or missing sources

The compiled curve is the runtime product; diagnostics remain the explanation of how it was produced.

### 5. Project-level activation

The active authored model selection is durable project metadata, not transient UI-only state.

Rules:

- active model selection is stored per wellbore
- changing canonical wellbore geometry, resolved trajectory, relevant source assets, or authored-model settings invalidates the compiled output
- apps may later add workspace/session overrides, but project persistence is the canonical base

## Conditioning and Conversion Rules

Phase 1 keeps the conditioning rules explicit:

- checkshot/VSP and manual pick assets are imported directly as depth-time observations
- sonic and Vp inputs remain logs until converted through an explicit conditioning/build step
- sonic `DT -> Vp` conversion and resampling are build/analysis operations, not mutations of the original log asset
- the first compiled output should use a regular depth sampling grid by default
- depth-axis support is limited to `TVD` or `TVDSS`
- the active well model must never be driven canonically by MD

## Non-Goals

This ADR does not by itself define:

- survey-wide layered velocity models
- weighted/blended multi-source inversion
- section display DTO shapes
- frontend workflow details

Those stay in adjacent architecture boundaries.

## Implementation Order

1. Add phase 1 source asset families for `checkshot_vsp` and `manual_time_depth_picks`.
2. Add `WellTimeDepthAuthoredModel1D`.
3. Add compile/build APIs that produce `WellTimeDepthModel1D` plus diagnostics.
4. Persist active authored-model selection per wellbore.
5. Add log-derived candidate workflows for sonic and Vp inputs.

## Follow-On Work

The next phase after this ADR should add:

- source-conditioning APIs for sonic/Vp conversion
- richer assumption kinds beyond constant velocity
- UI workflows for selecting sources and editing intervals
- section overlay queries that consume compiled well models
