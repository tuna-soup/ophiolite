use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ophiolite_seismic::{
    CoordinateReferenceDescriptor, ImportedHorizonDescriptor, SectionAxis, SectionHorizonLineStyle,
    SectionHorizonOverlayView, SectionHorizonSample, SectionHorizonStyle, SurveyGridTransform,
};
use proj::{Proj, ProjBuilder};
use serde::{Deserialize, Serialize};

use crate::error::SeismicStoreError;
use crate::store::open_store;

const HORIZONS_DIR: &str = "horizons";
const HORIZON_MANIFEST_FILE: &str = "manifest.json";
const HORIZON_STORE_VERSION: u32 = 2;
const GRID_SNAP_TOLERANCE: f64 = 0.6;
const PROJ_RESOURCE_PATH_ENV: &str = "OPHIOLITE_PROJ_RESOURCE_PATH";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HorizonStoreManifest {
    version: u32,
    horizons: Vec<StoredHorizonManifest>,
}

impl Default for HorizonStoreManifest {
    fn default() -> Self {
        Self {
            version: HORIZON_STORE_VERSION,
            horizons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredHorizonManifest {
    id: String,
    name: String,
    source_path: String,
    point_count: usize,
    mapped_point_count: usize,
    missing_cell_count: usize,
    imported_at_unix_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    aligned_coordinate_reference: Option<CoordinateReferenceDescriptor>,
    #[serde(default)]
    transformed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<String>,
    style: SectionHorizonStyle,
    values_file: String,
    validity_file: String,
}

#[derive(Debug)]
struct HorizonGridImport {
    point_count: usize,
    mapped_point_count: usize,
    missing_cell_count: usize,
    values: Vec<f32>,
    validity: Vec<u8>,
}

#[derive(Debug, Clone)]
struct HorizonImportCoordinateReferences {
    source: CoordinateReferenceDescriptor,
    aligned: CoordinateReferenceDescriptor,
    transformed: bool,
    notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportedHorizonGrid {
    pub descriptor: ImportedHorizonDescriptor,
    pub inline_count: usize,
    pub xline_count: usize,
    pub values: Vec<f32>,
    pub validity: Vec<u8>,
}

pub fn import_horizon_xyzs<P: AsRef<Path>>(
    root: impl AsRef<Path>,
    input_paths: &[P],
    source_coordinate_reference_id: Option<&str>,
    source_coordinate_reference_name: Option<&str>,
    assume_same_as_survey: bool,
) -> Result<Vec<ImportedHorizonDescriptor>, SeismicStoreError> {
    if input_paths.is_empty() {
        return Ok(Vec::new());
    }

    let root = root.as_ref();
    let handle = open_store(root)?;
    let shape = handle.manifest.volume.shape;
    let inline_count = shape[0];
    let xline_count = shape[1];
    let transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.as_ref())
        .ok_or_else(|| {
            SeismicStoreError::Message(String::from(
                "horizon import requires a store with a resolved survey grid transform",
            ))
        })?;
    let coordinate_references = resolve_horizon_import_coordinate_references(
        handle.manifest.volume.coordinate_reference_binding.as_ref(),
        source_coordinate_reference_id,
        source_coordinate_reference_name,
        assume_same_as_survey,
    )?;

    let horizons_root = root.join(HORIZONS_DIR);
    fs::create_dir_all(&horizons_root)?;
    let mut manifest = load_horizon_manifest(&horizons_root)?;
    let occupied_ids = manifest
        .horizons
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<HashSet<_>>();
    let mut imported = Vec::with_capacity(input_paths.len());
    let mut batch_ids = HashSet::<String>::new();

    for input_path in input_paths {
        let input_path = input_path.as_ref();
        let stem = input_path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("horizon");
        let base_id = sanitize_horizon_id(stem);
        let id = unique_horizon_id(&base_id, &occupied_ids, &batch_ids);
        batch_ids.insert(id.clone());

        let style = manifest
            .horizons
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.style.clone())
            .unwrap_or_else(|| default_horizon_style(manifest.horizons.len() + imported.len()));
        let import = import_xyz_grid(
            input_path,
            transform,
            inline_count,
            xline_count,
            &coordinate_references,
        )?;

        let values_file = format!("{id}.values.f32le.bin");
        let validity_file = format!("{id}.validity.u8.bin");
        fs::write(
            horizons_root.join(&values_file),
            f32_slice_to_le_bytes(&import.values),
        )?;
        fs::write(horizons_root.join(&validity_file), &import.validity)?;

        let descriptor = ImportedHorizonDescriptor {
            id: id.clone(),
            name: stem.trim().to_string(),
            source_path: input_path.to_string_lossy().into_owned(),
            point_count: import.point_count,
            mapped_point_count: import.mapped_point_count,
            missing_cell_count: import.missing_cell_count,
            source_coordinate_reference: Some(coordinate_references.source.clone()),
            aligned_coordinate_reference: Some(coordinate_references.aligned.clone()),
            transformed: coordinate_references.transformed,
            notes: coordinate_references.notes.clone(),
            style: style.clone(),
        };

        manifest.horizons.retain(|entry| entry.id != id);
        manifest.horizons.push(StoredHorizonManifest {
            id: descriptor.id.clone(),
            name: descriptor.name.clone(),
            source_path: descriptor.source_path.clone(),
            point_count: descriptor.point_count,
            mapped_point_count: descriptor.mapped_point_count,
            missing_cell_count: descriptor.missing_cell_count,
            imported_at_unix_s: unix_timestamp_now(),
            source_coordinate_reference: descriptor.source_coordinate_reference.clone(),
            aligned_coordinate_reference: descriptor.aligned_coordinate_reference.clone(),
            transformed: descriptor.transformed,
            notes: descriptor.notes.clone(),
            style,
            values_file,
            validity_file,
        });
        imported.push(descriptor);
    }

    manifest.horizons.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    save_horizon_manifest(&horizons_root, &manifest)?;
    Ok(imported)
}

pub fn section_horizon_overlays(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
) -> Result<Vec<SectionHorizonOverlayView>, SeismicStoreError> {
    let root = root.as_ref();
    let grids = load_horizon_grids(root)?;
    if grids.is_empty() {
        return Ok(Vec::new());
    }
    let handle = open_store(root)?;
    let sample_axis = &handle.manifest.volume.axes.sample_axis_ms;
    let mut overlays = Vec::with_capacity(grids.len());
    for horizon in grids {
        let samples = match axis {
            SectionAxis::Inline => build_inline_samples(
                index,
                horizon.inline_count,
                horizon.xline_count,
                sample_axis,
                &horizon.values,
                &horizon.validity,
            )?,
            SectionAxis::Xline => build_xline_samples(
                index,
                horizon.inline_count,
                horizon.xline_count,
                sample_axis,
                &horizon.values,
                &horizon.validity,
            )?,
        };
        overlays.push(SectionHorizonOverlayView {
            id: horizon.descriptor.id,
            name: Some(horizon.descriptor.name),
            style: horizon.descriptor.style,
            samples,
        });
    }
    Ok(overlays)
}

pub fn load_horizon_grids(
    root: impl AsRef<Path>,
) -> Result<Vec<ImportedHorizonGrid>, SeismicStoreError> {
    let root = root.as_ref();
    let handle = open_store(root)?;
    let shape = handle.manifest.volume.shape;
    let horizons_root = root.join(HORIZONS_DIR);
    if !horizons_root.exists() {
        return Ok(Vec::new());
    }

    let manifest = load_horizon_manifest(&horizons_root)?;
    if manifest.horizons.is_empty() {
        return Ok(Vec::new());
    }

    let expected_cells = shape[0] * shape[1];
    manifest
        .horizons
        .into_iter()
        .map(|horizon| {
            let values = read_f32le_file(&horizons_root.join(&horizon.values_file))?;
            let validity = fs::read(horizons_root.join(&horizon.validity_file))?;
            if values.len() != expected_cells {
                return Err(SeismicStoreError::Message(format!(
                    "horizon values for {} do not match the store grid shape",
                    horizon.name
                )));
            }
            if validity.len() != expected_cells {
                return Err(SeismicStoreError::Message(format!(
                    "horizon validity mask for {} does not match the store grid shape",
                    horizon.name
                )));
            }

            Ok(ImportedHorizonGrid {
                descriptor: imported_horizon_descriptor_from_manifest(&horizon),
                inline_count: shape[0],
                xline_count: shape[1],
                values,
                validity,
            })
        })
        .collect()
}

fn build_inline_samples(
    inline_index: usize,
    inline_count: usize,
    xline_count: usize,
    sample_axis: &[f32],
    values: &[f32],
    validity: &[u8],
) -> Result<Vec<SectionHorizonSample>, SeismicStoreError> {
    if inline_index >= inline_count {
        return Err(SeismicStoreError::InvalidSectionIndex {
            index: inline_index,
            len: inline_count,
        });
    }

    let mut samples = Vec::with_capacity(xline_count);
    for xline_index in 0..xline_count {
        let offset = inline_index * xline_count + xline_index;
        if validity[offset] == 0 {
            continue;
        }
        let sample_value = values[offset];
        let Some(sample_index) = sample_index_for_value(sample_axis, sample_value) else {
            continue;
        };
        samples.push(SectionHorizonSample {
            trace_index: xline_index,
            sample_index,
            sample_value: Some(sample_value),
        });
    }
    Ok(samples)
}

fn build_xline_samples(
    xline_index: usize,
    inline_count: usize,
    xline_count: usize,
    sample_axis: &[f32],
    values: &[f32],
    validity: &[u8],
) -> Result<Vec<SectionHorizonSample>, SeismicStoreError> {
    if xline_index >= xline_count {
        return Err(SeismicStoreError::InvalidSectionIndex {
            index: xline_index,
            len: xline_count,
        });
    }

    let mut samples = Vec::with_capacity(inline_count);
    for inline_index in 0..inline_count {
        let offset = inline_index * xline_count + xline_index;
        if validity[offset] == 0 {
            continue;
        }
        let sample_value = values[offset];
        let Some(sample_index) = sample_index_for_value(sample_axis, sample_value) else {
            continue;
        };
        samples.push(SectionHorizonSample {
            trace_index: inline_index,
            sample_index,
            sample_value: Some(sample_value),
        });
    }
    Ok(samples)
}

fn sample_index_for_value(sample_axis: &[f32], sample_value: f32) -> Option<usize> {
    match sample_axis {
        [] => None,
        [first] => ((sample_value - *first).abs() <= 1e-3).then_some(0),
        [first, second, ..] => {
            let step = *second - *first;
            if step.abs() <= f32::EPSILON {
                return None;
            }
            let lower_bound = *first - step.abs() * 0.5;
            let upper_bound = sample_axis[sample_axis.len() - 1] + step.abs() * 0.5;
            if sample_value < lower_bound || sample_value > upper_bound {
                return None;
            }
            let sample_index = ((sample_value - *first) / step).round() as isize;
            if sample_index < 0 || sample_index as usize >= sample_axis.len() {
                return None;
            }
            Some(sample_index as usize)
        }
    }
}

fn import_xyz_grid(
    input_path: &Path,
    transform: &SurveyGridTransform,
    inline_count: usize,
    xline_count: usize,
    coordinate_references: &HorizonImportCoordinateReferences,
) -> Result<HorizonGridImport, SeismicStoreError> {
    let total_cells = inline_count * xline_count;
    let file = fs::File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut values = vec![0.0_f32; total_cells];
    let mut validity = vec![0_u8; total_cells];
    let mut point_count = 0_usize;
    let mut mapped_point_count = 0_usize;
    let transformer = coordinate_references
        .transformed
        .then(|| {
            build_proj_transformer(
                coordinate_references
                    .source
                    .id
                    .as_deref()
                    .expect("source CRS id should be resolved"),
                coordinate_references
                    .aligned
                    .id
                    .as_deref()
                    .expect("aligned CRS id should be resolved"),
            )
        })
        .transpose()?;

    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let fields = trimmed
            .split(|character: char| {
                character.is_ascii_whitespace() || character == ',' || character == ';'
            })
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if fields.len() < 3 {
            return Err(SeismicStoreError::Message(format!(
                "invalid horizon xyz row {} in {}",
                line_index + 1,
                input_path.display()
            )));
        }

        let x = fields[0].parse::<f64>().map_err(|error| {
            SeismicStoreError::Message(format!(
                "invalid x coordinate at row {} in {}: {error}",
                line_index + 1,
                input_path.display()
            ))
        })?;
        let y = fields[1].parse::<f64>().map_err(|error| {
            SeismicStoreError::Message(format!(
                "invalid y coordinate at row {} in {}: {error}",
                line_index + 1,
                input_path.display()
            ))
        })?;
        let z = fields[2].parse::<f32>().map_err(|error| {
            SeismicStoreError::Message(format!(
                "invalid z coordinate at row {} in {}: {error}",
                line_index + 1,
                input_path.display()
            ))
        })?;
        point_count += 1;

        let (aligned_x, aligned_y) = if let Some(transformer) = transformer.as_ref() {
            transform_projected_coordinate(transformer, x, y)?
        } else {
            (x, y)
        };

        let Some((inline_index, xline_index)) = snap_projected_point_to_grid(
            transform,
            aligned_x,
            aligned_y,
            inline_count,
            xline_count,
        ) else {
            continue;
        };
        let offset = inline_index * xline_count + xline_index;
        if validity[offset] == 0 {
            mapped_point_count += 1;
        }
        values[offset] = z;
        validity[offset] = 1;
    }

    if point_count == 0 {
        return Err(SeismicStoreError::Message(format!(
            "no horizon xyz rows were parsed from {}",
            input_path.display()
        )));
    }
    if mapped_point_count == 0 {
        return Err(SeismicStoreError::Message(format!(
            "none of the horizon xyz rows in {} matched the survey grid",
            input_path.display()
        )));
    }
    if mapped_point_count.saturating_mul(10) < point_count.saturating_mul(9) {
        return Err(SeismicStoreError::Message(format!(
            "only {mapped_point_count} of {point_count} horizon xyz rows in {} matched the survey grid",
            input_path.display()
        )));
    }

    Ok(HorizonGridImport {
        point_count,
        mapped_point_count,
        missing_cell_count: total_cells.saturating_sub(mapped_point_count),
        values,
        validity,
    })
}

fn resolve_horizon_import_coordinate_references(
    binding: Option<&ophiolite_seismic::CoordinateReferenceBinding>,
    source_coordinate_reference_id: Option<&str>,
    source_coordinate_reference_name: Option<&str>,
    assume_same_as_survey: bool,
) -> Result<HorizonImportCoordinateReferences, SeismicStoreError> {
    let aligned = binding
        .and_then(|binding| binding.effective.clone())
        .filter(|reference| {
            reference
                .id
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
        })
        .ok_or_else(|| {
            SeismicStoreError::Message(String::from(
                "horizon import requires the active survey store to have an effective native CRS before imported horizons can be aligned",
            ))
        })?;
    let aligned_id = aligned
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .expect("aligned CRS id should be present");

    if !is_supported_epsg_identifier(aligned_id) {
        return Err(SeismicStoreError::Message(format!(
            "survey effective native CRS '{aligned_id}' is not yet supported for horizon import reprojection; this path currently accepts only EPSG identifiers",
        )));
    }

    let normalized_source_id = source_coordinate_reference_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let normalized_source_name = source_coordinate_reference_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let (source, notes) = match normalized_source_id {
        Some(source_id) => {
            if !is_supported_epsg_identifier(&source_id) {
                return Err(SeismicStoreError::Message(format!(
                    "horizon source CRS '{source_id}' is not yet supported for reprojection; this path currently accepts only EPSG identifiers",
                )));
            }
            let mut notes = Vec::new();
            notes.push(format!(
                "horizon source CRS resolved as {source_id} before alignment into survey CRS {aligned_id}"
            ));
            (
                CoordinateReferenceDescriptor {
                    id: Some(source_id),
                    name: normalized_source_name,
                    geodetic_datum: None,
                    unit: None,
                },
                notes,
            )
        }
        None if assume_same_as_survey => {
            let mut notes = Vec::new();
            notes.push(format!(
                "horizon source CRS was explicitly assumed to match the survey effective native CRS {aligned_id}"
            ));
            (aligned.clone(), notes)
        }
        None => {
            return Err(SeismicStoreError::Message(String::from(
                "horizon import requires either a source CRS identifier or an explicit same-as-survey assumption",
            )));
        }
    };

    let transformed = source
        .id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.eq_ignore_ascii_case(aligned_id));
    let mut notes = notes;
    if transformed {
        notes.push(format!(
            "horizon XYZ coordinates will be reprojected from {} into survey CRS {aligned_id} before grid alignment",
            source.id.as_deref().unwrap_or_default()
        ));
    } else {
        notes.push(format!(
            "horizon XYZ coordinates are already expressed in survey CRS {aligned_id}"
        ));
    }

