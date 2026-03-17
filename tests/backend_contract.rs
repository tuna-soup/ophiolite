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
fn backend_session_open_keeps_metadata_queries_available_without_preloading_samples() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-lazy-open");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let summary = backend.session_summary(&session.session_id).unwrap();
    let metadata = backend.session_metadata(&session.session_id).unwrap();
    let catalog = backend.session_curve_catalog(&session.session_id).unwrap();
    let err = backend
        .read_curve_window(
            &session.session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 1,
            },
        )
        .unwrap_err();

    assert_eq!(summary.session_id, session.session_id);
    assert_eq!(metadata.session.session_id, session.session_id);
    assert_eq!(catalog.session.session_id, session.session_id);
    assert_eq!(catalog.curves.len(), 8);
    match err {
        LasError::Io(_) => {}
        other => panic!("expected io error when lazy window read cannot open parquet, got {other}"),
    }
}

#[test]
fn backend_clean_lazy_save_stays_lazy_and_preserves_session_state() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-clean-lazy-save");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let saved = backend.save_session(&session.session_id).unwrap();
    let save_result = match saved {
        SaveSessionResponseDto::Saved(result) => result,
        SaveSessionResponseDto::Conflict(conflict) => {
            panic!("unexpected save conflict: {}", conflict.actual_revision.0)
        }
    };

    assert_eq!(save_result.session_id, session.session_id);
    assert_eq!(save_result.root, package_dir.display().to_string());
    assert!(save_result.dirty_cleared);
    assert_eq!(
        backend
            .open_package_session(&package_dir)
            .unwrap()
            .session_id,
        session.session_id
    );

    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let summary = backend.session_summary(&session.session_id).unwrap();
    assert_eq!(summary.session_id, session.session_id);
    assert!(!summary.dirty.has_unsaved_changes);
    let err = backend
        .read_curve_window(
            &session.session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 1,
            },
        )
        .unwrap_err();

    match err {
        LasError::Io(_) => {}
        other => panic!("expected lazy window read io error after clean save, got {other}"),
    }
}

#[test]
fn backend_metadata_only_edit_and_save_stay_lazy() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-lazy-metadata-save");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let session_id = session.session_id.clone();

    let edited = backend
        .apply_metadata_edit(
            &session_id,
            &MetadataUpdateRequest {
                items: vec![HeaderItemUpdate {
                    section: MetadataSectionDto::Well,
                    mnemonic: String::from("COMP"),
                    unit: String::new(),
                    value: LasValue::Text(String::from("LAZY METADATA SAVE")),
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

    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let metadata = backend.session_metadata(&session_id).unwrap();
    assert_eq!(
        metadata.metadata.metadata.well.company.as_deref(),
        Some("LAZY METADATA SAVE")
    );
    let err = backend
        .read_curve_window(
            &session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 2,
            },
        )
        .unwrap_err();
    match err {
        LasError::Io(_) => {}
        other => panic!("expected lazy window read io error after metadata-only save, got {other}"),
    }
}

#[test]
fn backend_metadata_only_save_as_stays_lazy_and_rebinds_root() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-lazy-metadata-save-as");
    let copy_dir = temp_package_dir("backend-lazy-metadata-save-as-copy");
    lithos_las::write_package(&las, &package_dir).unwrap();

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
                    value: LasValue::Text(String::from("LAZY SAVE AS")),
                    description: String::from("COMPANY"),
                }],
                other: None,
            },
        )
        .unwrap();

    let saved = backend.save_session_as(&session_id, &copy_dir).unwrap();
    let save_result = match saved {
        SaveSessionResponseDto::Saved(result) => result,
        SaveSessionResponseDto::Conflict(conflict) => {
            panic!(
                "unexpected save-as conflict: {}",
                conflict.actual_revision.0
            )
        }
    };
    assert_eq!(save_result.session_id, session_id);
    assert_eq!(save_result.root, copy_dir.display().to_string());

    fs::remove_file(copy_dir.join("curves.parquet")).unwrap();

    let summary = backend.session_summary(&session_id).unwrap();
    let metadata = backend.session_metadata(&session_id).unwrap();
    assert_eq!(summary.root, copy_dir.display().to_string());
    assert_eq!(
        metadata.metadata.metadata.well.company.as_deref(),
        Some("LAZY SAVE AS")
    );
    let err = backend
        .read_curve_window(
            &session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 1,
            },
        )
        .unwrap_err();
    match err {
        LasError::Io(_) => {}
        other => {
            panic!("expected lazy window read io error after metadata-only save-as, got {other}")
        }
    }
}

