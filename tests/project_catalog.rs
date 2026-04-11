use ndarray::Array3;
use ophiolite::{
    AssetBindingInput, AssetKind, AssetStatus, ComputeAvailability, ComputeParameterValue,
    CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
    CurveSemanticType, DatasetKind, DepthRangeQuery, GeometryProvenance, HeaderFieldSpec,
    OphioliteProject, ProjectComputeRunRequest, ProjectedPoint2, ProjectedPolygon2,
    ProjectedVector2, SourceIdentity, SurveyGridTransform, SurveyMapRequestDto,
    SurveyMapSpatialAvailabilityDto, SurveyMapTransformStatusDto, SurveySpatialAvailability,
    SurveySpatialDescriptor, TbvolManifest, VolumeAxes, VolumeMetadata, WellAzimuthReferenceKind,
    WellPanelRequestDto, WellboreAnchorKind, WellboreAnchorReference, WellboreGeometry,
    create_tbvol_store, examples, import_las_asset,
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
    let mut project = OphioliteProject::create(&root).unwrap();

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

    let reopened = OphioliteProject::open(&root).unwrap();
    assert!(reopened.catalog_path().exists());
}

#[test]
fn project_exposes_summary_api_for_apps() {
    let root = temp_project_root("project_exposes_summary_api_for_apps");
    let mut project = OphioliteProject::create(&root).unwrap();

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
    let mut project = OphioliteProject::create(&root).unwrap();

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
    let mut project = OphioliteProject::create(&root).unwrap();

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
fn project_persists_wellbore_geometry_across_reopen() {
    let root = temp_project_root("project_persists_wellbore_geometry_across_reopen");
    let mut project = OphioliteProject::create(&root).unwrap();

    project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let well = project.list_wells().unwrap().remove(0);
    let wellbore = project.list_wellbores(&well.id).unwrap().remove(0);
    let geometry = WellboreGeometry {
        anchor: Some(WellboreAnchorReference {
            kind: WellboreAnchorKind::Surface,
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: Some("EPSG:32631".to_string()),
                name: Some("UTM 31N".to_string()),
                geodetic_datum: Some("WGS 84".to_string()),
                unit: Some("m".to_string()),
            }),
            location: ProjectedPoint2 {
                x: 512345.0,
                y: 6123456.0,
            },
            parent_wellbore_id: None,
            parent_measured_depth_m: None,
            notes: vec!["surveyed surface location".to_string()],
        }),
        vertical_datum: Some("KellyBushing".to_string()),
        depth_unit: Some("m".to_string()),
        azimuth_reference: WellAzimuthReferenceKind::GridNorth,
        notes: vec!["authoritative anchor".to_string()],
    };

    let updated = project
        .set_wellbore_geometry(&wellbore.id, Some(geometry.clone()))
        .unwrap();
    assert_eq!(updated.geometry, Some(geometry.clone()));

    let reopened = OphioliteProject::open(&root).unwrap();
    let reopened_wellbore = reopened.list_wellbores(&well.id).unwrap().remove(0);
    assert_eq!(reopened_wellbore.geometry, Some(geometry));
}

