from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from ophiolite_sdk.seismic import (
    ConstantVelocity,
    GatherPipeline,
    GatherSelection,
    PostStackNeighborhoodPipeline,
    PostStackNeighborhoodWindow,
    SectionSelection,
    SubvolumePipeline,
    TraceLocalPipeline,
    TraceBoostApp,
    TraceProcessingPipeline,
    VelocityAutopick,
    VelocityScanSpec,
)
from ophiolite_sdk.models import SurveySummary
from ophiolite_sdk.project import Project
from ophiolite_sdk.surveys import Survey


def small_segy_fixture() -> Path | None:
    candidates = (
        Path("/Users/sc/dev/ophiolite/test-data/small.sgy"),
        Path("/Users/sc/dev/ophiolite/test_data/small.sgy"),
        Path("/Users/sc/dev/TraceBoost/test-data/small.sgy"),
    )
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return None


class TraceProcessingPipelineTests(unittest.TestCase):
    def test_trace_local_alias_preserves_existing_pipeline_type(self) -> None:
        self.assertIs(TraceLocalPipeline, TraceProcessingPipeline)

    def test_pipeline_builder_emits_domain_first_payload(self) -> None:
        pipeline = (
            TraceProcessingPipeline.named(
                "Bandpass + RMS AGC",
                description="Golden-path seismic processing surface.",
            )
            .bandpass(8.0, 12.0, 45.0, 60.0)
            .agc_rms(40.0)
        )

        self.assertEqual(pipeline.operator_ids(), ("bandpass_filter", "agc_rms"))
        self.assertEqual(
            pipeline.to_payload(),
            {
                "schema_version": 2,
                "revision": 1,
                "name": "Bandpass + RMS AGC",
                "description": "Golden-path seismic processing surface.",
                "steps": [
                    {
                        "operation": {
                            "bandpass_filter": {
                                "f1_hz": 8.0,
                                "f2_hz": 12.0,
                                "f3_hz": 45.0,
                                "f4_hz": 60.0,
                                "phase": "zero",
                                "window": "cosine_taper",
                            }
                        },
                        "checkpoint": False,
                    },
                    {
                        "operation": {"agc_rms": {"window_ms": 40.0}},
                        "checkpoint": False,
                    },
                ],
            },
        )

    def test_subvolume_pipeline_emits_typed_payload(self) -> None:
        trace_local = TraceLocalPipeline.named("Prefix").agc_rms(40.0)
        pipeline = SubvolumePipeline.crop(
            inline_min=100,
            inline_max=140,
            xline_min=200,
            xline_max=280,
            z_min_ms=800.0,
            z_max_ms=1600.0,
            trace_local_pipeline=trace_local,
            name="Crop Window",
            description="Subvolume extraction with a trace-local prefix.",
        )

        self.assertEqual(pipeline.operator_ids(), ("crop",))
        self.assertEqual(
            pipeline.to_payload(),
            {
                "schema_version": 2,
                "revision": 1,
                "name": "Crop Window",
                "description": "Subvolume extraction with a trace-local prefix.",
                "trace_local_pipeline": {
                    "schema_version": 2,
                    "revision": 1,
                    "name": "Prefix",
                    "steps": [
                        {
                            "operation": {"agc_rms": {"window_ms": 40.0}},
                            "checkpoint": False,
                        }
                    ],
                },
                "crop": {
                    "inline_min": 100,
                    "inline_max": 140,
                    "xline_min": 200,
                    "xline_max": 280,
                    "z_min_ms": 800.0,
                    "z_max_ms": 1600.0,
                },
            },
        )

    def test_gather_pipeline_emits_typed_payload(self) -> None:
        trace_local = TraceLocalPipeline.named("Prefix").bandpass(8.0, 12.0, 45.0, 60.0)
        pipeline = (
            GatherPipeline.named(
                "NMO + mute",
                description="Offset gather conditioning.",
                trace_local_pipeline=trace_local,
            )
            .nmo_correction(ConstantVelocity(velocity_m_per_s=2200.0))
            .stretch_mute(
                ConstantVelocity(velocity_m_per_s=2200.0),
                max_stretch_ratio=0.35,
            )
            .offset_mute(min_offset=300.0, max_offset=2400.0)
        )

        self.assertEqual(
            pipeline.to_payload(),
            {
                "schema_version": 2,
                "revision": 1,
                "name": "NMO + mute",
                "description": "Offset gather conditioning.",
                "trace_local_pipeline": {
                    "schema_version": 2,
                    "revision": 1,
                    "name": "Prefix",
                    "steps": [
                        {
                            "operation": {
                                "bandpass_filter": {
                                    "f1_hz": 8.0,
                                    "f2_hz": 12.0,
                                    "f3_hz": 45.0,
                                    "f4_hz": 60.0,
                                    "phase": "zero",
                                    "window": "cosine_taper",
                                }
                            },
                            "checkpoint": False,
                        }
                    ],
                },
                "operations": [
                    {
                        "nmo_correction": {
                            "velocity_model": {
                                "constant_velocity": {"velocity_m_per_s": 2200.0}
                            },
                            "interpolation": "linear",
                        }
                    },
                    {
                        "stretch_mute": {
                            "velocity_model": {
                                "constant_velocity": {"velocity_m_per_s": 2200.0}
                            },
                            "max_stretch_ratio": 0.35,
                        }
                    },
                    {
                        "offset_mute": {
                            "min_offset": 300.0,
                            "max_offset": 2400.0,
                        }
                    },
                ],
            },
        )

    def test_velocity_scan_spec_emits_canonical_request_payload(self) -> None:
        gather = GatherSelection.inline_xline(1200, 3400)
        spec = VelocityScanSpec(
            min_velocity_m_per_s=1500.0,
            max_velocity_m_per_s=4200.0,
            velocity_step_m_per_s=100.0,
            trace_local_pipeline=TraceLocalPipeline.named("Prefix").agc_rms(50.0),
            autopick=VelocityAutopick(
                sample_stride=2,
                min_semblance=0.45,
                smoothing_samples=3,
                min_time_ms=300.0,
            ),
        )

        self.assertEqual(
            spec.to_payload(
                store_path=Path("survey.tbgath"),
                gather=gather,
                dataset_id="dataset-1",
            ),
            {
                "schema_version": 1,
                "store_path": "survey.tbgath",
                "gather": {
                    "dataset_id": "dataset-1",
                    "selector": {"inline_xline": {"inline": 1200, "xline": 3400}},
                },
                "min_velocity_m_per_s": 1500.0,
                "max_velocity_m_per_s": 4200.0,
                "velocity_step_m_per_s": 100.0,
                "trace_local_pipeline": {
                    "schema_version": 2,
                    "revision": 1,
                    "name": "Prefix",
                    "steps": [
                        {
                            "operation": {"agc_rms": {"window_ms": 50.0}},
                            "checkpoint": False,
                        }
                    ],
                },
                "autopick": {
                    "sample_stride": 2,
                    "min_semblance": 0.45,
                    "smoothing_samples": 3,
                    "min_time_ms": 300.0,
                },
            },
        )

    def test_velocity_scan_spec_emits_project_asset_request_payload(self) -> None:
        gather = GatherSelection.inline_xline(1200, 3400)
        spec = VelocityScanSpec(
            min_velocity_m_per_s=1500.0,
            max_velocity_m_per_s=4200.0,
            velocity_step_m_per_s=100.0,
            trace_local_pipeline=TraceLocalPipeline.named("Prefix").agc_rms(50.0),
        )

        self.assertEqual(
            spec.to_project_payload(source_asset_id="survey-asset-1", gather=gather),
            {
                "source_asset_id": "survey-asset-1",
                "gather": {"selector": {"inline_xline": {"inline": 1200, "xline": 3400}}},
                "min_velocity_m_per_s": 1500.0,
                "max_velocity_m_per_s": 4200.0,
                "velocity_step_m_per_s": 100.0,
                "trace_local_pipeline": {
                    "schema_version": 2,
                    "revision": 1,
                    "name": "Prefix",
                    "steps": [
                        {
                            "operation": {"agc_rms": {"window_ms": 50.0}},
                            "checkpoint": False,
                        }
                    ],
                },
            },
        )

    def test_post_stack_neighborhood_pipeline_emits_typed_payload(self) -> None:
        pipeline = (
            PostStackNeighborhoodPipeline.named(
                "Similarity",
                description="Neighborhood similarity preview.",
                trace_local_pipeline=TraceLocalPipeline.named("Prefix").agc_rms(20.0),
            )
            .similarity(PostStackNeighborhoodWindow(gate_ms=24.0, inline_stepout=1, xline_stepout=2))
            .dip(
                PostStackNeighborhoodWindow(gate_ms=16.0, inline_stepout=1, xline_stepout=1),
                output="inline",
            )
        )

        self.assertEqual(
            pipeline.to_payload(),
            {
                "schema_version": 2,
                "revision": 1,
                "name": "Similarity",
                "description": "Neighborhood similarity preview.",
                "trace_local_pipeline": {
                    "schema_version": 2,
                    "revision": 1,
                    "name": "Prefix",
                    "steps": [
                        {
                            "operation": {"agc_rms": {"window_ms": 20.0}},
                            "checkpoint": False,
                        }
                    ],
                },
                "operations": [
                    {
                        "similarity": {
                            "window": {
                                "gate_ms": 24.0,
                                "inline_stepout": 1,
                                "xline_stepout": 2,
                            }
                        }
                    },
                    {
                        "dip": {
                            "window": {
                                "gate_ms": 16.0,
                                "inline_stepout": 1,
                                "xline_stepout": 1,
                            },
                            "output": "inline",
                        }
                    },
                ],
            },
        )


