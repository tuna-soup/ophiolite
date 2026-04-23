use std::path::PathBuf;
use std::time::Instant;

use seis_contracts_operations::IPC_SCHEMA_VERSION;
use seis_runtime::{
    DatasetId, FrequencyPhaseMode, FrequencyWindowShape, LocalVolumeStatistic,
    NeighborhoodDipOutput, PostStackNeighborhoodProcessingOperation,
    PostStackNeighborhoodProcessingPipeline, PostStackNeighborhoodWindow,
    PreviewPostStackNeighborhoodProcessingRequest, PreviewPostStackNeighborhoodProcessingResponse,
    PreviewTraceLocalProcessingRequest, PreviewTraceLocalProcessingResponse, ProcessingOperation,
    SectionAxis, SectionRequest, TraceLocalProcessingPipeline, TraceLocalProcessingStep,
    open_store,
};
use traceboost_app::{preview_post_stack_neighborhood_processing, preview_processing};

use crate::preview_session::PreviewSessionState;

const BENCH_ITERATIONS: usize = 3;

#[derive(Debug, Clone)]
struct Scenario {
    name: &'static str,
    warmup_pipeline: TraceLocalProcessingPipeline,
    full_pipeline: TraceLocalProcessingPipeline,
    prefix_len: usize,
}

#[derive(Debug, Clone)]
struct NeighborhoodScenario {
    name: &'static str,
    warmup_pipeline: PostStackNeighborhoodProcessingPipeline,
    full_pipeline: PostStackNeighborhoodProcessingPipeline,
    expect_prefix_cache_hit: bool,
    min_reused_prefix_operations: usize,
}

#[derive(Debug)]
struct BenchmarkCase {
    axis: &'static str,
    index: usize,
    scenario: &'static str,
    strategy: &'static str,
    iterations_ms: Vec<f64>,
    median_ms: f64,
}

fn benchmark_store_path() -> PathBuf {
    let candidates = [
        std::env::temp_dir().join("f3_dataset-smoke.tbvol"),
        PathBuf::from(r"H:\traceboost-bench\f3_dataset-smoke.tbvol"),
    ];
    for candidate in candidates {
        if candidate.is_dir() {
            return candidate;
        }
    }
    panic!("Expected large F3 tbvol benchmark dataset to exist in %TEMP% or H:\\traceboost-bench");
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap());
    sorted[sorted.len() / 2]
}

fn axis_label(axis: SectionAxis) -> &'static str {
    match axis {
        SectionAxis::Inline => "inline",
        SectionAxis::Xline => "xline",
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

fn trace_local_pipeline_named(
    name: &str,
    steps: Vec<TraceLocalProcessingStep>,
) -> TraceLocalProcessingPipeline {
    TraceLocalProcessingPipeline {
        schema_version: 2,
        revision: 1,
        preset_id: None,
        name: Some(name.to_string()),
        description: None,
        steps,
    }
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "late_scalar_edit",
            warmup_pipeline: TraceLocalProcessingPipeline {
                schema_version: 2,
                revision: 1,
                preset_id: None,
                name: Some("Late Scalar Warmup".to_string()),
                description: None,
                steps: steps(vec![
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
                ]),
            },
            full_pipeline: TraceLocalProcessingPipeline {
                schema_version: 2,
                revision: 1,
                preset_id: None,
                name: Some("Late Scalar Full".to_string()),
                description: None,
                steps: steps(vec![
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
                ]),
            },
            prefix_len: 4,
        },
        Scenario {
            name: "late_filter_edit",
            warmup_pipeline: TraceLocalProcessingPipeline {
                schema_version: 2,
                revision: 1,
                preset_id: None,
                name: Some("Late Filter Warmup".to_string()),
                description: None,
                steps: steps(vec![
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
                ]),
            },
            full_pipeline: TraceLocalProcessingPipeline {
                schema_version: 2,
                revision: 1,
                preset_id: None,
                name: Some("Late Filter Full".to_string()),
                description: None,
                steps: steps(vec![
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
                ]),
            },
            prefix_len: 4,
        },
        Scenario {
            name: "late_agc_edit",
            warmup_pipeline: TraceLocalProcessingPipeline {
                schema_version: 2,
                revision: 1,
                preset_id: None,
                name: Some("Late AGC Warmup".to_string()),
                description: None,
                steps: steps(vec![
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
                ]),
            },
            full_pipeline: TraceLocalProcessingPipeline {
                schema_version: 2,
                revision: 1,
                preset_id: None,
                name: Some("Late AGC Full".to_string()),
                description: None,
                steps: steps(vec![
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
                ]),
            },
            prefix_len: 3,
        },
    ]
}

