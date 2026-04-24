use ophiolite_operators::{
    GatherProcessingDetail, OPERATOR_CATALOG_SCHEMA_VERSION, OperatorAvailability, OperatorCatalog,
    OperatorCatalogEntry, OperatorContractRef, OperatorDetail, OperatorDocumentation,
    OperatorExecutionKind, OperatorFamily, OperatorOutputLifecycle, OperatorParameterDoc,
    OperatorStability, OperatorSubjectKind, PostStackNeighborhoodProcessingDetail,
    ProcessingPlannerHintSummary as ProcessingPlannerHints, SeismicAnalysisDetail,
    SubvolumeProcessingDetail, TraceLocalProcessingDetail,
};

use crate::{
    CapabilityAvailability, CapabilityContractSet, CapabilityDetail, CapabilityDocumentation,
    CapabilityIsolation, CapabilityKind, CapabilityLoadPolicy, CapabilityRecord,
    CapabilityRegistry, CapabilitySource, CapabilityStability, OperatorCapabilityDetail,
    SeismicLayout, SeismicTraceDataDescriptor,
};

use super::domain::DatasetId;
use super::models::VelocityFunctionSource;
use super::operations::{GatherRequest, GatherSelector};
use super::processing::{
    FrequencyPhaseMode, FrequencyWindowShape, GatherInterpolationMode, GatherProcessingOperation,
    LocalVolumeStatistic, PostStackNeighborhoodProcessingOperation, ProcessingLayoutCompatibility,
    SubvolumeCropOperation, TraceLocalProcessingOperation, TraceLocalVolumeArithmeticOperator,
    velocity_scan_catalog_metadata,
};

const SEISMIC_OPERATOR_CONTRACT_SCHEMA_ID: &str = "ophiolite.seismic.operations.v1";

struct SeismicCatalogRegistration {
    id: String,
    name: String,
    group: &'static str,
    group_id: &'static str,
    description: String,
    execution_kind: OperatorExecutionKind,
    output_lifecycle: OperatorOutputLifecycle,
    stability: OperatorStability,
    compatibility: ProcessingLayoutCompatibility,
    tags: Vec<String>,
    documentation: OperatorDocumentation,
    parameter_docs: Vec<OperatorParameterDoc>,
    request_contract_id: &'static str,
    response_contract_id: &'static str,
    detail: SeismicCatalogDetailRegistration,
}

enum SeismicCatalogDetailRegistration {
    TraceLocal(TraceLocalProcessingDetail),
    PostStackNeighborhood(PostStackNeighborhoodProcessingDetail),
    Subvolume(SubvolumeProcessingDetail),
    Gather(GatherProcessingDetail),
    SeismicAnalysis(SeismicAnalysisDetail),
}

impl SeismicCatalogDetailRegistration {
    fn family(&self) -> OperatorFamily {
        match self {
            Self::TraceLocal(_) => OperatorFamily::TraceLocalProcessing,
            Self::PostStackNeighborhood(_) => OperatorFamily::PostStackNeighborhoodProcessing,
            Self::Subvolume(_) => OperatorFamily::SubvolumeProcessing,
            Self::Gather(_) => OperatorFamily::GatherProcessing,
            Self::SeismicAnalysis(_) => OperatorFamily::SeismicAnalysis,
        }
    }

    fn into_detail(self) -> OperatorDetail {
        match self {
            Self::TraceLocal(detail) => OperatorDetail::TraceLocalProcessing(detail),
            Self::PostStackNeighborhood(detail) => {
                OperatorDetail::PostStackNeighborhoodProcessing(detail)
            }
            Self::Subvolume(detail) => OperatorDetail::SubvolumeProcessing(detail),
            Self::Gather(detail) => OperatorDetail::GatherProcessing(detail),
            Self::SeismicAnalysis(detail) => OperatorDetail::SeismicAnalysis(detail),
        }
    }
}

fn register_seismic_operator(
    layout: SeismicLayout,
    registration: SeismicCatalogRegistration,
) -> OperatorCatalogEntry {
    let family = registration.detail.family();
    OperatorCatalogEntry {
        id: registration.id,
        provider: "ophiolite".to_string(),
        name: registration.name,
        group: registration.group.to_string(),
        group_id: registration.group_id.to_string(),
        description: registration.description,
        family,
        execution_kind: registration.execution_kind,
        output_lifecycle: registration.output_lifecycle,
        stability: registration.stability,
        availability: availability_for_layout(registration.compatibility, layout),
        tags: registration.tags,
        documentation: registration.documentation,
        parameter_docs: registration.parameter_docs,
        request_contract: contract_ref(registration.request_contract_id),
        response_contract: contract_ref(registration.response_contract_id),
        detail: registration.detail.into_detail(),
    }
}

pub fn operator_catalog_for_trace_data(descriptor: &SeismicTraceDataDescriptor) -> OperatorCatalog {
    let mut operators = Vec::new();
    operators.extend(trace_local_operator_entries(descriptor.layout));
    operators.extend(post_stack_neighborhood_operator_entries(descriptor.layout));
    operators.push(subvolume_operator_entry(descriptor.layout));
    operators.extend(gather_operator_entries(descriptor.layout));
    operators.push(velocity_scan_operator_entry(descriptor.layout));

    OperatorCatalog {
        schema_version: OPERATOR_CATALOG_SCHEMA_VERSION,
        subject_kind: OperatorSubjectKind::SeismicTraceData,
        operators,
    }
}

pub fn operator_capability_registry_for_catalog(catalog: &OperatorCatalog) -> CapabilityRegistry {
    debug_assert_eq!(catalog.subject_kind, OperatorSubjectKind::SeismicTraceData);

    let mut registry = CapabilityRegistry::new();
    registry.extend(
        catalog
            .operators
            .iter()
            .map(|entry| operator_capability_record_for_entry(&catalog.subject_kind, entry)),
    );
    registry
}

