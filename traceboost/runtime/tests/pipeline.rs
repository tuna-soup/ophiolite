use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ndarray::Array3;
use seis_runtime::{
    DatasetId, IngestOptions, InterpMethod, PreflightAction, SectionAxis, SectionRequest,
    SeisGeometryOptions, SeisRefineError, SparseSurveyPolicy, ValidationOptions, describe_store,
    export_store_to_segy, export_store_to_zarr, ingest_segy, ingest_volume, load_array,
    load_occupancy, load_source_volume_with_options, open_store, preflight_segy,
    render_section_csv, render_section_csv_for_request, run_validation, section_view, upscale_2x,
    upscale_store,
};
use tempfile::tempdir;

fn fixture_path(relative: &str) -> PathBuf {
    let root = find_monorepo_root();
    if root.join("test-data").is_dir() {
        root.join("test-data").join(relative)
    } else {
        root.join("test_data").join(relative)
    }
}

fn require_fixture(relative: &str) -> Option<PathBuf> {
    let path = fixture_path(relative);
    if path.exists() {
        Some(path)
    } else {
        eprintln!("skipping test; missing fixture {}", path.display());
        None
    }
}

fn find_monorepo_root() -> PathBuf {
    let start = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    for ancestor in start.ancestors() {
        if ancestor.join("Cargo.lock").is_file()
            && (ancestor.join("test-data").is_dir() || ancestor.join("test_data").is_dir())
        {
            return ancestor.to_path_buf();
        }
    }
    panic!("unable to locate monorepo root from CARGO_MANIFEST_DIR");
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

fn populate_small_offset_headers(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    let first_trace_offset = 3600usize;
    let trace_size = 240 + (50 * 4);

    for trace_index in 0..25 {
        let trace_offset = first_trace_offset + trace_index * trace_size;
        let offset_dst = trace_offset + 36;
        let offset = i32::try_from(trace_index + 1).unwrap().to_be_bytes();
        bytes[offset_dst..offset_dst + 4].copy_from_slice(&offset);
    }

    fs::write(path, bytes).unwrap();
}

fn bytes_per_sample(sample_format_code: u16) -> usize {
    match sample_format_code {
        1 | 2 | 4 | 5 => 4,
        3 => 2,
        8 => 1,
        code => panic!("unsupported sample format code in test fixture helper: {code}"),
    }
}

fn remove_last_trace(src: &Path, dst: &Path) {
    let summary = seis_io::inspect_file(src).unwrap();
    let mut bytes = fs::read(src).unwrap();
    let trace_size =
        240 + summary.samples_per_trace as usize * bytes_per_sample(summary.sample_format_code);
    bytes.truncate(bytes.len() - trace_size);
    fs::write(dst, bytes).unwrap();
}

fn assert_arrays_close(left: &Array3<f32>, right: &Array3<f32>) {
    assert_eq!(left.shape(), right.shape());
    for (index, (lhs, rhs)) in left.iter().zip(right.iter()).enumerate() {
        assert!(
            (lhs - rhs).abs() <= 1.0e-6,
            "array mismatch at linear index {index}: left={lhs} right={rhs}"
        );
    }
}

fn python_has_segyio() -> bool {
    Command::new("python")
        .args([
            "-c",
            "import importlib.util; raise SystemExit(0 if importlib.util.find_spec('segyio') else 1)",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn ingest_writes_a_store_and_manifest() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let store_root = temp.path().join("small.tbvol");

    let handle = ingest_segy(
        fixture,
        &store_root,
        IngestOptions {
            chunk_shape: [2, 3, 50],
            ..IngestOptions::default()
        },
    )
    .unwrap();

    assert!(store_root.join("manifest.json").exists() || handle.manifest_path().exists());
    assert!(handle.manifest_path().exists());
    assert_eq!(handle.manifest.volume.shape, [5, 5, 50]);
    assert_eq!(handle.manifest.tile_shape, [2, 3, 50]);
    assert_eq!(
        handle
            .manifest
            .volume
            .source
            .geometry
            .inline_field
            .start_byte,
        189
    );
    assert_eq!(
        handle
            .manifest
            .volume
            .source
            .geometry
            .crossline_field
            .start_byte,
        193
    );
    assert!(
        handle
            .manifest
            .volume
            .source
            .geometry
            .third_axis_field
            .is_none()
    );

    let array = load_array(&handle).unwrap();
    assert_eq!(array.shape(), &[5, 5, 50]);
    assert!((array[[0, 0, 0]] - 1.199_999_8).abs() < 1.0e-5);
}

#[test]
fn ingest_rejects_irregular_geometry_with_structured_error() {
    let Some(fixture) = require_fixture("small-ps.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let store_root = temp.path().join("small-ps.tbvol");
    let error = ingest_segy(fixture, &store_root, IngestOptions::default()).unwrap_err();

    assert!(matches!(
        error,
        SeisRefineError::UnsupportedSurveyGeometry { .. }
    ));
}

#[test]
fn preflight_reports_sparse_regular_recommendation() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let sparse_path = temp.path().join("small-sparse.sgy");
    remove_last_trace(&fixture, &sparse_path);

    let preflight = preflight_segy(&sparse_path, &IngestOptions::default()).unwrap();

    assert_eq!(
        preflight.geometry.classification,
        "regular_sparse".to_string()
    );
    assert_eq!(
        preflight.recommended_action,
        PreflightAction::RegularizeSparseSurvey
    );
    assert_eq!(preflight.geometry.missing_bin_count, 1);
}

#[test]
fn ingest_accepts_explicit_header_mapping_for_nonstandard_dense_file() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let segy_path = temp.path().join("small-alt.sgy");
    fs::copy(fixture, &segy_path).unwrap();
    relocate_small_geometry_headers(&segy_path);

    let volume = load_source_volume_with_options(
        &segy_path,
        &IngestOptions {
            geometry: SeisGeometryOptions {
                header_mapping: seis_io::HeaderMapping {
                    inline_3d: Some(seis_io::HeaderField::new_i32("INLINE_3D_ALT", 17)),
                    crossline_3d: Some(seis_io::HeaderField::new_i32("CROSSLINE_3D_ALT", 25)),
                    ..seis_io::HeaderMapping::default()
                },
                third_axis_field: None,
            },
            ..IngestOptions::default()
        },
    )
    .unwrap();

    assert_eq!(volume.data.shape(), &[5, 5, 50]);
    assert_eq!(volume.source.geometry.inline_field.start_byte, 17);
    assert_eq!(volume.source.geometry.crossline_field.start_byte, 25);
}

#[test]
fn ingest_ignores_varying_offset_header_for_poststack_dense_file() {
    let source_fixture = fixture_path("small.sgy");
    if !source_fixture.exists() {
        return;
    }

    let temp = tempdir().unwrap();
    let segy_path = temp.path().join("small-offset-poststack.sgy");
    let store_root = temp.path().join("small-offset-poststack.tbvol");
    fs::copy(&source_fixture, &segy_path).unwrap();
    populate_small_offset_headers(&segy_path);

    let preflight = preflight_segy(&segy_path, &IngestOptions::default()).unwrap();
    assert_eq!(
        preflight.recommended_action,
        PreflightAction::DirectDenseIngest
    );

    let expected =
        load_source_volume_with_options(&source_fixture, &IngestOptions::default()).unwrap();
    let handle = ingest_segy(&segy_path, &store_root, IngestOptions::default()).unwrap();
    let actual = load_array(&handle).unwrap();

    assert_eq!(actual.shape(), expected.data.shape());
    assert_arrays_close(&actual, &expected.data);
}

#[test]
fn ingest_can_regularize_sparse_poststack_with_occupancy_mask() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let sparse_path = temp.path().join("small-sparse.sgy");
    let store_root = temp.path().join("small-sparse.tbvol");
    remove_last_trace(&fixture, &sparse_path);

    let handle = ingest_segy(
        &sparse_path,
        &store_root,
        IngestOptions {
            sparse_survey_policy: SparseSurveyPolicy::RegularizeToDense {
                fill_value: -999.25,
            },
            ..IngestOptions::default()
        },
    )
    .unwrap();

    let array = load_array(&handle).unwrap();
    let occupancy = load_occupancy(&handle).unwrap().unwrap();
    assert_eq!(array.shape(), &[5, 5, 50]);
    assert_eq!(occupancy.shape(), &[5, 5]);
    assert_eq!(occupancy[[4, 4]], 0);
    assert!((array[[4, 4, 0]] + 999.25).abs() < 1.0e-6);
    assert_eq!(
        handle
            .manifest
            .volume
            .source
            .regularization
            .as_ref()
            .unwrap()
            .source_classification,
        "regular_sparse"
    );
    assert!(handle.manifest.has_occupancy);
}

#[test]
fn ingest_defaults_to_regularize_sparse_poststack() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let sparse_path = temp.path().join("small-sparse-default.sgy");
    let store_root = temp.path().join("small-sparse-default.tbvol");
    remove_last_trace(&fixture, &sparse_path);

    let handle = ingest_segy(&sparse_path, &store_root, IngestOptions::default()).unwrap();

    let occupancy = load_occupancy(&handle).unwrap().unwrap();
    assert_eq!(occupancy[[4, 4]], 0);
    assert_eq!(
        handle
            .manifest
            .volume
            .source
            .regularization
            .as_ref()
            .expect("regularization provenance")
            .fill_value,
        0.0
    );
}

