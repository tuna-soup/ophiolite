use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};
use ophiolite_seismic::{
    AxisSummaryF32, AxisSummaryI32, DatasetId, GatherAxisKind, GatherPreviewView, GatherRequest,
    GatherSampleDomain, GatherSelector, GatherView, GeometryDescriptor, GeometryProvenanceSummary,
    GeometrySummary, ProcessingArtifactRole, ProcessingLineageSummary, ProcessingPipelineSpec,
    SectionColorMap, SectionDisplayDefaults, SectionMetadata, SectionPolarity, SectionRenderMode,
    SectionUnits, SeismicLayout, VolumeDescriptor,
};
use serde::{Deserialize, Serialize};

use crate::error::SeismicStoreError;
use crate::gather_processing::{GatherPlane, apply_gather_processing_pipeline};
use crate::metadata::{
    DatasetKind, ProcessingLineage, VolumeMetadata, generate_store_id, normalize_source_identity,
    normalize_volume_axes, validate_vertical_axis,
};
use crate::store::apply_native_coordinate_reference_override;

const MANIFEST_FILE: &str = "manifest.json";
const AMPLITUDE_FILE: &str = "amplitude.bin";
const GEOMETRY_COMPARE_FAMILY: &str = "seismic-grid:v1";
const GEOMETRY_FINGERPRINT_VERSION: &str = "geom:v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbgathManifest {
    pub format: String,
    pub version: u32,
    pub volume: VolumeMetadata,
    pub layout: SeismicLayout,
    pub gather_axis_kind: GatherAxisKind,
    pub gather_axis_values: Vec<f64>,
    pub sample_type: String,
    pub endianness: String,
    pub amplitude_trace_bytes: u64,
}

impl TbgathManifest {
    pub fn new(
        mut volume: VolumeMetadata,
        layout: SeismicLayout,
        gather_axis_kind: GatherAxisKind,
        gather_axis_values: Vec<f64>,
    ) -> Self {
        if volume.store_id.trim().is_empty() {
            volume.store_id = generate_store_id();
        }
        Self {
            format: "tbgath".to_string(),
            version: 1,
            amplitude_trace_bytes: (volume.shape[2] * std::mem::size_of::<f32>()) as u64,
            volume,
            layout,
            gather_axis_kind,
            gather_axis_values,
            sample_type: "f32".to_string(),
            endianness: "little".to_string(),
        }
    }

    pub fn gather_count(&self) -> usize {
        self.gather_axis_values.len()
    }

    pub fn gather_grid_shape(&self) -> [usize; 2] {
        [self.volume.shape[0], self.volume.shape[1]]
    }

    pub fn gather_stride_values(&self) -> usize {
        self.gather_count() * self.volume.shape[2]
    }

    pub fn gather_stride_bytes(&self) -> u64 {
        (self.gather_stride_values() * std::mem::size_of::<f32>()) as u64
    }

    pub fn total_gathers(&self) -> usize {
        self.volume.shape[0] * self.volume.shape[1]
    }

    pub fn total_values(&self) -> usize {
        self.total_gathers() * self.gather_stride_values()
    }

    pub fn total_amplitude_bytes(&self) -> u64 {
        self.total_values() as u64 * std::mem::size_of::<f32>() as u64
    }

    pub fn gather_linear_index(&self, iline_index: usize, xline_index: usize) -> usize {
        iline_index * self.volume.shape[1] + xline_index
    }

    pub fn gather_byte_offset(&self, iline_index: usize, xline_index: usize) -> u64 {
        self.gather_linear_index(iline_index, xline_index) as u64 * self.gather_stride_bytes()
    }
}

pub struct TbgathReader {
    manifest: TbgathManifest,
    amplitude_map: Mmap,
}

impl TbgathReader {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SeismicStoreError> {
        let root = root.as_ref();
        let manifest_path = root.join(MANIFEST_FILE);
        let mut manifest = serde_json::from_slice::<TbgathManifest>(&fs::read(&manifest_path)?)?;
        let mut changed = false;
        if normalize_source_identity(&mut manifest.volume.source) {
            changed = true;
        }
        if normalize_volume_axes(&mut manifest.volume.axes) {
            changed = true;
        }
        if changed {
            fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
        }
        validate_manifest(&manifest)?;

        let amplitude_file = File::open(root.join(AMPLITUDE_FILE))?;
        let amplitude_len = amplitude_file.metadata()?.len();
        let expected = manifest.total_amplitude_bytes();
        if amplitude_len != expected {
            return Err(SeismicStoreError::Message(format!(
                "tbgath amplitude size mismatch: expected {expected}, found {amplitude_len}"
            )));
        }
        let amplitude_map = unsafe { MmapOptions::new().map(&amplitude_file)? };
        Ok(Self {
            manifest,
            amplitude_map,
        })
    }

