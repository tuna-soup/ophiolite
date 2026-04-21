use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

use super::default_pipeline_schema_version;
use super::domain::{DatasetId, SampleDataFidelity, SectionAxis, VolumeDescriptor};
use super::models::{TimeDepthDomain, WellTieObservationSet1D};
use super::processing::{
    GatherProcessingPipeline, ProcessingJobStatus, SubvolumeProcessingPipeline,
    TraceLocalProcessingPipeline, TraceLocalProcessingPreset,
};
use super::views::{
    GatherPreviewView, ImportedHorizonDescriptor, PreviewView, SectionHorizonOverlayView,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionRequest {
    pub dataset_id: DatasetId,
    pub axis: SectionAxis,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionTileRequest {
    pub section: SectionRequest,
    pub trace_range: [usize; 2],
    pub sample_range: [usize; 2],
    pub lod: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SectionSpectrumSelection {
    WholeSection,
    TraceRange {
        trace_start: usize,
        trace_end: usize,
    },
    RectWindow {
        trace_start: usize,
        trace_end: usize,
        sample_start: usize,
        sample_end: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AmplitudeSpectrumCurve {
    pub frequencies_hz: Vec<f32>,
    pub amplitudes: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AmplitudeSpectrumRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub section: SectionRequest,
    pub selection: SectionSpectrumSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<TraceLocalProcessingPipeline>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AmplitudeSpectrumResponse {
    pub schema_version: u32,
    pub section: SectionRequest,
    pub selection: SectionSpectrumSelection,
    pub sample_interval_ms: f32,
    pub curve: AmplitudeSpectrumCurve,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum WellTieVelocitySourceKind {
    PVelocityCurve,
    SonicCurveConvertedToVp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieLogCurveSource {
    pub asset_id: String,
    pub asset_name: String,
    pub curve_name: String,
    pub original_mnemonic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieLogSelection1D {
    pub density_curve: WellTieLogCurveSource,
    pub velocity_curve: WellTieLogCurveSource,
    pub velocity_source_kind: WellTieVelocitySourceKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieCurve1D {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub times_ms: Vec<f32>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieTrace1D {
    pub id: String,
    pub label: String,
    pub times_ms: Vec<f32>,
    pub amplitudes: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieWavelet {
    pub id: String,
    pub label: String,
    pub times_ms: Vec<f32>,
    pub amplitudes: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieSectionWindow {
    pub id: String,
    pub label: String,
    pub times_ms: Vec<f32>,
    pub trace_offsets_m: Vec<f32>,
    pub amplitudes: Vec<f32>,
    pub trace_count: usize,
    pub sample_count: usize,
    pub well_trace_index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct WellTieAnalysis1D {
    pub draft_observation_set: WellTieObservationSet1D,
    pub log_selection: WellTieLogSelection1D,
    pub acoustic_impedance_curve: WellTieCurve1D,
    pub reflectivity_trace: WellTieTrace1D,
    pub synthetic_trace: WellTieTrace1D,
    pub best_match_trace: WellTieTrace1D,
    pub well_trace: WellTieTrace1D,
    pub section_window: WellTieSectionWindow,
    pub wavelet: WellTieWavelet,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AvoReflectivityMethod {
    ShueyTwoTerm,
    ShueyThreeTerm,
    AkiRichards,
    AkiRichardsAlt,
    Fatti,
    Bortfeld,
    Hilterman,
    ApproxZoeppritzPp,
    ZoeppritzPp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoReflectivityRequest {
    #[serde(default = "default_pipeline_schema_version")]
    pub schema_version: u32,
    pub method: AvoReflectivityMethod,
    pub sample_shape: Vec<usize>,
    pub angles_deg: Vec<f32>,
    pub upper_vp_m_per_s: Vec<f32>,
    pub upper_vs_m_per_s: Vec<f32>,
    pub upper_density_g_cc: Vec<f32>,
    pub lower_vp_m_per_s: Vec<f32>,
    pub lower_vs_m_per_s: Vec<f32>,
    pub lower_density_g_cc: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoReflectivityResponse {
    pub schema_version: u32,
    pub method: AvoReflectivityMethod,
    pub sample_shape: Vec<usize>,
    pub angles_deg: Vec<f32>,
    pub pp_real_f32le: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pp_imag_f32le: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercept_f32le: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_f32le: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RockPhysicsAttributeMethod {
    AcousticImpedance,
    ShearImpedance,
    LambdaRho,
    MuRho,
    VpVsRatio,
    PoissonsRatio,
    ElasticImpedance,
    ExtendedElasticImpedance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsAttributeRequest {
    #[serde(default = "default_pipeline_schema_version")]
    pub schema_version: u32,
    pub method: RockPhysicsAttributeMethod,
    pub sample_shape: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vp_m_per_s: Option<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vs_m_per_s: Option<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density_g_cc: Option<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incident_angle_deg: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chi_angle_deg: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RockPhysicsAttributeResponse {
    pub schema_version: u32,
    pub method: RockPhysicsAttributeMethod,
    pub sample_shape: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub values_f32le: Vec<u8>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub semantic_parameters: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AvoInterceptGradientAttributeMethod {
    ChiProjection,
    FluidFactor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoInterceptGradientAttributeRequest {
    #[serde(default = "default_pipeline_schema_version")]
    pub schema_version: u32,
    pub method: AvoInterceptGradientAttributeMethod,
    pub sample_shape: Vec<usize>,
    pub intercept: Vec<f32>,
    pub gradient: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chi_angle_deg: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercept_scalar: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AvoInterceptGradientAttributeResponse {
    pub schema_version: u32,
    pub method: AvoInterceptGradientAttributeMethod,
    pub sample_shape: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub values_f32le: Vec<u8>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub semantic_parameters: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GatherSelector {
    InlineXline { inline: i32, xline: i32 },
    Coordinate { coordinate: f64 },
    Ordinal { index: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GatherRequest {
    pub dataset_id: DatasetId,
    pub selector: GatherSelector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SuggestedImportAction {
    DirectDenseIngest,
    RegularizeSparseSurvey,
    ReviewGeometryMapping,
    UnsupportedInV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DatasetSummary {
    pub store_path: String,
    pub descriptor: VolumeDescriptor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyHeaderValueType {
    I16,
    I32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyHeaderField {
    pub start_byte: u16,
    pub value_type: SegyHeaderValueType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyGeometryOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_3d: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossline_3d: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub third_axis: Option<SegyHeaderField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyGeometryCandidate {
    pub label: String,
    pub geometry: SegyGeometryOverride,
    pub classification: String,
    pub stacking_state: String,
    pub organization: String,
    pub layout: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_axis_kind: Option<String>,
    pub suggested_action: SuggestedImportAction,
    pub inline_count: usize,
    pub crossline_count: usize,
    pub third_axis_count: usize,
    pub observed_trace_count: usize,
    pub expected_trace_count: usize,
    pub completeness_ratio: f64,
    pub auto_selectable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyPreflightRequest {
    pub schema_version: u32,
    pub input_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_override: Option<SegyGeometryOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SurveyPreflightResponse {
    pub schema_version: u32,
    pub input_path: String,
    pub trace_count: u64,
    pub samples_per_trace: usize,
    pub sample_data_fidelity: SampleDataFidelity,
    pub classification: String,
    pub stacking_state: String,
    pub organization: String,
    pub layout: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_axis_kind: Option<String>,
    pub suggested_action: SuggestedImportAction,
    pub observed_trace_count: usize,
    pub expected_trace_count: usize,
    pub completeness_ratio: f64,
    pub resolved_geometry: SegyGeometryOverride,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_geometry_override: Option<SegyGeometryOverride>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometry_candidates: Vec<SegyGeometryCandidate>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyImportWizardStage {
    Scan,
    Structure,
    Spatial,
    Review,
    Import,
    RawInspect,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyImportIssueSeverity {
    Blocking,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyImportIssueSection {
    Scan,
    Structure,
    Spatial,
    Review,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyImportSparseHandling {
    BlockImport,
    RegularizeToDense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyImportPlanSource {
    ScanDefault,
    Candidate,
    SavedRecipe,
    SourceMemory,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SegyImportRecipeScope {
    Global,
    SourceFingerprint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportPolicy {
    pub sparse_handling: SegyImportSparseHandling,
    pub output_store_path: String,
    #[serde(default)]
    pub overwrite_existing: bool,
    #[serde(default)]
    pub acknowledge_warnings: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportSpatialPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_field: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_field: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_scalar_field: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_units: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportProvenance {
    pub plan_source: SegyImportPlanSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportPlan {
    pub input_path: String,
    pub source_fingerprint: String,
    pub header_mapping: SegyGeometryOverride,
    pub spatial: SegyImportSpatialPlan,
    pub policy: SegyImportPolicy,
    pub provenance: SegyImportProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportIssue {
    pub severity: SegyImportIssueSeverity,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
    pub section: SegyImportIssueSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportRiskSummary {
    pub observed_trace_count: u64,
    pub expected_trace_count: u64,
    pub completeness_ratio: f64,
    pub blowup_ratio: f64,
    pub estimated_amplitude_bytes: u64,
    pub estimated_occupancy_bytes: u64,
    pub estimated_total_bytes: u64,
    pub classification: String,
    pub suggested_action: SuggestedImportAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportResolvedDataset {
    pub classification: String,
    pub stacking_state: String,
    pub organization: String,
    pub layout: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_axis_kind: Option<String>,
    pub inline_count: usize,
    pub crossline_count: usize,
    pub third_axis_count: usize,
    pub trace_count: u64,
    pub samples_per_trace: usize,
    pub sample_data_fidelity: SampleDataFidelity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportResolvedSpatial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_field: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_field: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_scalar_field: Option<SegyHeaderField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_units: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinate_reference_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportFieldObservation {
    pub field: SegyHeaderField,
    pub label: String,
    pub unique_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<i64>,
    pub zero_count: usize,
    pub nonzero_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportCandidatePlan {
    pub candidate_id: String,
    pub label: String,
    pub plan_patch: SegyImportPlan,
    pub resolved_dataset: SegyImportResolvedDataset,
    pub risk_summary: SegyImportRiskSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<SegyImportIssue>,
    pub auto_selectable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ScanSegyImportRequest {
    pub schema_version: u32,
    pub input_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportScanResponse {
    pub schema_version: u32,
    pub input_path: String,
    pub source_fingerprint: String,
    pub file_size: u64,
    pub trace_count: u64,
    pub samples_per_trace: usize,
    pub sample_interval_us: u16,
    pub sample_format_code: u16,
    pub endianness: String,
    pub default_plan: SegyImportPlan,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_plans: Vec<SegyImportCandidatePlan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_observations: Vec<SegyImportFieldObservation>,
    pub risk_summary: SegyImportRiskSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<SegyImportIssue>,
    pub recommended_next_stage: SegyImportWizardStage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ValidateSegyImportPlanRequest {
    pub schema_version: u32,
    pub plan: SegyImportPlan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportValidationResponse {
    pub schema_version: u32,
    pub validated_plan: SegyImportPlan,
    pub validation_fingerprint: String,
    pub resolved_dataset: SegyImportResolvedDataset,
    pub resolved_spatial: SegyImportResolvedSpatial,
    pub risk_summary: SegyImportRiskSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<SegyImportIssue>,
    pub can_import: bool,
    pub requires_acknowledgement: bool,
    pub recommended_next_stage: SegyImportWizardStage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportSegyWithPlanRequest {
    pub schema_version: u32,
    pub plan: SegyImportPlan,
    pub validation_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportSegyWithPlanResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SegyImportRecipe {
    pub recipe_id: String,
    pub name: String,
    pub scope: SegyImportRecipeScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
    pub plan: SegyImportPlan,
    pub created_at_unix_s: u64,
    pub updated_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ListSegyImportRecipesRequest {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ListSegyImportRecipesResponse {
    pub schema_version: u32,
    pub recipes: Vec<SegyImportRecipe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SaveSegyImportRecipeRequest {
    pub schema_version: u32,
    pub recipe: SegyImportRecipe,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SaveSegyImportRecipeResponse {
    pub schema_version: u32,
    pub recipe: SegyImportRecipe,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeleteSegyImportRecipeRequest {
    pub schema_version: u32,
    pub recipe_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeleteSegyImportRecipeResponse {
    pub schema_version: u32,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportDatasetRequest {
    pub schema_version: u32,
    pub input_path: String,
    pub output_store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_override: Option<SegyGeometryOverride>,
    #[serde(default)]
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportDatasetResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum PrestackThirdAxisField {
    Offset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportPrestackOffsetDatasetRequest {
    pub schema_version: u32,
    pub input_path: String,
    pub output_store_path: String,
    pub third_axis_field: PrestackThirdAxisField,
    #[serde(default)]
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportPrestackOffsetDatasetResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct OpenDatasetRequest {
    pub schema_version: u32,
    pub store_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct OpenDatasetResponse {
    pub schema_version: u32,
    pub dataset: DatasetSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExportSegyRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub output_path: String,
    #[serde(default)]
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExportSegyResponse {
    pub schema_version: u32,
    pub store_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportHorizonXyzRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub input_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_domain: Option<TimeDepthDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_coordinate_reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_coordinate_reference_name: Option<String>,
    #[serde(default)]
    pub assume_same_as_survey: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportHorizonXyzResponse {
    pub schema_version: u32,
    pub imported: Vec<ImportedHorizonDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct LoadSectionHorizonsRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub axis: SectionAxis,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct LoadSectionHorizonsResponse {
    pub schema_version: u32,
    pub overlays: Vec<SectionHorizonOverlayView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewCommand {
    pub schema_version: u32,
    pub request: SectionRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewResponse {
    pub schema_version: u32,
    pub preview: PreviewView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewTraceLocalProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub section: SectionRequest,
    pub pipeline: TraceLocalProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewTraceLocalProcessingResponse {
    pub schema_version: u32,
    pub preview: PreviewView,
    pub pipeline: TraceLocalProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewSubvolumeProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub section: SectionRequest,
    pub pipeline: SubvolumeProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewSubvolumeProcessingResponse {
    pub schema_version: u32,
    pub preview: PreviewView,
    pub pipeline: SubvolumeProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunTraceLocalProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_store_path: Option<String>,
    #[serde(default)]
    pub overwrite_existing: bool,
    pub pipeline: TraceLocalProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunTraceLocalProcessingResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunSubvolumeProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_store_path: Option<String>,
    #[serde(default)]
    pub overwrite_existing: bool,
    pub pipeline: SubvolumeProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunSubvolumeProcessingResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewGatherProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub gather: GatherRequest,
    pub pipeline: GatherProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct PreviewGatherProcessingResponse {
    pub schema_version: u32,
    pub preview: GatherPreviewView,
    pub pipeline: GatherProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunGatherProcessingRequest {
    pub schema_version: u32,
    pub store_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_store_path: Option<String>,
    #[serde(default)]
    pub overwrite_existing: bool,
    pub pipeline: GatherProcessingPipeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct RunGatherProcessingResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityScanRequest {
    pub schema_version: u32,
    pub store_path: String,
    pub gather: GatherRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_local_pipeline: Option<TraceLocalProcessingPipeline>,
    pub min_velocity_m_per_s: f32,
    pub max_velocity_m_per_s: f32,
    pub velocity_step_m_per_s: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autopick: Option<VelocityAutopickParameters>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SemblancePanel {
    pub velocities_m_per_s: Vec<f32>,
    pub sample_axis_ms: Vec<f32>,
    pub semblance_f32le: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum VelocityPickStrategy {
    MaximumSemblance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityAutopickParameters {
    pub sample_stride: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_time_ms: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_time_ms: Option<f32>,
    pub min_semblance: f32,
    pub smoothing_samples: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityFunctionEstimate {
    pub strategy: VelocityPickStrategy,
    pub times_ms: Vec<f32>,
    pub velocities_m_per_s: Vec<f32>,
    pub semblance: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct VelocityScanResponse {
    pub schema_version: u32,
    pub gather: GatherRequest,
    pub panel: SemblancePanel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autopicked_velocity_function: Option<VelocityFunctionEstimate>,
}

pub type PreviewProcessingRequest = PreviewTraceLocalProcessingRequest;
pub type PreviewProcessingResponse = PreviewTraceLocalProcessingResponse;
pub type RunProcessingRequest = RunTraceLocalProcessingRequest;
pub type RunProcessingResponse = RunTraceLocalProcessingResponse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GetProcessingJobRequest {
    pub schema_version: u32,
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct GetProcessingJobResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CancelProcessingJobRequest {
    pub schema_version: u32,
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct CancelProcessingJobResponse {
    pub schema_version: u32,
    pub job: ProcessingJobStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ListPipelinePresetsResponse {
    pub schema_version: u32,
    pub presets: Vec<TraceLocalProcessingPreset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SavePipelinePresetRequest {
    pub schema_version: u32,
    pub preset: TraceLocalProcessingPreset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SavePipelinePresetResponse {
    pub schema_version: u32,
    pub preset: TraceLocalProcessingPreset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeletePipelinePresetRequest {
    pub schema_version: u32,
    pub preset_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeletePipelinePresetResponse {
    pub schema_version: u32,
    pub deleted: bool,
}

pub const IPC_SCHEMA_VERSION: u32 = 1;
