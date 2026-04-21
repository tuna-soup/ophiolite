use crate::{
    AssetBindingInput, AssetId, CoordinateReference, IndexKind, IngestIssue, LasFile,
    OphioliteProject, ProjectAssetImportResult, Provenance, TopRow, TrajectoryRow, WellMarkerRow,
};
use ophiolite_core::{
    CurveItem, HeaderItem, IndexDescriptor, LasError, LasFileSummary, LasValue, MnemonicCase,
    Result, SectionItems, revision_token_for_bytes,
};
use ophiolite_seismic::{
    CheckshotVspObservationSet1D, CoordinateReferenceDescriptor, DepthReferenceKind,
    TimeDepthDomain, TimeDepthSample1D, TimeDepthTransformSourceKind, TravelTimeReference,
    WellTimeDepthModel1D, WellTimeDepthObservationSample,
};
use ophiolite_seismic_runtime::build_suggested_horizon_source_import_draft;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const VENDOR_PROJECT_IMPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectImportVendor {
    Opendtect,
    Petrel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectBridgeKind {
    OpendtectCbvsVolumeExport,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectBridgeFormat {
    Segy,
    ZarrStore,
    TbvolStore,
    OpenVdsStore,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectObjectKind {
    Survey,
    SeismicVolume,
    SeismicHorizon,
    Well,
    WellLog,
    TopSet,
    WellMarkerSet,
    WellTimeDepthModel,
    CheckshotVspObservationSet,
    Fault,
    Body,
    PickSet,
    RandomLine,
    VelocityFunction,
    Shapefile,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectCanonicalTargetKind {
    SeismicTraceData,
    SurveyStoreHorizon,
    Log,
    Trajectory,
    TopSet,
    WellMarkerSet,
    WellTimeDepthModel,
    CheckshotVspObservationSet,
    RawSourceBundle,
    ExternalOpenFormat,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectImportDisposition {
    Canonical,
    CanonicalWithLoss,
    RawSourceOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectImportIssueSeverity {
    Info,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectImportIssue {
    pub severity: VendorProjectImportIssueSeverity,
    pub code: String,
    pub message: String,
    pub source_path: Option<String>,
    pub vendor_object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectSurveyMetadata {
    pub name: Option<String>,
    pub survey_data_type: Option<String>,
    pub inline_range: Option<[i32; 3]>,
    pub crossline_range: Option<[i32; 3]>,
    pub z_range: Option<[f64; 3]>,
    pub z_domain: Option<String>,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub coordinate_reference_source_path: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectObjectPreview {
    pub vendor_object_id: String,
    pub vendor_kind: VendorProjectObjectKind,
    pub display_name: String,
    pub source_paths: Vec<String>,
    pub canonical_target_kind: VendorProjectCanonicalTargetKind,
    pub disposition: VendorProjectImportDisposition,
    pub requires_crs_decision: bool,
    pub default_selected: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectScanRequest {
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectScanResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub vendor_project: Option<String>,
    pub survey_metadata: VendorProjectSurveyMetadata,
    pub objects: Vec<VendorProjectObjectPreview>,
    pub issues: Vec<VendorProjectImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectPlanRequest {
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub selected_vendor_object_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_project_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_survey_asset_id: Option<String>,
    pub binding: Option<crate::AssetBindingInput>,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_probe: Option<VendorProjectRuntimeProbeRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectPlannedImport {
    pub vendor_object_id: String,
    pub display_name: String,
    pub canonical_target_kind: VendorProjectCanonicalTargetKind,
    pub disposition: VendorProjectImportDisposition,
    #[serde(default)]
    pub requires_target_survey_asset: bool,
    pub source_paths: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeRequest {
    pub vendor_object_id: String,
    pub display_name: String,
    pub bridge_kind: VendorProjectBridgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_native_id: Option<String>,
    pub recommended_output_format: VendorProjectBridgeFormat,
    pub accepted_output_formats: Vec<VendorProjectBridgeFormat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automatic_execution_formats: Vec<VendorProjectBridgeFormat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_requirements: Vec<VendorProjectBridgeRuntimeRequirement>,
    pub source_paths: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectPlanResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub planned_imports: Vec<VendorProjectPlannedImport>,
    pub bridge_requests: Vec<VendorProjectBridgeRequest>,
    #[serde(default)]
    pub target_survey_asset_required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_survey_asset_candidates: Vec<crate::ProjectSurveyAssetInventoryItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_target_survey_asset: Option<crate::ProjectSurveyAssetInventoryItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_probe: Option<VendorProjectRuntimeProbeResponse>,
    pub blocking_issues: Vec<VendorProjectImportIssue>,
    pub warnings: Vec<VendorProjectImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeOutput {
    pub vendor_object_id: String,
    pub format: VendorProjectBridgeFormat,
    pub path: String,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectBridgeExecutionStatus {
    Prepared,
    Executed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectBridgeRuntimeRequirement {
    VendorBatchExecutable,
    VendorProjectDataRoot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectConnectorPhase {
    Discovery,
    Planning,
    RuntimeProbe,
    BridgePreparation,
    BridgeExecution,
    CanonicalCommit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectConnectorIsolationBoundary {
    InProcess,
    OutOfProcess,
    Hybrid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectConnectorProvenanceGuarantee {
    VendorObjectId,
    SourcePath,
    BridgeArtifactPath,
    RuntimeIssue,
    CoordinateReferenceDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectConnectorPhaseSupport {
    pub phase: VendorProjectConnectorPhase,
    pub isolation_boundary: VendorProjectConnectorIsolationBoundary,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeCapability {
    pub bridge_kind: VendorProjectBridgeKind,
    pub supported_vendor_object_prefixes: Vec<String>,
    pub recommended_output_format: VendorProjectBridgeFormat,
    pub accepted_output_formats: Vec<VendorProjectBridgeFormat>,
    pub automatic_execution_formats: Vec<VendorProjectBridgeFormat>,
    pub runtime_requirements: Vec<VendorProjectBridgeRuntimeRequirement>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeCapabilitiesResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub capabilities: Vec<VendorProjectBridgeCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectConnectorContractResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub phases: Vec<VendorProjectConnectorPhaseSupport>,
    pub supported_runtime_kinds: Vec<VendorProjectRuntimeKind>,
    pub bridge_capabilities: Vec<VendorProjectBridgeCapability>,
    pub provenance_guarantees: Vec<VendorProjectConnectorProvenanceGuarantee>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectRuntimeKind {
    OpendtectOdbind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectRuntimeProbeStatus {
    Ok,
    ImportError,
    SurveyError,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectRuntimeObjectOpenStatus {
    NotAttempted,
    Opened,
    OpenError,
    ImportError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectRuntimeProbeRequest {
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub runtime: VendorProjectRuntimeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survey_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python_executable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odbind_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dtect_appl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_library_path: Option<String>,
    #[serde(default = "default_true")]
    pub probe_bridgeable_objects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectRuntimeObjectGroup {
    pub name: String,
    pub object_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectRuntimeObjectStatus {
    pub vendor_object_id: String,
    pub display_name: String,
    pub runtime_group: String,
    pub listed_in_runtime: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_object: Option<bool>,
    pub open_status: VendorProjectRuntimeObjectOpenStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_info_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_error: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectRuntimeProbeResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub runtime: VendorProjectRuntimeKind,
    pub survey_name: String,
    pub project_data_root: String,
    pub probe_status: VendorProjectRuntimeProbeStatus,
    pub survey_visible: bool,
    pub survey_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survey_info: Option<serde_json::Value>,
    pub object_groups: Vec<VendorProjectRuntimeObjectGroup>,
    pub object_statuses: Vec<VendorProjectRuntimeObjectStatus>,
    pub notes: Vec<String>,
    pub issues: Vec<VendorProjectImportIssue>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorProjectBridgeArtifactKind {
    ParameterFile,
    LogFile,
    BridgeOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeArtifact {
    pub kind: VendorProjectBridgeArtifactKind,
    pub path: String,
    pub exists: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeRunRequest {
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub vendor_object_id: String,
    pub output_format: VendorProjectBridgeFormat,
    pub output_path: String,
    pub installation_root: Option<String>,
    pub executable_path: Option<String>,
    pub parameter_file_path: Option<String>,
    pub log_path: Option<String>,
    #[serde(default)]
    pub execute: bool,
    #[serde(default)]
    pub overwrite_existing_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeRunResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub vendor_object_id: String,
    pub display_name: String,
    pub bridge_kind: VendorProjectBridgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_native_id: Option<String>,
    pub output: VendorProjectBridgeOutput,
    pub parameter_file_path: String,
    pub log_path: Option<String>,
    pub artifacts: Vec<VendorProjectBridgeArtifact>,
    pub command: Vec<String>,
    pub execution_status: VendorProjectBridgeExecutionStatus,
    pub notes: Vec<String>,
    pub issues: Vec<VendorProjectImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeCommitRequest {
    pub bridge_run: VendorProjectBridgeRunRequest,
    pub target_project_root: Option<String>,
    pub binding: Option<AssetBindingInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_survey_asset_id: Option<String>,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectBridgeCommitResponse {
    pub bridge: VendorProjectBridgeRunResponse,
    pub commit: VendorProjectCommitResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectCommitRequest {
    pub plan: VendorProjectPlanResponse,
    pub target_project_root: Option<String>,
    pub binding: Option<AssetBindingInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_survey_asset_id: Option<String>,
    pub coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    pub bridge_outputs: Vec<VendorProjectBridgeOutput>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectValidationReport {
    pub vendor_object_id: String,
    pub display_name: String,
    pub checks: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectCommittedAsset {
    pub vendor_object_id: String,
    pub display_name: String,
    pub canonical_target_kind: VendorProjectCanonicalTargetKind,
    pub disposition: VendorProjectImportDisposition,
    pub asset_id: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub source_paths: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorProjectCommitResponse {
    pub schema_version: u32,
    pub vendor: VendorProjectImportVendor,
    pub project_root: String,
    pub target_project_root: Option<String>,
    pub imported_assets: Vec<VendorProjectCommittedAsset>,
    pub preserved_raw_sources: Vec<VendorProjectCommittedAsset>,
    pub validation_reports: Vec<VendorProjectValidationReport>,
    pub issues: Vec<VendorProjectImportIssue>,
}

struct VendorProjectBridgeCapabilityDefinition {
    bridge_kind: VendorProjectBridgeKind,
    supported_vendor_object_prefixes: &'static [&'static str],
    recommended_output_format: VendorProjectBridgeFormat,
    accepted_output_formats: &'static [VendorProjectBridgeFormat],
    automatic_execution_formats: &'static [VendorProjectBridgeFormat],
    runtime_requirements: &'static [VendorProjectBridgeRuntimeRequirement],
    notes: &'static [&'static str],
}

struct VendorProjectConnectorPhaseDefinition {
    phase: VendorProjectConnectorPhase,
    isolation_boundary: VendorProjectConnectorIsolationBoundary,
    notes: &'static [&'static str],
}

const OPENDTECT_CBVS_ACCEPTED_OUTPUT_FORMATS: &[VendorProjectBridgeFormat] = &[
    VendorProjectBridgeFormat::Segy,
    VendorProjectBridgeFormat::TbvolStore,
    VendorProjectBridgeFormat::ZarrStore,
    VendorProjectBridgeFormat::OpenVdsStore,
];

const OPENDTECT_CBVS_AUTOMATIC_EXECUTION_FORMATS: &[VendorProjectBridgeFormat] =
    &[VendorProjectBridgeFormat::Segy];

const OPENDTECT_CBVS_RUNTIME_REQUIREMENTS: &[VendorProjectBridgeRuntimeRequirement] = &[
    VendorProjectBridgeRuntimeRequirement::VendorBatchExecutable,
    VendorProjectBridgeRuntimeRequirement::VendorProjectDataRoot,
];

const OPENDTECT_CBVS_SUPPORTED_PREFIXES: &[&str] = &["seismic-cbvs:"];

const OPENDTECT_CBVS_BRIDGE_NOTES: &[&str] = &[
    "Phase-one canonical import can consume a bridge-exported SEG-Y companion immediately.",
    "A future native bridge may emit tbvol directly and skip the extra ingest step.",
];

const OPENDTECT_DISCOVERY_PHASE_NOTES: &[&str] =
    &["Discovery is file-system metadata driven and does not require a vendor runtime."];
const OPENDTECT_PLANNING_PHASE_NOTES: &[&str] = &[
    "Planning resolves canonical targets, bridge requests, CRS decisions, and warnings before import.",
];
const OPENDTECT_RUNTIME_PROBE_PHASE_NOTES: &[&str] =
    &["Runtime probing executes the vendor-supported ODBind surface out of process."];
const OPENDTECT_BRIDGE_PREPARATION_PHASE_NOTES: &[&str] =
    &["Bridge preparation materializes parameter files and command lines inside Ophiolite."];
const OPENDTECT_BRIDGE_EXECUTION_PHASE_NOTES: &[&str] = &[
    "Bridge execution is runtime-dependent and should run in an isolated vendor worker environment.",
];
const OPENDTECT_CANONICAL_COMMIT_PHASE_NOTES: &[&str] = &[
    "Canonical commit remains inside Ophiolite and records provenance from vendor discovery and bridge outputs.",
];

const OPENDTECT_CONNECTOR_PHASES: &[VendorProjectConnectorPhaseDefinition] = &[
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::Discovery,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: OPENDTECT_DISCOVERY_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::Planning,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: OPENDTECT_PLANNING_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::RuntimeProbe,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::OutOfProcess,
        notes: OPENDTECT_RUNTIME_PROBE_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::BridgePreparation,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: OPENDTECT_BRIDGE_PREPARATION_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::BridgeExecution,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::OutOfProcess,
        notes: OPENDTECT_BRIDGE_EXECUTION_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::CanonicalCommit,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: OPENDTECT_CANONICAL_COMMIT_PHASE_NOTES,
    },
];

const OPENDTECT_SUPPORTED_RUNTIME_KINDS: &[VendorProjectRuntimeKind] =
    &[VendorProjectRuntimeKind::OpendtectOdbind];

const OPENDTECT_PROVENANCE_GUARANTEES: &[VendorProjectConnectorProvenanceGuarantee] = &[
    VendorProjectConnectorProvenanceGuarantee::VendorObjectId,
    VendorProjectConnectorProvenanceGuarantee::SourcePath,
    VendorProjectConnectorProvenanceGuarantee::BridgeArtifactPath,
    VendorProjectConnectorProvenanceGuarantee::RuntimeIssue,
    VendorProjectConnectorProvenanceGuarantee::CoordinateReferenceDecision,
];

const OPENDTECT_CONNECTOR_NOTES: &[&str] = &[
    "Vendor import is modeled as phased discovery, planning, runtime validation, extraction, and canonical commit.",
    "Bridge execution remains capability-gated because real ODBind and batch extraction behavior is runtime-dependent.",
];

const PETREL_DISCOVERY_PHASE_NOTES: &[&str] = &[
    "Discovery parses a Petrel export-bundle folder in process and previews wells, logs, trajectories, tops, checkshots, and horizon point exports.",
];
const PETREL_PLANNING_PHASE_NOTES: &[&str] = &[
    "Planning resolves export-bundle object selection, single-well commit boundaries, and geospatial gating.",
];
const PETREL_CANONICAL_COMMIT_PHASE_NOTES: &[&str] = &[
    "Phase one canonical commit supports Petrel LAS logs, .dev trajectories, tops exports, and checkshot exports for one selected well per request.",
    "Horizon point exports currently preserve as raw source bundles until their survey/grid-aware canonical mappings and policies are finalized.",
];
const PETREL_CONNECTOR_PHASES: &[VendorProjectConnectorPhaseDefinition] = &[
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::Discovery,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: PETREL_DISCOVERY_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::Planning,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: PETREL_PLANNING_PHASE_NOTES,
    },
    VendorProjectConnectorPhaseDefinition {
        phase: VendorProjectConnectorPhase::CanonicalCommit,
        isolation_boundary: VendorProjectConnectorIsolationBoundary::InProcess,
        notes: PETREL_CANONICAL_COMMIT_PHASE_NOTES,
    },
];
const PETREL_SUPPORTED_RUNTIME_KINDS: &[VendorProjectRuntimeKind] = &[];
const PETREL_PROVENANCE_GUARANTEES: &[VendorProjectConnectorProvenanceGuarantee] = &[
    VendorProjectConnectorProvenanceGuarantee::VendorObjectId,
    VendorProjectConnectorProvenanceGuarantee::SourcePath,
    VendorProjectConnectorProvenanceGuarantee::CoordinateReferenceDecision,
];
const PETREL_CONNECTOR_NOTES: &[&str] = &[
    "Phase one targets Petrel export bundles and future plugin-emitted export contracts, not private native project internals.",
    "Petrel canonical commit is available for single-well logs, trajectories, tops, and checkshots; horizon point exports preserve as raw source bundles, and runtime probe and bridge execution remain unimplemented.",
];
const PETREL_BRIDGE_CAPABILITIES: &[VendorProjectBridgeCapabilityDefinition] = &[];

const OPENDTECT_BRIDGE_CAPABILITIES: &[VendorProjectBridgeCapabilityDefinition] =
    &[VendorProjectBridgeCapabilityDefinition {
        bridge_kind: VendorProjectBridgeKind::OpendtectCbvsVolumeExport,
        supported_vendor_object_prefixes: OPENDTECT_CBVS_SUPPORTED_PREFIXES,
        recommended_output_format: VendorProjectBridgeFormat::Segy,
        accepted_output_formats: OPENDTECT_CBVS_ACCEPTED_OUTPUT_FORMATS,
        automatic_execution_formats: OPENDTECT_CBVS_AUTOMATIC_EXECUTION_FORMATS,
        runtime_requirements: OPENDTECT_CBVS_RUNTIME_REQUIREMENTS,
        notes: OPENDTECT_CBVS_BRIDGE_NOTES,
    }];

const OPENDTECT_ODBIND_PROBE_SCRIPT: &str =
    include_str!("../scripts/validation/opendtect_odbind_probe.py");

pub fn scan_vendor_project(
    request: &VendorProjectScanRequest,
) -> Result<VendorProjectScanResponse> {
    match request.vendor {
        VendorProjectImportVendor::Opendtect => scan_opendtect_project(request),
        VendorProjectImportVendor::Petrel => scan_petrel_project(request),
    }
}

pub fn vendor_project_bridge_capabilities(
    vendor: VendorProjectImportVendor,
) -> VendorProjectBridgeCapabilitiesResponse {
    VendorProjectBridgeCapabilitiesResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor,
        capabilities: bridge_capability_definitions(vendor)
            .iter()
            .map(public_bridge_capability)
            .collect(),
    }
}

pub fn vendor_project_connector_contract(
    vendor: VendorProjectImportVendor,
) -> VendorProjectConnectorContractResponse {
    VendorProjectConnectorContractResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor,
        phases: connector_phase_definitions(vendor)
            .iter()
            .map(public_connector_phase)
            .collect(),
        supported_runtime_kinds: supported_runtime_kinds(vendor).to_vec(),
        bridge_capabilities: bridge_capability_definitions(vendor)
            .iter()
            .map(public_bridge_capability)
            .collect(),
        provenance_guarantees: connector_provenance_guarantees(vendor).to_vec(),
        notes: connector_notes(vendor)
            .iter()
            .map(|note| (*note).to_string())
            .collect(),
    }
}

pub fn probe_vendor_project_runtime(
    request: &VendorProjectRuntimeProbeRequest,
) -> Result<VendorProjectRuntimeProbeResponse> {
    match (request.vendor, request.runtime) {
        (VendorProjectImportVendor::Opendtect, VendorProjectRuntimeKind::OpendtectOdbind) => {
            probe_opendtect_runtime(request)
        }
        _ => Err(LasError::Validation(format!(
            "Vendor runtime `{:?}` is not supported for vendor `{:?}`.",
            request.runtime, request.vendor
        ))),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpendtectOdbindProbeResponse {
    status: String,
    #[serde(default)]
    survey_names: Vec<String>,
    #[serde(default)]
    survey_info: Option<serde_json::Value>,
    #[serde(default)]
    object_names: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    error: Option<OpendtectOdbindProbeError>,
    #[serde(default)]
    volume_probe: Option<OpendtectOdbindVolumeProbe>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpendtectOdbindProbeError {
    #[serde(default)]
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpendtectOdbindVolumeProbe {
    #[serde(default)]
    has_object: Option<bool>,
    #[serde(default)]
    object_info_error: Option<OpendtectOdbindProbeError>,
    #[serde(default)]
    open_status: Option<String>,
    #[serde(default)]
    import_error: Option<OpendtectOdbindProbeError>,
    #[serde(default)]
    open_error: Option<OpendtectOdbindProbeError>,
}

fn probe_opendtect_runtime(
    request: &VendorProjectRuntimeProbeRequest,
) -> Result<VendorProjectRuntimeProbeResponse> {
    let project_root = Path::new(&request.project_root);
    if !project_root.is_dir() {
        return Err(LasError::Validation(format!(
            "Vendor project root `{}` does not exist or is not a directory.",
            request.project_root
        )));
    }
    let survey_name = request
        .survey_name
        .clone()
        .unwrap_or_else(|| infer_vendor_project_name(project_root));
    let project_data_root = prepare_opendtect_runtime_data_root(project_root, &survey_name)?;
    let scan = scan_vendor_project(&VendorProjectScanRequest {
        vendor: request.vendor,
        project_root: request.project_root.clone(),
    })?;
    let survey_probe = run_opendtect_odbind_probe(
        request,
        &project_data_root,
        &survey_name,
        None,
        OPENDTECT_ODBIND_PROBE_SCRIPT,
    )?;
    let probe_status = map_odbind_probe_status(&survey_probe.status);
    let survey_visible = survey_probe
        .survey_names
        .iter()
        .any(|candidate| candidate == &survey_name);
    let mut issues = Vec::new();
    let mut notes = vec![format!(
        "Prepared synthetic OpendTect data root at `{}` for runtime probing.",
        project_data_root.display()
    )];
    add_probe_status_issue(
        &mut issues,
        probe_status,
        survey_probe
            .error
            .as_ref()
            .map(|error| error.message.as_str()),
    );
    if !survey_visible {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("vendor_runtime_survey_unavailable"),
            message: format!(
                "Vendor runtime `{}` did not report survey `{}` under synthetic data root `{}`.",
                runtime_label(request.runtime),
                survey_name,
                project_data_root.display()
            ),
            source_path: Some(request.project_root.clone()),
            vendor_object_id: None,
        });
    }
    let mut object_groups = survey_probe
        .object_names
        .into_iter()
        .map(|(name, mut object_names)| {
            object_names.sort();
            VendorProjectRuntimeObjectGroup { name, object_names }
        })
        .collect::<Vec<_>>();
    object_groups.sort_by(|left, right| left.name.cmp(&right.name));

    let seismic_runtime_names = object_groups
        .iter()
        .find(|group| group.name == "Seismic Data")
        .map(|group| group.object_names.iter().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();

    let mut object_statuses = Vec::new();
    if request.probe_bridgeable_objects {
        for object in scan
            .objects
            .iter()
            .filter(|object| bridge_capability_for_object(request.vendor, object).is_some())
        {
            let listed_runtime_name =
                resolve_opendtect_runtime_name(&object.display_name, &seismic_runtime_names);
            let object_probe = run_opendtect_odbind_probe(
                request,
                &project_data_root,
                &survey_name,
                listed_runtime_name
                    .as_deref()
                    .or(Some(object.display_name.as_str())),
                OPENDTECT_ODBIND_PROBE_SCRIPT,
            )?;
            let volume_probe = object_probe.volume_probe.as_ref();
            let open_status = volume_probe
                .map(map_volume_probe_open_status)
                .unwrap_or(VendorProjectRuntimeObjectOpenStatus::NotAttempted);
            let object_info_error = volume_probe
                .and_then(|probe| probe.object_info_error.as_ref())
                .map(|error| error.message.clone());
            let open_error = volume_probe.and_then(|probe| {
                probe
                    .open_error
                    .as_ref()
                    .or(probe.import_error.as_ref())
                    .map(|error| error.message.clone())
            });
            let listed_in_runtime = listed_runtime_name.is_some();
            if open_status != VendorProjectRuntimeObjectOpenStatus::Opened {
                issues.push(VendorProjectImportIssue {
                    severity: VendorProjectImportIssueSeverity::Warning,
                    code: String::from("vendor_runtime_object_open_failed"),
                    message: format!(
                        "Vendor runtime `{}` could not open `{}`: {}",
                        runtime_label(request.runtime),
                        object.display_name,
                        open_error
                            .clone()
                            .or(object_info_error.clone())
                            .unwrap_or_else(|| String::from("object was not opened"))
                    ),
                    source_path: object.source_paths.first().cloned(),
                    vendor_object_id: Some(object.vendor_object_id.clone()),
                });
            }
            object_statuses.push(VendorProjectRuntimeObjectStatus {
                vendor_object_id: object.vendor_object_id.clone(),
                display_name: object.display_name.clone(),
                runtime_group: String::from("Seismic Data"),
                listed_in_runtime,
                has_object: volume_probe.and_then(|probe| probe.has_object),
                open_status,
                object_info_error,
                open_error,
                notes: vec![String::from(
                    "Bridgeable OpendTect seismic objects are probed through the vendor ODBind runtime before extraction.",
                )],
            });
        }
    } else {
        notes.push(String::from(
            "Bridgeable object open probes were disabled for this runtime check.",
        ));
    }

    Ok(VendorProjectRuntimeProbeResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor: request.vendor,
        project_root: request.project_root.clone(),
        runtime: request.runtime,
        survey_name,
        project_data_root: path_string(&project_data_root),
        probe_status,
        survey_visible,
        survey_names: survey_probe.survey_names,
        survey_info: survey_probe.survey_info,
        object_groups,
        object_statuses,
        notes,
        issues,
    })
}

fn run_opendtect_odbind_probe(
    request: &VendorProjectRuntimeProbeRequest,
    basedir: &Path,
    survey_name: &str,
    volume: Option<&str>,
    script_source: &str,
) -> Result<OpendtectOdbindProbeResponse> {
    let python_executable = request.python_executable.as_deref().unwrap_or("python3");
    let mut command = Command::new(python_executable);
    command.arg("-c").arg(script_source);
    command.arg("--basedir").arg(basedir);
    command.arg("--survey").arg(survey_name);
    if let Some(odbind_root) = request.odbind_root.as_deref() {
        command.arg("--odbind-root").arg(odbind_root);
    }
    if let Some(volume_name) = volume {
        command.arg("--volume").arg(volume_name);
    }
    if let Some(dtect_appl) = request.dtect_appl.as_deref() {
        command.env("DTECT_APPL", dtect_appl);
    }
    if let Some(shared_library_path) = request.shared_library_path.as_deref() {
        command.env("DYLD_LIBRARY_PATH", shared_library_path);
        command.env("LD_LIBRARY_PATH", shared_library_path);
    }
    let output = command.output().map_err(|error| {
        LasError::Validation(format!(
            "Failed to launch runtime probe executable `{python_executable}`: {error}"
        ))
    })?;
    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        LasError::Validation(format!(
            "Runtime probe emitted non-UTF8 stdout for `{python_executable}`: {error}"
        ))
    })?;
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LasError::Validation(format!(
            "Runtime probe `{python_executable}` did not emit JSON output. stderr: {}",
            stderr.trim()
        )));
    }
    serde_json::from_str::<OpendtectOdbindProbeResponse>(&stdout).map_err(|error| {
        LasError::Validation(format!(
            "Failed to parse runtime probe JSON from `{python_executable}`: {error}"
        ))
    })
}

fn prepare_opendtect_runtime_data_root(project_root: &Path, survey_name: &str) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| LasError::Validation(format!("system clock error: {error}")))?
        .as_nanos();
    let root = env::temp_dir().join(format!("ophiolite-opendtect-runtime-{timestamp}"));
    fs::create_dir_all(&root)?;
    write_opendtect_runtime_root_omf(&root, survey_name)?;
    symlink_directory(project_root, &root.join(survey_name))?;
    Ok(root)
}

fn write_opendtect_runtime_root_omf(root: &Path, survey_name: &str) -> Result<()> {
    let omf = format!(
        "dTect V4.0\nObject Management file\nGenerated by Ophiolite\n!\nID: -1\n!\nAppl dir: 1\nAppl: dGB`Stream\n$Name: appl\n!\n@0: {survey_name}\n!\n"
    );
    fs::write(root.join(".omf"), omf)?;
    Ok(())
}

fn symlink_directory(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)?;
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(source, target)?;
    }
    Ok(())
}

fn infer_vendor_project_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(String::from)
        .unwrap_or_else(|| String::from("vendor_project"))
}

fn map_odbind_probe_status(status: &str) -> VendorProjectRuntimeProbeStatus {
    match status {
        "ok" => VendorProjectRuntimeProbeStatus::Ok,
        "import_error" => VendorProjectRuntimeProbeStatus::ImportError,
        "survey_error" => VendorProjectRuntimeProbeStatus::SurveyError,
        _ => VendorProjectRuntimeProbeStatus::SurveyError,
    }
}

fn map_volume_probe_open_status(
    probe: &OpendtectOdbindVolumeProbe,
) -> VendorProjectRuntimeObjectOpenStatus {
    match probe.open_status.as_deref() {
        Some("ok") => VendorProjectRuntimeObjectOpenStatus::Opened,
        Some("import_error") => VendorProjectRuntimeObjectOpenStatus::ImportError,
        Some("error") => VendorProjectRuntimeObjectOpenStatus::OpenError,
        _ => VendorProjectRuntimeObjectOpenStatus::NotAttempted,
    }
}

fn add_probe_status_issue(
    issues: &mut Vec<VendorProjectImportIssue>,
    status: VendorProjectRuntimeProbeStatus,
    message: Option<&str>,
) {
    match status {
        VendorProjectRuntimeProbeStatus::Ok => {}
        VendorProjectRuntimeProbeStatus::ImportError => issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("vendor_runtime_import_error"),
            message: format!(
                "Vendor runtime probe could not import its vendor-supported runtime surface: {}",
                message.unwrap_or("runtime import failed")
            ),
            source_path: None,
            vendor_object_id: None,
        }),
        VendorProjectRuntimeProbeStatus::SurveyError => issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("vendor_runtime_survey_error"),
            message: format!(
                "Vendor runtime probe could not open the survey: {}",
                message.unwrap_or("survey open failed")
            ),
            source_path: None,
            vendor_object_id: None,
        }),
    }
}

fn runtime_label(runtime: VendorProjectRuntimeKind) -> &'static str {
    match runtime {
        VendorProjectRuntimeKind::OpendtectOdbind => "opendtect_odbind",
    }
}

fn validate_runtime_probe_request_matches_plan(request: &VendorProjectPlanRequest) -> Result<()> {
    let Some(runtime_probe) = request.runtime_probe.as_ref() else {
        return Ok(());
    };
    if runtime_probe.vendor != request.vendor {
        return Err(LasError::Validation(format!(
            "Runtime probe vendor `{:?}` does not match plan vendor `{:?}`.",
            runtime_probe.vendor, request.vendor
        )));
    }
    if runtime_probe.project_root != request.project_root {
        return Err(LasError::Validation(format!(
            "Runtime probe project root `{}` does not match plan project root `{}`.",
            runtime_probe.project_root, request.project_root
        )));
    }
    Ok(())
}

fn resolve_opendtect_runtime_name(
    display_name: &str,
    runtime_names: &BTreeSet<String>,
) -> Option<String> {
    let runtime_name_map = runtime_names
        .iter()
        .map(|name| (normalized_vendor_name_key(name), name))
        .collect::<BTreeMap<_, _>>();
    opendtect_runtime_name_candidates(display_name)
        .into_iter()
        .find_map(|candidate| {
            runtime_name_map
                .get(&normalized_vendor_name_key(&candidate))
                .map(|name| (*name).clone())
        })
}

fn opendtect_runtime_name_candidates(display_name: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    candidates.push(display_name.to_string());
    let spaced = display_name.replace('_', " ");
    if spaced != display_name {
        candidates.push(spaced);
    }
    candidates
}

pub fn plan_vendor_project_import(
    request: &VendorProjectPlanRequest,
) -> Result<VendorProjectPlanResponse> {
    validate_runtime_probe_request_matches_plan(request)?;
    let scan = scan_vendor_project(&VendorProjectScanRequest {
        vendor: request.vendor,
        project_root: request.project_root.clone(),
    })?;
    let selected_ids = if request.selected_vendor_object_ids.is_empty() {
        scan.objects
            .iter()
            .filter(|object| object.default_selected)
            .map(|object| object.vendor_object_id.clone())
            .collect::<Vec<_>>()
    } else {
        request.selected_vendor_object_ids.clone()
    };
    let selected_set = selected_ids.iter().cloned().collect::<BTreeSet<_>>();
    let object_map = scan
        .objects
        .iter()
        .map(|object| (object.vendor_object_id.clone(), object))
        .collect::<BTreeMap<_, _>>();
    let effective_coordinate_reference = request
        .coordinate_reference
        .clone()
        .or_else(|| scan.survey_metadata.coordinate_reference.clone());
    let runtime_probe = request
        .runtime_probe
        .as_ref()
        .map(probe_vendor_project_runtime)
        .transpose()?;
    let mut planned_imports = Vec::new();
    let mut bridge_requests = Vec::new();
    let mut warnings = scan.issues.clone();
    if let Some(runtime_probe) = runtime_probe.as_ref() {
        warnings.extend(runtime_probe.issues.clone());
    }
    let mut blocking_issues = Vec::new();
    let target_survey_asset_required = scan.objects.iter().any(|object| {
        selected_set.contains(&object.vendor_object_id)
            && object.canonical_target_kind == VendorProjectCanonicalTargetKind::SurveyStoreHorizon
            && object.disposition != VendorProjectImportDisposition::RawSourceOnly
    });
    let (target_survey_asset_candidates, selected_target_survey_asset, target_survey_issues) =
        resolve_plan_target_survey_context(request, target_survey_asset_required)?;
    merge_plan_target_survey_issues(&mut blocking_issues, &mut warnings, target_survey_issues);

    for selected_id in &selected_ids {
        let Some(object) = object_map.get(selected_id) else {
            warnings.push(VendorProjectImportIssue {
                severity: VendorProjectImportIssueSeverity::Warning,
                code: String::from("unknown_selected_vendor_object"),
                message: format!("Selected vendor object id `{selected_id}` was not found."),
                source_path: None,
                vendor_object_id: Some(selected_id.clone()),
            });
            continue;
        };

        if object.requires_crs_decision
            && object.disposition != VendorProjectImportDisposition::RawSourceOnly
            && effective_coordinate_reference.is_none()
        {
            blocking_issues.push(VendorProjectImportIssue {
                severity: VendorProjectImportIssueSeverity::Blocking,
                code: String::from("missing_coordinate_reference"),
                message: format!(
                    "Vendor object `{}` requires a coordinate-reference decision before canonical import.",
                    object.display_name
                ),
                source_path: object.source_paths.first().cloned(),
                vendor_object_id: Some(object.vendor_object_id.clone()),
            });
        }

        planned_imports.push(VendorProjectPlannedImport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            canonical_target_kind: object.canonical_target_kind,
            disposition: object.disposition,
            requires_target_survey_asset: object.canonical_target_kind
                == VendorProjectCanonicalTargetKind::SurveyStoreHorizon
                && object.disposition != VendorProjectImportDisposition::RawSourceOnly,
            source_paths: object.source_paths.clone(),
            notes: object.notes.clone(),
        });
        if let Some(bridge_request) =
            bridge_request_for_object(request.vendor, Path::new(&request.project_root), object)?
        {
            bridge_requests.push(bridge_request);
        }
    }

    if request.vendor == VendorProjectImportVendor::Petrel {
        blocking_issues.extend(petrel_plan_blocking_issues(
            &selected_ids,
            &object_map,
            effective_coordinate_reference.as_ref(),
        ));
    }

    if selected_set.is_empty() {
        warnings.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("no_selected_vendor_objects"),
            message: String::from("No vendor objects were selected for planning."),
            source_path: None,
            vendor_object_id: None,
        });
    }

    Ok(VendorProjectPlanResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor: request.vendor,
        project_root: request.project_root.clone(),
        planned_imports,
        bridge_requests,
        target_survey_asset_required,
        target_survey_asset_candidates,
        selected_target_survey_asset,
        runtime_probe,
        blocking_issues,
        warnings,
    })
}

fn resolve_plan_target_survey_context(
    request: &VendorProjectPlanRequest,
    target_survey_asset_required: bool,
) -> Result<(
    Vec<crate::ProjectSurveyAssetInventoryItem>,
    Option<crate::ProjectSurveyAssetInventoryItem>,
    Vec<VendorProjectImportIssue>,
)> {
    let mut issues = Vec::new();
    let Some(target_project_root) = request.target_project_root.as_ref() else {
        if target_survey_asset_required {
            issues.push(VendorProjectImportIssue {
                severity: VendorProjectImportIssueSeverity::Warning,
                code: String::from("target_project_root_recommended_for_survey_target_selection"),
                message: String::from(
                    "Selected imports include survey-owned horizon targets; provide `targetProjectRoot` during planning to inspect candidate survey assets and preselect `targetSurveyAssetId`.",
                ),
                source_path: None,
                vendor_object_id: None,
            });
        }
        if request.target_survey_asset_id.is_some() {
            issues.push(VendorProjectImportIssue {
                severity: VendorProjectImportIssueSeverity::Warning,
                code: String::from("target_survey_asset_id_ignored_without_target_project_root"),
                message: String::from(
                    "`targetSurveyAssetId` was supplied without `targetProjectRoot`; planning could not validate the selected survey asset.",
                ),
                source_path: None,
                vendor_object_id: None,
            });
        }
        return Ok((Vec::new(), None, issues));
    };

    let project = OphioliteProject::open(target_project_root)?;
    let candidates = project.project_well_overlay_inventory()?.surveys;
    let selected = request
        .target_survey_asset_id
        .as_ref()
        .and_then(|asset_id| {
            candidates
                .iter()
                .find(|candidate| candidate.asset_id.0 == *asset_id)
                .cloned()
        });

    if target_survey_asset_required && candidates.is_empty() {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Blocking,
            code: String::from("no_target_survey_assets_available"),
            message: format!(
                "Target project `{target_project_root}` does not contain any survey-backed seismic assets that can accept imported horizons."
            ),
            source_path: None,
            vendor_object_id: None,
        });
    }

    if target_survey_asset_required && request.target_survey_asset_id.is_none() {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("missing_target_survey_asset_id"),
            message: String::from(
                "Selected imports include survey-owned horizon targets; choose `targetSurveyAssetId` before non-dry-run commit.",
            ),
            source_path: None,
            vendor_object_id: None,
        });
    }

    if let Some(target_survey_asset_id) = request.target_survey_asset_id.as_ref() {
        if selected.is_none() {
            issues.push(VendorProjectImportIssue {
                severity: VendorProjectImportIssueSeverity::Blocking,
                code: String::from("unknown_target_survey_asset_id"),
                message: format!(
                    "Selected `targetSurveyAssetId` `{target_survey_asset_id}` was not found among the survey-backed seismic assets in target project `{target_project_root}`."
                ),
                source_path: None,
                vendor_object_id: None,
            });
        }
    }

    Ok((candidates, selected, issues))
}

fn merge_plan_target_survey_issues(
    blocking_issues: &mut Vec<VendorProjectImportIssue>,
    warnings: &mut Vec<VendorProjectImportIssue>,
    issues: Vec<VendorProjectImportIssue>,
) {
    for issue in issues {
        match issue.severity {
            VendorProjectImportIssueSeverity::Blocking => blocking_issues.push(issue),
            VendorProjectImportIssueSeverity::Info | VendorProjectImportIssueSeverity::Warning => {
                warnings.push(issue)
            }
        }
    }
}

pub fn commit_vendor_project_import(
    request: &VendorProjectCommitRequest,
) -> Result<VendorProjectCommitResponse> {
    if request.plan.schema_version != VENDOR_PROJECT_IMPORT_SCHEMA_VERSION {
        return Err(LasError::Validation(format!(
            "Unsupported vendor import plan schema version `{}`.",
            request.plan.schema_version
        )));
    }

    let mut issues = request.plan.warnings.clone();
    issues.extend(request.plan.blocking_issues.clone());
    let scan = scan_vendor_project(&VendorProjectScanRequest {
        vendor: request.plan.vendor,
        project_root: request.plan.project_root.clone(),
    })?;
    let object_map = scan
        .objects
        .iter()
        .map(|object| (object.vendor_object_id.clone(), object))
        .collect::<BTreeMap<_, _>>();
    let bridge_output_map = request
        .bridge_outputs
        .iter()
        .map(|output| (output.vendor_object_id.clone(), output))
        .collect::<BTreeMap<_, _>>();
    let effective_coordinate_reference = request
        .coordinate_reference
        .clone()
        .or_else(|| scan.survey_metadata.coordinate_reference.clone());

    let mut imported_assets = Vec::new();
    let mut preserved_raw_sources = Vec::new();
    let mut validation_reports = Vec::new();
    let mut unsupported_selected = Vec::new();

    for planned in &request.plan.planned_imports {
        let checks = if let Some(object) = object_map.get(&planned.vendor_object_id) {
            commit_support_checks(
                request.plan.vendor,
                object,
                bridge_output_map.get(&planned.vendor_object_id).copied(),
                request.target_survey_asset_id.as_deref(),
            )
        } else {
            vec![String::from(
                "selected object was not found during commit rescanning",
            )]
        };
        let unsupported = checks
            .iter()
            .any(|check| check.starts_with("unsupported_for_non_dry_run"));
        if unsupported {
            unsupported_selected.push(planned.display_name.clone());
        }
        let asset = VendorProjectCommittedAsset {
            vendor_object_id: planned.vendor_object_id.clone(),
            display_name: planned.display_name.clone(),
            canonical_target_kind: planned.canonical_target_kind,
            disposition: planned.disposition,
            asset_id: None,
            collection_id: None,
            collection_name: None,
            source_paths: planned.source_paths.clone(),
            notes: planned.notes.clone(),
        };
        if planned.disposition == VendorProjectImportDisposition::RawSourceOnly {
            preserved_raw_sources.push(asset.clone());
        } else {
            imported_assets.push(asset.clone());
        }
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: planned.vendor_object_id.clone(),
            display_name: planned.display_name.clone(),
            checks,
            notes: planned.notes.clone(),
        });
    }

    if request.dry_run {
        return Ok(VendorProjectCommitResponse {
            schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
            vendor: request.plan.vendor,
            project_root: request.plan.project_root.clone(),
            target_project_root: request.target_project_root.clone(),
            imported_assets,
            preserved_raw_sources,
            validation_reports,
            issues,
        });
    }

    if !request.plan.blocking_issues.is_empty() {
        return Err(LasError::Validation(String::from(
            "vendor project commit is blocked by unresolved planning issues",
        )));
    }
    if !unsupported_selected.is_empty() {
        return Err(LasError::Validation(format!(
            "vendor project commit does not yet support non-dry-run execution for: {}",
            unsupported_selected.join(", ")
        )));
    }

    let target_project_root = request.target_project_root.as_ref().ok_or_else(|| {
        LasError::Validation(String::from(
            "vendor project commit requires `targetProjectRoot` for non-dry-run execution",
        ))
    })?;
    let needs_explicit_binding = request.plan.planned_imports.iter().any(|planned| {
        planned.disposition != VendorProjectImportDisposition::RawSourceOnly
            && planned.canonical_target_kind != VendorProjectCanonicalTargetKind::SurveyStoreHorizon
    });
    if !needs_explicit_binding && request.binding.is_none() {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Info,
            code: String::from("project_archive_binding_used"),
            message: String::from(
                "No explicit binding was supplied for raw-source-only vendor commit; preserved sources were attached to the system-owned project archive wellbore.",
            ),
            source_path: None,
            vendor_object_id: None,
        });
    }
    let binding = request.binding.as_ref();
    let target_survey_asset_id = request
        .target_survey_asset_id
        .as_ref()
        .map(|value| AssetId(value.clone()));
    if needs_explicit_binding && binding.is_none() {
        return Err(LasError::Validation(String::from(
            "vendor project commit requires `binding` for non-dry-run execution when canonical well-scoped assets are selected",
        )));
    }
    let mut project = OphioliteProject::open(target_project_root)?;

    imported_assets.clear();
    preserved_raw_sources.clear();
    validation_reports.clear();

    for planned in &request.plan.planned_imports {
        let object = object_map.get(&planned.vendor_object_id).ok_or_else(|| {
            LasError::Validation(format!(
                "planned vendor object `{}` was not found during commit rescanning",
                planned.vendor_object_id
            ))
        })?;
        match request.plan.vendor {
            VendorProjectImportVendor::Opendtect => commit_opendtect_object(
                &mut project,
                &request.plan.project_root,
                object,
                binding,
                effective_coordinate_reference.as_ref(),
                bridge_output_map.get(&planned.vendor_object_id).copied(),
                &mut imported_assets,
                &mut preserved_raw_sources,
                &mut validation_reports,
            )?,
            VendorProjectImportVendor::Petrel => commit_petrel_object(
                &mut project,
                object,
                binding,
                target_survey_asset_id.as_ref(),
                effective_coordinate_reference.as_ref(),
                &mut imported_assets,
                &mut preserved_raw_sources,
                &mut validation_reports,
            )?,
        }
    }

    Ok(VendorProjectCommitResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor: request.plan.vendor,
        project_root: request.plan.project_root.clone(),
        target_project_root: Some(target_project_root.clone()),
        imported_assets,
        preserved_raw_sources,
        validation_reports,
        issues,
    })
}

pub fn run_vendor_project_bridge(
    request: &VendorProjectBridgeRunRequest,
) -> Result<VendorProjectBridgeRunResponse> {
    let scan = scan_vendor_project(&VendorProjectScanRequest {
        vendor: request.vendor,
        project_root: request.project_root.clone(),
    })?;
    let object = scan
        .objects
        .iter()
        .find(|object| object.vendor_object_id == request.vendor_object_id)
        .ok_or_else(|| {
            LasError::Validation(format!(
                "Vendor object `{}` was not found in vendor project `{}`.",
                request.vendor_object_id, request.project_root
            ))
        })?;
    let bridge_request =
        bridge_request_for_object(request.vendor, Path::new(&request.project_root), object)?
            .ok_or_else(|| {
                LasError::Validation(format!(
                    "Vendor object `{}` does not have a supported bridge workflow.",
                    request.vendor_object_id
                ))
            })?;
    let capability = bridge_capability_for_object(request.vendor, object).ok_or_else(|| {
        LasError::Validation(format!(
            "Vendor object `{}` does not have a registered bridge capability.",
            request.vendor_object_id
        ))
    })?;

    match (request.vendor, bridge_request.bridge_kind) {
        (
            VendorProjectImportVendor::Opendtect,
            VendorProjectBridgeKind::OpendtectCbvsVolumeExport,
        ) => run_opendtect_cbvs_bridge(request, object, &bridge_request, capability),
        _ => Err(LasError::Validation(format!(
            "Bridge kind `{:?}` is not implemented for vendor `{:?}`.",
            bridge_request.bridge_kind, request.vendor
        ))),
    }
}

pub fn bridge_commit_vendor_project_object(
    request: &VendorProjectBridgeCommitRequest,
) -> Result<VendorProjectBridgeCommitResponse> {
    let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
        vendor: request.bridge_run.vendor,
        project_root: request.bridge_run.project_root.clone(),
        selected_vendor_object_ids: vec![request.bridge_run.vendor_object_id.clone()],
        target_project_root: request.target_project_root.clone(),
        target_survey_asset_id: request.target_survey_asset_id.clone(),
        binding: request.binding.clone(),
        coordinate_reference: request.coordinate_reference.clone(),
        runtime_probe: None,
    })?;
    let bridge = run_vendor_project_bridge(&request.bridge_run)?;

    if !request.dry_run && !Path::new(&bridge.output.path).exists() {
        return Err(LasError::Validation(format!(
            "Bridge output path `{}` does not exist after bridge preparation. Execute the bridge or supply an existing output path before non-dry-run commit.",
            bridge.output.path
        )));
    }

    let commit = commit_vendor_project_import(&VendorProjectCommitRequest {
        plan,
        target_project_root: request.target_project_root.clone(),
        binding: request.binding.clone(),
        target_survey_asset_id: request.target_survey_asset_id.clone(),
        coordinate_reference: request.coordinate_reference.clone(),
        bridge_outputs: vec![bridge.output.clone()],
        dry_run: request.dry_run,
    })?;

    Ok(VendorProjectBridgeCommitResponse { bridge, commit })
}

fn scan_opendtect_project(request: &VendorProjectScanRequest) -> Result<VendorProjectScanResponse> {
    let project_root = Path::new(&request.project_root);
    if !project_root.is_dir() {
        return Err(LasError::Validation(format!(
            "Vendor project root `{}` does not exist or is not a directory.",
            request.project_root
        )));
    }

    let mut issues = Vec::new();
    let survey_path = project_root.join(".survey");
    let survey_metadata = if survey_path.is_file() {
        parse_opendtect_survey(&survey_path)?
    } else {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("missing_survey_file"),
            message: String::from("Vendor project does not contain a `.survey` file."),
            source_path: None,
            vendor_object_id: None,
        });
        VendorProjectSurveyMetadata::default()
    };

    let mut objects = Vec::new();
    scan_opendtect_rawdata(project_root, &mut objects)?;
    scan_opendtect_seismics(project_root, &mut objects)?;
    scan_opendtect_surfaces(project_root, &mut objects)?;
    scan_opendtect_wells(project_root, &mut objects)?;
    scan_opendtect_locations(project_root, &mut objects)?;
    scan_opendtect_shapefiles(project_root, &mut objects)?;
    objects.sort_by(|left, right| left.vendor_object_id.cmp(&right.vendor_object_id));

    Ok(VendorProjectScanResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor: request.vendor,
        project_root: request.project_root.clone(),
        vendor_project: survey_metadata.name.clone(),
        survey_metadata,
        objects,
        issues,
    })
}

