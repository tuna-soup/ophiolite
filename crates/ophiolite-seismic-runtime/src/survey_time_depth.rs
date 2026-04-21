use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ophiolite_seismic::{
    BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest,
    CoordinateReferenceDescriptor, DepthReferenceKind, LateralInterpolationMethod,
    LayeredVelocityInterval, SectionAxis, SpatialCoverageRelationship, SpatialCoverageSummary,
    StratigraphicBoundaryReference, SurveyGridTransform, SurveyPropertyField3D,
    SurveyTimeDepthTransform3D, TimeDepthDomain, TimeDepthTransformSourceKind, TravelTimeReference,
    VelocityControlProfile, VelocityControlProfileSet, VelocityIntervalTrend, VelocityQuantityKind,
    VerticalAxisDescriptor, VerticalInterpolationMethod,
};
use serde::{Deserialize, Serialize};

use crate::error::SeismicStoreError;
use crate::horizons::{ImportedHorizonGrid, load_horizon_grids};
use crate::store::open_store;

const TRANSFORMS_DIR: &str = "time-depth-transforms";
const PROPERTY_FIELDS_DIR: &str = "property-fields";
const TRANSFORM_MANIFEST_FILE: &str = "manifest.json";
const PROPERTY_FIELD_MANIFEST_FILE: &str = "manifest.json";
const TRANSFORM_STORE_VERSION: u32 = 1;
const PROPERTY_FIELD_STORE_VERSION: u32 = 1;
const AXIS_TOLERANCE: f32 = 1.0e-3;
const MILLIS_TO_SECONDS: f32 = 0.001;
const MIN_SUPPORTED_VELOCITY_M_PER_S: f32 = 1.0;

#[derive(Debug, Clone)]
struct PreparedControlProfile {
    inline_index: usize,
    xline_index: usize,
    curve_m_per_s: Vec<f32>,
}

#[derive(Debug, Clone)]
struct CompiledControlCurveSource {
    inline_index: usize,
    xline_index: usize,
    curve_m_per_s: Vec<f32>,
}

#[derive(Debug, Clone)]
struct CompiledVelocityFieldPayload {
    descriptor: SurveyPropertyField3D,
    values_f32: Vec<f32>,
    validity: Vec<u8>,
    curves_m_per_s: Vec<Vec<f32>>,
    preferred_velocity_kind: VelocityQuantityKind,
}

#[derive(Debug, Clone)]
struct ResolvedIntervalBoundaryGrid {
    top_times_ms: Vec<f32>,
    base_times_ms: Vec<f32>,
    validity: Vec<u8>,
    top_label: String,
    base_label: String,
}

#[derive(Debug, Clone)]
struct CompiledIntervalVelocityField {
    interval: LayeredVelocityInterval,
    boundary_grid: ResolvedIntervalBoundaryGrid,
    curves_m_per_s: Vec<Vec<f32>>,
    compilation_note: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct GridSearchState {
    distance_milli: u32,
    inline_index: usize,
    xline_index: usize,
    source_index: usize,
}

impl Ord for GridSearchState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .distance_milli
            .cmp(&self.distance_milli)
            .then_with(|| other.inline_index.cmp(&self.inline_index))
            .then_with(|| other.xline_index.cmp(&self.xline_index))
            .then_with(|| other.source_index.cmp(&self.source_index))
    }
}

impl PartialOrd for GridSearchState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SurveyTimeDepthTransformStoreManifest {
    version: u32,
    transforms: Vec<StoredSurveyTimeDepthTransformManifest>,
}

impl Default for SurveyTimeDepthTransformStoreManifest {
    fn default() -> Self {
        Self {
            version: TRANSFORM_STORE_VERSION,
            transforms: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SurveyPropertyFieldStoreManifest {
    version: u32,
    fields: Vec<StoredSurveyPropertyFieldManifest>,
}

impl Default for SurveyPropertyFieldStoreManifest {
    fn default() -> Self {
        Self {
            version: PROPERTY_FIELD_STORE_VERSION,
            fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSurveyTimeDepthTransformManifest {
    descriptor: SurveyTimeDepthTransform3D,
    stored_at_unix_s: u64,
    depths_file: String,
    validity_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSurveyPropertyFieldManifest {
    descriptor: SurveyPropertyField3D,
    stored_at_unix_s: u64,
    values_file: String,
    validity_file: String,
}

#[derive(Debug, Clone)]
pub struct StoredSurveyTimeDepthTransform {
    pub descriptor: SurveyTimeDepthTransform3D,
    pub depths_m: Vec<f32>,
    pub validity: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StoredSurveyPropertyField {
    pub descriptor: SurveyPropertyField3D,
    pub values_f32: Vec<f32>,
    pub validity: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SectionSurveyTimeDepthTransformSlice {
    pub descriptor: SurveyTimeDepthTransform3D,
    pub trace_depths_m: Vec<Vec<f32>>,
    pub trace_validity: Vec<bool>,
    pub coverage_relationship: SpatialCoverageRelationship,
}

pub fn store_survey_time_depth_transform(
    root: impl AsRef<Path>,
    descriptor: SurveyTimeDepthTransform3D,
    depths_m: &[f32],
    validity: &[u8],
) -> Result<SurveyTimeDepthTransform3D, SeismicStoreError> {
    let root = root.as_ref();
    let handle = open_store(root)?;
    let shape = handle.manifest.volume.shape;
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;

    if descriptor.time_axis.domain != TimeDepthDomain::Time {
        return Err(SeismicStoreError::Message(
            "survey time-depth transforms must use a time-domain source axis".to_string(),
        ));
    }
    if descriptor.inline_count != shape[0]
        || descriptor.xline_count != shape[1]
        || descriptor.sample_count != shape[2]
        || descriptor.time_axis.count != shape[2]
    {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' does not match store grid shape {:?}",
            descriptor.id, shape
        )));
    }
    validate_time_axis_matches_store(&descriptor, sample_axis_ms)?;

    let expected_cells = descriptor.inline_count * descriptor.xline_count * descriptor.sample_count;
    if depths_m.len() != expected_cells {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' depth sample count mismatch: expected {expected_cells}, found {}",
            descriptor.id,
            depths_m.len()
        )));
    }
    if validity.len() != expected_cells {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' validity sample count mismatch: expected {expected_cells}, found {}",
            descriptor.id,
            validity.len()
        )));
    }

    let survey_coordinate_reference = handle
        .manifest
        .volume
        .coordinate_reference_binding
        .as_ref()
        .and_then(|binding| binding.effective.clone());
    let survey_grid_transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.clone());
    validate_transform_alignment(
        descriptor.coordinate_reference.as_ref(),
        survey_coordinate_reference.as_ref(),
        descriptor.grid_transform.as_ref(),
        survey_grid_transform.as_ref(),
    )?;

    validate_depth_payload(&descriptor, depths_m, validity)?;

    let coverage_relationship = derive_coverage_relationship(validity);
    let mut descriptor = descriptor;
    descriptor.coordinate_reference = survey_coordinate_reference;
    descriptor.grid_transform = survey_grid_transform;
    descriptor.coverage = SpatialCoverageSummary {
        relationship: coverage_relationship,
        source_coordinate_reference: descriptor.coverage.source_coordinate_reference.clone(),
        target_coordinate_reference: descriptor.coverage.target_coordinate_reference.clone(),
        notes: descriptor.coverage.notes.clone(),
    };

    let transforms_root = root.join(TRANSFORMS_DIR);
    fs::create_dir_all(&transforms_root)?;
    let mut manifest = load_transform_manifest(&transforms_root)?;
    let occupied_ids = manifest
        .transforms
        .iter()
        .map(|entry| entry.descriptor.id.clone())
        .collect::<HashSet<_>>();
    let descriptor_id = if descriptor.id.trim().is_empty() {
        unique_transform_id("time-depth-transform", &occupied_ids)
    } else if occupied_ids.contains(&descriptor.id) {
        descriptor.id.clone()
    } else {
        descriptor.id.clone()
    };
    descriptor.id = descriptor_id.clone();

    let depths_file = format!("{descriptor_id}.depths.f32le.bin");
    let validity_file = format!("{descriptor_id}.validity.u8.bin");
    fs::write(
        transforms_root.join(&depths_file),
        f32_slice_to_le_bytes(depths_m),
    )?;
    fs::write(transforms_root.join(&validity_file), validity)?;

    manifest
        .transforms
        .retain(|entry| entry.descriptor.id != descriptor_id);
    manifest
        .transforms
        .push(StoredSurveyTimeDepthTransformManifest {
            descriptor: descriptor.clone(),
            stored_at_unix_s: unix_timestamp_now(),
            depths_file,
            validity_file,
        });
    manifest.transforms.sort_by(|left, right| {
        left.descriptor
            .name
            .cmp(&right.descriptor.name)
            .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
    });
    save_transform_manifest(&transforms_root, &manifest)?;

    Ok(descriptor)
}

pub fn load_survey_time_depth_transforms(
    root: impl AsRef<Path>,
) -> Result<Vec<StoredSurveyTimeDepthTransform>, SeismicStoreError> {
    let root = root.as_ref();
    let transforms_root = root.join(TRANSFORMS_DIR);
    if !transforms_root.exists() {
        return Ok(Vec::new());
    }

    let manifest = load_transform_manifest(&transforms_root)?;
    manifest
        .transforms
        .into_iter()
        .map(|entry| {
            let expected_cells = entry.descriptor.inline_count
                * entry.descriptor.xline_count
                * entry.descriptor.sample_count;
            let depths_m = read_f32le_file(&transforms_root.join(&entry.depths_file))?;
            let validity = fs::read(transforms_root.join(&entry.validity_file))?;
            if depths_m.len() != expected_cells || validity.len() != expected_cells {
                return Err(SeismicStoreError::Message(format!(
                    "survey time-depth transform '{}' payload does not match its declared grid dimensions",
                    entry.descriptor.id
                )));
            }

            Ok(StoredSurveyTimeDepthTransform {
                descriptor: entry.descriptor,
                depths_m,
                validity,
            })
        })
        .collect()
}

pub fn load_survey_time_depth_transform(
    root: impl AsRef<Path>,
    asset_id: &str,
) -> Result<StoredSurveyTimeDepthTransform, SeismicStoreError> {
    load_survey_time_depth_transforms(root)?
        .into_iter()
        .find(|transform| transform.descriptor.id == asset_id)
        .ok_or_else(|| {
            SeismicStoreError::Message(format!(
                "survey time-depth transform asset '{}' was not found in the store",
                asset_id
            ))
        })
}

pub fn store_survey_property_field(
    root: impl AsRef<Path>,
    descriptor: SurveyPropertyField3D,
    values_f32: &[f32],
    validity: &[u8],
) -> Result<SurveyPropertyField3D, SeismicStoreError> {
    let root = root.as_ref();
    let handle = open_store(root)?;
    let shape = handle.manifest.volume.shape;
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;

    if descriptor.vertical_axis.domain != TimeDepthDomain::Time {
        return Err(SeismicStoreError::Message(
            "survey property fields currently require a time-domain vertical axis".to_string(),
        ));
    }
    if descriptor.inline_count != shape[0]
        || descriptor.xline_count != shape[1]
        || descriptor.sample_count != shape[2]
        || descriptor.vertical_axis.count != shape[2]
    {
        return Err(SeismicStoreError::Message(format!(
            "survey property field '{}' does not match store grid shape {:?}",
            descriptor.id, shape
        )));
    }
    validate_property_field_axis_matches_store(&descriptor, sample_axis_ms)?;

    let expected_cells = descriptor.inline_count * descriptor.xline_count * descriptor.sample_count;
    if values_f32.len() != expected_cells || validity.len() != expected_cells {
        return Err(SeismicStoreError::Message(format!(
            "survey property field '{}' payload does not match its declared grid dimensions",
            descriptor.id
        )));
    }

    let survey_coordinate_reference = handle
        .manifest
        .volume
        .coordinate_reference_binding
        .as_ref()
        .and_then(|binding| binding.effective.clone());
    let survey_grid_transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.clone());
    validate_property_field_alignment(
        descriptor.coordinate_reference.as_ref(),
        survey_coordinate_reference.as_ref(),
        descriptor.grid_transform.as_ref(),
        survey_grid_transform.as_ref(),
    )?;

    let coverage_relationship = derive_coverage_relationship(validity);
    let mut descriptor = descriptor;
    descriptor.coordinate_reference = survey_coordinate_reference;
    descriptor.grid_transform = survey_grid_transform;
    descriptor.coverage = SpatialCoverageSummary {
        relationship: coverage_relationship,
        source_coordinate_reference: descriptor.coverage.source_coordinate_reference.clone(),
        target_coordinate_reference: descriptor.coverage.target_coordinate_reference.clone(),
        notes: descriptor.coverage.notes.clone(),
    };

    let fields_root = root.join(PROPERTY_FIELDS_DIR);
    fs::create_dir_all(&fields_root)?;
    let values_file = format!("{}.values_f32.le", descriptor.id);
    let validity_file = format!("{}.validity_u8.bin", descriptor.id);
    write_f32le_file(fields_root.join(&values_file), values_f32)?;
    fs::write(fields_root.join(&validity_file), validity)?;

    let mut manifest = load_property_field_manifest(&fields_root)?;
    manifest
        .fields
        .retain(|entry| entry.descriptor.id != descriptor.id);
    manifest.fields.push(StoredSurveyPropertyFieldManifest {
        descriptor: descriptor.clone(),
        stored_at_unix_s: unix_timestamp_now(),
        values_file,
        validity_file,
    });
    manifest.fields.sort_by(|left, right| {
        left.descriptor
            .name
            .cmp(&right.descriptor.name)
            .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
    });
    save_property_field_manifest(&fields_root, &manifest)?;

    Ok(descriptor)
}

pub fn load_survey_property_fields(
    root: impl AsRef<Path>,
) -> Result<Vec<StoredSurveyPropertyField>, SeismicStoreError> {
    let root = root.as_ref();
    let fields_root = root.join(PROPERTY_FIELDS_DIR);
    if !fields_root.exists() {
        return Ok(Vec::new());
    }

    let manifest = load_property_field_manifest(&fields_root)?;
    manifest
        .fields
        .into_iter()
        .map(|entry| {
            let expected_cells = entry.descriptor.inline_count
                * entry.descriptor.xline_count
                * entry.descriptor.sample_count;
            let values_f32 = read_f32le_file(&fields_root.join(&entry.values_file))?;
            let validity = fs::read(fields_root.join(&entry.validity_file))?;
            if values_f32.len() != expected_cells || validity.len() != expected_cells {
                return Err(SeismicStoreError::Message(format!(
                    "survey property field '{}' payload does not match its declared grid dimensions",
                    entry.descriptor.id
                )));
            }

            Ok(StoredSurveyPropertyField {
                descriptor: entry.descriptor,
                values_f32,
                validity,
            })
        })
        .collect()
}

