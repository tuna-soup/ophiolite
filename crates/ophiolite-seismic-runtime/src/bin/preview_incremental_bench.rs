use clap::{Parser, ValueEnum};
use ophiolite_seismic_runtime::{
    FrequencyPhaseMode, FrequencyWindowShape, PreviewSectionPrefixCache, PreviewSectionSession,
    ProcessingOperation, ProcessingSampleDependency, ProcessingSpatialDependency, SectionAxis,
    SectionPlane, StoreHandle, TbvolReader, apply_pipeline_to_plane, assemble_section_plane,
    open_store, preview_section_view, preview_section_view_with_prefix_cache,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_ITERATIONS: usize = 3;
const NUMERICAL_TOLERANCE: f32 = 1.0e-5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AxisSelection {
    Inline,
    Xline,
    Both,
}

#[derive(Debug, Parser)]
#[command(name = "preview-incremental-bench")]
#[command(about = "Benchmark section preview reruns versus in-memory unchanged-prefix reuse")]
struct Cli {
    #[arg(long)]
    store: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = AxisSelection::Both)]
    axis: AxisSelection,
    #[arg(long)]
    inline_index: Option<usize>,
    #[arg(long)]
    xline_index: Option<usize>,
    #[arg(long, default_value_t = DEFAULT_ITERATIONS)]
    iterations: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug)]
struct AxisPlan {
    axis: SectionAxis,
    index: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct BenchmarkReport {
    dataset: DatasetSummary,
    iterations: usize,
    cases: Vec<BenchmarkCase>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DatasetSummary {
    store_path: String,
    shape: [usize; 3],
    tile_shape: [usize; 3],
}

#[derive(Debug, Clone, serde::Serialize)]
struct BenchmarkCase {
    axis: String,
    section_index: usize,
    scenario: String,
    strategy: String,
    prefix_operations: usize,
    suffix_operations: usize,
    prefix_dependency: String,
    suffix_dependency: String,
    total_ms_iterations: Vec<f64>,
    total_ms_median: f64,
    phase_a_ms_iterations: Vec<f64>,
    phase_a_ms_median: f64,
    phase_b_ms_iterations: Vec<f64>,
    phase_b_ms_median: f64,
    phase_c_ms_iterations: Vec<f64>,
    phase_c_ms_median: f64,
    setup_ms: Option<f64>,
}

#[derive(Debug, Clone)]
struct Scenario {
    name: &'static str,
    warmup_pipeline: Vec<ProcessingOperation>,
    full_pipeline: Vec<ProcessingOperation>,
    prefix_len: usize,
}

#[derive(Debug)]
struct IterationBreakdown {
    total_ms: f64,
    phase_a_ms: f64,
    phase_b_ms: f64,
    phase_c_ms: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = resolve_store_path(cli.store)?;
    let handle = open_store(&store)?;
    let reader = TbvolReader::open(&handle.root)?;
    let axes = planned_axes(&handle, cli.axis, cli.inline_index, cli.xline_index);
    let scenarios = benchmark_scenarios();
    let mut cases = Vec::new();

    for axis_plan in axes {
        for scenario in &scenarios {
            cases.push(run_baseline_case(
                &handle,
                &reader,
                axis_plan.axis,
                axis_plan.index,
                scenario,
                cli.iterations,
            )?);
            cases.push(run_ephemeral_reuse_case(
                &handle,
                &reader,
                axis_plan.axis,
                axis_plan.index,
                scenario,
                cli.iterations,
            )?);
            cases.push(run_runtime_baseline_case(
                &store,
                axis_plan.axis,
                axis_plan.index,
                scenario,
                cli.iterations,
            )?);
            cases.push(run_runtime_cache_case(
                &store,
                axis_plan.axis,
                axis_plan.index,
                scenario,
                cli.iterations,
            )?);
            cases.push(run_pinned_session_baseline_case(
                &store,
                axis_plan.axis,
                axis_plan.index,
                scenario,
                cli.iterations,
            )?);
            cases.push(run_pinned_session_cache_case(
                &store,
                axis_plan.axis,
                axis_plan.index,
                scenario,
                cli.iterations,
            )?);
        }
    }

    let report = BenchmarkReport {
        dataset: DatasetSummary {
            store_path: store.display().to_string(),
            shape: handle.manifest.volume.shape,
            tile_shape: handle.manifest.tile_shape,
        },
        iterations: cli.iterations,
        cases,
    };

    match cli.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Text => print_report(&report),
    }

    Ok(())
}

fn resolve_store_path(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        if path.is_dir() {
            return Ok(path);
        }
        return Err(format!(
            "preview benchmark store not found: {}",
            path.display()
        ));
    }

