use crate::asset::{
    Curve, CurveDescriptor, HeaderItem, HeaderSection, IndexDescriptor, IngestIssue, IssueSeverity,
    LasAsset, LasAssetSummary, Provenance, derive_asset_id, derive_canonical_alias,
    derive_index_kind,
};
use crate::{LasError, Result};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn import_las_file(path: impl AsRef<Path>) -> Result<LasAsset> {
    let path = path.as_ref();
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n");

    let mut issues = Vec::new();
    let sections = capture_sections(&text, &mut issues)?;

    if sections
        .iter()
        .any(|section| section.kind == SectionKind::UnsupportedLas3)
    {
        return Err(LasError::Unsupported(String::from(
            "LAS 3 section groups are present. This v1 implementation supports LAS 1.2/2.0 only.",
        )));
    }

    let mut headers = Vec::new();
    let mut version_items = Vec::new();
    let mut well_items = Vec::new();
    let mut curve_items = Vec::new();
    let mut data_lines = Vec::new();

    for section in &sections {
        match section.kind {
            SectionKind::Version
            | SectionKind::Well
            | SectionKind::Curves
            | SectionKind::Parameter => {
                let parsed_items = parse_header_items(section, &mut issues);
                let header_section = HeaderSection {
                    name: canonical_section_name(section.kind),
                    title_line: section.title_line.clone(),
                    raw_body: join_body(section),
                    items: parsed_items.clone(),
                };
                match section.kind {
                    SectionKind::Version => version_items = parsed_items,
                    SectionKind::Well => well_items = parsed_items,
                    SectionKind::Curves => curve_items = parsed_items,
                    SectionKind::Parameter => {}
                    _ => {}
                }
                headers.push(header_section);
            }
            SectionKind::Other => {
                headers.push(HeaderSection {
                    name: canonical_section_name(section.kind),
                    title_line: section.title_line.clone(),
                    raw_body: join_body(section),
                    items: Vec::new(),
                });
            }
            SectionKind::Data => data_lines = section.body_lines.clone(),
            SectionKind::Unknown => {
                issues.push(IngestIssue {
                    severity: IssueSeverity::Warning,
                    code: String::from("UNKNOWN_SECTION"),
                    message: format!(
                        "Ignoring unsupported section title '{}'",
                        section.title_line.trim()
                    ),
                    line: Some(section.start_line_number),
                });
            }
            SectionKind::UnsupportedLas3 => {}
        }
    }

    if curve_items.is_empty() {
        return Err(LasError::Parse(String::from(
            "No ~Curve section could be parsed from the LAS file.",
        )));
    }

    if data_lines.is_empty() {
        return Err(LasError::Parse(String::from(
            "No ~A/~ASCII data section could be parsed from the LAS file.",
        )));
    }

    let las_version =
        find_item_value(&version_items, "VERS").unwrap_or_else(|| String::from("unknown"));
    let wrap_mode = find_item_value(&version_items, "WRAP").unwrap_or_else(|| String::from("NO"));
    let delimiter = find_item_value(&version_items, "DLM").unwrap_or_else(|| String::from("SPACE"));

    let wrap_yes = wrap_mode.trim().eq_ignore_ascii_case("YES");
    let null_value =
        find_item_value(&well_items, "NULL").and_then(|value| value.parse::<f64>().ok());

    let curve_descriptors = build_curve_descriptors(&curve_items);
    let columns = parse_data_rows(
        &data_lines,
        curve_descriptors.len(),
        wrap_yes,
        &delimiter,
        null_value,
        &mut issues,
    );

    let row_count = columns.first().map_or(0, Vec::len);
    let curves = curve_descriptors
        .into_iter()
        .zip(columns)
        .map(|(mut descriptor, samples)| {
            descriptor.sample_count = samples.len();
            Curve {
                descriptor,
                samples,
            }
        })
        .collect::<Vec<_>>();

    if curves.is_empty() {
        return Err(LasError::Parse(String::from(
            "No curve data rows could be parsed.",
        )));
    }

    let fingerprint = fingerprint_bytes(&bytes);
    let imported_at_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let provenance = Provenance::from_path(path, fingerprint.clone(), imported_at_unix_seconds);
    let asset_id = derive_asset_id(&fingerprint);

    let index_curve = curves
        .first()
        .ok_or_else(|| LasError::Parse(String::from("Expected at least one curve.")))?;
    let index = IndexDescriptor {
        curve_id: index_curve.descriptor.id.clone(),
        raw_mnemonic: index_curve.descriptor.raw_mnemonic.clone(),
        unit: index_curve.descriptor.unit.clone(),
        kind: derive_index_kind(&index_curve.descriptor.raw_mnemonic),
    };

    let summary = LasAssetSummary {
        asset_id,
        source_path: provenance.source_path.clone(),
        original_filename: provenance.original_filename.clone(),
        source_fingerprint: provenance.source_fingerprint.clone(),
        las_version,
        wrap_mode,
        delimiter,
        row_count,
        curve_count: curves.len(),
        issue_count: issues.len(),
    };

    Ok(LasAsset {
        summary,
        provenance,
        index,
        headers,
        curves,
        issues,
    })
}

