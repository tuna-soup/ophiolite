use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use seis_io::{
    CubeChunkDescriptor, CubeChunkShape, Hdf5CubeLayout, Hdf5CubeWriteError, Hdf5CubeWriter,
    HeaderField, HeaderMapping, ReaderOptions, open,
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

struct MockWriter {
    layout: Option<Hdf5CubeLayout>,
    chunks: Vec<(CubeChunkDescriptor, usize, Vec<f32>)>,
}

impl MockWriter {
    fn new() -> Self {
        Self {
            layout: None,
            chunks: Vec::new(),
        }
    }
}

impl Hdf5CubeWriter for MockWriter {
    type Error = &'static str;

    fn write_layout(&mut self, layout: &Hdf5CubeLayout) -> Result<(), Self::Error> {
        self.layout = Some(layout.clone());
        Ok(())
    }

    fn write_chunk(
        &mut self,
        descriptor: CubeChunkDescriptor,
        samples_per_trace: usize,
        data: &[f32],
    ) -> Result<(), Self::Error> {
        self.chunks
            .push((descriptor, samples_per_trace, data.to_vec()));
        Ok(())
    }
}

#[test]
fn reader_plans_hdf5_cube_layout_without_loading_full_cube() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let layout = reader
        .plan_hdf5_cube_layout(CubeChunkShape {
            iline_count: 2,
            xline_count: 2,
            offset_count: 1,
        })
        .unwrap();

    assert_eq!(layout.shape, (4, 3, 2, 10));
    assert_eq!(layout.chunk_shape, (2, 2, 1, 10));
    assert_eq!(layout.ilines, vec![1, 2, 3, 4]);
    assert_eq!(layout.xlines, vec![1, 2, 3]);
    assert_eq!(layout.offsets, vec![1, 2]);
}

#[test]
fn cube_writes_hdf5_like_chunks_through_mock_sink() {
    let reader = open(fixture_path("small-ps.sgy"), ReaderOptions::default()).unwrap();
    let cube = reader.assemble_cube().unwrap();
    let mut writer = MockWriter::new();

    cube.write_hdf5_like(
        CubeChunkShape {
            iline_count: 2,
            xline_count: 2,
            offset_count: 1,
        },
        &mut writer,
    )
    .unwrap();

    let layout = writer.layout.unwrap();
    assert_eq!(layout.shape, (4, 3, 2, 10));
    assert_eq!(writer.chunks.len(), 8);
    assert_eq!(writer.chunks[0].0.iline_start, 0);
    assert_eq!(writer.chunks[0].1, 10);
    assert_eq!(&writer.chunks[0].2[..10], cube.trace(0, 0, 0));
}

#[test]
fn cube_hdf5_writer_propagates_sink_errors() {
    struct FailingWriter;

    impl Hdf5CubeWriter for FailingWriter {
        type Error = &'static str;

        fn write_layout(&mut self, _layout: &Hdf5CubeLayout) -> Result<(), Self::Error> {
            Ok(())
        }

        fn write_chunk(
            &mut self,
            _descriptor: CubeChunkDescriptor,
            _samples_per_trace: usize,
            _data: &[f32],
        ) -> Result<(), Self::Error> {
            Err("sink failed")
        }
    }

    let reader = open(fixture_path("small.sgy"), ReaderOptions::default()).unwrap();
    let cube = reader.assemble_cube().unwrap();
    let error = cube
        .write_hdf5_like(
            CubeChunkShape {
                iline_count: 2,
                xline_count: 2,
                offset_count: 1,
            },
            &mut FailingWriter,
        )
        .unwrap_err();

    assert!(matches!(error, Hdf5CubeWriteError::Sink("sink failed")));
}

#[test]
fn reader_plans_hdf5_layout_with_geometry_overrides() {
    let src = fixture_path("small.sgy");
    let dst = temp_path("hdf5-override-small.sgy");
    fs::copy(&src, &dst).unwrap();
    relocate_small_geometry_headers(&dst);

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

    let layout = reader
        .plan_hdf5_cube_layout(CubeChunkShape {
            iline_count: 2,
            xline_count: 2,
            offset_count: 1,
        })
        .unwrap();
    assert_eq!(layout.shape, (5, 5, 1, 50));
    assert_eq!(layout.ilines, vec![1, 2, 3, 4, 5]);
    assert_eq!(layout.xlines, vec![20, 21, 22, 23, 24]);

    let _ = fs::remove_file(&dst);
}