    pub fn manifest(&self) -> &TbgathManifest {
        &self.manifest
    }

    pub fn read_gather<'a>(
        &'a self,
        iline_index: usize,
        xline_index: usize,
    ) -> Result<&'a [f32], SeismicStoreError> {
        validate_gather_indices(&self.manifest, iline_index, xline_index)?;
        let offset = self.manifest.gather_byte_offset(iline_index, xline_index) as usize;
        let end = offset + self.manifest.gather_stride_bytes() as usize;
        bytes_as_f32_slice(&self.amplitude_map[offset..end])
    }
}

pub struct TbgathWriter {
    final_root: PathBuf,
    temp_root: PathBuf,
    manifest: TbgathManifest,
    amplitude_file: File,
}

impl TbgathWriter {
    pub fn create(
        root: impl AsRef<Path>,
        manifest: TbgathManifest,
    ) -> Result<Self, SeismicStoreError> {
        let final_root = root.as_ref().to_path_buf();
        if final_root.exists() {
            return Err(SeismicStoreError::StoreAlreadyExists(final_root));
        }
        validate_manifest(&manifest)?;

        let temp_root = final_root.with_extension("tbgath.tmp");
        if temp_root.exists() {
            fs::remove_dir_all(&temp_root)?;
        }
        fs::create_dir_all(&temp_root)?;

        let amplitude_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(temp_root.join(AMPLITUDE_FILE))?;
        amplitude_file.set_len(manifest.total_amplitude_bytes())?;

        Ok(Self {
            final_root,
            temp_root,
            manifest,
            amplitude_file,
        })
    }

    pub fn write_gather(
        &self,
        iline_index: usize,
        xline_index: usize,
        amplitudes: &[f32],
    ) -> Result<(), SeismicStoreError> {
        validate_gather_indices(&self.manifest, iline_index, xline_index)?;
        if amplitudes.len() != self.manifest.gather_stride_values() {
            return Err(SeismicStoreError::Message(format!(
                "tbgath gather length mismatch: expected {}, found {}",
                self.manifest.gather_stride_values(),
                amplitudes.len()
            )));
        }
        write_all_at(
            &self.amplitude_file,
            f32_slice_as_bytes(amplitudes),
            self.manifest.gather_byte_offset(iline_index, xline_index),
        )?;
        Ok(())
    }

