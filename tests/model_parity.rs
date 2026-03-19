use ophiolite::{
    CurveItem, CurveSelector, HeaderItem, LasValue, MnemonicCase, SectionItems, examples,
    import_las_file,
};

#[test]
fn supports_curve_lookup_by_name_and_index() {
    let las = import_las_file(examples::path("sample.las")).unwrap();
    assert_eq!(las.curve_data("DT").unwrap()[0].as_f64().unwrap(), 123.45);
    assert_eq!(las.curve_data_at(1).unwrap()[0].as_f64().unwrap(), 123.45);
    assert_eq!(las.curve_data_at(-2).unwrap()[0].as_f64().unwrap(), 110.2);
}

#[test]
fn supports_curve_mutation_and_replacement() {
    let mut las = import_las_file(examples::path("sample.las")).unwrap();
    assert!(las.update_curve_data(
        "NPHI",
        vec![
            LasValue::Number(45.0),
            LasValue::Number(45.0),
            LasValue::Number(45.0)
        ]
    ));
    assert_eq!(las.curve_data("NPHI").unwrap()[0].as_f64().unwrap(), 45.0);

    las.replace_curve_item(
        "NPHI",
        CurveItem::new(
            "NPHI",
            "%",
            LasValue::Empty,
            "Porosity",
            vec![
                LasValue::Number(99.0),
                LasValue::Number(98.0),
                LasValue::Number(97.0),
            ],
        ),
    );
    assert_eq!(las.curve_data("NPHI").unwrap()[2].as_f64().unwrap(), 97.0);
}

#[test]
fn supports_stack_curves_with_prefix() {
    let las = import_las_file(examples::path("multi_channel_natural_sorting.las")).unwrap();
    let stack = las
        .stack_curves(CurveSelector::Prefix(String::from("CBP")), true)
        .unwrap();
    assert_eq!(
        stack[0],
        vec![
            0.0144, 0.0011, 0.0013, 0.002, 0.0055, 0.0103, 0.0543, 0.2003
        ]
    );
}

#[test]
fn supports_stack_curves_with_explicit_order_without_sorting() {
    let las = import_las_file(examples::path("multi_channel_natural_sorting.las")).unwrap();
    let stack = las
        .stack_curves(
            CurveSelector::Names(vec![
                String::from("CBP13"),
                String::from("CBP2003"),
                String::from("CBP11"),
                String::from("CBP1"),
                String::from("CBP103"),
                String::from("CBP543"),
                String::from("CBP20"),
                String::from("CBP55"),
            ]),
            false,
        )
        .unwrap();
    assert_eq!(
        stack[0],
        vec![
            0.0013, 0.2003, 0.0011, 0.0144, 0.0103, 0.0543, 0.002, 0.0055
        ]
    );
}

#[test]
fn stack_curves_reports_missing_names_like_lasio() {
    let las = import_las_file(examples::path("multi_channel_natural_sorting.las")).unwrap();
    let err = las
        .stack_curves(
            CurveSelector::Names(vec![
                String::from("CBP1"),
                String::from("CBP13"),
                String::from("KTIM"),
                String::from("TCMR"),
            ]),
            true,
        )
        .unwrap_err();
    assert!(
        err == "KTIM, TCMR not found in LAS curves."
            || err == "TCMR, KTIM not found in LAS curves."
    );
}

#[test]
fn section_items_get_or_create_behaves_like_header_container() {
    let mut items: SectionItems<HeaderItem> = SectionItems::new(MnemonicCase::Preserve);
    let item = items.get_or_create("WELL", None, true);
    assert_eq!(item.mnemonic, "WELL");
    assert!(items.contains("WELL"));
}
