use lithos_las::{
    CommandErrorKind, CommandResponse, CurveEditRequest, CurveUpdateRequest, HeaderItemUpdate,
    LasValue, MetadataSectionDto, MetadataUpdateRequest, PackageCommandService, PackagePathRequest,
    SessionCurveEditRequest, SessionDepthWindowRequest, SessionMetadataEditRequest, SessionRequest,
    SessionSaveAsRequest, SessionWindowRequest, ValidationKind, examples, open_package,
    write_package,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn command_service_reuses_shared_session_by_default() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-shared");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let request = PackagePathRequest {
        path: package_dir.display().to_string(),
    };

    let first = unwrap_ok(service.open_package_session(&request));
    let second = unwrap_ok(service.open_package_session(&request));

    assert_eq!(first.session_id, second.session_id);
    assert_eq!(first.root, package_dir.display().to_string());
    let metadata = unwrap_ok(service.session_metadata(&SessionRequest {
        session_id: first.session_id.clone(),
    }));
    assert_eq!(metadata.session.session_id, first.session_id);
    assert_eq!(metadata.session.root, package_dir.display().to_string());
}

#[test]
fn command_service_reports_closed_session_errors_structurally() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-close");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let request = PackagePathRequest {
        path: package_dir.display().to_string(),
    };
    let session = unwrap_ok(service.open_package_session(&request));

    let closed = unwrap_ok(service.close_session(&SessionRequest {
        session_id: session.session_id.clone(),
    }));
    assert!(closed.closed);

    let after_close = service.session_summary(&SessionRequest {
        session_id: session.session_id.clone(),
    });
    let error = unwrap_err(after_close);
    assert_eq!(error.kind, CommandErrorKind::SessionNotFound);
    assert_eq!(error.session_id, Some(session.session_id));
}

#[test]
fn command_service_preserves_validation_errors() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-validation");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let session = unwrap_ok(service.open_package_session(&PackagePathRequest {
        path: package_dir.display().to_string(),
    }));

    let response = service.apply_curve_edit(&SessionCurveEditRequest {
        session_id: session.session_id.clone(),
        edit: CurveEditRequest::Upsert(CurveUpdateRequest {
            mnemonic: String::from("DT"),
            original_mnemonic: Some(String::from("DT")),
            unit: String::from("US/M"),
            header_value: LasValue::Empty,
            description: String::from("invalid"),
            data: vec![LasValue::Number(1.0)],
        }),
    });
    let error = unwrap_err(response);

    assert_eq!(error.kind, CommandErrorKind::ValidationFailed);
    assert_eq!(error.session_id, Some(session.session_id));
    let validation = error.validation.expect("validation report");
    assert_eq!(validation.kind, ValidationKind::Edit);
    assert_eq!(validation.issues.len(), 1);
    assert_eq!(validation.issues[0].code, "curve.row_count_mismatch");
    assert!(error.message.contains("expects 3"));
}

#[test]
fn command_service_uses_last_save_wins_for_external_changes() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-conflict");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let session = unwrap_ok(service.open_package_session(&PackagePathRequest {
        path: package_dir.display().to_string(),
    }));

    let _edited = unwrap_ok(service.apply_metadata_edit(&SessionMetadataEditRequest {
        session_id: session.session_id.clone(),
        update: MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("ADAPTER EDIT")),
                description: String::from("COMPANY"),
            }],
            other: None,
        },
    }));

    let mut external = open_package(&package_dir).unwrap();
    external
        .apply_metadata_update(&MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("EXTERNAL EDIT")),
                description: String::from("COMPANY"),
            }],
            other: None,
        })
        .unwrap();
    external.save_with_result().unwrap();

    let saved = unwrap_ok(service.save_session(&SessionRequest {
        session_id: session.session_id.clone(),
    }));

    assert_eq!(saved.session_id, session.session_id);
    let reopened = open_package(&package_dir).unwrap();
    assert_eq!(
        reopened
            .file()
            .well
            .get("COMP")
            .unwrap()
            .value
            .display_string(),
        "ADAPTER EDIT"
    );
}

#[test]
fn command_service_reports_save_as_validation_failures_as_save_errors() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-save-as-fail");
    let existing_dir = temp_package_dir("adapter-save-as-existing");
    write_package(&las, &package_dir).unwrap();
    fs::create_dir_all(&existing_dir).unwrap();

    let service = PackageCommandService::new();
    let session = unwrap_ok(service.open_package_session(&PackagePathRequest {
        path: package_dir.display().to_string(),
    }));
    let edited = unwrap_ok(service.apply_metadata_edit(&SessionMetadataEditRequest {
        session_id: session.session_id.clone(),
        update: MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("COMMAND FAILURE EDIT")),
                description: String::from("COMPANY"),
            }],
            other: None,
        },
    }));
    let before_metadata = unwrap_ok(service.session_metadata(&SessionRequest {
        session_id: session.session_id.clone(),
    }));

    let error = unwrap_err(service.save_session_as(&SessionSaveAsRequest {
        session_id: session.session_id.clone(),
        output_dir: existing_dir.display().to_string(),
    }));

    assert_eq!(error.kind, CommandErrorKind::ValidationFailed);
    assert_eq!(error.session_id, Some(session.session_id.clone()));
    let validation = error.validation.expect("save validation report");
    assert_eq!(validation.kind, ValidationKind::Save);
    assert_eq!(validation.issues.len(), 1);
    assert_eq!(validation.issues[0].code, "save.output_dir.exists");

    let summary = unwrap_ok(service.session_summary(&SessionRequest {
        session_id: session.session_id.clone(),
    }));
    assert_eq!(summary.root, package_dir.display().to_string());
    assert_eq!(
        summary.dirty.has_unsaved_changes,
        edited.dirty.has_unsaved_changes
    );
    let after_metadata = unwrap_ok(service.session_metadata(&SessionRequest {
        session_id: session.session_id.clone(),
    }));
    assert_eq!(
        after_metadata.metadata.metadata.well.company,
        before_metadata.metadata.metadata.well.company
    );
    assert_eq!(
        after_metadata.metadata.metadata.well.company.as_deref(),
        Some("COMMAND FAILURE EDIT")
    );
}

