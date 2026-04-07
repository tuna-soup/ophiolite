use crate::compute;
use crate::{
    GatherAxisKind, GatherInterpolationMode, GatherProcessingOperation, GatherProcessingPipeline,
    GatherSampleDomain, SeismicLayout, SeismicStoreError, TraceLocalProcessingPipeline,
    VelocityFunctionSource,
};

const MIN_SAMPLE_AXIS_LEN: usize = 2;
const MIN_VELOCITY_M_PER_S: f32 = 1.0;
const MAX_STRETCH_RATIO: f32 = 100.0;
const TIME_EPSILON_MS: f32 = 1.0e-3;

#[derive(Debug, Clone, PartialEq)]
pub struct GatherPlane {
    pub label: String,
    pub gather_axis_kind: GatherAxisKind,
    pub sample_domain: GatherSampleDomain,
    pub traces: usize,
    pub samples: usize,
    pub horizontal_axis: Vec<f64>,
    pub sample_axis_ms: Vec<f32>,
    pub amplitudes: Vec<f32>,
}

pub fn validate_gather_processing_pipeline(
    pipeline: &GatherProcessingPipeline,
) -> Result<(), SeismicStoreError> {
    validate_gather_processing_pipeline_for_layout(pipeline, SeismicLayout::PreStack3DOffset)
}

pub fn validate_gather_processing_pipeline_for_layout(
    pipeline: &GatherProcessingPipeline,
    layout: SeismicLayout,
) -> Result<(), SeismicStoreError> {
    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        compute::validate_processing_pipeline_for_layout(trace_local_pipeline, layout)?;
    }

    if pipeline.operations.is_empty() {
        return Err(SeismicStoreError::Message(
            "gather processing pipeline must contain at least one gather operator".to_string(),
        ));
    }

    for operation in &pipeline.operations {
        let compatibility = operation.compatibility();
        if !compatibility.supports_layout(layout) {
            return Err(SeismicStoreError::Message(format!(
                "gather processing operator '{}' requires {}, found layout {:?}",
                operation.operator_id(),
                compatibility.label(),
                layout
            )));
        }

        match operation {
            GatherProcessingOperation::NmoCorrection { velocity_model, .. } => {
                validate_velocity_function_source(velocity_model)?;
            }
            GatherProcessingOperation::StretchMute {
                velocity_model,
                max_stretch_ratio,
            } => {
                validate_velocity_function_source(velocity_model)?;
                validate_max_stretch_ratio(*max_stretch_ratio)?;
            }
            GatherProcessingOperation::OffsetMute {
                min_offset,
                max_offset,
            } => {
                validate_offset_range(*min_offset, *max_offset)?;
            }
        }
    }

    Ok(())
}

pub fn apply_gather_processing_pipeline(
    gather: &mut GatherPlane,
    pipeline: &GatherProcessingPipeline,
) -> Result<(), SeismicStoreError> {
    validate_gather_processing_pipeline(pipeline)?;
    validate_offset_gather(gather)?;

    if let Some(trace_local_pipeline) = pipeline.trace_local_pipeline.as_ref() {
        apply_trace_local_pipeline_to_gather(gather, trace_local_pipeline)?;
    }

    let sample_interval_ms = sample_interval_ms_from_axis(&gather.sample_axis_ms)?;
    for operation in &pipeline.operations {
        apply_gather_processing_operation(gather, sample_interval_ms, operation)?;
    }

    Ok(())
}

pub fn apply_trace_local_pipeline_to_gather(
    gather: &mut GatherPlane,
    pipeline: &TraceLocalProcessingPipeline,
) -> Result<(), SeismicStoreError> {
    validate_offset_gather(gather)?;
    let sample_interval_ms = sample_interval_ms_from_axis(&gather.sample_axis_ms)?;
    compute::apply_pipeline_to_traces(
        &mut gather.amplitudes,
        gather.traces,
        gather.samples,
        sample_interval_ms,
        None,
        &pipeline.operations,
    )
}

