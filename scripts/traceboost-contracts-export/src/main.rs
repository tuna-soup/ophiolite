use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::schema_for;
use ts_rs::TS;

macro_rules! public_contracts {
    ($callback:ident, $($args:tt)*) => {
        $callback! {
            $($args)*
            {
                "DatasetId" => seis_contracts_core::DatasetId,
                "AxisSummaryF32" => seis_contracts_core::AxisSummaryF32,
                "AxisSummaryI32" => seis_contracts_core::AxisSummaryI32,
                "GeometryDescriptor" => seis_contracts_core::GeometryDescriptor,
                "GeometryProvenanceSummary" => seis_contracts_core::GeometryProvenanceSummary,
                "GeometrySummary" => seis_contracts_core::GeometrySummary,
                "ProcessingArtifactRole" => seis_contracts_core::ProcessingArtifactRole,
                "ProcessingLineageSummary" => seis_contracts_core::ProcessingLineageSummary,
                "SampleDataConversionKind" => seis_contracts_core::SampleDataConversionKind,
                "SampleDataFidelity" => seis_contracts_core::SampleDataFidelity,
                "SampleValuePreservation" => seis_contracts_core::SampleValuePreservation,
                "VolumeDescriptor" => seis_contracts_core::VolumeDescriptor,
                "SectionAxis" => seis_contracts_core::SectionAxis,
                "SectionRequest" => seis_contracts_core::SectionRequest,
                "GatherRequest" => seis_contracts_core::GatherRequest,
                "GatherSelector" => seis_contracts_core::GatherSelector,
                "SectionTileRequest" => seis_contracts_core::SectionTileRequest,
                "FrequencyPhaseMode" => seis_contracts_core::FrequencyPhaseMode,
                "FrequencyWindowShape" => seis_contracts_core::FrequencyWindowShape,
                "VelocityFunctionSource" => seis_contracts_core::VelocityFunctionSource,
                "VelocityQuantityKind" => seis_contracts_core::VelocityQuantityKind,
                "GatherInterpolationMode" => seis_contracts_core::GatherInterpolationMode,
                "SectionSpectrumSelection" => seis_contracts_core::SectionSpectrumSelection,
                "AmplitudeSpectrumCurve" => seis_contracts_core::AmplitudeSpectrumCurve,
                "AmplitudeSpectrumRequest" => seis_contracts_core::AmplitudeSpectrumRequest,
                "AmplitudeSpectrumResponse" => seis_contracts_core::AmplitudeSpectrumResponse,
                "TraceLocalProcessingOperation" => seis_contracts_core::TraceLocalProcessingOperation,
                "TraceLocalProcessingPipeline" => seis_contracts_core::TraceLocalProcessingPipeline,
                "TraceLocalProcessingStep" => seis_contracts_core::TraceLocalProcessingStep,
                "SubvolumeCropOperation" => seis_contracts_core::processing::SubvolumeCropOperation,
                "SubvolumeProcessingPipeline" => seis_contracts_core::processing::SubvolumeProcessingPipeline,
                "TraceLocalVolumeArithmeticOperator" => seis_contracts_core::TraceLocalVolumeArithmeticOperator,
                "GatherProcessingOperation" => seis_contracts_core::GatherProcessingOperation,
                "GatherProcessingPipeline" => seis_contracts_core::GatherProcessingPipeline,
                "ProcessingPipelineFamily" => seis_contracts_core::ProcessingPipelineFamily,
                "ProcessingPipelineSpec" => seis_contracts_core::ProcessingPipelineSpec,
                "ProcessingJobState" => seis_contracts_core::ProcessingJobState,
                "ProcessingJobProgress" => seis_contracts_core::ProcessingJobProgress,
                "ProcessingJobArtifactKind" => seis_contracts_core::ProcessingJobArtifactKind,
                "ProcessingJobArtifact" => seis_contracts_core::ProcessingJobArtifact,
                "ProcessingJobStatus" => seis_contracts_core::ProcessingJobStatus,
                "TraceLocalProcessingPreset" => seis_contracts_core::TraceLocalProcessingPreset,
                "InterpretationPoint" => seis_contracts_core::InterpretationPoint,
                "SectionColorMap" => seis_contracts_core::views::SectionColorMap,
                "SectionRenderMode" => seis_contracts_core::views::SectionRenderMode,
                "SectionPolarity" => seis_contracts_core::views::SectionPolarity,
                "SectionPrimaryMode" => seis_contracts_core::views::SectionPrimaryMode,
                "SectionCoordinate" => seis_contracts_core::views::SectionCoordinate,
                "SectionUnits" => seis_contracts_core::views::SectionUnits,
                "SectionMetadata" => seis_contracts_core::views::SectionMetadata,
                "SectionDisplayDefaults" => seis_contracts_core::views::SectionDisplayDefaults,
                "SectionView" => seis_contracts_core::views::SectionView,
                "SectionTimeDepthTransformMode" => seis_contracts_core::views::SectionTimeDepthTransformMode,
                "SectionTimeDepthDiagnostics" => seis_contracts_core::views::SectionTimeDepthDiagnostics,
                "SectionScalarOverlayColorMap" => seis_contracts_core::views::SectionScalarOverlayColorMap,
                "SectionScalarOverlayValueRange" => seis_contracts_core::views::SectionScalarOverlayValueRange,
                "SectionScalarOverlayView" => seis_contracts_core::views::SectionScalarOverlayView,
                "SectionHorizonLineStyle" => seis_contracts_core::views::SectionHorizonLineStyle,
                "SectionHorizonStyle" => seis_contracts_core::views::SectionHorizonStyle,
                "SectionHorizonSample" => seis_contracts_core::views::SectionHorizonSample,
                "SectionHorizonOverlayView" => seis_contracts_core::views::SectionHorizonOverlayView,
                "ResolvedSectionDisplayView" => seis_contracts_core::views::ResolvedSectionDisplayView,
                "GatherView" => seis_contracts_core::views::GatherView,
                "PreviewView" => seis_contracts_core::views::PreviewView,
                "ProjectSurveyMapRequestDto" => ophiolite_project::ProjectSurveyMapRequestDto,
                "GatherPreviewView" => seis_contracts_core::views::GatherPreviewView,
                "SectionViewport" => seis_contracts_core::views::SectionViewport,
                "GatherViewport" => seis_contracts_core::views::GatherViewport,
                "SectionProbe" => seis_contracts_core::views::SectionProbe,
                "GatherProbe" => seis_contracts_core::views::GatherProbe,
                "SectionProbeChanged" => seis_contracts_core::views::SectionProbeChanged,
                "GatherProbeChanged" => seis_contracts_core::views::GatherProbeChanged,
                "SectionViewportChanged" => seis_contracts_core::views::SectionViewportChanged,
                "GatherViewportChanged" => seis_contracts_core::views::GatherViewportChanged,
                "SectionInteractionChanged" => seis_contracts_core::views::SectionInteractionChanged,
                "SemblancePanel" => seis_contracts_core::SemblancePanel,
                "VelocityScanRequest" => seis_contracts_core::VelocityScanRequest,
                "VelocityScanResponse" => seis_contracts_core::VelocityScanResponse,
                "SegyHeaderValueType" => seis_contracts_operations::SegyHeaderValueType,
                "SegyHeaderField" => seis_contracts_operations::SegyHeaderField,
                "SegyGeometryOverride" => seis_contracts_operations::SegyGeometryOverride,
                "SegyGeometryCandidate" => seis_contracts_operations::SegyGeometryCandidate,
                "SuggestedImportAction" => seis_contracts_operations::SuggestedImportAction,
                "DatasetSummary" => seis_contracts_operations::DatasetSummary,
                "SurveyPreflightRequest" => seis_contracts_operations::SurveyPreflightRequest,
                "SurveyPreflightResponse" => seis_contracts_operations::SurveyPreflightResponse,
                "SegyImportWizardStage" => seis_contracts_operations::SegyImportWizardStage,
                "SegyImportIssueSeverity" => seis_contracts_operations::SegyImportIssueSeverity,
                "SegyImportIssueSection" => seis_contracts_operations::SegyImportIssueSection,
                "SegyImportSparseHandling" => seis_contracts_operations::SegyImportSparseHandling,
                "SegyImportPlanSource" => seis_contracts_operations::SegyImportPlanSource,
                "SegyImportRecipeScope" => seis_contracts_operations::SegyImportRecipeScope,
                "SegyImportPolicy" => seis_contracts_operations::SegyImportPolicy,
                "SegyImportSpatialPlan" => seis_contracts_operations::SegyImportSpatialPlan,
                "SegyImportProvenance" => seis_contracts_operations::SegyImportProvenance,
                "SegyImportPlan" => seis_contracts_operations::SegyImportPlan,
                "SegyImportIssue" => seis_contracts_operations::SegyImportIssue,
                "SegyImportRiskSummary" => seis_contracts_operations::SegyImportRiskSummary,
                "SegyImportResolvedDataset" => seis_contracts_operations::SegyImportResolvedDataset,
                "SegyImportResolvedSpatial" => seis_contracts_operations::SegyImportResolvedSpatial,
                "SegyImportFieldObservation" => seis_contracts_operations::SegyImportFieldObservation,
                "SegyImportCandidatePlan" => seis_contracts_operations::SegyImportCandidatePlan,
                "ScanSegyImportRequest" => seis_contracts_operations::ScanSegyImportRequest,
                "SegyImportScanResponse" => seis_contracts_operations::SegyImportScanResponse,
                "ValidateSegyImportPlanRequest" => seis_contracts_operations::ValidateSegyImportPlanRequest,
                "SegyImportValidationResponse" => seis_contracts_operations::SegyImportValidationResponse,
                "ImportSegyWithPlanRequest" => seis_contracts_operations::ImportSegyWithPlanRequest,
                "ImportSegyWithPlanResponse" => seis_contracts_operations::ImportSegyWithPlanResponse,
                "SegyImportRecipe" => seis_contracts_operations::SegyImportRecipe,
                "ListSegyImportRecipesRequest" => seis_contracts_operations::ListSegyImportRecipesRequest,
                "ListSegyImportRecipesResponse" => seis_contracts_operations::ListSegyImportRecipesResponse,
                "SaveSegyImportRecipeRequest" => seis_contracts_operations::SaveSegyImportRecipeRequest,
                "SaveSegyImportRecipeResponse" => seis_contracts_operations::SaveSegyImportRecipeResponse,
                "DeleteSegyImportRecipeRequest" => seis_contracts_operations::DeleteSegyImportRecipeRequest,
                "DeleteSegyImportRecipeResponse" => seis_contracts_operations::DeleteSegyImportRecipeResponse,
                "ImportDatasetRequest" => seis_contracts_operations::ImportDatasetRequest,
                "ImportDatasetResponse" => seis_contracts_operations::ImportDatasetResponse,
                "ExportSegyRequest" => seis_contracts_operations::ExportSegyRequest,
                "ExportSegyResponse" => seis_contracts_operations::ExportSegyResponse,
                "ImportedHorizonDescriptor" => seis_contracts_core::ImportedHorizonDescriptor,
                "ImportHorizonXyzRequest" => seis_contracts_operations::ImportHorizonXyzRequest,
                "ImportHorizonXyzResponse" => seis_contracts_operations::ImportHorizonXyzResponse,
                "LoadSectionHorizonsRequest" => seis_contracts_operations::LoadSectionHorizonsRequest,
                "LoadSectionHorizonsResponse" => seis_contracts_operations::LoadSectionHorizonsResponse,
                "OpenDatasetRequest" => seis_contracts_operations::OpenDatasetRequest,
                "OpenDatasetResponse" => seis_contracts_operations::OpenDatasetResponse,
                "PreviewCommand" => seis_contracts_operations::PreviewCommand,
                "PreviewResponse" => seis_contracts_operations::PreviewResponse,
                "PreviewTraceLocalProcessingRequest" => seis_contracts_operations::PreviewTraceLocalProcessingRequest,
                "PreviewTraceLocalProcessingResponse" => seis_contracts_operations::PreviewTraceLocalProcessingResponse,
                "PreviewSubvolumeProcessingRequest" => seis_contracts_operations::PreviewSubvolumeProcessingRequest,
                "PreviewSubvolumeProcessingResponse" => seis_contracts_operations::PreviewSubvolumeProcessingResponse,
                "RunTraceLocalProcessingRequest" => seis_contracts_operations::RunTraceLocalProcessingRequest,
                "RunTraceLocalProcessingResponse" => seis_contracts_operations::RunTraceLocalProcessingResponse,
                "RunSubvolumeProcessingRequest" => seis_contracts_operations::RunSubvolumeProcessingRequest,
                "RunSubvolumeProcessingResponse" => seis_contracts_operations::RunSubvolumeProcessingResponse,
                "PreviewGatherProcessingRequest" => seis_contracts_operations::PreviewGatherProcessingRequest,
                "PreviewGatherProcessingResponse" => seis_contracts_operations::PreviewGatherProcessingResponse,
                "RunGatherProcessingRequest" => seis_contracts_operations::RunGatherProcessingRequest,
                "RunGatherProcessingResponse" => seis_contracts_operations::RunGatherProcessingResponse,
                "GetProcessingJobRequest" => seis_contracts_operations::GetProcessingJobRequest,
                "GetProcessingJobResponse" => seis_contracts_operations::GetProcessingJobResponse,
                "CancelProcessingJobRequest" => seis_contracts_operations::CancelProcessingJobRequest,
                "CancelProcessingJobResponse" => seis_contracts_operations::CancelProcessingJobResponse,
                "ListPipelinePresetsResponse" => seis_contracts_operations::ListPipelinePresetsResponse,
                "SavePipelinePresetRequest" => seis_contracts_operations::SavePipelinePresetRequest,
                "SavePipelinePresetResponse" => seis_contracts_operations::SavePipelinePresetResponse,
                "DeletePipelinePresetRequest" => seis_contracts_operations::DeletePipelinePresetRequest,
                "DeletePipelinePresetResponse" => seis_contracts_operations::DeletePipelinePresetResponse,
                "DatasetRegistryStatus" => seis_contracts_operations::DatasetRegistryStatus,
                "WorkspacePipelineEntry" => seis_contracts_operations::WorkspacePipelineEntry,
                "DatasetRegistryEntry" => seis_contracts_operations::DatasetRegistryEntry,
                "WorkspaceSession" => seis_contracts_operations::WorkspaceSession,
                "LoadWorkspaceStateResponse" => seis_contracts_operations::LoadWorkspaceStateResponse,
                "UpsertDatasetEntryRequest" => seis_contracts_operations::UpsertDatasetEntryRequest,
                "UpsertDatasetEntryResponse" => seis_contracts_operations::UpsertDatasetEntryResponse,
                "RemoveDatasetEntryRequest" => seis_contracts_operations::RemoveDatasetEntryRequest,
                "RemoveDatasetEntryResponse" => seis_contracts_operations::RemoveDatasetEntryResponse,
                "SetActiveDatasetEntryRequest" => seis_contracts_operations::SetActiveDatasetEntryRequest,
                "SetActiveDatasetEntryResponse" => seis_contracts_operations::SetActiveDatasetEntryResponse,
                "SaveWorkspaceSessionRequest" => seis_contracts_operations::SaveWorkspaceSessionRequest,
                "SaveWorkspaceSessionResponse" => seis_contracts_operations::SaveWorkspaceSessionResponse,
                "DescribeVelocityVolumeRequest" => seis_contracts_operations::DescribeVelocityVolumeRequest,
                "DescribeVelocityVolumeResponse" => seis_contracts_operations::DescribeVelocityVolumeResponse,
                "IngestVelocityVolumeRequest" => seis_contracts_operations::IngestVelocityVolumeRequest,
                "IngestVelocityVolumeResponse" => seis_contracts_operations::IngestVelocityVolumeResponse,
                "SetDatasetNativeCoordinateReferenceRequest" => seis_contracts_operations::SetDatasetNativeCoordinateReferenceRequest,
                "SetDatasetNativeCoordinateReferenceResponse" => seis_contracts_operations::SetDatasetNativeCoordinateReferenceResponse,
                "ResolvedSurveyMapSourceDto" => seis_contracts_operations::ResolvedSurveyMapSourceDto,
                "ResolveSurveyMapRequest" => seis_contracts_operations::ResolveSurveyMapRequest,
                "ResolveSurveyMapResponse" => seis_contracts_operations::ResolveSurveyMapResponse,
                "BuildSurveyTimeDepthTransformRequest" => seis_contracts_operations::BuildSurveyTimeDepthTransformRequest,
                "LayeredVelocityModel" => seis_contracts_operations::LayeredVelocityModel,
                "LayeredVelocityInterval" => seis_contracts_operations::LayeredVelocityInterval,
                "VelocityIntervalTrend" => seis_contracts_operations::VelocityIntervalTrend,
                "StratigraphicBoundaryReference" => seis_contracts_operations::StratigraphicBoundaryReference,
                "LateralInterpolationMethod" => seis_contracts_operations::LateralInterpolationMethod,
                "VerticalInterpolationMethod" => seis_contracts_operations::VerticalInterpolationMethod,
                "TimeDepthDomain" => seis_contracts_operations::TimeDepthDomain,
                "TravelTimeReference" => seis_contracts_operations::TravelTimeReference,
                "DepthReferenceKind" => seis_contracts_operations::DepthReferenceKind,
                "SurveyTimeDepthTransform3D" => seis_contracts_operations::SurveyTimeDepthTransform3D,
                "LoadVelocityModelsRequest" => seis_contracts_operations::LoadVelocityModelsRequest,
                "LoadVelocityModelsResponse" => seis_contracts_operations::LoadVelocityModelsResponse,
            }
        }
    };
}

