use std::path::Path;

use ophiolite_seismic::{
    ResolvedSectionDisplayView, SectionHorizonOverlayView, SectionMetadata,
    SectionScalarOverlayColorMap, SectionScalarOverlayValueRange, SectionScalarOverlayView,
    SectionTimeDepthDiagnostics, SectionTimeDepthTransformMode, SectionUnits, SectionView,
    SpatialCoverageRelationship, TimeDepthDomain, TimeDepthTransformSourceKind,
    VelocityFunctionSource, VelocityQuantityKind,
};

use crate::SectionAxis;
use crate::error::SeismicStoreError;
use crate::gather_processing::{validate_velocity_function_source, velocity_at_time_ms};
use crate::horizons::section_horizon_overlays;
use crate::store::section_view;
use crate::survey_time_depth::section_time_depth_transform_slice;

const DEPTH_UNIT_METERS: &str = "m";

pub fn depth_converted_section_view(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    velocity_model: &VelocityFunctionSource,
    velocity_kind: VelocityQuantityKind,
) -> Result<SectionView, SeismicStoreError> {
    let root = root.as_ref();
    let section = section_view(root, axis, index)?;
    match velocity_model {
        VelocityFunctionSource::VelocityAssetReference { asset_id } => {
            let sample_axis_ms = decode_f32le(&section.sample_axis_f32le)?;
            let slice = section_time_depth_transform_slice(root, asset_id, axis, index)?;
            let (converted, _) = convert_section_view_to_depth_with_trace_mappings(
                &section,
                &sample_axis_ms,
                &slice.trace_depths_m,
                &slice.trace_validity,
                Some(asset_id.as_str()),
            )?;
            Ok(converted)
        }
        _ => convert_section_view_to_depth(&section, velocity_model, velocity_kind),
    }
}

pub fn resolved_section_display_view(
    root: impl AsRef<Path>,
    axis: SectionAxis,
    index: usize,
    domain: TimeDepthDomain,
    velocity_model: Option<&VelocityFunctionSource>,
    velocity_kind: Option<VelocityQuantityKind>,
    include_velocity_overlay: bool,
) -> Result<ResolvedSectionDisplayView, SeismicStoreError> {
    let root = root.as_ref();
    let source_section = section_view(root, axis, index)?;
    let source_horizons = section_horizon_overlays(root, axis, index)?;

    match domain {
        TimeDepthDomain::Time => {
            let (scalar_overlays, time_depth_diagnostics) = match velocity_model {
                Some(VelocityFunctionSource::VelocityAssetReference { asset_id }) => {
                    let sample_axis_ms = decode_f32le(&source_section.sample_axis_f32le)?;
                    let slice = section_time_depth_transform_slice(root, asset_id, axis, index)?;
                    let scalar_overlays = if include_velocity_overlay {
                        vec![build_time_velocity_overlay_from_transform(
                            &source_section,
                            &sample_axis_ms,
                            &slice.trace_depths_m,
                            &slice.trace_validity,
                            velocity_kind.unwrap_or(VelocityQuantityKind::Average),
                        )?]
                    } else {
                        Vec::new()
                    };
                    (
                        scalar_overlays,
                        Some(section_time_depth_diagnostics_from_transform(
                            TimeDepthDomain::Time,
                            &slice,
                            velocity_kind,
                        )),
                    )
                }
                Some(model) => {
                    let scalar_overlays = if include_velocity_overlay {
                        vec![build_time_velocity_overlay(&source_section, model)?]
                    } else {
                        Vec::new()
                    };
                    (
                        scalar_overlays,
                        Some(section_time_depth_diagnostics(
                            TimeDepthDomain::Time,
                            Some(model),
                            velocity_kind,
                        )),
                    )
                }
                None => (
                    Vec::new(),
                    Some(section_time_depth_diagnostics(
                        TimeDepthDomain::Time,
                        None,
                        None,
                    )),
                ),
            };

            Ok(ResolvedSectionDisplayView {
                section: source_section,
                time_depth_diagnostics,
                scalar_overlays,
                horizon_overlays: source_horizons,
            })
        }
        TimeDepthDomain::Depth => {
            let model = velocity_model.ok_or_else(|| {
                SeismicStoreError::Message("depth display requires a velocity model".to_string())
            })?;
            let kind = velocity_kind.ok_or_else(|| {
                SeismicStoreError::Message(
                    "depth display requires a velocity quantity kind".to_string(),
                )
            })?;
            match model {
                VelocityFunctionSource::VelocityAssetReference { asset_id } => {
                    let source_time_axis_ms = decode_f32le(&source_section.sample_axis_f32le)?;
                    let slice = section_time_depth_transform_slice(root, asset_id, axis, index)?;
                    let (converted_section, output_depth_axis_m) =
                        convert_section_view_to_depth_with_trace_mappings(
                            &source_section,
                            &source_time_axis_ms,
                            &slice.trace_depths_m,
                            &slice.trace_validity,
                            Some(asset_id.as_str()),
                        )?;
                    let scalar_overlays = if include_velocity_overlay {
                        vec![build_depth_velocity_overlay_from_transform(
                            &converted_section,
                            &source_time_axis_ms,
                            &slice.trace_depths_m,
                            &slice.trace_validity,
                            &output_depth_axis_m,
                            kind,
                        )?]
                    } else {
                        Vec::new()
                    };
                    let horizon_overlays = convert_horizon_overlays_to_depth_with_trace_mappings(
                        &source_horizons,
                        &source_time_axis_ms,
                        &slice.trace_depths_m,
                        &slice.trace_validity,
                        &output_depth_axis_m,
                    )?;

                    Ok(ResolvedSectionDisplayView {
                        section: converted_section,
                        time_depth_diagnostics: Some(
                            section_time_depth_diagnostics_from_transform(
                                TimeDepthDomain::Depth,
                                &slice,
                                Some(kind),
                            ),
                        ),
                        scalar_overlays,
                        horizon_overlays,
                    })
                }
                _ => {
                    let converted_section =
                        convert_section_view_to_depth(&source_section, model, kind)?;
                    let source_time_axis_ms = decode_f32le(&source_section.sample_axis_f32le)?;
                    let source_depths_m =
                        depth_at_input_samples(&source_time_axis_ms, model, kind)?;
                    let output_depth_axis_m = decode_f32le(&converted_section.sample_axis_f32le)?;
                    let scalar_overlays = if include_velocity_overlay {
                        vec![build_depth_velocity_overlay(
                            &converted_section,
                            model,
                            &source_time_axis_ms,
                            &source_depths_m,
                            &output_depth_axis_m,
                        )?]
                    } else {
                        Vec::new()
                    };
                    let horizon_overlays = convert_horizon_overlays_to_depth(
                        &source_horizons,
                        &source_time_axis_ms,
                        &source_depths_m,
                        &output_depth_axis_m,
                    )?;

                    Ok(ResolvedSectionDisplayView {
                        section: converted_section,
                        time_depth_diagnostics: Some(section_time_depth_diagnostics(
                            TimeDepthDomain::Depth,
                            Some(model),
                            Some(kind),
                        )),
                        scalar_overlays,
                        horizon_overlays,
                    })
                }
            }
        }
    }
}