    let candidates = [
        std::env::temp_dir().join("f3_dataset-smoke.tbvol"),
        PathBuf::from(r"H:\traceboost-bench\f3_dataset-smoke.tbvol"),
    ];
    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }
    Err("preview benchmark store not found; pass --store <tbvol path>".to_string())
}

fn planned_axes(
    handle: &StoreHandle,
    selection: AxisSelection,
    inline_index: Option<usize>,
    xline_index: Option<usize>,
) -> Vec<AxisPlan> {
    let mid_inline = handle.manifest.volume.shape[0] / 2;
    let mid_xline = handle.manifest.volume.shape[1] / 2;
    match selection {
        AxisSelection::Inline => vec![AxisPlan {
            axis: SectionAxis::Inline,
            index: inline_index.unwrap_or(mid_inline),
        }],
        AxisSelection::Xline => vec![AxisPlan {
            axis: SectionAxis::Xline,
            index: xline_index.unwrap_or(mid_xline),
        }],
        AxisSelection::Both => vec![
            AxisPlan {
                axis: SectionAxis::Inline,
                index: inline_index.unwrap_or(mid_inline),
            },
            AxisPlan {
                axis: SectionAxis::Xline,
                index: xline_index.unwrap_or(mid_xline),
            },
        ],
    }
}

fn benchmark_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "late_scalar_edit",
            warmup_pipeline: vec![
                ProcessingOperation::HighpassFilter {
                    f1_hz: 4.0,
                    f2_hz: 8.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
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
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AmplitudeScalar { factor: 1.15 },
            ],
            full_pipeline: vec![
                ProcessingOperation::HighpassFilter {
                    f1_hz: 4.0,
                    f2_hz: 8.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
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
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AmplitudeScalar { factor: 0.85 },
            ],
            prefix_len: 4,
        },
        Scenario {
            name: "late_filter_edit",
            warmup_pipeline: vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.05 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 3.0,
                    f2_hz: 6.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 150.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 20.0,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 9.0,
                    f2_hz: 13.0,
                    f3_hz: 40.0,
                    f4_hz: 55.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
            ],
            full_pipeline: vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.05 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 3.0,
                    f2_hz: 6.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::AgcRms { window_ms: 150.0 },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 20.0,
                },
                ProcessingOperation::BandpassFilter {
                    f1_hz: 7.0,
                    f2_hz: 11.0,
                    f3_hz: 42.0,
                    f4_hz: 58.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
            ],
            prefix_len: 4,
        },
        Scenario {
            name: "late_agc_edit",
            warmup_pipeline: vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.02 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 4.0,
                    f2_hz: 7.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 15.0,
                },
                ProcessingOperation::AgcRms { window_ms: 180.0 },
                ProcessingOperation::AmplitudeScalar { factor: 0.92 },
            ],
            full_pipeline: vec![
                ProcessingOperation::AmplitudeScalar { factor: 1.02 },
                ProcessingOperation::HighpassFilter {
                    f1_hz: 4.0,
                    f2_hz: 7.0,
                    phase: FrequencyPhaseMode::Zero,
                    window: FrequencyWindowShape::CosineTaper,
                },
                ProcessingOperation::PhaseRotation {
                    angle_degrees: 15.0,
                },
                ProcessingOperation::AgcRms { window_ms: 220.0 },
                ProcessingOperation::AmplitudeScalar { factor: 0.92 },
            ],
            prefix_len: 3,
        },
    ]
}

