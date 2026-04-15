from __future__ import annotations

from ophiolite_automation.client import OphioliteApp

from .models import (
    AvoInterceptGradientAttributeRequest,
    AvoInterceptGradientAttributeResponse,
    AvoReflectivityRequest,
    AvoReflectivityResponse,
    RockPhysicsAttributeRequest,
    RockPhysicsAttributeResponse,
)


def avo_reflectivity(
    request: AvoReflectivityRequest, *, app: OphioliteApp | None = None
) -> AvoReflectivityResponse:
    resolved_app = app or OphioliteApp()
    payload = resolved_app.run_avo_reflectivity(request.to_payload())
    return AvoReflectivityResponse.from_json(payload)


def rock_physics_attribute(
    request: RockPhysicsAttributeRequest, *, app: OphioliteApp | None = None
) -> RockPhysicsAttributeResponse:
    resolved_app = app or OphioliteApp()
    payload = resolved_app.run_rock_physics_attribute(request.to_payload())
    return RockPhysicsAttributeResponse.from_json(payload)


def avo_intercept_gradient_attribute(
    request: AvoInterceptGradientAttributeRequest, *, app: OphioliteApp | None = None
) -> AvoInterceptGradientAttributeResponse:
    resolved_app = app or OphioliteApp()
    payload = resolved_app.run_avo_intercept_gradient_attribute(request.to_payload())
    return AvoInterceptGradientAttributeResponse.from_json(payload)
