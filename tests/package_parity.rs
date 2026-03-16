use lithos_las::{ReadOptions, examples, import_las_file, open_package, write_package};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn writes_and_reopens_option_a_style_package() {
    let las = import_las_file(examples::path("sample.las")).unwrap();
    let package_dir = temp_package_dir("sample-package");

    let package = write_package(&las, &package_dir).unwrap();

    assert!(package_dir.join("metadata.json").exists());
    assert!(package_dir.join("curves.parquet").exists());
    assert_eq!(package.summary().las_version, "1.2");
    assert_eq!(
        package.read_curve("DT").unwrap()[0].as_f64().unwrap(),
        123.45
    );

    let reopened = open_package(&package_dir).unwrap();
    assert_eq!(reopened.file().keys(), las.keys());
    assert_eq!(
        reopened
            .file()
            .well
            .get("COMP")
            .unwrap()
            .value
            .display_string(),
        "# ANY OIL COMPANY LTD."
    );
    assert_eq!(
        reopened
            .data()
            .column("DT")
            .unwrap()
            .numeric_values()
            .unwrap()[0],
        123.45
    );
}

#[test]
fn package_roundtrip_preserves_mixed_curve_columns() {
    let las = examples::open("null_policy_ERR.las", &ReadOptions::default()).unwrap();
    let package_dir = temp_package_dir("mixed-package");

    let reopened = write_package(&las, &package_dir).unwrap();
    let rhob = reopened.file().get_curve("RHOB").unwrap();
    assert_eq!(rhob.data[2].as_str(), Some("ERR"));
    assert_eq!(
        reopened.file().get_curve("ILD").unwrap().data[2]
            .as_f64()
            .unwrap(),
        105.6
    );
}

#[test]
fn curve_table_supports_column_access_and_slicing() {
    let las = import_las_file(examples::path("sample.las")).unwrap();
    let table = las.data();
    assert_eq!(table.row_count(), 3);
    assert_eq!(table.column_names(), las.keys());

    let window = table.slice_rows(1, 3);
    assert_eq!(window.row_count(), 2);
    assert_eq!(
        window.column("DT").unwrap().numeric_values().unwrap(),
        vec![123.45, 123.45]
    );
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
