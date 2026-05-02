use clap::{Parser, ValueEnum};
use ophiolite_seismic::SectionAxis;
use ophiolite_seismic_runtime::{
    SectionPlane, TbvolReader, TbvolcReader, TileGeometry, VolumeStoreReader,
    assemble_section_plane, read_section_window_artifact,
};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

const DEFAULT_ITERATIONS: usize = 5;
const DEFAULT_SCREEN_TRACES: usize = 1200;
const DEFAULT_SCREEN_SAMPLES: usize = 900;
const DEFAULT_FOCUS_TRACES: usize = 512;
const DEFAULT_FOCUS_SAMPLES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AxisSelection {
    Inline,
    Xline,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "section-tile-bench")]
#[command(about = "Benchmark full-section reads versus tiled and decimated section views")]
struct Cli {
    #[arg(long)]
    store: PathBuf,
    #[arg(long, value_enum, default_value_t = AxisSelection::Both)]
    axis: AxisSelection,
    #[arg(long)]
    inline_index: Option<usize>,
    #[arg(long)]
    xline_index: Option<usize>,
    #[arg(long, default_value_t = DEFAULT_ITERATIONS)]
    iterations: usize,
    #[arg(long, default_value_t = DEFAULT_SCREEN_TRACES)]
    screen_traces: usize,
    #[arg(long, default_value_t = DEFAULT_SCREEN_SAMPLES)]
    screen_samples: usize,
    #[arg(long, default_value_t = DEFAULT_FOCUS_TRACES)]
    focus_traces: usize,
    #[arg(long, default_value_t = DEFAULT_FOCUS_SAMPLES)]
    focus_samples: usize,
    #[arg(long, value_delimiter = ',', default_values_t = [0u8, 1u8])]
    focus_lod: Vec<u8>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkReport {
    dataset: DatasetSummary,
    iterations: usize,
    overview_target: [usize; 2],
    focus_target: [usize; 2],
    cases: Vec<BenchmarkCase>,
}

#[derive(Debug, Clone, Serialize)]
struct DatasetSummary {
    store_path: String,
    store_format: String,
    shape: [usize; 3],
    tile_shape: [usize; 3],
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkCase {
    axis: String,
    index: usize,
    scenario: String,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    trace_step: usize,
    sample_step: usize,
    output_traces: usize,
    output_samples: usize,
    payload_bytes: u64,
    payload_fraction_of_full: f64,
    iteration_ms: Vec<f64>,
    median_ms: f64,
    mean_ms: f64,
}

#[derive(Debug, Clone, Copy)]
struct AxisPlan {
    axis: SectionAxis,
    index: usize,
}

#[derive(Debug, Clone)]
enum Scenario {
    FullSection,
    OverviewFit {
        trace_range: [usize; 2],
        sample_range: [usize; 2],
        lod: u8,
    },
    FocusTile {
        trace_range: [usize; 2],
        sample_range: [usize; 2],
        lod: u8,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = BenchStore::open(cli.store.clone())?;
    let axes = planned_axes(store.shape(), cli.axis, cli.inline_index, cli.xline_index);
    let mut cases = Vec::new();

    for plan in axes {
        let full_section = store.read_section_plane(plan.axis, plan.index)?;
        let full_payload_bytes = payload_bytes(&full_section);
        let scenarios = planned_scenarios(
            &full_section,
            cli.screen_traces,
            cli.screen_samples,
            cli.focus_traces,
            cli.focus_samples,
            &cli.focus_lod,
        );

        for scenario in scenarios {
            cases.push(run_case(
                &store,
                plan,
                &scenario,
                cli.iterations,
                full_payload_bytes,
            )?);
        }
    }

    let report = BenchmarkReport {
        dataset: DatasetSummary {
            store_path: cli.store.display().to_string(),
            store_format: store.format().to_string(),
            shape: store.shape(),
            tile_shape: store.tile_shape(),
        },
        iterations: cli.iterations,
        overview_target: [cli.screen_traces, cli.screen_samples],
        focus_target: [cli.focus_traces, cli.focus_samples],
        cases,
    };

    match cli.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Text => print_report(&report),
    }