fn apply_gather_processing_operation(
    gather: &mut GatherPlane,
    sample_interval_ms: f32,
    operation: &GatherProcessingOperation,
) -> Result<(), SeismicStoreError> {
    match operation {
        GatherProcessingOperation::NmoCorrection {
            velocity_model,
            interpolation,
        } => apply_nmo_correction(gather, sample_interval_ms, velocity_model, *interpolation),
        GatherProcessingOperation::StretchMute {
            velocity_model,
            max_stretch_ratio,
        } => apply_stretch_mute(gather, velocity_model, *max_stretch_ratio),
        GatherProcessingOperation::OffsetMute {
            min_offset,
            max_offset,
        } => apply_offset_mute(gather, *min_offset, *max_offset),
    }
}

fn apply_nmo_correction(
    gather: &mut GatherPlane,
    sample_interval_ms: f32,
    velocity_model: &VelocityFunctionSource,
    interpolation: GatherInterpolationMode,
) -> Result<(), SeismicStoreError> {
    let original = gather.amplitudes.clone();
    for (trace_index, offset) in gather.horizontal_axis.iter().copied().enumerate() {
        let trace_start = trace_index * gather.samples;
        let trace_end = trace_start + gather.samples;
        let source_trace = &original[trace_start..trace_end];
        let target_trace = &mut gather.amplitudes[trace_start..trace_end];

        for (sample_index, output) in target_trace.iter_mut().enumerate() {
            let zero_offset_time_ms = gather.sample_axis_ms[sample_index];
            if zero_offset_time_ms <= 0.0 {
                *output = 0.0;
                continue;
            }

            let input_time_ms =
                nmo_input_time_ms(zero_offset_time_ms, offset as f32, velocity_model)?;
            *output = interpolate_trace_sample(
                source_trace,
                input_time_ms,
                sample_interval_ms,
                interpolation,
            );
        }
    }

    Ok(())
}

fn apply_stretch_mute(
    gather: &mut GatherPlane,
    velocity_model: &VelocityFunctionSource,
    max_stretch_ratio: f32,
) -> Result<(), SeismicStoreError> {
    validate_max_stretch_ratio(max_stretch_ratio)?;

    for (trace_index, offset) in gather.horizontal_axis.iter().copied().enumerate() {
        let trace_start = trace_index * gather.samples;
        let trace_end = trace_start + gather.samples;
        let trace = &mut gather.amplitudes[trace_start..trace_end];

        for (sample_index, sample) in trace.iter_mut().enumerate() {
            let zero_offset_time_ms = gather.sample_axis_ms[sample_index];
            if zero_offset_time_ms <= 0.0 {
                *sample = 0.0;
                continue;
            }

            let input_time_ms =
                nmo_input_time_ms(zero_offset_time_ms, offset as f32, velocity_model)?;
            let stretch_ratio =
                ((input_time_ms - zero_offset_time_ms) / zero_offset_time_ms.max(TIME_EPSILON_MS))
                    .max(0.0);
            if stretch_ratio > max_stretch_ratio {
                *sample = 0.0;
            }
        }
    }

    Ok(())
}

fn apply_offset_mute(
    gather: &mut GatherPlane,
    min_offset: Option<f32>,
    max_offset: Option<f32>,
) -> Result<(), SeismicStoreError> {
    validate_offset_range(min_offset, max_offset)?;

    for (trace_index, offset) in gather.horizontal_axis.iter().copied().enumerate() {
        let offset = offset as f32;
        let keep_min = min_offset.is_none_or(|min| offset >= min);
        let keep_max = max_offset.is_none_or(|max| offset <= max);
        if keep_min && keep_max {
            continue;
        }

        let trace_start = trace_index * gather.samples;
        let trace_end = trace_start + gather.samples;
        gather.amplitudes[trace_start..trace_end].fill(0.0);
    }

    Ok(())
}

