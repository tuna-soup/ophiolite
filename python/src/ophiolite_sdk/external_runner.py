from __future__ import annotations

import argparse
import importlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any

from .external import OperatorRegistry


def _load_module(entrypoint: str, manifest_dir: Path) -> ModuleType:
    manifest_dir = manifest_dir.resolve()
    if str(manifest_dir) not in sys.path:
        sys.path.insert(0, str(manifest_dir))

    entrypoint_path = Path(entrypoint)
    if not entrypoint_path.is_absolute():
        entrypoint_path = manifest_dir / entrypoint_path
    if entrypoint_path.is_file():
        spec = importlib.util.spec_from_file_location(
            f"ophiolite_external_{entrypoint_path.stem}", entrypoint_path
        )
        if spec is None or spec.loader is None:
            raise RuntimeError(f"failed to load operator entrypoint '{entrypoint_path}'")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    return importlib.import_module(entrypoint)


def _resolve_registry(module: ModuleType) -> OperatorRegistry:
    registry = getattr(module, "registry", None)
    if isinstance(registry, OperatorRegistry):
        return registry
    factory = getattr(module, "get_registry", None)
    if callable(factory):
        registry = factory()
        if isinstance(registry, OperatorRegistry):
            return registry
    raise RuntimeError(
        "operator entrypoint must expose 'registry' or 'get_registry()' returning OperatorRegistry"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--entrypoint", required=True)
    parser.add_argument("--manifest-dir", required=True)
    args = parser.parse_args(argv)

    module = _load_module(args.entrypoint, Path(args.manifest_dir))
    registry = _resolve_registry(module)
    request = json.load(sys.stdin)
    response: dict[str, Any] = registry.invoke(request)
    json.dump(response, sys.stdout)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
