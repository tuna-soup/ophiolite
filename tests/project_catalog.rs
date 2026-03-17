use lithos_las::{
    AssetBindingInput, AssetKind, AssetStatus, DepthRangeQuery, LithosProject, examples,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn project_import_creates_catalog_entities_and_asset_manifest() {
    let root = temp_project_root("project_import_creates_catalog_entities_and_asset_manifest");
    let mut project = LithosProject::create(&root).unwrap();

    let result = project
        .import_las(examples::path("6038187_v1.2_short.las"), None)
        .unwrap();

    let wells = project.list_wells().unwrap();
    assert_eq!(wells.len(), 1);
    let wellbores = project.list_wellbores(&wells[0].id).unwrap();
    assert_eq!(wellbores.len(), 1);
    let collections = project.list_asset_collections(&wellbores[0].id).unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].asset_kind, AssetKind::Log);

    let assets = project
        .list_assets(&wellbores[0].id, Some(AssetKind::Log))
        .unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].status, AssetStatus::Bound);
    assert!(
        PathBuf::from(&assets[0].package_path)
            .join("metadata.json")
            .exists()
    );
    assert!(
        PathBuf::from(&assets[0].package_path)
            .join("curves.parquet")
            .exists()
    );
    assert!(
        PathBuf::from(&assets[0].package_path)
            .join("asset_manifest.json")
            .exists()
    );
    assert_eq!(
        result.asset.manifest.logical_asset_id,
        collections[0].logical_asset_id
    );

    let reopened = LithosProject::open(&root).unwrap();
    assert!(reopened.catalog_path().exists());
}

#[test]
fn project_reimport_supersedes_storage_instance_but_keeps_logical_asset() {
    let root =
        temp_project_root("project_reimport_supersedes_storage_instance_but_keeps_logical_asset");
    let mut project = LithosProject::create(&root).unwrap();

    let first = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("short-run"))
        .unwrap();
    let second = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("short-run"))
        .unwrap();

    assert_eq!(
        first.collection.logical_asset_id,
        second.collection.logical_asset_id
    );
    assert_ne!(first.asset.id, second.asset.id);
    assert_eq!(
        second.asset.manifest.supersedes,
        Some(first.asset.id.clone())
    );

    let assets = project
        .list_assets(&second.resolution.wellbore_id, Some(AssetKind::Log))
        .unwrap();
    assert_eq!(assets.len(), 2);
    assert!(
        assets
            .iter()
            .any(|asset| asset.status == AssetStatus::Bound)
    );
    assert!(
        assets
            .iter()
            .any(|asset| asset.status == AssetStatus::Superseded)
    );
}

#[test]
fn project_reuses_well_and_wellbore_for_related_log_assets() {
    let root = temp_project_root("project_reuses_well_and_wellbore_for_related_log_assets");
    let mut project = LithosProject::create(&root).unwrap();

    project
        .import_las(examples::path("6038187_v1.2.las"), Some("full-run"))
        .unwrap();
    project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("short-run"))
        .unwrap();

    let wells = project.list_wells().unwrap();
    assert_eq!(wells.len(), 1);
    let wellbores = project.list_wellbores(&wells[0].id).unwrap();
    assert_eq!(wellbores.len(), 1);
    let collections = project.list_asset_collections(&wellbores[0].id).unwrap();
    assert_eq!(collections.len(), 2);
}

