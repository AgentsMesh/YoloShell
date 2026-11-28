use crate::constants::{CURRENT_DIRECTORY_PREFIX, EXTERNAL_ARTIFACT_DIRECTORY, PROCESS_CWD_PREFIX};
use crate::staged_paths::normalized_path;
use std::path::Path;

const RUST_EXTENSION: &str = ".rs";

/// Identify a normalized Bazel dependency source without reclassifying repository paths.
pub fn is_external_rust_source(source: &str, execution_root: &Path) -> bool {
    let root_prefix = format!("{}/", execution_root.display());
    let relative = source
        .strip_prefix(PROCESS_CWD_PREFIX)
        .or_else(|| source.strip_prefix(&root_prefix))
        .unwrap_or(source);
    let relative = relative
        .strip_prefix(CURRENT_DIRECTORY_PREFIX)
        .unwrap_or(relative);
    if !relative.ends_with(RUST_EXTENSION) || !normalized_path(relative) {
        return false;
    }
    let mut parts = relative.split('/');
    parts.next() == Some(EXTERNAL_ARTIFACT_DIRECTORY)
        && parts.next().is_some()
        && parts.next().is_some()
}
