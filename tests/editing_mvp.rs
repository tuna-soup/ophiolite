use lithos_las::{
    CurveEditRequest, CurveUpdateRequest, CurveWindowRequest, HeaderItemUpdate, LasError, LasValue,
    MetadataSectionDto, MetadataUpdateRequest, examples, open_package, open_package_metadata,
    open_package_summary, validate_package, write_package,
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