fn section_time_depth_diagnostics(
    display_domain: TimeDepthDomain,
    velocity_model: Option<&VelocityFunctionSource>,
    velocity_kind: Option<VelocityQuantityKind>,
) -> SectionTimeDepthDiagnostics {
    match velocity_model {
        Some(model) => SectionTimeDepthDiagnostics {
            display_domain,
            transform_mode: SectionTimeDepthTransformMode::Global1d,
            source_kind: velocity_model_source_kind(model),
            velocity_kind,
            trace_varying: false,
            coverage_relationship: SpatialCoverageRelationship::Unknown,
            notes: vec![
                "Current section conversion uses one shared time-depth mapping for every trace in the section.".to_string(),
                "Lateral velocity variation, partial spatial coverage checks, and CRS-aware survey-bound 3D transforms are not applied yet.".to_string(),
            ],
        },
        None => SectionTimeDepthDiagnostics {
            display_domain,
            transform_mode: SectionTimeDepthTransformMode::None,
            source_kind: None,
            velocity_kind: None,
            trace_varying: false,
            coverage_relationship: SpatialCoverageRelationship::Unknown,
            notes: vec!["No time-depth transform is active for this section display.".to_string()],
        },
    }
}

fn velocity_model_source_kind(
    velocity_model: &VelocityFunctionSource,
) -> Option<TimeDepthTransformSourceKind> {
    Some(match velocity_model {
        VelocityFunctionSource::ConstantVelocity { .. } => {
            TimeDepthTransformSourceKind::ConstantVelocity
        }
        VelocityFunctionSource::TimeVelocityPairs { .. } => {
            TimeDepthTransformSourceKind::VelocityFunction1D
        }
        VelocityFunctionSource::VelocityAssetReference { .. } => {
            TimeDepthTransformSourceKind::VelocityGrid3D
        }
    })
}

fn section_time_depth_diagnostics_from_transform(
    display_domain: TimeDepthDomain,
    slice: &crate::survey_time_depth::SectionSurveyTimeDepthTransformSlice,
    velocity_kind: Option<VelocityQuantityKind>,
) -> SectionTimeDepthDiagnostics {
    let mut notes = vec![format!(
        "Survey-bound time-depth transform asset '{}' is driving this section display.",
        slice.descriptor.id
    )];
    match slice.coverage_relationship {
        SpatialCoverageRelationship::Exact => {
            notes.push("The transform covers every trace in the current section.".to_string())
        }
        SpatialCoverageRelationship::PartialOverlap => notes.push(
            "Only part of the current section is covered by the active transform; uncovered traces are left blank."
                .to_string(),
        ),
        SpatialCoverageRelationship::Disjoint => notes.push(
            "The active transform does not cover the current section.".to_string(),
        ),
        _ => {}
    }

    SectionTimeDepthDiagnostics {
        display_domain,
        transform_mode: SectionTimeDepthTransformMode::Survey3d,
        source_kind: Some(slice.descriptor.source_kind),
        velocity_kind,
        trace_varying: true,
        coverage_relationship: slice.coverage_relationship,
        notes,
    }
}