    pub fn finalize(self) -> Result<(), SeismicStoreError> {
        self.amplitude_file.sync_all()?;
        let final_root = self.final_root.clone();
        let temp_root = self.temp_root.clone();
        let manifest = self.manifest.clone();
        let amplitude_len = fs::metadata(temp_root.join(AMPLITUDE_FILE))?.len();
        let expected = manifest.total_amplitude_bytes();
        if amplitude_len != expected {
            return Err(SeismicStoreError::Message(format!(
                "tbgath amplitude finalize size mismatch: expected {expected}, found {amplitude_len}"
            )));
        }
        fs::write(
            temp_root.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        drop(self);
        fs::rename(temp_root, final_root)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PrestackStoreHandle {
    pub root: PathBuf,
    pub manifest: TbgathManifest,
}

impl PrestackStoreHandle {
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    pub fn dataset_id(&self) -> DatasetId {
        DatasetId(dataset_id_string(&self.root))
    }

    pub fn volume_descriptor(&self) -> VolumeDescriptor {
        VolumeDescriptor {
            id: self.dataset_id(),
            store_id: self.manifest.volume.store_id.clone(),
            label: dataset_label(&self.root),
            shape: self.manifest.volume.shape,
            chunk_shape: [1, 1, self.manifest.volume.shape[2]],
            sample_interval_ms: self.manifest.volume.source.sample_interval_us as f32 / 1000.0,
            sample_data_fidelity: self.manifest.volume.source.sample_data_fidelity.clone(),
            geometry: self.geometry_descriptor(),
            coordinate_reference_binding: self.manifest.volume.coordinate_reference_binding.clone(),
            spatial: self.manifest.volume.spatial.clone(),
            processing_lineage_summary: processing_lineage_summary(
                self.manifest.volume.processing_lineage.as_ref(),
            ),
        }
    }

    pub fn read_gather_plane(
        &self,
        request: &GatherRequest,
    ) -> Result<GatherPlane, SeismicStoreError> {
        ensure_dataset_matches(self, &request.dataset_id.0)?;
        let reader = TbgathReader::open(&self.root)?;
        let (iline_index, xline_index) =
            resolve_gather_selector(&self.manifest, &request.selector)?;
        gather_plane_from_reader(self, &reader, iline_index, xline_index)
    }

    pub fn gather_view(&self, request: &GatherRequest) -> Result<GatherView, SeismicStoreError> {
        let plane = self.read_gather_plane(request)?;
        Ok(self.gather_view_from_plane(&plane))
    }

    pub fn gather_view_from_plane(&self, plane: &GatherPlane) -> GatherView {
        GatherView {
            dataset_id: self.dataset_id(),
            label: plane.label.clone(),
            gather_axis_kind: plane.gather_axis_kind,
            sample_domain: plane.sample_domain,
            traces: plane.traces,
            samples: plane.samples,
            horizontal_axis_f64le: f64_vec_to_le_bytes(&plane.horizontal_axis),
            sample_axis_f32le: f32_vec_to_le_bytes(&plane.sample_axis_ms),
            amplitudes_f32le: f32_vec_to_le_bytes(&plane.amplitudes),
            units: Some(SectionUnits {
                horizontal: Some(gather_axis_label(self.manifest.gather_axis_kind).to_string()),
                sample: Some("ms".to_string()),
                amplitude: Some("amplitude".to_string()),
            }),
            metadata: Some(SectionMetadata {
                store_id: Some(self.manifest.volume.store_id.clone()),
                derived_from: self
                    .manifest
                    .volume
                    .processing_lineage
                    .as_ref()
                    .map(|lineage| lineage.parent_store.to_string_lossy().into_owned()),
                notes: vec![
                    format!("kind:{:?}", self.manifest.volume.kind),
                    format!("layout:{:?}", self.manifest.layout),
                ],
            }),
            display_defaults: Some(SectionDisplayDefaults {
                gain: 1.0,
                clip_min: None,
                clip_max: None,
                render_mode: SectionRenderMode::Heatmap,
                colormap: SectionColorMap::Grayscale,
                polarity: SectionPolarity::Normal,
            }),
        }
    }

    fn geometry_descriptor(&self) -> GeometryDescriptor {
        GeometryDescriptor {
            compare_family: GEOMETRY_COMPARE_FAMILY.to_string(),
            fingerprint: geometry_fingerprint(&self.manifest),
            summary: GeometrySummary {
                inline_axis: summarize_i32_axis(&self.manifest.volume.axes.ilines),
                xline_axis: summarize_i32_axis(&self.manifest.volume.axes.xlines),
                sample_axis: summarize_f32_axis(
                    &self.manifest.volume.axes.sample_axis_ms,
                    Some(self.manifest.volume.axes.sample_axis_unit.clone()),
                ),
                layout: Some(self.manifest.layout),
                gather_axis_kind: Some(self.manifest.gather_axis_kind),
                gather_axis: Some(summarize_f64_axis_as_f32(
                    &self.manifest.gather_axis_values,
                    Some(gather_axis_label(self.manifest.gather_axis_kind).to_string()),
                )),
                provenance: geometry_provenance_summary(&self.manifest.volume),
            },
        }
    }
}

fn processing_lineage_summary(
    lineage: Option<&ProcessingLineage>,
) -> Option<ProcessingLineageSummary> {
    let lineage = lineage?;
    let (pipeline_name, pipeline_revision) = match &lineage.pipeline {
        ProcessingPipelineSpec::TraceLocal { pipeline } => {
            (pipeline.name.clone(), pipeline.revision)
        }
        ProcessingPipelineSpec::PostStackNeighborhood { pipeline } => {
            (pipeline.name.clone(), pipeline.revision)
        }
        ProcessingPipelineSpec::Subvolume { pipeline } => {
            (pipeline.name.clone(), pipeline.revision)
        }
        ProcessingPipelineSpec::Gather { pipeline } => (pipeline.name.clone(), pipeline.revision),
    };
    Some(ProcessingLineageSummary {
        parent_store_path: lineage.parent_store.to_string_lossy().into_owned(),
        parent_store_id: lineage.parent_store_id.clone(),
        artifact_role: lineage.artifact_role,
        pipeline_family: lineage.pipeline.family(),
        pipeline_name: pipeline_name.filter(|value| !value.trim().is_empty()),
        pipeline_revision,
    })
}

pub fn create_tbgath_store(
    root: impl AsRef<Path>,
    manifest: TbgathManifest,
    amplitudes: &[f32],
) -> Result<PrestackStoreHandle, SeismicStoreError> {
    if amplitudes.len() != manifest.total_values() {
        return Err(SeismicStoreError::Message(format!(
            "tbgath value length mismatch: expected {}, found {}",
            manifest.total_values(),
            amplitudes.len()
        )));
    }
    let writer = TbgathWriter::create(&root, manifest.clone())?;
    let gather_stride = manifest.gather_stride_values();
    for iline_index in 0..manifest.volume.shape[0] {
        for xline_index in 0..manifest.volume.shape[1] {
            let linear_index = manifest.gather_linear_index(iline_index, xline_index);
            let start = linear_index * gather_stride;
            let end = start + gather_stride;
            writer.write_gather(iline_index, xline_index, &amplitudes[start..end])?;
        }
    }
    writer.finalize()?;
    open_prestack_store(root)
}

pub fn open_prestack_store(
    root: impl AsRef<Path>,
) -> Result<PrestackStoreHandle, SeismicStoreError> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = root.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return Err(SeismicStoreError::MissingManifest(manifest_path));
    }
    let manifest = serde_json::from_slice::<TbgathManifest>(&fs::read(&manifest_path)?)?;
    validate_manifest(&manifest)?;
    Ok(PrestackStoreHandle { root, manifest })
}

pub fn describe_prestack_store(
    root: impl AsRef<Path>,
) -> Result<VolumeDescriptor, SeismicStoreError> {
    Ok(open_prestack_store(root)?.volume_descriptor())
}

pub fn set_prestack_store_native_coordinate_reference(
    root: impl AsRef<Path>,
    coordinate_reference_id: Option<&str>,
    coordinate_reference_name: Option<&str>,
) -> Result<VolumeDescriptor, SeismicStoreError> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = root.join(MANIFEST_FILE);
    let mut manifest = serde_json::from_slice::<TbgathManifest>(&fs::read(&manifest_path)?)?;
    manifest.volume.coordinate_reference_binding = apply_native_coordinate_reference_override(
        manifest.volume.coordinate_reference_binding.take(),
        manifest.volume.spatial.as_mut(),
        coordinate_reference_id,
        coordinate_reference_name,
    );
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(PrestackStoreHandle { root, manifest }.volume_descriptor())
}

