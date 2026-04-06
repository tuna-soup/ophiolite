use std::collections::HashMap;
use std::path::Path;

use ndarray::{Array2, Array3};
use ophiolite_seismic_io::{
    ChunkReadConfig, GeometryClassification, GeometryOptions, GeometryReport, HeaderField,
    HeaderMapping, ValidationMode, inspect_file, open,
};

use crate::error::SeismicStoreError;
use crate::metadata::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, RegularizationProvenance, SourceIdentity,
    VolumeAxes, VolumeMetadata,
};
use crate::storage::tbvol::{TbvolWriter, recommended_tbvol_tile_shape};
use crate::storage::volume_store::{VolumeStoreWriter, write_dense_volume};
use crate::store::{StoreHandle, open_store};

#[derive(Debug, Clone)]
pub struct SourceVolume {
    pub source: SourceIdentity,
    pub axes: VolumeAxes,
    pub data: Array3<f32>,
    pub occupancy: Option<Array2<u8>>,
}

#[derive(Debug, Clone)]
pub struct SeisGeometryOptions {
    pub header_mapping: HeaderMapping,
    pub third_axis_field: Option<HeaderField>,
}

impl Default for SeisGeometryOptions {
    fn default() -> Self {
        Self {
            header_mapping: HeaderMapping::default(),
            third_axis_field: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SparseSurveyPolicy {
    Reject,
    RegularizeToDense { fill_value: f32 },
}

impl Default for SparseSurveyPolicy {
    fn default() -> Self {
        Self::Reject
    }
}

#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub chunk_shape: [usize; 3],
    pub validation_mode: ValidationMode,
    pub geometry: SeisGeometryOptions,
    pub sparse_survey_policy: SparseSurveyPolicy,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            chunk_shape: [0, 0, 0],
            validation_mode: ValidationMode::Strict,
            geometry: SeisGeometryOptions::default(),
            sparse_survey_policy: SparseSurveyPolicy::Reject,
        }
    }
}

pub fn ingest_segy(
    segy_path: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    let volume = load_source_volume_with_options(segy_path, &options)?;
    let shape = [
        volume.data.shape()[0],
        volume.data.shape()[1],
        volume.data.shape()[2],
    ];
    let volume_metadata = VolumeMetadata {
        kind: DatasetKind::Source,
        source: volume.source.clone(),
        shape,
        axes: volume.axes.clone(),
        created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
        processing_lineage: None,
    };
    let tile_shape = resolve_chunk_shape(options.chunk_shape, shape);
    let writer = TbvolWriter::create(
        &store_root,
        volume_metadata,
        tile_shape,
        volume.occupancy.is_some(),
    )?;
    write_dense_volume(&writer, &volume.data, volume.occupancy.as_ref())?;
    writer.finalize()?;
    open_store(store_root)
}

pub fn load_source_volume(
    segy_path: impl AsRef<Path>,
    validation_mode: ValidationMode,
) -> Result<SourceVolume, SeismicStoreError> {
    load_source_volume_with_options(
        segy_path,
        &IngestOptions {
            validation_mode,
            ..IngestOptions::default()
        },
    )
}

