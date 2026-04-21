use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source_path: String,
    pub original_filename: String,
    pub source_fingerprint: String,
    pub imported_at_unix_seconds: u64,
}

impl Provenance {
    pub fn from_path(
        path: &Path,
        source_fingerprint: String,
        imported_at_unix_seconds: u64,
    ) -> Self {
        let original_filename = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("unknown.las"));

        Self {
            source_path: path.to_string_lossy().into_owned(),
            original_filename,
            source_fingerprint,
            imported_at_unix_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LasValue {
    Number(f64),
    Text(String),
    Empty,
}

impl LasValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn is_nan(&self) -> bool {
        matches!(self, Self::Number(value) if value.is_nan())
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn display_string(&self) -> String {
        match self {
            Self::Number(value) => value.to_string(),
            Self::Text(value) => value.clone(),
            Self::Empty => String::new(),
        }
    }
}

impl From<f64> for LasValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for LasValue {
    fn from(value: String) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Text(value)
        }
    }
}

impl From<&str> for LasValue {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MnemonicCase {
    Preserve,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexKind {
    Depth,
    Time,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalAlias {
    pub mnemonic: Option<String>,
    pub unit_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderItem {
    pub mnemonic: String,
    pub original_mnemonic: String,
    pub unit: String,
    pub value: LasValue,
    pub description: String,
    pub line_number: usize,
    pub raw_line: String,
}

impl HeaderItem {
    pub fn new(
        mnemonic: impl Into<String>,
        unit: impl Into<String>,
        value: impl Into<LasValue>,
        description: impl Into<String>,
    ) -> Self {
        let original_mnemonic = mnemonic.into();
        let mnemonic = useful_mnemonic(&original_mnemonic);
        Self {
            mnemonic,
            original_mnemonic,
            unit: unit.into(),
            value: value.into(),
            description: description.into(),
            line_number: 0,
            raw_line: String::new(),
        }
    }

    pub fn rename(&mut self, mnemonic: impl Into<String>) {
        self.original_mnemonic = mnemonic.into();
        self.mnemonic = useful_mnemonic(&self.original_mnemonic);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveItem {
    pub mnemonic: String,
    pub original_mnemonic: String,
    pub unit: String,
    pub value: LasValue,
    pub description: String,
    pub data: Vec<LasValue>,
}

impl CurveItem {
    pub fn new(
        mnemonic: impl Into<String>,
        unit: impl Into<String>,
        value: impl Into<LasValue>,
        description: impl Into<String>,
        data: Vec<LasValue>,
    ) -> Self {
        let original_mnemonic = mnemonic.into();
        let mnemonic = useful_mnemonic(&original_mnemonic);
        Self {
            mnemonic,
            original_mnemonic,
            unit: unit.into(),
            value: value.into(),
            description: description.into(),
            data,
        }
    }

    pub fn rename(&mut self, mnemonic: impl Into<String>) {
        self.original_mnemonic = mnemonic.into();
        self.mnemonic = useful_mnemonic(&self.original_mnemonic);
    }

    pub fn numeric_data(&self) -> Option<Vec<f64>> {
        self.data.iter().map(LasValue::as_f64).collect()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

pub trait SectionItem: Clone {
    fn session_mnemonic(&self) -> &str;
    fn original_mnemonic(&self) -> &str;
    fn useful_mnemonic(&self) -> String;
    fn set_session_mnemonic_only(&mut self, mnemonic: String);
}

impl SectionItem for HeaderItem {
    fn session_mnemonic(&self) -> &str {
        &self.mnemonic
    }

    fn original_mnemonic(&self) -> &str {
        &self.original_mnemonic
    }

    fn useful_mnemonic(&self) -> String {
        useful_mnemonic(&self.original_mnemonic)
    }

    fn set_session_mnemonic_only(&mut self, mnemonic: String) {
        self.mnemonic = mnemonic;
    }
}

impl SectionItem for CurveItem {
    fn session_mnemonic(&self) -> &str {
        &self.mnemonic
    }

    fn original_mnemonic(&self) -> &str {
        &self.original_mnemonic
    }

    fn useful_mnemonic(&self) -> String {
        useful_mnemonic(&self.original_mnemonic)
    }

    fn set_session_mnemonic_only(&mut self, mnemonic: String) {
        self.mnemonic = mnemonic;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionItems<T: SectionItem> {
    items: Vec<T>,
    pub mnemonic_case: MnemonicCase,
}

impl<T: SectionItem> Default for SectionItems<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            mnemonic_case: MnemonicCase::Preserve,
        }
    }
}

impl<T: SectionItem> SectionItems<T> {
    pub fn new(mnemonic_case: MnemonicCase) -> Self {
        Self {
            items: Vec::new(),
            mnemonic_case,
        }
    }

    pub fn from_items(items: Vec<T>, mnemonic_case: MnemonicCase) -> Self {
        let mut section = Self {
            items,
            mnemonic_case,
        };
        section.assign_duplicate_suffixes(None);
        section
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut()
    }

    pub fn keys(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| item.session_mnemonic().to_string())
            .collect()
    }

    pub fn contains(&self, mnemonic: &str) -> bool {
        self.items
            .iter()
            .any(|item| mnemonic_match(self.mnemonic_case, item.session_mnemonic(), mnemonic))
    }

    pub fn get(&self, mnemonic: &str) -> Option<&T> {
        self.items
            .iter()
            .find(|item| mnemonic_match(self.mnemonic_case, item.session_mnemonic(), mnemonic))
    }

    pub fn get_mut(&mut self, mnemonic: &str) -> Option<&mut T> {
        self.items
            .iter_mut()
            .find(|item| mnemonic_match(self.mnemonic_case, item.session_mnemonic(), mnemonic))
    }

    pub fn get_index(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn get_index_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    pub fn get_index_signed(&self, index: isize) -> Option<&T> {
        if index >= 0 {
            self.get_index(index as usize)
        } else {
            let offset = self.items.len() as isize + index;
            if offset < 0 {
                None
            } else {
                self.items.get(offset as usize)
            }
        }
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        Self::from_items(self.items[range].to_vec(), self.mnemonic_case)
    }

    pub fn push(&mut self, item: T) {
        let test_mnemonic = item.useful_mnemonic();
        self.items.push(item);
        self.assign_duplicate_suffixes(Some(test_mnemonic));
    }

    pub fn insert(&mut self, index: usize, item: T) {
        let test_mnemonic = item.useful_mnemonic();
        self.items.insert(index, item);
        self.assign_duplicate_suffixes(Some(test_mnemonic));
    }

    pub fn delete_index(&mut self, index: usize) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }
        let item = self.items.remove(index);
        self.assign_duplicate_suffixes(None);
        Some(item)
    }

    pub fn delete_mnemonic(&mut self, mnemonic: &str) -> Option<T> {
        let index = self.items.iter().position(|item| {
            mnemonic_match(self.mnemonic_case, item.session_mnemonic(), mnemonic)
        })?;
        let item = self.items.remove(index);
        self.assign_duplicate_suffixes(None);
        Some(item)
    }

    pub fn set_item(&mut self, mnemonic: &str, new_item: T) {
        if let Some(index) = self
            .items
            .iter()
            .position(|item| mnemonic_match(self.mnemonic_case, item.session_mnemonic(), mnemonic))
        {
            self.items[index] = new_item;
        } else {
            self.items.push(new_item);
        }
        self.assign_duplicate_suffixes(None);
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items
    }

    fn assign_duplicate_suffixes(&mut self, test_mnemonic: Option<String>) {
        if let Some(test_mnemonic) = test_mnemonic {
            self.assign_duplicate_suffix_for(&test_mnemonic);
            return;
        }

        let mut unique = BTreeMap::<String, usize>::new();
        for item in &self.items {
            unique.insert(item.useful_mnemonic(), 0);
        }
        for mnemonic in unique.keys().cloned().collect::<Vec<_>>() {
            self.assign_duplicate_suffix_for(&mnemonic);
        }
    }

    fn assign_duplicate_suffix_for(&mut self, test_mnemonic: &str) {
        let mut matching = self
            .items
            .iter_mut()
            .filter(|item| {
                mnemonic_match(self.mnemonic_case, &item.useful_mnemonic(), test_mnemonic)
            })
            .collect::<Vec<_>>();

        if matching.len() <= 1 {
            if let Some(item) = matching.first_mut() {
                item.set_session_mnemonic_only(item.useful_mnemonic());
            }
            return;
        }

        for (index, item) in matching.into_iter().enumerate() {
            item.set_session_mnemonic_only(format!("{}:{}", item.useful_mnemonic(), index + 1));
        }
    }
}

impl SectionItems<HeaderItem> {
    pub fn get_or_create(
        &mut self,
        mnemonic: &str,
        default: Option<HeaderItem>,
        add: bool,
    ) -> HeaderItem {
        if let Some(item) = self.get(mnemonic) {
            return item.clone();
        }

        let mut item =
            default.unwrap_or_else(|| HeaderItem::new(mnemonic, "", LasValue::Empty, ""));
        item.rename(mnemonic);
        if add {
            self.push(item.clone());
        }
        item
    }
}

impl SectionItems<CurveItem> {
    pub fn get_or_create(
        &mut self,
        mnemonic: &str,
        default: Option<CurveItem>,
        add: bool,
    ) -> CurveItem {
        if let Some(item) = self.get(mnemonic) {
            return item.clone();
        }

        let mut item = default.unwrap_or_else(|| {
            let template_len = self
                .items
                .first()
                .map(|curve| curve.data.len())
                .unwrap_or(0);
            CurveItem::new(
                mnemonic,
                "",
                LasValue::Empty,
                "",
                vec![LasValue::Number(f64::NAN); template_len],
            )
        });
        item.rename(mnemonic);
        if add {
            self.push(item.clone());
        }
        item
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDescriptor {
    pub curve_id: String,
    pub raw_mnemonic: String,
    pub unit: String,
    pub kind: IndexKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LasFileSummary {
    pub source_path: String,
    pub original_filename: String,
    pub source_fingerprint: String,
    pub las_version: String,
    pub wrap_mode: String,
    pub delimiter: String,
    pub row_count: usize,
    pub curve_count: usize,
    pub issue_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LasFile {
    pub summary: LasFileSummary,
    pub provenance: Provenance,
    pub encoding: Option<String>,
    pub index: IndexDescriptor,
    pub version: SectionItems<HeaderItem>,
    pub well: SectionItems<HeaderItem>,
    pub params: SectionItems<HeaderItem>,
    pub curves: SectionItems<CurveItem>,
    pub other: String,
    pub extra_sections: BTreeMap<String, String>,
    pub issues: Vec<IngestIssue>,
    pub index_unit: Option<String>,
}

impl LasFile {
    pub fn keys(&self) -> Vec<String> {
        self.curves.keys()
    }

    pub fn curve_names(&self) -> Vec<String> {
        self.keys()
    }

    pub fn curve_data(&self, mnemonic: &str) -> Option<&[LasValue]> {
        self.curves.get(mnemonic).map(|curve| curve.data.as_slice())
    }

    pub fn curve_data_at(&self, index: isize) -> Option<&[LasValue]> {
        self.curves
            .get_index_signed(index)
            .map(|curve| curve.data.as_slice())
    }

    pub fn get_curve(&self, mnemonic: &str) -> Option<&CurveItem> {
        self.curves.get(mnemonic)
    }

    pub fn curve(&self, mnemonic: &str) -> crate::Result<&CurveItem> {
        self.get_curve(mnemonic)
            .ok_or_else(|| crate::LasError::Validation(format!("curve '{mnemonic}' not found")))
    }

    pub fn data_matrix(&self) -> Option<Vec<Vec<f64>>> {
        let columns = self
            .curves
            .iter()
            .map(CurveItem::numeric_data)
            .collect::<Option<Vec<_>>>()?;

        if columns.is_empty() {
            return Some(Vec::new());
        }

        let row_count = columns[0].len();
        let mut rows = Vec::with_capacity(row_count);
        for row_index in 0..row_count {
            let row = columns
                .iter()
                .map(|column| column[row_index])
                .collect::<Vec<_>>();
            rows.push(row);
        }
        Some(rows)
    }

    pub fn append_curve(&mut self, mnemonic: &str, data: Vec<LasValue>) {
        self.curves
            .push(CurveItem::new(mnemonic, "", LasValue::Empty, "", data));
    }

    pub fn append_curve_item(&mut self, curve: CurveItem) {
        self.curves.push(curve);
    }

    pub fn insert_curve(&mut self, index: usize, curve: CurveItem) {
        self.curves.insert(index, curve);
    }

    pub fn delete_curve_by_index(&mut self, index: usize) -> Option<CurveItem> {
        self.curves.delete_index(index)
    }

    pub fn delete_curve_by_mnemonic(&mut self, mnemonic: &str) -> Option<CurveItem> {
        self.curves.delete_mnemonic(mnemonic)
    }

    pub fn update_curve_data(&mut self, mnemonic: &str, data: Vec<LasValue>) -> bool {
        if let Some(curve) = self.curves.get_mut(mnemonic) {
            curve.data = data;
            true
        } else {
            false
        }
    }

    pub fn replace_curve_item(&mut self, mnemonic: &str, curve: CurveItem) {
        self.curves.set_item(mnemonic, curve);
    }

    pub fn stack_curves(
        &self,
        selector: CurveSelector,
        natural_sort: bool,
    ) -> Result<Vec<Vec<f64>>, String> {
        let mut names = match selector {
            CurveSelector::Prefix(prefix) => {
                let keys = self.keys();
                let matched = keys
                    .into_iter()
                    .filter(|name| name.starts_with(&prefix))
                    .collect::<Vec<_>>();
                if matched.is_empty() {
                    vec![prefix]
                } else {
                    matched
                }
            }
            CurveSelector::Names(names) => names,
        };

        if names.is_empty() || names.iter().any(|name| name.is_empty()) {
            return Err(String::from("curve selector must not be empty"));
        }

        if natural_sort {
            names.sort_by(|left, right| natural_cmp(left, right));
        }

        let mut columns = Vec::new();
        let mut missing = Vec::new();
        for name in &names {
            let Some(curve) = self.get_curve(name) else {
                missing.push(name.clone());
                continue;
            };
            let numeric = curve
                .numeric_data()
                .ok_or_else(|| format!("{name} contains non-numeric values."))?;
            columns.push(numeric);
        }

        if !missing.is_empty() {
            return Err(format!("{} not found in LAS curves.", missing.join(", ")));
        }

        if columns.is_empty() {
            return Ok(Vec::new());
        }

        let row_count = columns[0].len();
        if columns.iter().any(|column| column.len() != row_count) {
            return Err(String::from(
                "selected curves do not have a consistent row count",
            ));
        }

        let mut rows = Vec::with_capacity(row_count);
        for row_index in 0..row_count {
            rows.push(
                columns
                    .iter()
                    .map(|column| column[row_index])
                    .collect::<Vec<_>>(),
            );
        }

        Ok(rows)
    }

    pub fn depth_m(&self) -> Result<Vec<f64>, String> {
        let index = self
            .curves
            .get_index(0)
            .ok_or_else(|| String::from("no index curve"))?;
        let values = index
            .numeric_data()
            .ok_or_else(|| String::from("index curve is not numeric"))?;
        match normalized_depth_unit(self.index_unit.as_deref()) {
            Some("M") => Ok(values),
            Some("FT") => Ok(values.into_iter().map(|value| value * 0.3048).collect()),
            Some(".1IN") => Ok(values
                .into_iter()
                .map(|value| (value / 120.0) * 0.3048)
                .collect()),
            _ => Err(String::from("unit of depth index not known")),
        }
    }

    pub fn depth_ft(&self) -> Result<Vec<f64>, String> {
        let index = self
            .curves
            .get_index(0)
            .ok_or_else(|| String::from("no index curve"))?;
        let values = index
            .numeric_data()
            .ok_or_else(|| String::from("index curve is not numeric"))?;
        match normalized_depth_unit(self.index_unit.as_deref()) {
            Some("M") => Ok(values.into_iter().map(|value| value / 0.3048).collect()),
            Some("FT") => Ok(values),
            Some(".1IN") => Ok(values.into_iter().map(|value| value / 120.0).collect()),
            _ => Err(String::from("unit of depth index not known")),
        }
    }

    pub fn row_count(&self) -> usize {
        self.curves
            .get_index(0)
            .map(|curve| curve.data.len())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub enum CurveSelector {
    Prefix(String),
    Names(Vec<String>),
}

pub fn useful_mnemonic(original: &str) -> String {
    let trimmed = original.trim();
    if trimmed.is_empty() {
        String::from("UNKNOWN")
    } else {
        trimmed.to_string()
    }
}

pub fn mnemonic_match(case: MnemonicCase, left: &str, right: &str) -> bool {
    match case {
        MnemonicCase::Preserve => left == right,
        MnemonicCase::Upper | MnemonicCase::Lower => left.eq_ignore_ascii_case(right),
    }
}

pub fn natural_sort_key(input: &str) -> Vec<String> {
    let mut key = Vec::new();
    let mut current = String::new();
    let mut current_is_digit = None;

    for ch in input.chars() {
        let is_digit = ch.is_ascii_digit();
        match current_is_digit {
            Some(value) if value == is_digit => current.push(ch),
            Some(_) => {
                key.push(current.clone());
                current.clear();
                current.push(ch);
                current_is_digit = Some(is_digit);
            }
            None => {
                current.push(ch);
                current_is_digit = Some(is_digit);
            }
        }
    }

    if !current.is_empty() {
        key.push(current);
    }

    key
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let left_chunks = natural_sort_key(left);
    let right_chunks = natural_sort_key(right);

    for (left_chunk, right_chunk) in left_chunks.iter().zip(right_chunks.iter()) {
        let left_is_digit = left_chunk.chars().all(|ch| ch.is_ascii_digit());
        let right_is_digit = right_chunk.chars().all(|ch| ch.is_ascii_digit());
        let ordering = match (left_is_digit, right_is_digit) {
            (true, true) => compare_numeric_text(left_chunk, right_chunk),
            _ => left_chunk.cmp(right_chunk),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    left_chunks.len().cmp(&right_chunks.len())
}

fn compare_numeric_text(left: &str, right: &str) -> Ordering {
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');
    let left_normalized = if left_trimmed.is_empty() {
        "0"
    } else {
        left_trimmed
    };
    let right_normalized = if right_trimmed.is_empty() {
        "0"
    } else {
        right_trimmed
    };

    left_normalized
        .len()
        .cmp(&right_normalized.len())
        .then_with(|| left_normalized.cmp(right_normalized))
        .then_with(|| left.len().cmp(&right.len()))
}

pub fn normalized_depth_unit(unit: Option<&str>) -> Option<&'static str> {
    let Some(unit) = unit.map(|value| value.trim().to_ascii_uppercase()) else {
        return None;
    };

    match unit.as_str() {
        "FT" | "F" | "FEET" | "FOOT" => Some("FT"),
        "M" | "METER" | "METERS" | "METRE" | "METRES" | "Ð¼ÐµÑ‚ÐµÑ€" | "Ð¼" => {
            Some("M")
        }
        ".1IN" | "0.1IN" | ".1INCH" | "0.1INCH" => Some(".1IN"),
        _ => None,
    }
}

pub fn derive_index_kind(mnemonic: &str) -> IndexKind {
    match mnemonic.trim().to_ascii_uppercase().as_str() {
        "DEPT" | "DEPTH" => IndexKind::Depth,
        "TIME" | "ETIM" => IndexKind::Time,
        _ => IndexKind::Unknown,
    }
}

#[allow(dead_code)]
pub fn derive_canonical_alias(raw_mnemonic: &str, unit: &str) -> CanonicalAlias {
    let mnemonic = match raw_mnemonic.trim().to_ascii_uppercase().as_str() {
        "DEPT" | "DEPTH" => Some(String::from("depth")),
        "TIME" | "ETIM" => Some(String::from("time")),
        "GR" | "GAMN" | "GRC" | "GRAX" | "GR1AX" => Some(String::from("gamma_ray")),
        "RHOB" | "RHO" | "RHOZ" | "DEN" | "DENS" | "BDCX" => Some(String::from("bulk_density")),
        "NPHI" | "NPLX" | "CNL" => Some(String::from("neutron_porosity")),
        "DT" | "DTC" | "DTCO" | "AC" => Some(String::from("sonic")),
        "DTS" | "DTSM" | "DTSH" => Some(String::from("shear_sonic")),
        "VP" | "PVEL" | "P_VEL" => Some(String::from("p_velocity")),
        "VS" | "SVEL" | "S_VEL" => Some(String::from("s_velocity")),
        "SP" | "SPBL" => Some(String::from("spontaneous_potential")),
        "ILD" | "RESD" => Some(String::from("deep_resistivity")),
        "ILM" | "RESM" => Some(String::from("medium_resistivity")),
        "SFLA" | "RESS" | "MSFL" | "RX0" | "RXO" => Some(String::from("shallow_resistivity")),
        _ => None,
    };

    let unit_hint = if unit.trim().is_empty() {
        None
    } else {
        Some(unit.trim().to_ascii_lowercase())
    };

    CanonicalAlias {
        mnemonic,
        unit_hint,
    }
}

pub fn bundle_manifest_path(root: &Path) -> PathBuf {
    root.join("bundle.json")
}