#[test]
fn backend_first_successful_curve_edit_materializes_and_stays_materialized() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-materialized-after-curve-edit");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let session_id = session.session_id.clone();
    let before = backend
        .read_curve_window(
            &session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 3,
            },
        )
        .unwrap();
    let dt_values = before.window.columns[0].values.clone();

    let edited = backend
        .apply_curve_edit(
            &session_id,
            &CurveEditRequest::Upsert(CurveUpdateRequest {
                mnemonic: String::from("DT"),
                original_mnemonic: Some(String::from("DT")),
                unit: String::from("US/M"),
                header_value: LasValue::Empty,
                description: String::from("materialized curve edit"),
                data: dt_values,
            }),
        )
        .unwrap();
    assert!(edited.dirty.has_unsaved_changes);

    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let window = backend
        .read_curve_window(
            &session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 2,
            },
        )
        .unwrap();

    assert_eq!(window.session.session_id, session_id);
    assert_eq!(window.window.row_count, 2);
}

#[test]
fn backend_failed_materialization_keeps_lazy_session_open_and_unchanged() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-failed-materialize");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();
    let session_id = session.session_id.clone();
    let before = backend.session_summary(&session_id).unwrap();

    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let err = backend
        .apply_metadata_edit(
            &session_id,
            &MetadataUpdateRequest {
                items: vec![HeaderItemUpdate {
                    section: MetadataSectionDto::Well,
                    mnemonic: String::from("COMP"),
                    unit: String::new(),
                    value: LasValue::Text(String::from("SHOULD NOT APPLY")),
                    description: String::from("COMPANY"),
                }],
                other: None,
            },
        )
        .unwrap_err();
    match err {
        LasError::Io(_) => {}
        other => panic!("expected io error during lazy materialization, got {other}"),
    }

    let after = backend.session_summary(&session_id).unwrap();
    let metadata = backend.session_metadata(&session_id).unwrap();
    assert_eq!(after.session_id, before.session_id);
    assert_eq!(after.root, before.root);
    assert_eq!(
        after.dirty.has_unsaved_changes,
        before.dirty.has_unsaved_changes
    );
    assert_ne!(
        metadata.metadata.metadata.well.company.as_deref(),
        Some("SHOULD NOT APPLY")
    );

    let window_err = backend
        .read_curve_window(
            &session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 1,
            },
        )
        .unwrap_err();
    match window_err {
        LasError::Io(_) => {}
        other => {
            panic!("expected lazy window read io error after failed materialization, got {other}")
        }
    }
}

#[test]
fn backend_lazy_window_reads_reject_stale_sessions_after_external_change() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("backend-stale-lazy");
    lithos_las::write_package(&las, &package_dir).unwrap();

    let mut backend = PackageBackend::new();
    let session = backend.open_package_session(&package_dir).unwrap();

    let mut external = open_package(&package_dir).unwrap();
    external
        .apply_metadata_update(&MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("EXTERNAL CHANGE")),
                description: String::from("COMPANY"),
            }],
            other: None,
        })
        .unwrap();
    external.save_with_result().unwrap();

    let err = backend
        .read_curve_window(
            &session.session_id,
            &CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 1,
            },
        )
        .unwrap_err();

    match err {
        LasError::Validation(message) => {
            assert!(message.contains("changed since session"));
        }
        other => panic!("expected stale-session validation error, got {other}"),
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
