use std::path::Path;

use serde::Serialize;

use crate::error::SeismicStoreError;
use crate::ingest::{IngestOptions, geometry_classification_label, header_field_spec};
use crate::metadata::segy_sample_data_fidelity;
use crate::{SegyInspection, inspect_segy};
use ophiolite_seismic::SampleValuePreservation;
use ophiolite_seismic::{
    SeismicGatherAxisKind, SeismicLayout, SeismicOrganization, SeismicStackingState,
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightAction {
    DirectDenseIngest,
    RegularizeSparseSurvey,
    ReviewGeometryMapping,
    UnsupportedInV1,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurveyPreflight {
    pub inspection: SegyInspection,
    pub sample_data_fidelity: ophiolite_seismic::SampleDataFidelity,
    pub geometry: PreflightGeometry,
    pub recommended_action: PreflightAction,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreflightGeometry {
    pub classification: String,
    pub stacking_state: String,
    pub organization: String,
    pub layout: String,
    pub gather_axis_kind: Option<String>,
    pub inline_field: crate::HeaderFieldSpec,
    pub crossline_field: crate::HeaderFieldSpec,
    pub third_axis_field: Option<crate::HeaderFieldSpec>,
    pub inline_count: usize,
    pub crossline_count: usize,
    pub third_axis_count: usize,
    pub observed_trace_count: usize,
    pub unique_coordinate_count: usize,
    pub expected_trace_count: usize,
    pub completeness_ratio: f64,
    pub missing_bin_count: usize,
    pub duplicate_coordinate_count: usize,
}

pub fn preflight_segy(
    path: impl AsRef<Path>,
    options: &IngestOptions,
) -> Result<SurveyPreflight, SeismicStoreError> {
    let path = path.as_ref();
    let inspection = inspect_segy(path)?;
    let reader = ophiolite_seismic_io::open(
        path,
        ophiolite_seismic_io::ReaderOptions {
            validation_mode: options.validation_mode,
            header_mapping: options.geometry.header_mapping.clone(),
            ..ophiolite_seismic_io::ReaderOptions::default()
        },
    )?;
    let report = reader.analyze_geometry(ophiolite_seismic_io::GeometryOptions {
        third_axis_field: options.geometry.third_axis_field,
        ..ophiolite_seismic_io::GeometryOptions::default()
    })?;

    let sample_data_fidelity = segy_sample_data_fidelity(inspection.sample_format_code);
    let recommended_action = match (report.stacking_state, report.layout, report.classification) {
        (
            SeismicStackingState::PreStack,
            SeismicLayout::PreStack3DOffset,
            ophiolite_seismic_io::GeometryClassification::RegularDense,
        ) => PreflightAction::DirectDenseIngest,
        (SeismicStackingState::PreStack, _, _) => PreflightAction::UnsupportedInV1,
        (_, _, ophiolite_seismic_io::GeometryClassification::RegularDense) => {
            PreflightAction::DirectDenseIngest
        }
        (_, _, ophiolite_seismic_io::GeometryClassification::RegularSparse) => {
            PreflightAction::RegularizeSparseSurvey
        }
        (
            _,
            _,
            ophiolite_seismic_io::GeometryClassification::DuplicateCoordinates
            | ophiolite_seismic_io::GeometryClassification::AmbiguousMapping,
        ) => PreflightAction::ReviewGeometryMapping,
        (_, _, ophiolite_seismic_io::GeometryClassification::NonCartesian) => {
            PreflightAction::UnsupportedInV1
        }
    };

    let mut notes = Vec::new();
    match report.classification {
        ophiolite_seismic_io::GeometryClassification::RegularDense => {
            notes.push("Survey is already dense under the resolved geometry mapping.".to_string());
        }
        ophiolite_seismic_io::GeometryClassification::RegularSparse => {
            notes.push(
                "Survey is sparse on an otherwise regular inline/xline grid and can be regularized explicitly."
                    .to_string(),
            );
            if options.geometry.third_axis_field.is_some() || !report.third_axis_values.is_empty() {
                notes.push(
                    "Current v1 sparse regularization only supports post-stack surveys without an explicit third axis."
                        .to_string(),
                );
            }
        }
        ophiolite_seismic_io::GeometryClassification::DuplicateCoordinates => {
            notes.push(
                "Duplicate coordinate tuples were observed; a duplicate-resolution policy is required before ingest."
                    .to_string(),
            );
        }
        ophiolite_seismic_io::GeometryClassification::AmbiguousMapping => {
            notes.push(
                "The resolved geometry mapping produces both missing bins and duplicate coordinates. Review header selection before ingest."
                    .to_string(),
            );
        }
        ophiolite_seismic_io::GeometryClassification::NonCartesian => {
            notes.push(
                "The dataset does not map cleanly to a Cartesian inline/xline grid under the current mapping."
                    .to_string(),
            );
        }
    }

    if report.stacking_state == SeismicStackingState::PreStack {
        let gather_axis = report
            .gather_axis_kind
            .map(seismic_gather_axis_kind_label)
            .unwrap_or("unknown");
        if report.layout == SeismicLayout::PreStack3DOffset
            && report.classification == ophiolite_seismic_io::GeometryClassification::RegularDense
        {
            notes.push(format!(
                "Resolved survey layout is {} with gather axis {}. Phase-one runtime ingest supports this path through ingest_prestack_offset_segy.",
                seismic_layout_label(report.layout),
                gather_axis,
            ));
        } else {
            notes.push(format!(
                "Resolved survey layout is {} with gather axis {}. Phase-one runtime ingest only supports regular dense pre_stack_3d_offset surveys.",
                seismic_layout_label(report.layout),
                gather_axis,
            ));
        }
    }

    if sample_data_fidelity.preservation == SampleValuePreservation::PotentiallyLossy {
        notes.push(format!(
            "Source sample conversion may be lossy: {} -> {}.",
            sample_data_fidelity.source_sample_type, sample_data_fidelity.working_sample_type
        ));
    }
    notes.extend(sample_data_fidelity.notes.iter().cloned());

    Ok(SurveyPreflight {
        inspection,
        sample_data_fidelity,
        geometry: PreflightGeometry {
            classification: geometry_classification_label(report.classification).to_string(),
            stacking_state: seismic_stacking_state_label(report.stacking_state).to_string(),
            organization: seismic_organization_label(report.organization).to_string(),
            layout: seismic_layout_label(report.layout).to_string(),
            gather_axis_kind: report
                .gather_axis_kind
                .map(|kind| seismic_gather_axis_kind_label(kind).to_string()),
            inline_field: header_field_spec(report.inline_field),
            crossline_field: header_field_spec(report.crossline_field),
            third_axis_field: report.third_axis_field.map(header_field_spec),
            inline_count: report.inline_values.len(),
            crossline_count: report.crossline_values.len(),
            third_axis_count: report.third_axis_values.len(),
            observed_trace_count: report.observed_trace_count,
            unique_coordinate_count: report.unique_coordinate_count,
            expected_trace_count: report.expected_trace_count,
            completeness_ratio: report.completeness_ratio,
            missing_bin_count: report.missing_bin_count,
            duplicate_coordinate_count: report.duplicate_coordinate_count,
        },
        recommended_action,
        notes,
    })
}

fn seismic_stacking_state_label(value: SeismicStackingState) -> &'static str {
    match value {
        SeismicStackingState::PostStack => "post_stack",
        SeismicStackingState::PreStack => "pre_stack",
        SeismicStackingState::Unknown => "unknown",
    }
}

fn seismic_organization_label(value: SeismicOrganization) -> &'static str {
    match value {
        SeismicOrganization::BinnedGrid => "binned_grid",
        SeismicOrganization::GatherCollection => "gather_collection",
        SeismicOrganization::Unstructured => "unstructured",
    }
}

fn seismic_layout_label(value: SeismicLayout) -> &'static str {
    match value {
        SeismicLayout::PostStack3D => "post_stack_3d",
        SeismicLayout::PostStack2D => "post_stack_2d",
        SeismicLayout::PreStack3DOffset => "pre_stack_3d_offset",
        SeismicLayout::PreStack3DAngle => "pre_stack_3d_angle",
        SeismicLayout::PreStack3DAzimuth => "pre_stack_3d_azimuth",
        SeismicLayout::PreStack3DUnknownAxis => "pre_stack_3d_unknown_axis",
        SeismicLayout::PreStack2DOffset => "pre_stack_2d_offset",
        SeismicLayout::ShotGatherSet => "shot_gather_set",
        SeismicLayout::ReceiverGatherSet => "receiver_gather_set",
        SeismicLayout::CmpGatherSet => "cmp_gather_set",
        SeismicLayout::UnstructuredTraceCollection => "unstructured_trace_collection",
    }
}

fn seismic_gather_axis_kind_label(value: SeismicGatherAxisKind) -> &'static str {
    match value {
        SeismicGatherAxisKind::Offset => "offset",
        SeismicGatherAxisKind::Angle => "angle",
        SeismicGatherAxisKind::Azimuth => "azimuth",
        SeismicGatherAxisKind::Shot => "shot",
        SeismicGatherAxisKind::Receiver => "receiver",
        SeismicGatherAxisKind::Cmp => "cmp",
        SeismicGatherAxisKind::Unknown => "unknown",
    }
}
