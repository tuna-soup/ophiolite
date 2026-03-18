use lithos_las::{
    AssetBindingInput, AssetKind, AssetStatus, ComputeAvailability, ComputeParameterValue,
    CurveSemanticType, DepthRangeQuery, LithosProject, ProjectComputeRunRequest, examples,
    import_las_asset,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn project_import_creates_catalog_entities_and_asset_manifest() {
    let root = temp_project_root("project_import_creates_catalog_entities_and_asset_manifest");
    let mut project = LithosProject::create(&root).unwrap();

    let result =
        import_las_asset(&mut project, examples::path("6038187_v1.2_short.las"), None).unwrap();

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
fn project_exposes_summary_api_for_apps() {
    let root = temp_project_root("project_exposes_summary_api_for_apps");
    let mut project = LithosProject::create(&root).unwrap();

    project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let summary = project.summary().unwrap();
    assert_eq!(summary.well_count, 1);
    assert_eq!(summary.wellbore_count, 1);
    assert_eq!(summary.asset_collection_count, 1);
    assert_eq!(summary.asset_count, 1);

    let well_summaries = project.well_summaries().unwrap();
    assert_eq!(well_summaries.len(), 1);
    assert_eq!(well_summaries[0].wellbore_count, 1);
    assert_eq!(well_summaries[0].asset_count, 1);

    let wellbore_summaries = project
        .wellbore_summaries(&well_summaries[0].well.id)
        .unwrap();
    assert_eq!(wellbore_summaries.len(), 1);
    assert_eq!(wellbore_summaries[0].collection_count, 1);
    assert_eq!(wellbore_summaries[0].asset_count, 1);

    let collection_summaries = project
        .asset_collection_summaries(&wellbore_summaries[0].wellbore.id)
        .unwrap();
    assert_eq!(collection_summaries.len(), 1);
    assert_eq!(collection_summaries[0].asset_count, 1);
    assert!(collection_summaries[0].current_asset_id.is_some());

    let asset_summaries = project
        .asset_summaries(&wellbore_summaries[0].wellbore.id, Some(AssetKind::Log))
        .unwrap();
    assert_eq!(asset_summaries.len(), 1);
    assert!(asset_summaries[0].is_current);
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

    let revisions = project.asset_revisions(&second.asset.id).unwrap();
    assert_eq!(revisions.len(), 1);
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

#[test]
fn structured_asset_edits_create_revision_history() {
    let root = temp_project_root("structured_asset_edits_create_revision_history");
    let mut project = LithosProject::create(&root).unwrap();
    let csv_path = write_csv(
        &root,
        "tops.csv",
        "name,top_depth,base_depth,source,depth_reference\nTop A,100,101,interp,MD\n",
    );
    let binding = AssetBindingInput {
        well_name: "Well A".to_string(),
        wellbore_name: "WB-1".to_string(),
        uwi: Some("UWI-1".to_string()),
        api: None,
        operator_aliases: Vec::new(),
    };
    let imported = project
        .import_tops_csv(&csv_path, &binding, Some("tops"))
        .unwrap();

    let initial_revisions = project.asset_revisions(&imported.asset.id).unwrap();
    assert_eq!(initial_revisions.len(), 1);

    let updated_rows = vec![lithos_las::TopRow {
        name: "Top B".to_string(),
        top_depth: 110.0,
        base_depth: Some(111.0),
        source: Some("interp".to_string()),
        depth_reference: Some("MD".to_string()),
    }];
    project
        .overwrite_tops_asset(&imported.asset.id, &updated_rows)
        .unwrap();

    let revisions = project.asset_revisions(&imported.asset.id).unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(revisions[1].parent_revision_id.is_some());
}

#[test]
fn project_can_sync_log_asset_head_after_package_edits() {
    let root = temp_project_root("project_can_sync_log_asset_head_after_package_edits");
    let mut project = LithosProject::create(&root).unwrap();
    let imported = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let mut session = lithos_las::open_package(&imported.asset.package_path).unwrap();
    let curve_name = session
        .file()
        .curve_names()
        .into_iter()
        .find(|name| name != &session.file().index.curve_id)
        .unwrap();
    let original = session.file().curve(&curve_name).unwrap().clone();
    let mut data = original.data.clone();
    data[0] = lithos_las::LasValue::Number(999.0);
    session
        .apply_curve_edit(&lithos_las::CurveEditRequest::Upsert(
            lithos_las::CurveUpdateRequest {
                mnemonic: curve_name,
                original_mnemonic: Some(original.original_mnemonic.clone()),
                unit: original.unit.clone(),
                header_value: original.value.clone(),
                description: original.description.clone(),
                data,
            },
        ))
        .unwrap();
    session.save_checked().unwrap();

    let revision = project
        .sync_log_asset_head_revision(&imported.asset.id)
        .unwrap();
    let revisions = project.asset_revisions(&imported.asset.id).unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions.last().unwrap().revision_id, revision.revision_id);
}

#[test]
fn project_lists_type_safe_compute_catalog_for_log_assets() {
    let root = temp_project_root("project_lists_type_safe_compute_catalog_for_log_assets");
    let mut project = LithosProject::create(&root).unwrap();

    let log = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let catalog = project.list_compute_catalog(&log.asset.id).unwrap();
    let vshale = catalog
        .functions
        .iter()
        .find(|entry| entry.metadata.id == "petro:vshale_linear")
        .unwrap();
    assert!(matches!(
        vshale.availability,
        ComputeAvailability::Available
    ));
    assert!(vshale.binding_candidates.iter().any(|candidate| {
        candidate.parameter_name == "gr_curve"
            && candidate
                .matches
                .iter()
                .any(|curve| curve.semantic_type == CurveSemanticType::GammaRay)
    }));

    let acoustic_impedance = catalog
        .functions
        .iter()
        .find(|entry| entry.metadata.id == "rock_physics:acoustic_impedance")
        .unwrap();
    assert!(matches!(
        acoustic_impedance.availability,
        ComputeAvailability::Unavailable { .. }
    ));
}

#[test]
fn project_runs_compute_and_persists_derived_log_assets() {
    let root = temp_project_root("project_runs_compute_and_persists_derived_log_assets");
    let mut project = LithosProject::create(&root).unwrap();

    let log = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let semantics = project.log_curve_semantics(&log.asset.id).unwrap();
    let gr_curve = semantics
        .iter()
        .find(|curve| curve.semantic_type == CurveSemanticType::GammaRay)
        .unwrap();

    let mut bindings = BTreeMap::new();
    bindings.insert("gr_curve".to_string(), gr_curve.curve_name.clone());
    let mut parameters = BTreeMap::new();
    parameters.insert("gr_min".to_string(), ComputeParameterValue::Number(30.0));
    parameters.insert("gr_max".to_string(), ComputeParameterValue::Number(120.0));

    let result = project
        .run_compute(&ProjectComputeRunRequest {
            source_asset_id: log.asset.id.clone(),
            function_id: "petro:vshale_linear".to_string(),
            curve_bindings: bindings.clone(),
            parameters: parameters.clone(),
            output_collection_name: None,
            output_mnemonic: None,
        })
        .unwrap();

    assert_eq!(
        result.asset.manifest.derived_from,
        Some(log.asset.logical_asset_id.clone())
    );
    assert!(result.asset.manifest.compute_manifest.is_some());
    assert_eq!(
        result
            .asset
            .manifest
            .curve_semantics
            .iter()
            .find(|curve| curve.curve_name == "VSH_LIN")
            .unwrap()
            .semantic_type,
        CurveSemanticType::VShale
    );

    let package_root = PathBuf::from(&result.asset.package_path);
    assert!(package_root.join("metadata.json").exists());
    assert!(package_root.join("curves.parquet").exists());
    assert!(package_root.join("asset_manifest.json").exists());

    let assets = project
        .list_assets(&log.resolution.wellbore_id, Some(AssetKind::Log))
        .unwrap();
    assert_eq!(assets.len(), 2);
    assert!(assets.iter().any(|asset| asset.id == result.asset.id));

    let rerun = project
        .run_compute(&ProjectComputeRunRequest {
            source_asset_id: log.asset.id.clone(),
            function_id: "petro:vshale_linear".to_string(),
            curve_bindings: bindings,
            parameters,
            output_collection_name: Some(result.collection.name.clone()),
            output_mnemonic: Some("VSH_LIN".to_string()),
        })
        .unwrap();

    let asset_summaries = project
        .asset_summaries(&log.resolution.wellbore_id, Some(AssetKind::Log))
        .unwrap();
    let derived_entries = asset_summaries
        .iter()
        .filter(|summary| summary.asset.collection_id == rerun.collection.id)
        .collect::<Vec<_>>();
    assert_eq!(derived_entries.len(), 2);
    assert!(derived_entries.iter().any(|summary| summary.is_current));
    assert!(
        derived_entries
            .iter()
            .any(|summary| summary.asset.status == AssetStatus::Superseded)
    );
}

#[test]
fn project_runs_structured_compute_and_persists_derived_assets() {
    let root = temp_project_root("project_runs_structured_compute_and_persists_derived_assets");
    let mut project = LithosProject::create(&root).unwrap();

    let trajectory_csv = write_csv(
        &root,
        "trajectory_compute.csv",
        "md,tvd,azimuth,inclination\n1000,950,-10,2.0\n1010,958,370,4.0\n1020,966,725,8.0\n",
    );
    let binding = AssetBindingInput {
        well_name: "Well Alpha".to_string(),
        wellbore_name: "Well Alpha".to_string(),
        uwi: Some("UWI-001".to_string()),
        api: None,
        operator_aliases: vec!["Lithos".to_string()],
    };

    let trajectory = project
        .import_trajectory_csv(&trajectory_csv, &binding, Some("survey-main"))
        .unwrap();

    let catalog = project.list_compute_catalog(&trajectory.asset.id).unwrap();
    assert!(
        catalog
            .functions
            .iter()
            .any(|entry| entry.metadata.id == "trajectory:normalize_azimuth")
    );

    let result = project
        .run_compute(&ProjectComputeRunRequest {
            source_asset_id: trajectory.asset.id.clone(),
            function_id: "trajectory:normalize_azimuth".to_string(),
            curve_bindings: BTreeMap::new(),
            parameters: BTreeMap::new(),
            output_collection_name: None,
            output_mnemonic: None,
        })
        .unwrap();

    assert_eq!(result.asset.asset_kind, AssetKind::Trajectory);
    assert_eq!(
        result.asset.manifest.derived_from,
        Some(trajectory.asset.logical_asset_id.clone())
    );
    assert!(result.asset.manifest.compute_manifest.is_some());

    let rows = project
        .read_trajectory_rows(&result.asset.id, None)
        .unwrap();
    assert_eq!(rows[0].azimuth_deg, Some(350.0));
    assert_eq!(rows[1].azimuth_deg, Some(10.0));
    assert_eq!(rows[2].azimuth_deg, Some(5.0));

    let summaries = project
        .asset_summaries(
            &trajectory.resolution.wellbore_id,
            Some(AssetKind::Trajectory),
        )
        .unwrap();
    assert_eq!(
        summaries
            .iter()
            .filter(|summary| summary.asset.collection_id == result.collection.id)
            .count(),
        1
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
