use std::path::PathBuf;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ndarray::Array3;
use ophiolite_seismic_runtime::{
    DatasetId, DatasetKind, FrequencyPhaseMode, FrequencyWindowShape, GatherAxisKind,
    GatherInterpolationMode, GatherPlane, GatherProcessingOperation, GatherProcessingPipeline,
    GatherRequest, GatherSampleDomain, GatherSelector, GeometryProvenance, HeaderFieldSpec,
    MaterializeOptions, ProcessingOperation, SeismicLayout, SourceIdentity, TbgathManifest,
    TbvolManifest, VelocityAutopickParameters, VelocityFunctionSource, VelocityScanRequest,
    VolumeAxes, VolumeMetadata, apply_gather_processing_pipeline, apply_pipeline_to_traces,
    create_tbgath_store, create_tbvol_store, generate_store_id, materialize_volume,
    preview_section_plane, recommended_chunk_shape, segy_sample_data_fidelity, velocity_scan,
};
use tempfile::tempdir;

fn compute_storage_preview(c: &mut Criterion) {
    let temp = tempdir().expect("temp dir");
    let store_root = temp.path().join("bench-preview.tbvol");
    let shape = [64, 64, 256];
    let data = synthetic_data(shape);
    let manifest = fixture_manifest(shape);
    create_tbvol_store(&store_root, manifest, &data, None).expect("store should be created");
    let pipeline = [
        ProcessingOperation::AmplitudeScalar { factor: 2.0 },
        ProcessingOperation::TraceRmsNormalize,
    ];

    c.bench_function("preview_section_pipeline", |b| {
        b.iter(|| {
            preview_section_plane(
                &store_root,
                ophiolite_seismic_runtime::SectionAxis::Inline,
                32,
                &pipeline,
            )
        })
    });
}

fn compute_storage_materialize(c: &mut Criterion) {
    let temp = tempdir().expect("temp dir");
    let input_root = temp.path().join("bench-materialize-input.tbvol");
    let shape = [64, 64, 256];
    let data = synthetic_data(shape);
    let manifest = fixture_manifest(shape);
    create_tbvol_store(&input_root, manifest, &data, None).expect("store should be created");
    let pipeline = [
        ProcessingOperation::AmplitudeScalar { factor: 2.0 },
        ProcessingOperation::TraceRmsNormalize,
    ];

    c.bench_function("materialize_volume_pipeline", |b| {
        b.iter(|| {
            let output_root = temp.path().join("bench-materialize-output.tbvol");
            if output_root.exists() {
                std::fs::remove_dir_all(&output_root).expect("clean output");
            }
            materialize_volume(
                &input_root,
                &output_root,
                &pipeline,
                MaterializeOptions::default(),
            )
            .expect("materialize should succeed");
        })
    });
}

