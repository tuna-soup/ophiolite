use ophiolite_seismic::{
    GatherInterpolationMode, SemblancePanel, TraceLocalProcessingPipeline,
    VelocityAutopickParameters, VelocityFunctionEstimate, VelocityFunctionSource,
    VelocityPickStrategy, VelocityScanRequest, VelocityScanResponse,
};

use crate::error::SeismicStoreError;
#[cfg(test)]
use crate::gather_processing::nmo_input_time_ms;
use crate::gather_processing::{
    interpolate_trace_sample, sample_interval_ms_from_axis, validate_velocity_function_source,
};
use crate::prestack_store::{open_prestack_store, read_gather_plane};

const SEMBLANCE_EPSILON: f32 = 1.0e-12;
const MAX_VELOCITY_SCAN_SAMPLES: usize = 4096;
const MAX_AUTOPICK_SMOOTHING_SAMPLES: usize = 1024;

#[derive(Debug, Default)]
struct SemblanceWorkspace {
    sample_time_sq_s: Vec<f32>,
    offset_sq: Vec<f32>,
}

pub fn velocity_scan(
    request: VelocityScanRequest,
) -> Result<VelocityScanResponse, SeismicStoreError> {
    let handle = open_prestack_store(&request.store_path)?;
    let mut gather = read_gather_plane(&request.store_path, &request.gather)?;

    if let Some(trace_local_pipeline) = request.trace_local_pipeline.as_ref() {
        crate::compute::validate_processing_pipeline_for_layout(
            trace_local_pipeline,
            handle.manifest.layout,
        )?;
        crate::gather_processing::apply_trace_local_pipeline_to_gather(
            &mut gather,
            trace_local_pipeline,
        )?;
    }

    let velocities_m_per_s = velocity_scan_axis(
        request.min_velocity_m_per_s,
        request.max_velocity_m_per_s,
        request.velocity_step_m_per_s,
    )?;
    let sample_interval_ms = sample_interval_ms_from_axis(&gather.sample_axis_ms)?;
    let sample_axis_ms = gather.sample_axis_ms.clone();
    let semblance = compute_semblance_panel(
        &gather.amplitudes,
        gather.traces,
        gather.samples,
        &gather.horizontal_axis,
        &sample_axis_ms,
        sample_interval_ms,
        &velocities_m_per_s,
    )?;
    let autopicked_velocity_function = request
        .autopick
        .as_ref()
        .map(|autopick| {
            pick_velocity_function(&semblance, &velocities_m_per_s, &sample_axis_ms, autopick)
        })
        .transpose()?;

    Ok(VelocityScanResponse {
        schema_version: request.schema_version,
        gather: request.gather,
        panel: SemblancePanel {
            velocities_m_per_s: velocities_m_per_s.clone(),
            sample_axis_ms,
            semblance_f32le: f32_vec_to_le_bytes(&semblance),
        },
        processing_label: request
            .trace_local_pipeline
            .as_ref()
            .map(trace_local_pipeline_label),
        autopicked_velocity_function,
    })
}

fn velocity_scan_axis(
    min_velocity_m_per_s: f32,
    max_velocity_m_per_s: f32,
    velocity_step_m_per_s: f32,
) -> Result<Vec<f32>, SeismicStoreError> {
    validate_velocity_function_source(&VelocityFunctionSource::ConstantVelocity {
        velocity_m_per_s: min_velocity_m_per_s,
    })?;
    validate_velocity_function_source(&VelocityFunctionSource::ConstantVelocity {
        velocity_m_per_s: max_velocity_m_per_s,
    })?;
    if !velocity_step_m_per_s.is_finite() || velocity_step_m_per_s <= 0.0 {
        return Err(SeismicStoreError::Message(format!(
            "velocity scan step must be finite and > 0, found {velocity_step_m_per_s}"
        )));
    }
    if min_velocity_m_per_s > max_velocity_m_per_s {
        return Err(SeismicStoreError::Message(format!(
            "velocity scan range must satisfy min <= max, found [{min_velocity_m_per_s}, {max_velocity_m_per_s}]"
        )));
    }

    let mut velocities = Vec::new();
    let mut current = min_velocity_m_per_s;
    while current <= max_velocity_m_per_s + (velocity_step_m_per_s * 0.5) {
        velocities.push(current.min(max_velocity_m_per_s));
        current += velocity_step_m_per_s;
        if velocities.len() > MAX_VELOCITY_SCAN_SAMPLES {
            return Err(SeismicStoreError::Message(format!(
                "velocity scan produced more than {MAX_VELOCITY_SCAN_SAMPLES} trial velocities"
            )));
        }
    }

    if velocities.is_empty() {
        return Err(SeismicStoreError::Message(
            "velocity scan requires at least one trial velocity".to_string(),
        ));
    }
    Ok(velocities)
}