fn run_baseline_case(
    handle: &StoreHandle,
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
    iterations: usize,
) -> Result<BenchmarkCase, String> {
    let mut totals = Vec::with_capacity(iterations);
    let mut fetches = Vec::with_capacity(iterations);
    let mut applies = Vec::with_capacity(iterations);
    let mut views = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let breakdown = baseline_iteration(handle, reader, axis, index, &scenario.full_pipeline)?;
        totals.push(breakdown.total_ms);
        fetches.push(breakdown.phase_a_ms);
        applies.push(breakdown.phase_b_ms);
        views.push(breakdown.phase_c_ms);
    }

    Ok(BenchmarkCase {
        axis: axis_label(axis).to_string(),
        section_index: index,
        scenario: scenario.name.to_string(),
        strategy: "fused_selective_recompute".to_string(),
        prefix_operations: scenario.prefix_len,
        suffix_operations: scenario
            .full_pipeline
            .len()
            .saturating_sub(scenario.prefix_len),
        prefix_dependency: summarize_dependency(&scenario.full_pipeline[..scenario.prefix_len]),
        suffix_dependency: summarize_dependency(&scenario.full_pipeline[scenario.prefix_len..]),
        total_ms_median: median(&totals),
        total_ms_iterations: totals,
        phase_a_ms_median: median(&fetches),
        phase_a_ms_iterations: fetches,
        phase_b_ms_median: median(&applies),
        phase_b_ms_iterations: applies,
        phase_c_ms_median: median(&views),
        phase_c_ms_iterations: views,
        setup_ms: None,
    })
}

fn run_ephemeral_reuse_case(
    handle: &StoreHandle,
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
    iterations: usize,
) -> Result<BenchmarkCase, String> {
    let prefix = &scenario.full_pipeline[..scenario.prefix_len];
    let suffix = &scenario.full_pipeline[scenario.prefix_len..];

    let setup_started = Instant::now();
    let mut prefix_plane =
        assemble_section_plane(reader, axis, index).map_err(|e| e.to_string())?;
    apply_pipeline_to_plane(&mut prefix_plane, prefix).map_err(|e| e.to_string())?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;

    let baseline = fully_processed_plane(reader, axis, index, &scenario.full_pipeline)?;
    let mut totals = Vec::with_capacity(iterations);
    let mut clones = Vec::with_capacity(iterations);
    let mut suffix_applies = Vec::with_capacity(iterations);
    let mut views = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let breakdown = ephemeral_iteration(handle, &prefix_plane, suffix, &baseline)
            .map_err(|e| e.to_string())?;
        totals.push(breakdown.total_ms);
        clones.push(breakdown.phase_a_ms);
        suffix_applies.push(breakdown.phase_b_ms);
        views.push(breakdown.phase_c_ms);
    }

    Ok(BenchmarkCase {
        axis: axis_label(axis).to_string(),
        section_index: index,
        scenario: scenario.name.to_string(),
        strategy: "ideal_ephemeral_prefix_reuse".to_string(),
        prefix_operations: scenario.prefix_len,
        suffix_operations: suffix.len(),
        prefix_dependency: summarize_dependency(prefix),
        suffix_dependency: summarize_dependency(suffix),
        total_ms_median: median(&totals),
        total_ms_iterations: totals,
        phase_a_ms_median: median(&clones),
        phase_a_ms_iterations: clones,
        phase_b_ms_median: median(&suffix_applies),
        phase_b_ms_iterations: suffix_applies,
        phase_c_ms_median: median(&views),
        phase_c_ms_iterations: views,
        setup_ms: Some(setup_ms),
    })
}