    Ok(())
}

fn planned_axes(
    shape: [usize; 3],
    selection: AxisSelection,
    inline_index: Option<usize>,
    xline_index: Option<usize>,
) -> Vec<AxisPlan> {
    let mid_inline = shape[0] / 2;
    let mid_xline = shape[1] / 2;
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

fn planned_scenarios(
    section: &SectionPlane,
    screen_traces: usize,
    screen_samples: usize,
    focus_traces: usize,
    focus_samples: usize,
    focus_lods: &[u8],
) -> Vec<Scenario> {
    let overview_lod = choose_overview_lod(
        section.traces,
        section.samples,
        screen_traces,
        screen_samples,
    );
    let focus_trace_range = centered_range(section.traces, focus_traces);
    let focus_sample_range = centered_range(section.samples, focus_samples);
    let mut scenarios = vec![
        Scenario::FullSection,
        Scenario::OverviewFit {
            trace_range: [0, section.traces],
            sample_range: [0, section.samples],
            lod: overview_lod,
        },
    ];

    for &lod in focus_lods {
        scenarios.push(Scenario::FocusTile {
            trace_range: focus_trace_range,
            sample_range: focus_sample_range,
            lod,
        });
    }

    scenarios
}

fn centered_range(total: usize, target_span: usize) -> [usize; 2] {
    let span = total.min(target_span.max(1));
    let start = total.saturating_sub(span) / 2;
    [start, start + span]
}

fn choose_overview_lod(
    total_traces: usize,
    total_samples: usize,
    target_traces: usize,
    target_samples: usize,
) -> u8 {
    choose_axis_lod(total_traces, target_traces).max(choose_axis_lod(total_samples, target_samples))
}

fn choose_axis_lod(total: usize, target: usize) -> u8 {
    let mut lod = 0u8;
    let mut visible = total.max(1);
    let target = target.max(1);
    while visible > target && lod < u8::MAX {
        lod = lod.saturating_add(1);
        visible = visible.div_ceil(2);
    }
    lod
}

fn run_case(
    store: &BenchStore,
    plan: AxisPlan,
    scenario: &Scenario,
    iterations: usize,
    full_payload_bytes: u64,
) -> Result<BenchmarkCase, Box<dyn std::error::Error>> {
    let mut iteration_ms = Vec::with_capacity(iterations);
    let mut measured = MeasuredPayload {
        scenario: String::new(),
        trace_range: [0, 0],
        sample_range: [0, 0],
        lod: 0,
        trace_step: 1,
        sample_step: 1,
        output_traces: 0,
        output_samples: 0,
        payload_bytes: 0,
    };

    for _ in 0..iterations {
        let started = Instant::now();
        measured = match scenario {
            Scenario::FullSection => {
                let section = store.read_section_plane(plan.axis, plan.index)?;
                MeasuredPayload {
                    scenario: "full_section".to_string(),
                    trace_range: [0, section.traces],
                    sample_range: [0, section.samples],
                    lod: 0,
                    trace_step: 1,
                    sample_step: 1,
                    output_traces: section.traces,
                    output_samples: section.samples,
                    payload_bytes: payload_bytes(&section),
                }
            }
            Scenario::OverviewFit {
                trace_range,
                sample_range,
                lod,
            } => measured_tile(
                "overview_fit".to_string(),
                store.read_section_tile_plane(
                    plan.axis,
                    plan.index,
                    *trace_range,
                    *sample_range,
                    *lod,
                )?,
            ),
            Scenario::FocusTile {
                trace_range,
                sample_range,
                lod,
            } => measured_tile(
                format!("focus_tile_lod_{lod}"),
                store.read_section_tile_plane(
                    plan.axis,
                    plan.index,
                    *trace_range,
                    *sample_range,
                    *lod,
                )?,
            ),
        };
        iteration_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    let mut sorted = iteration_ms.clone();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let median_ms = if sorted.is_empty() {
        0.0
    } else if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        let right = sorted.len() / 2;
        (sorted[right - 1] + sorted[right]) / 2.0
    };
    let mean_ms = if iteration_ms.is_empty() {
        0.0
    } else {
        iteration_ms.iter().sum::<f64>() / iteration_ms.len() as f64
    };

    Ok(BenchmarkCase {
        axis: axis_name(plan.axis).to_string(),
        index: plan.index,
        scenario: measured.scenario,
        trace_range: measured.trace_range,
        sample_range: measured.sample_range,
        lod: measured.lod,
        trace_step: measured.trace_step,
        sample_step: measured.sample_step,
        output_traces: measured.output_traces,
        output_samples: measured.output_samples,
        payload_bytes: measured.payload_bytes,
        payload_fraction_of_full: if full_payload_bytes == 0 {
            0.0
        } else {
            measured.payload_bytes as f64 / full_payload_bytes as f64
        },
        iteration_ms,
        median_ms,
        mean_ms,
    })
}

enum BenchStore {
    Tbvol { reader: TbvolReader },
    Tbvolc { reader: TbvolcReader },
}

impl BenchStore {
    fn open(root: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let manifest_path = root.join("manifest.json");
        let manifest =
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&manifest_path)?)?;
        let format = manifest
            .get("format")
            .and_then(|value| value.as_str())
            .ok_or_else(|| format!("missing store format in {}", manifest_path.display()))?;
        match format {
            "tbvol" => Ok(Self::Tbvol {
                reader: TbvolReader::open(&root)?,
            }),
            "tbvolc" => Ok(Self::Tbvolc {
                reader: TbvolcReader::open(&root)?,
            }),
            other => {
                Err(format!("unsupported store format for section_tile_bench: {other}").into())
            }
        }
    }

