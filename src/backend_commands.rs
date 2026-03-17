use crate::{
    AssetSummaryDto, CloseSessionResultDto, CurveCatalogDto, DirtyStateDto, LasError, MetadataDto,
    PackageBackend, PackagePathRequest, Result, SaveSessionResponseDto, SessionCurveEditRequest,
    SessionDepthWindowRequest, SessionMetadataDto, SessionMetadataEditRequest, SessionRequest,
    SessionSummaryDto, SessionWindowDto, SessionWindowRequest, ValidationReportDto,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct PackageBackendState {
    backend: Arc<Mutex<PackageBackend>>,
}

impl PackageBackendState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inspect_package_summary(&self, request: &PackagePathRequest) -> Result<AssetSummaryDto> {
        self.with_backend(|backend| backend.inspect_package_summary(&request.path))
    }

    pub fn inspect_package_metadata(&self, request: &PackagePathRequest) -> Result<MetadataDto> {
        self.with_backend(|backend| backend.inspect_package_metadata(&request.path))
    }

    pub fn validate_package(&self, request: &PackagePathRequest) -> Result<ValidationReportDto> {
        self.with_backend(|backend| backend.validate_package_path(&request.path))
    }

    pub fn open_package_session(&self, request: &PackagePathRequest) -> Result<SessionSummaryDto> {
        self.with_backend(|backend| backend.open_package_session(&request.path))
    }

    pub fn session_summary(&self, request: &SessionRequest) -> Result<SessionSummaryDto> {
        self.with_backend(|backend| backend.session_summary(&request.session_id))
    }

    pub fn session_metadata(&self, request: &SessionRequest) -> Result<SessionMetadataDto> {
        self.with_backend(|backend| backend.session_metadata(&request.session_id))
    }

    pub fn session_curve_catalog(&self, request: &SessionRequest) -> Result<CurveCatalogDto> {
        self.with_backend(|backend| backend.session_curve_catalog(&request.session_id))
    }

    pub fn read_curve_window(&self, request: &SessionWindowRequest) -> Result<SessionWindowDto> {
        self.with_backend(|backend| backend.read_curve_window(&request.session_id, &request.window))
    }

    pub fn read_depth_window(
        &self,
        request: &SessionDepthWindowRequest,
    ) -> Result<SessionWindowDto> {
        self.with_backend(|backend| backend.read_depth_window(&request.session_id, &request.window))
    }

    pub fn dirty_state(&self, request: &SessionRequest) -> Result<DirtyStateDto> {
        self.with_backend(|backend| backend.dirty_state(&request.session_id))
    }

    pub fn apply_metadata_edit(
        &self,
        request: &SessionMetadataEditRequest,
    ) -> Result<SessionSummaryDto> {
        self.with_backend(|backend| {
            backend.apply_metadata_edit(&request.session_id, &request.update)
        })
    }

    pub fn apply_curve_edit(&self, request: &SessionCurveEditRequest) -> Result<SessionSummaryDto> {
        self.with_backend(|backend| backend.apply_curve_edit(&request.session_id, &request.edit))
    }

    pub fn save_session(&self, request: &SessionRequest) -> Result<SaveSessionResponseDto> {
        self.with_backend(|backend| backend.save_session(&request.session_id))
    }

    pub fn save_session_as(
        &self,
        request: &crate::SessionSaveAsRequest,
    ) -> Result<SaveSessionResponseDto> {
        self.with_backend(|backend| {
            backend.save_session_as(&request.session_id, &request.output_dir)
        })
    }

    pub fn close_session(&self, request: &SessionRequest) -> Result<CloseSessionResultDto> {
        self.with_backend(|backend| backend.close_session(&request.session_id))
    }

    fn with_backend<T>(&self, f: impl FnOnce(&mut PackageBackend) -> Result<T>) -> Result<T> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| LasError::Validation(String::from("package backend state is poisoned")))?;
        f(&mut backend)
    }
}
