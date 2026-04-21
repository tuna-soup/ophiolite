use crate::{AssetKind, DepthReference, LasError, Result, VerticalDatum};
use arrow_array::{Array, ArrayRef, Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use csv::StringRecord;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::{EnabledStatistics, WriterProperties};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

const ASSET_TABLE_METADATA_SCHEMA_VERSION: &str = "0.1.0";
const DATA_FILENAME: &str = "data.parquet";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetColumnType {
    Float64,
    Utf8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetColumnMetadata {
    pub name: String,
    pub data_type: AssetColumnType,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub nullable: bool,
    pub is_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetTableMetadata {
    pub schema_version: String,
    pub asset_kind: AssetKind,
    pub row_count: usize,
    pub columns: Vec<AssetColumnMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetBindingInput {
    pub well_name: String,
    pub wellbore_name: String,
    pub uwi: Option<String>,
    pub api: Option<String>,
    pub operator_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryRow {
    pub measured_depth: f64,
    pub true_vertical_depth: Option<f64>,
    pub true_vertical_depth_subsea: Option<f64>,
    pub azimuth_deg: Option<f64>,
    pub inclination_deg: Option<f64>,
    pub northing_offset: Option<f64>,
    pub easting_offset: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopRow {
    pub name: String,
    pub top_depth: f64,
    pub base_depth: Option<f64>,
    pub source: Option<String>,
    pub source_depth_reference: Option<String>,
    pub depth_domain: Option<String>,
    pub depth_datum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellMarkerRow {
    pub name: String,
    pub marker_kind: Option<String>,
    pub top_depth: f64,
    pub base_depth: Option<f64>,
    pub source: Option<String>,
    pub source_depth_reference: Option<String>,
    pub depth_domain: Option<String>,
    pub depth_datum: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellMarkerHorizonResidualRow {
    pub marker_name: String,
    pub marker_kind: Option<String>,
    pub source_depth: f64,
    pub source_depth_reference: Option<String>,
    pub source_depth_domain: Option<String>,
    pub source_depth_datum: Option<String>,
    pub measured_depth: Option<f64>,
    pub true_vertical_depth: Option<f64>,
    pub true_vertical_depth_subsea: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub horizon_depth: Option<f64>,
    pub residual: Option<f64>,
    pub horizon_inline_ordinal: Option<f64>,
    pub horizon_xline_ordinal: Option<f64>,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PressureObservationRow {
    pub measured_depth: Option<f64>,
    pub pressure: f64,
    pub phase: Option<String>,
    pub test_kind: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DrillingObservationRow {
    pub measured_depth: Option<f64>,
    pub event_kind: String,
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub timestamp: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepthRangeQuery {
    pub depth_min: Option<f64>,
    pub depth_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NormalizedDepthSemantics {
    pub source_depth_reference: Option<String>,
    pub depth_domain: Option<String>,
    pub depth_datum: Option<String>,
}

pub fn normalize_depth_semantics(
    source_depth_reference: Option<&str>,
    depth_domain: Option<&str>,
    depth_datum: Option<&str>,
) -> NormalizedDepthSemantics {
    let source_depth_reference = source_depth_reference
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let mut depth_domain = normalize_depth_domain(depth_domain);
    let mut depth_datum = normalize_depth_datum(depth_datum);

    if let Some(source_text) = source_depth_reference.as_deref() {
        if depth_domain.is_none() {
            depth_domain = inferred_depth_domain_from_source_reference(source_text);
        }
        if depth_datum.is_none() {
            depth_datum = inferred_depth_datum_from_source_reference(source_text);
        }
    }

    if depth_domain.is_none() && depth_datum.is_some() {
        depth_domain = Some(String::from("md"));
    }

    NormalizedDepthSemantics {
        source_depth_reference,
        depth_domain,
        depth_datum,
    }
}

pub fn normalize_top_row_depth_semantics(row: &mut TopRow) {
    let normalized = normalize_depth_semantics(
        row.source_depth_reference.as_deref(),
        row.depth_domain.as_deref(),
        row.depth_datum.as_deref(),
    );
    row.source_depth_reference = normalized.source_depth_reference;
    row.depth_domain = normalized.depth_domain;
    row.depth_datum = normalized.depth_datum;
}

pub fn normalize_well_marker_row_depth_semantics(row: &mut WellMarkerRow) {
    let normalized = normalize_depth_semantics(
        row.source_depth_reference.as_deref(),
        row.depth_domain.as_deref(),
        row.depth_datum.as_deref(),
    );
    row.source_depth_reference = normalized.source_depth_reference;
    row.depth_domain = normalized.depth_domain;
    row.depth_datum = normalized.depth_datum;
}

pub fn depth_reference_from_domain_code(code: Option<&str>) -> DepthReference {
    match normalize_depth_domain(code) {
        Some(value) if value == "md" => DepthReference::MeasuredDepth,
        Some(value) if value == "tvd" => DepthReference::TrueVerticalDepth,
        Some(value) if value == "tvdss" => DepthReference::TrueVerticalDepthSubsea,
        _ => DepthReference::Unknown,
    }
}

pub fn vertical_datum_from_code(code: Option<&str>) -> Option<VerticalDatum> {
    match normalize_depth_datum(code) {
        Some(value) if value == "kb" => Some(VerticalDatum::KellyBushing),
        Some(value) if value == "rt" => Some(VerticalDatum::RotaryTable),
        Some(value) if value == "df" => Some(VerticalDatum::DrillFloor),
        Some(value) if value == "gl" => Some(VerticalDatum::GroundLevel),
        Some(value) if value == "msl" => Some(VerticalDatum::MeanSeaLevel),
        Some(_) => Some(VerticalDatum::Unknown),
        None => None,
    }
}

pub fn inferred_reference_metadata_for_top_rows(
    rows: &[TopRow],
) -> (DepthReference, Option<VerticalDatum>) {
    infer_reference_metadata(
        rows.iter().map(|row| row.depth_domain.as_deref()),
        rows.iter().map(|row| row.depth_datum.as_deref()),
    )
}

pub fn inferred_reference_metadata_for_well_marker_rows(
    rows: &[WellMarkerRow],
) -> (DepthReference, Option<VerticalDatum>) {
    infer_reference_metadata(
        rows.iter().map(|row| row.depth_domain.as_deref()),
        rows.iter().map(|row| row.depth_datum.as_deref()),
    )
}

fn normalize_depth_domain(value: Option<&str>) -> Option<String> {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("md") | Some("measured_depth") | Some("measured depth") => Some(String::from("md")),
        Some("tvd") | Some("true_vertical_depth") | Some("true vertical depth") => {
            Some(String::from("tvd"))
        }
        Some("tvdss")
        | Some("tvdss_m")
        | Some("true_vertical_depth_subsea")
        | Some("true vertical depth subsea") => Some(String::from("tvdss")),
        Some("elev") | Some("elevation") => Some(String::from("elevation")),
        _ => None,
    }
}

fn normalize_depth_datum(value: Option<&str>) -> Option<String> {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("kb") | Some("kelly_bushing") | Some("kelly bushing") => Some(String::from("kb")),
        Some("rt") | Some("rotary_table") | Some("rotary table") => Some(String::from("rt")),
        Some("df") | Some("drill_floor") | Some("drill floor") => Some(String::from("df")),
        Some("gl") | Some("ground_level") | Some("ground level") => Some(String::from("gl")),
        Some("msl") | Some("mean_sea_level") | Some("mean sea level") | Some("sea level") => {
            Some(String::from("msl"))
        }
        _ => None,
    }
}

fn inferred_depth_domain_from_source_reference(value: &str) -> Option<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "kb" | "kelly bushing" | "rt" | "rotary table" | "df" | "drill floor" | "gl"
        | "ground level" => Some(String::from("md")),
        other => normalize_depth_domain(Some(other)),
    }
}

fn inferred_depth_datum_from_source_reference(value: &str) -> Option<String> {
    normalize_depth_datum(Some(value))
}

fn infer_reference_metadata<'a, D, V>(
    depth_domains: D,
    depth_datums: V,
) -> (DepthReference, Option<VerticalDatum>)
where
    D: Iterator<Item = Option<&'a str>>,
    V: Iterator<Item = Option<&'a str>>,
{
    let mut resolved_depth_reference = None;
    let mut conflicting_depth_reference = false;
    for domain in depth_domains {
        let mapped = depth_reference_from_domain_code(domain);
        if mapped == DepthReference::Unknown {
            conflicting_depth_reference = true;
            continue;
        }
        match &resolved_depth_reference {
            None => resolved_depth_reference = Some(mapped),
            Some(current) if *current == mapped => {}
            Some(_) => conflicting_depth_reference = true,
        }
    }

    let mut resolved_vertical_datum = None;
    let mut saw_any_vertical_datum = false;
    let mut conflicting_vertical_datum = false;
    for datum in depth_datums {
        let mapped = vertical_datum_from_code(datum);
        if mapped.is_none() {
            continue;
        }
        saw_any_vertical_datum = true;
        match (&resolved_vertical_datum, mapped) {
            (None, value) => resolved_vertical_datum = value,
            (Some(current), Some(value)) if *current == value => {}
            _ => conflicting_vertical_datum = true,
        }
    }

    (
        if conflicting_depth_reference {
            DepthReference::Unknown
        } else {
            resolved_depth_reference.unwrap_or(DepthReference::Unknown)
        },
        if !saw_any_vertical_datum {
            None
        } else if conflicting_vertical_datum {
            Some(VerticalDatum::Unknown)
        } else {
            resolved_vertical_datum
        },
    )
}

pub fn data_filename() -> &'static str {
    DATA_FILENAME
}

pub fn trajectory_metadata(rows: &[TrajectoryRow]) -> AssetTableMetadata {
    AssetTableMetadata {
        schema_version: ASSET_TABLE_METADATA_SCHEMA_VERSION.to_string(),
        asset_kind: AssetKind::Trajectory,
        row_count: rows.len(),
        columns: vec![
            column(
                "measured_depth",
                AssetColumnType::Float64,
                None,
                true,
                false,
            ),
            column(
                "true_vertical_depth",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column(
                "true_vertical_depth_subsea",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column("azimuth_deg", AssetColumnType::Float64, None, false, false),
            column(
                "inclination_deg",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column(
                "northing_offset",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column(
                "easting_offset",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
        ],
    }
}

pub fn tops_metadata(rows: &[TopRow]) -> AssetTableMetadata {
    AssetTableMetadata {
        schema_version: ASSET_TABLE_METADATA_SCHEMA_VERSION.to_string(),
        asset_kind: AssetKind::TopSet,
        row_count: rows.len(),
        columns: vec![
            column("name", AssetColumnType::Utf8, None, true, false),
            column("top_depth", AssetColumnType::Float64, None, true, false),
            column("base_depth", AssetColumnType::Float64, None, false, false),
            column("source", AssetColumnType::Utf8, None, false, false),
            column(
                "source_depth_reference",
                AssetColumnType::Utf8,
                None,
                false,
                false,
            ),
            column("depth_domain", AssetColumnType::Utf8, None, false, false),
            column("depth_datum", AssetColumnType::Utf8, None, false, false),
        ],
    }
}

pub fn pressure_metadata(rows: &[PressureObservationRow]) -> AssetTableMetadata {
    AssetTableMetadata {
        schema_version: ASSET_TABLE_METADATA_SCHEMA_VERSION.to_string(),
        asset_kind: AssetKind::PressureObservation,
        row_count: rows.len(),
        columns: vec![
            column(
                "measured_depth",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column("pressure", AssetColumnType::Float64, None, true, false),
            column("phase", AssetColumnType::Utf8, None, false, false),
            column("test_kind", AssetColumnType::Utf8, None, false, false),
            column("timestamp", AssetColumnType::Utf8, None, false, false),
        ],
    }
}

pub fn well_marker_metadata(rows: &[WellMarkerRow]) -> AssetTableMetadata {
    AssetTableMetadata {
        schema_version: ASSET_TABLE_METADATA_SCHEMA_VERSION.to_string(),
        asset_kind: AssetKind::WellMarkerSet,
        row_count: rows.len(),
        columns: vec![
            column("name", AssetColumnType::Utf8, None, true, false),
            column("marker_kind", AssetColumnType::Utf8, None, false, false),
            column("top_depth", AssetColumnType::Float64, None, true, false),
            column("base_depth", AssetColumnType::Float64, None, false, false),
            column("source", AssetColumnType::Utf8, None, false, false),
            column(
                "source_depth_reference",
                AssetColumnType::Utf8,
                None,
                false,
                false,
            ),
            column("depth_domain", AssetColumnType::Utf8, None, false, false),
            column("depth_datum", AssetColumnType::Utf8, None, false, false),
            column("note", AssetColumnType::Utf8, None, false, false),
        ],
    }
}

pub fn well_marker_horizon_residual_metadata(
    rows: &[WellMarkerHorizonResidualRow],
) -> AssetTableMetadata {
    AssetTableMetadata {
        schema_version: ASSET_TABLE_METADATA_SCHEMA_VERSION.to_string(),
        asset_kind: AssetKind::WellMarkerHorizonResidualSet,
        row_count: rows.len(),
        columns: vec![
            column("marker_name", AssetColumnType::Utf8, None, true, false),
            column("marker_kind", AssetColumnType::Utf8, None, false, false),
            column(
                "source_depth",
                AssetColumnType::Float64,
                Some("m"),
                true,
                true,
            ),
            column(
                "source_depth_reference",
                AssetColumnType::Utf8,
                None,
                false,
                false,
            ),
            column(
                "source_depth_domain",
                AssetColumnType::Utf8,
                None,
                false,
                false,
            ),
            column(
                "source_depth_datum",
                AssetColumnType::Utf8,
                None,
                false,
                false,
            ),
            column(
                "measured_depth",
                AssetColumnType::Float64,
                Some("m"),
                false,
                false,
            ),
            column(
                "true_vertical_depth",
                AssetColumnType::Float64,
                Some("m"),
                false,
                false,
            ),
            column(
                "true_vertical_depth_subsea",
                AssetColumnType::Float64,
                Some("m"),
                false,
                false,
            ),
            column("x", AssetColumnType::Float64, Some("m"), false, false),
            column("y", AssetColumnType::Float64, Some("m"), false, false),
            column(
                "horizon_depth",
                AssetColumnType::Float64,
                Some("m"),
                false,
                false,
            ),
            column(
                "residual",
                AssetColumnType::Float64,
                Some("m"),
                false,
                false,
            ),
            column(
                "horizon_inline_ordinal",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column(
                "horizon_xline_ordinal",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column("status", AssetColumnType::Utf8, None, true, false),
            column("note", AssetColumnType::Utf8, None, false, false),
        ],
    }
}

pub fn drilling_metadata(rows: &[DrillingObservationRow]) -> AssetTableMetadata {
    AssetTableMetadata {
        schema_version: ASSET_TABLE_METADATA_SCHEMA_VERSION.to_string(),
        asset_kind: AssetKind::DrillingObservation,
        row_count: rows.len(),
        columns: vec![
            column(
                "measured_depth",
                AssetColumnType::Float64,
                None,
                false,
                false,
            ),
            column("event_kind", AssetColumnType::Utf8, None, true, false),
            column("value", AssetColumnType::Float64, None, false, false),
            column("unit", AssetColumnType::Utf8, None, false, false),
            column("timestamp", AssetColumnType::Utf8, None, false, false),
            column("comment", AssetColumnType::Utf8, None, false, false),
        ],
    }
}

pub fn depth_reference_for_kind(kind: &AssetKind) -> DepthReference {
    match kind {
        AssetKind::Log | AssetKind::Trajectory | AssetKind::TopSet | AssetKind::WellMarkerSet => {
            DepthReference::MeasuredDepth
        }
        AssetKind::WellMarkerHorizonResidualSet => DepthReference::Unknown,
        AssetKind::PressureObservation | AssetKind::DrillingObservation => DepthReference::Unknown,
        AssetKind::CheckshotVspObservationSet => DepthReference::Unknown,
        AssetKind::ManualTimeDepthPickSet => DepthReference::Unknown,
        AssetKind::WellTieObservationSet => DepthReference::Unknown,
        AssetKind::WellTimeDepthAuthoredModel => DepthReference::Unknown,
        AssetKind::WellTimeDepthModel => DepthReference::Unknown,
        AssetKind::RawSourceBundle => DepthReference::Unknown,
        AssetKind::SeismicTraceData => DepthReference::Unknown,
    }
}

pub fn vertical_datum_for_kind(kind: &AssetKind) -> Option<VerticalDatum> {
    match kind {
        AssetKind::Trajectory | AssetKind::TopSet | AssetKind::WellMarkerSet => {
            Some(VerticalDatum::Unknown)
        }
        AssetKind::WellMarkerHorizonResidualSet => None,
        AssetKind::CheckshotVspObservationSet => None,
        AssetKind::ManualTimeDepthPickSet => None,
        AssetKind::WellTieObservationSet => None,
        AssetKind::WellTimeDepthAuthoredModel => None,
        AssetKind::WellTimeDepthModel => None,
        AssetKind::RawSourceBundle => None,
        AssetKind::SeismicTraceData => None,
        _ => None,
    }
}

pub fn trajectory_extent(rows: &[TrajectoryRow]) -> (Option<f64>, Option<f64>, Option<usize>) {
    numeric_extent(rows.iter().map(|row| row.measured_depth), rows.len())
}

pub fn tops_extent(rows: &[TopRow]) -> (Option<f64>, Option<f64>, Option<usize>) {
    numeric_extent(rows.iter().map(|row| row.top_depth), rows.len())
}

pub fn pressure_extent(
    rows: &[PressureObservationRow],
) -> (Option<f64>, Option<f64>, Option<usize>) {
    numeric_extent(rows.iter().filter_map(|row| row.measured_depth), rows.len())
}

pub fn well_marker_extent(rows: &[WellMarkerRow]) -> (Option<f64>, Option<f64>, Option<usize>) {
    numeric_extent(rows.iter().map(|row| row.top_depth), rows.len())
}

pub fn well_marker_horizon_residual_extent(
    rows: &[WellMarkerHorizonResidualRow],
) -> (Option<f64>, Option<f64>, Option<usize>) {
    numeric_extent(rows.iter().map(|row| row.source_depth), rows.len())
}

pub fn drilling_extent(
    rows: &[DrillingObservationRow],
) -> (Option<f64>, Option<f64>, Option<usize>) {
    numeric_extent(rows.iter().filter_map(|row| row.measured_depth), rows.len())
}

pub fn parse_trajectory_csv(path: &Path) -> Result<Vec<TrajectoryRow>> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| LasError::Parse(format!("failed to read trajectory csv: {error}")))?;
    let headers = reader
        .headers()
        .map_err(|error| LasError::Parse(format!("failed to read trajectory headers: {error}")))?
        .clone();
    let md_index = required_header(&headers, &["md", "measured_depth"])?;
    let tvd_index = optional_header(&headers, &["tvd", "true_vertical_depth", "tvd_m"]);
    let tvdss_index = optional_header(
        &headers,
        &[
            "tvdss",
            "true_vertical_depth_subsea",
            "true_vertical_depth_ss",
            "tvdss_m",
        ],
    );
    let azimuth_index = optional_header(&headers, &["azimuth", "azimuth_deg", "azi"]);
    let inclination_index = optional_header(&headers, &["inclination", "inclination_deg", "inc"]);
    let northing_index = optional_header(
        &headers,
        &["northing_offset", "northing", "northing_m", "y_offset"],
    );
    let easting_index = optional_header(
        &headers,
        &["easting_offset", "easting", "easting_m", "x_offset"],
    );

    reader
        .records()
        .map(|record| {
            let record = record.map_err(csv_record_error)?;
            Ok(TrajectoryRow {
                measured_depth: required_f64(&record, md_index, "measured_depth")?,
                true_vertical_depth: optional_f64(&record, tvd_index),
                true_vertical_depth_subsea: optional_f64(&record, tvdss_index),
                azimuth_deg: optional_f64(&record, azimuth_index),
                inclination_deg: optional_f64(&record, inclination_index),
                northing_offset: optional_f64(&record, northing_index),
                easting_offset: optional_f64(&record, easting_index),
            })
        })
        .collect()
}

pub fn parse_tops_csv(path: &Path) -> Result<Vec<TopRow>> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| LasError::Parse(format!("failed to read tops csv: {error}")))?;
    let headers = reader
        .headers()
        .map_err(|error| LasError::Parse(format!("failed to read tops headers: {error}")))?
        .clone();
    let name_index = required_header(&headers, &["name", "top_name"])?;
    let top_index = required_header(&headers, &["top_depth", "top"])?;
    let base_index = optional_header(&headers, &["base_depth", "base"]);
    let source_index = optional_header(&headers, &["source"]);
    let source_reference_index = optional_header(
        &headers,
        &["source_depth_reference", "depth_reference", "reference"],
    );
    let depth_domain_index = optional_header(&headers, &["depth_domain"]);
    let depth_datum_index = optional_header(&headers, &["depth_datum"]);

    reader
        .records()
        .map(|record| {
            let record = record.map_err(csv_record_error)?;
            let mut row = TopRow {
                name: required_string(&record, name_index, "name")?,
                top_depth: required_f64(&record, top_index, "top_depth")?,
                base_depth: optional_f64(&record, base_index),
                source: optional_string(&record, source_index),
                source_depth_reference: optional_string(&record, source_reference_index),
                depth_domain: optional_string(&record, depth_domain_index),
                depth_datum: optional_string(&record, depth_datum_index),
            };
            normalize_top_row_depth_semantics(&mut row);
            Ok(row)
        })
        .collect()
}

pub fn parse_pressure_csv(path: &Path) -> Result<Vec<PressureObservationRow>> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| LasError::Parse(format!("failed to read pressure csv: {error}")))?;
    let headers = reader
        .headers()
        .map_err(|error| LasError::Parse(format!("failed to read pressure headers: {error}")))?
        .clone();
    let md_index = optional_header(&headers, &["md", "measured_depth"]);
    let pressure_index = required_header(&headers, &["pressure"])?;
    let phase_index = optional_header(&headers, &["phase"]);
    let kind_index = optional_header(&headers, &["test_kind", "kind"]);
    let timestamp_index = optional_header(&headers, &["timestamp", "time"]);

    reader
        .records()
        .map(|record| {
            let record = record.map_err(csv_record_error)?;
            Ok(PressureObservationRow {
                measured_depth: optional_f64(&record, md_index),
                pressure: required_f64(&record, pressure_index, "pressure")?,
                phase: optional_string(&record, phase_index),
                test_kind: optional_string(&record, kind_index),
                timestamp: optional_string(&record, timestamp_index),
            })
        })
        .collect()
}

pub fn parse_well_markers_csv(path: &Path) -> Result<Vec<WellMarkerRow>> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| LasError::Parse(format!("failed to read well marker csv: {error}")))?;
    let headers = reader
        .headers()
        .map_err(|error| LasError::Parse(format!("failed to read well marker headers: {error}")))?
        .clone();
    let name_index = required_header(&headers, &["name", "marker_name", "top_name"])?;
    let marker_kind_index = optional_header(&headers, &["marker_kind", "kind"]);
    let top_index = required_header(&headers, &["top_depth", "top"])?;
    let base_index = optional_header(&headers, &["base_depth", "base"]);
    let source_index = optional_header(&headers, &["source"]);
    let source_reference_index = optional_header(
        &headers,
        &["source_depth_reference", "depth_reference", "reference"],
    );
    let depth_domain_index = optional_header(&headers, &["depth_domain"]);
    let depth_datum_index = optional_header(&headers, &["depth_datum"]);
    let note_index = optional_header(&headers, &["note", "notes", "comment"]);

    reader
        .records()
        .map(|record| {
            let record = record.map_err(csv_record_error)?;
            let mut row = WellMarkerRow {
                name: required_string(&record, name_index, "name")?,
                marker_kind: optional_string(&record, marker_kind_index),
                top_depth: required_f64(&record, top_index, "top_depth")?,
                base_depth: optional_f64(&record, base_index),
                source: optional_string(&record, source_index),
                source_depth_reference: optional_string(&record, source_reference_index),
                depth_domain: optional_string(&record, depth_domain_index),
                depth_datum: optional_string(&record, depth_datum_index),
                note: optional_string(&record, note_index),
            };
            normalize_well_marker_row_depth_semantics(&mut row);
            Ok(row)
        })
        .collect()
}

pub fn parse_drilling_csv(path: &Path) -> Result<Vec<DrillingObservationRow>> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| LasError::Parse(format!("failed to read drilling csv: {error}")))?;
    let headers = reader
        .headers()
        .map_err(|error| LasError::Parse(format!("failed to read drilling headers: {error}")))?
        .clone();
    let md_index = optional_header(&headers, &["md", "measured_depth"]);
    let kind_index = required_header(&headers, &["event_kind", "kind"])?;
    let value_index = optional_header(&headers, &["value"]);
    let unit_index = optional_header(&headers, &["unit"]);
    let timestamp_index = optional_header(&headers, &["timestamp", "time"]);
    let comment_index = optional_header(&headers, &["comment", "description"]);

    reader
        .records()
        .map(|record| {
            let record = record.map_err(csv_record_error)?;
            Ok(DrillingObservationRow {
                measured_depth: optional_f64(&record, md_index),
                event_kind: required_string(&record, kind_index, "event_kind")?,
                value: optional_f64(&record, value_index),
                unit: optional_string(&record, unit_index),
                timestamp: optional_string(&record, timestamp_index),
                comment: optional_string(&record, comment_index),
            })
        })
        .collect()
}

pub fn write_trajectory_package(path: &Path, rows: &[TrajectoryRow]) -> Result<()> {
    write_record_batch(path, trajectory_batch(rows), trajectory_metadata(rows))
}

pub fn write_tops_package(path: &Path, rows: &[TopRow]) -> Result<()> {
    write_record_batch(path, tops_batch(rows), tops_metadata(rows))
}

pub fn write_pressure_package(path: &Path, rows: &[PressureObservationRow]) -> Result<()> {
    write_record_batch(path, pressure_batch(rows), pressure_metadata(rows))
}

pub fn write_well_markers_package(path: &Path, rows: &[WellMarkerRow]) -> Result<()> {
    write_record_batch(path, well_markers_batch(rows), well_marker_metadata(rows))
}

pub fn write_well_marker_horizon_residuals_package(
    path: &Path,
    rows: &[WellMarkerHorizonResidualRow],
) -> Result<()> {
    write_record_batch(
        path,
        well_marker_horizon_residuals_batch(rows),
        well_marker_horizon_residual_metadata(rows),
    )
}

pub fn write_drilling_package(path: &Path, rows: &[DrillingObservationRow]) -> Result<()> {
    write_record_batch(path, drilling_batch(rows), drilling_metadata(rows))
}

pub fn read_trajectory_rows(
    path: &Path,
    range: Option<&DepthRangeQuery>,
) -> Result<Vec<TrajectoryRow>> {
    let batch = read_batch(path)?;
    let rows = trajectory_rows_from_batch(&batch)?;
    Ok(filter_trajectory_rows(rows, range))
}

pub fn read_tops_rows(path: &Path) -> Result<Vec<TopRow>> {
    tops_rows_from_batch(&read_batch(path)?)
}

pub fn read_pressure_rows(
    path: &Path,
    range: Option<&DepthRangeQuery>,
) -> Result<Vec<PressureObservationRow>> {
    let batch = read_batch(path)?;
    let rows = pressure_rows_from_batch(&batch)?;
    Ok(filter_pressure_rows(rows, range))
}

pub fn read_well_marker_rows(path: &Path) -> Result<Vec<WellMarkerRow>> {
    well_marker_rows_from_batch(&read_batch(path)?)
}

pub fn read_well_marker_horizon_residual_rows(
    path: &Path,
) -> Result<Vec<WellMarkerHorizonResidualRow>> {
    well_marker_horizon_residual_rows_from_batch(&read_batch(path)?)
}

pub fn read_drilling_rows(
    path: &Path,
    range: Option<&DepthRangeQuery>,
) -> Result<Vec<DrillingObservationRow>> {
    let batch = read_batch(path)?;
    let rows = drilling_rows_from_batch(&batch)?;
    Ok(filter_drilling_rows(rows, range))
}

fn write_record_batch(path: &Path, batch: RecordBatch, metadata: AssetTableMetadata) -> Result<()> {
    std::fs::create_dir_all(path)?;
    std::fs::write(
        path.join("metadata.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    let file = File::create(path.join(DATA_FILENAME))?;
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .set_statistics_enabled(EnabledStatistics::Page)
        .build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
        .map_err(|error| LasError::Storage(error.to_string()))?;
    writer
        .write(&batch)
        .map_err(|error| LasError::Storage(error.to_string()))?;
    writer
        .close()
        .map_err(|error| LasError::Storage(error.to_string()))?;
    Ok(())
}

fn read_batch(path: &Path) -> Result<RecordBatch> {
    let file = File::open(path.join(DATA_FILENAME))?;
    let mut reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|error| LasError::Storage(error.to_string()))?
        .with_batch_size(8192)
        .build()
        .map_err(|error| LasError::Storage(error.to_string()))?;
    reader
        .next()
        .transpose()
        .map_err(|error| LasError::Storage(error.to_string()))?
        .ok_or_else(|| LasError::Storage("asset package parquet file was empty".to_string()))
}

fn trajectory_batch(rows: &[TrajectoryRow]) -> RecordBatch {
    build_batch(vec![
        (
            "measured_depth",
            float_field(false),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| Some(row.measured_depth))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "true_vertical_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.true_vertical_depth)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "true_vertical_depth_subsea",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.true_vertical_depth_subsea)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "azimuth_deg",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.azimuth_deg).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "inclination_deg",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.inclination_deg)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "northing_offset",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.northing_offset)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "easting_offset",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.easting_offset)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
    ])
}

fn tops_batch(rows: &[TopRow]) -> RecordBatch {
    build_batch(vec![
        (
            "name",
            string_field(false),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.name.as_str()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "top_depth",
            float_field(false),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| Some(row.top_depth))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "base_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.base_depth).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source_depth_reference",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_depth_reference.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "depth_domain",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.depth_domain.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "depth_datum",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.depth_datum.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
    ])
}

fn pressure_batch(rows: &[PressureObservationRow]) -> RecordBatch {
    build_batch(vec![
        (
            "measured_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.measured_depth)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "pressure",
            float_field(false),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| Some(row.pressure))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "phase",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.phase.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "test_kind",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.test_kind.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "timestamp",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.timestamp.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
    ])
}

fn well_markers_batch(rows: &[WellMarkerRow]) -> RecordBatch {
    build_batch(vec![
        (
            "name",
            string_field(false),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.name.as_str()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "marker_kind",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.marker_kind.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "top_depth",
            float_field(false),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| Some(row.top_depth))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "base_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.base_depth).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source_depth_reference",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_depth_reference.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "depth_domain",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.depth_domain.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "depth_datum",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.depth_datum.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "note",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.note.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
    ])
}

fn well_marker_horizon_residuals_batch(rows: &[WellMarkerHorizonResidualRow]) -> RecordBatch {
    build_batch(vec![
        (
            "marker_name",
            string_field(false),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.marker_name.as_str()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "marker_kind",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.marker_kind.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source_depth",
            float_field(false),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| Some(row.source_depth))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source_depth_reference",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_depth_reference.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source_depth_domain",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_depth_domain.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "source_depth_datum",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_depth_datum.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "measured_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.measured_depth)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "true_vertical_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.true_vertical_depth)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "true_vertical_depth_subsea",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.true_vertical_depth_subsea)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "x",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.x).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "y",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.y).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "horizon_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.horizon_depth).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "residual",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.residual).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "horizon_inline_ordinal",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.horizon_inline_ordinal)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "horizon_xline_ordinal",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.horizon_xline_ordinal)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "status",
            string_field(false),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.status.as_str()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "note",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.note.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
    ])
}

fn drilling_batch(rows: &[DrillingObservationRow]) -> RecordBatch {
    build_batch(vec![
        (
            "measured_depth",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.measured_depth)
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "event_kind",
            string_field(false),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.event_kind.as_str()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "value",
            float_field(true),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.value).collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "unit",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.unit.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "timestamp",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.timestamp.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
        (
            "comment",
            string_field(true),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.comment.as_deref())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
        ),
    ])
}

