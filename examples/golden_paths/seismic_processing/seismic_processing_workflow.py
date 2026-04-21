#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_SRC = REPO_ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from ophiolite_sdk.seismic import (  # noqa: E402
    SectionSelection,
    TraceBoostApp,
    TraceProcessingPipeline,
)


DEFAULT_SECTION_AXIS = "inline"
DEFAULT_AGC_WINDOW_MS = 40.0


@dataclass(frozen=True)
class RunLayout:
    run_root: Path
    store_path: Path
    processed_store_path: Path
    output_root: Path


def default_segy_candidates() -> list[Path]:
    return [
        REPO_ROOT / "test-data" / "small.sgy",
        REPO_ROOT / "test_data" / "small.sgy",
        REPO_ROOT.parent / "TraceBoost" / "test-data" / "small.sgy",
        Path("/Users/sc/Downloads/SubsurfaceData/blocks/F3/f3_dataset.sgy"),
    ]


def detect_default_segy_path() -> Path | None:
    configured = os.environ.get("OPHIOLITE_SEISMIC_GOLDEN_PATH_SEGY")
    if configured:
        candidate = Path(configured).expanduser()
        return candidate if candidate.exists() else None

    for candidate in default_segy_candidates():
        if candidate.exists():
            return candidate
    return None


def prepare_layout(run_root: str | None, *, overwrite: bool) -> RunLayout:
    if run_root is None:
        root = Path(tempfile.mkdtemp(prefix="ophiolite_seismic_processing_"))
    else:
        root = Path(run_root).expanduser().resolve()
        if root.exists() and overwrite:
            shutil.rmtree(root)
        root.mkdir(parents=True, exist_ok=True)

    output_root = root / "outputs"
    output_root.mkdir(parents=True, exist_ok=True)
    return RunLayout(
        run_root=root,
        store_path=root / "input.tbvol",
        processed_store_path=root / "input_bandpass_agc.tbvol",
        output_root=output_root,
    )


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2)
        handle.write("\n")


def parse_bandpass(text: str) -> tuple[float, float, float, float]:
    values = tuple(float(item.strip()) for item in text.split(",") if item.strip())
    if len(values) != 4:
        raise ValueError("--bandpass-hz must contain exactly four comma-separated values")
    return values


def build_pipeline(
    *,
    bandpass_hz: tuple[float, float, float, float],
    agc_window_ms: float,
) -> TraceProcessingPipeline:
    f1_hz, f2_hz, f3_hz, f4_hz = bandpass_hz
    return (
        TraceProcessingPipeline.named(
            "Bandpass + RMS AGC",
            description=(
                "Trace-local seismic golden path: bandpass filter followed by RMS AGC."
            ),
        )
        .bandpass(f1_hz, f2_hz, f3_hz, f4_hz)
        .agc_rms(agc_window_ms)
    )


