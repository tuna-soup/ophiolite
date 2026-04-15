from __future__ import annotations

from dataclasses import dataclass
import struct
from pathlib import Path
from typing import Any, Mapping


def _string(value: Any, field_name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"expected '{field_name}' to be a string")
    return value


def _optional_string(value: Any, field_name: str) -> str | None:
    if value is None:
        return None
    return _string(value, field_name)


def _string_list(value: Any, field_name: str) -> tuple[str, ...]:
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise ValueError(f"expected '{field_name}' to be a list of strings")
    return tuple(value)


def _mapping(value: Any, field_name: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"expected '{field_name}' to be an object")
    return value


def _mapping_list(value: Any, field_name: str) -> tuple[Mapping[str, Any], ...]:
    if not isinstance(value, list):
        raise ValueError(f"expected '{field_name}' to be a list")
    return tuple(_mapping(item, field_name) for item in value)


@dataclass(frozen=True)
class WellIdentifierSet:
    primary_name: str | None
    uwi: str | None
    api: str | None
    operator_aliases: tuple[str, ...]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> WellIdentifierSet:
        return cls(
            primary_name=_optional_string(data.get("primary_name"), "primary_name"),
            uwi=_optional_string(data.get("uwi"), "uwi"),
            api=_optional_string(data.get("api"), "api"),
            operator_aliases=_string_list(data.get("operator_aliases", []), "operator_aliases"),
        )


@dataclass(frozen=True)
class WellRecord:
    id: str
    name: str
    identifiers: WellIdentifierSet

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> WellRecord:
        return cls(
            id=_string(data["id"], "id"),
            name=_string(data["name"], "name"),
            identifiers=WellIdentifierSet.from_json(
                _mapping(data["identifiers"], "identifiers")
            ),
        )


@dataclass(frozen=True)
class WellSummary:
    well: WellRecord
    wellbore_count: int
    asset_count: int

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> WellSummary:
        return cls(
            well=WellRecord.from_json(_mapping(data["well"], "well")),
            wellbore_count=int(data["wellbore_count"]),
            asset_count=int(data["asset_count"]),
        )


@dataclass(frozen=True)
class WellboreRecord:
    id: str
    well_id: str
    name: str
    identifiers: WellIdentifierSet
    active_well_time_depth_model_asset_id: str | None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> WellboreRecord:
        return cls(
            id=_string(data["id"], "id"),
            well_id=_string(data["well_id"], "well_id"),
            name=_string(data["name"], "name"),
            identifiers=WellIdentifierSet.from_json(
                _mapping(data["identifiers"], "identifiers")
            ),
            active_well_time_depth_model_asset_id=_optional_string(
                data.get("active_well_time_depth_model_asset_id"),
                "active_well_time_depth_model_asset_id",
            ),
        )


@dataclass(frozen=True)
class WellboreSummary:
    wellbore: WellboreRecord
    collection_count: int
    asset_count: int

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> WellboreSummary:
        return cls(
            wellbore=WellboreRecord.from_json(_mapping(data["wellbore"], "wellbore")),
            collection_count=int(data["collection_count"]),
            asset_count=int(data["asset_count"]),
        )


@dataclass(frozen=True)
class ProjectSummary:
    root: Path
    catalog_path: Path
    manifest_path: Path
    well_count: int
    wellbore_count: int
    asset_collection_count: int
    asset_count: int

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> ProjectSummary:
        return cls(
            root=Path(_string(data["root"], "root")),
            catalog_path=Path(_string(data["catalog_path"], "catalog_path")),
            manifest_path=Path(_string(data["manifest_path"], "manifest_path")),
            well_count=int(data["well_count"]),
            wellbore_count=int(data["wellbore_count"]),
            asset_collection_count=int(data["asset_collection_count"]),
            asset_count=int(data["asset_count"]),
        )


@dataclass(frozen=True)
class PlatformOperation:
    id: str
    summary: str
    owner: str
    domain: str
    stability: str
    surfaces: tuple[str, ...]
    bindings: Mapping[str, Any]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> PlatformOperation:
        return cls(
            id=_string(data["id"], "id"),
            summary=_string(data["summary"], "summary"),
            owner=_string(data["owner"], "owner"),
            domain=_string(data["domain"], "domain"),
            stability=_string(data["stability"], "stability"),
            surfaces=_string_list(data.get("surfaces", []), "surfaces"),
            bindings=_mapping(data.get("bindings", {}), "bindings"),
        )


@dataclass(frozen=True)
class PlatformCatalog:
    schema_version: int
    catalog_name: str
    product: str
    operations: tuple[PlatformOperation, ...]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> PlatformCatalog:
        return cls(
            schema_version=int(data["schema_version"]),
            catalog_name=_string(data["catalog_name"], "catalog_name"),
            product=_string(data["product"], "product"),
            operations=tuple(
                PlatformOperation.from_json(item)
                for item in _mapping_list(data.get("operations", []), "operations")
            ),
        )