pub fn operator_capability_record_for_entry(
    subject_kind: &OperatorSubjectKind,
    entry: &OperatorCatalogEntry,
) -> CapabilityRecord {
    CapabilityRecord {
        id: entry.id.clone(),
        kind: CapabilityKind::Operator,
        source: CapabilitySource::BuiltIn,
        provider: entry.provider.clone(),
        name: entry.name.clone(),
        summary: Some(entry.description.clone()),
        version: None,
        stability: capability_stability_for_operator(entry.stability.clone()),
        availability: capability_availability_for_operator(&entry.availability),
        tags: entry.tags.clone(),
        documentation: vec![CapabilityDocumentation {
            short_help: entry.documentation.short_help.clone(),
            help_markdown: entry.documentation.help_markdown.clone(),
            help_url: entry.documentation.help_url.clone(),
        }],
        load_policy: CapabilityLoadPolicy::Never,
        isolation: CapabilityIsolation::InProcess,
        permissions: Vec::new(),
        bindings: Vec::new(),
        host_compatibility: Vec::new(),
        contracts: CapabilityContractSet {
            request: Some(capability_contract_ref(&entry.request_contract)),
            response: Some(capability_contract_ref(&entry.response_contract)),
        },
        artifacts: Vec::new(),
        detail: CapabilityDetail::Operator(OperatorCapabilityDetail {
            family_id: capability_family_id(entry.family.clone()).to_string(),
            subject_kind: capability_subject_kind_id(subject_kind).to_string(),
            execution_kind: capability_execution_kind_id(entry.execution_kind.clone()).to_string(),
            output_lifecycle: capability_output_lifecycle_id(entry.output_lifecycle.clone())
                .to_string(),
            deterministic: capability_operator_deterministic(&entry.detail),
            parameter_schema_id: None,
        }),
    }
}

pub fn trace_local_operator_planner_hints(
    operation: &TraceLocalProcessingOperation,
) -> ProcessingPlannerHints {
    operation.catalog_metadata().planner_hint_summary
}

pub fn subvolume_operator_planner_hints(
    operation: &SubvolumeCropOperation,
) -> ProcessingPlannerHints {
    operation.catalog_metadata().planner_hint_summary
}

pub fn post_stack_neighborhood_operator_planner_hints(
    operation: &PostStackNeighborhoodProcessingOperation,
) -> ProcessingPlannerHints {
    operation.catalog_metadata().planner_hint_summary
}

pub fn gather_operator_planner_hints(
    operation: &GatherProcessingOperation,
) -> ProcessingPlannerHints {
    operation.catalog_metadata().planner_hint_summary
}

fn trace_local_operator_entries(layout: SeismicLayout) -> Vec<OperatorCatalogEntry> {
    trace_local_operator_prototypes()
        .into_iter()
        .map(|operation| {
            let metadata = operation.catalog_metadata();
            register_seismic_operator(
                layout,
                SeismicCatalogRegistration {
                    id: metadata.operation_id.to_string(),
                    name: title_case(metadata.operation_id),
                    group: "Trace Local",
                    group_id: "trace_local",
                    description: format!(
                        "Trace-local seismic processing operator '{}' for {} datasets.",
                        metadata.operation_id,
                        metadata.compatibility.label()
                    ),
                    execution_kind: OperatorExecutionKind::Job,
                    output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                    stability: OperatorStability::Preview,
                    compatibility: metadata.compatibility,
                    tags: vec!["seismic".to_string(), "trace_local".to_string()],
                    documentation: trace_local_documentation(&operation),
                    parameter_docs: trace_local_parameter_docs(&operation),
                    request_contract_id: "run_trace_local_processing_request",
                    response_contract_id: "run_trace_local_processing_response",
                    detail: SeismicCatalogDetailRegistration::TraceLocal(
                        TraceLocalProcessingDetail {
                            operation_id: metadata.operation_id.to_string(),
                            scope: metadata.scope.label().to_string(),
                            layout_compatibility: metadata.compatibility.label().to_string(),
                            preview_contract: contract_ref(
                                "preview_trace_local_processing_request",
                            ),
                            checkpoint_supported: metadata.capabilities.checkpoint_supported,
                            planner_hint_summary: metadata.planner_hint_summary,
                            dependency_profile_summary: metadata.dependency_profile_summary,
                            capabilities: metadata.capabilities,
                        },
                    ),
                },
            )
        })
        .collect()
}

fn subvolume_operator_entry(layout: SeismicLayout) -> OperatorCatalogEntry {
    let operation = SubvolumeCropOperation {
        inline_min: 0,
        inline_max: 0,
        xline_min: 0,
        xline_max: 0,
        z_min_ms: 0.0,
        z_max_ms: 0.0,
    };
    let metadata = operation.catalog_metadata();
    register_seismic_operator(
        layout,
        SeismicCatalogRegistration {
            id: "crop".to_string(),
            name: "Crop".to_string(),
            group: "Subvolume",
            group_id: "subvolume",
            description: "Terminal subvolume derivation that crops a post-stack survey."
                .to_string(),
            execution_kind: OperatorExecutionKind::Job,
            output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
            stability: OperatorStability::Preview,
            compatibility: metadata.compatibility,
            tags: vec![
                "seismic".to_string(),
                "subvolume".to_string(),
                "geometry".to_string(),
            ],
            documentation: OperatorDocumentation {
                short_help: "Write a strict post-stack subvolume bounded by inline, xline, and vertical windows.".to_string(),
                help_markdown: Some(
                    "Crop is a terminal subvolume derivation. It preserves sample values inside the requested bounds and emits a new geometry-limited seismic asset.".to_string(),
                ),
                help_url: None,
            },
            parameter_docs: vec![
                number_parameter_doc("inline_min", "Inline Min", "Inclusive minimum inline index for the output subvolume.", None, Some("survey index"), None, None),
                number_parameter_doc("inline_max", "Inline Max", "Inclusive maximum inline index for the output subvolume.", None, Some("survey index"), None, None),
                number_parameter_doc("xline_min", "Xline Min", "Inclusive minimum crossline index for the output subvolume.", None, Some("survey index"), None, None),
                number_parameter_doc("xline_max", "Xline Max", "Inclusive maximum crossline index for the output subvolume.", None, Some("survey index"), None, None),
                number_parameter_doc("z_min_ms", "Z Min", "Inclusive minimum vertical bound for the output subvolume.", None, Some("ms"), Some("dataset minimum"), Some("dataset maximum")),
                number_parameter_doc("z_max_ms", "Z Max", "Inclusive maximum vertical bound for the output subvolume.", None, Some("ms"), Some("dataset minimum"), Some("dataset maximum")),
            ],
            request_contract_id: "run_subvolume_processing_request",
            response_contract_id: "run_subvolume_processing_response",
            detail: SeismicCatalogDetailRegistration::Subvolume(SubvolumeProcessingDetail {
                terminal_operation_id: metadata.terminal_operation_id.to_string(),
                layout_compatibility: metadata.compatibility.label().to_string(),
                preview_contract: contract_ref("preview_subvolume_processing_request"),
                trace_local_prefix_supported: metadata.capabilities.trace_local_prefix_supported,
                planner_hint_summary: metadata.planner_hint_summary,
                dependency_profile_summary: metadata.dependency_profile_summary,
                capabilities: metadata.capabilities,
            }),
        },
    )
}

