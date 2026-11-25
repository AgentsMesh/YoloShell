use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ARGUMENT_SEPARATOR: &str = ",";
const SAFE_BOOLEAN: &str = "false";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative) {
        return;
    }
    for (index, token) in source.tokens.iter().enumerate() {
        let sensitive = contract::IMPLICIT_CHILD_TERMINATION_NAMES
            .iter()
            .any(|name| token.identifier(name));
        if !sensitive || unit_test_function(source, index) {
            continue;
        }
        if token.identifier(contract::LIBC_PRCTL)
            && seccomp_owner(file, source, &source.tokens, index)
        {
            continue;
        }
        if token.identifier(contract::KILL_ON_DROP_METHOD)
            && exact_false_argument(&source.tokens, index)
        {
            continue;
        }
        violations.push(Violation::line(
            &file.relative,
            token.line,
            contract::CHILD_LIFECYCLE_MESSAGE,
        ));
    }
}

fn seccomp_owner(
    file: &SourceFile,
    source: &RustSource,
    tokens: &[Token],
    call: usize,
) -> bool {
    file.relative == contract::SECCOMP_OWNER_PATH
        && source
            .function_at(call)
            .is_some_and(|function| function.matches(None, contract::SECCOMP_OWNER_FUNCTION))
        && allowed_prctl_option(tokens, call)
}

fn allowed_prctl_option(tokens: &[Token], call: usize) -> bool {
    tokens.get(call + 1).is_some_and(|token| token.symbol(CALL_OPEN))
        && tokens
            .get(call + 2)
            .and_then(identifier)
            .is_some_and(|option| contract::SECCOMP_PRCTL_OPTIONS.contains(&option))
}

fn exact_false_argument(tokens: &[Token], call: usize) -> bool {    let open = call + 1;
    let Some(close) = closing(tokens, open, CALL_OPEN, CALL_CLOSE) else {
        return false;
    };
    let end = close - usize::from(tokens[close - 1].symbol(ARGUMENT_SEPARATOR));
    let start = tokens[open + 1..end]
        .iter()
        .rposition(|token| token.symbol(ARGUMENT_SEPARATOR))
        .map_or(open + 1, |offset| open + 2 + offset);
    matches!(tokens.get(start..end), Some([argument]) if argument.identifier(SAFE_BOOLEAN))
}

fn unit_test_function(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_unit_test_guarded(function))
}
