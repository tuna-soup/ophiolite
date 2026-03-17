mod commands;

use commands::{
    HarnessState, apply_curve_edit, apply_metadata_edit, close_session, create_project,
    dirty_state, import_las_into_workspace, import_project_drilling_csv, import_project_las,
    import_project_pressure_csv, import_project_tops_csv, import_project_trajectory_csv,
    inspect_las_curve_catalog, inspect_las_depth_window, inspect_las_metadata,
    inspect_las_summary, inspect_las_window, inspect_package_metadata, inspect_package_summary,
    list_project_asset_collections, list_project_assets, list_project_wellbores,
    list_project_wells, open_package_session, open_project, project_assets_covering_depth_range,
    read_curve_window, read_depth_window, read_package_files, read_project_drilling_observations,
    read_project_pressure_observations, read_project_tops, read_project_trajectory_rows,
    save_session, save_session_as, session_curve_catalog, session_metadata, session_summary,
    validate_las, validate_package,
};
use tauri::menu::{MenuBuilder, MenuEvent, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::Emitter;

const MENU_EVENT_NAME: &str = "menu-action";
const MENU_FILE_NEW_PROJECT: &str = "file.new-project";
const MENU_FILE_OPEN_PROJECT: &str = "file.open-project";
const MENU_FILE_IMPORT_ASSET: &str = "file.import-asset";
const MENU_FILE_SAVE: &str = "file.save";
const MENU_FILE_SAVE_AS: &str = "file.save-as";
const MENU_FILE_CLOSE_WORKSPACE: &str = "file.close-workspace";

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let new_package = MenuItemBuilder::with_id(MENU_FILE_NEW_PROJECT, "New Project")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_package = MenuItemBuilder::with_id(MENU_FILE_OPEN_PROJECT, "Open Project...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let import_las = MenuItemBuilder::with_id(MENU_FILE_IMPORT_ASSET, "Import Asset...")
        .accelerator("CmdOrCtrl+I")
        .build(app)?;
    let save = MenuItemBuilder::with_id(MENU_FILE_SAVE, "Save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let save_as = MenuItemBuilder::with_id(MENU_FILE_SAVE_AS, "Save As...")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;
    let close_workspace =
        MenuItemBuilder::with_id(MENU_FILE_CLOSE_WORKSPACE, "Close Workspace")
            .accelerator("CmdOrCtrl+W")
            .build(app)?;
    let quit = PredefinedMenuItem::quit(app, None)?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&new_package)
        .item(&open_package)
        .item(&import_las)
        .separator()
        .item(&save)
        .item(&save_as)
        .item(&close_workspace)
        .separator()
        .item(&quit)
        .build()?;

    MenuBuilder::new(app).item(&file_menu).build()
}

fn emit_menu_event(app: &tauri::AppHandle, event: MenuEvent) {
    let action = match event.id().as_ref() {
        MENU_FILE_NEW_PROJECT
        | MENU_FILE_OPEN_PROJECT
        | MENU_FILE_IMPORT_ASSET
        | MENU_FILE_SAVE
        | MENU_FILE_SAVE_AS
        | MENU_FILE_CLOSE_WORKSPACE => event.id().0.clone(),
        _ => return,
    };

    let _ = app.emit(MENU_EVENT_NAME, action);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(build_menu)
        .on_menu_event(emit_menu_event)
        .manage(HarnessState::default())
        .invoke_handler(tauri::generate_handler![
            inspect_package_summary,
            inspect_las_summary,
            inspect_las_metadata,
            inspect_las_curve_catalog,
            inspect_las_depth_window,
            inspect_las_window,
            validate_las,
            inspect_package_metadata,
            validate_package,
            open_package_session,
            session_summary,
            session_metadata,
            session_curve_catalog,
            read_curve_window,
            read_depth_window,
            dirty_state,
            close_session,
            apply_metadata_edit,
            apply_curve_edit,
            save_session,
            save_session_as,
            import_las_into_workspace,
            read_package_files,
            create_project,
            open_project,
            list_project_wells,
            list_project_wellbores,
            list_project_asset_collections,
            list_project_assets,
            import_project_las,
            import_project_trajectory_csv,
            import_project_tops_csv,
            import_project_pressure_csv,
            import_project_drilling_csv,
            project_assets_covering_depth_range,
            read_project_trajectory_rows,
            read_project_tops,
            read_project_pressure_observations,
            read_project_drilling_observations
        ])
        .run(tauri::generate_context!())
        .expect("error while running lithos harness");
}
