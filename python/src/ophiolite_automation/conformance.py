from __future__ import annotations

import argparse
from typing import Any

from .catalog import load_operation_catalog
from .cli import build_parser
from .client import OphioliteApp


INTERNAL_PYTHON_METHODS = {"command_prefix", "run_json"}
INTERNAL_PYTHON_CLI_COMMANDS = {"verify-surface-contracts"}


def verify_surface_contracts() -> dict[str, Any]:
    catalog = load_operation_catalog()
    operations = catalog["operations"]

    declared_python_methods = set()
    declared_python_cli_commands = set()
    issues: list[str] = []

    for operation in operations:
        bindings = operation.get("bindings", {})
        surfaces = set(operation.get("surfaces", []))
        operation_id = operation["id"]

        if "rust-cli" in surfaces:
            rust_cli = bindings.get("rust_cli")
            if rust_cli == "operation-catalog":
                declared_python_methods.add("operation_catalog")
                declared_python_cli_commands.add("operation-catalog")
            elif rust_cli == "create-project":
                declared_python_methods.add("create_project")
                declared_python_cli_commands.add("create-project")
            elif rust_cli == "open-project":
                declared_python_methods.add("open_project")
                declared_python_cli_commands.add("open-project")
            elif rust_cli == "project-summary":
                declared_python_methods.add("project_summary")
                declared_python_cli_commands.add("project-summary")
            elif rust_cli == "import-las-asset":
                declared_python_methods.add("import_project_las")
                declared_python_cli_commands.add("import-las-asset")
            elif rust_cli == "import-las-asset-with-binding":
                declared_python_methods.add("import_project_las_with_binding")
                declared_python_cli_commands.add("import-las-asset-with-binding")
            elif rust_cli == "import-tops-source-with-binding":
                declared_python_methods.add("import_project_tops_source_with_binding")
                declared_python_cli_commands.add("import-tops-source-with-binding")
            elif rust_cli == "list-project-wells":
                declared_python_methods.add("list_project_wells")
                declared_python_cli_commands.add("list-project-wells")
            elif rust_cli == "list-project-wellbores":
                declared_python_methods.add("list_project_wellbores")
                declared_python_cli_commands.add("list-project-wellbores")
            elif rust_cli == "list-project-surveys":
                declared_python_methods.add("list_project_surveys")
                declared_python_cli_commands.add("list-project-surveys")
            elif rust_cli == "resolve-well-panel-source":
                declared_python_methods.add("resolve_well_panel_source")
                declared_python_cli_commands.add("resolve-well-panel-source")
            elif rust_cli == "resolve-survey-map-source":
                declared_python_methods.add("resolve_survey_map_source")
                declared_python_cli_commands.add("resolve-survey-map-source")
            elif rust_cli == "resolve-wellbore-trajectory":
                declared_python_methods.add("resolve_wellbore_trajectory")
                declared_python_cli_commands.add("resolve-wellbore-trajectory")
            elif rust_cli == "resolve-section-well-overlays":
                declared_python_methods.add("resolve_section_well_overlays")
                declared_python_cli_commands.add("resolve-section-well-overlays")
            elif rust_cli == "project-operator-lock":
                declared_python_methods.add("project_operator_lock")
                declared_python_cli_commands.add("project-operator-lock")
            elif rust_cli == "install-operator-package":
                declared_python_methods.add("install_operator_package")
                declared_python_cli_commands.add("install-operator-package")
            elif rust_cli == "list-project-compute-catalog":
                declared_python_methods.add("list_project_compute_catalog")
                declared_python_cli_commands.add("list-project-compute-catalog")
            elif rust_cli == "run-project-compute":
                declared_python_methods.add("run_project_compute")
                declared_python_cli_commands.add("run-project-compute")
            elif rust_cli == "run-avo-reflectivity":
                declared_python_methods.add("run_avo_reflectivity")
                declared_python_cli_commands.add("run-avo-reflectivity")
            elif rust_cli == "run-rock-physics-attribute":
                declared_python_methods.add("run_rock_physics_attribute")
                declared_python_cli_commands.add("run-rock-physics-attribute")
            elif rust_cli == "run-avo-intercept-gradient-attribute":
                declared_python_methods.add("run_avo_intercept_gradient_attribute")
                declared_python_cli_commands.add("run-avo-intercept-gradient-attribute")
            elif rust_cli == "preview-well-folder-import":
                declared_python_methods.add("preview_well_folder_import")
                declared_python_cli_commands.add("preview-well-folder-import")
            elif rust_cli == "preview-well-source-import":
                declared_python_methods.add("preview_well_source_import")
                declared_python_cli_commands.add("preview-well-source-import")
            elif rust_cli == "vendor-scan":
                declared_python_methods.add("vendor_scan")
                declared_python_cli_commands.add("vendor-scan")
            elif rust_cli == "vendor-plan":
                declared_python_methods.add("vendor_plan")
                declared_python_cli_commands.add("vendor-plan")
            elif rust_cli == "vendor-commit":
                declared_python_methods.add("vendor_commit")
                declared_python_cli_commands.add("vendor-commit")
            elif rust_cli == "vendor-runtime-probe":
                declared_python_methods.add("vendor_runtime_probe")
                declared_python_cli_commands.add("vendor-runtime-probe")
            elif rust_cli == "vendor-connector-contract":
                declared_python_methods.add("vendor_connector_contract")
                declared_python_cli_commands.add("vendor-connector-contract")
            elif rust_cli == "vendor-bridge-capabilities":
                declared_python_methods.add("vendor_bridge_capabilities")
                declared_python_cli_commands.add("vendor-bridge-capabilities")
            elif rust_cli == "vendor-bridge-run":
                declared_python_methods.add("vendor_bridge_run")
                declared_python_cli_commands.add("vendor-bridge-run")
            elif rust_cli == "vendor-bridge-commit":
                declared_python_methods.add("vendor_bridge_commit")
                declared_python_cli_commands.add("vendor-bridge-commit")
            elif rust_cli == "inspect-horizon-xyz":
                declared_python_methods.add("inspect_horizon_xyz")
                declared_python_cli_commands.add("inspect-horizon-xyz")
            elif rust_cli == "preview-horizon-source-import":
                declared_python_methods.add("preview_horizon_source_import")
                declared_python_cli_commands.add("preview-horizon-source-import")
            elif rust_cli == "import":
                declared_python_methods.add("import_las_bundle")
                declared_python_cli_commands.add("import")
            elif rust_cli == "inspect-file":
                declared_python_methods.add("inspect_las_file")
                declared_python_cli_commands.add("inspect-file")
            elif rust_cli == "summary":
                declared_python_methods.add("open_bundle_summary")
                declared_python_cli_commands.add("summary")
            elif rust_cli == "list-curves":
                declared_python_methods.add("list_bundle_curves")
                declared_python_cli_commands.add("list-curves")
            elif rust_cli == "examples":
                declared_python_methods.add("examples_root")
                declared_python_cli_commands.add("examples")
            elif rust_cli == "generate-fixture-packages":
                declared_python_methods.add("generate_fixture_packages")
                declared_python_cli_commands.add("generate-fixture-packages")
            elif rust_cli is not None:
                issues.append(f"{operation_id}: unsupported rust-cli to python binding '{rust_cli}'")

    actual_python_methods = {
        name
        for name, value in OphioliteApp.__dict__.items()
        if callable(value) and not name.startswith("_") and name not in INTERNAL_PYTHON_METHODS
    }
    actual_python_cli_commands = parser_subcommand_names(build_parser()) - INTERNAL_PYTHON_CLI_COMMANDS

    for missing_method in sorted(declared_python_methods - actual_python_methods):
        issues.append(f"catalog expects python method '{missing_method}' but OphioliteApp does not expose it")

    for extra_method in sorted(actual_python_methods - declared_python_methods):
        issues.append(f"OphioliteApp exposes uncatalogued python method '{extra_method}'")

    for missing_command in sorted(declared_python_cli_commands - actual_python_cli_commands):
        issues.append(
            f"catalog expects python CLI command '{missing_command}' but ophiolite-automation does not expose it"
        )

    for extra_command in sorted(actual_python_cli_commands - declared_python_cli_commands):
        issues.append(f"ophiolite-automation exposes uncatalogued CLI command '{extra_command}'")

    return {
        "ok": not issues,
        "catalog_name": catalog["catalog_name"],
        "catalog_schema_version": catalog["schema_version"],
        "checked_python_method_count": len(declared_python_methods),
        "checked_python_cli_command_count": len(declared_python_cli_commands),
        "issues": issues,
    }


def parser_subcommand_names(parser: argparse.ArgumentParser) -> set[str]:
    for action in parser._actions:
        if isinstance(action, argparse._SubParsersAction):
            return set(action.choices.keys())
    return set()
