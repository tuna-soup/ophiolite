use std::path::{Path, PathBuf};

use seis_io::{
    Endianness, SampleFormat, SegyWarning, TextualHeaderEncoding, curated_fixtures, inspect_file,
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
fn curated_fixture_inventory_exists() {
    assert!(fixture_path("small.sgy").exists());
    assert!(fixture_path("multiformats/Format1msb.sgy").exists());
    assert!(fixture_path("\u{5c0f}\u{6587}\u{4ef6}.sgy").exists());
    assert!(fixture_path("small.su").exists());
}

#[test]
fn inspect_curated_fixtures_matches_expected_metadata() {
    for fixture in curated_fixtures() {
        let summary = inspect_file(fixture_path(fixture.path)).unwrap_or_else(|error| {
            panic!("failed to inspect {}: {error}", fixture.path);
        });

        assert_eq!(summary.endianness, fixture.endianness, "{}", fixture.path);
        assert_eq!(
            summary.sample_format, fixture.sample_format,
            "{}",
            fixture.path
        );
        assert_eq!(
            summary.samples_per_trace, fixture.samples_per_trace,
            "{}",
            fixture.path
        );
        assert_eq!(summary.trace_count, fixture.trace_count, "{}", fixture.path);
        assert_eq!(
            summary.extended_textual_headers, fixture.extended_textual_headers,
            "{}",
            fixture.path
        );
    }
}

#[test]
fn inspect_small_and_f3_capture_known_invariants_from_segyio() {
    let small = inspect_file(fixture_path("small.sgy")).unwrap();
    assert_eq!(small.sample_format, SampleFormat::IbmFloat32);
    assert_eq!(small.samples_per_trace, 50);
    assert_eq!(small.trace_count, 25);
    assert_eq!(small.sample_interval_us, 4000);
    assert_eq!(small.first_trace_offset, 3600);

    let f3 = inspect_file(fixture_path("f3.sgy")).unwrap();
    assert_eq!(f3.sample_format, SampleFormat::Int16);
    assert_eq!(f3.samples_per_trace, 75);
    assert_eq!(f3.trace_count, 414);
    assert_eq!(f3.sample_interval_us, 4000);
    assert_eq!(f3.revision.map(|rev| rev.major), Some(1));
    assert_eq!(f3.fixed_length_trace, Some(true));
}

#[test]
fn inspect_multitext_detects_extended_headers() {
    let summary = inspect_file(fixture_path("multi-text.sgy")).unwrap();
    assert_eq!(summary.endianness, Endianness::Big);
    assert_eq!(summary.extended_textual_headers, 4);
    assert_eq!(summary.total_textual_headers(), 5);
    assert_eq!(summary.trace_count, 1);
}

#[test]
fn inspect_long_fixture_handles_large_sample_count() {
    let summary = inspect_file(fixture_path("long.sgy")).unwrap();
    assert_eq!(summary.samples_per_trace, 60_000);
    assert_eq!(summary.trace_count, 3);
    assert_eq!(summary.trace_size_bytes, 240_240);
}

#[test]
fn inspect_autodetects_endianness_for_matching_fixture_pairs() {
    let msb = inspect_file(fixture_path("f3.sgy")).unwrap();
    let lsb = inspect_file(fixture_path("f3-lsb.sgy")).unwrap();

    assert_eq!(msb.endianness, Endianness::Big);
    assert_eq!(lsb.endianness, Endianness::Little);
    assert_eq!(msb.trace_count, lsb.trace_count);
    assert_eq!(msb.samples_per_trace, lsb.samples_per_trace);
    assert_eq!(msb.sample_format, lsb.sample_format);
}

#[test]
fn inspect_decodes_primary_and_extended_textual_headers() {
    let summary = inspect_file(fixture_path("multi-text.sgy")).unwrap();

    assert_eq!(summary.textual_headers.len(), 5);
    assert_eq!(
        summary.textual_headers[0].encoding,
        TextualHeaderEncoding::Ebcdic
    );
    assert!(summary.textual_headers[0].decoded.starts_with("C 1 "));
    assert!(summary.warnings.iter().any(|warning| matches!(
        warning,
        SegyWarning::NonAsciiTextHeader {
            header_index: 0,
            encoding: TextualHeaderEncoding::Ebcdic
        }
    )));
}

#[test]
fn inspect_preserves_embedded_nulls_in_textual_headers() {
    let summary = inspect_file(fixture_path("text-embed-null.sgy")).unwrap();
    assert_eq!(summary.textual_headers.len(), 1);
    assert_eq!(
        summary.textual_headers[0].encoding,
        TextualHeaderEncoding::Ebcdic
    );
    assert!(summary.textual_headers[0].decoded.contains('\0'));
}
