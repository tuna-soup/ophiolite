use crate::CurveStorageKind;
use crate::asset::{CurveItem, LasFile, LasValue};
use serde::{Deserialize, Serialize};

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
}

impl LasFile {
    pub fn data(&self) -> CurveTable {
        CurveTable::from_curves(self.curves.as_slice())
    }
}

pub fn detect_storage_kind(values: &[LasValue]) -> CurveStorageKind {
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
