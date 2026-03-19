use crate::{
    AssetBindingInput, AssetId, DepthRangeQuery, OphioliteProject, Result, WellId, WellboreId,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntheticProjectAssetIds {
    pub log: AssetId,
    pub trajectory: AssetId,
    pub tops: AssetId,
    pub pressure: AssetId,
    pub drilling: AssetId,
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

    for dir in [
        &logs_root,
        &trajectory_root,
        &tops_root,
        &pressure_root,
        &drilling_root,
    ] {
        fs::create_dir_all(dir)?;
    }

    let log_las = logs_root.join("synthetic_well.las");
    let trajectory_csv = trajectory_root.join("synthetic_trajectory.csv");
    let tops_csv = tops_root.join("synthetic_tops.csv");
    let pressure_csv = pressure_root.join("synthetic_pressure.csv");
    let drilling_csv = drilling_root.join("synthetic_drilling.csv");

    fs::write(&log_las, synthetic_las_contents())?;
    fs::write(&trajectory_csv, synthetic_trajectory_csv())?;
    fs::write(&tops_csv, synthetic_tops_csv())?;
    fs::write(&pressure_csv, synthetic_pressure_csv())?;
    fs::write(&drilling_csv, synthetic_drilling_csv())?;

    Ok(SyntheticProjectSourcePaths {
        log_las,
        trajectory_csv,
        tops_csv,
        pressure_csv,
        drilling_csv,
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
        rows.push_str(&format!("{depth:.1} {gr:.2} {rhob:.2} {nphi:.4}\n"));
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
