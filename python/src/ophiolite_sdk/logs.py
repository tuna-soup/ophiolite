from __future__ import annotations

from bisect import bisect_left
from dataclasses import dataclass
import math
from typing import TYPE_CHECKING, Any, Mapping, Sequence

from .analysis import avo_reflectivity
from .models import AvoReflectivityRequest, AvoReflectivityResponse, ComputeRequest, ComputeRun

if TYPE_CHECKING:
    from .avo import AvoExperiment, AvoResult, ElasticChannelBindings, LayeringSpec
    from .wells import Wellbore


_DEFAULT_INTERFACE_COLORS = (
    "#0B7285",
    "#D9480F",
    "#2B8A3E",
    "#C2255C",
    "#E67700",
    "#1C7ED6",
)

_EPSILON = 1.0e-9
_MEASURED_DEPTH_INTERVAL_REFERENCES = {
    "",
    "MD",
    "MEASURED DEPTH",
    "MEASURED_DEPTH",
    "KB",
    "KELLY BUSHING",
    "RT",
    "ROTARY TABLE",
    "DF",
    "DRILL FLOOR",
}


_LOG_TYPE_SELECTOR_ALIASES = {
    "gr": "GammaRay",
    "gamma": "GammaRay",
    "gammaray": "GammaRay",
    "rhob": "BulkDensity",
    "rho": "BulkDensity",
    "density": "BulkDensity",
    "bulkdensity": "BulkDensity",
    "bdcx": "BulkDensity",
    "nphi": "NeutronPorosity",
    "cnl": "NeutronPorosity",
    "neutronporosity": "NeutronPorosity",
    "dt": "CompressionalSlowness",
    "dtc": "CompressionalSlowness",
    "dtco": "CompressionalSlowness",
    "sonic": "CompressionalSlowness",
    "compressionalsonic": "CompressionalSlowness",
    "compressionalslowness": "CompressionalSlowness",
    "dts": "ShearSlowness",
    "dtsm": "ShearSlowness",
    "dtsh": "ShearSlowness",
    "shearsonic": "ShearSlowness",
    "shearslowness": "ShearSlowness",
    "vp": "PVelocity",
    "pvelocity": "PVelocity",
    "pwavevelocity": "PVelocity",
    "vs": "SVelocity",
    "svelocity": "SVelocity",
    "swavevelocity": "SVelocity",
    "depth": "Depth",
}

_INTERVAL_SET_KIND_ALIASES = {
    "top": "top_set",
    "tops": "top_set",
    "topset": "top_set",
    "top_set": "top_set",
    "marker": "well_marker_set",
    "markers": "well_marker_set",
    "markerset": "well_marker_set",
    "marker_set": "well_marker_set",
    "wellmarker": "well_marker_set",
    "wellmarkers": "well_marker_set",
    "wellmarkerset": "well_marker_set",
    "well_marker_set": "well_marker_set",
}


def _curve_log_type_from_semantic_type(semantic_type: str) -> str:
    return {
        "Sonic": "CompressionalSlowness",
        "ShearSonic": "ShearSlowness",
    }.get(semantic_type, semantic_type)


def normalize_log_type_selector(selector: str) -> str:
    normalized = selector.strip().replace(" ", "").replace("_", "")
    if not normalized:
        raise ValueError("log type selectors must be non-empty")
    return _LOG_TYPE_SELECTOR_ALIASES.get(normalized.lower(), selector.strip())


def normalize_interval_set_kind(selector: str) -> str:
    normalized = selector.strip().replace(" ", "").replace("-", "").replace("_", "")
    if not normalized:
        raise ValueError("interval set kind selectors must be non-empty")
    return _INTERVAL_SET_KIND_ALIASES.get(normalized.lower(), selector.strip())


def _normalize_name_token(value: str) -> str:
    return value.strip().casefold()


def _string(value: Any, field_name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"expected '{field_name}' to be a string")
    return value


def _optional_string(value: Any, field_name: str) -> str | None:
    if value is None:
        return None
    return _string(value, field_name)


def _float_list(value: Any, field_name: str) -> tuple[float, ...]:
    if not isinstance(value, list):
        raise ValueError(f"expected '{field_name}' to be a list")
    return tuple(float(item) for item in value)


def _optional_float_list(value: Any, field_name: str) -> tuple[float | None, ...]:
    if not isinstance(value, list):
        raise ValueError(f"expected '{field_name}' to be a list")
    return tuple(None if item is None else float(item) for item in value)


def _mapping_list(value: Any, field_name: str) -> tuple[Mapping[str, Any], ...]:
    if not isinstance(value, list):
        raise ValueError(f"expected '{field_name}' to be a list")
    result = []
    for item in value:
        if not isinstance(item, Mapping):
            raise ValueError(f"expected '{field_name}' items to be objects")
        result.append(item)
    return tuple(result)


def _sample_count(shape: Sequence[int]) -> int:
    count = 1
    for dim in shape:
        count *= int(dim)
    return count


def _normalize_avo_method(method: str) -> str:
    normalized = method.strip().lower().replace("-", "_")
    aliases = {
        "zoeppritz": "zoeppritz_pp",
        "zoeppritz_pp": "zoeppritz_pp",
        "approx_zoeppritz": "approx_zoeppritz_pp",
        "approx_zoeppritz_pp": "approx_zoeppritz_pp",
        "shuey_two_term": "shuey_two_term",
        "shuey_three_term": "shuey_three_term",
        "aki_richards": "aki_richards",
        "aki_richards_alt": "aki_richards_alt",
        "fatti": "fatti",
        "bortfeld": "bortfeld",
        "hilterman": "hilterman",
    }
    try:
        return aliases[normalized]
    except KeyError as exc:
        supported = ", ".join(sorted(aliases))
        raise ValueError(f"unsupported AVO method '{method}'. Supported methods: {supported}") from exc


def _reflectivity_model_for_chart(method: str) -> str:
    normalized = _normalize_avo_method(method)
    if normalized == "zoeppritz_pp":
        return "zoeppritz"
    return normalized


def _normalized_unit(unit: str | None) -> str:
    return (unit or "").strip().lower().replace(" ", "")


def _slowness_to_velocity(slowness: float, unit: str | None) -> float | None:
    if not math.isfinite(slowness) or slowness <= 0.0:
        return None
    normalized_unit = _normalized_unit(unit)
    if "us/m" in normalized_unit:
        return 1_000_000.0 / slowness
    if "us/ft" in normalized_unit:
        return 304_800.0 / slowness
    if slowness > 1_000.0:
        return 1_000_000.0 / slowness
    return 304_800.0 / slowness


def _normalize_velocity_to_m_per_s(value: float, unit: str | None) -> float | None:
    if not math.isfinite(value):
        return None
    normalized_unit = _normalized_unit(unit)
    if "ft/s" in normalized_unit or "ft/sec" in normalized_unit:
        return value * 0.3048
    if "km/s" in normalized_unit or "km/sec" in normalized_unit:
        return value * 1_000.0
    return value


def _normalize_density_to_g_cc(value: float, unit: str | None) -> float | None:
    if not math.isfinite(value):
        return None
    normalized_unit = _normalized_unit(unit)
    if (
        "g/cc" in normalized_unit
        or "g/cm3" in normalized_unit
        or "g/cm^3" in normalized_unit
        or normalized_unit == "gcc"
    ):
        return value
    if "kg/m3" in normalized_unit or "kg/m^3" in normalized_unit:
        return value / 1_000.0
    return value


def _normalized_interval_depth_domain(
    depth_domain: str | None,
    source_depth_reference: str | None = None,
) -> str:
    if depth_domain is not None and depth_domain.strip():
        normalized = depth_domain.strip().upper()
        return normalized or "MD"
    if source_depth_reference is None:
        return "MD"
    normalized = source_depth_reference.strip().upper()
    return normalized or "MD"


def _finite_mean(values: Sequence[float]) -> float | None:
    if not values:
        return None
    total = 0.0
    count = 0
    for value in values:
        if math.isfinite(value):
            total += value
            count += 1
    if count == 0:
        return None
    return total / count


