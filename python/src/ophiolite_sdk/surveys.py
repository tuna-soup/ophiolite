from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Iterable

from .models import SurveySummary
from .seismic import (
    GatherPipeline,
    GatherPreview,
    GatherSelection,
    PostStackNeighborhoodPipeline,
    ProcessingPreview,
    SectionSelection,
    SubvolumePipeline,
    TraceLocalPipeline,
    VelocityScanResult,
    VelocityScanSpec,
)

if TYPE_CHECKING:
    from .project import Project
    from .wells import Well, Wellbore


@dataclass(frozen=True)
class Survey:
    """Canonical project-owned seismic asset surface addressed by asset id."""

    project: Project
    summary_data: SurveySummary

    @property
    def id(self) -> str:
        return self.summary_data.asset_id

    @property
    def asset_id(self) -> str:
        return self.summary_data.asset_id

    @property
    def logical_asset_id(self) -> str:
        return self.summary_data.logical_asset_id

    @property
    def collection_id(self) -> str:
        return self.summary_data.collection_id

    @property
    def name(self) -> str:
        return self.summary_data.name

    @property
    def status(self) -> str:
        return self.summary_data.status

    @property
    def owner_scope(self) -> str:
        return self.summary_data.owner_scope

    @property
    def owner_id(self) -> str:
        return self.summary_data.owner_id

    @property
    def owner_name(self) -> str:
        return self.summary_data.owner_name

    @property
    def well_id(self) -> str:
        return self.summary_data.well_id

    @property
    def well_name(self) -> str:
        return self.summary_data.well_name

    @property
    def wellbore_id(self) -> str:
        return self.summary_data.wellbore_id

    @property
    def wellbore_name(self) -> str:
        return self.summary_data.wellbore_name

    @property
    def effective_coordinate_reference_id(self) -> str | None:
        return self.summary_data.effective_coordinate_reference_id

    @property
    def effective_coordinate_reference_name(self) -> str | None:
        return self.summary_data.effective_coordinate_reference_name

    def summary(self) -> SurveySummary:
        return self.summary_data

    def well(self) -> Well:
        for well in self.project.wells():
            if well.id == self.well_id:
                return well
        raise LookupError(f"well '{self.well_id}' was not found in project '{self.project.root}'")

    def wellbore(self) -> Wellbore:
        for wellbore in self.project.wellbores(self.well_id):
            if wellbore.id == self.wellbore_id:
                return wellbore
        raise LookupError(
            f"wellbore '{self.wellbore_id}' was not found in project '{self.project.root}'"
        )

    def map_view(
        self,
        *,
        wellbores: Iterable[str | Well | Wellbore] = (),
        display_coordinate_reference_id: str,
    ) -> dict[str, Any]:
        return self.project.views.survey_map(
            surveys=[self],
            wellbores=wellbores,
            display_coordinate_reference_id=display_coordinate_reference_id,
        )

    def section_well_overlays(
        self,
        *,
        wellbores: Iterable[str | Well | Wellbore],
        axis: str,
        index: int,
        display_domain: str,
        tolerance_m: float | None = None,
    ) -> dict[str, Any]:
        return self.project.views.section_well_overlays(
            survey=self,
            wellbores=wellbores,
            axis=axis,
            index=index,
            display_domain=display_domain,
            tolerance_m=tolerance_m,
        )

    def operator_catalog(self):
        return self.project.operator_catalog(self.asset_id)

    def preview_processing(
        self,
        selection: SectionSelection,
        pipeline: TraceLocalPipeline,
    ) -> ProcessingPreview:
        return self.project.preview_trace_local_processing(
            source_asset_id=self.asset_id,
            selection=selection,
            pipeline=pipeline,
        )

    def run_processing(
        self,
        pipeline: TraceLocalPipeline,
        *,
        output_collection_name: str | None = None,
    ):
        return self.project.run_trace_local_processing(
            source_asset_id=self.asset_id,
            pipeline=pipeline,
            output_collection_name=output_collection_name,
        )

    def preview_subvolume(
        self,
        selection: SectionSelection,
        pipeline: SubvolumePipeline,
    ) -> ProcessingPreview:
        return self.project.preview_subvolume_processing(
            source_asset_id=self.asset_id,
            selection=selection,
            pipeline=pipeline,
        )

    def run_subvolume(
        self,
        pipeline: SubvolumePipeline,
        *,
        output_collection_name: str | None = None,
    ):
        return self.project.run_subvolume_processing(
            source_asset_id=self.asset_id,
            pipeline=pipeline,
            output_collection_name=output_collection_name,
        )

    def preview_post_stack_neighborhood(
        self,
        selection: SectionSelection,
        pipeline: PostStackNeighborhoodPipeline,
    ) -> ProcessingPreview:
        return self.project.preview_post_stack_neighborhood_processing(
            source_asset_id=self.asset_id,
            selection=selection,
            pipeline=pipeline,
        )

    def run_post_stack_neighborhood(
        self,
        pipeline: PostStackNeighborhoodPipeline,
        *,
        output_collection_name: str | None = None,
    ):
        return self.project.run_post_stack_neighborhood_processing(
            source_asset_id=self.asset_id,
            pipeline=pipeline,
            output_collection_name=output_collection_name,
        )

    def preview_gather(
        self,
        gather: GatherSelection,
        pipeline: GatherPipeline,
    ) -> GatherPreview:
        return self.project.preview_gather_processing(
            source_asset_id=self.asset_id,
            gather=gather,
            pipeline=pipeline,
        )

    def run_gather(
        self,
        pipeline: GatherPipeline,
        *,
        output_collection_name: str | None = None,
    ):
        return self.project.run_gather_processing(
            source_asset_id=self.asset_id,
            pipeline=pipeline,
            output_collection_name=output_collection_name,
        )

    def velocity_scan(
        self,
        gather: GatherSelection,
        spec: VelocityScanSpec,
    ) -> VelocityScanResult:
        return self.project.run_velocity_scan(
            source_asset_id=self.asset_id,
            gather=gather,
            spec=spec,
        )
