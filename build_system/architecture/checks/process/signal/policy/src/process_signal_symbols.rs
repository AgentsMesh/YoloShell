use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_exec_contract as exec_contract;
use yolo_architecture_syntax::rust_source::{RustSource, string};

const NULL_TERMINATOR: char = '\0';

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for token in &source.tokens {
        let Some(value) = string(token) else {
            continue;
        };
        let symbol = value.strip_suffix(NULL_TERMINATOR).unwrap_or(value);
        let message = if symbol == exec_contract::EXECVEAT_FUNCTION {
            exec_contract::RAW_EXEC_MESSAGE
        } else if contract::RAW_LIBC_SIGNAL_NAMES.contains(&symbol) {
            contract::RAW_SIGNAL_MESSAGE
        } else {
            continue;
        };
        violations.push(Violation::line(&file.relative, token.line, message));
    }
}
