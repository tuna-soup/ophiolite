use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use seis_io::{
    ChunkReadConfig, GeometryClassification, GeometryOptions, HeaderField, HeaderLoadConfig,
    HeaderMapping, IntervalOptions, IoStrategy, ReadError, ReaderOptions, SampleIntervalUnit,
    SegyReader, SegyWarning, TraceSelection, ValidationMode, open,
};

fn fixture_path(relative: &str) -> PathBuf {
    monorepo_root().join("test-data").join(relative)
}

fn monorepo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("io should live directly under the monorepo root")
        .to_path_buf()
}

fn temp_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("seis-io-{nonce}-{name}"))
}

fn assert_close(actual: f32, expected: f32) {
    let delta = (actual - expected).abs();
    assert!(
        delta < 1.0e-4,
        "expected {expected}, got {actual}, |delta|={delta}"
    );
}

fn assert_close_rel(actual: f32, expected: f32, rel_tol: f32) {
    let scale = expected.abs().max(1.0);
    let delta = (actual - expected).abs();
    assert!(
        delta <= scale * rel_tol,
        "expected {expected}, got {actual}, |delta|={delta}, rel_tol={rel_tol}"
    );
}

fn relocate_small_geometry_headers(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    let first_trace_offset = 3600usize;
    let trace_size = 240 + (50 * 4);

    for trace_index in 0..25 {
        let trace_offset = first_trace_offset + trace_index * trace_size;
        let inline_src = trace_offset + 188;
        let crossline_src = trace_offset + 192;
        let inline_dst = trace_offset + 16;
        let crossline_dst = trace_offset + 24;

        let inline = bytes[inline_src..inline_src + 4].to_vec();
        let crossline = bytes[crossline_src..crossline_src + 4].to_vec();
        bytes[inline_dst..inline_dst + 4].copy_from_slice(&inline);
        bytes[crossline_dst..crossline_dst + 4].copy_from_slice(&crossline);
        bytes[inline_src..inline_src + 4].fill(0);
        bytes[crossline_src..crossline_src + 4].fill(0);
    }

    fs::write(path, bytes).unwrap();
}

fn write_sample_intervals(path: &Path, binary_interval: u16, trace_interval: u16) {
    let mut bytes = fs::read(path).unwrap();
    bytes[3216..3218].copy_from_slice(&binary_interval.to_be_bytes());
    let first_trace_offset = 3600usize;
    bytes[first_trace_offset + 116..first_trace_offset + 118]
        .copy_from_slice(&trace_interval.to_be_bytes());
    fs::write(path, bytes).unwrap();
}

fn write_sample_count(path: &Path, trace_sample_count: u16) {
    let mut bytes = fs::read(path).unwrap();
    let first_trace_offset = 3600usize;
    bytes[first_trace_offset + 114..first_trace_offset + 116]
        .copy_from_slice(&trace_sample_count.to_be_bytes());
    fs::write(path, bytes).unwrap();
}

fn remove_last_trace(path: &Path, trace_size: usize) {
    let mut bytes = fs::read(path).unwrap();
    bytes.truncate(bytes.len() - trace_size);
    fs::write(path, bytes).unwrap();
}

#[test]
fn reader_loads_inline_and_crossline_headers_for_small() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let headers = reader
        .load_trace_headers(
            &[HeaderField::INLINE_3D, HeaderField::CROSSLINE_3D],
            TraceSelection::All,
        )
        .unwrap();

    let ilines = headers.column(HeaderField::INLINE_3D).unwrap();
    let xlines = headers.column(HeaderField::CROSSLINE_3D).unwrap();

    assert_eq!(headers.rows(), 25);
    assert_eq!(&ilines[..10], &[1, 1, 1, 1, 1, 2, 2, 2, 2, 2]);
    assert_eq!(&xlines[..10], &[20, 21, 22, 23, 24, 20, 21, 22, 23, 24]);
}

