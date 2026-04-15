use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OperationCatalog {
    pub schema_version: u32,
    pub catalog_name: String,
    pub product: String,
    pub operations: Vec<OperationDescriptor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OperationDescriptor {
    pub id: String,
    pub summary: String,
    pub owner: String,
    pub domain: String,
    pub stability: String,
    #[serde(default)]
    pub surfaces: Vec<String>,
    pub bindings: OperationBindings,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OperationBindings {
    #[serde(default)]
    pub rust_function: Option<String>,
    #[serde(default)]
    pub rust_cli: Option<String>,
}

static OPERATION_CATALOG: OnceLock<OperationCatalog> = OnceLock::new();

pub fn operation_catalog() -> &'static OperationCatalog {
    OPERATION_CATALOG.get_or_init(|| {
        serde_json::from_str(operation_catalog_json())
            .expect("ophiolite operation catalog to parse")
    })
}

pub fn operation_catalog_json() -> &'static str {
    include_str!("../operations/catalog.json")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::operation_catalog;
    use crate::supported_cli_commands;

    fn surface_commands(
        catalog: &super::OperationCatalog,
        surface: &str,
        binding_selector: impl Fn(&super::OperationBindings) -> Option<&str>,
    ) -> BTreeSet<String> {
        catalog
            .operations
            .iter()
            .filter(|operation| {
                operation
                    .surfaces
                    .iter()
                    .any(|declared| declared == surface)
            })
            .filter_map(|operation| binding_selector(&operation.bindings).map(ToOwned::to_owned))
            .collect()
    }

    #[test]
    fn embedded_catalog_parses() {
        let catalog = operation_catalog();
        assert_eq!(catalog.schema_version, 1);
        assert_eq!(catalog.catalog_name, "ophiolite-operations");
        assert_eq!(catalog.product, "Ophiolite");
        assert!(!catalog.operations.is_empty());
    }

    #[test]
    fn operation_ids_are_unique() {
        let mut ids = HashSet::new();
        for operation in &operation_catalog().operations {
            assert!(
                ids.insert(operation.id.as_str()),
                "duplicate operation id {}",
                operation.id
            );
        }
    }

    #[test]
    fn rust_cli_surface_matches_actual_commands() {
        let declared = surface_commands(operation_catalog(), "rust-cli", |bindings| {
            bindings.rust_cli.as_deref()
        });
        let actual = supported_cli_commands()
            .iter()
            .map(|command| (*command).to_string())
            .collect();
        assert_eq!(declared, actual);
    }

    #[test]
    fn rust_api_surface_declares_function_binding() {
        for operation in &operation_catalog().operations {
            if operation
                .surfaces
                .iter()
                .any(|surface| surface == "rust-api")
            {
                assert!(
                    operation.bindings.rust_function.is_some(),
                    "rust-api surface missing rust_function for {}",
                    operation.id
                );
            }
        }
    }

    #[test]
    fn operation_descriptors_have_basic_metadata() {
        for operation in &operation_catalog().operations {
            assert!(
                !operation.summary.trim().is_empty(),
                "missing summary for {}",
                operation.id
            );
            assert!(
                !operation.owner.trim().is_empty(),
                "missing owner for {}",
                operation.id
            );
            assert!(
                !operation.domain.trim().is_empty(),
                "missing domain for {}",
                operation.id
            );
            assert!(
                !operation.stability.trim().is_empty(),
                "missing stability for {}",
                operation.id
            );
        }
    }
}
