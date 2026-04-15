from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Mapping


def _decode_parameter_value(value: Any) -> Any:
    if not isinstance(value, Mapping) or len(value) != 1:
        return value
    kind, raw_value = next(iter(value.items()))
    if kind in {"Number", "String", "Boolean"}:
        return raw_value
    return value


def _decode_parameters(parameters: Mapping[str, Any]) -> dict[str, Any]:
    return {
        name: _decode_parameter_value(value)
        for name, value in parameters.items()
    }


@dataclass(frozen=True)
class OperatorRequest:
    operator_id: str
    package_name: str
    package_version: str
    parameters: dict[str, Any]
    payload: dict[str, Any]

    @property
    def kind(self) -> str:
        kind = self.payload.get("kind")
        if not isinstance(kind, str):
            raise ValueError("external operator payload is missing a string kind")
        return kind

    @classmethod
    def from_payload(cls, payload: Mapping[str, Any]) -> "OperatorRequest":
        return cls(
            operator_id=str(payload["operator_id"]),
            package_name=str(payload["package_name"]),
            package_version=str(payload["package_version"]),
            parameters=_decode_parameters(
                payload.get("parameters", {}) if isinstance(payload, Mapping) else {}
            ),
            payload=dict(payload["payload"]),
        )


def computed_curve(
    curve_name: str,
    values: list[float | None],
    *,
    original_mnemonic: str | None = None,
    unit: str | None = None,
    description: str | None = None,
    semantic_type: str = "Computed",
) -> dict[str, Any]:
    return {
        "curve_name": curve_name,
        "original_mnemonic": original_mnemonic or curve_name,
        "unit": unit,
        "description": description,
        "semantic_type": semantic_type,
        "values": values,
    }


class OperatorRegistry:
    def __init__(self) -> None:
        self._operators: dict[str, Callable[[OperatorRequest], Any]] = {}

    def register(
        self, operator_id: str
    ) -> Callable[[Callable[[OperatorRequest], Any]], Callable[[OperatorRequest], Any]]:
        def decorator(func: Callable[[OperatorRequest], Any]) -> Callable[[OperatorRequest], Any]:
            self._operators[operator_id] = func
            return func

        return decorator

    def invoke(self, payload: Mapping[str, Any]) -> dict[str, Any]:
        request = OperatorRequest.from_payload(payload)
        handler = self._operators.get(request.operator_id)
        if handler is None:
            raise KeyError(f"external operator '{request.operator_id}' is not registered")
        result = handler(request)
        return self._normalize_response(request, result)

    def _normalize_response(
        self, request: OperatorRequest, result: Any
    ) -> dict[str, Any]:
        if isinstance(result, Mapping) and "payload" in result:
            return dict(result)
        if request.kind == "log":
            if not isinstance(result, Mapping):
                raise TypeError(
                    "log operators must return a computed curve mapping or a full response"
                )
            return {
                "payload": {
                    "kind": "log",
                    "computed_curve": dict(result),
                }
            }
        if not isinstance(result, list):
            raise TypeError(
                f"{request.kind} operators must return a list of rows or a full response"
            )
        return {
            "payload": {
                "kind": request.kind,
                "rows": result,
            }
        }
