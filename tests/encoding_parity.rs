use lithos_las::{ReadOptions, examples, read_path};

#[test]
fn detects_utf8_and_utf8_bom_examples() {
    let utf8 = read_path(
        examples::path("encodings_utf8.las"),
        &ReadOptions::default(),
    )
    .unwrap();
    assert_eq!(utf8.encoding.as_deref(), Some("utf-8"));

    let utf8_bom = read_path(
        examples::path("encodings_utf8wbom.las"),
        &ReadOptions::default(),
    )
    .unwrap();
    assert_eq!(utf8_bom.encoding.as_deref(), Some("utf-8-sig"));
}

#[test]
fn reads_utf16_examples_with_autodetect_and_explicit_encoding() {
    let autodetect = read_path(
        examples::path("encodings_utf16le.las"),
        &ReadOptions::default(),
    )
    .unwrap();
    assert_eq!(autodetect.encoding.as_deref(), Some("utf-16le"));

    let explicit = read_path(
        examples::path("encodings_utf16le.las"),
        &ReadOptions {
            encoding: Some(String::from("utf-16le")),
            autodetect_encoding: false,
            ..ReadOptions::default()
        },
    )
    .unwrap();
    assert_eq!(explicit.encoding.as_deref(), Some("utf-16le"));
}

#[test]
fn decodes_latin_encodings_from_fixture_corpus() {
    let iso = read_path(
        examples::path("encodings_iso88591.las"),
        &ReadOptions::default(),
    )
    .unwrap();
    let cp1252 = read_path(
        examples::path("encodings_cp1252.las"),
        &ReadOptions::default(),
    )
    .unwrap();

    assert!(iso.encoding.is_some());
    assert!(cp1252.encoding.is_some());
    assert_eq!(iso.summary.row_count, 3);
    assert_eq!(cp1252.summary.row_count, 3);
}