fn bandpass_10_20_60_80() -> ProcessingOperation {
    ProcessingOperation::BandpassFilter {
        f1_hz: 10.0,
        f2_hz: 20.0,
        f3_hz: 60.0,
        f4_hz: 80.0,
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn neighborhood_window(
    gate_ms: f32,
    inline_stepout: usize,
    xline_stepout: usize,
) -> PostStackNeighborhoodWindow {
    PostStackNeighborhoodWindow {
        gate_ms,
        inline_stepout,
        xline_stepout,
    }
}

fn neighborhood_pipeline_named(
    name: &str,
    trace_local_pipeline: Option<TraceLocalProcessingPipeline>,
    operation: PostStackNeighborhoodProcessingOperation,
) -> PostStackNeighborhoodProcessingPipeline {
    PostStackNeighborhoodProcessingPipeline {
        schema_version: 1,
        revision: 1,
        preset_id: None,
        name: Some(name.to_string()),
        description: None,
        trace_local_pipeline,
        operations: vec![operation],
    }
}

fn neighborhood_scenarios() -> Vec<NeighborhoodScenario> {
    let balanced_bandpass_prefix = trace_local_pipeline_named(
        "Bandpass 10-20-60-80 Hz",
        vec![TraceLocalProcessingStep {
            operation: bandpass_10_20_60_80(),
            checkpoint: false,
        }],
    );
    vec![
        NeighborhoodScenario {
            name: "similarity_tight_no_prefix",
            warmup_pipeline: neighborhood_pipeline_named(
                "Similarity Tight",
                None,
                PostStackNeighborhoodProcessingOperation::Similarity {
                    window: neighborhood_window(24.0, 1, 1),
                },
            ),
            full_pipeline: neighborhood_pipeline_named(
                "Similarity Tight",
                None,
                PostStackNeighborhoodProcessingOperation::Similarity {
                    window: neighborhood_window(24.0, 1, 1),
                },
            ),
            expect_prefix_cache_hit: false,
            min_reused_prefix_operations: 0,
        },
        NeighborhoodScenario {
            name: "similarity_balanced_bandpass_prefix",
            warmup_pipeline: neighborhood_pipeline_named(
                "Similarity Warmup",
                Some(balanced_bandpass_prefix.clone()),
                PostStackNeighborhoodProcessingOperation::Similarity {
                    window: neighborhood_window(24.0, 1, 1),
                },
            ),
            full_pipeline: neighborhood_pipeline_named(
                "Similarity Balanced",
                Some(balanced_bandpass_prefix.clone()),
                PostStackNeighborhoodProcessingOperation::Similarity {
                    window: neighborhood_window(32.0, 2, 2),
                },
            ),
            expect_prefix_cache_hit: true,
            min_reused_prefix_operations: 1,
        },
        NeighborhoodScenario {
            name: "local_stats_balanced_no_prefix",
            warmup_pipeline: neighborhood_pipeline_named(
                "Local RMS Balanced",
                None,
                PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
                    window: neighborhood_window(32.0, 2, 2),
                    statistic: LocalVolumeStatistic::Rms,
                },
            ),
            full_pipeline: neighborhood_pipeline_named(
                "Local RMS Balanced",
                None,
                PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
                    window: neighborhood_window(32.0, 2, 2),
                    statistic: LocalVolumeStatistic::Rms,
                },
            ),
            expect_prefix_cache_hit: false,
            min_reused_prefix_operations: 0,
        },
        NeighborhoodScenario {
            name: "dip_balanced_bandpass_prefix",
            warmup_pipeline: neighborhood_pipeline_named(
                "Dip Warmup",
                Some(balanced_bandpass_prefix.clone()),
                PostStackNeighborhoodProcessingOperation::Dip {
                    window: neighborhood_window(24.0, 1, 1),
                    output: NeighborhoodDipOutput::Inline,
                },
            ),
            full_pipeline: neighborhood_pipeline_named(
                "Dip Balanced",
                Some(balanced_bandpass_prefix),
                PostStackNeighborhoodProcessingOperation::Dip {
                    window: neighborhood_window(32.0, 2, 2),
                    output: NeighborhoodDipOutput::Inline,
                },
            ),
            expect_prefix_cache_hit: true,
            min_reused_prefix_operations: 1,
        },
    ]
}

