pub mod domain;
pub mod models;
pub mod operations;
pub mod processing;
pub mod views;

pub use domain::*;
pub use models::*;
pub use operations::*;
pub use processing::*;
pub use views::*;

pub(super) fn default_pipeline_schema_version() -> u32 {
    2
}

pub(super) fn default_pipeline_revision() -> u32 {
    1
}
