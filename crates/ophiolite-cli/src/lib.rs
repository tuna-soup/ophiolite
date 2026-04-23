mod operation_catalog;

use operation_catalog::operation_catalog;
use ophiolite_core::{LasError, Result};
use ophiolite_package::{StoredLasFile, write_bundle};
use ophiolite_parser::{ReadOptions, import_las_file};
use ophiolite_project::{
    AssetBindingInput, AssetId, OphioliteProject, ProjectComputeRunRequest,
    ProjectGatherProcessingPreviewRequest, ProjectGatherProcessingRunRequest,
    ProjectPostStackNeighborhoodProcessingPreviewRequest,
    ProjectPostStackNeighborhoodProcessingRunRequest, ProjectSubvolumeProcessingPreviewRequest,
    ProjectSubvolumeProcessingRunRequest, ProjectSurveyMapRequestDto,
    ProjectTopsSourceImportResult, ProjectTraceLocalProcessingPreviewRequest,
    ProjectTraceLocalProcessingRunRequest, ProjectVelocityScanRequest,
    SectionWellOverlayRequestDto, VendorProjectBridgeCommitRequest, VendorProjectBridgeRunRequest,
    VendorProjectCommitRequest, VendorProjectImportVendor, VendorProjectPlanRequest,
    VendorProjectRuntimeProbeRequest, VendorProjectScanRequest, WellId, WellPanelRequestDto,
    WellboreId, bridge_commit_vendor_project_object, commit_vendor_project_import,
    import_tops_source, plan_vendor_project_import, preview_well_folder_import,
    preview_well_source_import_sources, probe_vendor_project_runtime, run_vendor_project_bridge,
    scan_vendor_project, vendor_project_bridge_capabilities, vendor_project_connector_contract,
};
use ophiolite_seismic::{
    AvoInterceptGradientAttributeRequest, AvoReflectivityRequest, RockPhysicsAttributeRequest,
};
use ophiolite_seismic_runtime::{
    avo_intercept_gradient_attribute, avo_reflectivity, inspect_horizon_xyz_files,
    preview_horizon_source_import, rock_physics_attribute,
};
use serde_json::json;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub fn supported_cli_commands() -> &'static [&'static str] {
    &[
        "operation-catalog",
        "create-project",
        "open-project",
        "project-summary",
        "import-las-asset",
        "import-las-asset-with-binding",
        "import-tops-source-with-binding",
        "list-project-wells",
        "list-project-wellbores",
        "list-project-surveys",
        "resolve-well-panel-source",
        "resolve-survey-map-source",
        "resolve-wellbore-trajectory",
        "resolve-section-well-overlays",
        "project-operator-lock",
        "install-operator-package",
        "list-project-compute-catalog",
        "list-project-operator-catalog",
        "run-project-compute",
        "preview-project-trace-local-processing",
        "run-project-trace-local-processing",
        "preview-project-subvolume-processing",
        "run-project-subvolume-processing",
        "preview-project-post-stack-neighborhood-processing",
        "run-project-post-stack-neighborhood-processing",
        "preview-project-gather-processing",
        "run-project-gather-processing",
        "run-project-velocity-scan",
        "run-avo-reflectivity",
        "run-rock-physics-attribute",
        "run-avo-intercept-gradient-attribute",
        "preview-well-folder-import",
        "preview-well-source-import",
        "vendor-scan",
        "vendor-plan",
        "vendor-commit",
        "vendor-runtime-probe",
        "vendor-connector-contract",
        "vendor-bridge-capabilities",
        "vendor-bridge-run",
        "vendor-bridge-commit",
        "inspect-horizon-xyz",
        "preview-horizon-source-import",
        "import",
        "inspect-file",
        "summary",
        "list-curves",
        "examples",
        "generate-fixture-packages",
    ]
}