class _FakeProjectApp:
    def __init__(self) -> None:
        self.last_request: dict[str, object] | None = None

    def preview_project_trace_local_processing(self, _project_root: str, request: dict[str, object]):
        self.last_request = request
        return {
            "preview": {
                "section": {
                    "dataset_id": "dataset-1",
                    "axis": "inline",
                    "coordinate": {"index": 12, "value": 12.0},
                    "traces": 10,
                    "samples": 20,
                },
                "processing_label": "Trace Local",
                "preview_ready": True,
            },
            "pipeline": request["pipeline"],
        }

    def run_project_trace_local_processing(self, _project_root: str, request: dict[str, object]):
        self.last_request = request
        return {
            "resolution": {
                "status": "bound",
                "well_id": "project-archive-well",
                "wellbore_id": "project-archive-wellbore",
                "created_well": False,
                "created_wellbore": False,
            },
            "collection": {"id": "collection-1"},
            "asset": {"id": "asset-2"},
        }

    def run_project_velocity_scan(self, _project_root: str, request: dict[str, object]):
        self.last_request = request
        return {
            "gather": request["gather"],
            "panel": {
                "velocities_m_per_s": [1500.0, 1600.0],
                "sample_axis_ms": [0.0, 4.0],
                "semblance_f32le": [],
            },
            "processing_label": "Velocity Scan",
            "autopicked_velocity_function": None,
        }


