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
class SurveySummary:
    asset_id: str
    logical_asset_id: str
    collection_id: str
    name: str
    status: str
    owner_scope: str
    owner_id: str
    owner_name: str
    well_id: str
    well_name: str
    wellbore_id: str
    wellbore_name: str
    effective_coordinate_reference_id: str | None
    effective_coordinate_reference_name: str | None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SurveySummary:
        return cls(
            asset_id=_string(data["asset_id"], "asset_id"),
            logical_asset_id=_string(data["logical_asset_id"], "logical_asset_id"),
            collection_id=_string(data["collection_id"], "collection_id"),
            name=_string(data["name"], "name"),
            status=_string(data["status"], "status"),
            owner_scope=_string(data["owner_scope"], "owner_scope"),
            owner_id=_string(data["owner_id"], "owner_id"),
            owner_name=_string(data["owner_name"], "owner_name"),
            well_id=_string(data["well_id"], "well_id"),
            well_name=_string(data["well_name"], "well_name"),
            wellbore_id=_string(data["wellbore_id"], "wellbore_id"),
            wellbore_name=_string(data["wellbore_name"], "wellbore_name"),
            effective_coordinate_reference_id=_optional_string(
                data.get("effective_coordinate_reference_id"),
                "effective_coordinate_reference_id",
            ),
            effective_coordinate_reference_name=_optional_string(
                data.get("effective_coordinate_reference_name"),
                "effective_coordinate_reference_name",
            ),
        )


@dataclass(frozen=True)
class ImportResolution:
    status: str
    well_id: str
    wellbore_id: str
    created_well: bool
    created_wellbore: bool

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> ImportResolution:
        return cls(
            status=_string(data["status"], "status"),
            well_id=_string(data["well_id"], "well_id"),
            wellbore_id=_string(data["wellbore_id"], "wellbore_id"),
            created_well=bool(data["created_well"]),
            created_wellbore=bool(data["created_wellbore"]),
        )


@dataclass(frozen=True)
class LogAssetImportResult:
    resolution: ImportResolution
    collection: Mapping[str, Any]
    asset: Mapping[str, Any]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> LogAssetImportResult:
        return cls(
            resolution=ImportResolution.from_json(_mapping(data["resolution"], "resolution")),
            collection=_mapping(data["collection"], "collection"),
            asset=_mapping(data["asset"], "asset"),
        )


@dataclass(frozen=True)
class TopsSourceImportResult:
    schema_version: int
    source_path: Path
    source_name: str | None
    reported_well_name: str | None
    reported_depth_reference: str | None
    resolved_source_depth_reference: str | None
    resolved_depth_domain: str | None
    resolved_depth_datum: str | None
    source_row_count: int
    imported_row_count: int
    omitted_row_count: int
    import_result: LogAssetImportResult
    issues: tuple[Mapping[str, Any], ...]
    omissions: tuple[Mapping[str, Any], ...]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> TopsSourceImportResult:
        schema_version = data.get("schemaVersion", data.get("schema_version"))
        source_path = data.get("sourcePath", data.get("source_path"))
        source_name = data.get("sourceName", data.get("source_name"))
        reported_well_name = data.get("reportedWellName", data.get("reported_well_name"))
        reported_depth_reference = data.get(
            "reportedDepthReference",
            data.get("reported_depth_reference"),
        )
        resolved_source_depth_reference = data.get(
            "resolvedSourceDepthReference",
            data.get("resolved_source_depth_reference"),
        )
        resolved_depth_domain = data.get(
            "resolvedDepthDomain",
            data.get("resolved_depth_domain"),
        )
        resolved_depth_datum = data.get(
            "resolvedDepthDatum",
            data.get("resolved_depth_datum"),
        )
        source_row_count = data.get("sourceRowCount", data.get("source_row_count"))
        imported_row_count = data.get("importedRowCount", data.get("imported_row_count"))
        omitted_row_count = data.get("omittedRowCount", data.get("omitted_row_count"))
        import_result = data.get("importResult", data.get("import_result"))

        return cls(
            schema_version=int(schema_version),
            source_path=Path(_string(source_path, "source_path")),
            source_name=_optional_string(source_name, "source_name"),
            reported_well_name=_optional_string(reported_well_name, "reported_well_name"),
            reported_depth_reference=_optional_string(
                reported_depth_reference,
                "reported_depth_reference",
            ),
            resolved_source_depth_reference=_optional_string(
                resolved_source_depth_reference,
                "resolved_source_depth_reference",
            ),
            resolved_depth_domain=_optional_string(
                resolved_depth_domain,
                "resolved_depth_domain",
            ),
            resolved_depth_datum=_optional_string(
                resolved_depth_datum,
                "resolved_depth_datum",
            ),
            source_row_count=int(source_row_count),
            imported_row_count=int(imported_row_count),
            omitted_row_count=int(omitted_row_count),
            import_result=LogAssetImportResult.from_json(_mapping(import_result, "import_result")),
            issues=_mapping_list(data.get("issues", []), "issues"),
            omissions=_mapping_list(data.get("omissions", []), "omissions"),
        )