pub fn load_survey_property_field(
    root: impl AsRef<Path>,
    asset_id: &str,
) -> Result<StoredSurveyPropertyField, SeismicStoreError> {
    load_survey_property_fields(root)?
        .into_iter()
        .find(|field| field.descriptor.id == asset_id)
        .ok_or_else(|| {
            SeismicStoreError::Message(format!(
                "survey property field '{}' was not found",
                asset_id
            ))
        })
}

pub fn section_time_depth_transform_slice(
    root: impl AsRef<Path>,
    asset_id: &str,
    axis: SectionAxis,
    index: usize,
) -> Result<SectionSurveyTimeDepthTransformSlice, SeismicStoreError> {
    let transform = load_survey_time_depth_transform(root, asset_id)?;
    let descriptor = transform.descriptor.clone();
    let sample_count = descriptor.sample_count;

    let (trace_count, trace_depths_m, trace_validity) = match axis {
        SectionAxis::Inline => {
            if index >= descriptor.inline_count {
                return Err(SeismicStoreError::InvalidSectionIndex {
                    index,
                    len: descriptor.inline_count,
                });
            }
            let mut trace_depths_m = Vec::with_capacity(descriptor.xline_count);
            let mut trace_validity = Vec::with_capacity(descriptor.xline_count);
            for xline_index in 0..descriptor.xline_count {
                let mut trace = Vec::with_capacity(sample_count);
                let mut valid = true;
                for sample_index in 0..sample_count {
                    let offset = ((index * descriptor.xline_count + xline_index) * sample_count)
                        + sample_index;
                    if transform.validity[offset] == 0 {
                        valid = false;
                    }
                    trace.push(transform.depths_m[offset]);
                }
                trace_depths_m.push(trace);
                trace_validity.push(valid);
            }
            (descriptor.xline_count, trace_depths_m, trace_validity)
        }
        SectionAxis::Xline => {
            if index >= descriptor.xline_count {
                return Err(SeismicStoreError::InvalidSectionIndex {
                    index,
                    len: descriptor.xline_count,
                });
            }
            let mut trace_depths_m = Vec::with_capacity(descriptor.inline_count);
            let mut trace_validity = Vec::with_capacity(descriptor.inline_count);
            for inline_index in 0..descriptor.inline_count {
                let mut trace = Vec::with_capacity(sample_count);
                let mut valid = true;
                for sample_index in 0..sample_count {
                    let offset = ((inline_index * descriptor.xline_count + index) * sample_count)
                        + sample_index;
                    if transform.validity[offset] == 0 {
                        valid = false;
                    }
                    trace.push(transform.depths_m[offset]);
                }
                trace_depths_m.push(trace);
                trace_validity.push(valid);
            }
            (descriptor.inline_count, trace_depths_m, trace_validity)
        }
    };

    if trace_depths_m.len() != trace_count || trace_validity.len() != trace_count {
        return Err(SeismicStoreError::Message(
            "survey time-depth slice internal size mismatch".to_string(),
        ));
    }

    let valid_trace_count = trace_validity.iter().filter(|valid| **valid).count();
    let coverage_relationship = if valid_trace_count == 0 {
        SpatialCoverageRelationship::Disjoint
    } else if valid_trace_count == trace_validity.len() {
        SpatialCoverageRelationship::Exact
    } else {
        SpatialCoverageRelationship::PartialOverlap
    };

    Ok(SectionSurveyTimeDepthTransformSlice {
        descriptor,
        trace_depths_m,
        trace_validity,
        coverage_relationship,
    })
}

pub fn build_survey_property_field(
    request: &BuildSurveyPropertyFieldRequest,
) -> Result<StoredSurveyPropertyField, SeismicStoreError> {
    let store_root = Path::new(&request.store_path);
    if request.output_vertical_domain != TimeDepthDomain::Time {
        return Err(SeismicStoreError::Message(
            "survey property field builder currently only supports time-domain output fields"
                .to_string(),
        ));
    }
    let payload = compile_velocity_field_payload(
        &request.store_path,
        &request.model,
        &request.control_profile_sets,
        Some(request.preferred_velocity_kind),
        false,
        &request.property_name,
        &request.property_unit,
        request.output_id.clone(),
        request.output_name.clone(),
        &request.notes,
    )?;
    let descriptor = store_survey_property_field(
        store_root,
        payload.descriptor,
        &payload.values_f32,
        &payload.validity,
    )?;
    Ok(StoredSurveyPropertyField {
        descriptor,
        values_f32: payload.values_f32,
        validity: payload.validity,
    })
}

pub fn build_survey_time_depth_transform(
    request: &BuildSurveyTimeDepthTransformRequest,
) -> Result<SurveyTimeDepthTransform3D, SeismicStoreError> {
    let store_root = Path::new(&request.store_path);
    let handle = open_store(store_root)?;
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;
    let inline_count = handle.manifest.volume.shape[0];
    let xline_count = handle.manifest.volume.shape[1];
    let sample_count = handle.manifest.volume.shape[2];

    if request.output_depth_unit.trim() != "m" {
        return Err(SeismicStoreError::Message(
            "survey time-depth transform builder currently only supports output_depth_unit = 'm'"
                .to_string(),
        ));
    }

    let payload = compile_velocity_field_payload(
        &request.store_path,
        &request.model,
        &request.control_profile_sets,
        request.preferred_velocity_kind,
        true,
        "velocity",
        "m/s",
        request.output_id.clone(),
        request.output_name.clone(),
        &request.notes,
    )?;
    let travel_time_reference = resolve_model_travel_time_reference(
        &request.control_profile_sets,
        &request.model,
        &resolve_supported_intervals_from_model(&request.model)?,
    )?;
    let (depths_m, validity) = compile_depth_payload_from_velocity_curves(
        &payload.curves_m_per_s,
        &payload.validity,
        sample_axis_ms,
        travel_time_reference,
        inline_count,
        xline_count,
        sample_count,
    )?;

    let descriptor = SurveyTimeDepthTransform3D {
        id: request
            .output_id
            .clone()
            .unwrap_or_else(|| request.model.id.clone()),
        name: request
            .output_name
            .clone()
            .unwrap_or_else(|| request.model.name.clone()),
        derived_from: derived_transform_sources(request),
        source_kind: time_depth_source_kind_for_model(&request.model),
        coordinate_reference: request.model.coordinate_reference.clone(),
        grid_transform: request.model.grid_transform.clone(),
        time_axis: VerticalAxisDescriptor {
            domain: TimeDepthDomain::Time,
            unit: "ms".to_string(),
            start: sample_axis_ms.first().copied().unwrap_or(0.0),
            step: if sample_axis_ms.len() >= 2 {
                sample_axis_ms[1] - sample_axis_ms[0]
            } else {
                0.0
            },
            count: sample_count,
        },
        depth_unit: request.output_depth_unit.clone(),
        inline_count,
        xline_count,
        sample_count,
        coverage: SpatialCoverageSummary {
            relationship: SpatialCoverageRelationship::Unknown,
            source_coordinate_reference: request
                .control_profile_sets
                .iter()
                .find_map(|set| set.coordinate_reference.clone()),
            target_coordinate_reference: handle
                .manifest
                .volume
                .coordinate_reference_binding
                .as_ref()
                .and_then(|binding| binding.effective.clone()),
            notes: Vec::new(),
        },
        notes: {
            let mut notes = payload.descriptor.notes.clone();
            notes.push(format!(
                "Built from layered velocity model '{}' with velocity kind {:?}.",
                request.model.id, payload.preferred_velocity_kind
            ));
            notes
        },
    };

    store_survey_time_depth_transform(store_root, descriptor, &depths_m, &validity)
}

pub fn build_survey_time_depth_transform_from_horizon_pairs(
    root: impl AsRef<Path>,
    time_horizon_ids: &[String],
    depth_horizon_ids: &[String],
    output_id: Option<String>,
    output_name: Option<String>,
    notes: &[String],
) -> Result<SurveyTimeDepthTransform3D, SeismicStoreError> {
    if time_horizon_ids.is_empty() || depth_horizon_ids.is_empty() {
        return Err(SeismicStoreError::Message(
            "paired-horizon transform builder requires at least one time horizon and one depth horizon"
                .to_string(),
        ));
    }
    if time_horizon_ids.len() != depth_horizon_ids.len() {
        return Err(SeismicStoreError::Message(format!(
            "paired-horizon transform builder requires matching time and depth horizon counts; found {} time horizons and {} depth horizons",
            time_horizon_ids.len(),
            depth_horizon_ids.len()
        )));
    }

    let root = root.as_ref();
    let handle = open_store(root)?;
    let shape = handle.manifest.volume.shape;
    let inline_count = shape[0];
    let xline_count = shape[1];
    let sample_count = shape[2];
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;
    let last_sample_time_ms = sample_axis_ms.last().copied().ok_or_else(|| {
        SeismicStoreError::Message(
            "paired-horizon transform builder requires a survey with a non-empty sample axis"
                .to_string(),
        )
    })?;
    let all_horizons = load_horizon_grids(root)?;
    let horizons_by_id = all_horizons
        .iter()
        .map(|grid| (grid.descriptor.id.as_str(), grid))
        .collect::<HashMap<_, _>>();

    let mut paired_horizons = Vec::with_capacity(time_horizon_ids.len());
    for (time_horizon_id, depth_horizon_id) in time_horizon_ids.iter().zip(depth_horizon_ids.iter())
    {
        let time_horizon = lookup_horizon_grid(&horizons_by_id, time_horizon_id)?;
        if time_horizon.descriptor.vertical_domain != TimeDepthDomain::Time
            || time_horizon.descriptor.vertical_unit != "ms"
        {
            return Err(SeismicStoreError::Message(format!(
                "time horizon '{}' must be stored in canonical time domain ms",
                time_horizon_id
            )));
        }

        let depth_horizon = lookup_horizon_grid(&horizons_by_id, depth_horizon_id)?;
        if depth_horizon.descriptor.vertical_domain != TimeDepthDomain::Depth
            || depth_horizon.descriptor.vertical_unit != "m"
        {
            return Err(SeismicStoreError::Message(format!(
                "depth horizon '{}' must be stored in canonical depth domain m",
                depth_horizon_id
            )));
        }

        if time_horizon.inline_count != inline_count
            || time_horizon.xline_count != xline_count
            || depth_horizon.inline_count != inline_count
            || depth_horizon.xline_count != xline_count
        {
            return Err(SeismicStoreError::Message(format!(
                "paired horizons '{}' and '{}' do not align with the survey grid {}x{}",
                time_horizon_id, depth_horizon_id, inline_count, xline_count
            )));
        }
        if time_horizon.values.len() != inline_count * xline_count
            || depth_horizon.values.len() != inline_count * xline_count
            || time_horizon.validity.len() != inline_count * xline_count
            || depth_horizon.validity.len() != inline_count * xline_count
        {
            return Err(SeismicStoreError::Message(format!(
                "paired horizons '{}' and '{}' contain unexpected payload sizes",
                time_horizon_id, depth_horizon_id
            )));
        }

        paired_horizons.push((time_horizon, depth_horizon));
    }

    let mut depths_m = Vec::with_capacity(inline_count * xline_count * sample_count);
    let mut validity = Vec::with_capacity(inline_count * xline_count * sample_count);
    let mut invalid_trace_count = 0_usize;

    for trace_index in 0..(inline_count * xline_count) {
        match paired_boundary_pairs_for_trace(trace_index, &paired_horizons, last_sample_time_ms)
            .and_then(|boundary_pairs| {
                compile_trace_depths_from_boundary_pairs(sample_axis_ms, &boundary_pairs)
            }) {
            Ok(trace_depths_m) => {
                depths_m.extend(trace_depths_m);
                validity.extend(std::iter::repeat_n(1_u8, sample_count));
            }
            Err(_) => {
                invalid_trace_count += 1;
                depths_m.extend(std::iter::repeat_n(0.0_f32, sample_count));
                validity.extend(std::iter::repeat_n(0_u8, sample_count));
            }
        }
    }

    let survey_coordinate_reference = handle
        .manifest
        .volume
        .coordinate_reference_binding
        .as_ref()
        .and_then(|binding| binding.effective.clone());
    let survey_grid_transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.clone());

    let descriptor = SurveyTimeDepthTransform3D {
        id: output_id.unwrap_or_else(|| "paired-horizon-survey-transform".to_string()),
        name: output_name.unwrap_or_else(|| "Paired Horizon Survey Transform".to_string()),
        derived_from: time_horizon_ids
            .iter()
            .chain(depth_horizon_ids.iter())
            .cloned()
            .collect(),
        source_kind: TimeDepthTransformSourceKind::HorizonLayerModel,
        coordinate_reference: survey_coordinate_reference.clone(),
        grid_transform: survey_grid_transform.clone(),
        time_axis: VerticalAxisDescriptor {
            domain: TimeDepthDomain::Time,
            unit: "ms".to_string(),
            start: sample_axis_ms.first().copied().unwrap_or(0.0),
            step: if sample_axis_ms.len() >= 2 {
                sample_axis_ms[1] - sample_axis_ms[0]
            } else {
                0.0
            },
            count: sample_count,
        },
        depth_unit: "m".to_string(),
        inline_count,
        xline_count,
        sample_count,
        coverage: SpatialCoverageSummary {
            relationship: if invalid_trace_count == 0 {
                SpatialCoverageRelationship::Exact
            } else if invalid_trace_count == inline_count * xline_count {
                SpatialCoverageRelationship::Unknown
            } else {
                SpatialCoverageRelationship::PartialOverlap
            },
            source_coordinate_reference: survey_coordinate_reference.clone(),
            target_coordinate_reference: survey_coordinate_reference,
            notes: {
                let mut coverage_notes = vec![format!(
                    "Built from {} paired canonical TWT/depth horizons.",
                    paired_horizons.len()
                )];
                if invalid_trace_count > 0 {
                    coverage_notes.push(format!(
                        "{invalid_trace_count} traces were marked invalid because the paired horizon boundaries were non-monotonic or incomplete."
                    ));
                }
                coverage_notes
            },
        },
        notes: {
            let mut descriptor_notes = notes.to_vec();
            descriptor_notes.push(
                "Per-trace time-depth mapping is piecewise linear between paired TWT/depth horizons."
                    .to_string(),
            );
            descriptor_notes.push(
                "The transform is anchored at survey top time 0 ms / depth 0 m and extrapolated to survey base from the deepest paired interval."
                    .to_string(),
            );
            descriptor_notes
        },
    };

    store_survey_time_depth_transform(root, descriptor, &depths_m, &validity)
}