#[test]
fn upscale_generates_dense_midpoints_and_preserves_samples() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source.tbvol");
    let derived_root = temp.path().join("derived.tbvol");

    ingest_segy(fixture, &source_root, IngestOptions::default()).unwrap();
    let derived = upscale_store(&source_root, &derived_root, Default::default()).unwrap();
    let array = load_array(&derived).unwrap();

    assert_eq!(derived.manifest.volume.shape, [9, 9, 50]);
    let source = load_array(&open_store(&source_root).unwrap()).unwrap();
    let midpoint = array[[0, 1, 0]];
    let expected = (source[[0, 0, 0]] + source[[0, 1, 0]]) * 0.5;
    assert!((midpoint - expected).abs() < 1.0e-5);
    assert_eq!(derived.manifest.volume.axes.sample_axis_ms.len(), 50);
}

#[test]
fn render_exports_inline_section_to_csv() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source.tbvol");
    let csv_path = temp.path().join("inline.csv");

    ingest_segy(fixture, &source_root, IngestOptions::default()).unwrap();
    render_section_csv(&source_root, SectionAxis::Inline, 0, &csv_path).unwrap();

    let csv = fs::read_to_string(csv_path).unwrap();
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert!(header.starts_with("position,"));
    let first_row = lines.next().unwrap();
    assert!(first_row.starts_with("20,"));
}

