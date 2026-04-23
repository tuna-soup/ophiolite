use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use ndarray::Array3;
use ophiolite_seismic_runtime::{
    DatasetKind, GeometryProvenance, HeaderFieldSpec, LocalVolumeStatistic, NeighborhoodDipOutput,
    PostStackNeighborhoodProcessingOperation, PostStackNeighborhoodProcessingPipeline,
    PostStackNeighborhoodWindow, SectionAxis, SourceIdentity, TbvolManifest, VolumeAxes,
    VolumeMetadata, create_tbvol_store, generate_store_id,
    preview_post_stack_neighborhood_processing_section_view, recommended_chunk_shape,
    segy_sample_data_fidelity,
};
use tempfile::TempDir;

// Use one neutral fixture for apples-to-apples axis comparisons and one
// asymmetric fixture to preserve the "inline section is wider than xline"
// behavior seen in real surveys such as F3.
const SYMMETRIC_FIXTURE_SHAPE: [usize; 3] = [128, 128, 512];
const ASYMMETRIC_FIXTURE_SHAPE: [usize; 3] = [96, 128, 512];

fn dip_kernels(c: &mut Criterion) {
    let symmetric_fixture = BenchmarkFixture::create(
        "post-stack-neighborhood-bench-symmetric",
        SYMMETRIC_FIXTURE_SHAPE,
    );
    let asymmetric_fixture = BenchmarkFixture::create(
        "post-stack-neighborhood-bench-asymmetric",
        ASYMMETRIC_FIXTURE_SHAPE,
    );
    let inline_pipeline = neighborhood_pipeline(
        "Dip Balanced Dense Inline",
        PostStackNeighborhoodProcessingOperation::Dip {
            window: neighborhood_window(32.0, 2, 2),
            output: NeighborhoodDipOutput::Inline,
        },
    );
    let xline_pipeline = neighborhood_pipeline(
        "Dip Balanced Dense Xline",
        PostStackNeighborhoodProcessingOperation::Dip {
            window: neighborhood_window(32.0, 2, 2),
            output: NeighborhoodDipOutput::Xline,
        },
    );

    let inline_baseline = preview_post_stack_neighborhood_processing_section_view(
        &symmetric_fixture.store_root,
        SectionAxis::Inline,
        symmetric_fixture.inline_index,
        &inline_pipeline,
    )
    .expect("inline dip baseline should succeed");
    assert_valid_view(&inline_baseline);

    let xline_baseline = preview_post_stack_neighborhood_processing_section_view(
        &symmetric_fixture.store_root,
        SectionAxis::Xline,
        symmetric_fixture.xline_index,
        &xline_pipeline,
    )
    .expect("xline dip baseline should succeed");
    assert_valid_view(&xline_baseline);

    let mut group = c.benchmark_group("dip_kernels");
    group.bench_function("dip_balanced_dense_inline_symmetric", |b| {
        b.iter(|| {
            let view = preview_post_stack_neighborhood_processing_section_view(
                &symmetric_fixture.store_root,
                SectionAxis::Inline,
                symmetric_fixture.inline_index,
                &inline_pipeline,
            )
            .expect("inline dip preview should succeed");
            std::hint::black_box(view);
        })
    });
    group.bench_function("dip_balanced_dense_xline_symmetric", |b| {
        b.iter(|| {
            let view = preview_post_stack_neighborhood_processing_section_view(
                &symmetric_fixture.store_root,
                SectionAxis::Xline,
                symmetric_fixture.xline_index,
                &xline_pipeline,
            )
            .expect("xline dip preview should succeed");
            std::hint::black_box(view);
        })
    });
    group.bench_function("dip_balanced_dense_inline_asymmetric", |b| {
        b.iter(|| {
            let view = preview_post_stack_neighborhood_processing_section_view(
                &asymmetric_fixture.store_root,
                SectionAxis::Inline,
                asymmetric_fixture.inline_index,
                &inline_pipeline,
            )
            .expect("inline asymmetric dip preview should succeed");
            std::hint::black_box(view);
        })
    });
    group.bench_function("dip_balanced_dense_xline_asymmetric", |b| {
        b.iter(|| {
            let view = preview_post_stack_neighborhood_processing_section_view(
                &asymmetric_fixture.store_root,
                SectionAxis::Xline,
                asymmetric_fixture.xline_index,
                &xline_pipeline,
            )
            .expect("xline asymmetric dip preview should succeed");
            std::hint::black_box(view);
        })
    });
    group.finish();
}

