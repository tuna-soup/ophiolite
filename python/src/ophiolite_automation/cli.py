from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from .client import DEFAULT_MANIFEST_PATH, REPO_ROOT, OphioliteApp, OphioliteCommandError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Python automation wrapper around ophiolite-cli")
    parser.add_argument("--repo-root", default=str(REPO_ROOT))
    parser.add_argument("--manifest-path", default=str(DEFAULT_MANIFEST_PATH))
    parser.add_argument("--binary", default=None)

    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("operation-catalog")

    create_project = subparsers.add_parser("create-project")
    create_project.add_argument("project_root")

    open_project = subparsers.add_parser("open-project")
    open_project.add_argument("project_root")

    project_summary = subparsers.add_parser("project-summary")
    project_summary.add_argument("project_root")

    list_project_wells = subparsers.add_parser("list-project-wells")
    list_project_wells.add_argument("project_root")

    list_project_wellbores = subparsers.add_parser("list-project-wellbores")
    list_project_wellbores.add_argument("project_root")
    list_project_wellbores.add_argument("well_id")

    project_operator_lock = subparsers.add_parser("project-operator-lock")
    project_operator_lock.add_argument("project_root")

    install_operator_package = subparsers.add_parser("install-operator-package")
    install_operator_package.add_argument("project_root")
    install_operator_package.add_argument("manifest_path")

    list_project_compute_catalog = subparsers.add_parser("list-project-compute-catalog")
    list_project_compute_catalog.add_argument("project_root")
    list_project_compute_catalog.add_argument("asset_id")

    run_project_compute = subparsers.add_parser("run-project-compute")
    run_project_compute.add_argument("project_root")
    run_project_compute.add_argument("request_json")

    run_avo_reflectivity = subparsers.add_parser("run-avo-reflectivity")
    run_avo_reflectivity.add_argument("request_json")

    run_rock_physics_attribute = subparsers.add_parser("run-rock-physics-attribute")
    run_rock_physics_attribute.add_argument("request_json")

    run_avo_intercept_gradient_attribute = subparsers.add_parser(
        "run-avo-intercept-gradient-attribute"
    )
    run_avo_intercept_gradient_attribute.add_argument("request_json")

    import_bundle = subparsers.add_parser("import")
    import_bundle.add_argument("input")
    import_bundle.add_argument("bundle_dir")

    inspect_file = subparsers.add_parser("inspect-file")
    inspect_file.add_argument("input")

    summary = subparsers.add_parser("summary")
    summary.add_argument("bundle_dir")

    list_curves = subparsers.add_parser("list-curves")
    list_curves.add_argument("bundle_dir")

    subparsers.add_parser("examples")

    generate_fixtures = subparsers.add_parser("generate-fixture-packages")
    generate_fixtures.add_argument("input_root")
    generate_fixtures.add_argument("output_root")

    subparsers.add_parser("verify-surface-contracts")

    return parser


def app_from_args(args: argparse.Namespace) -> OphioliteApp:
    return OphioliteApp(
        repo_root=Path(args.repo_root),
        manifest_path=Path(args.manifest_path),
        binary=args.binary,
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv) if argv is not None else None)
    app = app_from_args(args)

    try:
        if args.command == "operation-catalog":
            result = app.operation_catalog()
        elif args.command == "create-project":
            result = app.create_project(args.project_root)
        elif args.command == "open-project":
            result = app.open_project(args.project_root)
        elif args.command == "project-summary":
            result = app.project_summary(args.project_root)
        elif args.command == "list-project-wells":
            result = app.list_project_wells(args.project_root)
        elif args.command == "list-project-wellbores":
            result = app.list_project_wellbores(args.project_root, args.well_id)
        elif args.command == "project-operator-lock":
            result = app.project_operator_lock(args.project_root)
        elif args.command == "install-operator-package":
            result = app.install_operator_package(args.project_root, args.manifest_path)
        elif args.command == "list-project-compute-catalog":
            result = app.list_project_compute_catalog(args.project_root, args.asset_id)
        elif args.command == "run-project-compute":
            if args.request_json == "-":
                payload = json.load(sys.stdin)
            else:
                with open(args.request_json, encoding="utf-8") as handle:
                    payload = json.load(handle)
            result = app.run_project_compute(args.project_root, payload)
        elif args.command == "run-avo-reflectivity":
            if args.request_json == "-":
                payload = json.load(sys.stdin)
            else:
                with open(args.request_json, encoding="utf-8") as handle:
                    payload = json.load(handle)
            result = app.run_avo_reflectivity(payload)
        elif args.command == "run-rock-physics-attribute":
            if args.request_json == "-":
                payload = json.load(sys.stdin)
            else:
                with open(args.request_json, encoding="utf-8") as handle:
                    payload = json.load(handle)
            result = app.run_rock_physics_attribute(payload)
        elif args.command == "run-avo-intercept-gradient-attribute":
            if args.request_json == "-":
                payload = json.load(sys.stdin)
            else:
                with open(args.request_json, encoding="utf-8") as handle:
                    payload = json.load(handle)
            result = app.run_avo_intercept_gradient_attribute(payload)
        elif args.command == "import":
            result = app.import_las_bundle(args.input, args.bundle_dir)
        elif args.command == "inspect-file":
            result = app.inspect_las_file(args.input)
        elif args.command == "summary":
            result = app.open_bundle_summary(args.bundle_dir)
        elif args.command == "list-curves":
            result = app.list_bundle_curves(args.bundle_dir)
        elif args.command == "examples":
            result = app.examples_root()
        elif args.command == "generate-fixture-packages":
            result = app.generate_fixture_packages(args.input_root, args.output_root)
        elif args.command == "verify-surface-contracts":
            from .conformance import verify_surface_contracts

            result = verify_surface_contracts()
            print(json.dumps(result, indent=2))
            return 0 if result["ok"] else 1
        else:
            parser.error(f"unsupported command: {args.command}")
    except OphioliteCommandError as exc:
        if exc.stderr:
            print(exc.stderr, file=sys.stderr)
        print(str(exc), file=sys.stderr)
        return 1

    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
