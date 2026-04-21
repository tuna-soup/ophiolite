#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from dataclasses import dataclass
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Any, Sequence


REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_SRC = REPO_ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from ophiolite_automation.client import OphioliteApp
from ophiolite_sdk import Project
from ophiolite_sdk.avo import (
    AngleSampling,
    AvoExperiment,
    ElasticChannelBindings,
    LayeringSpec,
)


DEFAULT_DSI_LAS_PATH = Path(
    "/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/"
    "f02a05_20030103-1-06_a5_main_log_dsi_030lup.las"
)
DEFAULT_DENSITY_LAS_PATH = Path(
    "/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/"
    "f02a05_20030103-1-06_lwd2259a5rt.las"
)
DEFAULT_TOPS_SOURCE_PATH = Path(
    "/Users/sc/Downloads/SubsurfaceData/blocks/F3/F02_wells_data/F02-A-05/"
    "lithostratigrafie.txt"
)
DEFAULT_INTERVAL_THICKNESS_M = 6.096
DEFAULT_ANGLES_DEG = tuple(float(value) for value in range(0, 41, 5))


@dataclass(frozen=True)
class RunLayout:
    run_root: Path
    project_root: Path
    output_root: Path


def configured_app() -> OphioliteApp:
    cli_binary = os.environ.get("OPHIOLITE_CLI_BIN")
    if cli_binary:
        return OphioliteApp(binary=cli_binary)
    return OphioliteApp()


def detect_real_las_paths() -> tuple[Path | None, Path | None]:
    dsi_candidate = Path(
        os.environ.get("OPHIOLITE_LOG_AVO_DSI_LAS", str(DEFAULT_DSI_LAS_PATH))
    )
    density_candidate = Path(
        os.environ.get("OPHIOLITE_LOG_AVO_DENSITY_LAS", str(DEFAULT_DENSITY_LAS_PATH))
    )
    dsi_path = dsi_candidate if dsi_candidate.exists() else None
    density_path = density_candidate if density_candidate.exists() else None
    return dsi_path, density_path


def detect_real_tops_path() -> Path | None:
    tops_candidate = Path(
        os.environ.get("OPHIOLITE_LOG_AVO_TOPS_SOURCE", str(DEFAULT_TOPS_SOURCE_PATH))
    )
    return tops_candidate if tops_candidate.exists() else None


def parse_float_list(text: str | None) -> list[float] | None:
    if text is None:
        return None
    values = [item.strip() for item in text.split(",")]
    normalized = [float(item) for item in values if item]
    return normalized or None


def parse_string_list(text: str | None) -> list[str] | None:
    if text is None:
        return None
    values = [item.strip() for item in text.split(",")]
    normalized = [item for item in values if item]
    return normalized or None


def prepare_layout(run_root: str | None, *, overwrite: bool) -> RunLayout:
    if run_root is None:
        root = Path(tempfile.mkdtemp(prefix="ophiolite_log_avo_"))
    else:
        root = Path(run_root).expanduser().resolve()
        if root.exists() and overwrite:
            shutil.rmtree(root)
        root.mkdir(parents=True, exist_ok=True)
    project_root = root / "project"
    output_root = root / "outputs"
    output_root.mkdir(parents=True, exist_ok=True)
    return RunLayout(run_root=root, project_root=project_root, output_root=output_root)


