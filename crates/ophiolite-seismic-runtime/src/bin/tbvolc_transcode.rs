use clap::{Parser, Subcommand};
use ophiolite_seismic_runtime::{
    SeismicStoreError, transcode_tbvol_to_tbvolc, transcode_tbvolc_to_tbvol,
};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "tbvolc-transcode")]
#[command(about = "Offline exact tbvol <-> tbvolc transcoder")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Encode {
        input_tbvol: PathBuf,
        output_tbvolc: PathBuf,
    },
    Decode {
        input_tbvolc: PathBuf,
        output_tbvol: PathBuf,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), SeismicStoreError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Encode {
            input_tbvol,
            output_tbvolc,
        } => transcode_tbvol_to_tbvolc(input_tbvol, output_tbvolc),
        Command::Decode {
            input_tbvolc,
            output_tbvol,
        } => transcode_tbvolc_to_tbvol(input_tbvolc, output_tbvol),
    }
}