fn trajectory_rows_from_batch(batch: &RecordBatch) -> Result<Vec<TrajectoryRow>> {
    let md = float_column(batch, "measured_depth")?;
    let tvd = optional_float_column(batch, "true_vertical_depth")?;
    let tvdss = optional_float_column(batch, "true_vertical_depth_subsea")?;
    let azimuth = optional_float_column(batch, "azimuth_deg")?;
    let inclination = optional_float_column(batch, "inclination_deg")?;
    let northing = optional_float_column(batch, "northing_offset")?;
    let easting = optional_float_column(batch, "easting_offset")?;
    Ok((0..batch.num_rows())
        .map(|idx| TrajectoryRow {
            measured_depth: md[idx].unwrap_or(f64::NAN),
            true_vertical_depth: tvd[idx],
            true_vertical_depth_subsea: tvdss[idx],
            azimuth_deg: azimuth[idx],
            inclination_deg: inclination[idx],
            northing_offset: northing[idx],
            easting_offset: easting[idx],
        })
        .collect())
}

fn tops_rows_from_batch(batch: &RecordBatch) -> Result<Vec<TopRow>> {
    let names = string_column(batch, "name")?;
    let top = float_column(batch, "top_depth")?;
    let base = optional_float_column(batch, "base_depth")?;
    let source = optional_string_column(batch, "source")?;
    let source_reference = optional_string_column_with_fallback(
        batch,
        &["source_depth_reference", "depth_reference"],
    )?;
    let depth_domain = optional_string_column_if_present(batch, "depth_domain")?;
    let depth_datum = optional_string_column_if_present(batch, "depth_datum")?;
    Ok((0..batch.num_rows())
        .map(|idx| {
            let mut row = TopRow {
                name: names[idx].clone().unwrap_or_default(),
                top_depth: top[idx].unwrap_or(f64::NAN),
                base_depth: base[idx],
                source: source[idx].clone(),
                source_depth_reference: source_reference[idx].clone(),
                depth_domain: depth_domain[idx].clone(),
                depth_datum: depth_datum[idx].clone(),
            };
            normalize_top_row_depth_semantics(&mut row);
            row
        })
        .collect())
}

