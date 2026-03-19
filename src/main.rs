use std::env;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args
        .get(1)
        .is_some_and(|arg| arg == "generate-synthetic-project")
    {
        if args.len() != 3 {
            eprintln!("Usage:");
            eprintln!("  {} generate-synthetic-project <output_root>", args[0]);
            std::process::exit(1);
        }

        match ophiolite::generate_synthetic_project_fixture(&args[2]) {
            Ok(summary) => match serde_json::to_string_pretty(&summary) {
                Ok(text) => println!("{text}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            },
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

    ophiolite_cli::main_entry();
}
