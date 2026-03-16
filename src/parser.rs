use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};
use lithos_core::{
    CurveItem, HeaderItem, IndexDescriptor, IngestIssue, IssueSeverity, LasFile, LasFileSummary,
    LasValue, MnemonicCase, Provenance, SectionItem, SectionItems, bundle_manifest_path,
    derive_index_kind, normalized_depth_unit,
};
use lithos_core::{LasError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NullPolicy {
    None,
    Strict,
    Common,
    Aggressive,
    All,
    Custom(Vec<NullRule>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NullRule {
    HeaderNull,
    Numeric(f64),
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    NumbersOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadPolicy {
    None,
    Default,
    Custom(Vec<(String, String)>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DType {
    Auto,
    Float,
    Integer,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DTypeSpec {
    Auto,
    AllText,
    PerColumn(Vec<DType>),
    PerMnemonic(BTreeMap<String, DType>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOptions {
    pub ignore_header_errors: bool,
    pub mnemonic_case: MnemonicCase,
    pub null_policy: NullPolicy,
    pub read_policy: ReadPolicy,
    pub index_unit: Option<String>,
    pub encoding: Option<String>,
    pub autodetect_encoding: bool,
    pub dtypes: DTypeSpec,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            ignore_header_errors: false,
            mnemonic_case: MnemonicCase::Preserve,
            null_policy: NullPolicy::Strict,
            read_policy: ReadPolicy::Default,
            index_unit: None,
            encoding: None,
            autodetect_encoding: true,
            dtypes: DTypeSpec::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKind {
    Version,
    Well,
    Curves,
    Parameter,
    Other,
    Data,
    UnsupportedLas3,
    Unknown,
}

#[derive(Debug, Clone)]
struct SectionCapture {
    kind: SectionKind,
    title_line: String,
    start_line_number: usize,
    body_lines: Vec<(usize, String)>,
}

#[derive(Debug, Clone)]
pub struct DecodedText {
    pub text: String,
    pub encoding_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHeaderLine {
    pub name: String,
    pub unit: String,
    pub value: String,
    pub description: String,
}

pub fn import_las_file(path: impl AsRef<Path>) -> Result<LasFile> {
    read_path(path, &ReadOptions::default())
}

pub fn read_path(path: impl AsRef<Path>, options: &ReadOptions) -> Result<LasFile> {
    let path = path.as_ref();
    let bytes = fs::read(path)?;
    let decoded = decode_bytes(&bytes, options)?;
    let imported_at_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let fingerprint = fingerprint_bytes(&bytes);
    let provenance = Provenance::from_path(path, fingerprint, imported_at_unix_seconds);
    parse_las_text(decoded, provenance, options)
}

pub fn read_string(text: &str, options: &ReadOptions) -> Result<LasFile> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let provenance = Provenance {
        source_path: String::from("<inline>"),
        original_filename: String::from("inline.las"),
        source_fingerprint: fingerprint_bytes(text.as_bytes()),
        imported_at_unix_seconds: now,
    };
    parse_las_text(
        DecodedText {
            text: normalize_newlines(text),
            encoding_label: Some(String::from("utf-8")),
        },
        provenance,
        options,
    )
}

pub fn read_reader<R: Read>(mut reader: R, options: &ReadOptions) -> Result<LasFile> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let decoded = decode_bytes(&bytes, options)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let provenance = Provenance {
        source_path: String::from("<reader>"),
        original_filename: String::from("reader.las"),
        source_fingerprint: fingerprint_bytes(&bytes),
        imported_at_unix_seconds: now,
    };
    parse_las_text(decoded, provenance, options)
}

pub fn decode_bytes(bytes: &[u8], options: &ReadOptions) -> Result<DecodedText> {
    if let Some(label) = &options.encoding {
        let encoding = lookup_encoding(label)
            .ok_or_else(|| LasError::Parse(format!("unknown encoding '{label}'")))?;
        let (text, _, _) = encoding.decode(bytes);
        return Ok(DecodedText {
            text: normalize_newlines(text.as_ref()),
            encoding_label: Some(label.clone()),
        });
    }

    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let (text, _) = UTF_8.decode_with_bom_removal(bytes);
        return Ok(DecodedText {
            text: normalize_newlines(text.as_ref()),
            encoding_label: Some(String::from("utf-8-sig")),
        });
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (text, _) = UTF_16LE.decode_with_bom_removal(bytes);
        return Ok(DecodedText {
            text: normalize_newlines(text.as_ref()),
            encoding_label: Some(String::from("utf-16le")),
        });
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, _) = UTF_16BE.decode_with_bom_removal(bytes);
        return Ok(DecodedText {
            text: normalize_newlines(text.as_ref()),
            encoding_label: Some(String::from("utf-16be")),
        });
    }

    if options.autodetect_encoding {
        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
            return Ok(DecodedText {
                text: normalize_newlines(&text),
                encoding_label: Some(String::from("utf-8")),
            });
        }
        if looks_like_utf16_le(bytes) {
            let (text, _, _) = UTF_16LE.decode(bytes);
            return Ok(DecodedText {
                text: normalize_newlines(text.as_ref()),
                encoding_label: Some(String::from("utf-16le")),
            });
        }
        if looks_like_utf16_be(bytes) {
            let (text, _, _) = UTF_16BE.decode(bytes);
            return Ok(DecodedText {
                text: normalize_newlines(text.as_ref()),
                encoding_label: Some(String::from("utf-16be")),
            });
        }
    }

    let (text, _, _) = WINDOWS_1252.decode(bytes);
    Ok(DecodedText {
        text: normalize_newlines(text.as_ref()),
        encoding_label: Some(String::from("windows-1252")),
    })
}

pub fn parse_header_line(line: &str, section_name: Option<&str>) -> Result<ParsedHeaderLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(LasError::Parse(String::from("empty header line")));
    }

    if let Some(first_colon) = trimmed.find(':') {
        let before_colon = &trimmed[..first_colon];
        if !before_colon.contains('.') {
            return Ok(ParsedHeaderLine {
                name: before_colon.trim().to_string(),
                unit: String::new(),
                value: trimmed[first_colon + 1..].trim().to_string(),
                description: String::new(),
            });
        }
    }

    let (head, description) = split_description(trimmed);
    let (name, unit, value) = split_name_unit_value(head, section_name);
    Ok(ParsedHeaderLine {
        name,
        unit: strip_brackets(&unit),
        value: value.trim().to_string(),
        description: description.trim().to_string(),
    })
}

fn parse_las_text(
    decoded: DecodedText,
    provenance: Provenance,
    options: &ReadOptions,
) -> Result<LasFile> {
    let mut issues = Vec::new();
    let sections = capture_sections(&decoded.text, &mut issues)?;

    if sections
        .iter()
        .any(|section| section.kind == SectionKind::UnsupportedLas3)
    {
        return Err(LasError::Unsupported(String::from(
            "LAS 3 section groups are present. This implementation handles non-v3 LAS only.",
        )));
    }

    let version_section = sections
        .iter()
        .find(|section| section.kind == SectionKind::Version);
    let version = if let Some(section) = version_section {
        parse_header_section(section, "Version", None, options, &mut issues)?
    } else {
        SectionItems::new(options.mnemonic_case)
    };
    let las_version_number = version.get("VERS").and_then(|item| item.value.as_f64());

    let mut well = SectionItems::new(options.mnemonic_case);
    let mut params = SectionItems::new(options.mnemonic_case);
    let mut curves = SectionItems::new(options.mnemonic_case);
    let mut other = String::new();
    let mut extra_sections = BTreeMap::new();
    let mut data_lines = Vec::new();

    for section in &sections {
        match section.kind {
            SectionKind::Version => {}
            SectionKind::Well => {
                well =
                    parse_header_section(section, "Well", las_version_number, options, &mut issues)?
            }
            SectionKind::Parameter => {
                params = parse_header_section(
                    section,
                    "Parameter",
                    las_version_number,
                    options,
                    &mut issues,
                )?
            }
            SectionKind::Curves => curves = parse_curve_section(section, options, &mut issues)?,
            SectionKind::Other => other = join_body(section),
            SectionKind::Data => data_lines = section.body_lines.clone(),
            SectionKind::Unknown => {
                extra_sections.insert(extra_section_key(&section.title_line), join_body(section));
                issues.push(IngestIssue {
                    severity: IssueSeverity::Warning,
                    code: String::from("UNKNOWN_SECTION"),
                    message: format!("preserving unknown section '{}'", section.title_line.trim()),
                    line: Some(section.start_line_number),
                });
            }
            SectionKind::UnsupportedLas3 => {}
        }
    }

    if curves.is_empty() {
        return Err(LasError::Parse(String::from(
            "no curve section could be parsed from the LAS file",
        )));
    }

    let las_version = version
        .get("VERS")
        .and_then(|item| item.value.as_f64())
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("unknown"));
    let wrap_mode = version
        .get("WRAP")
        .map(|item| item.value.display_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("NO"));
    let delimiter = version
        .get("DLM")
        .map(|item| item.value.display_string())
        .unwrap_or_else(|| infer_delimiter(&data_lines));
    let null_value = well.get("NULL").and_then(|item| item.value.as_f64());

    let read_subs = compile_read_substitutions(&options.read_policy)?;
    let null_rules = resolve_null_rules(&options.null_policy, null_value)?;
    let parsed_curve_data = parse_data_section(
        &data_lines,
        curves.keys(),
        &delimiter,
        wrap_mode.eq_ignore_ascii_case("YES"),
        &read_subs,
        &null_rules,
        &options.dtypes,
        &mut issues,
    )?;

    for curve in curves.as_mut_slice() {
        curve.data = parsed_curve_data
            .get(curve.session_mnemonic())
            .cloned()
            .unwrap_or_default();
    }

    let index_curve = curves
        .get_index(0)
        .ok_or_else(|| LasError::Parse(String::from("expected index curve")))?;
    let detected_index_unit = options
        .index_unit
        .clone()
        .or_else(|| detect_index_unit(index_curve, &well));

    let summary = LasFileSummary {
        source_path: provenance.source_path.clone(),
        original_filename: provenance.original_filename.clone(),
        source_fingerprint: provenance.source_fingerprint.clone(),
        las_version,
        wrap_mode,
        delimiter,
        row_count: index_curve.data.len(),
        curve_count: curves.len(),
        issue_count: issues.len(),
    };

    Ok(LasFile {
        summary,
        provenance,
        encoding: decoded.encoding_label,
        index: IndexDescriptor {
            curve_id: index_curve.session_mnemonic().to_string(),
            raw_mnemonic: index_curve.original_mnemonic.clone(),
            unit: index_curve.unit.clone(),
            kind: derive_index_kind(index_curve.session_mnemonic()),
        },
        version,
        well,
        params,
        curves,
        other,
        extra_sections,
        issues,
        index_unit: detected_index_unit,
    })
}

fn capture_sections(text: &str, issues: &mut Vec<IngestIssue>) -> Result<Vec<SectionCapture>> {
    let mut sections = Vec::new();
    let mut current: Option<SectionCapture> = None;

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with('~') {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(SectionCapture {
                kind: classify_section(trimmed),
                title_line: line.to_string(),
                start_line_number: line_number,
                body_lines: Vec::new(),
            });
        } else if let Some(section) = current.as_mut() {
            section.body_lines.push((line_number, line.to_string()));
        } else if !trimmed.is_empty() {
            issues.push(IngestIssue {
                severity: IssueSeverity::Warning,
                code: String::from("CONTENT_OUTSIDE_SECTION"),
                message: String::from("ignoring content before first LAS section"),
                line: Some(line_number),
            });
        }
    }

    if let Some(section) = current.take() {
        sections.push(section);
    }

    if sections.is_empty() {
        return Err(LasError::Parse(String::from(
            "no LAS sections were found in the input",
        )));
    }

    Ok(sections)
}

fn classify_section(title: &str) -> SectionKind {
    let normalized = title.trim().to_ascii_uppercase();
    if normalized.contains("_DEFINITION")
        || normalized.contains("_DATA")
        || normalized.contains("_PARAMETER")
    {
        if normalized.contains("~LOG_") {
            if normalized.contains("_DEFINITION") {
                return SectionKind::Curves;
            }
            if normalized.contains("_PARAMETER") {
                return SectionKind::Parameter;
            }
            if normalized.contains("_DATA") {
                return SectionKind::Data;
            }
        }
        return SectionKind::UnsupportedLas3;
    }

    if normalized.starts_with("~A")
        || normalized.starts_with("~ASCII")
        || normalized.starts_with("~ASC")
    {
        SectionKind::Data
    } else if normalized.starts_with("~V") || normalized.contains("VERSION") {
        SectionKind::Version
    } else if normalized.starts_with("~W") || normalized.contains("WELL") {
        SectionKind::Well
    } else if normalized.starts_with("~C") || normalized.contains("CURVE") {
        SectionKind::Curves
    } else if normalized.starts_with("~P") || normalized.contains("PARAMETER") {
        SectionKind::Parameter
    } else if normalized.starts_with("~O") || normalized.contains("OTHER") {
        SectionKind::Other
    } else {
        SectionKind::Unknown
    }
}

fn parse_header_section(
    section: &SectionCapture,
    section_name: &str,
    las_version_number: Option<f64>,
    options: &ReadOptions,
    issues: &mut Vec<IngestIssue>,
) -> Result<SectionItems<HeaderItem>> {
    let mut items = Vec::new();
    for (line_number, line) in &section.body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match parse_header_line(trimmed, Some(section_name)) {
            Ok(mut parsed) => {
                let name = apply_mnemonic_case(&parsed.name, options.mnemonic_case);
                parsed.name = name.clone();
                if should_swap_well_fields(las_version_number, section_name, &name) {
                    std::mem::swap(&mut parsed.value, &mut parsed.description);
                }
                items.push(HeaderItem {
                    mnemonic: String::new(),
                    original_mnemonic: name.clone(),
                    unit: parsed.unit,
                    value: parse_header_value(&name, &parsed.value),
                    description: parsed.description,
                    line_number: *line_number,
                    raw_line: line.clone(),
                });
            }
            Err(err) if options.ignore_header_errors => issues.push(IngestIssue {
                severity: IssueSeverity::Warning,
                code: String::from("HEADER_PARSE_ERROR"),
                message: err.to_string(),
                line: Some(*line_number),
            }),
            Err(err) => {
                return Err(LasError::Parse(format!(
                    "{} section line {} failed: {}",
                    section_name, line_number, err
                )));
            }
        }
    }

    Ok(SectionItems::from_items(items, options.mnemonic_case))
}

fn parse_curve_section(
    section: &SectionCapture,
    options: &ReadOptions,
    issues: &mut Vec<IngestIssue>,
) -> Result<SectionItems<CurveItem>> {
    let mut items = Vec::new();
    for (line_number, line) in &section.body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match parse_header_line(trimmed, Some("Curves")) {
            Ok(parsed) => {
                items.push(CurveItem {
                    mnemonic: String::new(),
                    original_mnemonic: apply_mnemonic_case(&parsed.name, options.mnemonic_case),
                    unit: parsed.unit,
                    value: LasValue::from(parsed.value),
                    description: parsed.description,
                    data: Vec::new(),
                });
            }
            Err(err) if options.ignore_header_errors => issues.push(IngestIssue {
                severity: IssueSeverity::Warning,
                code: String::from("CURVE_PARSE_ERROR"),
                message: err.to_string(),
                line: Some(*line_number),
            }),
            Err(err) => {
                return Err(LasError::Parse(format!(
                    "curve section line {} failed: {}",
                    line_number, err
                )));
            }
        }
    }

    Ok(SectionItems::from_items(items, options.mnemonic_case))
}