#[test]
fn reader_loads_offset_headers_for_small_prestack_fixture() {
    let reader = SegyReader::open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let headers = reader
        .load_trace_headers(
            &[
                HeaderField::INLINE_3D,
                HeaderField::CROSSLINE_3D,
                HeaderField::OFFSET,
            ],
            TraceSelection::Range { start: 0, end: 10 },
        )
        .unwrap();

    let ilines = headers.column(HeaderField::INLINE_3D).unwrap();
    let xlines = headers.column(HeaderField::CROSSLINE_3D).unwrap();
    let offsets = headers.column(HeaderField::OFFSET).unwrap();

    assert_eq!(ilines, &[1, 1, 1, 1, 1, 1, 2, 2, 2, 2]);
    assert_eq!(xlines, &[1, 1, 2, 2, 3, 3, 1, 1, 2, 2]);
    assert_eq!(offsets, &[1, 2, 1, 2, 1, 2, 1, 2, 1, 2]);
}

#[test]
fn header_load_configs_match_default_results() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();

    let default_headers = reader
        .load_trace_headers(
            &[
                HeaderField::INLINE_3D,
                HeaderField::CROSSLINE_3D,
                HeaderField::OFFSET,
            ],
            TraceSelection::All,
        )
        .unwrap();

    let configured_headers = reader
        .load_trace_headers_with_config(
            &[
                HeaderField::INLINE_3D,
                HeaderField::CROSSLINE_3D,
                HeaderField::OFFSET,
            ],
            HeaderLoadConfig {
                selection: TraceSelection::All,
                traces_per_chunk: 3,
                parallel_extract: false,
                io_strategy: IoStrategy::Auto,
            },
        )
        .unwrap();

    let parallel_headers = reader
        .load_trace_headers_with_config(
            &[
                HeaderField::INLINE_3D,
                HeaderField::CROSSLINE_3D,
                HeaderField::OFFSET,
            ],
            HeaderLoadConfig {
                selection: TraceSelection::All,
                traces_per_chunk: 5,
                parallel_extract: true,
                io_strategy: IoStrategy::Auto,
            },
        )
        .unwrap();

    assert_eq!(default_headers, configured_headers);
    assert_eq!(default_headers, parallel_headers);
}

#[test]
fn header_load_stream_and_mmap_paths_match() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();

    let stream_headers = reader
        .load_trace_headers_with_config(
            &[
                HeaderField::INLINE_3D,
                HeaderField::CROSSLINE_3D,
                HeaderField::OFFSET,
            ],
            HeaderLoadConfig {
                selection: TraceSelection::All,
                traces_per_chunk: 4,
                parallel_extract: false,
                io_strategy: IoStrategy::Stream,
            },
        )
        .unwrap();

    let mmap_headers = reader
        .load_trace_headers_with_config(
            &[
                HeaderField::INLINE_3D,
                HeaderField::CROSSLINE_3D,
                HeaderField::OFFSET,
            ],
            HeaderLoadConfig {
                selection: TraceSelection::All,
                traces_per_chunk: 4,
                parallel_extract: true,
                io_strategy: IoStrategy::Mmap,
            },
        )
        .unwrap();

    assert_eq!(stream_headers, mmap_headers);
}

#[test]
fn header_load_rejects_zero_chunk_size() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let error = reader
        .load_trace_headers_with_config(
            &[HeaderField::INLINE_3D],
            HeaderLoadConfig {
                selection: TraceSelection::All,
                traces_per_chunk: 0,
                parallel_extract: true,
                io_strategy: IoStrategy::Auto,
            },
        )
        .unwrap_err();

    assert!(matches!(error, ReadError::InvalidChunkSize));
}

#[test]
fn reader_reads_chunked_int16_traces() {
    let reader = open(fixture_path("f3.sgy"), ReaderOptions::default()).unwrap();
    let mut iter = reader
        .read_trace_chunks(ChunkReadConfig {
            traces_per_chunk: 11,
            selection: TraceSelection::Range { start: 0, end: 22 },
            ..ChunkReadConfig::default()
        })
        .unwrap();

    let first_chunk = iter.next().unwrap().unwrap();
    let second_chunk = iter.next().unwrap().unwrap();

    assert_eq!(first_chunk.start_trace, 0);
    assert_eq!(first_chunk.trace_count(), 11);
    assert_eq!(second_chunk.start_trace, 11);
    assert_eq!(second_chunk.trace_count(), 11);

    let trace_ten = first_chunk.trace(10);
    assert_eq!(trace_ten[20], 0.0);
    assert_eq!(trace_ten[25], -1170.0);
    assert_eq!(trace_ten[30], 5198.0);
    assert_eq!(trace_ten[35], -2213.0);
    assert_eq!(trace_ten[40], -888.0);
}

