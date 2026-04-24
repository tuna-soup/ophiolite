use clap::{Parser, Subcommand, ValueEnum};
use ndarray::{Array2, Array3};
use ophiolite_seismic_runtime::{
    CompressionKind, DatasetKind, FrequencyPhaseMode, FrequencyWindowShape, HeaderFieldSpec,
    IngestOptions, ProcessingArtifactRole, ProcessingLineage, ProcessingOperation,
    ProcessingPipeline, ProcessingPipelineSpec, SectionAxis, SourceIdentity, SparseSurveyPolicy,
    StorageLayout, TbvolReader, TbvolWriter, TileCoord, TileGeometry, VolumeAxes, VolumeMetadata,
    VolumeStoreReader, VolumeStoreWriter, ZarrVolumeStoreReader, ZarrVolumeStoreWriter,
    apply_pipeline_to_traces, assemble_section_plane, generate_store_id, load_array,
    load_occupancy, load_source_volume_with_options, materialize_from_reader_writer, open_store,
    preflight_segy, preview_section_from_reader, recommended_chunk_shape,
    recommended_tbvol_tile_shape, write_dense_volume,
};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum DatasetClass {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum StorageCandidate {
    ZarrUncompressedUnsharded,
    ZarrLz4Unsharded,
    ZarrZstdUnsharded,
    ZarrUncompressedSharded,
    ZarrLz4Sharded,
    ZarrZstdSharded,
    Tbvol,
    FlatBinaryControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, Parser)]
