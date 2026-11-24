use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    yolo_architecture_runtime_safety_boundary_policy::inspect_repository(files, violations);
    yolo_architecture_runtime_safety_pty_policy::inspect_repository(files, violations);
}

pub fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    yolo_architecture_runtime_safety_boundary_policy::inspect_file(file, violations);
    yolo_architecture_runtime_safety_process_policy::inspect_file(file, violations);
    yolo_architecture_runtime_safety_pty_policy::inspect_file(file, violations);
}

#[cfg(test)]
#[path = "../unit_tests/delegation_tests.rs"]
mod delegation_tests;
