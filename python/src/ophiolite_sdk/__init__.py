from __future__ import annotations

from importlib import import_module
import warnings

from .analysis import (
    avo_intercept_gradient_attribute,
    avo_reflectivity,
    rock_physics_attribute,
)
from .models import WellboreBinding
from .project import Project
from .seismic import (
    BandpassFilter,
    ConstantVelocity,
    GatherPipeline,
    GatherSelection,
    NmoCorrection,
    OffsetMute,
    PostStackNeighborhoodPipeline,
    PostStackNeighborhoodWindow,
    RmsAgc,
    SectionSelection,
    SeismicDataset,
    StretchMute,
    SubvolumePipeline,
    TraceBoostApp,
    TraceLocalPipeline,
    TraceProcessingPipeline,
    VelocityScanSpec,
)
from .surveys import Survey
from .wells import Well, Wellbore

__all__ = [
    "BandpassFilter",
    "ConstantVelocity",
    "GatherPipeline",
    "GatherSelection",
    "NmoCorrection",
    "OffsetMute",
    "PostStackNeighborhoodPipeline",
    "PostStackNeighborhoodWindow",
    "Project",
    "RmsAgc",
    "SectionSelection",
    "SeismicDataset",
    "StretchMute",
    "SubvolumePipeline",
    "Survey",
    "TraceBoostApp",
    "TraceLocalPipeline",
    "TraceProcessingPipeline",
    "VelocityScanSpec",
    "Well",
    "Wellbore",
    "WellboreBinding",
    "avo_reflectivity",
    "avo_intercept_gradient_attribute",
    "rock_physics_attribute",
]

_DEPRECATED_EXPORTS = {
    "AvoReflectivityRequest": ("ophiolite_sdk.analysis", "analysis"),
    "AvoReflectivityResponse": ("ophiolite_sdk.analysis", "analysis"),
    "AvoInterceptGradientAttributeRequest": ("ophiolite_sdk.analysis", "analysis"),
    "AvoInterceptGradientAttributeResponse": ("ophiolite_sdk.analysis", "analysis"),
    "RockPhysicsAttributeRequest": ("ophiolite_sdk.analysis", "analysis"),
    "RockPhysicsAttributeResponse": ("ophiolite_sdk.analysis", "analysis"),
    "ComputeCatalog": ("ophiolite_sdk.interop", "interop"),
    "OperatorCatalog": ("ophiolite_sdk.interop", "interop"),
    "OperatorCatalogEntry": ("ophiolite_sdk.interop", "interop"),
    "OperatorContractRef": ("ophiolite_sdk.interop", "interop"),
    "OperatorDocumentation": ("ophiolite_sdk.interop", "interop"),
    "OperatorParameterDoc": ("ophiolite_sdk.interop", "interop"),
    "ComputeRequest": ("ophiolite_sdk.interop", "interop"),
    "ComputeRun": ("ophiolite_sdk.interop", "interop"),
    "OperatorLock": ("ophiolite_sdk.interop", "interop"),
    "OperatorPackageInstallResult": ("ophiolite_sdk.interop", "interop"),
    "PlatformCatalog": ("ophiolite_sdk.interop", "interop"),
    "PlatformOperation": ("ophiolite_sdk.interop", "interop"),
    "ProjectSummary": ("ophiolite_sdk.interop", "interop"),
    "SurveySummary": ("ophiolite_sdk.interop", "interop"),
    "OperatorRegistry": ("ophiolite_sdk.operators", "operators"),
    "OperatorRequest": ("ophiolite_sdk.operators", "operators"),
    "computed_curve": ("ophiolite_sdk.operators", "operators"),
}


def __getattr__(name: str):
    if name not in _DEPRECATED_EXPORTS:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

    module_name, _namespace = _DEPRECATED_EXPORTS[name]
    warnings.warn(
        (
            f"'{name}' is deprecated in 'ophiolite_sdk' and will move after the "
            f"current preview cycle. Import it from '{module_name}' instead."
        ),
        DeprecationWarning,
        stacklevel=2,
    )
    module = import_module(module_name)
    return getattr(module, name)


def __dir__() -> list[str]:
    return sorted(set(globals()) | set(__all__) | set(_DEPRECATED_EXPORTS))