#[test]
fn project_supports_non_log_asset_imports_and_cross_asset_queries() {
    let root = temp_project_root("project_supports_non_log_asset_imports_and_cross_asset_queries");
    let mut project = LithosProject::create(&root).unwrap();

    let log = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let trajectory_csv = write_csv(
        &root,
        "trajectory.csv",
        "md,tvd,azimuth,inclination,northing_offset,easting_offset\n100.0,95.0,180,2,0,0\n105.0,100.0,182,3,10,4\n110.0,105.0,184,4,20,8\n",
    );
    let tops_csv = write_csv(
        &root,
        "tops.csv",
        "name,top_depth,base_depth,source,depth_reference\nSand A,101.0,103.0,Interpreter,MD\nSand B,106.0,108.5,Interpreter,MD\n",
    );
    let pressure_csv = write_csv(
        &root,
        "pressure.csv",
        "measured_depth,pressure,phase,test_kind,timestamp\n102.5,4200,oil,RFT,2024-01-01T00:00:00Z\n107.5,4100,water,MDT,2024-01-02T00:00:00Z\n",
    );
    let drilling_csv = write_csv(
        &root,
        "drilling.csv",
        "measured_depth,event_kind,value,unit,timestamp,comment\n101.5,ROP,32,m/h,2024-01-01T01:00:00Z,stable\n108.0,WOB,12,klbf,2024-01-01T02:00:00Z,build section\n",
    );

    let las = examples::open("6038187_v1.2_short.las", &Default::default()).unwrap();
    let well_info = las.well_info();
    let binding = AssetBindingInput {
        well_name: well_info.well.clone().unwrap_or_else(|| "WELL".to_string()),
        wellbore_name: well_info.well.clone().unwrap_or_else(|| "WELL".to_string()),
        uwi: well_info.uwi.clone(),
        api: well_info.api.clone(),
        operator_aliases: well_info.company.into_iter().collect(),
    };

    let trajectory = project
        .import_trajectory_csv(&trajectory_csv, &binding, Some("survey-main"))
        .unwrap();
    let tops = project
        .import_tops_csv(&tops_csv, &binding, Some("tops-main"))
        .unwrap();
    let pressure = project
        .import_pressure_csv(&pressure_csv, &binding, Some("pressure-main"))
        .unwrap();
    let drilling = project
        .import_drilling_csv(&drilling_csv, &binding, Some("drilling-main"))
        .unwrap();

    for asset in [
        &trajectory.asset,
        &tops.asset,
        &pressure.asset,
        &drilling.asset,
    ] {
        let root = PathBuf::from(&asset.package_path);
        assert!(root.join("metadata.json").exists());
        assert!(root.join("data.parquet").exists());
        assert!(root.join("asset_manifest.json").exists());
    }

    let wells = project.list_wells().unwrap();
    assert_eq!(wells.len(), 1);
    let wellbores = project.list_wellbores(&wells[0].id).unwrap();
    assert_eq!(wellbores.len(), 1);
    let collections = project.list_asset_collections(&wellbores[0].id).unwrap();
    assert_eq!(collections.len(), 5);

    let trajectory_rows = project
        .read_trajectory_rows(
            &trajectory.asset.id,
            Some(&DepthRangeQuery {
                depth_min: Some(102.5),
                depth_max: Some(110.0),
            }),
        )
        .unwrap();
    assert_eq!(trajectory_rows.len(), 2);

    let tops_rows = project.read_tops(&tops.asset.id).unwrap();
    assert_eq!(tops_rows.len(), 2);

    let pressure_rows = project
        .read_pressure_observations(
            &pressure.asset.id,
            Some(&DepthRangeQuery {
                depth_min: Some(100.0),
                depth_max: Some(105.0),
            }),
        )
        .unwrap();
    assert_eq!(pressure_rows.len(), 1);

    let drilling_rows = project
        .read_drilling_observations(
            &drilling.asset.id,
            Some(&DepthRangeQuery {
                depth_min: Some(107.0),
                depth_max: Some(109.0),
            }),
        )
        .unwrap();
    assert_eq!(drilling_rows.len(), 1);

    let covering = project
        .assets_covering_depth_range(&log.resolution.wellbore_id, 100.0, 110.0)
        .unwrap();
    assert!(
        covering
            .iter()
            .any(|asset| asset.asset_kind == AssetKind::Log)
    );
    assert!(
        covering
            .iter()
            .any(|asset| asset.asset_kind == AssetKind::Trajectory)
    );
    assert!(
        covering
            .iter()
            .any(|asset| asset.asset_kind == AssetKind::TopSet)
    );
    assert!(
        covering
            .iter()
            .any(|asset| asset.asset_kind == AssetKind::PressureObservation)
    );
    assert!(
        covering
            .iter()
            .any(|asset| asset.asset_kind == AssetKind::DrillingObservation)
    );
}

fn temp_project_root(label: &str) -> PathBuf {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("lithos_{label}_{nanos}_{unique}"));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    root
}

fn write_csv(root: &std::path::Path, name: &str, contents: &str) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, contents).unwrap();
    path
}