fn run_runtime_cache_case(
    store_root: &Path,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
    iterations: usize,
) -> Result<BenchmarkCase, String> {
    let mut setups = Vec::with_capacity(iterations);
    let mut totals = Vec::with_capacity(iterations);
    let mut phase_as = Vec::with_capacity(iterations);
    let mut phase_bs = Vec::with_capacity(iterations);
    let mut phase_cs = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let mut cache = PreviewSectionPrefixCache::default();

        let setup_started = Instant::now();
        let (_, warmup_reuse) = preview_section_view_with_prefix_cache(
            store_root,
            axis,
            index,
            &scenario.warmup_pipeline,
            &mut cache,
        )
        .map_err(|e| e.to_string())?;
        let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;
        if warmup_reuse.cache_hit {
            return Err("warmup run unexpectedly hit the runtime prefix cache".to_string());
        }
        setups.push(setup_ms);

        let total_started = Instant::now();
        let (view, reuse) = preview_section_view_with_prefix_cache(
            store_root,
            axis,
            index,
            &scenario.full_pipeline,
            &mut cache,
        )
        .map_err(|e| e.to_string())?;
        let total_ms = total_started.elapsed().as_secs_f64() * 1000.0;
        if !reuse.cache_hit || reuse.reused_prefix_operations < scenario.prefix_len {
            return Err(format!(
                "expected runtime cache hit with at least {} reused operations, found hit={} prefix_ops={}",
                scenario.prefix_len, reuse.cache_hit, reuse.reused_prefix_operations
            ));
        }

        let view_started = Instant::now();
        std::hint::black_box(view);
        let view_ms = view_started.elapsed().as_secs_f64() * 1000.0;

        totals.push(total_ms);
        phase_as.push(0.0);
        phase_bs.push((total_ms - view_ms).max(0.0));
        phase_cs.push(view_ms);
    }

    Ok(BenchmarkCase {
        axis: axis_label(axis).to_string(),
        section_index: index,
        scenario: scenario.name.to_string(),
        strategy: "runtime_session_prefix_cache".to_string(),
        prefix_operations: scenario.prefix_len,
        suffix_operations: scenario
            .full_pipeline
            .len()
            .saturating_sub(scenario.prefix_len),
        prefix_dependency: summarize_dependency(&scenario.full_pipeline[..scenario.prefix_len]),
        suffix_dependency: summarize_dependency(&scenario.full_pipeline[scenario.prefix_len..]),
        total_ms_median: median(&totals),
        total_ms_iterations: totals,
        phase_a_ms_median: median(&phase_as),
        phase_a_ms_iterations: phase_as,
        phase_b_ms_median: median(&phase_bs),
        phase_b_ms_iterations: phase_bs,
        phase_c_ms_median: median(&phase_cs),
        phase_c_ms_iterations: phase_cs,
        setup_ms: Some(median(&setups)),
    })
}

fn run_runtime_baseline_case(
    store_root: &Path,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
    iterations: usize,
) -> Result<BenchmarkCase, String> {
    let mut totals = Vec::with_capacity(iterations);
    let mut phase_as = Vec::with_capacity(iterations);
    let mut phase_bs = Vec::with_capacity(iterations);
    let mut phase_cs = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let total_started = Instant::now();
        let view = preview_section_view(store_root, axis, index, &scenario.full_pipeline)
            .map_err(|e| e.to_string())?;
        let total_ms = total_started.elapsed().as_secs_f64() * 1000.0;

        let view_started = Instant::now();
        std::hint::black_box(view);
        let view_ms = view_started.elapsed().as_secs_f64() * 1000.0;

        totals.push(total_ms);
        phase_as.push(0.0);
        phase_bs.push((total_ms - view_ms).max(0.0));
        phase_cs.push(view_ms);
    }

    Ok(BenchmarkCase {
        axis: axis_label(axis).to_string(),
        section_index: index,
        scenario: scenario.name.to_string(),
        strategy: "runtime_full_preview_api".to_string(),
        prefix_operations: scenario.prefix_len,
        suffix_operations: scenario
            .full_pipeline
            .len()
            .saturating_sub(scenario.prefix_len),
        prefix_dependency: summarize_dependency(&scenario.full_pipeline[..scenario.prefix_len]),
        suffix_dependency: summarize_dependency(&scenario.full_pipeline[scenario.prefix_len..]),
        total_ms_median: median(&totals),
        total_ms_iterations: totals,
        phase_a_ms_median: median(&phase_as),
        phase_a_ms_iterations: phase_as,
        phase_b_ms_median: median(&phase_bs),
        phase_b_ms_iterations: phase_bs,
        phase_c_ms_median: median(&phase_cs),
        phase_c_ms_iterations: phase_cs,
        setup_ms: None,
    })
}

