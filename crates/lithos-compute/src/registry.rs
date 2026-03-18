use crate::functions::{
    ComputeExecutionManifest, ComputeFunctionMetadata, ComputeInputBinding, ComputeParameterValue,
    ComputedCurve, LogCurveData,
};
use crate::semantics::{
    AssetSemanticFamily, CurveBindingCandidate, CurveSemanticDescriptor, CurveSemanticType,
};
use lithos_core::{LasError, Result};
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

trait ComputeFunction: Send + Sync {
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

#[derive(Default)]
pub struct ComputeRegistry {
    functions: Vec<Box<dyn ComputeFunction>>,
}

impl ComputeRegistry {
    pub fn new() -> Self {
        Self {
            functions: vec![
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
        }
    }

    pub fn catalog_for_log_asset(
        &self,
        curves: &[CurveSemanticDescriptor],
        numeric_curve_names: &[String],
    ) -> ComputeCatalog {
        let functions = self
            .functions
            .iter()
            .map(|function| {
                catalog_entry_for_function(function.as_ref(), curves, numeric_curve_names)
            })
            .collect();

        ComputeCatalog {
            asset_family: AssetSemanticFamily::Log,
            functions,
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
            .functions
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

fn catalog_entry_for_function(
    function: &dyn ComputeFunction,
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
                if let Some(minimum) = min {
                    if value < minimum {
                        return Err(LasError::Validation(format!(
                            "parameter '{}' must be >= {}",
                            name, minimum
                        )));
                    }
                }
                if let Some(maximum) = max {
                    if value > maximum {
                        return Err(LasError::Validation(format!(
                            "parameter '{}' must be <= {}",
                            name, maximum
                        )));
                    }
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

impl ComputeFunction for MovingAverageFunction {
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
        let mut window = numeric_param_value(parameters, "window", Some(5.0))?.round() as usize;
        if window == 0 {
            window = 1;
        }
        if window % 2 == 0 {
            window += 1;
        }
        let radius = window / 2;
        let mut values = Vec::with_capacity(curve.values.len());
        for index in 0..curve.values.len() {
            let start = index.saturating_sub(radius);
            let stop = (index + radius + 1).min(curve.values.len());
            let mut sum = 0.0;
            let mut count = 0usize;
            for value in &curve.values[start..stop] {
                if let Some(number) = value {
                    sum += *number;
                    count += 1;
                }
            }
            values.push((count > 0).then_some(sum / count as f64));
        }
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("MA").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("MA").to_string(),
            unit: curve.unit.clone(),
            description: Some(format!(
                "Moving average ({window}-sample) of {}",
                curve.curve_name
            )),
            semantic_type: CurveSemanticType::Computed,
            values,
        })
    }
}

impl ComputeFunction for ZScoreNormalizeFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "core:zscore_normalize".to_string(),
            provider: "core".to_string(),
            name: "Z-score Normalize".to_string(),
            category: "Transforms".to_string(),
            description: "Normalize a numeric log curve to zero mean and unit variance."
                .to_string(),
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
            LasError::Validation("z-score normalization requires a bound input curve".to_string())
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
        let values = if std_dev == 0.0 {
            curve
                .values
                .iter()
                .map(|value| value.map(|_| 0.0))
                .collect()
        } else {
            curve
                .values
                .iter()
                .map(|value| value.map(|number| (number - mean) / std_dev))
                .collect()
        };
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

impl ComputeFunction for MinMaxScaleFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "core:min_max_scale".to_string(),
            provider: "core".to_string(),
            name: "Min-Max Scale".to_string(),
            category: "Transforms".to_string(),
            description: "Rescale a numeric log curve to a configurable interval.".to_string(),
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
                "Lower bound of output.",
                Some(0.0),
                None,
                None,
                None,
            ),
            number_param(
                "target_max",
                "Target Max",
                "Upper bound of output.",
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
        if target_max <= target_min {
            return Err(LasError::Validation(
                "target_max must be greater than target_min".to_string(),
            ));
        }
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

impl ComputeFunction for GapFlagFunction {
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
        impl ComputeFunction for $name {
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
                    .map(|value| value.map(|number| $body(number, gr_min, span)))
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
            (1.7 - inner.sqrt()).clamp(0.0, 1.0)
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
            (igr / denominator).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
);

macro_rules! impl_velocity_from_slowness {
    ($name:ident, $id:literal, $display:literal, $param:literal, $allowed:expr, $mnemonic:literal, $semantic:expr, $description:literal) => {
        impl ComputeFunction for $name {
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

impl ComputeFunction for AcousticImpedanceFunction {
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
        let values = vp
            .values
            .iter()
            .zip(&density.values)
            .map(|(left, right)| match (left, right) {
                (Some(vp), Some(rho)) => Some(vp * rho),
                _ => None,
            })
            .collect();
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("AI").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("AI").to_string(),
            unit: Some("(m/s)*(g/cm3)".to_string()),
            description: Some(format!(
                "Acoustic impedance from {} and {}",
                vp.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::AcousticImpedance,
            values,
        })
    }
}

impl ComputeFunction for PoissonsRatioFunction {
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
        let values = vp
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
            .collect();
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("PR").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("PR").to_string(),
            unit: None,
            description: Some(format!(
                "Poisson's ratio from {} and {}",
                vp.curve_name, vs.curve_name
            )),
            semantic_type: CurveSemanticType::PoissonsRatio,
            values,
        })
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
}
