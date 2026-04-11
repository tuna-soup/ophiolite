#![recursion_limit = "512"]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use ophiolite::{
    BuildSurveyPropertyFieldRequest, BuildSurveyTimeDepthTransformRequest,
    CheckshotVspObservationSet1D, CompiledWellTimeDepthLineage, CoordinateReferenceBindingDto,
    CoordinateReferenceDto, CoordinateReferenceSourceDto, DepthReferenceKind, GatherAxisKind,
    GatherInteractionChanged, GatherProbe, GatherProbeChanged, GatherSampleDomain, GatherView,
    GatherViewport, GatherViewportChanged, LateralInterpolationMethod, LayeredVelocityInterval,
    LayeredVelocityModel, ManualTimeDepthPickSet1D, ProjectedPoint2Dto, ProjectedPolygon2Dto,
    ProjectedVector2Dto, ResolveSectionWellOverlaysResponse, ResolvedSectionWellOverlayDto,
    ResolvedSurveyMapSourceDto, ResolvedSurveyMapSurveyDto, ResolvedSurveyMapWellDto,
    ResolvedTrajectoryGeometry, ResolvedTrajectoryStation, ResolvedWellPanelSourceDto,
    ResolvedWellPanelWellDto, SECTION_WELL_OVERLAY_CONTRACT_VERSION, SURVEY_MAP_CONTRACT_VERSION,
    SectionWellOverlayDomainDto, SectionWellOverlayRequestDto, SectionWellOverlaySampleDto,
    SectionWellOverlaySegmentDto, SpatialCoverageRelationship, SpatialCoverageSummary,
    StratigraphicBoundaryReference, SurveyIndexAxisDto, SurveyIndexGridDto,
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
    WellPanelTrajectoryDto, WellPanelTrajectoryRowDto, WellTimeDepthAssumptionInterval,
    WellTimeDepthAssumptionKind, WellTimeDepthAuthoredModel1D, WellTimeDepthModel1D,
    WellTimeDepthObservationSample, WellTimeDepthSourceBinding, WellboreAnchorKind,
    WellboreAnchorReference, WellboreGeometry,
};
use schemars::schema_for;
use ts_rs::TS;

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
        "GatherAxisKind.ts",
        "GatherInteractionChanged.ts",
        "GatherProbe.ts",
        "GatherProbeChanged.ts",
        "GatherSampleDomain.ts",
        "GatherView.ts",
        "GatherViewport.ts",
        "GatherViewportChanged.ts",
        "DepthReferenceKind.ts",
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
        "CheckshotVspObservationSet1D.ts",
        "ManualTimeDepthPickSet1D.ts",
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
    ResolvedWellPanelSourceDto::export_all_to(output_dir)?;
    SurveyMapRequestDto::export_all_to(output_dir)?;
    ResolvedSurveyMapSourceDto::export_all_to(output_dir)?;
    SectionWellOverlayRequestDto::export_all_to(output_dir)?;
    ResolveSectionWellOverlaysResponse::export_all_to(output_dir)?;
    GatherView::export_all_to(output_dir)?;
    GatherViewportChanged::export_all_to(output_dir)?;
    GatherProbeChanged::export_all_to(output_dir)?;
    GatherInteractionChanged::export_all_to(output_dir)?;
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
    CheckshotVspObservationSet1D::export_all_to(output_dir)?;
    ManualTimeDepthPickSet1D::export_all_to(output_dir)?;
    WellTimeDepthAuthoredModel1D::export_all_to(output_dir)?;
    CompiledWellTimeDepthLineage::export_all_to(output_dir)?;
    WellTimeDepthModel1D::export_all_to(output_dir)?;
    SurveyPropertyField3D::export_all_to(output_dir)?;
    SurveyTimeDepthTransform3D::export_all_to(output_dir)?;

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
export type { ProjectedPoint2Dto } from "./ProjectedPoint2Dto";
export type { ProjectedPolygon2Dto } from "./ProjectedPolygon2Dto";
export type { ProjectedVector2Dto } from "./ProjectedVector2Dto";
export type { ResolvedSurveyMapSourceDto } from "./ResolvedSurveyMapSourceDto";
export type { ResolvedSurveyMapSurveyDto } from "./ResolvedSurveyMapSurveyDto";
export type { ResolvedSurveyMapWellDto } from "./ResolvedSurveyMapWellDto";
export type { ResolvedWellPanelSourceDto } from "./ResolvedWellPanelSourceDto";
export type { ResolvedWellPanelWellDto } from "./ResolvedWellPanelWellDto";
export type { GatherAxisKind } from "./GatherAxisKind";
export type { GatherInteractionChanged } from "./GatherInteractionChanged";
export type { GatherProbe } from "./GatherProbe";
export type { GatherProbeChanged } from "./GatherProbeChanged";
export type { GatherSampleDomain } from "./GatherSampleDomain";
export type { GatherView } from "./GatherView";
export type { GatherViewport } from "./GatherViewport";
export type { GatherViewportChanged } from "./GatherViewportChanged";
export type { DepthReferenceKind } from "./DepthReferenceKind";
export type { StratigraphicBoundaryReference } from "./StratigraphicBoundaryReference";
export type { LateralInterpolationMethod } from "./LateralInterpolationMethod";
export type { VerticalInterpolationMethod } from "./VerticalInterpolationMethod";
export type { SurveyIndexAxisDto } from "./SurveyIndexAxisDto";
export type { SurveyIndexGridDto } from "./SurveyIndexGridDto";
export type { SurveyMapGridTransformDto } from "./SurveyMapGridTransformDto";
export type { SurveyMapRequestDto } from "./SurveyMapRequestDto";
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
export type { CheckshotVspObservationSet1D } from "./CheckshotVspObservationSet1D";
export type { ManualTimeDepthPickSet1D } from "./ManualTimeDepthPickSet1D";
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
export { SECTION_WELL_OVERLAY_CONTRACT_VERSION } from "./section-well-overlay-contract-version";
export { SURVEY_MAP_CONTRACT_VERSION } from "./survey-map-contract-version";
export { WELL_PANEL_CONTRACT_VERSION } from "./well-panel-contract-version";
"#;

    fs::write(output_dir.join("index.ts"), index)?;
    Ok(())
}