#[test]
fn project_supports_non_log_asset_imports_and_cross_asset_queries() {
    let root = temp_project_root("project_supports_non_log_asset_imports_and_cross_asset_queries");
    let mut project = OphioliteProject::create(&root).unwrap();

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
fn project_resolves_well_panel_source_dto_for_frontend_workflows() {
    let root = temp_project_root("project_resolves_well_panel_source_dto_for_frontend_workflows");
    let mut project = OphioliteProject::create(&root).unwrap();

    let log = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let trajectory_csv = write_csv(
        &root,
        "trajectory_panel.csv",
        "md,tvd,azimuth,inclination,northing_offset,easting_offset\n100.0,95.0,180,2,0,0\n105.0,100.0,182,3,10,4\n110.0,105.0,184,4,20,8\n",
    );
    let tops_csv = write_csv(
        &root,
        "tops_panel.csv",
        "name,top_depth,base_depth,source,depth_reference\nSand A,101.0,103.0,Interpreter,MD\nSand B,106.0,108.5,Interpreter,MD\n",
    );
    let pressure_csv = write_csv(
        &root,
        "pressure_panel.csv",
        "measured_depth,pressure,phase,test_kind,timestamp\n102.5,4200,oil,RFT,2024-01-01T00:00:00Z\n107.5,4100,water,MDT,2024-01-02T00:00:00Z\n",
    );
    let drilling_csv = write_csv(
        &root,
        "drilling_panel.csv",
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

    project
        .import_trajectory_csv(&trajectory_csv, &binding, Some("survey-main"))
        .unwrap();
    project
        .import_tops_csv(&tops_csv, &binding, Some("tops-main"))
        .unwrap();
    project
        .import_pressure_csv(&pressure_csv, &binding, Some("pressure-main"))
        .unwrap();
    project
        .import_drilling_csv(&drilling_csv, &binding, Some("drilling-main"))
        .unwrap();

    let resolved = project
        .resolve_well_panel_source(&WellPanelRequestDto {
            schema_version: 1,
            wellbore_ids: vec![log.resolution.wellbore_id.0.clone()],
            depth_min: None,
            depth_max: None,
        })
        .unwrap();

    assert_eq!(resolved.schema_version, 1);
    assert_eq!(resolved.wells.len(), 1);

    let well = &resolved.wells[0];
    assert_eq!(well.wellbore_id, log.resolution.wellbore_id.0);
    assert!(!well.logs.is_empty());
    assert_eq!(well.trajectories.len(), 1);
    assert_eq!(well.top_sets.len(), 1);
    assert_eq!(well.pressure_observations.len(), 1);
    assert_eq!(well.drilling_observations.len(), 1);
    assert!(!well.panel_depth_mapping.is_empty());
    assert_eq!(well.top_sets[0].rows.len(), 2);
    assert_eq!(well.pressure_observations[0].rows.len(), 2);
    assert_eq!(well.drilling_observations[0].rows.len(), 2);
}

#[test]
fn project_imports_seismic_trace_data_store_and_tracks_it_in_catalog() {
    let root =
        temp_project_root("project_imports_seismic_trace_data_store_and_tracks_it_in_catalog");
    let source_root = root.join("source").join("survey.tbvol");
    let manifest = sample_store_manifest();
    let data = Array3::from_shape_fn((2, 3, 4), |(iline, xline, sample)| {
        iline as f32 * 100.0 + xline as f32 * 10.0 + sample as f32
    });
    create_tbvol_store(&source_root, manifest, &data, None).unwrap();

    let mut project = OphioliteProject::create(root.join("project")).unwrap();
    let binding = AssetBindingInput {
        well_name: "Well Seis".to_string(),
        wellbore_name: "Well Seis".to_string(),
        uwi: Some("SEIS-UWI-001".to_string()),
        api: None,
        operator_aliases: vec!["Ophiolite".to_string()],
    };

    let imported = project
        .import_seismic_trace_data_store(&source_root, &binding, Some("survey-main"))
        .unwrap();

    assert_eq!(imported.asset.asset_kind, AssetKind::SeismicTraceData);
    assert_eq!(imported.asset.manifest.bulk_data_descriptors.len(), 1);
    assert_eq!(
        imported.asset.manifest.bulk_data_descriptors[0].relative_path,
        "store"
    );

    let assets = project
        .list_assets(
            &imported.resolution.wellbore_id,
            Some(AssetKind::SeismicTraceData),
        )
        .unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].status, AssetStatus::Bound);

    let package_root = PathBuf::from(&assets[0].package_path);
    assert!(package_root.join("metadata.json").exists());
    assert!(package_root.join("asset_manifest.json").exists());
    assert!(package_root.join("store").join("manifest.json").exists());

    let descriptor = ophiolite::describe_store(package_root.join("store")).unwrap();
    assert_eq!(descriptor.shape, [2, 3, 4]);
    assert_eq!(descriptor.chunk_shape, [2, 3, 4]);

    let metadata_json = fs::read_to_string(package_root.join("metadata.json")).unwrap();
    assert!(metadata_json.contains("\"family\": \"Volume\""));
    assert!(metadata_json.contains("\"label\": \"survey\""));
    assert!(metadata_json.contains("\"trace_data_descriptor\""));
    assert!(metadata_json.contains("\"layout\": \"PostStack3D\""));
}