fn validate_offset_gather(gather: &GatherPlane) -> Result<(), SeismicStoreError> {
    if gather.gather_axis_kind != GatherAxisKind::Offset {
        return Err(SeismicStoreError::Message(format!(
            "gather processing requires offset gathers in phase one, found {:?}",
            gather.gather_axis_kind
        )));
    }

    if gather.sample_domain != GatherSampleDomain::Time {
        return Err(SeismicStoreError::Message(format!(
            "gather processing requires time-domain gathers in phase one, found {:?}",
            gather.sample_domain
        )));
    }

    if gather.horizontal_axis.len() != gather.traces {
        return Err(SeismicStoreError::Message(format!(
            "gather horizontal axis length mismatch: expected {}, found {}",
            gather.traces,
            gather.horizontal_axis.len()
        )));
    }

    if gather.sample_axis_ms.len() != gather.samples {
        return Err(SeismicStoreError::Message(format!(
            "gather sample axis length mismatch: expected {}, found {}",
            gather.samples,
            gather.sample_axis_ms.len()
        )));
    }

    if gather.amplitudes.len() != gather.traces * gather.samples {
        return Err(SeismicStoreError::Message(format!(
            "gather amplitude buffer length mismatch: expected {}, found {}",
            gather.traces * gather.samples,
            gather.amplitudes.len()
        )));
    }

    Ok(())
}

pub(crate) fn sample_interval_ms_from_axis(sample_axis_ms: &[f32]) -> Result<f32, SeismicStoreError> {
    if sample_axis_ms.len() < MIN_SAMPLE_AXIS_LEN {
        return Err(SeismicStoreError::Message(
            "gather sample axis must contain at least two entries".to_string(),
        ));
    }

    let step = (sample_axis_ms[1] - sample_axis_ms[0]).abs();
    if !step.is_finite() || step <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "gather sample axis step must be finite and > 0 ms, found {step}"
        )));
    }

    Ok(step)
}

pub(crate) fn validate_velocity_function_source(
    source: &VelocityFunctionSource,
) -> Result<(), SeismicStoreError> {
    match source {
        VelocityFunctionSource::ConstantVelocity { velocity_m_per_s } => {
            validate_velocity_value(*velocity_m_per_s, "constant velocity")
        }
        VelocityFunctionSource::TimeVelocityPairs {
            times_ms,
            velocities_m_per_s,
        } => {
            if times_ms.is_empty() {
                return Err(SeismicStoreError::Message(
                    "velocity model time-velocity pairs must not be empty".to_string(),
                ));
            }
            if times_ms.len() != velocities_m_per_s.len() {
                return Err(SeismicStoreError::Message(format!(
                    "velocity model pair length mismatch: {} times, {} velocities",
                    times_ms.len(),
                    velocities_m_per_s.len()
                )));
            }
            let mut previous_time = None;
            for (index, (time_ms, velocity_m_per_s)) in times_ms
                .iter()
                .zip(velocities_m_per_s.iter())
                .enumerate()
            {
                if !time_ms.is_finite() || *time_ms < 0.0 {
                    return Err(SeismicStoreError::Message(format!(
                        "velocity model time at index {index} must be finite and >= 0, found {time_ms}"
                    )));
                }
                if let Some(previous_time) = previous_time
                    && *time_ms < previous_time
                {
                    return Err(SeismicStoreError::Message(format!(
                        "velocity model times must be nondecreasing, found {time_ms} after {previous_time}"
                    )));
                }
                validate_velocity_value(*velocity_m_per_s, "velocity model pair velocity")?;
                previous_time = Some(*time_ms);
            }
            Ok(())
        }
        VelocityFunctionSource::VelocityAssetReference { asset_id } => {
            if asset_id.trim().is_empty() {
                return Err(SeismicStoreError::Message(
                    "velocity asset reference must not be empty".to_string(),
                ));
            }
            Ok(())
        }
    }
}