class ProjectBoundSurveyTests(unittest.TestCase):
    def test_survey_processing_uses_project_asset_id_requests(self) -> None:
        project = Project(root=Path("/tmp/project"), app=_FakeProjectApp())
        survey = Survey(
            project=project,
            summary_data=SurveySummary(
                asset_id="survey-asset-1",
                logical_asset_id="logical-1",
                collection_id="collection-1",
                name="Survey",
                status="current",
                owner_scope="survey",
                owner_id="survey-owner",
                owner_name="Survey Owner",
                well_id="well-1",
                well_name="Well",
                wellbore_id="wellbore-1",
                wellbore_name="Wellbore",
                effective_coordinate_reference_id=None,
                effective_coordinate_reference_name=None,
            ),
        )

        preview = survey.preview_processing(
            SectionSelection.inline(12),
            TraceLocalPipeline.named("Prefix").agc_rms(40.0),
        )
        self.assertTrue(preview.preview_ready)
        self.assertEqual(project.app.last_request["source_asset_id"], "survey-asset-1")
        self.assertEqual(
            project.app.last_request["section"],
            {"axis": "inline", "index": 12},
        )
        self.assertNotIn("store_path", project.app.last_request)

    def test_survey_velocity_scan_uses_project_asset_id_requests(self) -> None:
        project = Project(root=Path("/tmp/project"), app=_FakeProjectApp())
        survey = Survey(
            project=project,
            summary_data=SurveySummary(
                asset_id="survey-asset-1",
                logical_asset_id="logical-1",
                collection_id="collection-1",
                name="Survey",
                status="current",
                owner_scope="survey",
                owner_id="survey-owner",
                owner_name="Survey Owner",
                well_id="well-1",
                well_name="Well",
                wellbore_id="wellbore-1",
                wellbore_name="Wellbore",
                effective_coordinate_reference_id=None,
                effective_coordinate_reference_name=None,
            ),
        )

        result = survey.velocity_scan(
            GatherSelection.ordinal(3),
            VelocityScanSpec(
                min_velocity_m_per_s=1500.0,
                max_velocity_m_per_s=2200.0,
                velocity_step_m_per_s=100.0,
            ),
        )
        self.assertEqual(result.processing_label, "Velocity Scan")
        self.assertEqual(project.app.last_request["source_asset_id"], "survey-asset-1")
        self.assertEqual(project.app.last_request["gather"], {"selector": {"ordinal": {"index": 3}}})
        self.assertNotIn("store_path", project.app.last_request)


