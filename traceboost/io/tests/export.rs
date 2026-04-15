use std::path::{Path, PathBuf};

use seis_io::{
    ChunkReadConfig, CubeChunkDescriptor, CubeChunkShape, IoStrategy, ReaderOptions,
    TraceSelection, open,
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

#[test]
fn trace_export_metadata_and_chunks_match_bulk_read() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let selection = TraceSelection::Range { start: 5, end: 9 };

    let metadata = reader.trace_export_metadata(selection).unwrap();
    assert_eq!(metadata.start_trace, 5);
    assert_eq!(metadata.trace_count, 4);
    assert_eq!(metadata.samples_per_trace, 50);
    assert_eq!(metadata.sample_interval_us, 4000);
    assert_eq!(metadata.sample_axis_ms.len(), 50);
    assert_eq!(metadata.sample_axis_ms[0], 0.0);
    assert_eq!(metadata.sample_axis_ms[1], 4.0);

    let expected = reader
        .read_all_traces(ChunkReadConfig {
            traces_per_chunk: 4,
            selection,
            parallel_decode: false,
            io_strategy: IoStrategy::Stream,
        })
        .unwrap();

    let chunks = reader
        .export_trace_chunks(ChunkReadConfig {
            traces_per_chunk: 3,
            selection,
            parallel_decode: true,
            io_strategy: IoStrategy::Auto,
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].start_trace, 5);
    assert_eq!(chunks[0].trace_count, 3);
    assert_eq!(chunks[1].start_trace, 8);
    assert_eq!(chunks[1].trace_count, 1);
    assert_eq!(chunks[0].samples_per_trace, 50);
    assert_eq!(chunks[0].trace(0), expected.trace(0));
    assert_eq!(chunks[0].trace(2), expected.trace(2));
    assert_eq!(chunks[1].trace(0), expected.trace(3));

    let flattened = chunks
        .into_iter()
        .flat_map(|chunk| chunk.data)
        .collect::<Vec<_>>();
    assert_eq!(flattened, expected.data);

    let mut scratch = vec![0.0_f32; 2 * expected.samples_per_trace];
    let mut streamed = Vec::new();
    let mut streamed_starts = Vec::new();
    reader
        .export_trace_chunks_into(
            ChunkReadConfig {
                traces_per_chunk: 3,
                selection,
                parallel_decode: true,
                io_strategy: IoStrategy::Stream,
            },
            &mut scratch,
            |chunk| {
                streamed_starts.push((chunk.start_trace, chunk.trace_count));
                streamed.extend_from_slice(chunk.data);
                Ok::<_, &'static str>(())
            },
        )
        .unwrap();

    assert_eq!(streamed_starts, vec![(5, 2), (7, 2)]);
    assert_eq!(streamed, expected.data);
}

#[test]
fn cube_export_metadata_and_chunk_descriptors_cover_cube() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let cube = reader.assemble_cube().unwrap();

    let metadata = cube.export_metadata();
    assert_eq!(metadata.ilines, vec![1, 2, 3, 4]);
    assert_eq!(metadata.xlines, vec![1, 2, 3]);
    assert_eq!(metadata.offsets, vec![1, 2]);
    assert_eq!(metadata.samples_per_trace, 10);
    assert_eq!(metadata.sample_interval_us, 4000);
    assert_eq!(metadata.sample_axis_ms[0], 0.0);
    assert_eq!(metadata.sample_axis_ms[1], 4.0);

    let descriptors = cube
        .chunk_descriptors(CubeChunkShape {
            iline_count: 3,
            xline_count: 2,
            offset_count: 1,
        })
        .unwrap();

    assert_eq!(descriptors.len(), 8);
    assert_eq!(
        descriptors.first().copied(),
        Some(CubeChunkDescriptor {
            iline_start: 0,
            iline_count: 3,
            xline_start: 0,
            xline_count: 2,
            offset_start: 0,
            offset_count: 1,
        })
    );
    assert_eq!(
        descriptors.last().copied(),
        Some(CubeChunkDescriptor {
            iline_start: 3,
            iline_count: 1,
            xline_start: 2,
            xline_count: 1,
            offset_start: 1,
            offset_count: 1,
        })
    );

    let chunk = cube.export_chunk(descriptors[0]).unwrap();
    assert_eq!(chunk.dimensions(), (3, 2, 1, 10));
    assert_eq!(chunk.data.len(), 60);
    assert_eq!(&chunk.data[..10], cube.trace(0, 0, 0));
    assert_eq!(&chunk.data[10..20], cube.trace(0, 1, 0));
    assert_eq!(&chunk.data[20..30], cube.trace(1, 0, 0));

    let mut buffer = vec![0.0_f32; 60];
    cube.export_chunk_into(descriptors[0], &mut buffer).unwrap();
    assert_eq!(buffer, chunk.data);
}

#[test]
fn cube_export_rejects_invalid_shapes_and_descriptors() {
    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let cube = reader.assemble_cube().unwrap();

    let shape_error = cube
        .chunk_descriptors(CubeChunkShape {
            iline_count: 0,
            xline_count: 1,
            offset_count: 1,
        })
        .unwrap_err();
    assert_eq!(
        shape_error.to_string(),
        "chunk size must be greater than zero"
    );

    let descriptor_error = cube
        .export_chunk(CubeChunkDescriptor {
            iline_start: 5,
            iline_count: 1,
            xline_start: 0,
            xline_count: 1,
            offset_start: 0,
            offset_count: 1,
        })
        .unwrap_err();
    assert_eq!(
        descriptor_error.to_string(),
        "chunk size must be greater than zero"
    );

    let buffer_error = cube
        .export_chunk_into(
            CubeChunkDescriptor {
                iline_start: 0,
                iline_count: 1,
                xline_start: 0,
                xline_count: 1,
                offset_start: 0,
                offset_count: 1,
            },
            &mut vec![0.0_f32; 10],
        )
        .unwrap_err();
    assert_eq!(
        buffer_error.to_string(),
        "destination buffer has length 10, expected 50"
    );
}