fn scan_petrel_project(request: &VendorProjectScanRequest) -> Result<VendorProjectScanResponse> {
    let project_root = Path::new(&request.project_root);
    if !project_root.is_dir() {
        return Err(LasError::Validation(format!(
            "Vendor project root `{}` does not exist or is not a directory.",
            request.project_root
        )));
    }

    let mut issues = Vec::new();
    let mut survey_metadata = VendorProjectSurveyMetadata {
        name: project_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(String::from),
        notes: vec![String::from(
            "Petrel export-bundle discovery is file-system driven; native Petrel project internals and Ocean runtime workflows are out of scope for this phase.",
        )],
        ..VendorProjectSurveyMetadata::default()
    };
    let wells_dir = project_root.join("Wells");
    let wellheader_path = wells_dir.join("Wellheader");
    let wellheaders = if wellheader_path.is_file() {
        parse_petrel_wellheader(&wellheader_path)?
    } else {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Warning,
            code: String::from("petrel_wellheader_missing"),
            message: String::from(
                "Petrel export bundle does not contain `Wells/Wellheader`; trajectory identity and CRS context may be weaker.",
            ),
            source_path: Some(path_string(&wells_dir)),
            vendor_object_id: None,
        });
        BTreeMap::new()
    };

    if let Some((reference_name, source_path)) = infer_petrel_coordinate_reference(project_root)? {
        survey_metadata.coordinate_reference = Some(CoordinateReferenceDescriptor {
            id: None,
            name: Some(reference_name),
            geodetic_datum: None,
            unit: None,
        });
        survey_metadata.coordinate_reference_source_path = Some(source_path);
        survey_metadata.notes.push(String::from(
            "Coordinate reference was inferred from an exported Petrel text artifact and should be confirmed before canonical geometry commit.",
        ));
    } else {
        survey_metadata.notes.push(String::from(
            "No explicit coordinate reference was inferred from the Petrel export bundle.",
        ));
    }

    let mut objects = Vec::new();
    let well_files = read_dir_sorted(&wells_dir)?;
    for path in &well_files {
        if extension_eq(path, "dev") {
            let stem = file_stem_string(path)?;
            let mut source_paths = vec![path_string(path)];
            let mut notes = vec![String::from(
                "Petrel well trace export maps to canonical trajectory rows via a Petrel-specific parser.",
            )];
            if let Some(header) = wellheaders.get(&stem) {
                source_paths.push(path_string(&wellheader_path));
                notes.push(format!(
                    "Matched `Wells/Wellheader` entry for `{}` at x={}, y={}.",
                    header.well_name, header.x, header.y
                ));
            } else {
                issues.push(VendorProjectImportIssue {
                    severity: VendorProjectImportIssueSeverity::Warning,
                    code: String::from("petrel_wellheader_entry_missing"),
                    message: format!(
                        "Trajectory export `{}` does not have a matching `Wellheader` entry; filename stem will be used as the well key.",
                        path.file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or_default()
                    ),
                    source_path: Some(path_string(path)),
                    vendor_object_id: Some(format!(
                        "petrel-trajectory:{}",
                        sanitize_vendor_object_key(&stem)
                    )),
                });
            }
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!(
                    "petrel-trajectory:{}",
                    sanitize_vendor_object_key(&stem)
                ),
                vendor_kind: VendorProjectObjectKind::Well,
                display_name: format!("{stem} trajectory"),
                source_paths,
                canonical_target_kind: VendorProjectCanonicalTargetKind::Trajectory,
                disposition: VendorProjectImportDisposition::Canonical,
                requires_crs_decision: true,
                default_selected: true,
                notes,
            });
        } else if extension_eq(path, "las") {
            let stem = file_stem_string(path)?;
            let mut notes = vec![String::from(
                "LAS export can flow through the existing canonical log import path.",
            )];
            if !wellheaders.contains_key(&stem) {
                notes.push(String::from(
                    "No matching `Wellheader` entry was found; filename stem will anchor the well identity unless later binding overrides it.",
                ));
            }
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("petrel-log:{}", sanitize_vendor_object_key(&stem)),
                vendor_kind: VendorProjectObjectKind::WellLog,
                display_name: format!("{stem} logs"),
                source_paths: vec![path_string(path)],
                canonical_target_kind: VendorProjectCanonicalTargetKind::Log,
                disposition: VendorProjectImportDisposition::Canonical,
                requires_crs_decision: false,
                default_selected: true,
                notes,
            });
        }
    }

    let tops_candidates = [
        project_root.join("well tops.txt"),
        project_root.join("well_tops_type.prn"),
    ];
    let preferred_tops_path = tops_candidates.iter().find(|path| path.is_file()).cloned();
    if let Some(preferred_tops_path) = preferred_tops_path {
        let companion_top_paths = tops_candidates
            .iter()
            .filter(|path| path.is_file())
            .map(|path| path_string(path))
            .collect::<Vec<_>>();
        if companion_top_paths.len() > 1 {
            issues.push(VendorProjectImportIssue {
                severity: VendorProjectImportIssueSeverity::Info,
                code: String::from("petrel_duplicate_tops_exports"),
                message: String::from(
                    "Multiple Petrel tops exports were detected; the first preferred text export is used as the grouping source and the companion export is retained as provenance.",
                ),
                source_path: Some(path_string(&preferred_tops_path)),
                vendor_object_id: None,
            });
        }
        for (well_name, row_count) in parse_petrel_grouped_well_counts(&preferred_tops_path, 4, 0)?
        {
            let mut notes = vec![format!(
                "Petrel tops export contributes {row_count} rows for `{well_name}` and maps to a canonical TopSet."
            )];
            if companion_top_paths.len() > 1 {
                notes.push(String::from(
                    "A companion Petrel tops export is also present and will remain attached as provenance.",
                ));
            }
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("petrel-tops:{}", sanitize_vendor_object_key(&well_name)),
                vendor_kind: VendorProjectObjectKind::TopSet,
                display_name: format!("{well_name} tops"),
                source_paths: companion_top_paths.clone(),
                canonical_target_kind: VendorProjectCanonicalTargetKind::TopSet,
                disposition: VendorProjectImportDisposition::Canonical,
                requires_crs_decision: false,
                default_selected: true,
                notes,
            });
        }
    }

    let checkshot_path = project_root.join("CheckShots1.txt");
    if checkshot_path.is_file() {
        for (well_name, row_count) in parse_petrel_grouped_well_counts(&checkshot_path, 3, 2)? {
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!(
                    "petrel-checkshot:{}",
                    sanitize_vendor_object_key(&well_name)
                ),
                vendor_kind: VendorProjectObjectKind::CheckshotVspObservationSet,
                display_name: format!("{well_name} checkshots"),
                source_paths: vec![path_string(&checkshot_path)],
                canonical_target_kind:
                    VendorProjectCanonicalTargetKind::CheckshotVspObservationSet,
                disposition: VendorProjectImportDisposition::CanonicalWithLoss,
                requires_crs_decision: false,
                default_selected: true,
                notes: vec![
                    format!(
                        "Petrel checkshot export contributes {row_count} samples for `{well_name}`."
                    ),
                    String::from(
                        "Depth and travel-time sign conventions should be confirmed explicitly before canonical commit.",
                    ),
                ],
            });
        }
    }

    let horizon_dir = project_root.join("Seismic Interpretation (time)");
    for path in read_dir_sorted(&horizon_dir)? {
        if !path.is_file() {
            continue;
        }
        let stem = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(String::from)
            .unwrap_or_else(|| String::from("petrel_horizon"));
        let sample_count = petrel_non_empty_row_count(&path)?;
        objects.push(VendorProjectObjectPreview {
            vendor_object_id: format!(
                "petrel-horizon-points:{}",
                sanitize_vendor_object_key(&stem)
            ),
            vendor_kind: VendorProjectObjectKind::SeismicHorizon,
            display_name: stem.clone(),
            source_paths: vec![path_string(&path)],
            canonical_target_kind: VendorProjectCanonicalTargetKind::SurveyStoreHorizon,
            disposition: VendorProjectImportDisposition::CanonicalWithLoss,
            requires_crs_decision: false,
            default_selected: false,
            notes: vec![
                format!(
                    "Petrel horizon point export preview detected {sample_count} non-empty rows."
                ),
                String::from(
                    "Canonical import can append these points into an existing survey-owned horizon store when `targetSurveyAssetId` is supplied at commit time.",
                ),
            ],
        });
    }

    for path in read_dir_sorted(project_root)? {
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if matches!(
            name,
            "CheckShots1.txt" | "well tops.txt" | "well_tops_type.prn" | ".DS_Store"
        ) {
            continue;
        }
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Info,
            code: String::from("petrel_export_unclassified"),
            message: format!(
                "Petrel export file `{name}` is present but not yet classified into a canonical import family."
            ),
            source_path: Some(path_string(&path)),
            vendor_object_id: None,
        });
    }

    objects.sort_by(|left, right| left.vendor_object_id.cmp(&right.vendor_object_id));

    Ok(VendorProjectScanResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor: request.vendor,
        project_root: request.project_root.clone(),
        vendor_project: survey_metadata.name.clone(),
        survey_metadata,
        objects,
        issues,
    })
}

