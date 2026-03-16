use crate::{
    AssetSummaryDto, CloseSessionResultDto, CommandErrorDto, CommandErrorKind, CommandGroup,
    CommandResponse, CurveCatalogDto, DirtyStateDto, LasError, MetadataDto, PackageBackendState,
    PackagePathRequest, SavePackageResultDto, SaveSessionResponseDto, SessionCurveEditRequest,
    SessionId, SessionMetadataDto, SessionMetadataEditRequest, SessionRequest,
    SessionSaveAsRequest, SessionSummaryDto, SessionWindowDto, SessionWindowRequest,
    ValidationKind, ValidationReportDto, command_error_dto,
};

#[derive(Clone, Default)]
pub struct PackageCommandService {
    state: PackageBackendState,
}

impl PackageCommandService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_state(state: PackageBackendState) -> Self {
        Self { state }
    }

    pub fn inspect_package_summary(
        &self,
        request: &PackagePathRequest,
    ) -> CommandResponse<AssetSummaryDto> {
        self.map_result(
            CommandGroup::Inspect,
            None,
            self.state.inspect_package_summary(request),
        )
    }

    pub fn inspect_package_metadata(
        &self,
        request: &PackagePathRequest,
    ) -> CommandResponse<MetadataDto> {
        self.map_result(
            CommandGroup::Inspect,
            None,
            self.state.inspect_package_metadata(request),
        )
    }

    pub fn validate_package(
        &self,
        request: &PackagePathRequest,
    ) -> CommandResponse<ValidationReportDto> {
        self.map_result(
            CommandGroup::Inspect,
            None,
            self.state.validate_package(request),
        )
    }

    pub fn open_package_session(
        &self,
        request: &PackagePathRequest,
    ) -> CommandResponse<SessionSummaryDto> {
        self.map_result(
            CommandGroup::Session,
            None,
            self.state.open_package_session(request),
        )
    }

    pub fn session_summary(&self, request: &SessionRequest) -> CommandResponse<SessionSummaryDto> {
        self.map_result(
            CommandGroup::Session,
            Some(request.session_id.clone()),
            self.state.session_summary(request),
        )
    }

    pub fn session_metadata(
        &self,
        request: &SessionRequest,
    ) -> CommandResponse<SessionMetadataDto> {
        self.map_result(
            CommandGroup::Session,
            Some(request.session_id.clone()),
            self.state.session_metadata(request),
        )
    }

    pub fn session_curve_catalog(
        &self,
        request: &SessionRequest,
    ) -> CommandResponse<CurveCatalogDto> {
        self.map_result(
            CommandGroup::Session,
            Some(request.session_id.clone()),
            self.state.session_curve_catalog(request),
        )
    }

    pub fn read_curve_window(
        &self,
        request: &SessionWindowRequest,
    ) -> CommandResponse<SessionWindowDto> {
        self.map_result(
            CommandGroup::Session,
            Some(request.session_id.clone()),
            self.state.read_curve_window(request),
        )
    }

    pub fn dirty_state(&self, request: &SessionRequest) -> CommandResponse<DirtyStateDto> {
        self.map_result(
            CommandGroup::Session,
            Some(request.session_id.clone()),
            self.state.dirty_state(request),
        )
    }

    pub fn close_session(
        &self,
        request: &SessionRequest,
    ) -> CommandResponse<CloseSessionResultDto> {
        self.map_result(
            CommandGroup::Session,
            Some(request.session_id.clone()),
            self.state.close_session(request),
        )
    }

    pub fn apply_metadata_edit(
        &self,
        request: &SessionMetadataEditRequest,
    ) -> CommandResponse<SessionSummaryDto> {
        self.map_result(
            CommandGroup::EditPersist,
            Some(request.session_id.clone()),
            self.state.apply_metadata_edit(request),
        )
    }

    pub fn apply_curve_edit(
        &self,
        request: &SessionCurveEditRequest,
    ) -> CommandResponse<SessionSummaryDto> {
        self.map_result(
            CommandGroup::EditPersist,
            Some(request.session_id.clone()),
            self.state.apply_curve_edit(request),
        )
    }

    pub fn save_session(&self, request: &SessionRequest) -> CommandResponse<SavePackageResultDto> {
        match self.state.save_session(request) {
            Ok(SaveSessionResponseDto::Saved(saved)) => CommandResponse::Ok(saved),
            Ok(SaveSessionResponseDto::Conflict(conflict)) => {
                CommandResponse::Err(self.save_conflict_error(CommandGroup::EditPersist, conflict))
            }
            Err(error) => self.map_error(
                CommandGroup::EditPersist,
                Some(request.session_id.clone()),
                error,
            ),
        }
    }

    pub fn save_session_as(
        &self,
        request: &SessionSaveAsRequest,
    ) -> CommandResponse<SavePackageResultDto> {
        match self.state.save_session_as(request) {
            Ok(SaveSessionResponseDto::Saved(saved)) => CommandResponse::Ok(saved),
            Ok(SaveSessionResponseDto::Conflict(conflict)) => {
                CommandResponse::Err(self.save_conflict_error(CommandGroup::EditPersist, conflict))
            }
            Err(error) => self.map_error(
                CommandGroup::EditPersist,
                Some(request.session_id.clone()),
                error,
            ),
        }
    }

    fn map_result<T>(
        &self,
        group: CommandGroup,
        session_id: Option<SessionId>,
        result: crate::Result<T>,
    ) -> CommandResponse<T> {
        match result {
            Ok(value) => CommandResponse::Ok(value),
            Err(error) => self.map_error(group, session_id, error),
        }
    }

    fn map_error<T>(
        &self,
        group: CommandGroup,
        session_id: Option<SessionId>,
        error: LasError,
    ) -> CommandResponse<T> {
        CommandResponse::Err(command_error_for(group, session_id, error))
    }

    fn save_conflict_error(
        &self,
        group: CommandGroup,
        conflict: crate::SaveConflictDto,
    ) -> CommandErrorDto {
        let mut error = command_error_dto(group, CommandErrorKind::SaveConflict, "save conflict");
        error.session_id = Some(conflict.session_id.clone());
        error.save_conflict = Some(conflict.clone());
        error.message = format!(
            "save conflict for session '{}': expected {}, found {}",
            conflict.session_id.0, conflict.expected_revision.0, conflict.actual_revision.0
        );
        error
    }
}

