use std::path::Path;
use yolo_architecture::{
    CONFORMANCE_ROOT_ENV, TEST_SOURCE_DIRECTORY_ENV, TEST_WORKSPACE_ENV, WORKSPACE_MARKER, inspect,
    locate,
};

const REPORT_SEPARATOR: &str = "\n";

#[test]
fn configured_workspace_conforms() {
    let override_root = std::env::var_os(CONFORMANCE_ROOT_ENV);
    let under_bazel = std::env::var_os(TEST_SOURCE_DIRECTORY_ENV).is_some()
        && std::env::var_os(TEST_WORKSPACE_ENV).is_some();
    if override_root.is_none() && !under_bazel {
        return;
    }
    let root = locate(override_root.as_deref().map(Path::new)).unwrap();
    assert!(
        root.join(WORKSPACE_MARKER).is_file(),
        "architecture conformance root has no workspace marker: {}",
        root.display()
    );
    let violations = inspect(&root).unwrap();
    let report = violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(REPORT_SEPARATOR);
    assert!(
        violations.is_empty(),
        "workspace architecture violations:\n{report}"
    );
}