fn compile_velocity_field_payload(
    store_path: &str,
    model: &ophiolite_seismic::LayeredVelocityModel,
    control_profile_sets: &[VelocityControlProfileSet],
    preferred_velocity_kind: Option<VelocityQuantityKind>,
    allow_rms: bool,
    property_name: &str,
    property_unit: &str,
    output_id: Option<String>,
    output_name: Option<String>,
    request_notes: &[String],
) -> Result<CompiledVelocityFieldPayload, SeismicStoreError> {
    let store_root = Path::new(store_path);
    let handle = open_store(store_root)?;
    let sample_axis_ms = &handle.manifest.volume.axes.sample_axis_ms;
    let inline_count = handle.manifest.volume.shape[0];
    let xline_count = handle.manifest.volume.shape[1];
    let sample_count = handle.manifest.volume.shape[2];

    if model.vertical_domain != TimeDepthDomain::Time {
        return Err(SeismicStoreError::Message(
            "survey property builders currently only support time-domain authored models"
                .to_string(),
        ));
    }
    validate_model_alignment_against_store(
        model.coordinate_reference.as_ref(),
        handle
            .manifest
            .volume
            .coordinate_reference_binding
            .as_ref()
            .and_then(|binding| binding.effective.as_ref()),
        model.grid_transform.as_ref(),
        handle
            .manifest
            .volume
            .spatial
            .as_ref()
            .and_then(|spatial| spatial.grid_transform.as_ref()),
    )?;

    let intervals = resolve_supported_intervals_from_model(model)?;
    let preferred_velocity_kind =
        resolve_model_preferred_velocity_kind(preferred_velocity_kind, &intervals)?;
    if !allow_rms && preferred_velocity_kind == VelocityQuantityKind::Rms {
        return Err(SeismicStoreError::Message(
            "survey property builders do not support RMS velocity yet; normalize to interval or average velocity first"
                .to_string(),
        ));
    }

    let travel_time_reference =
        resolve_model_travel_time_reference(control_profile_sets, model, &intervals)?;
    let compiled_intervals = compile_interval_stack(
        store_root,
        model,
        control_profile_sets,
        &handle,
        sample_axis_ms,
        inline_count,
        xline_count,
        &intervals,
        preferred_velocity_kind,
        travel_time_reference,
    )?;
    let (curves_m_per_s, trace_validity) =
        merge_compiled_interval_stack(&compiled_intervals, sample_axis_ms)?;

    let values_f32 = curves_m_per_s
        .iter()
        .flat_map(|curve| curve.iter().copied())
        .collect::<Vec<_>>();
    let validity = expand_trace_validity_to_samples(&trace_validity, sample_count);
    let descriptor = SurveyPropertyField3D {
        id: output_id.unwrap_or_else(|| model.id.clone()),
        name: output_name.unwrap_or_else(|| model.name.clone()),
        derived_from: derived_property_field_sources(model, control_profile_sets),
        property_name: property_name.to_string(),
        property_unit: property_unit.to_string(),
        coordinate_reference: model.coordinate_reference.clone(),
        grid_transform: model.grid_transform.clone(),
        vertical_axis: VerticalAxisDescriptor {
            domain: TimeDepthDomain::Time,
            unit: "ms".to_string(),
            start: sample_axis_ms.first().copied().unwrap_or(0.0),
            step: if sample_axis_ms.len() >= 2 {
                sample_axis_ms[1] - sample_axis_ms[0]
            } else {
                0.0
            },
            count: sample_count,
        },
        inline_count,
        xline_count,
        sample_count,
        coverage: SpatialCoverageSummary {
            relationship: SpatialCoverageRelationship::Unknown,
            source_coordinate_reference: control_profile_sets
                .iter()
                .find_map(|set| set.coordinate_reference.clone()),
            target_coordinate_reference: handle
                .manifest
                .volume
                .coordinate_reference_binding
                .as_ref()
                .and_then(|binding| binding.effective.clone()),
            notes: Vec::new(),
        },
        notes: {
            let mut notes = model.notes.clone();
            notes.extend(request_notes.iter().cloned());
            notes.extend(
                compiled_intervals
                    .iter()
                    .map(|compiled| compiled.compilation_note.clone()),
            );
            notes.push(format!(
                "Built property field '{}' from layered velocity model '{}' with velocity kind {:?}.",
                property_name, model.id, preferred_velocity_kind
            ));
            notes.push(format!(
                "Compiled {} stratigraphic interval(s) across the active survey grid.",
                compiled_intervals.len()
            ));
            if compiled_intervals.len() == 1
                && (!matches!(
                    compiled_intervals[0].interval.top_boundary,
                    StratigraphicBoundaryReference::SurveyTop
                ) || !matches!(
                    compiled_intervals[0].interval.base_boundary,
                    StratigraphicBoundaryReference::SurveyBase
                ))
            {
                notes.push(
                    "Single-interval horizon-bounded compilation currently extrapolates the interval-edge velocity above/below the modeled interval to preserve a full trace transform."
                        .to_string(),
                );
            }
            notes
        },
    };

    Ok(CompiledVelocityFieldPayload {
        descriptor,
        values_f32,
        validity,
        curves_m_per_s,
        preferred_velocity_kind,
    })
}

fn validate_model_alignment_against_store(
    model_crs: Option<&CoordinateReferenceDescriptor>,
    survey_crs: Option<&CoordinateReferenceDescriptor>,
    model_grid_transform: Option<&SurveyGridTransform>,
    survey_grid_transform: Option<&SurveyGridTransform>,
) -> Result<(), SeismicStoreError> {
    if let (Some(model_crs), Some(survey_crs)) = (model_crs, survey_crs)
        && model_crs != survey_crs
    {
        return Err(SeismicStoreError::Message(
            "layered velocity models must already be aligned into the active survey CRS"
                .to_string(),
        ));
    }
    if let (Some(model_grid_transform), Some(survey_grid_transform)) =
        (model_grid_transform, survey_grid_transform)
        && model_grid_transform != survey_grid_transform
    {
        return Err(SeismicStoreError::Message(
            "layered velocity models must already be aligned to the active survey grid transform"
                .to_string(),
        ));
    }
    Ok(())
}

fn resolve_supported_intervals_from_model(
    model: &ophiolite_seismic::LayeredVelocityModel,
) -> Result<Vec<LayeredVelocityInterval>, SeismicStoreError> {
    if model.intervals.is_empty() {
        return Err(SeismicStoreError::Message(
            "survey time-depth transform builder requires at least one interval".to_string(),
        ));
    }
    if !matches!(
        model
            .intervals
            .first()
            .map(|interval| &interval.top_boundary),
        Some(StratigraphicBoundaryReference::SurveyTop)
    ) {
        return Err(SeismicStoreError::Message(
            "stacked velocity-model compilation currently requires the first interval to start at survey_top"
                .to_string(),
        ));
    }
    if !matches!(
        model
            .intervals
            .last()
            .map(|interval| &interval.base_boundary),
        Some(StratigraphicBoundaryReference::SurveyBase)
    ) {
        return Err(SeismicStoreError::Message(
            "stacked velocity-model compilation currently requires the last interval to end at survey_base"
                .to_string(),
        ));
    }

    for interval in &model.intervals {
        if matches!(
            (&interval.top_boundary, &interval.base_boundary),
            (
                StratigraphicBoundaryReference::HorizonAsset { horizon_id: top_id },
                StratigraphicBoundaryReference::HorizonAsset { horizon_id: base_id }
            ) if top_id == base_id
        ) {
            return Err(SeismicStoreError::Message(
                "survey time-depth transform builder requires distinct top/base horizon boundaries"
                    .to_string(),
            ));
        }
        if let Some(method) = interval.lateral_interpolation
            && method != LateralInterpolationMethod::Nearest
        {
            return Err(SeismicStoreError::Message(format!(
                "survey time-depth transform builder currently only supports nearest lateral interpolation, found {method:?}"
            )));
        }
        if let Some(method) = interval.vertical_interpolation
            && !matches!(
                method,
                VerticalInterpolationMethod::Step | VerticalInterpolationMethod::Linear
            )
        {
            return Err(SeismicStoreError::Message(format!(
                "survey time-depth transform builder currently only supports step or linear vertical interpolation, found {method:?}"
            )));
        }
    }

    for pair in model.intervals.windows(2) {
        if pair[0].base_boundary != pair[1].top_boundary {
            return Err(SeismicStoreError::Message(format!(
                "stacked velocity-model compilation currently requires interval '{}' base boundary to equal interval '{}' top boundary",
                pair[0].name, pair[1].name
            )));
        }
    }

    Ok(model.intervals.clone())
}

fn resolve_interval_boundary_grid(
    store_root: &Path,
    interval: &LayeredVelocityInterval,
    sample_axis_ms: &[f32],
    inline_count: usize,
    xline_count: usize,
) -> Result<ResolvedIntervalBoundaryGrid, SeismicStoreError> {
    let horizons = if matches!(
        interval.top_boundary,
        StratigraphicBoundaryReference::HorizonAsset { .. }
    ) || matches!(
        interval.base_boundary,
        StratigraphicBoundaryReference::HorizonAsset { .. }
    ) {
        load_horizon_grids(store_root)?
    } else {
        Vec::new()
    };
    let top = resolve_boundary_reference_grid(
        &interval.top_boundary,
        sample_axis_ms,
        inline_count,
        xline_count,
        &horizons,
    )?;
    let base = resolve_boundary_reference_grid(
        &interval.base_boundary,
        sample_axis_ms,
        inline_count,
        xline_count,
        &horizons,
    )?;

    let mut validity = Vec::with_capacity(top.0.len());
    for ((top_time_ms, top_valid), (base_time_ms, base_valid)) in top
        .0
        .iter()
        .zip(top.1.iter())
        .zip(base.0.iter().zip(base.1.iter()))
    {
        let valid = *top_valid != 0
            && *base_valid != 0
            && top_time_ms.is_finite()
            && base_time_ms.is_finite()
            && *top_time_ms < *base_time_ms;
        validity.push(if valid { 1 } else { 0 });
    }

    Ok(ResolvedIntervalBoundaryGrid {
        top_times_ms: top.0,
        base_times_ms: base.0,
        validity,
        top_label: top.2,
        base_label: base.2,
    })
}

fn resolve_boundary_reference_grid(
    boundary: &StratigraphicBoundaryReference,
    sample_axis_ms: &[f32],
    inline_count: usize,
    xline_count: usize,
    horizons: &[ImportedHorizonGrid],
) -> Result<(Vec<f32>, Vec<u8>, String), SeismicStoreError> {
    let cell_count = inline_count * xline_count;
    match boundary {
        StratigraphicBoundaryReference::SurveyTop => Ok((
            vec![sample_axis_ms.first().copied().unwrap_or(0.0); cell_count],
            vec![1_u8; cell_count],
            "survey_top".to_string(),
        )),
        StratigraphicBoundaryReference::SurveyBase => Ok((
            vec![sample_axis_ms.last().copied().unwrap_or(0.0); cell_count],
            vec![1_u8; cell_count],
            "survey_base".to_string(),
        )),
        StratigraphicBoundaryReference::HorizonAsset { horizon_id } => {
            let horizon = horizons
                .iter()
                .find(|grid| grid.descriptor.id == *horizon_id)
                .ok_or_else(|| {
                    SeismicStoreError::Message(format!(
                        "layered velocity model references missing imported horizon '{}'",
                        horizon_id
                    ))
                })?;
            if horizon.inline_count != inline_count || horizon.xline_count != xline_count {
                return Err(SeismicStoreError::Message(format!(
                    "imported horizon '{}' does not match the active survey grid shape",
                    horizon_id
                )));
            }
            let min_time_ms = sample_axis_ms.first().copied().unwrap_or(0.0);
            let max_time_ms = sample_axis_ms.last().copied().unwrap_or(0.0);
            let mut validity = horizon.validity.clone();
            for ((valid, value_ms), horizon_valid) in validity
                .iter_mut()
                .zip(horizon.values.iter())
                .zip(horizon.validity.iter())
            {
                if *horizon_valid == 0
                    || !value_ms.is_finite()
                    || *value_ms < min_time_ms
                    || *value_ms > max_time_ms
                {
                    *valid = 0;
                }
            }
            Ok((
                horizon.values.clone(),
                validity,
                format!("horizon:{}", horizon.descriptor.name),
            ))
        }
    }
}

fn resolve_preferred_velocity_kind(
    request_kind: Option<VelocityQuantityKind>,
    interval: &LayeredVelocityInterval,
) -> Result<VelocityQuantityKind, SeismicStoreError> {
    request_kind
        .or(interval.control_profile_velocity_kind)
        .ok_or_else(|| {
            SeismicStoreError::Message(
                "survey time-depth transform builder requires a preferred velocity kind"
                    .to_string(),
            )
        })
}

fn resolve_model_preferred_velocity_kind(
    request_kind: Option<VelocityQuantityKind>,
    intervals: &[LayeredVelocityInterval],
) -> Result<VelocityQuantityKind, SeismicStoreError> {
    let mut resolved_kind = None;
    for interval in intervals {
        let interval_kind = resolve_preferred_velocity_kind(request_kind, interval)?;
        if let Some(existing_kind) = resolved_kind
            && existing_kind != interval_kind
        {
            return Err(SeismicStoreError::Message(
                "stacked velocity-model compilation currently requires one consistent preferred velocity kind across all intervals"
                    .to_string(),
            ));
        }
        resolved_kind = Some(interval_kind);
    }
    resolved_kind.ok_or_else(|| {
        SeismicStoreError::Message(
            "survey time-depth transform builder requires a preferred velocity kind".to_string(),
        )
    })
}

