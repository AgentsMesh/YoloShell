mod pty_alias_admission_rules;
mod pty_alias_admission_semantics;
mod pty_master_closure_ack_syntax;
#[path = "pty_master_closure/constants.rs"]
mod pty_master_closure_contract;
mod pty_master_closure_rules;
mod pty_master_closure_sites;
mod pty_master_closure_syntax;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    pty_master_closure_rules::inspect(files, violations);
}

pub fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    pty_alias_admission_rules::inspect(file, violations);
    pty_alias_admission_semantics::inspect(file, violations);
}

#[cfg(test)]
fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
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

#[cfg(all(test, feature = "yolo-runtime-pty-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