macro_rules! wrapper_files {
    ($callback:ident, $($args:tt)*) => {
        $callback! {
            $($args)*
            {
                "BuildSurveyTimeDepthTransformRequest" => seis_contracts_operations::BuildSurveyTimeDepthTransformRequest,
                "CoordinateReferenceBindingDto" => seis_contracts_operations::CoordinateReferenceBindingDto,
                "CoordinateReferenceDto" => seis_contracts_operations::CoordinateReferenceDto,
                "CoordinateReferenceSourceDto" => seis_contracts_operations::CoordinateReferenceSourceDto,
                "DepthReferenceKind" => seis_contracts_operations::DepthReferenceKind,
                "GatherAxisKind" => seis_contracts_core::domain::GatherAxisKind,
                "GatherInteractionChanged" => seis_contracts_core::views::GatherInteractionChanged,
                "GatherPreviewView" => seis_contracts_core::views::GatherPreviewView,
                "GatherProbe" => seis_contracts_core::views::GatherProbe,
                "GatherProbeChanged" => seis_contracts_core::views::GatherProbeChanged,
                "GatherSampleDomain" => seis_contracts_core::domain::GatherSampleDomain,
                "GatherView" => seis_contracts_core::views::GatherView,
                "GatherViewport" => seis_contracts_core::views::GatherViewport,
                "GatherViewportChanged" => seis_contracts_core::views::GatherViewportChanged,
                "ImportedHorizonDescriptor" => seis_contracts_core::ImportedHorizonDescriptor,
                "LateralInterpolationMethod" => seis_contracts_operations::LateralInterpolationMethod,
                "LayeredVelocityInterval" => seis_contracts_operations::LayeredVelocityInterval,
                "LayeredVelocityModel" => seis_contracts_operations::LayeredVelocityModel,
                "PreviewView" => seis_contracts_core::views::PreviewView,
                "ProjectSurveyMapRequestDto" => ophiolite_project::ProjectSurveyMapRequestDto,
                "ProjectedPoint2Dto" => seis_contracts_operations::ProjectedPoint2Dto,
                "ProjectedPolygon2Dto" => seis_contracts_operations::ProjectedPolygon2Dto,
                "ProjectedVector2Dto" => seis_contracts_operations::ProjectedVector2Dto,
                "ResolvedSectionDisplayView" => seis_contracts_core::views::ResolvedSectionDisplayView,
                "ResolvedSurveyMapSourceDto" => seis_contracts_operations::ResolvedSurveyMapSourceDto,
                "ResolvedSurveyMapSurveyDto" => seis_contracts_operations::ResolvedSurveyMapSurveyDto,
                "ResolvedSurveyMapWellDto" => seis_contracts_operations::ResolvedSurveyMapWellDto,
                "SectionColorMap" => seis_contracts_core::views::SectionColorMap,
                "SectionCoordinate" => seis_contracts_core::views::SectionCoordinate,
                "SectionDisplayDefaults" => seis_contracts_core::views::SectionDisplayDefaults,
                "SectionHorizonLineStyle" => seis_contracts_core::views::SectionHorizonLineStyle,
                "SectionHorizonOverlayView" => seis_contracts_core::views::SectionHorizonOverlayView,
                "SectionHorizonSample" => seis_contracts_core::views::SectionHorizonSample,
                "SectionHorizonStyle" => seis_contracts_core::views::SectionHorizonStyle,
                "SectionInteractionChanged" => seis_contracts_core::views::SectionInteractionChanged,
                "SectionMetadata" => seis_contracts_core::views::SectionMetadata,
                "SectionPolarity" => seis_contracts_core::views::SectionPolarity,
                "SectionPrimaryMode" => seis_contracts_core::views::SectionPrimaryMode,
                "SectionProbe" => seis_contracts_core::views::SectionProbe,
                "SectionProbeChanged" => seis_contracts_core::views::SectionProbeChanged,
                "SectionRenderMode" => seis_contracts_core::views::SectionRenderMode,
                "SectionScalarOverlayColorMap" => seis_contracts_core::views::SectionScalarOverlayColorMap,
                "SectionScalarOverlayValueRange" => seis_contracts_core::views::SectionScalarOverlayValueRange,
                "SectionScalarOverlayView" => seis_contracts_core::views::SectionScalarOverlayView,
                "SectionTimeDepthDiagnostics" => seis_contracts_core::views::SectionTimeDepthDiagnostics,
                "SectionTimeDepthTransformMode" => seis_contracts_core::views::SectionTimeDepthTransformMode,
                "SectionUnits" => seis_contracts_core::views::SectionUnits,
                "SectionView" => seis_contracts_core::views::SectionView,
                "SectionViewport" => seis_contracts_core::views::SectionViewport,
                "SectionViewportChanged" => seis_contracts_core::views::SectionViewportChanged,
                "StratigraphicBoundaryReference" => seis_contracts_operations::StratigraphicBoundaryReference,
                "SurveyIndexAxisDto" => seis_contracts_operations::SurveyIndexAxisDto,
                "SurveyIndexGridDto" => seis_contracts_operations::SurveyIndexGridDto,
                "SurveyMapGridTransformDto" => seis_contracts_operations::SurveyMapGridTransformDto,
                "SurveyMapSpatialAvailabilityDto" => seis_contracts_operations::SurveyMapSpatialAvailabilityDto,
                "SurveyMapSpatialDescriptorDto" => seis_contracts_operations::SurveyMapSpatialDescriptorDto,
                "SurveyMapTrajectoryDto" => seis_contracts_operations::SurveyMapTrajectoryDto,
                "SurveyMapTrajectoryStationDto" => seis_contracts_operations::SurveyMapTrajectoryStationDto,
                "SurveyTimeDepthTransform3D" => seis_contracts_operations::SurveyTimeDepthTransform3D,
                "TimeDepthDomain" => seis_contracts_operations::TimeDepthDomain,
                "TravelTimeReference" => seis_contracts_operations::TravelTimeReference,
                "VelocityIntervalTrend" => seis_contracts_operations::VelocityIntervalTrend,
                "VelocityQuantityKind" => seis_contracts_core::VelocityQuantityKind,
                "VerticalInterpolationMethod" => seis_contracts_operations::VerticalInterpolationMethod,
            }
        }
    };
}

