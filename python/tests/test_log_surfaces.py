from __future__ import annotations

import unittest

from ophiolite_sdk.avo import LayeringSpec
from ophiolite_sdk.logs import WellTopSet


class WellTopSetSelectorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.top_set = WellTopSet.from_panel_json(
            {
                "asset_id": "asset-1",
                "logical_asset_id": "logical-1",
                "asset_name": "lithostrat-tops",
                "set_kind": "top_set",
                "rows": [
                    {"name": "Alpha", "top_depth": 1000.0, "base_depth": 1010.0},
                    {"name": "Beta", "top_depth": 1010.0, "base_depth": 1020.0},
                    {"name": "Beta", "top_depth": 1020.0, "base_depth": 1030.0},
                    {"name": "Gamma", "top_depth": 1030.0, "base_depth": 1040.0},
                ],
            }
        )

    def test_interval_selectors_disambiguate_repeated_labels(self) -> None:
        self.assertEqual(
            self.top_set.interval_selectors,
            ("Alpha", "Beta#1", "Beta#2", "Gamma"),
        )

    def test_select_intervals_by_selector_returns_exact_matches(self) -> None:
        selected = self.top_set.select_intervals(selectors=["Beta#2", "Gamma"])
        self.assertEqual([interval.label for interval in selected], ["Beta", "Gamma"])
        self.assertEqual(
            [(interval.top_depth_m, interval.base_depth_m) for interval in selected],
            [(1020.0, 1030.0), (1030.0, 1040.0)],
        )

    def test_select_intervals_by_label_keeps_all_matching_occurrences(self) -> None:
        selected = self.top_set.select_intervals(labels=["Beta"])
        self.assertEqual(len(selected), 2)
        self.assertTrue(all(interval.label == "Beta" for interval in selected))

    def test_layering_from_top_set_preserves_selectors(self) -> None:
        layering = self.top_set.layering(selectors=["Beta#1", "Beta#2"])
        self.assertEqual(layering.kind, "top_set")
        self.assertEqual(layering.asset_name, "lithostrat-tops")
        self.assertEqual(layering.selectors, ("Beta#1", "Beta#2"))
        self.assertEqual(layering.labels, ())

    def test_label_and_selector_filters_are_mutually_exclusive(self) -> None:
        with self.assertRaises(ValueError):
            self.top_set.select_intervals(labels=["Alpha"], selectors=["Beta#1"])

        with self.assertRaises(ValueError):
            LayeringSpec.from_top_set(
                "lithostrat-tops",
                labels=["Alpha"],
                selectors=["Beta#1"],
            )


if __name__ == "__main__":
    unittest.main()