fn parse_data_section(
    lines: &[(usize, String)],
    curve_names: Vec<String>,
    delimiter: &str,
    wrapped: bool,
    read_subs: &[(Regex, String)],
    null_rules: &[ResolvedNullRule],
    dtypes: &DTypeSpec,
    issues: &mut Vec<IngestIssue>,
) -> Result<BTreeMap<String, Vec<LasValue>>> {
    if curve_names.is_empty() {
        return Ok(BTreeMap::new());
    }

    let expected_columns = curve_names.len();
    let mut raw_rows = Vec::<Vec<String>>::new();
    let mut wrapped_buffer = Vec::<String>::new();
    let mut row_lengths = Vec::<usize>::new();

    for (line_number, line) in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let normalized = apply_substitutions(trimmed, read_subs);
        let tokens = split_data_line(&normalized, delimiter);
        if tokens.is_empty() {
            continue;
        }

        if wrapped {
            for token in tokens {
                wrapped_buffer.push(token);
                if wrapped_buffer.len() == expected_columns {
                    raw_rows.push(wrapped_buffer.clone());
                    row_lengths.push(expected_columns);
                    wrapped_buffer.clear();
                }
            }
            continue;
        }

        row_lengths.push(tokens.len());
        if tokens.iter().any(|value| value.contains('*')) {
            return Err(LasError::Parse(format!(
                "line {} contains non-numeric placeholder data",
                line_number
            )));
        }
        raw_rows.push(tokens);
    }

    if wrapped && !wrapped_buffer.is_empty() {
        issues.push(IngestIssue {
            severity: IssueSeverity::Warning,
            code: String::from("TRAILING_WRAPPED_VALUES"),
            message: String::from("discarded trailing wrapped values that did not form a full row"),
            line: None,
        });
    }

    let min_columns = row_lengths.iter().copied().min().unwrap_or(0);
    let max_columns = row_lengths.iter().copied().max().unwrap_or(0);
    if max_columns > expected_columns {
        return Err(LasError::Parse(format!(
            "data rows contain more values ({max_columns}) than defined curves ({expected_columns})"
        )));
    }

    let allow_padding = max_columns < expected_columns && min_columns == max_columns;
    if min_columns != max_columns && !allow_padding {
        return Err(LasError::Parse(String::from(
            "data rows have inconsistent column counts",
        )));
    }

    let mut columns = vec![Vec::<LasValue>::new(); expected_columns];
    for row in raw_rows {
        let normalized_row = if row.len() < expected_columns && allow_padding {
            let mut values = row.clone();
            values.resize(expected_columns, String::from("NaN"));
            values
        } else {
            row
        };

        for (index, name) in curve_names.iter().enumerate() {
            let token = normalized_row
                .get(index)
                .cloned()
                .unwrap_or_else(|| String::from("NaN"));
            let curve_dtype = resolve_dtype(dtypes, index, name);
            columns[index].push(parse_token_to_value(&token, curve_dtype, null_rules));
        }
    }

    let mut result = BTreeMap::new();
    for (name, values) in curve_names.into_iter().zip(columns) {
        result.insert(name, values);
    }
    Ok(result)
}