#[test]
fn describe_store_returns_shared_volume_descriptor() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source.tbvol");

    ingest_segy(fixture, &source_root, IngestOptions::default()).unwrap();

    let descriptor = describe_store(&source_root).unwrap();
    assert_eq!(descriptor.id.0, "source.tbvol");
    assert_eq!(descriptor.label, "source");
    assert_eq!(descriptor.shape, [5, 5, 50]);
    assert_eq!(descriptor.chunk_shape, [5, 5, 50]);
}

#[test]
fn section_view_returns_shared_section_view() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source.tbvol");

    ingest_segy(fixture, &source_root, IngestOptions::default()).unwrap();

    let view = section_view(&source_root, SectionAxis::Inline, 0).unwrap();
    assert_eq!(view.dataset_id.0, "source.tbvol");
    assert_eq!(view.axis, SectionAxis::Inline);
    assert_eq!(view.coordinate.index, 0);
    assert_eq!(view.coordinate.value, 1.0);
    assert_eq!(view.traces, 5);
    assert_eq!(view.samples, 50);
    assert_eq!(view.horizontal_axis_f64le.len(), 5 * 8);
    assert_eq!(view.sample_axis_f32le.len(), 50 * 4);
    assert_eq!(view.amplitudes_f32le.len(), 5 * 50 * 4);
}

