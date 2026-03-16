use arrow_array::Array;
use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use lithos_core::{
    AssetSummaryDto, CurveCatalogEntryDto, CurveColumnMetadata, CurveEditRequest, CurveItem,
    CurveStorageKind, CurveTable, CurveWindowDto, CurveWindowRequest, LasError, LasFile,
    LasFileSummary, LasValue, MetadataDto, MetadataUpdateRequest, PackageMetadata, Result,
    SavePackageResultDto, SectionItems, ValidationReportDto, apply_curve_edit,
    apply_metadata_update, asset_summary_dto, curve_catalog_dto, curve_window_dto, metadata_dto,
    package_metadata_for, validate_edit_state, validation_report_dto,
};
use lithos_table::CurveColumnDescriptor;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

const PACKAGE_VERSION: u32 = 1;
const METADATA_FILENAME: &str = "metadata.json";
const CURVES_FILENAME: &str = "curves.parquet";

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

    pub fn summary_dto(&self) -> AssetSummaryDto {
        asset_summary_dto(&self.file)
    }

    pub fn metadata_dto(&self) -> MetadataDto {
        metadata_dto(&self.file)
    }

    pub fn curve_catalog(&self) -> Vec<CurveCatalogEntryDto> {
        curve_catalog_dto(&self.file)
    }

    pub fn read_window(&self, request: &CurveWindowRequest) -> Result<CurveWindowDto> {
        curve_window_dto(&self.file, request)
    }

    pub fn validation_report(&self) -> ValidationReportDto {
        validation_report_dto(&self.file)
    }

    pub fn apply_metadata_update(&mut self, request: &MetadataUpdateRequest) -> Result<()> {
        apply_metadata_update(&mut self.file, request)?;
        self.table = self.file.data();
        Ok(())
    }

    pub fn apply_curve_edit(&mut self, request: &CurveEditRequest) -> Result<()> {
        apply_curve_edit(&mut self.file, request)?;
        self.table = self.file.data();
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        let reopened = write_package_internal(&self.file, &self.root, true)?;
        *self = reopened;
        Ok(())
    }

    pub fn save_with_result(&mut self) -> Result<SavePackageResultDto> {
        let reopened = write_package_internal(&self.file, &self.root, true)?;
        let result = SavePackageResultDto {
            root: reopened.root.display().to_string(),
            overwritten: true,
            summary: reopened.summary_dto(),
        };
        *self = reopened;
        Ok(result)
    }

    pub fn save_as(&self, output_dir: impl AsRef<Path>) -> Result<StoredLasFile> {
        write_package_internal(&self.file, output_dir.as_ref(), false)
    }

    pub fn save_as_with_result(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<SavePackageResultDto> {
        let reopened = write_package_internal(&self.file, output_dir.as_ref(), false)?;
        Ok(SavePackageResultDto {
            root: reopened.root.display().to_string(),
            overwritten: false,
            summary: reopened.summary_dto(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

pub fn open_package(path: impl AsRef<Path>) -> Result<StoredLasFile> {
    let root = path.as_ref().to_path_buf();
    debug!(package = %root.display(), "opening LAS package");

    let metadata = read_package_metadata(&root)?;

    let batch = read_parquet_batch(curves_path(&root), metadata.summary.row_count)?;
    let descriptors = metadata
        .curve_columns
        .iter()
        .map(|curve| CurveColumnDescriptor {
            name: curve.name.clone(),
            storage_kind: curve.storage_kind,
        })
        .collect::<Vec<_>>();
    let table = table_from_record_batch(&batch, &descriptors)?;
    let curves = materialize_curves(&metadata.curve_columns, &table)?;

    Ok(StoredLasFile {
        root,
        file: LasFile {
            summary: metadata.summary,
            provenance: metadata.provenance,
            encoding: metadata.encoding,
            index: metadata.index,
            version: metadata.raw_sections.version,
            well: metadata.raw_sections.well,
            params: metadata.raw_sections.params,
            curves: SectionItems::from_items(curves, metadata.raw_sections.curve_mnemonic_case),
            other: metadata.raw_sections.other,
            extra_sections: metadata.raw_sections.extra_sections,
            issues: metadata.issues,
            index_unit: metadata.index_unit,
        },
        table,
    })
}

pub fn open_package_summary(path: impl AsRef<Path>) -> Result<AssetSummaryDto> {
    let metadata = read_package_metadata(path.as_ref())?;
    Ok(AssetSummaryDto {
        summary: metadata.summary,
        encoding: metadata.encoding,
        index: metadata.canonical.index,
    })
}

pub fn open_package_metadata(path: impl AsRef<Path>) -> Result<MetadataDto> {
    let metadata = read_package_metadata(path.as_ref())?;
    Ok(MetadataDto {
        metadata: metadata.canonical,
        issues: metadata.issues,
        extra_sections: metadata.raw_sections.extra_sections,
    })
}

pub fn validate_package(path: impl AsRef<Path>) -> Result<ValidationReportDto> {
    let package = open_package(path)?;
    Ok(package.validation_report())
}

pub fn write_package(file: &LasFile, output_dir: impl AsRef<Path>) -> Result<StoredLasFile> {
    write_package_internal(file, output_dir.as_ref(), false)
}

pub fn write_package_overwrite(
    file: &LasFile,
    output_dir: impl AsRef<Path>,
) -> Result<StoredLasFile> {
    write_package_internal(file, output_dir.as_ref(), true)
}

pub fn write_bundle(file: &LasFile, output_dir: impl AsRef<Path>) -> Result<StoredLasFile> {
    write_package(file, output_dir)
}

fn write_package_internal(
    file: &LasFile,
    output_dir: &Path,
    overwrite: bool,
) -> Result<StoredLasFile> {
    validate_edit_state(file)?;
    if output_dir.exists() {
        if overwrite {
            fs::remove_dir_all(output_dir)?;
        } else {
            return Err(LasError::Storage(format!(
                "output directory '{}' already exists",
                output_dir.display()
            )));
        }
    }

    debug!(package = %output_dir.display(), "writing LAS package");
    fs::create_dir_all(output_dir)?;

    let table = file.data();
    let metadata = package_metadata_for(file, PACKAGE_VERSION);

    fs::write(
        metadata_path(output_dir),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    write_parquet_batch(curves_path(output_dir), &table)?;

    open_package(output_dir)
}

fn metadata_path(root: &Path) -> PathBuf {
    root.join(METADATA_FILENAME)
}

fn curves_path(root: &Path) -> PathBuf {
    root.join(CURVES_FILENAME)
}

fn read_package_metadata(root: &Path) -> Result<PackageMetadata> {
    let metadata_text = fs::read_to_string(metadata_path(root))?;
    let metadata: PackageMetadata = serde_json::from_str(&metadata_text)?;
    if metadata.package_version != PACKAGE_VERSION {
        return Err(LasError::Storage(format!(
            "unsupported package version {}",
            metadata.package_version
        )));
    }
    Ok(metadata)
}

fn write_parquet_batch(path: PathBuf, table: &CurveTable) -> Result<()> {
    let batch = table_to_record_batch(table)?;
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)
        .map_err(|err| LasError::Storage(err.to_string()))?;
    writer
        .write(&batch)
        .map_err(|err| LasError::Storage(err.to_string()))?;
    writer
        .close()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    Ok(())
}

fn read_parquet_batch(path: PathBuf, row_count: usize) -> Result<RecordBatch> {
    let file = File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|err| LasError::Storage(err.to_string()))?
        .with_batch_size(row_count.max(1))
        .build()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    let mut batches = reader
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| LasError::Storage(err.to_string()))?;
    match batches.len() {
        0 => Err(LasError::Storage(String::from(
            "package parquet data did not contain any record batches",
        ))),
        1 => Ok(batches.remove(0)),
        _ => merge_batches(batches),
    }
}

fn merge_batches(batches: Vec<RecordBatch>) -> Result<RecordBatch> {
    let first = batches
        .first()
        .ok_or_else(|| LasError::Storage(String::from("no batches to merge")))?;
    let schema = first.schema();
    let mut merged = Vec::with_capacity(schema.fields().len());

    for column_index in 0..schema.fields().len() {
        match schema.field(column_index).data_type() {
            DataType::Float64 => {
                let mut values = Vec::new();
                for batch in &batches {
                    let array = batch
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<Float64Array>()
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
                merged.push(Arc::new(Float64Array::from(values)) as ArrayRef);
            }
            DataType::Utf8 => {
                let mut values = Vec::new();
                for batch in &batches {
                    let array = batch
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<StringArray>()
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
                merged.push(Arc::new(StringArray::from_iter(values)) as ArrayRef);
            }
            other => {
                return Err(LasError::Storage(format!(
                    "unsupported parquet column type during merge: {other:?}"
                )));
            }
        }
    }

    RecordBatch::try_new(schema, merged).map_err(|err| LasError::Storage(err.to_string()))
}

fn materialize_curves(
    curves: &[CurveColumnMetadata],
    table: &CurveTable,
) -> Result<Vec<CurveItem>> {
    curves
        .iter()
        .map(|curve| {
            let values = table
                .column(&curve.name)
                .ok_or_else(|| {
                    LasError::Storage(format!(
                        "column '{}' missing from package table",
                        curve.name
                    ))
                })?
                .values()
                .to_vec();
            Ok(CurveItem {
                mnemonic: curve.name.clone(),
                original_mnemonic: curve.original_mnemonic.clone(),
                unit: curve.unit.clone(),
                value: curve.header_value.clone(),
                description: curve.description.clone(),
                data: values,
            })
        })
        .collect()
}

fn table_to_record_batch(table: &CurveTable) -> Result<RecordBatch> {
    let descriptors = table.descriptors();
    let schema = Arc::new(Schema::new(
        descriptors
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

    let arrays = table
        .column_names()
        .iter()
        .map(|name| {
            let column = table
                .column(name)
                .ok_or_else(|| LasError::Storage(format!("column '{name}' missing from table")))?;
            match column.storage_kind() {
                CurveStorageKind::Numeric => {
                    let values = column
                        .values()
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
                    let values = column.values().iter().map(|value| match value {
                        LasValue::Number(number) => Some(number.to_string()),
                        LasValue::Text(text) => Some(text.clone()),
                        LasValue::Empty => None,
                    });
                    Ok(Arc::new(StringArray::from_iter(values)) as ArrayRef)
                }
            }
        })
        .collect::<Result<Vec<_>>>()?;

    RecordBatch::try_new(schema, arrays).map_err(|err| LasError::Storage(err.to_string()))
}

fn table_from_record_batch(
    batch: &RecordBatch,
    descriptors: &[CurveColumnDescriptor],
) -> Result<CurveTable> {
    let columns = descriptors
        .iter()
        .map(|descriptor| {
            let index = batch.schema().index_of(&descriptor.name).map_err(|err| {
                LasError::Storage(format!(
                    "column '{}' missing from parquet data: {err}",
                    descriptor.name
                ))
            })?;
            let values = values_from_array(batch.column(index).as_ref(), descriptor.storage_kind)?;
            Ok(CurveItem {
                mnemonic: descriptor.name.clone(),
                original_mnemonic: descriptor.name.clone(),
                unit: String::new(),
                value: LasValue::Empty,
                description: String::new(),
                data: values,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(CurveTable::from_curves(&columns))
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