fn compute_semblance_panel(
    amplitudes: &[f32],
    traces: usize,
    samples: usize,
    offsets_m: &[f64],
    sample_axis_ms: &[f32],
    sample_interval_ms: f32,
    velocities_m_per_s: &[f32],
) -> Result<Vec<f32>, SeismicStoreError> {
    if amplitudes.len() != traces * samples {
        return Err(SeismicStoreError::Message(format!(
            "gather amplitude length mismatch: expected {}, found {}",
            traces * samples,
            amplitudes.len()
        )));
    }
    if offsets_m.len() != traces {
        return Err(SeismicStoreError::Message(format!(
            "gather offset axis mismatch: expected {traces}, found {}",
            offsets_m.len()
        )));
    }
    if sample_axis_ms.len() != samples {
        return Err(SeismicStoreError::Message(format!(
            "gather sample axis mismatch: expected {samples}, found {}",
            sample_axis_ms.len()
        )));
    }

    let mut workspace = SemblanceWorkspace::default();
    workspace.prepare(sample_axis_ms, offsets_m);
    let mut panel = vec![0.0_f32; velocities_m_per_s.len() * samples];
    for (velocity_index, velocity_m_per_s) in velocities_m_per_s.iter().copied().enumerate() {
        let inverse_velocity_sq = 1.0 / (velocity_m_per_s * velocity_m_per_s);
        let panel_row = &mut panel[velocity_index * samples..(velocity_index + 1) * samples];
        for sample_index in 0..samples {
            let mut stack_sum = 0.0_f32;
            let mut energy_sum = 0.0_f32;
            for trace_index in 0..traces {
                let trace_start = trace_index * samples;
                let trace = &amplitudes[trace_start..trace_start + samples];
                let input_time_ms = semblance_input_time_ms(
                    workspace.sample_time_sq_s[sample_index],
                    workspace.offset_sq[trace_index],
                    inverse_velocity_sq,
                );
                let sample = interpolate_trace_sample(
                    trace,
                    input_time_ms,
                    sample_interval_ms,
                    GatherInterpolationMode::Linear,
                );
                stack_sum += sample;
                energy_sum += sample * sample;
            }

            let semblance = if energy_sum <= SEMBLANCE_EPSILON {
                0.0
            } else {
                ((stack_sum * stack_sum) / (traces as f32 * energy_sum + SEMBLANCE_EPSILON))
                    .clamp(0.0, 1.0)
            };
            panel_row[sample_index] = semblance;
        }
    }

    Ok(panel)
}

impl SemblanceWorkspace {
    fn prepare(&mut self, sample_axis_ms: &[f32], offsets_m: &[f64]) {
        self.sample_time_sq_s.resize(sample_axis_ms.len(), 0.0);
        for (index, time_ms) in sample_axis_ms.iter().copied().enumerate() {
            let time_s = time_ms.max(0.0) / 1000.0;
            self.sample_time_sq_s[index] = time_s * time_s;
        }

        self.offset_sq.resize(offsets_m.len(), 0.0);
        for (index, offset_m) in offsets_m.iter().copied().enumerate() {
            let offset_m = offset_m as f32;
            self.offset_sq[index] = offset_m * offset_m;
        }
    }
}

fn semblance_input_time_ms(sample_time_sq_s: f32, offset_sq: f32, inverse_velocity_sq: f32) -> f32 {
    (sample_time_sq_s + (offset_sq * inverse_velocity_sq)).sqrt() * 1000.0
}

