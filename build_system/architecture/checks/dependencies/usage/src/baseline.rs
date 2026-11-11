use std::collections::BTreeSet;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::starlark_baseline::tuple_entries;

const BASELINE_FILE: &str = "build_system/architecture/dependency_usage_baseline.bzl";
const BASELINE_ASSIGNMENT: &str = "DEPENDENCY_USAGE_BASELINE";
pub(super) const STALE_POLICY: &str = "stale aggregate dependency baseline entry";
const MALFORMED_POLICY: &str =
    "aggregate dependency baseline entries need a path, target, and label string";

pub(super) fn sites(
    files: &[SourceFile],
    violations: &mut Vec<Violation>,
) -> BTreeSet<(String, String, String)> {
    let Some(baseline) = files.iter().find(|file| file.relative == BASELINE_FILE) else {
        return BTreeSet::new();
    };
    let entries = match tuple_entries(&baseline.text, BASELINE_ASSIGNMENT, 3) {
        Ok(entries) => entries,
        Err(error) => {
            violations.push(Violation::file(
                &baseline.relative,
                format!("{MALFORMED_POLICY}: line {}", error.line),
            ));
            return BTreeSet::new();
        }
    };
    entries
        .into_iter()
        .map(|entry| {
            let (path, target, dependency) = (&entry.values[0], &entry.values[1], &entry.values[2]);
            (path.clone(), target.clone(), dependency.clone())
        })
        .collect()
}
