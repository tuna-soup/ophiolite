from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_MANIFEST_PATH = REPO_ROOT / "Cargo.toml"


class OphioliteCommandError(RuntimeError):
    def __init__(self, message: str, *, command: Sequence[str], stderr: str | None = None) -> None:
        super().__init__(message)
        self.command = list(command)
        self.stderr = stderr


@dataclass(frozen=True)
class OphioliteApp:
    repo_root: Path = REPO_ROOT
    manifest_path: Path = DEFAULT_MANIFEST_PATH
    binary: str | None = None

    def command_prefix(self) -> list[str]:
        binary = self.binary or os.environ.get("OPHIOLITE_CLI_BIN")
        if binary:
            return [binary]
        return [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(self.manifest_path),
            "-p",
            "ophiolite-cli",
            "--",
        ]

    def run_json(self, *args: str) -> Any:
        command = [*self.command_prefix(), *args]
        completed = subprocess.run(
            command,
            cwd=self.repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if completed.returncode != 0:
            raise OphioliteCommandError(
                f"ophiolite-cli exited with status {completed.returncode}",
                command=command,
                stderr=completed.stderr.strip() or None,
            )
        stdout = completed.stdout.strip()
        if not stdout:
            return None
        try:
            return json.loads(stdout)
        except json.JSONDecodeError as exc:
            raise OphioliteCommandError(
                "ophiolite-cli did not return valid JSON",
                command=command,
                stderr=completed.stderr.strip() or stdout,
            ) from exc

    def operation_catalog(self) -> Any:
        return self.run_json("operation-catalog")

    def create_project(self, project_root: str) -> Any:
        return self.run_json("create-project", project_root)

    def open_project(self, project_root: str) -> Any:
        return self.run_json("open-project", project_root)

    def project_summary(self, project_root: str) -> Any:
        return self.run_json("project-summary", project_root)

    def list_project_wells(self, project_root: str) -> Any:
        return self.run_json("list-project-wells", project_root)

    def list_project_wellbores(self, project_root: str, well_id: str) -> Any:
        return self.run_json("list-project-wellbores", project_root, well_id)

    def import_las_bundle(self, input_path: str, bundle_dir: str) -> Any:
        return self.run_json("import", input_path, bundle_dir)

    def inspect_las_file(self, input_path: str) -> Any:
        return self.run_json("inspect-file", input_path)

    def open_bundle_summary(self, bundle_dir: str) -> Any:
        return self.run_json("summary", bundle_dir)

    def list_bundle_curves(self, bundle_dir: str) -> Any:
        return self.run_json("list-curves", bundle_dir)

    def examples_root(self) -> Any:
        return self.run_json("examples")

    def generate_fixture_packages(self, input_root: str, output_root: str) -> Any:
        return self.run_json("generate-fixture-packages", input_root, output_root)
