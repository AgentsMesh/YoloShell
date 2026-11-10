use crate::normalize;
use std::path::Path;

const EXECUTION_ROOT: &str = "/tmp/execroot/_main";

#[test]
fn reports_remove_execution_and_process_working_directory_prefixes() {
    assert_eq!(
        normalize::report(
            "SF:#/proc/self/cwd/core/src/lib.rs\nSF:/tmp/execroot/_main/core/src/main.rs\n",
            Path::new(EXECUTION_ROOT),
        ),
        "SF:core/src/lib.rs\nSF:/core/src/main.rs\n",
    );
}

#[test]
fn source_keys_are_workspace_relative() {
    assert_eq!(
        normalize::source(
            "/tmp/execroot/_main/core/src/lib.rs",
            Path::new(EXECUTION_ROOT)
        ),
        "core/src/lib.rs",
    );
    assert_eq!(
        normalize::lcov_source("./core/src/lib.rs"),
        "core/src/lib.rs"
    );
}
