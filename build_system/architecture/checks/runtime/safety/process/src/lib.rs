mod constants;
mod detached_process_rules;
mod pre_exec_rlimit_callbacks;
#[path = "pre_exec_rlimit/constants.rs"]
mod pre_exec_rlimit_contract;
mod pre_exec_rlimit_rules;
mod pre_exec_rlimit_syntax;
#[path = "process_retirement/constants.rs"]
mod process_retirement_contract;
mod process_retirement_rules;
mod pty_session_spawn_rules;
#[path = "remote_broker_process/constants.rs"]
mod remote_broker_process_contract;
mod remote_broker_process_rules;
#[path = "rust_process_spawn/constants.rs"]
mod rust_process_spawn_contract;
mod rust_process_spawn_rules;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    detached_process_rules::inspect(file, violations);
    pre_exec_rlimit_rules::inspect(file, violations);
    process_retirement_rules::inspect(file, violations);
    pty_session_spawn_rules::inspect(file, violations);
    remote_broker_process_rules::inspect(file, violations);
    rust_process_spawn_rules::inspect(file, violations);
}

#[cfg(test)]
fn inspect(root: &std::path::Path) -> std::io::Result<Vec<Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let mut violations = Vec::new();
    for file in &files {
        inspect_file(file, &mut violations);
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}

#[cfg(all(test, feature = "yolo-runtime-process-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
