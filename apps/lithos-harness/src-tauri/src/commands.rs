use lithos_las::{
    AssetSummaryDto, CloseSessionResultDto, CommandErrorKind, CommandGroup, CommandResponse,
    CurveCatalogDto, CurveStorageKind, CurveWindowDto, DepthRangeQuery, DepthWindowRequest,
    DirtyStateDto, LithosProject, MetadataDto, PackageCommandService, PackagePathRequest,
    PackageStorageMetadata, RawLasWindowRequest, SavePackageResultDto, SessionCurveEditRequest,
    SessionDepthWindowRequest, SessionId, SessionMetadataDto, SessionMetadataEditRequest,
    SessionRequest, SessionSaveAsRequest, SessionSummaryDto, SessionWindowDto,
    SessionWindowRequest, TopRow, TrajectoryRow, ValidationReportDto, WellRecord, WellboreRecord,
    asset_summary_dto, command_error_dto, curve_catalog_dto, curve_depth_window_dto,
    curve_window_dto, metadata_dto, parse_package_metadata, read_path, write_package_overwrite,
    validation_report_dto,
};
use lithos_las::{
    AssetBindingInput, AssetCollectionRecord, AssetKind, AssetRecord, DrillingObservationRow,
    PressureObservationRow,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummaryDto {
    pub root: String,
    pub catalog_path: String,
    pub manifest_path: String,
    pub well_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWellRequest {
    pub project_root: String,
    pub well_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWellboreRequest {
    pub project_root: String,
    pub wellbore_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssetsRequest {
    pub project_root: String,
    pub wellbore_id: String,
    pub asset_kind: Option<AssetKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssetRequest {
    pub project_root: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDepthCoverageRequest {
    pub project_root: String,
    pub wellbore_id: String,
    pub depth_min: f64,
    pub depth_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDepthReadRequest {
    pub project_root: String,
    pub asset_id: String,
    pub depth_min: Option<f64>,
    pub depth_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStructuredAssetRequest {
    pub project_root: String,
    pub csv_path: String,
    pub binding: AssetBindingInput,
    pub collection_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProjectLasRequest {
    pub project_root: String,
    pub las_path: String,
    pub collection_name: Option<String>,
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

fn create_project_impl(request: ProjectPathRequest) -> CommandResponse<ProjectSummaryDto> {
    match LithosProject::create(&request.path) {
        Ok(project) => project_summary_response(project),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::EditPersist,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn open_project_impl(request: ProjectPathRequest) -> CommandResponse<ProjectSummaryDto> {
    match LithosProject::open(&request.path) {
        Ok(project) => project_summary_response(project),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::Inspect,
            CommandErrorKind::OpenFailed,
            error.to_string(),
        )),
    }
}

fn list_project_wells_impl(request: ProjectPathRequest) -> CommandResponse<Vec<WellRecord>> {
    match LithosProject::open(&request.path).and_then(|project| project.list_wells()) {
        Ok(wells) => CommandResponse::Ok(wells),
        Err(error) => project_read_error(error),
    }
}

fn list_project_wellbores_impl(
    request: ProjectWellRequest,
) -> CommandResponse<Vec<WellboreRecord>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        project.list_wellbores(&lithos_las::WellId(request.well_id))
    }) {
        Ok(wellbores) => CommandResponse::Ok(wellbores),
        Err(error) => project_read_error(error),
    }
}

fn list_project_asset_collections_impl(
    request: ProjectWellboreRequest,
) -> CommandResponse<Vec<AssetCollectionRecord>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        project.list_asset_collections(&lithos_las::WellboreId(request.wellbore_id))
    }) {
        Ok(collections) => CommandResponse::Ok(collections),
        Err(error) => project_read_error(error),
    }
}

fn list_project_assets_impl(request: ProjectAssetsRequest) -> CommandResponse<Vec<AssetRecord>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        project.list_assets(
            &lithos_las::WellboreId(request.wellbore_id),
            request.asset_kind,
        )
    }) {
        Ok(assets) => CommandResponse::Ok(assets),
        Err(error) => project_read_error(error),
    }
}

fn import_project_las_impl(
    request: ImportProjectLasRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    match LithosProject::open(&request.project_root).and_then(|mut project| {
        project.import_las(&request.las_path, request.collection_name.as_deref())
    }) {
        Ok(result) => CommandResponse::Ok(result),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::EditPersist,
            CommandErrorKind::ValidationFailed,
            error.to_string(),
        )),
    }
}

