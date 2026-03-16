use lithos_las::{
    CurveEditRequest, CurveUpdateRequest, CurveWindowRequest, HeaderItemUpdate, LasError, LasValue,
    MetadataSectionDto, MetadataUpdateRequest, PackageBackend, SaveSessionResponseDto, examples,
    open_package,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn backend_supports_read_inspection_and_shared_session_queries() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-shared");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let summary = backend.inspect_package_summary(&package_dir).unwrap();
    let metadata = backend.inspect_package_metadata(&package_dir).unwrap();
    let first = backend.open_package_session(&package_dir).unwrap();
    let second = backend.open_package_session(&package_dir).unwrap();

    assert_eq!(summary.summary.las_version, "1.2");
    assert_eq!(
        metadata.metadata.well.well.as_deref(),
        Some("ANY ET AL OIL WELL #12")
    );
    assert_eq!(first.session_id, second.session_id);

    let catalog = backend.session_curve_catalog(&first.session_id).unwrap();
    let window = backend
        .read_curve_window(
            &first.session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT"), String::from("RHOB")],
                start_row: 0,
                row_count: 2,
            },
        )
        .unwrap();

    assert_eq!(catalog.curves.len(), 8);
    assert_eq!(catalog.session.session_id, first.session_id);
    assert_eq!(window.window.columns.len(), 2);
    assert_eq!(window.session.session_id, first.session_id);
    assert_eq!(window.window.columns[0].name, "DT");
}

#[test]
fn backend_rejected_curve_edits_are_atomic() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-atomic");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let before = backend
        .read_curve_window(
            &session.session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 3,
            },
        )
        .unwrap();

    let err = backend
        .apply_curve_edit(
            &session.session_id,
            &CurveEditRequest::Upsert(CurveUpdateRequest {
                mnemonic: String::from("DT"),
                original_mnemonic: Some(String::from("DT")),
                unit: String::from("US/M"),
                header_value: LasValue::Empty,
                description: String::from("invalid"),
                data: vec![LasValue::Number(1.0)],
            }),
        )
        .unwrap_err();

    match err {
        LasError::Validation(message) => assert!(message.contains("expects 3")),
        other => panic!("expected validation error, got {other}"),
    }

    let after = backend
        .read_curve_window(
            &session.session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 3,
            },
        )
        .unwrap();

    assert_eq!(
        before.window.columns[0].values,
        after.window.columns[0].values
    );
    assert!(
        !backend
            .dirty_state(&session.session_id)
            .unwrap()
            .has_unsaved_changes
    );
}

#[test]
fn backend_save_flows_preserve_session_identity_and_rebind_save_as() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-save");
    let copy_dir = temp_package_dir("backend-save-copy");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let session_id = session.session_id.clone();
    assert_eq!(session.root, package_dir.display().to_string());

    let edited = backend
        .apply_metadata_edit(
            &session_id,
            &MetadataUpdateRequest {
                items: vec![HeaderItemUpdate {
                    section: MetadataSectionDto::Well,
                    mnemonic: String::from("COMP"),
                    unit: String::new(),
                    value: LasValue::Text(String::from("BACKEND EDIT")),
                    description: String::from("COMPANY"),
                }],
                other: None,
            },
        )
        .unwrap();
    assert!(edited.dirty.has_unsaved_changes);

    let saved = backend.save_session(&session_id).unwrap();
    let save_result = match saved {
        SaveSessionResponseDto::Saved(result) => result,
        SaveSessionResponseDto::Conflict(conflict) => {
            panic!("unexpected save conflict: {}", conflict.actual_revision.0)
        }
    };
    assert_eq!(save_result.session_id, session_id);
    assert_eq!(save_result.root, package_dir.display().to_string());
    assert!(
        !backend
            .dirty_state(&session_id)
            .unwrap()
            .has_unsaved_changes
    );

    let saved_as = backend.save_session_as(&session_id, &copy_dir).unwrap();
    let save_as_result = match saved_as {
        SaveSessionResponseDto::Saved(result) => result,
        SaveSessionResponseDto::Conflict(conflict) => {
            panic!(
                "unexpected save-as conflict: {}",
                conflict.actual_revision.0
            )
        }
    };
    assert_eq!(save_as_result.session_id, session_id);
    assert_eq!(save_as_result.root, copy_dir.display().to_string());
    assert_eq!(
        backend.open_package_session(&copy_dir).unwrap().session_id,
        session_id
    );
    assert_ne!(
        backend
            .open_package_session(&package_dir)
            .unwrap()
            .session_id,
        session_id
    );

    let reopened_copy = open_package(&copy_dir).unwrap();
    assert_eq!(
        reopened_copy
            .file()
            .well
            .get("COMP")
            .unwrap()
            .value
            .display_string(),
        "BACKEND EDIT"
    );
}

#[test]
fn backend_failed_save_as_keeps_existing_session_binding() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-save-as-fail");
    let existing_dir = temp_package_dir("backend-save-as-existing");
    lithos_las::write_package(&las, &package_dir).unwrap();
    fs::create_dir_all(&existing_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let session_id = session.session_id.clone();
    backend
        .apply_metadata_edit(
            &session_id,
            &MetadataUpdateRequest {
                items: vec![HeaderItemUpdate {
                    section: MetadataSectionDto::Well,
                    mnemonic: String::from("COMP"),
                    unit: String::new(),
                    value: LasValue::Text(String::from("UNCHANGED AFTER FAILURE")),
                    description: String::from("COMPANY"),
                }],
                other: None,
            },
        )
        .unwrap();
    let before = backend.session_summary(&session_id).unwrap();
    let before_metadata = backend.session_metadata(&session_id).unwrap();

    let err = backend
        .save_session_as(&session_id, &existing_dir)
        .unwrap_err();
    match err {
        LasError::Storage(message) => assert!(message.contains("already exists")),
        other => panic!("expected storage error, got {other}"),
    }

    let after = backend.session_summary(&session_id).unwrap();
    let after_metadata = backend.session_metadata(&session_id).unwrap();
    assert_eq!(after.root, package_dir.display().to_string());
    assert_eq!(after.session_id, session_id);
    assert_eq!(
        after.dirty.has_unsaved_changes,
        before.dirty.has_unsaved_changes
    );
    assert_eq!(
        after_metadata.metadata.metadata.well.company,
        before_metadata.metadata.metadata.well.company
    );
    assert_eq!(
        after_metadata.metadata.metadata.well.company.as_deref(),
        Some("UNCHANGED AFTER FAILURE")
    );
}

#[test]
fn backend_metadata_inspection_does_not_require_parquet_samples() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-metadata-only");
    lithos_las::write_package(&las, &package_dir).unwrap();
    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let mut backend = PackageBackend::new();
    let summary = backend.inspect_package_summary(&package_dir).unwrap();
    let metadata = backend.inspect_package_metadata(&package_dir).unwrap();
    let err = backend.open_package_session(&package_dir).unwrap_err();

    assert_eq!(summary.summary.las_version, "1.2");
    assert_eq!(metadata.metadata.curves.len(), 8);
    match err {
        LasError::Io(_) => {}
        other => panic!("expected io error when parquet data is missing, got {other}"),
    }
}

#[test]
fn backend_requires_explicit_close_for_session_cleanup() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-close");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let closed = backend.close_session(&session.session_id).unwrap();
    let reopened = backend.open_package_session(&package_dir).unwrap();

    assert!(closed.closed);
    assert_eq!(closed.session_id, session.session_id);
    assert_ne!(reopened.session_id, session.session_id);
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
