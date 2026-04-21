from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from ophiolite_sdk.seismic import TraceBoostApp, TraceProcessingPipeline


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
