use crate::asset::{CurveItem, LasFile, LasValue};
use crate::{LasError, Result};
use arrow_array::{Array, ArrayRef, Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurveStorageKind {
    Numeric,
    Text,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurveColumnDescriptor {
    pub name: String,
    pub storage_kind: CurveStorageKind,
}

#[derive(Debug, Clone)]
pub struct CurveColumn {
    name: String,
    storage_kind: CurveStorageKind,
    values: Vec<LasValue>,
}

impl CurveColumn {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn storage_kind(&self) -> CurveStorageKind {
        self.storage_kind
    }

    pub fn values(&self) -> &[LasValue] {
        &self.values
    }

    pub fn numeric_values(&self) -> Option<Vec<f64>> {
        self.values.iter().map(LasValue::as_f64).collect()
    }

    fn descriptor(&self) -> CurveColumnDescriptor {
        CurveColumnDescriptor {
            name: self.name.clone(),
            storage_kind: self.storage_kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurveTable {
    columns: Vec<CurveColumn>,
    row_count: usize,
}

impl CurveTable {
    pub fn from_curves(curves: &[CurveItem]) -> Self {
        let columns = curves
            .iter()
            .map(|curve| CurveColumn {
                name: curve.mnemonic.clone(),
                storage_kind: detect_storage_kind(&curve.data),
                values: curve.data.clone(),
            })
            .collect::<Vec<_>>();
        let row_count = curves.first().map(|curve| curve.data.len()).unwrap_or(0);
        Self { columns, row_count }
    }

    pub fn descriptors(&self) -> Vec<CurveColumnDescriptor> {
        self.columns.iter().map(CurveColumn::descriptor).collect()
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn column_names(&self) -> Vec<String> {
        self.columns
            .iter()
            .map(|column| column.name.clone())
            .collect()
    }

    pub fn column(&self, name: &str) -> Option<&CurveColumn> {
        self.columns.iter().find(|column| column.name == name)
    }

    pub fn slice_rows(&self, start: usize, end: usize) -> Self {
        let safe_end = end.min(self.row_count);
        let safe_start = start.min(safe_end);
        let columns = self
            .columns
            .iter()
            .map(|column| CurveColumn {
                name: column.name.clone(),
                storage_kind: column.storage_kind,
                values: column.values[safe_start..safe_end].to_vec(),
            })
            .collect::<Vec<_>>();
        Self {
            columns,
            row_count: safe_end.saturating_sub(safe_start),
        }
    }

    pub fn to_record_batch(&self) -> Result<RecordBatch> {
        let schema = Arc::new(Schema::new(
            self.columns
                .iter()
                .map(|column| {
                    let data_type = match column.storage_kind {
                        CurveStorageKind::Numeric => DataType::Float64,
                        CurveStorageKind::Text | CurveStorageKind::Mixed => DataType::Utf8,
                    };
                    Field::new(&column.name, data_type, true)
                })
                .collect::<Vec<_>>(),
        ));

        let arrays = self
            .columns
            .iter()
            .map(|column| match column.storage_kind {
                CurveStorageKind::Numeric => {
                    let values = column
                        .values
                        .iter()
                        .map(|value| match value {
                            LasValue::Number(number) => Some(*number),
                            LasValue::Empty => None,
                            LasValue::Text(text) => text.parse::<f64>().ok(),
                        })
                        .collect::<Vec<_>>();
                    Ok(Arc::new(Float64Array::from(values)) as ArrayRef)
                }
                CurveStorageKind::Text | CurveStorageKind::Mixed => {
                    let values = column.values.iter().map(|value| match value {
                        LasValue::Number(number) => Some(number.to_string()),
                        LasValue::Text(text) => Some(text.clone()),
                        LasValue::Empty => None,
                    });
                    Ok(Arc::new(StringArray::from_iter(values)) as ArrayRef)
                }
            })
            .collect::<Result<Vec<_>>>()?;

        RecordBatch::try_new(schema, arrays)
            .map_err(|err| LasError::Storage(format!("failed to build arrow record batch: {err}")))
    }

    pub fn from_record_batch(
        batch: &RecordBatch,
        descriptors: &[CurveColumnDescriptor],
    ) -> Result<Self> {
        let mut columns = Vec::with_capacity(descriptors.len());
        for descriptor in descriptors {
            let index = batch.schema().index_of(&descriptor.name).map_err(|err| {
                LasError::Storage(format!(
                    "column '{}' missing from parquet data: {err}",
                    descriptor.name
                ))
            })?;
            let array = batch.column(index);
            let values = values_from_array(array.as_ref(), descriptor.storage_kind)?;
            columns.push(CurveColumn {
                name: descriptor.name.clone(),
                storage_kind: descriptor.storage_kind,
                values,
            });
        }

        Ok(Self {
            row_count: batch.num_rows(),
            columns,
        })
    }
}

impl LasFile {
    pub fn data(&self) -> CurveTable {
        CurveTable::from_curves(self.curves.as_slice())
    }
}

fn detect_storage_kind(values: &[LasValue]) -> CurveStorageKind {
    let has_numbers = values
        .iter()
        .any(|value| matches!(value, LasValue::Number(_)));
    let has_text = values
        .iter()
        .any(|value| matches!(value, LasValue::Text(_)));
    match (has_numbers, has_text) {
        (true, true) => CurveStorageKind::Mixed,
        (true, false) => CurveStorageKind::Numeric,
        (false, true) => CurveStorageKind::Text,
        (false, false) => CurveStorageKind::Numeric,
    }
}

fn values_from_array(array: &dyn Array, storage_kind: CurveStorageKind) -> Result<Vec<LasValue>> {
    match storage_kind {
        CurveStorageKind::Numeric => {
            let numbers = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or_else(|| {
                    LasError::Storage(String::from(
                        "expected Float64Array for numeric parquet column",
                    ))
                })?;
            Ok((0..numbers.len())
                .map(|index| {
                    if numbers.is_null(index) {
                        LasValue::Empty
                    } else {
                        LasValue::Number(numbers.value(index))
                    }
                })
                .collect())
        }
        CurveStorageKind::Text | CurveStorageKind::Mixed => {
            let strings = array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| {
                    LasError::Storage(String::from(
                        "expected StringArray for text or mixed parquet column",
                    ))
                })?;
            Ok((0..strings.len())
                .map(|index| {
                    if strings.is_null(index) {
                        LasValue::Empty
                    } else {
                        let value = strings.value(index).to_string();
                        if storage_kind == CurveStorageKind::Mixed {
                            value
                                .parse::<f64>()
                                .map(LasValue::Number)
                                .unwrap_or_else(|_| LasValue::Text(value))
                        } else {
                            LasValue::Text(value)
                        }
                    }
                })
                .collect())
        }
    }
}
