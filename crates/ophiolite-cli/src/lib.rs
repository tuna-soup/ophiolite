mod operation_catalog;

use operation_catalog::operation_catalog;
use ophiolite_core::Result;
use ophiolite_package::{StoredLasFile, write_bundle};
use ophiolite_parser::{ReadOptions, import_las_file};
use ophiolite_project::{OphioliteProject, WellId};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn supported_cli_commands() -> &'static [&'static str] {
    &[
        "operation-catalog",
        "create-project",
        "open-project",
        "project-summary",
        "list-project-wells",
        "list-project-wellbores",
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
    eprintln!("  {binary} import <input.las> <bundle_dir>");
    eprintln!("  {binary} inspect-file <input.las>");
    eprintln!("  {binary} summary <bundle_dir>");
    eprintln!("  {binary} list-curves <bundle_dir>");
    eprintln!("  {binary} generate-fixture-packages <input_root> <output_root>");
}