@unittest.skipIf(small_segy_fixture() is None, "small.sgy fixture not available")
class SeismicSdkIntegrationTests(unittest.TestCase):
    def test_import_preview_and_materialize_flow(self) -> None:
        fixture = small_segy_fixture()
        assert fixture is not None

        app = TraceBoostApp()
        with tempfile.TemporaryDirectory(prefix="ophiolite_seismic_sdk_") as temp_root:
            root = Path(temp_root)
            store_path = root / "input.tbvol"
            processed_store_path = root / "input_bandpass_agc.tbvol"

            preflight = app.preflight_import(fixture)
            self.assertEqual(preflight.layout, "post_stack_3d")

            dataset = app.import_segy(
                fixture,
                store_path,
                overwrite_existing=True,
                preflight=preflight,
            )
            self.assertEqual(dataset.descriptor.shape, (5, 5, 50))
            self.assertAlmostEqual(dataset.descriptor.sample_interval_ms, 4.0)

            selection = dataset.midpoint_section(axis="inline")
            self.assertEqual(selection.axis, "inline")
            self.assertEqual(selection.index, 2)

            raw_section = dataset.section(selection)
            self.assertEqual(raw_section.traces, 5)
            self.assertEqual(raw_section.samples, 50)

            pipeline = (
                TraceProcessingPipeline.named(
                    "Bandpass + RMS AGC",
                    description="Golden-path seismic processing surface.",
                )
                .bandpass(8.0, 12.0, 45.0, 60.0)
                .agc_rms(40.0)
            )
            preview = dataset.preview_processing(selection, pipeline)
            self.assertTrue(preview.preview_ready)
            self.assertEqual(preview.processing_label, "Bandpass + RMS AGC")

            processed = dataset.run_processing(
                pipeline,
                output_store_path=processed_store_path,
                overwrite_existing=True,
            )
            self.assertEqual(processed.descriptor.shape, (5, 5, 50))
            self.assertIsNotNone(processed.descriptor.processing_lineage_summary)
            assert processed.descriptor.processing_lineage_summary is not None
            self.assertEqual(
                processed.descriptor.processing_lineage_summary["pipeline_name"],
                "Bandpass + RMS AGC",
            )

            processed_section = processed.section(selection)
            self.assertEqual(processed_section.traces, 5)
            self.assertEqual(processed_section.samples, 50)


if __name__ == "__main__":
    unittest.main()
