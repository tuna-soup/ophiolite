use ophiolite_core::Result;
use ophiolite_project::{
    AssetBindingInput, LogAssetImportResult, OphioliteProject, ProjectAssetImportResult,
    SeismicAssetImportResult,
};
use std::path::Path;

pub fn import_las_asset(
    project: &mut OphioliteProject,
    las_path: impl AsRef<Path>,
    collection_name: Option<&str>,
) -> Result<LogAssetImportResult> {
    project.import_las(las_path, collection_name)
}

pub fn import_trajectory_csv_asset(
    project: &mut OphioliteProject,
    csv_path: impl AsRef<Path>,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
) -> Result<ProjectAssetImportResult> {
    project.import_trajectory_csv(csv_path, binding, collection_name)
}

pub fn import_tops_csv_asset(
    project: &mut OphioliteProject,
    csv_path: impl AsRef<Path>,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
) -> Result<ProjectAssetImportResult> {
    project.import_tops_csv(csv_path, binding, collection_name)
}

pub fn import_pressure_csv_asset(
    project: &mut OphioliteProject,
    csv_path: impl AsRef<Path>,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
) -> Result<ProjectAssetImportResult> {
    project.import_pressure_csv(csv_path, binding, collection_name)
}

pub fn import_drilling_csv_asset(
    project: &mut OphioliteProject,
    csv_path: impl AsRef<Path>,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
) -> Result<ProjectAssetImportResult> {
    project.import_drilling_csv(csv_path, binding, collection_name)
}

pub fn import_seismic_volume_store_asset(
    project: &mut OphioliteProject,
    store_root: impl AsRef<Path>,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
) -> Result<SeismicAssetImportResult> {
    project.import_seismic_volume_store(store_root, binding, collection_name)
}