    Ok(HorizonImportCoordinateReferences {
        source,
        aligned,
        transformed,
        notes,
    })
}

fn is_supported_epsg_identifier(value: &str) -> bool {
    value.trim().to_ascii_uppercase().starts_with("EPSG:")
}

fn build_proj_transformer(
    source_coordinate_reference_id: &str,
    target_coordinate_reference_id: &str,
) -> Result<Proj, SeismicStoreError> {
    let mut builder = ProjBuilder::new();
    if let Ok(resource_path) = std::env::var(PROJ_RESOURCE_PATH_ENV) {
        let resource_path = resource_path.trim();
        if !resource_path.is_empty() {
            builder.set_search_paths(resource_path).map_err(|error| {
                SeismicStoreError::Message(format!("failed to set PROJ search path: {error}"))
            })?;
        }
    }

    builder
        .proj_known_crs(
            source_coordinate_reference_id,
            target_coordinate_reference_id,
            None,
        )
        .map_err(|error| {
            SeismicStoreError::Message(format!("failed to build PROJ transformer: {error}"))
        })
}

fn transform_projected_coordinate(
    transformer: &Proj,
    x: f64,
    y: f64,
) -> Result<(f64, f64), SeismicStoreError> {
    transformer.convert((x, y)).map_err(|error| {
        SeismicStoreError::Message(format!("PROJ coordinate transform failed: {error}"))
    })
}

