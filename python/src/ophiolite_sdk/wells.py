from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

from .logs import (
    ElasticLogSet,
    WellLogCurve,
    WellTopSet,
    resolve_well_interval_sets,
    resolve_well_marker_sets,
    normalize_log_type_selector,
    resolve_elastic_log_set,
    resolve_preferred_curve,
    resolve_well_top_sets,
)
from .models import WellIdentifierSet, WellSummary, WellboreBinding, WellboreSummary

if TYPE_CHECKING:
    from .avo import ElasticChannelBindings
    from .project import Project
    from .surveys import Survey
    from .wells import Well


@dataclass(frozen=True)
class Well:
    project: Project
    summary_data: WellSummary

    @property
    def id(self) -> str:
        return self.summary_data.well.id

    @property
    def name(self) -> str:
        return self.summary_data.well.name

    @property
    def identifiers(self) -> WellIdentifierSet:
        return self.summary_data.well.identifiers

    @property
    def wellbore_count(self) -> int:
        return self.summary_data.wellbore_count

    @property
    def asset_count(self) -> int:
        return self.summary_data.asset_count

    def summary(self) -> WellSummary:
        return self.summary_data

    def wellbores(self) -> list[Wellbore]:
        return self.project.wellbores(self)

    def surveys(self) -> list[Survey]:
        return [survey for survey in self.project.surveys() if survey.well_id == self.id]

    def panel(
        self,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
    ) -> dict[str, Any]:
        return self.project.views.well_panel(
            self.wellbores(),
            depth_min=depth_min,
            depth_max=depth_max,
        )


@dataclass(frozen=True)
class Wellbore:
    project: Project
    summary_data: WellboreSummary

    @property
    def id(self) -> str:
        return self.summary_data.wellbore.id

    @property
    def well_id(self) -> str:
        return self.summary_data.wellbore.well_id

    @property
    def name(self) -> str:
        return self.summary_data.wellbore.name

    @property
    def identifiers(self) -> WellIdentifierSet:
        return self.summary_data.wellbore.identifiers

    @property
    def active_well_time_depth_model_asset_id(self) -> str | None:
        return self.summary_data.wellbore.active_well_time_depth_model_asset_id

    @property
    def collection_count(self) -> int:
        return self.summary_data.collection_count

    @property
    def asset_count(self) -> int:
        return self.summary_data.asset_count

    def summary(self) -> WellboreSummary:
        return self.summary_data

    def well(self) -> Well:
        for well in self.project.wells():
            if well.id == self.well_id:
                return well
        raise LookupError(f"well '{self.well_id}' was not found in project '{self.project.root}'")

    def surveys(self) -> list[Survey]:
        return [survey for survey in self.project.surveys() if survey.wellbore_id == self.id]

    def trajectory(self) -> dict[str, object]:
        return self.project.app.resolve_wellbore_trajectory(str(self.project.root), self.id)

    def binding(self) -> WellboreBinding:
        well = self.well()
        return WellboreBinding(
            well_name=well.name,
            wellbore_name=self.name,
            uwi=well.identifiers.uwi or self.identifiers.uwi,
            api=well.identifiers.api or self.identifiers.api,
            operator_aliases=well.identifiers.operator_aliases or self.identifiers.operator_aliases,
        )

    def log_curves(
        self,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
    ) -> list[WellLogCurve]:
        panel = self.panel(depth_min=depth_min, depth_max=depth_max)
        wells = panel.get("wells", [])
        for well in wells:
            if isinstance(well, dict) and well.get("wellbore_id") == self.id:
                logs = well.get("logs", [])
                return [
                    WellLogCurve.from_panel_json(log)
                    for log in logs
                    if isinstance(log, dict)
                ]
        return []

    def available_log_types(
        self,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
        include_index: bool = False,
        include_unknown: bool = False,
    ) -> list[str]:
        ignored = set()
        if not include_index:
            ignored.add("Depth")
        if not include_unknown:
            ignored.add("Unknown")
        return sorted(
            {
                curve.log_type
                for curve in self.log_curves(depth_min=depth_min, depth_max=depth_max)
                if curve.log_type not in ignored
            }
        )

    def log_curves_by_type(
        self,
        log_type: str,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
    ) -> list[WellLogCurve]:
        normalized = normalize_log_type_selector(log_type)
        curves = [
            curve
            for curve in self.log_curves(depth_min=depth_min, depth_max=depth_max)
            if curve.log_type == normalized or curve.semantic_type == normalized
        ]
        curves.sort(
            key=lambda curve: (-curve.valid_sample_count, curve.asset_name, curve.curve_name)
        )
        return curves

    def preferred_log_curve(
        self,
        log_type: str,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
    ) -> WellLogCurve | None:
        return resolve_preferred_curve(
            self.log_curves(depth_min=depth_min, depth_max=depth_max),
            log_type,
        )

    def top_sets(
        self,
    ) -> list[WellTopSet]:
        return list(resolve_well_top_sets(self))

    def interval_sets(
        self,
    ) -> list[WellTopSet]:
        return list(resolve_well_interval_sets(self))

    def marker_sets(
        self,
    ) -> list[WellTopSet]:
        return list(resolve_well_marker_sets(self))

    def available_top_sets(self) -> list[str]:
        return sorted({top_set.asset_name for top_set in self.top_sets()})

    def available_marker_sets(self) -> list[str]:
        return sorted({top_set.asset_name for top_set in self.marker_sets()})

    def top_set(self, asset_name: str | None = None) -> WellTopSet | None:
        top_sets = self.top_sets()
        if asset_name is None:
            return top_sets[0] if top_sets else None
        normalized = asset_name.strip().casefold()
        for top_set in top_sets:
            if top_set.asset_name.strip().casefold() == normalized:
                return top_set
        return None

    def marker_set(self, asset_name: str | None = None) -> WellTopSet | None:
        marker_sets = self.marker_sets()
        if asset_name is None:
            return marker_sets[0] if marker_sets else None
        normalized = asset_name.strip().casefold()
        for marker_set in marker_sets:
            if marker_set.asset_name.strip().casefold() == normalized:
                return marker_set
        return None

    def elastic_log_set(
        self,
        *,
        bindings: ElasticChannelBindings | None = None,
    ) -> ElasticLogSet:
        return resolve_elastic_log_set(self, bindings=bindings)

    def panel(
        self,
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
    ) -> dict[str, Any]:
        return self.project.views.well_panel(
            [self],
            depth_min=depth_min,
            depth_max=depth_max,
        )
