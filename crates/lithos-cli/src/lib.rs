use lithos_core::Result;
use lithos_package::{StoredLasFile, write_bundle};
use lithos_parser::{ReadOptions, import_las_file};
use serde_json::json;
use std::env;
use std::path::Path;

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
                    "root": lithos_parser::examples::path(""),
                    "options": ReadOptions::default(),
                }))?
            );
        }
        _ => print_usage(),
    }

    Ok(())
}

fn print_usage() {
    let binary = env::args()
        .next()
        .and_then(|path| {
            Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| String::from("lithos_las"));

    eprintln!("Usage:");
    eprintln!("  {binary} import <input.las> <bundle_dir>");
    eprintln!("  {binary} inspect-file <input.las>");
    eprintln!("  {binary} summary <bundle_dir>");
    eprintln!("  {binary} list-curves <bundle_dir>");
}
