mod process_signal_aliases;
mod process_signal_audit;
mod process_signal_audit_dataflow;
mod process_signal_audit_provenance;
mod process_signal_audit_wrapper_provenance;
mod process_signal_authorities;
mod process_signal_bridges;
mod process_signal_direct_custody;
mod process_signal_direct_custody_sites;
mod process_signal_imports;
mod process_signal_literals;
mod process_signal_primitives;
mod process_signal_rules;
mod process_signal_shell_commands;
mod process_signal_symbols;
mod process_signal_syscalls;
mod process_signal_test_modules;

pub(crate) use yolo_architecture_process_signal_child_policy::{
    process_signal_child_flow, process_signal_child_kills, process_signal_child_methods,
};
pub(crate) use yolo_architecture_process_signal_retained_policy::process_signal_retained_repository;
pub use yolo_architecture_process_signal_retained_policy::process_signal_retained_visibility;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect_repository(files: &[SourceFile], violations: &mut Vec<Violation>) {
    process_signal_retained_repository::inspect(files, violations);
}

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    process_signal_rules::inspect(file, violations);
}

#[cfg(test)]
#[path = "../unit_tests/audit_internal.rs"]
mod audit_internal;
