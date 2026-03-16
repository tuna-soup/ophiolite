use lithos_las::{
    CurveStorageKind, PACKAGE_METADATA_SCHEMA_VERSION, PackageMetadata, examples, import_las_file,
    write_package,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn derives_typed_canonical_metadata_from_las_file() {
    let las = import_las_file(examples::path("sample.las")).unwrap();

    let metadata = las.metadata();
    assert_eq!(metadata.version.vers.as_deref(), Some("1.2"));
    assert_eq!(metadata.version.wrap.as_deref(), Some("NO"));
    assert_eq!(
        metadata.well.company.as_deref(),
        Some("# ANY OIL COMPANY LTD.")
    );
    assert_eq!(
        metadata.well.well.as_deref(),
        Some("ANY ET AL OIL WELL #12")
    );
    assert_eq!(metadata.well.start, Some(1670.0));
    assert_eq!(metadata.well.step, Some(-0.125));
    assert_eq!(metadata.index.name, "DEPT");
    assert_eq!(metadata.index.original_mnemonic, "DEPT");
    assert_eq!(metadata.index.canonical_name, "index");
    assert_eq!(metadata.index.unit.as_deref(), Some("M"));
    assert_eq!(metadata.curves.len(), 8);
    assert_eq!(metadata.parameters.len(), 7);
    assert_eq!(metadata.issue_count, las.issues.len());
    assert!(metadata.other.as_deref().unwrap().contains("logging tools"));

    let rhob = metadata
        .curves
        .iter()
        .find(|curve| curve.name == "RHOB")
        .unwrap();
    assert_eq!(rhob.unit.as_deref(), Some("K/M3"));
    assert_eq!(rhob.storage_kind, CurveStorageKind::Numeric);
    assert!(!rhob.nullable);
    assert_eq!(rhob.alias.mnemonic.as_deref(), Some("bulk_density"));

    let rmf = metadata
        .parameters
        .iter()
        .find(|param| param.name == "RMF")
        .unwrap();
    assert_eq!(rmf.unit.as_deref(), Some("OHMM"));
    assert_eq!(rmf.value.as_deref(), Some("0.216"));
}

#[test]
fn writes_explicit_package_metadata_contract() {
    let las = examples::open("sample.las", &Default::default()).unwrap();
    let package_dir = temp_package_dir("canonical-metadata");

    let package = write_package(&las, &package_dir).unwrap();
    let metadata_text = fs::read_to_string(package_dir.join("metadata.json")).unwrap();
    let metadata: PackageMetadata = serde_json::from_str(&metadata_text).unwrap();

    assert_eq!(metadata.package_version, 1);
    assert_eq!(
        metadata.metadata_schema_version,
        PACKAGE_METADATA_SCHEMA_VERSION
    );
    assert_eq!(metadata.summary.las_version, "1.2");
    assert_eq!(metadata.canonical.index.canonical_name, "index");
    assert_eq!(metadata.curve_columns.len(), metadata.summary.curve_count);
    assert_eq!(
        metadata.raw_sections.curve_mnemonic_case,
        las.curves.mnemonic_case
    );
    assert_eq!(
        metadata
            .raw_sections
            .version
            .get("VERS")
            .unwrap()
            .value
            .display_string(),
        "1.2"
    );

    let rhob_column = metadata
        .curve_columns
        .iter()
        .find(|column| column.name == "RHOB")
        .unwrap();
    assert_eq!(rhob_column.original_mnemonic, "RHOB");
    assert_eq!(rhob_column.unit, "K/M3");
    assert_eq!(rhob_column.description.trim(), "3  BULK DENSITY");

    assert_eq!(package.file().version_info().vers.as_deref(), Some("1.2"));
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