pub fn main_entry() {
    if let Err(err) = run(env::args()) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "operation-catalog" if args.len() == 2 => {
            println!("{}", serde_json::to_string_pretty(operation_catalog())?);
        }
        "create-project" if args.len() == 3 => {
            let project = OphioliteProject::create(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&project.summary()?)?);
        }
        "open-project" if args.len() == 3 => {
            let project = OphioliteProject::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&project.summary()?)?);
        }
        "project-summary" if args.len() == 3 => {
            let project = OphioliteProject::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&project.summary()?)?);
        }
        "import-las-asset" if args.len() == 4 || args.len() == 5 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let collection_name = args.get(4).map(String::as_str);
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.import_las(Path::new(&args[3]), collection_name)?
                )?
            );
        }
        "import-las-asset-with-binding" if args.len() == 5 || args.len() == 6 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let binding_json = read_json_argument(&args[4])?;
            let binding = serde_json::from_str::<AssetBindingInput>(&binding_json)?;
            let collection_name = args.get(5).map(String::as_str);
            println!(
                "{}",
                serde_json::to_string_pretty(&project.import_las_with_binding(
                    Path::new(&args[3]),
                    &binding,
                    collection_name
                )?)?
            );
        }
        "import-tops-source-with-binding" if args.len() >= 5 && args.len() <= 7 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let binding_json = read_json_argument(&args[4])?;
            let binding = serde_json::from_str::<AssetBindingInput>(&binding_json)?;
            let collection_name = args
                .get(5)
                .map(String::as_str)
                .filter(|value| !value.is_empty());
            let depth_reference = args.get(6).map(String::as_str);
            println!(
                "{}",
                serde_json::to_string_pretty(&import_tops_source_with_binding(
                    &mut project,
                    Path::new(&args[3]),
                    &binding,
                    collection_name,
                    depth_reference,
                )?)?
            );
        }
        "list-project-wells" if args.len() == 3 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.well_summaries()?)?
            );
        }
        "list-project-wellbores" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.wellbore_summaries(&WellId(args[3].clone()))?
                )?
            );
        }
        "list-project-surveys" if args.len() == 3 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.project_well_overlay_inventory()?.surveys)?
            );
        }
        "resolve-well-panel-source" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<WellPanelRequestDto>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.resolve_well_panel_source(&request)?)?
            );
        }
        "resolve-survey-map-source" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<ProjectSurveyMapRequestDto>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.resolve_survey_map_source(&request)?)?
            );
        }
        "resolve-wellbore-trajectory" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.resolve_wellbore_trajectory(&WellboreId(args[3].clone()))?
                )?
            );
        }
        "resolve-section-well-overlays" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<SectionWellOverlayRequestDto>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.resolve_section_well_overlays(&request)?)?
            );
        }
        "project-operator-lock" if args.len() == 3 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.operator_lock()?)?
            );
        }
        "install-operator-package" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.install_operator_package(&args[3])?)?
            );
        }
        "list-project-compute-catalog" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.list_compute_catalog(&AssetId(args[3].clone()))?
                )?
            );
        }
        "list-project-operator-catalog" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.list_operator_catalog(&AssetId(args[3].clone()))?
                )?
            );
        }
        "run-project-compute" if args.len() == 4 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<ProjectComputeRunRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.run_compute(&request)?)?
            );
        }
        "preview-project-trace-local-processing" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request =
                serde_json::from_str::<ProjectTraceLocalProcessingPreviewRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.preview_trace_local_processing(&request)?)?
            );
        }
        "run-project-trace-local-processing" if args.len() == 4 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request =
                serde_json::from_str::<ProjectTraceLocalProcessingRunRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.run_trace_local_processing(&request)?)?
            );
        }
        "preview-project-subvolume-processing" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request =
                serde_json::from_str::<ProjectSubvolumeProcessingPreviewRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.preview_subvolume_processing(&request)?)?
            );
        }
        "run-project-subvolume-processing" if args.len() == 4 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request =
                serde_json::from_str::<ProjectSubvolumeProcessingRunRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.run_subvolume_processing(&request)?)?
            );
        }
        "preview-project-post-stack-neighborhood-processing" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<
                ProjectPostStackNeighborhoodProcessingPreviewRequest,
            >(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.preview_post_stack_neighborhood_processing(&request)?
                )?
            );
        }
        "run-project-post-stack-neighborhood-processing" if args.len() == 4 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<ProjectPostStackNeighborhoodProcessingRunRequest>(
                &request_json,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &project.run_post_stack_neighborhood_processing(&request)?
                )?
            );
        }
        "preview-project-gather-processing" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request =
                serde_json::from_str::<ProjectGatherProcessingPreviewRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.preview_gather_processing(&request)?)?
            );
        }
        "run-project-gather-processing" if args.len() == 4 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<ProjectGatherProcessingRunRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.run_gather_processing(&request)?)?
            );
        }
        "run-project-velocity-scan" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<ProjectVelocityScanRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.run_velocity_scan(&request)?)?
            );
        }
        "run-avo-reflectivity" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<AvoReflectivityRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &avo_reflectivity(request)
                        .map_err(|error| LasError::Validation(error.to_string()))?
                )?
            );
        }
        "run-rock-physics-attribute" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<RockPhysicsAttributeRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &rock_physics_attribute(request)
                        .map_err(|error| LasError::Validation(error.to_string()))?
                )?
            );
        }
        "run-avo-intercept-gradient-attribute" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request =
                serde_json::from_str::<AvoInterceptGradientAttributeRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &avo_intercept_gradient_attribute(request)
                        .map_err(|error| LasError::Validation(error.to_string()))?
                )?
            );
        }
        "preview-well-folder-import" if args.len() == 3 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&preview_well_folder_import(Path::new(&args[2]))?)?
            );
        }
        "preview-well-source-import" if args.len() >= 4 => {
            let source_root = Path::new(&args[2]);
            let source_paths = args[3..].iter().map(PathBuf::from).collect::<Vec<_>>();
            println!(
                "{}",
                serde_json::to_string_pretty(&preview_well_source_import_sources(
                    &source_paths,
                    Some(source_root),
                )?)?
            );
        }
        "vendor-scan" if args.len() == 4 => {
            let vendor = parse_vendor_arg(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&scan_vendor_project(&VendorProjectScanRequest {
                    vendor,
                    project_root: args[3].clone(),
                })?)?
            );
        }
        "vendor-plan" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<VendorProjectPlanRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&plan_vendor_project_import(&request)?)?
            );
        }
        "vendor-commit" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<VendorProjectCommitRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&commit_vendor_project_import(&request)?)?
            );
        }
        "vendor-runtime-probe" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<VendorProjectRuntimeProbeRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&probe_vendor_project_runtime(&request)?)?
            );
        }
        "vendor-connector-contract" if args.len() == 3 => {
            let vendor = parse_vendor_arg(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&vendor_project_connector_contract(vendor))?
            );
        }
        "vendor-bridge-capabilities" if args.len() == 3 => {
            let vendor = parse_vendor_arg(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&vendor_project_bridge_capabilities(vendor))?
            );
        }
        "vendor-bridge-run" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<VendorProjectBridgeRunRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&run_vendor_project_bridge(&request)?)?
            );
        }
        "vendor-bridge-commit" if args.len() == 3 => {
            let request_json = read_json_argument(&args[2])?;
            let request = serde_json::from_str::<VendorProjectBridgeCommitRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&bridge_commit_vendor_project_object(&request)?)?
            );
        }
        "inspect-horizon-xyz" if args.len() >= 3 => {
            let input_paths = args[2..].iter().map(Path::new).collect::<Vec<_>>();
            let preview = inspect_horizon_xyz_files(&input_paths)
                .map_err(|error| LasError::Validation(error.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&preview)?);
        }
        "preview-horizon-source-import" if args.len() >= 4 => {
            let store_path = Path::new(&args[2]);
            let input_paths = args[3..].iter().map(Path::new).collect::<Vec<_>>();
            let preview = preview_horizon_source_import(store_path, &input_paths, None)
                .map_err(|error| LasError::Validation(error.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&preview)?);
        }
        "import" if args.len() == 4 => {
            let file = import_las_file(&args[2])?;
            let bundle = write_bundle(&file, &args[3])?;
            println!("{}", serde_json::to_string_pretty(bundle.summary())?);
        }
        "inspect-file" if args.len() == 3 => {
            let file = import_las_file(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "summary": file.summary,
                    "encoding": file.encoding,
                    "index": file.index,
                    "issues": file.issues,
                    "curves": file.keys(),
                }))?
            );
        }
        "summary" if args.len() == 3 => {
            let bundle = StoredLasFile::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(bundle.summary())?);
        }
        "list-curves" if args.len() == 3 => {
            let bundle = StoredLasFile::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&bundle.file().keys())?);
        }
        "examples" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "root": ophiolite_parser::examples::path(""),
                    "options": ReadOptions::default(),
                }))?
            );
        }
        "generate-fixture-packages" if args.len() == 4 => {
            let generated = generate_fixture_packages(&args[2], &args[3])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "input_root": args[2],
                    "output_root": args[3],
                    "generated_packages": generated,
                }))?
            );
        }
        _ => print_usage(),
    }

    Ok(())
}

