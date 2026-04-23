from __future__ import annotations

import json
import math
import os
import struct
import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Any, Mapping, Sequence

from .models import OperatorCatalog


REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_MANIFEST_PATH = REPO_ROOT / "Cargo.toml"

__all__ = [
    "AmplitudeScalar",
    "BandpassFilter",
    "ConstantVelocity",
    "DatasetDescriptor",
    "GatherPipeline",
    "GatherPreview",
    "GatherSelection",
    "GatherView",
    "HighpassFilter",
    "LowpassFilter",
    "NmoCorrection",
    "OffsetMute",
    "PhaseRotation",
    "PostStackNeighborhoodPipeline",
    "PostStackNeighborhoodWindow",
    "ProcessingPreview",
    "ProcessingBatchItemResult",
    "ProcessingBatchResult",
    "RmsAgc",
    "SectionSelection",
    "SectionView",
    "SeismicDataset",
    "SegyGeometryField",
    "SegyGeometryOverride",
    "SegyPreflight",
    "SemblancePanel",
    "StretchMute",
    "SubvolumeCrop",
    "SubvolumePipeline",
    "TraceBoostApp",
    "TraceBoostCommandError",
    "TraceLocalPipeline",
    "TraceProcessingPipeline",
    "TraceProcessingStep",
    "TraceRmsNormalize",
    "TimeVelocityPairs",
    "VelocityAssetReference",
    "VelocityAutopick",
    "VelocityFunctionEstimate",
    "VelocityScanResult",
    "VelocityScanSpec",
]


def _mapping(value: Any, field_name: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"expected '{field_name}' to be an object")
    return value


def _string(value: Any, field_name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"expected '{field_name}' to be a string")
    return value


def _optional_mapping(value: Any, field_name: str) -> Mapping[str, Any] | None:
    if value is None:
        return None
    return _mapping(value, field_name)


def _optional_string(value: Any, field_name: str) -> str | None:
    if value is None:
        return None
    return _string(value, field_name)


def _to_f32_sequence(byte_values: Sequence[int]) -> tuple[float, ...]:
    raw = bytes(byte_values)
    if len(raw) % 4 != 0:
        raise ValueError(f"expected byte length divisible by 4, got {len(raw)}")
    return tuple(item[0] for item in struct.iter_unpack("<f", raw))


def _to_f64_sequence(byte_values: Sequence[int]) -> tuple[float, ...]:
    raw = bytes(byte_values)
    if len(raw) % 8 != 0:
        raise ValueError(f"expected byte length divisible by 8, got {len(raw)}")
    return tuple(item[0] for item in struct.iter_unpack("<d", raw))


class TraceBoostCommandError(RuntimeError):
    def __init__(self, message: str, *, command: Sequence[str], stderr: str | None = None) -> None:
        super().__init__(message)
        self.command = tuple(command)
        self.stderr = stderr


@dataclass(frozen=True)
class ProcessingBatchItemResult:
    store_path: Path
    output_store_path: Path | None
    dataset: SeismicDataset | None
    error: str | None = None

    @property
    def succeeded(self) -> bool:
        return self.dataset is not None and self.error is None


@dataclass(frozen=True)
class ProcessingBatchResult:
    items: tuple[ProcessingBatchItemResult, ...]

    @property
    def succeeded(self) -> bool:
        return all(item.succeeded for item in self.items)

    @property
    def completed_with_errors(self) -> bool:
        return any(not item.succeeded for item in self.items)


@dataclass(frozen=True)
class SegyGeometryField:
    start_byte: int
    value_type: str

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SegyGeometryField:
        return cls(
            start_byte=int(data["start_byte"]),
            value_type=_string(data["value_type"], "value_type"),
        )

    def to_payload(self) -> dict[str, Any]:
        return {
            "start_byte": self.start_byte,
            "value_type": self.value_type,
        }


@dataclass(frozen=True)
class SegyGeometryOverride:
    inline_3d: SegyGeometryField | None = None
    crossline_3d: SegyGeometryField | None = None
    third_axis: SegyGeometryField | None = None

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SegyGeometryOverride:
        return cls(
            inline_3d=None
            if data.get("inline_3d") is None
            else SegyGeometryField.from_json(_mapping(data["inline_3d"], "inline_3d")),
            crossline_3d=None
            if data.get("crossline_3d") is None
            else SegyGeometryField.from_json(
                _mapping(data["crossline_3d"], "crossline_3d")
            ),
            third_axis=None
            if data.get("third_axis") is None
            else SegyGeometryField.from_json(_mapping(data["third_axis"], "third_axis")),
        )

    def to_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {}
        if self.inline_3d is not None:
            payload["inline_3d"] = self.inline_3d.to_payload()
        if self.crossline_3d is not None:
            payload["crossline_3d"] = self.crossline_3d.to_payload()
        if self.third_axis is not None:
            payload["third_axis"] = self.third_axis.to_payload()
        return payload

    def to_cli_args(self) -> list[str]:
        args: list[str] = []
        for field, byte_flag, type_flag in (
            (self.inline_3d, "--inline-byte", "--inline-type"),
            (self.crossline_3d, "--crossline-byte", "--crossline-type"),
            (self.third_axis, "--third-axis-byte", "--third-axis-type"),
        ):
            if field is None:
                continue
            args.extend([byte_flag, str(field.start_byte), type_flag, field.value_type])
        return args