fn post_stack_neighborhood_operator_entries(layout: SeismicLayout) -> Vec<OperatorCatalogEntry> {
    post_stack_neighborhood_operator_prototypes()
        .into_iter()
        .map(|operation| {
            let metadata = operation.catalog_metadata();
            register_seismic_operator(
                layout,
                SeismicCatalogRegistration {
                    id: metadata.operation_id.to_string(),
                    name: title_case(metadata.operation_id),
                    group: "Post-Stack Neighborhood",
                    group_id: "post_stack_neighborhood",
                    description: format!(
                        "Post-stack neighborhood seismic operator '{}' for {} datasets.",
                        metadata.operation_id,
                        metadata.compatibility.label()
                    ),
                    execution_kind: OperatorExecutionKind::Job,
                    output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                    stability: OperatorStability::Preview,
                    compatibility: metadata.compatibility,
                    tags: vec!["seismic".to_string(), "post_stack_neighborhood".to_string()],
                    documentation: post_stack_neighborhood_documentation(&operation),
                    parameter_docs: post_stack_neighborhood_parameter_docs(&operation),
                    request_contract_id: "run_post_stack_neighborhood_processing_request",
                    response_contract_id: "run_post_stack_neighborhood_processing_response",
                    detail: SeismicCatalogDetailRegistration::PostStackNeighborhood(
                        PostStackNeighborhoodProcessingDetail {
                            operation_id: metadata.operation_id.to_string(),
                            scope: metadata.scope.label().to_string(),
                            layout_compatibility: metadata.compatibility.label().to_string(),
                            preview_contract: contract_ref(
                                "preview_post_stack_neighborhood_processing_request",
                            ),
                            trace_local_prefix_supported: metadata
                                .capabilities
                                .trace_local_prefix_supported,
                            planner_hint_summary: metadata.planner_hint_summary,
                            dependency_profile_summary: metadata.dependency_profile_summary,
                            capabilities: metadata.capabilities,
                        },
                    ),
                },
            )
        })
        .collect()
}

fn gather_operator_entries(layout: SeismicLayout) -> Vec<OperatorCatalogEntry> {
    gather_operator_prototypes()
        .into_iter()
        .map(|operation| {
            let metadata = operation.catalog_metadata();
            register_seismic_operator(
                layout,
                SeismicCatalogRegistration {
                    id: metadata.operation_id.to_string(),
                    name: title_case(metadata.operation_id),
                    group: "Gather",
                    group_id: "gather",
                    description: format!(
                        "Gather-native seismic processing operator '{}' for {} datasets.",
                        metadata.operation_id,
                        metadata.compatibility.label()
                    ),
                    execution_kind: OperatorExecutionKind::Job,
                    output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                    stability: OperatorStability::Preview,
                    compatibility: metadata.compatibility,
                    tags: vec!["seismic".to_string(), "gather".to_string()],
                    documentation: gather_documentation(&operation),
                    parameter_docs: gather_parameter_docs(&operation),
                    request_contract_id: "run_gather_processing_request",
                    response_contract_id: "run_gather_processing_response",
                    detail: SeismicCatalogDetailRegistration::Gather(GatherProcessingDetail {
                        operation_id: metadata.operation_id.to_string(),
                        scope: metadata.scope.label().to_string(),
                        layout_compatibility: metadata.compatibility.label().to_string(),
                        preview_contract: contract_ref("preview_gather_processing_request"),
                        trace_local_prefix_supported: metadata
                            .capabilities
                            .trace_local_prefix_supported,
                        planner_hint_summary: metadata.planner_hint_summary,
                        dependency_profile_summary: metadata.dependency_profile_summary,
                        capabilities: metadata.capabilities,
                    }),
                },
            )
        })
        .collect()
}

fn velocity_scan_operator_entry(layout: SeismicLayout) -> OperatorCatalogEntry {
    let metadata = velocity_scan_catalog_metadata();
    let _sample_gather_request = GatherRequest {
        dataset_id: DatasetId("dataset:placeholder".to_string()),
        selector: GatherSelector::Ordinal { index: 0 },
    };

    register_seismic_operator(
        layout,
        SeismicCatalogRegistration {
            id: "velocity_scan".to_string(),
            name: "Velocity Scan".to_string(),
            group: "Analysis",
            group_id: "analysis",
            description: "Prestack offset-gather semblance scan with optional autopick output."
                .to_string(),
            execution_kind: OperatorExecutionKind::Immediate,
            output_lifecycle: OperatorOutputLifecycle::AnalysisOnly,
            stability: OperatorStability::Preview,
            compatibility: metadata.compatibility,
            tags: vec![
                "seismic".to_string(),
                "analysis".to_string(),
                "velocity".to_string(),
            ],
            documentation: OperatorDocumentation {
                short_help: "Compute a prestack offset-gather semblance panel with optional picked velocity guidance.".to_string(),
                help_markdown: Some(
                    "Velocity Scan is an analysis workflow over offset gathers. It does not materialize a derived seismic asset; instead it returns semblance-style analysis output for interpretation and picking.".to_string(),
                ),
                help_url: None,
            },
            parameter_docs: vec![
                parameter_doc(
                    "gather_selector",
                    "Gather Selector",
                    "Selects the prestack gather to analyze.",
                    "gather_selector",
                    true,
                    None,
                    None,
                    Vec::new(),
                    None,
                    None,
                ),
                number_parameter_doc("velocity_min_m_per_s", "Velocity Min", "Minimum scan velocity.", None, Some("m/s"), None, None),
                number_parameter_doc("velocity_max_m_per_s", "Velocity Max", "Maximum scan velocity.", None, Some("m/s"), None, None),
                number_parameter_doc("velocity_step_m_per_s", "Velocity Step", "Sampling increment in velocity space.", None, Some("m/s"), Some("positive"), None),
            ],
            request_contract_id: "velocity_scan_request",
            response_contract_id: "velocity_scan_response",
            detail: SeismicCatalogDetailRegistration::SeismicAnalysis(SeismicAnalysisDetail {
                analysis_kind: metadata.analysis_kind.to_string(),
                layout_compatibility: metadata.compatibility.label().to_string(),
                output_kind: metadata.output_kind.to_string(),
                planner_hint_summary: metadata.planner_hint_summary,
                dependency_profile_summary: metadata.dependency_profile_summary,
                capabilities: metadata.capabilities,
            }),
        },
    )
}