pub fn read_gather_plane(
    root: impl AsRef<Path>,
    request: &GatherRequest,
) -> Result<GatherPlane, SeismicStoreError> {
    open_prestack_store(root)?.read_gather_plane(request)
}

pub fn gather_view(
    root: impl AsRef<Path>,
    request: &GatherRequest,
) -> Result<GatherView, SeismicStoreError> {
    open_prestack_store(root)?.gather_view(request)
}

pub fn preview_gather_processing_view(
    root: impl AsRef<Path>,
    request: &GatherRequest,
    pipeline: &ophiolite_seismic::GatherProcessingPipeline,
) -> Result<GatherPreviewView, SeismicStoreError> {
    let handle = open_prestack_store(root)?;
    let mut gather = handle.read_gather_plane(request)?;
    apply_gather_processing_pipeline(&mut gather, pipeline)?;
    Ok(GatherPreviewView {
        gather: handle.gather_view_from_plane(&gather),
        processing_label: gather_pipeline_label(pipeline),
        preview_ready: true,
    })
}

pub fn materialize_gather_processing_store(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
    pipeline: &ophiolite_seismic::GatherProcessingPipeline,
) -> Result<PrestackStoreHandle, SeismicStoreError> {
    materialize_gather_processing_store_with_progress(input_root, output_root, pipeline, |_, _| {
        Ok(())
    })
}

pub fn materialize_gather_processing_store_with_progress<F>(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
    pipeline: &ophiolite_seismic::GatherProcessingPipeline,
    mut on_progress: F,
) -> Result<PrestackStoreHandle, SeismicStoreError>
where
    F: FnMut(usize, usize) -> Result<(), SeismicStoreError>,
{
    let input = open_prestack_store(input_root)?;
    crate::gather_processing::validate_gather_processing_pipeline_for_layout(
        pipeline,
        input.manifest.layout,
    )?;

    let reader = TbgathReader::open(&input.root)?;
    let derived_manifest = derived_manifest(&input, pipeline);
    let writer = TbgathWriter::create(&output_root, derived_manifest)?;
    let total = input.manifest.total_gathers();
    let mut completed = 0usize;

    for iline_index in 0..input.manifest.volume.shape[0] {
        for xline_index in 0..input.manifest.volume.shape[1] {
            let mut gather = gather_plane_from_reader(&input, &reader, iline_index, xline_index)?;
            apply_gather_processing_pipeline(&mut gather, pipeline)?;
            writer.write_gather(iline_index, xline_index, &gather.amplitudes)?;
            completed += 1;
            on_progress(completed, total)?;
        }
    }

    writer.finalize()?;
    open_prestack_store(output_root)
}