fn compute_operator_kernels(c: &mut Criterion) {
    let traces = 64usize;
    let samples = 256usize;
    let sample_interval_ms = 2.0_f32;
    let source = synthetic_section(traces, samples);
    let occupancy = vec![1_u8; traces];
    let phase_rotation_pipeline = [ProcessingOperation::PhaseRotation {
        angle_degrees: 35.0,
    }];
    let lowpass_pipeline = [benchmark_lowpass_operation(sample_interval_ms)];
    let highpass_pipeline = [benchmark_highpass_operation(sample_interval_ms)];
    let bandpass_pipeline = [benchmark_bandpass_operation(sample_interval_ms)];
    let envelope_pipeline = [ProcessingOperation::Envelope];
    let instantaneous_phase_pipeline = [ProcessingOperation::InstantaneousPhase];
    let instantaneous_frequency_pipeline = [ProcessingOperation::InstantaneousFrequency];
    let sweetness_pipeline = [ProcessingOperation::Sweetness];
    let bandpass_phase_rotation_pipeline = [
        benchmark_bandpass_operation(sample_interval_ms),
        ProcessingOperation::PhaseRotation {
            angle_degrees: 35.0,
        },
    ];
    let analytic_stack_pipeline = [
        ProcessingOperation::TraceRmsNormalize,
        ProcessingOperation::Envelope,
        ProcessingOperation::InstantaneousPhase,
        ProcessingOperation::InstantaneousFrequency,
        ProcessingOperation::Sweetness,
    ];

    let mut group = c.benchmark_group("operator_kernels");

    group.bench_function("amplitude_scalar", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &[ProcessingOperation::AmplitudeScalar { factor: 2.0 }],
                )
                .expect("scalar pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("trace_rms_normalize", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &[ProcessingOperation::TraceRmsNormalize],
                )
                .expect("normalize pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("phase_rotation", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &phase_rotation_pipeline,
                )
                .expect("phase rotation pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("lowpass_filter", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &lowpass_pipeline,
                )
                .expect("lowpass pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("highpass_filter", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &highpass_pipeline,
                )
                .expect("highpass pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("bandpass_filter", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &bandpass_pipeline,
                )
                .expect("bandpass pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("envelope", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &envelope_pipeline,
                )
                .expect("envelope pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("instantaneous_phase", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &instantaneous_phase_pipeline,
                )
                .expect("instantaneous phase pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("instantaneous_frequency", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &instantaneous_frequency_pipeline,
                )
                .expect("instantaneous frequency pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("sweetness", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &sweetness_pipeline,
                )
                .expect("sweetness pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("bandpass_plus_phase_rotation", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &bandpass_phase_rotation_pipeline,
                )
                .expect("spectral combo pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("analytic_stack", |b| {
        b.iter_batched(
            || source.clone(),
            |mut values| {
                apply_pipeline_to_traces(
                    &mut values,
                    traces,
                    samples,
                    sample_interval_ms,
                    Some(&occupancy),
                    &analytic_stack_pipeline,
                )
                .expect("analytic stack pipeline should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

fn prestack_kernel_benchmarks(c: &mut Criterion) {
    let offsets = [-1000.0_f64, -500.0, 0.0, 500.0, 1000.0];
    let samples = 256usize;
    let sample_interval_ms = 4.0_f32;
    let true_velocity = 2000.0_f32;
    let base_gather =
        synthetic_prestack_gather(&offsets, samples, sample_interval_ms, true_velocity);
    let constant_velocity = VelocityFunctionSource::ConstantVelocity {
        velocity_m_per_s: true_velocity,
    };
    let nmo_pipeline = gather_pipeline(vec![GatherProcessingOperation::NmoCorrection {
        velocity_model: constant_velocity.clone(),
        interpolation: GatherInterpolationMode::Linear,
    }]);
    let stretch_mute_pipeline = gather_pipeline(vec![GatherProcessingOperation::StretchMute {
        velocity_model: constant_velocity.clone(),
        max_stretch_ratio: 0.25,
    }]);
    let offset_mute_pipeline = gather_pipeline(vec![GatherProcessingOperation::OffsetMute {
        min_offset: Some(-600.0),
        max_offset: Some(600.0),
    }]);

    let temp = tempdir().expect("temp dir");
    let store_root = temp.path().join("criterion-prestack.tbgath");
    let manifest = synthetic_prestack_manifest(&offsets, samples, sample_interval_ms);
    let data = synthetic_prestack_store_data(&offsets, samples, sample_interval_ms, true_velocity);
    create_tbgath_store(&store_root, manifest, &data).expect("prestack store should be created");
    let store_path = store_root.display().to_string();

    let mut group = c.benchmark_group("prestack_kernels");

    group.bench_function("semblance_panel_constant_velocity", |b| {
        b.iter(|| {
            velocity_scan(VelocityScanRequest {
                schema_version: 1,
                store_path: store_path.clone(),
                gather: GatherRequest {
                    dataset_id: DatasetId("criterion-prestack.tbgath".to_string()),
                    selector: GatherSelector::Ordinal { index: 0 },
                },
                trace_local_pipeline: None,
                min_velocity_m_per_s: 1500.0,
                max_velocity_m_per_s: 2500.0,
                velocity_step_m_per_s: 250.0,
                autopick: None,
            })
            .expect("semblance scan should succeed");
        })
    });

    group.bench_function("velocity_autopick", |b| {
        b.iter(|| {
            velocity_scan(VelocityScanRequest {
                schema_version: 1,
                store_path: store_path.clone(),
                gather: GatherRequest {
                    dataset_id: DatasetId("criterion-prestack.tbgath".to_string()),
                    selector: GatherSelector::Ordinal { index: 0 },
                },
                trace_local_pipeline: None,
                min_velocity_m_per_s: 1500.0,
                max_velocity_m_per_s: 2500.0,
                velocity_step_m_per_s: 250.0,
                autopick: Some(VelocityAutopickParameters {
                    sample_stride: 2,
                    min_time_ms: Some(200.0),
                    max_time_ms: Some(900.0),
                    min_semblance: 0.0,
                    smoothing_samples: 3,
                }),
            })
            .expect("velocity autopick should succeed");
        })
    });

    group.bench_function("gather_nmo_correction", |b| {
        b.iter_batched(
            || base_gather.clone(),
            |mut gather| {
                apply_gather_processing_pipeline(&mut gather, &nmo_pipeline)
                    .expect("NMO correction should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("gather_stretch_mute", |b| {
        b.iter_batched(
            || base_gather.clone(),
            |mut gather| {
                apply_gather_processing_pipeline(&mut gather, &stretch_mute_pipeline)
                    .expect("stretch mute should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("gather_offset_mute", |b| {
        b.iter_batched(
            || base_gather.clone(),
            |mut gather| {
                apply_gather_processing_pipeline(&mut gather, &offset_mute_pipeline)
                    .expect("offset mute should succeed");
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

fn synthetic_data(shape: [usize; 3]) -> Array3<f32> {
    Array3::from_shape_fn((shape[0], shape[1], shape[2]), |(iline, xline, sample)| {
        let il = iline as f32 / shape[0].max(1) as f32;
        let xl = xline as f32 / shape[1].max(1) as f32;
        let smp = sample as f32 / shape[2].max(1) as f32;
        ((il * 17.0).sin() + (xl * 11.0).cos()) * (1.0 - smp) + (smp * 31.0).sin() * 0.35
    })
}

fn synthetic_section(traces: usize, samples: usize) -> Vec<f32> {
    (0..traces)
        .flat_map(|trace| {
            (0..samples).map(move |sample| {
                let trace_ratio = trace as f32 / traces.max(1) as f32;
                let sample_ratio = sample as f32 / samples.max(1) as f32;
                ((trace_ratio * 17.0).sin() + (sample_ratio * 31.0).cos()) * (1.0 - sample_ratio)
                    + (sample_ratio * 11.0).sin() * 0.35
            })
        })
        .collect()
}

fn benchmark_bandpass_operation(sample_interval_ms: f32) -> ProcessingOperation {
    let nyquist_hz = 500.0 / sample_interval_ms.max(f32::EPSILON);
    let f1_hz = (nyquist_hz * 0.06).max(4.0);
    let f2_hz = (nyquist_hz * 0.1).max(f1_hz + 1.0);
    let f4_hz = (nyquist_hz * 0.45).max(f2_hz + 6.0).min(nyquist_hz);
    let f3_hz = (nyquist_hz * 0.32).max(f2_hz + 4.0).min(f4_hz);
    ProcessingOperation::BandpassFilter {
        f1_hz,
        f2_hz,
        f3_hz,
        f4_hz,
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn benchmark_lowpass_operation(sample_interval_ms: f32) -> ProcessingOperation {
    let nyquist_hz = 500.0 / sample_interval_ms.max(f32::EPSILON);
    let f4_hz = (nyquist_hz * 0.22).max(8.0).min(nyquist_hz);
    let f3_hz = (f4_hz * 0.75).max(4.0).min(f4_hz);
    ProcessingOperation::LowpassFilter {
        f3_hz,
        f4_hz,
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn benchmark_highpass_operation(sample_interval_ms: f32) -> ProcessingOperation {
    let nyquist_hz = 500.0 / sample_interval_ms.max(f32::EPSILON);
    let f1_hz = (nyquist_hz * 0.04).max(2.0);
    let f2_hz = (nyquist_hz * 0.08).max(f1_hz + 1.0).min(nyquist_hz);
    ProcessingOperation::HighpassFilter {
        f1_hz,
        f2_hz,
        phase: FrequencyPhaseMode::Zero,
        window: FrequencyWindowShape::CosineTaper,
    }
}

fn fixture_manifest(shape: [usize; 3]) -> TbvolManifest {
    TbvolManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://criterion"),
                file_size: (shape[0] * shape[1] * shape[2] * std::mem::size_of::<f32>()) as u64,
                trace_count: (shape[0] * shape[1]) as u64,
                samples_per_trace: shape[2],
                sample_interval_us: 2000,
                sample_format_code: 5,
                sample_data_fidelity: segy_sample_data_fidelity(5),
                endianness: "big".to_string(),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
                geometry: GeometryProvenance {
                    inline_field: HeaderFieldSpec {
                        name: "INLINE_SYNTHETIC".to_string(),
                        start_byte: 189,
                        value_type: "I32".to_string(),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: "CROSSLINE_SYNTHETIC".to_string(),
                        start_byte: 193,
                        value_type: "I32".to_string(),
                    },
                    third_axis_field: None,
                },
                regularization: None,
            },
            shape,
            axes: VolumeAxes::from_time_axis(
                (0..shape[0]).map(|value| value as f64).collect(),
                (0..shape[1]).map(|value| value as f64).collect(),
                (0..shape[2]).map(|value| value as f32 * 2.0).collect(),
            ),
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "criterion".to_string(),
            processing_lineage: None,
            segy_export: None,
        },
        recommended_chunk_shape(shape, 8),
        false,
    )
}

fn gather_pipeline(operations: Vec<GatherProcessingOperation>) -> GatherProcessingPipeline {
    GatherProcessingPipeline {
        schema_version: 1,
        revision: 1,
        preset_id: None,
        name: None,
        description: None,
        trace_local_pipeline: None,
        operations,
    }
}

fn synthetic_prestack_gather(
    offsets: &[f64],
    samples: usize,
    sample_interval_ms: f32,
    true_velocity: f32,
) -> GatherPlane {
    let mut gather = GatherPlane {
        label: "criterion-prestack".to_string(),
        gather_axis_kind: GatherAxisKind::Offset,
        sample_domain: GatherSampleDomain::Time,
        traces: offsets.len(),
        samples,
        horizontal_axis: offsets.to_vec(),
        sample_axis_ms: (0..samples)
            .map(|index| index as f32 * sample_interval_ms)
            .collect(),
        amplitudes: vec![0.0; offsets.len() * samples],
    };
    let zero_offset_time_ms = 600.0_f32;
    for (trace_index, offset) in offsets.iter().enumerate() {
        let event_time_ms = (((zero_offset_time_ms / 1000.0).powi(2)
            + ((*offset as f32 / true_velocity).powi(2)))
        .sqrt())
            * 1000.0;
        let sample_index = (event_time_ms / sample_interval_ms).round() as usize;
        if sample_index < samples {
            gather.amplitudes[trace_index * samples + sample_index] = 1.0;
        }
    }
    gather
}

fn synthetic_prestack_manifest(
    offsets: &[f64],
    samples: usize,
    sample_interval_ms: f32,
) -> TbgathManifest {
    TbgathManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://criterion-prestack"),
                file_size: (offsets.len() * samples * std::mem::size_of::<f32>()) as u64,
                trace_count: offsets.len() as u64,
                samples_per_trace: samples,
                sample_interval_us: (sample_interval_ms * 1000.0) as u16,
                sample_format_code: 5,
                sample_data_fidelity: segy_sample_data_fidelity(5),
                endianness: "big".to_string(),
                revision_raw: 0,
                fixed_length_trace_flag_raw: 1,
                extended_textual_headers: 0,
                geometry: GeometryProvenance {
                    inline_field: HeaderFieldSpec {
                        name: "INLINE_SYNTHETIC".to_string(),
                        start_byte: 189,
                        value_type: "I32".to_string(),
                    },
                    crossline_field: HeaderFieldSpec {
                        name: "CROSSLINE_SYNTHETIC".to_string(),
                        start_byte: 193,
                        value_type: "I32".to_string(),
                    },
                    third_axis_field: Some(HeaderFieldSpec {
                        name: "OFFSET_SYNTHETIC".to_string(),
                        start_byte: 37,
                        value_type: "I32".to_string(),
                    }),
                },
                regularization: None,
            },
            shape: [1, 1, samples],
            axes: VolumeAxes::from_time_axis(
                vec![1000.0],
                vec![2000.0],
                (0..samples)
                    .map(|value| value as f32 * sample_interval_ms)
                    .collect(),
            ),
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "criterion".to_string(),
            processing_lineage: None,
            segy_export: None,
        },
        SeismicLayout::PreStack3DOffset,
        GatherAxisKind::Offset,
        offsets.to_vec(),
    )
}

fn synthetic_prestack_store_data(
    offsets: &[f64],
    samples: usize,
    sample_interval_ms: f32,
    true_velocity: f32,
) -> Vec<f32> {
    synthetic_prestack_gather(offsets, samples, sample_interval_ms, true_velocity).amplitudes
}

criterion_group!(
    benches,
    compute_operator_kernels,
    prestack_kernel_benchmarks,
    compute_storage_preview,
    compute_storage_materialize
);
criterion_main!(benches);