#[test]
fn project_resolves_survey_map_source_with_explicit_spatial_gaps() {
    let root = temp_project_root("project_resolves_survey_map_source_with_explicit_spatial_gaps");
    let source_root = root.join("source").join("survey.tbvol");
    let manifest = sample_store_manifest();
    let data = Array3::from_shape_fn((2, 3, 4), |(iline, xline, sample)| {
        iline as f32 * 100.0 + xline as f32 * 10.0 + sample as f32
    });
    create_tbvol_store(&source_root, manifest, &data, None).unwrap();

    let trajectory_csv = write_csv(
        &root,
        "trajectory_map.csv",
        "md,tvd,azimuth,inclination,northing_offset,easting_offset\n100.0,95.0,180,2,0,0\n105.0,100.0,182,3,10,4\n110.0,105.0,184,4,20,8\n",
    );

    let mut project = OphioliteProject::create(root.join("project")).unwrap();
    let binding = AssetBindingInput {
        well_name: "Map Well".to_string(),
        wellbore_name: "Map WB".to_string(),
        uwi: Some("MAP-UWI-001".to_string()),
        api: None,
        operator_aliases: vec!["Ophiolite".to_string()],
    };

    let seismic = project
        .import_seismic_trace_data_store(&source_root, &binding, Some("survey-main"))
        .unwrap();
    project
        .import_trajectory_csv(&trajectory_csv, &binding, Some("survey-main"))
        .unwrap();

    let resolved = project
        .resolve_survey_map_source(&SurveyMapRequestDto {
            schema_version: 1,
            survey_asset_ids: vec![seismic.asset.id.0.clone()],
            wellbore_ids: vec![seismic.resolution.wellbore_id.0.clone()],
            display_coordinate_reference_id: None,
        })
        .unwrap();

    assert_eq!(resolved.schema_version, 1);
    assert_eq!(resolved.surveys.len(), 1);
    assert_eq!(resolved.wells.len(), 1);
    assert_eq!(resolved.surveys[0].index_grid.inline_axis.count, 2);
    assert_eq!(resolved.surveys[0].index_grid.xline_axis.count, 3);
    assert!(resolved.surveys[0].native_spatial.footprint.is_none());
    assert!(resolved.surveys[0].display_spatial.is_none());
    assert!(matches!(
        resolved.surveys[0].native_spatial.availability,
        SurveyMapSpatialAvailabilityDto::Unavailable
    ));
    assert!(matches!(
        resolved.surveys[0].transform_status,
        ophiolite::SurveyMapTransformStatusDto::NativeOnly
    ));
    assert!(matches!(
        resolved.surveys[0].transform_diagnostics.policy,
        ophiolite::SurveyMapTransformPolicyDto::BestAvailable
    ));
    assert!(
        resolved.surveys[0]
            .transform_diagnostics
            .target_coordinate_reference_id
            .is_none()
    );
    assert!(!resolved.surveys[0].native_spatial.notes.is_empty());
    assert!(resolved.wells[0].surface_location.is_none());
    assert_eq!(resolved.wells[0].trajectories.len(), 1);
    assert_eq!(resolved.wells[0].trajectories[0].rows.len(), 3);
}