#[test]
fn request_driven_render_rejects_dataset_mismatch() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source.tbvol");
    let csv_path = temp.path().join("inline.csv");

    ingest_segy(fixture, &source_root, IngestOptions::default()).unwrap();

    let error = render_section_csv_for_request(
        &source_root,
        &SectionRequest {
            dataset_id: DatasetId("other.tbvol".to_string()),
            axis: SectionAxis::Inline,
            index: 0,
        },
        &csv_path,
    )
    .unwrap_err();

    assert!(matches!(error, SeisRefineError::DatasetIdMismatch { .. }));
}

#[test]
fn cubic_matches_linear_on_linear_ramp_midpoints() {
    let input = Array3::from_shape_vec(
        (3, 3, 1),
        vec![
            0.0, 1.0, 2.0, //
            10.0, 11.0, 12.0, //
            20.0, 21.0, 22.0,
        ],
    )
    .unwrap();

    let linear = upscale_2x(&input, InterpMethod::Linear);
    let cubic = upscale_2x(&input, InterpMethod::Cubic);

    assert_eq!(linear.shape(), &[5, 5, 1]);
    assert_eq!(cubic.shape(), &[5, 5, 1]);
    assert!((linear[[0, 1, 0]] - 0.5).abs() < 1.0e-6);
    assert!((cubic[[0, 1, 0]] - 0.5).abs() < 1.0e-6);
    assert!((cubic[[1, 1, 0]] - 5.5).abs() < 1.0e-6);
}

#[test]
fn validation_writes_dataset_and_summary_reports() {
    let Some(fixture) = require_fixture("small.sgy") else {
        return;
    };
    let temp = tempdir().unwrap();
    let output_dir = temp.path().join("validation");
    let summary = run_validation(ValidationOptions {
        output_dir: output_dir.clone(),
        dataset_paths: vec![fixture],
        validation_mode: seis_io::ValidationMode::Strict,
    })
    .unwrap();

    assert_eq!(summary.dataset_count, 1);
    assert!(output_dir.join("summary.json").exists());
    assert!(output_dir.join("small.json").exists());
}