fn command_error_for(
    group: CommandGroup,
    session_id: Option<SessionId>,
    error: LasError,
) -> CommandErrorDto {
    match error {
        LasError::Validation(message) if is_backend_state_error(&message) => {
            let mut dto = command_error_dto(group, CommandErrorKind::Internal, message);
            dto.session_id = session_id;
            dto
        }
        LasError::Validation(message) if is_missing_session(&message) => {
            let mut dto = command_error_dto(group, CommandErrorKind::SessionNotFound, message);
            dto.session_id = session_id;
            dto
        }
        LasError::Validation(message) => {
            let mut dto = command_error_dto(group, CommandErrorKind::ValidationFailed, &message);
            dto.session_id = session_id;
            dto.validation = Some(validation_error_report(group, message));
            dto
        }
        LasError::Io(err) => {
            command_error_dto(group, CommandErrorKind::OpenFailed, err.to_string())
        }
        LasError::Parse(message) => command_error_dto(group, CommandErrorKind::OpenFailed, message),
        LasError::Unsupported(message) => {
            command_error_dto(group, CommandErrorKind::OpenFailed, message)
        }
        LasError::Storage(message) => {
            command_error_dto(group, CommandErrorKind::OpenFailed, message)
        }
        LasError::Serialization(err) => {
            command_error_dto(group, CommandErrorKind::Internal, err.to_string())
        }
    }
}

fn validation_error_report(group: CommandGroup, message: String) -> ValidationReportDto {
    let kind = match group {
        CommandGroup::Inspect | CommandGroup::Session => ValidationKind::Package,
        CommandGroup::EditPersist => ValidationKind::Edit,
    };

    ValidationReportDto {
        dto_contract_version: String::from(crate::DTO_CONTRACT_VERSION),
        kind,
        valid: false,
        errors: vec![message],
    }
}

fn is_missing_session(message: &str) -> bool {
    message.starts_with("session '") && message.ends_with("' not found")
}

fn is_backend_state_error(message: &str) -> bool {
    message == "package backend state is poisoned"
}