fn derived_manifest(
    input: &PrestackStoreHandle,
    pipeline: &ophiolite_seismic::GatherProcessingPipeline,
) -> TbgathManifest {
    TbgathManifest::new(
        VolumeMetadata {
            kind: DatasetKind::Derived,
            store_id: generate_store_id(),
            source: input.manifest.volume.source.clone(),
            shape: input.manifest.volume.shape,
            axes: input.manifest.volume.axes.clone(),
            segy_export: None,
            coordinate_reference_binding: input
                .manifest
                .volume
                .coordinate_reference_binding
                .clone(),
            spatial: input.manifest.volume.spatial.clone(),
            created_by: "ophiolite-seismic-runtime-0.1.0".to_string(),
            processing_lineage: Some(ProcessingLineage {
                parent_store: input.root.clone(),
                parent_store_id: input.manifest.volume.store_id.clone(),
                artifact_role: ProcessingArtifactRole::FinalOutput,
                pipeline: ProcessingPipelineSpec::Gather {
                    pipeline: pipeline.clone(),
                },
                runtime_version: "ophiolite-seismic-runtime-0.1.0".to_string(),
                created_at_unix_s: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }),
        },
        input.manifest.layout,
        input.manifest.gather_axis_kind,
        input.manifest.gather_axis_values.clone(),
    )
}

fn gather_plane_from_reader(
    handle: &PrestackStoreHandle,
    reader: &TbgathReader,
    iline_index: usize,
    xline_index: usize,
) -> Result<GatherPlane, SeismicStoreError> {
    let amplitudes = reader.read_gather(iline_index, xline_index)?.to_vec();
    Ok(GatherPlane {
        label: format!(
            "inline {} xline {}",
            handle.manifest.volume.axes.ilines[iline_index] as i32,
            handle.manifest.volume.axes.xlines[xline_index] as i32
        ),
        gather_axis_kind: handle.manifest.gather_axis_kind,
        sample_domain: GatherSampleDomain::Time,
        traces: handle.manifest.gather_count(),
        samples: handle.manifest.volume.shape[2],
        horizontal_axis: handle.manifest.gather_axis_values.clone(),
        sample_axis_ms: handle.manifest.volume.axes.sample_axis_ms.clone(),
        amplitudes,
    })
}

fn resolve_gather_selector(
    manifest: &TbgathManifest,
    selector: &GatherSelector,
) -> Result<(usize, usize), SeismicStoreError> {
    match selector {
        GatherSelector::InlineXline { inline, xline } => {
            let iline_index = manifest
                .volume
                .axes
                .ilines
                .iter()
                .position(|value| (*value as i32) == *inline)
                .ok_or_else(|| {
                    SeismicStoreError::Message(format!("unknown inline selector: {inline}"))
                })?;
            let xline_index = manifest
                .volume
                .axes
                .xlines
                .iter()
                .position(|value| (*value as i32) == *xline)
                .ok_or_else(|| {
                    SeismicStoreError::Message(format!("unknown xline selector: {xline}"))
                })?;
            Ok((iline_index, xline_index))
        }
        GatherSelector::Coordinate { coordinate } => Err(SeismicStoreError::Message(format!(
            "coordinate gather selection is not supported for {:?} prestack stores; use inline/xline or ordinal, received {coordinate}",
            manifest.layout
        ))),
        GatherSelector::Ordinal { index } => {
            let total = manifest.total_gathers();
            if *index >= total {
                return Err(SeismicStoreError::Message(format!(
                    "gather ordinal {index} is out of bounds for {total} gathers"
                )));
            }
            Ok((
                index / manifest.volume.shape[1],
                index % manifest.volume.shape[1],
            ))
        }
    }
}

fn ensure_dataset_matches(
    handle: &PrestackStoreHandle,
    expected_dataset_id: &str,
) -> Result<(), SeismicStoreError> {
    let actual = handle.dataset_id().0;
    if actual != expected_dataset_id {
        return Err(SeismicStoreError::DatasetIdMismatch {
            expected: expected_dataset_id.to_string(),
            found: actual,
        });
    }
    Ok(())
}

fn validate_manifest(manifest: &TbgathManifest) -> Result<(), SeismicStoreError> {
    if manifest.format != "tbgath" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbgath format marker: {}",
            manifest.format
        )));
    }
    if manifest.endianness != "little" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbgath endianness: {}",
            manifest.endianness
        )));
    }
    if manifest.sample_type != "f32" {
        return Err(SeismicStoreError::Message(format!(
            "unsupported tbgath sample type: {}",
            manifest.sample_type
        )));
    }
    if manifest.layout != SeismicLayout::PreStack3DOffset {
        return Err(SeismicStoreError::Message(format!(
            "phase-one prestack store only supports {:?}, found {:?}",
            SeismicLayout::PreStack3DOffset,
            manifest.layout
        )));
    }
    if manifest.gather_axis_kind != GatherAxisKind::Offset {
        return Err(SeismicStoreError::Message(format!(
            "phase-one prestack store only supports offset gathers, found {:?}",
            manifest.gather_axis_kind
        )));
    }
    if manifest.volume.axes.ilines.len() != manifest.volume.shape[0] {
        return Err(SeismicStoreError::Message(format!(
            "iline axis length mismatch: expected {}, found {}",
            manifest.volume.shape[0],
            manifest.volume.axes.ilines.len()
        )));
    }
    if manifest.volume.axes.xlines.len() != manifest.volume.shape[1] {
        return Err(SeismicStoreError::Message(format!(
            "xline axis length mismatch: expected {}, found {}",
            manifest.volume.shape[1],
            manifest.volume.axes.xlines.len()
        )));
    }
    validate_vertical_axis(
        &manifest.volume.axes.sample_axis_ms,
        manifest.volume.shape[2],
        "sample axis",
    )
    .map_err(SeismicStoreError::Message)?;
    if manifest.gather_axis_values.is_empty() {
        return Err(SeismicStoreError::Message(
            "prestack gather axis must contain at least one offset".to_string(),
        ));
    }
    Ok(())
}