#[derive(Debug, Clone)]
struct PetrelWellHeaderRecord {
    well_name: String,
    x: f64,
    y: f64,
}

fn parse_petrel_wellheader(path: &Path) -> Result<BTreeMap<String, PetrelWellHeaderRecord>> {
    let text = fs::read_to_string(path)?;
    let mut records = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            continue;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 3 {
            continue;
        }
        let Some(x) = fields.get(1).and_then(|value| value.parse::<f64>().ok()) else {
            continue;
        };
        let Some(y) = fields.get(2).and_then(|value| value.parse::<f64>().ok()) else {
            continue;
        };
        let well_name = fields[0].to_string();
        records.insert(
            well_name.clone(),
            PetrelWellHeaderRecord { well_name, x, y },
        );
    }
    Ok(records)
}

fn parse_petrel_grouped_well_counts(
    path: &Path,
    min_fields: usize,
    well_name_index: usize,
) -> Result<BTreeMap<String, usize>> {
    let text = fs::read_to_string(path)?;
    let mut counts = BTreeMap::new();
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < min_fields {
            continue;
        }
        let Some(well_name) = fields.get(well_name_index) else {
            continue;
        };
        *counts.entry((*well_name).to_string()).or_default() += 1;
    }
    Ok(counts)
}

fn parse_petrel_trajectory(path: &Path) -> Result<Vec<TrajectoryRow>> {
    let text = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    let mut in_table = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("MD") {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 9 {
            continue;
        }
        let measured_depth = parts[0].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel trajectory measured depth from `{trimmed}`: {error}"
            ))
        })?;
        let true_vertical_depth = parts[4].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel trajectory TVD from `{trimmed}`: {error}"
            ))
        })?;
        let easting_offset = parts[5].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel trajectory DX from `{trimmed}`: {error}"
            ))
        })?;
        let northing_offset = parts[6].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel trajectory DY from `{trimmed}`: {error}"
            ))
        })?;
        let azimuth_deg = parts[7].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel trajectory AZIM from `{trimmed}`: {error}"
            ))
        })?;
        let inclination_deg = parts[8].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel trajectory INCL from `{trimmed}`: {error}"
            ))
        })?;
        rows.push(TrajectoryRow {
            measured_depth,
            true_vertical_depth: Some(true_vertical_depth),
            true_vertical_depth_subsea: None,
            azimuth_deg: Some(azimuth_deg),
            inclination_deg: Some(inclination_deg),
            northing_offset: Some(northing_offset),
            easting_offset: Some(easting_offset),
        });
    }

    if rows.is_empty() {
        return Err(LasError::Parse(format!(
            "Petrel trajectory file `{}` did not contain any trajectory stations.",
            path_string(path)
        )));
    }

    Ok(rows)
}

fn parse_petrel_tops_rows_for_well(path: &Path, well_key: &str) -> Result<Vec<TopRow>> {
    let text = fs::read_to_string(path)?;
    let mut rows = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let fields = trimmed.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 4 || fields[0] != well_key {
            continue;
        }
        let top_depth = fields[2].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse Petrel top depth from `{trimmed}`: {error}"
            ))
        })?;
        rows.push(TopRow {
            name: fields[1].to_string(),
            top_depth,
            base_depth: None,
            source: Some(fields[3..].join(" ")),
            source_depth_reference: Some(String::from("petrel_export_depth_reference_unspecified")),
            depth_domain: None,
            depth_datum: None,
        });
    }

    if rows.is_empty() {
        return Err(LasError::Parse(format!(
            "Petrel tops file `{}` did not contain any rows for well `{well_key}`.",
            path_string(path)
        )));
    }

    Ok(rows)
}

fn parse_petrel_checkshot_set_for_well(
    path: &Path,
    well_key: &str,
) -> Result<CheckshotVspObservationSet1D> {
    let text = fs::read_to_string(path)?;
    let mut normalized_negative_time = false;
    let mut normalized_negative_depth = false;
    let mut samples = text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let fields = trimmed.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 3 || sanitize_vendor_object_key(fields[2]) != well_key {
                return None;
            }
            Some((fields[0].to_string(), fields[1].to_string()))
        })
        .enumerate()
        .map(|(index, (time_text, depth_text))| {
            let raw_time_ms = time_text.parse::<f64>().map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse Petrel checkshot time `{time_text}` in `{}`: {error}",
                    path_string(path)
                ))
            })?;
            let raw_depth_m = depth_text.parse::<f64>().map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse Petrel checkshot depth `{depth_text}` in `{}`: {error}",
                    path_string(path)
                ))
            })?;
            normalized_negative_time |= raw_time_ms < 0.0;
            normalized_negative_depth |= raw_depth_m < 0.0;
            Ok(WellTimeDepthObservationSample {
                depth_m: raw_depth_m.abs(),
                time_ms: raw_time_ms.abs(),
                quality: None,
                station_id: Some((index + 1).to_string()),
                note: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    if samples.is_empty() {
        return Err(LasError::Validation(format!(
            "No Petrel checkshot samples for well `{well_key}` were found in `{}`.",
            path_string(path)
        )));
    }

    samples.sort_by(|left, right| {
        left.depth_m
            .partial_cmp(&right.depth_m)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.time_ms
                    .partial_cmp(&right.time_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    samples.dedup_by(|left, right| {
        (left.depth_m - right.depth_m).abs() <= f64::EPSILON
            && (left.time_ms - right.time_ms).abs() <= f64::EPSILON
    });

    let mut notes = vec![String::from(
        "Imported from Petrel checkshot export as true-vertical-depth-subsea to two-way-time observations.",
    )];
    if normalized_negative_time || normalized_negative_depth {
        notes.push(String::from(
            "Normalized negative-downward Petrel export values to positive canonical depth/time samples.",
        ));
    }

    Ok(CheckshotVspObservationSet1D {
        id: format!("petrel-checkshot-{well_key}"),
        name: format!("{well_key} checkshots"),
        wellbore_id: None,
        depth_reference: DepthReferenceKind::TrueVerticalDepthSubsea,
        travel_time_reference: TravelTimeReference::TwoWay,
        samples,
        notes,
    })
}

fn petrel_non_empty_row_count(path: &Path) -> Result<usize> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count())
}

fn infer_petrel_coordinate_reference(project_root: &Path) -> Result<Option<(String, String)>> {
    for path in read_dir_sorted(project_root)? {
        if !path.is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let trimmed = line.trim();
            let Some(value) = trimmed.strip_prefix("!     COORDINATE REFERENCE SYSTEM:") else {
                continue;
            };
            if let Some(reference_name) = non_empty_string(value) {
                return Ok(Some((reference_name, path_string(&path))));
            }
        }
    }
    Ok(None)
}

fn petrel_plan_blocking_issues(
    selected_ids: &[String],
    object_map: &BTreeMap<String, &VendorProjectObjectPreview>,
    _effective_coordinate_reference: Option<&CoordinateReferenceDescriptor>,
) -> Vec<VendorProjectImportIssue> {
    let mut issues = Vec::new();
    let mut selected_well_keys = BTreeSet::new();

    for selected_id in selected_ids {
        let Some(object) = object_map.get(selected_id) else {
            continue;
        };
        if let Some(well_key) = petrel_object_well_key(object) {
            selected_well_keys.insert(well_key);
        }
    }

    if selected_well_keys.len() > 1 {
        issues.push(VendorProjectImportIssue {
            severity: VendorProjectImportIssueSeverity::Blocking,
            code: String::from("petrel_multi_well_selection_unsupported"),
            message: String::from(
                "Phase-one Petrel commit supports one well family per request. Split the selection by well before non-dry-run commit.",
            ),
            source_path: None,
            vendor_object_id: None,
        });
    }

    issues
}

fn petrel_object_well_key(object: &VendorProjectObjectPreview) -> Option<String> {
    for prefix in [
        "petrel-trajectory:",
        "petrel-log:",
        "petrel-tops:",
        "petrel-checkshot:",
    ] {
        if let Some(value) = object.vendor_object_id.strip_prefix(prefix) {
            return Some(value.to_string());
        }
    }
    None
}

fn parse_opendtect_survey(survey_path: &Path) -> Result<VendorProjectSurveyMetadata> {
    let text = fs::read_to_string(survey_path)?;
    let mut metadata = VendorProjectSurveyMetadata::default();
    let mut coordinate_reference_id = None;
    let mut coordinate_reference_name = None;

    for line in text.lines() {
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let key = raw_key.trim();
        let value = raw_value.trim();
        match key {
            "Name" => metadata.name = non_empty_string(value),
            "Survey Data Type" => metadata.survey_data_type = non_empty_string(value),
            "In-line range" => metadata.inline_range = parse_i32_triplet(value),
            "Cross-line range" => metadata.crossline_range = parse_i32_triplet(value),
            "Z range" => {
                let parts = split_backtick_parts(value);
                if parts.len() >= 3 {
                    let parsed = [
                        parts[0].parse::<f64>().ok(),
                        parts[1].parse::<f64>().ok(),
                        parts[2].parse::<f64>().ok(),
                    ];
                    if let [Some(start), Some(stop), Some(step)] = parsed {
                        metadata.z_range = Some([start, stop, step]);
                    }
                }
                if parts.len() >= 4 {
                    metadata.z_domain = non_empty_string(parts[3]);
                }
            }
            "Coordinate System.Projection.ID" => {
                let parts = split_backtick_parts(value);
                if parts.len() >= 2 {
                    coordinate_reference_id = Some(format!("{}:{}", parts[0], parts[1]));
                } else {
                    coordinate_reference_id = non_empty_string(value);
                }
            }
            "Coordinate System.Projection.Name" => {
                coordinate_reference_name = non_empty_string(value);
            }
            _ => {}
        }
    }

    if coordinate_reference_id.is_some() || coordinate_reference_name.is_some() {
        metadata.coordinate_reference = Some(CoordinateReferenceDescriptor {
            id: coordinate_reference_id,
            name: coordinate_reference_name,
            geodetic_datum: None,
            unit: None,
        });
        metadata.coordinate_reference_source_path = Some(path_string(survey_path));
    } else {
        metadata.notes.push(String::from(
            "No explicit coordinate reference was detected in the OpendTect survey file.",
        ));
    }

    Ok(metadata)
}

fn scan_opendtect_rawdata(
    project_root: &Path,
    objects: &mut Vec<VendorProjectObjectPreview>,
) -> Result<()> {
    let rawdata_dir = project_root.join("Rawdata");
    let segy_path = rawdata_dir.join("Seismic_data.sgy");
    if segy_path.is_file() {
        objects.push(VendorProjectObjectPreview {
            vendor_object_id: String::from("seismic-segy:Seismic_data"),
            vendor_kind: VendorProjectObjectKind::SeismicVolume,
            display_name: String::from("Seismic_data"),
            source_paths: vec![path_string(&segy_path)],
            canonical_target_kind: VendorProjectCanonicalTargetKind::SeismicTraceData,
            disposition: VendorProjectImportDisposition::Canonical,
            requires_crs_decision: false,
            default_selected: true,
            notes: vec![String::from(
                "Prefer the direct SEG-Y companion for canonical seismic trace-data import.",
            )],
        });
    }

    let velocity_path = rawdata_dir.join("Velocity_functions.txt");
    if velocity_path.is_file() {
        objects.push(VendorProjectObjectPreview {
            vendor_object_id: String::from("velocity-function:Velocity_functions"),
            vendor_kind: VendorProjectObjectKind::VelocityFunction,
            display_name: String::from("Velocity_functions"),
            source_paths: vec![path_string(&velocity_path)],
            canonical_target_kind: VendorProjectCanonicalTargetKind::ExternalOpenFormat,
            disposition: VendorProjectImportDisposition::CanonicalWithLoss,
            requires_crs_decision: false,
            default_selected: false,
            notes: vec![String::from(
                "Velocity text can flow through an open-text adapter before canonical time-depth work.",
            )],
        });
    }

    Ok(())
}

fn scan_opendtect_seismics(
    project_root: &Path,
    objects: &mut Vec<VendorProjectObjectPreview>,
) -> Result<()> {
    let seismic_dir = project_root.join("Seismics");
    for path in read_dir_sorted(&seismic_dir)? {
        if !extension_eq(&path, "cbvs") {
            continue;
        }
        let stem = file_stem_string(&path)?;
        objects.push(VendorProjectObjectPreview {
            vendor_object_id: format!("seismic-cbvs:{}", sanitize_vendor_object_key(&stem)),
            vendor_kind: VendorProjectObjectKind::SeismicVolume,
            display_name: stem.clone(),
            source_paths: collect_primary_with_companions(&path, &["par", "proc"]),
            canonical_target_kind: VendorProjectCanonicalTargetKind::SeismicTraceData,
            disposition: VendorProjectImportDisposition::CanonicalWithLoss,
            requires_crs_decision: true,
            default_selected: stem == "7a_AI_Cube_Std",
            notes: vec![String::from(
                "Requires an OpendTect-native bridge for CBVS payloads before canonical trace-data import.",
            )],
        });
    }
    Ok(())
}

fn scan_opendtect_surfaces(
    project_root: &Path,
    objects: &mut Vec<VendorProjectObjectPreview>,
) -> Result<()> {
    let surface_dir = project_root.join("Surfaces");

    for path in read_dir_sorted(&surface_dir)? {
        if extension_eq(&path, "hor") {
            let stem = file_stem_string(&path)?;
            let default_selected = matches!(stem.as_str(), "Demo_0_--__FS4" | "Demo_1_--__MFS4");
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("horizon-hor:{}", sanitize_vendor_object_key(&stem)),
                vendor_kind: VendorProjectObjectKind::SeismicHorizon,
                display_name: stem,
                source_paths: collect_primary_with_companions(&path, &["par", "ts"]),
                canonical_target_kind: VendorProjectCanonicalTargetKind::SurveyStoreHorizon,
                disposition: VendorProjectImportDisposition::CanonicalWithLoss,
                requires_crs_decision: true,
                default_selected,
                notes: vec![String::from(
                    "Canonical import targets the survey horizon store and may drop vendor display state.",
                )],
            });
        } else if extension_eq(&path, "flt") {
            let stem = file_stem_string(&path)?;
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("fault:{}", sanitize_vendor_object_key(&stem)),
                vendor_kind: VendorProjectObjectKind::Fault,
                display_name: stem.clone(),
                source_paths: collect_primary_with_companions(&path, &["par"]),
                canonical_target_kind: VendorProjectCanonicalTargetKind::RawSourceBundle,
                disposition: VendorProjectImportDisposition::RawSourceOnly,
                requires_crs_decision: false,
                default_selected: stem == "Fault_A",
                notes: vec![String::from(
                    "Fault surfaces are preserved as raw source bundles until a richer canonical family exists.",
                )],
            });
        } else if extension_eq(&path, "body") {
            let stem = file_stem_string(&path)?;
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("body:{}", sanitize_vendor_object_key(&stem)),
                vendor_kind: VendorProjectObjectKind::Body,
                display_name: stem,
                source_paths: collect_primary_with_companions(&path, &["par"]),
                canonical_target_kind: VendorProjectCanonicalTargetKind::RawSourceBundle,
                disposition: VendorProjectImportDisposition::RawSourceOnly,
                requires_crs_decision: false,
                default_selected: false,
                notes: vec![String::from(
                    "OpendTect bodies remain preserved raw sources in phase one.",
                )],
            });
        }
    }

    let mut horizon_sets = BTreeMap::<String, Vec<PathBuf>>::new();
    for path in read_dir_sorted(&surface_dir)? {
        if extension_eq(&path, "hcs") || extension_eq(&path, "hci") {
            let stem = file_stem_string(&path)?;
            horizon_sets.entry(stem).or_default().push(path);
        }
    }
    for (stem, mut paths) in horizon_sets {
        let par = surface_dir.join(format!("{stem}.par"));
        if par.is_file() {
            paths.push(par);
        }
        let sti = surface_dir.join(format!("{stem}.sti"));
        if sti.is_file() {
            paths.push(sti);
        }
        paths.sort();
        objects.push(VendorProjectObjectPreview {
            vendor_object_id: format!("horizon-set:{}", sanitize_vendor_object_key(&stem)),
            vendor_kind: VendorProjectObjectKind::SeismicHorizon,
            display_name: stem,
            source_paths: paths.into_iter().map(|path| path_string(&path)).collect(),
            canonical_target_kind: VendorProjectCanonicalTargetKind::SurveyStoreHorizon,
            disposition: VendorProjectImportDisposition::CanonicalWithLoss,
            requires_crs_decision: true,
            default_selected: false,
            notes: vec![String::from(
                "HCS/HCI surface containers may flatten vendor-specific topology on canonical import.",
            )],
        });
    }

    Ok(())
}