fn resolve_travel_time_reference(
    control_profile_sets: &[VelocityControlProfileSet],
    model: &ophiolite_seismic::LayeredVelocityModel,
    interval: &LayeredVelocityInterval,
) -> Result<TravelTimeReference, SeismicStoreError> {
    if let Some(control_profile_set_id) = interval.control_profile_set_id.as_deref() {
        let set = find_control_profile_set(control_profile_sets, control_profile_set_id)?;
        if let Some(model_reference) = model.travel_time_reference
            && model_reference != set.travel_time_reference
        {
            return Err(SeismicStoreError::Message(format!(
                "layered velocity model travel_time_reference {:?} does not match control-profile set '{}' reference {:?}",
                model_reference, set.id, set.travel_time_reference
            )));
        }
        return Ok(set.travel_time_reference);
    }
    Ok(model
        .travel_time_reference
        .unwrap_or(TravelTimeReference::TwoWay))
}

fn resolve_model_travel_time_reference(
    control_profile_sets: &[VelocityControlProfileSet],
    model: &ophiolite_seismic::LayeredVelocityModel,
    intervals: &[LayeredVelocityInterval],
) -> Result<TravelTimeReference, SeismicStoreError> {
    let mut resolved_reference = None;
    for interval in intervals {
        let interval_reference =
            resolve_travel_time_reference(control_profile_sets, model, interval)?;
        if let Some(existing_reference) = resolved_reference
            && existing_reference != interval_reference
        {
            return Err(SeismicStoreError::Message(
                "stacked velocity-model compilation currently requires one consistent travel_time_reference across all intervals"
                    .to_string(),
            ));
        }
        resolved_reference = Some(interval_reference);
    }
    Ok(resolved_reference.unwrap_or(TravelTimeReference::TwoWay))
}

fn find_control_profile_set<'a>(
    control_profile_sets: &'a [VelocityControlProfileSet],
    id: &str,
) -> Result<&'a VelocityControlProfileSet, SeismicStoreError> {
    control_profile_sets
        .iter()
        .find(|set| set.id == id)
        .ok_or_else(|| {
            SeismicStoreError::Message(format!(
                "layered velocity model references missing control-profile set '{}'",
                id
            ))
        })
}

