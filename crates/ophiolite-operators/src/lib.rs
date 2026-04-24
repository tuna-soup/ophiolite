use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const OPERATOR_CATALOG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorSubjectKind {
    Log,
    Trajectory,
    TopSet,
    WellMarkerSet,
    PressureObservation,
    DrillingObservation,
    SeismicTraceData,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorFamily {
    LogCompute,
    TrajectoryCompute,
    TopSetCompute,
    WellMarkerCompute,
    PressureCompute,
    DrillingCompute,
    TraceLocalProcessing,
    PostStackNeighborhoodProcessing,
    SubvolumeProcessing,
    GatherProcessing,
    SeismicAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorExecutionKind {
    Immediate,
    Job,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorOutputLifecycle {
    DerivedAsset,
    AnalysisOnly,
    ViewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorStability {
    Internal,
    Preview,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct OperatorContractRef {
    pub schema_id: String,
    pub contract_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct OperatorDocumentation {
    pub short_help: String,
    pub help_markdown: Option<String>,
    pub help_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct OperatorParameterDoc {
    pub name: String,
    pub label: String,
    pub description: String,
    pub value_kind: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub units: Option<String>,
    pub options: Vec<String>,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingPlannerPartitioningHint {
    TileGroup,
    Section,
    GatherGroup,
    FullVolume,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingPlannerCostClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingPlannerParallelEfficiencyClass {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct ProcessingPlannerHintSummary {
    pub preferred_partitioning: ProcessingPlannerPartitioningHint,
    pub requires_full_volume: bool,
    pub checkpoint_safe: bool,
    pub memory_cost_class: ProcessingPlannerCostClass,
    pub cpu_cost_class: ProcessingPlannerCostClass,
    pub io_cost_class: ProcessingPlannerCostClass,
    pub parallel_efficiency_class: ProcessingPlannerParallelEfficiencyClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingSampleDependencyKind {
    Pointwise,
    BoundedWindow,
    WholeTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ProcessingSpatialDependencyKind {
    SingleTrace,
    SectionNeighborhood,
    GatherNeighborhood,
    ExternalVolumePointwise,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct ProcessingDependencyProfileSummary {
    pub deterministic: bool,
    pub sample_dependency: ProcessingSampleDependencyKind,
    pub sample_window_ms_hint: Option<f32>,
    pub spatial_dependency: ProcessingSpatialDependencyKind,
    pub inline_radius: usize,
    pub crossline_radius: usize,
    pub same_section_ephemeral_reuse_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct TraceLocalProcessingCapabilities {
    pub checkpoint_supported: bool,
    pub secondary_volume_input_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct PostStackNeighborhoodProcessingCapabilities {
    pub trace_local_prefix_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct SubvolumeProcessingCapabilities {
    pub trace_local_prefix_supported: bool,
    pub terminal_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct GatherProcessingCapabilities {
    pub trace_local_prefix_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
pub struct SeismicAnalysisCapabilities {
    pub preview_supported: bool,
    pub autopick_output_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorAvailability {
    Available,
    Unavailable { reasons: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct OperatorCatalog {
    pub schema_version: u32,
    pub subject_kind: OperatorSubjectKind,
    pub operators: Vec<OperatorCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct OperatorCatalogEntry {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub group: String,
    pub group_id: String,
    pub description: String,
    pub family: OperatorFamily,
    pub execution_kind: OperatorExecutionKind,
    pub output_lifecycle: OperatorOutputLifecycle,
    pub stability: OperatorStability,
    pub availability: OperatorAvailability,
    pub tags: Vec<String>,
    pub documentation: OperatorDocumentation,
    pub parameter_docs: Vec<OperatorParameterDoc>,
    pub request_contract: OperatorContractRef,
    pub response_contract: OperatorContractRef,
    pub detail: OperatorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct LogComputeDetail {
    pub default_output_mnemonic: String,
    pub output_kind: String,
    pub input_kinds: Vec<String>,
    pub parameter_names: Vec<String>,
    pub binding_parameter_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct TrajectoryComputeDetail {
    pub input_kinds: Vec<String>,
    pub parameter_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct TopSetComputeDetail {
    pub input_kinds: Vec<String>,
    pub parameter_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct WellMarkerComputeDetail {
    pub input_kinds: Vec<String>,
    pub parameter_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct PressureComputeDetail {
    pub input_kinds: Vec<String>,
    pub parameter_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct DrillingComputeDetail {
    pub input_kinds: Vec<String>,
    pub parameter_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct TraceLocalProcessingDetail {
    pub operation_id: String,
    pub scope: String,
    pub layout_compatibility: String,
    pub preview_contract: OperatorContractRef,
    pub checkpoint_supported: bool,
    pub planner_hint_summary: ProcessingPlannerHintSummary,
    pub dependency_profile_summary: ProcessingDependencyProfileSummary,
    pub capabilities: TraceLocalProcessingCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct SubvolumeProcessingDetail {
    pub terminal_operation_id: String,
    pub layout_compatibility: String,
    pub preview_contract: OperatorContractRef,
    pub trace_local_prefix_supported: bool,
    pub planner_hint_summary: ProcessingPlannerHintSummary,
    pub dependency_profile_summary: ProcessingDependencyProfileSummary,
    pub capabilities: SubvolumeProcessingCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct PostStackNeighborhoodProcessingDetail {
    pub operation_id: String,
    pub scope: String,
    pub layout_compatibility: String,
    pub preview_contract: OperatorContractRef,
    pub trace_local_prefix_supported: bool,
    pub planner_hint_summary: ProcessingPlannerHintSummary,
    pub dependency_profile_summary: ProcessingDependencyProfileSummary,
    pub capabilities: PostStackNeighborhoodProcessingCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct GatherProcessingDetail {
    pub operation_id: String,
    pub scope: String,
    pub layout_compatibility: String,
    pub preview_contract: OperatorContractRef,
    pub trace_local_prefix_supported: bool,
    pub planner_hint_summary: ProcessingPlannerHintSummary,
    pub dependency_profile_summary: ProcessingDependencyProfileSummary,
    pub capabilities: GatherProcessingCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
pub struct SeismicAnalysisDetail {
    pub analysis_kind: String,
    pub layout_compatibility: String,
    pub output_kind: String,
    pub planner_hint_summary: ProcessingPlannerHintSummary,
    pub dependency_profile_summary: ProcessingDependencyProfileSummary,
    pub capabilities: SeismicAnalysisCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OperatorDetail {
    LogCompute(LogComputeDetail),
    TrajectoryCompute(TrajectoryComputeDetail),
    TopSetCompute(TopSetComputeDetail),
    WellMarkerCompute(WellMarkerComputeDetail),
    PressureCompute(PressureComputeDetail),
    DrillingCompute(DrillingComputeDetail),
    TraceLocalProcessing(TraceLocalProcessingDetail),
    PostStackNeighborhoodProcessing(PostStackNeighborhoodProcessingDetail),
    SubvolumeProcessing(SubvolumeProcessingDetail),
    GatherProcessing(GatherProcessingDetail),
    SeismicAnalysis(SeismicAnalysisDetail),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_schema_version_is_stable() {
        assert_eq!(OPERATOR_CATALOG_SCHEMA_VERSION, 1);
    }
}
