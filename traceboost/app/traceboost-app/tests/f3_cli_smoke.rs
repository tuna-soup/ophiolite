use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn f3_store_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sandbox/f3_dataset_regularized.tbvol")
}

fn load_horizon_entry(store: &Path, horizon_id: &str) -> Value {
    let manifest_path = store.join("horizons").join("manifest.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read horizons manifest"))
            .expect("parse horizons manifest");
    manifest["horizons"]
        .as_array()
        .expect("horizon manifest entries")
        .iter()
        .find(|entry| entry["id"].as_str() == Some(horizon_id))
        .cloned()
        .expect("expected horizon entry in manifest")
}

#[test]
fn f3_cli_convert_horizon_domain_materializes_expected_ids() {
    let store = f3_store_path();
    if !store.exists() {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_traceboost-app");
    let transform_id = "f3-paired-horizon-survey-transform";

    let output = Command::new(binary)
        .arg("convert-horizon-domain")
        .arg(&store)
        .arg("--source-horizon-id")
        .arg("horizon_01_twt_ms")
        .arg("--transform-id")
        .arg(transform_id)
        .arg("--target-domain")
        .arg("depth")
        .arg("--output-id")
        .arg("horizon_01_twt_ms-derived_depth_m")
        .output()
        .expect("run CLI time-to-depth conversion");
    assert!(
        output.status.success(),
        "CLI time-to-depth conversion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).expect("parse CLI depth response");
    assert_eq!(
        response["id"].as_str(),
        Some("horizon_01_twt_ms-derived_depth_m")
    );
    assert_eq!(response["vertical_domain"].as_str(), Some("depth"));
    assert_eq!(response["vertical_unit"].as_str(), Some("m"));

    let output = Command::new(binary)
        .arg("convert-horizon-domain")
        .arg(&store)
        .arg("--source-horizon-id")
        .arg("horizon_01_depth_m")
        .arg("--transform-id")
        .arg(transform_id)
        .arg("--target-domain")
        .arg("time")
        .arg("--output-id")
        .arg("horizon_01_depth_m-derived_twt_ms")
        .output()
        .expect("run CLI depth-to-time conversion");
    assert!(
        output.status.success(),
        "CLI depth-to-time conversion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).expect("parse CLI time response");
    assert_eq!(
        response["id"].as_str(),
        Some("horizon_01_depth_m-derived_twt_ms")
    );
    assert_eq!(response["vertical_domain"].as_str(), Some("time"));
    assert_eq!(response["vertical_unit"].as_str(), Some("ms"));

    let depth_entry = load_horizon_entry(&store, "horizon_01_twt_ms-derived_depth_m");
    let time_entry = load_horizon_entry(&store, "horizon_01_depth_m-derived_twt_ms");

    for entry in [&depth_entry, &time_entry] {
        let values_file = entry["values_file"]
            .as_str()
            .expect("stored horizon values file");
        let validity_file = entry["validity_file"]
            .as_str()
            .expect("stored horizon validity file");
        assert!(store.join("horizons").join(values_file).exists());
        assert!(store.join("horizons").join(validity_file).exists());
        assert!(
            entry["source_path"]
                .as_str()
                .expect("stored source path")
                .starts_with("derived://horizon-conversion/")
        );
    }
}
