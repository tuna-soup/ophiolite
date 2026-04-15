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
            elif rust_cli == "list-project-wells":
                declared_python_methods.add("list_project_wells")
                declared_python_cli_commands.add("list-project-wells")
            elif rust_cli == "list-project-wellbores":
                declared_python_methods.add("list_project_wellbores")
                declared_python_cli_commands.add("list-project-wellbores")
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