pub fn load_source_volume_with_options(
    segy_path: impl AsRef<Path>,
    options: &IngestOptions,
) -> Result<SourceVolume, SeismicStoreError> {
    let segy_path = segy_path.as_ref();
    let summary = inspect_file(segy_path)?;
    let reader = open(
        segy_path,
        ophiolite_seismic_io::ReaderOptions {
            validation_mode: options.validation_mode,
            header_mapping: options.geometry.header_mapping.clone(),
            ..ophiolite_seismic_io::ReaderOptions::default()
        },
    )?;
    let geometry_report = reader.analyze_geometry(GeometryOptions {
        third_axis_field: options.geometry.third_axis_field,
        ..GeometryOptions::default()
    })?;

    match geometry_report.classification {
        GeometryClassification::RegularDense => {
            let cube = reader.assemble_cube()?;
            if cube.offsets.len() != 1 {
                return Err(SeismicStoreError::UnsupportedOffsetCount {
                    offset_count: cube.offsets.len(),
                });
            }

            let shape = [cube.ilines.len(), cube.xlines.len(), cube.samples_per_trace];
            let data = Array3::from_shape_vec(shape, cube.data)?;
            Ok(SourceVolume {
                source: build_source_identity(
                    segy_path,
                    &summary,
                    cube.samples_per_trace,
                    cube.sample_interval_us,
                    &geometry_report,
                    None,
                ),
                axes: VolumeAxes {
                    ilines: cube.ilines.into_iter().map(|value| value as f64).collect(),
                    xlines: cube.xlines.into_iter().map(|value| value as f64).collect(),
                    sample_axis_ms: cube.sample_axis_ms,
                },
                data,
                occupancy: None,
            })
        }
        GeometryClassification::RegularSparse => regularize_sparse_regular_poststack(
            segy_path,
            &summary,
            &reader,
            &geometry_report,
            options,
        ),
        _ => Err(unsupported_geometry_error(&geometry_report)),
    }
}

fn regularize_sparse_regular_poststack(
    segy_path: &Path,
    summary: &ophiolite_seismic_io::FileSummary,
    reader: &ophiolite_seismic_io::SegyReader,
    geometry_report: &GeometryReport,
    options: &IngestOptions,
) -> Result<SourceVolume, SeismicStoreError> {
    let SparseSurveyPolicy::RegularizeToDense { fill_value } = options.sparse_survey_policy else {
        return Err(unsupported_geometry_error(geometry_report));
    };

    if options.geometry.third_axis_field.is_some() || !geometry_report.third_axis_values.is_empty()
    {
        return Err(SeismicStoreError::UnsupportedRegularizationTarget);
    }

    let headers = reader.load_trace_headers(
        &[
            geometry_report.inline_field,
            geometry_report.crossline_field,
        ],
        ophiolite_seismic_io::TraceSelection::All,
    )?;
    let ilines = headers
        .column(geometry_report.inline_field)
        .expect("geometry analysis validated inline field");
    let xlines = headers
        .column(geometry_report.crossline_field)
        .expect("geometry analysis validated crossline field");
    let traces = reader.read_all_traces(ChunkReadConfig::default())?;
    let samples_per_trace = traces.samples_per_trace;

    let shape = [
        geometry_report.inline_values.len(),
        geometry_report.crossline_values.len(),
        samples_per_trace,
    ];
    let mut data = vec![fill_value; shape[0] * shape[1] * shape[2]];
    let mut occupancy = Array2::<u8>::zeros((shape[0], shape[1]));
    let inline_lookup = index_lookup(&geometry_report.inline_values);
    let xline_lookup = index_lookup(&geometry_report.crossline_values);

    for trace_index in 0..headers.rows() {
        let inline_index = inline_lookup[&ilines[trace_index]];
        let xline_index = xline_lookup[&xlines[trace_index]];
        let dst_start = (inline_index * shape[1] + xline_index) * shape[2];
        let dst_end = dst_start + shape[2];
        data[dst_start..dst_end].copy_from_slice(traces.trace(trace_index));
        occupancy[[inline_index, xline_index]] = 1;
    }

    Ok(SourceVolume {
        source: build_source_identity(
            segy_path,
            summary,
            samples_per_trace,
            reader.resolved_sample_interval_us(),
            geometry_report,
            Some(RegularizationProvenance {
                source_classification: geometry_classification_label(
                    geometry_report.classification,
                )
                .to_string(),
                fill_value,
                observed_trace_count: geometry_report.observed_trace_count,
                expected_trace_count: geometry_report.expected_trace_count,
                missing_bin_count: geometry_report.missing_bin_count,
            }),
        ),
        axes: VolumeAxes {
            ilines: geometry_report
                .inline_values
                .iter()
                .map(|value| *value as f64)
                .collect(),
            xlines: geometry_report
                .crossline_values
                .iter()
                .map(|value| *value as f64)
                .collect(),
            sample_axis_ms: reader.sample_axis_ms(),
        },
        data: Array3::from_shape_vec(shape, data)?,
        occupancy: Some(occupancy),
    })
}

