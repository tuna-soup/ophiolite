#![recursion_limit = "1024"]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use ophiolite::{
    AVO_ANALYSIS_CONTRACT_VERSION, AvoAnisotropyModeDto, AvoAxisDto, AvoBackgroundRegionDto,
    AvoChiProjectionSeriesDto, AvoCrossplotPointDto, AvoCurveStyleDto, AvoInterfaceDto,
    AvoReferenceLineDto, AvoReflectivityModelDto, AvoResponseSeriesDto,
    BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest,
    CheckshotVspObservationSet1D, CompiledWellTimeDepthLineage, CoordinateReferenceBindingDto,
    CoordinateReferenceDto, CoordinateReferenceSourceDto, DepthReferenceKind, GatherAxisKind,
    GatherInteractionChanged, GatherPreviewView, GatherProbe, GatherProbeChanged,
    GatherSampleDomain, GatherView, GatherViewport, GatherViewportChanged,
    ImportedHorizonDescriptor, LateralInterpolationMethod, LayeredVelocityInterval,
    LayeredVelocityModel, ManualTimeDepthPickSet1D, OperatorAvailability, OperatorCatalog,
    OperatorCatalogEntry, OperatorCatalogOutputLifecycle, OperatorCatalogStability,
    OperatorContractRef, OperatorDetail, OperatorDocumentation, OperatorExecutionKind,
    OperatorFamily, OperatorParameterDoc, OperatorSubjectKind, PreviewView,
    ProjectSurveyMapRequestDto, ProjectedPoint2Dto, ProjectedPolygon2Dto, ProjectedVector2Dto,
    ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION, ResolveSectionWellOverlaysResponse,
    ResolvedAvoChiProjectionSourceDto, ResolvedAvoCrossplotSourceDto, ResolvedAvoResponseSourceDto,
    ResolvedRockPhysicsCrossplotSourceDto, ResolvedSectionDisplayView,
    ResolvedSectionWellOverlayDto, ResolvedSurveyMapSourceDto, ResolvedSurveyMapSurveyDto,
    ResolvedSurveyMapWellDto, ResolvedTrajectoryGeometry, ResolvedTrajectoryStation,
    ResolvedWellPanelSourceDto, ResolvedWellPanelWellDto, RockPhysicsAxisDto,
    RockPhysicsCategoricalColorBindingDto, RockPhysicsCategoricalColorRequestDto,
    RockPhysicsCategoricalSemanticDto, RockPhysicsCategoryDto, RockPhysicsColorBindingDto,
    RockPhysicsColorRequestDto, RockPhysicsContinuousColorBindingDto,
    RockPhysicsContinuousColorRequestDto, RockPhysicsCrossplotRequestDto,
    RockPhysicsCurveSemanticDto, RockPhysicsInteractionThresholdsDto, RockPhysicsPointSymbolDto,
    RockPhysicsSampleDto, RockPhysicsSourceBindingDto, RockPhysicsTemplateIdDto,
    RockPhysicsTemplateLineDto, RockPhysicsTemplateOverlayDto, RockPhysicsTemplatePointDto,
    RockPhysicsTemplatePolygonOverlayDto, RockPhysicsTemplatePolylineOverlayDto,
    RockPhysicsTemplateTextOverlayDto, RockPhysicsTextAlignDto, RockPhysicsTextBaselineDto,
    RockPhysicsWellDto, SECTION_WELL_OVERLAY_CONTRACT_VERSION, SURVEY_MAP_CONTRACT_VERSION,
    SectionColorMap, SectionCoordinate, SectionDisplayDefaults, SectionHorizonLineStyle,
    SectionHorizonOverlayView, SectionHorizonSample, SectionHorizonStyle,
    SectionInteractionChanged, SectionMetadata, SectionPolarity, SectionPrimaryMode, SectionProbe,
    SectionProbeChanged, SectionRenderMode, SectionScalarOverlayColorMap,
    SectionScalarOverlayValueRange, SectionScalarOverlayView, SectionTimeDepthDiagnostics,
    SectionTimeDepthTransformMode, SectionUnits, SectionView, SectionViewport,
    SectionViewportChanged, SectionWellOverlayDomainDto, SectionWellOverlayRequestDto,
    SectionWellOverlaySampleDto, SectionWellOverlaySegmentDto, SpatialCoverageRelationship,
    SpatialCoverageSummary, StratigraphicBoundaryReference, SurveyIndexAxisDto, SurveyIndexGridDto,
    SurveyMapGridTransformDto, SurveyMapRequestDto, SurveyMapSpatialAvailabilityDto,
    SurveyMapSpatialDescriptorDto, SurveyMapTrajectoryDto, SurveyMapTrajectoryStationDto,
    SurveyPropertyField3D, SurveyTimeDepthTransform3D, TimeDepthDomain, TimeDepthSample1D,
    TimeDepthTransformSourceKind, TrajectoryInputSchemaKind, TrajectoryValueOrigin,
    TravelTimeReference, VelocityControlProfile, VelocityControlProfileSample,
    VelocityControlProfileSet, VelocityIntervalTrend, VelocityQuantityKind, VelocitySource3D,
    VerticalAxisDescriptor, VerticalInterpolationMethod, WELL_PANEL_CONTRACT_VERSION,
    WellAzimuthReferenceKind, WellPanelDepthSampleDto, WellPanelDrillingObservationDto,
    WellPanelDrillingSetDto, WellPanelLogCurveDto, WellPanelPressureObservationDto,
    WellPanelPressureSetDto, WellPanelRequestDto, WellPanelTopRowDto, WellPanelTopSetDto,
    WellPanelTrajectoryDto, WellPanelTrajectoryRowDto, WellTieAnalysis1D, WellTieCurve1D,
    WellTieLogCurveSource, WellTieLogSelection1D, WellTieObservationSet1D, WellTieSectionWindow,
    WellTieTrace1D, WellTieVelocitySourceKind, WellTieWavelet, WellTimeDepthAssumptionInterval,
    WellTimeDepthAssumptionKind, WellTimeDepthAuthoredModel1D, WellTimeDepthModel1D,
    WellTimeDepthObservationSample, WellTimeDepthSourceBinding, WellboreAnchorKind,
    WellboreAnchorReference, WellboreGeometry,
};
use schemars::schema_for;
use ts_rs::{Config, ExportError, TS};

trait ExportAllTo: TS + 'static {
    fn export_all_to(output_dir: &Path) -> Result<(), ExportError> {
        let config = Config::default().with_out_dir(output_dir);
        Self::export_all(&config)
    }
}

impl<T> ExportAllTo for T where T: TS + 'static {}