#[command(name = "compute-storage-bench")]
#[command(about = "TraceBoost compute/storage benchmark runner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Matrix {
        #[arg(long, value_enum)]
        dataset: Vec<DatasetClass>,
        #[arg(long, value_enum)]
        candidate: Vec<StorageCandidate>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    AnalyzeSegy {
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    AnalyzeSynthetic {
        #[arg(long)]
        ilines: usize,
        #[arg(long)]
        xlines: usize,
        #[arg(long)]
        samples: usize,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    BenchmarkSegy {
        input: PathBuf,
        #[arg(long, default_value_t = 4)]
        chunk_target_mib: u16,
        #[arg(long)]
        shard_target_mib: Option<u16>,
        #[arg(long, value_enum)]
        candidate: Vec<StorageCandidate>,
        #[arg(long, default_value_t = 2.0)]
        scalar_factor: f32,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    BenchmarkSynthetic {
        #[arg(long)]
        ilines: usize,
        #[arg(long)]
        xlines: usize,
        #[arg(long)]
        samples: usize,
        #[arg(long, default_value_t = 4)]
        chunk_target_mib: u16,
        #[arg(long)]
        shard_target_mib: Option<u16>,
        #[arg(long, value_enum)]
        candidate: Vec<StorageCandidate>,
        #[arg(long, default_value_t = 2.0)]
        scalar_factor: f32,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    SweepSynthetic {
        #[arg(long)]
        ilines: usize,
        #[arg(long)]
        xlines: usize,
        #[arg(long)]
        samples: usize,
        #[arg(long, default_value_t = 2.0)]
        scalar_factor: f32,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    SweepTbvol {
        input: PathBuf,
        #[arg(long)]
        chunk_target_mib: Vec<u16>,
        #[arg(long, default_value_t = 2.0)]
        scalar_factor: f32,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Serialize)]
struct BenchPlanRow {
    dataset: DatasetClass,
    candidate: StorageCandidate,
    chunk_target_mib: u16,
    shard_target_mib: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
struct DatasetAnalysis {
    name: String,
    source_kind: String,
    shape: [usize; 3],
    trace_count: usize,
    samples_per_trace: usize,
    runtime_store_bytes_f32: u64,
    runtime_store_mib_f32: f64,
    chunk_candidates: Vec<ChunkCandidateAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
struct ChunkCandidateAnalysis {
    chunk_target_mib: u16,
    chunk_shape: [usize; 3],
    chunk_bytes: u64,
    total_chunks: u64,
    unsharded_file_count: u64,
    shard_candidates: Vec<ShardCandidateAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
struct ShardCandidateAnalysis {
    shard_target_mib: u16,
    shard_shape: [usize; 3],
    approx_shard_count: u64,
}

#[derive(Debug)]
struct BenchmarkDataset {
    name: String,
    shape: [usize; 3],
    data: Array3<f32>,
    axes: VolumeAxes,
    source: SourceIdentity,
    occupancy: Option<Array2<u8>>,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkSummary {
    dataset_name: String,
    shape: [usize; 3],
    chunk_shape: [usize; 3],
    shard_target_mib: Option<u16>,
    scalar_factor: f32,
    results: Vec<StorageBenchmarkResult>,
}

#[derive(Debug, Clone, Serialize)]
struct StorageBenchmarkResult {
    candidate: StorageCandidate,
    chunk_shape: [usize; 3],
    shard_target_mib: Option<u16>,
    compression: String,
    shard_shape: Option<[usize; 3]>,
    input_store_bytes: u64,
    input_file_count: u64,
    inline_section_read_ms: f64,
    xline_section_read_ms: f64,
    preview_amplitude_scalar_ms: f64,
    preview_trace_rms_normalize_ms: f64,
    preview_phase_rotation_ms: f64,
    preview_bandpass_ms: f64,
    preview_bandpass_phase_rotation_ms: f64,
    preview_pipeline_ms: f64,
    apply_amplitude_scalar_ms: f64,
    apply_trace_rms_normalize_ms: f64,
    apply_phase_rotation_ms: f64,
    apply_bandpass_ms: f64,
    apply_bandpass_phase_rotation_ms: f64,
    apply_pipeline_ms: f64,
    pipeline_output_bytes: u64,
    pipeline_output_file_count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct TbvolTileSweepSummary {
    dataset_name: String,
    source_kind: String,
    shape: [usize; 3],
    scalar_factor: f32,
    results: Vec<TbvolTileSweepResult>,
    best_preview_chunk_target_mib: u16,
    best_apply_chunk_target_mib: u16,
    best_io_chunk_target_mib: u16,
    best_balanced_chunk_target_mib: u16,
}

#[derive(Debug, Clone, Serialize)]
struct TbvolTileSweepResult {
    chunk_target_mib: u16,
    chunk_shape: [usize; 3],
    tile_bytes: u64,
    tile_count: u64,
    input_store_bytes: u64,
    input_file_count: u64,
    inline_section_read_ms: f64,
    xline_section_read_ms: f64,
    preview_pipeline_ms: f64,
    apply_pipeline_ms: f64,
    balanced_score: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Matrix {
            dataset,
            candidate,
            format,
        } => {
            let rows = planned_matrix(&dataset, &candidate);
            match format {
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&rows)?),
                OutputFormat::Text => print_text_summary(&rows),
            }
        }
        Command::AnalyzeSegy { input, format } => {
            let preflight = preflight_segy(&input, &IngestOptions::default())?;
            let analysis = analyze_dataset(
                input
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("segy"),
                "segy",
                [
                    preflight.geometry.inline_count,
                    preflight.geometry.crossline_count,
                    preflight.inspection.samples_per_trace as usize,
                ],
            );
            print_dataset_analysis(&analysis, format)?;
        }
        Command::AnalyzeSynthetic {
            ilines,
            xlines,
            samples,
            format,
        } => {
            let analysis = analyze_dataset(
                &format!("synthetic-{ilines}x{xlines}x{samples}"),
                "synthetic",
                [ilines, xlines, samples],
            );
            print_dataset_analysis(&analysis, format)?;
        }
        Command::BenchmarkSegy {
            input,
            chunk_target_mib,
            shard_target_mib,
            candidate,
            scalar_factor,
            format,
        } => {
            let volume = load_source_volume_with_options(
                &input,
                &IngestOptions {
                    sparse_survey_policy: SparseSurveyPolicy::RegularizeToDense { fill_value: 0.0 },
                    ..IngestOptions::default()
                },
            )?;
            let dataset = BenchmarkDataset {
                name: input
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("segy")
                    .to_string(),
                shape: [
                    volume.data.shape()[0],
                    volume.data.shape()[1],
                    volume.data.shape()[2],
                ],
                data: volume.data,
                axes: volume.axes,
                source: volume.source,
                occupancy: volume.occupancy,
            };
            let summary = benchmark_dataset(
                &dataset,
                chunk_target_mib,
                shard_target_mib,
                scalar_factor,
                &selected_or_default_candidates(&candidate),
            )?;
            print_benchmark_summary(&summary, format)?;
        }
        Command::BenchmarkSynthetic {
            ilines,
            xlines,
            samples,
            chunk_target_mib,
            shard_target_mib,
            candidate,
            scalar_factor,
            format,
        } => {
            let dataset = synthetic_dataset(ilines, xlines, samples);
            let summary = benchmark_dataset(
                &dataset,
                chunk_target_mib,
                shard_target_mib,
                scalar_factor,
                &selected_or_default_candidates(&candidate),
            )?;
            print_benchmark_summary(&summary, format)?;
        }
        Command::SweepSynthetic {
            ilines,
            xlines,
            samples,
            scalar_factor,
            format,
        } => {
            let dataset = synthetic_dataset(ilines, xlines, samples);
            let mut results = Vec::new();
            for chunk_target_mib in chunk_targets_mib() {
                for candidate in all_candidates() {
                    if candidate == StorageCandidate::FlatBinaryControl {
                        let bench_root = create_bench_root(&format!(
                            "{}-{chunk_target_mib}-{}",
                            dataset.name,
                            candidate_label(candidate)
                        ))?;
                        results.push(run_candidate_benchmark(
                            &dataset,
                            &bench_root,
                            candidate,
                            chunk_target_mib,
                            None,
                            scalar_factor,
                        )?);
                        continue;
                    }

                    let shard_targets = shard_targets_for(candidate);
                    if shard_targets.is_empty() {
                        let bench_root = create_bench_root(&format!(
                            "{}-{chunk_target_mib}-{}",
                            dataset.name,
                            candidate_label(candidate)
                        ))?;
                        results.push(run_candidate_benchmark(
                            &dataset,
                            &bench_root,
                            candidate,
                            chunk_target_mib,
                            None,
                            scalar_factor,
                        )?);
                    } else {
                        for shard_target_mib in shard_targets {
                            let bench_root = create_bench_root(&format!(
                                "{}-{chunk_target_mib}-{shard_target_mib}-{}",
                                dataset.name,
                                candidate_label(candidate)
                            ))?;
                            results.push(run_candidate_benchmark(
                                &dataset,
                                &bench_root,
                                candidate,
                                chunk_target_mib,
                                Some(shard_target_mib),
                                scalar_factor,
                            )?);
                        }
                    }
                }
            }

            let summary = BenchmarkSummary {
                dataset_name: dataset.name,
                shape: dataset.shape,
                chunk_shape: [0, 0, 0],
                shard_target_mib: None,
                scalar_factor,
                results,
            };
            print_benchmark_summary(&summary, format)?;
        }
        Command::SweepTbvol {
            input,
            chunk_target_mib,
            scalar_factor,
            format,
        } => {
            let (dataset, source_kind) = load_benchmark_dataset_from_path(&input)?;
            let summary = benchmark_tbvol_tile_sweep(
                &dataset,
                &source_kind,
                &requested_or_default_chunk_targets(&chunk_target_mib),
                scalar_factor,
            )?;
            print_tbvol_tile_sweep_summary(&summary, format)?;
        }
    }

    Ok(())
}

fn load_benchmark_dataset_from_path(
    input: &Path,
) -> Result<(BenchmarkDataset, String), Box<dyn std::error::Error>> {
    if input.is_dir() {
        let handle = open_store(input)?;
        let data = load_array(&handle)?;
        let occupancy = load_occupancy(&handle)?;
        let dataset = BenchmarkDataset {
            name: input
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("tbvol")
                .to_string(),
            shape: handle.manifest.volume.shape,
            data,
            axes: handle.manifest.volume.axes.clone(),
            source: handle.manifest.volume.source.clone(),
            occupancy,
        };
        return Ok((dataset, "tbvol".to_string()));
    }

    let extension = input
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "sgy" | "segy" | "su") {
        return Err(format!(
            "unsupported sweep input: {}; expected a SEG-Y/SU file or tbvol directory",
            input.display()
        )
        .into());
    }

    let volume = load_source_volume_with_options(
        input,
        &IngestOptions {
            sparse_survey_policy: SparseSurveyPolicy::RegularizeToDense { fill_value: 0.0 },
            ..IngestOptions::default()
        },
    )?;
    let dataset = BenchmarkDataset {
        name: input
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("segy")
            .to_string(),
        shape: [
            volume.data.shape()[0],
            volume.data.shape()[1],
            volume.data.shape()[2],
        ],
        data: volume.data,
        axes: volume.axes,
        source: volume.source,
        occupancy: volume.occupancy,
    };
    Ok((dataset, "segy".to_string()))
}

fn benchmark_dataset(
    dataset: &BenchmarkDataset,
    chunk_target_mib: u16,
    shard_target_mib: Option<u16>,
    scalar_factor: f32,
    candidates: &[StorageCandidate],
) -> Result<BenchmarkSummary, Box<dyn std::error::Error>> {
    let chunk_shape = recommended_chunk_shape(dataset.shape, chunk_target_mib);
    let bench_root = create_bench_root(&dataset.name)?;
    let mut results = Vec::new();
    for candidate in candidates {
        let candidate_root = bench_root.join(candidate_label(*candidate));
        results.push(run_candidate_benchmark(
            dataset,
            &candidate_root,
            *candidate,
            chunk_target_mib,
            shard_target_mib,
            scalar_factor,
        )?);
    }

    let _ = fs::remove_dir_all(&bench_root);

    Ok(BenchmarkSummary {
        dataset_name: dataset.name.clone(),
        shape: dataset.shape,
        chunk_shape,
        shard_target_mib,
        scalar_factor,
        results,
    })
}

fn benchmark_tbvol_tile_sweep(
    dataset: &BenchmarkDataset,
    source_kind: &str,
    chunk_targets_mib: &[u16],
    scalar_factor: f32,
) -> Result<TbvolTileSweepSummary, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for chunk_target_mib in chunk_targets_mib {
        let bench_root =
            create_bench_root(&format!("{}-tbvol-sweep-{chunk_target_mib}", dataset.name))?;
        let result = run_candidate_benchmark(
            dataset,
            &bench_root,
            StorageCandidate::Tbvol,
            *chunk_target_mib,
            None,
            scalar_factor,
        )?;
        let geometry = TileGeometry::new(dataset.shape, result.chunk_shape);
        rows.push(TbvolTileSweepResult {
            chunk_target_mib: *chunk_target_mib,
            chunk_shape: result.chunk_shape,
            tile_bytes: geometry.amplitude_tile_bytes(),
            tile_count: geometry.tile_count() as u64,
            input_store_bytes: result.input_store_bytes,
            input_file_count: result.input_file_count,
            inline_section_read_ms: result.inline_section_read_ms,
            xline_section_read_ms: result.xline_section_read_ms,
            preview_pipeline_ms: result.preview_pipeline_ms,
            apply_pipeline_ms: result.apply_pipeline_ms,
            balanced_score: 0.0,
        });
    }

    if rows.is_empty() {
        return Err("tbvol tile sweep requires at least one chunk_target_mib".into());
    }

    let min_preview = rows
        .iter()
        .map(|row| row.preview_pipeline_ms)
        .fold(f64::INFINITY, f64::min);
    let min_apply = rows
        .iter()
        .map(|row| row.apply_pipeline_ms)
        .fold(f64::INFINITY, f64::min);
    let min_io = rows
        .iter()
        .map(|row| row.inline_section_read_ms + row.xline_section_read_ms)
        .fold(f64::INFINITY, f64::min);

    for row in &mut rows {
        let io_total = row.inline_section_read_ms + row.xline_section_read_ms;
        row.balanced_score = (row.preview_pipeline_ms / min_preview)
            + (row.apply_pipeline_ms / min_apply)
            + (io_total / min_io);
    }

    let best_preview_chunk_target_mib = rows
        .iter()
        .min_by(|lhs, rhs| lhs.preview_pipeline_ms.total_cmp(&rhs.preview_pipeline_ms))
        .map(|row| row.chunk_target_mib)
        .ok_or("missing preview benchmark rows")?;
    let best_apply_chunk_target_mib = rows
        .iter()
        .min_by(|lhs, rhs| lhs.apply_pipeline_ms.total_cmp(&rhs.apply_pipeline_ms))
        .map(|row| row.chunk_target_mib)
        .ok_or("missing apply benchmark rows")?;
    let best_io_chunk_target_mib = rows
        .iter()
        .min_by(|lhs, rhs| {
            (lhs.inline_section_read_ms + lhs.xline_section_read_ms)
                .total_cmp(&(rhs.inline_section_read_ms + rhs.xline_section_read_ms))
        })
        .map(|row| row.chunk_target_mib)
        .ok_or("missing io benchmark rows")?;
    let best_balanced_chunk_target_mib = rows
        .iter()
        .min_by(|lhs, rhs| lhs.balanced_score.total_cmp(&rhs.balanced_score))
        .map(|row| row.chunk_target_mib)
        .ok_or("missing balanced benchmark rows")?;

    Ok(TbvolTileSweepSummary {
        dataset_name: dataset.name.clone(),
        source_kind: source_kind.to_string(),
        shape: dataset.shape,
        scalar_factor,
        results: rows,
        best_preview_chunk_target_mib,
        best_apply_chunk_target_mib,
        best_io_chunk_target_mib,
        best_balanced_chunk_target_mib,
    })
}

fn run_candidate_benchmark(
    dataset: &BenchmarkDataset,
    bench_root: &Path,
    candidate: StorageCandidate,
    chunk_target_mib: u16,
    shard_target_mib: Option<u16>,
    scalar_factor: f32,
) -> Result<StorageBenchmarkResult, Box<dyn std::error::Error>> {
    if bench_root.exists() {
        fs::remove_dir_all(bench_root)?;
    }
    fs::create_dir_all(bench_root)?;
    let chunk_shape = candidate_chunk_shape(candidate, dataset.shape, chunk_target_mib);
    let sample_interval_ms = sample_interval_ms(&dataset.axes.sample_axis_ms)?;

    let mid_inline = dataset.shape[0] / 2;
    let mid_xline = dataset.shape[1] / 2;
    let amplitude_pipeline = [ProcessingOperation::AmplitudeScalar {
        factor: scalar_factor,
    }];
    let normalize_pipeline = [ProcessingOperation::TraceRmsNormalize];
    let phase_rotation_pipeline = [benchmark_phase_rotation_operation()];
    let bandpass_pipeline = [benchmark_bandpass_operation(sample_interval_ms)];
    let bandpass_phase_rotation_pipeline = [
        benchmark_bandpass_operation(sample_interval_ms),
        benchmark_phase_rotation_operation(),
    ];
    let combined_pipeline = [
        ProcessingOperation::AmplitudeScalar {
            factor: scalar_factor,
        },
        ProcessingOperation::TraceRmsNormalize,
    ];

    let result = match candidate {
        StorageCandidate::FlatBinaryControl => {
            let store = FlatBinaryStore::create(
                bench_root,
                &dataset.data,
                dataset.shape,
                sample_interval_ms,
                dataset.occupancy.as_ref(),
            )?;
            benchmark_flat(
                &store,
                candidate,
                chunk_shape,
                shard_target_mib,
                mid_inline,
                mid_xline,
                &amplitude_pipeline,
                &normalize_pipeline,
                &phase_rotation_pipeline,
                &bandpass_pipeline,
                &bandpass_phase_rotation_pipeline,
                &combined_pipeline,
            )?
        }
        StorageCandidate::Tbvol => {
            let input_root = bench_root.join("input.tbvol");
            create_tbvol_input(&input_root, dataset, chunk_shape)?;
            benchmark_tbvol(
                &input_root,
                candidate,
                chunk_shape,
                mid_inline,
                mid_xline,
                &amplitude_pipeline,
                &normalize_pipeline,
                &phase_rotation_pipeline,
                &bandpass_pipeline,
                &bandpass_phase_rotation_pipeline,
                &combined_pipeline,
            )?
        }
        _ => {
            let layout = storage_layout(candidate, chunk_shape, dataset.shape, shard_target_mib)?;
            let input_root = bench_root.join("input.zarr");
            let volume = VolumeMetadata {
                kind: ophiolite_seismic_runtime::DatasetKind::Source,
                store_id: generate_store_id(),
                source: dataset.source.clone(),
                shape: dataset.shape,
                axes: dataset.axes.clone(),
                coordinate_reference_binding: None,
                spatial: None,
                created_by: "compute-storage-bench".to_string(),
                processing_lineage: None,
                segy_export: None,
            };
            let writer = ZarrVolumeStoreWriter::create(
                &input_root,
                volume,
                chunk_shape,
                layout.clone(),
                dataset.occupancy.is_some(),
            )?;
            write_dense_volume(&writer, &dataset.data, dataset.occupancy.as_ref())?;
            writer.finalize()?;
            benchmark_zarr(
                &input_root,
                candidate,
                &layout,
                chunk_shape,
                shard_target_mib,
                mid_inline,
                mid_xline,
                &amplitude_pipeline,
                &normalize_pipeline,
                &phase_rotation_pipeline,
                &bandpass_pipeline,
                &bandpass_phase_rotation_pipeline,
                &combined_pipeline,
            )?
        }
    };

    let _ = fs::remove_dir_all(bench_root);
    Ok(result)
}

fn benchmark_zarr(
    input_root: &Path,
    candidate: StorageCandidate,
    layout: &StorageLayout,
    chunk_shape: [usize; 3],
    shard_target_mib: Option<u16>,
    mid_inline: usize,
    mid_xline: usize,
    amplitude_pipeline: &[ProcessingOperation],
    normalize_pipeline: &[ProcessingOperation],
    phase_rotation_pipeline: &[ProcessingOperation],
    bandpass_pipeline: &[ProcessingOperation],
    bandpass_phase_rotation_pipeline: &[ProcessingOperation],
    combined_pipeline: &[ProcessingOperation],
) -> Result<StorageBenchmarkResult, Box<dyn std::error::Error>> {
    let reader = ZarrVolumeStoreReader::open(input_root)?;
    let started = Instant::now();
    let _inline_plane = assemble_section_plane(&reader, SectionAxis::Inline, mid_inline)?;
    let inline_section_read_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _xline_plane = assemble_section_plane(&reader, SectionAxis::Xline, mid_xline)?;
    let xline_section_read_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, amplitude_pipeline)?;
    let preview_amplitude_scalar_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, normalize_pipeline)?;
    let preview_trace_rms_normalize_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ = preview_section_from_reader(
        &reader,
        SectionAxis::Inline,
        mid_inline,
        phase_rotation_pipeline,
    )?;
    let preview_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, bandpass_pipeline)?;
    let preview_bandpass_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ = preview_section_from_reader(
        &reader,
        SectionAxis::Inline,
        mid_inline,
        bandpass_phase_rotation_pipeline,
    )?;
    let preview_bandpass_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, combined_pipeline)?;
    let preview_pipeline_ms = started.elapsed().as_secs_f64() * 1000.0;

    let amplitude_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-amplitude.zarr");
    let amplitude_writer = ZarrVolumeStoreWriter::create(
        &amplitude_output,
        derived_output_volume(reader.volume(), input_root, amplitude_pipeline),
        chunk_shape,
        layout.clone(),
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, amplitude_writer, amplitude_pipeline)?;
    let apply_amplitude_scalar_ms = started.elapsed().as_secs_f64() * 1000.0;

    let normalize_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-normalize.zarr");
    let normalize_writer = ZarrVolumeStoreWriter::create(
        &normalize_output,
        derived_output_volume(reader.volume(), input_root, normalize_pipeline),
        chunk_shape,
        layout.clone(),
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, normalize_writer, normalize_pipeline)?;
    let apply_trace_rms_normalize_ms = started.elapsed().as_secs_f64() * 1000.0;

    let phase_rotation_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-phase-rotation.zarr");
    let phase_rotation_writer = ZarrVolumeStoreWriter::create(
        &phase_rotation_output,
        derived_output_volume(reader.volume(), input_root, phase_rotation_pipeline),
        chunk_shape,
        layout.clone(),
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, phase_rotation_writer, phase_rotation_pipeline)?;
    let apply_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let bandpass_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-bandpass.zarr");
    let bandpass_writer = ZarrVolumeStoreWriter::create(
        &bandpass_output,
        derived_output_volume(reader.volume(), input_root, bandpass_pipeline),
        chunk_shape,
        layout.clone(),
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, bandpass_writer, bandpass_pipeline)?;
    let apply_bandpass_ms = started.elapsed().as_secs_f64() * 1000.0;

    let bandpass_phase_rotation_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-bandpass-phase-rotation.zarr");
    let bandpass_phase_rotation_writer = ZarrVolumeStoreWriter::create(
        &bandpass_phase_rotation_output,
        derived_output_volume(
            reader.volume(),
            input_root,
            bandpass_phase_rotation_pipeline,
        ),
        chunk_shape,
        layout.clone(),
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(
        &reader,
        bandpass_phase_rotation_writer,
        bandpass_phase_rotation_pipeline,
    )?;
    let apply_bandpass_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let pipeline_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-pipeline.zarr");
    let pipeline_writer = ZarrVolumeStoreWriter::create(
        &pipeline_output,
        derived_output_volume(reader.volume(), input_root, combined_pipeline),
        chunk_shape,
        layout.clone(),
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, pipeline_writer, combined_pipeline)?;
    let apply_pipeline_ms = started.elapsed().as_secs_f64() * 1000.0;

    let (input_store_bytes, input_file_count) = directory_metrics(input_root)?;
    let (pipeline_output_bytes, pipeline_output_file_count) = directory_metrics(&pipeline_output)?;

    let _ = fs::remove_dir_all(amplitude_output);
    let _ = fs::remove_dir_all(normalize_output);
    let _ = fs::remove_dir_all(phase_rotation_output);
    let _ = fs::remove_dir_all(bandpass_output);
    let _ = fs::remove_dir_all(bandpass_phase_rotation_output);
    let _ = fs::remove_dir_all(&pipeline_output);

    Ok(StorageBenchmarkResult {
        candidate,
        chunk_shape,
        shard_target_mib,
        compression: compression_label(&layout.compression).to_string(),
        shard_shape: layout.shard_shape,
        input_store_bytes,
        input_file_count,
        inline_section_read_ms,
        xline_section_read_ms,
        preview_amplitude_scalar_ms,
        preview_trace_rms_normalize_ms,
        preview_phase_rotation_ms,
        preview_bandpass_ms,
        preview_bandpass_phase_rotation_ms,
        preview_pipeline_ms,
        apply_amplitude_scalar_ms,
        apply_trace_rms_normalize_ms,
        apply_phase_rotation_ms,
        apply_bandpass_ms,
        apply_bandpass_phase_rotation_ms,
        apply_pipeline_ms,
        pipeline_output_bytes,
        pipeline_output_file_count,
    })
}

fn benchmark_tbvol(
    input_root: &Path,
    candidate: StorageCandidate,
    chunk_shape: [usize; 3],
    mid_inline: usize,
    mid_xline: usize,
    amplitude_pipeline: &[ProcessingOperation],
    normalize_pipeline: &[ProcessingOperation],
    phase_rotation_pipeline: &[ProcessingOperation],
    bandpass_pipeline: &[ProcessingOperation],
    bandpass_phase_rotation_pipeline: &[ProcessingOperation],
    combined_pipeline: &[ProcessingOperation],
) -> Result<StorageBenchmarkResult, Box<dyn std::error::Error>> {
    let reader = TbvolReader::open(input_root)?;

    let started = Instant::now();
    let _inline_plane = assemble_section_plane(&reader, SectionAxis::Inline, mid_inline)?;
    let inline_section_read_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _xline_plane = assemble_section_plane(&reader, SectionAxis::Xline, mid_xline)?;
    let xline_section_read_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, amplitude_pipeline)?;
    let preview_amplitude_scalar_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, normalize_pipeline)?;
    let preview_trace_rms_normalize_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ = preview_section_from_reader(
        &reader,
        SectionAxis::Inline,
        mid_inline,
        phase_rotation_pipeline,
    )?;
    let preview_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, bandpass_pipeline)?;
    let preview_bandpass_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ = preview_section_from_reader(
        &reader,
        SectionAxis::Inline,
        mid_inline,
        bandpass_phase_rotation_pipeline,
    )?;
    let preview_bandpass_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ =
        preview_section_from_reader(&reader, SectionAxis::Inline, mid_inline, combined_pipeline)?;
    let preview_pipeline_ms = started.elapsed().as_secs_f64() * 1000.0;

    let amplitude_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-amplitude.tbvol");
    let amplitude_writer = TbvolWriter::create(
        &amplitude_output,
        derived_output_volume(reader.volume(), input_root, amplitude_pipeline),
        chunk_shape,
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, amplitude_writer, amplitude_pipeline)?;
    let apply_amplitude_scalar_ms = started.elapsed().as_secs_f64() * 1000.0;

    let normalize_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-normalize.tbvol");
    let normalize_writer = TbvolWriter::create(
        &normalize_output,
        derived_output_volume(reader.volume(), input_root, normalize_pipeline),
        chunk_shape,
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, normalize_writer, normalize_pipeline)?;
    let apply_trace_rms_normalize_ms = started.elapsed().as_secs_f64() * 1000.0;

    let phase_rotation_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-phase-rotation.tbvol");
    let phase_rotation_writer = TbvolWriter::create(
        &phase_rotation_output,
        derived_output_volume(reader.volume(), input_root, phase_rotation_pipeline),
        chunk_shape,
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, phase_rotation_writer, phase_rotation_pipeline)?;
    let apply_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let bandpass_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-bandpass.tbvol");
    let bandpass_writer = TbvolWriter::create(
        &bandpass_output,
        derived_output_volume(reader.volume(), input_root, bandpass_pipeline),
        chunk_shape,
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, bandpass_writer, bandpass_pipeline)?;
    let apply_bandpass_ms = started.elapsed().as_secs_f64() * 1000.0;

    let bandpass_phase_rotation_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-bandpass-phase-rotation.tbvol");
    let bandpass_phase_rotation_writer = TbvolWriter::create(
        &bandpass_phase_rotation_output,
        derived_output_volume(
            reader.volume(),
            input_root,
            bandpass_phase_rotation_pipeline,
        ),
        chunk_shape,
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(
        &reader,
        bandpass_phase_rotation_writer,
        bandpass_phase_rotation_pipeline,
    )?;
    let apply_bandpass_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let pipeline_output = input_root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("apply-pipeline.tbvol");
    let pipeline_writer = TbvolWriter::create(
        &pipeline_output,
        derived_output_volume(reader.volume(), input_root, combined_pipeline),
        chunk_shape,
        reader_has_occupancy(&reader)?,
    )?;
    let started = Instant::now();
    materialize_from_reader_writer(&reader, pipeline_writer, combined_pipeline)?;
    let apply_pipeline_ms = started.elapsed().as_secs_f64() * 1000.0;

    let (input_store_bytes, input_file_count) = directory_metrics(input_root)?;
    let (pipeline_output_bytes, pipeline_output_file_count) = directory_metrics(&pipeline_output)?;

    let _ = fs::remove_dir_all(amplitude_output);
    let _ = fs::remove_dir_all(normalize_output);
    let _ = fs::remove_dir_all(phase_rotation_output);
    let _ = fs::remove_dir_all(bandpass_output);
    let _ = fs::remove_dir_all(bandpass_phase_rotation_output);
    let _ = fs::remove_dir_all(&pipeline_output);

    Ok(StorageBenchmarkResult {
        candidate,
        chunk_shape,
        shard_target_mib: None,
        compression: "none".to_string(),
        shard_shape: None,
        input_store_bytes,
        input_file_count,
        inline_section_read_ms,
        xline_section_read_ms,
        preview_amplitude_scalar_ms,
        preview_trace_rms_normalize_ms,
        preview_phase_rotation_ms,
        preview_bandpass_ms,
        preview_bandpass_phase_rotation_ms,
        preview_pipeline_ms,
        apply_amplitude_scalar_ms,
        apply_trace_rms_normalize_ms,
        apply_phase_rotation_ms,
        apply_bandpass_ms,
        apply_bandpass_phase_rotation_ms,
        apply_pipeline_ms,
        pipeline_output_bytes,
        pipeline_output_file_count,
    })
}

