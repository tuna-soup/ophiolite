use crate::{
    AssetSummaryDto, CloseSessionResultDto, CurveCatalogDto, CurveEditRequest, CurveWindowRequest,
    DTO_CONTRACT_VERSION, DepthWindowRequest, DirtyStateDto, MetadataDto, MetadataUpdateRequest,
    Result, SaveSessionResponseDto, SessionId, SessionMetadataDto, SessionSummaryDto,
    SessionWindowDto, ValidationReportDto, close_session_result_dto, open_package_metadata,
    open_package_summary, validate_package,
};
use lithos_package::PackageBackendSessionStore;
use std::path::Path;

#[derive(Debug, Default)]
pub struct PackageBackend {
    sessions: PackageBackendSessionStore,
}

impl PackageBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inspect_package_summary(&self, path: impl AsRef<Path>) -> Result<AssetSummaryDto> {
        open_package_summary(path)
    }

    pub fn inspect_package_metadata(&self, path: impl AsRef<Path>) -> Result<MetadataDto> {
        open_package_metadata(path)
    }

    pub fn validate_package_path(&self, path: impl AsRef<Path>) -> Result<ValidationReportDto> {
        validate_package(path)
    }

    pub fn open_package_session(&mut self, path: impl AsRef<Path>) -> Result<SessionSummaryDto> {
        self.sessions.open_shared(path)
    }

    pub fn session_summary(&self, session_id: &SessionId) -> Result<SessionSummaryDto> {
        self.sessions.session_summary(session_id)
    }

    pub fn session_metadata(&self, session_id: &SessionId) -> Result<SessionMetadataDto> {
        self.sessions.session_metadata(session_id)
    }

    pub fn session_curve_catalog(&self, session_id: &SessionId) -> Result<CurveCatalogDto> {
        self.sessions.session_curve_catalog(session_id)
    }

    pub fn read_curve_window(
        &self,
        session_id: &SessionId,
        request: &CurveWindowRequest,
    ) -> Result<SessionWindowDto> {
        self.sessions.read_curve_window(session_id, request)
    }

    pub fn read_depth_window(
        &self,
        session_id: &SessionId,
        request: &DepthWindowRequest,
    ) -> Result<SessionWindowDto> {
        self.sessions.read_depth_window(session_id, request)
    }

    pub fn dirty_state(&self, session_id: &SessionId) -> Result<DirtyStateDto> {
        self.sessions.dirty_state(session_id)
    }

    pub fn apply_metadata_edit(
        &mut self,
        session_id: &SessionId,
        request: &MetadataUpdateRequest,
    ) -> Result<SessionSummaryDto> {
        self.sessions.apply_metadata_edit(session_id, request)
    }

    pub fn apply_curve_edit(
        &mut self,
        session_id: &SessionId,
        request: &CurveEditRequest,
    ) -> Result<SessionSummaryDto> {
        self.sessions.apply_curve_edit(session_id, request)
    }

    pub fn save_session(&mut self, session_id: &SessionId) -> Result<SaveSessionResponseDto> {
        self.sessions.save_session(session_id)
    }

    pub fn save_session_as(
        &mut self,
        session_id: &SessionId,
        output_dir: impl AsRef<Path>,
    ) -> Result<SaveSessionResponseDto> {
        self.sessions.save_session_as(session_id, output_dir)
    }

    pub fn close_session(&mut self, session_id: &SessionId) -> Result<CloseSessionResultDto> {
        let package_id = self.sessions.close(session_id).ok_or_else(|| {
            crate::LasError::Validation(format!("session '{}' not found", session_id.0))
        })?;
        Ok(close_session_result_dto(
            package_id,
            session_id.clone(),
            true,
        ))
    }
}

pub fn dto_contract_version() -> &'static str {
    DTO_CONTRACT_VERSION
}