fn pick_velocity_function(
    semblance_panel: &[f32],
    velocities_m_per_s: &[f32],
    sample_axis_ms: &[f32],
    parameters: &VelocityAutopickParameters,
) -> Result<VelocityFunctionEstimate, SeismicStoreError> {
    validate_autopick_parameters(parameters)?;

    let sample_count = sample_axis_ms.len();
    if sample_count == 0 {
        return Err(SeismicStoreError::Message(
            "velocity autopick requires a non-empty sample axis".to_string(),
        ));
    }
    if semblance_panel.len() != velocities_m_per_s.len() * sample_count {
        return Err(SeismicStoreError::Message(format!(
            "semblance panel length mismatch: expected {}, found {}",
            velocities_m_per_s.len() * sample_count,
            semblance_panel.len()
        )));
    }
    if velocities_m_per_s.is_empty() {
        return Err(SeismicStoreError::Message(
            "velocity autopick requires at least one trial velocity".to_string(),
        ));
    }

    let mut times_ms = Vec::new();
    let mut picked_velocities = Vec::new();
    let mut picked_semblance = Vec::new();
    let stride = parameters.sample_stride.max(1);

    for sample_index in (0..sample_count).step_by(stride) {
        let time_ms = sample_axis_ms[sample_index];
        if parameters
            .min_time_ms
            .is_some_and(|min_time_ms| time_ms < min_time_ms)
        {
            continue;
        }
        if parameters
            .max_time_ms
            .is_some_and(|max_time_ms| time_ms > max_time_ms)
        {
            continue;
        }

        let mut best_velocity_index = 0usize;
        let mut best_value = f32::MIN;
        for velocity_index in 0..velocities_m_per_s.len() {
            let value = semblance_panel[velocity_index * sample_count + sample_index];
            if value > best_value {
                best_value = value;
                best_velocity_index = velocity_index;
            }
        }

        if best_value < parameters.min_semblance {
            continue;
        }

        times_ms.push(time_ms);
        picked_velocities.push(velocities_m_per_s[best_velocity_index]);
        picked_semblance.push(best_value);
    }

    smooth_velocity_samples(&mut picked_velocities, parameters.smoothing_samples);

    Ok(VelocityFunctionEstimate {
        strategy: VelocityPickStrategy::MaximumSemblance,
        times_ms,
        velocities_m_per_s: picked_velocities,
        semblance: picked_semblance,
    })
}

fn validate_autopick_parameters(
    parameters: &VelocityAutopickParameters,
) -> Result<(), SeismicStoreError> {
    if parameters.sample_stride == 0 {
        return Err(SeismicStoreError::Message(
            "velocity autopick sample_stride must be >= 1".to_string(),
        ));
    }
    if !parameters.min_semblance.is_finite() || !(0.0..=1.0).contains(&parameters.min_semblance) {
        return Err(SeismicStoreError::Message(format!(
            "velocity autopick min_semblance must be finite and in [0, 1], found {}",
            parameters.min_semblance
        )));
    }
    if parameters.smoothing_samples == 0
        || parameters.smoothing_samples > MAX_AUTOPICK_SMOOTHING_SAMPLES
    {
        return Err(SeismicStoreError::Message(format!(
            "velocity autopick smoothing_samples must be in [1, {MAX_AUTOPICK_SMOOTHING_SAMPLES}], found {}",
            parameters.smoothing_samples
        )));
    }
    if let (Some(min_time_ms), Some(max_time_ms)) = (parameters.min_time_ms, parameters.max_time_ms)
    {
        if !min_time_ms.is_finite() || !max_time_ms.is_finite() || min_time_ms > max_time_ms {
            return Err(SeismicStoreError::Message(format!(
                "velocity autopick time range must be finite and satisfy min <= max, found [{min_time_ms}, {max_time_ms}]"
            )));
        }
    }
    for (label, value) in [
        ("min_time_ms", parameters.min_time_ms),
        ("max_time_ms", parameters.max_time_ms),
    ] {
        if let Some(value) = value
            && !value.is_finite()
        {
            return Err(SeismicStoreError::Message(format!(
                "velocity autopick {label} must be finite, found {value}"
            )));
        }
    }

    Ok(())
}

fn smooth_velocity_samples(values: &mut [f32], smoothing_samples: usize) {
    if values.len() < 2 || smoothing_samples <= 1 {
        return;
    }

    let radius = smoothing_samples / 2;
    let original = values.to_vec();
    for (index, value) in values.iter_mut().enumerate() {
        let start = index.saturating_sub(radius);
        let end = (index + radius + 1).min(original.len());
        let window = &original[start..end];
        let sum = window.iter().copied().sum::<f32>();
        *value = sum / window.len() as f32;
    }
}

fn trace_local_pipeline_label(pipeline: &TraceLocalProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return name.to_string();
    }
    if pipeline.operation_count() == 0 {
        return "trace-local".to_string();
    }
    pipeline
        .operations()
        .map(|operation| operation.operator_id().to_string())
        .collect::<Vec<_>>()
        .join("__")
}