fn create_tbvol_input(
    root: &Path,
    dataset: &BenchmarkDataset,
    chunk_shape: [usize; 3],
) -> Result<(), Box<dyn std::error::Error>> {
    let volume = benchmark_volume(dataset);
    let geometry = TileGeometry::new(dataset.shape, chunk_shape);
    let writer = TbvolWriter::create(root, volume, chunk_shape, dataset.occupancy.is_some())?;

    for tile in geometry.iter_tiles() {
        let amplitude_tile = build_amplitude_tile(dataset, &geometry, tile);
        writer.write_tile(tile, &amplitude_tile)?;
        if let Some(mask) = dataset.occupancy.as_ref() {
            let occupancy_tile = build_occupancy_tile(mask, &geometry, tile);
            writer.write_tile_occupancy(tile, &occupancy_tile)?;
        }
    }

    writer.finalize()?;
    Ok(())
}

fn build_amplitude_tile(
    dataset: &BenchmarkDataset,
    geometry: &TileGeometry,
    tile: TileCoord,
) -> Vec<f32> {
    let tile_shape = geometry.tile_shape();
    let effective = geometry.effective_tile_shape(tile);
    let origin = geometry.tile_origin(tile);
    let mut values = vec![0.0_f32; geometry.amplitude_tile_len()];

    for local_i in 0..effective[0] {
        for local_x in 0..effective[1] {
            let dst = ((local_i * tile_shape[1]) + local_x) * tile_shape[2];
            for sample in 0..effective[2] {
                values[dst + sample] =
                    dataset.data[[origin[0] + local_i, origin[1] + local_x, sample]];
            }
        }
    }

    values
}