#[test]
fn command_service_keeps_metadata_only_save_flows_lazy() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-lazy-metadata-save");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let session = unwrap_ok(service.open_package_session(&PackagePathRequest {
        path: package_dir.display().to_string(),
    }));

    let edited = unwrap_ok(service.apply_metadata_edit(&SessionMetadataEditRequest {
        session_id: session.session_id.clone(),
        update: MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("COMMAND LAZY SAVE")),
                description: String::from("COMPANY"),
            }],
            other: None,
        },
    }));
    assert!(edited.dirty.has_unsaved_changes);

    let saved = unwrap_ok(service.save_session(&SessionRequest {
        session_id: session.session_id.clone(),
    }));
    assert_eq!(saved.session_id, session.session_id);

    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let metadata = unwrap_ok(service.session_metadata(&SessionRequest {
        session_id: session.session_id.clone(),
    }));
    assert_eq!(
        metadata.metadata.metadata.well.company.as_deref(),
        Some("COMMAND LAZY SAVE")
    );

    let err = unwrap_err(service.read_curve_window(&SessionWindowRequest {
        session_id: session.session_id.clone(),
        window: lithos_las::CurveWindowRequest {
            curve_names: vec![String::from("DT")],
            start_row: 0,
            row_count: 1,
        },
    }));
    assert_eq!(err.kind, CommandErrorKind::OpenFailed);
}

#[test]
fn command_service_supports_metadata_only_inspection_without_parquet() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-metadata-only");
    write_package(&las, &package_dir).unwrap();
    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let service = PackageCommandService::new();
    let request = PackagePathRequest {
        path: package_dir.display().to_string(),
    };

    let summary = unwrap_ok(service.inspect_package_summary(&request));
    let metadata = unwrap_ok(service.inspect_package_metadata(&request));
    let session_error = unwrap_err(service.open_package_session(&request));

    assert_eq!(summary.summary.las_version, "1.2");
    assert_eq!(metadata.metadata.curves.len(), 8);
    assert_eq!(session_error.kind, CommandErrorKind::OpenFailed);
}

#[test]
fn command_service_supports_depth_window_queries() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-depth-window");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let session = unwrap_ok(service.open_package_session(&PackagePathRequest {
        path: package_dir.display().to_string(),
    }));
    let row_window = unwrap_ok(service.read_curve_window(&SessionWindowRequest {
        session_id: session.session_id.clone(),
        window: lithos_las::CurveWindowRequest {
            curve_names: vec![String::from("DEPT"), String::from("DT")],
            start_row: 0,
            row_count: 3,
        },
    }));
    let depth_values = row_window.window.columns[0]
        .values
        .iter()
        .map(|value| value.as_f64().unwrap())
        .collect::<Vec<_>>();

    let depth_window = unwrap_ok(service.read_depth_window(&SessionDepthWindowRequest {
        session_id: session.session_id.clone(),
        window: lithos_las::DepthWindowRequest {
            curve_names: vec![String::from("DEPT"), String::from("DT")],
            depth_min: depth_values[0].min(depth_values[1]),
            depth_max: depth_values[0].max(depth_values[1]),
        },
    }));

    assert_eq!(depth_window.session.session_id, session.session_id);
    assert_eq!(depth_window.window.start_row, 0);
    assert_eq!(depth_window.window.row_count, 2);
}

#[test]
fn command_service_validate_package_returns_structured_diagnostics() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("adapter-validate");
    write_package(&las, &package_dir).unwrap();

    let service = PackageCommandService::new();
    let validation = unwrap_ok(service.validate_package(&PackagePathRequest {
        path: package_dir.display().to_string(),
    }));

    assert!(validation.valid);
    assert!(validation.errors.is_empty());
    assert!(validation.issues.is_empty());
    assert_eq!(validation.kind, ValidationKind::Package);
}

fn unwrap_ok<T>(response: CommandResponse<T>) -> T {
    match response {
        CommandResponse::Ok(value) => value,
        CommandResponse::Err(error) => panic!("expected ok response, got {}", error.message),
    }
}

fn unwrap_err<T>(response: CommandResponse<T>) -> lithos_las::CommandErrorDto {
    match response {
        CommandResponse::Ok(_) => panic!("expected error response"),
        CommandResponse::Err(error) => error,
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
