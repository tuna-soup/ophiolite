use crate::functions::ComputeFunctionMetadata;
use crate::registry::{
    ComputeAvailability, ComputeCatalogEntry, ComputeInputSpec, ComputeParameterDefinition,
    availability_for_binding_candidates, binding_candidates_for_input_specs,
};
use crate::semantics::{AssetSemanticFamily, CurveSemanticType};
use ophiolite_core::{LasError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub const OPERATOR_PACKAGE_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const BUILTIN_OPERATOR_PACKAGE_NAME: &str = "ophiolite-builtins";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorRuntimeKind {
    Rust,
    Python,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorStability {
    Internal,
    Preview,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorOutputLifecycle {
    DerivedAsset,
    AnalysisOnly,
    ViewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorPackageCompatibility {
    pub ophiolite_api: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorManifest {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub asset_family: AssetSemanticFamily,
    pub category: String,
    pub description: String,
    pub default_output_mnemonic: String,
    pub output_curve_type: CurveSemanticType,
    #[serde(default)]
    pub input_specs: Vec<ComputeInputSpec>,
    #[serde(default)]
    pub parameters: Vec<ComputeParameterDefinition>,
    pub output_lifecycle: OperatorOutputLifecycle,
    pub deterministic: bool,
    pub stability: OperatorStability,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorPackageManifest {
    pub schema_version: u32,
    pub package_name: String,
    pub package_version: String,
    pub provider: String,
    pub runtime: OperatorRuntimeKind,
    pub compatibility: OperatorPackageCompatibility,
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub operators: Vec<OperatorManifest>,
}

impl OperatorPackageManifest {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != OPERATOR_PACKAGE_MANIFEST_SCHEMA_VERSION {
            return Err(LasError::Validation(format!(
                "unsupported operator package manifest schema version '{}'",
                self.schema_version
            )));
        }

        ensure_non_empty(&self.package_name, "package_name")?;
        ensure_non_empty(&self.package_version, "package_version")?;
        ensure_non_empty(&self.provider, "provider")?;
        ensure_non_empty(
            &self.compatibility.ophiolite_api,
            "compatibility.ophiolite_api",
        )?;

        if matches!(self.runtime, OperatorRuntimeKind::Python) {
            let Some(entrypoint) = &self.entrypoint else {
                return Err(LasError::Validation(
                    "python operator packages require an entrypoint".to_string(),
                ));
            };
            ensure_non_empty(entrypoint, "entrypoint")?;
        }

        if self.operators.is_empty() {
            return Err(LasError::Validation(
                "operator package manifests must declare at least one operator".to_string(),
            ));
        }

        let mut operator_ids = BTreeSet::new();
        for operator in &self.operators {
            ensure_non_empty(&operator.id, "operator.id")?;
            ensure_non_empty(&operator.provider, "operator.provider")?;
            ensure_non_empty(&operator.name, "operator.name")?;
            ensure_non_empty(&operator.category, "operator.category")?;
            ensure_non_empty(&operator.description, "operator.description")?;
            ensure_non_empty(
                &operator.default_output_mnemonic,
                "operator.default_output_mnemonic",
            )?;
            if !operator_ids.insert(operator.id.clone()) {
                return Err(LasError::Validation(format!(
                    "duplicate operator id '{}'",
                    operator.id
                )));
            }
        }

        Ok(())
    }
}

pub fn parse_operator_package_manifest(text: &str) -> Result<OperatorPackageManifest> {
    let text = text.trim_start_matches('\u{feff}');
    let manifest = serde_json::from_str::<OperatorPackageManifest>(text)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn load_operator_package_manifest(path: impl AsRef<Path>) -> Result<OperatorPackageManifest> {
    parse_operator_package_manifest(&fs::read_to_string(path)?)
}

pub fn unavailable_catalog_entry_for_operator(
    package: &OperatorPackageManifest,
    operator: &OperatorManifest,
) -> ComputeCatalogEntry {
    ComputeCatalogEntry {
        metadata: ComputeFunctionMetadata {
            id: operator.id.clone(),
            provider: operator.provider.clone(),
            name: operator.name.clone(),
            category: operator.category.clone(),
            description: operator.description.clone(),
            default_output_mnemonic: operator.default_output_mnemonic.clone(),
            output_curve_type: operator.output_curve_type.clone(),
            tags: operator.tags.clone(),
        },
        input_specs: operator.input_specs.clone(),
        parameters: operator.parameters.clone(),
        binding_candidates: Vec::new(),
        availability: ComputeAvailability::Unavailable {
            reasons: vec![format!(
                "operator package '{}@{}' is not available in this context",
                package.package_name, package.package_version
            )],
        },
    }
}

pub fn available_catalog_entry_for_operator(
    operator: &OperatorManifest,
    curves: Option<(&[crate::CurveSemanticDescriptor], &[String])>,
) -> ComputeCatalogEntry {
    let binding_candidates = match (operator.asset_family.clone(), curves) {
        (AssetSemanticFamily::Log, Some((curves, numeric_curve_names))) => {
            binding_candidates_for_input_specs(&operator.input_specs, curves, numeric_curve_names)
        }
        _ => Vec::new(),
    };

    ComputeCatalogEntry {
        metadata: ComputeFunctionMetadata {
            id: operator.id.clone(),
            provider: operator.provider.clone(),
            name: operator.name.clone(),
            category: operator.category.clone(),
            description: operator.description.clone(),
            default_output_mnemonic: operator.default_output_mnemonic.clone(),
            output_curve_type: operator.output_curve_type.clone(),
            tags: operator.tags.clone(),
        },
        input_specs: operator.input_specs.clone(),
        parameters: operator.parameters.clone(),
        binding_candidates: binding_candidates.clone(),
        availability: if operator.asset_family == AssetSemanticFamily::Log {
            availability_for_binding_candidates(&binding_candidates)
        } else {
            ComputeAvailability::Available
        },
    }
}

fn ensure_non_empty(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(LasError::Validation(format!(
            "field '{field_name}' must not be empty"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_python_entrypoint_and_unique_operator_ids() {
        let manifest = OperatorPackageManifest {
            schema_version: OPERATOR_PACKAGE_MANIFEST_SCHEMA_VERSION,
            package_name: "acme-tools".to_string(),
            package_version: "0.1.0".to_string(),
            provider: "acme".to_string(),
            runtime: OperatorRuntimeKind::Python,
            compatibility: OperatorPackageCompatibility {
                ophiolite_api: "0.1.0".to_string(),
            },
            entrypoint: None,
            operators: vec![OperatorManifest {
                id: "acme:demo".to_string(),
                provider: "acme".to_string(),
                name: "Demo".to_string(),
                asset_family: AssetSemanticFamily::Log,
                category: "Demo".to_string(),
                description: "Demo".to_string(),
                default_output_mnemonic: "DEMO".to_string(),
                output_curve_type: CurveSemanticType::Computed,
                input_specs: Vec::new(),
                parameters: Vec::new(),
                output_lifecycle: OperatorOutputLifecycle::DerivedAsset,
                deterministic: true,
                stability: OperatorStability::Preview,
                tags: Vec::new(),
            }],
        };

        assert!(manifest.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_operator_ids() {
        let text = r#"{
          "schema_version": 1,
          "package_name": "acme-tools",
          "package_version": "0.1.0",
          "provider": "acme",
          "runtime": "python",
          "compatibility": { "ophiolite_api": "0.1.0" },
          "entrypoint": "acme_ops",
          "operators": [
            {
              "id": "acme:demo",
              "provider": "acme",
              "name": "Demo One",
              "asset_family": "Log",
              "category": "Demo",
              "description": "Demo",
              "default_output_mnemonic": "DEMO",
              "output_curve_type": "Computed",
              "output_lifecycle": "derived_asset",
              "deterministic": true,
              "stability": "preview",
              "tags": []
            },
            {
              "id": "acme:demo",
              "provider": "acme",
              "name": "Demo Two",
              "asset_family": "Log",
              "category": "Demo",
              "description": "Demo",
              "default_output_mnemonic": "DEMO2",
              "output_curve_type": "Computed",
              "output_lifecycle": "derived_asset",
              "deterministic": true,
              "stability": "preview",
              "tags": []
            }
          ]
        }"#;

        assert!(parse_operator_package_manifest(text).is_err());
    }
}
