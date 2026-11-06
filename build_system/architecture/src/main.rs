use std::path::PathBuf;
use yolo_architecture::{inspect_with_runfiles, locate, locate_runfiles_overlay};

const ROOT_ARGUMENT_INDEX: usize = 1;
const POLICY_VIOLATION_EXIT_CODE: i32 = 1;
const CHECK_FAILURE_EXIT_CODE: i32 = 2;

fn main() {
    if let Err(error) = run() {
        eprintln!("architecture check failed: {error}");
        std::process::exit(CHECK_FAILURE_EXIT_CODE);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let explicit = std::env::args_os()
        .nth(ROOT_ARGUMENT_INDEX)
        .map(PathBuf::from);
    let root = locate(explicit.as_deref())?;
    let runfiles = locate_runfiles_overlay()?;
    let violations = inspect_with_runfiles(&root, runfiles.as_deref())?;
    if violations.is_empty() {
        println!("Architecture checks passed.");
        return Ok(());
    }
    for violation in violations {
        eprintln!("{violation}");
    }
    std::process::exit(POLICY_VIOLATION_EXIT_CODE);
}