#[test]
fn project_resolves_survey_map_source_with_proj_display_transform_and_cache() {
    let root = temp_project_root(
        "project_resolves_survey_map_source_with_proj_display_transform_and_cache",
    );
    let source_root = root.join("source").join("survey-4326.tbvol");
    let mut manifest = sample_store_manifest();
    let native_coordinate_reference = CoordinateReferenceDescriptor {
        id: Some("EPSG:4326".to_string()),
        name: Some("WGS 84".to_string()),
        geodetic_datum: Some("WGS84".to_string()),
        unit: Some("degree".to_string()),
    };
    manifest.volume.coordinate_reference_binding = Some(CoordinateReferenceBinding {
        detected: Some(native_coordinate_reference.clone()),
        effective: Some(native_coordinate_reference.clone()),
        source: CoordinateReferenceSource::Header,
        notes: Vec::new(),
    });
    manifest.volume.spatial = Some(SurveySpatialDescriptor {
        coordinate_reference: Some(native_coordinate_reference),
        grid_transform: Some(SurveyGridTransform {
            origin: ProjectedPoint2 { x: 4.0, y: 52.0 },
            inline_basis: ProjectedVector2 { x: 0.05, y: 0.0 },
            xline_basis: ProjectedVector2 { x: 0.0, y: 0.05 },
        }),
        footprint: Some(ProjectedPolygon2 {
            exterior: vec![
                ProjectedPoint2 { x: 4.0, y: 52.0 },
                ProjectedPoint2 { x: 4.0, y: 52.1 },
                ProjectedPoint2 { x: 4.1, y: 52.1 },
                ProjectedPoint2 { x: 4.1, y: 52.0 },
                ProjectedPoint2 { x: 4.0, y: 52.0 },
            ],
        }),
        availability: SurveySpatialAvailability::Available,
        notes: vec!["synthetic test geometry".to_string()],
    });
    let data = Array3::from_shape_fn((2, 3, 4), |(iline, xline, sample)| {
        iline as f32 * 100.0 + xline as f32 * 10.0 + sample as f32
    });
    create_tbvol_store(&source_root, manifest, &data, None).unwrap();

    let mut project = OphioliteProject::create(root.join("project")).unwrap();
    let binding = AssetBindingInput {
        well_name: "Map Well".to_string(),
        wellbore_name: "Map WB".to_string(),
        uwi: Some("MAP-UWI-3857".to_string()),
        api: None,
        operator_aliases: vec!["Ophiolite".to_string()],
    };
    let seismic = project
        .import_seismic_trace_data_store(&source_root, &binding, Some("survey-4326"))
        .unwrap();

    let resolved = project
        .resolve_survey_map_source(&SurveyMapRequestDto {
            schema_version: 1,
            survey_asset_ids: vec![seismic.asset.id.0.clone()],
            wellbore_ids: Vec::new(),
            display_coordinate_reference_id: Some("EPSG:3857".to_string()),
        })
        .unwrap();

    let display_spatial = resolved.surveys[0].display_spatial.as_ref().unwrap();
    assert!(matches!(
        resolved.surveys[0].transform_status,
        SurveyMapTransformStatusDto::DisplayTransformed
    ));
    assert_eq!(
        display_spatial
            .coordinate_reference
            .as_ref()
            .and_then(|reference| reference.id.as_deref()),
        Some("EPSG:3857")
    );
    assert!(display_spatial.grid_transform.as_ref().unwrap().origin.x > 400_000.0);
    assert!(display_spatial.grid_transform.as_ref().unwrap().origin.y > 6_000_000.0);

    let cache_dir = root
        .join("project")
        .join(".ophiolite")
        .join("map-transform-cache");
    let cache_entries = fs::read_dir(&cache_dir)
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(!cache_entries.is_empty());

    let resolved_cached = project
        .resolve_survey_map_source(&SurveyMapRequestDto {
            schema_version: 1,
            survey_asset_ids: vec![seismic.asset.id.0.clone()],
            wellbore_ids: Vec::new(),
            display_coordinate_reference_id: Some("EPSG:3857".to_string()),
        })
        .unwrap();
    assert!(
        resolved_cached.surveys[0]
            .transform_diagnostics
            .notes
            .iter()
            .any(|note| note.contains("loaded from cache"))
    );
}

#[test]
fn structured_asset_edits_create_revision_history() {
    let root = temp_project_root("structured_asset_edits_create_revision_history");
    let mut project = OphioliteProject::create(&root).unwrap();
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

    let updated_rows = vec![ophiolite::TopRow {
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
    let mut project = OphioliteProject::create(&root).unwrap();
    let imported = project
        .import_las(examples::path("6038187_v1.2_short.las"), Some("logs"))
        .unwrap();

    let mut session = ophiolite::open_package(&imported.asset.package_path).unwrap();
    let curve_name = session
        .file()
        .curve_names()
        .into_iter()
        .find(|name| name != &session.file().index.curve_id)
        .unwrap();
    let original = session.file().curve(&curve_name).unwrap().clone();
    let mut data = original.data.clone();
    data[0] = ophiolite::LasValue::Number(999.0);
    session
        .apply_curve_edit(&ophiolite::CurveEditRequest::Upsert(
            ophiolite::CurveUpdateRequest {
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
    let mut project = OphioliteProject::create(&root).unwrap();

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
    let mut project = OphioliteProject::create(&root).unwrap();

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
    let mut project = OphioliteProject::create(&root).unwrap();

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
        operator_aliases: vec!["Ophiolite".to_string()],
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
    let root = std::env::temp_dir().join(format!("ophiolite_{label}_{nanos}_{unique}"));
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

fn sample_store_manifest() -> TbvolManifest {
    TbvolManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            source: SourceIdentity {
                source_path: PathBuf::from("survey.sgy"),
                file_size: 1024,
                trace_count: 6,
                samples_per_trace: 4,
                sample_interval_us: 2000,
                sample_format_code: 5,
                geometry: GeometryProvenance {
                    inline_field: HeaderFieldSpec {
                        name: "INLINE_3D".to_string(),
                        start_byte: 189,
                        value_type: "I32".to_string(),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: "CROSSLINE_3D".to_string(),
                        start_byte: 193,
                        value_type: "I32".to_string(),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape: [2, 3, 4],
            axes: VolumeAxes {
                ilines: vec![1000.0, 1001.0],
                xlines: vec![2000.0, 2001.0, 2002.0],
                sample_axis_ms: vec![0.0, 2.0, 4.0, 6.0],
            },
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "project_catalog_test".to_string(),
            processing_lineage: None,
        },
        [2, 3, 4],
        false,
    )
}
