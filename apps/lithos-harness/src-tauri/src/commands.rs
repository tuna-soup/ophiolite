use lithos_las::{
    AssetSummaryDto, CloseSessionResultDto, CommandResponse, CurveCatalogDto, DirtyStateDto,
    MetadataDto, PackageCommandService, PackagePathRequest, SavePackageResultDto,
    SessionCurveEditRequest, SessionMetadataDto, SessionMetadataEditRequest, SessionRequest,
    SessionSaveAsRequest, SessionSummaryDto, SessionWindowDto, SessionWindowRequest,
    ValidationReportDto,
};

#[derive(Clone, Default)]
pub struct HarnessState {
    service: PackageCommandService,
}

impl HarnessState {
    fn service(&self) -> &PackageCommandService {
        &self.service
    }
}

fn inspect_package_summary_impl(
    service: &PackageCommandService,
    request: PackagePathRequest,
) -> CommandResponse<AssetSummaryDto> {
    service.inspect_package_summary(&request)
}

fn inspect_package_metadata_impl(
    service: &PackageCommandService,
    request: PackagePathRequest,
) -> CommandResponse<MetadataDto> {
    service.inspect_package_metadata(&request)
}

fn validate_package_impl(
    service: &PackageCommandService,
    request: PackagePathRequest,
) -> CommandResponse<ValidationReportDto> {
    service.validate_package(&request)
}

fn open_package_session_impl(
    service: &PackageCommandService,
    request: PackagePathRequest,
) -> CommandResponse<SessionSummaryDto> {
    service.open_package_session(&request)
}

fn session_summary_impl(
    service: &PackageCommandService,
    request: SessionRequest,
) -> CommandResponse<SessionSummaryDto> {
    service.session_summary(&request)
}

fn session_metadata_impl(
    service: &PackageCommandService,
    request: SessionRequest,
) -> CommandResponse<SessionMetadataDto> {
    service.session_metadata(&request)
}

fn session_curve_catalog_impl(
    service: &PackageCommandService,
    request: SessionRequest,
) -> CommandResponse<CurveCatalogDto> {
    service.session_curve_catalog(&request)
}

fn read_curve_window_impl(
    service: &PackageCommandService,
    request: SessionWindowRequest,
) -> CommandResponse<SessionWindowDto> {
    service.read_curve_window(&request)
}

fn dirty_state_impl(
    service: &PackageCommandService,
    request: SessionRequest,
) -> CommandResponse<DirtyStateDto> {
    service.dirty_state(&request)
}

fn close_session_impl(
    service: &PackageCommandService,
    request: SessionRequest,
) -> CommandResponse<CloseSessionResultDto> {
    service.close_session(&request)
}

fn apply_metadata_edit_impl(
    service: &PackageCommandService,
    request: SessionMetadataEditRequest,
) -> CommandResponse<SessionSummaryDto> {
    service.apply_metadata_edit(&request)
}

fn apply_curve_edit_impl(
    service: &PackageCommandService,
    request: SessionCurveEditRequest,
) -> CommandResponse<SessionSummaryDto> {
    service.apply_curve_edit(&request)
}

fn save_session_impl(
    service: &PackageCommandService,
    request: SessionRequest,
) -> CommandResponse<SavePackageResultDto> {
    service.save_session(&request)
}

fn save_session_as_impl(
    service: &PackageCommandService,
    request: SessionSaveAsRequest,
) -> CommandResponse<SavePackageResultDto> {
    service.save_session_as(&request)
}

#[tauri::command]
pub fn inspect_package_summary(
    state: tauri::State<HarnessState>,
    request: PackagePathRequest,
) -> CommandResponse<AssetSummaryDto> {
    inspect_package_summary_impl(state.service(), request)
}

#[tauri::command]
pub fn inspect_package_metadata(
    state: tauri::State<HarnessState>,
    request: PackagePathRequest,
) -> CommandResponse<MetadataDto> {
    inspect_package_metadata_impl(state.service(), request)
}

#[tauri::command]
pub fn validate_package(
    state: tauri::State<HarnessState>,
    request: PackagePathRequest,
) -> CommandResponse<ValidationReportDto> {
    validate_package_impl(state.service(), request)
}

#[tauri::command]
pub fn open_package_session(
    state: tauri::State<HarnessState>,
    request: PackagePathRequest,
) -> CommandResponse<SessionSummaryDto> {
    open_package_session_impl(state.service(), request)
}

#[tauri::command]
pub fn session_summary(
    state: tauri::State<HarnessState>,
    request: SessionRequest,
) -> CommandResponse<SessionSummaryDto> {
    session_summary_impl(state.service(), request)
}

#[tauri::command]
pub fn session_metadata(
    state: tauri::State<HarnessState>,
    request: SessionRequest,
) -> CommandResponse<SessionMetadataDto> {
    session_metadata_impl(state.service(), request)
}