fn validate_velocity_value(value: f32, label: &str) -> Result<(), SeismicStoreError> {
    if !value.is_finite() || value < MIN_VELOCITY_M_PER_S {
        return Err(SeismicStoreError::Message(format!(
            "{label} must be finite and >= {MIN_VELOCITY_M_PER_S} m/s, found {value}"
        )));
    }
    Ok(())
}

fn validate_max_stretch_ratio(value: f32) -> Result<(), SeismicStoreError> {
    if !value.is_finite() || !(0.0..=MAX_STRETCH_RATIO).contains(&value) || value <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "max_stretch_ratio must be in (0.0, {MAX_STRETCH_RATIO}], found {value}"
        )));
    }
    Ok(())
}

fn validate_offset_range(
    min_offset: Option<f32>,
    max_offset: Option<f32>,
) -> Result<(), SeismicStoreError> {
    if min_offset.is_none() && max_offset.is_none() {
        return Err(SeismicStoreError::Message(
            "offset mute must define at least one of min_offset or max_offset".to_string(),
        ));
    }
    for (label, value) in [("min_offset", min_offset), ("max_offset", max_offset)] {
        if let Some(value) = value
            && !value.is_finite()
        {
            return Err(SeismicStoreError::Message(format!(
                "{label} must be finite, found {value}"
            )));
        }
    }
    if let (Some(min_offset), Some(max_offset)) = (min_offset, max_offset)
        && min_offset > max_offset
    {
        return Err(SeismicStoreError::Message(format!(
            "offset mute range must satisfy min_offset <= max_offset, found [{min_offset}, {max_offset}]"
        )));
    }
    Ok(())
}

pub(crate) fn nmo_input_time_ms(
    zero_offset_time_ms: f32,
    offset_m: f32,
    velocity_model: &VelocityFunctionSource,
) -> Result<f32, SeismicStoreError> {
    let velocity_m_per_s = velocity_at_time_ms(velocity_model, zero_offset_time_ms)?;
    let zero_offset_time_s = zero_offset_time_ms / 1000.0;
    let offset_term_s = offset_m.abs() / velocity_m_per_s;
    Ok(((zero_offset_time_s * zero_offset_time_s) + (offset_term_s * offset_term_s)).sqrt()
        * 1000.0)
}

fn velocity_at_time_ms(
    source: &VelocityFunctionSource,
    time_ms: f32,
) -> Result<f32, SeismicStoreError> {
    match source {
        VelocityFunctionSource::ConstantVelocity { velocity_m_per_s } => Ok(*velocity_m_per_s),
        VelocityFunctionSource::TimeVelocityPairs {
            times_ms,
            velocities_m_per_s,
        } => {
            if times_ms.len() == 1 {
                return Ok(velocities_m_per_s[0]);
            }

            let mut upper_index = 0usize;
            while upper_index < times_ms.len() && times_ms[upper_index] < time_ms {
                upper_index += 1;
            }

            if upper_index == 0 {
                return Ok(velocities_m_per_s[0]);
            }
            if upper_index >= times_ms.len() {
                return Ok(*velocities_m_per_s.last().expect("velocity model should not be empty"));
            }

            let lower_index = upper_index - 1;
            let lower_time = times_ms[lower_index];
            let upper_time = times_ms[upper_index];
            if (upper_time - lower_time).abs() <= TIME_EPSILON_MS {
                return Ok(velocities_m_per_s[upper_index]);
            }

            let t = ((time_ms - lower_time) / (upper_time - lower_time)).clamp(0.0, 1.0);
            let lower_velocity = velocities_m_per_s[lower_index];
            let upper_velocity = velocities_m_per_s[upper_index];
            Ok(lower_velocity + (upper_velocity - lower_velocity) * t)
        }
        VelocityFunctionSource::VelocityAssetReference { asset_id } => Err(
            SeismicStoreError::Message(format!(
                "velocity asset references are not yet resolvable in the runtime kernel path: {asset_id}"
            )),
        ),
    }
}