fn availability_for_layout(
    compatibility: ProcessingLayoutCompatibility,
    layout: SeismicLayout,
) -> OperatorAvailability {
    if compatibility.supports_layout(layout) {
        OperatorAvailability::Available
    } else {
        OperatorAvailability::Unavailable {
            reasons: vec![format!(
                "requires {} but asset layout is {:?}",
                compatibility.label(),
                layout
            )],
        }
    }
}

fn capability_stability_for_operator(stability: OperatorStability) -> CapabilityStability {
    match stability {
        OperatorStability::Internal => CapabilityStability::Internal,
        OperatorStability::Preview => CapabilityStability::Preview,
        OperatorStability::Stable => CapabilityStability::Stable,
    }
}

fn capability_availability_for_operator(
    availability: &OperatorAvailability,
) -> CapabilityAvailability {
    match availability {
        OperatorAvailability::Available => CapabilityAvailability::Available,
        OperatorAvailability::Unavailable { reasons } => CapabilityAvailability::Unavailable {
            reasons: reasons.clone(),
        },
    }
}

fn capability_contract_ref(contract: &OperatorContractRef) -> String {
    format!("{}#{}", contract.schema_id, contract.contract_id)
}

fn capability_subject_kind_id(subject_kind: &OperatorSubjectKind) -> &'static str {
    match subject_kind {
        OperatorSubjectKind::Log => "log",
        OperatorSubjectKind::Trajectory => "trajectory",
        OperatorSubjectKind::TopSet => "top_set",
        OperatorSubjectKind::WellMarkerSet => "well_marker_set",
        OperatorSubjectKind::PressureObservation => "pressure_observation",
        OperatorSubjectKind::DrillingObservation => "drilling_observation",
        OperatorSubjectKind::SeismicTraceData => "seismic_trace_data",
    }
}

fn capability_family_id(family: OperatorFamily) -> &'static str {
    match family {
        OperatorFamily::LogCompute => "log_compute",
        OperatorFamily::TrajectoryCompute => "trajectory_compute",
        OperatorFamily::TopSetCompute => "top_set_compute",
        OperatorFamily::WellMarkerCompute => "well_marker_compute",
        OperatorFamily::PressureCompute => "pressure_compute",
        OperatorFamily::DrillingCompute => "drilling_compute",
        OperatorFamily::TraceLocalProcessing => "trace_local_processing",
        OperatorFamily::PostStackNeighborhoodProcessing => "post_stack_neighborhood_processing",
        OperatorFamily::SubvolumeProcessing => "subvolume_processing",
        OperatorFamily::GatherProcessing => "gather_processing",
        OperatorFamily::SeismicAnalysis => "seismic_analysis",
    }
}

fn capability_execution_kind_id(kind: OperatorExecutionKind) -> &'static str {
    match kind {
        OperatorExecutionKind::Immediate => "immediate",
        OperatorExecutionKind::Job => "job",
    }
}

fn capability_output_lifecycle_id(lifecycle: OperatorOutputLifecycle) -> &'static str {
    match lifecycle {
        OperatorOutputLifecycle::DerivedAsset => "derived_asset",
        OperatorOutputLifecycle::AnalysisOnly => "analysis_only",
        OperatorOutputLifecycle::ViewOnly => "view_only",
    }
}

fn capability_operator_deterministic(detail: &OperatorDetail) -> bool {
    match detail {
        OperatorDetail::TraceLocalProcessing(detail) => {
            detail.dependency_profile_summary.deterministic
        }
        OperatorDetail::PostStackNeighborhoodProcessing(detail) => {
            detail.dependency_profile_summary.deterministic
        }
        OperatorDetail::SubvolumeProcessing(detail) => {
            detail.dependency_profile_summary.deterministic
        }
        OperatorDetail::GatherProcessing(detail) => detail.dependency_profile_summary.deterministic,
        OperatorDetail::SeismicAnalysis(detail) => detail.dependency_profile_summary.deterministic,
        OperatorDetail::LogCompute(_)
        | OperatorDetail::TrajectoryCompute(_)
        | OperatorDetail::TopSetCompute(_)
        | OperatorDetail::WellMarkerCompute(_)
        | OperatorDetail::PressureCompute(_)
        | OperatorDetail::DrillingCompute(_) => {
            unreachable!("seismic capability projection only supports seismic operator details")
        }
    }
}

fn contract_ref(contract_id: &str) -> OperatorContractRef {
    OperatorContractRef {
        schema_id: SEISMIC_OPERATOR_CONTRACT_SCHEMA_ID.to_string(),
        contract_id: contract_id.to_string(),
    }
}

fn trace_local_operator_prototypes() -> Vec<TraceLocalProcessingOperation> {
    vec![
        TraceLocalProcessingOperation::AmplitudeScalar { factor: 1.0 },
        TraceLocalProcessingOperation::TraceRmsNormalize,
        TraceLocalProcessingOperation::AgcRms { window_ms: 500.0 },
        TraceLocalProcessingOperation::PhaseRotation { angle_degrees: 0.0 },
        TraceLocalProcessingOperation::Envelope,
        TraceLocalProcessingOperation::InstantaneousPhase,
        TraceLocalProcessingOperation::InstantaneousFrequency,
        TraceLocalProcessingOperation::Sweetness,
        TraceLocalProcessingOperation::LowpassFilter {
            f3_hz: 10.0,
            f4_hz: 20.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        },
        TraceLocalProcessingOperation::HighpassFilter {
            f1_hz: 5.0,
            f2_hz: 10.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        },
        TraceLocalProcessingOperation::BandpassFilter {
            f1_hz: 5.0,
            f2_hz: 10.0,
            f3_hz: 50.0,
            f4_hz: 60.0,
            phase: FrequencyPhaseMode::Zero,
            window: FrequencyWindowShape::CosineTaper,
        },
        TraceLocalProcessingOperation::VolumeArithmetic {
            operator: TraceLocalVolumeArithmeticOperator::Add,
            secondary_store_path: String::new(),
        },
    ]
}