def generate_synthetic_project(project_root: Path) -> dict[str, Any]:
    project_root.parent.mkdir(parents=True, exist_ok=True)
    binary = REPO_ROOT / "target" / "debug" / "ophiolite"
    commands: list[list[str]] = []
    if binary.exists():
        commands.append([str(binary), "generate-synthetic-project", str(project_root)])
    commands.append(
        [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(REPO_ROOT / "Cargo.toml"),
            "-p",
            "ophiolite",
            "--",
            "generate-synthetic-project",
            str(project_root),
        ]
    )

    last_error: subprocess.CalledProcessError | None = None
    for command in commands:
        try:
            completed = subprocess.run(
                command,
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            stdout = completed.stdout.strip()
            return json.loads(stdout) if stdout else {}
        except subprocess.CalledProcessError as error:
            last_error = error
            continue
    raise RuntimeError(
        "failed to generate the synthetic project fixture"
        if last_error is None
        else (
            "failed to generate the synthetic project fixture: "
            f"{last_error.stderr.strip() or last_error.stdout.strip()}"
        )
    )


def import_real_las_pair(
    layout: RunLayout,
    *,
    dsi_path: Path,
    density_path: Path,
    app: OphioliteApp,
) -> tuple[Project, Any, dict[str, Any]]:
    project = Project.create(layout.project_root, app=app)
    first_import = project.import_las(dsi_path, collection_name="golden-path-dsi")
    wells = project.wells()
    if not wells:
        raise RuntimeError("LAS ingest did not produce a well")
    wellbore = wells[0].wellbores()[0]
    second_import = project.import_las(
        density_path,
        binding=wellbore.binding(),
        collection_name="golden-path-density",
    )
    refreshed_wellbore = project.wells()[0].wellbores()[0]
    tops_path = detect_real_tops_path()
    tops_import = None
    if tops_path is not None:
        tops_result = project.import_tops_source(
            tops_path,
            binding=refreshed_wellbore.binding(),
            collection_name="lithostrat-tops",
        )
        tops_import = {
            "source_path": str(tops_result.source_path),
            "source_name": tops_result.source_name,
            "reported_well_name": tops_result.reported_well_name,
            "reported_depth_reference": tops_result.reported_depth_reference,
            "resolved_source_depth_reference": tops_result.resolved_source_depth_reference,
            "resolved_depth_domain": tops_result.resolved_depth_domain,
            "resolved_depth_datum": tops_result.resolved_depth_datum,
            "source_row_count": tops_result.source_row_count,
            "imported_row_count": tops_result.imported_row_count,
            "omitted_row_count": tops_result.omitted_row_count,
            "collection_name": tops_result.import_result.collection["name"],
            "asset_id": tops_result.import_result.asset["id"],
            "issues": list(tops_result.issues),
            "omissions": list(tops_result.omissions),
        }
    return project, refreshed_wellbore, {
        "data_mode": "real",
        "dsi_las_path": str(dsi_path),
        "density_las_path": str(density_path),
        "tops_source_path": None if tops_path is None else str(tops_path),
        "tops_import": tops_import,
        "imports": [
            {
                "asset_id": first_import.asset["id"],
                "asset_label": asset_label(first_import.asset),
                "collection_name": first_import.collection["name"],
            },
            {
                "asset_id": second_import.asset["id"],
                "asset_label": asset_label(second_import.asset),
                "collection_name": second_import.collection["name"],
            },
        ],
    }


def open_synthetic_project(
    layout: RunLayout,
    *,
    app: OphioliteApp,
) -> tuple[Project, Any, dict[str, Any]]:
    fixture = generate_synthetic_project(layout.project_root)
    project = Project.open(layout.project_root, app=app)
    wells = project.wells()
    if not wells:
        raise RuntimeError("synthetic fixture did not produce a well")
    wellbore = wells[0].wellbores()[0]
    return project, wellbore, {
        "data_mode": "synthetic",
        "fixture_summary": fixture,
    }


def load_demo_project(
    *,
    data_mode: str = "auto",
    run_root: str | None = None,
    overwrite: bool = False,
) -> tuple[RunLayout, Project, Any, dict[str, Any]]:
    layout = prepare_layout(run_root, overwrite=overwrite)
    app = configured_app()

    resolved_mode = data_mode
    if resolved_mode == "auto":
        dsi_path, density_path = detect_real_las_paths()
        resolved_mode = "real" if dsi_path is not None and density_path is not None else "synthetic"
    else:
        dsi_path, density_path = detect_real_las_paths()

    if resolved_mode == "real":
        if dsi_path is None or density_path is None:
            raise FileNotFoundError(
                "real mode requires both OPHIOLITE_LOG_AVO_DSI_LAS and "
                "OPHIOLITE_LOG_AVO_DENSITY_LAS, or the default local F3 files"
            )
        project, wellbore, data_summary = import_real_las_pair(
            layout,
            dsi_path=dsi_path,
            density_path=density_path,
            app=app,
        )
        return layout, project, wellbore, data_summary

    if resolved_mode == "synthetic":
        project, wellbore, data_summary = open_synthetic_project(layout, app=app)
        return layout, project, wellbore, data_summary

    raise ValueError(f"unsupported data mode '{data_mode}'")


def channel_summary(channel: Any) -> dict[str, Any]:
    return {
        "semantic_type": channel.semantic_type,
        "source_semantic_type": channel.source_semantic_type,
        "source_log_type": channel.source_curve.log_type,
        "source_asset_name": channel.source_curve.asset_name,
        "source_curve_name": channel.source_curve.curve_name,
        "source_mnemonic": channel.source_curve.original_mnemonic,
        "source_unit": channel.source_curve.unit,
        "derivation": channel.derivation,
        "depth_range_m": list(channel.depth_range_m) if channel.depth_range_m is not None else None,
        "estimated_step_m": channel.estimated_step_m,
    }


def asset_label(asset: dict[str, Any]) -> str | None:
    manifest = asset.get("manifest")
    if isinstance(manifest, dict):
        provenance = manifest.get("provenance")
        if isinstance(provenance, dict):
            original_filename = provenance.get("original_filename")
            if isinstance(original_filename, str) and original_filename:
                return original_filename
        source_artifacts = manifest.get("source_artifacts")
        if isinstance(source_artifacts, list) and source_artifacts:
            first = source_artifacts[0]
            if isinstance(first, dict):
                original_filename = first.get("original_filename")
                if isinstance(original_filename, str) and original_filename:
                    return original_filename
    value = asset.get("id")
    return value if isinstance(value, str) else None


def materialization_summary(runs: dict[str, Any]) -> dict[str, Any]:
    return {
        key: {
            "asset_id": run.asset.get("id"),
            "asset_label": asset_label(run.asset),
            "collection_name": run.collection.get("name"),
            "execution_status": run.execution.get("status"),
        }
        for key, run in runs.items()
    }


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2)
        handle.write("\n")


