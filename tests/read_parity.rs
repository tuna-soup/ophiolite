use ophiolite::{DType, DTypeSpec, LasValue, NullPolicy, ReadOptions, examples, import_las_file};

#[test]
fn reads_sample_file_and_curve_keys() {
    let las = import_las_file(examples::path("sample.las")).unwrap();
    assert_eq!(
        las.keys(),
        vec!["DEPT", "DT", "RHOB", "NPHI", "SFLU", "SFLA", "ILM", "ILD"]
    );
    assert_eq!(las.summary.las_version, "1.2");
    assert_eq!(las.summary.wrap_mode, "NO");
    assert_eq!(
        las.version.get("VERS").unwrap().value.as_f64().unwrap(),
        1.2
    );
    assert_eq!(
        las.well.get("COMP").unwrap().value.display_string(),
        "# ANY OIL COMPANY LTD."
    );
    assert_eq!(las.well.get("COMP").unwrap().description, "COMPANY");
    assert_eq!(las.summary.row_count, 3);
}

#[test]
fn reads_wrapped_sample() {
    let las = import_las_file(examples::path("1.2/sample_wrapped.las")).unwrap();
    assert_eq!(las.summary.wrap_mode.to_ascii_uppercase(), "YES");
    assert_eq!(las.summary.row_count, 5);
    assert_eq!(
        las.get_curve("GR").unwrap().data[0].as_f64().unwrap(),
        96.5306
    );
}

#[test]
fn preserves_duplicate_and_unknown_mnemonics() {
    let duplicate = import_las_file(examples::path("mnemonic_duplicate.las")).unwrap();
    assert_eq!(
        duplicate.keys(),
        vec![
            "DEPT", "DT", "RHOB", "NPHI", "SFLU:1", "SFLU:2", "ILM", "ILD"
        ]
    );

    let missing = import_las_file(examples::path("mnemonic_missing_multiple.las")).unwrap();
    assert_eq!(
        missing.keys(),
        vec![
            "DEPT",
            "DT",
            "RHOB",
            "NPHI",
            "UNKNOWN:1",
            "UNKNOWN:2",
            "ILM",
            "ILD"
        ]
    );
}

#[test]
fn local_examples_helper_opens_fixture() {
    let las = examples::open("sample.las", &ReadOptions::default()).unwrap();
    assert_eq!(las.summary.original_filename, "sample.las");
}

#[test]
fn supports_text_columns_when_dtype_forced() {
    let las = examples::open(
        "sample_str_in_data.las",
        &ReadOptions {
            dtypes: DTypeSpec::PerColumn(vec![
                DType::Float,
                DType::Text,
                DType::Integer,
                DType::Float,
            ]),
            read_policy: ophiolite::ReadPolicy::None,
            ..ReadOptions::default()
        },
    )
    .unwrap();

    assert!(matches!(
        las.get_curve("DT_STR").unwrap().data[0],
        LasValue::Text(_)
    ));
    assert!(matches!(
        las.get_curve("NPHI_FLOAT").unwrap().data[0],
        LasValue::Number(_)
    ));
}

#[test]
fn null_policy_none_keeps_declared_null() {
    let las = examples::open(
        "null_policy_-999.25.las",
        &ReadOptions {
            null_policy: NullPolicy::None,
            ..ReadOptions::default()
        },
    )
    .unwrap();

    assert_eq!(
        las.get_curve("DT").unwrap().data[0].as_f64().unwrap(),
        -999.25
    );
}

#[test]
fn aggressive_null_policy_replaces_common_sentinels() {
    let las = examples::open(
        "null_policy_9999.las",
        &ReadOptions {
            null_policy: NullPolicy::Aggressive,
            ..ReadOptions::default()
        },
    )
    .unwrap();

    assert!(las.get_curve("DT").unwrap().data[1].is_nan());
}
