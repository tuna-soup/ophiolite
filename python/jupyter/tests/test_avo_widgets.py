from __future__ import annotations

from pathlib import Path
import unittest

from ophiolite_jupyter import (
    AvoInterceptGradientCrossplotWidget,
    AvoResponseWidget,
)


class StubAvoResult:
    def response_source(self, **kwargs):
        title = kwargs.get("title", "Stub Response")
        return {
            "schema_version": 1,
            "id": "stub-response",
            "name": "Stub Response",
            "title": title,
            "x_axis": {"label": "Incidence Angle", "unit": "deg"},
            "y_axis": {"label": "PP Reflectivity", "unit": "ratio"},
            "interfaces": [{"id": "interface-1", "label": "Interface 1", "color": "#4c8bf5"}],
            "series": [
                {
                    "id": "series-1",
                    "interface_id": "interface-1",
                    "label": "Interface 1",
                    "color": "#4c8bf5",
                    "style": "solid",
                    "reflectivity_model": "zoeppritz",
                    "anisotropy_mode": "isotropic",
                    "incidence_angles_deg": [0.0, 10.0, 20.0],
                    "values": [0.08, 0.07, 0.05],
                }
            ],
        }

    def crossplot_source(self, **kwargs):
        title = kwargs.get("title", "Stub Crossplot")
        return {
            "schema_version": 1,
            "id": "stub-crossplot",
            "name": "Stub Crossplot",
            "title": title,
            "x_axis": {"label": "Intercept", "unit": "ratio"},
            "y_axis": {"label": "Gradient", "unit": "ratio"},
            "interfaces": [{"id": "interface-1", "label": "Interface 1", "color": "#4c8bf5"}],
            "points": [
                {
                    "interface_id": "interface-1",
                    "intercept": 0.08,
                    "gradient": -0.12,
                }
            ],
        }


class AvoWidgetTests(unittest.TestCase):
    def test_widget_assets_exist_and_export_default_modules(self):
        for widget_cls in (AvoResponseWidget, AvoInterceptGradientCrossplotWidget):
            esm_path = Path(widget_cls._esm_path)
            self.assertTrue(esm_path.exists(), f"missing widget asset: {esm_path}")
            esm_source = esm_path.read_text()
            self.assertIn("as default", esm_source)
            self.assertNotIn("./assets/", esm_source)

        shared_assets = sorted((Path(AvoResponseWidget._esm_path).parent / "assets").glob("*.js"))
        self.assertTrue(shared_assets, "expected at least one shared widget asset bundle")

    def test_response_widget_from_result_uses_response_source(self):
        widget = AvoResponseWidget.from_result(StubAvoResult(), title="Notebook Response")
        self.assertEqual(widget.source["title"], "Notebook Response")
        self.assertEqual(widget.height_px, 520)

    def test_crossplot_widget_from_result_uses_crossplot_source(self):
        widget = AvoInterceptGradientCrossplotWidget.from_result(StubAvoResult(), title="Notebook Crossplot")
        self.assertEqual(widget.source["title"], "Notebook Crossplot")
        self.assertEqual(widget.height_px, 520)

    def test_fit_to_data_advances_request_counter(self):
        widget = AvoResponseWidget(StubAvoResult().response_source())
        widget.fit_to_data()
        self.assertEqual(widget.fit_request_id, 1)

    def test_invalid_source_shape_fails_fast(self):
        with self.assertRaises(ValueError):
            AvoResponseWidget({"schema_version": 1})


if __name__ == "__main__":
    unittest.main()