fn scan_opendtect_wells(
    project_root: &Path,
    objects: &mut Vec<VendorProjectObjectPreview>,
) -> Result<()> {
    let well_dir = project_root.join("WellInfo");
    let mut well_groups = BTreeMap::<String, Vec<PathBuf>>::new();
    for path in read_dir_sorted(&well_dir)? {
        let Some(stem) = vendor_group_stem(&path) else {
            continue;
        };
        well_groups.entry(stem).or_default().push(path);
    }

    for (stem, mut paths) in well_groups {
        paths.sort();
        let default_selected = stem == "F03-2";

        let well_path = paths
            .iter()
            .find(|path| extension_eq(path, "well"))
            .cloned();
        if let Some(path) = well_path {
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("well:{stem}"),
                vendor_kind: VendorProjectObjectKind::Well,
                display_name: stem.clone(),
                source_paths: vec![path_string(&path)],
                canonical_target_kind: VendorProjectCanonicalTargetKind::Trajectory,
                disposition: VendorProjectImportDisposition::CanonicalWithLoss,
                requires_crs_decision: true,
                default_selected,
                notes: vec![String::from(
                    "Trajectory import will need an OpendTect well parser for vendor-native geometry payloads.",
                )],
            });
        }

        let log_paths = paths
            .iter()
            .filter(|path| extension_eq(path, "wll"))
            .map(|path| path_string(path))
            .collect::<Vec<_>>();
        if !log_paths.is_empty() {
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("well-logs:{stem}"),
                vendor_kind: VendorProjectObjectKind::WellLog,
                display_name: format!("{stem} logs"),
                source_paths: log_paths,
                canonical_target_kind: VendorProjectCanonicalTargetKind::Log,
                disposition: VendorProjectImportDisposition::CanonicalWithLoss,
                requires_crs_decision: false,
                default_selected,
                notes: vec![String::from(
                    "Well-log import can map vendor WLL curves into canonical log assets.",
                )],
            });
        }

        let marker_paths = paths
            .iter()
            .filter(|path| extension_eq(path, "wlm"))
            .map(|path| path_string(path))
            .collect::<Vec<_>>();
        if !marker_paths.is_empty() {
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("well-markers:{stem}"),
                vendor_kind: VendorProjectObjectKind::WellMarkerSet,
                display_name: format!("{stem} markers"),
                source_paths: marker_paths,
                canonical_target_kind: VendorProjectCanonicalTargetKind::WellMarkerSet,
                disposition: VendorProjectImportDisposition::CanonicalWithLoss,
                requires_crs_decision: true,
                default_selected,
                notes: vec![String::from(
                    "Marker rows depend on a coordinate-referenced well trajectory for spatial reconciliation.",
                )],
            });
        }

        let mut time_depth_paths = paths
            .iter()
            .filter(|path| {
                extension_eq(path, "wlt")
                    || extension_eq(path, "csmdl")
                    || extension_eq(path, "tie")
            })
            .map(|path| path_string(path))
            .collect::<Vec<_>>();
        time_depth_paths.sort();
        if !time_depth_paths.is_empty() {
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("well-time-depth:{stem}"),
                vendor_kind: VendorProjectObjectKind::WellTimeDepthModel,
                display_name: format!("{stem} time-depth"),
                source_paths: time_depth_paths,
                canonical_target_kind: VendorProjectCanonicalTargetKind::WellTimeDepthModel,
                disposition: VendorProjectImportDisposition::CanonicalWithLoss,
                requires_crs_decision: false,
                default_selected,
                notes: vec![String::from(
                    "WLT and CSMDL inputs map toward canonical well time-depth assets.",
                )],
            });
        }
    }

    Ok(())
}

fn scan_opendtect_locations(
    project_root: &Path,
    objects: &mut Vec<VendorProjectObjectPreview>,
) -> Result<()> {
    let location_dir = project_root.join("Locations");
    for path in read_dir_sorted(&location_dir)? {
        if extension_eq(&path, "pck") {
            let stem = file_stem_string(&path)?;
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("pick-set:{}", sanitize_vendor_object_key(&stem)),
                vendor_kind: VendorProjectObjectKind::PickSet,
                display_name: stem,
                source_paths: vec![path_string(&path)],
                canonical_target_kind: VendorProjectCanonicalTargetKind::RawSourceBundle,
                disposition: VendorProjectImportDisposition::RawSourceOnly,
                requires_crs_decision: false,
                default_selected: false,
                notes: vec![String::from(
                    "Pick sets are preserved raw until a canonical interpretation family is added.",
                )],
            });
        } else if extension_eq(&path, "rdl") {
            let stem = file_stem_string(&path)?;
            objects.push(VendorProjectObjectPreview {
                vendor_object_id: format!("random-line:{}", sanitize_vendor_object_key(&stem)),
                vendor_kind: VendorProjectObjectKind::RandomLine,
                display_name: stem,
                source_paths: vec![path_string(&path)],
                canonical_target_kind: VendorProjectCanonicalTargetKind::RawSourceBundle,
                disposition: VendorProjectImportDisposition::RawSourceOnly,
                requires_crs_decision: false,
                default_selected: false,
                notes: vec![String::from(
                    "Random-line definitions are preserved raw in phase one.",
                )],
            });
        }
    }
    Ok(())
}

fn scan_opendtect_shapefiles(
    project_root: &Path,
    objects: &mut Vec<VendorProjectObjectPreview>,
) -> Result<()> {
    let shape_dir = project_root.join("Shapefiles");
    for path in read_dir_sorted(&shape_dir)? {
        if !extension_eq(&path, "shp") {
            continue;
        }
        let stem = file_stem_string(&path)?;
        objects.push(VendorProjectObjectPreview {
            vendor_object_id: format!("shapefile:{}", sanitize_vendor_object_key(&stem)),
            vendor_kind: VendorProjectObjectKind::Shapefile,
            display_name: stem.clone(),
            source_paths: collect_primary_with_companions(&path, &["dbf", "shx", "prj", "disp"]),
            canonical_target_kind: VendorProjectCanonicalTargetKind::ExternalOpenFormat,
            disposition: VendorProjectImportDisposition::Canonical,
            requires_crs_decision: true,
            default_selected: false,
            notes: vec![String::from(
                "Shapefile import can route through an open geospatial bridge after CRS validation.",
            )],
        });
    }
    Ok(())
}

fn bridge_request_for_object(
    vendor: VendorProjectImportVendor,
    project_root: &Path,
    object: &VendorProjectObjectPreview,
) -> Result<Option<VendorProjectBridgeRequest>> {
    let Some(capability) = bridge_capability_for_object(vendor, object) else {
        return Ok(None);
    };
    let vendor_native_id =
        if capability.bridge_kind == VendorProjectBridgeKind::OpendtectCbvsVolumeExport {
            discover_opendtect_native_id(project_root, &object.display_name)?
                .map(|match_info| match_info.native_id)
        } else {
            None
        };
    Ok(Some(VendorProjectBridgeRequest {
        vendor_object_id: object.vendor_object_id.clone(),
        display_name: object.display_name.clone(),
        bridge_kind: capability.bridge_kind,
        vendor_native_id,
        recommended_output_format: capability.recommended_output_format,
        accepted_output_formats: capability.accepted_output_formats.to_vec(),
        automatic_execution_formats: capability.automatic_execution_formats.to_vec(),
        runtime_requirements: capability.runtime_requirements.to_vec(),
        source_paths: object.source_paths.clone(),
        notes: capability
            .notes
            .iter()
            .map(|note| String::from(*note))
            .collect(),
    }))
}

fn bridge_capability_definitions(
    vendor: VendorProjectImportVendor,
) -> &'static [VendorProjectBridgeCapabilityDefinition] {
    match vendor {
        VendorProjectImportVendor::Opendtect => OPENDTECT_BRIDGE_CAPABILITIES,
        VendorProjectImportVendor::Petrel => PETREL_BRIDGE_CAPABILITIES,
    }
}

fn connector_phase_definitions(
    vendor: VendorProjectImportVendor,
) -> &'static [VendorProjectConnectorPhaseDefinition] {
    match vendor {
        VendorProjectImportVendor::Opendtect => OPENDTECT_CONNECTOR_PHASES,
        VendorProjectImportVendor::Petrel => PETREL_CONNECTOR_PHASES,
    }
}

fn supported_runtime_kinds(
    vendor: VendorProjectImportVendor,
) -> &'static [VendorProjectRuntimeKind] {
    match vendor {
        VendorProjectImportVendor::Opendtect => OPENDTECT_SUPPORTED_RUNTIME_KINDS,
        VendorProjectImportVendor::Petrel => PETREL_SUPPORTED_RUNTIME_KINDS,
    }
}

fn connector_provenance_guarantees(
    vendor: VendorProjectImportVendor,
) -> &'static [VendorProjectConnectorProvenanceGuarantee] {
    match vendor {
        VendorProjectImportVendor::Opendtect => OPENDTECT_PROVENANCE_GUARANTEES,
        VendorProjectImportVendor::Petrel => PETREL_PROVENANCE_GUARANTEES,
    }
}

fn connector_notes(vendor: VendorProjectImportVendor) -> &'static [&'static str] {
    match vendor {
        VendorProjectImportVendor::Opendtect => OPENDTECT_CONNECTOR_NOTES,
        VendorProjectImportVendor::Petrel => PETREL_CONNECTOR_NOTES,
    }
}

fn bridge_capability_for_object(
    vendor: VendorProjectImportVendor,
    object: &VendorProjectObjectPreview,
) -> Option<&'static VendorProjectBridgeCapabilityDefinition> {
    bridge_capability_definitions(vendor)
        .iter()
        .find(|capability| {
            capability
                .supported_vendor_object_prefixes
                .iter()
                .any(|prefix| object.vendor_object_id.starts_with(prefix))
        })
}

fn public_bridge_capability(
    capability: &VendorProjectBridgeCapabilityDefinition,
) -> VendorProjectBridgeCapability {
    VendorProjectBridgeCapability {
        bridge_kind: capability.bridge_kind,
        supported_vendor_object_prefixes: capability
            .supported_vendor_object_prefixes
            .iter()
            .map(|prefix| String::from(*prefix))
            .collect(),
        recommended_output_format: capability.recommended_output_format,
        accepted_output_formats: capability.accepted_output_formats.to_vec(),
        automatic_execution_formats: capability.automatic_execution_formats.to_vec(),
        runtime_requirements: capability.runtime_requirements.to_vec(),
        notes: capability
            .notes
            .iter()
            .map(|note| String::from(*note))
            .collect(),
    }
}

fn public_connector_phase(
    phase: &VendorProjectConnectorPhaseDefinition,
) -> VendorProjectConnectorPhaseSupport {
    VendorProjectConnectorPhaseSupport {
        phase: phase.phase,
        isolation_boundary: phase.isolation_boundary,
        notes: phase.notes.iter().map(|note| (*note).to_string()).collect(),
    }
}

fn run_opendtect_cbvs_bridge(
    request: &VendorProjectBridgeRunRequest,
    object: &VendorProjectObjectPreview,
    bridge_request: &VendorProjectBridgeRequest,
    capability: &VendorProjectBridgeCapabilityDefinition,
) -> Result<VendorProjectBridgeRunResponse> {
    if !capability
        .accepted_output_formats
        .contains(&request.output_format)
    {
        return Err(LasError::Validation(format!(
            "Bridge output format `{:?}` is not supported for bridge kind `{:?}`.",
            request.output_format, capability.bridge_kind
        )));
    }
    if request.execute
        && !capability
            .automatic_execution_formats
            .contains(&request.output_format)
    {
        return Err(LasError::Validation(format!(
            "Automatic execution for bridge kind `{:?}` does not support output format `{:?}`.",
            capability.bridge_kind, request.output_format
        )));
    }

    let project_root = Path::new(&request.project_root);
    let output_path = PathBuf::from(&request.output_path);
    if output_path.exists() && !request.overwrite_existing_output {
        return Err(LasError::Validation(format!(
            "Bridge output path `{}` already exists. Set `overwriteExistingOutput` to true to reuse it.",
            request.output_path
        )));
    }

    let native_id_match = discover_opendtect_native_id(project_root, &object.display_name)?
        .ok_or_else(|| {
            LasError::Validation(format!(
                "Could not discover an OpendTect native storage id for `{}`. A matching `Proc/*.par` with `Input.ID` or `Attribs/*.attr` with `Storage id=` is required for automatic bridge preparation.",
                object.display_name
            ))
        })?;

    let parameter_file_path = request
        .parameter_file_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| output_path.with_extension("opendtect-export.par"));
    let log_path = request
        .log_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| output_path.with_extension("opendtect-export.log"));

    if let Some(parent) = parameter_file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let parameter_text = render_opendtect_cbvs_export_parameter_file(
        project_root,
        &native_id_match.native_id,
        &output_path,
        &log_path,
    )?;
    fs::write(&parameter_file_path, parameter_text)?;

    let executable = resolve_opendtect_batch_executable(request)?;
    let command = vec![
        executable.command.clone(),
        parameter_file_path.to_string_lossy().into_owned(),
    ];

    let mut notes = bridge_request.notes.clone();
    notes.push(format!(
        "Prepared OpendTect SEG-Y export parameter file using native storage id `{}` discovered from `{}`.",
        native_id_match.native_id, native_id_match.source_path
    ));
    if let Some(source_note) = executable.source_note.as_ref() {
        notes.push(source_note.clone());
    }
    if !request.execute {
        notes.push(String::from(
            "Execution was not requested; the output path is prepared but not materialized yet.",
        ));
    }

    let execution_status = if request.execute {
        let status = Command::new(&executable.command)
            .arg(&parameter_file_path)
            .status()
            .map_err(|error| {
                LasError::Validation(format!(
                    "Failed to launch OpendTect bridge executable `{}`: {}",
                    executable.command, error
                ))
            })?;
        if !status.success() {
            return Err(LasError::Validation(format!(
                "OpendTect bridge executable `{}` exited with status `{}`.",
                executable.command, status
            )));
        }
        notes.push(String::from(
            "Executed the generated OpendTect batch export command successfully.",
        ));
        VendorProjectBridgeExecutionStatus::Executed
    } else {
        VendorProjectBridgeExecutionStatus::Prepared
    };

    let output_path_string = output_path.to_string_lossy().into_owned();
    let parameter_file_path_string = parameter_file_path.to_string_lossy().into_owned();
    let log_path_string = log_path.to_string_lossy().into_owned();
    let artifacts = bridge_artifacts(
        &parameter_file_path_string,
        Some(&log_path_string),
        &output_path_string,
    );

    Ok(VendorProjectBridgeRunResponse {
        schema_version: VENDOR_PROJECT_IMPORT_SCHEMA_VERSION,
        vendor: request.vendor,
        project_root: request.project_root.clone(),
        vendor_object_id: object.vendor_object_id.clone(),
        display_name: object.display_name.clone(),
        bridge_kind: bridge_request.bridge_kind,
        vendor_native_id: Some(native_id_match.native_id.clone()),
        output: VendorProjectBridgeOutput {
            vendor_object_id: object.vendor_object_id.clone(),
            format: request.output_format,
            path: output_path_string,
            coordinate_reference: None,
            notes: vec![format!(
                "Prepared by the OpendTect CBVS bridge from native storage id `{}`.",
                native_id_match.native_id
            )],
        },
        parameter_file_path: parameter_file_path_string,
        log_path: Some(log_path_string),
        artifacts,
        command,
        execution_status,
        notes,
        issues: Vec::new(),
    })
}

fn bridge_artifacts(
    parameter_file_path: &str,
    log_path: Option<&str>,
    output_path: &str,
) -> Vec<VendorProjectBridgeArtifact> {
    let mut artifacts = vec![
        bridge_artifact(
            VendorProjectBridgeArtifactKind::ParameterFile,
            parameter_file_path,
        ),
        bridge_artifact(VendorProjectBridgeArtifactKind::BridgeOutput, output_path),
    ];

    if let Some(log_path) = log_path {
        artifacts.push(bridge_artifact(
            VendorProjectBridgeArtifactKind::LogFile,
            log_path,
        ));
    }

    artifacts
}

fn bridge_artifact(
    kind: VendorProjectBridgeArtifactKind,
    path: &str,
) -> VendorProjectBridgeArtifact {
    let metadata = fs::metadata(path).ok();
    VendorProjectBridgeArtifact {
        kind,
        path: String::from(path),
        exists: metadata.is_some(),
        size_bytes: metadata.map(|value| value.len()),
    }
}

fn commit_support_checks(
    vendor: VendorProjectImportVendor,
    object: &VendorProjectObjectPreview,
    bridge_output: Option<&VendorProjectBridgeOutput>,
    target_survey_asset_id: Option<&str>,
) -> Vec<String> {
    if object.disposition == VendorProjectImportDisposition::RawSourceOnly {
        return vec![String::from(
            "raw_source_bundle_commit_supported: selected object can be preserved as a raw source bundle",
        )];
    }
    if bridge_capability_for_object(vendor, object).is_some() {
        let capability = bridge_capability_for_object(vendor, object)
            .expect("capability existence checked above");
        return match validate_bridge_output_for_object(vendor, object, bridge_output) {
            Ok(Some(output)) => vec![format!(
                "bridge_output_supported:{:?}: supplied {:?} bridge output at `{}` can be imported canonically",
                capability.bridge_kind, output.format, output.path
            )],
            Ok(None) => vec![format!(
                "unsupported_for_non_dry_run: selected object requires bridge kind `{:?}` output before canonical commit (recommended: {:?})",
                capability.bridge_kind, capability.recommended_output_format
            )],
            Err(error) => vec![format!("unsupported_for_non_dry_run: {error}")],
        };
    }
    if object.vendor_object_id.starts_with("well:")
        || object.vendor_object_id.starts_with("seismic-segy:")
        || object.vendor_object_id.starts_with("well-logs:")
        || object.vendor_object_id.starts_with("well-markers:")
        || object.vendor_object_id.starts_with("well-time-depth:")
    {
        return vec![String::from(
            "opendtect_well_family_commit_supported: selected object can be imported canonically in phase one",
        )];
    }
    if vendor == VendorProjectImportVendor::Petrel
        && (object.vendor_object_id.starts_with("petrel-trajectory:")
            || object.vendor_object_id.starts_with("petrel-log:")
            || object.vendor_object_id.starts_with("petrel-tops:")
            || object.vendor_object_id.starts_with("petrel-checkshot:"))
    {
        return vec![String::from(
            "petrel_export_bundle_commit_supported: selected object can be imported canonically in phase one",
        )];
    }
    if vendor == VendorProjectImportVendor::Petrel
        && object
            .vendor_object_id
            .starts_with("petrel-horizon-points:")
    {
        return if let Some(target_survey_asset_id) = target_survey_asset_id {
            vec![format!(
                "petrel_horizon_commit_supported: selected horizon points can be imported into survey asset `{target_survey_asset_id}`"
            )]
        } else {
            vec![String::from(
                "unsupported_for_non_dry_run: selected Petrel horizon points require `targetSurveyAssetId` for canonical survey-horizon commit",
            )]
        };
    }
    vec![String::from(
        "unsupported_for_non_dry_run: selected object is not yet implemented for canonical vendor commit",
    )]
}

