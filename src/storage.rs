use crate::asset::{CurveItem, HeaderItem, IndexDescriptor, IngestIssue, LasFile, LasFileSummary};
use crate::table::{CurveColumnDescriptor, CurveStorageKind, CurveTable};
use crate::{LasError, LasValue, Provenance, Result};
use arrow_array::Array;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::debug;

const PACKAGE_VERSION: u32 = 1;
const METADATA_FILENAME: &str = "metadata.json";
const CURVES_FILENAME: &str = "curves.parquet";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCurveMetadata {
    mnemonic: String,
    original_mnemonic: String,
    unit: String,
    value: LasValue,
    description: String,
    storage_kind: CurveStorageKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageMetadata {
    package_version: u32,
    summary: LasFileSummary,
    provenance: Provenance,
    encoding: Option<String>,
    index: IndexDescriptor,
    version: crate::SectionItems<HeaderItem>,
    well: crate::SectionItems<HeaderItem>,
    params: crate::SectionItems<HeaderItem>,
    curve_mnemonic_case: crate::MnemonicCase,
    curves: Vec<StoredCurveMetadata>,
    other: String,
    extra_sections: std::collections::BTreeMap<String, String>,
    issues: Vec<IngestIssue>,
    index_unit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredLasFile {
    root: PathBuf,
    file: LasFile,
    table: CurveTable,
}

impl StoredLasFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        open_package(path)
    }

    pub fn file(&self) -> &LasFile {
        &self.file
    }

    pub fn summary(&self) -> &LasFileSummary {
        &self.file.summary
    }

    pub fn read_curve(&self, mnemonic: &str) -> Option<&[LasValue]> {
        self.file.curve_data(mnemonic)
    }

    pub fn data(&self) -> &CurveTable {
        &self.table
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

pub fn open_package(path: impl AsRef<Path>) -> Result<StoredLasFile> {
    let root = path.as_ref().to_path_buf();
    debug!(package = %root.display(), "opening LAS package");

    let metadata_text = fs::read_to_string(metadata_path(&root))?;
    let metadata: PackageMetadata = serde_json::from_str(&metadata_text)?;
    if metadata.package_version != PACKAGE_VERSION {
        return Err(LasError::Storage(format!(
            "unsupported package version {}",
            metadata.package_version
        )));
    }

    let batch = read_parquet_batch(curves_path(&root), metadata.summary.row_count)?;
    let descriptors = metadata
        .curves
        .iter()
        .map(|curve| CurveColumnDescriptor {
            name: curve.mnemonic.clone(),
            storage_kind: curve.storage_kind,
        })
        .collect::<Vec<_>>();
    let table = CurveTable::from_record_batch(&batch, &descriptors)?;
    let curves = materialize_curves(&metadata.curves, &table)?;

    Ok(StoredLasFile {
        root,
        file: LasFile {
            summary: metadata.summary,
            provenance: metadata.provenance,
            encoding: metadata.encoding,
            index: metadata.index,
            version: metadata.version,
            well: metadata.well,
            params: metadata.params,
            curves: crate::SectionItems::from_items(curves, metadata.curve_mnemonic_case),
            other: metadata.other,
            extra_sections: metadata.extra_sections,
            issues: metadata.issues,
            index_unit: metadata.index_unit,
        },
        table,
    })
}

pub fn write_package(file: &LasFile, output_dir: impl AsRef<Path>) -> Result<StoredLasFile> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() {
        return Err(LasError::Storage(format!(
            "output directory '{}' already exists",
            output_dir.display()
        )));
    }

    debug!(package = %output_dir.display(), "writing LAS package");
    fs::create_dir_all(output_dir)?;

    let table = file.data();
    let metadata = PackageMetadata {
        package_version: PACKAGE_VERSION,
        summary: file.summary.clone(),
        provenance: file.provenance.clone(),
        encoding: file.encoding.clone(),
        index: file.index.clone(),
        version: file.version.clone(),
        well: file.well.clone(),
        params: file.params.clone(),
        curve_mnemonic_case: file.curves.mnemonic_case,
        curves: file
            .curves
            .iter()
            .zip(table.descriptors())
            .map(|(curve, descriptor)| StoredCurveMetadata {
                mnemonic: curve.mnemonic.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: curve.unit.clone(),
                value: curve.value.clone(),
                description: curve.description.clone(),
                storage_kind: descriptor.storage_kind,
            })
            .collect(),
        other: file.other.clone(),
        extra_sections: file.extra_sections.clone(),
        issues: file.issues.clone(),
        index_unit: file.index_unit.clone(),
    };

    fs::write(
        metadata_path(output_dir),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    write_parquet_batch(curves_path(output_dir), &table)?;

    open_package(output_dir)
}

pub fn write_bundle(file: &LasFile, output_dir: impl AsRef<Path>) -> Result<StoredLasFile> {
    write_package(file, output_dir)
}

fn metadata_path(root: &Path) -> PathBuf {
    root.join(METADATA_FILENAME)
}

fn curves_path(root: &Path) -> PathBuf {
    root.join(CURVES_FILENAME)
}

fn write_parquet_batch(path: PathBuf, table: &CurveTable) -> Result<()> {
    let batch = table.to_record_batch()?;
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

fn read_parquet_batch(path: PathBuf, row_count: usize) -> Result<arrow_array::RecordBatch> {
    let file = File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?
        .with_batch_size(row_count.max(1))
        .build()?;
    let mut batches = reader.collect::<std::result::Result<Vec<_>, _>>()?;
    match batches.len() {
        0 => {
            return Err(LasError::Storage(String::from(
                "package parquet data did not contain any record batches",
            )));
        }
        1 => Ok(batches.remove(0)),
        _ => merge_batches(batches),
    }
}

fn merge_batches(batches: Vec<arrow_array::RecordBatch>) -> Result<arrow_array::RecordBatch> {
    let first = batches
        .first()
        .ok_or_else(|| LasError::Storage(String::from("no batches to merge")))?;
    let schema = first.schema();
    let mut merged = Vec::with_capacity(schema.fields().len());

    for column_index in 0..schema.fields().len() {
        match schema.field(column_index).data_type() {
            arrow_schema::DataType::Float64 => {
                let mut values = Vec::new();
                for batch in &batches {
                    let array = batch
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<arrow_array::Float64Array>()
                        .ok_or_else(|| {
                            LasError::Storage(String::from(
                                "expected Float64Array while merging parquet batches",
                            ))
                        })?;
                    values.extend((0..array.len()).map(|row| {
                        if array.is_null(row) {
                            None
                        } else {
                            Some(array.value(row))
                        }
                    }));
                }
                merged.push(std::sync::Arc::new(arrow_array::Float64Array::from(values))
                    as arrow_array::ArrayRef);
            }
            arrow_schema::DataType::Utf8 => {
                let mut values = Vec::new();
                for batch in &batches {
                    let array = batch
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<arrow_array::StringArray>()
                        .ok_or_else(|| {
                            LasError::Storage(String::from(
                                "expected StringArray while merging parquet batches",
                            ))
                        })?;
                    values.extend((0..array.len()).map(|row| {
                        if array.is_null(row) {
                            None
                        } else {
                            Some(array.value(row).to_string())
                        }
                    }));
                }
                merged.push(
                    std::sync::Arc::new(arrow_array::StringArray::from_iter(values))
                        as arrow_array::ArrayRef,
                );
            }
            other => {
                return Err(LasError::Storage(format!(
                    "unsupported parquet column type during merge: {other:?}"
                )));
            }
        }
    }

    Ok(arrow_array::RecordBatch::try_new(schema, merged)?)
}

fn materialize_curves(
    curves: &[StoredCurveMetadata],
    table: &CurveTable,
) -> Result<Vec<CurveItem>> {
    curves
        .iter()
        .map(|curve| {
            let values = table
                .column(&curve.mnemonic)
                .ok_or_else(|| {
                    LasError::Storage(format!(
                        "column '{}' missing from package table",
                        curve.mnemonic
                    ))
                })?
                .values()
                .to_vec();
            Ok(CurveItem {
                mnemonic: curve.mnemonic.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: curve.unit.clone(),
                value: curve.value.clone(),
                description: curve.description.clone(),
                data: values,
            })
        })
        .collect()
}