#[test]
fn segy_roundtrip_preserves_f3_amplitudes_and_key_survey_metadata() {
    let fixture = fixture_path("f3.sgy");
    if !fixture.exists() {
        return;
    }

    let temp = tempdir().unwrap();
    let source_root = temp.path().join("f3-source.tbvol");
    let exported_segy = temp.path().join("f3-roundtrip.sgy");
    let roundtrip_root = temp.path().join("f3-roundtrip.tbvol");

    let source = ingest_segy(&fixture, &source_root, IngestOptions::default()).unwrap();
    export_store_to_segy(&source_root, &exported_segy, false).unwrap();
    let roundtrip = ingest_segy(&exported_segy, &roundtrip_root, IngestOptions::default()).unwrap();

    let source_array = load_array(&source).unwrap();
    let roundtrip_array = load_array(&roundtrip).unwrap();
    assert_arrays_close(&source_array, &roundtrip_array);

    let source_summary = seis_io::inspect_file(&fixture).unwrap();
    let roundtrip_summary = seis_io::inspect_file(&exported_segy).unwrap();
    assert_eq!(source_summary.endianness, roundtrip_summary.endianness);
    assert_eq!(
        source_summary.sample_interval_us,
        roundtrip_summary.sample_interval_us
    );
    assert_eq!(
        source_summary.samples_per_trace,
        roundtrip_summary.samples_per_trace
    );
    assert_eq!(
        source_summary.sample_format_code,
        roundtrip_summary.sample_format_code
    );
    assert_eq!(source_summary.revision_raw, roundtrip_summary.revision_raw);
    assert_eq!(
        source_summary.fixed_length_trace_flag_raw,
        roundtrip_summary.fixed_length_trace_flag_raw
    );
    assert_eq!(
        source_summary.extended_textual_headers,
        roundtrip_summary.extended_textual_headers
    );
    assert_eq!(source_summary.trace_count, roundtrip_summary.trace_count);
    assert_eq!(
        source_summary
            .textual_headers
            .iter()
            .map(|header| header.raw.clone())
            .collect::<Vec<_>>(),
        roundtrip_summary
            .textual_headers
            .iter()
            .map(|header| header.raw.clone())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        source.manifest.volume.shape,
        roundtrip.manifest.volume.shape
    );
    assert_eq!(
        source.manifest.volume.axes.ilines,
        roundtrip.manifest.volume.axes.ilines
    );
    assert_eq!(
        source.manifest.volume.axes.xlines,
        roundtrip.manifest.volume.axes.xlines
    );
    assert_eq!(
        source.manifest.volume.axes.sample_axis_ms,
        roundtrip.manifest.volume.axes.sample_axis_ms
    );
    assert_eq!(
        source.manifest.volume.source.trace_count,
        roundtrip.manifest.volume.source.trace_count
    );
    assert_eq!(
        source.manifest.volume.source.samples_per_trace,
        roundtrip.manifest.volume.source.samples_per_trace
    );
}