fn build_occupancy_tile(
    occupancy: &Array2<u8>,
    geometry: &TileGeometry,
    tile: TileCoord,
) -> Vec<u8> {
    let tile_shape = geometry.tile_shape();
    let effective = geometry.effective_tile_shape(tile);
    let origin = geometry.tile_origin(tile);
    let mut values = vec![0_u8; geometry.occupancy_tile_len()];

    for local_i in 0..effective[0] {
        for local_x in 0..effective[1] {
            let dst = local_i * tile_shape[1] + local_x;
            values[dst] = occupancy[[origin[0] + local_i, origin[1] + local_x]];
        }
    }

    values
}

fn benchmark_volume(dataset: &BenchmarkDataset) -> VolumeMetadata {
    VolumeMetadata {
        kind: DatasetKind::Source,
        store_id: generate_store_id(),
        source: dataset.source.clone(),
        shape: dataset.shape,
        axes: dataset.axes.clone(),
        coordinate_reference_binding: None,
        spatial: None,
        created_by: "compute-storage-bench".to_string(),
        processing_lineage: None,
        segy_export: None,
    }
}

fn derived_output_volume(
    input: &VolumeMetadata,
    parent_store: &Path,
    pipeline: &[ProcessingOperation],
) -> VolumeMetadata {
    VolumeMetadata {
        kind: DatasetKind::Derived,
        store_id: generate_store_id(),
        source: input.source.clone(),
        shape: input.shape,
        axes: input.axes.clone(),
        coordinate_reference_binding: input.coordinate_reference_binding.clone(),
        spatial: input.spatial.clone(),
        created_by: "compute-storage-bench".to_string(),
        processing_lineage: Some(ProcessingLineage {
            schema_version: 1,
            parent_store: parent_store.to_path_buf(),
            parent_store_id: input.store_id.clone(),
            artifact_role: ProcessingArtifactRole::FinalOutput,
            pipeline: ProcessingPipelineSpec::TraceLocal {
                pipeline: ProcessingPipeline {
                    schema_version: 2,
                    revision: 1,
                    preset_id: None,
                    name: Some("compute-storage-bench".to_string()),
                    description: None,
                    steps: pipeline
                        .iter()
                        .cloned()
                        .map(|operation| ophiolite_seismic::TraceLocalProcessingStep {
                            operation,
                            checkpoint: false,
                        })
                        .collect(),
                },
            },
            pipeline_identity: None,
            operator_set_identity: None,
            planner_profile_identity: None,
            source_identity: None,
            runtime_semantics_version: String::new(),
            store_writer_semantics_version: String::new(),
            runtime_version: "compute-storage-bench".to_string(),
            created_at_unix_s: unix_timestamp_s(),
            artifact_key: None,
            input_artifact_keys: Vec::new(),
            produced_by_stage_id: None,
            boundary_reason: None,
            logical_domain: None,
            chunk_grid_spec: None,
            geometry_fingerprints: None,
        }),
        segy_export: None,
    }
}