fn validate_control_profile_set_alignment(
    control_profile_set: &VelocityControlProfileSet,
    model: &ophiolite_seismic::LayeredVelocityModel,
    handle: &crate::store::StoreHandle,
) -> Result<(), SeismicStoreError> {
    if let Some(model_reference) = model.depth_reference
        && model_reference != control_profile_set.depth_reference
    {
        return Err(SeismicStoreError::Message(format!(
            "layered velocity model depth_reference {:?} does not match control-profile set '{}' reference {:?}",
            model_reference, control_profile_set.id, control_profile_set.depth_reference
        )));
    }
    if !matches!(
        control_profile_set.depth_reference,
        DepthReferenceKind::TrueVerticalDepth | DepthReferenceKind::TrueVerticalDepthSubsea
    ) {
        return Err(SeismicStoreError::Message(
            "survey time-depth transform builder currently requires TVD-style control profiles"
                .to_string(),
        ));
    }
    if let (Some(set_crs), Some(store_crs)) = (
        control_profile_set.coordinate_reference.as_ref(),
        handle
            .manifest
            .volume
            .coordinate_reference_binding
            .as_ref()
            .and_then(|binding| binding.effective.as_ref()),
    ) && set_crs != store_crs
    {
        return Err(SeismicStoreError::Message(
            "control-profile sets must already be aligned into the active survey CRS".to_string(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compile_interval_stack(
    store_root: &Path,
    model: &ophiolite_seismic::LayeredVelocityModel,
    control_profile_sets: &[VelocityControlProfileSet],
    handle: &crate::store::StoreHandle,
    sample_axis_ms: &[f32],
    inline_count: usize,
    xline_count: usize,
    intervals: &[LayeredVelocityInterval],
    preferred_velocity_kind: VelocityQuantityKind,
    travel_time_reference: TravelTimeReference,
) -> Result<Vec<CompiledIntervalVelocityField>, SeismicStoreError> {
    let grid_transform = handle
        .manifest
        .volume
        .spatial
        .as_ref()
        .and_then(|spatial| spatial.grid_transform.as_ref())
        .ok_or_else(|| {
            SeismicStoreError::Message(
                "survey property builders require a resolved survey grid transform".to_string(),
            )
        })?;

    let mut compiled = Vec::with_capacity(intervals.len());
    for interval in intervals {
        let boundary_grid = resolve_interval_boundary_grid(
            store_root,
            interval,
            sample_axis_ms,
            inline_count,
            xline_count,
        )?;
        let trend_curves_m_per_s = build_interval_trend_curves(
            interval,
            sample_axis_ms,
            travel_time_reference,
            &boundary_grid,
        )?;

        let (curves_m_per_s, compilation_note) = if let Some(control_profile_set_id) =
            interval.control_profile_set_id.as_deref()
        {
            let control_profile_set =
                find_control_profile_set(control_profile_sets, control_profile_set_id)?;
            validate_control_profile_set_alignment(control_profile_set, model, handle)?;
            let compiled_control_curves = compile_control_profile_curves(
                control_profile_set,
                sample_axis_ms,
                preferred_velocity_kind,
                interval
                    .vertical_interpolation
                    .unwrap_or(VerticalInterpolationMethod::Linear),
                grid_transform,
                inline_count,
                xline_count,
            )?;
            let control_blend_weight = interval.control_blend_weight.unwrap_or(1.0).clamp(0.0, 1.0);
            let blended_curves_m_per_s = blend_compiled_curves_with_trend_curves(
                &compiled_control_curves,
                &trend_curves_m_per_s,
                &boundary_grid,
                sample_axis_ms,
                control_blend_weight,
            )?;
            (
                blended_curves_m_per_s,
                format!(
                    "Compiled interval '{}' from control-profile set '{}' using {:?} lateral interpolation and {:?} vertical interpolation between '{}' and '{}'.",
                    interval.name,
                    control_profile_set.id,
                    interval
                        .lateral_interpolation
                        .unwrap_or(LateralInterpolationMethod::Nearest),
                    interval
                        .vertical_interpolation
                        .unwrap_or(VerticalInterpolationMethod::Linear),
                    boundary_grid.top_label,
                    boundary_grid.base_label,
                ),
            )
        } else {
            (
                trend_curves_m_per_s,
                format!(
                    "Compiled interval '{}' from trend {:?} between '{}' and '{}' without control profiles.",
                    interval.name,
                    interval.trend,
                    boundary_grid.top_label,
                    boundary_grid.base_label
                ),
            )
        };

        compiled.push(CompiledIntervalVelocityField {
            interval: interval.clone(),
            boundary_grid,
            curves_m_per_s,
            compilation_note,
        });
    }

    Ok(compiled)
}

fn merge_compiled_interval_stack(
    compiled_intervals: &[CompiledIntervalVelocityField],
    sample_axis_ms: &[f32],
) -> Result<(Vec<Vec<f32>>, Vec<u8>), SeismicStoreError> {
    let first_interval = compiled_intervals.first().ok_or_else(|| {
        SeismicStoreError::Message("compiled interval stack must not be empty".to_string())
    })?;
    let trace_count = first_interval.curves_m_per_s.len();
    let sample_count = sample_axis_ms.len();
    let mut merged_curves = vec![vec![0.0; sample_count]; trace_count];
    let mut trace_validity = vec![1_u8; trace_count];

    for trace_index in 0..trace_count {
        let mut valid = true;
        for compiled in compiled_intervals {
            if compiled.curves_m_per_s.len() != trace_count
                || compiled.boundary_grid.validity.len() != trace_count
            {
                return Err(SeismicStoreError::Message(
                    "compiled interval stack trace-count mismatch".to_string(),
                ));
            }
            if compiled.boundary_grid.validity[trace_index] == 0 {
                valid = false;
                break;
            }
        }
        if !valid {
            trace_validity[trace_index] = 0;
            continue;
        }

        let last_interval_index = compiled_intervals.len() - 1;
        let mut assigned_samples = 0usize;
        for (interval_index, compiled) in compiled_intervals.iter().enumerate() {
            let top_time_ms = compiled.boundary_grid.top_times_ms[trace_index];
            let base_time_ms = compiled.boundary_grid.base_times_ms[trace_index];
            if !top_time_ms.is_finite() || !base_time_ms.is_finite() || top_time_ms >= base_time_ms
            {
                valid = false;
                break;
            }
            let curve = &compiled.curves_m_per_s[trace_index];
            if curve.len() != sample_count {
                return Err(SeismicStoreError::Message(
                    "compiled interval curve length does not match the survey sample axis"
                        .to_string(),
                ));
            }
            for (sample_index, time_ms) in sample_axis_ms.iter().copied().enumerate() {
                let in_interval = if interval_index == last_interval_index {
                    time_ms >= top_time_ms && time_ms <= base_time_ms
                } else {
                    time_ms >= top_time_ms && time_ms < base_time_ms
                };
                if in_interval {
                    merged_curves[trace_index][sample_index] = curve[sample_index];
                    assigned_samples += 1;
                }
            }
        }
        if !valid || assigned_samples != sample_count {
            trace_validity[trace_index] = 0;
            merged_curves[trace_index].fill(0.0);
        }
    }

    Ok((merged_curves, trace_validity))
}

fn build_interval_trend_curves(
    interval: &LayeredVelocityInterval,
    sample_axis_ms: &[f32],
    travel_time_reference: TravelTimeReference,
    boundaries: &ResolvedIntervalBoundaryGrid,
) -> Result<Vec<Vec<f32>>, SeismicStoreError> {
    let sample_count = sample_axis_ms.len();
    let mut curves = Vec::with_capacity(boundaries.top_times_ms.len());
    for trace_index in 0..boundaries.top_times_ms.len() {
        if boundaries.validity[trace_index] == 0 {
            curves.push(vec![0.0; sample_count]);
            continue;
        }
        curves.push(build_interval_trend_curve_for_bounds(
            interval,
            sample_axis_ms,
            travel_time_reference,
            boundaries.top_times_ms[trace_index],
            boundaries.base_times_ms[trace_index],
        )?);
    }
    Ok(curves)
}

fn build_interval_trend_curve_for_bounds(
    interval: &LayeredVelocityInterval,
    sample_axis_ms: &[f32],
    travel_time_reference: TravelTimeReference,
    top_time_ms: f32,
    base_time_ms: f32,
) -> Result<Vec<f32>, SeismicStoreError> {
    if sample_axis_ms.is_empty() {
        return Err(SeismicStoreError::Message(
            "survey store sample axis must not be empty".to_string(),
        ));
    }
    if !top_time_ms.is_finite() || !base_time_ms.is_finite() || top_time_ms >= base_time_ms {
        return Err(SeismicStoreError::Message(
            "interval boundaries must resolve to finite increasing sample times".to_string(),
        ));
    }

    let mut curve = Vec::with_capacity(sample_axis_ms.len());
    match &interval.trend {
        VelocityIntervalTrend::Constant { velocity_m_per_s } => {
            validate_velocity_value(*velocity_m_per_s, "interval trend constant velocity")?;
            curve.resize(sample_axis_ms.len(), *velocity_m_per_s);
        }
        VelocityIntervalTrend::LinearWithTime {
            velocity_at_top_m_per_s,
            gradient_m_per_s_per_ms,
        } => {
            validate_velocity_value(
                *velocity_at_top_m_per_s,
                "interval trend top velocity for linear_with_time",
            )?;
            for time_ms in sample_axis_ms {
                let effective_time_ms = (*time_ms).clamp(top_time_ms, base_time_ms);
                let velocity_m_per_s = velocity_at_top_m_per_s
                    + gradient_m_per_s_per_ms * (effective_time_ms - top_time_ms);
                validate_velocity_value(
                    velocity_m_per_s,
                    "interval trend velocity for linear_with_time",
                )?;
                curve.push(velocity_m_per_s);
            }
        }
        VelocityIntervalTrend::LinearWithDepth {
            velocity_at_top_m_per_s,
            gradient_m_per_s_per_m,
        } => {
            validate_velocity_value(
                *velocity_at_top_m_per_s,
                "interval trend top velocity for linear_with_depth",
            )?;
            let mut running_depth_m = 0.0_f32;
            let mut last_effective_time_ms = top_time_ms;
            let mut current_velocity_m_per_s = *velocity_at_top_m_per_s;
            for time_ms in sample_axis_ms.iter().copied() {
                if time_ms <= top_time_ms {
                    curve.push(*velocity_at_top_m_per_s);
                    continue;
                }

                let effective_time_ms = time_ms.min(base_time_ms);
                if effective_time_ms > last_effective_time_ms {
                    let delta_time_ms = effective_time_ms - last_effective_time_ms;
                    let time_factor = travel_time_scale(travel_time_reference);
                    running_depth_m +=
                        delta_time_ms * MILLIS_TO_SECONDS * current_velocity_m_per_s * time_factor;
                    current_velocity_m_per_s =
                        velocity_at_top_m_per_s + gradient_m_per_s_per_m * running_depth_m;
                    last_effective_time_ms = effective_time_ms;
                }
                validate_velocity_value(
                    current_velocity_m_per_s,
                    "interval trend velocity for linear_with_depth",
                )?;
                curve.push(current_velocity_m_per_s);
            }
        }
    }
    Ok(curve)
}

fn compile_control_profile_curves(
    control_profile_set: &VelocityControlProfileSet,
    sample_axis_ms: &[f32],
    velocity_kind: VelocityQuantityKind,
    interpolation_method: VerticalInterpolationMethod,
    grid_transform: &SurveyGridTransform,
    inline_count: usize,
    xline_count: usize,
) -> Result<Vec<Vec<f32>>, SeismicStoreError> {
    let mut prepared_profiles = Vec::new();
    for profile in &control_profile_set.profiles {
        let Some((inline_index, xline_index)) = snap_projected_point_to_grid(
            grid_transform,
            profile.location.x,
            profile.location.y,
            inline_count,
            xline_count,
        ) else {
            continue;
        };
        prepared_profiles.push(PreparedControlProfile {
            inline_index,
            xline_index,
            curve_m_per_s: interpolate_profile_velocity_curve(
                profile,
                sample_axis_ms,
                velocity_kind,
                interpolation_method,
            )?,
        });
    }

    if prepared_profiles.is_empty() {
        return Err(SeismicStoreError::Message(format!(
            "control-profile set '{}' does not contribute any profiles inside the active survey grid",
            control_profile_set.id
        )));
    }

    let compiled_sources = collapse_profiles_by_grid_cell(&prepared_profiles, sample_axis_ms.len());
    assign_nearest_source_curves(
        &compiled_sources,
        grid_transform,
        inline_count,
        xline_count,
        sample_axis_ms.len(),
    )
}

fn interpolate_profile_velocity_curve(
    profile: &VelocityControlProfile,
    sample_axis_ms: &[f32],
    velocity_kind: VelocityQuantityKind,
    interpolation_method: VerticalInterpolationMethod,
) -> Result<Vec<f32>, SeismicStoreError> {
    let mut support = Vec::<(f32, f32)>::new();
    for sample in &profile.samples {
        let Some(velocity_m_per_s) = velocity_from_profile_sample(sample, velocity_kind) else {
            continue;
        };
        validate_velocity_value(velocity_m_per_s, "control-profile velocity")?;
        if !sample.time_ms.is_finite() || sample.time_ms < 0.0 {
            return Err(SeismicStoreError::Message(format!(
                "control-profile '{}' contains an invalid time sample {}",
                profile.id, sample.time_ms
            )));
        }
        support.push((sample.time_ms, velocity_m_per_s));
    }

    if support.is_empty() {
        return Err(SeismicStoreError::Message(format!(
            "control-profile '{}' does not contain any {:?} velocity samples",
            profile.id, velocity_kind
        )));
    }

    support.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut curve = Vec::with_capacity(sample_axis_ms.len());
    for time_ms in sample_axis_ms {
        curve.push(interpolate_support_velocity(
            *time_ms,
            &support,
            interpolation_method,
        ));
    }
    Ok(curve)
}

fn velocity_from_profile_sample(
    sample: &ophiolite_seismic::VelocityControlProfileSample,
    velocity_kind: VelocityQuantityKind,
) -> Option<f32> {
    match velocity_kind {
        VelocityQuantityKind::Interval => sample.vint_m_per_s,
        VelocityQuantityKind::Rms => sample.vrms_m_per_s,
        VelocityQuantityKind::Average => sample.vavg_m_per_s,
    }
}

fn interpolate_support_velocity(
    time_ms: f32,
    support: &[(f32, f32)],
    interpolation_method: VerticalInterpolationMethod,
) -> f32 {
    if support.len() == 1 {
        return support[0].1;
    }
    let mut upper_index = 0usize;
    while upper_index < support.len() && support[upper_index].0 < time_ms {
        upper_index += 1;
    }
    if upper_index == 0 {
        return support[0].1;
    }
    if upper_index >= support.len() {
        return support[support.len() - 1].1;
    }
    let lower_index = upper_index - 1;
    let (lower_time_ms, lower_velocity_m_per_s) = support[lower_index];
    let (upper_time_ms, upper_velocity_m_per_s) = support[upper_index];
    match interpolation_method {
        VerticalInterpolationMethod::Step => lower_velocity_m_per_s,
        VerticalInterpolationMethod::Linear => {
            if (upper_time_ms - lower_time_ms).abs() <= f32::EPSILON {
                upper_velocity_m_per_s
            } else {
                let t =
                    ((time_ms - lower_time_ms) / (upper_time_ms - lower_time_ms)).clamp(0.0, 1.0);
                lower_velocity_m_per_s + (upper_velocity_m_per_s - lower_velocity_m_per_s) * t
            }
        }
        VerticalInterpolationMethod::MonotonicCubic => unreachable!(),
    }
}

fn collapse_profiles_by_grid_cell(
    profiles: &[PreparedControlProfile],
    sample_count: usize,
) -> Vec<CompiledControlCurveSource> {
    let mut grouped = HashMap::<(usize, usize), (Vec<f32>, usize)>::new();
    for profile in profiles {
        let entry = grouped
            .entry((profile.inline_index, profile.xline_index))
            .or_insert_with(|| (vec![0.0; sample_count], 0));
        for (target, value) in entry.0.iter_mut().zip(&profile.curve_m_per_s) {
            *target += *value;
        }
        entry.1 += 1;
    }

    grouped
        .into_iter()
        .map(
            |((inline_index, xline_index), (mut curve_m_per_s, count))| {
                let scale = 1.0 / count as f32;
                for value in &mut curve_m_per_s {
                    *value *= scale;
                }
                CompiledControlCurveSource {
                    inline_index,
                    xline_index,
                    curve_m_per_s,
                }
            },
        )
        .collect()
}

fn assign_nearest_source_curves(
    sources: &[CompiledControlCurveSource],
    grid_transform: &SurveyGridTransform,
    inline_count: usize,
    xline_count: usize,
    sample_count: usize,
) -> Result<Vec<Vec<f32>>, SeismicStoreError> {
    let cell_count = inline_count * xline_count;
    let mut distances = vec![u32::MAX; cell_count];
    let mut assignments = vec![None::<usize>; cell_count];
    let mut queue = BinaryHeap::<GridSearchState>::new();

    for (source_index, source) in sources.iter().enumerate() {
        let offset = source.inline_index * xline_count + source.xline_index;
        distances[offset] = 0;
        assignments[offset] = Some(source_index);
        queue.push(GridSearchState {
            distance_milli: 0,
            inline_index: source.inline_index,
            xline_index: source.xline_index,
            source_index,
        });
    }

    let inline_step = ((grid_transform.inline_basis.x * grid_transform.inline_basis.x)
        + (grid_transform.inline_basis.y * grid_transform.inline_basis.y))
        .sqrt();
    let xline_step = ((grid_transform.xline_basis.x * grid_transform.xline_basis.x)
        + (grid_transform.xline_basis.y * grid_transform.xline_basis.y))
        .sqrt();
    let diagonal_plus = (((grid_transform.inline_basis.x + grid_transform.xline_basis.x)
        * (grid_transform.inline_basis.x + grid_transform.xline_basis.x))
        + ((grid_transform.inline_basis.y + grid_transform.xline_basis.y)
            * (grid_transform.inline_basis.y + grid_transform.xline_basis.y)))
        .sqrt();
    let diagonal_minus = (((grid_transform.inline_basis.x - grid_transform.xline_basis.x)
        * (grid_transform.inline_basis.x - grid_transform.xline_basis.x))
        + ((grid_transform.inline_basis.y - grid_transform.xline_basis.y)
            * (grid_transform.inline_basis.y - grid_transform.xline_basis.y)))
        .sqrt();
    let neighbor_steps = [
        (1_isize, 0_isize, round_distance_milli(inline_step)),
        (-1, 0, round_distance_milli(inline_step)),
        (0, 1, round_distance_milli(xline_step)),
        (0, -1, round_distance_milli(xline_step)),
        (1, 1, round_distance_milli(diagonal_plus)),
        (-1, -1, round_distance_milli(diagonal_plus)),
        (1, -1, round_distance_milli(diagonal_minus)),
        (-1, 1, round_distance_milli(diagonal_minus)),
    ];

    while let Some(state) = queue.pop() {
        let offset = state.inline_index * xline_count + state.xline_index;
        if state.distance_milli != distances[offset]
            || assignments[offset] != Some(state.source_index)
        {
            continue;
        }

        for (inline_step_index, xline_step_index, step_cost_milli) in &neighbor_steps {
            let next_inline = state.inline_index as isize + inline_step_index;
            let next_xline = state.xline_index as isize + xline_step_index;
            if next_inline < 0
                || next_inline >= inline_count as isize
                || next_xline < 0
                || next_xline >= xline_count as isize
            {
                continue;
            }
            let next_inline = next_inline as usize;
            let next_xline = next_xline as usize;
            let next_offset = next_inline * xline_count + next_xline;
            let next_distance = state.distance_milli.saturating_add(*step_cost_milli);
            if next_distance < distances[next_offset] {
                distances[next_offset] = next_distance;
                assignments[next_offset] = Some(state.source_index);
                queue.push(GridSearchState {
                    distance_milli: next_distance,
                    inline_index: next_inline,
                    xline_index: next_xline,
                    source_index: state.source_index,
                });
            }
        }
    }

    let mut curves = Vec::with_capacity(cell_count);
    for assignment in assignments {
        let source_index = assignment.ok_or_else(|| {
            SeismicStoreError::Message(
                "failed to assign every survey cell to a control-profile source".to_string(),
            )
        })?;
        let curve = &sources[source_index].curve_m_per_s;
        if curve.len() != sample_count {
            return Err(SeismicStoreError::Message(
                "compiled control-profile curve length mismatch".to_string(),
            ));
        }
        curves.push(curve.clone());
    }
    Ok(curves)
}

fn blend_compiled_curves_with_trend_curves(
    curves: &[Vec<f32>],
    trend_curves_m_per_s: &[Vec<f32>],
    boundaries: &ResolvedIntervalBoundaryGrid,
    sample_axis_ms: &[f32],
    control_blend_weight: f32,
) -> Result<Vec<Vec<f32>>, SeismicStoreError> {
    let trend_weight = 1.0 - control_blend_weight;
    let mut blended = Vec::with_capacity(curves.len());
    for (trace_index, curve) in curves.iter().enumerate() {
        let trend_curve_m_per_s = trend_curves_m_per_s.get(trace_index).ok_or_else(|| {
            SeismicStoreError::Message(
                "compiled trend-curve count does not match control-curve count".to_string(),
            )
        })?;
        if curve.len() != trend_curve_m_per_s.len() {
            return Err(SeismicStoreError::Message(
                "compiled control-profile curve and trend curve length mismatch".to_string(),
            ));
        }
        if boundaries.validity.get(trace_index).copied().unwrap_or(0) == 0 {
            blended.push(vec![0.0; curve.len()]);
            continue;
        }
        let mut out = Vec::with_capacity(curve.len());
        for (control_value, trend_value) in curve.iter().zip(trend_curve_m_per_s.iter()) {
            let velocity_m_per_s =
                control_value * control_blend_weight + trend_value * trend_weight;
            validate_velocity_value(velocity_m_per_s, "blended interval velocity")?;
            out.push(velocity_m_per_s);
        }
        clamp_curve_outside_interval_edges(
            &mut out,
            sample_axis_ms,
            boundaries.top_times_ms[trace_index],
            boundaries.base_times_ms[trace_index],
        )?;
        blended.push(out);
    }
    Ok(blended)
}

fn compile_depth_payload_from_velocity_curves(
    curves_m_per_s: &[Vec<f32>],
    validity: &[u8],
    sample_axis_ms: &[f32],
    travel_time_reference: TravelTimeReference,
    inline_count: usize,
    xline_count: usize,
    sample_count: usize,
) -> Result<(Vec<f32>, Vec<u8>), SeismicStoreError> {
    if curves_m_per_s.len() != inline_count * xline_count {
        return Err(SeismicStoreError::Message(
            "compiled velocity-curve count does not match the survey grid".to_string(),
        ));
    }
    if validity.len() != curves_m_per_s.len() * sample_count {
        return Err(SeismicStoreError::Message(
            "compiled velocity-field validity does not match the survey grid".to_string(),
        ));
    }

    let mut depths_m = Vec::with_capacity(curves_m_per_s.len() * sample_count);
    let mut depth_validity = Vec::with_capacity(curves_m_per_s.len() * sample_count);
    for (trace_index, curve_m_per_s) in curves_m_per_s.iter().enumerate() {
        if curve_m_per_s.len() != sample_count {
            return Err(SeismicStoreError::Message(
                "compiled velocity curve length does not match the survey sample axis".to_string(),
            ));
        }
        let trace_start = trace_index * sample_count;
        let trace_valid = validity[trace_start..trace_start + sample_count]
            .iter()
            .all(|value| *value != 0);
        if !trace_valid {
            depths_m.extend(std::iter::repeat_n(0.0_f32, sample_count));
            depth_validity.extend(std::iter::repeat_n(0_u8, sample_count));
            continue;
        }
        let trace_depths_m = integrate_velocity_curve_to_depth(
            curve_m_per_s,
            sample_axis_ms,
            travel_time_reference,
        )?;
        depths_m.extend(trace_depths_m);
        depth_validity.extend(std::iter::repeat_n(1_u8, sample_count));
    }
    Ok((depths_m, depth_validity))
}

fn integrate_velocity_curve_to_depth(
    curve_m_per_s: &[f32],
    sample_axis_ms: &[f32],
    travel_time_reference: TravelTimeReference,
) -> Result<Vec<f32>, SeismicStoreError> {
    if curve_m_per_s.is_empty() || sample_axis_ms.is_empty() {
        return Err(SeismicStoreError::Message(
            "velocity curve must not be empty".to_string(),
        ));
    }
    if curve_m_per_s.len() != sample_axis_ms.len() {
        return Err(SeismicStoreError::Message(
            "velocity curve and sample axis length mismatch".to_string(),
        ));
    }

    let mut depths_m = Vec::with_capacity(curve_m_per_s.len());
    let time_factor = travel_time_scale(travel_time_reference);
    for (index, velocity_m_per_s) in curve_m_per_s.iter().copied().enumerate() {
        validate_velocity_value(velocity_m_per_s, "compiled interval velocity")?;
        if index == 0 {
            let first_time_ms = sample_axis_ms[0].max(0.0);
            depths_m.push(first_time_ms * MILLIS_TO_SECONDS * velocity_m_per_s * time_factor);
            continue;
        }
        let previous_velocity_m_per_s = curve_m_per_s[index - 1];
        let previous_depth_m = *depths_m.last().expect("depths contain previous sample");
        let delta_time_ms = sample_axis_ms[index] - sample_axis_ms[index - 1];
        let delta_depth_m = delta_time_ms
            * MILLIS_TO_SECONDS
            * (previous_velocity_m_per_s + velocity_m_per_s)
            * 0.5
            * time_factor;
        depths_m.push(previous_depth_m + delta_depth_m);
    }
    Ok(depths_m)
}

fn travel_time_scale(reference: TravelTimeReference) -> f32 {
    match reference {
        TravelTimeReference::OneWay => 1.0,
        TravelTimeReference::TwoWay => 0.5,
    }
}

fn lookup_horizon_grid<'a>(
    horizons_by_id: &'a HashMap<&str, &'a ImportedHorizonGrid>,
    horizon_id: &str,
) -> Result<&'a ImportedHorizonGrid, SeismicStoreError> {
    horizons_by_id.get(horizon_id).copied().ok_or_else(|| {
        SeismicStoreError::Message(format!(
            "horizon asset '{}' was not found in the store",
            horizon_id
        ))
    })
}

fn paired_boundary_pairs_for_trace(
    trace_index: usize,
    paired_horizons: &[(&ImportedHorizonGrid, &ImportedHorizonGrid)],
    last_sample_time_ms: f32,
) -> Result<Vec<(f32, f32)>, SeismicStoreError> {
    let mut pairs = Vec::with_capacity(paired_horizons.len() + 1);
    pairs.push((0.0, 0.0));
    for (time_horizon, depth_horizon) in paired_horizons {
        let time_valid = time_horizon.validity.get(trace_index).copied().unwrap_or(0) != 0;
        let depth_valid = depth_horizon
            .validity
            .get(trace_index)
            .copied()
            .unwrap_or(0)
            != 0;
        if !time_valid || !depth_valid {
            return Err(SeismicStoreError::Message(
                "paired-horizon trace is missing one or more boundary values".to_string(),
            ));
        }

        let time_ms = time_horizon
            .values
            .get(trace_index)
            .copied()
            .ok_or_else(|| {
                SeismicStoreError::Message(
                    "time horizon payload did not contain the requested trace".to_string(),
                )
            })?;
        let depth_m = depth_horizon
            .values
            .get(trace_index)
            .copied()
            .ok_or_else(|| {
                SeismicStoreError::Message(
                    "depth horizon payload did not contain the requested trace".to_string(),
                )
            })?;
        if !time_ms.is_finite() || !depth_m.is_finite() {
            return Err(SeismicStoreError::Message(
                "paired-horizon trace contains non-finite values".to_string(),
            ));
        }
        if time_ms > last_sample_time_ms + AXIS_TOLERANCE {
            return Err(SeismicStoreError::Message(format!(
                "paired-horizon time boundary {time_ms} ms exceeds the survey sample axis maximum {last_sample_time_ms} ms"
            )));
        }

        let (previous_time_ms, previous_depth_m) = *pairs.last().expect("paired boundary origin");
        if time_ms <= previous_time_ms + AXIS_TOLERANCE {
            return Err(SeismicStoreError::Message(
                "paired-horizon time boundaries must increase strictly for every trace".to_string(),
            ));
        }
        if depth_m <= previous_depth_m + AXIS_TOLERANCE {
            return Err(SeismicStoreError::Message(
                "paired-horizon depth boundaries must increase strictly for every trace"
                    .to_string(),
            ));
        }
        pairs.push((time_ms, depth_m));
    }
    Ok(pairs)
}

fn compile_trace_depths_from_boundary_pairs(
    sample_axis_ms: &[f32],
    boundary_pairs: &[(f32, f32)],
) -> Result<Vec<f32>, SeismicStoreError> {
    if sample_axis_ms.is_empty() {
        return Err(SeismicStoreError::Message(
            "paired-horizon transform builder requires a non-empty sample axis".to_string(),
        ));
    }
    if boundary_pairs.len() < 2 {
        return Err(SeismicStoreError::Message(
            "paired-horizon transform builder requires at least one paired boundary".to_string(),
        ));
    }

    let mut depths_m = Vec::with_capacity(sample_axis_ms.len());
    let mut segment_index = 0_usize;
    for sample_time_ms in sample_axis_ms.iter().copied() {
        while segment_index + 1 < boundary_pairs.len()
            && sample_time_ms > boundary_pairs[segment_index + 1].0
        {
            segment_index += 1;
        }
        let depth_m = if sample_time_ms <= boundary_pairs.last().expect("boundary").0 {
            let lower = boundary_pairs[segment_index];
            let upper = boundary_pairs
                .get(segment_index + 1)
                .copied()
                .ok_or_else(|| {
                    SeismicStoreError::Message(
                        "paired-horizon transform builder ran out of boundary segments".to_string(),
                    )
                })?;
            interpolate_depth_on_time_segment(sample_time_ms, lower, upper)?
        } else {
            let lower = boundary_pairs[boundary_pairs.len() - 2];
            let upper = boundary_pairs[boundary_pairs.len() - 1];
            interpolate_depth_on_time_segment(sample_time_ms, lower, upper)?
        };
        depths_m.push(depth_m);
    }
    Ok(depths_m)
}

fn interpolate_depth_on_time_segment(
    sample_time_ms: f32,
    lower: (f32, f32),
    upper: (f32, f32),
) -> Result<f32, SeismicStoreError> {
    let delta_time_ms = upper.0 - lower.0;
    let delta_depth_m = upper.1 - lower.1;
    if delta_time_ms <= AXIS_TOLERANCE || delta_depth_m <= AXIS_TOLERANCE {
        return Err(SeismicStoreError::Message(
            "paired-horizon transform segments must be strictly increasing in time and depth"
                .to_string(),
        ));
    }
    let slope_m_per_ms = delta_depth_m / delta_time_ms;
    Ok(lower.1 + (sample_time_ms - lower.0) * slope_m_per_ms)
}

fn validate_velocity_value(value: f32, label: &str) -> Result<(), SeismicStoreError> {
    if !value.is_finite() || value < MIN_SUPPORTED_VELOCITY_M_PER_S {
        return Err(SeismicStoreError::Message(format!(
            "{label} must be finite and >= {MIN_SUPPORTED_VELOCITY_M_PER_S} m/s, found {value}"
        )));
    }
    Ok(())
}

fn clamp_curve_outside_interval_edges(
    curve_m_per_s: &mut [f32],
    sample_axis_ms: &[f32],
    top_time_ms: f32,
    base_time_ms: f32,
) -> Result<(), SeismicStoreError> {
    if curve_m_per_s.len() != sample_axis_ms.len() {
        return Err(SeismicStoreError::Message(
            "curve and sample axis length mismatch when clamping interval boundaries".to_string(),
        ));
    }
    let top_velocity_m_per_s =
        interpolate_curve_value_at_time(curve_m_per_s, sample_axis_ms, top_time_ms)?;
    let base_velocity_m_per_s =
        interpolate_curve_value_at_time(curve_m_per_s, sample_axis_ms, base_time_ms)?;
    for (index, time_ms) in sample_axis_ms.iter().copied().enumerate() {
        if time_ms < top_time_ms {
            curve_m_per_s[index] = top_velocity_m_per_s;
        } else if time_ms > base_time_ms {
            curve_m_per_s[index] = base_velocity_m_per_s;
        }
    }
    Ok(())
}

fn interpolate_curve_value_at_time(
    curve_m_per_s: &[f32],
    sample_axis_ms: &[f32],
    target_time_ms: f32,
) -> Result<f32, SeismicStoreError> {
    if curve_m_per_s.len() != sample_axis_ms.len() || curve_m_per_s.is_empty() {
        return Err(SeismicStoreError::Message(
            "curve and sample axis length mismatch when interpolating interval edge velocity"
                .to_string(),
        ));
    }
    if target_time_ms <= sample_axis_ms[0] {
        return Ok(curve_m_per_s[0]);
    }
    let last_index = sample_axis_ms.len() - 1;
    if target_time_ms >= sample_axis_ms[last_index] {
        return Ok(curve_m_per_s[last_index]);
    }

    let mut upper_index = 0usize;
    while upper_index < sample_axis_ms.len() && sample_axis_ms[upper_index] < target_time_ms {
        upper_index += 1;
    }
    if upper_index == 0 {
        return Ok(curve_m_per_s[0]);
    }
    if upper_index >= sample_axis_ms.len() {
        return Ok(curve_m_per_s[last_index]);
    }

    let lower_index = upper_index - 1;
    let lower_time_ms = sample_axis_ms[lower_index];
    let upper_time_ms = sample_axis_ms[upper_index];
    if (upper_time_ms - lower_time_ms).abs() <= f32::EPSILON {
        return Ok(curve_m_per_s[upper_index]);
    }
    let t = ((target_time_ms - lower_time_ms) / (upper_time_ms - lower_time_ms)).clamp(0.0, 1.0);
    Ok(curve_m_per_s[lower_index] + (curve_m_per_s[upper_index] - curve_m_per_s[lower_index]) * t)
}

fn expand_trace_validity_to_samples(trace_validity: &[u8], sample_count: usize) -> Vec<u8> {
    let mut validity = Vec::with_capacity(trace_validity.len() * sample_count);
    for trace_valid in trace_validity {
        validity.extend(std::iter::repeat_n(*trace_valid, sample_count));
    }
    validity
}

fn derived_transform_sources(request: &BuildSurveyTimeDepthTransformRequest) -> Vec<String> {
    let mut derived_from = request.model.derived_from.clone();
    derived_from.extend(referenced_horizon_ids(&request.model));
    for set in &request.control_profile_sets {
        derived_from.push(set.id.clone());
        derived_from.extend(set.derived_from.iter().cloned());
    }
    derived_from.sort();
    derived_from.dedup();
    derived_from
}

fn derived_property_field_sources(
    model: &ophiolite_seismic::LayeredVelocityModel,
    control_profile_sets: &[VelocityControlProfileSet],
) -> Vec<String> {
    let mut derived_from = model.derived_from.clone();
    derived_from.extend(referenced_horizon_ids(model));
    for set in control_profile_sets {
        derived_from.push(set.id.clone());
        derived_from.extend(set.derived_from.iter().cloned());
    }
    derived_from.sort();
    derived_from.dedup();
    derived_from
}

fn referenced_horizon_ids(model: &ophiolite_seismic::LayeredVelocityModel) -> Vec<String> {
    let mut horizon_ids = Vec::new();
    for interval in &model.intervals {
        if let StratigraphicBoundaryReference::HorizonAsset { horizon_id } = &interval.top_boundary
        {
            horizon_ids.push(horizon_id.clone());
        }
        if let StratigraphicBoundaryReference::HorizonAsset { horizon_id } = &interval.base_boundary
        {
            horizon_ids.push(horizon_id.clone());
        }
    }
    horizon_ids
}

fn time_depth_source_kind_for_model(
    model: &ophiolite_seismic::LayeredVelocityModel,
) -> TimeDepthTransformSourceKind {
    let intervals = match resolve_supported_intervals_from_model(model) {
        Ok(intervals) => intervals,
        Err(_) => return TimeDepthTransformSourceKind::ConstantVelocity,
    };
    if intervals
        .iter()
        .any(|interval| interval.control_profile_set_id.is_some())
        || intervals.len() > 1
        || intervals.iter().any(|interval| {
            !matches!(
                interval.top_boundary,
                StratigraphicBoundaryReference::SurveyTop
            ) || !matches!(
                interval.base_boundary,
                StratigraphicBoundaryReference::SurveyBase
            )
        })
    {
        TimeDepthTransformSourceKind::VelocityGrid3D
    } else {
        TimeDepthTransformSourceKind::ConstantVelocity
    }
}

fn round_distance_milli(distance: f64) -> u32 {
    distance
        .mul_add(1000.0, 0.0)
        .round()
        .clamp(1.0, u32::MAX as f64) as u32
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
    if inline_snapped < 0.0
        || inline_snapped >= inline_count as f64
        || xline_snapped < 0.0
        || xline_snapped >= xline_count as f64
    {
        return None;
    }

    Some((inline_snapped as usize, xline_snapped as usize))
}

fn validate_time_axis_matches_store(
    descriptor: &SurveyTimeDepthTransform3D,
    sample_axis_ms: &[f32],
) -> Result<(), SeismicStoreError> {
    if descriptor.time_axis.unit.trim() != "ms" {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' currently requires a millisecond time axis",
            descriptor.id
        )));
    }
    if sample_axis_ms.is_empty() {
        return Err(SeismicStoreError::Message(
            "survey store sample axis must not be empty".to_string(),
        ));
    }
    if descriptor.time_axis.count != sample_axis_ms.len() {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' sample count mismatch against store sample axis",
            descriptor.id
        )));
    }
    if (descriptor.time_axis.start - sample_axis_ms[0]).abs() > AXIS_TOLERANCE {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' time-axis start does not match the store sample axis",
            descriptor.id
        )));
    }
    let descriptor_step = descriptor.time_axis.step;
    let store_step = if sample_axis_ms.len() >= 2 {
        sample_axis_ms[1] - sample_axis_ms[0]
    } else {
        0.0
    };
    if (descriptor_step - store_step).abs() > AXIS_TOLERANCE {
        return Err(SeismicStoreError::Message(format!(
            "survey time-depth transform '{}' time-axis step does not match the store sample axis",
            descriptor.id
        )));
    }
    Ok(())
}