@dataclass(frozen=True)
class OperatorLockEntry:
    package_name: str
    package_version: str
    provider: str
    runtime: str
    source_kind: str
    source_reference: str | None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorLockEntry:
        return cls(
            package_name=_string(data["package_name"], "package_name"),
            package_version=_string(data["package_version"], "package_version"),
            provider=_string(data["provider"], "provider"),
            runtime=_string(data["runtime"], "runtime"),
            source_kind=_string(data["source_kind"], "source_kind"),
            source_reference=_optional_string(data.get("source_reference"), "source_reference"),
        )


@dataclass(frozen=True)
class OperatorLock:
    schema_version: int
    packages: tuple[OperatorLockEntry, ...]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorLock:
        return cls(
            schema_version=int(data["schema_version"]),
            packages=tuple(
                OperatorLockEntry.from_json(item)
                for item in _mapping_list(data.get("packages", []), "packages")
            ),
        )


@dataclass(frozen=True)
class OperatorPackageInstallResult:
    package_name: str
    package_version: str
    installed_manifest_path: Path
    python_environment_path: Path | None
    operator_count: int
    operator_lock: OperatorLock

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorPackageInstallResult:
        return cls(
            package_name=_string(data["package_name"], "package_name"),
            package_version=_string(data["package_version"], "package_version"),
            installed_manifest_path=Path(
                _string(data["installed_manifest_path"], "installed_manifest_path")
            ),
            python_environment_path=(
                Path(_string(data["python_environment_path"], "python_environment_path"))
                if data.get("python_environment_path") is not None
                else None
            ),
            operator_count=int(data["operator_count"]),
            operator_lock=OperatorLock.from_json(_mapping(data["operator_lock"], "operator_lock")),
        )


@dataclass(frozen=True)
class ComputeCatalog:
    asset_family: str
    functions: tuple[Mapping[str, Any], ...]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> ComputeCatalog:
        return cls(
            asset_family=_string(data["asset_family"], "asset_family"),
            functions=_mapping_list(data.get("functions", []), "functions"),
        )


@dataclass(frozen=True)
class ComputeRequest:
    source_asset_id: str
    function_id: str
    curve_bindings: Mapping[str, str]
    parameters: Mapping[str, Any]
    output_collection_name: str | None = None
    output_mnemonic: str | None = None

    def to_payload(self) -> dict[str, Any]:
        return {
            "source_asset_id": self.source_asset_id,
            "function_id": self.function_id,
            "curve_bindings": dict(self.curve_bindings),
            "parameters": dict(self.parameters),
            "output_collection_name": self.output_collection_name,
            "output_mnemonic": self.output_mnemonic,
        }


@dataclass(frozen=True)
class ComputeRun:
    collection: Mapping[str, Any]
    asset: Mapping[str, Any]
    execution: Mapping[str, Any]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> ComputeRun:
        return cls(
            collection=_mapping(data["collection"], "collection"),
            asset=_mapping(data["asset"], "asset"),
            execution=_mapping(data["execution"], "execution"),
        )


@dataclass(frozen=True)
class AvoReflectivityRequest:
    method: str
    sample_shape: tuple[int, ...]
    angles_deg: tuple[float, ...]
    upper_vp_m_per_s: tuple[float, ...]
    upper_vs_m_per_s: tuple[float, ...]
    upper_density_g_cc: tuple[float, ...]
    lower_vp_m_per_s: tuple[float, ...]
    lower_vs_m_per_s: tuple[float, ...]
    lower_density_g_cc: tuple[float, ...]
    schema_version: int = 2

    def to_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "method": self.method,
            "sample_shape": list(self.sample_shape),
            "angles_deg": list(self.angles_deg),
            "upper_vp_m_per_s": list(self.upper_vp_m_per_s),
            "upper_vs_m_per_s": list(self.upper_vs_m_per_s),
            "upper_density_g_cc": list(self.upper_density_g_cc),
            "lower_vp_m_per_s": list(self.lower_vp_m_per_s),
            "lower_vs_m_per_s": list(self.lower_vs_m_per_s),
            "lower_density_g_cc": list(self.lower_density_g_cc),
        }