fn pressure_rows_from_batch(batch: &RecordBatch) -> Result<Vec<PressureObservationRow>> {
    let md = optional_float_column(batch, "measured_depth")?;
    let pressure = float_column(batch, "pressure")?;
    let phase = optional_string_column(batch, "phase")?;
    let kind = optional_string_column(batch, "test_kind")?;
    let timestamp = optional_string_column(batch, "timestamp")?;
    Ok((0..batch.num_rows())
        .map(|idx| PressureObservationRow {
            measured_depth: md[idx],
            pressure: pressure[idx].unwrap_or(f64::NAN),
            phase: phase[idx].clone(),
            test_kind: kind[idx].clone(),
            timestamp: timestamp[idx].clone(),
        })
        .collect())
}

fn well_marker_rows_from_batch(batch: &RecordBatch) -> Result<Vec<WellMarkerRow>> {
    let names = string_column(batch, "name")?;
    let marker_kinds = optional_string_column(batch, "marker_kind")?;
    let top = float_column(batch, "top_depth")?;
    let base = optional_float_column(batch, "base_depth")?;
    let source = optional_string_column(batch, "source")?;
    let source_reference = optional_string_column_with_fallback(
        batch,
        &["source_depth_reference", "depth_reference"],
    )?;
    let depth_domain = optional_string_column_if_present(batch, "depth_domain")?;
    let depth_datum = optional_string_column_if_present(batch, "depth_datum")?;
    let note = optional_string_column(batch, "note")?;
    Ok((0..batch.num_rows())
        .map(|idx| {
            let mut row = WellMarkerRow {
                name: names[idx].clone().unwrap_or_default(),
                marker_kind: marker_kinds[idx].clone(),
                top_depth: top[idx].unwrap_or(f64::NAN),
                base_depth: base[idx],
                source: source[idx].clone(),
                source_depth_reference: source_reference[idx].clone(),
                depth_domain: depth_domain[idx].clone(),
                depth_datum: depth_datum[idx].clone(),
                note: note[idx].clone(),
            };
            normalize_well_marker_row_depth_semantics(&mut row);
            row
        })
        .collect())
}