pub fn convert_section_view_to_depth(
    section: &SectionView,
    velocity_model: &VelocityFunctionSource,
    velocity_kind: VelocityQuantityKind,
) -> Result<SectionView, SeismicStoreError> {
    validate_velocity_function_source(velocity_model)?;

    let sample_axis_ms = decode_f32le(&section.sample_axis_f32le)?;
    if sample_axis_ms.len() != section.samples {
        return Err(SeismicStoreError::Message(format!(
            "section sample axis length mismatch: expected {}, found {}",
            section.samples,
            sample_axis_ms.len()
        )));
    }
    validate_time_axis(&sample_axis_ms)?;

    let amplitudes = decode_f32le(&section.amplitudes_f32le)?;
    let expected_amplitude_count = section.traces * section.samples;
    if amplitudes.len() != expected_amplitude_count {
        return Err(SeismicStoreError::Message(format!(
            "section amplitudes length mismatch: expected {expected_amplitude_count}, found {}",
            amplitudes.len()
        )));
    }

    let source_depths_m = depth_at_input_samples(&sample_axis_ms, velocity_model, velocity_kind)?;
    let output_depth_axis_m = regular_depth_axis(&source_depths_m, section.samples)?;

    let mut converted = Vec::with_capacity(amplitudes.len());
    for trace_index in 0..section.traces {
        let trace_start = trace_index * section.samples;
        let trace_end = trace_start + section.samples;
        let input_trace = &amplitudes[trace_start..trace_end];
        for depth_m in &output_depth_axis_m {
            let time_ms = time_at_depth(&source_depths_m, &sample_axis_ms, *depth_m)?;
            converted.push(interpolate_trace_sample(
                input_trace,
                &sample_axis_ms,
                time_ms,
            ));
        }
    }

    let mut units = section.units.clone().unwrap_or(SectionUnits {
        horizontal: None,
        sample: None,
        amplitude: None,
    });
    units.sample = Some(DEPTH_UNIT_METERS.to_string());

    let mut metadata = section.metadata.clone().unwrap_or(SectionMetadata {
        store_id: None,
        derived_from: None,
        notes: Vec::new(),
    });
    metadata
        .notes
        .retain(|note| note != "sample_domain:time" && note != "sample_domain:depth");
    metadata.notes.push("sample_domain:depth".to_string());
    metadata.notes.push(format!(
        "depth_conversion:velocity_kind={}",
        velocity_kind_slug(velocity_kind)
    ));

    Ok(SectionView {
        dataset_id: section.dataset_id.clone(),
        axis: section.axis,
        coordinate: section.coordinate.clone(),
        traces: section.traces,
        samples: section.samples,
        horizontal_axis_f64le: section.horizontal_axis_f64le.clone(),
        inline_axis_f64le: section.inline_axis_f64le.clone(),
        xline_axis_f64le: section.xline_axis_f64le.clone(),
        sample_axis_f32le: encode_f32le(&output_depth_axis_m),
        amplitudes_f32le: encode_f32le(&converted),
        units: Some(units),
        metadata: Some(metadata),
        display_defaults: section.display_defaults.clone(),
    })
}

fn convert_section_view_to_depth_with_trace_mappings(
    section: &SectionView,
    sample_axis_ms: &[f32],
    trace_depths_m: &[Vec<f32>],
    trace_validity: &[bool],
    transform_asset_id: Option<&str>,
) -> Result<(SectionView, Vec<f32>), SeismicStoreError> {
    if trace_depths_m.len() != section.traces || trace_validity.len() != section.traces {
        return Err(SeismicStoreError::Message(
            "trace-varying time-depth mappings do not match the section trace count".to_string(),
        ));
    }

    let amplitudes = decode_f32le(&section.amplitudes_f32le)?;
    let expected_amplitude_count = section.traces * section.samples;
    if amplitudes.len() != expected_amplitude_count {
        return Err(SeismicStoreError::Message(format!(
            "section amplitudes length mismatch: expected {expected_amplitude_count}, found {}",
            amplitudes.len()
        )));
    }

    let output_depth_axis_m =
        regular_depth_axis_from_trace_mappings(trace_depths_m, trace_validity, section.samples)?;
    let mut converted = Vec::with_capacity(amplitudes.len());
    for trace_index in 0..section.traces {
        let trace_start = trace_index * section.samples;
        let trace_end = trace_start + section.samples;
        let input_trace = &amplitudes[trace_start..trace_end];
        if !trace_validity[trace_index] {
            converted.resize(converted.len() + section.samples, 0.0);
            continue;
        }

        let trace_depths = &trace_depths_m[trace_index];
        if trace_depths.len() != section.samples {
            return Err(SeismicStoreError::Message(
                "trace-varying time-depth mapping sample count does not match the section sample axis"
                    .to_string(),
            ));
        }
        for depth_m in &output_depth_axis_m {
            let time_ms = time_at_depth(trace_depths, sample_axis_ms, *depth_m)?;
            converted.push(interpolate_trace_sample(
                input_trace,
                sample_axis_ms,
                time_ms,
            ));
        }
    }

    let mut units = section.units.clone().unwrap_or(SectionUnits {
        horizontal: None,
        sample: None,
        amplitude: None,
    });
    units.sample = Some(DEPTH_UNIT_METERS.to_string());

    let mut metadata = section.metadata.clone().unwrap_or(SectionMetadata {
        store_id: None,
        derived_from: None,
        notes: Vec::new(),
    });
    metadata
        .notes
        .retain(|note| note != "sample_domain:time" && note != "sample_domain:depth");
    metadata.notes.push("sample_domain:depth".to_string());
    if let Some(transform_asset_id) = transform_asset_id {
        metadata
            .notes
            .push(format!("time_depth_transform_asset:{transform_asset_id}"));
    }

    Ok((
        SectionView {
            dataset_id: section.dataset_id.clone(),
            axis: section.axis,
            coordinate: section.coordinate.clone(),
            traces: section.traces,
            samples: section.samples,
            horizontal_axis_f64le: section.horizontal_axis_f64le.clone(),
            inline_axis_f64le: section.inline_axis_f64le.clone(),
            xline_axis_f64le: section.xline_axis_f64le.clone(),
            sample_axis_f32le: encode_f32le(&output_depth_axis_m),
            amplitudes_f32le: encode_f32le(&converted),
            units: Some(units),
            metadata: Some(metadata),
            display_defaults: section.display_defaults.clone(),
        },
        output_depth_axis_m,
    ))
}

