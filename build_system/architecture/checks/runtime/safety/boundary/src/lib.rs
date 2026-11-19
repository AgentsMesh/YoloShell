mod constants;
mod linux_containment_thaw_rules;
mod scm_rights_contract_rules;
mod socket_root_atomic_rules;
mod targeted_frame_rules;
pub mod targeted_regression_rules;
mod targeted_rust_rules;
mod targeted_swift_rules;
mod test_isolation_contract;
mod test_isolation_rules;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    scm_rights_contract_rules::inspect(files, violations);
    socket_root_atomic_rules::inspect(files, violations);
}

pub fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    linux_containment_thaw_rules::inspect(file, violations);
    targeted_regression_rules::inspect(file, violations);
    test_isolation_rules::inspect(file, violations);
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

#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