fn reader_has_occupancy<R: VolumeStoreReader>(
    reader: &R,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(reader
        .read_tile_occupancy(TileCoord {
            tile_i: 0,
            tile_x: 0,
        })?
        .is_some())
}

fn benchmark_flat(
    store: &FlatBinaryStore,
    candidate: StorageCandidate,
    chunk_shape: [usize; 3],
    shard_target_mib: Option<u16>,
    mid_inline: usize,
    mid_xline: usize,
    amplitude_pipeline: &[ProcessingOperation],
    normalize_pipeline: &[ProcessingOperation],
    phase_rotation_pipeline: &[ProcessingOperation],
    bandpass_pipeline: &[ProcessingOperation],
    bandpass_phase_rotation_pipeline: &[ProcessingOperation],
    combined_pipeline: &[ProcessingOperation],
) -> Result<StorageBenchmarkResult, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let _ = store.read_inline_section(mid_inline)?;
    let inline_section_read_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let _ = store.read_xline_section(mid_xline)?;
    let xline_section_read_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let mut plane = store.read_inline_section(mid_inline)?;
    apply_pipeline_to_traces(
        &mut plane,
        store.shape[1],
        store.shape[2],
        store.sample_interval_ms,
        store.occupancy_row(mid_inline).as_deref(),
        amplitude_pipeline,
    )?;
    let preview_amplitude_scalar_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let mut plane = store.read_inline_section(mid_inline)?;
    apply_pipeline_to_traces(
        &mut plane,
        store.shape[1],
        store.shape[2],
        store.sample_interval_ms,
        store.occupancy_row(mid_inline).as_deref(),
        normalize_pipeline,
    )?;
    let preview_trace_rms_normalize_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let mut plane = store.read_inline_section(mid_inline)?;
    apply_pipeline_to_traces(
        &mut plane,
        store.shape[1],
        store.shape[2],
        store.sample_interval_ms,
        store.occupancy_row(mid_inline).as_deref(),
        phase_rotation_pipeline,
    )?;
    let preview_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let mut plane = store.read_inline_section(mid_inline)?;
    apply_pipeline_to_traces(
        &mut plane,
        store.shape[1],
        store.shape[2],
        store.sample_interval_ms,
        store.occupancy_row(mid_inline).as_deref(),
        bandpass_pipeline,
    )?;
    let preview_bandpass_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let mut plane = store.read_inline_section(mid_inline)?;
    apply_pipeline_to_traces(
        &mut plane,
        store.shape[1],
        store.shape[2],
        store.sample_interval_ms,
        store.occupancy_row(mid_inline).as_deref(),
        bandpass_phase_rotation_pipeline,
    )?;
    let preview_bandpass_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let mut plane = store.read_inline_section(mid_inline)?;
    apply_pipeline_to_traces(
        &mut plane,
        store.shape[1],
        store.shape[2],
        store.sample_interval_ms,
        store.occupancy_row(mid_inline).as_deref(),
        combined_pipeline,
    )?;
    let preview_pipeline_ms = started.elapsed().as_secs_f64() * 1000.0;

    let amplitude_output = store.root.join("apply-amplitude.bin");
    let started = Instant::now();
    store.materialize(&amplitude_output, chunk_shape, amplitude_pipeline)?;
    let apply_amplitude_scalar_ms = started.elapsed().as_secs_f64() * 1000.0;

    let normalize_output = store.root.join("apply-normalize.bin");
    let started = Instant::now();
    store.materialize(&normalize_output, chunk_shape, normalize_pipeline)?;
    let apply_trace_rms_normalize_ms = started.elapsed().as_secs_f64() * 1000.0;

    let phase_rotation_output = store.root.join("apply-phase-rotation.bin");
    let started = Instant::now();
    store.materialize(&phase_rotation_output, chunk_shape, phase_rotation_pipeline)?;
    let apply_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let bandpass_output = store.root.join("apply-bandpass.bin");
    let started = Instant::now();
    store.materialize(&bandpass_output, chunk_shape, bandpass_pipeline)?;
    let apply_bandpass_ms = started.elapsed().as_secs_f64() * 1000.0;

    let bandpass_phase_rotation_output = store.root.join("apply-bandpass-phase-rotation.bin");
    let started = Instant::now();
    store.materialize(
        &bandpass_phase_rotation_output,
        chunk_shape,
        bandpass_phase_rotation_pipeline,
    )?;
    let apply_bandpass_phase_rotation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let pipeline_output = store.root.join("apply-pipeline.bin");
    let started = Instant::now();
    store.materialize(&pipeline_output, chunk_shape, combined_pipeline)?;
    let apply_pipeline_ms = started.elapsed().as_secs_f64() * 1000.0;

    let input_store_bytes = fs::metadata(&store.data_path)?.len();
    let input_file_count = if store.occupancy.is_some() { 3 } else { 2 };
    let pipeline_output_bytes = fs::metadata(&pipeline_output)?.len();

    let _ = fs::remove_file(amplitude_output);
    let _ = fs::remove_file(normalize_output);
    let _ = fs::remove_file(phase_rotation_output);
    let _ = fs::remove_file(bandpass_output);
    let _ = fs::remove_file(bandpass_phase_rotation_output);
    let _ = fs::remove_file(&pipeline_output);

    Ok(StorageBenchmarkResult {
        candidate,
        chunk_shape,
        shard_target_mib,
        compression: "none".to_string(),
        shard_shape: None,
        input_store_bytes,
        input_file_count,
        inline_section_read_ms,
        xline_section_read_ms,
        preview_amplitude_scalar_ms,
        preview_trace_rms_normalize_ms,
        preview_phase_rotation_ms,
        preview_bandpass_ms,
        preview_bandpass_phase_rotation_ms,
        preview_pipeline_ms,
        apply_amplitude_scalar_ms,
        apply_trace_rms_normalize_ms,
        apply_phase_rotation_ms,
        apply_bandpass_ms,
        apply_bandpass_phase_rotation_ms,
        apply_pipeline_ms,
        pipeline_output_bytes,
        pipeline_output_file_count: 1,
    })
}