def _monotonic_depth_samples(
    depths_m: Sequence[float],
    values: Sequence[float | None],
) -> tuple[tuple[float, float | None], ...]:
    samples = [
        (float(depth), value)
        for depth, value in zip(depths_m, values)
        if math.isfinite(depth)
    ]
    if not samples:
        return ()
    if len(samples) == 1:
        return (samples[0],)

    ascending = all(samples[index][0] <= samples[index + 1][0] for index in range(len(samples) - 1))
    descending = all(samples[index][0] >= samples[index + 1][0] for index in range(len(samples) - 1))
    if descending and not ascending:
        samples.reverse()
    elif not ascending and not descending:
        samples.sort(key=lambda item: item[0])
    return tuple(samples)


def _regular_sample_centers(
    depth_min_m: float,
    depth_max_m: float,
    sample_step_m: float,
) -> tuple[float, ...]:
    span = depth_max_m - depth_min_m
    if span <= 0.0:
        return ()
    if span <= sample_step_m:
        return (depth_min_m + (0.5 * span),)

    centers: list[float] = []
    depth_m = depth_min_m + (0.5 * sample_step_m)
    while depth_m < depth_max_m - _EPSILON:
        centers.append(depth_m)
        depth_m += sample_step_m
    if not centers:
        centers.append(depth_min_m + (0.5 * span))
    return tuple(centers)


def _series_values_for_sample(
    response: AvoReflectivityResponse,
    sample_index: int,
) -> tuple[float, ...]:
    sample_count = _sample_count(response.sample_shape)
    pp_real = response.pp_real()
    angle_count = len(response.angles_deg)
    return tuple(pp_real[(angle_index * sample_count) + sample_index] for angle_index in range(angle_count))


def _well_panel_top_sets(panel: Mapping[str, Any], wellbore_id: str) -> tuple[Mapping[str, Any], ...]:
    wells = panel.get("wells", [])
    if not isinstance(wells, list):
        raise ValueError("expected well-panel payload to contain a 'wells' list")
    for well in wells:
        if isinstance(well, Mapping) and well.get("wellbore_id") == wellbore_id:
            top_sets = well.get("top_sets", [])
            return _mapping_list(top_sets, "top_sets")
    return ()


@dataclass(frozen=True)
class WellLogCurve:
    asset_id: str
    logical_asset_id: str
    asset_name: str
    curve_name: str
    original_mnemonic: str
    unit: str | None
    semantic_type: str
    log_type: str
    depths_m: tuple[float, ...]
    values: tuple[float | None, ...]

    @classmethod
    def from_panel_json(cls, data: Mapping[str, Any]) -> WellLogCurve:
        return cls(
            asset_id=_string(data["asset_id"], "asset_id"),
            logical_asset_id=_string(data["logical_asset_id"], "logical_asset_id"),
            asset_name=_string(data["asset_name"], "asset_name"),
            curve_name=_string(data["curve_name"], "curve_name"),
            original_mnemonic=_string(data["original_mnemonic"], "original_mnemonic"),
            unit=_optional_string(data.get("unit"), "unit"),
            semantic_type=_string(data["semantic_type"], "semantic_type"),
            log_type=_string(
                data.get(
                    "log_type",
                    _curve_log_type_from_semantic_type(
                        _string(data["semantic_type"], "semantic_type")
                    ),
                ),
                "log_type",
            ),
            depths_m=_float_list(data.get("depths", []), "depths"),
            values=_optional_float_list(data.get("values", []), "values"),
        )

    @property
    def valid_sample_count(self) -> int:
        return sum(1 for value in self.values if value is not None)

    @property
    def depth_range_m(self) -> tuple[float, float] | None:
        valid_depths = [
            depth
            for depth, value in zip(self.depths_m, self.values)
            if value is not None and math.isfinite(depth)
        ]
        if not valid_depths:
            return None
        return (min(valid_depths), max(valid_depths))

    @property
    def estimated_step_m(self) -> float | None:
        samples = _monotonic_depth_samples(self.depths_m, self.values)
        if len(samples) < 2:
            return None
        deltas = [
            abs(samples[index + 1][0] - samples[index][0])
            for index in range(len(samples) - 1)
            if abs(samples[index + 1][0] - samples[index][0]) > _EPSILON
        ]
        if not deltas:
            return None
        deltas.sort()
        midpoint = len(deltas) // 2
        if len(deltas) % 2 == 1:
            return deltas[midpoint]
        return 0.5 * (deltas[midpoint - 1] + deltas[midpoint])

    def sample_at(self, depth_m: float) -> float | None:
        samples = _monotonic_depth_samples(self.depths_m, self.values)
        if not samples:
            return None

        depths = [depth for depth, _value in samples]
        values = [value for _depth, value in samples]
        if len(samples) == 1:
            only_value = values[0]
            return only_value if only_value is not None and math.isclose(depth_m, depths[0]) else None

        if depth_m < depths[0] or depth_m > depths[-1]:
            return None

        index = bisect_left(depths, depth_m)
        if index < len(depths) and math.isclose(depths[index], depth_m):
            exact_value = values[index]
            return exact_value if exact_value is not None and math.isfinite(exact_value) else None
        if index == 0 or index >= len(depths):
            return None

        left_depth = depths[index - 1]
        right_depth = depths[index]
        left_value = values[index - 1]
        right_value = values[index]
        if (
            left_value is None
            or right_value is None
            or not math.isfinite(left_value)
            or not math.isfinite(right_value)
        ):
            return None
        span = right_depth - left_depth
        if span <= 0.0:
            return left_value
        weight = (depth_m - left_depth) / span
        return left_value + ((right_value - left_value) * weight)


@dataclass(frozen=True)
class ResolvedElasticChannel:
    semantic_type: str
    source_curve: WellLogCurve
    source_semantic_type: str
    derivation: str | None = None

    @property
    def is_direct(self) -> bool:
        return self.derivation is None

    @property
    def depth_range_m(self) -> tuple[float, float] | None:
        return self.source_curve.depth_range_m

    @property
    def estimated_step_m(self) -> float | None:
        return self.source_curve.estimated_step_m

    def sample_at(self, depth_m: float) -> float | None:
        value = self.source_curve.sample_at(depth_m)
        if value is None:
            return None
        if self.derivation in {"sonic_to_vp", "shear_sonic_to_vs"}:
            return _slowness_to_velocity(value, self.source_curve.unit)
        if self.semantic_type in {"PVelocity", "SVelocity"}:
            return _normalize_velocity_to_m_per_s(value, self.source_curve.unit)
        if self.semantic_type == "BulkDensity":
            return _normalize_density_to_g_cc(value, self.source_curve.unit)
        return value


@dataclass(frozen=True)
class LogSetChannel:
    name: str
    semantic_type: str
    unit: str | None
    source_semantic_type: str
    derivation: str | None
    values: tuple[float | None, ...]

    @property
    def valid_sample_count(self) -> int:
        return sum(1 for value in self.values if value is not None)


@dataclass(frozen=True)
class WellInterval:
    interval_id: str
    label: str
    top_depth_m: float
    base_depth_m: float
    source: str | None = None
    source_depth_reference: str | None = None
    depth_domain: str | None = None
    depth_datum: str | None = None
    top_set_name: str | None = None

    @property
    def thickness_m(self) -> float:
        return self.base_depth_m - self.top_depth_m

    @property
    def depth_reference(self) -> str | None:
        return self.source_depth_reference or self.depth_domain

    @classmethod
    def from_top_row(
        cls,
        data: Mapping[str, Any],
        *,
        interval_id: str,
        top_set_name: str | None = None,
    ) -> WellInterval:
        top_depth_m = float(data["top_depth"])
        base_depth = data.get("base_depth")
        if base_depth is None:
            raise ValueError("top-set interval requires a finite base_depth")
        base_depth_m = float(base_depth)
        return cls(
            interval_id=interval_id,
            label=_string(data["name"], "name"),
            top_depth_m=top_depth_m,
            base_depth_m=base_depth_m,
            source=_optional_string(data.get("source"), "source"),
            source_depth_reference=_optional_string(
                data.get("source_depth_reference", data.get("depth_reference")),
                "source_depth_reference",
            ),
            depth_domain=_optional_string(data.get("depth_domain"), "depth_domain"),
            depth_datum=_optional_string(data.get("depth_datum"), "depth_datum"),
            top_set_name=top_set_name,
        )


