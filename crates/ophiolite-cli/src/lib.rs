mod operation_catalog;

use operation_catalog::operation_catalog;
use ophiolite_core::{LasError, Result};
use ophiolite_package::{StoredLasFile, write_bundle};
use ophiolite_parser::{ReadOptions, import_las_file};
use ophiolite_project::{AssetId, OphioliteProject, ProjectComputeRunRequest, WellId};
use ophiolite_seismic::{
    AvoInterceptGradientAttributeRequest, AvoReflectivityRequest, RockPhysicsAttributeRequest,
};
use ophiolite_seismic_runtime::{
    avo_intercept_gradient_attribute, avo_reflectivity, rock_physics_attribute,
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
        "list-project-wells",
        "list-project-wellbores",
        "project-operator-lock",
        "install-operator-package",
        "list-project-compute-catalog",
        "run-project-compute",
        "run-avo-reflectivity",
        "run-rock-physics-attribute",
        "run-avo-intercept-gradient-attribute",
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
        "list-project-wells" if args.len() == 3 => {
            let project = OphioliteProject::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&project.list_wells()?)?);
        }
        "list-project-wellbores" if args.len() == 4 => {
            let project = OphioliteProject::open(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.list_wellbores(&WellId(args[3].clone()))?)?
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
        "run-project-compute" if args.len() == 4 => {
            let mut project = OphioliteProject::open(&args[2])?;
            let request_json = read_json_argument(&args[3])?;
            let request = serde_json::from_str::<ProjectComputeRunRequest>(&request_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project.run_compute(&request)?)?
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
    eprintln!("  {binary} list-project-wells <project_root>");
    eprintln!("  {binary} list-project-wellbores <project_root> <well_id>");
    eprintln!("  {binary} project-operator-lock <project_root>");
    eprintln!("  {binary} install-operator-package <project_root> <manifest_path>");
    eprintln!("  {binary} list-project-compute-catalog <project_root> <asset_id>");
    eprintln!("  {binary} run-project-compute <project_root> <request_json_path|->");
    eprintln!("  {binary} run-avo-reflectivity <request_json_path|->");
    eprintln!("  {binary} run-rock-physics-attribute <request_json_path|->");
    eprintln!("  {binary} run-avo-intercept-gradient-attribute <request_json_path|->");
    eprintln!("  {binary} import <input.las> <bundle_dir>");
    eprintln!("  {binary} inspect-file <input.las>");
    eprintln!("  {binary} summary <bundle_dir>");
    eprintln!("  {binary} list-curves <bundle_dir>");
    eprintln!("  {binary} generate-fixture-packages <input_root> <output_root>");
}

fn read_json_argument(argument: &str) -> Result<String> {
    if argument == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        return Ok(buffer);
    }
    Ok(fs::read_to_string(argument)?)
}
