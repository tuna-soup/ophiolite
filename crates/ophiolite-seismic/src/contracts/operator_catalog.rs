use ophiolite_operators::{
    GatherProcessingDetail, OPERATOR_CATALOG_SCHEMA_VERSION, OperatorAvailability, OperatorCatalog,
    OperatorCatalogEntry, OperatorContractRef, OperatorDetail, OperatorDocumentation,
    OperatorExecutionKind, OperatorFamily, OperatorOutputLifecycle, OperatorParameterDoc,
    OperatorStability, OperatorSubjectKind, PostStackNeighborhoodProcessingDetail,
    SeismicAnalysisDetail, SubvolumeProcessingDetail, TraceLocalProcessingDetail,
};

use crate::{SeismicLayout, SeismicTraceDataDescriptor};

use super::domain::DatasetId;
use super::models::VelocityFunctionSource;
use super::operations::{GatherRequest, GatherSelector};
use super::processing::{
    FrequencyPhaseMode, FrequencyWindowShape, GatherInterpolationMode, GatherProcessingOperation,
    LocalVolumeStatistic, PostStackNeighborhoodProcessingOperation, ProcessingLayoutCompatibility,
    TraceLocalProcessingOperation, TraceLocalVolumeArithmeticOperator,
};

const SEISMIC_OPERATOR_CONTRACT_SCHEMA_ID: &str = "ophiolite.seismic.operations.v1";

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

fn trace_local_operator_entries(layout: SeismicLayout) -> Vec<OperatorCatalogEntry> {
    trace_local_operator_prototypes()
        .into_iter()
        .map(|operation| {
            let compatibility = operation.compatibility();
            let operation_id = operation.operator_id().to_string();
            OperatorCatalogEntry {
                id: operation_id.clone(),
                provider: "ophiolite".to_string(),
                name: title_case(&operation_id),
                group: "Trace Local".to_string(),
                group_id: "trace_local".to_string(),
                description: format!(
                    "Trace-local seismic processing operator '{}' for {} datasets.",
                    operation_id,
                    compatibility.label()
                ),
                family: OperatorFamily::TraceLocalProcessing,
                execution_kind: OperatorExecutionKind::Job,
                output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                stability: OperatorStability::Preview,
                availability: availability_for_layout(compatibility, layout),
                tags: vec!["seismic".to_string(), "trace_local".to_string()],
                documentation: trace_local_documentation(&operation),
                parameter_docs: trace_local_parameter_docs(&operation),
                request_contract: contract_ref("run_trace_local_processing_request"),
                response_contract: contract_ref("run_trace_local_processing_response"),
                detail: OperatorDetail::TraceLocalProcessing(TraceLocalProcessingDetail {
                    operation_id,
                    scope: operation.scope().label().to_string(),
                    layout_compatibility: compatibility.label().to_string(),
                    preview_contract: contract_ref("preview_trace_local_processing_request"),
                    checkpoint_supported: true,
                }),
            }
        })
        .collect()
}

fn subvolume_operator_entry(layout: SeismicLayout) -> OperatorCatalogEntry {
    let compatibility = ProcessingLayoutCompatibility::PostStackOnly;
    OperatorCatalogEntry {
        id: "crop".to_string(),
        provider: "ophiolite".to_string(),
        name: "Crop".to_string(),
        group: "Subvolume".to_string(),
        group_id: "subvolume".to_string(),
        description: "Terminal subvolume derivation that crops a post-stack survey.".to_string(),
        family: OperatorFamily::SubvolumeProcessing,
        execution_kind: OperatorExecutionKind::Job,
        output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
        stability: OperatorStability::Preview,
        availability: availability_for_layout(compatibility, layout),
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
        request_contract: contract_ref("run_subvolume_processing_request"),
        response_contract: contract_ref("run_subvolume_processing_response"),
        detail: OperatorDetail::SubvolumeProcessing(SubvolumeProcessingDetail {
            terminal_operation_id: "crop".to_string(),
            layout_compatibility: compatibility.label().to_string(),
            preview_contract: contract_ref("preview_subvolume_processing_request"),
            trace_local_prefix_supported: true,
        }),
    }
}

