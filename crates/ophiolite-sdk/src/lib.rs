//! Public-core facade for Ophiolite.
//!
//! This crate intentionally re-exports only the publishable core layers:
//! operator vocabulary, canonical shared seismic contracts, runtime/planner
//! layers, execution orchestration, and shared consumer-facing contract crates.
//! TraceBoost compatibility adapters stay outside this facade.

pub use ophiolite_capabilities as capabilities;
pub use ophiolite_operators as operators;
pub use ophiolite_seismic as seismic;
pub use ophiolite_seismic_execution as execution;
pub use ophiolite_seismic_runtime as runtime_core;
pub use seis_contracts_core as contracts_core;
pub use seis_contracts_operations as contracts_operations;
pub use seis_contracts_views as contracts_views;
pub use seis_runtime as runtime;
