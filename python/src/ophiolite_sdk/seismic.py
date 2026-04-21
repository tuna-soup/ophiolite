from __future__ import annotations

import json
import math
import os
import struct
import subprocess
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Any, Mapping, Sequence


REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_MANIFEST_PATH = REPO_ROOT / "Cargo.toml"

__all__ = [
    "AmplitudeScalar",
    "BandpassFilter",
    "DatasetDescriptor",
    "HighpassFilter",
    "LowpassFilter",
    "PhaseRotation",
    "ProcessingPreview",
    "RmsAgc",
    "SectionSelection",
    "SectionView",
    "SeismicDataset",
    "SegyGeometryField",
    "SegyGeometryOverride",
    "SegyPreflight",
    "TraceBoostApp",
    "TraceBoostCommandError",
    "TraceProcessingPipeline",
    "TraceProcessingStep",
    "TraceRmsNormalize",
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


@dataclass(frozen=True)
class ProcessingPreview:
    section: SectionView
    processing_label: str
    preview_ready: bool
    pipeline: TraceProcessingPipeline
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
class SeismicDataset:
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

    def to_payload(self) -> Mapping[str, Any]:
        return self.raw_payload


@dataclass(frozen=True)
class TraceBoostApp:
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

    def open_dataset(self, store_path: str | Path) -> SeismicDataset:
        payload = self.run_json("open-dataset", str(Path(store_path)))
        return SeismicDataset.from_json(_mapping(payload, "dataset"), app=self)
