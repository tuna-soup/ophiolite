use lithos_las::{CurveWindow, Result, StoredLasAsset, import_las_file, write_bundle};
use serde_json::json;
use std::env;
use std::path::Path;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "import" if args.len() == 4 => {
            let asset = import_las_file(&args[2])?;
            let bundle = write_bundle(&asset, &args[3])?;
            println!("{}", serde_json::to_string_pretty(bundle.summary())?);
        }
        "summary" if args.len() == 3 => {
            let bundle = StoredLasAsset::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(bundle.summary())?);
        }
        "list-curves" if args.len() == 3 => {
            let bundle = StoredLasAsset::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&bundle.list_curves())?);
        }
        "read-curve" if args.len() == 4 || args.len() == 6 => {
            let bundle = StoredLasAsset::open(&args[2])?;
            let window = parse_window(&args)?;
            let payload = bundle.read_curve(&args[3], window)?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        "inspect-file" if args.len() == 3 => {
            let asset = import_las_file(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "summary": asset.summary,
                    "index": asset.index,
                    "curve_count": asset.curves.len(),
                    "issues": asset.issues,
                }))?
            );
        }
        _ => print_usage(),
    }

    Ok(())
}

fn parse_window(args: &[String]) -> Result<Option<CurveWindow>> {
    if args.len() != 6 {
        return Ok(None);
    }

    let start = args[4]
        .parse::<usize>()
        .map_err(|_| lithos_las::LasError::Parse(String::from("Invalid window start.")))?;
    let end = args[5]
        .parse::<usize>()
        .map_err(|_| lithos_las::LasError::Parse(String::from("Invalid window end.")))?;
    Ok(Some(CurveWindow::new(start, end)))
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
    eprintln!("  {binary} read-curve <bundle_dir> <curve_id> [start end]");
}