#[test]
fn reader_reads_all_traces_for_ieee_fixture() {
    let reader = open(
        fixture_path("multiformats/Format5msb.sgy"),
        ReaderOptions::default(),
    )
    .unwrap();

    let block = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 128,
            selection: TraceSelection::Range { start: 10, end: 12 },
            ..ChunkReadConfig::default()
        })
        .unwrap();

    assert_eq!(block.trace_count, 2);
    assert_eq!(block.samples_per_trace, 75);
    let trace_zero = block.trace(0);
    assert_eq!(trace_zero[20], 0.0);
    assert_eq!(trace_zero[25], -1170.0);
    assert_eq!(trace_zero[30], 5198.0);
    assert_eq!(trace_zero[35], -2213.0);
    assert_eq!(trace_zero[40], -888.0);
}

#[test]
fn reader_reads_ibm_float_fixture() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let block = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 8,
            selection: TraceSelection::Range { start: 0, end: 1 },
            ..ChunkReadConfig::default()
        })
        .unwrap();

    let trace = block.trace(0);
    assert_close(trace[0], 1.1999998);
    assert_close(trace[1], 1.2000093);
    assert_close(trace[2], 1.2000198);
}

#[test]
fn reader_reads_all_traces_into_preallocated_buffer() {
    let reader = open(fixture_path("f3.sgy"), ReaderOptions::default()).unwrap();
    let config = ChunkReadConfig {
        traces_per_chunk: 16,
        selection: TraceSelection::Range { start: 4, end: 12 },
        parallel_decode: true,
        io_strategy: IoStrategy::Mmap,
    };

    let expected = reader.read_all_traces(config).unwrap();
    let mut buffer = vec![0.0_f32; expected.data.len()];
    let layout = reader.read_all_traces_into(config, &mut buffer).unwrap();

    assert_eq!(layout.start_trace, expected.start_trace);
    assert_eq!(layout.trace_count, expected.trace_count);
    assert_eq!(layout.samples_per_trace, expected.samples_per_trace);
    assert_eq!(buffer, expected.data);
}

#[test]
fn reader_reads_single_trace_into_destination_buffer() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let expected = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 1,
            selection: TraceSelection::Range { start: 3, end: 4 },
            parallel_decode: false,
            io_strategy: IoStrategy::Mmap,
        })
        .unwrap();

    let mut buffer = vec![0.0_f32; expected.samples_per_trace];
    reader
        .read_trace_into(3, IoStrategy::Stream, &mut buffer)
        .unwrap();

    assert_eq!(buffer, expected.data);
}

#[test]
fn reader_processes_trace_chunks_with_reused_scratch() {
    let reader = open(fixture_path("f3.sgy"), ReaderOptions::default()).unwrap();
    let expected = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 8,
            selection: TraceSelection::Range { start: 0, end: 24 },
            parallel_decode: false,
            io_strategy: IoStrategy::Stream,
        })
        .unwrap();

    let mut scratch = vec![0.0_f32; 6 * expected.samples_per_trace];
    let mut flattened = Vec::new();
    let mut starts = Vec::new();
    reader
        .process_trace_chunks_into(
            ChunkReadConfig {
                traces_per_chunk: 8,
                selection: TraceSelection::Range { start: 0, end: 24 },
                parallel_decode: true,
                io_strategy: IoStrategy::Stream,
            },
            &mut scratch,
            |chunk| {
                starts.push((chunk.start_trace, chunk.trace_count));
                flattened.extend_from_slice(chunk.data);
                Ok::<_, &'static str>(())
            },
        )
        .unwrap();

    assert_eq!(starts, vec![(0, 6), (6, 6), (12, 6), (18, 6)]);
    assert_eq!(flattened, expected.data);
}