pub fn generate_fixture_packages(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<Vec<PathBuf>> {
    let input_root = input_root.as_ref();
    let output_root = output_root.as_ref();

    fs::create_dir_all(output_root)?;

    let mut queue = vec![input_root.to_path_buf()];
    let mut las_files = Vec::new();

    while let Some(dir) = queue.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(input_root).unwrap_or(&path);
            let first_component = relative
                .components()
                .next()
                .and_then(|component| component.as_os_str().to_str());

            if path.is_dir() {
                if matches!(first_component, Some("packages" | "3.0")) {
                    continue;
                }
                queue.push(path);
                continue;
            }

            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("las"))
                && !matches!(first_component, Some("packages" | "3.0"))
            {
                las_files.push(path);
            }
        }
    }

    las_files.sort();

    let mut generated = Vec::with_capacity(las_files.len());
    for las_path in las_files {
        let relative = las_path
            .strip_prefix(input_root)
            .map_err(|err| ophiolite_core::LasError::Validation(err.to_string()))?;
        let mut package_root = output_root.join(relative);
        package_root.set_extension("laspkg");

        if let Some(parent) = package_root.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = import_las_file(&las_path)?;
        write_bundle(&file, &package_root)?;
        generated.push(package_root);
    }

    generated.sort();
    Ok(generated)
}

