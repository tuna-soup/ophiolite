use crate::asset::{
    CurveDescriptor, CurveWindow, HeaderSection, IngestIssue, LasAsset, LasAssetSummary,
    Provenance, bundle_manifest_path, slice_samples,
};
use crate::{LasError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const BUNDLE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleManifest {
    bundle_version: u32,
    summary: LasAssetSummary,
    provenance: Provenance,
    index_curve_id: String,
    headers: Vec<HeaderSection>,
    curves: Vec<StoredCurve>,
    issues: Vec<IngestIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCurve {
    descriptor: CurveDescriptor,
    binary_path: String,
}

#[derive(Debug, Clone)]
pub struct StoredLasAsset {
    root: PathBuf,
    manifest: BundleManifest,
}

impl StoredLasAsset {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let manifest_path = bundle_manifest_path(&root);
        let manifest_text = fs::read_to_string(&manifest_path)?;
        let manifest: BundleManifest = serde_json::from_str(&manifest_text)?;
        if manifest.bundle_version != BUNDLE_VERSION {
            return Err(LasError::Storage(format!(
                "Unsupported bundle version {}.",
                manifest.bundle_version
            )));
        }
        Ok(Self { root, manifest })
    }

    pub fn summary(&self) -> &LasAssetSummary {
        &self.manifest.summary
    }

    pub fn list_curves(&self) -> Vec<CurveDescriptor> {
        self.manifest
            .curves
            .iter()
            .map(|curve| curve.descriptor.clone())
            .collect()
    }

    pub fn get_curve_metadata(&self, curve_id: &str) -> Option<CurveDescriptor> {
        self.manifest
            .curves
            .iter()
            .find(|curve| curve.descriptor.id == curve_id)
            .map(|curve| curve.descriptor.clone())
    }

    pub fn read_curve(&self, curve_id: &str, window: Option<CurveWindow>) -> Result<Vec<f64>> {
        let curve = self
            .manifest
            .curves
            .iter()
            .find(|curve| curve.descriptor.id == curve_id)
            .ok_or_else(|| {
                LasError::Storage(format!("Curve '{curve_id}' was not found in the bundle."))
            })?;

        let samples = read_curve_binary(self.root.join(&curve.binary_path))?;
        Ok(slice_samples(&samples, window))
    }

    pub fn read_curves(
        &self,
        curve_ids: &[String],
        window: Option<CurveWindow>,
    ) -> Result<Vec<(String, Vec<f64>)>> {
        let mut results = Vec::with_capacity(curve_ids.len());
        for curve_id in curve_ids {
            results.push((curve_id.clone(), self.read_curve(curve_id, window)?));
        }
        Ok(results)
    }

    pub fn read_index(&self, window: Option<CurveWindow>) -> Result<Vec<f64>> {
        self.read_curve(&self.manifest.index_curve_id, window)
    }

    pub fn get_ingest_issues(&self) -> &[IngestIssue] {
        &self.manifest.issues
    }
}

pub fn write_bundle(asset: &LasAsset, output_dir: impl AsRef<Path>) -> Result<StoredLasAsset> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() {
        return Err(LasError::Storage(format!(
            "Output directory '{}' already exists.",
            output_dir.display()
        )));
    }

    fs::create_dir_all(output_dir)?;
    let curves_dir = output_dir.join("curves");
    fs::create_dir_all(&curves_dir)?;

    let mut stored_curves = Vec::with_capacity(asset.curves.len());
    for curve in &asset.curves {
        let binary_name = format!("{}.bin", sanitize_curve_id(&curve.descriptor.id));
        let binary_path = Path::new("curves").join(&binary_name);
        write_curve_binary(output_dir.join(&binary_path), &curve.samples)?;

        let mut descriptor = curve.descriptor.clone();
        descriptor.sample_count = curve.samples.len();
        stored_curves.push(StoredCurve {
            descriptor,
            binary_path: binary_path.to_string_lossy().into_owned(),
        });
    }

    let manifest = BundleManifest {
        bundle_version: BUNDLE_VERSION,
        summary: asset.summary.clone(),
        provenance: asset.provenance.clone(),
        index_curve_id: asset.index.curve_id.clone(),
        headers: asset.headers.clone(),
        curves: stored_curves,
        issues: asset.issues.clone(),
    };

    let manifest_path = bundle_manifest_path(output_dir);
    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_text)?;

    StoredLasAsset::open(output_dir)
}

fn sanitize_curve_id(curve_id: &str) -> String {
    curve_id
        .chars()
        .map(|ch| match ch {
            ':' | '/' | '\\' | ' ' => '_',
            other => other,
        })
        .collect()
}

fn write_curve_binary(path: PathBuf, samples: &[f64]) -> Result<()> {
    let mut file = fs::File::create(path)?;
    for sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }
    Ok(())
}

fn read_curve_binary(path: PathBuf) -> Result<Vec<f64>> {
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    if bytes.len() % 8 != 0 {
        return Err(LasError::Storage(String::from(
            "Curve binary payload length is not divisible by 8.",
        )));
    }

    let mut samples = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(chunk);
        samples.push(f64::from_le_bytes(buffer));
    }
    Ok(samples)
}