#[test]
fn reader_rejects_wrong_sized_destination_buffer() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let error = reader
        .read_all_traces_into(
            ChunkReadConfig {
                traces_per_chunk: 8,
                selection: TraceSelection::Range { start: 0, end: 2 },
                ..ChunkReadConfig::default()
            },
            &mut vec![0.0_f32; 10],
        )
        .unwrap_err();

    assert!(matches!(
        error,
        ReadError::InvalidDestinationBuffer {
            actual_len: 10,
            expected_len: 100
        }
    ));

    let process_error = reader
        .process_trace_chunks_into(
            ChunkReadConfig {
                traces_per_chunk: 8,
                selection: TraceSelection::Range { start: 0, end: 2 },
                ..ChunkReadConfig::default()
            },
            &mut vec![0.0_f32; 10],
            |_chunk| Ok::<_, &'static str>(()),
        )
        .unwrap_err();

    assert!(matches!(
        process_error,
        seis_io::ChunkProcessingError::Read(ReadError::InvalidDestinationBuffer {
            actual_len: 10,
            expected_len: 50
        })
    ));

    let trace_error = reader
        .read_trace_into(999, IoStrategy::Auto, &mut vec![0.0_f32; 50])
        .unwrap_err();
    assert!(matches!(trace_error, ReadError::InvalidSelection { .. }));
}

#[test]
fn reader_reads_int8_fixture() {
    let reader = open(
        fixture_path("multiformats/Format8msb.sgy"),
        ReaderOptions::default(),
    )
    .unwrap();
    let block = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 32,
            selection: TraceSelection::Range { start: 10, end: 11 },
            ..ChunkReadConfig::default()
        })
        .unwrap();

    let trace = block.trace(0);
    assert_eq!(trace[20], 0.0);
    assert_eq!(trace[25], 110.0);
    assert_eq!(trace[30], 78.0);
    assert_eq!(trace[35], 91.0);
    assert_eq!(trace[40], -120.0);
}

#[test]
fn reader_reads_additional_multiformat_fixtures() {
    let exact_cases: &[(&str, [f32; 5])] = &[
        (
            "multiformats/Format2msb.sgy",
            [0.0, -1170.0, 5198.0, -2213.0, -888.0],
        ),
        (
            "multiformats/Format2lsb.sgy",
            [0.0, -1170.0, 5198.0, -2213.0, -888.0],
        ),
        (
            "multiformats/Format6msb.sgy",
            [0.0, -1170.0, 5198.0, -2213.0, -888.0],
        ),
        (
            "multiformats/Format6lsb.sgy",
            [0.0, -1170.0, 5198.0, -2213.0, -888.0],
        ),
        (
            "multiformats/Format7msb.sgy",
            [0.0, 64366.0, 5198.0, 63323.0, 64648.0],
        ),
        (
            "multiformats/Format7lsb.sgy",
            [0.0, 64366.0, 5198.0, 63323.0, 64648.0],
        ),
        (
            "multiformats/Format9msb.sgy",
            [0.0, -1170.0, 5198.0, -2213.0, -888.0],
        ),
        (
            "multiformats/Format9lsb.sgy",
            [0.0, -1170.0, 5198.0, -2213.0, -888.0],
        ),
        (
            "multiformats/Format11msb.sgy",
            [0.0, 64366.0, 5198.0, 63323.0, 64648.0],
        ),
        (
            "multiformats/Format11lsb.sgy",
            [0.0, 64366.0, 5198.0, 63323.0, 64648.0],
        ),
        (
            "multiformats/Format15msb.sgy",
            [0.0, 64366.0, 5198.0, 63323.0, 64648.0],
        ),
        (
            "multiformats/Format15lsb.sgy",
            [0.0, 64366.0, 5198.0, 63323.0, 64648.0],
        ),
        (
            "multiformats/Format16msb.sgy",
            [0.0, 110.0, 78.0, 91.0, 136.0],
        ),
        (
            "multiformats/Format16lsb.sgy",
            [0.0, 110.0, 78.0, 91.0, 136.0],
        ),
    ];

    for (path, expected) in exact_cases {
        let reader = open(fixture_path(path), ReaderOptions::default()).unwrap();
        let block = reader
            .read_all_traces(ChunkReadConfig {
                traces_per_chunk: 64,
                selection: TraceSelection::Range { start: 10, end: 11 },
                ..ChunkReadConfig::default()
            })
            .unwrap();
        let trace = block.trace(0);
        assert_eq!(
            [trace[20], trace[25], trace[30], trace[35], trace[40]],
            *expected,
            "{path}"
        );
    }

    let approx_cases: &[(&str, [f32; 5])] = &[
        (
            "multiformats/Format10msb.sgy",
            [
                0.0,
                4_294_966_016.0,
                5198.0,
                4_294_964_992.0,
                4_294_966_528.0,
            ],
        ),
        (
            "multiformats/Format10lsb.sgy",
            [
                0.0,
                4_294_966_016.0,
                5198.0,
                4_294_964_992.0,
                4_294_966_528.0,
            ],
        ),
        (
            "multiformats/Format12msb.sgy",
            [0.0, 1.844_674_4e19, 5198.0, 1.844_674_4e19, 1.844_674_4e19],
        ),
        (
            "multiformats/Format12lsb.sgy",
            [0.0, 1.844_674_4e19, 5198.0, 1.844_674_4e19, 1.844_674_4e19],
        ),
    ];

    for (path, expected) in approx_cases {
        let reader = open(fixture_path(path), ReaderOptions::default()).unwrap();
        let block = reader
            .read_all_traces(ChunkReadConfig {
                traces_per_chunk: 64,
                selection: TraceSelection::Range { start: 10, end: 11 },
                ..ChunkReadConfig::default()
            })
            .unwrap();
        let trace = block.trace(0);
        let actual = [trace[20], trace[25], trace[30], trace[35], trace[40]];
        for (got, want) in actual.into_iter().zip(expected.iter().copied()) {
            assert_close_rel(got, want, 1.0e-5);
        }
    }
}

