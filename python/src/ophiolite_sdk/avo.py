from __future__ import annotations

from dataclasses import dataclass
import warnings
from typing import TYPE_CHECKING, Any, Sequence

from .models import AvoReflectivityResponse

if TYPE_CHECKING:
    from .logs import AvoInterface, AvoInterfaceModel, ElasticLayer, ElasticLayerModel, ElasticLogSet
    from .wells import Wellbore

__all__ = [
    "CurveSelector",
    "ElasticChannelBindings",
    "LayeringSpec",
    "AngleSampling",
    "AvoExperiment",
    "AvoResult",
]


_FEET_TO_METERS = 0.3048


@dataclass(frozen=True)
class CurveSelector:
    curve_name: str
    asset_name: str | None = None


@dataclass(frozen=True)
class ElasticChannelBindings:
    vp: str | CurveSelector | None = None
    vs: str | CurveSelector | None = None
    density: str | CurveSelector | None = None


@dataclass(frozen=True)
class LayeringSpec:
    kind: str
    thickness_value: float | None = None
    unit: str = "m"
    depth_edges: tuple[float, ...] = ()
    labels: tuple[str, ...] = ()
    selectors: tuple[str, ...] = ()
    asset_name: str | None = None
    min_samples_per_layer: int = 3
    include_partial_last_layer: bool = False
    allow_gaps: bool = False

    @classmethod
    def fixed_interval(
        cls,
        thickness: float,
        *,
        unit: str = "m",
        min_samples_per_layer: int = 3,
        include_partial_last_layer: bool = False,
    ) -> LayeringSpec:
        return cls(
            kind="fixed_interval",
            thickness_value=float(thickness),
            unit=unit,
            min_samples_per_layer=min_samples_per_layer,
            include_partial_last_layer=include_partial_last_layer,
        )

    @classmethod
    def from_edges(
        cls,
        depth_edges: Sequence[float],
        *,
        labels: Sequence[str] | None = None,
        unit: str = "m",
        min_samples_per_layer: int = 3,
    ) -> LayeringSpec:
        return cls(
            kind="explicit_edges",
            depth_edges=tuple(float(value) for value in depth_edges),
            labels=tuple(labels or ()),
            unit=unit,
            min_samples_per_layer=min_samples_per_layer,
        )

    @classmethod
    def from_top_set(
        cls,
        asset_name: str | None = None,
        *,
        labels: Sequence[str] | None = None,
        selectors: Sequence[str] | None = None,
        min_samples_per_layer: int = 3,
        allow_gaps: bool = True,
    ) -> LayeringSpec:
        if labels and selectors:
            raise ValueError("LayeringSpec.from_top_set accepts either labels or selectors, not both")
        return cls(
            kind="top_set",
            asset_name=asset_name,
            labels=tuple(labels or ()),
            selectors=tuple(selectors or ()),
            min_samples_per_layer=min_samples_per_layer,
            allow_gaps=allow_gaps,
        )

    def thickness_m(self) -> float:
        if self.kind != "fixed_interval" or self.thickness_value is None:
            raise ValueError("thickness_m is only defined for fixed-interval layering")
        normalized_unit = self.unit.strip().lower()
        if normalized_unit in {"m", "meter", "meters", "metre", "metres"}:
            return self.thickness_value
        if normalized_unit in {"ft", "foot", "feet"}:
            return self.thickness_value * _FEET_TO_METERS
        raise ValueError(f"unsupported layering unit '{self.unit}'. Supported units: m, ft")

    def depth_edges_m(self) -> tuple[float, ...]:
        if self.kind != "explicit_edges":
            raise ValueError("depth_edges_m is only defined for explicit-edge layering")
        normalized_unit = self.unit.strip().lower()
        if normalized_unit in {"m", "meter", "meters", "metre", "metres"}:
            return self.depth_edges
        if normalized_unit in {"ft", "foot", "feet"}:
            return tuple(value * _FEET_TO_METERS for value in self.depth_edges)
        raise ValueError(f"unsupported layering unit '{self.unit}'. Supported units: m, ft")

    def build(self, elastic: ElasticLogSet) -> tuple[ElasticLayerModel, AvoInterfaceModel]:
        if self.kind == "fixed_interval":
            layer_model = elastic.build_elastic_layer_model(
                interval_thickness_m=self.thickness_m(),
                min_samples_per_layer=self.min_samples_per_layer,
                include_partial_last_layer=self.include_partial_last_layer,
            )
            return layer_model, layer_model.to_avo_interface_model(allow_gaps=False)

        if self.kind == "explicit_edges":
            layer_model = elastic.build_elastic_layer_model_from_edges(
                self.depth_edges_m(),
                labels=self.labels or None,
                min_samples_per_layer=self.min_samples_per_layer,
            )
            return layer_model, layer_model.to_avo_interface_model(allow_gaps=False)

        if self.kind == "top_set":
            layer_model = elastic.build_elastic_layer_model_from_top_set(
                asset_name=self.asset_name,
                labels=self.labels or None,
                selectors=self.selectors or None,
                min_samples_per_layer=self.min_samples_per_layer,
            )
            return layer_model, layer_model.to_avo_interface_model(allow_gaps=self.allow_gaps)

        raise ValueError(f"unsupported layering kind '{self.kind}'")