fn capture_sections(text: &str, issues: &mut Vec<IngestIssue>) -> Result<Vec<SectionCapture>> {
    let mut sections = Vec::new();
    let mut current: Option<SectionCapture> = None;

    for (idx, line) in text.lines().enumerate() {
        let line_number = idx + 1;
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
                message: String::from(
                    "Ignoring content that appears before the first LAS section.",
                ),
                line: Some(line_number),
            });
        }
    }

    if let Some(section) = current.take() {
        sections.push(section);
    }

    if sections.is_empty() {
        return Err(LasError::Parse(String::from(
            "No LAS sections were found in the input.",
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
            return match () {
                _ if normalized.contains("_DEFINITION") => SectionKind::Curves,
                _ if normalized.contains("_DATA") => SectionKind::Data,
                _ if normalized.contains("_PARAMETER") => SectionKind::Parameter,
                _ => SectionKind::UnsupportedLas3,
            };
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

fn parse_header_items(section: &SectionCapture, issues: &mut Vec<IngestIssue>) -> Vec<HeaderItem> {
    let mut items = Vec::new();
    for (line_number, line) in &section.body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match parse_header_item(*line_number, line) {
            Some(item) => items.push(item),
            None => issues.push(IngestIssue {
                severity: IssueSeverity::Warning,
                code: String::from("UNPARSED_HEADER_LINE"),
                message: format!(
                    "Could not parse header line in {} section.",
                    canonical_section_name(section.kind)
                ),
                line: Some(*line_number),
            }),
        }
    }
    items
}

fn parse_header_item(line_number: usize, line: &str) -> Option<HeaderItem> {
    let colon = line.rfind(':')?;
    let head = &line[..colon];
    let description = line[colon + 1..].trim().to_string();
    let dot = head.find('.')?;
    let mnemonic = head[..dot].trim().to_string();
    if mnemonic.is_empty() {
        return None;
    }

    let after_dot = &head[dot + 1..];
    let split_at = after_dot
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx);

    let (unit, value) = match split_at {
        Some(idx) => (
            after_dot[..idx].trim().to_string(),
            after_dot[idx..].trim().to_string(),
        ),
        None => (after_dot.trim().to_string(), String::new()),
    };

    Some(HeaderItem {
        mnemonic,
        unit,
        value,
        description,
        line_number,
        raw_line: line.to_string(),
    })
}

fn join_body(section: &SectionCapture) -> String {
    section
        .body_lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_item_value(items: &[HeaderItem], mnemonic: &str) -> Option<String> {
    items
        .iter()
        .find(|item| item.mnemonic.trim().eq_ignore_ascii_case(mnemonic))
        .map(|item| item.value.trim().to_string())
}

fn build_curve_descriptors(curve_items: &[HeaderItem]) -> Vec<CurveDescriptor> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    curve_items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let base_name = if item.mnemonic.trim().is_empty() {
                String::from("UNKNOWN")
            } else {
                item.mnemonic.trim().to_string()
            };
            let count = counts.entry(base_name.clone()).or_insert(0);
            *count += 1;
            let id = if *count == 1 {
                base_name.clone()
            } else {
                format!("{base_name}:{}", *count)
            };

            CurveDescriptor {
                id,
                raw_mnemonic: item.mnemonic.clone(),
                unit: item.unit.clone(),
                value: item.value.clone(),
                description: item.description.clone(),
                canonical_alias: derive_canonical_alias(&item.mnemonic, &item.unit),
                sample_count: 0,
                is_index: index == 0,
            }
        })
        .collect()
}

