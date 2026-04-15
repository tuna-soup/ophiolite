use crate::functions::{
    ComputeExecutionManifest, ComputeFunctionMetadata, ComputeInputBinding, ComputeParameterValue,
    ComputedCurve, DrillingObservationDataRow, LogCurveData, PressureObservationDataRow,
    TopDataRow, TrajectoryDataRow,
};
use crate::operators::{
    BUILTIN_OPERATOR_PACKAGE_NAME, OperatorManifest, OperatorOutputLifecycle,
    OperatorPackageCompatibility, OperatorPackageManifest, OperatorRuntimeKind, OperatorStability,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
struct VpVsRatioFunction;
struct AcousticImpedanceFunction;
struct ElasticImpedanceFunction;
struct ExtendedElasticImpedanceFunction;
struct ShearImpedanceFunction;
struct LambdaRhoFunction;
struct MuRhoFunction;
struct PoissonsRatioFunction;
struct GassmannSubstitutedDensityFunction;
struct GassmannSubstitutedVpFunction;
struct GassmannSubstitutedVsFunction;
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
                Box::new(VpVsRatioFunction),
                Box::new(AcousticImpedanceFunction),
                Box::new(ElasticImpedanceFunction),
                Box::new(ExtendedElasticImpedanceFunction),
                Box::new(ShearImpedanceFunction),
                Box::new(LambdaRhoFunction),
                Box::new(MuRhoFunction),
                Box::new(PoissonsRatioFunction),
                Box::new(GassmannSubstitutedDensityFunction),
                Box::new(GassmannSubstitutedVpFunction),
                Box::new(GassmannSubstitutedVsFunction),
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

    pub fn built_in_operator_package_manifest(&self) -> OperatorPackageManifest {
        let mut operators = Vec::new();
        operators.extend(
            self.log_functions
                .iter()
                .map(|function| operator_manifest_for_log(function.as_ref())),
        );
        operators.extend(
            self.trajectory_functions
                .iter()
                .map(|function| operator_manifest_for_trajectory(function.as_ref())),
        );
        operators.extend(
            self.top_set_functions
                .iter()
                .map(|function| operator_manifest_for_top_set(function.as_ref())),
        );
        operators.extend(
            self.pressure_functions
                .iter()
                .map(|function| operator_manifest_for_pressure(function.as_ref())),
        );
        operators.extend(
            self.drilling_functions
                .iter()
                .map(|function| operator_manifest_for_drilling(function.as_ref())),
        );

        OperatorPackageManifest {
            schema_version: 1,
            package_name: BUILTIN_OPERATOR_PACKAGE_NAME.to_string(),
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            provider: "ophiolite".to_string(),
            runtime: OperatorRuntimeKind::Rust,
            compatibility: OperatorPackageCompatibility {
                ophiolite_api: env!("CARGO_PKG_VERSION").to_string(),
            },
            entrypoint: None,
            operators,
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

        let input_specs = function.input_specs();
        let parameter_definitions = function.parameters();
        let (resolved_inputs, manifest_inputs) =
            resolve_log_input_bindings(function_id, &input_specs, curves, bindings)?;
        validate_compute_parameters(&parameter_definitions, parameters)?;
        let output = function.execute(&resolved_inputs, parameters, output_mnemonic)?;
        let metadata = function.metadata();

        Ok((
            ComputeExecutionManifest {
                function_id: metadata.id,
                provider: metadata.provider,
                function_name: metadata.name,
                function_version: function.version().to_string(),
                operator_package: Some(BUILTIN_OPERATOR_PACKAGE_NAME.to_string()),
                operator_package_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                operator_runtime: Some(OperatorRuntimeKind::Rust),
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
        let parameter_definitions = function.parameters();
        validate_compute_parameters(&parameter_definitions, parameters)?;
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
        let parameter_definitions = function.parameters();
        validate_compute_parameters(&parameter_definitions, parameters)?;
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
        let parameter_definitions = function.parameters();
        validate_compute_parameters(&parameter_definitions, parameters)?;
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
        let parameter_definitions = function.parameters();
        validate_compute_parameters(&parameter_definitions, parameters)?;
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
    let binding_candidates =
        binding_candidates_for_input_specs(&input_specs, curves, numeric_curve_names);

    ComputeCatalogEntry {
        metadata,
        input_specs,
        parameters,
        binding_candidates: binding_candidates.clone(),
        availability: availability_for_binding_candidates(&binding_candidates),
    }
}

pub fn catalog_entry_for_operator_manifest(
    operator: &OperatorManifest,
    curves: Option<(&[CurveSemanticDescriptor], &[String])>,
) -> ComputeCatalogEntry {
    crate::operators::available_catalog_entry_for_operator(operator, curves)
}

pub fn binding_candidates_for_input_specs(
    input_specs: &[ComputeInputSpec],
    curves: &[CurveSemanticDescriptor],
    numeric_curve_names: &[String],
) -> Vec<ComputeBindingCandidate> {
    input_specs
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
        .collect()
}

pub fn availability_for_binding_candidates(
    binding_candidates: &[ComputeBindingCandidate],
) -> ComputeAvailability {
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

    if reasons.is_empty() {
        ComputeAvailability::Available
    } else {
        ComputeAvailability::Unavailable { reasons }
    }
}

pub fn resolve_log_input_bindings(
    function_id: &str,
    input_specs: &[ComputeInputSpec],
    curves: &[LogCurveData],
    bindings: &BTreeMap<String, String>,
) -> Result<(BTreeMap<String, LogCurveData>, Vec<ComputeInputBinding>)> {
    let available = curves
        .iter()
        .map(|curve| (curve.curve_name.clone(), curve))
        .collect::<BTreeMap<_, _>>();
    let mut resolved_inputs = BTreeMap::new();
    let mut manifest_inputs = Vec::new();

    for spec in input_specs {
        match spec {
            ComputeInputSpec::SingleCurve {
                parameter_name,
                allowed_types,
            } => bind_curve_input(
                &available,
                &mut resolved_inputs,
                &mut manifest_inputs,
                bindings,
                parameter_name,
                allowed_types,
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
                    left_parameter_name,
                    left_allowed_types,
                )?;
                bind_curve_input(
                    &available,
                    &mut resolved_inputs,
                    &mut manifest_inputs,
                    bindings,
                    right_parameter_name,
                    right_allowed_types,
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
    Ok((resolved_inputs, manifest_inputs))
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
            semantic_parameters: curve.semantic_parameters.clone(),
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

fn operator_manifest(
    metadata: ComputeFunctionMetadata,
    asset_family: AssetSemanticFamily,
    input_specs: Vec<ComputeInputSpec>,
    parameters: Vec<ComputeParameterDefinition>,
    deterministic: bool,
) -> OperatorManifest {
    OperatorManifest {
        id: metadata.id,
        provider: metadata.provider,
        name: metadata.name,
        asset_family,
        category: metadata.category,
        description: metadata.description,
        default_output_mnemonic: metadata.default_output_mnemonic,
        output_curve_type: metadata.output_curve_type,
        input_specs,
        parameters,
        output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
        deterministic,
        stability: OperatorStability::Preview,
        tags: metadata.tags,
    }
}

fn operator_manifest_for_log(function: &dyn LogComputeFunction) -> OperatorManifest {
    operator_manifest(
        function.metadata(),
        AssetSemanticFamily::Log,
        function.input_specs(),
        function.parameters(),
        function.is_deterministic(),
    )
}

fn operator_manifest_for_trajectory(function: &dyn TrajectoryComputeFunction) -> OperatorManifest {
    operator_manifest(
        function.metadata(),
        AssetSemanticFamily::Trajectory,
        vec![ComputeInputSpec::Trajectory],
        function.parameters(),
        function.is_deterministic(),
    )
}

fn operator_manifest_for_top_set(function: &dyn TopSetComputeFunction) -> OperatorManifest {
    operator_manifest(
        function.metadata(),
        AssetSemanticFamily::TopSet,
        vec![ComputeInputSpec::TopSet],
        function.parameters(),
        function.is_deterministic(),
    )
}

fn operator_manifest_for_pressure(function: &dyn PressureComputeFunction) -> OperatorManifest {
    operator_manifest(
        function.metadata(),
        AssetSemanticFamily::PressureObservation,
        vec![ComputeInputSpec::PressureObservation],
        function.parameters(),
        function.is_deterministic(),
    )
}

fn operator_manifest_for_drilling(function: &dyn DrillingComputeFunction) -> OperatorManifest {
    operator_manifest(
        function.metadata(),
        AssetSemanticFamily::DrillingObservation,
        vec![ComputeInputSpec::DrillingObservation],
        function.parameters(),
        function.is_deterministic(),
    )
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
        operator_package: Some(BUILTIN_OPERATOR_PACKAGE_NAME.to_string()),
        operator_package_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        operator_runtime: Some(OperatorRuntimeKind::Rust),
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

pub fn validate_compute_parameters(
    definitions: &[ComputeParameterDefinition],
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
                    .get(name)
                    .and_then(ComputeParameterValue::as_f64)
                    .or(*default)
                    .ok_or_else(|| {
                        LasError::Validation(format!(
                            "numeric compute parameter '{}' is required",
                            name
                        ))
                    })?;
                if let Some(minimum) = min
                    && value < *minimum
                {
                    return Err(LasError::Validation(format!(
                        "parameter '{}' must be >= {}",
                        name, minimum
                    )));
                }
                if let Some(maximum) = max
                    && value > *maximum
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

#[derive(Debug, Clone, Copy)]
struct GassmannParameters {
    matrix_bulk_modulus_gpa: f64,
    initial_fluid_bulk_modulus_gpa: f64,
    substituted_fluid_bulk_modulus_gpa: f64,
    initial_fluid_density_gcc: f64,
    substituted_fluid_density_gcc: f64,
}

#[derive(Debug, Clone, Copy)]
struct GassmannSubstitution {
    substituted_vp_m_per_s: f64,
    substituted_vs_m_per_s: f64,
}

#[derive(Debug, Clone, Copy)]
struct ImpedanceReferenceTerms {
    vp0_m_per_s: f64,
    vs0_m_per_s: f64,
    density0_gcc: f64,
    velocity_ratio_k: f64,
}

fn density_to_gcc(value: f64, unit: Option<&str>) -> Option<f64> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let normalized = unit
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "");
    if normalized.contains("kg/m3") || normalized.contains("kg/m^3") || normalized.contains("kg/m³")
    {
        Some(value / 1000.0)
    } else if value > 10.0 {
        Some(value / 1000.0)
    } else {
        Some(value)
    }
}

fn porosity_to_fraction(value: f64, unit: Option<&str>) -> Option<f64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let normalized = unit.unwrap_or_default().trim().to_ascii_lowercase();
    let fraction = if normalized.contains('%')
        || normalized.contains("pct")
        || normalized.contains("percent")
        || normalized.contains("pu")
        || value > 1.0
    {
        value / 100.0
    } else {
        value
    };
    (0.0..=1.0).contains(&fraction).then_some(fraction)
}

fn lambda_rho_gpa(vp_m_per_s: f64, vs_m_per_s: f64, density_gcc: f64) -> Option<f64> {
    if !vp_m_per_s.is_finite()
        || !vs_m_per_s.is_finite()
        || !density_gcc.is_finite()
        || vp_m_per_s <= 0.0
        || vs_m_per_s <= 0.0
        || density_gcc <= 0.0
    {
        return None;
    }
    let vp_km_per_s = vp_m_per_s / 1000.0;
    let vs_km_per_s = vs_m_per_s / 1000.0;
    Some(density_gcc * (vp_km_per_s.powi(2) - (2.0 * vs_km_per_s.powi(2))))
}

fn mu_rho_gpa(vs_m_per_s: f64, density_gcc: f64) -> Option<f64> {
    if !vs_m_per_s.is_finite()
        || !density_gcc.is_finite()
        || vs_m_per_s <= 0.0
        || density_gcc <= 0.0
    {
        return None;
    }
    let vs_km_per_s = vs_m_per_s / 1000.0;
    Some(density_gcc * vs_km_per_s.powi(2))
}

fn empty_semantic_parameters() -> BTreeMap<String, f64> {
    BTreeMap::new()
}

fn impedance_reference_terms(
    vp: &LogCurveData,
    vs: &LogCurveData,
    density: &LogCurveData,
) -> Option<ImpedanceReferenceTerms> {
    let mut count = 0usize;
    let mut vp_sum = 0.0;
    let mut vs_sum = 0.0;
    let mut density_sum = 0.0;
    let mut ratio_sum = 0.0;

    for ((vp_value, vs_value), density_value) in vp.values.iter().zip(&vs.values).zip(&density.values)
    {
        let (Some(vp_value), Some(vs_value), Some(density_value)) = (vp_value, vs_value, density_value)
        else {
            continue;
        };
        let Some(density_gcc) = density_to_gcc(*density_value, density.unit.as_deref()) else {
            continue;
        };
        if *vp_value <= 0.0 || *vs_value <= 0.0 {
            continue;
        }

        vp_sum += *vp_value;
        vs_sum += *vs_value;
        density_sum += density_gcc;
        ratio_sum += (*vs_value / *vp_value).powi(2);
        count += 1;
    }

    (count > 0).then_some(ImpedanceReferenceTerms {
        vp0_m_per_s: vp_sum / count as f64,
        vs0_m_per_s: vs_sum / count as f64,
        density0_gcc: density_sum / count as f64,
        velocity_ratio_k: ratio_sum / count as f64,
    })
}

fn impedance_semantic_parameters(
    angle_parameter_name: &str,
    angle_deg: f64,
    terms: ImpedanceReferenceTerms,
) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (angle_parameter_name.to_string(), angle_deg),
        (
            "normalization_reference_vp_m_per_s".to_string(),
            terms.vp0_m_per_s,
        ),
        (
            "normalization_reference_vs_m_per_s".to_string(),
            terms.vs0_m_per_s,
        ),
        (
            "normalization_reference_density_g_cc".to_string(),
            terms.density0_gcc,
        ),
        ("velocity_ratio_k".to_string(), terms.velocity_ratio_k),
    ])
}

fn normalized_elastic_impedance_sample(
    vp_m_per_s: f64,
    vs_m_per_s: f64,
    density_gcc: f64,
    angle_deg: f64,
    terms: ImpedanceReferenceTerms,
) -> Option<f64> {
    if !vp_m_per_s.is_finite()
        || !vs_m_per_s.is_finite()
        || !density_gcc.is_finite()
        || vp_m_per_s <= 0.0
        || vs_m_per_s <= 0.0
        || density_gcc <= 0.0
    {
        return None;
    }

    let theta = angle_deg.to_radians();
    let a = 1.0 + theta.tan().powi(2);
    let b = -8.0 * terms.velocity_ratio_k * theta.sin().powi(2);
    let c = 1.0 - 4.0 * terms.velocity_ratio_k * theta.sin().powi(2);

    Some(
        vp_m_per_s.powf(a)
            * vs_m_per_s.powf(b)
            * density_gcc.powf(c)
            * terms.vp0_m_per_s.powf(1.0 - a)
            * terms.vs0_m_per_s.powf(-b)
            * terms.density0_gcc.powf(1.0 - c),
    )
}

fn extended_elastic_impedance_sample(
    vp_m_per_s: f64,
    vs_m_per_s: f64,
    density_gcc: f64,
    chi_deg: f64,
    terms: ImpedanceReferenceTerms,
) -> Option<f64> {
    if !vp_m_per_s.is_finite()
        || !vs_m_per_s.is_finite()
        || !density_gcc.is_finite()
        || vp_m_per_s <= 0.0
        || vs_m_per_s <= 0.0
        || density_gcc <= 0.0
    {
        return None;
    }

    let chi = chi_deg.to_radians();
    let p = chi.cos() + chi.sin();
    let q = -8.0 * terms.velocity_ratio_k * chi.sin();
    let r = chi.cos() - 4.0 * terms.velocity_ratio_k * chi.sin();

    Some(
        terms.vp0_m_per_s
            * terms.density0_gcc
            * (vp_m_per_s / terms.vp0_m_per_s).powf(p)
            * (vs_m_per_s / terms.vs0_m_per_s).powf(q)
            * (density_gcc / terms.density0_gcc).powf(r),
    )
}

fn gassmann_parameters() -> Vec<ComputeParameterDefinition> {
    vec![
        number_param(
            "matrix_bulk_modulus_gpa",
            "Matrix Bulk Modulus",
            "Mineral frame bulk modulus K0 in GPa.",
            Some(37.0),
            Some(1.0e-6),
            None,
            Some("GPa"),
        ),
        number_param(
            "initial_fluid_bulk_modulus_gpa",
            "Initial Fluid Bulk Modulus",
            "Initial pore-fluid bulk modulus Kfl1 in GPa.",
            Some(2.3),
            Some(1.0e-6),
            None,
            Some("GPa"),
        ),
        number_param(
            "substituted_fluid_bulk_modulus_gpa",
            "Substituted Fluid Bulk Modulus",
            "Substituted pore-fluid bulk modulus Kfl2 in GPa.",
            Some(0.05),
            Some(1.0e-6),
            None,
            Some("GPa"),
        ),
        number_param(
            "initial_fluid_density_gcc",
            "Initial Fluid Density",
            "Initial pore-fluid density in g/cc.",
            Some(1.0),
            Some(1.0e-6),
            None,
            Some("g/cc"),
        ),
        number_param(
            "substituted_fluid_density_gcc",
            "Substituted Fluid Density",
            "Substituted pore-fluid density in g/cc.",
            Some(0.2),
            Some(1.0e-6),
            None,
            Some("g/cc"),
        ),
    ]
}

fn resolve_gassmann_parameters(
    parameters: &BTreeMap<String, ComputeParameterValue>,
) -> Result<GassmannParameters> {
    Ok(GassmannParameters {
        matrix_bulk_modulus_gpa: numeric_param_value(
            parameters,
            "matrix_bulk_modulus_gpa",
            Some(37.0),
        )?,
        initial_fluid_bulk_modulus_gpa: numeric_param_value(
            parameters,
            "initial_fluid_bulk_modulus_gpa",
            Some(2.3),
        )?,
        substituted_fluid_bulk_modulus_gpa: numeric_param_value(
            parameters,
            "substituted_fluid_bulk_modulus_gpa",
            Some(0.05),
        )?,
        initial_fluid_density_gcc: numeric_param_value(
            parameters,
            "initial_fluid_density_gcc",
            Some(1.0),
        )?,
        substituted_fluid_density_gcc: numeric_param_value(
            parameters,
            "substituted_fluid_density_gcc",
            Some(0.2),
        )?,
    })
}

fn gassmann_substitution_sample(
    vp_m_per_s: f64,
    vs_m_per_s: f64,
    density_value: f64,
    density_unit: Option<&str>,
    porosity_value: f64,
    porosity_unit: Option<&str>,
    parameters: GassmannParameters,
) -> Option<GassmannSubstitution> {
    let density_gcc = density_to_gcc(density_value, density_unit)?;
    let porosity_fraction = porosity_to_fraction(porosity_value, porosity_unit)?;
    if vp_m_per_s <= 0.0 || vs_m_per_s <= 0.0 || porosity_fraction <= 0.0 {
        return None;
    }

    let shear_modulus_gpa = density_gcc * vs_m_per_s.powi(2) * 1.0e-6;
    let saturated_bulk_modulus_gpa =
        (density_gcc * vp_m_per_s.powi(2) * 1.0e-6) - ((4.0 / 3.0) * shear_modulus_gpa);
    if !shear_modulus_gpa.is_finite()
        || !saturated_bulk_modulus_gpa.is_finite()
        || shear_modulus_gpa <= 0.0
        || saturated_bulk_modulus_gpa <= 0.0
    {
        return None;
    }

    let matrix_minus_sat = parameters.matrix_bulk_modulus_gpa - saturated_bulk_modulus_gpa;
    let matrix_minus_initial =
        parameters.matrix_bulk_modulus_gpa - parameters.initial_fluid_bulk_modulus_gpa;
    let matrix_minus_substituted =
        parameters.matrix_bulk_modulus_gpa - parameters.substituted_fluid_bulk_modulus_gpa;
    if matrix_minus_sat.abs() <= f64::EPSILON
        || matrix_minus_initial.abs() <= f64::EPSILON
        || matrix_minus_substituted.abs() <= f64::EPSILON
    {
        return None;
    }

    let substituted_density_gcc = density_gcc
        - (porosity_fraction * parameters.initial_fluid_density_gcc)
        + (porosity_fraction * parameters.substituted_fluid_density_gcc);
    if !substituted_density_gcc.is_finite() || substituted_density_gcc <= 0.0 {
        return None;
    }

    let gassmann_term = (saturated_bulk_modulus_gpa / matrix_minus_sat)
        - (parameters.initial_fluid_bulk_modulus_gpa / (porosity_fraction * matrix_minus_initial))
        + (parameters.substituted_fluid_bulk_modulus_gpa
            / (porosity_fraction * matrix_minus_substituted));
    if !gassmann_term.is_finite() || (1.0 + gassmann_term).abs() <= f64::EPSILON {
        return None;
    }

    let substituted_bulk_modulus_gpa =
        (gassmann_term * parameters.matrix_bulk_modulus_gpa) / (1.0 + gassmann_term);
    if !substituted_bulk_modulus_gpa.is_finite() || substituted_bulk_modulus_gpa <= 0.0 {
        return None;
    }

    let substituted_vp_term = (substituted_bulk_modulus_gpa + ((4.0 / 3.0) * shear_modulus_gpa))
        / substituted_density_gcc;
    let substituted_vs_term = shear_modulus_gpa / substituted_density_gcc;
    if !substituted_vp_term.is_finite()
        || !substituted_vs_term.is_finite()
        || substituted_vp_term <= 0.0
        || substituted_vs_term <= 0.0
    {
        return None;
    }

    Some(GassmannSubstitution {
        substituted_vp_m_per_s: substituted_vp_term.sqrt() * 1000.0,
        substituted_vs_m_per_s: substituted_vs_term.sqrt() * 1000.0,
    })
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
            semantic_parameters: empty_semantic_parameters(),
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
            semantic_parameters: empty_semantic_parameters(),
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
            semantic_parameters: empty_semantic_parameters(),
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
            semantic_parameters: empty_semantic_parameters(),
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
                    semantic_parameters: empty_semantic_parameters(),
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
                    semantic_parameters: empty_semantic_parameters(),
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

impl LogComputeFunction for VpVsRatioFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:vp_vs_ratio".to_string(),
            provider: "rock_physics".to_string(),
            name: "Vp/Vs Ratio".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute Vp/Vs ratio from P-wave and S-wave velocity.".to_string(),
            default_output_mnemonic: "VPVS".to_string(),
            output_curve_type: CurveSemanticType::VpVsRatio,
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
            curve_name: output_mnemonic.unwrap_or("VPVS").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("VPVS").to_string(),
            unit: Some("ratio".to_string()),
            description: Some(format!(
                "Vp/Vs ratio from {} and {}",
                vp.curve_name, vs.curve_name
            )),
            semantic_type: CurveSemanticType::VpVsRatio,
            semantic_parameters: empty_semantic_parameters(),
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .map(|(left, right)| match (left, right) {
                    (Some(vp_value), Some(vs_value)) if *vp_value > 0.0 && *vs_value > 0.0 => {
                        Some(vp_value / vs_value)
                    }
                    _ => None,
                })
                .collect(),
        })
    }
}

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
            unit: Some("(m/s)*(g/cc)".to_string()),
            description: Some(format!(
                "Acoustic impedance from {} and {}",
                vp.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::AcousticImpedance,
            semantic_parameters: empty_semantic_parameters(),
            values: vp
                .values
                .iter()
                .zip(&density.values)
                .map(|(left, right)| match (left, right) {
                    (Some(vp_value), Some(rho_value)) => {
                        density_to_gcc(*rho_value, density.unit.as_deref())
                            .map(|density_gcc| vp_value * density_gcc)
                    }
                    _ => None,
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for ElasticImpedanceFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:elastic_impedance".to_string(),
            provider: "rock_physics".to_string(),
            name: "Elastic Impedance".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute normalized elastic impedance from Vp, Vs, and bulk density."
                .to_string(),
            default_output_mnemonic: "EI".to_string(),
            output_curve_type: CurveSemanticType::ElasticImpedance,
            tags: vec![
                "rock-physics".to_string(),
                "impedance".to_string(),
                "avo".to_string(),
            ],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "vp_curve".to_string(),
                allowed_types: vec![CurveSemanticType::PVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "vs_curve".to_string(),
                allowed_types: vec![CurveSemanticType::SVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
        ]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![number_param(
            "angle_deg",
            "Incident Angle",
            "Incident angle in degrees for normalized elastic impedance.",
            Some(0.0),
            Some(0.0),
            Some(89.0),
            Some("deg"),
        )]
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let vp = inputs
            .get("vp_curve")
            .ok_or_else(|| LasError::Validation("vp curve is required".to_string()))?;
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        let angle_deg = numeric_param_value(parameters, "angle_deg", Some(0.0))?;
        if !angle_deg.is_finite() || !(0.0..90.0).contains(&angle_deg) {
            return Err(LasError::Validation(
                "parameter 'angle_deg' must be in [0, 90) degrees".to_string(),
            ));
        }
        let terms = impedance_reference_terms(vp, vs, density).ok_or_else(|| {
            LasError::Validation(
                "elastic impedance requires at least one positive Vp, Vs, and density sample"
                    .to_string(),
            )
        })?;

        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("EI").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("EI").to_string(),
            unit: Some("(m/s)*(g/cc)".to_string()),
            description: Some(format!(
                "Elastic impedance from {}, {}, and {} at {angle_deg} deg",
                vp.curve_name, vs.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::ElasticImpedance,
            semantic_parameters: impedance_semantic_parameters("incident_angle_deg", angle_deg, terms),
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .zip(&density.values)
                .map(|((vp_value, vs_value), density_value)| {
                    match (vp_value, vs_value, density_value) {
                        (Some(vp_value), Some(vs_value), Some(density_value)) => {
                            density_to_gcc(*density_value, density.unit.as_deref()).and_then(
                                |density_gcc| {
                                    normalized_elastic_impedance_sample(
                                        *vp_value,
                                        *vs_value,
                                        density_gcc,
                                        angle_deg,
                                        terms,
                                    )
                                },
                            )
                        }
                        _ => None,
                    }
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for ExtendedElasticImpedanceFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:extended_elastic_impedance".to_string(),
            provider: "rock_physics".to_string(),
            name: "Extended Elastic Impedance".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute rotated extended elastic impedance from Vp, Vs, and bulk density."
                .to_string(),
            default_output_mnemonic: "EEI".to_string(),
            output_curve_type: CurveSemanticType::ExtendedElasticImpedance,
            tags: vec![
                "rock-physics".to_string(),
                "impedance".to_string(),
                "avo".to_string(),
            ],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "vp_curve".to_string(),
                allowed_types: vec![CurveSemanticType::PVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "vs_curve".to_string(),
                allowed_types: vec![CurveSemanticType::SVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
        ]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        vec![number_param(
            "chi_deg",
            "Chi Rotation",
            "Rotation angle chi in degrees for extended elastic impedance.",
            Some(0.0),
            Some(-90.0),
            Some(90.0),
            Some("deg"),
        )]
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let vp = inputs
            .get("vp_curve")
            .ok_or_else(|| LasError::Validation("vp curve is required".to_string()))?;
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        let chi_deg = numeric_param_value(parameters, "chi_deg", Some(0.0))?;
        if !chi_deg.is_finite() || !(-90.0..=90.0).contains(&chi_deg) {
            return Err(LasError::Validation(
                "parameter 'chi_deg' must be in [-90, 90] degrees".to_string(),
            ));
        }
        let terms = impedance_reference_terms(vp, vs, density).ok_or_else(|| {
            LasError::Validation(
                "extended elastic impedance requires at least one positive Vp, Vs, and density sample"
                    .to_string(),
            )
        })?;

        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("EEI").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("EEI").to_string(),
            unit: Some("(m/s)*(g/cc)".to_string()),
            description: Some(format!(
                "Extended elastic impedance from {}, {}, and {} at chi {chi_deg} deg",
                vp.curve_name, vs.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::ExtendedElasticImpedance,
            semantic_parameters: impedance_semantic_parameters("chi_angle_deg", chi_deg, terms),
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .zip(&density.values)
                .map(|((vp_value, vs_value), density_value)| {
                    match (vp_value, vs_value, density_value) {
                        (Some(vp_value), Some(vs_value), Some(density_value)) => {
                            density_to_gcc(*density_value, density.unit.as_deref()).and_then(
                                |density_gcc| {
                                    extended_elastic_impedance_sample(
                                        *vp_value,
                                        *vs_value,
                                        density_gcc,
                                        chi_deg,
                                        terms,
                                    )
                                },
                            )
                        }
                        _ => None,
                    }
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for ShearImpedanceFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:shear_impedance".to_string(),
            provider: "rock_physics".to_string(),
            name: "Shear Impedance".to_string(),
            category: "Rock Physics".to_string(),
            description: "Multiply S-wave velocity by bulk density.".to_string(),
            default_output_mnemonic: "SI".to_string(),
            output_curve_type: CurveSemanticType::ShearImpedance,
            tags: vec!["rock-physics".to_string(), "impedance".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![ComputeInputSpec::CurvePair {
            left_parameter_name: "vs_curve".to_string(),
            left_allowed_types: vec![CurveSemanticType::SVelocity],
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
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("SI").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("SI").to_string(),
            unit: Some("(m/s)*(g/cc)".to_string()),
            description: Some(format!(
                "Shear impedance from {} and {}",
                vs.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::ShearImpedance,
            semantic_parameters: empty_semantic_parameters(),
            values: vs
                .values
                .iter()
                .zip(&density.values)
                .map(|(left, right)| match (left, right) {
                    (Some(vs_value), Some(rho_value)) => {
                        density_to_gcc(*rho_value, density.unit.as_deref())
                            .map(|density_gcc| vs_value * density_gcc)
                    }
                    _ => None,
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for LambdaRhoFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:lambda_rho".to_string(),
            provider: "rock_physics".to_string(),
            name: "Lambda-Rho".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute lambda-rho from Vp, Vs, and bulk density.".to_string(),
            default_output_mnemonic: "LRHO".to_string(),
            output_curve_type: CurveSemanticType::LambdaRho,
            tags: vec!["rock-physics".to_string(), "elastic".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "vp_curve".to_string(),
                allowed_types: vec![CurveSemanticType::PVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "vs_curve".to_string(),
                allowed_types: vec![CurveSemanticType::SVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
        ]
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
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("LRHO").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("LRHO").to_string(),
            unit: Some("GPa".to_string()),
            description: Some(format!(
                "Lambda-rho from {}, {}, and {}",
                vp.curve_name, vs.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::LambdaRho,
            semantic_parameters: empty_semantic_parameters(),
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .zip(&density.values)
                .map(|((vp_value, vs_value), density_value)| {
                    match (vp_value, vs_value, density_value) {
                        (Some(vp_value), Some(vs_value), Some(density_value)) => {
                            density_to_gcc(*density_value, density.unit.as_deref()).and_then(
                                |density_gcc| lambda_rho_gpa(*vp_value, *vs_value, density_gcc),
                            )
                        }
                        _ => None,
                    }
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for MuRhoFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:mu_rho".to_string(),
            provider: "rock_physics".to_string(),
            name: "Mu-Rho".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute mu-rho from Vs and bulk density.".to_string(),
            default_output_mnemonic: "MRHO".to_string(),
            output_curve_type: CurveSemanticType::MuRho,
            tags: vec!["rock-physics".to_string(), "elastic".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "vs_curve".to_string(),
                allowed_types: vec![CurveSemanticType::SVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
        ]
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
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("MRHO").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("MRHO").to_string(),
            unit: Some("GPa".to_string()),
            description: Some(format!(
                "Mu-rho from {} and {}",
                vs.curve_name, density.curve_name
            )),
            semantic_type: CurveSemanticType::MuRho,
            semantic_parameters: empty_semantic_parameters(),
            values: vs
                .values
                .iter()
                .zip(&density.values)
                .map(
                    |(vs_value, density_value)| match (vs_value, density_value) {
                        (Some(vs_value), Some(density_value)) => {
                            density_to_gcc(*density_value, density.unit.as_deref())
                                .and_then(|density_gcc| mu_rho_gpa(*vs_value, density_gcc))
                        }
                        _ => None,
                    },
                )
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
            semantic_parameters: empty_semantic_parameters(),
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

impl LogComputeFunction for GassmannSubstitutedDensityFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:gassmann_substituted_density".to_string(),
            provider: "rock_physics".to_string(),
            name: "Gassmann Substituted Density".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute substituted bulk density from porosity and fluid properties."
                .to_string(),
            default_output_mnemonic: "RHOB_GASS".to_string(),
            output_curve_type: CurveSemanticType::BulkDensity,
            tags: vec!["rock-physics".to_string(), "gassmann".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "porosity_curve".to_string(),
                allowed_types: vec![
                    CurveSemanticType::EffectivePorosity,
                    CurveSemanticType::NeutronPorosity,
                ],
            },
        ]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        gassmann_parameters()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        let porosity = inputs
            .get("porosity_curve")
            .ok_or_else(|| LasError::Validation("porosity curve is required".to_string()))?;
        let parameters = resolve_gassmann_parameters(parameters)?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("RHOB_GASS").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("RHOB_GASS").to_string(),
            unit: Some("g/cc".to_string()),
            description: Some(format!(
                "Gassmann substituted density from {} and {}",
                density.curve_name, porosity.curve_name
            )),
            semantic_type: CurveSemanticType::BulkDensity,
            semantic_parameters: empty_semantic_parameters(),
            values: density
                .values
                .iter()
                .zip(&porosity.values)
                .map(
                    |(density_value, porosity_value)| match (density_value, porosity_value) {
                        (Some(density_value), Some(porosity_value)) => {
                            let density_gcc =
                                density_to_gcc(*density_value, density.unit.as_deref())?;
                            let porosity_fraction =
                                porosity_to_fraction(*porosity_value, porosity.unit.as_deref())?;
                            if porosity_fraction <= 0.0 {
                                return None;
                            }
                            let substituted_density_gcc = density_gcc
                                - (porosity_fraction * parameters.initial_fluid_density_gcc)
                                + (porosity_fraction * parameters.substituted_fluid_density_gcc);
                            (substituted_density_gcc > 0.0 && substituted_density_gcc.is_finite())
                                .then_some(substituted_density_gcc)
                        }
                        _ => None,
                    },
                )
                .collect(),
        })
    }
}

impl LogComputeFunction for GassmannSubstitutedVpFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:gassmann_substituted_vp".to_string(),
            provider: "rock_physics".to_string(),
            name: "Gassmann Substituted Vp".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute substituted P-wave velocity from Gassmann fluid substitution."
                .to_string(),
            default_output_mnemonic: "VP_GASS".to_string(),
            output_curve_type: CurveSemanticType::PVelocity,
            tags: vec!["rock-physics".to_string(), "gassmann".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "vp_curve".to_string(),
                allowed_types: vec![CurveSemanticType::PVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "vs_curve".to_string(),
                allowed_types: vec![CurveSemanticType::SVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "porosity_curve".to_string(),
                allowed_types: vec![
                    CurveSemanticType::EffectivePorosity,
                    CurveSemanticType::NeutronPorosity,
                ],
            },
        ]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        gassmann_parameters()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let vp = inputs
            .get("vp_curve")
            .ok_or_else(|| LasError::Validation("vp curve is required".to_string()))?;
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        let porosity = inputs
            .get("porosity_curve")
            .ok_or_else(|| LasError::Validation("porosity curve is required".to_string()))?;
        let parameters = resolve_gassmann_parameters(parameters)?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("VP_GASS").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("VP_GASS").to_string(),
            unit: Some("m/s".to_string()),
            description: Some(format!(
                "Gassmann substituted Vp from {}, {}, {}, and {}",
                vp.curve_name, vs.curve_name, density.curve_name, porosity.curve_name
            )),
            semantic_type: CurveSemanticType::PVelocity,
            semantic_parameters: empty_semantic_parameters(),
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .zip(&density.values)
                .zip(&porosity.values)
                .map(|(((vp_value, vs_value), density_value), porosity_value)| {
                    match (vp_value, vs_value, density_value, porosity_value) {
                        (
                            Some(vp_value),
                            Some(vs_value),
                            Some(density_value),
                            Some(porosity_value),
                        ) => gassmann_substitution_sample(
                            *vp_value,
                            *vs_value,
                            *density_value,
                            density.unit.as_deref(),
                            *porosity_value,
                            porosity.unit.as_deref(),
                            parameters,
                        )
                        .map(|result| result.substituted_vp_m_per_s),
                        _ => None,
                    }
                })
                .collect(),
        })
    }
}

impl LogComputeFunction for GassmannSubstitutedVsFunction {
    fn metadata(&self) -> ComputeFunctionMetadata {
        ComputeFunctionMetadata {
            id: "rock_physics:gassmann_substituted_vs".to_string(),
            provider: "rock_physics".to_string(),
            name: "Gassmann Substituted Vs".to_string(),
            category: "Rock Physics".to_string(),
            description: "Compute substituted S-wave velocity from Gassmann fluid substitution."
                .to_string(),
            default_output_mnemonic: "VS_GASS".to_string(),
            output_curve_type: CurveSemanticType::SVelocity,
            tags: vec!["rock-physics".to_string(), "gassmann".to_string()],
        }
    }

    fn input_specs(&self) -> Vec<ComputeInputSpec> {
        vec![
            ComputeInputSpec::SingleCurve {
                parameter_name: "vp_curve".to_string(),
                allowed_types: vec![CurveSemanticType::PVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "vs_curve".to_string(),
                allowed_types: vec![CurveSemanticType::SVelocity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "density_curve".to_string(),
                allowed_types: vec![CurveSemanticType::BulkDensity],
            },
            ComputeInputSpec::SingleCurve {
                parameter_name: "porosity_curve".to_string(),
                allowed_types: vec![
                    CurveSemanticType::EffectivePorosity,
                    CurveSemanticType::NeutronPorosity,
                ],
            },
        ]
    }

    fn parameters(&self) -> Vec<ComputeParameterDefinition> {
        gassmann_parameters()
    }

    fn execute(
        &self,
        inputs: &BTreeMap<String, LogCurveData>,
        parameters: &BTreeMap<String, ComputeParameterValue>,
        output_mnemonic: Option<&str>,
    ) -> Result<ComputedCurve> {
        let vp = inputs
            .get("vp_curve")
            .ok_or_else(|| LasError::Validation("vp curve is required".to_string()))?;
        let vs = inputs
            .get("vs_curve")
            .ok_or_else(|| LasError::Validation("vs curve is required".to_string()))?;
        let density = inputs
            .get("density_curve")
            .ok_or_else(|| LasError::Validation("density curve is required".to_string()))?;
        let porosity = inputs
            .get("porosity_curve")
            .ok_or_else(|| LasError::Validation("porosity curve is required".to_string()))?;
        let parameters = resolve_gassmann_parameters(parameters)?;
        Ok(ComputedCurve {
            curve_name: output_mnemonic.unwrap_or("VS_GASS").to_string(),
            original_mnemonic: output_mnemonic.unwrap_or("VS_GASS").to_string(),
            unit: Some("m/s".to_string()),
            description: Some(format!(
                "Gassmann substituted Vs from {}, {}, {}, and {}",
                vp.curve_name, vs.curve_name, density.curve_name, porosity.curve_name
            )),
            semantic_type: CurveSemanticType::SVelocity,
            semantic_parameters: empty_semantic_parameters(),
            values: vp
                .values
                .iter()
                .zip(&vs.values)
                .zip(&density.values)
                .zip(&porosity.values)
                .map(|(((vp_value, vs_value), density_value), porosity_value)| {
                    match (vp_value, vs_value, density_value, porosity_value) {
                        (
                            Some(vp_value),
                            Some(vs_value),
                            Some(density_value),
                            Some(porosity_value),
                        ) => gassmann_substitution_sample(
                            *vp_value,
                            *vs_value,
                            *density_value,
                            density.unit.as_deref(),
                            *porosity_value,
                            porosity.unit.as_deref(),
                            parameters,
                        )
                        .map(|result| result.substituted_vs_m_per_s),
                        _ => None,
                    }
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
            semantic_parameters: empty_semantic_parameters(),
        }
    }

    fn curve(name: &str, semantic: CurveSemanticType, values: &[Option<f64>]) -> LogCurveData {
        curve_with_unit(name, semantic, "unit", values)
    }

    fn curve_with_unit(
        name: &str,
        semantic: CurveSemanticType,
        unit: &str,
        values: &[Option<f64>],
    ) -> LogCurveData {
        LogCurveData {
            curve_name: name.to_string(),
            original_mnemonic: name.to_string(),
            unit: Some(unit.to_string()),
            semantic_type: semantic,
            depths: vec![100.0, 100.5, 101.0, 101.5],
            values: values.to_vec(),
        }
    }

    fn assert_values_close(actual: &[Option<f64>], expected: &[Option<f64>], tolerance: f64) {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            match (actual, expected) {
                (Some(actual), Some(expected)) => {
                    assert!(
                        (actual - expected).abs() <= tolerance,
                        "row {index}: expected {expected}, found {actual}"
                    );
                }
                (None, None) => {}
                _ => panic!("row {index}: expected {expected:?}, found {actual:?}"),
            }
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
    fn elastic_rock_physics_functions_match_reference_values() {
        let registry = ComputeRegistry::new();
        let vp = curve_with_unit(
            "VP",
            CurveSemanticType::PVelocity,
            "m/s",
            &[Some(2200.0), Some(2500.0), Some(2800.0), Some(3000.0)],
        );
        let vs = curve_with_unit(
            "VS",
            CurveSemanticType::SVelocity,
            "m/s",
            &[Some(1200.0), Some(1400.0), Some(1600.0), Some(1700.0)],
        );
        let rho = curve_with_unit(
            "RHOB",
            CurveSemanticType::BulkDensity,
            "g/cc",
            &[Some(2.15), Some(2.22), Some(2.28), Some(2.35)],
        );

        let mut vp_vs_bindings = BTreeMap::new();
        vp_vs_bindings.insert("vp_curve".to_string(), "VP".to_string());
        vp_vs_bindings.insert("vs_curve".to_string(), "VS".to_string());
        let (_, vp_vs_output) = registry
            .run_log_compute(
                "rock_physics:vp_vs_ratio",
                &[vp.clone(), vs.clone()],
                &vp_vs_bindings,
                &BTreeMap::new(),
                None,
            )
            .unwrap();
        assert_values_close(
            &vp_vs_output.values,
            &[
                Some(1.8333333333),
                Some(1.7857142857),
                Some(1.75),
                Some(1.7647058824),
            ],
            1.0e-9,
        );

        let mut shear_impedance_bindings = BTreeMap::new();
        shear_impedance_bindings.insert("vs_curve".to_string(), "VS".to_string());
        shear_impedance_bindings.insert("density_curve".to_string(), "RHOB".to_string());
        let (_, shear_impedance_output) = registry
            .run_log_compute(
                "rock_physics:shear_impedance",
                &[vs.clone(), rho.clone()],
                &shear_impedance_bindings,
                &BTreeMap::new(),
                None,
            )
            .unwrap();
        assert_values_close(
            &shear_impedance_output.values,
            &[Some(2580.0), Some(3108.0), Some(3648.0), Some(3995.0)],
            1.0e-9,
        );

        let mut lambda_bindings = BTreeMap::new();
        lambda_bindings.insert("vp_curve".to_string(), "VP".to_string());
        lambda_bindings.insert("vs_curve".to_string(), "VS".to_string());
        lambda_bindings.insert("density_curve".to_string(), "RHOB".to_string());
        let (_, lambda_output) = registry
            .run_log_compute(
                "rock_physics:lambda_rho",
                &[vp.clone(), vs.clone(), rho.clone()],
                &lambda_bindings,
                &BTreeMap::new(),
                None,
            )
            .unwrap();
        assert_values_close(
            &lambda_output.values,
            &[Some(4.214), Some(5.1726), Some(6.2016), Some(7.567)],
            1.0e-9,
        );

        let mut mu_bindings = BTreeMap::new();
        mu_bindings.insert("vs_curve".to_string(), "VS".to_string());
        mu_bindings.insert("density_curve".to_string(), "RHOB".to_string());
        let (_, mu_output) = registry
            .run_log_compute(
                "rock_physics:mu_rho",
                &[vs.clone(), rho.clone()],
                &mu_bindings,
                &BTreeMap::new(),
                None,
            )
            .unwrap();
        assert_values_close(
            &mu_output.values,
            &[Some(3.096), Some(4.3512), Some(5.8368), Some(6.7915)],
            1.0e-9,
        );

        let impedance_bindings = BTreeMap::from([
            ("vp_curve".to_string(), "VP".to_string()),
            ("vs_curve".to_string(), "VS".to_string()),
            ("density_curve".to_string(), "RHOB".to_string()),
        ]);

        let elastic_parameters = BTreeMap::from([(
            "angle_deg".to_string(),
            ComputeParameterValue::Number(30.0),
        )]);
        let (_, elastic_output) = registry
            .run_log_compute(
                "rock_physics:elastic_impedance",
                &[vp.clone(), vs.clone(), rho.clone()],
                &impedance_bindings,
                &elastic_parameters,
                None,
            )
            .unwrap();
        assert_eq!(elastic_output.semantic_type, CurveSemanticType::ElasticImpedance);
        assert_eq!(
            elastic_output.semantic_parameters.get("incident_angle_deg"),
            Some(&30.0)
        );
        assert!(elastic_output
            .semantic_parameters
            .contains_key("normalization_reference_vp_m_per_s"));
        assert_values_close(
            &elastic_output.values,
            &[
                Some(5151.157993034937),
                Some(5666.679780103422),
                Some(6171.5013359980485),
                Some(6649.22504260223),
            ],
            1.0e-9,
        );

        let extended_parameters = BTreeMap::from([(
            "chi_deg".to_string(),
            ComputeParameterValue::Number(20.0),
        )]);
        let (_, extended_output) = registry
            .run_log_compute(
                "rock_physics:extended_elastic_impedance",
                &[vp, vs, rho],
                &impedance_bindings,
                &extended_parameters,
                None,
            )
            .unwrap();
        assert_eq!(
            extended_output.semantic_type,
            CurveSemanticType::ExtendedElasticImpedance
        );
        assert_eq!(
            extended_output.semantic_parameters.get("chi_angle_deg"),
            Some(&20.0)
        );
        assert!(extended_output
            .semantic_parameters
            .contains_key("velocity_ratio_k"));
        assert_values_close(
            &extended_output.values,
            &[
                Some(5496.695845152869),
                Some(5763.67774803316),
                Some(6022.080803912498),
                Some(6341.140694559189),
            ],
            1.0e-9,
        );
    }

    #[test]
    fn gassmann_substitution_functions_match_briges_reference_case() {
        let registry = ComputeRegistry::new();
        let vp = curve_with_unit(
            "VP",
            CurveSemanticType::PVelocity,
            "m/s",
            &[Some(2600.0), Some(2600.0), Some(2600.0), None],
        );
        let vs = curve_with_unit(
            "VS",
            CurveSemanticType::SVelocity,
            "m/s",
            &[Some(1450.0), Some(1450.0), Some(1450.0), None],
        );
        let rho = curve_with_unit(
            "RHOB",
            CurveSemanticType::BulkDensity,
            "kg/m3",
            &[Some(2250.0), Some(2250.0), Some(2250.0), None],
        );
        let phi = curve_with_unit(
            "PHIE",
            CurveSemanticType::EffectivePorosity,
            "%",
            &[Some(24.0), Some(24.0), Some(24.0), None],
        );
        let parameters = BTreeMap::from([
            (
                "matrix_bulk_modulus_gpa".to_string(),
                ComputeParameterValue::Number(37.0),
            ),
            (
                "initial_fluid_bulk_modulus_gpa".to_string(),
                ComputeParameterValue::Number(2.3),
            ),
            (
                "substituted_fluid_bulk_modulus_gpa".to_string(),
                ComputeParameterValue::Number(0.05),
            ),
            (
                "initial_fluid_density_gcc".to_string(),
                ComputeParameterValue::Number(1.0),
            ),
            (
                "substituted_fluid_density_gcc".to_string(),
                ComputeParameterValue::Number(0.2),
            ),
        ]);

        let density_bindings = BTreeMap::from([
            ("density_curve".to_string(), "RHOB".to_string()),
            ("porosity_curve".to_string(), "PHIE".to_string()),
        ]);
        let (_, density_output) = registry
            .run_log_compute(
                "rock_physics:gassmann_substituted_density",
                &[rho.clone(), phi.clone()],
                &density_bindings,
                &parameters,
                None,
            )
            .unwrap();
        assert_values_close(
            &density_output.values,
            &[Some(2.058), Some(2.058), Some(2.058), None],
            1.0e-9,
        );

        let substitution_bindings = BTreeMap::from([
            ("vp_curve".to_string(), "VP".to_string()),
            ("vs_curve".to_string(), "VS".to_string()),
            ("density_curve".to_string(), "RHOB".to_string()),
            ("porosity_curve".to_string(), "PHIE".to_string()),
        ]);
        let curves = [vp, vs, rho, phi];
        let (_, vp_output) = registry
            .run_log_compute(
                "rock_physics:gassmann_substituted_vp",
                &curves,
                &substitution_bindings,
                &parameters,
                None,
            )
            .unwrap();
        assert_values_close(
            &vp_output.values,
            &[
                Some(1964.8205678316335),
                Some(1964.8205678316335),
                Some(1964.8205678316335),
                None,
            ],
            1.0e-9,
        );

        let (_, vs_output) = registry
            .run_log_compute(
                "rock_physics:gassmann_substituted_vs",
                &curves,
                &substitution_bindings,
                &parameters,
                None,
            )
            .unwrap();
        assert_values_close(
            &vs_output.values,
            &[
                Some(1516.130470473614),
                Some(1516.130470473614),
                Some(1516.130470473614),
                None,
            ],
            1.0e-9,
        );
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
