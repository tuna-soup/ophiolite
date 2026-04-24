use super::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use seis_runtime::{
    InterpMethod, MaterializeOptions, ProcessingOperation, TraceLocalProcessingStep,
    UpscaleOptions, materialize_processing_volume, trace_local_pipeline_prefix,
    trace_local_pipeline_segment, upscale_store,
};

const BENCH_ITERATIONS: usize = 3;

#[derive(Debug)]
struct BenchmarkCaseResult {
    name: String,
    iterations_ms: Vec<f64>,
    median_ms: f64,
}

#[derive(Debug)]
struct BenchmarkDataset {
    name: String,
    store_path: PathBuf,
    shape: [usize; 3],
}

fn bench_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("canonicalize CARGO_MANIFEST_DIR");
    for ancestor in manifest_dir.ancestors() {
        if ancestor.join("test-data").is_dir() && ancestor.join("runtime").is_dir() {
            return ancestor.to_path_buf();
        }
    }
    panic!("unable to locate Ophiolite workspace root from CARGO_MANIFEST_DIR");
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("traceboost-cache-bench-{label}-{stamp}"));
    fs::create_dir_all(&path).expect("create benchmark temp dir");
    path
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap());
    sorted[sorted.len() / 2]
}

fn benchmark_case<F>(name: &str, mut run: F) -> BenchmarkCaseResult
where
    F: FnMut(usize) -> Result<(), String>,
{
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for iteration in 0..BENCH_ITERATIONS {
        let start = Instant::now();
        run(iteration)
            .unwrap_or_else(|error| panic!("{name} failed on iteration {iteration}: {error}"));
        iterations_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    BenchmarkCaseResult {
        name: name.to_string(),
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn steps(operations: Vec<ProcessingOperation>) -> Vec<TraceLocalProcessingStep> {
    operations
        .into_iter()
        .map(|operation| TraceLocalProcessingStep {
            operation,
            checkpoint: false,
        })
        .collect()
}

fn benchmark_case_with_setup<S, Setup, Run>(
    name: &str,
    mut setup: Setup,
    mut run: Run,
) -> BenchmarkCaseResult
where
    Setup: FnMut(usize) -> Result<S, String>,
    Run: FnMut(S) -> Result<(), String>,
{
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for iteration in 0..BENCH_ITERATIONS {
        let state = setup(iteration).unwrap_or_else(|error| {
            panic!("{name} setup failed on iteration {iteration}: {error}")
        });
        let start = Instant::now();
        run(state)
            .unwrap_or_else(|error| panic!("{name} failed on iteration {iteration}: {error}"));
        iterations_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    BenchmarkCaseResult {
        name: name.to_string(),
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn make_pipeline(variant: &str) -> TraceLocalProcessingPipeline {
    match variant {
        "five-step-a" => TraceLocalProcessingPipeline {
            schema_version: 2,
            revision: 1,
            preset_id: None,
            name: Some("Five Step A".to_string()),
            description: None,
            steps: steps(vec![
                ProcessingOperation::HighpassFilter {
                    f1_hz: 4.0,
                    f2_hz: 8.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 180.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 35.0,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 8.0,
                    f2_hz: 12.0,
                    f3_hz: 45.0,
                    f4_hz: 60.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AmplitudeScalar { factor: 1.15 },
            ]),
        },
        "five-step-b" => TraceLocalProcessingPipeline {
            steps: steps(vec![
                ProcessingOperation::HighpassFilter {
                    f1_hz: 4.0,
                    f2_hz: 8.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 180.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 35.0,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 8.0,
                    f2_hz: 12.0,
                    f3_hz: 45.0,
                    f4_hz: 60.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AmplitudeScalar { factor: 0.85 },
            ]),
            ..make_pipeline("five-step-a")
        },
        "eight-step-a" => TraceLocalProcessingPipeline {
            schema_version: 2,
            revision: 1,
            preset_id: None,
            name: Some("Eight Step A".to_string()),
            description: None,
            steps: steps(vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.05 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 3.0,
                    f2_hz: 6.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 120.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 20.0,
                },
                ProcessingOperation::LowpassFilter {
                    f3_hz: 38.0,
                    f4_hz: 52.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 6.0,
                    f2_hz: 10.0,
                    f3_hz: 40.0,
                    f4_hz: 58.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: -15.0,
                },
                ProcessingOperation::AmplitudeScalar { factor: 1.20 },
            ]),
        },
        "eight-step-b" => TraceLocalProcessingPipeline {
            steps: steps(vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.05 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 3.0,
                    f2_hz: 6.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 120.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 20.0,
                },
                ProcessingOperation::LowpassFilter {
                    f3_hz: 38.0,
                    f4_hz: 52.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 6.0,
                    f2_hz: 10.0,
                    f3_hz: 40.0,
                    f4_hz: 58.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: -15.0,
                },
                ProcessingOperation::AmplitudeScalar { factor: 0.70 },
            ]),
            ..make_pipeline("eight-step-a")
        },
        "ten-step-a" => TraceLocalProcessingPipeline {
            schema_version: 2,
            revision: 1,
            preset_id: None,
            name: Some("Ten Step A".to_string()),
            description: None,
            steps: steps(vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.05 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 3.0,
                    f2_hz: 6.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 120.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 20.0,
                },
                ProcessingOperation::LowpassFilter {
                    f3_hz: 36.0,
                    f4_hz: 48.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 6.0,
                    f2_hz: 10.0,
                    f3_hz: 40.0,
                    f4_hz: 58.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 180.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: -15.0,
                },
                ProcessingOperation::LowpassFilter {
                    f3_hz: 32.0,
                    f4_hz: 44.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AmplitudeScalar { factor: 1.20 },
            ]),
        },
        "ten-step-middle-edit" => TraceLocalProcessingPipeline {
            steps: steps(vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.05 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 3.0,
                    f2_hz: 6.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 120.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 20.0,
                },
                ProcessingOperation::LowpassFilter {
                    f3_hz: 28.0,
                    f4_hz: 42.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 6.0,
                    f2_hz: 10.0,
                    f3_hz: 40.0,
                    f4_hz: 58.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 180.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: -15.0,
                },
                ProcessingOperation::LowpassFilter {
                    f3_hz: 32.0,
                    f4_hz: 44.0,
                    phase: seis_runtime::FrequencyPhaseMode::Zero,
                    window: seis_runtime::FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AmplitudeScalar { factor: 1.20 },
            ]),
            ..make_pipeline("ten-step-a")
        },
        _ => panic!("unknown benchmark pipeline variant: {variant}"),
    }
}

fn run_full_pipeline(
    input_store: &str,
    output_store: &str,
    pipeline: &TraceLocalProcessingPipeline,
) -> Result<(), String> {
    let chunk_shape = open_store(input_store)
        .map_err(|error| format!("{error:?}"))?
        .manifest
        .tile_shape;
    materialize_processing_volume(
        input_store,
        output_store,
        pipeline,
        MaterializeOptions {
            chunk_shape,
            ..MaterializeOptions::default()
        },
    )
    .map(|_| ())
    .map_err(|error| format!("{error:?}"))
}

fn run_balanced_hidden_prefix(
    processing_cache: &ProcessingCacheState,
    input_store: &str,
    output_store: &str,
    pipeline: &TraceLocalProcessingPipeline,
) -> Result<(), String> {
    let source_fingerprint = trace_local_source_fingerprint(input_store)?;
    let prefix_index = pipeline.operation_count().checked_sub(2).ok_or_else(|| {
        "Balanced hidden-prefix benchmark requires at least two pipeline operations.".to_string()
    })?;
    let prefix_pipeline = trace_local_pipeline_prefix(pipeline, prefix_index);
    let prefix_hash = trace_local_pipeline_hash(&prefix_pipeline)?;
    let prefix_len = prefix_index + 1;

    let prefix_store = if let Some(hit) = processing_cache.lookup_any_prefix_artifact(
        TRACE_LOCAL_CACHE_FAMILY,
        &source_fingerprint,
        &prefix_hash,
        prefix_len,
        PROCESSING_CACHE_RUNTIME_VERSION,
        PROCESSING_CACHE_STORE_WRITER_VERSION,
        TBVOL_STORE_FORMAT_VERSION,
    )? {
        hit.path
    } else {
        let store = benchmark_hidden_prefix_output_store_path(
            processing_cache,
            &source_fingerprint,
            &prefix_hash,
            prefix_len,
        );
        prepare_processing_output_store(input_store, &store, true)?;
        run_full_pipeline(input_store, &store, &prefix_pipeline)?;
        let _ = processing_cache.register_hidden_prefix(
            TRACE_LOCAL_CACHE_FAMILY,
            &store,
            &source_fingerprint,
            &prefix_hash,
            prefix_len,
            PROCESSING_CACHE_RUNTIME_VERSION,
            TBVOL_STORE_FORMAT_VERSION,
        )?;
        store
    };

    prepare_processing_output_store(&prefix_store, output_store, true)?;
    run_full_pipeline(
        &prefix_store,
        output_store,
        &trace_local_pipeline_segment(pipeline, prefix_len, pipeline.operation_count() - 1),
    )?;
    Ok(())
}

fn benchmark_hidden_prefix_output_store_path(
    processing_cache: &ProcessingCacheState,
    source_fingerprint: &str,
    prefix_hash: &str,
    prefix_len: usize,
) -> String {
    processing_cache
        .volumes_dir()
        .join(format!(
            "trace-local-{source_fingerprint}-prefix-{prefix_len:02}-{prefix_hash}.tbvol"
        ))
        .display()
        .to_string()
}

fn run_visible_checkpoint_pipeline(
    processing_cache: &ProcessingCacheState,
    input_store: &str,
    output_store: &str,
    pipeline: &TraceLocalProcessingPipeline,
    checkpoint_index: usize,
    reuse_prefix: bool,
) -> Result<(), String> {
    let source_fingerprint = trace_local_source_fingerprint(input_store)?;
    let prefix_pipeline = trace_local_pipeline_prefix(pipeline, checkpoint_index);
    let prefix_hash = trace_local_pipeline_hash(&prefix_pipeline)?;
    let prefix_len = checkpoint_index + 1;

    let checkpoint_store = if reuse_prefix {
        if let Some(hit) = processing_cache.lookup_prefix_artifact(
            TRACE_LOCAL_CACHE_FAMILY,
            &source_fingerprint,
            &prefix_hash,
            prefix_len,
            PROCESSING_CACHE_RUNTIME_VERSION,
            PROCESSING_CACHE_STORE_WRITER_VERSION,
            TBVOL_STORE_FORMAT_VERSION,
        )? {
            hit.path
        } else {
            return Err("Expected visible checkpoint cache hit but none was found.".to_string());
        }
    } else {
        let store = output_store.replace(".tbvol", ".checkpoint.tbvol");
        prepare_processing_output_store(input_store, &store, true)?;
        run_full_pipeline(input_store, &store, &prefix_pipeline)?;
        let _ = processing_cache.register_visible_checkpoint(
            TRACE_LOCAL_CACHE_FAMILY,
            &store,
            &source_fingerprint,
            &prefix_hash,
            prefix_len,
            PROCESSING_CACHE_RUNTIME_VERSION,
            TBVOL_STORE_FORMAT_VERSION,
        )?;
        store
    };

    prepare_processing_output_store(&checkpoint_store, output_store, true)?;
    run_full_pipeline(
        &checkpoint_store,
        output_store,
        &trace_local_pipeline_segment(pipeline, prefix_len, pipeline.operation_count() - 1),
    )?;
    Ok(())
}

fn describe_dataset(store_path: &Path) -> BenchmarkDataset {
    let handle = open_store(store_path).expect("open benchmark dataset");
    BenchmarkDataset {
        name: store_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dataset")
            .to_string(),
        store_path: store_path.to_path_buf(),
        shape: handle.manifest.volume.shape,
    }
}

fn ensure_upscaled_dataset(
    input_store: &Path,
    workspace: &Path,
    levels: usize,
) -> Result<BenchmarkDataset, String> {
    let mut current = input_store.to_path_buf();
    for level in 0..levels {
        let next = workspace.join(format!("f3-upscaled-level-{}.tbvol", level + 1));
        if !next.exists() {
            upscale_store(
                &current,
                &next,
                UpscaleOptions {
                    scale: 2,
                    method: InterpMethod::Linear,
                    chunk_shape: [0, 0, 0],
                },
            )
            .map_err(|error| error.to_string())?;
        }
        current = next;
    }
    Ok(describe_dataset(&current))
}

fn print_results(dataset: &BenchmarkDataset, results: &[BenchmarkCaseResult]) {
    println!();
    println!(
        "Processing Cache Benchmark Results: {} [{} x {} x {}]",
        dataset.name, dataset.shape[0], dataset.shape[1], dataset.shape[2]
    );
    println!("| Case | Iterations (ms) | Median (ms) |");
    println!("| --- | --- | ---: |");
    for result in results {
        let iterations = result
            .iterations_ms
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "| {} | {} | {:.3} |",
            result.name, iterations, result.median_ms
        );
    }
}

fn benchmark_dataset(dataset: &BenchmarkDataset, workspace: &Path) -> Vec<BenchmarkCaseResult> {
    fs::create_dir_all(workspace).expect("create benchmark workspace");
    let input_store = dataset.store_path.display().to_string();
    let five_a = make_pipeline("five-step-a");
    let five_b = make_pipeline("five-step-b");
    let eight_a = make_pipeline("eight-step-a");
    let eight_b = make_pipeline("eight-step-b");
    let ten_a = make_pipeline("ten-step-a");
    let ten_middle = make_pipeline("ten-step-middle-edit");

    let dataset_slug = sanitized_stem(&dataset.name, "dataset");

    let five_baseline = benchmark_case("five-step uncached full rerun", |iteration| {
        let output = workspace.join(format!("{dataset_slug}-five-baseline-{iteration}.tbvol"));
        prepare_processing_output_store(&input_store, &output.display().to_string(), true)?;
        run_full_pipeline(&input_store, &output.display().to_string(), &five_b)
    });

    let five_hidden_prefix_contrast_first =
        benchmark_case("five-step hidden-prefix contrast first run", |iteration| {
            let iteration_root =
                workspace.join(format!("{dataset_slug}-five-balanced-first-{iteration}"));
            fs::create_dir_all(&iteration_root).map_err(|error| error.to_string())?;
            let cache = ProcessingCacheState::initialize(
                &iteration_root.join("cache"),
                &iteration_root.join("cache").join("volumes"),
                &iteration_root.join("cache").join("index.sqlite"),
                &iteration_root.join("settings.json"),
            )?;
            let output = iteration_root.join("result.tbvol");
            run_balanced_hidden_prefix(&cache, &input_store, &output.display().to_string(), &five_a)
        });

    let five_hidden_prefix_contrast_late_edit = benchmark_case_with_setup(
        "five-step hidden-prefix contrast late edit rerun",
        |iteration| {
            let iteration_root =
                workspace.join(format!("{dataset_slug}-five-balanced-late-{iteration}"));
            fs::create_dir_all(&iteration_root).map_err(|error| error.to_string())?;
            let cache = ProcessingCacheState::initialize(
                &iteration_root.join("cache"),
                &iteration_root.join("cache").join("volumes"),
                &iteration_root.join("cache").join("index.sqlite"),
                &iteration_root.join("settings.json"),
            )?;
            let first_output = iteration_root.join("first.tbvol");
            run_balanced_hidden_prefix(
                &cache,
                &input_store,
                &first_output.display().to_string(),
                &five_a,
            )?;
            Ok((cache, iteration_root.join("rerun.tbvol")))
        },
        |(cache, rerun_output)| {
            run_balanced_hidden_prefix(
                &cache,
                &input_store,
                &rerun_output.display().to_string(),
                &five_b,
            )
        },
    );

    let five_checkpoint_late_edit = benchmark_case_with_setup(
        "five-step explicit checkpoint late edit rerun",
        |iteration| {
            let iteration_root =
                workspace.join(format!("{dataset_slug}-five-checkpoint-late-{iteration}"));
            fs::create_dir_all(&iteration_root).map_err(|error| error.to_string())?;
            let cache = ProcessingCacheState::initialize(
                &iteration_root.join("cache"),
                &iteration_root.join("cache").join("volumes"),
                &iteration_root.join("cache").join("index.sqlite"),
                &iteration_root.join("settings.json"),
            )?;
            let first_output = iteration_root.join("first.tbvol");
            run_visible_checkpoint_pipeline(
                &cache,
                &input_store,
                &first_output.display().to_string(),
                &five_a,
                3,
                false,
            )?;
            Ok((cache, iteration_root.join("rerun.tbvol")))
        },
        |(cache, rerun_output)| {
            run_visible_checkpoint_pipeline(
                &cache,
                &input_store,
                &rerun_output.display().to_string(),
                &five_b,
                3,
                true,
            )
        },
    );

    let eight_baseline = benchmark_case("eight-step uncached full rerun", |iteration| {
        let output = workspace.join(format!("{dataset_slug}-eight-baseline-{iteration}.tbvol"));
        prepare_processing_output_store(&input_store, &output.display().to_string(), true)?;
        run_full_pipeline(&input_store, &output.display().to_string(), &eight_b)
    });

    let eight_hidden_prefix_contrast_late_edit = benchmark_case_with_setup(
        "eight-step hidden-prefix contrast late edit rerun",
        |iteration| {
            let iteration_root =
                workspace.join(format!("{dataset_slug}-eight-balanced-late-{iteration}"));
            fs::create_dir_all(&iteration_root).map_err(|error| error.to_string())?;
            let cache = ProcessingCacheState::initialize(
                &iteration_root.join("cache"),
                &iteration_root.join("cache").join("volumes"),
                &iteration_root.join("cache").join("index.sqlite"),
                &iteration_root.join("settings.json"),
            )?;
            let first_output = iteration_root.join("first.tbvol");
            run_balanced_hidden_prefix(
                &cache,
                &input_store,
                &first_output.display().to_string(),
                &eight_a,
            )?;
            Ok((cache, iteration_root.join("rerun.tbvol")))
        },
        |(cache, rerun_output)| {
            run_balanced_hidden_prefix(
                &cache,
                &input_store,
                &rerun_output.display().to_string(),
                &eight_b,
            )
        },
    );

    let ten_baseline = benchmark_case("ten-step uncached middle-edit rerun", |iteration| {
        let output = workspace.join(format!("{dataset_slug}-ten-baseline-{iteration}.tbvol"));
        prepare_processing_output_store(&input_store, &output.display().to_string(), true)?;
        run_full_pipeline(&input_store, &output.display().to_string(), &ten_middle)
    });

    let ten_hidden_prefix_contrast_middle_edit = benchmark_case_with_setup(
        "ten-step hidden-prefix contrast middle-edit rerun",
        |iteration| {
            let iteration_root =
                workspace.join(format!("{dataset_slug}-ten-balanced-middle-{iteration}"));
            fs::create_dir_all(&iteration_root).map_err(|error| error.to_string())?;
            let cache = ProcessingCacheState::initialize(
                &iteration_root.join("cache"),
                &iteration_root.join("cache").join("volumes"),
                &iteration_root.join("cache").join("index.sqlite"),
                &iteration_root.join("settings.json"),
            )?;
            let first_output = iteration_root.join("first.tbvol");
            run_balanced_hidden_prefix(
                &cache,
                &input_store,
                &first_output.display().to_string(),
                &ten_a,
            )?;
            Ok((cache, iteration_root.join("rerun.tbvol")))
        },
        |(cache, rerun_output)| {
            run_balanced_hidden_prefix(
                &cache,
                &input_store,
                &rerun_output.display().to_string(),
                &ten_middle,
            )
        },
    );

    let exact_lookup = benchmark_case_with_setup(
        "exact output lookup",
        |iteration| {
            let iteration_root = workspace.join(format!("{dataset_slug}-exact-hit-{iteration}"));
            fs::create_dir_all(&iteration_root).map_err(|error| error.to_string())?;
            let cache = ProcessingCacheState::initialize(
                &iteration_root.join("cache"),
                &iteration_root.join("cache").join("volumes"),
                &iteration_root.join("cache").join("index.sqlite"),
                &iteration_root.join("settings.json"),
            )?;
            let output = iteration_root.join("exact.tbvol");
            run_full_pipeline(&input_store, &output.display().to_string(), &five_a)?;
            let source_fingerprint = trace_local_source_fingerprint(&input_store)?;
            let pipeline_hash = trace_local_pipeline_hash(&five_a)?;
            let _ = cache.register_visible_output(
                TRACE_LOCAL_CACHE_FAMILY,
                &output.display().to_string(),
                &source_fingerprint,
                &pipeline_hash,
                &pipeline_hash,
                five_a.operation_count(),
                PROCESSING_CACHE_RUNTIME_VERSION,
                TBVOL_STORE_FORMAT_VERSION,
            )?;
            Ok((cache, source_fingerprint, pipeline_hash))
        },
        |(cache, source_fingerprint, pipeline_hash)| {
            cache
                .lookup_exact_visible_output(
                    TRACE_LOCAL_CACHE_FAMILY,
                    &source_fingerprint,
                    &pipeline_hash,
                    PROCESSING_CACHE_RUNTIME_VERSION,
                    PROCESSING_CACHE_STORE_WRITER_VERSION,
                    TBVOL_STORE_FORMAT_VERSION,
                )?
                .ok_or_else(|| "expected exact lookup hit".to_string())?;
            Ok(())
        },
    );

    vec![
        five_baseline,
        five_hidden_prefix_contrast_first,
        five_hidden_prefix_contrast_late_edit,
        five_checkpoint_late_edit,
        eight_baseline,
        eight_hidden_prefix_contrast_late_edit,
        ten_baseline,
        ten_hidden_prefix_contrast_middle_edit,
        exact_lookup,
    ]
}

#[test]
#[ignore]
fn benchmark_processing_cache_f3() {
    let repo_root = bench_root();
    let input_store = repo_root.join("test-data").join("f3.tbvol");
    assert!(
        input_store.is_dir(),
        "Expected test-data/f3.tbvol to exist for processing-cache benchmark."
    );

    let workspace = unique_temp_dir("f3");
    let f3 = describe_dataset(&input_store);
    let f3_upscaled = ensure_upscaled_dataset(&input_store, &workspace, 3)
        .expect("create upscaled benchmark dataset");

    let f3_results = benchmark_dataset(&f3, &unique_temp_dir("f3-results"));
    let upscaled_results = benchmark_dataset(&f3_upscaled, &unique_temp_dir("f3-upscaled-results"));

    print_results(&f3, &f3_results);
    print_results(&f3_upscaled, &upscaled_results);
}

#[test]
#[ignore]
fn benchmark_processing_cache_large_f3_smoke() {
    let input_store = std::env::temp_dir().join("f3_dataset-smoke.tbvol");
    assert!(
        input_store.is_dir(),
        "Expected %TEMP%/f3_dataset-smoke.tbvol to exist for processing-cache benchmark."
    );

    let dataset = describe_dataset(&input_store);
    let results = benchmark_dataset(&dataset, &unique_temp_dir("f3-large-results"));

    print_results(&dataset, &results);
}