fn validate_time_axis(sample_axis_ms: &[f32]) -> Result<(), SeismicStoreError> {
    if sample_axis_ms.is_empty() {
        return Err(SeismicStoreError::Message(
            "section sample axis must not be empty".to_string(),
        ));
    }
    let mut previous = None;
    for (index, value) in sample_axis_ms.iter().copied().enumerate() {
        if !value.is_finite() || value < 0.0 {
            return Err(SeismicStoreError::Message(format!(
                "section sample axis value at index {index} must be finite and >= 0 ms, found {value}"
            )));
        }
        if let Some(previous) = previous
            && value < previous
        {
            return Err(SeismicStoreError::Message(format!(
                "section sample axis must be nondecreasing, found {value} after {previous}"
            )));
        }
        previous = Some(value);
    }
    Ok(())
}

fn depth_at_input_samples(
    sample_axis_ms: &[f32],
    velocity_model: &VelocityFunctionSource,
    velocity_kind: VelocityQuantityKind,
) -> Result<Vec<f32>, SeismicStoreError> {
    let mut depths_m = Vec::with_capacity(sample_axis_ms.len());
    match velocity_kind {
        VelocityQuantityKind::Average => {
            for time_ms in sample_axis_ms {
                let velocity_m_per_s = velocity_at_time_ms(velocity_model, *time_ms)?;
                depths_m.push(time_ms * 0.001 * velocity_m_per_s * 0.5);
            }
        }
        VelocityQuantityKind::Interval => {
            for (index, time_ms) in sample_axis_ms.iter().copied().enumerate() {
                let velocity_m_per_s = velocity_at_time_ms(velocity_model, time_ms)?;
                if index == 0 {
                    depths_m.push(time_ms * 0.001 * velocity_m_per_s * 0.5);
                    continue;
                }

                let previous_time_ms = sample_axis_ms[index - 1];
                let previous_velocity_m_per_s =
                    velocity_at_time_ms(velocity_model, previous_time_ms)?;
                let delta_time_s = (time_ms - previous_time_ms) * 0.001;
                let previous_depth_m = *depths_m
                    .last()
                    .expect("depth list should contain previous sample");
                let delta_depth_m =
                    delta_time_s * (previous_velocity_m_per_s + velocity_m_per_s) * 0.25;
                depths_m.push(previous_depth_m + delta_depth_m);
            }
        }
        VelocityQuantityKind::Rms => {
            return Err(SeismicStoreError::Message(
                "RMS velocity is not supported yet for section time-depth conversion; normalize it first into average or interval velocity".to_string(),
            ));
        }
    }

    if depths_m.len() >= 2 {
        let first = depths_m[0];
        let last = depths_m[depths_m.len() - 1];
        if !last.is_finite() || last < first {
            return Err(SeismicStoreError::Message(
                "computed depth mapping must be finite and nondecreasing".to_string(),
            ));
        }
    }

    Ok(depths_m)
}

fn regular_depth_axis(
    depths_m: &[f32],
    sample_count: usize,
) -> Result<Vec<f32>, SeismicStoreError> {
    if depths_m.is_empty() || sample_count == 0 {
        return Err(SeismicStoreError::Message(
            "depth conversion requires at least one sample".to_string(),
        ));
    }
    if sample_count == 1 {
        return Ok(vec![depths_m[0]]);
    }
    let first = depths_m[0];
    let last = depths_m[depths_m.len() - 1];
    let step = (last - first) / (sample_count - 1) as f32;
    if !step.is_finite() || step <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "depth conversion produced a non-positive regular sample step: {step}"
        )));
    }
    Ok((0..sample_count)
        .map(|index| first + step * index as f32)
        .collect())
}

fn regular_depth_axis_from_trace_mappings(
    trace_depths_m: &[Vec<f32>],
    trace_validity: &[bool],
    sample_count: usize,
) -> Result<Vec<f32>, SeismicStoreError> {
    if sample_count == 0 {
        return Err(SeismicStoreError::Message(
            "depth conversion requires at least one sample".to_string(),
        ));
    }
    let mut first = f32::INFINITY;
    let mut last = f32::NEG_INFINITY;
    let mut valid_trace_count = 0usize;
    for (trace_depths, valid) in trace_depths_m.iter().zip(trace_validity.iter()) {
        if !*valid || trace_depths.is_empty() {
            continue;
        }
        first = first.min(trace_depths[0]);
        last = last.max(trace_depths[trace_depths.len() - 1]);
        valid_trace_count += 1;
    }
    if valid_trace_count == 0 {
        return Err(SeismicStoreError::Message(
            "active survey time-depth transform does not cover this section".to_string(),
        ));
    }
    if sample_count == 1 {
        return Ok(vec![first]);
    }
    let step = (last - first) / (sample_count - 1) as f32;
    if !step.is_finite() || step <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "trace-varying depth conversion produced a non-positive regular sample step: {step}"
        )));
    }
    Ok((0..sample_count)
        .map(|index| first + step * index as f32)
        .collect())
}

