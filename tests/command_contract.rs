use ophiolite::{
    CurveEditRequest, CurveUpdateRequest, CurveWindowRequest, HeaderItemUpdate, LasError, LasValue,
    MetadataSectionDto, MetadataUpdateRequest, PackageBackendState, PackagePathRequest,
    SaveSessionResponseDto, SessionCurveEditRequest, SessionMetadataEditRequest, SessionRequest,
    SessionSaveAsRequest, SessionWindowRequest, examples, write_package,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn command_state_supports_shared_session_open_and_query() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("command-open");
    write_package(&las, &package_dir).unwrap();

    let state = PackageBackendState::new();
    let request = PackagePathRequest {
        path: package_dir.display().to_string(),
    };
    let first = state.open_package_session(&request).unwrap();
    let second = state.open_package_session(&request).unwrap();

    assert_eq!(first.session_id, second.session_id);
    let catalog = state
        .session_curve_catalog(&SessionRequest {
            session_id: first.session_id.clone(),
        })
        .unwrap();
    let window = state
        .read_curve_window(&SessionWindowRequest {
            session_id: first.session_id.clone(),
            window: CurveWindowRequest {
                curve_names: vec![String::from("DT")],
                start_row: 0,
                row_count: 2,
            },
        })
        .unwrap();

    assert_eq!(catalog.curves.len(), 8);
    assert_eq!(catalog.session.session_id, first.session_id);
    assert_eq!(window.window.columns.len(), 1);
    assert_eq!(window.window.columns[0].values.len(), 2);
}

#[test]
fn command_state_applies_atomic_edits_and_save_flows() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("command-save");
    let copy_dir = temp_package_dir("command-save-copy");
    write_package(&las, &package_dir).unwrap();

    let state = PackageBackendState::new();
    let session = state
        .open_package_session(&PackagePathRequest {
            path: package_dir.display().to_string(),
        })
        .unwrap();

    let err = state
        .apply_curve_edit(&SessionCurveEditRequest {
            session_id: session.session_id.clone(),
            edit: CurveEditRequest::Upsert(CurveUpdateRequest {
                mnemonic: String::from("DT"),
                original_mnemonic: Some(String::from("DT")),
                unit: String::from("US/M"),
                header_value: LasValue::Empty,
                description: String::from("invalid"),
                data: vec![LasValue::Number(1.0)],
            }),
        })
        .unwrap_err();
    match err {
        LasError::Validation(message) => assert!(message.contains("expects 3")),
        other => panic!("expected validation error, got {other}"),
    }

    let edited = state
        .apply_metadata_edit(&SessionMetadataEditRequest {
            session_id: session.session_id.clone(),
            update: MetadataUpdateRequest {
                items: vec![HeaderItemUpdate {
                    section: MetadataSectionDto::Well,
                    mnemonic: String::from("COMP"),
                    unit: String::new(),
                    value: LasValue::Text(String::from("COMMAND EDIT")),
                    description: String::from("COMPANY"),
                }],
                other: None,
            },
        })
        .unwrap();
    assert!(edited.dirty.has_unsaved_changes);

    let saved = state
        .save_session(&SessionRequest {
            session_id: session.session_id.clone(),
        })
        .unwrap();
    match saved {
        SaveSessionResponseDto::Saved(result) => assert_eq!(result.session_id, session.session_id),
    }

    let saved_as = state
        .save_session_as(&SessionSaveAsRequest {
            session_id: session.session_id.clone(),
            output_dir: copy_dir.display().to_string(),
        })
        .unwrap();
    match saved_as {
        SaveSessionResponseDto::Saved(result) => assert_eq!(result.session_id, session.session_id),
    }
}

#[test]
fn command_state_supports_metadata_only_inspection_without_parquet() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("command-metadata-only");
    write_package(&las, &package_dir).unwrap();
    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let state = PackageBackendState::new();
    let request = PackagePathRequest {
        path: package_dir.display().to_string(),
    };

    let summary = state.inspect_package_summary(&request).unwrap();
    let metadata = state.inspect_package_metadata(&request).unwrap();
    let err = state.open_package_session(&request).unwrap_err();

    assert_eq!(summary.summary.las_version, "1.2");
    assert_eq!(metadata.metadata.curves.len(), 8);
    match err {
        LasError::Io(_) => {}
        other => panic!("expected io error when parquet data is missing, got {other}"),
    }
}

fn temp_package_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ophiolite-{prefix}-{unique}"));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}
