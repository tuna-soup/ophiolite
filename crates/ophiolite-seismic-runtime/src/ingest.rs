use std::collections::HashMap;
use std::path::Path;

use ndarray::{Array2, Array3};
use ophiolite_seismic_io::{
    ChunkReadConfig, GeometryClassification, GeometryOptions, GeometryReport, HeaderField,
    HeaderMapping, ValidationMode, inspect_file, open,
};

use crate::TileCoord;
use crate::error::SeismicStoreError;
use crate::metadata::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, RegularizationProvenance, SourceIdentity,
    VolumeAxes, VolumeMetadata,
};
use crate::prestack_store::{PrestackStoreHandle, TbgathManifest, create_tbgath_store};
use crate::storage::tbvol::{TbvolWriter, recommended_tbvol_tile_shape};
use crate::storage::volume_store::{VolumeStoreWriter, write_dense_volume};
use crate::store::{StoreHandle, open_store};
use ophiolite_seismic::{GatherAxisKind, SeismicLayout, SeismicStackingState};

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
        Self::RegularizeToDense { fill_value: 0.0 }
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
            sparse_survey_policy: SparseSurveyPolicy::default(),
        }
    }
}

pub fn ingest_segy(
    segy_path: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
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

    if geometry_report.stacking_state == SeismicStackingState::PreStack {
        return Err(SeismicStoreError::Message(format!(
            "prestack ingest requires ingest_prestack_offset_segy; generic ingest_segy remains post-stack-only, resolved layout {:?}",
            geometry_report.layout
        )));
    }

    if geometry_report.classification == GeometryClassification::RegularSparse
        && matches!(
            options.sparse_survey_policy,
            SparseSurveyPolicy::RegularizeToDense { .. }
        )
    {
        return ingest_sparse_regular_poststack_to_tbvol(
            segy_path,
            store_root,
            &summary,
            &reader,
            &geometry_report,
            &options,
        );
    }

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

pub fn ingest_prestack_offset_segy(
    segy_path: impl AsRef<Path>,
    store_root: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<PrestackStoreHandle, SeismicStoreError> {
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

    validate_prestack_offset_geometry(&geometry_report, &options)?;
    let cube = reader.assemble_cube()?;
    let shape = [cube.ilines.len(), cube.xlines.len(), cube.samples_per_trace];
    let source = build_source_identity(
        segy_path,
        &summary,
        cube.samples_per_trace,
        cube.sample_interval_us,
        &geometry_report,
        None,
    );
    let axes = VolumeAxes {
        ilines: cube.ilines.iter().map(|value| *value as f64).collect(),
        xlines: cube.xlines.iter().map(|value| *value as f64).collect(),
        sample_axis_ms: cube.sample_axis_ms.clone(),
    };
    let manifest = TbgathManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            source,
            shape,
            axes,
            created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
            processing_lineage: None,
        },
        SeismicLayout::PreStack3DOffset,
        GatherAxisKind::Offset,
        cube.offsets.into_iter().map(|value| value as f64).collect(),
    );
    create_tbgath_store(store_root, manifest, &cube.data)
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

fn ingest_sparse_regular_poststack_to_tbvol(
    segy_path: &Path,
    store_root: impl AsRef<Path>,
    summary: &ophiolite_seismic_io::FileSummary,
    reader: &ophiolite_seismic_io::SegyReader,
    geometry_report: &GeometryReport,
    options: &IngestOptions,
) -> Result<StoreHandle, SeismicStoreError> {
    let SparseSurveyPolicy::RegularizeToDense { fill_value } = options.sparse_survey_policy else {
        return Err(unsupported_geometry_error(geometry_report));
    };

    if options.geometry.third_axis_field.is_some() || !geometry_report.third_axis_values.is_empty()
    {
        return Err(SeismicStoreError::UnsupportedRegularizationTarget);
    }

    let shape = [
        geometry_report.inline_values.len(),
        geometry_report.crossline_values.len(),
        summary.samples_per_trace as usize,
    ];
    let source = build_source_identity(
        segy_path,
        summary,
        shape[2],
        reader.resolved_sample_interval_us(),
        geometry_report,
        Some(RegularizationProvenance {
            source_classification: geometry_classification_label(geometry_report.classification)
                .to_string(),
            fill_value,
            observed_trace_count: geometry_report.observed_trace_count,
            expected_trace_count: geometry_report.expected_trace_count,
            missing_bin_count: geometry_report.missing_bin_count,
        }),
    );
    let axes = VolumeAxes {
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
    };
    let volume_metadata = VolumeMetadata {
        kind: DatasetKind::Source,
        source,
        shape,
        axes,
        created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
        processing_lineage: None,
    };
    let tile_shape = resolve_chunk_shape(options.chunk_shape, shape);
    let writer = TbvolWriter::create(&store_root, volume_metadata, tile_shape, true)?;
    let geometry = writer.tile_geometry().clone();
    let tile_shape = geometry.tile_shape();

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
    let inline_lookup = index_lookup(&geometry_report.inline_values);
    let xline_lookup = index_lookup(&geometry_report.crossline_values);

    let mut amplitude_map = writer.map_amplitude_mut()?;
    let amplitudes = amplitude_map_as_f32_slice(&mut amplitude_map)?;
    if fill_value != 0.0 {
        amplitudes.fill(fill_value);
    }
    let mut occupancy_map = writer.map_occupancy_mut()?;
    if let Some(mask) = occupancy_map.as_mut() {
        mask.fill(0);
    }

    let chunk = ChunkReadConfig::default();
    let mut scratch = vec![0.0_f32; chunk.traces_per_chunk * shape[2]];
    reader
        .process_trace_chunks_into(chunk, &mut scratch, |trace_chunk| {
            for local_trace in 0..trace_chunk.trace_count {
                let trace_index = trace_chunk.start_trace as usize + local_trace;
                let inline_index = inline_lookup[&ilines[trace_index]];
                let xline_index = xline_lookup[&xlines[trace_index]];
                let tile = TileCoord {
                    tile_i: inline_index / tile_shape[0],
                    tile_x: xline_index / tile_shape[1],
                };
                let local_i = inline_index % tile_shape[0];
                let local_x = xline_index % tile_shape[1];
                let tile_trace_index = (local_i * tile_shape[1]) + local_x;
                let dst_trace_start = (geometry.amplitude_offset(tile) as usize
                    / std::mem::size_of::<f32>())
                    + tile_trace_index * tile_shape[2];
                let src_trace_start = local_trace * trace_chunk.samples_per_trace;
                let src_trace_end = src_trace_start + trace_chunk.samples_per_trace;
                amplitudes[dst_trace_start..dst_trace_start + trace_chunk.samples_per_trace]
                    .copy_from_slice(&trace_chunk.data[src_trace_start..src_trace_end]);

                if let Some(mask) = occupancy_map.as_mut() {
                    let occupancy_index =
                        geometry.occupancy_offset(tile) as usize + tile_trace_index;
                    mask[occupancy_index] = 1;
                }
            }
            Ok::<(), SeismicStoreError>(())
        })
        .map_err(|error| match error {
            ophiolite_seismic_io::ChunkProcessingError::Read(error) => {
                SeismicStoreError::SeisIoRead(error)
            }
            ophiolite_seismic_io::ChunkProcessingError::Sink(error) => error,
        })?;

    amplitude_map.flush()?;
    drop(amplitude_map);
    if let Some(mask) = occupancy_map.as_mut() {
        mask.flush()?;
    }
    drop(occupancy_map);
    writer.finalize()?;
    open_store(store_root)
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

fn validate_prestack_offset_geometry(
    geometry_report: &GeometryReport,
    options: &IngestOptions,
) -> Result<(), SeismicStoreError> {
    if geometry_report.stacking_state != SeismicStackingState::PreStack {
        return Err(SeismicStoreError::Message(format!(
            "prestack offset ingest expected a prestack survey, found {:?}",
            geometry_report.stacking_state
        )));
    }
    if geometry_report.layout != SeismicLayout::PreStack3DOffset {
        return Err(SeismicStoreError::Message(format!(
            "phase-one prestack ingest only supports {:?}, found {:?}",
            SeismicLayout::PreStack3DOffset,
            geometry_report.layout
        )));
    }
    if geometry_report.classification != GeometryClassification::RegularDense {
        return Err(SeismicStoreError::Message(format!(
            "phase-one prestack ingest requires regular dense geometry, found {:?}",
            geometry_report.classification
        )));
    }
    if !matches!(options.sparse_survey_policy, SparseSurveyPolicy::Reject)
        && geometry_report.classification == GeometryClassification::RegularSparse
    {
        return Err(SeismicStoreError::Message(
            "prestack sparse regularization is not implemented in phase one".to_string(),
        ));
    }
    if options.geometry.third_axis_field.is_none() {
        return Err(SeismicStoreError::Message(
            "prestack offset ingest requires resolving the third-axis header field explicitly".to_string(),
        ));
    }
    Ok(())
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

fn amplitude_map_as_f32_slice(map: &mut memmap2::MmapMut) -> Result<&mut [f32], SeismicStoreError> {
    if map.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "tbvol amplitude byte length is not f32 aligned: {}",
            map.len()
        )));
    }
    let (prefix, aligned, suffix) = unsafe { map.align_to_mut::<f32>() };
    if !prefix.is_empty() || !suffix.is_empty() {
        return Err(SeismicStoreError::Message(
            "tbvol amplitude mapping is not aligned to f32".to_string(),
        ));
    }
    Ok(aligned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{DatasetId, GatherRequest, GatherSelector};
    use tempfile::tempdir;

    fn prestack_fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../TraceBoost/test-data/small-ps.sgy")
    }

    #[test]
    fn ingest_prestack_offset_segy_builds_offset_gather_store_when_fixture_is_available() {
        let fixture = prestack_fixture_path();
        if !fixture.exists() {
            return;
        }

        let temp_dir = tempdir().expect("temp dir");
        let output_root = temp_dir.path().join("small-ps.tbgath");
        let handle = ingest_prestack_offset_segy(
            &fixture,
            &output_root,
            IngestOptions {
                geometry: SeisGeometryOptions {
                    third_axis_field: Some(HeaderField::OFFSET),
                    ..SeisGeometryOptions::default()
                },
                sparse_survey_policy: SparseSurveyPolicy::Reject,
                ..IngestOptions::default()
            },
        )
        .expect("fixture should ingest as prestack offset store");

        assert_eq!(handle.manifest.layout, SeismicLayout::PreStack3DOffset);
        assert_eq!(handle.manifest.gather_axis_kind, GatherAxisKind::Offset);
        assert!(!handle.manifest.gather_axis_values.is_empty());

        let gather = handle
            .read_gather_plane(&GatherRequest {
                dataset_id: DatasetId("small-ps.tbgath".to_string()),
                selector: GatherSelector::Ordinal { index: 0 },
            })
            .expect("first prestack gather should be readable");
        assert_eq!(gather.traces, handle.manifest.gather_axis_values.len());
        assert_eq!(gather.samples, handle.manifest.volume.shape[2]);
    }
}