fn commit_petrel_object(
    project: &mut OphioliteProject,
    object: &VendorProjectObjectPreview,
    binding: Option<&AssetBindingInput>,
    target_survey_asset_id: Option<&AssetId>,
    effective_coordinate_reference: Option<&CoordinateReferenceDescriptor>,
    imported_assets: &mut Vec<VendorProjectCommittedAsset>,
    preserved_raw_sources: &mut Vec<VendorProjectCommittedAsset>,
    validation_reports: &mut Vec<VendorProjectValidationReport>,
) -> Result<()> {
    if object.disposition == VendorProjectImportDisposition::RawSourceOnly {
        let source_paths = object
            .source_paths
            .iter()
            .map(Path::new)
            .collect::<Vec<_>>();
        let result = match binding {
            Some(binding) => project.import_raw_source_bundle_with_binding(
                &source_paths,
                binding,
                Some(&object.display_name),
            )?,
            None => project.import_raw_source_bundle_into_project_archive(
                &source_paths,
                Some(&object.display_name),
            )?,
        };
        preserved_raw_sources.push(committed_asset_from_project_result(
            object,
            result,
            object.source_paths.clone(),
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!(
                "preserved_raw_source_bundle: {} source files",
                object.source_paths.len()
            )],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object
        .vendor_object_id
        .starts_with("petrel-horizon-points:")
    {
        let target_survey_asset_id = target_survey_asset_id.ok_or_else(|| {
            LasError::Validation(format!(
                "Petrel horizon object `{}` requires `targetSurveyAssetId` for non-dry-run commit.",
                object.display_name
            ))
        })?;
        let source_paths = object
            .source_paths
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let normalized_source_paths = normalize_petrel_horizon_source_paths(&source_paths)?;
        let source_path_refs = normalized_source_paths.iter().collect::<Vec<_>>();
        let draft = build_suggested_horizon_source_import_draft(
            source_path_refs.as_slice(),
            TimeDepthDomain::Time,
            None,
            effective_coordinate_reference.and_then(|value| value.id.as_deref()),
            effective_coordinate_reference.and_then(|value| value.name.as_deref()),
            effective_coordinate_reference.is_none(),
        );
        let result = project.import_horizon_source_into_survey_asset(
            target_survey_asset_id,
            &draft,
            &source_paths,
        )?;
        imported_assets.push(committed_asset_from_existing_asset(
            object,
            &result.collection,
            &result.asset,
            object.source_paths.clone(),
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![
                format!(
                    "imported_survey_horizons: {}",
                    result.imported_horizons.len()
                ),
                format!("target_survey_asset_id: {}", target_survey_asset_id.0),
            ],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    let binding = binding.ok_or_else(|| {
        LasError::Validation(format!(
            "Vendor object `{}` requires a binding for non-dry-run commit.",
            object.display_name
        ))
    })?;

    if object.requires_crs_decision && effective_coordinate_reference.is_none() {
        return Err(LasError::Validation(format!(
            "Vendor object `{}` requires a coordinate reference for non-dry-run commit.",
            object.display_name
        )));
    }

    if object.vendor_object_id.starts_with("petrel-log:") {
        let las_path = Path::new(object.source_paths.first().ok_or_else(|| {
            LasError::Validation(format!(
                "Missing LAS source path for Petrel log object `{}`.",
                object.display_name
            ))
        })?);
        let result = project.import_las_with_binding_and_supporting_sources(
            las_path,
            binding,
            Some("logs"),
            &[],
        )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            vec![path_string(las_path)],
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![String::from(
                "imported_petrel_las_logs: canonical log assets created from the Petrel LAS export",
            )],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("petrel-trajectory:") {
        let trajectory_path = Path::new(object.source_paths.first().ok_or_else(|| {
            LasError::Validation(format!(
                "Missing trajectory source path for Petrel object `{}`.",
                object.display_name
            ))
        })?);
        let rows = parse_petrel_trajectory(trajectory_path)?;
        let supporting_sources = object
            .source_paths
            .iter()
            .skip(1)
            .map(Path::new)
            .collect::<Vec<_>>();
        let result = project
            .import_trajectory_rows_with_coordinate_reference_and_supporting_sources(
                trajectory_path,
                binding,
                Some("trajectory"),
                &rows,
                effective_coordinate_reference.map(project_coordinate_reference_from_descriptor),
                &supporting_sources,
            )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            object.source_paths.clone(),
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!("imported_trajectory_rows: {}", rows.len())],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("petrel-tops:") {
        let well_key = petrel_object_well_key(object).ok_or_else(|| {
            LasError::Validation(format!(
                "Could not determine the well key for Petrel tops object `{}`.",
                object.vendor_object_id
            ))
        })?;
        let tops_path = Path::new(object.source_paths.first().ok_or_else(|| {
            LasError::Validation(format!(
                "Missing tops source path for Petrel object `{}`.",
                object.display_name
            ))
        })?);
        let rows = parse_petrel_tops_rows_for_well(tops_path, &well_key)?;
        let supporting_sources = object
            .source_paths
            .iter()
            .skip(1)
            .map(Path::new)
            .collect::<Vec<_>>();
        let result = project.import_tops_rows_with_supporting_sources(
            tops_path,
            binding,
            Some("tops"),
            &rows,
            &supporting_sources,
        )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            object.source_paths.clone(),
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!("imported_tops_rows: {}", rows.len())],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("petrel-checkshot:") {
        let well_key = petrel_object_well_key(object).ok_or_else(|| {
            LasError::Validation(format!(
                "Could not determine the well key for Petrel checkshot object `{}`.",
                object.vendor_object_id
            ))
        })?;
        let checkshot_path = Path::new(object.source_paths.first().ok_or_else(|| {
            LasError::Validation(format!(
                "Missing checkshot source path for Petrel object `{}`.",
                object.display_name
            ))
        })?);
        let observation_set = parse_petrel_checkshot_set_for_well(checkshot_path, &well_key)?;
        let sample_count = observation_set.samples.len();
        let result = project.create_checkshot_vsp_observation_set(
            checkshot_path,
            binding.clone(),
            Some("checkshots"),
            &observation_set,
        )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            object.source_paths.clone(),
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!(
                "imported_checkshot_observation_rows: {sample_count}"
            )],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    Err(LasError::Validation(format!(
        "Petrel vendor object `{}` is not supported for non-dry-run commit yet.",
        object.vendor_object_id
    )))
}

fn commit_opendtect_object(
    project: &mut OphioliteProject,
    vendor_project_root: &str,
    object: &VendorProjectObjectPreview,
    binding: Option<&AssetBindingInput>,
    effective_coordinate_reference: Option<&CoordinateReferenceDescriptor>,
    bridge_output: Option<&VendorProjectBridgeOutput>,
    imported_assets: &mut Vec<VendorProjectCommittedAsset>,
    preserved_raw_sources: &mut Vec<VendorProjectCommittedAsset>,
    validation_reports: &mut Vec<VendorProjectValidationReport>,
) -> Result<()> {
    if object.disposition == VendorProjectImportDisposition::RawSourceOnly {
        let source_paths = object
            .source_paths
            .iter()
            .map(Path::new)
            .collect::<Vec<_>>();
        let result = match binding {
            Some(binding) => project.import_raw_source_bundle_with_binding(
                &source_paths,
                binding,
                Some(&object.display_name),
            )?,
            None => project.import_raw_source_bundle_into_project_archive(
                &source_paths,
                Some(&object.display_name),
            )?,
        };
        preserved_raw_sources.push(committed_asset_from_project_result(
            object,
            result,
            object.source_paths.clone(),
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!(
                "preserved_raw_source_bundle: {} source files",
                object.source_paths.len()
            )],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    let binding = binding.ok_or_else(|| {
        LasError::Validation(format!(
            "Vendor object `{}` requires a binding for non-dry-run commit.",
            object.display_name
        ))
    })?;

    if object.requires_crs_decision && effective_coordinate_reference.is_none() {
        return Err(LasError::Validation(format!(
            "Vendor object `{}` requires a coordinate reference for non-dry-run commit.",
            object.display_name
        )));
    }

    if object.vendor_object_id.starts_with("seismic-segy:") {
        let segy_path = Path::new(object.source_paths.first().ok_or_else(|| {
            LasError::Validation(format!(
                "Missing SEG-Y source path for OpendTect seismic object `{}`.",
                object.display_name
            ))
        })?);
        let result = project.import_seismic_volume(
            segy_path,
            binding,
            Some(&object.display_name),
            effective_coordinate_reference,
        )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            vec![path_string(segy_path)],
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![String::from(
                "imported_open_segy_volume: canonical seismic trace-data asset created from the vendor project's SEG-Y companion",
            )],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("seismic-cbvs:") {
        let bridge_output = validate_bridge_output_for_object(
            VendorProjectImportVendor::Opendtect,
            object,
            bridge_output,
        )?
        .ok_or_else(|| {
            let capability = bridge_capability_for_object(VendorProjectImportVendor::Opendtect, object)
                .expect("OpendTect CBVS objects should have a registered bridge capability");
            LasError::Validation(format!(
                "OpendTect seismic object `{}` requires bridge kind `{:?}` output before canonical commit.",
                object.display_name, capability.bridge_kind
            ))
        })?;
        let coordinate_reference = bridge_output
            .coordinate_reference
            .as_ref()
            .or(effective_coordinate_reference);
        let result = match bridge_output.format {
            VendorProjectBridgeFormat::Segy
            | VendorProjectBridgeFormat::ZarrStore
            | VendorProjectBridgeFormat::OpenVdsStore => project.import_seismic_volume(
                &bridge_output.path,
                binding,
                Some(&object.display_name),
                coordinate_reference,
            )?,
            VendorProjectBridgeFormat::TbvolStore => project
                .import_seismic_trace_data_store_with_coordinate_reference(
                    &bridge_output.path,
                    binding,
                    Some(&object.display_name),
                    coordinate_reference,
                )?,
        };
        let mut notes = object.notes.clone();
        notes.extend(bridge_output.notes.clone());
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            bridge_output_source_paths(object, bridge_output),
            notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!(
                "imported_cbvs_via_bridge_output: {:?} -> canonical seismic trace-data asset",
                bridge_output.format
            )],
            notes,
        });
        return Ok(());
    }

    let stem = opendtect_stem_for_vendor_object_id(&object.vendor_object_id).ok_or_else(|| {
        LasError::Validation(format!(
            "Could not resolve an OpendTect well-family stem from `{}`.",
            object.vendor_object_id
        ))
    })?;
    let family = opendtect_well_family_files(Path::new(vendor_project_root), &stem)?;

    if object.vendor_object_id.starts_with("well:") {
        let well_path = family.well_path.as_ref().ok_or_else(|| {
            LasError::Validation(format!("Missing `.well` file for OpendTect well `{stem}`."))
        })?;
        let rows = parse_opendtect_well_track(well_path)?;
        let result = project
            .import_trajectory_rows_with_coordinate_reference_and_supporting_sources(
                well_path,
                binding,
                Some("trajectory"),
                &rows,
                effective_coordinate_reference.map(project_coordinate_reference_from_descriptor),
                &[],
            )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            vec![path_string(well_path)],
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!("imported_trajectory_rows: {}", rows.len())],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("well-logs:") {
        let mut log_names = Vec::new();
        for log_path in &family.log_paths {
            let parsed = parse_opendtect_log_file(log_path, binding)?;
            let collection_name = parsed.collection_name.clone();
            let result = project.import_log_file_with_binding_and_supporting_sources(
                parsed.file,
                binding,
                Some(&collection_name),
                &[],
            )?;
            log_names.push(collection_name.clone());
            imported_assets.push(committed_asset_from_project_result(
                object,
                result,
                vec![path_string(log_path)],
                object.notes.clone(),
            ));
        }
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!("imported_logs: {}", log_names.len())],
            notes: log_names,
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("well-markers:") {
        let marker_path = family.marker_path.as_ref().ok_or_else(|| {
            LasError::Validation(format!("Missing `.wlm` file for OpendTect well `{stem}`."))
        })?;
        let rows = parse_opendtect_well_markers(marker_path)?;
        let result = project.import_well_marker_rows_with_supporting_sources(
            marker_path,
            binding,
            Some("markers"),
            &rows,
            &[],
        )?;
        imported_assets.push(committed_asset_from_project_result(
            object,
            result,
            vec![path_string(marker_path)],
            object.notes.clone(),
        ));
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!("imported_markers: {}", rows.len())],
            notes: object.notes.clone(),
        });
        return Ok(());
    }

    if object.vendor_object_id.starts_with("well-time-depth:") {
        let mut model_names = Vec::new();
        if let Some(wlt_path) = &family.wlt_path {
            let model = parse_opendtect_time_depth_model(
                wlt_path,
                TimeDepthTransformSourceKind::VelocityFunction1D,
            )?;
            let collection_name = model.name.clone();
            let result = project.create_well_time_depth_model(
                wlt_path,
                binding.clone(),
                Some(&collection_name),
                &model,
            )?;
            model_names.push(collection_name);
            imported_assets.push(committed_asset_from_project_result(
                object,
                result,
                vec![path_string(wlt_path)],
                object.notes.clone(),
            ));
        }
        if let Some(csmdl_path) = &family.csmdl_path {
            let model = parse_opendtect_time_depth_model(
                csmdl_path,
                TimeDepthTransformSourceKind::CheckshotModel1D,
            )?;
            let collection_name = model.name.clone();
            let result = project.create_well_time_depth_model(
                csmdl_path,
                binding.clone(),
                Some(&collection_name),
                &model,
            )?;
            model_names.push(collection_name);
            imported_assets.push(committed_asset_from_project_result(
                object,
                result,
                vec![path_string(csmdl_path)],
                object.notes.clone(),
            ));
        }
        validation_reports.push(VendorProjectValidationReport {
            vendor_object_id: object.vendor_object_id.clone(),
            display_name: object.display_name.clone(),
            checks: vec![format!("imported_time_depth_models: {}", model_names.len())],
            notes: model_names,
        });
        return Ok(());
    }

    Err(LasError::Validation(format!(
        "OpendTect vendor object `{}` is not supported for non-dry-run commit yet.",
        object.vendor_object_id
    )))
}

#[derive(Debug, Default)]
struct OpendtectWellFamilyFiles {
    well_path: Option<PathBuf>,
    marker_path: Option<PathBuf>,
    wlt_path: Option<PathBuf>,
    csmdl_path: Option<PathBuf>,
    log_paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct ParsedOpendtectLogFile {
    collection_name: String,
    file: LasFile,
}

fn opendtect_well_family_files(
    vendor_project_root: &Path,
    stem: &str,
) -> Result<OpendtectWellFamilyFiles> {
    let well_dir = vendor_project_root.join("WellInfo");
    let mut files = OpendtectWellFamilyFiles::default();
    for path in read_dir_sorted(&well_dir)? {
        let Some(group_stem) = vendor_group_stem(&path) else {
            continue;
        };
        if group_stem != stem {
            continue;
        }
        if extension_eq(&path, "well") {
            files.well_path = Some(path);
        } else if extension_eq(&path, "wlm") {
            files.marker_path = Some(path);
        } else if extension_eq(&path, "wlt") {
            files.wlt_path = Some(path);
        } else if extension_eq(&path, "csmdl") {
            files.csmdl_path = Some(path);
        } else if extension_eq(&path, "wll") {
            files.log_paths.push(path);
        }
    }
    files.log_paths.sort();
    Ok(files)
}

fn opendtect_stem_for_vendor_object_id(vendor_object_id: &str) -> Option<String> {
    vendor_object_id
        .split_once(':')
        .map(|(_, value)| value.to_string())
}

fn parse_opendtect_well_track(path: &Path) -> Result<Vec<TrajectoryRow>> {
    let text = fs::read_to_string(path)?;
    let data_offset = find_data_section_offset(text.as_bytes()).unwrap_or(0);
    let mut stations = Vec::new();
    for line in text[data_offset..].lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 4 {
            continue;
        }
        let x = parts[0].parse::<f64>().map_err(|error| {
            LasError::Parse(format!("failed to parse X from `{trimmed}`: {error}"))
        })?;
        let y = parts[1].parse::<f64>().map_err(|error| {
            LasError::Parse(format!("failed to parse Y from `{trimmed}`: {error}"))
        })?;
        let z = parts[2].parse::<f64>().map_err(|error| {
            LasError::Parse(format!("failed to parse Z from `{trimmed}`: {error}"))
        })?;
        let measured_depth = parts[3].parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse measured depth from `{trimmed}`: {error}"
            ))
        })?;
        stations.push((x, y, z, measured_depth));
    }
    if stations.is_empty() {
        return Err(LasError::Parse(format!(
            "OpendTect well file `{}` did not contain any trajectory stations.",
            path_string(path)
        )));
    }
    let (origin_x, origin_y, origin_z, _) = stations[0];
    Ok(stations
        .into_iter()
        .map(|(x, y, z, measured_depth)| TrajectoryRow {
            measured_depth,
            true_vertical_depth: Some(z - origin_z),
            true_vertical_depth_subsea: Some(z),
            azimuth_deg: None,
            inclination_deg: None,
            northing_offset: Some(y - origin_y),
            easting_offset: Some(x - origin_x),
        })
        .collect())
}

fn parse_opendtect_well_markers(path: &Path) -> Result<Vec<WellMarkerRow>> {
    let text = fs::read_to_string(path)?;
    let mut grouped = BTreeMap::<usize, BTreeMap<String, String>>::new();
    for line in text.lines() {
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let raw_key = raw_key.trim();
        let Some((index_text, field_name)) = raw_key.split_once('.') else {
            continue;
        };
        let Ok(index) = index_text.parse::<usize>() else {
            continue;
        };
        grouped
            .entry(index)
            .or_default()
            .insert(field_name.trim().to_string(), raw_value.trim().to_string());
    }

    let mut rows = Vec::new();
    for fields in grouped.into_values() {
        let Some(name) = fields.get("Name").cloned() else {
            continue;
        };
        let Some(depth_text) = fields.get("Depth along hole") else {
            continue;
        };
        let top_depth = depth_text.parse::<f64>().map_err(|error| {
            LasError::Parse(format!(
                "failed to parse marker depth `{depth_text}` in `{}`: {error}",
                path_string(path)
            ))
        })?;
        let mut note_parts = Vec::new();
        if let Some(strat_level) = fields.get("Strat Level") {
            note_parts.push(format!("strat_level={strat_level}"));
        }
        if let Some(color) = fields.get("Color") {
            note_parts.push(format!("color={color}"));
        }
        rows.push(WellMarkerRow {
            name,
            marker_kind: None,
            top_depth,
            base_depth: None,
            source: Some(String::from("opendtect_wlm")),
            source_depth_reference: Some(String::from("md")),
            depth_domain: Some(String::from("md")),
            depth_datum: None,
            note: (!note_parts.is_empty()).then_some(note_parts.join("; ")),
        });
    }
    Ok(rows)
}

fn parse_opendtect_log_file(
    path: &Path,
    binding: &AssetBindingInput,
) -> Result<ParsedOpendtectLogFile> {
    let bytes = fs::read(path)?;
    let header_end = find_data_section_offset(&bytes).ok_or_else(|| {
        LasError::Parse(format!(
            "OpendTect log file `{}` is missing a binary data section marker.",
            path_string(path)
        ))
    })?;
    let header_text = String::from_utf8_lossy(&bytes[..header_end]);
    let header = parse_opendtect_key_value_header(&header_text);
    let payload = &bytes[header_end..];
    let sample_count = payload.len() / (2 * std::mem::size_of::<f32>());
    if sample_count == 0 {
        return Err(LasError::Parse(format!(
            "OpendTect log file `{}` did not contain any binary samples.",
            path_string(path)
        )));
    }

    let mut depth_values = Vec::with_capacity(sample_count);
    let mut curve_values = Vec::with_capacity(sample_count);
    for chunk in payload.chunks_exact(8) {
        let depth = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64;
        let value = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as f64;
        if !depth.is_finite() {
            continue;
        }
        depth_values.push(LasValue::Number(depth));
        curve_values.push(if value.is_finite() {
            LasValue::Number(value)
        } else {
            LasValue::Empty
        });
    }
    if depth_values.is_empty() {
        return Err(LasError::Parse(format!(
            "OpendTect log file `{}` did not produce any finite depth samples.",
            path_string(path)
        )));
    }

    let imported_at_unix_seconds = now_unix_seconds();
    let provenance = Provenance::from_path(
        path,
        source_fingerprint_for_bytes(&bytes),
        imported_at_unix_seconds,
    );
    let log_name = normalized_opendtect_display_name(
        header.get("Name").map(String::as_str),
        &file_stem_string(path)?,
    );
    let mnemonic_raw = header
        .get("Mnemonic")
        .cloned()
        .unwrap_or_else(|| log_name.clone());
    let mnemonic = sanitize_las_mnemonic(&mnemonic_raw);
    let unit = header.get("Unit of Measure").cloned().unwrap_or_default();
    let depth_unit = header
        .get("Depth-Unit")
        .cloned()
        .unwrap_or_else(|| String::from("Meter"));
    let row_count = depth_values.len();
    let start = depth_values
        .first()
        .and_then(LasValue::as_f64)
        .unwrap_or(0.0);
    let stop = depth_values
        .last()
        .and_then(LasValue::as_f64)
        .unwrap_or(start);
    let step = depth_values
        .windows(2)
        .find_map(|pair| Some(pair[1].as_f64()? - pair[0].as_f64()?))
        .unwrap_or(0.0);
    let curve_count = 2;

    let file = LasFile {
        summary: LasFileSummary {
            source_path: provenance.source_path.clone(),
            original_filename: provenance.original_filename.clone(),
            source_fingerprint: provenance.source_fingerprint.clone(),
            las_version: String::from("2.0"),
            wrap_mode: String::from("NO"),
            delimiter: String::from("space"),
            row_count,
            curve_count,
            issue_count: 0,
        },
        provenance,
        encoding: None,
        index: IndexDescriptor {
            curve_id: String::from("DEPT"),
            raw_mnemonic: String::from("DEPT"),
            unit: depth_unit.clone(),
            kind: IndexKind::Depth,
        },
        version: SectionItems::from_items(
            vec![
                HeaderItem::new("VERS", "", 2.0, "CWLS log ASCII Standard"),
                HeaderItem::new("WRAP", "", "NO", "One line per depth step"),
            ],
            MnemonicCase::Upper,
        ),
        well: SectionItems::from_items(
            vec![
                HeaderItem::new("WELL", "", binding.well_name.clone(), "Well name"),
                HeaderItem::new(
                    "UWI",
                    "",
                    binding.uwi.clone().unwrap_or_default(),
                    "Well identifier",
                ),
                HeaderItem::new("STRT", depth_unit.clone(), start, "Start depth"),
                HeaderItem::new("STOP", depth_unit.clone(), stop, "Stop depth"),
                HeaderItem::new("STEP", depth_unit.clone(), step, "Step"),
                HeaderItem::new("NULL", "", -999.25, "Null value"),
            ],
            MnemonicCase::Upper,
        ),
        params: SectionItems::from_items(Vec::new(), MnemonicCase::Upper),
        curves: SectionItems::from_items(
            vec![
                CurveItem::new(
                    "DEPT",
                    depth_unit.clone(),
                    LasValue::Empty,
                    "Depth",
                    depth_values,
                ),
                CurveItem::new(
                    mnemonic,
                    unit,
                    LasValue::Empty,
                    log_name.clone(),
                    curve_values,
                ),
            ],
            MnemonicCase::Upper,
        ),
        other: String::new(),
        extra_sections: BTreeMap::new(),
        issues: Vec::<IngestIssue>::new(),
        index_unit: Some(depth_unit),
    };

    Ok(ParsedOpendtectLogFile {
        collection_name: log_name,
        file,
    })
}

