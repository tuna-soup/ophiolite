mod commands;

use commands::{
    HarnessState, apply_curve_edit, apply_metadata_edit, close_session, dirty_state,
    inspect_package_metadata, inspect_package_summary, open_package_session, read_curve_window,
    save_session, save_session_as, session_curve_catalog, session_metadata, session_summary,
    validate_package,
};

pub fn run() {
    tauri::Builder::default()
        .manage(HarnessState::default())
        .invoke_handler(tauri::generate_handler![
            inspect_package_summary,
            inspect_package_metadata,
            validate_package,
            open_package_session,
            session_summary,
            session_metadata,
            session_curve_catalog,
            read_curve_window,
            dirty_state,
            close_session,
            apply_metadata_edit,
            apply_curve_edit,
            save_session,
            save_session_as
        ])
        .run(tauri::generate_context!())
        .expect("error while running lithos harness");
}
