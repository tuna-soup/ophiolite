use crate::{
    AssetId, AssetKind, AssetRecord, DrillingObservationRow, LasError, OphioliteProject,
    PressureObservationRow, Result, TopRow, TrajectoryRow, WellMarkerRow,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static STRUCTURED_EDIT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructuredAssetEditSessionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredAssetEditSessionSummary {
    pub session_id: StructuredAssetEditSessionId,
    pub project_root: String,
    pub asset_id: AssetId,
    pub asset_kind: AssetKind,
    pub row_count: usize,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredAssetSessionRequest {
    pub session_id: StructuredAssetEditSessionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenStructuredAssetEditSessionRequest {
    pub project_root: String,
    pub asset_id: AssetId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredAssetSaveResult {
    pub session: StructuredAssetEditSessionSummary,
    pub asset: AssetRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionalFieldPatch<T> {
    #[serde(default)]
    pub set: Option<T>,
    #[serde(default)]
    pub clear: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TrajectoryRowPatch {
    #[serde(default)]
    pub measured_depth: Option<f64>,
    #[serde(default)]
    pub true_vertical_depth: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub true_vertical_depth_subsea: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub azimuth_deg: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub inclination_deg: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub northing_offset: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub easting_offset: Option<OptionalFieldPatch<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TopRowPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub top_depth: Option<f64>,
    #[serde(default)]
    pub base_depth: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub source: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub source_depth_reference: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub depth_domain: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub depth_datum: Option<OptionalFieldPatch<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WellMarkerRowPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub marker_kind: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub top_depth: Option<f64>,
    #[serde(default)]
    pub base_depth: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub source: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub source_depth_reference: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub depth_domain: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub depth_datum: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub note: Option<OptionalFieldPatch<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PressureObservationRowPatch {
    #[serde(default)]
    pub measured_depth: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub pressure: Option<f64>,
    #[serde(default)]
    pub phase: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub test_kind: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub timestamp: Option<OptionalFieldPatch<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DrillingObservationRowPatch {
    #[serde(default)]
    pub measured_depth: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub event_kind: Option<String>,
    #[serde(default)]
    pub value: Option<OptionalFieldPatch<f64>>,
    #[serde(default)]
    pub unit: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub timestamp: Option<OptionalFieldPatch<String>>,
    #[serde(default)]
    pub comment: Option<OptionalFieldPatch<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrajectoryEditRequest {
    AddRow {
        row: TrajectoryRow,
        at_index: Option<usize>,
    },
    UpdateRow {
        row_index: usize,
        patch: TrajectoryRowPatch,
    },
    DeleteRow {
        row_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TopSetEditRequest {
    AddRow {
        row: TopRow,
        at_index: Option<usize>,
    },
    UpdateRow {
        row_index: usize,
        patch: TopRowPatch,
    },
    DeleteRow {
        row_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WellMarkerSetEditRequest {
    AddRow {
        row: WellMarkerRow,
        at_index: Option<usize>,
    },
    UpdateRow {
        row_index: usize,
        patch: WellMarkerRowPatch,
    },
    DeleteRow {
        row_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PressureObservationEditRequest {
    AddRow {
        row: PressureObservationRow,
        at_index: Option<usize>,
    },
    UpdateRow {
        row_index: usize,
        patch: PressureObservationRowPatch,
    },
    DeleteRow {
        row_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DrillingObservationEditRequest {
    AddRow {
        row: DrillingObservationRow,
        at_index: Option<usize>,
    },
    UpdateRow {
        row_index: usize,
        patch: DrillingObservationRowPatch,
    },
    DeleteRow {
        row_index: usize,
    },
}

#[derive(Debug, Clone)]
enum StructuredRows {
    Trajectory(Vec<TrajectoryRow>),
    TopSet(Vec<TopRow>),
    WellMarkerSet(Vec<WellMarkerRow>),
    Pressure(Vec<PressureObservationRow>),
    Drilling(Vec<DrillingObservationRow>),
}

#[derive(Debug, Clone)]
struct StructuredAssetEditSession {
    session_id: StructuredAssetEditSessionId,
    project_root: String,
    asset_id: AssetId,
    asset_kind: AssetKind,
    dirty: bool,
    rows: StructuredRows,
}

#[derive(Default)]
pub struct StructuredAssetEditSessionStore {
    sessions: BTreeMap<StructuredAssetEditSessionId, StructuredAssetEditSession>,
}

impl StructuredAssetEditSessionStore {
    pub fn open_session(
        &mut self,
        request: &OpenStructuredAssetEditSessionRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        let project = OphioliteProject::open(&request.project_root)?;
        let asset = project.asset_record(&request.asset_id)?;
        let rows = match asset.asset_kind {
            AssetKind::Trajectory => {
                StructuredRows::Trajectory(project.read_trajectory_rows(&request.asset_id, None)?)
            }
            AssetKind::TopSet => StructuredRows::TopSet(project.read_tops(&request.asset_id)?),
            AssetKind::WellMarkerSet => {
                StructuredRows::WellMarkerSet(project.read_well_marker_rows(&request.asset_id)?)
            }
            AssetKind::WellMarkerHorizonResidualSet => {
                return Err(LasError::Validation(
                    "structured edit sessions do not support well marker horizon residual assets"
                        .to_string(),
                ))
            }
            AssetKind::PressureObservation => StructuredRows::Pressure(
                project.read_pressure_observations(&request.asset_id, None)?,
            ),
            AssetKind::DrillingObservation => StructuredRows::Drilling(
                project.read_drilling_observations(&request.asset_id, None)?,
            ),
            AssetKind::CheckshotVspObservationSet
            | AssetKind::ManualTimeDepthPickSet
            | AssetKind::WellTieObservationSet
            | AssetKind::WellTimeDepthAuthoredModel
            | AssetKind::WellTimeDepthModel
            | AssetKind::RawSourceBundle => {
                return Err(LasError::Validation(
                    "structured edit sessions do not support well time-depth model assets"
                        .to_string(),
                ))
            }
            AssetKind::Log => {
                return Err(LasError::Validation(
                    "structured edit sessions only support trajectory, tops, well markers, pressure, and drilling assets".to_string(),
                ))
            }
            AssetKind::SeismicTraceData => {
                return Err(LasError::Validation(
                    "structured edit sessions do not support seismic assets".to_string(),
                ))
            }
        };

        let session = StructuredAssetEditSession {
            session_id: next_session_id(),
            project_root: request.project_root.clone(),
            asset_id: request.asset_id.clone(),
            asset_kind: asset.asset_kind,
            dirty: false,
            rows,
        };
        let summary = session.summary();
        self.sessions.insert(session.session_id.clone(), session);
        Ok(summary)
    }

    pub fn session_summary(
        &self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        Ok(self.session(request)?.summary())
    }

    pub fn close_session(&mut self, request: &StructuredAssetSessionRequest) -> Result<bool> {
        Ok(self.sessions.remove(&request.session_id).is_some())
    }

    pub fn trajectory_rows(
        &self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<Vec<TrajectoryRow>> {
        match &self.session(request)?.rows {
            StructuredRows::Trajectory(rows) => Ok(rows.clone()),
            _ => Err(kind_mismatch_error(
                self.session(request)?.asset_kind.clone(),
                AssetKind::Trajectory,
            )),
        }
    }

    pub fn tops_rows(&self, request: &StructuredAssetSessionRequest) -> Result<Vec<TopRow>> {
        match &self.session(request)?.rows {
            StructuredRows::TopSet(rows) => Ok(rows.clone()),
            _ => Err(kind_mismatch_error(
                self.session(request)?.asset_kind.clone(),
                AssetKind::TopSet,
            )),
        }
    }

    pub fn well_marker_rows(
        &self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<Vec<WellMarkerRow>> {
        match &self.session(request)?.rows {
            StructuredRows::WellMarkerSet(rows) => Ok(rows.clone()),
            _ => Err(kind_mismatch_error(
                self.session(request)?.asset_kind.clone(),
                AssetKind::WellMarkerSet,
            )),
        }
    }

    pub fn pressure_rows(
        &self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<Vec<PressureObservationRow>> {
        match &self.session(request)?.rows {
            StructuredRows::Pressure(rows) => Ok(rows.clone()),
            _ => Err(kind_mismatch_error(
                self.session(request)?.asset_kind.clone(),
                AssetKind::PressureObservation,
            )),
        }
    }

    pub fn drilling_rows(
        &self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<Vec<DrillingObservationRow>> {
        match &self.session(request)?.rows {
            StructuredRows::Drilling(rows) => Ok(rows.clone()),
            _ => Err(kind_mismatch_error(
                self.session(request)?.asset_kind.clone(),
                AssetKind::DrillingObservation,
            )),
        }
    }

    pub fn apply_trajectory_edit(
        &mut self,
        request: &StructuredAssetSessionRequest,
        edit: &TrajectoryEditRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        let session = self.session_mut(request)?;
        if let StructuredRows::Trajectory(rows) = &mut session.rows {
            apply_trajectory_edit(rows, edit)?;
            session.dirty = true;
            return Ok(session.summary());
        }
        Err(kind_mismatch_error(
            session.asset_kind.clone(),
            AssetKind::Trajectory,
        ))
    }

    pub fn apply_tops_edit(
        &mut self,
        request: &StructuredAssetSessionRequest,
        edit: &TopSetEditRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        let session = self.session_mut(request)?;
        if let StructuredRows::TopSet(rows) = &mut session.rows {
            apply_tops_edit(rows, edit)?;
            session.dirty = true;
            return Ok(session.summary());
        }
        Err(kind_mismatch_error(
            session.asset_kind.clone(),
            AssetKind::TopSet,
        ))
    }

    pub fn apply_well_marker_edit(
        &mut self,
        request: &StructuredAssetSessionRequest,
        edit: &WellMarkerSetEditRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        let session = self.session_mut(request)?;
        if let StructuredRows::WellMarkerSet(rows) = &mut session.rows {
            apply_well_marker_edit(rows, edit)?;
            session.dirty = true;
            return Ok(session.summary());
        }
        Err(kind_mismatch_error(
            session.asset_kind.clone(),
            AssetKind::WellMarkerSet,
        ))
    }

    pub fn apply_pressure_edit(
        &mut self,
        request: &StructuredAssetSessionRequest,
        edit: &PressureObservationEditRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        let session = self.session_mut(request)?;
        if let StructuredRows::Pressure(rows) = &mut session.rows {
            apply_pressure_edit(rows, edit)?;
            session.dirty = true;
            return Ok(session.summary());
        }
        Err(kind_mismatch_error(
            session.asset_kind.clone(),
            AssetKind::PressureObservation,
        ))
    }

    pub fn apply_drilling_edit(
        &mut self,
        request: &StructuredAssetSessionRequest,
        edit: &DrillingObservationEditRequest,
    ) -> Result<StructuredAssetEditSessionSummary> {
        let session = self.session_mut(request)?;
        if let StructuredRows::Drilling(rows) = &mut session.rows {
            apply_drilling_edit(rows, edit)?;
            session.dirty = true;
            return Ok(session.summary());
        }
        Err(kind_mismatch_error(
            session.asset_kind.clone(),
            AssetKind::DrillingObservation,
        ))
    }

    pub fn save_session(
        &mut self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<StructuredAssetSaveResult> {
        let snapshot = self.session(request)?.clone();
        validate_rows(&snapshot.rows)?;

        let mut project = OphioliteProject::open(&snapshot.project_root)?;
        let asset = match &snapshot.rows {
            StructuredRows::Trajectory(rows) => {
                project.overwrite_trajectory_asset(&snapshot.asset_id, rows)?
            }
            StructuredRows::TopSet(rows) => {
                project.overwrite_tops_asset(&snapshot.asset_id, rows)?
            }
            StructuredRows::WellMarkerSet(rows) => {
                project.overwrite_well_marker_set_asset(&snapshot.asset_id, rows)?
            }
            StructuredRows::Pressure(rows) => {
                project.overwrite_pressure_asset(&snapshot.asset_id, rows)?
            }
            StructuredRows::Drilling(rows) => {
                project.overwrite_drilling_asset(&snapshot.asset_id, rows)?
            }
        };

        let session = self.session_mut(request)?;
        session.dirty = false;
        Ok(StructuredAssetSaveResult {
            session: session.summary(),
            asset,
        })
    }

    fn session(
        &self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<&StructuredAssetEditSession> {
        self.sessions.get(&request.session_id).ok_or_else(|| {
            LasError::Validation(format!(
                "structured asset edit session '{}' was not found",
                request.session_id.0
            ))
        })
    }

    fn session_mut(
        &mut self,
        request: &StructuredAssetSessionRequest,
    ) -> Result<&mut StructuredAssetEditSession> {
        self.sessions.get_mut(&request.session_id).ok_or_else(|| {
            LasError::Validation(format!(
                "structured asset edit session '{}' was not found",
                request.session_id.0
            ))
        })
    }
}

impl StructuredAssetEditSession {
    fn summary(&self) -> StructuredAssetEditSessionSummary {
        StructuredAssetEditSessionSummary {
            session_id: self.session_id.clone(),
            project_root: self.project_root.clone(),
            asset_id: self.asset_id.clone(),
            asset_kind: self.asset_kind.clone(),
            row_count: self.rows.len(),
            dirty: self.dirty,
        }
    }
}

impl StructuredRows {
    fn len(&self) -> usize {
        match self {
            Self::Trajectory(rows) => rows.len(),
            Self::TopSet(rows) => rows.len(),
            Self::WellMarkerSet(rows) => rows.len(),
            Self::Pressure(rows) => rows.len(),
            Self::Drilling(rows) => rows.len(),
        }
    }
}

fn apply_trajectory_edit(
    rows: &mut Vec<TrajectoryRow>,
    edit: &TrajectoryEditRequest,
) -> Result<()> {
    match edit {
        TrajectoryEditRequest::AddRow { row, at_index } => insert_row(rows, row.clone(), *at_index),
        TrajectoryEditRequest::UpdateRow { row_index, patch } => {
            let row_count = rows.len();
            let row = rows
                .get_mut(*row_index)
                .ok_or_else(|| row_index_error(*row_index, row_count))?;
            if let Some(value) = patch.measured_depth {
                row.measured_depth = value;
            }
            apply_optional_field_patch(
                &mut row.true_vertical_depth,
                patch.true_vertical_depth.as_ref(),
            )?;
            apply_optional_field_patch(
                &mut row.true_vertical_depth_subsea,
                patch.true_vertical_depth_subsea.as_ref(),
            )?;
            apply_optional_field_patch(&mut row.azimuth_deg, patch.azimuth_deg.as_ref())?;
            apply_optional_field_patch(&mut row.inclination_deg, patch.inclination_deg.as_ref())?;
            apply_optional_field_patch(&mut row.northing_offset, patch.northing_offset.as_ref())?;
            apply_optional_field_patch(&mut row.easting_offset, patch.easting_offset.as_ref())?;
            Ok(())
        }
        TrajectoryEditRequest::DeleteRow { row_index } => delete_row(rows, *row_index),
    }
}

fn apply_tops_edit(rows: &mut Vec<TopRow>, edit: &TopSetEditRequest) -> Result<()> {
    match edit {
        TopSetEditRequest::AddRow { row, at_index } => insert_row(rows, row.clone(), *at_index),
        TopSetEditRequest::UpdateRow { row_index, patch } => {
            let row_count = rows.len();
            let row = rows
                .get_mut(*row_index)
                .ok_or_else(|| row_index_error(*row_index, row_count))?;
            if let Some(value) = &patch.name {
                row.name = value.clone();
            }
            if let Some(value) = patch.top_depth {
                row.top_depth = value;
            }
            apply_optional_field_patch(&mut row.base_depth, patch.base_depth.as_ref())?;
            apply_optional_field_patch(&mut row.source, patch.source.as_ref())?;
            apply_optional_field_patch(
                &mut row.source_depth_reference,
                patch.source_depth_reference.as_ref(),
            )?;
            apply_optional_field_patch(&mut row.depth_domain, patch.depth_domain.as_ref())?;
            apply_optional_field_patch(&mut row.depth_datum, patch.depth_datum.as_ref())?;
            crate::project_assets::normalize_top_row_depth_semantics(row);
            Ok(())
        }
        TopSetEditRequest::DeleteRow { row_index } => delete_row(rows, *row_index),
    }
}

fn apply_well_marker_edit(
    rows: &mut Vec<WellMarkerRow>,
    edit: &WellMarkerSetEditRequest,
) -> Result<()> {
    match edit {
        WellMarkerSetEditRequest::AddRow { row, at_index } => {
            insert_row(rows, row.clone(), *at_index)
        }
        WellMarkerSetEditRequest::UpdateRow { row_index, patch } => {
            let row_count = rows.len();
            let row = rows
                .get_mut(*row_index)
                .ok_or_else(|| row_index_error(*row_index, row_count))?;
            if let Some(value) = &patch.name {
                row.name = value.clone();
            }
            apply_optional_field_patch(&mut row.marker_kind, patch.marker_kind.as_ref())?;
            if let Some(value) = patch.top_depth {
                row.top_depth = value;
            }
            apply_optional_field_patch(&mut row.base_depth, patch.base_depth.as_ref())?;
            apply_optional_field_patch(&mut row.source, patch.source.as_ref())?;
            apply_optional_field_patch(
                &mut row.source_depth_reference,
                patch.source_depth_reference.as_ref(),
            )?;
            apply_optional_field_patch(&mut row.depth_domain, patch.depth_domain.as_ref())?;
            apply_optional_field_patch(&mut row.depth_datum, patch.depth_datum.as_ref())?;
            apply_optional_field_patch(&mut row.note, patch.note.as_ref())?;
            crate::project_assets::normalize_well_marker_row_depth_semantics(row);
            Ok(())
        }
        WellMarkerSetEditRequest::DeleteRow { row_index } => delete_row(rows, *row_index),
    }
}

fn apply_pressure_edit(
    rows: &mut Vec<PressureObservationRow>,
    edit: &PressureObservationEditRequest,
) -> Result<()> {
    match edit {
        PressureObservationEditRequest::AddRow { row, at_index } => {
            insert_row(rows, row.clone(), *at_index)
        }
        PressureObservationEditRequest::UpdateRow { row_index, patch } => {
            let row_count = rows.len();
            let row = rows
                .get_mut(*row_index)
                .ok_or_else(|| row_index_error(*row_index, row_count))?;
            apply_optional_field_patch(&mut row.measured_depth, patch.measured_depth.as_ref())?;
            if let Some(value) = patch.pressure {
                row.pressure = value;
            }
            apply_optional_field_patch(&mut row.phase, patch.phase.as_ref())?;
            apply_optional_field_patch(&mut row.test_kind, patch.test_kind.as_ref())?;
            apply_optional_field_patch(&mut row.timestamp, patch.timestamp.as_ref())?;
            Ok(())
        }
        PressureObservationEditRequest::DeleteRow { row_index } => delete_row(rows, *row_index),
    }
}

fn apply_drilling_edit(
    rows: &mut Vec<DrillingObservationRow>,
    edit: &DrillingObservationEditRequest,
) -> Result<()> {
    match edit {
        DrillingObservationEditRequest::AddRow { row, at_index } => {
            insert_row(rows, row.clone(), *at_index)
        }
        DrillingObservationEditRequest::UpdateRow { row_index, patch } => {
            let row_count = rows.len();
            let row = rows
                .get_mut(*row_index)
                .ok_or_else(|| row_index_error(*row_index, row_count))?;
            apply_optional_field_patch(&mut row.measured_depth, patch.measured_depth.as_ref())?;
            if let Some(value) = &patch.event_kind {
                row.event_kind = value.clone();
            }
            apply_optional_field_patch(&mut row.value, patch.value.as_ref())?;
            apply_optional_field_patch(&mut row.unit, patch.unit.as_ref())?;
            apply_optional_field_patch(&mut row.timestamp, patch.timestamp.as_ref())?;
            apply_optional_field_patch(&mut row.comment, patch.comment.as_ref())?;
            Ok(())
        }
        DrillingObservationEditRequest::DeleteRow { row_index } => delete_row(rows, *row_index),
    }
}

fn insert_row<T>(rows: &mut Vec<T>, row: T, at_index: Option<usize>) -> Result<()> {
    match at_index {
        Some(index) if index <= rows.len() => rows.insert(index, row),
        Some(index) => return Err(row_index_error(index, rows.len() + 1)),
        None => rows.push(row),
    }
    Ok(())
}

fn delete_row<T>(rows: &mut Vec<T>, row_index: usize) -> Result<()> {
    if row_index >= rows.len() {
        return Err(row_index_error(row_index, rows.len()));
    }
    rows.remove(row_index);
    Ok(())
}

fn apply_optional_field_patch<T: Clone>(
    slot: &mut Option<T>,
    patch: Option<&OptionalFieldPatch<T>>,
) -> Result<()> {
    let Some(patch) = patch else {
        return Ok(());
    };
    if patch.clear && patch.set.is_some() {
        return Err(LasError::Validation(
            "field patch cannot set and clear the same field".to_string(),
        ));
    }
    if patch.clear {
        *slot = None;
    } else if let Some(value) = &patch.set {
        *slot = Some(value.clone());
    }
    Ok(())
}

fn validate_rows(rows: &StructuredRows) -> Result<()> {
    match rows {
        StructuredRows::Trajectory(rows) => validate_trajectory_rows(rows),
        StructuredRows::TopSet(rows) => validate_top_rows(rows),
        StructuredRows::WellMarkerSet(rows) => validate_well_marker_rows(rows),
        StructuredRows::Pressure(rows) => validate_pressure_rows(rows),
        StructuredRows::Drilling(rows) => validate_drilling_rows(rows),
    }
}

fn validate_trajectory_rows(rows: &[TrajectoryRow]) -> Result<()> {
    let mut previous_depth = None;
    for (index, row) in rows.iter().enumerate() {
        if !row.measured_depth.is_finite() {
            return Err(LasError::Validation(format!(
                "trajectory row {index} measured_depth must be a finite number"
            )));
        }
        if let Some(previous) = previous_depth
            && row.measured_depth < previous
        {
            return Err(LasError::Validation(format!(
                "trajectory rows must be monotonic by measured_depth; row {index} is out of order"
            )));
        }
        previous_depth = Some(row.measured_depth);
    }
    Ok(())
}

fn validate_top_rows(rows: &[TopRow]) -> Result<()> {
    for (index, row) in rows.iter().enumerate() {
        if row.name.trim().is_empty() {
            return Err(LasError::Validation(format!(
                "top row {index} requires a name"
            )));
        }
        if !row.top_depth.is_finite() {
            return Err(LasError::Validation(format!(
                "top row {index} requires a finite top_depth"
            )));
        }
        if let Some(base_depth) = row.base_depth
            && base_depth < row.top_depth
        {
            return Err(LasError::Validation(format!(
                "top row {index} base_depth must be greater than or equal to top_depth"
            )));
        }
    }
    Ok(())
}

fn validate_pressure_rows(rows: &[PressureObservationRow]) -> Result<()> {
    for (index, row) in rows.iter().enumerate() {
        if !row.pressure.is_finite() {
            return Err(LasError::Validation(format!(
                "pressure row {index} requires a finite pressure value"
            )));
        }
    }
    Ok(())
}

fn validate_well_marker_rows(rows: &[WellMarkerRow]) -> Result<()> {
    for (index, row) in rows.iter().enumerate() {
        if row.name.trim().is_empty() {
            return Err(LasError::Validation(format!(
                "well marker row {index} requires a name"
            )));
        }
        if !row.top_depth.is_finite() {
            return Err(LasError::Validation(format!(
                "well marker row {index} requires a finite top_depth"
            )));
        }
        if let Some(base_depth) = row.base_depth
            && base_depth < row.top_depth
        {
            return Err(LasError::Validation(format!(
                "well marker row {index} base_depth must be greater than or equal to top_depth"
            )));
        }
    }
    Ok(())
}

fn validate_drilling_rows(rows: &[DrillingObservationRow]) -> Result<()> {
    for (index, row) in rows.iter().enumerate() {
        if row.event_kind.trim().is_empty() {
            return Err(LasError::Validation(format!(
                "drilling row {index} requires an event_kind"
            )));
        }
    }
    Ok(())
}

fn row_index_error(row_index: usize, row_count: usize) -> LasError {
    LasError::Validation(format!(
        "row index {row_index} was out of bounds for a session with {row_count} rows"
    ))
}

fn kind_mismatch_error(actual: AssetKind, expected: AssetKind) -> LasError {
    LasError::Validation(format!(
        "structured asset edit session targets {}, not {}",
        asset_kind_name(&actual),
        asset_kind_name(&expected)
    ))
}

fn asset_kind_name(kind: &AssetKind) -> &'static str {
    match kind {
        AssetKind::Log => "log",
        AssetKind::Trajectory => "trajectory",
        AssetKind::TopSet => "top_set",
        AssetKind::WellMarkerSet => "well_marker_set",
        AssetKind::WellMarkerHorizonResidualSet => "well_marker_horizon_residual_set",
        AssetKind::PressureObservation => "pressure_observation",
        AssetKind::DrillingObservation => "drilling_observation",
        AssetKind::CheckshotVspObservationSet => "checkshot_vsp_observation_set",
        AssetKind::ManualTimeDepthPickSet => "manual_time_depth_pick_set",
        AssetKind::WellTieObservationSet => "well_tie_observation_set",
        AssetKind::WellTimeDepthAuthoredModel => "well_time_depth_authored_model",
        AssetKind::WellTimeDepthModel => "well_time_depth_model",
        AssetKind::RawSourceBundle => "raw_source_bundle",
        AssetKind::SeismicTraceData => "seismic_trace_data",
    }
}

fn next_session_id() -> StructuredAssetEditSessionId {
    let counter = STRUCTURED_EDIT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    StructuredAssetEditSessionId(format!("structured_edit_{nanos}_{counter}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn structured_edit_store_updates_and_saves_tops() {
        let root = temp_project_root("structured_edit_store_updates_and_saves_tops");
        let mut project = OphioliteProject::create(&root).unwrap();
        let csv_path = write_csv(
            &root,
            "tops.csv",
            "name,top_depth,base_depth,source,depth_reference\nTop A,100,101,interp,MD\n",
        );
        let binding = crate::AssetBindingInput {
            well_name: "Well A".to_string(),
            wellbore_name: "WB-1".to_string(),
            uwi: Some("UWI-1".to_string()),
            api: None,
            operator_aliases: Vec::new(),
        };
        let imported = project
            .import_tops_csv(&csv_path, &binding, Some("tops"))
            .unwrap();

        let mut store = StructuredAssetEditSessionStore::default();
        let summary = store
            .open_session(&OpenStructuredAssetEditSessionRequest {
                project_root: root.display().to_string(),
                asset_id: imported.asset.id.clone(),
            })
            .unwrap();

        store
            .apply_tops_edit(
                &StructuredAssetSessionRequest {
                    session_id: summary.session_id.clone(),
                },
                &TopSetEditRequest::UpdateRow {
                    row_index: 0,
                    patch: TopRowPatch {
                        name: Some("Top B".to_string()),
                        top_depth: Some(110.0),
                        base_depth: Some(OptionalFieldPatch {
                            set: Some(111.0),
                            clear: false,
                        }),
                        ..Default::default()
                    },
                },
            )
            .unwrap();
        let saved = store
            .save_session(&StructuredAssetSessionRequest {
                session_id: summary.session_id.clone(),
            })
            .unwrap();

        assert!(!saved.session.dirty);
        let reopened = OphioliteProject::open(&root).unwrap();
        let rows = reopened.read_tops(&imported.asset.id).unwrap();
        assert_eq!(rows[0].name, "Top B");
        assert_eq!(rows[0].top_depth, 110.0);
        assert_eq!(rows[0].base_depth, Some(111.0));
    }

    #[test]
    fn structured_edit_store_rejects_invalid_trajectory_save() {
        let root = temp_project_root("structured_edit_store_rejects_invalid_trajectory_save");
        let mut project = OphioliteProject::create(&root).unwrap();
        let csv_path = write_csv(
            &root,
            "trajectory.csv",
            "measured_depth,true_vertical_depth\n100,90\n110,99\n",
        );
        let binding = crate::AssetBindingInput {
            well_name: "Well A".to_string(),
            wellbore_name: "WB-1".to_string(),
            uwi: Some("UWI-1".to_string()),
            api: None,
            operator_aliases: Vec::new(),
        };
        let imported = project
            .import_trajectory_csv(&csv_path, &binding, Some("trajectory"))
            .unwrap();

        let mut store = StructuredAssetEditSessionStore::default();
        let summary = store
            .open_session(&OpenStructuredAssetEditSessionRequest {
                project_root: root.display().to_string(),
                asset_id: imported.asset.id.clone(),
            })
            .unwrap();

        store
            .apply_trajectory_edit(
                &StructuredAssetSessionRequest {
                    session_id: summary.session_id.clone(),
                },
                &TrajectoryEditRequest::UpdateRow {
                    row_index: 1,
                    patch: TrajectoryRowPatch {
                        measured_depth: Some(95.0),
                        ..Default::default()
                    },
                },
            )
            .unwrap();

        let error = store
            .save_session(&StructuredAssetSessionRequest {
                session_id: summary.session_id.clone(),
            })
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("trajectory rows must be monotonic")
        );

        let reopened = OphioliteProject::open(&root).unwrap();
        let rows = reopened
            .read_trajectory_rows(&imported.asset.id, None)
            .unwrap();
        assert_eq!(rows[1].measured_depth, 110.0);
    }

    #[test]
    fn structured_edit_store_updates_and_saves_well_markers() {
        let root = temp_project_root("structured_edit_store_updates_and_saves_well_markers");
        let mut project = OphioliteProject::create(&root).unwrap();
        let csv_path = write_csv(
            &root,
            "markers.csv",
            "name,marker_kind,top_depth,base_depth,source,depth_reference,note\nMarker A,formation,100,101,interp,MD,initial\n",
        );
        let binding = crate::AssetBindingInput {
            well_name: "Well A".to_string(),
            wellbore_name: "WB-1".to_string(),
            uwi: Some("UWI-1".to_string()),
            api: None,
            operator_aliases: Vec::new(),
        };
        let imported = project
            .import_well_markers_csv(&csv_path, &binding, Some("markers"))
            .unwrap();

        let mut store = StructuredAssetEditSessionStore::default();
        let summary = store
            .open_session(&OpenStructuredAssetEditSessionRequest {
                project_root: root.display().to_string(),
                asset_id: imported.asset.id.clone(),
            })
            .unwrap();

        store
            .apply_well_marker_edit(
                &StructuredAssetSessionRequest {
                    session_id: summary.session_id.clone(),
                },
                &WellMarkerSetEditRequest::UpdateRow {
                    row_index: 0,
                    patch: WellMarkerRowPatch {
                        name: Some("Marker B".to_string()),
                        marker_kind: Some(OptionalFieldPatch {
                            set: Some("fault".to_string()),
                            clear: false,
                        }),
                        top_depth: Some(110.0),
                        base_depth: Some(OptionalFieldPatch {
                            set: Some(111.0),
                            clear: false,
                        }),
                        note: Some(OptionalFieldPatch {
                            set: Some("revised".to_string()),
                            clear: false,
                        }),
                        ..Default::default()
                    },
                },
            )
            .unwrap();
        let saved = store
            .save_session(&StructuredAssetSessionRequest {
                session_id: summary.session_id.clone(),
            })
            .unwrap();

        assert!(!saved.session.dirty);
        let reopened = OphioliteProject::open(&root).unwrap();
        let rows = reopened.read_well_marker_rows(&imported.asset.id).unwrap();
        assert_eq!(rows[0].name, "Marker B");
        assert_eq!(rows[0].marker_kind.as_deref(), Some("fault"));
        assert_eq!(rows[0].top_depth, 110.0);
        assert_eq!(rows[0].base_depth, Some(111.0));
        assert_eq!(rows[0].note.as_deref(), Some("revised"));

        let canonical = reopened
            .list_well_markers(&imported.resolution.wellbore_id)
            .unwrap();
        assert_eq!(canonical[0].name, "Marker B");
        assert_eq!(canonical[0].marker_kind.as_deref(), Some("fault"));
    }

    fn temp_project_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ophiolite_{label}_{unique}"));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        root
    }

    fn write_csv(root: &std::path::Path, name: &str, contents: &str) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
}
