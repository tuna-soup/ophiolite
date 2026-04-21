from __future__ import annotations

from pathlib import Path
from typing import Any, Mapping

import anywidget
import traitlets

__all__ = [
    "AvoResponseWidget",
    "AvoInterceptGradientCrossplotWidget",
]

_PACKAGE_ROOT = Path(__file__).resolve().parent
_STATIC_ROOT = _PACKAGE_ROOT / "static"
_WIDGET_CSS_PATH = _STATIC_ROOT / "widget.css"


class _BaseAvoWidget(anywidget.AnyWidget):
    source = traitlets.Dict(default_value={}).tag(sync=True)
    chart_id = traitlets.Unicode(default_value="").tag(sync=True)
    height_px = traitlets.Int(default_value=520).tag(sync=True)
    fit_request_id = traitlets.Int(default_value=0).tag(sync=True)
    frontend_error = traitlets.Unicode(allow_none=True, default_value=None).tag(sync=True)

    _esm_path: Path
    _css = _WIDGET_CSS_PATH if _WIDGET_CSS_PATH.exists() else ""

    def __init__(
        self,
        source: Mapping[str, Any],
        *,
        chart_id: str | None = None,
        height_px: int = 520,
    ) -> None:
        _ensure_widget_assets_exist(self._esm_path)
        coerced_source = self._coerce_source(source)
        self._validate_source_shape(coerced_source)
        super().__init__(
            source=coerced_source,
            chart_id=chart_id or "",
            height_px=max(240, int(height_px)),
            frontend_error=None,
        )

    def set_source(self, source: Mapping[str, Any]) -> None:
        coerced_source = self._coerce_source(source)
        self._validate_source_shape(coerced_source)
        self.source = coerced_source

    def fit_to_data(self) -> None:
        self.fit_request_id += 1

    @staticmethod
    def _coerce_source(source: Mapping[str, Any]) -> dict[str, Any]:
        if not isinstance(source, Mapping):
            raise TypeError("widget source must be a mapping")
        return dict(source)

    def _validate_source_shape(self, source: Mapping[str, Any]) -> None:
        raise NotImplementedError


class AvoResponseWidget(_BaseAvoWidget):
    _esm_path = _STATIC_ROOT / "widget-avo-response.js"
    _esm = _esm_path

    @classmethod
    def from_result(
        cls,
        result: Any,
        *,
        chart_id: str | None = None,
        height_px: int = 520,
        **source_kwargs: Any,
    ) -> AvoResponseWidget:
        if not hasattr(result, "response_source"):
            raise TypeError("result must expose a 'response_source(...)' method")
        source = result.response_source(**source_kwargs)
        return cls(source=source, chart_id=chart_id, height_px=height_px)

    def _validate_source_shape(self, source: Mapping[str, Any]) -> None:
        _require_mapping_keys(
            source,
            required_keys=("schema_version", "id", "name", "x_axis", "y_axis", "interfaces", "series"),
            label="AVO response widget source",
        )


class AvoInterceptGradientCrossplotWidget(_BaseAvoWidget):
    _esm_path = _STATIC_ROOT / "widget-avo-crossplot.js"
    _esm = _esm_path

    @classmethod
    def from_result(
        cls,
        result: Any,
        *,
        chart_id: str | None = None,
        height_px: int = 520,
        **source_kwargs: Any,
    ) -> AvoInterceptGradientCrossplotWidget:
        if not hasattr(result, "crossplot_source"):
            raise TypeError("result must expose a 'crossplot_source(...)' method")
        source = result.crossplot_source(**source_kwargs)
        return cls(source=source, chart_id=chart_id, height_px=height_px)

    def _validate_source_shape(self, source: Mapping[str, Any]) -> None:
        _require_mapping_keys(
            source,
            required_keys=("schema_version", "id", "name", "x_axis", "y_axis", "interfaces", "points"),
            label="AVO crossplot widget source",
        )


def _require_mapping_keys(
    source: Mapping[str, Any],
    *,
    required_keys: tuple[str, ...],
    label: str,
) -> None:
    missing = [key for key in required_keys if key not in source]
    if missing:
        joined = ", ".join(missing)
        raise ValueError(f"{label} is missing required keys: {joined}")


def _ensure_widget_assets_exist(path: Path) -> None:
    if path.exists():
        return
    raise RuntimeError(
        (
            f"expected widget asset '{path}' to exist. Build the notebook frontend with "
            "'bun run --cwd ./charts --filter @ophiolite/charts-jupyter-host build' from the repo root."
        )
    )