#[test]
fn format4_is_explicitly_unsupported_for_now() {
    let reader = open(
        fixture_path("multiformats/Format4msb.sgy"),
        ReaderOptions::default(),
    )
    .unwrap();

    let error = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 64,
            selection: TraceSelection::Range { start: 10, end: 11 },
            ..ChunkReadConfig::default()
        })
        .unwrap_err();

    assert!(matches!(error, ReadError::UnsupportedSampleFormat { .. }));
}

#[test]
fn parallel_and_sequential_decode_paths_match() {
    let reader = open(fixture_path("f3.sgy"), ReaderOptions::default()).unwrap();

    let parallel = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 32,
            selection: TraceSelection::Range { start: 0, end: 64 },
            parallel_decode: true,
            io_strategy: IoStrategy::Auto,
        })
        .unwrap();

    let sequential = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 32,
            selection: TraceSelection::Range { start: 0, end: 64 },
            parallel_decode: false,
            io_strategy: IoStrategy::Auto,
        })
        .unwrap();

    assert_eq!(parallel.samples_per_trace, sequential.samples_per_trace);
    assert_eq!(parallel.trace_count, sequential.trace_count);
    assert_eq!(parallel.data, sequential.data);
}

#[test]
fn stream_and_mmap_trace_reads_match() {
    let reader = open(fixture_path("f3.sgy"), ReaderOptions::default()).unwrap();

    let stream = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 32,
            selection: TraceSelection::Range { start: 0, end: 64 },
            parallel_decode: false,
            io_strategy: IoStrategy::Stream,
        })
        .unwrap();

    let mmap = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 32,
            selection: TraceSelection::Range { start: 0, end: 64 },
            parallel_decode: true,
            io_strategy: IoStrategy::Mmap,
        })
        .unwrap();

    assert_eq!(stream.trace_count, mmap.trace_count);
    assert_eq!(stream.samples_per_trace, mmap.samples_per_trace);
    assert_eq!(stream.data, mmap.data);
}

#[test]
fn geometry_analysis_reports_dense_poststack_cube() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let report = reader.analyze_geometry(GeometryOptions::default()).unwrap();

    assert_eq!(report.classification, GeometryClassification::RegularDense);
    assert_eq!(report.observed_trace_count, 25);
    assert_eq!(report.expected_trace_count, 25);
    assert_eq!(report.missing_bin_count, 0);
    assert_eq!(report.duplicate_coordinate_count, 0);
    assert_eq!(report.inline_values, vec![1, 2, 3, 4, 5]);
    assert_eq!(report.crossline_values, vec![20, 21, 22, 23, 24]);
    assert!(report.third_axis_values.is_empty());
}

#[test]
fn geometry_analysis_reports_sparse_regular_grid() {
    let src = fixture_path("small.sgy");
    let dst = temp_path("sparse-small.sgy");
    fs::copy(&src, &dst).unwrap();
    remove_last_trace(&dst, 240 + (50 * 4));

    let reader = open(&dst, ReaderOptions::default()).unwrap();
    let report = reader.analyze_geometry(GeometryOptions::default()).unwrap();

    assert_eq!(report.classification, GeometryClassification::RegularSparse);
    assert_eq!(report.observed_trace_count, 24);
    assert_eq!(report.expected_trace_count, 25);
    assert_eq!(report.missing_bin_count, 1);
    assert_eq!(report.duplicate_coordinate_count, 0);

    let _ = fs::remove_file(&dst);
}

