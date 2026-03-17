use lithos_las::{
    AssetSummaryDto, CloseSessionResultDto, CommandErrorKind, CommandGroup, CommandResponse,
    CurveCatalogDto, CurveStorageKind, CurveWindowDto, DepthWindowRequest, DirtyStateDto,
    MetadataDto, PackageCommandService, PackagePathRequest, PackageStorageMetadata,
    RawLasWindowRequest, SavePackageResultDto, SessionCurveEditRequest, SessionDepthWindowRequest,
    SessionId, SessionMetadataDto, SessionMetadataEditRequest, SessionRequest,
    SessionSaveAsRequest, SessionSummaryDto, SessionWindowDto, SessionWindowRequest,
    ValidationReportDto, asset_summary_dto, command_error_dto, curve_catalog_dto,
    curve_depth_window_dto, curve_window_dto, metadata_dto, parse_package_metadata, read_path,
    write_package_overwrite, validation_report_dto,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Clone, Default)]
pub struct HarnessState {
    service: PackageCommandService,
}

impl HarnessState {
    fn service(&self) -> &PackageCommandService {
        &self.service
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportLasRequest {
    pub package_root: String,
    pub las_path: String,
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFilesViewDto {
    pub root: String,
    pub has_package_files: bool,
    pub metadata_path: String,
    pub metadata_json: Option<String>,
    pub parquet_path: String,
    pub parquet_exists: bool,
    pub parquet_size_bytes: Option<u64>,
    pub row_count: Option<usize>,
    pub curve_count: usize,
    pub index_name: Option<String>,
    pub columns: Vec<PackageFileColumnDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFileColumnDto {
    pub name: String,
    pub canonical_name: String,
    pub original_mnemonic: String,
    pub unit: String,
    pub storage_kind: CurveStorageKind,
    pub row_count: usize,
    pub nullable: bool,
    pub is_index: bool,
}

fn inspect_package_summary_impl(
    service: &PackageCommandService,
    request: PackagePathRequest,
) -> CommandResponse<AssetSummaryDto> {
    service.inspect_package_summary(&request)
}

fn inspect_las_summary_impl(request: PackagePathRequest) -> CommandResponse<AssetSummaryDto> {
    match read_path(&request.path, &Default::default()) {
        Ok(file) => CommandResponse::Ok(asset_summary_dto(&file)),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn inspect_las_metadata_impl(request: PackagePathRequest) -> CommandResponse<MetadataDto> {
    match read_path(&request.path, &Default::default()) {
        Ok(file) => CommandResponse::Ok(metadata_dto(&file)),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn inspect_las_curve_catalog_impl(
    request: PackagePathRequest,
) -> CommandResponse<Vec<lithos_las::CurveCatalogEntryDto>> {
    match read_path(&request.path, &Default::default()) {
        Ok(file) => CommandResponse::Ok(curve_catalog_dto(&file)),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn inspect_las_window_impl(request: RawLasWindowRequest) -> CommandResponse<CurveWindowDto> {
    match read_path(&request.path, &Default::default()) {
        Ok(file) => match curve_window_dto(&file, &request.window) {
            Ok(window) => CommandResponse::Ok(window),
            Err(error) => CommandResponse::Err(command_error_dto(
                CommandGroup::Inspect,
                CommandErrorKind::ValidationFailed,
                error.to_string(),
            )),
        },
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn inspect_las_depth_window_impl(
    request: PackagePathRequest,
    window: DepthWindowRequest,
) -> CommandResponse<CurveWindowDto> {
    match read_path(&request.path, &Default::default()) {
        Ok(file) => match curve_depth_window_dto(&file, &window) {
            Ok(result) => CommandResponse::Ok(result),
            Err(error) => CommandResponse::Err(command_error_dto(
                CommandGroup::Inspect,
                CommandErrorKind::ValidationFailed,
                error.to_string(),
            )),
        },
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn validate_las_impl(request: PackagePathRequest) -> CommandResponse<ValidationReportDto> {
    match read_path(&request.path, &Default::default()) {
        Ok(file) => CommandResponse::Ok(validation_report_dto(&file)),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
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

fn read_depth_window_impl(
    service: &PackageCommandService,
    request: SessionDepthWindowRequest,
) -> CommandResponse<SessionWindowDto> {
    service.read_depth_window(&request)
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

fn import_las_into_workspace_impl(
    service: &PackageCommandService,
    request: ImportLasRequest,
) -> CommandResponse<SessionSummaryDto> {
    let las = match read_path(&request.las_path, &Default::default()) {
        Ok(file) => file,
        Err(error) => {
            return CommandResponse::Err(command_error_dto(
                CommandGroup::EditPersist,
                CommandErrorKind::OpenFailed,
                error.to_string(),
            ));
        }
    };

    if let Some(session_id) = &request.session_id {
        let _ = service.close_session(&SessionRequest {
            session_id: session_id.clone(),
        });
    }

    if let Err(error) = write_package_overwrite(&las, &request.package_root) {
        return CommandResponse::Err(command_error_dto(
            CommandGroup::EditPersist,
            CommandErrorKind::ValidationFailed,
            error.to_string(),
        ));
    }

    service.open_package_session(&PackagePathRequest {
        path: request.package_root,
    })
}

fn read_package_files_impl(request: PackagePathRequest) -> CommandResponse<PackageFilesViewDto> {
    let root = Path::new(&request.path);
    let metadata_path = root.join("metadata.json");
    let parquet_path = root.join("curves.parquet");
    let parquet_meta = fs::metadata(&parquet_path).ok();

    if !metadata_path.exists() {
        return CommandResponse::Ok(PackageFilesViewDto {
            root: request.path,
            has_package_files: false,
            metadata_path: metadata_path.display().to_string(),
            metadata_json: None,
            parquet_path: parquet_path.display().to_string(),
            parquet_exists: parquet_path.exists(),
            parquet_size_bytes: parquet_meta.map(|value| value.len()),
            row_count: None,
            curve_count: 0,
            index_name: None,
            columns: Vec::new(),
        });
    }

    let raw_json = match fs::read_to_string(&metadata_path) {
        Ok(contents) => contents,
        Err(error) => {
            return CommandResponse::Err(command_error_dto(
                CommandGroup::Inspect,
                CommandErrorKind::OpenFailed,
                error.to_string(),
            ));
        }
    };

    let pretty_json = match serde_json::from_str::<Value>(&raw_json) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or(raw_json.clone()),
        Err(_) => raw_json.clone(),
    };

    let package_metadata = match parse_package_metadata(&raw_json) {
        Ok(metadata) => metadata,
        Err(error) => {
            return CommandResponse::Err(command_error_dto(
                CommandGroup::Inspect,
                CommandErrorKind::OpenFailed,
                error.to_string(),
            ));
        }
    };

    CommandResponse::Ok(PackageFilesViewDto {
        root: request.path,
        has_package_files: true,
        metadata_path: metadata_path.display().to_string(),
        metadata_json: Some(pretty_json),
        parquet_path: parquet_path.display().to_string(),
        parquet_exists: parquet_path.exists(),
        parquet_size_bytes: parquet_meta.map(|value| value.len()),
        row_count: Some(package_metadata.document.summary.row_count),
        curve_count: package_metadata.storage.curve_columns.len(),
        index_name: Some(package_metadata.storage.index.curve_id.clone()),
        columns: package_file_columns(&package_metadata.storage),
    })
}

fn package_file_columns(storage: &PackageStorageMetadata) -> Vec<PackageFileColumnDto> {
    storage
        .curve_columns
        .iter()
        .map(|column| PackageFileColumnDto {
            name: column.name.clone(),
            canonical_name: column.canonical_name.clone(),
            original_mnemonic: column.original_mnemonic.clone(),
            unit: column.unit.clone(),
            storage_kind: column.storage_kind,
            row_count: column.row_count,
            nullable: column.nullable,
            is_index: column.is_index,
        })
        .collect()
}

#[tauri::command]
pub fn inspect_package_summary(
    state: tauri::State<HarnessState>,
    request: PackagePathRequest,
) -> CommandResponse<AssetSummaryDto> {
    inspect_package_summary_impl(state.service(), request)
}

#[tauri::command]
pub fn inspect_las_summary(request: PackagePathRequest) -> CommandResponse<AssetSummaryDto> {
    inspect_las_summary_impl(request)
}

#[tauri::command]
pub fn inspect_las_metadata(request: PackagePathRequest) -> CommandResponse<MetadataDto> {
    inspect_las_metadata_impl(request)
}

#[tauri::command]
pub fn inspect_las_curve_catalog(
    request: PackagePathRequest,
) -> CommandResponse<Vec<lithos_las::CurveCatalogEntryDto>> {
    inspect_las_curve_catalog_impl(request)
}

#[tauri::command]
pub fn inspect_las_window(request: RawLasWindowRequest) -> CommandResponse<CurveWindowDto> {
    inspect_las_window_impl(request)
}

#[tauri::command]
pub fn inspect_las_depth_window(
    path: PackagePathRequest,
    window: DepthWindowRequest,
) -> CommandResponse<CurveWindowDto> {
    inspect_las_depth_window_impl(path, window)
}

#[tauri::command]
pub fn validate_las(request: PackagePathRequest) -> CommandResponse<ValidationReportDto> {
    validate_las_impl(request)
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
pub fn read_depth_window(
    state: tauri::State<HarnessState>,
    request: SessionDepthWindowRequest,
) -> CommandResponse<SessionWindowDto> {
    read_depth_window_impl(state.service(), request)
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

#[tauri::command]
pub fn import_las_into_workspace(
    state: tauri::State<HarnessState>,
    request: ImportLasRequest,
) -> CommandResponse<SessionSummaryDto> {
    import_las_into_workspace_impl(state.service(), request)
}

#[tauri::command]
pub fn read_package_files(request: PackagePathRequest) -> CommandResponse<PackageFilesViewDto> {
    read_package_files_impl(request)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_metadata_edit_impl, import_las_into_workspace_impl, inspect_las_summary_impl,
        inspect_las_window_impl, open_package_session_impl, read_curve_window_impl,
        read_depth_window_impl, read_package_files_impl, save_session_impl, session_metadata_impl,
    };
    use lithos_las::{
        CommandResponse, CurveWindowRequest, DepthWindowRequest, HeaderItemUpdate,
        MetadataSectionDto, MetadataUpdateRequest, PackageCommandService, PackagePathRequest,
        RawLasWindowRequest, SessionDepthWindowRequest, SessionMetadataEditRequest, SessionRequest,
        SessionWindowRequest, examples, write_package,
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
    fn harness_commands_open_session_and_query_depth_window() {
        let las = examples::open("sample.las", &Default::default()).unwrap();
        let package_dir = temp_package_dir("harness-depth");
        write_package(&las, &package_dir).unwrap();
        let service = PackageCommandService::new();

        let session = expect_ok(open_package_session_impl(
            &service,
            PackagePathRequest {
                path: package_dir.display().to_string(),
            },
        ));
        let window = expect_ok(read_depth_window_impl(
            &service,
            SessionDepthWindowRequest {
                session_id: session.session_id.clone(),
                window: DepthWindowRequest {
                    curve_names: vec![String::from("DEPT"), String::from("DT")],
                    depth_min: 1669.875,
                    depth_max: 1670.0,
                },
            },
        ));

        assert_eq!(window.session.session_id, session.session_id);
        assert_eq!(window.window.columns[0].name, "DEPT");
        assert_eq!(window.window.row_count, 2);
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

    #[test]
    fn harness_commands_inspect_raw_las_files() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test_data")
            .join("logs")
            .join("6038187_v1.2_short.las");
        let path = fixture_path.display().to_string();

        let summary = expect_ok(inspect_las_summary_impl(PackagePathRequest {
            path: path.clone(),
        }));
        let window = expect_ok(inspect_las_window_impl(RawLasWindowRequest {
            path,
            window: CurveWindowRequest {
                curve_names: vec![String::from("CALI")],
                start_row: 0,
                row_count: 2,
            },
        }));

        assert!(summary.summary.curve_count >= 1);
        assert_eq!(window.columns[0].name, "CALI");
    }

    #[test]
    fn harness_commands_import_into_workspace_and_read_package_files() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test_data")
            .join("logs")
            .join("6038187_v1.2_short.las");
        let package_dir = temp_package_dir("harness-import");
        fs::create_dir_all(&package_dir).unwrap();
        let service = PackageCommandService::new();

        let session = expect_ok(import_las_into_workspace_impl(
            &service,
            super::ImportLasRequest {
                package_root: package_dir.display().to_string(),
                las_path: fixture_path.display().to_string(),
                session_id: None,
            },
        ));

        let files = expect_ok(read_package_files_impl(PackagePathRequest {
            path: package_dir.display().to_string(),
        }));

        assert_eq!(session.root, package_dir.display().to_string());
        assert!(files.has_package_files);
        assert!(files.metadata_json.as_ref().unwrap().contains("\"canonical\""));
        assert!(files.parquet_exists);
        assert!(files.curve_count >= 1);
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
