use crate::semantics::CurveSemanticType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComputeParameterValue {
    Number(f64),
    String(String),
    Boolean(bool),
}

impl ComputeParameterValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeFunctionMetadata {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub default_output_mnemonic: String,
    pub output_curve_type: CurveSemanticType,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputeInputBinding {
    pub parameter_name: String,
    pub curve_name: String,
    pub semantic_type: CurveSemanticType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputeExecutionManifest {
    pub function_id: String,
    pub provider: String,
    pub function_name: String,
    pub function_version: String,
    pub deterministic: bool,
    pub source_asset_id: String,
    pub source_logical_asset_id: String,
    pub inputs: Vec<ComputeInputBinding>,
    pub parameters: BTreeMap<String, ComputeParameterValue>,
    pub output_curve_name: String,
    pub output_curve_type: CurveSemanticType,
    pub executed_at_unix_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct LogCurveData {
    pub curve_name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub semantic_type: CurveSemanticType,
    pub depths: Vec<f64>,
    pub values: Vec<Option<f64>>,
}

#[derive(Debug, Clone)]
pub struct ComputedCurve {
    pub curve_name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub semantic_type: CurveSemanticType,
    pub values: Vec<Option<f64>>,
}
