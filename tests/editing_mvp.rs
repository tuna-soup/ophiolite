use lithos_las::{
    CurveEditRequest, CurveUpdateRequest, CurveWindowRequest, HeaderItemUpdate, LasError, LasValue,
    MetadataSectionDto, MetadataUpdateRequest, PackageSessionStore, ValidationKind, examples,
    open_package, open_package_metadata, open_package_summary, validate_package, write_package,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn stored_package_supports_metadata_curve_edits_and_overwrite() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("edit-overwrite");
    let mut stored = write_package(&las, &package_dir).unwrap();

    stored
        .apply_metadata_update(&MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("EDITED OIL COMPANY")),
                description: String::from("COMPANY"),
            }],
            other: Some(String::from("Edited package note")),
        })
        .unwrap();

    let original_dt = stored.file().curve("DT").unwrap().clone();
    let mut dt_data = original_dt.data.clone();
    dt_data[0] = LasValue::Number(222.2);
    stored
        .apply_curve_edit(&CurveEditRequest::Upsert(CurveUpdateRequest {
            mnemonic: String::from("DT"),
            original_mnemonic: Some(original_dt.original_mnemonic.clone()),
            unit: original_dt.unit.clone(),
            header_value: original_dt.value.clone(),
            description: original_dt.description.clone(),
            data: dt_data,
        }))
        .unwrap();
    stored
        .apply_curve_edit(&CurveEditRequest::Remove {
            mnemonic: String::from("SFLU"),
        })
        .unwrap();

    let summary = stored.summary_dto();
    assert_eq!(summary.summary.curve_count, 7);

    let window = stored
        .read_window(&CurveWindowRequest {
            curve_names: vec![String::from("DT"), String::from("RHOB")],
            start_row: 1,
            row_count: 2,
        })
        .unwrap();
    assert_eq!(window.row_count, 2);
    assert_eq!(window.columns[0].values[0].as_f64(), Some(123.45));

    let save_result = stored.save_with_result().unwrap();
    assert!(save_result.overwritten);
    assert_eq!(save_result.summary.summary.curve_count, 7);

    let reopened = open_package(&package_dir).unwrap();
    assert_eq!(
        reopened
            .file()
            .well
            .get("COMP")
            .unwrap()
            .value
            .display_string(),
        "EDITED OIL COMPANY"
    );
    assert_eq!(
        reopened.file().curve("DT").unwrap().data[0].as_f64(),
        Some(222.2)
    );
    assert!(reopened.file().get_curve("SFLU").is_none());
    assert_eq!(reopened.summary().curve_count, 7);
    assert!(reopened.file().other.contains("Edited package note"));
}

#[test]
fn stored_package_supports_save_as_copy() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("save-as-source");
    let copy_dir = temp_package_dir("save-as-copy");
    let stored = write_package(&las, &package_dir).unwrap();

    let save_result = stored.save_as_with_result(&copy_dir).unwrap();
    let copy = open_package(&copy_dir).unwrap();

    assert!(copy_dir.join("metadata.json").exists());
    assert!(copy_dir.join("curves.parquet").exists());
    assert!(!save_result.overwritten);
    assert_eq!(
        save_result.summary.summary.curve_count,
        stored.summary().curve_count
    );
    assert_eq!(copy.summary().curve_count, stored.summary().curve_count);
    assert_eq!(copy.file().curve_names(), stored.file().curve_names());
}

#[test]
fn curve_edits_reject_inconsistent_row_counts() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("invalid-edit");
    let mut stored = write_package(&las, &package_dir).unwrap();

    let err = stored
        .apply_curve_edit(&CurveEditRequest::Upsert(CurveUpdateRequest {
            mnemonic: String::from("DT"),
            original_mnemonic: Some(String::from("DT")),
            unit: String::from("US/M"),
            header_value: LasValue::Empty,
            description: String::from("invalid"),
            data: vec![LasValue::Number(1.0)],
        }))
        .unwrap_err();

    match err {
        LasError::Validation(message) => {
            assert!(message.contains("expects 3"));
        }
        other => panic!("expected validation error, got {other}"),
    }
}