fn validate_gather_indices(
    manifest: &TbgathManifest,
    iline_index: usize,
    xline_index: usize,
) -> Result<(), SeismicStoreError> {
    if iline_index >= manifest.volume.shape[0] {
        return Err(SeismicStoreError::Message(format!(
            "inline gather index {iline_index} is out of bounds for {} inlines",
            manifest.volume.shape[0]
        )));
    }
    if xline_index >= manifest.volume.shape[1] {
        return Err(SeismicStoreError::Message(format!(
            "xline gather index {xline_index} is out of bounds for {} xlines",
            manifest.volume.shape[1]
        )));
    }
    Ok(())
}

fn geometry_provenance_summary(volume: &VolumeMetadata) -> GeometryProvenanceSummary {
    if volume.source.regularization.is_some() {
        GeometryProvenanceSummary::Regularized
    } else {
        match volume.kind {
            DatasetKind::Source => GeometryProvenanceSummary::Source,
            DatasetKind::Derived => GeometryProvenanceSummary::Derived,
        }
    }
}

fn summarize_i32_axis(values: &[f64]) -> AxisSummaryI32 {
    let first = values.first().copied().unwrap_or_default() as i32;
    let last = values.last().copied().unwrap_or_default() as i32;
    let step = axis_step(values).map(|step| step as i32);
    AxisSummaryI32 {
        count: values.len(),
        first,
        last,
        step,
        regular: step.is_some(),
    }
}

fn summarize_f32_axis(values: &[f32], units: Option<String>) -> AxisSummaryF32 {
    let first = values.first().copied().unwrap_or_default();
    let last = values.last().copied().unwrap_or_default();
    let step = if values.len() < 2 {
        None
    } else {
        let candidate = values[1] - values[0];
        values
            .windows(2)
            .all(|window| (window[1] - window[0] - candidate).abs() < 1.0e-6)
            .then_some(candidate)
    };
    AxisSummaryF32 {
        count: values.len(),
        first,
        last,
        step,
        regular: step.is_some(),
        units,
    }
}

fn summarize_f64_axis_as_f32(values: &[f64], units: Option<String>) -> AxisSummaryF32 {
    let first = values.first().copied().unwrap_or_default() as f32;
    let last = values.last().copied().unwrap_or_default() as f32;
    let step = axis_step(values).map(|step| step as f32);
    AxisSummaryF32 {
        count: values.len(),
        first,
        last,
        step,
        regular: step.is_some(),
        units,
    }
}

fn axis_step(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let candidate = values[1] - values[0];
    values
        .windows(2)
        .all(|window| (window[1] - window[0] - candidate).abs() < 1.0e-9)
        .then_some(candidate)
}

fn geometry_fingerprint(manifest: &TbgathManifest) -> String {
    let mut hash = fnv1a64_update(FNV_OFFSET, GEOMETRY_FINGERPRINT_VERSION.as_bytes());
    hash = fnv1a64_update_u64(hash, manifest.volume.shape[0] as u64);
    hash = fnv1a64_update_u64(hash, manifest.volume.shape[1] as u64);
    hash = fnv1a64_update_u64(hash, manifest.volume.shape[2] as u64);
    hash = fnv1a64_update_f64_slice(hash, &manifest.volume.axes.ilines);
    hash = fnv1a64_update_f64_slice(hash, &manifest.volume.axes.xlines);
    hash = fnv1a64_update_f32_slice(hash, &manifest.volume.axes.sample_axis_ms);
    hash = fnv1a64_update_f64_slice(hash, &manifest.gather_axis_values);
    hash = fnv1a64_update_u64(hash, manifest.layout as u64);
    hash = fnv1a64_update_u64(hash, manifest.gather_axis_kind as u64);
    format!("{hash:016x}")
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn fnv1a64_update_u64(hash: u64, value: u64) -> u64 {
    fnv1a64_update(hash, &value.to_le_bytes())
}

fn fnv1a64_update_f64_slice(mut hash: u64, values: &[f64]) -> u64 {
    hash = fnv1a64_update_u64(hash, values.len() as u64);
    for value in values {
        hash = fnv1a64_update(hash, &value.to_le_bytes());
    }
    hash
}

fn fnv1a64_update_f32_slice(mut hash: u64, values: &[f32]) -> u64 {
    hash = fnv1a64_update_u64(hash, values.len() as u64);
    for value in values {
        hash = fnv1a64_update(hash, &value.to_le_bytes());
    }
    hash
}

fn gather_pipeline_label(pipeline: &ophiolite_seismic::GatherProcessingPipeline) -> String {
    if let Some(name) = pipeline
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return name.to_string();
    }

    let mut parts = Vec::new();
    if let Some(trace_local) = &pipeline.trace_local_pipeline {
        for operation in trace_local.operations() {
            parts.push(operation.operator_id().to_string());
        }
    }
    for operation in &pipeline.operations {
        parts.push(operation.operator_id().to_string());
    }

    if parts.is_empty() {
        "gather-processing".to_string()
    } else {
        parts.join("__")
    }
}

