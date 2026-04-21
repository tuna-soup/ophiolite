use crate::{
    AssetBindingInput, AssetId, DepthRangeQuery, LasError, OphioliteProject,
    ProjectSurveyMapRequestDto, Result, SECTION_WELL_OVERLAY_CONTRACT_VERSION,
    SURVEY_MAP_CONTRACT_VERSION, SectionWellOverlayDomainDto, SectionWellOverlayRequestDto,
    WELL_PANEL_CONTRACT_VERSION, WellId, WellPanelRequestDto, WellboreId,
};
use ndarray::Array3;
use ophiolite_seismic::SampleDataFidelity;
use ophiolite_seismic::{
    CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
    ProjectedPoint2, ProjectedPolygon2, ProjectedVector2, SectionAxis, SurveyGridTransform,
    SurveySpatialAvailability, SurveySpatialDescriptor, WellAzimuthReferenceKind,
    WellboreAnchorKind, WellboreAnchorReference, WellboreGeometry,
};
use ophiolite_seismic_runtime::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, TbvolManifest, VolumeAxes,
    VolumeMetadata, create_tbvol_store,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const WELL_NAME: &str = "SYNTHETIC-1";
const UWI: &str = "SYN-UWI-001";
const API: &str = "SYN-API-001";
const COMPANY: &str = "Ophiolite Synthetic Energy";
const LOG_COLLECTION_NAME: &str = "synthetic-log";
const TRAJECTORY_COLLECTION_NAME: &str = "synthetic-survey";
const TOPS_COLLECTION_NAME: &str = "synthetic-tops";
const PRESSURE_COLLECTION_NAME: &str = "synthetic-pressure";
const DRILLING_COLLECTION_NAME: &str = "synthetic-drilling";
const SURVEY_COLLECTION_NAME: &str = "synthetic-survey-3d";
const SURVEY_CRS_ID: &str = "EPSG:23031";
const DEPTH_START: f64 = 1000.0;
const DEPTH_STOP: f64 = 1100.0;
const DEPTH_STEP: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntheticProjectSourcePaths {
    pub log_las: PathBuf,
    pub trajectory_csv: PathBuf,
    pub tops_csv: PathBuf,
    pub pressure_csv: PathBuf,
    pub drilling_csv: PathBuf,
    pub survey_store: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntheticProjectAssetIds {
    pub log: AssetId,
    pub trajectory: AssetId,
    pub tops: AssetId,
    pub pressure: AssetId,
    pub drilling: AssetId,
    pub survey: AssetId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntheticProjectFixture {
    pub project_root: PathBuf,
    pub sources: SyntheticProjectSourcePaths,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub asset_ids: SyntheticProjectAssetIds,
}

pub fn generate_synthetic_project_fixture(
    output_root: impl AsRef<Path>,
) -> Result<SyntheticProjectFixture> {
    let output_root = output_root.as_ref();
    if output_root.exists() {
        fs::remove_dir_all(output_root)?;
    }
    fs::create_dir_all(output_root)?;

    let sources = write_source_files(output_root)?;
    let mut project = OphioliteProject::create(output_root)?;

    let log = project.import_las(&sources.log_las, Some(LOG_COLLECTION_NAME))?;
    let binding = AssetBindingInput {
        well_name: WELL_NAME.to_string(),
        wellbore_name: WELL_NAME.to_string(),
        uwi: Some(UWI.to_string()),
        api: Some(API.to_string()),
        operator_aliases: vec![COMPANY.to_string()],
    };
    let trajectory = project.import_trajectory_csv(
        &sources.trajectory_csv,
        &binding,
        Some(TRAJECTORY_COLLECTION_NAME),
    )?;
    let tops = project.import_tops_csv(&sources.tops_csv, &binding, Some(TOPS_COLLECTION_NAME))?;
    let pressure = project.import_pressure_csv(
        &sources.pressure_csv,
        &binding,
        Some(PRESSURE_COLLECTION_NAME),
    )?;
    let drilling = project.import_drilling_csv(
        &sources.drilling_csv,
        &binding,
        Some(DRILLING_COLLECTION_NAME),
    )?;
    project.set_wellbore_geometry(
        &log.resolution.wellbore_id,
        Some(synthetic_wellbore_geometry()),
    )?;
    let survey = project.import_seismic_trace_data_store(
        &sources.survey_store,
        &binding,
        Some(SURVEY_COLLECTION_NAME),
    )?;

    // Confirm the generated fixture is queryable as a coherent wellbore dataset.
    let _ = project.assets_covering_depth_range(
        &log.resolution.wellbore_id,
        DEPTH_START + 5.0,
        DEPTH_STOP - 5.0,
    )?;
    let _ = project.read_trajectory_rows(
        &trajectory.asset.id,
        Some(&DepthRangeQuery {
            depth_min: Some(DEPTH_START + 10.0),
            depth_max: Some(DEPTH_STOP - 10.0),
        }),
    )?;
    let _ = project.resolve_well_panel_source(&WellPanelRequestDto {
        schema_version: WELL_PANEL_CONTRACT_VERSION,
        wellbore_ids: vec![log.resolution.wellbore_id.0.clone()],
        depth_min: None,
        depth_max: None,
    })?;
    let _ = project.resolve_wellbore_trajectory(&log.resolution.wellbore_id)?;
    let _ = project.resolve_survey_map_source(&ProjectSurveyMapRequestDto {
        schema_version: SURVEY_MAP_CONTRACT_VERSION,
        survey_asset_ids: vec![survey.asset.id.0.clone()],
        wellbore_ids: vec![log.resolution.wellbore_id.0.clone()],
        display_coordinate_reference_id: SURVEY_CRS_ID.to_string(),
    })?;
    let _ = project.resolve_section_well_overlays(&SectionWellOverlayRequestDto {
        schema_version: SECTION_WELL_OVERLAY_CONTRACT_VERSION,
        project_root: output_root.to_string_lossy().into_owned(),
        survey_asset_id: survey.asset.id.0.clone(),
        wellbore_ids: vec![log.resolution.wellbore_id.0.clone()],
        axis: SectionAxis::Inline,
        index: 1001,
        tolerance_m: None,
        display_domain: SectionWellOverlayDomainDto::Depth,
        active_well_model_ids: Vec::new(),
    })?;

    Ok(SyntheticProjectFixture {
        project_root: output_root.to_path_buf(),
        sources,
        well_id: log.resolution.well_id,
        wellbore_id: log.resolution.wellbore_id,
        asset_ids: SyntheticProjectAssetIds {
            log: log.asset.id,
            trajectory: trajectory.asset.id,
            tops: tops.asset.id,
            pressure: pressure.asset.id,
            drilling: drilling.asset.id,
            survey: survey.asset.id,
        },
    })
}

fn write_source_files(root: &Path) -> Result<SyntheticProjectSourcePaths> {
    let sources_root = root.join("sources");
    let logs_root = sources_root.join("logs");
    let trajectory_root = sources_root.join("trajectory");
    let tops_root = sources_root.join("tops");
    let pressure_root = sources_root.join("pressure");
    let drilling_root = sources_root.join("drilling");
    let survey_root = sources_root.join("survey");

    for dir in [
        &logs_root,
        &trajectory_root,
        &tops_root,
        &pressure_root,
        &drilling_root,
        &survey_root,
    ] {
        fs::create_dir_all(dir)?;
    }

    let log_las = logs_root.join("synthetic_well.las");
    let trajectory_csv = trajectory_root.join("synthetic_trajectory.csv");
    let tops_csv = tops_root.join("synthetic_tops.csv");
    let pressure_csv = pressure_root.join("synthetic_pressure.csv");
    let drilling_csv = drilling_root.join("synthetic_drilling.csv");
    let survey_store = survey_root.join("synthetic_survey.tbvol");

    fs::write(&log_las, synthetic_las_contents())?;
    fs::write(&trajectory_csv, synthetic_trajectory_csv())?;
    fs::write(&tops_csv, synthetic_tops_csv())?;
    fs::write(&pressure_csv, synthetic_pressure_csv())?;
    fs::write(&drilling_csv, synthetic_drilling_csv())?;
    create_tbvol_store(
        &survey_store,
        synthetic_survey_store_manifest(),
        &synthetic_survey_cube(),
        None,
    )
    .map_err(|error| {
        LasError::Storage(format!("failed to create synthetic survey store: {error}"))
    })?;

    Ok(SyntheticProjectSourcePaths {
        log_las,
        trajectory_csv,
        tops_csv,
        pressure_csv,
        drilling_csv,
        survey_store,
    })
}

fn synthetic_las_contents() -> String {
    let row_count = ((DEPTH_STOP - DEPTH_START) / DEPTH_STEP).round() as usize + 1;
    let mut rows = String::new();
    for index in 0..row_count {
        let depth = DEPTH_START + (index as f64 * DEPTH_STEP);
        let gr = 72.0 + ((index % 11) as f64 * 2.4);
        let rhob = 2425.0 + ((index % 9) as f64 * 6.5);
        let nphi = 0.19 + ((index % 7) as f64 * 0.0075);
        let dt = 106.0 - ((index % 13) as f64 * 0.8);
        let dts = 205.0 - ((index % 15) as f64 * 1.1);
        rows.push_str(&format!(
            "{depth:.1} {gr:.2} {rhob:.2} {nphi:.4} {dt:.2} {dts:.2}\n"
        ));
    }

    format!(
        "~Version Information Section\n\
VERS.                  2.0 : CWLS log ASCII standard version\n\
WRAP.                   NO : One line per depth step\n\
~Well Information Section\n\
STRT.M             {DEPTH_START:.1} : Start depth\n\
STOP.M             {DEPTH_STOP:.1} : Stop depth\n\
STEP.M               {DEPTH_STEP:.1} : Step increment\n\
NULL.               -999.25 : Null value\n\
COMP. {COMPANY} : Company\n\
WELL. {WELL_NAME} : Well\n\
UWI. {UWI} : Unique Well Identifier\n\
API. {API} : API number\n\
FLD. Synthetic Field : Field\n\
~Curve Information Section\n\
DEPT.M : Measured depth\n\
GR.GAPI : Gamma ray\n\
RHOB.KG/M3 : Bulk density\n\
NPHI.V/V : Neutron porosity\n\
DT.US/FT : Compressional slowness\n\
DTS.US/FT : Shear slowness\n\
~ASCII Log Data Section\n\
{rows}"
    )
}

fn synthetic_trajectory_csv() -> String {
    let rows = [
        "md,tvd,azimuth,inclination,northing_offset,easting_offset",
        "1000.0,998.5,180.0,0.5,0.0,0.0",
        "1025.0,1022.0,181.5,1.2,8.0,2.0",
        "1050.0,1044.5,183.0,2.4,18.0,7.5",
        "1075.0,1066.0,184.0,3.1,30.0,14.0",
        "1100.0,1087.0,185.0,4.0,44.0,23.0",
    ];
    rows.join("\n") + "\n"
}

fn synthetic_tops_csv() -> String {
    let rows = [
        "name,top_depth,base_depth,source,depth_reference",
        "Shale A,1008.0,1018.0,Synthetic Interpreter,MD",
        "Sand A,1030.0,1044.0,Synthetic Interpreter,MD",
        "Sand B,1062.5,1074.0,Synthetic Interpreter,MD",
    ];
    rows.join("\n") + "\n"
}

fn synthetic_pressure_csv() -> String {
    let rows = [
        "measured_depth,pressure,phase,test_kind,timestamp",
        "1012.5,4185.0,oil,RFT,2024-01-01T00:00:00Z",
        "1040.0,4120.0,water,MDT,2024-01-02T00:00:00Z",
        "1068.0,4055.0,gas,RFT,2024-01-03T00:00:00Z",
    ];
    rows.join("\n") + "\n"
}

fn synthetic_drilling_csv() -> String {
    let rows = [
        "measured_depth,event_kind,value,unit,timestamp,comment",
        "1005.0,ROP,28.0,m/h,2024-01-01T01:00:00Z,build-up",
        "1038.0,WOB,12.5,klbf,2024-01-01T02:00:00Z,stable drilling",
        "1072.0,ROP,24.0,m/h,2024-01-01T03:00:00Z,sand section",
    ];
    rows.join("\n") + "\n"
}

fn synthetic_survey_cube() -> Array3<f32> {
    Array3::from_shape_fn((2, 3, 4), |(iline, xline, sample)| {
        iline as f32 * 100.0 + xline as f32 * 10.0 + sample as f32
    })
}

fn synthetic_survey_store_manifest() -> TbvolManifest {
    let coordinate_reference = CoordinateReferenceDescriptor {
        id: Some(SURVEY_CRS_ID.to_string()),
        name: Some("ED50 / UTM zone 31N".to_string()),
        geodetic_datum: Some("ED50".to_string()),
        unit: Some("m".to_string()),
    };
    let coordinate_reference_binding = CoordinateReferenceBinding {
        detected: Some(coordinate_reference.clone()),
        effective: Some(coordinate_reference.clone()),
        source: CoordinateReferenceSource::Header,
        notes: Vec::new(),
    };
    TbvolManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: "synthetic-survey-store".to_string(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic_survey.segy"),
                file_size: 1024,
                trace_count: 6,
                samples_per_trace: 4,
                sample_interval_us: 2000,
                sample_format_code: 5,
                sample_data_fidelity: SampleDataFidelity::default(),
                endianness: "big".to_string(),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
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
            axes: VolumeAxes::from_time_axis(
                vec![1000.0, 1001.0],
                vec![2000.0, 2001.0, 2002.0],
                vec![0.0, 2.0, 4.0, 6.0],
            ),
            segy_export: None,
            coordinate_reference_binding: Some(coordinate_reference_binding.clone()),
            spatial: Some(SurveySpatialDescriptor {
                coordinate_reference: Some(coordinate_reference),
                grid_transform: Some(SurveyGridTransform {
                    origin: ProjectedPoint2 {
                        x: 500_000.0,
                        y: 6_200_000.0,
                    },
                    inline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                    xline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                }),
                footprint: Some(ProjectedPolygon2 {
                    exterior: vec![
                        ProjectedPoint2 {
                            x: 500_000.0,
                            y: 6_200_000.0,
                        },
                        ProjectedPoint2 {
                            x: 500_000.0,
                            y: 6_200_025.0,
                        },
                        ProjectedPoint2 {
                            x: 500_050.0,
                            y: 6_200_025.0,
                        },
                        ProjectedPoint2 {
                            x: 500_050.0,
                            y: 6_200_000.0,
                        },
                        ProjectedPoint2 {
                            x: 500_000.0,
                            y: 6_200_000.0,
                        },
                    ],
                }),
                availability: SurveySpatialAvailability::Available,
                notes: vec!["synthetic projected survey".to_string()],
            }),
            created_by: "synthetic_project_fixture".to_string(),
            processing_lineage: None,
        },
        [2, 3, 4],
        false,
    )
}

fn synthetic_wellbore_geometry() -> WellboreGeometry {
    WellboreGeometry {
        anchor: Some(WellboreAnchorReference {
            kind: WellboreAnchorKind::Surface,
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: Some(SURVEY_CRS_ID.to_string()),
                name: Some("ED50 / UTM zone 31N".to_string()),
                geodetic_datum: Some("ED50".to_string()),
                unit: Some("m".to_string()),
            }),
            location: ProjectedPoint2 {
                x: 500_000.0,
                y: 6_200_000.0,
            },
            parent_wellbore_id: None,
            parent_measured_depth_m: None,
            notes: vec!["synthetic survey anchor".to_string()],
        }),
        vertical_datum: Some("KellyBushing".to_string()),
        depth_unit: Some("m".to_string()),
        azimuth_reference: WellAzimuthReferenceKind::GridNorth,
        notes: vec!["synthetic projected wellbore geometry".to_string()],
    }
}