fn build_source_identity(
    segy_path: &Path,
    summary: &ophiolite_seismic_io::FileSummary,
    samples_per_trace: usize,
    sample_interval_us: u16,
    geometry_report: &GeometryReport,
    regularization: Option<RegularizationProvenance>,
) -> SourceIdentity {
    SourceIdentity {
        source_path: segy_path.to_path_buf(),
        file_size: summary.file_size,
        trace_count: summary.trace_count,
        samples_per_trace,
        sample_interval_us,
        sample_format_code: summary.sample_format_code,
        geometry: GeometryProvenance {
            inline_field: header_field_spec(geometry_report.inline_field),
            crossline_field: header_field_spec(geometry_report.crossline_field),
            third_axis_field: geometry_report.third_axis_field.map(header_field_spec),
        },
        regularization,
    }
}

fn unsupported_geometry_error(report: &GeometryReport) -> SeismicStoreError {
    SeismicStoreError::UnsupportedSurveyGeometry {
        classification: report.classification,
        observed_trace_count: report.observed_trace_count,
        expected_trace_count: report.expected_trace_count,
        missing_bin_count: report.missing_bin_count,
        duplicate_coordinate_count: report.duplicate_coordinate_count,
    }
}

pub(crate) fn geometry_classification_label(
    classification: GeometryClassification,
) -> &'static str {
    match classification {
        GeometryClassification::RegularDense => "regular_dense",
        GeometryClassification::RegularSparse => "regular_sparse",
        GeometryClassification::DuplicateCoordinates => "duplicate_coordinates",
        GeometryClassification::NonCartesian => "non_cartesian",
        GeometryClassification::AmbiguousMapping => "ambiguous_mapping",
    }
}

pub fn recommended_chunk_shape(shape: [usize; 3], chunk_target_mib: u16) -> [usize; 3] {
    let [ilines, xlines, samples] = shape;
    let bytes_per_trace = (samples.max(1) * std::mem::size_of::<f32>()) as u64;
    let target_bytes = chunk_target_mib as u64 * 1024 * 1024;
    let trace_budget = (target_bytes / bytes_per_trace).max(1) as usize;
    let max_traces = ilines.max(1) * xlines.max(1);
    let trace_budget = trace_budget.min(max_traces);

    if trace_budget >= max_traces {
        return [ilines.max(1), xlines.max(1), samples.max(1)];
    }

    let ilines_f = ilines.max(1) as f64;
    let xlines_f = xlines.max(1) as f64;
    let ratio = (ilines_f / xlines_f).sqrt();
    let mut ci = ((trace_budget as f64).sqrt() * ratio).floor() as usize;
    ci = ci.clamp(1, ilines.max(1));
    let mut cx = (trace_budget / ci).max(1);
    cx = cx.clamp(1, xlines.max(1));

    while ci < ilines && ci * cx < trace_budget {
        ci += 1;
        if ci * cx > trace_budget {
            ci -= 1;
            break;
        }
    }
    while cx < xlines && ci * cx < trace_budget {
        cx += 1;
        if ci * cx > trace_budget {
            cx -= 1;
            break;
        }
    }

    [ci.max(1), cx.max(1), samples.max(1)]
}

fn resolve_chunk_shape(chunk_shape: [usize; 3], shape: [usize; 3]) -> [usize; 3] {
    if chunk_shape.iter().all(|value| *value == 0) {
        return recommended_tbvol_tile_shape(shape, 4);
    }

    [
        chunk_shape[0].max(1).min(shape[0].max(1)),
        chunk_shape[1].max(1).min(shape[1].max(1)),
        chunk_shape[2].max(1).min(shape[2].max(1)),
    ]
}

pub(crate) fn header_field_spec(field: HeaderField) -> HeaderFieldSpec {
    HeaderFieldSpec {
        name: field.name.to_string(),
        start_byte: field.start_byte,
        value_type: format!("{:?}", field.value_type),
    }
}

fn index_lookup(values: &[i64]) -> HashMap<i64, usize> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| (*value, index))
        .collect()
}
