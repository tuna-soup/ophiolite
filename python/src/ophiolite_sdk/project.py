from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import warnings

from ophiolite_automation.client import OphioliteApp

from .models import (
    ComputeCatalog,
    ComputeRequest,
    ComputeRun,
    LogAssetImportResult,
    OperatorLock,
    OperatorPackageInstallResult,
    PlatformCatalog,
    ProjectSummary,
    SurveySummary,
    TopsSourceImportResult,
    WellSummary,
    WellboreBinding,
    WellboreSummary,
)
from .platform import catalog as load_platform_catalog
from .surveys import Survey
from .views import ProjectViews
from .wells import Well, Wellbore


@dataclass(frozen=True)
class Project:
    root: Path
    app: OphioliteApp

    @classmethod
    def create(cls, project_root: str | Path, *, app: OphioliteApp | None = None) -> Project:
        resolved_app = app or OphioliteApp()
        root = Path(project_root)
        resolved_app.create_project(str(root))
        return cls(root=root, app=resolved_app)

    @classmethod
    def open(cls, project_root: str | Path, *, app: OphioliteApp | None = None) -> Project:
        resolved_app = app or OphioliteApp()
        root = Path(project_root)
        resolved_app.open_project(str(root))
        return cls(root=root, app=resolved_app)

    @staticmethod
    def platform_catalog() -> PlatformCatalog:
        warnings.warn(
            (
                "'Project.platform_catalog()' is deprecated and will move after "
                "the current preview cycle. Use "
                "'ophiolite_sdk.platform.catalog()' instead."
            ),
            DeprecationWarning,
            stacklevel=2,
        )
        return load_platform_catalog()

    def summary(self) -> ProjectSummary:
        payload = self.app.project_summary(str(self.root))
        return ProjectSummary.from_json(payload)

    def import_las(
        self,
        las_path: str | Path,
        *,
        binding: WellboreBinding | None = None,
        collection_name: str | None = None,
    ) -> LogAssetImportResult:
        if binding is None:
            payload = self.app.import_project_las(
                str(self.root),
                str(las_path),
                collection_name,
            )
        else:
            payload = self.app.import_project_las_with_binding(
                str(self.root),
                str(las_path),
                binding.to_payload(),
                collection_name,
            )
        return LogAssetImportResult.from_json(payload)

    def import_tops_source(
        self,
        source_path: str | Path,
        *,
        binding: WellboreBinding,
        collection_name: str | None = None,
        depth_reference: str | None = None,
    ) -> TopsSourceImportResult:
        payload = self.app.import_project_tops_source_with_binding(
            str(self.root),
            str(source_path),
            binding.to_payload(),
            collection_name,
            depth_reference,
        )
        return TopsSourceImportResult.from_json(payload)

    def well_summaries(self) -> list[WellSummary]:
        payload = self.app.list_project_wells(str(self.root))
        return [WellSummary.from_json(item) for item in payload]

    def wells(self) -> list[Well]:
        return [Well(project=self, summary_data=summary) for summary in self.well_summaries()]

    def wellbore_summaries(self, well_id: str | Well) -> list[WellboreSummary]:
        normalized_well_id = well_id.id if isinstance(well_id, Well) else well_id
        payload = self.app.list_project_wellbores(str(self.root), normalized_well_id)
        return [WellboreSummary.from_json(item) for item in payload]

    def wellbores(self, well_id: str | Well) -> list[Wellbore]:
        return [
            Wellbore(project=self, summary_data=summary)
            for summary in self.wellbore_summaries(well_id)
        ]

    def survey_summaries(self) -> list[SurveySummary]:
        payload = self.app.list_project_surveys(str(self.root))
        return [SurveySummary.from_json(item) for item in payload]

    def surveys(self) -> list[Survey]:
        return [Survey(project=self, summary_data=summary) for summary in self.survey_summaries()]

    @property
    def views(self) -> ProjectViews:
        return ProjectViews(self)

    def operator_lock(self) -> OperatorLock:
        payload = self.app.project_operator_lock(str(self.root))
        return OperatorLock.from_json(payload)

    def install_operator_package(
        self, manifest_path: str | Path
    ) -> OperatorPackageInstallResult:
        payload = self.app.install_operator_package(str(self.root), str(manifest_path))
        return OperatorPackageInstallResult.from_json(payload)

    def compute_catalog(self, asset_id: str) -> ComputeCatalog:
        payload = self.app.list_project_compute_catalog(str(self.root), asset_id)
        return ComputeCatalog.from_json(payload)

    def run_compute(self, request: ComputeRequest) -> ComputeRun:
        payload = self.app.run_project_compute(str(self.root), request.to_payload())
        return ComputeRun.from_json(payload)

    def run_elastic_impedance(
        self,
        *,
        source_asset_id: str,
        vp_curve: str,
        vs_curve: str,
        density_curve: str,
        angle_deg: float,
        output_collection_name: str | None = None,
        output_mnemonic: str | None = None,
    ) -> ComputeRun:
        return self.run_compute(
            ComputeRequest(
                source_asset_id=source_asset_id,
                function_id="rock_physics:elastic_impedance",
                curve_bindings={
                    "vp_curve": vp_curve,
                    "vs_curve": vs_curve,
                    "density_curve": density_curve,
                },
                parameters={"angle_deg": angle_deg},
                output_collection_name=output_collection_name,
                output_mnemonic=output_mnemonic,
            )
        )

    def run_extended_elastic_impedance(
        self,
        *,
        source_asset_id: str,
        vp_curve: str,
        vs_curve: str,
        density_curve: str,
        chi_angle_deg: float,
        output_collection_name: str | None = None,
        output_mnemonic: str | None = None,
    ) -> ComputeRun:
        return self.run_compute(
            ComputeRequest(
                source_asset_id=source_asset_id,
                function_id="rock_physics:extended_elastic_impedance",
                curve_bindings={
                    "vp_curve": vp_curve,
                    "vs_curve": vs_curve,
                    "density_curve": density_curve,
                },
                parameters={"chi_deg": chi_angle_deg},
                output_collection_name=output_collection_name,
                output_mnemonic=output_mnemonic,
            )
        )
