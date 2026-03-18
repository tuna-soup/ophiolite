use lithos_las::{
    AssetBindingInput, LithosProject, OpenStructuredAssetEditSessionRequest,
    StructuredAssetEditSessionStore, StructuredAssetSessionRequest, TopRowPatch,
    TopSetEditRequest, TrajectoryEditRequest, TrajectoryRowPatch,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn structured_edit_session_updates_and_saves_tops() {
    let root = temp_project_root("structured_edit_session_updates_and_saves_tops");
    let mut project = LithosProject::create(&root).unwrap();
    let csv_path = write_csv(
        &root,
        "tops.csv",
        "name,top_depth,base_depth,source,depth_reference\nTop A,100,101,interp,MD\n",
    );
    let binding = AssetBindingInput {
        well_name: "Well A".to_string(),
        wellbore_name: "WB-1".to_string(),
        uwi: Some("UWI-1".to_string()),
        api: None,
        operator_aliases: Vec::new(),
    };
    let imported = project.import_tops_csv(&csv_path, &binding, Some("tops")).unwrap();

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
                    base_depth: Some(lithos_las::OptionalFieldPatch { set: Some(111.0), clear: false }),
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
    let reopened = LithosProject::open(&root).unwrap();
    let rows = reopened.read_tops(&imported.asset.id).unwrap();
    assert_eq!(rows[0].name, "Top B");
    assert_eq!(rows[0].top_depth, 110.0);
}

#[test]
fn structured_edit_session_rejects_invalid_trajectory_save_without_touching_disk() {
    let root = temp_project_root("structured_edit_session_rejects_invalid_trajectory_save");
    let mut project = LithosProject::create(&root).unwrap();
    let csv_path = write_csv(
        &root,
        "trajectory.csv",
        "measured_depth,true_vertical_depth\n100,90\n110,99\n",
    );
    let binding = AssetBindingInput {
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
    assert!(error
        .to_string()
        .contains("trajectory rows must be monotonic"));

    let session = store
        .session_summary(&StructuredAssetSessionRequest {
            session_id: summary.session_id.clone(),
        })
        .unwrap();
    assert!(session.dirty);

    let reopened = LithosProject::open(&root).unwrap();
    let rows = reopened.read_trajectory_rows(&imported.asset.id, None).unwrap();
    assert_eq!(rows[1].measured_depth, 110.0);
}

fn temp_project_root(label: &str) -> PathBuf {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("lithos_{label}_{nanos}_{unique}"));
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
