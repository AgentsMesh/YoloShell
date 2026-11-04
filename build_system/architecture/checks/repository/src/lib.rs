use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_repository_literal_policy as literal_policy;

pub use literal_policy::LiteralRegistry;

#[cfg(test)]
pub(crate) use yolo_architecture_repository_contract_policy::RUST_COVERAGE_DECLARATIONS;
#[cfg(test)]
pub(crate) use yolo_architecture_repository_policy_foundation as constants;

pub fn literal_registry(files: &[SourceFile]) -> LiteralRegistry {
    literal_policy::registry(files)
}

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    yolo_architecture_repository_codex_hook_policy::inspect_repository(files, violations);
    yolo_architecture_repository_contract_policy::inspect_repository(files, violations);
}

pub fn inspect_file(
    file: &SourceFile,
    literals: &LiteralRegistry,
    violations: &mut Vec<Violation>,
) {
    yolo_architecture_repository_ui_policy::inspect_file(file, violations);
    literal_policy::inspect_file(file, literals, violations);
    yolo_architecture_repository_contract_policy::inspect_file(file, violations);
}

#[cfg(test)]
pub fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let literals = literal_registry(&files);
    let mut violations = Vec::new();
    inspect_repository(&files, &mut violations);
    for file in &files {
        yolo_architecture_source_policy::line_rules::inspect(file, &mut violations);
        yolo_architecture_source_policy::source_policy::inspect(file, &mut violations);
        inspect_file(file, &literals, &mut violations);
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
