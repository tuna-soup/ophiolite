pub mod domain;
pub mod models;
pub mod operations;
pub mod operator_catalog;
pub mod processing;
pub mod resolve_dtos;
pub mod views;

pub use domain::*;
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