fn post_stack_neighborhood_operator_entries(layout: SeismicLayout) -> Vec<OperatorCatalogEntry> {
    post_stack_neighborhood_operator_prototypes()
        .into_iter()
        .map(|operation| {
            let compatibility = operation.compatibility();
            let operation_id = operation.operator_id().to_string();
            OperatorCatalogEntry {
                id: operation_id.clone(),
                provider: "ophiolite".to_string(),
                name: title_case(&operation_id),
                group: "Post-Stack Neighborhood".to_string(),
                group_id: "post_stack_neighborhood".to_string(),
                description: format!(
                    "Post-stack neighborhood seismic operator '{}' for {} datasets.",
                    operation_id,
                    compatibility.label()
                ),
                family: OperatorFamily::PostStackNeighborhoodProcessing,
                execution_kind: OperatorExecutionKind::Job,
                output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                stability: OperatorStability::Preview,
                availability: availability_for_layout(compatibility, layout),
                tags: vec!["seismic".to_string(), "post_stack_neighborhood".to_string()],
                documentation: post_stack_neighborhood_documentation(&operation),
                parameter_docs: post_stack_neighborhood_parameter_docs(&operation),
                request_contract: contract_ref("run_post_stack_neighborhood_processing_request"),
                response_contract: contract_ref("run_post_stack_neighborhood_processing_response"),
                detail: OperatorDetail::PostStackNeighborhoodProcessing(
                    PostStackNeighborhoodProcessingDetail {
                        operation_id,
                        scope: operation.scope().label().to_string(),
                        layout_compatibility: compatibility.label().to_string(),
                        preview_contract: contract_ref(
                            "preview_post_stack_neighborhood_processing_request",
                        ),
                        trace_local_prefix_supported: true,
                    },
                ),
            }
        })
        .collect()
}

fn gather_operator_entries(layout: SeismicLayout) -> Vec<OperatorCatalogEntry> {
    gather_operator_prototypes()
        .into_iter()
        .map(|operation| {
            let compatibility = operation.compatibility();
            let operation_id = operation.operator_id().to_string();
            OperatorCatalogEntry {
                id: operation_id.clone(),
                provider: "ophiolite".to_string(),
                name: title_case(&operation_id),
                group: "Gather".to_string(),
                group_id: "gather".to_string(),
                description: format!(
                    "Gather-native seismic processing operator '{}' for {} datasets.",
                    operation_id,
                    compatibility.label()
                ),
                family: OperatorFamily::GatherProcessing,
                execution_kind: OperatorExecutionKind::Job,
                output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                stability: OperatorStability::Preview,
                availability: availability_for_layout(compatibility, layout),
                tags: vec!["seismic".to_string(), "gather".to_string()],
                documentation: gather_documentation(&operation),
                parameter_docs: gather_parameter_docs(&operation),
                request_contract: contract_ref("run_gather_processing_request"),
                response_contract: contract_ref("run_gather_processing_response"),
                detail: OperatorDetail::GatherProcessing(GatherProcessingDetail {
                    operation_id,
                    scope: operation.scope().label().to_string(),
                    layout_compatibility: compatibility.label().to_string(),
                    preview_contract: contract_ref("preview_gather_processing_request"),
                    trace_local_prefix_supported: true,
                }),
            }
        })
        .collect()
}

fn velocity_scan_operator_entry(layout: SeismicLayout) -> OperatorCatalogEntry {
    let compatibility = ProcessingLayoutCompatibility::PreStackOffsetOnly;
    let _sample_gather_request = GatherRequest {
        dataset_id: DatasetId("dataset:placeholder".to_string()),
        selector: GatherSelector::Ordinal { index: 0 },
    };

    OperatorCatalogEntry {
        id: "velocity_scan".to_string(),
        provider: "ophiolite".to_string(),
        name: "Velocity Scan".to_string(),
        group: "Analysis".to_string(),
        group_id: "analysis".to_string(),
        description: "Prestack offset-gather semblance scan with optional autopick output."
            .to_string(),
        family: OperatorFamily::SeismicAnalysis,
        execution_kind: OperatorExecutionKind::Immediate,
        output_lifecycle: OperatorOutputLifecycle::AnalysisOnly,
        stability: OperatorStability::Preview,
        availability: availability_for_layout(compatibility, layout),
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
        request_contract: contract_ref("velocity_scan_request"),
        response_contract: contract_ref("velocity_scan_response"),
        detail: OperatorDetail::SeismicAnalysis(SeismicAnalysisDetail {
            analysis_kind: "velocity_scan".to_string(),
            layout_compatibility: compatibility.label().to_string(),
            output_kind: "semblance_panel".to_string(),
        }),
    }
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
}