fn gather_operator_prototypes() -> Vec<GatherProcessingOperation> {
    let velocity_model = VelocityFunctionSource::ConstantVelocity {
        velocity_m_per_s: 2000.0,
    };
    vec![
        GatherProcessingOperation::NmoCorrection {
            velocity_model: velocity_model.clone(),
            interpolation: GatherInterpolationMode::Linear,
        },
        GatherProcessingOperation::StretchMute {
            velocity_model,
            max_stretch_ratio: 0.3,
        },
        GatherProcessingOperation::OffsetMute {
            min_offset: Some(0.0),
            max_offset: Some(2500.0),
        },
    ]
}

fn post_stack_neighborhood_operator_prototypes() -> Vec<PostStackNeighborhoodProcessingOperation> {
    vec![
        PostStackNeighborhoodProcessingOperation::Similarity {
            window: crate::PostStackNeighborhoodWindow {
                gate_ms: 24.0,
                inline_stepout: 1,
                xline_stepout: 1,
            },
        },
        PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
            window: crate::PostStackNeighborhoodWindow {
                gate_ms: 24.0,
                inline_stepout: 1,
                xline_stepout: 1,
            },
            statistic: LocalVolumeStatistic::Rms,
        },
        PostStackNeighborhoodProcessingOperation::Dip {
            window: crate::PostStackNeighborhoodWindow {
                gate_ms: 24.0,
                inline_stepout: 1,
                xline_stepout: 1,
            },
            output: crate::NeighborhoodDipOutput::Inline,
        },
    ]
}