fn time_at_depth(
    depths_m: &[f32],
    sample_axis_ms: &[f32],
    target_depth_m: f32,
) -> Result<f32, SeismicStoreError> {
    if depths_m.len() != sample_axis_ms.len() || depths_m.is_empty() {
        return Err(SeismicStoreError::Message(
            "depth mapping and sample axis lengths must match".to_string(),
        ));
    }
    if target_depth_m <= depths_m[0] {
        return Ok(sample_axis_ms[0]);
    }
    let last_index = depths_m.len() - 1;
    if target_depth_m >= depths_m[last_index] {
        return Ok(sample_axis_ms[last_index]);
    }

    let mut upper_index = 0usize;
    while upper_index < depths_m.len() && depths_m[upper_index] < target_depth_m {
        upper_index += 1;
    }

    if upper_index == 0 {
        return Ok(sample_axis_ms[0]);
    }
    if upper_index >= depths_m.len() {
        return Ok(sample_axis_ms[last_index]);
    }

    let lower_index = upper_index - 1;
    let lower_depth = depths_m[lower_index];
    let upper_depth = depths_m[upper_index];
    if (upper_depth - lower_depth).abs() <= f32::EPSILON {
        return Ok(sample_axis_ms[upper_index]);
    }

    let t = ((target_depth_m - lower_depth) / (upper_depth - lower_depth)).clamp(0.0, 1.0);
    let lower_time = sample_axis_ms[lower_index];
    let upper_time = sample_axis_ms[upper_index];
    Ok(lower_time + (upper_time - lower_time) * t)
}

fn depth_at_time(
    depths_m: &[f32],
    sample_axis_ms: &[f32],
    target_time_ms: f32,
) -> Result<f32, SeismicStoreError> {
    if depths_m.len() != sample_axis_ms.len() || depths_m.is_empty() {
        return Err(SeismicStoreError::Message(
            "depth mapping and sample axis lengths must match".to_string(),
        ));
    }
    if target_time_ms <= sample_axis_ms[0] {
        return Ok(depths_m[0]);
    }
    let last_index = sample_axis_ms.len() - 1;
    if target_time_ms >= sample_axis_ms[last_index] {
        return Ok(depths_m[last_index]);
    }

    let mut upper_index = 0usize;
    while upper_index < sample_axis_ms.len() && sample_axis_ms[upper_index] < target_time_ms {
        upper_index += 1;
    }

    if upper_index == 0 {
        return Ok(depths_m[0]);
    }
    if upper_index >= sample_axis_ms.len() {
        return Ok(depths_m[last_index]);
    }

    let lower_index = upper_index - 1;
    let lower_time = sample_axis_ms[lower_index];
    let upper_time = sample_axis_ms[upper_index];
    if (upper_time - lower_time).abs() <= f32::EPSILON {
        return Ok(depths_m[upper_index]);
    }

    let t = ((target_time_ms - lower_time) / (upper_time - lower_time)).clamp(0.0, 1.0);
    let lower_depth = depths_m[lower_index];
    let upper_depth = depths_m[upper_index];
    Ok(lower_depth + (upper_depth - lower_depth) * t)
}

fn interpolate_trace_sample(trace: &[f32], sample_axis_ms: &[f32], target_time_ms: f32) -> f32 {
    if trace.is_empty() || sample_axis_ms.is_empty() || target_time_ms.is_nan() {
        return 0.0;
    }
    if target_time_ms <= sample_axis_ms[0] {
        return trace[0];
    }
    let last_index = trace.len() - 1;
    if target_time_ms >= sample_axis_ms[last_index] {
        return trace[last_index];
    }

    let mut upper_index = 0usize;
    while upper_index < sample_axis_ms.len() && sample_axis_ms[upper_index] < target_time_ms {
        upper_index += 1;
    }

    if upper_index == 0 {
        return trace[0];
    }
    if upper_index >= sample_axis_ms.len() {
        return trace[last_index];
    }

    let lower_index = upper_index - 1;
    let lower_time = sample_axis_ms[lower_index];
    let upper_time = sample_axis_ms[upper_index];
    if (upper_time - lower_time).abs() <= f32::EPSILON {
        return trace[upper_index];
    }

    let t = ((target_time_ms - lower_time) / (upper_time - lower_time)).clamp(0.0, 1.0);
    trace[lower_index] * (1.0 - t) + trace[upper_index] * t
}

fn decode_f32le(bytes: &[u8]) -> Result<Vec<f32>, SeismicStoreError> {
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "expected f32 little-endian bytes, found length {}",
            bytes.len()
        )));
    }
    Ok(bytes
        .chunks_exact(std::mem::size_of::<f32>())
        .map(|chunk| {
            let array = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_le_bytes(array)
        })
        .collect())
}

fn encode_f32le(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(values));
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn velocity_kind_slug(kind: VelocityQuantityKind) -> &'static str {
    match kind {
        VelocityQuantityKind::Interval => "interval",
        VelocityQuantityKind::Rms => "rms",
        VelocityQuantityKind::Average => "average",
    }
}