fn snap_projected_point_to_grid(
    transform: &SurveyGridTransform,
    x: f64,
    y: f64,
    inline_count: usize,
    xline_count: usize,
) -> Option<(usize, usize)> {
    let determinant = transform.inline_basis.x * transform.xline_basis.y
        - transform.inline_basis.y * transform.xline_basis.x;
    if determinant.abs() <= f64::EPSILON {
        return None;
    }

    let dx = x - transform.origin.x;
    let dy = y - transform.origin.y;
    let inline_index = (dx * transform.xline_basis.y - dy * transform.xline_basis.x) / determinant;
    let xline_index = (dy * transform.inline_basis.x - dx * transform.inline_basis.y) / determinant;
    let inline_snapped = inline_index.round();
    let xline_snapped = xline_index.round();
    if (inline_index - inline_snapped).abs() > GRID_SNAP_TOLERANCE
        || (xline_index - xline_snapped).abs() > GRID_SNAP_TOLERANCE
    {
        return None;
    }
    if inline_snapped < 0.0
        || inline_snapped >= inline_count as f64
        || xline_snapped < 0.0
        || xline_snapped >= xline_count as f64
    {
        return None;
    }

    Some((inline_snapped as usize, xline_snapped as usize))
}

fn default_horizon_style(slot: usize) -> SectionHorizonStyle {
    const COLORS: [&str; 7] = [
        "#ff4d4f", "#78dce8", "#f7b267", "#9b8cff", "#7bd389", "#ffd166", "#ff85a1",
    ];
    const LINE_STYLES: [SectionHorizonLineStyle; 3] = [
        SectionHorizonLineStyle::Solid,
        SectionHorizonLineStyle::Dashed,
        SectionHorizonLineStyle::Dotted,
    ];

    SectionHorizonStyle {
        color: COLORS[slot % COLORS.len()].to_string(),
        line_width: Some(if slot % LINE_STYLES.len() == 0 {
            3.5
        } else {
            2.5
        }),
        line_style: LINE_STYLES[slot % LINE_STYLES.len()],
        opacity: Some(0.95),
    }
}