fn planned_matrix(dataset: &[DatasetClass], candidate: &[StorageCandidate]) -> Vec<BenchPlanRow> {
    let datasets = if dataset.is_empty() {
        vec![
            DatasetClass::Small,
            DatasetClass::Medium,
            DatasetClass::Large,
        ]
    } else {
        dataset.to_vec()
    };
    let candidates = selected_or_default_candidates(candidate);

    let mut rows = Vec::new();
    for dataset in datasets {
        for candidate in &candidates {
            for chunk_target_mib in chunk_targets_mib() {
                let shard_targets = shard_targets_for(*candidate);
                if shard_targets.is_empty() {
                    rows.push(BenchPlanRow {
                        dataset,
                        candidate: *candidate,
                        chunk_target_mib,
                        shard_target_mib: None,
                    });
                } else {
                    for shard_target_mib in shard_targets {
                        rows.push(BenchPlanRow {
                            dataset,
                            candidate: *candidate,
                            chunk_target_mib,
                            shard_target_mib: Some(shard_target_mib),
                        });
                    }
                }
            }
        }
    }
    rows
}

fn selected_or_default_candidates(selected: &[StorageCandidate]) -> Vec<StorageCandidate> {
    if selected.is_empty() {
        all_candidates()
    } else {
        selected.to_vec()
    }
}

fn all_candidates() -> Vec<StorageCandidate> {
    vec![
        StorageCandidate::ZarrUncompressedUnsharded,
        StorageCandidate::ZarrLz4Unsharded,
        StorageCandidate::ZarrZstdUnsharded,
        StorageCandidate::ZarrUncompressedSharded,
        StorageCandidate::ZarrLz4Sharded,
        StorageCandidate::ZarrZstdSharded,
        StorageCandidate::Tbvol,
        StorageCandidate::FlatBinaryControl,
    ]
}

fn storage_layout(
    candidate: StorageCandidate,
    chunk_shape: [usize; 3],
    shape: [usize; 3],
    shard_target_mib: Option<u16>,
) -> Result<StorageLayout, Box<dyn std::error::Error>> {
    let compression = match candidate {
        StorageCandidate::ZarrUncompressedUnsharded | StorageCandidate::ZarrUncompressedSharded => {
            CompressionKind::None
        }
        StorageCandidate::ZarrLz4Unsharded | StorageCandidate::ZarrLz4Sharded => {
            CompressionKind::BloscLz4
        }
        StorageCandidate::ZarrZstdUnsharded | StorageCandidate::ZarrZstdSharded => {
            CompressionKind::Zstd
        }
        StorageCandidate::Tbvol => CompressionKind::None,
        StorageCandidate::FlatBinaryControl => CompressionKind::None,
    };

    let shard_shape = match candidate {
        StorageCandidate::ZarrUncompressedSharded
        | StorageCandidate::ZarrLz4Sharded
        | StorageCandidate::ZarrZstdSharded => Some(derive_shard_shape(
            shape,
            chunk_shape,
            shard_target_mib.ok_or("sharded candidate requires shard_target_mib")?,
        )),
        _ => None,
    };

    Ok(StorageLayout {
        compression,
        shard_shape,
    })
}

fn candidate_chunk_shape(
    candidate: StorageCandidate,
    shape: [usize; 3],
    chunk_target_mib: u16,
) -> [usize; 3] {
    match candidate {
        StorageCandidate::Tbvol => recommended_tbvol_tile_shape(shape, chunk_target_mib),
        _ => recommended_chunk_shape(shape, chunk_target_mib),
    }
}

fn analyze_dataset(name: &str, source_kind: &str, shape: [usize; 3]) -> DatasetAnalysis {
    let trace_count = shape[0] * shape[1];
    let samples_per_trace = shape[2];
    let runtime_store_bytes_f32 = trace_count as u64 * samples_per_trace as u64 * 4;
    let chunk_candidates = chunk_targets_mib()
        .into_iter()
        .map(|chunk_target_mib| {
            let chunk_shape = recommended_chunk_shape(shape, chunk_target_mib);
            let chunk_bytes =
                chunk_shape[0] as u64 * chunk_shape[1] as u64 * chunk_shape[2] as u64 * 4;
            let chunks_i = div_ceil(shape[0] as u64, chunk_shape[0] as u64);
            let chunks_x = div_ceil(shape[1] as u64, chunk_shape[1] as u64);
            let total_chunks = chunks_i * chunks_x;
            let shard_candidates = shard_targets_mib()
                .into_iter()
                .map(|shard_target_mib| {
                    let shard_shape = derive_shard_shape(shape, chunk_shape, shard_target_mib);
                    let shard_chunks_i = div_ceil(shape[0] as u64, shard_shape[0] as u64);
                    let shard_chunks_x = div_ceil(shape[1] as u64, shard_shape[1] as u64);
                    ShardCandidateAnalysis {
                        shard_target_mib,
                        shard_shape,
                        approx_shard_count: shard_chunks_i * shard_chunks_x,
                    }
                })
                .collect();

            ChunkCandidateAnalysis {
                chunk_target_mib,
                chunk_shape,
                chunk_bytes,
                total_chunks,
                unsharded_file_count: total_chunks,
                shard_candidates,
            }
        })
        .collect();

    DatasetAnalysis {
        name: name.to_string(),
        source_kind: source_kind.to_string(),
        shape,
        trace_count,
        samples_per_trace,
        runtime_store_bytes_f32,
        runtime_store_mib_f32: bytes_to_mib(runtime_store_bytes_f32),
        chunk_candidates,
    }
}

fn chunk_targets_mib() -> Vec<u16> {
    vec![1, 2, 4, 8]
}

fn requested_or_default_chunk_targets(selected: &[u16]) -> Vec<u16> {
    if selected.is_empty() {
        return chunk_targets_mib();
    }

    let mut chunk_targets = selected.to_vec();
    chunk_targets.sort_unstable();
    chunk_targets.dedup();
    chunk_targets
}

fn shard_targets_mib() -> Vec<u16> {
    vec![32, 64, 128, 256]
}

fn shard_targets_for(candidate: StorageCandidate) -> Vec<u16> {
    match candidate {
        StorageCandidate::ZarrUncompressedSharded
        | StorageCandidate::ZarrLz4Sharded
        | StorageCandidate::ZarrZstdSharded => shard_targets_mib(),
        _ => Vec::new(),
    }
}

fn derive_shard_shape(
    shape: [usize; 3],
    chunk_shape: [usize; 3],
    shard_target_mib: u16,
) -> [usize; 3] {
    let target_bytes = shard_target_mib as u64 * 1024 * 1024;
    let inner_chunk_bytes =
        chunk_shape[0] as u64 * chunk_shape[1] as u64 * chunk_shape[2] as u64 * 4;
    let chunk_budget = (target_bytes / inner_chunk_bytes).max(1) as usize;
    let chunk_grid = [
        div_ceil(shape[0] as u64, chunk_shape[0] as u64) as usize,
        div_ceil(shape[1] as u64, chunk_shape[1] as u64) as usize,
    ];
    let max_inner_chunks = chunk_grid[0].max(1) * chunk_grid[1].max(1);
    let chunk_budget = chunk_budget.min(max_inner_chunks);
    if chunk_budget >= max_inner_chunks {
        return [
            chunk_grid[0].max(1) * chunk_shape[0],
            chunk_grid[1].max(1) * chunk_shape[1],
            chunk_shape[2].max(1),
        ];
    }

    let ratio = (chunk_grid[0].max(1) as f64 / chunk_grid[1].max(1) as f64).sqrt();
    let mut si = ((chunk_budget as f64).sqrt() * ratio).floor() as usize;
    si = si.clamp(1, chunk_grid[0].max(1));
    let mut sx = (chunk_budget / si).max(1);
    sx = sx.clamp(1, chunk_grid[1].max(1));

    [
        (si * chunk_shape[0]).max(chunk_shape[0]),
        (sx * chunk_shape[1]).max(chunk_shape[1]),
        chunk_shape[2].max(1),
    ]
}