fn import_project_trajectory_csv_impl(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_structured_asset(
        request,
        |project, csv_path, binding, collection_name| {
            project.import_trajectory_csv(csv_path, binding, collection_name)
        },
    )
}

fn import_project_tops_csv_impl(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_structured_asset(
        request,
        |project, csv_path, binding, collection_name| {
            project.import_tops_csv(csv_path, binding, collection_name)
        },
    )
}

fn import_project_pressure_csv_impl(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_structured_asset(
        request,
        |project, csv_path, binding, collection_name| {
            project.import_pressure_csv(csv_path, binding, collection_name)
        },
    )
}

fn import_project_drilling_csv_impl(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_structured_asset(
        request,
        |project, csv_path, binding, collection_name| {
            project.import_drilling_csv(csv_path, binding, collection_name)
        },
    )
}

fn project_assets_covering_depth_range_impl(
    request: ProjectDepthCoverageRequest,
) -> CommandResponse<Vec<AssetRecord>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        project.assets_covering_depth_range(
            &lithos_las::WellboreId(request.wellbore_id),
            request.depth_min,
            request.depth_max,
        )
    }) {
        Ok(assets) => CommandResponse::Ok(assets),
        Err(error) => project_read_error(error),
    }
}

fn read_project_trajectory_rows_impl(
    request: ProjectDepthReadRequest,
) -> CommandResponse<Vec<TrajectoryRow>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        let query = depth_query(&request);
        project.read_trajectory_rows(&lithos_las::AssetId(request.asset_id), query.as_ref())
    }) {
        Ok(rows) => CommandResponse::Ok(rows),
        Err(error) => project_read_error(error),
    }
}

fn read_project_tops_impl(request: ProjectAssetRequest) -> CommandResponse<Vec<TopRow>> {
    match LithosProject::open(&request.project_root)
        .and_then(|project| project.read_tops(&lithos_las::AssetId(request.asset_id)))
    {
        Ok(rows) => CommandResponse::Ok(rows),
        Err(error) => project_read_error(error),
    }
}

fn read_project_pressure_observations_impl(
    request: ProjectDepthReadRequest,
) -> CommandResponse<Vec<PressureObservationRow>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        let query = depth_query(&request);
        project.read_pressure_observations(
            &lithos_las::AssetId(request.asset_id),
            query.as_ref(),
        )
    }) {
        Ok(rows) => CommandResponse::Ok(rows),
        Err(error) => project_read_error(error),
    }
}

fn read_project_drilling_observations_impl(
    request: ProjectDepthReadRequest,
) -> CommandResponse<Vec<DrillingObservationRow>> {
    match LithosProject::open(&request.project_root).and_then(|project| {
        let query = depth_query(&request);
        project.read_drilling_observations(
            &lithos_las::AssetId(request.asset_id),
            query.as_ref(),
        )
    }) {
        Ok(rows) => CommandResponse::Ok(rows),
        Err(error) => project_read_error(error),
    }
}