fn parse_token_to_value(token: &str, dtype: DType, null_rules: &[ResolvedNullRule]) -> LasValue {
    let trimmed = token.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return LasValue::Empty;
    }

    if let Some(value) = apply_null_rules(trimmed, null_rules) {
        return value;
    }

    match dtype {
        DType::Text => LasValue::Text(trimmed.to_string()),
        DType::Integer => trimmed
            .parse::<i64>()
            .map(|value| LasValue::Text(value.to_string()))
            .unwrap_or_else(|_| LasValue::Text(trimmed.to_string())),
        DType::Float | DType::Auto => trimmed
            .parse::<f64>()
            .map(LasValue::Number)
            .unwrap_or_else(|_| LasValue::Text(trimmed.to_string())),
    }
}

fn parse_header_value(name: &str, raw_value: &str) -> LasValue {
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return LasValue::Empty;
    }

    let upper = name.trim().to_ascii_uppercase();
    if matches!(upper.as_str(), "API" | "UWI") || looks_like_preserved_identifier(trimmed) {
        return LasValue::Text(trimmed.to_string());
    }

    trimmed
        .parse::<f64>()
        .map(LasValue::Number)
        .unwrap_or_else(|_| LasValue::Text(trimmed.to_string()))
}

fn looks_like_preserved_identifier(value: &str) -> bool {
    (value.starts_with('0') && value.chars().all(|ch| ch.is_ascii_digit()))
        || value.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn split_description(line: &str) -> (&str, &str) {
    for (index, ch) in line.char_indices() {
        if ch == ':' {
            let next = line[index + 1..].chars().next();
            if next.is_none() || next.is_some_and(|value| value.is_whitespace()) {
                return (&line[..index], line[index + 1..].trim_start());
            }
        }
    }

    if let Some(index) = line.rfind(':') {
        (&line[..index], line[index + 1..].trim_start())
    } else {
        (line, "")
    }
}

fn split_name_unit_value(head: &str, section_name: Option<&str>) -> (String, String, String) {
    let trimmed = head.trim();
    if let Some(curves) = try_curve_double_dot_patterns(trimmed, section_name) {
        return curves;
    }

    let Some(dot_index) = trimmed.find('.') else {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default().trim().to_string();
        let value = parts.next().unwrap_or_default().trim().to_string();
        return (name, String::new(), value);
    };

    let name = trimmed[..dot_index].trim().to_string();
    let rest = &trimmed[dot_index + 1..];
    if rest.chars().next().is_none_or(char::is_whitespace) {
        return (name, String::new(), rest.trim().to_string());
    }

    let unit_pattern =
        Regex::new(r"^(?P<unit>(?:[0-9]+\s+)?\S+)(?P<value>.*)$").expect("valid unit regex");
    if let Some(captures) = unit_pattern.captures(rest) {
        return (
            name,
            captures["unit"].trim().trim_end_matches('.').to_string(),
            captures
                .name("value")
                .map(|value| value.as_str().trim().to_string())
                .unwrap_or_default(),
        );
    }

    (name, String::new(), rest.trim().to_string())
}

fn try_curve_double_dot_patterns(
    line: &str,
    section_name: Option<&str>,
) -> Option<(String, String, String)> {
    if section_name != Some("Curves") || !line.contains("..") {
        return None;
    }

    let dotted_unit =
        Regex::new(r"^(?P<name>.+?)\s+\.\.(?P<unit>\S*)(?:\s+(?P<value>.*))?$").ok()?;
    if let Some(captures) = dotted_unit.captures(line) {
        return Some((
            captures["name"].trim().to_string(),
            format!(
                ".{}",
                captures
                    .name("unit")
                    .map(|value| value.as_str())
                    .unwrap_or("")
            ),
            captures
                .name("value")
                .map(|value| value.as_str().trim().to_string())
                .unwrap_or_default(),
        ));
    }

    let dotted_name = Regex::new(r"^(?P<name>.+\.)\.(?P<unit>\S*)(?:\s+(?P<value>.*))?$").ok()?;
    dotted_name.captures(line).map(|captures| {
        (
            captures["name"].trim().to_string(),
            captures["unit"].trim().to_string(),
            captures
                .name("value")
                .map(|value| value.as_str().trim().to_string())
                .unwrap_or_default(),
        )
    })
}

fn strip_brackets(unit: &str) -> String {
    let trimmed = unit.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('[') && trimmed.ends_with(']'))
            || (trimmed.starts_with('(') && trimmed.ends_with(')')))
    {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    trimmed.to_string()
}

