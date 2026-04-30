mod operation_catalog;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use traceboost_app::workflow_report::WorkflowRunReport;
use traceboost_app::workflow_report_render::{
    render_workflow_report_markdown, render_workflow_report_mermaid,
};
use traceboost_app::workflow_runner::{
    RunWorkflowOptions, load_workflow_recipe_from_json_path, run_workflow_recipe,
    validate_workflow_recipe,
};
use traceboost_app::{
    PostStackNeighborhoodBenchmarkOperator, PostStackNeighborhoodPreviewBenchmarkRequest,
    PostStackNeighborhoodProcessingBenchmarkRequest, PrepareSurveyDemoRequest,
    TraceBoostWorkflowService, TraceLocalBatchBenchmarkRequest, TraceLocalBenchmarkRequest,
    TraceLocalBenchmarkScenario, apply_gather_processing, apply_processing,
    apply_subvolume_processing, benchmark_post_stack_neighborhood_preview,
    benchmark_post_stack_neighborhood_processing, benchmark_trace_local_batch_processing,
    benchmark_trace_local_processing, dataset_operator_catalog, import_horizon_xyz,
    preview_gather_processing, preview_processing, preview_subvolume_processing, run_velocity_scan,
};

use clap::{Parser, Subcommand, ValueEnum};
use operation_catalog::operation_catalog;
use seis_contracts_operations::datasets::OpenDatasetRequest;
use seis_contracts_operations::import_ops::{
    ExportSegyRequest, ImportDatasetRequest, ImportHorizonXyzRequest,
    ImportPrestackOffsetDatasetRequest, LoadSectionHorizonsRequest, PrestackThirdAxisField,
    SegyGeometryOverride, SegyHeaderField, SegyHeaderValueType, SurveyPreflightRequest,
};
use seis_contracts_operations::processing_ops::{
    PreviewGatherProcessingRequest, PreviewSubvolumeProcessingRequest,
    PreviewTraceLocalProcessingRequest, RunGatherProcessingRequest, RunSubvolumeProcessingRequest,
    RunTraceLocalProcessingRequest, VelocityScanRequest,
};
use seis_contracts_operations::resolve::{
    IPC_SCHEMA_VERSION, ResolveSurveyMapRequest, SetDatasetNativeCoordinateReferenceRequest,
};
use seis_contracts_operations::workspace::{
    DescribeVelocityVolumeRequest, IngestVelocityVolumeRequest,
};
use seis_runtime::{
    IngestOptions, ProcessingExecutionMode, SeisGeometryOptions, SparseSurveyPolicy,
    TimeDepthDomain, ValidationOptions, VelocityQuantityKind, ingest_segy, inspect_segy,
    open_store, preflight_segy, run_validation,
};
#[derive(Debug, Parser)]
#[command(name = "traceboost-app")]
#[command(about = "Thin app-side shell for TraceBoost, backed by the in-repo runtime layer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    BackendInfo,
    OperationCatalog,
    DatasetOperatorCatalog {
        store: PathBuf,
    },
    Inspect {
        input: PathBuf,
    },
    Analyze {
        input: PathBuf,
        #[arg(long)]
        inline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        inline_type: HeaderTypeArg,
        #[arg(long)]
        crossline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        crossline_type: HeaderTypeArg,
        #[arg(long)]
        third_axis_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        third_axis_type: HeaderTypeArg,
    },
    Ingest {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, value_delimiter = ',')]
        chunk: Vec<usize>,
        #[arg(long)]
        inline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        inline_type: HeaderTypeArg,
        #[arg(long)]
        crossline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        crossline_type: HeaderTypeArg,
        #[arg(long)]
        third_axis_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        third_axis_type: HeaderTypeArg,
        #[arg(long)]
        regularize_sparse: bool,
        #[arg(long, default_value_t = 0.0)]
        fill_value: f32,
    },
    Validate {
        output: PathBuf,
        #[arg(long = "input")]
        inputs: Vec<PathBuf>,
    },
    PreflightImport {
        input: PathBuf,
        #[arg(long)]
        inline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        inline_type: HeaderTypeArg,
        #[arg(long)]
        crossline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        crossline_type: HeaderTypeArg,
        #[arg(long)]
        third_axis_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        third_axis_type: HeaderTypeArg,
    },
    ImportDataset {
        input: PathBuf,
        output: PathBuf,
        #[arg(long)]
        inline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        inline_type: HeaderTypeArg,
        #[arg(long)]
        crossline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        crossline_type: HeaderTypeArg,
        #[arg(long)]
        third_axis_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        third_axis_type: HeaderTypeArg,
        #[arg(long, default_value_t = false)]
        overwrite_existing: bool,
    },
    ImportPrestackOffsetDataset {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite_existing: bool,
    },
    OpenDataset {
        store: PathBuf,
    },
    SetNativeCoordinateReference {
        store: PathBuf,
        #[arg(long)]
        coordinate_reference_id: Option<String>,
        #[arg(long)]
        coordinate_reference_name: Option<String>,
    },
    ResolveSurveyMap {
        store: PathBuf,
        #[arg(long)]
        display_coordinate_reference_id: Option<String>,
    },
    ExportSegy {
        store: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite_existing: bool,
    },
    ExportZarr {
        store: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite_existing: bool,
    },
    ImportHorizons {
        store: PathBuf,
        #[arg(long, value_enum)]
        vertical_domain: Option<VerticalDomainArg>,
        #[arg(long)]
        vertical_unit: Option<String>,
        #[arg(long)]
        source_coordinate_reference_id: Option<String>,
        #[arg(long)]
        source_coordinate_reference_name: Option<String>,
        #[arg(long, default_value_t = false)]
        assume_same_as_survey: bool,
        inputs: Vec<PathBuf>,
    },
    ViewSection {
        store: PathBuf,
        #[arg(value_enum)]
        axis: SectionAxisArg,
        index: usize,
    },
    PreviewProcessing {
        request_json: String,
    },
    RunProcessing {
        request_json: String,
    },
    BenchmarkTraceLocalProcessing {
        store: PathBuf,
        #[arg(long)]
        output_root: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = TraceLocalBenchmarkScenarioArg::Agc)]
        scenario: TraceLocalBenchmarkScenarioArg,
        #[arg(long = "partition-target-mib", value_delimiter = ',')]
        partition_target_mib: Vec<u64>,
        #[arg(long, default_value_t = false)]
        adaptive_partition_target: bool,
        #[arg(long, default_value_t = true)]
        include_serial: bool,
        #[arg(long, default_value_t = 1)]
        repeat_count: usize,
        #[arg(long, default_value_t = false)]
        keep_outputs: bool,
    },
    BenchmarkTraceLocalBatchProcessing {
        store: PathBuf,
        #[arg(long)]
        output_root: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = TraceLocalBenchmarkScenarioArg::Agc)]
        scenario: TraceLocalBenchmarkScenarioArg,
        #[arg(long, default_value_t = 4)]
        job_count: usize,
        #[arg(long = "max-active-jobs", value_delimiter = ',')]
        max_active_jobs: Vec<usize>,
        #[arg(long, value_enum)]
        execution_mode: Option<BatchExecutionModeArg>,
        #[arg(long, default_value_t = 64)]
        partition_target_mib: u64,
        #[arg(long, default_value_t = false)]
        adaptive_partition_target: bool,
        #[arg(long, default_value_t = 1)]
        repeat_count: usize,
        #[arg(long, default_value_t = false)]
        keep_outputs: bool,
    },
    BenchmarkPostStackNeighborhoodPreview {
        store: PathBuf,
        #[arg(long, value_enum, default_value_t = NeighborhoodBenchmarkOperatorArg::Similarity)]
        operator: NeighborhoodBenchmarkOperatorArg,
        #[arg(long, default_value_t = 24.0)]
        gate_ms: f32,
        #[arg(long, default_value_t = 1)]
        inline_stepout: usize,
        #[arg(long, default_value_t = 1)]
        xline_stepout: usize,
        #[arg(long, value_enum, default_value_t = SectionAxisArg::Inline)]
        axis: SectionAxisArg,
        #[arg(long, default_value_t = 0)]
        section_index: usize,
        #[arg(long, default_value_t = false)]
        include_trace_local_prefix: bool,
        #[arg(long, default_value_t = 1)]
        repeat_count: usize,
    },
    BenchmarkPostStackNeighborhoodProcessing {
        store: PathBuf,
        #[arg(long)]
        output_root: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = NeighborhoodBenchmarkOperatorArg::Similarity)]
        operator: NeighborhoodBenchmarkOperatorArg,
        #[arg(long, default_value_t = 24.0)]
        gate_ms: f32,
        #[arg(long, default_value_t = 1)]
        inline_stepout: usize,
        #[arg(long, default_value_t = 1)]
        xline_stepout: usize,
        #[arg(long, default_value_t = false)]
        include_trace_local_prefix: bool,
        #[arg(long, default_value_t = 1)]
        repeat_count: usize,
        #[arg(long, default_value_t = false)]
        keep_outputs: bool,
    },
    PreviewSubvolumeProcessing {
        request_json: String,
    },
    RunSubvolumeProcessing {
        request_json: String,
    },
    PreviewGatherProcessing {
        request_json: String,
    },
    RunGatherProcessing {
        request_json: String,
    },
    RunVelocityScan {
        request_json: String,
    },
    ViewSectionHorizons {
        store: PathBuf,
        #[arg(value_enum)]
        axis: SectionAxisArg,
        index: usize,
    },
    LoadVelocityModels {
        store: PathBuf,
    },
    EnsureDemoSurveyTimeDepthTransform {
        store: PathBuf,
    },
    PrepareSurveyDemo {
        store: PathBuf,
        #[arg(long)]
        display_coordinate_reference_id: Option<String>,
    },
    BuildPairedHorizonTransform {
        store: PathBuf,
        #[arg(long, value_delimiter = ',')]
        time_horizon_ids: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        depth_horizon_ids: Vec<String>,
        #[arg(long)]
        output_id: Option<String>,
        #[arg(long)]
        output_name: Option<String>,
    },
    ConvertHorizonDomain {
        store: PathBuf,
        #[arg(long)]
        source_horizon_id: String,
        #[arg(long)]
        transform_id: String,
        #[arg(long, value_enum)]
        target_domain: VerticalDomainArg,
        #[arg(long)]
        output_id: Option<String>,
        #[arg(long)]
        output_name: Option<String>,
    },
    ImportVelocityFunctionsModel {
        store: PathBuf,
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = VelocityKindArg::Interval)]
        velocity_kind: VelocityKindArg,
    },
    DescribeVelocityVolume {
        store: PathBuf,
        #[arg(long, value_enum, default_value_t = VelocityKindArg::Interval)]
        velocity_kind: VelocityKindArg,
        #[arg(long, value_enum)]
        vertical_domain: Option<VerticalDomainArg>,
        #[arg(long)]
        vertical_unit: Option<String>,
        #[arg(long)]
        vertical_start: Option<f32>,
        #[arg(long)]
        vertical_step: Option<f32>,
    },
    IngestVelocityVolume {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, value_enum, default_value_t = VelocityKindArg::Interval)]
        velocity_kind: VelocityKindArg,
        #[arg(long)]
        inline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        inline_type: HeaderTypeArg,
        #[arg(long)]
        crossline_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        crossline_type: HeaderTypeArg,
        #[arg(long)]
        third_axis_byte: Option<u16>,
        #[arg(long, value_enum, default_value_t = HeaderTypeArg::I32)]
        third_axis_type: HeaderTypeArg,
        #[arg(long, value_enum, default_value_t = VerticalDomainArg::Time)]
        vertical_domain: VerticalDomainArg,
        #[arg(long)]
        vertical_unit: Option<String>,
        #[arg(long)]
        vertical_start: Option<f32>,
        #[arg(long)]
        vertical_step: Option<f32>,
        #[arg(long, default_value_t = false)]
        overwrite_existing: bool,
        #[arg(long, default_value_t = false)]
        delete_input_on_success: bool,
    },
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },
}