fn parse_data_rows(
    lines: &[(usize, String)],
    expected_columns: usize,
    wrap_yes: bool,
    delimiter: &str,
    null_value: Option<f64>,
    issues: &mut Vec<IngestIssue>,
) -> Vec<Vec<f64>> {
    let mut columns = vec![Vec::new(); expected_columns];
    let mut buffered_tokens = Vec::<(usize, String)>::new();

    for (line_number, line) in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let tokens = split_data_line(trimmed, delimiter);
        if tokens.is_empty() {
            continue;
        }

        if wrap_yes {
            for token in tokens {
                buffered_tokens.push((*line_number, token));
                if buffered_tokens.len() == expected_columns {
                    emit_row(&buffered_tokens, &mut columns, null_value, issues);
                    buffered_tokens.clear();
                }
            }
        } else {
            if tokens.len() < expected_columns {
                issues.push(IngestIssue {
                    severity: IssueSeverity::Warning,
                    code: String::from("SHORT_DATA_ROW"),
                    message: format!(
                        "Expected {expected_columns} columns but found {}. Missing values were padded with NaN.",
                        tokens.len()
                    ),
                    line: Some(*line_number),
                });
            } else if tokens.len() > expected_columns {
                issues.push(IngestIssue {
                    severity: IssueSeverity::Warning,
                    code: String::from("LONG_DATA_ROW"),
                    message: format!(
                        "Expected {expected_columns} columns but found {}. Extra values were ignored.",
                        tokens.len()
                    ),
                    line: Some(*line_number),
                });
            }

            let padded_tokens = (0..expected_columns)
                .map(|idx| {
                    let token = tokens
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| String::from("NaN"));
                    (*line_number, token)
                })
                .collect::<Vec<_>>();
            emit_row(&padded_tokens, &mut columns, null_value, issues);
        }
    }

    if wrap_yes && !buffered_tokens.is_empty() {
        issues.push(IngestIssue {
            severity: IssueSeverity::Warning,
            code: String::from("TRAILING_WRAPPED_VALUES"),
            message: String::from(
                "Discarded trailing wrapped values that did not complete a full row.",
            ),
            line: buffered_tokens.first().map(|(line_number, _)| *line_number),
        });
    }

    columns
}

fn emit_row(
    tokens: &[(usize, String)],
    columns: &mut [Vec<f64>],
    null_value: Option<f64>,
    issues: &mut Vec<IngestIssue>,
) {
    for (idx, (_, token)) in tokens.iter().enumerate() {
        if idx >= columns.len() {
            break;
        }
        columns[idx].push(parse_numeric_token(
            token,
            null_value,
            issues,
            tokens[idx].0,
        ));
    }
}

fn parse_numeric_token(
    token: &str,
    null_value: Option<f64>,
    issues: &mut Vec<IngestIssue>,
    line_number: usize,
) -> f64 {
    let cleaned = token.trim().trim_matches('"');
    if cleaned.eq_ignore_ascii_case("NaN") {
        return f64::NAN;
    }

    let parsed = cleaned.parse::<f64>().unwrap_or_else(|_| {
        issues.push(IngestIssue {
            severity: IssueSeverity::Warning,
            code: String::from("NON_NUMERIC_VALUE"),
            message: format!("Converted non-numeric value '{cleaned}' to NaN."),
            line: Some(line_number),
        });
        f64::NAN
    });

    if let Some(null_value) = null_value {
        if parsed.is_finite() && (parsed - null_value).abs() < 1e-12 {
            return f64::NAN;
        }
    }

    parsed
}

fn split_data_line(line: &str, delimiter: &str) -> Vec<String> {
    if delimiter.trim().eq_ignore_ascii_case("COMMA") || line.contains(',') {
        line.split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(String::from)
            .collect()
    } else {
        line.split_whitespace().map(String::from).collect()
    }
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

fn canonical_section_name(kind: SectionKind) -> String {
    match kind {
        SectionKind::Version => String::from("Version"),
        SectionKind::Well => String::from("Well"),
        SectionKind::Curves => String::from("Curves"),
        SectionKind::Parameter => String::from("Parameter"),
        SectionKind::Other => String::from("Other"),
        SectionKind::Data => String::from("Data"),
        SectionKind::UnsupportedLas3 => String::from("UnsupportedLas3"),
        SectionKind::Unknown => String::from("Unknown"),
    }
}

#[allow(dead_code)]
fn _group_headers_by_name(headers: &[HeaderSection]) -> HashMap<String, Vec<HeaderSection>> {
    let mut grouped = HashMap::new();
    for section in headers {
        grouped
            .entry(section.name.clone())
            .or_insert_with(Vec::new)
            .push(section.clone());
    }
    grouped
}
