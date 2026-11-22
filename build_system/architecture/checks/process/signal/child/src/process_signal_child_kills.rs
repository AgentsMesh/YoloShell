use crate::process_signal_child_imports::{unsafe_import, use_statement};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::CHILD_KILL_SITES;
use yolo_architecture_process_signal_foundation::process_signal_unidentified_custody as unidentified;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const METHOD_SEPARATOR: &str = ".";
const PATH_SEPARATOR: &str = "::";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const STD_NAMESPACE: &str = "std";
const PROCESS_NAMESPACE: &str = "process";
const REFERENCE: &str = "&";
const MUTABLE_KEYWORD: &str = "mut";
const CHILD_CALL_PREFIX_LENGTH: usize = 7;

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative)
        || file.relative == contract::PTY_SESSION_LINUX_INTEGRATION_HOLDER_PATH
    {
        return;
    }
    inspect_kill_references(file, source, violations);
}

fn inspect_kill_references(
    file: &SourceFile,
    source: &RustSource,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(contract::KILL_METHOD) || libc_reference(&source.tokens, index) {
            continue;
        }
        let import = use_statement(&source.tokens, index);
        let method = import.is_none() && method_call(&source.tokens, index);
        let protected = import.map_or_else(
            || method || qualified_reference(&source.tokens, index),
            |statement| unsafe_import(&source.tokens, index, statement),
        );
        if !protected {
            continue;
        }
        if unit_test_function(source, index)
            || CHILD_KILL_SITES.iter().any(|site| {
                site.site.matches(&file.relative, source.function_at(index))
                    && exact_child_call(&source.tokens, index, site)
            })
        {
            continue;
        }
        violation(file, token, contract::CHILD_KILL_MESSAGE, violations);
    }
}

fn exact_child_call(
    tokens: &[Token],
    call: usize,
    site: &yolo_architecture_process_signal_foundation::process_signal_sites::ReceiverSite,
) -> bool {
    if site.site.path == unidentified::PATH {
        return exact_portable_pty_child_call(tokens, call);
    }
    exact_std_child_call(tokens, call, site.receiver)
}

fn exact_portable_pty_child_call(tokens: &[Token], call: usize) -> bool {
    let Some(prefix_start) = call.checked_sub(4) else {
        return false;
    };
    matches!(tokens.get(prefix_start..call),
        Some([receiver, first, child, second])
            if receiver.identifier(contract::SELF_RECEIVER)
                && first.symbol(METHOD_SEPARATOR)
                && child.identifier(contract::CHILD_RECEIVER)
                && second.symbol(METHOD_SEPARATOR))
}

fn method_call(tokens: &[Token], index: usize) -> bool {
    index > 0
        && tokens[index - 1].symbol(METHOD_SEPARATOR)
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(CALL_OPEN))
}

fn exact_std_child_call(tokens: &[Token], call: usize, expected: &str) -> bool {
    let Some(prefix_start) = call.checked_sub(CHILD_CALL_PREFIX_LENGTH) else {
        return false;
    };
    let rooted_child = matches!(tokens.get(prefix_start..call),
        Some([root, standard, first, process, second, child, third])
            if root.symbol(PATH_SEPARATOR)
                && standard.identifier(STD_NAMESPACE)
                && first.symbol(PATH_SEPARATOR)
                && process.identifier(PROCESS_NAMESPACE)
                && second.symbol(PATH_SEPARATOR)
                && child.identifier(contract::CHILD)
                && third.symbol(PATH_SEPARATOR));
    if !rooted_child {
        return false;
    }
    let open = call + 1;
    let Some(close) = closing(tokens, open, CALL_OPEN, CALL_CLOSE) else {
        return false;
    };
    let arguments = &tokens[open + 1..close];
    if expected == contract::SELF_RECEIVER {
        matches!(arguments, [receiver] if receiver.identifier(contract::SELF_RECEIVER))
    } else if expected == contract::SELF_CHILD_RECEIVER {
        matches!(arguments, [reference, mutable, receiver, member, child]
            if reference.symbol(REFERENCE)
                && mutable.identifier(MUTABLE_KEYWORD)
                && receiver.identifier(contract::SELF_RECEIVER)
                && member.symbol(METHOD_SEPARATOR)
                && child.identifier(contract::CHILD_RECEIVER))
    } else {
        matches!(arguments, [reference, mutable, receiver]
            if reference.symbol(REFERENCE)
                && mutable.identifier(MUTABLE_KEYWORD)
                && receiver.identifier(contract::CHILD_RECEIVER))
    }
}

fn qualified_reference(tokens: &[Token], index: usize) -> bool {
    index > 0
        && tokens[index - 1].symbol(PATH_SEPARATOR)
        && !tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(PATH_SEPARATOR))
}

fn libc_reference(tokens: &[Token], index: usize) -> bool {
    index >= 2
        && tokens[index - 2].identifier(contract::LIBC_NAMESPACE)
        && tokens[index - 1].symbol(PATH_SEPARATOR)
}

fn unit_test_function(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_unit_test_guarded(function))
}

fn violation(file: &SourceFile, token: &Token, message: &str, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, message));
}

#[cfg(test)]
#[path = "../unit_tests/kill_tokens.rs"]
mod token_tests;