#[test]
fn geometry_analysis_reports_duplicates_when_third_axis_is_ignored() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let report = reader.analyze_geometry(GeometryOptions::default()).unwrap();

    assert_eq!(
        report.classification,
        GeometryClassification::DuplicateCoordinates
    );
    assert_eq!(report.expected_trace_count, 12);
    assert_eq!(report.observed_trace_count, 24);
    assert_eq!(report.duplicate_coordinate_count, 12);
    assert!(!report.duplicate_examples.is_empty());
}

#[test]
fn geometry_analysis_supports_explicit_third_axis() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let report = reader
        .analyze_geometry(GeometryOptions {
            third_axis_field: Some(HeaderField::OFFSET),
            ..GeometryOptions::default()
        })
        .unwrap();

    assert_eq!(report.classification, GeometryClassification::RegularDense);
    assert_eq!(report.expected_trace_count, 24);
    assert_eq!(report.observed_trace_count, 24);
    assert_eq!(report.duplicate_coordinate_count, 0);
    assert_eq!(report.third_axis_values, vec![1, 2]);
}

#[test]
fn reader_assembles_poststack_cube() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let cube = reader.assemble_cube().unwrap();

    assert_eq!(cube.dimensions(), (5, 5, 1, 50));
    assert_eq!(cube.ilines, vec![1, 2, 3, 4, 5]);
    assert_eq!(cube.xlines, vec![20, 21, 22, 23, 24]);
    assert_eq!(cube.offsets, vec![1]);
    assert_close(cube.trace(0, 0, 0)[0], 1.1999998);
}

#[test]
fn reader_assembles_prestack_cube() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let cube = reader.assemble_cube().unwrap();

    assert_eq!(cube.dimensions(), (4, 3, 2, 10));
    assert_eq!(cube.ilines, vec![1, 2, 3, 4]);
    assert_eq!(cube.xlines, vec![1, 2, 3]);
    assert_eq!(cube.offsets, vec![1, 2]);
    assert_close(cube.trace(0, 0, 0)[0], 101.0099945);
    assert_close(cube.trace(0, 0, 1)[0], 201.0099945);
    assert_close(cube.trace(0, 1, 0)[0], 101.0199890);
}

#[test]
fn reader_can_use_geometry_header_overrides_for_cube_assembly() {
    let src = fixture_path("small.sgy");
    let dst = temp_path("override-geometry-small.sgy");
    fs::copy(&src, &dst).unwrap();
    relocate_small_geometry_headers(&dst);

    let default_error = open(&dst, ReaderOptions::default())
        .unwrap()
        .assemble_cube()
        .unwrap_err();
    assert!(matches!(
        default_error,
        ReadError::IrregularGeometry { .. } | ReadError::DuplicateTraceCoordinate { .. }
    ));

    let reader = open(
        &dst,
        ReaderOptions {
            header_mapping: HeaderMapping {
                inline_3d: Some(HeaderField::new_i32("INLINE_3D_ALT", 17)),
                crossline_3d: Some(HeaderField::new_i32("CROSSLINE_3D_ALT", 25)),
                ..HeaderMapping::default()
            },
            ..ReaderOptions::default()
        },
    )
    .unwrap();
    let cube = reader.assemble_cube().unwrap();

    assert_eq!(cube.dimensions(), (5, 5, 1, 50));
    assert_eq!(cube.ilines, vec![1, 2, 3, 4, 5]);
    assert_eq!(cube.xlines, vec![20, 21, 22, 23, 24]);

    let _ = fs::remove_file(&dst);
}

#[test]
fn reader_rejects_irregular_cube_geometry() {
    let reader = open(fixture_path("shot-gather.sgy"), ReaderOptions::default()).unwrap();
    let error = reader.assemble_cube().unwrap_err();

    assert!(matches!(
        error,
        ReadError::IrregularGeometry { .. } | ReadError::DuplicateTraceCoordinate { .. }
    ));
}

