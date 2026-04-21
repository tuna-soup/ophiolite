from __future__ import annotations

from ophiolite_automation.catalog import load_operation_catalog

from .models import PlatformCatalog

__all__ = ["catalog", "PlatformCatalog"]


def catalog() -> PlatformCatalog:
    return PlatformCatalog.from_json(load_operation_catalog())