@dataclass(frozen=True)
class WellboreBinding:
    well_name: str
    wellbore_name: str
    uwi: str | None = None
    api: str | None = None
    operator_aliases: tuple[str, ...] = ()

    def to_payload(self) -> dict[str, Any]:
        return {
            "well_name": self.well_name,
            "wellbore_name": self.wellbore_name,
            "uwi": self.uwi,
            "api": self.api,
            "operator_aliases": list(self.operator_aliases),
        }

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> WellboreBinding:
        return cls(
            well_name=_string(data["well_name"], "well_name"),
            wellbore_name=_string(data["wellbore_name"], "wellbore_name"),
            uwi=_optional_string(data.get("uwi"), "uwi"),
            api=_optional_string(data.get("api"), "api"),
            operator_aliases=_string_list(data.get("operator_aliases", []), "operator_aliases"),
        )


@dataclass(frozen=True)
class WellPanelRequest:
    wellbore_ids: tuple[str, ...]
    depth_min: float | None = None
    depth_max: float | None = None
    schema_version: int = 1

    def to_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "wellbore_ids": list(self.wellbore_ids),
            "depth_min": self.depth_min,
            "depth_max": self.depth_max,
        }


@dataclass(frozen=True)
class SurveyMapRequest:
    survey_asset_ids: tuple[str, ...]
    wellbore_ids: tuple[str, ...]
    display_coordinate_reference_id: str
    schema_version: int = 2

    def to_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "survey_asset_ids": list(self.survey_asset_ids),
            "wellbore_ids": list(self.wellbore_ids),
            "display_coordinate_reference_id": self.display_coordinate_reference_id,
        }


@dataclass(frozen=True)
class SectionWellOverlayRequest:
    survey_asset_id: str
    wellbore_ids: tuple[str, ...]
    axis: str
    index: int
    display_domain: str
    tolerance_m: float | None = None
    schema_version: int = 1

    def to_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "survey_asset_id": self.survey_asset_id,
            "wellbore_ids": list(self.wellbore_ids),
            "axis": self.axis,
            "index": self.index,
            "display_domain": self.display_domain,
            "tolerance_m": self.tolerance_m,
        }


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
class OperatorContractRef:
    schema_id: str
    contract_id: str

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorContractRef:
        return cls(
            schema_id=_string(data["schema_id"], "schema_id"),
            contract_id=_string(data["contract_id"], "contract_id"),
        )


@dataclass(frozen=True)
class OperatorDocumentation:
    short_help: str
    help_markdown: str | None
    help_url: str | None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorDocumentation:
        return cls(
            short_help=_string(data["short_help"], "short_help"),
            help_markdown=_optional_string(data.get("help_markdown"), "help_markdown"),
            help_url=_optional_string(data.get("help_url"), "help_url"),
        )


@dataclass(frozen=True)
class OperatorParameterDoc:
    name: str
    label: str
    description: str
    value_kind: str
    required: bool
    default_value: str | None
    units: str | None
    options: tuple[str, ...]
    minimum: str | None
    maximum: str | None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorParameterDoc:
        return cls(
            name=_string(data["name"], "name"),
            label=_string(data["label"], "label"),
            description=_string(data["description"], "description"),
            value_kind=_string(data["value_kind"], "value_kind"),
            required=bool(data["required"]),
            default_value=_optional_string(data.get("default_value"), "default_value"),
            units=_optional_string(data.get("units"), "units"),
            options=_string_list(data.get("options", []), "options"),
            minimum=_optional_string(data.get("minimum"), "minimum"),
            maximum=_optional_string(data.get("maximum"), "maximum"),
        )


@dataclass(frozen=True)
class OperatorCatalogEntry:
    id: str
    provider: str
    name: str
    group: str
    group_id: str
    description: str
    family: str
    execution_kind: str
    output_lifecycle: str
    stability: str
    availability: Mapping[str, Any]
    tags: tuple[str, ...]
    documentation: OperatorDocumentation
    parameter_docs: tuple[OperatorParameterDoc, ...]
    request_contract: OperatorContractRef
    response_contract: OperatorContractRef
    detail: Mapping[str, Any]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorCatalogEntry:
        return cls(
            id=_string(data["id"], "id"),
            provider=_string(data["provider"], "provider"),
            name=_string(data["name"], "name"),
            group=_string(data["group"], "group"),
            group_id=_string(data["group_id"], "group_id"),
            description=_string(data["description"], "description"),
            family=_string(data["family"], "family"),
            execution_kind=_string(data["execution_kind"], "execution_kind"),
            output_lifecycle=_string(data["output_lifecycle"], "output_lifecycle"),
            stability=_string(data["stability"], "stability"),
            availability=_mapping(data["availability"], "availability"),
            tags=_string_list(data.get("tags", []), "tags"),
            documentation=OperatorDocumentation.from_json(
                _mapping(data["documentation"], "documentation")
            ),
            parameter_docs=tuple(
                OperatorParameterDoc.from_json(item)
                for item in _mapping_list(data.get("parameter_docs", []), "parameter_docs")
            ),
            request_contract=OperatorContractRef.from_json(
                _mapping(data["request_contract"], "request_contract")
            ),
            response_contract=OperatorContractRef.from_json(
                _mapping(data["response_contract"], "response_contract")
            ),
            detail=_mapping(data["detail"], "detail"),
        )


@dataclass(frozen=True)
class OperatorCatalog:
    schema_version: int
    subject_kind: str
    operators: tuple[OperatorCatalogEntry, ...]

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> OperatorCatalog:
        return cls(
            schema_version=int(data["schema_version"]),
            subject_kind=_string(data["subject_kind"], "subject_kind"),
            operators=tuple(
                OperatorCatalogEntry.from_json(item)
                for item in _mapping_list(data.get("operators", []), "operators")
            ),
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
