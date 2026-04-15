from __future__ import annotations

import json
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_OPERATION_CATALOG_PATH = REPO_ROOT / "crates" / "ophiolite-cli" / "operations" / "catalog.json"


def load_operation_catalog(path: Path = DEFAULT_OPERATION_CATALOG_PATH) -> dict[str, Any]:
    return json.loads(path.read_text())