fn build_time_velocity_overlay(
    section: &SectionView,
    velocity_model: &VelocityFunctionSource,
) -> Result<SectionScalarOverlayView, SeismicStoreError> {
    validate_velocity_function_source(velocity_model)?;
    let sample_axis_ms = decode_f32le(&section.sample_axis_f32le)?;
    let per_sample = build_velocity_samples_for_time_axis(&sample_axis_ms, velocity_model)?;
    let values = tile_overlay_per_trace(section.traces, &per_sample);

    Ok(SectionScalarOverlayView {
        id: "velocity-model".to_string(),
        name: Some("Velocity Model".to_string()),
        width: section.traces,
        height: section.samples,
        values_f32le: encode_f32le(&values),
        color_map: SectionScalarOverlayColorMap::Turbo,
        opacity: 0.52,
        value_range: overlay_value_range(&per_sample),
        units: Some("m/s".to_string()),
    })
}

fn build_time_velocity_overlay_from_transform(
    section: &SectionView,
    sample_axis_ms: &[f32],
    trace_depths_m: &[Vec<f32>],
    trace_validity: &[bool],
    velocity_kind: VelocityQuantityKind,
) -> Result<SectionScalarOverlayView, SeismicStoreError> {
    let mut values = Vec::with_capacity(section.traces * section.samples);
    for trace_index in 0..section.traces {
        if !trace_validity[trace_index] {
            values.resize(values.len() + section.samples, f32::NAN);
            continue;
        }
        let per_trace = velocity_trace_from_depth_mapping(
            sample_axis_ms,
            &trace_depths_m[trace_index],
            velocity_kind,
        )?;
        values.extend_from_slice(&per_trace);
    }

    Ok(SectionScalarOverlayView {
        id: "velocity-model".to_string(),
        name: Some("Velocity Model".to_string()),
        width: section.traces,
        height: section.samples,
        values_f32le: encode_f32le(&values),
        color_map: SectionScalarOverlayColorMap::Turbo,
        opacity: 0.52,
        value_range: overlay_value_range(&values),
        units: Some("m/s".to_string()),
    })
}

fn build_depth_velocity_overlay(
    section: &SectionView,
    velocity_model: &VelocityFunctionSource,
    source_time_axis_ms: &[f32],
    source_depths_m: &[f32],
    output_depth_axis_m: &[f32],
) -> Result<SectionScalarOverlayView, SeismicStoreError> {
    validate_velocity_function_source(velocity_model)?;
    let mut per_sample = Vec::with_capacity(output_depth_axis_m.len());
    for depth_m in output_depth_axis_m {
        let time_ms = time_at_depth(source_depths_m, source_time_axis_ms, *depth_m)?;
        per_sample.push(velocity_at_time_ms(velocity_model, time_ms)?);
    }
    let values = tile_overlay_per_trace(section.traces, &per_sample);

    Ok(SectionScalarOverlayView {
        id: "velocity-model".to_string(),
        name: Some("Velocity Model".to_string()),
        width: section.traces,
        height: section.samples,
        values_f32le: encode_f32le(&values),
        color_map: SectionScalarOverlayColorMap::Turbo,
        opacity: 0.52,
        value_range: overlay_value_range(&per_sample),
        units: Some("m/s".to_string()),
    })
}

fn build_depth_velocity_overlay_from_transform(
    section: &SectionView,
    source_time_axis_ms: &[f32],
    trace_depths_m: &[Vec<f32>],
    trace_validity: &[bool],
    output_depth_axis_m: &[f32],
    velocity_kind: VelocityQuantityKind,
) -> Result<SectionScalarOverlayView, SeismicStoreError> {
    let mut values = Vec::with_capacity(section.traces * section.samples);
    for trace_index in 0..section.traces {
        if !trace_validity[trace_index] {
            values.resize(values.len() + section.samples, f32::NAN);
            continue;
        }
        let per_trace = velocity_trace_for_depth_axis(
            output_depth_axis_m,
            source_time_axis_ms,
            &trace_depths_m[trace_index],
            velocity_kind,
        )?;
        values.extend_from_slice(&per_trace);
    }

    Ok(SectionScalarOverlayView {
        id: "velocity-model".to_string(),
        name: Some("Velocity Model".to_string()),
        width: section.traces,
        height: section.samples,
        values_f32le: encode_f32le(&values),
        color_map: SectionScalarOverlayColorMap::Turbo,
        opacity: 0.52,
        value_range: overlay_value_range(&values),
        units: Some("m/s".to_string()),
    })
}

fn convert_horizon_overlays_to_depth(
    overlays: &[SectionHorizonOverlayView],
    source_time_axis_ms: &[f32],
    source_depths_m: &[f32],
    output_depth_axis_m: &[f32],
) -> Result<Vec<SectionHorizonOverlayView>, SeismicStoreError> {
    overlays
        .iter()
        .map(|overlay| {
            let samples = overlay
                .samples
                .iter()
                .filter_map(|sample| {
                    let source_time_ms = sample
                        .sample_value
                        .or_else(|| source_time_axis_ms.get(sample.sample_index).copied())?;
                    let depth_m =
                        depth_at_time(source_depths_m, source_time_axis_ms, source_time_ms).ok()?;
                    let sample_index = nearest_sample_index(output_depth_axis_m, depth_m)?;
                    Some(ophiolite_seismic::SectionHorizonSample {
                        trace_index: sample.trace_index,
                        sample_index,
                        sample_value: Some(depth_m),
                    })
                })
                .collect();

            Ok(SectionHorizonOverlayView {
                id: overlay.id.clone(),
                name: overlay.name.clone(),
                style: overlay.style.clone(),
                samples,
            })
        })
        .collect()
}

