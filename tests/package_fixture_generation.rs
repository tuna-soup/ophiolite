use lithos_cli::generate_fixture_packages;
use lithos_las::open_package_summary;

#[test]
#[ignore = "generates inspectable packages under test_data/logs/packages"]
fn generates_packages_for_log_fixtures_under_test_data() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input_root = repo_root.join("test_data").join("logs");
    let output_root = input_root.join("packages");

    let generated = generate_fixture_packages(&input_root, &output_root).unwrap();

    assert!(!generated.is_empty());
    assert!(
        generated
            .iter()
            .any(|path| path.ends_with("6038187_v1.2_short.laspkg"))
    );
    assert!(
        generated
            .iter()
            .any(|path| path.ends_with("1.2\\sample_big.laspkg"))
    );

    for package_root in generated {
        assert!(
            package_root.join("metadata.json").exists(),
            "{package_root:?}"
        );
        assert!(
            package_root.join("curves.parquet").exists(),
            "{package_root:?}"
        );
        open_package_summary(&package_root).unwrap();
    }
}
