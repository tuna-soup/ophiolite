use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Instant;

use seis_io::{
    ChunkReadConfig, CubeChunkShape, IoStrategy, ReaderOptions, SegyReader, TraceSelection, open,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let reader = open(&args.path, ReaderOptions::default())?;

    println!("file: {}", args.path.display());
    println!("mode: {}", args.mode.as_str());
    println!("iterations: {}", args.iterations);
    println!("cached: {}", args.cached);
    println!("io: {}", args.io_strategy.as_str());
    println!(
        "note: true cold-read benchmarking still requires external cache flushing between runs"
    );

    match args.mode {
        Mode::Inspect => bench_inspect(&args.path, args.iterations)?,
        Mode::Headers => bench_headers(&reader, args.iterations)?,
        Mode::Ingest => bench_ingest(&reader, args.iterations, args.io_strategy)?,
        Mode::AssembleCube => bench_assemble_cube(&reader, args.iterations)?,
        Mode::InlineSeq | Mode::InlineRand | Mode::CrosslineSeq | Mode::CrosslineRand => {
            let index = RegularVolumeIndex::build(&reader)?;
            bench_slices(&reader, &index, &args)?;
        }
        Mode::All => {
            bench_inspect(&args.path, args.iterations)?;
            bench_headers(&reader, args.iterations)?;
            bench_ingest(&reader, args.iterations, args.io_strategy)?;
            bench_assemble_cube(&reader, args.iterations)?;

            if let Ok(index) = RegularVolumeIndex::build(&reader) {
                for mode in [
                    Mode::InlineSeq,
                    Mode::InlineRand,
                    Mode::CrosslineSeq,
                    Mode::CrosslineRand,
                ] {
                    let slice_args = Args {
                        mode,
                        ..args.clone()
                    };
                    bench_slices(&reader, &index, &slice_args)?;
                }
            } else {
                println!("slice benchmarks skipped: file is not a regular post-stack cube");
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
struct Args {
    path: PathBuf,
    mode: Mode,
    iterations: usize,
    cached: bool,
    slice_count: usize,
    io_strategy: IoStrategyArg,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut args = env::args().skip(1);
        let path = args
            .next()
            .map(PathBuf::from)
            .ok_or("usage: cargo run --release --bin real_cube_bench -- <path> [mode] [iterations] [cached] [slice_count] [io]")?;
        let mode = args
            .next()
            .map(|value| Mode::parse(&value))
            .transpose()?
            .unwrap_or(Mode::All);
        let iterations = args
            .next()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(10);
        let cached = args
            .next()
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(false);
        let slice_count = args
            .next()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(5);
        let io_strategy = args
            .next()
            .map(|value| IoStrategyArg::parse(&value))
            .transpose()?
            .unwrap_or(IoStrategyArg::Auto);

        Ok(Self {
            path,
            mode,
            iterations,
            cached,
            slice_count,
            io_strategy,
        })
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Inspect,
    Headers,
    Ingest,
    AssembleCube,
    InlineSeq,
    InlineRand,
    CrosslineSeq,
    CrosslineRand,
    All,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(match value {
            "inspect" => Self::Inspect,
            "headers" => Self::Headers,
            "ingest" => Self::Ingest,
            "assemble-cube" => Self::AssembleCube,
            "inline-seq" => Self::InlineSeq,
            "inline-rand" => Self::InlineRand,
            "crossline-seq" => Self::CrosslineSeq,
            "crossline-rand" => Self::CrosslineRand,
            "all" => Self::All,
            _ => return Err(format!("unsupported mode: {value}").into()),
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Headers => "headers",
            Self::Ingest => "ingest",
            Self::AssembleCube => "assemble-cube",
            Self::InlineSeq => "inline-seq",
            Self::InlineRand => "inline-rand",
            Self::CrosslineSeq => "crossline-seq",
            Self::CrosslineRand => "crossline-rand",
            Self::All => "all",
        }
    }
}

#[derive(Clone, Copy)]
enum IoStrategyArg {
    Auto,
    Stream,
    Mmap,
}

impl IoStrategyArg {
    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(match value {
            "auto" => Self::Auto,
            "stream" => Self::Stream,
            "mmap" => Self::Mmap,
            _ => return Err(format!("unsupported io strategy: {value}").into()),
        })
    }

    fn into_reader(self) -> IoStrategy {
        match self {
            Self::Auto => IoStrategy::Auto,
            Self::Stream => IoStrategy::Stream,
            Self::Mmap => IoStrategy::Mmap,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Stream => "stream",
            Self::Mmap => "mmap",
        }
    }
}

struct RegularVolumeIndex {
    ilines: Vec<i64>,
    xlines: Vec<i64>,
    traces_by_coord: HashMap<(i64, i64), u64>,
    samples_per_trace: usize,
}

impl RegularVolumeIndex {
    fn build(reader: &SegyReader) -> Result<Self, Box<dyn std::error::Error>> {
        let inline_field = reader.header_mapping().inline_3d();
        let crossline_field = reader.header_mapping().crossline_3d();
        let offset_field = reader.header_mapping().offset();
        let headers = reader.load_trace_headers(
            &[inline_field, crossline_field, offset_field],
            TraceSelection::All,
        )?;

        let ilines = headers
            .column(inline_field)
            .ok_or("missing inline header")?;
        let xlines = headers
            .column(crossline_field)
            .ok_or("missing crossline header")?;
        let offsets = headers
            .column(offset_field)
            .ok_or("missing offset header")?;

        let unique_offsets = sorted_unique(offsets);
        if unique_offsets.len() != 1 {
            return Err("slice benchmark currently expects a post-stack cube".into());
        }

        let unique_ilines = sorted_unique(ilines);
        let unique_xlines = sorted_unique(xlines);
        if unique_ilines.len() * unique_xlines.len() != headers.rows() {
            return Err("slice benchmark currently expects regular cube geometry".into());
        }

        let mut traces_by_coord = HashMap::with_capacity(headers.rows());
        for trace_index in 0..headers.rows() {
            let key = (ilines[trace_index], xlines[trace_index]);
            traces_by_coord.insert(key, headers.trace_numbers[trace_index]);
        }

        Ok(Self {
            ilines: unique_ilines,
            xlines: unique_xlines,
            traces_by_coord,
            samples_per_trace: reader.summary().samples_per_trace as usize,
        })
    }
}

fn bench_inspect(path: &PathBuf, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    let elapsed = timed(iterations, || {
        let _ = seis_io::inspect_file(path).unwrap();
    });
    report("inspect", elapsed, iterations);
    Ok(())
}

fn bench_headers(reader: &SegyReader, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    let inline_field = reader.header_mapping().inline_3d();
    let crossline_field = reader.header_mapping().crossline_3d();
    let offset_field = reader.header_mapping().offset();
    let elapsed = timed(iterations, || {
        let _ = reader
            .load_trace_headers(
                &[inline_field, crossline_field, offset_field],
                TraceSelection::All,
            )
            .unwrap();
    });
    report("headers", elapsed, iterations);
    Ok(())
}

fn bench_ingest(
    reader: &SegyReader,
    iterations: usize,
    io_strategy: IoStrategyArg,
) -> Result<(), Box<dyn std::error::Error>> {
    let elapsed = timed(iterations, || {
        let mut seen = 0usize;
        let mut scratch = vec![0.0_f32; 128 * reader.summary().samples_per_trace as usize];
        reader
            .export_trace_chunks_into(
                ChunkReadConfig {
                    traces_per_chunk: 128,
                    selection: TraceSelection::All,
                    parallel_decode: true,
                    io_strategy: io_strategy.into_reader(),
                },
                &mut scratch,
                |chunk| {
                    seen += chunk.trace_count;
                    Ok::<_, std::convert::Infallible>(())
                },
            )
            .unwrap();
        std::hint::black_box(seen);
    });
    report("ingest", elapsed, iterations);
    Ok(())
}

fn bench_assemble_cube(
    reader: &SegyReader,
    iterations: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let elapsed = timed(iterations, || {
        let cube = reader.assemble_cube().unwrap();
        let layout = cube
            .hdf5_layout(CubeChunkShape {
                iline_count: 16,
                xline_count: 16,
                offset_count: 1,
            })
            .unwrap();
        std::hint::black_box(layout.shape);
    });
    report("assemble-cube", elapsed, iterations);
    Ok(())
}

fn bench_slices(
    reader: &SegyReader,
    index: &RegularVolumeIndex,
    args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let scenario = match args.mode {
        Mode::InlineSeq => SliceScenario::inline_sequential(index, args.slice_count),
        Mode::InlineRand => SliceScenario::inline_random(index, args.slice_count),
        Mode::CrosslineSeq => SliceScenario::crossline_sequential(index, args.slice_count),
        Mode::CrosslineRand => SliceScenario::crossline_random(index, args.slice_count),
        _ => return Err("invalid slice mode".into()),
    };

    let mut scratch = vec![0.0_f32; index.samples_per_trace];
    let mode_name = args.mode.as_str();
    let io_strategy = args.io_strategy.into_reader();

    if args.cached {
        for slice_index in &scenario.indices {
            read_slice(
                reader,
                index,
                scenario.orientation,
                *slice_index,
                io_strategy,
                &mut scratch,
            )?;
        }
    }

    let elapsed = timed(args.iterations, || {
        for slice_index in &scenario.indices {
            read_slice(
                reader,
                index,
                scenario.orientation,
                *slice_index,
                io_strategy,
                &mut scratch,
            )
            .unwrap();
        }
    });
    report(mode_name, elapsed, args.iterations);
    Ok(())
}

fn read_slice(
    reader: &SegyReader,
    index: &RegularVolumeIndex,
    orientation: Orientation,
    slice_index: usize,
    io_strategy: IoStrategy,
    scratch: &mut [f32],
) -> Result<(), Box<dyn std::error::Error>> {
    match orientation {
        Orientation::Inline => {
            let inline = index.ilines[slice_index];
            for &xline in &index.xlines {
                let trace_index = index.traces_by_coord[&(inline, xline)];
                reader.read_trace_into(trace_index, io_strategy, scratch)?;
                std::hint::black_box(&scratch[..]);
            }
        }
        Orientation::Crossline => {
            let xline = index.xlines[slice_index];
            for &iline in &index.ilines {
                let trace_index = index.traces_by_coord[&(iline, xline)];
                reader.read_trace_into(trace_index, io_strategy, scratch)?;
                std::hint::black_box(&scratch[..]);
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum Orientation {
    Inline,
    Crossline,
}

struct SliceScenario {
    orientation: Orientation,
    indices: Vec<usize>,
}

impl SliceScenario {
    fn inline_sequential(index: &RegularVolumeIndex, count: usize) -> Self {
        Self {
            orientation: Orientation::Inline,
            indices: (0..count.min(index.ilines.len())).collect(),
        }
    }

    fn inline_random(index: &RegularVolumeIndex, count: usize) -> Self {
        Self {
            orientation: Orientation::Inline,
            indices: pseudo_random_indices(index.ilines.len(), count),
        }
    }

    fn crossline_sequential(index: &RegularVolumeIndex, count: usize) -> Self {
        Self {
            orientation: Orientation::Crossline,
            indices: (0..count.min(index.xlines.len())).collect(),
        }
    }

    fn crossline_random(index: &RegularVolumeIndex, count: usize) -> Self {
        Self {
            orientation: Orientation::Crossline,
            indices: pseudo_random_indices(index.xlines.len(), count),
        }
    }
}

fn pseudo_random_indices(limit: usize, count: usize) -> Vec<usize> {
    if limit == 0 {
        return Vec::new();
    }

    let mut state = 0x1234_5678_9abc_def0_u64;
    (0..count.min(limit))
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state as usize) % limit
        })
        .collect()
}

fn timed(iterations: usize, mut f: impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed().as_secs_f64()
}

fn report(name: &str, total_seconds: f64, iterations: usize) {
    let mean_ms = (total_seconds / iterations as f64) * 1000.0;
    println!("{name}: total={total_seconds:.3}s mean={mean_ms:.3}ms");
}

fn parse_bool(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match value {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(format!("invalid boolean: {value}").into()),
    }
}

fn sorted_unique(values: &[i64]) -> Vec<i64> {
    let mut values = values.to_vec();
    values.sort_unstable();
    values.dedup();
    values
}
