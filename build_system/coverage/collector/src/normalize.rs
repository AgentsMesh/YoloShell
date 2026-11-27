use std::path::Path;
use yolo_coverage_source_contract::{CURRENT_DIRECTORY_PREFIX, PROCESS_CWD_PREFIX};

pub fn report(text: &str, execution_root: &Path) -> String {
    text.replace(PROCESS_CWD_PREFIX, "")
        .replace(&execution_root.display().to_string(), "")
}

pub fn source(path: &str, execution_root: &Path) -> String {
    let normalized = report(path, execution_root);
    let relative = normalized.strip_prefix('/').unwrap_or(&normalized);
    relative
        .strip_prefix(CURRENT_DIRECTORY_PREFIX)
        .unwrap_or(relative)
        .to_owned()
}

pub fn lcov_source(path: &str) -> &str {
    let relative = path.strip_prefix('/').unwrap_or(path);
    relative
        .strip_prefix(CURRENT_DIRECTORY_PREFIX)
        .unwrap_or(relative)
}
