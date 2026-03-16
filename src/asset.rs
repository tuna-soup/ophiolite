use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalAlias {
    pub mnemonic: Option<String>,
    pub unit_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderItem {
    pub mnemonic: String,
    pub unit: String,
    pub value: String,
    pub description: String,
    pub line_number: usize,
    pub raw_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderSection {
    pub name: String,
    pub title_line: String,
    pub raw_body: String,
    pub items: Vec<HeaderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveDescriptor {
    pub id: String,
    pub raw_mnemonic: String,
    pub unit: String,
    pub value: String,
    pub description: String,
    pub canonical_alias: CanonicalAlias,
    pub sample_count: usize,
    pub is_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curve {
    pub descriptor: CurveDescriptor,
    pub samples: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexKind {
    Depth,
    Time,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDescriptor {
    pub curve_id: String,
    pub raw_mnemonic: String,
    pub unit: String,
    pub kind: IndexKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LasAssetSummary {
    pub asset_id: String,
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
pub struct LasAsset {
    pub summary: LasAssetSummary,
    pub provenance: Provenance,
    pub index: IndexDescriptor,
    pub headers: Vec<HeaderSection>,
    pub curves: Vec<Curve>,
    pub issues: Vec<IngestIssue>,
}

#[derive(Debug, Clone, Copy)]
pub struct CurveWindow {
    pub start: usize,
    pub end: usize,
}

impl CurveWindow {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl LasAsset {
    pub fn list_curves(&self) -> Vec<CurveDescriptor> {
        self.curves
            .iter()
            .map(|curve| curve.descriptor.clone())
            .collect()
    }

    pub fn get_curve_metadata(&self, curve_id: &str) -> Option<CurveDescriptor> {
        self.curves
            .iter()
            .find(|curve| curve.descriptor.id == curve_id)
            .map(|curve| curve.descriptor.clone())
    }

    pub fn read_curve(&self, curve_id: &str, window: Option<CurveWindow>) -> Option<Vec<f64>> {
        self.curves
            .iter()
            .find(|curve| curve.descriptor.id == curve_id)
            .map(|curve| slice_samples(&curve.samples, window))
    }

    pub fn read_curves(
        &self,
        curve_ids: &[String],
        window: Option<CurveWindow>,
    ) -> Vec<(String, Vec<f64>)> {
        curve_ids
            .iter()
            .filter_map(|curve_id| {
                self.read_curve(curve_id, window)
                    .map(|samples| (curve_id.clone(), samples))
            })
            .collect()
    }

    pub fn read_index(&self, window: Option<CurveWindow>) -> Vec<f64> {
        self.read_curve(&self.index.curve_id, window)
            .unwrap_or_default()
    }

    pub fn get_ingest_issues(&self) -> &[IngestIssue] {
        &self.issues
    }
}

pub fn slice_samples(samples: &[f64], window: Option<CurveWindow>) -> Vec<f64> {
    let Some(window) = window else {
        return samples.to_vec();
    };

    if window.start >= window.end || window.start >= samples.len() {
        return Vec::new();
    }

    let end = window.end.min(samples.len());
    samples[window.start..end].to_vec()
}

pub fn derive_asset_id(source_fingerprint: &str) -> String {
    let prefix: String = source_fingerprint.chars().take(16).collect();
    format!("las-{prefix}")
}

pub fn derive_index_kind(mnemonic: &str) -> IndexKind {
    match mnemonic.trim().to_ascii_uppercase().as_str() {
        "DEPT" | "DEPTH" => IndexKind::Depth,
        "TIME" | "ETIM" => IndexKind::Time,
        _ => IndexKind::Unknown,
    }
}

pub fn derive_canonical_alias(raw_mnemonic: &str, unit: &str) -> CanonicalAlias {
    let mnemonic = match raw_mnemonic.trim().to_ascii_uppercase().as_str() {
        "DEPT" | "DEPTH" => Some(String::from("depth")),
        "TIME" | "ETIM" => Some(String::from("time")),
        "GR" | "GAMN" | "GRC" => Some(String::from("gamma_ray")),
        "RHOB" => Some(String::from("bulk_density")),
        "NPHI" => Some(String::from("neutron_porosity")),
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