    fn format(&self) -> &'static str {
        match self {
            Self::Tbvol { .. } => "tbvol",
            Self::Tbvolc { .. } => "tbvolc",
        }
    }

    fn shape(&self) -> [usize; 3] {
        match self {
            Self::Tbvol { reader, .. } => reader.volume().shape,
            Self::Tbvolc { reader, .. } => reader.volume().shape,
        }
    }

    fn tile_shape(&self) -> [usize; 3] {
        self.tile_geometry().tile_shape()
    }

    fn tile_geometry(&self) -> &TileGeometry {
        match self {
            Self::Tbvol { reader, .. } => reader.tile_geometry(),
            Self::Tbvolc { reader, .. } => reader.tile_geometry(),
        }
    }

    fn read_section_plane(
        &self,
        axis: SectionAxis,
        index: usize,
    ) -> Result<SectionPlane, Box<dyn std::error::Error>> {
        match self {
            Self::Tbvol { reader, .. } => Ok(assemble_section_plane(reader, axis, index)?),
            Self::Tbvolc { reader, .. } => Ok(assemble_section_plane(reader, axis, index)?),
        }
    }

    fn read_section_tile_plane(
        &self,
        axis: SectionAxis,
        index: usize,
        trace_range: [usize; 2],
        sample_range: [usize; 2],
        lod: u8,
    ) -> Result<MeasuredPayload, Box<dyn std::error::Error>> {
        let artifact = match self {
            Self::Tbvol { reader, .. } => {
                read_section_window_artifact(reader, axis, index, trace_range, sample_range, lod)?
            }
            Self::Tbvolc { reader, .. } => {
                read_section_window_artifact(reader, axis, index, trace_range, sample_range, lod)?
            }
        };
        let trace_step = artifact.assembly_plan.output_shape[0].max(1);
        let sample_step = artifact.assembly_plan.output_shape[1].max(1);
        Ok(MeasuredPayload {
            scenario: String::new(),
            trace_range,
            sample_range,
            lod,
            trace_step: (trace_range[1] - trace_range[0]).div_ceil(trace_step),
            sample_step: (sample_range[1] - sample_range[0]).div_ceil(sample_step),
            output_traces: artifact.plane.traces,
            output_samples: artifact.plane.samples,
            payload_bytes: payload_bytes(&artifact.plane),
        })
    }
}

#[derive(Debug, Clone)]
struct MeasuredPayload {
    scenario: String,
    trace_range: [usize; 2],
    sample_range: [usize; 2],
    lod: u8,
    trace_step: usize,
    sample_step: usize,
    output_traces: usize,
    output_samples: usize,
    payload_bytes: u64,
}

fn measured_tile(scenario: String, mut measured: MeasuredPayload) -> MeasuredPayload {
    measured.scenario = scenario;
    measured
}

fn payload_bytes(section: &SectionPlane) -> u64 {
    ((section.horizontal_axis.len() * std::mem::size_of::<f64>())
        + (section.sample_axis_ms.len() * std::mem::size_of::<f32>())
        + (section.amplitudes.len() * std::mem::size_of::<f32>())) as u64
}

fn axis_name(axis: SectionAxis) -> &'static str {
    match axis {
        SectionAxis::Inline => "inline",
        SectionAxis::Xline => "xline",
    }
}

fn print_report(report: &BenchmarkReport) {
    println!("store: {}", report.dataset.store_path);
    println!(
        "format: {} | shape: {:?} | tile_shape: {:?}",
        report.dataset.store_format, report.dataset.shape, report.dataset.tile_shape
    );
    println!(
        "iterations: {} | overview target: {}x{} | focus target: {}x{}",
        report.iterations,
        report.overview_target[0],
        report.overview_target[1],
        report.focus_target[0],
        report.focus_target[1]
    );
    println!();

    let mut current_axis = "";
    for case in &report.cases {
        if case.axis != current_axis {
            current_axis = &case.axis;
            println!("{} @ {}", case.axis, case.index);
        }

        println!(
            "  {:<18} median={:>8.3} ms mean={:>8.3} ms payload={:>8} KiB ({:>5.1}%) output={}x{} range=t{}..{} s{}..{} lod={} step={}x{}",
            case.scenario,
            case.median_ms,
            case.mean_ms,
            case.payload_bytes as f64 / 1024.0,
            case.payload_fraction_of_full * 100.0,
            case.output_traces,
            case.output_samples,
            case.trace_range[0],
            case.trace_range[1],
            case.sample_range[0],
            case.sample_range[1],
            case.lod,
            case.trace_step,
            case.sample_step
        );
        println!("    iterations_ms={:?}", case.iteration_ms);
    }
}