fn import_structured_asset<F>(
    request: ImportStructuredAssetRequest,
    importer: F,
) -> CommandResponse<lithos_las::ProjectAssetImportResult>
where
    F: FnOnce(
        &mut LithosProject,
        &str,
        &AssetBindingInput,
        Option<&str>,
    ) -> lithos_las::Result<lithos_las::ProjectAssetImportResult>,
{
    match LithosProject::open(&request.project_root).and_then(|mut project| {
        importer(
            &mut project,
            &request.csv_path,
            &request.binding,
            request.collection_name.as_deref(),
        )
    }) {
        Ok(result) => CommandResponse::Ok(result),
        Err(error) => CommandResponse::Err(command_error_dto(
            CommandGroup::EditPersist,
            CommandErrorKind::ValidationFailed,
            error.to_string(),
        )),
    }
}

fn project_summary_response(project: LithosProject) -> CommandResponse<ProjectSummaryDto> {
    match project.summary() {
        Ok(summary) => CommandResponse::Ok(ProjectSummaryDto {
            root: summary.root,
            catalog_path: summary.catalog_path,
            manifest_path: summary.manifest_path,
            well_count: summary.well_count,
            asset_count: summary.asset_count,
        }),
        Err(error) => project_read_error(error),
    }
}

fn project_read_error<T>(error: lithos_las::LasError) -> CommandResponse<T> {
    CommandResponse::Err(command_error_dto(
        CommandGroup::Inspect,
        CommandErrorKind::OpenFailed,
        error.to_string(),
    ))
}

