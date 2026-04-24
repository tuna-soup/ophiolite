use std::process::ExitCode;

use ophiolite_boundary_check::{repo_root, run_boundary_check};

fn main() -> ExitCode {
    match run_boundary_check(&repo_root()) {
        Ok(report) if report.is_empty() => {
            println!("No workspace boundary violations found.");
            ExitCode::SUCCESS
        }
        Ok(report) => {
            eprintln!("{report}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("Boundary check failed: {error}");
            ExitCode::FAILURE
        }
    }
}