fn title_case(id: &str) -> String {
    id.split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut label = first.to_uppercase().collect::<String>();
                    label.push_str(chars.as_str());
                    label
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parameter_doc(
    name: &str,
    label: &str,
    description: &str,
    value_kind: &str,
    required: bool,
    default_value: Option<&str>,
    units: Option<&str>,
    options: Vec<String>,
    minimum: Option<&str>,
    maximum: Option<&str>,
) -> OperatorParameterDoc {
    OperatorParameterDoc {
        name: name.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        value_kind: value_kind.to_string(),
        required,
        default_value: default_value.map(str::to_string),
        units: units.map(str::to_string),
        options,
        minimum: minimum.map(str::to_string),
        maximum: maximum.map(str::to_string),
    }
}

fn number_parameter_doc(
    name: &str,
    label: &str,
    description: &str,
    default_value: Option<&str>,
    units: Option<&str>,
    minimum: Option<&str>,
    maximum: Option<&str>,
) -> OperatorParameterDoc {
    parameter_doc(
        name,
        label,
        description,
        "number",
        true,
        default_value,
        units,
        Vec::new(),
        minimum,
        maximum,
    )
}

fn enum_parameter_doc(
    name: &str,
    label: &str,
    description: &str,
    default_value: Option<&str>,
    options: &[&str],
) -> OperatorParameterDoc {
    parameter_doc(
        name,
        label,
        description,
        "enum",
        true,
        default_value,
        None,
        options.iter().map(|option| option.to_string()).collect(),
        None,
        None,
    )
}

fn trace_local_documentation(operation: &TraceLocalProcessingOperation) -> OperatorDocumentation {
    let (short_help, help_markdown) = match operation {
        TraceLocalProcessingOperation::AmplitudeScalar { .. } => (
            "Scale each trace by a constant amplitude factor.",
            "Amplitude Scalar is a trace-preserving conditioning step. Use it when you need predictable linear scaling before later spectral or attribute operations.",
        ),
        TraceLocalProcessingOperation::TraceRmsNormalize => (
            "Normalize each trace to unit RMS amplitude.",
            "Trace RMS Normalize balances traces independently by their RMS amplitude. This is useful for visual balancing, but it is not amplitude preserving.",
        ),
        TraceLocalProcessingOperation::AgcRms { .. } => (
            "Apply centered moving-window RMS automatic gain control.",
            "RMS AGC boosts weaker events and suppresses stronger events within a local window. Treat it as conditioning rather than amplitude-faithful processing.",
        ),
        TraceLocalProcessingOperation::PhaseRotation { .. } => (
            "Rotate trace phase by a constant angle in degrees.",
            "Phase Rotation changes wavelet character without changing the amplitude spectrum magnitude. Use it for phase testing or alignment workflows.",
        ),
        TraceLocalProcessingOperation::Envelope => (
            "Compute analytic-trace magnitude.",
            "Envelope returns the magnitude of the analytic signal derived from the trace and its Hilbert transform. It is commonly used as a reflection-strength style attribute.",
        ),
        TraceLocalProcessingOperation::InstantaneousPhase => (
            "Compute wrapped analytic-trace phase in degrees.",
            "Instantaneous Phase reports the wrapped phase of the analytic signal over [-180, 180]. It is useful for phase character and continuity work.",
        ),
        TraceLocalProcessingOperation::InstantaneousFrequency => (
            "Compute stabilized analytic-signal instantaneous frequency.",
            "Instantaneous Frequency estimates local apparent frequency in Hz from the analytic signal. Real seismic data can still produce noisy or negative values.",
        ),
        TraceLocalProcessingOperation::Sweetness => (
            "Compute sweetness from envelope and instantaneous frequency.",
            "Sweetness combines reflection strength with a stabilized frequency measure. It highlights strong low-frequency responses and is sensitive to the stability floor applied to instantaneous frequency.",
        ),
        TraceLocalProcessingOperation::LowpassFilter { .. } => (
            "Apply a zero-phase FFT lowpass with a cosine taper.",
            "Lowpass Filter attenuates frequencies above the high-cut transition. The current runtime uses zero-phase spectral filtering with cosine tapering.",
        ),
        TraceLocalProcessingOperation::HighpassFilter { .. } => (
            "Apply a zero-phase FFT highpass with a cosine taper.",
            "Highpass Filter attenuates low frequencies and drift below the low-cut transition. The current runtime uses zero-phase spectral filtering with cosine tapering.",
        ),
        TraceLocalProcessingOperation::BandpassFilter { .. } => (
            "Apply a zero-phase FFT bandpass with cosine tapers.",
            "Bandpass Filter preserves energy inside the pass band and tapers both low-cut and high-cut transitions. Corner ordering must remain physically valid for the sample rate.",
        ),
        TraceLocalProcessingOperation::VolumeArithmetic { .. } => (
            "Combine the active volume with a second compatible volume sample by sample.",
            "Volume Arithmetic is still trace-local, but it requires a second geometry-compatible post-stack volume. Use it for difference, sum, product, or ratio style workflows.",
        ),
    };
    OperatorDocumentation {
        short_help: short_help.to_string(),
        help_markdown: Some(help_markdown.to_string()),
        help_url: None,
    }
}

#[cfg(test)]
mod capability_projection_tests {
    use super::*;
    use crate::{
        SeismicAssetId, SeismicAxisRole, SeismicDimensionDescriptor, SeismicOrganization,
        SeismicSampleDomain, SeismicStackingState, SeismicUnits,
    };

    fn sample_trace_data_descriptor() -> SeismicTraceDataDescriptor {
        SeismicTraceDataDescriptor {
            id: SeismicAssetId("dataset:test".to_string()),
            label: "Test Survey".to_string(),
            stacking_state: SeismicStackingState::PostStack,
            organization: SeismicOrganization::BinnedGrid,
            layout: SeismicLayout::PostStack3D,
            gather_axis_kind: None,
            dimensions: vec![
                SeismicDimensionDescriptor {
                    role: SeismicAxisRole::Inline,
                    label: "inline".to_string(),
                    start: Some(100.0),
                    step: Some(1.0),
                    count: 2,
                    values: None,
                    unit: None,
                },
                SeismicDimensionDescriptor {
                    role: SeismicAxisRole::Crossline,
                    label: "crossline".to_string(),
                    start: Some(200.0),
                    step: Some(1.0),
                    count: 2,
                    values: None,
                    unit: None,
                },
                SeismicDimensionDescriptor {
                    role: SeismicAxisRole::Sample,
                    label: "time".to_string(),
                    start: Some(0.0),
                    step: Some(4.0),
                    count: 2,
                    values: None,
                    unit: Some("ms".to_string()),
                },
            ],
            chunk_shape: Some(vec![2, 2, 2]),
            sample_domain: SeismicSampleDomain::Time,
            units: SeismicUnits {
                sample: "ms".to_string(),
                amplitude: None,
            },
            bin_grid: None,
        }
    }

    #[test]
    fn projects_trace_data_catalog_into_discovery_capabilities() {
        let catalog = operator_catalog_for_trace_data(&sample_trace_data_descriptor());
        let registry = operator_capability_registry_for_catalog(&catalog);

        assert_eq!(registry.records.len(), catalog.operators.len());

        let trace_local = registry.get("amplitude_scalar").unwrap();
        assert_eq!(trace_local.kind, CapabilityKind::Operator);
        assert_eq!(trace_local.source, CapabilitySource::BuiltIn);
        assert_eq!(trace_local.load_policy, CapabilityLoadPolicy::Never);
        assert_eq!(trace_local.isolation, CapabilityIsolation::InProcess);
        assert_eq!(
            trace_local.contracts.request.as_deref(),
            Some("ophiolite.seismic.operations.v1#run_trace_local_processing_request")
        );
        match &trace_local.detail {
            CapabilityDetail::Operator(detail) => {
                assert_eq!(detail.family_id, "trace_local_processing");
                assert_eq!(detail.subject_kind, "seismic_trace_data");
                assert_eq!(detail.execution_kind, "job");
                assert_eq!(detail.output_lifecycle, "derived_asset");
                assert!(detail.deterministic);
                assert_eq!(detail.parameter_schema_id, None);
            }
            other => panic!("unexpected capability detail: {other:?}"),
        }

        let analysis = registry.get("velocity_scan").unwrap();
        match &analysis.detail {
            CapabilityDetail::Operator(detail) => {
                assert_eq!(detail.family_id, "seismic_analysis");
                assert_eq!(detail.execution_kind, "immediate");
                assert_eq!(detail.output_lifecycle, "analysis_only");
            }
            other => panic!("unexpected capability detail: {other:?}"),
        }
    }
}

fn trace_local_parameter_docs(
    operation: &TraceLocalProcessingOperation,
) -> Vec<OperatorParameterDoc> {
    match operation {
        TraceLocalProcessingOperation::AmplitudeScalar { .. } => vec![number_parameter_doc(
            "factor",
            "Factor",
            "Linear multiplier applied to every trace sample.",
            Some("1.0"),
            None,
            None,
            None,
        )],
        TraceLocalProcessingOperation::AgcRms { .. } => vec![number_parameter_doc(
            "window_ms",
            "Window",
            "Centered RMS window length used for AGC balancing.",
            Some("500.0"),
            Some("ms"),
            Some("positive"),
            None,
        )],
        TraceLocalProcessingOperation::PhaseRotation { .. } => vec![number_parameter_doc(
            "angle_degrees",
            "Angle",
            "Constant phase rotation angle applied to the trace.",
            Some("0.0"),
            Some("degrees"),
            Some("-180"),
            Some("180"),
        )],
        TraceLocalProcessingOperation::LowpassFilter { .. } => vec![
            number_parameter_doc(
                "f3_hz",
                "F3",
                "Pass-band corner before tapering to zero.",
                Some("10.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            number_parameter_doc(
                "f4_hz",
                "F4",
                "Stop-band corner where the response reaches zero.",
                Some("20.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            enum_parameter_doc(
                "phase",
                "Phase",
                "Phase mode used by the spectral filter.",
                Some("zero"),
                &["zero"],
            ),
            enum_parameter_doc(
                "window",
                "Window",
                "Transition window used in the taper region.",
                Some("cosine_taper"),
                &["cosine_taper"],
            ),
        ],
        TraceLocalProcessingOperation::HighpassFilter { .. } => vec![
            number_parameter_doc(
                "f1_hz",
                "F1",
                "Stop-band corner before the low-cut taper begins.",
                Some("5.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            number_parameter_doc(
                "f2_hz",
                "F2",
                "Pass-band corner after the low-cut taper ends.",
                Some("10.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            enum_parameter_doc(
                "phase",
                "Phase",
                "Phase mode used by the spectral filter.",
                Some("zero"),
                &["zero"],
            ),
            enum_parameter_doc(
                "window",
                "Window",
                "Transition window used in the taper region.",
                Some("cosine_taper"),
                &["cosine_taper"],
            ),
        ],
        TraceLocalProcessingOperation::BandpassFilter { .. } => vec![
            number_parameter_doc(
                "f1_hz",
                "F1",
                "Low stop corner.",
                Some("5.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            number_parameter_doc(
                "f2_hz",
                "F2",
                "Low pass corner.",
                Some("10.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            number_parameter_doc(
                "f3_hz",
                "F3",
                "High pass corner.",
                Some("50.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            number_parameter_doc(
                "f4_hz",
                "F4",
                "High stop corner.",
                Some("60.0"),
                Some("Hz"),
                Some("0"),
                Some("Nyquist"),
            ),
            enum_parameter_doc(
                "phase",
                "Phase",
                "Phase mode used by the spectral filter.",
                Some("zero"),
                &["zero"],
            ),
            enum_parameter_doc(
                "window",
                "Window",
                "Transition window used in the taper region.",
                Some("cosine_taper"),
                &["cosine_taper"],
            ),
        ],
        TraceLocalProcessingOperation::VolumeArithmetic { .. } => vec![
            enum_parameter_doc(
                "operator",
                "Operator",
                "Sample-wise arithmetic mode applied to the secondary volume.",
                Some("add"),
                &["add", "subtract", "multiply", "divide"],
            ),
            parameter_doc(
                "secondary_input",
                "Secondary Input",
                "Reference to a second compatible seismic volume.",
                "seismic_asset_ref",
                true,
                None,
                None,
                Vec::new(),
                None,
                None,
            ),
        ],
        TraceLocalProcessingOperation::TraceRmsNormalize
        | TraceLocalProcessingOperation::Envelope
        | TraceLocalProcessingOperation::InstantaneousPhase
        | TraceLocalProcessingOperation::InstantaneousFrequency
        | TraceLocalProcessingOperation::Sweetness => Vec::new(),
    }
}

fn post_stack_neighborhood_documentation(
    operation: &PostStackNeighborhoodProcessingOperation,
) -> OperatorDocumentation {
    let (short_help, help_markdown) = match operation {
        PostStackNeighborhoodProcessingOperation::Similarity { .. } => (
            "Compute a post-stack neighborhood similarity volume.",
            "Similarity compares each sample neighborhood against nearby traces over a finite inline, crossline, and vertical window. It is intended for continuity-style interpretation workflows.",
        ),
        PostStackNeighborhoodProcessingOperation::LocalVolumeStats { .. } => (
            "Compute a local neighborhood statistic over a post-stack volume.",
            "Local Volume Stats evaluates a chosen statistic inside a spatial neighborhood window. It is useful for local attribute smoothing and texture-style measures.",
        ),
        PostStackNeighborhoodProcessingOperation::Dip { .. } => (
            "Estimate local structural dip over a post-stack neighborhood window.",
            "Dip derives local structural orientation attributes from a post-stack neighborhood. The chosen output controls which dip representation is materialized.",
        ),
    };
    OperatorDocumentation {
        short_help: short_help.to_string(),
        help_markdown: Some(help_markdown.to_string()),
        help_url: None,
    }
}

fn post_stack_neighborhood_parameter_docs(
    operation: &PostStackNeighborhoodProcessingOperation,
) -> Vec<OperatorParameterDoc> {
    let mut docs = vec![
        number_parameter_doc(
            "gate_ms",
            "Gate",
            "Vertical analysis gate for the neighborhood operator.",
            Some("24.0"),
            Some("ms"),
            Some("positive"),
            None,
        ),
        number_parameter_doc(
            "inline_stepout",
            "Inline Stepout",
            "Number of neighboring inlines included on each side.",
            Some("1"),
            Some("traces"),
            Some("0"),
            None,
        ),
        number_parameter_doc(
            "xline_stepout",
            "Xline Stepout",
            "Number of neighboring crosslines included on each side.",
            Some("1"),
            Some("traces"),
            Some("0"),
            None,
        ),
    ];
    match operation {
        PostStackNeighborhoodProcessingOperation::LocalVolumeStats { .. } => {
            docs.push(enum_parameter_doc(
                "statistic",
                "Statistic",
                "Neighborhood statistic to materialize.",
                Some("rms"),
                &["mean", "rms", "variance", "minimum", "maximum"],
            ))
        }
        PostStackNeighborhoodProcessingOperation::Dip { .. } => docs.push(enum_parameter_doc(
            "output",
            "Dip Output",
            "Dip representation to materialize.",
            Some("inline"),
            &["inline", "xline", "azimuth", "abs_dip"],
        )),
        PostStackNeighborhoodProcessingOperation::Similarity { .. } => {}
    }
    docs
}

fn gather_documentation(operation: &GatherProcessingOperation) -> OperatorDocumentation {
    let (short_help, help_markdown) = match operation {
        GatherProcessingOperation::NmoCorrection { .. } => (
            "Apply normal moveout correction across each selected gather.",
            "NMO Correction re-aligns reflection events using the supplied velocity model and interpolation mode. It is a gather-native conditioning step for prestack workflows.",
        ),
        GatherProcessingOperation::StretchMute { .. } => (
            "Mute samples that exceed an allowed NMO stretch ratio.",
            "Stretch Mute suppresses samples where NMO correction would introduce excessive temporal stretch. It relies on the same velocity model family as NMO-style workflows.",
        ),
        GatherProcessingOperation::OffsetMute { .. } => (
            "Mute traces outside an allowed offset range.",
            "Offset Mute removes traces using simple minimum and maximum offset limits. It is often used before stack or analysis workflows to constrain gather aperture.",
        ),
    };
    OperatorDocumentation {
        short_help: short_help.to_string(),
        help_markdown: Some(help_markdown.to_string()),
        help_url: None,
    }
}

fn gather_parameter_docs(operation: &GatherProcessingOperation) -> Vec<OperatorParameterDoc> {
    match operation {
        GatherProcessingOperation::NmoCorrection { .. } => vec![
            parameter_doc(
                "velocity_model",
                "Velocity Model",
                "Velocity definition used for NMO correction.",
                "velocity_model",
                true,
                None,
                None,
                Vec::new(),
                None,
                None,
            ),
            enum_parameter_doc(
                "interpolation",
                "Interpolation",
                "Interpolation mode used during gather resampling.",
                Some("linear"),
                &["linear"],
            ),
        ],
        GatherProcessingOperation::StretchMute { .. } => vec![
            parameter_doc(
                "velocity_model",
                "Velocity Model",
                "Velocity definition used to evaluate NMO stretch.",
                "velocity_model",
                true,
                None,
                None,
                Vec::new(),
                None,
                None,
            ),
            number_parameter_doc(
                "max_stretch_ratio",
                "Max Stretch Ratio",
                "Maximum allowed fractional NMO stretch before muting.",
                Some("0.3"),
                None,
                Some("0"),
                None,
            ),
        ],
        GatherProcessingOperation::OffsetMute { .. } => vec![
            number_parameter_doc(
                "min_offset",
                "Min Offset",
                "Optional minimum offset to retain.",
                Some("0.0"),
                Some("m"),
                None,
                None,
            ),
            number_parameter_doc(
                "max_offset",
                "Max Offset",
                "Optional maximum offset to retain.",
                Some("2500.0"),
                Some("m"),
                None,
                None,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SeismicAxisRole, SeismicDimensionDescriptor, SeismicGatherAxisKind, SeismicOrganization,
        SeismicSampleDomain, SeismicTraceDataDescriptor, SeismicUnits,
    };

    fn descriptor(layout: SeismicLayout) -> SeismicTraceDataDescriptor {
        SeismicTraceDataDescriptor {
            id: crate::SeismicAssetId("asset-demo".to_string()),
            label: "Demo".to_string(),
            stacking_state: crate::SeismicStackingState::PostStack,
            organization: match layout {
                SeismicLayout::PreStack3DOffset | SeismicLayout::PreStack2DOffset => {
                    SeismicOrganization::GatherCollection
                }
                _ => SeismicOrganization::BinnedGrid,
            },
            layout,
            gather_axis_kind: matches!(
                layout,
                SeismicLayout::PreStack3DOffset | SeismicLayout::PreStack2DOffset
            )
            .then_some(SeismicGatherAxisKind::Offset),
            dimensions: vec![
                SeismicDimensionDescriptor {
                    role: SeismicAxisRole::Inline,
                    label: "inline".to_string(),
                    start: Some(1000.0),
                    step: Some(1.0),
                    count: 4,
                    values: None,
                    unit: None,
                },
                SeismicDimensionDescriptor {
                    role: SeismicAxisRole::Crossline,
                    label: "xline".to_string(),
                    start: Some(2000.0),
                    step: Some(1.0),
                    count: 4,
                    values: None,
                    unit: None,
                },
                SeismicDimensionDescriptor {
                    role: SeismicAxisRole::Sample,
                    label: "sample".to_string(),
                    start: Some(0.0),
                    step: Some(4.0),
                    count: 128,
                    values: None,
                    unit: Some("ms".to_string()),
                },
            ],
            chunk_shape: Some(vec![4, 4, 64]),
            sample_domain: SeismicSampleDomain::Time,
            units: SeismicUnits {
                sample: "ms".to_string(),
                amplitude: None,
            },
            bin_grid: None,
        }
    }

    #[test]
    fn post_stack_catalog_marks_velocity_scan_unavailable() {
        let catalog = operator_catalog_for_trace_data(&descriptor(SeismicLayout::PostStack3D));
        let velocity_scan = catalog
            .operators
            .iter()
            .find(|entry| entry.id == "velocity_scan")
            .unwrap();
        let similarity = catalog
            .operators
            .iter()
            .find(|entry| entry.id == "similarity")
            .unwrap();
        assert!(matches!(
            velocity_scan.availability,
            OperatorAvailability::Unavailable { .. }
        ));
        assert_eq!(
            similarity.family,
            OperatorFamily::PostStackNeighborhoodProcessing
        );
        assert!(matches!(
            similarity.availability,
            OperatorAvailability::Available
        ));
    }

    #[test]
    fn prestack_offset_catalog_marks_gather_and_analysis_available() {
        let catalog = operator_catalog_for_trace_data(&descriptor(SeismicLayout::PreStack3DOffset));
        let velocity_scan = catalog
            .operators
            .iter()
            .find(|entry| entry.id == "velocity_scan")
            .unwrap();
        let nmo = catalog
            .operators
            .iter()
            .find(|entry| entry.id == "nmo_correction")
            .unwrap();
        assert!(matches!(
            velocity_scan.availability,
            OperatorAvailability::Available
        ));
        assert!(matches!(nmo.availability, OperatorAvailability::Available));
    }

    #[test]
    fn trace_local_catalog_detail_exposes_additive_metadata() {
        let catalog = operator_catalog_for_trace_data(&descriptor(SeismicLayout::PostStack3D));
        let amplitude = catalog
            .operators
            .iter()
            .find(|entry| entry.id == "amplitude_scalar")
            .unwrap();
        let arithmetic = catalog
            .operators
            .iter()
            .find(|entry| entry.id == "volume_arithmetic")
            .unwrap();

        let amplitude_detail = match &amplitude.detail {
            OperatorDetail::TraceLocalProcessing(detail) => detail,
            other => panic!("unexpected amplitude detail: {other:?}"),
        };
        let arithmetic_detail = match &arithmetic.detail {
            OperatorDetail::TraceLocalProcessing(detail) => detail,
            other => panic!("unexpected arithmetic detail: {other:?}"),
        };

        assert_eq!(
            amplitude_detail.planner_hint_summary.preferred_partitioning,
            ophiolite_operators::ProcessingPlannerPartitioningHint::TileGroup
        );
        assert_eq!(
            amplitude_detail
                .dependency_profile_summary
                .sample_dependency,
            ophiolite_operators::ProcessingSampleDependencyKind::Pointwise
        );
        assert!(
            !amplitude_detail
                .capabilities
                .secondary_volume_input_supported
        );
        assert_eq!(
            arithmetic_detail
                .dependency_profile_summary
                .spatial_dependency,
            ophiolite_operators::ProcessingSpatialDependencyKind::ExternalVolumePointwise
        );
        assert_eq!(
            arithmetic_detail.planner_hint_summary.io_cost_class,
            ophiolite_operators::ProcessingPlannerCostClass::Medium
        );
        assert!(
            arithmetic_detail
                .capabilities
                .secondary_volume_input_supported
        );
    }

    #[test]
    fn subvolume_catalog_detail_exposes_terminal_full_volume_metadata() {
        let catalog = operator_catalog_for_trace_data(&descriptor(SeismicLayout::PostStack3D));
        let crop = catalog
            .operators
            .iter()
            .find(|entry| entry.family == OperatorFamily::SubvolumeProcessing)
            .unwrap();

        let detail = match &crop.detail {
            OperatorDetail::SubvolumeProcessing(detail) => detail,
            other => panic!("unexpected subvolume detail: {other:?}"),
        };

        assert_eq!(
            detail.planner_hint_summary.preferred_partitioning,
            ophiolite_operators::ProcessingPlannerPartitioningHint::FullVolume
        );
        assert!(detail.planner_hint_summary.requires_full_volume);
        assert_eq!(
            detail.dependency_profile_summary.sample_dependency,
            ophiolite_operators::ProcessingSampleDependencyKind::Pointwise
        );
        assert!(detail.capabilities.terminal_only);
    }
}