fn convert_horizon_overlays_to_depth_with_trace_mappings(
    overlays: &[SectionHorizonOverlayView],
    source_time_axis_ms: &[f32],
    trace_depths_m: &[Vec<f32>],
    trace_validity: &[bool],
    output_depth_axis_m: &[f32],
) -> Result<Vec<SectionHorizonOverlayView>, SeismicStoreError> {
    overlays
        .iter()
        .map(|overlay| {
            let samples = overlay
                .samples
                .iter()
                .filter_map(|sample| {
                    let trace_index = sample.trace_index;
                    if trace_index >= trace_depths_m.len() || !trace_validity[trace_index] {
                        return None;
                    }
                    let source_time_ms = sample
                        .sample_value
                        .or_else(|| source_time_axis_ms.get(sample.sample_index).copied())?;
                    let depth_m = depth_at_time(
                        &trace_depths_m[trace_index],
                        source_time_axis_ms,
                        source_time_ms,
                    )
                    .ok()?;
                    let sample_index = nearest_sample_index(output_depth_axis_m, depth_m)?;
                    Some(ophiolite_seismic::SectionHorizonSample {
                        trace_index,
                        sample_index,
                        sample_value: Some(depth_m),
                    })
                })
                .collect();

            Ok(SectionHorizonOverlayView {
                id: overlay.id.clone(),
                name: overlay.name.clone(),
                style: overlay.style.clone(),
                samples,
            })
        })
        .collect()
}

fn build_velocity_samples_for_time_axis(
    sample_axis_ms: &[f32],
    velocity_model: &VelocityFunctionSource,
) -> Result<Vec<f32>, SeismicStoreError> {
    sample_axis_ms
        .iter()
        .map(|time_ms| velocity_at_time_ms(velocity_model, *time_ms))
        .collect()
}

fn tile_overlay_per_trace(trace_count: usize, per_sample: &[f32]) -> Vec<f32> {
    let mut values = Vec::with_capacity(trace_count * per_sample.len());
    for _ in 0..trace_count {
        values.extend_from_slice(per_sample);
    }
    values
}

fn overlay_value_range(values: &[f32]) -> SectionScalarOverlayValueRange {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for value in values {
        if !value.is_finite() {
            continue;
        }
        min = min.min(*value);
        max = max.max(*value);
    }
    if !min.is_finite() || !max.is_finite() || (max - min).abs() <= f32::EPSILON {
        return SectionScalarOverlayValueRange { min: 0.0, max: 1.0 };
    }
    SectionScalarOverlayValueRange { min, max }
}

fn velocity_trace_from_depth_mapping(
    sample_axis_ms: &[f32],
    trace_depths_m: &[f32],
    velocity_kind: VelocityQuantityKind,
) -> Result<Vec<f32>, SeismicStoreError> {
    if sample_axis_ms.len() != trace_depths_m.len() {
        return Err(SeismicStoreError::Message(
            "time-depth trace mapping length mismatch".to_string(),
        ));
    }
    match velocity_kind {
        VelocityQuantityKind::Average => Ok(sample_axis_ms
            .iter()
            .zip(trace_depths_m.iter())
            .map(|(time_ms, depth_m)| average_velocity_from_time_depth(*time_ms, *depth_m))
            .collect()),
        VelocityQuantityKind::Interval => {
            if trace_depths_m.len() == 1 {
                return Ok(vec![f32::NAN]);
            }
            let mut values = vec![f32::NAN; trace_depths_m.len()];
            for sample_index in 1..trace_depths_m.len() {
                values[sample_index] = interval_velocity_from_time_depth_samples(
                    sample_axis_ms[sample_index - 1],
                    sample_axis_ms[sample_index],
                    trace_depths_m[sample_index - 1],
                    trace_depths_m[sample_index],
                );
            }
            values[0] = values[1];
            Ok(values)
        }
        VelocityQuantityKind::Rms => Err(SeismicStoreError::Message(
            "RMS velocity overlays are not derivable from a normalized survey time-depth transform"
                .to_string(),
        )),
    }
}

fn velocity_trace_for_depth_axis(
    output_depth_axis_m: &[f32],
    source_time_axis_ms: &[f32],
    trace_depths_m: &[f32],
    velocity_kind: VelocityQuantityKind,
) -> Result<Vec<f32>, SeismicStoreError> {
    let times_ms = output_depth_axis_m
        .iter()
        .map(|depth_m| time_at_depth(trace_depths_m, source_time_axis_ms, *depth_m))
        .collect::<Result<Vec<_>, _>>()?;
    match velocity_kind {
        VelocityQuantityKind::Average => Ok(output_depth_axis_m
            .iter()
            .zip(times_ms.iter())
            .map(|(depth_m, time_ms)| average_velocity_from_time_depth(*time_ms, *depth_m))
            .collect()),
        VelocityQuantityKind::Interval => {
            if output_depth_axis_m.len() == 1 {
                return Ok(vec![f32::NAN]);
            }
            let mut values = vec![f32::NAN; output_depth_axis_m.len()];
            for sample_index in 1..output_depth_axis_m.len() {
                values[sample_index] = interval_velocity_from_time_depth_samples(
                    times_ms[sample_index - 1],
                    times_ms[sample_index],
                    output_depth_axis_m[sample_index - 1],
                    output_depth_axis_m[sample_index],
                );
            }
            values[0] = values[1];
            Ok(values)
        }
        VelocityQuantityKind::Rms => Err(SeismicStoreError::Message(
            "RMS velocity overlays are not derivable from a normalized survey time-depth transform"
                .to_string(),
        )),
    }
}