#[test]
fn dto_views_reflect_current_package_state() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("dto-state");
    let stored = write_package(&las, &package_dir).unwrap();

    let metadata = stored.metadata_dto();
    assert_eq!(
        metadata.metadata.well.well.as_deref(),
        Some("ANY ET AL OIL WELL #12")
    );

    let catalog = stored.curve_catalog();
    assert_eq!(catalog.len(), 8);
    let rhob = catalog.iter().find(|curve| curve.name == "RHOB").unwrap();
    assert_eq!(rhob.alias.mnemonic.as_deref(), Some("bulk_density"));
    assert_eq!(rhob.row_count, 3);
}

#[test]
fn package_supports_metadata_only_open_and_validation_views() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("metadata-only");
    write_package(&las, &package_dir).unwrap();

    let summary = open_package_summary(&package_dir).unwrap();
    let metadata = open_package_metadata(&package_dir).unwrap();
    let validation = validate_package(&package_dir).unwrap();

    assert_eq!(summary.summary.las_version, "1.2");
    assert_eq!(metadata.metadata.index.canonical_name, "index");
    assert!(validation.valid);
    assert!(validation.errors.is_empty());
    assert_eq!(validation.kind, ValidationKind::Package);
}

#[test]
fn package_session_tracks_dirty_state_and_conflicts() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("session-conflict");
    let mut first = write_package(&las, &package_dir).unwrap();
    let mut second = open_package(&package_dir).unwrap();

    assert!(!first.dirty_state().has_unsaved_changes);
    let first_revision = first.revision().clone();

    first
        .apply_metadata_update(&MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("FIRST EDIT")),
                description: String::from("COMPANY"),
            }],
            other: None,
        })
        .unwrap();
    assert!(first.dirty_state().has_unsaved_changes);

    second
        .apply_metadata_update(&MetadataUpdateRequest {
            items: vec![HeaderItemUpdate {
                section: MetadataSectionDto::Well,
                mnemonic: String::from("COMP"),
                unit: String::new(),
                value: LasValue::Text(String::from("SECOND EDIT")),
                description: String::from("COMPANY"),
            }],
            other: None,
        })
        .unwrap();

    let second_save = second.save_with_result().unwrap();
    assert!(second_save.dirty_cleared);
    assert_ne!(second_save.revision, first_revision);

    let conflict = first.save_checked().unwrap().unwrap_err();
    assert_eq!(conflict.expected_revision, first_revision);
    assert_eq!(conflict.actual_revision, second_save.revision);
    assert_eq!(conflict.package_id, first.package_id().clone());
    assert_eq!(conflict.session_id, first.session_id().clone());
}

#[test]
fn package_session_store_reuses_shared_session_identity() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("shared-session");
    write_package(&las, &package_dir).unwrap();

    let mut store = PackageSessionStore::default();
    let first = store.open_shared(&package_dir).unwrap();
    let second = store.open_shared(&package_dir).unwrap();

    assert_eq!(first.package_id, second.package_id);
    assert_eq!(first.session_id, second.session_id);
    assert!(!first.dirty.has_unsaved_changes);
    assert!(store.get(&first.session_id).is_some());
    assert!(store.close(&first.session_id).is_some());
    assert!(store.get(&first.session_id).is_none());
}

#[test]
fn metadata_only_open_does_not_require_parquet_samples() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("metadata-no-parquet");
    write_package(&las, &package_dir).unwrap();
    fs::remove_file(package_dir.join("curves.parquet")).unwrap();

    let summary = open_package_summary(&package_dir).unwrap();
    let metadata = open_package_metadata(&package_dir).unwrap();
    let err = open_package(&package_dir).unwrap_err();

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
    let path = std::env::temp_dir().join(format!("lithos-{prefix}-{unique}"));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}
