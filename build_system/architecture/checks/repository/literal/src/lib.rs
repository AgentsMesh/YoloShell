mod literal_rules;
mod timing_rules;

use std::collections::BTreeSet;
use yolo_architecture_foundation::{SourceFile, model::Violation};

pub(crate) use yolo_architecture_repository_policy_foundation as constants;

pub type LiteralRegistry = BTreeSet<String>;

pub fn registry(files: &[SourceFile]) -> LiteralRegistry {
    literal_rules::registry(files)
}

pub fn inspect_file(
    file: &SourceFile,
    literals: &LiteralRegistry,
    violations: &mut Vec<Violation>,
) {
    timing_rules::inspect(file, violations);
    literal_rules::inspect(file, literals, violations);
}

#[cfg(test)]
pub fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let literals = registry(&files);
    let mut violations = Vec::new();
    for file in &files {
        inspect_file(file, &literals, &mut violations);
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