fn dip_profile_scenarios() -> Vec<NeighborhoodScenario> {
    let balanced_bandpass_prefix = trace_local_pipeline_named(
        "Bandpass 10-20-60-80 Hz",
        vec![TraceLocalProcessingStep {
            operation: bandpass_10_20_60_80(),
            checkpoint: false,
        }],
    );
    vec![
        NeighborhoodScenario {
            name: "dip_balanced_no_prefix",
            warmup_pipeline: neighborhood_pipeline_named(
                "Dip Balanced",
                None,
                PostStackNeighborhoodProcessingOperation::Dip {
                    window: neighborhood_window(32.0, 2, 2),
                    output: NeighborhoodDipOutput::Inline,
                },
            ),
            full_pipeline: neighborhood_pipeline_named(
                "Dip Balanced",
                None,
                PostStackNeighborhoodProcessingOperation::Dip {
                    window: neighborhood_window(32.0, 2, 2),
                    output: NeighborhoodDipOutput::Inline,
                },
            ),
            expect_prefix_cache_hit: false,
            min_reused_prefix_operations: 0,
        },
        NeighborhoodScenario {
            name: "dip_balanced_bandpass_prefix",
            warmup_pipeline: neighborhood_pipeline_named(
                "Dip Warmup",
                Some(balanced_bandpass_prefix.clone()),
                PostStackNeighborhoodProcessingOperation::Dip {
                    window: neighborhood_window(24.0, 1, 1),
                    output: NeighborhoodDipOutput::Inline,
                },
            ),
            full_pipeline: neighborhood_pipeline_named(
                "Dip Balanced",
                Some(balanced_bandpass_prefix),
                PostStackNeighborhoodProcessingOperation::Dip {
                    window: neighborhood_window(32.0, 2, 2),
                    output: NeighborhoodDipOutput::Inline,
                },
            ),
            expect_prefix_cache_hit: true,
            min_reused_prefix_operations: 1,
        },
    ]
}

fn make_request(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    pipeline: &TraceLocalProcessingPipeline,
) -> PreviewTraceLocalProcessingRequest {
    PreviewTraceLocalProcessingRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: store_path.to_string(),
        section: SectionRequest {
            dataset_id: dataset_id.clone(),
            axis,
            index,
        },
        pipeline: pipeline.clone(),
    }
}

fn make_neighborhood_request(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    pipeline: &PostStackNeighborhoodProcessingPipeline,
) -> PreviewPostStackNeighborhoodProcessingRequest {
    PreviewPostStackNeighborhoodProcessingRequest {
        schema_version: IPC_SCHEMA_VERSION,
        store_path: store_path.to_string(),
        section: SectionRequest {
            dataset_id: dataset_id.clone(),
            axis,
            index,
        },
        pipeline: pipeline.clone(),
    }
}

fn assert_same_preview(
    left: &PreviewTraceLocalProcessingResponse,
    right: &PreviewTraceLocalProcessingResponse,
) {
    assert_eq!(left.preview.section.traces, right.preview.section.traces);
    assert_eq!(left.preview.section.samples, right.preview.section.samples);
    assert_eq!(
        left.preview.section.amplitudes_f32le,
        right.preview.section.amplitudes_f32le
    );
    assert_eq!(
        left.preview.section.sample_axis_f32le,
        right.preview.section.sample_axis_f32le
    );
    assert_eq!(
        left.preview.section.horizontal_axis_f64le,
        right.preview.section.horizontal_axis_f64le
    );
}

fn assert_same_neighborhood_preview(
    left: &PreviewPostStackNeighborhoodProcessingResponse,
    right: &PreviewPostStackNeighborhoodProcessingResponse,
) {
    assert_eq!(left.preview.section.traces, right.preview.section.traces);
    assert_eq!(left.preview.section.samples, right.preview.section.samples);
    assert_eq!(
        left.preview.section.amplitudes_f32le,
        right.preview.section.amplitudes_f32le
    );
    assert_eq!(
        left.preview.section.sample_axis_f32le,
        right.preview.section.sample_axis_f32le
    );
    assert_eq!(
        left.preview.section.horizontal_axis_f64le,
        right.preview.section.horizontal_axis_f64le
    );
}