fn gather_axis_label(kind: GatherAxisKind) -> &'static str {
    match kind {
        GatherAxisKind::Offset => "offset",
        GatherAxisKind::Angle => "angle",
        GatherAxisKind::Azimuth => "azimuth",
        GatherAxisKind::Shot => "shot",
        GatherAxisKind::Receiver => "receiver",
        GatherAxisKind::Cmp => "cmp",
        GatherAxisKind::TraceOrdinal => "trace",
        GatherAxisKind::Unknown => "gather",
    }
}

fn dataset_leaf_name(root: &Path) -> String {
    let raw = root.to_string_lossy();
    raw.rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| raw.into_owned())
}

fn dataset_id_string(root: &Path) -> String {
    dataset_leaf_name(root)
}

fn dataset_label(root: &Path) -> String {
    let leaf = dataset_leaf_name(root);
    leaf.trim_end_matches(".tbvol")
        .trim_end_matches(".tbgath")
        .to_string()
}

fn f64_vec_to_le_bytes(values: &[f64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f64>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn f32_vec_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn bytes_as_f32_slice(bytes: &[u8]) -> Result<&[f32], SeismicStoreError> {
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(SeismicStoreError::Message(format!(
            "tbgath amplitude byte length is not f32 aligned: {}",
            bytes.len()
        )));
    }
    let (prefix, aligned, suffix) = unsafe { bytes.align_to::<f32>() };
    if !prefix.is_empty() || !suffix.is_empty() {
        return Err(SeismicStoreError::Message(
            "tbgath amplitude mapping is not aligned to f32".to_string(),
        ));
    }
    Ok(aligned)
}

fn f32_slice_as_bytes(values: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

#[cfg(unix)]
fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;

    while !bytes.is_empty() {
        let written = file.write_at(bytes, offset)?;
        if written == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "failed to write tbgath bytes",
            ));
        }
        bytes = &bytes[written..];
        offset += written as u64;
    }
    Ok(())
}