fn validate_property_field_axis_matches_store(
    descriptor: &SurveyPropertyField3D,
    sample_axis_ms: &[f32],
) -> Result<(), SeismicStoreError> {
    if descriptor.vertical_axis.unit.trim() != "ms" {
        return Err(SeismicStoreError::Message(format!(
            "survey property field '{}' currently requires a millisecond vertical axis",
            descriptor.id
        )));
    }
    if sample_axis_ms.is_empty() {
        return Err(SeismicStoreError::Message(
            "survey store sample axis must not be empty".to_string(),
        ));
    }
    if descriptor.vertical_axis.count != sample_axis_ms.len() {
        return Err(SeismicStoreError::Message(format!(
            "survey property field '{}' sample count mismatch against store sample axis",
            descriptor.id
        )));
    }
    if (descriptor.vertical_axis.start - sample_axis_ms[0]).abs() > AXIS_TOLERANCE {
        return Err(SeismicStoreError::Message(format!(
            "survey property field '{}' vertical-axis start does not match the store sample axis",
            descriptor.id
        )));
    }
    let descriptor_step = descriptor.vertical_axis.step;
    let store_step = if sample_axis_ms.len() >= 2 {
        sample_axis_ms[1] - sample_axis_ms[0]
    } else {
        0.0
    };
    if (descriptor_step - store_step).abs() > AXIS_TOLERANCE {
        return Err(SeismicStoreError::Message(format!(
            "survey property field '{}' vertical-axis step does not match the store sample axis",
            descriptor.id
        )));
    }
    Ok(())
}