fn run_pinned_session_baseline_case(
    store_root: &Path,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
    iterations: usize,
) -> Result<BenchmarkCase, String> {
    let mut setups = Vec::with_capacity(iterations);
    let mut totals = Vec::with_capacity(iterations);
    let mut phase_as = Vec::with_capacity(iterations);
    let mut phase_bs = Vec::with_capacity(iterations);
    let mut phase_cs = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let session = PreviewSectionSession::open(store_root).map_err(|e| e.to_string())?;
        let setup_started = Instant::now();
        let warmup = session
            .preview_section_view(axis, index, &scenario.warmup_pipeline)
            .map_err(|e| e.to_string())?;
        std::hint::black_box(warmup);
        setups.push(setup_started.elapsed().as_secs_f64() * 1000.0);

        let total_started = Instant::now();
        let view = session
            .preview_section_view(axis, index, &scenario.full_pipeline)
            .map_err(|e| e.to_string())?;
        let total_ms = total_started.elapsed().as_secs_f64() * 1000.0;

        let view_started = Instant::now();
        std::hint::black_box(view);
        let view_ms = view_started.elapsed().as_secs_f64() * 1000.0;

        totals.push(total_ms);
        phase_as.push(0.0);
        phase_bs.push((total_ms - view_ms).max(0.0));
        phase_cs.push(view_ms);
    }

    Ok(BenchmarkCase {
        axis: axis_label(axis).to_string(),
        section_index: index,
        scenario: scenario.name.to_string(),
        strategy: "pinned_session_full_preview".to_string(),
        prefix_operations: scenario.prefix_len,
        suffix_operations: scenario
            .full_pipeline
            .len()
            .saturating_sub(scenario.prefix_len),
        prefix_dependency: summarize_dependency(&scenario.full_pipeline[..scenario.prefix_len]),
        suffix_dependency: summarize_dependency(&scenario.full_pipeline[scenario.prefix_len..]),
        total_ms_median: median(&totals),
        total_ms_iterations: totals,
        phase_a_ms_median: median(&phase_as),
        phase_a_ms_iterations: phase_as,
        phase_b_ms_median: median(&phase_bs),
        phase_b_ms_iterations: phase_bs,
        phase_c_ms_median: median(&phase_cs),
        phase_c_ms_iterations: phase_cs,
        setup_ms: Some(median(&setups)),
    })
}

fn run_pinned_session_cache_case(
    store_root: &Path,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
    iterations: usize,
) -> Result<BenchmarkCase, String> {
    let mut setups = Vec::with_capacity(iterations);
    let mut totals = Vec::with_capacity(iterations);
    let mut phase_as = Vec::with_capacity(iterations);
    let mut phase_bs = Vec::with_capacity(iterations);
    let mut phase_cs = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let mut session = PreviewSectionSession::open(store_root).map_err(|e| e.to_string())?;
        let setup_started = Instant::now();
        let (_, warmup_reuse) = session
            .preview_section_view_with_prefix_cache(axis, index, &scenario.warmup_pipeline)
            .map_err(|e| e.to_string())?;
        if warmup_reuse.cache_hit {
            return Err("warmup run unexpectedly hit the pinned-session prefix cache".to_string());
        }
        setups.push(setup_started.elapsed().as_secs_f64() * 1000.0);

        let total_started = Instant::now();
        let (view, reuse) = session
            .preview_section_view_with_prefix_cache(axis, index, &scenario.full_pipeline)
            .map_err(|e| e.to_string())?;
        let total_ms = total_started.elapsed().as_secs_f64() * 1000.0;
        if !reuse.cache_hit || reuse.reused_prefix_operations < scenario.prefix_len {
            return Err(format!(
                "expected pinned session cache hit with at least {} reused operations, found hit={} prefix_ops={}",
                scenario.prefix_len, reuse.cache_hit, reuse.reused_prefix_operations
            ));
        }

        let view_started = Instant::now();
        std::hint::black_box(view);
        let view_ms = view_started.elapsed().as_secs_f64() * 1000.0;

        totals.push(total_ms);
        phase_as.push(0.0);
        phase_bs.push((total_ms - view_ms).max(0.0));
        phase_cs.push(view_ms);
    }

    Ok(BenchmarkCase {
        axis: axis_label(axis).to_string(),
        section_index: index,
        scenario: scenario.name.to_string(),
        strategy: "pinned_session_prefix_cache".to_string(),
        prefix_operations: scenario.prefix_len,
        suffix_operations: scenario
            .full_pipeline
            .len()
            .saturating_sub(scenario.prefix_len),
        prefix_dependency: summarize_dependency(&scenario.full_pipeline[..scenario.prefix_len]),
        suffix_dependency: summarize_dependency(&scenario.full_pipeline[scenario.prefix_len..]),
        total_ms_median: median(&totals),
        total_ms_iterations: totals,
        phase_a_ms_median: median(&phase_as),
        phase_a_ms_iterations: phase_as,
        phase_b_ms_median: median(&phase_bs),
        phase_b_ms_iterations: phase_bs,
        phase_c_ms_median: median(&phase_cs),
        phase_c_ms_iterations: phase_cs,
        setup_ms: Some(median(&setups)),
    })
}