@dataclass(frozen=True)
class AvoReflectivityResponse:
    schema_version: int
    method: str
    sample_shape: tuple[int, ...]
    angles_deg: tuple[float, ...]
    pp_real_f32le: bytes
    pp_imag_f32le: bytes | None
    intercept_f32le: bytes | None
    gradient_f32le: bytes | None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> AvoReflectivityResponse:
        return cls(
            schema_version=int(data["schema_version"]),
            method=_string(data["method"], "method"),
            sample_shape=tuple(int(value) for value in data.get("sample_shape", [])),
            angles_deg=tuple(float(value) for value in data.get("angles_deg", [])),
            pp_real_f32le=bytes(data.get("pp_real_f32le", [])),
            pp_imag_f32le=(
                bytes(data["pp_imag_f32le"]) if data.get("pp_imag_f32le") is not None else None
            ),
            intercept_f32le=(
                bytes(data["intercept_f32le"]) if data.get("intercept_f32le") is not None else None
            ),
            gradient_f32le=(
                bytes(data["gradient_f32le"]) if data.get("gradient_f32le") is not None else None
            ),
        )

    def decode_f32le(self, payload: bytes | None) -> tuple[float, ...]:
        if payload is None:
            return ()
        if len(payload) % 4 != 0:
            raise ValueError("expected f32 little-endian byte payload")
        return tuple(value[0] for value in struct.iter_unpack("<f", payload))

    def pp_real(self) -> tuple[float, ...]:
        return self.decode_f32le(self.pp_real_f32le)

    def pp_imag(self) -> tuple[float, ...]:
        return self.decode_f32le(self.pp_imag_f32le)

    def intercept(self) -> tuple[float, ...]:
        return self.decode_f32le(self.intercept_f32le)

    def gradient(self) -> tuple[float, ...]:
        return self.decode_f32le(self.gradient_f32le)


@dataclass(frozen=True)
class RockPhysicsAttributeRequest:
    method: str
    sample_shape: tuple[int, ...]
    vp_m_per_s: tuple[float, ...] | None = None
    vs_m_per_s: tuple[float, ...] | None = None
    density_g_cc: tuple[float, ...] | None = None
    incident_angle_deg: float | None = None
    chi_angle_deg: float | None = None
    schema_version: int = 2

    def to_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "method": self.method,
            "sample_shape": list(self.sample_shape),
            "vp_m_per_s": list(self.vp_m_per_s) if self.vp_m_per_s is not None else None,
            "vs_m_per_s": list(self.vs_m_per_s) if self.vs_m_per_s is not None else None,
            "density_g_cc": list(self.density_g_cc) if self.density_g_cc is not None else None,
            "incident_angle_deg": self.incident_angle_deg,
            "chi_angle_deg": self.chi_angle_deg,
        }


@dataclass(frozen=True)
class RockPhysicsAttributeResponse:
    schema_version: int
    method: str
    sample_shape: tuple[int, ...]
    unit: str | None
    values_f32le: bytes
    semantic_parameters: Mapping[str, float]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> RockPhysicsAttributeResponse:
        return cls(
            schema_version=int(data["schema_version"]),
            method=_string(data["method"], "method"),
            sample_shape=tuple(int(value) for value in data.get("sample_shape", [])),
            unit=_optional_string(data.get("unit"), "unit"),
            values_f32le=bytes(data.get("values_f32le", [])),
            semantic_parameters={
                _string(key, "semantic_parameters"): float(value)
                for key, value in _mapping(
                    data.get("semantic_parameters", {}),
                    "semantic_parameters",
                ).items()
            },
        )

    def values(self) -> tuple[float, ...]:
        return AvoReflectivityResponse.decode_f32le(self, self.values_f32le)


@dataclass(frozen=True)
class AvoInterceptGradientAttributeRequest:
    method: str
    sample_shape: tuple[int, ...]
    intercept: tuple[float, ...]
    gradient: tuple[float, ...]
    chi_angle_deg: float | None = None
    intercept_scalar: float | None = None
    schema_version: int = 2

    def to_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "method": self.method,
            "sample_shape": list(self.sample_shape),
            "intercept": list(self.intercept),
            "gradient": list(self.gradient),
            "chi_angle_deg": self.chi_angle_deg,
            "intercept_scalar": self.intercept_scalar,
        }


@dataclass(frozen=True)
class AvoInterceptGradientAttributeResponse:
    schema_version: int
    method: str
    sample_shape: tuple[int, ...]
    unit: str | None
    values_f32le: bytes
    semantic_parameters: Mapping[str, float]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> AvoInterceptGradientAttributeResponse:
        return cls(
            schema_version=int(data["schema_version"]),
            method=_string(data["method"], "method"),
            sample_shape=tuple(int(value) for value in data.get("sample_shape", [])),
            unit=_optional_string(data.get("unit"), "unit"),
            values_f32le=bytes(data.get("values_f32le", [])),
            semantic_parameters={
                _string(key, "semantic_parameters"): float(value)
                for key, value in _mapping(
                    data.get("semantic_parameters", {}),
                    "semantic_parameters",
                ).items()
            },
        )

    def values(self) -> tuple[float, ...]:
        return AvoReflectivityResponse.decode_f32le(self, self.values_f32le)