fn validate_transform_alignment(
    descriptor_crs: Option<&CoordinateReferenceDescriptor>,
    survey_crs: Option<&CoordinateReferenceDescriptor>,
    descriptor_grid_transform: Option<&SurveyGridTransform>,
    survey_grid_transform: Option<&SurveyGridTransform>,
) -> Result<(), SeismicStoreError> {
    if let (Some(descriptor_crs), Some(survey_crs)) = (descriptor_crs, survey_crs)
        && descriptor_crs != survey_crs
    {
        return Err(SeismicStoreError::Message(
            "survey time-depth transforms must already be aligned into the active survey CRS"
                .to_string(),
        ));
    }
    if let (Some(descriptor_grid_transform), Some(survey_grid_transform)) =
        (descriptor_grid_transform, survey_grid_transform)
        && descriptor_grid_transform != survey_grid_transform
    {
        return Err(SeismicStoreError::Message(
            "survey time-depth transforms must already be aligned to the active survey grid transform"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_property_field_alignment(
    descriptor_crs: Option<&CoordinateReferenceDescriptor>,
    survey_crs: Option<&CoordinateReferenceDescriptor>,
    descriptor_grid_transform: Option<&SurveyGridTransform>,
    survey_grid_transform: Option<&SurveyGridTransform>,
) -> Result<(), SeismicStoreError> {
    if let (Some(descriptor_crs), Some(survey_crs)) = (descriptor_crs, survey_crs)
        && descriptor_crs != survey_crs
    {
        return Err(SeismicStoreError::Message(
            "survey property fields must already be aligned into the active survey CRS".to_string(),
        ));
    }
    if let (Some(descriptor_grid_transform), Some(survey_grid_transform)) =
        (descriptor_grid_transform, survey_grid_transform)
        && descriptor_grid_transform != survey_grid_transform
    {
        return Err(SeismicStoreError::Message(
            "survey property fields must already be aligned to the active survey grid transform"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_depth_payload(
    descriptor: &SurveyTimeDepthTransform3D,
    depths_m: &[f32],
    validity: &[u8],
) -> Result<(), SeismicStoreError> {
    let trace_count = descriptor.inline_count * descriptor.xline_count;
    let sample_count = descriptor.sample_count;
    for trace_index in 0..trace_count {
        let mut previous_depth = None;
        for sample_index in 0..sample_count {
            let offset = trace_index * sample_count + sample_index;
            if validity[offset] == 0 {
                continue;
            }
            let depth_m = depths_m[offset];
            if !depth_m.is_finite() {
                return Err(SeismicStoreError::Message(format!(
                    "survey time-depth transform '{}' contains a non-finite depth value",
                    descriptor.id
                )));
            }
            if let Some(previous_depth) = previous_depth
                && depth_m < previous_depth
            {
                return Err(SeismicStoreError::Message(format!(
                    "survey time-depth transform '{}' must be nondecreasing within each trace",
                    descriptor.id
                )));
            }
            previous_depth = Some(depth_m);
        }
    }
    Ok(())
}

fn derive_coverage_relationship(validity: &[u8]) -> SpatialCoverageRelationship {
    let valid_count = validity.iter().filter(|value| **value != 0).count();
    if valid_count == 0 {
        SpatialCoverageRelationship::Disjoint
    } else if valid_count == validity.len() {
        SpatialCoverageRelationship::Exact
    } else {
        SpatialCoverageRelationship::PartialOverlap
    }
}

fn load_transform_manifest(
    transforms_root: &Path,
) -> Result<SurveyTimeDepthTransformStoreManifest, SeismicStoreError> {
    let manifest_path = transforms_root.join(TRANSFORM_MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(SurveyTimeDepthTransformStoreManifest::default());
    }
    Ok(serde_json::from_slice(&fs::read(&manifest_path)?)?)
}

fn save_transform_manifest(
    transforms_root: &Path,
    manifest: &SurveyTimeDepthTransformStoreManifest,
) -> Result<(), SeismicStoreError> {
    fs::write(
        transforms_root.join(TRANSFORM_MANIFEST_FILE),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn load_property_field_manifest(
    fields_root: &Path,
) -> Result<SurveyPropertyFieldStoreManifest, SeismicStoreError> {
    let manifest_path = fields_root.join(PROPERTY_FIELD_MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(SurveyPropertyFieldStoreManifest::default());
    }
    Ok(serde_json::from_slice(&fs::read(&manifest_path)?)?)
}

fn save_property_field_manifest(
    fields_root: &Path,
    manifest: &SurveyPropertyFieldStoreManifest,
) -> Result<(), SeismicStoreError> {
    fs::write(
        fields_root.join(PROPERTY_FIELD_MANIFEST_FILE),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn f32_slice_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(values));
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn write_f32le_file(path: impl AsRef<Path>, values: &[f32]) -> Result<(), SeismicStoreError> {
    fs::write(path.as_ref(), f32_slice_to_le_bytes(values))?;
    Ok(())
}

fn read_f32le_file(path: &Path) -> Result<Vec<f32>, SeismicStoreError> {
    let bytes = fs::read(path)?;
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "expected f32 payload at {}, found {} bytes",
            path.display(),
            bytes.len()
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
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn unique_transform_id(base: &str, occupied: &HashSet<String>) -> String {
    if !occupied.contains(base) {
        return base.to_string();
    }

    let mut counter = 2usize;
    loop {
        let candidate = format!("{base}-{counter}");
        if !occupied.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use ndarray::Array3;
    use tempfile::tempdir;

    use crate::horizons::{import_horizon_xyzs, import_horizon_xyzs_with_vertical_domain};
    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        VolumeMetadata,
    };
    use crate::storage::tbvol::TbvolManifest;
    use crate::store::create_tbvol_store;
    use crate::{
        CoordinateReferenceBinding, CoordinateReferenceSource, ProjectedPoint2, ProjectedVector2,
        SurveySpatialAvailability, SurveySpatialDescriptor,
    };
    use ophiolite_seismic::{
        BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest, DepthReferenceKind,
        LateralInterpolationMethod, LayeredVelocityInterval, LayeredVelocityModel,
        StratigraphicBoundaryReference, TimeDepthTransformSourceKind, TravelTimeReference,
        VelocityControlProfile, VelocityControlProfileSample, VelocityControlProfileSet,
        VelocityIntervalTrend, VelocityQuantityKind, VerticalInterpolationMethod,
    };

    use super::*;

    fn assert_f32_slice_close(actual: &[f32], expected: &[f32], tolerance: f32) {
        assert_eq!(actual.len(), expected.len());
        for (left, right) in actual.iter().zip(expected.iter()) {
            assert!(
                (*left - *right).abs() <= tolerance,
                "expected {right}, got {left}"
            );
        }
    }

    #[test]
    fn stores_and_slices_survey_time_depth_transform() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 4,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
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
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 10.0, 20.0, 30.0],
                ),
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
                        origin: ProjectedPoint2 { x: 0.0, y: 0.0 },
                        inline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                        xline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
            },
            [2, 2, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 2, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let descriptor = SurveyTimeDepthTransform3D {
            id: String::from("velocity-f3"),
            name: String::from("Velocity F3"),
            derived_from: Vec::new(),
            source_kind: ophiolite_seismic::TimeDepthTransformSourceKind::VelocityGrid3D,
            coordinate_reference: None,
            grid_transform: None,
            time_axis: ophiolite_seismic::VerticalAxisDescriptor {
                domain: TimeDepthDomain::Time,
                unit: String::from("ms"),
                start: 0.0,
                step: 10.0,
                count: 4,
            },
            depth_unit: String::from("m"),
            inline_count: 2,
            xline_count: 2,
            sample_count: 4,
            coverage: SpatialCoverageSummary {
                relationship: SpatialCoverageRelationship::Unknown,
                source_coordinate_reference: None,
                target_coordinate_reference: None,
                notes: Vec::new(),
            },
            notes: Vec::new(),
        };
        let depths_m = vec![
            0.0, 12.0, 24.0, 36.0, // inline 0 xline 0
            0.0, 18.0, 36.0, 54.0, // inline 0 xline 1
            0.0, 10.0, 20.0, 30.0, // inline 1 xline 0
            0.0, 16.0, 32.0, 48.0, // inline 1 xline 1
        ];
        let validity = vec![1_u8; depths_m.len()];
        store_survey_time_depth_transform(&store_root, descriptor, &depths_m, &validity)
            .expect("store transform");

        let slice =
            section_time_depth_transform_slice(&store_root, "velocity-f3", SectionAxis::Inline, 0)
                .expect("slice transform");
        assert_eq!(
            slice.coverage_relationship,
            SpatialCoverageRelationship::Exact
        );
        assert_eq!(slice.trace_depths_m.len(), 2);
        assert_eq!(slice.trace_depths_m[0], vec![0.0, 12.0, 24.0, 36.0]);
        assert_eq!(slice.trace_depths_m[1], vec![0.0, 18.0, 36.0, 54.0]);
        assert_eq!(slice.trace_validity, vec![true, true]);
    }

    #[test]
    fn integrates_velocity_curve_from_nonzero_sample_origin() {
        let depths_m = integrate_velocity_curve_to_depth(
            &[1500.0, 1500.0, 1500.0],
            &[4.0, 8.0, 12.0],
            TravelTimeReference::TwoWay,
        )
        .expect("integrate constant interval velocity");
        assert_f32_slice_close(&depths_m, &[3.0, 6.0, 9.0], 1e-4);
    }

    #[test]
    fn builds_survey_time_depth_transform_from_control_profiles() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 4,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
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
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 10.0, 20.0, 30.0],
                ),
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
                        origin: ProjectedPoint2 { x: 0.0, y: 0.0 },
                        inline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                        xline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
            },
            [2, 2, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 2, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let request = BuildSurveyTimeDepthTransformRequest {
            schema_version: 2,
            store_path: store_root.to_string_lossy().into_owned(),
            model: LayeredVelocityModel {
                id: String::from("layered-model"),
                name: String::from("Layered Model"),
                derived_from: Vec::new(),
                coordinate_reference: None,
                grid_transform: None,
                vertical_domain: TimeDepthDomain::Time,
                travel_time_reference: Some(TravelTimeReference::TwoWay),
                depth_reference: Some(DepthReferenceKind::TrueVerticalDepth),
                intervals: vec![LayeredVelocityInterval {
                    id: String::from("main"),
                    name: String::from("Main"),
                    top_boundary: StratigraphicBoundaryReference::SurveyTop,
                    base_boundary: StratigraphicBoundaryReference::SurveyBase,
                    trend: VelocityIntervalTrend::Constant {
                        velocity_m_per_s: 1500.0,
                    },
                    control_profile_set_id: Some(String::from("profiles")),
                    control_profile_velocity_kind: Some(VelocityQuantityKind::Interval),
                    lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
                    vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
                    control_blend_weight: Some(1.0),
                    notes: Vec::new(),
                }],
                notes: Vec::new(),
            },
            control_profile_sets: vec![VelocityControlProfileSet {
                id: String::from("profiles"),
                name: String::from("Profiles"),
                derived_from: Vec::new(),
                coordinate_reference: Some(CoordinateReferenceDescriptor {
                    id: Some(String::from("EPSG:32631")),
                    name: Some(String::from("WGS 84 / UTM zone 31N")),
                    geodetic_datum: None,
                    unit: Some(String::from("metre")),
                }),
                travel_time_reference: TravelTimeReference::TwoWay,
                depth_reference: DepthReferenceKind::TrueVerticalDepth,
                profiles: vec![
                    control_profile("p00", 0.0, 0.0, 1000.0),
                    control_profile("p01", 0.0, 25.0, 2000.0),
                    control_profile("p10", 25.0, 0.0, 1500.0),
                    control_profile("p11", 25.0, 25.0, 2500.0),
                ],
                notes: Vec::new(),
            }],
            output_id: Some(String::from("built-transform")),
            output_name: Some(String::from("Built Transform")),
            preferred_velocity_kind: Some(VelocityQuantityKind::Interval),
            output_depth_unit: String::from("m"),
            notes: Vec::new(),
        };

        let descriptor =
            build_survey_time_depth_transform(&request).expect("build transform from profiles");
        assert_eq!(descriptor.id, "built-transform");
        assert_eq!(
            descriptor.source_kind,
            TimeDepthTransformSourceKind::VelocityGrid3D
        );

        let inline0 = section_time_depth_transform_slice(
            &store_root,
            "built-transform",
            SectionAxis::Inline,
            0,
        )
        .expect("slice inline 0");
        assert_f32_slice_close(&inline0.trace_depths_m[0], &[0.0, 5.0, 10.0, 15.0], 1e-4);
        assert_f32_slice_close(&inline0.trace_depths_m[1], &[0.0, 10.0, 20.0, 30.0], 1e-4);

        let inline1 = section_time_depth_transform_slice(
            &store_root,
            "built-transform",
            SectionAxis::Inline,
            1,
        )
        .expect("slice inline 1");
        assert_f32_slice_close(&inline1.trace_depths_m[0], &[0.0, 7.5, 15.0, 22.5], 1e-4);
        assert_f32_slice_close(&inline1.trace_depths_m[1], &[0.0, 12.5, 25.0, 37.5], 1e-4);
    }

    #[test]
    fn builds_survey_time_depth_transform_with_horizon_top_boundary() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo-horizon.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 4,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
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
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 10.0, 20.0, 30.0],
                ),
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
            [2, 2, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 2, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let xyz_path = temp.path().join("top-horizon.xyz");
        fs::write(
            &xyz_path,
            [
                "1000 2000 10",
                "1000 2020 20",
                "1010 2000 10",
                "1010 2020 20",
            ]
            .join("\n"),
        )
        .expect("write xyz");
        let imported = import_horizon_xyzs(&store_root, &[&xyz_path], None, None, true)
            .expect("import horizon");

        let descriptor = build_survey_time_depth_transform(&BuildSurveyTimeDepthTransformRequest {
            schema_version: 2,
            store_path: store_root.to_string_lossy().into_owned(),
            model: LayeredVelocityModel {
                id: String::from("layered-model"),
                name: String::from("Layered Model"),
                derived_from: Vec::new(),
                coordinate_reference: None,
                grid_transform: None,
                vertical_domain: TimeDepthDomain::Time,
                travel_time_reference: Some(TravelTimeReference::TwoWay),
                depth_reference: Some(DepthReferenceKind::TrueVerticalDepth),
                intervals: vec![
                    LayeredVelocityInterval {
                        id: String::from("shallow"),
                        name: String::from("Shallow"),
                        top_boundary: StratigraphicBoundaryReference::SurveyTop,
                        base_boundary: StratigraphicBoundaryReference::HorizonAsset {
                            horizon_id: imported[0].id.clone(),
                        },
                        trend: VelocityIntervalTrend::Constant {
                            velocity_m_per_s: 1000.0,
                        },
                        control_profile_set_id: None,
                        control_profile_velocity_kind: None,
                        lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
                        vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
                        control_blend_weight: None,
                        notes: Vec::new(),
                    },
                    LayeredVelocityInterval {
                        id: String::from("deep"),
                        name: String::from("Deep"),
                        top_boundary: StratigraphicBoundaryReference::HorizonAsset {
                            horizon_id: imported[0].id.clone(),
                        },
                        base_boundary: StratigraphicBoundaryReference::SurveyBase,
                        trend: VelocityIntervalTrend::LinearWithTime {
                            velocity_at_top_m_per_s: 2000.0,
                            gradient_m_per_s_per_ms: 100.0,
                        },
                        control_profile_set_id: None,
                        control_profile_velocity_kind: None,
                        lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
                        vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
                        control_blend_weight: None,
                        notes: Vec::new(),
                    },
                ],
                notes: Vec::new(),
            },
            control_profile_sets: Vec::new(),
            output_id: Some(String::from("built-transform-horizon")),
            output_name: Some(String::from("Built Transform Horizon")),
            preferred_velocity_kind: Some(VelocityQuantityKind::Interval),
            output_depth_unit: String::from("m"),
            notes: Vec::new(),
        })
        .expect("build transform from horizon-bounded interval");

        assert_eq!(descriptor.id, "built-transform-horizon");

        let inline0 = section_time_depth_transform_slice(
            &store_root,
            "built-transform-horizon",
            SectionAxis::Inline,
            0,
        )
        .expect("slice inline 0");
        assert_f32_slice_close(&inline0.trace_depths_m[0], &[0.0, 7.5, 20.0, 37.5], 1e-4);
        assert_eq!(inline0.trace_validity, vec![true, true]);
    }

    #[test]
    fn builds_survey_time_depth_transform_from_paired_horizons() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo-paired.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 1,
                    samples_per_trace: 4,
                    sample_interval_us: 4_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
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
                shape: [1, 1, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0],
                    vec![200.0],
                    vec![4.0, 8.0, 12.0, 16.0],
                ),
                segy_export: None,
                coordinate_reference_binding: Some(CoordinateReferenceBinding {
                    detected: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("LOCAL:PAIR")),
                        name: Some(String::from("Local Pair Test")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    effective: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("LOCAL:PAIR")),
                        name: Some(String::from("Local Pair Test")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    source: CoordinateReferenceSource::UserOverride,
                    notes: Vec::new(),
                }),
                spatial: Some(SurveySpatialDescriptor {
                    coordinate_reference: Some(CoordinateReferenceDescriptor {
                        id: Some(String::from("LOCAL:PAIR")),
                        name: Some(String::from("Local Pair Test")),
                        geodetic_datum: None,
                        unit: Some(String::from("metre")),
                    }),
                    grid_transform: Some(SurveyGridTransform {
                        origin: ProjectedPoint2 {
                            x: 1_000.0,
                            y: 2_000.0,
                        },
                        inline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                        xline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
            },
            [1, 1, 4],
            false,
        );
        let data = Array3::<f32>::zeros((1, 1, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let twt_one_path = temp.path().join("pair_twt_01.xyz");
        let twt_two_path = temp.path().join("pair_twt_02.xyz");
        let depth_one_path = temp.path().join("pair_depth_01.xyz");
        let depth_two_path = temp.path().join("pair_depth_02.xyz");
        fs::write(&twt_one_path, "1000 2000 8\n").expect("write twt one");
        fs::write(&twt_two_path, "1000 2000 12\n").expect("write twt two");
        fs::write(&depth_one_path, "1000 2000 8\n").expect("write depth one");
        fs::write(&depth_two_path, "1000 2000 18\n").expect("write depth two");

        let imported_time = import_horizon_xyzs_with_vertical_domain(
            &store_root,
            &[&twt_one_path, &twt_two_path],
            TimeDepthDomain::Time,
            Some("ms"),
            None,
            None,
            true,
        )
        .expect("import time horizons");
        let imported_depth = import_horizon_xyzs_with_vertical_domain(
            &store_root,
            &[&depth_one_path, &depth_two_path],
            TimeDepthDomain::Depth,
            Some("m"),
            None,
            None,
            true,
        )
        .expect("import depth horizons");

        let descriptor = build_survey_time_depth_transform_from_horizon_pairs(
            &store_root,
            &imported_time
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>(),
            &imported_depth
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>(),
            Some(String::from("paired-transform")),
            Some(String::from("Paired Transform")),
            &Vec::new(),
        )
        .expect("build paired transform");

        assert_eq!(descriptor.id, "paired-transform");
        assert_eq!(
            descriptor.source_kind,
            TimeDepthTransformSourceKind::HorizonLayerModel
        );
        assert_eq!(
            descriptor.coverage.relationship,
            SpatialCoverageRelationship::Exact
        );

        let slice = section_time_depth_transform_slice(
            &store_root,
            "paired-transform",
            SectionAxis::Inline,
            0,
        )
        .expect("slice paired transform");
        assert_f32_slice_close(&slice.trace_depths_m[0], &[4.0, 8.0, 18.0, 28.0], 1e-4);
        assert_eq!(slice.trace_validity, vec![true]);
    }

    #[test]
    fn builds_stacked_survey_time_depth_transform_from_horizon_bounded_intervals() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo-stacked.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 4,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
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
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 10.0, 20.0, 30.0],
                ),
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
            [2, 2, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 2, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let xyz_path = temp.path().join("mid-horizon.xyz");
        fs::write(
            &xyz_path,
            [
                "1000 2000 20",
                "1000 2020 20",
                "1010 2000 20",
                "1010 2020 20",
            ]
            .join("\n"),
        )
        .expect("write xyz");
        let imported = import_horizon_xyzs(&store_root, &[&xyz_path], None, None, true)
            .expect("import horizon");

        let descriptor = build_survey_time_depth_transform(&BuildSurveyTimeDepthTransformRequest {
            schema_version: 2,
            store_path: store_root.to_string_lossy().into_owned(),
            model: LayeredVelocityModel {
                id: String::from("layered-model"),
                name: String::from("Layered Model"),
                derived_from: Vec::new(),
                coordinate_reference: None,
                grid_transform: None,
                vertical_domain: TimeDepthDomain::Time,
                travel_time_reference: Some(TravelTimeReference::TwoWay),
                depth_reference: Some(DepthReferenceKind::TrueVerticalDepth),
                intervals: vec![
                    LayeredVelocityInterval {
                        id: String::from("shallow"),
                        name: String::from("Shallow"),
                        top_boundary: StratigraphicBoundaryReference::SurveyTop,
                        base_boundary: StratigraphicBoundaryReference::HorizonAsset {
                            horizon_id: imported[0].id.clone(),
                        },
                        trend: VelocityIntervalTrend::Constant {
                            velocity_m_per_s: 1000.0,
                        },
                        control_profile_set_id: None,
                        control_profile_velocity_kind: None,
                        lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
                        vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
                        control_blend_weight: None,
                        notes: Vec::new(),
                    },
                    LayeredVelocityInterval {
                        id: String::from("deep"),
                        name: String::from("Deep"),
                        top_boundary: StratigraphicBoundaryReference::HorizonAsset {
                            horizon_id: imported[0].id.clone(),
                        },
                        base_boundary: StratigraphicBoundaryReference::SurveyBase,
                        trend: VelocityIntervalTrend::Constant {
                            velocity_m_per_s: 3000.0,
                        },
                        control_profile_set_id: None,
                        control_profile_velocity_kind: None,
                        lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
                        vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
                        control_blend_weight: None,
                        notes: Vec::new(),
                    },
                ],
                notes: Vec::new(),
            },
            control_profile_sets: Vec::new(),
            output_id: Some(String::from("built-transform-stacked")),
            output_name: Some(String::from("Built Transform Stacked")),
            preferred_velocity_kind: Some(VelocityQuantityKind::Interval),
            output_depth_unit: String::from("m"),
            notes: Vec::new(),
        })
        .expect("build stacked transform");

        assert_eq!(descriptor.id, "built-transform-stacked");

        let inline0 = section_time_depth_transform_slice(
            &store_root,
            "built-transform-stacked",
            SectionAxis::Inline,
            0,
        )
        .expect("slice inline 0");
        assert_f32_slice_close(&inline0.trace_depths_m[0], &[0.0, 5.0, 15.0, 30.0], 1e-4);
        assert_f32_slice_close(&inline0.trace_depths_m[1], &[0.0, 5.0, 15.0, 30.0], 1e-4);
        assert_eq!(inline0.trace_validity, vec![true, true]);
    }

    #[test]
    fn builds_survey_property_field_from_control_profiles() {
        let temp = tempdir().expect("tempdir");
        let store_root = temp.path().join("demo.tbvol");
        let manifest = TbvolManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: String::from("store-demo"),
                source: SourceIdentity {
                    source_path: std::path::PathBuf::from("demo.segy"),
                    file_size: 0,
                    trace_count: 4,
                    samples_per_trace: 4,
                    sample_interval_us: 10_000,
                    sample_format_code: 1,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(1),
                    endianness: String::from("big"),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
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
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 10.0, 20.0, 30.0],
                ),
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
                        origin: ProjectedPoint2 { x: 0.0, y: 0.0 },
                        inline_basis: ProjectedVector2 { x: 25.0, y: 0.0 },
                        xline_basis: ProjectedVector2 { x: 0.0, y: 25.0 },
                    }),
                    footprint: None,
                    availability: SurveySpatialAvailability::Available,
                    notes: Vec::new(),
                }),
                created_by: String::from("test"),
                processing_lineage: None,
            },
            [2, 2, 4],
            false,
        );
        let data = Array3::<f32>::zeros((2, 2, 4));
        create_tbvol_store(&store_root, manifest, &data, None).expect("create store");

        let field = build_survey_property_field(&BuildSurveyPropertyFieldRequest {
            schema_version: 2,
            store_path: store_root.to_string_lossy().into_owned(),
            model: LayeredVelocityModel {
                id: String::from("layered-model"),
                name: String::from("Layered Model"),
                derived_from: Vec::new(),
                coordinate_reference: None,
                grid_transform: None,
                vertical_domain: TimeDepthDomain::Time,
                travel_time_reference: Some(TravelTimeReference::TwoWay),
                depth_reference: Some(DepthReferenceKind::TrueVerticalDepth),
                intervals: vec![LayeredVelocityInterval {
                    id: String::from("main"),
                    name: String::from("Main"),
                    top_boundary: StratigraphicBoundaryReference::SurveyTop,
                    base_boundary: StratigraphicBoundaryReference::SurveyBase,
                    trend: VelocityIntervalTrend::Constant {
                        velocity_m_per_s: 1500.0,
                    },
                    control_profile_set_id: Some(String::from("profiles")),
                    control_profile_velocity_kind: Some(VelocityQuantityKind::Interval),
                    lateral_interpolation: Some(LateralInterpolationMethod::Nearest),
                    vertical_interpolation: Some(VerticalInterpolationMethod::Linear),
                    control_blend_weight: Some(1.0),
                    notes: Vec::new(),
                }],
                notes: Vec::new(),
            },
            control_profile_sets: vec![VelocityControlProfileSet {
                id: String::from("profiles"),
                name: String::from("Profiles"),
                derived_from: Vec::new(),
                coordinate_reference: Some(CoordinateReferenceDescriptor {
                    id: Some(String::from("EPSG:32631")),
                    name: Some(String::from("WGS 84 / UTM zone 31N")),
                    geodetic_datum: None,
                    unit: Some(String::from("metre")),
                }),
                travel_time_reference: TravelTimeReference::TwoWay,
                depth_reference: DepthReferenceKind::TrueVerticalDepth,
                profiles: vec![
                    control_profile("p00", 0.0, 0.0, 1000.0),
                    control_profile("p01", 0.0, 25.0, 2000.0),
                    control_profile("p10", 25.0, 0.0, 1500.0),
                    control_profile("p11", 25.0, 25.0, 2500.0),
                ],
                notes: Vec::new(),
            }],
            output_id: Some(String::from("built-field")),
            output_name: Some(String::from("Built Field")),
            property_name: String::from("velocity"),
            property_unit: String::from("m/s"),
            preferred_velocity_kind: VelocityQuantityKind::Interval,
            output_vertical_domain: TimeDepthDomain::Time,
            notes: Vec::new(),
        })
        .expect("build property field from profiles");

        assert_eq!(field.descriptor.id, "built-field");
        assert_eq!(field.descriptor.property_name, "velocity");
        assert_eq!(
            field.values_f32,
            vec![
                1000.0, 1000.0, 1000.0, 1000.0, 2000.0, 2000.0, 2000.0, 2000.0, 1500.0, 1500.0,
                1500.0, 1500.0, 2500.0, 2500.0, 2500.0, 2500.0
            ]
        );
    }

    fn control_profile(
        id: &str,
        x: f64,
        y: f64,
        interval_velocity_m_per_s: f32,
    ) -> VelocityControlProfile {
        VelocityControlProfile {
            id: id.to_string(),
            location: ProjectedPoint2 { x, y },
            wellbore_id: None,
            samples: vec![
                VelocityControlProfileSample {
                    time_ms: 0.0,
                    depth_m: Some(0.0),
                    vrms_m_per_s: None,
                    vint_m_per_s: Some(interval_velocity_m_per_s),
                    vavg_m_per_s: Some(interval_velocity_m_per_s),
                },
                VelocityControlProfileSample {
                    time_ms: 10.0,
                    depth_m: Some(interval_velocity_m_per_s * 0.005),
                    vrms_m_per_s: None,
                    vint_m_per_s: Some(interval_velocity_m_per_s),
                    vavg_m_per_s: Some(interval_velocity_m_per_s),
                },
                VelocityControlProfileSample {
                    time_ms: 20.0,
                    depth_m: Some(interval_velocity_m_per_s * 0.010),
                    vrms_m_per_s: None,
                    vint_m_per_s: Some(interval_velocity_m_per_s),
                    vavg_m_per_s: Some(interval_velocity_m_per_s),
                },
                VelocityControlProfileSample {
                    time_ms: 30.0,
                    depth_m: Some(interval_velocity_m_per_s * 0.015),
                    vrms_m_per_s: None,
                    vint_m_per_s: Some(interval_velocity_m_per_s),
                    vavg_m_per_s: Some(interval_velocity_m_per_s),
                },
            ],
            notes: Vec::new(),
        }
    }
}