fn baseline_iteration(
    handle: &StoreHandle,
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
) -> Result<IterationBreakdown, String> {
    let total_started = Instant::now();

    let fetch_started = Instant::now();
    let mut plane = assemble_section_plane(reader, axis, index).map_err(|e| e.to_string())?;
    let fetch_ms = fetch_started.elapsed().as_secs_f64() * 1000.0;

    let apply_started = Instant::now();
    apply_pipeline_to_plane(&mut plane, pipeline).map_err(|e| e.to_string())?;
    let apply_ms = apply_started.elapsed().as_secs_f64() * 1000.0;

    let view_started = Instant::now();
    let _ = handle.section_view_from_plane(&plane);
    let view_ms = view_started.elapsed().as_secs_f64() * 1000.0;

    Ok(IterationBreakdown {
        total_ms: total_started.elapsed().as_secs_f64() * 1000.0,
        phase_a_ms: fetch_ms,
        phase_b_ms: apply_ms,
        phase_c_ms: view_ms,
    })
}

fn ephemeral_iteration(
    handle: &StoreHandle,
    prefix_plane: &SectionPlane,
    suffix: &[ProcessingOperation],
    baseline: &SectionPlane,
) -> Result<IterationBreakdown, String> {
    let total_started = Instant::now();

    let clone_started = Instant::now();
    let mut plane = prefix_plane.clone();
    let clone_ms = clone_started.elapsed().as_secs_f64() * 1000.0;

    let apply_started = Instant::now();
    apply_pipeline_to_plane(&mut plane, suffix).map_err(|e| e.to_string())?;
    let apply_ms = apply_started.elapsed().as_secs_f64() * 1000.0;

    assert_planes_close(&plane, baseline)?;

    let view_started = Instant::now();
    let _ = handle.section_view_from_plane(&plane);
    let view_ms = view_started.elapsed().as_secs_f64() * 1000.0;

    Ok(IterationBreakdown {
        total_ms: total_started.elapsed().as_secs_f64() * 1000.0,
        phase_a_ms: clone_ms,
        phase_b_ms: apply_ms,
        phase_c_ms: view_ms,
    })
}

fn fully_processed_plane(
    reader: &TbvolReader,
    axis: SectionAxis,
    index: usize,
    pipeline: &[ProcessingOperation],
) -> Result<SectionPlane, String> {
    let mut plane = assemble_section_plane(reader, axis, index).map_err(|e| e.to_string())?;
    apply_pipeline_to_plane(&mut plane, pipeline).map_err(|e| e.to_string())?;
    Ok(plane)
}

fn assert_planes_close(left: &SectionPlane, right: &SectionPlane) -> Result<(), String> {
    if left.axis != right.axis
        || left.coordinate_index != right.coordinate_index
        || left.traces != right.traces
        || left.samples != right.samples
        || left.amplitudes.len() != right.amplitudes.len()
    {
        return Err("plane metadata mismatch between baseline and reuse result".to_string());
    }
    for (index, (lhs, rhs)) in left
        .amplitudes
        .iter()
        .zip(right.amplitudes.iter())
        .enumerate()
    {
        if (lhs - rhs).abs() > NUMERICAL_TOLERANCE {
            return Err(format!(
                "plane amplitude mismatch at sample {}: left={} right={}",
                index, lhs, rhs
            ));
        }
    }
    if left.occupancy != right.occupancy {
        return Err("plane occupancy mismatch between baseline and reuse result".to_string());
    }
    Ok(())
}