#[cfg(windows)]
fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;

    while !bytes.is_empty() {
        let written = file.seek_write(bytes, offset)?;
        if written == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "failed to write tbgath bytes",
            ));
        }
        bytes = &bytes[written..];
        offset += written as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::generate_store_id;
    use crate::metadata::{GeometryProvenance, HeaderFieldSpec, SourceIdentity, VolumeAxes};
    use tempfile::tempdir;

    fn fixture_manifest() -> TbgathManifest {
        TbgathManifest::new(
            VolumeMetadata {
                kind: DatasetKind::Source,
                store_id: generate_store_id(),
                source: SourceIdentity {
                    source_path: PathBuf::from("input-prestack.sgy"),
                    file_size: 1,
                    trace_count: 12,
                    samples_per_trace: 4,
                    sample_interval_us: 2000,
                    sample_format_code: 5,
                    sample_data_fidelity: crate::metadata::segy_sample_data_fidelity(5),
                    endianness: "big".to_string(),
                    revision_raw: 0,
                    fixed_length_trace_flag_raw: 1,
                    extended_textual_headers: 0,
                    geometry: GeometryProvenance {
                        inline_field: HeaderFieldSpec {
                            name: "INLINE_3D".to_string(),
                            start_byte: 189,
                            value_type: "I32".to_string(),
                        },
                        crossline_field: HeaderFieldSpec {
                            name: "CROSSLINE_3D".to_string(),
                            start_byte: 193,
                            value_type: "I32".to_string(),
                        },
                        third_axis_field: Some(HeaderFieldSpec {
                            name: "OFFSET".to_string(),
                            start_byte: 37,
                            value_type: "I32".to_string(),
                        }),
                    },
                    regularization: None,
                },
                shape: [2, 2, 4],
                axes: VolumeAxes::from_time_axis(
                    vec![100.0, 101.0],
                    vec![200.0, 201.0],
                    vec![0.0, 2.0, 4.0, 6.0],
                ),
                segy_export: None,
                coordinate_reference_binding: None,
                spatial: None,
                created_by: "test".to_string(),
                processing_lineage: None,
            },
            SeismicLayout::PreStack3DOffset,
            GatherAxisKind::Offset,
            vec![-500.0, 0.0, 500.0],
        )
    }

    fn fixture_data(manifest: &TbgathManifest) -> Vec<f32> {
        let mut values = vec![0.0_f32; manifest.total_values()];
        let gather_stride = manifest.gather_stride_values();
        for iline_index in 0..manifest.volume.shape[0] {
            for xline_index in 0..manifest.volume.shape[1] {
                let gather_start =
                    manifest.gather_linear_index(iline_index, xline_index) * gather_stride;
                for offset_index in 0..manifest.gather_count() {
                    for sample_index in 0..manifest.volume.shape[2] {
                        let value = (iline_index as f32 * 1000.0)
                            + (xline_index as f32 * 100.0)
                            + (offset_index as f32 * 10.0)
                            + sample_index as f32;
                        values[gather_start
                            + offset_index * manifest.volume.shape[2]
                            + sample_index] = value;
                    }
                }
            }
        }
        values
    }

    #[test]
    fn prestack_descriptor_exposes_layout_and_gather_axis() {
        let handle = PrestackStoreHandle {
            root: PathBuf::from("/tmp/survey.tbgath"),
            manifest: fixture_manifest(),
        };
        let descriptor = handle.volume_descriptor();

        assert_eq!(descriptor.shape, [2, 2, 4]);
        assert_eq!(
            descriptor.geometry.summary.layout,
            Some(SeismicLayout::PreStack3DOffset)
        );
        assert_eq!(
            descriptor.geometry.summary.gather_axis_kind,
            Some(GatherAxisKind::Offset)
        );
        assert_eq!(
            descriptor
                .geometry
                .summary
                .gather_axis
                .as_ref()
                .map(|axis| axis.count),
            Some(3)
        );
    }

    #[test]
    fn read_gather_plane_returns_expected_trace_order() {
        let temp_dir = tempdir().expect("temp dir");
        let root = temp_dir.path().join("survey.tbgath");
        let manifest = fixture_manifest();
        let data = fixture_data(&manifest);
        create_tbgath_store(&root, manifest, &data).expect("store should be created");

        let request = GatherRequest {
            dataset_id: DatasetId("survey.tbgath".to_string()),
            selector: GatherSelector::InlineXline {
                inline: 101,
                xline: 200,
            },
        };
        let plane = read_gather_plane(&root, &request).expect("gather should be readable");

        assert_eq!(plane.traces, 3);
        assert_eq!(plane.samples, 4);
        assert_eq!(plane.horizontal_axis, vec![-500.0, 0.0, 500.0]);
        assert_eq!(plane.amplitudes[0], 1000.0);
        assert_eq!(plane.amplitudes[4], 1010.0);
        assert_eq!(plane.amplitudes[8], 1020.0);
    }

    #[test]
    fn materialize_gather_processing_store_applies_offset_mute() {
        let temp_dir = tempdir().expect("temp dir");
        let input_root = temp_dir.path().join("input.tbgath");
        let output_root = temp_dir.path().join("derived.tbgath");
        let manifest = fixture_manifest();
        let data = fixture_data(&manifest);
        create_tbgath_store(&input_root, manifest, &data).expect("input store should be created");

        let pipeline = ophiolite_seismic::GatherProcessingPipeline {
            schema_version: 1,
            revision: 1,
            preset_id: None,
            name: Some("mute-far".to_string()),
            description: None,
            trace_local_pipeline: None,
            operations: vec![ophiolite_seismic::GatherProcessingOperation::OffsetMute {
                min_offset: Some(-100.0),
                max_offset: Some(100.0),
            }],
        };
        let derived = materialize_gather_processing_store(&input_root, &output_root, &pipeline)
            .expect("derived store should materialize");

        assert_eq!(derived.manifest.volume.kind, DatasetKind::Derived);
        let request = GatherRequest {
            dataset_id: DatasetId("derived.tbgath".to_string()),
            selector: GatherSelector::Ordinal { index: 0 },
        };
        let plane = derived
            .read_gather_plane(&request)
            .expect("derived gather should read");
        assert!(plane.amplitudes[..4].iter().all(|value| *value == 0.0));
        assert!(plane.amplitudes[4..8].iter().any(|value| *value != 0.0));
        assert!(plane.amplitudes[8..12].iter().all(|value| *value == 0.0));
    }
}
