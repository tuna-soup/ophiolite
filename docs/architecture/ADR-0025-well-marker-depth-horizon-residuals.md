# ADR-0025: Well-Marker Depth-Horizon Residuals

## Status

Accepted

## Context

ADR-0022 through ADR-0024 established canonical well and wellbore metadata, definitive
trajectories, and authored marker-set assets. That gives Ophiolite enough canonical context to
compute a common subsurface calibration product: the depth residual between a picked well marker and
an interpreted depth horizon at the well location.

OSDU models the inputs as separate wellbore and interpretation-related records. Ophiolite needs the
same practical scope without introducing a large new interpretation framework.

## Decision

Ophiolite adds a built-in project compute function:

- `well_markers:depth_horizon_residuals`

The function accepts a `TopSet` or `WellMarkerSet` source plus:

- a seismic survey asset id
- a depth-domain horizon id inside that survey

The result is persisted as a structured asset kind:

- `WellMarkerHorizonResidualSet`

### Residual definition

For each marker row, Ophiolite computes:

`residual = marker_depth_tvd - horizon_depth_tvd`

Positive residual means the marker is deeper than the sampled horizon.

### Sampling rule

Ophiolite resolves marker XY from the definitive wellbore trajectory and samples the horizon
vertically at that XY location. It does not intersect the horizon with the 3D well path.

This matches the common residual-map workflow where well control calibrates a depth horizon at the
well location rather than along a deviated trajectory crossing.

### Depth-reference rule

Phase one supports marker depths that can be normalized to TVD for a depth-domain horizon:

- `md` markers are interpolated through the definitive trajectory
- `tvd` markers are matched to the definitive trajectory to recover XY
- `tvdss` markers are retained but rejected for horizon residual computation unless an explicit datum
  relation is available

Ophiolite computes internally in meters and rejects incompatible datum mixes instead of silently
converting them.

### Result rows

`WellMarkerHorizonResidualSet` rows carry:

- marker identity and marker kind
- source depth and source depth reference
- resolved MD, TVD, and TVDSS when available
- sampled XY
- sampled horizon depth
- residual value
- sampled inline and xline ordinals
- status and note

This keeps the derived asset auditable and reusable by APIs and apps without recomputing the
operator immediately.

## Consequences

### Positive

- Ophiolite now captures a standard well-to-horizon calibration product close to the practical OSDU
  scope
- the operator is project-level and cross-asset, which fits the need to combine well markers,
  wellbore geometry, and seismic interpretation data
- the persisted residual asset gives downstream APIs a stable DTO surface
- strict datum handling avoids quiet depth errors

### Tradeoffs

- TVDSS-based markers need explicit datum support before they can participate in depth-horizon
  residuals
- residuals are produced per marker row only; gridded residual maps and corrected horizons remain a
  later phase
- the operator currently samples a structured depth horizon in the seismic store and does not yet
  support arbitrary surface asset families

## Non-goals

This ADR does not add:

- residual gridding or kriging
- corrected horizon materialization
- trajectory/horizon geometric intersection along the deviated borehole
- automatic vertical datum transforms
