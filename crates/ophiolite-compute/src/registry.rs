use crate::functions::{
    ComputeExecutionManifest, ComputeFunctionMetadata, ComputeInputBinding, ComputeParameterValue,
    ComputedCurve, DrillingObservationDataRow, LogCurveData, PressureObservationDataRow,
    TopDataRow, TrajectoryDataRow,
};
use crate::semantics::{
    AssetSemanticFamily, CurveBindingCandidate, CurveSemanticDescriptor, CurveSemanticType,
};
use ophiolite_core::{LasError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComputeParameterDefinition {
    Number {
        name: String,
        label: String,
        description: String,
        default: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
        unit: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComputeInputSpec {
    SingleCurve {
        parameter_name: String,
        allowed_types: Vec<CurveSemanticType>,
    },
    CurvePair {
        left_parameter_name: String,
        left_allowed_types: Vec<CurveSemanticType>,
        right_parameter_name: String,
        right_allowed_types: Vec<CurveSemanticType>,
    },
    Trajectory,
    TopSet,
    PressureObservation,
    DrillingObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeBindingCandidate {
    pub parameter_name: String,
    pub allowed_types: Vec<CurveSemanticType>,
    pub matches: Vec<CurveBindingCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComputeAvailability {
    Available,
    Unavailable { reasons: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputeCatalogEntry {
    pub metadata: ComputeFunctionMetadata,
    pub input_specs: Vec<ComputeInputSpec>,
    pub parameters: Vec<ComputeParameterDefinition>,
    pub binding_candidates: Vec<ComputeBindingCandidate>,
    pub availability: ComputeAvailability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputeCatalog {
    pub asset_family: AssetSemanticFamily,
    pub functions: Vec<ComputeCatalogEntry>,
}

trait LogComputeFunction: Send + Sync {
    fn metadata(&self) -> ComputeFunctionMetadata;
    fn input_specs(&self) -> Vec<ComputeInputSpec>;
    fn parameters(&self) -> Vec<ComputeParameterDefinition>;
    fn is_deterministic(&self) -> bool {
        true
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve>;
}

trait TrajectoryComputeFunction: Send + Sync {
    fn metadata(&self) -> ComputeFunctionMetadata;
    fn parameters(&self) -> Vec<ComputeParameterDefinition>;
    fn is_deterministic(&self) -> bool {
        true
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    fn execute(
        &self,
        rows: &[TrajectoryDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<TrajectoryDataRow>>;
}

trait TopSetComputeFunction: Send + Sync {
    fn metadata(&self) -> ComputeFunctionMetadata;
    fn parameters(&self) -> Vec<ComputeParameterDefinition>;
    fn is_deterministic(&self) -> bool {
        true
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    fn execute(
        &self,
        rows: &[TopDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<TopDataRow>>;
}

trait PressureComputeFunction: Send + Sync {
    fn metadata(&self) -> ComputeFunctionMetadata;
    fn parameters(&self) -> Vec<ComputeParameterDefinition>;
    fn is_deterministic(&self) -> bool {
        true
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    fn execute(
        &self,
        rows: &[PressureObservationDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<PressureObservationDataRow>>;
}

trait DrillingComputeFunction: Send + Sync {
    fn metadata(&self) -> ComputeFunctionMetadata;
    fn parameters(&self) -> Vec<ComputeParameterDefinition>;
    fn is_deterministic(&self) -> bool {
        true
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    fn execute(
        &self,
        rows: &[DrillingObservationDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<DrillingObservationDataRow>>;
}

#[derive(Default)]
pub struct ComputeRegistry {
    log_functions: Vec<Box<dyn LogComputeFunction>>,
    trajectory_functions: Vec<Box<dyn TrajectoryComputeFunction>>,
    top_set_functions: Vec<Box<dyn TopSetComputeFunction>>,
    pressure_functions: Vec<Box<dyn PressureComputeFunction>>,
    drilling_functions: Vec<Box<dyn DrillingComputeFunction>>,
}

struct MovingAverageFunction;
struct ZScoreNormalizeFunction;
struct MinMaxScaleFunction;
struct GapFlagFunction;
struct VShaleLinearFunction;
struct VShaleClavierFunction;
struct VShaleSteiberFunction;
struct SonicToVpFunction;
struct ShearSonicToVsFunction;
struct AcousticImpedanceFunction;
struct PoissonsRatioFunction;
struct TrajectorySmoothInclinationFunction;
struct TrajectoryNormalizeAzimuthFunction;
struct TopSortByDepthFunction;
struct TopFillBaseFromNextTopFunction;
struct PressureMovingAverageFunction;
struct PressureNormalizePhaseFunction;
struct DrillingMovingAverageValueFunction;
struct DrillingNormalizeEventKindFunction;

impl ComputeRegistry {
    pub fn new() -> Self {
        Self {
            log_functions: vec![
                Box::new(MovingAverageFunction),
                Box::new(ZScoreNormalizeFunction),
                Box::new(MinMaxScaleFunction),
                Box::new(GapFlagFunction),
                Box::new(VShaleLinearFunction),
                Box::new(VShaleClavierFunction),
                Box::new(VShaleSteiberFunction),
                Box::new(SonicToVpFunction),
                Box::new(ShearSonicToVsFunction),
                Box::new(AcousticImpedanceFunction),
                Box::new(PoissonsRatioFunction),
            ],
            trajectory_functions: vec![
                Box::new(TrajectorySmoothInclinationFunction),
                Box::new(TrajectoryNormalizeAzimuthFunction),
            ],
            top_set_functions: vec![
                Box::new(TopSortByDepthFunction),
                Box::new(TopFillBaseFromNextTopFunction),
            ],
            pressure_functions: vec![
                Box::new(PressureMovingAverageFunction),
                Box::new(PressureNormalizePhaseFunction),
            ],
            drilling_functions: vec![
                Box::new(DrillingMovingAverageValueFunction),
                Box::new(DrillingNormalizeEventKindFunction),
            ],
        }
    }

    pub fn catalog_for_log_asset(
        &self,
        curves: &[CurveSemanticDescriptor],
        numeric_curve_names: &[String],
    ) -> ComputeCatalog {
        ComputeCatalog {
            asset_family: AssetSemanticFamily::Log,
            functions: self
                .log_functions
                .iter()
                .map(|function| {
                    catalog_entry_for_log_function(function.as_ref(), curves, numeric_curve_names)
                })
                .collect(),
        }
    }

    pub fn catalog_for_trajectory_asset(&self) -> ComputeCatalog {
        ComputeCatalog {
            asset_family: AssetSemanticFamily::Trajectory,
            functions: self
                .trajectory_functions
                .iter()
                .map(|function| {
                    structured_entry(
                        function.metadata(),
                        function.parameters(),
                        ComputeInputSpec::Trajectory,
                    )
                })
                .collect(),
        }
    }

    pub fn catalog_for_top_set_asset(&self) -> ComputeCatalog {
        ComputeCatalog {
            asset_family: AssetSemanticFamily::TopSet,
            functions: self
                .top_set_functions
                .iter()
                .map(|function| {
                    structured_entry(
                        function.metadata(),
                        function.parameters(),
                        ComputeInputSpec::TopSet,
                    )
                })
                .collect(),
        }
    }

    pub fn catalog_for_pressure_asset(&self) -> ComputeCatalog {
        ComputeCatalog {
            asset_family: AssetSemanticFamily::PressureObservation,
            functions: self
                .pressure_functions
                .iter()
                .map(|function| {
                    structured_entry(
                        function.metadata(),
                        function.parameters(),
                        ComputeInputSpec::PressureObservation,
                    )
                })
                .collect(),
        }
    }

    pub fn catalog_for_drilling_asset(&self) -> ComputeCatalog {
        ComputeCatalog {
            asset_family: AssetSemanticFamily::DrillingObservation,
            functions: self
                .drilling_functions
                .iter()
                .map(|function| {
                    structured_entry(
                        function.metadata(),
                        function.parameters(),
                        ComputeInputSpec::DrillingObservation,
                    )
                })
                .collect(),
        }
    }

    pub fn run_log_compute(
        &self,
        function_id: &str,
        curves: &[LogCurveData],
        bindings: &BTreeMap<String, String>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<(ComputeExecutionManifest, ComputedCurve)> {
        let function = self
            .log_functions
            .iter()
            .find(|item| item.metadata().id == function_id)
            .ok_or_else(|| {
                LasError::Validation(format!("unknown compute function '{function_id}'"))
            })?;

        let available = curves
            .iter()
            .map(|curve| (curve.curve_name.clone(), curve))
            .collect::<BTreeMap<_, _>>();
        let mut resolved_inputs = BTreeMap::new();
        let mut manifest_inputs = Vec::new();

        for spec in function.input_specs() {
            match spec {
                ComputeInputSpec::SingleCurve {
                    parameter_name,
                    allowed_types,
                } => bind_curve_input(
                    &available,
                    &mut resolved_inputs,
                    &mut manifest_inputs,
                    bindings,
                    &parameter_name,
                    &allowed_types,
                )?,
                ComputeInputSpec::CurvePair {
                    left_parameter_name,
                    left_allowed_types,
                    right_parameter_name,
                    right_allowed_types,
                } => {
                    bind_curve_input(
                        &available,
                        &mut resolved_inputs,
                        &mut manifest_inputs,
                        bindings,
                        &left_parameter_name,
                        &left_allowed_types,
                    )?;
                    bind_curve_input(
                        &available,
                        &mut resolved_inputs,
                        &mut manifest_inputs,
                        bindings,
                        &right_parameter_name,
                        &right_allowed_types,
                    )?;
                }
                _ => {
                    return Err(LasError::Validation(format!(
                        "function '{function_id}' is not supported for log execution",
                    )));
                }
            }
        }

        ensure_shared_depths(&resolved_inputs)?;
        validate_parameters(function.parameters(), parameters)?;
        let output = function.execute(&resolved_inputs, parameters, output_mnemonic)?;
        let metadata = function.metadata();

        Ok((
            ComputeExecutionManifest {
                function_id: metadata.id,
                provider: metadata.provider,
                function_name: metadata.name,
                function_version: function.version().to_string(),
                deterministic: function.is_deterministic(),
                source_asset_id: String::new(),
                source_logical_asset_id: String::new(),
                inputs: manifest_inputs,
                parameters: parameters.clone(),
                output_curve_name: output.curve_name.clone(),
                output_curve_type: output.semantic_type.clone(),
                executed_at_unix_seconds: 0,
            },
            output,
        ))
    }

    pub fn run_trajectory_compute(
        &self,
        function_id: &str,
        rows: &[TrajectoryDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<(ComputeExecutionManifest, Vec<TrajectoryDataRow>)> {
        let function = self
            .trajectory_functions
            .iter()
            .find(|item| item.metadata().id == function_id)
            .ok_or_else(|| {
                LasError::Validation(format!("unknown compute function '{function_id}'"))
            })?;
        validate_parameters(function.parameters(), parameters)?;
        let metadata = function.metadata();
        let output = function.execute(rows, parameters)?;
        Ok((
            structured_manifest(
                metadata,
                function.version(),
                function.is_deterministic(),
                parameters,
                "trajectory_rows",
                "trajectory",
            ),
            output,
        ))
    }

    pub fn run_top_set_compute(
        &self,
        function_id: &str,
        rows: &[TopDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<(ComputeExecutionManifest, Vec<TopDataRow>)> {
        let function = self
            .top_set_functions
            .iter()
            .find(|item| item.metadata().id == function_id)
            .ok_or_else(|| {
                LasError::Validation(format!("unknown compute function '{function_id}'"))
            })?;
        validate_parameters(function.parameters(), parameters)?;
        let metadata = function.metadata();
        let output = function.execute(rows, parameters)?;
        Ok((
            structured_manifest(
                metadata,
                function.version(),
                function.is_deterministic(),
                parameters,
                "tops_rows",
                "tops",
            ),
            output,
        ))
    }

    pub fn run_pressure_compute(
        &self,
        function_id: &str,
        rows: &[PressureObservationDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<(ComputeExecutionManifest, Vec<PressureObservationDataRow>)> {
        let function = self
            .pressure_functions
            .iter()
            .find(|item| item.metadata().id == function_id)
            .ok_or_else(|| {
                LasError::Validation(format!("unknown compute function '{function_id}'"))
            })?;
        validate_parameters(function.parameters(), parameters)?;
        let metadata = function.metadata();
        let output = function.execute(rows, parameters)?;
        Ok((
            structured_manifest(
                metadata,
                function.version(),
                function.is_deterministic(),
                parameters,
                "pressure_rows",
                "pressure",
            ),
            output,
        ))
    }

    pub fn run_drilling_compute(
        &self,
        function_id: &str,
        rows: &[DrillingObservationDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<(ComputeExecutionManifest, Vec<DrillingObservationDataRow>)> {
        let function = self
            .drilling_functions
            .iter()
            .find(|item| item.metadata().id == function_id)
            .ok_or_else(|| {
                LasError::Validation(format!("unknown compute function '{function_id}'"))
            })?;
        validate_parameters(function.parameters(), parameters)?;
        let metadata = function.metadata();
        let output = function.execute(rows, parameters)?;
        Ok((
            structured_manifest(
                metadata,
                function.version(),
                function.is_deterministic(),
                parameters,
                "drilling_rows",
                "drilling",
            ),
            output,
        ))
    }
}

fn bind_curve_input<'a>(
    available: &BTreeMap<String, &'a LogCurveData>,
    resolved_inputs: &mut BTreeMap<String, LogCurveData>,
    manifest_inputs: &mut Vec<ComputeInputBinding>,
    bindings: &BTreeMap<String, String>,
    parameter_name: &str,
    allowed_types: &[CurveSemanticType],
) -> Result<()> {
    let curve_name = bindings.get(parameter_name).ok_or_else(|| {
        LasError::Validation(format!("compute binding '{}' is required", parameter_name))
    })?;
    let curve = available.get(curve_name).ok_or_else(|| {
        LasError::Validation(format!(
            "curve '{}' is not available in the selected log asset",
            curve_name
        ))
    })?;
    ensure_allowed_type(parameter_name, curve.semantic_type.clone(), allowed_types)?;
    resolved_inputs.insert(parameter_name.to_string(), (*curve).clone());
    manifest_inputs.push(ComputeInputBinding {
        parameter_name: parameter_name.to_string(),
        curve_name: curve.curve_name.clone(),
        semantic_type: curve.semantic_type.clone(),
    });
    Ok(())
}

fn catalog_entry_for_log_function(
    function: &dyn LogComputeFunction,
    curves: &[CurveSemanticDescriptor],
    numeric_curve_names: &[String],
) -> ComputeCatalogEntry {
    let metadata = function.metadata();
    let input_specs = function.input_specs();
    let parameters = function.parameters();
    let binding_candidates = input_specs
        .iter()
        .flat_map(|spec| match spec {
            ComputeInputSpec::SingleCurve {
                parameter_name,
                allowed_types,
            } => vec![binding_candidates_for(
                parameter_name,
                allowed_types,
                curves,
                numeric_curve_names,
            )],
            ComputeInputSpec::CurvePair {
                left_parameter_name,
                left_allowed_types,
                right_parameter_name,
                right_allowed_types,
            } => vec![
                binding_candidates_for(
                    left_parameter_name,
                    left_allowed_types,
                    curves,
                    numeric_curve_names,
                ),
                binding_candidates_for(
                    right_parameter_name,
                    right_allowed_types,
                    curves,
                    numeric_curve_names,
                ),
            ],
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();

    let reasons = binding_candidates
        .iter()
        .filter(|candidate| candidate.matches.is_empty())
        .map(|candidate| {
            if candidate.allowed_types.is_empty() {
                format!("no numeric curve matches '{}'", candidate.parameter_name)
            } else {
                format!(
                    "no compatible curve matches '{}' ({})",
                    candidate.parameter_name,
                    candidate
                        .allowed_types
                        .iter()
                        .map(CurveSemanticType::display_name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        })
        .collect::<Vec<_>>();

    ComputeCatalogEntry {
        metadata,
        input_specs,
        parameters,
        binding_candidates,
        availability: if reasons.is_empty() {
            ComputeAvailability::Available
        } else {
            ComputeAvailability::Unavailable { reasons }
        },
    }
}

fn binding_candidates_for(
    parameter_name: &str,
    allowed_types: &[CurveSemanticType],
    curves: &[CurveSemanticDescriptor],
    numeric_curve_names: &[String],
) -> ComputeBindingCandidate {
    let matches = curves
        .iter()
        .filter(|curve| {
            if curve.semantic_type == CurveSemanticType::Depth
                || curve.semantic_type == CurveSemanticType::Time
            {
                return false;
            }
            if allowed_types.is_empty() {
                numeric_curve_names
                    .iter()
                    .any(|name| name == &curve.curve_name)
            } else {
                allowed_types.contains(&curve.semantic_type)
            }
        })
        .map(|curve| CurveBindingCandidate {
            curve_name: curve.curve_name.clone(),
            original_mnemonic: curve.original_mnemonic.clone(),
            semantic_type: curve.semantic_type.clone(),
            unit: curve.unit.clone(),
        })
        .collect();

    ComputeBindingCandidate {
        parameter_name: parameter_name.to_string(),
        allowed_types: allowed_types.to_vec(),
        matches,
    }
}

fn structured_entry(
    metadata: ComputeFunctionMetadata,
    parameters: Vec<ComputeParameterDefinition>,
    input_spec: ComputeInputSpec,
) -> ComputeCatalogEntry {
    ComputeCatalogEntry {
        metadata,
        input_specs: vec![input_spec],
        parameters,
        binding_candidates: Vec::new(),
        availability: ComputeAvailability::Available,
    }
}

fn structured_manifest(
    metadata: ComputeFunctionMetadata,
    function_version: &str,
    deterministic: bool,
    parameters: &BTreeMap<String, ComputeParameterValue>,
    parameter_name: &str,
    label: &str,
) -> ComputeExecutionManifest {
    ComputeExecutionManifest {
        function_id: metadata.id,
        provider: metadata.provider,
        function_name: metadata.name,
        function_version: function_version.to_string(),
        deterministic,
        source_asset_id: String::new(),
        source_logical_asset_id: String::new(),
        inputs: vec![ComputeInputBinding {
            parameter_name: parameter_name.to_string(),
            curve_name: label.to_string(),
            semantic_type: CurveSemanticType::Computed,
        }],
        parameters: parameters.clone(),
        output_curve_name: label.to_string(),
        output_curve_type: metadata.output_curve_type,
        executed_at_unix_seconds: 0,
    }
}

fn validate_parameters(
    definitions: Vec<ComputeParameterDefinition>,
    parameters: &BTreeMap<String, ComputeParameterValue>,
) -> Result<()> {
    for definition in definitions {
        match definition {
            ComputeParameterDefinition::Number {
                name,
                default,
                min,
                max,
                ..
            } => {
                let value = parameters
                    .get(&name)
                    .and_then(ComputeParameterValue::as_f64)
                    .or(default)
                    .ok_or_else(|| {
                        LasError::Validation(format!(
                            "numeric compute parameter '{}' is required",
                            name
                        ))
                    })?;
                if let Some(minimum) = min
                    && value < minimum
                {
                    return Err(LasError::Validation(format!(
                        "parameter '{}' must be >= {}",
                        name, minimum
                    )));
                }
                if let Some(maximum) = max
                    && value > maximum
                {
                    return Err(LasError::Validation(format!(
                        "parameter '{}' must be <= {}",
                        name, maximum
                    )));
                }
            }
        }
    }
    Ok(())
}

fn ensure_allowed_type(
    parameter_name: &str,
    actual: CurveSemanticType,
    allowed: &[CurveSemanticType],
) -> Result<()> {
    if allowed.is_empty() || allowed.contains(&actual) {
        Ok(())
    } else {
        Err(LasError::Validation(format!(
            "curve bound to '{}' has semantic type '{}' but expected {}",
            parameter_name,
            actual.display_name(),
            allowed
                .iter()
                .map(CurveSemanticType::display_name)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

fn ensure_shared_depths(inputs: &BTreeMap<String, LogCurveData>) -> Result<()> {
    let mut iter = inputs.values();
    let Some(first) = iter.next() else {
        return Ok(());
    };
    for curve in iter {
        if curve.depths.len() != first.depths.len() {
            return Err(LasError::Validation(format!(
                "curve '{}' does not share the same sample count as '{}'",
                curve.curve_name, first.curve_name
            )));
        }
        for (index, (left, right)) in first.depths.iter().zip(&curve.depths).enumerate() {
            if (left - right).abs() > 1e-9 {
                return Err(LasError::Validation(format!(
                    "curve '{}' depth index diverges from '{}' at row {}",
                    curve.curve_name, first.curve_name, index
                )));
            }
        }
    }
    Ok(())
}

fn number_param(
    name: &str,
    label: &str,
    description: &str,
    default: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    unit: Option<&str>,
) -> ComputeParameterDefinition {
    ComputeParameterDefinition::Number {
        name: name.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        default,
        min,
        max,
        unit: unit.map(str::to_string),
    }
}

fn numeric_param_value(
    parameters: &BTreeMap<String, ComputeParameterValue>,
    name: &str,
    default: Option<f64>,
) -> Result<f64> {
    parameters
        .get(name)
        .and_then(ComputeParameterValue::as_f64)
        .or(default)
        .ok_or_else(|| LasError::Validation(format!("parameter '{}' is required", name)))
}

fn moving_average_values(values: &[Option<f64>], window: usize) -> Vec<Option<f64>> {
    let mut window = window.max(1);
    if window % 2 == 0 {
        window += 1;
    }
    let radius = window / 2;
    (0..values.len())
        .map(|index| {
            let start = index.saturating_sub(radius);
            let stop = (index + radius + 1).min(values.len());
            let mut sum = 0.0;
            let mut count = 0usize;
            for value in &values[start..stop] {
                if let Some(number) = value {
                    sum += *number;
                    count += 1;
                }
            }
            (count > 0).then_some(sum / count as f64)
        })
        .collect()
}

fn slowness_to_velocity(slowness: f64) -> Option<f64> {
    if slowness <= 0.0 {
        return None;
    }
    if slowness > 1_000.0 {
        Some(1_000_000.0 / slowness)
    } else {
        Some(304_800.0 / slowness)
    }
}

impl LogComputeFunction for MovingAverageFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "core:moving_average".to_string(),
            provider: "core".to_string(),
            name: "Moving Average".to_string(),
            category: "Transforms".to_string(),
            description: "Smooth a numeric log curve with a centered moving average window."
                .to_string(),
            default_output_mnemonic: "MA".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["transform".to_string(), "smoothing".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::SingleCurve {
            parameter_name: "curve".to_string(),
            allowed_types: Vec::new(),
        }]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![number_param(
            "window",
            "Window",
            "Centered moving average window size in samples.",
            Some(5.0),
            Some(1.0),
            Some(501.0),
            Some("samples"),
        )]
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let curve = inputs.get("curve").ok_or_else(|| {
            LasError::Validation("moving average requires a bound input curve".to_string())
        })?;
        let window = numeric_param_value(parameters, "window", Some(5.0))?.round() as usize;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("MA").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("MA").to_string(),
            unit: curve.unit.clone(),
            description: Some(format!("Moving average of {}", curve.curve_name)),
            semantic_type: CurveSemanticType::Computed,
            values: moving_average_values(&curve.values, window),
        })
    }
}

impl LogComputeFunction for ZScoreNormalizeFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "core:zscore_normalize".to_string(),
            provider: "core".to_string(),
            name: "Z-score Normalize".to_string(),
            category: "Transforms".to_string(),
            description: "Convert a numeric log curve to z-scores.".to_string(),
            default_output_mnemonic: "ZSCORE".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["transform".to_string(), "normalize".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::SingleCurve {
            parameter_name: "curve".to_string(),
            allowed_types: Vec::new(),
        }]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        _parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let curve = inputs.get("curve").ok_or_else(|| {
            LasError::Validation("z-score normalize requires a bound input curve".to_string())
        })?;
        let valid = curve.values.iter().flatten().copied().collect::<Vec<_>>();
        if valid.is_empty() {
            return Err(LasError::Validation(format!(
                "curve '{}' has no numeric samples to normalize",
                curve.curve_name
            )));
        }
        let mean = valid.iter().sum::<f64>() / valid.len() as f64;
        let variance = valid
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / valid.len() as f64;
        let std_dev = variance.sqrt();
        let values = curve
            .values
            .iter()
            .map(|value| {
                value.map(|number| {
                    if std_dev.abs() < f64::EPSILON {
                        0.0
                    } else {
                        (number - mean) / std_dev
                    }
                })
            })
            .collect();
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("ZSCORE").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("ZSCORE").to_string(),
            unit: None,
            description: Some(format!("Z-score normalized {}", curve.curve_name)),
            semantic_type: CurveSemanticType::Computed,
            values,
        })
    }
}

impl LogComputeFunction for MinMaxScaleFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "core:minmax_scale".to_string(),
            provider: "core".to_string(),
            name: "Min-Max Scale".to_string(),
            category: "Transforms".to_string(),
            description: "Scale a numeric log curve into a target numeric range.".to_string(),
            default_output_mnemonic: "SCALED".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["transform".to_string(), "scale".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::SingleCurve {
            parameter_name: "curve".to_string(),
            allowed_types: Vec::new(),
        }]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![
            number_param(
                "target_min",
                "Target Min",
                "Minimum output value.",
                Some(0.0),
                None,
                None,
                None,
            ),
            number_param(
                "target_max",
                "Target Max",
                "Maximum output value.",
                Some(1.0),
                None,
                None,
                None,
            ),
        ]
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let curve = inputs.get("curve").ok_or_else(|| {
            LasError::Validation("min-max scale requires a bound input curve".to_string())
        })?;
        let target_min = numeric_param_value(parameters, "target_min", Some(0.0))?;
        let target_max = numeric_param_value(parameters, "target_max", Some(1.0))?;
        let valid = curve.values.iter().flatten().copied().collect::<Vec<_>>();
        if valid.is_empty() {
            return Err(LasError::Validation(format!(
                "curve '{}' has no numeric samples to scale",
                curve.curve_name
            )));
        }
        let min = valid
            .iter()
            .fold(f64::INFINITY, |acc, value| acc.min(*value));
        let max = valid
            .iter()
            .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value));
        let values = if (max - min).abs() < f64::EPSILON {
            curve
                .values
                .iter()
                .map(|value| value.map(|_| target_min))
                .collect()
        } else {
            curve
                .values
                .iter()
                .map(|value| {
                    value.map(|number| {
                        let normalized = (number - min) / (max - min);
                        target_min + normalized * (target_max - target_min)
                    })
                })
                .collect()
        };
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("SCALED").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("SCALED").to_string(),
            unit: None,
            description: Some(format!("Min-max scaled {}", curve.curve_name)),
            semantic_type: CurveSemanticType::Computed,
            values,
        })
    }
}

impl LogComputeFunction for GapFlagFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "core:gap_flag".to_string(),
            provider: "core".to_string(),
            name: "Gap Flags".to_string(),
            category: "Quality Control".to_string(),
            description: "Flag missing samples in a numeric log curve.".to_string(),
            default_output_mnemonic: "GAP_FLAG".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["qc".to_string(), "gaps".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::SingleCurve {
            parameter_name: "curve".to_string(),
            allowed_types: Vec::new(),
        }]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        _parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let curve = inputs.get("curve").ok_or_else(|| {
            LasError::Validation("gap flag requires a bound input curve".to_string())
        })?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("GAP_FLAG").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("GAP_FLAG").to_string(),
            unit: None,
            description: Some(format!("Gap flags for {}", curve.curve_name)),
            semantic_type: CurveSemanticType::Computed,
            values: curve
                .values
                .iter()
                .map(|value| Some(if value.is_some() { 0.0 } else { 1.0 }))
                .collect(),
        })
    }
}

macro_rules! impl_vshale {
    ($name:ident, $id:literal, $display:literal, $mnemonic:literal, $body:expr) => {
        impl LogComputeFunction for $name {
            fn metadata(&self) -> ComputeFunctionMetadata {
                ComputeFunctionMetadata {
                    id: $id.to_string(),
                    provider: "petro".to_string(),
                    name: $display.to_string(),
                    category: "Petrophysics".to_string(),
                    description: format!("Calculate {} from gamma ray.", $display),
                    default_output_mnemonic: $mnemonic.to_string(),
                    output_curve_type: CurveSemanticType::VShale,
                    tags: vec!["petrophysics".to_string(), "vshale".to_string()],
                }
            }

            fn input_specs(&self) -> Vec<ComputeInputSpec> {
                vec![ComputeInputSpec::SingleCurve {
                    parameter_name: "gr_curve".to_string(),
                    allowed_types: vec![CurveSemanticType::GammaRay],
                }]
            }

            fn parameters(&self) -> Vec<ComputeParameterDefinition> {
                vec![
                    number_param(
                        "gr_min",
                        "GR Clean",
                        "Gamma ray in clean sand.",
                        Some(30.0),
                        Some(0.0),
                        None,
                        Some("gAPI"),
                    ),
                    number_param(
                        "gr_max",
                        "GR Shale",
                        "Gamma ray in shale.",
                        Some(100.0),
                        Some(0.0),
                        None,
                        Some("gAPI"),
                    ),
                ]
            }

            fn execute(
                &self,
                inputs: &BTreeMap<String, LogCurveData>,
                parameters: &BTreeMap<String, ComputeParameterValue>,
                output_mnemonic: Option<&str>,
            ) -> Result<ComputedCurve> {
                let curve = inputs.get("gr_curve").ok_or_else(|| {
                    LasError::Validation("gamma ray curve is required".to_string())
                })?;
                let gr_min = numeric_param_value(parameters, "gr_min", Some(30.0))?;
                let gr_max = numeric_param_value(parameters, "gr_max", Some(100.0))?;
                if gr_max <= gr_min {
                    return Err(LasError::Validation(
                        "gr_max must be greater than gr_min".to_string(),
                    ));
                }
                let span = gr_max - gr_min;
                let values = curve
                    .values
                    .iter()
                    .map(|value| value.map(|number| $body(number, gr_min, span).clamp(0.0, 1.0)))
                    .collect();
                Ok(ComputedCurve {
                    curve_name: output_mnemonic.unwrap_or($mnemonic).to_string(),
                    original_mnemonic: output_mnemonic.unwrap_or($mnemonic).to_string(),
                    unit: Some("v/v".to_string()),
                    description: Some(format!("{} from {}", $display, curve.curve_name)),
                    semantic_type: CurveSemanticType::VShale,
                    values,
                })
            }
        }
    };
}

impl_vshale!(
    VShaleLinearFunction,
    "petro:vshale_linear",
    "VShale (Linear)",
    "VSH_LIN",
    |number: f64, gr_min: f64, span: f64| { (number - gr_min) / span }
);
impl_vshale!(
    VShaleClavierFunction,
    "petro:vshale_clavier",
    "VShale (Clavier)",
    "VSH_CLAV",
    |number: f64, gr_min: f64, span: f64| {
        let igr = (number - gr_min) / span;
        let inner = 3.38 - (igr + 0.7).powi(2);
        if inner >= 0.0 {
            1.7 - inner.sqrt()
        } else {
            1.0
        }
    }
);
impl_vshale!(
    VShaleSteiberFunction,
    "petro:vshale_steiber",
    "VShale (Steiber)",
    "VSH_STEI",
    |number: f64, gr_min: f64, span: f64| {
        let igr = ((number - gr_min) / span).clamp(0.0, 1.0);
        let denominator = 3.0 - 2.0 * igr;
        if denominator > 0.0 {
            igr / denominator
        } else {
            1.0
        }
    }
);

macro_rules! impl_velocity_from_slowness {
    ($name:ident, $id:literal, $display:literal, $param:literal, $allowed:expr, $mnemonic:literal, $semantic:expr, $description:literal) => {
        impl LogComputeFunction for $name {
            fn metadata(&self) -> ComputeFunctionMetadata {
                ComputeFunctionMetadata {
                    id: $id.to_string(),
                    provider: "rock_physics".to_string(),
                    name: $display.to_string(),
                    category: "Rock Physics".to_string(),
                    description: $description.to_string(),
                    default_output_mnemonic: $mnemonic.to_string(),
                    output_curve_type: $semantic,
                    tags: vec!["velocity".to_string(), "rock-physics".to_string()],
                }
            }

            fn input_specs(&self) -> Vec<ComputeInputSpec> {
                vec![ComputeInputSpec::SingleCurve {
                    parameter_name: $param.to_string(),
                    allowed_types: $allowed,
                }]
            }

            fn parameters(&self) -> Vec<ComputeParameterDefinition> {
                Vec::new()
            }

            fn execute(
                &self,
                inputs: &BTreeMap<String, LogCurveData>,
                _parameters: &BTreeMap<String, ComputeParameterValue>,
                output_mnemonic: Option<&str>,
            ) -> Result<ComputedCurve> {
                let curve = inputs
                    .get($param)
                    .ok_or_else(|| LasError::Validation(format!("{} is required", $param)))?;
                Ok(ComputedCurve {
                    curve_name: output_mnemonic.unwrap_or($mnemonic).to_string(),
                    original_mnemonic: output_mnemonic.unwrap_or($mnemonic).to_string(),
                    unit: Some("m/s".to_string()),
                    description: Some(format!("{} from {}", $display, curve.curve_name)),
                    semantic_type: $semantic,
                    values: curve
                        .values
                        .iter()
                        .map(|value| value.and_then(slowness_to_velocity))
                        .collect(),
                })
            }
        }
    };
}

impl_velocity_from_slowness!(
    SonicToVpFunction,
    "rock_physics:sonic_to_vp",
    "Sonic to Vp",
    "sonic_curve",
    vec![CurveSemanticType::Sonic],
    "VP",
    CurveSemanticType::PVelocity,
    "Convert compressional slowness to P-wave velocity."
);
impl_velocity_from_slowness!(
    ShearSonicToVsFunction,
    "rock_physics:shear_sonic_to_vs",
    "Shear Sonic to Vs",
    "shear_sonic_curve",
    vec![CurveSemanticType::ShearSonic],
    "VS",
    CurveSemanticType::SVelocity,
    "Convert shear slowness to S-wave velocity."
);

impl LogComputeFunction for AcousticImpedanceFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:acoustic_impedance".to_string(),
            provider: "rock_physics".to_string(),
            name: "Acoustic Impedance".to_string(),
            category: "Rock Physics".to_string(),
            description: "Multiply P-wave velocity by bulk density.".to_string(),
            default_output_mnemonic: "AI".to_string(),
            output_curve_type: CurveSemanticType::AcousticImpedance,
            tags: vec!["rock-physics".to_string(), "impedance".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::CurvePair {
            left_parameter_name: "vp_curve".to_string(),
            left_allowed_types: vec![CurveSemanticType::PVelocity],
            right_parameter_name: "density_curve".to_string(),
            right_allowed_types: vec![CurveSemanticType::BulkDensity],
        }]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        _parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let vp = inputs
            .get("vp_curve")
            .ok_or_else(|| LasError::Validation("vp curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("AI").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("AI").to_string(),
            unit: Some("(m/s)*(g/cm3)".to_string()),
            description: Some(format!(
                "Acoustic impedance from {} and {}",
                vp.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::AcousticImpedance,
            values: vp
                .values
                .iter()
                .zip(&density.values)
                .map(|(left, right)| match (left, right) {
                    (Some(vp), Some(rho)) => Some(vp * rho),
                    _ => None,
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for PoissonsRatioFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:poissons_ratio".to_string(),
            provider: "rock_physics".to_string(),
            name: "Poisson's Ratio".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute Poisson's ratio from P-wave and S-wave velocity.".to_string(),
            default_output_mnemonic: "PR".to_string(),
            output_curve_type: CurveSemanticType::PoissonsRatio,
            tags: vec!["rock-physics".to_string(), "elastic".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::CurvePair {
            left_parameter_name: "vp_curve".to_string(),
            left_allowed_types: vec![CurveSemanticType::PVelocity],
            right_parameter_name: "vs_curve".to_string(),
            right_allowed_types: vec![CurveSemanticType::SVelocity],
        }]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        _parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let vp = inputs
            .get("vp_curve")
            .ok_or_else(|| LasError::Validation("vp curve is required".to_string()))?;
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("PR").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("PR").to_string(),
            unit: None,
            description: Some(format!(
                "Poisson's ratio from {} and {}",
                vp.curve_name, vs.curve_name
            )),
            semantic_type: CurveSemanticType::PoissonsRatio,
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .map(|(left, right)| match (left, right) {
                    (Some(vp_value), Some(vs_value)) if *vp_value > 0.0 && *vs_value > 0.0 => {
                        let ratio_sq = (vp_value / vs_value).powi(2);
                        let denominator = 2.0 * (ratio_sq - 1.0);
                        (denominator.abs() > f64::EPSILON).then_some((ratio_sq - 2.0) / denominator)
                    }
                    _ => None,
                })
                .collect(),
        })
    }
}

impl TrajectoryComputeFunction for TrajectorySmoothInclinationFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "trajectory:smooth_inclination".to_string(),
            provider: "trajectory".to_string(),
            name: "Smooth Inclination".to_string(),
            category: "Trajectory".to_string(),
            description: "Apply a moving average to trajectory inclination.".to_string(),
            default_output_mnemonic: "trajectory_smooth_inclination".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["trajectory".to_string(), "smoothing".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![number_param(
            "window",
            "Window",
            "Centered moving-average window in rows.",
            Some(5.0),
            Some(1.0),
            Some(101.0),
            Some("rows"),
        )]
    }

    fn execute(
        &self,
        rows: &[TrajectoryDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<TrajectoryDataRow>> {
        let window = numeric_param_value(parameters, "window", Some(5.0))?.round() as usize;
        let smoothed = moving_average_values(
            &rows
                .iter()
                .map(|row| row.inclination_deg)
                .collect::<Vec<_>>(),
            window,
        );
        Ok(rows
            .iter()
            .cloned()
            .zip(smoothed)
            .map(|(mut row, inclination)| {
                row.inclination_deg = inclination;
                row
            })
            .collect())
    }
}

impl TrajectoryComputeFunction for TrajectoryNormalizeAzimuthFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "trajectory:normalize_azimuth".to_string(),
            provider: "trajectory".to_string(),
            name: "Normalize Azimuth".to_string(),
            category: "Trajectory".to_string(),
            description: "Wrap azimuth values into the 0-360 degree interval.".to_string(),
            default_output_mnemonic: "trajectory_normalize_azimuth".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["trajectory".to_string(), "normalization".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        rows: &[TrajectoryDataRow],
        _parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<TrajectoryDataRow>> {
        Ok(rows
            .iter()
            .cloned()
            .map(|mut row| {
                row.azimuth_deg = row.azimuth_deg.map(|value| value.rem_euclid(360.0));
                row
            })
            .collect())
    }
}

impl TopSetComputeFunction for TopSortByDepthFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "tops:sort_by_depth".to_string(),
            provider: "tops".to_string(),
            name: "Sort Tops by Depth".to_string(),
            category: "Interpretation".to_string(),
            description: "Sort top markers by ascending top depth.".to_string(),
            default_output_mnemonic: "tops_sort_by_depth".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["tops".to_string(), "ordering".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        rows: &[TopDataRow],
        _parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<TopDataRow>> {
        let mut result = rows.to_vec();
        result.sort_by(|left, right| left.top_depth.total_cmp(&right.top_depth));
        Ok(result)
    }
}

impl TopSetComputeFunction for TopFillBaseFromNextTopFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "tops:fill_base_from_next_top".to_string(),
            provider: "tops".to_string(),
            name: "Fill Base Depths".to_string(),
            category: "Interpretation".to_string(),
            description: "Fill missing base depths from the next top in depth order.".to_string(),
            default_output_mnemonic: "tops_fill_base".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["tops".to_string(), "intervals".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        rows: &[TopDataRow],
        _parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<TopDataRow>> {
        let mut result = rows.to_vec();
        result.sort_by(|left, right| left.top_depth.total_cmp(&right.top_depth));
        for index in 0..result.len() {
            if result[index].base_depth.is_none() {
                result[index].base_depth = result.get(index + 1).map(|next| next.top_depth);
            }
        }
        Ok(result)
    }
}

impl PressureComputeFunction for PressureMovingAverageFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "pressure:moving_average".to_string(),
            provider: "pressure".to_string(),
            name: "Smooth Pressure".to_string(),
            category: "Pressure".to_string(),
            description: "Apply a moving average to pressure observations.".to_string(),
            default_output_mnemonic: "pressure_moving_average".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["pressure".to_string(), "smoothing".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![number_param(
            "window",
            "Window",
            "Centered moving-average window in rows.",
            Some(3.0),
            Some(1.0),
            Some(51.0),
            Some("rows"),
        )]
    }

    fn execute(
        &self,
        rows: &[PressureObservationDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<PressureObservationDataRow>> {
        let window = numeric_param_value(parameters, "window", Some(3.0))?.round() as usize;
        let smoothed = moving_average_values(
            &rows
                .iter()
                .map(|row| Some(row.pressure))
                .collect::<Vec<_>>(),
            window,
        );
        Ok(rows
            .iter()
            .cloned()
            .zip(smoothed)
            .map(|(mut row, pressure)| {
                row.pressure = pressure.unwrap_or(row.pressure);
                row
            })
            .collect())
    }
}

impl PressureComputeFunction for PressureNormalizePhaseFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "pressure:normalize_phase_labels".to_string(),
            provider: "pressure".to_string(),
            name: "Normalize Phase Labels".to_string(),
            category: "Pressure".to_string(),
            description: "Normalize phase strings into uppercase labels.".to_string(),
            default_output_mnemonic: "pressure_normalize_phase".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["pressure".to_string(), "cleanup".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        rows: &[PressureObservationDataRow],
        _parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<PressureObservationDataRow>> {
        Ok(rows
            .iter()
            .cloned()
            .map(|mut row| {
                row.phase = row.phase.map(|phase| phase.trim().to_ascii_uppercase());
                row
            })
            .collect())
    }
}

impl DrillingComputeFunction for DrillingMovingAverageValueFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "drilling:moving_average_value".to_string(),
            provider: "drilling".to_string(),
            name: "Smooth Observation Values".to_string(),
            category: "Drilling".to_string(),
            description: "Apply a moving average to drilling observation values.".to_string(),
            default_output_mnemonic: "drilling_moving_average".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["drilling".to_string(), "smoothing".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![number_param(
            "window",
            "Window",
            "Centered moving-average window in rows.",
            Some(3.0),
            Some(1.0),
            Some(51.0),
            Some("rows"),
        )]
    }

    fn execute(
        &self,
        rows: &[DrillingObservationDataRow],
        parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<DrillingObservationDataRow>> {
        let window = numeric_param_value(parameters, "window", Some(3.0))?.round() as usize;
        let smoothed = moving_average_values(
            &rows.iter().map(|row| row.value).collect::<Vec<_>>(),
            window,
        );
        Ok(rows
            .iter()
            .cloned()
            .zip(smoothed)
            .map(|(mut row, value)| {
                row.value = value;
                row
            })
            .collect())
    }
}

impl DrillingComputeFunction for DrillingNormalizeEventKindFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "drilling:normalize_event_kind".to_string(),
            provider: "drilling".to_string(),
            name: "Normalize Event Kind".to_string(),
            category: "Drilling".to_string(),
            description: "Normalize drilling event labels into uppercase underscore form."
                .to_string(),
            default_output_mnemonic: "drilling_normalize_event".to_string(),
            output_curve_type: CurveSemanticType::Computed,
            tags: vec!["drilling".to_string(), "cleanup".to_string()],
        }
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        rows: &[DrillingObservationDataRow],
        _parameters: &BTreeMap<String, ComputeParameterValue>,
    ) -> Result<Vec<DrillingObservationDataRow>> {
        Ok(rows
            .iter()
            .cloned()
            .map(|mut row| {
                row.event_kind = row
                    .event_kind
                    .trim()
                    .to_ascii_uppercase()
                    .replace([' ', '-'], "_");
                row
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantics::{CurveSemanticDescriptor, CurveSemanticSource};

    fn descriptor(
        name: &str,
        mnemonic: &str,
        semantic: CurveSemanticType,
    ) -> CurveSemanticDescriptor {
        CurveSemanticDescriptor {
            curve_name: name.to_string(),
            original_mnemonic: mnemonic.to_string(),
            unit: Some("unit".to_string()),
            semantic_type: semantic,
            source: CurveSemanticSource::Derived,
        }
    }

    fn curve(name: &str, semantic: CurveSemanticType, values: &[Option<f64>]) -> LogCurveData {
        LogCurveData {
            curve_name: name.to_string(),
            original_mnemonic: name.to_string(),
            unit: Some("unit".to_string()),
            semantic_type: semantic,
            depths: vec![100.0, 100.5, 101.0, 101.5],
            values: values.to_vec(),
        }
    }

    #[test]
    fn vshale_is_only_available_for_gamma_ray() {
        let registry = ComputeRegistry::new();
        let catalog = registry.catalog_for_log_asset(
            &[descriptor("GR", "GR", CurveSemanticType::GammaRay)],
            &["GR".to_string()],
        );
        let vshale = catalog
            .functions
            .iter()
            .find(|entry| entry.metadata.id == "petro:vshale_linear")
            .unwrap();
        assert!(matches!(
            vshale.availability,
            ComputeAvailability::Available
        ));

        let unavailable = registry.catalog_for_log_asset(
            &[descriptor("RHOB", "RHOB", CurveSemanticType::BulkDensity)],
            &["RHOB".to_string()],
        );
        let vshale = unavailable
            .functions
            .iter()
            .find(|entry| entry.metadata.id == "petro:vshale_linear")
            .unwrap();
        assert!(matches!(
            vshale.availability,
            ComputeAvailability::Unavailable { .. }
        ));
    }

    #[test]
    fn run_vshale_linear_requires_gr_and_returns_vshale_curve() {
        let registry = ComputeRegistry::new();
        let mut bindings = BTreeMap::new();
        bindings.insert("gr_curve".to_string(), "GR".to_string());
        let mut parameters = BTreeMap::new();
        parameters.insert("gr_min".to_string(), ComputeParameterValue::Number(30.0));
        parameters.insert("gr_max".to_string(), ComputeParameterValue::Number(100.0));

        let (_, output) = registry
            .run_log_compute(
                "petro:vshale_linear",
                &[curve(
                    "GR",
                    CurveSemanticType::GammaRay,
                    &[Some(30.0), Some(65.0), Some(100.0), None],
                )],
                &bindings,
                &parameters,
                None,
            )
            .unwrap();

        assert_eq!(output.semantic_type, CurveSemanticType::VShale);
        assert_eq!(output.curve_name, "VSH_LIN");
        assert_eq!(output.values[0], Some(0.0));
        assert_eq!(output.values[2], Some(1.0));
        assert_eq!(output.values[3], None);
    }

    #[test]
    fn multi_curve_compute_rejects_mismatched_depths() {
        let registry = ComputeRegistry::new();
        let mut bindings = BTreeMap::new();
        bindings.insert("vp_curve".to_string(), "VP".to_string());
        bindings.insert("density_curve".to_string(), "RHOB".to_string());
        let vp = curve(
            "VP",
            CurveSemanticType::PVelocity,
            &[Some(2000.0), Some(2100.0), Some(2200.0), Some(2300.0)],
        );
        let mut rho = curve(
            "RHOB",
            CurveSemanticType::BulkDensity,
            &[Some(2.2), Some(2.3), Some(2.4), Some(2.5)],
        );
        rho.depths[2] = 999.0;
        let error = registry
            .run_log_compute(
                "rock_physics:acoustic_impedance",
                &[vp, rho],
                &bindings,
                &BTreeMap::new(),
                None,
            )
            .unwrap_err();
        assert!(error.to_string().contains("depth index diverges"));
    }

    #[test]
    fn structured_compute_catalogs_are_family_specific() {
        let registry = ComputeRegistry::new();
        let catalog = registry.catalog_for_trajectory_asset();
        assert_eq!(catalog.asset_family, AssetSemanticFamily::Trajectory);
        assert!(
            catalog
                .functions
                .iter()
                .any(|entry| entry.metadata.id == "trajectory:smooth_inclination")
        );
    }

    #[test]
    fn structured_compute_runs_and_preserves_shape() {
        let registry = ComputeRegistry::new();
        let mut parameters = BTreeMap::new();
        parameters.insert("window".to_string(), ComputeParameterValue::Number(3.0));
        let (_, rows) = registry
            .run_pressure_compute(
                "pressure:moving_average",
                &[
                    PressureObservationDataRow {
                        measured_depth: Some(1000.0),
                        pressure: 4500.0,
                        phase: Some("oil".to_string()),
                        test_kind: Some("mdt".to_string()),
                        timestamp: None,
                    },
                    PressureObservationDataRow {
                        measured_depth: Some(1001.0),
                        pressure: 4800.0,
                        phase: Some("oil".to_string()),
                        test_kind: Some("mdt".to_string()),
                        timestamp: None,
                    },
                    PressureObservationDataRow {
                        measured_depth: Some(1002.0),
                        pressure: 5100.0,
                        phase: Some("oil".to_string()),
                        test_kind: Some("mdt".to_string()),
                        timestamp: None,
                    },
                ],
                &parameters,
            )
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows[1].pressure > 4700.0 && rows[1].pressure < 4900.0);
    }
}