fn load_horizon_manifest(horizons_root: &Path) -> Result<HorizonStoreManifest, SeismicStoreError> {
    let manifest_path = horizons_root.join(HORIZON_MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(HorizonStoreManifest::default());
    }
    Ok(serde_json::from_slice(&fs::read(manifest_path)?)?)
}

fn imported_horizon_descriptor_from_manifest(
    manifest: &StoredHorizonManifest,
) -> ImportedHorizonDescriptor {
    ImportedHorizonDescriptor {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        source_path: manifest.source_path.clone(),
        point_count: manifest.point_count,
        mapped_point_count: manifest.mapped_point_count,
        missing_cell_count: manifest.missing_cell_count,
        source_coordinate_reference: manifest.source_coordinate_reference.clone(),
        aligned_coordinate_reference: manifest.aligned_coordinate_reference.clone(),
        transformed: manifest.transformed,
        notes: manifest.notes.clone(),
        style: manifest.style.clone(),
    }
}

fn save_horizon_manifest(
    horizons_root: &Path,
    manifest: &HorizonStoreManifest,
) -> Result<(), SeismicStoreError> {
    fs::write(
        horizons_root.join(HORIZON_MANIFEST_FILE),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn sanitize_horizon_id(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
        } else if character == '-' || character == '_' {
            sanitized.push(character);
        } else if !sanitized.ends_with('-') {
            sanitized.push('-');
        }
    }
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        String::from("horizon")
    } else {
        trimmed.to_string()
    }
}