fn parse_opendtect_time_depth_model(
    path: &Path,
    source_kind: TimeDepthTransformSourceKind,
) -> Result<WellTimeDepthModel1D> {
    let text = fs::read_to_string(path)?;
    let data_offset = find_data_section_offset(text.as_bytes()).unwrap_or(0);
    let header = parse_opendtect_key_value_header(&text[..data_offset]);
    let mut samples = text[data_offset..]
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let parts = trimmed.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }
            Some((parts[0].to_string(), parts[1].to_string()))
        })
        .map(|(depth_text, time_text)| {
            let depth = depth_text.parse::<f32>().map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse time-depth depth `{depth_text}` in `{}`: {error}",
                    path_string(path)
                ))
            })?;
            let time_seconds = time_text.parse::<f32>().map_err(|error| {
                LasError::Parse(format!(
                    "failed to parse time-depth time `{time_text}` in `{}`: {error}",
                    path_string(path)
                ))
            })?;
            Ok(TimeDepthSample1D {
                depth,
                time_ms: time_seconds * 1000.0,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    samples.sort_by(|left, right| {
        left.depth
            .partial_cmp(&right.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    samples.dedup_by(|left, right| (left.depth - right.depth).abs() <= f32::EPSILON);

    let fallback_name = file_stem_string(path)?;
    Ok(WellTimeDepthModel1D {
        id: format!(
            "opendtect-{}",
            sanitize_vendor_object_key(
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("time-depth")
            )
        ),
        name: normalized_opendtect_display_name(
            header.get("Name").map(String::as_str),
            &fallback_name,
        ),
        wellbore_id: None,
        source_kind,
        depth_reference: DepthReferenceKind::MeasuredDepth,
        travel_time_reference: TravelTimeReference::TwoWay,
        samples,
        notes: vec![format!(
            "Imported from OpendTect {} as measured-depth to two-way-time samples.",
            path.extension()
                .and_then(|value| value.to_str())
                .unwrap_or("time-depth")
                .to_ascii_uppercase()
        )],
    })
}

fn parse_opendtect_key_value_header(text: &str) -> BTreeMap<String, String> {
    let mut header = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        header.insert(key.trim().to_string(), value.trim().to_string());
    }
    header
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpendtectNativeIdMatch {
    native_id: String,
    source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatchExecutableResolution {
    command: String,
    source_note: Option<String>,
}

fn discover_opendtect_native_id(
    project_root: &Path,
    display_name: &str,
) -> Result<Option<OpendtectNativeIdMatch>> {
    let target = normalized_vendor_name_key(display_name);

    let proc_dir = project_root.join("Proc");
    for path in read_dir_sorted(&proc_dir)? {
        if !extension_eq(&path, "par") {
            continue;
        }
        let stem = file_stem_string(&path)?;
        let stem_key = normalized_vendor_name_key(&stem);
        if !stem_key.contains(&target) {
            continue;
        }
        if let Some(native_id) = parse_opendtect_input_id_from_par(&path)? {
            return Ok(Some(OpendtectNativeIdMatch {
                native_id,
                source_path: path_string(&path),
            }));
        }
    }

    let attrib_dir = project_root.join("Attribs");
    for path in read_dir_sorted(&attrib_dir)? {
        if !extension_eq(&path, "attr") {
            continue;
        }
        if let Some(native_id) = parse_opendtect_storage_id_from_attr(&path, &target)? {
            return Ok(Some(OpendtectNativeIdMatch {
                native_id,
                source_path: path_string(&path),
            }));
        }
    }

    Ok(None)
}

fn parse_opendtect_input_id_from_par(path: &Path) -> Result<Option<String>> {
    let text = fs::read_to_string(path)?;
    Ok(parse_opendtect_key_value_header(&text)
        .get("Input.ID")
        .and_then(|value| non_empty_string(value)))
}

fn parse_opendtect_storage_id_from_attr(
    path: &Path,
    normalized_display_name: &str,
) -> Result<Option<String>> {
    let text = fs::read_to_string(path)?;
    let mut grouped = BTreeMap::<usize, BTreeMap<String, String>>::new();
    for line in text.lines() {
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let raw_key = raw_key.trim();
        let Some((index_text, field_name)) = raw_key.split_once('.') else {
            continue;
        };
        let Ok(index) = index_text.parse::<usize>() else {
            continue;
        };
        grouped
            .entry(index)
            .or_default()
            .insert(field_name.trim().to_string(), raw_value.trim().to_string());
    }

    for fields in grouped.into_values() {
        let user_ref = fields
            .get("UserRef")
            .map(String::as_str)
            .unwrap_or_default();
        if normalized_vendor_name_key(user_ref) != normalized_display_name {
            continue;
        }
        if let Some(definition) = fields.get("Definition") {
            if let Some(native_id) = parse_opendtect_storage_id_from_definition(definition) {
                return Ok(Some(native_id));
            }
        }
    }

    Ok(None)
}

fn parse_opendtect_storage_id_from_definition(definition: &str) -> Option<String> {
    let marker = "Storage id=";
    let start = definition.find(marker)? + marker.len();
    let tail = &definition[start..];
    let value = tail
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(value.to_string())
}

fn resolve_opendtect_batch_executable(
    request: &VendorProjectBridgeRunRequest,
) -> Result<BatchExecutableResolution> {
    const PROGRAM_NAME: &str = "od_process_segyio";

    if let Some(explicit_path) = request.executable_path.as_ref() {
        return resolve_batch_executable_candidate(explicit_path, request.execute).map(
            |resolved| BatchExecutableResolution {
                command: resolved.unwrap_or_else(|| explicit_path.clone()),
                source_note: Some(String::from(
                    "Using caller-supplied OpendTect batch executable.",
                )),
            },
        );
    }

    if let Some(installation_root) = request.installation_root.as_ref() {
        if let Some(resolved) =
            resolve_opendtect_executable_from_root(Path::new(installation_root), PROGRAM_NAME)
        {
            return Ok(BatchExecutableResolution {
                command: resolved,
                source_note: Some(format!(
                    "Resolved the OpendTect batch executable from installation root `{installation_root}`."
                )),
            });
        }
    }

    if let Some(dtect_appl) = env::var_os("DTECT_APPL") {
        if let Some(resolved) =
            resolve_opendtect_executable_from_root(Path::new(&dtect_appl), PROGRAM_NAME)
        {
            return Ok(BatchExecutableResolution {
                command: resolved,
                source_note: Some(String::from(
                    "Resolved the OpendTect batch executable from `DTECT_APPL`.",
                )),
            });
        }
    }

    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            let candidate = directory.join(PROGRAM_NAME);
            if candidate.is_file() {
                return Ok(BatchExecutableResolution {
                    command: path_string(&candidate),
                    source_note: Some(String::from(
                        "Resolved the OpendTect batch executable from `PATH`.",
                    )),
                });
            }
        }
    }

    if request.execute {
        return Err(LasError::Validation(format!(
            "Could not resolve `od_process_segyio`. Supply `executablePath`, `installationRoot`, or set `DTECT_APPL`."
        )));
    }

    Ok(BatchExecutableResolution {
        command: String::from(PROGRAM_NAME),
        source_note: None,
    })
}

fn resolve_batch_executable_candidate(
    candidate: &str,
    require_exists: bool,
) -> Result<Option<String>> {
    let path = Path::new(candidate);
    if path.components().count() > 1 || path.is_absolute() {
        if path.is_file() {
            return Ok(Some(path_string(path)));
        }
        if require_exists {
            return Err(LasError::Validation(format!(
                "Bridge executable path `{candidate}` does not exist."
            )));
        }
        return Ok(Some(candidate.to_string()));
    }
    Ok(None)
}

fn resolve_opendtect_executable_from_root(root: &Path, program_name: &str) -> Option<String> {
    let candidates = opendtect_executable_candidates(root, program_name);
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| path_string(&candidate))
}

fn opendtect_executable_candidates(root: &Path, program_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(root.join(program_name));
    candidates.push(root.join("Contents/MacOS").join(program_name));
    candidates.push(root.join("Contents/Resources/bin").join(program_name));
    candidates.push(root.join("bin").join(program_name));

    if root.file_name().and_then(|value| value.to_str()) == Some("MacOS") {
        if let Some(contents_dir) = root.parent() {
            if let Some(app_root) = contents_dir.parent() {
                candidates.push(app_root.join("Contents/MacOS").join(program_name));
                candidates.push(app_root.join("Contents/Resources/bin").join(program_name));
            }
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn render_opendtect_cbvs_export_parameter_file(
    project_root: &Path,
    native_id: &str,
    output_path: &Path,
    log_path: &Path,
) -> Result<String> {
    let survey_dir = project_root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            LasError::Validation(format!(
                "Could not determine survey directory name from `{}`.",
                path_string(project_root)
            ))
        })?;
    let data_root = project_root.parent().ok_or_else(|| {
        LasError::Validation(format!(
            "Could not determine data root parent for `{}`.",
            path_string(project_root)
        ))
    })?;

    Ok(format!(
        "dTect V7.0\nParameters\nGenerated by Ophiolite\n!\nTask: Export\nIs 2D: No\nInput.ID: {native_id}\nInput.Component: 0\nOutput.File name: {}\nLog file: {}\nProgram.Name: od_process_segyio\nSurvey: {survey_dir}\nData Root: {}\n!\n",
        path_string(output_path),
        path_string(log_path),
        path_string(data_root),
    ))
}

fn find_data_section_offset(bytes: &[u8]) -> Option<usize> {
    let markers = find_subsequence_positions(bytes, b"\n!\n");
    markers
        .get(1)
        .copied()
        .map(|index| index + 3)
        .or_else(|| markers.first().copied().map(|index| index + 3))
}

fn find_subsequence_positions(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }
    haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, window)| (window == needle).then_some(index))
        .collect()
}

fn committed_asset_from_project_result(
    object: &VendorProjectObjectPreview,
    result: ProjectAssetImportResult,
    source_paths: Vec<String>,
    notes: Vec<String>,
) -> VendorProjectCommittedAsset {
    VendorProjectCommittedAsset {
        vendor_object_id: object.vendor_object_id.clone(),
        display_name: object.display_name.clone(),
        canonical_target_kind: object.canonical_target_kind,
        disposition: object.disposition,
        asset_id: Some(result.asset.id.0),
        collection_id: Some(result.collection.id.0),
        collection_name: Some(result.collection.name),
        source_paths,
        notes,
    }
}

fn committed_asset_from_existing_asset(
    object: &VendorProjectObjectPreview,
    collection: &crate::AssetCollectionRecord,
    asset: &crate::AssetRecord,
    source_paths: Vec<String>,
    notes: Vec<String>,
) -> VendorProjectCommittedAsset {
    VendorProjectCommittedAsset {
        vendor_object_id: object.vendor_object_id.clone(),
        display_name: object.display_name.clone(),
        canonical_target_kind: object.canonical_target_kind,
        disposition: object.disposition,
        asset_id: Some(asset.id.0.clone()),
        collection_id: Some(collection.id.0.clone()),
        collection_name: Some(collection.name.clone()),
        source_paths,
        notes,
    }
}

fn validate_bridge_output_for_object<'a>(
    vendor: VendorProjectImportVendor,
    object: &VendorProjectObjectPreview,
    bridge_output: Option<&'a VendorProjectBridgeOutput>,
) -> Result<Option<&'a VendorProjectBridgeOutput>> {
    let Some(bridge_output) = bridge_output else {
        return Ok(None);
    };
    if bridge_output.vendor_object_id != object.vendor_object_id {
        return Err(LasError::Validation(format!(
            "Bridge output vendor object id `{}` does not match planned object `{}`.",
            bridge_output.vendor_object_id, object.vendor_object_id
        )));
    }
    let capability = bridge_capability_for_object(vendor, object).ok_or_else(|| {
        LasError::Validation(format!(
            "Vendor object `{}` does not have a registered bridge capability.",
            object.vendor_object_id
        ))
    })?;
    if !capability
        .accepted_output_formats
        .contains(&bridge_output.format)
    {
        return Err(LasError::Validation(format!(
            "Bridge output format `{:?}` is not supported for bridge kind `{:?}` and object `{}`.",
            bridge_output.format, capability.bridge_kind, object.vendor_object_id
        )));
    }
    let path = Path::new(&bridge_output.path);
    if !path.exists() {
        return Err(LasError::Validation(format!(
            "Bridge output path `{}` does not exist.",
            bridge_output.path
        )));
    }
    Ok(Some(bridge_output))
}

fn bridge_output_source_paths(
    object: &VendorProjectObjectPreview,
    bridge_output: &VendorProjectBridgeOutput,
) -> Vec<String> {
    let mut source_paths = object.source_paths.clone();
    if !source_paths.iter().any(|path| path == &bridge_output.path) {
        source_paths.push(bridge_output.path.clone());
    }
    source_paths
}

fn source_fingerprint_for_bytes(bytes: &[u8]) -> String {
    let checksum = bytes.iter().fold(0u64, |acc, byte| {
        acc.wrapping_mul(16777619).wrapping_add(u64::from(*byte))
    });
    revision_token_for_bytes("source", &format!("{}:{checksum}", bytes.len())).0
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn sanitize_las_mnemonic(value: &str) -> String {
    let mut mnemonic = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    if mnemonic.is_empty() {
        mnemonic.push_str("CURVE");
    }
    mnemonic.make_ascii_uppercase();
    mnemonic
}

fn normalized_opendtect_display_name(value: Option<&str>, fallback: &str) -> String {
    let Some(candidate) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return fallback.to_string();
    };
    let looks_like_filename = candidate
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|basename| basename.contains('.'));
    if looks_like_filename {
        Path::new(candidate)
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(candidate)
            .to_string()
    } else {
        candidate.to_string()
    }
}

fn normalize_petrel_horizon_source_paths(source_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if source_paths.is_empty() {
        return Err(LasError::Validation(String::from(
            "Petrel horizon import requires at least one source path.",
        )));
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let output_root = env::temp_dir().join(format!("ophiolite-petrel-horizon-{timestamp}"));
    fs::create_dir_all(&output_root)?;

    source_paths
        .iter()
        .map(|path| normalize_petrel_horizon_source_path(path, &output_root))
        .collect()
}

fn normalize_petrel_horizon_source_path(input_path: &Path, output_root: &Path) -> Result<PathBuf> {
    let text = fs::read_to_string(input_path)?;
    let mut normalized = String::new();
    let mut normalized_row_count = 0_usize;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let fields = trimmed
            .split(|character: char| {
                character.is_ascii_whitespace() || character == ',' || character == ';'
            })
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let xyz = if fields.len() >= 5 {
            match (
                fields[2].parse::<f64>(),
                fields[3].parse::<f64>(),
                fields[4].parse::<f64>(),
            ) {
                (Ok(x), Ok(y), Ok(z)) => Some((x, y, z)),
                _ => None,
            }
        } else if fields.len() >= 3 {
            match (
                fields[0].parse::<f64>(),
                fields[1].parse::<f64>(),
                fields[2].parse::<f64>(),
            ) {
                (Ok(x), Ok(y), Ok(z)) => Some((x, y, z)),
                _ => None,
            }
        } else {
            None
        };

        let Some((x, y, z)) = xyz else {
            continue;
        };
        normalized.push_str(&format!("{x} {y} {z}\n"));
        normalized_row_count += 1;
    }

    if normalized_row_count == 0 {
        return Err(LasError::Parse(format!(
            "Petrel horizon export `{}` did not contain any importable x/y/z rows.",
            path_string(input_path)
        )));
    }

    let output_name = input_path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("petrel_horizon"));
    let output_path = output_root.join(output_name).with_extension("xyz");
    fs::write(&output_path, normalized)?;
    Ok(output_path)
}

fn project_coordinate_reference_from_descriptor(
    reference: &CoordinateReferenceDescriptor,
) -> CoordinateReference {
    CoordinateReference {
        id: reference.id.clone(),
        name: reference.name.clone(),
        geodetic_datum: reference.geodetic_datum.clone(),
    }
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    paths.sort();
    Ok(paths)
}

fn collect_primary_with_companions(primary: &Path, companion_extensions: &[&str]) -> Vec<String> {
    let stem = primary
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let parent = primary.parent().unwrap_or_else(|| Path::new(""));
    let mut paths = vec![path_string(primary)];
    for extension in companion_extensions {
        let sibling = parent.join(format!("{stem}.{extension}"));
        if sibling.is_file() {
            paths.push(path_string(&sibling));
        }
    }
    paths
}

fn file_stem_string(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            LasError::Validation(format!(
                "Could not determine a UTF-8 file stem for `{}`.",
                path_string(path)
            ))
        })
}

fn vendor_group_stem(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    Some(stem.split('^').next().unwrap_or(stem).to_string())
}

