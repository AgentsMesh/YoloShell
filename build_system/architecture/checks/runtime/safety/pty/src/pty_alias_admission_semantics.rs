use crate::pty_alias_admission_rules as contract;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;

#[path = "pty_alias_admission_semantics/closure.rs"]
mod closure;
#[path = "pty_alias_admission_semantics/termination.rs"]
mod termination;

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if file.relative == contract::PTY_PROCESS_TERMINATION_PATH {
        termination::inspect(file, violations);
    }
    if file.relative.starts_with(contract::WORKER_SOURCE_PREFIX)
        && file.relative.ends_with(contract::RUST_SOURCE_SUFFIX)
    {
        closure::inspect(file, violations);
    }
}