@dataclass(frozen=True)
class WellTopSet:
    asset_id: str
    logical_asset_id: str
    asset_name: str
    intervals: tuple[WellInterval, ...]
    set_kind: str = "top_set"

    @property
    def is_top_set(self) -> bool:
        return self.set_kind == "top_set"

    @property
    def is_marker_set(self) -> bool:
        return self.set_kind == "well_marker_set"

    @property
    def labels(self) -> tuple[str, ...]:
        return tuple(interval.label for interval in self.intervals)

    @property
    def interval_selectors(self) -> tuple[str, ...]:
        label_totals: dict[str, int] = {}
        for interval in self.intervals:
            key = _normalize_name_token(interval.label)
            label_totals[key] = label_totals.get(key, 0) + 1

        label_seen: dict[str, int] = {}
        selectors: list[str] = []
        for interval in self.intervals:
            key = _normalize_name_token(interval.label)
            occurrence = label_seen.get(key, 0) + 1
            label_seen[key] = occurrence
            if label_totals[key] == 1:
                selectors.append(interval.label)
            else:
                selectors.append(f"{interval.label}#{occurrence}")
        return tuple(selectors)

    @property
    def depth_range_m(self) -> tuple[float, float] | None:
        if not self.intervals:
            return None
        return (
            min(interval.top_depth_m for interval in self.intervals),
            max(interval.base_depth_m for interval in self.intervals),
        )

    def intervals_by_label(
        self,
        label: str,
        *,
        case_sensitive: bool = False,
    ) -> tuple[WellInterval, ...]:
        if case_sensitive:
            return tuple(interval for interval in self.intervals if interval.label == label)
        normalized_label = _normalize_name_token(label)
        return tuple(
            interval
            for interval in self.intervals
            if _normalize_name_token(interval.label) == normalized_label
        )

    def interval(
        self,
        label: str,
        *,
        case_sensitive: bool = False,
    ) -> WellInterval | None:
        matches = self.intervals_by_label(label, case_sensitive=case_sensitive)
        return matches[0] if matches else None

    def interval_by_selector(
        self,
        selector: str,
        *,
        case_sensitive: bool = False,
    ) -> WellInterval | None:
        normalized_selector = selector if case_sensitive else _normalize_name_token(selector)
        for interval_selector, interval in zip(self.interval_selectors, self.intervals):
            candidate = (
                interval_selector
                if case_sensitive
                else _normalize_name_token(interval_selector)
            )
            if candidate == normalized_selector:
                return interval
        return None

    def select_intervals(
        self,
        *,
        labels: Sequence[str] | None = None,
        selectors: Sequence[str] | None = None,
        case_sensitive: bool = False,
    ) -> tuple[WellInterval, ...]:
        if labels and selectors:
            raise ValueError("specify either labels or selectors, not both")

        if selectors:
            resolved = []
            for selector in selectors:
                interval = self.interval_by_selector(selector, case_sensitive=case_sensitive)
                if interval is None:
                    available = ", ".join(self.interval_selectors)
                    raise LookupError(
                        f"interval selector '{selector}' was not found in interval set "
                        f"'{self.asset_name}'. Available selectors: {available}"
                    )
                resolved.append(interval)
            return tuple(resolved)

        if labels:
            requested = (
                set(labels)
                if case_sensitive
                else {_normalize_name_token(label) for label in labels}
            )
            filtered = tuple(
                interval
                for interval in self.intervals
                if (
                    interval.label in requested
                    if case_sensitive
                    else _normalize_name_token(interval.label) in requested
                )
            )
            if not filtered:
                available = ", ".join(self.labels)
                raise LookupError(
                    f"requested interval labels were not found in interval set "
                    f"'{self.asset_name}'. Available labels: {available}"
                )
            return filtered

        return self.intervals

    def layering(
        self,
        *,
        labels: Sequence[str] | None = None,
        selectors: Sequence[str] | None = None,
        min_samples_per_layer: int = 3,
        allow_gaps: bool = True,
    ) -> LayeringSpec:
        from .avo import LayeringSpec

        return LayeringSpec.from_top_set(
            self.asset_name,
            labels=labels,
            selectors=selectors,
            min_samples_per_layer=min_samples_per_layer,
            allow_gaps=allow_gaps,
        )

    @classmethod
    def from_panel_json(cls, data: Mapping[str, Any]) -> WellTopSet:
        asset_name = _string(data["asset_name"], "asset_name")
        rows = _mapping_list(data.get("rows", []), "rows")
        intervals = []
        for index, row in enumerate(rows):
            if row.get("base_depth") is None:
                continue
            intervals.append(
                WellInterval.from_top_row(
                    row,
                    interval_id=f"{asset_name}-interval-{index + 1}",
                    top_set_name=asset_name,
                )
            )
        return cls(
            asset_id=_string(data["asset_id"], "asset_id"),
            logical_asset_id=_string(data["logical_asset_id"], "logical_asset_id"),
            asset_name=asset_name,
            intervals=tuple(intervals),
            set_kind=normalize_interval_set_kind(
                _optional_string(data.get("set_kind"), "set_kind") or "top_set"
            ),
        )


@dataclass(frozen=True)
class ElasticLayer:
    layer_id: str
    index: int
    top_depth_m: float
    base_depth_m: float
    sample_count: int
    vp_m_per_s: float
    vs_m_per_s: float
    density_g_cc: float

    @property
    def midpoint_depth_m(self) -> float:
        return 0.5 * (self.top_depth_m + self.base_depth_m)

    @property
    def thickness_m(self) -> float:
        return self.base_depth_m - self.top_depth_m


@dataclass(frozen=True)
class AvoInterface:
    interface_id: str
    index: int
    depth_m: float
    upper_layer: ElasticLayer
    lower_layer: ElasticLayer

    @property
    def default_label(self) -> str:
        return f"Interface {self.index + 1}"