def default_edge_depths(elastic: Any, *, interval_thickness_m: float) -> list[float]:
    aligned = elastic.align_log_set()
    depth_min = aligned.depth_min_m
    depth_max = aligned.depth_max_m
    max_layers = max(2, min(4, int((depth_max - depth_min) / interval_thickness_m)))
    edges = [depth_min + (index * interval_thickness_m) for index in range(max_layers + 1)]
    clipped = [edge for edge in edges if edge < depth_max]
    if not clipped or clipped[-1] < depth_max:
        clipped.append(min(depth_max, depth_min + (max_layers * interval_thickness_m)))
    unique_edges: list[float] = []
    for edge in clipped:
        if not unique_edges or edge > unique_edges[-1]:
            unique_edges.append(edge)
    if len(unique_edges) < 3:
        unique_edges = [depth_min, depth_min + interval_thickness_m, min(depth_max, depth_min + (2.0 * interval_thickness_m))]
    return unique_edges


def run_workflow(
    *,
    data_mode: str,
    run_root: str | None = None,
    overwrite: bool = False,
    interval_thickness_m: float = DEFAULT_INTERVAL_THICKNESS_M,
    angles_deg: Sequence[float] = DEFAULT_ANGLES_DEG,
    edge_depths_m: Sequence[float] | None = None,
    edge_labels: Sequence[str] | None = None,
    top_set_asset_name: str | None = None,
    top_set_labels: Sequence[str] | None = None,
    top_set_selectors: Sequence[str] | None = None,
    materialize_derived_channels: bool = False,
) -> dict[str, Any]:
    layout, project, wellbore, data_summary = load_demo_project(
        data_mode=data_mode,
        run_root=run_root,
        overwrite=overwrite,
    )

    bindings = ElasticChannelBindings(
        vp="Dt",
        vs="Dts",
        density="Rho",
    )
    elastic = wellbore.elastic_log_set(bindings=bindings)
    available_log_types = wellbore.available_log_types()
    available_top_sets = wellbore.available_top_sets()
    available_marker_sets = wellbore.available_marker_sets()
    derived_runs = {}
    if materialize_derived_channels:
        derived_runs = elastic.materialize_missing_channels(
            output_collection_name="golden-path-derived-elastic"
        )

    top_sets = wellbore.top_sets()
    marker_sets = wellbore.marker_sets()
    selected_top_set = wellbore.top_set(top_set_asset_name) if top_sets else None

    experiment = AvoExperiment.zoeppritz(
        angles=AngleSampling.explicit(angles_deg),
    )
    crossplot_experiment = AvoExperiment.shuey_two_term(
        angles=AngleSampling.explicit(angles_deg),
    )

    fixed_layering = LayeringSpec.fixed_interval(interval_thickness_m, unit="m")
    fixed_result = elastic.run_avo(
        layering=fixed_layering,
        experiment=experiment,
    )
    fixed_source = fixed_result.response_source(
        title="Fixed Interval Zoeppritz AVO",
        subtitle=f"{wellbore.name} fixed {interval_thickness_m:.3f} m layering",
        source_id="avo-fixed-interval",
        name=f"{wellbore.name} Fixed Interval AVO",
    )
    fixed_output_path = layout.output_root / "avo_fixed_interval_source.json"
    write_json(fixed_output_path, fixed_source)
    fixed_crossplot_path = layout.output_root / "avo_fixed_interval_crossplot.json"
    write_json(
        fixed_crossplot_path,
        elastic.run_avo(
            layering=fixed_layering,
            experiment=crossplot_experiment,
        ).crossplot_source(
            title="Fixed Interval Intercept-Gradient Crossplot",
            subtitle=f"{wellbore.name} fixed {interval_thickness_m:.3f} m layering (Shuey two-term)",
            source_id="avo-fixed-interval-crossplot",
            name=f"{wellbore.name} Fixed Interval Crossplot",
        ),
    )

    resolved_edges = list(edge_depths_m) if edge_depths_m is not None else default_edge_depths(
        elastic,
        interval_thickness_m=interval_thickness_m,
    )
    explicit_layering = LayeringSpec.from_edges(
        resolved_edges,
        labels=edge_labels,
        unit="m",
    )
    explicit_result = elastic.run_avo(
        layering=explicit_layering,
        experiment=experiment,
    )
    explicit_source = explicit_result.response_source(
        title="Explicit Edge Zoeppritz AVO",
        subtitle=f"{wellbore.name} user-defined interval edges",
        source_id="avo-explicit-edges",
        name=f"{wellbore.name} Explicit Edge AVO",
    )
    explicit_output_path = layout.output_root / "avo_explicit_edges_source.json"
    write_json(explicit_output_path, explicit_source)

    top_set_output_path: Path | None = None
    top_set_source: dict[str, Any] | None = None
    top_set_error: str | None = None
    if top_sets and selected_top_set is None and top_set_asset_name is not None:
        available = ", ".join(top_set.asset_name for top_set in top_sets)
        top_set_error = (
            f"top set '{top_set_asset_name}' was not found on wellbore '{wellbore.id}'. "
            f"Available top sets: {available}"
        )
    if selected_top_set is not None:
        try:
            top_set_layering = selected_top_set.layering(
                labels=top_set_labels,
                selectors=top_set_selectors,
            )
            top_set_result = elastic.run_avo(
                layering=top_set_layering,
                experiment=experiment,
            )
            top_set_source = top_set_result.response_source(
                title="Top-Set Zoeppritz AVO",
                subtitle=f"{wellbore.name} interval layers from tops",
                source_id="avo-top-set",
                name=f"{wellbore.name} Top-Set AVO",
            )
            top_set_output_path = layout.output_root / "avo_top_set_source.json"
            write_json(top_set_output_path, top_set_source)
        except Exception as error:
            top_set_error = str(error)

    summary = {
        "repo_root": str(REPO_ROOT),
        "run_root": str(layout.run_root),
        "project_root": str(layout.project_root),
        "output_root": str(layout.output_root),
        "project": {
            "root": str(project.root),
            "summary": {
                "well_count": project.summary().well_count,
                "wellbore_count": project.summary().wellbore_count,
                "asset_collection_count": project.summary().asset_collection_count,
                "asset_count": project.summary().asset_count,
            },
        },
        "wellbore": {
            "id": wellbore.id,
            "well_id": wellbore.well_id,
            "name": wellbore.name,
        },
        "data": data_summary,
        "public_surface": {
            "bindings": {
                "vp": bindings.vp,
                "vs": bindings.vs,
                "density": bindings.density,
            },
            "available_log_types": available_log_types,
            "available_top_sets": available_top_sets,
            "available_marker_sets": available_marker_sets,
            "experiment": {
                "method": experiment.method,
                "angles_deg": list(experiment.angles.values),
            },
            "crossplot_experiment": {
                "method": crossplot_experiment.method,
                "angles_deg": list(crossplot_experiment.angles.values),
            },
        },
        "elastic_channels": {
            "vp": channel_summary(elastic.vp),
            "vs": channel_summary(elastic.vs),
            "density": channel_summary(elastic.density),
            "materialized_outputs": materialization_summary(derived_runs),
        },
        "top_sets": [
            {
                "asset_name": top_set.asset_name,
                "set_kind": top_set.set_kind,
                "interval_labels": [interval.label for interval in top_set.intervals],
                "interval_selectors": list(top_set.interval_selectors),
            }
            for top_set in top_sets
        ],
        "marker_sets": [
            {
                "asset_name": marker_set.asset_name,
                "set_kind": marker_set.set_kind,
                "interval_labels": [interval.label for interval in marker_set.intervals],
            }
            for marker_set in marker_sets
        ],
        "avo": {
            "fixed_interval": {
                "interval_thickness_m": interval_thickness_m,
                "interface_count": len(fixed_result.interfaces),
                "output_path": str(fixed_output_path),
                "crossplot_output_path": str(fixed_crossplot_path),
            },
            "explicit_edges": {
                "depth_edges_m": [float(value) for value in resolved_edges],
                "interface_count": len(explicit_result.interfaces),
                "output_path": str(explicit_output_path),
            },
            "top_set": (
                {
                    "asset_name": selected_top_set.asset_name,
                    "labels": list(top_set_labels or []),
                    "selectors": list(top_set_selectors or []),
                    "interface_count": 0 if top_set_source is None else len(top_set_source["series"]),
                    "output_path": None if top_set_output_path is None else str(top_set_output_path),
                    "error": top_set_error,
                }
                if selected_top_set is not None
                else {
                    "asset_name": None,
                    "labels": list(top_set_labels or []),
                    "selectors": list(top_set_selectors or []),
                    "interface_count": 0,
                    "output_path": None,
                    "error": top_set_error or "no top sets available on the selected wellbore",
                }
            ),
        },
    }
    summary_path = layout.output_root / "workflow_summary.json"
    write_json(summary_path, summary)
    summary["summary_path"] = str(summary_path)
    return summary


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run the Ophiolite log-to-AVO golden-path workflow.",
    )
    parser.add_argument(
        "--data-mode",
        choices=("auto", "real", "synthetic"),
        default="auto",
        help="Choose real LAS ingest, synthetic fixture fallback, or auto-detect.",
    )
    parser.add_argument(
        "--run-root",
        help="Directory for the generated project and output JSON files. Defaults to a temp directory.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Remove an existing run root before writing the workflow outputs.",
    )
    parser.add_argument(
        "--interval-thickness-m",
        type=float,
        default=DEFAULT_INTERVAL_THICKNESS_M,
        help="Fixed-layer interval thickness in meters. Default is 6.096 m (20 ft).",
    )
    parser.add_argument(
        "--angles-deg",
        default="0,5,10,15,20,25,30,35,40",
        help="Comma-separated incidence angles in degrees.",
    )
    parser.add_argument(
        "--edge-depths-m",
        help="Comma-separated explicit depth edges in meters for the edge-based AVO workflow.",
    )
    parser.add_argument(
        "--edge-labels",
        help="Comma-separated labels for the explicit edge intervals.",
    )
    parser.add_argument(
        "--top-set-asset-name",
        help="Optional top-set asset name for the top-driven AVO workflow.",
    )
    parser.add_argument(
        "--top-set-labels",
        help="Optional comma-separated interval labels to select from the chosen top set.",
    )
    parser.add_argument(
        "--top-set-selectors",
        help="Optional comma-separated interval selectors for precise top-set selection when labels repeat.",
    )
    parser.add_argument(
        "--materialize-derived-channels",
        action="store_true",
        help="Persist derived VP and VS logs when they come from sonic curves.",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_arg_parser()
    args = parser.parse_args(argv)

    summary = run_workflow(
        data_mode=args.data_mode,
        run_root=args.run_root,
        overwrite=args.overwrite,
        interval_thickness_m=float(args.interval_thickness_m),
        angles_deg=parse_float_list(args.angles_deg) or list(DEFAULT_ANGLES_DEG),
        edge_depths_m=parse_float_list(args.edge_depths_m),
        edge_labels=parse_string_list(args.edge_labels),
        top_set_asset_name=args.top_set_asset_name,
        top_set_labels=parse_string_list(args.top_set_labels),
        top_set_selectors=parse_string_list(args.top_set_selectors),
        materialize_derived_channels=bool(args.materialize_derived_channels),
    )

    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