@dataclass(frozen=True)
class SegyPreflight:
    input_path: Path
    trace_count: int
    samples_per_trace: int
    classification: str
    stacking_state: str
    organization: str
    layout: str
    suggested_action: str
    resolved_geometry: SegyGeometryOverride
    notes: tuple[str, ...]
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SegyPreflight:
        return cls(
            input_path=Path(_string(data["input_path"], "input_path")),
            trace_count=int(data["trace_count"]),
            samples_per_trace=int(data["samples_per_trace"]),
            classification=_string(data["classification"], "classification"),
            stacking_state=_string(data["stacking_state"], "stacking_state"),
            organization=_string(data["organization"], "organization"),
            layout=_string(data["layout"], "layout"),
            suggested_action=_string(data["suggested_action"], "suggested_action"),
            resolved_geometry=SegyGeometryOverride.from_json(
                _mapping(data["resolved_geometry"], "resolved_geometry")
            ),
            notes=tuple(_string(item, "notes[]") for item in data.get("notes", [])),
            raw_payload=data,
        )

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class DatasetDescriptor:
    id: str
    store_id: str
    label: str
    shape: tuple[int, int, int]
    chunk_shape: tuple[int, int, int]
    sample_interval_ms: float
    processing_lineage_summary: Mapping[str, Any] | None
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> DatasetDescriptor:
        return cls(
            id=_string(data["id"], "id"),
            store_id=_string(data["store_id"], "store_id"),
            label=_string(data["label"], "label"),
            shape=tuple(int(value) for value in data["shape"]),
            chunk_shape=tuple(int(value) for value in data["chunk_shape"]),
            sample_interval_ms=float(data["sample_interval_ms"]),
            processing_lineage_summary=_optional_mapping(
                data.get("processing_lineage_summary"),
                "processing_lineage_summary",
            ),
            raw_payload=data,
        )

    @property
    def inline_count(self) -> int:
        return self.shape[0]

    @property
    def xline_count(self) -> int:
        return self.shape[1]

    @property
    def sample_count(self) -> int:
        return self.shape[2]

    @property
    def nyquist_hz(self) -> float:
        return 1000.0 / (2.0 * self.sample_interval_ms)

    def midpoint_section(self, axis: str = "inline") -> SectionSelection:
        if axis == "inline":
            return SectionSelection.inline(self.inline_count // 2)
        if axis == "xline":
            return SectionSelection.xline(self.xline_count // 2)
        raise ValueError(f"unsupported section axis '{axis}'")

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class SectionSelection:
    axis: str
    index: int

    @classmethod
    def inline(cls, index: int) -> SectionSelection:
        return cls(axis="inline", index=index)

    @classmethod
    def xline(cls, index: int) -> SectionSelection:
        return cls(axis="xline", index=index)

    def to_payload(self, *, dataset_id: str) -> dict[str, Any]:
        return {
            "dataset_id": dataset_id,
            "axis": self.axis,
            "index": self.index,
        }

    def to_project_payload(self) -> dict[str, Any]:
        return {
            "axis": self.axis,
            "index": self.index,
        }


@dataclass(frozen=True)
class SectionView:
    dataset_id: str
    axis: str
    coordinate_index: int
    coordinate_value: float
    traces: int
    samples: int
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SectionView:
        coordinate = _mapping(data["coordinate"], "coordinate")
        return cls(
            dataset_id=_string(data["dataset_id"], "dataset_id"),
            axis=_string(data["axis"], "axis"),
            coordinate_index=int(coordinate["index"]),
            coordinate_value=float(coordinate["value"]),
            traces=int(data["traces"]),
            samples=int(data["samples"]),
            raw_payload=data,
        )

    @property
    def coordinate(self) -> dict[str, Any]:
        return {
            "index": self.coordinate_index,
            "value": self.coordinate_value,
        }

    def horizontal_axis(self) -> tuple[float, ...]:
        return _to_f64_sequence(self.raw_payload["horizontal_axis_f64le"])

    def sample_axis_ms(self) -> tuple[float, ...]:
        return _to_f32_sequence(self.raw_payload["sample_axis_f32le"])

    def amplitudes(self) -> tuple[float, ...]:
        return _to_f32_sequence(self.raw_payload["amplitudes_f32le"])

    def stats(self) -> dict[str, Any]:
        amplitudes = self.amplitudes()
        if not amplitudes:
            return {
                "trace_count": self.traces,
                "sample_count": self.samples,
                "amplitude_count": 0,
            }
        amplitude_count = len(amplitudes)
        mean = sum(amplitudes) / amplitude_count
        mean_abs = sum(abs(value) for value in amplitudes) / amplitude_count
        rms = math.sqrt(sum(value * value for value in amplitudes) / amplitude_count)
        return {
            "trace_count": self.traces,
            "sample_count": self.samples,
            "amplitude_count": amplitude_count,
            "min_amplitude": min(amplitudes),
            "max_amplitude": max(amplitudes),
            "mean_amplitude": mean,
            "mean_abs_amplitude": mean_abs,
            "rms_amplitude": rms,
        }

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class GatherSelection:
    selector: Mapping[str, Any]
    dataset_id: str | None = None

    @classmethod
    def inline_xline(cls, inline: int, xline: int) -> GatherSelection:
        return cls(selector={"inline_xline": {"inline": inline, "xline": xline}})

    @classmethod
    def coordinate(cls, coordinate: float) -> GatherSelection:
        return cls(selector={"coordinate": {"coordinate": coordinate}})

    @classmethod
    def ordinal(cls, index: int) -> GatherSelection:
        return cls(selector={"ordinal": {"index": index}})

    def to_payload(self, *, dataset_id: str | None = None) -> dict[str, Any]:
        resolved_dataset_id = dataset_id or self.dataset_id
        if resolved_dataset_id is None:
            raise ValueError("gather selection requires a dataset_id")
        return {
            "dataset_id": resolved_dataset_id,
            "selector": dict(self.selector),
        }

    def to_project_payload(self) -> dict[str, Any]:
        return {"selector": dict(self.selector)}


@dataclass(frozen=True)
class GatherView:
    dataset_id: str
    label: str
    gather_axis_kind: str
    sample_domain: str
    traces: int
    samples: int
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> GatherView:
        return cls(
            dataset_id=_string(data["dataset_id"], "dataset_id"),
            label=_string(data["label"], "label"),
            gather_axis_kind=_string(data["gather_axis_kind"], "gather_axis_kind"),
            sample_domain=_string(data["sample_domain"], "sample_domain"),
            traces=int(data["traces"]),
            samples=int(data["samples"]),
            raw_payload=data,
        )

    def horizontal_axis(self) -> tuple[float, ...]:
        return _to_f64_sequence(self.raw_payload["horizontal_axis_f64le"])

    def sample_axis(self) -> tuple[float, ...]:
        return _to_f32_sequence(self.raw_payload["sample_axis_f32le"])

    def amplitudes(self) -> tuple[float, ...]:
        return _to_f32_sequence(self.raw_payload["amplitudes_f32le"])

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


class _TraceProcessingOperation:
    def operator_id(self) -> str:
        raise NotImplementedError

    def to_payload(self) -> dict[str, Any]:
        raise NotImplementedError


@dataclass(frozen=True)
class AmplitudeScalar(_TraceProcessingOperation):
    factor: float

    def operator_id(self) -> str:
        return "amplitude_scalar"

    def to_payload(self) -> dict[str, Any]:
        return {self.operator_id(): {"factor": self.factor}}


@dataclass(frozen=True)
class TraceRmsNormalize(_TraceProcessingOperation):
    def operator_id(self) -> str:
        return "trace_rms_normalize"

    def to_payload(self) -> dict[str, Any]:
        return self.operator_id()


@dataclass(frozen=True)
class RmsAgc(_TraceProcessingOperation):
    window_ms: float

    def operator_id(self) -> str:
        return "agc_rms"

    def to_payload(self) -> dict[str, Any]:
        return {self.operator_id(): {"window_ms": self.window_ms}}


@dataclass(frozen=True)
class PhaseRotation(_TraceProcessingOperation):
    angle_degrees: float

    def operator_id(self) -> str:
        return "phase_rotation"

    def to_payload(self) -> dict[str, Any]:
        return {self.operator_id(): {"angle_degrees": self.angle_degrees}}


@dataclass(frozen=True)
class Envelope(_TraceProcessingOperation):
    def operator_id(self) -> str:
        return "envelope"

    def to_payload(self) -> str:
        return self.operator_id()


@dataclass(frozen=True)
class InstantaneousPhase(_TraceProcessingOperation):
    def operator_id(self) -> str:
        return "instantaneous_phase"

    def to_payload(self) -> str:
        return self.operator_id()


@dataclass(frozen=True)
class InstantaneousFrequency(_TraceProcessingOperation):
    def operator_id(self) -> str:
        return "instantaneous_frequency"

    def to_payload(self) -> str:
        return self.operator_id()


@dataclass(frozen=True)
class Sweetness(_TraceProcessingOperation):
    def operator_id(self) -> str:
        return "sweetness"

    def to_payload(self) -> str:
        return self.operator_id()


@dataclass(frozen=True)
class LowpassFilter(_TraceProcessingOperation):
    f3_hz: float
    f4_hz: float
    phase: str = "zero"
    window: str = "cosine_taper"

    def operator_id(self) -> str:
        return "lowpass_filter"

    def to_payload(self) -> dict[str, Any]:
        return {
            self.operator_id(): {
                "f3_hz": self.f3_hz,
                "f4_hz": self.f4_hz,
                "phase": self.phase,
                "window": self.window,
            }
        }


@dataclass(frozen=True)
class HighpassFilter(_TraceProcessingOperation):
    f1_hz: float
    f2_hz: float
    phase: str = "zero"
    window: str = "cosine_taper"

    def operator_id(self) -> str:
        return "highpass_filter"

    def to_payload(self) -> dict[str, Any]:
        return {
            self.operator_id(): {
                "f1_hz": self.f1_hz,
                "f2_hz": self.f2_hz,
                "phase": self.phase,
                "window": self.window,
            }
        }


@dataclass(frozen=True)
class BandpassFilter(_TraceProcessingOperation):
    f1_hz: float
    f2_hz: float
    f3_hz: float
    f4_hz: float
    phase: str = "zero"
    window: str = "cosine_taper"

    def operator_id(self) -> str:
        return "bandpass_filter"

    def to_payload(self) -> dict[str, Any]:
        return {
            self.operator_id(): {
                "f1_hz": self.f1_hz,
                "f2_hz": self.f2_hz,
                "f3_hz": self.f3_hz,
                "f4_hz": self.f4_hz,
                "phase": self.phase,
                "window": self.window,
            }
        }


@dataclass(frozen=True)
class TraceProcessingStep:
    operation: _TraceProcessingOperation
    checkpoint: bool = False

    def to_payload(self) -> dict[str, Any]:
        return {
            "operation": self.operation.to_payload(),
            "checkpoint": self.checkpoint,
        }


@dataclass(frozen=True)
class TraceProcessingPipeline:
    steps: tuple[TraceProcessingStep, ...] = ()
    name: str | None = None
    description: str | None = None
    schema_version: int = 2
    revision: int = 1

    @classmethod
    def named(
        cls,
        name: str,
        *,
        description: str | None = None,
        revision: int = 1,
    ) -> TraceProcessingPipeline:
        return cls(
            name=name,
            description=description,
            revision=revision,
        )

    def with_step(
        self,
        operation: _TraceProcessingOperation,
        *,
        checkpoint: bool = False,
    ) -> TraceProcessingPipeline:
        return replace(
            self,
            steps=(*self.steps, TraceProcessingStep(operation=operation, checkpoint=checkpoint)),
        )

    def amplitude_scalar(self, factor: float, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(AmplitudeScalar(factor=factor), checkpoint=checkpoint)

    def trace_rms_normalize(self, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(TraceRmsNormalize(), checkpoint=checkpoint)

    def agc_rms(self, window_ms: float, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(RmsAgc(window_ms=window_ms), checkpoint=checkpoint)

    def phase_rotation(
        self, angle_degrees: float, *, checkpoint: bool = False
    ) -> TraceProcessingPipeline:
        return self.with_step(
            PhaseRotation(angle_degrees=angle_degrees),
            checkpoint=checkpoint,
        )

    def envelope(self, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(Envelope(), checkpoint=checkpoint)

    def instantaneous_phase(self, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(InstantaneousPhase(), checkpoint=checkpoint)

    def instantaneous_frequency(self, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(InstantaneousFrequency(), checkpoint=checkpoint)

    def sweetness(self, *, checkpoint: bool = False) -> TraceProcessingPipeline:
        return self.with_step(Sweetness(), checkpoint=checkpoint)

    def lowpass(
        self,
        f3_hz: float,
        f4_hz: float,
        *,
        phase: str = "zero",
        window: str = "cosine_taper",
        checkpoint: bool = False,
    ) -> TraceProcessingPipeline:
        return self.with_step(
            LowpassFilter(f3_hz=f3_hz, f4_hz=f4_hz, phase=phase, window=window),
            checkpoint=checkpoint,
        )

    def highpass(
        self,
        f1_hz: float,
        f2_hz: float,
        *,
        phase: str = "zero",
        window: str = "cosine_taper",
        checkpoint: bool = False,
    ) -> TraceProcessingPipeline:
        return self.with_step(
            HighpassFilter(f1_hz=f1_hz, f2_hz=f2_hz, phase=phase, window=window),
            checkpoint=checkpoint,
        )

    def bandpass(
        self,
        f1_hz: float,
        f2_hz: float,
        f3_hz: float,
        f4_hz: float,
        *,
        phase: str = "zero",
        window: str = "cosine_taper",
        checkpoint: bool = False,
    ) -> TraceProcessingPipeline:
        return self.with_step(
            BandpassFilter(
                f1_hz=f1_hz,
                f2_hz=f2_hz,
                f3_hz=f3_hz,
                f4_hz=f4_hz,
                phase=phase,
                window=window,
            ),
            checkpoint=checkpoint,
        )

    def operator_ids(self) -> tuple[str, ...]:
        return tuple(step.operation.operator_id() for step in self.steps)

    def validate_for(self, dataset: DatasetDescriptor) -> None:
        for step in self.steps:
            operation = step.operation
            if isinstance(operation, BandpassFilter):
                if not (
                    operation.f1_hz < operation.f2_hz < operation.f3_hz < operation.f4_hz
                ):
                    raise ValueError("bandpass frequencies must satisfy f1 < f2 < f3 < f4")
                if operation.f4_hz >= dataset.nyquist_hz:
                    raise ValueError(
                        f"bandpass f4_hz={operation.f4_hz} exceeds or equals Nyquist "
                        f"{dataset.nyquist_hz:.3f} Hz"
                    )
            if isinstance(operation, LowpassFilter):
                if not (operation.f3_hz < operation.f4_hz):
                    raise ValueError("lowpass frequencies must satisfy f3 < f4")
                if operation.f4_hz >= dataset.nyquist_hz:
                    raise ValueError(
                        f"lowpass f4_hz={operation.f4_hz} exceeds or equals Nyquist "
                        f"{dataset.nyquist_hz:.3f} Hz"
                    )
            if isinstance(operation, HighpassFilter):
                if not (operation.f1_hz < operation.f2_hz):
                    raise ValueError("highpass frequencies must satisfy f1 < f2")
                if operation.f2_hz >= dataset.nyquist_hz:
                    raise ValueError(
                        f"highpass f2_hz={operation.f2_hz} exceeds or equals Nyquist "
                        f"{dataset.nyquist_hz:.3f} Hz"
                    )

    def to_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "schema_version": self.schema_version,
            "revision": self.revision,
            "steps": [step.to_payload() for step in self.steps],
        }
        if self.name is not None:
            payload["name"] = self.name
        if self.description is not None:
            payload["description"] = self.description
        return payload


TraceLocalPipeline = TraceProcessingPipeline


@dataclass(frozen=True)
class SubvolumeCrop:
    inline_min: int
    inline_max: int
    xline_min: int
    xline_max: int
    z_min_ms: float
    z_max_ms: float

    def to_payload(self) -> dict[str, Any]:
        return {
            "inline_min": self.inline_min,
            "inline_max": self.inline_max,
            "xline_min": self.xline_min,
            "xline_max": self.xline_max,
            "z_min_ms": self.z_min_ms,
            "z_max_ms": self.z_max_ms,
        }


@dataclass(frozen=True)
class SubvolumePipeline:
    crop_operation: SubvolumeCrop
    trace_local_pipeline: TraceLocalPipeline | None = None
    name: str | None = None
    description: str | None = None
    schema_version: int = 2
    revision: int = 1

    @classmethod
    def crop(
        cls,
        *,
        inline_min: int,
        inline_max: int,
        xline_min: int,
        xline_max: int,
        z_min_ms: float,
        z_max_ms: float,
        trace_local_pipeline: TraceLocalPipeline | None = None,
        name: str | None = None,
        description: str | None = None,
        revision: int = 1,
    ) -> SubvolumePipeline:
        return cls(
            crop_operation=SubvolumeCrop(
                inline_min=inline_min,
                inline_max=inline_max,
                xline_min=xline_min,
                xline_max=xline_max,
                z_min_ms=z_min_ms,
                z_max_ms=z_max_ms,
            ),
            trace_local_pipeline=trace_local_pipeline,
            name=name,
            description=description,
            revision=revision,
        )

    def with_trace_local(self, pipeline: TraceLocalPipeline) -> SubvolumePipeline:
        return replace(self, trace_local_pipeline=pipeline)

    def operator_ids(self) -> tuple[str, ...]:
        return ("crop",)

    def to_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "schema_version": self.schema_version,
            "revision": self.revision,
            "crop": self.crop_operation.to_payload(),
        }
        if self.name is not None:
            payload["name"] = self.name
        if self.description is not None:
            payload["description"] = self.description
        if self.trace_local_pipeline is not None:
            payload["trace_local_pipeline"] = self.trace_local_pipeline.to_payload()
        return payload


@dataclass(frozen=True)
class PostStackNeighborhoodWindow:
    gate_ms: float
    inline_stepout: int
    xline_stepout: int

    def to_payload(self) -> dict[str, Any]:
        return {
            "gate_ms": self.gate_ms,
            "inline_stepout": self.inline_stepout,
            "xline_stepout": self.xline_stepout,
        }


class _PostStackNeighborhoodOperation:
    def operator_id(self) -> str:
        raise NotImplementedError

    def to_payload(self) -> Mapping[str, Any]:
        raise NotImplementedError


@dataclass(frozen=True)
class _SimilarityNeighborhood(_PostStackNeighborhoodOperation):
    window: PostStackNeighborhoodWindow

    def operator_id(self) -> str:
        return "similarity"

    def to_payload(self) -> Mapping[str, Any]:
        return {self.operator_id(): {"window": self.window.to_payload()}}


@dataclass(frozen=True)
class _LocalVolumeStatsNeighborhood(_PostStackNeighborhoodOperation):
    window: PostStackNeighborhoodWindow
    statistic: str

    def operator_id(self) -> str:
        return "local_volume_stats"

    def to_payload(self) -> Mapping[str, Any]:
        return {
            self.operator_id(): {
                "window": self.window.to_payload(),
                "statistic": self.statistic,
            }
        }


@dataclass(frozen=True)
class _DipNeighborhood(_PostStackNeighborhoodOperation):
    window: PostStackNeighborhoodWindow
    output: str

    def operator_id(self) -> str:
        return "dip"

    def to_payload(self) -> Mapping[str, Any]:
        return {
            self.operator_id(): {
                "window": self.window.to_payload(),
                "output": self.output,
            }
        }


@dataclass(frozen=True)
class PostStackNeighborhoodPipeline:
    operations: tuple[_PostStackNeighborhoodOperation, ...] = ()
    trace_local_pipeline: TraceLocalPipeline | None = None
    name: str | None = None
    description: str | None = None
    schema_version: int = 2
    revision: int = 1

    @classmethod
    def named(
        cls,
        name: str,
        *,
        description: str | None = None,
        trace_local_pipeline: TraceLocalPipeline | None = None,
        revision: int = 1,
    ) -> PostStackNeighborhoodPipeline:
        return cls(
            name=name,
            description=description,
            trace_local_pipeline=trace_local_pipeline,
            revision=revision,
        )

    def with_trace_local(
        self, pipeline: TraceLocalPipeline
    ) -> PostStackNeighborhoodPipeline:
        return replace(self, trace_local_pipeline=pipeline)

    def with_operation(
        self, operation: _PostStackNeighborhoodOperation
    ) -> PostStackNeighborhoodPipeline:
        return replace(self, operations=(*self.operations, operation))

    def similarity(self, window: PostStackNeighborhoodWindow) -> PostStackNeighborhoodPipeline:
        return self.with_operation(_SimilarityNeighborhood(window=window))

    def local_volume_stats(
        self,
        window: PostStackNeighborhoodWindow,
        *,
        statistic: str,
    ) -> PostStackNeighborhoodPipeline:
        return self.with_operation(
            _LocalVolumeStatsNeighborhood(window=window, statistic=statistic)
        )

    def dip(
        self,
        window: PostStackNeighborhoodWindow,
        *,
        output: str,
    ) -> PostStackNeighborhoodPipeline:
        return self.with_operation(_DipNeighborhood(window=window, output=output))

    def operator_ids(self) -> tuple[str, ...]:
        return tuple(operation.operator_id() for operation in self.operations)

    def to_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "schema_version": self.schema_version,
            "revision": self.revision,
            "operations": [operation.to_payload() for operation in self.operations],
        }
        if self.name is not None:
            payload["name"] = self.name
        if self.description is not None:
            payload["description"] = self.description
        if self.trace_local_pipeline is not None:
            payload["trace_local_pipeline"] = self.trace_local_pipeline.to_payload()
        return payload


class _VelocityFunctionSource:
    def to_payload(self) -> Mapping[str, Any]:
        raise NotImplementedError


@dataclass(frozen=True)
class ConstantVelocity(_VelocityFunctionSource):
    velocity_m_per_s: float

    def to_payload(self) -> Mapping[str, Any]:
        return {"constant_velocity": {"velocity_m_per_s": self.velocity_m_per_s}}


@dataclass(frozen=True)
class TimeVelocityPairs(_VelocityFunctionSource):
    times_ms: tuple[float, ...]
    velocities_m_per_s: tuple[float, ...]

    def to_payload(self) -> Mapping[str, Any]:
        return {
            "time_velocity_pairs": {
                "times_ms": list(self.times_ms),
                "velocities_m_per_s": list(self.velocities_m_per_s),
            }
        }


@dataclass(frozen=True)
class VelocityAssetReference(_VelocityFunctionSource):
    asset_id: str

    def to_payload(self) -> Mapping[str, Any]:
        return {"velocity_asset_reference": {"asset_id": self.asset_id}}


class _GatherProcessingOperation:
    def operator_id(self) -> str:
        raise NotImplementedError

    def to_payload(self) -> Mapping[str, Any]:
        raise NotImplementedError


@dataclass(frozen=True)
class NmoCorrection(_GatherProcessingOperation):
    velocity_model: _VelocityFunctionSource
    interpolation: str = "linear"

    def operator_id(self) -> str:
        return "nmo_correction"

    def to_payload(self) -> Mapping[str, Any]:
        return {
            self.operator_id(): {
                "velocity_model": self.velocity_model.to_payload(),
                "interpolation": self.interpolation,
            }
        }


@dataclass(frozen=True)
class StretchMute(_GatherProcessingOperation):
    velocity_model: _VelocityFunctionSource
    max_stretch_ratio: float

    def operator_id(self) -> str:
        return "stretch_mute"

    def to_payload(self) -> Mapping[str, Any]:
        return {
            self.operator_id(): {
                "velocity_model": self.velocity_model.to_payload(),
                "max_stretch_ratio": self.max_stretch_ratio,
            }
        }


@dataclass(frozen=True)
class OffsetMute(_GatherProcessingOperation):
    min_offset: float | None = None
    max_offset: float | None = None

    def operator_id(self) -> str:
        return "offset_mute"

    def to_payload(self) -> Mapping[str, Any]:
        payload: dict[str, Any] = {}
        if self.min_offset is not None:
            payload["min_offset"] = self.min_offset
        if self.max_offset is not None:
            payload["max_offset"] = self.max_offset
        return {self.operator_id(): payload}


@dataclass(frozen=True)
class GatherPipeline:
    operations: tuple[_GatherProcessingOperation, ...] = ()
    trace_local_pipeline: TraceLocalPipeline | None = None
    name: str | None = None
    description: str | None = None
    schema_version: int = 2
    revision: int = 1

    @classmethod
    def named(
        cls,
        name: str,
        *,
        description: str | None = None,
        trace_local_pipeline: TraceLocalPipeline | None = None,
        revision: int = 1,
    ) -> GatherPipeline:
        return cls(
            name=name,
            description=description,
            trace_local_pipeline=trace_local_pipeline,
            revision=revision,
        )

    def with_trace_local(self, pipeline: TraceLocalPipeline) -> GatherPipeline:
        return replace(self, trace_local_pipeline=pipeline)

    def with_operation(self, operation: _GatherProcessingOperation) -> GatherPipeline:
        return replace(self, operations=(*self.operations, operation))

    def nmo_correction(
        self,
        velocity_model: _VelocityFunctionSource,
        *,
        interpolation: str = "linear",
    ) -> GatherPipeline:
        return self.with_operation(
            NmoCorrection(velocity_model=velocity_model, interpolation=interpolation)
        )

    def stretch_mute(
        self,
        velocity_model: _VelocityFunctionSource,
        *,
        max_stretch_ratio: float,
    ) -> GatherPipeline:
        return self.with_operation(
            StretchMute(
                velocity_model=velocity_model,
                max_stretch_ratio=max_stretch_ratio,
            )
        )

    def offset_mute(
        self,
        *,
        min_offset: float | None = None,
        max_offset: float | None = None,
    ) -> GatherPipeline:
        return self.with_operation(OffsetMute(min_offset=min_offset, max_offset=max_offset))

    def operator_ids(self) -> tuple[str, ...]:
        return tuple(operation.operator_id() for operation in self.operations)

    def to_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "schema_version": self.schema_version,
            "revision": self.revision,
            "operations": [operation.to_payload() for operation in self.operations],
        }
        if self.name is not None:
            payload["name"] = self.name
        if self.description is not None:
            payload["description"] = self.description
        if self.trace_local_pipeline is not None:
            payload["trace_local_pipeline"] = self.trace_local_pipeline.to_payload()
        return payload


@dataclass(frozen=True)
class ProcessingPreview:
    section: SectionView
    processing_label: str
    preview_ready: bool
    pipeline: Any
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(
        cls,
        data: Mapping[str, Any],
        *,
        pipeline: TraceProcessingPipeline,
    ) -> ProcessingPreview:
        preview = _mapping(data["preview"], "preview")
        return cls(
            section=SectionView.from_json(_mapping(preview["section"], "preview.section")),
            processing_label=_string(preview["processing_label"], "preview.processing_label"),
            preview_ready=bool(preview["preview_ready"]),
            pipeline=pipeline,
            raw_payload=data,
        )

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class GatherPreview:
    gather: GatherView
    processing_label: str
    preview_ready: bool
    pipeline: GatherPipeline
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(
        cls,
        data: Mapping[str, Any],
        *,
        pipeline: GatherPipeline,
    ) -> GatherPreview:
        preview = _mapping(data["preview"], "preview")
        return cls(
            gather=GatherView.from_json(_mapping(preview["gather"], "preview.gather")),
            processing_label=_string(preview["processing_label"], "preview.processing_label"),
            preview_ready=bool(preview["preview_ready"]),
            pipeline=pipeline,
            raw_payload=data,
        )

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class VelocityAutopick:
    sample_stride: int
    min_semblance: float
    smoothing_samples: int
    min_time_ms: float | None = None
    max_time_ms: float | None = None

    def to_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "sample_stride": self.sample_stride,
            "min_semblance": self.min_semblance,
            "smoothing_samples": self.smoothing_samples,
        }
        if self.min_time_ms is not None:
            payload["min_time_ms"] = self.min_time_ms
        if self.max_time_ms is not None:
            payload["max_time_ms"] = self.max_time_ms
        return payload


@dataclass(frozen=True)
class VelocityScanSpec:
    min_velocity_m_per_s: float
    max_velocity_m_per_s: float
    velocity_step_m_per_s: float
    trace_local_pipeline: TraceLocalPipeline | None = None
    autopick: VelocityAutopick | None = None
    schema_version: int = 1

    def to_payload(self, *, store_path: Path, gather: GatherSelection, dataset_id: str) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "schema_version": self.schema_version,
            "store_path": str(store_path),
            "gather": gather.to_payload(dataset_id=dataset_id),
            "min_velocity_m_per_s": self.min_velocity_m_per_s,
            "max_velocity_m_per_s": self.max_velocity_m_per_s,
            "velocity_step_m_per_s": self.velocity_step_m_per_s,
        }
        if self.trace_local_pipeline is not None:
            payload["trace_local_pipeline"] = self.trace_local_pipeline.to_payload()
        if self.autopick is not None:
            payload["autopick"] = self.autopick.to_payload()
        return payload

    def to_project_payload(
        self,
        *,
        source_asset_id: str,
        gather: GatherSelection,
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "source_asset_id": source_asset_id,
            "gather": gather.to_project_payload(),
            "min_velocity_m_per_s": self.min_velocity_m_per_s,
            "max_velocity_m_per_s": self.max_velocity_m_per_s,
            "velocity_step_m_per_s": self.velocity_step_m_per_s,
        }
        if self.trace_local_pipeline is not None:
            payload["trace_local_pipeline"] = self.trace_local_pipeline.to_payload()
        if self.autopick is not None:
            payload["autopick"] = self.autopick.to_payload()
        return payload


@dataclass(frozen=True)
class SemblancePanel:
    velocities_m_per_s: tuple[float, ...]
    sample_axis_ms: tuple[float, ...]
    semblance_f32le: tuple[float, ...]
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SemblancePanel:
        return cls(
            velocities_m_per_s=tuple(float(value) for value in data["velocities_m_per_s"]),
            sample_axis_ms=tuple(float(value) for value in data["sample_axis_ms"]),
            semblance_f32le=_to_f32_sequence(data["semblance_f32le"]),
            raw_payload=data,
        )

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class VelocityFunctionEstimate:
    strategy: str
    times_ms: tuple[float, ...]
    velocities_m_per_s: tuple[float, ...]
    semblance: tuple[float, ...]
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> VelocityFunctionEstimate:
        return cls(
            strategy=_string(data["strategy"], "strategy"),
            times_ms=tuple(float(value) for value in data["times_ms"]),
            velocities_m_per_s=tuple(float(value) for value in data["velocities_m_per_s"]),
            semblance=tuple(float(value) for value in data["semblance"]),
            raw_payload=data,
        )

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class VelocityScanResult:
    gather: GatherSelection
    panel: SemblancePanel
    processing_label: str | None
    autopicked_velocity_function: VelocityFunctionEstimate | None
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> VelocityScanResult:
        payload = _mapping(data, "velocity_scan")
        gather = _mapping(payload["gather"], "gather")
        return cls(
            gather=GatherSelection(
                selector=_mapping(gather["selector"], "gather.selector"),
                dataset_id=_optional_string(gather.get("dataset_id"), "gather.dataset_id"),
            ),
            panel=SemblancePanel.from_json(_mapping(payload["panel"], "panel")),
            processing_label=_optional_string(payload.get("processing_label"), "processing_label"),
            autopicked_velocity_function=None
            if payload.get("autopicked_velocity_function") is None
            else VelocityFunctionEstimate.from_json(
                _mapping(
                    payload["autopicked_velocity_function"],
                    "autopicked_velocity_function",
                )
            ),
            raw_payload=payload,
        )

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class SeismicDataset:
    """Compatibility surface for direct loose-store seismic workflows."""

    app: TraceBoostApp
    store_path: Path
    descriptor: DatasetDescriptor
    raw_payload: Mapping[str, Any] = field(repr=False)

    @classmethod
    def from_json(
        cls,
        data: Mapping[str, Any],
        *,
        app: TraceBoostApp,
    ) -> SeismicDataset:
        if "dataset" in data:
            dataset = _mapping(data["dataset"], "dataset")
        else:
            dataset = data
        return cls(
            app=app,
            store_path=Path(_string(dataset["store_path"], "store_path")),
            descriptor=DatasetDescriptor.from_json(
                _mapping(dataset["descriptor"], "descriptor")
            ),
            raw_payload=data,
        )

    def midpoint_section(self, axis: str = "inline") -> SectionSelection:
        return self.descriptor.midpoint_section(axis=axis)

    def operator_catalog(self) -> OperatorCatalog:
        return self.app.dataset_operator_catalog(self.store_path)

    def section(self, selection: SectionSelection) -> SectionView:
        payload = self.app.run_json(
            "view-section",
            str(self.store_path),
            selection.axis,
            str(selection.index),
        )
        return SectionView.from_json(_mapping(payload, "section"))

    def preview_processing(
        self,
        selection: SectionSelection,
        pipeline: TraceProcessingPipeline,
    ) -> ProcessingPreview:
        pipeline.validate_for(self.descriptor)
        request = {
            "schema_version": 1,
            "store_path": str(self.store_path),
            "section": selection.to_payload(dataset_id=self.descriptor.id),
            "pipeline": pipeline.to_payload(),
        }
        payload = self.app.run_json(
            "preview-processing",
            "-",
            stdin_text=json.dumps(request),
        )
        return ProcessingPreview.from_json(
            _mapping(payload, "preview"),
            pipeline=pipeline,
        )

    def preview_subvolume(
        self,
        selection: SectionSelection,
        pipeline: SubvolumePipeline,
    ) -> ProcessingPreview:
        request = {
            "schema_version": 1,
            "store_path": str(self.store_path),
            "section": selection.to_payload(dataset_id=self.descriptor.id),
            "pipeline": pipeline.to_payload(),
        }
        payload = self.app.run_json(
            "preview-subvolume-processing",
            "-",
            stdin_text=json.dumps(request),
        )
        return ProcessingPreview.from_json(
            _mapping(payload, "preview"),
            pipeline=pipeline,
        )

    def run_processing(
        self,
        pipeline: TraceProcessingPipeline,
        *,
        output_store_path: str | Path | None = None,
        overwrite_existing: bool = False,
    ) -> SeismicDataset:
        pipeline.validate_for(self.descriptor)
        request: dict[str, Any] = {
            "schema_version": 1,
            "store_path": str(self.store_path),
            "overwrite_existing": overwrite_existing,
            "pipeline": pipeline.to_payload(),
        }
        if output_store_path is not None:
            request["output_store_path"] = str(Path(output_store_path))
        payload = self.app.run_json(
            "run-processing",
            "-",
            stdin_text=json.dumps(request),
        )
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self.app)

    def run_subvolume(
        self,
        pipeline: SubvolumePipeline,
        *,
        output_store_path: str | Path | None = None,
        overwrite_existing: bool = False,
    ) -> SeismicDataset:
        request: dict[str, Any] = {
            "schema_version": 1,
            "store_path": str(self.store_path),
            "overwrite_existing": overwrite_existing,
            "pipeline": pipeline.to_payload(),
        }
        if output_store_path is not None:
            request["output_store_path"] = str(Path(output_store_path))
        payload = self.app.run_json(
            "run-subvolume-processing",
            "-",
            stdin_text=json.dumps(request),
        )
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self.app)

    def preview_gather(
        self,
        gather: GatherSelection,
        pipeline: GatherPipeline,
    ) -> GatherPreview:
        request = {
            "schema_version": 1,
            "store_path": str(self.store_path),
            "gather": gather.to_payload(dataset_id=self.descriptor.id),
            "pipeline": pipeline.to_payload(),
        }
        payload = self.app.run_json(
            "preview-gather-processing",
            "-",
            stdin_text=json.dumps(request),
        )
        return GatherPreview.from_json(_mapping(payload, "preview"), pipeline=pipeline)

    def run_gather(
        self,
        pipeline: GatherPipeline,
        *,
        output_store_path: str | Path | None = None,
        overwrite_existing: bool = False,
    ) -> SeismicDataset:
        request: dict[str, Any] = {
            "schema_version": 1,
            "store_path": str(self.store_path),
            "overwrite_existing": overwrite_existing,
            "pipeline": pipeline.to_payload(),
        }
        if output_store_path is not None:
            request["output_store_path"] = str(Path(output_store_path))
        payload = self.app.run_json(
            "run-gather-processing",
            "-",
            stdin_text=json.dumps(request),
        )
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self.app)

    def velocity_scan(
        self,
        gather: GatherSelection,
        spec: VelocityScanSpec,
    ) -> VelocityScanResult:
        payload = self.app.run_json(
            "run-velocity-scan",
            "-",
            stdin_text=json.dumps(
                spec.to_payload(
                    store_path=self.store_path,
                    gather=gather,
                    dataset_id=self.descriptor.id,
                )
            ),
        )
        return VelocityScanResult.from_json(_mapping(payload, "velocity_scan"))

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class TraceBoostApp:
    """Compatibility app shell for direct .tbvol and .tbgath workflows."""

    repo_root: Path = REPO_ROOT
    manifest_path: Path = DEFAULT_MANIFEST_PATH
    binary: str | None = None

    def command_prefix(self) -> list[str]:
        binary = self.binary or os.environ.get("TRACEBOOST_APP_BIN")
        if binary:
            return [binary]

        built_binary = self.repo_root / "target" / "debug" / "traceboost-app"
        if built_binary.exists():
            return [str(built_binary)]

        return [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(self.manifest_path),
            "-p",
            "traceboost-app",
            "--",
        ]

    def run_json(self, *args: str, stdin_text: str | None = None) -> Any:
        command = [*self.command_prefix(), *args]
        completed = subprocess.run(
            command,
            cwd=self.repo_root,
            input=stdin_text,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if completed.returncode != 0:
            raise TraceBoostCommandError(
                f"traceboost-app exited with status {completed.returncode}",
                command=command,
                stderr=completed.stderr.strip() or None,
            )
        stdout = completed.stdout.strip()
        if not stdout:
            return None
        try:
            return json.loads(stdout)
        except json.JSONDecodeError as exc:
            raise TraceBoostCommandError(
                "traceboost-app did not return valid JSON",
                command=command,
                stderr=completed.stderr.strip() or stdout,
            ) from exc

    def operation_catalog(self) -> Any:
        return self.run_json("operation-catalog")

    def dataset_operator_catalog(self, store_path: str | Path) -> OperatorCatalog:
        payload = self.run_json("dataset-operator-catalog", str(Path(store_path)))
        return OperatorCatalog.from_json(payload)

    def preflight_import(self, input_path: str | Path) -> SegyPreflight:
        payload = self.run_json("preflight-import", str(Path(input_path)))
        return SegyPreflight.from_json(_mapping(payload, "preflight"))

    def import_segy(
        self,
        input_path: str | Path,
        output_store_path: str | Path,
        *,
        overwrite_existing: bool = False,
        geometry_override: SegyGeometryOverride | None = None,
        preflight: SegyPreflight | None = None,
    ) -> SeismicDataset:
        resolved_geometry = geometry_override
        if resolved_geometry is None:
            effective_preflight = preflight or self.preflight_import(input_path)
            resolved_geometry = effective_preflight.resolved_geometry

        args = [
            "import-dataset",
            str(Path(input_path)),
            str(Path(output_store_path)),
        ]
        if overwrite_existing:
            args.append("--overwrite-existing")
        if resolved_geometry is not None:
            args.extend(resolved_geometry.to_cli_args())

        payload = self.run_json(*args)
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self)

    def import_prestack_offset_dataset(
        self,
        input_path: str | Path,
        output_store_path: str | Path,
        *,
        overwrite_existing: bool = False,
    ) -> SeismicDataset:
        args = [
            "import-prestack-offset-dataset",
            str(Path(input_path)),
            str(Path(output_store_path)),
        ]
        if overwrite_existing:
            args.append("--overwrite-existing")
        payload = self.run_json(*args)
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self)

    def open_dataset(self, store_path: str | Path) -> SeismicDataset:
        payload = self.run_json("open-dataset", str(Path(store_path)))
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self)

    def run_processing_batch_compatibility(
        self,
        store_paths: Sequence[str | Path],
        pipeline: TraceProcessingPipeline,
        *,
        output_store_paths: Sequence[str | Path | None] | None = None,
        overwrite_existing: bool = False,
        max_workers: int | None = None,
    ) -> ProcessingBatchResult:
        """Run a synchronous loose-store batch via a local thread pool.

        This compatibility helper shells out to `traceboost-app` per dataset. It is not the
        shared desktop job service and does not support submit/poll/cancel across calls.
        """
        resolved_store_paths = tuple(Path(path) for path in store_paths)
        if not resolved_store_paths:
            return ProcessingBatchResult(items=())

        if output_store_paths is None:
            resolved_output_store_paths = tuple(None for _ in resolved_store_paths)
        else:
            if len(output_store_paths) != len(resolved_store_paths):
                raise ValueError("output_store_paths must match store_paths length")
            resolved_output_store_paths = tuple(
                None if path is None else Path(path) for path in output_store_paths
            )

        worker_count = max_workers or max(1, (os.cpu_count() or 4) - 1)
        worker_count = max(1, min(worker_count, len(resolved_store_paths)))
        results: list[ProcessingBatchItemResult | None] = [None] * len(resolved_store_paths)

        def _run_one(index: int) -> tuple[int, ProcessingBatchItemResult]:
            store_path = resolved_store_paths[index]
            output_store_path = resolved_output_store_paths[index]
            try:
                dataset = self.open_dataset(store_path)
                processed = dataset.run_processing(
                    pipeline,
                    output_store_path=output_store_path,
                    overwrite_existing=overwrite_existing,
                )
                return index, ProcessingBatchItemResult(
                    store_path=store_path,
                    output_store_path=output_store_path,
                    dataset=processed,
                    error=None,
                )
            except Exception as exc:
                return index, ProcessingBatchItemResult(
                    store_path=store_path,
                    output_store_path=output_store_path,
                    dataset=None,
                    error=str(exc),
                )

        with ThreadPoolExecutor(max_workers=worker_count) as executor:
            futures = [executor.submit(_run_one, index) for index in range(len(resolved_store_paths))]
            for future in as_completed(futures):
                index, item = future.result()
                results[index] = item

        return ProcessingBatchResult(items=tuple(item for item in results if item is not None))

    def run_processing_batch(
        self,
        store_paths: Sequence[str | Path],
        pipeline: TraceProcessingPipeline,
        *,
        output_store_paths: Sequence[str | Path | None] | None = None,
        overwrite_existing: bool = False,
        max_workers: int | None = None,
    ) -> ProcessingBatchResult:
        return self.run_processing_batch_compatibility(
            store_paths,
            pipeline,
            output_store_paths=output_store_paths,
            overwrite_existing=overwrite_existing,
            max_workers=max_workers,
        )