fn extension_eq(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn sanitize_vendor_object_key(value: &str) -> String {
    value.replace(' ', "_")
}

fn default_true() -> bool {
    true
}

fn normalized_vendor_name_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn split_backtick_parts(value: &str) -> Vec<&str> {
    value.split('`').map(str::trim).collect()
}

fn parse_i32_triplet(value: &str) -> Option<[i32; 3]> {
    let parts = split_backtick_parts(value);
    if parts.len() < 3 {
        return None;
    }
    Some([
        parts[0].parse::<i32>().ok()?,
        parts[1].parse::<i32>().ok()?,
        parts[2].parse::<i32>().ok()?,
    ])
}

#[cfg(test)]
mod tests {
    use super::{
        VENDOR_PROJECT_IMPORT_SCHEMA_VERSION, VendorProjectBridgeArtifactKind,
        VendorProjectBridgeCapabilitiesResponse, VendorProjectBridgeCommitRequest,
        VendorProjectBridgeExecutionStatus, VendorProjectBridgeFormat, VendorProjectBridgeOutput,
        VendorProjectBridgeRunRequest, VendorProjectBridgeRuntimeRequirement,
        VendorProjectCommitRequest, VendorProjectConnectorContractResponse,
        VendorProjectConnectorIsolationBoundary, VendorProjectConnectorPhase,
        VendorProjectConnectorProvenanceGuarantee, VendorProjectImportVendor,
        VendorProjectPlanRequest, VendorProjectRuntimeKind, VendorProjectRuntimeObjectOpenStatus,
        VendorProjectRuntimeProbeRequest, VendorProjectRuntimeProbeStatus,
        VendorProjectScanRequest, bridge_commit_vendor_project_object,
        commit_vendor_project_import, plan_vendor_project_import, probe_vendor_project_runtime,
        run_vendor_project_bridge, scan_vendor_project, vendor_project_bridge_capabilities,
        vendor_project_connector_contract,
    };
    use crate::{AssetBindingInput, AssetId, OphioliteProject};
    use ndarray::Array3;
    use ophiolite_seismic::{
        CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
        DepthReferenceKind, ProjectedPoint2, ProjectedPolygon2, ProjectedVector2,
        SampleDataFidelity, SurveyGridTransform, SurveySpatialAvailability,
        SurveySpatialDescriptor, TravelTimeReference,
    };
    use ophiolite_seismic_runtime::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, TbvolManifest,
        VolumeAxes, VolumeMetadata, create_tbvol_store,
    };
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn scan_opendtect_project_detects_f3_like_inventory() {
        let root = write_temp_dir();
        write_file(
            &root.join(".survey"),
            "\
dTect V7.0.0
Survey Info
2023-06-12T10:25:29Z
!
Name: F3 Demo 2023
Survey Data Type: Both 2D and 3D
In-line range: 100`750`1
Cross-line range: 300`1250`1
Z range: 0`1.848`0.004`T
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&root.join("Rawdata/Seismic_data.sgy"), "segy");
        write_file(&root.join("Rawdata/Velocity_functions.txt"), "velocity");
        write_file(&root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(&root.join("Seismics/7a_AI_Cube_Std.par"), "par");
        write_file(&root.join("Surfaces/Demo_0_--__FS4.hor"), "hor");
        write_file(&root.join("Surfaces/Demo_1_--__MFS4.hor"), "hor");
        write_file(&root.join("Surfaces/Fault_A.flt"), "flt");
        write_file(&root.join("WellInfo/F03-2.well"), "well");
        write_file(&root.join("WellInfo/F03-2.wlm"), "markers");
        write_file(&root.join("WellInfo/F03-2.wlt"), "wlt");
        write_file(&root.join("WellInfo/F03-2.csmdl"), "csmdl");
        write_file(&root.join("WellInfo/F03-2^1.wll"), "log");
        write_file(&root.join("Shapefiles/NL offshore outline.shp"), "shape");
        write_file(&root.join("Shapefiles/NL offshore outline.prj"), "proj");
        write_file(
            &root.join("Locations/Random_Line_through_wells_extended.rdl"),
            "rdl",
        );

        let response = scan_vendor_project(&VendorProjectScanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
        })
        .expect("scan should succeed");

        assert_eq!(
            response.schema_version,
            VENDOR_PROJECT_IMPORT_SCHEMA_VERSION
        );
        assert_eq!(response.vendor_project.as_deref(), Some("F3 Demo 2023"));
        assert_eq!(
            response
                .survey_metadata
                .coordinate_reference
                .as_ref()
                .and_then(|value| value.id.as_deref()),
            Some("EPSG:23031")
        );

        let object_ids = response
            .objects
            .iter()
            .map(|object| object.vendor_object_id.as_str())
            .collect::<Vec<_>>();
        assert!(object_ids.contains(&"seismic-segy:Seismic_data"));
        assert!(object_ids.contains(&"seismic-cbvs:7a_AI_Cube_Std"));
        assert!(object_ids.contains(&"horizon-hor:Demo_0_--__FS4"));
        assert!(object_ids.contains(&"fault:Fault_A"));
        assert!(object_ids.contains(&"well:F03-2"));
        assert!(object_ids.contains(&"well-logs:F03-2"));
        assert!(object_ids.contains(&"well-markers:F03-2"));
        assert!(object_ids.contains(&"well-time-depth:F03-2"));

        let default_selected = response
            .objects
            .iter()
            .filter(|object| object.default_selected)
            .map(|object| object.vendor_object_id.as_str())
            .collect::<Vec<_>>();
        assert!(default_selected.contains(&"seismic-segy:Seismic_data"));
        assert!(default_selected.contains(&"seismic-cbvs:7a_AI_Cube_Std"));
        assert!(default_selected.contains(&"horizon-hor:Demo_0_--__FS4"));
        assert!(default_selected.contains(&"horizon-hor:Demo_1_--__MFS4"));
        assert!(default_selected.contains(&"fault:Fault_A"));
        assert!(default_selected.contains(&"well:F03-2"));
    }

    #[test]
    fn scan_petrel_project_detects_export_bundle_inventory() {
        let root = write_temp_dir();
        write_file(
            &root.join("Wells/Wellheader"),
            "\
WellName   X-Coord   Y-Coord            Top_Depth      Bottom Depth    KB          Symbol
A10  456979.063700   6782712.412000     1499.878992    2415.802791     0.000000    4
",
        );
        write_file(
            &root.join("Wells/A10.dev"),
            "\
# WELL TRACE FROM PETREL
# WELL NAME:              Well A-10
      MD              X             Y            Z          TVD           DX           DY         AZIM         INCL
  1499.878992   456979.063700 6782712.412000 -1499.878992  1499.878992     0.000000     0.000000    99.853422    42.277454
",
        );
        write_file(
            &root.join("Wells/A10.las"),
            "\
~Version
 VERS. 2.0
~Well
 WELL. A10
",
        );
        write_file(
            &root.join("well tops.txt"),
            "A10\tTop_Tarbert\t1858.55      HORIZON\n",
        );
        write_file(
            &root.join("well_tops_type.prn"),
            "A10\tTop_Tarbert\t1858.55      HORIZON\n",
        );
        write_file(&root.join("CheckShots1.txt"), "-1504.00  -1456.00  A10\n");
        write_file(
            &root.join("Seismic Interpretation (time)/Top Tarbert"),
            "1869182049 245518488 453561.805650 6780244.884530 1945.790000 Top_Tarbert\n",
        );
        write_file(
            &root.join("channel_probability"),
            "\
!
!     COORDINATE REFERENCE SYSTEM: ED50-UTM31
!
@Grid HEADER, GRID, 5
",
        );

        let response = scan_vendor_project(&VendorProjectScanRequest {
            vendor: VendorProjectImportVendor::Petrel,
            project_root: root.to_string_lossy().into_owned(),
        })
        .expect("petrel bundle scan should succeed");
        let preferred_tops_path = root.join("well tops.txt").to_string_lossy().into_owned();
        let channel_probability_path = root
            .join("channel_probability")
            .to_string_lossy()
            .into_owned();

        assert_eq!(response.vendor, VendorProjectImportVendor::Petrel);
        assert_eq!(
            response
                .survey_metadata
                .coordinate_reference
                .as_ref()
                .and_then(|value| value.name.as_deref()),
            Some("ED50-UTM31")
        );

        let object_ids = response
            .objects
            .iter()
            .map(|object| object.vendor_object_id.as_str())
            .collect::<Vec<_>>();
        assert!(object_ids.contains(&"petrel-checkshot:A10"));
        assert!(object_ids.contains(&"petrel-horizon-points:Top_Tarbert"));
        assert!(object_ids.contains(&"petrel-log:A10"));
        assert!(object_ids.contains(&"petrel-tops:A10"));
        assert!(object_ids.contains(&"petrel-trajectory:A10"));

        let trajectory = response
            .objects
            .iter()
            .find(|object| object.vendor_object_id == "petrel-trajectory:A10")
            .expect("trajectory preview");
        assert_eq!(
            trajectory.canonical_target_kind,
            super::VendorProjectCanonicalTargetKind::Trajectory
        );
        assert!(trajectory.requires_crs_decision);
        assert!(trajectory.default_selected);

        let tops = response
            .objects
            .iter()
            .find(|object| object.vendor_object_id == "petrel-tops:A10")
            .expect("tops preview");
        assert_eq!(
            tops.canonical_target_kind,
            super::VendorProjectCanonicalTargetKind::TopSet
        );
        assert_eq!(tops.source_paths.len(), 2);

        let checkshot = response
            .objects
            .iter()
            .find(|object| object.vendor_object_id == "petrel-checkshot:A10")
            .expect("checkshot preview");
        assert_eq!(
            checkshot.canonical_target_kind,
            super::VendorProjectCanonicalTargetKind::CheckshotVspObservationSet
        );
        assert_eq!(
            checkshot.disposition,
            super::VendorProjectImportDisposition::CanonicalWithLoss
        );

        let horizon = response
            .objects
            .iter()
            .find(|object| object.vendor_object_id == "petrel-horizon-points:Top_Tarbert")
            .expect("horizon preview");
        assert_eq!(
            horizon.canonical_target_kind,
            super::VendorProjectCanonicalTargetKind::SurveyStoreHorizon
        );
        assert_eq!(
            horizon.disposition,
            super::VendorProjectImportDisposition::CanonicalWithLoss
        );
        assert!(!horizon.requires_crs_decision);
        assert!(!horizon.default_selected);

        assert!(response.issues.iter().any(|issue| {
            issue.code == "petrel_duplicate_tops_exports"
                && issue.source_path.as_deref() == Some(preferred_tops_path.as_str())
        }));
        assert!(response.issues.iter().any(|issue| {
            issue.code == "petrel_export_unclassified"
                && issue.source_path.as_deref() == Some(channel_probability_path.as_str())
        }));
    }

    #[test]
    fn plan_blocks_petrel_multi_well_selection_and_allows_horizon_preservation() {
        let root = write_temp_dir();
        write_petrel_sample_well(&root, "A10");
        write_petrel_sample_well(&root, "B2");
        write_file(
            &root.join("Seismic Interpretation (time)/Top Tarbert"),
            "1869182049 245518488 453561.805650 6780244.884530 1945.790000 Top_Tarbert\n",
        );
        write_file(
            &root.join("channel_probability"),
            "\
!
!     COORDINATE REFERENCE SYSTEM: ED50-UTM31
!
@Grid HEADER, GRID, 5
",
        );

        let response = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Petrel,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![
                String::from("petrel-log:A10"),
                String::from("petrel-log:B2"),
                String::from("petrel-checkshot:A10"),
                String::from("petrel-horizon-points:Top_Tarbert"),
            ],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert!(
            response
                .blocking_issues
                .iter()
                .any(|issue| { issue.code == "petrel_multi_well_selection_unsupported" })
        );
        assert_eq!(response.blocking_issues.len(), 1);
    }

    #[test]
    fn commit_imports_petrel_single_well_logs_trajectory_tops_and_checkshots() {
        let vendor_root = write_temp_dir();
        write_petrel_sample_well(&vendor_root, "A10");
        write_file(
            &vendor_root.join("channel_probability"),
            "\
!
!     COORDINATE REFERENCE SYSTEM: ED50-UTM31
!
@Grid HEADER, GRID, 5
",
        );

        let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Petrel,
            project_root: vendor_root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![
                String::from("petrel-log:A10"),
                String::from("petrel-tops:A10"),
                String::from("petrel-trajectory:A10"),
                String::from("petrel-checkshot:A10"),
            ],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert!(plan.blocking_issues.is_empty());

        let target_project_root = write_temp_dir();
        OphioliteProject::create(&target_project_root).expect("project should be created");

        let committed = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: Some(sample_binding("A10")),
            target_survey_asset_id: None,
            coordinate_reference: None,
            bridge_outputs: Vec::new(),
            dry_run: false,
        })
        .expect("petrel commit should succeed");

        assert_eq!(committed.imported_assets.len(), 4);
        assert!(committed.preserved_raw_sources.is_empty());
        assert!(committed.validation_reports.iter().any(|report| {
            report.vendor_object_id == "petrel-log:A10"
                && report
                    .checks
                    .iter()
                    .any(|check| check.contains("imported_petrel_las_logs"))
        }));
        assert!(committed.validation_reports.iter().any(|report| {
            report.vendor_object_id == "petrel-trajectory:A10"
                && report
                    .checks
                    .iter()
                    .any(|check| check.contains("imported_trajectory_rows: 2"))
        }));
        assert!(committed.validation_reports.iter().any(|report| {
            report.vendor_object_id == "petrel-tops:A10"
                && report
                    .checks
                    .iter()
                    .any(|check| check.contains("imported_tops_rows: 1"))
        }));
        assert!(committed.validation_reports.iter().any(|report| {
            report.vendor_object_id == "petrel-checkshot:A10"
                && report
                    .checks
                    .iter()
                    .any(|check| check.contains("imported_checkshot_observation_rows: 2"))
        }));

        let project = OphioliteProject::open(&target_project_root).expect("project should open");
        let trajectory_asset_id = AssetId(
            committed
                .imported_assets
                .iter()
                .find(|asset| asset.vendor_object_id == "petrel-trajectory:A10")
                .and_then(|asset| asset.asset_id.clone())
                .expect("trajectory asset id"),
        );
        let tops_asset_id = AssetId(
            committed
                .imported_assets
                .iter()
                .find(|asset| asset.vendor_object_id == "petrel-tops:A10")
                .and_then(|asset| asset.asset_id.clone())
                .expect("tops asset id"),
        );
        let checkshot_asset_id = AssetId(
            committed
                .imported_assets
                .iter()
                .find(|asset| asset.vendor_object_id == "petrel-checkshot:A10")
                .and_then(|asset| asset.asset_id.clone())
                .expect("checkshot asset id"),
        );
        let trajectory_rows = project
            .read_trajectory_rows(&trajectory_asset_id, None)
            .expect("trajectory rows");
        let tops_rows = project.read_tops(&tops_asset_id).expect("tops rows");
        let checkshot_set = project
            .read_checkshot_vsp_observation_set(&checkshot_asset_id)
            .expect("checkshot set");
        assert_eq!(trajectory_rows.len(), 2);
        assert_eq!(tops_rows.len(), 1);
        assert_eq!(tops_rows[0].name, "Top_Tarbert");
        assert_eq!(checkshot_set.samples.len(), 2);
        assert_eq!(
            checkshot_set.depth_reference,
            DepthReferenceKind::TrueVerticalDepthSubsea
        );
        assert_eq!(
            checkshot_set.travel_time_reference,
            TravelTimeReference::TwoWay
        );
        assert_eq!(checkshot_set.samples[1].depth_m, 1456.0);
        assert_eq!(checkshot_set.samples[1].time_ms, 1504.0);
    }

    #[test]
    fn plan_reports_target_survey_candidates_for_petrel_horizon_imports() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join("Seismic Interpretation (time)/Top Tarbert"),
            "1869182049 245518488 453561.805650 6780244.884530 1945.790000 Top_Tarbert\n",
        );

        let target_project_root = write_temp_dir();
        let survey_asset_id = create_test_survey_asset(&target_project_root, "F3 Survey");

        let response = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Petrel,
            project_root: vendor_root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("petrel-horizon-points:Top_Tarbert")],
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            target_survey_asset_id: Some(survey_asset_id.0.clone()),
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert!(response.blocking_issues.is_empty());
        assert!(response.target_survey_asset_required);
        assert_eq!(response.planned_imports.len(), 1);
        assert!(response.planned_imports[0].requires_target_survey_asset);
        assert!(
            response
                .target_survey_asset_candidates
                .iter()
                .any(|candidate| candidate.asset_id == survey_asset_id)
        );
        assert_eq!(
            response
                .selected_target_survey_asset
                .as_ref()
                .map(|asset| asset.asset_id.clone()),
            Some(survey_asset_id)
        );
    }

    #[test]
    fn commit_imports_petrel_horizon_points_into_target_survey_asset() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join("Seismic Interpretation (time)/Top Tarbert"),
            "1869182049 245518488 453561.805650 6780244.884530 1945.790000 Top_Tarbert\n",
        );

        let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Petrel,
            project_root: vendor_root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("petrel-horizon-points:Top_Tarbert")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert!(plan.blocking_issues.is_empty());

        let target_project_root = write_temp_dir();
        let survey_asset_id = create_test_survey_asset(&target_project_root, "F3 Survey");

        let committed = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: None,
            target_survey_asset_id: Some(survey_asset_id.0.clone()),
            coordinate_reference: None,
            bridge_outputs: Vec::new(),
            dry_run: false,
        })
        .expect("petrel horizon commit should succeed");

        assert_eq!(committed.imported_assets.len(), 1);
        assert!(committed.preserved_raw_sources.is_empty());
        assert_eq!(
            committed.imported_assets[0].vendor_object_id,
            "petrel-horizon-points:Top_Tarbert"
        );
        assert_eq!(
            committed.imported_assets[0].canonical_target_kind,
            super::VendorProjectCanonicalTargetKind::SurveyStoreHorizon
        );
        assert!(committed.validation_reports.iter().any(|report| {
            report.vendor_object_id == "petrel-horizon-points:Top_Tarbert"
                && report
                    .checks
                    .iter()
                    .any(|check| check.contains("imported_survey_horizons: 1"))
        }));

        let project = OphioliteProject::open(&target_project_root).expect("project should open");
        let imported_asset_id = AssetId(
            committed.imported_assets[0]
                .asset_id
                .clone()
                .expect("survey asset id should be reported"),
        );
        assert_eq!(imported_asset_id, survey_asset_id);
        let survey_asset = project
            .asset_by_id(&imported_asset_id)
            .expect("survey asset should exist");
        let horizons = ophiolite_seismic_runtime::load_horizon_grids(
            &Path::new(&survey_asset.package_path).join("store"),
        )
        .expect("survey horizons should load");
        assert_eq!(horizons.len(), 1);
        assert_eq!(horizons[0].descriptor.name, "Top Tarbert");
    }

    #[test]
    fn commit_still_requires_binding_for_canonical_petrel_assets() {
        let vendor_root = write_temp_dir();
        write_petrel_sample_well(&vendor_root, "A10");

        let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Petrel,
            project_root: vendor_root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("petrel-log:A10")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        let target_project_root = write_temp_dir();
        OphioliteProject::create(&target_project_root).expect("project should be created");

        let error = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: None,
            target_survey_asset_id: None,
            coordinate_reference: None,
            bridge_outputs: Vec::new(),
            dry_run: false,
        })
        .expect_err("canonical petrel commit should require binding");
        assert!(error.to_string().contains("requires `binding`"));
    }

    #[test]
    fn plan_reports_bridge_request_for_cbvs_imports() {
        let root = write_temp_dir();
        write_file(
            &root.join(".survey"),
            "\
Name: F3 Demo 2023
Survey Data Type: 3D
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(&root.join("Seismics/7a_AI_Cube_Std.par"), "par");
        write_file(
            &root.join("Proc/7a_AI_Cube_Std_16bit.par"),
            "\
Input.ID: 100010.7
",
        );

        let response = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("seismic-cbvs:7a_AI_Cube_Std")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert_eq!(response.planned_imports.len(), 1);
        assert_eq!(response.bridge_requests.len(), 1);
        assert_eq!(
            response.bridge_requests[0].recommended_output_format,
            VendorProjectBridgeFormat::Segy
        );
        assert_eq!(
            response.bridge_requests[0].vendor_native_id.as_deref(),
            Some("100010.7")
        );
        assert_eq!(
            response.bridge_requests[0].automatic_execution_formats,
            vec![VendorProjectBridgeFormat::Segy]
        );
        assert_eq!(
            response.bridge_requests[0].runtime_requirements,
            vec![
                VendorProjectBridgeRuntimeRequirement::VendorBatchExecutable,
                VendorProjectBridgeRuntimeRequirement::VendorProjectDataRoot,
            ]
        );
    }

    #[test]
    fn bridge_capabilities_report_registered_vendor_registry() {
        let response: VendorProjectBridgeCapabilitiesResponse =
            vendor_project_bridge_capabilities(VendorProjectImportVendor::Opendtect);

        assert_eq!(
            response.schema_version,
            VENDOR_PROJECT_IMPORT_SCHEMA_VERSION
        );
        assert_eq!(response.capabilities.len(), 1);
        assert_eq!(
            response.capabilities[0].supported_vendor_object_prefixes,
            vec![String::from("seismic-cbvs:")]
        );
        assert_eq!(
            response.capabilities[0].accepted_output_formats,
            vec![
                VendorProjectBridgeFormat::Segy,
                VendorProjectBridgeFormat::TbvolStore,
                VendorProjectBridgeFormat::ZarrStore,
                VendorProjectBridgeFormat::OpenVdsStore,
            ]
        );
        assert_eq!(
            response.capabilities[0].runtime_requirements,
            vec![
                VendorProjectBridgeRuntimeRequirement::VendorBatchExecutable,
                VendorProjectBridgeRuntimeRequirement::VendorProjectDataRoot,
            ]
        );
    }

    #[test]
    fn connector_contract_reports_phased_external_runtime_design() {
        let response: VendorProjectConnectorContractResponse =
            vendor_project_connector_contract(VendorProjectImportVendor::Opendtect);

        assert_eq!(
            response.schema_version,
            VENDOR_PROJECT_IMPORT_SCHEMA_VERSION
        );
        assert_eq!(
            response.supported_runtime_kinds,
            vec![VendorProjectRuntimeKind::OpendtectOdbind]
        );
        assert_eq!(response.bridge_capabilities.len(), 1);
        assert_eq!(response.phases.len(), 6);
        assert_eq!(
            response.phases[0].phase,
            VendorProjectConnectorPhase::Discovery
        );
        assert_eq!(
            response.phases[0].isolation_boundary,
            VendorProjectConnectorIsolationBoundary::InProcess
        );
        assert!(response.phases.iter().any(|phase| {
            phase.phase == VendorProjectConnectorPhase::RuntimeProbe
                && phase.isolation_boundary == VendorProjectConnectorIsolationBoundary::OutOfProcess
        }));
        assert!(response.phases.iter().any(|phase| {
            phase.phase == VendorProjectConnectorPhase::BridgeExecution
                && phase.isolation_boundary == VendorProjectConnectorIsolationBoundary::OutOfProcess
        }));
        assert!(
            response
                .provenance_guarantees
                .contains(&VendorProjectConnectorProvenanceGuarantee::RuntimeIssue)
        );
        assert!(
            response
                .provenance_guarantees
                .contains(&VendorProjectConnectorProvenanceGuarantee::VendorObjectId)
        );
    }

    #[test]
    fn connector_contract_reports_petrel_export_bundle_surface() {
        let response: VendorProjectConnectorContractResponse =
            vendor_project_connector_contract(VendorProjectImportVendor::Petrel);

        assert_eq!(response.vendor, VendorProjectImportVendor::Petrel);
        assert!(response.bridge_capabilities.is_empty());
        assert!(response.supported_runtime_kinds.is_empty());
        assert_eq!(
            response.provenance_guarantees,
            vec![
                VendorProjectConnectorProvenanceGuarantee::VendorObjectId,
                VendorProjectConnectorProvenanceGuarantee::SourcePath,
                VendorProjectConnectorProvenanceGuarantee::CoordinateReferenceDecision,
            ]
        );
        assert_eq!(response.phases.len(), 3);
        assert_eq!(
            response.phases[0].phase,
            VendorProjectConnectorPhase::Discovery
        );
        assert_eq!(
            response.phases[1].phase,
            VendorProjectConnectorPhase::Planning
        );
        assert_eq!(
            response.phases[2].phase,
            VendorProjectConnectorPhase::CanonicalCommit
        );
        assert!(
            response
                .notes
                .iter()
                .any(|note| note.contains("single-well logs, trajectories, tops, and checkshots"))
        );
    }

    #[test]
    fn runtime_probe_reports_runtime_visibility_and_open_errors_for_bridgeable_objects() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        let fake_python = write_executable(
            "fake_runtime_ok.sh",
            r#"#!/bin/sh
shift 2
volume=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--volume" ]; then
    volume="$2"
    shift 2
  else
    shift
  fi
done
if [ -n "$volume" ]; then
  printf '%s\n' '{"status":"ok","surveyNames":["F3_Demo_2023"],"objectNames":{"Seismic Data":["7a AI Cube Std"]},"volumeProbe":{"hasObject":true,"objectInfoError":{"message":"translator group not found"},"openStatus":"error","openError":{"message":"IO object read error - "}}}'
else
  printf '%s\n' '{"status":"ok","surveyNames":["F3_Demo_2023"],"surveyInfo":{"name":"F3 Demo 2023","type":"2D3D"},"objectNames":{"Seismic Data":["7a AI Cube Std"],"Well":["F03-2"]}}'
fi
"#,
        );

        let response = probe_vendor_project_runtime(&VendorProjectRuntimeProbeRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: vendor_root.to_string_lossy().into_owned(),
            runtime: VendorProjectRuntimeKind::OpendtectOdbind,
            survey_name: Some(String::from("F3_Demo_2023")),
            python_executable: Some(fake_python.to_string_lossy().into_owned()),
            odbind_root: Some(String::from("/tmp/fake-odbind")),
            dtect_appl: Some(String::from("/tmp/fake-dtect-appl")),
            shared_library_path: Some(String::from("/tmp/fake-libs")),
            probe_bridgeable_objects: true,
        })
        .expect("runtime probe should succeed");

        assert_eq!(response.probe_status, VendorProjectRuntimeProbeStatus::Ok);
        assert!(response.survey_visible);
        assert_eq!(response.survey_names, vec![String::from("F3_Demo_2023")]);
        assert_eq!(response.object_groups.len(), 2);
        assert_eq!(response.object_statuses.len(), 1);
        assert_eq!(
            response.object_statuses[0].open_status,
            VendorProjectRuntimeObjectOpenStatus::OpenError
        );
        assert!(response.object_statuses[0].listed_in_runtime);
        assert_eq!(response.object_statuses[0].has_object, Some(true));
        assert_eq!(
            response.object_statuses[0].object_info_error.as_deref(),
            Some("translator group not found")
        );
        assert_eq!(
            response.object_statuses[0].open_error.as_deref(),
            Some("IO object read error - ")
        );
        assert!(response.issues.iter().any(|issue| {
            issue.code == "vendor_runtime_object_open_failed"
                && issue.vendor_object_id.as_deref() == Some("seismic-cbvs:7a_AI_Cube_Std")
        }));
    }

    #[test]
    fn runtime_probe_returns_import_error_status_when_runtime_surface_is_unavailable() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        let fake_python = write_executable(
            "fake_runtime_import_error.sh",
            r#"#!/bin/sh
printf '%s\n' '{"status":"import_error","error":{"message":"No module named odbind"}}'
exit 1
"#,
        );

        let response = probe_vendor_project_runtime(&VendorProjectRuntimeProbeRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: vendor_root.to_string_lossy().into_owned(),
            runtime: VendorProjectRuntimeKind::OpendtectOdbind,
            survey_name: Some(String::from("F3_Demo_2023")),
            python_executable: Some(fake_python.to_string_lossy().into_owned()),
            odbind_root: None,
            dtect_appl: None,
            shared_library_path: None,
            probe_bridgeable_objects: false,
        })
        .expect("runtime probe should still produce a structured response");

        assert_eq!(
            response.probe_status,
            VendorProjectRuntimeProbeStatus::ImportError
        );
        assert!(!response.survey_visible);
        assert!(response.object_statuses.is_empty());
        assert!(
            response
                .issues
                .iter()
                .any(|issue| issue.code == "vendor_runtime_import_error")
        );
    }

    #[test]
    fn plan_embeds_runtime_probe_response_and_surfaces_runtime_warnings() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(
            &vendor_root.join("Proc/7a_AI_Cube_Std_16bit.par"),
            "\
Input.ID: 100010.7
",
        );
        let fake_python = write_executable(
            "fake_runtime_plan_ok.sh",
            r#"#!/bin/sh
shift 2
volume=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--volume" ]; then
    volume="$2"
    shift 2
  else
    shift
  fi
done
if [ -n "$volume" ]; then
  printf '%s\n' '{"status":"ok","surveyNames":["F3_Demo_2023"],"objectNames":{"Seismic Data":["7a AI Cube Std"]},"volumeProbe":{"hasObject":false,"objectInfoError":{"message":"translator group not found"},"openStatus":"error","openError":{"message":"IO object read error - "}}}'
else
  printf '%s\n' '{"status":"ok","surveyNames":["F3_Demo_2023"],"surveyInfo":{"name":"F3 Demo 2023","type":"2D3D"},"objectNames":{"Seismic Data":["7a AI Cube Std"]}}'
fi
"#,
        );

        let response = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: vendor_root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("seismic-cbvs:7a_AI_Cube_Std")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: Some(String::from("EPSG:23031")),
                name: Some(String::from("ED50 / UTM zone 31N")),
                geodetic_datum: None,
                unit: None,
            }),
            runtime_probe: Some(VendorProjectRuntimeProbeRequest {
                vendor: VendorProjectImportVendor::Opendtect,
                project_root: vendor_root.to_string_lossy().into_owned(),
                runtime: VendorProjectRuntimeKind::OpendtectOdbind,
                survey_name: Some(String::from("F3_Demo_2023")),
                python_executable: Some(fake_python.to_string_lossy().into_owned()),
                odbind_root: Some(String::from("/tmp/fake-odbind")),
                dtect_appl: Some(String::from("/tmp/fake-dtect-appl")),
                shared_library_path: Some(String::from("/tmp/fake-libs")),
                probe_bridgeable_objects: true,
            }),
        })
        .expect("plan should succeed");

        assert_eq!(response.planned_imports.len(), 1);
        assert!(response.blocking_issues.is_empty());
        let runtime_probe = response
            .runtime_probe
            .as_ref()
            .expect("runtime probe should be embedded");
        assert_eq!(
            runtime_probe.probe_status,
            VendorProjectRuntimeProbeStatus::Ok
        );
        assert!(runtime_probe.survey_visible);
        assert_eq!(runtime_probe.object_statuses.len(), 1);
        assert_eq!(
            runtime_probe.object_statuses[0].vendor_object_id,
            "seismic-cbvs:7a_AI_Cube_Std"
        );
        assert_eq!(
            runtime_probe.object_statuses[0].open_status,
            VendorProjectRuntimeObjectOpenStatus::OpenError
        );
        assert!(response.warnings.iter().any(|issue| {
            issue.code == "vendor_runtime_object_open_failed"
                && issue.vendor_object_id.as_deref() == Some("seismic-cbvs:7a_AI_Cube_Std")
        }));
    }

    #[test]
    fn bridge_runner_prepares_opendtect_cbvs_export_parameter_file() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(
            &vendor_root.join("Proc/7a_AI_Cube_Std_16bit.par"),
            "\
Input.ID: 100010.7
",
        );
        let output_path = write_temp_dir().join("exports/7a_AI_Cube_Std.sgy");

        let response = run_vendor_project_bridge(&VendorProjectBridgeRunRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: vendor_root.to_string_lossy().into_owned(),
            vendor_object_id: String::from("seismic-cbvs:7a_AI_Cube_Std"),
            output_format: VendorProjectBridgeFormat::Segy,
            output_path: output_path.to_string_lossy().into_owned(),
            installation_root: None,
            executable_path: None,
            parameter_file_path: None,
            log_path: None,
            execute: false,
            overwrite_existing_output: false,
        })
        .expect("bridge preparation should succeed");

        assert_eq!(
            response.execution_status,
            VendorProjectBridgeExecutionStatus::Prepared
        );
        assert_eq!(response.vendor_native_id.as_deref(), Some("100010.7"));
        assert_eq!(response.output.format, VendorProjectBridgeFormat::Segy);
        let parameter_text =
            fs::read_to_string(&response.parameter_file_path).expect("parameter file should exist");
        assert!(parameter_text.contains("Task: Export"));
        assert!(parameter_text.contains("Input.ID: 100010.7"));
        assert!(parameter_text.contains("Output.File name:"));
        assert!(parameter_text.contains("Program.Name: od_process_segyio"));
        assert_eq!(response.command[0], "od_process_segyio");
        assert_eq!(response.artifacts.len(), 3);
        assert!(response.artifacts.iter().any(|artifact| {
            artifact.kind == VendorProjectBridgeArtifactKind::ParameterFile && artifact.exists
        }));
        assert!(response.artifacts.iter().any(|artifact| {
            artifact.kind == VendorProjectBridgeArtifactKind::BridgeOutput && !artifact.exists
        }));
    }

    #[test]
    fn bridge_runner_resolves_batch_executable_from_installation_root() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(
            &vendor_root.join("Proc/7a_AI_Cube_Std_16bit.par"),
            "\
Input.ID: 100010.7
",
        );

        let install_root = write_temp_dir().join("OpendTect.app");
        write_file(
            &install_root.join("Contents/MacOS/od_process_segyio"),
            "#!/bin/sh\nexit 0\n",
        );
        let output_path = write_temp_dir().join("exports/7a_AI_Cube_Std.sgy");

        let response = run_vendor_project_bridge(&VendorProjectBridgeRunRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: vendor_root.to_string_lossy().into_owned(),
            vendor_object_id: String::from("seismic-cbvs:7a_AI_Cube_Std"),
            output_format: VendorProjectBridgeFormat::Segy,
            output_path: output_path.to_string_lossy().into_owned(),
            installation_root: Some(install_root.to_string_lossy().into_owned()),
            executable_path: None,
            parameter_file_path: None,
            log_path: None,
            execute: false,
            overwrite_existing_output: false,
        })
        .expect("bridge preparation should succeed");

        assert_eq!(
            response.command[0],
            install_root
                .join("Contents/MacOS/od_process_segyio")
                .to_string_lossy()
        );
        assert_eq!(response.artifacts.len(), 3);
        assert!(response.notes.iter().any(|note| {
            note.contains("Resolved the OpendTect batch executable from installation root")
        }));
    }

    #[test]
    fn bridge_commit_supports_dry_run_with_prepared_bridge_output_path() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(
            &vendor_root.join("Proc/7a_AI_Cube_Std_16bit.par"),
            "\
Input.ID: 100010.7
",
        );
        let output_path = write_temp_dir().join("exports/7a_AI_Cube_Std.sgy");
        write_file(&output_path, "placeholder-segy");

        let response = bridge_commit_vendor_project_object(&VendorProjectBridgeCommitRequest {
            bridge_run: VendorProjectBridgeRunRequest {
                vendor: VendorProjectImportVendor::Opendtect,
                project_root: vendor_root.to_string_lossy().into_owned(),
                vendor_object_id: String::from("seismic-cbvs:7a_AI_Cube_Std"),
                output_format: VendorProjectBridgeFormat::Segy,
                output_path: output_path.to_string_lossy().into_owned(),
                installation_root: None,
                executable_path: None,
                parameter_file_path: None,
                log_path: None,
                execute: false,
                overwrite_existing_output: true,
            },
            target_project_root: None,
            binding: None,
            target_survey_asset_id: None,
            coordinate_reference: None,
            dry_run: true,
        })
        .expect("bridge commit dry-run should succeed");

        assert_eq!(
            response.bridge.execution_status,
            VendorProjectBridgeExecutionStatus::Prepared
        );
        assert_eq!(response.commit.validation_reports.len(), 1);
        assert!(
            response.commit.validation_reports[0].checks[0].contains("bridge_output_supported")
        );
    }

    #[test]
    fn bridge_commit_blocks_non_dry_run_when_bridge_output_is_not_materialized() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(
            &vendor_root.join("Proc/7a_AI_Cube_Std_16bit.par"),
            "\
Input.ID: 100010.7
",
        );
        let output_path = write_temp_dir().join("exports/7a_AI_Cube_Std.sgy");
        let target_project_root = write_temp_dir();
        OphioliteProject::create(&target_project_root).expect("project should be created");

        let error = bridge_commit_vendor_project_object(&VendorProjectBridgeCommitRequest {
            bridge_run: VendorProjectBridgeRunRequest {
                vendor: VendorProjectImportVendor::Opendtect,
                project_root: vendor_root.to_string_lossy().into_owned(),
                vendor_object_id: String::from("seismic-cbvs:7a_AI_Cube_Std"),
                output_format: VendorProjectBridgeFormat::Segy,
                output_path: output_path.to_string_lossy().into_owned(),
                installation_root: None,
                executable_path: None,
                parameter_file_path: None,
                log_path: None,
                execute: false,
                overwrite_existing_output: true,
            },
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: Some(sample_binding("F3 Survey")),
            target_survey_asset_id: None,
            coordinate_reference: None,
            dry_run: false,
        })
        .expect_err("missing bridge output should block non-dry-run bridge commit");

        assert!(
            error
                .to_string()
                .contains("does not exist after bridge preparation")
        );
    }

    #[test]
    fn plan_treats_opendtect_segy_as_supported_canonical_seismic_import() {
        let root = write_temp_dir();
        write_file(
            &root.join(".survey"),
            "\
Name: F3 Demo 2023
Survey Data Type: 3D
",
        );
        write_file(&root.join("Rawdata/Seismic_data.sgy"), "segy");

        let response = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("seismic-segy:Seismic_data")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert_eq!(response.planned_imports.len(), 1);
        assert!(response.blocking_issues.is_empty());
        assert_eq!(
            response.planned_imports[0].canonical_target_kind,
            super::VendorProjectCanonicalTargetKind::SeismicTraceData
        );
    }

    #[test]
    fn plan_blocks_geometry_objects_when_coordinate_reference_is_missing() {
        let root = write_temp_dir();
        write_file(
            &root.join(".survey"),
            "\
Name: Missing CRS
Survey Data Type: 3D
",
        );
        write_file(&root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");

        let response = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("seismic-cbvs:7a_AI_Cube_Std")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        assert_eq!(response.planned_imports.len(), 1);
        assert_eq!(response.blocking_issues.len(), 1);
        assert_eq!(
            response.blocking_issues[0].code,
            "missing_coordinate_reference"
        );
    }

    #[test]
    fn commit_imports_cbvs_when_tbvol_bridge_output_is_supplied() {
        let vendor_root = write_temp_dir();
        write_file(
            &vendor_root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.cbvs"), "cbvs");
        write_file(&vendor_root.join("Seismics/7a_AI_Cube_Std.par"), "par");

        let bridge_store = write_temp_dir().join("bridge.tbvol");
        let volume = VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: String::from("bridge-store"),
            source: SourceIdentity {
                source_path: bridge_store.clone(),
                file_size: 1,
                trace_count: 4,
                samples_per_trace: 3,
                sample_interval_us: 4000,
                sample_format_code: 5,
                sample_data_fidelity: SampleDataFidelity::default(),
                endianness: String::from("big"),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
                geometry: GeometryProvenance {
                    inline_field: HeaderFieldSpec {
                        name: String::from("INLINE_3D"),
                        start_byte: 189,
                        value_type: String::from("I32"),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: String::from("CROSSLINE_3D"),
                        start_byte: 193,
                        value_type: String::from("I32"),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape: [2, 2, 3],
            axes: VolumeAxes::from_time_axis(
                vec![100.0, 101.0],
                vec![200.0, 201.0],
                vec![0.0, 4.0, 8.0],
            ),
            segy_export: None,
            coordinate_reference_binding: None,
            spatial: None,
            created_by: String::from("test"),
            processing_lineage: None,
        };
        let data = Array3::from_shape_vec((2, 2, 3), vec![1.0; 12]).expect("shape");
        let manifest = TbvolManifest::new(volume, [2, 2, 3], false);
        create_tbvol_store(&bridge_store, manifest, &data, None).expect("bridge store");

        let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: vendor_root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("seismic-cbvs:7a_AI_Cube_Std")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        let target_project_root = write_temp_dir();
        OphioliteProject::create(&target_project_root).expect("project should be created");
        let binding = sample_binding("F3 Survey");

        let committed = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: Some(binding),
            target_survey_asset_id: None,
            coordinate_reference: None,
            bridge_outputs: vec![VendorProjectBridgeOutput {
                vendor_object_id: String::from("seismic-cbvs:7a_AI_Cube_Std"),
                format: VendorProjectBridgeFormat::TbvolStore,
                path: bridge_store.to_string_lossy().into_owned(),
                coordinate_reference: None,
                notes: vec![String::from("synthetic bridge output for test")],
            }],
            dry_run: false,
        })
        .expect("cbvs bridge commit should succeed");

        assert_eq!(committed.imported_assets.len(), 1);
        assert!(committed.imported_assets[0].asset_id.is_some());
        assert_eq!(committed.validation_reports.len(), 1);
        assert!(
            committed.validation_reports[0].checks[0].contains("imported_cbvs_via_bridge_output")
        );
    }

    #[test]
    fn commit_raw_source_bundle_supports_dry_run_and_non_dry_run() {
        let root = write_temp_dir();
        write_file(
            &root.join(".survey"),
            "\
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(&root.join("Surfaces/Fault_A.flt"), "flt");

        let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("fault:Fault_A")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");

        let dry_run = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan: plan.clone(),
            target_project_root: None,
            binding: None,
            target_survey_asset_id: None,
            coordinate_reference: None,
            bridge_outputs: Vec::new(),
            dry_run: true,
        })
        .expect("dry-run should succeed");
        assert_eq!(dry_run.validation_reports.len(), 1);
        assert_eq!(dry_run.preserved_raw_sources.len(), 1);
        assert!(dry_run.imported_assets.is_empty());

        let target_project_root = write_temp_dir();
        OphioliteProject::create(&target_project_root).expect("project should be created");

        let committed = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: None,
            target_survey_asset_id: None,
            coordinate_reference: None,
            bridge_outputs: Vec::new(),
            dry_run: false,
        })
        .expect("raw-source commit should succeed");
        assert_eq!(committed.preserved_raw_sources.len(), 1);
        assert!(committed.imported_assets.is_empty());
        assert!(committed.preserved_raw_sources[0].asset_id.is_some());
        assert!(committed.preserved_raw_sources[0].collection_id.is_some());
        assert!(
            committed
                .issues
                .iter()
                .any(|issue| { issue.code == "project_archive_binding_used" })
        );

        let project = OphioliteProject::open(&target_project_root).expect("project should open");
        let wells = project.list_wells().expect("wells should list");
        let archive_well = wells
            .iter()
            .find(|well| well.name == "Ophiolite Project Archive")
            .expect("archive well should exist");
        let wellbores = project
            .list_wellbores(&archive_well.id)
            .expect("wellbores should list");
        assert_eq!(wellbores.len(), 1);
        assert_eq!(wellbores[0].name, "Ophiolite Project Archive");

        write_file(&root.join("Surfaces/Fault_B.flt"), "flt");
        let second_plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![String::from("fault:Fault_B")],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("second raw-source plan should succeed");

        let second_commit = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan: second_plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: None,
            target_survey_asset_id: None,
            coordinate_reference: None,
            bridge_outputs: Vec::new(),
            dry_run: false,
        })
        .expect("second raw-source commit should succeed");
        assert_eq!(second_commit.preserved_raw_sources.len(), 1);
        assert!(
            second_commit
                .issues
                .iter()
                .any(|issue| issue.code == "project_archive_binding_used")
        );

        let reopened_project =
            OphioliteProject::open(&target_project_root).expect("project should reopen");
        let reopened_wells = reopened_project.list_wells().expect("wells should list");
        let archive_wells = reopened_wells
            .iter()
            .filter(|well| well.name == "Ophiolite Project Archive")
            .collect::<Vec<_>>();
        assert_eq!(archive_wells.len(), 1);
        let reopened_wellbores = reopened_project
            .list_wellbores(&archive_wells[0].id)
            .expect("wellbores should list");
        assert_eq!(reopened_wellbores.len(), 1);
        assert_eq!(reopened_wellbores[0].name, "Ophiolite Project Archive");
    }

    #[test]
    fn commit_imports_opendtect_well_family_into_project() {
        let root = write_temp_dir();
        write_file(
            &root.join(".survey"),
            "\
dTect V7.0.0
Survey Info
!
Name: F3 Demo 2023
Coordinate System.Projection.ID: EPSG`23031
Coordinate System.Projection.Name: ED50 / UTM zone 31N
",
        );
        write_file(
            &root.join("WellInfo/F03-2.well"),
            "\
dTect V6.0
Well
!
Unique Well ID:
Surface coordinate: (619101,6089491)
!
619101 6089491 -30 0
619111 6089501 970 1000
619121 6089511 2110 2140
",
        );
        write_file(
            &root.join("WellInfo/F03-2.wlm"),
            "\
dTect V6.0
Markers
1.Name: FS4
1.Depth along hole: 1200
1.Strat Level: North Sea
1.Color: 255,0,0
2.Name: MFS4
2.Depth along hole: 1800
2.Strat Level: North Sea
2.Color: 0,255,0
",
        );
        write_file(
            &root.join("WellInfo/F03-2.wlt"),
            "\
Depth2Time Model
Name: Integrated Depth/Time Model
!
Type: Velocity model
!
0 0.0
1000 1.0
2140 2.14
",
        );
        write_file(
            &root.join("WellInfo/F03-2.csmdl"),
            "\
Depth2Time Model
!
Type: Checkshot model
!
0 0.0
1932 1.732
1885 1.698
2140 1.95
",
        );
        write_binary_file(
            &root.join("WellInfo/F03-2^1.wll"),
            opendtect_wll_bytes(
                "\
dTect V6.0
Log
Name: Density
Mnemonic: RHOB
Unit of Measure: g/cc
Depth-Unit: m
!
Binary Data
!
",
                &[(30.15, 2.05), (1000.0, 2.25), (2139.6, 2.45)],
            ),
        );

        let plan = plan_vendor_project_import(&VendorProjectPlanRequest {
            vendor: VendorProjectImportVendor::Opendtect,
            project_root: root.to_string_lossy().into_owned(),
            selected_vendor_object_ids: vec![
                String::from("well:F03-2"),
                String::from("well-logs:F03-2"),
                String::from("well-markers:F03-2"),
                String::from("well-time-depth:F03-2"),
            ],
            target_project_root: None,
            target_survey_asset_id: None,
            binding: None,
            coordinate_reference: None,
            runtime_probe: None,
        })
        .expect("plan should succeed");
        assert!(plan.blocking_issues.is_empty());

        let target_project_root = write_temp_dir();
        OphioliteProject::create(&target_project_root).expect("project should be created");
        let binding = sample_binding("F03-2");

        let committed = commit_vendor_project_import(&VendorProjectCommitRequest {
            plan,
            target_project_root: Some(target_project_root.to_string_lossy().into_owned()),
            binding: Some(binding),
            target_survey_asset_id: None,
            coordinate_reference: Some(CoordinateReferenceDescriptor {
                id: Some(String::from("EPSG:23031")),
                name: Some(String::from("ED50 / UTM zone 31N")),
                geodetic_datum: None,
                unit: None,
            }),
            bridge_outputs: Vec::new(),
            dry_run: false,
        })
        .expect("well-family commit should succeed");

        assert_eq!(committed.imported_assets.len(), 5);
        assert!(committed.preserved_raw_sources.is_empty());
        assert_eq!(committed.validation_reports.len(), 4);
        assert!(
            committed
                .imported_assets
                .iter()
                .all(|asset| asset.asset_id.is_some() && asset.collection_id.is_some())
        );

        let collection_names = committed
            .imported_assets
            .iter()
            .filter_map(|asset| asset.collection_name.as_deref())
            .collect::<Vec<_>>();
        assert!(collection_names.contains(&"trajectory"));
        assert!(collection_names.contains(&"Density"));
        assert!(collection_names.contains(&"markers"));
        assert!(collection_names.contains(&"Integrated Depth/Time Model"));
        assert!(collection_names.contains(&"F03-2"));
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should be created");
        }
        fs::write(path, contents).expect("file should be written");
    }

    fn write_binary_file(path: &Path, contents: Vec<u8>) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should be created");
        }
        fs::write(path, contents).expect("binary file should be written");
    }

    fn opendtect_wll_bytes(header: &str, samples: &[(f32, f32)]) -> Vec<u8> {
        let mut bytes = header.as_bytes().to_vec();
        for (depth, value) in samples {
            bytes.extend_from_slice(&depth.to_le_bytes());
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn sample_binding(name: &str) -> AssetBindingInput {
        AssetBindingInput {
            well_name: name.to_string(),
            wellbore_name: name.to_string(),
            uwi: None,
            api: None,
            operator_aliases: Vec::new(),
        }
    }

    fn create_test_survey_asset(project_root: &Path, survey_name: &str) -> AssetId {
        fs::create_dir_all(project_root).expect("project root should exist");
        let store_root = write_temp_dir().join("survey.tbvol");
        let coordinate_reference = CoordinateReferenceDescriptor {
            id: Some(String::from("EPSG:23031")),
            name: Some(String::from("ED50 / UTM zone 31N")),
            geodetic_datum: Some(String::from("ED50")),
            unit: Some(String::from("m")),
        };
        let volume = VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: String::from("store:f3-survey"),
            source: SourceIdentity {
                source_path: PathBuf::from("/tmp/f3-survey.sgy"),
                file_size: 1,
                trace_count: 4,
                samples_per_trace: 3,
                sample_interval_us: 4000,
                sample_format_code: 5,
                sample_data_fidelity: SampleDataFidelity::default(),
                endianness: String::from("little"),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
                geometry: GeometryProvenance {
                    inline_field: HeaderFieldSpec {
                        name: String::from("INLINE_3D"),
                        start_byte: 189,
                        value_type: String::from("I32"),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: String::from("CROSSLINE_3D"),
                        start_byte: 193,
                        value_type: String::from("I32"),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape: [2, 2, 3],
            axes: VolumeAxes::from_time_axis(
                vec![100.0, 101.0],
                vec![200.0, 201.0],
                vec![0.0, 4.0, 8.0],
            ),
            segy_export: None,
            coordinate_reference_binding: Some(CoordinateReferenceBinding {
                detected: Some(coordinate_reference.clone()),
                effective: Some(coordinate_reference.clone()),
                source: CoordinateReferenceSource::ImportManifest,
                notes: Vec::new(),
            }),
            spatial: Some(SurveySpatialDescriptor {
                coordinate_reference: Some(coordinate_reference),
                grid_transform: Some(SurveyGridTransform {
                    origin: ProjectedPoint2 {
                        x: 453_561.805_650,
                        y: 6_780_244.884_530,
                    },
                    inline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                    xline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                }),
                footprint: Some(ProjectedPolygon2 {
                    exterior: vec![
                        ProjectedPoint2 {
                            x: 453_561.805_650,
                            y: 6_780_244.884_530,
                        },
                        ProjectedPoint2 {
                            x: 453_611.805_650,
                            y: 6_780_244.884_530,
                        },
                        ProjectedPoint2 {
                            x: 453_611.805_650,
                            y: 6_780_294.884_530,
                        },
                        ProjectedPoint2 {
                            x: 453_561.805_650,
                            y: 6_780_294.884_530,
                        },
                    ],
                }),
                availability: SurveySpatialAvailability::Available,
                notes: Vec::new(),
            }),
            created_by: String::from("test"),
            processing_lineage: None,
        };
        let data = Array3::from_shape_vec((2, 2, 3), vec![1.0; 12]).expect("shape");
        let manifest = TbvolManifest::new(volume, [2, 2, 3], false);
        create_tbvol_store(&store_root, manifest, &data, None).expect("survey store");

        let mut project =
            OphioliteProject::create(project_root).expect("project should be created");
        let result = project
            .import_seismic_trace_data_store_with_coordinate_reference(
                &store_root,
                &sample_binding(survey_name),
                Some(survey_name),
                None,
            )
            .expect("survey store import should succeed");
        result.asset.id
    }

    fn write_petrel_sample_well(root: &Path, well_name: &str) {
        let wellheader_path = root.join("Wells/Wellheader");
        let mut wellheader = if wellheader_path.is_file() {
            fs::read_to_string(&wellheader_path).expect("wellheader should read")
        } else {
            String::from(
                "WellName   X-Coord   Y-Coord            Top_Depth      Bottom Depth    KB          Symbol\n",
            )
        };
        wellheader.push_str(&format!(
            "{well_name}  456979.063700   6782712.412000     1499.878992    2415.802791     0.000000    4\n"
        ));
        write_file(&wellheader_path, &wellheader);
        write_file(
            &root.join(format!("Wells/{well_name}.dev")),
            &format!(
                "\
# WELL TRACE FROM PETREL
# WELL NAME:              {well_name}
      MD              X             Y            Z          TVD           DX           DY         AZIM         INCL
  1499.878992   456979.063700 6782712.412000 -1499.878992  1499.878992     0.000000     0.000000    99.853422    42.277454
  1500.031292   456979.164600 6782712.395000 -1499.991678  1499.991678     0.100944    -0.017533    99.852733    42.278832
"
            ),
        );
        write_file(
            &root.join(format!("Wells/{well_name}.las")),
            &format!(
                r#"~Version Information
 VERS.                  2.0 : CWLS LOG ASCII STANDARD - VERSION 2.0
 WRAP.                   NO : ONE LINE PER DEPTH STEP
~Well Information
 STRT.M              1000.0 :
 STOP.M              1010.0 :
 STEP.M                10.0 :
 NULL.             -999.2500 :
 WELL.               {well_name} :
 UWI.           TEST-{well_name} :
~Curve Information
 DEPT.M                     : Depth
 GR  .API                   : Gamma Ray
~A
1000.0 80.0
1010.0 81.0
"#
            ),
        );
        let tops_path = root.join("well tops.txt");
        let mut tops = if tops_path.is_file() {
            fs::read_to_string(&tops_path).expect("tops should read")
        } else {
            String::new()
        };
        tops.push_str(&format!("{well_name}\tTop_Tarbert\t1858.55      HORIZON\n"));
        write_file(&tops_path, &tops);

        let tops_prn_path = root.join("well_tops_type.prn");
        let mut tops_prn = if tops_prn_path.is_file() {
            fs::read_to_string(&tops_prn_path).expect("tops prn should read")
        } else {
            String::new()
        };
        tops_prn.push_str(&format!("{well_name}\tTop_Tarbert\t1858.55      HORIZON\n"));
        write_file(&tops_prn_path, &tops_prn);

        let checkshots_path = root.join("CheckShots1.txt");
        let mut checkshots = if checkshots_path.is_file() {
            fs::read_to_string(&checkshots_path).expect("checkshots should read")
        } else {
            String::new()
        };
        checkshots.push_str(&format!(
            "0.00  0.00  {well_name}\n-1504.00  -1456.00  {well_name}\n"
        ));
        write_file(&checkshots_path, &checkshots);
    }

    fn write_temp_dir() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let sequence = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("ophiolite-vendor-import-{unique}-{sequence}"));
        fs::create_dir_all(&root).expect("temp dir should be created");
        root
    }

    fn write_executable(name: &str, contents: &str) -> std::path::PathBuf {
        let path = write_temp_dir().join(name);
        write_file(&path, contents);
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("chmod");
        }
        path
    }
}