fn write_schema_bundle(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let bundle = serde_json::json!({
        "sectionWellOverlayContractVersion": SECTION_WELL_OVERLAY_CONTRACT_VERSION,
        "surveyMapContractVersion": SURVEY_MAP_CONTRACT_VERSION,
        "wellPanelContractVersion": WELL_PANEL_CONTRACT_VERSION,
        "types": {
            "CoordinateReferenceDto": schema_for!(CoordinateReferenceDto),
            "CoordinateReferenceBindingDto": schema_for!(CoordinateReferenceBindingDto),
            "CoordinateReferenceSourceDto": schema_for!(CoordinateReferenceSourceDto),
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
            "GatherAxisKind": schema_for!(GatherAxisKind),
            "GatherSampleDomain": schema_for!(GatherSampleDomain),
            "GatherView": schema_for!(GatherView),
            "GatherViewport": schema_for!(GatherViewport),
            "GatherProbe": schema_for!(GatherProbe),
            "GatherViewportChanged": schema_for!(GatherViewportChanged),
            "GatherProbeChanged": schema_for!(GatherProbeChanged),
            "GatherInteractionChanged": schema_for!(GatherInteractionChanged),
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
            "CheckshotVspObservationSet1D": schema_for!(CheckshotVspObservationSet1D),
            "ManualTimeDepthPickSet1D": schema_for!(ManualTimeDepthPickSet1D),
            "WellTimeDepthSourceBinding": schema_for!(WellTimeDepthSourceBinding),
            "WellTimeDepthAssumptionKind": schema_for!(WellTimeDepthAssumptionKind),
            "WellTimeDepthAssumptionInterval": schema_for!(WellTimeDepthAssumptionInterval),
            "WellTimeDepthAuthoredModel1D": schema_for!(WellTimeDepthAuthoredModel1D),
            "CompiledWellTimeDepthLineage": schema_for!(CompiledWellTimeDepthLineage),
            "WellTimeDepthModel1D": schema_for!(WellTimeDepthModel1D),
            "SurveyPropertyField3D": schema_for!(SurveyPropertyField3D),
            "SurveyTimeDepthTransform3D": schema_for!(SurveyTimeDepthTransform3D),
        }
    });

    fs::write(
        output_dir.join("ophiolite-contracts.schema.json"),
        serde_json::to_string_pretty(&bundle)?,
    )?;

    Ok(())
}