macro_rules! export_types {
    ($output_dir:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( <$ty as TS>::export_all_to($output_dir)?; )*
    }};
}

macro_rules! write_index_lines {
    ($buffer:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( $buffer.push_str(&format!("export type {{ {0} }} from \"./{0}\";\n", $name)); )*
    }};
}

macro_rules! insert_schema_entries {
    ($types:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( $types.insert($name.to_string(), serde_json::to_value(schema_for!($ty))?); )*
    }};
}

macro_rules! write_wrapper_files {
    ($output_dir:expr, { $( $name:literal => $ty:ty, )* }) => {{
        $( write_wrapper_file($output_dir, $name)?; )*
    }};
}

fn main() -> Result<(), Box<dyn Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("scripts/traceboost-contracts-export should live two levels under repo root")
        .to_path_buf();

    let package_root = repo_root
        .join("traceboost")
        .join("contracts")
        .join("ts")
        .join("seis-contracts");
    let generated_dir = package_root.join("src").join("generated");
    let schema_dir = package_root.join("schemas");

    fs::create_dir_all(&generated_dir)?;
    fs::create_dir_all(&schema_dir)?;

    clear_generated_ts(&generated_dir)?;
    export_ts_types(&generated_dir)?;
    write_generated_index(&generated_dir)?;
    write_schema_bundle(&schema_dir)?;

    Ok(())
}