pub(crate) fn interpolate_trace_sample(
    trace: &[f32],
    time_ms: f32,
    sample_interval_ms: f32,
    interpolation: GatherInterpolationMode,
) -> f32 {
    match interpolation {
        GatherInterpolationMode::Linear => linear_interpolate_trace_sample(trace, time_ms, sample_interval_ms),
    }
}

fn linear_interpolate_trace_sample(trace: &[f32], time_ms: f32, sample_interval_ms: f32) -> f32 {
    if trace.is_empty() || !time_ms.is_finite() || time_ms < 0.0 {
        return 0.0;
    }

    let sample_position = time_ms / sample_interval_ms;
    let lower_index = sample_position.floor() as usize;
    if lower_index >= trace.len() {
        return 0.0;
    }
    let upper_index = (lower_index + 1).min(trace.len() - 1);
    if lower_index == upper_index {
        return trace[lower_index];
    }

    let fraction = (sample_position - lower_index as f32).clamp(0.0, 1.0);
    trace[lower_index] * (1.0 - fraction) + trace[upper_index] * fraction
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FrequencyPhaseMode, FrequencyWindowShape, TraceLocalProcessingOperation};

    fn synthetic_gather(offsets: &[f64], samples: usize, sample_interval_ms: f32) -> GatherPlane {
        GatherPlane {
            label: "synthetic".to_string(),
            gather_axis_kind: GatherAxisKind::Offset,
            sample_domain: GatherSampleDomain::Time,
            traces: offsets.len(),
            samples,
            horizontal_axis: offsets.to_vec(),
            sample_axis_ms: (0..samples)
                .map(|index| index as f32 * sample_interval_ms)
                .collect(),
            amplitudes: vec![0.0; offsets.len() * samples],
        }
    }

    fn constant_velocity_model(velocity_m_per_s: f32) -> VelocityFunctionSource {
        VelocityFunctionSource::ConstantVelocity { velocity_m_per_s }
    }

    #[test]
    fn gather_pipeline_validation_rejects_non_offset_layouts() {
        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: None,
            description: None,
            trace_local_pipeline: None,
            operations: vec![GatherProcessingOperation::OffsetMute {
                min_offset: None,
                max_offset: Some(1500.0),
            }],
        };

        let result =
            validate_gather_processing_pipeline_for_layout(&pipeline, SeismicLayout::PreStack3DAngle);
        assert!(result.is_err());
    }

    #[test]
    fn gather_pipeline_validation_rejects_invalid_stretch_ratio() {
        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: None,
            description: None,
            trace_local_pipeline: None,
            operations: vec![GatherProcessingOperation::StretchMute {
                velocity_model: constant_velocity_model(2000.0),
                max_stretch_ratio: 0.0,
            }],
        };

        let result = validate_gather_processing_pipeline(&pipeline);
        assert!(result.is_err());
    }

    #[test]
    fn offset_mute_zeros_traces_outside_window() {
        let mut gather = synthetic_gather(&[-1500.0, -500.0, 500.0, 1500.0], 8, 4.0);
        gather.amplitudes.fill(1.0);

        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: None,
            description: None,
            trace_local_pipeline: None,
            operations: vec![GatherProcessingOperation::OffsetMute {
                min_offset: Some(-700.0),
                max_offset: Some(700.0),
            }],
        };

        apply_gather_processing_pipeline(&mut gather, &pipeline).unwrap();

        for trace_index in [0usize, 3usize] {
            let start = trace_index * gather.samples;
            assert!(gather.amplitudes[start..start + gather.samples]
                .iter()
                .all(|value| *value == 0.0));
        }
        for trace_index in [1usize, 2usize] {
            let start = trace_index * gather.samples;
            assert!(gather.amplitudes[start..start + gather.samples]
                .iter()
                .all(|value| *value == 1.0));
        }
    }

    #[test]
    fn nmo_correction_flattens_synthetic_hyperbola() {
        let offsets = [-1000.0_f64, 0.0, 1000.0];
        let mut gather = synthetic_gather(&offsets, 256, 4.0);
        let zero_offset_time_ms = 500.0_f32;
        let velocity_m_per_s = 2000.0_f32;

        for (trace_index, offset) in offsets.iter().enumerate() {
            let event_time_ms = nmo_input_time_ms(
                zero_offset_time_ms,
                *offset as f32,
                &constant_velocity_model(velocity_m_per_s),
            )
            .unwrap();
            let sample_index = (event_time_ms / 4.0).round() as usize;
            gather.amplitudes[trace_index * gather.samples + sample_index] = 1.0;
        }

        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: None,
            description: None,
            trace_local_pipeline: None,
            operations: vec![GatherProcessingOperation::NmoCorrection {
                velocity_model: constant_velocity_model(velocity_m_per_s),
                interpolation: GatherInterpolationMode::Linear,
            }],
        };

        apply_gather_processing_pipeline(&mut gather, &pipeline).unwrap();

        let expected_index = (zero_offset_time_ms / 4.0).round() as usize;
        for trace_index in 0..gather.traces {
            let trace_start = trace_index * gather.samples;
            let trace = &gather.amplitudes[trace_start..trace_start + gather.samples];
            let (peak_index, _) = trace
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    left.abs()
                        .partial_cmp(&right.abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("trace should not be empty");
            assert!(peak_index.abs_diff(expected_index) <= 1);
        }
    }

    #[test]
    fn stretch_mute_zeros_far_offset_early_samples() {
        let offsets = [0.0_f64, 1500.0];
        let mut gather = synthetic_gather(&offsets, 128, 4.0);
        gather.amplitudes.fill(1.0);

        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: None,
            description: None,
            trace_local_pipeline: None,
            operations: vec![GatherProcessingOperation::StretchMute {
                velocity_model: constant_velocity_model(2000.0),
                max_stretch_ratio: 0.25,
            }],
        };

        apply_gather_processing_pipeline(&mut gather, &pipeline).unwrap();

        let near_trace = &gather.amplitudes[..gather.samples];
        let far_trace = &gather.amplitudes[gather.samples..];
        assert!(near_trace.iter().skip(10).take(20).all(|value| *value == 1.0));
        assert!(far_trace.iter().take(10).all(|value| *value == 0.0));
    }

    #[test]
    fn trace_local_pipeline_runs_on_gather_before_gather_ops() {
        let mut gather = synthetic_gather(&[0.0_f64, 500.0], 32, 4.0);
        gather.amplitudes.fill(2.0);

        let pipeline = GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: None,
            description: None,
            trace_local_pipeline: Some(TraceLocalProcessingPipeline {
                schema_version: 1,
                revision: 1,
                preset_id: None,
                name: None,
                description: None,
                operations: vec![
                    TraceLocalProcessingOperation::AmplitudeScalar { factor: 0.5 },
                    TraceLocalProcessingOperation::BandpassFilter {
                        f1_hz: 2.0,
                        f2_hz: 4.0,
                        f3_hz: 20.0,
                        f4_hz: 28.0,
                        phase: FrequencyPhaseMode::Zero,
                        window: FrequencyWindowShape::CosineTaper,
                    },
                ],
            }),
            operations: vec![GatherProcessingOperation::OffsetMute {
                min_offset: None,
                max_offset: Some(100.0),
            }],
        };

        apply_gather_processing_pipeline(&mut gather, &pipeline).unwrap();

        assert!(gather.amplitudes[..gather.samples]
            .iter()
            .any(|value| value.abs() <= 1.0));
        assert!(gather.amplitudes[gather.samples..]
            .iter()
            .all(|value| *value == 0.0));
    }
}