@dataclass(frozen=True)
class AngleSampling:
    values: tuple[float, ...]

    @classmethod
    def range(cls, start_deg: float, stop_deg: float, step_deg: float) -> AngleSampling:
        start = float(start_deg)
        stop = float(stop_deg)
        step = float(step_deg)
        if step <= 0.0:
            raise ValueError("step_deg must be a positive number")
        if stop < start:
            raise ValueError("stop_deg must be greater than or equal to start_deg")

        values: list[float] = []
        current = start
        epsilon = step * 1.0e-9
        while current <= stop + epsilon:
            values.append(round(current, 10))
            current += step
        return cls(values=tuple(values))

    @classmethod
    def explicit(cls, values: Sequence[float]) -> AngleSampling:
        resolved = tuple(float(value) for value in values)
        if not resolved:
            raise ValueError("AngleSampling.explicit requires at least one angle")
        return cls(values=resolved)


@dataclass(frozen=True)
class AvoExperiment:
    method: str
    angles: AngleSampling

    @classmethod
    def zoeppritz(cls, *, angles: AngleSampling) -> AvoExperiment:
        return cls(method="zoeppritz", angles=angles)

    @classmethod
    def shuey_two_term(cls, *, angles: AngleSampling) -> AvoExperiment:
        return cls(method="shuey_two_term", angles=angles)


@dataclass(frozen=True)
class AvoResult:
    wellbore: Wellbore
    layering: LayeringSpec
    experiment: AvoExperiment
    layer_model: ElasticLayerModel
    interface_model: AvoInterfaceModel
    response: AvoReflectivityResponse

    @property
    def layers(self) -> tuple[ElasticLayer, ...]:
        return self.layer_model.layers

    @property
    def interfaces(self) -> tuple[AvoInterface, ...]:
        return self.interface_model.interfaces

    def chart_source(
        self,
        *,
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        warnings.warn(
            (
                "'AvoResult.chart_source()' is deprecated and will move after the "
                "current preview cycle. Use 'AvoResult.response_source()' instead."
            ),
            DeprecationWarning,
            stacklevel=2,
        )
        return self.response_source(
            interface_labels=interface_labels,
            title=title,
            subtitle=subtitle,
            colors=colors,
            source_id=source_id,
            name=name,
        )

    def response_source(
        self,
        *,
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        return self.interface_model.to_avo_response_source(
            self.response,
            interface_labels=interface_labels,
            title=title,
            subtitle=subtitle,
            colors=colors,
            source_id=source_id,
            name=name,
        )

    def crossplot_source(
        self,
        *,
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        if not self.response.intercept() or not self.response.gradient():
            raise ValueError(
                "AVO crossplot handoff requires an experiment that produces intercept and "
                "gradient attributes, such as 'shuey_two_term'"
            )
        return self.interface_model.to_avo_crossplot_source(
            self.response,
            interface_labels=interface_labels,
            title=title,
            subtitle=subtitle,
            colors=colors,
            source_id=source_id,
            name=name,
        )
