#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]

AUDITED_STRUCTS: dict[Path, set[str]] = {
    Path("traceboost/contracts/seis-contracts-operations/src/workspace.rs"): {
        "WorkspacePipelineEntry",
    },
    Path("crates/ophiolite-seismic/src/contracts/domain.rs"): {
        "SampleDataFidelity",
    },
    Path("crates/ophiolite-seismic/src/contracts/models.rs"): {
        "SpatialCoverageSummary",
        "VelocitySource3D",
        "SurveyPropertyField3D",
        "CheckshotVspObservationSet1D",
        "ManualTimeDepthPickSet1D",
        "WellTieObservationSet1D",
        "WellTimeDepthSourceBinding",
        "WellTimeDepthAssumptionInterval",
        "WellTimeDepthAuthoredModel1D",
        "CompiledWellTimeDepthLineage",
        "WellTimeDepthModel1D",
        "SurveyTimeDepthTransform3D",
    },
    Path("crates/ophiolite-seismic/src/contracts/processing.rs"): {
        "TraceLocalProcessingPipeline",
        "SubvolumeProcessingPipeline",
        "PostStackNeighborhoodProcessingPipeline",
        "GatherProcessingPipeline",
        "ProcessingJobPlanSummary",
        "ProcessingJobStageExecutionSummary",
        "ProcessingJobExecutionSummary",
        "ProcessingJobStatus",
        "ProcessingBatchItemStatus",
        "ProcessingBatchStatus",
    },
    Path("crates/ophiolite-seismic/src/contracts/views.rs"): {
        "SectionTimeDepthDiagnostics",
        "SectionScalarOverlayView",
        "SectionHorizonStyle",
        "SectionHorizonSample",
        "SectionHorizonOverlayView",
        "ResolvedSectionDisplayView",
        "ImportedHorizonDescriptor",
    },
    Path("crates/ophiolite-seismic/src/contracts/operations.rs"): {
        "AmplitudeSpectrumResponse",
        "WellTieLogCurveSource",
        "WellTieCurve1D",
        "WellTieAnalysis1D",
        "AvoReflectivityResponse",
        "RockPhysicsAttributeResponse",
        "AvoInterceptGradientAttributeResponse",
        "SegyGeometryOverride",
        "SegyGeometryCandidate",
        "SurveyPreflightResponse",
        "SegyImportSpatialPlan",
        "SegyImportProvenance",
        "SegyImportIssue",
        "SegyImportResolvedDataset",
        "SegyImportResolvedSpatial",
        "SegyImportFieldObservation",
        "SegyImportCandidatePlan",
        "SegyImportScanResponse",
        "SegyImportValidationResponse",
        "SegyImportRecipe",
        "VelocityScanResponse",
    },
    Path("src/project_contracts.rs"): {
        "SectionWellOverlaySampleDto",
        "SectionWellOverlaySegmentDto",
        "ResolvedSectionWellOverlayDto",
        "ResolveSectionWellOverlaysResponse",
    },
}

FORBIDDEN_PATTERNS = ("skip_serializing_if", "#[ts(optional_fields")


def audit_file(relative_path: Path, audited_structs: set[str]) -> list[str]:
    path = ROOT / relative_path
    lines = path.read_text(encoding="utf-8").splitlines()
    violations: list[str] = []
    current_struct: str | None = None
    brace_depth = 0

    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        if current_struct is None:
            for struct_name in audited_structs:
                marker = f"pub struct {struct_name} {{"
                if stripped == marker:
                    current_struct = struct_name
                    brace_depth = line.count("{") - line.count("}")
                    break
            continue

        if any(pattern in stripped for pattern in FORBIDDEN_PATTERNS):
            violations.append(
                f"{relative_path}:{index}: forbidden omission helper in {current_struct}: {stripped}"
            )

        brace_depth += line.count("{") - line.count("}")
        if brace_depth <= 0:
            current_struct = None
            brace_depth = 0

    return violations


def main() -> int:
    violations: list[str] = []
    for relative_path, audited_structs in AUDITED_STRUCTS.items():
        violations.extend(audit_file(relative_path, audited_structs))

    if violations:
        print("Contract presence-policy audit failed:", file=sys.stderr)
        for violation in violations:
            print(f"  {violation}", file=sys.stderr)
        return 1

    audited_count = sum(len(items) for items in AUDITED_STRUCTS.values())
    print(f"Presence-policy audit passed for {audited_count} structs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
