use lithos_las::{CurveWindow, StoredLasAsset, import_las_file, write_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("lithos_las_{name}_{nanos}"))
}

#[test]
fn imports_minimal_las_20_file() {
    let asset =
        import_las_file(repo_root().join("test_data/logs/2.0/sample_2.0_minimal.las")).unwrap();

    assert_eq!(asset.summary.las_version, "2.0");
    assert_eq!(asset.summary.wrap_mode.to_ascii_uppercase(), "NO");
    assert_eq!(asset.index.raw_mnemonic.to_ascii_uppercase(), "DEPT");
    assert_eq!(asset.summary.curve_count, asset.curves.len());
    assert!(asset.summary.row_count > 0);
}

#[test]
fn imports_wrapped_las_file() {
    let asset = import_las_file(repo_root().join("test_data/logs/1.2/sample_wrapped.las")).unwrap();

    assert_eq!(asset.summary.wrap_mode.to_ascii_uppercase(), "YES");
    assert!(asset.summary.row_count > 0);
    assert_eq!(asset.curves[0].samples.len(), asset.summary.row_count);
    assert!(asset.read_index(Some(CurveWindow::new(0, 3))).len() <= 3);
}

#[test]
fn bundle_round_trip_preserves_curve_queries() {
    let asset = import_las_file(repo_root().join("test_data/logs/6038187_v1.2_short.las")).unwrap();
    let bundle_dir = unique_temp_dir("bundle_roundtrip");
    let stored = write_bundle(&asset, &bundle_dir).unwrap();

    let metadata = stored.get_curve_metadata("DEPT").unwrap();
    assert!(metadata.is_index);

    let reloaded = StoredLasAsset::open(&bundle_dir).unwrap();
    let depth = reloaded
        .read_curve("DEPT", Some(CurveWindow::new(0, 5)))
        .unwrap();
    let gamma = reloaded
        .read_curve("GAMN", Some(CurveWindow::new(0, 5)))
        .unwrap();

    assert_eq!(depth.len(), 5);
    assert_eq!(gamma.len(), 5);
    assert_eq!(reloaded.summary().row_count, asset.summary.row_count);

    fs::remove_dir_all(bundle_dir).unwrap();
}
