use crate::process_signal_audit_dataflow as dataflow;
use crate::process_signal_audit_provenance as provenance;
use crate::process_signal_audit_wrapper_provenance as wrapper_provenance;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::AUDIT_SIGNAL_SITE;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource, identifier, string};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const PATH_SEPARATOR: &str = "::";
const MEMBER_ACCESS: &str = ".";
const UNSAFE_KEYWORD: &str = "unsafe";
const REQUIRED_AUDIT_CALL_COUNT: usize = 1;
const REQUIRED_WRAPPER_TYPE_COUNT: usize = 1;

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    inspect_references(file, source, violations);
    dataflow::inspect(file, source, violations);
    if file.relative == contract::MACOS_SIGNAL_PATH {
        inspect_wrapper(file, source, violations);
    }
}

fn inspect_references(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for (index, token) in source.tokens.iter().enumerate() {
        let symbol = string(token).is_some_and(|value| {
            value
                .trim_end_matches('\0')
                .ends_with(contract::AUDIT_SIGNAL_SYMBOL)
        }) || token.identifier(contract::AUDIT_SIGNAL_SYMBOL);
        let function_type = token.identifier(contract::AUDIT_SIGNAL_FUNCTION);
        let static_binding = token.identifier(contract::AUDIT_STATIC_BINDING);
        let binding_call = file.relative == contract::MACOS_SIGNAL_PATH
            && token.identifier(contract::AUDIT_CALL_BINDING)
            && direct_call(&source.tokens, index);
        let wrapper_reference = token.identifier(contract::AUDIT_WRAPPER_FUNCTION);
        if !symbol && !function_type && !static_binding && !binding_call && !wrapper_reference {
            continue;
        }
        let function = source.function_at(index);
        let allowed = if symbol {
            provenance::approved_symbol(file, source, index)
        } else if function_type {
            provenance::approved_function_type(file, source, index)
        } else if static_binding {
            provenance::approved_static_binding(file, source, index)
        } else if wrapper_reference {
            provenance::approved_wrapper_reference(file, source, index)
        } else {
            AUDIT_SIGNAL_SITE.matches(&file.relative, function)
        };
        if !allowed {
            violation(file, token.line, violations);
        }
    }
}

fn inspect_wrapper(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    let mut wrappers = source
        .functions()
        .filter(|function| function.matches(None, contract::AUDIT_WRAPPER_FUNCTION));
    let Some(wrapper) = wrappers.next() else {
        violations.push(Violation::file(
            &file.relative,
            contract::AUDIT_SIGNAL_MESSAGE,
        ));
        return;
    };
    let type_count = wrapper
        .body
        .clone()
        .filter(|index| source.tokens[*index].identifier(contract::AUDIT_SIGNAL_FUNCTION))
        .count();
    if wrappers.next().is_some()
        || type_count != REQUIRED_WRAPPER_TYPE_COUNT
        || !has_exact_unsafe_call(source, wrapper)
        || !wrapper_provenance::exact_bindings(&source.tokens, wrapper)
    {
        violation(file, wrapper.line, violations);
    }
}

fn has_exact_unsafe_call(source: &RustSource, wrapper: &RustFunction) -> bool {
    let mut expected_calls = 0_usize;
    for index in wrapper.body.clone() {
        if !source.tokens[index].identifier(UNSAFE_KEYWORD)
            || !source
                .tokens
                .get(index + 1)
                .is_some_and(|token| token.symbol(BLOCK_OPEN))
        {
            continue;
        }
        let open = index + 1;
        let Some(close) = closing(&source.tokens, open, BLOCK_OPEN, BLOCK_CLOSE) else {
            return false;
        };
        for call in open + 1..close {
            if parenthesized_call(&source.tokens, call) {
                return false;
            }
            if !direct_call(&source.tokens, call) {
                continue;
            }
            if !source.tokens[call].identifier(contract::AUDIT_CALL_BINDING) {
                return false;
            }
            expected_calls += 1;
        }
    }
    expected_calls == REQUIRED_AUDIT_CALL_COUNT
}

fn direct_call(tokens: &[Token], index: usize) -> bool {
    identifier(&tokens[index]).is_some()
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(CALL_OPEN))
        && (index == 0
            || !tokens[index - 1].symbol(PATH_SEPARATOR)
                && !tokens[index - 1].symbol(MEMBER_ACCESS))
}

fn parenthesized_call(tokens: &[Token], index: usize) -> bool {
    tokens[index].symbol(CALL_CLOSE)
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(CALL_OPEN))
}

fn violation(file: &SourceFile, line: usize, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        line,
        contract::AUDIT_SIGNAL_MESSAGE,
    ));
}

#[cfg(test)]
#[path = "../unit_tests/audit/audit_unsafe_boundary_tests.rs"]
mod boundary_tests;