def run_workflow(
    *,
    segy_path: Path,
    run_root: str | None,
    overwrite: bool,
    section_axis: str,
    section_index: int | None,
    bandpass_hz: tuple[float, float, float, float],
    agc_window_ms: float,
    traceboost_app_bin: str | None,
) -> dict[str, Any]:
    app = TraceBoostApp(binary=traceboost_app_bin)
    layout = prepare_layout(run_root, overwrite=overwrite)

    preflight = app.preflight_import(segy_path)
    preflight_path = layout.output_root / "preflight.json"
    write_json(preflight_path, preflight.to_payload())

    imported = app.import_segy(
        segy_path,
        layout.store_path,
        overwrite_existing=True,
        preflight=preflight,
    )
    import_path = layout.output_root / "import.json"
    write_json(import_path, imported.to_payload())

    dataset = app.open_dataset(layout.store_path)
    dataset_path = layout.output_root / "dataset.json"
    write_json(dataset_path, dataset.to_payload())

    selection = (
        SectionSelection(axis=section_axis, index=section_index)
        if section_index is not None
        else dataset.midpoint_section(axis=section_axis)
    )
    raw_section = dataset.section(selection)
    raw_section_path = layout.output_root / f"raw_{section_axis}_section.json"
    write_json(raw_section_path, raw_section.to_payload())

    pipeline = build_pipeline(bandpass_hz=bandpass_hz, agc_window_ms=agc_window_ms)
    pipeline_path = layout.output_root / "pipeline.json"
    write_json(pipeline_path, pipeline.to_payload())

    preview_request = {
        "schema_version": 1,
        "store_path": str(dataset.store_path),
        "section": selection.to_payload(dataset_id=dataset.descriptor.id),
        "pipeline": pipeline.to_payload(),
    }
    preview_request_path = layout.output_root / "preview_request.json"
    write_json(preview_request_path, preview_request)

    preview = dataset.preview_processing(selection, pipeline)
    preview_path = layout.output_root / "preview_bandpass_agc_section.json"
    write_json(preview_path, preview.to_payload())

    run_request = {
        "schema_version": 1,
        "store_path": str(dataset.store_path),
        "output_store_path": str(layout.processed_store_path),
        "overwrite_existing": True,
        "pipeline": pipeline.to_payload(),
    }
    run_request_path = layout.output_root / "run_request.json"
    write_json(run_request_path, run_request)

    processed_dataset = dataset.run_processing(
        pipeline,
        output_store_path=layout.processed_store_path,
        overwrite_existing=True,
    )
    processed_dataset_path = layout.output_root / "processed_dataset.json"
    write_json(processed_dataset_path, processed_dataset.to_payload())

    processed_section = processed_dataset.section(selection)
    processed_section_path = layout.output_root / f"processed_{section_axis}_section.json"
    write_json(processed_section_path, processed_section.to_payload())

    summary = {
        "repo_root": str(REPO_ROOT),
        "segy_path": str(segy_path),
        "run_root": str(layout.run_root),
        "store_path": str(layout.store_path),
        "processed_store_path": str(layout.processed_store_path),
        "section": {
            "axis": selection.axis,
            "index": selection.index,
            "coordinate": raw_section.coordinate,
        },
        "preflight": {
            "suggested_action": preflight.suggested_action,
            "layout": preflight.layout,
            "classification": preflight.classification,
            "resolved_geometry": preflight.resolved_geometry.to_payload(),
            "notes": list(preflight.notes),
            "output_path": str(preflight_path),
        },
        "dataset_descriptor": dataset.descriptor.to_payload(),
        "pipeline": {
            "name": pipeline.name,
            "operator_ids": list(pipeline.operator_ids()),
            "request_path": str(run_request_path),
            "preview_request_path": str(preview_request_path),
        },
        "raw_section_stats": raw_section.stats(),
        "preview": {
            "processing_label": preview.processing_label,
            "preview_ready": preview.preview_ready,
            "section_stats": preview.section.stats(),
            "output_path": str(preview_path),
        },
        "processed_dataset_descriptor": processed_dataset.descriptor.to_payload(),
        "processed_section_stats": processed_section.stats(),
        "chart_payloads": {
            "raw_section": str(raw_section_path),
            "preview_section": str(preview_path),
            "processed_section": str(processed_section_path),
        },
    }
    summary_path = layout.output_root / "workflow_summary.json"
    write_json(summary_path, summary)
    summary["summary_path"] = str(summary_path)
    return summary


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Run the Ophiolite seismic golden path through the Python SDK surface: "
            "preflight SEG-Y, import a dataset, preview bandpass + RMS AGC, "
            "materialize the processed store, and export chart-ready sections."
        )
    )
    parser.add_argument(
        "--segy-path",
        help=(
            "SEG-Y file to ingest. Defaults to OPHIOLITE_SEISMIC_GOLDEN_PATH_SEGY or the first "
            "available local fixture candidate."
        ),
    )
    parser.add_argument(
        "--run-root",
        help="Directory for the generated stores and JSON outputs. Defaults to a temp directory.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Remove an existing run root before writing outputs.",
    )
    parser.add_argument(
        "--section-axis",
        choices=("inline", "xline"),
        default=DEFAULT_SECTION_AXIS,
        help="Section axis for the preview and chart payloads.",
    )
    parser.add_argument(
        "--section-index",
        type=int,
        help="Section index to view. Defaults to the midpoint along the selected axis.",
    )
    parser.add_argument(
        "--bandpass-hz",
        default="8,12,45,60",
        help="Four comma-separated bandpass corner frequencies f1,f2,f3,f4 in Hz.",
    )
    parser.add_argument(
        "--agc-window-ms",
        type=float,
        default=DEFAULT_AGC_WINDOW_MS,
        help="RMS AGC window in milliseconds.",
    )
    parser.add_argument(
        "--traceboost-app-bin",
        help=(
            "Optional explicit traceboost-app binary. Defaults to TRACEBOOST_APP_BIN, then "
            "target/debug/traceboost-app, then cargo run."
        ),
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    segy_path = Path(args.segy_path).expanduser() if args.segy_path else detect_default_segy_path()
    if segy_path is None or not segy_path.exists():
        parser.error(
            "could not resolve a SEG-Y input. Pass --segy-path or set "
            "OPHIOLITE_SEISMIC_GOLDEN_PATH_SEGY."
        )

    summary = run_workflow(
        segy_path=segy_path.resolve(),
        run_root=args.run_root,
        overwrite=args.overwrite,
        section_axis=args.section_axis,
        section_index=args.section_index,
        bandpass_hz=parse_bandpass(args.bandpass_hz),
        agc_window_ms=args.agc_window_ms,
        traceboost_app_bin=args.traceboost_app_bin,
    )
    json.dump(summary, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