fn well_marker_horizon_residual_rows_from_batch(
    batch: &RecordBatch,
) -> Result<Vec<WellMarkerHorizonResidualRow>> {
    let marker_names = string_column(batch, "marker_name")?;
    let marker_kinds = optional_string_column(batch, "marker_kind")?;
    let source_depths = float_column(batch, "source_depth")?;
    let source_depth_references = optional_string_column(batch, "source_depth_reference")?;
    let source_depth_domains = optional_string_column_if_present(batch, "source_depth_domain")?;
    let source_depth_datums = optional_string_column_if_present(batch, "source_depth_datum")?;
    let measured_depths = optional_float_column(batch, "measured_depth")?;
    let tvds = optional_float_column(batch, "true_vertical_depth")?;
    let tvdss = optional_float_column(batch, "true_vertical_depth_subsea")?;
    let xs = optional_float_column(batch, "x")?;
    let ys = optional_float_column(batch, "y")?;
    let horizon_depths = optional_float_column(batch, "horizon_depth")?;
    let residuals = optional_float_column(batch, "residual")?;
    let inline_ordinals = optional_float_column(batch, "horizon_inline_ordinal")?;
    let xline_ordinals = optional_float_column(batch, "horizon_xline_ordinal")?;
    let statuses = string_column(batch, "status")?;
    let notes = optional_string_column(batch, "note")?;
    Ok((0..batch.num_rows())
        .map(|idx| WellMarkerHorizonResidualRow {
            marker_name: marker_names[idx].clone().unwrap_or_default(),
            marker_kind: marker_kinds[idx].clone(),
            source_depth: source_depths[idx].unwrap_or(f64::NAN),
            source_depth_reference: source_depth_references[idx].clone(),
            source_depth_domain: source_depth_domains[idx].clone(),
            source_depth_datum: source_depth_datums[idx].clone(),
            measured_depth: measured_depths[idx],
            true_vertical_depth: tvds[idx],
            true_vertical_depth_subsea: tvdss[idx],
            x: xs[idx],
            y: ys[idx],
            horizon_depth: horizon_depths[idx],
            residual: residuals[idx],
            horizon_inline_ordinal: inline_ordinals[idx],
            horizon_xline_ordinal: xline_ordinals[idx],
            status: statuses[idx].clone().unwrap_or_default(),
            note: notes[idx].clone(),
        })
        .collect())
}

