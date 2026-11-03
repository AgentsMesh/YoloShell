mod foundation_directory;
mod shell_rules;
mod ui_rules;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub use shell_rules::{command_name, split_commands};
pub(crate) use yolo_architecture_repository_policy_foundation as constants;

pub fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    ui_rules::inspect(file, violations);
    shell_rules::inspect(file, violations);
}

#[cfg(test)]
pub fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let mut violations = Vec::new();
    for file in &files {
        inspect_file(file, &mut violations);
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
