use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use seis_runtime::{
    IngestOptions, InterpMethod, SectionAxis, SeisGeometryOptions, SparseSurveyPolicy,
    UpscaleOptions, describe_store, describe_tbvol_archive_sibling, ingest_segy, inspect_segy,
    preflight_segy, render_section_csv, run_validation, suggested_tbvol_restore_path,
    suggested_tbvolc_archive_path, transcode_tbvol_to_tbvolc, transcode_tbvolc_to_tbvol,
    upscale_store,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "seis-runtime")]
#[command(about = "TraceBoost runtime for SEG-Y ingest, working-store creation, and refinement")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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
        chunk: Option<Vec<usize>>,
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
    Upscale {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = 2)]
        scale: u8,
        #[arg(long, value_enum, default_value_t = MethodArg::Linear)]
        method: MethodArg,
        #[arg(long, value_delimiter = ',')]
        chunk: Option<Vec<usize>>,
    },
    Validate {
        output: PathBuf,
        #[arg(long = "input")]
        inputs: Vec<PathBuf>,
    },
    Render {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, value_enum)]
        axis: AxisArg,
        #[arg(long)]
        index: usize,
    },
    ArchiveStatus {
        input: PathBuf,
    },
    ArchiveCreate {
        input: PathBuf,
        output: Option<PathBuf>,
    },
    ArchiveRestore {
        input: PathBuf,
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Serialize)]
struct ArchiveRestoreSummary {
    archive_root: PathBuf,
    working_store_root: PathBuf,
    volume: seis_runtime::VolumeDescriptor,
}

#[derive(Debug, Serialize)]
struct ArchiveCreateSummary {
    working_store_root: PathBuf,
    archive_root: PathBuf,
    is_default_sibling: bool,
    default_sibling_status: Option<seis_runtime::TbvolArchiveSiblingStatus>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AxisArg {
    Inline,
    Xline,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MethodArg {
    Linear,
    Cubic,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HeaderTypeArg {
    I16,
    I32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
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
        Command::Upscale {
            input,
            output,
            scale,
            method,
            chunk,
        } => {
            let handle = upscale_store(
                input,
                output,
                UpscaleOptions {
                    scale,
                    method: method.into(),
                    chunk_shape: parse_chunk_shape(&chunk),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&handle.manifest)?);
        }
        Command::Validate { output, inputs } => {
            let summary = run_validation(seis_runtime::ValidationOptions {
                output_dir: output,
                dataset_paths: inputs,
                validation_mode: seis_io::ValidationMode::Strict,
            })?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Render {
            input,
            output,
            axis,
            index,
        } => {
            render_section_csv(input, axis.into(), index, output)?;
        }
        Command::ArchiveStatus { input } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&describe_tbvol_archive_sibling(input)?)?
            );
        }
        Command::ArchiveCreate { input, output } => {
            let default_output = suggested_tbvolc_archive_path(&input);
            let output = output.unwrap_or_else(|| default_output.clone());
            transcode_tbvol_to_tbvolc(&input, &output)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&ArchiveCreateSummary {
                    working_store_root: input.clone(),
                    archive_root: output.clone(),
                    is_default_sibling: output == default_output,
                    default_sibling_status: (output == default_output)
                        .then(|| describe_tbvol_archive_sibling(&input))
                        .transpose()?,
                })?
            );
        }
        Command::ArchiveRestore { input, output } => {
            let output = output.unwrap_or_else(|| suggested_tbvol_restore_path(&input));
            transcode_tbvolc_to_tbvol(&input, &output)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&ArchiveRestoreSummary {
                    archive_root: input,
                    working_store_root: output.clone(),
                    volume: describe_store(&output)?,
                })?
            );
        }
    }

    Ok(())
}

fn parse_chunk_shape(values: &Option<Vec<usize>>) -> [usize; 3] {
    match values.as_deref() {
        Some([a, b, c]) => [*a, *b, *c],
        _ => [0, 0, 0],
    }
}

impl From<AxisArg> for SectionAxis {
    fn from(value: AxisArg) -> Self {
        match value {
            AxisArg::Inline => SectionAxis::Inline,
            AxisArg::Xline => SectionAxis::Xline,
        }
    }
}

impl From<MethodArg> for InterpMethod {
    fn from(value: MethodArg) -> Self {
        match value {
            MethodArg::Linear => InterpMethod::Linear,
            MethodArg::Cubic => InterpMethod::Cubic,
        }
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