fn drilling_rows_from_batch(batch: &RecordBatch) -> Result<Vec<DrillingObservationRow>> {
    let md = optional_float_column(batch, "measured_depth")?;
    let kind = string_column(batch, "event_kind")?;
    let value = optional_float_column(batch, "value")?;
    let unit = optional_string_column(batch, "unit")?;
    let timestamp = optional_string_column(batch, "timestamp")?;
    let comment = optional_string_column(batch, "comment")?;
    Ok((0..batch.num_rows())
        .map(|idx| DrillingObservationRow {
            measured_depth: md[idx],
            event_kind: kind[idx].clone().unwrap_or_default(),
            value: value[idx],
            unit: unit[idx].clone(),
            timestamp: timestamp[idx].clone(),
            comment: comment[idx].clone(),
        })
        .collect())
}

fn filter_trajectory_rows(
    rows: Vec<TrajectoryRow>,
    range: Option<&DepthRangeQuery>,
) -> Vec<TrajectoryRow> {
    match range {
        Some(range) => rows
            .into_iter()
            .filter(|row| depth_matches(row.measured_depth, range))
            .collect(),
        None => rows,
    }
}

fn filter_pressure_rows(
    rows: Vec<PressureObservationRow>,
    range: Option<&DepthRangeQuery>,
) -> Vec<PressureObservationRow> {
    match range {
        Some(range) => rows
            .into_iter()
            .filter(|row| {
                row.measured_depth
                    .is_none_or(|depth| depth_matches(depth, range))
            })
            .collect(),
        None => rows,
    }
}