fn similarity_kernels(c: &mut Criterion) {
    let fixture = BenchmarkFixture::create(
        "post-stack-neighborhood-bench-symmetric",
        SYMMETRIC_FIXTURE_SHAPE,
    );
    let tight_pipeline = neighborhood_pipeline(
        "Similarity Tight Dense",
        PostStackNeighborhoodProcessingOperation::Similarity {
            window: neighborhood_window(24.0, 1, 1),
        },
    );
    let balanced_pipeline = neighborhood_pipeline(
        "Similarity Balanced Dense",
        PostStackNeighborhoodProcessingOperation::Similarity {
            window: neighborhood_window(32.0, 2, 2),
        },
    );

    let baseline = preview_post_stack_neighborhood_processing_section_view(
        &fixture.store_root,
        SectionAxis::Inline,
        fixture.inline_index,
        &tight_pipeline,
    )
    .expect("tight similarity baseline should succeed");
    assert_valid_view(&baseline);

    let mut group = c.benchmark_group("similarity_kernels");
    group.bench_function("similarity_tight_dense", |b| {
        b.iter(|| {
            let view = preview_post_stack_neighborhood_processing_section_view(
                &fixture.store_root,
                SectionAxis::Inline,
                fixture.inline_index,
                &tight_pipeline,
            )
            .expect("tight similarity preview should succeed");
            std::hint::black_box(view);
        })
    });
    group.bench_function("similarity_balanced_dense", |b| {
        b.iter(|| {
            let view = preview_post_stack_neighborhood_processing_section_view(
                &fixture.store_root,
                SectionAxis::Inline,
                fixture.inline_index,
                &balanced_pipeline,
            )
            .expect("balanced similarity preview should succeed");
            std::hint::black_box(view);
        })
    });
    group.finish();
}

fn local_volume_stats_kernels(c: &mut Criterion) {
    let fixture = BenchmarkFixture::create(
        "post-stack-neighborhood-bench-symmetric",
        SYMMETRIC_FIXTURE_SHAPE,
    );
    let mut group = c.benchmark_group("local_volume_stats_kernels");

    for (name, statistic) in [
        ("local_stats_mean", LocalVolumeStatistic::Mean),
        ("local_stats_rms", LocalVolumeStatistic::Rms),
        ("local_stats_variance", LocalVolumeStatistic::Variance),
        ("local_stats_minimum", LocalVolumeStatistic::Minimum),
        ("local_stats_maximum", LocalVolumeStatistic::Maximum),
    ] {
        let pipeline = neighborhood_pipeline(
            name,
            PostStackNeighborhoodProcessingOperation::LocalVolumeStats {
                window: neighborhood_window(32.0, 2, 2),
                statistic,
            },
        );
        let baseline = preview_post_stack_neighborhood_processing_section_view(
            &fixture.store_root,
            SectionAxis::Inline,
            fixture.inline_index,
            &pipeline,
        )
        .expect("local stats baseline should succeed");
        assert_valid_view(&baseline);

        group.bench_function(name, |b| {
            b.iter(|| {
                let view = preview_post_stack_neighborhood_processing_section_view(
                    &fixture.store_root,
                    SectionAxis::Inline,
                    fixture.inline_index,
                    &pipeline,
                )
                .expect("local stats preview should succeed");
                std::hint::black_box(view);
            })
        });
    }

    group.finish();
}

