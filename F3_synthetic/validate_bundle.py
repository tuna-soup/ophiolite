#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


EXPECTED_TRANSFORM_ID = "f3-paired-horizon-survey-transform"
EXPECTED_TIME_HORIZONS = [f"horizon_{index:02d}_twt_ms" for index in range(1, 5)]
EXPECTED_DEPTH_HORIZONS = [f"horizon_{index:02d}_depth_m" for index in range(1, 5)]
EXPECTED_DERIVED_DEPTH_HORIZONS = [
    f"horizon_{index:02d}_twt_ms-derived_depth_m" for index in range(1, 5)
]
EXPECTED_DERIVED_TIME_HORIZONS = [
    f"horizon_{index:02d}_depth_m-derived_twt_ms" for index in range(1, 5)
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate the local F3 synthetic bundle, linked store, and stored reports."
    )
    parser.add_argument(
        "--bundle-root",
        type=Path,
        default=Path(__file__).resolve().parent,
    )
    parser.add_argument("--output-json", type=Path)
    return parser.parse_args()


def read_json(path: Path) -> dict:
    return json.loads(path.read_text())


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def main() -> int:
    args = parse_args()
    bundle_root = args.bundle_root.resolve()
    failures: list[str] = []

    bundle_files = {
        "README.md": bundle_root / "README.md",
        "benchmark_report.json": bundle_root / "benchmark_report.json",
        "benchmark_report.md": bundle_root / "benchmark_report.md",
        "derived_horizon_conversion_report.json": bundle_root / "derived_horizon_conversion_report.json",
        "derived_horizon_conversion_report.md": bundle_root / "derived_horizon_conversion_report.md",
        "survey_package": bundle_root / "survey_package",
        "f3_store": bundle_root / "f3_dataset_regularized.tbvol",
        "depth_cube_manifest": bundle_root / "depth_velocity_cube" / "manifest.json",
        "depth_cube_values": bundle_root / "depth_velocity_cube" / "interval_velocity.values.f32le.bin",
        "depth_cube_validity": bundle_root / "depth_velocity_cube" / "interval_velocity.validity.u8.bin",
    }
    for label, path in bundle_files.items():
        require(path.exists(), f"missing required bundle artifact: {label} -> {path}", failures)

    survey_package = bundle_files["survey_package"]
    store_root = bundle_files["f3_store"]
    if survey_package.exists():
        require(survey_package.is_dir(), f"survey package is not a directory: {survey_package}", failures)
        for relative in ["survey_spec.json", "Velocity_functions.txt"]:
            require(
                (survey_package / relative).exists(),
                f"missing survey package file: {(survey_package / relative)}",
                failures,
            )
        for horizon_id in range(1, 5):
            for suffix in ("twt_ms", "depth_m", "depth_ft"):
                file_path = survey_package / f"horizon_{horizon_id:02d}_{suffix}.xyz"
                require(file_path.exists(), f"missing survey package horizon file: {file_path}", failures)

    if store_root.exists():
        require(store_root.is_dir(), f"store path is not a directory: {store_root}", failures)

    transform_manifest_path = store_root / "time-depth-transforms" / "manifest.json"
    horizons_manifest_path = store_root / "horizons" / "manifest.json"
    require(transform_manifest_path.exists(), f"missing transform manifest: {transform_manifest_path}", failures)
    require(horizons_manifest_path.exists(), f"missing horizons manifest: {horizons_manifest_path}", failures)

    transform_descriptor = None
    if transform_manifest_path.exists():
        transform_manifest = read_json(transform_manifest_path)
        for item in transform_manifest.get("transforms", []):
            descriptor = item.get("descriptor", {})
            if descriptor.get("id") == EXPECTED_TRANSFORM_ID:
                transform_descriptor = descriptor
                require(
                    (store_root / "time-depth-transforms" / item["depths_file"]).exists(),
                    f"missing transform depths payload for {EXPECTED_TRANSFORM_ID}",
                    failures,
                )
                require(
                    (store_root / "time-depth-transforms" / item["validity_file"]).exists(),
                    f"missing transform validity payload for {EXPECTED_TRANSFORM_ID}",
                    failures,
                )
                break
        require(transform_descriptor is not None, f"missing transform descriptor: {EXPECTED_TRANSFORM_ID}", failures)

    if transform_descriptor is not None:
        require(transform_descriptor.get("source_kind") == "horizon_layer_model", "unexpected transform source_kind", failures)
        time_axis = transform_descriptor.get("time_axis", {})
        require(time_axis.get("domain") == "time", "unexpected transform time_axis domain", failures)
        require(time_axis.get("unit") == "ms", "unexpected transform time_axis unit", failures)
        require(abs(float(time_axis.get("start", -1.0)) - 4.0) <= 1e-6, "unexpected transform time_axis start", failures)
        require(abs(float(time_axis.get("step", -1.0)) - 4.0) <= 1e-6, "unexpected transform time_axis step", failures)
        require(int(time_axis.get("count", -1)) == 462, "unexpected transform sample count in time axis", failures)
        require(int(transform_descriptor.get("inline_count", -1)) == 651, "unexpected transform inline_count", failures)
        require(int(transform_descriptor.get("xline_count", -1)) == 951, "unexpected transform xline_count", failures)
        require(int(transform_descriptor.get("sample_count", -1)) == 462, "unexpected transform sample_count", failures)

    horizon_entries: dict[str, dict] = {}
    if horizons_manifest_path.exists():
        horizons_manifest = read_json(horizons_manifest_path)
        horizon_entries = {item["id"]: item for item in horizons_manifest.get("horizons", [])}
        for horizon_id in (
            EXPECTED_TIME_HORIZONS
            + EXPECTED_DEPTH_HORIZONS
            + EXPECTED_DERIVED_DEPTH_HORIZONS
            + EXPECTED_DERIVED_TIME_HORIZONS
        ):
            require(horizon_id in horizon_entries, f"missing expected horizon id: {horizon_id}", failures)
        for horizon_id in EXPECTED_DERIVED_DEPTH_HORIZONS + EXPECTED_DERIVED_TIME_HORIZONS:
            entry = horizon_entries.get(horizon_id)
            if not entry:
                continue
            require(
                str(entry.get("source_path", "")).startswith("derived://horizon-conversion/"),
                f"derived horizon has unexpected source_path: {horizon_id}",
                failures,
            )
            require(
                (store_root / "horizons" / entry["values_file"]).exists(),
                f"missing values payload for derived horizon: {horizon_id}",
                failures,
            )
            require(
                (store_root / "horizons" / entry["validity_file"]).exists(),
                f"missing validity payload for derived horizon: {horizon_id}",
                failures,
            )

    depth_cube_manifest = None
    if bundle_files["depth_cube_manifest"].exists():
        depth_cube_manifest = read_json(bundle_files["depth_cube_manifest"])
        require(
            depth_cube_manifest.get("source_transform_id") == EXPECTED_TRANSFORM_ID,
            "unexpected depth cube source_transform_id",
            failures,
        )
        axis = depth_cube_manifest.get("depth_axis", {})
        require(axis.get("domain") == "depth", "unexpected depth cube axis domain", failures)
        require(axis.get("unit") == "m", "unexpected depth cube axis unit", failures)
        require(abs(float(axis.get("start", -1.0)) - 0.0) <= 1e-6, "unexpected depth cube axis start", failures)
        require(abs(float(axis.get("step", -1.0)) - 50.0) <= 1e-6, "unexpected depth cube axis step", failures)
        require(int(axis.get("count", -1)) == 51, "unexpected depth cube axis count", failures)
        require(int(depth_cube_manifest.get("invalid_trace_count", -1)) == 0, "unexpected depth cube invalid_trace_count", failures)

    benchmark_report = read_json(bundle_files["benchmark_report.json"])
    benchmark_summary = benchmark_report["conversion_summary"]
    require(
        float(benchmark_summary["twt_to_depth_m"]["full_grid"]["mean_rmse"]) <= 0.11,
        "benchmark TWT->depth mean RMSE exceeded threshold",
        failures,
    )
    require(
        float(benchmark_summary["depth_to_twt_ms"]["full_grid"]["mean_rmse"]) <= 0.10,
        "benchmark depth->TWT mean RMSE exceeded threshold",
        failures,
    )

    derived_report = read_json(bundle_files["derived_horizon_conversion_report.json"])
    derived_summary = derived_report["summary"]
    require(
        int(derived_summary["twt_to_depth_m"]["total_invalid_cell_count"]) == 0,
        "derived report TWT->depth invalid cells are nonzero",
        failures,
    )
    require(
        int(derived_summary["depth_to_twt_ms"]["total_invalid_cell_count"]) == 0,
        "derived report depth->TWT invalid cells are nonzero",
        failures,
    )
    require(
        float(derived_summary["twt_to_depth_m"]["mean_rmse"]) <= 0.11,
        "derived report TWT->depth mean RMSE exceeded threshold",
        failures,
    )
    require(
        float(derived_summary["depth_to_twt_ms"]["mean_rmse"]) <= 0.10,
        "derived report depth->TWT mean RMSE exceeded threshold",
        failures,
    )

    result = {
        "bundle_root": str(bundle_root),
        "store_root": str(store_root),
        "survey_package": str(survey_package),
        "expected_transform_id": EXPECTED_TRANSFORM_ID,
        "checked_horizon_count": len(horizon_entries),
        "status": "ok" if not failures else "failed",
        "failures": failures,
    }

    payload = json.dumps(result, indent=2) + "\n"
    if args.output_json:
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(payload, encoding="utf-8")
    print(payload, end="")
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