fn median(values: &[f64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_by(|left, right| left.partial_cmp(right).unwrap());
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn axis_label(axis: SectionAxis) -> &'static str {
    match axis {
        SectionAxis::Inline => "inline",
        SectionAxis::Xline => "xline",
    }
}

fn summarize_dependency(operations: &[ProcessingOperation]) -> String {
    if operations.is_empty() {
        return "none".to_string();
    }

    let mut sample_dependency = ProcessingSampleDependency::Pointwise;
    let mut spatial_dependency = ProcessingSpatialDependency::SingleTrace;
    let mut deterministic = true;
    let mut reuse_safe = true;

    for operation in operations {
        let profile = operation.dependency_profile();
        sample_dependency =
            stronger_sample_dependency(sample_dependency, profile.sample_dependency);
        spatial_dependency =
            stronger_spatial_dependency(spatial_dependency, profile.spatial_dependency);
        deterministic &= profile.deterministic;
        reuse_safe &= profile.same_section_ephemeral_reuse_safe;
    }

    format!(
        "{}/{}; deterministic={}; same_section_reuse_safe={}",
        sample_dependency.label(),
        spatial_dependency.label(),
        deterministic,
        reuse_safe
    )
}

fn stronger_sample_dependency(
    left: ProcessingSampleDependency,
    right: ProcessingSampleDependency,
) -> ProcessingSampleDependency {
    if sample_dependency_rank(right) > sample_dependency_rank(left) {
        right
    } else {
        left
    }
}

fn sample_dependency_rank(value: ProcessingSampleDependency) -> usize {
    match value {
        ProcessingSampleDependency::Pointwise => 0,
        ProcessingSampleDependency::BoundedWindow { .. } => 1,
        ProcessingSampleDependency::WholeTrace => 2,
    }
}

fn stronger_spatial_dependency(
    left: ProcessingSpatialDependency,
    right: ProcessingSpatialDependency,
) -> ProcessingSpatialDependency {
    if spatial_dependency_rank(right) > spatial_dependency_rank(left) {
        right
    } else {
        left
    }
}

fn spatial_dependency_rank(value: ProcessingSpatialDependency) -> usize {
    match value {
        ProcessingSpatialDependency::SingleTrace => 0,
        ProcessingSpatialDependency::ExternalVolumePointwise => 1,
        ProcessingSpatialDependency::SectionNeighborhood => 2,
        ProcessingSpatialDependency::GatherNeighborhood => 3,
        ProcessingSpatialDependency::Global => 4,
    }
}

fn print_report(report: &BenchmarkReport) {
    println!(
        "Preview Incremental Benchmark: {} [{} x {} x {}], tile [{} x {} x {}]",
        Path::new(&report.dataset.store_path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| report.dataset.store_path.clone()),
        report.dataset.shape[0],
        report.dataset.shape[1],
        report.dataset.shape[2],
        report.dataset.tile_shape[0],
        report.dataset.tile_shape[1],
        report.dataset.tile_shape[2],
    );
    println!("Iterations per case: {}", report.iterations);
    println!(
        "| Axis | Section | Scenario | Strategy | Prefix Ops | Suffix Ops | Prefix Dependency | Suffix Dependency | Setup ms | Total median ms | Phase A median ms | Phase B median ms | Phase C median ms |"
    );
    println!(
        "| --- | ---: | --- | --- | ---: | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |"
    );
    for case in &report.cases {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} |",
            case.axis,
            case.section_index,
            case.scenario,
            case.strategy,
            case.prefix_operations,
            case.suffix_operations,
            case.prefix_dependency,
            case.suffix_dependency,
            case.setup_ms
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "-".to_string()),
            case.total_ms_median,
            case.phase_a_ms_median,
            case.phase_b_ms_median,
            case.phase_c_ms_median,
        );
    }
    println!();
    println!("Phase labels:");
    println!(
        "- fused_selective_recompute: phase A = section fetch, phase B = full pipeline apply, phase C = view shaping"
    );
    println!(
        "- reuse: phase A = prefix-plane clone, phase B = suffix apply, phase C = view shaping"
    );
}
