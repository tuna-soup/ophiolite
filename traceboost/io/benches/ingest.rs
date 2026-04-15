use criterion::{Criterion, criterion_group, criterion_main};
use seis_io::{
    ChunkReadConfig, HeaderField, HeaderLoadConfig, IoStrategy, ReaderOptions, TraceSelection,
    inspect_file, open,
};
use std::hint::black_box;
use std::path::Path;

fn fixture(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("io should live directly under the monorepo root")
        .join("test-data")
        .join(relative)
}

fn bench_inspect(c: &mut Criterion) {
    let path = fixture("small.sgy");
    c.bench_function("inspect small", |b| {
        b.iter(|| black_box(inspect_file(&path).unwrap()));
    });
}

fn bench_headers(c: &mut Criterion) {
    let reader = open(fixture("small-ps.sgy"), ReaderOptions::default()).unwrap();
    c.bench_function("load small-ps headers parallel", |b| {
        b.iter(|| {
            black_box(
                reader
                    .load_trace_headers_with_config(
                        &[
                            HeaderField::INLINE_3D,
                            HeaderField::CROSSLINE_3D,
                            HeaderField::OFFSET,
                        ],
                        HeaderLoadConfig {
                            selection: TraceSelection::All,
                            traces_per_chunk: 16,
                            parallel_extract: true,
                            io_strategy: IoStrategy::Auto,
                        },
                    )
                    .unwrap(),
            )
        });
    });
}

fn bench_headers_sequential(c: &mut Criterion) {
    let reader = open(fixture("small-ps.sgy"), ReaderOptions::default()).unwrap();
    c.bench_function("load small-ps headers sequential", |b| {
        b.iter(|| {
            black_box(
                reader
                    .load_trace_headers_with_config(
                        &[
                            HeaderField::INLINE_3D,
                            HeaderField::CROSSLINE_3D,
                            HeaderField::OFFSET,
                        ],
                        HeaderLoadConfig {
                            selection: TraceSelection::All,
                            traces_per_chunk: 16,
                            parallel_extract: false,
                            io_strategy: IoStrategy::Stream,
                        },
                    )
                    .unwrap(),
            )
        });
    });
}

fn bench_headers_mmap(c: &mut Criterion) {
    let reader = open(fixture("small-ps.sgy"), ReaderOptions::default()).unwrap();
    c.bench_function("load small-ps headers mmap", |b| {
        b.iter(|| {
            black_box(
                reader
                    .load_trace_headers_with_config(
                        &[
                            HeaderField::INLINE_3D,
                            HeaderField::CROSSLINE_3D,
                            HeaderField::OFFSET,
                        ],
                        HeaderLoadConfig {
                            selection: TraceSelection::All,
                            traces_per_chunk: 16,
                            parallel_extract: true,
                            io_strategy: IoStrategy::Mmap,
                        },
                    )
                    .unwrap(),
            )
        });
    });
}

fn bench_trace_read(c: &mut Criterion) {
    let reader = open(fixture("f3.sgy"), ReaderOptions::default()).unwrap();
    c.bench_function("read all f3 traces", |b| {
        b.iter(|| {
            black_box(
                reader
                    .read_all_traces(ChunkReadConfig {
                        traces_per_chunk: 64,
                        selection: TraceSelection::All,
                        ..ChunkReadConfig::default()
                    })
                    .unwrap(),
            )
        });
    });
}

fn bench_trace_read_sequential(c: &mut Criterion) {
    let reader = open(fixture("f3.sgy"), ReaderOptions::default()).unwrap();
    c.bench_function("read all f3 traces sequential decode", |b| {
        b.iter(|| {
            black_box(
                reader
                    .read_all_traces(ChunkReadConfig {
                        traces_per_chunk: 64,
                        selection: TraceSelection::All,
                        parallel_decode: false,
                        io_strategy: IoStrategy::Stream,
                    })
                    .unwrap(),
            )
        });
    });
}

fn bench_trace_read_mmap(c: &mut Criterion) {
    let reader = open(fixture("f3.sgy"), ReaderOptions::default()).unwrap();
    c.bench_function("read all f3 traces mmap decode", |b| {
        b.iter(|| {
            black_box(
                reader
                    .read_all_traces(ChunkReadConfig {
                        traces_per_chunk: 64,
                        selection: TraceSelection::All,
                        parallel_decode: true,
                        io_strategy: IoStrategy::Mmap,
                    })
                    .unwrap(),
            )
        });
    });
}

fn bench_trace_process_into(c: &mut Criterion) {
    let reader = open(fixture("f3.sgy"), ReaderOptions::default()).unwrap();
    let layout = reader.trace_block_layout(TraceSelection::All).unwrap();
    let mut scratch = vec![0.0_f32; 64 * layout.samples_per_trace];
    c.bench_function("process f3 trace chunks into scratch", |b| {
        b.iter(|| {
            let mut seen = 0usize;
            black_box(
                reader
                    .process_trace_chunks_into(
                        ChunkReadConfig {
                            traces_per_chunk: 64,
                            selection: TraceSelection::All,
                            parallel_decode: true,
                            io_strategy: IoStrategy::Stream,
                        },
                        &mut scratch,
                        |chunk| {
                            seen += chunk.trace_count;
                            Ok::<_, ()>(())
                        },
                    )
                    .unwrap(),
            );
            black_box(seen);
        });
    });
}

criterion_group!(
    benches,
    bench_inspect,
    bench_headers,
    bench_headers_sequential,
    bench_headers_mmap,
    bench_trace_read,
    bench_trace_read_sequential,
    bench_trace_read_mmap,
    bench_trace_process_into
);
criterion_main!(benches);