#[test]
fn segy_roundtrip_preserves_extended_textual_headers_for_multi_text_fixture() {
    let fixture = fixture_path("multi-text.sgy");
    if !fixture.exists() {
        return;
    }

    let temp = tempdir().unwrap();
    let source_root = temp.path().join("multi-text-source.tbvol");
    let exported_segy = temp.path().join("multi-text-roundtrip.sgy");
    let roundtrip_root = temp.path().join("multi-text-roundtrip.tbvol");

    let source = ingest_segy(&fixture, &source_root, IngestOptions::default()).unwrap();
    export_store_to_segy(&source_root, &exported_segy, false).unwrap();
    let roundtrip = ingest_segy(&exported_segy, &roundtrip_root, IngestOptions::default()).unwrap();

    let source_array = load_array(&source).unwrap();
    let roundtrip_array = load_array(&roundtrip).unwrap();
    assert_arrays_close(&source_array, &roundtrip_array);

    let source_summary = seis_io::inspect_file(&fixture).unwrap();
    let roundtrip_summary = seis_io::inspect_file(&exported_segy).unwrap();
    assert_eq!(source_summary.extended_textual_headers, 4);
    assert_eq!(
        source_summary.extended_textual_headers,
        roundtrip_summary.extended_textual_headers
    );
    assert_eq!(
        source_summary
            .textual_headers
            .iter()
            .map(|header| header.raw.clone())
            .collect::<Vec<_>>(),
        roundtrip_summary
            .textual_headers
            .iter()
            .map(|header| header.raw.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(source_summary.trace_count, roundtrip_summary.trace_count);
    assert_eq!(
        source.manifest.volume.shape,
        roundtrip.manifest.volume.shape
    );
}

#[test]
fn zarr_import_matches_f3_reference_amplitudes_and_axes() {
    let segy_fixture = fixture_path("f3.sgy");
    let zarr_fixture = fixture_path("survey.zarr");
    if !segy_fixture.exists() || !zarr_fixture.exists() {
        return;
    }

    let temp = tempdir().unwrap();
    let segy_root = temp.path().join("f3-reference.tbvol");
    let zarr_root = temp.path().join("survey.tbvol");

    let segy = ingest_segy(&segy_fixture, &segy_root, IngestOptions::default()).unwrap();
    let zarr = ingest_volume(&zarr_fixture, &zarr_root, IngestOptions::default()).unwrap();

    let segy_array = load_array(&segy).unwrap();
    let zarr_array = load_array(&zarr).unwrap();
    assert_arrays_close(&segy_array, &zarr_array);

    assert_eq!(segy.manifest.volume.shape, zarr.manifest.volume.shape);
    assert_eq!(
        segy.manifest.volume.axes.ilines,
        zarr.manifest.volume.axes.ilines
    );
    assert_eq!(
        segy.manifest.volume.axes.xlines,
        zarr.manifest.volume.axes.xlines
    );
    assert_eq!(
        segy.manifest.volume.axes.sample_axis_ms,
        zarr.manifest.volume.axes.sample_axis_ms
    );
    assert_eq!(
        segy.manifest.volume.source.trace_count,
        zarr.manifest.volume.source.trace_count
    );
    assert_eq!(
        segy.manifest.volume.source.samples_per_trace,
        zarr.manifest.volume.source.samples_per_trace
    );
    assert_eq!(
        segy.manifest.volume.source.sample_interval_us,
        zarr.manifest.volume.source.sample_interval_us
    );
    assert_eq!(
        segy.manifest.volume.source.sample_format_code,
        zarr.manifest.volume.source.sample_format_code
    );
}

#[test]
fn zarr_roundtrip_preserves_f3_amplitudes_and_axes() {
    let fixture = fixture_path("f3.sgy");
    if !fixture.exists() {
        return;
    }

    let temp = tempdir().unwrap();
    let source_root = temp.path().join("f3-source.tbvol");
    let exported_zarr = temp.path().join("f3-roundtrip.zarr");
    let roundtrip_root = temp.path().join("f3-zarr-roundtrip.tbvol");

    let source = ingest_segy(&fixture, &source_root, IngestOptions::default()).unwrap();
    export_store_to_zarr(&source_root, &exported_zarr, false).unwrap();
    let roundtrip =
        ingest_volume(&exported_zarr, &roundtrip_root, IngestOptions::default()).unwrap();

    let source_array = load_array(&source).unwrap();
    let roundtrip_array = load_array(&roundtrip).unwrap();
    assert_arrays_close(&source_array, &roundtrip_array);

    assert_eq!(
        source.manifest.volume.shape,
        roundtrip.manifest.volume.shape
    );
    assert_eq!(
        source.manifest.volume.axes.ilines,
        roundtrip.manifest.volume.axes.ilines
    );
    assert_eq!(
        source.manifest.volume.axes.xlines,
        roundtrip.manifest.volume.axes.xlines
    );
    assert_eq!(
        source.manifest.volume.axes.sample_axis_ms,
        roundtrip.manifest.volume.axes.sample_axis_ms
    );
    assert!(exported_zarr.join("metadata").exists());
    assert!(exported_zarr.join("metadata").join("iline").exists());
    assert!(exported_zarr.join("metadata").join("xline").exists());
    assert!(exported_zarr.join("metadata").join("sample_ms").exists());
}

#[test]
fn segy_roundtrip_matches_external_segyio_verifier() {
    let fixture = fixture_path("f3.sgy");
    if !fixture.exists() || !python_has_segyio() {
        return;
    }

    let temp = tempdir().unwrap();
    let source_root = temp.path().join("f3-source.tbvol");
    let exported_segy = temp.path().join("f3-roundtrip.sgy");
    let script = find_monorepo_root()
        .join("scripts")
        .join("verify_segy_roundtrip_with_segyio.py");

    ingest_segy(&fixture, &source_root, IngestOptions::default()).unwrap();
    export_store_to_segy(&source_root, &exported_segy, false).unwrap();

    let output = Command::new("python")
        .arg(script)
        .arg(&fixture)
        .arg(&exported_segy)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "segyio verifier failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
