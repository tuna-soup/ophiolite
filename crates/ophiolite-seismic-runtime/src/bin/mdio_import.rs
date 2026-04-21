use clap::Parser;
use ophiolite_seismic_runtime::{SeismicStoreError, VolumeSubset, ingest_mdio_store, open_store};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "mdio-import")]
#[command(about = "Import an MDIO/Zarr seismic store into tbvol, optionally as an ROI subset")]
struct Cli {
    input_mdio: PathBuf,
    output_tbvol: PathBuf,
    #[arg(long, num_args = 3, value_names = ["INLINE", "XLINE", "SAMPLE"])]
    chunk_shape: Option<Vec<usize>>,
    #[arg(long)]
    inline_start: Option<usize>,
    #[arg(long)]
    inline_count: Option<usize>,
    #[arg(long)]
    xline_start: Option<usize>,
    #[arg(long)]
    xline_count: Option<usize>,
    #[arg(long)]
    sample_start: Option<usize>,
    #[arg(long)]
    sample_count: Option<usize>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), SeismicStoreError> {
    let cli = Cli::parse();
    let subset = parse_subset(&cli)?;
    let chunk_shape = parse_chunk_shape(cli.chunk_shape)?;
    let handle = ingest_mdio_store(&cli.input_mdio, &cli.output_tbvol, chunk_shape, subset)?;
    let opened = open_store(&handle.root)?;
    let descriptor = opened.volume_descriptor();
    println!("tbvol: {}", opened.root.display());
    println!("shape: {:?}", descriptor.shape);
    println!("chunk_shape: {:?}", descriptor.chunk_shape);
    println!(
        "vertical_axis: {:?} ({})",
        opened.manifest.volume.axes.sample_axis_domain,
        opened.manifest.volume.axes.sample_axis_unit
    );
    println!(
        "inline_range: {}..={}",
        opened
            .manifest
            .volume
            .axes
            .ilines
            .first()
            .copied()
            .unwrap_or_default(),
        opened
            .manifest
            .volume
            .axes
            .ilines
            .last()
            .copied()
            .unwrap_or_default()
    );
    println!(
        "xline_range: {}..={}",
        opened
            .manifest
            .volume
            .axes
            .xlines
            .first()
            .copied()
            .unwrap_or_default(),
        opened
            .manifest
            .volume
            .axes
            .xlines
            .last()
            .copied()
            .unwrap_or_default()
    );
    println!(
        "sample_range: {}..={}",
        opened
            .manifest
            .volume
            .axes
            .sample_axis_ms
            .first()
            .copied()
            .unwrap_or_default(),
        opened
            .manifest
            .volume
            .axes
            .sample_axis_ms
            .last()
            .copied()
            .unwrap_or_default()
    );
    println!(
        "spatial_descriptor: {}",
        if opened.manifest.volume.spatial.is_some() {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "occupancy: {}",
        if opened.manifest.has_occupancy {
            "present"
        } else {
            "missing"
        }
    );
    Ok(())
}

fn parse_chunk_shape(values: Option<Vec<usize>>) -> Result<[usize; 3], SeismicStoreError> {
    match values {
        Some(values) => {
            let values: [usize; 3] = values.try_into().map_err(|_| {
                SeismicStoreError::Message(
                    "--chunk-shape expects exactly three integers".to_string(),
                )
            })?;
            if values.iter().any(|value| *value == 0) {
                return Err(SeismicStoreError::Message(
                    "--chunk-shape values must be greater than zero".to_string(),
                ));
            }
            Ok(values)
        }
        None => Ok([0, 0, 0]),
    }
}

fn parse_subset(cli: &Cli) -> Result<Option<VolumeSubset>, SeismicStoreError> {
    let inline = range_pair(cli.inline_start, cli.inline_count, "inline")?;
    let xline = range_pair(cli.xline_start, cli.xline_count, "xline")?;
    let sample = range_pair(cli.sample_start, cli.sample_count, "sample")?;
    match (inline, xline, sample) {
        (None, None, None) => Ok(None),
        (
            Some((inline_start, inline_count)),
            Some((xline_start, xline_count)),
            Some((sample_start, sample_count)),
        ) => Ok(Some(VolumeSubset {
            inline_start,
            inline_count,
            xline_start,
            xline_count,
            sample_start,
            sample_count,
        })),
        _ => Err(SeismicStoreError::Message(
            "provide all --*-start/--*-count ROI arguments together or omit all of them"
                .to_string(),
        )),
    }
}

fn range_pair(
    start: Option<usize>,
    count: Option<usize>,
    axis_name: &str,
) -> Result<Option<(usize, usize)>, SeismicStoreError> {
    match (start, count) {
        (None, None) => Ok(None),
        (Some(start), Some(count)) if count > 0 => Ok(Some((start, count))),
        (Some(_), Some(_)) => Err(SeismicStoreError::Message(format!(
            "{axis_name} ROI count must be greater than zero"
        ))),
        _ => Err(SeismicStoreError::Message(format!(
            "missing {axis_name} ROI start/count pair"
        ))),
    }
}
