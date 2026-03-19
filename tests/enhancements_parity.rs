use ophiolite::{MnemonicCase, ReadOptions, examples};

#[test]
fn preserves_non_standard_header_sections() {
    let las = examples::open("non-standard-header-sections.las", &ReadOptions::default()).unwrap();
    assert!(las.extra_sections.contains_key("SPECIAL INFORMATION"));
    assert!(las.extra_sections.contains_key("extra special information"));
}

#[test]
fn mnemonic_case_modes_match_lasio_style_behavior() {
    let preserve = examples::open(
        "mnemonic_case.las",
        &ReadOptions {
            mnemonic_case: MnemonicCase::Preserve,
            ..ReadOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        preserve.keys(),
        vec!["Dept", "Sflu", "NPHI", "SFLU:1", "SFLU:2", "sflu", "SfLu"]
    );

    let upper = examples::open(
        "mnemonic_case.las",
        &ReadOptions {
            mnemonic_case: MnemonicCase::Upper,
            ..ReadOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        upper.keys(),
        vec![
            "DEPT", "SFLU:1", "NPHI", "SFLU:2", "SFLU:3", "SFLU:4", "SFLU:5"
        ]
    );

    let lower = examples::open(
        "mnemonic_case.las",
        &ReadOptions {
            mnemonic_case: MnemonicCase::Lower,
            ..ReadOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        lower.keys(),
        vec![
            "dept", "sflu:1", "nphi", "sflu:2", "sflu:3", "sflu:4", "sflu:5"
        ]
    );
}

#[test]
fn detects_depth_units_and_converts_depth_axes() {
    let metres = examples::open("autodepthindex_M.las", &ReadOptions::default()).unwrap();
    assert_eq!(metres.index_unit.as_deref(), Some("M"));
    let metres_index = metres.get_curve("DEPT").unwrap().numeric_data().unwrap();
    let metres_depth_ft = metres.depth_ft().unwrap();
    assert!((metres_depth_ft.last().unwrap() * 0.3048 - metres_index.last().unwrap()).abs() < 1e-9);

    let feet = examples::open("autodepthindex_FT.las", &ReadOptions::default()).unwrap();
    assert_eq!(feet.index_unit.as_deref(), Some("FT"));
    let feet_index = feet.get_curve("DEPT").unwrap().numeric_data().unwrap();
    let feet_depth_m = feet.depth_m().unwrap();
    assert!((feet_depth_m.last().unwrap() / 0.3048 - feet_index.last().unwrap()).abs() < 1e-9);

    let inconsistent = examples::open("autodepthindex_M_FT.las", &ReadOptions::default()).unwrap();
    assert_eq!(inconsistent.index_unit, None);
    assert!(inconsistent.depth_m().is_err());
}

#[test]
fn preserves_leading_zero_identifiers_and_tab_delimiters() {
    let leading_zero = examples::open("UWI_API_leading_zero.las", &ReadOptions::default()).unwrap();
    assert_eq!(
        leading_zero.well.get("UWI").unwrap().value.display_string(),
        "05123370660000"
    );

    let tab_delimited =
        examples::open("2.0/sample_2.0_tab_dlm.las", &ReadOptions::default()).unwrap();
    assert_eq!(tab_delimited.summary.delimiter, "TAB");
    assert_eq!(tab_delimited.summary.row_count, 3);
    assert_eq!(
        tab_delimited.get_curve("DT").unwrap().data[0]
            .as_f64()
            .unwrap(),
        123.45
    );
}