fn f32_vec_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use ophiolite_seismic::{
        DatasetId, GatherRequest, GatherSelector, VelocityAutopickParameters, VelocityPickStrategy,
    };

    use std::path::PathBuf;

    use crate::metadata::{
        DatasetKind, GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes,
        VolumeMetadata, generate_store_id,
    };
    use crate::prestack_store::{TbgathManifest, create_tbgath_store};

    fn fixture_manifest() -> TbgathManifest {
        TbgathManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: generate_store_id(),
                source: SourceIdentity {
                    source_path: PathBuf::from("synthetic.sgy"),
                    file_size: 0,
                    trace_count: 3,
                    samples_per_trace: 4,
                    sample_interval_us: 2000,
                    sample_format_code: 5,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(5),
                    endianness: "big".to_string(),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
                        inline_field: HeaderFieldSpec {
                            name: "INLINE_3D".to_string(),
                            start_byte: 189,
                            value_type: "i32".to_string(),
                        },
                        crossline_field: HeaderFieldSpec {
                            name: "CROSSLINE_3D".to_string(),
                            start_byte: 193,
                            value_type: "i32".to_string(),
                        },
                        third_axis_field: Some(HeaderFieldSpec {
                            name: "OFFSET".to_string(),
                            start_byte: 37,
                            value_type: "i32".to_string(),
                        }),
                    },
                    regularization: None,
                },
                shape: [1, 1, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![1000.0],
                    vec![2000.0],
                    vec![0.0, 4.0, 8.0, 12.0],
                ),
                segy_export: None,
                coordinate_reference_binding: None,
                spatial: None,
                created_by: "test".to_string(),
                processing_lineage: None,
            },
            ophiolite_seismic::SeismicLayout::PreStack3DOffset,
            ophiolite_seismic::GatherAxisKind::Offset,
            vec![-500.0, 0.0, 500.0],
        )
    }

    fn fixture_data(manifest: &TbgathManifest) -> Vec<f32> {
        let samples = manifest.volume.shape[2];
        let offsets = &manifest.gather_axis_values;
        let true_velocity = 2000.0_f32;
        let zero_offset_time_ms = 4.0_f32;
        let sample_interval_ms = 4.0_f32;
        let mut values = vec![0.0_f32; manifest.total_values()];

        for (trace_index, offset) in offsets.iter().enumerate() {
            let event_time_ms = nmo_input_time_ms(
                zero_offset_time_ms,
                *offset as f32,
                &VelocityFunctionSource::ConstantVelocity {
                    velocity_m_per_s: true_velocity,
                },
            )
            .expect("fixture NMO time");
            let sample_index = (event_time_ms / sample_interval_ms).round() as usize;
            if sample_index < samples {
                values[trace_index * samples + sample_index] = 1.0;
            }
        }

        values
    }

    #[test]
    fn velocity_scan_axis_rejects_invalid_range() {
        let result = velocity_scan_axis(3000.0, 1500.0, 100.0);
        assert!(result.is_err());
    }

    #[test]
    fn autopick_validation_rejects_invalid_threshold() {
        let result = validate_autopick_parameters(&VelocityAutopickParameters {
            sample_stride: 1,
            min_time_ms: None,
            max_time_ms: None,
            min_semblance: 1.5,
            smoothing_samples: 1,
        });
        assert!(result.is_err());
    }

    #[test]
    fn semblance_panel_peaks_near_true_velocity_for_synthetic_hyperbola() {
        let traces = 3usize;
        let samples = 256usize;
        let sample_interval_ms = 4.0_f32;
        let offsets = [-1000.0_f64, 0.0, 1000.0];
        let sample_axis_ms = (0..samples)
            .map(|index| index as f32 * sample_interval_ms)
            .collect::<Vec<_>>();
        let true_velocity = 2000.0_f32;
        let zero_offset_time_ms = 600.0_f32;
        let mut amplitudes = vec![0.0_f32; traces * samples];

        for (trace_index, offset) in offsets.iter().enumerate() {
            let event_time_ms = nmo_input_time_ms(
                zero_offset_time_ms,
                *offset as f32,
                &VelocityFunctionSource::ConstantVelocity {
                    velocity_m_per_s: true_velocity,
                },
            )
            .unwrap();
            let sample_index = (event_time_ms / sample_interval_ms).round() as usize;
            amplitudes[trace_index * samples + sample_index] = 1.0;
        }

        let velocities = vec![1500.0_f32, 2000.0_f32, 2500.0_f32];
        let panel = compute_semblance_panel(
            &amplitudes,
            traces,
            samples,
            &offsets,
            &sample_axis_ms,
            sample_interval_ms,
            &velocities,
        )
        .unwrap();

        let target_sample = (zero_offset_time_ms / sample_interval_ms).round() as usize;
        let target_values = (0..velocities.len())
            .map(|velocity_index| panel[velocity_index * samples + target_sample])
            .collect::<Vec<_>>();
        let mut best_velocity_index = 0usize;
        let mut best_value = f32::MIN;
        for (velocity_index, value) in target_values.iter().copied().enumerate() {
            if value > best_value {
                best_value = value;
                best_velocity_index = velocity_index;
            }
        }

        assert_eq!(velocities[best_velocity_index], true_velocity);
        assert!(best_value > target_values[0]);
        assert!(best_value > target_values[2]);
    }

    #[test]
    fn autopick_recovers_true_velocity_for_synthetic_hyperbola() {
        let traces = 3usize;
        let samples = 256usize;
        let sample_interval_ms = 4.0_f32;
        let offsets = [-1000.0_f64, 0.0, 1000.0];
        let sample_axis_ms = (0..samples)
            .map(|index| index as f32 * sample_interval_ms)
            .collect::<Vec<_>>();
        let true_velocity = 2000.0_f32;
        let zero_offset_time_ms = 600.0_f32;
        let mut amplitudes = vec![0.0_f32; traces * samples];

        for (trace_index, offset) in offsets.iter().enumerate() {
            let event_time_ms = nmo_input_time_ms(
                zero_offset_time_ms,
                *offset as f32,
                &VelocityFunctionSource::ConstantVelocity {
                    velocity_m_per_s: true_velocity,
                },
            )
            .unwrap();
            let sample_index = (event_time_ms / sample_interval_ms).round() as usize;
            amplitudes[trace_index * samples + sample_index] = 1.0;
        }

        let velocities = vec![1500.0_f32, 2000.0_f32, 2500.0_f32];
        let panel = compute_semblance_panel(
            &amplitudes,
            traces,
            samples,
            &offsets,
            &sample_axis_ms,
            sample_interval_ms,
            &velocities,
        )
        .unwrap();

        let estimate = pick_velocity_function(
            &panel,
            &velocities,
            &sample_axis_ms,
            &VelocityAutopickParameters {
                sample_stride: 1,
                min_time_ms: Some(600.0),
                max_time_ms: Some(600.0),
                min_semblance: 0.0,
                smoothing_samples: 1,
            },
        )
        .unwrap();

        assert_eq!(estimate.strategy, VelocityPickStrategy::MaximumSemblance);
        assert_eq!(estimate.velocities_m_per_s, vec![true_velocity]);
        assert_eq!(estimate.times_ms, vec![600.0]);
        assert_eq!(estimate.semblance.len(), estimate.times_ms.len());
    }

    #[test]
    fn velocity_scan_response_round_trips_from_prestack_store() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let root = temp_dir.path().join("scan.tbgath");
        let manifest = fixture_manifest();
        let data = fixture_data(&manifest);
        create_tbgath_store(&root, manifest, &data).expect("store");

        let response = velocity_scan(VelocityScanRequest {
            schema_version: 1,
            store_path: root.display().to_string(),
            gather: GatherRequest {
                dataset_id: DatasetId("scan.tbgath".to_string()),
                selector: GatherSelector::Ordinal { index: 0 },
            },
            trace_local_pipeline: None,
            min_velocity_m_per_s: 1500.0,
            max_velocity_m_per_s: 2500.0,
            velocity_step_m_per_s: 500.0,
            autopick: Some(VelocityAutopickParameters {
                sample_stride: 1,
                min_time_ms: Some(0.0),
                max_time_ms: Some(12.0),
                min_semblance: 0.0,
                smoothing_samples: 1,
            }),
        })
        .expect("velocity scan should succeed");

        assert_eq!(
            response.panel.velocities_m_per_s,
            vec![1500.0, 2000.0, 2500.0]
        );
        assert_eq!(response.panel.sample_axis_ms.len(), 4);
        assert_eq!(
            response.panel.semblance_f32le.len(),
            response.panel.velocities_m_per_s.len()
                * response.panel.sample_axis_ms.len()
                * std::mem::size_of::<f32>()
        );
        let estimate = response
            .autopicked_velocity_function
            .expect("autopick should be present");
        assert_eq!(estimate.strategy, VelocityPickStrategy::MaximumSemblance);
        assert!(!estimate.times_ms.is_empty());
        assert_eq!(estimate.times_ms.len(), estimate.velocities_m_per_s.len());
        assert_eq!(estimate.times_ms.len(), estimate.semblance.len());
    }
}
