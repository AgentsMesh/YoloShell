use super::ObservedProcess;
use super::constants::{NODE_ENTRYPOINT_SCAN_LIMIT, NODE_EXECUTABLES};
use std::path::{Component, Path};

pub fn matches_executable_or_node_entrypoint(
    process: &ObservedProcess,
    executable_names: &[&str],
    package_components: &[&str],
    entrypoint_names: &[&str],
) -> bool {
    let executable = Path::new(&process.executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&process.executable);
    if executable_names.contains(&executable) {
        return true;
    }
    if !NODE_EXECUTABLES.contains(&executable) {
        return false;
    }
    node_entrypoint_index(&process.argv, package_components, entrypoint_names).is_some()
}

/// Index of the script argument for a node process. Leading node options are
/// skipped within a bounded window; the candidate must still carry the exact
/// package component sequence and entrypoint file name.
pub fn node_entrypoint_index(
    argv: &[String],
    package_components: &[&str],
    entrypoint_names: &[&str],
) -> Option<usize> {
    argv.iter()
        .enumerate()
        .skip(1)
        .take(NODE_ENTRYPOINT_SCAN_LIMIT)
        .find_map(|(index, entrypoint)| {
            (!entrypoint.starts_with('-')
                && has_component_sequence(entrypoint, package_components)
                && Path::new(entrypoint)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| entrypoint_names.contains(&name)))
            .then_some(index)
        })
}

fn has_component_sequence(path: &str, expected: &[&str]) -> bool {
    if expected.is_empty() {
        return true;
    }
    let components: Vec<_> = Path::new(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect();
    components
        .windows(expected.len())
        .any(|window| window == expected)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/matching_tests.rs"]
mod tests;