fn filter_drilling_rows(
    rows: Vec<DrillingObservationRow>,
    range: Option<&DepthRangeQuery>,
) -> Vec<DrillingObservationRow> {
    match range {
        Some(range) => rows
            .into_iter()
            .filter(|row| {
                row.measured_depth
                    .is_none_or(|depth| depth_matches(depth, range))
            })
            .collect(),
        None => rows,
    }
}

fn depth_matches(value: f64, range: &DepthRangeQuery) -> bool {
    if let Some(min) = range.depth_min
        && value < min
    {
        return false;
    }
    if let Some(max) = range.depth_max
        && value > max
    {
        return false;
    }
    true
}

fn build_batch(columns: Vec<(&str, Field, ArrayRef)>) -> RecordBatch {
    let fields = columns
        .iter()
        .map(|(name, field, _)| {
            Field::new(
                name.to_string(),
                field.data_type().clone(),
                field.is_nullable(),
            )
        })
        .collect::<Vec<_>>();
    let arrays = columns
        .into_iter()
        .map(|(_, _, array)| array)
        .collect::<Vec<_>>();
    RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays).expect("valid asset record batch")
}

fn float_field(nullable: bool) -> Field {
    Field::new("value", DataType::Float64, nullable)
}

fn string_field(nullable: bool) -> Field {
    Field::new("value", DataType::Utf8, nullable)
}