#[tauri::command]
pub fn session_curve_catalog(
    state: tauri::State<HarnessState>,
    request: SessionRequest,
) -> CommandResponse<CurveCatalogDto> {
    session_curve_catalog_impl(state.service(), request)
}

#[tauri::command]
pub fn read_curve_window(
    state: tauri::State<HarnessState>,
    request: SessionWindowRequest,
) -> CommandResponse<SessionWindowDto> {
    read_curve_window_impl(state.service(), request)
}

#[tauri::command]
pub fn dirty_state(
    state: tauri::State<HarnessState>,
    request: SessionRequest,
) -> CommandResponse<DirtyStateDto> {
    dirty_state_impl(state.service(), request)
}

#[tauri::command]
pub fn close_session(
    state: tauri::State<HarnessState>,
    request: SessionRequest,
) -> CommandResponse<CloseSessionResultDto> {
    close_session_impl(state.service(), request)
}

#[tauri::command]
pub fn apply_metadata_edit(
    state: tauri::State<HarnessState>,
    request: SessionMetadataEditRequest,
) -> CommandResponse<SessionSummaryDto> {
    apply_metadata_edit_impl(state.service(), request)
}

#[tauri::command]
pub fn apply_curve_edit(
    state: tauri::State<HarnessState>,
    request: SessionCurveEditRequest,
) -> CommandResponse<SessionSummaryDto> {
    apply_curve_edit_impl(state.service(), request)
}

#[tauri::command]
pub fn save_session(
    state: tauri::State<HarnessState>,
    request: SessionRequest,
) -> CommandResponse<SavePackageResultDto> {
    save_session_impl(state.service(), request)
}

#[tauri::command]
pub fn save_session_as(
    state: tauri::State<HarnessState>,
    request: SessionSaveAsRequest,
) -> CommandResponse<SavePackageResultDto> {
    save_session_as_impl(state.service(), request)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_metadata_edit_impl, open_package_session_impl, read_curve_window_impl,
        save_session_impl, session_metadata_impl,
    };
    use lithos_las::{
        CommandResponse, CurveWindowRequest, HeaderItemUpdate, MetadataSectionDto,
        MetadataUpdateRequest, PackageCommandService, PackagePathRequest,
        SessionMetadataEditRequest, SessionRequest, SessionWindowRequest, examples, write_package,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn harness_commands_open_session_and_query_window() {
        let las = examples::open("sample.las", &Default::default()).unwrap();
        let package_dir = temp_package_dir("harness-open");
        write_package(&las, &package_dir).unwrap();
        let service = PackageCommandService::new();

        let session = expect_ok(open_package_session_impl(
            &service,
            PackagePathRequest {
                path: package_dir.display().to_string(),
            },
        ));
        let window = expect_ok(read_curve_window_impl(
            &service,
            SessionWindowRequest {
                session_id: session.session_id.clone(),
                window: CurveWindowRequest {
                    curve_names: vec![String::from("DT")],
                    start_row: 0,
                    row_count: 2,
                },
            },
        ));

        assert_eq!(window.session.session_id, session.session_id);
        assert_eq!(window.window.columns[0].name, "DT");
    }

    #[test]
    fn harness_commands_preserve_metadata_edit_and_save() {
        let las = examples::open("sample.las", &Default::default()).unwrap();
        let package_dir = temp_package_dir("harness-save");
        write_package(&las, &package_dir).unwrap();
        let service = PackageCommandService::new();

        let session = expect_ok(open_package_session_impl(
            &service,
            PackagePathRequest {
                path: package_dir.display().to_string(),
            },
        ));
        expect_ok(apply_metadata_edit_impl(
            &service,
            SessionMetadataEditRequest {
                session_id: session.session_id.clone(),
                update: MetadataUpdateRequest {
                    items: vec![HeaderItemUpdate {
                        section: MetadataSectionDto::Well,
                        mnemonic: String::from("COMP"),
                        unit: String::new(),
                        value: lithos_las::LasValue::Text(String::from("HARNESS TEST")),
                        description: String::from("COMPANY"),
                    }],
                    other: None,
                },
            },
        ));
        expect_ok(save_session_impl(
            &service,
            SessionRequest {
                session_id: session.session_id.clone(),
            },
        ));

        let metadata = expect_ok(session_metadata_impl(
            &service,
            SessionRequest {
                session_id: session.session_id.clone(),
            },
        ));
        assert_eq!(
            metadata.metadata.metadata.well.company.as_deref(),
            Some("HARNESS TEST")
        );
    }

    fn expect_ok<T>(response: CommandResponse<T>) -> T {
        match response {
            CommandResponse::Ok(value) => value,
            CommandResponse::Err(error) => panic!("expected ok response, got {}", error.message),
        }
    }

    fn temp_package_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lithos-{prefix}-{unique}"));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        path
    }
}