fn run_stateless_case(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
) -> BenchmarkCase {
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for _ in 0..BENCH_ITERATIONS {
        let warmup = make_request(
            store_path,
            dataset_id,
            axis,
            index,
            &scenario.warmup_pipeline,
        );
        let full = make_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let expected = preview_processing(full.clone()).expect("stateless full preview baseline");
        let _ = preview_processing(warmup).expect("stateless warmup preview");
        let started = Instant::now();
        let response = preview_processing(full).expect("stateless full preview");
        iterations_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        assert_same_preview(&response, &expected);
    }
    BenchmarkCase {
        axis: axis_label(axis),
        index,
        scenario: scenario.name,
        strategy: "desktop_stateless_preview",
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn run_session_case(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    scenario: &Scenario,
) -> BenchmarkCase {
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for _ in 0..BENCH_ITERATIONS {
        let baseline_request =
            make_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let baseline = preview_processing(baseline_request).expect("stateless baseline preview");
        let state = PreviewSessionState::default();
        let warmup_request = make_request(
            store_path,
            dataset_id,
            axis,
            index,
            &scenario.warmup_pipeline,
        );
        let full_request =
            make_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let (_, warmup_reuse) = state
            .preview_processing(warmup_request)
            .expect("preview session warmup");
        assert!(
            !warmup_reuse.cache_hit,
            "warmup should not hit session cache"
        );
        let started = Instant::now();
        let (response, reuse) = state
            .preview_processing(full_request)
            .expect("preview session cached preview");
        iterations_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        assert!(reuse.cache_hit, "expected preview session cache hit");
        assert!(
            reuse.reused_prefix_operations >= scenario.prefix_len,
            "expected at least {} reused operations, got {}",
            scenario.prefix_len,
            reuse.reused_prefix_operations
        );
        assert_same_preview(&response, &baseline);
    }
    BenchmarkCase {
        axis: axis_label(axis),
        index,
        scenario: scenario.name,
        strategy: "desktop_session_preview_cache",
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn run_neighborhood_stateless_case(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    scenario: &NeighborhoodScenario,
) -> BenchmarkCase {
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for _ in 0..BENCH_ITERATIONS {
        let full =
            make_neighborhood_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let started = Instant::now();
        let _ = preview_post_stack_neighborhood_processing(full)
            .expect("stateless neighborhood preview");
        iterations_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    BenchmarkCase {
        axis: axis_label(axis),
        index,
        scenario: scenario.name,
        strategy: "desktop_stateless_preview",
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn run_neighborhood_session_repeat_case(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    scenario: &NeighborhoodScenario,
) -> BenchmarkCase {
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for _ in 0..BENCH_ITERATIONS {
        let baseline_request =
            make_neighborhood_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let baseline = preview_post_stack_neighborhood_processing(baseline_request)
            .expect("stateless neighborhood baseline preview");
        let state = PreviewSessionState::default();
        let full_request =
            make_neighborhood_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let (_, cold_reuse) = state
            .preview_post_stack_neighborhood_processing(full_request.clone())
            .expect("neighborhood session cold preview");
        assert!(
            !cold_reuse.cache_hit,
            "cold neighborhood preview should not hit session cache"
        );
        let started = Instant::now();
        let (response, reuse) = state
            .preview_post_stack_neighborhood_processing(full_request)
            .expect("neighborhood session repeated preview");
        iterations_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        if scenario.expect_prefix_cache_hit {
            assert!(reuse.cache_hit, "expected neighborhood preview cache hit");
            assert!(
                reuse.reused_prefix_operations >= scenario.min_reused_prefix_operations,
                "expected at least {} reused prefix operations, got {}",
                scenario.min_reused_prefix_operations,
                reuse.reused_prefix_operations
            );
        } else {
            assert!(
                !reuse.cache_hit,
                "unexpected neighborhood preview cache hit for {}",
                scenario.name
            );
            assert_eq!(reuse.reused_prefix_operations, 0);
        }
        assert_same_neighborhood_preview(&response, &baseline);
    }
    BenchmarkCase {
        axis: axis_label(axis),
        index,
        scenario: scenario.name,
        strategy: "desktop_session_repeat",
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn run_neighborhood_session_prefix_edit_case(
    store_path: &str,
    dataset_id: &DatasetId,
    axis: SectionAxis,
    index: usize,
    scenario: &NeighborhoodScenario,
) -> BenchmarkCase {
    let mut iterations_ms = Vec::with_capacity(BENCH_ITERATIONS);
    for _ in 0..BENCH_ITERATIONS {
        let baseline_request =
            make_neighborhood_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let baseline = preview_post_stack_neighborhood_processing(baseline_request)
            .expect("stateless neighborhood baseline preview");
        let state = PreviewSessionState::default();
        let warmup_request = make_neighborhood_request(
            store_path,
            dataset_id,
            axis,
            index,
            &scenario.warmup_pipeline,
        );
        let full_request =
            make_neighborhood_request(store_path, dataset_id, axis, index, &scenario.full_pipeline);
        let (_, warmup_reuse) = state
            .preview_post_stack_neighborhood_processing(warmup_request)
            .expect("neighborhood session warmup preview");
        assert!(
            !warmup_reuse.cache_hit,
            "warmup neighborhood preview should not hit session cache"
        );
        let started = Instant::now();
        let (response, reuse) = state
            .preview_post_stack_neighborhood_processing(full_request)
            .expect("neighborhood session prefix-edit preview");
        iterations_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        if scenario.expect_prefix_cache_hit {
            assert!(reuse.cache_hit, "expected neighborhood preview cache hit");
            assert!(
                reuse.reused_prefix_operations >= scenario.min_reused_prefix_operations,
                "expected at least {} reused prefix operations, got {}",
                scenario.min_reused_prefix_operations,
                reuse.reused_prefix_operations
            );
        } else {
            assert!(
                !reuse.cache_hit,
                "unexpected neighborhood preview cache hit for {}",
                scenario.name
            );
            assert_eq!(reuse.reused_prefix_operations, 0);
        }
        assert_same_neighborhood_preview(&response, &baseline);
    }
    BenchmarkCase {
        axis: axis_label(axis),
        index,
        scenario: scenario.name,
        strategy: "desktop_session_prefix_edit",
        median_ms: median(&iterations_ms),
        iterations_ms,
    }
}

fn print_cases(store_path: &str, cases: &[BenchmarkCase]) {
    println!("Desktop Preview Session Benchmark: {store_path}");
    println!("Iterations per case: {BENCH_ITERATIONS}");
    println!("| Axis | Section | Scenario | Strategy | Median ms | Iterations ms |");
    println!("| --- | ---: | --- | --- | ---: | --- |");
    for case in cases {
        println!(
            "| {} | {} | {} | {} | {:.3} | {:?} |",
            case.axis, case.index, case.scenario, case.strategy, case.median_ms, case.iterations_ms
        );
    }
}

#[test]
#[ignore]
fn benchmark_desktop_preview_session_large_f3() {
    let store_path = benchmark_store_path();
    let handle = open_store(&store_path).expect("open desktop preview benchmark store");
    let dataset_id = handle.dataset_id();
    let inline_index = handle.manifest.volume.shape[0] / 2;
    let xline_index = handle.manifest.volume.shape[1] / 2;

    let mut cases = Vec::new();
    for scenario in scenarios() {
        cases.push(run_stateless_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_session_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_stateless_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
        cases.push(run_session_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
    }
    for scenario in neighborhood_scenarios() {
        cases.push(run_neighborhood_stateless_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_repeat_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_prefix_edit_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_stateless_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_repeat_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_prefix_edit_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
    }

    print_cases(&store_path.display().to_string(), &cases);
}

#[test]
#[ignore]
fn benchmark_desktop_preview_session_dip_profile_large_f3() {
    let store_path = benchmark_store_path();
    let handle = open_store(&store_path).expect("open desktop preview benchmark store");
    let dataset_id = handle.dataset_id();
    let inline_index = handle.manifest.volume.shape[0] / 2;
    let xline_index = handle.manifest.volume.shape[1] / 2;

    let mut cases = Vec::new();
    for scenario in dip_profile_scenarios() {
        cases.push(run_neighborhood_stateless_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_repeat_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_prefix_edit_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Inline,
            inline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_stateless_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_repeat_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
        cases.push(run_neighborhood_session_prefix_edit_case(
            &store_path.to_string_lossy(),
            &dataset_id,
            SectionAxis::Xline,
            xline_index,
            &scenario,
        ));
    }

    print_cases(&store_path.display().to_string(), &cases);
}
