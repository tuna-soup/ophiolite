use std::path::PathBuf;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ndarray::Array3;
use ophiolite_seismic_runtime::{
    DatasetKind, FrequencyPhaseMode, FrequencyWindowShape, GeometryProvenance, HeaderFieldSpec,
    MaterializeOptions, ProcessingOperation, SourceIdentity, TbvolManifest, VolumeAxes,
    VolumeMetadata, apply_pipeline_to_traces, create_tbvol_store, materialize_volume,
    preview_section_plane, recommended_chunk_shape,
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
    let bandpass_pipeline = [benchmark_bandpass_operation(sample_interval_ms)];
    let bandpass_phase_rotation_pipeline = [
        benchmark_bandpass_operation(sample_interval_ms),
        ProcessingOperation::PhaseRotation {
            angle_degrees: 35.0,
        },
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

fn fixture_manifest(shape: [usize; 3]) -> TbvolManifest {
    TbvolManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://criterion"),
                file_size: (shape[0] * shape[1] * shape[2] * std::mem::size_of::<f32>()) as u64,
                trace_count: (shape[0] * shape[1]) as u64,
                samples_per_trace: shape[2],
                sample_interval_us: 2000,
                sample_format_code: 5,
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
            axes: VolumeAxes {
                ilines: (0..shape[0]).map(|value| value as f64).collect(),
                xlines: (0..shape[1]).map(|value| value as f64).collect(),
                sample_axis_ms: (0..shape[2]).map(|value| value as f32 * 2.0).collect(),
            },
            coordinate_reference_binding: None,
            spatial: None,
            created_by: "criterion".to_string(),
            processing_lineage: None,
        },
        recommended_chunk_shape(shape, 8),
        false,
    )
}

criterion_group!(
    benches,
    compute_operator_kernels,
    compute_storage_preview,
    compute_storage_materialize
);
criterion_main!(benches);