fn clear_generated_ts(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("ts") {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn export_ts_types(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    public_contracts!(export_types, output_dir,);
    wrapper_files!(write_wrapper_files, output_dir,);

    fs::write(
        output_dir.join("ipc-schema-version.ts"),
        format!(
            "// Generated by `cargo run -p traceboost-contracts-export`\nexport const IPC_SCHEMA_VERSION = {} as const;\n",
            seis_contracts_operations::IPC_SCHEMA_VERSION
        ),
    )?;

    Ok(())
}

fn write_wrapper_file(output_dir: &Path, type_name: &str) -> Result<(), Box<dyn Error>> {
    fs::write(
        output_dir.join(format!("{type_name}.ts")),
        format!(
            "// Generated by `cargo run -p traceboost-contracts-export`\nexport type {{ {type_name} }} from \"@ophiolite/contracts\";\n"
        ),
    )?;

    Ok(())
}

fn write_generated_index(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut index = String::from("// Generated by `cargo run -p traceboost-contracts-export`\n");
    public_contracts!(write_index_lines, index,);
    index.push_str("export { IPC_SCHEMA_VERSION } from \"./ipc-schema-version\";\n");
    fs::write(output_dir.join("index.ts"), index)?;
    Ok(())
}

fn write_schema_bundle(schema_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut types = BTreeMap::new();
    public_contracts!(insert_schema_entries, types,);

    let schema = serde_json::json!({
        "ipcSchemaVersion": seis_contracts_operations::IPC_SCHEMA_VERSION,
        "types": types,
    });

    fs::write(
        schema_dir.join("seis-contracts.schema.json"),
        serde_json::to_string_pretty(&schema)?,
    )?;

    Ok(())
}