fn average_velocity_from_time_depth(time_ms: f32, depth_m: f32) -> f32 {
    if !time_ms.is_finite() || !depth_m.is_finite() || time_ms <= f32::EPSILON {
        return f32::NAN;
    }
    depth_m * 2000.0 / time_ms
}

fn interval_velocity_from_time_depth_samples(
    previous_time_ms: f32,
    current_time_ms: f32,
    previous_depth_m: f32,
    current_depth_m: f32,
) -> f32 {
    let delta_time_s = (current_time_ms - previous_time_ms) * 0.001;
    if !delta_time_s.is_finite() || delta_time_s <= f32::EPSILON {
        return f32::NAN;
    }
    2.0 * (current_depth_m - previous_depth_m) / delta_time_s
}

fn nearest_sample_index(axis: &[f32], value: f32) -> Option<usize> {
    if axis.is_empty() || !value.is_finite() {
        return None;
    }
    if value <= axis[0] {
        return Some(0);
    }
    let last_index = axis.len() - 1;
    if value >= axis[last_index] {
        return Some(last_index);
    }

    let mut upper_index = 0usize;
    while upper_index < axis.len() && axis[upper_index] < value {
        upper_index += 1;
    }

    if upper_index == 0 {
        return Some(0);
    }
    if upper_index >= axis.len() {
        return Some(last_index);
    }

    let lower_index = upper_index - 1;
    let lower_distance = (value - axis[lower_index]).abs();
    let upper_distance = (axis[upper_index] - value).abs();
    Some(if lower_distance <= upper_distance {
        lower_index
    } else {
        upper_index
    })
}

#[cfg(test)]
mod tests {
    use ophiolite_seismic::{DatasetId, SectionAxis, SectionCoordinate};

    use super::*;

    #[test]
    fn constant_average_velocity_depth_conversion_resamples_section() {
        let section = SectionView {
            dataset_id: DatasetId("mock".to_string()),
            axis: SectionAxis::Inline,
            coordinate: SectionCoordinate {
                index: 0,
                value: 111.0,
            },
            traces: 2,
            samples: 4,
            horizontal_axis_f64le: Vec::new(),
            inline_axis_f64le: None,
            xline_axis_f64le: None,
            sample_axis_f32le: encode_f32le(&[0.0, 4.0, 8.0, 12.0]),
            amplitudes_f32le: encode_f32le(&[1.0, 2.0, 3.0, 4.0, 10.0, 20.0, 30.0, 40.0]),
            units: Some(SectionUnits {
                horizontal: Some("xline".to_string()),
                sample: Some("ms".to_string()),
                amplitude: Some("arb".to_string()),
            }),
            metadata: Some(SectionMetadata {
                store_id: Some("store-1".to_string()),
                derived_from: None,
                notes: vec!["sample_domain:time".to_string()],
            }),
            display_defaults: None,
        };

        let converted = convert_section_view_to_depth(
            &section,
            &VelocityFunctionSource::ConstantVelocity {
                velocity_m_per_s: 3000.0,
            },
            VelocityQuantityKind::Average,
        )
        .expect("convert section");

        let sample_axis_m = decode_f32le(&converted.sample_axis_f32le).expect("decode depth axis");
        assert_eq!(sample_axis_m, vec![0.0, 6.0, 12.0, 18.0]);
        let amplitudes = decode_f32le(&converted.amplitudes_f32le).expect("decode amplitudes");
        assert_eq!(amplitudes, vec![1.0, 2.0, 3.0, 4.0, 10.0, 20.0, 30.0, 40.0]);
        assert_eq!(
            converted
                .units
                .as_ref()
                .and_then(|units| units.sample.as_deref()),
            Some(DEPTH_UNIT_METERS)
        );
        assert!(
            converted
                .metadata
                .as_ref()
                .expect("metadata")
                .notes
                .iter()
                .any(|note| note == "sample_domain:depth")
        );
    }

    #[test]
    fn rms_velocity_is_rejected_for_now() {
        let section = SectionView {
            dataset_id: DatasetId("mock".to_string()),
            axis: SectionAxis::Inline,
            coordinate: SectionCoordinate {
                index: 0,
                value: 111.0,
            },
            traces: 1,
            samples: 2,
            horizontal_axis_f64le: Vec::new(),
            inline_axis_f64le: None,
            xline_axis_f64le: None,
            sample_axis_f32le: encode_f32le(&[0.0, 4.0]),
            amplitudes_f32le: encode_f32le(&[1.0, 2.0]),
            units: None,
            metadata: None,
            display_defaults: None,
        };

        let error = convert_section_view_to_depth(
            &section,
            &VelocityFunctionSource::ConstantVelocity {
                velocity_m_per_s: 2500.0,
            },
            VelocityQuantityKind::Rms,
        )
        .expect_err("RMS conversion should fail");
        assert!(error.to_string().contains("RMS velocity"));
    }
}