fn column(
    name: &str,
    data_type: AssetColumnType,
    unit: Option<&str>,
    nullable: bool,
    is_index: bool,
) -> AssetColumnMetadata {
    AssetColumnMetadata {
        name: name.to_string(),
        data_type,
        unit: unit.map(str::to_string),
        description: None,
        nullable,
        is_index,
    }
}

fn required_header(headers: &StringRecord, candidates: &[&str]) -> Result<usize> {
    optional_header(headers, candidates).ok_or_else(|| {
        LasError::Validation(format!(
            "csv is missing required header; expected one of {}",
            candidates.join(", ")
        ))
    })
}

fn optional_header(headers: &StringRecord, candidates: &[&str]) -> Option<usize> {
    headers.iter().position(|header| {
        let normalized = header.trim().to_ascii_lowercase();
        candidates
            .iter()
            .any(|candidate| normalized == candidate.trim().to_ascii_lowercase())
    })
}

fn required_f64(record: &StringRecord, index: usize, field: &str) -> Result<f64> {
    optional_f64(record, Some(index)).ok_or_else(|| {
        LasError::Validation(format!(
            "csv row is missing required numeric field '{field}'"
        ))
    })
}

fn optional_f64(record: &StringRecord, index: Option<usize>) -> Option<f64> {
    let text = index.and_then(|idx| record.get(idx))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse::<f64>().ok()
    }
}

