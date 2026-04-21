from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable, TYPE_CHECKING

from .models import SectionWellOverlayRequest, SurveyMapRequest, WellPanelRequest
from .surveys import Survey
from .wells import Well, Wellbore

if TYPE_CHECKING:
    from .project import Project


def _normalize_wellbore_ids(items: Iterable[str | Well | Wellbore]) -> tuple[str, ...]:
    normalized: list[str] = []
    for item in items:
        if isinstance(item, Well):
            normalized.extend(wellbore.id for wellbore in item.wellbores())
        elif isinstance(item, Wellbore):
            normalized.append(item.id)
        else:
            normalized.append(item)
    return tuple(normalized)


def _normalize_survey_ids(items: Iterable[str | Survey]) -> tuple[str, ...]:
    normalized: list[str] = []
    for item in items:
        if isinstance(item, Survey):
            normalized.append(item.asset_id)
        else:
            normalized.append(item)
    return tuple(normalized)


@dataclass(frozen=True)
class ProjectViews:
    project: Project

    def well_panel(
        self,
        wellbores: Iterable[str | Well | Wellbore],
        *,
        depth_min: float | None = None,
        depth_max: float | None = None,
    ) -> dict[str, Any]:
        request = WellPanelRequest(
            wellbore_ids=_normalize_wellbore_ids(wellbores),
            depth_min=depth_min,
            depth_max=depth_max,
        )
        return self.project.app.resolve_well_panel_source(
            str(self.project.root), request.to_payload()
        )

    def survey_map(
        self,
        *,
        surveys: Iterable[str | Survey] = (),
        wellbores: Iterable[str | Well | Wellbore] = (),
        display_coordinate_reference_id: str,
    ) -> dict[str, Any]:
        request = SurveyMapRequest(
            survey_asset_ids=_normalize_survey_ids(surveys),
            wellbore_ids=_normalize_wellbore_ids(wellbores),
            display_coordinate_reference_id=display_coordinate_reference_id,
        )
        return self.project.app.resolve_survey_map_source(
            str(self.project.root), request.to_payload()
        )

    def section_well_overlays(
        self,
        *,
        survey: str | Survey,
        wellbores: Iterable[str | Well | Wellbore],
        axis: str,
        index: int,
        display_domain: str,
        tolerance_m: float | None = None,
    ) -> dict[str, Any]:
        survey_asset_id = survey.asset_id if isinstance(survey, Survey) else survey
        request = SectionWellOverlayRequest(
            survey_asset_id=survey_asset_id,
            wellbore_ids=_normalize_wellbore_ids(wellbores),
            axis=axis,
            index=index,
            display_domain=display_domain,
            tolerance_m=tolerance_m,
        )
        return self.project.app.resolve_section_well_overlays(
            str(self.project.root), request.to_payload()
        )
