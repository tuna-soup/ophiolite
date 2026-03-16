use crate::{
    AssetSummaryDto, CloseSessionResultDto, CurveCatalogDto, CurveEditRequest, CurveWindowRequest,
    DTO_CONTRACT_VERSION, DirtyStateDto, LasError, MetadataDto, MetadataUpdateRequest,
    PackageSessionStore, Result, SaveSessionResponseDto, SessionId, SessionMetadataDto,
    SessionSummaryDto, SessionWindowDto, ValidationReportDto, close_session_result_dto,
    curve_catalog_result_dto, open_package_metadata, open_package_summary, session_metadata_dto,
    session_window_dto, validate_package,
};
use std::path::Path;

#[derive(Debug, Default)]
pub struct PackageBackend {
    sessions: PackageSessionStore,
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
        let session = self.session(session_id)?;
        Ok(session.session_summary())
    }

    pub fn session_metadata(&self, session_id: &SessionId) -> Result<SessionMetadataDto> {
        let session = self.session(session_id)?;
        Ok(session_metadata_dto(
            session.package_id().clone(),
            session.session_id().clone(),
            session.revision().clone(),
            session.metadata_dto(),
        ))
    }

    pub fn session_curve_catalog(&self, session_id: &SessionId) -> Result<CurveCatalogDto> {
        let session = self.session(session_id)?;
        Ok(curve_catalog_result_dto(
            session.package_id().clone(),
            session.session_id().clone(),
            session.revision().clone(),
            session.curve_catalog(),
        ))
    }

    pub fn read_curve_window(
        &self,
        session_id: &SessionId,
        request: &CurveWindowRequest,
    ) -> Result<SessionWindowDto> {
        let session = self.session(session_id)?;
        Ok(session_window_dto(
            session.package_id().clone(),
            session.session_id().clone(),
            session.revision().clone(),
            session.read_window(request)?,
        ))
    }

    pub fn dirty_state(&self, session_id: &SessionId) -> Result<DirtyStateDto> {
        let session = self.session(session_id)?;
        Ok(session.dirty_state())
    }

    pub fn apply_metadata_edit(
        &mut self,
        session_id: &SessionId,
        request: &MetadataUpdateRequest,
    ) -> Result<SessionSummaryDto> {
        let session = self.session_mut(session_id)?;
        session.apply_metadata_update(request)?;
        Ok(session.session_summary())
    }

    pub fn apply_curve_edit(
        &mut self,
        session_id: &SessionId,
        request: &CurveEditRequest,
    ) -> Result<SessionSummaryDto> {
        let session = self.session_mut(session_id)?;
        session.apply_curve_edit(request)?;
        Ok(session.session_summary())
    }

    pub fn save_session(&mut self, session_id: &SessionId) -> Result<SaveSessionResponseDto> {
        let session = self.session_mut(session_id)?;
        match session.save_checked()? {
            Ok(saved) => Ok(SaveSessionResponseDto::Saved(saved)),
            Err(conflict) => Ok(SaveSessionResponseDto::Conflict(conflict)),
        }
    }

    pub fn save_session_as(
        &mut self,
        session_id: &SessionId,
        output_dir: impl AsRef<Path>,
    ) -> Result<SaveSessionResponseDto> {
        let old_root = {
            let session = self.session(session_id)?;
            session.root().to_path_buf()
        };
        let save_result = {
            let session = self.session_mut(session_id)?;
            session.save_as_in_place(output_dir.as_ref())?
        };
        let new_root = {
            let session = self.session(session_id)?;
            session.root().to_path_buf()
        };
        self.sessions.rebind_path(session_id, &old_root, &new_root);
        Ok(SaveSessionResponseDto::Saved(save_result))
    }

    pub fn close_session(&mut self, session_id: &SessionId) -> Result<CloseSessionResultDto> {
        let closed = self
            .sessions
            .close(session_id)
            .ok_or_else(|| LasError::Validation(format!("session '{}' not found", session_id.0)))?;
        Ok(close_session_result_dto(
            closed.package_id().clone(),
            session_id.clone(),
            true,
        ))
    }

    fn session(&self, session_id: &SessionId) -> Result<&crate::PackageSession> {
        self.sessions
            .get(session_id)
            .ok_or_else(|| LasError::Validation(format!("session '{}' not found", session_id.0)))
    }

    fn session_mut(&mut self, session_id: &SessionId) -> Result<&mut crate::PackageSession> {
        self.sessions
            .get_mut(session_id)
            .ok_or_else(|| LasError::Validation(format!("session '{}' not found", session_id.0)))
    }
}

pub fn dto_contract_version() -> &'static str {
    DTO_CONTRACT_VERSION
}
