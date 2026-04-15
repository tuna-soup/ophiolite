use crate::functions::{
    ComputeParameterValue, ComputedCurve, DrillingObservationDataRow, LogCurveData,
    PressureObservationDataRow, TopDataRow, TrajectoryDataRow,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalOperatorRequest {
    pub operator_id: String,
    pub package_name: String,
    pub package_version: String,
    pub parameters: BTreeMap<String, ComputeParameterValue>,
    pub payload: ExternalOperatorRequestPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExternalOperatorRequestPayload {
    Log {
        inputs: BTreeMap<String, LogCurveData>,
        output_mnemonic: Option<String>,
    },
    Trajectory {
        rows: Vec<TrajectoryDataRow>,
    },
    TopSet {
        rows: Vec<TopDataRow>,
    },
    PressureObservation {
        rows: Vec<PressureObservationDataRow>,
    },
    DrillingObservation {
        rows: Vec<DrillingObservationDataRow>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalOperatorResponse {
    pub payload: ExternalOperatorResponsePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExternalOperatorResponsePayload {
    Log {
        computed_curve: ComputedCurve,
    },
    Trajectory {
        rows: Vec<TrajectoryDataRow>,
    },
    TopSet {
        rows: Vec<TopDataRow>,
    },
    PressureObservation {
        rows: Vec<PressureObservationDataRow>,
    },
    DrillingObservation {
        rows: Vec<DrillingObservationDataRow>,
    },
}
