use ophiolite::{ReadOptions, examples, read_path, read_reader, read_string};
use std::fs;
use std::fs::File;
use std::path::PathBuf;

#[test]
fn opens_pathbuf_file_reader_and_inline_string() {
    let path = examples::path("sample.las");
    let from_path = read_path(PathBuf::from(&path), &ReadOptions::default()).unwrap();

    let reader = File::open(&path).unwrap();
    let from_reader = read_reader(reader, &ReadOptions::default()).unwrap();

    let inline = fs::read_to_string(&path).unwrap();
    let from_string = read_string(&inline, &ReadOptions::default()).unwrap();

    assert_eq!(from_path.keys(), from_reader.keys());
    assert_eq!(from_path.keys(), from_string.keys());
    assert_eq!(from_string.summary.original_filename, "inline.las");
}

#[test]
fn handles_missing_wrap_and_missing_vers_files() {
    let missing_wrap = examples::open("missing_wrap.las", &ReadOptions::default()).unwrap();
    assert_eq!(missing_wrap.summary.wrap_mode, "NO");

    let missing_vers = examples::open("missing_vers.las", &ReadOptions::default()).unwrap();
    assert_eq!(missing_vers.summary.las_version, "unknown");
    assert_eq!(missing_vers.summary.wrap_mode, "NO");
}

#[test]
fn supports_header_only_and_empty_ascii_sections() {
    let header_only = examples::open("header_only.las", &ReadOptions::default()).unwrap();
    assert_eq!(header_only.summary.row_count, 0);

    let missing_a = examples::open("missing_a_section.las", &ReadOptions::default()).unwrap();
    assert_eq!(missing_a.summary.row_count, 0);
    assert_eq!(missing_a.keys(), vec!["DEPT"]);
}
