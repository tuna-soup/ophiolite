use ophiolite::{
    DepthRangeQuery, OphioliteProject, ProjectSurveyMapRequestDto,
    SECTION_WELL_OVERLAY_CONTRACT_VERSION, SURVEY_MAP_CONTRACT_VERSION, SectionAxis,
    SectionWellOverlayDomainDto, SectionWellOverlayRequestDto, WELL_PANEL_CONTRACT_VERSION,
    WellPanelRequestDto, generate_synthetic_project_fixture,
};
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
    assert!(fixture.sources.survey_store.exists());

    let project = OphioliteProject::open(&fixture.project_root).unwrap();
    let wells = project.list_wells().unwrap();
    assert_eq!(wells.len(), 1);
    let wellbores = project.list_wellbores(&wells[0].id).unwrap();
    assert_eq!(wellbores.len(), 1);
    let collections = project.list_asset_collections(&wellbores[0].id).unwrap();
    assert_eq!(collections.len(), 6);
    let assets = project.list_assets(&wellbores[0].id, None).unwrap();
    assert_eq!(assets.len(), 6);

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

    let inventory = project.project_well_overlay_inventory().unwrap();
    assert_eq!(inventory.surveys.len(), 1);
    assert_eq!(inventory.surveys[0].asset_id, fixture.asset_ids.survey);

    let trajectory = project
        .resolve_wellbore_trajectory(&fixture.wellbore_id)
        .unwrap();
    assert_eq!(
        trajectory.source_asset_ids,
        vec![fixture.asset_ids.trajectory.0.clone()]
    );
    assert_eq!(trajectory.stations.len(), 5);
    assert!(
        trajectory
            .stations
            .iter()
            .all(|station| station.absolute_xy.is_some())
    );

    let panel = project
        .resolve_well_panel_source(&WellPanelRequestDto {
            schema_version: WELL_PANEL_CONTRACT_VERSION,
            wellbore_ids: vec![fixture.wellbore_id.0.clone()],
            depth_min: None,
            depth_max: None,
        })
        .unwrap();
    assert_eq!(panel.wells.len(), 1);
    assert_eq!(panel.wells[0].wellbore_id, fixture.wellbore_id.0);
    assert_eq!(panel.wells[0].trajectories.len(), 1);
    assert_eq!(panel.wells[0].top_sets.len(), 1);
    assert_eq!(panel.wells[0].pressure_observations.len(), 1);
    assert_eq!(panel.wells[0].drilling_observations.len(), 1);
    let panel_semantics = panel.wells[0]
        .logs
        .iter()
        .map(|curve| curve.semantic_type.as_str())
        .collect::<Vec<_>>();
    assert!(panel_semantics.contains(&"BulkDensity"));
    assert!(panel_semantics.contains(&"Sonic"));
    assert!(panel_semantics.contains(&"ShearSonic"));

    let survey_map = project
        .resolve_survey_map_source(&ProjectSurveyMapRequestDto {
            schema_version: SURVEY_MAP_CONTRACT_VERSION,
            survey_asset_ids: vec![fixture.asset_ids.survey.0.clone()],
            wellbore_ids: vec![fixture.wellbore_id.0.clone()],
            display_coordinate_reference_id: "EPSG:23031".to_string(),
        })
        .unwrap();
    assert_eq!(survey_map.surveys.len(), 1);
    assert_eq!(survey_map.wells.len(), 1);
    assert!(
        survey_map.surveys[0]
            .native_spatial
            .grid_transform
            .is_some()
    );
    assert_eq!(survey_map.wells[0].trajectories.len(), 1);
    assert_eq!(survey_map.wells[0].trajectories[0].rows.len(), 5);

    let overlays = project
        .resolve_section_well_overlays(&SectionWellOverlayRequestDto {
            schema_version: SECTION_WELL_OVERLAY_CONTRACT_VERSION,
            project_root: fixture.project_root.to_string_lossy().into_owned(),
            survey_asset_id: fixture.asset_ids.survey.0.clone(),
            wellbore_ids: vec![fixture.wellbore_id.0.clone()],
            axis: SectionAxis::Inline,
            index: 1001,
            tolerance_m: None,
            display_domain: SectionWellOverlayDomainDto::Depth,
            active_well_model_ids: Vec::new(),
        })
        .unwrap();
    assert_eq!(overlays.overlays.len(), 1);
    assert!(
        overlays.overlays[0]
            .segments
            .iter()
            .any(|segment| !segment.samples.is_empty())
    );
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
            .project_well_overlay_inventory()
            .unwrap()
            .surveys
            .len(),
        1
    );
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