fn synthetic_dataset(ilines: usize, xlines: usize, samples: usize) -> BenchmarkDataset {
    let shape = [ilines, xlines, samples];
    let data = Array3::from_shape_fn((ilines, xlines, samples), |(iline, xline, sample)| {
        let il = iline as f32 / ilines.max(1) as f32;
        let xl = xline as f32 / xlines.max(1) as f32;
        let smp = sample as f32 / samples.max(1) as f32;
        ((il * 17.0).sin() + (xl * 11.0).cos()) * (1.0 - smp) + (smp * 31.0).sin() * 0.35
    });
    let axes = VolumeAxes::from_time_axis(
        (0..ilines).map(|value| value as f64).collect(),
        (0..xlines).map(|value| value as f64).collect(),
        (0..samples).map(|value| value as f32 * 2.0).collect(),
    );
    let source = SourceIdentity {
        source_path: PathBuf::from(format!("synthetic://{ilines}x{xlines}x{samples}")),
        file_size: (ilines * xlines * samples * std::mem::size_of::<f32>()) as u64,
        trace_count: (ilines * xlines) as u64,
        samples_per_trace: samples,
        sample_interval_us: 2000,
        sample_format_code: 5,
        sample_data_fidelity: ophiolite_seismic_runtime::segy_sample_data_fidelity(5),
        endianness: "big".to_string(),
        revision_raw: 0,
        fixed_length_trace_flag_raw: 1,
        extended_textual_headers: 0,
        geometry: ophiolite_seismic_runtime::GeometryProvenance {
            inline_field: HeaderFieldSpec {
                name: "INLINE_SYNTHETIC".to_string(),
                start_byte: 189,
                value_type: "I32".to_string(),
            },
            crossline_field: HeaderFieldSpec {
                name: "CROSSLINE_SYNTHETIC".to_string(),
                start_byte: 193,
                value_type: "I32".to_string(),
            },
            third_axis_field: None,
        },
        regularization: None,
    };

    BenchmarkDataset {
        name: format!("synthetic-{ilines}x{xlines}x{samples}"),
        shape,
        data,
        axes,
        source,
        occupancy: None,
    }
}

fn create_bench_root(dataset_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let sanitized = dataset_name
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value
            } else {
                '_'
            }
        })
        .collect::<String>();
    let root = std::env::temp_dir().join(format!(
        "ophiolite-seismic-compute-bench-{sanitized}-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn directory_metrics(root: &Path) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let mut bytes = 0_u64;
    let mut files = 0_u64;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let (child_bytes, child_files) = directory_metrics(&path)?;
            bytes += child_bytes;
            files += child_files;
        } else {
            bytes += entry.metadata()?.len();
            files += 1;
        }
    }
    Ok((bytes, files))
}

fn candidate_label(candidate: StorageCandidate) -> &'static str {
    match candidate {
        StorageCandidate::ZarrUncompressedUnsharded => "zarr_uncompressed_unsharded",
        StorageCandidate::ZarrLz4Unsharded => "zarr_lz4_unsharded",
        StorageCandidate::ZarrZstdUnsharded => "zarr_zstd_unsharded",
        StorageCandidate::ZarrUncompressedSharded => "zarr_uncompressed_sharded",
        StorageCandidate::ZarrLz4Sharded => "zarr_lz4_sharded",
        StorageCandidate::ZarrZstdSharded => "zarr_zstd_sharded",
        StorageCandidate::Tbvol => "tbvol",
        StorageCandidate::FlatBinaryControl => "flat_binary_control",
    }
}

fn compression_label(compression: &CompressionKind) -> &'static str {
    match compression {
        CompressionKind::None => "none",
        CompressionKind::BloscLz4 => "blosc_lz4",
        CompressionKind::Zstd => "zstd",
    }
}

fn div_ceil(lhs: u64, rhs: u64) -> u64 {
    lhs.div_ceil(rhs)
}

fn bytes_to_mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn unix_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn print_dataset_analysis(
    analysis: &DatasetAnalysis,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(analysis)?),
        OutputFormat::Text => {
            println!("Dataset analysis");
            println!("- name: {}", analysis.name);
            println!("- source_kind: {}", analysis.source_kind);
            println!(
                "- shape: {} x {} x {}",
                analysis.shape[0], analysis.shape[1], analysis.shape[2]
            );
            println!(
                "- runtime_store_f32_mib: {:.2}",
                analysis.runtime_store_mib_f32
            );
            for candidate in &analysis.chunk_candidates {
                println!(
                    "- chunk_target={}MiB chunk_shape={:?} chunk_bytes={} total_chunks={}",
                    candidate.chunk_target_mib,
                    candidate.chunk_shape,
                    candidate.chunk_bytes,
                    candidate.total_chunks
                );
                for shard in &candidate.shard_candidates {
                    println!(
                        "  shard_target={}MiB shard_shape={:?} approx_shards={}",
                        shard.shard_target_mib, shard.shard_shape, shard.approx_shard_count
                    );
                }
            }
        }
    }
    Ok(())
}

fn print_benchmark_summary(
    summary: &BenchmarkSummary,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(summary)?),
        OutputFormat::Text => {
            println!("Benchmark summary");
            println!("- dataset: {}", summary.dataset_name);
            println!(
                "- shape: {} x {} x {}",
                summary.shape[0], summary.shape[1], summary.shape[2]
            );
            if summary.chunk_shape != [0, 0, 0] {
                println!("- chunk_shape: {:?}", summary.chunk_shape);
            }
            if let Some(shard_target_mib) = summary.shard_target_mib {
                println!("- shard_target_mib: {shard_target_mib}");
            }
            for result in &summary.results {
                println!("- candidate: {}", candidate_label(result.candidate));
                println!("  chunk_shape: {:?}", result.chunk_shape);
                println!("  shard_target_mib: {:?}", result.shard_target_mib);
                println!("  compression: {}", result.compression);
                println!("  shard_shape: {:?}", result.shard_shape);
                println!("  input_store_bytes: {}", result.input_store_bytes);
                println!("  input_file_count: {}", result.input_file_count);
                println!(
                    "  inline_section_read_ms: {:.3}",
                    result.inline_section_read_ms
                );
                println!(
                    "  xline_section_read_ms: {:.3}",
                    result.xline_section_read_ms
                );
                println!(
                    "  preview_amplitude_scalar_ms: {:.3}",
                    result.preview_amplitude_scalar_ms
                );
                println!(
                    "  preview_trace_rms_normalize_ms: {:.3}",
                    result.preview_trace_rms_normalize_ms
                );
                println!(
                    "  preview_phase_rotation_ms: {:.3}",
                    result.preview_phase_rotation_ms
                );
                println!("  preview_bandpass_ms: {:.3}", result.preview_bandpass_ms);
                println!(
                    "  preview_bandpass_phase_rotation_ms: {:.3}",
                    result.preview_bandpass_phase_rotation_ms
                );
                println!("  preview_pipeline_ms: {:.3}", result.preview_pipeline_ms);
                println!(
                    "  apply_amplitude_scalar_ms: {:.3}",
                    result.apply_amplitude_scalar_ms
                );
                println!(
                    "  apply_trace_rms_normalize_ms: {:.3}",
                    result.apply_trace_rms_normalize_ms
                );
                println!(
                    "  apply_phase_rotation_ms: {:.3}",
                    result.apply_phase_rotation_ms
                );
                println!("  apply_bandpass_ms: {:.3}", result.apply_bandpass_ms);
                println!(
                    "  apply_bandpass_phase_rotation_ms: {:.3}",
                    result.apply_bandpass_phase_rotation_ms
                );
                println!("  apply_pipeline_ms: {:.3}", result.apply_pipeline_ms);
                println!("  pipeline_output_bytes: {}", result.pipeline_output_bytes);
                println!(
                    "  pipeline_output_file_count: {}",
                    result.pipeline_output_file_count
                );
            }
        }
    }
    Ok(())
}