fn unique_horizon_id(base: &str, occupied: &HashSet<String>, reserved: &HashSet<String>) -> String {
    if !reserved.contains(base) {
        return base.to_string();
    }

    let mut suffix = 2_usize;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !reserved.contains(&candidate) && !occupied.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn f32_slice_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn read_f32le_file(path: &Path) -> Result<Vec<f32>, SeismicStoreError> {
    let bytes = fs::read(path)?;
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "expected a little-endian f32 file at {}",
            path.display()
        )));
    }
    Ok(bytes
        .chunks_exact(std::mem::size_of::<f32>())
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn unix_timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ndarray::Array3;
    use tempfile::tempdir;

    use crate::metadata::{
        DatasetKind, HeaderFieldSpec, SourceIdentity, VolumeAxes, VolumeMetadata,
    };
    use crate::storage::tbvol::TbvolManifest;
    use crate::store::create_tbvol_store;
    use crate::{
        CoordinateReferenceBinding, CoordinateReferenceDescriptor, CoordinateReferenceSource,
        ProjectedPoint2, ProjectedVector2, SurveySpatialAvailability, SurveySpatialDescriptor,
    };

    use super::*;

    #[test]
    fn imports_xyz_horizon_and_slices_inline_section() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 6,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: crate::metadata::GeometryProvenance {
                        inline_field: HeaderFieldSpec {
                            name: String::from("INLINE_3D"),
                            start_byte: 189,
                            value_type: String::from("I32"),
                        },
                        crossline_field: HeaderFieldSpec {
                            name: String::from("CROSSLINE_3D"),
                            start_byte: 193,
                            value_type: String::from("I32"),
                        },
                        third_axis_field: None,
                    },
                    regularization: None,
                },
                shape: [2, 3, 4],
                axes: VolumeAxes {
                    ilines: vec![100.0, 101.0],
                    xlines: vec![200.0, 201.0, 202.0],
                    sample_axis_ms: vec![0.0, 10.0, 20.0, 30.0],
                },
                segy_export: None,
                coordinate_reference_binding: Some(CoordinateReferenceBinding {
                    detected: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:32631")),
                        name: Some(String::from("WGS 84 / UTM zone 31N")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    effective: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:32631")),
                        name: Some(String::from("WGS 84 / UTM zone 31N")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    source: CoordinateReferenceSource::Header,
                    notes: Vec::new(),
                }),
                spatial: Some(SurveySpatialDescriptor {
                    coordinate_reference: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:32631")),
                        name: Some(String::from("WGS 84 / UTM zone 31N")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    grid_transform: Some(SurveyGridTransform {
                        origin: ProjectedPoint2 {
                            x: 1_000.0,
                            y: 2_000.0,
                        },
                        inline_basis: ProjectedVector2 { x: 10.0, y: 0.0 },
                        xline_basis: ProjectedVector2 { x: 0.0, y: 20.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
            },
            [2, 3, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 3, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let xyz_path = temp.path().join("h1.xyz");
        fs::write(
            &xyz_path,
            ["1010 2000 10", "1010 2020 20", "1010 2040 30"].join("\n"),
        )
        .expect("write xyz");

        let imported = import_horizon_xyzs(&store_root, &[&xyz_path], None, None, true)
            .expect("import horizons");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].mapped_point_count, 3);
        assert!(imported[0].source_coordinate_reference.is_some());
        assert!(!imported[0].transformed);

        let overlays =
            section_horizon_overlays(&store_root, SectionAxis::Inline, 1).expect("slice overlays");
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].samples.len(), 3);
        assert_eq!(overlays[0].samples[0].trace_index, 0);
        assert_eq!(overlays[0].samples[0].sample_index, 1);
        assert_eq!(overlays[0].samples[2].trace_index, 2);
        assert_eq!(overlays[0].samples[2].sample_index, 3);
    }

    #[test]
    fn imports_xyz_horizon_after_reprojecting_into_survey_crs() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo-reproject.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo-reproject"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo-reproject.segy"),
                    file_size: 0,
                    trace_count: 6,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: crate::metadata::GeometryProvenance {
                        inline_field: HeaderFieldSpec {
                            name: String::from("INLINE_3D"),
                            start_byte: 189,
                            value_type: String::from("I32"),
                        },
                        crossline_field: HeaderFieldSpec {
                            name: String::from("CROSSLINE_3D"),
                            start_byte: 193,
                            value_type: String::from("I32"),
                        },
                        third_axis_field: None,
                    },
                    regularization: None,
                },
                shape: [2, 3, 4],
                axes: VolumeAxes {
                    ilines: vec![100.0, 101.0],
                    xlines: vec![200.0, 201.0, 202.0],
                    sample_axis_ms: vec![0.0, 10.0, 20.0, 30.0],
                },
                segy_export: None,
                coordinate_reference_binding: Some(CoordinateReferenceBinding {
                    detected: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:3857")),
                        name: Some(String::from("WGS 84 / Pseudo-Mercator")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    effective: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:3857")),
                        name: Some(String::from("WGS 84 / Pseudo-Mercator")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    source: CoordinateReferenceSource::Header,
                    notes: Vec::new(),
                }),
                spatial: Some(SurveySpatialDescriptor {
                    coordinate_reference: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("EPSG:3857")),
                        name: Some(String::from("WGS 84 / Pseudo-Mercator")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    grid_transform: Some(SurveyGridTransform {
                        origin: ProjectedPoint2 {
                            x: 1_113_194.907_932_735_7,
                            y: 6_446_275.841_017_161,
                        },
                        inline_basis: ProjectedVector2 { x: 100.0, y: 0.0 },
                        xline_basis: ProjectedVector2 { x: 0.0, y: 100.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
            },
            [2, 3, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 3, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let inverse_transformer =
            build_proj_transformer("EPSG:3857", "EPSG:4326").expect("inverse transformer");
        let xyz_rows = [
            (1_113_294.907_932_735_7, 6_446_275.841_017_161, 10.0_f32),
            (1_113_294.907_932_735_7, 6_446_375.841_017_161, 20.0_f32),
            (1_113_294.907_932_735_7, 6_446_475.841_017_161, 30.0_f32),
        ]
        .iter()
        .map(|(x, y, z)| {
            let (lon, lat) = transform_projected_coordinate(&inverse_transformer, *x, *y)
                .expect("inverse point");
            format!("{lon:.9} {lat:.9} {z}")
        })
        .collect::<Vec<_>>();
        let xyz_path = temp.path().join("h1-wgs84.xyz");
        fs::write(&xyz_path, xyz_rows.join("\n")).expect("write xyz");

        let imported = import_horizon_xyzs(
            &store_root,
            &[&xyz_path],
            Some("EPSG:4326"),
            Some("WGS 84"),
            false,
        )
        .expect("import horizons");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].mapped_point_count, 3);
        assert!(imported[0].transformed);
        assert_eq!(
            imported[0]
                .aligned_coordinate_reference
                .as_ref()
                .and_then(|reference| reference.id.as_deref()),
            Some("EPSG:3857")
        );

        let overlays =
            section_horizon_overlays(&store_root, SectionAxis::Inline, 1).expect("slice overlays");
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].samples.len(), 3);
        assert_eq!(overlays[0].samples[0].sample_index, 1);
        assert_eq!(overlays[0].samples[2].sample_index, 3);
    }
}