fn split_data_line(line: &str, delimiter: &str) -> Vec<String> {
    match delimiter.trim().to_ascii_uppercase().as_str() {
        "COMMA" => line
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(String::from)
            .collect(),
        "TAB" => split_quoted_tokens(line, '\t'),
        _ => split_whitespace_tokens(line),
    }
}

fn split_whitespace_tokens(line: &str) -> Vec<String> {
    let token_regex =
        Regex::new(r#"([^\s"']+)|"([^"]*)"|'([^']*)'"#).expect("valid whitespace regex");
    token_regex
        .captures_iter(line)
        .filter_map(|captures| {
            captures.get(0).map(|value| {
                value
                    .as_str()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string()
            })
        })
        .collect()
}

fn split_quoted_tokens(line: &str, delimiter: char) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for ch in line.chars() {
        if ch == '"' || ch == '\'' {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch == delimiter && quote.is_none() {
            if !current.trim().is_empty() {
                tokens.push(current.trim().to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

fn apply_substitutions(line: &str, substitutions: &[(Regex, String)]) -> String {
    let mut current = line.to_string();
    for (pattern, replacement) in substitutions {
        current = pattern
            .replace_all(&current, replacement.as_str())
            .into_owned();
    }
    current
}

#[derive(Debug, Clone)]
enum ResolvedNullRule {
    HeaderNull(f64),
    Numeric(f64),
    RegexReplace(Regex, String),
    NumbersOnly(Regex),
}

fn resolve_null_rules(
    policy: &NullPolicy,
    header_null: Option<f64>,
) -> Result<Vec<ResolvedNullRule>> {
    let mut rules = Vec::new();
    match policy {
        NullPolicy::None => {}
        NullPolicy::Strict => {
            if let Some(value) = header_null {
                rules.push(ResolvedNullRule::HeaderNull(value));
            }
        }
        NullPolicy::Common => {
            if let Some(value) = header_null {
                rules.push(ResolvedNullRule::HeaderNull(value));
            }
            for value in [999.25, -999.25, 9999.25, -9999.25] {
                rules.push(ResolvedNullRule::Numeric(value));
            }
            rules.push(ResolvedNullRule::RegexReplace(
                Regex::new(
                    r"(?i)^\(null\)$|^NULL$|^NA$|^-?1\.#INF[0-9]*$|^-?1\.#IO$|^-?1\.#IND[0-9]*$",
                )
                .expect("valid common null regex"),
                String::from("NaN"),
            ));
        }
        NullPolicy::Aggressive | NullPolicy::All => {
            rules.extend(resolve_null_rules(&NullPolicy::Common, header_null)?);
            for value in [999.0, -999.0, 9999.0, -9999.0, 32767.0, -32767.0] {
                rules.push(ResolvedNullRule::Numeric(value));
            }
            rules.push(ResolvedNullRule::RegexReplace(
                Regex::new(r"^-0\.0+$").expect("valid aggressive null regex"),
                String::from("NaN"),
            ));
            if *policy == NullPolicy::All {
                rules.push(ResolvedNullRule::NumbersOnly(
                    Regex::new(r"[^ 0-9.\-+]").expect("valid numbers-only regex"),
                ));
            }
        }
        NullPolicy::Custom(custom_rules) => {
            for rule in custom_rules {
                match rule {
                    NullRule::HeaderNull => {
                        if let Some(value) = header_null {
                            rules.push(ResolvedNullRule::HeaderNull(value));
                        }
                    }
                    NullRule::Numeric(value) => rules.push(ResolvedNullRule::Numeric(*value)),
                    NullRule::RegexReplace {
                        pattern,
                        replacement,
                    } => rules.push(ResolvedNullRule::RegexReplace(
                        Regex::new(pattern)
                            .map_err(|err| LasError::Parse(format!("invalid null regex: {err}")))?,
                        replacement.clone(),
                    )),
                    NullRule::NumbersOnly => rules.push(ResolvedNullRule::NumbersOnly(
                        Regex::new(r"[^ 0-9.\-+]").expect("valid numbers-only regex"),
                    )),
                }
            }
        }
    }
    Ok(rules)
}

fn apply_null_rules(token: &str, rules: &[ResolvedNullRule]) -> Option<LasValue> {
    for rule in rules {
        match rule {
            ResolvedNullRule::HeaderNull(value) | ResolvedNullRule::Numeric(value) => {
                if token
                    .parse::<f64>()
                    .ok()
                    .is_some_and(|parsed| (parsed - value).abs() < 1e-12)
                {
                    return Some(LasValue::Number(f64::NAN));
                }
            }
            ResolvedNullRule::RegexReplace(pattern, replacement) => {
                if pattern.is_match(token) {
                    if replacement.eq_ignore_ascii_case("NaN") {
                        return Some(LasValue::Number(f64::NAN));
                    }
                    return Some(LasValue::Text(replacement.clone()));
                }
            }
            ResolvedNullRule::NumbersOnly(pattern) => {
                if pattern.is_match(token) {
                    return Some(LasValue::Number(f64::NAN));
                }
            }
        }
    }
    None
}

fn compile_read_substitutions(policy: &ReadPolicy) -> Result<Vec<(Regex, String)>> {
    match policy {
        ReadPolicy::None => Ok(Vec::new()),
        ReadPolicy::Default => Ok(vec![
            (
                Regex::new(r"(\d),(\d)").expect("valid decimal comma regex"),
                String::from("$1.$2"),
            ),
            (
                Regex::new(r"(\d)-(\d)").expect("valid run-on hyphen regex"),
                String::from("$1 -$2"),
            ),
            (
                Regex::new(r"-?\d*\.\d*\.\d*|NaN[\.-]\d+").expect("valid run-on NaN regex"),
                String::from(" NaN NaN "),
            ),
        ]),
        ReadPolicy::Custom(values) => values
            .iter()
            .map(|(pattern, replacement)| {
                Ok((
                    Regex::new(pattern)
                        .map_err(|err| LasError::Parse(format!("invalid read regex: {err}")))?,
                    replacement.clone(),
                ))
            })
            .collect(),
    }
}

fn resolve_dtype(dtypes: &DTypeSpec, index: usize, mnemonic: &str) -> DType {
    match dtypes {
        DTypeSpec::Auto => DType::Auto,
        DTypeSpec::AllText => DType::Text,
        DTypeSpec::PerColumn(values) => values.get(index).cloned().unwrap_or(DType::Auto),
        DTypeSpec::PerMnemonic(values) => values.get(mnemonic).cloned().unwrap_or(DType::Auto),
    }
}

fn join_body(section: &SectionCapture) -> String {
    section
        .body_lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn apply_mnemonic_case(name: &str, mnemonic_case: MnemonicCase) -> String {
    match mnemonic_case {
        MnemonicCase::Preserve => name.to_string(),
        MnemonicCase::Upper => name.to_ascii_uppercase(),
        MnemonicCase::Lower => name.to_ascii_lowercase(),
    }
}

fn infer_delimiter(lines: &[(usize, String)]) -> String {
    if lines.iter().any(|(_, line)| line.contains('\t')) {
        String::from("TAB")
    } else if lines.iter().any(|(_, line)| line.contains(',')) {
        String::from("COMMA")
    } else {
        String::from("SPACE")
    }
}

fn detect_index_unit(index_curve: &CurveItem, well: &SectionItems<HeaderItem>) -> Option<String> {
    let mut discovered = Vec::new();
    if let Some(unit) = normalized_depth_unit(Some(&index_curve.unit)) {
        discovered.push(unit);
    }

    for mnemonic in ["STRT", "STOP", "STEP"] {
        if let Some(unit) = normalized_depth_unit(well.get(mnemonic).map(|item| item.unit.as_str()))
        {
            discovered.push(unit);
        }
    }

    let first = discovered.first()?;
    if discovered.iter().all(|unit| unit == first) {
        Some((*first).to_string())
    } else {
        None
    }
}

fn extra_section_key(title_line: &str) -> String {
    title_line.trim().trim_start_matches('~').trim().to_string()
}

fn should_swap_well_fields(
    las_version_number: Option<f64>,
    section_name: &str,
    mnemonic: &str,
) -> bool {
    if !section_name.eq_ignore_ascii_case("Well") {
        return false;
    }

    let Some(version) = las_version_number else {
        return false;
    };
    if version >= 2.0 {
        return false;
    }

    !matches!(
        mnemonic.trim().to_ascii_uppercase().as_str(),
        "STRT" | "STOP" | "STEP" | "NULL"
    )
}

fn lookup_encoding(label: &str) -> Option<&'static Encoding> {
    let normalized = label.to_ascii_lowercase();
    match normalized.as_str() {
        "utf-8" | "utf8" | "utf-8-sig" => Some(UTF_8),
        "utf-16" | "utf-16le" | "utf16le" => Some(UTF_16LE),
        "utf-16be" | "utf16be" => Some(UTF_16BE),
        "cp1252" | "windows-1252" => Some(WINDOWS_1252),
        "latin-1" | "iso-8859-1" => Encoding::for_label(b"iso-8859-1"),
        _ => None,
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn looks_like_utf16_le(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .skip(1)
        .step_by(2)
        .take(16)
        .filter(|byte| **byte == 0)
        .count()
        >= 4
}

fn looks_like_utf16_be(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .step_by(2)
        .take(16)
        .filter(|byte| **byte == 0)
        .count()
        >= 4
}

fn fingerprint_bytes(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

#[allow(dead_code)]
fn _bundle_manifest_path(path: &Path) -> PathBuf {
    bundle_manifest_path(path)
}