fn print_tbvol_tile_sweep_summary(
    summary: &TbvolTileSweepSummary,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(summary)?),
        OutputFormat::Text => {
            println!("tbvol tile sweep");
            println!("- dataset: {}", summary.dataset_name);
            println!("- source_kind: {}", summary.source_kind);
            println!(
                "- shape: {} x {} x {}",
                summary.shape[0], summary.shape[1], summary.shape[2]
            );
            println!(
                "- best_preview_chunk_target_mib: {}",
                summary.best_preview_chunk_target_mib
            );
            println!(
                "- best_apply_chunk_target_mib: {}",
                summary.best_apply_chunk_target_mib
            );
            println!(
                "- best_io_chunk_target_mib: {}",
                summary.best_io_chunk_target_mib
            );
            println!(
                "- best_balanced_chunk_target_mib: {}",
                summary.best_balanced_chunk_target_mib
            );
            for row in &summary.results {
                println!("- chunk_target_mib: {}", row.chunk_target_mib);
                println!("  chunk_shape: {:?}", row.chunk_shape);
                println!("  tile_bytes: {}", row.tile_bytes);
                println!("  tile_count: {}", row.tile_count);
                println!("  input_store_bytes: {}", row.input_store_bytes);
                println!("  input_file_count: {}", row.input_file_count);
                println!(
                    "  inline_section_read_ms: {:.3}",
                    row.inline_section_read_ms
                );
                println!("  xline_section_read_ms: {:.3}", row.xline_section_read_ms);
                println!("  preview_pipeline_ms: {:.3}", row.preview_pipeline_ms);
                println!("  apply_pipeline_ms: {:.3}", row.apply_pipeline_ms);
                println!("  balanced_score: {:.3}", row.balanced_score);
            }
        }
    }
    Ok(())
}

fn print_text_summary(rows: &[BenchPlanRow]) {
    println!("TraceBoost compute/storage benchmark matrix");
    println!("rows: {}", rows.len());
    println!(
        "- keep Zarr if the best Zarr configuration stays within about 20% of the best overall result on full apply and preview latency"
    );
    for row in rows.iter().take(12) {
        println!(
            "- dataset={:?} candidate={:?} chunk_target_mib={} shard_target_mib={}",
            row.dataset,
            row.candidate,
            row.chunk_target_mib,
            row.shard_target_mib
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string())
        );
    }
    if rows.len() > 12 {
        println!("... {} more rows omitted", rows.len() - 12);
    }
}

#[derive(Debug)]
struct FlatBinaryStore {
    root: PathBuf,
    data_path: PathBuf,
    shape: [usize; 3],
    sample_interval_ms: f32,
    occupancy: Option<Array2<u8>>,
}

impl FlatBinaryStore {
    fn create(
        root: &Path,
        data: &Array3<f32>,
        shape: [usize; 3],
        sample_interval_ms: f32,
        occupancy: Option<&Array2<u8>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if root.exists() {
            fs::remove_dir_all(root)?;
        }
        fs::create_dir_all(root)?;
        let data_path = root.join("amplitude.bin");
        let mut writer = File::create(&data_path)?;
        write_f32_le(
            &mut writer,
            data.as_slice_memory_order()
                .expect("benchmark arrays should be contiguous"),
        )?;
        if let Some(mask) = occupancy {
            let mut mask_writer = File::create(root.join("occupancy.bin"))?;
            mask_writer.write_all(
                mask.as_slice_memory_order()
                    .expect("occupancy arrays should be contiguous"),
            )?;
        }
        fs::write(
            root.join("metadata.json"),
            serde_json::to_vec_pretty(&serde_json::json!({ "shape": shape }))?,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            data_path,
            shape,
            sample_interval_ms,
            occupancy: occupancy.cloned(),
        })
    }

    fn occupancy_row(&self, iline: usize) -> Option<Vec<u8>> {
        self.occupancy.as_ref().map(|mask| {
            (0..self.shape[1])
                .map(|xline| mask[[iline, xline]])
                .collect()
        })
    }

    fn read_inline_section(&self, iline: usize) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let traces = self.shape[1];
        let samples = self.shape[2];
        let floats = traces * samples;
        let byte_offset = iline as u64 * floats as u64 * 4;
        let mut file = File::open(&self.data_path)?;
        file.seek(SeekFrom::Start(byte_offset))?;
        let mut bytes = vec![0_u8; floats * 4];
        file.read_exact(&mut bytes)?;
        bytes_to_f32_vec(&bytes)
    }

    fn read_xline_section(&self, xline: usize) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let ilines = self.shape[0];
        let xlines = self.shape[1];
        let samples = self.shape[2];
        let mut file = File::open(&self.data_path)?;
        let mut section = vec![0.0_f32; ilines * samples];
        let mut row_bytes = vec![0_u8; samples * 4];
        for iline in 0..ilines {
            let float_offset = (iline * xlines + xline) * samples;
            file.seek(SeekFrom::Start(float_offset as u64 * 4))?;
            file.read_exact(&mut row_bytes)?;
            let row = bytes_to_f32_vec(&row_bytes)?;
            let dst_start = iline * samples;
            section[dst_start..dst_start + samples].copy_from_slice(&row);
        }
        Ok(section)
    }

    fn materialize(
        &self,
        output_path: &Path,
        chunk_shape: [usize; 3],
        pipeline: &[ProcessingOperation],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = File::open(&self.data_path)?;
        let mut writer = File::create(output_path)?;
        let block_traces = chunk_shape[0] * chunk_shape[1];
        let block_samples = block_traces * self.shape[2];
        let block_bytes = block_samples * std::mem::size_of::<f32>();
        let mut processed_traces = 0_usize;
        loop {
            let mut bytes = vec![0_u8; block_bytes];
            let read = reader.read(&mut bytes)?;
            if read == 0 {
                break;
            }
            bytes.truncate(read);
            let mut values = bytes_to_f32_vec(&bytes)?;
            if values.is_empty() {
                continue;
            }
            let traces = values.len() / self.shape[2];
            let occupancy = self
                .occupancy
                .as_ref()
                .map(|mask| occupancy_slice(mask, self.shape[1], processed_traces, traces));
            apply_pipeline_to_traces(
                &mut values,
                traces,
                self.shape[2],
                self.sample_interval_ms,
                occupancy.as_deref(),
                pipeline,
            )?;
            write_f32_le(&mut writer, &values)?;
            processed_traces += traces;
        }
        Ok(())
    }
}

fn occupancy_slice(
    mask: &Array2<u8>,
    xlines: usize,
    trace_start: usize,
    trace_count: usize,
) -> Vec<u8> {
    let mut values = Vec::with_capacity(trace_count);
    for flat in trace_start..trace_start + trace_count {
        let iline = flat / xlines;
        let xline = flat % xlines;
        values.push(mask[[iline, xline]]);
    }
    values
}

fn sample_interval_ms(sample_axis_ms: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
    if sample_axis_ms.len() < 2 {
        return Err("sample axis must contain at least two entries".into());
    }
    let step = (sample_axis_ms[1] - sample_axis_ms[0]).abs();
    if !step.is_finite() || step <= 0.0 {
        return Err(format!("invalid sample axis step: {step}").into());
    }
    Ok(step)
}

fn benchmark_bandpass_operation(sample_interval_ms: f32) -> ProcessingOperation {
    let nyquist_hz = 500.0 / sample_interval_ms.max(f32::EPSILON);
    let f1_hz = (nyquist_hz * 0.06).max(4.0);
    let f2_hz = (nyquist_hz * 0.1).max(f1_hz + 1.0);
    let f4_hz = (nyquist_hz * 0.45).max(f2_hz + 6.0).min(nyquist_hz);
    let f3_hz = (nyquist_hz * 0.32).max(f2_hz + 4.0).min(f4_hz);
    ProcessingOperation::BandpassFilter {
        f1_hz,
        f2_hz,
        f3_hz,
        f4_hz,
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn benchmark_phase_rotation_operation() -> ProcessingOperation {
    ProcessingOperation::PhaseRotation {
        angle_degrees: 35.0,
    }
}

fn write_f32_le(writer: &mut File, values: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    writer.write_all(&bytes)?;
    Ok(())
}

fn bytes_to_f32_vec(bytes: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if bytes.len() % 4 != 0 {
        return Err(format!("invalid f32 byte length: {}", bytes.len()).into());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}
