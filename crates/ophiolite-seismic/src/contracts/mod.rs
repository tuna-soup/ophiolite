pub mod domain;
pub mod inspectable_processing_plan;
pub mod models;
pub mod operations;
pub mod operator_catalog;
pub mod processing;
pub mod resolve_dtos;
pub mod views;

pub use domain::*;
pub use inspectable_processing_plan::*;
pub use models::*;
pub use operations::*;
pub use operator_catalog::*;
pub use processing::*;
pub use resolve_dtos::*;
pub use views::*;

pub(super) fn default_pipeline_schema_version() -> u32 {
    2
}

pub(super) fn default_pipeline_revision() -> u32 {
    1
}

pub fn default_inspectable_plan_schema_version() -> u32 {
    2
}

pub fn default_execution_plan_schema_version() -> u32 {
    1
}

pub fn default_processing_lineage_schema_version() -> u32 {
    2
}

pub fn default_semantic_identity_schema_version() -> u32 {
    1
}
