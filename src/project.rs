use crate::project_assets::{
    WellMarkerHorizonResidualRow, data_filename, depth_reference_for_kind, drilling_extent,
    drilling_metadata, inferred_reference_metadata_for_top_rows,
    inferred_reference_metadata_for_well_marker_rows, parse_drilling_csv, parse_pressure_csv,
    parse_tops_csv, parse_trajectory_csv, parse_well_markers_csv, pressure_extent,
    pressure_metadata, read_drilling_rows, read_pressure_rows, read_tops_rows,
    read_trajectory_rows, read_well_marker_horizon_residual_rows, read_well_marker_rows,
    tops_extent, tops_metadata, trajectory_extent, trajectory_metadata, vertical_datum_for_kind,
    well_marker_extent, well_marker_horizon_residual_extent, well_marker_horizon_residual_metadata,
    well_marker_metadata, write_drilling_package, write_pressure_package, write_tops_package,
    write_trajectory_package, write_well_marker_horizon_residuals_package,
    write_well_markers_package,
};
use crate::project_contracts::{
    CoordinateReferenceBindingDto, CoordinateReferenceDto, CoordinateReferenceSourceDto,
    ProjectSurveyMapRequestDto, ProjectedPoint2Dto, ProjectedPolygon2Dto, ProjectedVector2Dto,
    ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION, ResolveSectionWellOverlaysResponse,
    ResolvedRockPhysicsCrossplotSourceDto, ResolvedSectionWellOverlayDto,
    ResolvedSurveyMapHorizonDto, ResolvedSurveyMapSourceDto, ResolvedSurveyMapSurveyDto,
    ResolvedSurveyMapWellDto, ResolvedWellMarkerHorizonResidualSourceDto,
    ResolvedWellPanelSourceDto, ResolvedWellPanelWellDto, RockPhysicsAxisDto,
    RockPhysicsCategoricalColorBindingDto, RockPhysicsCategoricalSemanticDto,
    RockPhysicsCategoryDto, RockPhysicsColorRequestDto, RockPhysicsContinuousColorBindingDto,
    RockPhysicsCrossplotRequestDto, RockPhysicsCurveSemanticDto,
    RockPhysicsInteractionThresholdsDto, RockPhysicsSampleDto, RockPhysicsSourceBindingDto,
    RockPhysicsTemplateIdDto, RockPhysicsWellDto, SECTION_WELL_OVERLAY_CONTRACT_VERSION,
    SURVEY_MAP_CONTRACT_VERSION, SectionWellOverlayDomainDto, SectionWellOverlayRequestDto,
    SectionWellOverlaySampleDto, SectionWellOverlaySegmentDto, SurveyIndexAxisDto,
    SurveyIndexGridDto, SurveyMapGridTransformDto, SurveyMapScalarFieldDto,
    SurveyMapSpatialAvailabilityDto, SurveyMapSpatialDescriptorDto, SurveyMapTrajectoryDto,
    SurveyMapTrajectoryStationDto, SurveyMapTransformDiagnosticsDto, SurveyMapTransformPolicyDto,
    SurveyMapTransformStatusDto, WELL_MARKER_HORIZON_RESIDUAL_CONTRACT_VERSION,
    WELL_PANEL_CONTRACT_VERSION, WellMarkerHorizonResidualRequestDto,
    WellMarkerHorizonResidualRowDto, WellPanelDepthSampleDto, WellPanelDrillingObservationDto,
    WellPanelDrillingSetDto, WellPanelLogCurveDto, WellPanelPressureObservationDto,
    WellPanelPressureSetDto, WellPanelRequestDto, WellPanelTopRowDto, WellPanelTopSetDto,
    WellPanelTrajectoryDto, WellPanelTrajectoryRowDto,
};
use crate::{
    AssetBindingInput, AssetTableMetadata, DepthRangeQuery, DrillingObservationRow, IndexKind,
    IngestIssue, LasError, LasFile, PressureObservationRow, Provenance, Result, TopRow,
    TrajectoryRow, WellInfo, WellMarkerRow, package_metadata_for, read_path,
    revision_token_for_bytes, write_package_overwrite,
};
use ophiolite_compute::{
    AssetSemanticFamily, BUILTIN_OPERATOR_PACKAGE_NAME, ComputeAvailability, ComputeCatalog,
    ComputeCatalogEntry, ComputeExecutionManifest, ComputeInputBinding, ComputeInputSpec,
    ComputeParameterDefinition, ComputeParameterValue, ComputeRegistry, CurveSemanticDescriptor,
    CurveSemanticSource, CurveSemanticType, DrillingObservationDataRow, ExternalOperatorRequest,
    ExternalOperatorRequestPayload, ExternalOperatorResponse, ExternalOperatorResponsePayload,
    LogCurveData, OperatorManifest, OperatorPackageManifest, OperatorRuntimeKind,
    PressureObservationDataRow, TopDataRow, TrajectoryDataRow, catalog_entry_for_operator_manifest,
    classify_curve_semantic, load_operator_package_manifest, resolve_log_input_bindings,
    validate_compute_parameters,
};
use ophiolite_core::{CurveItem, LasValue, SectionItems, derive_canonical_alias};
use ophiolite_package::open_package;
use ophiolite_seismic::{
    CheckshotVspObservationSet1D, CoordinateReferenceDescriptor, DatasetSummary,
    DepthReferenceKind, ManualTimeDepthPickSet1D, ProjectedPoint2, ResolvedTrajectoryGeometry,
    ResolvedTrajectoryStation, SectionAxis, SeismicAssetFamily, SeismicTraceDataDescriptor,
    TimeDepthTransformSourceKind, TrajectoryValueOrigin, TravelTimeReference, VolumeDescriptor,
    WellTieAnalysis1D, WellTieCurve1D, WellTieLogCurveSource, WellTieLogSelection1D,
    WellTieObservationSet1D, WellTieSectionWindow, WellTieTrace1D, WellTieVelocitySourceKind,
    WellTieWavelet, WellTimeDepthAuthoredModel1D, WellTimeDepthModel1D, WellboreGeometry,
};
use ophiolite_seismic_runtime::{
    HorizonSourceImportCanonicalDraft, ImportedHorizonDescriptor, ImportedHorizonGrid,
    TbvolManifest, describe_store, import_horizon_xyzs_from_draft, ingest_volume,
    load_horizon_grids, open_store, preview_horizon_source_import,
    set_any_store_native_coordinate_reference,
};
use proj::{Proj, ProjBuilder};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PROJECT_SCHEMA_VERSION: &str = "0.6.0";
const PROJECT_MANIFEST_FILENAME: &str = "ophiolite-project.json";
const PROJECT_CATALOG_FILENAME: &str = "catalog.sqlite";
const ASSET_MANIFEST_FILENAME: &str = "asset_manifest.json";
const PROJECT_REVISION_STORE_DIRNAME: &str = ".ophiolite";
const PROJECT_ASSET_REVISION_STORE_DIRNAME: &str = "asset-revisions";
const PROJECT_OPERATOR_PACKAGE_STORE_DIRNAME: &str = "operator-packages";
const PROJECT_OPERATOR_PACKAGE_PYTHON_ENV_DIRNAME: &str = ".venv";
const WELL_MARKER_HORIZON_RESIDUAL_FUNCTION_ID: &str = "well_markers:depth_horizon_residuals";
const WELL_MARKER_HORIZON_RESIDUAL_FUNCTION_VERSION: &str = "1.0.0";
const WELL_MARKER_HORIZON_RESIDUAL_SURVEY_ASSET_PARAMETER: &str = "survey_asset_id";
const WELL_MARKER_HORIZON_RESIDUAL_HORIZON_PARAMETER: &str = "horizon_id";
const WELL_MARKER_HORIZON_RESIDUAL_MARKER_PARAMETER: &str = "marker_name";
const WELL_MARKER_HORIZON_RESIDUAL_SAMPLING_METHOD: &str = "bilinear";
const WELL_MARKER_HORIZON_RESIDUAL_DEPTH_REFERENCE: &str = "tvd";
const WELL_MARKER_HORIZON_RESIDUAL_SIGN_CONVENTION: &str = "marker_minus_horizon";
const PROJECT_STAGING_DIRNAME: &str = "staging";
const PROJECT_MAP_TRANSFORM_CACHE_DIRNAME: &str = "map-transform-cache";
const SURVEY_MAP_TRANSFORM_CACHE_SCHEMA_VERSION: u32 = 1;
const PROJECT_OPERATOR_LOCK_SCHEMA_VERSION: u32 = 1;
const PROJECT_OPERATOR_PACKAGE_MANIFEST_FILENAME: &str = "operator-package.json";
const PROJ_RESOURCE_PATH_ENV: &str = "OPHIOLITE_PROJ_RESOURCE_PATH";
const CHECKSHOT_VSP_OBSERVATION_SET_FILENAME: &str = "checkshot_vsp_observation_set.json";
const MANUAL_TIME_DEPTH_PICK_SET_FILENAME: &str = "manual_time_depth_pick_set.json";
const WELL_TIE_OBSERVATION_SET_FILENAME: &str = "well_tie_observation_set.json";
const WELL_TIME_DEPTH_AUTHORED_MODEL_FILENAME: &str = "well_time_depth_authored_model.json";
const WELL_TIME_DEPTH_MODEL_FILENAME: &str = "well_time_depth_model.json";

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectWellTimeDepthPreviewIssueSeverity {
    Info,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellTimeDepthPreviewIssue {
    pub severity: ProjectWellTimeDepthPreviewIssueSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellTimeDepthAssetPreview {
    pub asset_kind: String,
    pub json_path: String,
    pub can_import: bool,
    pub id: Option<String>,
    pub name: Option<String>,
    pub wellbore_id: Option<String>,
    pub depth_reference: Option<String>,
    pub travel_time_reference: Option<String>,
    pub sample_count: Option<usize>,
    pub note_count: Option<usize>,
    pub source_kind: Option<String>,
    pub source_binding_count: Option<usize>,
    pub assumption_interval_count: Option<usize>,
    pub sampling_step_m: Option<f64>,
    pub resolved_trajectory_fingerprint: Option<String>,
    pub source_well_time_depth_model_asset_id: Option<String>,
    pub tie_window_start_ms: Option<f64>,
    pub tie_window_end_ms: Option<f64>,
    pub trace_search_radius_m: Option<f64>,
    pub bulk_shift_ms: Option<f64>,
    pub stretch_factor: Option<f64>,
    pub trace_search_offset_m: Option<f64>,
    pub correlation: Option<f64>,
    pub issues: Vec<ProjectWellTimeDepthPreviewIssue>,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellTimeDepthImportCanonicalDraft {
    pub asset_kind: String,
    pub json_payload: String,
    pub collection_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWellTimeDepthImportPreview {
    pub parsed: ProjectWellTimeDepthAssetPreview,
    pub suggested_draft: ProjectWellTimeDepthImportCanonicalDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OphioliteProjectManifest {
    pub schema_version: String,
    pub created_at_unix_seconds: u64,
    #[serde(default)]
    pub operator_lock: ProjectOperatorLock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorPackageSourceKind {
    BuiltIn,
    Path,
    PythonDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorPackageLockEntry {
    pub package_name: String,
    pub package_version: String,
    pub provider: String,
    pub runtime: OperatorRuntimeKind,
    pub source_kind: OperatorPackageSourceKind,
    pub source_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectOperatorLock {
    pub schema_version: u32,
    #[serde(default)]
    pub packages: Vec<OperatorPackageLockEntry>,
}

impl Default for ProjectOperatorLock {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_OPERATOR_LOCK_SCHEMA_VERSION,
            packages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WellId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WellboreId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WellMarkerId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetCollectionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetRevisionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetKind {
    Log,
    Trajectory,
    TopSet,
    WellMarkerSet,
    WellMarkerHorizonResidualSet,
    PressureObservation,
    DrillingObservation,
    CheckshotVspObservationSet,
    ManualTimeDepthPickSet,
    WellTieObservationSet,
    WellTimeDepthAuthoredModel,
    WellTimeDepthModel,
    RawSourceBundle,
    SeismicTraceData,
}

impl AssetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Trajectory => "trajectory",
            Self::TopSet => "top_set",
            Self::WellMarkerSet => "well_marker_set",
            Self::WellMarkerHorizonResidualSet => "well_marker_horizon_residual_set",
            Self::PressureObservation => "pressure_observation",
            Self::DrillingObservation => "drilling_observation",
            Self::CheckshotVspObservationSet => "checkshot_vsp_observation_set",
            Self::ManualTimeDepthPickSet => "manual_time_depth_pick_set",
            Self::WellTieObservationSet => "well_tie_observation_set",
            Self::WellTimeDepthAuthoredModel => "well_time_depth_authored_model",
            Self::WellTimeDepthModel => "well_time_depth_model",
            Self::RawSourceBundle => "raw_source_bundle",
            Self::SeismicTraceData => "seismic_trace_data",
        }
    }

    fn asset_dir_name(&self) -> &'static str {
        match self {
            Self::Log => "logs",
            Self::Trajectory => "trajectory",
            Self::TopSet => "tops",
            Self::WellMarkerSet => "well-markers",
            Self::WellMarkerHorizonResidualSet => "well-marker-horizon-residuals",
            Self::PressureObservation => "pressure",
            Self::DrillingObservation => "drilling",
            Self::CheckshotVspObservationSet => "checkshot-vsp-observations",
            Self::ManualTimeDepthPickSet => "manual-time-depth-picks",
            Self::WellTieObservationSet => "well-tie-observations",
            Self::WellTimeDepthAuthoredModel => "well-time-depth-authored-models",
            Self::WellTimeDepthModel => "well-time-depth-models",
            Self::RawSourceBundle => "raw-source-bundles",
            Self::SeismicTraceData => "seismic-trace-data",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "log" => Ok(Self::Log),
            "trajectory" => Ok(Self::Trajectory),
            "top_set" => Ok(Self::TopSet),
            "well_marker_set" => Ok(Self::WellMarkerSet),
            "well_marker_horizon_residual_set" => Ok(Self::WellMarkerHorizonResidualSet),
            "pressure_observation" => Ok(Self::PressureObservation),
            "drilling_observation" => Ok(Self::DrillingObservation),
            "checkshot_vsp_observation_set" => Ok(Self::CheckshotVspObservationSet),
            "manual_time_depth_pick_set" => Ok(Self::ManualTimeDepthPickSet),
            "well_tie_observation_set" => Ok(Self::WellTieObservationSet),
            "well_time_depth_authored_model" => Ok(Self::WellTimeDepthAuthoredModel),
            "well_time_depth_model" => Ok(Self::WellTimeDepthModel),
            "raw_source_bundle" => Ok(Self::RawSourceBundle),
            "seismic_trace_data" => Ok(Self::SeismicTraceData),
            _ => Err(LasError::Validation(format!(
                "unknown asset kind '{value}' in project catalog"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetStatus {
    Imported,
    Validated,
    Bound,
    NeedsReview,
    Rejected,
    Superseded,
}

impl AssetStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::Validated => "validated",
            Self::Bound => "bound",
            Self::NeedsReview => "needs_review",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "imported" => Ok(Self::Imported),
            "validated" => Ok(Self::Validated),
            "bound" => Ok(Self::Bound),
            "needs_review" => Ok(Self::NeedsReview),
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            _ => Err(LasError::Validation(format!(
                "unknown asset status '{value}' in project catalog"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WellIdentifierSet {
    pub primary_name: Option<String>,
    pub uwi: Option<String>,
    pub api: Option<String>,
    pub operator_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DepthReference {
    MeasuredDepth,
    TrueVerticalDepth,
    TrueVerticalDepthSubsea,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalDatum {
    KellyBushing,
    RotaryTable,
    GroundLevel,
    DrillFloor,
    MeanSeaLevel,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinateReference {
    pub id: Option<String>,
    pub name: Option<String>,
    pub geodetic_datum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OperatorAssignment {
    pub organisation_name: Option<String>,
    pub organisation_id: Option<String>,
    pub effective_at: Option<String>,
    pub terminated_at: Option<String>,
    pub source: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExternalReference {
    pub system: String,
    pub id: String,
    pub kind: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocatedPoint {
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub point: ProjectedPoint2,
    pub recorded_at: Option<String>,
    pub source: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerticalMeasurementPath {
    MeasuredDepth,
    TrueVerticalDepth,
    TrueVerticalDepthSubsea,
    Elevation,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VerticalMeasurement {
    pub measurement_id: Option<String>,
    pub value: f64,
    pub unit: Option<String>,
    pub path: VerticalMeasurementPath,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub reference_measurement_id: Option<String>,
    pub reference_entity_id: Option<String>,
    pub source: Option<String>,
    pub description: Option<String>,
}

impl Default for VerticalMeasurementPath {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WellMetadata {
    pub field_name: Option<String>,
    pub block_name: Option<String>,
    pub basin_name: Option<String>,
    pub country: Option<String>,
    pub province_state: Option<String>,
    pub location_text: Option<String>,
    pub interest_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operator_history: Vec<OperatorAssignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_location: Option<LocatedPoint>,
    pub default_vertical_measurement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_vertical_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vertical_measurements: Vec<VerticalMeasurement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_references: Vec<ExternalReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WellboreMetadata {
    pub sequence_number: Option<u32>,
    pub status: Option<String>,
    pub purpose: Option<String>,
    pub trajectory_type: Option<String>,
    pub parent_wellbore_id: Option<String>,
    pub target_formation: Option<String>,
    pub primary_material: Option<String>,
    pub location_text: Option<String>,
    pub service_company_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operator_history: Vec<OperatorAssignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom_hole_location: Option<LocatedPoint>,
    pub default_vertical_measurement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_vertical_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vertical_measurements: Vec<VerticalMeasurement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_references: Vec<ExternalReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellMarkerRecord {
    pub id: WellMarkerId,
    pub wellbore_id: WellboreId,
    pub source_asset_id: Option<AssetId>,
    pub name: String,
    pub sequence_number: Option<u32>,
    pub marker_kind: Option<String>,
    pub top_measurement: VerticalMeasurement,
    pub base_measurement: Option<VerticalMeasurement>,
    pub depth_reference: Option<String>,
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_references: Vec<ExternalReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnitSystem {
    pub depth_unit: Option<String>,
    pub coordinate_unit: Option<String>,
    pub pressure_unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetExtent {
    pub index_kind: Option<IndexKind>,
    pub start: Option<f64>,
    pub stop: Option<f64>,
    pub row_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkDataDescriptor {
    pub relative_path: String,
    pub media_type: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetBlobRef {
    pub relative_path: String,
    pub media_type: String,
    pub byte_count: u64,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurveValueChangeSummary {
    pub curve_name: String,
    pub changed_value_count: usize,
    pub first_changed_row: Option<usize>,
    pub last_changed_row: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LogAssetDiffSummary {
    pub metadata_changed: bool,
    pub row_count_changed: bool,
    pub curve_count_changed: bool,
    pub curves_added: Vec<String>,
    pub curves_removed: Vec<String>,
    pub modified_curves: Vec<CurveValueChangeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StructuredAssetDiffSummary {
    pub rows_added: usize,
    pub rows_removed: usize,
    pub rows_updated: usize,
    pub extent_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DirectoryAssetDiffSummary {
    pub entry_count_changed: bool,
    pub changed_path_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetDiffSummary {
    Log(LogAssetDiffSummary),
    Trajectory(StructuredAssetDiffSummary),
    TopSet(StructuredAssetDiffSummary),
    WellMarkerSet(StructuredAssetDiffSummary),
    WellMarkerHorizonResidualSet(StructuredAssetDiffSummary),
    PressureObservation(StructuredAssetDiffSummary),
    DrillingObservation(StructuredAssetDiffSummary),
    CheckshotVspObservationSet(DirectoryAssetDiffSummary),
    ManualTimeDepthPickSet(DirectoryAssetDiffSummary),
    WellTieObservationSet(DirectoryAssetDiffSummary),
    WellTimeDepthAuthoredModel(DirectoryAssetDiffSummary),
    WellTimeDepthModel(DirectoryAssetDiffSummary),
    RawSourceBundle(DirectoryAssetDiffSummary),
    SeismicTraceData(DirectoryAssetDiffSummary),
    MetadataOnly { changed_fields: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetRevisionRecord {
    pub revision_id: AssetRevisionId,
    pub asset_id: AssetId,
    pub logical_asset_id: AssetId,
    pub asset_kind: AssetKind,
    pub parent_revision_id: Option<AssetRevisionId>,
    pub package_snapshot_rel_path: String,
    pub created_at_unix_seconds: u64,
    pub metadata_blob: AssetBlobRef,
    pub data_blob: AssetBlobRef,
    pub diff_summary: AssetDiffSummary,
    #[serde(default)]
    pub change_summary: String,
}

#[derive(Debug)]
struct StagedAssetSnapshot {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceArtifactRef {
    pub source_path: String,
    pub original_filename: String,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetReferenceMetadata {
    pub identifiers: WellIdentifierSet,
    pub coordinate_reference: Option<CoordinateReference>,
    pub vertical_datum: Option<VerticalDatum>,
    pub depth_reference: DepthReference,
    pub unit_system: UnitSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub asset_kind: AssetKind,
    pub asset_schema_version: String,
    pub logical_asset_id: AssetId,
    pub storage_asset_id: AssetId,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub asset_collection_id: AssetCollectionId,
    pub source_artifacts: Vec<SourceArtifactRef>,
    pub provenance: Provenance,
    pub diagnostics: Vec<IngestIssue>,
    pub extents: AssetExtent,
    pub bulk_data_descriptors: Vec<BulkDataDescriptor>,
    pub reference_metadata: AssetReferenceMetadata,
    pub created_at_unix_seconds: u64,
    pub imported_at_unix_seconds: u64,
    pub supersedes: Option<AssetId>,
    pub derived_from: Option<AssetId>,
    #[serde(default)]
    pub curve_semantics: Vec<CurveSemanticDescriptor>,
    #[serde(default)]
    pub compute_manifest: Option<ComputeExecutionManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellRecord {
    pub id: WellId,
    pub name: String,
    pub identifiers: WellIdentifierSet,
    pub metadata: Option<WellMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellboreRecord {
    pub id: WellboreId,
    pub well_id: WellId,
    pub name: String,
    pub identifiers: WellIdentifierSet,
    pub metadata: Option<WellboreMetadata>,
    pub geometry: Option<WellboreGeometry>,
    pub definitive_trajectory_asset_id: Option<AssetId>,
    pub definitive_marker_set_asset_id: Option<AssetId>,
    pub definitive_top_set_asset_id: Option<AssetId>,
    pub active_well_time_depth_model_asset_id: Option<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetCollectionRecord {
    pub id: AssetCollectionId,
    pub wellbore_id: WellboreId,
    pub asset_kind: AssetKind,
    pub name: String,
    pub logical_asset_id: AssetId,
    pub status: AssetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: AssetId,
    pub logical_asset_id: AssetId,
    pub collection_id: AssetCollectionId,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub asset_kind: AssetKind,
    pub status: AssetStatus,
    pub package_path: String,
    pub manifest: AssetManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSummary {
    pub root: String,
    pub catalog_path: String,
    pub manifest_path: String,
    pub well_count: usize,
    pub wellbore_count: usize,
    pub asset_collection_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellSummary {
    pub well: WellRecord,
    pub wellbore_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellboreSummary {
    pub wellbore: WellboreRecord,
    pub collection_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSurveyAssetInventoryItem {
    pub asset_id: AssetId,
    pub logical_asset_id: AssetId,
    pub collection_id: AssetCollectionId,
    pub name: String,
    pub status: AssetStatus,
    pub owner_scope: AssetOwnerScope,
    pub owner_id: String,
    pub owner_name: String,
    pub well_id: WellId,
    pub well_name: String,
    pub wellbore_id: WellboreId,
    pub wellbore_name: String,
    pub effective_coordinate_reference_id: Option<String>,
    pub effective_coordinate_reference_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectWellboreInventoryItem {
    pub well_id: WellId,
    pub well_name: String,
    pub wellbore_id: WellboreId,
    pub wellbore_name: String,
    pub trajectory_asset_count: usize,
    pub well_time_depth_model_count: usize,
    pub active_well_time_depth_model_asset_id: Option<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectWellOverlayInventory {
    pub surveys: Vec<ProjectSurveyAssetInventoryItem>,
    pub wellbores: Vec<ProjectWellboreInventoryItem>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetOwnerScope {
    Wellbore,
    Survey,
    Project,
}

impl AssetOwnerScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Wellbore => "wellbore",
            Self::Survey => "survey",
            Self::Project => "project",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "wellbore" => Ok(Self::Wellbore),
            "survey" => Ok(Self::Survey),
            "project" => Ok(Self::Project),
            _ => Err(LasError::Validation(format!(
                "unsupported asset owner scope '{value}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct AssetOwnerBinding {
    scope: AssetOwnerScope,
    owner_id: String,
    display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetCollectionSummary {
    pub collection: AssetCollectionRecord,
    pub asset_count: usize,
    pub current_asset_id: Option<AssetId>,
    pub superseded_asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssetSummary {
    pub asset: AssetRecord,
    pub is_current: bool,
    pub supersedes: Option<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportResolution {
    pub status: AssetStatus,
    pub well_id: WellId,
    pub wellbore_id: WellboreId,
    pub created_well: bool,
    pub created_wellbore: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAssetImportResult {
    pub resolution: ImportResolution,
    pub collection: AssetCollectionRecord,
    pub asset: AssetRecord,
}

pub type ProjectAssetImportResult = LogAssetImportResult;
pub type SeismicAssetImportResult = ProjectAssetImportResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SurveyHorizonImportResult {
    pub collection: AssetCollectionRecord,
    pub asset: AssetRecord,
    pub imported_horizons: Vec<ImportedHorizonDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
struct RawSourceBundleMetadata {
    schema_version: String,
    source_count: usize,
    source_artifacts: Vec<SourceArtifactRef>,
}

const PROJECT_ARCHIVE_WELL_NAME: &str = "Ophiolite Project Archive";
const PROJECT_ARCHIVE_WELLBORE_NAME: &str = "Ophiolite Project Archive";
const PROJECT_ARCHIVE_NOTE: &str =
    "System-owned archive container for preserved non-well source bundles.";
const PROJECT_ARCHIVE_OWNER_ID: &str = "project-archive";
const PROJECT_ARCHIVE_OWNER_NAME: &str = "Project Archive";

#[derive(Debug, Clone)]
struct AssetOwnerHandle {
    well: WellRecord,
    wellbore: WellboreRecord,
    identifiers: WellIdentifierSet,
    created_well: bool,
    created_wellbore: bool,
    catalog_owner: Option<AssetOwnerBinding>,
}

impl AssetOwnerHandle {
    fn import_resolution(&self) -> ImportResolution {
        ImportResolution {
            status: AssetStatus::Bound,
            well_id: self.well.id.clone(),
            wellbore_id: self.wellbore.id.clone(),
            created_well: self.created_well,
            created_wellbore: self.created_wellbore,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptedWellTieResult {
    pub observation_result: ProjectAssetImportResult,
    pub authored_result: ProjectAssetImportResult,
    pub compiled_result: ProjectAssetImportResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeismicAssetMetadata {
    pub family: SeismicAssetFamily,
    pub descriptor: VolumeDescriptor,
    pub trace_data_descriptor: SeismicTraceDataDescriptor,
    pub store: TbvolManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectComputeRunRequest {
    pub source_asset_id: AssetId,
    pub function_id: String,
    pub curve_bindings: BTreeMap<String, String>,
    pub parameters: BTreeMap<String, ComputeParameterValue>,
    pub output_collection_name: Option<String>,
    pub output_mnemonic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectComputeRunResult {
    pub collection: AssetCollectionRecord,
    pub asset: AssetRecord,
    pub execution: ComputeExecutionManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellMarkerHorizonResidualPointRecord {
    pub marker_name: String,
    pub marker_kind: Option<String>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub horizon_depth: f64,
    pub residual: f64,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellMarkerHorizonResidualResolveRequest {
    pub source_asset_id: AssetId,
    pub survey_asset_id: AssetId,
    pub horizon_id: String,
    pub marker_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectOperatorPackageInstallResult {
    pub package_name: String,
    pub package_version: String,
    pub installed_manifest_path: String,
    pub python_environment_path: Option<String>,
    pub operator_count: usize,
    pub operator_lock: ProjectOperatorLock,
}

pub struct OphioliteProject {
    root: PathBuf,
    catalog_path: PathBuf,
    connection: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SurveyMapTransformCacheArtifact {
    schema_version: u32,
    cache_key: String,
    asset_id: String,
    geometry_fingerprint: String,
    source_coordinate_reference_id: String,
    target_coordinate_reference_id: String,
    policy: SurveyMapTransformPolicyDto,
    display_spatial: SurveyMapSpatialDescriptorDto,
    transform_status: SurveyMapTransformStatusDto,
    transform_diagnostics: SurveyMapTransformDiagnosticsDto,
}

#[derive(Debug, Clone)]
struct ResidualMarkerInput {
    marker_name: String,
    marker_kind: Option<String>,
    source_depth: f64,
    source_depth_reference: Option<String>,
    source_depth_domain: Option<String>,
    source_depth_datum: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone)]
struct SectionAxisSpec {
    axis: SectionAxis,
    requested_coordinate: f64,
    inline_first: f64,
    inline_step: f64,
    xline_first: f64,
    xline_step: f64,
    trace_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct ProjectedSectionSample {
    trace_index: usize,
    trace_coordinate: f64,
    sample_value: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct SectionTrajectoryDensificationSettings {
    max_md_step_m: f64,
    max_xy_step_m: f64,
    max_vertical_step_m: f64,
}

impl OphioliteProject {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(root.join("assets"))?;
        let manifest = OphioliteProjectManifest {
            schema_version: PROJECT_SCHEMA_VERSION.to_string(),
            created_at_unix_seconds: now_unix_seconds(),
            operator_lock: ProjectOperatorLock::default(),
        };
        fs::write(
            root.join(PROJECT_MANIFEST_FILENAME),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        let catalog_path = root.join(PROJECT_CATALOG_FILENAME);
        let connection = Connection::open(&catalog_path).map_err(|error| {
            LasError::Storage(format!("failed to open project catalog: {error}"))
        })?;
        initialize_project_schema(&connection)?;
        Ok(Self {
            root,
            catalog_path,
            connection,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let manifest_path = root.join(PROJECT_MANIFEST_FILENAME);
        if !manifest_path.exists() {
            return Err(LasError::Validation(format!(
                "project manifest not found at '{}'",
                manifest_path.display()
            )));
        }
        let _: OphioliteProjectManifest =
            serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
        let catalog_path = root.join(PROJECT_CATALOG_FILENAME);
        let connection = Connection::open(&catalog_path).map_err(|error| {
            LasError::Storage(format!("failed to open project catalog: {error}"))
        })?;
        initialize_project_schema(&connection)?;
        Ok(Self {
            root,
            catalog_path,
            connection,
        })
    }

    pub fn project_manifest(&self) -> Result<OphioliteProjectManifest> {
        read_project_manifest(&self.root)
    }

    pub fn operator_lock(&self) -> Result<ProjectOperatorLock> {
        Ok(self.project_manifest()?.operator_lock)
    }

    pub fn install_operator_package(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<ProjectOperatorPackageInstallResult> {
        let manifest_path = manifest_path.as_ref();
        let package = load_operator_package_manifest(manifest_path)?;
        let install_relative_path = PathBuf::from(PROJECT_REVISION_STORE_DIRNAME)
            .join(PROJECT_OPERATOR_PACKAGE_STORE_DIRNAME)
            .join(&package.package_name)
            .join(&package.package_version)
            .join(PROJECT_OPERATOR_PACKAGE_MANIFEST_FILENAME);
        let install_path = self.root.join(&install_relative_path);
        let mut installed_package = package.clone();
        let mut python_environment_path = None;

        if let Some(parent) = install_path.parent() {
            fs::create_dir_all(parent)?;
            if matches!(package.runtime, OperatorRuntimeKind::Python)
                && let Some(entrypoint) = &package.entrypoint
            {
                installed_package.entrypoint = Some(copy_operator_package_entrypoint(
                    manifest_path.parent().unwrap_or_else(|| Path::new(".")),
                    parent,
                    entrypoint,
                )?);
                copy_operator_package_support_files(
                    manifest_path.parent().unwrap_or_else(|| Path::new(".")),
                    parent,
                )?;
                python_environment_path = Some(
                    ensure_python_operator_environment(parent)?
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        fs::write(
            &install_path,
            serde_json::to_vec_pretty(&installed_package)?,
        )?;

        let mut project_manifest = self.project_manifest()?;
        if project_manifest.operator_lock.packages.is_empty() {
            project_manifest
                .operator_lock
                .packages
                .push(builtin_operator_lock_entry());
        }

        let lock_entry = OperatorPackageLockEntry {
            package_name: installed_package.package_name.clone(),
            package_version: installed_package.package_version.clone(),
            provider: installed_package.provider.clone(),
            runtime: installed_package.runtime.clone(),
            source_kind: OperatorPackageSourceKind::Path,
            source_reference: Some(normalize_relative_path(&install_relative_path)),
        };

        match project_manifest
            .operator_lock
            .packages
            .iter_mut()
            .find(|entry| entry.package_name == lock_entry.package_name)
        {
            Some(existing) => *existing = lock_entry,
            None => project_manifest.operator_lock.packages.push(lock_entry),
        }

        write_project_manifest(&self.root, &project_manifest)?;

        Ok(ProjectOperatorPackageInstallResult {
            package_name: installed_package.package_name,
            package_version: installed_package.package_version,
            installed_manifest_path: install_path.to_string_lossy().into_owned(),
            python_environment_path,
            operator_count: installed_package.operators.len(),
            operator_lock: project_manifest.operator_lock,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn catalog_path(&self) -> &Path {
        &self.catalog_path
    }

    fn effective_operator_lock(&self) -> Result<ProjectOperatorLock> {
        let mut lock = self.operator_lock()?;
        if lock.packages.is_empty() {
            lock.packages.push(builtin_operator_lock_entry());
        }
        Ok(lock)
    }

    fn resolve_operator_package_source_path(
        &self,
        entry: &OperatorPackageLockEntry,
    ) -> Result<Option<PathBuf>> {
        match entry.source_kind {
            OperatorPackageSourceKind::BuiltIn => Ok(None),
            OperatorPackageSourceKind::Path | OperatorPackageSourceKind::PythonDistribution => {
                let source_reference = entry.source_reference.as_ref().ok_or_else(|| {
                    LasError::Validation(format!(
                        "operator package '{}@{}' is missing a source reference",
                        entry.package_name, entry.package_version
                    ))
                })?;
                let source_path = PathBuf::from(source_reference);
                Ok(Some(if source_path.is_absolute() {
                    source_path
                } else {
                    self.root.join(source_path)
                }))
            }
        }
    }

    fn resolve_operator_package_manifest(
        &self,
        entry: &OperatorPackageLockEntry,
    ) -> Result<OperatorPackageManifest> {
        let manifest = match entry.source_kind {
            OperatorPackageSourceKind::BuiltIn => {
                ComputeRegistry::new().built_in_operator_package_manifest()
            }
            OperatorPackageSourceKind::Path | OperatorPackageSourceKind::PythonDistribution => {
                let source_path = self
                    .resolve_operator_package_source_path(entry)?
                    .ok_or_else(|| {
                        LasError::Validation(format!(
                            "operator package '{}@{}' is missing a source path",
                            entry.package_name, entry.package_version
                        ))
                    })?;
                load_operator_package_manifest(source_path)?
            }
        };

        if manifest.package_name != entry.package_name {
            return Err(LasError::Validation(format!(
                "operator lock entry '{}' resolved package '{}' instead",
                entry.package_name, manifest.package_name
            )));
        }
        if manifest.package_version != entry.package_version {
            return Err(LasError::Validation(format!(
                "operator package '{}@{}' does not match locked version '{}'",
                manifest.package_name, manifest.package_version, entry.package_version
            )));
        }
        if manifest.provider != entry.provider {
            return Err(LasError::Validation(format!(
                "operator package '{}@{}' resolved provider '{}' instead of '{}'",
                manifest.package_name, manifest.package_version, manifest.provider, entry.provider
            )));
        }
        if manifest.runtime != entry.runtime {
            return Err(LasError::Validation(format!(
                "operator package '{}@{}' resolved runtime '{:?}' instead of '{:?}'",
                manifest.package_name, manifest.package_version, manifest.runtime, entry.runtime
            )));
        }

        Ok(manifest)
    }

    fn resolved_operator_packages(&self) -> Result<Vec<OperatorPackageManifest>> {
        let lock = self.effective_operator_lock()?;
        let mut manifests = Vec::with_capacity(lock.packages.len());
        let mut seen_operator_ids = BTreeMap::new();

        for entry in &lock.packages {
            let manifest = self.resolve_operator_package_manifest(entry)?;
            for operator in &manifest.operators {
                if let Some(existing) = seen_operator_ids.insert(
                    operator.id.clone(),
                    format!("{}@{}", manifest.package_name, manifest.package_version),
                ) {
                    return Err(LasError::Validation(format!(
                        "operator id '{}' is provided by both '{}' and '{}@{}'",
                        operator.id, existing, manifest.package_name, manifest.package_version
                    )));
                }
            }
            manifests.push(manifest);
        }

        Ok(manifests)
    }

    fn resolved_operator_entry(
        &self,
        function_id: &str,
    ) -> Result<(
        OperatorPackageLockEntry,
        OperatorPackageManifest,
        OperatorManifest,
    )> {
        let lock = self.effective_operator_lock()?;
        for entry in lock.packages {
            let package = self.resolve_operator_package_manifest(&entry)?;
            if let Some(operator) = package
                .operators
                .iter()
                .find(|operator| operator.id == function_id)
                .cloned()
            {
                return Ok((entry, package, operator));
            }
        }

        Err(LasError::Validation(format!(
            "operator '{function_id}' is not available in this project's operator lock"
        )))
    }

    pub fn summary(&self) -> Result<ProjectSummary> {
        let wells = self.list_wells()?;
        let well_count = wells.len();
        let mut wellbore_count = 0usize;
        let mut asset_collection_count = 0usize;
        let mut asset_count = 0usize;

        for well in &wells {
            let wellbores = self.list_wellbores(&well.id)?;
            wellbore_count += wellbores.len();
            for wellbore in &wellbores {
                asset_collection_count += self.list_asset_collections(&wellbore.id)?.len();
                asset_count += self.list_assets(&wellbore.id, None)?.len();
            }
        }

        Ok(ProjectSummary {
            root: self.root.display().to_string(),
            catalog_path: self.catalog_path.display().to_string(),
            manifest_path: self
                .root
                .join(PROJECT_MANIFEST_FILENAME)
                .display()
                .to_string(),
            well_count,
            wellbore_count,
            asset_collection_count,
            asset_count,
        })
    }

    pub fn list_wells(&self) -> Result<Vec<WellRecord>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, primary_name, identifiers_json, metadata_json FROM wells ORDER BY primary_name",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], well_record_from_row)
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn well_summaries(&self) -> Result<Vec<WellSummary>> {
        self.list_wells()?
            .into_iter()
            .map(|well| {
                let wellbores = self.list_wellbores(&well.id)?;
                let asset_count = wellbores.iter().try_fold(0usize, |count, wellbore| {
                    self.list_assets(&wellbore.id, None)
                        .map(|assets| count + assets.len())
                })?;
                Ok(WellSummary {
                    well,
                    wellbore_count: wellbores.len(),
                    asset_count,
                })
            })
            .collect()
    }

    pub fn set_well_metadata(
        &self,
        well_id: &WellId,
        metadata: Option<WellMetadata>,
    ) -> Result<WellRecord> {
        let metadata_json = metadata.as_ref().map(serde_json::to_string).transpose()?;
        self.connection
            .execute(
                "UPDATE wells SET metadata_json = ?2 WHERE id = ?1",
                params![well_id.0, metadata_json],
            )
            .map_err(sqlite_error)?;
        self.well_by_id(well_id)
    }

    pub fn list_wellbores(&self, well_id: &WellId) -> Result<Vec<WellboreRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, well_id, primary_name, identifiers_json, metadata_json, geometry_json, definitive_trajectory_asset_id, definitive_marker_set_asset_id, definitive_top_set_asset_id, active_well_time_depth_model_asset_id FROM wellbores WHERE well_id = ?1 ORDER BY primary_name",
        ).map_err(sqlite_error)?;
        let rows = statement
            .query_map([&well_id.0], wellbore_record_from_row)
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn list_well_markers(&self, wellbore_id: &WellboreId) -> Result<Vec<WellMarkerRecord>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT marker_json
                 FROM well_markers
                 WHERE wellbore_id = ?1
                 ORDER BY sequence_number ASC, name ASC, id ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([&wellbore_id.0], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str::<WellMarkerRecord>(&json).map_err(sql_json_error)
            })
            .map_err(sqlite_error)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn definitive_marker_source_asset_id(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<Option<AssetId>> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        Ok(wellbore
            .definitive_marker_set_asset_id
            .or(wellbore.definitive_top_set_asset_id))
    }

    pub fn set_wellbore_metadata(
        &self,
        wellbore_id: &WellboreId,
        metadata: Option<WellboreMetadata>,
    ) -> Result<WellboreRecord> {
        let metadata_json = metadata.as_ref().map(serde_json::to_string).transpose()?;
        self.connection
            .execute(
                "UPDATE wellbores SET metadata_json = ?2 WHERE id = ?1",
                params![wellbore_id.0, metadata_json],
            )
            .map_err(sqlite_error)?;
        self.wellbore_by_id(wellbore_id)
    }

    pub fn set_wellbore_geometry(
        &self,
        wellbore_id: &WellboreId,
        geometry: Option<WellboreGeometry>,
    ) -> Result<WellboreRecord> {
        let geometry_json = geometry.as_ref().map(serde_json::to_string).transpose()?;
        self.connection
            .execute(
                "UPDATE wellbores SET geometry_json = ?2 WHERE id = ?1",
                params![wellbore_id.0, geometry_json],
            )
            .map_err(sqlite_error)?;
        self.wellbore_by_id(wellbore_id)
    }

    pub fn set_definitive_trajectory_asset(
        &self,
        wellbore_id: &WellboreId,
        asset_id: Option<&AssetId>,
    ) -> Result<WellboreRecord> {
        if let Some(asset_id) = asset_id {
            let asset = self.asset_by_id(asset_id)?;
            require_asset_kind(&asset, AssetKind::Trajectory)?;
            if asset.wellbore_id != *wellbore_id {
                return Err(LasError::Validation(format!(
                    "trajectory asset '{}' does not belong to wellbore '{}'",
                    asset_id.0, wellbore_id.0
                )));
            }
        }
        self.connection
            .execute(
                "UPDATE wellbores SET definitive_trajectory_asset_id = ?2 WHERE id = ?1",
                params![wellbore_id.0, asset_id.map(|value| value.0.clone())],
            )
            .map_err(sqlite_error)?;
        self.wellbore_by_id(wellbore_id)
    }

    pub fn set_definitive_marker_set_asset(
        &self,
        wellbore_id: &WellboreId,
        asset_id: Option<&AssetId>,
    ) -> Result<WellboreRecord> {
        if let Some(asset_id) = asset_id {
            let asset = self.asset_by_id(asset_id)?;
            require_asset_kind(&asset, AssetKind::WellMarkerSet)?;
            if asset.wellbore_id != *wellbore_id {
                return Err(LasError::Validation(format!(
                    "well marker set asset '{}' does not belong to wellbore '{}'",
                    asset_id.0, wellbore_id.0
                )));
            }
        }
        self.connection
            .execute(
                "UPDATE wellbores SET definitive_marker_set_asset_id = ?2 WHERE id = ?1",
                params![wellbore_id.0, asset_id.map(|value| value.0.clone())],
            )
            .map_err(sqlite_error)?;
        match asset_id {
            Some(asset_id) => {
                self.sync_well_markers_from_marker_set_asset(wellbore_id, asset_id)?
            }
            None => {
                if let Some(top_set_asset_id) = self
                    .wellbore_by_id(wellbore_id)?
                    .definitive_top_set_asset_id
                {
                    self.sync_well_markers_from_top_set_asset(wellbore_id, &top_set_asset_id)?;
                } else {
                    self.replace_well_markers(wellbore_id, &[])?;
                }
            }
        }
        self.wellbore_by_id(wellbore_id)
    }

    pub fn set_definitive_top_set_asset(
        &self,
        wellbore_id: &WellboreId,
        asset_id: Option<&AssetId>,
    ) -> Result<WellboreRecord> {
        if let Some(asset_id) = asset_id {
            let asset = self.asset_by_id(asset_id)?;
            require_asset_kind(&asset, AssetKind::TopSet)?;
            if asset.wellbore_id != *wellbore_id {
                return Err(LasError::Validation(format!(
                    "top set asset '{}' does not belong to wellbore '{}'",
                    asset_id.0, wellbore_id.0
                )));
            }
        }
        self.connection
            .execute(
                "UPDATE wellbores SET definitive_top_set_asset_id = ?2 WHERE id = ?1",
                params![wellbore_id.0, asset_id.map(|value| value.0.clone())],
            )
            .map_err(sqlite_error)?;
        if self
            .wellbore_by_id(wellbore_id)?
            .definitive_marker_set_asset_id
            .is_none()
        {
            match asset_id {
                Some(asset_id) => {
                    self.sync_well_markers_from_top_set_asset(wellbore_id, asset_id)?
                }
                None => self.replace_well_markers(wellbore_id, &[])?,
            }
        }
        self.wellbore_by_id(wellbore_id)
    }

    pub fn set_active_well_time_depth_model(
        &self,
        wellbore_id: &WellboreId,
        asset_id: Option<&AssetId>,
    ) -> Result<WellboreRecord> {
        if let Some(asset_id) = asset_id {
            let asset = self.asset_by_id(asset_id)?;
            require_asset_kind(&asset, AssetKind::WellTimeDepthModel)?;
            if asset.wellbore_id != *wellbore_id {
                return Err(LasError::Validation(format!(
                    "well time-depth model '{}' does not belong to wellbore '{}'",
                    asset_id.0, wellbore_id.0
                )));
            }
        }
        self.connection
            .execute(
                "UPDATE wellbores SET active_well_time_depth_model_asset_id = ?2 WHERE id = ?1",
                params![wellbore_id.0, asset_id.map(|value| value.0.clone())],
            )
            .map_err(sqlite_error)?;
        self.wellbore_by_id(wellbore_id)
    }

    pub fn wellbore_summaries(&self, well_id: &WellId) -> Result<Vec<WellboreSummary>> {
        self.list_wellbores(well_id)?
            .into_iter()
            .map(|wellbore| {
                let collection_count = self.list_asset_collections(&wellbore.id)?.len();
                let asset_count = self.list_assets(&wellbore.id, None)?.len();
                Ok(WellboreSummary {
                    wellbore,
                    collection_count,
                    asset_count,
                })
            })
            .collect()
    }

    pub fn project_well_overlay_inventory(&self) -> Result<ProjectWellOverlayInventory> {
        let mut surveys = Vec::new();
        let mut survey_statement = self
            .connection
            .prepare(
                "SELECT id
                 FROM assets
                 WHERE asset_kind = ?1 AND status = ?2
                 ORDER BY created_at_unix_seconds DESC, id ASC",
            )
            .map_err(sqlite_error)?;
        let survey_rows = survey_statement
            .query_map(
                params![
                    AssetKind::SeismicTraceData.as_str(),
                    AssetStatus::Bound.as_str()
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(sqlite_error)?;
        for asset_id in survey_rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)?
        {
            let asset = self.asset_by_id(&AssetId(asset_id))?;
            let collection = self.collection_by_id(&asset.collection_id)?;
            let owner = self.collection_owner_binding(&collection.id)?;
            let well = self.well_by_id(&asset.well_id)?;
            let wellbore = self.wellbore_by_id(&asset.wellbore_id)?;
            surveys.push(ProjectSurveyAssetInventoryItem {
                asset_id: asset.id.clone(),
                logical_asset_id: asset.logical_asset_id.clone(),
                collection_id: collection.id.clone(),
                name: collection.name.clone(),
                status: asset.status,
                owner_scope: owner.scope,
                owner_id: owner.owner_id,
                owner_name: owner.display_name,
                well_id: well.id,
                well_name: well.name,
                wellbore_id: wellbore.id,
                wellbore_name: wellbore.name,
                effective_coordinate_reference_id: None,
                effective_coordinate_reference_name: None,
            });
        }
        surveys.sort_by(|left, right| {
            left.owner_name
                .cmp(&right.owner_name)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.wellbore_name.cmp(&right.wellbore_name))
                .then_with(|| left.asset_id.0.cmp(&right.asset_id.0))
        });
        for survey in &mut surveys {
            let asset = self.asset_by_id(&survey.asset_id)?;
            if let Ok(metadata) = read_seismic_asset_metadata(Path::new(&asset.package_path)) {
                if let Some(effective_coordinate_reference) = metadata
                    .descriptor
                    .coordinate_reference_binding
                    .as_ref()
                    .and_then(|binding| binding.effective.as_ref())
                {
                    survey.effective_coordinate_reference_id =
                        effective_coordinate_reference.id.clone();
                    survey.effective_coordinate_reference_name =
                        effective_coordinate_reference.name.clone();
                }
            }
        }

        let mut wellbores = Vec::new();
        let mut wellbore_statement = self
            .connection
            .prepare(
                "SELECT wb.id, w.id, w.primary_name, wb.primary_name, wb.active_well_time_depth_model_asset_id,
                        COALESCE(SUM(CASE WHEN a.asset_kind = 'trajectory' AND a.status = 'bound' THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN a.asset_kind = 'well_time_depth_model' AND a.status = 'bound' THEN 1 ELSE 0 END), 0)
                 FROM wellbores wb
                 JOIN wells w ON w.id = wb.well_id
                 LEFT JOIN assets a ON a.wellbore_id = wb.id
                 GROUP BY wb.id, w.id, w.primary_name, wb.primary_name, wb.active_well_time_depth_model_asset_id
                 ORDER BY w.primary_name, wb.primary_name",
            )
            .map_err(sqlite_error)?;
        let wellbore_rows = wellbore_statement
            .query_map([], |row| {
                Ok(ProjectWellboreInventoryItem {
                    wellbore_id: WellboreId(row.get(0)?),
                    well_id: WellId(row.get(1)?),
                    well_name: row.get(2)?,
                    wellbore_name: row.get(3)?,
                    active_well_time_depth_model_asset_id: row
                        .get::<_, Option<String>>(4)?
                        .map(AssetId),
                    trajectory_asset_count: row.get::<_, i64>(5)? as usize,
                    well_time_depth_model_count: row.get::<_, i64>(6)? as usize,
                })
            })
            .map_err(sqlite_error)?;
        wellbores.extend(
            wellbore_rows
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sqlite_error)?,
        );

        Ok(ProjectWellOverlayInventory { surveys, wellbores })
    }

    pub fn list_asset_collections(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<Vec<AssetCollectionRecord>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, wellbore_id, asset_kind, name, logical_asset_id, status
             FROM asset_collections
             WHERE wellbore_id = ?1
             ORDER BY asset_kind, name",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([&wellbore_id.0], |row| {
                Ok(AssetCollectionRecord {
                    id: AssetCollectionId(row.get(0)?),
                    wellbore_id: WellboreId(row.get(1)?),
                    asset_kind: AssetKind::from_str(&row.get::<_, String>(2)?)
                        .map_err(sql_validation_error)?,
                    name: row.get(3)?,
                    logical_asset_id: AssetId(row.get(4)?),
                    status: AssetStatus::from_str(&row.get::<_, String>(5)?)
                        .map_err(sql_validation_error)?,
                })
            })
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn asset_collection_summaries(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<Vec<AssetCollectionSummary>> {
        self.list_asset_collections(wellbore_id)?
            .into_iter()
            .map(|collection| {
                let assets = self.list_assets(wellbore_id, Some(collection.asset_kind.clone()))?;
                let collection_assets = assets
                    .into_iter()
                    .filter(|asset| asset.collection_id == collection.id)
                    .collect::<Vec<_>>();
                let current_asset_id = collection_assets
                    .iter()
                    .find(|asset| asset.status == AssetStatus::Bound)
                    .map(|asset| asset.id.clone());
                let superseded_asset_count = collection_assets
                    .iter()
                    .filter(|asset| asset.status == AssetStatus::Superseded)
                    .count();
                Ok(AssetCollectionSummary {
                    collection,
                    asset_count: collection_assets.len(),
                    current_asset_id,
                    superseded_asset_count,
                })
            })
            .collect()
    }

    pub fn list_assets(
        &self,
        wellbore_id: &WellboreId,
        asset_kind: Option<AssetKind>,
    ) -> Result<Vec<AssetRecord>> {
        let (sql, params): (&str, Vec<String>) = match asset_kind {
            Some(kind) => (
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE wellbore_id = ?1 AND asset_kind = ?2
                 ORDER BY created_at_unix_seconds DESC",
                vec![wellbore_id.0.clone(), kind.as_str().to_string()],
            ),
            None => (
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE wellbore_id = ?1
                 ORDER BY created_at_unix_seconds DESC",
                vec![wellbore_id.0.clone()],
            ),
        };

        let mut statement = self.connection.prepare(sql).map_err(sqlite_error)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let manifest_text: String = row.get(8)?;
                let manifest = serde_json::from_str::<AssetManifest>(&manifest_text)
                    .map_err(sql_json_error)?;
                Ok(AssetRecord {
                    id: AssetId(row.get(0)?),
                    logical_asset_id: AssetId(row.get(1)?),
                    collection_id: AssetCollectionId(row.get(2)?),
                    well_id: WellId(row.get(3)?),
                    wellbore_id: WellboreId(row.get(4)?),
                    asset_kind: AssetKind::from_str(&row.get::<_, String>(5)?)
                        .map_err(sql_validation_error)?,
                    status: AssetStatus::from_str(&row.get::<_, String>(6)?)
                        .map_err(sql_validation_error)?,
                    package_path: self
                        .root
                        .join(row.get::<_, String>(7)?)
                        .to_string_lossy()
                        .into(),
                    manifest,
                })
            })
            .map_err(sqlite_error)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn asset_summaries(
        &self,
        wellbore_id: &WellboreId,
        asset_kind: Option<AssetKind>,
    ) -> Result<Vec<ProjectAssetSummary>> {
        self.list_assets(wellbore_id, asset_kind)?
            .into_iter()
            .map(|asset| {
                Ok(ProjectAssetSummary {
                    is_current: asset.status == AssetStatus::Bound,
                    supersedes: asset.manifest.supersedes.clone(),
                    asset,
                })
            })
            .collect()
    }

    pub fn import_las(
        &mut self,
        las_path: impl AsRef<Path>,
        collection_name: Option<&str>,
    ) -> Result<LogAssetImportResult> {
        let las_path = las_path.as_ref();
        let file = read_path(las_path, &Default::default())?;
        let well_info = file.well_info();
        let identifiers = identifiers_from_well_info(&well_info);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore(&well.id, &identifiers)?;
        if created_well {
            if let Some(metadata) = well_metadata_from_well_info(&well_info) {
                self.set_well_metadata(&well.id, Some(metadata))?;
            }
        }
        if created_wellbore {
            if let Some(metadata) = wellbore_metadata_from_well_info(&well_info) {
                self.set_wellbore_metadata(&wellbore.id, Some(metadata))?;
            }
        }
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                las_path
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "log".to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, AssetKind::Log, &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::Log.asset_dir_name())
            .join(format!("{}.laspkg", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        write_package_overwrite(&file, &staged.root)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = log_asset_manifest(
            &file,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            supersedes.clone(),
        );
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind: AssetKind::Log,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest: manifest.clone(),
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::Log(Default::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(LogAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    pub fn ensure_well_binding(&self, binding: &AssetBindingInput) -> Result<ImportResolution> {
        Ok(self
            .resolve_asset_owner_for_binding(binding)?
            .import_resolution())
    }

    pub fn import_las_with_binding(
        &mut self,
        las_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<LogAssetImportResult> {
        self.import_las_with_binding_and_supporting_sources(las_path, binding, collection_name, &[])
    }

    pub fn import_las_with_binding_and_supporting_sources(
        &mut self,
        las_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        supporting_source_paths: &[&Path],
    ) -> Result<LogAssetImportResult> {
        let las_path = las_path.as_ref();
        let file = read_path(las_path, &Default::default())?;
        self.import_log_file_with_binding_and_supporting_sources(
            file,
            binding,
            collection_name,
            supporting_source_paths,
        )
    }

    pub fn import_log_file_with_binding(
        &mut self,
        file: LasFile,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<LogAssetImportResult> {
        self.import_log_file_with_binding_and_supporting_sources(
            file,
            binding,
            collection_name,
            &[],
        )
    }

    pub fn import_log_file_with_binding_and_supporting_sources(
        &mut self,
        file: LasFile,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        supporting_source_paths: &[&Path],
    ) -> Result<LogAssetImportResult> {
        let identifiers = identifiers_from_binding(binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                Path::new(&file.provenance.original_filename)
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "log".to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, AssetKind::Log, &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::Log.asset_dir_name())
            .join(format!("{}.laspkg", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        write_package_overwrite(&file, &staged.root)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let mut manifest = log_asset_manifest(
            &file,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            supersedes.clone(),
        );
        let supporting_source_paths = supporting_source_paths
            .iter()
            .copied()
            .filter(|path| path.to_string_lossy() != file.provenance.source_path)
            .collect::<Vec<_>>();
        let supporting_descriptors =
            copy_supporting_source_artifacts(&staged.root, &supporting_source_paths)?;
        if !supporting_descriptors.is_empty() {
            manifest
                .bulk_data_descriptors
                .extend(supporting_descriptors);
            manifest
                .source_artifacts
                .extend(source_artifact_refs_for_paths(
                    &supporting_source_paths,
                    manifest.imported_at_unix_seconds,
                )?);
            manifest
                .source_artifacts
                .sort_by(|left, right| left.source_path.cmp(&right.source_path));
            manifest
                .source_artifacts
                .dedup_by(|left, right| left.source_path == right.source_path);
        }
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind: AssetKind::Log,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest: manifest.clone(),
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::Log(Default::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(LogAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    pub fn import_raw_source_bundle_with_binding(
        &mut self,
        source_paths: &[&Path],
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let preserved_source_paths = source_paths
            .iter()
            .copied()
            .filter(|path| path.exists())
            .collect::<Vec<_>>();
        if preserved_source_paths.is_empty() {
            return Err(LasError::Validation(
                "raw source bundle import requires at least one existing source file".to_string(),
            ));
        }
        if let Some(directory) = preserved_source_paths.iter().find(|path| path.is_dir()) {
            return Err(LasError::Validation(format!(
                "raw source bundle import only supports files, but '{}' is a directory",
                directory.display()
            )));
        }

        let owner = self.resolve_asset_owner_for_binding(binding)?;
        self.import_raw_source_bundle_for_owner(&preserved_source_paths, &owner, collection_name)
    }

    pub fn import_raw_source_bundle_into_project_archive(
        &mut self,
        source_paths: &[&Path],
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let preserved_source_paths = source_paths
            .iter()
            .copied()
            .filter(|path| path.exists())
            .collect::<Vec<_>>();
        if preserved_source_paths.is_empty() {
            return Err(LasError::Validation(
                "raw source bundle import requires at least one existing source file".to_string(),
            ));
        }
        if let Some(directory) = preserved_source_paths.iter().find(|path| path.is_dir()) {
            return Err(LasError::Validation(format!(
                "raw source bundle import only supports files, but '{}' is a directory",
                directory.display()
            )));
        }

        let owner = self.resolve_project_archive_asset_owner()?;
        self.import_raw_source_bundle_for_owner(&preserved_source_paths, &owner, collection_name)
    }

    fn import_raw_source_bundle_for_owner(
        &mut self,
        preserved_source_paths: &[&Path],
        owner: &AssetOwnerHandle,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                preserved_source_paths
                    .first()
                    .and_then(|path| path.file_stem())
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "raw source bundle".to_string());
        let collection = self.resolve_or_create_collection(
            &owner.wellbore.id,
            AssetKind::RawSourceBundle,
            &collection_name,
        )?;
        let collection_owner = match owner.catalog_owner.as_ref() {
            Some(binding) => binding.clone(),
            None => self.infer_wellbore_owner_binding(&owner.wellbore.id)?,
        };
        self.bind_collection_owner(&collection.id, &collection_owner)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::RawSourceBundle.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        let source_artifacts =
            source_artifact_refs_for_paths(&preserved_source_paths, now_unix_seconds())?;
        fs::write(
            staged.root.join("metadata.json"),
            serde_json::to_vec_pretty(&RawSourceBundleMetadata {
                schema_version: "0.1.0".to_string(),
                source_count: source_artifacts.len(),
                source_artifacts: source_artifacts.clone(),
            })?,
        )?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = raw_source_bundle_manifest(
            &preserved_source_paths,
            source_artifacts,
            &owner.well.id,
            &owner.wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            owner.identifiers.clone(),
            supersedes.clone(),
            &staged.root,
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: owner.well.id.clone(),
            wellbore_id: owner.wellbore.id.clone(),
            asset_kind: AssetKind::RawSourceBundle,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::RawSourceBundle(DirectoryAssetDiffSummary::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;

        Ok(ProjectAssetImportResult {
            resolution: owner.import_resolution(),
            collection,
            asset,
        })
    }

    pub fn import_trajectory_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_trajectory_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::Trajectory,
            collection_name,
            |root| write_trajectory_package(root, &rows),
            trajectory_metadata(&rows),
            structured_asset_extent(AssetKind::Trajectory, trajectory_extent(&rows)),
        )
    }

    pub fn import_trajectory_rows(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[TrajectoryRow],
    ) -> Result<ProjectAssetImportResult> {
        self.import_trajectory_rows_with_supporting_sources(
            source_path,
            binding,
            collection_name,
            rows,
            &[],
        )
    }

    pub fn import_trajectory_rows_with_supporting_sources(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[TrajectoryRow],
        supporting_source_paths: &[&Path],
    ) -> Result<ProjectAssetImportResult> {
        self.import_trajectory_rows_with_coordinate_reference_and_supporting_sources(
            source_path,
            binding,
            collection_name,
            rows,
            None,
            supporting_source_paths,
        )
    }

    pub(crate) fn import_trajectory_rows_with_coordinate_reference_and_supporting_sources(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[TrajectoryRow],
        coordinate_reference: Option<CoordinateReference>,
        supporting_source_paths: &[&Path],
    ) -> Result<ProjectAssetImportResult> {
        self.import_structured_asset_with_supporting_sources(
            source_path.as_ref(),
            binding,
            AssetKind::Trajectory,
            collection_name,
            |root| write_trajectory_package(root, rows),
            trajectory_metadata(rows),
            structured_asset_extent(AssetKind::Trajectory, trajectory_extent(rows)),
            coordinate_reference,
            None,
            supporting_source_paths,
        )
    }

    pub fn import_tops_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_tops_csv(csv_path.as_ref())?;
        self.import_tops_rows(csv_path, binding, collection_name, &rows)
    }

    pub fn import_tops_rows(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[TopRow],
    ) -> Result<ProjectAssetImportResult> {
        self.import_tops_rows_with_supporting_sources(
            source_path,
            binding,
            collection_name,
            rows,
            &[],
        )
    }

    pub fn import_tops_rows_with_supporting_sources(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[TopRow],
        supporting_source_paths: &[&Path],
    ) -> Result<ProjectAssetImportResult> {
        let mut normalized_rows = rows.to_vec();
        for row in &mut normalized_rows {
            crate::project_assets::normalize_top_row_depth_semantics(row);
        }
        self.import_structured_asset_with_supporting_sources(
            source_path.as_ref(),
            binding,
            AssetKind::TopSet,
            collection_name,
            |root| write_tops_package(root, &normalized_rows),
            tops_metadata(&normalized_rows),
            structured_asset_extent(AssetKind::TopSet, tops_extent(&normalized_rows)),
            None,
            Some(inferred_reference_metadata_for_top_rows(&normalized_rows)),
            supporting_source_paths,
        )
    }

    pub fn import_well_markers_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_well_markers_csv(csv_path.as_ref())?;
        self.import_well_marker_rows(csv_path, binding, collection_name, &rows)
    }

    pub fn import_well_marker_rows(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[WellMarkerRow],
    ) -> Result<ProjectAssetImportResult> {
        self.import_well_marker_rows_with_supporting_sources(
            source_path,
            binding,
            collection_name,
            rows,
            &[],
        )
    }

    pub fn import_well_marker_rows_with_supporting_sources(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        rows: &[WellMarkerRow],
        supporting_source_paths: &[&Path],
    ) -> Result<ProjectAssetImportResult> {
        let mut normalized_rows = rows.to_vec();
        for row in &mut normalized_rows {
            crate::project_assets::normalize_well_marker_row_depth_semantics(row);
        }
        self.import_structured_asset_with_supporting_sources(
            source_path.as_ref(),
            binding,
            AssetKind::WellMarkerSet,
            collection_name,
            |root| write_well_markers_package(root, &normalized_rows),
            well_marker_metadata(&normalized_rows),
            structured_asset_extent(
                AssetKind::WellMarkerSet,
                well_marker_extent(&normalized_rows),
            ),
            None,
            Some(inferred_reference_metadata_for_well_marker_rows(
                &normalized_rows,
            )),
            supporting_source_paths,
        )
    }

    pub fn import_pressure_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_pressure_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::PressureObservation,
            collection_name,
            |root| write_pressure_package(root, &rows),
            pressure_metadata(&rows),
            structured_asset_extent(AssetKind::PressureObservation, pressure_extent(&rows)),
        )
    }

    pub fn import_drilling_csv(
        &mut self,
        csv_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let rows = parse_drilling_csv(csv_path.as_ref())?;
        self.import_structured_asset(
            csv_path.as_ref(),
            binding,
            AssetKind::DrillingObservation,
            collection_name,
            |root| write_drilling_package(root, &rows),
            drilling_metadata(&rows),
            structured_asset_extent(AssetKind::DrillingObservation, drilling_extent(&rows)),
        )
    }

    pub fn import_seismic_trace_data_store(
        &mut self,
        store_root: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<SeismicAssetImportResult> {
        let store_root = store_root.as_ref();
        self.import_seismic_store_with_kind(
            store_root,
            store_root,
            binding,
            AssetKind::SeismicTraceData,
            SeismicAssetFamily::Volume,
            collection_name,
        )
    }

    pub fn import_seismic_trace_data_store_with_coordinate_reference(
        &mut self,
        store_root: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        coordinate_reference: Option<&CoordinateReferenceDescriptor>,
    ) -> Result<SeismicAssetImportResult> {
        let store_root = store_root.as_ref();
        let Some(coordinate_reference) = coordinate_reference else {
            return self.import_seismic_trace_data_store(store_root, binding, collection_name);
        };

        let temp_root = std::env::temp_dir().join(unique_id("seismic-store-import"));
        let staged_store_root = temp_root.join(
            store_root
                .file_name()
                .map(|value| value.to_os_string())
                .unwrap_or_else(|| "store.tbvol".into()),
        );
        fs::create_dir_all(&temp_root)?;

        let result = (|| {
            copy_path(store_root, &staged_store_root)?;
            set_any_store_native_coordinate_reference(
                &staged_store_root,
                coordinate_reference.id.as_deref(),
                coordinate_reference.name.as_deref(),
            )
            .map_err(|error| {
                LasError::Storage(format!(
                    "failed to apply coordinate reference to seismic store `{}`: {error}",
                    store_root.display()
                ))
            })?;
            self.import_seismic_store_with_kind(
                &staged_store_root,
                store_root,
                binding,
                AssetKind::SeismicTraceData,
                SeismicAssetFamily::Volume,
                collection_name,
            )
        })();

        let _ = fs::remove_dir_all(&temp_root);
        result
    }

    pub fn import_seismic_volume(
        &mut self,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        collection_name: Option<&str>,
        coordinate_reference: Option<&CoordinateReferenceDescriptor>,
    ) -> Result<SeismicAssetImportResult> {
        let source_path = source_path.as_ref();
        let temp_root = std::env::temp_dir().join(unique_id("seismic-volume-import"));
        let store_root = temp_root.join("store.tbvol");
        fs::create_dir_all(&temp_root)?;

        let result = (|| {
            ingest_volume(source_path, &store_root, Default::default()).map_err(|error| {
                LasError::Storage(format!(
                    "failed to ingest seismic volume `{}`: {error}",
                    source_path.display()
                ))
            })?;
            if let Some(coordinate_reference) = coordinate_reference {
                set_any_store_native_coordinate_reference(
                    &store_root,
                    coordinate_reference.id.as_deref(),
                    coordinate_reference.name.as_deref(),
                )
                .map_err(|error| {
                    LasError::Storage(format!(
                        "failed to apply coordinate reference to ingested seismic store `{}`: {error}",
                        source_path.display()
                    ))
                })?;
            }
            self.import_seismic_store_with_kind(
                &store_root,
                source_path,
                binding,
                AssetKind::SeismicTraceData,
                SeismicAssetFamily::Volume,
                collection_name,
            )
        })();

        let _ = fs::remove_dir_all(&temp_root);
        result
    }

    fn import_seismic_store_with_kind(
        &mut self,
        store_root: impl AsRef<Path>,
        source_path: impl AsRef<Path>,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        family: SeismicAssetFamily,
        collection_name: Option<&str>,
    ) -> Result<SeismicAssetImportResult> {
        let store_root = store_root.as_ref();
        let source_path = source_path.as_ref();
        let descriptor = describe_store(store_root).map_err(|error| {
            LasError::Storage(format!("failed to describe seismic store: {error}"))
        })?;
        let handle = open_store(store_root)
            .map_err(|error| LasError::Storage(format!("failed to open seismic store: {error}")))?;
        let metadata = SeismicAssetMetadata {
            family,
            trace_data_descriptor: SeismicTraceDataDescriptor::from(&descriptor),
            descriptor,
            store: handle.manifest,
        };

        self.import_seismic_asset(
            store_root,
            source_path,
            binding,
            asset_kind,
            collection_name,
            &metadata,
        )
    }

    pub fn read_trajectory_rows(
        &self,
        asset_id: &AssetId,
        range: Option<&DepthRangeQuery>,
    ) -> Result<Vec<TrajectoryRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Trajectory)?;
        read_trajectory_rows(Path::new(&asset.package_path), range)
    }

    pub fn resolve_wellbore_trajectory(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<ResolvedTrajectoryGeometry> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        let anchor = wellbore
            .geometry
            .as_ref()
            .and_then(|geometry| geometry.anchor.as_ref());
        let anchor_fingerprint = anchor
            .map(serde_json::to_vec)
            .transpose()?
            .map(|bytes| stable_project_blob_hash("wellbore-anchor", &bytes));
        let mut coordinate_reference = anchor.and_then(|item| item.coordinate_reference.clone());
        let mut notes = Vec::new();
        let mut source_asset_ids = Vec::new();
        let mut stations = Vec::new();
        let mut assumed_metric_depth_units = false;
        let mut assumed_metric_coordinate_units = false;
        let mut multiple_assets_note = false;
        let trajectory_assets =
            if let Some(asset_id) = wellbore.definitive_trajectory_asset_id.as_ref() {
                let asset = self.asset_by_id(asset_id)?;
                require_asset_kind(&asset, AssetKind::Trajectory)?;
                if asset.wellbore_id != *wellbore_id {
                    return Err(LasError::Validation(format!(
                        "definitive trajectory asset '{}' does not belong to wellbore '{}'",
                        asset_id.0, wellbore_id.0
                    )));
                }
                notes.push(format!(
                    "using definitive trajectory asset '{}'",
                    asset.id.0
                ));
                vec![asset]
            } else {
                self.asset_summaries(wellbore_id, Some(AssetKind::Trajectory))?
                    .into_iter()
                    .filter(|summary| summary.is_current)
                    .map(|summary| summary.asset)
                    .collect::<Vec<_>>()
            };

        if trajectory_assets.is_empty() {
            notes.push(String::from(
                "no current trajectory assets are available for this wellbore",
            ));
        }

        for asset in trajectory_assets {
            if !multiple_assets_note && !source_asset_ids.is_empty() {
                notes.push(String::from(
                    "multiple current trajectory assets were merged in measured-depth order",
                ));
                multiple_assets_note = true;
            }
            let rows = self.read_trajectory_rows(&asset.id, None)?;
            if rows.is_empty() {
                notes.push(format!(
                    "trajectory asset '{}' has no trajectory rows",
                    asset.id.0
                ));
                continue;
            }

            let asset_coordinate_reference = coordinate_reference_descriptor_from_project(
                asset
                    .manifest
                    .reference_metadata
                    .coordinate_reference
                    .as_ref(),
                asset
                    .manifest
                    .reference_metadata
                    .unit_system
                    .coordinate_unit
                    .as_deref(),
            );
            validate_metric_length_unit(
                asset
                    .manifest
                    .reference_metadata
                    .unit_system
                    .depth_unit
                    .as_deref(),
                "depth",
                &asset.id,
            )?;
            validate_metric_length_unit(
                asset
                    .manifest
                    .reference_metadata
                    .unit_system
                    .coordinate_unit
                    .as_deref(),
                "coordinate",
                &asset.id,
            )?;

            if asset
                .manifest
                .reference_metadata
                .unit_system
                .depth_unit
                .is_none()
                && !assumed_metric_depth_units
            {
                notes.push(format!(
                    "trajectory asset '{}' does not store a depth unit; resolved depth fields assume the source values are already meters",
                    asset.id.0
                ));
                assumed_metric_depth_units = true;
            }
            if asset
                .manifest
                .reference_metadata
                .unit_system
                .coordinate_unit
                .is_none()
                && !assumed_metric_coordinate_units
            {
                notes.push(format!(
                    "trajectory asset '{}' does not store a coordinate unit; resolved offset fields assume the source values are already meters",
                    asset.id.0
                ));
                assumed_metric_coordinate_units = true;
            }

            if coordinate_reference.is_none() {
                coordinate_reference = asset_coordinate_reference.clone();
            } else if let (Some(existing), Some(candidate)) = (
                coordinate_reference.as_ref(),
                asset_coordinate_reference.as_ref(),
            ) {
                if !coordinate_reference_descriptors_compatible(existing, candidate) {
                    notes.push(format!(
                        "trajectory asset '{}' uses a different coordinate reference than the current resolved trajectory geometry; absolute XY was left unresolved",
                        asset.id.0
                    ));
                    coordinate_reference = None;
                }
            }

            let can_resolve_absolute_xy = can_resolve_absolute_xy(
                anchor,
                asset_coordinate_reference.as_ref(),
                &asset.id,
                &mut notes,
            );

            source_asset_ids.push(asset.id.0.clone());
            for mut station in resolve_trajectory_rows(&rows, &asset.id, &mut notes) {
                let absolute_xy = if can_resolve_absolute_xy {
                    match (anchor, station.northing_offset_m, station.easting_offset_m) {
                        (Some(anchor_reference), Some(northing), Some(easting)) => {
                            Some(ProjectedPoint2 {
                                x: anchor_reference.location.x + easting,
                                y: anchor_reference.location.y + northing,
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                station.absolute_xy = absolute_xy;
                stations.push(station);
            }
        }

        if anchor.is_none() {
            notes.push(String::from(
                "wellbore geometry has no anchor, so absolute XY could not be resolved from relative offsets",
            ));
        }

        stations.sort_by(|left, right| {
            left.measured_depth_m
                .partial_cmp(&right.measured_depth_m)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let geometry_id = stable_project_blob_hash(
            "resolved-trajectory",
            format!(
                "{}|{:?}|{}",
                wellbore.id.0,
                anchor_fingerprint,
                source_asset_ids.join("|")
            )
            .as_bytes(),
        );

        Ok(ResolvedTrajectoryGeometry {
            id: geometry_id,
            wellbore_id: wellbore.id.0,
            source_asset_ids,
            coordinate_reference,
            anchor_fingerprint,
            stations,
            notes,
        })
    }

    pub fn resolve_section_well_overlays(
        &self,
        request: &SectionWellOverlayRequestDto,
    ) -> Result<ResolveSectionWellOverlaysResponse> {
        if request.survey_asset_id.trim().is_empty() {
            return Err(LasError::Validation(
                "section well overlay request requires a survey asset id".to_string(),
            ));
        }
        if request.wellbore_ids.is_empty() {
            return Err(LasError::Validation(
                "section well overlay request requires at least one wellbore id".to_string(),
            ));
        }

        let survey =
            self.resolve_survey_map_survey(&AssetId(request.survey_asset_id.clone()), None)?;
        let grid_transform = preferred_section_grid_transform(&survey).ok_or_else(|| {
            LasError::Validation(format!(
                "survey '{}' has no mappable grid transform for section overlays",
                survey.name
            ))
        })?;
        let section_axis = section_axis_spec(&survey.index_grid, request.axis, request.index)?;
        let section_tolerance_m = request
            .tolerance_m
            .unwrap_or_else(|| default_section_tolerance_m(grid_transform, request.axis));

        let overlays = request
            .wellbore_ids
            .iter()
            .map(|wellbore_id| {
                self.resolve_single_section_well_overlay(
                    &WellboreId(wellbore_id.clone()),
                    &survey,
                    grid_transform,
                    &section_axis,
                    section_tolerance_m,
                    request,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ResolveSectionWellOverlaysResponse {
            schema_version: SECTION_WELL_OVERLAY_CONTRACT_VERSION,
            overlays,
        })
    }

    pub fn read_tops(&self, asset_id: &AssetId) -> Result<Vec<TopRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::TopSet)?;
        read_tops_rows(Path::new(&asset.package_path))
    }

    pub fn read_well_marker_rows(&self, asset_id: &AssetId) -> Result<Vec<WellMarkerRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellMarkerSet)?;
        read_well_marker_rows(Path::new(&asset.package_path))
    }

    pub fn read_well_marker_horizon_residual_rows(
        &self,
        asset_id: &AssetId,
    ) -> Result<Vec<WellMarkerHorizonResidualRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellMarkerHorizonResidualSet)?;
        read_well_marker_horizon_residual_rows(Path::new(&asset.package_path))
    }

    pub fn read_well_marker_horizon_residual_points(
        &self,
        asset_id: &AssetId,
    ) -> Result<Vec<WellMarkerHorizonResidualPointRecord>> {
        Ok(self
            .read_well_marker_horizon_residual_rows(asset_id)?
            .into_iter()
            .filter_map(well_marker_horizon_residual_point_from_row)
            .collect())
    }

    pub fn resolve_well_marker_horizon_residual_source(
        &self,
        request: &WellMarkerHorizonResidualRequestDto,
    ) -> Result<ResolvedWellMarkerHorizonResidualSourceDto> {
        self.resolve_well_marker_horizon_residual_rows_from_request(
            &WellMarkerHorizonResidualResolveRequest {
                source_asset_id: AssetId(request.source_asset_id.clone()),
                survey_asset_id: AssetId(request.survey_asset_id.clone()),
                horizon_id: request.horizon_id.clone(),
                marker_name: request.marker_name.clone(),
            },
        )
    }

    fn resolve_well_marker_horizon_residual_rows_from_request(
        &self,
        request: &WellMarkerHorizonResidualResolveRequest,
    ) -> Result<ResolvedWellMarkerHorizonResidualSourceDto> {
        let source_asset = self.asset_by_id(&request.source_asset_id)?;
        let source_markers = residual_marker_inputs_for_asset(self, &source_asset)?;
        let selected_marker_name = request
            .marker_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let source_markers = if let Some(selected_marker_name) = selected_marker_name.as_deref() {
            let filtered = source_markers
                .into_iter()
                .filter(|marker| {
                    marker
                        .marker_name
                        .eq_ignore_ascii_case(selected_marker_name)
                })
                .collect::<Vec<_>>();
            if filtered.is_empty() {
                return Err(LasError::Validation(format!(
                    "marker '{}' was not found on source asset '{}'",
                    selected_marker_name, source_asset.id.0
                )));
            }
            filtered
        } else {
            source_markers
        };
        let survey_asset = self.asset_by_id(&request.survey_asset_id)?;
        require_asset_kind(&survey_asset, AssetKind::SeismicTraceData)?;
        let well = self.well_by_id(&source_asset.well_id)?;
        let wellbore = self.wellbore_by_id(&source_asset.wellbore_id)?;
        let resolved_trajectory = self.resolve_wellbore_trajectory(&source_asset.wellbore_id)?;
        let survey_store_root = Path::new(&survey_asset.package_path).join("store");
        let metadata = read_seismic_asset_metadata(Path::new(&survey_asset.package_path))?;
        let survey_coordinate_reference = metadata
            .descriptor
            .coordinate_reference_binding
            .as_ref()
            .and_then(|binding| binding.effective.as_ref())
            .cloned()
            .or_else(|| {
                metadata
                    .descriptor
                    .spatial
                    .as_ref()
                    .and_then(|spatial| spatial.coordinate_reference.clone())
            });
        if let (Some(trajectory_coordinate_reference), Some(survey_coordinate_reference)) = (
            resolved_trajectory.coordinate_reference.as_ref(),
            survey_coordinate_reference.as_ref(),
        ) {
            if !coordinate_reference_descriptors_compatible(
                trajectory_coordinate_reference,
                survey_coordinate_reference,
            ) {
                return Err(LasError::Validation(format!(
                    "wellbore '{}' trajectory coordinates are incompatible with survey '{}'",
                    wellbore.id.0, survey_asset.id.0
                )));
            }
        }
        let handle = open_store(&survey_store_root).map_err(|error| {
            LasError::Storage(format!(
                "failed to open seismic survey store '{}': {error}",
                survey_store_root.display()
            ))
        })?;
        let grid_transform = handle
            .manifest
            .volume
            .spatial
            .as_ref()
            .and_then(|spatial| spatial.grid_transform.as_ref())
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "survey '{}' has no resolved grid transform for horizon residual sampling",
                    survey_asset.id.0
                ))
            })?;
        let grid_transform = SurveyMapGridTransformDto {
            origin: projected_point_dto_from_seismic(&grid_transform.origin),
            inline_basis: ProjectedVector2Dto {
                x: grid_transform.inline_basis.x,
                y: grid_transform.inline_basis.y,
            },
            xline_basis: ProjectedVector2Dto {
                x: grid_transform.xline_basis.x,
                y: grid_transform.xline_basis.y,
            },
        };
        let horizons = load_horizon_grids(&survey_store_root).map_err(|error| {
            LasError::Storage(format!(
                "failed to load horizons from survey '{}': {error}",
                survey_asset.id.0
            ))
        })?;
        let horizon = horizons
            .iter()
            .find(|candidate| candidate.descriptor.id == request.horizon_id)
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "survey '{}' does not contain horizon '{}'",
                    survey_asset.id.0, request.horizon_id
                ))
            })?;
        if horizon.descriptor.vertical_domain != ophiolite_seismic::TimeDepthDomain::Depth {
            let vertical_domain = match horizon.descriptor.vertical_domain {
                ophiolite_seismic::TimeDepthDomain::Time => "time",
                ophiolite_seismic::TimeDepthDomain::Depth => "depth",
            };
            return Err(LasError::Validation(format!(
                "horizon '{}' is '{}' domain; depth-horizon residuals require a depth-domain horizon",
                request.horizon_id, vertical_domain
            )));
        }

        let mut diagnostics = Vec::new();
        diagnostics.extend(resolved_trajectory.notes.iter().cloned());
        diagnostics.push(format!(
            "residual sign convention is '{}' and sampling method is '{}'",
            WELL_MARKER_HORIZON_RESIDUAL_SIGN_CONVENTION,
            WELL_MARKER_HORIZON_RESIDUAL_SAMPLING_METHOD
        ));
        if let Some(selected_marker_name) = selected_marker_name {
            diagnostics.push(format!(
                "filtered source markers to canonical marker '{}'",
                selected_marker_name
            ));
        }
        let rows = source_markers
            .iter()
            .map(|marker| {
                resolve_single_well_marker_horizon_residual(
                    marker,
                    &resolved_trajectory,
                    horizon,
                    &grid_transform,
                )
            })
            .collect::<Vec<_>>();

        Ok(ResolvedWellMarkerHorizonResidualSourceDto {
            schema_version: WELL_MARKER_HORIZON_RESIDUAL_CONTRACT_VERSION,
            source_asset_id: source_asset.id.0.clone(),
            source_asset_kind: source_asset.asset_kind.as_str().to_string(),
            survey_asset_id: survey_asset.id.0.clone(),
            horizon_id: horizon.descriptor.id.clone(),
            horizon_name: horizon.descriptor.name.clone(),
            well_id: well.id.0.clone(),
            well_name: well.name,
            wellbore_id: wellbore.id.0.clone(),
            wellbore_name: wellbore.name,
            residual_sign_convention: WELL_MARKER_HORIZON_RESIDUAL_SIGN_CONVENTION.to_string(),
            sampling_method: WELL_MARKER_HORIZON_RESIDUAL_SAMPLING_METHOD.to_string(),
            depth_reference_used: WELL_MARKER_HORIZON_RESIDUAL_DEPTH_REFERENCE.to_string(),
            trajectory_asset_ids: resolved_trajectory.source_asset_ids,
            rows: rows
                .into_iter()
                .map(well_marker_horizon_residual_row_dto)
                .collect(),
            diagnostics,
        })
    }

    pub fn read_pressure_observations(
        &self,
        asset_id: &AssetId,
        range: Option<&DepthRangeQuery>,
    ) -> Result<Vec<PressureObservationRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::PressureObservation)?;
        read_pressure_rows(Path::new(&asset.package_path), range)
    }

    pub fn read_drilling_observations(
        &self,
        asset_id: &AssetId,
        range: Option<&DepthRangeQuery>,
    ) -> Result<Vec<DrillingObservationRow>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::DrillingObservation)?;
        read_drilling_rows(Path::new(&asset.package_path), range)
    }

    pub fn read_well_time_depth_model(&self, asset_id: &AssetId) -> Result<WellTimeDepthModel1D> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellTimeDepthModel)?;
        read_well_time_depth_model_package(Path::new(&asset.package_path))
    }

    pub fn read_checkshot_vsp_observation_set(
        &self,
        asset_id: &AssetId,
    ) -> Result<CheckshotVspObservationSet1D> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::CheckshotVspObservationSet)?;
        read_checkshot_vsp_observation_set_package(Path::new(&asset.package_path))
    }

    pub fn read_manual_time_depth_pick_set(
        &self,
        asset_id: &AssetId,
    ) -> Result<ManualTimeDepthPickSet1D> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::ManualTimeDepthPickSet)?;
        read_manual_time_depth_pick_set_package(Path::new(&asset.package_path))
    }

    pub fn read_well_tie_observation_set(
        &self,
        asset_id: &AssetId,
    ) -> Result<WellTieObservationSet1D> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellTieObservationSet)?;
        read_well_tie_observation_set_package(Path::new(&asset.package_path))
    }

    pub fn read_well_time_depth_authored_model(
        &self,
        asset_id: &AssetId,
    ) -> Result<WellTimeDepthAuthoredModel1D> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellTimeDepthAuthoredModel)?;
        read_well_time_depth_authored_model_package(Path::new(&asset.package_path))
    }

    pub fn import_checkshot_vsp_observation_set_json(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let observation_set: CheckshotVspObservationSet1D =
            serde_json::from_slice(&fs::read(source_path)?).map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse checkshot/VSP observation json '{}': {error}",
                    source_path.display()
                ))
            })?;
        validate_checkshot_vsp_observation_set(&observation_set)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::CheckshotVspObservationSet,
            observation_set.name.clone(),
            &observation_set,
            write_checkshot_vsp_observation_set_package,
        )
    }

    pub fn create_checkshot_vsp_observation_set(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
        observation_set: &CheckshotVspObservationSet1D,
    ) -> Result<ProjectAssetImportResult> {
        validate_checkshot_vsp_observation_set(observation_set)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::CheckshotVspObservationSet,
            observation_set.name.clone(),
            observation_set,
            write_checkshot_vsp_observation_set_package,
        )
    }

    pub fn import_manual_time_depth_pick_set_json(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let pick_set: ManualTimeDepthPickSet1D = serde_json::from_slice(&fs::read(source_path)?)
            .map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse manual time-depth pick json '{}': {error}",
                    source_path.display()
                ))
            })?;
        validate_manual_time_depth_pick_set(&pick_set)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::ManualTimeDepthPickSet,
            pick_set.name.clone(),
            &pick_set,
            write_manual_time_depth_pick_set_package,
        )
    }

    pub fn create_manual_time_depth_pick_set(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
        pick_set: &ManualTimeDepthPickSet1D,
    ) -> Result<ProjectAssetImportResult> {
        validate_manual_time_depth_pick_set(pick_set)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::ManualTimeDepthPickSet,
            pick_set.name.clone(),
            pick_set,
            write_manual_time_depth_pick_set_package,
        )
    }

    pub fn create_well_tie_observation_set(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
        observation_set: &WellTieObservationSet1D,
    ) -> Result<ProjectAssetImportResult> {
        validate_well_tie_observation_set(observation_set)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::WellTieObservationSet,
            observation_set.name.clone(),
            observation_set,
            write_well_tie_observation_set_package,
        )
    }

    pub fn create_well_time_depth_authored_model(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
        authored_model: &WellTimeDepthAuthoredModel1D,
    ) -> Result<ProjectAssetImportResult> {
        validate_well_time_depth_authored_model(authored_model)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::WellTimeDepthAuthoredModel,
            authored_model.name.clone(),
            authored_model,
            write_well_time_depth_authored_model_package,
        )
    }

    pub fn create_well_time_depth_model(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
        model: &WellTimeDepthModel1D,
    ) -> Result<ProjectAssetImportResult> {
        validate_well_time_depth_model(model)?;
        let identifiers = identifiers_from_binding(&binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, &binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| Some(model.name.clone()))
            .unwrap_or_else(|| AssetKind::WellTimeDepthModel.as_str().to_string());
        let collection = self.resolve_or_create_collection(
            &wellbore.id,
            AssetKind::WellTimeDepthModel,
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::WellTimeDepthModel.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        write_well_time_depth_model_package(&staged.root, model)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = well_time_depth_model_manifest(
            source_path,
            model,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            identifiers_from_binding(&binding),
            supersedes.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id.clone(),
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind: AssetKind::WellTimeDepthModel,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::WellTimeDepthModel(DirectoryAssetDiffSummary::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;

        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    pub fn import_well_time_depth_authored_model_json(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let authored_model: WellTimeDepthAuthoredModel1D =
            serde_json::from_slice(&fs::read(source_path)?).map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse well time-depth authored model json '{}': {error}",
                    source_path.display()
                ))
            })?;
        validate_well_time_depth_authored_model(&authored_model)?;
        self.import_well_time_depth_json_asset(
            source_path,
            binding,
            collection_name,
            AssetKind::WellTimeDepthAuthoredModel,
            authored_model.name.clone(),
            &authored_model,
            write_well_time_depth_authored_model_package,
        )
    }

    pub fn import_well_time_depth_model_json(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
    ) -> Result<ProjectAssetImportResult> {
        let model: WellTimeDepthModel1D =
            serde_json::from_slice(&fs::read(source_path)?).map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse well time-depth model json '{}': {error}",
                    source_path.display()
                ))
            })?;
        validate_well_time_depth_model(&model)?;

        let identifiers = identifiers_from_binding(&binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, &binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| Some(model.name.clone()))
            .unwrap_or_else(|| AssetKind::WellTimeDepthModel.as_str().to_string());
        let collection = self.resolve_or_create_collection(
            &wellbore.id,
            AssetKind::WellTimeDepthModel,
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::WellTimeDepthModel.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        write_well_time_depth_model_package(&staged.root, &model)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = well_time_depth_model_manifest(
            source_path,
            &model,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            identifiers_from_binding(&binding),
            supersedes.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id.clone(),
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind: AssetKind::WellTimeDepthModel,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::WellTimeDepthModel(DirectoryAssetDiffSummary::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;

        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    fn import_well_time_depth_json_asset<T, F>(
        &mut self,
        source_path: &Path,
        binding: AssetBindingInput,
        collection_name: Option<&str>,
        asset_kind: AssetKind,
        default_name: String,
        asset_payload: &T,
        writer: F,
    ) -> Result<ProjectAssetImportResult>
    where
        T: Serialize,
        F: Fn(&Path, &T) -> Result<()>,
    {
        let identifiers = identifiers_from_binding(&binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, &binding)?;
        let collection_name = collection_name.map(str::to_owned).unwrap_or(default_name);
        let collection =
            self.resolve_or_create_collection(&wellbore.id, asset_kind.clone(), &collection_name)?;
        if matches!(asset_kind, AssetKind::SeismicTraceData) {
            self.bind_collection_owner(
                &collection.id,
                &AssetOwnerBinding {
                    scope: AssetOwnerScope::Survey,
                    owner_id: collection.logical_asset_id.0.clone(),
                    display_name: collection.name.clone(),
                },
            )?;
        }
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        writer(&staged.root, asset_payload)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = well_time_depth_json_manifest(
            source_path,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            asset_kind.clone(),
            identifiers_from_binding(&binding),
            supersedes.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id.clone(),
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind: asset_kind.clone(),
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        let refreshed_wellbore = self.wellbore_by_id(&wellbore.id)?;
        if matches!(asset.asset_kind, AssetKind::Trajectory)
            && refreshed_wellbore.definitive_trajectory_asset_id.is_none()
        {
            self.set_definitive_trajectory_asset(&wellbore.id, Some(&asset.id))?;
        }
        if matches!(asset.asset_kind, AssetKind::TopSet)
            && refreshed_wellbore.definitive_top_set_asset_id.is_none()
        {
            self.set_definitive_top_set_asset(&wellbore.id, Some(&asset.id))?;
        }
        if matches!(asset.asset_kind, AssetKind::WellMarkerSet)
            && refreshed_wellbore.definitive_marker_set_asset_id.is_none()
        {
            self.set_definitive_marker_set_asset(&wellbore.id, Some(&asset.id))?;
        }
        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    pub fn compile_well_time_depth_authored_model_to_asset(
        &mut self,
        authored_model_asset_id: &AssetId,
        output_collection_name: Option<&str>,
        set_active: bool,
    ) -> Result<ProjectAssetImportResult> {
        let authored_asset = self.asset_by_id(authored_model_asset_id)?;
        require_asset_kind(&authored_asset, AssetKind::WellTimeDepthAuthoredModel)?;
        let authored_model = self.read_well_time_depth_authored_model(authored_model_asset_id)?;
        let resolved_trajectory =
            self.resolve_wellbore_trajectory(&WellboreId(authored_model.wellbore_id.clone()))?;
        if resolved_trajectory.id != authored_model.resolved_trajectory_fingerprint {
            return Err(LasError::Validation(format!(
                "authored well time-depth model '{}' targets resolved trajectory '{}' but the current wellbore trajectory fingerprint is '{}'",
                authored_model_asset_id.0,
                authored_model.resolved_trajectory_fingerprint,
                resolved_trajectory.id
            )));
        }
        let compiled_model =
            compile_well_time_depth_authored_model(&authored_model, &resolved_trajectory, self)?;
        let collection_name = output_collection_name
            .map(str::to_owned)
            .unwrap_or_else(|| authored_model.name.clone());
        let collection = self.resolve_or_create_collection(
            &authored_asset.wellbore_id,
            AssetKind::WellTimeDepthModel,
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::WellTimeDepthModel.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        write_well_time_depth_model_package(&staged.root, &compiled_model)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let source_path =
            Path::new(&authored_asset.package_path).join(WELL_TIME_DEPTH_AUTHORED_MODEL_FILENAME);
        let mut manifest = well_time_depth_model_manifest(
            &source_path,
            &compiled_model,
            &authored_asset.well_id,
            &authored_asset.wellbore_id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            authored_asset
                .manifest
                .reference_metadata
                .identifiers
                .clone(),
            supersedes.clone(),
        )?;
        manifest.derived_from = Some(authored_asset.logical_asset_id.clone());
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id.clone(),
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: authored_asset.well_id.clone(),
            wellbore_id: authored_asset.wellbore_id.clone(),
            asset_kind: AssetKind::WellTimeDepthModel,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::WellTimeDepthModel(DirectoryAssetDiffSummary::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        if set_active {
            self.set_active_well_time_depth_model(
                &authored_asset.wellbore_id,
                Some(&storage_asset_id),
            )?;
        }
        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: authored_asset.well_id,
                wellbore_id: authored_asset.wellbore_id,
                created_well: false,
                created_wellbore: false,
            },
            collection,
            asset,
        })
    }

    pub fn analyze_well_tie_from_model(
        &self,
        source_model_asset_id: &AssetId,
        tie_name: &str,
        tie_start_ms: f64,
        tie_end_ms: f64,
        search_radius_m: f64,
    ) -> Result<WellTieAnalysis1D> {
        if !tie_start_ms.is_finite() || !tie_end_ms.is_finite() || tie_end_ms <= tie_start_ms {
            return Err(LasError::Validation(
                "well tie analysis requires a finite tie window with end > start".to_string(),
            ));
        }

        let source_asset = self.asset_by_id(source_model_asset_id)?;
        require_asset_kind(&source_asset, AssetKind::WellTimeDepthModel)?;
        let source_model = self.read_well_time_depth_model(source_model_asset_id)?;
        let log_selection = select_well_tie_log_selection(self, &source_asset.wellbore_id)?;
        let time_window_samples = source_model
            .samples
            .iter()
            .filter(|sample| {
                let time_ms = f64::from(sample.time_ms);
                time_ms >= tie_start_ms && time_ms <= tie_end_ms
            })
            .cloned()
            .collect::<Vec<_>>();
        if time_window_samples.len() < 2 {
            return Err(LasError::Validation(format!(
                "selected well time-depth model '{}' does not provide enough samples inside the requested tie window",
                source_model_asset_id.0
            )));
        }

        let sample_step_ms = infer_mean_time_step_ms(&time_window_samples)
            .unwrap_or(4.0)
            .clamp(1.0, 8.0);
        let times_ms = time_window_samples
            .iter()
            .map(|sample| sample.time_ms)
            .collect::<Vec<_>>();
        let depths_m = time_window_samples
            .iter()
            .map(|sample| f64::from(sample.depth))
            .collect::<Vec<_>>();

        let density_curve = prepare_interpolated_log_curve(&log_selection.density_curve.curve)?;
        let velocity_curve = prepare_interpolated_log_curve(&log_selection.velocity_curve.curve)?;
        let density_values = depths_m
            .iter()
            .map(|depth_m| {
                interpolate_prepared_curve(&density_curve, *depth_m).map(|value| value as f32)
            })
            .collect::<Vec<_>>();
        let velocity_values = depths_m
            .iter()
            .map(|depth_m| {
                interpolate_prepared_curve(&velocity_curve, *depth_m).and_then(|value| {
                    velocity_value_for_well_tie(value, log_selection.velocity_source_kind)
                })
            })
            .collect::<Vec<_>>();
        let density_values = fill_missing_f32_series(&density_values, "density curve")?;
        let velocity_values = fill_missing_f32_series(&velocity_values, "velocity curve")?;
        let acoustic_impedance_values = density_values
            .iter()
            .zip(velocity_values.iter())
            .map(|(density, velocity)| density * velocity)
            .collect::<Vec<_>>();
        let reflectivity_values = acoustic_impedance_to_reflectivity(&acoustic_impedance_values);
        let wavelet = build_provisional_well_tie_wavelet(sample_step_ms, 28.0);
        let synthetic_values = convolve_same(&reflectivity_values, &wavelet.amplitudes);

        let midpoint_ms = (tie_start_ms + tie_end_ms) * 0.5;
        let interval_ms = tie_end_ms - tie_start_ms;
        let bounded_search_radius_m = if search_radius_m.is_finite() {
            search_radius_m.max(0.0)
        } else {
            0.0
        };
        let stretch_factor = 1.0 + (interval_ms / 6000.0).clamp(0.01, 0.05);
        let provisional_shift_samples =
            -(1_i32 + (bounded_search_radius_m / 75.0).round() as i32).clamp(1, 5);
        let well_shift_samples = (provisional_shift_samples / 2).min(0);
        let bulk_shift_ms = provisional_shift_samples as f64 * f64::from(sample_step_ms);
        let trace_search_offset_m = if bounded_search_radius_m > 0.0 {
            (bounded_search_radius_m * 0.125).min(bounded_search_radius_m)
        } else {
            0.0
        };
        let best_match_values =
            create_matched_trace(&synthetic_values, provisional_shift_samples, 0.94, 0.035);
        let well_trace_values =
            create_matched_trace(&synthetic_values, well_shift_samples, 0.86, 0.055);
        let correlation = trace_correlation(&synthetic_values, &best_match_values)
            .unwrap_or(0.82)
            .clamp(0.0, 0.999);

        let samples = time_window_samples
            .iter()
            .map(|sample| {
                let source_time_ms = f64::from(sample.time_ms);
                let adjusted_time_ms =
                    ((source_time_ms - midpoint_ms) * stretch_factor) + midpoint_ms + bulk_shift_ms;
                ophiolite_seismic::WellTimeDepthObservationSample {
                    depth_m: f64::from(sample.depth),
                    time_ms: adjusted_time_ms,
                    quality: Some(correlation),
                    station_id: None,
                    note: Some(format!(
                        "Derived from log-based synthetic preview using source model '{}'.",
                        source_model_asset_id.0,
                    )),
                }
            })
            .collect::<Vec<_>>();

        let cleaned_name = tie_name.trim();
        let name = if cleaned_name.is_empty() {
            format!("{} Well Tie", source_model.name)
        } else {
            cleaned_name.to_string()
        };
        let observation_set = WellTieObservationSet1D {
            id: unique_id("well_tie"),
            name,
            wellbore_id: source_model.wellbore_id.clone(),
            depth_reference: source_model.depth_reference,
            travel_time_reference: source_model.travel_time_reference,
            samples,
            source_well_time_depth_model_asset_id: Some(source_model_asset_id.0.clone()),
            tie_window_start_ms: Some(tie_start_ms),
            tie_window_end_ms: Some(tie_end_ms),
            trace_search_radius_m: Some(bounded_search_radius_m as f32),
            bulk_shift_ms: Some(bulk_shift_ms as f32),
            stretch_factor: Some(stretch_factor as f32),
            trace_search_offset_m: Some(trace_search_offset_m as f32),
            correlation: Some(correlation as f32),
            notes: vec![
                format!(
                    "Derived from source well time-depth model '{}' and density/velocity logs.",
                    source_model_asset_id.0
                ),
                format!(
                    "Accepted tie window {:.0}-{:.0} ms with +/-{:.0} m local search radius.",
                    tie_start_ms, tie_end_ms, bounded_search_radius_m
                ),
                format!(
                    "Preview uses density '{}' and velocity '{}'.",
                    log_selection.density_curve.curve.curve_name,
                    log_selection.velocity_curve.curve.curve_name
                ),
            ],
        };
        validate_well_tie_observation_set(&observation_set)?;

        let section_window = build_well_tie_section_window(
            &times_ms,
            &synthetic_values,
            bounded_search_radius_m as f32,
        );
        Ok(WellTieAnalysis1D {
            draft_observation_set: observation_set,
            log_selection: WellTieLogSelection1D {
                density_curve: build_well_tie_log_curve_source(
                    &log_selection.density_curve,
                    density_curve.point_count,
                ),
                velocity_curve: build_well_tie_log_curve_source(
                    &log_selection.velocity_curve,
                    velocity_curve.point_count,
                ),
                velocity_source_kind: log_selection.velocity_source_kind,
            },
            acoustic_impedance_curve: WellTieCurve1D {
                id: "acoustic-impedance".to_string(),
                label: "AI".to_string(),
                unit: Some("vp*density".to_string()),
                times_ms: times_ms.clone(),
                values: acoustic_impedance_values,
            },
            reflectivity_trace: WellTieTrace1D {
                id: "reflectivity".to_string(),
                label: "Reflectivity".to_string(),
                times_ms: times_ms.clone(),
                amplitudes: reflectivity_values,
            },
            synthetic_trace: WellTieTrace1D {
                id: "synthetic".to_string(),
                label: "Syn".to_string(),
                times_ms: times_ms.clone(),
                amplitudes: synthetic_values,
            },
            best_match_trace: WellTieTrace1D {
                id: "best-seismic".to_string(),
                label: "Best Seis".to_string(),
                times_ms: times_ms.clone(),
                amplitudes: best_match_values,
            },
            well_trace: WellTieTrace1D {
                id: "well-seismic".to_string(),
                label: "Well Seis".to_string(),
                times_ms,
                amplitudes: well_trace_values,
            },
            section_window,
            wavelet,
            notes: vec![
                "Acoustic impedance, reflectivity, and synthetic traces are computed from density plus sonic/Vp logs.".to_string(),
                "Seismic traces and the local seismic window remain provisional until survey/store trace extraction is wired into the project well-tie workflow.".to_string(),
            ],
        })
    }

    pub fn draft_well_tie_observation_set_from_model(
        &self,
        source_model_asset_id: &AssetId,
        tie_name: &str,
        tie_start_ms: f64,
        tie_end_ms: f64,
        search_radius_m: f64,
    ) -> Result<WellTieObservationSet1D> {
        Ok(self
            .analyze_well_tie_from_model(
                source_model_asset_id,
                tie_name,
                tie_start_ms,
                tie_end_ms,
                search_radius_m,
            )?
            .draft_observation_set)
    }

    pub fn accept_well_tie_from_model(
        &mut self,
        source_model_asset_id: &AssetId,
        binding: AssetBindingInput,
        tie_name: &str,
        tie_start_ms: f64,
        tie_end_ms: f64,
        search_radius_m: f64,
        output_collection_name: Option<&str>,
        set_active: bool,
    ) -> Result<AcceptedWellTieResult> {
        let observation_set = self.draft_well_tie_observation_set_from_model(
            source_model_asset_id,
            tie_name,
            tie_start_ms,
            tie_end_ms,
            search_radius_m,
        )?;
        self.accept_well_tie_observation_set_from_model(
            source_model_asset_id,
            binding,
            tie_name,
            &observation_set,
            output_collection_name,
            set_active,
        )
    }

    pub fn accept_well_tie_observation_set_from_model(
        &mut self,
        source_model_asset_id: &AssetId,
        binding: AssetBindingInput,
        tie_name: &str,
        observation_set: &WellTieObservationSet1D,
        output_collection_name: Option<&str>,
        set_active: bool,
    ) -> Result<AcceptedWellTieResult> {
        let source_asset = self.asset_by_id(source_model_asset_id)?;
        require_asset_kind(&source_asset, AssetKind::WellTimeDepthModel)?;
        let source_model = self.read_well_time_depth_model(source_model_asset_id)?;
        validate_well_tie_observation_set(observation_set)?;
        let source_model_package =
            Path::new(&source_asset.package_path).join(WELL_TIME_DEPTH_MODEL_FILENAME);
        let observation_collection_name = format!("{} observations", observation_set.name.trim());
        let observation_result = self.create_well_tie_observation_set(
            &source_model_package,
            binding.clone(),
            Some(&observation_collection_name),
            observation_set,
        )?;

        let resolved_trajectory = self.resolve_wellbore_trajectory(&source_asset.wellbore_id)?;
        let valid_from_depth_m = observation_set.samples.first().map(|sample| sample.depth_m);
        let valid_to_depth_m = observation_set.samples.last().map(|sample| sample.depth_m);
        let authored_name = if tie_name.trim().is_empty() {
            format!("{} Accepted Tie", source_model.name)
        } else {
            format!("{} Accepted Tie", tie_name.trim())
        };
        let authored_model = WellTimeDepthAuthoredModel1D {
            id: unique_id("well_tie_authored"),
            name: authored_name,
            wellbore_id: source_asset.wellbore_id.0.clone(),
            resolved_trajectory_fingerprint: resolved_trajectory.id,
            depth_reference: observation_set.depth_reference,
            travel_time_reference: observation_set.travel_time_reference,
            source_bindings: vec![
                ophiolite_seismic::WellTimeDepthSourceBinding {
                    source_kind: TimeDepthTransformSourceKind::WellTieObservationSet1D,
                    asset_id: observation_result.asset.id.0.clone(),
                    enabled: true,
                    priority: 0,
                    valid_from_depth_m,
                    valid_to_depth_m,
                    notes: vec![
                        "Accepted well-tie override inside the analyzed interval.".to_string(),
                    ],
                },
                ophiolite_seismic::WellTimeDepthSourceBinding {
                    source_kind: source_model.source_kind,
                    asset_id: source_model_asset_id.0.clone(),
                    enabled: true,
                    priority: 1,
                    valid_from_depth_m: None,
                    valid_to_depth_m: None,
                    notes: vec![
                        "Fallback to the selected compiled source model outside tie coverage."
                            .to_string(),
                    ],
                },
            ],
            assumption_intervals: Vec::new(),
            sampling_step_m: Some(10.0),
            notes: vec![
                format!(
                    "Accepted from tie '{}' against compiled source model '{}'.",
                    observation_set.name, source_model_asset_id.0
                ),
                "Acceptance uses the analyzed tie-window corrections stored on the observation set."
                    .to_string(),
            ],
        };
        let observation_package = Path::new(&observation_result.asset.package_path)
            .join(WELL_TIE_OBSERVATION_SET_FILENAME);
        let authored_collection_name = format!("{} authored", observation_set.name.trim());
        let authored_result = self.create_well_time_depth_authored_model(
            &observation_package,
            binding,
            Some(&authored_collection_name),
            &authored_model,
        )?;
        let compiled_collection_name =
            output_collection_name.or(Some(observation_set.name.as_str()));
        let compiled_result = self.compile_well_time_depth_authored_model_to_asset(
            &authored_result.asset.id,
            compiled_collection_name,
            set_active,
        )?;

        Ok(AcceptedWellTieResult {
            observation_result,
            authored_result,
            compiled_result,
        })
    }

    pub fn read_log_curve_data(&self, asset_id: &AssetId) -> Result<Vec<LogCurveData>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        let package = open_package(&asset.package_path)?;
        let semantics = if asset.manifest.curve_semantics.is_empty() {
            classify_log_curves_from_file(package.file())
        } else {
            asset.manifest.curve_semantics.clone()
        };
        log_curve_data_for_compute(package.file(), &semantics)
    }

    pub fn resolve_rock_physics_crossplot_source(
        &self,
        request: &RockPhysicsCrossplotRequestDto,
    ) -> Result<ResolvedRockPhysicsCrossplotSourceDto> {
        validate_rock_physics_request(request)?;

        let wells = request
            .wellbore_ids
            .iter()
            .map(|wellbore_id| self.prepare_rock_physics_well(&WellboreId(wellbore_id.clone())))
            .collect::<Result<Vec<_>>>()?;

        if wells.is_empty() {
            return Err(LasError::Validation(
                "rock-physics crossplot request requires at least one wellbore id".to_string(),
            ));
        }

        let mut samples = Vec::new();
        let mut source_bindings = Vec::new();

        for prepared in &wells {
            let x_binding = resolve_rock_physics_semantic_binding(
                prepared,
                request.x_semantic,
                prepared.well.name.as_str(),
            )?;
            let y_binding = resolve_rock_physics_semantic_binding(
                prepared,
                request.y_semantic,
                prepared.well.name.as_str(),
            )?;
            let color_binding = match &request.color_binding {
                RockPhysicsColorRequestDto::Continuous(binding) => {
                    Some(resolve_rock_physics_semantic_binding(
                        prepared,
                        binding.semantic,
                        prepared.well.name.as_str(),
                    )?)
                }
                RockPhysicsColorRequestDto::Categorical(binding) => {
                    if binding.semantic == RockPhysicsCategoricalSemanticDto::Facies {
                        return Err(LasError::Validation(
                            "facies categorical coloring is not materializable from canonical well-log semantics yet".to_string(),
                        ));
                    }
                    None
                }
            };

            let anchor = primary_rock_physics_anchor(
                prepared,
                request.x_semantic,
                request.y_semantic,
                match &request.color_binding {
                    RockPhysicsColorRequestDto::Continuous(binding) => Some(binding.semantic),
                    RockPhysicsColorRequestDto::Categorical(_) => None,
                },
            )
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "well '{}' does not provide a usable anchor curve for rock-physics sampling",
                    prepared.well.name
                ))
            })?;

            let source_binding_id = format!("rock-physics:{}:binding", prepared.well.wellbore_id);
            let derived_channels = collect_derived_channels([
                Some(&x_binding),
                Some(&y_binding),
                color_binding.as_ref(),
            ]);
            source_bindings.push(RockPhysicsSourceBindingDto {
                id: source_binding_id.clone(),
                well_id: prepared.well.well_id.clone(),
                wellbore_id: prepared.well.wellbore_id.clone(),
                x_curve_id: x_binding.curve_id.clone(),
                y_curve_id: y_binding.curve_id.clone(),
                color_curve_id: color_binding
                    .as_ref()
                    .map(|binding| binding.curve_id.clone()),
                derived_channels: (!derived_channels.is_empty()).then_some(derived_channels),
            });

            for depth_m in &anchor.prepared.depths_m {
                if !depth_in_range(*depth_m, request.depth_min, request.depth_max) {
                    continue;
                }

                let x_value = rock_physics_value_at_depth(prepared, request.x_semantic, *depth_m);
                let y_value = rock_physics_value_at_depth(prepared, request.y_semantic, *depth_m);
                let color_value = match &request.color_binding {
                    RockPhysicsColorRequestDto::Continuous(binding) => {
                        rock_physics_value_at_depth(prepared, binding.semantic, *depth_m)
                    }
                    RockPhysicsColorRequestDto::Categorical(_) => None,
                };
                let (color_category_id, symbol_category_id) = match &request.color_binding {
                    RockPhysicsColorRequestDto::Categorical(binding) => match binding.semantic {
                        RockPhysicsCategoricalSemanticDto::Well => (Some(0), None),
                        RockPhysicsCategoricalSemanticDto::Wellbore => (Some(0), None),
                        RockPhysicsCategoricalSemanticDto::Facies => (None, None),
                    },
                    RockPhysicsColorRequestDto::Continuous(_) => (None, None),
                };

                let (Some(x_value), Some(y_value)) = (x_value, y_value) else {
                    continue;
                };
                if matches!(
                    &request.color_binding,
                    RockPhysicsColorRequestDto::Continuous(_)
                ) && color_value.is_none()
                {
                    continue;
                }

                samples.push(RockPhysicsSampleDto {
                    well_id: prepared.well.well_id.clone(),
                    wellbore_id: Some(prepared.well.wellbore_id.clone()),
                    sample_depth_m: *depth_m,
                    x_value,
                    y_value,
                    color_value,
                    color_category_id,
                    symbol_category_id,
                    source_binding_id: Some(source_binding_id.clone()),
                });
            }
        }

        if samples.is_empty() {
            return Err(LasError::Validation(
                "rock-physics crossplot request did not materialize any samples from the selected wells".to_string(),
            ));
        }

        let (x_min_value, x_max_value) =
            derive_rock_physics_range(samples.iter().map(|sample| sample.x_value));
        let (y_min_value, y_max_value) =
            derive_rock_physics_range(samples.iter().map(|sample| sample.y_value));

        Ok(ResolvedRockPhysicsCrossplotSourceDto {
            schema_version: ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION,
            id: format!(
                "rock-physics:{}:{}",
                rock_physics_template_slug(request.template_id),
                request.wellbore_ids.join(",")
            ),
            name: format!(
                "Resolved Rock Physics {}",
                rock_physics_template_title(request.template_id)
            ),
            template_id: request.template_id,
            title: request
                .title
                .clone()
                .or_else(|| Some(rock_physics_template_title(request.template_id).to_string())),
            subtitle: request.subtitle.clone(),
            x_axis: RockPhysicsAxisDto {
                label: Some(default_rock_physics_axis_label(request.x_semantic).to_string()),
                unit: rock_physics_semantic_unit(request.x_semantic).map(str::to_string),
                semantic: request.x_semantic,
                min_value: x_min_value,
                max_value: x_max_value,
            },
            y_axis: RockPhysicsAxisDto {
                label: Some(default_rock_physics_axis_label(request.y_semantic).to_string()),
                unit: rock_physics_semantic_unit(request.y_semantic).map(str::to_string),
                semantic: request.y_semantic,
                min_value: y_min_value,
                max_value: y_max_value,
            },
            color_binding: resolve_rock_physics_color_binding(request, &samples),
            wells: wells.iter().map(|entry| entry.well.clone()).collect(),
            samples,
            source_bindings,
            template_lines: None,
            template_overlays: None,
            interaction_thresholds: Some(RockPhysicsInteractionThresholdsDto {
                exact_point_limit: 100_000,
                progressive_point_limit: 1_000_000,
            }),
        })
    }

    pub fn resolve_well_panel_source(
        &self,
        request: &WellPanelRequestDto,
    ) -> Result<ResolvedWellPanelSourceDto> {
        if request.wellbore_ids.is_empty() {
            return Err(LasError::Validation(
                "well-panel request requires at least one wellbore id".to_string(),
            ));
        }
        if let (Some(depth_min), Some(depth_max)) = (request.depth_min, request.depth_max) {
            if depth_min > depth_max {
                return Err(LasError::Validation(
                    "well-panel request requires depth_min <= depth_max".to_string(),
                ));
            }
        }

        let wells = request
            .wellbore_ids
            .iter()
            .map(|wellbore_id| {
                self.resolve_well_panel_well(
                    &WellboreId(wellbore_id.clone()),
                    request.depth_min,
                    request.depth_max,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ResolvedWellPanelSourceDto {
            schema_version: WELL_PANEL_CONTRACT_VERSION,
            id: format!("well-panel:{}", request.wellbore_ids.join(",")),
            name: "Resolved Well Panel Source".to_string(),
            wells,
        })
    }

    pub fn resolve_survey_map_source(
        &self,
        request: &ProjectSurveyMapRequestDto,
    ) -> Result<ResolvedSurveyMapSourceDto> {
        if request.survey_asset_ids.is_empty() && request.wellbore_ids.is_empty() {
            return Err(LasError::Validation(
                "survey-map request requires at least one survey asset id or wellbore id"
                    .to_string(),
            ));
        }
        let display_coordinate_reference_id = require_project_display_coordinate_reference_id(
            &request.display_coordinate_reference_id,
        )?;

        let mut surveys = Vec::with_capacity(request.survey_asset_ids.len());
        let mut horizons = Vec::new();
        let mut scalar_field = None;
        let mut scalar_field_horizon_id = None;
        for asset_id in &request.survey_asset_ids {
            let asset_id = AssetId(asset_id.clone());
            let mut survey =
                self.resolve_survey_map_survey(&asset_id, Some(display_coordinate_reference_id))?;
            let store_root = Path::new(&self.asset_by_id(&asset_id)?.package_path).join("store");
            match resolve_survey_map_horizons_for_store(
                &asset_id.0,
                &store_root,
                &survey,
                Some(display_coordinate_reference_id),
            ) {
                Ok(resolved) => {
                    if scalar_field.is_none() {
                        scalar_field = resolved.scalar_field;
                        scalar_field_horizon_id = resolved.scalar_field_horizon_id;
                    }
                    horizons.extend(resolved.horizons);
                }
                Err(error) => survey.notes.push(format!(
                    "failed to resolve imported horizons for survey '{}': {error}",
                    survey.name
                )),
            }
            surveys.push(survey);
        }
        let wells = request
            .wellbore_ids
            .iter()
            .map(|wellbore_id| {
                self.resolve_survey_map_well(
                    &WellboreId(wellbore_id.clone()),
                    Some(display_coordinate_reference_id),
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ResolvedSurveyMapSourceDto {
            schema_version: SURVEY_MAP_CONTRACT_VERSION,
            id: format!(
                "survey-map:{}:{}",
                request.survey_asset_ids.join(","),
                request.wellbore_ids.join(",")
            ),
            name: "Resolved Survey Map Source".to_string(),
            surveys,
            wells,
            horizons,
            scalar_field,
            scalar_field_horizon_id,
        })
    }

    pub fn asset_record(&self, asset_id: &AssetId) -> Result<AssetRecord> {
        self.asset_by_id(asset_id)
    }

    pub fn asset_revisions(&self, asset_id: &AssetId) -> Result<Vec<AssetRevisionRecord>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT revision_json
                 FROM asset_revisions
                 WHERE asset_id = ?1
                 ORDER BY created_at_unix_seconds",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([&asset_id.0], |row| {
                serde_json::from_str::<AssetRevisionRecord>(&row.get::<_, String>(0)?)
                    .map_err(sql_json_error)
            })
            .map_err(sqlite_error)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)
    }

    pub fn current_asset_revision(
        &self,
        asset_id: &AssetId,
    ) -> Result<Option<AssetRevisionRecord>> {
        let mut revisions = self.asset_revisions(asset_id)?;
        Ok(revisions.pop())
    }

    pub fn overwrite_trajectory_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[TrajectoryRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Trajectory)?;
        let previous_rows = self.read_trajectory_rows(asset_id, None)?;
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_trajectory_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = trajectory_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::Trajectory, trajectory_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::Trajectory,
                previous_rows.as_slice(),
                rows,
                asset.manifest.extents
                    != structured_asset_extent(
                        AssetKind::Trajectory,
                        trajectory_extent(&previous_rows),
                    ),
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn overwrite_tops_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[TopRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::TopSet)?;
        let previous_rows = self.read_tops(asset_id)?;
        let previous_extent =
            structured_asset_extent(AssetKind::TopSet, tops_extent(&previous_rows));
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_tops_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = tops_metadata(rows).schema_version;
        asset.manifest.extents = structured_asset_extent(AssetKind::TopSet, tops_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::TopSet,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        if self
            .wellbore_by_id(&asset.wellbore_id)?
            .definitive_top_set_asset_id
            .as_ref()
            == Some(asset_id)
        {
            self.sync_well_markers_from_top_set_asset(&asset.wellbore_id, asset_id)?;
        }
        Ok(asset)
    }

    pub fn overwrite_well_marker_set_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[WellMarkerRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellMarkerSet)?;
        let previous_rows = self.read_well_marker_rows(asset_id)?;
        let previous_extent =
            structured_asset_extent(AssetKind::WellMarkerSet, well_marker_extent(&previous_rows));
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_well_markers_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = well_marker_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::WellMarkerSet, well_marker_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::WellMarkerSet,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        if self
            .wellbore_by_id(&asset.wellbore_id)?
            .definitive_marker_set_asset_id
            .as_ref()
            == Some(asset_id)
        {
            self.sync_well_markers_from_marker_set_asset(&asset.wellbore_id, asset_id)?;
        }
        Ok(asset)
    }

    pub fn overwrite_pressure_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[PressureObservationRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::PressureObservation)?;
        let previous_rows = self.read_pressure_observations(asset_id, None)?;
        let previous_extent = structured_asset_extent(
            AssetKind::PressureObservation,
            pressure_extent(&previous_rows),
        );
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_pressure_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = pressure_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::PressureObservation, pressure_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::PressureObservation,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn overwrite_drilling_asset(
        &mut self,
        asset_id: &AssetId,
        rows: &[DrillingObservationRow],
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::DrillingObservation)?;
        let previous_rows = self.read_drilling_observations(asset_id, None)?;
        let previous_extent = structured_asset_extent(
            AssetKind::DrillingObservation,
            drilling_extent(&previous_rows),
        );
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_project_asset_root(&self.root, &asset.id)?;
        write_drilling_package(&staged.root, rows)?;
        asset.manifest.asset_schema_version = drilling_metadata(rows).schema_version;
        asset.manifest.extents =
            structured_asset_extent(AssetKind::DrillingObservation, drilling_extent(rows));
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            diff_structured_rows(
                AssetKind::DrillingObservation,
                previous_rows.as_slice(),
                rows,
                previous_extent != asset.manifest.extents,
            ),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn log_curve_semantics(&self, asset_id: &AssetId) -> Result<Vec<CurveSemanticDescriptor>> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        Ok(asset.manifest.curve_semantics)
    }

    pub fn set_log_curve_semantic_override(
        &mut self,
        asset_id: &AssetId,
        curve_name: &str,
        semantic_type: CurveSemanticType,
    ) -> Result<AssetRecord> {
        let mut asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        let previous_semantics = asset.manifest.curve_semantics.clone();
        let parent_revision = self
            .current_asset_revision(asset_id)?
            .map(|item| item.revision_id);

        let mut curve_semantics = if asset.manifest.curve_semantics.is_empty() {
            classify_log_curves_from_package(&asset.package_path)?
        } else {
            asset.manifest.curve_semantics.clone()
        };

        let descriptor = curve_semantics
            .iter_mut()
            .find(|item| item.curve_name == curve_name)
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "curve '{}' not found in log asset '{}'",
                    curve_name, asset.id.0
                ))
            })?;
        descriptor.semantic_type = semantic_type;
        descriptor.source = CurveSemanticSource::Override;
        asset.manifest.curve_semantics = curve_semantics;
        let staged = stage_existing_asset_root(&self.root, &asset)?;
        write_asset_manifest(&staged.root, &asset.manifest)?;
        let changed_fields =
            semantic_diff_fields(&previous_semantics, &asset.manifest.curve_semantics);
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            AssetDiffSummary::MetadataOnly { changed_fields },
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;
        Ok(asset)
    }

    pub fn sync_log_asset_head_revision(
        &mut self,
        asset_id: &AssetId,
    ) -> Result<AssetRevisionRecord> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::Log)?;
        let current = open_package(&asset.package_path)?;
        let parent = self.current_asset_revision(asset_id)?;
        let staged = stage_existing_asset_root(&self.root, &asset)?;
        let diff_summary = if let Some(previous) = &parent {
            let snapshot_root = self.root.join(&previous.package_snapshot_rel_path);
            if snapshot_root.exists() {
                let previous_package = open_package(&snapshot_root)?;
                AssetDiffSummary::Log(diff_log_files(previous_package.file(), current.file()))
            } else {
                default_asset_diff_summary(&AssetKind::Log)
            }
        } else {
            default_asset_diff_summary(&AssetKind::Log)
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent.as_ref().map(|item| &item.revision_id),
            diff_summary,
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        Ok(revision)
    }

    pub fn list_compute_catalog(&self, asset_id: &AssetId) -> Result<ComputeCatalog> {
        let asset = self.asset_by_id(asset_id)?;
        let registry = ComputeRegistry::new();
        let resolved_packages = self.resolved_operator_packages()?;
        let asset_family = asset_semantic_family(&asset.asset_kind)?;
        let built_in_enabled = resolved_packages
            .iter()
            .any(|package| package.package_name == BUILTIN_OPERATOR_PACKAGE_NAME);
        match asset.asset_kind {
            AssetKind::Log => {
                let semantics = if asset.manifest.curve_semantics.is_empty() {
                    classify_log_curves_from_package(&asset.package_path)?
                } else {
                    asset.manifest.curve_semantics.clone()
                };
                let package = open_package(&asset.package_path)?;
                let numeric_curve_names = package
                    .file()
                    .curves
                    .iter()
                    .filter_map(|curve| curve.numeric_data().map(|_| curve.mnemonic.clone()))
                    .collect::<Vec<_>>();
                let mut catalog = if built_in_enabled {
                    registry.catalog_for_log_asset(&semantics, &numeric_curve_names)
                } else {
                    ComputeCatalog {
                        asset_family: asset_family.clone(),
                        functions: Vec::new(),
                    }
                };
                catalog.functions.extend(external_operator_catalog_entries(
                    &resolved_packages,
                    &asset_family,
                    Some((&semantics, &numeric_curve_names)),
                ));
                Ok(catalog)
            }
            AssetKind::Trajectory => {
                let mut catalog = if built_in_enabled {
                    registry.catalog_for_trajectory_asset()
                } else {
                    ComputeCatalog {
                        asset_family: asset_family.clone(),
                        functions: Vec::new(),
                    }
                };
                catalog.functions.extend(external_operator_catalog_entries(
                    &resolved_packages,
                    &asset_family,
                    None,
                ));
                Ok(catalog)
            }
            AssetKind::TopSet => {
                let mut catalog = if built_in_enabled {
                    registry.catalog_for_top_set_asset()
                } else {
                    ComputeCatalog {
                        asset_family: asset_family.clone(),
                        functions: Vec::new(),
                    }
                };
                catalog.functions.extend(external_operator_catalog_entries(
                    &resolved_packages,
                    &asset_family,
                    None,
                ));
                if built_in_enabled {
                    catalog
                        .functions
                        .push(well_marker_horizon_residual_catalog_entry(
                            AssetKind::TopSet,
                        ));
                }
                Ok(catalog)
            }
            AssetKind::WellMarkerSet => {
                let mut catalog = ComputeCatalog {
                    asset_family,
                    functions: Vec::new(),
                };
                if built_in_enabled {
                    catalog
                        .functions
                        .push(well_marker_horizon_residual_catalog_entry(
                            AssetKind::WellMarkerSet,
                        ));
                }
                Ok(catalog)
            }
            AssetKind::WellMarkerHorizonResidualSet => Err(LasError::Validation(
                "compute catalog is not implemented for well marker horizon residual assets"
                    .to_string(),
            )),
            AssetKind::PressureObservation => {
                let mut catalog = if built_in_enabled {
                    registry.catalog_for_pressure_asset()
                } else {
                    ComputeCatalog {
                        asset_family: asset_family.clone(),
                        functions: Vec::new(),
                    }
                };
                catalog.functions.extend(external_operator_catalog_entries(
                    &resolved_packages,
                    &asset_family,
                    None,
                ));
                Ok(catalog)
            }
            AssetKind::DrillingObservation => {
                let mut catalog = if built_in_enabled {
                    registry.catalog_for_drilling_asset()
                } else {
                    ComputeCatalog {
                        asset_family: asset_family.clone(),
                        functions: Vec::new(),
                    }
                };
                catalog.functions.extend(external_operator_catalog_entries(
                    &resolved_packages,
                    &asset_family,
                    None,
                ));
                Ok(catalog)
            }
            AssetKind::CheckshotVspObservationSet
            | AssetKind::ManualTimeDepthPickSet
            | AssetKind::WellTieObservationSet
            | AssetKind::WellTimeDepthAuthoredModel
            | AssetKind::RawSourceBundle => Err(LasError::Validation(
                "compute catalog is not implemented for well time-depth observation/model assets"
                    .to_string(),
            )),
            AssetKind::WellTimeDepthModel => Err(LasError::Validation(
                "compute catalog is not implemented for well time-depth model assets".to_string(),
            )),
            AssetKind::SeismicTraceData => Err(LasError::Validation(
                "compute catalog is not implemented for seismic assets yet".to_string(),
            )),
        }
    }

    pub fn run_compute(
        &mut self,
        request: &ProjectComputeRunRequest,
    ) -> Result<ProjectComputeRunResult> {
        let source_asset = self.asset_by_id(&request.source_asset_id)?;
        let source_collection = self.collection_by_id(&source_asset.collection_id)?;
        if request.function_id == WELL_MARKER_HORIZON_RESIDUAL_FUNCTION_ID {
            return self.run_well_marker_horizon_residual_compute(
                &source_asset,
                &source_collection,
                request,
            );
        }
        let (resolved_entry, resolved_package, resolved_operator) =
            self.resolved_operator_entry(&request.function_id)?;
        if resolved_package.package_name != BUILTIN_OPERATOR_PACKAGE_NAME {
            return self.run_external_compute(
                &resolved_entry,
                &resolved_package,
                &resolved_operator,
                &source_asset,
                &source_collection,
                request,
            );
        }
        let registry = ComputeRegistry::new();

        let (collection, asset, execution) = match source_asset.asset_kind {
            AssetKind::Log => {
                let source_package = open_package(&source_asset.package_path)?;
                let source_file = source_package.file();
                let semantics = if source_asset.manifest.curve_semantics.is_empty() {
                    classify_log_curves_from_package(&source_asset.package_path)?
                } else {
                    source_asset.manifest.curve_semantics.clone()
                };
                let log_curves = log_curve_data_for_compute(source_file, &semantics)?;
                let (execution, computed_curve) = registry.run_log_compute(
                    &request.function_id,
                    &log_curves,
                    &request.curve_bindings,
                    &request.parameters,
                    request.output_mnemonic.as_deref(),
                )?;
                self.persist_log_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    source_file,
                    execution,
                    &computed_curve,
                )?
            }
            AssetKind::Trajectory => {
                let rows = self.read_trajectory_rows(&source_asset.id, None)?;
                let compute_rows = trajectory_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_trajectory_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    trajectory_rows_from_compute(&derived_rows),
                    AssetKind::Trajectory,
                )?
            }
            AssetKind::TopSet => {
                let rows = self.read_tops(&source_asset.id)?;
                let compute_rows = top_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_top_set_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    top_rows_from_compute(&derived_rows),
                    AssetKind::TopSet,
                )?
            }
            AssetKind::WellMarkerSet => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well marker set assets".to_string(),
                ));
            }
            AssetKind::WellMarkerHorizonResidualSet => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well marker horizon residual assets"
                        .to_string(),
                ));
            }
            AssetKind::PressureObservation => {
                let rows = self.read_pressure_observations(&source_asset.id, None)?;
                let compute_rows = pressure_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_pressure_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    pressure_rows_from_compute(&derived_rows),
                    AssetKind::PressureObservation,
                )?
            }
            AssetKind::DrillingObservation => {
                let rows = self.read_drilling_observations(&source_asset.id, None)?;
                let compute_rows = drilling_rows_for_compute(&rows);
                let (execution, derived_rows) = registry.run_drilling_compute(
                    &request.function_id,
                    &compute_rows,
                    &request.parameters,
                )?;
                self.persist_structured_compute_result(
                    &source_asset,
                    &source_collection,
                    request,
                    execution,
                    drilling_rows_from_compute(&derived_rows),
                    AssetKind::DrillingObservation,
                )?
            }
            AssetKind::CheckshotVspObservationSet
            | AssetKind::ManualTimeDepthPickSet
            | AssetKind::WellTieObservationSet
            | AssetKind::WellTimeDepthAuthoredModel
            | AssetKind::RawSourceBundle => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well time-depth observation/model assets"
                        .to_string(),
                ));
            }
            AssetKind::WellTimeDepthModel => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well time-depth model assets"
                        .to_string(),
                ));
            }
            AssetKind::SeismicTraceData => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for seismic assets yet".to_string(),
                ));
            }
        };

        Ok(ProjectComputeRunResult {
            collection,
            asset,
            execution,
        })
    }

    fn run_external_compute(
        &mut self,
        entry: &OperatorPackageLockEntry,
        package: &OperatorPackageManifest,
        operator: &OperatorManifest,
        source_asset: &AssetRecord,
        source_collection: &AssetCollectionRecord,
        request: &ProjectComputeRunRequest,
    ) -> Result<ProjectComputeRunResult> {
        let (collection, asset, execution) = match source_asset.asset_kind {
            AssetKind::Log => {
                let source_package = open_package(&source_asset.package_path)?;
                let source_file = source_package.file();
                let semantics = if source_asset.manifest.curve_semantics.is_empty() {
                    classify_log_curves_from_package(&source_asset.package_path)?
                } else {
                    source_asset.manifest.curve_semantics.clone()
                };
                let log_curves = log_curve_data_for_compute(source_file, &semantics)?;
                let (resolved_inputs, manifest_inputs) = resolve_log_input_bindings(
                    &request.function_id,
                    &operator.input_specs,
                    &log_curves,
                    &request.curve_bindings,
                )?;
                validate_compute_parameters(&operator.parameters, &request.parameters)?;
                let response = self.invoke_external_python_operator(
                    entry,
                    package,
                    ExternalOperatorRequest {
                        operator_id: operator.id.clone(),
                        package_name: package.package_name.clone(),
                        package_version: package.package_version.clone(),
                        parameters: request.parameters.clone(),
                        payload: ExternalOperatorRequestPayload::Log {
                            inputs: resolved_inputs,
                            output_mnemonic: request.output_mnemonic.clone(),
                        },
                    },
                )?;
                let ExternalOperatorResponsePayload::Log { computed_curve } = response.payload
                else {
                    return Err(LasError::Validation(format!(
                        "external operator '{}' returned a non-log payload for a log asset",
                        operator.id
                    )));
                };
                let execution = external_execution_manifest(
                    package,
                    operator,
                    manifest_inputs,
                    &request.parameters,
                    computed_curve.curve_name.clone(),
                    computed_curve.semantic_type.clone(),
                );
                self.persist_log_compute_result(
                    source_asset,
                    source_collection,
                    request,
                    source_file,
                    execution,
                    &computed_curve,
                )?
            }
            AssetKind::Trajectory => {
                let rows = self.read_trajectory_rows(&source_asset.id, None)?;
                let compute_rows = trajectory_rows_for_compute(&rows);
                validate_compute_parameters(&operator.parameters, &request.parameters)?;
                let response = self.invoke_external_python_operator(
                    entry,
                    package,
                    ExternalOperatorRequest {
                        operator_id: operator.id.clone(),
                        package_name: package.package_name.clone(),
                        package_version: package.package_version.clone(),
                        parameters: request.parameters.clone(),
                        payload: ExternalOperatorRequestPayload::Trajectory { rows: compute_rows },
                    },
                )?;
                let ExternalOperatorResponsePayload::Trajectory { rows: derived_rows } =
                    response.payload
                else {
                    return Err(LasError::Validation(format!(
                        "external operator '{}' returned a non-trajectory payload for a trajectory asset",
                        operator.id
                    )));
                };
                self.persist_structured_compute_result(
                    source_asset,
                    source_collection,
                    request,
                    external_execution_manifest(
                        package,
                        operator,
                        vec![structured_compute_input_binding(
                            "trajectory_rows",
                            "trajectory",
                        )],
                        &request.parameters,
                        "trajectory",
                        operator.output_curve_type.clone(),
                    ),
                    trajectory_rows_from_compute(&derived_rows),
                    AssetKind::Trajectory,
                )?
            }
            AssetKind::TopSet => {
                let rows = self.read_tops(&source_asset.id)?;
                let compute_rows = top_rows_for_compute(&rows);
                validate_compute_parameters(&operator.parameters, &request.parameters)?;
                let response = self.invoke_external_python_operator(
                    entry,
                    package,
                    ExternalOperatorRequest {
                        operator_id: operator.id.clone(),
                        package_name: package.package_name.clone(),
                        package_version: package.package_version.clone(),
                        parameters: request.parameters.clone(),
                        payload: ExternalOperatorRequestPayload::TopSet { rows: compute_rows },
                    },
                )?;
                let ExternalOperatorResponsePayload::TopSet { rows: derived_rows } =
                    response.payload
                else {
                    return Err(LasError::Validation(format!(
                        "external operator '{}' returned a non-top-set payload for a tops asset",
                        operator.id
                    )));
                };
                self.persist_structured_compute_result(
                    source_asset,
                    source_collection,
                    request,
                    external_execution_manifest(
                        package,
                        operator,
                        vec![structured_compute_input_binding("tops_rows", "tops")],
                        &request.parameters,
                        "tops",
                        operator.output_curve_type.clone(),
                    ),
                    top_rows_from_compute(&derived_rows),
                    AssetKind::TopSet,
                )?
            }
            AssetKind::WellMarkerSet => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well marker set assets".to_string(),
                ));
            }
            AssetKind::WellMarkerHorizonResidualSet => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well marker horizon residual assets"
                        .to_string(),
                ));
            }
            AssetKind::PressureObservation => {
                let rows = self.read_pressure_observations(&source_asset.id, None)?;
                let compute_rows = pressure_rows_for_compute(&rows);
                validate_compute_parameters(&operator.parameters, &request.parameters)?;
                let response = self.invoke_external_python_operator(
                    entry,
                    package,
                    ExternalOperatorRequest {
                        operator_id: operator.id.clone(),
                        package_name: package.package_name.clone(),
                        package_version: package.package_version.clone(),
                        parameters: request.parameters.clone(),
                        payload: ExternalOperatorRequestPayload::PressureObservation {
                            rows: compute_rows,
                        },
                    },
                )?;
                let ExternalOperatorResponsePayload::PressureObservation { rows: derived_rows } =
                    response.payload
                else {
                    return Err(LasError::Validation(format!(
                        "external operator '{}' returned a non-pressure payload for a pressure asset",
                        operator.id
                    )));
                };
                self.persist_structured_compute_result(
                    source_asset,
                    source_collection,
                    request,
                    external_execution_manifest(
                        package,
                        operator,
                        vec![structured_compute_input_binding(
                            "pressure_rows",
                            "pressure",
                        )],
                        &request.parameters,
                        "pressure",
                        operator.output_curve_type.clone(),
                    ),
                    pressure_rows_from_compute(&derived_rows),
                    AssetKind::PressureObservation,
                )?
            }
            AssetKind::DrillingObservation => {
                let rows = self.read_drilling_observations(&source_asset.id, None)?;
                let compute_rows = drilling_rows_for_compute(&rows);
                validate_compute_parameters(&operator.parameters, &request.parameters)?;
                let response = self.invoke_external_python_operator(
                    entry,
                    package,
                    ExternalOperatorRequest {
                        operator_id: operator.id.clone(),
                        package_name: package.package_name.clone(),
                        package_version: package.package_version.clone(),
                        parameters: request.parameters.clone(),
                        payload: ExternalOperatorRequestPayload::DrillingObservation {
                            rows: compute_rows,
                        },
                    },
                )?;
                let ExternalOperatorResponsePayload::DrillingObservation { rows: derived_rows } =
                    response.payload
                else {
                    return Err(LasError::Validation(format!(
                        "external operator '{}' returned a non-drilling payload for a drilling asset",
                        operator.id
                    )));
                };
                self.persist_structured_compute_result(
                    source_asset,
                    source_collection,
                    request,
                    external_execution_manifest(
                        package,
                        operator,
                        vec![structured_compute_input_binding(
                            "drilling_rows",
                            "drilling",
                        )],
                        &request.parameters,
                        "drilling",
                        operator.output_curve_type.clone(),
                    ),
                    drilling_rows_from_compute(&derived_rows),
                    AssetKind::DrillingObservation,
                )?
            }
            AssetKind::CheckshotVspObservationSet
            | AssetKind::ManualTimeDepthPickSet
            | AssetKind::WellTieObservationSet
            | AssetKind::WellTimeDepthAuthoredModel
            | AssetKind::RawSourceBundle => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well time-depth observation/model assets"
                        .to_string(),
                ));
            }
            AssetKind::WellTimeDepthModel => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for well time-depth model assets"
                        .to_string(),
                ));
            }
            AssetKind::SeismicTraceData => {
                return Err(LasError::Validation(
                    "compute execution is not implemented for seismic assets yet".to_string(),
                ));
            }
        };

        Ok(ProjectComputeRunResult {
            collection,
            asset,
            execution,
        })
    }

    fn run_well_marker_horizon_residual_compute(
        &mut self,
        source_asset: &AssetRecord,
        source_collection: &AssetCollectionRecord,
        request: &ProjectComputeRunRequest,
    ) -> Result<ProjectComputeRunResult> {
        match source_asset.asset_kind {
            AssetKind::TopSet | AssetKind::WellMarkerSet => {}
            _ => {
                return Err(LasError::Validation(
                    "depth-horizon residuals are only available for top-set and well-marker-set assets"
                        .to_string(),
                ));
            }
        }

        let catalog_entry =
            well_marker_horizon_residual_catalog_entry(source_asset.asset_kind.clone());
        validate_compute_parameters(&catalog_entry.parameters, &request.parameters)?;
        let survey_asset_id = AssetId(required_string_compute_parameter(
            &request.parameters,
            WELL_MARKER_HORIZON_RESIDUAL_SURVEY_ASSET_PARAMETER,
        )?);
        let horizon_id = required_string_compute_parameter(
            &request.parameters,
            WELL_MARKER_HORIZON_RESIDUAL_HORIZON_PARAMETER,
        )?;
        let marker_name = optional_string_compute_parameter(
            &request.parameters,
            WELL_MARKER_HORIZON_RESIDUAL_MARKER_PARAMETER,
        );
        let resolved = self.resolve_well_marker_horizon_residual_rows_from_request(
            &WellMarkerHorizonResidualResolveRequest {
                source_asset_id: source_asset.id.clone(),
                survey_asset_id: survey_asset_id.clone(),
                horizon_id: horizon_id.clone(),
                marker_name,
            },
        )?;

        let source_binding = match source_asset.asset_kind {
            AssetKind::TopSet => structured_compute_input_binding("tops_rows", "tops"),
            AssetKind::WellMarkerSet => {
                structured_compute_input_binding("well_marker_rows", "well_markers")
            }
            _ => unreachable!(),
        };
        let horizon_binding = structured_compute_input_binding(
            "survey_horizon",
            &format!("{}::{}", survey_asset_id.0, horizon_id),
        );
        let execution = builtin_structured_execution_manifest(
            WELL_MARKER_HORIZON_RESIDUAL_FUNCTION_ID,
            "well_markers",
            "Compute Depth-Horizon Residuals",
            vec![source_binding, horizon_binding],
            &request.parameters,
            "well_marker_horizon_residuals",
        );
        let rows = resolved
            .rows
            .iter()
            .map(well_marker_horizon_residual_row_from_dto)
            .collect::<Vec<_>>();
        let (collection, asset, execution) = self
            .persist_well_marker_horizon_residual_compute_result(
                source_asset,
                source_collection,
                request,
                execution,
                &rows,
            )?;
        Ok(ProjectComputeRunResult {
            collection,
            asset,
            execution,
        })
    }

    fn persist_well_marker_horizon_residual_compute_result(
        &mut self,
        source_asset: &AssetRecord,
        source_collection: &AssetCollectionRecord,
        request: &ProjectComputeRunRequest,
        mut execution: ComputeExecutionManifest,
        rows: &[WellMarkerHorizonResidualRow],
    ) -> Result<(AssetCollectionRecord, AssetRecord, ComputeExecutionManifest)> {
        execution.source_asset_id = source_asset.id.0.clone();
        execution.source_logical_asset_id = source_asset.logical_asset_id.0.clone();
        execution.executed_at_unix_seconds = now_unix_seconds();

        let collection_name = request.output_collection_name.clone().unwrap_or_else(|| {
            format!(
                "{} / Derived / {}",
                source_collection.name, execution.function_name
            )
        });
        let collection = self.resolve_or_create_collection(
            &source_asset.wellbore_id,
            AssetKind::WellMarkerHorizonResidualSet,
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::WellMarkerHorizonResidualSet.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        write_well_marker_horizon_residuals_package(&staged.root, rows)?;
        let mut manifest = derived_structured_asset_manifest(
            source_asset,
            &collection,
            &storage_asset_id,
            supersedes.clone(),
            &execution,
            AssetKind::WellMarkerHorizonResidualSet,
            well_marker_horizon_residual_metadata(rows),
            structured_asset_extent(
                AssetKind::WellMarkerHorizonResidualSet,
                well_marker_horizon_residual_extent(rows),
            ),
        )?;
        manifest.reference_metadata.depth_reference = DepthReference::Unknown;
        manifest.reference_metadata.vertical_datum = None;
        manifest.reference_metadata.unit_system.depth_unit = Some("m".to_string());
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: source_asset.well_id.clone(),
            wellbore_id: source_asset.wellbore_id.clone(),
            asset_kind: AssetKind::WellMarkerHorizonResidualSet,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok((collection, asset, execution))
    }

    fn invoke_external_python_operator(
        &self,
        entry: &OperatorPackageLockEntry,
        package: &OperatorPackageManifest,
        request: ExternalOperatorRequest,
    ) -> Result<ExternalOperatorResponse> {
        if package.runtime != OperatorRuntimeKind::Python {
            return Err(LasError::Validation(format!(
                "operator package '{}@{}' uses runtime '{:?}', which is not supported for external dispatch",
                package.package_name, package.package_version, package.runtime
            )));
        }

        let entrypoint = package.entrypoint.as_ref().ok_or_else(|| {
            LasError::Validation(format!(
                "operator package '{}@{}' is missing a python entrypoint",
                package.package_name, package.package_version
            ))
        })?;
        let manifest_path = self
            .resolve_operator_package_source_path(entry)?
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "operator package '{}@{}' is missing a manifest path",
                    package.package_name, package.package_version
                ))
            })?;
        let manifest_dir = manifest_path.parent().ok_or_else(|| {
            LasError::Validation(format!(
                "operator package manifest '{}' has no parent directory",
                manifest_path.display()
            ))
        })?;

        let mut command = Command::new(python_runtime_for_operator_manifest_dir(manifest_dir));
        command
            .arg("-m")
            .arg("ophiolite_sdk.external_runner")
            .arg("--entrypoint")
            .arg(entrypoint)
            .arg("--manifest-dir")
            .arg(manifest_dir)
            .current_dir(manifest_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONPATH", external_pythonpath()?);

        let mut child = command.spawn().map_err(|error| {
            LasError::Storage(format!(
                "failed to start python operator runtime for '{}@{}': {error}",
                package.package_name, package.package_version
            ))
        })?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(&serde_json::to_vec(&request)?)?;
        } else {
            return Err(LasError::Storage(
                "failed to open stdin for python operator runtime".to_string(),
            ));
        }

        let output = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            let detail = if stderr.is_empty() {
                stdout
            } else if stdout.is_empty() {
                stderr
            } else {
                format!("{stderr}\n{stdout}")
            };
            return Err(LasError::Validation(format!(
                "python operator '{}' failed: {detail}",
                request.operator_id
            )));
        }

        serde_json::from_str(&stdout).map_err(|error| {
            LasError::Validation(format!(
                "python operator '{}' returned invalid json: {error}",
                request.operator_id
            ))
        })
    }

    fn persist_log_compute_result(
        &mut self,
        source_asset: &AssetRecord,
        source_collection: &AssetCollectionRecord,
        request: &ProjectComputeRunRequest,
        source_file: &LasFile,
        mut execution: ComputeExecutionManifest,
        computed_curve: &ophiolite_compute::ComputedCurve,
    ) -> Result<(AssetCollectionRecord, AssetRecord, ComputeExecutionManifest)> {
        execution.source_asset_id = source_asset.id.0.clone();
        execution.source_logical_asset_id = source_asset.logical_asset_id.0.clone();
        execution.executed_at_unix_seconds = now_unix_seconds();

        let collection_name = request.output_collection_name.clone().unwrap_or_else(|| {
            format!(
                "{} / Derived / {}",
                source_collection.name, execution.function_name
            )
        });
        let collection = self.resolve_or_create_collection(
            &source_asset.wellbore_id,
            AssetKind::Log,
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::Log.asset_dir_name())
            .join(format!("{}.laspkg", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        let derived_file = build_derived_log_file(
            source_file,
            source_asset,
            &collection,
            &storage_asset_id,
            computed_curve,
            &execution,
        );
        write_package_overwrite(&derived_file, &staged.root)?;

        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = derived_log_asset_manifest(
            &derived_file,
            source_asset,
            &collection,
            &storage_asset_id,
            supersedes.clone(),
            computed_curve,
            &execution,
        );
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: source_asset.well_id.clone(),
            wellbore_id: source_asset.wellbore_id.clone(),
            asset_kind: AssetKind::Log,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            AssetDiffSummary::Log(Default::default()),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        let refreshed_wellbore = self.wellbore_by_id(&asset.wellbore_id)?;
        if matches!(asset.asset_kind, AssetKind::Trajectory)
            && refreshed_wellbore.definitive_trajectory_asset_id.is_none()
        {
            self.set_definitive_trajectory_asset(&asset.wellbore_id, Some(&asset.id))?;
        }
        if matches!(asset.asset_kind, AssetKind::TopSet)
            && refreshed_wellbore.definitive_top_set_asset_id.is_none()
        {
            self.set_definitive_top_set_asset(&asset.wellbore_id, Some(&asset.id))?;
        }
        if matches!(asset.asset_kind, AssetKind::WellMarkerSet)
            && refreshed_wellbore.definitive_marker_set_asset_id.is_none()
        {
            self.set_definitive_marker_set_asset(&asset.wellbore_id, Some(&asset.id))?;
        }
        Ok((collection, asset, execution))
    }

    fn persist_structured_compute_result(
        &mut self,
        source_asset: &AssetRecord,
        source_collection: &AssetCollectionRecord,
        request: &ProjectComputeRunRequest,
        mut execution: ComputeExecutionManifest,
        rows: StructuredComputedRows,
        asset_kind: AssetKind,
    ) -> Result<(AssetCollectionRecord, AssetRecord, ComputeExecutionManifest)> {
        execution.source_asset_id = source_asset.id.0.clone();
        execution.source_logical_asset_id = source_asset.logical_asset_id.0.clone();
        execution.executed_at_unix_seconds = now_unix_seconds();

        let collection_name = request.output_collection_name.clone().unwrap_or_else(|| {
            format!(
                "{} / Derived / {}",
                source_collection.name, execution.function_name
            )
        });
        let collection = self.resolve_or_create_collection(
            &source_asset.wellbore_id,
            asset_kind.clone(),
            &collection_name,
        )?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(match asset_kind {
                AssetKind::Log => format!("{}.laspkg", storage_asset_id.0),
                _ => format!("{}.ophiolite-asset", storage_asset_id.0),
            });
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = write_structured_compute_rows(
            &staged.root,
            source_asset,
            &collection,
            &storage_asset_id,
            supersedes.clone(),
            &rows,
            &execution,
            asset_kind.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: source_asset.well_id.clone(),
            wellbore_id: source_asset.wellbore_id.clone(),
            asset_kind,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok((collection, asset, execution))
    }

    pub fn assets_covering_depth_range(
        &self,
        wellbore_id: &WellboreId,
        depth_min: f64,
        depth_max: f64,
    ) -> Result<Vec<AssetRecord>> {
        if depth_min > depth_max {
            return Err(LasError::Validation(String::from(
                "depth range requires depth_min <= depth_max",
            )));
        }
        Ok(self
            .list_assets(wellbore_id, None)?
            .into_iter()
            .filter(|asset| asset_covers_depth_range(asset, depth_min, depth_max))
            .collect())
    }

    fn resolve_well_panel_well(
        &self,
        wellbore_id: &WellboreId,
        depth_min: Option<f64>,
        depth_max: Option<f64>,
    ) -> Result<ResolvedWellPanelWellDto> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        let current_assets = self
            .asset_summaries(wellbore_id, None)?
            .into_iter()
            .filter(|summary| summary.is_current)
            .collect::<Vec<_>>();

        let mut logs = Vec::new();
        let mut trajectories = Vec::new();
        let mut top_sets = Vec::new();
        let mut pressure_observations = Vec::new();
        let mut drilling_observations = Vec::new();
        let mut depth_samples = Vec::new();

        for summary in current_assets {
            let asset = summary.asset;
            let collection = self.collection_by_id(&asset.collection_id)?;
            match asset.asset_kind {
                AssetKind::Log => {
                    for curve in self.read_log_curve_data(&asset.id)? {
                        let filtered =
                            filter_log_curve_for_depth_range(&curve, depth_min, depth_max);
                        if filtered.depths.is_empty() {
                            continue;
                        }
                        depth_samples.extend(filtered.depths.iter().copied());
                        logs.push(WellPanelLogCurveDto {
                            asset_id: asset.id.0.clone(),
                            logical_asset_id: asset.logical_asset_id.0.clone(),
                            asset_name: collection.name.clone(),
                            curve_name: filtered.curve_name,
                            original_mnemonic: filtered.original_mnemonic,
                            unit: filtered.unit,
                            semantic_type: format!("{:?}", filtered.semantic_type),
                            log_type: filtered.semantic_type.log_type_name().to_string(),
                            depths: filtered.depths,
                            values: filtered.values,
                        });
                    }
                }
                AssetKind::Trajectory => {
                    let range = depth_query(depth_min, depth_max);
                    let rows = self.read_trajectory_rows(&asset.id, range.as_ref())?;
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().map(|row| row.measured_depth));
                    trajectories.push(WellPanelTrajectoryDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelTrajectoryRowDto {
                                measured_depth: row.measured_depth,
                                true_vertical_depth: row.true_vertical_depth,
                                true_vertical_depth_subsea: row.true_vertical_depth_subsea,
                                azimuth_deg: row.azimuth_deg,
                                inclination_deg: row.inclination_deg,
                                northing_offset: row.northing_offset,
                                easting_offset: row.easting_offset,
                            })
                            .collect(),
                    });
                }
                AssetKind::TopSet => {
                    let rows = filter_top_rows_for_depth_range(
                        self.read_tops(&asset.id)?,
                        depth_min,
                        depth_max,
                    );
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().map(|row| row.top_depth));
                    top_sets.push(WellPanelTopSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        set_kind: "top_set".to_string(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelTopRowDto {
                                name: row.name,
                                top_depth: row.top_depth,
                                base_depth: row.base_depth,
                                source: row.source,
                                source_depth_reference: row.source_depth_reference,
                                depth_domain: row.depth_domain,
                                depth_datum: row.depth_datum,
                            })
                            .collect(),
                    });
                }
                AssetKind::WellMarkerSet => {
                    let rows = filter_well_marker_rows_for_depth_range(
                        self.read_well_marker_rows(&asset.id)?,
                        depth_min,
                        depth_max,
                    );
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().map(|row| row.top_depth));
                    top_sets.push(WellPanelTopSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        set_kind: "well_marker_set".to_string(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelTopRowDto {
                                name: row.name,
                                top_depth: row.top_depth,
                                base_depth: row.base_depth,
                                source: row.source,
                                source_depth_reference: row.source_depth_reference,
                                depth_domain: row.depth_domain,
                                depth_datum: row.depth_datum,
                            })
                            .collect(),
                    });
                }
                AssetKind::WellMarkerHorizonResidualSet => {}
                AssetKind::PressureObservation => {
                    let range = depth_query(depth_min, depth_max);
                    let rows = self.read_pressure_observations(&asset.id, range.as_ref())?;
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().filter_map(|row| row.measured_depth));
                    pressure_observations.push(WellPanelPressureSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelPressureObservationDto {
                                measured_depth: row.measured_depth,
                                pressure: row.pressure,
                                phase: row.phase,
                                test_kind: row.test_kind,
                                timestamp: row.timestamp,
                            })
                            .collect(),
                    });
                }
                AssetKind::DrillingObservation => {
                    let range = depth_query(depth_min, depth_max);
                    let rows = self.read_drilling_observations(&asset.id, range.as_ref())?;
                    if rows.is_empty() {
                        continue;
                    }
                    depth_samples.extend(rows.iter().filter_map(|row| row.measured_depth));
                    drilling_observations.push(WellPanelDrillingSetDto {
                        asset_id: asset.id.0.clone(),
                        logical_asset_id: asset.logical_asset_id.0.clone(),
                        asset_name: collection.name.clone(),
                        rows: rows
                            .into_iter()
                            .map(|row| WellPanelDrillingObservationDto {
                                measured_depth: row.measured_depth,
                                event_kind: row.event_kind,
                                value: row.value,
                                unit: row.unit,
                                timestamp: row.timestamp,
                                comment: row.comment,
                            })
                            .collect(),
                    });
                }
                AssetKind::CheckshotVspObservationSet => {}
                AssetKind::ManualTimeDepthPickSet => {}
                AssetKind::WellTieObservationSet => {}
                AssetKind::WellTimeDepthAuthoredModel => {}
                AssetKind::WellTimeDepthModel => {}
                AssetKind::RawSourceBundle => {}
                AssetKind::SeismicTraceData => {}
            }
        }

        let panel_depth_mapping = identity_panel_depth_mapping(depth_samples);
        Ok(ResolvedWellPanelWellDto {
            well_id: wellbore.well_id.0.clone(),
            wellbore_id: wellbore.id.0.clone(),
            name: wellbore.name,
            native_depth_datum: "measured_depth".to_string(),
            panel_depth_mapping,
            logs,
            trajectories,
            top_sets,
            pressure_observations,
            drilling_observations,
        })
    }

    fn prepare_rock_physics_well(
        &self,
        wellbore_id: &WellboreId,
    ) -> Result<RockPhysicsPreparedWell> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        let current_assets = self
            .asset_summaries(wellbore_id, None)?
            .into_iter()
            .filter(|summary| summary.is_current)
            .collect::<Vec<_>>();

        let mut curves_by_semantic =
            HashMap::<CurveSemanticType, Vec<RockPhysicsCurveSource>>::new();
        for summary in current_assets {
            let asset = summary.asset;
            if asset.asset_kind != AssetKind::Log {
                continue;
            }
            for curve in self.read_log_curve_data(&asset.id)? {
                let prepared = match prepare_interpolated_log_curve(&curve) {
                    Ok(prepared) => prepared,
                    Err(_) => continue,
                };
                curves_by_semantic
                    .entry(curve.semantic_type.clone())
                    .or_default()
                    .push(RockPhysicsCurveSource {
                        curve_id: format!("{}:{}", asset.id.0, curve.curve_name),
                        curve,
                        prepared,
                    });
            }
        }

        for curves in curves_by_semantic.values_mut() {
            curves.sort_by(|left, right| {
                right
                    .prepared
                    .point_count
                    .cmp(&left.prepared.point_count)
                    .then_with(|| left.curve.curve_name.cmp(&right.curve.curve_name))
            });
        }

        Ok(RockPhysicsPreparedWell {
            well: RockPhysicsWellDto {
                well_id: wellbore.well_id.0.clone(),
                wellbore_id: wellbore.id.0.clone(),
                name: wellbore.name,
                color: None,
            },
            curves_by_semantic,
        })
    }

    fn resolve_survey_map_survey(
        &self,
        asset_id: &AssetId,
        display_coordinate_reference_id: Option<&str>,
    ) -> Result<ResolvedSurveyMapSurveyDto> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::SeismicTraceData)?;
        let collection = self.collection_by_id(&asset.collection_id)?;
        let metadata = read_seismic_asset_metadata(Path::new(&asset.package_path))?;
        let inline_axis = &metadata.descriptor.geometry.summary.inline_axis;
        let xline_axis = &metadata.descriptor.geometry.summary.xline_axis;
        let survey_name = if collection.name.trim().is_empty() {
            metadata.descriptor.label.clone()
        } else {
            collection.name.clone()
        };
        let coordinate_reference_binding = metadata
            .descriptor
            .coordinate_reference_binding
            .as_ref()
            .map(coordinate_reference_binding_dto_from_seismic);
        let native_spatial = metadata
            .descriptor
            .spatial
            .as_ref()
            .map(survey_spatial_descriptor_dto_from_seismic)
            .unwrap_or_else(|| SurveyMapSpatialDescriptorDto {
                coordinate_reference: coordinate_reference_dto(
                    asset
                        .manifest
                        .reference_metadata
                        .coordinate_reference
                        .as_ref(),
                    asset
                        .manifest
                        .reference_metadata
                        .unit_system
                        .coordinate_unit
                        .as_deref(),
                ),
                grid_transform: None,
                footprint: None,
                availability: SurveyMapSpatialAvailabilityDto::Unavailable,
                notes: vec![String::from(
                    "projected seismic survey geometry is not yet materialized from ingest metadata",
                )],
            });
        let transform_cache_dir = project_map_transform_cache_dir(&self.root);
        let mut notes = Vec::new();
        let (display_spatial, transform_status, transform_diagnostics) =
            resolve_display_spatial_descriptor(
                Some(&transform_cache_dir),
                &asset.id.0,
                &metadata.descriptor.geometry.fingerprint,
                coordinate_reference_binding.as_ref(),
                &native_spatial,
                display_coordinate_reference_id,
                &mut notes,
            );

        Ok(ResolvedSurveyMapSurveyDto {
            asset_id: asset.id.0.clone(),
            logical_asset_id: asset.logical_asset_id.0.clone(),
            name: survey_name,
            index_grid: SurveyIndexGridDto {
                inline_axis: SurveyIndexAxisDto {
                    count: inline_axis.count,
                    first: inline_axis.first,
                    last: inline_axis.last,
                    step: inline_axis.step,
                    regular: inline_axis.regular,
                },
                xline_axis: SurveyIndexAxisDto {
                    count: xline_axis.count,
                    first: xline_axis.first,
                    last: xline_axis.last,
                    step: xline_axis.step,
                    regular: xline_axis.regular,
                },
            },
            coordinate_reference_binding,
            native_spatial,
            display_spatial,
            transform_status,
            transform_diagnostics,
            notes,
        })
    }

    fn resolve_survey_map_well(
        &self,
        wellbore_id: &WellboreId,
        display_coordinate_reference_id: Option<&str>,
    ) -> Result<ResolvedSurveyMapWellDto> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        let resolved_trajectory = self.resolve_wellbore_trajectory(wellbore_id)?;
        let current_assets = self
            .asset_summaries(wellbore_id, Some(AssetKind::Trajectory))?
            .into_iter()
            .filter(|summary| summary.is_current)
            .collect::<Vec<_>>();

        let mut trajectories = Vec::new();
        let mut notes = resolved_trajectory.notes.clone();

        for summary in current_assets {
            let asset = summary.asset;
            let collection = self.collection_by_id(&asset.collection_id)?;

            let rows = self.read_trajectory_rows(&asset.id, None)?;
            if rows.is_empty() {
                continue;
            }
            trajectories.push(SurveyMapTrajectoryDto {
                asset_id: asset.id.0.clone(),
                logical_asset_id: asset.logical_asset_id.0.clone(),
                asset_name: collection.name,
                rows: rows
                    .into_iter()
                    .map(|row| SurveyMapTrajectoryStationDto {
                        measured_depth: row.measured_depth,
                        true_vertical_depth: row.true_vertical_depth,
                        true_vertical_depth_subsea: row.true_vertical_depth_subsea,
                        azimuth_deg: row.azimuth_deg,
                        inclination_deg: row.inclination_deg,
                        northing_offset: row.northing_offset,
                        easting_offset: row.easting_offset,
                    })
                    .collect(),
            });
        }

        if trajectories.is_empty() {
            notes.push(String::from(
                "no current trajectory assets are available for this wellbore",
            ));
        }
        let (
            coordinate_reference,
            transform_status,
            transform_diagnostics,
            surface_location,
            plan_trajectory,
        ) = resolve_survey_map_well_geometry(
            &resolved_trajectory,
            display_coordinate_reference_id,
            &mut notes,
        )?;

        Ok(ResolvedSurveyMapWellDto {
            well_id: wellbore.well_id.0.clone(),
            wellbore_id: wellbore.id.0.clone(),
            name: wellbore.name,
            coordinate_reference,
            transform_status,
            transform_diagnostics,
            surface_location,
            plan_trajectory,
            trajectories,
            notes,
        })
    }

    fn resolve_single_section_well_overlay(
        &self,
        wellbore_id: &WellboreId,
        survey: &ResolvedSurveyMapSurveyDto,
        grid_transform: &SurveyMapGridTransformDto,
        section_axis: &SectionAxisSpec,
        tolerance_m: f64,
        request: &SectionWellOverlayRequestDto,
    ) -> Result<ResolvedSectionWellOverlayDto> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        let resolved_trajectory = self.resolve_wellbore_trajectory(wellbore_id)?;
        let mut diagnostics = resolved_trajectory.notes.clone();
        diagnostics.push(format!(
            "section overlay projected against survey '{}'",
            survey.name
        ));
        diagnostics.push(format!("section tolerance is {:.3} m", tolerance_m));
        let densification = section_densification_settings(grid_transform, tolerance_m);
        let densified_stations =
            densify_trajectory_for_section(&resolved_trajectory.stations, densification);
        if densified_stations.len() > resolved_trajectory.stations.len() {
            diagnostics.push(format!(
                "trajectory densified from {} to {} stations before section projection",
                resolved_trajectory.stations.len(),
                densified_stations.len()
            ));
        }

        if request.display_domain == SectionWellOverlayDomainDto::Time {
            return self.resolve_time_section_well_overlay(
                wellbore,
                &densified_stations,
                grid_transform,
                section_axis,
                tolerance_m,
                request,
                diagnostics,
            );
        }

        let mut segments = Vec::new();
        let mut current_samples = Vec::new();
        let mut current_notes = vec![format!(
            "depth overlay uses a densified trajectory polyline with max MD step {:.3} m, max XY chord {:.3} m, and max vertical chord {:.3} m",
            densification.max_md_step_m,
            densification.max_xy_step_m,
            densification.max_vertical_step_m
        )];
        let mut depth_reference = None;

        for station in &densified_stations {
            let Some(absolute_xy) = station.absolute_xy.as_ref() else {
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because a resolved trajectory station has no absolute XY",
                )];
                continue;
            };

            let Some(projected) = project_well_station_onto_section(
                station,
                absolute_xy,
                grid_transform,
                section_axis,
                tolerance_m,
            ) else {
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because the resolved trajectory moved outside the section tolerance ribbon",
                )];
                continue;
            };

            let Some(sample_value) = projected.sample_value else {
                diagnostics.push(format!(
                    "station at measured depth {:.3} m has no TVD/TVDSS depth, so it was omitted from the depth overlay",
                    station.measured_depth_m
                ));
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because a resolved trajectory station has no depth-domain value",
                )];
                continue;
            };

            if depth_reference.is_none() {
                depth_reference = if station.true_vertical_depth_m.is_some() {
                    Some(DepthReferenceKind::TrueVerticalDepth)
                } else if station.true_vertical_depth_subsea_m.is_some() {
                    Some(DepthReferenceKind::TrueVerticalDepthSubsea)
                } else {
                    None
                };
            }
            current_samples.push(SectionWellOverlaySampleDto {
                trace_index: projected.trace_index,
                trace_coordinate: projected.trace_coordinate,
                sample_index: None,
                sample_value: Some(sample_value),
                x: absolute_xy.x,
                y: absolute_xy.y,
                measured_depth_m: station.measured_depth_m,
                true_vertical_depth_m: station.true_vertical_depth_m,
                true_vertical_depth_subsea_m: station.true_vertical_depth_subsea_m,
                twt_ms: None,
            });
        }

        if !current_samples.is_empty() {
            segments.push(SectionWellOverlaySegmentDto {
                samples: current_samples,
                notes: current_notes,
            });
        }

        if segments.is_empty() {
            diagnostics.push(String::from(
                "no resolved trajectory stations fell within the requested section tolerance and depth-domain coverage",
            ));
        }

        Ok(ResolvedSectionWellOverlayDto {
            well_id: wellbore.well_id.0,
            wellbore_id: wellbore.id.0,
            name: wellbore.name,
            display_domain: request.display_domain,
            segments,
            diagnostics,
            active_model_id: None,
            depth_reference,
            travel_time_reference: None,
        })
    }

    fn resolve_time_section_well_overlay(
        &self,
        wellbore: WellboreRecord,
        densified_stations: &[ResolvedTrajectoryStation],
        grid_transform: &SurveyMapGridTransformDto,
        section_axis: &SectionAxisSpec,
        tolerance_m: f64,
        request: &SectionWellOverlayRequestDto,
        mut diagnostics: Vec<String>,
    ) -> Result<ResolvedSectionWellOverlayDto> {
        let Some((active_model_id, model)) = self
            .resolve_active_well_time_depth_model(&wellbore.id, &request.active_well_model_ids)?
        else {
            diagnostics.push(String::from(
                "time-domain section overlay requires an active compiled well time-depth model for this wellbore",
            ));
            return Ok(ResolvedSectionWellOverlayDto {
                well_id: wellbore.well_id.0,
                wellbore_id: wellbore.id.0,
                name: wellbore.name,
                display_domain: request.display_domain,
                segments: Vec::new(),
                diagnostics,
                active_model_id: request.active_well_model_ids.first().cloned(),
                depth_reference: None,
                travel_time_reference: None,
            });
        };

        let mut segments = Vec::new();
        let mut current_samples = Vec::new();
        let mut current_notes = vec![String::from(
            "time overlay uses the active compiled well time-depth model evaluated along the densified trajectory polyline",
        )];
        let display_reference = TravelTimeReference::TwoWay;
        if model.travel_time_reference == TravelTimeReference::OneWay {
            diagnostics.push(format!(
                "active well model '{}' is one-way time; section overlay converts it to two-way time for display",
                active_model_id
            ));
        }

        for station in densified_stations {
            let Some(absolute_xy) = station.absolute_xy.as_ref() else {
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because a resolved trajectory station has no absolute XY",
                )];
                continue;
            };

            let Some(projected) = project_well_station_onto_section(
                station,
                absolute_xy,
                grid_transform,
                section_axis,
                tolerance_m,
            ) else {
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because the resolved trajectory moved outside the section tolerance ribbon",
                )];
                continue;
            };

            let Some(depth_m) = depth_for_model(station, model.depth_reference) else {
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because a resolved trajectory station does not contain the depth reference required by the active well model",
                )];
                continue;
            };

            let Some(time_ms) = interpolate_well_time_depth_model_ms(&model, depth_m) else {
                if !current_samples.is_empty() {
                    segments.push(SectionWellOverlaySegmentDto {
                        samples: std::mem::take(&mut current_samples),
                        notes: current_notes.clone(),
                    });
                }
                current_notes = vec![String::from(
                    "segment was split because the active well model has no time coverage for that trajectory interval",
                )];
                continue;
            };

            let twt_ms = display_time_ms(time_ms, model.travel_time_reference, display_reference);
            current_samples.push(SectionWellOverlaySampleDto {
                trace_index: projected.trace_index,
                trace_coordinate: projected.trace_coordinate,
                sample_index: None,
                sample_value: Some(twt_ms),
                x: absolute_xy.x,
                y: absolute_xy.y,
                measured_depth_m: station.measured_depth_m,
                true_vertical_depth_m: station.true_vertical_depth_m,
                true_vertical_depth_subsea_m: station.true_vertical_depth_subsea_m,
                twt_ms: Some(twt_ms),
            });
        }

        if !current_samples.is_empty() {
            segments.push(SectionWellOverlaySegmentDto {
                samples: current_samples,
                notes: current_notes,
            });
        }

        if segments.is_empty() {
            diagnostics.push(String::from(
                "no resolved trajectory stations fell within both the section tolerance ribbon and the active well-model time coverage",
            ));
        }

        Ok(ResolvedSectionWellOverlayDto {
            well_id: wellbore.well_id.0,
            wellbore_id: wellbore.id.0,
            name: wellbore.name,
            display_domain: request.display_domain,
            segments,
            diagnostics,
            active_model_id: Some(active_model_id),
            depth_reference: Some(model.depth_reference),
            travel_time_reference: Some(display_reference),
        })
    }

    fn resolve_active_well_time_depth_model(
        &self,
        wellbore_id: &WellboreId,
        requested_ids: &[String],
    ) -> Result<Option<(String, WellTimeDepthModel1D)>> {
        for requested_id in requested_ids {
            let asset_id = AssetId(requested_id.clone());
            let Ok(model) = self.read_well_time_depth_model(&asset_id) else {
                continue;
            };
            if model
                .wellbore_id
                .as_deref()
                .is_some_and(|model_wellbore_id| model_wellbore_id != wellbore_id.0)
            {
                continue;
            }
            return Ok(Some((requested_id.clone(), model)));
        }
        if let Some(active_asset_id) = self
            .wellbore_by_id(wellbore_id)?
            .active_well_time_depth_model_asset_id
        {
            let model = self.read_well_time_depth_model(&active_asset_id)?;
            if model
                .wellbore_id
                .as_deref()
                .is_none_or(|model_wellbore_id| model_wellbore_id == wellbore_id.0)
            {
                return Ok(Some((active_asset_id.0, model)));
            }
        }
        Ok(None)
    }

    pub(crate) fn preview_horizon_source_import_for_survey_asset(
        &self,
        survey_asset_id: &AssetId,
        draft: Option<&HorizonSourceImportCanonicalDraft>,
    ) -> Result<ophiolite_seismic_runtime::HorizonSourceImportPreview> {
        let survey_asset = self.asset_by_id(survey_asset_id)?;
        require_asset_kind(&survey_asset, AssetKind::SeismicTraceData)?;
        let store_root = Path::new(&survey_asset.package_path).join("store");
        let source_paths = draft
            .map(|value| {
                value
                    .selected_source_paths
                    .iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        preview_horizon_source_import(&store_root, source_paths.as_slice(), draft).map_err(
            |error| {
                LasError::Storage(format!(
                    "failed to preview survey-horizon import for asset '{}': {error}",
                    survey_asset_id.0
                ))
            },
        )
    }

    pub(crate) fn import_horizon_source_into_survey_asset(
        &mut self,
        survey_asset_id: &AssetId,
        draft: &HorizonSourceImportCanonicalDraft,
        supporting_source_paths: &[PathBuf],
    ) -> Result<SurveyHorizonImportResult> {
        if draft.selected_source_paths.is_empty() {
            return Err(LasError::Validation(
                "survey-horizon import requires at least one source path".to_string(),
            ));
        }

        let mut asset = self.asset_by_id(survey_asset_id)?;
        require_asset_kind(&asset, AssetKind::SeismicTraceData)?;
        let collection = self.collection_by_id(&asset.collection_id)?;
        if self.collection_owner_binding(&collection.id)?.scope != AssetOwnerScope::Survey {
            self.bind_collection_owner(
                &collection.id,
                &AssetOwnerBinding {
                    scope: AssetOwnerScope::Survey,
                    owner_id: collection.logical_asset_id.0.clone(),
                    display_name: collection.name.clone(),
                },
            )?;
        }

        let parent_revision = self
            .current_asset_revision(survey_asset_id)?
            .map(|item| item.revision_id);
        let staged = stage_existing_asset_root(&self.root, &asset)?;
        let staged_store_root = staged.root.join("store");
        let imported_horizons =
            import_horizon_xyzs_from_draft(&staged_store_root, draft).map_err(|error| {
                LasError::Storage(format!(
                    "failed to import horizons into survey asset '{}': {error}",
                    survey_asset_id.0
                ))
            })?;

        let import_source_paths = draft
            .selected_source_paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let source_paths = if supporting_source_paths.is_empty() {
            import_source_paths
        } else {
            supporting_source_paths.to_vec()
        };
        let source_path_refs = source_paths
            .iter()
            .map(PathBuf::as_path)
            .collect::<Vec<_>>();
        let supporting_descriptors =
            copy_supporting_source_artifacts(&staged.root, source_path_refs.as_slice())?;
        if !supporting_descriptors.is_empty() {
            asset
                .manifest
                .bulk_data_descriptors
                .extend(supporting_descriptors);
            asset.manifest.bulk_data_descriptors.sort_by(|left, right| {
                left.relative_path
                    .cmp(&right.relative_path)
                    .then_with(|| left.role.cmp(&right.role))
            });
            asset
                .manifest
                .bulk_data_descriptors
                .dedup_by(|left, right| {
                    left.relative_path == right.relative_path && left.role == right.role
                });
        }

        let imported_at_unix_seconds = now_unix_seconds();
        asset.manifest.imported_at_unix_seconds = imported_at_unix_seconds;
        asset
            .manifest
            .source_artifacts
            .extend(source_artifact_refs_for_paths(
                source_path_refs.as_slice(),
                imported_at_unix_seconds,
            )?);
        asset.manifest.source_artifacts.sort_by(|left, right| {
            left.source_path
                .cmp(&right.source_path)
                .then_with(|| left.original_filename.cmp(&right.original_filename))
        });
        asset
            .manifest
            .source_artifacts
            .dedup_by(|left, right| left.source_path == right.source_path);

        write_asset_manifest(&staged.root, &asset.manifest)?;
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            parent_revision.as_ref(),
            AssetDiffSummary::SeismicTraceData(DirectoryAssetDiffSummary {
                entry_count_changed: true,
                changed_path_count: imported_horizons.len().max(1),
            }),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.update_asset_manifest(&asset)?;

        Ok(SurveyHorizonImportResult {
            collection,
            asset,
            imported_horizons,
        })
    }

    fn import_structured_asset<F>(
        &mut self,
        source_path: &Path,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        collection_name: Option<&str>,
        writer: F,
        metadata: AssetTableMetadata,
        extent: AssetExtent,
    ) -> Result<ProjectAssetImportResult>
    where
        F: FnOnce(&Path) -> Result<()>,
    {
        self.import_structured_asset_with_supporting_sources(
            source_path,
            binding,
            asset_kind,
            collection_name,
            writer,
            metadata,
            extent,
            None,
            None,
            &[],
        )
    }

    fn import_structured_asset_with_supporting_sources<F>(
        &mut self,
        source_path: &Path,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        collection_name: Option<&str>,
        writer: F,
        metadata: AssetTableMetadata,
        extent: AssetExtent,
        coordinate_reference: Option<CoordinateReference>,
        reference_metadata_override: Option<(DepthReference, Option<VerticalDatum>)>,
        supporting_source_paths: &[&Path],
    ) -> Result<ProjectAssetImportResult>
    where
        F: FnOnce(&Path) -> Result<()>,
    {
        let identifiers = identifiers_from_binding(binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                source_path
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| asset_kind.as_str().to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, asset_kind.clone(), &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        writer(&staged.root)?;
        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let mut manifest = structured_asset_manifest(
            source_path,
            &metadata,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            asset_kind.clone(),
            extent,
            identifiers.clone(),
            coordinate_reference,
            supersedes.clone(),
        )?;
        if let Some((depth_reference, vertical_datum)) = reference_metadata_override {
            manifest.reference_metadata.depth_reference = depth_reference;
            manifest.reference_metadata.vertical_datum = vertical_datum;
        }
        let supporting_descriptors =
            copy_supporting_source_artifacts(&staged.root, supporting_source_paths)?;
        if !supporting_descriptors.is_empty() {
            manifest
                .bulk_data_descriptors
                .extend(supporting_descriptors);
            manifest
                .source_artifacts
                .extend(source_artifact_refs_for_paths(
                    supporting_source_paths,
                    manifest.imported_at_unix_seconds,
                )?);
        }
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        let refreshed_wellbore = self.wellbore_by_id(&wellbore.id)?;
        if matches!(asset.asset_kind, AssetKind::Trajectory)
            && refreshed_wellbore.definitive_trajectory_asset_id.is_none()
        {
            self.set_definitive_trajectory_asset(&wellbore.id, Some(&asset.id))?;
        }
        if matches!(asset.asset_kind, AssetKind::TopSet)
            && refreshed_wellbore.definitive_top_set_asset_id.is_none()
        {
            self.set_definitive_top_set_asset(&wellbore.id, Some(&asset.id))?;
        }
        if matches!(asset.asset_kind, AssetKind::WellMarkerSet)
            && refreshed_wellbore.definitive_marker_set_asset_id.is_none()
        {
            self.set_definitive_marker_set_asset(&wellbore.id, Some(&asset.id))?;
        }
        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    fn import_seismic_asset(
        &mut self,
        store_root: &Path,
        source_path: &Path,
        binding: &AssetBindingInput,
        asset_kind: AssetKind,
        collection_name: Option<&str>,
        metadata: &SeismicAssetMetadata,
    ) -> Result<SeismicAssetImportResult> {
        let identifiers = identifiers_from_binding(binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, binding)?;
        let collection_name = collection_name
            .map(str::to_owned)
            .or_else(|| {
                source_path
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| asset_kind.as_str().to_string());
        let collection =
            self.resolve_or_create_collection(&wellbore.id, asset_kind.clone(), &collection_name)?;
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(asset_kind.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = self.root.join(&package_rel_path);
        let staged = stage_project_asset_root(&self.root, &storage_asset_id)?;
        copy_path(store_root, &staged.root.join("store"))?;
        fs::write(
            staged.root.join("metadata.json"),
            serde_json::to_vec_pretty(metadata)?,
        )?;

        let supersedes = self
            .latest_active_asset_for_collection(&collection.id)?
            .map(|asset| asset.id);
        let manifest = seismic_asset_manifest(
            source_path,
            metadata,
            &well.id,
            &wellbore.id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            asset_kind.clone(),
            identifiers.clone(),
            supersedes.clone(),
        )?;
        write_asset_manifest(&staged.root, &manifest)?;
        if let Some(asset_id) = &supersedes {
            self.mark_asset_superseded(asset_id)?;
        }

        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: well.id.clone(),
            wellbore_id: wellbore.id.clone(),
            asset_kind,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        let revision = self.build_asset_revision_from_snapshot(
            &asset,
            None,
            default_asset_diff_summary(&asset.asset_kind),
            &staged,
        )?;
        self.commit_asset_revision(&asset, &revision)?;
        self.insert_asset(&asset, &package_rel_path)?;
        Ok(ProjectAssetImportResult {
            resolution: ImportResolution {
                status: AssetStatus::Bound,
                well_id: well.id,
                wellbore_id: wellbore.id,
                created_well,
                created_wellbore,
            },
            collection,
            asset,
        })
    }

    pub fn asset_by_id(&self, asset_id: &AssetId) -> Result<AssetRecord> {
        self.connection
            .query_row(
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE id = ?1",
                params![asset_id.0],
                |row| {
                    let manifest = serde_json::from_str::<AssetManifest>(&row.get::<_, String>(8)?)
                        .map_err(sql_json_error)?;
                    Ok(AssetRecord {
                        id: AssetId(row.get(0)?),
                        logical_asset_id: AssetId(row.get(1)?),
                        collection_id: AssetCollectionId(row.get(2)?),
                        well_id: WellId(row.get(3)?),
                        wellbore_id: WellboreId(row.get(4)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                        status: AssetStatus::from_str(&row.get::<_, String>(6)?)
                            .map_err(sql_validation_error)?,
                        package_path: self
                            .root
                            .join(row.get::<_, String>(7)?)
                            .to_string_lossy()
                            .into_owned(),
                        manifest,
                    })
                },
            )
            .map_err(sqlite_error)
    }

    fn collection_by_id(&self, collection_id: &AssetCollectionId) -> Result<AssetCollectionRecord> {
        self.connection
            .query_row(
                "SELECT id, wellbore_id, asset_kind, name, logical_asset_id, status
                 FROM asset_collections
                 WHERE id = ?1",
                params![collection_id.0],
                |row| {
                    Ok(AssetCollectionRecord {
                        id: AssetCollectionId(row.get(0)?),
                        wellbore_id: WellboreId(row.get(1)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(2)?)
                            .map_err(sql_validation_error)?,
                        name: row.get(3)?,
                        logical_asset_id: AssetId(row.get(4)?),
                        status: AssetStatus::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                    })
                },
            )
            .map_err(sqlite_error)
    }

    fn well_by_id(&self, well_id: &WellId) -> Result<WellRecord> {
        self.connection
            .query_row(
                "SELECT id, primary_name, identifiers_json, metadata_json
                 FROM wells
                 WHERE id = ?1",
                params![well_id.0],
                well_record_from_row,
            )
            .map_err(sqlite_error)
    }

    fn wellbore_by_id(&self, wellbore_id: &WellboreId) -> Result<WellboreRecord> {
        self.connection
            .query_row(
                "SELECT id, well_id, primary_name, identifiers_json, metadata_json, geometry_json, definitive_trajectory_asset_id, definitive_marker_set_asset_id, definitive_top_set_asset_id, active_well_time_depth_model_asset_id
                 FROM wellbores
                 WHERE id = ?1",
                params![wellbore_id.0],
                wellbore_record_from_row,
            )
            .map_err(sqlite_error)
    }

    fn replace_well_markers(
        &self,
        wellbore_id: &WellboreId,
        markers: &[WellMarkerRecord],
    ) -> Result<()> {
        self.connection
            .execute(
                "DELETE FROM well_markers WHERE wellbore_id = ?1",
                params![wellbore_id.0],
            )
            .map_err(sqlite_error)?;
        for marker in markers {
            self.connection
                .execute(
                    "INSERT INTO well_markers (id, wellbore_id, source_asset_id, sequence_number, name, marker_json, created_at_unix_seconds)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        marker.id.0.clone(),
                        marker.wellbore_id.0.clone(),
                        marker.source_asset_id.as_ref().map(|value| value.0.clone()),
                        marker.sequence_number.map(i64::from),
                        marker.name.clone(),
                        serde_json::to_string(marker)?,
                        now_unix_seconds() as i64,
                    ],
                )
                .map_err(sqlite_error)?;
        }
        Ok(())
    }

    fn sync_well_markers_from_top_set_asset(
        &self,
        wellbore_id: &WellboreId,
        asset_id: &AssetId,
    ) -> Result<()> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::TopSet)?;
        if asset.wellbore_id != *wellbore_id {
            return Err(LasError::Validation(format!(
                "top set asset '{}' does not belong to wellbore '{}'",
                asset_id.0, wellbore_id.0
            )));
        }
        let rows = self.read_tops(asset_id)?;
        let markers = canonical_markers_from_top_rows(wellbore_id, asset_id, &rows);
        self.replace_well_markers(wellbore_id, &markers)
    }

    fn sync_well_markers_from_marker_set_asset(
        &self,
        wellbore_id: &WellboreId,
        asset_id: &AssetId,
    ) -> Result<()> {
        let asset = self.asset_by_id(asset_id)?;
        require_asset_kind(&asset, AssetKind::WellMarkerSet)?;
        if asset.wellbore_id != *wellbore_id {
            return Err(LasError::Validation(format!(
                "well marker set asset '{}' does not belong to wellbore '{}'",
                asset_id.0, wellbore_id.0
            )));
        }
        let rows = self.read_well_marker_rows(asset_id)?;
        let markers = canonical_markers_from_well_marker_rows(wellbore_id, asset_id, &rows);
        self.replace_well_markers(wellbore_id, &markers)
    }

    fn resolve_or_create_well(
        &self,
        identifiers: &WellIdentifierSet,
    ) -> Result<(WellRecord, bool)> {
        if let Some(well) = self.find_matching_well(identifiers)? {
            return Ok((well, false));
        }

        let well = WellRecord {
            id: WellId(unique_id("well")),
            name: identifiers
                .primary_name
                .clone()
                .unwrap_or_else(|| "Unknown Well".to_string()),
            identifiers: identifiers.clone(),
            metadata: None,
        };
        self.connection.execute(
            "INSERT INTO wells (id, primary_name, normalized_name, uwi, api, identifiers_json, metadata_json, created_at_unix_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                well.id.0,
                well.name,
                normalized_text(&well.name),
                optional_db_text(&well.identifiers.uwi),
                optional_db_text(&well.identifiers.api),
                serde_json::to_string(&well.identifiers)?,
                Option::<String>::None,
                now_unix_seconds() as i64,
            ],
        ).map_err(sqlite_error)?;
        Ok((well, true))
    }

    fn resolve_or_create_wellbore(
        &self,
        well_id: &WellId,
        identifiers: &WellIdentifierSet,
    ) -> Result<(WellboreRecord, bool)> {
        let wellbore_name = identifiers
            .primary_name
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let normalized = normalized_text(&wellbore_name);
        let existing = self
            .connection
            .query_row(
                "SELECT id, well_id, primary_name, identifiers_json, metadata_json, geometry_json, definitive_trajectory_asset_id, definitive_marker_set_asset_id, definitive_top_set_asset_id, active_well_time_depth_model_asset_id
                 FROM wellbores
                 WHERE well_id = ?1 AND normalized_name = ?2",
                params![well_id.0, normalized],
                wellbore_record_from_row,
            )
            .optional()
            .map_err(sqlite_error)?;
        if let Some(wellbore) = existing {
            return Ok((wellbore, false));
        }

        let wellbore = WellboreRecord {
            id: WellboreId(unique_id("wellbore")),
            well_id: well_id.clone(),
            name: wellbore_name,
            identifiers: identifiers.clone(),
            metadata: None,
            geometry: None,
            definitive_trajectory_asset_id: None,
            definitive_marker_set_asset_id: None,
            definitive_top_set_asset_id: None,
            active_well_time_depth_model_asset_id: None,
        };
        self.connection.execute(
            "INSERT INTO wellbores (id, well_id, primary_name, normalized_name, identifiers_json, metadata_json, geometry_json, definitive_trajectory_asset_id, definitive_marker_set_asset_id, definitive_top_set_asset_id, active_well_time_depth_model_asset_id, created_at_unix_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                wellbore.id.0,
                wellbore.well_id.0,
                wellbore.name,
                normalized_text(&wellbore.name),
                serde_json::to_string(&wellbore.identifiers)?,
                Option::<String>::None,
                Option::<String>::None,
                Option::<String>::None,
                Option::<String>::None,
                Option::<String>::None,
                Option::<String>::None,
                now_unix_seconds() as i64,
            ],
        ).map_err(sqlite_error)?;
        Ok((wellbore, true))
    }

    fn resolve_or_create_wellbore_for_binding(
        &self,
        well_id: &WellId,
        binding: &AssetBindingInput,
    ) -> Result<(WellboreRecord, bool)> {
        let identifiers = WellIdentifierSet {
            primary_name: Some(binding.wellbore_name.clone()),
            uwi: binding.uwi.clone(),
            api: binding.api.clone(),
            operator_aliases: binding.operator_aliases.clone(),
        };
        self.resolve_or_create_wellbore(well_id, &identifiers)
    }

    fn resolve_or_create_project_archive_wellbore(
        &self,
    ) -> Result<(WellRecord, WellboreRecord, bool, bool)> {
        let archive_identifiers = WellIdentifierSet {
            primary_name: Some(String::from(PROJECT_ARCHIVE_WELL_NAME)),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        };
        let (well, created_well) = self.resolve_or_create_well(&archive_identifiers)?;
        if created_well || well.metadata.is_none() {
            self.set_well_metadata(
                &well.id,
                Some(WellMetadata {
                    notes: vec![String::from(PROJECT_ARCHIVE_NOTE)],
                    ..Default::default()
                }),
            )?;
        }

        let archive_wellbore_identifiers = WellIdentifierSet {
            primary_name: Some(String::from(PROJECT_ARCHIVE_WELLBORE_NAME)),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        };
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore(&well.id, &archive_wellbore_identifiers)?;
        if created_wellbore || wellbore.metadata.is_none() {
            self.set_wellbore_metadata(
                &wellbore.id,
                Some(WellboreMetadata {
                    purpose: Some(String::from("archive")),
                    notes: vec![String::from(PROJECT_ARCHIVE_NOTE)],
                    ..Default::default()
                }),
            )?;
        }

        Ok((
            self.well_by_id(&well.id)?,
            self.wellbore_by_id(&wellbore.id)?,
            created_well,
            created_wellbore,
        ))
    }

    fn resolve_asset_owner_for_binding(
        &self,
        binding: &AssetBindingInput,
    ) -> Result<AssetOwnerHandle> {
        let identifiers = identifiers_from_binding(binding);
        let (well, created_well) = self.resolve_or_create_well(&identifiers)?;
        let (wellbore, created_wellbore) =
            self.resolve_or_create_wellbore_for_binding(&well.id, binding)?;
        Ok(AssetOwnerHandle {
            well,
            wellbore,
            identifiers,
            created_well,
            created_wellbore,
            catalog_owner: None,
        })
    }

    fn resolve_project_archive_asset_owner(&self) -> Result<AssetOwnerHandle> {
        let (well, wellbore, created_well, created_wellbore) =
            self.resolve_or_create_project_archive_wellbore()?;
        Ok(AssetOwnerHandle {
            identifiers: well.identifiers.clone(),
            well,
            wellbore,
            created_well,
            created_wellbore,
            catalog_owner: Some(AssetOwnerBinding {
                scope: AssetOwnerScope::Project,
                owner_id: String::from(PROJECT_ARCHIVE_OWNER_ID),
                display_name: String::from(PROJECT_ARCHIVE_OWNER_NAME),
            }),
        })
    }

    fn infer_wellbore_owner_binding(&self, wellbore_id: &WellboreId) -> Result<AssetOwnerBinding> {
        let wellbore = self.wellbore_by_id(wellbore_id)?;
        Ok(AssetOwnerBinding {
            scope: AssetOwnerScope::Wellbore,
            owner_id: wellbore.id.0.clone(),
            display_name: wellbore.name,
        })
    }

    fn upsert_asset_owner(&self, owner: &AssetOwnerBinding) -> Result<()> {
        self.connection
            .execute(
                "INSERT INTO asset_owners (scope, owner_id, display_name, metadata_json, created_at_unix_seconds)
                 VALUES (?1, ?2, ?3, NULL, ?4)
                 ON CONFLICT(scope, owner_id) DO UPDATE SET display_name = excluded.display_name",
                params![
                    owner.scope.as_str(),
                    owner.owner_id,
                    owner.display_name,
                    now_unix_seconds() as i64,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn bind_collection_owner(
        &self,
        collection_id: &AssetCollectionId,
        owner: &AssetOwnerBinding,
    ) -> Result<()> {
        self.upsert_asset_owner(owner)?;
        self.connection
            .execute(
                "INSERT INTO asset_collection_owners (collection_id, owner_scope, owner_id, created_at_unix_seconds)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(collection_id) DO UPDATE SET owner_scope = excluded.owner_scope, owner_id = excluded.owner_id",
                params![
                    collection_id.0,
                    owner.scope.as_str(),
                    owner.owner_id,
                    now_unix_seconds() as i64,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn collection_owner_binding(
        &self,
        collection_id: &AssetCollectionId,
    ) -> Result<AssetOwnerBinding> {
        let owner = self
            .connection
            .query_row(
                "SELECT o.scope, o.owner_id, o.display_name
                 FROM asset_collection_owners c
                 JOIN asset_owners o
                   ON o.scope = c.owner_scope AND o.owner_id = c.owner_id
                 WHERE c.collection_id = ?1",
                params![collection_id.0],
                |row| {
                    let scope = AssetOwnerScope::from_str(&row.get::<_, String>(0)?)
                        .map_err(sql_validation_error)?;
                    Ok(AssetOwnerBinding {
                        scope,
                        owner_id: row.get(1)?,
                        display_name: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        if let Some(owner) = owner {
            return Ok(owner);
        }

        let collection = self.collection_by_id(collection_id)?;
        self.infer_wellbore_owner_binding(&collection.wellbore_id)
    }

    fn resolve_or_create_collection(
        &self,
        wellbore_id: &WellboreId,
        asset_kind: AssetKind,
        name: &str,
    ) -> Result<AssetCollectionRecord> {
        let existing = self
            .connection
            .query_row(
                "SELECT id, wellbore_id, asset_kind, name, logical_asset_id, status
                 FROM asset_collections
                 WHERE wellbore_id = ?1 AND asset_kind = ?2 AND name = ?3",
                params![wellbore_id.0, asset_kind.as_str(), name],
                |row| {
                    Ok(AssetCollectionRecord {
                        id: AssetCollectionId(row.get(0)?),
                        wellbore_id: WellboreId(row.get(1)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(2)?)
                            .map_err(sql_validation_error)?,
                        name: row.get(3)?,
                        logical_asset_id: AssetId(row.get(4)?),
                        status: AssetStatus::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        if let Some(collection) = existing {
            return Ok(collection);
        }

        let collection = AssetCollectionRecord {
            id: AssetCollectionId(unique_id("collection")),
            wellbore_id: wellbore_id.clone(),
            asset_kind: asset_kind.clone(),
            name: name.to_string(),
            logical_asset_id: AssetId(unique_id("logical")),
            status: AssetStatus::Bound,
        };
        self.connection.execute(
            "INSERT INTO asset_collections (id, wellbore_id, asset_kind, name, logical_asset_id, status, created_at_unix_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                collection.id.0,
                collection.wellbore_id.0,
                collection.asset_kind.as_str(),
                collection.name,
                collection.logical_asset_id.0,
                collection.status.as_str(),
                now_unix_seconds() as i64,
            ],
        ).map_err(sqlite_error)?;
        Ok(collection)
    }

    fn latest_active_asset_for_collection(
        &self,
        collection_id: &AssetCollectionId,
    ) -> Result<Option<AssetRecord>> {
        let row = self
            .connection
            .query_row(
                "SELECT id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json
                 FROM assets
                 WHERE collection_id = ?1 AND status != 'superseded'
                 ORDER BY created_at_unix_seconds DESC
                 LIMIT 1",
                params![collection_id.0],
                |row| {
                    let manifest = serde_json::from_str::<AssetManifest>(&row.get::<_, String>(8)?)
                        .map_err(sql_json_error)?;
                    Ok(AssetRecord {
                        id: AssetId(row.get(0)?),
                        logical_asset_id: AssetId(row.get(1)?),
                        collection_id: AssetCollectionId(row.get(2)?),
                        well_id: WellId(row.get(3)?),
                        wellbore_id: WellboreId(row.get(4)?),
                        asset_kind: AssetKind::from_str(&row.get::<_, String>(5)?)
                            .map_err(sql_validation_error)?,
                        status: AssetStatus::from_str(&row.get::<_, String>(6)?)
                            .map_err(sql_validation_error)?,
                        package_path: self.root.join(row.get::<_, String>(7)?).to_string_lossy().into_owned(),
                        manifest,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        Ok(row)
    }

    fn mark_asset_superseded(&self, asset_id: &AssetId) -> Result<()> {
        self.connection
            .execute(
                "UPDATE assets SET status = 'superseded' WHERE id = ?1",
                params![asset_id.0],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn insert_asset(&self, asset: &AssetRecord, package_rel_path: &Path) -> Result<()> {
        self.connection.execute(
            "INSERT INTO assets
             (id, logical_asset_id, collection_id, well_id, wellbore_id, asset_kind, status, package_rel_path, manifest_json, created_at_unix_seconds, source_path, source_fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                asset.id.0,
                asset.logical_asset_id.0,
                asset.collection_id.0,
                asset.well_id.0,
                asset.wellbore_id.0,
                asset.asset_kind.as_str(),
                asset.status.as_str(),
                package_rel_path.to_string_lossy().to_string(),
                serde_json::to_string(&asset.manifest)?,
                now_unix_seconds() as i64,
                asset.manifest.provenance.source_path.clone(),
                asset.manifest.provenance.source_fingerprint.clone(),
            ],
        ).map_err(sqlite_error)?;
        Ok(())
    }

    fn update_asset_manifest(&self, asset: &AssetRecord) -> Result<()> {
        let package_rel_path = Path::new(&asset.package_path)
            .strip_prefix(&self.root)
            .map_err(|_| {
                LasError::Storage(format!(
                    "asset '{}' does not live under project root '{}'",
                    asset.id.0,
                    self.root.display()
                ))
            })?
            .to_string_lossy()
            .to_string();
        self.connection
            .execute(
                "UPDATE assets
                 SET manifest_json = ?2, package_rel_path = ?3, source_path = ?4, source_fingerprint = ?5
                 WHERE id = ?1",
                params![
                    asset.id.0,
                    serde_json::to_string(&asset.manifest)?,
                    package_rel_path,
                    asset.manifest.provenance.source_path.clone(),
                    asset.manifest.provenance.source_fingerprint.clone(),
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn find_matching_well(&self, identifiers: &WellIdentifierSet) -> Result<Option<WellRecord>> {
        if let Some(uwi) = &identifiers.uwi {
            if let Some(well) = self
                .connection
                .query_row(
                    "SELECT id, primary_name, identifiers_json, metadata_json FROM wells WHERE uwi = ?1",
                    params![uwi],
                    well_record_from_row,
                )
                .optional()
                .map_err(sqlite_error)?
            {
                return Ok(Some(well));
            }
        }

        if let Some(api) = &identifiers.api {
            if let Some(well) = self
                .connection
                .query_row(
                    "SELECT id, primary_name, identifiers_json, metadata_json FROM wells WHERE api = ?1",
                    params![api],
                    well_record_from_row,
                )
                .optional()
                .map_err(sqlite_error)?
            {
                return Ok(Some(well));
            }
        }

        if let Some(name) = &identifiers.primary_name {
            if let Some(well) = self
                .connection
                .query_row(
                    "SELECT id, primary_name, identifiers_json, metadata_json FROM wells WHERE normalized_name = ?1",
                    params![normalized_text(name)],
                    well_record_from_row,
                )
                .optional()
                .map_err(sqlite_error)?
            {
                return Ok(Some(well));
            }
        }

        Ok(None)
    }

    fn insert_asset_revision(&self, revision: &AssetRevisionRecord) -> Result<()> {
        self.connection
            .execute(
                "INSERT INTO asset_revisions
                 (id, asset_id, logical_asset_id, asset_kind, parent_revision_id, revision_json, created_at_unix_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    revision.revision_id.0,
                    revision.asset_id.0,
                    revision.logical_asset_id.0,
                    revision.asset_kind.as_str(),
                    revision
                        .parent_revision_id
                        .as_ref()
                        .map(|value| value.0.clone()),
                    serde_json::to_string(revision)?,
                    revision.created_at_unix_seconds as i64,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn build_asset_revision_from_snapshot(
        &self,
        asset: &AssetRecord,
        parent_revision_id: Option<&AssetRevisionId>,
        diff_summary: AssetDiffSummary,
        staged_snapshot: &StagedAssetSnapshot,
    ) -> Result<AssetRevisionRecord> {
        let created_at_unix_seconds = now_unix_seconds();
        let manifest_path = staged_snapshot.root.join(ASSET_MANIFEST_FILENAME);
        let manifest_bytes = fs::read(&manifest_path)?;
        let metadata_blob = AssetBlobRef {
            relative_path: ASSET_MANIFEST_FILENAME.to_string(),
            media_type: "application/json".to_string(),
            byte_count: manifest_bytes.len() as u64,
            content_hash: stable_project_blob_hash("asset-manifest", &manifest_bytes),
        };
        let data_blob = asset_primary_blob_ref(&asset.manifest, &staged_snapshot.root)?;
        let revision_id = AssetRevisionId(
            revision_token_for_bytes(
                "asset-revision",
                &format!(
                    "{}:{}:{}",
                    metadata_blob.content_hash, data_blob.content_hash, created_at_unix_seconds
                ),
            )
            .0,
        );
        let snapshot_rel_path = project_asset_revision_store_rel_path(&asset.id, &revision_id);
        let snapshot_root = self.root.join(&snapshot_rel_path);
        if let Some(parent) = snapshot_root.parent() {
            fs::create_dir_all(parent)?;
        }
        if snapshot_root.exists() {
            fs::remove_dir_all(&snapshot_root)?;
        }
        fs::rename(&staged_snapshot.root, &snapshot_root)?;
        let change_summary = summarize_asset_diff(&asset.asset_kind, &diff_summary);
        let revision = AssetRevisionRecord {
            revision_id,
            asset_id: asset.id.clone(),
            logical_asset_id: asset.logical_asset_id.clone(),
            asset_kind: asset.asset_kind.clone(),
            parent_revision_id: parent_revision_id.cloned(),
            package_snapshot_rel_path: snapshot_rel_path.to_string_lossy().to_string(),
            created_at_unix_seconds,
            metadata_blob,
            data_blob,
            diff_summary,
            change_summary,
        };
        Ok(revision)
    }

    fn commit_asset_revision(
        &self,
        asset: &AssetRecord,
        revision: &AssetRevisionRecord,
    ) -> Result<()> {
        let previous_head = self.current_asset_revision(&asset.id)?;
        self.materialize_asset_head_from_revision(asset, revision)?;
        if let Err(error) = self.insert_asset_revision(revision) {
            if let Some(previous_revision) = previous_head.as_ref() {
                self.materialize_asset_head_from_revision(asset, previous_revision)?;
            } else {
                clear_project_visible_files(asset)?;
            }
            return Err(error);
        }
        Ok(())
    }

    fn materialize_asset_head_from_revision(
        &self,
        asset: &AssetRecord,
        revision: &AssetRevisionRecord,
    ) -> Result<()> {
        let revision_root = self.root.join(&revision.package_snapshot_rel_path);
        fs::create_dir_all(&asset.package_path)?;
        materialize_project_visible_files(
            &self.root,
            &artifact_mappings_for_asset(asset, &revision_root),
        )
    }
}

fn resolve_survey_map_well_geometry(
    resolved_trajectory: &ResolvedTrajectoryGeometry,
    display_coordinate_reference_id: Option<&str>,
    notes: &mut Vec<String>,
) -> Result<(
    Option<CoordinateReferenceDto>,
    SurveyMapTransformStatusDto,
    SurveyMapTransformDiagnosticsDto,
    Option<ProjectedPoint2Dto>,
    Vec<ProjectedPoint2Dto>,
)> {
    let mut coordinate_reference = resolved_trajectory
        .coordinate_reference
        .as_ref()
        .map(coordinate_reference_dto_from_seismic);
    let native_plan_trajectory = resolved_trajectory
        .stations
        .iter()
        .filter_map(|station| station.absolute_xy.as_ref())
        .map(projected_point_dto_from_seismic)
        .collect::<Vec<_>>();
    if native_plan_trajectory.len() < resolved_trajectory.stations.len() {
        notes.push(String::from(
            "trajectory stations without resolved absolute XY were omitted from the survey-map plan trajectory",
        ));
    }

    let request = SurveyMapDisplayTransformRequest::new(
        coordinate_reference.as_ref().and_then(|reference| {
            reference
                .id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        }),
        display_coordinate_reference_id,
    );
    let degraded = native_plan_trajectory.len() < resolved_trajectory.stations.len();

    let Some(display_coordinate_reference_id) = request.target_coordinate_reference_id() else {
        let surface_location = native_plan_trajectory.first().cloned();
        let (coordinate_reference, transform_status, transform_diagnostics) = request.resolution(
            coordinate_reference.clone(),
            SurveyMapTransformStatusDto::NativeOnly,
            None,
            None,
            None,
            degraded,
            Vec::new(),
        );
        return Ok((
            coordinate_reference,
            transform_status,
            transform_diagnostics,
            surface_location,
            native_plan_trajectory,
        ));
    };

    if !request.target_supported() {
        let note = format!(
            "display coordinate reference '{display_coordinate_reference_id}' is not yet supported for well-map reprojection; phase 2 currently accepts only EPSG identifiers"
        );
        notes.push(note.clone());
        let (coordinate_reference, transform_status, transform_diagnostics) = request.resolution(
            None,
            SurveyMapTransformStatusDto::DisplayUnavailable,
            None,
            None,
            None,
            false,
            vec![note],
        );
        return Ok((
            coordinate_reference,
            transform_status,
            transform_diagnostics,
            None,
            Vec::new(),
        ));
    }

    if native_plan_trajectory.is_empty() {
        let note = format!(
            "display coordinate reference '{display_coordinate_reference_id}' was requested but the well trajectory has no transformable absolute XY geometry"
        );
        notes.push(note.clone());
        let (coordinate_reference, transform_status, transform_diagnostics) = request.resolution(
            None,
            SurveyMapTransformStatusDto::DisplayUnavailable,
            None,
            None,
            None,
            false,
            vec![note],
        );
        return Ok((
            coordinate_reference,
            transform_status,
            transform_diagnostics,
            None,
            Vec::new(),
        ));
    }

    match request.source_coordinate_reference_id() {
        Some(_) if request.is_identity_transform() => {
            if let Some(reference) = coordinate_reference.as_mut() {
                reference.id = Some(display_coordinate_reference_id.to_string());
            } else {
                coordinate_reference = Some(CoordinateReferenceDto {
                    id: Some(display_coordinate_reference_id.to_string()),
                    name: None,
                    geodetic_datum: None,
                    unit: None,
                });
            }
            let surface_location = native_plan_trajectory.first().cloned();
            let (coordinate_reference, transform_status, transform_diagnostics) =
                request.resolution(
                    coordinate_reference,
                    if degraded {
                        SurveyMapTransformStatusDto::DisplayDegraded
                    } else {
                        SurveyMapTransformStatusDto::DisplayEquivalent
                    },
                    None,
                    Some("identity".to_string()),
                    Some(0.0),
                    degraded,
                    if degraded {
                        vec![String::from(
                            "display well-map geometry is partial because some stations have no absolute XY",
                        )]
                    } else {
                        Vec::new()
                    },
                );
            Ok((
                coordinate_reference,
                transform_status,
                transform_diagnostics,
                surface_location,
                native_plan_trajectory,
            ))
        }
        Some(source_coordinate_reference_id) => {
            if !request.source_supported() {
                let note = format!(
                    "well effective native CRS '{source_coordinate_reference_id}' is not yet supported for reprojection; phase 2 currently accepts only EPSG identifiers"
                );
                notes.push(note.clone());
                let (coordinate_reference, transform_status, transform_diagnostics) = request
                    .resolution(
                        None,
                        SurveyMapTransformStatusDto::DisplayUnavailable,
                        None,
                        None,
                        None,
                        false,
                        vec![note],
                    );
                return Ok((
                    coordinate_reference,
                    transform_status,
                    transform_diagnostics,
                    None,
                    Vec::new(),
                ));
            }

            let transformer = build_proj_transformer(
                source_coordinate_reference_id,
                display_coordinate_reference_id,
            )?;
            let display_plan_trajectory = native_plan_trajectory
                .iter()
                .map(|point| transform_point(&transformer, point))
                .collect::<Result<Vec<_>>>()?;
            notes.push(format!(
                "display well-map geometry was reprojected from {source_coordinate_reference_id} to {display_coordinate_reference_id}"
            ));
            let surface_location = display_plan_trajectory.first().cloned();
            let (coordinate_reference, transform_status, transform_diagnostics) =
                request.resolution(
                    Some(CoordinateReferenceDto {
                        id: Some(display_coordinate_reference_id.to_string()),
                        name: None,
                        geodetic_datum: None,
                        unit: None,
                    }),
                    if degraded {
                        SurveyMapTransformStatusDto::DisplayDegraded
                    } else {
                        SurveyMapTransformStatusDto::DisplayTransformed
                    },
                    Some("proj_crs_to_crs".to_string()),
                    Some(format!(
                        "proj_crs_to_crs:{source_coordinate_reference_id}->{display_coordinate_reference_id}"
                    )),
                    None,
                    degraded,
                    if degraded {
                        vec![format!(
                            "display well-map geometry was reprojected from {source_coordinate_reference_id} to {display_coordinate_reference_id}, but stations without absolute XY were omitted"
                        )]
                    } else {
                        vec![format!(
                            "display well-map geometry was reprojected from {source_coordinate_reference_id} to {display_coordinate_reference_id}"
                        )]
                    },
                );
            Ok((
                coordinate_reference,
                transform_status,
                transform_diagnostics,
                surface_location,
                display_plan_trajectory,
            ))
        }
        None => {
            let note = format!(
                "display coordinate reference '{display_coordinate_reference_id}' was requested but the well effective native CRS is unknown"
            );
            notes.push(note.clone());
            let (coordinate_reference, transform_status, transform_diagnostics) = request
                .resolution(
                    None,
                    SurveyMapTransformStatusDto::DisplayUnavailable,
                    None,
                    None,
                    None,
                    false,
                    vec![note],
                );
            Ok((
                coordinate_reference,
                transform_status,
                transform_diagnostics,
                None,
                Vec::new(),
            ))
        }
    }
}

pub fn resolve_dataset_summary_survey_map_source(
    dataset: &DatasetSummary,
    display_coordinate_reference_id: Option<&str>,
    cache_dir: Option<&Path>,
    store_root: Option<&Path>,
) -> Result<ResolvedSurveyMapSourceDto> {
    let inline_axis = &dataset.descriptor.geometry.summary.inline_axis;
    let xline_axis = &dataset.descriptor.geometry.summary.xline_axis;
    let coordinate_reference_binding = dataset
        .descriptor
        .coordinate_reference_binding
        .as_ref()
        .map(coordinate_reference_binding_dto_from_seismic);
    let native_spatial = dataset
        .descriptor
        .spatial
        .as_ref()
        .map(survey_spatial_descriptor_dto_from_seismic)
        .unwrap_or_else(|| SurveyMapSpatialDescriptorDto {
            coordinate_reference: None,
            grid_transform: None,
            footprint: None,
            availability: SurveyMapSpatialAvailabilityDto::Unavailable,
            notes: vec![String::from(
                "dataset does not expose native survey map geometry in its descriptor",
            )],
        });
    let mut notes = Vec::new();
    let dataset_id = dataset.descriptor.id.0.clone();
    let (display_spatial, transform_status, transform_diagnostics) =
        resolve_display_spatial_descriptor(
            cache_dir,
            &dataset_id,
            &dataset.descriptor.geometry.fingerprint,
            coordinate_reference_binding.as_ref(),
            &native_spatial,
            display_coordinate_reference_id,
            &mut notes,
        );

    let mut survey = ResolvedSurveyMapSurveyDto {
        asset_id: dataset_id.clone(),
        logical_asset_id: dataset_id.clone(),
        name: dataset.descriptor.label.clone(),
        index_grid: SurveyIndexGridDto {
            inline_axis: SurveyIndexAxisDto {
                count: inline_axis.count,
                first: inline_axis.first,
                last: inline_axis.last,
                step: inline_axis.step,
                regular: inline_axis.regular,
            },
            xline_axis: SurveyIndexAxisDto {
                count: xline_axis.count,
                first: xline_axis.first,
                last: xline_axis.last,
                step: xline_axis.step,
                regular: xline_axis.regular,
            },
        },
        coordinate_reference_binding,
        native_spatial,
        display_spatial,
        transform_status,
        transform_diagnostics,
        notes,
    };
    let mut horizons = Vec::new();
    let mut scalar_field = None;
    let mut scalar_field_horizon_id = None;
    if let Some(store_root) = store_root {
        match resolve_survey_map_horizons_for_store(
            &survey.asset_id,
            store_root,
            &survey,
            display_coordinate_reference_id,
        ) {
            Ok(resolved) => {
                horizons = resolved.horizons;
                scalar_field = resolved.scalar_field;
                scalar_field_horizon_id = resolved.scalar_field_horizon_id;
            }
            Err(error) => survey.notes.push(format!(
                "failed to resolve imported horizons for dataset '{}': {error}",
                survey.name
            )),
        }
    }

    Ok(ResolvedSurveyMapSourceDto {
        schema_version: SURVEY_MAP_CONTRACT_VERSION,
        id: format!("{dataset_id}-survey-map"),
        name: dataset.descriptor.label.clone(),
        surveys: vec![survey],
        wells: Vec::new(),
        horizons,
        scalar_field,
        scalar_field_horizon_id,
    })
}

const SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON: f64 = 1e-6;

struct ResolvedSurveyMapHorizonCollection {
    horizons: Vec<ResolvedSurveyMapHorizonDto>,
    scalar_field: Option<SurveyMapScalarFieldDto>,
    scalar_field_horizon_id: Option<String>,
}

struct ResolvedSurveyMapHorizonPreview {
    horizon: ResolvedSurveyMapHorizonDto,
    scalar_field: Option<SurveyMapScalarFieldDto>,
}

fn resolve_survey_map_horizons_for_store(
    survey_asset_id: &str,
    store_root: &Path,
    survey: &ResolvedSurveyMapSurveyDto,
    display_coordinate_reference_id: Option<&str>,
) -> Result<ResolvedSurveyMapHorizonCollection> {
    let imported = load_horizon_grids(store_root).map_err(|error| {
        LasError::Storage(format!(
            "failed to load imported horizons from '{}': {error}",
            store_root.display()
        ))
    })?;
    let mut horizons = Vec::with_capacity(imported.len());
    let mut scalar_field = None;
    let mut scalar_field_horizon_id = None;
    for horizon in imported {
        let resolved = resolve_survey_map_horizon_preview(
            survey_asset_id,
            survey,
            &horizon,
            display_coordinate_reference_id,
        );
        if scalar_field.is_none() {
            scalar_field = resolved.scalar_field;
            scalar_field_horizon_id = scalar_field.as_ref().map(|_| resolved.horizon.id.clone());
        }
        horizons.push(resolved.horizon);
    }
    Ok(ResolvedSurveyMapHorizonCollection {
        horizons,
        scalar_field,
        scalar_field_horizon_id,
    })
}

fn resolve_survey_map_horizon_preview(
    survey_asset_id: &str,
    survey: &ResolvedSurveyMapSurveyDto,
    horizon: &ImportedHorizonGrid,
    display_coordinate_reference_id: Option<&str>,
) -> ResolvedSurveyMapHorizonPreview {
    let display_requested = display_coordinate_reference_id
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let resolved_horizon_id = format!("{survey_asset_id}::{}", horizon.descriptor.id);
    let mut notes = horizon.descriptor.notes.clone();
    let native_scalar_field = survey_map_scalar_field_from_horizon_grid(
        horizon,
        &resolved_horizon_id,
        &horizon.descriptor.name,
        survey.native_spatial.grid_transform.as_ref(),
        "native",
        &mut notes,
    );

    let display_scalar_field = if display_requested {
        match survey.display_spatial.as_ref() {
            Some(display_spatial) => survey_map_scalar_field_from_horizon_grid(
                horizon,
                &resolved_horizon_id,
                &horizon.descriptor.name,
                display_spatial.grid_transform.as_ref(),
                "display",
                &mut notes,
            ),
            None => {
                notes.push(format!(
                    "display horizon preview is unavailable because the survey map transform status is {}",
                    survey_map_transform_status_label(survey.transform_status)
                ));
                None
            }
        }
    } else {
        None
    };

    let preview_status = if display_requested {
        if display_scalar_field.is_some() {
            match survey.transform_status {
                SurveyMapTransformStatusDto::DisplayEquivalent => {
                    SurveyMapTransformStatusDto::DisplayEquivalent
                }
                SurveyMapTransformStatusDto::DisplayTransformed => {
                    SurveyMapTransformStatusDto::DisplayTransformed
                }
                _ => SurveyMapTransformStatusDto::DisplayEquivalent,
            }
        } else if native_scalar_field.is_some() && survey.display_spatial.is_some() {
            notes.push(String::from(
                "map preview fell back to native survey coordinates because the display-transformed horizon grid cannot be represented by the current axis-aligned scalar-field renderer",
            ));
            SurveyMapTransformStatusDto::DisplayDegraded
        } else if survey.display_spatial.is_none() {
            SurveyMapTransformStatusDto::DisplayUnavailable
        } else {
            SurveyMapTransformStatusDto::DisplayDegraded
        }
    } else {
        SurveyMapTransformStatusDto::NativeOnly
    };

    let scalar_field = if display_requested {
        display_scalar_field.or(native_scalar_field)
    } else {
        native_scalar_field
    };

    ResolvedSurveyMapHorizonPreview {
        horizon: ResolvedSurveyMapHorizonDto {
            id: resolved_horizon_id,
            survey_asset_id: survey_asset_id.to_string(),
            name: horizon.descriptor.name.clone(),
            source_path: horizon.descriptor.source_path.clone(),
            point_count: horizon.descriptor.point_count,
            mapped_point_count: horizon.descriptor.mapped_point_count,
            missing_cell_count: horizon.descriptor.missing_cell_count,
            source_coordinate_reference: horizon
                .descriptor
                .source_coordinate_reference
                .as_ref()
                .map(coordinate_reference_dto_from_seismic),
            aligned_coordinate_reference: horizon
                .descriptor
                .aligned_coordinate_reference
                .as_ref()
                .map(coordinate_reference_dto_from_seismic),
            transformed: horizon.descriptor.transformed,
            preview_available: scalar_field.is_some(),
            preview_status,
            notes,
        },
        scalar_field,
    }
}

fn survey_map_scalar_field_from_horizon_grid(
    horizon: &ImportedHorizonGrid,
    field_id: &str,
    field_name: &str,
    grid_transform: Option<&SurveyMapGridTransformDto>,
    label: &str,
    notes: &mut Vec<String>,
) -> Option<SurveyMapScalarFieldDto> {
    let Some(grid_transform) = grid_transform else {
        notes.push(format!(
            "{label} horizon preview is unavailable because the survey grid transform is missing"
        ));
        return None;
    };

    let inline_axis_aligned_y = grid_transform.inline_basis.x.abs()
        <= SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON
        && grid_transform.inline_basis.y.abs() > SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON;
    let xline_axis_aligned_x = grid_transform.xline_basis.y.abs()
        <= SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON
        && grid_transform.xline_basis.x.abs() > SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON;
    let inline_axis_aligned_x = grid_transform.inline_basis.y.abs()
        <= SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON
        && grid_transform.inline_basis.x.abs() > SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON;
    let xline_axis_aligned_y = grid_transform.xline_basis.x.abs()
        <= SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON
        && grid_transform.xline_basis.y.abs() > SURVEY_MAP_SCALAR_ALIGNMENT_EPSILON;

    let (columns, rows, step, values) = if inline_axis_aligned_y && xline_axis_aligned_x {
        (
            horizon.xline_count,
            horizon.inline_count,
            ProjectedPoint2Dto {
                x: grid_transform.xline_basis.x,
                y: grid_transform.inline_basis.y,
            },
            horizon_scalar_values_inline_rows(horizon),
        )
    } else if inline_axis_aligned_x && xline_axis_aligned_y {
        (
            horizon.inline_count,
            horizon.xline_count,
            ProjectedPoint2Dto {
                x: grid_transform.inline_basis.x,
                y: grid_transform.xline_basis.y,
            },
            horizon_scalar_values_transposed(horizon),
        )
    } else {
        notes.push(format!(
            "{label} horizon preview is unavailable because the survey grid is rotated or skewed and the current map scalar-field renderer only supports axis-aligned rectilinear grids"
        ));
        return None;
    };

    let (min_value, max_value) = finite_f32_range(&values);
    Some(SurveyMapScalarFieldDto {
        id: field_id.to_string(),
        name: field_name.to_string(),
        columns,
        rows,
        values,
        origin: grid_transform.origin.clone(),
        step,
        unit: None,
        min_value,
        max_value,
    })
}

fn horizon_scalar_values_inline_rows(horizon: &ImportedHorizonGrid) -> Vec<f32> {
    horizon
        .values
        .iter()
        .zip(&horizon.validity)
        .map(|(value, valid)| if *valid == 0 { f32::NAN } else { *value })
        .collect()
}

fn horizon_scalar_values_transposed(horizon: &ImportedHorizonGrid) -> Vec<f32> {
    let mut values = vec![f32::NAN; horizon.inline_count * horizon.xline_count];
    for inline_index in 0..horizon.inline_count {
        for xline_index in 0..horizon.xline_count {
            let source_offset = inline_index * horizon.xline_count + xline_index;
            let target_offset = xline_index * horizon.inline_count + inline_index;
            values[target_offset] = if horizon.validity[source_offset] == 0 {
                f32::NAN
            } else {
                horizon.values[source_offset]
            };
        }
    }
    values
}

fn finite_f32_range(values: &[f32]) -> (Option<f32>, Option<f32>) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for value in values {
        if !value.is_finite() {
            continue;
        }
        min = min.min(*value);
        max = max.max(*value);
    }
    if min.is_finite() && max.is_finite() {
        (Some(min), Some(max))
    } else {
        (None, None)
    }
}

fn survey_map_transform_status_label(status: SurveyMapTransformStatusDto) -> &'static str {
    match status {
        SurveyMapTransformStatusDto::NativeOnly => "native_only",
        SurveyMapTransformStatusDto::DisplayEquivalent => "display_equivalent",
        SurveyMapTransformStatusDto::DisplayTransformed => "display_transformed",
        SurveyMapTransformStatusDto::DisplayDegraded => "display_degraded",
        SurveyMapTransformStatusDto::DisplayUnavailable => "display_unavailable",
    }
}

fn copy_if_exists(source: &Path, target: &Path) -> Result<()> {
    if source.exists() {
        copy_path(source, target)?;
    }
    Ok(())
}

fn copy_path(source: &Path, target: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path(&entry.path(), &target.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

fn stage_project_asset_root(root: &Path, asset_id: &AssetId) -> Result<StagedAssetSnapshot> {
    let staging_root = PathBuf::from(root)
        .join(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_STAGING_DIRNAME)
        .join(&asset_id.0)
        .join(
            revision_token_for_bytes(
                "asset-stage",
                &format!("{}:{}", asset_id.0, now_unix_nanos()),
            )
            .0,
        );
    fs::create_dir_all(&staging_root)?;
    Ok(StagedAssetSnapshot { root: staging_root })
}

fn stage_existing_asset_root(root: &Path, asset: &AssetRecord) -> Result<StagedAssetSnapshot> {
    let staged = stage_project_asset_root(root, &asset.id)?;
    for relative_path in asset_visible_relative_paths(&asset.manifest) {
        copy_if_exists(
            &Path::new(&asset.package_path).join(&relative_path),
            &staged.root.join(relative_path),
        )?;
    }
    Ok(staged)
}

fn project_asset_revision_store_rel_path(
    asset_id: &AssetId,
    revision_id: &AssetRevisionId,
) -> PathBuf {
    PathBuf::from(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_ASSET_REVISION_STORE_DIRNAME)
        .join(&asset_id.0)
        .join(&revision_id.0)
}

fn stable_project_blob_hash(scope: &str, bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    bytes.hash(&mut hasher);
    format!("{scope}-{:016x}", hasher.finish())
}

fn stable_project_path_hash(scope: &str, path: &Path) -> Result<String> {
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    hash_path_into(path, path, &mut hasher)?;
    Ok(format!("{scope}-{:016x}", hasher.finish()))
}

fn hash_path_into(root: &Path, path: &Path, hasher: &mut DefaultHasher) -> Result<()> {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    relative.hash(hasher);
    if path.is_dir() {
        "dir".hash(hasher);
        let mut entries = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            hash_path_into(root, &entry.path(), hasher)?;
        }
    } else {
        "file".hash(hasher);
        fs::read(path)?.hash(hasher);
    }
    Ok(())
}

fn path_byte_count(path: &Path) -> Result<u64> {
    if path.is_dir() {
        let mut total = 0u64;
        for entry in fs::read_dir(path)? {
            total += path_byte_count(&entry?.path())?;
        }
        Ok(total)
    } else {
        Ok(fs::metadata(path)?.len())
    }
}

fn remove_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn asset_primary_blob_ref(manifest: &AssetManifest, root: &Path) -> Result<AssetBlobRef> {
    let descriptor = manifest
        .bulk_data_descriptors
        .iter()
        .find(|descriptor| descriptor.role != "metadata")
        .or_else(|| manifest.bulk_data_descriptors.first())
        .ok_or_else(|| {
            LasError::Validation("asset manifest is missing bulk data descriptors".to_string())
        })?;
    let data_path = root.join(&descriptor.relative_path);
    Ok(AssetBlobRef {
        relative_path: descriptor.relative_path.clone(),
        media_type: descriptor.media_type.clone(),
        byte_count: path_byte_count(&data_path)?,
        content_hash: stable_project_path_hash("asset-data", &data_path)?,
    })
}

fn asset_visible_relative_paths(manifest: &AssetManifest) -> Vec<String> {
    let mut paths = Vec::with_capacity(manifest.bulk_data_descriptors.len() + 1);
    paths.push("metadata.json".to_string());
    paths.push(ASSET_MANIFEST_FILENAME.to_string());
    for descriptor in &manifest.bulk_data_descriptors {
        if !paths.iter().any(|path| path == &descriptor.relative_path) {
            paths.push(descriptor.relative_path.clone());
        }
    }
    paths
}

fn artifact_mappings_for_asset(
    asset: &AssetRecord,
    revision_root: &Path,
) -> Vec<(PathBuf, PathBuf)> {
    asset_visible_relative_paths(&asset.manifest)
        .into_iter()
        .map(|relative_path| {
            (
                revision_root.join(&relative_path),
                Path::new(&asset.package_path).join(relative_path),
            )
        })
        .collect()
}

fn materialize_project_visible_files(root: &Path, mappings: &[(PathBuf, PathBuf)]) -> Result<()> {
    let backup_root = PathBuf::from(root)
        .join(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_STAGING_DIRNAME)
        .join(
            revision_token_for_bytes(
                "project-materialize-backup",
                &format!("{}:{}", root.display(), now_unix_nanos()),
            )
            .0,
        );
    fs::create_dir_all(&backup_root)?;

    for (_, destination) in mappings {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            copy_path(
                destination,
                &backup_root.join(
                    destination
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_else(|| String::from("backup")),
                ),
            )?;
        }
    }

    for (source, destination) in mappings {
        let temp_path = destination.with_extension("next");
        remove_path(&temp_path)?;
        copy_path(source, &temp_path)?;
        if destination.exists() {
            remove_path(destination)?;
        }
        if let Err(error) = fs::rename(&temp_path, destination) {
            restore_project_visible_files(&backup_root, mappings)?;
            return Err(LasError::Io(error));
        }
    }

    if backup_root.exists() {
        fs::remove_dir_all(backup_root)?;
    }
    Ok(())
}

fn restore_project_visible_files(root: &Path, mappings: &[(PathBuf, PathBuf)]) -> Result<()> {
    for (_, destination) in mappings {
        let backup_path = root.join(
            destination
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| String::from("backup")),
        );
        if backup_path.exists() {
            if destination.exists() {
                remove_path(destination)?;
            }
            copy_path(&backup_path, destination)?;
        }
    }
    Ok(())
}

fn clear_project_visible_files(asset: &AssetRecord) -> Result<()> {
    for relative_path in asset_visible_relative_paths(&asset.manifest) {
        let path = Path::new(&asset.package_path).join(relative_path);
        if path.exists() {
            remove_path(&path)?;
        }
    }
    Ok(())
}

fn default_asset_diff_summary(asset_kind: &AssetKind) -> AssetDiffSummary {
    match asset_kind {
        AssetKind::Log => AssetDiffSummary::Log(LogAssetDiffSummary::default()),
        AssetKind::Trajectory => {
            AssetDiffSummary::Trajectory(StructuredAssetDiffSummary::default())
        }
        AssetKind::TopSet => AssetDiffSummary::TopSet(StructuredAssetDiffSummary::default()),
        AssetKind::WellMarkerSet => {
            AssetDiffSummary::WellMarkerSet(StructuredAssetDiffSummary::default())
        }
        AssetKind::WellMarkerHorizonResidualSet => {
            AssetDiffSummary::WellMarkerHorizonResidualSet(StructuredAssetDiffSummary::default())
        }
        AssetKind::PressureObservation => {
            AssetDiffSummary::PressureObservation(StructuredAssetDiffSummary::default())
        }
        AssetKind::DrillingObservation => {
            AssetDiffSummary::DrillingObservation(StructuredAssetDiffSummary::default())
        }
        AssetKind::CheckshotVspObservationSet => {
            AssetDiffSummary::CheckshotVspObservationSet(DirectoryAssetDiffSummary::default())
        }
        AssetKind::ManualTimeDepthPickSet => {
            AssetDiffSummary::ManualTimeDepthPickSet(DirectoryAssetDiffSummary::default())
        }
        AssetKind::WellTieObservationSet => {
            AssetDiffSummary::WellTieObservationSet(DirectoryAssetDiffSummary::default())
        }
        AssetKind::WellTimeDepthAuthoredModel => {
            AssetDiffSummary::WellTimeDepthAuthoredModel(DirectoryAssetDiffSummary::default())
        }
        AssetKind::WellTimeDepthModel => {
            AssetDiffSummary::WellTimeDepthModel(DirectoryAssetDiffSummary::default())
        }
        AssetKind::RawSourceBundle => {
            AssetDiffSummary::RawSourceBundle(DirectoryAssetDiffSummary::default())
        }
        AssetKind::SeismicTraceData => {
            AssetDiffSummary::SeismicTraceData(DirectoryAssetDiffSummary::default())
        }
    }
}

fn diff_structured_rows<T: PartialEq>(
    asset_kind: AssetKind,
    previous_rows: &[T],
    current_rows: &[T],
    extent_changed: bool,
) -> AssetDiffSummary {
    let rows_updated = previous_rows
        .iter()
        .zip(current_rows.iter())
        .filter(|(left, right)| left != right)
        .count();
    let summary = StructuredAssetDiffSummary {
        rows_added: current_rows.len().saturating_sub(previous_rows.len()),
        rows_removed: previous_rows.len().saturating_sub(current_rows.len()),
        rows_updated,
        extent_changed,
    };
    match asset_kind {
        AssetKind::Trajectory => AssetDiffSummary::Trajectory(summary),
        AssetKind::TopSet => AssetDiffSummary::TopSet(summary),
        AssetKind::WellMarkerSet => AssetDiffSummary::WellMarkerSet(summary),
        AssetKind::WellMarkerHorizonResidualSet => {
            AssetDiffSummary::WellMarkerHorizonResidualSet(summary)
        }
        AssetKind::PressureObservation => AssetDiffSummary::PressureObservation(summary),
        AssetKind::DrillingObservation => AssetDiffSummary::DrillingObservation(summary),
        AssetKind::Log => AssetDiffSummary::Log(LogAssetDiffSummary::default()),
        AssetKind::CheckshotVspObservationSet => {
            AssetDiffSummary::CheckshotVspObservationSet(DirectoryAssetDiffSummary::default())
        }
        AssetKind::ManualTimeDepthPickSet => {
            AssetDiffSummary::ManualTimeDepthPickSet(DirectoryAssetDiffSummary::default())
        }
        AssetKind::WellTieObservationSet => {
            AssetDiffSummary::WellTieObservationSet(DirectoryAssetDiffSummary::default())
        }
        AssetKind::WellTimeDepthAuthoredModel => {
            AssetDiffSummary::WellTimeDepthAuthoredModel(DirectoryAssetDiffSummary::default())
        }
        AssetKind::WellTimeDepthModel => {
            AssetDiffSummary::WellTimeDepthModel(DirectoryAssetDiffSummary::default())
        }
        AssetKind::RawSourceBundle => {
            AssetDiffSummary::RawSourceBundle(DirectoryAssetDiffSummary::default())
        }
        AssetKind::SeismicTraceData => {
            AssetDiffSummary::SeismicTraceData(DirectoryAssetDiffSummary::default())
        }
    }
}

fn semantic_diff_fields(
    previous: &[CurveSemanticDescriptor],
    current: &[CurveSemanticDescriptor],
) -> Vec<String> {
    current
        .iter()
        .filter_map(|descriptor| {
            let previous_descriptor = previous
                .iter()
                .find(|item| item.curve_name == descriptor.curve_name)?;
            (previous_descriptor.semantic_type != descriptor.semantic_type
                || previous_descriptor.source != descriptor.source
                || previous_descriptor.semantic_parameters != descriptor.semantic_parameters)
                .then(|| format!("curve_semantics.{}", descriptor.curve_name))
        })
        .collect()
}

fn diff_log_files(previous: &LasFile, current: &LasFile) -> LogAssetDiffSummary {
    let previous_curves = previous
        .curves
        .iter()
        .map(|curve| (curve.mnemonic.clone(), curve))
        .collect::<BTreeMap<_, _>>();
    let current_curves = current
        .curves
        .iter()
        .map(|curve| (curve.mnemonic.clone(), curve))
        .collect::<BTreeMap<_, _>>();
    let curves_added = current_curves
        .keys()
        .filter(|name| !previous_curves.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let curves_removed = previous_curves
        .keys()
        .filter(|name| !current_curves.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let modified_curves = current_curves
        .iter()
        .filter_map(|(name, current_curve)| {
            let previous_curve = previous_curves.get(name)?;
            let summary = diff_log_curve_values(name, previous_curve, current_curve);
            (summary.changed_value_count > 0).then_some(summary)
        })
        .collect::<Vec<_>>();

    LogAssetDiffSummary {
        metadata_changed: serde_json::to_string(&package_metadata_for(current, 1).canonical).ok()
            != serde_json::to_string(&package_metadata_for(previous, 1).canonical).ok(),
        row_count_changed: current.row_count() != previous.row_count(),
        curve_count_changed: current.curves.len() != previous.curves.len(),
        curves_added,
        curves_removed,
        modified_curves,
    }
}

fn diff_log_curve_values(
    curve_name: &str,
    previous: &CurveItem,
    current: &CurveItem,
) -> CurveValueChangeSummary {
    let max_len = previous.data.len().max(current.data.len());
    let mut changed_value_count = 0usize;
    let mut first_changed_row = None;
    let mut last_changed_row = None;

    for row_index in 0..max_len {
        let previous_value = previous.data.get(row_index);
        let current_value = current.data.get(row_index);
        if log_values_equal(previous_value, current_value) {
            continue;
        }
        changed_value_count += 1;
        first_changed_row.get_or_insert(row_index);
        last_changed_row = Some(row_index);
    }

    CurveValueChangeSummary {
        curve_name: curve_name.to_string(),
        changed_value_count,
        first_changed_row,
        last_changed_row,
    }
}

fn summarize_asset_diff(asset_kind: &AssetKind, diff: &AssetDiffSummary) -> String {
    match diff {
        AssetDiffSummary::Log(summary) => summarize_log_asset_diff(summary),
        AssetDiffSummary::Trajectory(summary) => {
            summarize_structured_asset_diff("trajectory", summary)
        }
        AssetDiffSummary::TopSet(summary) => summarize_structured_asset_diff("tops", summary),
        AssetDiffSummary::WellMarkerSet(summary) => {
            summarize_structured_asset_diff("well markers", summary)
        }
        AssetDiffSummary::WellMarkerHorizonResidualSet(summary) => {
            summarize_structured_asset_diff("well marker horizon residuals", summary)
        }
        AssetDiffSummary::PressureObservation(summary) => {
            summarize_structured_asset_diff("pressure observations", summary)
        }
        AssetDiffSummary::DrillingObservation(summary) => {
            summarize_structured_asset_diff("drilling observations", summary)
        }
        AssetDiffSummary::CheckshotVspObservationSet(summary) => {
            summarize_directory_asset_diff("checkshot/VSP observations", summary)
        }
        AssetDiffSummary::ManualTimeDepthPickSet(summary) => {
            summarize_directory_asset_diff("manual time-depth picks", summary)
        }
        AssetDiffSummary::WellTieObservationSet(summary) => {
            summarize_directory_asset_diff("well tie observations", summary)
        }
        AssetDiffSummary::WellTimeDepthAuthoredModel(summary) => {
            summarize_directory_asset_diff("well time-depth authored model", summary)
        }
        AssetDiffSummary::WellTimeDepthModel(summary) => {
            summarize_directory_asset_diff("well time-depth model", summary)
        }
        AssetDiffSummary::RawSourceBundle(summary) => {
            summarize_directory_asset_diff("raw source bundle", summary)
        }
        AssetDiffSummary::SeismicTraceData(summary) => {
            summarize_directory_asset_diff("seismic trace data", summary)
        }
        AssetDiffSummary::MetadataOnly { changed_fields } => {
            if changed_fields.is_empty() {
                format!("updated {} metadata", asset_kind.as_str())
            } else {
                format!("updated metadata fields {}", changed_fields.join(", "))
            }
        }
    }
}

fn summarize_log_asset_diff(diff: &LogAssetDiffSummary) -> String {
    let mut parts = Vec::new();
    if diff.metadata_changed {
        parts.push(String::from("metadata updated"));
    }
    if !diff.curves_added.is_empty() {
        parts.push(format!("added curves {}", diff.curves_added.join(", ")));
    }
    if !diff.curves_removed.is_empty() {
        parts.push(format!("removed curves {}", diff.curves_removed.join(", ")));
    }
    if !diff.modified_curves.is_empty() {
        parts.push(format!(
            "updated {} curve value ranges",
            diff.modified_curves.len()
        ));
    }
    if diff.row_count_changed {
        parts.push(String::from("row count changed"));
    }
    if diff.curve_count_changed {
        parts.push(String::from("curve count changed"));
    }
    if parts.is_empty() {
        String::from("initial log asset revision")
    } else {
        parts.join("; ")
    }
}

fn summarize_structured_asset_diff(label: &str, diff: &StructuredAssetDiffSummary) -> String {
    let mut parts = Vec::new();
    if diff.rows_added > 0 {
        parts.push(format!("added {} rows", diff.rows_added));
    }
    if diff.rows_removed > 0 {
        parts.push(format!("removed {} rows", diff.rows_removed));
    }
    if diff.rows_updated > 0 {
        parts.push(format!("updated {} rows", diff.rows_updated));
    }
    if diff.extent_changed {
        parts.push(String::from("extent changed"));
    }
    if parts.is_empty() {
        format!("initial {label} asset revision")
    } else {
        parts.join("; ")
    }
}

fn summarize_directory_asset_diff(label: &str, diff: &DirectoryAssetDiffSummary) -> String {
    let mut parts = Vec::new();
    if diff.entry_count_changed {
        parts.push(String::from("entry count changed"));
    }
    if diff.changed_path_count > 0 {
        parts.push(format!("updated {} paths", diff.changed_path_count));
    }
    if parts.is_empty() {
        format!("initial {label} asset revision")
    } else {
        parts.join("; ")
    }
}

fn log_values_equal(previous: Option<&LasValue>, current: Option<&LasValue>) -> bool {
    match (previous, current) {
        (Some(LasValue::Number(left)), Some(LasValue::Number(right))) => {
            (left.is_nan() && right.is_nan()) || left == right
        }
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

fn initialize_project_schema(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "
        CREATE TABLE IF NOT EXISTS project_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS wells (
            id TEXT PRIMARY KEY,
            primary_name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            uwi TEXT,
            api TEXT,
            identifiers_json TEXT NOT NULL,
            metadata_json TEXT,
            created_at_unix_seconds INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS wellbores (
            id TEXT PRIMARY KEY,
            well_id TEXT NOT NULL,
            primary_name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            identifiers_json TEXT NOT NULL,
            metadata_json TEXT,
            geometry_json TEXT,
            definitive_trajectory_asset_id TEXT,
            definitive_marker_set_asset_id TEXT,
            definitive_top_set_asset_id TEXT,
            active_well_time_depth_model_asset_id TEXT,
            created_at_unix_seconds INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS well_markers (
            id TEXT PRIMARY KEY,
            wellbore_id TEXT NOT NULL,
            source_asset_id TEXT,
            sequence_number INTEGER,
            name TEXT NOT NULL,
            marker_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS asset_collections (
            id TEXT PRIMARY KEY,
            wellbore_id TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            name TEXT NOT NULL,
            logical_asset_id TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL,
            UNIQUE(wellbore_id, asset_kind, name)
        );
        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            logical_asset_id TEXT NOT NULL,
            collection_id TEXT NOT NULL,
            well_id TEXT NOT NULL,
            wellbore_id TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            package_rel_path TEXT NOT NULL,
            manifest_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL,
            source_path TEXT,
            source_fingerprint TEXT
        );
        CREATE TABLE IF NOT EXISTS asset_owners (
            scope TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            display_name TEXT NOT NULL,
            metadata_json TEXT,
            created_at_unix_seconds INTEGER NOT NULL,
            PRIMARY KEY (scope, owner_id)
        );
        CREATE TABLE IF NOT EXISTS asset_collection_owners (
            collection_id TEXT PRIMARY KEY,
            owner_scope TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS asset_revisions (
            id TEXT PRIMARY KEY,
            asset_id TEXT NOT NULL,
            logical_asset_id TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            parent_revision_id TEXT,
            revision_json TEXT NOT NULL,
            created_at_unix_seconds INTEGER NOT NULL
        );
        ",
        )
        .map_err(sqlite_error)?;
    connection
        .execute(
            "INSERT OR REPLACE INTO project_meta (key, value) VALUES ('schema_version', ?1)",
            params![PROJECT_SCHEMA_VERSION],
        )
        .map_err(sqlite_error)?;
    ensure_optional_text_column(connection, "wells", "metadata_json")?;
    ensure_optional_text_column(connection, "wellbores", "metadata_json")?;
    ensure_optional_text_column(connection, "wellbores", "geometry_json")?;
    ensure_optional_text_column(connection, "wellbores", "definitive_trajectory_asset_id")?;
    ensure_optional_text_column(connection, "wellbores", "definitive_marker_set_asset_id")?;
    ensure_optional_text_column(connection, "wellbores", "definitive_top_set_asset_id")?;
    ensure_optional_text_column(
        connection,
        "wellbores",
        "active_well_time_depth_model_asset_id",
    )?;
    Ok(())
}

fn ensure_optional_text_column(connection: &Connection, table: &str, column: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut statement = connection.prepare(&pragma).map_err(sqlite_error)?;
    let mut rows = statement.query([]).map_err(sqlite_error)?;
    while let Some(row) = rows.next().map_err(sqlite_error)? {
        let existing: String = row.get(1).map_err(sqlite_error)?;
        if existing == column {
            return Ok(());
        }
    }
    connection
        .execute(&format!("ALTER TABLE {table} ADD COLUMN {column} TEXT"), [])
        .map_err(sqlite_error)?;
    Ok(())
}

fn parse_optional_json_column<T>(
    value: Option<String>,
) -> std::result::Result<Option<T>, serde_json::Error>
where
    T: for<'de> Deserialize<'de>,
{
    value
        .map(|json| serde_json::from_str::<T>(&json))
        .transpose()
}

fn well_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WellRecord> {
    Ok(WellRecord {
        id: WellId(row.get(0)?),
        name: row.get(1)?,
        identifiers: serde_json::from_str::<WellIdentifierSet>(&row.get::<_, String>(2)?)
            .map_err(sql_json_error)?,
        metadata: parse_optional_json_column::<WellMetadata>(row.get(3)?)
            .map_err(sql_json_error)?,
    })
}

fn wellbore_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WellboreRecord> {
    Ok(WellboreRecord {
        id: WellboreId(row.get(0)?),
        well_id: WellId(row.get(1)?),
        name: row.get(2)?,
        identifiers: serde_json::from_str::<WellIdentifierSet>(&row.get::<_, String>(3)?)
            .map_err(sql_json_error)?,
        metadata: parse_optional_json_column::<WellboreMetadata>(row.get(4)?)
            .map_err(sql_json_error)?,
        geometry: parse_optional_json_column::<WellboreGeometry>(row.get(5)?)
            .map_err(sql_json_error)?,
        definitive_trajectory_asset_id: row.get::<_, Option<String>>(6)?.map(AssetId),
        definitive_marker_set_asset_id: row.get::<_, Option<String>>(7)?.map(AssetId),
        definitive_top_set_asset_id: row.get::<_, Option<String>>(8)?.map(AssetId),
        active_well_time_depth_model_asset_id: row.get::<_, Option<String>>(9)?.map(AssetId),
    })
}

fn classify_log_curves_from_file(file: &LasFile) -> Vec<CurveSemanticDescriptor> {
    let package_metadata = package_metadata_for(file, 1);
    package_metadata
        .storage
        .curve_columns
        .iter()
        .map(|curve| CurveSemanticDescriptor {
            curve_name: curve.name.clone(),
            original_mnemonic: curve.original_mnemonic.clone(),
            unit: (!curve.unit.trim().is_empty()).then_some(curve.unit.clone()),
            semantic_type: classify_curve_semantic(
                &curve.alias,
                &curve.original_mnemonic,
                Some(&curve.unit),
                curve.is_index,
            ),
            source: CurveSemanticSource::Derived,
            semantic_parameters: BTreeMap::new(),
        })
        .collect()
}

fn classify_log_curves_from_package(package_path: &str) -> Result<Vec<CurveSemanticDescriptor>> {
    let package = open_package(package_path)?;
    Ok(classify_log_curves_from_file(package.file()))
}

fn log_curve_data_for_compute(
    file: &LasFile,
    semantics: &[CurveSemanticDescriptor],
) -> Result<Vec<LogCurveData>> {
    let index_curve = file.curve(&file.index.curve_id)?;
    let depths = index_curve.numeric_data().ok_or_else(|| {
        LasError::Validation(format!(
            "index curve '{}' must remain numeric for compute execution",
            file.index.curve_id
        ))
    })?;
    let semantics_by_name = semantics
        .iter()
        .map(|item| (item.curve_name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut result = Vec::new();
    for curve in file.curves.iter() {
        let numeric = curve.numeric_data();
        if numeric.is_none() {
            continue;
        }
        let descriptor = semantics_by_name
            .get(&curve.mnemonic)
            .cloned()
            .unwrap_or_else(|| CurveSemanticDescriptor {
                curve_name: curve.mnemonic.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: (!curve.unit.trim().is_empty()).then_some(curve.unit.clone()),
                semantic_type: classify_curve_semantic(
                    &derive_canonical_alias(&curve.original_mnemonic, &curve.unit),
                    &curve.original_mnemonic,
                    Some(&curve.unit),
                    curve.mnemonic == file.index.curve_id,
                ),
                source: CurveSemanticSource::Derived,
                semantic_parameters: BTreeMap::new(),
            });
        let values = numeric
            .unwrap()
            .into_iter()
            .map(|value| (!value.is_nan()).then_some(value))
            .collect::<Vec<_>>();
        result.push(LogCurveData {
            curve_name: curve.mnemonic.clone(),
            original_mnemonic: curve.original_mnemonic.clone(),
            unit: (!curve.unit.trim().is_empty()).then_some(curve.unit.clone()),
            semantic_type: descriptor.semantic_type,
            depths: depths.clone(),
            values,
        });
    }
    Ok(result)
}

fn filter_log_curve_for_depth_range(
    curve: &LogCurveData,
    depth_min: Option<f64>,
    depth_max: Option<f64>,
) -> LogCurveData {
    let mut depths = Vec::new();
    let mut values = Vec::new();
    for (depth, value) in curve.depths.iter().zip(curve.values.iter()) {
        if !depth_in_range(*depth, depth_min, depth_max) {
            continue;
        }
        depths.push(*depth);
        values.push(*value);
    }
    LogCurveData {
        curve_name: curve.curve_name.clone(),
        original_mnemonic: curve.original_mnemonic.clone(),
        unit: curve.unit.clone(),
        semantic_type: curve.semantic_type.clone(),
        depths,
        values,
    }
}

fn filter_top_rows_for_depth_range(
    rows: Vec<TopRow>,
    depth_min: Option<f64>,
    depth_max: Option<f64>,
) -> Vec<TopRow> {
    rows.into_iter()
        .filter(|row| depth_in_range(row.top_depth, depth_min, depth_max))
        .collect()
}

fn filter_well_marker_rows_for_depth_range(
    rows: Vec<WellMarkerRow>,
    depth_min: Option<f64>,
    depth_max: Option<f64>,
) -> Vec<WellMarkerRow> {
    rows.into_iter()
        .filter(|row| depth_in_range(row.top_depth, depth_min, depth_max))
        .collect()
}

fn identity_panel_depth_mapping(depths: Vec<f64>) -> Vec<WellPanelDepthSampleDto> {
    let mut unique = depths
        .into_iter()
        .filter(|depth| depth.is_finite())
        .collect::<Vec<_>>();
    unique.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    unique.dedup_by(|left, right| (*left - *right).abs() < 1e-6);
    unique
        .into_iter()
        .map(|depth| WellPanelDepthSampleDto {
            native_depth: depth,
            panel_depth: depth,
        })
        .collect()
}

fn depth_in_range(depth: f64, depth_min: Option<f64>, depth_max: Option<f64>) -> bool {
    if let Some(min) = depth_min {
        if depth < min {
            return false;
        }
    }
    if let Some(max) = depth_max {
        if depth > max {
            return false;
        }
    }
    true
}

fn depth_query(depth_min: Option<f64>, depth_max: Option<f64>) -> Option<DepthRangeQuery> {
    if depth_min.is_none() && depth_max.is_none() {
        None
    } else {
        Some(DepthRangeQuery {
            depth_min,
            depth_max,
        })
    }
}

fn residual_marker_inputs_for_asset(
    project: &OphioliteProject,
    asset: &AssetRecord,
) -> Result<Vec<ResidualMarkerInput>> {
    match asset.asset_kind {
        AssetKind::TopSet => Ok(project
            .read_tops(&asset.id)?
            .into_iter()
            .map(|row| ResidualMarkerInput {
                marker_name: row.name,
                marker_kind: Some("top".to_string()),
                source_depth: row.top_depth,
                source_depth_reference: row.source_depth_reference,
                source_depth_domain: row.depth_domain,
                source_depth_datum: row.depth_datum,
                note: row.source,
            })
            .collect()),
        AssetKind::WellMarkerSet => Ok(project
            .read_well_marker_rows(&asset.id)?
            .into_iter()
            .map(|row| ResidualMarkerInput {
                marker_name: row.name,
                marker_kind: row.marker_kind,
                source_depth: row.top_depth,
                source_depth_reference: row.source_depth_reference,
                source_depth_domain: row.depth_domain,
                source_depth_datum: row.depth_datum,
                note: row.note.or(row.source),
            })
            .collect()),
        _ => Err(LasError::Validation(format!(
            "asset '{}' is {}, not a top-set or well-marker-set source",
            asset.id.0,
            asset.asset_kind.as_str()
        ))),
    }
}

fn well_marker_horizon_residual_row_dto(
    row: WellMarkerHorizonResidualRow,
) -> WellMarkerHorizonResidualRowDto {
    WellMarkerHorizonResidualRowDto {
        marker_name: row.marker_name,
        marker_kind: row.marker_kind,
        source_depth: row.source_depth,
        source_depth_reference: row.source_depth_reference,
        source_depth_domain: row.source_depth_domain,
        source_depth_datum: row.source_depth_datum,
        measured_depth: row.measured_depth,
        true_vertical_depth: row.true_vertical_depth,
        true_vertical_depth_subsea: row.true_vertical_depth_subsea,
        x: row.x,
        y: row.y,
        horizon_depth: row.horizon_depth,
        residual: row.residual,
        horizon_inline_ordinal: row.horizon_inline_ordinal,
        horizon_xline_ordinal: row.horizon_xline_ordinal,
        status: row.status,
        note: row.note,
    }
}

fn well_marker_horizon_residual_row_from_dto(
    row: &WellMarkerHorizonResidualRowDto,
) -> WellMarkerHorizonResidualRow {
    WellMarkerHorizonResidualRow {
        marker_name: row.marker_name.clone(),
        marker_kind: row.marker_kind.clone(),
        source_depth: row.source_depth,
        source_depth_reference: row.source_depth_reference.clone(),
        source_depth_domain: row.source_depth_domain.clone(),
        source_depth_datum: row.source_depth_datum.clone(),
        measured_depth: row.measured_depth,
        true_vertical_depth: row.true_vertical_depth,
        true_vertical_depth_subsea: row.true_vertical_depth_subsea,
        x: row.x,
        y: row.y,
        horizon_depth: row.horizon_depth,
        residual: row.residual,
        horizon_inline_ordinal: row.horizon_inline_ordinal,
        horizon_xline_ordinal: row.horizon_xline_ordinal,
        status: row.status.clone(),
        note: row.note.clone(),
    }
}

fn well_marker_horizon_residual_point_from_row(
    row: WellMarkerHorizonResidualRow,
) -> Option<WellMarkerHorizonResidualPointRecord> {
    if row.status != "ok" && row.status != "resolved" {
        return None;
    }
    Some(WellMarkerHorizonResidualPointRecord {
        marker_name: row.marker_name,
        marker_kind: row.marker_kind,
        x: row.x?,
        y: row.y?,
        z: row.true_vertical_depth?,
        horizon_depth: row.horizon_depth?,
        residual: row.residual?,
        status: row.status,
        note: row.note,
    })
}

fn resolve_single_well_marker_horizon_residual(
    marker: &ResidualMarkerInput,
    trajectory: &ResolvedTrajectoryGeometry,
    horizon: &ImportedHorizonGrid,
    grid_transform: &SurveyMapGridTransformDto,
) -> WellMarkerHorizonResidualRow {
    let mut row = WellMarkerHorizonResidualRow {
        marker_name: marker.marker_name.clone(),
        marker_kind: marker.marker_kind.clone(),
        source_depth: marker.source_depth,
        source_depth_reference: marker.source_depth_reference.clone(),
        source_depth_domain: marker.source_depth_domain.clone(),
        source_depth_datum: marker.source_depth_datum.clone(),
        measured_depth: None,
        true_vertical_depth: None,
        true_vertical_depth_subsea: None,
        x: None,
        y: None,
        horizon_depth: None,
        residual: None,
        horizon_inline_ordinal: None,
        horizon_xline_ordinal: None,
        status: "unresolved".to_string(),
        note: marker.note.clone(),
    };

    let Some(marker_station) = resolve_marker_station(
        trajectory,
        marker.source_depth_domain.as_deref(),
        marker.source_depth,
    ) else {
        row.status = "marker_not_resolved_on_trajectory".to_string();
        row.note = merge_optional_notes(
            row.note.take(),
            Some("marker depth could not be located on the definitive trajectory".to_string()),
        );
        return row;
    };
    row.measured_depth = Some(marker_station.measured_depth_m);
    row.true_vertical_depth = marker_station.true_vertical_depth_m;
    row.true_vertical_depth_subsea = marker_station.true_vertical_depth_subsea_m;
    row.x = marker_station.absolute_xy.as_ref().map(|point| point.x);
    row.y = marker_station.absolute_xy.as_ref().map(|point| point.y);

    if normalized_marker_depth_domain(marker.source_depth_domain.as_deref()) == Some("tvdss") {
        row.status = "unsupported_depth_reference_for_horizon".to_string();
        row.note = merge_optional_notes(
            row.note.take(),
            Some(
                "marker uses TVDSS but imported horizons do not yet carry an explicit vertical datum relation".to_string(),
            ),
        );
        return row;
    }

    let Some(marker_tvd) = marker_station.true_vertical_depth_m else {
        row.status = "marker_missing_tvd".to_string();
        row.note = merge_optional_notes(
            row.note.take(),
            Some("resolved marker station does not contain TVD in meters".to_string()),
        );
        return row;
    };
    let Some(absolute_xy) = marker_station.absolute_xy.as_ref() else {
        row.status = "marker_missing_xy".to_string();
        row.note = merge_optional_notes(
            row.note.take(),
            Some("resolved marker station has no absolute XY coordinates".to_string()),
        );
        return row;
    };

    let Some((inline_ordinal, xline_ordinal)) =
        invert_survey_grid_transform(grid_transform, absolute_xy.x, absolute_xy.y)
    else {
        row.status = "survey_grid_transform_unavailable".to_string();
        row.note = merge_optional_notes(
            row.note.take(),
            Some("marker XY could not be inverted into survey grid coordinates".to_string()),
        );
        return row;
    };
    row.horizon_inline_ordinal = Some(inline_ordinal);
    row.horizon_xline_ordinal = Some(xline_ordinal);

    let Some(horizon_depth) = bilinear_sample_horizon_depth(horizon, inline_ordinal, xline_ordinal)
    else {
        row.status = "horizon_sample_unavailable".to_string();
        row.note = merge_optional_notes(
            row.note.take(),
            Some(
                "marker lies outside the horizon grid or adjacent cells are invalid for bilinear sampling".to_string(),
            ),
        );
        return row;
    };
    row.horizon_depth = Some(f64::from(horizon_depth));
    row.residual = Some(marker_tvd - f64::from(horizon_depth));
    row.status = "ok".to_string();
    row
}

fn resolve_marker_station(
    trajectory: &ResolvedTrajectoryGeometry,
    depth_domain: Option<&str>,
    depth_value: f64,
) -> Option<ResolvedTrajectoryStation> {
    let normalized = normalized_marker_depth_domain(depth_domain).unwrap_or("md");
    let depth_reference = match normalized {
        "md" => DepthReferenceKind::MeasuredDepth,
        "tvd" => DepthReferenceKind::TrueVerticalDepth,
        "tvdss" => DepthReferenceKind::TrueVerticalDepthSubsea,
        _ => return None,
    };
    interpolate_trajectory_station_by_depth(&trajectory.stations, depth_reference, depth_value)
}

fn normalized_marker_depth_domain(depth_domain: Option<&str>) -> Option<&'static str> {
    match depth_domain
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("md") | Some("measured_depth") | Some("measured depth") => Some("md"),
        Some("tvd") | Some("true_vertical_depth") | Some("true vertical depth") => Some("tvd"),
        Some("tvdss")
        | Some("tvdss_m")
        | Some("true_vertical_depth_subsea")
        | Some("true vertical depth subsea") => Some("tvdss"),
        Some("elevation") | Some("elev") => Some("elevation"),
        Some(_) | None => None,
    }
}

fn interpolate_trajectory_station_by_depth(
    stations: &[ResolvedTrajectoryStation],
    depth_reference: DepthReferenceKind,
    depth_value: f64,
) -> Option<ResolvedTrajectoryStation> {
    let first = stations.first()?;
    let first_depth = depth_for_model(first, depth_reference)?;
    if (first_depth - depth_value).abs() <= 1e-6 {
        return Some(first.clone());
    }

    for window in stations.windows(2) {
        let start = &window[0];
        let end = &window[1];
        let start_depth = depth_for_model(start, depth_reference)?;
        let end_depth = depth_for_model(end, depth_reference)?;
        if (end_depth - depth_value).abs() <= 1e-6 {
            return Some(end.clone());
        }
        if !depth_brackets_value(start_depth, end_depth, depth_value) {
            continue;
        }
        let denominator = end_depth - start_depth;
        let fraction = if denominator.abs() <= f64::EPSILON {
            0.0
        } else {
            (depth_value - start_depth) / denominator
        };
        return Some(interpolate_resolved_trajectory_station(
            start, end, fraction,
        ));
    }

    None
}

fn depth_brackets_value(start: f64, end: f64, value: f64) -> bool {
    (value >= start && value <= end) || (value >= end && value <= start)
}

fn bilinear_sample_horizon_depth(
    horizon: &ImportedHorizonGrid,
    inline_ordinal: f64,
    xline_ordinal: f64,
) -> Option<f32> {
    if !inline_ordinal.is_finite() || !xline_ordinal.is_finite() {
        return None;
    }
    if inline_ordinal < 0.0
        || xline_ordinal < 0.0
        || inline_ordinal > (horizon.inline_count.saturating_sub(1)) as f64
        || xline_ordinal > (horizon.xline_count.saturating_sub(1)) as f64
    {
        return None;
    }

    let inline0 = inline_ordinal.floor() as usize;
    let xline0 = xline_ordinal.floor() as usize;
    let inline1 = inline_ordinal.ceil() as usize;
    let xline1 = xline_ordinal.ceil() as usize;

    let q00 = horizon_cell_value(horizon, inline0, xline0)?;
    if inline0 == inline1 && xline0 == xline1 {
        return Some(q00);
    }

    let q10 = horizon_cell_value(horizon, inline1, xline0)?;
    let q01 = horizon_cell_value(horizon, inline0, xline1)?;
    let q11 = horizon_cell_value(horizon, inline1, xline1)?;

    let tx = (inline_ordinal - inline0 as f64) as f32;
    let ty = (xline_ordinal - xline0 as f64) as f32;
    let top = q00 + (q10 - q00) * tx;
    let bottom = q01 + (q11 - q01) * tx;
    Some(top + (bottom - top) * ty)
}

fn horizon_cell_value(
    horizon: &ImportedHorizonGrid,
    inline_index: usize,
    xline_index: usize,
) -> Option<f32> {
    let offset = inline_index.checked_mul(horizon.xline_count)? + xline_index;
    (offset < horizon.values.len()
        && offset < horizon.validity.len()
        && horizon.validity[offset] != 0)
        .then_some(horizon.values[offset])
}

fn merge_optional_notes(existing: Option<String>, extra: Option<String>) -> Option<String> {
    match (existing, extra) {
        (Some(existing), Some(extra)) if !existing.trim().is_empty() => {
            Some(format!("{existing}; {extra}"))
        }
        (Some(existing), _) => Some(existing),
        (None, Some(extra)) => Some(extra),
        (None, None) => None,
    }
}

fn required_string_compute_parameter(
    parameters: &BTreeMap<String, ComputeParameterValue>,
    name: &str,
) -> Result<String> {
    parameters
        .get(name)
        .and_then(ComputeParameterValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| LasError::Validation(format!("parameter '{}' is required", name)))
}

fn optional_string_compute_parameter(
    parameters: &BTreeMap<String, ComputeParameterValue>,
    name: &str,
) -> Option<String> {
    parameters
        .get(name)
        .and_then(ComputeParameterValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

enum StructuredComputedRows {
    Trajectory(Vec<TrajectoryRow>),
    TopSet(Vec<TopRow>),
    Pressure(Vec<PressureObservationRow>),
    Drilling(Vec<DrillingObservationRow>),
}

fn trajectory_rows_for_compute(rows: &[TrajectoryRow]) -> Vec<TrajectoryDataRow> {
    rows.iter()
        .cloned()
        .map(|row| TrajectoryDataRow {
            measured_depth: row.measured_depth,
            true_vertical_depth: row.true_vertical_depth,
            azimuth_deg: row.azimuth_deg,
            inclination_deg: row.inclination_deg,
            northing_offset: row.northing_offset,
            easting_offset: row.easting_offset,
        })
        .collect()
}

fn trajectory_rows_from_compute(rows: &[TrajectoryDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::Trajectory(
        rows.iter()
            .cloned()
            .map(|row| TrajectoryRow {
                measured_depth: row.measured_depth,
                true_vertical_depth: row.true_vertical_depth,
                true_vertical_depth_subsea: None,
                azimuth_deg: row.azimuth_deg,
                inclination_deg: row.inclination_deg,
                northing_offset: row.northing_offset,
                easting_offset: row.easting_offset,
            })
            .collect(),
    )
}

fn top_rows_for_compute(rows: &[TopRow]) -> Vec<TopDataRow> {
    rows.iter()
        .cloned()
        .map(|row| TopDataRow {
            name: row.name,
            top_depth: row.top_depth,
            base_depth: row.base_depth,
            source: row.source,
            source_depth_reference: row.source_depth_reference,
            depth_domain: row.depth_domain,
            depth_datum: row.depth_datum,
        })
        .collect()
}

fn top_rows_from_compute(rows: &[TopDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::TopSet(
        rows.iter()
            .cloned()
            .map(|row| TopRow {
                name: row.name,
                top_depth: row.top_depth,
                base_depth: row.base_depth,
                source: row.source,
                source_depth_reference: row.source_depth_reference,
                depth_domain: row.depth_domain,
                depth_datum: row.depth_datum,
            })
            .collect(),
    )
}

fn pressure_rows_for_compute(rows: &[PressureObservationRow]) -> Vec<PressureObservationDataRow> {
    rows.iter()
        .cloned()
        .map(|row| PressureObservationDataRow {
            measured_depth: row.measured_depth,
            pressure: row.pressure,
            phase: row.phase,
            test_kind: row.test_kind,
            timestamp: row.timestamp,
        })
        .collect()
}

fn pressure_rows_from_compute(rows: &[PressureObservationDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::Pressure(
        rows.iter()
            .cloned()
            .map(|row| PressureObservationRow {
                measured_depth: row.measured_depth,
                pressure: row.pressure,
                phase: row.phase,
                test_kind: row.test_kind,
                timestamp: row.timestamp,
            })
            .collect(),
    )
}

fn drilling_rows_for_compute(rows: &[DrillingObservationRow]) -> Vec<DrillingObservationDataRow> {
    rows.iter()
        .cloned()
        .map(|row| DrillingObservationDataRow {
            measured_depth: row.measured_depth,
            event_kind: row.event_kind,
            value: row.value,
            unit: row.unit,
            timestamp: row.timestamp,
            comment: row.comment,
        })
        .collect()
}

fn drilling_rows_from_compute(rows: &[DrillingObservationDataRow]) -> StructuredComputedRows {
    StructuredComputedRows::Drilling(
        rows.iter()
            .cloned()
            .map(|row| DrillingObservationRow {
                measured_depth: row.measured_depth,
                event_kind: row.event_kind,
                value: row.value,
                unit: row.unit,
                timestamp: row.timestamp,
                comment: row.comment,
            })
            .collect(),
    )
}

fn build_derived_log_file(
    source_file: &LasFile,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    computed_curve: &ophiolite_compute::ComputedCurve,
    execution: &ComputeExecutionManifest,
) -> LasFile {
    let mut derived = source_file.clone();
    let index_curve = source_file
        .curve(&source_file.index.curve_id)
        .cloned()
        .unwrap();
    let curve_item = CurveItem::new(
        computed_curve.curve_name.clone(),
        computed_curve.unit.clone().unwrap_or_default(),
        LasValue::Empty,
        computed_curve.description.clone().unwrap_or_default(),
        computed_curve
            .values
            .iter()
            .map(|value| match value {
                Some(number) => LasValue::Number(*number),
                None => LasValue::Empty,
            })
            .collect(),
    );
    derived.curves = SectionItems::from_items(
        vec![index_curve, curve_item],
        source_file.curves.mnemonic_case,
    );
    derived.summary.source_path = source_asset.package_path.clone();
    derived.summary.original_filename = format!(
        "{}-{}.las",
        collection.name.replace(' ', "_"),
        storage_asset_id.0
    );
    derived.summary.source_fingerprint =
        revision_token_for_bytes("compute", &execution.function_id).0;
    derived.summary.curve_count = derived.curves.len();
    derived.summary.row_count = derived.row_count();
    derived.summary.issue_count = derived.issues.len();
    derived.provenance = Provenance {
        source_path: source_asset.package_path.clone(),
        original_filename: derived.summary.original_filename.clone(),
        source_fingerprint: derived.summary.source_fingerprint.clone(),
        imported_at_unix_seconds: execution.executed_at_unix_seconds,
    };
    derived
}

fn log_asset_manifest(
    file: &LasFile,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
) -> AssetManifest {
    let metadata = package_metadata_for(file, 1);
    let imported_at = now_unix_seconds();
    AssetManifest {
        asset_kind: AssetKind::Log,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: file.provenance.source_path.clone(),
            original_filename: file.provenance.original_filename.clone(),
            source_fingerprint: file.provenance.source_fingerprint.clone(),
        }],
        provenance: file.provenance.clone(),
        diagnostics: file.issues.clone(),
        extents: AssetExtent {
            index_kind: Some(file.index.kind.clone()),
            start: metadata.canonical.well.start,
            stop: metadata.canonical.well.stop,
            row_count: Some(file.row_count()),
        },
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: "curves.parquet".to_string(),
                media_type: "application/vnd.apache.parquet".to_string(),
                role: "curve_samples".to_string(),
            },
        ],
        reference_metadata: AssetReferenceMetadata {
            identifiers: identifiers_from_well_info(&metadata.canonical.well),
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: DepthReference::MeasuredDepth,
            unit_system: UnitSystem {
                depth_unit: metadata.storage.index_unit.clone(),
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: classify_log_curves_from_file(file),
        compute_manifest: None,
    }
}

fn structured_asset_manifest(
    source_path: &Path,
    metadata: &AssetTableMetadata,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    asset_kind: AssetKind,
    extent: AssetExtent,
    identifiers: WellIdentifierSet,
    coordinate_reference: Option<CoordinateReference>,
    supersedes: Option<AssetId>,
) -> Result<AssetManifest> {
    let imported_at = now_unix_seconds();
    let source_bytes = fs::read(source_path)?;
    let fingerprint = source_fingerprint(&source_bytes);
    let provenance = Provenance::from_path(source_path, fingerprint.clone(), imported_at);
    Ok(AssetManifest {
        asset_kind: asset_kind.clone(),
        asset_schema_version: metadata.schema_version.clone(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
        }],
        provenance,
        diagnostics: Vec::new(),
        extents: extent,
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: data_filename().to_string(),
                media_type: "application/vnd.apache.parquet".to_string(),
                role: "bulk_data".to_string(),
            },
        ],
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference,
            vertical_datum: vertical_datum_for_kind(&asset_kind),
            depth_reference: depth_reference_for_kind(&asset_kind),
            unit_system: UnitSystem {
                depth_unit: None,
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn raw_source_bundle_manifest(
    source_paths: &[&Path],
    source_artifacts: Vec<SourceArtifactRef>,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    identifiers: WellIdentifierSet,
    supersedes: Option<AssetId>,
    package_root: &Path,
) -> Result<AssetManifest> {
    let primary_source = source_paths.first().copied().ok_or_else(|| {
        LasError::Validation(
            "raw source bundle import requires at least one source file".to_string(),
        )
    })?;
    let imported_at = now_unix_seconds();
    let source_bytes = fs::read(primary_source)?;
    let provenance = Provenance::from_path(
        primary_source,
        source_fingerprint(&source_bytes),
        imported_at,
    );
    let mut bulk_data_descriptors = vec![BulkDataDescriptor {
        relative_path: "metadata.json".to_string(),
        media_type: "application/json".to_string(),
        role: "metadata".to_string(),
    }];
    bulk_data_descriptors.extend(copy_supporting_source_artifacts(
        package_root,
        source_paths,
    )?);
    Ok(AssetManifest {
        asset_kind: AssetKind::RawSourceBundle,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts,
        provenance,
        diagnostics: Vec::new(),
        extents: AssetExtent {
            index_kind: None,
            start: None,
            stop: None,
            row_count: None,
        },
        bulk_data_descriptors,
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: DepthReference::Unknown,
            unit_system: UnitSystem {
                depth_unit: None,
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn source_artifact_refs_for_paths(
    source_paths: &[&Path],
    imported_at_unix_seconds: u64,
) -> Result<Vec<SourceArtifactRef>> {
    let mut refs = Vec::new();
    for path in source_paths {
        if !path.exists() {
            continue;
        }
        let bytes = fs::read(path)?;
        refs.push(SourceArtifactRef {
            source_path: path.to_string_lossy().into_owned(),
            original_filename: path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| "source".to_string()),
            source_fingerprint: source_fingerprint(&bytes),
        });
    }
    refs.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    refs.dedup_by(|left, right| left.source_path == right.source_path);
    let _ = imported_at_unix_seconds;
    Ok(refs)
}

fn copy_supporting_source_artifacts(
    package_root: &Path,
    source_paths: &[&Path],
) -> Result<Vec<BulkDataDescriptor>> {
    let mut descriptors = Vec::new();
    let mut used_relative_paths = BTreeMap::<String, usize>::new();
    for path in source_paths {
        if !path.exists() {
            continue;
        }
        let original_name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "source".to_string());
        let duplicate_count = used_relative_paths
            .entry(original_name.clone())
            .and_modify(|count| *count += 1)
            .or_insert(0);
        let relative_path = if *duplicate_count == 0 {
            format!("sources/{original_name}")
        } else {
            format!("sources/{}-{}", duplicate_count, original_name)
        };
        copy_path(path, &package_root.join(&relative_path))?;
        descriptors.push(BulkDataDescriptor {
            relative_path,
            media_type: supporting_source_media_type(path).to_string(),
            role: "source_artifact".to_string(),
        });
    }
    Ok(descriptors)
}

fn supporting_source_media_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("txt") => "text/plain",
        Some("asc") => "text/plain",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("las") => "application/x-las",
        Some("nlog") => "text/plain",
        Some("dlis") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

fn well_time_depth_model_manifest(
    source_path: &Path,
    model: &WellTimeDepthModel1D,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    identifiers: WellIdentifierSet,
    supersedes: Option<AssetId>,
) -> Result<AssetManifest> {
    let imported_at = now_unix_seconds();
    let source_bytes = fs::read(source_path)?;
    let fingerprint = source_fingerprint(&source_bytes);
    let provenance = Provenance::from_path(source_path, fingerprint.clone(), imported_at);
    let start = model.samples.first().map(|sample| f64::from(sample.depth));
    let stop = model.samples.last().map(|sample| f64::from(sample.depth));
    Ok(AssetManifest {
        asset_kind: AssetKind::WellTimeDepthModel,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
        }],
        provenance,
        diagnostics: Vec::new(),
        extents: AssetExtent {
            index_kind: Some(IndexKind::Depth),
            start,
            stop,
            row_count: Some(model.samples.len()),
        },
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: WELL_TIME_DEPTH_MODEL_FILENAME.to_string(),
                media_type: "application/json".to_string(),
                role: "well_time_depth_model".to_string(),
            },
        ],
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: project_depth_reference_from_model(model.depth_reference),
            unit_system: UnitSystem {
                depth_unit: Some("m".to_string()),
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn well_time_depth_json_manifest(
    source_path: &Path,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    asset_kind: AssetKind,
    identifiers: WellIdentifierSet,
    supersedes: Option<AssetId>,
) -> Result<AssetManifest> {
    let imported_at = now_unix_seconds();
    let source_bytes = fs::read(source_path)?;
    let fingerprint = source_fingerprint(&source_bytes);
    let provenance = Provenance::from_path(source_path, fingerprint.clone(), imported_at);
    let payload_filename = match asset_kind {
        AssetKind::CheckshotVspObservationSet => CHECKSHOT_VSP_OBSERVATION_SET_FILENAME,
        AssetKind::ManualTimeDepthPickSet => MANUAL_TIME_DEPTH_PICK_SET_FILENAME,
        AssetKind::WellTieObservationSet => WELL_TIE_OBSERVATION_SET_FILENAME,
        AssetKind::WellTimeDepthAuthoredModel => WELL_TIME_DEPTH_AUTHORED_MODEL_FILENAME,
        _ => {
            return Err(LasError::Validation(format!(
                "asset kind '{}' is not supported by the well time-depth json manifest helper",
                asset_kind.as_str()
            )));
        }
    };
    Ok(AssetManifest {
        asset_kind,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
        }],
        provenance,
        diagnostics: Vec::new(),
        extents: AssetExtent {
            index_kind: Some(IndexKind::Depth),
            start: None,
            stop: None,
            row_count: None,
        },
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: payload_filename.to_string(),
                media_type: "application/json".to_string(),
                role: "payload".to_string(),
            },
        ],
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: DepthReference::Unknown,
            unit_system: UnitSystem {
                depth_unit: Some("m".to_string()),
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn seismic_asset_manifest(
    source_root: &Path,
    metadata: &SeismicAssetMetadata,
    well_id: &WellId,
    wellbore_id: &WellboreId,
    collection_id: &AssetCollectionId,
    logical_asset_id: &AssetId,
    storage_asset_id: &AssetId,
    asset_kind: AssetKind,
    identifiers: WellIdentifierSet,
    supersedes: Option<AssetId>,
) -> Result<AssetManifest> {
    let imported_at = now_unix_seconds();
    let source_fingerprint = stable_project_path_hash("seismic-source", source_root)?;
    let provenance = Provenance {
        source_path: source_root.display().to_string(),
        original_filename: source_root
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| asset_kind.as_str().to_string()),
        source_fingerprint,
        imported_at_unix_seconds: imported_at,
    };
    Ok(AssetManifest {
        asset_kind,
        asset_schema_version: "0.1.0".to_string(),
        logical_asset_id: logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: well_id.clone(),
        wellbore_id: wellbore_id.clone(),
        asset_collection_id: collection_id.clone(),
        source_artifacts: vec![SourceArtifactRef {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
        }],
        provenance,
        diagnostics: Vec::new(),
        extents: seismic_asset_extent(metadata),
        bulk_data_descriptors: vec![BulkDataDescriptor {
            relative_path: "store".to_string(),
            media_type: "application/vnd.ophiolite.tbvol".to_string(),
            role: "seismic_store".to_string(),
        }],
        reference_metadata: AssetReferenceMetadata {
            identifiers,
            coordinate_reference: None,
            vertical_datum: None,
            depth_reference: DepthReference::Unknown,
            unit_system: UnitSystem {
                depth_unit: None,
                coordinate_unit: None,
                pressure_unit: None,
            },
        },
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: None,
        curve_semantics: Vec::new(),
        compute_manifest: None,
    })
}

fn derived_log_asset_manifest(
    file: &LasFile,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    computed_curve: &ophiolite_compute::ComputedCurve,
    execution: &ComputeExecutionManifest,
) -> AssetManifest {
    let mut manifest = log_asset_manifest(
        file,
        &source_asset.well_id,
        &source_asset.wellbore_id,
        &collection.id,
        &collection.logical_asset_id,
        storage_asset_id,
        supersedes,
    );
    manifest.asset_schema_version = "0.2.0".to_string();
    manifest.source_artifacts = source_asset.manifest.source_artifacts.clone();
    manifest.reference_metadata = source_asset.manifest.reference_metadata.clone();
    manifest.derived_from = Some(source_asset.logical_asset_id.clone());
    manifest.curve_semantics = vec![
        CurveSemanticDescriptor {
            curve_name: file.index.curve_id.clone(),
            original_mnemonic: file.index.raw_mnemonic.clone(),
            unit: (!file.index.unit.trim().is_empty()).then_some(file.index.unit.clone()),
            semantic_type: classify_curve_semantic(
                &derive_canonical_alias(&file.index.raw_mnemonic, &file.index.unit),
                &file.index.raw_mnemonic,
                Some(&file.index.unit),
                true,
            ),
            source: CurveSemanticSource::Derived,
            semantic_parameters: BTreeMap::new(),
        },
        CurveSemanticDescriptor {
            curve_name: computed_curve.curve_name.clone(),
            original_mnemonic: computed_curve.original_mnemonic.clone(),
            unit: computed_curve.unit.clone(),
            semantic_type: computed_curve.semantic_type.clone(),
            source: CurveSemanticSource::Computed,
            semantic_parameters: computed_curve.semantic_parameters.clone(),
        },
    ];
    manifest.compute_manifest = Some(execution.clone());
    manifest
}

fn write_structured_compute_rows(
    package_root: &Path,
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    rows: &StructuredComputedRows,
    execution: &ComputeExecutionManifest,
    asset_kind: AssetKind,
) -> Result<AssetManifest> {
    match rows {
        StructuredComputedRows::Trajectory(rows) => {
            write_trajectory_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                trajectory_metadata(rows),
                structured_asset_extent(AssetKind::Trajectory, trajectory_extent(rows)),
            )
        }
        StructuredComputedRows::TopSet(rows) => {
            write_tops_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                tops_metadata(rows),
                structured_asset_extent(AssetKind::TopSet, tops_extent(rows)),
            )
        }
        StructuredComputedRows::Pressure(rows) => {
            write_pressure_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                pressure_metadata(rows),
                structured_asset_extent(AssetKind::PressureObservation, pressure_extent(rows)),
            )
        }
        StructuredComputedRows::Drilling(rows) => {
            write_drilling_package(package_root, rows)?;
            derived_structured_asset_manifest(
                source_asset,
                collection,
                storage_asset_id,
                supersedes,
                execution,
                asset_kind,
                drilling_metadata(rows),
                structured_asset_extent(AssetKind::DrillingObservation, drilling_extent(rows)),
            )
        }
    }
}

fn derived_structured_asset_manifest(
    source_asset: &AssetRecord,
    collection: &AssetCollectionRecord,
    storage_asset_id: &AssetId,
    supersedes: Option<AssetId>,
    execution: &ComputeExecutionManifest,
    asset_kind: AssetKind,
    metadata: AssetTableMetadata,
    extent: AssetExtent,
) -> Result<AssetManifest> {
    let imported_at = execution.executed_at_unix_seconds;
    let mut manifest = AssetManifest {
        asset_kind: asset_kind.clone(),
        asset_schema_version: metadata.schema_version.clone(),
        logical_asset_id: collection.logical_asset_id.clone(),
        storage_asset_id: storage_asset_id.clone(),
        well_id: source_asset.well_id.clone(),
        wellbore_id: source_asset.wellbore_id.clone(),
        asset_collection_id: collection.id.clone(),
        source_artifacts: source_asset.manifest.source_artifacts.clone(),
        provenance: Provenance {
            source_path: source_asset.package_path.clone(),
            original_filename: format!("derived-{}", execution.function_id),
            source_fingerprint: revision_token_for_bytes("compute", &execution.function_id).0,
            imported_at_unix_seconds: imported_at,
        },
        diagnostics: Vec::new(),
        extents: extent,
        bulk_data_descriptors: vec![
            BulkDataDescriptor {
                relative_path: "metadata.json".to_string(),
                media_type: "application/json".to_string(),
                role: "metadata".to_string(),
            },
            BulkDataDescriptor {
                relative_path: data_filename().to_string(),
                media_type: "application/vnd.apache.parquet".to_string(),
                role: "bulk_data".to_string(),
            },
        ],
        reference_metadata: source_asset.manifest.reference_metadata.clone(),
        created_at_unix_seconds: imported_at,
        imported_at_unix_seconds: imported_at,
        supersedes,
        derived_from: Some(source_asset.logical_asset_id.clone()),
        curve_semantics: Vec::new(),
        compute_manifest: Some(execution.clone()),
    };
    manifest.source_artifacts = source_asset.manifest.source_artifacts.clone();
    Ok(manifest)
}

fn write_asset_manifest(root: &Path, manifest: &AssetManifest) -> Result<()> {
    fs::write(
        root.join(ASSET_MANIFEST_FILENAME),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn read_project_manifest(root: &Path) -> Result<OphioliteProjectManifest> {
    let manifest_path = root.join(PROJECT_MANIFEST_FILENAME);
    serde_json::from_str(&fs::read_to_string(manifest_path)?).map_err(Into::into)
}

fn write_project_manifest(root: &Path, manifest: &OphioliteProjectManifest) -> Result<()> {
    fs::write(
        root.join(PROJECT_MANIFEST_FILENAME),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn builtin_operator_lock_entry() -> OperatorPackageLockEntry {
    OperatorPackageLockEntry {
        package_name: BUILTIN_OPERATOR_PACKAGE_NAME.to_string(),
        package_version: env!("CARGO_PKG_VERSION").to_string(),
        provider: "ophiolite".to_string(),
        runtime: OperatorRuntimeKind::Rust,
        source_kind: OperatorPackageSourceKind::BuiltIn,
        source_reference: None,
    }
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn copy_operator_package_entrypoint(
    source_root: &Path,
    install_root: &Path,
    entrypoint: &str,
) -> Result<String> {
    let Some((source_path, install_relative_path, preserve_entrypoint)) =
        resolve_operator_package_entrypoint_copy(source_root, entrypoint)?
    else {
        return Ok(entrypoint.to_string());
    };
    let install_path = install_root.join(&install_relative_path);

    if source_path.is_dir() {
        copy_directory_recursive(&source_path, &install_path)?;
    } else {
        if let Some(parent) = install_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source_path, &install_path)?;
    }

    Ok(if preserve_entrypoint {
        entrypoint.to_string()
    } else {
        normalize_relative_path(&install_relative_path)
    })
}

fn copy_operator_package_support_files(source_root: &Path, install_root: &Path) -> Result<()> {
    for name in [
        "requirements.txt",
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "MANIFEST.in",
    ] {
        let source_path = source_root.join(name);
        if source_path.exists() {
            fs::copy(&source_path, install_root.join(name))?;
        }
    }
    Ok(())
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn resolve_operator_package_entrypoint_copy(
    source_root: &Path,
    entrypoint: &str,
) -> Result<Option<(PathBuf, PathBuf, bool)>> {
    let entrypoint_path = PathBuf::from(entrypoint);
    let module_path = module_entrypoint_path(entrypoint);
    let is_module_style = !entrypoint_path.is_absolute()
        && !entrypoint.contains('/')
        && !entrypoint.contains('\\')
        && entrypoint_path.extension().is_none();

    let candidates = if is_module_style {
        vec![
            (
                source_root.join(module_path.with_extension("py")),
                module_path.with_extension("py"),
                true,
            ),
            (source_root.join(&module_path), module_path.clone(), true),
        ]
    } else {
        let joined = if entrypoint_path.is_absolute() {
            entrypoint_path.clone()
        } else {
            source_root.join(&entrypoint_path)
        };
        let relative = if entrypoint_path.is_absolute() {
            PathBuf::from(
                joined
                    .file_name()
                    .ok_or_else(|| {
                        LasError::Validation(format!(
                            "python operator entrypoint '{}' does not have a filename",
                            joined.display()
                        ))
                    })?
                    .to_string_lossy()
                    .into_owned(),
            )
        } else {
            entrypoint_path.clone()
        };
        vec![
            (joined.clone(), relative.clone(), false),
            (
                joined.with_extension("py"),
                relative.with_extension("py"),
                false,
            ),
        ]
    };

    for (source_path, install_relative_path, preserve_entrypoint) in candidates {
        if source_path.exists() {
            return Ok(Some((
                source_path,
                install_relative_path,
                preserve_entrypoint,
            )));
        }
    }
    Ok(None)
}

fn module_entrypoint_path(entrypoint: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in entrypoint.split('.') {
        path.push(segment);
    }
    path
}

fn ensure_python_operator_environment(install_root: &Path) -> Result<PathBuf> {
    let env_root = install_root.join(PROJECT_OPERATOR_PACKAGE_PYTHON_ENV_DIRNAME);
    if env_root.exists() {
        fs::remove_dir_all(&env_root)?;
    }

    let mut create_command = Command::new(external_python_bin());
    create_command.arg("-m").arg("venv").arg(&env_root);
    run_python_command(create_command, "create python operator environment")?;
    let env_python = python_virtualenv_executable(&env_root);
    if !env_python.exists() {
        return Err(LasError::Validation(format!(
            "python operator environment '{}' is missing its interpreter",
            env_root.display()
        )));
    }

    let requirements_path = install_root.join("requirements.txt");
    if requirements_path.exists() {
        let mut command = Command::new(&env_python);
        command
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("-r")
            .arg(&requirements_path)
            .current_dir(install_root);
        run_python_command(command, "install python operator requirements")?;
    } else if ["pyproject.toml", "setup.py", "setup.cfg"]
        .iter()
        .any(|name| install_root.join(name).exists())
    {
        let mut command = Command::new(&env_python);
        command
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg(".")
            .current_dir(install_root);
        run_python_command(command, "install python operator package")?;
    }

    Ok(env_root)
}

fn run_python_command(mut command: Command, action: &str) -> Result<()> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let detail = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stderr}\n{stdout}")
    };
    Err(LasError::Validation(format!(
        "failed to {action}: {detail}"
    )))
}

fn structured_compute_input_binding(parameter_name: &str, curve_name: &str) -> ComputeInputBinding {
    ComputeInputBinding {
        parameter_name: parameter_name.to_string(),
        curve_name: curve_name.to_string(),
        semantic_type: CurveSemanticType::Computed,
    }
}

fn well_marker_horizon_residual_catalog_entry(asset_kind: AssetKind) -> ComputeCatalogEntry {
    let (input_specs, provider, tags) = match asset_kind {
        AssetKind::TopSet => (
            vec![ComputeInputSpec::TopSet],
            "tops",
            vec![
                "tops".to_string(),
                "horizon".to_string(),
                "residual".to_string(),
            ],
        ),
        AssetKind::WellMarkerSet => (
            vec![ComputeInputSpec::WellMarkerSet],
            "well_markers",
            vec![
                "well-markers".to_string(),
                "horizon".to_string(),
                "residual".to_string(),
            ],
        ),
        _ => (Vec::new(), "well_markers", vec!["residual".to_string()]),
    };
    ComputeCatalogEntry {
        metadata: ophiolite_compute::ComputeFunctionMetadata {
            id: WELL_MARKER_HORIZON_RESIDUAL_FUNCTION_ID.to_string(),
            provider: provider.to_string(),
            name: "Compute Depth-Horizon Residuals".to_string(),
            category: "Interpretation".to_string(),
            description: "Sample a depth-domain horizon at each marker XY and compute residual = marker TVD - horizon depth.".to_string(),
            default_output_mnemonic: "marker_horizon_residuals".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags,
        },
        input_specs,
        parameters: vec![
            ComputeParameterDefinition::String {
                name: WELL_MARKER_HORIZON_RESIDUAL_SURVEY_ASSET_PARAMETER.to_string(),
                label: "Survey Asset Id".to_string(),
                description: "Seismic survey asset that owns the imported depth horizon."
                    .to_string(),
                default: None,
            },
            ComputeParameterDefinition::String {
                name: WELL_MARKER_HORIZON_RESIDUAL_HORIZON_PARAMETER.to_string(),
                label: "Horizon Id".to_string(),
                description: "Imported horizon identifier inside the selected survey asset."
                    .to_string(),
                default: None,
            },
            ComputeParameterDefinition::String {
                name: WELL_MARKER_HORIZON_RESIDUAL_MARKER_PARAMETER.to_string(),
                label: "Marker Name".to_string(),
                description:
                    "Optional canonical marker name to filter before computing residuals."
                        .to_string(),
                default: None,
            },
        ],
        binding_candidates: Vec::new(),
        availability: ComputeAvailability::Available,
    }
}

fn builtin_structured_execution_manifest(
    function_id: &str,
    provider: &str,
    function_name: &str,
    inputs: Vec<ComputeInputBinding>,
    parameters: &BTreeMap<String, ComputeParameterValue>,
    output_curve_name: &str,
) -> ComputeExecutionManifest {
    ComputeExecutionManifest {
        function_id: function_id.to_string(),
        provider: provider.to_string(),
        function_name: function_name.to_string(),
        function_version: WELL_MARKER_HORIZON_RESIDUAL_FUNCTION_VERSION.to_string(),
        operator_package: Some(BUILTIN_OPERATOR_PACKAGE_NAME.to_string()),
        operator_package_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        operator_runtime: Some(OperatorRuntimeKind::Rust),
        deterministic: true,
        source_asset_id: String::new(),
        source_logical_asset_id: String::new(),
        inputs,
        parameters: parameters.clone(),
        output_curve_name: output_curve_name.to_string(),
        output_curve_type: CurveSemanticType::Computed,
        executed_at_unix_seconds: 0,
    }
}

fn external_execution_manifest(
    package: &OperatorPackageManifest,
    operator: &OperatorManifest,
    inputs: Vec<ComputeInputBinding>,
    parameters: &BTreeMap<String, ComputeParameterValue>,
    output_curve_name: impl Into<String>,
    output_curve_type: CurveSemanticType,
) -> ComputeExecutionManifest {
    ComputeExecutionManifest {
        function_id: operator.id.clone(),
        provider: operator.provider.clone(),
        function_name: operator.name.clone(),
        function_version: package.package_version.clone(),
        operator_package: Some(package.package_name.clone()),
        operator_package_version: Some(package.package_version.clone()),
        operator_runtime: Some(package.runtime.clone()),
        deterministic: operator.deterministic,
        source_asset_id: String::new(),
        source_logical_asset_id: String::new(),
        inputs,
        parameters: parameters.clone(),
        output_curve_name: output_curve_name.into(),
        output_curve_type,
        executed_at_unix_seconds: 0,
    }
}

fn external_python_bin() -> String {
    if let Ok(bin) = std::env::var("OPHIOLITE_PYTHON_BIN") {
        return bin;
    }
    let candidates: &[&str] = if cfg!(windows) {
        &["python.exe", "python3.exe", "python", "python3"]
    } else {
        &["python", "python3"]
    };
    for candidate in candidates {
        if executable_exists_in_path(candidate) {
            return (*candidate).to_string();
        }
    }
    if cfg!(windows) {
        "python.exe".to_string()
    } else {
        "python".to_string()
    }
}

fn executable_exists_in_path(candidate: &str) -> bool {
    let path = Path::new(candidate);
    if path.components().count() > 1 {
        return path.exists();
    }
    let Some(path_env) = std::env::var_os("PATH") else {
        return false;
    };
    for directory in std::env::split_paths(&path_env) {
        if directory.join(candidate).is_file() {
            return true;
        }
    }
    false
}

fn python_runtime_for_operator_manifest_dir(manifest_dir: &Path) -> PathBuf {
    let env_python = python_virtualenv_executable(
        &manifest_dir.join(PROJECT_OPERATOR_PACKAGE_PYTHON_ENV_DIRNAME),
    );
    if env_python.exists() {
        env_python
    } else {
        PathBuf::from(external_python_bin())
    }
}

fn python_virtualenv_executable(env_root: &Path) -> PathBuf {
    if cfg!(windows) {
        env_root.join("Scripts").join("python.exe")
    } else {
        env_root.join("bin").join("python")
    }
}

fn external_pythonpath() -> Result<OsString> {
    let mut paths = vec![external_python_support_path()?];
    if let Some(existing) = std::env::var_os("PYTHONPATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).map_err(|error| {
        LasError::Validation(format!(
            "failed to build PYTHONPATH for external operators: {error}"
        ))
    })
}

fn external_python_support_path() -> Result<PathBuf> {
    let path = workspace_root_path()?.join("python").join("src");
    if path.exists() {
        Ok(path)
    } else {
        Err(LasError::Validation(format!(
            "python sdk source directory '{}' was not found",
            path.display()
        )))
    }
}

fn workspace_root_path() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.clone(),
        manifest_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| manifest_dir.clone()),
        manifest_dir
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| manifest_dir.clone()),
    ];
    for candidate in candidates {
        if candidate.join("Cargo.toml").exists() && candidate.join("python").exists() {
            return Ok(candidate);
        }
    }
    Err(LasError::Validation(format!(
        "failed to resolve workspace root from '{}'",
        manifest_dir.display()
    )))
}

fn asset_semantic_family(asset_kind: &AssetKind) -> Result<AssetSemanticFamily> {
    match asset_kind {
        AssetKind::Log => Ok(AssetSemanticFamily::Log),
        AssetKind::Trajectory => Ok(AssetSemanticFamily::Trajectory),
        AssetKind::TopSet => Ok(AssetSemanticFamily::TopSet),
        AssetKind::WellMarkerSet => Ok(AssetSemanticFamily::WellMarkerSet),
        AssetKind::WellMarkerHorizonResidualSet => Err(LasError::Validation(
            "compute catalog is not implemented for well marker horizon residual assets"
                .to_string(),
        )),
        AssetKind::PressureObservation => Ok(AssetSemanticFamily::PressureObservation),
        AssetKind::DrillingObservation => Ok(AssetSemanticFamily::DrillingObservation),
        AssetKind::CheckshotVspObservationSet
        | AssetKind::ManualTimeDepthPickSet
        | AssetKind::WellTieObservationSet
        | AssetKind::WellTimeDepthAuthoredModel
        | AssetKind::RawSourceBundle => Err(LasError::Validation(
            "compute catalog is not implemented for well time-depth observation/model assets"
                .to_string(),
        )),
        AssetKind::WellTimeDepthModel => Err(LasError::Validation(
            "compute catalog is not implemented for well time-depth model assets".to_string(),
        )),
        AssetKind::SeismicTraceData => Err(LasError::Validation(
            "compute catalog is not implemented for seismic assets yet".to_string(),
        )),
    }
}

fn external_operator_catalog_entries(
    packages: &[OperatorPackageManifest],
    asset_family: &AssetSemanticFamily,
    log_context: Option<(&[CurveSemanticDescriptor], &[String])>,
) -> Vec<ComputeCatalogEntry> {
    packages
        .iter()
        .filter(|package| package.package_name != BUILTIN_OPERATOR_PACKAGE_NAME)
        .flat_map(|package| {
            package
                .operators
                .iter()
                .filter(move |operator| &operator.asset_family == asset_family)
                .map(move |operator| catalog_entry_for_operator_manifest(operator, log_context))
        })
        .collect()
}

fn identifiers_from_well_info(info: &WellInfo) -> WellIdentifierSet {
    WellIdentifierSet {
        primary_name: info.well.clone(),
        uwi: info.uwi.clone(),
        api: info.api.clone(),
        operator_aliases: info
            .company
            .clone()
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect(),
    }
}

fn operator_history_from_name(name: Option<String>, source: &str) -> Vec<OperatorAssignment> {
    name.into_iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| OperatorAssignment {
            organisation_name: Some(value),
            organisation_id: None,
            effective_at: None,
            terminated_at: None,
            source: Some(source.to_string()),
            note: None,
        })
        .collect()
}

fn well_metadata_from_well_info(info: &WellInfo) -> Option<WellMetadata> {
    let metadata = WellMetadata {
        field_name: info.field.clone(),
        block_name: None,
        basin_name: None,
        country: None,
        province_state: info.province.clone(),
        location_text: info.location.clone(),
        interest_type: None,
        operator_history: operator_history_from_name(info.company.clone(), "las_header"),
        surface_location: None,
        default_vertical_measurement_id: None,
        default_vertical_coordinate_reference: None,
        vertical_measurements: Vec::new(),
        external_references: Vec::new(),
        notes: Vec::new(),
    };
    if metadata.field_name.is_none()
        && metadata.province_state.is_none()
        && metadata.location_text.is_none()
        && metadata.operator_history.is_empty()
    {
        None
    } else {
        Some(metadata)
    }
}

fn wellbore_metadata_from_well_info(info: &WellInfo) -> Option<WellboreMetadata> {
    let metadata = WellboreMetadata {
        sequence_number: None,
        status: None,
        purpose: None,
        trajectory_type: None,
        parent_wellbore_id: None,
        target_formation: None,
        primary_material: None,
        location_text: info.location.clone(),
        service_company_name: info.service_company.clone(),
        operator_history: operator_history_from_name(info.company.clone(), "las_header"),
        bottom_hole_location: None,
        default_vertical_measurement_id: None,
        default_vertical_coordinate_reference: None,
        vertical_measurements: Vec::new(),
        external_references: Vec::new(),
        notes: Vec::new(),
    };
    if metadata.location_text.is_none()
        && metadata.service_company_name.is_none()
        && metadata.operator_history.is_empty()
    {
        None
    } else {
        Some(metadata)
    }
}

fn vertical_measurement_path_from_depth_domain(
    depth_domain: Option<&str>,
) -> VerticalMeasurementPath {
    match normalized_marker_depth_domain(depth_domain) {
        Some("md") => VerticalMeasurementPath::MeasuredDepth,
        Some("tvd") => VerticalMeasurementPath::TrueVerticalDepth,
        Some("tvdss") => VerticalMeasurementPath::TrueVerticalDepthSubsea,
        Some("elevation") => VerticalMeasurementPath::Elevation,
        Some(_) | None => VerticalMeasurementPath::Unknown,
    }
}

fn canonical_markers_from_top_rows(
    wellbore_id: &WellboreId,
    source_asset_id: &AssetId,
    rows: &[TopRow],
) -> Vec<WellMarkerRecord> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let sequence_number = u32::try_from(index + 1).ok();
            let marker_id = WellMarkerId(
                revision_token_for_bytes(
                    "well-marker",
                    &format!(
                        "{}:{}:{}:{}:{}",
                        wellbore_id.0, source_asset_id.0, index, row.name, row.top_depth
                    ),
                )
                .0,
            );
            let measurement_path =
                vertical_measurement_path_from_depth_domain(row.depth_domain.as_deref());
            WellMarkerRecord {
                id: marker_id.clone(),
                wellbore_id: wellbore_id.clone(),
                source_asset_id: Some(source_asset_id.clone()),
                name: row.name.clone(),
                sequence_number,
                marker_kind: Some("top".to_string()),
                top_measurement: VerticalMeasurement {
                    measurement_id: Some(format!("{}:top", marker_id.0.clone())),
                    value: row.top_depth,
                    unit: None,
                    path: measurement_path.clone(),
                    coordinate_reference: None,
                    reference_measurement_id: None,
                    reference_entity_id: None,
                    source: row.source.clone(),
                    description: row.source_depth_reference.clone(),
                },
                base_measurement: row.base_depth.map(|value| VerticalMeasurement {
                    measurement_id: Some(format!("{}:base", marker_id.0.clone())),
                    value,
                    unit: None,
                    path: measurement_path,
                    coordinate_reference: None,
                    reference_measurement_id: None,
                    reference_entity_id: None,
                    source: row.source.clone(),
                    description: row.source_depth_reference.clone(),
                }),
                depth_reference: row.source_depth_reference.clone(),
                source: row.source.clone(),
                external_references: Vec::new(),
                notes: Vec::new(),
            }
        })
        .collect()
}

fn canonical_markers_from_well_marker_rows(
    wellbore_id: &WellboreId,
    source_asset_id: &AssetId,
    rows: &[WellMarkerRow],
) -> Vec<WellMarkerRecord> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let sequence_number = u32::try_from(index + 1).ok();
            let marker_id = WellMarkerId(
                revision_token_for_bytes(
                    "well-marker",
                    &format!(
                        "{}:{}:{}:{}:{}:{}",
                        wellbore_id.0,
                        source_asset_id.0,
                        index,
                        row.name,
                        row.marker_kind.as_deref().unwrap_or(""),
                        row.top_depth
                    ),
                )
                .0,
            );
            let measurement_path =
                vertical_measurement_path_from_depth_domain(row.depth_domain.as_deref());
            let mut notes = Vec::new();
            if let Some(note) = row.note.as_ref().filter(|value| !value.trim().is_empty()) {
                notes.push(note.clone());
            }
            WellMarkerRecord {
                id: marker_id.clone(),
                wellbore_id: wellbore_id.clone(),
                source_asset_id: Some(source_asset_id.clone()),
                name: row.name.clone(),
                sequence_number,
                marker_kind: row.marker_kind.clone(),
                top_measurement: VerticalMeasurement {
                    measurement_id: Some(format!("{}:top", marker_id.0.clone())),
                    value: row.top_depth,
                    unit: None,
                    path: measurement_path.clone(),
                    coordinate_reference: None,
                    reference_measurement_id: None,
                    reference_entity_id: None,
                    source: row.source.clone(),
                    description: row.source_depth_reference.clone(),
                },
                base_measurement: row.base_depth.map(|value| VerticalMeasurement {
                    measurement_id: Some(format!("{}:base", marker_id.0.clone())),
                    value,
                    unit: None,
                    path: measurement_path,
                    coordinate_reference: None,
                    reference_measurement_id: None,
                    reference_entity_id: None,
                    source: row.source.clone(),
                    description: row.source_depth_reference.clone(),
                }),
                depth_reference: row.source_depth_reference.clone(),
                source: row.source.clone(),
                external_references: Vec::new(),
                notes,
            }
        })
        .collect()
}

fn identifiers_from_binding(binding: &AssetBindingInput) -> WellIdentifierSet {
    WellIdentifierSet {
        primary_name: Some(binding.well_name.clone()),
        uwi: binding.uwi.clone(),
        api: binding.api.clone(),
        operator_aliases: binding.operator_aliases.clone(),
    }
}

fn structured_asset_extent(
    asset_kind: AssetKind,
    extent: (Option<f64>, Option<f64>, Option<usize>),
) -> AssetExtent {
    AssetExtent {
        index_kind: match asset_kind {
            AssetKind::Trajectory
            | AssetKind::TopSet
            | AssetKind::WellMarkerSet
            | AssetKind::WellMarkerHorizonResidualSet
            | AssetKind::PressureObservation
            | AssetKind::DrillingObservation
            | AssetKind::CheckshotVspObservationSet
            | AssetKind::ManualTimeDepthPickSet
            | AssetKind::WellTieObservationSet
            | AssetKind::WellTimeDepthAuthoredModel
            | AssetKind::WellTimeDepthModel => Some(IndexKind::Depth),
            AssetKind::Log => Some(IndexKind::Depth),
            AssetKind::RawSourceBundle => None,
            AssetKind::SeismicTraceData => Some(IndexKind::Time),
        },
        start: extent.0,
        stop: extent.1,
        row_count: extent.2,
    }
}

fn seismic_asset_extent(metadata: &SeismicAssetMetadata) -> AssetExtent {
    let sample_axis = &metadata.store.volume.axes.sample_axis_ms;
    AssetExtent {
        index_kind: Some(IndexKind::Time),
        start: sample_axis.first().copied().map(f64::from),
        stop: sample_axis.last().copied().map(f64::from),
        row_count: Some(metadata.store.volume.shape[0] * metadata.store.volume.shape[1]),
    }
}

fn write_well_time_depth_model_package(
    package_root: &Path,
    model: &WellTimeDepthModel1D,
) -> Result<()> {
    validate_well_time_depth_model(model)?;
    fs::create_dir_all(package_root)?;
    fs::write(
        package_root.join("metadata.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": "0.1.0",
            "asset_kind": "well_time_depth_model",
            "id": model.id,
            "name": model.name,
            "wellbore_id": model.wellbore_id,
            "source_kind": model.source_kind,
            "depth_reference": model.depth_reference,
            "travel_time_reference": model.travel_time_reference,
            "sample_count": model.samples.len(),
            "depth_start": model.samples.first().map(|sample| sample.depth),
            "depth_stop": model.samples.last().map(|sample| sample.depth),
        }))?,
    )?;
    fs::write(
        package_root.join(WELL_TIME_DEPTH_MODEL_FILENAME),
        serde_json::to_vec_pretty(model)?,
    )?;
    Ok(())
}

fn write_checkshot_vsp_observation_set_package(
    package_root: &Path,
    observation_set: &CheckshotVspObservationSet1D,
) -> Result<()> {
    validate_checkshot_vsp_observation_set(observation_set)?;
    write_well_time_depth_json_package(
        package_root,
        CHECKSHOT_VSP_OBSERVATION_SET_FILENAME,
        observation_set,
    )
}

fn write_manual_time_depth_pick_set_package(
    package_root: &Path,
    pick_set: &ManualTimeDepthPickSet1D,
) -> Result<()> {
    validate_manual_time_depth_pick_set(pick_set)?;
    write_well_time_depth_json_package(package_root, MANUAL_TIME_DEPTH_PICK_SET_FILENAME, pick_set)
}

fn write_well_tie_observation_set_package(
    package_root: &Path,
    observation_set: &WellTieObservationSet1D,
) -> Result<()> {
    validate_well_tie_observation_set(observation_set)?;
    write_well_time_depth_json_package(
        package_root,
        WELL_TIE_OBSERVATION_SET_FILENAME,
        observation_set,
    )
}

fn write_well_time_depth_authored_model_package(
    package_root: &Path,
    model: &WellTimeDepthAuthoredModel1D,
) -> Result<()> {
    validate_well_time_depth_authored_model(model)?;
    write_well_time_depth_json_package(package_root, WELL_TIME_DEPTH_AUTHORED_MODEL_FILENAME, model)
}

pub fn preview_well_time_depth_json_asset(
    source_path: &Path,
    asset_kind: AssetKind,
) -> Result<ProjectWellTimeDepthAssetPreview> {
    preview_well_time_depth_json_asset_bytes(source_path, &fs::read(source_path)?, asset_kind)
}

pub fn preview_well_time_depth_json_payload(
    source_path: &Path,
    json_payload: &str,
    asset_kind: AssetKind,
) -> Result<ProjectWellTimeDepthAssetPreview> {
    preview_well_time_depth_json_asset_bytes(source_path, json_payload.as_bytes(), asset_kind)
}

pub fn preview_well_time_depth_import_draft(
    source_path: &Path,
    json_payload: Option<&str>,
    asset_kind: AssetKind,
    collection_name: Option<&str>,
) -> Result<ProjectWellTimeDepthImportPreview> {
    let parsed = if let Some(json_payload) = json_payload {
        preview_well_time_depth_json_payload(source_path, json_payload, asset_kind.clone())?
    } else {
        preview_well_time_depth_json_asset(source_path, asset_kind.clone())?
    };
    Ok(ProjectWellTimeDepthImportPreview {
        suggested_draft: ProjectWellTimeDepthImportCanonicalDraft {
            asset_kind: asset_kind.as_str().to_string(),
            json_payload: parsed.raw_json.clone(),
            collection_name: collection_name
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        },
        parsed,
    })
}

fn preview_well_time_depth_json_asset_bytes(
    source_path: &Path,
    raw_bytes: &[u8],
    asset_kind: AssetKind,
) -> Result<ProjectWellTimeDepthAssetPreview> {
    let raw_value: serde_json::Value = serde_json::from_slice(raw_bytes).map_err(|error| {
        LasError::Parse(format!(
            "failed to parse well time-depth json '{}': {error}",
            source_path.display()
        ))
    })?;
    let raw_json = serde_json::to_string_pretty(&raw_value)
        .unwrap_or_else(|_| String::from_utf8_lossy(raw_bytes).into_owned());
    let mut preview =
        build_well_time_depth_asset_preview(source_path, &asset_kind, &raw_value, raw_json);
    preview
        .issues
        .extend(required_well_time_depth_preview_issues(
            &raw_value,
            &asset_kind,
        ));
    preview.can_import = match asset_kind {
        AssetKind::CheckshotVspObservationSet => {
            preview_checkshot_vsp_asset(&raw_value, source_path, &mut preview)
        }
        AssetKind::ManualTimeDepthPickSet => {
            preview_manual_time_depth_pick_asset(&raw_value, source_path, &mut preview)
        }
        AssetKind::WellTieObservationSet => {
            preview_well_tie_observation_asset(&raw_value, source_path, &mut preview)
        }
        AssetKind::WellTimeDepthAuthoredModel => {
            preview_well_time_depth_authored_asset(&raw_value, source_path, &mut preview)
        }
        AssetKind::WellTimeDepthModel => {
            preview_well_time_depth_model_asset(&raw_value, source_path, &mut preview)
        }
        other => {
            preview.issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                "unsupported_asset_kind",
                format!(
                    "asset kind '{}' is not supported by the well time-depth preview",
                    other.as_str()
                ),
            ));
            false
        }
    };
    Ok(preview)
}

fn build_well_time_depth_asset_preview(
    source_path: &Path,
    asset_kind: &AssetKind,
    raw_value: &serde_json::Value,
    raw_json: String,
) -> ProjectWellTimeDepthAssetPreview {
    ProjectWellTimeDepthAssetPreview {
        asset_kind: asset_kind.as_str().to_string(),
        json_path: source_path.to_string_lossy().into_owned(),
        can_import: false,
        id: preview_string_field(raw_value, "id"),
        name: preview_string_field(raw_value, "name"),
        wellbore_id: preview_string_field(raw_value, "wellbore_id"),
        depth_reference: preview_string_field(raw_value, "depth_reference"),
        travel_time_reference: preview_string_field(raw_value, "travel_time_reference"),
        sample_count: preview_array_len_field(raw_value, "samples"),
        note_count: preview_array_len_field(raw_value, "notes"),
        source_kind: preview_string_field(raw_value, "source_kind"),
        source_binding_count: preview_array_len_field(raw_value, "source_bindings"),
        assumption_interval_count: preview_array_len_field(raw_value, "assumption_intervals"),
        sampling_step_m: preview_number_field(raw_value, "sampling_step_m"),
        resolved_trajectory_fingerprint: preview_string_field(
            raw_value,
            "resolved_trajectory_fingerprint",
        ),
        source_well_time_depth_model_asset_id: preview_string_field(
            raw_value,
            "source_well_time_depth_model_asset_id",
        ),
        tie_window_start_ms: preview_number_field(raw_value, "tie_window_start_ms"),
        tie_window_end_ms: preview_number_field(raw_value, "tie_window_end_ms"),
        trace_search_radius_m: preview_number_field(raw_value, "trace_search_radius_m"),
        bulk_shift_ms: preview_number_field(raw_value, "bulk_shift_ms"),
        stretch_factor: preview_number_field(raw_value, "stretch_factor"),
        trace_search_offset_m: preview_number_field(raw_value, "trace_search_offset_m"),
        correlation: preview_number_field(raw_value, "correlation"),
        issues: Vec::new(),
        raw_json,
    }
}

fn required_well_time_depth_preview_issues(
    raw_value: &serde_json::Value,
    asset_kind: &AssetKind,
) -> Vec<ProjectWellTimeDepthPreviewIssue> {
    let Some(object) = raw_value.as_object() else {
        return vec![project_well_time_depth_preview_issue(
            ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
            "invalid_root",
            "well time-depth json must contain a top-level object",
        )];
    };
    let required_fields: &[&str] = match asset_kind {
        AssetKind::CheckshotVspObservationSet
        | AssetKind::ManualTimeDepthPickSet
        | AssetKind::WellTieObservationSet => &[
            "id",
            "name",
            "depth_reference",
            "travel_time_reference",
            "samples",
        ],
        AssetKind::WellTimeDepthAuthoredModel => &[
            "id",
            "name",
            "wellbore_id",
            "resolved_trajectory_fingerprint",
            "depth_reference",
            "travel_time_reference",
            "source_bindings",
            "assumption_intervals",
        ],
        AssetKind::WellTimeDepthModel => &[
            "id",
            "name",
            "source_kind",
            "depth_reference",
            "travel_time_reference",
            "samples",
        ],
        _ => &[],
    };
    let mut issues = Vec::new();
    for field in required_fields.iter().copied() {
        if object.get(field).is_none() {
            issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                format!("missing_{}", field),
                format!(
                    "required field '{}' is missing from the json payload",
                    field
                ),
            ));
        }
    }
    issues
}

fn preview_checkshot_vsp_asset(
    raw_value: &serde_json::Value,
    source_path: &Path,
    preview: &mut ProjectWellTimeDepthAssetPreview,
) -> bool {
    match serde_json::from_value::<CheckshotVspObservationSet1D>(raw_value.clone()) {
        Ok(observation_set) => match validate_checkshot_vsp_observation_set(&observation_set) {
            Ok(()) => true,
            Err(error) => {
                preview.issues.push(project_well_time_depth_preview_issue(
                    ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                    "validation_failed",
                    format!(
                        "checkshot/VSP observation set cannot be imported yet: {}",
                        error
                    ),
                ));
                false
            }
        },
        Err(error) => {
            preview.issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                "parse_failed",
                format!(
                    "failed to decode checkshot/VSP observation set '{}': {}",
                    source_path.display(),
                    error
                ),
            ));
            false
        }
    }
}

fn preview_manual_time_depth_pick_asset(
    raw_value: &serde_json::Value,
    source_path: &Path,
    preview: &mut ProjectWellTimeDepthAssetPreview,
) -> bool {
    match serde_json::from_value::<ManualTimeDepthPickSet1D>(raw_value.clone()) {
        Ok(pick_set) => match validate_manual_time_depth_pick_set(&pick_set) {
            Ok(()) => true,
            Err(error) => {
                preview.issues.push(project_well_time_depth_preview_issue(
                    ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                    "validation_failed",
                    format!(
                        "manual time-depth pick set cannot be imported yet: {}",
                        error
                    ),
                ));
                false
            }
        },
        Err(error) => {
            preview.issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                "parse_failed",
                format!(
                    "failed to decode manual time-depth pick set '{}': {}",
                    source_path.display(),
                    error
                ),
            ));
            false
        }
    }
}

fn preview_well_tie_observation_asset(
    raw_value: &serde_json::Value,
    source_path: &Path,
    preview: &mut ProjectWellTimeDepthAssetPreview,
) -> bool {
    match serde_json::from_value::<WellTieObservationSet1D>(raw_value.clone()) {
        Ok(observation_set) => match validate_well_tie_observation_set(&observation_set) {
            Ok(()) => true,
            Err(error) => {
                preview.issues.push(project_well_time_depth_preview_issue(
                    ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                    "validation_failed",
                    format!("well tie observation set cannot be imported yet: {}", error),
                ));
                false
            }
        },
        Err(error) => {
            preview.issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                "parse_failed",
                format!(
                    "failed to decode well tie observation set '{}': {}",
                    source_path.display(),
                    error
                ),
            ));
            false
        }
    }
}

fn preview_well_time_depth_authored_asset(
    raw_value: &serde_json::Value,
    source_path: &Path,
    preview: &mut ProjectWellTimeDepthAssetPreview,
) -> bool {
    match serde_json::from_value::<WellTimeDepthAuthoredModel1D>(raw_value.clone()) {
        Ok(model) => match validate_well_time_depth_authored_model(&model) {
            Ok(()) => true,
            Err(error) => {
                preview.issues.push(project_well_time_depth_preview_issue(
                    ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                    "validation_failed",
                    format!(
                        "well time-depth authored model cannot be imported yet: {}",
                        error
                    ),
                ));
                false
            }
        },
        Err(error) => {
            preview.issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                "parse_failed",
                format!(
                    "failed to decode well time-depth authored model '{}': {}",
                    source_path.display(),
                    error
                ),
            ));
            false
        }
    }
}

fn preview_well_time_depth_model_asset(
    raw_value: &serde_json::Value,
    source_path: &Path,
    preview: &mut ProjectWellTimeDepthAssetPreview,
) -> bool {
    match serde_json::from_value::<WellTimeDepthModel1D>(raw_value.clone()) {
        Ok(model) => match validate_well_time_depth_model(&model) {
            Ok(()) => true,
            Err(error) => {
                preview.issues.push(project_well_time_depth_preview_issue(
                    ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                    "validation_failed",
                    format!("well time-depth model cannot be imported yet: {}", error),
                ));
                false
            }
        },
        Err(error) => {
            preview.issues.push(project_well_time_depth_preview_issue(
                ProjectWellTimeDepthPreviewIssueSeverity::Blocking,
                "parse_failed",
                format!(
                    "failed to decode well time-depth model '{}': {}",
                    source_path.display(),
                    error
                ),
            ));
            false
        }
    }
}

fn preview_string_field(raw_value: &serde_json::Value, field_name: &str) -> Option<String> {
    raw_value
        .as_object()
        .and_then(|object| object.get(field_name))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn preview_array_len_field(raw_value: &serde_json::Value, field_name: &str) -> Option<usize> {
    raw_value
        .as_object()
        .and_then(|object| object.get(field_name))
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
}

fn preview_number_field(raw_value: &serde_json::Value, field_name: &str) -> Option<f64> {
    raw_value
        .as_object()
        .and_then(|object| object.get(field_name))
        .and_then(|value| match value {
            serde_json::Value::Number(number) => number.as_f64(),
            serde_json::Value::String(text) => text.parse::<f64>().ok(),
            _ => None,
        })
}

fn project_well_time_depth_preview_issue(
    severity: ProjectWellTimeDepthPreviewIssueSeverity,
    code: impl Into<String>,
    message: impl Into<String>,
) -> ProjectWellTimeDepthPreviewIssue {
    ProjectWellTimeDepthPreviewIssue {
        severity,
        code: code.into(),
        message: message.into(),
    }
}

fn read_well_time_depth_model_package(package_root: &Path) -> Result<WellTimeDepthModel1D> {
    let model: WellTimeDepthModel1D = serde_json::from_slice(&fs::read(
        package_root.join(WELL_TIME_DEPTH_MODEL_FILENAME),
    )?)
    .map_err(|error| {
        LasError::Parse(format!(
            "failed to parse well time-depth model package '{}': {error}",
            package_root.display()
        ))
    })?;
    validate_well_time_depth_model(&model)?;
    Ok(model)
}

fn read_checkshot_vsp_observation_set_package(
    package_root: &Path,
) -> Result<CheckshotVspObservationSet1D> {
    let observation_set: CheckshotVspObservationSet1D =
        read_well_time_depth_json_package(package_root, CHECKSHOT_VSP_OBSERVATION_SET_FILENAME)?;
    validate_checkshot_vsp_observation_set(&observation_set)?;
    Ok(observation_set)
}

fn read_manual_time_depth_pick_set_package(
    package_root: &Path,
) -> Result<ManualTimeDepthPickSet1D> {
    let pick_set: ManualTimeDepthPickSet1D =
        read_well_time_depth_json_package(package_root, MANUAL_TIME_DEPTH_PICK_SET_FILENAME)?;
    validate_manual_time_depth_pick_set(&pick_set)?;
    Ok(pick_set)
}

fn read_well_tie_observation_set_package(package_root: &Path) -> Result<WellTieObservationSet1D> {
    let observation_set: WellTieObservationSet1D =
        read_well_time_depth_json_package(package_root, WELL_TIE_OBSERVATION_SET_FILENAME)?;
    validate_well_tie_observation_set(&observation_set)?;
    Ok(observation_set)
}

fn read_well_time_depth_authored_model_package(
    package_root: &Path,
) -> Result<WellTimeDepthAuthoredModel1D> {
    let model: WellTimeDepthAuthoredModel1D =
        read_well_time_depth_json_package(package_root, WELL_TIME_DEPTH_AUTHORED_MODEL_FILENAME)?;
    validate_well_time_depth_authored_model(&model)?;
    Ok(model)
}

fn write_well_time_depth_json_package<T: Serialize>(
    package_root: &Path,
    payload_filename: &str,
    payload: &T,
) -> Result<()> {
    fs::create_dir_all(package_root)?;
    fs::write(
        package_root.join("metadata.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": "0.1.0",
            "asset_kind": payload_filename.strip_suffix(".json").unwrap_or(payload_filename),
            "payload_filename": payload_filename,
        }))?,
    )?;
    fs::write(
        package_root.join(payload_filename),
        serde_json::to_vec_pretty(payload)?,
    )?;
    Ok(())
}

fn read_well_time_depth_json_package<T: for<'de> Deserialize<'de>>(
    package_root: &Path,
    payload_filename: &str,
) -> Result<T> {
    serde_json::from_slice(&fs::read(package_root.join(payload_filename))?).map_err(|error| {
        LasError::Parse(format!(
            "failed to parse well time-depth json package '{}': {error}",
            package_root.display()
        ))
    })
}

fn read_seismic_asset_metadata(package_root: &Path) -> Result<SeismicAssetMetadata> {
    let metadata_path = package_root.join("metadata.json");
    let bytes = fs::read(&metadata_path).map_err(|error| {
        LasError::Storage(format!(
            "failed to read seismic asset metadata '{}': {error}",
            metadata_path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        LasError::Storage(format!(
            "failed to parse seismic asset metadata '{}': {error}",
            metadata_path.display()
        ))
    })
}

fn coordinate_reference_dto(
    reference: Option<&CoordinateReference>,
    unit: Option<&str>,
) -> Option<CoordinateReferenceDto> {
    reference.map(|reference| CoordinateReferenceDto {
        id: reference.id.clone(),
        name: reference.name.clone(),
        geodetic_datum: reference.geodetic_datum.clone(),
        unit: unit.map(str::to_owned),
    })
}

fn survey_spatial_descriptor_dto_from_seismic(
    spatial: &ophiolite_seismic::SurveySpatialDescriptor,
) -> SurveyMapSpatialDescriptorDto {
    SurveyMapSpatialDescriptorDto {
        coordinate_reference: spatial
            .coordinate_reference
            .as_ref()
            .map(coordinate_reference_dto_from_seismic),
        grid_transform: spatial.grid_transform.as_ref().map(|transform| {
            SurveyMapGridTransformDto {
                origin: projected_point_dto_from_seismic(&transform.origin),
                inline_basis: ProjectedVector2Dto {
                    x: transform.inline_basis.x,
                    y: transform.inline_basis.y,
                },
                xline_basis: ProjectedVector2Dto {
                    x: transform.xline_basis.x,
                    y: transform.xline_basis.y,
                },
            }
        }),
        footprint: spatial
            .footprint
            .as_ref()
            .map(|polygon| ProjectedPolygon2Dto {
                exterior: polygon
                    .exterior
                    .iter()
                    .map(projected_point_dto_from_seismic)
                    .collect(),
            }),
        availability: match spatial.availability {
            ophiolite_seismic::SurveySpatialAvailability::Available => {
                SurveyMapSpatialAvailabilityDto::Available
            }
            ophiolite_seismic::SurveySpatialAvailability::Partial => {
                SurveyMapSpatialAvailabilityDto::Partial
            }
            ophiolite_seismic::SurveySpatialAvailability::Unavailable => {
                SurveyMapSpatialAvailabilityDto::Unavailable
            }
        },
        notes: spatial.notes.clone(),
    }
}

fn coordinate_reference_binding_dto_from_seismic(
    binding: &ophiolite_seismic::CoordinateReferenceBinding,
) -> CoordinateReferenceBindingDto {
    CoordinateReferenceBindingDto {
        detected: binding
            .detected
            .as_ref()
            .map(coordinate_reference_dto_from_seismic),
        effective: binding
            .effective
            .as_ref()
            .map(coordinate_reference_dto_from_seismic),
        source: match binding.source {
            ophiolite_seismic::CoordinateReferenceSource::Header => {
                CoordinateReferenceSourceDto::Header
            }
            ophiolite_seismic::CoordinateReferenceSource::ImportManifest => {
                CoordinateReferenceSourceDto::ImportManifest
            }
            ophiolite_seismic::CoordinateReferenceSource::UserOverride => {
                CoordinateReferenceSourceDto::UserOverride
            }
            ophiolite_seismic::CoordinateReferenceSource::Unknown => {
                CoordinateReferenceSourceDto::Unknown
            }
        },
        notes: binding.notes.clone(),
    }
}

type SurveyMapTransformResolution<T> = (
    Option<T>,
    SurveyMapTransformStatusDto,
    SurveyMapTransformDiagnosticsDto,
);

#[derive(Debug, Clone)]
struct SurveyMapDisplayTransformRequest {
    source_coordinate_reference_id: Option<String>,
    target_coordinate_reference_id: Option<String>,
    policy: SurveyMapTransformPolicyDto,
}

impl SurveyMapDisplayTransformRequest {
    fn new(
        source_coordinate_reference_id: Option<String>,
        display_coordinate_reference_id: Option<&str>,
    ) -> Self {
        Self {
            source_coordinate_reference_id,
            target_coordinate_reference_id: display_coordinate_reference_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            policy: SurveyMapTransformPolicyDto::BestAvailable,
        }
    }

    fn source_coordinate_reference_id(&self) -> Option<&str> {
        self.source_coordinate_reference_id.as_deref()
    }

    fn target_coordinate_reference_id(&self) -> Option<&str> {
        self.target_coordinate_reference_id.as_deref()
    }

    fn target_supported(&self) -> bool {
        self.target_coordinate_reference_id()
            .is_some_and(is_supported_epsg_identifier)
    }

    fn source_supported(&self) -> bool {
        self.source_coordinate_reference_id()
            .is_some_and(is_supported_epsg_identifier)
    }

    fn is_identity_transform(&self) -> bool {
        matches!(
            (
                self.source_coordinate_reference_id(),
                self.target_coordinate_reference_id(),
            ),
            (Some(source), Some(target)) if source.eq_ignore_ascii_case(target)
        )
    }

    fn diagnostics(
        &self,
        operation_id: Option<String>,
        operation_name: Option<String>,
        accuracy_meters: Option<f64>,
        degraded: bool,
        notes: Vec<String>,
    ) -> SurveyMapTransformDiagnosticsDto {
        SurveyMapTransformDiagnosticsDto {
            source_coordinate_reference_id: self.source_coordinate_reference_id.clone(),
            target_coordinate_reference_id: self.target_coordinate_reference_id.clone(),
            policy: self.policy,
            operation_id,
            operation_name,
            accuracy_meters,
            degraded,
            notes,
        }
    }

    fn resolution<T>(
        &self,
        value: Option<T>,
        status: SurveyMapTransformStatusDto,
        operation_id: Option<String>,
        operation_name: Option<String>,
        accuracy_meters: Option<f64>,
        degraded: bool,
        notes: Vec<String>,
    ) -> SurveyMapTransformResolution<T> {
        (
            value,
            status,
            self.diagnostics(
                operation_id,
                operation_name,
                accuracy_meters,
                degraded,
                notes,
            ),
        )
    }
}

fn resolve_display_spatial_descriptor(
    cache_dir: Option<&Path>,
    asset_id: &str,
    geometry_fingerprint: &str,
    coordinate_reference_binding: Option<&CoordinateReferenceBindingDto>,
    native_spatial: &SurveyMapSpatialDescriptorDto,
    display_coordinate_reference_id: Option<&str>,
    notes: &mut Vec<String>,
) -> (
    Option<SurveyMapSpatialDescriptorDto>,
    SurveyMapTransformStatusDto,
    SurveyMapTransformDiagnosticsDto,
) {
    let request = SurveyMapDisplayTransformRequest::new(
        coordinate_reference_binding
            .and_then(|binding| binding.effective.as_ref())
            .and_then(|reference| reference.id.clone()),
        display_coordinate_reference_id,
    );
    let Some(display_coordinate_reference_id) = request.target_coordinate_reference_id() else {
        return request.resolution(
            None,
            SurveyMapTransformStatusDto::NativeOnly,
            None,
            None,
            None,
            false,
            Vec::new(),
        );
    };

    if !request.target_supported() {
        let note = format!(
            "display coordinate reference '{display_coordinate_reference_id}' is not yet supported; phase 2 currently accepts only EPSG identifiers"
        );
        notes.push(note.clone());
        return request.resolution(
            None,
            SurveyMapTransformStatusDto::DisplayUnavailable,
            None,
            None,
            None,
            false,
            vec![note],
        );
    }

    match request.source_coordinate_reference_id() {
        Some(_) if request.is_identity_transform() => {
            let mut display_spatial = native_spatial.clone();
            if let Some(binding) = coordinate_reference_binding {
                display_spatial.coordinate_reference = binding.effective.clone();
            } else {
                display_spatial.coordinate_reference = Some(CoordinateReferenceDto {
                    id: Some(display_coordinate_reference_id.to_string()),
                    name: None,
                    geodetic_datum: None,
                    unit: native_spatial
                        .coordinate_reference
                        .as_ref()
                        .and_then(|reference| reference.unit.clone()),
                });
            }
            request.resolution(
                Some(display_spatial),
                SurveyMapTransformStatusDto::DisplayEquivalent,
                None,
                Some("identity".to_string()),
                Some(0.0),
                false,
                Vec::new(),
            )
        }
        Some(source_coordinate_reference_id) => {
            if !request.source_supported() {
                let note = format!(
                    "survey effective native CRS '{source_coordinate_reference_id}' is not yet supported for reprojection; phase 2 currently accepts only EPSG identifiers"
                );
                notes.push(note.clone());
                return request.resolution(
                    None,
                    SurveyMapTransformStatusDto::DisplayUnavailable,
                    None,
                    None,
                    None,
                    false,
                    vec![note],
                );
            }

            if !native_spatial_has_transformable_geometry(native_spatial) {
                let note = format!(
                    "display coordinate reference '{display_coordinate_reference_id}' was requested but the survey has no transformable native map geometry"
                );
                notes.push(note.clone());
                return request.resolution(
                    None,
                    SurveyMapTransformStatusDto::DisplayUnavailable,
                    None,
                    None,
                    None,
                    false,
                    vec![note],
                );
            }

            let cache_key = survey_map_transform_cache_key(
                asset_id,
                geometry_fingerprint,
                source_coordinate_reference_id,
                display_coordinate_reference_id,
                request.policy,
            );
            if let Some(cached) = read_survey_map_transform_cache(cache_dir, &cache_key) {
                let mut diagnostics = cached.transform_diagnostics.clone();
                diagnostics
                    .notes
                    .push("display spatial loaded from cache".to_string());
                return (
                    Some(cached.display_spatial),
                    cached.transform_status,
                    diagnostics,
                );
            }

            match transform_survey_map_spatial_descriptor(
                native_spatial,
                source_coordinate_reference_id,
                display_coordinate_reference_id,
            ) {
                Ok(display_spatial) => {
                    let diagnostics = request.diagnostics(
                        Some("proj_crs_to_crs".to_string()),
                        Some(format!(
                            "proj_crs_to_crs:{source_coordinate_reference_id}->{display_coordinate_reference_id}"
                        )),
                        None,
                        false,
                        vec![format!(
                            "display spatial was reprojected from {source_coordinate_reference_id} to {display_coordinate_reference_id}"
                        )],
                    );
                    let artifact = SurveyMapTransformCacheArtifact {
                        schema_version: SURVEY_MAP_TRANSFORM_CACHE_SCHEMA_VERSION,
                        cache_key,
                        asset_id: asset_id.to_string(),
                        geometry_fingerprint: geometry_fingerprint.to_string(),
                        source_coordinate_reference_id: source_coordinate_reference_id.to_string(),
                        target_coordinate_reference_id: display_coordinate_reference_id.to_string(),
                        policy: request.policy,
                        display_spatial: display_spatial.clone(),
                        transform_status: SurveyMapTransformStatusDto::DisplayTransformed,
                        transform_diagnostics: diagnostics.clone(),
                    };
                    if let Err(error) = write_survey_map_transform_cache(cache_dir, &artifact) {
                        notes.push(format!(
                            "failed to persist survey-map transform cache artifact: {error}"
                        ));
                    }
                    (
                        Some(display_spatial),
                        SurveyMapTransformStatusDto::DisplayTransformed,
                        diagnostics,
                    )
                }
                Err(error) => {
                    let note = format!(
                        "display coordinate reference '{display_coordinate_reference_id}' could not be resolved from '{source_coordinate_reference_id}': {error}"
                    );
                    notes.push(note.clone());
                    request.resolution(
                        None,
                        SurveyMapTransformStatusDto::DisplayUnavailable,
                        Some("proj_crs_to_crs".to_string()),
                        Some(format!(
                            "proj_crs_to_crs:{source_coordinate_reference_id}->{display_coordinate_reference_id}"
                        )),
                        None,
                        false,
                        vec![note],
                    )
                }
            }
        }
        None => {
            let note = format!(
                "display coordinate reference '{display_coordinate_reference_id}' was requested but the survey effective native CRS is unknown"
            );
            notes.push(note.clone());
            request.resolution(
                None,
                SurveyMapTransformStatusDto::DisplayUnavailable,
                None,
                None,
                None,
                false,
                vec![note],
            )
        }
    }
}

fn require_project_display_coordinate_reference_id(
    display_coordinate_reference_id: &str,
) -> Result<&str> {
    let normalized = display_coordinate_reference_id.trim();
    if normalized.is_empty() {
        return Err(LasError::Validation(
            "project survey-map requests require a non-empty display coordinate reference id"
                .to_string(),
        ));
    }
    Ok(normalized)
}

fn is_supported_epsg_identifier(value: &str) -> bool {
    value.trim().to_ascii_uppercase().starts_with("EPSG:")
}

fn native_spatial_has_transformable_geometry(spatial: &SurveyMapSpatialDescriptorDto) -> bool {
    spatial.grid_transform.is_some() || spatial.footprint.is_some()
}

fn transform_survey_map_spatial_descriptor(
    native_spatial: &SurveyMapSpatialDescriptorDto,
    source_coordinate_reference_id: &str,
    target_coordinate_reference_id: &str,
) -> Result<SurveyMapSpatialDescriptorDto> {
    let transformer = build_proj_transformer(
        source_coordinate_reference_id,
        target_coordinate_reference_id,
    )?;
    let grid_transform = native_spatial
        .grid_transform
        .as_ref()
        .map(|transform| transform_grid_transform(&transformer, transform))
        .transpose()?;
    let footprint = native_spatial
        .footprint
        .as_ref()
        .map(|polygon| transform_polygon(&transformer, polygon))
        .transpose()?;

    let mut display_spatial = native_spatial.clone();
    display_spatial.coordinate_reference = Some(CoordinateReferenceDto {
        id: Some(target_coordinate_reference_id.to_string()),
        name: None,
        geodetic_datum: None,
        unit: None,
    });
    display_spatial.grid_transform = grid_transform;
    display_spatial.footprint = footprint;
    display_spatial.notes.push(format!(
        "display spatial was reprojected from {source_coordinate_reference_id} to {target_coordinate_reference_id}"
    ));
    Ok(display_spatial)
}

fn build_proj_transformer(
    source_coordinate_reference_id: &str,
    target_coordinate_reference_id: &str,
) -> Result<Proj> {
    let mut builder = ProjBuilder::new();
    if let Ok(resource_path) = std::env::var(PROJ_RESOURCE_PATH_ENV) {
        let resource_path = resource_path.trim();
        if !resource_path.is_empty() {
            builder.set_search_paths(resource_path).map_err(|error| {
                LasError::Storage(format!("failed to set PROJ search path: {error}"))
            })?;
        }
    }
    builder
        .proj_known_crs(
            source_coordinate_reference_id,
            target_coordinate_reference_id,
            None,
        )
        .map_err(|error| LasError::Storage(format!("failed to build PROJ transformer: {error}")))
}

fn transform_grid_transform(
    transformer: &Proj,
    transform: &SurveyMapGridTransformDto,
) -> Result<SurveyMapGridTransformDto> {
    let origin = transform_point(transformer, &transform.origin)?;
    let inline_endpoint = transform_point(
        transformer,
        &ProjectedPoint2Dto {
            x: transform.origin.x + transform.inline_basis.x,
            y: transform.origin.y + transform.inline_basis.y,
        },
    )?;
    let xline_endpoint = transform_point(
        transformer,
        &ProjectedPoint2Dto {
            x: transform.origin.x + transform.xline_basis.x,
            y: transform.origin.y + transform.xline_basis.y,
        },
    )?;
    Ok(SurveyMapGridTransformDto {
        origin: origin.clone(),
        inline_basis: ProjectedVector2Dto {
            x: inline_endpoint.x - origin.x,
            y: inline_endpoint.y - origin.y,
        },
        xline_basis: ProjectedVector2Dto {
            x: xline_endpoint.x - origin.x,
            y: xline_endpoint.y - origin.y,
        },
    })
}

fn transform_polygon(
    transformer: &Proj,
    polygon: &ProjectedPolygon2Dto,
) -> Result<ProjectedPolygon2Dto> {
    Ok(ProjectedPolygon2Dto {
        exterior: polygon
            .exterior
            .iter()
            .map(|point| transform_point(transformer, point))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn transform_point(transformer: &Proj, point: &ProjectedPoint2Dto) -> Result<ProjectedPoint2Dto> {
    let transformed = transformer
        .convert((point.x, point.y))
        .map_err(|error| LasError::Storage(format!("PROJ coordinate transform failed: {error}")))?;
    Ok(ProjectedPoint2Dto {
        x: transformed.0,
        y: transformed.1,
    })
}

fn project_map_transform_cache_dir(project_root: &Path) -> PathBuf {
    project_root
        .join(PROJECT_REVISION_STORE_DIRNAME)
        .join(PROJECT_MAP_TRANSFORM_CACHE_DIRNAME)
}

fn survey_map_transform_cache_key(
    asset_id: &str,
    geometry_fingerprint: &str,
    source_coordinate_reference_id: &str,
    target_coordinate_reference_id: &str,
    policy: SurveyMapTransformPolicyDto,
) -> String {
    stable_project_blob_hash(
        "survey-map-transform",
        format!(
            "{asset_id}|{geometry_fingerprint}|{source_coordinate_reference_id}|{target_coordinate_reference_id}|{policy:?}"
        )
        .as_bytes(),
    )
}

fn survey_map_transform_cache_path(cache_dir: &Path, cache_key: &str) -> PathBuf {
    cache_dir.join(format!("{cache_key}.json"))
}

fn read_survey_map_transform_cache(
    cache_dir: Option<&Path>,
    cache_key: &str,
) -> Option<SurveyMapTransformCacheArtifact> {
    let cache_dir = cache_dir?;
    let cache_path = survey_map_transform_cache_path(cache_dir, cache_key);
    let bytes = fs::read(cache_path).ok()?;
    let artifact = serde_json::from_slice::<SurveyMapTransformCacheArtifact>(&bytes).ok()?;
    if artifact.schema_version != SURVEY_MAP_TRANSFORM_CACHE_SCHEMA_VERSION {
        return None;
    }
    if artifact.cache_key != cache_key {
        return None;
    }
    Some(artifact)
}

fn write_survey_map_transform_cache(
    cache_dir: Option<&Path>,
    artifact: &SurveyMapTransformCacheArtifact,
) -> Result<()> {
    let Some(cache_dir) = cache_dir else {
        return Ok(());
    };
    fs::create_dir_all(&cache_dir)?;
    fs::write(
        survey_map_transform_cache_path(cache_dir, &artifact.cache_key),
        serde_json::to_vec_pretty(artifact)?,
    )?;
    Ok(())
}

fn preferred_section_grid_transform(
    survey: &ResolvedSurveyMapSurveyDto,
) -> Option<&SurveyMapGridTransformDto> {
    survey
        .display_spatial
        .as_ref()
        .and_then(|display_spatial| display_spatial.grid_transform.as_ref())
        .or(survey.native_spatial.grid_transform.as_ref())
}

fn section_axis_spec(
    index_grid: &SurveyIndexGridDto,
    axis: SectionAxis,
    requested_index: i32,
) -> Result<SectionAxisSpec> {
    let inline_step = required_regular_axis_step(&index_grid.inline_axis, "inline")?;
    let xline_step = required_regular_axis_step(&index_grid.xline_axis, "xline")?;
    Ok(match axis {
        SectionAxis::Inline => SectionAxisSpec {
            axis,
            requested_coordinate: f64::from(requested_index),
            inline_first: f64::from(index_grid.inline_axis.first),
            inline_step,
            xline_first: f64::from(index_grid.xline_axis.first),
            xline_step,
            trace_count: index_grid.xline_axis.count,
        },
        SectionAxis::Xline => SectionAxisSpec {
            axis,
            requested_coordinate: f64::from(requested_index),
            inline_first: f64::from(index_grid.inline_axis.first),
            inline_step,
            xline_first: f64::from(index_grid.xline_axis.first),
            xline_step,
            trace_count: index_grid.inline_axis.count,
        },
    })
}

fn required_regular_axis_step(axis: &SurveyIndexAxisDto, axis_name: &str) -> Result<f64> {
    let step = axis.step.ok_or_else(|| {
        LasError::Validation(format!(
            "survey {axis_name} axis is irregular; section overlays currently require regular inline/xline axes"
        ))
    })?;
    if step == 0 {
        return Err(LasError::Validation(format!(
            "survey {axis_name} axis step cannot be zero"
        )));
    }
    Ok(f64::from(step))
}

fn default_section_tolerance_m(
    grid_transform: &SurveyMapGridTransformDto,
    axis: SectionAxis,
) -> f64 {
    0.5 * match axis {
        SectionAxis::Inline => vector_length_m(&grid_transform.inline_basis),
        SectionAxis::Xline => vector_length_m(&grid_transform.xline_basis),
    }
}

fn section_densification_settings(
    grid_transform: &SurveyMapGridTransformDto,
    tolerance_m: f64,
) -> SectionTrajectoryDensificationSettings {
    let inline_spacing_m = vector_length_m(&grid_transform.inline_basis);
    let xline_spacing_m = vector_length_m(&grid_transform.xline_basis);
    let nominal_spacing_m = inline_spacing_m.min(xline_spacing_m).max(1.0);
    let max_xy_step_m = (nominal_spacing_m * 0.5).min(tolerance_m.max(1.0));
    let max_vertical_step_m = max_xy_step_m;
    let max_md_step_m = max_xy_step_m.max(5.0);
    SectionTrajectoryDensificationSettings {
        max_md_step_m,
        max_xy_step_m,
        max_vertical_step_m,
    }
}

fn densify_trajectory_for_section(
    stations: &[ResolvedTrajectoryStation],
    settings: SectionTrajectoryDensificationSettings,
) -> Vec<ResolvedTrajectoryStation> {
    if stations.len() <= 1 {
        return stations.to_vec();
    }

    let mut densified = Vec::with_capacity(stations.len());
    densified.push(stations[0].clone());

    for pair in stations.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        let subdivisions = required_section_subdivisions(start, end, settings);
        for step in 1..subdivisions {
            densified.push(interpolate_resolved_trajectory_station(
                start,
                end,
                step as f64 / subdivisions as f64,
            ));
        }
        densified.push(end.clone());
    }

    densified
}

fn required_section_subdivisions(
    start: &ResolvedTrajectoryStation,
    end: &ResolvedTrajectoryStation,
    settings: SectionTrajectoryDensificationSettings,
) -> usize {
    let md_subdivisions = required_axis_subdivisions(
        end.measured_depth_m - start.measured_depth_m,
        settings.max_md_step_m,
    );
    let xy_subdivisions = match (start.absolute_xy.as_ref(), end.absolute_xy.as_ref()) {
        (Some(start_xy), Some(end_xy)) => required_axis_subdivisions(
            planar_distance_m(start_xy.x, start_xy.y, end_xy.x, end_xy.y),
            settings.max_xy_step_m,
        ),
        _ => match (
            start.easting_offset_m,
            start.northing_offset_m,
            end.easting_offset_m,
            end.northing_offset_m,
        ) {
            (Some(start_e), Some(start_n), Some(end_e), Some(end_n)) => required_axis_subdivisions(
                planar_distance_m(start_e, start_n, end_e, end_n),
                settings.max_xy_step_m,
            ),
            _ => 1,
        },
    };
    let vertical_subdivisions = start
        .true_vertical_depth_m
        .zip(end.true_vertical_depth_m)
        .map(|(start_depth, end_depth)| {
            required_axis_subdivisions(end_depth - start_depth, settings.max_vertical_step_m)
        })
        .or_else(|| {
            start
                .true_vertical_depth_subsea_m
                .zip(end.true_vertical_depth_subsea_m)
                .map(|(start_depth, end_depth)| {
                    required_axis_subdivisions(
                        end_depth - start_depth,
                        settings.max_vertical_step_m,
                    )
                })
        })
        .unwrap_or(1);

    md_subdivisions
        .max(xy_subdivisions)
        .max(vertical_subdivisions)
}

fn required_axis_subdivisions(delta: f64, max_step: f64) -> usize {
    if !delta.is_finite() || !max_step.is_finite() || max_step <= 0.0 {
        return 1;
    }
    let subdivisions = (delta.abs() / max_step).ceil() as usize;
    subdivisions.max(1)
}

fn interpolate_resolved_trajectory_station(
    start: &ResolvedTrajectoryStation,
    end: &ResolvedTrajectoryStation,
    fraction: f64,
) -> ResolvedTrajectoryStation {
    ResolvedTrajectoryStation {
        measured_depth_m: lerp(start.measured_depth_m, end.measured_depth_m, fraction),
        true_vertical_depth_m: lerp_option(
            start.true_vertical_depth_m,
            end.true_vertical_depth_m,
            fraction,
        ),
        true_vertical_depth_subsea_m: lerp_option(
            start.true_vertical_depth_subsea_m,
            end.true_vertical_depth_subsea_m,
            fraction,
        ),
        northing_offset_m: lerp_option(start.northing_offset_m, end.northing_offset_m, fraction),
        easting_offset_m: lerp_option(start.easting_offset_m, end.easting_offset_m, fraction),
        absolute_xy: match (start.absolute_xy.as_ref(), end.absolute_xy.as_ref()) {
            (Some(start_xy), Some(end_xy)) => Some(ProjectedPoint2 {
                x: lerp(start_xy.x, end_xy.x, fraction),
                y: lerp(start_xy.y, end_xy.y, fraction),
            }),
            _ => None,
        },
        inclination_deg: lerp_option(start.inclination_deg, end.inclination_deg, fraction),
        azimuth_deg: lerp_option(start.azimuth_deg, end.azimuth_deg, fraction),
        true_vertical_depth_origin: start
            .true_vertical_depth_m
            .zip(end.true_vertical_depth_m)
            .map(|_| TrajectoryValueOrigin::Derived),
        true_vertical_depth_subsea_origin: start
            .true_vertical_depth_subsea_m
            .zip(end.true_vertical_depth_subsea_m)
            .map(|_| TrajectoryValueOrigin::Derived),
        northing_offset_origin: start
            .northing_offset_m
            .zip(end.northing_offset_m)
            .map(|_| TrajectoryValueOrigin::Derived),
        easting_offset_origin: start
            .easting_offset_m
            .zip(end.easting_offset_m)
            .map(|_| TrajectoryValueOrigin::Derived),
        inclination_origin: start
            .inclination_deg
            .zip(end.inclination_deg)
            .map(|_| TrajectoryValueOrigin::Derived),
        azimuth_origin: start
            .azimuth_deg
            .zip(end.azimuth_deg)
            .map(|_| TrajectoryValueOrigin::Derived),
    }
}

fn lerp(start: f64, end: f64, fraction: f64) -> f64 {
    start + (end - start) * fraction
}

fn lerp_option(start: Option<f64>, end: Option<f64>, fraction: f64) -> Option<f64> {
    start
        .zip(end)
        .map(|(start, end)| lerp(start, end, fraction))
}

fn project_well_station_onto_section(
    station: &ResolvedTrajectoryStation,
    absolute_xy: &ProjectedPoint2,
    grid_transform: &SurveyMapGridTransformDto,
    section_axis: &SectionAxisSpec,
    tolerance_m: f64,
) -> Option<ProjectedSectionSample> {
    let (inline_ordinal, xline_ordinal) =
        invert_survey_grid_transform(grid_transform, absolute_xy.x, absolute_xy.y)?;
    let inline_value = section_axis.inline_first + inline_ordinal * section_axis.inline_step;
    let xline_value = section_axis.xline_first + xline_ordinal * section_axis.xline_step;
    let inline_basis_length = vector_length_m(&grid_transform.inline_basis);
    let xline_basis_length = vector_length_m(&grid_transform.xline_basis);
    let (section_coordinate, section_step, section_basis_length_m, trace_value, trace_ordinal) =
        match section_axis.axis {
            SectionAxis::Inline => (
                inline_value,
                section_axis.inline_step,
                inline_basis_length,
                xline_value,
                xline_ordinal,
            ),
            SectionAxis::Xline => (
                xline_value,
                section_axis.xline_step,
                xline_basis_length,
                inline_value,
                inline_ordinal,
            ),
        };
    let distance_m = ((section_coordinate - section_axis.requested_coordinate) / section_step)
        .abs()
        * section_basis_length_m;
    if distance_m > tolerance_m {
        return None;
    }

    let trace_index = rounded_trace_index(trace_ordinal, section_axis.trace_count)?;
    Some(ProjectedSectionSample {
        trace_index,
        trace_coordinate: trace_value,
        sample_value: station
            .true_vertical_depth_m
            .or(station.true_vertical_depth_subsea_m),
    })
}

fn rounded_trace_index(trace_ordinal: f64, trace_count: usize) -> Option<usize> {
    let rounded = trace_ordinal.round();
    if !rounded.is_finite() || rounded < 0.0 || rounded > (trace_count.saturating_sub(1)) as f64 {
        return None;
    }
    Some(rounded as usize)
}

fn invert_survey_grid_transform(
    transform: &SurveyMapGridTransformDto,
    x: f64,
    y: f64,
) -> Option<(f64, f64)> {
    let determinant = transform.inline_basis.x * transform.xline_basis.y
        - transform.inline_basis.y * transform.xline_basis.x;
    if determinant.abs() <= f64::EPSILON {
        return None;
    }

    let dx = x - transform.origin.x;
    let dy = y - transform.origin.y;
    let inline_ordinal =
        (dx * transform.xline_basis.y - dy * transform.xline_basis.x) / determinant;
    let xline_ordinal =
        (dy * transform.inline_basis.x - dx * transform.inline_basis.y) / determinant;
    Some((inline_ordinal, xline_ordinal))
}

fn vector_length_m(vector: &ProjectedVector2Dto) -> f64 {
    (vector.x * vector.x + vector.y * vector.y).sqrt()
}

fn planar_distance_m(start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> f64 {
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    (dx * dx + dy * dy).sqrt()
}

fn depth_for_model(
    station: &ResolvedTrajectoryStation,
    depth_reference: DepthReferenceKind,
) -> Option<f64> {
    match depth_reference {
        DepthReferenceKind::MeasuredDepth => Some(station.measured_depth_m),
        DepthReferenceKind::TrueVerticalDepth => station.true_vertical_depth_m,
        DepthReferenceKind::TrueVerticalDepthSubsea => station.true_vertical_depth_subsea_m,
    }
}

#[derive(Clone)]
struct SelectedWellTieCurveSource {
    asset_id: AssetId,
    asset_name: String,
    curve: LogCurveData,
}

struct SelectedWellTieLogSelection {
    density_curve: SelectedWellTieCurveSource,
    velocity_curve: SelectedWellTieCurveSource,
    velocity_source_kind: WellTieVelocitySourceKind,
}

#[derive(Clone)]
struct PreparedInterpolatedLogCurve {
    depths_m: Vec<f64>,
    values: Vec<f64>,
    point_count: usize,
}

#[derive(Clone)]
struct RockPhysicsCurveSource {
    curve_id: String,
    curve: LogCurveData,
    prepared: PreparedInterpolatedLogCurve,
}

struct RockPhysicsPreparedWell {
    well: RockPhysicsWellDto,
    curves_by_semantic: HashMap<CurveSemanticType, Vec<RockPhysicsCurveSource>>,
}

fn select_well_tie_log_selection(
    project: &OphioliteProject,
    wellbore_id: &WellboreId,
) -> Result<SelectedWellTieLogSelection> {
    let log_assets = project.list_assets(wellbore_id, Some(AssetKind::Log))?;
    let mut density_candidates = Vec::new();
    let mut velocity_candidates = Vec::new();
    let mut sonic_candidates = Vec::new();

    for asset in log_assets {
        let asset_name = project.collection_by_id(&asset.collection_id)?.name;
        for curve in project.read_log_curve_data(&asset.id)? {
            let valid_count = curve.values.iter().filter(|value| value.is_some()).count();
            if valid_count < 2 {
                continue;
            }
            let source = SelectedWellTieCurveSource {
                asset_id: asset.id.clone(),
                asset_name: asset_name.clone(),
                curve,
            };
            match source.curve.semantic_type {
                CurveSemanticType::BulkDensity => density_candidates.push((valid_count, source)),
                CurveSemanticType::PVelocity => velocity_candidates.push((valid_count, source)),
                CurveSemanticType::Sonic => sonic_candidates.push((valid_count, source)),
                _ => {}
            }
        }
    }

    density_candidates.sort_by(|left, right| right.0.cmp(&left.0));
    velocity_candidates.sort_by(|left, right| right.0.cmp(&left.0));
    sonic_candidates.sort_by(|left, right| right.0.cmp(&left.0));

    let density_curve = density_candidates
        .into_iter()
        .next()
        .map(|(_, source)| source)
        .ok_or_else(|| {
            LasError::Validation(
                "well tie analysis requires a bulk density log on the selected wellbore"
                    .to_string(),
            )
        })?;
    if let Some((_, velocity_curve)) = velocity_candidates.into_iter().next() {
        return Ok(SelectedWellTieLogSelection {
            density_curve,
            velocity_curve,
            velocity_source_kind: WellTieVelocitySourceKind::PVelocityCurve,
        });
    }
    if let Some((_, velocity_curve)) = sonic_candidates.into_iter().next() {
        return Ok(SelectedWellTieLogSelection {
            density_curve,
            velocity_curve,
            velocity_source_kind: WellTieVelocitySourceKind::SonicCurveConvertedToVp,
        });
    }
    Err(LasError::Validation(
        "well tie analysis requires a P-wave velocity log or a sonic log on the selected wellbore"
            .to_string(),
    ))
}

fn build_well_tie_log_curve_source(
    source: &SelectedWellTieCurveSource,
    point_count: usize,
) -> WellTieLogCurveSource {
    WellTieLogCurveSource {
        asset_id: source.asset_id.0.clone(),
        asset_name: source.asset_name.clone(),
        curve_name: source.curve.curve_name.clone(),
        original_mnemonic: source.curve.original_mnemonic.clone(),
        unit: source.curve.unit.clone(),
        sample_count: point_count,
    }
}

fn prepare_interpolated_log_curve(curve: &LogCurveData) -> Result<PreparedInterpolatedLogCurve> {
    let mut depths_m = Vec::new();
    let mut values = Vec::new();
    for (depth_m, value) in curve.depths.iter().zip(curve.values.iter()) {
        if let Some(value) = value {
            if depth_m.is_finite() && value.is_finite() {
                depths_m.push(*depth_m);
                values.push(*value);
            }
        }
    }
    if depths_m.len() < 2 {
        return Err(LasError::Validation(format!(
            "curve '{}' does not provide enough numeric samples for well-tie interpolation",
            curve.curve_name
        )));
    }
    Ok(PreparedInterpolatedLogCurve {
        point_count: depths_m.len(),
        depths_m,
        values,
    })
}

fn interpolate_prepared_curve(curve: &PreparedInterpolatedLogCurve, depth_m: f64) -> Option<f64> {
    if !depth_m.is_finite() {
        return None;
    }
    let first_depth = *curve.depths_m.first()?;
    let last_depth = *curve.depths_m.last()?;
    if depth_m < first_depth || depth_m > last_depth {
        return None;
    }
    match curve
        .depths_m
        .binary_search_by(|probe| probe.partial_cmp(&depth_m).unwrap())
    {
        Ok(index) => curve.values.get(index).copied(),
        Err(index) => {
            if index == 0 || index >= curve.depths_m.len() {
                return None;
            }
            let left_depth = curve.depths_m[index - 1];
            let right_depth = curve.depths_m[index];
            let left_value = curve.values[index - 1];
            let right_value = curve.values[index];
            let span = right_depth - left_depth;
            if span <= 0.0 {
                Some(left_value)
            } else {
                let weight = ((depth_m - left_depth) / span).clamp(0.0, 1.0);
                Some(left_value + ((right_value - left_value) * weight))
            }
        }
    }
}

struct ResolvedRockPhysicsSemanticBinding {
    curve_id: String,
    derived_channels: Vec<String>,
}

fn validate_rock_physics_request(request: &RockPhysicsCrossplotRequestDto) -> Result<()> {
    if request.schema_version != ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION {
        return Err(LasError::Validation(format!(
            "rock-physics crossplot requests require schema_version {}",
            ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION
        )));
    }
    if request.wellbore_ids.is_empty() {
        return Err(LasError::Validation(
            "rock-physics crossplot request requires at least one wellbore id".to_string(),
        ));
    }
    if let (Some(depth_min), Some(depth_max)) = (request.depth_min, request.depth_max) {
        if depth_min > depth_max {
            return Err(LasError::Validation(
                "rock-physics crossplot request requires depth_min <= depth_max".to_string(),
            ));
        }
    }
    if !rock_physics_template_allows_x(request.template_id, request.x_semantic) {
        return Err(LasError::Validation(format!(
            "template '{}' does not allow x semantic '{}'",
            rock_physics_template_slug(request.template_id),
            rock_physics_curve_semantic_slug(request.x_semantic)
        )));
    }
    if !rock_physics_template_allows_y(request.template_id, request.y_semantic) {
        return Err(LasError::Validation(format!(
            "template '{}' does not allow y semantic '{}'",
            rock_physics_template_slug(request.template_id),
            rock_physics_curve_semantic_slug(request.y_semantic)
        )));
    }
    match &request.color_binding {
        RockPhysicsColorRequestDto::Categorical(binding) => {
            if !rock_physics_template_allows_categorical_color(
                request.template_id,
                binding.semantic,
            ) {
                return Err(LasError::Validation(format!(
                    "template '{}' does not allow categorical color semantic '{}'",
                    rock_physics_template_slug(request.template_id),
                    rock_physics_categorical_semantic_slug(binding.semantic)
                )));
            }
        }
        RockPhysicsColorRequestDto::Continuous(binding) => {
            if !rock_physics_template_allows_continuous_color(request.template_id, binding.semantic)
            {
                return Err(LasError::Validation(format!(
                    "template '{}' does not allow continuous color semantic '{}'",
                    rock_physics_template_slug(request.template_id),
                    rock_physics_curve_semantic_slug(binding.semantic)
                )));
            }
        }
    }
    Ok(())
}

fn rock_physics_template_slug(template_id: RockPhysicsTemplateIdDto) -> &'static str {
    match template_id {
        RockPhysicsTemplateIdDto::VpVsVsAi => "vp-vs-vs-ai",
        RockPhysicsTemplateIdDto::AiVsSi => "ai-vs-si",
        RockPhysicsTemplateIdDto::VpVsVs => "vp-vs-vs",
        RockPhysicsTemplateIdDto::PorosityVsVp => "porosity-vs-vp",
        RockPhysicsTemplateIdDto::LambdaRhoVsMuRho => "lambda-rho-vs-mu-rho",
        RockPhysicsTemplateIdDto::NeutronPorosityVsBulkDensity => {
            "neutron-porosity-vs-bulk-density"
        }
        RockPhysicsTemplateIdDto::PhiVsAi => "phi-vs-ai",
        RockPhysicsTemplateIdDto::PrVsAi => "pr-vs-ai",
        RockPhysicsTemplateIdDto::VpVsDensity => "vp-vs-density",
    }
}

fn rock_physics_template_title(template_id: RockPhysicsTemplateIdDto) -> &'static str {
    match template_id {
        RockPhysicsTemplateIdDto::VpVsVsAi => "Vp/Vs vs AI",
        RockPhysicsTemplateIdDto::AiVsSi => "AI vs SI",
        RockPhysicsTemplateIdDto::VpVsVs => "Vp vs Vs",
        RockPhysicsTemplateIdDto::PorosityVsVp => "Porosity vs Vp",
        RockPhysicsTemplateIdDto::LambdaRhoVsMuRho => "Lambda-Rho vs Mu-Rho",
        RockPhysicsTemplateIdDto::NeutronPorosityVsBulkDensity => {
            "Neutron Porosity vs Bulk Density"
        }
        RockPhysicsTemplateIdDto::PhiVsAi => "Phi vs AI",
        RockPhysicsTemplateIdDto::PrVsAi => "PR vs AI",
        RockPhysicsTemplateIdDto::VpVsDensity => "Vp vs Density",
    }
}

fn rock_physics_template_allows_x(
    template_id: RockPhysicsTemplateIdDto,
    semantic: RockPhysicsCurveSemanticDto,
) -> bool {
    match template_id {
        RockPhysicsTemplateIdDto::VpVsVsAi
        | RockPhysicsTemplateIdDto::AiVsSi
        | RockPhysicsTemplateIdDto::PhiVsAi
        | RockPhysicsTemplateIdDto::PrVsAi => {
            semantic == RockPhysicsCurveSemanticDto::AcousticImpedance
        }
        RockPhysicsTemplateIdDto::VpVsVs => semantic == RockPhysicsCurveSemanticDto::SVelocity,
        RockPhysicsTemplateIdDto::PorosityVsVp => {
            matches!(
                semantic,
                RockPhysicsCurveSemanticDto::NeutronPorosity
                    | RockPhysicsCurveSemanticDto::EffectivePorosity
            )
        }
        RockPhysicsTemplateIdDto::LambdaRhoVsMuRho => {
            semantic == RockPhysicsCurveSemanticDto::LambdaRho
        }
        RockPhysicsTemplateIdDto::NeutronPorosityVsBulkDensity => {
            semantic == RockPhysicsCurveSemanticDto::NeutronPorosity
        }
        RockPhysicsTemplateIdDto::VpVsDensity => {
            semantic == RockPhysicsCurveSemanticDto::BulkDensity
        }
    }
}

fn rock_physics_template_allows_y(
    template_id: RockPhysicsTemplateIdDto,
    semantic: RockPhysicsCurveSemanticDto,
) -> bool {
    match template_id {
        RockPhysicsTemplateIdDto::VpVsVsAi => semantic == RockPhysicsCurveSemanticDto::VpVsRatio,
        RockPhysicsTemplateIdDto::AiVsSi => semantic == RockPhysicsCurveSemanticDto::ShearImpedance,
        RockPhysicsTemplateIdDto::VpVsVs | RockPhysicsTemplateIdDto::PorosityVsVp => {
            semantic == RockPhysicsCurveSemanticDto::PVelocity
        }
        RockPhysicsTemplateIdDto::LambdaRhoVsMuRho => {
            semantic == RockPhysicsCurveSemanticDto::MuRho
        }
        RockPhysicsTemplateIdDto::NeutronPorosityVsBulkDensity
        | RockPhysicsTemplateIdDto::VpVsDensity => {
            semantic == RockPhysicsCurveSemanticDto::BulkDensity
        }
        RockPhysicsTemplateIdDto::PhiVsAi => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::NeutronPorosity
                | RockPhysicsCurveSemanticDto::EffectivePorosity
        ),
        RockPhysicsTemplateIdDto::PrVsAi => semantic == RockPhysicsCurveSemanticDto::PoissonsRatio,
    }
}

fn rock_physics_template_allows_continuous_color(
    template_id: RockPhysicsTemplateIdDto,
    semantic: RockPhysicsCurveSemanticDto,
) -> bool {
    match template_id {
        RockPhysicsTemplateIdDto::VpVsVsAi => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::BulkDensity
                | RockPhysicsCurveSemanticDto::NeutronPorosity
                | RockPhysicsCurveSemanticDto::PoissonsRatio
                | RockPhysicsCurveSemanticDto::GammaRay
        ),
        RockPhysicsTemplateIdDto::AiVsSi => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::BulkDensity
                | RockPhysicsCurveSemanticDto::GammaRay
                | RockPhysicsCurveSemanticDto::NeutronPorosity
        ),
        RockPhysicsTemplateIdDto::VpVsVs => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::GammaRay
                | RockPhysicsCurveSemanticDto::BulkDensity
                | RockPhysicsCurveSemanticDto::NeutronPorosity
        ),
        RockPhysicsTemplateIdDto::PorosityVsVp => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::GammaRay
        ),
        RockPhysicsTemplateIdDto::LambdaRhoVsMuRho => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::GammaRay
                | RockPhysicsCurveSemanticDto::BulkDensity
        ),
        RockPhysicsTemplateIdDto::NeutronPorosityVsBulkDensity => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::GammaRay
                | RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
        ),
        RockPhysicsTemplateIdDto::PhiVsAi => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::BulkDensity
                | RockPhysicsCurveSemanticDto::GammaRay
        ),
        RockPhysicsTemplateIdDto::PrVsAi => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::BulkDensity
                | RockPhysicsCurveSemanticDto::NeutronPorosity
        ),
        RockPhysicsTemplateIdDto::VpVsDensity => matches!(
            semantic,
            RockPhysicsCurveSemanticDto::WaterSaturation
                | RockPhysicsCurveSemanticDto::VShale
                | RockPhysicsCurveSemanticDto::NeutronPorosity
                | RockPhysicsCurveSemanticDto::GammaRay
        ),
    }
}

fn rock_physics_template_allows_categorical_color(
    _template_id: RockPhysicsTemplateIdDto,
    semantic: RockPhysicsCategoricalSemanticDto,
) -> bool {
    matches!(
        semantic,
        RockPhysicsCategoricalSemanticDto::Well
            | RockPhysicsCategoricalSemanticDto::Wellbore
            | RockPhysicsCategoricalSemanticDto::Facies
    )
}

fn rock_physics_curve_semantic_slug(semantic: RockPhysicsCurveSemanticDto) -> &'static str {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity => "p-velocity",
        RockPhysicsCurveSemanticDto::SVelocity => "s-velocity",
        RockPhysicsCurveSemanticDto::VpVsRatio => "vp-vs-ratio",
        RockPhysicsCurveSemanticDto::AcousticImpedance => "acoustic-impedance",
        RockPhysicsCurveSemanticDto::ElasticImpedance => "elastic-impedance",
        RockPhysicsCurveSemanticDto::ExtendedElasticImpedance => "extended-elastic-impedance",
        RockPhysicsCurveSemanticDto::ShearImpedance => "shear-impedance",
        RockPhysicsCurveSemanticDto::LambdaRho => "lambda-rho",
        RockPhysicsCurveSemanticDto::MuRho => "mu-rho",
        RockPhysicsCurveSemanticDto::BulkDensity => "bulk-density",
        RockPhysicsCurveSemanticDto::Resistivity => "resistivity",
        RockPhysicsCurveSemanticDto::Sonic => "sonic",
        RockPhysicsCurveSemanticDto::ShearSonic => "shear-sonic",
        RockPhysicsCurveSemanticDto::PoissonsRatio => "poissons-ratio",
        RockPhysicsCurveSemanticDto::NeutronPorosity => "neutron-porosity",
        RockPhysicsCurveSemanticDto::EffectivePorosity => "effective-porosity",
        RockPhysicsCurveSemanticDto::WaterSaturation => "water-saturation",
        RockPhysicsCurveSemanticDto::VShale => "v-shale",
        RockPhysicsCurveSemanticDto::GammaRay => "gamma-ray",
    }
}

fn rock_physics_categorical_semantic_slug(
    semantic: RockPhysicsCategoricalSemanticDto,
) -> &'static str {
    match semantic {
        RockPhysicsCategoricalSemanticDto::Well => "well",
        RockPhysicsCategoricalSemanticDto::Wellbore => "wellbore",
        RockPhysicsCategoricalSemanticDto::Facies => "facies",
    }
}

fn default_rock_physics_axis_label(semantic: RockPhysicsCurveSemanticDto) -> &'static str {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity => "Vp",
        RockPhysicsCurveSemanticDto::SVelocity => "Vs",
        RockPhysicsCurveSemanticDto::VpVsRatio => "Vp/Vs",
        RockPhysicsCurveSemanticDto::AcousticImpedance => "AI",
        RockPhysicsCurveSemanticDto::ElasticImpedance => "EI",
        RockPhysicsCurveSemanticDto::ExtendedElasticImpedance => "EEI",
        RockPhysicsCurveSemanticDto::ShearImpedance => "SI",
        RockPhysicsCurveSemanticDto::LambdaRho => "Lambda-Rho",
        RockPhysicsCurveSemanticDto::MuRho => "Mu-Rho",
        RockPhysicsCurveSemanticDto::BulkDensity => "Bulk Density",
        RockPhysicsCurveSemanticDto::Resistivity => "Resistivity",
        RockPhysicsCurveSemanticDto::Sonic => "Sonic",
        RockPhysicsCurveSemanticDto::ShearSonic => "Shear Sonic",
        RockPhysicsCurveSemanticDto::PoissonsRatio => "Poisson's Ratio",
        RockPhysicsCurveSemanticDto::NeutronPorosity => "Neutron Porosity",
        RockPhysicsCurveSemanticDto::EffectivePorosity => "Effective Porosity",
        RockPhysicsCurveSemanticDto::WaterSaturation => "Water Saturation",
        RockPhysicsCurveSemanticDto::VShale => "V-Shale",
        RockPhysicsCurveSemanticDto::GammaRay => "Gamma Ray",
    }
}

fn rock_physics_semantic_unit(semantic: RockPhysicsCurveSemanticDto) -> Option<&'static str> {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity | RockPhysicsCurveSemanticDto::SVelocity => {
            Some("m/s")
        }
        RockPhysicsCurveSemanticDto::VpVsRatio | RockPhysicsCurveSemanticDto::PoissonsRatio => {
            Some("ratio")
        }
        RockPhysicsCurveSemanticDto::AcousticImpedance
        | RockPhysicsCurveSemanticDto::ElasticImpedance
        | RockPhysicsCurveSemanticDto::ExtendedElasticImpedance
        | RockPhysicsCurveSemanticDto::ShearImpedance => Some("(m/s)*(g/cc)"),
        RockPhysicsCurveSemanticDto::LambdaRho | RockPhysicsCurveSemanticDto::MuRho => Some("GPa"),
        RockPhysicsCurveSemanticDto::BulkDensity => Some("g/cc"),
        RockPhysicsCurveSemanticDto::Resistivity => Some("ohm.m"),
        RockPhysicsCurveSemanticDto::Sonic | RockPhysicsCurveSemanticDto::ShearSonic => {
            Some("us/m")
        }
        RockPhysicsCurveSemanticDto::NeutronPorosity
        | RockPhysicsCurveSemanticDto::EffectivePorosity => Some("%"),
        RockPhysicsCurveSemanticDto::WaterSaturation | RockPhysicsCurveSemanticDto::VShale => {
            Some("fraction")
        }
        RockPhysicsCurveSemanticDto::GammaRay => Some("gAPI"),
    }
}

fn resolve_rock_physics_color_binding(
    request: &RockPhysicsCrossplotRequestDto,
    samples: &[RockPhysicsSampleDto],
) -> crate::project_contracts::RockPhysicsColorBindingDto {
    match &request.color_binding {
        RockPhysicsColorRequestDto::Categorical(binding) => {
            crate::project_contracts::RockPhysicsColorBindingDto::Categorical(
                RockPhysicsCategoricalColorBindingDto {
                    label: binding.label.clone().or_else(|| {
                        Some(
                            match binding.semantic {
                                RockPhysicsCategoricalSemanticDto::Well => "Well",
                                RockPhysicsCategoricalSemanticDto::Wellbore => "Wellbore",
                                RockPhysicsCategoricalSemanticDto::Facies => "Facies",
                            }
                            .to_string(),
                        )
                    }),
                    semantic: binding.semantic,
                    categories: match binding.semantic {
                        RockPhysicsCategoricalSemanticDto::Well
                        | RockPhysicsCategoricalSemanticDto::Wellbore => None,
                        RockPhysicsCategoricalSemanticDto::Facies => {
                            Some(Vec::<RockPhysicsCategoryDto>::new())
                        }
                    },
                },
            )
        }
        RockPhysicsColorRequestDto::Continuous(binding) => {
            let (min_value, max_value) =
                derive_rock_physics_range(samples.iter().filter_map(|sample| sample.color_value));
            crate::project_contracts::RockPhysicsColorBindingDto::Continuous(
                RockPhysicsContinuousColorBindingDto {
                    label: binding.label.clone().or_else(|| {
                        Some(default_rock_physics_axis_label(binding.semantic).to_string())
                    }),
                    semantic: binding.semantic,
                    min_value,
                    max_value,
                    palette: None,
                },
            )
        }
    }
}

fn direct_rock_physics_source<'a>(
    prepared: &'a RockPhysicsPreparedWell,
    semantic_type: CurveSemanticType,
) -> Option<&'a RockPhysicsCurveSource> {
    prepared
        .curves_by_semantic
        .get(&semantic_type)
        .and_then(|candidates| candidates.first())
}

fn primary_rock_physics_anchor<'a>(
    prepared: &'a RockPhysicsPreparedWell,
    x_semantic: RockPhysicsCurveSemanticDto,
    y_semantic: RockPhysicsCurveSemanticDto,
    color_semantic: Option<RockPhysicsCurveSemanticDto>,
) -> Option<&'a RockPhysicsCurveSource> {
    primary_rock_physics_source(prepared, x_semantic)
        .or_else(|| primary_rock_physics_source(prepared, y_semantic))
        .or_else(|| {
            color_semantic.and_then(|semantic| primary_rock_physics_source(prepared, semantic))
        })
}

fn primary_rock_physics_source<'a>(
    prepared: &'a RockPhysicsPreparedWell,
    semantic: RockPhysicsCurveSemanticDto,
) -> Option<&'a RockPhysicsCurveSource> {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity => {
            direct_rock_physics_source(prepared, CurveSemanticType::PVelocity)
                .or_else(|| direct_rock_physics_source(prepared, CurveSemanticType::Sonic))
        }
        RockPhysicsCurveSemanticDto::SVelocity => {
            direct_rock_physics_source(prepared, CurveSemanticType::SVelocity)
                .or_else(|| direct_rock_physics_source(prepared, CurveSemanticType::ShearSonic))
        }
        RockPhysicsCurveSemanticDto::VpVsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::VpVsRatio)
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::PVelocity)
                })
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::SVelocity)
                })
        }
        RockPhysicsCurveSemanticDto::AcousticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::AcousticImpedance)
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::PVelocity)
                })
                .or_else(|| direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity))
        }
        RockPhysicsCurveSemanticDto::ElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ElasticImpedance)
        }
        RockPhysicsCurveSemanticDto::ExtendedElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ExtendedElasticImpedance)
        }
        RockPhysicsCurveSemanticDto::ShearImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearImpedance)
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::SVelocity)
                })
                .or_else(|| direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity))
        }
        RockPhysicsCurveSemanticDto::LambdaRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::LambdaRho)
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::PVelocity)
                })
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::SVelocity)
                })
                .or_else(|| direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity))
        }
        RockPhysicsCurveSemanticDto::MuRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::MuRho)
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::SVelocity)
                })
                .or_else(|| direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity))
        }
        RockPhysicsCurveSemanticDto::BulkDensity => {
            direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity)
        }
        RockPhysicsCurveSemanticDto::Resistivity => {
            direct_rock_physics_source(prepared, CurveSemanticType::DeepResistivity)
                .or_else(|| {
                    direct_rock_physics_source(prepared, CurveSemanticType::MediumResistivity)
                })
                .or_else(|| {
                    direct_rock_physics_source(prepared, CurveSemanticType::ShallowResistivity)
                })
        }
        RockPhysicsCurveSemanticDto::Sonic => {
            direct_rock_physics_source(prepared, CurveSemanticType::Sonic)
        }
        RockPhysicsCurveSemanticDto::ShearSonic => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearSonic)
        }
        RockPhysicsCurveSemanticDto::PoissonsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::PoissonsRatio)
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::PVelocity)
                })
                .or_else(|| {
                    primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::SVelocity)
                })
        }
        RockPhysicsCurveSemanticDto::NeutronPorosity => {
            direct_rock_physics_source(prepared, CurveSemanticType::NeutronPorosity)
        }
        RockPhysicsCurveSemanticDto::EffectivePorosity => {
            direct_rock_physics_source(prepared, CurveSemanticType::EffectivePorosity)
        }
        RockPhysicsCurveSemanticDto::WaterSaturation => {
            direct_rock_physics_source(prepared, CurveSemanticType::WaterSaturation)
        }
        RockPhysicsCurveSemanticDto::VShale => {
            direct_rock_physics_source(prepared, CurveSemanticType::VShale)
        }
        RockPhysicsCurveSemanticDto::GammaRay => {
            direct_rock_physics_source(prepared, CurveSemanticType::GammaRay)
        }
    }
}

fn resolve_rock_physics_semantic_binding(
    prepared: &RockPhysicsPreparedWell,
    semantic: RockPhysicsCurveSemanticDto,
    well_name: &str,
) -> Result<ResolvedRockPhysicsSemanticBinding> {
    let direct_curve_id =
        primary_rock_physics_source(prepared, semantic).map(|source| source.curve_id.clone());
    if !rock_physics_semantic_is_available(prepared, semantic) {
        return Err(LasError::Validation(format!(
            "well '{}' does not provide the required '{}' rock-physics semantic",
            well_name,
            rock_physics_curve_semantic_slug(semantic)
        )));
    }
    let derived = !rock_physics_has_direct_binding(prepared, semantic);
    Ok(ResolvedRockPhysicsSemanticBinding {
        curve_id: direct_curve_id
            .unwrap_or_else(|| format!("derived:{}", rock_physics_curve_semantic_slug(semantic))),
        derived_channels: derived
            .then_some(vec![rock_physics_curve_semantic_slug(semantic).to_string()])
            .unwrap_or_default(),
    })
}

fn rock_physics_has_direct_binding(
    prepared: &RockPhysicsPreparedWell,
    semantic: RockPhysicsCurveSemanticDto,
) -> bool {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity => {
            direct_rock_physics_source(prepared, CurveSemanticType::PVelocity).is_some()
        }
        RockPhysicsCurveSemanticDto::SVelocity => {
            direct_rock_physics_source(prepared, CurveSemanticType::SVelocity).is_some()
        }
        RockPhysicsCurveSemanticDto::VpVsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::VpVsRatio).is_some()
        }
        RockPhysicsCurveSemanticDto::AcousticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::AcousticImpedance).is_some()
        }
        RockPhysicsCurveSemanticDto::ElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ElasticImpedance).is_some()
        }
        RockPhysicsCurveSemanticDto::ExtendedElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ExtendedElasticImpedance)
                .is_some()
        }
        RockPhysicsCurveSemanticDto::ShearImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearImpedance).is_some()
        }
        RockPhysicsCurveSemanticDto::LambdaRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::LambdaRho).is_some()
        }
        RockPhysicsCurveSemanticDto::MuRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::MuRho).is_some()
        }
        RockPhysicsCurveSemanticDto::BulkDensity => {
            direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity).is_some()
        }
        RockPhysicsCurveSemanticDto::Resistivity => {
            direct_rock_physics_source(prepared, CurveSemanticType::DeepResistivity).is_some()
                || direct_rock_physics_source(prepared, CurveSemanticType::MediumResistivity)
                    .is_some()
                || direct_rock_physics_source(prepared, CurveSemanticType::ShallowResistivity)
                    .is_some()
        }
        RockPhysicsCurveSemanticDto::Sonic => {
            direct_rock_physics_source(prepared, CurveSemanticType::Sonic).is_some()
        }
        RockPhysicsCurveSemanticDto::ShearSonic => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearSonic).is_some()
        }
        RockPhysicsCurveSemanticDto::PoissonsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::PoissonsRatio).is_some()
        }
        RockPhysicsCurveSemanticDto::NeutronPorosity => {
            direct_rock_physics_source(prepared, CurveSemanticType::NeutronPorosity).is_some()
        }
        RockPhysicsCurveSemanticDto::EffectivePorosity => {
            direct_rock_physics_source(prepared, CurveSemanticType::EffectivePorosity).is_some()
        }
        RockPhysicsCurveSemanticDto::WaterSaturation => {
            direct_rock_physics_source(prepared, CurveSemanticType::WaterSaturation).is_some()
        }
        RockPhysicsCurveSemanticDto::VShale => {
            direct_rock_physics_source(prepared, CurveSemanticType::VShale).is_some()
        }
        RockPhysicsCurveSemanticDto::GammaRay => {
            direct_rock_physics_source(prepared, CurveSemanticType::GammaRay).is_some()
        }
    }
}

fn rock_physics_semantic_is_available(
    prepared: &RockPhysicsPreparedWell,
    semantic: RockPhysicsCurveSemanticDto,
) -> bool {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity => {
            primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::PVelocity).is_some()
        }
        RockPhysicsCurveSemanticDto::SVelocity => {
            primary_rock_physics_source(prepared, RockPhysicsCurveSemanticDto::SVelocity).is_some()
        }
        RockPhysicsCurveSemanticDto::VpVsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::VpVsRatio).is_some()
                || (rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::PVelocity,
                ) && rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::SVelocity,
                ))
        }
        RockPhysicsCurveSemanticDto::AcousticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::AcousticImpedance).is_some()
                || (rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::PVelocity,
                ) && rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::BulkDensity,
                ))
        }
        RockPhysicsCurveSemanticDto::ElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ElasticImpedance).is_some()
        }
        RockPhysicsCurveSemanticDto::ExtendedElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ExtendedElasticImpedance)
                .is_some()
        }
        RockPhysicsCurveSemanticDto::ShearImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearImpedance).is_some()
                || (rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::SVelocity,
                ) && rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::BulkDensity,
                ))
        }
        RockPhysicsCurveSemanticDto::LambdaRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::LambdaRho).is_some()
                || (rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::PVelocity,
                ) && rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::SVelocity,
                ) && rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::BulkDensity,
                ))
        }
        RockPhysicsCurveSemanticDto::MuRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::MuRho).is_some()
                || (rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::SVelocity,
                ) && rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::BulkDensity,
                ))
        }
        RockPhysicsCurveSemanticDto::PoissonsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::PoissonsRatio).is_some()
                || rock_physics_semantic_is_available(
                    prepared,
                    RockPhysicsCurveSemanticDto::VpVsRatio,
                )
        }
        _ => primary_rock_physics_source(prepared, semantic).is_some(),
    }
}

fn collect_derived_channels<'a>(
    bindings: [Option<&'a ResolvedRockPhysicsSemanticBinding>; 3],
) -> Vec<String> {
    let mut result = Vec::new();
    for binding in bindings.into_iter().flatten() {
        for channel in &binding.derived_channels {
            if !result.contains(channel) {
                result.push(channel.clone());
            }
        }
    }
    result
}

fn derive_rock_physics_range<I>(values: I) -> (Option<f64>, Option<f64>)
where
    I: IntoIterator<Item = f64>,
{
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;

    for value in values {
        if !value.is_finite() {
            continue;
        }
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    if min_value.is_infinite() || max_value.is_infinite() {
        (None, None)
    } else {
        (Some(min_value), Some(max_value))
    }
}

fn rock_physics_value_at_depth(
    prepared: &RockPhysicsPreparedWell,
    semantic: RockPhysicsCurveSemanticDto,
    depth_m: f64,
) -> Option<f64> {
    match semantic {
        RockPhysicsCurveSemanticDto::PVelocity => {
            direct_rock_physics_source(prepared, CurveSemanticType::PVelocity)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    direct_rock_physics_source(prepared, CurveSemanticType::Sonic).and_then(
                        |source| {
                            interpolate_prepared_curve(&source.prepared, depth_m).and_then(
                                |value| {
                                    sonic_value_to_velocity_m_s(value, source.curve.unit.as_deref())
                                },
                            )
                        },
                    )
                })
        }
        RockPhysicsCurveSemanticDto::SVelocity => {
            direct_rock_physics_source(prepared, CurveSemanticType::SVelocity)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    direct_rock_physics_source(prepared, CurveSemanticType::ShearSonic).and_then(
                        |source| {
                            interpolate_prepared_curve(&source.prepared, depth_m).and_then(
                                |value| {
                                    sonic_value_to_velocity_m_s(value, source.curve.unit.as_deref())
                                },
                            )
                        },
                    )
                })
        }
        RockPhysicsCurveSemanticDto::VpVsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::VpVsRatio)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    let vp = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::PVelocity,
                        depth_m,
                    )?;
                    let vs = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::SVelocity,
                        depth_m,
                    )?;
                    (vs > 0.0).then_some(vp / vs)
                })
        }
        RockPhysicsCurveSemanticDto::AcousticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::AcousticImpedance)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    let vp = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::PVelocity,
                        depth_m,
                    )?;
                    let density = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::BulkDensity,
                        depth_m,
                    )?;
                    Some(vp * density)
                })
        }
        RockPhysicsCurveSemanticDto::ElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ElasticImpedance)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::ExtendedElasticImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ExtendedElasticImpedance)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::ShearImpedance => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearImpedance)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    let vs = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::SVelocity,
                        depth_m,
                    )?;
                    let density = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::BulkDensity,
                        depth_m,
                    )?;
                    Some(vs * density)
                })
        }
        RockPhysicsCurveSemanticDto::LambdaRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::LambdaRho)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    let vp = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::PVelocity,
                        depth_m,
                    )?;
                    let vs = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::SVelocity,
                        depth_m,
                    )?;
                    let density = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::BulkDensity,
                        depth_m,
                    )?;
                    let vp_km = vp / 1000.0;
                    let vs_km = vs / 1000.0;
                    Some(density * (vp_km * vp_km - (2.0 * vs_km * vs_km)))
                })
        }
        RockPhysicsCurveSemanticDto::MuRho => {
            direct_rock_physics_source(prepared, CurveSemanticType::MuRho)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    let vs = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::SVelocity,
                        depth_m,
                    )?;
                    let density = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::BulkDensity,
                        depth_m,
                    )?;
                    let vs_km = vs / 1000.0;
                    Some(density * vs_km * vs_km)
                })
        }
        RockPhysicsCurveSemanticDto::BulkDensity => {
            direct_rock_physics_source(prepared, CurveSemanticType::BulkDensity)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .and_then(normalize_density_gcc)
        }
        RockPhysicsCurveSemanticDto::Resistivity => primary_rock_physics_source(prepared, semantic)
            .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m)),
        RockPhysicsCurveSemanticDto::Sonic => {
            direct_rock_physics_source(prepared, CurveSemanticType::Sonic)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::ShearSonic => {
            direct_rock_physics_source(prepared, CurveSemanticType::ShearSonic)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::PoissonsRatio => {
            direct_rock_physics_source(prepared, CurveSemanticType::PoissonsRatio)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
                .or_else(|| {
                    let ratio = rock_physics_value_at_depth(
                        prepared,
                        RockPhysicsCurveSemanticDto::VpVsRatio,
                        depth_m,
                    )?;
                    let squared = ratio * ratio;
                    let denominator = 2.0 * (squared - 1.0);
                    (denominator.abs() > 1e-6).then_some((squared - 2.0) / denominator)
                })
        }
        RockPhysicsCurveSemanticDto::NeutronPorosity => {
            direct_rock_physics_source(prepared, CurveSemanticType::NeutronPorosity)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::EffectivePorosity => {
            direct_rock_physics_source(prepared, CurveSemanticType::EffectivePorosity)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::WaterSaturation => {
            direct_rock_physics_source(prepared, CurveSemanticType::WaterSaturation)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::VShale => {
            direct_rock_physics_source(prepared, CurveSemanticType::VShale)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
        RockPhysicsCurveSemanticDto::GammaRay => {
            direct_rock_physics_source(prepared, CurveSemanticType::GammaRay)
                .and_then(|source| interpolate_prepared_curve(&source.prepared, depth_m))
        }
    }
}

fn sonic_value_to_velocity_m_s(raw_value: f64, unit: Option<&str>) -> Option<f64> {
    if !raw_value.is_finite() || raw_value <= 0.0 {
        return None;
    }
    let normalized = unit.unwrap_or_default().trim().to_ascii_lowercase();
    if normalized.contains("us/ft") {
        Some(304_800.0 / raw_value)
    } else {
        Some(1_000_000.0 / raw_value)
    }
}

fn normalize_density_gcc(value: f64) -> Option<f64> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    if value > 10.0 {
        Some(value / 1000.0)
    } else {
        Some(value)
    }
}

fn velocity_value_for_well_tie(
    raw_value: f64,
    source_kind: WellTieVelocitySourceKind,
) -> Option<f32> {
    if !raw_value.is_finite() || raw_value <= 0.0 {
        return None;
    }
    let velocity_m_per_s = match source_kind {
        WellTieVelocitySourceKind::PVelocityCurve => raw_value,
        WellTieVelocitySourceKind::SonicCurveConvertedToVp => {
            if raw_value > 1_000.0 {
                1_000_000.0 / raw_value
            } else {
                304_800.0 / raw_value
            }
        }
    };
    velocity_m_per_s
        .is_finite()
        .then_some(velocity_m_per_s as f32)
}

fn fill_missing_f32_series(values: &[Option<f32>], label: &str) -> Result<Vec<f32>> {
    let first_valid = values
        .iter()
        .position(|value| value.is_some())
        .ok_or_else(|| {
            LasError::Validation(format!(
                "{label} does not overlap the selected well time-depth interval"
            ))
        })?;
    let last_valid = values.iter().rposition(|value| value.is_some()).unwrap();
    let mut output = values
        .iter()
        .map(|value| value.unwrap_or(0.0))
        .collect::<Vec<_>>();
    for index in 0..first_valid {
        output[index] = output[first_valid];
    }
    for index in (last_valid + 1)..output.len() {
        output[index] = output[last_valid];
    }
    let mut cursor = first_valid;
    while cursor < last_valid {
        if values[cursor].is_some() {
            cursor += 1;
            continue;
        }
        let start = cursor - 1;
        let mut end = cursor + 1;
        while end < values.len() && values[end].is_none() {
            end += 1;
        }
        if end >= values.len() {
            break;
        }
        let left = output[start];
        let right = output[end];
        let span = (end - start) as f32;
        for fill_index in cursor..end {
            let weight = (fill_index - start) as f32 / span;
            output[fill_index] = left + ((right - left) * weight);
        }
        cursor = end + 1;
    }
    Ok(output)
}

fn infer_mean_time_step_ms(samples: &[ophiolite_seismic::TimeDepthSample1D]) -> Option<f32> {
    let mut sum = 0.0_f64;
    let mut count = 0_u32;
    for window in samples.windows(2) {
        let step = f64::from(window[1].time_ms - window[0].time_ms);
        if step.is_finite() && step > 0.0 {
            sum += step;
            count += 1;
        }
    }
    (count > 0).then_some((sum / f64::from(count)) as f32)
}

fn acoustic_impedance_to_reflectivity(acoustic_impedance: &[f32]) -> Vec<f32> {
    let mut reflectivity = vec![0.0; acoustic_impedance.len()];
    for index in 1..acoustic_impedance.len() {
        let left = acoustic_impedance[index - 1];
        let right = acoustic_impedance[index];
        let denominator = (left + right).abs().max(1.0e-6);
        reflectivity[index] = (right - left) / denominator;
    }
    reflectivity
}

fn build_provisional_well_tie_wavelet(sample_step_ms: f32, frequency_hz: f32) -> WellTieWavelet {
    let half_count = 24_i32;
    let mut times_ms = Vec::with_capacity((half_count * 2 + 1) as usize);
    let mut amplitudes = Vec::with_capacity((half_count * 2 + 1) as usize);
    let pi_squared = std::f64::consts::PI * std::f64::consts::PI;
    let frequency_squared = f64::from(frequency_hz).powi(2);
    for sample_index in -half_count..=half_count {
        let time_ms = sample_index as f32 * sample_step_ms;
        let time_seconds = f64::from(time_ms) / 1000.0;
        let argument = pi_squared * frequency_squared * time_seconds.powi(2);
        let amplitude = (1.0 - (2.0 * argument)) * (-argument).exp();
        times_ms.push(time_ms);
        amplitudes.push(amplitude as f32);
    }
    normalize_trace_in_place(&mut amplitudes);
    WellTieWavelet {
        id: "provisional-wavelet".to_string(),
        label: "Provisional Wavelet".to_string(),
        times_ms,
        amplitudes,
    }
}

fn convolve_same(signal: &[f32], kernel: &[f32]) -> Vec<f32> {
    if signal.is_empty() || kernel.is_empty() {
        return Vec::new();
    }
    let kernel_half = kernel.len() / 2;
    let mut output = vec![0.0; signal.len()];
    for output_index in 0..signal.len() {
        let mut sum = 0.0_f32;
        for (kernel_index, coefficient) in kernel.iter().enumerate() {
            let signal_index = output_index as isize + kernel_index as isize - kernel_half as isize;
            if signal_index >= 0 && (signal_index as usize) < signal.len() {
                sum += signal[signal_index as usize] * coefficient;
            }
        }
        output[output_index] = sum;
    }
    normalize_trace_in_place(&mut output);
    output
}

fn create_matched_trace(
    seed: &[f32],
    bulk_shift_samples: i32,
    amplitude_scale: f32,
    noise_scale: f32,
) -> Vec<f32> {
    let mut output = Vec::with_capacity(seed.len());
    for index in 0..seed.len() {
        let shifted_index = clamp_trace_index(index as i32 + bulk_shift_samples, seed.len());
        let deterministic_noise = (f32::sin(index as f32 / 8.5) * noise_scale)
            + (f32::cos(index as f32 / 15.0) * noise_scale * 0.6);
        output.push((seed[shifted_index] * amplitude_scale) + deterministic_noise);
    }
    normalize_trace_in_place(&mut output);
    output
}

fn build_well_tie_section_window(
    times_ms: &[f32],
    seed_trace: &[f32],
    search_radius_m: f32,
) -> WellTieSectionWindow {
    let trace_count = if search_radius_m > 0.0 { 17 } else { 1 };
    let trace_offsets_m = if trace_count == 1 {
        vec![0.0]
    } else {
        (0..trace_count)
            .map(|index| {
                let normalized = index as f32 / (trace_count - 1) as f32;
                -search_radius_m + (2.0 * search_radius_m * normalized)
            })
            .collect::<Vec<_>>()
    };
    let sample_count = times_ms.len();
    let well_trace_index = trace_count / 2;
    let mut amplitudes = vec![0.0; trace_count * sample_count];
    for trace_index in 0..trace_count {
        let shift_samples = trace_index as i32 - well_trace_index as i32;
        let energy_scale = (0.94 - (shift_samples.unsigned_abs() as f32 * 0.03)).max(0.6);
        let lateral_bias = shift_samples as f32 * 0.045;
        for sample_index in 0..sample_count {
            let source_index = clamp_trace_index(sample_index as i32 + shift_samples, sample_count);
            let noise = (f32::sin(sample_index as f32 / 13.0 + trace_index as f32 * 0.7) * 0.07)
                + (f32::cos(sample_index as f32 / 7.5 + trace_index as f32 * 0.35) * 0.045);
            amplitudes[trace_index * sample_count + sample_index] =
                (seed_trace[source_index] * energy_scale) + noise + lateral_bias;
        }
    }
    normalize_trace_in_place(&mut amplitudes);
    WellTieSectionWindow {
        id: "local-seismic-window".to_string(),
        label: "Local Seismic Window".to_string(),
        times_ms: times_ms.to_vec(),
        trace_offsets_m,
        amplitudes,
        trace_count,
        sample_count,
        well_trace_index,
    }
}

fn clamp_trace_index(index: i32, len: usize) -> usize {
    index.clamp(0, (len.saturating_sub(1)) as i32) as usize
}

fn normalize_trace_in_place(values: &mut [f32]) {
    let max_abs = values
        .iter()
        .fold(0.0_f32, |current, value| current.max(value.abs()));
    if max_abs > 0.0 {
        for value in values {
            *value /= max_abs;
        }
    }
}

fn trace_correlation(left: &[f32], right: &[f32]) -> Option<f32> {
    let count = left.len().min(right.len());
    if count < 2 {
        return None;
    }
    let left_mean = left.iter().take(count).copied().sum::<f32>() / count as f32;
    let right_mean = right.iter().take(count).copied().sum::<f32>() / count as f32;
    let mut numerator = 0.0_f32;
    let mut left_energy = 0.0_f32;
    let mut right_energy = 0.0_f32;
    for index in 0..count {
        let left_value = left[index] - left_mean;
        let right_value = right[index] - right_mean;
        numerator += left_value * right_value;
        left_energy += left_value * left_value;
        right_energy += right_value * right_value;
    }
    let denominator = f32::sqrt(left_energy * right_energy);
    (denominator > 0.0).then_some(numerator / denominator)
}

fn interpolate_well_time_depth_model_ms(model: &WellTimeDepthModel1D, depth_m: f64) -> Option<f64> {
    let (first, last) = (model.samples.first()?, model.samples.last()?);
    if depth_m < f64::from(first.depth) || depth_m > f64::from(last.depth) {
        return None;
    }
    for pair in model.samples.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        let start_depth = f64::from(start.depth);
        let end_depth = f64::from(end.depth);
        if depth_m < start_depth || depth_m > end_depth {
            continue;
        }
        if (end_depth - start_depth).abs() <= f64::EPSILON {
            return Some(f64::from(start.time_ms));
        }
        let fraction = (depth_m - start_depth) / (end_depth - start_depth);
        let start_time_ms = f64::from(start.time_ms);
        let end_time_ms = f64::from(end.time_ms);
        return Some(lerp(start_time_ms, end_time_ms, fraction));
    }
    model.samples.last().and_then(|sample| {
        ((f64::from(sample.depth) - depth_m).abs() <= f64::EPSILON)
            .then_some(f64::from(sample.time_ms))
    })
}

fn display_time_ms(
    time_ms: f64,
    source_reference: TravelTimeReference,
    display_reference: TravelTimeReference,
) -> f64 {
    match (source_reference, display_reference) {
        (TravelTimeReference::OneWay, TravelTimeReference::TwoWay) => time_ms * 2.0,
        (TravelTimeReference::TwoWay, TravelTimeReference::OneWay) => time_ms * 0.5,
        _ => time_ms,
    }
}

fn validate_well_time_depth_model(model: &WellTimeDepthModel1D) -> Result<()> {
    if model.samples.is_empty() {
        return Err(LasError::Validation(
            "well time-depth model requires at least one sample".to_string(),
        ));
    }
    for pair in model.samples.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        if end.depth < start.depth {
            return Err(LasError::Validation(
                "well time-depth model depths must be monotonically increasing".to_string(),
            ));
        }
        if end.time_ms < start.time_ms {
            return Err(LasError::Validation(
                "well time-depth model times must be monotonically increasing".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_checkshot_vsp_observation_set(
    observation_set: &CheckshotVspObservationSet1D,
) -> Result<()> {
    validate_well_time_depth_observation_samples(
        &observation_set.samples,
        "checkshot/VSP observation set",
    )
}

fn validate_manual_time_depth_pick_set(pick_set: &ManualTimeDepthPickSet1D) -> Result<()> {
    validate_well_time_depth_observation_samples(&pick_set.samples, "manual time-depth pick set")
}

fn validate_well_tie_observation_set(observation_set: &WellTieObservationSet1D) -> Result<()> {
    validate_well_time_depth_observation_samples(
        &observation_set.samples,
        "well tie observation set",
    )?;
    for (label, value) in [
        ("tie_window_start_ms", observation_set.tie_window_start_ms),
        ("tie_window_end_ms", observation_set.tie_window_end_ms),
    ] {
        if value.is_some_and(|candidate| !candidate.is_finite()) {
            return Err(LasError::Validation(format!(
                "well tie observation set {label} must be finite when present"
            )));
        }
    }
    for (label, value) in [
        (
            "trace_search_radius_m",
            observation_set.trace_search_radius_m,
        ),
        ("bulk_shift_ms", observation_set.bulk_shift_ms),
        ("stretch_factor", observation_set.stretch_factor),
        (
            "trace_search_offset_m",
            observation_set.trace_search_offset_m,
        ),
        ("correlation", observation_set.correlation),
    ] {
        if value.is_some_and(|candidate| !candidate.is_finite()) {
            return Err(LasError::Validation(format!(
                "well tie observation set {label} must be finite when present"
            )));
        }
    }
    Ok(())
}

fn validate_well_time_depth_authored_model(model: &WellTimeDepthAuthoredModel1D) -> Result<()> {
    if model.wellbore_id.trim().is_empty() {
        return Err(LasError::Validation(
            "well time-depth authored model requires a wellbore id".to_string(),
        ));
    }
    if model.resolved_trajectory_fingerprint.trim().is_empty() {
        return Err(LasError::Validation(
            "well time-depth authored model requires a resolved trajectory fingerprint".to_string(),
        ));
    }
    if let Some(step_m) = model.sampling_step_m {
        if !step_m.is_finite() || step_m <= 0.0 {
            return Err(LasError::Validation(
                "well time-depth authored model sampling_step_m must be positive".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_well_time_depth_observation_samples(
    samples: &[ophiolite_seismic::WellTimeDepthObservationSample],
    label: &str,
) -> Result<()> {
    for pair in samples.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        if end.depth_m < start.depth_m {
            return Err(LasError::Validation(format!(
                "{label} depths must be monotonically increasing"
            )));
        }
        if end.time_ms < start.time_ms {
            return Err(LasError::Validation(format!(
                "{label} times must be monotonically increasing"
            )));
        }
    }
    Ok(())
}

fn compile_well_time_depth_authored_model(
    authored_model: &WellTimeDepthAuthoredModel1D,
    resolved_trajectory: &ResolvedTrajectoryGeometry,
    project: &OphioliteProject,
) -> Result<WellTimeDepthModel1D> {
    validate_well_time_depth_authored_model(authored_model)?;

    let mut bindings = authored_model
        .source_bindings
        .iter()
        .filter(|binding| binding.enabled)
        .cloned()
        .collect::<Vec<_>>();
    bindings.sort_by_key(|binding| binding.priority);

    let mut source_sets = Vec::new();
    for binding in &bindings {
        source_sets.push((
            binding.clone(),
            read_well_time_depth_source_samples(project, authored_model, binding)?,
        ));
    }

    let mut trajectory_depths = resolved_trajectory
        .stations
        .iter()
        .filter_map(|station| depth_for_model(station, authored_model.depth_reference))
        .collect::<Vec<_>>();
    trajectory_depths
        .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let source_depth_range = source_sets
        .iter()
        .flat_map(|(_, samples)| samples.iter().map(|sample| f64::from(sample.depth)))
        .fold(None, |acc, depth| match acc {
            None => Some((depth, depth)),
            Some((min_depth, max_depth)) => Some((min_depth.min(depth), max_depth.max(depth))),
        });
    let (depth_start, depth_stop) = match (
        trajectory_depths.first().copied(),
        trajectory_depths.last().copied(),
        source_depth_range,
    ) {
        (Some(start), Some(stop), _) => (start, stop),
        (None, None, Some((start, stop))) => (start, stop),
        _ => {
            return Err(LasError::Validation(
                "authored well time-depth model compilation requires trajectory depth coverage or source depth coverage".to_string(),
            ))
        }
    };

    let sampling_step_m = authored_model.sampling_step_m.unwrap_or(10.0);
    let mut compiled_samples: Vec<ophiolite_seismic::TimeDepthSample1D> = Vec::new();
    let mut depth_m = depth_start;
    while depth_m <= depth_stop + (sampling_step_m * 0.5) {
        let source_time_ms = source_sets.iter().find_map(|(binding, samples)| {
            if depth_outside_binding_interval(depth_m, binding) {
                return None;
            }
            interpolate_observation_time_ms(samples, depth_m)
        });

        let time_ms = match source_time_ms {
            Some(time_ms) => time_ms,
            None => {
                let anchor_sample = compiled_samples
                    .last()
                    .map(|sample| ophiolite_seismic::TimeDepthSample1D {
                        depth: sample.depth,
                        time_ms: sample.time_ms,
                    })
                    .or_else(|| {
                        source_sets
                            .iter()
                            .flat_map(|(_, samples)| samples.iter())
                            .find(|sample| f64::from(sample.depth) >= depth_m)
                            .cloned()
                    });
                let Some(anchor_sample) = anchor_sample else {
                    depth_m += sampling_step_m;
                    continue;
                };
                let Some(assumed_time_ms) = assumption_time_ms_for_depth(
                    &authored_model.assumption_intervals,
                    depth_m,
                    &anchor_sample,
                    authored_model.travel_time_reference,
                    source_time_ms.is_some(),
                ) else {
                    depth_m += sampling_step_m;
                    continue;
                };
                assumed_time_ms
            }
        };

        compiled_samples.push(ophiolite_seismic::TimeDepthSample1D {
            depth: depth_m as f32,
            time_ms: time_ms as f32,
        });
        depth_m += sampling_step_m;
    }

    let source_kind = bindings
        .first()
        .map(|binding| binding.source_kind)
        .unwrap_or(TimeDepthTransformSourceKind::ConstantVelocity);
    let compiled = WellTimeDepthModel1D {
        id: format!("compiled-{}", authored_model.id),
        name: format!("{} (compiled)", authored_model.name),
        wellbore_id: Some(authored_model.wellbore_id.clone()),
        source_kind,
        depth_reference: authored_model.depth_reference,
        travel_time_reference: authored_model.travel_time_reference,
        samples: compiled_samples,
        notes: vec![format!(
            "compiled from authored model '{}' against trajectory '{}'",
            authored_model.id, authored_model.resolved_trajectory_fingerprint
        )],
    };
    validate_well_time_depth_model(&compiled)?;
    Ok(compiled)
}

fn read_well_time_depth_source_samples(
    project: &OphioliteProject,
    authored_model: &WellTimeDepthAuthoredModel1D,
    binding: &ophiolite_seismic::WellTimeDepthSourceBinding,
) -> Result<Vec<ophiolite_seismic::TimeDepthSample1D>> {
    let asset_id = AssetId(binding.asset_id.clone());
    let asset = project.asset_by_id(&asset_id)?;
    match asset.asset_kind {
        AssetKind::CheckshotVspObservationSet => {
            let source = project.read_checkshot_vsp_observation_set(&asset_id)?;
            if source.depth_reference != authored_model.depth_reference
                || source.travel_time_reference != authored_model.travel_time_reference
            {
                return Err(LasError::Validation(format!(
                    "checkshot/VSP source '{}' does not match the authored model depth/time references",
                    asset_id.0
                )));
            }
            Ok(source
                .samples
                .into_iter()
                .map(|sample| ophiolite_seismic::TimeDepthSample1D {
                    depth: sample.depth_m as f32,
                    time_ms: sample.time_ms as f32,
                })
                .collect())
        }
        AssetKind::ManualTimeDepthPickSet => {
            let source = project.read_manual_time_depth_pick_set(&asset_id)?;
            if source.depth_reference != authored_model.depth_reference
                || source.travel_time_reference != authored_model.travel_time_reference
            {
                return Err(LasError::Validation(format!(
                    "manual time-depth pick source '{}' does not match the authored model depth/time references",
                    asset_id.0
                )));
            }
            Ok(source
                .samples
                .into_iter()
                .map(|sample| ophiolite_seismic::TimeDepthSample1D {
                    depth: sample.depth_m as f32,
                    time_ms: sample.time_ms as f32,
                })
                .collect())
        }
        AssetKind::WellTieObservationSet => {
            let source = project.read_well_tie_observation_set(&asset_id)?;
            if source.depth_reference != authored_model.depth_reference
                || source.travel_time_reference != authored_model.travel_time_reference
            {
                return Err(LasError::Validation(format!(
                    "well tie source '{}' does not match the authored model depth/time references",
                    asset_id.0
                )));
            }
            Ok(source
                .samples
                .into_iter()
                .map(|sample| ophiolite_seismic::TimeDepthSample1D {
                    depth: sample.depth_m as f32,
                    time_ms: sample.time_ms as f32,
                })
                .collect())
        }
        AssetKind::WellTimeDepthModel => {
            let source = project.read_well_time_depth_model(&asset_id)?;
            if source.depth_reference != authored_model.depth_reference
                || source.travel_time_reference != authored_model.travel_time_reference
            {
                return Err(LasError::Validation(format!(
                    "well time-depth model source '{}' does not match the authored model depth/time references",
                    asset_id.0
                )));
            }
            Ok(source.samples)
        }
        _ => Err(LasError::Validation(format!(
            "source asset '{}' is not a supported well time-depth source asset",
            asset_id.0
        ))),
    }
}

fn depth_outside_binding_interval(
    depth_m: f64,
    binding: &ophiolite_seismic::WellTimeDepthSourceBinding,
) -> bool {
    binding
        .valid_from_depth_m
        .is_some_and(|min_depth| depth_m < min_depth)
        || binding
            .valid_to_depth_m
            .is_some_and(|max_depth| depth_m > max_depth)
}

fn interpolate_observation_time_ms(
    samples: &[ophiolite_seismic::TimeDepthSample1D],
    depth_m: f64,
) -> Option<f64> {
    let first = samples.first()?;
    let last = samples.last()?;
    let first_depth = f64::from(first.depth);
    let last_depth = f64::from(last.depth);
    if depth_m < first_depth || depth_m > last_depth {
        return None;
    }
    for pair in samples.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        let start_depth = f64::from(start.depth);
        let end_depth = f64::from(end.depth);
        if depth_m < start_depth || depth_m > end_depth {
            continue;
        }
        let span = end_depth - start_depth;
        if span.abs() < f64::EPSILON {
            return Some(f64::from(start.time_ms));
        }
        let fraction = (depth_m - start_depth) / span;
        return Some(
            f64::from(start.time_ms)
                + (f64::from(end.time_ms) - f64::from(start.time_ms)) * fraction,
        );
    }
    Some(f64::from(last.time_ms))
}

fn assumption_time_ms_for_depth(
    assumptions: &[ophiolite_seismic::WellTimeDepthAssumptionInterval],
    depth_m: f64,
    anchor_sample: &ophiolite_seismic::TimeDepthSample1D,
    travel_time_reference: TravelTimeReference,
    has_source_coverage: bool,
) -> Option<f64> {
    assumptions.iter().find_map(|assumption| {
        if !assumption.overwrite_existing_source_coverage && has_source_coverage {
            return None;
        }
        if assumption
            .from_depth_m
            .is_some_and(|min_depth| depth_m < min_depth)
            || assumption
                .to_depth_m
                .is_some_and(|max_depth| depth_m > max_depth)
        {
            return None;
        }
        match assumption.kind {
            ophiolite_seismic::WellTimeDepthAssumptionKind::ConstantVelocity => {
                let velocity = assumption.velocity_m_per_s?;
                if !velocity.is_finite() || velocity <= 0.0 {
                    return None;
                }
                let scale = match travel_time_reference {
                    TravelTimeReference::OneWay => 1000.0,
                    TravelTimeReference::TwoWay => 2000.0,
                };
                let delta_depth = depth_m - f64::from(anchor_sample.depth);
                Some(f64::from(anchor_sample.time_ms) + (delta_depth / velocity) * scale)
            }
        }
    })
}

fn project_depth_reference_from_model(reference: DepthReferenceKind) -> DepthReference {
    match reference {
        DepthReferenceKind::MeasuredDepth => DepthReference::MeasuredDepth,
        DepthReferenceKind::TrueVerticalDepth => DepthReference::TrueVerticalDepth,
        DepthReferenceKind::TrueVerticalDepthSubsea => DepthReference::TrueVerticalDepthSubsea,
    }
}

fn value_origin(value: Option<f64>) -> Option<TrajectoryValueOrigin> {
    value.map(|_| TrajectoryValueOrigin::Imported)
}

fn derived_value_origin(value: Option<f64>, derived: bool) -> Option<TrajectoryValueOrigin> {
    value.map(|_| {
        if derived {
            TrajectoryValueOrigin::Derived
        } else {
            TrajectoryValueOrigin::Imported
        }
    })
}

fn resolve_trajectory_rows(
    rows: &[TrajectoryRow],
    asset_id: &AssetId,
    notes: &mut Vec<String>,
) -> Vec<ResolvedTrajectoryStation> {
    let mut ordered_rows = rows.to_vec();
    ordered_rows.sort_by(|left, right| {
        left.measured_depth
            .partial_cmp(&right.measured_depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if ordered_rows.len() >= 2
        && ordered_rows
            .windows(2)
            .all(|window| window[0].inclination_deg.is_some() && window[0].azimuth_deg.is_some())
        && ordered_rows
            .last()
            .is_some_and(|row| row.inclination_deg.is_some() && row.azimuth_deg.is_some())
    {
        return resolve_trajectory_rows_with_minimum_curvature(&ordered_rows, asset_id, notes);
    }

    ordered_rows
        .into_iter()
        .map(|row| ResolvedTrajectoryStation {
            measured_depth_m: row.measured_depth,
            true_vertical_depth_m: row.true_vertical_depth,
            true_vertical_depth_subsea_m: row.true_vertical_depth_subsea,
            northing_offset_m: row.northing_offset,
            easting_offset_m: row.easting_offset,
            absolute_xy: None,
            inclination_deg: row.inclination_deg,
            azimuth_deg: row.azimuth_deg,
            true_vertical_depth_origin: value_origin(row.true_vertical_depth),
            true_vertical_depth_subsea_origin: value_origin(row.true_vertical_depth_subsea),
            northing_offset_origin: value_origin(row.northing_offset),
            easting_offset_origin: value_origin(row.easting_offset),
            inclination_origin: value_origin(row.inclination_deg),
            azimuth_origin: value_origin(row.azimuth_deg),
        })
        .collect()
}

fn resolve_trajectory_rows_with_minimum_curvature(
    rows: &[TrajectoryRow],
    asset_id: &AssetId,
    notes: &mut Vec<String>,
) -> Vec<ResolvedTrajectoryStation> {
    let derive_offsets = rows
        .iter()
        .any(|row| row.northing_offset.is_none() || row.easting_offset.is_none());
    let derive_tvd = rows.iter().any(|row| row.true_vertical_depth.is_none())
        && rows.iter().any(|row| row.true_vertical_depth.is_some())
        || (rows.iter().all(|row| row.true_vertical_depth.is_none())
            && rows
                .iter()
                .all(|row| row.true_vertical_depth_subsea.is_none()));
    let derive_tvdss = rows
        .iter()
        .any(|row| row.true_vertical_depth_subsea.is_some());

    if derive_offsets {
        notes.push(format!(
            "trajectory asset '{}' used minimum-curvature interpolation to resolve missing lateral offsets from MD/inc/azi stations",
            asset_id.0
        ));
    }
    if derive_tvd {
        notes.push(format!(
            "trajectory asset '{}' used minimum-curvature interpolation to resolve true vertical depth from MD/inc/azi stations",
            asset_id.0
        ));
    }
    if derive_tvdss
        && rows
            .iter()
            .any(|row| row.true_vertical_depth_subsea.is_none())
    {
        notes.push(format!(
            "trajectory asset '{}' used minimum-curvature interpolation to fill missing TVDSS samples from MD/inc/azi stations",
            asset_id.0
        ));
    }

    let first = rows
        .first()
        .expect("minimum-curvature rows require at least one row");
    let mut cumulative_northing = first.northing_offset.unwrap_or(0.0);
    let mut cumulative_easting = first.easting_offset.unwrap_or(0.0);
    let mut cumulative_tvd = first.true_vertical_depth.unwrap_or(0.0);
    let mut cumulative_tvdss = first.true_vertical_depth_subsea.unwrap_or(0.0);

    if derive_offsets && (first.northing_offset.is_none() || first.easting_offset.is_none()) {
        notes.push(format!(
            "trajectory asset '{}' is missing offset values at its first station; minimum-curvature offsets assume a zero-origin there",
            asset_id.0
        ));
    }
    if derive_tvd && first.true_vertical_depth.is_none() {
        notes.push(format!(
            "trajectory asset '{}' is missing TVD at its first station; minimum-curvature TVD assumes a zero-origin there",
            asset_id.0
        ));
    }
    if derive_tvdss && first.true_vertical_depth_subsea.is_none() {
        notes.push(format!(
            "trajectory asset '{}' is missing TVDSS at its first station; minimum-curvature TVDSS assumes a zero-origin there",
            asset_id.0
        ));
    }

    let mut stations = Vec::with_capacity(rows.len());
    stations.push(ResolvedTrajectoryStation {
        measured_depth_m: first.measured_depth,
        true_vertical_depth_m: first
            .true_vertical_depth
            .or(derive_tvd.then_some(cumulative_tvd)),
        true_vertical_depth_subsea_m: first
            .true_vertical_depth_subsea
            .or(derive_tvdss.then_some(cumulative_tvdss)),
        northing_offset_m: first
            .northing_offset
            .or(derive_offsets.then_some(cumulative_northing)),
        easting_offset_m: first
            .easting_offset
            .or(derive_offsets.then_some(cumulative_easting)),
        absolute_xy: None,
        inclination_deg: first.inclination_deg,
        azimuth_deg: first.azimuth_deg,
        true_vertical_depth_origin: derived_value_origin(
            first
                .true_vertical_depth
                .or(derive_tvd.then_some(cumulative_tvd)),
            first.true_vertical_depth.is_none()
                && first
                    .true_vertical_depth
                    .or(derive_tvd.then_some(cumulative_tvd))
                    .is_some(),
        ),
        true_vertical_depth_subsea_origin: derived_value_origin(
            first
                .true_vertical_depth_subsea
                .or(derive_tvdss.then_some(cumulative_tvdss)),
            first.true_vertical_depth_subsea.is_none()
                && first
                    .true_vertical_depth_subsea
                    .or(derive_tvdss.then_some(cumulative_tvdss))
                    .is_some(),
        ),
        northing_offset_origin: derived_value_origin(
            first
                .northing_offset
                .or(derive_offsets.then_some(cumulative_northing)),
            first.northing_offset.is_none()
                && first
                    .northing_offset
                    .or(derive_offsets.then_some(cumulative_northing))
                    .is_some(),
        ),
        easting_offset_origin: derived_value_origin(
            first
                .easting_offset
                .or(derive_offsets.then_some(cumulative_easting)),
            first.easting_offset.is_none()
                && first
                    .easting_offset
                    .or(derive_offsets.then_some(cumulative_easting))
                    .is_some(),
        ),
        inclination_origin: value_origin(first.inclination_deg),
        azimuth_origin: value_origin(first.azimuth_deg),
    });

    for window in rows.windows(2) {
        let start = &window[0];
        let end = &window[1];
        let (northing_delta, easting_delta, tvd_delta) = minimum_curvature_delta(
            start.measured_depth,
            start.inclination_deg.unwrap_or(0.0),
            start.azimuth_deg.unwrap_or(0.0),
            end.measured_depth,
            end.inclination_deg.unwrap_or(0.0),
            end.azimuth_deg.unwrap_or(0.0),
        );
        cumulative_northing += northing_delta;
        cumulative_easting += easting_delta;
        cumulative_tvd += tvd_delta;
        cumulative_tvdss += tvd_delta;

        let true_vertical_depth_m = end
            .true_vertical_depth
            .or(derive_tvd.then_some(cumulative_tvd));
        let true_vertical_depth_subsea_m = end
            .true_vertical_depth_subsea
            .or(derive_tvdss.then_some(cumulative_tvdss));
        let northing_offset_m = end
            .northing_offset
            .or(derive_offsets.then_some(cumulative_northing));
        let easting_offset_m = end
            .easting_offset
            .or(derive_offsets.then_some(cumulative_easting));

        stations.push(ResolvedTrajectoryStation {
            measured_depth_m: end.measured_depth,
            true_vertical_depth_m,
            true_vertical_depth_subsea_m,
            northing_offset_m,
            easting_offset_m,
            absolute_xy: None,
            inclination_deg: end.inclination_deg,
            azimuth_deg: end.azimuth_deg,
            true_vertical_depth_origin: derived_value_origin(
                true_vertical_depth_m,
                end.true_vertical_depth.is_none() && true_vertical_depth_m.is_some(),
            ),
            true_vertical_depth_subsea_origin: derived_value_origin(
                true_vertical_depth_subsea_m,
                end.true_vertical_depth_subsea.is_none() && true_vertical_depth_subsea_m.is_some(),
            ),
            northing_offset_origin: derived_value_origin(
                northing_offset_m,
                end.northing_offset.is_none() && northing_offset_m.is_some(),
            ),
            easting_offset_origin: derived_value_origin(
                easting_offset_m,
                end.easting_offset.is_none() && easting_offset_m.is_some(),
            ),
            inclination_origin: value_origin(end.inclination_deg),
            azimuth_origin: value_origin(end.azimuth_deg),
        });
    }

    stations
}

fn minimum_curvature_delta(
    start_md_m: f64,
    start_inclination_deg: f64,
    start_azimuth_deg: f64,
    end_md_m: f64,
    end_inclination_deg: f64,
    end_azimuth_deg: f64,
) -> (f64, f64, f64) {
    let delta_md = end_md_m - start_md_m;
    if !delta_md.is_finite() || delta_md <= 0.0 {
        return (0.0, 0.0, 0.0);
    }

    let start_inclination_rad = start_inclination_deg.to_radians();
    let end_inclination_rad = end_inclination_deg.to_radians();
    let start_azimuth_rad = start_azimuth_deg.to_radians();
    let end_azimuth_rad = end_azimuth_deg.to_radians();

    let dogleg_argument = (start_inclination_rad.cos() * end_inclination_rad.cos()
        + start_inclination_rad.sin()
            * end_inclination_rad.sin()
            * (end_azimuth_rad - start_azimuth_rad).cos())
    .clamp(-1.0, 1.0);
    let dogleg = dogleg_argument.acos();
    let ratio_factor = if dogleg.abs() < 1.0e-12 {
        1.0
    } else {
        (2.0 / dogleg) * (dogleg / 2.0).tan()
    };

    let northing_delta = (delta_md / 2.0)
        * (start_inclination_rad.sin() * start_azimuth_rad.cos()
            + end_inclination_rad.sin() * end_azimuth_rad.cos())
        * ratio_factor;
    let easting_delta = (delta_md / 2.0)
        * (start_inclination_rad.sin() * start_azimuth_rad.sin()
            + end_inclination_rad.sin() * end_azimuth_rad.sin())
        * ratio_factor;
    let tvd_delta =
        (delta_md / 2.0) * (start_inclination_rad.cos() + end_inclination_rad.cos()) * ratio_factor;

    (northing_delta, easting_delta, tvd_delta)
}

fn validate_metric_length_unit(unit: Option<&str>, axis: &str, asset_id: &AssetId) -> Result<()> {
    let Some(unit) = unit else {
        return Ok(());
    };
    if is_metric_length_unit(unit) {
        return Ok(());
    }
    Err(LasError::Validation(format!(
        "trajectory asset '{}' declares a non-metric {axis} unit '{}'; the resolved trajectory contract currently requires meter-based inputs",
        asset_id.0, unit
    )))
}

fn is_metric_length_unit(unit: &str) -> bool {
    matches!(
        unit.trim().to_ascii_lowercase().as_str(),
        "m" | "meter" | "meters" | "metre" | "metres"
    )
}

fn coordinate_reference_descriptor_from_project(
    reference: Option<&CoordinateReference>,
    unit: Option<&str>,
) -> Option<CoordinateReferenceDescriptor> {
    reference.map(|value| CoordinateReferenceDescriptor {
        id: value.id.clone(),
        name: value.name.clone(),
        geodetic_datum: value.geodetic_datum.clone(),
        unit: unit.map(str::to_string),
    })
}

fn coordinate_reference_descriptors_compatible(
    left: &CoordinateReferenceDescriptor,
    right: &CoordinateReferenceDescriptor,
) -> bool {
    if let (Some(left_id), Some(right_id)) = (left.id.as_ref(), right.id.as_ref()) {
        return left_id.eq_ignore_ascii_case(right_id);
    }
    if let (Some(left_name), Some(right_name)) = (left.name.as_ref(), right.name.as_ref()) {
        return left_name.eq_ignore_ascii_case(right_name);
    }
    left.geodetic_datum == right.geodetic_datum && left.unit == right.unit
}

fn can_resolve_absolute_xy(
    anchor: Option<&ophiolite_seismic::WellboreAnchorReference>,
    asset_coordinate_reference: Option<&CoordinateReferenceDescriptor>,
    asset_id: &AssetId,
    notes: &mut Vec<String>,
) -> bool {
    let Some(anchor_reference) = anchor else {
        return false;
    };

    if let Some(anchor_coordinate_reference) = anchor_reference.coordinate_reference.as_ref() {
        if let Some(anchor_unit) = anchor_coordinate_reference.unit.as_deref() {
            if !is_metric_length_unit(anchor_unit) {
                notes.push(format!(
                    "wellbore anchor for asset '{}' is not expressed in a metric coordinate system, so relative offsets were not promoted to absolute XY",
                    asset_id.0
                ));
                return false;
            }
        }
    } else {
        notes.push(format!(
            "wellbore anchor for asset '{}' has no coordinate reference; absolute XY assumes the anchor location already uses the same projected units as the trajectory offsets",
            asset_id.0
        ));
    }

    match (
        anchor_reference.coordinate_reference.as_ref(),
        asset_coordinate_reference,
    ) {
        (Some(anchor_coordinate_reference), Some(asset_coordinate_reference))
            if !coordinate_reference_descriptors_compatible(
                anchor_coordinate_reference,
                asset_coordinate_reference,
            ) =>
        {
            notes.push(format!(
                "trajectory asset '{}' does not share the wellbore anchor coordinate reference, so absolute XY was left unresolved",
                asset_id.0
            ));
            false
        }
        _ => true,
    }
}

fn coordinate_reference_dto_from_seismic(
    reference: &ophiolite_seismic::CoordinateReferenceDescriptor,
) -> CoordinateReferenceDto {
    CoordinateReferenceDto {
        id: reference.id.clone(),
        name: reference.name.clone(),
        geodetic_datum: reference.geodetic_datum.clone(),
        unit: reference.unit.clone(),
    }
}

fn projected_point_dto_from_seismic(
    point: &ophiolite_seismic::ProjectedPoint2,
) -> ProjectedPoint2Dto {
    ProjectedPoint2Dto {
        x: point.x,
        y: point.y,
    }
}

fn require_asset_kind(asset: &AssetRecord, expected: AssetKind) -> Result<()> {
    if asset.asset_kind != expected {
        return Err(LasError::Validation(format!(
            "asset '{}' is {}, not {}",
            asset.id.0,
            asset.asset_kind.as_str(),
            expected.as_str()
        )));
    }
    Ok(())
}

fn asset_covers_depth_range(asset: &AssetRecord, depth_min: f64, depth_max: f64) -> bool {
    let start = asset.manifest.extents.start;
    let stop = asset.manifest.extents.stop;
    match (start, stop) {
        (Some(start), Some(stop)) => start <= depth_max && stop >= depth_min,
        _ => false,
    }
}

fn unique_id(prefix: &str) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{prefix}_{nanos}_{counter}")
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn normalized_text(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn optional_db_text(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn sqlite_error(error: rusqlite::Error) -> LasError {
    LasError::Storage(format!("project catalog error: {error}"))
}

fn sql_json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn sql_validation_error(error: LasError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::other(error.to_string())),
    )
}

fn source_fingerprint(bytes: &[u8]) -> String {
    let checksum = bytes.iter().fold(0u64, |acc, byte| {
        acc.wrapping_mul(16777619).wrapping_add(u64::from(*byte))
    });
    revision_token_for_bytes("source", &format!("{}:{checksum}", bytes.len())).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{
        AxisSummaryF32, AxisSummaryI32, CoordinateReferenceBinding, CoordinateReferenceDescriptor,
        CoordinateReferenceSource, DatasetId, GeometryDescriptor, GeometryProvenanceSummary,
        GeometrySummary, ProjectedPoint2, ProjectedPolygon2, ProjectedVector2, SampleDataFidelity,
        SurveyGridTransform, SurveySpatialAvailability, SurveySpatialDescriptor, TimeDepthDomain,
        VolumeDescriptor, WellAzimuthReferenceKind, WellboreAnchorKind, WellboreAnchorReference,
    };
    use std::path::PathBuf;

    #[test]
    fn resolve_wellbore_trajectory_projects_offsets_from_anchor() {
        let root = temp_project_root("resolve_wellbore_trajectory_projects_offsets_from_anchor");
        let csv_path = root.join("trajectory.csv");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &csv_path,
            "md,tvd,northing,easting,inclination,azimuth\n0,0,0,0,0,0\n100,90,20,10,12,45\n",
        )
        .unwrap();

        let mut project = OphioliteProject::create(&root).unwrap();
        let binding = AssetBindingInput {
            well_name: "Well A".to_string(),
            wellbore_name: "Well A".to_string(),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        };
        let import = project
            .import_trajectory_csv(&csv_path, &binding, Some("trajectory"))
            .unwrap();
        let geometry = WellboreGeometry {
            anchor: Some(WellboreAnchorReference {
                kind: WellboreAnchorKind::Surface,
                coordinate_reference: Some(CoordinateReferenceDescriptor {
                    id: Some("EPSG:23031".to_string()),
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
                notes: Vec::new(),
            }),
            vertical_datum: Some("KB".to_string()),
            depth_unit: Some("m".to_string()),
            azimuth_reference: WellAzimuthReferenceKind::GridNorth,
            notes: Vec::new(),
        };
        project
            .set_wellbore_geometry(&import.resolution.wellbore_id, Some(geometry))
            .unwrap();

        let resolved = project
            .resolve_wellbore_trajectory(&import.resolution.wellbore_id)
            .unwrap();

        assert_eq!(resolved.wellbore_id, import.resolution.wellbore_id.0);
        assert_eq!(resolved.source_asset_ids.len(), 1);
        assert_eq!(resolved.source_asset_ids[0], import.asset.id.0);
        assert_eq!(resolved.stations.len(), 2);
        assert_eq!(
            resolved.stations[1].absolute_xy,
            Some(ProjectedPoint2 {
                x: 500_010.0,
                y: 6_200_020.0,
            })
        );
        assert_eq!(
            resolved
                .coordinate_reference
                .as_ref()
                .and_then(|value| value.id.as_deref()),
            Some("EPSG:23031")
        );
        assert!(resolved.notes.iter().any(|note| {
            note.contains("does not store a depth unit")
                || note.contains("does not store a coordinate unit")
        }));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_survey_map_well_returns_resolved_plan_geometry() {
        let root = temp_project_root("resolve_survey_map_well_returns_resolved_plan_geometry");
        let csv_path = root.join("trajectory.csv");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &csv_path,
            "md,tvd,northing,easting,inclination,azimuth\n0,0,0,0,0,0\n100,90,20,10,12,45\n",
        )
        .unwrap();

        let mut project = OphioliteProject::create(&root).unwrap();
        let binding = AssetBindingInput {
            well_name: "Well A".to_string(),
            wellbore_name: "Well A".to_string(),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        };
        let import = project
            .import_trajectory_csv(&csv_path, &binding, Some("trajectory"))
            .unwrap();
        let geometry = WellboreGeometry {
            anchor: Some(WellboreAnchorReference {
                kind: WellboreAnchorKind::Surface,
                coordinate_reference: Some(CoordinateReferenceDescriptor {
                    id: Some("EPSG:23031".to_string()),
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
                notes: Vec::new(),
            }),
            vertical_datum: Some("KB".to_string()),
            depth_unit: Some("m".to_string()),
            azimuth_reference: WellAzimuthReferenceKind::GridNorth,
            notes: Vec::new(),
        };
        project
            .set_wellbore_geometry(&import.resolution.wellbore_id, Some(geometry))
            .unwrap();

        let resolved = project
            .resolve_survey_map_well(&import.resolution.wellbore_id, Some("EPSG:23031"))
            .unwrap();

        assert_eq!(
            resolved
                .coordinate_reference
                .as_ref()
                .and_then(|value| value.id.as_deref()),
            Some("EPSG:23031")
        );
        assert_eq!(
            resolved.transform_status,
            SurveyMapTransformStatusDto::DisplayEquivalent
        );
        assert_eq!(
            resolved
                .transform_diagnostics
                .source_coordinate_reference_id
                .as_deref(),
            Some("EPSG:23031")
        );
        assert_eq!(
            resolved
                .transform_diagnostics
                .target_coordinate_reference_id
                .as_deref(),
            Some("EPSG:23031")
        );
        assert_eq!(
            resolved.surface_location,
            Some(ProjectedPoint2Dto {
                x: 500_000.0,
                y: 6_200_000.0,
            })
        );
        assert_eq!(resolved.plan_trajectory.len(), 2);
        assert_eq!(
            resolved.plan_trajectory[1],
            ProjectedPoint2Dto {
                x: 500_010.0,
                y: 6_200_020.0,
            }
        );
        assert_eq!(resolved.trajectories.len(), 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_survey_map_source_rejects_blank_project_display_crs() {
        let root = temp_project_root("resolve_survey_map_source_rejects_blank_project_display_crs");
        fs::create_dir_all(&root).unwrap();
        let project = OphioliteProject::create(&root).unwrap();

        let error = project
            .resolve_survey_map_source(&ProjectSurveyMapRequestDto {
                schema_version: 1,
                survey_asset_ids: vec![String::from("asset-1")],
                wellbore_ids: Vec::new(),
                display_coordinate_reference_id: String::from("   "),
            })
            .unwrap_err();

        assert!(error.to_string().contains(
            "project survey-map requests require a non-empty display coordinate reference id"
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_well_station_onto_inline_section_uses_inverse_grid_transform() {
        let grid_transform = SurveyMapGridTransformDto {
            origin: ProjectedPoint2Dto {
                x: 1000.0,
                y: 2000.0,
            },
            inline_basis: ProjectedVector2Dto { x: 0.0, y: 25.0 },
            xline_basis: ProjectedVector2Dto { x: 25.0, y: 0.0 },
        };
        let section_axis = section_axis_spec(
            &SurveyIndexGridDto {
                inline_axis: SurveyIndexAxisDto {
                    count: 10,
                    first: 1000,
                    last: 1009,
                    step: Some(1),
                    regular: true,
                },
                xline_axis: SurveyIndexAxisDto {
                    count: 20,
                    first: 2000,
                    last: 2019,
                    step: Some(1),
                    regular: true,
                },
            },
            SectionAxis::Inline,
            1002,
        )
        .unwrap();
        let station = ResolvedTrajectoryStation {
            measured_depth_m: 1500.0,
            true_vertical_depth_m: Some(1400.0),
            true_vertical_depth_subsea_m: None,
            northing_offset_m: Some(50.0),
            easting_offset_m: Some(75.0),
            absolute_xy: Some(ProjectedPoint2 {
                x: 1075.0,
                y: 2050.0,
            }),
            inclination_deg: None,
            azimuth_deg: None,
            true_vertical_depth_origin: Some(TrajectoryValueOrigin::Imported),
            true_vertical_depth_subsea_origin: None,
            northing_offset_origin: Some(TrajectoryValueOrigin::Imported),
            easting_offset_origin: Some(TrajectoryValueOrigin::Imported),
            inclination_origin: None,
            azimuth_origin: None,
        };

        let projected = project_well_station_onto_section(
            &station,
            station.absolute_xy.as_ref().unwrap(),
            &grid_transform,
            &section_axis,
            12.5,
        )
        .unwrap();

        assert_eq!(projected.trace_index, 3);
        assert_eq!(projected.trace_coordinate, 2003.0);
        assert_eq!(projected.sample_value, Some(1400.0));
    }

    #[test]
    fn densify_trajectory_for_section_inserts_intermediate_station() {
        let stations = vec![
            ResolvedTrajectoryStation {
                measured_depth_m: 0.0,
                true_vertical_depth_m: Some(0.0),
                true_vertical_depth_subsea_m: None,
                northing_offset_m: Some(0.0),
                easting_offset_m: Some(0.0),
                absolute_xy: Some(ProjectedPoint2 { x: 0.0, y: 0.0 }),
                inclination_deg: None,
                azimuth_deg: None,
                true_vertical_depth_origin: Some(TrajectoryValueOrigin::Imported),
                true_vertical_depth_subsea_origin: None,
                northing_offset_origin: Some(TrajectoryValueOrigin::Imported),
                easting_offset_origin: Some(TrajectoryValueOrigin::Imported),
                inclination_origin: None,
                azimuth_origin: None,
            },
            ResolvedTrajectoryStation {
                measured_depth_m: 20.0,
                true_vertical_depth_m: Some(10.0),
                true_vertical_depth_subsea_m: None,
                northing_offset_m: Some(0.0),
                easting_offset_m: Some(20.0),
                absolute_xy: Some(ProjectedPoint2 { x: 20.0, y: 0.0 }),
                inclination_deg: None,
                azimuth_deg: None,
                true_vertical_depth_origin: Some(TrajectoryValueOrigin::Imported),
                true_vertical_depth_subsea_origin: None,
                northing_offset_origin: Some(TrajectoryValueOrigin::Imported),
                easting_offset_origin: Some(TrajectoryValueOrigin::Imported),
                inclination_origin: None,
                azimuth_origin: None,
            },
        ];

        let densified = densify_trajectory_for_section(
            &stations,
            SectionTrajectoryDensificationSettings {
                max_md_step_m: 10.0,
                max_xy_step_m: 10.0,
                max_vertical_step_m: 10.0,
            },
        );

        assert_eq!(densified.len(), 3);
        assert_eq!(densified[1].measured_depth_m, 10.0);
        assert_eq!(densified[1].true_vertical_depth_m, Some(5.0));
        assert_eq!(
            densified[1].absolute_xy,
            Some(ProjectedPoint2 { x: 10.0, y: 0.0 })
        );
        assert_eq!(
            densified[1].true_vertical_depth_origin,
            Some(TrajectoryValueOrigin::Derived)
        );
        assert_eq!(
            densified[1].easting_offset_origin,
            Some(TrajectoryValueOrigin::Derived)
        );
    }

    #[test]
    fn interpolate_well_time_depth_model_ms_linearly_interpolates() {
        let model = WellTimeDepthModel1D {
            id: "model-1".to_string(),
            name: "Model 1".to_string(),
            wellbore_id: Some("wb-1".to_string()),
            source_kind: ophiolite_seismic::TimeDepthTransformSourceKind::CheckshotModel1D,
            depth_reference: DepthReferenceKind::TrueVerticalDepth,
            travel_time_reference: TravelTimeReference::OneWay,
            samples: vec![
                ophiolite_seismic::TimeDepthSample1D {
                    depth: 1000.0,
                    time_ms: 800.0,
                },
                ophiolite_seismic::TimeDepthSample1D {
                    depth: 1200.0,
                    time_ms: 1000.0,
                },
            ],
            notes: Vec::new(),
        };

        assert_eq!(interpolate_well_time_depth_model_ms(&model, 900.0), None);
        assert_eq!(
            interpolate_well_time_depth_model_ms(&model, 1100.0),
            Some(900.0)
        );
        assert_eq!(
            display_time_ms(
                900.0,
                TravelTimeReference::OneWay,
                TravelTimeReference::TwoWay
            ),
            1800.0
        );
    }

    #[test]
    fn import_well_time_depth_model_json_round_trips() {
        let root = temp_project_root("import_well_time_depth_model_json_round_trips");
        fs::create_dir_all(&root).unwrap();
        let model_path = root.join("well-model.json");
        fs::write(
            &model_path,
            serde_json::to_vec_pretty(&WellTimeDepthModel1D {
                id: "model-1".to_string(),
                name: "Well Model".to_string(),
                wellbore_id: Some("Well A".to_string()),
                source_kind: ophiolite_seismic::TimeDepthTransformSourceKind::CheckshotModel1D,
                depth_reference: DepthReferenceKind::TrueVerticalDepth,
                travel_time_reference: TravelTimeReference::TwoWay,
                samples: vec![
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 0.0,
                        time_ms: 0.0,
                    },
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1000.0,
                        time_ms: 1200.0,
                    },
                ],
                notes: vec!["test".to_string()],
            })
            .unwrap(),
        )
        .unwrap();

        let mut project = OphioliteProject::create(&root).unwrap();
        let result = project
            .import_well_time_depth_model_json(
                &model_path,
                AssetBindingInput {
                    well_name: "Well A".to_string(),
                    wellbore_name: "Well A".to_string(),
                    uwi: None,
                    api: None,
                    operator_aliases: Vec::new(),
                },
                Some("well model"),
            )
            .unwrap();

        let round_trip = project
            .read_well_time_depth_model(&result.asset.id)
            .unwrap();
        assert_eq!(result.asset.asset_kind, AssetKind::WellTimeDepthModel);
        assert_eq!(result.resolution.status, AssetStatus::Bound);
        assert_eq!(round_trip.name, "Well Model");
        assert_eq!(round_trip.samples.len(), 2);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn preview_well_time_depth_import_draft_preserves_partial_payload() {
        let root =
            temp_project_root("preview_well_time_depth_import_draft_preserves_partial_payload");
        fs::create_dir_all(&root).unwrap();
        let model_path = root.join("partial-well-model.json");
        fs::write(
            &model_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "id": "partial-model",
                "depth_reference": "true_vertical_depth",
                "travel_time_reference": "two_way",
                "samples": [
                    { "depth": 0.0, "time_ms": 0.0 }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let preview = preview_well_time_depth_import_draft(
            &model_path,
            None,
            AssetKind::WellTimeDepthModel,
            Some("draft collection"),
        )
        .unwrap();

        assert_eq!(preview.suggested_draft.asset_kind, "well_time_depth_model");
        assert_eq!(
            preview.suggested_draft.collection_name.as_deref(),
            Some("draft collection")
        );
        assert!(
            preview
                .suggested_draft
                .json_payload
                .contains("\"partial-model\"")
        );
        assert!(!preview.parsed.can_import);
        assert!(
            preview
                .parsed
                .issues
                .iter()
                .any(|issue| issue.code == "missing_name")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn analyze_well_tie_from_model_builds_log_derived_payload() {
        let root = temp_project_root("analyze_well_tie_from_model_builds_log_derived_payload");
        fs::create_dir_all(&root).unwrap();
        let las_path = root.join("well-logs.las");
        let model_path = root.join("well-model.json");

        fs::write(
            &las_path,
            r#"~Version Information
 VERS.                  2.0 : CWLS LOG ASCII STANDARD - VERSION 2.0
 WRAP.                   NO : ONE LINE PER DEPTH STEP
~Well Information
 STRT.M              1000.0 :
 STOP.M              1100.0 :
 STEP.M                10.0 :
 NULL.             -999.2500 :
 WELL.               Well A :
 UWI.           TEST-UWI-01 :
~Curve Information
 DEPT.M                     : Depth
 DT  .US/M                  : Sonic Transit Time
 RHOB.K/M3                  : Bulk Density
~A
1000.0 420.0 2300.0
1010.0 418.0 2310.0
1020.0 416.0 2320.0
1030.0 414.0 2335.0
1040.0 412.0 2350.0
1050.0 410.0 2365.0
1060.0 408.0 2380.0
1070.0 406.0 2395.0
1080.0 404.0 2410.0
1090.0 402.0 2425.0
1100.0 400.0 2440.0
"#,
        )
        .unwrap();
        fs::write(
            &model_path,
            serde_json::to_vec_pretty(&WellTimeDepthModel1D {
                id: "model-1".to_string(),
                name: "Well Model".to_string(),
                wellbore_id: Some("Well A".to_string()),
                source_kind: ophiolite_seismic::TimeDepthTransformSourceKind::CheckshotModel1D,
                depth_reference: DepthReferenceKind::TrueVerticalDepth,
                travel_time_reference: TravelTimeReference::TwoWay,
                samples: vec![
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1000.0,
                        time_ms: 1000.0,
                    },
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1020.0,
                        time_ms: 1040.0,
                    },
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1040.0,
                        time_ms: 1080.0,
                    },
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1060.0,
                        time_ms: 1120.0,
                    },
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1080.0,
                        time_ms: 1160.0,
                    },
                    ophiolite_seismic::TimeDepthSample1D {
                        depth: 1100.0,
                        time_ms: 1200.0,
                    },
                ],
                notes: vec!["test".to_string()],
            })
            .unwrap(),
        )
        .unwrap();

        let mut project = OphioliteProject::create(&root).unwrap();
        project.import_las(&las_path, Some("logs")).unwrap();
        let import = project
            .import_well_time_depth_model_json(
                &model_path,
                AssetBindingInput {
                    well_name: "Well A".to_string(),
                    wellbore_name: "Well A".to_string(),
                    uwi: Some("TEST-UWI-01".to_string()),
                    api: None,
                    operator_aliases: Vec::new(),
                },
                Some("well model"),
            )
            .unwrap();

        let analysis = project
            .analyze_well_tie_from_model(&import.asset.id, "Test Tie", 1000.0, 1200.0, 200.0)
            .unwrap();

        assert_eq!(
            analysis.log_selection.velocity_source_kind,
            WellTieVelocitySourceKind::SonicCurveConvertedToVp
        );
        assert_eq!(
            analysis.synthetic_trace.amplitudes.len(),
            analysis.acoustic_impedance_curve.values.len()
        );
        assert_eq!(
            analysis
                .draft_observation_set
                .trace_search_radius_m
                .map(f64::from),
            Some(200.0)
        );
        assert_eq!(
            analysis.section_window.trace_count * analysis.section_window.sample_count,
            analysis.section_window.amplitudes.len()
        );
        assert!(
            analysis
                .notes
                .iter()
                .any(|note| note.contains("computed from density"))
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_compute_dispatches_to_python_operator_package() {
        let root = temp_project_root("run_compute_dispatches_to_python_operator_package");
        let package_root = root.join("python-operator");
        fs::create_dir_all(&package_root).unwrap();

        let manifest_path = package_root.join("operator-package.json");
        let entrypoint_path = package_root.join("acme_ops.py");
        fs::write(
            &entrypoint_path,
            r#"from ophiolite_sdk.operators import OperatorRegistry, computed_curve
import sys

registry = OperatorRegistry()

@registry.register("acme:double")
def double_curve(request):
    if sys.prefix == sys.base_prefix:
        raise RuntimeError("python operator did not run inside a virtual environment")
    curve = request.payload["inputs"]["input_curve"]
    factor = float(request.parameters.get("factor", 2.0))
    mnemonic = request.payload.get("output_mnemonic") or "DTX"
    values = [None if value is None else value * factor for value in curve["values"]]
    return computed_curve(
        mnemonic,
        values,
        original_mnemonic=mnemonic,
        unit=curve.get("unit"),
        description="External double",
    )
"#,
        )
        .unwrap();
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "package_name": "acme-tools",
                "package_version": "0.1.0",
                "provider": "acme",
                "runtime": "python",
                "compatibility": { "ophiolite_api": env!("CARGO_PKG_VERSION") },
                "entrypoint": "acme_ops.py",
                "operators": [
                    {
                        "id": "acme:double",
                        "provider": "acme",
                        "name": "Double Curve",
                        "asset_family": "Log",
                        "category": "Demo",
                        "description": "Doubles a selected curve.",
                        "default_output_mnemonic": "DTX",
                        "output_curve_type": "Computed",
                        "input_specs": [
                            {
                                "SingleCurve": {
                                    "parameter_name": "input_curve",
                                    "allowed_types": []
                                }
                            }
                        ],
                        "parameters": [
                            {
                                "Number": {
                                    "name": "factor",
                                    "label": "Factor",
                                    "description": "Scale factor",
                                    "default": 2.0,
                                    "min": 0.0,
                                    "max": 10.0,
                                    "unit": null
                                }
                            }
                        ],
                        "output_lifecycle": "derived_asset",
                        "deterministic": true,
                        "stability": "preview",
                        "tags": ["demo"]
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let las_path = workspace_root_path()
            .unwrap()
            .join("examples")
            .join("sample.las");

        let mut project = OphioliteProject::create(&root).unwrap();
        let import = project.import_las(&las_path, Some("logs")).unwrap();
        let install = project.install_operator_package(&manifest_path).unwrap();
        let installed_manifest =
            load_operator_package_manifest(&install.installed_manifest_path).unwrap();
        assert_eq!(
            installed_manifest.entrypoint.as_deref(),
            Some("acme_ops.py")
        );
        assert!(install.python_environment_path.is_some());
        assert!(PathBuf::from(install.python_environment_path.clone().unwrap()).exists());
        assert!(
            PathBuf::from(&install.installed_manifest_path)
                .with_file_name("acme_ops.py")
                .exists()
        );

        let catalog = project.list_compute_catalog(&import.asset.id).unwrap();
        let operator = catalog
            .functions
            .iter()
            .find(|entry| entry.metadata.id == "acme:double")
            .unwrap();
        assert!(matches!(
            operator.availability,
            ophiolite_compute::ComputeAvailability::Available
        ));

        let result = project
            .run_compute(&ProjectComputeRunRequest {
                source_asset_id: import.asset.id.clone(),
                function_id: "acme:double".to_string(),
                curve_bindings: BTreeMap::from([("input_curve".to_string(), "DT".to_string())]),
                parameters: BTreeMap::from([(
                    "factor".to_string(),
                    ComputeParameterValue::Number(2.0),
                )]),
                output_collection_name: Some("Derived External".to_string()),
                output_mnemonic: Some("DTX".to_string()),
            })
            .unwrap();

        assert_eq!(result.execution.function_id, "acme:double");
        assert_eq!(
            result.execution.operator_package.as_deref(),
            Some("acme-tools")
        );
        assert_eq!(
            result.execution.operator_runtime,
            Some(OperatorRuntimeKind::Python)
        );
        assert_eq!(result.execution.output_curve_name, "DTX");

        let derived_package = open_package(&result.asset.package_path).unwrap();
        let derived_curve = derived_package.file().curve("DTX").unwrap();
        let derived_values = derived_curve.numeric_data().unwrap();
        assert_eq!(derived_values.first().copied(), Some(246.9));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_raw_source_bundle_into_project_archive_registers_project_owner() {
        let root = temp_project_root(
            "import_raw_source_bundle_into_project_archive_registers_project_owner",
        );
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("fault.txt");
        fs::write(&source_path, "fault export").unwrap();

        let mut project = OphioliteProject::create(&root).unwrap();
        let result = project
            .import_raw_source_bundle_into_project_archive(
                &[source_path.as_path()],
                Some("Fault Export"),
            )
            .unwrap();
        let owner = project
            .collection_owner_binding(&result.collection.id)
            .unwrap();

        assert_eq!(owner.scope, AssetOwnerScope::Project);
        assert_eq!(owner.owner_id, PROJECT_ARCHIVE_OWNER_ID);
        assert_eq!(owner.display_name, PROJECT_ARCHIVE_OWNER_NAME);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_well_overlay_inventory_reports_survey_owned_seismic_assets() {
        let root =
            temp_project_root("project_well_overlay_inventory_reports_survey_owned_seismic_assets");
        fs::create_dir_all(&root).unwrap();

        let project = OphioliteProject::create(&root).unwrap();
        let binding = AssetBindingInput {
            well_name: "Survey Anchor".to_string(),
            wellbore_name: "Survey Anchor".to_string(),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        };
        let resolution = project.ensure_well_binding(&binding).unwrap();
        let collection = project
            .resolve_or_create_collection(
                &resolution.wellbore_id,
                AssetKind::SeismicTraceData,
                "F3 Survey",
            )
            .unwrap();
        project
            .bind_collection_owner(
                &collection.id,
                &AssetOwnerBinding {
                    scope: AssetOwnerScope::Survey,
                    owner_id: collection.logical_asset_id.0.clone(),
                    display_name: "F3 Survey".to_string(),
                },
            )
            .unwrap();

        let metadata = test_seismic_asset_metadata("F3 Survey");
        let storage_asset_id = AssetId(unique_id("asset"));
        let package_rel_path = PathBuf::from("assets")
            .join(AssetKind::SeismicTraceData.asset_dir_name())
            .join(format!("{}.ophiolite-asset", storage_asset_id.0));
        let package_root = root.join(&package_rel_path);
        fs::create_dir_all(&package_root).unwrap();
        fs::write(
            package_root.join("metadata.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
        let manifest = seismic_asset_manifest(
            &package_root,
            &metadata,
            &resolution.well_id,
            &resolution.wellbore_id,
            &collection.id,
            &collection.logical_asset_id,
            &storage_asset_id,
            AssetKind::SeismicTraceData,
            WellIdentifierSet {
                primary_name: Some("Survey Anchor".to_string()),
                uwi: None,
                api: None,
                operator_aliases: Vec::new(),
            },
            None,
        )
        .unwrap();
        let asset = AssetRecord {
            id: storage_asset_id,
            logical_asset_id: collection.logical_asset_id.clone(),
            collection_id: collection.id.clone(),
            well_id: resolution.well_id.clone(),
            wellbore_id: resolution.wellbore_id.clone(),
            asset_kind: AssetKind::SeismicTraceData,
            status: AssetStatus::Bound,
            package_path: package_root.to_string_lossy().into_owned(),
            manifest,
        };
        project.insert_asset(&asset, &package_rel_path).unwrap();

        let inventory = project.project_well_overlay_inventory().unwrap();
        assert_eq!(inventory.surveys.len(), 1);
        assert_eq!(inventory.surveys[0].name, "F3 Survey");
        assert_eq!(inventory.surveys[0].owner_scope, AssetOwnerScope::Survey);
        assert_eq!(inventory.surveys[0].owner_id, collection.logical_asset_id.0);
        assert_eq!(inventory.surveys[0].owner_name, "F3 Survey");
        assert_eq!(inventory.surveys[0].well_name, "Survey Anchor");
        assert_eq!(inventory.surveys[0].wellbore_name, "Survey Anchor");
        assert_eq!(
            inventory.surveys[0]
                .effective_coordinate_reference_id
                .as_deref(),
            Some("EPSG:23031")
        );

        let _ = fs::remove_dir_all(&root);
    }

    fn test_seismic_asset_metadata(label: &str) -> SeismicAssetMetadata {
        let coordinate_reference = CoordinateReferenceDescriptor {
            id: Some("EPSG:23031".to_string()),
            name: Some("ED50 / UTM zone 31N".to_string()),
            geodetic_datum: Some("ED50".to_string()),
            unit: Some("m".to_string()),
        };
        let coordinate_reference_binding = CoordinateReferenceBinding {
            detected: Some(coordinate_reference.clone()),
            effective: Some(coordinate_reference.clone()),
            source: CoordinateReferenceSource::ImportManifest,
            notes: Vec::new(),
        };
        let descriptor = VolumeDescriptor {
            id: DatasetId(format!("dataset:{label}")),
            store_id: format!("store:{label}"),
            label: label.to_string(),
            shape: [2, 2, 2],
            chunk_shape: [2, 2, 2],
            sample_interval_ms: 4.0,
            sample_data_fidelity: SampleDataFidelity::default(),
            geometry: GeometryDescriptor {
                compare_family: "poststack_3d".to_string(),
                fingerprint: format!("fingerprint:{label}"),
                summary: GeometrySummary {
                    inline_axis: AxisSummaryI32 {
                        count: 2,
                        first: 100,
                        last: 101,
                        step: Some(1),
                        regular: true,
                    },
                    xline_axis: AxisSummaryI32 {
                        count: 2,
                        first: 200,
                        last: 201,
                        step: Some(1),
                        regular: true,
                    },
                    sample_axis: AxisSummaryF32 {
                        count: 2,
                        first: 0.0,
                        last: 4.0,
                        step: Some(4.0),
                        regular: true,
                        units: Some("ms".to_string()),
                    },
                    layout: Some(ophiolite_seismic::SeismicLayout::PostStack3D),
                    gather_axis_kind: None,
                    gather_axis: None,
                    provenance: GeometryProvenanceSummary::Source,
                },
            },
            coordinate_reference_binding: Some(coordinate_reference_binding.clone()),
            spatial: Some(SurveySpatialDescriptor {
                coordinate_reference: Some(coordinate_reference.clone()),
                grid_transform: Some(SurveyGridTransform {
                    origin: ProjectedPoint2 { x: 0.0, y: 0.0 },
                    inline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                    xline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                }),
                footprint: Some(ProjectedPolygon2 {
                    exterior: vec![
                        ProjectedPoint2 { x: 0.0, y: 0.0 },
                        ProjectedPoint2 { x: 50.0, y: 0.0 },
                        ProjectedPoint2 { x: 50.0, y: 50.0 },
                        ProjectedPoint2 { x: 0.0, y: 50.0 },
                    ],
                }),
                availability: SurveySpatialAvailability::Available,
                notes: Vec::new(),
            }),
            processing_lineage_summary: None,
        };
        let volume = ophiolite_seismic_runtime::VolumeMetadata {
            kind: ophiolite_seismic_runtime::DatasetKind::Source,
            store_id: descriptor.store_id.clone(),
            source: ophiolite_seismic_runtime::SourceIdentity {
                source_path: PathBuf::from(format!("/tmp/{label}.segy")),
                file_size: 1,
                trace_count: 4,
                samples_per_trace: 2,
                sample_interval_us: 4000,
                sample_format_code: 5,
                sample_data_fidelity: SampleDataFidelity::default(),
                endianness: "little".to_string(),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
                geometry: ophiolite_seismic_runtime::GeometryProvenance {
                    inline_field: ophiolite_seismic_runtime::HeaderFieldSpec {
                        name: "INLINE_3D".to_string(),
                        start_byte: 189,
                        value_type: "i32".to_string(),
                    },
                    crossline_field: ophiolite_seismic_runtime::HeaderFieldSpec {
                        name: "CROSSLINE_3D".to_string(),
                        start_byte: 193,
                        value_type: "i32".to_string(),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape: [2, 2, 2],
            axes: ophiolite_seismic_runtime::VolumeAxes::with_vertical_axis(
                vec![100.0, 101.0],
                vec![200.0, 201.0],
                TimeDepthDomain::Time,
                "ms".to_string(),
                vec![0.0, 4.0],
            ),
            segy_export: None,
            coordinate_reference_binding: Some(coordinate_reference_binding),
            spatial: descriptor.spatial.clone(),
            created_by: "test".to_string(),
            processing_lineage: None,
        };
        SeismicAssetMetadata {
            family: SeismicAssetFamily::Volume,
            trace_data_descriptor: SeismicTraceDataDescriptor::from(&descriptor),
            descriptor,
            store: TbvolManifest::new(volume, [2, 2, 2], false),
        }
    }

    fn temp_project_root(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ophiolite-{}-{}", test_name, now_unix_nanos()))
    }
}