#[test]
fn reader_resolves_sample_interval_and_delay_scalar() {
    let reader = open(fixture_path("delay-scalar.sgy"), ReaderOptions::default()).unwrap();
    assert_eq!(reader.resolved_sample_interval_us(), 4000);

    let axis = reader.sample_axis_ms();
    assert_eq!(axis[0], 1000.0);
    assert_eq!(axis[1], 1004.0);
}

#[test]
fn reader_uses_positive_trace_interval_when_binary_interval_is_negative() {
    let reader = open(
        fixture_path("interval-neg-bin-pos-trace.sgy"),
        ReaderOptions::default(),
    )
    .unwrap();
    assert_eq!(reader.resolved_sample_interval_us(), 4000);
}

#[test]
fn reader_uses_positive_binary_interval_when_trace_interval_is_negative() {
    let reader = open(
        fixture_path("interval-pos-bin-neg-trace.sgy"),
        ReaderOptions::default(),
    )
    .unwrap();
    assert_eq!(reader.resolved_sample_interval_us(), 2000);
}

#[test]
fn strict_mode_rejects_header_interval_mismatch_while_lenient_mode_allows_it() {
    let src = fixture_path("small.sgy");
    let dst = temp_path("mismatch-small.sgy");
    fs::copy(&src, &dst).unwrap();

    write_sample_intervals(&dst, 4000, 2000);

    let strict = open(&dst, ReaderOptions::default());
    assert!(matches!(
        strict,
        Err(ReadError::InconsistentSampleInterval {
            binary_header: 4000,
            trace_header: 2000
        })
    ));

    let lenient = open(
        &dst,
        ReaderOptions {
            validation_mode: ValidationMode::Lenient,
            ..ReaderOptions::default()
        },
    )
    .unwrap();
    assert_eq!(lenient.summary().trace_count, 25);
    assert!(lenient.warnings().iter().any(|warning| matches!(
        warning,
        SegyWarning::ConflictingSampleInterval {
            binary_header: 4000,
            trace_header: 2000,
            resolved_us: 4000,
        }
    )));

    let _ = fs::remove_file(&dst);
}

#[test]
fn reader_reports_sample_count_mismatch_as_warning() {
    let src = fixture_path("small.sgy");
    let dst = temp_path("mismatch-sample-count-small.sgy");
    fs::copy(&src, &dst).unwrap();
    write_sample_count(&dst, 49);

    let strict = open(&dst, ReaderOptions::default()).unwrap();
    assert!(strict.warnings().iter().any(|warning| matches!(
        warning,
        SegyWarning::ConflictingSampleCount {
            binary_header: 50,
            trace_header: 49
        }
    )));

    let lenient = open(
        &dst,
        ReaderOptions {
            validation_mode: ValidationMode::Lenient,
            ..ReaderOptions::default()
        },
    )
    .unwrap();
    assert!(lenient.warnings().iter().any(|warning| matches!(
        warning,
        SegyWarning::ConflictingSampleCount {
            binary_header: 50,
            trace_header: 49
        }
    )));

    let _ = fs::remove_file(&dst);
}

#[test]
fn reader_supports_interval_unit_override_and_lenient_ms_guess() {
    let src = fixture_path("small.sgy");
    let dst = temp_path("interval-ms-small.sgy");
    fs::copy(&src, &dst).unwrap();
    write_sample_intervals(&dst, 4, 4);

    let default_reader = open(&dst, ReaderOptions::default()).unwrap();
    assert_eq!(default_reader.resolved_sample_interval_us(), 4);
    assert!(
        default_reader
            .summary()
            .warnings
            .iter()
            .any(|warning| matches!(
                warning,
                SegyWarning::SuspiciousSampleInterval { raw_value: 4, .. }
            ))
    );

    let guessed = open(
        &dst,
        ReaderOptions {
            validation_mode: ValidationMode::Lenient,
            interval_options: IntervalOptions {
                unit_override: None,
                enable_lenient_ms_guess: true,
            },
            ..ReaderOptions::default()
        },
    )
    .unwrap();
    assert_eq!(guessed.resolved_sample_interval_us(), 4000);

    let overridden = open(
        &dst,
        ReaderOptions {
            interval_options: IntervalOptions {
                unit_override: Some(SampleIntervalUnit::Milliseconds),
                enable_lenient_ms_guess: false,
            },
            ..ReaderOptions::default()
        },
    )
    .unwrap();
    assert_eq!(overridden.resolved_sample_interval_us(), 4000);

    let _ = fs::remove_file(&dst);
}