fn main() -> Result<(), Box<dyn Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("scripts/contracts-export should live two levels under repo root")
        .to_path_buf();

    let package_root = repo_root
        .join("contracts")
        .join("ts")
        .join("ophiolite-contracts");
    let generated_dir = package_root.join("src").join("generated");
    let schema_dir = package_root.join("schemas");

    fs::create_dir_all(&generated_dir)?;
    fs::create_dir_all(&schema_dir)?;

    export_ts_types(&generated_dir)?;
    write_generated_index(&generated_dir)?;
    write_schema_bundle(&schema_dir)?;

    Ok(())
}

fn export_ts_types(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    for file in [
        "CoordinateReferenceBindingDto.ts",
        "CoordinateReferenceDto.ts",
        "CoordinateReferenceSourceDto.ts",
        "ResolvedWellPanelSourceDto.ts",
        "ResolvedWellPanelWellDto.ts",
        "ResolvedSurveyMapSourceDto.ts",
        "ResolvedSurveyMapSurveyDto.ts",
        "ResolvedSurveyMapWellDto.ts",
        "ResolvedAvoResponseSourceDto.ts",
        "ResolvedAvoCrossplotSourceDto.ts",
        "ResolvedAvoChiProjectionSourceDto.ts",
        "AvoAnisotropyModeDto.ts",
        "AvoAxisDto.ts",
        "AvoBackgroundRegionDto.ts",
        "AvoChiProjectionSeriesDto.ts",
        "AvoCrossplotPointDto.ts",
        "AvoCurveStyleDto.ts",
        "AvoInterfaceDto.ts",
        "AvoReferenceLineDto.ts",
        "AvoReflectivityModelDto.ts",
        "AvoResponseSeriesDto.ts",
        "GatherAxisKind.ts",
        "GatherInteractionChanged.ts",
        "GatherProbe.ts",
        "GatherProbeChanged.ts",
        "GatherSampleDomain.ts",
        "GatherView.ts",
        "GatherPreviewView.ts",
        "GatherViewport.ts",
        "GatherViewportChanged.ts",
        "ImportedHorizonDescriptor.ts",
        "PreviewView.ts",
        "DepthReferenceKind.ts",
        "ProjectSurveyMapRequestDto.ts",
        "ProjectedPoint2Dto.ts",
        "ProjectedPolygon2Dto.ts",
        "ProjectedVector2Dto.ts",
        "StratigraphicBoundaryReference.ts",
        "LateralInterpolationMethod.ts",
        "VerticalInterpolationMethod.ts",
        "SpatialCoverageRelationship.ts",
        "SpatialCoverageSummary.ts",
        "SurveyIndexAxisDto.ts",
        "SurveyIndexGridDto.ts",
        "SurveyMapGridTransformDto.ts",
        "SurveyMapRequestDto.ts",
        "SectionColorMap.ts",
        "SectionRenderMode.ts",
        "SectionPolarity.ts",
        "SectionPrimaryMode.ts",
        "SectionCoordinate.ts",
        "SectionUnits.ts",
        "SectionMetadata.ts",
        "SectionDisplayDefaults.ts",
        "SectionView.ts",
        "SectionScalarOverlayColorMap.ts",
        "SectionTimeDepthTransformMode.ts",
        "SectionTimeDepthDiagnostics.ts",
        "SectionScalarOverlayValueRange.ts",
        "SectionScalarOverlayView.ts",
        "SectionHorizonLineStyle.ts",
        "SectionHorizonStyle.ts",
        "SectionHorizonSample.ts",
        "SectionHorizonOverlayView.ts",
        "ResolvedSectionDisplayView.ts",
        "SectionViewport.ts",
        "SectionProbe.ts",
        "SectionProbeChanged.ts",
        "SectionViewportChanged.ts",
        "SectionInteractionChanged.ts",
        "SectionWellOverlayDomainDto.ts",
        "SectionWellOverlayRequestDto.ts",
        "SectionWellOverlaySampleDto.ts",
        "SectionWellOverlaySegmentDto.ts",
        "ResolvedSectionWellOverlayDto.ts",
        "ResolveSectionWellOverlaysResponse.ts",
        "SurveyMapSpatialAvailabilityDto.ts",
        "SurveyMapSpatialDescriptorDto.ts",
        "SurveyMapTrajectoryDto.ts",
        "SurveyMapTrajectoryStationDto.ts",
        "SurveyPropertyField3D.ts",
        "SurveyTimeDepthTransform3D.ts",
        "TimeDepthDomain.ts",
        "TimeDepthSample1D.ts",
        "TimeDepthTransformSourceKind.ts",
        "TravelTimeReference.ts",
        "VelocityControlProfileSample.ts",
        "VelocityControlProfile.ts",
        "VelocityControlProfileSet.ts",
        "VelocityIntervalTrend.ts",
        "VelocityQuantityKind.ts",
        "VelocitySource3D.ts",
        "LayeredVelocityInterval.ts",
        "LayeredVelocityModel.ts",
        "BuildSurveyTimeDepthTransformRequest.ts",
        "BuildSurveyPropertyFieldRequest.ts",
        "VerticalAxisDescriptor.ts",
        "WellboreAnchorKind.ts",
        "WellAzimuthReferenceKind.ts",
        "WellboreAnchorReference.ts",
        "WellboreGeometry.ts",
        "TrajectoryInputSchemaKind.ts",
        "TrajectoryValueOrigin.ts",
        "ResolvedTrajectoryStation.ts",
        "ResolvedTrajectoryGeometry.ts",
        "WellTimeDepthObservationSample.ts",
        "WellTieVelocitySourceKind.ts",
        "WellTieLogCurveSource.ts",
        "WellTieLogSelection1D.ts",
        "WellTieCurve1D.ts",
        "WellTieTrace1D.ts",
        "WellTieWavelet.ts",
        "WellTieSectionWindow.ts",
        "WellTieAnalysis1D.ts",
        "CheckshotVspObservationSet1D.ts",
        "ManualTimeDepthPickSet1D.ts",
        "WellTieObservationSet1D.ts",
        "WellTimeDepthSourceBinding.ts",
        "WellTimeDepthAssumptionKind.ts",
        "WellTimeDepthAssumptionInterval.ts",
        "WellTimeDepthAuthoredModel1D.ts",
        "CompiledWellTimeDepthLineage.ts",
        "WellPanelDepthSampleDto.ts",
        "WellPanelDrillingObservationDto.ts",
        "WellPanelDrillingSetDto.ts",
        "WellPanelLogCurveDto.ts",
        "WellPanelPressureObservationDto.ts",
        "WellPanelPressureSetDto.ts",
        "WellPanelRequestDto.ts",
        "WellPanelTopRowDto.ts",
        "WellPanelTopSetDto.ts",
        "WellPanelTrajectoryDto.ts",
        "WellPanelTrajectoryRowDto.ts",
        "ResolvedRockPhysicsCrossplotSourceDto.ts",
        "RockPhysicsAxisDto.ts",
        "RockPhysicsCategoricalColorBindingDto.ts",
        "RockPhysicsCategoricalColorRequestDto.ts",
        "RockPhysicsCategoricalSemanticDto.ts",
        "RockPhysicsCategoryDto.ts",
        "RockPhysicsColorBindingDto.ts",
        "RockPhysicsColorRequestDto.ts",
        "RockPhysicsContinuousColorBindingDto.ts",
        "RockPhysicsContinuousColorRequestDto.ts",
        "RockPhysicsCrossplotRequestDto.ts",
        "RockPhysicsCurveSemanticDto.ts",
        "RockPhysicsInteractionThresholdsDto.ts",
        "RockPhysicsPointSymbolDto.ts",
        "RockPhysicsSampleDto.ts",
        "RockPhysicsSourceBindingDto.ts",
        "RockPhysicsTemplateIdDto.ts",
        "RockPhysicsTemplateLineDto.ts",
        "RockPhysicsTemplateOverlayDto.ts",
        "RockPhysicsTemplatePointDto.ts",
        "RockPhysicsTemplatePolygonOverlayDto.ts",
        "RockPhysicsTemplatePolylineOverlayDto.ts",
        "RockPhysicsTemplateTextOverlayDto.ts",
        "RockPhysicsTextAlignDto.ts",
        "RockPhysicsTextBaselineDto.ts",
        "RockPhysicsWellDto.ts",
        "OperatorAvailability.ts",
        "OperatorCatalog.ts",
        "OperatorCatalogEntry.ts",
        "OperatorOutputLifecycle.ts",
        "OperatorStability.ts",
        "OperatorContractRef.ts",
        "OperatorDetail.ts",
        "OperatorDocumentation.ts",
        "OperatorExecutionKind.ts",
        "OperatorFamily.ts",
        "OperatorParameterDoc.ts",
        "OperatorSubjectKind.ts",
        "rock-physics-crossplot-contract-version.ts",
        "avo-analysis-contract-version.ts",
        "WellTimeDepthModel1D.ts",
        "section-well-overlay-contract-version.ts",
        "survey-map-contract-version.ts",
        "well-panel-contract-version.ts",
        "index.ts",
    ] {
        let path = output_dir.join(file);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    WellPanelRequestDto::export_all_to(output_dir)?;
    ProjectSurveyMapRequestDto::export_all_to(output_dir)?;
    ResolvedWellPanelSourceDto::export_all_to(output_dir)?;
    ResolvedAvoResponseSourceDto::export_all_to(output_dir)?;
    ResolvedAvoCrossplotSourceDto::export_all_to(output_dir)?;
    ResolvedAvoChiProjectionSourceDto::export_all_to(output_dir)?;
    RockPhysicsCrossplotRequestDto::export_all_to(output_dir)?;
    ResolvedRockPhysicsCrossplotSourceDto::export_all_to(output_dir)?;
    SurveyMapRequestDto::export_all_to(output_dir)?;
    ResolvedSurveyMapSourceDto::export_all_to(output_dir)?;
    SectionWellOverlayRequestDto::export_all_to(output_dir)?;
    ResolveSectionWellOverlaysResponse::export_all_to(output_dir)?;
    GatherView::export_all_to(output_dir)?;
    GatherPreviewView::export_all_to(output_dir)?;
    GatherViewportChanged::export_all_to(output_dir)?;
    GatherProbeChanged::export_all_to(output_dir)?;
    GatherInteractionChanged::export_all_to(output_dir)?;
    ImportedHorizonDescriptor::export_all_to(output_dir)?;
    PreviewView::export_all_to(output_dir)?;
    SectionColorMap::export_all_to(output_dir)?;
    SectionRenderMode::export_all_to(output_dir)?;
    SectionPolarity::export_all_to(output_dir)?;
    SectionPrimaryMode::export_all_to(output_dir)?;
    SectionCoordinate::export_all_to(output_dir)?;
    SectionUnits::export_all_to(output_dir)?;
    SectionMetadata::export_all_to(output_dir)?;
    SectionDisplayDefaults::export_all_to(output_dir)?;
    SectionView::export_all_to(output_dir)?;
    SectionScalarOverlayColorMap::export_all_to(output_dir)?;
    SectionTimeDepthTransformMode::export_all_to(output_dir)?;
    SectionTimeDepthDiagnostics::export_all_to(output_dir)?;
    SectionScalarOverlayValueRange::export_all_to(output_dir)?;
    SectionScalarOverlayView::export_all_to(output_dir)?;
    SectionHorizonLineStyle::export_all_to(output_dir)?;
    SectionHorizonStyle::export_all_to(output_dir)?;
    SectionHorizonSample::export_all_to(output_dir)?;
    SectionHorizonOverlayView::export_all_to(output_dir)?;
    ResolvedSectionDisplayView::export_all_to(output_dir)?;
    SectionViewport::export_all_to(output_dir)?;
    SectionProbe::export_all_to(output_dir)?;
    SectionProbeChanged::export_all_to(output_dir)?;
    SectionViewportChanged::export_all_to(output_dir)?;
    SectionInteractionChanged::export_all_to(output_dir)?;
    StratigraphicBoundaryReference::export_all_to(output_dir)?;
    LateralInterpolationMethod::export_all_to(output_dir)?;
    VerticalInterpolationMethod::export_all_to(output_dir)?;
    VelocityControlProfileSample::export_all_to(output_dir)?;
    VelocityControlProfile::export_all_to(output_dir)?;
    VelocityControlProfileSet::export_all_to(output_dir)?;
    VelocityIntervalTrend::export_all_to(output_dir)?;
    VelocitySource3D::export_all_to(output_dir)?;
    LayeredVelocityInterval::export_all_to(output_dir)?;
    LayeredVelocityModel::export_all_to(output_dir)?;
    BuildSurveyTimeDepthTransformRequest::export_all_to(output_dir)?;
    BuildSurveyPropertyFieldRequest::export_all_to(output_dir)?;
    WellboreGeometry::export_all_to(output_dir)?;
    TrajectoryInputSchemaKind::export_all_to(output_dir)?;
    ResolvedTrajectoryGeometry::export_all_to(output_dir)?;
    WellTieVelocitySourceKind::export_all_to(output_dir)?;
    WellTieLogCurveSource::export_all_to(output_dir)?;
    WellTieLogSelection1D::export_all_to(output_dir)?;
    WellTieCurve1D::export_all_to(output_dir)?;
    WellTieTrace1D::export_all_to(output_dir)?;
    WellTieWavelet::export_all_to(output_dir)?;
    WellTieSectionWindow::export_all_to(output_dir)?;
    WellTieAnalysis1D::export_all_to(output_dir)?;
    CheckshotVspObservationSet1D::export_all_to(output_dir)?;
    ManualTimeDepthPickSet1D::export_all_to(output_dir)?;
    WellTieObservationSet1D::export_all_to(output_dir)?;
    WellTimeDepthAuthoredModel1D::export_all_to(output_dir)?;
    CompiledWellTimeDepthLineage::export_all_to(output_dir)?;
    WellTimeDepthModel1D::export_all_to(output_dir)?;
    SurveyPropertyField3D::export_all_to(output_dir)?;
    SurveyTimeDepthTransform3D::export_all_to(output_dir)?;
    OperatorCatalog::export_all_to(output_dir)?;
    OperatorCatalogEntry::export_all_to(output_dir)?;
    OperatorSubjectKind::export_all_to(output_dir)?;
    OperatorFamily::export_all_to(output_dir)?;
    OperatorExecutionKind::export_all_to(output_dir)?;
    OperatorCatalogOutputLifecycle::export_all_to(output_dir)?;
    OperatorCatalogStability::export_all_to(output_dir)?;
    OperatorContractRef::export_all_to(output_dir)?;
    OperatorDocumentation::export_all_to(output_dir)?;
    OperatorParameterDoc::export_all_to(output_dir)?;
    OperatorAvailability::export_all_to(output_dir)?;
    OperatorDetail::export_all_to(output_dir)?;

    fs::write(
        output_dir.join("rock-physics-crossplot-contract-version.ts"),
        format!(
            "// Generated by `cargo run -p contracts-export`\nexport const ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION = {ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION} as const;\n"
        ),
    )?;
    fs::write(
        output_dir.join("avo-analysis-contract-version.ts"),
        format!(
            "// Generated by `cargo run -p contracts-export`\nexport const AVO_ANALYSIS_CONTRACT_VERSION = {AVO_ANALYSIS_CONTRACT_VERSION} as const;\n"
        ),
    )?;
    fs::write(
        output_dir.join("section-well-overlay-contract-version.ts"),
        format!(
            "// Generated by `cargo run -p contracts-export`\nexport const SECTION_WELL_OVERLAY_CONTRACT_VERSION = {SECTION_WELL_OVERLAY_CONTRACT_VERSION} as const;\n"
        ),
    )?;
    fs::write(
        output_dir.join("survey-map-contract-version.ts"),
        format!(
            "// Generated by `cargo run -p contracts-export`\nexport const SURVEY_MAP_CONTRACT_VERSION = {SURVEY_MAP_CONTRACT_VERSION} as const;\n"
        ),
    )?;
    fs::write(
        output_dir.join("well-panel-contract-version.ts"),
        format!(
            "// Generated by `cargo run -p contracts-export`\nexport const WELL_PANEL_CONTRACT_VERSION = {WELL_PANEL_CONTRACT_VERSION} as const;\n"
        ),
    )?;

    Ok(())
}

fn write_generated_index(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let index = r#"// Generated by `cargo run -p contracts-export`
export type { CoordinateReferenceDto } from "./CoordinateReferenceDto";
export type { CoordinateReferenceBindingDto } from "./CoordinateReferenceBindingDto";
export type { CoordinateReferenceSourceDto } from "./CoordinateReferenceSourceDto";
export type { CoordinateReferenceDescriptor } from "./CoordinateReferenceDescriptor";
export type { ProjectedPoint2Dto } from "./ProjectedPoint2Dto";
export type { ProjectedPolygon2Dto } from "./ProjectedPolygon2Dto";
export type { ProjectedVector2Dto } from "./ProjectedVector2Dto";
export type { ResolvedSurveyMapHorizonDto } from "./ResolvedSurveyMapHorizonDto";
export type { ResolvedSurveyMapSourceDto } from "./ResolvedSurveyMapSourceDto";
export type { ResolvedSurveyMapSurveyDto } from "./ResolvedSurveyMapSurveyDto";
export type { ResolvedSurveyMapWellDto } from "./ResolvedSurveyMapWellDto";
export type { ResolvedWellPanelSourceDto } from "./ResolvedWellPanelSourceDto";
export type { ResolvedWellPanelWellDto } from "./ResolvedWellPanelWellDto";
export type { ResolvedAvoResponseSourceDto } from "./ResolvedAvoResponseSourceDto";
export type { ResolvedAvoCrossplotSourceDto } from "./ResolvedAvoCrossplotSourceDto";
export type { ResolvedAvoChiProjectionSourceDto } from "./ResolvedAvoChiProjectionSourceDto";
export type { AvoAnisotropyModeDto } from "./AvoAnisotropyModeDto";
export type { AvoAxisDto } from "./AvoAxisDto";
export type { AvoBackgroundRegionDto } from "./AvoBackgroundRegionDto";
export type { AvoChiProjectionSeriesDto } from "./AvoChiProjectionSeriesDto";
export type { AvoCrossplotPointDto } from "./AvoCrossplotPointDto";
export type { AvoCurveStyleDto } from "./AvoCurveStyleDto";
export type { AvoInterfaceDto } from "./AvoInterfaceDto";
export type { AvoReferenceLineDto } from "./AvoReferenceLineDto";
export type { AvoReflectivityModelDto } from "./AvoReflectivityModelDto";
export type { AvoResponseSeriesDto } from "./AvoResponseSeriesDto";
export type { RockPhysicsCrossplotRequestDto } from "./RockPhysicsCrossplotRequestDto";
export type { ResolvedRockPhysicsCrossplotSourceDto } from "./ResolvedRockPhysicsCrossplotSourceDto";
export type { RockPhysicsAxisDto } from "./RockPhysicsAxisDto";
export type { RockPhysicsCategoricalColorBindingDto } from "./RockPhysicsCategoricalColorBindingDto";
export type { RockPhysicsCategoricalColorRequestDto } from "./RockPhysicsCategoricalColorRequestDto";
export type { RockPhysicsCategoricalSemanticDto } from "./RockPhysicsCategoricalSemanticDto";
export type { RockPhysicsCategoryDto } from "./RockPhysicsCategoryDto";
export type { RockPhysicsColorBindingDto } from "./RockPhysicsColorBindingDto";
export type { RockPhysicsColorRequestDto } from "./RockPhysicsColorRequestDto";
export type { RockPhysicsContinuousColorBindingDto } from "./RockPhysicsContinuousColorBindingDto";
export type { RockPhysicsContinuousColorRequestDto } from "./RockPhysicsContinuousColorRequestDto";
export type { RockPhysicsCurveSemanticDto } from "./RockPhysicsCurveSemanticDto";
export type { RockPhysicsInteractionThresholdsDto } from "./RockPhysicsInteractionThresholdsDto";
export type { RockPhysicsPointSymbolDto } from "./RockPhysicsPointSymbolDto";
export type { RockPhysicsSampleDto } from "./RockPhysicsSampleDto";
export type { RockPhysicsSourceBindingDto } from "./RockPhysicsSourceBindingDto";
export type { RockPhysicsTemplateIdDto } from "./RockPhysicsTemplateIdDto";
export type { RockPhysicsTemplateLineDto } from "./RockPhysicsTemplateLineDto";
export type { RockPhysicsTemplateOverlayDto } from "./RockPhysicsTemplateOverlayDto";
export type { RockPhysicsTemplatePointDto } from "./RockPhysicsTemplatePointDto";
export type { RockPhysicsTemplatePolygonOverlayDto } from "./RockPhysicsTemplatePolygonOverlayDto";
export type { RockPhysicsTemplatePolylineOverlayDto } from "./RockPhysicsTemplatePolylineOverlayDto";
export type { RockPhysicsTemplateTextOverlayDto } from "./RockPhysicsTemplateTextOverlayDto";
export type { RockPhysicsTextAlignDto } from "./RockPhysicsTextAlignDto";
export type { RockPhysicsTextBaselineDto } from "./RockPhysicsTextBaselineDto";
export type { RockPhysicsWellDto } from "./RockPhysicsWellDto";
export type { GatherAxisKind } from "./GatherAxisKind";
export type { GatherInteractionChanged } from "./GatherInteractionChanged";
export type { GatherProbe } from "./GatherProbe";
export type { GatherProbeChanged } from "./GatherProbeChanged";
export type { GatherSampleDomain } from "./GatherSampleDomain";
export type { GatherView } from "./GatherView";
export type { GatherPreviewView } from "./GatherPreviewView";
export type { GatherViewport } from "./GatherViewport";
export type { GatherViewportChanged } from "./GatherViewportChanged";
export type { ImportedHorizonDescriptor } from "./ImportedHorizonDescriptor";
export type { PreviewView } from "./PreviewView";
export type { DepthReferenceKind } from "./DepthReferenceKind";
export type { ProjectSurveyMapRequestDto } from "./ProjectSurveyMapRequestDto";
export type { StratigraphicBoundaryReference } from "./StratigraphicBoundaryReference";
export type { LateralInterpolationMethod } from "./LateralInterpolationMethod";
export type { VerticalInterpolationMethod } from "./VerticalInterpolationMethod";
export type { SurveyIndexAxisDto } from "./SurveyIndexAxisDto";
export type { SurveyIndexGridDto } from "./SurveyIndexGridDto";
export type { SurveyGridTransform } from "./SurveyGridTransform";
export type { SurveyMapGridTransformDto } from "./SurveyMapGridTransformDto";
export type { SurveyMapRequestDto } from "./SurveyMapRequestDto";
export type { SurveyMapScalarFieldDto } from "./SurveyMapScalarFieldDto";
export type { SectionColorMap } from "./SectionColorMap";
export type { SectionRenderMode } from "./SectionRenderMode";
export type { SectionPolarity } from "./SectionPolarity";
export type { SectionPrimaryMode } from "./SectionPrimaryMode";
export type { SectionCoordinate } from "./SectionCoordinate";
export type { SectionUnits } from "./SectionUnits";
export type { SectionMetadata } from "./SectionMetadata";
export type { SectionDisplayDefaults } from "./SectionDisplayDefaults";
export type { SectionView } from "./SectionView";
export type { SectionScalarOverlayColorMap } from "./SectionScalarOverlayColorMap";
export type { SectionTimeDepthTransformMode } from "./SectionTimeDepthTransformMode";
export type { SectionTimeDepthDiagnostics } from "./SectionTimeDepthDiagnostics";
export type { SectionScalarOverlayValueRange } from "./SectionScalarOverlayValueRange";
export type { SectionScalarOverlayView } from "./SectionScalarOverlayView";
export type { SectionHorizonLineStyle } from "./SectionHorizonLineStyle";
export type { SectionHorizonStyle } from "./SectionHorizonStyle";
export type { SectionHorizonSample } from "./SectionHorizonSample";
export type { SectionHorizonOverlayView } from "./SectionHorizonOverlayView";
export type { ResolvedSectionDisplayView } from "./ResolvedSectionDisplayView";
export type { SectionViewport } from "./SectionViewport";
export type { SectionProbe } from "./SectionProbe";
export type { SectionProbeChanged } from "./SectionProbeChanged";
export type { SectionViewportChanged } from "./SectionViewportChanged";
export type { SectionInteractionChanged } from "./SectionInteractionChanged";
export type { SectionWellOverlayDomainDto } from "./SectionWellOverlayDomainDto";
export type { SectionWellOverlayRequestDto } from "./SectionWellOverlayRequestDto";
export type { SectionWellOverlaySampleDto } from "./SectionWellOverlaySampleDto";
export type { SectionWellOverlaySegmentDto } from "./SectionWellOverlaySegmentDto";
export type { ResolvedSectionWellOverlayDto } from "./ResolvedSectionWellOverlayDto";
export type { ResolveSectionWellOverlaysResponse } from "./ResolveSectionWellOverlaysResponse";
export type { SurveyMapSpatialAvailabilityDto } from "./SurveyMapSpatialAvailabilityDto";
export type { SurveyMapSpatialDescriptorDto } from "./SurveyMapSpatialDescriptorDto";
export type { SurveyMapTrajectoryDto } from "./SurveyMapTrajectoryDto";
export type { SurveyMapTrajectoryStationDto } from "./SurveyMapTrajectoryStationDto";
export type { SurveyMapTransformDiagnosticsDto } from "./SurveyMapTransformDiagnosticsDto";
export type { SurveyMapTransformPolicyDto } from "./SurveyMapTransformPolicyDto";
export type { SurveyMapTransformStatusDto } from "./SurveyMapTransformStatusDto";
export type { SpatialCoverageRelationship } from "./SpatialCoverageRelationship";
export type { SpatialCoverageSummary } from "./SpatialCoverageSummary";
export type { SurveyPropertyField3D } from "./SurveyPropertyField3D";
export type { SurveyTimeDepthTransform3D } from "./SurveyTimeDepthTransform3D";
export type { TimeDepthDomain } from "./TimeDepthDomain";
export type { TimeDepthSample1D } from "./TimeDepthSample1D";
export type { TimeDepthTransformSourceKind } from "./TimeDepthTransformSourceKind";
export type { TravelTimeReference } from "./TravelTimeReference";
export type { VelocityControlProfileSample } from "./VelocityControlProfileSample";
export type { VelocityControlProfile } from "./VelocityControlProfile";
export type { VelocityControlProfileSet } from "./VelocityControlProfileSet";
export type { VelocityIntervalTrend } from "./VelocityIntervalTrend";
export type { VelocityQuantityKind } from "./VelocityQuantityKind";
export type { VelocitySource3D } from "./VelocitySource3D";
export type { LayeredVelocityInterval } from "./LayeredVelocityInterval";
export type { LayeredVelocityModel } from "./LayeredVelocityModel";
export type { BuildSurveyTimeDepthTransformRequest } from "./BuildSurveyTimeDepthTransformRequest";
export type { BuildSurveyPropertyFieldRequest } from "./BuildSurveyPropertyFieldRequest";
export type { VerticalAxisDescriptor } from "./VerticalAxisDescriptor";
export type { WellboreAnchorKind } from "./WellboreAnchorKind";
export type { WellAzimuthReferenceKind } from "./WellAzimuthReferenceKind";
export type { WellboreAnchorReference } from "./WellboreAnchorReference";
export type { WellboreGeometry } from "./WellboreGeometry";
export type { TrajectoryInputSchemaKind } from "./TrajectoryInputSchemaKind";
export type { TrajectoryValueOrigin } from "./TrajectoryValueOrigin";
export type { ResolvedTrajectoryStation } from "./ResolvedTrajectoryStation";
export type { ResolvedTrajectoryGeometry } from "./ResolvedTrajectoryGeometry";
export type { WellTimeDepthObservationSample } from "./WellTimeDepthObservationSample";
export type { WellTieVelocitySourceKind } from "./WellTieVelocitySourceKind";
export type { WellTieLogCurveSource } from "./WellTieLogCurveSource";
export type { WellTieLogSelection1D } from "./WellTieLogSelection1D";
export type { WellTieCurve1D } from "./WellTieCurve1D";
export type { WellTieTrace1D } from "./WellTieTrace1D";
export type { WellTieWavelet } from "./WellTieWavelet";
export type { WellTieSectionWindow } from "./WellTieSectionWindow";
export type { WellTieAnalysis1D } from "./WellTieAnalysis1D";
export type { CheckshotVspObservationSet1D } from "./CheckshotVspObservationSet1D";
export type { ManualTimeDepthPickSet1D } from "./ManualTimeDepthPickSet1D";
export type { WellTieObservationSet1D } from "./WellTieObservationSet1D";
export type { WellTimeDepthSourceBinding } from "./WellTimeDepthSourceBinding";
export type { WellTimeDepthAssumptionKind } from "./WellTimeDepthAssumptionKind";
export type { WellTimeDepthAssumptionInterval } from "./WellTimeDepthAssumptionInterval";
export type { WellTimeDepthAuthoredModel1D } from "./WellTimeDepthAuthoredModel1D";
export type { CompiledWellTimeDepthLineage } from "./CompiledWellTimeDepthLineage";
export type { WellPanelDepthSampleDto } from "./WellPanelDepthSampleDto";
export type { WellPanelDrillingObservationDto } from "./WellPanelDrillingObservationDto";
export type { WellPanelDrillingSetDto } from "./WellPanelDrillingSetDto";
export type { WellPanelLogCurveDto } from "./WellPanelLogCurveDto";
export type { WellPanelPressureObservationDto } from "./WellPanelPressureObservationDto";
export type { WellPanelPressureSetDto } from "./WellPanelPressureSetDto";
export type { WellPanelRequestDto } from "./WellPanelRequestDto";
export type { WellPanelTopRowDto } from "./WellPanelTopRowDto";
export type { WellPanelTopSetDto } from "./WellPanelTopSetDto";
export type { WellPanelTrajectoryDto } from "./WellPanelTrajectoryDto";
export type { WellPanelTrajectoryRowDto } from "./WellPanelTrajectoryRowDto";
export type { WellTimeDepthModel1D } from "./WellTimeDepthModel1D";
export type { OperatorCatalog } from "./OperatorCatalog";
export type { OperatorCatalogEntry } from "./OperatorCatalogEntry";
export type { OperatorSubjectKind } from "./OperatorSubjectKind";
export type { OperatorFamily } from "./OperatorFamily";
export type { OperatorExecutionKind } from "./OperatorExecutionKind";
export type { OperatorOutputLifecycle as OperatorCatalogOutputLifecycle } from "./OperatorOutputLifecycle";
export type { OperatorStability as OperatorCatalogStability } from "./OperatorStability";
export type { OperatorContractRef } from "./OperatorContractRef";
export type { OperatorDocumentation } from "./OperatorDocumentation";
export type { OperatorParameterDoc } from "./OperatorParameterDoc";
export type { OperatorAvailability } from "./OperatorAvailability";
export type { OperatorDetail } from "./OperatorDetail";
export { AVO_ANALYSIS_CONTRACT_VERSION } from "./avo-analysis-contract-version";
export { ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION } from "./rock-physics-crossplot-contract-version";
export { SECTION_WELL_OVERLAY_CONTRACT_VERSION } from "./section-well-overlay-contract-version";
export { SURVEY_MAP_CONTRACT_VERSION } from "./survey-map-contract-version";
export { WELL_PANEL_CONTRACT_VERSION } from "./well-panel-contract-version";
"#;

    fs::write(output_dir.join("index.ts"), index)?;
    Ok(())
}

fn write_schema_bundle(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let bundle = serde_json::json!({
        "avoAnalysisContractVersion": AVO_ANALYSIS_CONTRACT_VERSION,
        "rockPhysicsCrossplotContractVersion": ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION,
        "sectionWellOverlayContractVersion": SECTION_WELL_OVERLAY_CONTRACT_VERSION,
        "surveyMapContractVersion": SURVEY_MAP_CONTRACT_VERSION,
        "wellPanelContractVersion": WELL_PANEL_CONTRACT_VERSION,
        "types": {
            "CoordinateReferenceDto": schema_for!(CoordinateReferenceDto),
            "CoordinateReferenceBindingDto": schema_for!(CoordinateReferenceBindingDto),
            "CoordinateReferenceSourceDto": schema_for!(CoordinateReferenceSourceDto),
            "ProjectSurveyMapRequestDto": schema_for!(ProjectSurveyMapRequestDto),
            "ProjectedPoint2Dto": schema_for!(ProjectedPoint2Dto),
            "ProjectedPolygon2Dto": schema_for!(ProjectedPolygon2Dto),
            "ProjectedVector2Dto": schema_for!(ProjectedVector2Dto),
            "SurveyMapRequestDto": schema_for!(SurveyMapRequestDto),
            "SurveyIndexAxisDto": schema_for!(SurveyIndexAxisDto),
            "SurveyIndexGridDto": schema_for!(SurveyIndexGridDto),
            "SurveyMapGridTransformDto": schema_for!(SurveyMapGridTransformDto),
            "SectionWellOverlayDomainDto": schema_for!(SectionWellOverlayDomainDto),
            "SectionWellOverlayRequestDto": schema_for!(SectionWellOverlayRequestDto),
            "SectionWellOverlaySampleDto": schema_for!(SectionWellOverlaySampleDto),
            "SectionWellOverlaySegmentDto": schema_for!(SectionWellOverlaySegmentDto),
            "ResolvedSectionWellOverlayDto": schema_for!(ResolvedSectionWellOverlayDto),
            "ResolveSectionWellOverlaysResponse": schema_for!(ResolveSectionWellOverlaysResponse),
            "SurveyMapSpatialAvailabilityDto": schema_for!(SurveyMapSpatialAvailabilityDto),
            "SurveyMapSpatialDescriptorDto": schema_for!(SurveyMapSpatialDescriptorDto),
            "SurveyMapTrajectoryStationDto": schema_for!(SurveyMapTrajectoryStationDto),
            "SurveyMapTrajectoryDto": schema_for!(SurveyMapTrajectoryDto),
            "ResolvedSurveyMapSurveyDto": schema_for!(ResolvedSurveyMapSurveyDto),
            "ResolvedSurveyMapWellDto": schema_for!(ResolvedSurveyMapWellDto),
            "ResolvedSurveyMapSourceDto": schema_for!(ResolvedSurveyMapSourceDto),
            "WellPanelRequestDto": schema_for!(WellPanelRequestDto),
            "AvoReflectivityModelDto": schema_for!(AvoReflectivityModelDto),
            "AvoAnisotropyModeDto": schema_for!(AvoAnisotropyModeDto),
            "AvoCurveStyleDto": schema_for!(AvoCurveStyleDto),
            "AvoAxisDto": schema_for!(AvoAxisDto),
            "AvoInterfaceDto": schema_for!(AvoInterfaceDto),
            "AvoResponseSeriesDto": schema_for!(AvoResponseSeriesDto),
            "ResolvedAvoResponseSourceDto": schema_for!(ResolvedAvoResponseSourceDto),
            "AvoCrossplotPointDto": schema_for!(AvoCrossplotPointDto),
            "AvoReferenceLineDto": schema_for!(AvoReferenceLineDto),
            "AvoBackgroundRegionDto": schema_for!(AvoBackgroundRegionDto),
            "ResolvedAvoCrossplotSourceDto": schema_for!(ResolvedAvoCrossplotSourceDto),
            "AvoChiProjectionSeriesDto": schema_for!(AvoChiProjectionSeriesDto),
            "ResolvedAvoChiProjectionSourceDto": schema_for!(ResolvedAvoChiProjectionSourceDto),
            "WellPanelDepthSampleDto": schema_for!(WellPanelDepthSampleDto),
            "WellPanelLogCurveDto": schema_for!(WellPanelLogCurveDto),
            "WellPanelTrajectoryRowDto": schema_for!(WellPanelTrajectoryRowDto),
            "WellPanelTrajectoryDto": schema_for!(WellPanelTrajectoryDto),
            "WellPanelTopRowDto": schema_for!(WellPanelTopRowDto),
            "WellPanelTopSetDto": schema_for!(WellPanelTopSetDto),
            "WellPanelPressureObservationDto": schema_for!(WellPanelPressureObservationDto),
            "WellPanelPressureSetDto": schema_for!(WellPanelPressureSetDto),
            "WellPanelDrillingObservationDto": schema_for!(WellPanelDrillingObservationDto),
            "WellPanelDrillingSetDto": schema_for!(WellPanelDrillingSetDto),
            "ResolvedWellPanelWellDto": schema_for!(ResolvedWellPanelWellDto),
            "ResolvedWellPanelSourceDto": schema_for!(ResolvedWellPanelSourceDto),
            "RockPhysicsTemplateIdDto": schema_for!(RockPhysicsTemplateIdDto),
            "RockPhysicsCurveSemanticDto": schema_for!(RockPhysicsCurveSemanticDto),
            "RockPhysicsCategoricalSemanticDto": schema_for!(RockPhysicsCategoricalSemanticDto),
            "RockPhysicsPointSymbolDto": schema_for!(RockPhysicsPointSymbolDto),
            "RockPhysicsAxisDto": schema_for!(RockPhysicsAxisDto),
            "RockPhysicsCategoryDto": schema_for!(RockPhysicsCategoryDto),
            "RockPhysicsCategoricalColorBindingDto": schema_for!(RockPhysicsCategoricalColorBindingDto),
            "RockPhysicsCategoricalColorRequestDto": schema_for!(RockPhysicsCategoricalColorRequestDto),
            "RockPhysicsContinuousColorBindingDto": schema_for!(RockPhysicsContinuousColorBindingDto),
            "RockPhysicsContinuousColorRequestDto": schema_for!(RockPhysicsContinuousColorRequestDto),
            "RockPhysicsColorBindingDto": schema_for!(RockPhysicsColorBindingDto),
            "RockPhysicsColorRequestDto": schema_for!(RockPhysicsColorRequestDto),
            "RockPhysicsWellDto": schema_for!(RockPhysicsWellDto),
            "RockPhysicsSourceBindingDto": schema_for!(RockPhysicsSourceBindingDto),
            "RockPhysicsSampleDto": schema_for!(RockPhysicsSampleDto),
            "RockPhysicsInteractionThresholdsDto": schema_for!(RockPhysicsInteractionThresholdsDto),
            "RockPhysicsTemplatePointDto": schema_for!(RockPhysicsTemplatePointDto),
            "RockPhysicsTemplateLineDto": schema_for!(RockPhysicsTemplateLineDto),
            "RockPhysicsTextAlignDto": schema_for!(RockPhysicsTextAlignDto),
            "RockPhysicsTextBaselineDto": schema_for!(RockPhysicsTextBaselineDto),
            "RockPhysicsTemplatePolylineOverlayDto": schema_for!(RockPhysicsTemplatePolylineOverlayDto),
            "RockPhysicsTemplatePolygonOverlayDto": schema_for!(RockPhysicsTemplatePolygonOverlayDto),
            "RockPhysicsTemplateTextOverlayDto": schema_for!(RockPhysicsTemplateTextOverlayDto),
            "RockPhysicsTemplateOverlayDto": schema_for!(RockPhysicsTemplateOverlayDto),
            "RockPhysicsCrossplotRequestDto": schema_for!(RockPhysicsCrossplotRequestDto),
            "ResolvedRockPhysicsCrossplotSourceDto": schema_for!(ResolvedRockPhysicsCrossplotSourceDto),
            "GatherAxisKind": schema_for!(GatherAxisKind),
            "GatherSampleDomain": schema_for!(GatherSampleDomain),
            "GatherView": schema_for!(GatherView),
            "GatherPreviewView": schema_for!(GatherPreviewView),
            "GatherViewport": schema_for!(GatherViewport),
            "GatherProbe": schema_for!(GatherProbe),
            "GatherViewportChanged": schema_for!(GatherViewportChanged),
            "GatherProbeChanged": schema_for!(GatherProbeChanged),
            "GatherInteractionChanged": schema_for!(GatherInteractionChanged),
            "ImportedHorizonDescriptor": schema_for!(ImportedHorizonDescriptor),
            "PreviewView": schema_for!(PreviewView),
            "SectionColorMap": schema_for!(SectionColorMap),
            "SectionRenderMode": schema_for!(SectionRenderMode),
            "SectionPolarity": schema_for!(SectionPolarity),
            "SectionPrimaryMode": schema_for!(SectionPrimaryMode),
            "SectionCoordinate": schema_for!(SectionCoordinate),
            "SectionUnits": schema_for!(SectionUnits),
            "SectionMetadata": schema_for!(SectionMetadata),
            "SectionDisplayDefaults": schema_for!(SectionDisplayDefaults),
            "SectionView": schema_for!(SectionView),
            "SectionScalarOverlayColorMap": schema_for!(SectionScalarOverlayColorMap),
            "SectionTimeDepthTransformMode": schema_for!(SectionTimeDepthTransformMode),
            "SectionTimeDepthDiagnostics": schema_for!(SectionTimeDepthDiagnostics),
            "SectionScalarOverlayValueRange": schema_for!(SectionScalarOverlayValueRange),
            "SectionScalarOverlayView": schema_for!(SectionScalarOverlayView),
            "SectionHorizonLineStyle": schema_for!(SectionHorizonLineStyle),
            "SectionHorizonStyle": schema_for!(SectionHorizonStyle),
            "SectionHorizonSample": schema_for!(SectionHorizonSample),
            "SectionHorizonOverlayView": schema_for!(SectionHorizonOverlayView),
            "ResolvedSectionDisplayView": schema_for!(ResolvedSectionDisplayView),
            "SectionViewport": schema_for!(SectionViewport),
            "SectionProbe": schema_for!(SectionProbe),
            "SectionProbeChanged": schema_for!(SectionProbeChanged),
            "SectionViewportChanged": schema_for!(SectionViewportChanged),
            "SectionInteractionChanged": schema_for!(SectionInteractionChanged),
            "TimeDepthDomain": schema_for!(TimeDepthDomain),
            "TimeDepthTransformSourceKind": schema_for!(TimeDepthTransformSourceKind),
            "VelocityQuantityKind": schema_for!(VelocityQuantityKind),
            "TravelTimeReference": schema_for!(TravelTimeReference),
            "DepthReferenceKind": schema_for!(DepthReferenceKind),
            "StratigraphicBoundaryReference": schema_for!(StratigraphicBoundaryReference),
            "LateralInterpolationMethod": schema_for!(LateralInterpolationMethod),
            "VerticalInterpolationMethod": schema_for!(VerticalInterpolationMethod),
            "SpatialCoverageRelationship": schema_for!(SpatialCoverageRelationship),
            "VerticalAxisDescriptor": schema_for!(VerticalAxisDescriptor),
            "SpatialCoverageSummary": schema_for!(SpatialCoverageSummary),
            "VelocityControlProfileSample": schema_for!(VelocityControlProfileSample),
            "VelocityControlProfile": schema_for!(VelocityControlProfile),
            "VelocityControlProfileSet": schema_for!(VelocityControlProfileSet),
            "VelocityIntervalTrend": schema_for!(VelocityIntervalTrend),
            "VelocitySource3D": schema_for!(VelocitySource3D),
            "LayeredVelocityInterval": schema_for!(LayeredVelocityInterval),
            "LayeredVelocityModel": schema_for!(LayeredVelocityModel),
            "BuildSurveyTimeDepthTransformRequest": schema_for!(BuildSurveyTimeDepthTransformRequest),
            "BuildSurveyPropertyFieldRequest": schema_for!(BuildSurveyPropertyFieldRequest),
            "WellboreAnchorKind": schema_for!(WellboreAnchorKind),
            "WellAzimuthReferenceKind": schema_for!(WellAzimuthReferenceKind),
            "WellboreAnchorReference": schema_for!(WellboreAnchorReference),
            "WellboreGeometry": schema_for!(WellboreGeometry),
            "TrajectoryInputSchemaKind": schema_for!(TrajectoryInputSchemaKind),
            "TrajectoryValueOrigin": schema_for!(TrajectoryValueOrigin),
            "ResolvedTrajectoryStation": schema_for!(ResolvedTrajectoryStation),
            "ResolvedTrajectoryGeometry": schema_for!(ResolvedTrajectoryGeometry),
            "TimeDepthSample1D": schema_for!(TimeDepthSample1D),
            "WellTimeDepthObservationSample": schema_for!(WellTimeDepthObservationSample),
            "WellTieVelocitySourceKind": schema_for!(WellTieVelocitySourceKind),
            "WellTieLogCurveSource": schema_for!(WellTieLogCurveSource),
            "WellTieLogSelection1D": schema_for!(WellTieLogSelection1D),
            "WellTieCurve1D": schema_for!(WellTieCurve1D),
            "WellTieTrace1D": schema_for!(WellTieTrace1D),
            "WellTieWavelet": schema_for!(WellTieWavelet),
            "WellTieSectionWindow": schema_for!(WellTieSectionWindow),
            "WellTieAnalysis1D": schema_for!(WellTieAnalysis1D),
            "CheckshotVspObservationSet1D": schema_for!(CheckshotVspObservationSet1D),
            "ManualTimeDepthPickSet1D": schema_for!(ManualTimeDepthPickSet1D),
            "WellTieObservationSet1D": schema_for!(WellTieObservationSet1D),
            "WellTimeDepthSourceBinding": schema_for!(WellTimeDepthSourceBinding),
            "WellTimeDepthAssumptionKind": schema_for!(WellTimeDepthAssumptionKind),
            "WellTimeDepthAssumptionInterval": schema_for!(WellTimeDepthAssumptionInterval),
            "WellTimeDepthAuthoredModel1D": schema_for!(WellTimeDepthAuthoredModel1D),
            "CompiledWellTimeDepthLineage": schema_for!(CompiledWellTimeDepthLineage),
            "WellTimeDepthModel1D": schema_for!(WellTimeDepthModel1D),
            "SurveyPropertyField3D": schema_for!(SurveyPropertyField3D),
            "SurveyTimeDepthTransform3D": schema_for!(SurveyTimeDepthTransform3D),
            "OperatorCatalog": schema_for!(OperatorCatalog),
            "OperatorCatalogEntry": schema_for!(OperatorCatalogEntry),
            "OperatorSubjectKind": schema_for!(OperatorSubjectKind),
            "OperatorFamily": schema_for!(OperatorFamily),
            "OperatorExecutionKind": schema_for!(OperatorExecutionKind),
            "OperatorCatalogOutputLifecycle": schema_for!(OperatorCatalogOutputLifecycle),
            "OperatorCatalogStability": schema_for!(OperatorCatalogStability),
            "OperatorContractRef": schema_for!(OperatorContractRef),
            "OperatorDocumentation": schema_for!(OperatorDocumentation),
            "OperatorParameterDoc": schema_for!(OperatorParameterDoc),
            "OperatorAvailability": schema_for!(OperatorAvailability),
            "OperatorDetail": schema_for!(OperatorDetail),
        }
    });

    fs::write(
        output_dir.join("ophiolite-contracts.schema.json"),
        serde_json::to_string_pretty(&bundle)?,
    )?;

    Ok(())
}
