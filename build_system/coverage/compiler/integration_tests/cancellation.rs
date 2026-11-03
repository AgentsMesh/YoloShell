#[path = "../test_support/commands.rs"]
mod commands;
#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/coverage_report.rs"]
mod coverage_report;
use commands::{binary, output, tool};
use constants::*;
use serde_json::Value;
use std::process::Command;

#[tokio::test]
async fn cancellation_and_panicking_drop_preserve_zero_continuations_at_every_opt_level() {
    for variant in VARIANTS {
        let root = tempfile::tempdir().unwrap();
        let executable = binary(variant);
        let mut run = Command::new(&executable);
        run.env(PROFILE_ENV, root.path().join(PROFILE_PATTERN));
        assert_eq!(output(run).await, WITNESS.as_bytes(), "{variant}");
        let profiles = std::fs::read_dir(root.path())
            .unwrap()
            .map(Result::unwrap)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == PROFILE_EXTENSION))
            .collect::<Vec<_>>();
        assert_eq!(profiles.len(), SINGLE, "{variant}");
        let merged = root.path().join(MERGED_FILE);
        let mut merge = Command::new(tool(LLVM_PROFDATA));
        merge
            .args([MERGE, SPARSE, OUTPUT])
            .arg(&merged)
            .args(&profiles);
        output(merge).await;
        let mut export = Command::new(tool(LLVM_COV));
        export
            .arg(EXPORT)
            .arg(PROFILE_ARGUMENT)
            .arg(&merged)
            .arg(&executable);
        let report: Value = serde_json::from_slice(&output(export).await).unwrap();
        coverage_report::check(&report, variant);
    }
}
