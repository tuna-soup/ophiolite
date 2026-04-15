mod operation_catalog;

use std::path::PathBuf;

use traceboost_app::{PrepareSurveyDemoRequest, TraceBoostWorkflowService, import_horizon_xyz};

use clap::{Parser, Subcommand, ValueEnum};
use operation_catalog::operation_catalog;
use seis_contracts_operations::datasets::OpenDatasetRequest;
use seis_contracts_operations::import_ops::{
    ExportSegyRequest, ImportDatasetRequest, ImportHorizonXyzRequest, LoadSectionHorizonsRequest,
    SegyGeometryOverride, SegyHeaderField, SegyHeaderValueType, SurveyPreflightRequest,
};
use seis_contracts_operations::resolve::{
    IPC_SCHEMA_VERSION, ResolveSurveyMapRequest, SetDatasetNativeCoordinateReferenceRequest,
};
use seis_runtime::{
    IngestOptions, SeisGeometryOptions, SparseSurveyPolicy, TimeDepthDomain, ValidationOptions,
    VelocityQuantityKind, ingest_segy, inspect_segy, open_store, preflight_segy, run_validation,
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
    }

    Ok(())
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

fn parse_chunk_shape(values: &[usize]) -> [usize; 3] {
    match values {
        [a, b, c] => [*a, *b, *c],
        _ => [0, 0, 0],
    }
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
