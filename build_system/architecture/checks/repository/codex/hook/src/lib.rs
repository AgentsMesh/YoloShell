mod codex_hook_authority_rules;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    codex_hook_authority_rules::inspect(files, violations);
}

#[cfg(test)]
pub fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let mut violations = Vec::new();
    inspect_repository(&files, &mut violations);
    violations.sort();
    violations.dedup();
    Ok(violations)
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
