use ophiolite::{DepthRangeQuery, OphioliteProject, generate_synthetic_project_fixture};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn generates_coherent_synthetic_project_fixture_in_temp_directory() {
    let root = temp_fixture_root("synthetic_project_fixture");
    let fixture = generate_synthetic_project_fixture(&root).unwrap();

    assert!(fixture.project_root.join("ophiolite-project.json").exists());
    assert!(fixture.project_root.join("catalog.sqlite").exists());
    assert!(fixture.sources.log_las.exists());
    assert!(fixture.sources.trajectory_csv.exists());
    assert!(fixture.sources.tops_csv.exists());
    assert!(fixture.sources.pressure_csv.exists());
    assert!(fixture.sources.drilling_csv.exists());

    let project = OphioliteProject::open(&fixture.project_root).unwrap();
    let wells = project.list_wells().unwrap();
    assert_eq!(wells.len(), 1);
    let wellbores = project.list_wellbores(&wells[0].id).unwrap();
    assert_eq!(wellbores.len(), 1);
    let collections = project.list_asset_collections(&wellbores[0].id).unwrap();
    assert_eq!(collections.len(), 5);
    let assets = project.list_assets(&wellbores[0].id, None).unwrap();
    assert_eq!(assets.len(), 5);

    let trajectory_rows = project
        .read_trajectory_rows(
            &fixture.asset_ids.trajectory,
            Some(&DepthRangeQuery {
                depth_min: Some(1010.0),
                depth_max: Some(1080.0),
            }),
        )
        .unwrap();
    assert_eq!(trajectory_rows.len(), 3);

    let tops_rows = project.read_tops(&fixture.asset_ids.tops).unwrap();
    assert_eq!(tops_rows.len(), 3);

    let pressure_rows = project
        .read_pressure_observations(
            &fixture.asset_ids.pressure,
            Some(&DepthRangeQuery {
                depth_min: Some(1030.0),
                depth_max: Some(1050.0),
            }),
        )
        .unwrap();
    assert_eq!(pressure_rows.len(), 1);

    let drilling_rows = project
        .read_drilling_observations(
            &fixture.asset_ids.drilling,
            Some(&DepthRangeQuery {
                depth_min: Some(1060.0),
                depth_max: Some(1080.0),
            }),
        )
        .unwrap();
    assert_eq!(drilling_rows.len(), 1);

    let covering = project
        .assets_covering_depth_range(&fixture.wellbore_id, 1010.0, 1070.0)
        .unwrap();
    assert_eq!(covering.len(), 5);
}

#[test]
#[ignore = "generates an inspectable synthetic multi-asset project under test_data/projects"]
fn generates_synthetic_project_fixture_under_test_data() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_root = repo_root
        .join("test_data")
        .join("projects")
        .join("synthetic_well_project");

    let fixture = generate_synthetic_project_fixture(&output_root).unwrap();
    let project = OphioliteProject::open(&fixture.project_root).unwrap();

    assert!(fixture.project_root.join("assets").exists());
    assert!(fixture.project_root.join("sources").exists());
    assert_eq!(project.list_wells().unwrap().len(), 1);
    assert_eq!(
        project
            .assets_covering_depth_range(&fixture.wellbore_id, 1005.0, 1075.0)
            .unwrap()
            .len(),
        5
    );
}

fn temp_fixture_root(label: &str) -> PathBuf {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("ophiolite_{label}_{nanos}_{unique}"));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    root
}