#[derive(Debug, Subcommand)]
enum WorkflowCommand {
    Validate {
        recipe: PathBuf,
    },
    Run {
        recipe: PathBuf,
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long)]
        run_id: Option<String>,
    },
    RenderReport {
        report: PathBuf,
        #[arg(long, value_enum, default_value_t = WorkflowReportFormatArg::Markdown)]
        format: WorkflowReportFormatArg,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HeaderTypeArg {
    I16,
    I32,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SectionAxisArg {
    Inline,
    Xline,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum VelocityKindArg {
    Interval,
    Average,
    Rms,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum VerticalDomainArg {
    Time,
    Depth,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BatchExecutionModeArg {
    Auto,
    Conservative,
    Throughput,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TraceLocalBenchmarkScenarioArg {
    Scalar,
    Agc,
    Analytic,
    Bandpass,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum NeighborhoodBenchmarkOperatorArg {
    Similarity,
    LocalVolumeStatsMean,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum WorkflowReportFormatArg {
    Json,
    Markdown,
    Mermaid,
    Html,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let workflows = TraceBoostWorkflowService;
    match cli.command {
        Command::BackendInfo => {
            println!(
                "{}",
                serde_json::to_string_pretty(&workflows.backend_info())?
            );
        }
        Command::OperationCatalog => {
            println!("{}", serde_json::to_string_pretty(operation_catalog())?);
        }
        Command::DatasetOperatorCatalog { store } => {
            let response = dataset_operator_catalog(&store)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::Inspect { input } => {
            println!("{}", serde_json::to_string_pretty(&inspect_segy(input)?)?);
        }
        Command::Analyze {
            input,
            inline_byte,
            inline_type,
            crossline_byte,
            crossline_type,
            third_axis_byte,
            third_axis_type,
        } => {
            let options = IngestOptions {
                geometry: build_ingest_geometry(
                    inline_byte,
                    inline_type,
                    crossline_byte,
                    crossline_type,
                    third_axis_byte,
                    third_axis_type,
                ),
                ..IngestOptions::default()
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&preflight_segy(input, &options)?)?
            );
        }
        Command::Ingest {
            input,
            output,
            chunk,
            inline_byte,
            inline_type,
            crossline_byte,
            crossline_type,
            third_axis_byte,
            third_axis_type,
            regularize_sparse,
            fill_value,
        } => {
            let handle = ingest_segy(
                input,
                output,
                IngestOptions {
                    chunk_shape: parse_chunk_shape(&chunk),
                    geometry: build_ingest_geometry(
                        inline_byte,
                        inline_type,
                        crossline_byte,
                        crossline_type,
                        third_axis_byte,
                        third_axis_type,
                    ),
                    sparse_survey_policy: if regularize_sparse {
                        SparseSurveyPolicy::RegularizeToDense { fill_value }
                    } else {
                        SparseSurveyPolicy::default()
                    },
                    ..IngestOptions::default()
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&handle.manifest)?);
        }
        Command::Validate { output, inputs } => {
            let summary = run_validation(ValidationOptions {
                output_dir: output,
                dataset_paths: inputs,
                validation_mode: seis_io::ValidationMode::Strict,
            })?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::PreflightImport {
            input,
            inline_byte,
            inline_type,
            crossline_byte,
            crossline_type,
            third_axis_byte,
            third_axis_type,
        } => {
            let response = workflows.preflight_dataset(SurveyPreflightRequest {
                schema_version: IPC_SCHEMA_VERSION,
                input_path: input.to_string_lossy().into_owned(),
                geometry_override: build_geometry_override(
                    inline_byte,
                    inline_type,
                    crossline_byte,
                    crossline_type,
                    third_axis_byte,
                    third_axis_type,
                ),
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ImportDataset {
            input,
            output,
            inline_byte,
            inline_type,
            crossline_byte,
            crossline_type,
            third_axis_byte,
            third_axis_type,
            overwrite_existing,
        } => {
            let response = workflows.import_dataset(ImportDatasetRequest {
                schema_version: IPC_SCHEMA_VERSION,
                input_path: input.to_string_lossy().into_owned(),
                output_store_path: output.to_string_lossy().into_owned(),
                geometry_override: build_geometry_override(
                    inline_byte,
                    inline_type,
                    crossline_byte,
                    crossline_type,
                    third_axis_byte,
                    third_axis_type,
                ),
                overwrite_existing,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ImportPrestackOffsetDataset {
            input,
            output,
            overwrite_existing,
        } => {
            let response =
                workflows.import_prestack_offset_dataset(ImportPrestackOffsetDatasetRequest {
                    schema_version: IPC_SCHEMA_VERSION,
                    input_path: input.to_string_lossy().into_owned(),
                    output_store_path: output.to_string_lossy().into_owned(),
                    third_axis_field: PrestackThirdAxisField::Offset,
                    overwrite_existing,
                })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::OpenDataset { store } => {
            let response = workflows.open_dataset_summary(OpenDatasetRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: store.to_string_lossy().into_owned(),
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::SetNativeCoordinateReference {
            store,
            coordinate_reference_id,
            coordinate_reference_name,
        } => {
            let response = workflows.set_dataset_native_coordinate_reference(
                SetDatasetNativeCoordinateReferenceRequest {
                    schema_version: IPC_SCHEMA_VERSION,
                    store_path: store.to_string_lossy().into_owned(),
                    coordinate_reference_id,
                    coordinate_reference_name,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ResolveSurveyMap {
            store,
            display_coordinate_reference_id,
        } => {
            let response = workflows.resolve_survey_map(ResolveSurveyMapRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: store.to_string_lossy().into_owned(),
                display_coordinate_reference_id,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ExportSegy {
            store,
            output,
            overwrite_existing,
        } => {
            let response = workflows.export_dataset_segy(ExportSegyRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: store.to_string_lossy().into_owned(),
                output_path: output.to_string_lossy().into_owned(),
                overwrite_existing,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ExportZarr {
            store,
            output,
            overwrite_existing,
        } => {
            let response = workflows.export_dataset_zarr(
                store.to_string_lossy().into_owned(),
                output.to_string_lossy().into_owned(),
                overwrite_existing,
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ImportHorizons {
            store,
            vertical_domain,
            vertical_unit,
            source_coordinate_reference_id,
            source_coordinate_reference_name,
            assume_same_as_survey,
            inputs,
        } => {
            let response = import_horizon_xyz(ImportHorizonXyzRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: store.to_string_lossy().into_owned(),
                input_paths: inputs
                    .into_iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect(),
                vertical_domain: vertical_domain.map(Into::into),
                vertical_unit,
                source_coordinate_reference_id,
                source_coordinate_reference_name,
                assume_same_as_survey,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ViewSection { store, axis, index } => {
            let view = open_store(store)?.section_view(axis.into(), index)?;
            println!("{}", serde_json::to_string(&view)?);
        }
        Command::PreviewProcessing { request_json } => {
            let request: PreviewTraceLocalProcessingRequest = read_json_arg(&request_json)?;
            let response = preview_processing(request)?;
            println!("{}", serde_json::to_string(&response)?);
        }
        Command::RunProcessing { request_json } => {
            let request: RunTraceLocalProcessingRequest = read_json_arg(&request_json)?;
            let response = apply_processing(request)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::BenchmarkTraceLocalProcessing {
            store,
            output_root,
            scenario,
            partition_target_mib,
            adaptive_partition_target,
            include_serial,
            repeat_count,
            keep_outputs,
        } => {
            let response = benchmark_trace_local_processing(TraceLocalBenchmarkRequest {
                store_path: store.to_string_lossy().into_owned(),
                output_root: output_root.map(|path| path.to_string_lossy().into_owned()),
                scenario: scenario.into(),
                partition_target_mib,
                adaptive_partition_target,
                include_serial,
                repeat_count,
                keep_outputs,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::BenchmarkTraceLocalBatchProcessing {
            store,
            output_root,
            scenario,
            job_count,
            max_active_jobs,
            execution_mode,
            partition_target_mib,
            adaptive_partition_target,
            repeat_count,
            keep_outputs,
        } => {
            let response =
                benchmark_trace_local_batch_processing(TraceLocalBatchBenchmarkRequest {
                    store_path: store.to_string_lossy().into_owned(),
                    output_root: output_root.map(|path| path.to_string_lossy().into_owned()),
                    scenario: scenario.into(),
                    job_count,
                    max_active_jobs,
                    execution_mode: execution_mode.map(Into::into),
                    partition_target_mib,
                    adaptive_partition_target,
                    repeat_count,
                    keep_outputs,
                })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::BenchmarkPostStackNeighborhoodPreview {
            store,
            operator,
            gate_ms,
            inline_stepout,
            xline_stepout,
            axis,
            section_index,
            include_trace_local_prefix,
            repeat_count,
        } => {
            let response = benchmark_post_stack_neighborhood_preview(
                PostStackNeighborhoodPreviewBenchmarkRequest {
                    store_path: store.to_string_lossy().into_owned(),
                    operator: operator.into(),
                    gate_ms,
                    inline_stepout,
                    xline_stepout,
                    section_axis: axis.into(),
                    section_index,
                    include_trace_local_prefix,
                    repeat_count,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::BenchmarkPostStackNeighborhoodProcessing {
            store,
            output_root,
            operator,
            gate_ms,
            inline_stepout,
            xline_stepout,
            include_trace_local_prefix,
            repeat_count,
            keep_outputs,
        } => {
            let response = benchmark_post_stack_neighborhood_processing(
                PostStackNeighborhoodProcessingBenchmarkRequest {
                    store_path: store.to_string_lossy().into_owned(),
                    output_root: output_root.map(|path| path.to_string_lossy().into_owned()),
                    operator: operator.into(),
                    gate_ms,
                    inline_stepout,
                    xline_stepout,
                    include_trace_local_prefix,
                    repeat_count,
                    keep_outputs,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::PreviewSubvolumeProcessing { request_json } => {
            let request: PreviewSubvolumeProcessingRequest = read_json_arg(&request_json)?;
            let response = preview_subvolume_processing(request)?;
            println!("{}", serde_json::to_string(&response)?);
        }
        Command::RunSubvolumeProcessing { request_json } => {
            let request: RunSubvolumeProcessingRequest = read_json_arg(&request_json)?;
            let response = apply_subvolume_processing(request)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::PreviewGatherProcessing { request_json } => {
            let request: PreviewGatherProcessingRequest = read_json_arg(&request_json)?;
            let response = preview_gather_processing(request)?;
            println!("{}", serde_json::to_string(&response)?);
        }
        Command::RunGatherProcessing { request_json } => {
            let request: RunGatherProcessingRequest = read_json_arg(&request_json)?;
            let response = apply_gather_processing(request)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::RunVelocityScan { request_json } => {
            let request: VelocityScanRequest = read_json_arg(&request_json)?;
            let response = run_velocity_scan(request)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ViewSectionHorizons { store, axis, index } => {
            let response = workflows.load_section_horizons(LoadSectionHorizonsRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: store.to_string_lossy().into_owned(),
                axis: axis.into(),
                index,
            })?;
            println!("{}", serde_json::to_string(&response)?);
        }
        Command::LoadVelocityModels { store } => {
            let response = workflows.load_velocity_models(store.to_string_lossy().into_owned())?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::EnsureDemoSurveyTimeDepthTransform { store } => {
            let response = workflows
                .ensure_demo_survey_time_depth_transform(store.to_string_lossy().into_owned())?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::PrepareSurveyDemo {
            store,
            display_coordinate_reference_id,
        } => {
            let response = workflows.prepare_survey_demo(PrepareSurveyDemoRequest {
                store_path: store.to_string_lossy().into_owned(),
                display_coordinate_reference_id,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::BuildPairedHorizonTransform {
            store,
            time_horizon_ids,
            depth_horizon_ids,
            output_id,
            output_name,
        } => {
            let response = workflows.build_paired_horizon_transform(
                store.to_string_lossy().into_owned(),
                time_horizon_ids,
                depth_horizon_ids,
                output_id,
                output_name,
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ConvertHorizonDomain {
            store,
            source_horizon_id,
            transform_id,
            target_domain,
            output_id,
            output_name,
        } => {
            let response = workflows.convert_horizon_domain(
                store.to_string_lossy().into_owned(),
                source_horizon_id,
                transform_id,
                target_domain.into(),
                output_id,
                output_name,
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::ImportVelocityFunctionsModel {
            store,
            input,
            velocity_kind,
        } => {
            let response = workflows.import_velocity_functions_model(
                store.to_string_lossy().into_owned(),
                input.to_string_lossy().into_owned(),
                velocity_kind.into(),
            )?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::DescribeVelocityVolume {
            store,
            velocity_kind,
            vertical_domain,
            vertical_unit,
            vertical_start,
            vertical_step,
        } => {
            let response = workflows.describe_velocity_volume(DescribeVelocityVolumeRequest {
                schema_version: IPC_SCHEMA_VERSION,
                store_path: store.to_string_lossy().into_owned(),
                velocity_kind: velocity_kind.into(),
                vertical_domain: vertical_domain.map(Into::into),
                vertical_unit,
                vertical_start,
                vertical_step,
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::IngestVelocityVolume {
            input,
            output,
            velocity_kind,
            inline_byte,
            inline_type,
            crossline_byte,
            crossline_type,
            third_axis_byte,
            third_axis_type,
            vertical_domain,
            vertical_unit,
            vertical_start,
            vertical_step,
            overwrite_existing,
            delete_input_on_success,
        } => {
            let response =
                workflows.ingest_velocity_volume_request(IngestVelocityVolumeRequest {
                    schema_version: IPC_SCHEMA_VERSION,
                    input_path: input.to_string_lossy().into_owned(),
                    output_store_path: output.to_string_lossy().into_owned(),
                    velocity_kind: velocity_kind.into(),
                    vertical_domain: vertical_domain.into(),
                    vertical_unit,
                    vertical_start,
                    vertical_step,
                    overwrite_existing,
                    delete_input_on_success,
                    geometry_override: build_geometry_override(
                        inline_byte,
                        inline_type,
                        crossline_byte,
                        crossline_type,
                        third_axis_byte,
                        third_axis_type,
                    ),
                })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::Workflow { command } => match command {
            WorkflowCommand::Validate { recipe } => {
                let recipe = load_workflow_recipe_from_json_path(&recipe)?;
                validate_workflow_recipe(&recipe)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "valid",
                        "schema_version": recipe.schema_version,
                        "recipe_id": recipe.recipe_id,
                        "step_count": recipe.steps.len()
                    }))?
                );
            }
            WorkflowCommand::Run {
                recipe,
                report,
                run_id,
            } => {
                let recipe = load_workflow_recipe_from_json_path(&recipe)?;
                let workflow_report = run_workflow_recipe(
                    &recipe,
                    RunWorkflowOptions {
                        run_id: run_id.unwrap_or_else(default_workflow_run_id),
                        started_at: default_workflow_timestamp(),
                        app_version: env!("CARGO_PKG_VERSION").to_string(),
                        os: Some(std::env::consts::OS.to_string()),
                        arch: Some(std::env::consts::ARCH.to_string()),
                        host: None,
                        environment_variables: Vec::new(),
                    },
                )?;
                if let Some(report_path) = report {
                    fs::write(
                        &report_path,
                        serde_json::to_string_pretty(&workflow_report)?,
                    )?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "status": "written",
                            "report_path": report_path.to_string_lossy(),
                            "run_id": workflow_report.run_id,
                            "recipe_id": workflow_report.recipe_id
                        }))?
                    );
                } else {
                    println!("{}", serde_json::to_string_pretty(&workflow_report)?);
                }
            }
            WorkflowCommand::RenderReport { report, format } => {
                let report: WorkflowRunReport = serde_json::from_str(&fs::read_to_string(report)?)?;
                match format {
                    WorkflowReportFormatArg::Json => {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                    WorkflowReportFormatArg::Markdown => {
                        print!("{}", render_workflow_report_markdown(&report));
                    }
                    WorkflowReportFormatArg::Mermaid => {
                        print!("{}", render_workflow_report_mermaid(&report));
                    }
                    WorkflowReportFormatArg::Html => {
                        return Err("workflow HTML report rendering is not implemented yet".into());
                    }
                }
            }
        },
    }

    Ok(())
}

fn default_workflow_run_id() -> String {
    format!(
        "traceboost-workflow-{}",
        unix_timestamp_seconds().unwrap_or_default()
    )
}

fn default_workflow_timestamp() -> String {
    format!("unix:{}", unix_timestamp_seconds().unwrap_or_default())
}

fn unix_timestamp_seconds() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

impl From<SectionAxisArg> for seis_runtime::SectionAxis {
    fn from(value: SectionAxisArg) -> Self {
        match value {
            SectionAxisArg::Inline => Self::Inline,
            SectionAxisArg::Xline => Self::Xline,
        }
    }
}

impl From<VelocityKindArg> for VelocityQuantityKind {
    fn from(value: VelocityKindArg) -> Self {
        match value {
            VelocityKindArg::Interval => Self::Interval,
            VelocityKindArg::Average => Self::Average,
            VelocityKindArg::Rms => Self::Rms,
        }
    }
}

impl From<VerticalDomainArg> for TimeDepthDomain {
    fn from(value: VerticalDomainArg) -> Self {
        match value {
            VerticalDomainArg::Time => Self::Time,
            VerticalDomainArg::Depth => Self::Depth,
        }
    }
}

impl From<TraceLocalBenchmarkScenarioArg> for TraceLocalBenchmarkScenario {
    fn from(value: TraceLocalBenchmarkScenarioArg) -> Self {
        match value {
            TraceLocalBenchmarkScenarioArg::Scalar => Self::Scalar,
            TraceLocalBenchmarkScenarioArg::Agc => Self::Agc,
            TraceLocalBenchmarkScenarioArg::Analytic => Self::Analytic,
            TraceLocalBenchmarkScenarioArg::Bandpass => Self::Bandpass,
        }
    }
}

impl From<NeighborhoodBenchmarkOperatorArg> for PostStackNeighborhoodBenchmarkOperator {
    fn from(value: NeighborhoodBenchmarkOperatorArg) -> Self {
        match value {
            NeighborhoodBenchmarkOperatorArg::Similarity => Self::Similarity,
            NeighborhoodBenchmarkOperatorArg::LocalVolumeStatsMean => Self::LocalVolumeStatsMean,
        }
    }
}

impl From<BatchExecutionModeArg> for ProcessingExecutionMode {
    fn from(value: BatchExecutionModeArg) -> Self {
        match value {
            BatchExecutionModeArg::Auto => ProcessingExecutionMode::Auto,
            BatchExecutionModeArg::Conservative => ProcessingExecutionMode::Conservative,
            BatchExecutionModeArg::Throughput => ProcessingExecutionMode::Throughput,
        }
    }
}

fn parse_chunk_shape(values: &[usize]) -> [usize; 3] {
    match values {
        [a, b, c] => [*a, *b, *c],
        _ => [0, 0, 0],
    }
}

fn read_json_arg<T>(path_or_stdin: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: serde::de::DeserializeOwned,
{
    let text = if path_or_stdin == "-" {
        use std::io::Read;

        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        fs::read_to_string(path_or_stdin)?
    };
    Ok(serde_json::from_str(&text)?)
}

fn build_ingest_geometry(
    inline_byte: Option<u16>,
    inline_type: HeaderTypeArg,
    crossline_byte: Option<u16>,
    crossline_type: HeaderTypeArg,
    third_axis_byte: Option<u16>,
    third_axis_type: HeaderTypeArg,
) -> SeisGeometryOptions {
    let mut geometry = SeisGeometryOptions::default();
    geometry.header_mapping.inline_3d =
        inline_byte.map(|start_byte| header_field("INLINE_3D", start_byte, inline_type));
    geometry.header_mapping.crossline_3d =
        crossline_byte.map(|start_byte| header_field("CROSSLINE_3D", start_byte, crossline_type));
    geometry.third_axis_field =
        third_axis_byte.map(|start_byte| header_field("THIRD_AXIS", start_byte, third_axis_type));
    geometry
}

fn build_geometry_override(
    inline_byte: Option<u16>,
    inline_type: HeaderTypeArg,
    crossline_byte: Option<u16>,
    crossline_type: HeaderTypeArg,
    third_axis_byte: Option<u16>,
    third_axis_type: HeaderTypeArg,
) -> Option<SegyGeometryOverride> {
    let geometry = SegyGeometryOverride {
        inline_3d: inline_byte.map(|start_byte| SegyHeaderField {
            start_byte,
            value_type: segy_header_value_type(inline_type),
        }),
        crossline_3d: crossline_byte.map(|start_byte| SegyHeaderField {
            start_byte,
            value_type: segy_header_value_type(crossline_type),
        }),
        third_axis: third_axis_byte.map(|start_byte| SegyHeaderField {
            start_byte,
            value_type: segy_header_value_type(third_axis_type),
        }),
    };
    if geometry.inline_3d.is_none()
        && geometry.crossline_3d.is_none()
        && geometry.third_axis.is_none()
    {
        None
    } else {
        Some(geometry)
    }
}

fn segy_header_value_type(value_type: HeaderTypeArg) -> SegyHeaderValueType {
    match value_type {
        HeaderTypeArg::I16 => SegyHeaderValueType::I16,
        HeaderTypeArg::I32 => SegyHeaderValueType::I32,
    }
}

fn header_field(
    name: &'static str,
    start_byte: u16,
    value_type: HeaderTypeArg,
) -> seis_io::HeaderField {
    match value_type {
        HeaderTypeArg::I16 => seis_io::HeaderField::new_i16(name, start_byte),
        HeaderTypeArg::I32 => seis_io::HeaderField::new_i32(name, start_byte),
    }
}