fn print_usage() {
    let binary = env::args()
        .next()
        .and_then(|path| {
            Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| String::from("ophiolite"));

    eprintln!("Usage:");
    eprintln!("  {binary} operation-catalog");
    eprintln!("  {binary} create-project <project_root>");
    eprintln!("  {binary} open-project <project_root>");
    eprintln!("  {binary} project-summary <project_root>");
    eprintln!("  {binary} import-las-asset <project_root> <las_path> [collection_name]");
    eprintln!(
        "  {binary} import-las-asset-with-binding <project_root> <las_path> <binding_json_path|-> [collection_name]"
    );
    eprintln!(
        "  {binary} import-tops-source-with-binding <project_root> <tops_source_path> <binding_json_path|-> [collection_name] [depth_reference]"
    );
    eprintln!("  {binary} list-project-wells <project_root>");
    eprintln!("  {binary} list-project-wellbores <project_root> <well_id>");
    eprintln!("  {binary} list-project-surveys <project_root>");
    eprintln!("  {binary} resolve-well-panel-source <project_root> <request_json_path|->");
    eprintln!("  {binary} resolve-survey-map-source <project_root> <request_json_path|->");
    eprintln!("  {binary} resolve-wellbore-trajectory <project_root> <wellbore_id>");
    eprintln!("  {binary} resolve-section-well-overlays <project_root> <request_json_path|->");
    eprintln!("  {binary} project-operator-lock <project_root>");
    eprintln!("  {binary} install-operator-package <project_root> <manifest_path>");
    eprintln!("  {binary} list-project-compute-catalog <project_root> <asset_id>");
    eprintln!("  {binary} list-project-operator-catalog <project_root> <asset_id>");
    eprintln!("  {binary} run-project-compute <project_root> <request_json_path|->");
    eprintln!(
        "  {binary} preview-project-trace-local-processing <project_root> <request_json_path|->"
    );
    eprintln!("  {binary} run-project-trace-local-processing <project_root> <request_json_path|->");
    eprintln!(
        "  {binary} preview-project-subvolume-processing <project_root> <request_json_path|->"
    );
    eprintln!("  {binary} run-project-subvolume-processing <project_root> <request_json_path|->");
    eprintln!(
        "  {binary} preview-project-post-stack-neighborhood-processing <project_root> <request_json_path|->"
    );
    eprintln!(
        "  {binary} run-project-post-stack-neighborhood-processing <project_root> <request_json_path|->"
    );
    eprintln!("  {binary} preview-project-gather-processing <project_root> <request_json_path|->");
    eprintln!("  {binary} run-project-gather-processing <project_root> <request_json_path|->");
    eprintln!("  {binary} run-project-velocity-scan <project_root> <request_json_path|->");
    eprintln!("  {binary} run-avo-reflectivity <request_json_path|->");
    eprintln!("  {binary} run-rock-physics-attribute <request_json_path|->");
    eprintln!("  {binary} run-avo-intercept-gradient-attribute <request_json_path|->");
    eprintln!("  {binary} preview-well-folder-import <folder_path>");
    eprintln!("  {binary} preview-well-source-import <source_root_path> <source_path>...");
    eprintln!("  {binary} vendor-scan <vendor> <project_root>");
    eprintln!("  {binary} vendor-plan <request_json_path|->");
    eprintln!("  {binary} vendor-commit <request_json_path|->");
    eprintln!("  {binary} vendor-runtime-probe <request_json_path|->");
    eprintln!("  {binary} vendor-connector-contract <vendor>");
    eprintln!("  {binary} vendor-bridge-capabilities <vendor>");
    eprintln!("  {binary} vendor-bridge-run <request_json_path|->");
    eprintln!("  {binary} vendor-bridge-commit <request_json_path|->");
    eprintln!("  {binary} inspect-horizon-xyz <input.xyz>...");
    eprintln!("  {binary} preview-horizon-source-import <store_path> <input.xyz>...");
    eprintln!("  {binary} import <input.las> <bundle_dir>");
    eprintln!("  {binary} inspect-file <input.las>");
    eprintln!("  {binary} summary <bundle_dir>");
    eprintln!("  {binary} list-curves <bundle_dir>");
    eprintln!("  {binary} generate-fixture-packages <input_root> <output_root>");
}

fn import_tops_source_with_binding(
    project: &mut OphioliteProject,
    source_path: &Path,
    binding: &AssetBindingInput,
    collection_name: Option<&str>,
    depth_reference: Option<&str>,
) -> Result<ProjectTopsSourceImportResult> {
    import_tops_source(
        project,
        source_path,
        binding,
        collection_name,
        depth_reference,
    )
}

fn read_json_argument(argument: &str) -> Result<String> {
    if argument == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        return Ok(buffer);
    }
    Ok(fs::read_to_string(argument)?)
}

fn parse_vendor_arg(argument: &str) -> Result<VendorProjectImportVendor> {
    match argument.trim().to_ascii_lowercase().as_str() {
        "opendtect" => Ok(VendorProjectImportVendor::Opendtect),
        "petrel" => Ok(VendorProjectImportVendor::Petrel),
        _ => Err(LasError::Validation(format!(
            "Unsupported vendor `{argument}`. Supported vendors: opendtect, petrel."
        ))),
    }
}
