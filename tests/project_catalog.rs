use lithos_las::{AssetKind, AssetStatus, LithosProject, examples};
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