fn required_string(record: &StringRecord, index: usize, field: &str) -> Result<String> {
    optional_string(record, Some(index))
        .ok_or_else(|| LasError::Validation(format!("csv row is missing required field '{field}'")))
}

fn optional_string(record: &StringRecord, index: Option<usize>) -> Option<String> {
    let value = index.and_then(|idx| record.get(idx))?.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn csv_record_error(error: csv::Error) -> LasError {
    LasError::Parse(format!("failed to read csv record: {error}"))
}

fn float_column(batch: &RecordBatch, name: &str) -> Result<Vec<Option<f64>>> {
    let array = batch.column_by_name(name).ok_or_else(|| {
        LasError::Storage(format!("missing float column '{name}' in asset package"))
    })?;
    let floats = array
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| LasError::Storage(format!("column '{name}' was not Float64")))?;
    Ok((0..floats.len())
        .map(|idx| (!floats.is_null(idx)).then_some(floats.value(idx)))
        .collect())
}

fn optional_float_column(batch: &RecordBatch, name: &str) -> Result<Vec<Option<f64>>> {
    float_column(batch, name)
}

fn string_column(batch: &RecordBatch, name: &str) -> Result<Vec<Option<String>>> {
    let array = batch.column_by_name(name).ok_or_else(|| {
        LasError::Storage(format!("missing string column '{name}' in asset package"))
    })?;
    let strings = array
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| LasError::Storage(format!("column '{name}' was not Utf8")))?;
    Ok((0..strings.len())
        .map(|idx| (!strings.is_null(idx)).then_some(strings.value(idx).to_string()))
        .collect())
}

fn optional_string_column(batch: &RecordBatch, name: &str) -> Result<Vec<Option<String>>> {
    string_column(batch, name)
}

fn optional_string_column_if_present(
    batch: &RecordBatch,
    name: &str,
) -> Result<Vec<Option<String>>> {
    if batch.column_by_name(name).is_some() {
        optional_string_column(batch, name)
    } else {
        Ok(vec![None; batch.num_rows()])
    }
}

fn optional_string_column_with_fallback(
    batch: &RecordBatch,
    names: &[&str],
) -> Result<Vec<Option<String>>> {
    for name in names {
        if batch.column_by_name(name).is_some() {
            return optional_string_column(batch, name);
        }
    }
    Ok(vec![None; batch.num_rows()])
}

fn numeric_extent<I>(values: I, row_count: usize) -> (Option<f64>, Option<f64>, Option<usize>)
where
    I: Iterator<Item = f64>,
{
    let mut start = None;
    let mut stop = None;
    for value in values {
        start = Some(start.map_or(value, |current: f64| current.min(value)));
        stop = Some(stop.map_or(value, |current: f64| current.max(value)));
    }
    (start, stop, Some(row_count))
}