@dataclass(frozen=True)
class AvoInterfaceModel:
    wellbore: Wellbore
    interfaces: tuple[AvoInterface, ...]
    layer_interval_m: float
    sample_step_m: float

    def build_reflectivity_request(
        self,
        *,
        angles_deg: Sequence[float],
        method: str = "zoeppritz",
    ) -> AvoReflectivityRequest:
        if not self.interfaces:
            raise ValueError("AVO interface modeling requires at least one resolved interface")
        if not angles_deg:
            raise ValueError("AVO reflectivity requires at least one angle")

        return AvoReflectivityRequest(
            method=_normalize_avo_method(method),
            sample_shape=(len(self.interfaces),),
            angles_deg=tuple(float(angle) for angle in angles_deg),
            upper_vp_m_per_s=tuple(interface.upper_layer.vp_m_per_s for interface in self.interfaces),
            upper_vs_m_per_s=tuple(interface.upper_layer.vs_m_per_s for interface in self.interfaces),
            upper_density_g_cc=tuple(
                interface.upper_layer.density_g_cc for interface in self.interfaces
            ),
            lower_vp_m_per_s=tuple(interface.lower_layer.vp_m_per_s for interface in self.interfaces),
            lower_vs_m_per_s=tuple(interface.lower_layer.vs_m_per_s for interface in self.interfaces),
            lower_density_g_cc=tuple(
                interface.lower_layer.density_g_cc for interface in self.interfaces
            ),
        )

    def run_reflectivity(
        self,
        *,
        angles_deg: Sequence[float],
        method: str = "zoeppritz",
    ) -> AvoReflectivityResponse:
        return avo_reflectivity(
            self.build_reflectivity_request(angles_deg=angles_deg, method=method),
            app=self.wellbore.project.app,
        )

    def to_avo_response_source(
        self,
        response: AvoReflectivityResponse,
        *,
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        sample_count = _sample_count(response.sample_shape)
        if sample_count != len(self.interfaces):
            raise ValueError("interface count does not match AVO response sample shape")

        labels = tuple(interface_labels or ())
        palette = tuple(colors or _DEFAULT_INTERFACE_COLORS)
        interfaces = []
        series = []
        reflectivity_model = _reflectivity_model_for_chart(response.method)
        resolved_name = name or f"{self.wellbore.name} AVO"

        for index, interface in enumerate(self.interfaces):
            color = palette[index % len(palette)]
            label = labels[index] if index < len(labels) else interface.default_label
            interfaces.append(
                {
                    "id": interface.interface_id,
                    "label": label,
                    "color": color,
                    "reservoir_label": f"{interface.depth_m:.1f} m",
                }
            )
            series.append(
                {
                    "id": f"{interface.interface_id}-series",
                    "interface_id": interface.interface_id,
                    "label": label,
                    "color": color,
                    "style": "solid",
                    "reflectivity_model": reflectivity_model,
                    "anisotropy_mode": "isotropic",
                    "incidence_angles_deg": [float(value) for value in response.angles_deg],
                    "values": list(_series_values_for_sample(response, index)),
                }
            )

        return {
            "schema_version": 1,
            "id": source_id or f"{self.wellbore.id}-avo-response",
            "name": resolved_name,
            "title": title or resolved_name,
            "subtitle": subtitle,
            "x_axis": {"label": "Incidence Angle", "unit": "deg"},
            "y_axis": {"label": "PP Reflectivity", "unit": "ratio"},
            "interfaces": interfaces,
            "series": series,
        }

    def to_avo_crossplot_source(
        self,
        response: AvoReflectivityResponse,
        *,
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        sample_count = _sample_count(response.sample_shape)
        if sample_count != len(self.interfaces):
            raise ValueError("interface count does not match AVO response sample shape")
        intercept = response.intercept()
        gradient = response.gradient()
        if len(intercept) != sample_count or len(gradient) != sample_count:
            raise ValueError(
                "AVO crossplot source requires a response with intercept and gradient payloads"
            )

        labels = tuple(interface_labels or ())
        palette = tuple(colors or _DEFAULT_INTERFACE_COLORS)
        interfaces = []
        points = []
        resolved_name = name or f"{self.wellbore.name} AVO Crossplot"

        for index, interface in enumerate(self.interfaces):
            color = palette[index % len(palette)]
            label = labels[index] if index < len(labels) else interface.default_label
            interfaces.append(
                {
                    "id": interface.interface_id,
                    "label": label,
                    "color": color,
                    "reservoir_label": f"{interface.depth_m:.1f} m",
                }
            )
            points.append(
                {
                    "interface_id": interface.interface_id,
                    "intercept": float(intercept[index]),
                    "gradient": float(gradient[index]),
                }
            )

        return {
            "schema_version": 1,
            "id": source_id or f"{self.wellbore.id}-avo-crossplot",
            "name": resolved_name,
            "title": title or resolved_name,
            "subtitle": subtitle,
            "x_axis": {"label": "Intercept", "unit": "ratio"},
            "y_axis": {"label": "Gradient", "unit": "ratio"},
            "interfaces": interfaces,
            "points": points,
        }


@dataclass(frozen=True)
class ElasticLayerModel:
    wellbore: Wellbore
    layers: tuple[ElasticLayer, ...]
    interval_thickness_m: float
    sample_step_m: float

    def to_avo_interface_model(self, *, allow_gaps: bool = False) -> AvoInterfaceModel:
        interfaces: list[AvoInterface] = []
        for index in range(len(self.layers) - 1):
            upper_layer = self.layers[index]
            lower_layer = self.layers[index + 1]
            if not allow_gaps and not math.isclose(
                upper_layer.base_depth_m,
                lower_layer.top_depth_m,
                abs_tol=1.0e-6,
            ):
                continue
            interfaces.append(
                AvoInterface(
                    interface_id=f"interface-{index + 1}",
                    index=index,
                    depth_m=lower_layer.top_depth_m,
                    upper_layer=upper_layer,
                    lower_layer=lower_layer,
                )
            )
        if not interfaces:
            raise ValueError("elastic layer model did not produce any adjacent interfaces")
        return AvoInterfaceModel(
            wellbore=self.wellbore,
            interfaces=tuple(interfaces),
            layer_interval_m=self.interval_thickness_m,
            sample_step_m=self.sample_step_m,
        )


@dataclass(frozen=True)
class LogSet:
    wellbore: Wellbore
    depths_m: tuple[float, ...]
    channels: Mapping[str, LogSetChannel]
    sample_step_m: float
    depth_min_m: float
    depth_max_m: float

    def channel(self, name: str) -> LogSetChannel:
        try:
            return self.channels[name]
        except KeyError as exc:
            available = ", ".join(sorted(self.channels))
            raise KeyError(f"log set channel '{name}' was not found. Available channels: {available}") from exc

    def build_elastic_layer_model(
        self,
        *,
        interval_thickness_m: float,
        min_samples_per_layer: int = 3,
        include_partial_last_layer: bool = False,
    ) -> ElasticLayerModel:
        if not math.isfinite(interval_thickness_m) or interval_thickness_m <= 0.0:
            raise ValueError("interval_thickness_m must be a positive finite number")
        if min_samples_per_layer < 1:
            raise ValueError("min_samples_per_layer must be at least 1")
        if not self.depths_m:
            raise ValueError("cannot build elastic layers from an empty log set")

        vp_channel = self.channel("vp")
        vs_channel = self.channel("vs")
        density_channel = self.channel("density")

        layer_top = self.depth_min_m
        layers: list[ElasticLayer] = []
        bin_index = 0
        while layer_top < self.depth_max_m - _EPSILON:
            layer_base = min(layer_top + interval_thickness_m, self.depth_max_m)
            if not include_partial_last_layer and layer_base - layer_top < interval_thickness_m - _EPSILON:
                break

            vp_samples: list[float] = []
            vs_samples: list[float] = []
            density_samples: list[float] = []
            for depth_m, vp_value, vs_value, density_value in zip(
                self.depths_m,
                vp_channel.values,
                vs_channel.values,
                density_channel.values,
            ):
                in_interval = layer_top <= depth_m < layer_base or (
                    include_partial_last_layer
                    and math.isclose(depth_m, layer_base, abs_tol=_EPSILON)
                    and math.isclose(layer_base, self.depth_max_m, abs_tol=_EPSILON)
                )
                if not in_interval:
                    continue
                if vp_value is None or vs_value is None or density_value is None:
                    continue
                vp_samples.append(vp_value)
                vs_samples.append(vs_value)
                density_samples.append(density_value)

            if len(vp_samples) >= min_samples_per_layer:
                vp_mean = _finite_mean(vp_samples)
                vs_mean = _finite_mean(vs_samples)
                density_mean = _finite_mean(density_samples)
                if vp_mean is not None and vs_mean is not None and density_mean is not None:
                    layers.append(
                        ElasticLayer(
                            layer_id=f"layer-{len(layers) + 1}",
                            index=len(layers),
                            top_depth_m=layer_top,
                            base_depth_m=layer_base,
                            sample_count=len(vp_samples),
                            vp_m_per_s=vp_mean,
                            vs_m_per_s=vs_mean,
                            density_g_cc=density_mean,
                        )
                    )

            bin_index += 1
            layer_top = self.depth_min_m + (bin_index * interval_thickness_m)

        if len(layers) < 2:
            raise ValueError(
                "elastic layering produced fewer than two valid layers; widen the depth range, "
                "increase interval thickness, or lower min_samples_per_layer"
            )

        return ElasticLayerModel(
            wellbore=self.wellbore,
            layers=tuple(layers),
            interval_thickness_m=interval_thickness_m,
            sample_step_m=self.sample_step_m,
        )

    def build_elastic_layer_model_from_intervals(
        self,
        intervals: Sequence[WellInterval],
        *,
        min_samples_per_layer: int = 3,
    ) -> ElasticLayerModel:
        if min_samples_per_layer < 1:
            raise ValueError("min_samples_per_layer must be at least 1")
        if not intervals:
            raise ValueError("at least one interval is required")

        vp_channel = self.channel("vp")
        vs_channel = self.channel("vs")
        density_channel = self.channel("density")
        layers: list[ElasticLayer] = []

        for interval in intervals:
            if not math.isfinite(interval.top_depth_m) or not math.isfinite(interval.base_depth_m):
                raise ValueError(f"interval '{interval.label}' requires finite top/base depths")
            if interval.base_depth_m <= interval.top_depth_m:
                raise ValueError(f"interval '{interval.label}' base depth must exceed top depth")

            depth_domain = _normalized_interval_depth_domain(
                interval.depth_domain,
                interval.source_depth_reference,
            )
            if depth_domain not in _MEASURED_DEPTH_INTERVAL_REFERENCES:
                raise ValueError(
                    f"interval '{interval.label}' uses depth domain '{depth_domain}', "
                    "but elastic log layering currently requires measured depth intervals"
                )

            vp_samples: list[float] = []
            vs_samples: list[float] = []
            density_samples: list[float] = []
            for depth_m, vp_value, vs_value, density_value in zip(
                self.depths_m,
                vp_channel.values,
                vs_channel.values,
                density_channel.values,
            ):
                in_interval = interval.top_depth_m <= depth_m < interval.base_depth_m or (
                    math.isclose(depth_m, interval.base_depth_m, abs_tol=_EPSILON)
                    and math.isclose(interval.base_depth_m, self.depth_max_m, abs_tol=_EPSILON)
                )
                if not in_interval:
                    continue
                if vp_value is None or vs_value is None or density_value is None:
                    continue
                vp_samples.append(vp_value)
                vs_samples.append(vs_value)
                density_samples.append(density_value)

            if len(vp_samples) < min_samples_per_layer:
                continue

            vp_mean = _finite_mean(vp_samples)
            vs_mean = _finite_mean(vs_samples)
            density_mean = _finite_mean(density_samples)
            if vp_mean is None or vs_mean is None or density_mean is None:
                continue

            layers.append(
                ElasticLayer(
                    layer_id=interval.interval_id,
                    index=len(layers),
                    top_depth_m=interval.top_depth_m,
                    base_depth_m=interval.base_depth_m,
                    sample_count=len(vp_samples),
                    vp_m_per_s=vp_mean,
                    vs_m_per_s=vs_mean,
                    density_g_cc=density_mean,
                )
            )

        if len(layers) < 2:
            raise ValueError(
                "interval layering produced fewer than two valid layers; widen the intervals "
                "or lower min_samples_per_layer"
            )

        thicknesses = [layer.thickness_m for layer in layers if layer.thickness_m > 0.0]
        representative_interval_m = _finite_mean(thicknesses) or self.sample_step_m
        return ElasticLayerModel(
            wellbore=self.wellbore,
            layers=tuple(layers),
            interval_thickness_m=representative_interval_m,
            sample_step_m=self.sample_step_m,
        )


@dataclass(frozen=True)
class ElasticLogSet:
    wellbore: Wellbore
    vp: ResolvedElasticChannel
    vs: ResolvedElasticChannel
    density: ResolvedElasticChannel

    def run_avo(
        self,
        *,
        layering: LayeringSpec,
        experiment: AvoExperiment,
    ) -> AvoResult:
        from .avo import AvoResult

        layer_model, interface_model = layering.build(self)
        response = interface_model.run_reflectivity(
            angles_deg=experiment.angles.values,
            method=experiment.method,
        )
        return AvoResult(
            wellbore=self.wellbore,
            layering=layering,
            experiment=experiment,
            layer_model=layer_model,
            interface_model=interface_model,
            response=response,
        )

    def align_log_set(
        self,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
        sample_step_m: float | None = None,
    ) -> LogSet:
        ranges = [channel.depth_range_m for channel in (self.vp, self.vs, self.density)]
        if any(item is None for item in ranges):
            raise ValueError("elastic log alignment requires finite depth coverage for Vp, Vs, and density")

        overlap_min = max(item[0] for item in ranges if item is not None)
        overlap_max = min(item[1] for item in ranges if item is not None)
        if depth_min is not None:
            overlap_min = max(overlap_min, float(depth_min))
        if depth_max is not None:
            overlap_max = min(overlap_max, float(depth_max))
        if not math.isfinite(overlap_min) or not math.isfinite(overlap_max) or overlap_max <= overlap_min:
            raise ValueError("elastic log channels do not overlap on a usable depth interval")

        resolved_step_m = sample_step_m
        if resolved_step_m is None:
            candidate_steps = [
                step
                for step in (
                    self.vp.estimated_step_m,
                    self.vs.estimated_step_m,
                    self.density.estimated_step_m,
                )
                if step is not None and math.isfinite(step) and step > 0.0
            ]
            if not candidate_steps:
                raise ValueError("could not infer a regular sampling step from the selected elastic logs")
            resolved_step_m = max(candidate_steps)
        if not math.isfinite(resolved_step_m) or resolved_step_m <= 0.0:
            raise ValueError("sample_step_m must be a positive finite number")

        sample_depths_m = _regular_sample_centers(overlap_min, overlap_max, resolved_step_m)
        if not sample_depths_m:
            raise ValueError("elastic log alignment did not yield any aligned sample depths")

        return LogSet(
            wellbore=self.wellbore,
            depths_m=sample_depths_m,
            channels={
                "vp": LogSetChannel(
                    name="vp",
                    semantic_type="PVelocity",
                    unit="m/s",
                    source_semantic_type=self.vp.source_semantic_type,
                    derivation=self.vp.derivation,
                    values=tuple(self.vp.sample_at(depth_m) for depth_m in sample_depths_m),
                ),
                "vs": LogSetChannel(
                    name="vs",
                    semantic_type="SVelocity",
                    unit="m/s",
                    source_semantic_type=self.vs.source_semantic_type,
                    derivation=self.vs.derivation,
                    values=tuple(self.vs.sample_at(depth_m) for depth_m in sample_depths_m),
                ),
                "density": LogSetChannel(
                    name="density",
                    semantic_type="BulkDensity",
                    unit="g/cc",
                    source_semantic_type=self.density.source_semantic_type,
                    derivation=self.density.derivation,
                    values=tuple(self.density.sample_at(depth_m) for depth_m in sample_depths_m),
                ),
            },
            sample_step_m=resolved_step_m,
            depth_min_m=overlap_min,
            depth_max_m=overlap_max,
        )

    def build_elastic_layer_model(
        self,
        *,
        interval_thickness_m: float,
        depth_min: float | None = None,
        depth_max: float | None = None,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
        include_partial_last_layer: bool = False,
    ) -> ElasticLayerModel:
        return self.align_log_set(
            depth_min=depth_min,
            depth_max=depth_max,
            sample_step_m=sample_step_m,
        ).build_elastic_layer_model(
            interval_thickness_m=interval_thickness_m,
            min_samples_per_layer=min_samples_per_layer,
            include_partial_last_layer=include_partial_last_layer,
        )

    def intervals_from_edges(
        self,
        depth_edges_m: Sequence[float],
        *,
        labels: Sequence[str] | None = None,
    ) -> tuple[WellInterval, ...]:
        if len(depth_edges_m) < 2:
            raise ValueError("depth_edges_m must contain at least two depths")

        normalized_edges = [float(depth) for depth in depth_edges_m]
        if any(not math.isfinite(depth) for depth in normalized_edges):
            raise ValueError("depth_edges_m must contain only finite depths")
        if any(
            normalized_edges[index + 1] <= normalized_edges[index]
            for index in range(len(normalized_edges) - 1)
        ):
            raise ValueError("depth_edges_m must be strictly increasing")

        resolved_labels = tuple(labels or ())
        intervals = []
        for index in range(len(normalized_edges) - 1):
            label = (
                resolved_labels[index]
                if index < len(resolved_labels)
                else f"Interval {index + 1}"
            )
            intervals.append(
                WellInterval(
                    interval_id=f"edge-interval-{index + 1}",
                    label=label,
                    top_depth_m=normalized_edges[index],
                    base_depth_m=normalized_edges[index + 1],
                    source="explicit-depth-edges",
                    depth_domain="md",
                )
            )
        return tuple(intervals)

    def top_sets(self) -> tuple[WellTopSet, ...]:
        panel = self.wellbore.panel()
        return tuple(
            WellTopSet.from_panel_json(item)
            for item in _well_panel_top_sets(panel, self.wellbore.id)
            if normalize_interval_set_kind(item.get("set_kind", "top_set")) == "top_set"
        )

    def intervals_from_top_set(
        self,
        *,
        asset_name: str | None = None,
        labels: Sequence[str] | None = None,
        selectors: Sequence[str] | None = None,
    ) -> tuple[WellInterval, ...]:
        top_sets = self.top_sets()
        if not top_sets:
            raise LookupError(f"wellbore '{self.wellbore.id}' does not provide any top sets")

        selected_top_set = None
        if asset_name is None:
            selected_top_set = top_sets[0]
        else:
            for top_set in top_sets:
                if top_set.asset_name == asset_name:
                    selected_top_set = top_set
                    break
        if selected_top_set is None:
            available = ", ".join(top_set.asset_name for top_set in top_sets)
            raise LookupError(
                f"top set '{asset_name}' was not found on wellbore '{self.wellbore.id}'. "
                f"Available top sets: {available}"
            )

        intervals = selected_top_set.intervals
        if not intervals:
            raise ValueError(
                f"top set '{selected_top_set.asset_name}' does not contain any intervals with base depths"
            )

        return selected_top_set.select_intervals(labels=labels, selectors=selectors)

    def build_elastic_layer_model_from_edges(
        self,
        depth_edges_m: Sequence[float],
        *,
        labels: Sequence[str] | None = None,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
    ) -> ElasticLayerModel:
        intervals = self.intervals_from_edges(depth_edges_m, labels=labels)
        return self.build_elastic_layer_model_from_intervals(
            intervals,
            sample_step_m=sample_step_m,
            min_samples_per_layer=min_samples_per_layer,
        )

    def build_elastic_layer_model_from_top_set(
        self,
        *,
        asset_name: str | None = None,
        labels: Sequence[str] | None = None,
        selectors: Sequence[str] | None = None,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
    ) -> ElasticLayerModel:
        intervals = self.intervals_from_top_set(
            asset_name=asset_name,
            labels=labels,
            selectors=selectors,
        )
        return self.build_elastic_layer_model_from_intervals(
            intervals,
            sample_step_m=sample_step_m,
            min_samples_per_layer=min_samples_per_layer,
        )

    def build_elastic_layer_model_from_intervals(
        self,
        intervals: Sequence[WellInterval],
        *,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
    ) -> ElasticLayerModel:
        if not intervals:
            raise ValueError("at least one interval is required")
        depth_min = min(interval.top_depth_m for interval in intervals)
        depth_max = max(interval.base_depth_m for interval in intervals)
        return self.align_log_set(
            depth_min=depth_min,
            depth_max=depth_max,
            sample_step_m=sample_step_m,
        ).build_elastic_layer_model_from_intervals(
            intervals,
            min_samples_per_layer=min_samples_per_layer,
        )

    def build_avo_interface_model(
        self,
        *,
        interval_thickness_m: float,
        depth_min: float | None = None,
        depth_max: float | None = None,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
        include_partial_last_layer: bool = False,
        allow_gaps: bool = False,
    ) -> AvoInterfaceModel:
        return self.build_elastic_layer_model(
            interval_thickness_m=interval_thickness_m,
            depth_min=depth_min,
            depth_max=depth_max,
            sample_step_m=sample_step_m,
            min_samples_per_layer=min_samples_per_layer,
            include_partial_last_layer=include_partial_last_layer,
        ).to_avo_interface_model(allow_gaps=allow_gaps)

    def build_avo_interface_model_from_edges(
        self,
        depth_edges_m: Sequence[float],
        *,
        labels: Sequence[str] | None = None,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
        allow_gaps: bool = False,
    ) -> AvoInterfaceModel:
        return self.build_elastic_layer_model_from_edges(
            depth_edges_m,
            labels=labels,
            sample_step_m=sample_step_m,
            min_samples_per_layer=min_samples_per_layer,
        ).to_avo_interface_model(allow_gaps=allow_gaps)

    def build_avo_interface_model_from_top_set(
        self,
        *,
        asset_name: str | None = None,
        labels: Sequence[str] | None = None,
        selectors: Sequence[str] | None = None,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
        allow_gaps: bool = True,
    ) -> AvoInterfaceModel:
        return self.build_elastic_layer_model_from_top_set(
            asset_name=asset_name,
            labels=labels,
            selectors=selectors,
            sample_step_m=sample_step_m,
            min_samples_per_layer=min_samples_per_layer,
        ).to_avo_interface_model(allow_gaps=allow_gaps)

    def build_avo_interface_model_from_intervals(
        self,
        intervals: Sequence[WellInterval],
        *,
        sample_step_m: float | None = None,
        min_samples_per_layer: int = 3,
        allow_gaps: bool = True,
    ) -> AvoInterfaceModel:
        return self.build_elastic_layer_model_from_intervals(
            intervals,
            sample_step_m=sample_step_m,
            min_samples_per_layer=min_samples_per_layer,
        ).to_avo_interface_model(allow_gaps=allow_gaps)

    def build_avo_reflectivity_request(
        self,
        *,
        interface_depths_m: Sequence[float],
        angles_deg: Sequence[float],
        method: str = "zoeppritz",
        half_window_m: float = 0.5,
    ) -> AvoReflectivityRequest:
        if not interface_depths_m:
            raise ValueError("AVO interface sampling requires at least one interface depth")
        if not angles_deg:
            raise ValueError("AVO reflectivity requires at least one angle")
        if not math.isfinite(half_window_m) or half_window_m <= 0.0:
            raise ValueError("half_window_m must be a positive finite number")

        upper_vp: list[float] = []
        upper_vs: list[float] = []
        upper_density: list[float] = []
        lower_vp: list[float] = []
        lower_vs: list[float] = []
        lower_density: list[float] = []

        for depth_m in interface_depths_m:
            upper_depth = float(depth_m) - half_window_m
            lower_depth = float(depth_m) + half_window_m
            samples = {
                "upper_vp": self.vp.sample_at(upper_depth),
                "upper_vs": self.vs.sample_at(upper_depth),
                "upper_density": self.density.sample_at(upper_depth),
                "lower_vp": self.vp.sample_at(lower_depth),
                "lower_vs": self.vs.sample_at(lower_depth),
                "lower_density": self.density.sample_at(lower_depth),
            }
            missing = [name for name, value in samples.items() if value is None]
            if missing:
                joined = ", ".join(missing)
                raise ValueError(
                    f"could not sample elastic properties around interface depth {depth_m:.3f} m: {joined}"
                )
            upper_vp.append(float(samples["upper_vp"]))
            upper_vs.append(float(samples["upper_vs"]))
            upper_density.append(float(samples["upper_density"]))
            lower_vp.append(float(samples["lower_vp"]))
            lower_vs.append(float(samples["lower_vs"]))
            lower_density.append(float(samples["lower_density"]))

        return AvoReflectivityRequest(
            method=_normalize_avo_method(method),
            sample_shape=(len(interface_depths_m),),
            angles_deg=tuple(float(angle) for angle in angles_deg),
            upper_vp_m_per_s=tuple(upper_vp),
            upper_vs_m_per_s=tuple(upper_vs),
            upper_density_g_cc=tuple(upper_density),
            lower_vp_m_per_s=tuple(lower_vp),
            lower_vs_m_per_s=tuple(lower_vs),
            lower_density_g_cc=tuple(lower_density),
        )

    def run_avo_reflectivity(
        self,
        *,
        interface_depths_m: Sequence[float],
        angles_deg: Sequence[float],
        method: str = "zoeppritz",
        half_window_m: float = 0.5,
    ) -> AvoReflectivityResponse:
        request = self.build_avo_reflectivity_request(
            interface_depths_m=interface_depths_m,
            angles_deg=angles_deg,
            method=method,
            half_window_m=half_window_m,
        )
        return avo_reflectivity(request, app=self.wellbore.project.app)

    def materialize_missing_channels(
        self,
        *,
        output_collection_name: str | None = None,
        output_mnemonics: Mapping[str, str] | None = None,
    ) -> dict[str, ComputeRun]:
        overrides = dict(output_mnemonics or {})
        results: dict[str, ComputeRun] = {}
        for channel_name, channel in {
            "vp": self.vp,
            "vs": self.vs,
            "density": self.density,
        }.items():
            if channel.derivation is None:
                continue

            if channel.derivation == "sonic_to_vp":
                request = ComputeRequest(
                    source_asset_id=channel.source_curve.asset_id,
                    function_id="rock_physics:sonic_to_vp",
                    curve_bindings={"sonic_curve": channel.source_curve.curve_name},
                    parameters={},
                    output_collection_name=output_collection_name,
                    output_mnemonic=overrides.get(channel_name, "VP"),
                )
            elif channel.derivation == "shear_sonic_to_vs":
                request = ComputeRequest(
                    source_asset_id=channel.source_curve.asset_id,
                    function_id="rock_physics:shear_sonic_to_vs",
                    curve_bindings={"shear_sonic_curve": channel.source_curve.curve_name},
                    parameters={},
                    output_collection_name=output_collection_name,
                    output_mnemonic=overrides.get(channel_name, "VS"),
                )
            else:
                continue

            results[channel_name] = self.wellbore.project.run_compute(request)

        return results

    def to_avo_response_source(
        self,
        response: AvoReflectivityResponse,
        *,
        interface_depths_m: Sequence[float],
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        sample_count = _sample_count(response.sample_shape)
        if sample_count != len(interface_depths_m):
            raise ValueError("interface depth count does not match AVO response sample shape")

        labels = tuple(interface_labels or ())
        palette = tuple(colors or _DEFAULT_INTERFACE_COLORS)
        interfaces = []
        series = []
        reflectivity_model = _reflectivity_model_for_chart(response.method)
        resolved_name = name or f"{self.wellbore.name} AVO"

        for index, depth_m in enumerate(interface_depths_m):
            interface_id = f"interface-{index + 1}"
            label = labels[index] if index < len(labels) else f"Interface {index + 1}"
            color = palette[index % len(palette)]
            interfaces.append(
                {
                    "id": interface_id,
                    "label": label,
                    "color": color,
                    "reservoir_label": f"{depth_m:.1f} m",
                }
            )
            series.append(
                {
                    "id": f"{interface_id}-series",
                    "interface_id": interface_id,
                    "label": label,
                    "color": color,
                    "style": "solid",
                    "reflectivity_model": reflectivity_model,
                    "anisotropy_mode": "isotropic",
                    "incidence_angles_deg": [float(value) for value in response.angles_deg],
                    "values": list(_series_values_for_sample(response, index)),
                }
            )

        return {
            "schema_version": 1,
            "id": source_id or f"{self.wellbore.id}-avo-response",
            "name": resolved_name,
            "title": title or resolved_name,
            "subtitle": subtitle,
            "x_axis": {"label": "Incidence Angle", "unit": "deg"},
            "y_axis": {"label": "PP Reflectivity", "unit": "ratio"},
            "interfaces": interfaces,
            "series": series,
        }

    def to_avo_crossplot_source(
        self,
        response: AvoReflectivityResponse,
        *,
        interface_depths_m: Sequence[float],
        interface_labels: Sequence[str] | None = None,
        title: str | None = None,
        subtitle: str | None = None,
        colors: Sequence[str] | None = None,
        source_id: str | None = None,
        name: str | None = None,
    ) -> dict[str, Any]:
        sample_count = _sample_count(response.sample_shape)
        if sample_count != len(interface_depths_m):
            raise ValueError("interface depth count does not match AVO response sample shape")
        intercept = response.intercept()
        gradient = response.gradient()
        if len(intercept) != sample_count or len(gradient) != sample_count:
            raise ValueError(
                "AVO crossplot source requires a response with intercept and gradient payloads"
            )

        labels = tuple(interface_labels or ())
        palette = tuple(colors or _DEFAULT_INTERFACE_COLORS)
        interfaces = []
        points = []
        resolved_name = name or f"{self.wellbore.name} AVO Crossplot"

        for index, depth_m in enumerate(interface_depths_m):
            interface_id = f"interface-{index + 1}"
            label = labels[index] if index < len(labels) else f"Interface {index + 1}"
            color = palette[index % len(palette)]
            interfaces.append(
                {
                    "id": interface_id,
                    "label": label,
                    "color": color,
                    "reservoir_label": f"{depth_m:.1f} m",
                }
            )
            points.append(
                {
                    "interface_id": interface_id,
                    "intercept": float(intercept[index]),
                    "gradient": float(gradient[index]),
                }
            )

        return {
            "schema_version": 1,
            "id": source_id or f"{self.wellbore.id}-avo-crossplot",
            "name": resolved_name,
            "title": title or resolved_name,
            "subtitle": subtitle,
            "x_axis": {"label": "Intercept", "unit": "ratio"},
            "y_axis": {"label": "Gradient", "unit": "ratio"},
            "interfaces": interfaces,
            "points": points,
        }


def resolve_preferred_curve(
    curves: Sequence[WellLogCurve],
    selector: str,
) -> WellLogCurve | None:
    normalized_selector = normalize_log_type_selector(selector)
    candidates = [
        curve
        for curve in curves
        if curve.semantic_type == normalized_selector or curve.log_type == normalized_selector
    ]
    candidates.sort(
        key=lambda curve: (-curve.valid_sample_count, curve.asset_name, curve.curve_name)
    )
    return candidates[0] if candidates else None


def _available_curve_labels(curves: Sequence[WellLogCurve]) -> str:
    if not curves:
        return "none"
    return ", ".join(
        f"{curve.asset_name}:{curve.curve_name}<{curve.log_type}/{curve.semantic_type}>"
        for curve in sorted(curves, key=lambda item: (item.asset_name, item.curve_name))
    )


def _selector_label(selector: object) -> str:
    if isinstance(selector, str):
        return selector
    curve_name = getattr(selector, "curve_name", None)
    asset_name = getattr(selector, "asset_name", None)
    if isinstance(curve_name, str) and curve_name:
        if isinstance(asset_name, str) and asset_name:
            return f"{asset_name}:{curve_name}"
        return curve_name
    return repr(selector)


def _normalize_binding_selector(
    selector: str | object | None,
    *,
    role: str,
) -> str | object | None:
    if selector is None or not isinstance(selector, str):
        return selector

    normalized = selector.strip().lower().replace(" ", "").replace("_", "")
    if role == "vp":
        aliases = {
            "vp": "PVelocity",
            "pvelocity": "PVelocity",
            "pwavevelocity": "PVelocity",
            "pwave": "PVelocity",
            "compressionalslowness": "CompressionalSlowness",
            "compressionalsonic": "CompressionalSlowness",
            "dt": "CompressionalSlowness",
            "dtc": "CompressionalSlowness",
            "dtco": "CompressionalSlowness",
            "sonic": "CompressionalSlowness",
        }
    elif role == "vs":
        aliases = {
            "vs": "SVelocity",
            "svelocity": "SVelocity",
            "swavevelocity": "SVelocity",
            "swave": "SVelocity",
            "shearslowness": "ShearSlowness",
            "shearsonic": "ShearSlowness",
            "dts": "ShearSlowness",
            "dtsm": "ShearSlowness",
            "dtsh": "ShearSlowness",
        }
    elif role == "density":
        aliases = {
            "density": "BulkDensity",
            "rho": "BulkDensity",
            "rhob": "BulkDensity",
            "den": "BulkDensity",
            "bulkdensity": "BulkDensity",
        }
    else:
        aliases = {}
    try:
        return aliases[normalized]
    except KeyError as exc:
        supported = ", ".join(sorted(aliases.values()))
        raise ValueError(
            f"unsupported canonical binding '{selector}' for channel '{role}'. "
            f"Supported canonical bindings: {supported}"
        ) from exc


def _resolve_curve_from_selector(
    curves: Sequence[WellLogCurve],
    selector: object,
) -> WellLogCurve:
    curve_name = getattr(selector, "curve_name", None)
    asset_name = getattr(selector, "asset_name", None)
    if not isinstance(curve_name, str) or not curve_name:
        raise ValueError(
            "curve selectors must provide a non-empty 'curve_name' field"
        )

    candidates = [
        curve
        for curve in curves
        if curve.curve_name == curve_name or curve.original_mnemonic == curve_name
    ]
    if isinstance(asset_name, str) and asset_name:
        candidates = [curve for curve in candidates if curve.asset_name == asset_name]
    candidates.sort(
        key=lambda curve: (-curve.valid_sample_count, curve.asset_name, curve.curve_name)
    )
    if candidates:
        return candidates[0]

    raise LookupError(
        f"curve selector '{_selector_label(selector)}' did not match any log curve. "
        f"Available curves: {_available_curve_labels(curves)}"
    )


def _resolve_bound_elastic_channel(
    curves: Sequence[WellLogCurve],
    selector: str | object,
    *,
    role: str,
    wellbore_id: str,
) -> ResolvedElasticChannel:
    if role == "vp":
        compatible = {"PVelocity", "Sonic"}
        error_label = "P-wave velocity or sonic"
    elif role == "vs":
        compatible = {"SVelocity", "ShearSonic"}
        error_label = "S-wave velocity or shear sonic"
    elif role == "density":
        compatible = {"BulkDensity"}
        error_label = "bulk density"
    else:
        raise ValueError(f"unsupported elastic channel role '{role}'")

    normalized_selector = _normalize_binding_selector(selector, role=role)
    if isinstance(normalized_selector, str):
        curve = resolve_preferred_curve(curves, normalized_selector)
        if curve is None:
            available = _available_curve_labels(curves)
            raise LookupError(
                f"wellbore '{wellbore_id}' does not provide a '{normalized_selector}' curve "
                f"for channel '{role}'. Available curves: {available}"
            )
    else:
        curve = _resolve_curve_from_selector(curves, normalized_selector)
        if curve.semantic_type not in compatible:
            raise LookupError(
                f"curve selector '{_selector_label(normalized_selector)}' resolved to "
                f"semantic type '{curve.semantic_type}', but channel '{role}' requires {error_label}"
            )

    if curve.semantic_type == "PVelocity":
        return ResolvedElasticChannel(
            semantic_type="PVelocity",
            source_curve=curve,
            source_semantic_type=curve.semantic_type,
        )
    if curve.semantic_type == "Sonic":
        return ResolvedElasticChannel(
            semantic_type="PVelocity",
            source_curve=curve,
            source_semantic_type=curve.semantic_type,
            derivation="sonic_to_vp",
        )
    if curve.semantic_type == "SVelocity":
        return ResolvedElasticChannel(
            semantic_type="SVelocity",
            source_curve=curve,
            source_semantic_type=curve.semantic_type,
        )
    if curve.semantic_type == "ShearSonic":
        return ResolvedElasticChannel(
            semantic_type="SVelocity",
            source_curve=curve,
            source_semantic_type=curve.semantic_type,
            derivation="shear_sonic_to_vs",
        )
    if curve.semantic_type == "BulkDensity":
        return ResolvedElasticChannel(
            semantic_type="BulkDensity",
            source_curve=curve,
            source_semantic_type=curve.semantic_type,
        )

    raise LookupError(
        f"curve selector '{_selector_label(normalized_selector)}' resolved to unsupported "
        f"semantic type '{curve.semantic_type}' for channel '{role}'"
    )


def resolve_well_interval_sets(wellbore: Wellbore) -> tuple[WellTopSet, ...]:
    panel = wellbore.panel()
    return tuple(
        WellTopSet.from_panel_json(item)
        for item in _well_panel_top_sets(panel, wellbore.id)
    )


def resolve_well_top_sets(wellbore: Wellbore) -> tuple[WellTopSet, ...]:
    return tuple(
        top_set
        for top_set in resolve_well_interval_sets(wellbore)
        if top_set.set_kind == "top_set"
    )


def resolve_well_marker_sets(wellbore: Wellbore) -> tuple[WellTopSet, ...]:
    return tuple(
        top_set
        for top_set in resolve_well_interval_sets(wellbore)
        if top_set.set_kind == "well_marker_set"
    )


def resolve_elastic_log_set(
    wellbore: Wellbore,
    *,
    bindings: ElasticChannelBindings | None = None,
) -> ElasticLogSet:
    curves = wellbore.log_curves()

    if bindings is not None:
        density_selector = bindings.density if bindings.density is not None else "BulkDensity"
        vp_selector = bindings.vp
        vs_selector = bindings.vs

        density = _resolve_bound_elastic_channel(
            curves,
            density_selector,
            role="density",
            wellbore_id=wellbore.id,
        )
        if vp_selector is None:
            vp = _resolve_bound_elastic_channel(
                curves,
                "PVelocity"
                if resolve_preferred_curve(curves, "PVelocity") is not None
                else "CompressionalSlowness",
                role="vp",
                wellbore_id=wellbore.id,
            )
        else:
            vp = _resolve_bound_elastic_channel(
                curves,
                vp_selector,
                role="vp",
                wellbore_id=wellbore.id,
            )
        if vs_selector is None:
            vs = _resolve_bound_elastic_channel(
                curves,
                "SVelocity"
                if resolve_preferred_curve(curves, "SVelocity") is not None
                else "ShearSlowness",
                role="vs",
                wellbore_id=wellbore.id,
            )
        else:
            vs = _resolve_bound_elastic_channel(
                curves,
                vs_selector,
                role="vs",
                wellbore_id=wellbore.id,
            )
        return ElasticLogSet(wellbore=wellbore, vp=vp, vs=vs, density=density)

    density_curve = resolve_preferred_curve(curves, "BulkDensity")
    if density_curve is None:
        raise LookupError(
            f"wellbore '{wellbore.id}' does not provide a bulk density log for elastic workflows"
        )

    vp_curve = resolve_preferred_curve(curves, "PVelocity")
    if vp_curve is not None:
        vp = ResolvedElasticChannel(
            semantic_type="PVelocity",
            source_curve=vp_curve,
            source_semantic_type=vp_curve.semantic_type,
        )
    else:
        sonic_curve = resolve_preferred_curve(curves, "CompressionalSlowness")
        if sonic_curve is None:
            raise LookupError(
                f"wellbore '{wellbore.id}' does not provide P-wave velocity or compressional slowness logs"
            )
        vp = ResolvedElasticChannel(
            semantic_type="PVelocity",
            source_curve=sonic_curve,
            source_semantic_type=sonic_curve.semantic_type,
            derivation="sonic_to_vp",
        )

    vs_curve = resolve_preferred_curve(curves, "SVelocity")
    if vs_curve is not None:
        vs = ResolvedElasticChannel(
            semantic_type="SVelocity",
            source_curve=vs_curve,
            source_semantic_type=vs_curve.semantic_type,
        )
    else:
        shear_sonic_curve = resolve_preferred_curve(curves, "ShearSlowness")
        if shear_sonic_curve is None:
            raise LookupError(
                f"wellbore '{wellbore.id}' does not provide S-wave velocity or shear slowness logs"
            )
        vs = ResolvedElasticChannel(
            semantic_type="SVelocity",
            source_curve=shear_sonic_curve,
            source_semantic_type=shear_sonic_curve.semantic_type,
            derivation="shear_sonic_to_vs",
        )

    return ElasticLogSet(
        wellbore=wellbore,
        vp=vp,
        vs=vs,
        density=ResolvedElasticChannel(
            semantic_type="BulkDensity",
            source_curve=density_curve,
            source_semantic_type=density_curve.semantic_type,
        ),
    )


__all__ = [
    "AvoInterface",
    "AvoInterfaceModel",
    "ElasticLayer",
    "ElasticLayerModel",
    "ElasticLogSet",
    "LogSet",
    "LogSetChannel",
    "ResolvedElasticChannel",
    "WellInterval",
    "WellLogCurve",
    "WellTopSet",
    "normalize_interval_set_kind",
    "resolve_elastic_log_set",
    "resolve_well_interval_sets",
    "resolve_well_marker_sets",
    "resolve_well_top_sets",
]