struct BenchmarkFixture {
    _temp: TempDir,
    store_root: PathBuf,
    inline_index: usize,
    xline_index: usize,
}

impl BenchmarkFixture {
    fn create(name: &str, shape: [usize; 3]) -> Self {
        let temp = TempDir::new().expect("temp dir");
        let store_root = temp.path().join(format!("{name}.tbvol"));
        let manifest = fixture_manifest(shape);
        let data = synthetic_neighborhood_data(shape);
        create_tbvol_store(&store_root, manifest, &data, None).expect("fixture store");
        Self {
            _temp: temp,
            store_root,
            inline_index: shape[0] / 2,
            xline_index: shape[1] / 2,
        }
    }
}

fn neighborhood_window(
    gate_ms: f32,
    inline_stepout: usize,
    xline_stepout: usize,
) -> PostStackNeighborhoodWindow {
    PostStackNeighborhoodWindow {
        gate_ms,
        inline_stepout,
        xline_stepout,
    }
}

fn neighborhood_pipeline(
    name: &str,
    operation: PostStackNeighborhoodProcessingOperation,
) -> PostStackNeighborhoodProcessingPipeline {
    PostStackNeighborhoodProcessingPipeline {
        schema_version: 1,
        revision: 1,
        preset_id: None,
        name: Some(name.to_string()),
        description: None,
        trace_local_pipeline: None,
        operations: vec![operation],
    }
}

fn assert_valid_view(view: &ophiolite_seismic_runtime::SectionView) {
    assert!(view.traces > 0);
    assert!(view.samples > 0);
    assert_eq!(
        view.amplitudes_f32le.len(),
        view.traces * view.samples * std::mem::size_of::<f32>()
    );
    assert_eq!(
        view.sample_axis_f32le.len(),
        view.samples * std::mem::size_of::<f32>()
    );
}

fn synthetic_neighborhood_data(shape: [usize; 3]) -> Array3<f32> {
    Array3::from_shape_fn((shape[0], shape[1], shape[2]), |(iline, xline, sample)| {
        let il = iline as f32;
        let xl = xline as f32;
        let t = sample as f32;

        let event_a_center = shape[2] as f32 * 0.22 + (il * 0.45) + (xl * 0.75);
        let event_b_center = shape[2] as f32 * 0.61 - (il * 0.30) + (xl * 0.18);
        let event_c_center = shape[2] as f32 * 0.83 + (il * 0.12) - (xl * 0.28);

        let event_a = gaussian_wavelet(t, event_a_center, 4.0, 1.0);
        let event_b = gaussian_wavelet(t, event_b_center, 6.5, -0.8);
        let event_c = gaussian_wavelet(t, event_c_center, 5.0, 0.6);

        let background = ((il * 0.07).sin() + (xl * 0.05).cos()) * 0.08
            + ((t / shape[2].max(1) as f32) * 23.0).sin() * 0.03;

        event_a + event_b + event_c + background
    })
}

fn gaussian_wavelet(sample: f32, center: f32, width: f32, amplitude: f32) -> f32 {
    let delta = (sample - center) / width.max(f32::EPSILON);
    amplitude * (-delta * delta).exp()
}

fn fixture_manifest(shape: [usize; 3]) -> TbvolManifest {
    TbvolManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Source,
            store_id: generate_store_id(),
            source: SourceIdentity {
                source_path: PathBuf::from("synthetic://post-stack-neighborhood-bench"),
                file_size: (shape[0] * shape[1] * shape[2] * std::mem::size_of::<f32>()) as u64,
                trace_count: (shape[0] * shape[1]) as u64,
                samples_per_trace: shape[2],
                sample_interval_us: 2000,
                sample_format_code: 5,
                sample_data_fidelity: segy_sample_data_fidelity(5),
                endianness: "little".to_string(),
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
            segy_export: None,
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
    dip_kernels,
    similarity_kernels,
    local_volume_stats_kernels
);
criterion_main!(benches);
