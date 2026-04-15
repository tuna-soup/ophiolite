mod external;
mod functions;
mod operators;
mod registry;
mod semantics;

pub use external::{
    ExternalOperatorRequest, ExternalOperatorRequestPayload, ExternalOperatorResponse,
    ExternalOperatorResponsePayload,
};
pub use functions::{
    ComputeExecutionManifest, ComputeFunctionMetadata, ComputeInputBinding, ComputeParameterValue,
    ComputedCurve, DrillingObservationDataRow, LogCurveData, PressureObservationDataRow,
    TopDataRow, TrajectoryDataRow,
};
pub use operators::{
    BUILTIN_OPERATOR_PACKAGE_NAME, OPERATOR_PACKAGE_MANIFEST_SCHEMA_VERSION, OperatorManifest,
    OperatorOutputLifecycle, OperatorPackageCompatibility, OperatorPackageManifest,
    OperatorRuntimeKind, OperatorStability, load_operator_package_manifest,
    parse_operator_package_manifest, unavailable_catalog_entry_for_operator,
};
pub use registry::{
    ComputeAvailability, ComputeBindingCandidate, ComputeCatalog, ComputeCatalogEntry,
    ComputeInputSpec, ComputeParameterDefinition, ComputeRegistry,
    availability_for_binding_candidates, binding_candidates_for_input_specs,
    catalog_entry_for_operator_manifest, resolve_log_input_bindings, validate_compute_parameters,
};
pub use semantics::{
    AssetSemanticFamily, CurveBindingCandidate, CurveSemanticDescriptor, CurveSemanticSource,
    CurveSemanticType, classify_curve_semantic, default_curve_semantics,
};
