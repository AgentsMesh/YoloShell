use crate::arguments::resolve_report_path;
use std::path::PathBuf;

const RELATIVE_REPORT: &str = "bazel-out/_coverage/_coverage_report.dat";
const WORKSPACE: &str = "/workspace";

#[test]
fn bazel_run_paths_are_resolved_from_the_workspace() {
    assert_eq!(
        resolve_report_path(
            PathBuf::from(RELATIVE_REPORT),
            Some(PathBuf::from(WORKSPACE))
        ),
        PathBuf::from(WORKSPACE).join(RELATIVE_REPORT),
    );
}

#[test]
fn direct_execution_preserves_relative_and_absolute_paths() {
    let relative = PathBuf::from(RELATIVE_REPORT);
    assert_eq!(resolve_report_path(relative.clone(), None), relative);
    let absolute = PathBuf::from(WORKSPACE).join(RELATIVE_REPORT);
    assert_eq!(
        resolve_report_path(absolute.clone(), Some(PathBuf::from("/ignored"))),
        absolute,
    );
}