fn depth_query(request: &ProjectDepthReadRequest) -> Option<DepthRangeQuery> {
    if request.depth_min.is_none() && request.depth_max.is_none() {
        None
    } else {
        Some(DepthRangeQuery {
            depth_min: request.depth_min,
            depth_max: request.depth_max,
        })
    }
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

#[tauri::command]
pub fn create_project(request: ProjectPathRequest) -> CommandResponse<ProjectSummaryDto> {
    create_project_impl(request)
}

#[tauri::command]
pub fn open_project(request: ProjectPathRequest) -> CommandResponse<ProjectSummaryDto> {
    open_project_impl(request)
}

#[tauri::command]
pub fn list_project_wells(request: ProjectPathRequest) -> CommandResponse<Vec<WellRecord>> {
    list_project_wells_impl(request)
}

#[tauri::command]
pub fn list_project_wellbores(
    request: ProjectWellRequest,
) -> CommandResponse<Vec<WellboreRecord>> {
    list_project_wellbores_impl(request)
}

#[tauri::command]
pub fn list_project_asset_collections(
    request: ProjectWellboreRequest,
) -> CommandResponse<Vec<AssetCollectionRecord>> {
    list_project_asset_collections_impl(request)
}

#[tauri::command]
pub fn list_project_assets(request: ProjectAssetsRequest) -> CommandResponse<Vec<AssetRecord>> {
    list_project_assets_impl(request)
}

#[tauri::command]
pub fn import_project_las(
    request: ImportProjectLasRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_project_las_impl(request)
}

#[tauri::command]
pub fn import_project_trajectory_csv(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_project_trajectory_csv_impl(request)
}

#[tauri::command]
pub fn import_project_tops_csv(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_project_tops_csv_impl(request)
}

#[tauri::command]
pub fn import_project_pressure_csv(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_project_pressure_csv_impl(request)
}

#[tauri::command]
pub fn import_project_drilling_csv(
    request: ImportStructuredAssetRequest,
) -> CommandResponse<lithos_las::ProjectAssetImportResult> {
    import_project_drilling_csv_impl(request)
}

#[tauri::command]
pub fn project_assets_covering_depth_range(
    request: ProjectDepthCoverageRequest,
) -> CommandResponse<Vec<AssetRecord>> {
    project_assets_covering_depth_range_impl(request)
}

#[tauri::command]
pub fn read_project_trajectory_rows(
    request: ProjectDepthReadRequest,
) -> CommandResponse<Vec<TrajectoryRow>> {
    read_project_trajectory_rows_impl(request)
}

#[tauri::command]
pub fn read_project_tops(request: ProjectAssetRequest) -> CommandResponse<Vec<TopRow>> {
    read_project_tops_impl(request)
}

#[tauri::command]
pub fn read_project_pressure_observations(
    request: ProjectDepthReadRequest,
) -> CommandResponse<Vec<PressureObservationRow>> {
    read_project_pressure_observations_impl(request)
}

#[tauri::command]
pub fn read_project_drilling_observations(
    request: ProjectDepthReadRequest,
) -> CommandResponse<Vec<DrillingObservationRow>> {
    read_project_drilling_observations_impl(request)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_metadata_edit_impl, create_project_impl, import_las_into_workspace_impl,
        import_project_drilling_csv_impl, import_project_pressure_csv_impl,
        import_project_tops_csv_impl, import_project_trajectory_csv_impl,
        inspect_las_summary_impl, inspect_las_window_impl, list_project_asset_collections_impl,
        list_project_assets_impl, list_project_wellbores_impl, list_project_wells_impl,
        open_package_session_impl, open_project_impl, project_assets_covering_depth_range_impl,
        read_curve_window_impl, read_depth_window_impl, read_package_files_impl,
        read_project_drilling_observations_impl, read_project_pressure_observations_impl,
        read_project_tops_impl, read_project_trajectory_rows_impl, save_session_impl,
        session_metadata_impl,
    };
    use lithos_las::{
        AssetBindingInput, AssetKind,
        CommandResponse, CurveWindowRequest, DepthWindowRequest, HeaderItemUpdate,
        MetadataSectionDto, MetadataUpdateRequest, PackageCommandService, PackagePathRequest,
        RawLasWindowRequest, SessionDepthWindowRequest, SessionMetadataEditRequest,
        SessionRequest, SessionWindowRequest, examples, write_package,
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

    #[test]
    fn project_commands_create_open_and_list_empty_project() {
        let project_dir = temp_package_dir("harness-project");

        let created = expect_ok(create_project_impl(super::ProjectPathRequest {
            path: project_dir.display().to_string(),
        }));
        let opened = expect_ok(open_project_impl(super::ProjectPathRequest {
            path: project_dir.display().to_string(),
        }));
        let wells = expect_ok(list_project_wells_impl(super::ProjectPathRequest {
            path: project_dir.display().to_string(),
        }));

        assert_eq!(created.root, project_dir.display().to_string());
        assert_eq!(opened.root, created.root);
        assert!(created.catalog_path.ends_with("catalog.sqlite"));
        assert!(wells.is_empty());
    }

    #[test]
    fn project_commands_import_non_log_assets_and_read_them() {
        let project_dir = temp_package_dir("harness-project-assets");
        fs::create_dir_all(&project_dir).unwrap();
        let project_root = project_dir.display().to_string();
        expect_ok(create_project_impl(super::ProjectPathRequest {
            path: project_root.clone(),
        }));

        let binding = AssetBindingInput {
            well_name: String::from("Well Alpha"),
            wellbore_name: String::from("Well Alpha WB1"),
            uwi: Some(String::from("UWI-001")),
            api: None,
            operator_aliases: vec![String::from("Lithos Energy")],
        };

        let trajectory_csv = temp_csv(
            "trajectory",
            "measured_depth,true_vertical_depth,azimuth_deg,inclination_deg\n1000,900,10,2\n1010,909,11,2.1\n",
        );
        let tops_csv = temp_csv(
            "tops",
            "name,top_depth,base_depth,source,depth_reference\nTOP_A,995,1005,interp,MD\n",
        );
        let pressure_csv = temp_csv(
            "pressure",
            "measured_depth,pressure,phase,test_kind,timestamp\n1002,4500,oil,mdt,2026-01-01T00:00:00Z\n",
        );
        let drilling_csv = temp_csv(
            "drilling",
            "measured_depth,event_kind,value,unit,timestamp,comment\n1004,ROP,55,ft/hr,2026-01-02T00:00:00Z,steady\n",
        );

        let trajectory = expect_ok(import_project_trajectory_csv_impl(
            super::ImportStructuredAssetRequest {
                project_root: project_root.clone(),
                csv_path: trajectory_csv.display().to_string(),
                binding: binding.clone(),
                collection_name: Some(String::from("Survey 1")),
            },
        ));
        let tops = expect_ok(import_project_tops_csv_impl(super::ImportStructuredAssetRequest {
            project_root: project_root.clone(),
            csv_path: tops_csv.display().to_string(),
            binding: binding.clone(),
            collection_name: Some(String::from("Interp Tops")),
        }));
        let pressure =
            expect_ok(import_project_pressure_csv_impl(super::ImportStructuredAssetRequest {
                project_root: project_root.clone(),
                csv_path: pressure_csv.display().to_string(),
                binding: binding.clone(),
                collection_name: Some(String::from("Pressure 1")),
            }));
        let drilling =
            expect_ok(import_project_drilling_csv_impl(super::ImportStructuredAssetRequest {
                project_root: project_root.clone(),
                csv_path: drilling_csv.display().to_string(),
                binding,
                collection_name: Some(String::from("Drill Obs 1")),
            }));

        let wells = expect_ok(list_project_wells_impl(super::ProjectPathRequest {
            path: project_root.clone(),
        }));
        assert_eq!(wells.len(), 1);

        let wellbores = expect_ok(list_project_wellbores_impl(super::ProjectWellRequest {
            project_root: project_root.clone(),
            well_id: wells[0].id.0.clone(),
        }));
        assert_eq!(wellbores.len(), 1);

        let collections =
            expect_ok(list_project_asset_collections_impl(super::ProjectWellboreRequest {
                project_root: project_root.clone(),
                wellbore_id: wellbores[0].id.0.clone(),
            }));
        assert_eq!(collections.len(), 4);

        let assets = expect_ok(list_project_assets_impl(super::ProjectAssetsRequest {
            project_root: project_root.clone(),
            wellbore_id: wellbores[0].id.0.clone(),
            asset_kind: None,
        }));
        assert_eq!(assets.len(), 4);

        let depth_assets =
            expect_ok(project_assets_covering_depth_range_impl(
                super::ProjectDepthCoverageRequest {
                    project_root: project_root.clone(),
                    wellbore_id: wellbores[0].id.0.clone(),
                    depth_min: 994.0,
                    depth_max: 1004.0,
                },
            ));
        assert_eq!(depth_assets.len(), 4);

        let trajectory_rows = expect_ok(read_project_trajectory_rows_impl(
            super::ProjectDepthReadRequest {
                project_root: project_root.clone(),
                asset_id: trajectory.asset.id.0.clone(),
                depth_min: Some(1005.0),
                depth_max: Some(1015.0),
            },
        ));
        let top_rows = expect_ok(read_project_tops_impl(super::ProjectAssetRequest {
            project_root: project_root.clone(),
            asset_id: tops.asset.id.0.clone(),
        }));
        let pressure_rows = expect_ok(read_project_pressure_observations_impl(
            super::ProjectDepthReadRequest {
                project_root: project_root.clone(),
                asset_id: pressure.asset.id.0.clone(),
                depth_min: Some(1000.0),
                depth_max: Some(1003.0),
            },
        ));
        let drilling_rows = expect_ok(read_project_drilling_observations_impl(
            super::ProjectDepthReadRequest {
                project_root: project_root.clone(),
                asset_id: drilling.asset.id.0.clone(),
                depth_min: Some(1000.0),
                depth_max: Some(1005.0),
            },
        ));

        assert_eq!(trajectory.collection.asset_kind, AssetKind::Trajectory);
        assert_eq!(trajectory_rows.len(), 1);
        assert_eq!(top_rows.len(), 1);
        assert_eq!(pressure_rows.len(), 1);
        assert_eq!(drilling_rows.len(), 1);
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

    fn temp_csv(prefix: &str, contents: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lithos-{prefix}-{unique}.csv"));
        fs::write(&path, contents).unwrap();
        path
    }
}
