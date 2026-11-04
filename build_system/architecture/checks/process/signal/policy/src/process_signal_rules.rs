use crate::process_signal_aliases;
use crate::process_signal_audit;
use crate::process_signal_authorities;
use crate::process_signal_bridges;
use crate::process_signal_child_flow;
use crate::process_signal_child_kills;
use crate::process_signal_child_methods;
use crate::process_signal_direct_custody;
use crate::process_signal_imports;
use crate::process_signal_literals;
use crate::process_signal_primitives;
use crate::process_signal_shell_commands;
use crate::process_signal_symbols;
use crate::process_signal_syscalls;
use crate::process_signal_test_modules;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_source_policy::source_policy::is_rust_source;
use yolo_architecture_syntax::rust_source::RustSource;

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !is_rust_source(&file.relative) {
        return;
    }
    let source = RustSource::parse(&file.text);
    process_signal_aliases::inspect(file, &source, violations);
    process_signal_audit::inspect(file, &source, violations);
    process_signal_imports::inspect(file, &source, violations);
    process_signal_literals::inspect(file, &source, violations);
    process_signal_primitives::inspect(file, &source, violations);
    process_signal_symbols::inspect(file, &source, violations);
    process_signal_authorities::inspect(file, &source, violations);
    process_signal_bridges::inspect(file, &source, violations);
    process_signal_child_kills::inspect(file, &source, violations);
    process_signal_child_methods::inspect(file, &source, violations);
    process_signal_direct_custody::inspect(file, &source, violations);
    process_signal_shell_commands::inspect(file, &source, violations);
    process_signal_syscalls::inspect(file, &source, violations);
    process_signal_test_modules::inspect(file, &source, violations);
    process_signal_child_flow::inspect(file, &source, violations);
}
