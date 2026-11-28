mod build_entrypoint_constants;
mod build_entrypoint_rules;
mod current_product_rules;
mod protocol_version_rules;
mod release_acceptance_rules;
mod required_field_constants;
mod required_field_rules;
mod required_field_syntax;
mod retired_wire_constants;
mod retired_wire_rules;
mod toolchain_rules;
mod version_identity_constants;
mod version_identity_rules;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub use toolchain_rules::RUST_COVERAGE_DECLARATIONS;

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    release_acceptance_rules::inspect(files, violations);
    toolchain_rules::inspect(files, violations);
    version_identity_rules::inspect(files, violations);
}

pub fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    build_entrypoint_rules::inspect(file, violations);
    current_product_rules::inspect(file, violations);
    protocol_version_rules::inspect(file, violations);
    retired_wire_rules::inspect(file, violations);
    required_field_rules::inspect(file, violations);
}

#[cfg(test)]
pub fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let mut violations = Vec::new();
    inspect_repository(&files, &mut violations);
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
